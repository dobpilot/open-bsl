//! Объектный протокол: таблицы методов и свойств.

use super::*;

// --- объектный протокол -----------------------------------------------------

pub(crate) static DOM_BUILDER_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ПостроительDOM",
    type_display: "Построитель DOM",
    type_names: &["DOMBuilder"],
};

pub(crate) static DOM_WRITER_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ЗаписьDOM",
    type_display: "Запись DOM",
    type_names: &["DOMWriter"],
};

pub(crate) static DOM_DOCUMENT_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ДокументDOM",
    type_display: "Документ  DOM",
    type_names: &["DOMDocument"],
};

pub(crate) static DOM_ELEMENT_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ЭлементDOM",
    type_display: "Элемент DOM",
    type_names: &["DOMElement"],
};

pub(crate) static DOM_ATTRIBUTE_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "АтрибутDOM",
    type_display: "Атрибут DOM",
    type_names: &["DOMAttribute"],
};

pub(crate) static DOM_TEXT_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ТекстDOM",
    type_display: "Текст DOM",
    type_names: &["DOMText"],
};

pub(crate) static DOM_CDATA_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "СекцияCDATADOM",
    type_display: "Секция CDATA DOM",
    type_names: &["DOMCDATASection"],
};

pub(crate) static DOM_COMMENT_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "КомментарийDOM",
    type_display: "Комментарий  DOM",
    type_names: &["DOMComment"],
};

pub(crate) static DOM_PI_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ИнструкцияОбработкиDOM",
    type_display: "Инструкция обработки DOM",
    type_names: &["DOMProcessingInstruction"],
};

pub(crate) static DOM_ENTITY_REF_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "СсылкаНаСущностьDOM",
    type_display: "Ссылка на сущность DOM",
    type_names: &["DOMEntityReference"],
};

pub(crate) static DOM_NODE_LIST_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "СписокУзловDOM",
    type_display: "Список узлов DOM",
    type_names: &["DOMNodeList"],
};

pub(crate) static DOM_ATTR_MAP_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "КоллекцияАтрибутовDOM",
    type_display: "Коллекция атрибутов DOM",
    type_names: &["DOMAttributeMap"],
};

pub(crate) static DOM_ELEMENT_LIST_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "СписокЭлементовDOM",
    type_display: "Список элементов DOM",
    type_names: &["DOMElementList"],
};

/// `ПостроительDOM` — состояния нет: дерево целиком строит `Прочитать`.
#[derive(Debug)]
pub struct DomBuilderObject;

/// `ЗаписьDOM` — состояния нет: `Записать(Узел, ЗаписьXML)` обходит дерево.
#[derive(Debug)]
pub struct DomWriterObject;

/// Узел DOM вместе с документом-владельцем: второй `Rc` держит дерево
/// живым, у узлов внутри дерева ссылки вверх слабые.
#[derive(Debug)]
pub struct DomNodeObject {
    pub(crate) node: Rc<DomNode>,
    pub(crate) doc: Rc<DomNode>,
}

/// Коллекция узлов — снимок либо живая карта, по виду.
#[derive(Debug)]
pub struct DomListObject {
    pub(crate) kind: DomListKind,
    pub(crate) doc: Rc<DomNode>,
}

pub(crate) fn node_descriptor(kind: DomKind) -> &'static TypeDescriptor {
    match kind {
        DomKind::Document => &DOM_DOCUMENT_TYPE,
        DomKind::Element => &DOM_ELEMENT_TYPE,
        DomKind::Attribute => &DOM_ATTRIBUTE_TYPE,
        DomKind::Text => &DOM_TEXT_TYPE,
        DomKind::CdataSection => &DOM_CDATA_TYPE,
        DomKind::Comment => &DOM_COMMENT_TYPE,
        DomKind::ProcessingInstruction => &DOM_PI_TYPE,
        DomKind::EntityReference => &DOM_ENTITY_REF_TYPE,
    }
}

