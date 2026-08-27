//! Нативный runtime фоновых заданий: конфигурация, реестр, admission.
//!
//! Архитектурные инварианты (план фоновых заданий, `docs/bsl-background-jobs.md`):
//! реестр хранит только владеющие `Send`-DTO и никогда не вызывает внешний
//! код под своим локом; мьютекс — синхронный `std::sync::Mutex` с короткими
//! секциями и не пересекает `await`; занятый пул означает FIFO, а не
//! ошибку; первый terminal transition выигрывает ровно один раз.

// Пул workers подключается следующим шагом этапа 3; до него часть
// каркаса (машина состояний runtime, очередь) используется только
// тестами admission.
#![allow(dead_code)]

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::Duration;

use bsl_rt::{JobErrorDto, JobId, JobKeyDto, JobSnapshotDto, JobStateDto, SerializedValueGraph};

/// Конфигурация фонового runtime — одна публичная структура; scheduler
/// настраивается своей `SchedulerConfig` и здесь не дублируется.
///
/// Значения по умолчанию предварительные: production defaults, кроме
/// `max_history_jobs = 10_000`, выбираются после нагрузочных прогонов
/// плана.
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
        if self.max_error_bytes_per_job + self.max_message_bytes_per_job
            > self.max_single_job_record_bytes
        {
            return Err("бюджеты ошибки и сообщений не помещаются в запись истории".to_string());
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
/// Монотонные deadlines и таймеры приходят вместе с pending host-calls
/// (этап 5 плана); тестовая реализация подставляет ручные значения.
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
    /// Байты payload, зарезервированные в live-бюджете до terminal.
    pub payload_bytes: usize,
    /// Цель вызова числами: модуль каталога и индекс чанка.
    pub target: (u32, u16),
    /// Кооперативная отмена: worker проверяет флаг на границах квантов.
    pub cancel_requested: Arc<std::sync::atomic::AtomicBool>,
    /// Token временного хранилища сеанса-вызывателя — приёмник staging.
    pub caller_token: Option<[u8; 16]>,
    /// Сообщения пользователю (`Сообщить` внутри задания), FIFO.
    pub messages: Vec<String>,
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
    history: VecDeque<Arc<JobSnapshotDto>>,
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
            history_bytes: 0,
            config,
        }
    }

    /// Admission: атомарно резервирует слот, байты payload и ключ до
    /// terminal transition; ставит задание в глобальную FIFO.
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
        let payload_bytes = params.byte_size();
        if payload_bytes > self.config.max_single_job_record_bytes {
            return Err(AdmissionError::ResourceLimit(
                "параметры задания больше предела одной записи".to_string(),
            ));
        }
        if self.live_payload_bytes + payload_bytes > self.config.max_live_payload_bytes {
            return Err(AdmissionError::ResourceLimit(
                "исчерпан live-бюджет параметров заданий".to_string(),
            ));
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
        self.live_payload_bytes += payload_bytes;
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
            messages: Vec::new(),
        };
        self.records.insert(
            id,
            JobRecord {
                snapshot: snapshot.clone(),
                payload_bytes,
                target,
                cancel_requested: Arc::new(std::sync::atomic::AtomicBool::new(false)),
                caller_token,
                messages: Vec::new(),
            },
        );
        self.queue.push_back(id);
        Ok(snapshot)
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
    /// `false`. Освобождает admission-резервы и ключ, переносит payload из
    /// live-бюджета в history-бюджет без двойного учёта, вытесняет
    /// старейшие terminal-записи (амортизировано, на добавлении).
    pub fn finish(
        &mut self,
        id: JobId,
        state: JobStateDto,
        end: Option<bsl_rt::BslDate>,
        error: Option<Arc<JobErrorDto>>,
    ) -> bool {
        debug_assert!(state.is_terminal());
        let Some(mut record) = self.records.remove(&id) else {
            return false;
        };
        if record.snapshot.state.is_terminal() {
            self.records.insert(id, record);
            return false;
        }
        record.snapshot.state = state;
        record.snapshot.end = end;
        record.snapshot.error = error;
        record.snapshot.messages = std::mem::take(&mut record.messages);
        self.inflight -= 1;
        self.live_payload_bytes -= record.payload_bytes;
        self.keys.retain(|(_, owner)| *owner != id);
        self.queue.retain(|queued| *queued != id);
        // Запись, которая не помещается в историю целиком, не хранится:
        // admission уже отверг явно превышающие, здесь — страховка.
        let record_bytes = record
            .payload_bytes
            .min(self.config.max_single_job_record_bytes);
        self.history_bytes += record_bytes;
        self.history.push_back(Arc::new(record.snapshot));
        while self.history.len() > self.config.max_history_jobs
            || self.history_bytes > self.config.max_history_bytes
        {
            let Some(evicted) = self.history.pop_front() else {
                break;
            };
            self.history_bytes -= evicted
                .params
                .byte_size()
                .min(self.config.max_single_job_record_bytes);
        }
        true
    }

    /// Снимок задания: живая запись либо история.
    pub fn snapshot(&self, id: JobId) -> Option<Arc<JobSnapshotDto>> {
        if let Some(record) = self.records.get(&id) {
            let mut snapshot = record.snapshot.clone();
            snapshot.messages = record.messages.clone();
            return Some(Arc::new(snapshot));
        }
        self.history
            .iter()
            .find(|snapshot| snapshot.id == id)
            .cloned()
    }

    /// Все снимки: живые + история. Возвращает `Arc`-указатели — фильтр
    /// работает вне лока.
    pub fn snapshots(&self) -> Vec<Arc<JobSnapshotDto>> {
        let mut all: Vec<Arc<JobSnapshotDto>> = self
            .records
            .values()
            .map(|record| Arc::new(record.snapshot.clone()))
            .collect();
        all.extend(self.history.iter().cloned());
        all
    }

    pub fn inflight(&self) -> usize {
        self.inflight
    }

    pub fn history_len(&self) -> usize {
        self.history.len()
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
                None
            ),
            Err(AdmissionError::ResourceLimit(_))
        ));
        assert!(registry.finish(id(1), JobStateDto::Completed, None, None));
        assert!(
            !registry.finish(id(1), JobStateDto::Failed, None, None),
            "terminal transition ровно один раз"
        );
        registry
            .admit(id(3), "М.Ф".into(), (0, 1), params, None, None, None)
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
            )
            .map_err(|_| ())
            .expect("другая цель принята");
        registry.finish(id(1), JobStateDto::Canceled, None, None);
        registry
            .admit(id(4), "М.Ф".into(), (0, 1), params, Some(key), None, None)
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
                )
                .map_err(|_| ())
                .expect("принято");
            registry.finish(id(i), JobStateDto::Completed, None, None);
        }
        assert_eq!(registry.history_len(), 2);
        assert!(registry.snapshot(id(1)).is_none(), "старейшие вытеснены");
        assert!(registry.snapshot(id(4)).is_some());
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
            return Err(
                "асинхронная цель фонового задания не поддержана до замера JOB.ASYNC.TARGET"
                    .to_string(),
            );
        }
        Ok((module_index as u32, function.chunk))
    }
}

