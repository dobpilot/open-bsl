//! Нативный runtime фоновых заданий: конфигурация, реестр, admission.
//!
//! Архитектурные инварианты (план фоновых заданий,
//! `docs/archive/plans/background-jobs.md`):
//! реестр хранит только владеющие `Send`-DTO и никогда не вызывает внешний
//! код под своим локом; мьютекс — синхронный `std::sync::Mutex` с короткими
//! секциями и не пересекает `await`; занятый пул означает FIFO, а не
//! ошибку; первый terminal transition выигрывает ровно один раз.

use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::Duration;

use bsl_rt::{
    GlobalStagingBudget, HostError, HostErrorCode, JobErrorDto, JobId, JobKeyDto, JobSnapshotDto,
    JobStateDto, SerializedValueGraph, UserMessageDto,
};

/// Конфигурация фонового runtime — одна публичная структура; scheduler
/// настраивается своей `SchedulerConfig` и здесь не дублируется.
///
/// Значения по умолчанию подтверждены нагрузочной сессией 2026-08-27
/// (см. «Нагрузочные и регрессионные проверки» плана фоновых заданий).
#[derive(Debug, Clone)]
pub struct BackgroundJobConfig {
    /// Число OS-потоков пула; `None` — `available_parallelism`, минимум 1.
    pub workers: Option<usize>,
    pub max_inflight_jobs: usize,
    pub max_live_payload_bytes: usize,
    /// История: намеренное расширение open-bsl — 10 000 против
    /// документированных 1 000 у платформы.
    pub max_history_jobs: usize,
    pub max_history_bytes: usize,
    pub max_single_job_record_bytes: usize,
    pub max_error_bytes_per_job: usize,
    pub max_message_bytes_per_job: usize,
    pub max_staged_temp_bytes_per_job: usize,
    pub max_live_staged_temp_bytes: usize,
    pub shutdown_timeout: Duration,
}

impl Default for BackgroundJobConfig {
    fn default() -> Self {
        Self {
            workers: None,
            max_inflight_jobs: 1_024,
            max_live_payload_bytes: 256 << 20,
            max_history_jobs: 10_000,
            max_history_bytes: 256 << 20,
            max_single_job_record_bytes: 64 << 20,
            max_error_bytes_per_job: 1 << 20,
            max_message_bytes_per_job: 4 << 20,
            max_staged_temp_bytes_per_job: 64 << 20,
            max_live_staged_temp_bytes: 256 << 20,
            shutdown_timeout: Duration::from_secs(30),
        }
    }
}

impl BackgroundJobConfig {
    /// Проверка согласованности — скрытых clamp нет: несогласованная
    /// конфигурация отвергается сборкой движка.
    ///
    /// # Errors
    ///
    /// Текст первого нарушения.
    pub fn validate(&self) -> Result<(), String> {
        if self.workers == Some(0) {
            return Err("число workers не может быть нулевым".to_string());
        }
        if self.max_staged_temp_bytes_per_job > self.max_live_staged_temp_bytes {
            return Err("staging одного задания больше глобального staging-бюджета".to_string());
        }
        if self.max_single_job_record_bytes > self.max_history_bytes {
            return Err("одна запись истории больше всего бюджета истории".to_string());
        }
        // `checked_add`: публичная конфигурация — переполнение суммы
        // бюджетов отвергается как несогласованность, а не паникует.
        match self
            .max_error_bytes_per_job
            .checked_add(self.max_message_bytes_per_job)
        {
            Some(sum) if sum <= self.max_single_job_record_bytes => {}
            _ => {
                return Err("бюджеты ошибки и сообщений не помещаются в запись истории".to_string());
            }
        }
        if self.max_inflight_jobs == 0 {
            return Err("max_inflight_jobs не может быть нулевым".to_string());
        }
        Ok(())
    }

    /// Фактическое число workers.
    #[must_use]
    pub fn effective_workers(&self) -> usize {
        self.workers
            .unwrap_or_else(|| {
                std::thread::available_parallelism()
                    .map(std::num::NonZeroUsize::get)
                    .unwrap_or(1)
            })
            .max(1)
    }
}

/// Источник времени runtime: wall-часы для `Начало`/`Конец` снимков.
/// Монотонных deadlines и таймеров здесь нет — пул спит на condvar до
/// события (`wake_epoch`), а не по таймеру; тестовая реализация
/// подставляет ручные значения.
pub trait JobTimeSource: Send + Sync {
    fn wall_now(&self) -> Option<bsl_rt::BslDate>;
}

/// Системные wall-часы. Снимки получают наивное UTC-время: локальная
/// зона сеанса и точный формат платформенных `Начало`/`Конец` уточняются
/// замером `JOB.STATE.SNAPSHOT`. Wall clock не clamp'ится при скачке
/// назад — монотонные длительности придут отдельным источником.
struct SystemJobTime;

impl JobTimeSource for SystemJobTime {
    fn wall_now(&self) -> Option<bsl_rt::BslDate> {
        let unix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()?
            .as_secs();
        bsl_rt::BslDate::from_seconds(unix as i64 + bsl_rt::UNIX_EPOCH_SECONDS)
    }
}

/// Источник идентификаторов заданий: стандартный — OS random + UUID v4,
/// тестовый — детерминированная последовательность.
pub trait JobIdSource: Send + Sync {
    fn next_id(&self) -> JobId;
}

/// Состояние runtime. Занятость workers его не меняет: занятый пул — это
/// FIFO, а не ошибка.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RuntimeState {
    Cold,
    Starting,
    Running,
    Broken,
    Closed,
}

/// Запись реестра: снимок + резервы admission. Только владеющие DTO.
pub(crate) struct JobRecord {
    pub snapshot: JobSnapshotDto,
    /// Байты записи, зарезервированные в live-бюджете до terminal:
    /// payload, ключ и строки снимка (имя метода, наименование).
    pub base_bytes: usize,
    /// Цель вызова числами: модуль каталога и индекс чанка.
    pub target: (u32, u16),
    /// Host-профиль сеанса задания: 0 — системный, `i` — профиль `i - 1`
    /// таблицы движка. Дочернее задание наследует профиль родителя и
    /// повысить возможности не может — другого значения ему взять негде.
    pub profile_index: u32,
    /// Кооперативная отмена: worker проверяет флаг на границах квантов.
    pub cancel_requested: Arc<std::sync::atomic::AtomicBool>,
    /// Token временного хранилища сеанса-вызывателя — приёмник staging.
    pub caller_token: Option<[u8; 16]>,
    /// Сообщения пользователю (`Сообщить` внутри задания), FIFO.
    pub messages: Vec<UserMessageDto>,
    /// Накопленный (кумулятивный) размер сообщений: dren не возвращает
    /// бюджет, поэтому суммарная память сообщений одного задания
    /// ограничена `max_message_bytes_per_job` независимо от чтений.
    pub message_bytes: usize,
    /// Право на terminal transition забрано драйвером: он публикует
    /// write-set и завершит запись сам. Пока флаг взведён, поломка
    /// runtime эту запись не трогает — иначе публикация состоялась бы у
    /// задания, которое реестр уже объявил `Failed(RuntimeBroken)`, а
    /// матрица публикаций требует rollback на инфраструктурном сбое.
    pub committing: bool,
}

/// Ключ уникальности: пара «цель + снимок ключа» резервируется до
/// terminal transition (`JOB.KEY.QUEUED` уточнит участие queued).
type KeyReservation = (u32, u16, JobKeyDto);

/// Реестр под одним синхронным мьютексом: записи, очередь, ключи,
/// бюджеты, история. История хранит `Arc`-снимки: листинг клонирует
/// указатели под локом и фильтрует вне его.
pub(crate) struct JobRegistry {
    pub state: RuntimeState,
    records: HashMap<JobId, JobRecord>,
    pub queue: VecDeque<JobId>,
    /// Счётчик host-событий для точного сна worker: завершение
    /// host-операции и отмена поднимают его под локом и будят
    /// `work_available`. Спящий worker сравнивает счётчик со своим
    /// снимком — пропущенных пробуждений нет по построению, таймерный
    /// поллинг не нужен.
    pub wake_epoch: u64,
    keys: Vec<(KeyReservation, JobId)>,
    live_payload_bytes: usize,
    inflight: usize,
    /// История terminal-записей: точный размер записи (payload + ключ +
    /// строки + ошибка + сообщения) хранится рядом со снимком, чтобы
    /// вытеснение вычитало ровно то, что добавлялось.
    history: VecDeque<(usize, Arc<JobSnapshotDto>)>,
    /// Сообщения terminal-заданий: `ПолучитьСообщенияПользователю(Истина)`
    /// дренирует их атомарно (ИЗМЕРЕНО, `JOB.MESSAGES`), запись
    /// вытесняется вместе со своей строкой истории.
    history_messages: HashMap<JobId, Vec<UserMessageDto>>,
    history_bytes: usize,
    config: BackgroundJobConfig,
}

/// Исход admission.
pub(crate) enum AdmissionError {
    /// Явный ресурсный лимит — ловимая ошибка BSL.
    ResourceLimit(String),
    /// Дублирующий ключ активного задания.
    DuplicateKey,
    /// Runtime закрыт или сломан.
    Unavailable(RuntimeState),
}

impl JobRegistry {
    pub fn new(config: BackgroundJobConfig) -> Self {
        Self {
            state: RuntimeState::Cold,
            records: HashMap::new(),
            queue: VecDeque::new(),
            wake_epoch: 0,
            keys: Vec::new(),
            live_payload_bytes: 0,
            inflight: 0,
            history: VecDeque::new(),
            history_messages: HashMap::new(),
            history_bytes: 0,
            config,
        }
    }

    /// Admission: атомарно резервирует слот, байты записи (payload, ключ,
    /// строки снимка) и ключ до terminal transition; ставит задание в
    /// глобальную FIFO. Заранее отвергает задание, чья максимально
    /// возможная запись — вместе с бюджетами ошибки и сообщений — не
    /// помещается в `max_single_job_record_bytes`.
    #[allow(clippy::too_many_arguments)]
    pub fn admit(
        &mut self,
        id: JobId,
        method_name: String,
        target: (u32, u16),
        params: Arc<SerializedValueGraph>,
        key: Option<Arc<JobKeyDto>>,
        description: Option<String>,
        caller_token: Option<[u8; 16]>,
        profile_index: u32,
    ) -> Result<JobSnapshotDto, AdmissionError> {
        match self.state {
            RuntimeState::Broken | RuntimeState::Closed => {
                return Err(AdmissionError::Unavailable(self.state));
            }
            _ => {}
        }
        if self.inflight >= self.config.max_inflight_jobs {
            return Err(AdmissionError::ResourceLimit(format!(
                "достигнут предел одновременных заданий ({})",
                self.config.max_inflight_jobs
            )));
        }
        // Суммы размеров считаются `checked_add`: переполнение — тот же
        // ловимый отказ ресурса, что и выход за предел, а не паника
        // debug-сборки и не тихий wrap release.
        let base_bytes = params
            .byte_size()
            .checked_add(key.as_ref().map_or(0, |key| key.graph.byte_size()))
            .and_then(|bytes| bytes.checked_add(method_name.len()))
            .and_then(|bytes| bytes.checked_add(description.as_deref().map_or(0, str::len)));
        let worst_case = base_bytes
            .and_then(|bytes| bytes.checked_add(self.config.max_error_bytes_per_job))
            .and_then(|bytes| bytes.checked_add(self.config.max_message_bytes_per_job));
        let (base_bytes, worst_case) = match (base_bytes, worst_case) {
            (Some(base_bytes), Some(worst_case)) => (base_bytes, worst_case),
            _ => {
                return Err(AdmissionError::ResourceLimit(
                    "параметры задания больше предела одной записи истории".to_string(),
                ));
            }
        };
        if worst_case > self.config.max_single_job_record_bytes {
            return Err(AdmissionError::ResourceLimit(
                "параметры задания больше предела одной записи истории".to_string(),
            ));
        }
        match self.live_payload_bytes.checked_add(base_bytes) {
            Some(live) if live <= self.config.max_live_payload_bytes => {}
            _ => {
                return Err(AdmissionError::ResourceLimit(
                    "исчерпан live-бюджет параметров заданий".to_string(),
                ));
            }
        }
        if let Some(key) = &key {
            let reservation = (target.0, target.1, (**key).clone());
            if self
                .keys
                .iter()
                .any(|(existing, _)| *existing == reservation)
            {
                return Err(AdmissionError::DuplicateKey);
            }
            self.keys.push((reservation, id));
        }
        self.inflight += 1;
        self.live_payload_bytes += base_bytes;
        let snapshot = JobSnapshotDto {
            id,
            method_name,
            params,
            key,
            description,
            state: JobStateDto::Queued,
            begin: None,
            end: None,
            error: None,
        };
        self.records.insert(
            id,
            JobRecord {
                snapshot: snapshot.clone(),
                base_bytes,
                target,
                profile_index,
                cancel_requested: Arc::new(std::sync::atomic::AtomicBool::new(false)),
                caller_token,
                messages: Vec::new(),
                message_bytes: 0,
                committing: false,
            },
        );
        self.queue.push_back(id);
        Ok(snapshot)
    }

    /// Добавляет сообщение живой записи под бюджетом
    /// `max_message_bytes_per_job`. Бюджет кумулятивный: dren истории не
    /// возвращает байты, поэтому суммарная память сообщений задания
    /// ограничена независимо от числа чтений.
    pub fn push_message(&mut self, id: JobId, message: UserMessageDto) -> Result<(), HostError> {
        let Some(record) = self.records.get_mut(&id) else {
            // Запись уже terminal (гонка с отменой/закрытием): поздние
            // сообщения отвергаются, история неизменяема.
            return Err(HostError::new(
                HostErrorCode::JobExpired,
                "задание уже завершено и не принимает сообщения",
            ));
        };
        let bytes = message.byte_size();
        if record.message_bytes + bytes > self.config.max_message_bytes_per_job {
            return Err(HostError::new(
                HostErrorCode::ResourceLimit,
                "исчерпан бюджет сообщений задания",
            ));
        }
        record.message_bytes += bytes;
        record.messages.push(message);
        Ok(())
    }

    /// Сообщения задания: живая запись либо история. `remove` атомарно
    /// забирает FIFO-префикс у обеих (ИЗМЕРЕНО, `JOB.MESSAGES`).
    /// `None` — задание неизвестно или вытеснено.
    pub fn take_messages(&mut self, id: JobId, remove: bool) -> Option<Vec<UserMessageDto>> {
        if let Some(record) = self.records.get_mut(&id) {
            return Some(if remove {
                std::mem::take(&mut record.messages)
            } else {
                record.messages.clone()
            });
        }
        let messages = self.history_messages.get_mut(&id)?;
        Some(if remove {
            std::mem::take(messages)
        } else {
            messages.clone()
        })
    }

    /// Забирает право на terminal transition: драйвер, получивший
    /// `true`, публикует write-set и завершает запись сам, а поломка
    /// runtime её больше не перехватывает. `false` — публиковать нельзя:
    /// запись уже terminal, вытеснена, заклеймлена другим либо runtime
    /// закрыт/сломан. В последнем случае право не выдаётся и запись
    /// СРАЗУ завершается «Отменено»: закрытие обязано откатывать
    /// write-set (матрица публикаций плана), а не отдавать его наружу
    /// вместе с `Completed`.
    pub fn claim_terminal(&mut self, id: JobId, end: Option<bsl_rt::BslDate>) -> bool {
        if matches!(self.state, RuntimeState::Closed | RuntimeState::Broken) {
            self.finish(id, JobStateDto::Canceled, end, None);
            return false;
        }
        let Some(record) = self.records.get_mut(&id) else {
            return false;
        };
        if record.committing || record.snapshot.state.is_terminal() {
            return false;
        }
        record.committing = true;
        true
    }

    /// Задание известно реестру: живая запись либо строка истории.
    pub fn knows(&self, id: JobId) -> bool {
        self.records.contains_key(&id) || self.history_messages.contains_key(&id)
    }

    pub fn record(&self, id: JobId) -> Option<&JobRecord> {
        self.records.get(&id)
    }

    /// Все живые записи — для завершения runtime.
    pub fn records(&self) -> impl Iterator<Item = &JobRecord> {
        self.records.values()
    }

    pub fn record_mut(&mut self, id: JobId) -> Option<&mut JobRecord> {
        self.records.get_mut(&id)
    }

    /// Terminal transition: ровно один раз — повторный вызов no-op с
    /// `false`. Освобождает admission-резервы и ключ, переносит байты
    /// записи из live-бюджета в history-бюджет без двойного учёта,
    /// ограничивает диагностику бюджетом ошибки и вытесняет старейшие
    /// terminal-записи (амортизировано, на добавлении).
    pub fn finish(
        &mut self,
        id: JobId,
        state: JobStateDto,
        end: Option<bsl_rt::BslDate>,
        error: Option<JobErrorDto>,
    ) -> bool {
        debug_assert!(state.is_terminal());
        let Some(mut record) = self.records.remove(&id) else {
            return false;
        };
        if record.snapshot.state.is_terminal() {
            self.records.insert(id, record);
            return false;
        }
        let error = error.map(|error| error.bounded(self.config.max_error_bytes_per_job));
        let error_bytes = error.as_ref().map_or(0, JobErrorDto::byte_size);
        record.snapshot.state = state;
        record.snapshot.end = end;
        record.snapshot.error = error.map(Arc::new);
        let messages = std::mem::take(&mut record.messages);
        let message_bytes: usize = messages.iter().map(UserMessageDto::byte_size).sum();
        self.inflight -= 1;
        self.live_payload_bytes -= record.base_bytes;
        self.keys.retain(|(_, owner)| *owner != id);
        self.queue.retain(|queued| *queued != id);
        // Размер записи истории считает все составляющие: payload, ключ,
        // строки снимка, ограниченную ошибку и сообщения. Он хранится
        // рядом со снимком — вытеснение вычитает ровно его.
        let record_bytes = record.base_bytes + error_bytes + message_bytes;
        debug_assert!(record_bytes <= self.config.max_single_job_record_bytes);
        self.history_bytes += record_bytes;
        self.history
            .push_back((record_bytes, Arc::new(record.snapshot)));
        self.history_messages.insert(id, messages);
        while self.history.len() > self.config.max_history_jobs
            || self.history_bytes > self.config.max_history_bytes
        {
            let Some((evicted_bytes, evicted)) = self.history.pop_front() else {
                break;
            };
            self.history_bytes -= evicted_bytes;
            self.history_messages.remove(&evicted.id);
        }
        true
    }

