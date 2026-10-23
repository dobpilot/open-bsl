//! Измеренное самостоятельное значение элемента планировщика.

use std::cell::RefCell;

use open_bsl::{
    BslDate, CallContext, ObjectProtocol, PropertyDescriptor, RtError, RtResult, TypeDescriptor,
    Value, fixed_map_from_map, format_value,
};

pub(super) static TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-session",
    name: "ЭлементПланировщика",
    type_display: "Planner item",
    type_names: &["PlannerItem"],
};

#[derive(Debug)]
struct PlannerItem {
    values: RefCell<[Value; 8]>,
}

impl ObjectProtocol for PlannerItem {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &TYPE
    }

    fn property_table(&self) -> &'static [PropertyDescriptor] {
        PROPERTIES
    }
}

pub(super) fn construct(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    Ok(Value::new_object(PlannerItem {
        values: RefCell::new([
            Value::Undefined,
            Value::Date(BslDate::empty()),
            Value::Date(BslDate::empty()),
            Value::Str("".into()),
            Value::Str("".into()),
            Value::Undefined,
            Value::Undefined,
            fixed_map_from_map(&Value::new_map())?,
        ]),
    }))
}

fn item(receiver: &dyn ObjectProtocol) -> RtResult<&PlannerItem> {
    receiver.downcast_ref().ok_or(RtError::NotAnObject)
}

// Каждый обработчик связан с одним слотом таблицы; имена и приоритет
// обращения к свойству обрабатывает общий компонентный механизм.
macro_rules! getter {
    ($name:ident, $slot:expr) => {
        fn $name(receiver: &dyn ObjectProtocol, _: &mut CallContext<'_>) -> RtResult<Value> {
            Ok(item(receiver)?.values.borrow()[$slot].clone())
        }
    };
}
getter!(get_value, 0);
getter!(get_begin, 1);
getter!(get_end, 2);
getter!(get_text, 3);
getter!(get_tooltip, 4);
getter!(get_deleted, 5);
getter!(get_parent, 6);
getter!(get_dimensions, 7);

fn set_value(receiver: &dyn ObjectProtocol, value: Value, _: &mut CallContext<'_>) -> RtResult<()> {
    item(receiver)?.values.borrow_mut()[0] = value;
    Ok(())
}

fn text_value(value: Value) -> RtResult<Value> {
    match value {
        Value::Str(_) => Ok(value),
        Value::Number(_) => Ok(Value::Str(format_value(&value, None)?.as_str().into())),
        _ => Err(RtError::DynamicError(
            "это преобразование текста элемента планировщика ещё не реализовано".into(),
        )),
    }
}

fn date_value(value: Value) -> RtResult<Value> {
    match value {
        Value::Date(_) => Ok(value),
        Value::Number(_) => Ok(Value::Date(BslDate::empty())),
        _ => Err(RtError::DynamicError(
            "это преобразование даты элемента планировщика ещё не реализовано".into(),
        )),
    }
}

macro_rules! setter {
    ($name:ident, $slot:expr, $convert:ident) => {
        fn $name(
            receiver: &dyn ObjectProtocol,
            value: Value,
            _: &mut CallContext<'_>,
        ) -> RtResult<()> {
            let value = $convert(value)?;
            item(receiver)?.values.borrow_mut()[$slot] = value;
            Ok(())
        }
    };
}
setter!(set_begin, 1, date_value);
setter!(set_end, 2, date_value);
setter!(set_text, 3, text_value);
setter!(set_tooltip, 4, text_value);

fn set_deleted(
    receiver: &dyn ObjectProtocol,
    value: Value,
    _: &mut CallContext<'_>,
) -> RtResult<()> {
    item(receiver)?;
    match value {
        // Самостоятельный элемент не имеет замещаемого родителя; измеренное
        // присваивание истины оставляет признак удаления неопределённым.
        Value::Boolean(true) => Ok(()),
        _ => Err(RtError::DynamicError(
            "это преобразование признака удаления элемента планировщика ещё не реализовано".into(),
        )),
    }
}

pub(super) const PROPERTIES: &[PropertyDescriptor] = &[
    PropertyDescriptor {
        names: &["Значение", "Value"],
        get: get_value,
        set: Some(set_value),
    },
    PropertyDescriptor {
        names: &["Начало", "Begin"],
        get: get_begin,
        set: Some(set_begin),
    },
    PropertyDescriptor {
        names: &["Конец", "End"],
        get: get_end,
        set: Some(set_end),
    },
    PropertyDescriptor {
        names: &["Текст", "Text"],
        get: get_text,
        set: Some(set_text),
    },
    PropertyDescriptor {
        names: &["Подсказка", "ToolTip"],
        get: get_tooltip,
        set: Some(set_tooltip),
    },
    PropertyDescriptor {
        names: &["Удаленный", "Deleted"],
        get: get_deleted,
        set: Some(set_deleted),
    },
    PropertyDescriptor {
        names: &["Родитель", "Parent"],
        get: get_parent,
        set: None,
    },
    PropertyDescriptor {
        names: &["ЗначенияИзмерений", "DimensionValues"],
        get: get_dimensions,
        set: None,
    },
];
