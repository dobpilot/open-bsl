//! Мост к значениям BSL: свойства узлов и списков.

use super::*;

// --- Склейка с объектами BSL --------------------------------------------

/// Какой из трёх коллекций-типов платформы соответствует список узлов — и
/// откуда он берёт содержимое.
///
/// Тут ИЗМЕРЕНА тонкость, которую видно только вместе с мутацией: две первые
/// коллекции — ЖИВЫЕ окна, а третья — снимок. Список, взятый до
/// `ДобавитьДочерний`, показывает уже двоих детей, коллекция атрибутов,
/// взятая до `УстановитьАтрибут`, — уже два атрибута, а вот результат
/// `ПолучитьЭлементыПоИмени("*")` остаётся прежним, тогда как свежий запрос
/// находит на один элемент больше. Поэтому первые две помнят УЗЕЛ, а третья —
/// найденное.
#[derive(Debug, Clone)]
pub enum DomListKind {
    /// `СписокУзловDOM` — `ДочерниеУзлы` этого узла.
    Nodes(Rc<DomNode>),
    /// `КоллекцияАтрибутовDOM` — `Атрибуты` этого элемента.
    Attributes(Rc<DomNode>),
    /// `СписокЭлементовDOM` — снимок результата `ПолучитьЭлементыПоИмени`.
    Elements(Vec<Rc<DomNode>>),
}

impl DomListKind {
    /// Содержимое коллекции на СЕЙЧАС. Живые окна перечитывают узел, снимок
    /// отдаёт своё.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn items(&self) -> Vec<Rc<DomNode>> {
        match self {
            DomListKind::Nodes(node) => node.children.borrow().clone(),
            DomListKind::Attributes(el) => el.attrs.borrow().clone(),
            DomListKind::Elements(found) => found.clone(),
        }
    }

    /// Длина без копирования содержимого.
    pub fn len(&self) -> usize {
        match self {
            DomListKind::Nodes(node) => node.children.borrow().len(),
            DomListKind::Attributes(el) => el.attrs.borrow().len(),
            DomListKind::Elements(found) => found.len(),
        }
    }

    /// Пуста ли коллекция — `ЗначениеЗаполнено` спрашивает именно это.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Узел по номеру.
    pub fn get(&self, i: usize) -> Option<Rc<DomNode>> {
        match self {
            DomListKind::Nodes(node) => node.children.borrow().get(i).cloned(),
            DomListKind::Attributes(el) => el.attrs.borrow().get(i).cloned(),
            DomListKind::Elements(found) => found.get(i).cloned(),
        }
    }
}

/// Обёртка узла в значение BSL. Каждый вызов даёт НОВЫЙ объект-обёртку,
/// но равенство узлов идёт по первому `Rc` (см. `PartialEq` в `lib.rs`),
/// поэтому `Э.ПервыйДочерний = Э.ДочерниеУзлы[0]` — «Да», как на
/// платформе.
///
/// `doc` — документ этого узла; значение носит на него СИЛЬНУЮ ссылку,
/// см. [`BslObject::DomNode`]. Для самого документа обе ссылки — одна и
/// та же.
pub fn node_value(node: &Rc<DomNode>, doc: &Rc<DomNode>) -> BslValue {
    BslValue::new_object(DomNodeObject {
        node: node.clone(),
        doc: doc.clone(),
    })
}

pub(crate) fn list_value(kind: DomListKind, doc: &Rc<DomNode>) -> BslValue {
    BslValue::new_object(DomListObject {
        kind,
        doc: doc.clone(),
    })
}

pub(crate) fn opt_node(node: Option<Rc<DomNode>>, doc: &Rc<DomNode>) -> BslValue {
    match node {
        Some(n) => node_value(&n, doc),
        None => BslValue::Undefined,
    }
}

pub(crate) fn opt_str(value: Option<&String>) -> BslValue {
    match value {
        Some(s) => BslValue::Str(BslString::from_str(s)),
        None => BslValue::Undefined,
    }
}

pub(crate) fn str_value(s: &str) -> BslValue {
    BslValue::Str(BslString::from_str(s))
}

pub fn is_dom_builder(v: &BslValue) -> bool {
    v.object_ref()
        .is_some_and(|object| object.downcast_ref::<DomBuilderObject>().is_some())
}

/// Узел значения вместе с его документом: второй `Rc` нужен всем, кто
/// строит из этого узла новое значение, — иначе документ не дожил бы до
/// следующей навигации.
pub(crate) fn as_node(v: &BslValue, method: &'static str) -> RtResult<(Rc<DomNode>, Rc<DomNode>)> {
    match v
        .object_ref()
        .and_then(|object| object.downcast_ref::<DomNodeObject>())
    {
        Some(node) => Ok((node.node.clone(), node.doc.clone())),
        _ => Err(RtError::MethodNotApplicable {
            method,
            receiver: v.type_name(),
        }),
    }
}

