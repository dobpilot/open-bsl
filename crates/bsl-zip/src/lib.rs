//! Архивы BSL: `ЧтениеZipФайла`/`ЧтениеФайлаАрхива` и оба писателя.
//!
//! Контейнер разбирают и собирают внешние крейты `zip` и `flate2`; здесь
//! живёт измеренная на 8.3.27 поверхность встроенного языка — коллекции
//! элементов, пара `Имя`/`ИсходноеИмя`, подстановки и разрешение дублей.

mod archive;

use bsl_rt::{
    Arity, BslValue, CallContext, ConstructorCode, ConstructorDescriptor, LibraryDescriptor,
    RtResult, TypeDescriptor,
};

pub use archive::{
    ArchiveKind, EntriesObject, EntryObject, ReaderObject, WriterObject, new_archive_reader,
    new_archive_writer,
};

/// Идентификатор компонента в заголовке байткода.
pub const PACKAGE_NAME: &str = env!("CARGO_PKG_NAME");
/// Версия компонента в заголовке байткода.
pub const PACKAGE_VERSION: &str = env!("CARGO_PKG_VERSION");

fn argument(arguments: &[BslValue], index: usize) -> &BslValue {
    arguments.get(index).unwrap_or(&BslValue::Undefined)
}

fn construct_zip_reader(
    _context: &mut CallContext<'_>,
    arguments: &[BslValue],
) -> RtResult<BslValue> {
    new_archive_reader(
        true,
        argument(arguments, 0),
        argument(arguments, 1),
        &BslValue::Undefined,
    )
}

fn construct_archive_reader(
    _context: &mut CallContext<'_>,
    arguments: &[BslValue],
) -> RtResult<BslValue> {
    new_archive_reader(
        false,
        argument(arguments, 0),
        argument(arguments, 1),
        argument(arguments, 2),
    )
}

fn construct_zip_writer(
    _context: &mut CallContext<'_>,
    arguments: &[BslValue],
) -> RtResult<BslValue> {
    new_archive_writer(true, arguments)
}

fn construct_archive_writer(
    _context: &mut CallContext<'_>,
    arguments: &[BslValue],
) -> RtResult<BslValue> {
    new_archive_writer(false, arguments)
}

// Арности сняты с платформы и повторяют legacy-ветку `resolve_new`: у
// zip-читателя не больше двух аргументов, у архивного — трёх; у писателей
// семь и восемь мест соответственно, все необязательные.
const CONSTRUCTORS: &[ConstructorDescriptor] = &[
    ConstructorDescriptor {
        code: ConstructorCode::new(1),
        names: &["ЧтениеZipФайла", "ZipFileReader"],
        arity: Arity::range(0, 2),
        call: construct_zip_reader,
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(2),
        names: &["ЧтениеФайлаАрхива", "ArchiveFileReader"],
        arity: Arity::range(0, 3),
        call: construct_archive_reader,
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(3),
        names: &["ЗаписьZipФайла", "ZipFileWriter"],
        arity: Arity::range(0, 7),
        call: construct_zip_writer,
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(4),
        names: &["ЗаписьФайлаАрхива", "ArchiveFileWriter"],
        arity: Arity::range(0, 8),
        call: construct_archive_writer,
    },
];

/// Типы, которые компонент вводит в язык: по ним работает `Тип("Имя")`.
const TYPES: &[&TypeDescriptor] = &[
    &crate::archive::reader::ARCHIVE_ENTRIES_TYPE,
    &crate::archive::reader::ARCHIVE_ENTRY_TYPE,
    &crate::archive::reader::ARCHIVE_READER_TYPE,
    &crate::archive::reader::ARCHIVE_WRITER_TYPE,
    &crate::archive::reader::ZIP_ENTRIES_TYPE,
    &crate::archive::reader::ZIP_ENTRY_TYPE,
    &crate::archive::reader::ZIP_READER_TYPE,
    &crate::archive::reader::ZIP_WRITER_TYPE,
];

/// Дескриптор статически подключаемого компонента архивов.
pub const fn library() -> LibraryDescriptor {
    LibraryDescriptor {
        package: PACKAGE_NAME,
        object_jit: bsl_rt::ObjectJitPolicy::NativeContextCompatible,
        version: PACKAGE_VERSION,
        // Ядро в зависимостях не объявляется: реестр включает его в
        // требования любой программы (`RuntimeRegistry::requirements_for`).
        dependencies: &[],
        functions: &[],
        constructors: CONSTRUCTORS,
        types: TYPES,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Мест у писателей архива разное число, и обе границы ИЗМЕРЕНЫ с
    /// настоящим путём первым аргументом: zip-вариант принимает семь и
    /// отвергает восьмой, архивный принимает восемь и отвергает девятый.
    /// Разница ровно в одно место — на вставленный третьим
    /// `ТипФайлаАрхива`.
    #[test]
    fn the_two_archive_writers_have_different_argument_limits() {
        let arity_of = |name: &str| {
            library()
                .constructors
                .iter()
                .find(|constructor| constructor.names.contains(&name))
                .unwrap_or_else(|| panic!("нет конструктора {name}"))
                .arity
        };
        let zip = arity_of("ЗаписьZipФайла");
        assert!(zip.accepts(7) && !zip.accepts(8));
        let archive = arity_of("ЗаписьФайлаАрхива");
        assert!(archive.accepts(8) && !archive.accepts(9));
    }

    #[test]
    fn constructor_codes_are_static_and_dense() {
        let codes = library()
            .constructors
            .iter()
            .map(|constructor| constructor.code.get())
            .collect::<Vec<_>>();
        assert_eq!(codes, (1..=4).collect::<Vec<_>>());
    }
}
