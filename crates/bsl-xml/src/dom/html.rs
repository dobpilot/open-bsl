//! Измеренная конструкция HTML на общем дереве DOM.

use super::*;

pub(crate) static WRITER_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ЗаписьHTML",
    type_display: "HTML writer",
    type_names: &["HTMLWriter"],
};

/// Строковый получатель измеренного вывода DOM HTML.
#[derive(Debug, Default)]
pub(crate) struct HtmlWriterObject {
    output: RefCell<Option<String>>,
}

impl ObjectProtocol for HtmlWriterObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &WRITER_TYPE
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        WRITER_METHODS
    }
}

pub(crate) fn new_writer() -> BslValue {
    BslValue::new_object(HtmlWriterObject::default())
}

fn writer_set_string(
    receiver: &dyn ObjectProtocol,
    _: &[BslValue],
    _: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let writer = receiver
        .downcast_ref::<HtmlWriterObject>()
        .ok_or_else(|| bad("ожидалась ЗаписьHTML"))?;
    *writer.output.borrow_mut() = Some(String::new());
    Ok(BslValue::Undefined)
}

fn writer_close(
    receiver: &dyn ObjectProtocol,
    _: &[BslValue],
    _: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let writer = receiver
        .downcast_ref::<HtmlWriterObject>()
        .ok_or_else(|| bad("ожидалась ЗаписьHTML"))?;
    // ИЗМЕРЕНО: до открытия и после закрытия платформа возвращает
    // `Неопределено`; открытый пустой получатель возвращает пустую строку.
    Ok(match writer.output.borrow_mut().take() {
        Some(output) => BslValue::Str(output.as_str().into()),
        None => BslValue::Undefined,
    })
}

pub(crate) fn write_to_string(target: &BslValue, node: &Rc<DomNode>) -> RtResult<()> {
    let object = target
        .object_ref()
        .ok_or_else(|| bad("ожидалась ЗаписьHTML"))?;
    let writer = object
        .downcast_ref::<HtmlWriterObject>()
        .ok_or_else(|| bad("этот получатель HTML пока не реализован"))?;
    // Сначала проверяем всё дерево: отказ не оставляет частичный документ
    // в строковом получателе.
    let rendered = super::serialize::html_string(node)?;
    let mut output = writer.output.borrow_mut();
    let output = output
        .as_mut()
        .ok_or_else(|| bad("запись HTML без строкового получателя пока не реализована"))?;
    if !output.is_empty() {
        return Err(bad(
            "повторная запись документа HTML в один получатель запрещена",
        ));
    }
    output.push_str(&rendered);
    Ok(())
}

const WRITER_METHODS: &[MethodDescriptor] = &[
    MethodDescriptor::new(
        &["УстановитьСтроку", "SetString"],
        Arity::exact(0),
        writer_set_string,
    ),
    MethodDescriptor::new(&["Закрыть", "Close"], Arity::exact(0), writer_close),
];

pub(crate) static DOCUMENT_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ДокументHTML",
    type_display: "HTML document",
    type_names: &["HTMLDocument"],
};
pub(crate) static BODY_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ЭлементТелоHTML",
    type_display: "HTML body element",
    type_names: &["HTMLBodyElement"],
};
pub(crate) static ROOT_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ЭлементРазметкаHTML",
    type_display: "HTML Marking element",
    type_names: &["HTMLHtmlElement"],
};
pub(crate) static HEAD_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ЭлементЗаголовокHTML",
    type_display: "HTML Title element",
    type_names: &["HTMLHeadElement"],
};
pub(crate) static DIV_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ЭлементБлокHTML",
    type_display: "HTML Block element",
    type_names: &["HTMLDivElement"],
};
pub(crate) static ELEMENT_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ЭлементHTML",
    type_display: "HTML element",
    type_names: &["HTMLElement"],
};

pub(crate) static DOCUMENT_TYPE_DEFINITION_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ОпределениеТипаДокументаDOM",
    type_display: "DOM document type definition",
    type_names: &["DOMDocumentType"],
};

pub(crate) fn descriptor(node: &DomNode) -> &'static TypeDescriptor {
    if node.kind == DomKind::Document {
        return &DOCUMENT_TYPE;
    }
    if node.kind != DomKind::Element {
        return node_descriptor(node.kind);
    }
    match node.name.as_str() {
        "html" => &ROOT_TYPE,
        "body" => &BODY_TYPE,
        "head" => &HEAD_TYPE,
        "div" => &DIV_TYPE,
        _ => &ELEMENT_TYPE,
    }
}

pub(crate) fn validate_element(args: &[BslValue]) -> RtResult<()> {
    let [BslValue::Str(name)] = args else {
        return Err(bad("эта форма создания элемента HTML пока не реализована"));
    };
    if !matches!(
        name.to_string().as_str(),
        "html" | "body" | "head" | "title" | "div" | "br"
    ) {
        return Err(bad("этот вид элемента HTML пока не реализован"));
    }
    Ok(())
}

