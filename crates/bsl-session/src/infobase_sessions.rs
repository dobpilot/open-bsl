//! Соединения и сеансы информационной базы, текущее фоновое задание.

use super::*;

/// Снимок одного активного соединения информационной базы.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InfoBaseConnection {
    pub(crate) number: u64,
    application_name: Arc<str>,
    computer_name: Arc<str>,
}

impl InfoBaseConnection {
    /// Создаёт описание соединения из значений, полученных хостом.
    #[must_use]
    pub fn new(
        number: u64,
        application_name: impl Into<String>,
        computer_name: impl Into<String>,
    ) -> Self {
        Self {
            number,
            application_name: Arc::from(application_name.into()),
            computer_name: Arc::from(computer_name.into()),
        }
    }

    /// Возвращает номер соединения.
    #[must_use]
    pub const fn number(&self) -> u64 {
        self.number
    }

    /// Возвращает имя клиентского приложения.
    #[must_use]
    pub fn application_name(&self) -> &str {
        &self.application_name
    }

    /// Возвращает имя компьютера.
    #[must_use]
    pub fn computer_name(&self) -> &str {
        &self.computer_name
    }
}

#[derive(Debug)]
struct InfoBaseSessionObject {
    background_method: Option<Arc<str>>,
    session_number: Option<u64>,
    session_started: Option<BslDate>,
}

#[derive(Debug)]
struct CurrentBackgroundJobObject {
    method_name: Arc<str>,
}

#[derive(Debug)]
struct InfoBaseConnectionObject {
    connection: InfoBaseConnection,
}

pub(crate) static INFOBASE_SESSION_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-session",
    name: "СеансИнформационнойБазы",
    type_display: "Active session",
    type_names: &["InfoBaseSession"],
};

pub(crate) static INFOBASE_CONNECTION_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-session",
    name: "СоединениеИнформационнойБазы",
    type_display: "Active connection",
    type_names: &["InfoBaseConnection"],
};

impl ObjectProtocol for InfoBaseSessionObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &INFOBASE_SESSION_TYPE
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        INFOBASE_SESSION_METHODS
    }

    fn property_table(&self) -> &'static [PropertyDescriptor] {
        INFOBASE_SESSION_PROPERTIES
    }
}

impl ObjectProtocol for CurrentBackgroundJobObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        background_job_type_descriptor()
    }

    fn property_table(&self) -> &'static [PropertyDescriptor] {
        CURRENT_BACKGROUND_JOB_PROPERTIES
    }
}

impl ObjectProtocol for InfoBaseConnectionObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &INFOBASE_CONNECTION_TYPE
    }

    fn property_table(&self) -> &'static [PropertyDescriptor] {
        INFOBASE_CONNECTION_PROPERTIES
    }
}

pub(crate) fn infobase_sessions(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    // Список сеансов требует права `АктивныеПользователи` и живого снимка
    // сервера; текущий сеанс не заменяет такой список.
    Err(RtError::DynamicError(
        "список сеансов информационной базы не предоставлен хостом".into(),
    ))
}

pub(crate) fn infobase_connection_string(
    ctx: &mut CallContext<'_>,
    _: &[Value],
) -> RtResult<Value> {
    let value = runtime(ctx)?.connection_string().ok_or_else(|| {
        RtError::DynamicError("строка соединения с информационной базой не передана хостом".into())
    })?;
    Ok(Value::Str(value.into()))
}

pub(crate) fn infobase_url(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    // Строка SQL-соединения не содержит клиентский адрес запуска базы.
    // Подмена её навигационной ссылкой дала бы вызывающему ложный URL.
    Err(RtError::DynamicError(
        "навигационная ссылка информационной базы не предоставлена хостом".into(),
    ))
}

pub(crate) fn infobase_connection_number(
    ctx: &mut CallContext<'_>,
    _: &[Value],
) -> RtResult<Value> {
    Ok(Value::number_from_i64(
        runtime(ctx)?.connection_number() as i64
    ))
}

pub(crate) fn infobase_session_number(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    let number = runtime(ctx)?.session_number().ok_or_else(|| {
        RtError::DynamicError("номер сеанса информационной базы не передан хостом".into())
    })?;
    Ok(Value::number_from_i64(number as i64))
}

pub(crate) fn infobase_connections(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    let connections = runtime(ctx)?.connections().ok_or_else(|| {
        RtError::DynamicError(
            "снимок соединений информационной базы недоступен текущему пользователю".into(),
        )
    })?;
    Ok(Value::new_array(
        connections
            .iter()
            .cloned()
            .map(|connection| Value::new_object(InfoBaseConnectionObject { connection }))
            .collect(),
    ))
}

