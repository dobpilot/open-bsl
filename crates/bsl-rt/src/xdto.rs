//! Модель типов XDTO поверх компонентной модели XSD: `ТипЗначенияXDTO`,
//! `ТипОбъектаXDTO`, `СвойствоXDTO` и соответствие встроенных типов XML
//! Schema типам BSL.
//!
//! Второго разборщика схем здесь нет: на вход идёт готовая `XsSchemaData`
//! из [`crate::xsd`], а этот модуль превращает ЛЕКСИЧЕСКУЮ модель схемы в
//! РАЗРЕШЁННУЮ модель типов — ту, где ссылки уже связаны, наследование
//! сплющено, а границы вхождения посчитаны. Ровно этим две модели и
//! различаются: `ОбъявлениеЭлементаXS` показывает написанное, а
//! `СвойствоXDTO` — вычисленное.
//!
//! # Что ИЗМЕРЕНО на 8.3.27
//!
//! Всё перечисленное снято пробами (`tests/conformance/measure/measure-xdto.bsl`,
//! рядом лежит снятый `measure-xdto.platform.txt`), а не взято из справки.
//! Проб потребовалось много, потому что справка щедра на члены, которых у
//! платформы нет: у `ТипЗначенияXDTO` отвергнуты `Вариант`,
//! `ВариантПростогоТипа`, `Абстрактный`, `Длина`, `МинимальнаяДлина`,
//! `МаксимальнаяДлина`, `Образцы`, `Перечисления`, `ПробельныеСимволы`,
//! `ТипЭлемента`, `ТипыОбъединения`, `Пакет`, `Фабрика`, `Создать`,
//! `ПроверитьЗначение`, `ЭтоНаследник`, у `ТипОбъектаXDTO` — `Фабрика`,
//! `Пакет`, `Создать`, `ЭтоНаследник`, `ПолучитьСвойство`, у
//! `СвойствоXDTO` — `Владелец`, `Нулевой`, `Обязательный`, `Фиксированный`,
//! `Локальный`, `Порядок`, `ЛексическоеЗначениеПоУмолчанию`, `ФормаXML`.
//!
//! * все три типа существуют под двумя написаниями и печатаются с
//!   пробелами: `ТипЗначенияXDTO`/`XDTOValueType` -> «Тип значения XDTO»,
//!   `ТипОбъектаXDTO`/`XDTOObjectType` -> «Тип объекта XDTO»,
//!   `СвойствоXDTO`/`XDTOProperty` -> «Свойство XDTO». `Новый` ни один из
//!   них не строит (ошибка на всех трёх), а `Новый ФабрикаXDTO` —
//!   наоборот, работает;
//! * `Строка()` от типа — это `{URI}Имя` (`{urn:test}RootType`), но у
//!   АНОНИМНОГО типа, чьё имя пусто, — пустая строка, хотя `URI` у него при
//!   этом целевое пространство схемы. `Строка()` от свойства — его имя;
//! * тип ищется в фабрике парой (URI, имя) или `РасширенноеИмяXML`; одной
//!   строкой — ошибка, неизвестное имя — `Неопределено`, а объявление
//!   глобального ЭЛЕМЕНТА типом не является (`Тип("urn:test", "root")` —
//!   `Неопределено`). Два обращения к `Тип` за одним именем дают РАВНЫЕ
//!   значения, то есть тип — это ссылка на место в модели;
//! * `БазовыйТип` у типа объекта без явного базового — `{...}anyType`, а у
//!   типа значения без явного — `{...}anySimpleType`. У составного типа с
//!   ПРОСТЫМ содержимым (`xs:simpleContent`) базовый тип тоже `anyType`:
//!   простой базовый тип виден не через `БазовыйТип`, а через свойство
//!   `__content`;
//! * `Свойства` типа объекта — СПЛЮЩЕННЫЙ список: сначала свойства
//!   базового типа, потом собственные АТРИБУТЫ, потом собственные
//!   ЭЛЕМЕНТЫ. Измерено на схеме, где у базового типа четыре атрибута и
//!   девять элементов, а у наследника по одному своему:
//!   `id color opt q fx name code дата def many5 notype efx uq anon` и у
//!   наследника `… anon ea extra`;
//! * границы вхождения свойства — `НижняяГраница`/`ВерхняяГраница`,
//!   ЧИСЛА, и `unbounded` показывается числом `-1`. Лексическая модель
//!   схемы отвечает на то же самое иначе: `МаксимальноВходит` частицы с
//!   `maxOccurs="unbounded"` — это СТРОКА `unbounded` (измерено, см.
//!   [`crate::xsd`]). Границы ПЕРЕМНОЖАЮТСЯ по вложенным группам модели:
//!   `<xs:choice minOccurs="0">` превращает `1..1` вложенного элемента в
//!   `0..1`, а `<xs:sequence maxOccurs="unbounded">` — `0..1` в `0..-1`;
//! * `Форма` — член `ФормаXML` (`Элемент`, `Атрибут`, `Текст`), у
//!   `ФормаXML` есть и английские написания членов. `URIПространстваИмен`
//!   свойства подчиняется тому же правилу форм, что и в модели XSD:
//!   квалифицированное объявление даёт целевое пространство, неквалифи-
//!   цированное — пустую строку;
//! * составной тип с `xs:simpleContent` получает свойство с именем
//!   `__content`, формой `Текст`, границами `1..1`, пустым URI и типом
//!   базового простого типа. У СМЕШАННОГО типа (`mixed="true"`) такого
//!   свойства НЕТ — там только объявленные элементы;
//! * `Упорядоченный` — «Да» у последовательности, у пустого типа и у
//!   простого содержимого, «Нет» у `xs:choice` и `xs:all`;
//!   `Последовательный` во всех измеренных случаях равен «НЕ
//!   Упорядоченный ИЛИ Смешанный» (проверено на семи типах, включая
//!   `anyType`, у которого «Да» и то и другое). `Открытый` — «Да» ровно у
//!   `anyType`;
//! * `Фасеты` типа значения — `КоллекцияФасетовXDTO`, но у типа БЕЗ
//!   фасетов это `Неопределено`, а не пустая коллекция (измерено на
//!   `xs:date`). `ФасетXDTO` отдаёт ровно два члена: `Вид` (член
//!   `ВидФасетаXDTO`) и `Значение`, причём `Значение` — всегда СТРОКА,
//!   даже у числовых фасетов (`Минимальное включающее значение=[0]`);
//! * `ЗначениеПоУмолчанию` свойства — `ЗначениеXDTO` с членами `Значение`
//!   (значение BSL) и `ЛексическоеЗначение` (строка). Заполняется и от
//!   `default`, и от `fixed`; у свойства без того и другого —
//!   `Неопределено`;
//! * встроенные типы XML Schema образуют ИЕРАРХИЮ с фасетами: `string`
//!   наследует `anySimpleType` и несёт `Пробельные символы=[preserve]`,
//!   `int` наследует `long` и несёт границы диапазона
//!   `[-2147483648, 2147483647]`, `integer` наследует `decimal` с
//!   `Количество разрядов дробной части=[0]`. Вся таблица снята поимённо —
//!   см. [`BUILTIN_TYPES`];
//! * отображение встроенного типа в тип BSL снято через
//!   `ФабрикаXDTO.Создать(Тип, Лексика).Значение`. Свой тип наследует
//!   отображение базового (`Code` от `xs:string` -> `Строка`, `Small` от
//!   `xs:decimal` -> `Число`), СПИСОК даёт `ФиксированныйМассив`, а
//!   ОБЪЕДИНЕНИЕ выбирает первый член, который принимает лексическую
//!   форму: у `union memberTypes="xs:int xs:string"` запись «5» дала
//!   `Число`, а «аб» — `Строка`.
//!
//! # Сознательные расхождения и незакрытые углы
//!
//! * **Фасеты только хранятся.** Платформа ПРОВЕРЯЕТ по ним лексическую
//!   форму (измерено: `Создать` от `Small` с «1000» и от `Code` с «аб» —
//!   ошибка). Здесь фасет только читается: проверка образца требует
//!   движка регулярных выражений и делается отдельной задачей.
//! * **Двоичные лексические формы.** `base64Binary` и `hexBinary`
//!   отображаются в `ДвоичныеДанные` (измерено), и разбор обеих записей
//!   здесь есть, но обратной операции — двоичные данные в лексическую
//!   форму — нет: она нужна записи XML, а не модели типов.
//! * **QName с префиксом.** Платформа принимает только запись БЕЗ
//!   префикса (`Создать` от `xs:string` — ошибка, от `просто` —
//!   расширенное имя с пустым URI). Здесь так же: префикс — ошибка, а не
//!   попытка разрешить его по объявлениям схемы.

use std::rc::Rc;

use crate::object::BslObject;
use crate::string::BslString;
use crate::types::TypeId;
use crate::xsd::{FacetKind, XName, XsKind, XsSchemaData, XSD_NS};
use crate::{BslValue, EnumValue, RtError, RtResult};

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
    /// `ТипЗнч()` значения, которое получится из лексической формы.
    pub fn type_id(self) -> TypeId {
        match self {
            BuiltinBsl::Str => TypeId::String,
            BuiltinBsl::Number | BuiltinBsl::Double => TypeId::Number,
            BuiltinBsl::Boolean => TypeId::Boolean,
            BuiltinBsl::Date | BuiltinBsl::DateTime | BuiltinBsl::Time => TypeId::Date,
            BuiltinBsl::Base64 | BuiltinBsl::Hex => TypeId::BinaryData,
            BuiltinBsl::QName => TypeId::XmlExpandedName,
        }
    }
}

