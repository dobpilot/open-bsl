//! Сведения о клиенте сеанса и настройки интерфейса клиентского приложения.

use super::*;

/// Описание одного экрана, переданное хостом при открытии сеанса клиента.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClientDisplayInformation {
    width: u32,
    height: u32,
    dpi: u32,
}

/// Скорость соединения, которую хост назначил одному клиентскому сеансу.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientConnectionSpeed {
    /// Низкая скорость соединения.
    Low,
    /// Обычная скорость соединения.
    Normal,
}

impl ClientConnectionSpeed {
    const fn value(self) -> EnumValue {
        match self {
            Self::Low => EnumValue::ClientConnectionSpeedLow,
            Self::Normal => EnumValue::ClientConnectionSpeedNormal,
        }
    }
}

impl ClientDisplayInformation {
    /// Создаёт снимок размеров экрана и плотности пикселей.
    #[must_use]
    pub const fn new(width: u32, height: u32, dpi: u32) -> Self {
        Self { width, height, dpi }
    }

    /// Возвращает ширину экрана.
    #[must_use]
    pub const fn width(self) -> u32 {
        self.width
    }

    /// Возвращает высоту экрана.
    #[must_use]
    pub const fn height(self) -> u32 {
        self.height
    }

    /// Возвращает плотность экрана в точках на дюйм.
    #[must_use]
    pub const fn dpi(self) -> u32 {
        self.dpi
    }
}

#[derive(Debug)]
struct ClientSettingsObject {
    show_navigation_and_actions_panels: Cell<bool>,
    show_sections_panel: Cell<bool>,
    forms_openning_mode: Cell<EnumValue>,
    interface_variant: Cell<EnumValue>,
}

#[derive(Debug)]
struct ClientApplicationObject {
    kind: SessionKind,
}

#[derive(Debug)]
struct CommandInterfaceSettingsObject {
    sections_panel_representation: Cell<EnumValue>,
}

#[derive(Debug)]
struct ClientApplicationInterfaceSettingsObject {
    content: RefCell<Option<Value>>,
}

#[derive(Debug)]
struct ClientApplicationInterfaceContentSettingsObject {
    top: Value,
    left: Value,
    bottom: Value,
    right: Value,
}

#[derive(Debug)]
struct ClientApplicationInterfaceContentSettingsGroupObject {
    items: RefCell<Vec<Value>>,
}

#[derive(Debug)]
struct ClientApplicationInterfaceContentSettingsItemObject {
    name: Arc<str>,
}

#[derive(Debug)]
struct ClientDisplayInformationObject {
    display: ClientDisplayInformation,
}

pub(crate) static CLIENT_SETTINGS_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-session",
    name: "НастройкиКлиентскогоПриложения",
    type_display: "Настройки клиентского приложения",
    type_names: &["ClientSettings"],
};

pub(crate) static CLIENT_APPLICATION_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-session",
    name: "КлиентскоеПриложение",
    type_display: "Клиентское приложение",
    type_names: &["ClientApplication"],
};

pub(crate) static COMMAND_INTERFACE_SETTINGS_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-session",
    name: "НастройкиКомандногоИнтерфейса",
    type_display: "Настройки командного интерфейса",
    type_names: &["CommandInterfaceSettings"],
};

pub(crate) static CLIENT_APPLICATION_INTERFACE_SETTINGS_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-session",
    name: "НастройкиИнтерфейсаКлиентскогоПриложения",
    type_display: "Настройки панелей",
    type_names: &["ClientApplicationInterfaceSettings"],
};

pub(crate) static CLIENT_APPLICATION_INTERFACE_CONTENT_SETTINGS_TYPE: TypeDescriptor =
    TypeDescriptor {
        package: "bsl-session",
        name: "НастройкиСоставаИнтерфейсаКлиентскогоПриложения",
        type_display: "Настройки состава интерфейса клиентского приложения",
        type_names: &["ClientApplicationInterfaceContentSettings"],
    };

pub(crate) static CLIENT_APPLICATION_INTERFACE_CONTENT_SETTINGS_GROUP_TYPE: TypeDescriptor =
    TypeDescriptor {
        package: "bsl-session",
        name: "ГруппаНастройкиСоставаИнтерфейсаКлиентскогоПриложения",
        type_display: "Группа настройки состава интерфейса клиентского приложения",
        type_names: &["ClientApplicationInterfaceContentSettingsGroup"],
    };

