//! Модель схемы: узлы, виды компонент, фасеты.

use super::*;

/// Пространство имён XML Schema. Опознание элементов схемы идёт по нему, а
/// не по префиксу: измерено, что префикс может быть любым.
pub const XSD_NS: &str = "http://www.w3.org/2001/XMLSchema";

/// Вид фасета. Отдельный тип, потому что от него зависят и имя типа
/// значения, и член `ТипКомпонентыXS`, и способ разбора значения.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FacetKind {
    Length,
    MinLength,
    MaxLength,
    Pattern,
    Enumeration,
    WhiteSpace,
    TotalDigits,
    FractionDigits,
    MinInclusive,
    MaxInclusive,
    MinExclusive,
    MaxExclusive,
}

impl FacetKind {
    /// Имя элемента XSD, которым фасет записывается.
    pub(crate) fn from_element(local: &str) -> Option<FacetKind> {
        Some(match local {
            "length" => FacetKind::Length,
            "minLength" => FacetKind::MinLength,
            "maxLength" => FacetKind::MaxLength,
            "pattern" => FacetKind::Pattern,
            "enumeration" => FacetKind::Enumeration,
            "whiteSpace" => FacetKind::WhiteSpace,
            "totalDigits" => FacetKind::TotalDigits,
            "fractionDigits" => FacetKind::FractionDigits,
            "minInclusive" => FacetKind::MinInclusive,
            "maxInclusive" => FacetKind::MaxInclusive,
            "minExclusive" => FacetKind::MinExclusive,
            "maxExclusive" => FacetKind::MaxExclusive,
            _ => return None,
        })
    }

    /// Член `ТипКомпонентыXS` — измеренные написания, в том числе
    /// «перевёрнутые» у четырёх граничных фасетов.
    pub(crate) fn component_type(self) -> EnumValue {
        match self {
            FacetKind::Length => EnumValue::XsCompLengthFacet,
            FacetKind::MinLength => EnumValue::XsCompMinLengthFacet,
            FacetKind::MaxLength => EnumValue::XsCompMaxLengthFacet,
            FacetKind::Pattern => EnumValue::XsCompPatternFacet,
            FacetKind::Enumeration => EnumValue::XsCompEnumerationFacet,
            FacetKind::WhiteSpace => EnumValue::XsCompWhitespaceFacet,
            FacetKind::TotalDigits => EnumValue::XsCompTotalDigitsFacet,
            FacetKind::FractionDigits => EnumValue::XsCompFractionDigitsFacet,
            FacetKind::MinInclusive => EnumValue::XsCompMinInclusiveFacet,
            FacetKind::MaxInclusive => EnumValue::XsCompMaxInclusiveFacet,
            FacetKind::MinExclusive => EnumValue::XsCompMinExclusiveFacet,
            FacetKind::MaxExclusive => EnumValue::XsCompMaxExclusiveFacet,
        }
    }

    /// Имя типа ЗНАЧЕНИЯ фасета — то, что печатает `Строка()`.
    pub(crate) fn type_name(self) -> &'static str {
        match self {
            FacetKind::Length => "ФасетДлиныXS",
            FacetKind::MinLength => "ФасетМинимальнойДлиныXS",
            FacetKind::MaxLength => "ФасетМаксимальнойДлиныXS",
            FacetKind::Pattern => "ФасетОбразцаXS",
            FacetKind::Enumeration => "ФасетПеречисленияXS",
            FacetKind::WhiteSpace => "ФасетПробельныхСимволовXS",
            FacetKind::TotalDigits => "ФасетОбщегоКоличестваРазрядовXS",
            FacetKind::FractionDigits => "ФасетКоличестваРазрядовДробнойЧастиXS",
            FacetKind::MinInclusive => "ФасетМинимальногоВключающегоЗначенияXS",
            FacetKind::MaxInclusive => "ФасетМаксимальногоВключающегоЗначенияXS",
            FacetKind::MinExclusive => "ФасетМинимальногоИсключающегоЗначенияXS",
            FacetKind::MaxExclusive => "ФасетМаксимальногоИсключающегоЗначенияXS",
        }
    }

    /// Числовой ли это фасет — от этого зависит и вид `Значение`, и то,
    /// есть ли у фасета `Фиксированный`.
    ///
    /// Свойство `Фиксированный` платформа держит РОВНО у числовых фасетов:
    /// у образца, перечисления и пробельных символов обращение к нему —
    /// ошибка (измерено все три). Спецификация XML Schema разрешает
    /// `fixed` и у `whiteSpace`, но платформу мы не спорим, а измеряем.
    pub(crate) fn is_numeric(self) -> bool {
        !matches!(
            self,
            FacetKind::Pattern | FacetKind::Enumeration | FacetKind::WhiteSpace
        )
    }
}

