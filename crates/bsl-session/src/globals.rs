//! Прочие глобальные функции контекста сеанса.

use super::*;

pub(crate) static COLLABORATION_CONVERSATION_ID_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-session",
    name: "ИдентификаторОбсужденияСистемыВзаимодействия",
    type_display: "CollaborationSystemConversationID",
    type_names: &["CollaborationSystemConversationID"],
};

pub(crate) fn current_run_mode(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    match runtime(ctx)?.kind() {
        SessionKind::WebClient | SessionKind::ThinClient => {
            Ok(Value::Enum(EnumValue::ClientRunModeManagedApplication))
        }
        // Измерено на платформе 8.3.27 настоящим фоновым заданием:
        // `measure-session-current-run-mode.bsl`.
        SessionKind::BackgroundJob => Ok(Value::Undefined),
    }
}

pub(crate) fn computer_name(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    let name = runtime(ctx)?
        .computer_name()
        .ok_or_else(|| RtError::DynamicError("сетевое имя компьютера не передано хостом".into()))?;
    Ok(Value::Str(name.into()))
}

pub(crate) fn current_locale_code(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    let code = runtime(ctx)?
        .locale_code()
        .ok_or_else(|| RtError::DynamicError("код локализации не передан хостом".into()))?;
    Ok(Value::Str(code.into()))
}

pub(crate) fn infobase_locale_code(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    let code = runtime(ctx)?.infobase_locale_code().ok_or_else(|| {
        RtError::DynamicError("код локализации информационной базы не передан хостом".into())
    })?;
    Ok(Value::Str(code.into()))
}

pub(crate) fn current_system_language(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    let language = runtime(ctx)?
        .system_language()
        .ok_or_else(|| RtError::DynamicError("язык системы не передан хостом".into()))?;
    Ok(Value::Str(language.into()))
}

pub(crate) fn get_external_resources_mode(
    ctx: &mut CallContext<'_>,
    _: &[Value],
) -> RtResult<Value> {
    Ok(Value::Str(runtime(ctx)?.external_resources_mode().into()))
}

pub(crate) fn collaboration_system(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    Err(RtError::DynamicError(
        "система взаимодействия не предоставлена хостом".into(),
    ))
}

pub(crate) fn construct_collaboration_filter(
    _: &mut CallContext<'_>,
    _: &[Value],
) -> RtResult<Value> {
    Err(RtError::DynamicError(
        "отбор системы взаимодействия не поддерживается этим хостом".into(),
    ))
}

pub(crate) fn construct_collaboration_conversation_id(
    _: &mut CallContext<'_>,
    _: &[Value],
) -> RtResult<Value> {
    // Идентификатор нельзя подменять строкой: это отдельный тип платформы.
    Err(RtError::DynamicError(
        "идентификатор обсуждения системы взаимодействия не поддерживается этим хостом".into(),
    ))
}

pub(crate) fn construct_collaboration_conversation_context(
    _: &mut CallContext<'_>,
    _: &[Value],
) -> RtResult<Value> {
    Err(RtError::DynamicError(
        "контекст обсуждения системы взаимодействия не поддерживается этим хостом".into(),
    ))
}

pub(crate) fn construct_collaboration_application(
    _: &mut CallContext<'_>,
    _: &[Value],
) -> RtResult<Value> {
    // Идентификаторы и коллекции приложений — типизированные значения
    // системы взаимодействия; без её сервиса их нельзя заменять строкой.
    Err(RtError::DynamicError(
        "объекты приложений системы взаимодействия не поддерживаются этим хостом".into(),
    ))
}

pub(crate) fn construct_server_administration(
    _: &mut CallContext<'_>,
    _: &[Value],
) -> RtResult<Value> {
    // На платформе конструктор сразу подключается к ras. Декларация
    // позволяет связать исходный модуль, но не подменяет сетевое соединение.
    Err(RtError::DynamicError(
        "транспорт администрирования сервера ras не подключён".into(),
    ))
}

pub(crate) fn construct_internet_mail_message(
    _: &mut CallContext<'_>,
    _: &[Value],
) -> RtResult<Value> {
    Err(RtError::DynamicError(
        "интернет-почтовое сообщение не поддерживается этим хостом".into(),
    ))
}

pub(crate) fn deliverable_notification_send(
    _: &mut CallContext<'_>,
    _: &[Value],
) -> RtResult<Value> {
    // Отправка уведомлений требует отдельного транспорта; пустой менеджер
    // скрыл бы отсутствие сервиса текущего сеанса.
    Err(RtError::DynamicError(
        "отправка доставляемых уведомлений не предоставлена текущему сеансу".into(),
    ))
}