pub(crate) fn new_document(args: &[BslValue]) -> RtResult<BslValue> {
    let value = super::new_document();
    let (doc, _) = as_node(&value, "ДокументHTML")?;
    doc.html_document.set(true);
    let document_type = DomNode::new(DomKind::DocumentType, "html".into(), String::new(), None);
    DomNode::append(&doc, &document_type, &doc);
    let root = create_element(&value, &[BslValue::Str("html".into())])?;
    append_child(&value, std::slice::from_ref(&root))?;
    if let Some(title) = args.first() {
        let BslValue::Str(title) = title else {
            return Err(bad("эта форма заголовка HTML пока не реализована"));
        };
        if title.len_utf16() == 0 {
            return Ok(value);
        }
        let head = create_element(&value, &[BslValue::Str("head".into())])?;
        let title_element = create_element(&value, &[BslValue::Str("title".into())])?;
        let text = create_text_node(&value, &[BslValue::Str(title.clone())])?;
        append_child(&title_element, &[text])?;
        append_child(&head, &[title_element])?;
        append_child(&root, &[head])?;
    }
    Ok(value)
}

fn body_get(receiver: &dyn ObjectProtocol, _: &mut CallContext<'_>) -> RtResult<BslValue> {
    let object = receiver
        .downcast_ref::<DomNodeObject>()
        .ok_or_else(|| bad("ожидался документ HTML"))?;
    get_body(object)
}

fn get_body(object: &DomNodeObject) -> RtResult<BslValue> {
    let root = document_element(&object.node).ok_or_else(|| bad("корень HTML отсутствует"))?;
    let body = root
        .children
        .borrow()
        .iter()
        .find(|child| child.kind == DomKind::Element && child.name == "body")
        .cloned();
    Ok(opt_node(body, &object.doc))
}

fn body_set(
    receiver: &dyn ObjectProtocol,
    value: BslValue,
    _: &mut CallContext<'_>,
) -> RtResult<()> {
    let object = receiver
        .downcast_ref::<DomNodeObject>()
        .ok_or_else(|| bad("ожидался документ HTML"))?;
    set_body(object, value)
}

fn set_body(object: &DomNodeObject, value: BslValue) -> RtResult<()> {
    let body = need_node(Some(&value), "Тело")?;
    if body.kind != DomKind::Element || body.name != "body" || !same_document(&object.doc, &body) {
        return Err(bad("эта форма назначения тела HTML пока не реализована"));
    }
    let root = document_element(&object.node).ok_or_else(|| bad("корень HTML отсутствует"))?;
    let old = root
        .children
        .borrow()
        .iter()
        .find(|child| child.kind == DomKind::Element && child.name == "body")
        .cloned();
    if let Some(old) = old {
        detach(&old);
    }
    append_child(&node_value(&root, &object.doc), &[value])?;
    Ok(())
}

const BODY_PROPERTY: PropertyDescriptor = PropertyDescriptor {
    names: &["Тело", "Body"],
    get: body_get,
    set: Some(body_set),
};

// Общая модель владения не означает, что XML-свойства вроде `ВерсияXML`
// становятся свойствами HTML. Здесь опубликованы измеренные операции.
pub(crate) static ELEMENT_PROPERTIES: &[PropertyDescriptor] = &[
    NODE_PROPERTIES[0],
    NODE_PROPERTIES[1],
    NODE_PROPERTIES[7],
    NODE_PROPERTIES[13],
];

pub(crate) static DOCUMENT_TYPE_PROPERTIES: &[PropertyDescriptor] =
    &[NODE_PROPERTIES[0], NODE_PROPERTIES[1], NODE_PROPERTIES[7]];

pub(crate) static DOCUMENT_PROPERTIES: &[PropertyDescriptor] = &[
    NODE_PROPERTIES[0],
    NODE_PROPERTIES[7],
    NODE_PROPERTIES[13],
    NODE_PROPERTIES[15],
    BODY_PROPERTY,
];

pub(crate) static DOCUMENT_METHODS: &[MethodDescriptor] =
    &[NODE_METHODS[7], NODE_METHODS[9], NODE_METHODS[13]];

pub(crate) static ELEMENT_METHODS: &[MethodDescriptor] = &[NODE_METHODS[13]];

pub(crate) const API_MEMBERS: &[bsl_rt::ObjectMembersDescriptor] = &[
    bsl_rt::ObjectMembersDescriptor::new(&WRITER_TYPE).with_methods(WRITER_METHODS),
    bsl_rt::ObjectMembersDescriptor::new(&DOCUMENT_TYPE_DEFINITION_TYPE)
        .with_properties(DOCUMENT_TYPE_PROPERTIES),
    bsl_rt::ObjectMembersDescriptor::new(&DOCUMENT_TYPE)
        .with_properties(DOCUMENT_PROPERTIES)
        .with_methods(DOCUMENT_METHODS),
    bsl_rt::ObjectMembersDescriptor::new(&BODY_TYPE)
        .with_properties(ELEMENT_PROPERTIES)
        .with_methods(ELEMENT_METHODS),
    bsl_rt::ObjectMembersDescriptor::new(&ROOT_TYPE)
        .with_properties(ELEMENT_PROPERTIES)
        .with_methods(ELEMENT_METHODS),
    bsl_rt::ObjectMembersDescriptor::new(&HEAD_TYPE)
        .with_properties(ELEMENT_PROPERTIES)
        .with_methods(ELEMENT_METHODS),
    bsl_rt::ObjectMembersDescriptor::new(&DIV_TYPE)
        .with_properties(ELEMENT_PROPERTIES)
        .with_methods(ELEMENT_METHODS),
    bsl_rt::ObjectMembersDescriptor::new(&ELEMENT_TYPE)
        .with_properties(ELEMENT_PROPERTIES)
        .with_methods(ELEMENT_METHODS),
];

#[cfg(test)]
#[path = "html_tests.rs"]
mod tests;
