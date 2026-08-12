//! DOM на чтение: `ПостроительDOM` строит дерево узлов из `ЧтениеXML`.
//!
//! Второго разборщика здесь нет — дерево собирается из событий того же
//! [`XmlParser`](crate::xml::XmlParser), что питает `ЧтениеXML`. Отсюда
//! бесплатно наследуются его измеренные правила: секция CDATA вливается в
//! соседний текст, целиком пробельный текстовый прогон выбрасывается, а
//! пробел вокруг значащего текста сохраняется, ссылки на сущности
//! раскрываются. Разница ровно одна: построителю разборщик ОТДАЁТ
//! комментарии (`set_report_comments`), потому что в дереве они узлы, а
//! `ЧтениеXML` их не показывает.
//!
//! # Что здесь ИЗМЕРЕНО на 8.3.27
//!
//! Всё перечисленное снято пробами (`tests/conformance/measure/measure-dom.bsl`),
//! а не взято из спецификации DOM и не из справки — платформа отходит от
//! обеих:
//!
//! * `Прочитать(ЧтениеXML)` строит дерево ОТ ТЕКУЩЕЙ ПОЗИЦИИ читателя:
//!   цепочка предков восстанавливается из ещё не закрытых элементов —
//!   вместе с их атрибутами, — а содержимое берётся из событий ПОСЛЕ
//!   текущего. Читатель, доведённый до конца, даёт ПУСТОЙ документ
//!   (`ЭлементДокумента` — `Неопределено`), а не ошибку;
//! * комментарий ДО корневого элемента в дерево не попадает, а после
//!   корня — попадает; инструкция обработки попадает и до, и после;
//!   объявление `<?xml ...?>` узлом не становится;
//! * `ИмяУзла` у документа — `#document`, у текста — `#text`, у
//!   комментария — `#comment`, у инструкции обработки — её цель;
//! * `ЗначениеУзла` есть у текста, комментария и атрибута; у документа,
//!   элемента и инструкции обработки — `Неопределено`. У инструкции
//!   обработки данные отдаёт `Данные`, а `ТекстовоеСодержимое` —
//!   `Неопределено`;
//! * `ТекстовоеСодержимое` элемента — склейка ТЕКСТОВЫХ потомков без
//!   комментариев; у документа — `Неопределено`;
//! * объявления `xmlns` видны как обычные атрибуты, в порядке записи, с
//!   URI `http://www.w3.org/2000/xmlns/`;
//! * поиск атрибута ОДНИМ аргументом — это поиск по ЛОКАЛЬНОМУ имени
//!   среди атрибутов БЕЗ пространства имён: `ПолучитьАтрибут("к:б")` и
//!   `ПолучитьАтрибут("б")` у атрибута `к:б` дают `Неопределено`, а
//!   `ПолучитьАтрибут("urn:к", "б")` — значение. Пространство имён по
//!   умолчанию на атрибуты не распространяется;
//! * `ПолучитьЭлементыПоИмени` сопоставляет имя И как полное, И как
//!   локальное (у документа с `<к:а><а><к:а/></а></к:а>` запрос `"а"`
//!   находит ТРИ элемента, запрос `"к:а"` — два), `"*"` берёт все, а
//!   аргумент URI в двухаргументной форме платформа ИГНОРИРУЕТ: с
//!   `"urn:нет"` результат тот же. Обход — в порядке документа, у
//!   элемента — только потомки, себя он не включает;
//! * узлы сравниваются по тождеству (`Э.ПервыйДочерний =
//!   Э.ДочерниеУзлы[0]` — «Да»), а два обращения к `ДочерниеУзлы` дают
//!   РАЗНЫЕ списки («Нет»);
//! * `ВерсияXML` документа — `1.0` даже у документа без объявления;
//! * битый XML, пустой ввод, два корня, неизвестная сущность и читатель
//!   без источника — ошибка.
//!
//! # Сознательные расхождения
//!
//! * **Узел `ОпределениеТипаДокумента` не строится.** Платформа кладёт в
//!   документ узел для `<!DOCTYPE а SYSTEM "вн.dtd">` (`ИмяУзла` — `а`,
//!   `СистемныйИдентификатор` — `вн.dtd`), но разбор внутреннего
//!   подмножества DTD — отдельная работа, а придумывать половину узла
//!   нельзя. Здесь `<!DOCTYPE ...>` пропускается, как и в `ЧтениеXML`.
//! * **Дети у объявления `xmlns`.** У обычного атрибута `ДочерниеУзлы` —
//!   один текстовый узел со значением (измерено). У атрибута `xmlns:к`
//!   платформа отвечает нестабильно: на одном и том же документе в двух
//!   прогонах `Количество()` дало 2 и 3, а обход этих детей падает с
//!   ошибкой. Воспроизводить дефект незачем — здесь у ЛЮБОГО атрибута
//!   ровно один текстовый ребёнок.
//! * **Повторный `Прочитать` с тем же читателем.** Платформа на этом
//!   умирает (прогон снят по таймауту, вывода нет), поэтому её ответ
//!   неизвестен в принципе. Здесь второй вызов видит исчерпанного
//!   читателя и отдаёт пустой документ — ровно то же, что измерено для
//!   любого другого исчерпанного читателя.
//! * **Префиксы атрибутов у предков, восстановленных из стека читателя.**
//!   Дерево «с глубины» достраивает предков из `open_elements()`, и
//!   префикс их атрибута разрешается через `parser.namespace_of`, то есть
//!   по ТЕКУЩЕЙ области видимости, а не по той, что была на момент
//!   открытия предка. Разойтись это может только на документе, где тот же
//!   префикс переобъявлен глубже: `resolve_prefix` идёт по стеку с конца и
//!   вернёт внутреннее объявление. На платформе случай не мерился —
//!   контрактная проба «с глубины» переобъявления не содержит.

