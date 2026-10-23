//! Пользователи информационной базы, политики паролей, блокировка
//! аутентификации и генератор случайных паролей.

use super::*;

#[derive(Debug)]
struct InfoBaseUsersManager;

#[derive(Debug)]
struct UserPasswordPoliciesManager;

#[derive(Debug)]
pub(crate) struct InfoBaseUserValue {
    pub(crate) principal: Principal,
    data_separation: RefCell<Option<Value>>,
}

#[derive(Debug)]
struct UserPasswordPolicyValue {
    snapshot: UserPasswordPolicySnapshot,
}

#[derive(Debug)]
struct AuthenticationLockManager;

#[derive(Debug)]
struct AuthenticationLockSettingsValue {
    lock_duration: RefCell<BslNumber>,
    max_unsuccessful_attempts_count: RefCell<BslNumber>,
    user_name_addition_codes: RefCell<Arc<str>>,
}

#[derive(Debug)]
struct RandomPasswordGenerator;

pub(crate) static INFO_BASE_USERS_MANAGER_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-orm",
    name: "МенеджерПользователейИнформационнойБазы",
    type_display: "InfoBase users manager",
    type_names: &["InfoBaseUsersManager"],
};

pub(crate) static INFO_BASE_USER_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-orm",
    name: "ПользовательИнформационнойБазы",
    type_display: "InfoBase user",
    type_names: &["InfoBaseUser"],
};

pub(crate) static USER_PASSWORD_POLICIES_MANAGER_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-orm",
    name: "МенеджерПолитикПаролейПользователей",
    type_display: "User password policy manager",
    type_names: &["UserPasswordPolicyManager"],
};

pub(crate) static USER_PASSWORD_POLICY_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-orm",
    name: "ПолитикаПаролейПользователей",
    type_display: "User password policy",
    type_names: &["UserPasswordPolicy"],
};

pub(crate) static AUTHENTICATION_LOCK_MANAGER_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-orm",
    name: "МенеджерБлокировкиАутентификацииПользователейИнформационнойБазы",
    type_display: "Manager of infobase users authentication lock",
    type_names: &["InfoBaseUserAuthenticationLockManager"],
};

pub(crate) static AUTHENTICATION_LOCK_SETTINGS_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-orm",
    name: "НастройкиБлокировкиАутентификацииПользователейИнформационнойБазы",
    type_display: "Settings for infobase users authentication lock",
    type_names: &["InfoBaseUserAuthenticationLockSettings"],
};

pub(crate) static RANDOM_PASSWORD_GENERATOR_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-orm",
    name: "ГенераторСлучайныхПаролей",
    type_display: "Random password generator",
    type_names: &["RandomPasswordGenerator"],
};

impl ObjectProtocol for InfoBaseUsersManager {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &INFO_BASE_USERS_MANAGER_TYPE
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        INFO_BASE_USERS_MANAGER_METHODS
    }
}

impl ObjectProtocol for UserPasswordPoliciesManager {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &USER_PASSWORD_POLICIES_MANAGER_TYPE
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        USER_PASSWORD_POLICIES_MANAGER_METHODS
    }
}

impl ObjectProtocol for InfoBaseUserValue {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &INFO_BASE_USER_TYPE
    }

    fn property_table(&self) -> &'static [PropertyDescriptor] {
        INFO_BASE_USER_PROPERTIES
    }
}

impl ObjectProtocol for UserPasswordPolicyValue {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &USER_PASSWORD_POLICY_TYPE
    }

    fn property_table(&self) -> &'static [PropertyDescriptor] {
        USER_PASSWORD_POLICY_PROPERTIES
    }
}

impl ObjectProtocol for AuthenticationLockManager {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &AUTHENTICATION_LOCK_MANAGER_TYPE
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        AUTHENTICATION_LOCK_MANAGER_METHODS
    }
}

impl ObjectProtocol for AuthenticationLockSettingsValue {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &AUTHENTICATION_LOCK_SETTINGS_TYPE
    }

    fn property_table(&self) -> &'static [PropertyDescriptor] {
        AUTHENTICATION_LOCK_SETTINGS_PROPERTIES
    }
}

impl ObjectProtocol for RandomPasswordGenerator {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &RANDOM_PASSWORD_GENERATOR_TYPE
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        RANDOM_PASSWORD_GENERATOR_METHODS
    }
}