pub(crate) static CLIENT_APPLICATION_INTERFACE_CONTENT_SETTINGS_ITEM_TYPE: TypeDescriptor =
    TypeDescriptor {
        package: "bsl-session",
        name: "ЭлементНастройкиСоставаИнтерфейсаКлиентскогоПриложения",
        type_display: "Элемент настройки состава интерфейса клиентского приложения",
        type_names: &["ClientApplicationInterfaceContentSettingsItem"],
    };

pub(crate) static CLIENT_DISPLAY_INFORMATION_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-session",
    name: "ИнформацияЭкранаКлиента",
    type_display: "ИнформацияЭкранаКлиента",
    type_names: &["ClientDisplayInformation"],
};

impl ObjectProtocol for ClientDisplayInformationObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &CLIENT_DISPLAY_INFORMATION_TYPE
    }

    fn property_table(&self) -> &'static [PropertyDescriptor] {
        CLIENT_DISPLAY_INFORMATION_PROPERTIES
    }
}

fn client_display_information(
    receiver: &dyn ObjectProtocol,
) -> RtResult<&ClientDisplayInformationObject> {
    receiver
        .downcast_ref::<ClientDisplayInformationObject>()
        .ok_or(RtError::NotAnObject)
}

fn client_display_width(receiver: &dyn ObjectProtocol, _: &mut CallContext<'_>) -> RtResult<Value> {
    Ok(Value::number_from_i64(
        client_display_information(receiver)?.display.width() as i64,
    ))
}

fn client_display_height(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    Ok(Value::number_from_i64(
        client_display_information(receiver)?.display.height() as i64,
    ))
}

fn client_display_dpi(receiver: &dyn ObjectProtocol, _: &mut CallContext<'_>) -> RtResult<Value> {
    Ok(Value::number_from_i64(
        client_display_information(receiver)?.display.dpi() as i64,
    ))
}

pub(crate) static CLIENT_DISPLAY_INFORMATION_PROPERTIES: &[PropertyDescriptor] = &[
    PropertyDescriptor {
        names: &["Ширина", "Width"],
        get: client_display_width,
        set: None,
    },
    PropertyDescriptor {
        names: &["Высота", "Height"],
        get: client_display_height,
        set: None,
    },
    PropertyDescriptor {
        names: &["DPI"],
        get: client_display_dpi,
        set: None,
    },
];

impl ObjectProtocol for ClientSettingsObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &CLIENT_SETTINGS_TYPE
    }

    fn property_table(&self) -> &'static [PropertyDescriptor] {
        CLIENT_SETTINGS_PROPERTIES
    }
}

impl ObjectProtocol for ClientApplicationObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &CLIENT_APPLICATION_TYPE
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        CLIENT_APPLICATION_METHODS
    }
}

impl ObjectProtocol for CommandInterfaceSettingsObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &COMMAND_INTERFACE_SETTINGS_TYPE
    }

    fn property_table(&self) -> &'static [PropertyDescriptor] {
        COMMAND_INTERFACE_SETTINGS_PROPERTIES
    }
}

impl ObjectProtocol for ClientApplicationInterfaceSettingsObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &CLIENT_APPLICATION_INTERFACE_SETTINGS_TYPE
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        CLIENT_APPLICATION_INTERFACE_SETTINGS_METHODS
    }
}

impl ObjectProtocol for ClientApplicationInterfaceContentSettingsObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &CLIENT_APPLICATION_INTERFACE_CONTENT_SETTINGS_TYPE
    }

    fn property_table(&self) -> &'static [PropertyDescriptor] {
        CLIENT_APPLICATION_INTERFACE_CONTENT_SETTINGS_PROPERTIES
    }
}

impl ObjectProtocol for ClientApplicationInterfaceContentSettingsGroupObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &CLIENT_APPLICATION_INTERFACE_CONTENT_SETTINGS_GROUP_TYPE
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        CLIENT_APPLICATION_INTERFACE_CONTENT_SETTINGS_GROUP_METHODS
    }
}

impl ObjectProtocol for ClientApplicationInterfaceContentSettingsItemObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &CLIENT_APPLICATION_INTERFACE_CONTENT_SETTINGS_ITEM_TYPE
    }

    fn property_table(&self) -> &'static [PropertyDescriptor] {
        CLIENT_APPLICATION_INTERFACE_CONTENT_SETTINGS_ITEM_PROPERTIES
    }
}