    /// Снимок задания: живая запись либо история.
    pub fn snapshot(&self, id: JobId) -> Option<Arc<JobSnapshotDto>> {
        if let Some(record) = self.records.get(&id) {
            return Some(Arc::new(record.snapshot.clone()));
        }
        self.history
            .iter()
            .find(|(_, snapshot)| snapshot.id == id)
            .map(|(_, snapshot)| Arc::clone(snapshot))
    }

    /// Все снимки: живые + история. Возвращает `Arc`-указатели — фильтр
    /// работает вне лока.
    pub fn snapshots(&self) -> Vec<Arc<JobSnapshotDto>> {
        let mut all: Vec<Arc<JobSnapshotDto>> = self
            .records
            .values()
            .map(|record| Arc::new(record.snapshot.clone()))
            .collect();
        all.extend(
            self.history
                .iter()
                .map(|(_, snapshot)| Arc::clone(snapshot)),
        );
        all
    }

    /// Тестовая поверхность admission-инвариантов.
    #[cfg(test)]
    pub fn history_len(&self) -> usize {
        self.history.len()
    }

    /// Занятые байты live-бюджета — тестовая поверхность освобождения
    /// резервов.
    #[cfg(test)]
    pub fn live_payload_bytes(&self) -> usize {
        self.live_payload_bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bsl_rt::{GraphLimits, RuntimeShapes};

    fn graph(bytes_hint: usize) -> Arc<SerializedValueGraph> {
        let rt = RuntimeShapes::seeded(Vec::new(), Vec::new(), None);
        let value = bsl_rt::BslValue::Str(bsl_rt::BslString::from_str(&"ы".repeat(bytes_hint / 2)));
        Arc::new(
            SerializedValueGraph::capture(&[value], &rt, &GraphLimits::default()).expect("снимок"),
        )
    }

    fn id(byte: u8) -> JobId {
        JobId([byte; 16])
    }

    #[test]
    fn admission_reserves_and_finish_releases() {
        let mut registry = JobRegistry::new(BackgroundJobConfig {
            max_inflight_jobs: 2,
            ..BackgroundJobConfig::default()
        });
        let params = graph(64);
        registry
            .admit(
                id(1),
                "М.Ф".into(),
                (0, 1),
                params.clone(),
                None,
                None,
                None,
                0,
            )
            .map_err(|_| ())
            .expect("первое задание принято");
        registry
            .admit(
                id(2),
                "М.Ф".into(),
                (0, 1),
                params.clone(),
                None,
                None,
                None,
                0,
            )
            .map_err(|_| ())
            .expect("второе задание принято");
        assert!(matches!(
            registry.admit(
                id(3),
                "М.Ф".into(),
                (0, 1),
                params.clone(),
                None,
                None,
                None,
                0,
            ),
            Err(AdmissionError::ResourceLimit(_))
        ));
        assert!(registry.finish(id(1), JobStateDto::Completed, None, None));
        assert!(
            !registry.finish(id(1), JobStateDto::Failed, None, None),
            "terminal transition ровно один раз"
        );
        registry
            .admit(id(3), "М.Ф".into(), (0, 1), params, None, None, None, 0)
            .map_err(|_| ())
            .expect("слот освобождён");
        assert_eq!(registry.history_len(), 1);
        assert_eq!(
            registry
                .snapshot(id(1))
                .expect("история хранит снимок")
                .state,
            JobStateDto::Completed
        );
    }

    #[test]
    fn a_duplicate_key_is_rejected_until_terminal() {
        let mut registry = JobRegistry::new(BackgroundJobConfig::default());
        let params = graph(16);
        let key = {
            let rt = RuntimeShapes::seeded(Vec::new(), Vec::new(), None);
            Arc::new(JobKeyDto {
                graph: SerializedValueGraph::capture(
                    &[bsl_rt::BslValue::Boolean(true)],
                    &rt,
                    &GraphLimits::default(),
                )
                .expect("снимок ключа"),
            })
        };
        registry
            .admit(
                id(1),
                "М.Ф".into(),
                (0, 1),
                params.clone(),
                Some(key.clone()),
                None,
                None,
                0,
            )
            .map_err(|_| ())
            .expect("первое принято");
        assert!(matches!(
            registry.admit(
                id(2),
                "М.Ф".into(),
                (0, 1),
                params.clone(),
                Some(key.clone()),
                None,
                None,
                0,
            ),
            Err(AdmissionError::DuplicateKey)
        ));
        // Тот же ключ у ДРУГОЙ цели — не дубль.
        registry
            .admit(
                id(3),
                "М.Д".into(),
                (0, 2),
                params.clone(),
                Some(key.clone()),
                None,
                None,
                0,
            )
            .map_err(|_| ())
            .expect("другая цель принята");
        registry.finish(id(1), JobStateDto::Canceled, None, None);
        registry
            .admit(
                id(4),
                "М.Ф".into(),
                (0, 1),
                params,
                Some(key),
                None,
                None,
                0,
            )
            .map_err(|_| ())
            .expect("ключ освобождён terminal transition");
    }

    #[test]
    fn history_eviction_is_amortized_on_insert() {
        let mut registry = JobRegistry::new(BackgroundJobConfig {
            max_history_jobs: 2,
            ..BackgroundJobConfig::default()
        });
        let params = graph(16);
        for i in 1..=4u8 {
            registry
                .admit(
                    id(i),
                    "М.Ф".into(),
                    (0, 1),
                    params.clone(),
                    None,
                    None,
                    None,
                    0,
                )
                .map_err(|_| ())
                .expect("принято");
            registry.finish(id(i), JobStateDto::Completed, None, None);
        }
        assert_eq!(registry.history_len(), 2);
        assert!(registry.snapshot(id(1)).is_none(), "старейшие вытеснены");
        assert!(registry.snapshot(id(4)).is_some());
    }

    /// Размер записи истории считает все составляющие: payload, ключ,
    /// строки снимка, ограниченную ошибку и сообщения; live-бюджет
    /// резервирует базу и освобождается на terminal.
    #[test]
    fn record_bytes_count_key_strings_error_and_messages() {
        let config = BackgroundJobConfig {
            max_error_bytes_per_job: 200,
            max_message_bytes_per_job: 300,
            ..BackgroundJobConfig::default()
        };
        let mut registry = JobRegistry::new(config);
        let params = graph(64);
        let key = {
            let rt = RuntimeShapes::seeded(Vec::new(), Vec::new(), None);
            Arc::new(JobKeyDto {
                graph: SerializedValueGraph::capture(
                    &[bsl_rt::BslValue::Str(bsl_rt::BslString::from_str("ключ"))],
                    &rt,
                    &GraphLimits::default(),
                )
                .expect("снимок ключа"),
            })
        };
        registry
            .admit(
                id(1),
                "Модуль.Метод".into(),
                (0, 1),
                params.clone(),
                Some(key.clone()),
                Some("наименование".into()),
                None,
                0,
            )
            .map_err(|_| ())
            .expect("принято");
        let base = params.byte_size()
            + key.graph.byte_size()
            + "Модуль.Метод".len()
            + "наименование".len();
        assert_eq!(registry.live_payload_bytes(), base, "резерв базы записи");

        registry
            .push_message(id(1), UserMessageDto::from_text("привет"))
            .expect("сообщение в бюджете");
        let over = "м".repeat(300);
        let error = registry
            .push_message(id(1), UserMessageDto::from_text(over))
            .expect_err("кумулятивный бюджет сообщений");
        assert_eq!(error.code, HostErrorCode::ResourceLimit);

        let huge_error = JobErrorDto::from_text("о".repeat(400));
        assert!(registry.finish(id(1), JobStateDto::Failed, None, Some(huge_error)));
        assert_eq!(registry.live_payload_bytes(), 0, "live-бюджет освобождён");
        let snapshot = registry.snapshot(id(1)).expect("снимок истории");
        let bounded = snapshot.error.as_ref().expect("ошибка снимка");
        assert!(bounded.diagnostic_truncated, "ошибка ограничена бюджетом");
        assert!(bounded.byte_size() <= 200);
        let message_bytes = "привет".len();
        // history_bytes недоступен снаружи, но вытеснение по нему
        // проверяемо: добавляем записи, пока не вытеснится первая.
        assert!(registry.knows(id(1)));
        let drained = registry
            .take_messages(id(1), true)
            .expect("сообщения terminal-записи");
        assert_eq!(drained.len(), 1);
        assert_eq!(drained[0].byte_size(), message_bytes);
        assert!(
            registry
                .take_messages(id(1), false)
                .expect("повторное чтение")
                .is_empty(),
            "drain terminal-записи атомарен"
        );
    }

    /// Вытеснение по byte-бюджету истории вычитает ровно то, что
    /// добавлялось, и уносит сообщения вытесненной записи.
    #[test]
    fn history_byte_eviction_subtracts_what_was_added() {
        let params_bytes = graph(64).byte_size();
        let single = params_bytes + "М.Ф".len() + 200 + 300;
        let mut registry = JobRegistry::new(BackgroundJobConfig {
            max_error_bytes_per_job: 200,
            max_message_bytes_per_job: 300,
            max_single_job_record_bytes: single,
            // Помещаются ровно две записи без ошибок и сообщений.
            max_history_bytes: 2 * (params_bytes + "М.Ф".len()),
            ..BackgroundJobConfig::default()
        });
        for i in 1..=3u8 {
            registry
                .admit(id(i), "М.Ф".into(), (0, 1), graph(64), None, None, None, 0)
                .map_err(|_| ())
                .expect("принято");
            registry.finish(id(i), JobStateDto::Completed, None, None);
        }
        assert_eq!(registry.history_len(), 2, "третья запись вытеснила первую");
        assert!(
            !registry.knows(id(1)),
            "сообщения вытеснены вместе с записью"
        );
        assert!(registry.knows(id(2)) && registry.knows(id(3)));
    }

    #[test]
    fn an_invalid_config_is_rejected() {
        let config = BackgroundJobConfig {
            workers: Some(0),
            ..BackgroundJobConfig::default()
        };
        assert!(config.validate().is_err());
        let config = BackgroundJobConfig {
            max_staged_temp_bytes_per_job: 2,
            max_live_staged_temp_bytes: 1,
            ..BackgroundJobConfig::default()
        };
        assert!(config.validate().is_err());
        assert!(BackgroundJobConfig::default().validate().is_ok());
    }

    /// Публикация write-set и поломка runtime координируются claim'ом:
    /// кто забрал terminal transition, тот и определяет исход. Драйвер с
    /// claim публикует и завершает сам (поломка его запись не трогает);
    /// проигравший claim драйвер не публикует вовсе — задание остаётся
    /// `Failed(RuntimeBroken)` без публикации, как требует матрица
    /// «инфраструктурный сбой — rollback».
    #[test]
    fn a_claimed_terminal_transition_is_not_stolen_by_a_broken_runtime() {
        let mut registry = JobRegistry::new(BackgroundJobConfig::default());
        let publisher = id(1);
        let bystander = id(2);
        for job in [publisher, bystander] {
            registry
                .admit(
                    job,
                    "Модуль.Метод".to_string(),
                    (0, 1),
                    graph(64),
                    None,
                    None,
                    None,
                    0,
                )
                .ok()
                .expect("задание принято");
        }
        // Драйвер забрал право на публикацию первого задания.
        assert!(registry.claim_terminal(publisher, None));
        assert!(
            !registry.claim_terminal(publisher, None),
            "повторный claim не выдаётся"
        );

        // Поломка runtime роняет ВСЕ живые записи, кроме заклеймленной.
        fail_all_resident(&mut registry);
        assert_eq!(
            registry.snapshot(bystander).expect("снимок соседа").state,
            JobStateDto::Failed,
            "незаклеймленное задание роняет поломка"
        );
        assert!(
            !registry
                .snapshot(publisher)
                .expect("снимок публикующего")
                .state
                .is_terminal(),
            "заклеймленное задание поломка не трогает — иначе публикация \
             состоялась бы у Failed(RuntimeBroken)"
        );

        // Владелец claim доводит своё задание сам.
        assert!(registry.finish(publisher, JobStateDto::Completed, None, None));
        assert_eq!(
            registry.snapshot(publisher).expect("снимок").state,
            JobStateDto::Completed
        );

        // Обратный порядок: поломка выиграла — claim больше не выдаётся,
        // и драйвер не публикует.
        let loser = id(3);
        registry
            .admit(
                loser,
                "Модуль.Метод".to_string(),
                (0, 1),
                graph(64),
                None,
                None,
                None,
                0,
            )
            .ok()
            .expect("задание принято");
        fail_all_resident(&mut registry);
        assert!(
            !registry.claim_terminal(loser, None),
            "у завершённого поломкой задания claim не берётся"
        );
    }

    /// Переполняющие значения публичной конфигурации отвергаются, а не
    /// паникуют: сумма бюджетов считается `checked_add`.
    #[test]
    fn an_overflowing_config_is_rejected_without_panic() {
        let config = BackgroundJobConfig {
            max_error_bytes_per_job: usize::MAX,
            max_message_bytes_per_job: 2,
            ..BackgroundJobConfig::default()
        };
        assert!(config.validate().is_err());
    }

    /// Admission с непровалидированной переполняющей конфигурацией
    /// отвечает ловимым отказом ресурса, а не паникой debug-сборки.
    #[test]
    fn an_overflowing_admission_sum_is_a_resource_limit() {
        // Реестр строится напрямую, минуя validate, — так admission
        // обязан выдержать даже несогласованные значения.
        let mut registry = JobRegistry::new(BackgroundJobConfig {
            max_error_bytes_per_job: usize::MAX,
            max_message_bytes_per_job: usize::MAX,
            max_single_job_record_bytes: usize::MAX,
            max_history_bytes: usize::MAX,
            ..BackgroundJobConfig::default()
        });
        let error = registry
            .admit(
                id(1),
                "Модуль.Метод".to_string(),
                (0, 1),
                graph(64),
                None,
                None,
                None,
                0,
            )
            .expect_err("переполнение суммы — отказ, не паника");
        assert!(matches!(error, AdmissionError::ResourceLimit(_)));
    }
}

// --- Host-профили сеансов заданий --------------------------------------

/// Непрозрачный идентификатор host-профиля фоновых заданий, выданный
/// [`crate::EngineBuilder::register_host_profile`]. Привязан к движку,
/// зарегистрировавшему профиль: идентификатор чужого движка отвергается
/// при выборе профиля ошибкой, без fallback на process-профиль.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HostProfileId {
    pub(crate) engine: u64,
    pub(crate) index: u32,
}

/// Фабрика host-окружений сеансов фоновых заданий. Вызывается в потоке
/// worker для каждого задания своего профиля; `Send + Sync` — одна
/// фабрика разделяется всем пулом, а непереносимые сервисы сеанса
/// (`Rc`-обёртки ФС, часов, сети) строятся внутри вызова и поток worker
/// не покидают. Foreground-сервисы вызывающего `State` — его `files`,
/// `network`, часы и вывод — в worker не копируются: профиль и есть их
/// именованная замена.
pub trait BackgroundStateFactory: Send + Sync {
    /// Настраивает сеанс задания. Builder приходит с process-default
    /// сервисами worker-движка (стандартные компоненты плюс
    /// пользовательские библиотеки родительского движка); фабрика
    /// ограничивает или подменяет их — например `deny_network` или
    /// изолированная файловая система.
    ///
    /// # Errors
    ///
    /// Текст причины: задание завершается `Failed` с кодом
    /// `HostProfileUnavailable`, worker остаётся жив.
    fn configure(&self, builder: crate::StateBuilder) -> Result<crate::StateBuilder, String>;
}

// --- Пул workers -------------------------------------------------------

use std::sync::{Condvar, Mutex};

/// Экспортная поверхность каталога для разрешения целей submit:
/// снимается один раз при создании runtime, чтобы каждый запуск задания
/// не разбирал текстовый образ заново.
pub(crate) struct TargetTable {
    modules: Vec<(String, Vec<TargetFunction>)>,
}

struct TargetFunction {
    name: String,
    chunk: u16,
    exported: bool,
    is_async: bool,
}

impl TargetTable {
    fn from_catalog(catalog: &bsl_bytecode::ConfigurationProgram) -> Self {
        Self {
            modules: catalog
                .modules
                .iter()
                .map(|module| {
                    let functions = module
                        .program
                        .function_names
                        .iter()
                        .enumerate()
                        .map(|(i, name)| TargetFunction {
                            name: name.clone(),
                            chunk: (i + 1) as u16,
                            exported: module.program.exported_functions[i],
                            is_async: module.program.chunks[i + 1].is_async,
                        })
                        .collect();
                    (module.name.clone(), functions)
                })
                .collect(),
        }
    }

