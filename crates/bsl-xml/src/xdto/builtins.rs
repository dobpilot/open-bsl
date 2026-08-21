//! Соответствие встроенных типов XML Schema типам BSL.

use super::*;

// --- встроенные типы XML Schema ------------------------------------------

/// Тип BSL, в который платформа отображает лексическую форму встроенного
/// типа. Варианты различают не только результат, но и РАЗБОР: у трёх
/// временных типов и у двух двоичных лексические формы разные.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltinBsl {
    /// `Строка` — лексическая форма как есть.
    Str,
    /// `Число` без показателя степени: `xs:decimal` и все целые
    /// (измерено: `Создать` от `decimal` с «1.5E3» и от `int` с «1E2» —
    /// ошибка, как и от `int` с «1.5»).
    Number,
    /// `Число` с показателем степени: `xs:double` и `xs:float`
    /// (измерено: «1.5E3» -> 1500).
    Double,
    /// `Булево`: `true`/`1` и `false`/`0`.
    Boolean,
    /// `Дата` из `xs:date` — время суток нулевое.
    Date,
    /// `Дата` из `xs:dateTime`.
    DateTime,
    /// `Дата` из `xs:time` — дата 01.01.0001.
    Time,
    /// `ДвоичныеДанные` из `base64Binary`.
    Base64,
    /// `ДвоичныеДанные` из `hexBinary`.
    Hex,
    /// `РасширенноеИмяXML` из `QName`.
    QName,
}

impl BuiltinBsl {
    /// Имя типа BSL, который получится из лексической формы, — как его
    /// печатает `Строка(ТипЗнч(...))`. Вне тестов встроенные типы
    /// проверяются через модель, поэтому lib-цель метода не видит.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn type_name(self) -> &'static str {
        match self {
            BuiltinBsl::Str => TypeId::String.name(),
            BuiltinBsl::Number | BuiltinBsl::Double => TypeId::Number.name(),
            BuiltinBsl::Boolean => TypeId::Boolean.name(),
            BuiltinBsl::Date | BuiltinBsl::DateTime | BuiltinBsl::Time => TypeId::Date.name(),
            BuiltinBsl::Base64 | BuiltinBsl::Hex => TypeId::BinaryData.name(),
            // Расширенное имя — тип компонента: у него своё представление
            // в дескрипторе, а не строка в закрытом реестре ядра.
            BuiltinBsl::QName => crate::xsd::EXPANDED_NAME_TYPE.type_display,
        }
    }
}

/// Строка таблицы встроенных типов пространства
/// `http://www.w3.org/2001/XMLSchema`.
pub(crate) struct BuiltinType {
    pub(crate) name: &'static str,
    /// Имя базового встроенного типа; `None` только у `anyType`.
    pub(crate) base: Option<&'static str>,
    /// Отображение в тип BSL; `None` — это ТИП ОБЪЕКТА (`anyType`),
    /// значения из лексической формы он не строит.
    pub(crate) bsl: Option<BuiltinBsl>,
    /// Фасеты в том порядке, в каком их отдаёт платформа.
    pub(crate) facets: &'static [(FacetKind, &'static str)],
}

