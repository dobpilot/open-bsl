//! Разбор дерева DOM в модель схемы.

use super::*;

// --- разбор --------------------------------------------------------------

pub(crate) fn unsupported(construct: &str) -> RtError {
    RtError::Xsd(format!(
        "конструкция «xs:{construct}» в схеме XML не поддерживается"
    ))
}

/// Разрешить QName по объявлениям `xmlns`, действующим на узле.
///
/// Возвращает `None` там же, где платформа отдаёт `Неопределено`:
/// префикс не объявлен, либо префикса нет и нет пространства имён по
/// умолчанию (измерено обе ветки).
pub(crate) fn resolve_qname(elem: &Rc<DomNode>, qname: &str) -> Option<XName> {
    let (prefix, local) = match qname.split_once(':') {
        Some((p, l)) => (p, l),
        None => ("", qname),
    };
    if local.is_empty() {
        return None;
    }
    let mut cur = Some(elem.clone());
    while let Some(node) = cur {
        if let Some(uri) = node.xs_namespace_declaration(prefix) {
            // Отменённое объявление (`xmlns=""`) — это отсутствие
            // пространства имён по умолчанию.
            if uri.is_empty() && prefix.is_empty() {
                return None;
            }
            return Some(XName {
                uri,
                local: local.to_string(),
            });
        }
        cur = node.xs_parent();
    }
    None
}

/// Элементы-дети узла (текст, комментарии и инструкции обработки
/// отбрасываются — платформа их в схеме игнорирует).
pub(crate) fn child_elements(elem: &Rc<DomNode>) -> Vec<Rc<DomNode>> {
    elem.xs_children()
        .into_iter()
        .filter(|c| c.kind() == DomKind::Element)
        .collect()
}

/// Дети из пространства имён схемы. Всё остальное — чужое пространство
/// имён — платформа пропускает (измерено).
pub(crate) fn xsd_children(elem: &Rc<DomNode>) -> Vec<Rc<DomNode>> {
    child_elements(elem)
        .into_iter()
        .filter(|c| c.xs_uri() == XSD_NS)
        .collect()
}

/// Состояние разбора: массив узлов плюс то, что нужно знать про схему
/// целиком (целевое пространство имён и формы по умолчанию).
pub(crate) struct Parser {
    pub(crate) nodes: Vec<XsNode>,
    pub(crate) target_ns: String,
    pub(crate) element_form_qualified: bool,
    pub(crate) attribute_form_qualified: bool,
}

impl Parser {
    pub(crate) fn push(&mut self, node: XsNode) -> usize {
        self.nodes.push(node);
        self.nodes.len() - 1
    }

    pub(crate) fn add_child(&mut self, parent: usize, child: usize) {
        self.nodes[parent].children.push(child);
        self.nodes[child].parent = Some(parent);
    }

    /// Пометить БЛИЖАЙШИЙ объемлющий составной тип как несущий маску.
    ///
    /// Подъём по родителям, а не пометка на месте, потому что `xs:any`
    /// живёт в группе модели, и группа бывает вложенной: у типа `DeepAny`
    /// из `measure-xdto-order.bsl` маска лежит во внутренней
    /// `xs:sequence`, и платформа всё равно считает тип открытым
    /// (`XDTO.WRITE_ORDER.INHERIT`). Родители к этому моменту уже
    /// проставлены: [`Parser::add_child`] зовётся до разбора потомков.
    ///
    /// Объемлющего составного типа может и не быть — тогда помечать
    /// нечего, и это не ошибка.
    pub(crate) fn mark_wildcard(&mut self, from: usize) {
        let mut at = Some(from);
        while let Some(i) = at {
            if let XsData::ComplexType(d) = &mut self.nodes[i].data {
                d.has_wildcard = true;
                return;
            }
            at = self.nodes[i].parent;
        }
    }

    /// Пространство имён объявления: у глобального — целевое, у
    /// локального — целевое только при квалифицированной форме.
    pub(crate) fn declaration_ns(
        &self,
        global: bool,
        form: Option<EnumValue>,
        element: bool,
    ) -> String {
        let qualified = match form {
            Some(EnumValue::XsFormQualified) => true,
            Some(_) => false,
            None if element => self.element_form_qualified,
            None => self.attribute_form_qualified,
        };
        if global || qualified {
            self.target_ns.clone()
        } else {
            String::new()
        }
    }