pub(crate) fn dom_list_descriptor(kind: &DomListKind) -> &'static TypeDescriptor {
    match kind {
        DomListKind::Nodes(_) => &DOM_NODE_LIST_TYPE,
        DomListKind::Attributes(_) => &DOM_ATTR_MAP_TYPE,
        DomListKind::Elements(_) => &DOM_ELEMENT_LIST_TYPE,
    }
}

impl DomNodeObject {
    pub(crate) fn as_value(&self) -> BslValue {
        node_value(&self.node, &self.doc)
    }
}

/// Получатель нужного типа: чужой получает ту же ошибку, что и прежний
/// строковый путь.
macro_rules! receiver_of {
    ($fn_name:ident, $ty:ty, $type_name:expr) => {
        fn $fn_name<'r>(
            receiver: &'r dyn ObjectProtocol,
            method: &'static str,
        ) -> RtResult<&'r $ty> {
            receiver
                .downcast_ref::<$ty>()
                .ok_or(RtError::MethodNotApplicable {
                    method,
                    receiver: $type_name,
                })
        }
    };
}

receiver_of!(builder_of, DomBuilderObject, DOM_BUILDER_TYPE.name);
receiver_of!(writer_of, DomWriterObject, DOM_WRITER_TYPE.name);
receiver_of!(node_of, DomNodeObject, "Узел DOM");
receiver_of!(list_of, DomListObject, "Список DOM");

pub(crate) fn builder_read(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _c: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    builder_of(receiver, "Прочитать")?;
    read(&new_builder(), arguments)
}

pub(crate) static BUILDER_METHODS: &[MethodDescriptor] = &[MethodDescriptor {
    code: MethodCode::new(1),
    names: &["Прочитать", "Read"],
    call: builder_read,
}];

impl ObjectProtocol for DomBuilderObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &DOM_BUILDER_TYPE
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        BUILDER_METHODS
    }
}

pub(crate) fn writer_write(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _c: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    writer_of(receiver, "Записать")?;
    write(&new_writer(), arguments)
}

pub(crate) static WRITER_METHODS: &[MethodDescriptor] = &[MethodDescriptor {
    code: MethodCode::new(1),
    names: &["Записать", "Write"],
    call: writer_write,
}];

impl ObjectProtocol for DomWriterObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &DOM_WRITER_TYPE
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        WRITER_METHODS
    }
}

// Таблицы узла одни на все его виды: тип получателя решает
// `node_descriptor`, а доступность члена — сами реализации, ровно как в
// прежнем общем диспетчере.
macro_rules! node_property {
    ($get:ident, $canonical:expr) => {
        fn $get(receiver: &dyn ObjectProtocol, _c: &mut CallContext<'_>) -> RtResult<BslValue> {
            get_property(&node_of(receiver, $canonical)?.as_value(), $canonical)
        }
    };
    ($get:ident, $set:ident, $canonical:expr) => {
        node_property!($get, $canonical);

        fn $set(
            receiver: &dyn ObjectProtocol,
            value: BslValue,
            _c: &mut CallContext<'_>,
        ) -> RtResult<()> {
            set_property(
                &node_of(receiver, $canonical)?.as_value(),
                $canonical,
                &value,
            )
        }
    };
}

node_property!(node_get_1, node_set_1, "ИмяУзла");
node_property!(node_get_2, "ТипУзла");
node_property!(node_get_3, node_set_3, "ЗначениеУзла");
node_property!(node_get_4, node_set_4, "ТекстовоеСодержимое");
node_property!(node_get_5, "ЛокальноеИмя");
node_property!(node_get_6, "Префикс");
node_property!(node_get_7, "URIПространстваИмен");
node_property!(node_get_8, "РодительскийУзел");
node_property!(node_get_9, "ДокументВладелец");
node_property!(node_get_10, "ПервыйДочерний");
node_property!(node_get_11, "ПоследнийДочерний");
node_property!(node_get_12, "СледующийСоседний");
node_property!(node_get_13, "ПредыдущийСоседний");
node_property!(node_get_14, "ДочерниеУзлы");
node_property!(node_get_15, "Атрибуты");
node_property!(node_get_16, "ЭлементДокумента");
node_property!(node_get_17, "ВерсияXML");
node_property!(node_get_18, "Имя");
node_property!(node_get_19, node_set_19, "Значение");
node_property!(node_get_20, "ЭлементВладелец");
node_property!(node_get_21, node_set_21, "Данные");
node_property!(node_get_22, "Цель");

