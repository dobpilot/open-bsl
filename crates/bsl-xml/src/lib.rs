//! Стек XML BSL. Пока здесь живёт XDTO: фабрика, сериализатор, типы и
//! экземпляры; остальные подсистемы (XSD, XPath, DOM, поверхность
//! `ЧтениеXML`/`ЗаписьXML`) переезжают сюда следующими шагами, парсерное
//! ядро остаётся в `bsl_rt::xml`.

pub mod core;
mod dom;
mod xdto;
mod xml;
mod xpath;
mod xsd;

use bsl_rt::{
    Arity, BslValue, CallContext, ConstructorCode, ConstructorDescriptor, FunctionCode,
    FunctionDescriptor, FunctionKind, LibraryDescriptor, RtError, RtResult, TypeDescriptor,
};

pub use dom::{
    new_builder as new_dom_builder, new_document as new_dom_document, new_writer as new_dom_writer,
};
pub use xdto::{factory_of_file, factory_of_schema_set, serializer_of_factory};
pub use xml::{
    new_xml_reader, new_xml_writer, new_xml_writer_settings,
    writer_settings_from_args as xml_writer_settings_value,
};
pub use xpath::new_ns_resolver;
pub use xsd::{new_builder, new_expanded_name, new_schema, new_schema_set};

/// Идентификатор компонента в заголовке байткода.
pub const PACKAGE_NAME: &str = env!("CARGO_PKG_NAME");
/// Версия компонента в заголовке байткода.
pub const PACKAGE_VERSION: &str = env!("CARGO_PKG_VERSION");

fn argument(arguments: &[BslValue], index: usize) -> &BslValue {
    arguments.get(index).unwrap_or(&BslValue::Undefined)
}

fn construct_factory(context: &mut CallContext<'_>, arguments: &[BslValue]) -> RtResult<BslValue> {
    factory_of_schema_set(argument(arguments, 0), context.zone_rc()?)
}

fn construct_builder(
    _context: &mut CallContext<'_>,
    _arguments: &[BslValue],
) -> RtResult<BslValue> {
    Ok(new_builder())
}

fn construct_schema(_context: &mut CallContext<'_>, _arguments: &[BslValue]) -> RtResult<BslValue> {
    Ok(new_schema())
}

fn construct_schema_set(
    _context: &mut CallContext<'_>,
    _arguments: &[BslValue],
) -> RtResult<BslValue> {
    Ok(new_schema_set())
}

fn construct_expanded_name(
    _context: &mut CallContext<'_>,
    arguments: &[BslValue],
) -> RtResult<BslValue> {
    let text = |value: &BslValue| match value {
        BslValue::Str(s) => Ok(s.to_string()),
        _ => Err(RtError::TypeError {
            expected: "Строка",
            op: "Новый РасширенноеИмяXML",
        }),
    };
    Ok(new_expanded_name(
        &text(&arguments[0])?,
        &text(&arguments[1])?,
    ))
}

fn construct_dom_builder(
    _context: &mut CallContext<'_>,
    _arguments: &[BslValue],
) -> RtResult<BslValue> {
    Ok(new_dom_builder())
}

fn construct_dom_document(
    _context: &mut CallContext<'_>,
    _arguments: &[BslValue],
) -> RtResult<BslValue> {
    Ok(new_dom_document())
}

fn construct_dom_writer(
    _context: &mut CallContext<'_>,
    _arguments: &[BslValue],
) -> RtResult<BslValue> {
    Ok(new_dom_writer())
}

fn construct_ns_resolver(
    _context: &mut CallContext<'_>,
    arguments: &[BslValue],
) -> RtResult<BslValue> {
    new_ns_resolver(&arguments[0])
}

fn construct_xml_reader(
    _context: &mut CallContext<'_>,
    _arguments: &[BslValue],
) -> RtResult<BslValue> {
    Ok(new_xml_reader())
}

fn construct_xml_writer(
    _context: &mut CallContext<'_>,
    _arguments: &[BslValue],
) -> RtResult<BslValue> {
    Ok(new_xml_writer())
}

fn construct_xml_writer_settings(
    _context: &mut CallContext<'_>,
    arguments: &[BslValue],
) -> RtResult<BslValue> {
    xml::writer_settings_from_args(
        argument(arguments, 0),
        argument(arguments, 1),
        argument(arguments, 2),
    )
}

fn construct_serializer(
    _context: &mut CallContext<'_>,
    arguments: &[BslValue],
) -> RtResult<BslValue> {
    serializer_of_factory(&arguments[0])
}

