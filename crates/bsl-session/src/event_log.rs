//! Журнал регистрации: уровни, записи, адаптер записи и настройки
//! использования событий.

use super::*;

/// Уровень события журнала регистрации.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventLogLevel {
    /// Примечание.
    Note,
    /// Информационное событие.
    Information,
    /// Предупреждение.
    Warning,
    /// Ошибка.
    Error,
}

impl EventLogLevel {
    fn value(self) -> Value {
        Value::Enum(match self {
            Self::Note => EnumValue::EventLogLevelNote,
            Self::Information => EnumValue::EventLogLevelInformation,
            Self::Warning => EnumValue::EventLogLevelWarning,
            Self::Error => EnumValue::EventLogLevelError,
        })
    }
}

/// Связь записи журнала регистрации с транзакцией.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventLogTransactionMode {
    /// Запись выполняется независимо от текущей транзакции.
    Independent,
    /// Запись следует результату текущей транзакции.
    Transactional,
}

/// Неизменяемая запись, передаваемая журналу текущего сеанса.
#[derive(Debug, Clone)]
pub struct EventLogEntry {
    event_name: Arc<str>,
    level: EventLogLevel,
    metadata: Value,
    data: Value,
    comment: Arc<str>,
    transaction_mode: EventLogTransactionMode,
}

impl EventLogEntry {
    /// Создаёт запись из подготовленных хостом значений.
    ///
    /// Конструктор сохраняет значения без преобразований и не обращается
    /// к журналу. Для доставки адаптер передаёт запись в `EventLogWriter`
    /// текущего сеанса.
    #[must_use]
    pub fn new(
        event_name: impl Into<Arc<str>>,
        level: EventLogLevel,
        metadata: Value,
        data: Value,
        comment: impl Into<Arc<str>>,
        transaction_mode: EventLogTransactionMode,
    ) -> Self {
        Self {
            event_name: event_name.into(),
            level,
            metadata,
            data,
            comment: comment.into(),
            transaction_mode,
        }
    }

    /// Возвращает имя события.
    #[must_use]
    pub fn event_name(&self) -> &str {
        &self.event_name
    }

    /// Возвращает уровень события.
    #[must_use]
    pub const fn level(&self) -> EventLogLevel {
        self.level
    }

    /// Возвращает связанный объект метаданных либо `Неопределено`.
    #[must_use]
    pub const fn metadata(&self) -> &Value {
        &self.metadata
    }

    /// Возвращает связанные данные либо `Неопределено`.
    #[must_use]
    pub const fn data(&self) -> &Value {
        &self.data
    }

    /// Возвращает комментарий.
    #[must_use]
    pub fn comment(&self) -> &str {
        &self.comment
    }

    /// Возвращает режим транзакции.
    #[must_use]
    pub const fn transaction_mode(&self) -> EventLogTransactionMode {
        self.transaction_mode
    }
}

/// Снимок настройки использования одного события журнала регистрации.
#[derive(Debug, Clone)]
pub struct EventLogEventUseData {
    use_event: bool,
    description: Value,
}

impl EventLogEventUseData {
    /// Создаёт снимок настройки события.
    #[must_use]
    pub const fn new(use_event: bool, description: Value) -> Self {
        Self {
            use_event,
            description,
        }
    }

    /// Возвращает признак регистрации события.
    #[must_use]
    pub const fn use_event(&self) -> bool {
        self.use_event
    }

    /// Возвращает произвольное описание использования события.
    #[must_use]
    pub const fn description(&self) -> &Value {
        &self.description
    }
}

/// Приёмник событий журнала регистрации одного сеанса.
pub trait EventLogWriter: fmt::Debug {
    /// Записывает одно событие.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку хоста, проверки прав или хранения журнала.
    fn write(&self, entry: EventLogEntry) -> RtResult<()>;

    /// Читает настройку использования именованного события.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку неизвестного события, проверки прав, хранилища или
    /// неподдерживаемой операции.
    fn event_use(&self, _event_name: &str) -> RtResult<EventLogEventUseData> {
        Err(RtError::DynamicError(
            "чтение настроек событий журнала регистрации не поддерживается адаптером".into(),
        ))
    }

    /// Записывает настройку использования именованного события.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку неизвестного события, проверки прав, хранилища или
    /// неподдерживаемой операции.
    fn set_event_use(&self, _event_name: &str, _use: &EventLogEventUseData) -> RtResult<()> {
        Err(RtError::DynamicError(
            "изменение настроек событий журнала регистрации не поддерживается адаптером".into(),
        ))
    }