use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

use crate::object::{BslObject, XmlReaderState};
use crate::string::BslString;
use crate::xml::{local_of, prefix_of, XmlEvent, XmlParser};
use crate::xml::{COMMENT_NODE_NAME, DOCUMENT_NODE_NAME, TEXT_NODE_NAME};
use crate::{BslValue, EnumValue, RtError, RtResult};

/// URI, который платформа приписывает объявлениям пространств имён
/// (измерено: `xmlns:к` отдаёт именно это). Значение из спецификации XML
/// Namespaces, но здесь оно ИЗМЕРЕНО, а не процитировано.
const XMLNS_URI: &str = "http://www.w3.org/2000/xmlns/";

/// Вид узла. Ровно те виды, которые построитель умеет создавать; секции
/// CDATA среди них нет намеренно — она вливается в текст (измерено), а
/// определения типа документа нет по причине из заголовка модуля.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DomKind {
    Document,
    Element,
    Attribute,
    Text,
    Comment,
    ProcessingInstruction,
}

impl DomKind {
    /// Член `ТипУзлаDOM`, который отдаёт свойство `ТипУзла`.
    fn node_type(self) -> EnumValue {
        match self {
            DomKind::Document => EnumValue::DomDocument,
            DomKind::Element => EnumValue::DomElement,
            DomKind::Attribute => EnumValue::DomAttribute,
            DomKind::Text => EnumValue::DomText,
            DomKind::Comment => EnumValue::DomComment,
            DomKind::ProcessingInstruction => EnumValue::DomProcessingInstruction,
        }
    }
}

/// Узел дерева.
///
/// Дети и атрибуты складываются на месте при разборе, поэтому лежат в
/// `RefCell`; после того как построение кончилось, дерево только читается.
/// Ссылки вверх — слабые: иначе документ с детьми образовал бы цикл `Rc` и
/// не освобождался бы никогда.
#[derive(Debug)]
pub struct DomNode {
    kind: DomKind,
    /// `ИмяУзла`: полное имя элемента или атрибута, `#document`, `#text`,
    /// `#comment` либо цель инструкции обработки.
    name: String,
    /// `URIПространстваИмен`; пусто, если пространства имён нет.
    uri: String,
    /// `ЗначениеУзла`/`Данные`: текст, комментарий, значение атрибута,
    /// данные инструкции обработки. У документа и элемента значения нет.
    value: Option<String>,
    children: RefCell<Vec<Rc<DomNode>>>,
    attrs: RefCell<Vec<Rc<DomNode>>>,
    /// Родитель по дереву. У атрибута здесь лежит ВЛАДЕЮЩИЙ ЭЛЕМЕНТ, но
    /// наружу как `РодительскийУзел` он не показывается: платформа отдаёт
    /// у атрибута `Неопределено` (измерено).
    parent: RefCell<Weak<DomNode>>,
    owner: RefCell<Weak<DomNode>>,
    /// Номер среди детей родителя — по нему ищутся соседи.
    index: Cell<usize>,
    /// `ВерсияXML` — заполняется только у документа, из объявления.
    xml_version: RefCell<String>,
}

impl DomNode {
    fn new(kind: DomKind, name: String, uri: String, value: Option<String>) -> Rc<DomNode> {
        Rc::new(DomNode {
            kind,
            name,
            uri,
            value,
            children: RefCell::new(Vec::new()),
            attrs: RefCell::new(Vec::new()),
            parent: RefCell::new(Weak::new()),
            owner: RefCell::new(Weak::new()),
            index: Cell::new(0),
            xml_version: RefCell::new(String::from("1.0")),
        })
    }

