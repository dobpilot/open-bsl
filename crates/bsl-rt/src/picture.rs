//! Пустая картинка без загрузки и декодирования изображения.

use crate::{BslValue, CallContext, ObjectProtocol, ObjectRef, RtResult, TypeDescriptor};

/// Дескриптор единого платформенного типа `Картинка`.
///
/// Компоненты могут создавать свои виды картинок, но должны
/// возвращать этот дескриптор, чтобы тип не зависел от источника
/// значения.
pub static PICTURE_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "Картинка",
    type_display: "Картинка",
    type_names: &["Picture"],
};

#[derive(Debug)]
struct EmptyPicture;

impl ObjectProtocol for EmptyPicture {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &PICTURE_TYPE
    }

    fn is_filled(&self) -> RtResult<bool> {
        Ok(false)
    }

    fn value_eq(&self, other: &ObjectRef) -> Option<bool> {
        Some(other.downcast_ref::<Self>().is_some())
    }

    fn display(&self) -> String {
        // `measure-value-presentations-ru.platform.txt`.
        "Картинка".to_owned()
    }
}

pub(crate) fn construct_picture(
    _: &mut CallContext<'_>,
    arguments: &[BslValue],
) -> RtResult<BslValue> {
    if arguments.is_empty() {
        return Ok(BslValue::new_object(EmptyPicture));
    }
    // Источник картинки нельзя заменять пустым значением: вызывающий код
    // должен узнать, что байты изображения этим хостом ещё не декодируются.
    Err(crate::RtError::DynamicError(
        "загрузка картинки из файла или двоичных данных не реализована".into(),
    ))
}
