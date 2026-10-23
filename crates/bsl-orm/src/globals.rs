//! Прочие глобальные функции контекста конфигурации.

use super::*;

pub(crate) fn configuration_changed(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    let changed = session(ctx)?
        .catalog()
        .configuration_changed
        .ok_or_else(|| {
            RtError::DynamicError(
                "признак изменения конфигурации не предоставлен источником метаданных".into(),
            )
        })?;
    Ok(Value::Boolean(changed))
}

pub(crate) fn standard_odata_interface_content(
    _: &mut CallContext<'_>,
    _: &[Value],
) -> RtResult<Value> {
    // Состав OData хранится в информационной базе и не равен всему каталогу
    // метаданных; без чтения настройки нельзя возвращать массив объектов.
    Err(RtError::DynamicError(
        "состав стандартного интерфейса OData не предоставлен этим хостом".into(),
    ))
}

pub(crate) fn set_standard_odata_interface_content(
    _: &mut CallContext<'_>,
    _: &[Value],
) -> RtResult<Value> {
    // Эта процедура меняет настройку информационной базы. Без адаптера
    // записи нельзя объявлять успех и оставлять прежний состав OData.
    Err(RtError::DynamicError(
        "запись состава стандартного интерфейса OData не предоставлена этим хостом".into(),
    ))
}

pub(crate) fn current_language(context: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    let session = session(context)?;
    let id = session
        .principal()
        .interface_language_id()
        .or_else(|| session.catalog().primary_language_id())
        .ok_or_else(|| RtError::DynamicError("текущий язык не предоставлен сеансом".into()))?;
    let language = session
        .catalog()
        .materialized_object_by_id(id)?
        .filter(|object| object.kind() == MetadataKind::Language)
        .ok_or_else(|| {
            RtError::DynamicError("текущий язык отсутствует в каталоге метаданных".into())
        })?;
    Ok(metadata_object_value(session.catalog(), language))
}

pub(crate) fn predefined_value(
    context: &mut CallContext<'_>,
    arguments: &[Value],
) -> RtResult<Value> {
    let [Value::Str(path)] = arguments else {
        return Err(RtError::TypeError {
            expected: "Строка",
            op: "ПредопределенноеЗначение",
        });
    };
    let path = path.to_string();
    let parts = path.split('.').collect::<Vec<_>>();
    if let [kind, owner_name, member] = parts.as_slice()
        && (folded_eq(kind, "Справочник")
            || folded_eq(kind, "Catalog")
            || folded_eq(kind, "ПланВидовХарактеристик")
            || folded_eq(kind, "ChartOfCharacteristicTypes"))
    {
        let owner_kind = if folded_eq(kind, "ПланВидовХарактеристик")
            || folded_eq(kind, "ChartOfCharacteristicTypes")
        {
            MetadataKind::ChartOfCharacteristicTypes
        } else {
            MetadataKind::Catalog
        };
        let catalog = Arc::clone(session(context)?.catalog());
        if let Some(owner) = catalog.object(owner_kind, owner_name)
            && owner
                .catalog_predefined_names()
                .iter()
                .any(|known| folded_eq(known, member))
        {
            return ManagerValue {
                object_id: Arc::clone(&owner.id),
                catalog,
            }
            .get_property(member, context);
        }
    }
    if let [kind, member] = parts.as_slice()
        && let Some(value) = lookup_enum(kind).and_then(|kind| lookup_member(kind, member))
    {
        return Ok(Value::Enum(value));
    }
    if let [kind, owner_name, member] = parts.as_slice()
        && (folded_eq(kind, "Перечисление")
            || folded_eq(kind, "Enum")
            || folded_eq(kind, "Enumeration"))
    {
        let catalog = Arc::clone(session(context)?.catalog());
        if let Some(owner) = catalog.object(MetadataKind::Enumeration, owner_name)
            && let Some(value) = catalog.enumeration_value(owner.id(), member)
        {
            return enumeration_reference_value(
                Arc::clone(&catalog),
                &value.owner_id,
                &value.value_id,
            );
        }
    }
    Err(RtError::DynamicError(format!(
        "предопределённое значение `{path}` не найдено"
    )))
}