    /// Разрешает «Модуль.Метод»: неглобальный общий модуль, экспортная
    /// не-async цель. Детали validation — за `JOB.EXECUTE.VALIDATION`.
    fn resolve(&self, method_name: &str) -> Result<(u32, u16), String> {
        let Some((module_name, function_name)) = method_name.split_once('.') else {
            return Err(format!("цель «{method_name}» не в форме «Модуль.Метод»"));
        };
        let module_name = module_name.trim();
        let function_name = function_name.trim();
        let Some((module_index, (_, functions))) = self
            .modules
            .iter()
            .enumerate()
            .find(|(_, (name, _))| bsl_rt::folded_eq(name, module_name))
        else {
            return Err(format!("общий модуль «{module_name}» не найден"));
        };
        let Some(function) = functions
            .iter()
            .find(|function| bsl_rt::folded_eq(&function.name, function_name))
        else {
            return Err(format!(
                "в модуле «{module_name}» нет метода «{function_name}»"
            ));
        };
        if !function.exported {
            return Err(format!(
                "метод «{module_name}.{function_name}» не экспортирован"
            ));
        }
        if function.is_async {
            // НЕ ИЗМЕРЕНО(JOB.ASYNC.TARGET): ИЗМЕРЕНО на файловой базе
            // (2026-08-27), что «Асинх» в серверном общем модуле ломает
            // инициализацию ВСЕГО модуля при первом обращении, то есть
            // легальной async-цели у платформы нет; клиент-серверное
            // подтверждение и критерий завершения — за следующей сессией.
            // Здесь отказ синхронный: у платформы падает не submit, а
            // любой вызов отравленного модуля — расхождение осознанное.
            return Err(
                "асинхронная цель фонового задания не поддержана до замера JOB.ASYNC.TARGET"
                    .to_string(),
            );
        }
        Ok((module_index as u32, function.chunk))
    }
}

/// `Send`-рецепт worker: текстовый образ каталога, статические
/// `LibraryDescriptor` пользовательских библиотек и символы
/// препроцессора. Скрытого второго формата нет — worker разбирает тот же
/// публичный `BytecodeImage::Configuration`, что пишет `--emit-bytecode`,
/// один раз, и разделяет разобранные программы между своими сеансами.
#[derive(Clone)]
pub(crate) struct WorkerRecipe {
    pub image_text: Arc<str>,
    /// Библиотеки, добавленные `register_library` родительского движка:
    /// без них цель с пользовательским типом не слинкуется в worker.
    pub libraries: Vec<bsl_rt::LibraryDescriptor>,
    /// Символы условной компиляции — их видит динамический код задания.
    pub symbols: bsl_syntax::PreprocSymbols,
}

/// Разделяемое состояние runtime: реестр под синхронным мьютексом и два
/// условия — «есть работа» для workers и «есть terminal» для ожидающих.
/// Ни одно внешнее действие под локом не выполняется.
pub(crate) struct JobRuntimeShared {
    pub registry: Mutex<JobRegistry>,
    pub work_available: Condvar,
    pub terminal_watch: Condvar,
    pub recipe: WorkerRecipe,
    pub id_source: Arc<dyn JobIdSource>,
    pub time_source: Arc<dyn JobTimeSource>,
    pub targets: TargetTable,
    /// Реестр mailbox'ов временного хранилища родительского движка —
    /// публикации write-set'ов заданий идут сюда.
    pub temp_hub: Arc<bsl_rt::TempStorageHub>,
    /// Копия конфигурации для чтения лимитов без лока реестра.
    pub config: BackgroundJobConfig,
    /// Фабрики host-профилей движка: индекс записи + 1 — это
    /// `JobRecord::profile_index`; 0 — системный профиль без фабрики.
    pub profiles: Arc<[Arc<dyn BackgroundStateFactory>]>,
    /// Глобальный staging-бюджет временного хранилища всех живых заданий
    /// (`max_live_staged_temp_bytes`).
    pub staging_global: Arc<GlobalStagingBudget>,
    /// Внешний sink представления сообщений заданий, если host его
    /// зарегистрировал; история записи пишется до него и не теряется при
    /// его backpressure.
    pub message_display: Option<Arc<dyn bsl_rt::UserMessageSink + Send + Sync>>,
    /// OS-потоки пула. Список живёт в разделяемом состоянии, потому что
    /// соседей спавнит бутстраппер с потока первого worker; порядок локов
    /// всюду один — сначала этот список, затем реестр.
    pub threads: Mutex<Vec<std::thread::JoinHandle<()>>>,
    /// Тестовый шлюз бутстрапа: пока держится, первый worker не спавнит
    /// соседей и не берёт работу — тест наблюдает возврат первого
    /// admission до запуска пула. Продакшен-путь шлюз не выставляет.
    pub bootstrap_gate: Mutex<Option<Arc<BootstrapGate>>>,
}

/// Тестовый шлюз бутстрапа пула: флаг «удержан» под мьютексом и condvar
/// освобождения.
pub(crate) type BootstrapGate = (Mutex<bool>, Condvar);

impl JobRuntimeShared {
    /// Все перечисленные задания terminal (неизвестные считаются
    /// вытесненными и потому terminal).
    fn all_terminal(registry: &JobRegistry, ids: &[JobId]) -> bool {
        ids.iter().all(|id| {
            registry
                .snapshot(*id)
                .is_none_or(|snapshot| snapshot.state.is_terminal())
        })
    }

    /// Admission нового задания в общий реестр. Пул не поднимает: это
    /// обязанность внешнего `JobRuntime::submit`; вложенный submit из
    /// worker приходит, когда потоки уже работают.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn submit_shared(
        &self,
        method_name: &str,
        target: (u32, u16),
        params: Arc<SerializedValueGraph>,
        key: Option<Arc<JobKeyDto>>,
        description: Option<String>,
        caller_token: Option<[u8; 16]>,
        profile_index: u32,
    ) -> Result<JobSnapshotDto, HostError> {
        let id = self.id_source.next_id();
        let mut registry = self.registry.lock().expect("реестр без отравления");
        let snapshot = registry
            .admit(
                id,
                method_name.to_string(),
                target,
                params,
                key,
                description,
                caller_token,
                profile_index,
            )
            .map_err(|error| match error {
                AdmissionError::ResourceLimit(text) => {
                    HostError::new(HostErrorCode::ResourceLimit, text)
                }
                AdmissionError::DuplicateKey => HostError::new(
                    HostErrorCode::InvalidCall,
                    "задание с таким ключом уже активно",
                ),
                AdmissionError::Unavailable(state) => unavailable_error(state),
            })?;
        if registry.state == RuntimeState::Cold {
            registry.state = RuntimeState::Starting;
        }
        Ok(snapshot)
    }

    /// То же по имени «Модуль.Метод» — для вложенного submit из worker.
    pub(crate) fn submit_by_name_shared(
        &self,
        method_name: &str,
        params: Arc<SerializedValueGraph>,
        key: Option<Arc<JobKeyDto>>,
        description: Option<String>,
        caller_token: Option<[u8; 16]>,
        profile_index: u32,
    ) -> Result<JobSnapshotDto, HostError> {
        let target = self
            .targets
            .resolve(method_name)
            .map_err(|text| HostError::new(HostErrorCode::InvalidCall, text))?;
        let snapshot = self.submit_shared(
            method_name,
            target,
            params,
            key,
            description,
            caller_token,
            profile_index,
        )?;
        self.work_available.notify_one();
        // Helping-ожидание спит на terminal_watch без таймера: новое
        // задание в очереди — тоже его событие (появился кандидат на
        // доводку).
        self.terminal_watch.notify_all();
        Ok(snapshot)
    }

    /// Доступность live-методов: закрытый и сломанный runtime отвечают
    /// типизированной ловимой ошибкой, а не тихим no-op; свойства уже
    /// материализованных снимков при этом остаются читаемыми.
    fn live_guard(registry: &JobRegistry) -> Result<(), HostError> {
        match registry.state {
            RuntimeState::Closed | RuntimeState::Broken => Err(unavailable_error(registry.state)),
            _ => Ok(()),
        }
    }

    /// Задание известно либо ошибка `JobExpired`: live-методы вытесненного
    /// снимка не притворяются успешными и не скрывают потерю истории
    /// пустым результатом.
    fn known_guard(registry: &JobRegistry, id: JobId) -> Result<(), HostError> {
        if registry.knows(id) {
            return Ok(());
        }
        Err(HostError::new(
            HostErrorCode::JobExpired,
            "задание неизвестно: вытеснено из истории либо никогда не существовало",
        ))
    }

    /// Отмена задания. `Queued` завершается сразу; `Running` получает
    /// взведённый флаг и завершится на границе кванта; повторная отмена и
    /// отмена terminal — успешный no-op (ИЗМЕРЕНО, `JOB.CANCEL.RACES`).
    /// Вытесненное из истории задание — ловимая `JobExpired`. Отмена — не
    /// ошибка BSL: снимок получает «Отменено» без `ИнформацияОбОшибке`.
    pub(crate) fn cancel(&self, id: JobId, end: Option<bsl_rt::BslDate>) -> Result<(), HostError> {
        let mut registry = self.registry.lock().expect("реестр без отравления");
        Self::live_guard(&registry)?;
        Self::known_guard(&registry, id)?;
        let Some(record) = registry.record(id) else {
            // Запись в истории: terminal, no-op.
            return Ok(());
        };
        match record.snapshot.state {
            JobStateDto::Queued => {
                registry.finish(id, JobStateDto::Canceled, end, None);
                drop(registry);
                self.terminal_watch.notify_all();
            }
            JobStateDto::Running => {
                record
                    .cancel_requested
                    .store(true, std::sync::atomic::Ordering::Relaxed);
                // Точно спящий worker обязан проснуться и опросить
                // резидента со взведённым флагом: припаркованный на
                // host-операции poll вернёт Canceled, не дожидаясь
                // ответа транспорта.
                registry.wake_epoch += 1;
                drop(registry);
                self.work_available.notify_all();
                self.terminal_watch.notify_all();
            }
            _ => {}
        }
        Ok(())
    }

    /// Свежие снимки всех `ids`, удержанные под текущим локом реестра.
    /// Задание, известное на входе ожидания, но исчезнувшее к этому
    /// моменту, вытеснено из истории — ловимая `JobExpired`, устаревший
    /// снимок за свежий не выдаётся.
    fn held_snapshots(
        registry: &JobRegistry,
        ids: &[JobId],
    ) -> Result<Vec<Arc<JobSnapshotDto>>, HostError> {
        ids.iter()
            .map(|id| {
                registry.snapshot(*id).ok_or_else(|| {
                    HostError::new(
                        HostErrorCode::JobExpired,
                        "задание вытеснено из истории во время ожидания",
                    )
                })
            })
            .collect()
    }

    /// Флаги семантики менеджерного ожидания по снимкам, удержанным под
    /// одним локом: (есть активные, есть изменившиеся, все terminal,
    /// есть аварийные).
    fn first_change_flags(
        jobs: &[(JobId, bsl_rt::JobStateDto)],
        held: &[Arc<JobSnapshotDto>],
    ) -> (bool, bool, bool, bool) {
        let mut any_active = false;
        let mut any_changed = false;
        let mut all_terminal = true;
        let mut any_failed = false;
        for ((_, initial), snapshot) in jobs.iter().zip(held) {
            let state = snapshot.state;
            if !state.is_terminal() {
                any_active = true;
                all_terminal = false;
            }
            if state != *initial {
                any_changed = true;
            }
            if state == JobStateDto::Failed {
                any_failed = true;
            }
        }
        (any_active, any_changed, all_terminal, any_failed)
    }

    /// Ожидание по семантике синтакс-помощника: активных нет — сразу;
    /// с таймаутом — до первого изменения статуса; без — до завершения
    /// всех либо первого аварийного. Возвращает свежие снимки всех
    /// `jobs`, удержанные под финальным локом, — размер результата равен
    /// размеру запроса, вытеснение во время ожидания — `JobExpired`.
    pub(crate) fn wait_first_change(
        &self,
        jobs: &[(JobId, bsl_rt::JobStateDto)],
        timeout: Option<Duration>,
    ) -> Result<Vec<Arc<JobSnapshotDto>>, HostError> {
        let deadline = timeout.map(|t| std::time::Instant::now() + t);
        let ids: Vec<JobId> = jobs.iter().map(|(id, _)| *id).collect();
        let mut registry = self.registry.lock().expect("реестр без отравления");
        Self::live_guard(&registry)?;
        for id in &ids {
            Self::known_guard(&registry, *id)?;
        }
        loop {
            // Закрытие во время ожидания: ответ уже не придёт штатно —
            // ловимая ошибка вместо вечного сна на отсоединённом worker
            // (симметрично `wait_terminal_blocking`).
            Self::live_guard(&registry)?;
            let held = Self::held_snapshots(&registry, &ids)?;
            let (any_active, any_changed, all_terminal, any_failed) =
                Self::first_change_flags(jobs, &held);
            if !any_active {
                return Ok(held);
            }
            match deadline {
                Some(_) if any_changed => return Ok(held),
                None if all_terminal || any_failed => return Ok(held),
                _ => {}
            }
            match deadline {
                None => {
                    registry = self
                        .terminal_watch
                        .wait(registry)
                        .expect("реестр без отравления");
                }
                Some(deadline) => {
                    let now = std::time::Instant::now();
                    let Some(left) = deadline.checked_duration_since(now) else {
                        return Ok(held);
                    };
                    let (guard, _) = self
                        .terminal_watch
                        .wait_timeout(registry, left)
                        .expect("реестр без отравления");
                    registry = guard;
                }
            }
        }
    }

    /// Сообщения задания: живая запись и terminal-история отдают (и при
    /// `remove` атомарно забирают) свой FIFO — ИЗМЕРЕНО (`JOB.MESSAGES`).
    /// Вытесненное задание — ловимая `JobExpired`.
    pub(crate) fn take_messages(
        &self,
        id: JobId,
        remove: bool,
    ) -> Result<Vec<UserMessageDto>, HostError> {
        let mut registry = self.registry.lock().expect("реестр без отравления");
        Self::live_guard(&registry)?;
        registry.take_messages(id, remove).ok_or_else(|| {
            HostError::new(
                HostErrorCode::JobExpired,
                "задание неизвестно: вытеснено из истории либо никогда не существовало",
            )
        })
    }

    /// Блокирующее ожидание terminal-состояния всех `ids` — путь
    /// foreground-сеанса; worker вместо блокировки помогает пулу (см.
    /// `WorkerJobService`). Неизвестное на входе задание — `JobExpired`;
    /// задание, вытесненное УЖЕ ВО ВРЕМЯ ожидания, terminal, но его
    /// свежего снимка больше нет — тоже `JobExpired`, а не устаревший
    /// снимок. Снимки результата удерживаются под финальным локом.
    pub(crate) fn wait_terminal_blocking(
        &self,
        ids: &[JobId],
        timeout: Option<Duration>,
    ) -> Result<bsl_rt::JobWaitOutcome, HostError> {
        let deadline = timeout.map(|t| std::time::Instant::now() + t);
        let mut registry = self.registry.lock().expect("реестр без отравления");
        Self::live_guard(&registry)?;
        for id in ids {
            Self::known_guard(&registry, *id)?;
        }
        loop {
            if Self::all_terminal(&registry, ids) {
                return Ok(bsl_rt::JobWaitOutcome {
                    completed: true,
                    snapshots: Self::held_snapshots(&registry, ids)?,
                });
            }
            // Закрытие во время ожидания: ответ уже не придёт штатно —
            // ловимая ошибка вместо вечного сна на отсоединённом worker.
            Self::live_guard(&registry)?;
            match deadline {
                None => {
                    registry = self
                        .terminal_watch
                        .wait(registry)
                        .expect("реестр без отравления");
                }
                Some(deadline) => {
                    let now = std::time::Instant::now();
                    let Some(left) = deadline.checked_duration_since(now) else {
                        return Ok(bsl_rt::JobWaitOutcome {
                            completed: false,
                            snapshots: Self::held_snapshots(&registry, ids)?,
                        });
                    };
                    let (guard, result) = self
                        .terminal_watch
                        .wait_timeout(registry, left)
                        .expect("реестр без отравления");
                    registry = guard;
                    if result.timed_out() {
                        // Последняя проверка под локом — событие могло
                        // прийти на границе таймаута.
                        return Ok(bsl_rt::JobWaitOutcome {
                            completed: Self::all_terminal(&registry, ids),
                            snapshots: Self::held_snapshots(&registry, ids)?,
                        });
                    }
                }
            }
        }
    }
}

/// Нативный runtime фоновых заданий одного `Engine`. Клоны `Engine`
/// разделяют runtime; OS-потоки поднимаются лениво при первом успешном
/// admission — движок без заданий потоков не создаёт.
pub struct JobRuntime {
    shared: Arc<JobRuntimeShared>,
    workers: usize,
}

/// UUID v4 из ключей ОС — общий генератор идентификаторов и токенов.
pub(crate) fn random_uuid() -> [u8; 16] {
    use std::hash::{BuildHasher, Hasher};
    let mut bytes = [0u8; 16];
    for chunk in bytes.chunks_mut(8) {
        let state = std::collections::hash_map::RandomState::new();
        let value = state.build_hasher().finish().to_le_bytes();
        chunk.copy_from_slice(&value[..chunk.len()]);
    }
    bsl_rt::uuid::v4_from_bytes(bytes)
}

/// Стандартный источник идентификаторов: OS random + UUID v4.
struct SystemJobIds;

impl JobIdSource for SystemJobIds {
    fn next_id(&self) -> JobId {
        JobId(random_uuid())
    }
}