    pub fn kind(&self) -> DomKind {
        self.kind
    }

    /// Добавить ребёнка, проставив ему родителя, документ и номер.
    fn append(parent: &Rc<DomNode>, child: &Rc<DomNode>, doc: &Rc<DomNode>) {
        let mut kids = parent.children.borrow_mut();
        child.index.set(kids.len());
        *child.parent.borrow_mut() = Rc::downgrade(parent);
        *child.owner.borrow_mut() = Rc::downgrade(doc);
        kids.push(child.clone());
    }

    /// `ЛокальноеИмя`. У всего, кроме элемента и атрибута, платформа
    /// отдаёт пустую строку — измерено на документе, тексте и комментарии.
    fn local_name(&self) -> &str {
        match self.kind {
            DomKind::Element | DomKind::Attribute => local_of(&self.name),
            _ => "",
        }
    }

    /// `Префикс` — по тому же правилу, что и локальное имя.
    fn prefix(&self) -> &str {
        match self.kind {
            DomKind::Element | DomKind::Attribute => prefix_of(&self.name),
            _ => "",
        }
    }

    /// `ТекстовоеСодержимое`: склейка ТЕКСТОВЫХ потомков. Комментарии в
    /// неё не входят (измерено: `<ком>раз<!--к-->два</ком>` даёт «раздва»).
    fn text_content(&self) -> String {
        let mut out = String::new();
        self.collect_text(&mut out);
        out
    }

    fn collect_text(&self, out: &mut String) {
        if self.kind == DomKind::Text {
            if let Some(v) = &self.value {
                out.push_str(v);
            }
        }
        for c in self.children.borrow().iter() {
            c.collect_text(out);
        }
    }

    /// Элементы-потомки в порядке документа. Сам узел в результат не
    /// входит: у элемента платформа его не возвращает (измерено), а у
    /// документа корневой элемент — потомок, и он попадает сюда сам.
    fn descendants(&self, out: &mut Vec<Rc<DomNode>>) {
        for c in self.children.borrow().iter() {
            if c.kind == DomKind::Element {
                out.push(c.clone());
            }
            c.descendants(out);
        }
    }

    /// Совпадает ли имя элемента с запрошенным. ИЗМЕРЕНО: сопоставляется
    /// И полное имя, И локальное, а `*` берёт любое.
    fn name_matches(&self, wanted: &str) -> bool {
        wanted == "*" || self.name == wanted || self.local_name() == wanted
    }

    fn sibling(&self, forward: bool) -> Option<Rc<DomNode>> {
        // У атрибута соседей нет: он не лежит среди детей своего элемента,
        // и `РодительскийУзел` у него `Неопределено` (измерено).
        if self.kind == DomKind::Attribute {
            return None;
        }
        let parent = self.parent.borrow().upgrade()?;
        let kids = parent.children.borrow();
        let i = self.index.get();
        let j = if forward {
            i.checked_add(1)?
        } else {
            i.checked_sub(1)?
        };
        kids.get(j).cloned()
    }
}

// --- Построение ---------------------------------------------------------

fn bad(what: impl Into<String>) -> RtError {
    RtError::Xml(what.into())
}

/// Предел вложенности ЭЛЕМЕНТОВ в дереве DOM.
///
/// Само построение итеративно, а вот всё, что с готовым деревом потом
/// происходит, — нет: рекурсивны и `Drop` для `Rc<DomNode>` с детьми в
/// `RefCell<Vec<..>>`, и [`DomNode::collect_text`], и
/// [`DomNode::descendants`]. Без предела документ вида `<а><а><а>…` валил
/// бы процесс переполнением стека вместо перехватываемой ошибки — причём
/// на разрушении дерева, то есть уже после того, как скрипт напечатал весь
/// свой вывод. Предел ставится в одной точке — на построении, — и этого
/// достаточно: мутационного API у дерева нет, другого способа получить
/// узел, кроме чтения, тоже, поэтому переделывать обход и разрушение в
/// итеративные незачем — 500 кадров держит даже стек debug-сборки.
// НЕ ИЗМЕРЕНО(DOM.MAX_DEPTH) — какую глубину вложенности допускает
// `ПостроительDOM` на платформе; растущий зонд намеренно не ставится: если
// платформа на нём падает, он уносит весь сеанс замеров. Замер даёт нижнюю
// границу: 500 уровней обязаны читаться.
const MAX_DOM_DEPTH: usize = 500;