/// Строка таблицы встроенных типов пространства
/// `http://www.w3.org/2001/XMLSchema`.
struct BuiltinType {
    name: &'static str,
    /// Имя базового встроенного типа; `None` только у `anyType`.
    base: Option<&'static str>,
    /// Отображение в тип BSL; `None` — это ТИП ОБЪЕКТА (`anyType`),
    /// значения из лексической формы он не строит.
    bsl: Option<BuiltinBsl>,
    /// Фасеты в том порядке, в каком их отдаёт платформа.
    facets: &'static [(FacetKind, &'static str)],
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
static BUILTIN_TYPES: &[BuiltinType] = &[
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

// --- модель --------------------------------------------------------------

/// Устройство типа ЗНАЧЕНИЯ — то, от чего зависит разбор лексической
/// формы.
#[derive(Debug, Clone)]
enum ValueShape {
    /// Встроенный тип с прямым отображением в тип BSL.
    Builtin(BuiltinBsl),
    /// Атомарный производный тип: отображение берётся у базового
    /// (измерено на `Code` и `Small`).
    Atomic,
    /// Список: значение — `ФиксированныйМассив` значений типа элемента,
    /// лексическая форма разделяется пробельными символами.
    List(Option<usize>),
    /// Объединение: тип выбирается ПЕРВЫМ членом, который принимает
    /// лексическую форму (измерено).
    Union(Vec<usize>),
}

/// Тип модели: и тип значения, и тип объекта — разницу несёт `shape`.
#[derive(Debug)]
struct XdtoTypeData {
    name: String,
    ns: String,
    /// Базовый тип; `None` только у `anyType`.
    base: Option<usize>,
    /// `None` — это тип ОБЪЕКТА.
    shape: Option<ValueShape>,
    /// Фасеты типа значения: вид и лексическая запись значения.
    facets: Vec<(FacetKind, String)>,
    /// Свойства типа объекта — уже сплющенные вместе с унаследованными.
    properties: Vec<usize>,
    open: bool,
    is_abstract: bool,
    ordered: bool,
    mixed: bool,
}

impl XdtoTypeData {
    fn is_value(&self) -> bool {
        self.shape.is_some()
    }

    /// `Последовательный` — во всех измеренных случаях «НЕ Упорядоченный
    /// ИЛИ Смешанный»: последовательность даёт «Нет», `xs:choice` и
    /// `xs:all` — «Да», смешанный тип и `anyType` — «Да».
    fn sequenced(&self) -> bool {
        !self.ordered || self.mixed
    }
}

/// Свойство типа объекта.
#[derive(Debug)]
struct XdtoPropertyData {
    name: String,
    ns: String,
    type_index: usize,
    /// `None` — `unbounded`; наружу обе границы уходят числом, где
    /// `unbounded` — это `-1` (измерено).
    lower: Option<u32>,
    upper: Option<u32>,
    /// Член `ФормаXML`.
    form: EnumValue,
    default: Option<Rc<XdtoValueData>>,
}

/// `ЗначениеXDTO` — значение BSL вместе с лексической формой, из которой
/// оно получено.
#[derive(Debug)]
pub struct XdtoValueData {
    value: BslValue,
    lexical: String,
}

/// Разрешённая модель типов одной схемы вместе со встроенными типами XML
/// Schema. Значение `ТипЗначенияXDTO` — это `Rc` на неё плюс номер типа,
/// `СвойствоXDTO` — тот же `Rc` плюс номер свойства.
#[derive(Debug)]
pub struct XdtoModel {
    types: Vec<XdtoTypeData>,
    properties: Vec<XdtoPropertyData>,
}

impl XdtoModel {
    fn type_at(&self, i: usize) -> RtResult<&XdtoTypeData> {
        self.types.get(i).ok_or_else(|| broken("тип"))
    }

    fn property_at(&self, i: usize) -> RtResult<&XdtoPropertyData> {
        self.properties.get(i).ok_or_else(|| broken("свойство"))
    }

    /// Тип по расширенному имени — то, что делает `ФабрикаXDTO.Тип(URI,
    /// Имя)`. Анонимные типы сюда не попадают: у них нет имени.
    pub fn find(&self, uri: &str, name: &str) -> Option<usize> {
        if name.is_empty() {
            return None;
        }
        self.types
            .iter()
            .position(|t| t.name == name && t.ns == uri)
    }

    /// Тип BSL, в который отображается тип значения; `None` — у типа
    /// объекта либо у списка и объединения, где тип зависит от значения.
    pub fn builtin_of(&self, index: usize) -> Option<BuiltinBsl> {
        let mut cur = index;
        // Цепочка базовых типов конечна: длина модели — верхняя граница,
        // и она же страхует от цикла в испорченной схеме.
        for _ in 0..=self.types.len() {
            match self.types.get(cur)?.shape.as_ref()? {
                ValueShape::Builtin(b) => return Some(*b),
                ValueShape::List(_) | ValueShape::Union(_) => return None,
                ValueShape::Atomic => cur = self.types.get(cur)?.base?,
            }
        }
        None
    }
}

fn broken(what: &str) -> RtError {
    RtError::Xdto(format!("модель типов XDTO повреждена: нет узла «{what}»"))
}

// --- построение ----------------------------------------------------------

/// Модель типов по разобранной схеме.
///
/// # Errors
///
/// [`RtError::Xdto`], если схема ссылается на неизвестный тип, содержит
/// цикл наследования либо значение по умолчанию, которое не разбирается в
/// объявленном типе.
pub fn model_of_schema(schema: &Rc<XsSchemaData>) -> RtResult<Rc<XdtoModel>> {
    let mut builder = Builder::new(schema);
    builder.declare_builtins();
    builder.declare_schema_types()?;
    builder.link_bases()?;
    builder.build_properties()?;
    Ok(Rc::new(builder.model))
}

struct Builder<'a> {
    schema: &'a XsSchemaData,
    model: XdtoModel,
    /// Номер узла XSD -> номер типа модели, для типов, объявленных схемой.
    from_xs: Vec<(usize, usize)>,
    /// Номер типа модели -> номер узла XSD, откуда он построен.
    to_xs: Vec<Option<usize>>,
    /// Тип, чьи свойства сейчас считаются, — страховка от цикла
    /// наследования.
    busy: Vec<bool>,
    done: Vec<bool>,
}