/// Ошибка недоступного runtime по его состоянию — ловимая на стороне BSL.
fn unavailable_error(state: RuntimeState) -> HostError {
    match state {
        RuntimeState::Broken => HostError::new(
            HostErrorCode::RuntimeBroken,
            "фоновый runtime сломан и не принимает задания",
        ),
        _ => HostError::new(
            HostErrorCode::RuntimeClosed,
            "фоновый runtime закрыт и не принимает задания",
        ),
    }
}

impl JobRuntime {
    /// Создаёт холодный runtime: потоков нет до первого admission.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        config: BackgroundJobConfig,
        recipe: WorkerRecipe,
        targets: TargetTable,
        id_source: Arc<dyn JobIdSource>,
        temp_hub: Arc<bsl_rt::TempStorageHub>,
        profiles: Arc<[Arc<dyn BackgroundStateFactory>]>,
        message_display: Option<Arc<dyn bsl_rt::UserMessageSink + Send + Sync>>,
    ) -> Self {
        let workers = config.effective_workers();
        let staging_global = Arc::new(GlobalStagingBudget::new(config.max_live_staged_temp_bytes));
        Self {
            shared: Arc::new(JobRuntimeShared {
                registry: Mutex::new(JobRegistry::new(config.clone())),
                work_available: Condvar::new(),
                terminal_watch: Condvar::new(),
                recipe,
                id_source,
                time_source: Arc::new(SystemJobTime),
                targets,
                temp_hub,
                config,
                profiles,
                staging_global,
                message_display,
                threads: Mutex::new(Vec::new()),
                bootstrap_gate: Mutex::new(None),
            }),
            workers,
        }
    }

    /// Ставит тестовый шлюз бутстрапа — только до первого admission.
    #[cfg(test)]
    fn set_bootstrap_gate(&self, gate: Arc<BootstrapGate>) {
        *self
            .shared
            .bootstrap_gate
            .lock()
            .expect("шлюз без отравления") = Some(gate);
    }

    /// Запускает экспортный метод общего модуля в отдельном сеансе.
    /// Возвращает снимок `Queued`; сам запуск пула ленивый и submissions
    /// во время `Starting` не ждут создания всех потоков.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn submit(
        &self,
        method_name: &str,
        target: (u32, u16),
        params: Arc<SerializedValueGraph>,
        key: Option<Arc<JobKeyDto>>,
        description: Option<String>,
        caller_token: Option<[u8; 16]>,
        profile_index: u32,
    ) -> Result<JobSnapshotDto, HostError> {
        let snapshot = self.shared.submit_shared(
            method_name,
            target,
            params,
            key,
            description,
            caller_token,
            profile_index,
        )?;
        self.ensure_workers();
        self.shared.work_available.notify_one();
        // См. submit_by_name_shared: помощники ждут на terminal_watch.
        self.shared.terminal_watch.notify_all();
        Ok(snapshot)
    }

    /// Ленивый запуск пула: первый admission спавнит РОВНО ОДИН поток —
    /// первого worker, который перед собственным циклом бутстрапит
    /// соседей (`worker_bootstrap`). Возврат первого admission не ждёт
    /// создания всего пула; паника создания потока — `Broken` для всего
    /// runtime.
    fn ensure_workers(&self) {
        let mut threads = self
            .shared
            .threads
            .lock()
            .expect("список потоков без отравления");
        if !threads.is_empty() {
            return;
        }
        let shared = Arc::clone(&self.shared);
        let workers = self.workers;
        let builder = std::thread::Builder::new().name("bsl-job-worker-0".to_string());
        match builder.spawn(move || {
            worker_bootstrap(&shared, workers);
            worker_supervisor(&shared);
        }) {
            Ok(handle) => threads.push(handle),
            Err(_) => {
                let mut registry = self.shared.registry.lock().expect("реестр без отравления");
                registry.state = RuntimeState::Broken;
                fail_all_resident(&mut registry);
                self.shared.terminal_watch.notify_all();
            }
        }
    }

    /// Запускает цель по имени «Модуль.Метод»: разрешение цели по
    /// каталогу рецепта worker + admission. Ошибки цели и лимитов —
    /// ловимые на стороне BSL.
    ///
    /// # Errors
    ///
    /// [`HostError`] с причиной: `InvalidCall` для негодной цели и
    /// дублирующего ключа, `ResourceLimit` для явных лимитов,
    /// `RuntimeClosed`/`RuntimeBroken` для закрытого runtime.
    pub fn submit_by_name(
        &self,
        method_name: &str,
        params: Arc<SerializedValueGraph>,
        key: Option<Arc<JobKeyDto>>,
        description: Option<String>,
    ) -> Result<JobSnapshotDto, HostError> {
        self.submit_by_name_with_caller(method_name, params, key, description, None, 0)
    }

    /// То же с token'ом временного хранилища вызывателя и host-профилем —
    /// путь сервисов.
    pub(crate) fn submit_by_name_with_caller(
        &self,
        method_name: &str,
        params: Arc<SerializedValueGraph>,
        key: Option<Arc<JobKeyDto>>,
        description: Option<String>,
        caller_token: Option<[u8; 16]>,
        profile_index: u32,
    ) -> Result<JobSnapshotDto, HostError> {
        let target = self
            .shared
            .targets
            .resolve(method_name)
            .map_err(|text| HostError::new(HostErrorCode::InvalidCall, text))?;
        self.submit(
            method_name,
            target,
            params,
            key,
            description,
            caller_token,
            profile_index,
        )
    }

    /// Снимок задания по идентификатору.
    pub fn snapshot(&self, id: JobId) -> Option<Arc<JobSnapshotDto>> {
        self.shared
            .registry
            .lock()
            .expect("реестр без отравления")
            .snapshot(id)
    }

    /// Все снимки (живые и история) — фильтрация вне лока.
    pub fn snapshots(&self) -> Vec<Arc<JobSnapshotDto>> {
        self.shared
            .registry
            .lock()
            .expect("реестр без отравления")
            .snapshots()
    }

    /// Ожидает terminal-состояния ВСЕХ перечисленных заданий (ИЗМЕРЕНО,
    /// `JOB.WAIT.MANY`). `None` — без предела. `Ok(true)` — дождались,
    /// `Ok(false)` — таймаут.
    ///
    /// # Errors
    ///
    /// `JobExpired` для неизвестного задания на входе и для задания,
    /// вытесненного из истории во время ожидания; `RuntimeClosed`/
    /// `RuntimeBroken` после закрытия runtime.
    pub fn wait_terminal(
        &self,
        ids: &[JobId],
        timeout: Option<Duration>,
    ) -> Result<bool, HostError> {
        self.shared
            .wait_terminal_blocking(ids, timeout)
            .map(|outcome| outcome.completed)
    }

    /// Отмена задания: `Queued` — сразу, `Running` — кооперативно на
    /// границе кванта; повторная отмена и terminal — no-op.
    ///
    /// # Errors
    ///
    /// Как у [`JobRuntime::wait_terminal`].
    pub fn cancel(&self, id: JobId) -> Result<(), HostError> {
        let end = self.shared.time_source.wall_now();
        self.shared.cancel(id, end)
    }

    /// Сообщения задания в порядке FIFO; `remove` атомарно забирает их —
    /// и у живой записи, и у terminal (ИЗМЕРЕНО, `JOB.MESSAGES`).
    ///
    /// # Errors
    ///
    /// Как у [`JobRuntime::wait_terminal`].
    pub fn take_messages(&self, id: JobId, remove: bool) -> Result<Vec<UserMessageDto>, HostError> {
        self.shared.take_messages(id, remove)
    }

    /// Явное завершение runtime: queued отменяются сразу, running —
    /// кооперативно на границах квантов; потоки соединяются до `deadline`,
    /// неответившие отсоединяются и попадают в отчёт. Новые submissions
    /// после закрытия получают ловимую ошибку.
    pub fn shutdown(&self, deadline: Duration) -> ShutdownReport {
        let end = self.shared.time_source.wall_now();
        {
            let mut registry = self.shared.registry.lock().expect("реестр без отравления");
            registry.state = RuntimeState::Closed;
            let jobs: Vec<(JobId, JobStateDto)> = registry
                .records()
                .map(|record| (record.snapshot.id, record.snapshot.state))
                .collect();
            for (id, state) in jobs {
                match state {
                    JobStateDto::Queued => {
                        registry.finish(id, JobStateDto::Canceled, end, None);
                    }
                    JobStateDto::Running => {
                        if let Some(record) = registry.record(id) {
                            record
                                .cancel_requested
                                .store(true, std::sync::atomic::Ordering::Relaxed);
                        }
                    }
                    _ => {}
                }
            }
        }
        self.shared.work_available.notify_all();
        self.shared.terminal_watch.notify_all();

        let handles: Vec<std::thread::JoinHandle<()>> = std::mem::take(
            &mut *self
                .shared
                .threads
                .lock()
                .expect("список потоков без отравления"),
        );
        let deadline_at = std::time::Instant::now() + deadline;
        let mut detached_workers = 0usize;
        for handle in handles {
            loop {
                if handle.is_finished() {
                    let _ = handle.join();
                    break;
                }
                if std::time::Instant::now() >= deadline_at {
                    // Отсоединяем: поток физически может дорабатывать уже
                    // начатый внешний эффект, но реестр закрыт и его
                    // поздние публикации отвергаются состоянием Closed.
                    detached_workers += 1;
                    break;
                }
                std::thread::sleep(Duration::from_millis(1));
            }
        }
        ShutdownReport { detached_workers }
    }
}

/// Отчёт явного завершения: сколько workers не успели выйти до deadline
/// и были отсоединены.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShutdownReport {
    pub detached_workers: usize,
}

impl Drop for JobRuntime {
    /// Последний владелец сигнализирует завершение, но НЕ блокируется:
    /// потоки выйдут сами на ближайшей границе кванта.
    fn drop(&mut self) {
        let mut registry = self.shared.registry.lock().expect("реестр без отравления");
        if registry.state != RuntimeState::Closed {
            registry.state = RuntimeState::Closed;
            for record in registry.records() {
                record
                    .cancel_requested
                    .store(true, std::sync::atomic::Ordering::Relaxed);
            }
        }
        drop(registry);
        self.shared.work_available.notify_all();
        self.shared.terminal_watch.notify_all();
    }
}

/// Все живые задания — в `Failed(RuntimeBroken)`: вызывается при
/// поломке runtime под уже взятым локом.
fn fail_all_resident(registry: &mut JobRegistry) {
    fail_resident_jobs(registry, "фоновый runtime сломан и не принимает задания");
}

/// Все живые записи — в `Failed(текст)`, КРОМЕ заклеймленных драйвером:
/// их публикация уже идёт, и исход объявит владелец claim. Иначе задание
/// оказалось бы `Failed(RuntimeBroken)` с ОПУБЛИКОВАННЫМИ данными —
/// прямое нарушение матрицы «инфраструктурный сбой — rollback».
fn fail_resident_jobs(registry: &mut JobRegistry, text: &str) {
    let ids: Vec<JobId> = registry
        .records()
        .filter(|record| !record.committing)
        .map(|record| record.snapshot.id)
        .collect();
    for id in ids {
        registry.finish(
            id,
            JobStateDto::Failed,
            None,
            Some(JobErrorDto::from_text(text)),
        );
    }
}

/// Бутстрап пула на потоке ПЕРВОГО worker: соседи спавнятся отсюда, а не
/// из admission, поэтому первый submit возвращается после одного spawn,
/// а не N. Переход `Starting -> Running` делает бутстраппер после
/// последнего соседа; отказ spawn ломает runtime (`Broken`); закрытие во
/// время `Starting` останавливает спавн перед следующим соседом, и все
/// уже созданные handles остаются в разделяемом списке — shutdown их
/// соединяет. Порядок локов: сначала список потоков, затем реестр —
/// поэтому поздний spawn не может проскочить мимо взятого shutdown
/// списка.
fn worker_bootstrap(shared: &Arc<JobRuntimeShared>, workers: usize) {
    wait_bootstrap_gate(shared);
    for index in 1..workers {
        let mut threads = shared
            .threads
            .lock()
            .expect("список потоков без отравления");
        {
            let registry = shared.registry.lock().expect("реестр без отравления");
            if matches!(registry.state, RuntimeState::Closed | RuntimeState::Broken) {
                return;
            }
        }
        let sibling = Arc::clone(shared);
        let builder = std::thread::Builder::new().name(format!("bsl-job-worker-{index}"));
        match builder.spawn(move || worker_supervisor(&sibling)) {
            Ok(handle) => threads.push(handle),
            Err(_) => {
                drop(threads);
                let mut registry = shared.registry.lock().expect("реестр без отравления");
                registry.state = RuntimeState::Broken;
                fail_all_resident(&mut registry);
                drop(registry);
                shared.terminal_watch.notify_all();
                shared.work_available.notify_all();
                return;
            }
        }
    }
    let mut registry = shared.registry.lock().expect("реестр без отравления");
    if registry.state == RuntimeState::Starting {
        registry.state = RuntimeState::Running;
    }
}

/// Удержание тестового шлюза бутстрапа. Без шлюза — no-op; удержанный
/// шлюз пережидается с оглядкой на состояние runtime: закрытие или
/// поломка снимают бутстраппер и с шлюза. Пятидесятимиллисекундный тик
/// существует только на тестовом пути с выставленным шлюзом.
fn wait_bootstrap_gate(shared: &Arc<JobRuntimeShared>) {
    let gate = shared
        .bootstrap_gate
        .lock()
        .expect("шлюз без отравления")
        .clone();
    let Some(gate) = gate else {
        return;
    };
    let (held, released) = &*gate;
    let mut held_guard = held.lock().expect("шлюз без отравления");
    while *held_guard {
        {
            let registry = shared.registry.lock().expect("реестр без отравления");
            if matches!(registry.state, RuntimeState::Closed | RuntimeState::Broken) {
                return;
            }
        }
        let (guard, _) = released
            .wait_timeout(held_guard, Duration::from_millis(50))
            .expect("шлюз без отравления");
        held_guard = guard;
    }
}

/// Надзор за worker: паника ВНЕ границы задания роняет резидентов этого
/// worker в `Failed` (их Drop-гарды, см. `RunningJob`) и создаёт замену —
/// цикл перезапускается. Три ПОСЛЕДОВАТЕЛЬНЫЕ паники запуска — worker ни
/// разу не довёл задание до terminal между ними — переводят runtime в
/// `Broken`: живые задания завершаются `Failed`, новые submissions
/// получают ловимую ошибку, автоматического recovery нет.
fn worker_supervisor(shared: &Arc<JobRuntimeShared>) {
    let mut consecutive_panics = 0u32;
    loop {
        let progress = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let progress_flag = Arc::clone(&progress);
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            worker_main(shared, &progress_flag);
        }));
        match outcome {
            Ok(()) => return, // нормальный выход: runtime закрыт или сломан
            Err(_) => {
                // Резиденты живут в пуле ПОТОКА и разматыванием паники не
                // дропаются: забираем их здесь — Drop-гарды переводят их
                // в `Failed`, вечного `Running` не остаётся.
                drop(take_thread_residents());
                if progress.load(std::sync::atomic::Ordering::Relaxed) {
                    consecutive_panics = 1;
                } else {
                    consecutive_panics += 1;
                }
                if consecutive_panics >= 3 {
                    let mut registry = shared.registry.lock().expect("реестр без отравления");
                    registry.state = RuntimeState::Broken;
                    fail_all_resident(&mut registry);
                    drop(registry);
                    shared.terminal_watch.notify_all();
                    // Соседние worker'ы спят без таймера — поломку они
                    // обязаны увидеть по state, а не по случайному событию.
                    shared.work_available.notify_all();
                    return;
                }
            }
        }
    }
}

/// Главный цикл worker: резиденты локальной FIFO чередуются бюджетными
/// квантами, задание с припаркованным host-вызовом (`Waiting`) не мешает
/// соседям, а когда все резиденты ждут — worker спит точно, до события
/// (`wake_epoch`, очередь, отмена, закрытие), без таймерного поллинга.
fn worker_main(shared: &Arc<JobRuntimeShared>, progress: &std::sync::atomic::AtomicBool) {
    // Каталог разбирается один раз на worker; программы разделяются между
    // сеансами этого worker и не покидают его поток.
    let engine = match build_worker_engine(&shared.recipe) {
        Ok(engine) => engine,
        Err(error) => {
            let mut registry = shared.registry.lock().expect("реестр без отравления");
            registry.state = RuntimeState::Broken;
            let text = format!("worker не разобрал рецепт каталога: {error}");
            fail_resident_jobs(&mut registry, &text);
            drop(registry);
            shared.terminal_watch.notify_all();
            return;
        }
    };
    // Резиденты живут в пуле ПОТОКА, а не в кадре: вложенное ожидание
    // внутри кванта резидента обслуживает тот же пул и потому способно
    // доводить своих соседей (см. `drive_local`).
    drive_local(shared, &engine, DriveMode::Worker(progress));
}

thread_local! {
    /// Резиденты ЭТОГО потока. Пул общий для цикла worker и всех
    /// вложенных helping-драйверов: задание, припаркованное на
    /// host-операции, закреплено за своим потоком, и довести его может
    /// ТОЛЬКО этот поток. Пока набор жил в кадре `worker_main`, вложенное
    /// ожидание соседа-резидента (родитель ждёт ребёнка, который сам
    /// припаркован на HTTP этого же worker) не имело к нему доступа и
    /// парковалось навсегда.
    static RESIDENTS: RefCell<VecDeque<RunningJob>> = const { RefCell::new(VecDeque::new()) };
}

/// Забирает резидентов потока — путь очистки после паники worker:
/// `Drop`-гарды переводят их в `Failed`, вечного `Running` не остаётся.
fn take_thread_residents() -> VecDeque<RunningJob> {
    RESIDENTS.with(|residents| std::mem::take(&mut *residents.borrow_mut()))
}