    pub(crate) fn form_of(elem: &Rc<DomNode>) -> Option<EnumValue> {
        match elem.xs_attribute("form").as_deref() {
            Some("qualified") => Some(EnumValue::XsFormQualified),
            Some("unqualified") => Some(EnumValue::XsFormUnqualified),
            _ => None,
        }
    }

    /// `default`/`fixed` объявления: текст и вид ограничения.
    pub(crate) fn value_constraint(elem: &Rc<DomNode>) -> (String, Option<EnumValue>) {
        if let Some(v) = elem.xs_attribute("default") {
            return (v, Some(EnumValue::XsConstraintDefault));
        }
        if let Some(v) = elem.xs_attribute("fixed") {
            return (v, Some(EnumValue::XsConstraintFixed));
        }
        (String::new(), None)
    }

    pub(crate) fn boolean_attribute(elem: &Rc<DomNode>, name: &str) -> Option<bool> {
        // Лексические значения `xs:boolean`: платформа принимает и слова, и
        // цифры — измерены все четыре написания (пробы `абстр Абстрактный`
        // -> `abstract="true"` -> «Да», `mixed словом Ложь` ->
        // `mixed="false"` -> «Нет», `abstract цифрой` -> `abstract="1"` ->
        // «Да», `mixed цифрой` -> `mixed="0"` -> «Нет»).
        match elem.xs_attribute(name).as_deref() {
            Some("true") | Some("1") => Some(true),
            Some("false") | Some("0") => Some(false),
            _ => None,
        }
    }

    /// Граница вхождения частицы (`minOccurs`/`maxOccurs`) в том виде, в
    /// каком её хранит платформа: беззнаковое 32-битное число, в котором
    /// `unbounded` — это `u32::MAX`.
    ///
    /// Правило целиком снято на 8.3.27 (пробы `час0 …`, `выбор0 …`,
    /// `границы числом`, `границы мусором`, `границы наоборот` и `границы
    /// через край` в `measure-xsd.bsl`): `minOccurs="2"` -> 2,
    /// `maxOccurs="5"` -> 5, `minOccurs="0"` -> 0, `minOccurs="007"` -> 7,
    /// `minOccurs="+3"` -> 3, `maxOccurs=" 5 "` -> 5 (пробелы по краям
    /// отбрасываются), `minOccurs="-1"` -> 4294967295,
    /// `minOccurs="4294967296"` -> 0, `maxOccurs="99999999999999999999"` ->
    /// 1661992959 — это 10^20 - 1 по модулю 2^32. Запись, которая целым
    /// числом не является (`minOccurs="много"`, `maxOccurs=""`), границы не
    /// задаёт вовсе, как и отсутствующий атрибут: обе дают `Неопределено`.
    ///
    /// Отсюда и разбор: десятичное целое со знаком, накопленное ПО МОДУЛЮ
    /// 2^32, а всё остальное — `None`.
    pub(crate) fn occurs_attribute(elem: &Rc<DomNode>, name: &str) -> Option<u32> {
        let raw = elem.xs_attribute(name)?;
        let text = raw.trim();
        if text == "unbounded" {
            return Some(u32::MAX);
        }
        let (negative, digits) = match text.strip_prefix('-') {
            Some(rest) => (true, rest),
            None => (false, text.strip_prefix('+').unwrap_or(text)),
        };
        if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        let mut value: u32 = 0;
        for b in digits.bytes() {
            value = value.wrapping_mul(10).wrapping_add(u32::from(b - b'0'));
        }
        Some(if negative {
            value.wrapping_neg()
        } else {
            value
        })
    }

    pub(crate) fn new_node(
        kind: XsKind,
        name: String,
        ns: String,
        dom: &Rc<DomNode>,
        data: XsData,
    ) -> XsNode {
        XsNode {
            kind,
            parent: None,
            name,
            ns,
            children: Vec::new(),
            dom: Some(dom.clone()),
            data,
        }
    }