/// Вид компоненты схемы.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XsKind {
    Schema,
    Element,
    Attribute,
    SimpleType,
    ComplexType,
    Particle,
    ModelGroup,
    AttributeUse,
    Facet(FacetKind),
    Annotation,
    Documentation,
    AppInfo,
}

impl XsKind {
    /// Имя типа значения: `Строка(Компонента)`.
    pub fn type_name(self) -> &'static str {
        match self {
            XsKind::Schema => "СхемаXML",
            XsKind::Element => "ОбъявлениеЭлементаXS",
            XsKind::Attribute => "ОбъявлениеАтрибутаXS",
            XsKind::SimpleType => "ОпределениеПростогоТипаXS",
            XsKind::ComplexType => "ОпределениеСоставногоТипаXS",
            XsKind::Particle => "ФрагментXS",
            XsKind::ModelGroup => "ГруппаМоделиXS",
            XsKind::AttributeUse => "ИспользованиеАтрибутаXS",
            XsKind::Facet(f) => f.type_name(),
            XsKind::Annotation => "АннотацияXS",
            XsKind::Documentation => "ДокументацияXS",
            XsKind::AppInfo => "ИнформацияДляПриложенияXS",
        }
    }

    /// `ТипКомпоненты`.
    pub(crate) fn component_type(self) -> EnumValue {
        match self {
            XsKind::Schema => EnumValue::XsCompSchema,
            XsKind::Element => EnumValue::XsCompElementDeclaration,
            XsKind::Attribute => EnumValue::XsCompAttributeDeclaration,
            XsKind::SimpleType => EnumValue::XsCompSimpleTypeDefinition,
            XsKind::ComplexType => EnumValue::XsCompComplexTypeDefinition,
            XsKind::Particle => EnumValue::XsCompParticle,
            XsKind::ModelGroup => EnumValue::XsCompModelGroup,
            XsKind::AttributeUse => EnumValue::XsCompAttributeUse,
            XsKind::Facet(f) => f.component_type(),
            XsKind::Annotation => EnumValue::XsCompAnnotation,
            XsKind::Documentation => EnumValue::XsCompDocumentation,
            XsKind::AppInfo => EnumValue::XsCompAppInfo,
        }
    }
}

/// Расширенное имя XML: `{URI}ЛокальноеИмя`. Тип ЗНАЧЕНИЯ — два имени с
/// одинаковыми частями равны (измерено).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct XName {
    pub uri: String,
    pub local: String,
}

impl XName {
    /// Как печатает `Строка()`: с фигурными скобками, а при ПУСТОМ URI —
    /// одно локальное имя без скобок (измерено).
    pub fn display_text(&self) -> String {
        if self.uri.is_empty() {
            self.local.clone()
        } else {
            format!("{{{}}}{}", self.uri, self.local)
        }
    }
}

/// Данные, своие для каждого вида компоненты.
#[derive(Debug)]
pub(crate) enum XsData {
    Schema(SchemaData),
    Element(DeclData),
    Attribute(DeclData),
    SimpleType(SimpleTypeData),
    ComplexType(ComplexTypeData),
    /// Фрагмент: единственный терм и границы вхождения так, как они
    /// написаны в `minOccurs`/`maxOccurs` (см. [`Parser::occurs_attribute`]).
    Particle {
        term: usize,
        min_occurs: Option<u32>,
        max_occurs: Option<u32>,
    },
    ModelGroup {
        compositor: EnumValue,
        particles: Vec<usize>,
    },
    AttributeUse(AttributeUseData),
    Facet(FacetData),
    Annotation {
        documentation: Vec<usize>,
        appinfo: Vec<usize>,
    },
    /// `xs:documentation` и `xs:appinfo` — одинаковое устройство, вид
    /// различает [`XsKind`].
    Documentation {
        lang: String,
        source: String,
    },
}

#[derive(Debug, Default)]
pub(crate) struct SchemaData {
    pub(crate) version: String,
    pub(crate) location: String,
    pub(crate) element_form: Option<EnumValue>,
    pub(crate) attribute_form: Option<EnumValue>,
    /// Именованные коллекции — отсортированные по имени номера узлов.
    pub(crate) elements: Vec<usize>,
    pub(crate) attributes: Vec<usize>,
    pub(crate) types: Vec<usize>,
}

