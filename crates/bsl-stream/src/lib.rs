//! Байтовые потоки и последовательное чтение и запись данных BSL.

mod datarw;
mod stream;

use bsl_rt::{
    Arity, BslValue, CallContext, ConstructorCode, ConstructorDescriptor, LibraryDependency,
    LibraryDescriptor, RtResult, TypeDescriptor,
};

pub use datarw::{new_data_reader, new_data_writer};
pub use stream::{new_file_stream, new_file_streams_manager, new_memory_stream};

/// Идентификатор компонента в заголовке байткода.
pub const PACKAGE_NAME: &str = env!("CARGO_PKG_NAME");
/// Версия компонента в заголовке байткода.
pub const PACKAGE_VERSION: &str = env!("CARGO_PKG_VERSION");

fn argument(arguments: &[BslValue], index: usize) -> &BslValue {
    arguments.get(index).unwrap_or(&BslValue::Undefined)
}

fn construct_memory_stream(
    _context: &mut CallContext<'_>,
    arguments: &[BslValue],
) -> RtResult<BslValue> {
    new_memory_stream(argument(arguments, 0))
}

fn construct_file_stream(
    context: &mut CallContext<'_>,
    arguments: &[BslValue],
) -> RtResult<BslValue> {
    new_file_stream(
        &arguments[0],
        &arguments[1],
        argument(arguments, 2),
        context.files()?,
    )
}

fn construct_data_reader(
    context: &mut CallContext<'_>,
    arguments: &[BslValue],
) -> RtResult<BslValue> {
    new_data_reader(
        &arguments[0],
        argument(arguments, 1),
        argument(arguments, 2),
        argument(arguments, 3),
        context.files()?,
    )
}

fn construct_data_writer(
    context: &mut CallContext<'_>,
    arguments: &[BslValue],
) -> RtResult<BslValue> {
    new_data_writer(
        &arguments[0],
        argument(arguments, 1),
        argument(arguments, 2),
        argument(arguments, 3),
        context.files()?,
    )
}

fn construct_file_streams_manager(
    context: &mut CallContext<'_>,
    _arguments: &[BslValue],
) -> RtResult<BslValue> {
    Ok(new_file_streams_manager(context.files_rc()?))
}

const CONSTRUCTORS: &[ConstructorDescriptor] = &[
    ConstructorDescriptor {
        code: ConstructorCode::new(1),
        names: &["ПотокВПамяти", "MemoryStream"],
        arity: Arity::range(0, 1),
        call: construct_memory_stream,
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(2),
        names: &["ФайловыйПоток", "FileStream"],
        arity: Arity::range(2, 3),
        call: construct_file_stream,
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(3),
        names: &["ЧтениеДанных", "DataReader"],
        arity: Arity::range(1, 4),
        call: construct_data_reader,
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(4),
        names: &["ЗаписьДанных", "DataWriter"],
        arity: Arity::range(1, 4),
        call: construct_data_writer,
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(5),
        names: &["ФайловыеПотоки", "FileStreams"],
        arity: Arity::exact(0),
        call: construct_file_streams_manager,
    },
];

/// Типы, которые компонент вводит в язык: по ним работает `Тип("Имя")`.
const TYPES: &[&TypeDescriptor] = &[
    &crate::datarw::DATA_READER_TYPE,
    &crate::datarw::DATA_READ_RESULT_TYPE,
    &crate::datarw::DATA_WRITER_TYPE,
    &crate::stream::FILE_STREAMS_MANAGER_TYPE,
    &crate::stream::FILE_STREAM_TYPE,
    &crate::stream::MEMORY_STREAM_TYPE,
];

/// Дескриптор статически подключаемого компонента потоков.
pub const fn library() -> LibraryDescriptor {
    LibraryDescriptor::new(
        PACKAGE_NAME,
        PACKAGE_VERSION,
        bsl_rt::ObjectJitPolicy::NativeContextCompatible,
    )
    .with_dependencies(&[LibraryDependency {
        package: bsl_binbuf::PACKAGE_NAME,
        version: bsl_binbuf::PACKAGE_VERSION,
    }])
    .with_constructors(CONSTRUCTORS)
    .with_types(TYPES)
    .with_type_aliases(TYPE_ALIASES)
}

/// «Файловый поток» — представление ОБОИХ потоков (измерено), но
/// `Тип("Файловый поток")` обязан отдать `ФайловыйПоток`: единственный
/// случай неоднозначного написания в дереве, владелец объявлен явно
/// (см. ABI-D, каталог типов реестра).
const TYPE_ALIASES: &[(&str, &TypeDescriptor)] =
    &[("Файловый поток", &crate::stream::FILE_STREAM_TYPE)];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructor_codes_are_static_and_dense() {
        let codes = library()
            .constructors()
            .iter()
            .map(|constructor| constructor.code.get())
            .collect::<Vec<_>>();
        assert_eq!(codes, (1..=5).collect::<Vec<_>>());
    }
}
