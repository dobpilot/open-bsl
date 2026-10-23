//! Изменяемые флаги почтового сообщения без привязки к транспорту IMAP.

use std::cell::Cell;

use crate::{
    BslValue, CallContext, ObjectMembersDescriptor, ObjectProtocol, PropertyDescriptor, RtError,
    RtResult, TypeDescriptor, receiver_of,
};

pub(crate) static TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ФлагиИнтернетПочтовогоСообщения",
    type_display: "ФлагиИнтернетПочтовогоСообщения",
    type_names: &["InternetMailMessageFlags"],
};

#[derive(Debug)]
struct InternetMailMessageFlags {
    // `Новое` платформа даёт только читать; остальные шесть флагов
    // изначально не заданы, а не равны `Ложь`.
    writable: [Cell<Option<bool>>; 6],
}

impl ObjectProtocol for InternetMailMessageFlags {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &TYPE
    }

    fn property_table(&self) -> &'static [PropertyDescriptor] {
        PROPERTIES
    }
}

fn get_new(receiver: &dyn ObjectProtocol, _ctx: &mut CallContext<'_>) -> RtResult<BslValue> {
    receiver_of::<InternetMailMessageFlags>(receiver, "Новое")?;
    Ok(BslValue::Undefined)
}

macro_rules! flag_property {
    ($get:ident, $set:ident, $index:expr, $name:literal) => {
        fn $get(receiver: &dyn ObjectProtocol, _ctx: &mut CallContext<'_>) -> RtResult<BslValue> {
            let flags = receiver_of::<InternetMailMessageFlags>(receiver, $name)?;
            Ok(match flags.writable[$index].get() {
                Some(value) => BslValue::Boolean(value),
                None => BslValue::Undefined,
            })
        }

        fn $set(
            receiver: &dyn ObjectProtocol,
            value: BslValue,
            _ctx: &mut CallContext<'_>,
        ) -> RtResult<()> {
            let flags = receiver_of::<InternetMailMessageFlags>(receiver, $name)?;
            let value = match value {
                BslValue::Boolean(value) => Some(value),
                BslValue::Undefined => None,
                _ => {
                    // НЕ ИЗМЕРЕНО(MAIL_FLAGS.OTHER_ASSIGNMENTS): кроме числа и строки
                    // остальные неподходящие типы пока не проверены на платформе.
                    return Err(RtError::TypeError {
                        expected: "Булево или Неопределено",
                        op: $name,
                    });
                }
            };
            flags.writable[$index].set(value);
            Ok(())
        }
    };
}

flag_property!(get_answered, set_answered, 0, "Отвеченное");
flag_property!(get_canceled, set_canceled, 1, "Отмененное");
flag_property!(get_forwarded, set_forwarded, 2, "Пересланное");
flag_property!(get_flagged, set_flagged, 3, "Помеченное");
flag_property!(get_seen, set_seen, 4, "Прочитанное");
flag_property!(get_deleted, set_deleted, 5, "Удаленное");

static PROPERTIES: &[PropertyDescriptor] = &[
    PropertyDescriptor {
        names: &["Новое", "New"],
        get: get_new,
        set: None,
    },
    PropertyDescriptor {
        names: &["Отвеченное", "Answered"],
        get: get_answered,
        set: Some(set_answered),
    },
    PropertyDescriptor {
        names: &["Отмененное", "Canceled"],
        get: get_canceled,
        set: Some(set_canceled),
    },
    PropertyDescriptor {
        names: &["Пересланное", "Forwarded"],
        get: get_forwarded,
        set: Some(set_forwarded),
    },
    PropertyDescriptor {
        names: &["Помеченное", "Flagged"],
        get: get_flagged,
        set: Some(set_flagged),
    },
    PropertyDescriptor {
        names: &["Прочитанное", "Seen"],
        get: get_seen,
        set: Some(set_seen),
    },
    PropertyDescriptor {
        names: &["Удаленное", "Deleted"],
        get: get_deleted,
        set: Some(set_deleted),
    },
];

pub(crate) const API_MEMBERS: &[ObjectMembersDescriptor] =
    &[ObjectMembersDescriptor::new(&TYPE).with_properties(PROPERTIES)];

/// Создаёт набор, в котором ни один флаг ещё не установлен.
pub(crate) fn construct(_: &mut CallContext<'_>, _: &[BslValue]) -> RtResult<BslValue> {
    Ok(BslValue::new_object(InternetMailMessageFlags {
        writable: std::array::from_fn(|_| Cell::new(None)),
    }))
}