fn too_deep() -> RtError {
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
    let state = match &args[0] {
        BslValue::Object(o) => match &**o {
            BslObject::XmlReader(state) => state,
            _ => {
                return Err(RtError::TypeError {
                    expected: "ЧтениеXML",
                    op: "ПостроительDOM.Прочитать",
                })
            }
        },
        _ => {
            return Err(RtError::TypeError {
                expected: "ЧтениеXML",
                op: "ПостроительDOM.Прочитать",
            })
        }
    };
    let doc = build(&mut state.borrow_mut())?;
    Ok(node_value(&doc, &doc))
}

/// Собрать документ из состояния читателя, оставив читателя исчерпанным
/// (измерено: после построения `Прочитать()` отдаёт «Нет», а `ТипУзла` —
/// «Ничего»).
fn build(state: &mut XmlReaderState) -> RtResult<Rc<DomNode>> {
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

fn build_into(doc: &Rc<DomNode>, parser: &mut XmlParser) -> RtResult<()> {
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
fn attach_attribute(
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
    attr.index.set(el.attrs.borrow().len());
    el.attrs.borrow_mut().push(attr);
}

/// Корневой элемент документа, если он уже есть.
fn document_element(doc: &Rc<DomNode>) -> Option<Rc<DomNode>> {
    doc.children
        .borrow()
        .iter()
        .find(|c| c.kind == DomKind::Element)
        .cloned()
}

// --- Склейка с объектами BSL --------------------------------------------

/// Какой из трёх коллекций-типов платформы соответствует список узлов.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DomListKind {
    /// `СписокУзловDOM` — `ДочерниеУзлы`.
    Nodes,
    /// `КоллекцияАтрибутовDOM` — `Атрибуты`.
    Attributes,
    /// `СписокЭлементовDOM` — результат `ПолучитьЭлементыПоИмени`.
    Elements,
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
    BslValue::Object(Rc::new(BslObject::DomNode(node.clone(), doc.clone())))
}

fn list_value(kind: DomListKind, items: Vec<Rc<DomNode>>, doc: &Rc<DomNode>) -> BslValue {
    BslValue::Object(Rc::new(BslObject::DomList(kind, items, doc.clone())))
}

fn opt_node(node: Option<Rc<DomNode>>, doc: &Rc<DomNode>) -> BslValue {
    match node {
        Some(n) => node_value(&n, doc),
        None => BslValue::Undefined,
    }
}

fn opt_str(value: Option<&String>) -> BslValue {
    match value {
        Some(s) => BslValue::Str(BslString::from_str(s)),
        None => BslValue::Undefined,
    }
}

fn str_value(s: &str) -> BslValue {
    BslValue::Str(BslString::from_str(s))
}

pub fn is_dom_builder(v: &BslValue) -> bool {
    matches!(v, BslValue::Object(o) if matches!(&**o, BslObject::DomBuilder))
}

/// Узел значения вместе с его документом: второй `Rc` нужен всем, кто
/// строит из этого узла новое значение, — иначе документ не дожил бы до
/// следующей навигации.
fn as_node(v: &BslValue, method: &'static str) -> RtResult<(Rc<DomNode>, Rc<DomNode>)> {
    match v {
        BslValue::Object(o) => match &**o {
            BslObject::DomNode(n, doc) => Ok((n.clone(), doc.clone())),
            _ => Err(RtError::MethodNotApplicable {
                method,
                receiver: v.type_name(),
            }),
        },
        _ => Err(RtError::MethodNotApplicable {
            method,
            receiver: v.type_name(),
        }),
    }
}

fn as_element(v: &BslValue, method: &'static str) -> RtResult<(Rc<DomNode>, Rc<DomNode>)> {
    let (node, doc) = as_node(v, method)?;
    if node.kind != DomKind::Element {
        return Err(RtError::MethodNotApplicable {
            method,
            receiver: v.type_name(),
        });
    }
    Ok((node, doc))
}

fn need_str(arg: Option<&BslValue>, op: &'static str) -> RtResult<String> {
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
    let is = |ru: &str, en: &str| name.eq_ignore_ascii_case(ru) || name.eq_ignore_ascii_case(en);

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
            DomKind::Text | DomKind::Comment | DomKind::Attribute => opt_str(node.value.as_ref()),
            _ => BslValue::Undefined,
        });
    }
    if is("ТекстовоеСодержимое", "TextContent") {
        return Ok(match node.kind {
            // У документа и инструкции обработки — `Неопределено`
            // (измерено), у комментария и атрибута — их собственные
            // данные, у элемента и текста — склейка текста.
            DomKind::Document | DomKind::ProcessingInstruction => BslValue::Undefined,
            DomKind::Attribute | DomKind::Comment => opt_str(node.value.as_ref()),
            DomKind::Element | DomKind::Text => str_value(&node.text_content()),
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
        return Ok(opt_node(node.sibling(true), &doc));
    }
    if is("ПредыдущийСоседний", "PreviousSibling") {
        return Ok(opt_node(node.sibling(false), &doc));
    }
    if is("ДочерниеУзлы", "ChildNodes") {
        return Ok(list_value(
            DomListKind::Nodes,
            node.children.borrow().clone(),
            &doc,
        ));
    }
    if is("Атрибуты", "Attributes") {
        // Коллекция есть только у элемента — у документа и у текста
        // платформа отдаёт `Неопределено` (измерено оба).
        if node.kind != DomKind::Element {
            return Ok(BslValue::Undefined);
        }
        return Ok(list_value(
            DomListKind::Attributes,
            node.attrs.borrow().clone(),
            &doc,
        ));
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
        return Ok(opt_str(node.value.as_ref()));
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
            DomKind::Text | DomKind::Comment | DomKind::ProcessingInstruction => {
                Ok(opt_str(node.value.as_ref()))
            }
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
fn find_attribute(
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
            })
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
        Some(a) => opt_str(a.value.as_ref()),
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
            })
        }
    };
    let mut all = Vec::new();
    node.descendants(&mut all);
    all.retain(|e| e.name_matches(&wanted));
    Ok(list_value(DomListKind::Elements, all, &doc))
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