    /// Возвращает включённые уровни журнала регистрации текущей базы.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку проверки прав, хранилища или неподдерживаемой
    /// операции.
    fn enabled_levels(&self) -> RtResult<Vec<EventLogLevel>> {
        Err(RtError::DynamicError(
            "чтение включённых уровней журнала регистрации не поддерживается адаптером".into(),
        ))
    }
}

#[derive(Debug)]
struct EventLogEventUseObject {
    use_event: Cell<bool>,
    description: RefCell<Value>,
}

#[derive(Debug)]
struct EventLogAccessEventUseDescriptionObject {
    object: RefCell<Arc<str>>,
    access_fields: RefCell<Value>,
    logged_fields: RefCell<Value>,
}

pub(crate) static EVENT_LOG_EVENT_USE_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-session",
    name: "ИспользованиеСобытияЖурналаРегистрации",
    type_display: "Использование события журнала регистрации",
    type_names: &["EventLogEventUse"],
};

pub(crate) static EVENT_LOG_ACCESS_EVENT_USE_DESCRIPTION_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-session",
    name: "ОписаниеИспользованияСобытияДоступЖурналаРегистрации",
    type_display: "Описание использования события доступ журнала регистрации",
    type_names: &["EventLogAccessEventUseDescription"],
};

impl ObjectProtocol for EventLogEventUseObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &EVENT_LOG_EVENT_USE_TYPE
    }

    fn property_table(&self) -> &'static [PropertyDescriptor] {
        EVENT_LOG_EVENT_USE_PROPERTIES
    }
}

impl ObjectProtocol for EventLogAccessEventUseDescriptionObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &EVENT_LOG_ACCESS_EVENT_USE_DESCRIPTION_TYPE
    }

    fn property_table(&self) -> &'static [PropertyDescriptor] {
        EVENT_LOG_ACCESS_EVENT_USE_DESCRIPTION_PROPERTIES
    }
}

fn event_log_level(value: Option<&Value>) -> RtResult<EventLogLevel> {
    match value.unwrap_or(&Value::Undefined) {
        Value::Undefined => Ok(EventLogLevel::Note),
        Value::Enum(EnumValue::EventLogLevelNote) => Ok(EventLogLevel::Note),
        Value::Enum(EnumValue::EventLogLevelInformation) => Ok(EventLogLevel::Information),
        Value::Enum(EnumValue::EventLogLevelWarning) => Ok(EventLogLevel::Warning),
        Value::Enum(EnumValue::EventLogLevelError) => Ok(EventLogLevel::Error),
        _ => Err(RtError::TypeError {
            expected: "УровеньЖурналаРегистрации",
            op: "ЗаписьЖурналаРегистрации",
        }),
    }
}

fn event_log_transaction_mode(value: Option<&Value>) -> RtResult<EventLogTransactionMode> {
    match value.unwrap_or(&Value::Undefined) {
        Value::Undefined => Ok(EventLogTransactionMode::Independent),
        Value::Enum(EnumValue::EventLogEntryTransactionModeIndependent) => {
            Ok(EventLogTransactionMode::Independent)
        }
        Value::Enum(EnumValue::EventLogEntryTransactionModeTransactional) => {
            Ok(EventLogTransactionMode::Transactional)
        }
        _ => Err(RtError::TypeError {
            expected: "РежимТранзакцииЗаписиЖурналаРегистрации",
            op: "ЗаписьЖурналаРегистрации",
        }),
    }
}

pub(crate) fn write_log_event(ctx: &mut CallContext<'_>, arguments: &[Value]) -> RtResult<Value> {
    let event_name = match &arguments[0] {
        Value::Str(value) => Arc::from(value.to_string()),
        _ => {
            return Err(RtError::TypeError {
                expected: "Строка",
                op: "ЗаписьЖурналаРегистрации",
            });
        }
    };
    let comment = match arguments.get(4).unwrap_or(&Value::Undefined) {
        Value::Undefined => Arc::from(""),
        Value::Str(value) => Arc::from(value.to_string()),
        _ => {
            return Err(RtError::TypeError {
                expected: "Строка",
                op: "ЗаписьЖурналаРегистрации",
            });
        }
    };
    let entry = EventLogEntry::new(
        event_name,
        event_log_level(arguments.get(1))?,
        arguments.get(2).cloned().unwrap_or(Value::Undefined),
        arguments.get(3).cloned().unwrap_or(Value::Undefined),
        comment,
        event_log_transaction_mode(arguments.get(5))?,
    );
    let writer = Rc::clone(runtime(ctx)?.event_log_writer().ok_or_else(|| {
        RtError::DynamicError("журнал регистрации не подключён для текущего сеанса".into())
    })?);
    writer.write(entry)?;
    Ok(Value::Undefined)
}