pub(crate) fn new_random_password_generator(
    _: &mut CallContext<'_>,
    _: &[Value],
) -> RtResult<Value> {
    Ok(Value::new_object(RandomPasswordGenerator))
}

fn random_password(
    receiver: &dyn ObjectProtocol,
    _: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    receiver
        .downcast_ref::<RandomPasswordGenerator>()
        .ok_or(RtError::NotAnObject)?;
    Err(RtError::DynamicError(
        "криптографический источник генерации паролей не предоставлен хостом".into(),
    ))
}

pub(crate) static INFO_BASE_USERS_MANAGER_METHODS: &[MethodDescriptor] = &[MethodDescriptor::new(
    &["ТекущийПользователь", "CurrentUser"],
    Arity::exact(0),
    current_user,
)];

pub(crate) static USER_PASSWORD_POLICIES_MANAGER_METHODS: &[MethodDescriptor] =
    &[MethodDescriptor::new(
        &["НайтиПоИмени", "FindByName"],
        Arity::range(0, 1),
        find_user_password_policy,
    )];

pub(crate) static AUTHENTICATION_LOCK_MANAGER_METHODS: &[MethodDescriptor] = &[
    MethodDescriptor::new(
        &["ПолучитьНастройки", "GetSettings"],
        Arity::exact(0),
        get_authentication_lock_settings,
    ),
    MethodDescriptor::new(
        &["УстановитьНастройки", "SetSettings"],
        Arity::range(0, 1),
        set_authentication_lock_settings,
    )
    .as_procedure(),
];

pub(crate) static INFO_BASE_USER_PROPERTIES: &[PropertyDescriptor] = &[
    PropertyDescriptor {
        names: &["Имя", "Name"],
        get: current_user_name,
        set: None,
    },
    PropertyDescriptor {
        names: &["РазделениеДанных", "DataSeparation"],
        get: current_user_data_separation,
        set: None,
    },
    PropertyDescriptor {
        names: &["УникальныйИдентификатор", "UUID"],
        get: current_user_uuid,
        set: None,
    },
];

pub(crate) static USER_PASSWORD_POLICY_PROPERTIES: &[PropertyDescriptor] = &[PropertyDescriptor {
    names: &["МинимальнаяДлинаПаролей", "PasswordMinLength"],
    get: user_password_minimum_length,
    set: None,
}];

pub(crate) static AUTHENTICATION_LOCK_SETTINGS_PROPERTIES: &[PropertyDescriptor] = &[
    PropertyDescriptor {
        names: &["ДлительностьБлокировки", "LockDuration"],
        get: authentication_lock_duration,
        set: Some(set_authentication_lock_duration),
    },
    PropertyDescriptor {
        names: &[
            "МаксимальноеКоличествоНеуспешныхПопыток",
            "MaxUnsuccessfulAttemptsCount",
        ],
        get: authentication_lock_max_unsuccessful_attempts_count,
        set: Some(set_authentication_lock_max_unsuccessful_attempts_count),
    },
    PropertyDescriptor {
        names: &["КодыДополненияИмениПользователя", "UserNameAdditionCodes"],
        get: authentication_lock_user_name_addition_codes,
        set: Some(set_authentication_lock_user_name_addition_codes),
    },
];

pub(crate) static RANDOM_PASSWORD_GENERATOR_METHODS: &[MethodDescriptor] =
    &[MethodDescriptor::new(
        &["СлучайныйПароль", "RandomPassword"],
        Arity::range(0, 2),
        random_password,
    )];

pub(crate) fn info_base_users(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    Ok(Value::new_object(InfoBaseUsersManager))
}

pub(crate) fn user_password_policies(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    Ok(Value::new_object(UserPasswordPoliciesManager))
}

fn current_user(
    receiver: &dyn ObjectProtocol,
    _: &[Value],
    context: &mut CallContext<'_>,
) -> RtResult<Value> {
    receiver
        .downcast_ref::<InfoBaseUsersManager>()
        .ok_or(RtError::NotAnObject)?;
    Ok(Value::new_object(InfoBaseUserValue {
        principal: session(context)?.principal().clone(),
        data_separation: RefCell::new(None),
    }))
}