#[cfg(test)]
mod tests {
    use super::*;

    fn document_of(text: &str) -> Rc<DomNode> {
        let mut state = XmlReaderState::over(XmlParser::new(text));
        build(&mut state).expect("документ обязан построиться")
    }

    /// Корень ВМЕСТЕ с документом: документ возвращается, чтобы вызывающий
    /// держал его живым — ровно ту же работу в бою делает сильная ссылка
    /// внутри значения (см. [`node_value`]).
    fn root_of(text: &str) -> (Rc<DomNode>, Rc<DomNode>) {
        let doc = document_of(text);
        let root = document_element(&doc).expect("у документа обязан быть корень");
        (root, doc)
    }

    /// Значение узла. Документ берётся из самого узла — в тестах он жив,
    /// а там, где проверяется обратное, значение строится вручную.
    fn value(node: &Rc<DomNode>) -> BslValue {
        let doc = node
            .owner
            .borrow()
            .upgrade()
            .expect("документ узла обязан быть жив");
        node_value(node, &doc)
    }

    fn prop(node: &Rc<DomNode>, name: &str) -> BslValue {
        get_property(&value(node), name).expect("свойство обязано читаться")
    }

    /// Документ вложенности `depth` с текстом в самой глубине.
    fn nested(depth: usize) -> String {
        let mut text = String::new();
        for _ in 0..depth {
            text.push_str("<а>");
        }
        text.push('х');
        for _ in 0..depth {
            text.push_str("</а>");
        }
        text
    }

    fn text_of(v: &BslValue) -> String {
        match v {
            BslValue::Str(s) => s.to_string(),
            other => panic!("ожидалась строка, получено {other:?}"),
        }
    }

    #[test]
    fn dom_builds_the_element_tree() {
        let (root, _doc) = root_of("<а><б>текст</б><в/></а>");
        assert_eq!(root.name, "а");
        assert_eq!(root.children.borrow().len(), 2);
        assert_eq!(text_of(&prop(&root, "ТекстовоеСодержимое")), "текст");
        let first = root.children.borrow()[0].clone();
        assert_eq!(first.name, "б");
        assert_eq!(first.kind, DomKind::Element);
    }

    #[test]
    fn dom_navigation_links_parents_and_siblings() {
        let (root, _doc) = root_of("<а><б/><в/></а>");
        let first = root.children.borrow()[0].clone();
        let second = root.children.borrow()[1].clone();
        assert_eq!(text_of(&prop(&first, "ИмяУзла")), "б");
        let next = prop(&first, "СледующийСоседний");
        assert_eq!(next, value(&second));
        let back = prop(&second, "ПредыдущийСоседний");
        assert_eq!(back, value(&first));
        assert_eq!(prop(&first, "ПредыдущийСоседний"), BslValue::Undefined);
        assert_eq!(prop(&second, "СледующийСоседний"), BslValue::Undefined);
        let parent = prop(&first, "РодительскийУзел");
        assert_eq!(parent, value(&root));
    }