/// Режим общего цикла резидентов.
enum DriveMode<'a> {
    /// Цикл worker: живёт до закрытия runtime, отмечает прогресс для
    /// супервизора.
    Worker(&'a std::sync::atomic::AtomicBool),
    /// Helping-ожидание: выход по собственному предикату либо дедлайну.
    /// Ожидающий поток не простаивает — он двигает пул своего потока и
    /// подбирает задания из глобальной очереди.
    Until {
        /// Наблюдаемые задания — для тестового шлюза окна парковки.
        ids: &'a [JobId],
        /// Предикат завершения, вычисляемый ПОД локом реестра: там же,
        /// где принимается решение спать, — потерянных пробуждений нет.
        done: &'a dyn Fn(&JobRegistry) -> bool,
        deadline: Option<std::time::Instant>,
    },
}

/// Общий цикл резидентов worker и helping-ожиданий. Локальная FIFO
/// потока: runnable чередуются бюджетными квантами, waiting опрашиваются
/// вместе с ними (host-completions подбирает их собственный poll), новое
/// глобальное задание извлекается, только когда локально нет
/// runnable-резидентов, — по плану фоновых заданий.
///
/// В режиме `Until` тот же цикл обслуживает вложенное ожидание: пока цель
/// не достигнута, поток двигает СВОИХ резидентов (включая припаркованных
/// на host-операциях соседей — иначе их некому довести) и помогает
/// глобальной очереди, а спит только когда двигать нечего.
fn drive_local(shared: &Arc<JobRuntimeShared>, engine: &crate::Engine, mode: DriveMode<'_>) {
    let mark_progress = || {
        if let DriveMode::Worker(progress) = &mode {
            progress.store(true, std::sync::atomic::Ordering::Relaxed);
        }
    };
    let deadline = match &mode {
        DriveMode::Until { deadline, .. } => *deadline,
        DriveMode::Worker(_) => None,
    };
    // Снимок wake_epoch на момент последнего начала опроса ждущих
    // резидентов: спим, только если с тех пор не было ни одного
    // host-события. Событие между опросом и сном поднимает счётчик под
    // локом — пропущенных пробуждений нет.
    let mut seen_epoch = 0u64;
    loop {
        // Тестовое окно «предикат ещё не перечитан»: держится БЕЗ лока
        // реестра, иначе завершающий задание тест встал бы на нём сам.
        helping_window_pause(&mode);
        let next_global = {
            let mut registry = shared.registry.lock().expect("реестр без отравления");
            loop {
                let closed = matches!(registry.state, RuntimeState::Closed | RuntimeState::Broken);
                match &mode {
                    DriveMode::Until { ids, done, .. } => {
                        // Цель и недоступность runtime проверяются ПОД тем
                        // же локом, под которым принимается решение спать.
                        if closed || done(&registry) {
                            return;
                        }
                        let _ = ids;
                    }
                    DriveMode::Worker(_) => {
                        if closed {
                            // Закрытие на границе кванта — и есть
                            // кооперативная точка: резиденты завершаются
                            // «Отменено». Сначала terminal transitions под
                            // локом, а сами резиденты дропаются ПОСЛЕ его
                            // отпускания: Drop-гард берёт этот же мьютекс.
                            drop(registry);
                            let end = shared.time_source.wall_now();
                            let residents = take_thread_residents();
                            {
                                let mut registry =
                                    shared.registry.lock().expect("реестр без отравления");
                                for job in residents.iter() {
                                    registry.finish(job.id, JobStateDto::Canceled, end, None);
                                }
                            }
                            drop(residents);
                            shared.terminal_watch.notify_all();
                            return;
                        }
                    }
                }
                if deadline.is_some_and(|deadline| std::time::Instant::now() >= deadline) {
                    return;
                }
                let (has_residents, runnable_locally) = RESIDENTS.with(|residents| {
                    let residents = residents.borrow();
                    (
                        !residents.is_empty(),
                        residents.iter().any(|job| !job.waiting),
                    )
                });
                if runnable_locally {
                    // Есть чем заняться — глобальную очередь не трогаем.
                    break None;
                }
                if let Some(id) = registry.queue.pop_front() {
                    break Some(id);
                }
                // Двигать нечего: сон до события. В режиме worker это
                // `work_available` (новая работа и host-события), в
                // helping-ожидании — `terminal_watch`, который будят и
                // terminal-переходы чужих заданий, и host-completions, и
                // отмена, и закрытие.
                if has_residents && registry.wake_epoch != seen_epoch {
                    seen_epoch = registry.wake_epoch;
                    // Неизвестно, чьё завершение пришло: каждый ждущий
                    // резидент опрашивается заново (его собственный poll
                    // подберёт доставленные завершения либо снова уснёт).
                    RESIDENTS.with(|residents| {
                        for job in residents.borrow_mut().iter_mut() {
                            job.waiting = false;
                        }
                    });
                    break None;
                }
                let condvar = match &mode {
                    DriveMode::Worker(_) => &shared.work_available,
                    DriveMode::Until { .. } => &shared.terminal_watch,
                };
                HELPING_PARKS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                match deadline {
                    None => {
                        registry = condvar.wait(registry).expect("реестр без отравления");
                    }
                    Some(deadline) => {
                        let left = deadline.saturating_duration_since(std::time::Instant::now());
                        let (guard, _) = condvar
                            .wait_timeout(registry, left)
                            .expect("реестр без отравления");
                        registry = guard;
                    }
                }
            }
        };
        if let Some(id) = next_global {
            match start_job(shared, engine, id) {
                None => continue,
                Some(Ok(job)) => RESIDENTS.with(|residents| residents.borrow_mut().push_back(job)),
                Some(Err(error)) => {
                    finish_job(shared, id, JobStateDto::Failed, Some(error));
                    // Отказ подготовки — тоже доведённое до terminal
                    // задание: супервизор обязан считать это прогрессом.
                    mark_progress();
                    continue;
                }
            }
        }
        // Резидент ИЗВЛЕКАЕТСЯ из пула на время кванта: вложенное
        // ожидание внутри этого кванта увидит только соседей и не станет
        // опрашивать задание, чей квант уже на стеке.
        let Some(mut job) = RESIDENTS.with(|residents| {
            let mut residents = residents.borrow_mut();
            let index = residents.iter().position(|job| !job.waiting)?;
            residents.remove(index)
        }) else {
            continue;
        };
        // Паника BSL-исполнения ловится на границе кванта и роняет только
        // это задание; соседи-резиденты продолжают.
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| job.poll(engine)));
        match outcome {
            Ok(Ok(bsl_vm::ProgramPoll::Complete(..))) => {
                // Право на terminal transition забирается ДО публикации:
                // поломка runtime, выигравшая гонку, оставляет задание
                // `Failed` БЕЗ публикации (rollback по матрице), а
                // выигравший драйвер публикует и завершает сам.
                commit_window_pause(job.id);
                if claim_terminal(shared, job.id) {
                    if commit_staged(shared, &job) {
                        finish_job(shared, job.id, JobStateDto::Completed, None);
                    } else {
                        // Успешный job при закрытом сеансе вызывателя — Failed:
                        // частичной публикации нет (план, JOB.TEMP.CALLER_CLOSE_RACE).
                        finish_job(
                            shared,
                            job.id,
                            JobStateDto::Failed,
                            Some(JobErrorDto::from_text(
                                "сеанс-получатель временного хранилища закрыт",
                            )),
                        );
                    }
                }
                mark_progress();
            }
            Ok(Ok(bsl_vm::ProgramPoll::Runnable)) => {
                job.waiting = false;
                RESIDENTS.with(|residents| residents.borrow_mut().push_back(job));
            }
            Ok(Ok(bsl_vm::ProgramPoll::Waiting)) => {
                job.waiting = true;
                RESIDENTS.with(|residents| residents.borrow_mut().push_back(job));
            }
            Ok(Err(JobPollError::Canceled)) => {
                finish_job(shared, job.id, JobStateDto::Canceled, None);
                mark_progress();
            }
            Ok(Err(JobPollError::Failed(error))) => {
                // Неперехваченная BSL-ошибка ПУБЛИКУЕТ write-set — измерено
                // JOB.TEMP.FAILURE; неудача публикации остаётся вторичной
                // причиной в cause-цепочке, основная ошибка BSL важнее.
                // Публикация — только с забранным claim (см. выше).
                if claim_terminal(shared, job.id) {
                    let error = if commit_staged(shared, &job) {
                        error
                    } else {
                        with_commit_failure_cause(error)
                    };
                    finish_job(shared, job.id, JobStateDto::Failed, Some(error));
                }
                mark_progress();
            }
            Err(_) => {
                finish_job(
                    shared,
                    job.id,
                    JobStateDto::Failed,
                    Some(JobErrorDto::from_text(
                        "исполнение задания прервано паникой",
                    )),
                );
                mark_progress();
            }
        }
    }
}

/// Движок worker строится из того же публичного образа, которым ходит
/// `--emit-bytecode`: стандартный состав компонентов плюс
/// пользовательские библиотеки родительского движка и его символы
/// препроцессора — цель с пользовательским типом линкуется, а
/// динамический код задания видит те же `#Если`.
fn build_worker_engine(recipe: &WorkerRecipe) -> Result<crate::Engine, String> {
    let image = bsl_bytecode::parse_image(&recipe.image_text).map_err(|e| e.to_string())?;
    let bsl_bytecode::BytecodeImage::Configuration { catalog, entry: _ } = image else {
        return Err("рецепт worker обязан быть конфигурацией".to_string());
    };
    let mut builder = crate::Engine::builder();
    for library in &recipe.libraries {
        builder = builder.register_library(*library);
    }
    builder
        .preproc_symbols_all(recipe.symbols)
        .configuration_image(catalog, false)
        .build()
        .map_err(|e| e.to_string())
}

/// Маршрут сообщений задания: неблокирующий sink его сеанса. Сначала DTO
/// добавляется к записи реестра под коротким локом (бюджет
/// `max_message_bytes_per_job`), затем — уже без лока — уходит внешнему
/// sink представления, если host его зарегистрировал. Backpressure
/// внешнего sink не отменяет уже записанную историю и не повторяется
/// скрыто: `Сообщить()` возвращает ловимую ошибку, повторный вызов
/// создаёт новое сообщение.
struct JobMessageRoute {
    shared: Arc<JobRuntimeShared>,
    id: JobId,
}

impl bsl_rt::UserMessageSink for JobMessageRoute {
    fn enqueue(&self, message: &UserMessageDto) -> Result<(), HostError> {
        {
            let mut registry = self.shared.registry.lock().expect("реестр без отравления");
            registry.push_message(self.id, message.clone())?;
        }
        if let Some(display) = &self.shared.message_display {
            display.enqueue(message)?;
        }
        Ok(())
    }

    /// Остаток кумулятивного бюджета `max_message_bytes_per_job` этой
    /// записи: им ограничивается сериализация `КлючДанных` и
    /// `ИдентификаторНазначения` ДО крупной аллокации. Для уже
    /// terminal-записи предела нет — `enqueue` всё равно ответит
    /// `JobExpired`, и подменять его ошибкой бюджета нельзя.
    fn message_bytes_left(&self) -> Option<usize> {
        let registry = self.shared.registry.lock().expect("реестр без отравления");
        let record = registry.record(self.id)?;
        Some(
            self.shared
                .config
                .max_message_bytes_per_job
                .saturating_sub(record.message_bytes),
        )
    }
}

/// Резидент worker: изолированный сеанс одного задания с pollable
/// VM-прогоном. Начатый резидент закреплён за своим worker — `Rc`-графы
/// его сеанса поток не покидают.
///
/// Drop-гард: резидент, дропнутый БЕЗ terminal transition — разматывание
/// паники worker, замена worker супервизором, — завершает свою запись
/// `Failed`, будит ожидающих и не оставляет задание вечным `Running`.
/// Host-handles отменяет дроп VM (`AsyncState::drop`), staging
/// откатывается дропом сеанса, его кредиты возвращает `StagingBudget`.
/// Штатные пути уже сделали terminal transition к моменту дропа, и гард
/// для них — no-op. Ни один дроп резидента не происходит под локом
/// реестра — гард берёт его сам.
struct RunningJob {
    id: JobId,
    shared: Arc<JobRuntimeShared>,
    state: crate::State,
    module: crate::Module,
    vm: bsl_vm::ProgramExecution,
    /// Последний poll вернул `Waiting`: задание ждёт host-completion и
    /// runnable-резидентом не считается.
    waiting: bool,
}

impl Drop for RunningJob {
    fn drop(&mut self) {
        let end = self.shared.time_source.wall_now();
        // `lock()` без `expect`: гард срабатывает и при разматывании
        // паники — отравленный мьютекс здесь не повод для двойной паники.
        let Ok(mut registry) = self.shared.registry.lock() else {
            return;
        };
        let finished = registry.finish(
            self.id,
            JobStateDto::Failed,
            end,
            Some(JobErrorDto::from_text(
                "исполнение задания прервано сбоем worker",
            )),
        );
        drop(registry);
        if finished {
            self.shared.terminal_watch.notify_all();
        }
    }
}

/// Гард окна запуска: между `Queued -> Running` и передачей записи под
/// Drop-гард готового резидента задание не должно застрять вечным
/// `Running`, если подготовка сеанса паникует (паника worker вне границы
/// задания). Взведённый гард завершает запись `Failed`; штатные выходы
/// его разряжают.
struct StartGuard<'a> {
    shared: &'a Arc<JobRuntimeShared>,
    id: JobId,
    armed: bool,
}

impl Drop for StartGuard<'_> {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }
        let end = self.shared.time_source.wall_now();
        let Ok(mut registry) = self.shared.registry.lock() else {
            return;
        };
        let finished = registry.finish(
            self.id,
            JobStateDto::Failed,
            end,
            Some(JobErrorDto::from_text(
                "запуск задания прерван сбоем worker",
            )),
        );
        drop(registry);
        if finished {
            self.shared.terminal_watch.notify_all();
        }
    }
}

/// Стартует резидента: `Queued -> Running`, entry-программа цели,
/// изолированный сеанс, pollable-прогон с постоянным квантованием.
/// `None` — запись уже terminal (например, отменена до старта).
fn start_job(
    shared: &Arc<JobRuntimeShared>,
    engine: &crate::Engine,
    id: JobId,
) -> Option<Result<RunningJob, JobErrorDto>> {
    let mut guard = StartGuard {
        shared,
        id,
        armed: true,
    };
    let begin = shared.time_source.wall_now();
    // `Queued -> Running` — ПЕРВОЕ изменение статуса по семантике
    // менеджерного `ОжидатьЗавершенияВыполнения`: ожидающих будим.
    let mut started = false;
    let taken = {
        let mut registry = shared.registry.lock().expect("реестр без отравления");
        let record = registry.record_mut(id);
        match record {
            None => None,
            Some(record) => {
                record.snapshot.state = JobStateDto::Running;
                record.snapshot.begin = begin;
                started = true;
                Some((
                    record.target,
                    Arc::clone(&record.snapshot.params),
                    Arc::clone(&record.cancel_requested),
                    record.caller_token,
                    record.profile_index,
                ))
            }
        }
    };
    if started {
        shared.terminal_watch.notify_all();
    }
    let Some((target, params, cancel_requested, caller_token, profile_index)) = taken else {
        guard.armed = false;
        return None;
    };
    let prepared = prepare_job(
        shared,
        engine,
        id,
        target,
        &params,
        caller_token,
        profile_index,
    );
    guard.armed = false;
    Some(prepared.map(|(state, module, mut vm)| {
        vm.set_cancel_flag(cancel_requested);
        // Пробуждение из потока транспорта: поднять wake_epoch под
        // локом и разбудить пул. Сон worker сверяет счётчик со
        // своим снимком, поэтому пробуждение не теряется, даже
        // если пришло между опросом резидентов и засыпанием.
        let waker_shared = Arc::clone(shared);
        vm.set_host_waker(std::sync::Arc::new(move || {
            let mut registry = waker_shared.registry.lock().expect("реестр без отравления");
            registry.wake_epoch += 1;
            drop(registry);
            waker_shared.work_available.notify_all();
            // Helping-ожидание спит на `terminal_watch`: завершение
            // host-операции — его событие тоже (появился резидент,
            // которого стало можно двигать).
            waker_shared.terminal_watch.notify_all();
        }));
        RunningJob {
            id,
            shared: Arc::clone(shared),
            state,
            module,
            vm,
            waiting: false,
        }
    }))
}

/// Исход неудачного кванта: отмена — не ошибка BSL, снимок получает
/// состояние «Отменено» без `ИнформацияОбОшибке`.
enum JobPollError {
    Canceled,
    Failed(JobErrorDto),
}

impl RunningJob {
    /// Один бюджетный квант задания. Поток не блокируется: completions
    /// подбираются конечным срезом без ожидания первого.
    fn poll(&mut self, engine: &crate::Engine) -> Result<bsl_vm::ProgramPoll, JobPollError> {
        let catalog = engine
            .catalog()
            .expect("worker строится только с каталогом");
        self.vm
            .poll_configuration_with_budget(
                &self.module.program,
                catalog,
                engine.registry(),
                &mut self.state.host.stdout,
                &mut self.state.host.stderr,
                &mut self.state.dynamic,
                &mut self.state.host.env,
                1024,
                Some(1),
            )
            .map_err(|error| match error {
                bsl_rt::RtError::Canceled => JobPollError::Canceled,
                other => JobPollError::Failed(JobErrorDto::from_text(other.to_string())),
            })
    }
}

