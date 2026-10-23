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

pub(crate) static NUMBER_QUALIFIERS_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "КвалификаторыЧисла",
    type_display: "Number qualifiers",
    type_names: &["NumberQualifiers"],
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

#[derive(Debug)]
struct NumberQualifiersObject {
    digits: BslNumber,
    fraction_digits: BslNumber,
    allowed_sign: EnumValue,
}

impl ObjectProtocol for NumberQualifiersObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &NUMBER_QUALIFIERS_TYPE
    }

    fn property_table(&self) -> &'static [PropertyDescriptor] {
        NUMBER_QUALIFIERS_PROPERTIES
    }

    fn get_property(&self, name: &str, context: &mut CallContext<'_>) -> RtResult<BslValue> {
        crate::get_property_from_table(
            NUMBER_QUALIFIERS_PROPERTIES,
            "КвалификаторыЧисла",
            self,
            name,
            context,
        )
    }
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

fn get_number_digits(
    receiver: &dyn ObjectProtocol,
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let qualifiers = receiver_of::<NumberQualifiersObject>(receiver, "Разрядность")?;
    Ok(BslValue::Number(qualifiers.digits.clone()))
}

fn get_number_fraction_digits(
    receiver: &dyn ObjectProtocol,
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let qualifiers = receiver_of::<NumberQualifiersObject>(receiver, "РазрядностьДробнойЧасти")?;
    Ok(BslValue::Number(qualifiers.fraction_digits.clone()))
}

fn get_number_allowed_sign(
    receiver: &dyn ObjectProtocol,
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let qualifiers = receiver_of::<NumberQualifiersObject>(receiver, "ДопустимыйЗнак")?;
    Ok(BslValue::Enum(qualifiers.allowed_sign))
}

static NUMBER_QUALIFIERS_PROPERTIES: &[PropertyDescriptor] = &[
    PropertyDescriptor {
        names: &["Разрядность", "Digits"],
        get: get_number_digits,
        set: None,
    },
    PropertyDescriptor {
        names: &["РазрядностьДробнойЧасти", "FractionDigits"],
        get: get_number_fraction_digits,
        set: None,
    },
    PropertyDescriptor {
        names: &["ДопустимыйЗнак", "AllowedSign"],
        get: get_number_allowed_sign,
        set: None,
    },
];

pub(crate) const API_MEMBERS: &[ObjectMembersDescriptor] = &[
    ObjectMembersDescriptor::new(&STRING_QUALIFIERS_TYPE)
        .with_properties(STRING_QUALIFIERS_PROPERTIES),
    ObjectMembersDescriptor::new(&NUMBER_QUALIFIERS_TYPE)
        .with_properties(NUMBER_QUALIFIERS_PROPERTIES),
];

fn normalized_count(value: Option<&BslValue>) -> RtResult<BslNumber> {
    match value {
        None => Ok(BslNumber::from_i64(0)),
        Some(BslValue::Number(number)) => {
            let truncated = number.trunc_to_scale(0);
            let zero = BslNumber::from_i64(0);
            Ok(if truncated.cmp(&zero).is_lt() {
                zero
            } else {
                truncated
            })
        }
        Some(_) => Err(RtError::TypeError {
            expected: "Число",
            op: "Новый КвалификаторыЧисла",
        }),
    }
}

pub(crate) fn qualifier_property(object: &BslObject, name: &str) -> RtResult<BslValue> {
    let (target, number) = if crate::folded_eq(name, "КвалификаторыСтроки")
        || crate::folded_eq(name, "StringQualifiers")
    {
        (TypeId::String, false)
    } else if crate::folded_eq(name, "КвалификаторыЧисла")
        || crate::folded_eq(name, "NumberQualifiers")
    {
        (TypeId::Number, true)
    } else {
        return Err(RtError::UnknownColumn(name.to_owned()));
    };
    let qualifiers = match object {
        BslObject::QualifiedTypeDescription(types) => types
            .iter()
            .find(|value| value.id == TypeRef::Native(target))
            .map(|value| value.quals.as_slice())
            .unwrap_or_default(),
        BslObject::TypeDescription(_) => &[],
        _ => return Err(RtError::NotAnObject),
    };
    let count = |index: usize| -> RtResult<BslNumber> {
        match qualifiers.get(index) {
            None => Ok(BslNumber::from_i64(0)),
            Some(text) => BslNumber::parse_canonical(text.as_str()).map_err(|_| {
                RtError::DynamicError("неподдерживаемый квалификатор описания типов".into())
            }),
        }
    };
    // Нативная проба `type-description-qualifiers` подтверждает нулевые
    // квалификаторы отсутствующего типа и политики «Переменная» / «Любой».
    // Замер: `docs/research/fixtures/type-description-qualifiers/measure.platform.txt`.
    let policy_index = if number { 2 } else { 1 };
    if qualifiers
        .get(policy_index)
        .is_some_and(|value| value != "0" && (number || value != "1"))
    {
        return Err(RtError::DynamicError(
            "эта политика квалификатора описания типов пока не поддерживается".into(),
        ));
    }
    if number {
        Ok(BslValue::new_object(NumberQualifiersObject {
            digits: count(0)?,
            fraction_digits: count(1)?,
            allowed_sign: EnumValue::AllowedSignAny,
        }))
    } else {
        Ok(BslValue::new_object(StringQualifiersObject {
            length: count(0)?,
            allowed_length: if qualifiers.get(1).is_some_and(|value| value == "0") {
                EnumValue::AllowedLengthFixed
            } else {
                EnumValue::AllowedLengthVariable
            },
        }))
    }
}