fn call_create_factory(
    context: &mut CallContext<'_>,
    arguments: &[BslValue],
) -> RtResult<BslValue> {
    factory_of_file(arguments, context.zone_rc()?)
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
    ConstructorDescriptor {
        code: ConstructorCode::new(3),
        names: &["ПостроительСхемXML", "XMLSchemaBuilder"],
        arity: Arity::exact(0),
        call: construct_builder,
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(4),
        names: &["СхемаXML", "XMLSchema"],
        arity: Arity::exact(0),
        call: construct_schema,
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(5),
        names: &["НаборСхемXML", "XMLSchemaSet"],
        arity: Arity::exact(0),
        call: construct_schema_set,
    },
    ConstructorDescriptor {
        // Английское написание не измерено — есть только русское имя, как
        // в прежней таблице `NEW_TYPES`.
        code: ConstructorCode::new(6),
        names: &["РасширенноеИмяXML"],
        arity: Arity::exact(2),
        call: construct_expanded_name,
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(7),
        names: &["ПостроительDOM", "DOMBuilder"],
        arity: Arity::exact(0),
        call: construct_dom_builder,
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(8),
        names: &["ДокументDOM", "DOMDocument"],
        arity: Arity::exact(0),
        call: construct_dom_document,
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(9),
        names: &["ЗаписьDOM", "DOMWriter"],
        arity: Arity::exact(0),
        call: construct_dom_writer,
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(10),
        names: &["РазыменовательПространствИменDOM", "DOMNamespaceResolver"],
        arity: Arity::exact(1),
        call: construct_ns_resolver,
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(11),
        names: &["ЧтениеXML", "XMLReader"],
        arity: Arity::exact(0),
        call: construct_xml_reader,
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(12),
        names: &["ЗаписьXML", "XMLWriter"],
        arity: Arity::exact(0),
        call: construct_xml_writer,
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(13),
        names: &["ПараметрыЗаписиXML", "XMLWriterSettings"],
        arity: Arity::range(0, 3),
        call: construct_xml_writer_settings,
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

/// Типы, которые компонент вводит в язык: по ним работает `Тип("Имя")`.
const TYPES: &[&TypeDescriptor] = &[
    &crate::xsd::ANNOTATION_TYPE,
    &crate::xsd::APP_INFO_TYPE,
    &crate::xsd::ATTRIBUTE_TYPE,
    &crate::xsd::ATTRIBUTE_USE_TYPE,
    &crate::xsd::BUILDER_TYPE,
    &crate::xsd::COMPLEX_TYPE_TYPE,
    &crate::xsd::DOCUMENTATION_TYPE,
    &crate::dom::DOM_ATTRIBUTE_TYPE,
    &crate::dom::DOM_ATTR_MAP_TYPE,
    &crate::dom::DOM_BUILDER_TYPE,
    &crate::dom::DOM_CDATA_TYPE,
    &crate::dom::DOM_COMMENT_TYPE,
    &crate::dom::DOM_DOCUMENT_TYPE,
    &crate::dom::DOM_ELEMENT_LIST_TYPE,
    &crate::dom::DOM_ELEMENT_TYPE,
    &crate::dom::DOM_ENTITY_REF_TYPE,
    &crate::dom::DOM_NODE_LIST_TYPE,
    &crate::dom::DOM_PI_TYPE,
    &crate::dom::DOM_TEXT_TYPE,
    &crate::dom::DOM_WRITER_TYPE,
    &crate::xsd::ELEMENT_TYPE,
    &crate::xsd::EXPANDED_NAME_TYPE,
    &crate::xpath::EXPRESSION_TYPE,
    &crate::xdto::objects::FACETS_TYPE,
    &crate::xsd::FACET_ENUMERATION_TYPE,
    &crate::xsd::FACET_FRACTION_DIGITS_TYPE,
    &crate::xsd::FACET_LENGTH_TYPE,
    &crate::xsd::FACET_MAX_EXCLUSIVE_TYPE,
    &crate::xsd::FACET_MAX_INCLUSIVE_TYPE,
    &crate::xsd::FACET_MAX_LENGTH_TYPE,
    &crate::xsd::FACET_MIN_EXCLUSIVE_TYPE,
    &crate::xsd::FACET_MIN_INCLUSIVE_TYPE,
    &crate::xsd::FACET_MIN_LENGTH_TYPE,
    &crate::xsd::FACET_PATTERN_TYPE,
    &crate::xsd::FACET_TOTAL_DIGITS_TYPE,
    &crate::xdto::objects::FACET_TYPE,
    &crate::xsd::FACET_WHITE_SPACE_TYPE,
    &crate::xdto::objects::FACTORY_TYPE,
    &crate::xsd::LIST_FIXED_TYPE,
    &crate::xsd::LIST_NAMED_TYPE,
    &crate::xsd::LIST_PLAIN_TYPE,
    &crate::xdto::objects::LIST_TYPE,
    &crate::xsd::MODEL_GROUP_TYPE,
    &crate::xsd::NAME_LIST_TYPE,
    &crate::xdto::objects::OBJECT_TYPE,
    &crate::xdto::objects::OBJECT_TYPE_TYPE,
    &crate::xsd::PARTICLE_TYPE,
    &crate::xdto::objects::PROPERTIES_TYPE,
    &crate::xdto::objects::PROPERTY_TYPE,
    &crate::xml::READER_TYPE,
    &crate::xpath::RESOLVER_TYPE,
    &crate::xpath::RESULT_TYPE,
    &crate::xsd::SCHEMA_SET_TYPE,
    &crate::xsd::SCHEMA_TYPE,
    &crate::xdto::objects::SEQUENCE_TYPE,
    &crate::xdto::objects::SERIALIZER_TYPE,
    &crate::xml::SETTINGS_TYPE,
    &crate::xsd::SIMPLE_TYPE_TYPE,
    &crate::xdto::objects::VALUE_TYPE,
    &crate::xdto::objects::VALUE_TYPE_TYPE,
    &crate::xml::WRITER_TYPE,
];

/// Дескриптор статически подключаемого компонента стека XML.
pub const fn library() -> LibraryDescriptor {
    LibraryDescriptor {
        package: PACKAGE_NAME,
        object_jit: bsl_rt::ObjectJitPolicy::NativeContextCompatible,
        version: PACKAGE_VERSION,
        // Ядро в зависимостях не объявляется: реестр включает его в
        // требования любой программы (`RuntimeRegistry::requirements_for`).
        dependencies: &[],
        functions: FUNCTIONS,
        constructors: CONSTRUCTORS,
        types: TYPES,
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
        assert_eq!(constructors, (1..=13).collect::<Vec<_>>());
        let functions = library()
            .functions
            .iter()
            .map(|function| function.code.get())
            .collect::<Vec<_>>();
        assert_eq!(functions, (1..=2).collect::<Vec<_>>());
    }
}