/// Публикация write-set задания по измеренной terminal-матрице: commit
/// после успеха И неперехваченной BSL-ошибки (`JOB.TEMP.FAILURE`),
/// rollback — просто дроп staging — после отмены (`JOB.TEMP.CANCEL`),
/// паники и инфраструктурных сбоев. `false` — сеанс вызывателя уже
/// закрыт и публикация не состоялась.
// НЕ ИЗМЕРЕНО(JOB.TEMP.CALLER_CLOSE_RACE): гонка закрытия сеанса
// вызывателя с terminal commit на платформе не замерена; выбрано «без
// частичной публикации»: успешный BSL-job при закрытом получателе
// становится Failed, что закреплено тестом
// `a_closed_caller_session_fails_the_publishing_job`.
fn commit_staged(shared: &Arc<JobRuntimeShared>, job: &RunningJob) -> bool {
    let Some(session) = job.state.host.env.temp_storage() else {
        return true;
    };
    let mut session = session.borrow_mut();
    let Some(caller) = session.caller() else {
        return true;
    };
    let writes = session.take_staged();
    if writes.is_empty() {
        return true;
    }
    shared.temp_hub.commit(caller, writes)
}

/// Пристраивает отказ terminal commit в конец cause-цепочки ошибки:
/// при неперехваченной BSL-ошибке и закрытом сеансе-получателе основная
/// ошибка остаётся BSL-ошибкой, а закрытый mailbox виден вторичной
/// причиной, не подменяя её (измеренный порядок публикаций —
/// `JOB.TEMP.FAILURE`, гонка закрытия — `JOB.TEMP.CALLER_CLOSE_RACE`).
fn with_commit_failure_cause(mut error: JobErrorDto) -> JobErrorDto {
    let mut tail = &mut error;
    while tail.cause.is_some() {
        tail = tail.cause.as_mut().expect("проверено условием цикла");
    }
    tail.cause = Some(Box::new(JobErrorDto::from_text(
        "сеанс-получатель временного хранилища закрыт",
    )));
    error
}

/// Тестовый шлюз окна «задание завершилось, право ещё не забрано»:
/// проба закрывает runtime ровно здесь и проверяет, что публикация не
/// состоялась (`a_shutdown_during_the_commit_window_rolls_back`).
#[cfg(test)]
struct CommitWindowGate {
    target: JobId,
    held: Mutex<bool>,
    released: Condvar,
    entered: std::sync::atomic::AtomicBool,
}

#[cfg(test)]
static COMMIT_WINDOW_GATE: Mutex<Option<Arc<CommitWindowGate>>> = Mutex::new(None);

/// Пауза перед взятием claim. Вне тестов и без шлюза — пустышка.
fn commit_window_pause(id: JobId) {
    #[cfg(not(test))]
    let _ = id;
    #[cfg(test)]
    {
        let gate = COMMIT_WINDOW_GATE
            .lock()
            .expect("шлюз без отравления")
            .clone();
        let Some(gate) = gate else {
            return;
        };
        if gate.target != id {
            return;
        }
        gate.entered
            .store(true, std::sync::atomic::Ordering::SeqCst);
        let mut held = gate.held.lock().expect("шлюз без отравления");
        while *held {
            held = gate.released.wait(held).expect("шлюз без отравления");
        }
    }
}

/// Забирает право на terminal transition под локом реестра. Отказ при
/// закрытом runtime завершает запись «Отменено» прямо там же, поэтому
/// ожидающих будим в любом случае.
fn claim_terminal(shared: &Arc<JobRuntimeShared>, id: JobId) -> bool {
    let end = shared.time_source.wall_now();
    let granted = shared
        .registry
        .lock()
        .expect("реестр без отравления")
        .claim_terminal(id, end);
    if !granted {
        shared.terminal_watch.notify_all();
    }
    granted
}

/// Terminal transition резидента с пробуждением ожидающих.
fn finish_job(
    shared: &Arc<JobRuntimeShared>,
    id: JobId,
    state: JobStateDto,
    error: Option<JobErrorDto>,
) {
    let end = shared.time_source.wall_now();
    let mut registry = shared.registry.lock().expect("реестр без отравления");
    registry.finish(id, state, end, error);
    drop(registry);
    shared.terminal_watch.notify_all();
}

/// Готовит сеанс задания без компилятора: entry-программа собирается
/// руками — аргументы приходят константами чанка, вызов идёт обычным
/// `CallImported` по числовому манифесту. Возвращает изолированный сеанс,
/// модуль entry и pollable-прогон с постоянным квантованием.
// НЕ ИЗМЕРЕНО(JOB.MODULE.INIT): ИЗМЕРЕНО на файловой базе (2026-08-27),
// что у платформенных серверных общих модулей тел НЕТ вовсе — модуль с
// телом грузится молча и падает «Ошибка инициализации модуля» при первом
// обращении (проверено и с `Перем`, и без него, и с наблюдаемым без
// `Сообщить`); клиент-серверное подтверждение остаётся. Тела модулей
// open-bsl — осознанное расширение, их инициализация ленивая на сеанс
// (та же `ModuleInitState`, что у обычного прогона).
#[allow(clippy::too_many_arguments)]
fn prepare_job(
    shared: &Arc<JobRuntimeShared>,
    engine: &crate::Engine,
    id: JobId,
    target: (u32, u16),
    params: &SerializedValueGraph,
    caller_token: Option<[u8; 16]>,
    profile_index: u32,
) -> Result<(crate::State, crate::Module, bsl_vm::ProgramExecution), JobErrorDto> {
    let catalog = engine
        .catalog()
        .expect("worker строится только с каталогом");
    let callee = catalog
        .modules
        .get(target.0 as usize)
        .and_then(|module| module.program.chunks.get(target.1 as usize))
        .ok_or_else(|| JobErrorDto::from_text("цель задания вне каталога"))?;
    let n_params = callee.n_params as usize;
    let argc = params.root_count();
    let mut modes = Vec::with_capacity(n_params);
    for position in 0..n_params {
        if position < argc {
            modes.push(bsl_bytecode::ArgMode::Value);
        } else if callee.param_has_default.get(position) == Some(&true) {
            // Хвост без аргументов берёт умолчания; точная политика
            // платформы — за замером JOB.EXECUTE.DEFAULTS.
            modes.push(bsl_bytecode::ArgMode::Default);
        } else {
            return Err(JobErrorDto::from_text(format!(
                "цели передано {argc} аргументов, а обязательных параметров {n_params}"
            )));
        }
    }
    if argc > n_params {
        return Err(JobErrorDto::from_text(format!(
            "цели передано {argc} аргументов при {n_params} параметрах"
        )));
    }
    let base_program = &catalog.modules[0].program;
    // Аргументы материализуются ЗАРАНЕЕ интернером, который станет
    // таблицами entry-программы: `NameId` внутри значений согласованы с
    // `RuntimeShapes::seeded` прогона. Сами значения едут константами
    // чанка и раскладываются по регистрам обычными `LoadConst`.
    let mut shapes = bsl_rt::RuntimeShapes::seeded(
        base_program.names.clone(),
        base_program.shapes.clone(),
        Some(engine.registry()),
    );
    let arguments = params
        .materialize(&mut shapes)
        .map_err(|error| JobErrorDto::from_text(error.to_string()))?;
    let names = shapes.names.into_names();
    let shape_table = shapes.shapes.into_shapes();

    let mut instrs = Vec::with_capacity(argc + 2);
    for position in 0..argc {
        instrs.push(bsl_bytecode::Instr::LoadConst {
            dst: position as u8,
            k: position as u16,
        });
    }
    instrs.push(bsl_bytecode::Instr::CallImported {
        link_slot: 0,
        base: 0,
        arg_modes: 0,
        ret: n_params as u8,
    });
    instrs.push(bsl_bytecode::Instr::Return { src: None });
    let regs = (n_params + 1).max(1) as u8;
    let chunk = bsl_bytecode::Chunk {
        instrs,
        // Таблица констант здесь — ТРАНСПОРТ аргументов, а не
        // литеральный пул: значения задания материализуются в объекты и
        // типы, которых текстовый формат не представляет. Программа
        // собирается в памяти и не печатается — сериализуется каталог с
        // `entry: None`, — поэтому непредставимость ей ничем не грозит.
        consts: arguments
            .into_iter()
            .map(bsl_bytecode::BytecodeConst::transient)
            .collect(),
        call_arg_modes: vec![modes],
        exception_ranges: Vec::new(),
        n_params: 0,
        param_by_val: Vec::new(),
        param_has_default: Vec::new(),
        is_procedure: false,
        is_async: false,
        n_locals: n_params as u8,
        n_regs: regs,
        prop_cache: Vec::new(),
        method_cache: Vec::new(),
        local_names: Vec::new(),
        bundle_len: Vec::new(),
        touches_objects: false,
    };
    let mut entry = bsl_bytecode::Program {
        requirements: base_program.requirements.clone(),
        chunks: vec![chunk],
        names,
        shapes: shape_table,
        top_level_locals: (0..n_params).map(|i| format!("Параметр{i}")).collect(),
        function_names: Vec::new(),
        exported_functions: Vec::new(),
        module_vars: Vec::new(),
        exported_module_vars: Vec::new(),
        module_base: 0,
        links: vec![bsl_bytecode::LinkEntry::Function {
            module: bsl_bytecode::ModuleId::new(target.0),
            func: target.1,
        }],
    };
    bsl_bytecode::image::finalize(&mut entry);
    let module = engine
        .load_entry(bsl_bytecode::EntryProgram {
            id: bsl_bytecode::EntryId::new(0),
            program: entry,
        })
        .map_err(|error| JobErrorDto::from_text(error.to_string()))?;

    // Сеанс задания строит выбранный host-профиль: системный (0) — это
    // process-default сервисы worker-движка, зарегистрированный — его
    // фабрика. Ошибка фабрики без паники завершает ТОЛЬКО этот job как
    // `Failed` с кодом `HostProfileUnavailable`; worker остаётся жив.
    let state_builder = match profile_index.checked_sub(1) {
        None => engine.state_builder(),
        Some(index) => {
            let Some(factory) = shared.profiles.get(index as usize) else {
                // Защитная ветка: выбор профиля валидируется на
                // `StateBuilder::host_profile`, сюда попадает только
                // рассинхронизация таблиц.
                return Err(JobErrorDto::from_text(
                    "host-профиль недоступен: не зарегистрирован в этом движке",
                ));
            };
            factory.configure(engine.state_builder()).map_err(|text| {
                JobErrorDto::from_text(format!("host-профиль недоступен: {text}"))
            })?
        }
    };
    let mut state = state_builder.build();
    // stdout задания — сток: `Сообщить` идёт через sink сообщений ниже, а
    // прямых писателей stdout в фоновом сеансе не остаётся.
    state.host.stdout = Box::new(std::io::sink());
    // `Сообщить` задания кладёт владеющий DTO в FIFO-историю записи (её
    // читает `ПолучитьСообщенияПользователю`), затем — внешнему sink
    // представления, если он зарегистрирован.
    state
        .host
        .env
        .set_message_sink(std::rc::Rc::new(JobMessageRoute {
            shared: Arc::clone(shared),
            id,
        }));
    // Сеанс задания получает СВОЁ временное хранилище со ссылкой на
    // вызывателя: запись по его адресу — staging до terminal, под
    // per-job и глобальным кредитами. Mailbox сеанса регистрируется в
    // ОБЩЕМ hub движка: дочернее задание публикует write-set сюда — и
    // только сюда, транзитивного повышения capability нет.
    let session_token = random_uuid();
    {
        let session = std::rc::Rc::new(std::cell::RefCell::new(match caller_token {
            Some(caller) => bsl_rt::TempStorageSession::for_job(
                session_token,
                caller,
                state.host.env.random(),
                bsl_rt::StagingBudget::new(
                    shared.config.max_staged_temp_bytes_per_job,
                    Arc::clone(&shared.staging_global),
                ),
            ),
            None => bsl_rt::TempStorageSession::new(session_token, state.host.env.random()),
        }));
        shared
            .temp_hub
            .register(session_token, session.borrow().mailbox());
        state.host.env.set_temp_storage(session);
    }
    // Вложенные задания идут в ОБЩИЙ реестр родительского runtime, а не в
    // пул воркерного движка: сервис сеанса подменяется worker-обёрткой с
    // helping-ожиданием. Профиль наследуется как есть — повысить
    // возможности дочернему заданию негде.
    state
        .host
        .env
        .set_background_jobs(std::rc::Rc::new(WorkerJobService {
            shared: Arc::clone(shared),
            engine: engine.clone(),
            session_token,
            profile_index,
        }));
    let mut vm = bsl_vm::ProgramExecution::start_with_registry_and_scheduler(
        &module.program,
        engine.registry(),
        bsl_vm::JitMode::Off,
        &state.host.env,
        state.scheduler,
    )
    .map_err(|error| JobErrorDto::from_text(error.to_string()))?;
    let catalog = engine
        .catalog()
        .expect("worker строится только с каталогом");
    vm.attach_catalog(catalog);
    // Фоновый прогон всегда квантуется: бюджетный poll возвращает
    // управление драйверу, и worker чередует резидентов.
    vm.set_always_scheduled(true);
    Ok((state, module, vm))
}

/// Разрешает цель `Модуль.Метод` по каталогу: неглобальный общий модуль,
/// экспортная не-async функция или процедура — тестовая поверхность;
/// рабочий путь идёт через `TargetTable::resolve` (ИЗМЕРЕНО,
/// `JOB.EXECUTE.VALIDATION`: синхронная валидация цели).
#[cfg(test)]
pub(crate) fn resolve_target(
    catalog: &bsl_bytecode::ConfigurationProgram,
    method_name: &str,
) -> Result<(u32, u16), String> {
    let Some((module_name, function_name)) = method_name.split_once('.') else {
        return Err(format!("цель «{method_name}» не в форме «Модуль.Метод»"));
    };
    let Some((module_id, module)) = catalog.find(module_name.trim()) else {
        return Err(format!("общий модуль «{module_name}» не найден"));
    };
    let program = &module.program;
    let function_name = function_name.trim();
    let Some(index) = program
        .function_names
        .iter()
        .position(|name| bsl_rt::folded_eq(name, function_name))
    else {
        return Err(format!(
            "в модуле «{module_name}» нет метода «{function_name}»"
        ));
    };
    if !program.exported_functions[index] {
        return Err(format!(
            "метод «{module_name}.{function_name}» не экспортирован"
        ));
    }
    let chunk = (index + 1) as u16;
    if program.chunks[chunk as usize].is_async {
        return Err(
            "асинхронная цель фонового задания не поддержана: у платформы \
             «Асинх» в общем модуле ломает инициализацию всего модуля"
                .to_string(),
        );
    }
    Ok((module_id.index() as u32, chunk))
}

/// Runtime для движка с каталогом: рецепт — публичный текстовый образ
/// каталога без entry.
pub(crate) fn runtime_for_engine(
    engine: &crate::Engine,
    config: BackgroundJobConfig,
) -> Result<JobRuntime, String> {
    config.validate()?;
    let catalog = engine
        .catalog()
        .ok_or("фоновые задания требуют движка с каталогом общих модулей")?;
    let image = bsl_bytecode::BytecodeImage::Configuration {
        catalog: catalog.clone(),
        entry: None,
    };
    let text = bsl_bytecode::write_image(&image, None).map_err(|e| e.to_string())?;
    Ok(JobRuntime::new(
        config,
        WorkerRecipe {
            image_text: Arc::from(text),
            libraries: engine.extra_libraries().to_vec(),
            symbols: engine.preproc_symbols(),
        },
        TargetTable::from_catalog(catalog),
        Arc::new(SystemJobIds),
        Arc::clone(engine.temp_hub()),
        engine.job_profiles(),
        engine.job_message_display(),
    ))
}

#[cfg(test)]
mod pool_tests {
    use super::*;
    use bsl_rt::{GraphLimits, RuntimeShapes};

    fn engine() -> crate::Engine {
        crate::Engine::builder()
            .common_module(
                "Служебный",
                "Перем Счётчик Экспорт;\n\
                 Функция Сложить(Знач а, Знач б) Экспорт\n\
                     Возврат а + б;\n\
                 КонецФункции\n\
                 Процедура Упасть() Экспорт\n\
                     ВызватьИсключение \"задание падает\";\n\
                 КонецПроцедуры\n\
                 Процедура Вечно() Экспорт\n\
                     Пока Истина Цикл\n\
                     КонецЦикла;\n\
                 КонецПроцедуры\n\
                 Счётчик = 0;",
            )
            .build()
            .expect("движок с каталогом")
    }

    fn params(values: &[bsl_rt::BslValue]) -> Arc<SerializedValueGraph> {
        let rt = RuntimeShapes::seeded(Vec::new(), Vec::new(), None);
        Arc::new(
            SerializedValueGraph::capture(values, &rt, &GraphLimits::default())
                .expect("снимок параметров"),
        )
    }

    fn number(value: i64) -> bsl_rt::BslValue {
        bsl_rt::BslValue::number_from_i64(value)
    }

    /// Критерий готовности плана: Engine без заданий не создаёт
    /// OS-потоков — пул поднимается лениво, первым admission.
    #[test]
    fn the_pool_spawns_no_threads_before_the_first_admission() {
        let engine = engine();
        let runtime = runtime_for_engine(&engine, BackgroundJobConfig::default())
            .expect("runtime собирается");
        assert!(
            runtime
                .shared
                .threads
                .lock()
                .expect("список потоков без отравления")
                .is_empty(),
            "до первого admission пул обязан быть пустым"
        );
        let target = resolve_target(engine.catalog().unwrap(), "Служебный.Сложить")
            .expect("цель разрешается");
        let snapshot = runtime
            .submit(
                "Служебный.Сложить",
                target,
                params(&[number(1), number(2)]),
                None,
                None,
                None,
                0,
            )
            .expect("задание принято");
        assert!(
            !runtime
                .shared
                .threads
                .lock()
                .expect("список потоков без отравления")
                .is_empty(),
            "первый admission поднимает пул"
        );
        assert!(
            runtime
                .wait_terminal(&[snapshot.id], Some(Duration::from_secs(30)))
                .expect("ожидание без ошибок")
        );
    }