pub(crate) fn client_application(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    Ok(Value::new_object(ClientApplicationObject {
        kind: runtime(ctx)?.kind(),
    }))
}

fn client_application_type(
    receiver: &dyn ObjectProtocol,
    _: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let application = receiver
        .downcast_ref::<ClientApplicationObject>()
        .ok_or(RtError::NotAnObject)?;
    match application.kind {
        // НЕ ИЗМЕРЕНО(SESSION.CLIENT_APPLICATION.WEB_TYPE) — для веб-клиента
        // пока нет прямого платформенного прогона.
        SessionKind::WebClient => Ok(Value::Enum(EnumValue::ClientApplicationTypeWebClient)),
        SessionKind::ThinClient => Ok(Value::Enum(EnumValue::ClientApplicationTypeThinClient)),
        SessionKind::BackgroundJob => Ok(Value::Undefined),
    }
}

pub(crate) fn client_displays_information(
    ctx: &mut CallContext<'_>,
    _: &[Value],
) -> RtResult<Value> {
    let runtime = runtime(ctx)?;
    if runtime.kind() == SessionKind::BackgroundJob {
        return Ok(Value::Undefined);
    }
    let displays = runtime
        .client_displays()
        .ok_or_else(|| RtError::DynamicError("снимок экранов клиента не передан хостом".into()))?;
    Ok(fixed_array_from_values(
        displays
            .iter()
            .map(|display| Value::new_object(ClientDisplayInformationObject { display: *display }))
            .collect(),
    ))
}

pub(crate) fn client_connection_speed(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    let speed = runtime(ctx)?.client_connection_speed().ok_or_else(|| {
        RtError::DynamicError("скорость клиентского соединения не передана хостом".into())
    })?;
    Ok(Value::Enum(speed.value()))
}

pub(crate) fn current_client_interface_variant(
    ctx: &mut CallContext<'_>,
    _: &[Value],
) -> RtResult<Value> {
    let variant = runtime(ctx)?
        .current_client_interface_variant
        .ok_or_else(|| {
            RtError::DynamicError("вариант интерфейса клиента не передан хостом".into())
        })?;
    Ok(Value::Enum(variant))
}

pub(crate) fn construct_client_settings(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    Ok(Value::new_object(ClientSettingsObject {
        show_navigation_and_actions_panels: Cell::new(true),
        show_sections_panel: Cell::new(true),
        forms_openning_mode: Cell::new(EnumValue::ApplicationFormsOpenningModeTabs),
        interface_variant: Cell::new(EnumValue::ClientApplicationInterfaceVariantVersion8_2),
    }))
}

fn client_settings(receiver: &dyn ObjectProtocol) -> RtResult<&ClientSettingsObject> {
    receiver
        .downcast_ref::<ClientSettingsObject>()
        .ok_or(RtError::NotAnObject)
}

fn show_navigation_and_actions_panels(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    Ok(Value::Boolean(
        client_settings(receiver)?
            .show_navigation_and_actions_panels
            .get(),
    ))
}

fn set_show_navigation_and_actions_panels(
    receiver: &dyn ObjectProtocol,
    value: Value,
    _: &mut CallContext<'_>,
) -> RtResult<()> {
    let Value::Boolean(value) = value else {
        return Err(RtError::TypeError {
            expected: "Булево",
            op: "НастройкиКлиентскогоПриложения.ОтображатьПанелиНавигацииИДействий",
        });
    };
    client_settings(receiver)?
        .show_navigation_and_actions_panels
        .set(value);
    Ok(())
}

fn show_sections_panel(receiver: &dyn ObjectProtocol, _: &mut CallContext<'_>) -> RtResult<Value> {
    Ok(Value::Boolean(
        client_settings(receiver)?.show_sections_panel.get(),
    ))
}

fn set_show_sections_panel(
    receiver: &dyn ObjectProtocol,
    value: Value,
    _: &mut CallContext<'_>,
) -> RtResult<()> {
    let Value::Boolean(value) = value else {
        return Err(RtError::TypeError {
            expected: "Булево",
            op: "НастройкиКлиентскогоПриложения.ОтображатьПанельРазделов",
        });
    };
    client_settings(receiver)?.show_sections_panel.set(value);
    Ok(())
}

fn application_forms_openning_mode(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    Ok(Value::Enum(
        client_settings(receiver)?.forms_openning_mode.get(),
    ))
}