/// Объявление элемента и объявление атрибута устроены одинаково; поля, у
/// которых смысл только у одного из двух (`abstract` у элемента), у другого
/// просто пусты.
#[derive(Debug, Default)]
pub(crate) struct DeclData {
    pub(crate) type_name: Option<XName>,
    pub(crate) reference: Option<XName>,
    pub(crate) anonymous_type: Option<usize>,
    pub(crate) form: Option<EnumValue>,
    pub(crate) is_abstract: Option<bool>,
    /// `default`/`fixed` как написано.
    pub(crate) lexical: String,
    /// Который из двух — и `Неопределено`, если ни одного. У ЛОКАЛЬНОГО
    /// объявления атрибута платформа держит здесь `Неопределено` даже при
    /// `default` (измерено), поэтому поле заполняется не всегда.
    pub(crate) constraint: Option<EnumValue>,
    pub(crate) global: bool,
}

#[derive(Debug, Default)]
pub(crate) struct SimpleTypeData {
    pub(crate) base_name: Option<XName>,
    pub(crate) variety: Option<EnumValue>,
    pub(crate) item_type_name: Option<XName>,
    pub(crate) item_type: Option<usize>,
    pub(crate) member_type_names: Vec<XName>,
    pub(crate) facets: Vec<usize>,
}

#[derive(Debug, Default)]
pub(crate) struct ComplexTypeData {
    pub(crate) base_name: Option<XName>,
    pub(crate) derivation: Option<EnumValue>,
    pub(crate) content: Option<usize>,
    pub(crate) attributes: Vec<usize>,
    pub(crate) mixed: Option<bool>,
    pub(crate) is_abstract: Option<bool>,
    /// Есть ли в объявлении типа маска `xs:any` или `xs:anyAttribute` — на
    /// любой глубине модели содержимого. Сами маски по-прежнему
    /// пропускаются (`НЕ ИЗМЕРЕНО(XSD.WILDCARD.COMPONENT)`), но их наличие
    /// наблюдаемо: это `Открытый` типа XDTO, а от него зависит ПОРЯДОК
    /// записи свойств — см. модуль `xdto` компонента `bsl-xml`.
    pub(crate) has_wildcard: bool,
}

#[derive(Debug)]
pub(crate) struct AttributeUseData {
    pub(crate) declaration: usize,
    pub(crate) required: bool,
    pub(crate) lexical: String,
    pub(crate) constraint: Option<EnumValue>,
}

#[derive(Debug)]
pub(crate) struct FacetData {
    pub(crate) lexical: String,
    pub(crate) fixed: Option<bool>,
}

/// Одна компонента схемы. Дерево держится в общем массиве
/// [`XsSchemaData::nodes`], а связи — номерами: `Контейнер` и `Схема`
/// достаются подъёмом по `parent` без единой слабой ссылки, а тождество
/// компонент (измерено: два `Получить("root")` дают равные значения) — это
/// пара «та же схема, тот же номер».
#[derive(Debug)]
pub(crate) struct XsNode {
    pub(crate) kind: XsKind,
    pub(crate) parent: Option<usize>,
    pub(crate) name: String,
    pub(crate) ns: String,
    /// `Компоненты` — в порядке документа.
    pub(crate) children: Vec<usize>,
    /// `ЭлементDOM`; пусто только у схемы, построенной `Новый СхемаXML`.
    pub(crate) dom: Option<Rc<DomNode>>,
    pub(crate) data: XsData,
}

/// Разобранная схема целиком. Значение `СхемаXML` — это `Rc` на неё плюс
/// номер 0; любая другая компонента — тот же `Rc` плюс свой номер.
#[derive(Debug)]
pub struct XsSchemaData {
    pub(crate) nodes: Vec<XsNode>,
    /// Документ, из которого построена схема, — держит дерево живым, пока
    /// жива схема (у узлов внутри дерева ссылки вверх слабые).
    pub(crate) dom_doc: Option<Rc<DomNode>>,
}

impl XsSchemaData {
    pub(crate) fn node(&self, i: usize) -> &XsNode {
        &self.nodes[i]
    }

    /// Целевое пространство имён схемы.
    pub(crate) fn target_ns(&self) -> &str {
        &self.nodes[0].ns
    }
}

/// Вид коллекции компонент. Три разных типа значения с одинаковым
/// устройством: разница в имени типа и в том, что именованная умеет
/// `Получить(Имя)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum XsListKind {
    /// `ФиксированныйСписокКомпонентXS`.
    Fixed(Vec<usize>),
    /// `СписокКомпонентXS`.
    Plain(Vec<usize>),
    /// `КоллекцияИменованныхКомпонентXS`.
    Named(Vec<usize>),
}

impl XsListKind {
    pub(crate) fn items(&self) -> &[usize] {
        match self {
            XsListKind::Fixed(v) | XsListKind::Plain(v) | XsListKind::Named(v) => v,
        }
    }

    pub fn len(&self) -> usize {
        self.items().len()
    }
}
