//! Дерево DOM: узлы, их виды и навигация.

use super::*;

/// URI, который платформа приписывает объявлениям пространств имён
/// (измерено: `xmlns:к` отдаёт именно это). Значение из спецификации XML
/// Namespaces, но здесь оно ИЗМЕРЕНО, а не процитировано.
pub(crate) const XMLNS_URI: &str = "http://www.w3.org/2000/xmlns/";

/// Вид узла. Ровно те виды, которые умеют появиться в дереве: РАЗБОР секции
/// CDATA не создаёт (она вливается в текст — измерено), но
/// `СоздатьСекциюCDATA` создаёт, поэтому вид у неё свой. Определения типа
/// документа и фрагмента здесь нет по причинам из заголовка модуля.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DomKind {
    Document,
    Element,
    Attribute,
    Text,
    CdataSection,
    Comment,
    ProcessingInstruction,
    /// Ссылка на объявленную сущность: `<к>раз&е;два</к>` при объявленной
    /// `е` даёт троих детей, средний из которых — этот узел (измерено).
    EntityReference,
}

impl DomKind {
    /// Член `ТипУзлаDOM`, который отдаёт свойство `ТипУзла`.
    pub(crate) fn node_type(self) -> EnumValue {
        match self {
            DomKind::Document => EnumValue::DomDocument,
            DomKind::Element => EnumValue::DomElement,
            DomKind::Attribute => EnumValue::DomAttribute,
            DomKind::Text => EnumValue::DomText,
            DomKind::CdataSection => EnumValue::DomCdataSection,
            DomKind::Comment => EnumValue::DomComment,
            DomKind::ProcessingInstruction => EnumValue::DomProcessingInstruction,
            DomKind::EntityReference => EnumValue::DomEntityReference,
        }
    }

    /// Текстоподобен ли узел: даёт ли он вклад в `ТекстовоеСодержимое` и
    /// запрещён ли вне элемента при записи. ИЗМЕРЕНО, что секция CDATA в
    /// склейку входит (`раз` + CDATA `два` дают «раздва») и что записать её
    /// вне элемента — ошибка, ровно как текст.
    pub(crate) fn is_text_like(self) -> bool {
        matches!(self, DomKind::Text | DomKind::CdataSection)
    }
}

/// Узел дерева.
///
/// Дети, атрибуты и значение лежат в `RefCell`: дерево складывается на месте
/// при разборе, а после разбора его меняют фабрики и мутирующие методы
/// документа. Ссылки вверх — слабые: иначе документ с детьми образовал бы
/// цикл `Rc` и не освобождался бы никогда.
#[derive(Debug)]
pub struct DomNode {
    pub(crate) kind: DomKind,
    /// `ИмяУзла`: полное имя элемента или атрибута, `#document`, `#text`,
    /// `#cdata-section`, `#comment` либо цель инструкции обработки. Имя
    /// неизменяемо: присвоить `ИмяУзла` платформа не даёт (измерено).
    pub(crate) name: String,
    /// `ЛокальноеИмя` и `Префикс` — ХРАНИМЫЕ, а не выведенные из имени: у
    /// узлов без пространства имён платформа отдаёт совсем не то, что дало бы
    /// расщепление имени по двоеточию (см. заголовок модуля).
    pub(crate) local: String,
    pub(crate) prefix: String,
    /// `URIПространстваИмен`; пусто, если пространства имён нет.
    pub(crate) uri: String,
    /// `ЗначениеУзла`/`Данные`: текст, секция CDATA, комментарий, данные
    /// инструкции обработки. У документа, элемента и АТРИБУТА значения здесь
    /// нет: значение атрибута — это склейка его текстовых детей (измерено).
    pub(crate) value: RefCell<Option<String>>,
    pub(crate) children: RefCell<Vec<Rc<DomNode>>>,
    pub(crate) attrs: RefCell<Vec<Rc<DomNode>>>,
    /// Родитель по дереву. У атрибута здесь лежит ВЛАДЕЮЩИЙ ЭЛЕМЕНТ, но
    /// наружу как `РодительскийУзел` он не показывается: платформа отдаёт
    /// у атрибута `Неопределено` (измерено).
    pub(crate) parent: RefCell<Weak<DomNode>>,
    pub(crate) owner: RefCell<Weak<DomNode>>,
    /// `ВерсияXML` — заполняется только у документа, из объявления.
    pub(crate) xml_version: RefCell<String>,
}

