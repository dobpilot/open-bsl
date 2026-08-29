//! BSL-поверхность фоновых заданий: менеджер `ФоновыеЗадания`, снимок
//! `ФоновоеЗадание` и объектобезопасный сервис host.
//!
//! Сервис намеренно без supertrait `Send + Sync`: native-адаптер внедряет
//! `Rc`-обёртку над `Arc`-runtime, WASM-host может дать локальную
//! реализацию. Без сервиса запуск возвращает ловимую ошибку возможности.
//! Точные имена, арности и defaults поверхности — за `JOB.API.SURFACE`.

use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use crate::{
    Arity, BslValue, CallContext, EnumValue, GraphLimits, HostError, JobId, JobKeyDto,
    JobSnapshotDto, JobStateDto, MethodDescriptor, ObjectMembersDescriptor, ObjectProtocol,
    PropertyDescriptor, RtError, RtResult, SerializedValueGraph, TypeDescriptor, UserMessageDto,
    receiver_of,
};

/// Объектобезопасный сервис host: все операции на владеющих DTO. Ошибки —
/// типизированные [`HostError`]: BSL видит один класс ловимого
/// исключения, Rust-встраивание различает причину по коду. Вытесненное
/// из истории задание отвечает `JobExpired`, закрытый и сломанный
/// runtime — `RuntimeClosed`/`RuntimeBroken`; свойства уже
/// материализованного снимка при этом остаются читаемыми.
pub trait BackgroundJobService {
    /// Запуск цели `Модуль.Метод`; снимок `Queued` при успехе.
    ///
    /// # Errors
    ///
    /// [`HostError`] с причиной отказа admission или разрешения цели.
    fn submit(
        &self,
        method_name: &str,
        params: Arc<SerializedValueGraph>,
        key: Option<Arc<JobKeyDto>>,
        description: Option<String>,
    ) -> Result<Arc<JobSnapshotDto>, HostError>;
    /// Свежий снимок по идентификатору; `None` — неизвестный/вытесненный.
    fn snapshot(&self, id: JobId) -> Option<Arc<JobSnapshotDto>>;
    /// Все снимки: живые и история.
    fn snapshots(&self) -> Vec<Arc<JobSnapshotDto>>;
    /// Ожидание terminal-состояния всех `ids`. Снимки результата сервис
    /// удерживает под тем же локом, что и финальную проверку: между
    /// решением «дождались» и чтением снимков вытеснение невозможно.
    ///
    /// # Errors
    ///
    /// `JobExpired` для неизвестного/вытесненного задания на входе и для
    /// задания, вытесненного из истории УЖЕ ВО ВРЕМЯ ожидания, —
    /// устаревший снимок не выдаётся за свежий;
    /// `RuntimeClosed`/`RuntimeBroken` после закрытия runtime.
    fn wait_terminal(
        &self,
        ids: &[JobId],
        timeout: Option<Duration>,
    ) -> Result<JobWaitOutcome, HostError>;
    /// Отмена задания; повторная отмена и отмена terminal — успешный
    /// no-op (ИЗМЕРЕНО, `JOB.CANCEL.RACES`).
    ///
    /// # Errors
    ///
    /// Как у [`BackgroundJobService::wait_terminal`].
    fn cancel(&self, id: JobId) -> Result<(), HostError>;
    /// Сообщения задания в порядке FIFO; `remove` атомарно забирает
    /// возвращаемый префикс — и у живой записи, и у terminal
    /// (ИЗМЕРЕНО, `JOB.MESSAGES`: «Истина забирает всё, после — 0»).
    ///
    /// # Errors
    ///
    /// Как у [`BackgroundJobService::wait_terminal`].
    fn take_messages(&self, id: JobId, remove: bool) -> Result<Vec<UserMessageDto>, HostError>;
    /// Семантика менеджерного `ОжидатьЗавершенияВыполнения` по
    /// синтакс-помощнику 8.3.27: если активных нет — возврат сразу; с
    /// таймаутом — до ПЕРВОГО изменения статуса любого из `jobs`
    /// относительно переданного входного состояния либо до истечения
    /// таймаута; без таймаута — до завершения всех или первого
    /// аварийного. Возвращает свежие снимки всех `jobs` в порядке
    /// запроса, удержанные под финальным локом ожидания, — размер
    /// результата всегда равен размеру запроса.
    ///
    /// # Errors
    ///
    /// Как у [`BackgroundJobService::wait_terminal`], включая
    /// `JobExpired` для задания, вытесненного во время ожидания.
    fn wait_first_change(
        &self,
        jobs: &[(JobId, JobStateDto)],
        timeout: Option<Duration>,
    ) -> Result<Vec<Arc<JobSnapshotDto>>, HostError>;
    /// Бюджет сериализации графа параметров и ключа. Admission всё равно
    /// отвергнет payload больше своей записи, поэтому сериализатор
    /// останавливается на этом пределе до крупной аллокации.
    fn graph_limits(&self) -> GraphLimits {
        GraphLimits::default()
    }
}

