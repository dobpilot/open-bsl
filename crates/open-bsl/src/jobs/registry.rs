use super::BackgroundJobConfig;
use bsl_rt::{
    HostError, HostErrorCode, JobErrorDto, JobId, JobKeyDto, JobSnapshotDto, JobStateDto,
    SerializedValueGraph, UserMessageDto,
};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

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

/// Все живые задания — в `Failed(RuntimeBroken)`: вызывается при
/// поломке runtime под уже взятым локом.
pub(super) fn fail_all_resident(registry: &mut JobRegistry) {
    fail_resident_jobs(registry, "фоновый runtime сломан и не принимает задания");
}

/// Все живые записи — в `Failed(текст)`, КРОМЕ заклеймленных драйвером:
/// их публикация уже идёт, и исход объявит владелец claim. Иначе задание
/// оказалось бы `Failed(RuntimeBroken)` с ОПУБЛИКОВАННЫМИ данными —
/// прямое нарушение матрицы «инфраструктурный сбой — rollback».
pub(super) fn fail_resident_jobs(registry: &mut JobRegistry, text: &str) {
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
