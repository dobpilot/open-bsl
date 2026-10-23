//! Безопасный и привилегированный режимы, безопасный режим разделения
//! данных и описание защиты от опасных действий.

use super::*;

#[derive(Debug)]
struct UnsafeOperationProtectionDescriptionObject {
    warnings: Cell<bool>,
}

pub(crate) static UNSAFE_OPERATION_PROTECTION_DESCRIPTION_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-session",
    name: "ОписаниеЗащитыОтОпасныхДействий",
    type_display: "Unsafe operation protection description",
    type_names: &["UnsafeOperationProtectionDescription"],
};

impl ObjectProtocol for UnsafeOperationProtectionDescriptionObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &UNSAFE_OPERATION_PROTECTION_DESCRIPTION_TYPE
    }

    fn property_table(&self) -> &'static [PropertyDescriptor] {
        UNSAFE_OPERATION_PROTECTION_DESCRIPTION_PROPERTIES
    }
}

fn security_mode_flag(arguments: &[Value], operation: &'static str) -> RtResult<bool> {
    match &arguments[0] {
        Value::Boolean(value) => Ok(*value),
        Value::Str(_) => Err(RtError::DynamicError(format!(
            "{operation}: именованные профили безопасности не поддерживаются"
        ))),
        _ => Err(RtError::TypeError {
            expected: "Булево",
            op: operation,
        }),
    }
}

pub(crate) fn set_safe_mode(ctx: &mut CallContext<'_>, arguments: &[Value]) -> RtResult<Value> {
    ctx.set_safe_mode(security_mode_flag(arguments, "УстановитьБезопасныйРежим")?)?;
    Ok(Value::Undefined)
}

pub(crate) fn safe_mode(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    Ok(Value::Boolean(ctx.safe_mode()?))
}

pub(crate) fn set_safe_mode_disabled(
    ctx: &mut CallContext<'_>,
    arguments: &[Value],
) -> RtResult<Value> {
    ctx.set_safe_mode_disabled(security_mode_flag(
        arguments,
        "УстановитьОтключениеБезопасногоРежима",
    )?)?;
    Ok(Value::Undefined)
}

pub(crate) fn get_safe_mode_disabled(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    Ok(Value::Boolean(ctx.safe_mode_disabled()?))
}

pub(crate) fn set_privileged_mode(
    ctx: &mut CallContext<'_>,
    arguments: &[Value],
) -> RtResult<Value> {
    ctx.set_privileged_mode(security_mode_flag(
        arguments,
        "УстановитьПривилегированныйРежим",
    )?)?;
    Ok(Value::Undefined)
}

pub(crate) fn privileged_mode(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    Ok(Value::Boolean(ctx.privileged_mode()?))
}

pub(crate) fn set_data_separation_safe_mode(
    ctx: &mut CallContext<'_>,
    arguments: &[Value],
) -> RtResult<Value> {
    let Value::Str(common_attribute) = &arguments[0] else {
        return Err(RtError::TypeError {
            expected: "Строка",
            op: "УстановитьБезопасныйРежимРазделенияДанных",
        });
    };
    // Флаг использует измеренное обычное преобразование в Булево;
    // ошибка преобразования предшествует поиску имени и изменению счётчика.
    let enabled = arguments[1]
        .as_condition()
        .map_err(|_| RtError::TypeError {
            expected: "Булево",
            op: "УстановитьБезопасныйРежимРазделенияДанных",
        })?;
    let common_attribute =
        checked_data_separation_common_attribute(ctx, common_attribute.shared_utf8())?;
    // Счётчики кадров, вложенная видимость и очистка измерены на конфигурации
    // с разделяющим общим реквизитом; наблюдение хоста не подменяет геттер зоны.
    ctx.set_data_separation_safe_mode(&common_attribute, enabled)?;
    Ok(Value::Undefined)
}

fn checked_data_separation_common_attribute(
    ctx: &CallContext<'_>,
    common_attribute: Rc<str>,
) -> RtResult<Rc<str>> {
    let runtime = session_data(ctx)?
        .extension::<SessionRuntime>()
        .ok_or_else(|| RtError::DynamicError("runtime сеанса не подключён".into()))?;
    match runtime.is_data_separation_common_attribute(&common_attribute) {
        Some(true) => {}
        Some(false) => {
            return Err(RtError::DynamicError(format!(
                "общий реквизит разделения данных «{common_attribute}» не найден"
            )));
        }
        None => {
            return Err(RtError::DynamicError(
                "общие реквизиты разделения данных не переданы хостом".into(),
            ));
        }
    }
    Ok(common_attribute)
}

pub(crate) fn data_separation_safe_mode(
    ctx: &mut CallContext<'_>,
    arguments: &[Value],
) -> RtResult<Value> {
    let Value::Str(common_attribute) = &arguments[0] else {
        return Err(RtError::TypeError {
            expected: "Строка",
            op: "БезопасныйРежимРазделенияДанных",
        });
    };
    checked_data_separation_common_attribute(ctx, common_attribute.shared_utf8())?;
    // Счётчик VM не включает исходный безопасный режим из `default.vrd`.
    // Без сведений от хоста нельзя выдавать его за ответ платформы.
    Err(RtError::DynamicError(
        "безопасный режим разделения данных зоны не передан хостом".into(),
    ))
}

pub(crate) fn construct_unsafe_operation_protection_description(
    _: &mut CallContext<'_>,
    _: &[Value],
) -> RtResult<Value> {
    Ok(Value::new_object(
        UnsafeOperationProtectionDescriptionObject {
            warnings: Cell::new(true),
        },
    ))
}

fn unsafe_operation_protection_description(
    receiver: &dyn ObjectProtocol,
) -> RtResult<&UnsafeOperationProtectionDescriptionObject> {
    receiver
        .downcast_ref::<UnsafeOperationProtectionDescriptionObject>()
        .ok_or(RtError::NotAnObject)
}

fn unsafe_operation_warnings(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    Ok(Value::Boolean(
        unsafe_operation_protection_description(receiver)?
            .warnings
            .get(),
    ))
}

fn set_unsafe_operation_warnings(
    receiver: &dyn ObjectProtocol,
    value: Value,
    _: &mut CallContext<'_>,
) -> RtResult<()> {
    let Value::Boolean(value) = value else {
        return Err(RtError::TypeError {
            expected: "Булево",
            op: "ОписаниеЗащитыОтОпасныхДействий.ПредупреждатьОбОпасныхДействиях",
        });
    };
    unsafe_operation_protection_description(receiver)?
        .warnings
        .set(value);
    Ok(())
}

pub(crate) static UNSAFE_OPERATION_PROTECTION_DESCRIPTION_PROPERTIES: &[PropertyDescriptor] =
    &[PropertyDescriptor {
        names: &["ПредупреждатьОбОпасныхДействиях", "UnsafeOperationWarnings"],
        get: unsafe_operation_warnings,
        set: Some(set_unsafe_operation_warnings),
    }];