fn set_application_forms_openning_mode(
    receiver: &dyn ObjectProtocol,
    value: Value,
    _: &mut CallContext<'_>,
) -> RtResult<()> {
    let Value::Enum(
        value @ (EnumValue::ApplicationFormsOpenningModeTabs
        | EnumValue::ApplicationFormsOpenningModeSingleWindows),
    ) = value
    else {
        return Err(RtError::TypeError {
            expected: "РежимОткрытияФормПриложения",
            op: "НастройкиКлиентскогоПриложения.РежимОткрытияФормПриложения",
        });
    };
    client_settings(receiver)?.forms_openning_mode.set(value);
    Ok(())
}

fn client_application_interface_variant(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    Ok(Value::Enum(
        client_settings(receiver)?.interface_variant.get(),
    ))
}

fn set_client_application_interface_variant(
    receiver: &dyn ObjectProtocol,
    value: Value,
    _: &mut CallContext<'_>,
) -> RtResult<()> {
    let Value::Enum(
        value @ (EnumValue::ClientApplicationInterfaceVariantVersion8_2
        | EnumValue::ClientApplicationInterfaceVariantTaxi),
    ) = value
    else {
        return Err(RtError::TypeError {
            expected: "ВариантИнтерфейсаКлиентскогоПриложения",
            op: "НастройкиКлиентскогоПриложения.ВариантИнтерфейсаКлиентскогоПриложения",
        });
    };
    client_settings(receiver)?.interface_variant.set(value);
    Ok(())
}

pub(crate) fn construct_command_interface_settings(
    _: &mut CallContext<'_>,
    _: &[Value],
) -> RtResult<Value> {
    Ok(Value::new_object(CommandInterfaceSettingsObject {
        sections_panel_representation: Cell::new(
            EnumValue::SectionsPanelRepresentationPictureAndText,
        ),
    }))
}

fn command_interface_settings(
    receiver: &dyn ObjectProtocol,
) -> RtResult<&CommandInterfaceSettingsObject> {
    receiver
        .downcast_ref::<CommandInterfaceSettingsObject>()
        .ok_or(RtError::NotAnObject)
}

fn sections_panel_representation(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    Ok(Value::Enum(
        command_interface_settings(receiver)?
            .sections_panel_representation
            .get(),
    ))
}

fn set_sections_panel_representation(
    receiver: &dyn ObjectProtocol,
    value: Value,
    _: &mut CallContext<'_>,
) -> RtResult<()> {
    let Value::Enum(
        value @ (EnumValue::SectionsPanelRepresentationPicture
        | EnumValue::SectionsPanelRepresentationPictureAndText
        | EnumValue::SectionsPanelRepresentationPictureOnTopAndText
        | EnumValue::SectionsPanelRepresentationPictureOnLeftAndText
        | EnumValue::SectionsPanelRepresentationText),
    ) = value
    else {
        return Err(RtError::TypeError {
            expected: "ОтображениеПанелиРазделов",
            op: "НастройкиКомандногоИнтерфейса.ОтображениеПанелиРазделов",
        });
    };
    command_interface_settings(receiver)?
        .sections_panel_representation
        .set(value);
    Ok(())
}

fn new_client_application_interface_content_settings_group() -> Value {
    Value::new_object(ClientApplicationInterfaceContentSettingsGroupObject {
        items: RefCell::new(Vec::new()),
    })
}

pub(crate) fn construct_client_application_interface_settings(
    _: &mut CallContext<'_>,
    _: &[Value],
) -> RtResult<Value> {
    Ok(Value::new_object(
        ClientApplicationInterfaceSettingsObject {
            content: RefCell::new(None),
        },
    ))
}

pub(crate) fn construct_client_application_interface_content_settings(
    _: &mut CallContext<'_>,
    _: &[Value],
) -> RtResult<Value> {
    Ok(Value::new_object(
        ClientApplicationInterfaceContentSettingsObject {
            top: new_client_application_interface_content_settings_group(),
            left: new_client_application_interface_content_settings_group(),
            bottom: new_client_application_interface_content_settings_group(),
            right: new_client_application_interface_content_settings_group(),
        },
    ))
}

pub(crate) fn construct_client_application_interface_content_settings_group(
    _: &mut CallContext<'_>,
    _: &[Value],
) -> RtResult<Value> {
    Ok(new_client_application_interface_content_settings_group())
}

