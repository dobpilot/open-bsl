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
    Arity, BslValue, CallContext, EnumValue, GraphLimits, JobId, JobKeyDto, JobSnapshotDto,
    JobStateDto, MethodDescriptor, ObjectProtocol, PropertyDescriptor, RtError, RtResult,
    SerializedValueGraph, TypeDescriptor, receiver_of,
};

/// Объектобезопасный сервис host: все операции на владеющих DTO. Ошибки —
/// ловимые строки; закрытый `HostErrorCode` появится вместе с полным
/// набором host-ошибок.
pub trait BackgroundJobService {
    /// Запуск цели `Модуль.Метод`; снимок `Queued` при успехе.
    fn submit(
        &self,
        method_name: &str,
        params: Arc<SerializedValueGraph>,
        key: Option<Arc<JobKeyDto>>,
        description: Option<String>,
    ) -> Result<Arc<JobSnapshotDto>, String>;
    /// Свежий снимок по идентификатору; `None` — неизвестный/вытесненный.
    fn snapshot(&self, id: JobId) -> Option<Arc<JobSnapshotDto>>;
    /// Все снимки: живые и история.
    fn snapshots(&self) -> Vec<Arc<JobSnapshotDto>>;
    /// Ожидание terminal-состояния всех `ids`; `false` — таймаут.
    fn wait_terminal(&self, ids: &[JobId], timeout: Option<Duration>) -> bool;
    /// Отмена задания — приходит вместе с кооперативными safe points
    /// (этап 6 плана).
    fn cancel(&self, id: JobId) -> Result<(), String>;
    /// Сообщения задания в порядке FIFO; `remove` атомарно забирает
    /// возвращаемый префикс у живой записи (у terminal-снимка история
    /// неизменяема — точная модель за `JOB.MESSAGES`).
    fn take_messages(&self, id: JobId, remove: bool) -> Vec<String>;
}

pub(crate) static USER_MESSAGE_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "СообщениеПользователю",
    type_display: "User message",
    type_names: &["UserMessage"],
};

/// Сообщение пользователю в истории задания. Минимальная модель — текст;
/// поля назначения и ключа данных — за замером `JOB.MESSAGES`.
struct UserMessageObject {
    text: String,
}

impl std::fmt::Debug for UserMessageObject {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "СообщениеПользователю {:?}", self.text)
    }
}

impl ObjectProtocol for UserMessageObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &USER_MESSAGE_TYPE
    }

    fn property_table(&self) -> &'static [PropertyDescriptor] {
        USER_MESSAGE_PROPERTIES
    }

    fn get_property(&self, name: &str, ctx: &mut CallContext<'_>) -> RtResult<BslValue> {
        crate::get_property_from_table(
            USER_MESSAGE_PROPERTIES,
            "СообщениеПользователю",
            self,
            name,
            ctx,
        )
    }
}

fn user_message_text(
    receiver: &dyn ObjectProtocol,
    _ctx: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let message = receiver_of::<UserMessageObject>(receiver, "Текст")?;
    Ok(BslValue::Str(crate::BslString::from_str(&message.text)))
}

static USER_MESSAGE_PROPERTIES: &[PropertyDescriptor] = &[PropertyDescriptor {
    names: &["Текст", "Text"],
    get: user_message_text,
    set: None,
}];

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
    if seconds < 0 {
        return Err(RtError::TypeError {
            expected: "неотрицательный таймаут",
            op: "ОжидатьЗавершенияВыполнения",
        });
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
    let graph =
        SerializedValueGraph::capture(&params, ctx.runtime_shapes(), &GraphLimits::default())?;
    let key = match args.get(2) {
        None | Some(BslValue::Undefined) => None,
        Some(value) => Some(Arc::new(JobKeyDto {
            graph: SerializedValueGraph::capture(
                std::slice::from_ref(value),
                ctx.runtime_shapes(),
                &GraphLimits::default(),
            )?,
        })),
    };
    let description = match args.get(3) {
        None | Some(BslValue::Undefined) => None,
        Some(value) => Some(value.as_str("Выполнить")?.to_string()),
    };
    let snapshot = manager
        .service
        .submit(&method_name, Arc::new(graph), key, description)
        .map_err(RtError::ResourceLimit)?;
    Ok(job_value(&manager.service, snapshot))
}

fn manager_get_jobs(
    receiver: &dyn ObjectProtocol,
    args: &[BslValue],
    _ctx: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let manager = receiver_of::<BackgroundJobsManager>(receiver, "ПолучитьФоновыеЗадания")?;
    if args
        .first()
        .is_some_and(|v| !matches!(v, BslValue::Undefined))
    {
        // Поля и семантика отбора — за `JOB.LIST.FILTER_ORDER`.
        return Err(RtError::ResourceLimit(
            "отбор фоновых заданий появится после замера JOB.LIST.FILTER_ORDER".to_string(),
        ));
    }
    let jobs: Vec<BslValue> = manager
        .snapshots_sorted()
        .into_iter()
        .map(|snapshot| job_value(&manager.service, snapshot))
        .collect();
    Ok(BslValue::new_array(jobs))
}

impl BackgroundJobsManager {
    /// Снимки в стабильном порядке — по идентификатору; платформенный
    /// порядок листинга уточняется `JOB.LIST.FILTER_ORDER`.
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
    // Правило any/all для массива — за `JOB.WAIT.MANY`; пока — все.
    manager.service.wait_terminal(&ids, timeout);
    Ok(BslValue::Undefined)
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
        .map_err(RtError::ResourceLimit)?;
    Ok(BslValue::Undefined)
}

fn job_wait(
    receiver: &dyn ObjectProtocol,
    args: &[BslValue],
    _ctx: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let job = receiver_of::<BackgroundJobObject>(receiver, "ОжидатьЗавершенияВыполнения")?;
    let timeout = timeout_from_arg(args.first())?;
    job.service.wait_terminal(&[job.snapshot.id], timeout);
    // ИЗМЕРЕНО (JOB.WAIT.RETURN): метод — функция и возвращает ФоновоеЗадание;
    // отдаём свежий снимок, а при вытеснении из истории — прежний.
    let snapshot = job
        .service
        .snapshot(job.snapshot.id)
        .unwrap_or_else(|| Arc::clone(&job.snapshot));
    Ok(BslValue::new_object(BackgroundJobObject {
        snapshot,
        service: std::rc::Rc::clone(&job.service),
    }))
}

fn job_messages(
    receiver: &dyn ObjectProtocol,
    args: &[BslValue],
    _ctx: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let job = receiver_of::<BackgroundJobObject>(receiver, "ПолучитьСообщенияПользователю")?;
    let remove = matches!(args.first(), Some(BslValue::Boolean(true)));
    let messages = job.service.take_messages(job.snapshot.id, remove);
    Ok(BslValue::new_array(
        messages
            .into_iter()
            .map(|text| BslValue::new_object(UserMessageObject { text }))
            .collect(),
    ))
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
        // идентичность исходного объекта не сохраняется.
        Some(error) => crate::error_info::new_error_info(crate::BslString::from_str(&error.full)),
        None => BslValue::Undefined,
    })
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
        &["ОжидатьЗавершенияВыполнения", "WaitForExecutionCompletion"],
        Arity::range(1, 2),
        manager_wait,
    ),
];

static JOB_METHODS: &[MethodDescriptor] = &[
    MethodDescriptor::new(&["Отменить", "Cancel"], Arity::exact(0), job_cancel),
    MethodDescriptor::new(
        &["ОжидатьЗавершенияВыполнения", "WaitForExecutionCompletion"],
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