/// Результат ожидания terminal-состояния: признак «дождались» и свежие
/// снимки запрошенных заданий в порядке запроса. Снимки взяты под тем же
/// локом, что и финальная проверка ожидания, поэтому не бывают ни
/// устаревшими, ни «пропавшими» из-за вытеснения между проверкой и
/// чтением.
#[derive(Debug, Clone)]
pub struct JobWaitOutcome {
    /// Все запрошенные задания terminal; `false` — таймаут.
    pub completed: bool,
    /// Свежие снимки в порядке запрошенных идентификаторов.
    pub snapshots: Vec<Arc<JobSnapshotDto>>,
}

pub(crate) static BACKGROUND_JOBS_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "МенеджерФоновыхЗаданий",
    type_display: "Background jobs manager",
    type_names: &["BackgroundJobsManager"],
};

pub(crate) static BACKGROUND_JOB_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ФоновоеЗадание",
    type_display: "Background job",
    type_names: &["BackgroundJob"],
};

/// Менеджер: голое имя `ФоновыеЗадания` строит его заново при каждом
/// обращении — как `ФайловыеПотоки`.
struct BackgroundJobsManager {
    service: Rc<dyn BackgroundJobService>,
}

impl std::fmt::Debug for BackgroundJobsManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("МенеджерФоновыхЗаданий")
    }
}

/// Неизменяемый снимок задания. Свойства не обновляются; live-методы
/// (ожидание, отмена) ходят в сервис по `JobId`. `Параметры`
/// материализуются лениво при первом чтении и кэшируются в этом объекте.
struct BackgroundJobObject {
    service: Rc<dyn BackgroundJobService>,
    snapshot: Arc<JobSnapshotDto>,
}

impl std::fmt::Debug for BackgroundJobObject {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ФоновоеЗадание {}", self.snapshot.id.to_uuid_string())
    }
}

impl ObjectProtocol for BackgroundJobsManager {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &BACKGROUND_JOBS_TYPE
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        MANAGER_METHODS
    }
}

impl ObjectProtocol for BackgroundJobObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &BACKGROUND_JOB_TYPE
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        JOB_METHODS
    }

    fn property_table(&self) -> &'static [PropertyDescriptor] {
        JOB_PROPERTIES
    }

    fn get_property(&self, name: &str, ctx: &mut CallContext<'_>) -> RtResult<BslValue> {
        crate::get_property_from_table(JOB_PROPERTIES, "ФоновоеЗадание", self, name, ctx)
    }
}

/// Значение состояния снимка как член перечисления.
fn state_value(state: JobStateDto) -> BslValue {
    BslValue::Enum(match state {
        JobStateDto::Queued | JobStateDto::Running => EnumValue::BackgroundJobActive,
        JobStateDto::Completed => EnumValue::BackgroundJobCompleted,
        JobStateDto::Failed => EnumValue::BackgroundJobFailed,
        JobStateDto::Canceled => EnumValue::BackgroundJobCanceled,
    })
}

fn job_value(service: &Rc<dyn BackgroundJobService>, snapshot: Arc<JobSnapshotDto>) -> BslValue {
    BslValue::new_object(BackgroundJobObject {
        service: Rc::clone(service),
        snapshot,
    })
}

