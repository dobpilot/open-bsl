//! Создание узлов: элементы, атрибуты, текст, секции.

use super::*;

// --- Создание узлов -----------------------------------------------------

/// `Новый ДокументDOM` — пустой документ.
///
/// ИЗМЕРЕНО: детей ноль, `ЭлементДокумента` — `Неопределено`, `ВерсияXML` —
/// `1.0`, а `ДокументВладелец` — сам документ.
pub fn new_document() -> BslValue {
    let doc = DomNode::new(
        DomKind::Document,
        DOCUMENT_NODE_NAME.to_string(),
        String::new(),
        None,
    );
    *doc.owner.borrow_mut() = Rc::downgrade(&doc);
    node_value(&doc, &doc)
}

/// `Новый ЗаписьDOM`.
/// `Новый ПостроительDOM` — состояния нет.
pub fn new_builder() -> BslValue {
    BslValue::new_object(DomBuilderObject)
}

pub fn new_writer() -> BslValue {
    BslValue::new_object(DomWriterObject)
}

pub fn is_dom_writer(v: &BslValue) -> bool {
    v.object_ref()
        .is_some_and(|object| object.downcast_ref::<DomWriterObject>().is_some())
}

/// Годится ли строка как имя XML.
///
/// Платформа отвергает и пустое имя, и `1` — именно поэтому
/// `СоздатьАтрибут("а", "1")` ошибка: это форма `(URI, Имя)` с именем `1`.
/// Правило то же, что в XML: каждый отрезок между двоеточиями начинается с
/// буквы или подчёркивания, а дальше идут ещё цифры, дефис и точка. ИЗМЕРЕНО,
/// что `а б` и `-а` платформа отвергает, а `а.б` и даже `а:б:в` принимает —
/// поэтому число двоеточий не ограничено (префикс при этом отрезается по
/// ПЕРВОМУ, как в остальном коде).
pub(crate) fn valid_name(name: &str) -> bool {
    let part_ok = |part: &str| {
        let mut chars = part.chars();
        let Some(first) = chars.next() else {
            return false;
        };
        if !(first.is_alphabetic() || first == '_') {
            return false;
        }
        chars.all(|c| c.is_alphanumeric() || matches!(c, '_' | '-' | '.'))
    };
    !name.is_empty() && name.split(':').all(part_ok)
}

/// Получатель фабричного метода. ИЗМЕРЕНО, что фабрики есть только у
/// документа: `Э.СоздатьЭлемент("а")` платформа отвергает.
pub(crate) fn as_document(v: &BslValue, method: &'static str) -> RtResult<Rc<DomNode>> {
    let (node, _) = as_node(v, method)?;
    if node.kind != DomKind::Document {
        return Err(RtError::MethodNotApplicable {
            method,
            receiver: v.type_name(),
        });
    }
    Ok(node)
}

/// Разобрать аргументы формы `(Имя)` либо `(URI, Имя)`.
///
/// Одноаргументная форма даёт узел БЕЗ пространства имён, и локальное имя у
/// него платформозависимое: у элемента пустое, у атрибута — всё имя целиком
/// (измерено оба). Двухаргументная расщепляет имя по двоеточию.
pub(crate) fn named_parts(
    kind: DomKind,
    args: &[BslValue],
    op: &'static str,
) -> RtResult<(String, String, String, String)> {
    let bad_name = || RtError::TypeError {
        expected: "имя XML",
        op,
    };
    match args.len() {
        1 => {
            let name = need_str(args.first(), op)?;
            if !valid_name(&name) {
                return Err(bad_name());
            }
            let local = if kind == DomKind::Attribute {
                name.clone()
            } else {
                String::new()
            };
            Ok((name, local, String::new(), String::new()))
        }
        2 => {
            let uri = need_str(args.first(), op)?;
            let name = need_str(args.get(1), op)?;
            if !valid_name(&name) {
                return Err(bad_name());
            }
            let prefix = prefix_of(&name).to_string();
            // ИЗМЕРЕНО: `СоздатьЭлемент("", "п:а")` — ошибка, префикс без
            // пространства имён платформа не принимает.
            if uri.is_empty() && !prefix.is_empty() {
                return Err(RtError::TypeError {
                    expected: "непустой URI для имени с префиксом",
                    op,
                });
            }
            Ok((name.clone(), local_of(&name).to_string(), prefix, uri))
        }
        _ => Err(RtError::TypeError {
            expected: "один или два аргумента",
            op,
        }),
    }
}