fn current_user_data_separation(
    receiver: &dyn ObjectProtocol,
    context: &mut CallContext<'_>,
) -> RtResult<Value> {
    let user = receiver
        .downcast_ref::<InfoBaseUserValue>()
        .ok_or(RtError::NotAnObject)?;
    if let Some(value) = user.data_separation.borrow().as_ref() {
        return Ok(value.clone());
    }
    let graph = user.principal.data_separation.as_ref().ok_or_else(|| {
        RtError::DynamicError("разделение данных пользователя не предоставлено источником".into())
    })?;
    if graph.root_count() != 1 {
        return Err(RtError::DynamicError(
            "снимок разделения данных пользователя должен иметь один корень".into(),
        ));
    }
    let mut values = graph.materialize(context.runtime_shapes())?;
    let value = values.pop().expect("число корней проверено");
    if !matches!(&value, Value::Object(object) if matches!(object.as_ref(), BslObject::Structure(_)))
    {
        return Err(RtError::TypeError {
            expected: "Структура",
            op: "ПользовательИнформационнойБазы.РазделениеДанных",
        });
    }
    // Изменения возвращённой структуры принадлежат этому объекту пользователя;
    // следующий `ТекущийПользователь` снова материализует неизменяемый снимок.
    *user.data_separation.borrow_mut() = Some(value.clone());
    Ok(value)
}

fn current_user_uuid(receiver: &dyn ObjectProtocol, _: &mut CallContext<'_>) -> RtResult<Value> {
    let user = receiver
        .downcast_ref::<InfoBaseUserValue>()
        .ok_or(RtError::NotAnObject)?;
    Ok(Value::Object(Rc::new(BslObject::Uuid(
        open_bsl::uuid::parse(user.principal.id())?,
    ))))
}

fn current_user_name(receiver: &dyn ObjectProtocol, _: &mut CallContext<'_>) -> RtResult<Value> {
    let user = receiver
        .downcast_ref::<InfoBaseUserValue>()
        .ok_or(RtError::NotAnObject)?;
    Ok(Value::Str(user.principal.name().into()))
}

fn find_user_password_policy(
    receiver: &dyn ObjectProtocol,
    arguments: &[Value],
    context: &mut CallContext<'_>,
) -> RtResult<Value> {
    receiver
        .downcast_ref::<UserPasswordPoliciesManager>()
        .ok_or(RtError::NotAnObject)?;
    let name = match arguments.first() {
        None => None,
        Some(Value::Str(name)) => Some(name.to_string()),
        Some(_) => {
            return Err(RtError::TypeError {
                expected: "Строка",
                op: "НайтиПоИмени",
            });
        }
    };
    let (principal, data_access) = current_data_access(context)?;
    Ok(data_access
        .user_password_policy(name.as_deref(), &principal, context)?
        .map_or(Value::Undefined, |snapshot| {
            Value::new_object(UserPasswordPolicyValue { snapshot })
        }))
}

fn user_password_minimum_length(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let policy = receiver
        .downcast_ref::<UserPasswordPolicyValue>()
        .ok_or(RtError::NotAnObject)?;
    Ok(Value::number_from_i64(
        policy.snapshot.minimum_password_length().into(),
    ))
}

pub(crate) fn info_base_user_password_minimum_length(
    context: &mut CallContext<'_>,
    _: &[Value],
) -> RtResult<Value> {
    let (principal, data_access) = current_data_access(context)?;
    Ok(Value::number_from_i64(
        data_access
            .user_password_minimum_length(&principal, context)?
            .into(),
    ))
}

pub(crate) fn info_base_user_password_strength_check(
    context: &mut CallContext<'_>,
    _: &[Value],
) -> RtResult<Value> {
    let (principal, data_access) = current_data_access(context)?;
    Ok(Value::Boolean(
        data_access.user_password_strength_check(&principal, context)?,
    ))
}

fn user_password_numeric_setting(
    context: &mut CallContext<'_>,
    setting: UserPasswordNumericSetting,
) -> RtResult<Value> {
    let (principal, data_access) = current_data_access(context)?;
    Ok(Value::Number(data_access.user_password_numeric_setting(
        setting, &principal, context,
    )?))
}

