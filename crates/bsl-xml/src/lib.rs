//! Стек XML BSL. Пока здесь живёт XDTO: фабрика, сериализатор, типы и
//! экземпляры; остальные подсистемы (XSD, XPath, DOM, поверхность
//! `ЧтениеXML`/`ЗаписьXML`) переезжают сюда следующими шагами, парсерное
//! ядро остаётся в `bsl_rt::xml`.

mod xdto;

use bsl_rt::{
    Arity, BslValue, CallContext, ConstructorCode, ConstructorDescriptor, FunctionCode,
    FunctionDescriptor, FunctionKind, LibraryDependency, LibraryDescriptor, RtError, RtResult,
};

pub use xdto::{factory_of_file, factory_of_schema_set, serializer_of_factory};

/// Идентификатор компонента в заголовке байткода.
pub const PACKAGE_NAME: &str = env!("CARGO_PKG_NAME");
/// Версия компонента в заголовке байткода.
pub const PACKAGE_VERSION: &str = env!("CARGO_PKG_VERSION");

fn argument(arguments: &[BslValue], index: usize) -> &BslValue {
    arguments.get(index).unwrap_or(&BslValue::Undefined)
}

fn construct_factory(_context: &mut CallContext<'_>, arguments: &[BslValue]) -> RtResult<BslValue> {
    factory_of_schema_set(argument(arguments, 0))
}

fn construct_serializer(
    _context: &mut CallContext<'_>,
    arguments: &[BslValue],
) -> RtResult<BslValue> {
    serializer_of_factory(&arguments[0])
}

fn call_create_factory(
    _context: &mut CallContext<'_>,
    arguments: &[BslValue],
) -> RtResult<BslValue> {
    factory_of_file(arguments)
}

fn call_configuration_factory(
    _context: &mut CallContext<'_>,
    _arguments: &[BslValue],
) -> RtResult<BslValue> {
    configuration_factory()
}

/// Глобальная `ФабрикаXDTO` — фабрика КОНФИГУРАЦИИ, а конфигурации здесь
/// нет: скрипт исполняется сам по себе, метаданных с пакетами XDTO у него
/// нет и взяться им неоткуда. Платформа в этом месте отдаёт живую фабрику
/// (измерено), и расхождение сознательное: пустая фабрика молча отдавала
/// бы `Неопределено` вместо внятного отказа.
///
/// # Errors
///
/// Всегда возвращает ловимую [`RtError::Xdto`].
pub fn configuration_factory() -> RtResult<BslValue> {
    Err(RtError::Xdto(
        "глобальная ФабрикаXDTO — фабрика конфигурации, а метаданных конфигурации \
         у этой реализации нет; фабрику по схеме строят СоздатьФабрикуXDTO(ПутьКXSD) \
         и Новый ФабрикаXDTO(НаборСхемXML)"
            .to_string(),
    ))
}

// Арности сняты с платформы и повторяют прежние встроенные таблицы: у
// фабрики набор схем необязателен, сериализатору фабрика обязательна.
const CONSTRUCTORS: &[ConstructorDescriptor] = &[
    ConstructorDescriptor {
        code: ConstructorCode::new(1),
        names: &["ФабрикаXDTO", "XDTOFactory"],
        arity: Arity::range(0, 1),
        call: construct_factory,
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(2),
        names: &["СериализаторXDTO", "XDTOSerializer"],
        arity: Arity::exact(1),
        call: construct_serializer,
    },
];

const FUNCTIONS: &[FunctionDescriptor] = &[
    FunctionDescriptor {
        code: FunctionCode::new(1),
        names: &["СоздатьФабрикуXDTO", "CreateXDTOFactory"],
        arity: Arity::exact(1),
        kind: FunctionKind::Function,
        call: call_create_factory,
    },
    FunctionDescriptor {
        code: FunctionCode::new(2),
        names: &["ФабрикаXDTO", "XDTOFactory"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: call_configuration_factory,
    },
];

/// Дескриптор статически подключаемого компонента стека XML.
pub const fn library() -> LibraryDescriptor {
    LibraryDescriptor {
        package: PACKAGE_NAME,
        version: PACKAGE_VERSION,
        dependencies: &[LibraryDependency {
            package: bsl_rt::PACKAGE_NAME,
            version: bsl_rt::PACKAGE_VERSION,
        }],
        functions: FUNCTIONS,
        constructors: CONSTRUCTORS,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructor_and_function_codes_are_static_and_dense() {
        let constructors = library()
            .constructors
            .iter()
            .map(|constructor| constructor.code.get())
            .collect::<Vec<_>>();
        assert_eq!(constructors, (1..=2).collect::<Vec<_>>());
        let functions = library()
            .functions
            .iter()
            .map(|function| function.code.get())
            .collect::<Vec<_>>();
        assert_eq!(functions, (1..=2).collect::<Vec<_>>());
    }
}