pub(crate) fn construct_client_application_interface_content_settings_item(
    _: &mut CallContext<'_>,
    arguments: &[Value],
) -> RtResult<Value> {
    let Value::Str(name) = &arguments[0] else {
        return Err(RtError::TypeError {
            expected: "Строка",
            op: "Новый ЭлементНастройкиСоставаИнтерфейсаКлиентскогоПриложения",
        });
    };
    Ok(Value::new_object(
        ClientApplicationInterfaceContentSettingsItemObject {
            name: Arc::from(name.to_string()),
        },
    ))
}

fn client_application_interface_settings(
    receiver: &dyn ObjectProtocol,
) -> RtResult<&ClientApplicationInterfaceSettingsObject> {
    receiver
        .downcast_ref::<ClientApplicationInterfaceSettingsObject>()
        .ok_or(RtError::NotAnObject)
}

fn get_client_application_interface_content(
    receiver: &dyn ObjectProtocol,
    _: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    client_application_interface_settings(receiver)?
        .content
        .borrow()
        .clone()
        .ok_or_else(|| {
            RtError::DynamicError(
                "исходный состав интерфейса клиентского приложения не предоставлен".into(),
            )
        })
}

fn set_client_application_interface_content(
    receiver: &dyn ObjectProtocol,
    arguments: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let Some(content) = arguments.first() else {
        return Err(RtError::DynamicError(
            "состав интерфейса клиентского приложения не передан".into(),
        ));
    };
    let valid = content
        .object_ref()
        .and_then(|object| object.downcast_ref::<ClientApplicationInterfaceContentSettingsObject>())
        .is_some();
    if !valid {
        return Err(RtError::TypeError {
            expected: "НастройкиСоставаИнтерфейсаКлиентскогоПриложения",
            op: "УстановитьСостав",
        });
    }
    *client_application_interface_settings(receiver)?
        .content
        .borrow_mut() = Some(content.clone());
    Ok(Value::Undefined)
}

fn client_application_interface_content_settings(
    receiver: &dyn ObjectProtocol,
) -> RtResult<&ClientApplicationInterfaceContentSettingsObject> {
    receiver
        .downcast_ref::<ClientApplicationInterfaceContentSettingsObject>()
        .ok_or(RtError::NotAnObject)
}

macro_rules! client_application_interface_content_property {
    ($function:ident, $field:ident) => {
        fn $function(receiver: &dyn ObjectProtocol, _: &mut CallContext<'_>) -> RtResult<Value> {
            Ok(client_application_interface_content_settings(receiver)?
                .$field
                .clone())
        }
    };
}

client_application_interface_content_property!(client_interface_content_top, top);

client_application_interface_content_property!(client_interface_content_left, left);

client_application_interface_content_property!(client_interface_content_bottom, bottom);

client_application_interface_content_property!(client_interface_content_right, right);

fn client_application_interface_content_settings_group(
    receiver: &dyn ObjectProtocol,
) -> RtResult<&ClientApplicationInterfaceContentSettingsGroupObject> {
    receiver
        .downcast_ref::<ClientApplicationInterfaceContentSettingsGroupObject>()
        .ok_or(RtError::NotAnObject)
}

fn add_client_application_interface_content(
    receiver: &dyn ObjectProtocol,
    arguments: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let value = &arguments[0];
    let Some(object) = value.object_ref() else {
        return Err(RtError::TypeError {
            expected: "Элемент или группа настройки состава интерфейса клиентского приложения",
            op: "Добавить",
        });
    };
    if object
        .downcast_ref::<ClientApplicationInterfaceContentSettingsItemObject>()
        .is_none()
        && object
            .downcast_ref::<ClientApplicationInterfaceContentSettingsGroupObject>()
            .is_none()
    {
        return Err(RtError::TypeError {
            expected: "Элемент или группа настройки состава интерфейса клиентского приложения",
            op: "Добавить",
        });
    }
    let group = client_application_interface_content_settings_group(receiver)?;
    let mut items = group.items.borrow_mut();
    items.try_reserve(1).map_err(|_| {
        RtError::ResourceLimit("не удалось расширить группу состава интерфейса".into())
    })?;
    items.push(value.clone());
    Ok(Value::Undefined)
}

fn count_client_application_interface_content(
    receiver: &dyn ObjectProtocol,
    _: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let len = client_application_interface_content_settings_group(receiver)?
        .items
        .borrow()
        .len();
    Ok(Value::number_from_i64(
        i64::try_from(len).unwrap_or(i64::MAX),
    ))
}