/// Таймаут ожидания из BSL-числа секунд. Ноль и дробные значения — за
/// замером `JOB.WAIT.TIMEOUT`; пока ноль означает немедленную проверку.
fn timeout_from_arg(value: Option<&BslValue>) -> RtResult<Option<Duration>> {
    let Some(value) = value else {
        return Ok(None);
    };
    if matches!(value, BslValue::Undefined) {
        return Ok(None);
    }
    let BslValue::Number(number) = value else {
        return Err(RtError::TypeError {
            expected: "Число",
            op: "ОжидатьЗавершенияВыполнения",
        });
    };
    let seconds = number.to_i64_exact().ok_or(RtError::TypeError {
        expected: "целое число секунд",
        op: "ОжидатьЗавершенияВыполнения",
    })?;
    // ИЗМЕРЕНО (JOB.WAIT.NEGATIVE): отрицательный таймаут — не ошибка,
    // вызов возвращается немедленно, как и нулевой.
    if seconds < 0 {
        return Ok(Some(Duration::ZERO));
    }
    Ok(Some(Duration::from_secs(seconds as u64)))
}

// --- Методы менеджера --------------------------------------------------

fn manager_execute(
    receiver: &dyn ObjectProtocol,
    args: &[BslValue],
    ctx: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let manager = receiver_of::<BackgroundJobsManager>(receiver, "Выполнить")?;
    let method_name = args
        .first()
        .and_then(|v| v.as_str("Выполнить").ok())
        .ok_or(RtError::TypeError {
            expected: "Строка",
            op: "Выполнить",
        })?
        .to_string();
    // Параметры — массив; снимок делается здесь, в сеансе вызывающего.
    let params: Vec<BslValue> = match args.get(1) {
        None | Some(BslValue::Undefined) => Vec::new(),
        Some(BslValue::Object(object)) => match object.as_ref() {
            crate::BslObject::Array(items) => items.borrow().clone(),
            _ => {
                return Err(RtError::TypeError {
                    expected: "Массив",
                    op: "Выполнить",
                });
            }
        },
        Some(_) => {
            return Err(RtError::TypeError {
                expected: "Массив",
                op: "Выполнить",
            });
        }
    };
    let limits = manager.service.graph_limits();
    let graph = SerializedValueGraph::capture(&params, ctx.runtime_shapes(), &limits)?;
    // Ключ по сигнатуре «Выполнить» — Строка; ИЗМЕРЕНО
    // (JOB.KEY.NON_STRING), что нестроковое значение коэрцируется в
    // строковое представление, поэтому и уникальность строковая.
    let key = match args.get(2) {
        None | Some(BslValue::Undefined) => None,
        Some(value) => {
            let text = match value {
                BslValue::Str(text) => text.to_string(),
                other => ctx.format_value(other, None)?,
            };
            let coerced = BslValue::Str(crate::BslString::from_str(&text));
            Some(Arc::new(JobKeyDto {
                graph: SerializedValueGraph::capture(
                    std::slice::from_ref(&coerced),
                    ctx.runtime_shapes(),
                    &limits,
                )?,
            }))
        }
    };
    let description = match args.get(3) {
        None | Some(BslValue::Undefined) => None,
        Some(value) => Some(value.as_str("Выполнить")?.to_string()),
    };
    let snapshot = manager
        .service
        .submit(&method_name, Arc::new(graph), key, description)
        .map_err(HostError::raise)?;
    Ok(job_value(&manager.service, snapshot))
}

/// Один разобранный критерий отбора `ПолучитьФоновыеЗадания`.
enum JobFilter {
    Id(JobId),
    Key(String),
    States(Vec<crate::EnumValue>),
    /// «Запущенные после заданной даты». ИЗМЕРЕНО пробой
    /// `JOB.LIST.FILTER_ORDER` (сессия 2026-08-27): граница НЕстрогая —
    /// задание с `Начало`, равным границе, входит в результат.
    BeginAfter(crate::BslDate),
    /// «Завершённые до заданной даты» — строгое «до»; строгость этой
    /// границы не замерена (остаток `JOB.LIST.FILTER_ORDER`).
    EndBefore(crate::BslDate),
    Description(String),
    MethodName(String),
}