impl DomNode {
    /// Узел, каким его делает РАЗБОР: локальное имя и префикс расщеплением
    /// полного имени, как их отдаёт платформа у разобранного дерева.
    pub(crate) fn new(
        kind: DomKind,
        name: String,
        uri: String,
        value: Option<String>,
    ) -> Rc<DomNode> {
        let (local, prefix) = match kind {
            DomKind::Element | DomKind::Attribute => {
                (local_of(&name).to_string(), prefix_of(&name).to_string())
            }
            _ => (String::new(), String::new()),
        };
        DomNode::raw(kind, name, local, prefix, uri, value)
    }

    pub(crate) fn raw(
        kind: DomKind,
        name: String,
        local: String,
        prefix: String,
        uri: String,
        value: Option<String>,
    ) -> Rc<DomNode> {
        Rc::new(DomNode {
            kind,
            name,
            local,
            prefix,
            uri,
            value: RefCell::new(value),
            children: RefCell::new(Vec::new()),
            attrs: RefCell::new(Vec::new()),
            parent: RefCell::new(Weak::new()),
            owner: RefCell::new(Weak::new()),
            xml_version: RefCell::new(String::from("1.0")),
        })
    }

    pub fn kind(&self) -> DomKind {
        self.kind
    }

    /// Разбор схемы (`xsd.rs`) читает готовое дерево, а поля узла закрыты
    /// модулем — вот доступ ровно к тому, что ему нужно: имя, URI, дети,
    /// значение атрибута и подъём к родителю (для поиска объявлений
    /// `xmlns` в области видимости). Только чтение: модель схемы дерево не
    /// меняет.
    pub fn xs_local_name(&self) -> &str {
        &self.local
    }

    /// `URIПространстваИмен` узла: по нему `xsd.rs` и узнаёт элементы
    /// схемы — по пространству имён, а НЕ по префиксу `xs:`.
    pub fn xs_uri(&self) -> &str {
        &self.uri
    }

    pub fn xs_children(&self) -> Vec<Rc<DomNode>> {
        self.children.borrow().clone()
    }

    /// Значение атрибута с пустым пространством имён по локальному имени —
    /// именно так записаны все атрибуты XSD (`name`, `type`, `use`).
    pub fn xs_attribute(&self, local: &str) -> Option<String> {
        self.attrs
            .borrow()
            .iter()
            .find(|a| a.uri.is_empty() && a.local == local)
            .map(|a| a.attr_value())
    }

    /// URI объявления пространства имён, действующего на этом узле:
    /// `xmlns:префикс` при непустом префиксе и `xmlns` при пустом.
    pub fn xs_namespace_declaration(&self, prefix: &str) -> Option<String> {
        self.attrs
            .borrow()
            .iter()
            .find(|a| {
                a.uri == XMLNS_URI
                    && if prefix.is_empty() {
                        a.name == "xmlns"
                    } else {
                        a.prefix == "xmlns" && a.local == prefix
                    }
            })
            .map(|a| a.attr_value())
    }

    pub fn xs_parent(&self) -> Option<Rc<DomNode>> {
        self.parent.borrow().upgrade()
    }

    /// Доступ для вычислителя XPath (`xpath.rs`). Ему дерево нужно только
    /// на чтение, и нужно ему то, чего разбору схемы не требовалось: полное
    /// имя, атрибуты и строковое значение узла. Общее (локальное имя, URI,
    /// дети, родитель) он берёт теми же `xs_*` — второе имя на тот же
    /// геттер только развело бы их со временем.
    pub(crate) fn xp_node_name(&self) -> &str {
        &self.name
    }

    pub(crate) fn xp_attributes(&self) -> Vec<Rc<DomNode>> {
        self.attrs.borrow().clone()
    }

    /// Объявление ли это пространства имён (`xmlns` либо `xmlns:префикс`).
    /// В дереве такие объявления — обычные атрибуты (измерено), но на оси
    /// `attribute::` их у платформы НЕТ: `/*/@*` у корня с двумя
    /// объявлениями и одним обычным атрибутом отдаёт ровно один узел.
    pub(crate) fn xp_is_namespace_declaration(&self) -> bool {
        self.kind == DomKind::Attribute && (self.uri == XMLNS_URI || self.name == "xmlns")
    }

    /// Строковое значение узла в смысле XPath: у элемента и документа —
    /// склейка текстовых потомков, у атрибута — его значение, у текста,
    /// комментария и инструкции обработки — их собственные данные. Всё
    /// четыре случая измерены (`string(/)`, `string(.)` от атрибута,
    /// `string(//comment())`, `string(//processing-instruction())`).
    pub(crate) fn xp_string_value(&self) -> String {
        match self.kind {
            DomKind::Document | DomKind::Element => self.text_content(),
            DomKind::Attribute => self.attr_value(),
            DomKind::Text | DomKind::CdataSection | DomKind::Comment => {
                self.value.borrow().clone().unwrap_or_default()
            }
            DomKind::ProcessingInstruction => self.value.borrow().clone().unwrap_or_default(),
            // У ссылки на сущность собственного текста нет: в
            // `ТекстовоеСодержимое` родителя она не даёт ничего (измерено —
            // `<к>раз&е;два</к>` отдаёт «раздва»).
            DomKind::EntityReference => String::new(),
        }
    }