pub(crate) fn user_password_max_effective_period(
    context: &mut CallContext<'_>,
    _: &[Value],
) -> RtResult<Value> {
    user_password_numeric_setting(context, UserPasswordNumericSetting::MaxEffectivePeriod)
}

pub(crate) fn user_password_min_effective_period(
    context: &mut CallContext<'_>,
    _: &[Value],
) -> RtResult<Value> {
    user_password_numeric_setting(context, UserPasswordNumericSetting::MinEffectivePeriod)
}

pub(crate) fn user_password_reuse_limit(
    context: &mut CallContext<'_>,
    _: &[Value],
) -> RtResult<Value> {
    user_password_numeric_setting(context, UserPasswordNumericSetting::ReuseLimit)
}

pub(crate) fn user_password_expiration_notification_period(
    context: &mut CallContext<'_>,
    _: &[Value],
) -> RtResult<Value> {
    user_password_numeric_setting(
        context,
        UserPasswordNumericSetting::ExpirationNotificationPeriod,
    )
}

fn set_user_password_setting(
    context: &mut CallContext<'_>,
    arguments: &[Value],
    setting: UserPasswordSetting,
) -> RtResult<Value> {
    let value = &arguments[0];
    let (principal, data_access) = current_data_access(context)?;
    data_access.set_user_password_setting(setting, value, &principal, context)?;
    Ok(Value::Undefined)
}

pub(crate) fn set_user_password_strength_check(
    context: &mut CallContext<'_>,
    arguments: &[Value],
) -> RtResult<Value> {
    set_user_password_setting(context, arguments, UserPasswordSetting::StrengthCheck)
}

pub(crate) fn set_user_password_minimum_length(
    context: &mut CallContext<'_>,
    arguments: &[Value],
) -> RtResult<Value> {
    set_user_password_setting(context, arguments, UserPasswordSetting::MinimumLength)
}

pub(crate) fn set_user_password_max_effective_period(
    context: &mut CallContext<'_>,
    arguments: &[Value],
) -> RtResult<Value> {
    set_user_password_setting(
        context,
        arguments,
        UserPasswordSetting::Numeric(UserPasswordNumericSetting::MaxEffectivePeriod),
    )
}

pub(crate) fn set_user_password_min_effective_period(
    context: &mut CallContext<'_>,
    arguments: &[Value],
) -> RtResult<Value> {
    set_user_password_setting(
        context,
        arguments,
        UserPasswordSetting::Numeric(UserPasswordNumericSetting::MinEffectivePeriod),
    )
}

pub(crate) fn set_user_password_reuse_limit(
    context: &mut CallContext<'_>,
    arguments: &[Value],
) -> RtResult<Value> {
    set_user_password_setting(
        context,
        arguments,
        UserPasswordSetting::Numeric(UserPasswordNumericSetting::ReuseLimit),
    )
}

pub(crate) fn set_user_password_expiration_notification_period(
    context: &mut CallContext<'_>,
    arguments: &[Value],
) -> RtResult<Value> {
    set_user_password_setting(
        context,
        arguments,
        UserPasswordSetting::Numeric(UserPasswordNumericSetting::ExpirationNotificationPeriod),
    )
}

pub(crate) fn authentication_lock_manager(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    Ok(Value::new_object(AuthenticationLockManager))
}

fn authentication_lock_settings_value(
    receiver: &dyn ObjectProtocol,
) -> RtResult<&AuthenticationLockSettingsValue> {
    receiver
        .downcast_ref::<AuthenticationLockSettingsValue>()
        .ok_or(RtError::NotAnObject)
}

fn get_authentication_lock_settings(
    receiver: &dyn ObjectProtocol,
    _: &[Value],
    context: &mut CallContext<'_>,
) -> RtResult<Value> {
    receiver
        .downcast_ref::<AuthenticationLockManager>()
        .ok_or(RtError::NotAnObject)?;
    let (principal, data_access) = current_data_access(context)?;
    let settings = data_access.authentication_lock_settings(&principal, context)?;
    Ok(Value::new_object(AuthenticationLockSettingsValue {
        lock_duration: RefCell::new(settings.lock_duration().clone()),
        max_unsuccessful_attempts_count: RefCell::new(
            settings.max_unsuccessful_attempts_count().clone(),
        ),
        user_name_addition_codes: RefCell::new(Arc::from(settings.user_name_addition_codes())),
    }))
}

