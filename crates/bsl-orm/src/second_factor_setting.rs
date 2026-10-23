//! Измеренный объект настройки второго фактора, не записывающий пользователя.

use super::*;

#[derive(Debug)]
struct Setting {
    name: RefCell<Value>,
    parameters: RefCell<Value>,
}

pub(crate) static TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-orm",
    name: "НастройкаВторогоФактораАутентификации",
    type_display: "НастройкаВторогоФактораАутентификации",
    type_names: &["SecondAuthenticationFactorSetting"],
};

impl ObjectProtocol for Setting {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &TYPE
    }

    fn property_table(&self) -> &'static [PropertyDescriptor] {
        PROPERTIES
    }
}

pub(crate) fn construct(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    Ok(Value::new_object(Setting {
        name: RefCell::new(Value::Str("".into())),
        parameters: RefCell::new(Value::new_map()),
    }))
}

fn setting(receiver: &dyn ObjectProtocol) -> RtResult<&Setting> {
    receiver.downcast_ref::<Setting>().ok_or_else(|| {
        RtError::DynamicError("ожидалась настройка второго фактора аутентификации".into())
    })
}

fn name_get(receiver: &dyn ObjectProtocol, _: &mut CallContext<'_>) -> RtResult<Value> {
    Ok(setting(receiver)?.name.borrow().clone())
}

fn name_set(
    receiver: &dyn ObjectProtocol,
    value: Value,
    context: &mut CallContext<'_>,
) -> RtResult<()> {
    let value = match value {
        Value::Str(_) => value,
        Value::Number(_) => Value::Str(context.format_value(&value, None)?.as_str().into()),
        _ => {
            return Err(RtError::DynamicError(
                "преобразование этого значения в имя шаблона 2ФА пока не реализовано".into(),
            ));
        }
    };
    *setting(receiver)?.name.borrow_mut() = value;
    Ok(())
}

fn parameters_get(receiver: &dyn ObjectProtocol, _: &mut CallContext<'_>) -> RtResult<Value> {
    Ok(setting(receiver)?.parameters.borrow().clone())
}

fn parameters_set(
    receiver: &dyn ObjectProtocol,
    value: Value,
    _: &mut CallContext<'_>,
) -> RtResult<()> {
    let target = setting(receiver)?;
    if matches!(&value, Value::Object(object) if matches!(object.as_ref(), BslObject::Map(_))) {
        *target.parameters.borrow_mut() = value;
        return Ok(());
    }
    if matches!(&value, Value::Number(_))
        || matches!(&value, Value::Object(object) if matches!(object.as_ref(), BslObject::Structure(_)))
    {
        // ИЗМЕРЕНО: отказ преобразования очищает свойство; само прежнее
        // соответствие остаётся неизменным и доступным по другим ссылкам.
        *target.parameters.borrow_mut() = Value::Undefined;
        return Err(RtError::TypeError {
            expected: "Соответствие",
            op: "НастройкаВторогоФактораАутентификации.Параметры",
        });
    }
    Err(RtError::DynamicError(
        "эта форма параметров настройки 2ФА пока не реализована".into(),
    ))
}

pub(crate) const PROPERTIES: &[PropertyDescriptor] = &[
    PropertyDescriptor {
        names: &["ИмяШаблонаНастройки", "SettingTemplateName"],
        get: name_get,
        set: Some(name_set),
    },
    PropertyDescriptor {
        names: &["Параметры", "Parameters"],
        get: parameters_get,
        set: Some(parameters_set),
    },
];

#[cfg(test)]
#[path = "second_factor_setting_tests.rs"]
mod tests;
