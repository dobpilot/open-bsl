//! Построение дерева из потока событий XML.

use super::*;

// --- Построение ---------------------------------------------------------

pub(crate) fn bad(what: impl Into<String>) -> RtError {
    RtError::Xml(what.into())
}

/// Предел вложенности ЭЛЕМЕНТОВ в дереве DOM.
///
/// Само построение итеративно, а вот всё, что с готовым деревом потом
/// происходит, — нет: рекурсивны и `Drop` для `Rc<DomNode>` с детьми в
/// `RefCell<Vec<..>>`, и [`DomNode::collect_text`], и
/// [`DomNode::descendants`], и обход при записи. Без предела документ вида
/// `<а><а><а>…` валил бы процесс переполнением стека вместо перехватываемой
/// ошибки — причём на разрушении дерева, то есть уже после того, как скрипт
/// напечатал весь свой вывод.
///
/// Точек, где дерево растёт, ДВЕ: построение и вставка. Обе считают одной
/// мерой — уровнями элементов, где у документа 0, а у корневого элемента 1, —
/// поэтому [`check_insert`] складывает уровень приёмника с высотой
/// вставляемого поддерева и сравнивает с тем же числом. Инвариант
/// поддерживается по индукции: раз в дереве не больше `MAX_DOM_DEPTH`
/// уровней, то и высота любого его поддерева не больше, а значит рекурсия в
/// [`element_height`] тоже ограничена. 500 кадров держит даже стек
/// debug-сборки.
// НЕ ИЗМЕРЕНО(DOM.MAX_DEPTH) — какую глубину вложенности допускает
// `ПостроительDOM` на платформе; растущий зонд намеренно не ставится: если
// платформа на нём падает, он уносит весь сеанс замеров. Замер даёт нижнюю
// границу: 500 уровней обязаны читаться.
pub(crate) const MAX_DOM_DEPTH: usize = 500;

pub(crate) fn too_deep() -> RtError {
    RtError::StackOverflow {
        what: "слишком глубокая вложенность элементов при построении DOM",
    }
}

/// `ПостроительDOM.Прочитать(ЧтениеXML)`.
///
/// # Errors
///
/// [`RtError::Xml`] на битой разметке и на читателе без источника,
/// [`RtError::TypeError`], если аргумент не `ЧтениеXML`.
pub fn read(obj: &BslValue, args: &[BslValue]) -> RtResult<BslValue> {
    if !is_dom_builder(obj) {
        return Err(RtError::MethodNotApplicable {
            method: "Прочитать",
            receiver: obj.type_name(),
        });
    }
    // Арность ровно один аргумент: и без аргументов, и с двумя платформа
    // отказывает (измерено).
    if args.len() != 1 {
        return Err(RtError::TypeError {
            expected: "ровно один аргумент — ЧтениеXML",
            op: "ПостроительDOM.Прочитать",
        });
    }
    let state = match args[0]
        .object_ref()
        .and_then(|object| object.downcast_ref::<crate::xml::XmlReaderObject>())
    {
        Some(reader) => &reader.state,
        _ => {
            return Err(RtError::TypeError {
                expected: "ЧтениеXML",
                op: "ПостроительDOM.Прочитать",
            });
        }
    };
    let doc = build(&mut state.borrow_mut())?;
    Ok(node_value(&doc, &doc))
}

/// Дерево по готовому состоянию читателя — тот же путь, что и у
/// `Прочитать`, но без обёртки-значения `ЧтениеXML`. Этим ходит `xsd.rs`,
/// когда схему просят разобрать из текста, а не из переданного BSL-кодом
/// дерева.
///
/// # Errors
///
/// Всё, чем отвечает разбор XML.
pub fn build_tree(state: &mut XmlReaderState) -> RtResult<Rc<DomNode>> {
    build(state)
}