pub(crate) static NODE_PROPERTIES: &[PropertyDescriptor] = &[
    PropertyDescriptor {
        code: PropertyCode::new(1),
        names: &["ИмяУзла", "NodeName"],
        get: node_get_1,
        set: Some(node_set_1),
    },
    PropertyDescriptor {
        code: PropertyCode::new(2),
        names: &["ТипУзла", "NodeType"],
        get: node_get_2,
        set: None,
    },
    PropertyDescriptor {
        code: PropertyCode::new(3),
        names: &["ЗначениеУзла", "NodeValue"],
        get: node_get_3,
        set: Some(node_set_3),
    },
    PropertyDescriptor {
        code: PropertyCode::new(4),
        names: &["ТекстовоеСодержимое", "TextContent"],
        get: node_get_4,
        set: Some(node_set_4),
    },
    PropertyDescriptor {
        code: PropertyCode::new(5),
        names: &["ЛокальноеИмя", "LocalName"],
        get: node_get_5,
        set: None,
    },
    PropertyDescriptor {
        code: PropertyCode::new(6),
        names: &["Префикс", "Prefix"],
        get: node_get_6,
        set: None,
    },
    PropertyDescriptor {
        code: PropertyCode::new(7),
        names: &["URIПространстваИмен", "NamespaceURI"],
        get: node_get_7,
        set: None,
    },
    PropertyDescriptor {
        code: PropertyCode::new(8),
        names: &["РодительскийУзел", "ParentNode"],
        get: node_get_8,
        set: None,
    },
    PropertyDescriptor {
        code: PropertyCode::new(9),
        names: &["ДокументВладелец", "OwnerDocument"],
        get: node_get_9,
        set: None,
    },
    PropertyDescriptor {
        code: PropertyCode::new(10),
        names: &["ПервыйДочерний", "FirstChild"],
        get: node_get_10,
        set: None,
    },
    PropertyDescriptor {
        code: PropertyCode::new(11),
        names: &["ПоследнийДочерний", "LastChild"],
        get: node_get_11,
        set: None,
    },
    PropertyDescriptor {
        code: PropertyCode::new(12),
        names: &["СледующийСоседний", "NextSibling"],
        get: node_get_12,
        set: None,
    },
    PropertyDescriptor {
        code: PropertyCode::new(13),
        names: &["ПредыдущийСоседний", "PreviousSibling"],
        get: node_get_13,
        set: None,
    },
    PropertyDescriptor {
        code: PropertyCode::new(14),
        names: &["ДочерниеУзлы", "ChildNodes"],
        get: node_get_14,
        set: None,
    },
    PropertyDescriptor {
        code: PropertyCode::new(15),
        names: &["Атрибуты", "Attributes"],
        get: node_get_15,
        set: None,
    },
    PropertyDescriptor {
        code: PropertyCode::new(16),
        names: &["ЭлементДокумента", "DocumentElement"],
        get: node_get_16,
        set: None,
    },
    PropertyDescriptor {
        code: PropertyCode::new(17),
        names: &["ВерсияXML", "XmlVersion"],
        get: node_get_17,
        set: None,
    },
    PropertyDescriptor {
        code: PropertyCode::new(18),
        names: &["Имя", "Name"],
        get: node_get_18,
        set: None,
    },
    PropertyDescriptor {
        code: PropertyCode::new(19),
        names: &["Значение", "Value"],
        get: node_get_19,
        set: Some(node_set_19),
    },
    PropertyDescriptor {
        code: PropertyCode::new(20),
        names: &["ЭлементВладелец", "OwnerElement"],
        get: node_get_20,
        set: None,
    },
    PropertyDescriptor {
        code: PropertyCode::new(21),
        names: &["Данные", "Data"],
        get: node_get_21,
        set: Some(node_set_21),
    },
    PropertyDescriptor {
        code: PropertyCode::new(22),
        names: &["Цель", "Target"],
        get: node_get_22,
        set: None,
    },
];