/// `ДокументDOM.СоздатьЭлемент(Имя)` / `(URI, Имя)`.
///
/// # Errors
///
/// [`RtError::MethodNotApplicable`], если получатель не документ;
/// [`RtError::TypeError`] при негодном имени или неверной арности.
pub fn create_element(obj: &BslValue, args: &[BslValue]) -> RtResult<BslValue> {
    let doc = as_document(obj, "СоздатьЭлемент")?;
    let (name, local, prefix, uri) = named_parts(DomKind::Element, args, "СоздатьЭлемент")?;
    let el = DomNode::raw(
        DomKind::Element,
        name,
        local,
        prefix.clone(),
        uri.clone(),
        None,
    );
    *el.owner.borrow_mut() = Rc::downgrade(&doc);
    // ИЗМЕРЕНО: у элемента, созданного формой `(URI, Имя)`, объявление
    // пространства имён сразу ВИДНО как атрибут — `xmlns:п` для имени с
    // префиксом и `xmlns` для имени без него, с URI пространства объявлений.
    // Писатель его оттуда и берёт; выдумывать объявление приходится только
    // там, где атрибута нет (узел разобран или пишется в одиночку).
    if !uri.is_empty() {
        // Локальное имя и префикс у объявления — те же, что у РАЗОБРАННОГО
        // объявления (измерено на дереве из разбора: у `xmlns:к` лок «к» и
        // префикс «xmlns», у `xmlns` лок «xmlns» и пустой префикс), поэтому
        // узел заводится обычным путём, с расщеплением имени.
        let decl = DomNode::new(
            DomKind::Attribute,
            decl_name(&prefix),
            XMLNS_URI.to_string(),
            None,
        );
        *decl.parent.borrow_mut() = Rc::downgrade(&el);
        *decl.owner.borrow_mut() = Rc::downgrade(&doc);
        DomNode::set_text_children(&decl, &uri, &doc);
        el.attrs.borrow_mut().push(decl);
    }
    Ok(node_value(&el, &doc))
}

/// `ДокументDOM.СоздатьАтрибут(Имя)` / `(URI, Имя)`.
///
/// Значения у созданного атрибута нет вовсе: ИЗМЕРЕНО, что детей у него
/// ноль, а `Значение` пусто.
///
/// # Errors
///
/// Как у [`create_element`].
pub fn create_attribute(obj: &BslValue, args: &[BslValue]) -> RtResult<BslValue> {
    let doc = as_document(obj, "СоздатьАтрибут")?;
    let (name, local, prefix, uri) = named_parts(DomKind::Attribute, args, "СоздатьАтрибут")?;
    let attr = DomNode::raw(DomKind::Attribute, name, local, prefix, uri, None);
    *attr.owner.borrow_mut() = Rc::downgrade(&doc);
    Ok(node_value(&attr, &doc))
}

/// Общая фабрика узлов, у которых вместо имени данные: текст, секция CDATA
/// и комментарий. Имя узла у них служебное (`#text`, `#cdata-section`,
/// `#comment`) — измерено.
pub(crate) fn create_char_data(
    obj: &BslValue,
    args: &[BslValue],
    kind: DomKind,
    node_name: &str,
    op: &'static str,
) -> RtResult<BslValue> {
    let doc = as_document(obj, op)?;
    if args.len() != 1 {
        return Err(RtError::TypeError {
            expected: "ровно один аргумент",
            op,
        });
    }
    let text = need_str(args.first(), op)?;
    let node = DomNode::new(kind, node_name.to_string(), String::new(), Some(text));
    *node.owner.borrow_mut() = Rc::downgrade(&doc);
    Ok(node_value(&node, &doc))
}

/// `ДокументDOM.СоздатьТекстовыйУзел(Текст)`.
///
/// # Errors
///
/// Как у [`create_element`]; пустая строка допустима (измерено).
pub fn create_text_node(obj: &BslValue, args: &[BslValue]) -> RtResult<BslValue> {
    create_char_data(
        obj,
        args,
        DomKind::Text,
        TEXT_NODE_NAME,
        "СоздатьТекстовыйУзел",
    )
}

/// `ДокументDOM.СоздатьСекциюCDATA(Текст)`.
///
/// # Errors
///
/// Как у [`create_element`].
pub fn create_cdata_section(obj: &BslValue, args: &[BslValue]) -> RtResult<BslValue> {
    create_char_data(
        obj,
        args,
        DomKind::CdataSection,
        CDATA_NODE_NAME,
        "СоздатьСекциюCDATA",
    )
}

/// `ДокументDOM.СоздатьКомментарий(Текст)`.
///
/// # Errors
///
/// Как у [`create_element`].
pub fn create_comment(obj: &BslValue, args: &[BslValue]) -> RtResult<BslValue> {
    create_char_data(
        obj,
        args,
        DomKind::Comment,
        COMMENT_NODE_NAME,
        "СоздатьКомментарий",
    )
}

/// `ДокументDOM.СоздатьИнструкциюОбработки(Цель, Данные)`.
///
/// Ровно два аргумента: ИЗМЕРЕНО, что одноаргументную форму платформа
/// отвергает.
///
/// # Errors
///
/// Как у [`create_element`].
pub fn create_processing_instruction(obj: &BslValue, args: &[BslValue]) -> RtResult<BslValue> {
    let op = "СоздатьИнструкциюОбработки";
    let doc = as_document(obj, op)?;
    if args.len() != 2 {
        return Err(RtError::TypeError {
            expected: "ровно два аргумента — цель и данные",
            op,
        });
    }
    let target = need_str(args.first(), op)?;
    if !valid_name(&target) {
        return Err(RtError::TypeError {
            expected: "имя XML",
            op,
        });
    }
    let data = need_str(args.get(1), op)?;
    let node = DomNode::new(
        DomKind::ProcessingInstruction,
        target,
        String::new(),
        Some(data),
    );
    *node.owner.borrow_mut() = Rc::downgrade(&doc);
    Ok(node_value(&node, &doc))
}