    #[test]
    fn dom_attributes_carry_namespaces() {
        let (root, _doc) = root_of(r#"<к:а xmlns:к="urn:к" п="1" к:б="2"/>"#);
        let attrs = root.attrs.borrow().clone();
        assert_eq!(attrs.len(), 3, "объявление xmlns тоже атрибут");
        assert_eq!(attrs[0].name, "xmlns:к");
        assert_eq!(attrs[0].uri, XMLNS_URI);
        assert_eq!(attrs[1].uri, "", "атрибут без префикса — без URI");
        assert_eq!(attrs[2].uri, "urn:к");
        assert_eq!(attrs[2].local_name(), "б");
        assert_eq!(text_of(&prop(&root, "URIПространстваИмен")), "urn:к");
        assert_eq!(text_of(&prop(&root, "Префикс")), "к");
        assert_eq!(text_of(&prop(&root, "ЛокальноеИмя")), "а");
    }

    #[test]
    fn dom_attribute_lookup_ignores_prefixed_names() {
        let (root, doc) = root_of(r#"<к:а xmlns:к="urn:к" п="1" к:б="2"/>"#);
        let el = node_value(&root, &doc);
        let one = |args: &[BslValue]| get_attribute(&el, args).unwrap();
        assert_eq!(one(&[str_value("п")]), str_value("1"));
        // Полное имя одним аргументом не находится — измерено.
        assert_eq!(one(&[str_value("к:б")]), BslValue::Undefined);
        assert_eq!(one(&[str_value("б")]), BslValue::Undefined);
        assert_eq!(one(&[str_value("urn:к"), str_value("б")]), str_value("2"));
        assert_eq!(
            has_attribute(&el, &[str_value("нет")]).unwrap(),
            BslValue::Boolean(false)
        );
        assert_eq!(
            get_attribute_node(&el, &[str_value("нет")]).unwrap(),
            BslValue::Undefined
        );
    }

    #[test]
    fn dom_text_merges_cdata_and_drops_blank_runs() {
        let (root, _doc) = root_of("<а>раз<![CDATA[два]]>три</а>");
        assert_eq!(root.children.borrow().len(), 1);
        assert_eq!(text_of(&prop(&root, "ТекстовоеСодержимое")), "раздватри");
        let (blank, _blank_doc) = root_of("<а>   </а>");
        assert!(blank.children.borrow().is_empty());
        let (around, _around_doc) = root_of("<а>  <б/>  хвост  </а>");
        assert_eq!(around.children.borrow().len(), 2);
        assert_eq!(
            text_of(&prop(&around.children.borrow()[1], "ЗначениеУзла")),
            "  хвост  "
        );
    }

    #[test]
    fn dom_keeps_comments_inside_the_root_only() {
        let doc = document_of("<!--до--><а>раз<!--в середине-->два</а><!--после-->");
        let kids = doc.children.borrow().clone();
        assert_eq!(kids.len(), 2, "комментарий до корня в дерево не идёт");
        assert_eq!(kids[0].kind, DomKind::Element);
        assert_eq!(kids[1].kind, DomKind::Comment);
        assert_eq!(kids[1].value.as_deref(), Some("после"));
        let root = kids[0].clone();
        assert_eq!(root.children.borrow().len(), 3, "комментарий рвёт текст");
        assert_eq!(text_of(&prop(&root, "ТекстовоеСодержимое")), "раздва");
    }

    #[test]
    fn dom_processing_instructions_survive_around_the_root() {
        let doc = document_of("<?пи данные?><а/>");
        let kids = doc.children.borrow().clone();
        assert_eq!(kids.len(), 2);
        assert_eq!(kids[0].kind, DomKind::ProcessingInstruction);
        assert_eq!(text_of(&prop(&kids[0], "ИмяУзла")), "пи");
        assert_eq!(text_of(&prop(&kids[0], "Цель")), "пи");
        assert_eq!(text_of(&prop(&kids[0], "Данные")), "данные");
        assert_eq!(prop(&kids[0], "ЗначениеУзла"), BslValue::Undefined);
        assert_eq!(prop(&kids[0], "ТекстовоеСодержимое"), BslValue::Undefined);
    }

    #[test]
    fn dom_search_matches_full_and_local_names() {
        let tree = document_of(r#"<к:а xmlns:к="urn:к"><а><к:а/></а></к:а>"#);
        let doc = node_value(&tree, &tree);
        let count = |args: &[BslValue]| match get_elements_by_name(&doc, args).unwrap() {
            BslValue::Object(o) => match &*o {
                BslObject::DomList(_, items, _) => items.len(),
                _ => panic!("ожидался список"),
            },
            other => panic!("ожидался список, получено {other:?}"),
        };
        assert_eq!(count(&[str_value("а")]), 3, "локальное имя у всех трёх");
        assert_eq!(count(&[str_value("к:а")]), 2);
        assert_eq!(count(&[str_value("*")]), 3);
        // URI не фильтрует — измерено.
        assert_eq!(count(&[str_value("urn:нет"), str_value("а")]), 3);
        assert_eq!(count(&[str_value("нет")]), 0);
    }

    #[test]
    fn dom_build_continues_from_the_readers_position() {
        let mut state = XmlReaderState::over(XmlParser::new(r#"<а х="1">раз<б/>два</а>"#));
        // Два шага читателя: он стоит на тексте «раз».
        for _ in 0..2 {
            let parser = state.parser.as_mut().unwrap();
            let e = parser.read().unwrap();
            state.current = e;
        }
        let doc = build(&mut state).unwrap();
        let root = document_element(&doc).unwrap();
        assert_eq!(root.name, "а", "предок восстановлен из стека читателя");
        assert_eq!(root.attrs.borrow().len(), 1, "вместе с атрибутами");
        assert_eq!(
            text_of(&prop(&root, "ТекстовоеСодержимое")),
            "два",
            "уже прочитанный текст в дерево не попадает"
        );
    }

    #[test]
    fn dom_exhausted_reader_gives_an_empty_document() {
        let mut state = XmlReaderState::over(XmlParser::new("<а/>"));
        while let Some(e) = state.parser.as_mut().unwrap().read().unwrap() {
            state.current = Some(e);
        }
        let doc = build(&mut state).unwrap();
        assert!(doc.children.borrow().is_empty());
        assert!(document_element(&doc).is_none());
    }

    #[test]
    fn dom_broken_input_is_an_error() {
        let mut state = XmlReaderState::over(XmlParser::new("<а><б></а>"));
        assert!(build(&mut state).is_err());
        let mut empty = XmlReaderState::over(XmlParser::new(""));
        assert!(build(&mut empty).is_err());
        let mut two = XmlReaderState::over(XmlParser::new("<а/><б/>"));
        assert!(build(&mut two).is_err());
        // Читатель без источника — тоже ошибка, а не пустой документ.
        let mut none = XmlReaderState::default();
        assert!(build(&mut none).is_err());
    }

    #[test]
    fn dom_attribute_value_is_a_text_child() {
        let (root, _doc) = root_of(r#"<а п="1"/>"#);
        let attr = root.attrs.borrow()[0].clone();
        assert_eq!(attr.children.borrow().len(), 1);
        assert_eq!(text_of(&prop(&attr, "ЗначениеУзла")), "1");
        assert_eq!(text_of(&prop(&attr, "ТекстовоеСодержимое")), "1");
        assert_eq!(prop(&attr, "РодительскийУзел"), BslValue::Undefined);
        assert_eq!(prop(&attr, "ЭлементВладелец"), value(&root));
        assert_eq!(prop(&attr, "СледующийСоседний"), BslValue::Undefined);
    }

    #[test]
    fn dom_document_members_match_the_platform() {
        let doc = document_of("<а/>");
        assert_eq!(text_of(&prop(&doc, "ИмяУзла")), "#document");
        assert_eq!(prop(&doc, "ЗначениеУзла"), BslValue::Undefined);
        assert_eq!(prop(&doc, "ТекстовоеСодержимое"), BslValue::Undefined);
        assert_eq!(text_of(&prop(&doc, "ВерсияXML")), "1.0");
        assert_eq!(prop(&doc, "РодительскийУзел"), BslValue::Undefined);
        assert_eq!(prop(&doc, "Атрибуты"), BslValue::Undefined);
        let root = document_element(&doc).unwrap();
        assert_eq!(prop(&doc, "ЭлементДокумента"), value(&root));
        assert_eq!(prop(&root, "ДокументВладелец"), value(&doc));
    }

    #[test]
    fn dom_nodes_compare_by_identity() {
        let (root, _doc) = root_of("<а><б/></а>");
        let child = value(&root.children.borrow()[0]);
        assert_eq!(prop(&root, "ПервыйДочерний"), child);
        assert_ne!(prop(&root, "ДочерниеУзлы"), prop(&root, "ДочерниеУзлы"));
        assert_ne!(value(&root), child);
    }

    /// Сильная ссылка на документ внутри значения тождества не трогает:
    /// оно по-прежнему по адресу УЗЛА, значит и хэш обязан совпадать —
    /// иначе такой узел терялся бы ключом `Соответствие`.
    #[test]
    fn dom_node_values_of_one_node_are_equal_and_hash_alike() {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        fn hash_of(v: &BslValue) -> u64 {
            let mut h = DefaultHasher::new();
            v.hash(&mut h);
            h.finish()
        }

        let (root, doc) = root_of("<а><б/></а>");
        let one = node_value(&root, &doc);
        let two = node_value(&root, &doc);
        assert_eq!(one, two);
        assert_eq!(hash_of(&one), hash_of(&two));
        // И то же самое для узла, добытого навигацией, а не напрямую.
        let via_property = prop(&root, "ПервыйДочерний");
        let via_list = match prop(&root, "ДочерниеУзлы") {
            BslValue::Object(o) => match &*o {
                BslObject::DomList(_, items, list_doc) => node_value(&items[0], list_doc),
                _ => panic!("ожидался список"),
            },
            other => panic!("ожидался список, получено {other:?}"),
        };
        assert_eq!(via_property, via_list);
        assert_eq!(hash_of(&via_property), hash_of(&via_list));
    }

    /// Узел обязан пережить переменную с документом: в BSL идиома
    /// `Возврат Док.ЭлементДокумента` возвращает корень, а документ на
    /// выходе из функции умирает. Ссылки вверх по дереву слабые, поэтому
    /// живым дерево держит только сильная ссылка внутри значения.
    #[test]
    fn dom_node_value_outlives_the_document_variable() {
        let root_value = {
            let doc = document_of(r#"<а х="1"><б>т</б></а>"#);
            get_property(&node_value(&doc, &doc), "ЭлементДокумента")
                .expect("ЭлементДокумента обязан читаться")
        };
        // Переменной с документом больше нет — навигация обязана работать.
        assert_eq!(text_of(&get_property(&root_value, "ИмяУзла").unwrap()), "а");
        let parent = get_property(&root_value, "РодительскийУзел").unwrap();
        assert_eq!(
            text_of(&get_property(&parent, "ИмяУзла").unwrap()),
            DOCUMENT_NODE_NAME
        );
        let owner = get_property(&root_value, "ДокументВладелец").unwrap();
        assert_eq!(
            text_of(&get_property(&owner, "ИмяУзла").unwrap()),
            DOCUMENT_NODE_NAME
        );
        // Дети и атрибуты — тоже: у атрибута жив его `ЭлементВладелец`.
        let kids = get_property(&root_value, "ДочерниеУзлы").unwrap();
        assert_eq!(kids.collection_len().unwrap(), 1);
        let attr = get_attribute_node(&root_value, &[str_value("х")]).unwrap();
        assert_eq!(
            get_property(&attr, "ЭлементВладелец").unwrap(),
            root_value,
            "элемент-владелец — тот же узел, что и корень"
        );
    }

    /// Глубина ровно по пределу обязана и строиться, и обходиться, и
    /// РАЗРУШАТЬСЯ: разрушение рекурсивно, так что в debug-сборке этот тест
    /// и есть проверка того, что 500 уровней стек держит.
    #[test]
    fn dom_document_at_the_depth_limit_still_works() {
        let mut state = XmlReaderState::over(XmlParser::new(&nested(MAX_DOM_DEPTH)));
        let doc = build(&mut state).expect("глубина ровно по пределу обязана строиться");
        let root = document_element(&doc).expect("у документа обязан быть корень");
        let mut all = Vec::new();
        root.descendants(&mut all);
        assert_eq!(all.len(), MAX_DOM_DEPTH - 1, "корень себя не включает");
        assert_eq!(root.text_content(), "х");
        // Порядок важен: пока `all` и `root` живы, дерево разрушается по
        // кусочкам, а рекурсивный `Drop` во всю глубину случается только
        // на последней сильной ссылке — на документе.
        drop(all);
        drop(root);
        drop(doc);
    }

    /// Глубже предела — перехватываемая ошибка, а НЕ переполнение стека
    /// процесса: `Попытка` обязана такой документ поймать.
    #[test]
    fn dom_document_deeper_than_the_limit_is_an_error_not_a_crash() {
        let mut state = XmlReaderState::over(XmlParser::new(&nested(MAX_DOM_DEPTH + 1)));
        let err = build(&mut state).expect_err("глубже предела — ошибка");
        assert!(
            matches!(err, RtError::StackOverflow { .. }),
            "ожидалась StackOverflow, получено {err:?}"
        );
        // Тот же предел действует и на предков, восстановленных из стека
        // читателя: дерево «с глубины» второго пути в обход не даёт.
        let deep = nested(MAX_DOM_DEPTH + 1);
        let mut from_inside = XmlReaderState::over(XmlParser::new(&deep));
        for _ in 0..=MAX_DOM_DEPTH {
            let parser = from_inside.parser.as_mut().unwrap();
            from_inside.current = parser.read().unwrap();
        }
        let err = build(&mut from_inside).expect_err("предки глубже предела — тоже ошибка");
        assert!(matches!(err, RtError::StackOverflow { .. }));
    }
}