    /// Добавить ребёнка, проставив ему родителя и документ.
    pub(crate) fn append(parent: &Rc<DomNode>, child: &Rc<DomNode>, doc: &Rc<DomNode>) {
        *child.parent.borrow_mut() = Rc::downgrade(parent);
        *child.owner.borrow_mut() = Rc::downgrade(doc);
        parent.children.borrow_mut().push(child.clone());
    }

    /// `ЛокальноеИмя`.
    pub(crate) fn local_name(&self) -> &str {
        &self.local
    }

    /// `Префикс`.
    pub(crate) fn prefix(&self) -> &str {
        &self.prefix
    }

    /// `ТекстовоеСодержимое`: склейка ТЕКСТОВЫХ потомков. Комментарии в
    /// неё не входят (измерено: `<ком>раз<!--к-->два</ком>` даёт «раздва»),
    /// а секции CDATA входят (измерено).
    pub(crate) fn text_content(&self) -> String {
        let mut out = String::new();
        self.collect_text(&mut out);
        out
    }

    pub(crate) fn collect_text(&self, out: &mut String) {
        if self.kind.is_text_like()
            && let Some(v) = self.value.borrow().as_ref()
        {
            out.push_str(v);
        }
        for c in self.children.borrow().iter() {
            c.collect_text(out);
        }
    }

    /// `ЗначениеУзла`/`Значение` у атрибута — склейка его текстовых детей
    /// (измерено: у созданного атрибута детей ноль и значение пусто, а
    /// дописанный текстовый ребёнок дописывает и значение).
    pub(crate) fn attr_value(&self) -> String {
        self.text_content()
    }

    /// Заменить всех детей одним текстовым узлом; пустая строка оставляет
    /// узел вовсе без детей (измерено на `ТекстовоеСодержимое`).
    pub(crate) fn set_text_children(node: &Rc<DomNode>, text: &str, doc: &Rc<DomNode>) {
        Self::detach_children(node);
        if text.is_empty() {
            return;
        }
        let child = DomNode::new(
            DomKind::Text,
            TEXT_NODE_NAME.to_string(),
            String::new(),
            Some(text.to_string()),
        );
        DomNode::append(node, &child, doc);
    }

    /// Отцепить всех детей: родителя им обнулить, список очистить.
    pub(crate) fn detach_children(node: &Rc<DomNode>) {
        let old = std::mem::take(&mut *node.children.borrow_mut());
        for child in old {
            *child.parent.borrow_mut() = Weak::new();
        }
    }

    /// Номер узла среди детей его родителя. Ищется каждый раз, а не
    /// хранится: мутация переставляет детей, и хранимый номер устарел бы
    /// молча — в отличие от поиска, который не может разойтись с деревом.
    pub(crate) fn index_in(node: &Rc<DomNode>, parent: &Rc<DomNode>) -> Option<usize> {
        parent
            .children
            .borrow()
            .iter()
            .position(|c| Rc::ptr_eq(c, node))
    }

    /// Элементы-потомки в порядке документа. Сам узел в результат не
    /// входит: у элемента платформа его не возвращает (измерено), а у
    /// документа корневой элемент — потомок, и он попадает сюда сам.
    pub(crate) fn descendants(&self, out: &mut Vec<Rc<DomNode>>) {
        for c in self.children.borrow().iter() {
            if c.kind == DomKind::Element {
                out.push(c.clone());
            }
            c.descendants(out);
        }
    }

    /// Совпадает ли имя элемента с запрошенным. ИЗМЕРЕНО: сопоставляется
    /// И полное имя, И локальное, а `*` берёт любое.
    pub(crate) fn name_matches(&self, wanted: &str) -> bool {
        wanted == "*" || self.name == wanted || self.local_name() == wanted
    }

    pub(crate) fn sibling(node: &Rc<DomNode>, forward: bool) -> Option<Rc<DomNode>> {
        // У атрибута соседей нет: он не лежит среди детей своего элемента,
        // и `РодительскийУзел` у него `Неопределено` (измерено).
        if node.kind == DomKind::Attribute {
            return None;
        }
        let parent = node.parent.borrow().upgrade()?;
        let i = DomNode::index_in(node, &parent)?;
        let kids = parent.children.borrow();
        let j = if forward {
            i.checked_add(1)?
        } else {
            i.checked_sub(1)?
        };
        kids.get(j).cloned()
    }
}
