//! Настройки обработки ошибок и `ОбработкаОшибок`.

use super::*;

/// Режим отправки отчёта об ошибке из настроек текущего пользователя.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorReportingMode {
    /// Платформа выбирает режим автоматически.
    Auto,
    /// Отчёт не отправляется.
    DontSend,
    /// Решение принимает пользователь.
    AskUser,
    /// Отчёт отправляется.
    Send,
}

impl ErrorReportingMode {
    fn value(self) -> Value {
        Value::Enum(match self {
            Self::Auto => EnumValue::ErrorReportingModeAuto,
            Self::DontSend => EnumValue::ErrorReportingModeDontSend,
            Self::AskUser => EnumValue::ErrorReportingModeAskUser,
            Self::Send => EnumValue::ErrorReportingModeSend,
        })
    }
}

/// Неизменяемые настройки обработки ошибок текущего пользователя.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorProcessingSettingsData {
    send_report_on_client: ErrorReportingMode,
    service_address: Arc<str>,
}

impl ErrorProcessingSettingsData {
    /// Создаёт снимок с режимом отправки и адресом сервиса регистрации.
    #[must_use]
    pub fn new(
        send_report_on_client: ErrorReportingMode,
        service_address: impl Into<String>,
    ) -> Self {
        Self {
            send_report_on_client,
            service_address: Arc::from(service_address.into()),
        }
    }

    /// Возвращает режим отправки отчёта на клиенте.
    #[must_use]
    pub const fn send_report_on_client(&self) -> ErrorReportingMode {
        self.send_report_on_client
    }

    /// Возвращает адрес сервиса обработки ошибок.
    #[must_use]
    pub fn service_address(&self) -> &str {
        &self.service_address
    }
}

#[derive(Debug)]
struct ErrorProcessingManagerObject;

#[derive(Debug)]
struct ErrorProcessingSettingsObject {
    data: ErrorProcessingSettingsData,
}

pub(crate) static ERROR_PROCESSING_MANAGER_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-session",
    name: "МенеджерОбработкиОшибок",
    type_display: "Менеджер обработки ошибок",
    type_names: &["ErrorProcessingManager"],
};

pub(crate) static ERROR_PROCESSING_SETTINGS_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-session",
    name: "НастройкиОбработкиОшибок",
    type_display: "Настройки обработки ошибок на клиенте",
    type_names: &["ErrorProcessingSettings"],
};

impl ObjectProtocol for ErrorProcessingManagerObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &ERROR_PROCESSING_MANAGER_TYPE
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        ERROR_PROCESSING_MANAGER_METHODS
    }
}

impl ObjectProtocol for ErrorProcessingSettingsObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &ERROR_PROCESSING_SETTINGS_TYPE
    }

    fn property_table(&self) -> &'static [PropertyDescriptor] {
        ERROR_PROCESSING_SETTINGS_PROPERTIES
    }
}

pub(crate) fn error_processing(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    Ok(Value::new_object(ErrorProcessingManagerObject))
}

fn error_processing_brief(
    receiver: &dyn ObjectProtocol,
    arguments: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    receiver
        .downcast_ref::<ErrorProcessingManagerObject>()
        .ok_or(RtError::NotAnObject)?;
    brief_error_description(&arguments[0])
}

fn error_processing_detail(
    receiver: &dyn ObjectProtocol,
    arguments: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    receiver
        .downcast_ref::<ErrorProcessingManagerObject>()
        .ok_or(RtError::NotAnObject)?;
    detailed_error_description(&arguments[0])
}

fn error_processing_user_settings(
    receiver: &dyn ObjectProtocol,
    _: &[Value],
    ctx: &mut CallContext<'_>,
) -> RtResult<Value> {
    receiver
        .downcast_ref::<ErrorProcessingManagerObject>()
        .ok_or(RtError::NotAnObject)?;
    let data = runtime(ctx)?
        .error_processing_settings()
        .cloned()
        .ok_or_else(|| {
            RtError::DynamicError(
                "настройки обработки ошибок текущего пользователя не переданы хостом".into(),
            )
        })?;
    Ok(Value::new_object(ErrorProcessingSettingsObject { data }))
}

fn error_settings_send_report(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let settings = receiver
        .downcast_ref::<ErrorProcessingSettingsObject>()
        .ok_or(RtError::NotAnObject)?;
    Ok(settings.data.send_report_on_client().value())
}

fn error_settings_service_address(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let settings = receiver
        .downcast_ref::<ErrorProcessingSettingsObject>()
        .ok_or(RtError::NotAnObject)?;
    Ok(Value::Str(settings.data.service_address().into()))
}

pub(crate) static ERROR_PROCESSING_MANAGER_METHODS: &[MethodDescriptor] = &[
    MethodDescriptor::new(
        &["КраткоеПредставлениеОшибки", "BriefErrorDescription"],
        Arity::exact(1),
        error_processing_brief,
    ),
    MethodDescriptor::new(
        &["ПодробноеПредставлениеОшибки", "DetailErrorDescription"],
        Arity::exact(1),
        error_processing_detail,
    ),
    MethodDescriptor::new(
        &["ПолучитьНастройкиПользователя", "GetUserSettings"],
        Arity::exact(0),
        error_processing_user_settings,
    ),
];

pub(crate) static ERROR_PROCESSING_SETTINGS_PROPERTIES: &[PropertyDescriptor] = &[
    PropertyDescriptor {
        names: &["ОтправлятьОтчетНаКлиенте", "SendReportOnClient"],
        get: error_settings_send_report,
        set: None,
    },
    PropertyDescriptor {
        names: &[
            "АдресСервисаОбработкиОшибок",
            "ErrorProcessingServiceAddress",
        ],
        get: error_settings_service_address,
        set: None,
    },
];
