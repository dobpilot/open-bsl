//! Проверка прав доступа: `ПравоДоступа`, `ПроверитьПраваДоступа`, `РольДоступна`.

use super::*;

fn access_right_metadata(
    context: &CallContext<'_>,
    metadata: &Value,
    operation: &'static str,
) -> RtResult<(
    Arc<MetadataCatalog>,
    MetadataObject,
    Option<DocumentAttribute>,
)> {
    let session = session(context)?;
    let invalid_metadata = || RtError::TypeError {
        expected: "ОбъектМетаданных",
        op: operation,
    };
    let object = metadata.object_ref().ok_or_else(invalid_metadata)?;
    let mut attribute = None;
    let metadata = if let Some(value) = object
        .downcast_ref::<MetadataValue>()
        .filter(|value| Arc::ptr_eq(&value.catalog, session.catalog()))
    {
        session
            .catalog()
            .materialized_object_by_id(&value.object_id)?
            .cloned()
            .ok_or_else(|| {
                RtError::DynamicError("объект метаданных отсутствует в каталоге".into())
            })?
    } else if let Some(value) = object
        .downcast_ref::<metadata_attributes::Attribute>()
        .filter(|value| Arc::ptr_eq(&value.catalog, session.catalog()))
    {
        attribute = Some(value.descriptor().clone());
        session
            .catalog()
            .materialized_object_by_id(&value.owner_id)?
            .cloned()
            .ok_or_else(|| {
                RtError::DynamicError("владелец реквизита отсутствует в каталоге".into())
            })?
    } else if let Some(root) = object
        .downcast_ref::<MetadataRoot>()
        .filter(|root| Arc::ptr_eq(&root.catalog, session.catalog()))
    {
        MetadataObject::new(
            root.catalog
                .configuration_object_id
                .as_deref()
                .ok_or_else(|| {
                    RtError::DynamicError(
                        "идентификатор корневого объекта конфигурации не приобретён".into(),
                    )
                })?,
            configuration_description(root)?.name.as_ref(),
            MetadataKind::Configuration,
        )
    } else {
        return Err(invalid_metadata());
    };
    Ok((Arc::clone(session.catalog()), metadata, attribute))
}

pub(crate) fn access_right(context: &mut CallContext<'_>, arguments: &[Value]) -> RtResult<Value> {
    let [right, metadata, tail @ ..] = arguments else {
        return Err(RtError::DynamicError(
            "ПравоДоступа ожидает от двух до четырёх аргументов".into(),
        ));
    };
    let Value::Str(right) = right else {
        return Err(RtError::TypeError {
            expected: "Строка",
            op: "ПравоДоступа",
        });
    };
    let right = right.to_string();
    if right.is_empty() {
        return Err(RtError::DynamicError("имя права пусто".into()));
    }

    let metadata_value = metadata.clone();
    let (catalog, metadata, attribute) = access_right_metadata(context, metadata, "ПравоДоступа")?;

    let subject = match tail.first() {
        None | Some(Value::Undefined) => AccessRightSubject::CurrentPrincipal,
        Some(value) => {
            let object = value.object_ref().ok_or_else(|| RtError::TypeError {
                expected: "ПользовательИнформационнойБазы или ОбъектМетаданныхРоль",
                op: "ПравоДоступа",
            })?;
            if let Some(user) = object.downcast_ref::<InfoBaseUserValue>() {
                AccessRightSubject::Principal(user.principal.clone())
            } else if let Some(role) = object
                .downcast_ref::<MetadataValue>()
                .filter(|value| Arc::ptr_eq(&value.catalog, &catalog))
                .and_then(|value| catalog.object_by_id(&value.object_id))
                .filter(|object| object.kind() == MetadataKind::Role)
            {
                AccessRightSubject::Role(Arc::from(role.id()))
            } else {
                return Err(RtError::TypeError {
                    expected: "ПользовательИнформационнойБазы или ОбъектМетаданныхРоль",
                    op: "ПравоДоступа",
                });
            }
        }
    };
    let standard_attribute = match tail.get(1) {
        None | Some(Value::Undefined) => None,
        Some(Value::Str(value)) => Some(Rc::from(value.to_string())),
        Some(_) => {
            return Err(RtError::TypeError {
                expected: "Строка",
                op: "ПравоДоступа",
            });
        }
    };
    let request = AccessRightRequest {
        right: Rc::from(right),
        metadata,
        metadata_value,
        attribute,
        subject,
        standard_attribute,
    };
    // Адаптер проверяет имя, путь и применимость до обхода выдачи прав.
    // Привилегированный кадр не делает неизвестное право допустимым;
    // явно переданные пользователь и роль сохраняют собственные права.
    let (principal, data_access) = current_data_access(context)?;
    let granted = data_access.access_right(&request, &principal, context)?;
    check_standard_path_after_adapter(&request, "ПравоДоступа")?;
    Ok(Value::Boolean(granted))
}