/// Встроенные типы XML Schema: имя, базовый тип, отображение в BSL и
/// фасеты.
///
/// Таблица ИЗМЕРЕНА целиком и пришпилена к строкам
/// `measure-xdto.platform.txt`: столбец «базовый тип» и фасеты — к строкам
/// `фв <имя>`, столбец «тип BSL» — к строкам `вст <имя>`, где значение
/// строилось `ФабрикаXDTO.Создать(Тип, Лексика).Значение`. Ни одна строка
/// не выведена из спецификации W3C: например `фв unsignedInt` показал
/// базовым `unsignedLong` (а не `unsignedShort`, как можно было бы
/// достроить по убыванию разрядности), и здесь стоит измеренное.
///
/// Порядок строк — от корня иерархии вниз, чтобы связывание базовых типов
/// читалось глазами; на работу порядок не влияет.
pub(crate) static BUILTIN_TYPES: &[BuiltinType] = &[
    BuiltinType {
        name: "anyType",
        base: None,
        bsl: None,
        facets: &[],
    },
    BuiltinType {
        name: "anySimpleType",
        base: Some("anyType"),
        bsl: Some(BuiltinBsl::Str),
        facets: &[],
    },
    BuiltinType {
        name: "string",
        base: Some("anySimpleType"),
        bsl: Some(BuiltinBsl::Str),
        facets: &[(FacetKind::WhiteSpace, "preserve")],
    },
    BuiltinType {
        name: "normalizedString",
        base: Some("string"),
        bsl: Some(BuiltinBsl::Str),
        facets: &[(FacetKind::WhiteSpace, "replace")],
    },
    BuiltinType {
        name: "token",
        base: Some("normalizedString"),
        bsl: Some(BuiltinBsl::Str),
        facets: &[(FacetKind::WhiteSpace, "collapse")],
    },
    BuiltinType {
        name: "Name",
        base: Some("token"),
        bsl: Some(BuiltinBsl::Str),
        facets: &[(FacetKind::Pattern, r"\i\c*")],
    },
    BuiltinType {
        name: "NCName",
        base: Some("Name"),
        bsl: Some(BuiltinBsl::Str),
        facets: &[(FacetKind::Pattern, r"[\i-[:]][\c-[:]]*")],
    },
    BuiltinType {
        name: "ID",
        base: Some("NCName"),
        bsl: Some(BuiltinBsl::Str),
        facets: &[],
    },
    BuiltinType {
        name: "NMTOKEN",
        base: Some("token"),
        bsl: Some(BuiltinBsl::Str),
        facets: &[],
    },
    BuiltinType {
        name: "language",
        base: Some("token"),
        bsl: Some(BuiltinBsl::Str),
        facets: &[(FacetKind::Pattern, "[a-zA-Z]{1,8}(-[a-zA-Z0-9]{1,8})*")],
    },
    BuiltinType {
        name: "decimal",
        base: Some("anySimpleType"),
        bsl: Some(BuiltinBsl::Number),
        facets: &[],
    },
    BuiltinType {
        name: "integer",
        base: Some("decimal"),
        bsl: Some(BuiltinBsl::Number),
        facets: &[
            (FacetKind::FractionDigits, "0"),
            (FacetKind::Pattern, r"[\-+]?[0-9]+"),
        ],
    },
    BuiltinType {
        name: "long",
        base: Some("integer"),
        bsl: Some(BuiltinBsl::Number),
        facets: &[
            (FacetKind::MinInclusive, "-9223372036854775808"),
            (FacetKind::MaxInclusive, "9223372036854775807"),
        ],
    },
    BuiltinType {
        name: "int",
        base: Some("long"),
        bsl: Some(BuiltinBsl::Number),
        facets: &[
            (FacetKind::MinInclusive, "-2147483648"),
            (FacetKind::MaxInclusive, "2147483647"),
        ],
    },
    BuiltinType {
        name: "short",
        base: Some("int"),
        bsl: Some(BuiltinBsl::Number),
        facets: &[
            (FacetKind::MinInclusive, "-32768"),
            (FacetKind::MaxInclusive, "32767"),
        ],
    },
    BuiltinType {
        name: "byte",
        base: Some("short"),
        bsl: Some(BuiltinBsl::Number),
        facets: &[
            (FacetKind::MinInclusive, "-128"),
            (FacetKind::MaxInclusive, "127"),
        ],
    },
    BuiltinType {
        name: "nonNegativeInteger",
        base: Some("integer"),
        bsl: Some(BuiltinBsl::Number),
        facets: &[(FacetKind::MinInclusive, "0")],
    },
    BuiltinType {
        name: "positiveInteger",
        base: Some("nonNegativeInteger"),
        bsl: Some(BuiltinBsl::Number),
        facets: &[(FacetKind::MinInclusive, "1")],
    },
    BuiltinType {
        name: "nonPositiveInteger",
        base: Some("integer"),
        bsl: Some(BuiltinBsl::Number),
        facets: &[(FacetKind::MaxInclusive, "0")],
    },
    BuiltinType {
        name: "negativeInteger",
        base: Some("nonPositiveInteger"),
        bsl: Some(BuiltinBsl::Number),
        facets: &[(FacetKind::MaxInclusive, "-1")],
    },
    BuiltinType {
        name: "unsignedLong",
        base: Some("nonNegativeInteger"),
        bsl: Some(BuiltinBsl::Number),
        facets: &[(FacetKind::MaxInclusive, "18446744073709551615")],
    },
    BuiltinType {
        name: "unsignedInt",
        base: Some("unsignedLong"),
        bsl: Some(BuiltinBsl::Number),
        facets: &[(FacetKind::MaxInclusive, "4294967295")],
    },
    BuiltinType {
        name: "unsignedShort",
        base: Some("unsignedInt"),
        bsl: Some(BuiltinBsl::Number),
        facets: &[(FacetKind::MaxInclusive, "65535")],
    },
    BuiltinType {
        name: "unsignedByte",
        base: Some("unsignedShort"),
        bsl: Some(BuiltinBsl::Number),
        facets: &[(FacetKind::MaxInclusive, "255")],
    },
    BuiltinType {
        name: "double",
        base: Some("anySimpleType"),
        bsl: Some(BuiltinBsl::Double),
        facets: &[],
    },
    BuiltinType {
        name: "float",
        base: Some("anySimpleType"),
        bsl: Some(BuiltinBsl::Double),
        facets: &[],
    },
    BuiltinType {
        name: "boolean",
        base: Some("anySimpleType"),
        bsl: Some(BuiltinBsl::Boolean),
        facets: &[],
    },
    BuiltinType {
        name: "date",
        base: Some("anySimpleType"),
        bsl: Some(BuiltinBsl::Date),
        facets: &[],
    },
    BuiltinType {
        name: "dateTime",
        base: Some("anySimpleType"),
        bsl: Some(BuiltinBsl::DateTime),
        facets: &[],
    },
    BuiltinType {
        name: "time",
        base: Some("anySimpleType"),
        bsl: Some(BuiltinBsl::Time),
        facets: &[],
    },
    BuiltinType {
        name: "duration",
        base: Some("anySimpleType"),
        bsl: Some(BuiltinBsl::Str),
        facets: &[],
    },
    BuiltinType {
        name: "gYear",
        base: Some("anySimpleType"),
        bsl: Some(BuiltinBsl::Str),
        facets: &[],
    },
    BuiltinType {
        name: "base64Binary",
        base: Some("anySimpleType"),
        bsl: Some(BuiltinBsl::Base64),
        facets: &[],
    },
    BuiltinType {
        name: "hexBinary",
        base: Some("anySimpleType"),
        bsl: Some(BuiltinBsl::Hex),
        facets: &[],
    },
    BuiltinType {
        name: "anyURI",
        base: Some("anySimpleType"),
        bsl: Some(BuiltinBsl::Str),
        facets: &[],
    },
    BuiltinType {
        name: "QName",
        base: Some("anySimpleType"),
        bsl: Some(BuiltinBsl::QName),
        facets: &[],
    },
];
