//! Узкий контракт `ОписаниеТипов`, нужный Connector 2.6.0.

use crate::{
    BslDate, BslNumber, BslObject, BslValue, EnumValue, ObjectProtocol, RtError, RtResult,
    TypeDescriptor, TypeId, TypeRef,
};

pub(crate) static DATE_QUALIFIERS_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "КвалификаторыДаты",
    type_display: "Date qualifiers",
    type_names: &["DateQualifiers"],
};

#[derive(Debug)]
pub(crate) struct DateQualifiersObject {
    fractions: EnumValue,
}

impl ObjectProtocol for DateQualifiersObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &DATE_QUALIFIERS_TYPE
    }
}

pub(crate) fn new_date_qualifiers(argument: &BslValue) -> RtResult<BslValue> {
    let BslValue::Enum(EnumValue::DateFractionsDateTime) = argument else {
        return Err(RtError::TypeError {
            expected: "ЧастиДаты.ДатаВремя",
            op: "Новый КвалификаторыДаты",
        });
    };
    Ok(BslValue::new_object(DateQualifiersObject {
        fractions: EnumValue::DateFractionsDateTime,
    }))
}

pub(crate) fn is_date_time_qualifier(value: &BslValue) -> bool {
    value
        .object_ref()
        .and_then(|object| object.downcast_ref::<DateQualifiersObject>())
        .is_some_and(|qualifier| qualifier.fractions == EnumValue::DateFractionsDateTime)
}

/// `ОписаниеТипов.ПривестиЗначение` для двух форм Connector.
///
/// Платформа 8.3.27 измерена файлом
/// `measure-type-description-convert.bsl`: негодная строка даёт
/// значение по умолчанию (`0` либо пустую дату), а не ошибку.
pub(crate) fn adjust_value(receiver: &BslValue, value: &BslValue) -> RtResult<BslValue> {
    let BslValue::Object(object) = receiver else {
        return not_applicable(receiver);
    };
    let BslObject::TypeDescription(types) = &**object else {
        return not_applicable(receiver);
    };
    let [TypeRef::Native(target)] = types.as_slice() else {
        return Err(RtError::TypeError {
            expected: "ОписаниеТипов из одного примитивного типа",
            op: "ПривестиЗначение",
        });
    };

    match (target, value) {
        (TypeId::Number, BslValue::Number(_)) | (TypeId::Date, BslValue::Date(_)) => {
            Ok(value.clone())
        }
        (TypeId::Number, BslValue::Str(text)) => Ok(BslValue::Number(
            BslNumber::parse_canonical(&text.to_string())
                .unwrap_or_else(|_| BslNumber::from_i64(0)),
        )),
        (TypeId::Date, BslValue::Str(text)) => Ok(BslValue::Date(
            parse_date_digits(&text.to_string()).unwrap_or_else(BslDate::empty),
        )),
        (TypeId::Number, _) => Ok(BslValue::Number(BslNumber::from_i64(0))),
        (TypeId::Date, _) => Ok(BslValue::Date(BslDate::empty())),
        _ => Err(RtError::TypeError {
            expected: "ОписаниеТипов Число или Дата",
            op: "ПривестиЗначение",
        }),
    }
}

fn parse_date_digits(text: &str) -> Option<BslDate> {
    if text.len() != 14 || !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let part = |from: usize, to: usize| text[from..to].parse::<i64>().ok();
    BslDate::from_civil(
        part(0, 4)?,
        u32::try_from(part(4, 6)?).ok()?,
        u32::try_from(part(6, 8)?).ok()?,
        u32::try_from(part(8, 10)?).ok()?,
        u32::try_from(part(10, 12)?).ok()?,
        u32::try_from(part(12, 14)?).ok()?,
    )
}

fn not_applicable(receiver: &BslValue) -> RtResult<BslValue> {
    Err(RtError::MethodNotApplicable {
        method: "ПривестиЗначение",
        receiver: receiver.type_name(),
    })
}