fn event_log_event_name(value: &Value, operation: &'static str) -> RtResult<String> {
    let Value::Str(name) = value else {
        return Err(RtError::TypeError {
            expected: "Строка",
            op: operation,
        });
    };
    Ok(name.to_string())
}

pub(crate) fn get_event_log_event_use(
    ctx: &mut CallContext<'_>,
    arguments: &[Value],
) -> RtResult<Value> {
    let name = event_log_event_name(
        &arguments[0],
        "ПолучитьИспользованиеСобытияЖурналаРегистрации",
    )?;
    let writer = Rc::clone(runtime(ctx)?.event_log_writer().ok_or_else(|| {
        RtError::DynamicError("журнал регистрации не подключён для текущего сеанса".into())
    })?);
    let setting = writer.event_use(&name)?;
    Ok(Value::new_object(EventLogEventUseObject {
        use_event: Cell::new(setting.use_event()),
        description: RefCell::new(setting.description().clone()),
    }))
}

pub(crate) fn set_event_log_event_use(
    ctx: &mut CallContext<'_>,
    arguments: &[Value],
) -> RtResult<Value> {
    let name = event_log_event_name(
        &arguments[0],
        "УстановитьИспользованиеСобытияЖурналаРегистрации",
    )?;
    let object = arguments[1]
        .object_ref()
        .and_then(|object| object.downcast_ref::<EventLogEventUseObject>())
        .ok_or(RtError::TypeError {
            expected: "ИспользованиеСобытияЖурналаРегистрации",
            op: "УстановитьИспользованиеСобытияЖурналаРегистрации",
        })?;
    let setting =
        EventLogEventUseData::new(object.use_event.get(), object.description.borrow().clone());
    let writer = Rc::clone(runtime(ctx)?.event_log_writer().ok_or_else(|| {
        RtError::DynamicError("журнал регистрации не подключён для текущего сеанса".into())
    })?);
    writer.set_event_use(&name, &setting)?;
    Ok(Value::Undefined)
}

pub(crate) fn get_event_log_using(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    let writer = Rc::clone(runtime(ctx)?.event_log_writer().ok_or_else(|| {
        RtError::DynamicError("журнал регистрации не подключён для текущего сеанса".into())
    })?);
    Ok(Value::new_array(
        writer
            .enabled_levels()?
            .into_iter()
            .map(EventLogLevel::value)
            .collect(),
    ))
}

pub(crate) fn unload_event_log(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    // Запись отдельных событий не даёт доступа к чтению и выгрузке журнала.
    Err(RtError::DynamicError(
        "выгрузка журнала регистрации не поддерживается этим хостом".into(),
    ))
}

pub(crate) fn construct_event_log_event_use(
    _: &mut CallContext<'_>,
    arguments: &[Value],
) -> RtResult<Value> {
    let use_event = match arguments.first() {
        None => true,
        Some(Value::Boolean(value)) => *value,
        Some(_) => {
            return Err(RtError::TypeError {
                expected: "Булево",
                op: "Новый ИспользованиеСобытияЖурналаРегистрации",
            });
        }
    };
    let description = arguments.get(1).cloned().unwrap_or(Value::Undefined);
    Ok(Value::new_object(EventLogEventUseObject {
        use_event: Cell::new(use_event),
        description: RefCell::new(description),
    }))
}

fn event_log_event_use(receiver: &dyn ObjectProtocol) -> RtResult<&EventLogEventUseObject> {
    receiver
        .downcast_ref::<EventLogEventUseObject>()
        .ok_or(RtError::NotAnObject)
}

fn event_log_use(receiver: &dyn ObjectProtocol, _: &mut CallContext<'_>) -> RtResult<Value> {
    Ok(Value::Boolean(
        event_log_event_use(receiver)?.use_event.get(),
    ))
}

fn set_event_log_use(
    receiver: &dyn ObjectProtocol,
    value: Value,
    _: &mut CallContext<'_>,
) -> RtResult<()> {
    event_log_event_use(receiver)?
        .use_event
        .set(value.as_condition()?);
    Ok(())
}

fn event_log_use_description(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    Ok(event_log_event_use(receiver)?.description.borrow().clone())
}