    /// `xs:annotation` — единственная конструкция, у которой нет
    /// собственной семантики схемы, поэтому её содержимое просто
    /// раскладывается на документацию и информацию приложения.
    pub(crate) fn parse_annotation(&mut self, elem: &Rc<DomNode>, owner: usize) -> RtResult<usize> {
        let idx = self.push(Self::new_node(
            XsKind::Annotation,
            String::new(),
            String::new(),
            elem,
            XsData::Annotation {
                documentation: Vec::new(),
                appinfo: Vec::new(),
            },
        ));
        self.add_child(owner, idx);
        for child in xsd_children(elem) {
            let kind = match child.xs_local_name() {
                "documentation" => XsKind::Documentation,
                "appinfo" => XsKind::AppInfo,
                // Незнакомое внутри аннотации — пропускаем, как и
                // незнакомое внутри схемы.
                _ => continue,
            };
            let node = Self::new_node(
                kind,
                String::new(),
                String::new(),
                &child,
                XsData::Documentation {
                    // `xml:lang` лежит в дереве с локальным именем `lang`:
                    // префикс `xml` разбор ни к какому URI не привязывает,
                    // поэтому атрибут находится обычным поиском (измерено,
                    // что платформа отдаёт `Язык` = «ru» именно для
                    // `xml:lang="ru"`).
                    lang: child.xs_attribute("lang").unwrap_or_default(),
                    source: child.xs_attribute("source").unwrap_or_default(),
                },
            );
            let child_idx = self.push(node);
            self.add_child(idx, child_idx);
            match kind {
                XsKind::Documentation => {
                    if let XsData::Annotation { documentation, .. } = &mut self.nodes[idx].data {
                        documentation.push(child_idx);
                    }
                }
                XsKind::AppInfo => {
                    if let XsData::Annotation { appinfo, .. } = &mut self.nodes[idx].data {
                        appinfo.push(child_idx);
                    }
                }
                _ => unreachable!("вид выбран match'ем выше"),
            }
        }
        Ok(idx)
    }

    /// Объявление элемента: и глобальное (ребёнок схемы), и локальное
    /// (терм фрагмента).
    pub(crate) fn parse_element_declaration(
        &mut self,
        elem: &Rc<DomNode>,
        owner: usize,
        global: bool,
    ) -> RtResult<Option<usize>> {
        let name = elem.xs_attribute("name").unwrap_or_default();
        let reference = elem
            .xs_attribute("ref")
            .and_then(|r| resolve_qname(elem, &r));
        // Объявление без имени и без ссылки платформа пропускает молча
        // (измерено на `<xs:element type="xs:string"/>`).
        if name.is_empty() && reference.is_none() {
            return Ok(None);
        }
        let form = Self::form_of(elem);
        let (lexical, constraint) = Self::value_constraint(elem);
        let ns = if reference.is_some() {
            // У объявления-ссылки платформа отдаёт пустое имя и пустое
            // пространство имён: ссылка живёт отдельно, в `Ссылка`.
            String::new()
        } else {
            self.declaration_ns(global, form, true)
        };
        let idx = self.push(Self::new_node(
            XsKind::Element,
            name,
            ns,
            elem,
            XsData::Element(DeclData {
                type_name: elem
                    .xs_attribute("type")
                    .and_then(|t| resolve_qname(elem, &t)),
                reference,
                anonymous_type: None,
                form,
                is_abstract: Self::boolean_attribute(elem, "abstract"),
                lexical,
                constraint,
                global,
            }),
        ));
        self.add_child(owner, idx);
        self.parse_declaration_children(elem, idx)?;
        Ok(Some(idx))
    }