pub(crate) fn node_method_1(
    receiver: &dyn ObjectProtocol,
    _arguments: &[BslValue],
    _c: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let node = node_of(receiver, "ЕстьДочерниеУзлы")?.as_value();
    has_child_nodes(&node)
}

pub(crate) fn node_method_2(
    receiver: &dyn ObjectProtocol,
    _arguments: &[BslValue],
    _c: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let node = node_of(receiver, "ЕстьАтрибуты")?.as_value();
    has_attributes(&node)
}

pub(crate) fn node_method_3(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _c: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let node = node_of(receiver, "ПолучитьАтрибут")?.as_value();
    get_attribute(&node, arguments)
}

pub(crate) fn node_method_4(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _c: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let node = node_of(receiver, "ЕстьАтрибут")?.as_value();
    has_attribute(&node, arguments)
}

pub(crate) fn node_method_5(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _c: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let node = node_of(receiver, "ПолучитьУзелАтрибута")?.as_value();
    get_attribute_node(&node, arguments)
}

pub(crate) fn node_method_6(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _c: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let node = node_of(receiver, "ПолучитьЭлементыПоИмени")?.as_value();
    get_elements_by_name(&node, arguments)
}

pub(crate) fn node_method_7(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _c: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let node = node_of(receiver, "ПолучитьЭлементПоИдентификатору")?.as_value();
    get_element_by_id(&node, arguments)
}

pub(crate) fn node_method_8(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _c: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let node = node_of(receiver, "СоздатьЭлемент")?.as_value();
    create_element(&node, arguments)
}

pub(crate) fn node_method_9(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _c: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let node = node_of(receiver, "СоздатьАтрибут")?.as_value();
    create_attribute(&node, arguments)
}

pub(crate) fn node_method_10(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _c: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let node = node_of(receiver, "СоздатьТекстовыйУзел")?.as_value();
    create_text_node(&node, arguments)
}

pub(crate) fn node_method_11(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _c: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let node = node_of(receiver, "СоздатьСекциюCDATA")?.as_value();
    create_cdata_section(&node, arguments)
}

pub(crate) fn node_method_12(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _c: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let node = node_of(receiver, "СоздатьКомментарий")?.as_value();
    create_comment(&node, arguments)
}

pub(crate) fn node_method_13(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _c: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let node = node_of(receiver, "СоздатьИнструкциюОбработки")?.as_value();
    create_processing_instruction(&node, arguments)
}

pub(crate) fn node_method_14(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _c: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let node = node_of(receiver, "ДобавитьДочерний")?.as_value();
    append_child(&node, arguments)
}

pub(crate) fn node_method_15(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _c: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let node = node_of(receiver, "ВставитьПеред")?.as_value();
    insert_before(&node, arguments)
}

pub(crate) fn node_method_16(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _c: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let node = node_of(receiver, "УдалитьДочерний")?.as_value();
    remove_child(&node, arguments)
}

pub(crate) fn node_method_17(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _c: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let node = node_of(receiver, "ЗаменитьДочерний")?.as_value();
    replace_child(&node, arguments)
}

pub(crate) fn node_method_18(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _c: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let node = node_of(receiver, "УстановитьАтрибут")?.as_value();
    set_attribute(&node, arguments)
}

pub(crate) fn node_method_19(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _c: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let node = node_of(receiver, "УдалитьАтрибут")?.as_value();
    remove_attribute(&node, arguments)
}

pub(crate) fn node_method_20(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _c: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let node = node_of(receiver, "УстановитьУзелАтрибута")?.as_value();
    set_attribute_node(&node, arguments)
}