fn set_event_log_use_description(
    receiver: &dyn ObjectProtocol,
    value: Value,
    _: &mut CallContext<'_>,
) -> RtResult<()> {
    *event_log_event_use(receiver)?.description.borrow_mut() = value;
    Ok(())
}

pub(crate) fn construct_event_log_access_event_use_description(
    _: &mut CallContext<'_>,
    arguments: &[Value],
) -> RtResult<Value> {
    let object = match arguments.first() {
        None | Some(Value::Undefined) => Arc::from(""),
        Some(Value::Str(value)) => Arc::from(value.to_string()),
        Some(_) => {
            return Err(RtError::TypeError {
                expected: "Строка",
                op: "Новый ОписаниеИспользованияСобытияДоступЖурналаРегистрации",
            });
        }
    };
    Ok(Value::new_object(EventLogAccessEventUseDescriptionObject {
        object: RefCell::new(object),
        access_fields: RefCell::new(Value::new_array(Vec::new())),
        logged_fields: RefCell::new(Value::new_array(Vec::new())),
    }))
}

fn event_log_access_event_use_description(
    receiver: &dyn ObjectProtocol,
) -> RtResult<&EventLogAccessEventUseDescriptionObject> {
    receiver
        .downcast_ref::<EventLogAccessEventUseDescriptionObject>()
        .ok_or(RtError::NotAnObject)
}

fn event_log_access_object(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    Ok(Value::Str(
        event_log_access_event_use_description(receiver)?
            .object
            .borrow()
            .as_ref()
            .into(),
    ))
}

fn set_event_log_access_object(
    receiver: &dyn ObjectProtocol,
    value: Value,
    _: &mut CallContext<'_>,
) -> RtResult<()> {
    let Value::Str(value) = value else {
        return Err(RtError::TypeError {
            expected: "Строка",
            op: "ОписаниеИспользованияСобытияДоступЖурналаРегистрации.Объект",
        });
    };
    *event_log_access_event_use_description(receiver)?
        .object
        .borrow_mut() = Arc::from(value.to_string());
    Ok(())
}

fn event_log_access_fields(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    Ok(event_log_access_event_use_description(receiver)?
        .access_fields
        .borrow()
        .clone())
}

fn event_log_logged_fields(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    Ok(event_log_access_event_use_description(receiver)?
        .logged_fields
        .borrow()
        .clone())
}

fn event_log_field_array(value: Value, op: &'static str) -> RtResult<Value> {
    if matches!(&value, Value::Object(object) if matches!(&**object, BslObject::Array(_))) {
        Ok(value)
    } else {
        Err(RtError::TypeError {
            expected: "Массив",
            op,
        })
    }
}

fn set_event_log_access_fields(
    receiver: &dyn ObjectProtocol,
    value: Value,
    _: &mut CallContext<'_>,
) -> RtResult<()> {
    *event_log_access_event_use_description(receiver)?
        .access_fields
        .borrow_mut() = event_log_field_array(
        value,
        "ОписаниеИспользованияСобытияДоступЖурналаРегистрации.ПоляДоступа",
    )?;
    Ok(())
}

fn set_event_log_logged_fields(
    receiver: &dyn ObjectProtocol,
    value: Value,
    _: &mut CallContext<'_>,
) -> RtResult<()> {
    *event_log_access_event_use_description(receiver)?
        .logged_fields
        .borrow_mut() = event_log_field_array(
        value,
        "ОписаниеИспользованияСобытияДоступЖурналаРегистрации.ПоляРегистрации",
    )?;
    Ok(())
}

pub(crate) static EVENT_LOG_EVENT_USE_PROPERTIES: &[PropertyDescriptor] = &[
    PropertyDescriptor {
        names: &["Использование", "Use"],
        get: event_log_use,
        set: Some(set_event_log_use),
    },
    PropertyDescriptor {
        names: &["ОписаниеИспользования", "UseDescription"],
        get: event_log_use_description,
        set: Some(set_event_log_use_description),
    },
];

pub(crate) static EVENT_LOG_ACCESS_EVENT_USE_DESCRIPTION_PROPERTIES: &[PropertyDescriptor] = &[
    PropertyDescriptor {
        names: &["Объект", "Object"],
        get: event_log_access_object,
        set: Some(set_event_log_access_object),
    },
    PropertyDescriptor {
        names: &["ПоляДоступа", "AccessFields"],
        get: event_log_access_fields,
        set: Some(set_event_log_access_fields),
    },
    PropertyDescriptor {
        names: &["ПоляРегистрации", "LoggedFields"],
        get: event_log_logged_fields,
        set: Some(set_event_log_logged_fields),
    },
];