/// Собрать документ из состояния читателя, оставив читателя исчерпанным
/// (измерено: после построения `Прочитать()` отдаёт «Нет», а `ТипУзла` —
/// «Ничего»).
pub(crate) fn build(state: &mut XmlReaderState) -> RtResult<Rc<DomNode>> {
    let doc = DomNode::new(
        DomKind::Document,
        DOCUMENT_NODE_NAME.to_string(),
        String::new(),
        None,
    );
    // ИЗМЕРЕНО: `Док.ДокументВладелец = Док` — «Да». Ссылка слабая, так
    // что документ на себе не виснет.
    *doc.owner.borrow_mut() = Rc::downgrade(&doc);
    let Some(parser) = state.parser.as_mut() else {
        return Err(bad("источник для ЧтениеXML не задан"));
    };
    parser.set_report_comments(true);
    let result = build_into(&doc, parser);
    // Версию объявление отдаёт как есть: `version="1.1"` платформа
    // принимает и печатает «1.1» (измерено), без объявления — «1.0».
    if let Some(v) = parser.xml_version() {
        *doc.xml_version.borrow_mut() = v.to_string();
    }
    // Флаг снимается при любом исходе: разборщик остаётся жить в
    // читателе, а его собственный контракт — комментарии не отдавать.
    parser.set_report_comments(false);
    state.current = None;
    state.attr_cursor = None;
    state.depth = 0;
    result?;
    Ok(doc)
}

pub(crate) fn build_into(doc: &Rc<DomNode>, parser: &mut XmlParser) -> RtResult<()> {
    // Цепочка предков: элементы, которые читатель уже открыл, но ещё не
    // закрыл. Платформа их восстанавливает вместе с атрибутами (измерено:
    // после двух `Прочитать` на `<а х="1">раз<б/>два</а>` дерево — это
    // корень `а` с атрибутом `х` и детьми `б` и «два», а уже прочитанный
    // текст «раз» потерян).
    // В `stack` документ лежит первым, поэтому глубина уже открытых
    // элементов — это `stack.len() - 1`, а `stack.len()` до очередного
    // `push` — глубина, которую элемент получит. Отсюда сравнение
    // без поправки: уровни с первого по `MAX_DOM_DEPTH` проходят.
    let mut stack: Vec<Rc<DomNode>> = vec![doc.clone()];
    for open in parser.open_elements() {
        if stack.len() > MAX_DOM_DEPTH {
            return Err(too_deep());
        }
        let el = DomNode::new(DomKind::Element, open.name.clone(), open.uri.clone(), None);
        for a in open.attrs.iter() {
            attach_attribute(&el, doc, &a.name, &a.value, parser);
        }
        let parent = stack.last().cloned().unwrap_or_else(|| doc.clone());
        DomNode::append(&parent, &el, doc);
        stack.push(el);
    }
    while let Some(event) = parser.read()? {
        match event {
            XmlEvent::ElementStart { name, uri, attrs } => {
                if stack.len() > MAX_DOM_DEPTH {
                    return Err(too_deep());
                }
                let el = DomNode::new(DomKind::Element, name, uri, None);
                for a in attrs.iter() {
                    attach_attribute(&el, doc, &a.name, &a.value, parser);
                }
                let parent = stack.last().cloned().unwrap_or_else(|| doc.clone());
                DomNode::append(&parent, &el, doc);
                stack.push(el);
            }
            XmlEvent::ElementEnd { .. } => {
                // Корневой элемент документа снимает последний уровень;
                // глубже стека не уйти — разборщик не отдаёт лишних
                // закрывающих тегов, он на них падает.
                if stack.len() > 1 {
                    stack.pop();
                }
            }
            XmlEvent::Text(t) => {
                let node = DomNode::new(
                    DomKind::Text,
                    TEXT_NODE_NAME.to_string(),
                    String::new(),
                    Some(t),
                );
                let parent = stack.last().cloned().unwrap_or_else(|| doc.clone());
                DomNode::append(&parent, &node, doc);
            }
            XmlEvent::Comment(t) => {
                let parent = stack.last().cloned().unwrap_or_else(|| doc.clone());
                // ИЗМЕРЕНО: комментарий ДО корневого элемента в дерево не
                // попадает, а тот же комментарий ПОСЛЕ корня — попадает.
                // Инструкции обработки такого различия не знают.
                if Rc::ptr_eq(&parent, doc) && document_element(doc).is_none() {
                    continue;
                }
                let node = DomNode::new(
                    DomKind::Comment,
                    COMMENT_NODE_NAME.to_string(),
                    String::new(),
                    Some(t),
                );
                DomNode::append(&parent, &node, doc);
            }
            XmlEvent::ProcessingInstruction { target, data } => {
                let node = DomNode::new(
                    DomKind::ProcessingInstruction,
                    target,
                    String::new(),
                    Some(data),
                );
                let parent = stack.last().cloned().unwrap_or_else(|| doc.clone());
                DomNode::append(&parent, &node, doc);
            }
            XmlEvent::EntityReference { name } => {
                // ИЗМЕРЕНО: ссылка ложится узлом РЯДОМ с текстом
                // (`раз&е;два` — три ребёнка), `ИмяУзла` у неё — имя
                // сущности, `ЗначениеУзла` — `Неопределено`, локальное имя,
                // префикс и URI пусты, а родитель — окружающий элемент.
                let node = DomNode::new(DomKind::EntityReference, name, String::new(), None);
                let parent = stack.last().cloned().unwrap_or_else(|| doc.clone());
                DomNode::append(&parent, &node, doc);
            }
        }
    }
    Ok(())
}