pub(crate) fn node_method_21(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _c: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let node = node_of(receiver, "УдалитьУзелАтрибута")?.as_value();
    remove_attribute_node(&node, arguments)
}

pub(crate) fn node_method_22(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _c: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let node = node_of(receiver, "ВычислитьВыражениеXPath")?.as_value();
    crate::xpath::evaluate(&node, arguments)
}

pub(crate) fn node_method_23(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _c: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let node = node_of(receiver, "СоздатьВыражениеXPath")?.as_value();
    crate::xpath::create_expression(&node, arguments)
}

pub(crate) fn node_method_24(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _c: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let node = node_of(receiver, "СоздатьРазыменовательПИ")?.as_value();
    crate::xpath::create_ns_resolver(&node, arguments)
}

pub(crate) static NODE_METHODS: &[MethodDescriptor] = &[
    MethodDescriptor {
        code: MethodCode::new(1),
        names: &["ЕстьДочерниеУзлы", "HasChildNodes"],
        call: node_method_1,
    },
    MethodDescriptor {
        code: MethodCode::new(2),
        names: &["ЕстьАтрибуты", "HasAttributes"],
        call: node_method_2,
    },
    MethodDescriptor {
        code: MethodCode::new(3),
        names: &["ПолучитьАтрибут", "GetAttribute"],
        call: node_method_3,
    },
    MethodDescriptor {
        code: MethodCode::new(4),
        names: &["ЕстьАтрибут", "HasAttribute"],
        call: node_method_4,
    },
    MethodDescriptor {
        code: MethodCode::new(5),
        names: &["ПолучитьУзелАтрибута", "GetAttributeNode"],
        call: node_method_5,
    },
    MethodDescriptor {
        code: MethodCode::new(6),
        names: &["ПолучитьЭлементыПоИмени", "GetElementByTagName"],
        call: node_method_6,
    },
    MethodDescriptor {
        code: MethodCode::new(7),
        names: &["ПолучитьЭлементПоИдентификатору", "GetElementById"],
        call: node_method_7,
    },
    MethodDescriptor {
        code: MethodCode::new(8),
        names: &["СоздатьЭлемент", "CreateElement"],
        call: node_method_8,
    },
    MethodDescriptor {
        code: MethodCode::new(9),
        names: &["СоздатьАтрибут", "CreateAttribute"],
        call: node_method_9,
    },
    MethodDescriptor {
        code: MethodCode::new(10),
        names: &["СоздатьТекстовыйУзел", "CreateTextNode"],
        call: node_method_10,
    },
    MethodDescriptor {
        code: MethodCode::new(11),
        names: &["СоздатьСекциюCDATA", "CreateCDATASection"],
        call: node_method_11,
    },
    MethodDescriptor {
        code: MethodCode::new(12),
        names: &["СоздатьКомментарий", "CreateComment"],
        call: node_method_12,
    },
    MethodDescriptor {
        code: MethodCode::new(13),
        names: &["СоздатьИнструкциюОбработки", "CreateProcessingInstruction"],
        call: node_method_13,
    },
    MethodDescriptor {
        code: MethodCode::new(14),
        names: &["ДобавитьДочерний", "AppendChild"],
        call: node_method_14,
    },
    MethodDescriptor {
        code: MethodCode::new(15),
        names: &["ВставитьПеред", "InsertBefore"],
        call: node_method_15,
    },
    MethodDescriptor {
        code: MethodCode::new(16),
        names: &["УдалитьДочерний", "RemoveChild"],
        call: node_method_16,
    },
    MethodDescriptor {
        code: MethodCode::new(17),
        names: &["ЗаменитьДочерний", "ReplaceChild"],
        call: node_method_17,
    },
    MethodDescriptor {
        code: MethodCode::new(18),
        names: &["УстановитьАтрибут", "SetAttribute"],
        call: node_method_18,
    },
    MethodDescriptor {
        code: MethodCode::new(19),
        names: &["УдалитьАтрибут", "RemoveAttribute"],
        call: node_method_19,
    },
    MethodDescriptor {
        code: MethodCode::new(20),
        names: &["УстановитьУзелАтрибута", "SetAttributeNode"],
        call: node_method_20,
    },
    MethodDescriptor {
        code: MethodCode::new(21),
        names: &["УдалитьУзелАтрибута", "RemoveAttributeNode"],
        call: node_method_21,
    },
    MethodDescriptor {
        code: MethodCode::new(22),
        names: &["ВычислитьВыражениеXPath", "EvaluateXPathExpression"],
        call: node_method_22,
    },
    MethodDescriptor {
        code: MethodCode::new(23),
        names: &["СоздатьВыражениеXPath", "CreateXPathExpression"],
        call: node_method_23,
    },
    MethodDescriptor {
        code: MethodCode::new(24),
        names: &["СоздатьРазыменовательПИ", "CreateNSResolver"],
        call: node_method_24,
    },
];

