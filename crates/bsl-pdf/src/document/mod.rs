//! `ДокументPDF»: читатель контейнера PDF и поверхность встроенного языка.
//!
//! Писательское ядро (объектная модель, шрифты, примитивы страниц) живёт в
//! `crate::writer` — им пользуется и раскладка табличного документа; здесь
//! разбор существующего файла, вложения и измеренная на 8.3.27 поверхность
//! «ДокументPDF» с коллекциями страниц и вложений.

use std::cell::RefCell;
use std::rc::Rc;

use crate::writer::{PdfValue, inflate_with_limit, write_value, zlib_compress};
use bsl_rt::{
    BslNumber, BslString, BslValue, CallContext, EnumValue, ObjectProtocol, RtError, RtResult,
    TypeDescriptor, TypeId,
};

// Модуль разложен по подсистемам, но остаётся одним пространством
// имён: подмодули видят друг друга через `use super::*`.
mod container;
mod objects;
mod surface;

#[cfg(test)]
mod tests;

pub(crate) use container::*;
pub use surface::*;