pub(crate) fn as_element(
    v: &BslValue,
    method: &'static str,
) -> RtResult<(Rc<DomNode>, Rc<DomNode>)> {
    let (node, doc) = as_node(v, method)?;
    if node.kind != DomKind::Element {
        return Err(RtError::MethodNotApplicable {
            method,
            receiver: v.type_name(),
        });
    }
    Ok((node, doc))
}

pub(crate) fn need_str(arg: Option<&BslValue>, op: &'static str) -> RtResult<String> {
    match arg {
        Some(BslValue::Str(s)) => Ok(s.to_string()),
        _ => Err(RtError::TypeError {
            expected: "Строка",
            op,
        }),
    }
}

/// Свойство узла. Имена — измеренные, английские написания идут парой к
/// русским.
///
/// # Errors
///
/// [`RtError::UnknownColumn`], если такого свойства у узла нет: у
/// элемента, например, нет `Данные`, а у текста — `Имя` (измерено, что
/// платформа обе пробы отвергает).
pub fn get_property(obj: &BslValue, name: &str) -> RtResult<BslValue> {
    let (node, doc) = as_node(obj, "свойство узла DOM")?;
    let unknown = || RtError::UnknownColumn(name.to_string());
    let is = |ru: &str, en: &str| folded_eq(name, ru) || folded_eq(name, en);

    if is("ИмяУзла", "NodeName") {
        return Ok(str_value(&node.name));
    }
    if is("ТипУзла", "NodeType") {
        return Ok(BslValue::Enum(node.kind.node_type()));
    }
    if is("ЗначениеУзла", "NodeValue") {
        // У документа, элемента и инструкции обработки значения нет —
        // измерено все три.
        return Ok(match node.kind {
            DomKind::Text | DomKind::CdataSection | DomKind::Comment => {
                opt_str(node.value.borrow().as_ref())
            }
            DomKind::Attribute => str_value(&node.attr_value()),
            DomKind::Document
            | DomKind::Element
            | DomKind::ProcessingInstruction
            | DomKind::EntityReference => BslValue::Undefined,
        });
    }
    if is("ТекстовоеСодержимое", "TextContent") {
        return Ok(match node.kind {
            // У документа и инструкции обработки — `Неопределено`
            // (измерено), у комментария — его собственные данные, у
            // остальных — склейка текста.
            DomKind::Document | DomKind::ProcessingInstruction => BslValue::Undefined,
            DomKind::Comment => opt_str(node.value.borrow().as_ref()),
            DomKind::Attribute
            | DomKind::Element
            | DomKind::Text
            | DomKind::CdataSection
            // РАСХОЖДЕНИЕ: платформа на `ТекстовоеСодержимое` ссылки на
            // сущность ЗАВИСАЕТ — прогон снят по таймауту, вывода нет
            // (проверено на объявленной, НЕрекурсивной сущности). Здесь
            // отдаётся пустая строка: воспроизводить зависание незачем.
            | DomKind::EntityReference => str_value(&node.text_content()),
        });
    }
    if is("ЛокальноеИмя", "LocalName") {
        return Ok(str_value(node.local_name()));
    }
    if is("Префикс", "Prefix") {
        return Ok(str_value(node.prefix()));
    }
    if is("URIПространстваИмен", "NamespaceURI") {
        return Ok(str_value(&node.uri));
    }
    if is("РодительскийУзел", "ParentNode") {
        // У атрибута родителя нет (измерено), хотя владеющий элемент у
        // него есть — его отдаёт `ЭлементВладелец`.
        if node.kind == DomKind::Attribute {
            return Ok(BslValue::Undefined);
        }
        return Ok(opt_node(node.parent.borrow().upgrade(), &doc));
    }
    if is("ДокументВладелец", "OwnerDocument") {
        return Ok(opt_node(node.owner.borrow().upgrade(), &doc));
    }
    if is("ПервыйДочерний", "FirstChild") {
        return Ok(opt_node(node.children.borrow().first().cloned(), &doc));
    }
    if is("ПоследнийДочерний", "LastChild") {
        return Ok(opt_node(node.children.borrow().last().cloned(), &doc));
    }
    if is("СледующийСоседний", "NextSibling") {
        return Ok(opt_node(DomNode::sibling(&node, true), &doc));
    }
    if is("ПредыдущийСоседний", "PreviousSibling") {
        return Ok(opt_node(DomNode::sibling(&node, false), &doc));
    }
    if is("ДочерниеУзлы", "ChildNodes") {
        return Ok(list_value(DomListKind::Nodes(node.clone()), &doc));
    }
    if is("Атрибуты", "Attributes") {
        // Коллекция есть только у элемента — у документа и у текста
        // платформа отдаёт `Неопределено` (измерено оба).
        if node.kind != DomKind::Element {
            return Ok(BslValue::Undefined);
        }
        return Ok(list_value(DomListKind::Attributes(node.clone()), &doc));
    }
    if is("ЭлементДокумента", "DocumentElement") {
        if node.kind != DomKind::Document {
            return Err(unknown());
        }
        return Ok(opt_node(document_element(&node), &doc));
    }
    if is("ВерсияXML", "XmlVersion") {
        if node.kind != DomKind::Document {
            return Err(unknown());
        }
        // ИЗМЕРЕНО: без объявления — «1.0», а с `version="1.1"` —
        // «1.1»: платформа отдаёт объявленное значение как есть.
        return Ok(str_value(&node.xml_version.borrow()));
    }
    if is("Имя", "Name") {
        // `Имя` есть у атрибута; у элемента платформа его отвергает
        // (измерено), имя элемента отдаёт `ИмяУзла`.
        if node.kind != DomKind::Attribute {
            return Err(unknown());
        }
        return Ok(str_value(&node.name));
    }
    if is("Значение", "Value") {
        if node.kind != DomKind::Attribute {
            return Err(unknown());
        }
        return Ok(str_value(&node.attr_value()));
    }
    if is("ЭлементВладелец", "OwnerElement") {
        if node.kind != DomKind::Attribute {
            return Err(unknown());
        }
        return Ok(opt_node(node.parent.borrow().upgrade(), &doc));
    }
    if is("Данные", "Data") {
        // `Данные` есть у текста, комментария и инструкции обработки;
        // у элемента платформа отвергает (измерено).
        return match node.kind {
            DomKind::Text
            | DomKind::CdataSection
            | DomKind::Comment
            | DomKind::ProcessingInstruction => Ok(opt_str(node.value.borrow().as_ref())),
            _ => Err(unknown()),
        };
    }
    if is("Цель", "Target") {
        if node.kind != DomKind::ProcessingInstruction {
            return Err(unknown());
        }
        return Ok(str_value(&node.name));
    }
    Err(unknown())
}