    /// Общее у объявлений элемента и атрибута: аннотация и встроенный тип.
    pub(crate) fn parse_declaration_children(
        &mut self,
        elem: &Rc<DomNode>,
        idx: usize,
    ) -> RtResult<()> {
        for child in xsd_children(elem) {
            match child.xs_local_name() {
                "annotation" => {
                    self.parse_annotation(&child, idx)?;
                }
                "simpleType" | "complexType" => {
                    let anon = if child.xs_local_name() == "simpleType" {
                        self.parse_simple_type(&child, idx, false)?
                    } else {
                        self.parse_complex_type(&child, idx, false)?
                    };
                    let slot = match &mut self.nodes[idx].data {
                        XsData::Element(d) | XsData::Attribute(d) => &mut d.anonymous_type,
                        _ => unreachable!("вызывается только для объявлений"),
                    };
                    // Если типов записано два, платформа держит ПЕРВЫЙ:
                    // измерено на `<xs:element name="а">` с `simpleType` и
                    // `complexType` подряд (проба `элемент с двумя
                    // вложенными типами`) — `АнонимноеОпределениеТипа` там
                    // «Определение простого типа XML Schema», а
                    // `Компоненты` объявления при этом два: оба типа
                    // остаются компонентами, выбор касается только
                    // `АнонимноеОпределениеТипа`.
                    if slot.is_none() {
                        *slot = Some(anon);
                    }
                }
                "unique" | "key" | "keyref" => {
                    return Err(unsupported(child.xs_local_name()));
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// Объявление атрибута. Ссылка (`ref`) устроена как у элемента:
    /// объявление с ПУСТЫМ именем, а сама ссылка — в `Ссылка` (измерено:
    /// `<xs:attribute ref="t:а"/>` даёт использование атрибута, у чьего
    /// объявления `Имя` пусто).
    pub(crate) fn parse_attribute_declaration(
        &mut self,
        elem: &Rc<DomNode>,
        owner: usize,
        global: bool,
    ) -> RtResult<Option<usize>> {
        let name = elem.xs_attribute("name").unwrap_or_default();
        let reference = elem
            .xs_attribute("ref")
            .and_then(|r| resolve_qname(elem, &r));
        if name.is_empty() && reference.is_none() {
            return Ok(None);
        }
        let form = Self::form_of(elem);
        let (lexical, constraint) = Self::value_constraint(elem);
        let ns = if reference.is_some() {
            String::new()
        } else {
            self.declaration_ns(global, form, false)
        };
        let idx = self.push(Self::new_node(
            XsKind::Attribute,
            name,
            ns,
            elem,
            XsData::Attribute(DeclData {
                type_name: elem
                    .xs_attribute("type")
                    .and_then(|t| resolve_qname(elem, &t)),
                reference,
                anonymous_type: None,
                form,
                is_abstract: None,
                lexical,
                // У ЛОКАЛЬНОГО объявления вид ограничения платформа не
                // показывает — его несёт использование атрибута (измерено).
                constraint: if global { constraint } else { None },
                global,
            }),
        ));
        self.add_child(owner, idx);
        self.parse_declaration_children(elem, idx)?;
        Ok(Some(idx))
    }

    /// `xs:attribute` внутри составного типа — это ИСПОЛЬЗОВАНИЕ атрибута,
    /// внутри которого лежит объявление.
    pub(crate) fn parse_attribute_use(
        &mut self,
        elem: &Rc<DomNode>,
        owner: usize,
    ) -> RtResult<Option<usize>> {
        let (lexical, constraint) = Self::value_constraint(elem);
        let required = elem.xs_attribute("use").as_deref() == Some("required");
        let idx = self.push(Self::new_node(
            XsKind::AttributeUse,
            String::new(),
            String::new(),
            elem,
            XsData::AttributeUse(AttributeUseData {
                declaration: usize::MAX,
                required,
                lexical,
                constraint,
            }),
        ));
        self.add_child(owner, idx);
        let Some(decl) = self.parse_attribute_declaration(elem, idx, false)? else {
            // Атрибут без имени: использование без объявления платформа не
            // показывает, значит и заводить его незачем.
            self.nodes[owner].children.pop();
            return Ok(None);
        };
        if let XsData::AttributeUse(d) = &mut self.nodes[idx].data {
            d.declaration = decl;
        }
        Ok(Some(idx))
    }

    /// Простой тип. `anonymous` = встроенный в объявление или в список.
    pub(crate) fn parse_simple_type(
        &mut self,
        elem: &Rc<DomNode>,
        owner: usize,
        global: bool,
    ) -> RtResult<usize> {
        let name = elem.xs_attribute("name").unwrap_or_default();
        let ns = if global || !name.is_empty() {
            self.target_ns.clone()
        } else {
            String::new()
        };
        let idx = self.push(Self::new_node(
            XsKind::SimpleType,
            name,
            ns,
            elem,
            XsData::SimpleType(SimpleTypeData {
                // Пустой `<xs:simpleType/>` платформа считает атомарным
                // (измерено).
                variety: Some(EnumValue::XsVarietyAtomic),
                ..SimpleTypeData::default()
            }),
        ));
        self.add_child(owner, idx);
        for child in xsd_children(elem) {
            match child.xs_local_name() {
                "annotation" => {
                    self.parse_annotation(&child, idx)?;
                }
                "restriction" => self.parse_simple_restriction(&child, idx)?,
                "list" => self.parse_list(&child, idx)?,
                "union" => self.parse_union(&child, idx)?,
                _ => {}
            }
        }
        Ok(idx)
    }

    pub(crate) fn parse_simple_restriction(
        &mut self,
        elem: &Rc<DomNode>,
        owner: usize,
    ) -> RtResult<()> {
        let base = elem
            .xs_attribute("base")
            .and_then(|b| resolve_qname(elem, &b));
        if let XsData::SimpleType(d) = &mut self.nodes[owner].data {
            d.base_name = base;
            d.variety = Some(EnumValue::XsVarietyAtomic);
        }
        for child in xsd_children(elem) {
            let local = child.xs_local_name().to_string();
            if local == "annotation" {
                self.parse_annotation(&child, owner)?;
                continue;
            }
            if local == "simpleType" {
                // Базовый тип, записанный внутрь ограничения, платформа
                // показывает отдельной компонентой — как именно, не
                // измерено, поэтому отказ, а не догадка.
                return Err(RtError::Xsd(
                    "встроенный простой тип в `xs:restriction` не поддерживается".to_string(),
                ));
            }
            let Some(facet) = FacetKind::from_element(&local) else {
                // Незнакомый элемент внутри ограничения — пропускаем, как
                // и незнакомый элемент схемы.
                continue;
            };
            let node = Self::new_node(
                XsKind::Facet(facet),
                String::new(),
                String::new(),
                &child,
                XsData::Facet(FacetData {
                    lexical: child.xs_attribute("value").unwrap_or_default(),
                    fixed: Self::boolean_attribute(&child, "fixed"),
                }),
            );
            let facet_idx = self.push(node);
            self.add_child(owner, facet_idx);
            if let XsData::SimpleType(d) = &mut self.nodes[owner].data {
                d.facets.push(facet_idx);
            }
        }
        Ok(())
    }

    pub(crate) fn parse_list(&mut self, elem: &Rc<DomNode>, owner: usize) -> RtResult<()> {
        let item_name = elem
            .xs_attribute("itemType")
            .and_then(|t| resolve_qname(elem, &t));
        let mut item_type = None;
        for child in xsd_children(elem) {
            match child.xs_local_name() {
                "annotation" => {
                    self.parse_annotation(&child, owner)?;
                }
                "simpleType" => {
                    let inner = self.parse_simple_type(&child, owner, false)?;
                    item_type.get_or_insert(inner);
                }
                _ => {}
            }
        }
        if let XsData::SimpleType(d) = &mut self.nodes[owner].data {
            d.variety = Some(EnumValue::XsVarietyList);
            d.item_type_name = item_name;
            d.item_type = item_type;
        }
        Ok(())
    }

    pub(crate) fn parse_union(&mut self, elem: &Rc<DomNode>, owner: usize) -> RtResult<()> {
        for child in xsd_children(elem) {
            match child.xs_local_name() {
                "annotation" => {
                    self.parse_annotation(&child, owner)?;
                }
                "simpleType" => {
                    // Встроенные члены объединения платформа держит в
                    // `ОпределенияТиповОбъединения`, но у неё этот список
                    // оказался пуст даже для `memberTypes` — как он
                    // заполняется, не измерено.
                    return Err(RtError::Xsd(
                        "встроенный простой тип в `xs:union` не поддерживается".to_string(),
                    ));
                }
                _ => {}
            }
        }
        let members = elem
            .xs_attribute("memberTypes")
            .map(|s| {
                s.split_whitespace()
                    .filter_map(|q| resolve_qname(elem, q))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        if let XsData::SimpleType(d) = &mut self.nodes[owner].data {
            d.variety = Some(EnumValue::XsVarietyUnion);
            d.member_type_names = members;
        }
        Ok(())
    }

    pub(crate) fn parse_complex_type(
        &mut self,
        elem: &Rc<DomNode>,
        owner: usize,
        global: bool,
    ) -> RtResult<usize> {
        let name = elem.xs_attribute("name").unwrap_or_default();
        let ns = if global || !name.is_empty() {
            self.target_ns.clone()
        } else {
            String::new()
        };
        let idx = self.push(Self::new_node(
            XsKind::ComplexType,
            name,
            ns,
            elem,
            XsData::ComplexType(ComplexTypeData {
                mixed: Self::boolean_attribute(elem, "mixed"),
                is_abstract: Self::boolean_attribute(elem, "abstract"),
                ..ComplexTypeData::default()
            }),
        ));
        self.add_child(owner, idx);
        self.parse_complex_body(elem, idx)?;
        Ok(idx)
    }

    /// Тело составного типа — и прямое, и то, что лежит внутри
    /// `xs:extension`/`xs:restriction`.
    pub(crate) fn parse_complex_body(&mut self, elem: &Rc<DomNode>, idx: usize) -> RtResult<()> {
        for child in xsd_children(elem) {
            match child.xs_local_name() {
                "annotation" => {
                    self.parse_annotation(&child, idx)?;
                }
                "simpleContent" | "complexContent" => self.parse_derivation(&child, idx)?,
                "sequence" | "choice" | "all" => {
                    let particle = self.parse_particle_group(&child, idx)?;
                    if let XsData::ComplexType(d) = &mut self.nodes[idx].data {
                        d.content.get_or_insert(particle);
                    }
                }
                "attribute" => {
                    if let Some(use_idx) = self.parse_attribute_use(&child, idx)?
                        && let XsData::ComplexType(d) = &mut self.nodes[idx].data
                    {
                        d.attributes.push(use_idx);
                    }
                }
                // НЕ ИЗМЕРЕНО(XSD.WILDCARD.COMPONENT): как платформа
                // представляет маску `xs:anyAttribute` в компонентах типа.
                // Компонентой маска здесь не становится: открытого
                // содержимого в этой реализации нет (см. шапку `xdto.rs`),
                // запись объектов масок не порождает, а чтение
                // постороннего атрибута остаётся ошибкой, как и раньше.
                // Записывается только САМ ФАКТ маски — он делает тип XDTO
                // открытым.
                "anyAttribute" => self.mark_wildcard(idx),
                "group" | "attributeGroup" => {
                    return Err(unsupported(child.xs_local_name()));
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// `xs:simpleContent`/`xs:complexContent` — один `extension` или
    /// `restriction` внутри.
    pub(crate) fn parse_derivation(&mut self, elem: &Rc<DomNode>, idx: usize) -> RtResult<()> {
        for child in xsd_children(elem) {
            let method = match child.xs_local_name() {
                "extension" => EnumValue::XsDerivationExtension,
                "restriction" => EnumValue::XsDerivationRestriction,
                "annotation" => {
                    self.parse_annotation(&child, idx)?;
                    continue;
                }
                _ => continue,
            };
            let base = child
                .xs_attribute("base")
                .and_then(|b| resolve_qname(&child, &b));
            if let XsData::ComplexType(d) = &mut self.nodes[idx].data {
                d.derivation = Some(method);
                d.base_name = base;
            }
            self.parse_complex_body(&child, idx)?;
        }
        Ok(())
    }

    /// `xs:sequence`/`xs:choice`/`xs:all` — фрагмент, чей терм есть группа
    /// модели.
    pub(crate) fn parse_particle_group(
        &mut self,
        elem: &Rc<DomNode>,
        owner: usize,
    ) -> RtResult<usize> {
        let compositor = match elem.xs_local_name() {
            "sequence" => EnumValue::XsGroupSequence,
            "choice" => EnumValue::XsGroupChoice,
            "all" => EnumValue::XsGroupAll,
            other => unreachable!("группа модели вызвана для «{other}»"),
        };
        let particle = self.push(Self::new_node(
            XsKind::Particle,
            String::new(),
            String::new(),
            elem,
            XsData::Particle {
                term: usize::MAX,
                min_occurs: Self::occurs_attribute(elem, "minOccurs"),
                max_occurs: Self::occurs_attribute(elem, "maxOccurs"),
            },
        ));
        self.add_child(owner, particle);
        let group = self.push(Self::new_node(
            XsKind::ModelGroup,
            String::new(),
            String::new(),
            elem,
            XsData::ModelGroup {
                compositor,
                particles: Vec::new(),
            },
        ));
        self.add_child(particle, group);
        if let XsData::Particle { term, .. } = &mut self.nodes[particle].data {
            *term = group;
        }
        for child in xsd_children(elem) {
            match child.xs_local_name() {
                "annotation" => {
                    self.parse_annotation(&child, group)?;
                }
                "element" => {
                    let inner = self.push(Self::new_node(
                        XsKind::Particle,
                        String::new(),
                        String::new(),
                        &child,
                        XsData::Particle {
                            term: usize::MAX,
                            min_occurs: Self::occurs_attribute(&child, "minOccurs"),
                            max_occurs: Self::occurs_attribute(&child, "maxOccurs"),
                        },
                    ));
                    self.add_child(group, inner);
                    match self.parse_element_declaration(&child, inner, false)? {
                        Some(decl) => {
                            if let XsData::Particle { term, .. } = &mut self.nodes[inner].data {
                                *term = decl;
                            }
                            if let XsData::ModelGroup { particles, .. } =
                                &mut self.nodes[group].data
                            {
                                particles.push(inner);
                            }
                        }
                        None => {
                            // Элемент без имени: фрагмента без терма быть
                            // не может, поэтому пустой фрагмент снимаем.
                            self.nodes[group].children.pop();
                        }
                    }
                }
                "sequence" | "choice" | "all" => {
                    let inner = self.parse_particle_group(&child, group)?;
                    if let XsData::ModelGroup { particles, .. } = &mut self.nodes[group].data {
                        particles.push(inner);
                    }
                }
                // НЕ ИЗМЕРЕНО(XSD.WILDCARD.COMPONENT): `xs:any` ведёт себя
                // так же, как `xs:anyAttribute` в теле типа — маска не
                // попадает ни в дерево компонент, ни в модель содержимого,
                // открытое содержимое остаётся нереализованным, но САМ
                // ФАКТ маски запоминается у объемлющего составного типа.
                "any" => self.mark_wildcard(group),
                "group" => return Err(unsupported(child.xs_local_name())),
                _ => {}
            }
        }
        Ok(particle)
    }
}

/// `ПостроительСхемXML.СоздатьСхемуXML(ДокументDOM | ЭлементDOM)`.
///
/// # Errors
///
/// [`RtError::MethodNotApplicable`] — если получатель не построитель или
/// аргумент не один; [`RtError::Xsd`] — если аргумент не узел DOM, документ
/// без корня либо схема содержит конструкцию за границей модели.
pub fn create_schema(obj: &BslValue, args: &[BslValue]) -> RtResult<BslValue> {
    if !is_builder(obj) {
        return Err(RtError::MethodNotApplicable {
            method: "СоздатьСхемуXML",
            receiver: obj.type_name(),
        });
    }
    let [arg] = args else {
        return Err(RtError::MethodNotApplicable {
            method: "СоздатьСхемуXML",
            receiver: obj.type_name(),
        });
    };
    let (node, doc) = match arg
        .object_ref()
        .and_then(|object| object.downcast_ref::<crate::dom::DomNodeObject>())
    {
        Some(dom_node) => (dom_node.node.clone(), dom_node.doc.clone()),
        _ => return Err(RtError::Xsd(source_error())),
    };
    let root = match node.kind() {
        DomKind::Document => crate::dom::xs_document_element(&node)
            .ok_or_else(|| RtError::Xsd("документ без корневого элемента".to_string()))?,
        DomKind::Element => node,
        _ => return Err(RtError::Xsd(source_error())),
    };
    // Корень не схема — это `Неопределено`, а не ошибка (измерено).
    if root.xs_uri() != XSD_NS || root.xs_local_name() != "schema" {
        return Ok(BslValue::Undefined);
    }
    build_schema(&root, &doc)
}

pub(crate) fn source_error() -> String {
    "СоздатьСхемуXML принимает ДокументDOM или ЭлементDOM".to_string()
}

pub(crate) fn build_schema(root: &Rc<DomNode>, doc: &Rc<DomNode>) -> RtResult<BslValue> {
    let target_ns = root.xs_attribute("targetNamespace").unwrap_or_default();
    let element_form = match root.xs_attribute("elementFormDefault").as_deref() {
        Some("qualified") => Some(EnumValue::XsFormQualified),
        Some("unqualified") => Some(EnumValue::XsFormUnqualified),
        _ => None,
    };
    let attribute_form = match root.xs_attribute("attributeFormDefault").as_deref() {
        Some("qualified") => Some(EnumValue::XsFormQualified),
        Some("unqualified") => Some(EnumValue::XsFormUnqualified),
        _ => None,
    };
    let mut p = Parser {
        nodes: vec![XsNode {
            kind: XsKind::Schema,
            parent: None,
            name: String::new(),
            ns: target_ns.clone(),
            children: Vec::new(),
            dom: Some(root.clone()),
            data: XsData::Schema(SchemaData {
                version: root.xs_attribute("version").unwrap_or_default(),
                location: String::new(),
                element_form,
                attribute_form,
                ..SchemaData::default()
            }),
        }],
        target_ns,
        element_form_qualified: element_form == Some(EnumValue::XsFormQualified),
        attribute_form_qualified: attribute_form == Some(EnumValue::XsFormQualified),
    };

    let mut elements = Vec::new();
    let mut attributes = Vec::new();
    let mut types = Vec::new();
    for child in xsd_children(root) {
        match child.xs_local_name() {
            "annotation" => {
                p.parse_annotation(&child, 0)?;
            }
            "element" => {
                if let Some(i) = p.parse_element_declaration(&child, 0, true)? {
                    elements.push(i);
                }
            }
            "attribute" => {
                if let Some(i) = p.parse_attribute_declaration(&child, 0, true)? {
                    attributes.push(i);
                }
            }
            "simpleType" => {
                let i = p.parse_simple_type(&child, 0, true)?;
                if !p.nodes[i].name.is_empty() {
                    types.push(i);
                }
            }
            "complexType" => {
                let i = p.parse_complex_type(&child, 0, true)?;
                if !p.nodes[i].name.is_empty() {
                    types.push(i);
                }
            }
            // НЕ ИЗМЕРЕНО(XSD.IMPORT.COMPONENT): как платформа представляет
            // `xs:import` в компонентах схемы и загружает ли она
            // `schemaLocation` сама. Здесь директива пропускается: имена
            // чужого пространства имён разрешаются через общий
            // `НаборСхемXML`, а узла в дереве компонент не остаётся —
            // схема без пары в наборе честно упадёт неразрешённым именем
            // при построении фабрики.
            "import" => {}
            "include" | "redefine" | "group" | "attributeGroup" | "notation" => {
                return Err(unsupported(child.xs_local_name()));
            }
            // Незнакомый элемент в пространстве имён схемы платформа
            // пропускает (измерено на `<xs:чушь/>`).
            _ => {}
        }
    }

    let sort_named = |p: &Parser, mut v: Vec<usize>| {
        // Именованные коллекции отсортированы по имени, а дубль имени
        // теряется: остаётся первый по документу (измерено обе вещи).
        v.sort_by(|a, b| p.nodes[*a].name.cmp(&p.nodes[*b].name));
        v.dedup_by(|a, b| p.nodes[*a].name == p.nodes[*b].name);
        v
    };
    let elements = sort_named(&p, elements);
    let attributes = sort_named(&p, attributes);
    let types = sort_named(&p, types);
    if let XsData::Schema(d) = &mut p.nodes[0].data {
        d.elements = elements;
        d.attributes = attributes;
        d.types = types;
    }

    let schema = Rc::new(XsSchemaData {
        nodes: p.nodes,
        dom_doc: Some(doc.clone()),
    });
    Ok(component_value(&schema, 0))
}