fn infobase_connection_number_property(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let connection = receiver
        .downcast_ref::<InfoBaseConnectionObject>()
        .ok_or(RtError::NotAnObject)?;
    Ok(Value::number_from_i64(connection.connection.number as i64))
}

fn infobase_connection_application_name(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let connection = receiver
        .downcast_ref::<InfoBaseConnectionObject>()
        .ok_or(RtError::NotAnObject)?;
    Ok(Value::Str(
        connection.connection.application_name.as_ref().into(),
    ))
}

fn infobase_connection_computer_name(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let connection = receiver
        .downcast_ref::<InfoBaseConnectionObject>()
        .ok_or(RtError::NotAnObject)?;
    Ok(Value::Str(
        connection.connection.computer_name.as_ref().into(),
    ))
}

pub(crate) fn current_infobase_session(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    Ok(Value::new_object(InfoBaseSessionObject {
        background_method: runtime(ctx)?.background_method.clone(),
        session_number: runtime(ctx)?.session_number(),
        session_started: runtime(ctx)?.session_started,
    }))
}

fn current_infobase_session_number(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let session = receiver
        .downcast_ref::<InfoBaseSessionObject>()
        .ok_or(RtError::NotAnObject)?;
    let number = session.session_number.ok_or_else(|| {
        RtError::DynamicError("номер сеанса информационной базы не передан хостом".into())
    })?;
    Ok(Value::number_from_i64(number as i64))
}

fn current_infobase_session_started(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let session = receiver
        .downcast_ref::<InfoBaseSessionObject>()
        .ok_or(RtError::NotAnObject)?;
    let date = session.session_started.ok_or_else(|| {
        RtError::DynamicError("начало сеанса информационной базы не передано хостом".into())
    })?;
    Ok(Value::Date(date))
}

fn current_background_job(
    receiver: &dyn ObjectProtocol,
    _: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let session = receiver
        .downcast_ref::<InfoBaseSessionObject>()
        .ok_or(RtError::NotAnObject)?;
    Ok(session
        .background_method
        .as_ref()
        .map_or(Value::Undefined, |name| {
            Value::new_object(CurrentBackgroundJobObject {
                method_name: Arc::clone(name),
            })
        }))
}

fn current_background_job_method_name(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let job = receiver
        .downcast_ref::<CurrentBackgroundJobObject>()
        .ok_or(RtError::NotAnObject)?;
    Ok(Value::Str(job.method_name.as_ref().into()))
}

pub(crate) static INFOBASE_SESSION_METHODS: &[MethodDescriptor] = &[MethodDescriptor::new(
    &["ПолучитьФоновоеЗадание", "GetBackgroundJob"],
    Arity::exact(0),
    current_background_job,
)];

pub(crate) static INFOBASE_SESSION_PROPERTIES: &[PropertyDescriptor] = &[
    PropertyDescriptor {
        names: &["НомерСеанса", "SessionNumber"],
        get: current_infobase_session_number,
        set: None,
    },
    PropertyDescriptor {
        names: &["НачалоСеанса", "SessionStarted"],
        get: current_infobase_session_started,
        set: None,
    },
];

static CURRENT_BACKGROUND_JOB_PROPERTIES: &[PropertyDescriptor] = &[PropertyDescriptor {
    names: &["ИмяМетода", "MethodName"],
    get: current_background_job_method_name,
    set: None,
}];

pub(crate) static INFOBASE_CONNECTION_PROPERTIES: &[PropertyDescriptor] = &[
    PropertyDescriptor {
        names: &["НомерСоединения", "ConnectionNumber"],
        get: infobase_connection_number_property,
        set: None,
    },
    PropertyDescriptor {
        names: &["ИмяПриложения", "ApplicationName"],
        get: infobase_connection_application_name,
        set: None,
    },
    PropertyDescriptor {
        names: &["ИмяКомпьютера", "ComputerName"],
        get: infobase_connection_computer_name,
        set: None,
    },
];

pub(crate) fn get_sessions_lock(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    // Текущий режим блокировки хранится в базе, а не выводится из параметров
    // отдельного сеанса. Без администрационного адаптера он неизвестен.
    Err(RtError::DynamicError(
        "чтение блокировки сеансов не предоставлено этим хостом".into(),
    ))
}

pub(crate) fn set_sessions_lock(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    // Принять настройку без записи в базу означало бы сообщить ложный успех.
    Err(RtError::DynamicError(
        "запись блокировки сеансов не предоставлена этим хостом".into(),
    ))
}
