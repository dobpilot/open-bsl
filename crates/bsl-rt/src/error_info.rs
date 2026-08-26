//! Неизменяемый снимок информации об ошибке одного BSL-исполнения.

use crate::{BslString, BslValue, ObjectProtocol, RtError, RtResult, TypeDescriptor};

pub(crate) static ERROR_INFO_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ИнформацияОбОшибке",
    type_display: "Error information",
    type_names: &["ErrorInfo"],
};

#[derive(Debug)]
struct ErrorInfoObject {
    detail: BslString,
}

impl ObjectProtocol for ErrorInfoObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &ERROR_INFO_TYPE
    }
}

/// Создаёт снимок уже очищенного пользовательского текста ошибки.
#[must_use]
pub fn new_error_info(detail: impl Into<BslString>) -> BslValue {
    BslValue::new_object(ErrorInfoObject {
        detail: detail.into(),
    })
}

/// Возвращает подробное представление сохранённой ошибки.
///
/// # Errors
///
/// [`RtError::TypeError`], если передано не `ИнформацияОбОшибке`.
pub fn detailed_error_description(value: &BslValue) -> RtResult<BslValue> {
    let error = value
        .object_ref()
        .and_then(|object| object.downcast_ref::<ErrorInfoObject>())
        .ok_or(RtError::TypeError {
            expected: "ИнформацияОбОшибке",
            op: "ПодробноеПредставлениеОшибки",
        })?;
    Ok(BslValue::Str(error.detail.clone()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detail_is_an_immutable_snapshot() {
        let value = new_error_info(BslString::from_str("boom"));
        assert_eq!(
            detailed_error_description(&value).unwrap(),
            BslValue::Str(BslString::from_str("boom"))
        );
    }

    #[test]
    fn detail_rejects_another_value_type() {
        assert!(matches!(
            detailed_error_description(&BslValue::Undefined),
            Err(RtError::TypeError {
                expected: "ИнформацияОбОшибке",
                op: "ПодробноеПредставлениеОшибки"
            })
        ));
    }
}
