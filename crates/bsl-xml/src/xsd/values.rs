//! Значения фасетов и лексические формы.

use super::*;

// --- значения ------------------------------------------------------------

pub(crate) fn str_value(s: &str) -> BslValue {
    BslValue::Str(BslString::from_str(s))
}

/// `ПостроительСхемXML` — состояния нет, схему целиком строит
/// `СоздатьСхемуXML`.
#[derive(Debug)]
pub struct BuilderObject;

/// `НаборСхемXML` — упорядоченный список схем.
#[derive(Debug)]
pub struct SchemaSetObject {
    pub(crate) schemas: Rc<RefCell<Vec<Rc<XsSchemaData>>>>,
}

/// Компонента модели схемы: схема целиком и номер узла в ней.
#[derive(Debug)]
pub struct ComponentObject {
    pub(crate) schema: Rc<XsSchemaData>,
    pub(crate) index: usize,
}

/// Списки компонент — фиксированный, плоский и именованная коллекция.
#[derive(Debug)]
pub struct SchemaListObject {
    pub(crate) schema: Rc<XsSchemaData>,
    pub(crate) kind: XsListKind,
}

/// `РасширенноеИмяXML` — значение, а не объект: равенство по содержимому.
#[derive(Debug)]
pub struct ExpandedNameObject {
    pub(crate) name: Rc<XName>,
}

/// `СписокРасширенныхИменXML` — снимок имён.
#[derive(Debug)]
pub struct NameListObject {
    pub(crate) names: Rc<Vec<XName>>,
}

pub(crate) fn component_value(schema: &Rc<XsSchemaData>, index: usize) -> BslValue {
    BslValue::new_object(ComponentObject {
        schema: schema.clone(),
        index,
    })
}

pub(crate) fn list_value(schema: &Rc<XsSchemaData>, kind: XsListKind) -> BslValue {
    BslValue::new_object(SchemaListObject {
        schema: schema.clone(),
        kind,
    })
}

pub(crate) fn name_value(name: &XName) -> BslValue {
    BslValue::new_object(ExpandedNameObject {
        name: Rc::new(name.clone()),
    })
}

pub(crate) fn opt_name(name: Option<&XName>) -> BslValue {
    name.map_or(BslValue::Undefined, name_value)
}

pub(crate) fn opt_enum(v: Option<EnumValue>) -> BslValue {
    v.map_or(BslValue::Undefined, BslValue::Enum)
}

pub(crate) fn opt_bool(v: Option<bool>) -> BslValue {
    v.map_or(BslValue::Undefined, BslValue::Boolean)
}

/// Граница вхождения частицы наружу. Разбор оставляет её беззнаковым
/// 32-битным числом (см. [`Parser::occurs_attribute`]), а показывается она
/// по-разному с двух концов: `u32::MAX` — это внутреннее «без границы», и
/// `МаксимальноВходит` отдаёт его СТРОКОЙ `unbounded`, тогда как
/// `МинимальноВходит` — числом 4294967295 (измерено: `maxOccurs="unbounded"`
/// -> `Строка [unbounded]`, `minOccurs="unbounded"` -> `Число [4 294 967
/// 295]`). Различить «написано `unbounded`» и «написано 4294967295» модель
/// платформы поэтому не может, и здесь тоже не может.
pub(crate) fn occurs_value(occurs: Option<u32>, maximum: bool) -> BslValue {
    match occurs {
        None => BslValue::Undefined,
        Some(u32::MAX) if maximum => str_value("unbounded"),
        Some(n) => BslValue::Number(BslNumber::from_i64(i64::from(n))),
    }
}

/// `Новый ПостроительСхемXML`.
pub fn new_builder() -> BslValue {
    BslValue::new_object(BuilderObject)
}

/// `Новый НаборСхемXML`.
pub fn new_schema_set() -> BslValue {
    BslValue::new_object(SchemaSetObject {
        schemas: Rc::new(RefCell::new(Vec::new())),
    })
}

/// `Новый СхемаXML` — пустая схема без дерева за спиной.
pub fn new_schema() -> BslValue {
    let schema = Rc::new(XsSchemaData {
        nodes: vec![XsNode {
            kind: XsKind::Schema,
            parent: None,
            name: String::new(),
            ns: String::new(),
            children: Vec::new(),
            dom: None,
            data: XsData::Schema(SchemaData::default()),
        }],
        dom_doc: None,
    });
    component_value(&schema, 0)
}

/// `Новый РасширенноеИмяXML(URI, ЛокальноеИмя)`.
pub fn new_expanded_name(uri: &str, local: &str) -> BslValue {
    name_value(&XName {
        uri: uri.to_string(),
        local: local.to_string(),
    })
}

pub fn is_builder(v: &BslValue) -> bool {
    v.object_ref()
        .is_some_and(|object| object.downcast_ref::<BuilderObject>().is_some())
}

/// Компонента ли это и, если да, какая именно.
pub(crate) fn as_component(v: &BslValue) -> Option<(Rc<XsSchemaData>, usize)> {
    v.object_ref()
        .and_then(|object| object.downcast_ref::<ComponentObject>())
        .map(|component| (component.schema.clone(), component.index))
}