/// `ЕстьДочерниеУзлы()`.
///
/// # Errors
///
/// [`RtError::MethodNotApplicable`], если получатель не узел DOM.
pub fn has_child_nodes(obj: &BslValue) -> RtResult<BslValue> {
    let (node, _) = as_node(obj, "ЕстьДочерниеУзлы")?;
    let has = !node.children.borrow().is_empty();
    Ok(BslValue::Boolean(has))
}

/// `ЕстьАтрибуты()` — «Да» только у элемента с атрибутами (измерено: у
/// документа и у текстового узла «Нет», а не ошибка).
///
/// # Errors
///
/// [`RtError::MethodNotApplicable`], если получатель не узел DOM.
pub fn has_attributes(obj: &BslValue) -> RtResult<BslValue> {
    let (node, _) = as_node(obj, "ЕстьАтрибуты")?;
    let has = !node.attrs.borrow().is_empty();
    Ok(BslValue::Boolean(has))
}

/// Найти атрибут по правилам платформы: один аргумент — локальное имя
/// среди атрибутов без пространства имён, два — URI и локальное имя.
pub(crate) fn find_attribute(
    el: &Rc<DomNode>,
    args: &[BslValue],
    op: &'static str,
) -> RtResult<Option<Rc<DomNode>>> {
    let (uri, local) = match args.len() {
        1 => (String::new(), need_str(args.first(), op)?),
        2 => (need_str(args.first(), op)?, need_str(args.get(1), op)?),
        _ => {
            return Err(RtError::TypeError {
                expected: "один или два аргумента",
                op,
            });
        }
    };
    Ok(el
        .attrs
        .borrow()
        .iter()
        .find(|a| a.uri == uri && a.local_name() == local)
        .cloned())
}

/// `ПолучитьАтрибут(Имя)` / `ПолучитьАтрибут(URI, ЛокальноеИмя)` — ЗНАЧЕНИЕ
/// атрибута либо `Неопределено` (измерено: именно `Неопределено`, а не
/// пустая строка).
///
/// # Errors
///
/// [`RtError::MethodNotApplicable`], если получатель не элемент;
/// [`RtError::TypeError`] при неверных аргументах.
pub fn get_attribute(obj: &BslValue, args: &[BslValue]) -> RtResult<BslValue> {
    let (el, _) = as_element(obj, "ПолучитьАтрибут")?;
    let found = find_attribute(&el, args, "ПолучитьАтрибут")?;
    Ok(match found {
        Some(a) => str_value(&a.attr_value()),
        None => BslValue::Undefined,
    })
}