impl<'a> Builder<'a> {
    fn new(schema: &'a XsSchemaData) -> Builder<'a> {
        Builder {
            schema,
            model: XdtoModel {
                types: Vec::new(),
                properties: Vec::new(),
            },
            from_xs: Vec::new(),
            to_xs: Vec::new(),
            busy: Vec::new(),
            done: Vec::new(),
        }
    }

    fn push_type(&mut self, data: XdtoTypeData, xs: Option<usize>) -> usize {
        self.model.types.push(data);
        self.to_xs.push(xs);
        self.busy.push(false);
        self.done.push(false);
        if let Some(node) = xs {
            self.from_xs.push((node, self.model.types.len() - 1));
        }
        self.model.types.len() - 1
    }

    /// Встроенные типы пространства XML Schema — они есть у любой фабрики
    /// (измерено: `Новый ФабрикаXDTO` уже знает `{...}string`).
    fn declare_builtins(&mut self) {
        for row in BUILTIN_TYPES {
            // Единственный встроенный тип ОБЪЕКТА — `anyType`, и три его
            // флага измерены разом: открытый, упорядоченный и смешанный.
            // У всех остальных встроенных (это типы значения) те же флаги
            // не читаются вовсе.
            let is_any_type = row.bsl.is_none();
            self.push_type(
                XdtoTypeData {
                    name: row.name.to_string(),
                    ns: XSD_NS.to_string(),
                    base: None,
                    shape: row.bsl.map(ValueShape::Builtin),
                    facets: row
                        .facets
                        .iter()
                        .map(|(k, v)| (*k, (*v).to_string()))
                        .collect(),
                    properties: Vec::new(),
                    open: is_any_type,
                    is_abstract: false,
                    ordered: is_any_type,
                    mixed: is_any_type,
                },
                None,
            );
        }
        for (i, row) in BUILTIN_TYPES.iter().enumerate() {
            let base = row.base.and_then(|name| self.model.find(XSD_NS, name));
            self.model.types[i].base = base;
        }
    }

    /// Именованные глобальные типы схемы. Анонимные объявляются позже, при
    /// разборе свойств: на них ссылается только своё свойство.
    fn declare_schema_types(&mut self) -> RtResult<()> {
        // Номера копируются, потому что `declare_type` берёт `&mut self`,
        // а список живёт в схеме за общей ссылкой.
        let nodes: Vec<usize> = self.schema.global_types().to_vec();
        for node in nodes {
            self.declare_type(node)?;
        }
        Ok(())
    }

    fn declare_type(&mut self, node: usize) -> RtResult<usize> {
        if let Some((_, idx)) = self.from_xs.iter().find(|(n, _)| *n == node) {
            return Ok(*idx);
        }
        // Пространство имён у типа модели — ЦЕЛЕВОЕ пространство схемы, и
        // у анонимного тоже, хотя имя у него пусто (измерено: у типа
        // безымянного `<xs:complexType>` внутри объявления `URI` —
        // `urn:test`). Лексическая модель XSD здесь другая: там у
        // анонимного типа пространство имён пусто.
        let target_ns = self.schema.target_namespace().to_string();
        let name = self.schema.name_of(node).to_string();
        let data = match self.schema.kind_of(node) {
            XsKind::SimpleType => {
                let shape = match self.schema.simple_variety_of(node) {
                    Some((EnumValue::XsVarietyList, _, _)) => ValueShape::List(None),
                    Some((EnumValue::XsVarietyUnion, _, _)) => ValueShape::Union(Vec::new()),
                    _ => ValueShape::Atomic,
                };
                XdtoTypeData {
                    name,
                    ns: target_ns,
                    base: None,
                    shape: Some(shape),
                    facets: self
                        .schema
                        .facets_of(node)
                        .into_iter()
                        .map(|(k, v)| (k, v.to_string()))
                        .collect(),
                    properties: Vec::new(),
                    // Четыре флага ниже читаются только у типа ОБЪЕКТА
                    // (у типа значения обращение к ним платформа
                    // отвергает), поэтому у типов значения они выключены
                    // все и одинаково — и у схемных, и у встроенных.
                    open: false,
                    is_abstract: false,
                    ordered: false,
                    mixed: false,
                }
            }
            XsKind::ComplexType => {
                let (mixed, is_abstract) = self.schema.complex_flags_of(node);
                XdtoTypeData {
                    name,
                    ns: target_ns,
                    base: None,
                    shape: None,
                    facets: Vec::new(),
                    properties: Vec::new(),
                    open: false,
                    is_abstract,
                    ordered: self.content_is_ordered(node),
                    mixed,
                }
            }
            other => {
                return Err(RtError::Xdto(format!(
                    "типом XDTO может стать только определение типа, а не «{}»",
                    other.type_name()
                )))
            }
        };
        Ok(self.push_type(data, Some(node)))
    }

    /// `Упорядоченный` — «Да» у последовательности и у типа без модели
    /// содержимого, «Нет» у `xs:choice` и `xs:all` (измерено на пяти
    /// типах).
    fn content_is_ordered(&self, node: usize) -> bool {
        let Some(particle) = self.schema.complex_content_of(node) else {
            return true;
        };
        let Some((term, _, _)) = self.schema.particle_of(particle) else {
            return true;
        };
        match self.schema.model_group_of(term) {
            Some((EnumValue::XsGroupSequence, _)) => true,
            Some(_) => false,
            None => true,
        }
    }

    /// Базовые типы схемных типов: имя из `base` разрешается в номер.
    fn link_bases(&mut self) -> RtResult<()> {
        for i in 0..self.model.types.len() {
            let Some(node) = self.to_xs[i] else {
                continue;
            };
            let base = if self.model.types[i].is_value() {
                let name = self.schema.simple_base_of(node).cloned();
                match name {
                    Some(n) => Some(self.require_type(&n)?),
                    // Тип значения без явного базового наследует
                    // `anySimpleType` (измерено на списке и объединении).
                    None => self.model.find(XSD_NS, "anySimpleType"),
                }
            } else {
                // У типа объекта базовым становится только ОБЪЕКТНЫЙ
                // базовый тип: у составного типа с простым содержимым
                // платформа отдаёт `anyType`, а простой базовый тип
                // виден свойством `__content` (измерено).
                let name = self.schema.complex_base_of(node).cloned();
                let resolved = match name {
                    Some(n) => Some(self.require_type(&n)?),
                    None => None,
                };
                match resolved {
                    Some(b) if !self.model.types[b].is_value() => Some(b),
                    _ => self.model.find(XSD_NS, "anyType"),
                }
            };
            self.model.types[i].base = base;
        }
        // Тип элемента списка и члены объединения — по тем же именам.
        for i in 0..self.model.types.len() {
            let Some(node) = self.to_xs[i] else {
                continue;
            };
            let Some((variety, item, members)) = self.schema.simple_variety_of(node) else {
                continue;
            };
            let shape = match variety {
                EnumValue::XsVarietyList => {
                    let item = match item.cloned() {
                        Some(n) => Some(self.require_type(&n)?),
                        None => None,
                    };
                    ValueShape::List(item)
                }
                EnumValue::XsVarietyUnion => {
                    let names: Vec<XName> = members.to_vec();
                    let mut resolved = Vec::with_capacity(names.len());
                    for n in &names {
                        resolved.push(self.require_type(n)?);
                    }
                    ValueShape::Union(resolved)
                }
                _ => continue,
            };
            self.model.types[i].shape = Some(shape);
        }
        Ok(())
    }

    /// Тип по имени — с ошибкой вместо `Неопределено`: ссылка на
    /// несуществующий тип делает модель неполной, и молчать об этом хуже,
    /// чем отказать.
    fn require_type(&self, name: &XName) -> RtResult<usize> {
        self.model.find(&name.uri, &name.local).ok_or_else(|| {
            RtError::Xdto(format!(
                "в схеме нет типа «{}», на который ссылается модель",
                name.display_text()
            ))
        })
    }

    fn build_properties(&mut self) -> RtResult<()> {
        for i in 0..self.model.types.len() {
            self.ensure_properties(i)?;
        }
        Ok(())
    }

    /// Свойства типа объекта: сначала унаследованные, потом собственные
    /// атрибуты, потом собственные элементы (измеренный порядок).
    fn ensure_properties(&mut self, index: usize) -> RtResult<()> {
        if self.done[index] {
            return Ok(());
        }
        if self.busy[index] {
            return Err(RtError::Xdto(format!(
                "циклическое наследование типов XDTO вокруг «{}»",
                self.model.types[index].name
            )));
        }
        self.busy[index] = true;
        let mut props = Vec::new();
        if let Some(base) = self.model.types[index].base {
            if !self.model.types[base].is_value() {
                self.ensure_properties(base)?;
                props.extend_from_slice(&self.model.types[base].properties);
            }
        }
        if let Some(node) = self.to_xs[index] {
            if !self.model.types[index].is_value() {
                self.collect_attributes(node, &mut props)?;
                self.collect_content(node, &mut props)?;
            }
        }
        self.model.types[index].properties = props;
        self.busy[index] = false;
        self.done[index] = true;
        Ok(())
    }

    /// Собственные атрибуты составного типа. Обязательный атрибут даёт
    /// границы `1..1`, необязательный — `0..1` (измерено).
    fn collect_attributes(&mut self, node: usize, out: &mut Vec<usize>) -> RtResult<()> {
        let uses: Vec<usize> = self.schema.complex_attribute_uses_of(node).to_vec();
        for use_node in uses {
            let Some(view) = self.schema.attribute_use_of(use_node) else {
                continue;
            };
            let (decl_node, required, lexical, has_constraint) = (
                view.declaration,
                view.required,
                view.lexical.to_string(),
                view.has_constraint,
            );
            let Some(decl) = self.schema.decl_of(decl_node) else {
                continue;
            };
            let (name, ns) = (decl.name.to_string(), decl.ns.to_string());
            let type_index = self.property_type(decl_node)?;
            let default = if has_constraint {
                Some(self.value_of(type_index, &lexical)?)
            } else {
                None
            };
            let property = XdtoPropertyData {
                name,
                ns,
                type_index,
                lower: Some(u32::from(required)),
                upper: Some(1),
                form: EnumValue::XmlFormAttribute,
                default,
            };
            self.model.properties.push(property);
            out.push(self.model.properties.len() - 1);
        }
        Ok(())
    }

    /// Собственное содержимое: либо элементы модели содержимого, либо
    /// текстовое свойство `__content` у типа с простым содержимым.
    fn collect_content(&mut self, node: usize, out: &mut Vec<usize>) -> RtResult<()> {
        if let Some(particle) = self.schema.complex_content_of(node) {
            return self.collect_elements(particle, Some(1), Some(1), out);
        }
        // Простое содержимое: базовый тип — простой, и платформа
        // показывает его свойством `__content` с формой `Текст`
        // (измерено). Отличать `xs:simpleContent` от `xs:complexContent`
        // отдельным признаком не нужно: у простого содержимого нет модели
        // содержимого, а базовый тип — тип ЗНАЧЕНИЯ, и обе проверки уже
        // сделаны выше. СМЕШАННЫЙ тип сюда не доходит: модель содержимого
        // у него есть, и своего текстового свойства платформа ему не даёт
        // (измерено на `mixed="true"` — там только объявленный элемент).
        let Some(base_name) = self.schema.complex_base_of(node).cloned() else {
            return Ok(());
        };
        let base = self.require_type(&base_name)?;
        if !self.model.types[base].is_value() {
            return Ok(());
        }
        self.model.properties.push(XdtoPropertyData {
            name: CONTENT_PROPERTY.to_string(),
            ns: String::new(),
            type_index: base,
            lower: Some(1),
            upper: Some(1),
            form: EnumValue::XmlFormText,
            default: None,
        });
        out.push(self.model.properties.len() - 1);
        Ok(())
    }

    /// Разложить фрагмент в свойства, перемножая границы вхождения по
    /// вложенным группам модели.
    fn collect_elements(
        &mut self,
        particle: usize,
        outer_lower: Option<u32>,
        outer_upper: Option<u32>,
        out: &mut Vec<usize>,
    ) -> RtResult<()> {
        let Some((term, min, max)) = self.schema.particle_of(particle) else {
            return Ok(());
        };
        let lower = fold_bounds(outer_lower, bound_of(min, 1));
        let upper = fold_bounds(outer_upper, bound_of(max, 1));
        if let Some((_, particles)) = self.schema.model_group_of(term) {
            let inner: Vec<usize> = particles.to_vec();
            for p in inner {
                self.collect_elements(p, lower, upper, out)?;
            }
            return Ok(());
        }
        let Some(decl) = self.schema.decl_of(term) else {
            return Err(RtError::Xdto(
                "термом фрагмента может быть объявление элемента или группа модели".to_string(),
            ));
        };
        let (name, ns, lexical, has_constraint) = (
            decl.name.to_string(),
            decl.ns.to_string(),
            decl.lexical.to_string(),
            decl.has_constraint,
        );
        let type_index = self.property_type(term)?;
        let default = if has_constraint {
            Some(self.value_of(type_index, &lexical)?)
        } else {
            None
        };
        self.model.properties.push(XdtoPropertyData {
            name,
            ns,
            type_index,
            lower,
            upper,
            form: EnumValue::XmlFormElement,
            default,
        });
        out.push(self.model.properties.len() - 1);
        Ok(())
    }

    /// Тип свойства: объявленный `type`, встроенный анонимный тип или —
    /// если ни того, ни другого нет — `anyType` (измерено на
    /// `<xs:element name="notype"/>`).
    fn property_type(&mut self, decl_node: usize) -> RtResult<usize> {
        let (type_name, anonymous) = match self.schema.decl_of(decl_node) {
            Some(d) => (d.type_name.cloned(), d.anonymous_type),
            None => (None, None),
        };
        if let Some(name) = type_name {
            return self.require_type(&name);
        }
        if let Some(node) = anonymous {
            let index = self.declare_type(node)?;
            // Анонимный тип объявлен уже после связывания базовых типов,
            // поэтому его база и свойства достраиваются здесь же.
            self.link_one_base(index, node)?;
            self.ensure_properties(index)?;
            return Ok(index);
        }
        self.model
            .find(XSD_NS, "anyType")
            .ok_or_else(|| broken("anyType"))
    }

    /// Базовый тип одного (анонимного) типа — та же логика, что в
    /// [`Builder::link_bases`], но для типа, объявленного позже.
    fn link_one_base(&mut self, index: usize, node: usize) -> RtResult<()> {
        if self.model.types[index].base.is_some() {
            return Ok(());
        }
        let base = if self.model.types[index].is_value() {
            match self.schema.simple_base_of(node).cloned() {
                Some(n) => Some(self.require_type(&n)?),
                None => self.model.find(XSD_NS, "anySimpleType"),
            }
        } else {
            let resolved = match self.schema.complex_base_of(node).cloned() {
                Some(n) => Some(self.require_type(&n)?),
                None => None,
            };
            match resolved {
                Some(b) if !self.model.types[b].is_value() => Some(b),
                _ => self.model.find(XSD_NS, "anyType"),
            }
        };
        self.model.types[index].base = base;
        Ok(())
    }

    fn value_of(&self, type_index: usize, lexical: &str) -> RtResult<Rc<XdtoValueData>> {
        Ok(Rc::new(XdtoValueData {
            value: value_from_lexical(&self.model, type_index, lexical)?,
            lexical: lexical.to_string(),
        }))
    }
}

/// Имя свойства, которым платформа показывает текст типа с простым
/// содержимым (измерено).
const CONTENT_PROPERTY: &str = "__content";

/// Граница вхождения из лексической модели XSD: отсутствующий атрибут —
/// это `default`, а `unbounded` (то есть `u32::MAX`) — `None`.
fn bound_of(raw: Option<u32>, default: u32) -> Option<u32> {
    match raw {
        None => Some(default),
        Some(u32::MAX) => None,
        Some(n) => Some(n),
    }
}

/// Границы перемножаются по вложенным группам модели (измерено:
/// `<xs:choice minOccurs="0">` делает `1..1` вложенного элемента `0..1`, а
/// `<xs:sequence maxOccurs="unbounded">` делает `0..1` -> `0..-1`). Ноль
/// поглощает бесконечность: вхождений всё равно ноль.
fn fold_bounds(outer: Option<u32>, inner: Option<u32>) -> Option<u32> {
    match (outer, inner) {
        (Some(0), _) | (_, Some(0)) => Some(0),
        (Some(a), Some(b)) => Some(a.saturating_mul(b)),
        _ => None,
    }
}

// --- лексические формы ---------------------------------------------------

/// Значение BSL из лексической формы по типу модели.
///
/// # Errors
///
/// [`RtError::Xdto`], если тип — объектный, если лексическая форма не
/// разбирается в его отображении либо если ни один член объединения её не
/// принял.
pub fn value_from_lexical(
    model: &XdtoModel,
    type_index: usize,
    lexical: &str,
) -> RtResult<BslValue> {
    // Глубины хватает на любую честную схему: цепочка «производный тип ->
    // базовый» не длиннее числа типов, а список и объединение добавляют
    // по шагу на уровень вложенности. Ограничение здесь не оптимизация, а
    // страховка: `<xs:restriction base="t:A"/>` внутри самого `A` даёт
    // КОЛЬЦО, и без счётчика разбор ушёл бы в переполнение стека.
    value_from_lexical_at(model, type_index, lexical, model.types.len() + 8)
}

fn value_from_lexical_at(
    model: &XdtoModel,
    type_index: usize,
    lexical: &str,
    depth: usize,
) -> RtResult<BslValue> {
    let data = model.type_at(type_index)?;
    let Some(depth) = depth.checked_sub(1) else {
        return Err(RtError::Xdto(format!(
            "цепочка типов вокруг «{}» замкнута сама на себя",
            data.name
        )));
    };
    let Some(shape) = data.shape.as_ref() else {
        return Err(RtError::Xdto(format!(
            "тип объекта «{}» не строится из лексической формы",
            data.name
        )));
    };
    match shape {
        ValueShape::Builtin(bsl) => builtin_from_lexical(*bsl, lexical),
        ValueShape::Atomic => {
            let base = data.base.ok_or_else(|| {
                RtError::Xdto(format!("у типа значения «{}» нет базового типа", data.name))
            })?;
            value_from_lexical_at(model, base, lexical, depth)
        }
        // Список: лексическая форма делится пробельными символами.
        // Платформа отдаёт `ФиксированныйМассив` (измерено), а здесь это
        // обычный `Массив` — своего неизменяемого вида в этой реализации
        // нет, и `vstr.rs` читает фиксированный массив тем же обычным
        // (терять данные хуже, чем терять неизменяемость).
        ValueShape::List(item) => {
            let item = item.ok_or_else(|| {
                RtError::Xdto(format!(
                    "у списочного типа «{}» нет типа элемента",
                    data.name
                ))
            })?;
            let mut items = Vec::new();
            for part in lexical.split_whitespace() {
                items.push(value_from_lexical_at(model, item, part, depth)?);
            }
            Ok(BslValue::new_array(items))
        }
        // Объединение: первый член, который принял форму (измерено на
        // `union memberTypes="xs:int xs:string"`).
        ValueShape::Union(members) => {
            for member in members {
                if let Ok(v) = value_from_lexical_at(model, *member, lexical, depth) {
                    return Ok(v);
                }
            }
            Err(RtError::Xdto(format!(
                "лексическую форму «{lexical}» не принял ни один член объединения «{}»",
                data.name
            )))
        }
    }
}

fn bad_lexical(lexical: &str, what: &str) -> RtError {
    RtError::Xdto(format!("«{lexical}» — не лексическая форма {what}"))
}

/// Значение встроенного типа из лексической формы. Правила измерены
/// поимённо: у `xs:boolean` принимаются и слова, и цифры; у чисел —
/// ведущий плюс, хвостовые нули и показатель степени (`1.5E3` -> 1500);
/// пробелы по краям отбрасываются у всех.
fn builtin_from_lexical(bsl: BuiltinBsl, lexical: &str) -> RtResult<BslValue> {
    match bsl {
        // Строка идёт как есть, БЕЗ обрезки: `xs:string` с одними
        // пробелами — это пробелы (фасет `whiteSpace` их не трогает, он
        // только описан).
        BuiltinBsl::Str => Ok(BslValue::Str(BslString::from_str(lexical))),
        BuiltinBsl::Number => match bsl_number::BslNumber::parse_canonical(lexical.trim()) {
            Ok(n) => Ok(BslValue::Number(n)),
            Err(_) => Err(bad_lexical(lexical, "числа")),
        },
        BuiltinBsl::Double => parse_exponential(lexical.trim()),
        BuiltinBsl::Boolean => match lexical.trim() {
            "true" | "1" => Ok(BslValue::Boolean(true)),
            "false" | "0" => Ok(BslValue::Boolean(false)),
            _ => Err(bad_lexical(lexical, "«xs:boolean»")),
        },
        BuiltinBsl::Date => parse_xsd_date(lexical.trim()),
        BuiltinBsl::DateTime => parse_xsd_date_time(lexical.trim()),
        BuiltinBsl::Time => parse_xsd_time(lexical.trim()),
        BuiltinBsl::Base64 => {
            let bytes =
                decode_base64(lexical).ok_or_else(|| bad_lexical(lexical, "«base64Binary»"))?;
            Ok(BslValue::Object(Rc::new(BslObject::BinaryData(Rc::from(
                bytes.into_boxed_slice(),
            )))))
        }
        BuiltinBsl::Hex => {
            let bytes =
                decode_hex(lexical.trim()).ok_or_else(|| bad_lexical(lexical, "«hexBinary»"))?;
            Ok(BslValue::Object(Rc::new(BslObject::BinaryData(Rc::from(
                bytes.into_boxed_slice(),
            )))))
        }
        // Префикс платформа не разрешает вовсе: `Создать` от `xs:string`
        // — ошибка, а `просто` даёт имя с ПУСТЫМ URI (измерено).
        BuiltinBsl::QName => {
            let text = lexical.trim();
            if text.contains(':') || text.is_empty() {
                return Err(bad_lexical(lexical, "«QName» без префикса"));
            }
            Ok(crate::xsd::new_expanded_name("", text))
        }
    }
}

/// Лексическая форма `xs:double`/`xs:float`: то же десятичное число, но с
/// необязательным показателем степени. Показатель поддерживают ровно эти
/// два типа — `Создать` от `xs:decimal` с «1.5E3» и от `xs:int` с «1E2»
/// платформа отвергает (измерено), поэтому у остальных числовых типов
/// разбор обычный.
///
/// `INF`, `-INF` и `NaN` платформа принимает (измерено на `INF`), а здесь
/// они отвергаются: `Число` в 1С — десятичное с конечной точностью, и
/// бесконечности в нём нет.
fn parse_exponential(text: &str) -> RtResult<BslValue> {
    let (mantissa, exponent) = match text.split_once(['E', 'e']) {
        Some((m, e)) => {
            let e: i64 = e
                .strip_prefix('+')
                .unwrap_or(e)
                .parse()
                .map_err(|_| bad_lexical(text, "числа с показателем степени"))?;
            (m, e)
        }
        None => (text, 0),
    };
    let mantissa =
        bsl_number::BslNumber::parse_canonical(mantissa).map_err(|_| bad_lexical(text, "числа"))?;
    if exponent == 0 {
        return Ok(BslValue::Number(mantissa));
    }
    // Десятичный сдвиг — это умножение или деление на степень десяти;
    // умножение точное, а деление идёт через ту же операцию, что и
    // обычное `/`, то есть с округлением до 27 знаков.
    let magnitude = u32::try_from(exponent.unsigned_abs())
        .map_err(|_| bad_lexical(text, "числа с показателем степени"))?;
    let ten = bsl_number::BslNumber::from_i64(10);
    let mut power = bsl_number::BslNumber::from_i64(1);
    for _ in 0..magnitude {
        power = power
            .mul(&ten)
            .map_err(|_| bad_lexical(text, "числа с показателем степени"))?;
    }
    let scaled = if exponent > 0 {
        mantissa.mul(&power)
    } else {
        mantissa.div(&power)
    };
    scaled
        .map(BslValue::Number)
        .map_err(|_| bad_lexical(text, "числа с показателем степени"))
}

/// `xs:date`: `ГГГГ-ММ-ДД` с необязательным поясом. Пояс не отбрасывается,
/// а пересчитывается в местное время, поэтому `2026-08-12+02:00` на машине
/// с поясом +03:00 дало 12.08.2026 1:00:00 (измерено).
fn parse_xsd_date(text: &str) -> RtResult<BslValue> {
    let (body, tail) = split_zone(text, 10);
    let mut parts = body.split('-');
    let year: i64 = parse_part(parts.next(), text, "даты")?;
    let month: u32 = parse_part(parts.next(), text, "даты")?;
    let day: u32 = parse_part(parts.next(), text, "даты")?;
    if parts.next().is_some() {
        return Err(bad_lexical(text, "даты"));
    }
    let wall = crate::BslDate::from_civil(year, month, day, 0, 0, 0)
        .ok_or_else(|| bad_lexical(text, "даты"))?;
    Ok(BslValue::Date(apply_zone(wall, tail)?))
}

/// `xs:dateTime`: `ГГГГ-ММ-ДДTЧЧ:ММ:СС` с необязательным поясом.
fn parse_xsd_date_time(text: &str) -> RtResult<BslValue> {
    let t = text
        .find('T')
        .ok_or_else(|| bad_lexical(text, "«dateTime»"))?;
    let (body, tail) = split_zone(text, t + 1);
    let (date_part, time_part) = body.split_at(t);
    let mut dp = date_part.split('-');
    let year: i64 = parse_part(dp.next(), text, "«dateTime»")?;
    let month: u32 = parse_part(dp.next(), text, "«dateTime»")?;
    let day: u32 = parse_part(dp.next(), text, "«dateTime»")?;
    let (hour, minute, second) = parse_clock(&time_part[1..], text)?;
    let wall = crate::BslDate::from_civil(year, month, day, hour, minute, second)
        .ok_or_else(|| bad_lexical(text, "«dateTime»"))?;
    Ok(BslValue::Date(apply_zone(wall, tail)?))
}

/// `xs:time`: `ЧЧ:ММ:СС`. Дата у результата — 01.01.0001 (измерено).
fn parse_xsd_time(text: &str) -> RtResult<BslValue> {
    let (body, tail) = split_zone(text, 0);
    let (hour, minute, second) = parse_clock(body, text)?;
    let wall = crate::BslDate::from_civil(1, 1, 1, hour, minute, second)
        .ok_or_else(|| bad_lexical(text, "времени"))?;
    Ok(BslValue::Date(apply_zone(wall, tail)?))
}

fn parse_part<T: std::str::FromStr>(part: Option<&str>, text: &str, what: &str) -> RtResult<T> {
    part.and_then(|p| p.parse().ok())
        .ok_or_else(|| bad_lexical(text, what))
}

fn parse_clock(text: &str, whole: &str) -> RtResult<(u32, u32, u32)> {
    let mut parts = text.split(':');
    let hour: u32 = parse_part(parts.next(), whole, "времени")?;
    let minute: u32 = parse_part(parts.next(), whole, "времени")?;
    // Доли секунды платформа принимает, но `Дата` их не хранит.
    let seconds = parts.next().unwrap_or("0");
    let second: u32 = parse_part(seconds.split('.').next(), whole, "времени")?;
    if parts.next().is_some() {
        return Err(bad_lexical(whole, "времени"));
    }
    Ok((hour, minute, second))
}

/// Хвост часового пояса, если он есть: `Z` либо `±ЧЧ:ММ`. Знак ищется
/// начиная с `from`, чтобы дефисы самой даты не попали в пояс.
///
/// `from` — БАЙТОВОЕ смещение, посчитанное по ожидаемой длине формы
/// (`parse_xsd_date` передаёт 10 — длину `ГГГГ-ММ-ДД`), а лексическая форма
/// приходит из схемы и может быть какой угодно: у `2026-08-1я` смещение 10
/// попадает ВНУТРЬ многобайтового символа. Поэтому срез берётся через
/// `get`: не граница символа — значит пояса тут нет, форма возвращается
/// целиком, и ошибку выдаёт вызывающий разбор (`bad_lexical`), а не паника
/// на пользовательских данных.
fn split_zone(text: &str, from: usize) -> (&str, Option<i32>) {
    if let Some(body) = text.strip_suffix('Z') {
        return (body, Some(0));
    }
    if let Some(tail) = text.get(from..) {
        if let Some(rel) = tail.find(['+', '-']) {
            let at = from + rel;
            let sign = if text.as_bytes()[at] == b'-' { -1 } else { 1 };
            if let Some((h, m)) = text[at + 1..].split_once(':') {
                if let (Ok(h), Ok(m)) = (h.parse::<i32>(), m.parse::<i32>()) {
                    return (&text[..at], Some(sign * (h * 3600 + m * 60)));
                }
            }
        }
    }
    (text, None)
}

/// Пояс пересчитывается в МЕСТНОЕ время машины, как это делает платформа
/// (измерено: `2026-08-12T18:41:17Z` дало 21:41:17 на машине с +03:00, а
/// `…+02:00` — 19:41:17). Без пояса запись остаётся как есть.
fn apply_zone(wall: crate::BslDate, zone: Option<i32>) -> RtResult<crate::BslDate> {
    match zone {
        None => Ok(wall),
        Some(offset) => crate::json::local_date_from_utc_seconds(
            crate::json::pseudo_unix_seconds(wall) - i64::from(offset),
            "лексическая форма XDTO",
        ),
    }
}

/// Разбор `hexBinary`: пары шестнадцатеричных цифр, регистр не важен.
fn decode_hex(text: &str) -> Option<Vec<u8>> {
    if !text.len().is_multiple_of(2) {
        return None;
    }
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len() / 2);
    for pair in bytes.chunks(2) {
        let hi = (pair[0] as char).to_digit(16)?;
        let lo = (pair[1] as char).to_digit(16)?;
        out.push((hi * 16 + lo) as u8);
    }
    Some(out)
}