/// Завести узел атрибута и подвесить его к элементу.
///
/// Пространство имён атрибута разрешается ИНАЧЕ, чем у элемента, и это
/// измерено: объявления `xmlns`/`xmlns:префикс` получают URI
/// [`XMLNS_URI`], атрибут без префикса — пустой URI (пространство имён по
/// умолчанию на атрибуты не распространяется), а атрибут с префиксом —
/// URI этого префикса.
pub(crate) fn attach_attribute(
    el: &Rc<DomNode>,
    doc: &Rc<DomNode>,
    name: &str,
    value: &str,
    parser: &XmlParser,
) {
    let prefix = prefix_of(name);
    let uri = if name == "xmlns" || prefix == "xmlns" {
        XMLNS_URI.to_string()
    } else if prefix.is_empty() {
        String::new()
    } else {
        parser.namespace_of(prefix)
    };
    let attr = DomNode::new(
        DomKind::Attribute,
        name.to_string(),
        uri,
        Some(value.to_string()),
    );
    // Значение атрибута — ещё и текстовый ребёнок: измерено, что у
    // атрибута `а="1"` ровно один потомок, `ТекстDOM` со значением «1».
    let text = DomNode::new(
        DomKind::Text,
        TEXT_NODE_NAME.to_string(),
        String::new(),
        Some(value.to_string()),
    );
    DomNode::append(&attr, &text, doc);
    *attr.parent.borrow_mut() = Rc::downgrade(el);
    *attr.owner.borrow_mut() = Rc::downgrade(doc);
    el.attrs.borrow_mut().push(attr);
}

/// Корневой элемент документа — для разбора схемы (`xsd.rs`), которому
/// `СоздатьСхемуXML` даёт документ, а работать надо с его корнем.
pub fn xs_document_element(doc: &Rc<DomNode>) -> Option<Rc<DomNode>> {
    document_element(doc)
}

/// Корневой элемент документа, если он уже есть.
pub(crate) fn document_element(doc: &Rc<DomNode>) -> Option<Rc<DomNode>> {
    doc.children
        .borrow()
        .iter()
        .find(|c| c.kind == DomKind::Element)
        .cloned()
}
