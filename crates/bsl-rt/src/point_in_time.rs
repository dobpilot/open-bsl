//! Неизменяемый момент времени с отдельной идентичностью ссылки хоста.

use std::cmp::Ordering;

use crate::{
    Arity, BslDate, BslValue, CallContext, MethodDescriptor, ObjectMembersDescriptor,
    ObjectProtocol, ObjectRef, PropertyDescriptor, RtError, RtResult, TypeDescriptor, receiver_of,
};

/// Ключ сравнения ссылки, предоставленный её владельцем.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReferenceOrderKey {
    /// Приобретённый номер вида ссылки в хранилище; `None` означает неизвестность.
    pub storage_number: Option<u32>,
    /// UUID в каноническом порядке, до преобразования в физические байты 1С.
    pub uuid: [u8; 16],
}

/// Описание общего типа момента времени.
pub static POINT_IN_TIME_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "МоментВремени",
    type_display: "Момент времени",
    type_names: &["PointInTime"],
};

/// Неизменяемый снимок даты и ссылки объекта информационной базы.
#[derive(Debug)]
pub struct PointInTimeValue {
    date: BslDate,
    reference: BslValue,
    key: Option<ReferenceOrderKey>,
}

impl PointInTimeValue {
    /// Возвращает дату снимка без округления.
    #[must_use]
    pub const fn date(&self) -> BslDate {
        self.date
    }

    /// Возвращает ссылку либо `Неопределено`, если ссылка не задана.
    #[must_use]
    pub fn reference(&self) -> &BslValue {
        &self.reference
    }

    fn compare(&self, other: &Self) -> RtResult<Ordering> {
        let date = self.date.cmp(&other.date);
        if date != Ordering::Equal {
            return Ok(date);
        }
        match (self.key, other.key) {
            (None, None) => Ok(Ordering::Equal),
            (None, Some(_)) => Ok(Ordering::Less),
            (Some(_), None) => Ok(Ordering::Greater),
            (Some(first), Some(second)) => {
                if self.reference.type_of()? != other.reference.type_of()? {
                    let (Some(first_type), Some(second_type)) =
                        (first.storage_number, second.storage_number)
                    else {
                        return Err(RtError::DynamicError(
                            "порядок видов ссылок момента времени неизвестен".into(),
                        ));
                    };
                    let kind = first_type.cmp(&second_type);
                    if kind != Ordering::Equal {
                        return Ok(kind);
                    }
                    return Err(RtError::DynamicError(
                        "разные виды ссылок имеют одинаковый номер хранения".into(),
                    ));
                }
                Ok(
                    crate::uuid::to_1c_bytes(first.uuid)
                        .cmp(&crate::uuid::to_1c_bytes(second.uuid)),
                )
            }
        }
    }
}

/// Создаёт момент с датой и ссылкой, которая предоставляет ключ сравнения.
///
/// # Errors
/// Возвращает ошибку, если значение не является поддержанной ссылкой хоста.
pub fn new_point_in_time(date: BslDate, reference: BslValue) -> RtResult<BslValue> {
    let key = if matches!(reference, BslValue::Undefined) {
        None
    } else {
        Some(
            reference
                .object_ref()
                .and_then(|object| object.as_dyn().reference_order_key())
                .ok_or_else(|| {
                    RtError::DynamicError(
                        "ключ ссылки для момента времени не поддерживается".into(),
                    )
                })?,
        )
    };
    Ok(BslValue::new_object(PointInTimeValue {
        date,
        reference,
        key,
    }))
}

impl ObjectProtocol for PointInTimeValue {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &POINT_IN_TIME_TYPE
    }
    fn method_table(&self) -> &'static [MethodDescriptor] {
        METHODS
    }
    fn property_table(&self) -> &'static [PropertyDescriptor] {
        PROPERTIES
    }
    fn value_eq(&self, other: &ObjectRef) -> Option<bool> {
        Some(
            other
                .downcast_ref::<Self>()
                .is_some_and(|other| self.date == other.date && self.reference == other.reference),
        )
    }
    fn display(&self) -> String {
        format!("{}; {}", BslValue::Date(self.date), self.reference)
    }
    fn presentation_with_context(&self, context: &CallContext<'_>) -> RtResult<String> {
        Ok(format!(
            "{}; {}",
            context.format_value(&BslValue::Date(self.date), None)?,
            if matches!(self.reference, BslValue::Undefined) {
                String::new()
            } else {
                context.format_value(&self.reference, None)?
            }
        ))
    }
}

pub(crate) fn construct(_: &mut CallContext<'_>, args: &[BslValue]) -> RtResult<BslValue> {
    let date = match &args[0] {
        BslValue::Date(date) => *date,
        BslValue::Str(text) => BslDate::parse_digits(&text.to_string())
            .ok_or_else(|| RtError::DynamicError("некорректная дата момента времени".into()))?,
        _ => {
            return Err(RtError::TypeError {
                expected: "Дата",
                op: "МоментВремени",
            });
        }
    };
    new_point_in_time(date, args.get(1).cloned().unwrap_or(BslValue::Undefined))
}

fn compare(
    receiver: &dyn ObjectProtocol,
    args: &[BslValue],
    _: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let first = receiver_of::<PointInTimeValue>(receiver, "Сравнить")?;
    let second = args[0]
        .object_ref()
        .and_then(|value| value.downcast_ref::<PointInTimeValue>())
        .ok_or(RtError::TypeError {
            expected: "МоментВремени",
            op: "Сравнить",
        })?;
    Ok(BslValue::number_from_i64(match first.compare(second)? {
        Ordering::Less => -1,
        Ordering::Equal => 0,
        Ordering::Greater => 1,
    }))
}

fn date(receiver: &dyn ObjectProtocol, _: &mut CallContext<'_>) -> RtResult<BslValue> {
    Ok(BslValue::Date(
        receiver_of::<PointInTimeValue>(receiver, "Дата")?.date,
    ))
}
fn reference(receiver: &dyn ObjectProtocol, _: &mut CallContext<'_>) -> RtResult<BslValue> {
    Ok(receiver_of::<PointInTimeValue>(receiver, "Ссылка")?
        .reference
        .clone())
}

const METHODS: &[MethodDescriptor] = &[MethodDescriptor::new(
    &["Сравнить", "Compare"],
    Arity::exact(1),
    compare,
)];
const PROPERTIES: &[PropertyDescriptor] = &[
    PropertyDescriptor {
        names: &["Дата", "Date"],
        get: date,
        set: None,
    },
    PropertyDescriptor {
        names: &["Ссылка", "Ref"],
        get: reference,
        set: None,
    },
];
pub(crate) const API_MEMBERS: &[ObjectMembersDescriptor] =
    &[ObjectMembersDescriptor::new(&POINT_IN_TIME_TYPE)
        .with_methods(METHODS)
        .with_properties(PROPERTIES)];
