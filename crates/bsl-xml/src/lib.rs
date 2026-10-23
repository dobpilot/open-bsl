//! Стек XML BSL. Пока здесь живёт XDTO: фабрика, сериализатор, типы и
//! экземпляры; остальные подсистемы (XSD, XPath, DOM, поверхность
//! `ЧтениеXML`/`ЗаписьXML`) переезжают сюда следующими шагами, парсерное
//! ядро остаётся в `bsl_rt::xml`.

mod canonicalization;
pub mod core;
mod dom;
mod legacy_canonicalization;
mod xdto;
mod xml;
mod xml_data_type;
mod xml_string_processing;
mod xpath;
mod xsd;

use bsl_rt::{
    Arity, BslValue, CallContext, ConstructorCode, ConstructorDescriptor, FunctionCode,
    FunctionDescriptor, FunctionKind, GlobalDescriptor, LibraryDescriptor, RtError, RtResult,
    TypeDescriptor, TypeId, TypeRef,
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

fn construct_html_document(
    _context: &mut CallContext<'_>,
    arguments: &[BslValue],
) -> RtResult<BslValue> {
    crate::dom::html::new_document(arguments)
}

fn construct_html_writer(
    _context: &mut CallContext<'_>,
    _arguments: &[BslValue],
) -> RtResult<BslValue> {
    Ok(crate::dom::html::new_writer())
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
    context: &mut CallContext<'_>,
    _arguments: &[BslValue],
) -> RtResult<BslValue> {
    Ok(new_xml_reader(context.files_rc()?))
}

fn construct_xml_writer(
    context: &mut CallContext<'_>,
    _arguments: &[BslValue],
) -> RtResult<BslValue> {
    Ok(new_xml_writer(context.files_rc()?))
}

fn construct_fast_infoset_writer(
    _context: &mut CallContext<'_>,
    _arguments: &[BslValue],
) -> RtResult<BslValue> {
    // Обычная `ЗаписьXML` выдаёт другой формат; успешный конструктор без
    // кодировщика Fast Infoset скрыл бы неверный результат записи.
    Err(RtError::DynamicError(
        "запись Fast Infoset пока не поддерживается этим хостом".into(),
    ))
}

fn construct_fast_infoset_reader(
    _context: &mut CallContext<'_>,
    _arguments: &[BslValue],
) -> RtResult<BslValue> {
    // Обычное `ЧтениеXML` не разбирает бинарный Fast Infoset; его подмена
    // скрыла бы отсутствие декодера до первого чтения.
    Err(RtError::DynamicError(
        "чтение Fast Infoset пока не поддерживается этим хостом".into(),
    ))
}

fn construct_xsl_transform(
    _context: &mut CallContext<'_>,
    _arguments: &[BslValue],
) -> RtResult<BslValue> {
    // Объект должен выполнять XSLT 1.0; пустая оболочка отложила бы ошибку
    // до загрузки таблицы стилей и выдала бы ложную готовность преобразования.
    Err(RtError::DynamicError(
        "преобразование XSL пока не поддерживается этим хостом".into(),
    ))
}

fn construct_xml_writer_settings(
    _context: &mut CallContext<'_>,
    arguments: &[BslValue],
) -> RtResult<BslValue> {
    xml::writer_settings_from_five_args(
        argument(arguments, 0),
        argument(arguments, 1),
        argument(arguments, 2),
        argument(arguments, 3),
        argument(arguments, 4),
    )
}

fn construct_xml_reader_settings(
    _context: &mut CallContext<'_>,
    _arguments: &[BslValue],
) -> RtResult<BslValue> {
    // ЧтениеXML пока не принимает настройки; фиктивный объект молча
    // проигнорировал бы ограничения чтения, заданные прикладным кодом.
    Err(RtError::DynamicError(
        "параметры чтения XML пока не поддерживаются этим хостом".into(),
    ))
}

fn construct_serializer(
    _context: &mut CallContext<'_>,
    arguments: &[BslValue],
) -> RtResult<BslValue> {
    serializer_of_factory(&arguments[0])
}

fn construct_xml_data_type(
    _context: &mut CallContext<'_>,
    arguments: &[BslValue],
) -> RtResult<BslValue> {
    xml_data_type::construct(arguments)
}

fn call_create_factory(
    context: &mut CallContext<'_>,
    arguments: &[BslValue],
) -> RtResult<BslValue> {
    let files = context.files()?;
    factory_of_file(arguments, context.zone_rc()?, files)
}

fn call_configuration_factory(
    _context: &mut CallContext<'_>,
    _arguments: &[BslValue],
) -> RtResult<BslValue> {
    configuration_factory()
}

fn call_configuration_serializer(
    _context: &mut CallContext<'_>,
    _arguments: &[BslValue],
) -> RtResult<BslValue> {
    let factory = configuration_factory()?;
    serializer_of_factory(&factory)
}

fn call_xml_string(_context: &mut CallContext<'_>, arguments: &[BslValue]) -> RtResult<BslValue> {
    let lexical = match &arguments[0] {
        // ИЗМЕРЕНО на 8.3.27: глобальная `XMLСтрока` представляет оба
        // пустых значения пустой строкой. Приведение XDTO-свойства остаётся
        // строже и использует `lexical_of_value` напрямую.
        BslValue::Undefined | BslValue::Null => String::new(),
        value => {
            xdto::instance::lexical_of_value(value, None).ok_or_else(|| RtError::TypeError {
                expected: "Значение с лексическим представлением XML",
                op: "XMLСтрока",
            })?
        }
    };
    Ok(BslValue::Str(lexical.as_str().into()))
}

fn call_find_disallowed_xml_characters(
    _context: &mut CallContext<'_>,
    arguments: &[BslValue],
) -> RtResult<BslValue> {
    let BslValue::Str(text) = &arguments[0] else {
        return Err(RtError::TypeError {
            expected: "Строка",
            op: "НайтиНедопустимыеСимволыXML",
        });
    };
    Ok(BslValue::Boolean(
        xml_string_processing::has_disallowed_xml_characters(text),
    ))
}

fn call_xml_type_of(_context: &mut CallContext<'_>, arguments: &[BslValue]) -> RtResult<BslValue> {
    Ok(xml_data_type::type_of(&arguments[0]))
}

fn call_get_xml_type(_context: &mut CallContext<'_>, arguments: &[BslValue]) -> RtResult<BslValue> {
    xml::get_xml_type(xml::arg_object(&arguments[0])?)
}

fn call_from_xml_type(
    _context: &mut CallContext<'_>,
    arguments: &[BslValue],
) -> RtResult<BslValue> {
    xml_data_type::from_type(arguments)
}

fn call_xml_type(_context: &mut CallContext<'_>, _arguments: &[BslValue]) -> RtResult<BslValue> {
    // Тип метаданных конфигурации нельзя вывести из XML-типа значения:
    // для него требуется отдельное сопоставление с моделью конфигурации.
    Err(RtError::DynamicError(
        "глобальное сопоставление типа с XML пока не поддерживается этим хостом".into(),
    ))
}

fn call_xml_value(context: &mut CallContext<'_>, arguments: &[BslValue]) -> RtResult<BslValue> {
    let BslValue::Str(lexical) = &arguments[1] else {
        return Err(RtError::Xdto(
            "XMLЗначение требует строковое представление XML".into(),
        ));
    };
    let lexical = lexical.to_string();
    let kind = match &arguments[0] {
        BslValue::Type(TypeRef::Native(TypeId::Number)) => xdto::BuiltinBsl::Number,
        BslValue::Type(TypeRef::Native(TypeId::Boolean)) => xdto::BuiltinBsl::Boolean,
        BslValue::Type(TypeRef::Native(TypeId::String)) => xdto::BuiltinBsl::Str,
        BslValue::Type(TypeRef::Native(TypeId::Date)) => xdto::BuiltinBsl::DateTime,
        _ => {
            return Err(RtError::Xdto(
                "XMLЗначение для указанного типа не поддерживается".into(),
            ));
        }
    };
    // У глобальной функции пустая числовая лексика — ошибка. Парсер XDTO
    // принимает её как ноль, поэтому это различие проверяется до делегирования.
    if kind == xdto::BuiltinBsl::Number && lexical.is_empty() {
        return Err(RtError::Xdto(
            "пустая числовая лексическая форма XMLЗначение".into(),
        ));
    }
    xdto::builtin_from_lexical(kind, &lexical, context.zone_rc()?.as_ref())
}

fn call_write_xml(_context: &mut CallContext<'_>, _arguments: &[BslValue]) -> RtResult<BslValue> {
    // Значение может быть объектом конфигурации с правами текущего сеанса;
    // частичная запись примитива вместо полного объекта была бы обманчивой.
    Err(RtError::DynamicError(
        "глобальная запись значения XML пока не поддерживается этим хостом".into(),
    ))
}

fn call_read_xml(_context: &mut CallContext<'_>, _arguments: &[BslValue]) -> RtResult<BslValue> {
    // Нельзя возвращать `Неопределено`, когда содержимое потока не разобрано.
    Err(RtError::DynamicError(
        "глобальное чтение значения XML пока не поддерживается этим хостом".into(),
    ))
}

fn call_can_read_xml(
    _context: &mut CallContext<'_>,
    _arguments: &[BslValue],
) -> RtResult<BslValue> {
    // Ответ определяется XML-типом в текущем положении читателя; без
    // сопоставления этому типу BSL-значения `Ложь` была бы выдуманным ответом.
    Err(RtError::DynamicError(
        "глобальная проверка возможности чтения XML пока не поддерживается этим хостом".into(),
    ))
}

fn call_xml_string_processing(
    _context: &mut CallContext<'_>,
    _arguments: &[BslValue],
) -> RtResult<BslValue> {
    Ok(xml_string_processing::new_manager())
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
        arity: Arity::range(0, 5),
        call: construct_xml_writer_settings,
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(14),
        names: &["ТипДанныхXML", "XMLDataType"],
        arity: Arity::exact(2),
        call: construct_xml_data_type,
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(15),
        names: &["ПараметрыЧтенияXML", "XMLReaderSettings"],
        arity: Arity::range(0, 11),
        call: construct_xml_reader_settings,
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(16),
        names: &["ЗаписьFastInfoset", "FastInfosetWriter"],
        arity: Arity::exact(0),
        call: construct_fast_infoset_writer,
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(17),
        names: &["ЧтениеFastInfoset", "FastInfosetReader"],
        arity: Arity::exact(0),
        call: construct_fast_infoset_reader,
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(18),
        names: &["ПреобразованиеXSL", "XSLTransform"],
        arity: Arity::exact(0),
        call: construct_xsl_transform,
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(19),
        names: &["ДокументHTML", "HTMLDocument"],
        arity: Arity::range(0, 1),
        call: construct_html_document,
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(20),
        names: &["ЗаписьHTML", "HTMLWriter"],
        arity: Arity::exact(0),
        call: construct_html_writer,
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(21),
        names: &[
            "ПреобразованиеККаноническомуXML",
            "ConversionToCanonicalXML",
        ],
        arity: Arity::exact(0),
        call: canonicalization::construct,
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(22),
        names: &["КаноническаяЗаписьXML", "XMLCanonicalizingWriter"],
        arity: Arity::exact(0),
        call: legacy_canonicalization::construct_writer,
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(23),
        names: &["КаноническийDOM", "DOMCanonicalization"],
        arity: Arity::exact(0),
        call: legacy_canonicalization::construct_dom,
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
    FunctionDescriptor {
        code: FunctionCode::new(3),
        names: &["XMLСтрока", "XMLString"],
        arity: Arity::exact(1),
        kind: FunctionKind::Function,
        call: call_xml_string,
    },
    FunctionDescriptor {
        code: FunctionCode::new(4),
        names: &["НайтиНедопустимыеСимволыXML", "FindDisallowedXMLCharacters"],
        arity: Arity::exact(1),
        kind: FunctionKind::Function,
        call: call_find_disallowed_xml_characters,
    },
    FunctionDescriptor {
        code: FunctionCode::new(5),
        names: &["XMLТипЗнч", "XMLTypeOf"],
        arity: Arity::exact(1),
        kind: FunctionKind::Function,
        call: call_xml_type_of,
    },
    FunctionDescriptor {
        code: FunctionCode::new(6),
        names: &["ПолучитьСериализаторXDTO"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: call_configuration_serializer,
    },
    FunctionDescriptor {
        code: FunctionCode::new(7),
        names: &["ПолучитьОбработкуСтрокиXML"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: call_xml_string_processing,
    },
    FunctionDescriptor {
        code: FunctionCode::new(8),
        names: &["XMLЗначение", "XMLValue"],
        arity: Arity::exact(2),
        kind: FunctionKind::Function,
        call: call_xml_value,
    },
    FunctionDescriptor {
        code: FunctionCode::new(9),
        names: &["ЗаписатьXML", "WriteXML"],
        arity: Arity::range(2, 5),
        kind: FunctionKind::Procedure,
        call: call_write_xml,
    },
    FunctionDescriptor {
        code: FunctionCode::new(10),
        names: &["ПрочитатьXML", "ReadXML"],
        arity: Arity::range(1, 2),
        kind: FunctionKind::Function,
        call: call_read_xml,
    },
    FunctionDescriptor {
        code: FunctionCode::new(11),
        names: &["ВозможностьЧтенияXML", "CanReadXML"],
        arity: Arity::exact(1),
        kind: FunctionKind::Function,
        call: call_can_read_xml,
    },
    FunctionDescriptor {
        code: FunctionCode::new(12),
        names: &["XMLТип", "XMLType"],
        arity: Arity::exact(1),
        kind: FunctionKind::Function,
        call: call_xml_type,
    },
    FunctionDescriptor {
        code: FunctionCode::new(13),
        names: &["ИзXMLТипа", "FromXMLType"],
        arity: Arity::range(1, 2),
        kind: FunctionKind::Function,
        call: call_from_xml_type,
    },
    FunctionDescriptor {
        code: FunctionCode::new(14),
        names: &["ПолучитьXMLТип", "GetXMLType"],
        arity: Arity::exact(1),
        kind: FunctionKind::Function,
        call: call_get_xml_type,
    },
];

const GLOBALS: &[GlobalDescriptor] = &[
    GlobalDescriptor {
        names: &["СериализаторXDTO", "XDTOSerializer"],
        function: FunctionCode::new(6),
    },
    GlobalDescriptor {
        names: &["ОбработкаСтрокиXML", "XMLStringProcessing"],
        function: FunctionCode::new(7),
    },
];

/// Типы, которые компонент вводит в язык: по ним работает `Тип("Имя")`.
const TYPES: &[&TypeDescriptor] = &[
    &legacy_canonicalization::WRITER_TYPE,
    &legacy_canonicalization::DOM_TYPE,
    &canonicalization::TYPE,
    &crate::dom::html::DOCUMENT_TYPE,
    &crate::dom::html::DOCUMENT_TYPE_DEFINITION_TYPE,
    &crate::dom::html::WRITER_TYPE,
    &crate::dom::html::BODY_TYPE,
    &crate::dom::html::ROOT_TYPE,
    &crate::dom::html::HEAD_TYPE,
    &crate::dom::html::DIV_TYPE,
    &crate::dom::html::ELEMENT_TYPE,
    &crate::xml_data_type::XML_DATA_TYPE,
    &crate::xml_string_processing::TYPE,
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

const CONSTRUCTOR_TYPES: &[(ConstructorCode, &TypeDescriptor)] = &[
    (
        ConstructorCode::new(22),
        &legacy_canonicalization::WRITER_TYPE,
    ),
    (ConstructorCode::new(23), &legacy_canonicalization::DOM_TYPE),
    (ConstructorCode::new(21), &canonicalization::TYPE),
    (ConstructorCode::new(20), &crate::dom::html::WRITER_TYPE),
    (ConstructorCode::new(19), &crate::dom::html::DOCUMENT_TYPE),
    (ConstructorCode::new(1), &crate::xdto::objects::FACTORY_TYPE),
    (
        ConstructorCode::new(2),
        &crate::xdto::objects::SERIALIZER_TYPE,
    ),
    (ConstructorCode::new(3), &crate::xsd::BUILDER_TYPE),
    (ConstructorCode::new(4), &crate::xsd::SCHEMA_TYPE),
    (ConstructorCode::new(5), &crate::xsd::SCHEMA_SET_TYPE),
    (ConstructorCode::new(6), &crate::xsd::EXPANDED_NAME_TYPE),
    (ConstructorCode::new(7), &crate::dom::DOM_BUILDER_TYPE),
    (ConstructorCode::new(8), &crate::dom::DOM_DOCUMENT_TYPE),
    (ConstructorCode::new(9), &crate::dom::DOM_WRITER_TYPE),
    (ConstructorCode::new(10), &crate::xpath::RESOLVER_TYPE),
    (ConstructorCode::new(11), &crate::xml::READER_TYPE),
    (ConstructorCode::new(12), &crate::xml::WRITER_TYPE),
    (ConstructorCode::new(13), &crate::xml::SETTINGS_TYPE),
    (
        ConstructorCode::new(14),
        &crate::xml_data_type::XML_DATA_TYPE,
    ),
];

const OBJECT_MEMBER_GROUPS: &[&[bsl_rt::ObjectMembersDescriptor]] = &[
    legacy_canonicalization::API_MEMBERS,
    canonicalization::API_MEMBERS,
    crate::dom::html::API_MEMBERS,
    crate::xml_data_type::API_MEMBERS,
    crate::xml_string_processing::API_MEMBERS,
    crate::xml::API_MEMBERS,
    crate::xpath::API_MEMBERS,
    crate::dom::API_MEMBERS,
    crate::xsd::API_MEMBERS,
    crate::xdto::objects::API_MEMBERS,
];

/// Дескриптор статически подключаемого компонента стека XML.
pub const fn library() -> LibraryDescriptor {
    LibraryDescriptor::new(PACKAGE_NAME, PACKAGE_VERSION)
        .with_functions(FUNCTIONS)
        .with_constructors(CONSTRUCTORS)
        .with_constructor_types(CONSTRUCTOR_TYPES)
        .with_globals(GLOBALS)
        .with_types(TYPES)
        .with_object_member_groups(OBJECT_MEMBER_GROUPS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructor_and_function_codes_are_static_and_dense() {
        let constructors = library()
            .constructors()
            .iter()
            .map(|constructor| constructor.code.get())
            .collect::<Vec<_>>();
        assert_eq!(constructors, (1..=23).collect::<Vec<_>>());
        let functions = library()
            .functions()
            .iter()
            .map(|function| function.code.get())
            .collect::<Vec<_>>();
        assert_eq!(functions, (1..=14).collect::<Vec<_>>());
        assert_eq!(library().globals()[0].names, GLOBALS[0].names);
        assert_eq!(library().globals()[0].function, FunctionCode::new(6));
        assert_eq!(library().globals()[1].names, GLOBALS[1].names);
        assert_eq!(library().globals()[1].function, FunctionCode::new(7));
    }

    #[test]
    fn xml_character_ranges_match_the_platform_contract() {
        assert!(!xml_string_processing::has_disallowed_xml_characters(
            &"\t\n\r😀".into()
        ));
        assert!(xml_string_processing::has_disallowed_xml_characters(
            &"\0".into()
        ));
    }
}