/// Разбор `base64Binary`. Пробельные символы внутри записи игнорируются —
/// так требует XML Schema и так ведёт себя платформа с многострочным
/// содержимым элемента.
fn decode_base64(text: &str) -> Option<Vec<u8>> {
    let mut quad = [0u8; 4];
    let mut filled = 0usize;
    let mut padding = 0usize;
    let mut out = Vec::new();
    for ch in text.chars() {
        if ch.is_whitespace() {
            continue;
        }
        let value = match ch {
            'A'..='Z' => ch as u8 - b'A',
            'a'..='z' => ch as u8 - b'a' + 26,
            '0'..='9' => ch as u8 - b'0' + 52,
            '+' => 62,
            '/' => 63,
            '=' => {
                padding += 1;
                0
            }
            _ => return None,
        };
        // Значащий символ после заполнителя — испорченная запись.
        if padding > 0 && ch != '=' {
            return None;
        }
        quad[filled] = value;
        filled += 1;
        if filled == 4 {
            let triple = (u32::from(quad[0]) << 18)
                | (u32::from(quad[1]) << 12)
                | (u32::from(quad[2]) << 6)
                | u32::from(quad[3]);
            out.push((triple >> 16) as u8);
            if padding < 2 {
                out.push((triple >> 8) as u8);
            }
            if padding < 1 {
                out.push(triple as u8);
            }
            filled = 0;
        }
    }
    if filled != 0 || padding > 2 {
        return None;
    }
    Some(out)
}

// --- значения BSL --------------------------------------------------------

fn str_value(s: &str) -> BslValue {
    BslValue::Str(BslString::from_str(s))
}

fn number_value(n: i64) -> BslValue {
    BslValue::Number(bsl_number::BslNumber::from_i64(n))
}

/// `ТипЗначенияXDTO`/`ТипОбъектаXDTO` по номеру в модели.
pub fn type_value(model: &Rc<XdtoModel>, index: usize) -> BslValue {
    BslValue::Object(Rc::new(BslObject::XdtoType(model.clone(), index)))
}

fn property_value(model: &Rc<XdtoModel>, index: usize) -> BslValue {
    BslValue::Object(Rc::new(BslObject::XdtoProperty(model.clone(), index)))
}

/// `ЗначениеXDTO` из готовой пары «значение, лексическая форма».
fn data_value(data: &Rc<XdtoValueData>) -> BslValue {
    BslValue::Object(Rc::new(BslObject::XdtoValue(data.clone())))
}

/// Границы наружу: `unbounded` — это `-1` (измерено).
fn bound_value(bound: Option<u32>) -> BslValue {
    match bound {
        Some(n) => number_value(i64::from(n)),
        None => number_value(-1),
    }
}

