//! Описание передаваемого файла с именем и местом хранения.

use std::cell::RefCell;

use crate::{
    BslValue, CallContext, ObjectMembersDescriptor, ObjectProtocol, PropertyDescriptor, RtResult,
    TypeDescriptor, receiver_of,
};

pub(crate) static TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ОписаниеПередаваемогоФайла",
    type_display: "Описание передаваемого файла",
    type_names: &["TransferableFileDescription"],
};

#[derive(Debug)]
struct TransferableFileDescription {
    name: RefCell<String>,
    location: RefCell<String>,
}

impl ObjectProtocol for TransferableFileDescription {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &TYPE
    }

    fn property_table(&self) -> &'static [PropertyDescriptor] {
        PROPERTIES
    }
}

fn get_name(receiver: &dyn ObjectProtocol, _ctx: &mut CallContext<'_>) -> RtResult<BslValue> {
    let file = receiver_of::<TransferableFileDescription>(receiver, "Имя")?;
    Ok(BslValue::Str(file.name.borrow().as_str().into()))
}

fn set_name(
    receiver: &dyn ObjectProtocol,
    value: BslValue,
    ctx: &mut CallContext<'_>,
) -> RtResult<()> {
    let file = receiver_of::<TransferableFileDescription>(receiver, "Имя")?;
    *file.name.borrow_mut() = ctx.format_value(&value, None)?;
    Ok(())
}

fn get_location(receiver: &dyn ObjectProtocol, _ctx: &mut CallContext<'_>) -> RtResult<BslValue> {
    let file = receiver_of::<TransferableFileDescription>(receiver, "Хранение")?;
    Ok(BslValue::Str(file.location.borrow().as_str().into()))
}

fn set_location(
    receiver: &dyn ObjectProtocol,
    value: BslValue,
    ctx: &mut CallContext<'_>,
) -> RtResult<()> {
    let file = receiver_of::<TransferableFileDescription>(receiver, "Хранение")?;
    *file.location.borrow_mut() = ctx.format_value(&value, None)?;
    Ok(())
}

static PROPERTIES: &[PropertyDescriptor] = &[
    PropertyDescriptor {
        names: &["Имя", "Name"],
        get: get_name,
        set: Some(set_name),
    },
    PropertyDescriptor {
        names: &["Хранение", "Location"],
        get: get_location,
        set: Some(set_location),
    },
];

pub(crate) const API_MEMBERS: &[ObjectMembersDescriptor] =
    &[ObjectMembersDescriptor::new(&TYPE).with_properties(PROPERTIES)];

/// Создаёт описание с пустыми строками для опущенных аргументов.
///
/// # Errors
///
/// Возвращает ошибку форматирования переданного значения.
pub(crate) fn construct(ctx: &mut CallContext<'_>, args: &[BslValue]) -> RtResult<BslValue> {
    // НЕ ИЗМЕРЕНО(TRANSFER_FILE.OTHER_TYPES): вне строк и чисел используем
    // общее преобразование `Строка`, пока платформа не измерена для этих типов.
    let name = args
        .first()
        .map(|value| ctx.format_value(value, None))
        .transpose()?
        .unwrap_or_default();
    let location = args
        .get(1)
        .map(|value| ctx.format_value(value, None))
        .transpose()?
        .unwrap_or_default();
    Ok(BslValue::new_object(TransferableFileDescription {
        name: RefCell::new(name),
        location: RefCell::new(location),
    }))
}
