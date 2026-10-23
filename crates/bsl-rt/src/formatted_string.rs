//! Неизменяемое значение форматированной строки.

use crate::{BslObject, BslValue, CallContext, ObjectProtocol, ObjectRef, RtError, RtResult};

pub(crate) static FORMATTED_STRING_TYPE: crate::TypeDescriptor = crate::TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ФорматированнаяСтрока",
    type_display: "Форматированная строка",
    type_names: &["FormattedString"],
};

#[derive(Debug)]
struct FormattedStringValue {
    arguments: Vec<FormattedArgument>,
    plain: String,
}

// Массив содержимого снимается при создании: последующая замена его
// элементов не меняет ни текст, ни равенство форматированной строки.
// Измерено пробой `measure-formatted-string-array-isolation.bsl` на 8.3.27.
#[derive(Debug, PartialEq)]
enum FormattedArgument {
    Value(BslValue),
    Array(Vec<FormattedArgument>),
}

impl FormattedArgument {
    fn snapshot(value: &BslValue) -> Self {
        if let BslValue::Object(object) = value
            && let BslObject::Array(items) = object.as_ref()
        {
            return Self::Array(items.borrow().iter().map(Self::snapshot).collect());
        }
        Self::Value(value.clone())
    }
}

impl ObjectProtocol for FormattedStringValue {
    fn type_descriptor(&self) -> &'static crate::TypeDescriptor {
        &FORMATTED_STRING_TYPE
    }

    fn is_filled(&self) -> RtResult<bool> {
        Ok(!self.plain.is_empty())
    }

    fn value_eq(&self, other: &ObjectRef) -> Option<bool> {
        Some(
            other
                .downcast_ref::<Self>()
                .is_some_and(|other| self.arguments == other.arguments),
        )
    }

    fn display(&self) -> String {
        self.plain.clone()
    }
}

fn append_content(value: &BslValue, plain: &mut String) -> RtResult<()> {
    match value {
        BslValue::Str(text) => plain.push_str(&text.to_string()),
        BslValue::Object(object) => match &**object {
            BslObject::Array(items) => {
                for item in items.borrow().iter() {
                    append_content(item, plain)?;
                }
            }
            BslObject::Extension(object) => {
                if let Some(formatted) = object.downcast_ref::<FormattedStringValue>() {
                    plain.push_str(&formatted.plain);
                } else if object.type_descriptor().name == "Картинка" {
                    // ИЗМЕРЕНО: картинка в форматированной строке даёт один
                    // пробел при преобразовании результата в обычную строку.
                    plain.push(' ');
                } else {
                    return Err(bad_content());
                }
            }
            _ => return Err(bad_content()),
        },
        _ => return Err(bad_content()),
    }
    Ok(())
}

fn bad_content() -> RtError {
    RtError::TypeError {
        expected: "Строка, ФорматированнаяСтрока, Картинка или Массив",
        op: "Новый ФорматированнаяСтрока",
    }
}

fn formatting_form(arguments: &[BslValue]) -> bool {
    (2..=5).contains(&arguments.len())
        && arguments[1..].iter().any(|value| !is_content_value(value))
}

/// Возвращает, может ли значение быть самостоятельным элементом содержимого.
fn is_content_value(value: &BslValue) -> bool {
    match value {
        BslValue::Str(_) => true,
        BslValue::Object(object) => match &**object {
            BslObject::Array(_) => true,
            BslObject::Extension(object) => {
                object.downcast_ref::<FormattedStringValue>().is_some()
                    || object.type_descriptor().name == "Картинка"
            }
            _ => false,
        },
        _ => false,
    }
}

pub(crate) fn construct_formatted_string(
    _: &mut CallContext<'_>,
    arguments: &[BslValue],
) -> RtResult<BslValue> {
    let Some(first) = arguments.first() else {
        return Err(RtError::DynamicError(
            "Новый ФорматированнаяСтрока требует содержимое".into(),
        ));
    };
    let mut plain = String::new();
    if formatting_form(arguments) {
        append_content(first, &mut plain)?;
    } else {
        for value in arguments {
            append_content(value, &mut plain)?;
        }
    }
    Ok(BslValue::new_object(FormattedStringValue {
        arguments: arguments.iter().map(FormattedArgument::snapshot).collect(),
        plain,
    }))
}

#[cfg(test)]
#[path = "formatted_string_tests.rs"]
mod tests;