pub(crate) fn construct_number_qualifiers(arguments: &[BslValue]) -> RtResult<BslValue> {
    let digits = normalized_count(arguments.first())?;
    let fraction_digits = normalized_count(arguments.get(1))?;
    let fraction_digits = if fraction_digits.cmp(&digits).is_gt() {
        digits.clone()
    } else {
        fraction_digits
    };
    let allowed_sign = match arguments.get(2) {
        None => EnumValue::AllowedSignAny,
        Some(BslValue::Enum(value)) if value.kind() == EnumKind::AllowedSign => *value,
        Some(_) => {
            return Err(RtError::TypeError {
                expected: "ДопустимыйЗнак",
                op: "Новый КвалификаторыЧисла",
            });
        }
    };
    Ok(BslValue::new_object(NumberQualifiersObject {
        digits,
        fraction_digits,
        allowed_sign,
    }))
}

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

pub(crate) fn is_supported_qualifier(value: &BslValue) -> bool {
    matches!(value, BslValue::Undefined)
        || is_date_time_qualifier(value)
        || value
            .object_ref()
            .and_then(|object| object.downcast_ref::<StringQualifiersObject>())
            .is_some()
        || value
            .object_ref()
            .and_then(|object| object.downcast_ref::<NumberQualifiersObject>())
            .is_some()
}

pub(crate) fn retain_primitive_qualifiers(
    description: BslValue,
    qualifiers: &[BslValue],
) -> RtResult<BslValue> {
    let BslValue::Object(object) = &description else {
        return Err(RtError::NotAnObject);
    };
    let BslObject::TypeDescription(types) = object.as_ref() else {
        return Err(RtError::NotAnObject);
    };
    let mut columns = types
        .iter()
        .cloned()
        .map(crate::table::ColumnType::plain)
        .collect::<Vec<_>>();
    for column in &mut columns {
        if column.id == TypeRef::Native(TypeId::String) {
            if let Some(qualifier) = qualifiers
                .get(1)
                .and_then(BslValue::object_ref)
                .and_then(|object| object.downcast_ref::<StringQualifiersObject>())
            {
                column.quals = vec![
                    qualifier.length.to_string(),
                    if qualifier.allowed_length == EnumValue::AllowedLengthFixed {
                        "0"
                    } else {
                        "1"
                    }
                    .into(),
                ];
            }
        } else if column.id == TypeRef::Native(TypeId::Number)
            && let Some(qualifier) = qualifiers
                .first()
                .and_then(BslValue::object_ref)
                .and_then(|object| object.downcast_ref::<NumberQualifiersObject>())
        {
            if qualifier.allowed_sign != EnumValue::AllowedSignAny {
                return Err(RtError::DynamicError(
                    "эта политика квалификатора числа пока не поддерживается".into(),
                ));
            }
            column.quals = vec![
                qualifier.digits.to_string(),
                qualifier.fraction_digits.to_string(),
                "0".into(),
            ];
        }
    }
    if columns.iter().all(|column| column.quals.is_empty()) {
        Ok(description)
    } else {
        Ok(BslValue::new_qualified_type_description(columns))
    }
}

/// `ОписаниеТипов.ПривестиЗначение` для двух форм Connector.
///
/// Платформа 8.3.27 измерена файлом
/// `measure-type-description-convert.bsl`: негодная строка даёт
/// значение по умолчанию (`0` либо пустую дату), а не ошибку.
pub(crate) fn adjust_value(receiver: &BslValue, value: Option<&BslValue>) -> RtResult<BslValue> {
    let BslValue::Object(object) = receiver else {
        return not_applicable(receiver);
    };
    if let BslObject::QualifiedTypeDescription(types) = object.as_ref() {
        // Составное описание характеристики и квалификация измерены
        // независимой пробой `characteristic-value-type-qualifiers`.
        return Ok(crate::table::adjust_to_types(
            value.cloned().unwrap_or(BslValue::Undefined),
            types,
        ));
    }
    let BslObject::TypeDescription(types) = &**object else {
        return not_applicable(receiver);
    };
    let Some(value) = value else {
        return match types.as_slice() {
            [TypeRef::Native(TypeId::Number)] => Ok(BslValue::Number(BslNumber::from_i64(0))),
            [TypeRef::Native(TypeId::String)] => Ok(BslValue::Str(crate::BslString::from_str(""))),
            [TypeRef::Native(TypeId::Boolean)] => Ok(BslValue::Boolean(false)),
            [TypeRef::Native(TypeId::Date)] => Ok(BslValue::Date(BslDate::empty())),
            [] | [_, _, ..] => Ok(BslValue::Undefined),
            [_] => Err(RtError::DynamicError(
                "значение по умолчанию этого типа не реализовано".into(),
            )),
        };
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