/// Как печатает `Строка()` от типа: `{URI}Имя`, а у безымянного
/// (анонимного) типа — пустая строка (измерено).
fn type_display(data: &XdtoTypeData) -> String {
    if data.name.is_empty() {
        return String::new();
    }
    XName {
        uri: data.ns.clone(),
        local: data.name.clone(),
    }
    .display_text()
}

/// Строковое представление значения модели типов.
pub fn display_text(obj: &BslObject) -> Option<String> {
    Some(match obj {
        BslObject::XdtoType(model, i) => match model.types.get(*i) {
            Some(data) => type_display(data),
            None => String::new(),
        },
        // Свойство печатается ИМЕНЕМ (измерено: `Строка(Свв)` -> `name`).
        BslObject::XdtoProperty(model, i) => match model.properties.get(*i) {
            Some(data) => data.name.clone(),
            None => String::new(),
        },
        BslObject::XdtoProperties(..)
        | BslObject::XdtoFacets(..)
        | BslObject::XdtoFacet(..)
        | BslObject::XdtoValue(_) => type_name_of(obj)?.to_string(),
        _ => return None,
    })
}

/// Имя типа значения — то, чем зовут тип в коде.
pub fn type_name_of(obj: &BslObject) -> Option<&'static str> {
    Some(match obj {
        BslObject::XdtoType(model, i) => match model.types.get(*i) {
            Some(data) if data.is_value() => "ТипЗначенияXDTO",
            Some(_) => "ТипОбъектаXDTO",
            None => return None,
        },
        BslObject::XdtoProperty(..) => "СвойствоXDTO",
        BslObject::XdtoProperties(..) => "КоллекцияСвойствXDTO",
        BslObject::XdtoFacets(..) => "КоллекцияФасетовXDTO",
        BslObject::XdtoFacet(..) => "ФасетXDTO",
        BslObject::XdtoValue(_) => "ЗначениеXDTO",
        _ => return None,
    })
}

/// `ТипЗнч()` значения модели типов.
pub fn type_id_of(obj: &BslObject) -> Option<TypeId> {
    Some(match obj {
        BslObject::XdtoType(model, i) => match model.types.get(*i) {
            Some(data) if data.is_value() => TypeId::XdtoValueType,
            Some(_) => TypeId::XdtoObjectType,
            None => return None,
        },
        BslObject::XdtoProperty(..) => TypeId::XdtoProperty,
        BslObject::XdtoProperties(..) => TypeId::XdtoPropertyCollection,
        BslObject::XdtoFacets(..) => TypeId::XdtoFacetCollection,
        BslObject::XdtoFacet(..) => TypeId::XdtoFacet,
        BslObject::XdtoValue(_) => TypeId::XdtoDataValue,
        _ => return None,
    })
}

/// Член `ВидФасетаXDTO` по виду фасета лексической модели XSD.
fn facet_kind_value(kind: FacetKind) -> EnumValue {
    match kind {
        FacetKind::Length => EnumValue::XdtoFacetLength,
        FacetKind::MinLength => EnumValue::XdtoFacetMinLength,
        FacetKind::MaxLength => EnumValue::XdtoFacetMaxLength,
        FacetKind::Pattern => EnumValue::XdtoFacetPattern,
        FacetKind::Enumeration => EnumValue::XdtoFacetEnumeration,
        FacetKind::WhiteSpace => EnumValue::XdtoFacetWhiteSpace,
        FacetKind::TotalDigits => EnumValue::XdtoFacetTotalDigits,
        FacetKind::FractionDigits => EnumValue::XdtoFacetFractionDigits,
        FacetKind::MinInclusive => EnumValue::XdtoFacetMinInclusive,
        FacetKind::MaxInclusive => EnumValue::XdtoFacetMaxInclusive,
        FacetKind::MinExclusive => EnumValue::XdtoFacetMinExclusive,
        FacetKind::MaxExclusive => EnumValue::XdtoFacetMaxExclusive,
    }
}

/// Свойство значения модели типов.
///
/// # Errors
///
/// [`RtError::UnknownColumn`], если такого члена у этого вида значения
/// нет; [`RtError::Xdto`], если модель ссылается на несуществующий узел.
pub fn get_property(obj: &BslValue, name: &str) -> RtResult<BslValue> {
    let unknown = || RtError::UnknownColumn(name.to_string());
    let is = |ru: &str, en: &str| name.eq_ignore_ascii_case(ru) || name.eq_ignore_ascii_case(en);
    let BslValue::Object(o) = obj else {
        return Err(unknown());
    };
    match &**o {
        BslObject::XdtoType(model, i) => {
            let data = model.type_at(*i)?;
            if is("Имя", "Name") {
                return Ok(str_value(&data.name));
            }
            if is("URIПространстваИмен", "NamespaceURI") {
                return Ok(str_value(&data.ns));
            }
            if is("БазовыйТип", "BaseType") {
                return Ok(match data.base {
                    Some(b) => type_value(model, b),
                    None => BslValue::Undefined,
                });
            }
            if data.is_value() {
                if is("Фасеты", "Facets") {
                    // У типа БЕЗ фасетов это `Неопределено`, а не пустая
                    // коллекция (измерено на `xs:date`).
                    return Ok(if data.facets.is_empty() {
                        BslValue::Undefined
                    } else {
                        BslValue::Object(Rc::new(BslObject::XdtoFacets(model.clone(), *i)))
                    });
                }
                return Err(unknown());
            }
            if is("Свойства", "Properties") {
                return Ok(BslValue::Object(Rc::new(BslObject::XdtoProperties(
                    model.clone(),
                    *i,
                ))));
            }
            if is("Открытый", "Open") {
                return Ok(BslValue::Boolean(data.open));
            }
            if is("Абстрактный", "Abstract") {
                return Ok(BslValue::Boolean(data.is_abstract));
            }
            if is("Упорядоченный", "Ordered") {
                return Ok(BslValue::Boolean(data.ordered));
            }
            if is("Последовательный", "Sequenced") {
                return Ok(BslValue::Boolean(data.sequenced()));
            }
            if is("Смешанный", "Mixed") {
                return Ok(BslValue::Boolean(data.mixed));
            }
            Err(unknown())
        }
        BslObject::XdtoProperty(model, i) => {
            let data = model.property_at(*i)?;
            if is("Имя", "Name") {
                return Ok(str_value(&data.name));
            }
            if is("URIПространстваИмен", "NamespaceURI") {
                return Ok(str_value(&data.ns));
            }
            if is("Тип", "Type") {
                return Ok(type_value(model, data.type_index));
            }
            if is("НижняяГраница", "LowerBound") {
                return Ok(bound_value(data.lower));
            }
            if is("ВерхняяГраница", "UpperBound") {
                return Ok(bound_value(data.upper));
            }
            if is("Форма", "Form") {
                return Ok(BslValue::Enum(data.form));
            }
            if is("ЗначениеПоУмолчанию", "DefaultValue") {
                return Ok(match &data.default {
                    Some(v) => data_value(v),
                    None => BslValue::Undefined,
                });
            }
            Err(unknown())
        }
        BslObject::XdtoFacet(model, type_index, facet_index) => {
            let data = model.type_at(*type_index)?;
            let (kind, lexical) = data
                .facets
                .get(*facet_index)
                .ok_or_else(|| broken("фасет"))?;
            // Английское имя `Вид` — `Type`, а не `Kind`: `Kind` платформа
            // отвергает (измерено обе пробы).
            if is("Вид", "Type") {
                return Ok(BslValue::Enum(facet_kind_value(*kind)));
            }
            // `Значение` фасета — ВСЕГДА строка, даже у числовых
            // (измерено).
            if is("Значение", "Value") {
                return Ok(str_value(lexical));
            }
            Err(unknown())
        }
        BslObject::XdtoValue(data) => {
            if is("Значение", "Value") {
                return Ok(data.value.clone());
            }
            if is("ЛексическоеЗначение", "LexicalValue") {
                return Ok(str_value(&data.lexical));
            }
            Err(unknown())
        }
        _ => Err(unknown()),
    }
}

/// Длина коллекции свойств или фасетов.
///
/// # Errors
///
/// [`RtError::Xdto`], если модель ссылается на несуществующий тип.
pub fn collection_len(obj: &BslObject) -> Option<RtResult<usize>> {
    match obj {
        BslObject::XdtoProperties(model, i) => {
            Some(model.type_at(*i).map(|data| data.properties.len()))
        }
        BslObject::XdtoFacets(model, i) => Some(model.type_at(*i).map(|data| data.facets.len())),
        _ => None,
    }
}

/// Элемент коллекции по номеру.
///
/// # Errors
///
/// [`RtError::IndexOutOfBounds`], если номер за границей.
pub fn collection_get(obj: &BslObject, i: usize) -> RtResult<BslValue> {
    match obj {
        BslObject::XdtoProperties(model, t) => {
            let data = model.type_at(*t)?;
            match data.properties.get(i) {
                Some(p) => Ok(property_value(model, *p)),
                None => Err(RtError::IndexOutOfBounds {
                    index: i as i64,
                    len: data.properties.len(),
                }),
            }
        }
        BslObject::XdtoFacets(model, t) => {
            let data = model.type_at(*t)?;
            if i < data.facets.len() {
                Ok(BslValue::Object(Rc::new(BslObject::XdtoFacet(
                    model.clone(),
                    *t,
                    i,
                ))))
            } else {
                Err(RtError::IndexOutOfBounds {
                    index: i as i64,
                    len: data.facets.len(),
                })
            }
        }
        _ => Err(RtError::NotIndexable),
    }
}

