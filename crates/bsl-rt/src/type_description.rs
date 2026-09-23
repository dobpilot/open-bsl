//! Узкий контракт `ОписаниеТипов`, нужный Connector 2.6.0.

use crate::{
    BslDate, BslNumber, BslObject, BslValue, CallContext, EnumKind, EnumValue,
    ObjectMembersDescriptor, ObjectProtocol, PropertyDescriptor, RtError, RtResult, TypeDescriptor,
    TypeId, TypeRef, receiver_of,
};

pub(crate) static DATE_QUALIFIERS_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "КвалификаторыДаты",
    type_display: "Date qualifiers",
    type_names: &["DateQualifiers"],
};

pub(crate) static STRING_QUALIFIERS_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "КвалификаторыСтроки",
    type_display: "String qualifiers",
    type_names: &["StringQualifiers"],
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

#[derive(Debug)]
struct StringQualifiersObject {
    length: BslNumber,
    allowed_length: EnumValue,
}

impl ObjectProtocol for StringQualifiersObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &STRING_QUALIFIERS_TYPE
    }

    fn property_table(&self) -> &'static [PropertyDescriptor] {
        STRING_QUALIFIERS_PROPERTIES
    }

    fn get_property(&self, name: &str, context: &mut CallContext<'_>) -> RtResult<BslValue> {
        crate::get_property_from_table(
            STRING_QUALIFIERS_PROPERTIES,
            "КвалификаторыСтроки",
            self,
            name,
            context,
        )
    }
}

fn get_string_length(
    receiver: &dyn ObjectProtocol,
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let qualifiers = receiver_of::<StringQualifiersObject>(receiver, "Длина")?;
    Ok(BslValue::Number(qualifiers.length.clone()))
}

fn get_allowed_length(
    receiver: &dyn ObjectProtocol,
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let qualifiers = receiver_of::<StringQualifiersObject>(receiver, "ДопустимаяДлина")?;
    Ok(BslValue::Enum(qualifiers.allowed_length))
}

static STRING_QUALIFIERS_PROPERTIES: &[PropertyDescriptor] = &[
    PropertyDescriptor {
        names: &["Длина", "Length"],
        get: get_string_length,
        set: None,
    },
    PropertyDescriptor {
        names: &["ДопустимаяДлина", "AllowedLength"],
        get: get_allowed_length,
        set: None,
    },
];

pub(crate) const API_MEMBERS: &[ObjectMembersDescriptor] =
    &[ObjectMembersDescriptor::new(&STRING_QUALIFIERS_TYPE)
        .with_properties(STRING_QUALIFIERS_PROPERTIES)];

pub(crate) fn construct_string_qualifiers(arguments: &[BslValue]) -> RtResult<BslValue> {
    let length = match arguments.first() {
        None => BslNumber::from_i64(0),
        Some(BslValue::Number(number)) => {
            let truncated = number.trunc_to_scale(0);
            let zero = BslNumber::from_i64(0);
            if truncated.cmp(&zero).is_lt() {
                zero
            } else {
                truncated
            }
        }
        Some(_) => {
            return Err(RtError::TypeError {
                expected: "Число",
                op: "Новый КвалификаторыСтроки",
            });
        }
    };
    let allowed_length = match arguments.get(1) {
        None => EnumValue::AllowedLengthVariable,
        Some(BslValue::Enum(value)) if value.kind() == EnumKind::AllowedLength => *value,
        Some(_) => {
            return Err(RtError::TypeError {
                expected: "ДопустимаяДлина",
                op: "Новый КвалификаторыСтроки",
            });
        }
    };
    Ok(BslValue::new_object(StringQualifiersObject {
        length,
        allowed_length,
    }))
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