pub(crate) fn predefined_value_full_name(
    context: &mut CallContext<'_>,
    arguments: &[Value],
) -> RtResult<Value> {
    let [value] = arguments else {
        unreachable!("реестр проверяет арность функции");
    };
    if let Value::Enum(value) = value {
        // Язык имени задаёт конфигурация текущего сеанса, а не написание
        // вызова или язык интерфейса. Каталог без описания сохраняет прежний путь.
        let russian = context
            .host_data()
            .ok()
            .and_then(|data| data.downcast_ref::<SessionData>())
            .and_then(|data| data.catalog.configuration.as_ref())
            .is_some_and(|description| description.script_variant == ScriptVariant::Russian);
        let name = if russian {
            format!("{}.{}", value.kind().ru_name(), value.member_name())
        } else {
            format!("{}.{}", value.kind().english_name(), value.english_name())
        };
        return Ok(Value::Str(name.as_str().into()));
    }
    if let Some(reference) = value
        .object_ref()
        .and_then(|object| object.downcast_ref::<EnumerationReferenceValue>())
        && let Some(owner) = reference
            .catalog
            .materialized_object_by_id(&reference.object_id)?
    {
        // НЕ ИЗМЕРЕНО(PREDEFINED.FULL_NAME.CONFIG_ENUM): каталог без
        // приобретённого варианта встроенного языка сохраняет русский путь.
        // Для приобретённой конфигурации оба варианта и пустая ссылка измерены.
        let prefix = match reference
            .catalog
            .configuration
            .as_ref()
            .map(|description| description.script_variant)
        {
            Some(ScriptVariant::English) => "Enum",
            _ => "Перечисление",
        };
        let member = if reference.value_id.is_empty() {
            if prefix == "Enum" {
                "EmptyRef"
            } else {
                "ПустаяСсылка"
            }
        } else {
            reference
                .catalog
                .enumeration_values
                .iter()
                .find(|member| {
                    member.owner_id == reference.object_id && member.value_id == reference.value_id
                })
                .map(|member| member.name.as_ref())
                .ok_or_else(|| {
                    RtError::DynamicError(
                        "полное имя этого предопределённого значения недоступно".into(),
                    )
                })?
        };
        let name = format!("{prefix}.{}.{member}", owner.name);
        return Ok(Value::Str(name.as_str().into()));
    }
    Err(RtError::DynamicError(
        "полное имя этого предопределённого значения недоступно".into(),
    ))
}

pub(crate) fn get_choice_data(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    // Данные выбора зависят от метаданных и прав текущего пользователя сеанса.
    Err(RtError::DynamicError(
        "ПолучитьДанныеВыбора не поддерживается текущим адаптером данных".into(),
    ))
}

pub(crate) fn navigation_link(
    context: &mut CallContext<'_>,
    arguments: &[Value],
) -> RtResult<Value> {
    let (principal, data_access) = current_data_access(context)?;
    let link = data_access.navigation_link(arguments, &principal, context)?;
    Ok(Value::Str(link.as_str().into()))
}

pub(crate) fn common_template(
    context: &mut CallContext<'_>,
    arguments: &[Value],
) -> RtResult<Value> {
    if let Ok(session) = session(context) {
        let catalog = Arc::clone(session.catalog());
        if let Some(value) = file_source::template(&catalog, None, arguments, context)? {
            return Ok(value);
        }
    }
    // Код конфигурации может ссылаться на макет даже без его выполнения.
    // Пока ресурсов макетов нет, выдавать пустой документ было бы ошибкой.
    Err(RtError::DynamicError(
        "получение общего макета пока не поддерживается".into(),
    ))
}

pub(crate) fn external_data_processors(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    // Глобальное имя разрешается при сборке модуля, а загрузка внешних
    // обработок требует отдельного источника кода текущего сеанса.
    Err(RtError::DynamicError(
        "внешние обработки не подключены к текущему сеансу".into(),
    ))
}

pub(crate) fn external_reports(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    Err(RtError::DynamicError(
        "внешние отчёты не подключены к текущему сеансу".into(),
    ))
}

pub(crate) fn data_history(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    // История версий требует службы текущего сеанса, включая операции записи.
    // Подставлять пустой менеджер или доступ к SQL напрямую нельзя.
    Err(RtError::DynamicError(
        "история данных не подключена к текущему сеансу".into(),
    ))
}

pub(crate) fn second_authentication_factor_settings_templates(
    _: &mut CallContext<'_>,
    _: &[Value],
) -> RtResult<Value> {
    // Это платформенная служба управления настройками 2ФА, а не общий
    // модуль конфигурации. Без службы текущего пользователя нельзя
    // подставлять пустые шаблоны или имитировать успешное изменение.
    Err(RtError::DynamicError(
        "шаблоны настроек второго фактора аутентификации не подключены к текущему сеансу".into(),
    ))
}

pub(crate) fn functional_option(
    context: &mut CallContext<'_>,
    arguments: &[Value],
) -> RtResult<Value> {
    let [Value::Str(name), tail @ ..] = arguments else {
        return Err(RtError::TypeError {
            expected: "Строка, Структура",
            op: "ПолучитьФункциональнуюОпцию",
        });
    };
    let parameters = match tail {
        [] | [Value::Undefined] => Vec::new(),
        [Value::Object(object)] => {
            let BslObject::Structure(storage) = object.as_ref() else {
                return Err(RtError::TypeError {
                    expected: "Структура",
                    op: "ПолучитьФункциональнуюОпцию",
                });
            };
            let storage = storage.borrow();
            (0..storage.len())
                .map(|index| {
                    let (name, value) = storage.entry_at(index).ok_or_else(|| {
                        RtError::DynamicError(
                            "структура параметров функциональной опции повреждена".into(),
                        )
                    })?;
                    let name = context.runtime_shapes().names.name(name).ok_or_else(|| {
                        RtError::DynamicError(
                            "имя параметра функциональной опции отсутствует".into(),
                        )
                    })?;
                    Ok((Rc::from(name), value))
                })
                .collect::<RtResult<Vec<_>>>()?
        }
        _ => {
            return Err(RtError::TypeError {
                expected: "Структура",
                op: "ПолучитьФункциональнуюОпцию",
            });
        }
    };
    let request = FunctionalOptionRequest {
        name: Rc::from(name.to_string()),
        parameters: parameters.into(),
    };
    let (principal, data_access) = current_data_access(context)?;
    data_access.functional_option(&request, &principal, context)
}