impl JobFilter {
    fn matches(&self, snapshot: &JobSnapshotDto) -> bool {
        match self {
            JobFilter::Id(id) => snapshot.id == *id,
            JobFilter::Key(text) => snapshot
                .key
                .as_ref()
                .is_some_and(|key| key_display(key).as_deref() == Some(text)),
            JobFilter::States(states) => states
                .iter()
                .any(|state| state_value(snapshot.state) == BslValue::Enum(*state)),
            JobFilter::BeginAfter(date) => snapshot.begin.is_some_and(|begin| begin >= *date),
            JobFilter::EndBefore(date) => snapshot.end.is_some_and(|end| end < *date),
            JobFilter::Description(text) => snapshot.description.as_deref().unwrap_or("") == text,
            JobFilter::MethodName(text) => crate::folded_eq(&snapshot.method_name, text),
        }
    }
}

/// Строковое представление ключа снимка: ключ хранится захваченной
/// строкой (см. коэрцию в `manager_execute`).
fn key_display(key: &JobKeyDto) -> Option<String> {
    let mut shapes = crate::RuntimeShapes::seeded(Vec::new(), Vec::new(), None);
    key.graph
        .materialize(&mut shapes)
        .ok()
        .and_then(|mut values| values.pop())
        .and_then(|value| match value {
            BslValue::Str(text) => Some(text.to_string()),
            _ => None,
        })
}

fn manager_get_jobs(
    receiver: &dyn ObjectProtocol,
    args: &[BslValue],
    ctx: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let manager = receiver_of::<BackgroundJobsManager>(receiver, "ПолучитьФоновыеЗадания")?;
    // Поля отбора — по выписке синтакс-помощника 8.3.27: Структура со
    // значениями УникальныйИдентификатор, Ключ, Состояние (перечисление
    // либо массив перечислений), Начало, Конец, Наименование, ИмяМетода,
    // РегламентноеЗадание. Порядок результата документация не фиксирует
    // (остаток `JOB.LIST.FILTER_ORDER`) — у нас детерминированный.
    let mut filters: Vec<JobFilter> = Vec::new();
    match args.first() {
        None | Some(BslValue::Undefined) => {}
        Some(BslValue::Object(object)) => {
            let crate::BslObject::Structure(storage) = object.as_ref() else {
                return Err(RtError::TypeError {
                    expected: "Структура",
                    op: "ПолучитьФоновыеЗадания",
                });
            };
            let entries: Vec<(String, BslValue)> = {
                let storage = storage.borrow();
                (0..storage.len())
                    .filter_map(|i| storage.entry_at(i))
                    .filter_map(|(field, item)| {
                        ctx.runtime_shapes()
                            .names
                            .name(field)
                            .map(|name| (name.to_string(), item))
                    })
                    .collect()
            };
            for (name, value) in entries {
                filters.push(filter_from_entry(&name, &value, ctx)?);
            }
        }
        Some(_) => {
            return Err(RtError::TypeError {
                expected: "Структура",
                op: "ПолучитьФоновыеЗадания",
            });
        }
    }
    let jobs: Vec<BslValue> = manager
        .snapshots_sorted()
        .into_iter()
        .filter(|snapshot| filters.iter().all(|filter| filter.matches(snapshot)))
        .map(|snapshot| job_value(&manager.service, snapshot))
        .collect();
    Ok(BslValue::new_array(jobs))
}