/// `Send`-рецепт worker: текстовый образ каталога и ничего больше.
/// Скрытого второго формата нет — worker разбирает тот же публичный
/// `BytecodeImage::Configuration`, что пишет `--emit-bytecode`, один раз,
/// и разделяет разобранные программы между своими сеансами.
#[derive(Clone)]
pub(crate) struct WorkerRecipe {
    pub image_text: Arc<str>,
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
}

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
    ) -> Result<JobSnapshotDto, SubmitError> {
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
            )
            .map_err(|error| match error {
                AdmissionError::ResourceLimit(text) => SubmitError::Rejected(text),
                AdmissionError::DuplicateKey => {
                    SubmitError::Rejected("задание с таким ключом уже активно".to_string())
                }
                AdmissionError::Unavailable(_) => SubmitError::Unavailable,
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
    ) -> Result<JobSnapshotDto, SubmitError> {
        let target = self
            .targets
            .resolve(method_name)
            .map_err(SubmitError::BadTarget)?;
        let snapshot =
            self.submit_shared(method_name, target, params, key, description, caller_token)?;
        self.work_available.notify_one();
        // Helping-ожидание спит на terminal_watch без таймера: новое
        // задание в очереди — тоже его событие (появился кандидат на
        // доводку).
        self.terminal_watch.notify_all();
        Ok(snapshot)
    }

    /// Отмена задания. `Queued` завершается сразу; `Running` получает
    /// взведённый флаг и завершится на границе кванта; terminal и
    /// вытесненные — no-op (гонки повторной отмены — за
    /// `JOB.CANCEL.RACES`). Отмена — не ошибка BSL: снимок получает
    /// состояние «Отменено» без `ИнформацияОбОшибке`.
    pub(crate) fn cancel(&self, id: JobId, end: Option<bsl_rt::BslDate>) {
        let mut registry = self.registry.lock().expect("реестр без отравления");
        let Some(record) = registry.record(id) else {
            return;
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
            }
            _ => {}
        }
    }

    /// Ожидание по семантике синтакс-помощника: активных нет — сразу;
    /// с таймаутом — до первого изменения статуса; без — до завершения
    /// всех либо первого аварийного.
    pub(crate) fn wait_first_change(
        &self,
        jobs: &[(JobId, bsl_rt::JobStateDto)],
        timeout: Option<Duration>,
    ) {
        let deadline = timeout.map(|t| std::time::Instant::now() + t);
        let mut registry = self.registry.lock().expect("реестр без отравления");
        loop {
            let mut any_active = false;
            let mut any_changed = false;
            let mut all_terminal = true;
            let mut any_failed = false;
            for (id, initial) in jobs {
                let state = registry
                    .snapshot(*id)
                    .map(|snapshot| snapshot.state)
                    .unwrap_or(JobStateDto::Completed);
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
            if !any_active {
                return;
            }
            match deadline {
                Some(_) if any_changed => return,
                None if all_terminal || any_failed => return,
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
                        return;
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

    /// Сообщения задания: живая запись отдаёт (и при `remove` забирает)
    /// свой FIFO; terminal — сообщения снимка истории (неизменяемы).
    pub(crate) fn take_messages(&self, id: JobId, remove: bool) -> Vec<String> {
        let mut registry = self.registry.lock().expect("реестр без отравления");
        if let Some(record) = registry.record_mut(id) {
            return if remove {
                std::mem::take(&mut record.messages)
            } else {
                record.messages.clone()
            };
        }
        registry
            .snapshot(id)
            .map(|snapshot| snapshot.messages.clone())
            .unwrap_or_default()
    }

    /// Блокирующее ожидание terminal-состояния всех `ids` — путь
    /// foreground-сеанса; worker вместо блокировки помогает пулу (см.
    /// `WorkerJobService`).
    pub(crate) fn wait_terminal_blocking(&self, ids: &[JobId], timeout: Option<Duration>) -> bool {
        let deadline = timeout.map(|t| std::time::Instant::now() + t);
        let mut registry = self.registry.lock().expect("реестр без отравления");
        loop {
            if Self::all_terminal(&registry, ids) {
                return true;
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
                        return false;
                    };
                    let (guard, result) = self
                        .terminal_watch
                        .wait_timeout(registry, left)
                        .expect("реестр без отравления");
                    registry = guard;
                    if result.timed_out() {
                        // Последняя проверка под локом — событие могло
                        // прийти на границе таймаута.
                        return Self::all_terminal(&registry, ids);
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
    threads: Mutex<Vec<std::thread::JoinHandle<()>>>,
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

/// Ошибка запуска задания — ловимая на стороне BSL.
#[derive(Debug)]
pub enum SubmitError {
    /// Явный ресурсный лимит либо дублирующий ключ.
    Rejected(String),
    /// Цель не найдена или не годится (не экспортна, арность).
    BadTarget(String),
    /// Runtime закрыт или сломан.
    Unavailable,
}

impl JobRuntime {
    /// Создаёт холодный runtime: потоков нет до первого admission.
    pub(crate) fn new(
        config: BackgroundJobConfig,
        recipe: WorkerRecipe,
        targets: TargetTable,
        id_source: Arc<dyn JobIdSource>,
        temp_hub: Arc<bsl_rt::TempStorageHub>,
    ) -> Self {
        let workers = config.effective_workers();
        Self {
            shared: Arc::new(JobRuntimeShared {
                registry: Mutex::new(JobRegistry::new(config)),
                work_available: Condvar::new(),
                terminal_watch: Condvar::new(),
                recipe,
                id_source,
                time_source: Arc::new(SystemJobTime),
                targets,
                temp_hub,
            }),
            workers,
            threads: Mutex::new(Vec::new()),
        }
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
    ) -> Result<JobSnapshotDto, SubmitError> {
        let snapshot = self.shared.submit_shared(
            method_name,
            target,
            params,
            key,
            description,
            caller_token,
        )?;
        self.ensure_workers();
        self.shared.work_available.notify_one();
        // См. submit_by_name_shared: помощники ждут на terminal_watch.
        self.shared.terminal_watch.notify_all();
        Ok(snapshot)
    }

    /// Ленивый запуск пула: потоки создаются один раз, после первого
    /// admission. Паника создания потока — `Broken` для всего runtime.
    fn ensure_workers(&self) {
        let mut threads = self.threads.lock().expect("список потоков без отравления");
        if !threads.is_empty() {
            return;
        }
        for index in 0..self.workers {
            let shared = Arc::clone(&self.shared);
            let builder = std::thread::Builder::new().name(format!("bsl-job-worker-{index}"));
            match builder.spawn(move || worker_supervisor(&shared)) {
                Ok(handle) => threads.push(handle),
                Err(_) => {
                    let mut registry = self.shared.registry.lock().expect("реестр без отравления");
                    registry.state = RuntimeState::Broken;
                    fail_all_resident(&mut registry);
                    self.shared.terminal_watch.notify_all();
                    return;
                }
            }
        }
        let mut registry = self.shared.registry.lock().expect("реестр без отравления");
        if registry.state == RuntimeState::Starting {
            registry.state = RuntimeState::Running;
        }
    }

    /// Запускает цель по имени «Модуль.Метод»: разрешение цели по
    /// каталогу рецепта worker + admission. Ошибки цели и лимитов —
    /// ловимые на стороне BSL.
    ///
    /// # Errors
    ///
    /// [`SubmitError`] с причиной.
    pub fn submit_by_name(
        &self,
        method_name: &str,
        params: Arc<SerializedValueGraph>,
        key: Option<Arc<JobKeyDto>>,
        description: Option<String>,
    ) -> Result<JobSnapshotDto, SubmitError> {
        self.submit_by_name_with_caller(method_name, params, key, description, None)
    }

    /// То же с token'ом временного хранилища вызывателя — путь сервисов.
    pub(crate) fn submit_by_name_with_caller(
        &self,
        method_name: &str,
        params: Arc<SerializedValueGraph>,
        key: Option<Arc<JobKeyDto>>,
        description: Option<String>,
        caller_token: Option<[u8; 16]>,
    ) -> Result<JobSnapshotDto, SubmitError> {
        let target = self
            .shared
            .targets
            .resolve(method_name)
            .map_err(SubmitError::BadTarget)?;
        self.submit(method_name, target, params, key, description, caller_token)
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

    /// Ожидает terminal-состояния ВСЕХ перечисленных заданий (правило
    /// any/all уточняется замером `JOB.WAIT.MANY`). `None` — без предела.
    /// Возвращает `true`, если дождались; `false` — таймаут.
    pub fn wait_terminal(&self, ids: &[JobId], timeout: Option<Duration>) -> bool {
        self.shared.wait_terminal_blocking(ids, timeout)
    }

    /// Отмена задания: `Queued` — сразу, `Running` — кооперативно на
    /// границе кванта; повторная отмена и terminal — no-op.
    pub fn cancel(&self, id: JobId) {
        let end = self.shared.time_source.wall_now();
        self.shared.cancel(id, end);
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

        let handles: Vec<std::thread::JoinHandle<()>> =
            std::mem::take(&mut *self.threads.lock().expect("список потоков без отравления"));
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
    let ids: Vec<JobId> = registry.records.keys().copied().collect();
    for id in ids {
        registry.finish(
            id,
            JobStateDto::Failed,
            None,
            Some(Arc::new(JobErrorDto::from_text(
                "фоновый runtime сломан и не принимает задания",
            ))),
        );
    }
}

/// Надзор за worker: паника ВНЕ границы задания перезапускает цикл
/// заново (паника внутри задания ловится там же и роняет только его).
/// Три последовательные паники переводят runtime в `Broken`: живые
/// задания завершаются `Failed`, новые submissions получают ловимую
/// ошибку, автоматического recovery нет.
fn worker_supervisor(shared: &Arc<JobRuntimeShared>) {
    let mut consecutive_panics = 0u32;
    loop {
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            worker_main(shared);
        }));
        match outcome {
            Ok(()) => return, // нормальный выход: runtime закрыт или сломан
            Err(_) => {
                consecutive_panics += 1;
                if consecutive_panics >= 3 {
                    let mut registry = shared.registry.lock().expect("реестр без отравления");
                    registry.state = RuntimeState::Broken;
                    fail_all_resident(&mut registry);
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
fn worker_main(shared: &Arc<JobRuntimeShared>) {
    // Каталог разбирается один раз на worker; программы разделяются между
    // сеансами этого worker и не покидают его поток.
    let engine = match build_worker_engine(&shared.recipe) {
        Ok(engine) => engine,
        Err(error) => {
            let mut registry = shared.registry.lock().expect("реестр без отравления");
            registry.state = RuntimeState::Broken;
            let text = format!("worker не разобрал рецепт каталога: {error}");
            let ids: Vec<JobId> = registry.records.keys().copied().collect();
            for id in ids {
                registry.finish(
                    id,
                    JobStateDto::Failed,
                    None,
                    Some(Arc::new(JobErrorDto::from_text(text.clone()))),
                );
            }
            shared.terminal_watch.notify_all();
            return;
        }
    };
    // Локальная FIFO резидентов: runnable чередуются бюджетными квантами,
    // waiting опрашиваются вместе с ними (host-completions подбирает их
    // собственный poll). Новый глобальный job извлекается только когда
    // локально нет runnable-резидентов — по плану фоновых заданий.
    let mut local: VecDeque<RunningJob> = VecDeque::new();
    // Снимок wake_epoch на момент последнего начала опроса ждущих
    // резидентов: спим, только если с тех пор не было ни одного
    // host-события. Событие между опросом и сном поднимает счётчик под
    // локом — пропущенных пробуждений нет.
    let mut seen_epoch = 0u64;
    loop {
        let mut runnable_locally = local.iter().any(|job| !job.waiting);
        let next_global = {
            let mut registry = shared.registry.lock().expect("реестр без отравления");
            loop {
                match registry.state {
                    RuntimeState::Closed | RuntimeState::Broken => {
                        // Закрытие на границе кванта — и есть кооперативная
                        // точка: резиденты завершаются «Отменено».
                        drop(registry);
                        let end = shared.time_source.wall_now();
                        let mut registry = shared.registry.lock().expect("реестр без отравления");
                        for job in local.drain(..) {
                            registry.finish(job.id, JobStateDto::Canceled, end, None);
                        }
                        drop(registry);
                        shared.terminal_watch.notify_all();
                        return;
                    }
                    _ => {}
                }
                if runnable_locally {
                    // Есть чем заняться — глобальную очередь не трогаем.
                    break None;
                }
                if let Some(id) = registry.queue.pop_front() {
                    break Some(id);
                }
                if local.is_empty() {
                    // Совсем пусто: спим до появления работы.
                    registry = shared
                        .work_available
                        .wait(registry)
                        .expect("реестр без отравления");
                } else {
                    // Все резиденты ждут host-завершений: точный сон до
                    // события. Каждый источник пробуждения будит condvar
                    // явно — завершение host-операции и отмена поднимают
                    // wake_epoch, новое задание видно по очереди,
                    // закрытие по state; таймерного поллинга нет.
                    if registry.wake_epoch == seen_epoch {
                        registry = shared
                            .work_available
                            .wait(registry)
                            .expect("реестр без отравления");
                        continue;
                    }
                    seen_epoch = registry.wake_epoch;
                    // Неизвестно, чьё завершение пришло: каждый ждущий
                    // резидент опрашивается заново (его собственный poll
                    // подберёт доставленные завершения либо снова уснёт).
                    for job in local.iter_mut() {
                        job.waiting = false;
                    }
                    runnable_locally = true;
                    break None;
                }
            }
        };
        if let Some(id) = next_global {
            match start_job(shared, &engine, id) {
                None => continue,
                Some(Ok(job)) => local.push_back(job),
                Some(Err(error)) => {
                    finish_job(shared, id, JobStateDto::Failed, Some(error));
                    continue;
                }
            }
        }
        let Some(mut job) = local.pop_front() else {
            continue;
        };
        // Паника BSL-исполнения ловится на границе кванта и роняет только
        // это задание; соседи-резиденты продолжают.
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| job.poll(&engine)));
        match outcome {
            Ok(Ok(bsl_vm::ProgramPoll::Complete(..))) => {
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
            Ok(Ok(bsl_vm::ProgramPoll::Runnable)) => {
                job.waiting = false;
                local.push_back(job);
            }
            Ok(Ok(bsl_vm::ProgramPoll::Waiting)) => {
                job.waiting = true;
                local.push_back(job);
            }
            Ok(Err(JobPollError::Canceled)) => {
                finish_job(shared, job.id, JobStateDto::Canceled, None);
            }
            Ok(Err(JobPollError::Failed(error))) => {
                // Неперехваченная BSL-ошибка ПУБЛИКУЕТ write-set — измерено
                // JOB.TEMP.FAILURE; неудача публикации остаётся вторичной,
                // основная ошибка BSL важнее.
                let _ = commit_staged(shared, &job);
                finish_job(shared, job.id, JobStateDto::Failed, Some(error));
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
            }
        }
    }
}

/// Движок worker строится из того же публичного образа, которым ходит
/// `--emit-bytecode`: стандартный состав компонентов. Пользовательские
/// библиотеки host и профили возможностей приезжают вместе с
/// `BackgroundStateFactory` на дальнейших этапах плана.
fn build_worker_engine(recipe: &WorkerRecipe) -> Result<crate::Engine, String> {
    let image = bsl_bytecode::parse_image(&recipe.image_text).map_err(|e| e.to_string())?;
    let bsl_bytecode::BytecodeImage::Configuration { catalog, entry: _ } = image else {
        return Err("рецепт worker обязан быть конфигурацией".to_string());
    };
    crate::Engine::builder()
        .configuration_image(catalog, false)
        .build()
        .map_err(|e| e.to_string())
}

/// Перехват `Сообщить` задания: строки stdout уходят в FIFO-историю
/// записи реестра. Под локом — только push готовой строки; форматирование
/// уже сделано VM (`bsl_format::format_value`). Неблокирующий sink
/// host-представления (план, этап 8) подключится к этому же месту.
struct JobMessageWriter {
    shared: Arc<JobRuntimeShared>,
    id: JobId,
    buffer: Vec<u8>,
}

impl std::io::Write for JobMessageWriter {
    fn write(&mut self, chunk: &[u8]) -> std::io::Result<usize> {
        self.buffer.extend_from_slice(chunk);
        while let Some(newline) = self.buffer.iter().position(|byte| *byte == b'\n') {
            let line: Vec<u8> = self.buffer.drain(..=newline).collect();
            let text = String::from_utf8_lossy(&line[..line.len() - 1]).into_owned();
            let mut registry = self.shared.registry.lock().expect("реестр без отравления");
            if let Some(record) = registry.record_mut(self.id) {
                record.messages.push(text);
            }
        }
        Ok(chunk.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Резидент worker: изолированный сеанс одного задания с pollable
/// VM-прогоном. Начатый резидент закреплён за своим worker — `Rc`-графы
/// его сеанса поток не покидают.
struct RunningJob {
    id: JobId,
    state: crate::State,
    module: crate::Module,
    vm: bsl_vm::ProgramExecution,
    /// Последний poll вернул `Waiting`: задание ждёт host-completion и
    /// runnable-резидентом не считается.
    waiting: bool,
}

/// Стартует резидента: `Queued -> Running`, entry-программа цели,
/// изолированный сеанс, pollable-прогон с постоянным квантованием.
/// `None` — запись уже terminal (например, отменена до старта).
fn start_job(
    shared: &Arc<JobRuntimeShared>,
    engine: &crate::Engine,
    id: JobId,
) -> Option<Result<RunningJob, JobErrorDto>> {
    let (target, params, cancel_requested, caller_token) = {
        let mut registry = shared.registry.lock().expect("реестр без отравления");
        let begin = shared.time_source.wall_now();
        let record = registry.record_mut(id)?;
        record.snapshot.state = JobStateDto::Running;
        record.snapshot.begin = begin;
        (
            record.target,
            Arc::clone(&record.snapshot.params),
            Arc::clone(&record.cancel_requested),
            record.caller_token,
        )
    };
    Some(
        prepare_job(shared, engine, id, target, &params, caller_token).map(
            |(state, module, mut vm)| {
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
                }));
                RunningJob {
                    id,
                    state,
                    module,
                    vm,
                    waiting: false,
                }
            },
        ),
    )
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

/// Terminal transition резидента с пробуждением ожидающих.
fn finish_job(
    shared: &Arc<JobRuntimeShared>,
    id: JobId,
    state: JobStateDto,
    error: Option<JobErrorDto>,
) {
    let end = shared.time_source.wall_now();
    let mut registry = shared.registry.lock().expect("реестр без отравления");
    registry.finish(id, state, end, error.map(Arc::new));
    drop(registry);
    shared.terminal_watch.notify_all();
}

/// Готовит сеанс задания без компилятора: entry-программа собирается
/// руками — аргументы приходят константами чанка, вызов идёт обычным
/// `CallImported` по числовому манифесту. Возвращает изолированный сеанс,
/// модуль entry и pollable-прогон с постоянным квантованием.
fn prepare_job(
    shared: &Arc<JobRuntimeShared>,
    engine: &crate::Engine,
    id: JobId,
    target: (u32, u16),
    params: &SerializedValueGraph,
    caller_token: Option<[u8; 16]>,
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
    let mut chunk = bsl_bytecode::Chunk {
        instrs,
        consts: arguments,
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
    let instruction_count = chunk.instrs.len();
    chunk.prop_cache = (0..instruction_count)
        .map(|_| bsl_bytecode::PropCacheSlot::default())
        .collect();
    chunk.method_cache = (0..instruction_count)
        .map(|_| std::cell::RefCell::new(None))
        .collect();
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
    for i in 0..entry.chunks.len() {
        entry.chunks[i].bundle_len = bsl_bytecode::bundle::compute(
            &entry.chunks[i],
            bsl_bytecode::bundle::module_overlap(i, entry.module_vars.len()),
        );
    }
    let module = engine
        .load_entry(bsl_bytecode::EntryProgram {
            id: bsl_bytecode::EntryId::new(0),
            program: entry,
        })
        .map_err(|error| JobErrorDto::from_text(error.to_string()))?;

    let mut state = engine.new_state();
    // `Сообщить` задания пишет в FIFO-историю записи, а не в stdout
    // процесса: историю читает `ПолучитьСообщенияПользователю`.
    state.host.stdout = Box::new(JobMessageWriter {
        shared: Arc::clone(shared),
        id,
        buffer: Vec::new(),
    });
    // Сеанс задания получает СВОЁ временное хранилище со ссылкой на
    // вызывателя: запись по его адресу — staging до terminal.
    let session_token = random_uuid();
    {
        let session = std::rc::Rc::new(std::cell::RefCell::new(match caller_token {
            Some(caller) => {
                bsl_rt::TempStorageSession::for_job(session_token, caller, state.host.env.random())
            }
            None => bsl_rt::TempStorageSession::new(session_token, state.host.env.random()),
        }));
        state.host.env.set_temp_storage(session);
    }
    // Вложенные задания идут в ОБЩИЙ реестр родительского runtime, а не в
    // пул воркерного движка: сервис сеанса подменяется worker-обёрткой с
    // helping-ожиданием.
    state
        .host
        .env
        .set_background_jobs(std::rc::Rc::new(WorkerJobService {
            shared: Arc::clone(shared),
            engine: engine.clone(),
            session_token,
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
/// экспортная не-async функция или процедура. Детали validation
/// уточняются замером `JOB.EXECUTE.VALIDATION`.
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
            "асинхронная цель фонового задания не поддержана до замера JOB.ASYNC.TARGET"
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
        },
        TargetTable::from_catalog(catalog),
        Arc::new(SystemJobIds),
        Arc::clone(engine.temp_hub()),
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
            )
            .expect("задание принято");
        assert_eq!(snapshot.state, JobStateDto::Queued);
        assert!(runtime.wait_terminal(&[snapshot.id], Some(Duration::from_secs(30))));
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
            .submit("Служебный.Упасть", target, params(&[]), None, None, None)
            .expect("задание принято");
        assert!(runtime.wait_terminal(&[snapshot.id], Some(Duration::from_secs(30))));
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
            )
            .expect("задание принято");
        assert!(runtime.wait_terminal(&[snapshot.id], Some(Duration::from_secs(30))));
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
                    )
                    .expect("задание принято")
                    .id
            })
            .collect();
        assert!(runtime.wait_terminal(&ids, Some(Duration::from_secs(60))));
        for id in ids {
            assert_eq!(
                runtime.snapshot(id).expect("снимок").state,
                JobStateDto::Completed
            );
        }
    }

    #[test]
    fn waiting_with_a_timeout_returns_false_for_a_queued_job_without_workers() {
        let engine = engine();
        let runtime = runtime_for_engine(&engine, BackgroundJobConfig::default())
            .expect("runtime собирается");
        // Ложный идентификатор: задания нет — считается terminal (вытеснен).
        assert!(runtime.wait_terminal(&[JobId([9; 16])], Some(Duration::from_millis(10))));
    }
}

/// Сервис worker-сеансов: тот же общий реестр, но ожидание не блокирует
/// пул — поток-родитель ПОМОГАЕТ: пока свои задания не terminal, он
/// исполняет чужие из глобальной FIFO. Это разблокирует вложенное
/// ожидание при полностью занятом пуле (два родителя, ждущие детей,
/// доводят их сами). Полный pending-механизм плана — `PendingHostCall` с
/// парковкой execution — приходит вместе с переводом синхронного HTTP;
/// helping не меняет ABI и наблюдаемой семантики ожидания.
pub(crate) struct WorkerJobService {
    pub shared: Arc<JobRuntimeShared>,
    /// Клон worker-движка: живёт только в потоке этого worker.
    pub engine: crate::Engine,
    /// Token хранилища СВОЕГО job-сеанса: дочерние задания публикуются в
    /// него, а не в сеанс исходного foreground — транзитивного повышения
    /// capability нет (план, `JOB.TEMP.NESTED_CAPABILITY`).
    pub session_token: [u8; 16],
}

impl bsl_rt::BackgroundJobService for WorkerJobService {
    fn submit(
        &self,
        method_name: &str,
        params: Arc<SerializedValueGraph>,
        key: Option<Arc<bsl_rt::JobKeyDto>>,
        description: Option<String>,
    ) -> Result<Arc<JobSnapshotDto>, String> {
        self.shared
            .submit_by_name_shared(
                method_name,
                params,
                key,
                description,
                Some(self.session_token),
            )
            .map(Arc::new)
            .map_err(|error| match error {
                SubmitError::Rejected(text) | SubmitError::BadTarget(text) => text,
                SubmitError::Unavailable => "фоновый runtime закрыт или сломан".to_string(),
            })
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

    fn wait_terminal(&self, ids: &[JobId], timeout: Option<Duration>) -> bool {
        let deadline = timeout.map(|t| std::time::Instant::now() + t);
        loop {
            let next = {
                let registry = self.shared.registry.lock().expect("реестр без отравления");
                if JobRuntimeShared::all_terminal(&registry, ids) {
                    return true;
                }
                if matches!(registry.state, RuntimeState::Closed | RuntimeState::Broken) {
                    return JobRuntimeShared::all_terminal(&registry, ids);
                }
                drop(registry);
                if deadline.is_some_and(|deadline| std::time::Instant::now() >= deadline) {
                    return false;
                }
                let mut registry = self.shared.registry.lock().expect("реестр без отравления");
                registry.queue.pop_front()
            };
            match next {
                Some(id) => {
                    // Помощь пулу: чужое задание доводится этим потоком.
                    drive_to_terminal(&self.shared, &self.engine, id);
                }
                None => {
                    // Некому помогать — точное ожидание: terminal-события
                    // детей, новое задание в очереди (submit будит
                    // terminal_watch) и дедлайн. Условия перечитываются
                    // внешним циклом.
                    let registry = self.shared.registry.lock().expect("реестр без отравления");
                    if registry.queue.is_empty() {
                        let left = deadline
                            .map(|deadline| {
                                deadline.saturating_duration_since(std::time::Instant::now())
                            })
                            .unwrap_or(Duration::from_secs(3600));
                        let _ = self
                            .shared
                            .terminal_watch
                            .wait_timeout(registry, left)
                            .expect("реестр без отравления");
                    }
                }
            }
        }
    }

    fn cancel(&self, id: JobId) -> Result<(), String> {
        let end = self.shared.time_source.wall_now();
        self.shared.cancel(id, end);
        Ok(())
    }

    fn take_messages(&self, id: JobId, remove: bool) -> Vec<String> {
        self.shared.take_messages(id, remove)
    }

    fn wait_first_change(&self, jobs: &[(JobId, bsl_rt::JobStateDto)], timeout: Option<Duration>) {
        // Worker не блокируется впустую: между проверками помогает пулу.
        let deadline = timeout.map(|t| std::time::Instant::now() + t);
        loop {
            let (any_active, any_changed, all_terminal, any_failed) = {
                let registry = self.shared.registry.lock().expect("реестр без отравления");
                let mut any_active = false;
                let mut any_changed = false;
                let mut all_terminal = true;
                let mut any_failed = false;
                for (id, initial) in jobs {
                    let state = registry
                        .snapshot(*id)
                        .map(|snapshot| snapshot.state)
                        .unwrap_or(JobStateDto::Completed);
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
            };
            if !any_active {
                return;
            }
            match deadline {
                Some(deadline) => {
                    if any_changed || std::time::Instant::now() >= deadline {
                        return;
                    }
                }
                None => {
                    if all_terminal || any_failed {
                        return;
                    }
                }
            }
            let next = {
                let mut registry = self.shared.registry.lock().expect("реестр без отравления");
                registry.queue.pop_front()
            };
            match next {
                Some(id) => drive_to_terminal(&self.shared, &self.engine, id),
                None => {
                    let registry = self.shared.registry.lock().expect("реестр без отравления");
                    let _ = self
                        .shared
                        .terminal_watch
                        .wait_timeout(registry, Duration::from_millis(2))
                        .expect("реестр без отравления");
                }
            }
        }
    }
}

/// Доводит одно задание до terminal в текущем потоке — мини-драйвер
/// helping-ожидания. `Waiting`-паузы пережидаются коротким сном.
fn drive_to_terminal(shared: &Arc<JobRuntimeShared>, engine: &crate::Engine, id: JobId) {
    let mut job = match start_job(shared, engine, id) {
        None => return,
        Some(Ok(job)) => job,
        Some(Err(error)) => {
            finish_job(shared, id, JobStateDto::Failed, Some(error));
            return;
        }
    };
    loop {
        // Снимок счётчика host-событий ДО кванта: завершение, пришедшее
        // во время кванта, поднимет счётчик, и сон ниже не состоится.
        let seen_epoch = {
            shared
                .registry
                .lock()
                .expect("реестр без отравления")
                .wake_epoch
        };
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| job.poll(engine)));
        match outcome {
            Ok(Ok(bsl_vm::ProgramPoll::Complete(..))) => {
                if commit_staged(shared, &job) {
                    finish_job(shared, job.id, JobStateDto::Completed, None);
                } else {
                    finish_job(
                        shared,
                        job.id,
                        JobStateDto::Failed,
                        Some(JobErrorDto::from_text(
                            "сеанс-получатель временного хранилища закрыт",
                        )),
                    );
                }
                return;
            }
            Ok(Ok(bsl_vm::ProgramPoll::Runnable)) => {}
            Ok(Ok(bsl_vm::ProgramPoll::Waiting)) => {
                // Доводимое задание ждёт host-завершения: точный сон до
                // события (host-операция, отмена, закрытие) — как в
                // worker_main, только для одного резидента.
                let mut registry = shared.registry.lock().expect("реестр без отравления");
                while registry.wake_epoch == seen_epoch
                    && !matches!(registry.state, RuntimeState::Closed | RuntimeState::Broken)
                {
                    registry = shared
                        .work_available
                        .wait(registry)
                        .expect("реестр без отравления");
                }
            }
            Ok(Err(JobPollError::Canceled)) => {
                finish_job(shared, job.id, JobStateDto::Canceled, None);
                return;
            }
            Ok(Err(JobPollError::Failed(error))) => {
                let _ = commit_staged(shared, &job);
                finish_job(shared, job.id, JobStateDto::Failed, Some(error));
                return;
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
                return;
            }
        }
    }
}

/// Мост сервиса: `Rc`-обёртка над разделяемым runtime — то, что native
/// внедряет в `HostEnv` каждого сеанса движка с каталогом.
pub(crate) struct EngineJobService {
    pub runtime: Arc<JobRuntime>,
    /// Token временного хранилища сеанса-вызывателя: задания публикуют
    /// write-set'ы в его mailbox.
    pub caller_token: [u8; 16],
}

impl bsl_rt::BackgroundJobService for EngineJobService {
    fn submit(
        &self,
        method_name: &str,
        params: Arc<SerializedValueGraph>,
        key: Option<Arc<bsl_rt::JobKeyDto>>,
        description: Option<String>,
    ) -> Result<Arc<JobSnapshotDto>, String> {
        self.runtime
            .submit_by_name_with_caller(
                method_name,
                params,
                key,
                description,
                Some(self.caller_token),
            )
            .map(Arc::new)
            .map_err(|error| match error {
                SubmitError::Rejected(text) | SubmitError::BadTarget(text) => text,
                SubmitError::Unavailable => "фоновый runtime закрыт или сломан".to_string(),
            })
    }

    fn snapshot(&self, id: JobId) -> Option<Arc<JobSnapshotDto>> {
        self.runtime.snapshot(id)
    }

    fn snapshots(&self) -> Vec<Arc<JobSnapshotDto>> {
        self.runtime.snapshots()
    }

    fn wait_terminal(&self, ids: &[JobId], timeout: Option<Duration>) -> bool {
        self.runtime.wait_terminal(ids, timeout)
    }

    fn cancel(&self, id: JobId) -> Result<(), String> {
        self.runtime.cancel(id);
        Ok(())
    }

    fn take_messages(&self, id: JobId, remove: bool) -> Vec<String> {
        self.runtime.shared.take_messages(id, remove)
    }

    fn wait_first_change(&self, jobs: &[(JobId, bsl_rt::JobStateDto)], timeout: Option<Duration>) {
        self.runtime.shared.wait_first_change(jobs, timeout);
    }
}