fn check_standard_path_after_adapter(
    request: &AccessRightRequest,
    operation: &'static str,
) -> RtResult<()> {
    // Сначала адаптер сохраняет приоритет неизвестного права. Успешный
    // пользовательский адаптер всё равно не делает пустой путь допустимым.
    if request.standard_attribute() == Some("") {
        return Err(RtError::TypeError {
            expected: "непустая Строка",
            op: operation,
        });
    }
    Ok(())
}

pub(crate) fn verify_access_rights(
    context: &mut CallContext<'_>,
    arguments: &[Value],
) -> RtResult<Value> {
    let [right, metadata, tail @ ..] = arguments else {
        return Err(RtError::DynamicError(
            "ВыполнитьПроверкуПравДоступа ожидает два или три аргумента".into(),
        ));
    };
    let Value::Str(right) = right else {
        return Err(RtError::TypeError {
            expected: "Строка",
            op: "ВыполнитьПроверкуПравДоступа",
        });
    };
    if right.to_string().is_empty() {
        return Err(RtError::DynamicError("имя права пусто".into()));
    }
    let metadata_value = metadata.clone();
    let (_, metadata, attribute) =
        access_right_metadata(context, metadata, "ВыполнитьПроверкуПравДоступа")?;
    let standard_attribute = match tail.first() {
        None | Some(Value::Undefined) => None,
        Some(Value::Str(value)) => Some(Rc::from(value.to_string())),
        Some(_) => {
            return Err(RtError::TypeError {
                expected: "Строка",
                op: "ВыполнитьПроверкуПравДоступа",
            });
        }
    };
    let request = AccessRightRequest {
        right: Rc::from(right.to_string()),
        metadata,
        metadata_value,
        attribute,
        subject: AccessRightSubject::CurrentPrincipal,
        standard_attribute,
    };
    let (principal, data_access) = current_data_access(context)?;
    data_access.verify_access_rights(&request, &principal, context)?;
    check_standard_path_after_adapter(&request, "ВыполнитьПроверкуПравДоступа")?;
    Ok(Value::Undefined)
}

pub(crate) fn role_available(ctx: &mut CallContext<'_>, arguments: &[Value]) -> RtResult<Value> {
    let [argument] = arguments else {
        return Err(RtError::DynamicError(
            "РольДоступна ожидает один аргумент".into(),
        ));
    };
    let session = session(ctx)?;
    let role = match argument {
        Value::Str(name) => {
            let name = name.to_string();
            session.catalog.object(MetadataKind::Role, &name)
        }
        value => value
            .object_ref()
            .and_then(|value| value.downcast_ref::<MetadataValue>())
            .and_then(|value| value.catalog.object_by_id(&value.object_id))
            .filter(|object| object.kind == MetadataKind::Role),
    }
    .ok_or_else(|| RtError::DynamicError("роль конфигурации не найдена".into()))?;
    Ok(Value::Boolean(session.principal.has_role(role.id())))
}