fn filter_from_entry(
    name: &str,
    value: &BslValue,
    ctx: &mut CallContext<'_>,
) -> RtResult<JobFilter> {
    const OP: &str = "ПолучитьФоновыеЗадания";
    if crate::folded_eq(name, "УникальныйИдентификатор") || crate::folded_eq(name, "UUID")
    {
        return Ok(JobFilter::Id(job_id_from_value(Some(value), OP)?));
    }
    if crate::folded_eq(name, "Ключ") || crate::folded_eq(name, "Key") {
        let text = match value {
            BslValue::Str(text) => text.to_string(),
            other => ctx.format_value(other, None)?,
        };
        return Ok(JobFilter::Key(text));
    }
    if crate::folded_eq(name, "Состояние") || crate::folded_eq(name, "State") {
        let mut states = Vec::new();
        match value {
            BslValue::Enum(state) => states.push(*state),
            BslValue::Object(object) => {
                if let crate::BslObject::Array(items) = object.as_ref() {
                    for item in items.borrow().iter() {
                        let BslValue::Enum(state) = item else {
                            return Err(RtError::TypeError {
                                expected: "СостояниеФоновогоЗадания",
                                op: OP,
                            });
                        };
                        states.push(*state);
                    }
                } else {
                    return Err(RtError::TypeError {
                        expected: "СостояниеФоновогоЗадания",
                        op: OP,
                    });
                }
            }
            _ => {
                return Err(RtError::TypeError {
                    expected: "СостояниеФоновогоЗадания",
                    op: OP,
                });
            }
        }
        return Ok(JobFilter::States(states));
    }
    if crate::folded_eq(name, "Начало") || crate::folded_eq(name, "Begin") {
        if let BslValue::Date(date) = value {
            return Ok(JobFilter::BeginAfter(*date));
        }
        return Err(RtError::TypeError {
            expected: "Дата",
            op: OP,
        });
    }
    if crate::folded_eq(name, "Конец") || crate::folded_eq(name, "End") {
        if let BslValue::Date(date) = value {
            return Ok(JobFilter::EndBefore(*date));
        }
        return Err(RtError::TypeError {
            expected: "Дата",
            op: OP,
        });
    }
    if crate::folded_eq(name, "Наименование") || crate::folded_eq(name, "Description") {
        return Ok(JobFilter::Description(value.as_str(OP)?.to_string()));
    }
    if crate::folded_eq(name, "ИмяМетода") || crate::folded_eq(name, "MethodName") {
        return Ok(JobFilter::MethodName(value.as_str(OP)?.to_string()));
    }
    if crate::folded_eq(name, "РегламентноеЗадание") || crate::folded_eq(name, "ScheduledJob")
    {
        // Регламентные задания в 0.4.0 не моделируются.
        return Err(RtError::ResourceLimit(
            "отбор по регламентному заданию не поддерживается: регламентные задания не моделируются"
                .to_string(),
        ));
    }
    Err(RtError::ResourceLimit(format!(
        "неизвестное поле отбора фоновых заданий «{name}»"
    )))
}

impl BackgroundJobsManager {
    /// Снимки в стабильном порядке — по идентификатору.
    // НЕ ИЗМЕРЕНО(JOB.LIST.FILTER_ORDER): платформенный порядок листинга
    // `ПолучитьФоновыеЗадания` и строгость границы отбора `Конец`
    // («завершённые до») не замерены; выбраны детерминированный порядок
    // по идентификатору и строгое сравнение `Конец`. Граница `Начало`
    // уже ИЗМЕРЕНА нестрогой (сессия 2026-08-27) и закреплена в
    // `JobFilter::matches`.
    fn snapshots_sorted(&self) -> Vec<Arc<JobSnapshotDto>> {
        let mut snapshots = self.service.snapshots();
        snapshots.sort_by_key(|snapshot| snapshot.id.0);
        snapshots
    }
}

fn manager_find(
    receiver: &dyn ObjectProtocol,
    args: &[BslValue],
    _ctx: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let manager =
        receiver_of::<BackgroundJobsManager>(receiver, "НайтиПоУникальномуИдентификатору")?;
    let id = job_id_from_value(args.first(), "НайтиПоУникальномуИдентификатору")?;
    Ok(match manager.service.snapshot(id) {
        Some(snapshot) => job_value(&manager.service, snapshot),
        // Неизвестный и вытесненный идентификатор — `Неопределено`;
        // точное поведение — за `JOB.FIND.MISSING`.
        None => BslValue::Undefined,
    })
}