fn set_authentication_lock_settings(
    receiver: &dyn ObjectProtocol,
    arguments: &[Value],
    context: &mut CallContext<'_>,
) -> RtResult<Value> {
    receiver
        .downcast_ref::<AuthenticationLockManager>()
        .ok_or(RtError::NotAnObject)?;
    let Some(settings) = arguments
        .first()
        .and_then(Value::object_ref)
        .and_then(|object| object.downcast_ref::<AuthenticationLockSettingsValue>())
    else {
        return Err(RtError::TypeError {
            expected: "НастройкиБлокировкиАутентификацииПользователейИнформационнойБазы",
            op: "УстановитьНастройки",
        });
    };
    let snapshot = AuthenticationLockSettingsSnapshot::new(
        settings.lock_duration.borrow().clone(),
        settings.max_unsuccessful_attempts_count.borrow().clone(),
        Arc::clone(&settings.user_name_addition_codes.borrow()),
    );
    let (principal, data_access) = current_data_access(context)?;
    data_access.set_authentication_lock_settings(&snapshot, &principal, context)?;
    Ok(Value::Undefined)
}

fn authentication_lock_number(value: Value, op: &'static str) -> RtResult<BslNumber> {
    match value {
        Value::Number(value) => Ok(value),
        Value::Str(value) => BslNumber::parse_canonical(&value.to_string()).map_err(RtError::Num),
        _ => Err(RtError::TypeError {
            expected: "Число",
            op,
        }),
    }
}

fn authentication_lock_string(value: Value, op: &'static str) -> RtResult<Arc<str>> {
    match value {
        Value::Str(value) => Ok(Arc::from(value.to_string())),
        Value::Number(value) => Ok(Arc::from(value.to_string())),
        _ => Err(RtError::TypeError {
            expected: "Строка",
            op,
        }),
    }
}

fn authentication_lock_duration(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    Ok(Value::Number(
        authentication_lock_settings_value(receiver)?
            .lock_duration
            .borrow()
            .clone(),
    ))
}

fn set_authentication_lock_duration(
    receiver: &dyn ObjectProtocol,
    value: Value,
    _: &mut CallContext<'_>,
) -> RtResult<()> {
    *authentication_lock_settings_value(receiver)?
        .lock_duration
        .borrow_mut() = authentication_lock_number(
        value,
        "НастройкиБлокировкиАутентификацииПользователейИнформационнойБазы.ДлительностьБлокировки",
    )?;
    Ok(())
}

fn authentication_lock_max_unsuccessful_attempts_count(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    Ok(Value::Number(
        authentication_lock_settings_value(receiver)?
            .max_unsuccessful_attempts_count
            .borrow()
            .clone(),
    ))
}

fn set_authentication_lock_max_unsuccessful_attempts_count(
    receiver: &dyn ObjectProtocol,
    value: Value,
    _: &mut CallContext<'_>,
) -> RtResult<()> {
    *authentication_lock_settings_value(receiver)?
        .max_unsuccessful_attempts_count
        .borrow_mut() = authentication_lock_number(
        value,
        "НастройкиБлокировкиАутентификацииПользователейИнформационнойБазы.МаксимальноеКоличествоНеуспешныхПопыток",
    )?;
    Ok(())
}

fn authentication_lock_user_name_addition_codes(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    Ok(Value::Str(
        authentication_lock_settings_value(receiver)?
            .user_name_addition_codes
            .borrow()
            .as_ref()
            .into(),
    ))
}

fn set_authentication_lock_user_name_addition_codes(
    receiver: &dyn ObjectProtocol,
    value: Value,
    _: &mut CallContext<'_>,
) -> RtResult<()> {
    *authentication_lock_settings_value(receiver)?
        .user_name_addition_codes
        .borrow_mut() = authentication_lock_string(
        value,
        "НастройкиБлокировкиАутентификацииПользователейИнформационнойБазы.КодыДополненияИмениПользователя",
    )?;
    Ok(())
}

pub(crate) fn user_name(context: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    Ok(Value::Str(session(context)?.principal().name().into()))
}

pub(crate) fn user_full_name(context: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    Ok(Value::Str(session(context)?.principal().full_name().into()))
}
