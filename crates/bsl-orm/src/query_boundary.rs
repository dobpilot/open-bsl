//! Неизменяемая граница: значение обёртки не определяет допустимость параметра запроса.

use super::*;

#[derive(Debug)]
struct Boundary {
    value: Value,
    kind: EnumValue,
}

pub(crate) static TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-orm",
    name: "Граница",
    type_display: "Bound",
    type_names: &["Boundary"],
};

impl ObjectProtocol for Boundary {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &TYPE
    }

    fn property_table(&self) -> &'static [PropertyDescriptor] {
        PROPERTIES
    }

    fn display(&self) -> String {
        "Boundary".into()
    }

    fn value_eq(&self, other: &open_bsl::ObjectRef) -> Option<bool> {
        Some(
            other
                .downcast_ref::<Self>()
                .is_some_and(|other| self.kind == other.kind && self.value == other.value),
        )
    }
}

pub(crate) fn construct(_: &mut CallContext<'_>, args: &[Value]) -> RtResult<Value> {
    let kind = match args.get(1) {
        None => EnumValue::BoundaryTypeIncluding,
        Some(Value::Enum(
            kind @ (EnumValue::BoundaryTypeIncluding | EnumValue::BoundaryTypeExcluding),
        )) => *kind,
        _ => {
            return Err(RtError::TypeError {
                expected: "ВидГраницы",
                op: "Граница: параметр 2",
            });
        }
    };
    let value = &args[0];
    // Измерены скалярные значения и момент времени. Другие объекты не
    // допускаются моделью до отдельного измерения их поведения в обёртке.
    let supported = matches!(
        value,
        Value::Date(_)
            | Value::Number(_)
            | Value::Boolean(_)
            | Value::Str(_)
            | Value::Null
            | Value::Undefined
    ) || matches!(value, Value::Object(object) if matches!(object.as_ref(), BslObject::Extension(component) if component.downcast_ref::<open_bsl::PointInTimeValue>().is_some()));
    if !supported {
        return Err(RtError::DynamicError(
            "это значение границы пока не поддерживается".into(),
        ));
    }
    Ok(Value::new_object(Boundary {
        value: value.clone(),
        kind,
    }))
}

/// Возвращает исходное значение и вид границы ORM для адаптера данных.
/// Для значений других типов возвращает `None`.
pub fn query_boundary_parts(value: &Value) -> Option<(&Value, EnumValue)> {
    let Value::Object(object) = value else {
        return None;
    };
    let BslObject::Extension(component) = object.as_ref() else {
        return None;
    };
    let boundary = component.downcast_ref::<Boundary>()?;
    Some((&boundary.value, boundary.kind))
}

fn boundary(receiver: &dyn ObjectProtocol) -> RtResult<&Boundary> {
    receiver
        .downcast_ref::<Boundary>()
        .ok_or_else(|| RtError::DynamicError("ожидалась граница".into()))
}

fn value_get(receiver: &dyn ObjectProtocol, _: &mut CallContext<'_>) -> RtResult<Value> {
    Ok(boundary(receiver)?.value.clone())
}

fn kind_get(receiver: &dyn ObjectProtocol, _: &mut CallContext<'_>) -> RtResult<Value> {
    Ok(Value::Enum(boundary(receiver)?.kind))
}

pub(crate) const PROPERTIES: &[PropertyDescriptor] = &[
    PropertyDescriptor {
        names: &["Значение", "Value"],
        get: value_get,
        set: None,
    },
    PropertyDescriptor {
        names: &["ВидГраницы", "BoundaryType"],
        get: kind_get,
        set: None,
    },
];