fn manager_wait(
    receiver: &dyn ObjectProtocol,
    args: &[BslValue],
    _ctx: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let manager = receiver_of::<BackgroundJobsManager>(receiver, "ОжидатьЗавершенияВыполнения")?;
    let mut ids = Vec::new();
    match args.first() {
        None | Some(BslValue::Undefined) => {}
        Some(BslValue::Object(object)) => match object.as_ref() {
            crate::BslObject::Array(items) => {
                for item in items.borrow().iter() {
                    ids.push(job_object_id(item, "ОжидатьЗавершенияВыполнения")?);
                }
            }
            _ => ids.push(job_object_id(
                args.first().expect("ветка Some"),
                "ОжидатьЗавершенияВыполнения",
            )?),
        },
        Some(other) => {
            ids.push(job_object_id(other, "ОжидатьЗавершенияВыполнения")?);
        }
    }
    let timeout = timeout_from_arg(args.get(1))?;
    // По выписке синтакс-помощника: ожидание до первого изменения статуса
    // (с таймаутом) либо до завершения всех/первого аварийного (без), а
    // возврат — МАССИВ обновлённых заданий.
    let jobs: Vec<(JobId, JobStateDto)> = ids
        .iter()
        .map(|id| {
            let state = manager
                .service
                .snapshot(*id)
                .map(|snapshot| snapshot.state)
                .unwrap_or(JobStateDto::Completed);
            (*id, state)
        })
        .collect();
    let snapshots = manager
        .service
        .wait_first_change(&jobs, timeout)
        .map_err(HostError::raise)?;
    // Результат строится из снимков, удержанных сервисом под финальным
    // локом ожидания: вытеснение во время ожидания — ловимая JobExpired
    // выше, а не молчаливое сжатие массива.
    let updated: Vec<BslValue> = snapshots
        .into_iter()
        .map(|snapshot| job_value(&manager.service, snapshot))
        .collect();
    Ok(BslValue::new_array(updated))
}

fn job_id_from_value(value: Option<&BslValue>, op: &'static str) -> RtResult<JobId> {
    match value {
        Some(BslValue::Object(object)) => {
            if let crate::BslObject::Uuid(bytes) = object.as_ref() {
                return Ok(JobId(*bytes));
            }
            Err(RtError::TypeError {
                expected: "УникальныйИдентификатор",
                op,
            })
        }
        Some(BslValue::Str(text)) => {
            let bytes = crate::uuid::parse(&text.to_string())?;
            Ok(JobId(bytes))
        }
        _ => Err(RtError::TypeError {
            expected: "УникальныйИдентификатор",
            op,
        }),
    }
}

fn job_object_id(value: &BslValue, op: &'static str) -> RtResult<JobId> {
    if let BslValue::Object(object) = value
        && let crate::BslObject::Extension(extension) = object.as_ref()
        && let Some(job) = extension.downcast_ref::<BackgroundJobObject>()
    {
        return Ok(job.snapshot.id);
    }
    Err(RtError::TypeError {
        expected: "ФоновоеЗадание",
        op,
    })
}

// --- Методы и свойства задания -----------------------------------------

fn job_cancel(
    receiver: &dyn ObjectProtocol,
    _args: &[BslValue],
    _ctx: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let job = receiver_of::<BackgroundJobObject>(receiver, "Отменить")?;
    job.service
        .cancel(job.snapshot.id)
        .map_err(HostError::raise)?;
    Ok(BslValue::Undefined)
}

fn job_wait(
    receiver: &dyn ObjectProtocol,
    args: &[BslValue],
    _ctx: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let job = receiver_of::<BackgroundJobObject>(receiver, "ОжидатьЗавершенияВыполнения")?;
    let timeout = timeout_from_arg(args.first())?;
    let outcome = job
        .service
        .wait_terminal(&[job.snapshot.id], timeout)
        .map_err(HostError::raise)?;
    // ИЗМЕРЕНО (JOB.WAIT.RETURN): метод — функция и возвращает ФоновоеЗадание.
    // Снимок удержан сервисом под финальным локом ожидания; вытеснение из
    // истории ВО ВРЕМЯ ожидания — ловимая JobExpired выше, устаревший
    // снимок «Активно» за свежий не выдаётся.
    let snapshot = outcome
        .snapshots
        .into_iter()
        .next()
        .unwrap_or_else(|| Arc::clone(&job.snapshot));
    Ok(BslValue::new_object(BackgroundJobObject {
        snapshot,
        service: std::rc::Rc::clone(&job.service),
    }))
}