impl ObjectProtocol for DomNodeObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        node_descriptor(self.node.kind())
    }

    fn property_table(&self) -> &'static [PropertyDescriptor] {
        NODE_PROPERTIES
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        NODE_METHODS
    }

    // Узел равен узлу по ТОЖДЕСТВУ узла в дереве: два значения одного
    // узла, добытые разной навигацией, равны (измерено).
    fn identity_key(&self) -> Option<(usize, usize)> {
        Some((Rc::as_ptr(&self.node) as usize, 0))
    }
}

pub(crate) fn list_count(
    receiver: &dyn ObjectProtocol,
    _arguments: &[BslValue],
    _c: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let list = list_of(receiver, "Количество")?;
    Ok(BslValue::number_from_i64(list.kind.len() as i64))
}

/// `Получить(i)` — тот же путь, что `[i]` (измерено у списков DOM).
pub(crate) fn list_get(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _c: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let list = list_of(receiver, "Получить")?;
    match arguments {
        [index] => list.get_index(index),
        _ => Err(RtError::MethodNotApplicable {
            method: "Получить",
            receiver: list.type_descriptor().name,
        }),
    }
}

pub(crate) static LIST_METHODS: &[MethodDescriptor] = &[
    MethodDescriptor {
        code: MethodCode::new(1),
        names: &["Количество", "Count"],
        call: list_count,
    },
    MethodDescriptor {
        code: MethodCode::new(2),
        names: &["Получить", "Get"],
        call: list_get,
    },
];

impl ObjectProtocol for DomListObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        dom_list_descriptor(&self.kind)
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        LIST_METHODS
    }

    fn get_index(&self, index: &BslValue) -> RtResult<BslValue> {
        let i = dom_index(index)?;
        let node = self.kind.get(i).ok_or(RtError::IndexOutOfBounds {
            index: i as i64,
            len: self.kind.len(),
        })?;
        Ok(node_value(&node, &self.doc))
    }

    fn collection_len(&self) -> RtResult<usize> {
        Ok(self.kind.len())
    }

    // Критерий заполненности у коллекций DOM — ДЛИНА, как у массива
    // (измерено: непустые `ДочерниеУзлы` — «Да», пустые `Атрибуты` — «Нет»).
    fn is_filled(&self) -> RtResult<bool> {
        Ok(!self.kind.is_empty())
    }

    // Коллекции DOM равными не бывают: два обращения к `ДочерниеУзлы`
    // дают «Нет» (измерено), поэтому ключа тождества у списка нет.
}

/// Номер элемента из значения-индекса — та же семантика, что у `[]`
/// встроенных коллекций.
pub(crate) fn dom_index(index: &BslValue) -> RtResult<usize> {
    let BslValue::Number(number) = index else {
        return Err(RtError::BadIndex);
    };
    let index = number.to_i64_exact().ok_or(RtError::BadIndex)?;
    usize::try_from(index).map_err(|_| RtError::BadIndex)
}