    /// Первый успешный admission возвращается ДО полного запуска пула:
    /// удержанный тестовый шлюз не даёт бутстрапу спавнить соседей и
    /// брать работу, а submit при этом уже вернул снимок `Queued` — ровно
    /// один поток, состояние `Starting`. После открытия шлюза пул
    /// достраивается до `workers` и доводит задание.
    #[test]
    fn the_first_admission_returns_before_the_pool_is_fully_started() {
        let engine = engine();
        let runtime = runtime_for_engine(
            &engine,
            BackgroundJobConfig {
                workers: Some(4),
                ..BackgroundJobConfig::default()
            },
        )
        .expect("runtime собирается");
        let gate: Arc<BootstrapGate> = Arc::new((Mutex::new(true), Condvar::new()));
        runtime.set_bootstrap_gate(Arc::clone(&gate));
        let target = resolve_target(engine.catalog().unwrap(), "Служебный.Сложить")
            .expect("цель разрешается");
        let snapshot = runtime
            .submit(
                "Служебный.Сложить",
                target,
                params(&[number(1), number(2)]),
                None,
                None,
                None,
                0,
            )
            .expect("задание принято при удержанном бутстрапе");
        // Submit уже вернулся, а пул стоит на шлюзе: ровно один поток
        // (бутстраппер), состояние Starting, задание ещё Queued.
        assert_eq!(
            runtime
                .shared
                .threads
                .lock()
                .expect("список потоков без отравления")
                .len(),
            1,
            "admission спавнит ровно одного бутстраппера"
        );
        {
            let registry = runtime
                .shared
                .registry
                .lock()
                .expect("реестр без отравления");
            assert_eq!(registry.state, RuntimeState::Starting);
        }
        assert_eq!(
            runtime.snapshot(snapshot.id).expect("снимок").state,
            JobStateDto::Queued,
            "до открытия шлюза задание не берётся в работу"
        );
        // Открытый шлюз достраивает пул и доводит задание.
        *gate.0.lock().expect("шлюз без отравления") = false;
        gate.1.notify_all();
        assert!(
            runtime
                .wait_terminal(&[snapshot.id], Some(Duration::from_secs(30)))
                .expect("ожидание без ошибок")
        );
        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        loop {
            let spawned = runtime
                .shared
                .threads
                .lock()
                .expect("список потоков без отравления")
                .len();
            if spawned == 4 {
                break;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "пул обязан достроиться до workers, потоков {spawned}"
            );
            std::thread::yield_now();
        }
        {
            let registry = runtime
                .shared
                .registry
                .lock()
                .expect("реестр без отравления");
            assert_eq!(registry.state, RuntimeState::Running);
        }
        assert_eq!(
            runtime.snapshot(snapshot.id).expect("снимок").state,
            JobStateDto::Completed
        );
    }

    /// Shutdown во время `Starting` безопасен: бутстраппер снимается и со
    /// шлюза, очередь отменяется, ни один поток не отсоединяется, новые
    /// submissions получают ловимую ошибку закрытого runtime.
    #[test]
    fn a_shutdown_during_starting_is_safe() {
        let engine = engine();
        let runtime = runtime_for_engine(
            &engine,
            BackgroundJobConfig {
                workers: Some(4),
                ..BackgroundJobConfig::default()
            },
        )
        .expect("runtime собирается");
        let gate: Arc<BootstrapGate> = Arc::new((Mutex::new(true), Condvar::new()));
        runtime.set_bootstrap_gate(Arc::clone(&gate));
        let target = resolve_target(engine.catalog().unwrap(), "Служебный.Сложить")
            .expect("цель разрешается");
        let snapshot = runtime
            .submit(
                "Служебный.Сложить",
                target,
                params(&[number(1), number(2)]),
                None,
                None,
                None,
                0,
            )
            .expect("задание принято");
        let report = runtime.shutdown(Duration::from_secs(10));
        assert_eq!(
            report.detached_workers, 0,
            "удержанный шлюзом бутстраппер обязан выйти по закрытию"
        );
        assert_eq!(
            runtime.snapshot(snapshot.id).expect("снимок").state,
            JobStateDto::Canceled,
            "queued-задание отменяется закрытием"
        );
        let error = runtime
            .submit(
                "Служебный.Сложить",
                target,
                params(&[number(1), number(2)]),
                None,
                None,
                None,
                0,
            )
            .expect_err("закрытый runtime отвергает задания");
        assert_eq!(error.code, bsl_rt::HostErrorCode::RuntimeClosed);
    }

    #[test]
    fn a_job_runs_to_completion_in_a_worker() {
        let engine = engine();
        let runtime = runtime_for_engine(&engine, BackgroundJobConfig::default())
            .expect("runtime собирается");
        let target = resolve_target(engine.catalog().unwrap(), "Служебный.Сложить")
            .expect("цель разрешается");
        let snapshot = runtime
            .submit(
                "Служебный.Сложить",
                target,
                params(&[number(2), number(3)]),
                None,
                None,
                None,
                0,
            )
            .expect("задание принято");
        assert_eq!(snapshot.state, JobStateDto::Queued);
        assert!(
            runtime
                .wait_terminal(&[snapshot.id], Some(Duration::from_secs(30)))
                .expect("ожидание без ошибок")
        );
        let done = runtime
            .snapshot(snapshot.id)
            .expect("снимок после terminal");
        assert_eq!(
            done.state,
            JobStateDto::Completed,
            "ошибка: {:?}",
            done.error
        );
    }

    #[test]
    fn a_raising_job_finishes_as_failed_with_the_error_text() {
        let engine = engine();
        let runtime = runtime_for_engine(&engine, BackgroundJobConfig::default())
            .expect("runtime собирается");
        let target = resolve_target(engine.catalog().unwrap(), "Служебный.Упасть")
            .expect("цель разрешается");
        let snapshot = runtime
            .submit("Служебный.Упасть", target, params(&[]), None, None, None, 0)
            .expect("задание принято");
        assert!(
            runtime
                .wait_terminal(&[snapshot.id], Some(Duration::from_secs(30)))
                .expect("ожидание без ошибок")
        );
        let done = runtime.snapshot(snapshot.id).expect("снимок");
        assert_eq!(done.state, JobStateDto::Failed);
        let error = done.error.as_ref().expect("ошибка задания");
        assert!(
            error.brief.contains("задание падает"),
            "не тот текст: {}",
            error.brief
        );
    }

    #[test]
    fn missing_required_arguments_fail_the_job() {
        let engine = engine();
        let runtime = runtime_for_engine(&engine, BackgroundJobConfig::default())
            .expect("runtime собирается");
        let target = resolve_target(engine.catalog().unwrap(), "Служебный.Сложить")
            .expect("цель разрешается");
        let snapshot = runtime
            .submit(
                "Служебный.Сложить",
                target,
                params(&[number(1)]),
                None,
                None,
                None,
                0,
            )
            .expect("задание принято");
        assert!(
            runtime
                .wait_terminal(&[snapshot.id], Some(Duration::from_secs(30)))
                .expect("ожидание без ошибок")
        );
        let done = runtime.snapshot(snapshot.id).expect("снимок");
        assert_eq!(done.state, JobStateDto::Failed);
    }

    #[test]
    fn target_resolution_rejects_unknown_and_non_exported() {
        let engine = engine();
        let catalog = engine.catalog().unwrap();
        assert!(resolve_target(catalog, "Нет.Такого").is_err());
        assert!(resolve_target(catalog, "Служебный.НетМетода").is_err());
        assert!(resolve_target(catalog, "БезТочки").is_err());
        resolve_target(catalog, "Служебный.Сложить").expect("экспортная цель годится");
    }

    #[test]
    fn many_jobs_all_reach_terminal_states() {
        let engine = engine();
        let runtime = runtime_for_engine(
            &engine,
            BackgroundJobConfig {
                workers: Some(2),
                ..BackgroundJobConfig::default()
            },
        )
        .expect("runtime собирается");
        let target = resolve_target(engine.catalog().unwrap(), "Служебный.Сложить")
            .expect("цель разрешается");
        let ids: Vec<_> = (0..16)
            .map(|i| {
                runtime
                    .submit(
                        "Служебный.Сложить",
                        target,
                        params(&[number(i), number(i)]),
                        None,
                        None,
                        None,
                        0,
                    )
                    .expect("задание принято")
                    .id
            })
            .collect();
        assert!(
            runtime
                .wait_terminal(&ids, Some(Duration::from_secs(60)))
                .expect("ожидание без ошибок")
        );
        for id in ids {
            assert_eq!(
                runtime.snapshot(id).expect("снимок").state,
                JobStateDto::Completed
            );
        }
    }

    /// Закрытие runtime в окне «задание завершилось, право на terminal
    /// ещё не забрано» ОТКАТЫВАЕТ публикацию: claim не выдаётся, запись
    /// становится «Отменено», а данные в сеанс-получатель не попадают —
    /// как требует строка «shutdown — rollback» матрицы публикаций.
    #[test]
    fn a_shutdown_during_the_commit_window_rolls_back() {
        let engine = crate::Engine::builder()
            .common_module(
                "Служебный",
                "Процедура Пишет(Знач Адрес) Экспорт\n\
                     ПоместитьВоВременноеХранилище(\"из задания\", Адрес);\n\
                 КонецПроцедуры",
            )
            .build()
            .expect("движок с каталогом");
        let runtime = runtime_for_engine(&engine, BackgroundJobConfig::default())
            .expect("runtime собирается");
        // Живой сеанс-получатель: публикация, если бы она случилась,
        // была бы видна в его mailbox.
        let caller_token = [7u8; 16];
        let caller = std::rc::Rc::new(std::cell::RefCell::new(bsl_rt::TempStorageSession::new(
            caller_token,
            bsl_rt::HostEnv::process().random(),
        )));
        engine
            .temp_hub()
            .register(caller_token, caller.borrow().mailbox());
        let address = format!(
            "e1cib/tempstorage/{}?seanceId={}",
            bsl_rt::uuid::format(&[9u8; 16]),
            bsl_rt::uuid::format(&caller_token)
        );
        let target =
            resolve_target(engine.catalog().unwrap(), "Служебный.Пишет").expect("цель разрешается");
        let snapshot = runtime
            .submit(
                "Служебный.Пишет",
                target,
                params(&[bsl_rt::BslValue::Str(bsl_rt::BslString::from_str(&address))]),
                None,
                None,
                Some(caller_token),
                0,
            )
            .expect("задание принято");
        let gate = Arc::new(CommitWindowGate {
            target: snapshot.id,
            held: Mutex::new(true),
            released: Condvar::new(),
            entered: std::sync::atomic::AtomicBool::new(false),
        });
        *COMMIT_WINDOW_GATE.lock().expect("шлюз без отравления") = Some(Arc::clone(&gate));
        // Ждём, пока задание отработает и встанет в окно перед claim.
        let deadline = std::time::Instant::now() + Duration::from_secs(30);
        while !gate.entered.load(std::sync::atomic::Ordering::SeqCst) {
            assert!(
                std::time::Instant::now() < deadline,
                "задание не дошло до окна публикации"
            );
            std::thread::yield_now();
        }
        // Закрытие ИМЕННО в окне: право на terminal больше не выдаётся.
        // Само закрытие идёт в отдельном потоке — оно соединяет workers,
        // а наш worker стоит в шлюзе и выйдет только после его снятия.
        let closing = Arc::clone(&runtime.shared);
        let shutdown = std::thread::spawn(move || {
            let mut registry = closing.registry.lock().expect("реестр без отравления");
            registry.state = RuntimeState::Closed;
            drop(registry);
            closing.work_available.notify_all();
            closing.terminal_watch.notify_all();
        });
        shutdown.join().expect("поток закрытия");
        *gate.held.lock().expect("шлюз без отравления") = false;
        gate.released.notify_all();
        let report = runtime.shutdown(Duration::from_secs(30));
        assert_eq!(report.detached_workers, 0);
        *COMMIT_WINDOW_GATE.lock().expect("шлюз без отравления") = None;

        let done = runtime.snapshot(snapshot.id).expect("снимок");
        assert_eq!(
            done.state,
            JobStateDto::Canceled,
            "закрытие обязано откатить задание, а не завершить его успешно"
        );
        let mut shapes = bsl_rt::RuntimeShapes::seeded(Vec::new(), Vec::new(), None);
        let seen = caller
            .borrow()
            .get(&address, &mut shapes)
            .expect("чтение адреса");
        assert!(
            matches!(seen, bsl_rt::BslValue::Undefined),
            "write-set не имеет права попасть к получателю после закрытия: {seen:?}"
        );
    }

    /// Переход `Queued -> Running` — первое изменение статуса по семантике
    /// менеджерного `ОжидатьЗавершенияВыполнения`: ожидание с таймаутом
    /// обязано проснуться на нём, а не досидеть до дедлайна.
    #[test]
    fn a_queued_to_running_transition_wakes_the_manager_wait() {
        let engine = engine();
        let runtime = runtime_for_engine(
            &engine,
            BackgroundJobConfig {
                workers: Some(1),
                ..BackgroundJobConfig::default()
            },
        )
        .expect("runtime собирается");
        let target =
            resolve_target(engine.catalog().unwrap(), "Служебный.Вечно").expect("цель разрешается");
        let snapshot = runtime
            .submit("Служебный.Вечно", target, params(&[]), None, None, None, 0)
            .expect("задание принято");
        // Задание не завершится само: единственное изменение статуса —
        // старт. Без пробуждения на нём ожидание досидело бы до дедлайна.
        let started = std::time::Instant::now();
        let held = runtime
            .shared
            .wait_first_change(
                &[(snapshot.id, JobStateDto::Queued)],
                Some(Duration::from_secs(20)),
            )
            .expect("ожидание без ошибок");
        let elapsed = started.elapsed();
        assert_eq!(held.len(), 1);
        assert_eq!(
            held[0].state,
            JobStateDto::Running,
            "ожидание обязано вернуть свежий снимок «Активно»"
        );
        assert!(
            elapsed < Duration::from_secs(10),
            "ожидание досидело до дедлайна ({elapsed:?}) — переход в Running не будит"
        );
        runtime.cancel(snapshot.id).expect("отмена");
        assert!(
            runtime
                .wait_terminal(&[snapshot.id], Some(Duration::from_secs(30)))
                .expect("ожидание без ошибок")
        );
    }

    /// Общая сцена окна helping-ожидания: задание принято без пула,
    /// очередь вычищена, запись переведена в Running; тестовый шлюз
    /// держит ожидание МЕЖДУ проверкой предиката и парковкой, задание
    /// завершается и notify уходит именно в этом окне.
    fn helping_window_scenario(first_change: bool) {
        let engine = engine();
        let runtime = runtime_for_engine(&engine, BackgroundJobConfig::default())
            .expect("runtime собирается");
        let target = resolve_target(engine.catalog().unwrap(), "Служебный.Сложить")
            .expect("цель разрешается");
        let snapshot = runtime
            .shared
            .submit_shared(
                "Служебный.Сложить",
                target,
                params(&[number(1), number(2)]),
                None,
                None,
                None,
                0,
            )
            .expect("задание принято");
        let id = snapshot.id;
        {
            let mut registry = runtime
                .shared
                .registry
                .lock()
                .expect("реестр без отравления");
            registry.queue.clear();
            registry
                .record_mut(id)
                .expect("живая запись")
                .snapshot
                .state = JobStateDto::Running;
        }
        let gate = Arc::new(HelpingWindowGate {
            target: id,
            held: Mutex::new(true),
            released: Condvar::new(),
            entered: std::sync::atomic::AtomicBool::new(false),
        });
        *HELPING_WINDOW_GATE.lock().expect("шлюз без отравления") = Some(Arc::clone(&gate));
        let (result_sender, result_receiver) = std::sync::mpsc::channel();
        let shared = Arc::clone(&runtime.shared);
        std::thread::spawn(move || {
            // Сервис не `Send` из-за движка — поток строит свой.
            let worker_engine = self::engine();
            let service = WorkerJobService {
                shared,
                engine: worker_engine,
                session_token: [5; 16],
                profile_index: 0,
            };
            let state = if first_change {
                bsl_rt::BackgroundJobService::wait_first_change(
                    &service,
                    &[(id, JobStateDto::Running)],
                    None,
                )
                .map(|held| held[0].state)
            } else {
                bsl_rt::BackgroundJobService::wait_terminal(&service, &[id], None)
                    .map(|outcome| outcome.snapshots[0].state)
            };
            let _ = result_sender.send(state);
        });
        // Дождаться входа ожидания в окно.
        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        while !gate.entered.load(std::sync::atomic::Ordering::SeqCst) {
            assert!(
                std::time::Instant::now() < deadline,
                "ожидание не вошло в окно шлюза"
            );
            std::thread::yield_now();
        }
        // Terminal transition и «теряемое» уведомление — именно в окне,
        // до начала парковки.
        {
            let mut registry = runtime
                .shared
                .registry
                .lock()
                .expect("реестр без отравления");
            registry.finish(id, JobStateDto::Completed, None, None);
        }
        runtime.shared.terminal_watch.notify_all();
        // Шлюз открывается: ожидание идёт к парковке и обязано
        // перечитать предикат под guard, а не уснуть до дедлайна.
        *gate.held.lock().expect("шлюз без отравления") = false;
        gate.released.notify_all();
        let state = result_receiver
            .recv_timeout(Duration::from_secs(10))
            .expect("terminal-событие в окне не должно теряться парковкой")
            .expect("ожидание без ошибок");
        assert_eq!(state, JobStateDto::Completed);
        *HELPING_WINDOW_GATE.lock().expect("шлюз без отравления") = None;
    }