/// `ЕстьАтрибут(Имя)` / `ЕстьАтрибут(URI, ЛокальноеИмя)`.
///
/// # Errors
///
/// Как у [`get_attribute`].
pub fn has_attribute(obj: &BslValue, args: &[BslValue]) -> RtResult<BslValue> {
    let (el, _) = as_element(obj, "ЕстьАтрибут")?;
    let found = find_attribute(&el, args, "ЕстьАтрибут")?;
    Ok(BslValue::Boolean(found.is_some()))
}

/// `ПолучитьУзелАтрибута(Имя)` / `(URI, ЛокальноеИмя)` — сам узел
/// `АтрибутDOM` либо `Неопределено`.
///
/// # Errors
///
/// Как у [`get_attribute`].
pub fn get_attribute_node(obj: &BslValue, args: &[BslValue]) -> RtResult<BslValue> {
    let (el, doc) = as_element(obj, "ПолучитьУзелАтрибута")?;
    Ok(opt_node(
        find_attribute(&el, args, "ПолучитьУзелАтрибута")?,
        &doc,
    ))
}

/// `ПолучитьЭлементыПоИмени(Имя)` / `(URI, Имя)`.
///
/// Двухаргументная форма принимается, но URI НЕ ФИЛЬТРУЕТ: измерено, что
/// платформа отдаёт тот же результат и на заведомо чужом URI. Имя
/// сопоставляется и как полное, и как локальное, `*` берёт все элементы.
///
/// # Errors
///
/// [`RtError::MethodNotApplicable`], если получатель не документ и не
/// элемент; [`RtError::TypeError`] при неверных аргументах.
pub fn get_elements_by_name(obj: &BslValue, args: &[BslValue]) -> RtResult<BslValue> {
    let (node, doc) = as_node(obj, "ПолучитьЭлементыПоИмени")?;
    if !matches!(node.kind, DomKind::Document | DomKind::Element) {
        return Err(RtError::MethodNotApplicable {
            method: "ПолучитьЭлементыПоИмени",
            receiver: obj.type_name(),
        });
    }
    let wanted = match args.len() {
        1 => need_str(args.first(), "ПолучитьЭлементыПоИмени")?,
        2 => {
            // Первый аргумент читается только ради проверки типа: платформа
            // его игнорирует (измерено).
            need_str(args.first(), "ПолучитьЭлементыПоИмени")?;
            need_str(args.get(1), "ПолучитьЭлементыПоИмени")?
        }
        _ => {
            return Err(RtError::TypeError {
                expected: "один или два аргумента",
                op: "ПолучитьЭлементыПоИмени",
            });
        }
    };
    let mut all = Vec::new();
    node.descendants(&mut all);
    all.retain(|e| e.name_matches(&wanted));
    Ok(list_value(DomListKind::Elements(all), &doc))
}

/// `ПолучитьЭлементПоИдентификатору(Идентификатор)`.
///
/// Всегда `Неопределено` — и это ИЗМЕРЕНО с обеих сторон (якорь
/// `DOM.DOC.ELEMENT_BY_ID`): платформа не находит элемент ни по атрибуту,
/// объявленному типом `ID` во внутреннем подмножестве DTD, ни по атрибуту
/// с именем `id`. Своих идентификаторов в дереве и взяться неоткуда: DTD
/// здесь не разбирается вовсе.
///
/// # Errors
///
/// [`RtError::MethodNotApplicable`], если получатель не документ;
/// [`RtError::TypeError`] при неверном аргументе.
pub fn get_element_by_id(obj: &BslValue, args: &[BslValue]) -> RtResult<BslValue> {
    let (node, _) = as_node(obj, "ПолучитьЭлементПоИдентификатору")?;
    if node.kind != DomKind::Document {
        return Err(RtError::MethodNotApplicable {
            method: "ПолучитьЭлементПоИдентификатору",
            receiver: obj.type_name(),
        });
    }
    if args.len() != 1 {
        return Err(RtError::TypeError {
            expected: "ровно один аргумент",
            op: "ПолучитьЭлементПоИдентификатору",
        });
    }
    need_str(args.first(), "ПолучитьЭлементПоИдентификатору")?;
    Ok(BslValue::Undefined)
}