fn get_client_application_interface_content_item(
    receiver: &dyn ObjectProtocol,
    arguments: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let Value::Number(index) = &arguments[0] else {
        return Err(RtError::BadIndex);
    };
    let index = index.to_i64_exact().ok_or(RtError::BadIndex)?;
    let position = usize::try_from(index).map_err(|_| RtError::BadIndex)?;
    let items = client_application_interface_content_settings_group(receiver)?
        .items
        .borrow();
    items
        .get(position)
        .cloned()
        .ok_or(RtError::IndexOutOfBounds {
            index,
            len: items.len(),
        })
}

fn client_application_interface_content_item_name(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let item = receiver
        .downcast_ref::<ClientApplicationInterfaceContentSettingsItemObject>()
        .ok_or(RtError::NotAnObject)?;
    Ok(Value::Str(item.name.as_ref().into()))
}

pub(crate) static CLIENT_SETTINGS_PROPERTIES: &[PropertyDescriptor] = &[
    PropertyDescriptor {
        names: &[
            "ОтображатьПанелиНавигацииИДействий",
            "ShowNavigationAndActionsPanels",
        ],
        get: show_navigation_and_actions_panels,
        set: Some(set_show_navigation_and_actions_panels),
    },
    PropertyDescriptor {
        names: &["ОтображатьПанельРазделов", "ShowSectionsPanel"],
        get: show_sections_panel,
        set: Some(set_show_sections_panel),
    },
    PropertyDescriptor {
        names: &[
            "РежимОткрытияФормПриложения",
            "ApplicationFormsOpenningMode",
        ],
        get: application_forms_openning_mode,
        set: Some(set_application_forms_openning_mode),
    },
    PropertyDescriptor {
        names: &[
            "ВариантИнтерфейсаКлиентскогоПриложения",
            "ClientApplicationInterfaceVariant",
        ],
        get: client_application_interface_variant,
        set: Some(set_client_application_interface_variant),
    },
];

pub(crate) static COMMAND_INTERFACE_SETTINGS_PROPERTIES: &[PropertyDescriptor] =
    &[PropertyDescriptor {
        names: &["ОтображениеПанелиРазделов", "SectionsPanelRepresentation"],
        get: sections_panel_representation,
        set: Some(set_sections_panel_representation),
    }];

pub(crate) static CLIENT_APPLICATION_METHODS: &[MethodDescriptor] = &[MethodDescriptor::new(
    &["ТипПриложения", "ApplicationType"],
    Arity::exact(0),
    client_application_type,
)];

pub(crate) static CLIENT_APPLICATION_INTERFACE_SETTINGS_METHODS: &[MethodDescriptor] = &[
    MethodDescriptor::new(
        &["ПолучитьСостав", "GetContent"],
        Arity::exact(0),
        get_client_application_interface_content,
    ),
    MethodDescriptor::new(
        &["УстановитьСостав", "SetContent"],
        Arity::range(0, 1),
        set_client_application_interface_content,
    )
    .as_procedure(),
];

pub(crate) static CLIENT_APPLICATION_INTERFACE_CONTENT_SETTINGS_PROPERTIES:
    &[PropertyDescriptor] = &[
    PropertyDescriptor {
        names: &["Верх", "Top"],
        get: client_interface_content_top,
        set: None,
    },
    PropertyDescriptor {
        names: &["Лево", "Left"],
        get: client_interface_content_left,
        set: None,
    },
    PropertyDescriptor {
        names: &["Низ", "Bottom"],
        get: client_interface_content_bottom,
        set: None,
    },
    PropertyDescriptor {
        names: &["Право", "Right"],
        get: client_interface_content_right,
        set: None,
    },
];

pub(crate) static CLIENT_APPLICATION_INTERFACE_CONTENT_SETTINGS_GROUP_METHODS:
    &[MethodDescriptor] = &[
    MethodDescriptor::new(
        &["Добавить", "Add"],
        Arity::exact(1),
        add_client_application_interface_content,
    )
    .as_procedure(),
    MethodDescriptor::new(
        &["Количество", "Count"],
        Arity::exact(0),
        count_client_application_interface_content,
    ),
    MethodDescriptor::new(
        &["Получить", "Get"],
        Arity::exact(1),
        get_client_application_interface_content_item,
    ),
];

pub(crate) static CLIENT_APPLICATION_INTERFACE_CONTENT_SETTINGS_ITEM_PROPERTIES:
    &[PropertyDescriptor] = &[PropertyDescriptor {
    names: &["Имя", "Name"],
    get: client_application_interface_content_item_name,
    set: None,
}];
