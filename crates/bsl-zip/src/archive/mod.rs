//! Архивы: чтение и запись ZIP и вся BSL-поверхность файлов архивов.
//!
//! Разбор формата делегирован крейтам `zip` и `flate2`; здесь лежит
//! поверхность встроенного языка — `ЧтениеZipФайла`/`ЧтениеФайлаАрхива` со
//! своими коллекциями и элементами ([`ArchiveState`]) и оба писателя.
//!
//! Поверх читателя проходит важная граница: формат хранит имя записи БАЙТАМИ,
//! а всё, что платформа делает с именем дальше — декодирование как UTF-8 с
//! заменяющими символами, подстановка недопустимых в имени файла знаков, срез
//! хвостовых точек и пробелов, разрешение столкновений суффиксом `(N)` и
//! разделение на пару `Имя`/`ИсходноеИмя`, — измерено на 8.3.27 и живёт уже
//! здесь, а не в крейте `zip`.

use std::cell::RefCell;
use std::io::Read as _;
use std::io::Write as _;
use std::rc::Rc;

use bsl_rt::{
    Arity, BslDate, BslNumber, BslString, BslValue, CallContext, EnumKind, EnumValue, FileSystem,
    MethodDescriptor, ObjectProtocol, PropertyDescriptor, RtError, RtResult, TypeDescriptor,
    UNIX_EPOCH_SECONDS, folded_eq,
};

// Модуль разложен по подсистемам, но остаётся одним пространством имён:
// подмодули видят друг друга через `use super::*`.
pub(crate) mod container;
pub(crate) mod objects;
pub(crate) mod reader;
pub(crate) mod writer;

#[cfg(test)]
mod tests;

pub(crate) use container::*;
pub use reader::*;
pub use writer::*;