/// `Получить` у коллекции свойств (имя или номер) и у коллекции фасетов
/// (только номер — поиск по имени платформа отвергает, измерено).
///
/// # Errors
///
/// [`RtError::MethodNotApplicable`], если получатель не коллекция модели
/// типов или аргумент не тот; [`RtError::IndexOutOfBounds`] на номере за
/// границей.
pub fn collection_lookup(obj: &BslValue, args: &[BslValue]) -> RtResult<BslValue> {
    let not_applicable = || RtError::MethodNotApplicable {
        method: "Получить",
        receiver: obj.type_name(),
    };
    let BslValue::Object(o) = obj else {
        return Err(not_applicable());
    };
    let [arg] = args else {
        return Err(not_applicable());
    };
    match (&**o, arg) {
        (BslObject::XdtoProperties(model, t), BslValue::Str(s)) => {
            let data = model.type_at(*t)?;
            let name = s.to_string();
            // Неизвестное имя — `Неопределено`, а не ошибка (измерено).
            Ok(data
                .properties
                .iter()
                .find(|p| {
                    model
                        .properties
                        .get(**p)
                        .is_some_and(|prop| prop.name == name)
                })
                .map_or(BslValue::Undefined, |p| property_value(model, *p)))
        }
        (BslObject::XdtoProperties(..) | BslObject::XdtoFacets(..), BslValue::Number(n)) => {
            let index = n.to_i64_exact().ok_or_else(not_applicable)?;
            let len = match collection_len(o) {
                Some(len) => len?,
                None => return Err(not_applicable()),
            };
            let index =
                usize::try_from(index).map_err(|_| RtError::IndexOutOfBounds { index, len })?;
            collection_get(o, index)
        }
        _ => Err(not_applicable()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Модель типов из текста XSD — тем же путём, что и в бою: дерево
    /// строит `dom`, схему — `xsd`, а типы — этот модуль.
    fn model(text: &str) -> Rc<XdtoModel> {
        let schema = crate::xsd::build_for_tests(text).expect("схема обязана разбираться");
        model_of_schema(&schema).expect("модель обязана строиться")
    }

    /// Схема из `measure-xdto.bsl`, сокращённая до того, что проверяют
    /// тесты ниже. Имена и порядок объявлений — те же, поэтому измеренные
    /// строки платформы читаются рядом с ожиданиями.
    const SAMPLE: &str = concat!(
        r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" xmlns:t="urn:test" "#,
        r#"targetNamespace="urn:test" elementFormDefault="qualified" "#,
        r#"attributeFormDefault="unqualified">"#,
        r#"<xs:simpleType name="Code"><xs:restriction base="xs:string">"#,
        r#"<xs:minLength value="2"/><xs:maxLength value="5"/>"#,
        r#"<xs:pattern value="[A-Z]+"/></xs:restriction></xs:simpleType>"#,
        r#"<xs:simpleType name="Codes"><xs:list itemType="t:Code"/></xs:simpleType>"#,
        r#"<xs:simpleType name="Either2"><xs:union memberTypes="xs:int xs:string"/></xs:simpleType>"#,
        r#"<xs:complexType name="RootType"><xs:sequence>"#,
        r#"<xs:element name="name" type="xs:string"/>"#,
        r#"<xs:element name="code" type="t:Code" minOccurs="0" maxOccurs="unbounded"/>"#,
        r#"<xs:element name="def" type="xs:int" default="7" minOccurs="0"/>"#,
        r#"<xs:element name="many5" type="xs:string" maxOccurs="5"/>"#,
        r#"<xs:element name="notype" minOccurs="0"/>"#,
        r#"<xs:element name="uq" type="xs:string" form="unqualified"/>"#,
        r#"<xs:element name="anon"><xs:complexType><xs:sequence>"#,
        r#"<xs:element name="inner" type="xs:decimal"/>"#,
        r#"</xs:sequence></xs:complexType></xs:element>"#,
        r#"</xs:sequence>"#,
        r#"<xs:attribute name="id" type="xs:int" use="required"/>"#,
        r#"<xs:attribute name="opt" type="xs:string"/>"#,
        r#"<xs:attribute name="q" type="xs:string" form="qualified"/>"#,
        r#"<xs:attribute name="fx" type="xs:int" fixed="9"/>"#,
        r#"</xs:complexType>"#,
        r#"<xs:complexType name="ExtType"><xs:complexContent>"#,
        r#"<xs:extension base="t:RootType">"#,
        r#"<xs:sequence><xs:element name="extra" type="xs:boolean"/></xs:sequence>"#,
        r#"<xs:attribute name="ea" type="xs:string"/>"#,
        r#"</xs:extension></xs:complexContent></xs:complexType>"#,
        r#"<xs:complexType name="ChoiceType">"#,
        r#"<xs:choice minOccurs="0"><xs:element name="ca" type="xs:string"/>"#,
        r#"<xs:element name="cb" type="xs:string" maxOccurs="3"/></xs:choice>"#,
        r#"<xs:attribute name="cat" type="xs:string"/></xs:complexType>"#,
        r#"<xs:complexType name="AllType"><xs:all>"#,
        r#"<xs:element name="ap" type="xs:string"/>"#,
        r#"<xs:element name="aq" type="xs:string" minOccurs="0"/></xs:all></xs:complexType>"#,
        r#"<xs:complexType name="NestType"><xs:sequence>"#,
        r#"<xs:element name="nx" type="xs:string"/>"#,
        r#"<xs:sequence maxOccurs="unbounded"><xs:element name="ny" type="xs:string"/>"#,
        r#"<xs:element name="nz" type="xs:string" minOccurs="0"/></xs:sequence>"#,
        r#"<xs:element name="nw" type="xs:string"/>"#,
        r#"</xs:sequence></xs:complexType>"#,
        r#"<xs:complexType name="SimpContent"><xs:simpleContent>"#,
        r#"<xs:extension base="xs:string"><xs:attribute name="su" type="xs:string"/>"#,
        r#"</xs:extension></xs:simpleContent></xs:complexType>"#,
        r#"<xs:complexType name="AbstrType" abstract="true" mixed="true"><xs:sequence>"#,
        r#"<xs:element name="x" type="xs:string"/></xs:sequence></xs:complexType>"#,
        r#"<xs:complexType name="EmptyType"/>"#,
        r#"</xs:schema>"#,
    );

    fn type_of(model: &Rc<XdtoModel>, uri: &str, name: &str) -> BslValue {
        let index = model
            .find(uri, name)
            .unwrap_or_else(|| panic!("в модели нет типа {{{uri}}}{name}"));
        type_value(model, index)
    }

    fn prop(obj: &BslValue, name: &str) -> BslValue {
        get_property(obj, name).unwrap_or_else(|e| panic!("член «{name}»: {e}"))
    }

    fn text_of(v: &BslValue) -> String {
        match v {
            BslValue::Str(s) => s.to_string(),
            other => panic!("ожидалась строка, получено {other:?}"),
        }
    }

    fn number_of(v: &BslValue) -> i64 {
        match v {
            BslValue::Number(n) => n.to_i64_exact().expect("целое"),
            other => panic!("ожидалось число, получено {other:?}"),
        }
    }

    /// Число целиком, включая дробную часть, — как каноническая строка.
    fn decimal_of(v: &BslValue) -> String {
        match v {
            BslValue::Number(n) => n.to_canonical(),
            other => panic!("ожидалось число, получено {other:?}"),
        }
    }

    /// Имена свойств типа объекта по порядку — то, что печатает проба
    /// `об Свойства порядок`.
    fn property_names(model: &Rc<XdtoModel>, uri: &str, name: &str) -> Vec<String> {
        let t = type_of(model, uri, name);
        let props = prop(&t, "Свойства");
        let len = match &props {
            BslValue::Object(o) => collection_len(o).expect("коллекция").expect("длина"),
            other => panic!("ожидалась коллекция, получено {other:?}"),
        };
        (0..len)
            .map(|i| match &props {
                BslValue::Object(o) => text_of(&prop(
                    &collection_get(o, i).expect("элемент коллекции"),
                    "Имя",
                )),
                other => panic!("ожидалась коллекция, получено {other:?}"),
            })
            .collect()
    }

    /// Порядок свойств: унаследованные, потом свои атрибуты, потом свои
    /// элементы. Обе строки — из `measure-xdto.platform.txt`.
    #[test]
    fn properties_are_flattened_attributes_first_then_elements() {
        let m = model(SAMPLE);
        assert_eq!(
            property_names(&m, "urn:test", "RootType"),
            vec!["id", "opt", "q", "fx", "name", "code", "def", "many5", "notype", "uq", "anon"]
        );
        // Наследник: сначала весь базовый тип, потом СВОЙ атрибут, потом
        // свой элемент.
        let ext = property_names(&m, "urn:test", "ExtType");
        assert_eq!(ext.len(), 13);
        assert_eq!(
            &ext[ext.len() - 2..],
            &["ea".to_string(), "extra".to_string()]
        );
        assert_eq!(&ext[..4], &["id", "opt", "q", "fx"].map(str::to_string));
        assert!(property_names(&m, "urn:test", "EmptyType").is_empty());
    }

    /// Границы вхождения перемножаются по вложенным группам, а
    /// `unbounded` показывается как -1.
    #[test]
    fn occurrence_bounds_multiply_through_model_groups() {
        let m = model(SAMPLE);
        let bounds = |type_name: &str, prop_name: &str| {
            let t = type_of(&m, "urn:test", type_name);
            let props = prop(&t, "Свойства");
            let p = collection_lookup(&props, &[str_value(prop_name)]).expect("поиск свойства");
            (
                number_of(&prop(&p, "НижняяГраница")),
                number_of(&prop(&p, "ВерхняяГраница")),
            )
        };
        assert_eq!(bounds("RootType", "name"), (1, 1));
        assert_eq!(bounds("RootType", "code"), (0, -1), "maxOccurs=unbounded");
        assert_eq!(bounds("RootType", "many5"), (1, 5));
        assert_eq!(bounds("RootType", "id"), (1, 1), "use=required");
        assert_eq!(bounds("RootType", "opt"), (0, 1), "атрибут без use");
        // `<xs:choice minOccurs="0">` обнуляет нижние границы вложенного.
        assert_eq!(bounds("ChoiceType", "ca"), (0, 1));
        assert_eq!(bounds("ChoiceType", "cb"), (0, 3));
        assert_eq!(bounds("ChoiceType", "cat"), (0, 1));
        // Вложенная последовательность с `maxOccurs="unbounded"`.
        assert_eq!(bounds("NestType", "nx"), (1, 1));
        assert_eq!(bounds("NestType", "ny"), (1, -1));
        assert_eq!(bounds("NestType", "nz"), (0, -1));
        assert_eq!(bounds("NestType", "nw"), (1, 1));
        assert_eq!(bounds("AllType", "ap"), (1, 1));
        assert_eq!(bounds("AllType", "aq"), (0, 1));
    }

    /// Форма и пространство имён свойства — по правилу форм схемы.
    #[test]
    fn property_form_and_namespace_follow_the_schema_forms() {
        let m = model(SAMPLE);
        let root = type_of(&m, "urn:test", "RootType");
        let props = prop(&root, "Свойства");
        let by_name = |n: &str| collection_lookup(&props, &[str_value(n)]).expect("поиск");
        assert_eq!(
            prop(&by_name("name"), "Форма"),
            BslValue::Enum(EnumValue::XmlFormElement)
        );
        assert_eq!(
            prop(&by_name("id"), "Форма"),
            BslValue::Enum(EnumValue::XmlFormAttribute)
        );
        assert_eq!(
            text_of(&prop(&by_name("name"), "URIПространстваИмен")),
            "urn:test"
        );
        assert_eq!(
            text_of(&prop(&by_name("uq"), "URIПространстваИмен")),
            "",
            "form=unqualified"
        );
        assert_eq!(text_of(&prop(&by_name("id"), "URIПространстваИмен")), "");
        assert_eq!(
            text_of(&prop(&by_name("q"), "URIПространстваИмен")),
            "urn:test",
            "form=qualified"
        );
        // Неизвестное имя — `Неопределено`, а не ошибка.
        assert_eq!(
            collection_lookup(&props, &[str_value("нетТакого")]).expect("поиск"),
            BslValue::Undefined
        );
    }

    /// Тип свойства: объявленный, анонимный и — при отсутствии обоих —
    /// `anyType`.
    #[test]
    fn property_type_falls_back_to_any_type() {
        let m = model(SAMPLE);
        let root = type_of(&m, "urn:test", "RootType");
        let props = prop(&root, "Свойства");
        let by_name = |n: &str| collection_lookup(&props, &[str_value(n)]).expect("поиск");
        let name_type = prop(&by_name("name"), "Тип");
        assert_eq!(text_of(&prop(&name_type, "Имя")), "string");
        assert_eq!(text_of(&prop(&name_type, "URIПространстваИмен")), XSD_NS);
        let no_type = prop(&by_name("notype"), "Тип");
        assert_eq!(text_of(&prop(&no_type, "Имя")), "anyType");
        // Анонимный тип: имя пусто, а пространство имён — целевое.
        let anon = prop(&by_name("anon"), "Тип");
        assert_eq!(text_of(&prop(&anon, "Имя")), "");
        assert_eq!(text_of(&prop(&anon, "URIПространстваИмен")), "urn:test");
        let anon_props = prop(&anon, "Свойства");
        assert_eq!(
            match &anon_props {
                BslValue::Object(o) => collection_len(o).expect("коллекция").expect("длина"),
                other => panic!("ожидалась коллекция, получено {other:?}"),
            },
            1
        );
    }

    /// Флаги типа объекта: упорядоченность по виду группы, а
    /// «последовательный» — производный от неё и от смешанности.
    #[test]
    fn object_type_flags_follow_the_content_model() {
        let m = model(SAMPLE);
        let flags = |name: &str| {
            let t = type_of(&m, "urn:test", name);
            (
                prop(&t, "Упорядоченный"),
                prop(&t, "Последовательный"),
                prop(&t, "Смешанный"),
                prop(&t, "Открытый"),
            )
        };
        let yes = BslValue::Boolean(true);
        let no = BslValue::Boolean(false);
        assert_eq!(
            flags("RootType"),
            (yes.clone(), no.clone(), no.clone(), no.clone())
        );
        assert_eq!(
            flags("ChoiceType"),
            (no.clone(), yes.clone(), no.clone(), no.clone())
        );
        assert_eq!(
            flags("AllType"),
            (no.clone(), yes.clone(), no.clone(), no.clone())
        );
        assert_eq!(
            flags("AbstrType"),
            (yes.clone(), yes.clone(), yes.clone(), no.clone())
        );
        assert_eq!(
            flags("EmptyType"),
            (yes.clone(), no.clone(), no.clone(), no.clone())
        );
        assert_eq!(
            prop(&type_of(&m, "urn:test", "AbstrType"), "Абстрактный"),
            yes
        );
        assert_eq!(
            prop(&type_of(&m, "urn:test", "RootType"), "Абстрактный"),
            no
        );
        // Открыт ровно `anyType`.
        assert_eq!(prop(&type_of(&m, XSD_NS, "anyType"), "Открытый"), yes);
    }

    /// Базовый тип: у объекта без базового — `anyType`, у значения —
    /// `anySimpleType`, а у простого содержимого базовым остаётся
    /// `anyType`, простой же тип виден свойством `__content`.
    #[test]
    fn base_type_defaults_to_any_type_and_any_simple_type() {
        let m = model(SAMPLE);
        let base_of = |uri: &str, name: &str| {
            let t = type_of(&m, uri, name);
            text_of(&prop(&prop(&t, "БазовыйТип"), "Имя"))
        };
        assert_eq!(base_of("urn:test", "RootType"), "anyType");
        assert_eq!(base_of("urn:test", "ExtType"), "RootType");
        assert_eq!(base_of("urn:test", "Code"), "string");
        assert_eq!(base_of("urn:test", "Codes"), "anySimpleType");
        assert_eq!(base_of("urn:test", "SimpContent"), "anyType");
        assert_eq!(base_of(XSD_NS, "int"), "long");
        assert_eq!(base_of(XSD_NS, "string"), "anySimpleType");
        assert_eq!(
            prop(&type_of(&m, XSD_NS, "anyType"), "БазовыйТип"),
            BslValue::Undefined
        );
        // Простое содержимое: атрибут, затем текстовое свойство.
        assert_eq!(
            property_names(&m, "urn:test", "SimpContent"),
            vec!["su", "__content"]
        );
        let simp = type_of(&m, "urn:test", "SimpContent");
        let content =
            collection_lookup(&prop(&simp, "Свойства"), &[str_value("__content")]).expect("поиск");
        assert_eq!(
            prop(&content, "Форма"),
            BslValue::Enum(EnumValue::XmlFormText)
        );
        assert_eq!(text_of(&prop(&prop(&content, "Тип"), "Имя")), "string");
        assert_eq!(text_of(&prop(&content, "URIПространстваИмен")), "");
        // У СМЕШАННОГО типа текстового свойства нет.
        assert_eq!(property_names(&m, "urn:test", "AbstrType"), vec!["x"]);
    }

    /// Фасеты: вид и значение-строка, а у типа без фасетов —
    /// `Неопределено`.
    #[test]
    fn facets_report_their_kind_and_lexical_value() {
        let m = model(SAMPLE);
        let facets_of = |uri: &str, name: &str| {
            let t = type_of(&m, uri, name);
            let facets = prop(&t, "Фасеты");
            match &facets {
                BslValue::Undefined => Vec::new(),
                BslValue::Object(o) => {
                    let len = collection_len(o).expect("коллекция").expect("длина");
                    (0..len)
                        .map(|i| {
                            let f = collection_get(o, i).expect("фасет");
                            (prop(&f, "Вид"), text_of(&prop(&f, "Значение")))
                        })
                        .collect::<Vec<_>>()
                }
                other => panic!("ожидалась коллекция или Неопределено, получено {other:?}"),
            }
        };
        assert_eq!(
            facets_of("urn:test", "Code"),
            vec![
                (
                    BslValue::Enum(EnumValue::XdtoFacetMinLength),
                    "2".to_string()
                ),
                (
                    BslValue::Enum(EnumValue::XdtoFacetMaxLength),
                    "5".to_string()
                ),
                (
                    BslValue::Enum(EnumValue::XdtoFacetPattern),
                    "[A-Z]+".to_string()
                ),
            ]
        );
        // Встроенные типы несут измеренные фасеты.
        assert_eq!(
            facets_of(XSD_NS, "string"),
            vec![(
                BslValue::Enum(EnumValue::XdtoFacetWhiteSpace),
                "preserve".to_string()
            )]
        );
        assert_eq!(
            facets_of(XSD_NS, "int"),
            vec![
                (
                    BslValue::Enum(EnumValue::XdtoFacetMinInclusive),
                    "-2147483648".to_string()
                ),
                (
                    BslValue::Enum(EnumValue::XdtoFacetMaxInclusive),
                    "2147483647".to_string()
                ),
            ]
        );
        // `xs:date` фасетов не несёт, и `Фасеты` у него `Неопределено`.
        assert_eq!(
            prop(&type_of(&m, XSD_NS, "date"), "Фасеты"),
            BslValue::Undefined
        );
    }

    /// Значение по умолчанию — `ЗначениеXDTO` из `default` и из `fixed`.
    #[test]
    fn default_value_comes_from_both_default_and_fixed() {
        let m = model(SAMPLE);
        let root = type_of(&m, "urn:test", "RootType");
        let props = prop(&root, "Свойства");
        let by_name = |n: &str| collection_lookup(&props, &[str_value(n)]).expect("поиск");
        let def = prop(&by_name("def"), "ЗначениеПоУмолчанию");
        assert_eq!(number_of(&prop(&def, "Значение")), 7);
        assert_eq!(text_of(&prop(&def, "ЛексическоеЗначение")), "7");
        let fx = prop(&by_name("fx"), "ЗначениеПоУмолчанию");
        assert_eq!(number_of(&prop(&fx, "Значение")), 9);
        assert_eq!(text_of(&prop(&fx, "ЛексическоеЗначение")), "9");
        assert_eq!(
            prop(&by_name("name"), "ЗначениеПоУмолчанию"),
            BslValue::Undefined
        );
    }

    /// Таблица встроенных типов: каждая строка — из
    /// `measure-xdto.platform.txt` (`вст <имя>`).
    #[test]
    fn builtin_types_map_to_the_measured_bsl_types() {
        let m = model(SAMPLE);
        let value = |name: &str, lexical: &str| {
            let index = m.find(XSD_NS, name).expect("встроенный тип");
            value_from_lexical(&m, index, lexical).expect("лексическая форма")
        };
        let type_of_value = |name: &str, lexical: &str| value(name, lexical).type_of().unwrap();
        for name in [
            "string",
            "normalizedString",
            "token",
            "duration",
            "gYear",
            "anyURI",
            "NCName",
            "Name",
            "NMTOKEN",
            "language",
            "ID",
            "anySimpleType",
        ] {
            assert_eq!(
                type_of_value(name, "аб"),
                BslValue::Type(TypeId::String),
                "{name}"
            );
        }
        for name in [
            "decimal",
            "int",
            "integer",
            "long",
            "short",
            "byte",
            "unsignedInt",
            "unsignedLong",
            "unsignedShort",
            "unsignedByte",
            "nonNegativeInteger",
            "positiveInteger",
            "negativeInteger",
            "nonPositiveInteger",
            "double",
            "float",
        ] {
            assert_eq!(
                type_of_value(name, "-42"),
                BslValue::Type(TypeId::Number),
                "{name}"
            );
        }
        assert_eq!(
            type_of_value("boolean", "true"),
            BslValue::Type(TypeId::Boolean)
        );
        assert_eq!(
            type_of_value("date", "2026-08-12"),
            BslValue::Type(TypeId::Date)
        );
        assert_eq!(
            type_of_value("dateTime", "2026-08-12T18:41:17"),
            BslValue::Type(TypeId::Date)
        );
        assert_eq!(
            type_of_value("time", "18:41:17"),
            BslValue::Type(TypeId::Date)
        );
        assert_eq!(
            type_of_value("base64Binary", "0LDQsQ=="),
            BslValue::Type(TypeId::BinaryData)
        );
        assert_eq!(
            type_of_value("hexBinary", "D0B0D0B1"),
            BslValue::Type(TypeId::BinaryData)
        );
        assert_eq!(
            type_of_value("QName", "просто"),
            BslValue::Type(TypeId::XmlExpandedName)
        );
        // `anyType` — тип ОБЪЕКТА: значения из лексической формы он не
        // строит (измерено: `Создать` от него отвергает лексику).
        let any = m.find(XSD_NS, "anyType").expect("anyType");
        assert!(value_from_lexical(&m, any, "текст").is_err());
    }

    /// Разбор лексических форм — те же записи, что снимались на
    /// платформе.
    #[test]
    fn lexical_forms_follow_the_measured_conversions() {
        let m = model(SAMPLE);
        let value = |name: &str, lexical: &str| {
            let index = m.find(XSD_NS, name).expect("встроенный тип");
            value_from_lexical(&m, index, lexical).expect("лексическая форма")
        };
        assert_eq!(text_of(&value("string", "аб в")), "аб в");
        assert_eq!(decimal_of(&value("decimal", "-12.75")), "-12.75");
        assert_eq!(
            value("decimal", "+12.750"),
            value("decimal", "12.75"),
            "ведущий плюс и хвостовые нули"
        );
        assert_eq!(number_of(&value("double", "1.5E3")), 1500);
        assert_eq!(number_of(&value("int", " 42 ")), 42, "пробелы по краям");
        assert_eq!(value("boolean", "true"), BslValue::Boolean(true));
        assert_eq!(value("boolean", "1"), BslValue::Boolean(true));
        assert_eq!(value("boolean", "false"), BslValue::Boolean(false));
        assert_eq!(value("boolean", "0"), BslValue::Boolean(false));
        // Двоичные записи дают одни и те же байты — «аб» в UTF-8.
        let expected: &[u8] = &[0xD0, 0xB0, 0xD0, 0xB1];
        for (name, lexical) in [("base64Binary", "0LDQsQ=="), ("hexBinary", "D0B0D0B1")] {
            match value(name, lexical) {
                BslValue::Object(o) => match &*o {
                    BslObject::BinaryData(bytes) => assert_eq!(&**bytes, expected, "{name}"),
                    other => panic!("ожидались двоичные данные, получено {other:?}"),
                },
                other => panic!("ожидались двоичные данные, получено {other:?}"),
            }
        }
        // QName без префикса — расширенное имя с пустым URI, с префиксом
        // — ошибка (измерено обе стороны).
        let qname = m.find(XSD_NS, "QName").expect("QName");
        let name = value_from_lexical(&m, qname, "просто").expect("QName");
        // Расширенное имя — значение модели СХЕМЫ, и члены у него читает
        // она же.
        let expanded = |member: &str| {
            text_of(&crate::xsd::get_property(&name, member).expect("член расширенного имени"))
        };
        assert_eq!(expanded("ЛокальноеИмя"), "просто");
        assert_eq!(expanded("URIПространстваИмен"), "");
        assert!(value_from_lexical(&m, qname, "xs:string").is_err());
        // Непринимаемые записи — ошибка, а не подстановка.
        let int = m.find(XSD_NS, "int").expect("int");
        assert!(value_from_lexical(&m, int, "ерунда").is_err());
        let date = m.find(XSD_NS, "date").expect("date");
        assert!(value_from_lexical(&m, date, "ерунда").is_err());
        // Форма ДЛИННЕЕ `ГГГГ-ММ-ДД`, у которой многобайтовый символ
        // накрывает смещение 10: разбор ищет пояс именно с него, и срез по
        // сырому байтовому индексу здесь ронял процесс. Ожидается ошибка.
        assert!(value_from_lexical(&m, date, "2026-08-1я").is_err());
        assert!(value_from_lexical(&m, date, "2026-08-1я+03:00").is_err());
        let boolean = m.find(XSD_NS, "boolean").expect("boolean");
        assert!(value_from_lexical(&m, boolean, "да").is_err());
    }

    /// Свой тип наследует отображение базового, список даёт
    /// фиксированный массив, а объединение выбирает первый подошедший
    /// член.
    #[test]
    fn derived_list_and_union_types_map_as_measured() {
        let m = model(SAMPLE);
        let value = |uri: &str, name: &str, lexical: &str| {
            let index = m.find(uri, name).expect("тип");
            value_from_lexical(&m, index, lexical).expect("лексическая форма")
        };
        assert_eq!(text_of(&value("urn:test", "Code", "AB")), "AB");
        // `union memberTypes="xs:int xs:string"`: «5» — число, «аб» —
        // строка.
        assert_eq!(number_of(&value("urn:test", "Either2", "5")), 5);
        assert_eq!(text_of(&value("urn:test", "Either2", "аб")), "аб");
        // Платформа отдаёт здесь `ФиксированныйМассив`, здесь это обычный
        // массив: неизменяемого вида в этой реализации нет.
        match value("urn:test", "Codes", "AB CD") {
            BslValue::Object(o) => match &*o {
                BslObject::Array(items) => {
                    let items = items.borrow();
                    assert_eq!(items.len(), 2);
                    assert_eq!(text_of(&items[0]), "AB");
                    assert_eq!(text_of(&items[1]), "CD");
                }
                other => panic!("ожидался массив, получено {other:?}"),
            },
            other => panic!("ожидался массив, получено {other:?}"),
        }
    }

    /// Имена и представления значений модели — то, что печатают
    /// `Строка()` и `ТипЗнч()`.
    #[test]
    fn type_and_property_values_print_as_measured() {
        let m = model(SAMPLE);
        let root = type_of(&m, "urn:test", "RootType");
        assert_eq!(root.to_string(), "{urn:test}RootType");
        assert_eq!(
            root.type_of().unwrap(),
            BslValue::Type(TypeId::XdtoObjectType)
        );
        assert_eq!(TypeId::XdtoObjectType.name(), "Тип объекта XDTO");
        let code = type_of(&m, "urn:test", "Code");
        assert_eq!(code.to_string(), "{urn:test}Code");
        assert_eq!(
            code.type_of().unwrap(),
            BslValue::Type(TypeId::XdtoValueType)
        );
        let props = prop(&root, "Свойства");
        assert_eq!(
            props.type_of().unwrap(),
            BslValue::Type(TypeId::XdtoPropertyCollection)
        );
        assert_eq!(props.to_string(), "КоллекцияСвойствXDTO");
        let name = collection_lookup(&props, &[str_value("name")]).expect("поиск");
        assert_eq!(name.to_string(), "name", "свойство печатается именем");
        assert_eq!(
            name.type_of().unwrap(),
            BslValue::Type(TypeId::XdtoProperty)
        );
        // Анонимный тип печатается ПУСТОЙ строкой, хотя URI у него есть.
        let anon = prop(
            &collection_lookup(&props, &[str_value("anon")]).expect("поиск"),
            "Тип",
        );
        assert_eq!(anon.to_string(), "");
        assert_eq!(text_of(&prop(&anon, "URIПространстваИмен")), "urn:test");
        let facets = prop(&code, "Фасеты");
        assert_eq!(
            facets.type_of().unwrap(),
            BslValue::Type(TypeId::XdtoFacetCollection)
        );
        assert_eq!(facets.to_string(), "КоллекцияФасетовXDTO");
        match &facets {
            BslValue::Object(o) => {
                let f = collection_get(o, 0).expect("фасет");
                assert_eq!(f.type_of().unwrap(), BslValue::Type(TypeId::XdtoFacet));
                assert_eq!(f.to_string(), "ФасетXDTO");
            }
            other => panic!("ожидалась коллекция, получено {other:?}"),
        }
        let def = prop(
            &collection_lookup(&props, &[str_value("def")]).expect("поиск"),
            "ЗначениеПоУмолчанию",
        );
        assert_eq!(
            def.type_of().unwrap(),
            BslValue::Type(TypeId::XdtoDataValue)
        );
        assert_eq!(def.to_string(), "ЗначениеXDTO");
    }

    /// Разбор перечня фасетов из снятой строки: `Вид=[значение]` через
    /// пробел. Значение само может содержать скобки (`Образец` у
    /// `xs:integer` — это `[\-+]?[0-9]+`), поэтому конец значения ищется
    /// как последняя `]` перед следующим `=[`, а не как первая же.
    fn measured_facets(text: &str) -> Vec<(String, String)> {
        let mut out = Vec::new();
        let mut rest = text.trim();
        while let Some(eq) = rest.find("=[") {
            let key = rest[..eq].trim().to_string();
            let after = &rest[eq + 2..];
            let end = match after.find("=[") {
                Some(next) => after[..next].rfind(']'),
                None => after.rfind(']'),
            }
            .expect("у значения фасета есть закрывающая скобка");
            out.push((key, after[..end].to_string()));
            rest = &after[end + 1..];
        }
        out
    }

    /// Таблица встроенных типов сверяется СО СНЯТЫМ ФАЙЛОМ, а не с
    /// ожиданиями этого теста: строки `фв <имя>` дают базовый тип и
    /// фасеты, строки `вст <имя>` — тип BSL, в который платформа
    /// отобразила значение. Пока файл лежит рядом, ни одна строка
    /// [`BUILTIN_TYPES`] не может разъехаться с платформой незаметно.
    #[test]
    fn every_builtin_row_is_backed_by_a_measured_line() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/conformance/measure/measure-xdto.platform.txt");
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("не читается {}: {e}", path.display()));
        let m = model(SAMPLE);

        // `фв <имя>` -> базовый тип и фасеты.
        let mut checked_bases = 0;
        let mut checked_values = 0;
        for line in text.lines() {
            let Some((label, value)) = line.split_once('\t') else {
                continue;
            };
            if let Some(name) = label.strip_prefix("фв ") {
                let index = m
                    .find(XSD_NS, name)
                    .unwrap_or_else(|| panic!("в таблице нет встроенного типа {name}"));
                let t = type_value(&m, index);
                // `база=[{URI}имя] Вид=[значение] Вид=[значение] …`
                let (base_part, facet_part) = value.split_once(']').expect("база в строке");
                let base = base_part
                    .strip_prefix("база=[")
                    .expect("строка начинается с базы");
                assert_eq!(
                    prop(&t, "БазовыйТип").to_string(),
                    base,
                    "базовый тип {name}"
                );
                let measured = measured_facets(facet_part);
                let ours: Vec<(String, String)> = m.types[index]
                    .facets
                    .iter()
                    .map(|(kind, lexical)| {
                        (
                            facet_kind_value(*kind).display_text().to_string(),
                            lexical.clone(),
                        )
                    })
                    .collect();
                if facet_part.contains("фасеты=<нет>") {
                    assert!(ours.is_empty(), "у {name} фасетов быть не должно");
                } else {
                    assert_eq!(ours, measured, "фасеты {name}");
                }
                checked_bases += 1;
                continue;
            }
            // `вст <имя>` -> `… знач=<Тип> [значение] …`; строки с
            // уточнением в метке («вст boolean цифрой») пропускаются:
            // тип там тот же, а сверяется таблица, а не разбор.
            let Some(name) = label.strip_prefix("вст ") else {
                continue;
            };
            let Some(index) = m.find(XSD_NS, name) else {
                continue;
            };
            match value.split_once("знач=") {
                Some((_, rest)) if rest.starts_with("<не создаётся>") => {
                    assert!(
                        m.builtin_of(index).is_none(),
                        "{name} не должен строить значение"
                    );
                }
                Some((_, rest)) => {
                    let measured = rest.split_once(" [").expect("значение в скобках").0;
                    let ours = m
                        .builtin_of(index)
                        .unwrap_or_else(|| panic!("{name} обязан отображаться в тип BSL"));
                    assert_eq!(ours.type_id().name(), measured, "тип BSL для {name}");
                }
                None => continue,
            }
            checked_values += 1;
        }
        // Обе выборки непусты и покрывают таблицу: иначе тест зелен
        // просто оттого, что ничего не нашёл.
        assert_eq!(
            checked_bases,
            BUILTIN_TYPES.len() - 1,
            "строк «фв» должно быть по одной на каждый тип, кроме anyType"
        );
        assert!(checked_values >= 30, "строк «вст» найдено {checked_values}");
    }

    /// Тождество типов: два обращения к одному имени дают РАВНЫЕ значения
    /// (измерено), а разные типы не равны.
    #[test]
    fn types_are_references_into_one_model() {
        let m = model(SAMPLE);
        assert_eq!(
            type_of(&m, "urn:test", "RootType"),
            type_of(&m, "urn:test", "RootType")
        );
        assert_ne!(
            type_of(&m, "urn:test", "RootType"),
            type_of(&m, "urn:test", "ExtType")
        );
        assert!(m.find("urn:нет", "RootType").is_none(), "чужой URI");
        assert!(m.find("urn:test", "нетТакого").is_none());
    }

    /// Ошибочные пути отвечают `RtError`, а не паникой.
    #[test]
    fn broken_schemas_report_errors_instead_of_panicking() {
        // Ссылка на несуществующий тип.
        let schema = crate::xsd::build_for_tests(concat!(
            r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" targetNamespace="urn:t">"#,
            r#"<xs:complexType name="T"><xs:sequence>"#,
            r#"<xs:element name="a" type="xs:нетТакого"/>"#,
            r#"</xs:sequence></xs:complexType></xs:schema>"#,
        ))
        .expect("схема разбирается");
        let error = model_of_schema(&schema).expect_err("тип не разрешается");
        assert!(
            error.to_string().contains("нетТакого"),
            "в тексте ошибки нет имени типа: {error}"
        );

        // Кольцо в цепочке базовых типов простого типа: разбор
        // лексической формы обязан отвечать ошибкой, а не переполнением
        // стека.
        let schema = crate::xsd::build_for_tests(concat!(
            r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" xmlns:t="urn:t" "#,
            r#"targetNamespace="urn:t">"#,
            r#"<xs:simpleType name="A"><xs:restriction base="t:A"/></xs:simpleType>"#,
            r#"</xs:schema>"#,
        ))
        .expect("схема разбирается");
        let cyclic =
            model_of_schema(&schema).expect("модель строится: цикл здесь только у значений");
        let a = cyclic.find("urn:t", "A").expect("тип A");
        assert!(value_from_lexical(&cyclic, a, "что-нибудь").is_err());
        assert!(cyclic.builtin_of(a).is_none(), "кольцо не даёт отображения");

        // Цикл наследования типов ОБЪЕКТА ловится при построении модели.
        let schema = crate::xsd::build_for_tests(concat!(
            r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" xmlns:t="urn:t" "#,
            r#"targetNamespace="urn:t">"#,
            r#"<xs:complexType name="A"><xs:complexContent>"#,
            r#"<xs:extension base="t:B"/></xs:complexContent></xs:complexType>"#,
            r#"<xs:complexType name="B"><xs:complexContent>"#,
            r#"<xs:extension base="t:A"/></xs:complexContent></xs:complexType>"#,
            r#"</xs:schema>"#,
        ))
        .expect("схема разбирается");
        let error = model_of_schema(&schema).expect_err("цикл наследования");
        assert!(
            error.to_string().contains("цикл"),
            "в тексте ошибки нет слова про цикл: {error}"
        );

        // Значение по умолчанию, не разбирающееся в своём типе.
        let schema = crate::xsd::build_for_tests(concat!(
            r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" targetNamespace="urn:t">"#,
            r#"<xs:complexType name="T"><xs:sequence>"#,
            r#"<xs:element name="a" type="xs:int" default="ерунда"/>"#,
            r#"</xs:sequence></xs:complexType></xs:schema>"#,
        ))
        .expect("схема разбирается");
        assert!(model_of_schema(&schema).is_err(), "мусор в default");

        // Тот же путь, но лексическая форма испорчена так, что разбор
        // `xs:date` берёт срез по НЕ границе символа: «2026-08-1я» длиннее
        // десяти байт, и десятый байт лежит внутри «я». Схема доходит сюда
        // сама (`collect_elements` -> `has_constraint` -> `value_of`), так
        // что ответом обязана быть ошибка, а не паника процесса.
        let schema = crate::xsd::build_for_tests(concat!(
            r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" targetNamespace="urn:t">"#,
            r#"<xs:complexType name="T"><xs:sequence>"#,
            r#"<xs:element name="a" type="xs:date" default="2026-08-1я"/>"#,
            r#"</xs:sequence></xs:complexType></xs:schema>"#,
        ))
        .expect("схема разбирается");
        assert!(
            model_of_schema(&schema).is_err(),
            "испорченная дата в default"
        );

        // Неизвестный член — `RtError`, а не паника.
        let m = model(SAMPLE);
        let root = type_of(&m, "urn:test", "RootType");
        assert!(get_property(&root, "НетТакогоЧлена").is_err());
        // Члены типа ЗНАЧЕНИЯ на типе объекта не отвечают, и наоборот.
        assert!(get_property(&root, "Фасеты").is_err());
        assert!(get_property(&type_of(&m, "urn:test", "Code"), "Свойства").is_err());
    }
}