fn job_messages(
    receiver: &dyn ObjectProtocol,
    args: &[BslValue],
    ctx: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let job = receiver_of::<BackgroundJobObject>(receiver, "ПолучитьСообщенияПользователю")?;
    let remove = matches!(args.first(), Some(BslValue::Boolean(true)));
    let messages = job
        .service
        .take_messages(job.snapshot.id, remove)
        .map_err(HostError::raise)?;
    let mut values = Vec::with_capacity(messages.len());
    for message in messages {
        values.push(BslValue::new_object(
            crate::user_message::UserMessageObject::from_dto(&message, ctx)?,
        ));
    }
    Ok(BslValue::new_array(values))
}

fn job_uuid(receiver: &dyn ObjectProtocol, _ctx: &mut CallContext<'_>) -> RtResult<BslValue> {
    let job = receiver_of::<BackgroundJobObject>(receiver, "УникальныйИдентификатор")?;
    Ok(BslValue::Object(Rc::new(crate::BslObject::Uuid(
        job.snapshot.id.0,
    ))))
}

fn job_method_name(
    receiver: &dyn ObjectProtocol,
    _ctx: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let job = receiver_of::<BackgroundJobObject>(receiver, "ИмяМетода")?;
    Ok(BslValue::Str(crate::BslString::from_str(
        &job.snapshot.method_name,
    )))
}

fn job_key(receiver: &dyn ObjectProtocol, ctx: &mut CallContext<'_>) -> RtResult<BslValue> {
    let job = receiver_of::<BackgroundJobObject>(receiver, "Ключ")?;
    // ИЗМЕРЕНО (JOB.API.KEY_EMPTY, JOB.KEY.NON_STRING): свойство всегда
    // строковое — пустая строка без ключа, представление значения с ним.
    match &job.snapshot.key {
        None => Ok(BslValue::Str(crate::BslString::from_str(""))),
        Some(key) => {
            let mut values = key.graph.materialize(ctx.runtime_shapes())?;
            let value = values.pop().unwrap_or(BslValue::Undefined);
            let text = match &value {
                BslValue::Str(text) => text.to_string(),
                other => ctx.format_value(other, None)?,
            };
            Ok(BslValue::Str(crate::BslString::from_str(&text)))
        }
    }
}

fn job_description(
    receiver: &dyn ObjectProtocol,
    _ctx: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let job = receiver_of::<BackgroundJobObject>(receiver, "Наименование")?;
    Ok(match &job.snapshot.description {
        Some(text) => BslValue::Str(crate::BslString::from_str(text)),
        None => BslValue::Str(crate::BslString::from_str("")),
    })
}

fn job_state(receiver: &dyn ObjectProtocol, _ctx: &mut CallContext<'_>) -> RtResult<BslValue> {
    let job = receiver_of::<BackgroundJobObject>(receiver, "Состояние")?;
    Ok(state_value(job.snapshot.state))
}

fn job_begin(receiver: &dyn ObjectProtocol, _ctx: &mut CallContext<'_>) -> RtResult<BslValue> {
    let job = receiver_of::<BackgroundJobObject>(receiver, "Начало")?;
    Ok(match job.snapshot.begin {
        Some(date) => BslValue::Date(date),
        None => BslValue::Undefined,
    })
}

fn job_end(receiver: &dyn ObjectProtocol, _ctx: &mut CallContext<'_>) -> RtResult<BslValue> {
    let job = receiver_of::<BackgroundJobObject>(receiver, "Конец")?;
    Ok(match job.snapshot.end {
        Some(date) => BslValue::Date(date),
        None => BslValue::Undefined,
    })
}

fn job_error_info(receiver: &dyn ObjectProtocol, _ctx: &mut CallContext<'_>) -> RtResult<BslValue> {
    let job = receiver_of::<BackgroundJobObject>(receiver, "ИнформацияОбОшибке")?;
    Ok(match &job.snapshot.error {
        // Новая `ИнформацияОбОшибке` строится в сеансе читающего:
        // идентичность исходного объекта не сохраняется, cause-цепочка
        // разворачивается рекурсивно.
        Some(error) => error_info_from_dto(error),
        None => BslValue::Undefined,
    })
}