    /// Terminal-событие в окне «предикат проверен, парковка ещё не
    /// началась» не теряется ни одной из форм helping-ожидания: полный
    /// предикат перечитывается под guard, уходящим в `wait_timeout`.
    #[test]
    fn a_terminal_event_in_the_helping_window_is_not_lost() {
        helping_window_scenario(false);
        helping_window_scenario(true);
    }

    /// Helping-ожидание первого изменения паркуется на `terminal_watch`
    /// до события: за сотни миллисекунд тихого ожидания счётчик парковок
    /// растёт на единицы, а не на сотни двухмиллисекундных тиков — тест
    /// исключает периодические пробуждения без события.
    #[test]
    fn wait_first_change_parks_without_periodic_wakeups() {
        let engine = engine();
        let runtime = runtime_for_engine(&engine, BackgroundJobConfig::default())
            .expect("runtime собирается");
        // Задание принимается в реестр БЕЗ запуска пула (submit_shared не
        // спавнит потоки), очередь вычищается, а запись переводится в
        // Running вручную: helping-ожиданию некому помогать и не от кого
        // дождаться изменения — остаётся только спать.
        let target = resolve_target(engine.catalog().unwrap(), "Служебный.Сложить")
            .expect("цель разрешается");
        let snapshot = runtime
            .shared
            .submit_shared(
                "Служебный.Сложить",
                target,
                params(&[number(1), number(2)]),
                None,
                None,
                None,
                0,
            )
            .expect("задание принято");
        {
            let mut registry = runtime
                .shared
                .registry
                .lock()
                .expect("реестр без отравления");
            registry.queue.clear();
            registry
                .record_mut(snapshot.id)
                .expect("живая запись")
                .snapshot
                .state = JobStateDto::Running;
        }
        let parks_before = HELPING_PARKS.load(std::sync::atomic::Ordering::Relaxed);
        let shared = Arc::clone(&runtime.shared);
        let id = snapshot.id;
        let waiter = std::thread::spawn(move || {
            // Сервис не `Send` из-за движка — поток строит свой.
            let worker_engine = self::engine();
            let service = WorkerJobService {
                shared,
                engine: worker_engine,
                session_token: [3; 16],
                profile_index: 0,
            };
            bsl_rt::BackgroundJobService::wait_first_change(
                &service,
                &[(id, JobStateDto::Running)],
                None,
            )
        });
        std::thread::sleep(Duration::from_millis(300));
        // Изменение публикуется штатной парой finish + notify — ожидание
        // просыпается от события, а не от таймера.
        {
            let mut registry = runtime
                .shared
                .registry
                .lock()
                .expect("реестр без отравления");
            registry.finish(id, JobStateDto::Completed, None, None);
        }
        runtime.shared.terminal_watch.notify_all();
        let held = waiter
            .join()
            .expect("поток ожидания")
            .expect("ожидание без ошибок");
        assert_eq!(held.len(), 1);
        assert_eq!(held[0].state, JobStateDto::Completed);
        let parks = HELPING_PARKS.load(std::sync::atomic::Ordering::Relaxed) - parks_before;
        assert!(
            parks <= 8,
            "за 300 мс тихого ожидания {parks} парковок — таймерный поллинг вернулся"
        );
    }

    /// Неперехваченная BSL-ошибка при закрытом сеансе-получателе: BSL-ошибка
    /// остаётся основной, отказ terminal commit — вторичной причиной в
    /// `cause`, а не молча проглоченным результатом `commit_staged`.
    #[test]
    fn a_failed_commit_after_a_bsl_error_becomes_a_secondary_cause() {
        let engine = crate::Engine::builder()
            .common_module(
                "Служебный",
                "Процедура ПишетИПадает(Знач Адрес) Экспорт\n\
                     ПоместитьВоВременноеХранилище(\"из задания\", Адрес);\n\
                     ВызватьИсключение \"падение задания\";\n\
                 КонецПроцедуры",
            )
            .build()
            .expect("движок с каталогом");
        let runtime = runtime_for_engine(&engine, BackgroundJobConfig::default())
            .expect("runtime собирается");
        // Сеанс-вызыватель регистрируется в hub и закрывается ДО запуска
        // задания: staging пройдёт (он не смотрит в hub), а terminal
        // commit гарантированно встретит закрытый mailbox — без гонок.
        let caller_token = [7u8; 16];
        {
            let session =
                bsl_rt::TempStorageSession::new(caller_token, bsl_rt::HostEnv::process().random());
            engine.temp_hub().register(caller_token, session.mailbox());
        }
        let address = format!(
            "e1cib/tempstorage/{}?seanceId={}",
            bsl_rt::uuid::format(&[9u8; 16]),
            bsl_rt::uuid::format(&caller_token)
        );
        let target = resolve_target(engine.catalog().unwrap(), "Служебный.ПишетИПадает")
            .expect("цель разрешается");
        let snapshot = runtime
            .submit(
                "Служебный.ПишетИПадает",
                target,
                params(&[bsl_rt::BslValue::Str(bsl_rt::BslString::from_str(&address))]),
                None,
                None,
                Some(caller_token),
                0,
            )
            .expect("задание принято");
        assert!(
            runtime
                .wait_terminal(&[snapshot.id], Some(Duration::from_secs(30)))
                .expect("ожидание без ошибок")
        );
        let done = runtime.snapshot(snapshot.id).expect("снимок");
        assert_eq!(done.state, JobStateDto::Failed);
        let error = done.error.as_ref().expect("ошибка задания");
        assert!(
            error.brief.contains("падение задания"),
            "основной обязана остаться BSL-ошибка: {}",
            error.brief
        );
        let mut causes = Vec::new();
        let mut tail = error.cause.as_deref();
        while let Some(cause) = tail {
            causes.push(cause.brief.clone());
            tail = cause.cause.as_deref();
        }
        assert!(
            causes
                .iter()
                .any(|cause| cause.contains("сеанс-получатель временного хранилища закрыт")),
            "отказ commit обязан быть вторичной причиной: {causes:?}"
        );
    }

    #[test]
    fn waiting_for_an_unknown_job_is_a_job_expired_error() {
        let engine = engine();
        let runtime = runtime_for_engine(&engine, BackgroundJobConfig::default())
            .expect("runtime собирается");
        // Ложный идентификатор: live-метод не притворяется успешным —
        // ловимая ошибка с кодом JobExpired.
        let error = runtime
            .wait_terminal(&[JobId([9; 16])], Some(Duration::from_millis(10)))
            .expect_err("неизвестное задание — ошибка");
        assert_eq!(error.code, bsl_rt::HostErrorCode::JobExpired);
    }
}

/// Счётчик парковок helping-ожидания первого изменения — пробник для
/// теста `wait_first_change_parks_without_periodic_wakeups`: ожидание
/// спит на `terminal_watch` до события, и за сотни миллисекунд тихого
/// ожидания счётчик растёт на единицы, а не на сотни таймерных тиков.
/// Счётчик парковок helping-ожидания — пробник для теста
/// `wait_first_change_parks_without_periodic_wakeups`: ожидание спит до
/// события, и за сотни миллисекунд тихого ожидания счётчик растёт на
/// единицы, а не на сотни таймерных тиков.
static HELPING_PARKS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Тестовый шлюз окна helping-ожидания: удерживает поток МЕЖДУ проверкой
/// предиката и парковкой — ровно там, где terminal-уведомление терялось
/// бы без повторной проверки под guard. Тест завершает задание в этом
/// окне и убеждается, что ожидание возвращается сразу, а не спит до
/// дедлайна (`a_terminal_event_in_the_helping_window_is_not_lost`).
#[cfg(test)]
struct HelpingWindowGate {
    /// Пауза срабатывает только для ожиданий, среди целей которых это
    /// задание, — параллельные тесты с другими заданиями не задеваются.
    target: JobId,
    held: Mutex<bool>,
    released: Condvar,
    entered: std::sync::atomic::AtomicBool,
}

#[cfg(test)]
static HELPING_WINDOW_GATE: Mutex<Option<Arc<HelpingWindowGate>>> = Mutex::new(None);

/// Пауза в окне «предикат проверен, парковка ещё не началась». Вне
/// тестов и без выставленного шлюза — пустышка. Держится БЕЗ лока
/// реестра: вызывается перед взятием guard парковки.
fn helping_window_pause(mode: &DriveMode<'_>) {
    #[cfg(not(test))]
    let _ = mode;
    #[cfg(test)]
    {
        let DriveMode::Until { ids, .. } = mode else {
            return;
        };
        let gate = HELPING_WINDOW_GATE
            .lock()
            .expect("шлюз без отравления")
            .clone();
        let Some(gate) = gate else {
            return;
        };
        if !ids.contains(&gate.target) {
            return;
        }
        gate.entered
            .store(true, std::sync::atomic::Ordering::SeqCst);
        let mut held = gate.held.lock().expect("шлюз без отравления");
        while *held {
            held = gate.released.wait(held).expect("шлюз без отравления");
        }
    }
}

/// Сервис worker-сеансов: тот же общий реестр, но ожидание не блокирует
/// пул — поток-родитель ПОМОГАЕТ: пока свои задания не terminal, он
/// исполняет чужие из глобальной FIFO. Это разблокирует вложенное
/// ожидание при полностью занятом пуле (два родителя, ждущие детей,
/// доводят их сами); helping не меняет ABI и наблюдаемой семантики
/// ожидания.
pub(crate) struct WorkerJobService {
    pub shared: Arc<JobRuntimeShared>,
    /// Клон worker-движка: живёт только в потоке этого worker.
    pub engine: crate::Engine,
    /// Token хранилища СВОЕГО job-сеанса: дочерние задания публикуются в
    /// него, а не в сеанс исходного foreground — транзитивного повышения
    /// capability нет.
    // НЕ ИЗМЕРЕНО(JOB.TEMP.NESTED_CAPABILITY): может ли дочерний job
    // платформы писать по адресу исходного foreground-сеанса, не замерено;
    // выбрана capability только непосредственного родителя — теснее
    // некуда, расширить после замера легче, чем сузить.
    pub session_token: [u8; 16],
    /// Host-профиль этого job-сеанса: дочерние задания наследуют его —
    /// другого профиля сервису сеанса не передать.
    pub profile_index: u32,
}

impl bsl_rt::BackgroundJobService for WorkerJobService {
    fn submit(
        &self,
        method_name: &str,
        params: Arc<SerializedValueGraph>,
        key: Option<Arc<bsl_rt::JobKeyDto>>,
        description: Option<String>,
    ) -> Result<Arc<JobSnapshotDto>, HostError> {
        self.shared
            .submit_by_name_shared(
                method_name,
                params,
                key,
                description,
                Some(self.session_token),
                self.profile_index,
            )
            .map(Arc::new)
    }

    fn snapshot(&self, id: JobId) -> Option<Arc<JobSnapshotDto>> {
        self.shared
            .registry
            .lock()
            .expect("реестр без отравления")
            .snapshot(id)
    }

    fn snapshots(&self) -> Vec<Arc<JobSnapshotDto>> {
        self.shared
            .registry
            .lock()
            .expect("реестр без отравления")
            .snapshots()
    }

    fn wait_terminal(
        &self,
        ids: &[JobId],
        timeout: Option<Duration>,
    ) -> Result<bsl_rt::JobWaitOutcome, HostError> {
        let deadline = timeout.map(|t| std::time::Instant::now() + t);
        {
            let registry = self.shared.registry.lock().expect("реестр без отравления");
            JobRuntimeShared::live_guard(&registry)?;
            for id in ids {
                JobRuntimeShared::known_guard(&registry, *id)?;
            }
        }
        // Ожидание — драйвер пула СВОЕГО потока: он двигает и соседей
        // (в том числе припаркованных на host-операциях, довести которых
        // может только этот поток), и глобальную очередь, а спит лишь
        // когда двигать нечего. Предикат вычисляется под тем же локом,
        // под которым принимается решение спать.
        let done = |registry: &JobRegistry| JobRuntimeShared::all_terminal(registry, ids);
        drive_local(
            &self.shared,
            &self.engine,
            DriveMode::Until {
                ids,
                done: &done,
                deadline,
            },
        );
        let registry = self.shared.registry.lock().expect("реестр без отравления");
        // Закрытие runtime во время ожидания — ловимая ошибка, а не
        // молчаливый таймаут.
        JobRuntimeShared::live_guard(&registry)?;
        Ok(bsl_rt::JobWaitOutcome {
            completed: JobRuntimeShared::all_terminal(&registry, ids),
            snapshots: JobRuntimeShared::held_snapshots(&registry, ids)?,
        })
    }

    fn cancel(&self, id: JobId) -> Result<(), HostError> {
        let end = self.shared.time_source.wall_now();
        self.shared.cancel(id, end)
    }

    fn take_messages(&self, id: JobId, remove: bool) -> Result<Vec<UserMessageDto>, HostError> {
        self.shared.take_messages(id, remove)
    }

    fn graph_limits(&self) -> bsl_rt::GraphLimits {
        graph_limits_for(&self.shared.config)
    }

    fn wait_first_change(
        &self,
        jobs: &[(JobId, bsl_rt::JobStateDto)],
        timeout: Option<Duration>,
    ) -> Result<Vec<Arc<JobSnapshotDto>>, HostError> {
        // Worker не блокируется впустую: ожидание двигает пул своего
        // потока и глобальную очередь (см. `drive_local`).
        let deadline = timeout.map(|t| std::time::Instant::now() + t);
        let ids: Vec<JobId> = jobs.iter().map(|(id, _)| *id).collect();
        {
            let registry = self.shared.registry.lock().expect("реестр без отравления");
            JobRuntimeShared::live_guard(&registry)?;
            for id in &ids {
                JobRuntimeShared::known_guard(&registry, *id)?;
            }
        }
        let done = |registry: &JobRegistry| {
            // Вытеснение во время ожидания прекращает сон: внешняя
            // проверка ниже вернёт по нему ловимую `JobExpired`.
            let Ok(held) = JobRuntimeShared::held_snapshots(registry, &ids) else {
                return true;
            };
            let (any_active, any_changed, all_terminal, any_failed) =
                JobRuntimeShared::first_change_flags(jobs, &held);
            if !any_active {
                return true;
            }
            match deadline {
                Some(_) => any_changed,
                None => all_terminal || any_failed,
            }
        };
        drive_local(
            &self.shared,
            &self.engine,
            DriveMode::Until {
                ids: &ids,
                done: &done,
                deadline,
            },
        );
        let registry = self.shared.registry.lock().expect("реестр без отравления");
        JobRuntimeShared::live_guard(&registry)?;
        JobRuntimeShared::held_snapshots(&registry, &ids)
    }
}

/// Мост сервиса: `Rc`-обёртка над разделяемым runtime — то, что native
/// внедряет в `HostEnv` каждого сеанса движка с каталогом.
pub(crate) struct EngineJobService {
    pub runtime: Arc<JobRuntime>,
    /// Token временного хранилища сеанса-вызывателя: задания публикуют
    /// write-set'ы в его mailbox.
    pub caller_token: [u8; 16],
    /// Host-профиль, выбранный `StateBuilder::host_profile`: задания
    /// этого сеанса и их потомки строят host-окружение по нему.
    pub profile_index: u32,
}

impl bsl_rt::BackgroundJobService for EngineJobService {
    fn submit(
        &self,
        method_name: &str,
        params: Arc<SerializedValueGraph>,
        key: Option<Arc<bsl_rt::JobKeyDto>>,
        description: Option<String>,
    ) -> Result<Arc<JobSnapshotDto>, HostError> {
        self.runtime
            .submit_by_name_with_caller(
                method_name,
                params,
                key,
                description,
                Some(self.caller_token),
                self.profile_index,
            )
            .map(Arc::new)
    }

    fn snapshot(&self, id: JobId) -> Option<Arc<JobSnapshotDto>> {
        self.runtime.snapshot(id)
    }

    fn snapshots(&self) -> Vec<Arc<JobSnapshotDto>> {
        self.runtime.snapshots()
    }

    fn wait_terminal(
        &self,
        ids: &[JobId],
        timeout: Option<Duration>,
    ) -> Result<bsl_rt::JobWaitOutcome, HostError> {
        self.runtime.shared.wait_terminal_blocking(ids, timeout)
    }

    fn cancel(&self, id: JobId) -> Result<(), HostError> {
        self.runtime.cancel(id)
    }

    fn take_messages(&self, id: JobId, remove: bool) -> Result<Vec<UserMessageDto>, HostError> {
        self.runtime.shared.take_messages(id, remove)
    }

    fn graph_limits(&self) -> bsl_rt::GraphLimits {
        graph_limits_for(&self.runtime.shared.config)
    }

    fn wait_first_change(
        &self,
        jobs: &[(JobId, bsl_rt::JobStateDto)],
        timeout: Option<Duration>,
    ) -> Result<Vec<Arc<JobSnapshotDto>>, HostError> {
        self.runtime.shared.wait_first_change(jobs, timeout)
    }
}

/// Бюджет сериализации графов параметров и ключа: admission всё равно
/// отвергнет запись больше `max_single_job_record_bytes`, поэтому
/// сериализатор останавливается на нём до крупной аллокации.
fn graph_limits_for(config: &BackgroundJobConfig) -> bsl_rt::GraphLimits {
    let default_limit = bsl_rt::GraphLimits::default().max_bytes;
    bsl_rt::GraphLimits {
        max_bytes: config.max_single_job_record_bytes.min(default_limit),
    }
}