pub(crate) fn crypto_tools(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    // Без поставщика криптографии нельзя подменять сведения о его модуле
    // пустым менеджером или данными другого сеанса.
    Err(RtError::DynamicError(
        "средства криптографии не предоставлены текущему сеансу".into(),
    ))
}

pub(crate) fn get_infobase_time_zone(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    // Пояс информационной базы — отдельная настройка, которую нельзя
    // заменять часовым поясом компьютера или текущего сеанса.
    Err(RtError::DynamicError(
        "часовой пояс информационной базы не предоставлен текущему сеансу".into(),
    ))
}

pub(crate) fn find_marked_for_deletion(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    // Поиск обязан учитывать права чтения, RLS и разделение данных текущего
    // сеанса; пустой массив выдал бы отсутствие доступа за отсутствие данных.
    Err(RtError::DynamicError(
        "поиск помеченных на удаление объектов не поддерживается текущим сеансом".into(),
    ))
}

pub(crate) fn access_parameters(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    // Результат зависит от полей объекта, прав пользователя или роли и RLS.
    // Пустая структура ошибочно означала бы отсутствие ограничений.
    Err(RtError::DynamicError(
        "параметры доступа для текущего сеанса пока не поддерживаются".into(),
    ))
}

pub(crate) fn available_locale_codes(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    // Список локалей платформы не выводится из локали текущего сеанса.
    Err(RtError::DynamicError(
        "доступные коды локализации не предоставлены текущему сеансу".into(),
    ))
}

pub(crate) fn full_text_search(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    // Индекс, права и разделение данных принадлежат текущему сеансу.
    // Пустой менеджер сообщил бы, что разрешённый поиск ничего не нашёл.
    Err(RtError::DynamicError(
        "полнотекстовый поиск не предоставлен текущему сеансу".into(),
    ))
}

pub(crate) fn additional_authentication_settings(
    _: &mut CallContext<'_>,
    _: &[Value],
) -> RtResult<Value> {
    // Настройки аутентификации принадлежат информационной базе и доступны
    // только с её разрешениями. Пустой менеджер скрыл бы отсутствие данных.
    Err(RtError::DynamicError(
        "дополнительные настройки аутентификации не предоставлены текущему сеансу".into(),
    ))
}

pub(crate) fn get_server_path_separator(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    let separator = ctx
        .files()?
        .path_separator()
        .map_err(|error| RtError::IoError(error.to_string()))?;
    Ok(Value::Str(separator.as_str().into()))
}

pub(crate) fn get_licensing_client_name(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    // Имя хранится в информационной базе и доступно только администратору;
    // значение процесса или пустая строка подменили бы проверку прав.
    Err(RtError::DynamicError(
        "имя клиента лицензирования не предоставлено текущему сеансу".into(),
    ))
}

pub(crate) fn get_configuration_id(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    // Платформенный идентификатор зашифрован; GUID метаданных и отпечаток
    // ресурсов не могут заменить его для проверки лицензии.
    Err(RtError::DynamicError(
        "идентификатор конфигурации не предоставлен текущему сеансу".into(),
    ))
}

pub(crate) fn get_licensing_client_additional_parameter(
    _: &mut CallContext<'_>,
    _: &[Value],
) -> RtResult<Value> {
    // Дополнительный параметр хранится в базе и доступен администратору.
    Err(RtError::DynamicError(
        "дополнительный параметр клиента лицензирования не предоставлен текущему сеансу".into(),
    ))
}

pub(crate) fn set_licensing_client_parameters(
    _: &mut CallContext<'_>,
    _: &[Value],
) -> RtResult<Value> {
    // Успех без записи в базу ложно подтвердил бы смену учётных данных.
    Err(RtError::DynamicError(
        "запись настроек клиента лицензирования не предоставлена текущему сеансу".into(),
    ))
}

pub(crate) fn locale_code_presentation(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    // Представление зависит от платформенного каталога локалей.
    Err(RtError::DynamicError(
        "представление кода локализации не предоставлено текущему сеансу".into(),
    ))
}

pub(crate) fn refresh_reusable_values(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    // У интерпретатора пока нет повторно используемых возвращаемых значений:
    // каждый вызов функции исполняется. Очистка пустого кэша сохраняет
    // наблюдаемое состояние платформенной процедуры и успешно завершается.
    Ok(Value::Undefined)
}

pub(crate) fn attach_add_in(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    Err(RtError::DynamicError(
        "загрузчик внешних компонент не передан хостом".into(),
    ))
}

pub(crate) fn database_configuration_changed_dynamically(
    ctx: &mut CallContext<'_>,
    _: &[Value],
) -> RtResult<Value> {
    runtime(ctx)?
        .database_configuration_changed_dynamically()
        .map(Value::Boolean)
        .ok_or_else(|| {
            RtError::DynamicError(
                "признак динамического обновления конфигурации базы данных не передан хостом"
                    .into(),
            )
        })
}