fn error_info_from_dto(error: &crate::JobErrorDto) -> BslValue {
    let cause = match &error.cause {
        Some(cause) => error_info_from_dto(cause),
        None => BslValue::Undefined,
    };
    crate::error_info::new_error_info_detailed(
        &error.brief,
        &error.full,
        error.module.as_deref().unwrap_or(""),
        error.line,
        cause,
    )
}

static MANAGER_METHODS: &[MethodDescriptor] = &[
    MethodDescriptor::new(
        &["Выполнить", "Execute"],
        Arity::range(1, 4),
        manager_execute,
    ),
    MethodDescriptor::new(
        &["ПолучитьФоновыеЗадания", "GetBackgroundJobs"],
        Arity::range(0, 1),
        manager_get_jobs,
    ),
    MethodDescriptor::new(
        &["НайтиПоУникальномуИдентификатору", "FindByUUID"],
        Arity::exact(1),
        manager_find,
    ),
    MethodDescriptor::new(
        &[
            "ОжидатьЗавершенияВыполнения",
            "WaitForExecutionCompletion",
            "ОжидатьЗавершения",
            "WaitForCompletion",
        ],
        Arity::range(1, 2),
        manager_wait,
    ),
];

static JOB_METHODS: &[MethodDescriptor] = &[
    MethodDescriptor::new(&["Отменить", "Cancel"], Arity::exact(0), job_cancel),
    // «ОжидатьЗавершения» — документированный устаревший синоним
    // (синтакс-помощник 8.3.27 перечисляет оба имени у задания).
    MethodDescriptor::new(
        &[
            "ОжидатьЗавершенияВыполнения",
            "WaitForExecutionCompletion",
            "ОжидатьЗавершения",
            "WaitForCompletion",
        ],
        Arity::range(0, 1),
        job_wait,
    ),
    MethodDescriptor::new(
        &["ПолучитьСообщенияПользователю", "GetUserMessages"],
        Arity::range(0, 1),
        job_messages,
    ),
];

static JOB_PROPERTIES: &[PropertyDescriptor] = &[
    PropertyDescriptor {
        names: &["УникальныйИдентификатор", "UUID"],
        get: job_uuid,
        set: None,
    },
    PropertyDescriptor {
        names: &["ИмяМетода", "MethodName"],
        get: job_method_name,
        set: None,
    },
    // ИЗМЕРЕНО на 8.3.27 (JOB.PARAMS.SNAPSHOT, вторая строка сырого
    // вывода): свойства «Параметры» у ФоновоеЗадание НЕТ — «Object field
    // not found». Документация 1c-dn его упоминает; замер главнее.
    // Ленивая материализация снимка остаётся внутренним Rust-механизмом.
    PropertyDescriptor {
        names: &["Ключ", "Key"],
        get: job_key,
        set: None,
    },
    PropertyDescriptor {
        names: &["Наименование", "Description"],
        get: job_description,
        set: None,
    },
    PropertyDescriptor {
        names: &["Состояние", "State"],
        get: job_state,
        set: None,
    },
    PropertyDescriptor {
        names: &["Начало", "Begin"],
        get: job_begin,
        set: None,
    },
    PropertyDescriptor {
        names: &["Конец", "End"],
        get: job_end,
        set: None,
    },
    PropertyDescriptor {
        names: &["ИнформацияОбОшибке", "ErrorInfo"],
        get: job_error_info,
        set: None,
    },
];

pub(crate) const API_MEMBERS: &[ObjectMembersDescriptor] = &[
    ObjectMembersDescriptor::new(&BACKGROUND_JOBS_TYPE).with_methods(MANAGER_METHODS),
    ObjectMembersDescriptor::new(&BACKGROUND_JOB_TYPE)
        .with_properties(JOB_PROPERTIES)
        .with_methods(JOB_METHODS),
];

/// Конструктор менеджера для голого имени `ФоновыеЗадания`. Без
/// внедрённого сервиса — ловимая ошибка возможности host.
pub(crate) fn construct_manager(
    ctx: &mut CallContext<'_>,
    _args: &[BslValue],
) -> RtResult<BslValue> {
    let service = ctx.background_jobs()?;
    Ok(BslValue::new_object(BackgroundJobsManager {
        service: Rc::clone(service),
    }))
}
