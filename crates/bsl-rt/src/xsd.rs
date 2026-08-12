//! Объектная модель XML-схемы: `ПостроительСхемXML` разбирает готовое
//! дерево DOM в `СхемаXML`, `НаборСхемXML` собирает схемы по пространствам
//! имён.
//!
//! Второго разборщика XML здесь нет: на вход идёт дерево, построенное
//! [`crate::dom`], а этот модуль только опознаёт в нём элементы схемы — ПО
//! ПРОСТРАНСТВУ ИМЁН `http://www.w3.org/2001/XMLSchema`, а не по префиксу
//! `xs:`. Измерено, что префикс не важен: схема с `<я:schema
//! xmlns:я="...XMLSchema">` разбирается так же, как с `xs:`, а схема, чей
//! корень лежит в ЧУЖОМ пространстве имён, схемой не считается.
//!
//! Модель ЛЕКСИЧЕСКАЯ, и это главное, что нужно про неё знать. Платформа на
//! этом этапе НЕ разрешает ссылки и НЕ выводит ничего, чего в тексте схемы
//! не написано: `ОпределениеБазовогоТипа`, `МодельСодержимого` и
//! `Использование` отдают `Неопределено` даже там, где вывести значение
//! было бы можно (`Использование` пусто и при явном `use="required"`,
//! обязательность несёт `Обязательный`). Границы вхождения работают ровно
//! по тому же правилу, а не «всегда пусты»: `МинимальноВходит` и
//! `МаксимальноВходит` показывают НАПИСАННОЕ и пусты только там, где
//! атрибута нет (измерено на одной схеме: у безатрибутного `<xs:sequence>`
//! обе пусты, а у `<xs:element minOccurs="0" maxOccurs="unbounded">` — 0 и
//! строка `unbounded`). Правду про написанное отдают лексические свойства:
//! `ИмяТипа`, `ИмяБазовогоТипа`, `ЛексическоеЗначение`, `Ссылка`.
//!
//! # Что ИЗМЕРЕНО на 8.3.27
//!
//! Всё перечисленное снято пробами (`tests/conformance/measure/measure-xsd.bsl`),
//! а не взято из справки и не из спецификации W3C — справка врёт в именах, и
//! здесь это подтвердилось дважды: русского имени `ОпределениеКомплексного\
//! ТипаXS` у платформы НЕТ (тип зовётся `ОпределениеСоставногоТипаXS`), а
//! частица (`XSParticle`) называется `ФрагментXS`.
//!
//! * `Новый ПостроительСхемXML` — без аргументов; `СоздатьСхемуXML` берёт
//!   РОВНО один аргумент: `ДокументDOM` или `ЭлементDOM`. Строку, читателя,
//!   `Неопределено`, ноль и два аргумента платформа отвергает, а документ
//!   без корня — ошибка;
//! * корень не `{...XMLSchema}schema` — это НЕ ошибка, а `Неопределено`
//!   (измерено на `<а><б/></а>` и на `<schema xmlns="urn:не-схема">`);
//! * каждый вызов строит НОВУЮ схему: `Пост.СоздатьСхемуXML(Док) = Сх` —
//!   «Нет»;
//! * `Компоненты` — в порядке ДОКУМЕНТА (первым идёт то, что записано
//!   первым, включая `xs:annotation`), а именованные коллекции
//!   (`ОбъявленияЭлементов`, `ОпределенияТипов`, …) — ОТСОРТИРОВАНЫ по
//!   имени: схема с элементами `я`, `B`, `а`, `A` отдаёт их как `A B а я`,
//!   то есть по кодам символов, латиница раньше кириллицы;
//! * `Получить` у именованной коллекции берёт имя, пару (URI, имя),
//!   `РасширенноеИмяXML` и ЧИСЛО (тогда это индекс); чужой URI и
//!   несуществующее имя дают `Неопределено`, а индекс за границей — ошибку;
//! * незнакомый элемент в пространстве имён схемы платформа ПРОПУСКАЕТ
//!   (`<xs:чушь/>` — ноль компонент), как и элемент чужого пространства
//!   имён, текст, комментарий и инструкцию обработки внутри схемы;
//! * дубль имени глобального элемента не ошибка: в именованной коллекции
//!   остаётся ПЕРВЫЙ (у `<xs:element name="а" type="xs:string"/>` следом за
//!   которым идёт `<xs:element name="а" type="xs:int"/>`, `ИмяТипа` —
//!   `{...}string`);
//! * `<xs:element>` без имени и `<xs:complexType>` без имени на уровне
//!   схемы пропускаются молча;
//! * QName в `type`/`base`/`ref`/`itemType` разрешается по объявлениям
//!   `xmlns` В ОБЛАСТИ ВИДИМОСТИ; неразрешимый префикс и имя БЕЗ префикса
//!   при отсутствии пространства имён по умолчанию дают `Неопределено`, а
//!   не «имя в пустом пространстве» (измерено: `type="нетТакого"` -> пусто,
//!   `type="нет:такого"` -> пусто, а под `xmlns="...XMLSchema"` то же
//!   `type="string"` -> `{...XMLSchema}string`);
//! * `URIПространстваИмен` глобального объявления — целевое пространство
//!   имён схемы всегда, локального — оно же при КВАЛИФИЦИРОВАННОЙ форме и
//!   пусто иначе, причём форму задаёт не только сам `form`, но и
//!   `elementFormDefault`/`attributeFormDefault` схемы. Измерено на трёх
//!   объявлениях одной схемы с `elementFormDefault="qualified"` и
//!   `attributeFormDefault="unqualified"`: локальный элемент БЕЗ `form` ->
//!   `urn:test` (сработала умолчательная форма схемы), локальный атрибут с
//!   `form="qualified"` -> `urn:test`, локальный атрибут без `form` ->
//!   пусто. Само свойство `Форма` при этом лексическое: у объявления без
//!   `form` оно `Неопределено`, хотя URI уже посчитан по умолчанию схемы;
//! * необъявленные атрибуты XSD отдаются как `Неопределено`, а объявленные
//!   — значением: `abstract="true"` -> `Истина`, `mixed="false"` -> `Ложь`,
//!   `fixed="true"` у фасета -> `Истина`, `form="qualified"` ->
//!   `ФормаПредставленияXS.Квалифицированная`. `xs:boolean` принимается и
//!   словом, и цифрой — измерены все четыре написания (`abstract="1"` ->
//!   `Истина`, `mixed="0"` -> `Ложь`);
//! * `ЛексическоеЗначение` объявления — это `default`/`fixed` как есть, а
//!   `Ограничение` говорит, который из двух: `По умолчанию` или
//!   `Фиксированное`. У ЛОКАЛЬНОГО объявления атрибута `Ограничение` —
//!   `Неопределено` (его несёт `ИспользованиеАтрибутаXS`), а
//!   `ЛексическоеЗначение` есть у обоих;
//! * `Значение` объявления — ТИПИЗИРОВАННОЕ: при `type="xs:int"
//!   default="5"` это `Число` 5, при `xs:boolean default="true"` —
//!   `Булево`, при `xs:string` — `Строка`;
//! * `Часть` фрагмента — это терм: объявление элемента, группа модели,
//!   определение группы модели (для `<xs:group ref>`) или маска (для
//!   `<xs:any>`). `Компоненты` фрагмента — один элемент, сам терм;
//! * границы вхождения фрагмента хранятся БЕЗЗНАКОВЫМ 32-битным числом, в
//!   котором `unbounded` — это `u32::MAX`, и оттого показываются с двух
//!   концов по-разному: `maxOccurs="unbounded"` -> `Строка` `unbounded`, а
//!   `minOccurs="unbounded"` -> `Число` 4294967295. Запись разбирается как
//!   десятичное целое со знаком по модулю 2^32 (`"-1"` -> 4294967295,
//!   `"4294967296"` -> 0), а нечисловая запись границы не задаёт вовсе.
//!   Полный перечень проб — у `Parser::occurs_attribute`;
//! * `ГруппаМоделиXS` отдаёт `ВидГруппы`
//!   (`Последовательность`/`Выбор`/`Все`) и свои фрагменты И через
//!   `Компоненты`, И через `Фрагменты`;
//! * `<xs:element ref="t:а"/>` даёт объявление с ПУСТЫМ `Имя`,
//!   `ЭтоГлобальноеОбъявление` — `Ложь`, `ИмяТипа` — `Неопределено` и
//!   `Ссылка` — `{urn:test}а`; тождества с глобальным объявлением НЕТ;
//! * простой тип: `Вариант` — `Атомарная` даже у `<xs:simpleType/>` без
//!   тела, `Список` при `xs:list`, `Объединение` при `xs:union`. У списка
//!   `itemType` виден как `ИмяТипаЭлемента`, а встроенный `<xs:simpleType>`
//!   — как `ОпределениеТипаЭлемента`; у объединения `memberTypes` — это
//!   `ИменаТиповОбъединения` (`СписокРасширенныхИменXML`), а
//!   `ОпределенияТиповОбъединения` пуст;
//! * `xs:restriction` без `base` — не ошибка: `ИмяБазовогоТипа` пусто, а
//!   фасеты разбираются;
//! * `АнонимноеОпределениеТипа` живёт у объявления, в `ОпределенияТипов`
//!   его нет, а `Компоненты` объявления состоят ровно из него. Если у
//!   элемента есть И `type`, И встроенный тип, платформа держит ОБА;
//! * `Контейнер` — родительская компонента: у глобального объявления это
//!   схема, у анонимного типа — объявление, у фасета — простой тип, у
//!   фрагмента — составной тип, у группы модели — фрагмент, у локального
//!   объявления атрибута — его использование. `Схема` у всех — сама схема;
//! * `ЭлементDOM` есть у КАЖДОЙ компоненты и указывает на породивший её
//!   элемент дерева;
//! * `ref` даёт ОТДЕЛЬНОЕ объявление с пустым именем: у элемента —
//!   `ЭтоГлобальноеОбъявление` = `Ложь`, `ИмяТипа` — `Неопределено`,
//!   `Ссылка` — расширенное имя, и тождества с глобальным объявлением НЕТ;
//!   у атрибута — такое же безымянное объявление внутри использования;
//! * `Фиксированный` есть РОВНО у числовых фасетов: у образца, перечисления
//!   и пробельных символов обращение к нему — ошибка (измерено все три),
//!   хотя спецификация XML Schema разрешает `fixed` и у `whiteSpace`;
//! * имя ТИПА и его ПРЕДСТАВЛЕНИЕ у этой модели — разные строки:
//!   `Строка(ТипЗнч(Объявление))` даёт «Объявление элемента XML Schema», а
//!   ищется тип как `Тип("ОбъявлениеЭлементаXS")`. Английские написания
//!   есть и у типов (`XSElementDeclaration`), и у ЧЛЕНОВ перечислений
//!   (`ФормаПредставленияXS.Qualified`) — перебраны все сорок семь;
//! * `НаборСхемXML` умеет ровно `Добавить`, `Количество`, `Получить`,
//!   индекс и обход. `ОбновитьНабор`, `Удалить`, `Очистить`, `Содержит`,
//!   `ОбъединеннаяСхема` платформа не знает. Повторное `Добавить` ТОЙ ЖЕ
//!   схемы молча ничего не делает (количество остаётся 1), а добавление
//!   ДРУГОЙ схемы с тем же целевым пространством имён — ошибка; схемы с
//!   разными пространствами складываются (2);
//! * `Новый СхемаXML` даёт пустую схему: пространство имён и версия —
//!   пустые строки, все коллекции пусты, `ЭлементDOM` — `Неопределено`,
//!   `ТипКомпоненты` — `Схема`.
//!
//! # Сознательные расхождения
//!
//! * **Конструкции за границей модели — честная ошибка разбора.**
//!   `СоздатьСхемуXML` ОТКАЗЫВАЕТ с именем конструкции в тексте на
//!   `xs:group`, `xs:attributeGroup`, `xs:any`, `xs:anyAttribute`,
//!   `xs:notation`, `xs:import`, `xs:include`, `xs:redefine` и
//!   `xs:key`/`xs:keyref`/`xs:unique`. Платформа две из них заведомо
//!   разбирает: `<xs:group name="г">` попадает в `ОпределенияГруппМоделей`
//!   (проба `группа модели в схеме` -> `групп=1`), а `<xs:any>` даёт термом
//!   `МаскаXS` (проба `маска в модели`). Про `xs:import` не известно даже
//!   этого: проба `импорт в схеме` вернула на платформе `<ошибка>`, то есть
//!   до `Директивы` не дошла и не показала ни типа директивы, ни самого
//!   факта, что схема с импортом строится, — `ИмпортXS` было бы ожиданием
//!   по названию члена `ТипКомпонентыXS.Import`, а не замером. Молчаливый
//!   пропуск был бы хуже ошибки: модель, тихо потерявшая половину схемы,
//!   врёт всем, кто её читает. Соответствующие коллекции
//!   (`ОпределенияГруппМоделей`, `Директивы`, …) поэтому всегда пусты, а
//!   контрактный `measure-xsd.bsl` расходится с платформой ровно на двух
//!   строках — тех самых `группа модели в схеме` и `маска в модели`, и это
//!   единственное его расхождение. На `xs:import` расхождения нет: там
//!   `<ошибка>` с обеих сторон.
//! * **`БазовыйТип` и `ОпределениеПримитивногоТипа` не реализованы.**
//!   Платформа их отдаёт, РАЗРЕШАЯ ссылку: у `Code` с `base="xs:string"`
//!   `БазовыйТип.Имя` — `string` из пространства имён XML Schema, у
//!   `ExtType` — `RootType` из самой схемы, у `RootType` — `anyType`. Чтобы
//!   так отвечать, нужна материализованная схема-для-схем со всеми
//!   встроенными типами, а их собственные свойства (фасеты, вариант,
//!   базовый тип) не измерены — придумывать их значило бы сочинять модель.
//!   Лексическую правду отдаёт `ИмяБазовогоТипа`, и её достаточно всему,
//!   что строится поверх.
//! * **`Значение` у фасета.** Платформа отдаёт здесь МУСОР: при
//!   `<xs:minLength value="2"/>` — `Число` 0, при `value="7"` — снова 0,
//!   при `<xs:totalDigits value="4"/>` — 1, а при `value="9"` — опять 1;
//!   `<xs:minInclusive value="1"/>` даёт `Булево` Да, `value="0"` — Ложь, а
//!   `value="5"` — `Число` 5. Осмысленно заполнены только фасеты, чьё
//!   значение и есть строка: образец и перечисление отдают её как есть, а
//!   `whiteSpace` — член `ОбработкаПробельныхСимволовXS`. Воспроизводить
//!   неинициализированное поле незачем: здесь `Значение` числового фасета —
//!   это число из `ЛексическоеЗначение`, а строкового — сама строка.
//!   `ЛексическоеЗначение`, которым и надо пользоваться, совпадает с
//!   платформой во всех случаях.
//! * **Фасеты только хранятся.** Проверка значений по ним (в том числе
//!   `pattern`, для которого нужен движок регулярных выражений) — работа
//!   отдельной задачи; здесь `pattern` лежит сырой строкой.

use std::cell::RefCell;
use std::rc::Rc;

use crate::dom::{DomKind, DomNode};
use crate::object::BslObject;
use crate::string::BslString;
use crate::{BslValue, EnumValue, RtError, RtResult};

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
    fn from_element(local: &str) -> Option<FacetKind> {
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
    fn component_type(self) -> EnumValue {
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
    fn type_name(self) -> &'static str {
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

    fn type_id(self) -> crate::types::TypeId {
        use crate::types::TypeId;
        match self {
            FacetKind::Length => TypeId::XsLengthFacet,
            FacetKind::MinLength => TypeId::XsMinLengthFacet,
            FacetKind::MaxLength => TypeId::XsMaxLengthFacet,
            FacetKind::Pattern => TypeId::XsPatternFacet,
            FacetKind::Enumeration => TypeId::XsEnumerationFacet,
            FacetKind::WhiteSpace => TypeId::XsWhitespaceFacet,
            FacetKind::TotalDigits => TypeId::XsTotalDigitsFacet,
            FacetKind::FractionDigits => TypeId::XsFractionDigitsFacet,
            FacetKind::MinInclusive => TypeId::XsMinInclusiveFacet,
            FacetKind::MaxInclusive => TypeId::XsMaxInclusiveFacet,
            FacetKind::MinExclusive => TypeId::XsMinExclusiveFacet,
            FacetKind::MaxExclusive => TypeId::XsMaxExclusiveFacet,
        }
    }

    /// Числовой ли это фасет — от этого зависит и вид `Значение`, и то,
    /// есть ли у фасета `Фиксированный`.
    ///
    /// Свойство `Фиксированный` платформа держит РОВНО у числовых фасетов:
    /// у образца, перечисления и пробельных символов обращение к нему —
    /// ошибка (измерено все три). Спецификация XML Schema разрешает
    /// `fixed` и у `whiteSpace`, но платформу мы не спорим, а измеряем.
    fn is_numeric(self) -> bool {
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

    /// `ТипЗнч()` компоненты.
    pub fn type_id(self) -> crate::types::TypeId {
        use crate::types::TypeId;
        match self {
            XsKind::Schema => TypeId::XmlSchema,
            XsKind::Element => TypeId::XsElementDeclaration,
            XsKind::Attribute => TypeId::XsAttributeDeclaration,
            XsKind::SimpleType => TypeId::XsSimpleTypeDefinition,
            XsKind::ComplexType => TypeId::XsComplexTypeDefinition,
            XsKind::Particle => TypeId::XsParticle,
            XsKind::ModelGroup => TypeId::XsModelGroup,
            XsKind::AttributeUse => TypeId::XsAttributeUse,
            XsKind::Facet(f) => f.type_id(),
            XsKind::Annotation => TypeId::XsAnnotation,
            XsKind::Documentation => TypeId::XsDocumentation,
            XsKind::AppInfo => TypeId::XsAppInfo,
        }
    }

    /// `ТипКомпоненты`.
    fn component_type(self) -> EnumValue {
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
enum XsData {
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
struct SchemaData {
    version: String,
    location: String,
    element_form: Option<EnumValue>,
    attribute_form: Option<EnumValue>,
    /// Именованные коллекции — отсортированные по имени номера узлов.
    elements: Vec<usize>,
    attributes: Vec<usize>,
    types: Vec<usize>,
}

/// Объявление элемента и объявление атрибута устроены одинаково; поля, у
/// которых смысл только у одного из двух (`abstract` у элемента), у другого
/// просто пусты.
#[derive(Debug, Default)]
struct DeclData {
    type_name: Option<XName>,
    reference: Option<XName>,
    anonymous_type: Option<usize>,
    form: Option<EnumValue>,
    is_abstract: Option<bool>,
    /// `default`/`fixed` как написано.
    lexical: String,
    /// Который из двух — и `Неопределено`, если ни одного. У ЛОКАЛЬНОГО
    /// объявления атрибута платформа держит здесь `Неопределено` даже при
    /// `default` (измерено), поэтому поле заполняется не всегда.
    constraint: Option<EnumValue>,
    global: bool,
}

#[derive(Debug, Default)]
struct SimpleTypeData {
    base_name: Option<XName>,
    variety: Option<EnumValue>,
    item_type_name: Option<XName>,
    item_type: Option<usize>,
    member_type_names: Vec<XName>,
    facets: Vec<usize>,
}

#[derive(Debug, Default)]
struct ComplexTypeData {
    base_name: Option<XName>,
    derivation: Option<EnumValue>,
    content: Option<usize>,
    attributes: Vec<usize>,
    mixed: Option<bool>,
    is_abstract: Option<bool>,
}

#[derive(Debug)]
struct AttributeUseData {
    declaration: usize,
    required: bool,
    lexical: String,
    constraint: Option<EnumValue>,
}

#[derive(Debug)]
struct FacetData {
    lexical: String,
    fixed: Option<bool>,
}

/// Одна компонента схемы. Дерево держится в общем массиве
/// [`XsSchemaData::nodes`], а связи — номерами: `Контейнер` и `Схема`
/// достаются подъёмом по `parent` без единой слабой ссылки, а тождество
/// компонент (измерено: два `Получить("root")` дают равные значения) — это
/// пара «та же схема, тот же номер».
#[derive(Debug)]
struct XsNode {
    kind: XsKind,
    parent: Option<usize>,
    name: String,
    ns: String,
    /// `Компоненты` — в порядке документа.
    children: Vec<usize>,
    /// `ЭлементDOM`; пусто только у схемы, построенной `Новый СхемаXML`.
    dom: Option<Rc<DomNode>>,
    data: XsData,
}

/// Разобранная схема целиком. Значение `СхемаXML` — это `Rc` на неё плюс
/// номер 0; любая другая компонента — тот же `Rc` плюс свой номер.
#[derive(Debug)]
pub struct XsSchemaData {
    nodes: Vec<XsNode>,
    /// Документ, из которого построена схема, — держит дерево живым, пока
    /// жива схема (у узлов внутри дерева ссылки вверх слабые).
    dom_doc: Option<Rc<DomNode>>,
}

impl XsSchemaData {
    fn node(&self, i: usize) -> &XsNode {
        &self.nodes[i]
    }

    /// Целевое пространство имён схемы.
    fn target_ns(&self) -> &str {
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
    fn items(&self) -> &[usize] {
        match self {
            XsListKind::Fixed(v) | XsListKind::Plain(v) | XsListKind::Named(v) => v,
        }
    }

    pub fn len(&self) -> usize {
        self.items().len()
    }

    pub fn is_empty(&self) -> bool {
        self.items().is_empty()
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            XsListKind::Fixed(_) => "ФиксированныйСписокКомпонентXS",
            XsListKind::Plain(_) => "СписокКомпонентXS",
            XsListKind::Named(_) => "КоллекцияИменованныхКомпонентXS",
        }
    }

    pub fn type_id(&self) -> crate::types::TypeId {
        use crate::types::TypeId;
        match self {
            XsListKind::Fixed(_) => TypeId::XsComponentFixedList,
            XsListKind::Plain(_) => TypeId::XsComponentList,
            XsListKind::Named(_) => TypeId::XsNamedComponentMap,
        }
    }
}

// --- значения ------------------------------------------------------------

fn str_value(s: &str) -> BslValue {
    BslValue::Str(BslString::from_str(s))
}

fn component_value(schema: &Rc<XsSchemaData>, index: usize) -> BslValue {
    BslValue::Object(Rc::new(BslObject::XsComponent(schema.clone(), index)))
}

fn list_value(schema: &Rc<XsSchemaData>, kind: XsListKind) -> BslValue {
    BslValue::Object(Rc::new(BslObject::XsList(schema.clone(), kind)))
}

fn name_value(name: &XName) -> BslValue {
    BslValue::Object(Rc::new(BslObject::XmlExpandedName(Rc::new(name.clone()))))
}

fn opt_name(name: Option<&XName>) -> BslValue {
    name.map_or(BslValue::Undefined, name_value)
}

fn opt_enum(v: Option<EnumValue>) -> BslValue {
    v.map_or(BslValue::Undefined, BslValue::Enum)
}

fn opt_bool(v: Option<bool>) -> BslValue {
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
fn occurs_value(occurs: Option<u32>, maximum: bool) -> BslValue {
    match occurs {
        None => BslValue::Undefined,
        Some(u32::MAX) if maximum => str_value("unbounded"),
        Some(n) => BslValue::Number(bsl_number::BslNumber::from_i64(i64::from(n))),
    }
}

/// `Новый ПостроительСхемXML`.
pub fn new_builder() -> BslValue {
    BslValue::Object(Rc::new(BslObject::XsBuilder))
}

/// `Новый НаборСхемXML`.
pub fn new_schema_set() -> BslValue {
    BslValue::Object(Rc::new(BslObject::XsSchemaSet(Rc::new(RefCell::new(
        Vec::new(),
    )))))
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
    matches!(v, BslValue::Object(o) if matches!(&**o, BslObject::XsBuilder))
}

pub fn is_schema_set(v: &BslValue) -> bool {
    matches!(v, BslValue::Object(o) if matches!(&**o, BslObject::XsSchemaSet(_)))
}

/// Компонента ли это и, если да, какая именно.
fn as_component(v: &BslValue) -> Option<(Rc<XsSchemaData>, usize)> {
    match v {
        BslValue::Object(o) => match &**o {
            BslObject::XsComponent(s, i) => Some((s.clone(), *i)),
            _ => None,
        },
        _ => None,
    }
}

// --- разбор --------------------------------------------------------------

fn unsupported(construct: &str) -> RtError {
    RtError::Xsd(format!(
        "конструкция «xs:{construct}» в схеме XML не поддерживается"
    ))
}

/// Разрешить QName по объявлениям `xmlns`, действующим на узле.
///
/// Возвращает `None` там же, где платформа отдаёт `Неопределено`:
/// префикс не объявлен, либо префикса нет и нет пространства имён по
/// умолчанию (измерено обе ветки).
fn resolve_qname(elem: &Rc<DomNode>, qname: &str) -> Option<XName> {
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
fn child_elements(elem: &Rc<DomNode>) -> Vec<Rc<DomNode>> {
    elem.xs_children()
        .into_iter()
        .filter(|c| c.kind() == DomKind::Element)
        .collect()
}

/// Дети из пространства имён схемы. Всё остальное — чужое пространство
/// имён — платформа пропускает (измерено).
fn xsd_children(elem: &Rc<DomNode>) -> Vec<Rc<DomNode>> {
    child_elements(elem)
        .into_iter()
        .filter(|c| c.xs_uri() == XSD_NS)
        .collect()
}

/// Состояние разбора: массив узлов плюс то, что нужно знать про схему
/// целиком (целевое пространство имён и формы по умолчанию).
struct Parser {
    nodes: Vec<XsNode>,
    target_ns: String,
    element_form_qualified: bool,
    attribute_form_qualified: bool,
}

impl Parser {
    fn push(&mut self, node: XsNode) -> usize {
        self.nodes.push(node);
        self.nodes.len() - 1
    }

    fn add_child(&mut self, parent: usize, child: usize) {
        self.nodes[parent].children.push(child);
        self.nodes[child].parent = Some(parent);
    }

    /// Пространство имён объявления: у глобального — целевое, у
    /// локального — целевое только при квалифицированной форме.
    fn declaration_ns(&self, global: bool, form: Option<EnumValue>, element: bool) -> String {
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

    fn form_of(elem: &Rc<DomNode>) -> Option<EnumValue> {
        match elem.xs_attribute("form").as_deref() {
            Some("qualified") => Some(EnumValue::XsFormQualified),
            Some("unqualified") => Some(EnumValue::XsFormUnqualified),
            _ => None,
        }
    }

    /// `default`/`fixed` объявления: текст и вид ограничения.
    fn value_constraint(elem: &Rc<DomNode>) -> (String, Option<EnumValue>) {
        if let Some(v) = elem.xs_attribute("default") {
            return (v, Some(EnumValue::XsConstraintDefault));
        }
        if let Some(v) = elem.xs_attribute("fixed") {
            return (v, Some(EnumValue::XsConstraintFixed));
        }
        (String::new(), None)
    }

    fn boolean_attribute(elem: &Rc<DomNode>, name: &str) -> Option<bool> {
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
    fn occurs_attribute(elem: &Rc<DomNode>, name: &str) -> Option<u32> {
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

    fn new_node(kind: XsKind, name: String, ns: String, dom: &Rc<DomNode>, data: XsData) -> XsNode {
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
    fn parse_annotation(&mut self, elem: &Rc<DomNode>, owner: usize) -> RtResult<usize> {
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
    fn parse_element_declaration(
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
    fn parse_declaration_children(&mut self, elem: &Rc<DomNode>, idx: usize) -> RtResult<()> {
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
    fn parse_attribute_declaration(
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
    fn parse_attribute_use(&mut self, elem: &Rc<DomNode>, owner: usize) -> RtResult<Option<usize>> {
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
    fn parse_simple_type(
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

    fn parse_simple_restriction(&mut self, elem: &Rc<DomNode>, owner: usize) -> RtResult<()> {
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

    fn parse_list(&mut self, elem: &Rc<DomNode>, owner: usize) -> RtResult<()> {
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

    fn parse_union(&mut self, elem: &Rc<DomNode>, owner: usize) -> RtResult<()> {
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

    fn parse_complex_type(
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
    fn parse_complex_body(&mut self, elem: &Rc<DomNode>, idx: usize) -> RtResult<()> {
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
                    if let Some(use_idx) = self.parse_attribute_use(&child, idx)? {
                        if let XsData::ComplexType(d) = &mut self.nodes[idx].data {
                            d.attributes.push(use_idx);
                        }
                    }
                }
                "group" | "attributeGroup" | "anyAttribute" => {
                    return Err(unsupported(child.xs_local_name()));
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// `xs:simpleContent`/`xs:complexContent` — один `extension` или
    /// `restriction` внутри.
    fn parse_derivation(&mut self, elem: &Rc<DomNode>, idx: usize) -> RtResult<()> {
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
    fn parse_particle_group(&mut self, elem: &Rc<DomNode>, owner: usize) -> RtResult<usize> {
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
                "group" | "any" => return Err(unsupported(child.xs_local_name())),
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
    let (node, doc) = match arg {
        BslValue::Object(o) => match &**o {
            BslObject::DomNode(n, d) => (n.clone(), d.clone()),
            _ => return Err(RtError::Xsd(source_error())),
        },
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

fn source_error() -> String {
    "СоздатьСхемуXML принимает ДокументDOM или ЭлементDOM".to_string()
}

fn build_schema(root: &Rc<DomNode>, doc: &Rc<DomNode>) -> RtResult<BslValue> {
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
            "import" | "include" | "redefine" | "group" | "attributeGroup" | "notation" => {
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

// --- свойства ------------------------------------------------------------

/// Свойство компоненты схемы, коллекции компонент, набора схем или
/// расширенного имени.
///
/// # Errors
///
/// [`RtError::UnknownColumn`], если такого свойства у этого вида
/// компоненты нет.
pub fn get_property(obj: &BslValue, name: &str) -> RtResult<BslValue> {
    let unknown = || RtError::UnknownColumn(name.to_string());
    let is = |ru: &str, en: &str| name.eq_ignore_ascii_case(ru) || name.eq_ignore_ascii_case(en);

    match obj {
        BslValue::Object(o) => match &**o {
            BslObject::XmlExpandedName(n) => {
                if is("ЛокальноеИмя", "LocalName") {
                    return Ok(str_value(&n.local));
                }
                if is("URIПространстваИмен", "NamespaceURI") {
                    return Ok(str_value(&n.uri));
                }
                Err(unknown())
            }
            BslObject::XsComponent(schema, index) => component_property(schema, *index, name),
            _ => Err(unknown()),
        },
        _ => Err(unknown()),
    }
}

fn component_property(schema: &Rc<XsSchemaData>, index: usize, name: &str) -> RtResult<BslValue> {
    let unknown = || RtError::UnknownColumn(name.to_string());
    let is = |ru: &str, en: &str| name.eq_ignore_ascii_case(ru) || name.eq_ignore_ascii_case(en);
    let node = schema.node(index);

    // Общее у всех компонент.
    if is("ТипКомпоненты", "ComponentType") {
        return Ok(BslValue::Enum(node.kind.component_type()));
    }
    if is("Схема", "Schema") {
        return Ok(component_value(schema, 0));
    }
    if is("Контейнер", "Container") {
        return Ok(match node.parent {
            Some(p) => component_value(schema, p),
            None => BslValue::Undefined,
        });
    }
    if is("Компоненты", "Components") {
        return Ok(list_value(schema, XsListKind::Fixed(node.children.clone())));
    }
    if is("ЭлементDOM", "DOMElement") {
        return Ok(match (&node.dom, &schema.dom_doc) {
            (Some(n), Some(doc)) => crate::dom::node_value(n, doc),
            _ => BslValue::Undefined,
        });
    }
    if is("Аннотация", "Annotation") {
        return Ok(annotation_of(schema, index));
    }

    match &node.data {
        XsData::Schema(d) => schema_property(schema, node, d, name),
        XsData::Element(d) | XsData::Attribute(d) => declaration_property(
            schema,
            node,
            d,
            name,
            matches!(node.data, XsData::Element(_)),
        ),
        XsData::SimpleType(d) => simple_type_property(schema, node, d, name),
        XsData::ComplexType(d) => complex_type_property(schema, node, d, name),
        XsData::Particle {
            term,
            min_occurs,
            max_occurs,
        } => {
            if is("Часть", "Term") {
                return Ok(component_value(schema, *term));
            }
            // Границы вхождения — тоже ЛЕКСИЧЕСКИЕ: платформа отдаёт то,
            // что написано в самой частице, и `Неопределено`, когда
            // атрибута нет. Измерено на одной и той же схеме: у
            // безатрибутного `<xs:sequence>` обе границы пусты (проба `час
            // МинимальноВходит`), а у `<xs:element minOccurs="0"
            // maxOccurs="unbounded">` — 0 и строка `unbounded` (проба
            // `час0 …`).
            if is("МинимальноВходит", "MinOccurs") {
                return Ok(occurs_value(*min_occurs, false));
            }
            if is("МаксимальноВходит", "MaxOccurs") {
                return Ok(occurs_value(*max_occurs, true));
            }
            Err(unknown())
        }
        XsData::ModelGroup {
            compositor,
            particles,
        } => {
            if is("ВидГруппы", "GroupKind") {
                return Ok(BslValue::Enum(*compositor));
            }
            if is("Фрагменты", "Particles") {
                return Ok(list_value(schema, XsListKind::Plain(particles.clone())));
            }
            Err(unknown())
        }
        XsData::AttributeUse(d) => {
            if is("ОбъявлениеАтрибута", "AttributeDeclaration") {
                return Ok(component_value(schema, d.declaration));
            }
            if is("Обязательный", "Required") {
                return Ok(BslValue::Boolean(d.required));
            }
            // `Использование` платформа не заполняет (измерено даже при
            // `use="required"`), а обязательность отдаёт `Обязательный`.
            if is("Использование", "Use") {
                return Ok(BslValue::Undefined);
            }
            if is("ЛексическоеЗначение", "LexicalValue") {
                return Ok(str_value(&d.lexical));
            }
            if is("Ограничение", "Constraint") {
                return Ok(opt_enum(d.constraint));
            }
            if is("Значение", "Value") {
                let type_name = match &schema.node(d.declaration).data {
                    XsData::Attribute(decl) => decl.type_name.clone(),
                    _ => None,
                };
                return Ok(typed_value(&d.lexical, type_name.as_ref(), d.constraint));
            }
            Err(unknown())
        }
        XsData::Facet(d) => {
            if is("ЛексическоеЗначение", "LexicalValue") {
                return Ok(str_value(&d.lexical));
            }
            if is("Значение", "Value") {
                return Ok(facet_value(node.kind, &d.lexical));
            }
            if is("Фиксированный", "Fixed") {
                // Только у числовых фасетов — см. `FacetKind::is_numeric`.
                return match node.kind {
                    XsKind::Facet(f) if f.is_numeric() => Ok(opt_bool(d.fixed)),
                    _ => Err(unknown()),
                };
            }
            Err(unknown())
        }
        XsData::Annotation {
            documentation,
            appinfo,
        } => {
            if is("Документация", "Documentation") {
                return Ok(list_value(schema, XsListKind::Fixed(documentation.clone())));
            }
            if is("ИнформацияДляПриложения", "AppInfo") {
                return Ok(list_value(schema, XsListKind::Fixed(appinfo.clone())));
            }
            Err(unknown())
        }
        XsData::Documentation { lang, source } => {
            if is("Язык", "Language") {
                return Ok(str_value(lang));
            }
            if is("Источник", "Source") {
                return Ok(str_value(source));
            }
            Err(unknown())
        }
    }
}

/// Аннотация компоненты — первая среди её детей.
fn annotation_of(schema: &Rc<XsSchemaData>, index: usize) -> BslValue {
    schema
        .node(index)
        .children
        .iter()
        .find(|c| schema.node(**c).kind == XsKind::Annotation)
        .map_or(BslValue::Undefined, |c| component_value(schema, *c))
}

fn schema_property(
    schema: &Rc<XsSchemaData>,
    node: &XsNode,
    d: &SchemaData,
    name: &str,
) -> RtResult<BslValue> {
    let unknown = || RtError::UnknownColumn(name.to_string());
    let is = |ru: &str, en: &str| name.eq_ignore_ascii_case(ru) || name.eq_ignore_ascii_case(en);

    if is("ПространствоИмен", "TargetNamespace") {
        return Ok(str_value(&node.ns));
    }
    if is("Версия", "Version") {
        return Ok(str_value(&d.version));
    }
    if is("РасположениеСхемы", "SchemaLocation") {
        return Ok(str_value(&d.location));
    }
    if is("ФормаЭлементовПоУмолчанию", "ElementFormDefault") {
        return Ok(opt_enum(d.element_form));
    }
    if is("ФормаАтрибутовПоУмолчанию", "AttributeFormDefault") {
        return Ok(opt_enum(d.attribute_form));
    }
    // Блокировку и завершённость платформа отдаёт пустыми и там, где они
    // записаны; заводить для них поле незачем.
    if is("БлокировкаПоУмолчанию", "BlockDefault") || is("ЗавершенностьПоУмолчанию", "FinalDefault")
    {
        return Ok(BslValue::Undefined);
    }
    if is("ОбъявленияЭлементов", "ElementDeclarations") {
        return Ok(list_value(schema, XsListKind::Named(d.elements.clone())));
    }
    if is("ОбъявленияАтрибутов", "AttributeDeclarations") {
        return Ok(list_value(schema, XsListKind::Named(d.attributes.clone())));
    }
    if is("ОпределенияТипов", "TypeDefinitions") {
        return Ok(list_value(schema, XsListKind::Named(d.types.clone())));
    }
    // Групп, нотаций и ограничений идентичности эта модель не строит —
    // конструкции за её границей отвергаются разбором, — но коллекции
    // существуют и пусты.
    if is("ОпределенияГруппМоделей", "ModelGroupDefinitions")
        || is("ОпределенияГруппАтрибутов", "AttributeGroupDefinitions")
        || is("ОбъявленияНотаций", "NotationDeclarations")
        || is(
            "ОпределенияОграниченийИдентичности",
            "IdentityConstraintDefinitions",
        )
    {
        return Ok(list_value(schema, XsListKind::Named(Vec::new())));
    }
    if is("Директивы", "Directives") {
        return Ok(list_value(schema, XsListKind::Plain(Vec::new())));
    }
    Err(unknown())
}

fn declaration_property(
    schema: &Rc<XsSchemaData>,
    node: &XsNode,
    d: &DeclData,
    name: &str,
    element: bool,
) -> RtResult<BslValue> {
    let unknown = || RtError::UnknownColumn(name.to_string());
    let is = |ru: &str, en: &str| name.eq_ignore_ascii_case(ru) || name.eq_ignore_ascii_case(en);

    if is("Имя", "Name") {
        return Ok(str_value(&node.name));
    }
    if is("URIПространстваИмен", "NamespaceURI") {
        return Ok(str_value(&node.ns));
    }
    if is("ИмяТипа", "TypeName") {
        return Ok(opt_name(d.type_name.as_ref()));
    }
    if is("АнонимноеОпределениеТипа", "AnonymousTypeDefinition") {
        return Ok(match d.anonymous_type {
            Some(i) => component_value(schema, i),
            None => BslValue::Undefined,
        });
    }
    if is("Ссылка", "Reference") {
        return Ok(opt_name(d.reference.as_ref()));
    }
    if is("Форма", "Form") {
        return Ok(opt_enum(d.form));
    }
    if is("ЛексическоеЗначение", "LexicalValue") {
        return Ok(str_value(&d.lexical));
    }
    if is("Ограничение", "Constraint") {
        return Ok(opt_enum(d.constraint));
    }
    if is("Значение", "Value") {
        return Ok(typed_value(&d.lexical, d.type_name.as_ref(), d.constraint));
    }
    if is("ЭтоГлобальноеОбъявление", "IsGlobalDeclaration") {
        return Ok(BslValue::Boolean(d.global));
    }
    if is("Блокировка", "Block") || is("Завершенность", "Final") {
        return Ok(BslValue::Undefined);
    }
    if element && is("Абстрактный", "Abstract") {
        return Ok(opt_bool(d.is_abstract));
    }
    Err(unknown())
}

fn simple_type_property(
    schema: &Rc<XsSchemaData>,
    node: &XsNode,
    d: &SimpleTypeData,
    name: &str,
) -> RtResult<BslValue> {
    let unknown = || RtError::UnknownColumn(name.to_string());
    let is = |ru: &str, en: &str| name.eq_ignore_ascii_case(ru) || name.eq_ignore_ascii_case(en);

    if is("Имя", "Name") {
        return Ok(str_value(&node.name));
    }
    if is("URIПространстваИмен", "NamespaceURI") {
        return Ok(str_value(&node.ns));
    }
    if is("ИмяБазовогоТипа", "BaseTypeName") {
        return Ok(opt_name(d.base_name.as_ref()));
    }
    if is("ОпределениеБазовогоТипа", "BaseTypeDefinition") {
        return Ok(BslValue::Undefined);
    }
    if is("Вариант", "Variety") {
        return Ok(opt_enum(d.variety));
    }
    if is("Фасеты", "Facets") {
        return Ok(list_value(schema, XsListKind::Plain(d.facets.clone())));
    }
    if is("ИмяТипаЭлемента", "ItemTypeName") {
        return Ok(opt_name(d.item_type_name.as_ref()));
    }
    if is("ОпределениеТипаЭлемента", "ItemTypeDefinition") {
        return Ok(match d.item_type {
            Some(i) => component_value(schema, i),
            None => BslValue::Undefined,
        });
    }
    if is("ИменаТиповОбъединения", "MemberTypeNames") {
        return Ok(BslValue::Object(Rc::new(BslObject::XmlExpandedNameList(
            Rc::new(d.member_type_names.clone()),
        ))));
    }
    if is("ОпределенияТиповОбъединения", "MemberTypeDefinitions") {
        return Ok(list_value(schema, XsListKind::Plain(Vec::new())));
    }
    if is("Завершенность", "Final") {
        return Ok(BslValue::Undefined);
    }
    Err(unknown())
}

fn complex_type_property(
    schema: &Rc<XsSchemaData>,
    node: &XsNode,
    d: &ComplexTypeData,
    name: &str,
) -> RtResult<BslValue> {
    let unknown = || RtError::UnknownColumn(name.to_string());
    let is = |ru: &str, en: &str| name.eq_ignore_ascii_case(ru) || name.eq_ignore_ascii_case(en);

    if is("Имя", "Name") {
        return Ok(str_value(&node.name));
    }
    if is("URIПространстваИмен", "NamespaceURI") {
        return Ok(str_value(&node.ns));
    }
    if is("ИмяБазовогоТипа", "BaseTypeName") {
        return Ok(opt_name(d.base_name.as_ref()));
    }
    if is("ОпределениеБазовогоТипа", "BaseTypeDefinition") {
        return Ok(BslValue::Undefined);
    }
    if is("МетодНаследования", "DerivationMethod") {
        return Ok(opt_enum(d.derivation));
    }
    if is("Содержимое", "Content") {
        return Ok(match d.content {
            Some(i) => component_value(schema, i),
            None => BslValue::Undefined,
        });
    }
    // Модель содержимого и маска атрибутов — вычисляемые свойства, которых
    // платформа на этом этапе не заполняет.
    if is("МодельСодержимого", "ContentModel") || is("МаскаАтрибутов", "AttributeWildcard")
    {
        return Ok(BslValue::Undefined);
    }
    if is("Атрибуты", "Attributes") {
        return Ok(list_value(schema, XsListKind::Plain(d.attributes.clone())));
    }
    if is("Смешанный", "Mixed") {
        return Ok(opt_bool(d.mixed));
    }
    if is("Абстрактный", "Abstract") {
        return Ok(opt_bool(d.is_abstract));
    }
    if is("Блокировка", "Block") || is("Завершенность", "Final") {
        return Ok(BslValue::Undefined);
    }
    Err(unknown())
}

/// Типизированное `Значение` объявления: платформа приводит
/// `default`/`fixed` к типу, объявленному в `type`.
///
/// Измерены три приведения — `xs:string` -> `Строка`, `xs:int` -> `Число`,
/// `xs:boolean` -> `Булево`. `НЕ ИЗМЕРЕНО(XSD.TYPED_VALUE)`: во что
/// превращаются значения остальных встроенных типов (дата, десятичное,
/// `base64Binary`) и типов самой схемы; здесь всё прочее остаётся строкой,
/// то есть текстом, как записано.
fn typed_value(
    lexical: &str,
    type_name: Option<&XName>,
    constraint: Option<EnumValue>,
) -> BslValue {
    if constraint.is_none() {
        // Значения нет вовсе: `ЛексическоеЗначение` пусто, а `Значение` —
        // `Неопределено` (измерено на объявлении без `default`/`fixed`).
        return BslValue::Undefined;
    }
    let Some(name) = type_name else {
        return str_value(lexical);
    };
    if name.uri != XSD_NS {
        return str_value(lexical);
    }
    match name.local.as_str() {
        "boolean" => match lexical {
            "true" | "1" => BslValue::Boolean(true),
            "false" | "0" => BslValue::Boolean(false),
            _ => str_value(lexical),
        },
        "int" | "integer" | "long" | "short" | "byte" | "decimal" | "float" | "double"
        | "nonNegativeInteger" | "positiveInteger" | "nonPositiveInteger" | "negativeInteger"
        | "unsignedInt" | "unsignedLong" | "unsignedShort" | "unsignedByte" => {
            match bsl_number::BslNumber::parse_canonical(lexical) {
                Ok(n) => BslValue::Number(n),
                Err(_) => str_value(lexical),
            }
        }
        _ => str_value(lexical),
    }
}

/// `Значение` фасета: у строковых фасетов это сама строка, у `whiteSpace` —
/// член перечисления, у числовых — число. Расхождение с платформой описано
/// в заголовке модуля: там числовые фасеты отдают неинициализированное
/// поле.
fn facet_value(kind: XsKind, lexical: &str) -> BslValue {
    let XsKind::Facet(facet) = kind else {
        return str_value(lexical);
    };
    match facet {
        FacetKind::WhiteSpace => match lexical {
            "preserve" => BslValue::Enum(EnumValue::XsWhitespacePreserve),
            "replace" => BslValue::Enum(EnumValue::XsWhitespaceReplace),
            "collapse" => BslValue::Enum(EnumValue::XsWhitespaceCollapse),
            _ => str_value(lexical),
        },
        f if f.is_numeric() => match bsl_number::BslNumber::parse_canonical(lexical) {
            Ok(n) => BslValue::Number(n),
            Err(_) => str_value(lexical),
        },
        _ => str_value(lexical),
    }
}

// --- коллекции -----------------------------------------------------------

/// Элемент коллекции по номеру.
///
/// # Errors
///
/// [`RtError::IndexOutOfBounds`], если номер за границей: платформа на
/// `Фасеты[9]` и `ОбъявленияЭлементов[9]` отвечает ошибкой, а не
/// `Неопределено` (измерено).
pub fn list_get(schema: &Rc<XsSchemaData>, kind: &XsListKind, i: usize) -> RtResult<BslValue> {
    let items = kind.items();
    match items.get(i) {
        Some(idx) => Ok(component_value(schema, *idx)),
        None => Err(RtError::IndexOutOfBounds {
            index: i as i64,
            len: items.len(),
        }),
    }
}

/// `Получить` у коллекции компонент.
///
/// У именованной коллекции аргументом может быть имя, пара (URI, имя),
/// `РасширенноеИмяXML` или число (тогда это номер) — измерены все четыре
/// формы; ненайденное имя даёт `Неопределено`, а номер за границей —
/// ошибку.
///
/// # Errors
///
/// [`RtError::MethodNotApplicable`] при неподходящем числе аргументов и
/// [`RtError::IndexOutOfBounds`] при номере за границей.
pub fn list_lookup(obj: &BslValue, args: &[BslValue]) -> RtResult<BslValue> {
    let BslValue::Object(o) = obj else {
        return Err(method_error("Получить", obj));
    };
    let BslObject::XsList(schema, kind) = &**o else {
        return Err(method_error("Получить", obj));
    };
    let named = matches!(kind, XsListKind::Named(_));
    match args {
        [BslValue::Number(n)] => {
            let i = n
                .to_i64_exact()
                .and_then(|v| usize::try_from(v).ok())
                .ok_or(RtError::BadIndex)?;
            list_get(schema, kind, i)
        }
        [BslValue::Str(s)] if named => Ok(named_lookup(schema, kind, None, &s.to_string())),
        [BslValue::Object(name)] if named => match &**name {
            BslObject::XmlExpandedName(n) => Ok(named_lookup(schema, kind, Some(&n.uri), &n.local)),
            _ => Err(method_error("Получить", obj)),
        },
        [BslValue::Str(uri), BslValue::Str(local)] if named => Ok(named_lookup(
            schema,
            kind,
            Some(&uri.to_string()),
            &local.to_string(),
        )),
        _ => Err(method_error("Получить", obj)),
    }
}

fn named_lookup(
    schema: &Rc<XsSchemaData>,
    kind: &XsListKind,
    uri: Option<&str>,
    local: &str,
) -> BslValue {
    for idx in kind.items() {
        let node = schema.node(*idx);
        let uri_matches = match uri {
            Some(u) => u == node.ns,
            None => true,
        };
        if node.name == local && uri_matches {
            return component_value(schema, *idx);
        }
    }
    BslValue::Undefined
}

fn method_error(method: &'static str, obj: &BslValue) -> RtError {
    RtError::MethodNotApplicable {
        method,
        receiver: obj.type_name(),
    }
}

/// Элемент `СписокРасширенныхИменXML` по номеру.
///
/// # Errors
///
/// [`RtError::IndexOutOfBounds`] при номере за границей.
pub fn name_list_get(names: &[XName], i: usize) -> RtResult<BslValue> {
    match names.get(i) {
        Some(n) => Ok(name_value(n)),
        None => Err(RtError::IndexOutOfBounds {
            index: i as i64,
            len: names.len(),
        }),
    }
}

// --- набор схем ----------------------------------------------------------

/// `НаборСхемXML.Добавить(СхемаXML)`.
///
/// # Errors
///
/// [`RtError::Xsd`], если аргумент не схема либо в наборе уже есть ДРУГАЯ
/// схема с тем же целевым пространством имён (измерено: платформа на этом
/// отказывает, а повторное добавление ТОЙ ЖЕ схемы молча пропускает).
pub fn schema_set_add(obj: &BslValue, args: &[BslValue]) -> RtResult<BslValue> {
    let BslValue::Object(o) = obj else {
        return Err(method_error("Добавить", obj));
    };
    let BslObject::XsSchemaSet(list) = &**o else {
        return Err(method_error("Добавить", obj));
    };
    let [arg] = args else {
        return Err(method_error("Добавить", obj));
    };
    let schema = match as_component(arg) {
        Some((s, 0)) => s,
        _ => return Err(RtError::Xsd("Добавить принимает СхемаXML".to_string())),
    };
    let mut list = list.borrow_mut();
    if list.iter().any(|s| Rc::ptr_eq(s, &schema)) {
        return Ok(BslValue::Undefined);
    }
    if list.iter().any(|s| s.target_ns() == schema.target_ns()) {
        return Err(RtError::Xsd(format!(
            "в наборе уже есть схема пространства имён «{}»",
            schema.target_ns()
        )));
    }
    list.push(schema);
    Ok(BslValue::Undefined)
}

/// Схема набора по номеру.
///
/// # Errors
///
/// [`RtError::IndexOutOfBounds`] при номере за границей — измерено, что
/// платформа отвечает именно ошибкой.
pub fn schema_set_get(obj: &BslValue, i: usize) -> RtResult<BslValue> {
    let BslValue::Object(o) = obj else {
        return Err(method_error("Получить", obj));
    };
    let BslObject::XsSchemaSet(list) = &**o else {
        return Err(method_error("Получить", obj));
    };
    let list = list.borrow();
    match list.get(i) {
        Some(s) => Ok(component_value(s, 0)),
        None => Err(RtError::IndexOutOfBounds {
            index: i as i64,
            len: list.len(),
        }),
    }
}

/// Имя типа значения и `ТипЗнч()` для объектов этого модуля — обе таблицы
/// живут рядом, чтобы не разъезжались.
pub fn type_name_of(obj: &BslObject) -> Option<&'static str> {
    Some(match obj {
        BslObject::XsBuilder => "ПостроительСхемXML",
        BslObject::XsSchemaSet(_) => "НаборСхемXML",
        BslObject::XsComponent(schema, i) => schema.node(*i).kind.type_name(),
        BslObject::XsList(_, kind) => kind.type_name(),
        BslObject::XmlExpandedName(_) => "РасширенноеИмяXML",
        BslObject::XmlExpandedNameList(_) => "СписокРасширенныхИменXML",
        _ => return None,
    })
}

pub fn type_id_of(obj: &BslObject) -> Option<crate::types::TypeId> {
    use crate::types::TypeId;
    Some(match obj {
        BslObject::XsBuilder => TypeId::XmlSchemaBuilder,
        BslObject::XsSchemaSet(_) => TypeId::XmlSchemaSet,
        BslObject::XsComponent(schema, i) => schema.node(*i).kind.type_id(),
        BslObject::XsList(_, kind) => kind.type_id(),
        BslObject::XmlExpandedName(_) => TypeId::XmlExpandedName,
        BslObject::XmlExpandedNameList(_) => TypeId::XmlExpandedNameList,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::object::XmlReaderState;
    use crate::xml::XmlParser;

    /// Схема из текста XSD — тем же путём, что и в бою: дерево строит
    /// `dom`, а разбирает уже готовое дерево этот модуль.
    fn schema_of(text: &str) -> RtResult<BslValue> {
        let mut state = XmlReaderState::over(XmlParser::new(text));
        let doc = crate::dom::build_for_tests(&mut state).expect("дерево обязано строиться");
        let value = crate::dom::node_value(&doc, &doc);
        create_schema(&new_builder(), &[value])
    }

    fn schema(text: &str) -> BslValue {
        schema_of(text).expect("схема обязана разбираться")
    }

    fn prop(obj: &BslValue, name: &str) -> BslValue {
        get_property(obj, name).unwrap_or_else(|e| panic!("свойство «{name}»: {e}"))
    }

    fn text_of(v: &BslValue) -> String {
        match v {
            BslValue::Str(s) => s.to_string(),
            other => panic!("ожидалась строка, получено {other:?}"),
        }
    }

    fn count(v: &BslValue) -> usize {
        v.collection_len().expect("коллекция обязана иметь длину")
    }

    fn item(v: &BslValue, i: usize) -> BslValue {
        match v {
            BslValue::Object(o) => match &**o {
                BslObject::XsList(s, kind) => list_get(s, kind, i).expect("элемент обязан быть"),
                _ => panic!("ожидалась коллекция компонент"),
            },
            other => panic!("ожидалась коллекция, получено {other:?}"),
        }
    }

    /// Схема с двумя глобальными объявлениями, простым и составным типом —
    /// её хватает почти всем проверкам ниже.
    const SAMPLE: &str = concat!(
        r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" xmlns:t="urn:t" "#,
        r#"targetNamespace="urn:t" elementFormDefault="qualified" version="1.7">"#,
        r#"<xs:element name="корень" type="t:Тип"/>"#,
        r#"<xs:element name="Апельсин" type="xs:string" default="да"/>"#,
        r#"<xs:attribute name="глоб" type="xs:int"/>"#,
        r#"<xs:simpleType name="Код"><xs:restriction base="xs:string">"#,
        r#"<xs:minLength value="2"/><xs:maxLength value="5" fixed="true"/>"#,
        r#"<xs:pattern value="[А-Я]+"/></xs:restriction></xs:simpleType>"#,
        r#"<xs:complexType name="Тип"><xs:sequence>"#,
        r#"<xs:element name="имя" type="xs:string"/>"#,
        r#"<xs:element name="вложенный"><xs:complexType><xs:sequence>"#,
        r#"<xs:element name="внутри" type="xs:int"/>"#,
        r#"</xs:sequence></xs:complexType></xs:element>"#,
        r#"<xs:choice><xs:element name="а" type="xs:string"/></xs:choice>"#,
        r#"</xs:sequence>"#,
        r#"<xs:attribute name="ид" type="xs:int" use="required"/>"#,
        r#"</xs:complexType></xs:schema>"#,
    );

    #[test]
    fn schema_reports_its_namespace_version_and_forms() {
        let s = schema(SAMPLE);
        assert_eq!(text_of(&prop(&s, "ПространствоИмен")), "urn:t");
        assert_eq!(text_of(&prop(&s, "Версия")), "1.7");
        assert_eq!(
            prop(&s, "ФормаЭлементовПоУмолчанию"),
            BslValue::Enum(EnumValue::XsFormQualified)
        );
        // Неуказанное — `Неопределено`, а не значение по умолчанию.
        assert_eq!(prop(&s, "ФормаАтрибутовПоУмолчанию"), BslValue::Undefined);
        assert_eq!(prop(&s, "БлокировкаПоУмолчанию"), BslValue::Undefined);
        assert_eq!(prop(&s, "Контейнер"), BslValue::Undefined);
        assert_eq!(prop(&s, "Схема"), s, "схема — сама себе схема");
        assert_eq!(
            prop(&s, "ТипКомпоненты"),
            BslValue::Enum(EnumValue::XsCompSchema)
        );
    }

    /// Именованные коллекции ОТСОРТИРОВАНЫ по имени, а `Компоненты` идут в
    /// порядке документа — измерено на платформе, и это разный порядок.
    #[test]
    fn named_collections_are_sorted_while_components_follow_the_document() {
        let s = schema(SAMPLE);
        let elements = prop(&s, "ОбъявленияЭлементов");
        assert_eq!(count(&elements), 2);
        assert_eq!(text_of(&prop(&item(&elements, 0), "Имя")), "Апельсин");
        assert_eq!(text_of(&prop(&item(&elements, 1), "Имя")), "корень");

        let components = prop(&s, "Компоненты");
        assert_eq!(count(&components), 5);
        assert_eq!(text_of(&prop(&item(&components, 0), "Имя")), "корень");
        assert_eq!(text_of(&prop(&item(&components, 1), "Имя")), "Апельсин");
    }

    #[test]
    fn named_lookup_takes_a_name_a_pair_and_an_expanded_name() {
        let s = schema(SAMPLE);
        let elements = prop(&s, "ОбъявленияЭлементов");
        let by_name = list_lookup(&elements, &[BslValue::Str(BslString::from_str("корень"))])
            .expect("поиск по имени");
        assert_eq!(text_of(&prop(&by_name, "Имя")), "корень");
        let by_pair = list_lookup(
            &elements,
            &[
                BslValue::Str(BslString::from_str("urn:t")),
                BslValue::Str(BslString::from_str("корень")),
            ],
        )
        .expect("поиск по паре");
        assert_eq!(by_pair, by_name, "компоненты сравниваются по тождеству");
        let by_expanded = list_lookup(&elements, &[new_expanded_name("urn:t", "корень")])
            .expect("поиск по имени");
        assert_eq!(by_expanded, by_name);
        // Чужой URI и неизвестное имя — `Неопределено`, а не ошибка.
        assert_eq!(
            list_lookup(
                &elements,
                &[
                    BslValue::Str(BslString::from_str("urn:нет")),
                    BslValue::Str(BslString::from_str("корень")),
                ],
            )
            .expect("чужой URI"),
            BslValue::Undefined
        );
        assert_eq!(
            list_lookup(&elements, &[BslValue::Str(BslString::from_str("нет"))])
                .expect("нет имени"),
            BslValue::Undefined
        );
    }

    #[test]
    fn element_declaration_keeps_the_lexical_type_name_and_default() {
        let s = schema(SAMPLE);
        let elements = prop(&s, "ОбъявленияЭлементов");
        let root = list_lookup(&elements, &[BslValue::Str(BslString::from_str("корень"))]).unwrap();
        let type_name = prop(&root, "ИмяТипа");
        assert_eq!(text_of(&prop(&type_name, "ЛокальноеИмя")), "Тип");
        assert_eq!(text_of(&prop(&type_name, "URIПространстваИмен")), "urn:t");
        assert_eq!(text_of(&prop(&root, "URIПространстваИмен")), "urn:t");
        assert_eq!(
            prop(&root, "ЭтоГлобальноеОбъявление"),
            BslValue::Boolean(true)
        );
        assert_eq!(prop(&root, "АнонимноеОпределениеТипа"), BslValue::Undefined);
        // Без `default`/`fixed`: лексическое значение пусто, а
        // типизированное — `Неопределено` (измерено).
        assert_eq!(text_of(&prop(&root, "ЛексическоеЗначение")), "");
        assert_eq!(prop(&root, "Ограничение"), BslValue::Undefined);
        assert_eq!(prop(&root, "Значение"), BslValue::Undefined);

        let plain =
            list_lookup(&elements, &[BslValue::Str(BslString::from_str("Апельсин"))]).unwrap();
        assert_eq!(text_of(&prop(&plain, "ЛексическоеЗначение")), "да");
        assert_eq!(
            prop(&plain, "Ограничение"),
            BslValue::Enum(EnumValue::XsConstraintDefault)
        );
        assert_eq!(text_of(&prop(&plain, "Значение")), "да");
    }

    /// Границы вхождения частицы. Каждое ожидание здесь пришпилено к своей
    /// строке `measure-xsd.platform.txt`: `час0 …` (`minOccurs="0"
    /// maxOccurs="unbounded"` -> `Число [0]` и `Строка [unbounded]`),
    /// `выбор0 …` (написан только `minOccurs`, второй конец пуст), `границы
    /// числом` (2 и 5), `границы мусором` (`много` и пустая строка — обе
    /// пусты), `границы наоборот` (`unbounded` в `minOccurs` -> `Число [4
    /// 294 967 295]`, `-1` -> оно же) и `границы через край` (`4294967296`
    /// -> 0, двадцать девяток -> 1 661 992 959, `007` -> 7, `" 5 "` -> 5,
    /// `+3` -> 3).
    #[test]
    fn particle_occurs_bounds_repeat_what_is_written() {
        let s = schema(concat!(
            r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">"#,
            r#"<xs:complexType name="т"><xs:sequence>"#,
            r#"<xs:element name="а" type="xs:string" minOccurs="0" maxOccurs="unbounded"/>"#,
            r#"<xs:element name="б" type="xs:string" minOccurs="2" maxOccurs="5"/>"#,
            r#"<xs:element name="в" type="xs:string" minOccurs="много" maxOccurs=""/>"#,
            r#"<xs:element name="г" type="xs:string" minOccurs="unbounded"/>"#,
            r#"<xs:element name="д" type="xs:string" minOccurs="-1" maxOccurs="0"/>"#,
            r#"<xs:element name="е" type="xs:string" minOccurs="4294967296""#,
            r#" maxOccurs="99999999999999999999"/>"#,
            r#"<xs:element name="ж" type="xs:string" minOccurs="007" maxOccurs=" 5 "/>"#,
            r#"<xs:element name="з" type="xs:string" minOccurs="+3"/>"#,
            r#"<xs:choice minOccurs="0"><xs:element name="и" type="xs:string"/></xs:choice>"#,
            r#"</xs:sequence></xs:complexType></xs:schema>"#,
        ));
        let content = prop(&item(&prop(&s, "ОпределенияТипов"), 0), "Содержимое");
        // Сам `<xs:sequence>` написан без атрибутов — обе границы пусты.
        assert_eq!(prop(&content, "МинимальноВходит"), BslValue::Undefined);
        assert_eq!(prop(&content, "МаксимальноВходит"), BslValue::Undefined);

        let particles = prop(&prop(&content, "Часть"), "Фрагменты");
        let num = |n: i64| BslValue::Number(bsl_number::BslNumber::from_i64(n));
        let bounds = |i: usize| {
            let p = item(&particles, i);
            (prop(&p, "МинимальноВходит"), prop(&p, "МаксимальноВходит"))
        };
        // `unbounded` в `maxOccurs` — СТРОКА, а не число.
        assert_eq!(bounds(0), (num(0), str_value("unbounded")));
        assert_eq!(bounds(1), (num(2), num(5)));
        // Нечисло границы не задаёт: и слово, и пустая строка дают пусто.
        assert_eq!(bounds(2), (BslValue::Undefined, BslValue::Undefined));
        // Тот же `unbounded` со стороны `minOccurs` виден числом: внутри
        // это `u32::MAX`, а строкой его показывает только `МаксимальноВходит`.
        assert_eq!(bounds(3), (num(4_294_967_295), BslValue::Undefined));
        assert_eq!(bounds(4), (num(4_294_967_295), num(0)));
        // Разбор по модулю 2^32: 2^32 -> 0, а 10^20 - 1 -> 1 661 992 959.
        assert_eq!(bounds(5), (num(0), num(1_661_992_959)));
        assert_eq!(bounds(6), (num(7), num(5)));
        assert_eq!(bounds(7), (num(3), BslValue::Undefined));
        // Границы есть и у частицы, чей терм — вложенная группа модели.
        assert_eq!(bounds(8), (num(0), BslValue::Undefined));
    }

    /// `Значение` типизировано объявленным типом: `xs:int` даёт число,
    /// `xs:boolean` — булево (измерено), остальное остаётся строкой
    /// (`НЕ ИЗМЕРЕНО(XSD.TYPED_VALUE)`).
    #[test]
    fn typed_value_follows_the_declared_type() {
        let s = schema(concat!(
            r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">"#,
            r#"<xs:element name="ч" type="xs:int" default="5"/>"#,
            r#"<xs:element name="л" type="xs:boolean" default="true"/>"#,
            r#"<xs:element name="д" type="xs:date" default="2026-08-12"/>"#,
            r#"</xs:schema>"#,
        ));
        let elements = prop(&s, "ОбъявленияЭлементов");
        let get = |name: &str| {
            let d = list_lookup(&elements, &[BslValue::Str(BslString::from_str(name))]).unwrap();
            prop(&d, "Значение")
        };
        assert_eq!(
            get("ч"),
            BslValue::Number(bsl_number::BslNumber::from_i64(5))
        );
        assert_eq!(get("л"), BslValue::Boolean(true));
        assert_eq!(text_of(&get("д")), "2026-08-12");
    }

    #[test]
    fn simple_type_exposes_base_name_variety_and_facets() {
        let s = schema(SAMPLE);
        let types = prop(&s, "ОпределенияТипов");
        let code = list_lookup(&types, &[BslValue::Str(BslString::from_str("Код"))]).unwrap();
        let base = prop(&code, "ИмяБазовогоТипа");
        assert_eq!(text_of(&prop(&base, "ЛокальноеИмя")), "string");
        assert_eq!(text_of(&prop(&base, "URIПространстваИмен")), XSD_NS);
        // Разрешённое определение платформа на этом этапе не заполняет.
        assert_eq!(prop(&code, "ОпределениеБазовогоТипа"), BslValue::Undefined);
        assert_eq!(
            prop(&code, "Вариант"),
            BslValue::Enum(EnumValue::XsVarietyAtomic)
        );
        let facets = prop(&code, "Фасеты");
        assert_eq!(count(&facets), 3);
        let min = item(&facets, 0);
        assert_eq!(
            prop(&min, "ТипКомпоненты"),
            BslValue::Enum(EnumValue::XsCompMinLengthFacet)
        );
        assert_eq!(text_of(&prop(&min, "ЛексическоеЗначение")), "2");
        assert_eq!(prop(&min, "Фиксированный"), BslValue::Undefined);
        assert_eq!(
            prop(&item(&facets, 1), "Фиксированный"),
            BslValue::Boolean(true)
        );
        // Фасет — ребёнок своего типа, а схема у него общая.
        assert_eq!(prop(&min, "Контейнер"), code);
        assert_eq!(prop(&min, "Схема"), s);
    }

    /// У образца, перечисления и пробельных символов `Фиксированный` НЕТ —
    /// обращение к нему ошибка, а не `Неопределено` (измерено).
    #[test]
    fn only_numeric_facets_have_the_fixed_flag() {
        let s = schema(SAMPLE);
        let types = prop(&s, "ОпределенияТипов");
        let code = list_lookup(&types, &[BslValue::Str(BslString::from_str("Код"))]).unwrap();
        let pattern = item(&prop(&code, "Фасеты"), 2);
        assert_eq!(text_of(&prop(&pattern, "ЛексическоеЗначение")), "[А-Я]+");
        // Образец хранится СЫРОЙ строкой: движка регулярных выражений здесь
        // нет, и проверка по нему — работа отдельной задачи.
        assert_eq!(text_of(&prop(&pattern, "Значение")), "[А-Я]+");
        assert!(
            matches!(
                get_property(&pattern, "Фиксированный"),
                Err(RtError::UnknownColumn(_))
            ),
            "у фасета образца `Фиксированный` быть не должно"
        );
    }

    #[test]
    fn complex_type_exposes_content_particle_and_attribute_uses() {
        let s = schema(SAMPLE);
        let types = prop(&s, "ОпределенияТипов");
        let t = list_lookup(&types, &[BslValue::Str(BslString::from_str("Тип"))]).unwrap();
        assert_eq!(prop(&t, "ИмяБазовогоТипа"), BslValue::Undefined);
        assert_eq!(prop(&t, "МетодНаследования"), BslValue::Undefined);
        assert_eq!(prop(&t, "МодельСодержимого"), BslValue::Undefined);
        assert_eq!(prop(&t, "Смешанный"), BslValue::Undefined);

        let particle = prop(&t, "Содержимое");
        assert_eq!(
            prop(&particle, "ТипКомпоненты"),
            BslValue::Enum(EnumValue::XsCompParticle)
        );
        // У этого `<xs:sequence>` границы вхождения не написаны, поэтому
        // обе пусты (измерено, проба `час МинимальноВходит`). Что бывает
        // там, где они написаны, проверяет
        // `particle_occurs_bounds_repeat_what_is_written`.
        assert_eq!(prop(&particle, "МинимальноВходит"), BslValue::Undefined);
        assert_eq!(prop(&particle, "МаксимальноВходит"), BslValue::Undefined);
        assert_eq!(prop(&particle, "Контейнер"), t);

        let group = prop(&particle, "Часть");
        assert_eq!(
            prop(&group, "ВидГруппы"),
            BslValue::Enum(EnumValue::XsGroupSequence)
        );
        let particles = prop(&group, "Фрагменты");
        assert_eq!(count(&particles), 3);
        // Вложенный `xs:choice` — это фрагмент, чей терм снова группа.
        let nested = prop(&item(&particles, 2), "Часть");
        assert_eq!(
            prop(&nested, "ВидГруппы"),
            BslValue::Enum(EnumValue::XsGroupChoice)
        );

        let attributes = prop(&t, "Атрибуты");
        assert_eq!(count(&attributes), 1);
        let use_ = item(&attributes, 0);
        assert_eq!(prop(&use_, "Обязательный"), BslValue::Boolean(true));
        // `Использование` платформа не заполняет — обязательность отдаёт
        // `Обязательный`.
        assert_eq!(prop(&use_, "Использование"), BslValue::Undefined);
        let decl = prop(&use_, "ОбъявлениеАтрибута");
        assert_eq!(text_of(&prop(&decl, "Имя")), "ид");
        assert_eq!(prop(&decl, "Контейнер"), use_);
    }

    /// Анонимный тип живёт у объявления, а НЕ в `ОпределенияТипов`.
    #[test]
    fn anonymous_type_belongs_to_its_declaration() {
        let s = schema(SAMPLE);
        let types = prop(&s, "ОпределенияТипов");
        assert_eq!(count(&types), 2, "именованных типов ровно два");
        let t = list_lookup(&types, &[BslValue::Str(BslString::from_str("Тип"))]).unwrap();
        let group = prop(&prop(&t, "Содержимое"), "Часть");
        let nested_decl = prop(&item(&prop(&group, "Фрагменты"), 1), "Часть");
        assert_eq!(text_of(&prop(&nested_decl, "Имя")), "вложенный");
        assert_eq!(
            prop(&nested_decl, "ЭтоГлобальноеОбъявление"),
            BslValue::Boolean(false)
        );
        let anon = prop(&nested_decl, "АнонимноеОпределениеТипа");
        assert_eq!(
            text_of(&prop(&anon, "Имя")),
            "",
            "у анонимного типа имени нет"
        );
        assert_eq!(prop(&anon, "Контейнер"), nested_decl);
        assert_eq!(count(&prop(&nested_decl, "Компоненты")), 1);
        // Локальное объявление под `elementFormDefault="qualified"` берёт
        // целевое пространство имён, а `Форма` остаётся пустой (измерено).
        assert_eq!(text_of(&prop(&nested_decl, "URIПространстваИмен")), "urn:t");
        assert_eq!(prop(&nested_decl, "Форма"), BslValue::Undefined);
    }

    #[test]
    fn inheritance_keeps_the_lexical_base_name_and_method() {
        let s = schema(concat!(
            r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" xmlns:t="urn:t" "#,
            r#"targetNamespace="urn:t">"#,
            r#"<xs:complexType name="База"><xs:sequence>"#,
            r#"<xs:element name="а" type="xs:string"/></xs:sequence></xs:complexType>"#,
            r#"<xs:complexType name="Расширение"><xs:complexContent>"#,
            r#"<xs:extension base="t:База"><xs:sequence>"#,
            r#"<xs:element name="б" type="xs:string"/></xs:sequence>"#,
            r#"</xs:extension></xs:complexContent></xs:complexType>"#,
            r#"<xs:complexType name="Сужение"><xs:complexContent>"#,
            r#"<xs:restriction base="t:База"><xs:sequence>"#,
            r#"<xs:element name="а" type="xs:string"/></xs:sequence>"#,
            r#"</xs:restriction></xs:complexContent></xs:complexType>"#,
            r#"<xs:complexType name="Простое"><xs:simpleContent>"#,
            r#"<xs:extension base="xs:string">"#,
            r#"<xs:attribute name="у" type="xs:string"/>"#,
            r#"</xs:extension></xs:simpleContent></xs:complexType>"#,
            r#"</xs:schema>"#,
        ));
        let types = prop(&s, "ОпределенияТипов");
        let by =
            |name: &str| list_lookup(&types, &[BslValue::Str(BslString::from_str(name))]).unwrap();

        let ext = by("Расширение");
        assert_eq!(
            text_of(&prop(&prop(&ext, "ИмяБазовогоТипа"), "ЛокальноеИмя")),
            "База"
        );
        assert_eq!(
            prop(&ext, "МетодНаследования"),
            BslValue::Enum(EnumValue::XsDerivationExtension)
        );
        // Содержимое наследника — ТОЛЬКО его собственное: базовое платформа
        // сюда не вливает.
        let group = prop(&prop(&ext, "Содержимое"), "Часть");
        assert_eq!(count(&prop(&group, "Фрагменты")), 1);

        assert_eq!(
            prop(&by("Сужение"), "МетодНаследования"),
            BslValue::Enum(EnumValue::XsDerivationRestriction)
        );
        let simple = by("Простое");
        assert_eq!(
            text_of(&prop(&prop(&simple, "ИмяБазовогоТипа"), "ЛокальноеИмя")),
            "string"
        );
        assert_eq!(prop(&simple, "Содержимое"), BslValue::Undefined);
        assert_eq!(count(&prop(&simple, "Атрибуты")), 1);
    }

    #[test]
    fn list_and_union_keep_their_lexical_members() {
        let s = schema(concat!(
            r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" xmlns:t="urn:t" "#,
            r#"targetNamespace="urn:t">"#,
            r#"<xs:simpleType name="Код"><xs:restriction base="xs:string"/></xs:simpleType>"#,
            r#"<xs:simpleType name="Коды"><xs:list itemType="t:Код"/></xs:simpleType>"#,
            r#"<xs:simpleType name="Оба"><xs:union memberTypes="xs:string xs:int"/></xs:simpleType>"#,
            r#"</xs:schema>"#,
        ));
        let types = prop(&s, "ОпределенияТипов");
        let by =
            |name: &str| list_lookup(&types, &[BslValue::Str(BslString::from_str(name))]).unwrap();

        let list = by("Коды");
        assert_eq!(
            prop(&list, "Вариант"),
            BslValue::Enum(EnumValue::XsVarietyList)
        );
        assert_eq!(
            text_of(&prop(&prop(&list, "ИмяТипаЭлемента"), "ЛокальноеИмя")),
            "Код"
        );

        let union = by("Оба");
        assert_eq!(
            prop(&union, "Вариант"),
            BslValue::Enum(EnumValue::XsVarietyUnion)
        );
        let members = prop(&union, "ИменаТиповОбъединения");
        assert_eq!(count(&members), 2);
        // Разрешённых определений платформа не заполняет — список пуст.
        assert_eq!(count(&prop(&union, "ОпределенияТиповОбъединения")), 0);
    }

    /// `ref` — это ОТДЕЛЬНОЕ объявление с пустым именем: тождества с
    /// глобальным у него нет (измерено).
    #[test]
    fn references_are_separate_declarations() {
        let s = schema(concat!(
            r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" xmlns:t="urn:t" "#,
            r#"targetNamespace="urn:t">"#,
            r#"<xs:element name="а" type="xs:string"/>"#,
            r#"<xs:attribute name="б" type="xs:string"/>"#,
            r#"<xs:complexType name="Т"><xs:sequence>"#,
            r#"<xs:element ref="t:а"/></xs:sequence>"#,
            r#"<xs:attribute ref="t:б"/></xs:complexType>"#,
            r#"</xs:schema>"#,
        ));
        let types = prop(&s, "ОпределенияТипов");
        let t = list_lookup(&types, &[BslValue::Str(BslString::from_str("Т"))]).unwrap();
        let group = prop(&prop(&t, "Содержимое"), "Часть");
        let by_ref = prop(&item(&prop(&group, "Фрагменты"), 0), "Часть");
        assert_eq!(text_of(&prop(&by_ref, "Имя")), "");
        assert_eq!(prop(&by_ref, "ИмяТипа"), BslValue::Undefined);
        assert_eq!(
            prop(&by_ref, "ЭтоГлобальноеОбъявление"),
            BslValue::Boolean(false)
        );
        assert_eq!(
            text_of(&prop(&prop(&by_ref, "Ссылка"), "ЛокальноеИмя")),
            "а"
        );
        let global = list_lookup(
            &prop(&s, "ОбъявленияЭлементов"),
            &[BslValue::Str(BslString::from_str("а"))],
        )
        .unwrap();
        assert_ne!(by_ref, global, "ссылка не тождественна объявлению");

        let attr_decl = prop(&item(&prop(&t, "Атрибуты"), 0), "ОбъявлениеАтрибута");
        assert_eq!(text_of(&prop(&attr_decl, "Имя")), "");
        assert_eq!(
            text_of(&prop(&prop(&attr_decl, "Ссылка"), "ЛокальноеИмя")),
            "б"
        );
    }

    /// QName разрешается по объявлениям `xmlns` в области видимости, а
    /// неразрешимое имя даёт `Неопределено` — не «имя в пустом
    /// пространстве».
    #[test]
    fn unresolvable_qnames_become_undefined() {
        let s = schema(concat!(
            r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">"#,
            r#"<xs:element name="а" type="нетТакого"/>"#,
            r#"<xs:element name="б" type="нет:такого"/>"#,
            r#"</xs:schema>"#,
        ));
        let elements = prop(&s, "ОбъявленияЭлементов");
        for name in ["а", "б"] {
            let d = list_lookup(&elements, &[BslValue::Str(BslString::from_str(name))]).unwrap();
            assert_eq!(prop(&d, "ИмяТипа"), BslValue::Undefined);
        }
        // Префикс не важен: схему опознают по URI пространства имён.
        let other_prefix =
            schema(r#"<xs:schema xmlns:xs="urn:чужое"><xs:element name="а"/></xs:schema>"#);
        assert_eq!(
            other_prefix,
            BslValue::Undefined,
            "корень не из пространства XML Schema схемой не считается"
        );
    }

    #[test]
    fn broken_and_foreign_input_follows_the_platform() {
        // Корень не схема — `Неопределено`, а не ошибка.
        assert_eq!(schema("<а><б/></а>"), BslValue::Undefined);
        // Документ без корня — ошибка.
        let empty = create_schema(&new_builder(), &[BslValue::new_dom_document()]);
        assert!(matches!(empty, Err(RtError::Xsd(_))));
        // Не узел дерева и не один аргумент — тоже.
        assert!(matches!(
            create_schema(&new_builder(), &[BslValue::Undefined]),
            Err(RtError::Xsd(_))
        ));
        assert!(matches!(
            create_schema(&new_builder(), &[]),
            Err(RtError::MethodNotApplicable { .. })
        ));
        // Незнакомый элемент схемы и чужое пространство имён — пропускаются.
        let s = schema(concat!(
            r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" xmlns:ч="urn:ч">"#,
            r#"<xs:чушь name="а"/><ч:своё/>текст<!--к-->"#,
            r#"<xs:element name="б" type="xs:string"/>"#,
            r#"<xs:element type="xs:string"/>"#,
            r#"<xs:element name="б" type="xs:int"/>"#,
            r#"</xs:schema>"#,
        ));
        let elements = prop(&s, "ОбъявленияЭлементов");
        assert_eq!(
            count(&elements),
            1,
            "дубль имени и элемент без имени выпали"
        );
        // Из двух одноимённых остаётся ПЕРВЫЙ.
        let kept = item(&elements, 0);
        assert_eq!(
            text_of(&prop(&prop(&kept, "ИмяТипа"), "ЛокальноеИмя")),
            "string"
        );
        assert_eq!(count(&prop(&s, "Компоненты")), 2, "оба дубля — компоненты");
    }

    /// Конструкции за границей модели — честная ошибка с именем
    /// конструкции, а не молчаливый пропуск.
    #[test]
    fn constructs_outside_the_model_are_named_errors() {
        let cases = [
            (
                r#"<xs:group name="г"><xs:sequence/></xs:group>"#,
                "xs:group",
            ),
            (r#"<xs:attributeGroup name="г"/>"#, "xs:attributeGroup"),
            (r#"<xs:import namespace="urn:ч"/>"#, "xs:import"),
            (r#"<xs:include schemaLocation="а.xsd"/>"#, "xs:include"),
            (r#"<xs:redefine schemaLocation="а.xsd"/>"#, "xs:redefine"),
            (r#"<xs:notation name="н" public="п"/>"#, "xs:notation"),
            (
                r#"<xs:complexType name="т"><xs:sequence><xs:any/></xs:sequence></xs:complexType>"#,
                "xs:any",
            ),
            (
                r#"<xs:complexType name="т"><xs:anyAttribute/></xs:complexType>"#,
                "xs:anyAttribute",
            ),
            (
                r#"<xs:complexType name="т"><xs:sequence><xs:group ref="t:г"/></xs:sequence></xs:complexType>"#,
                "xs:group",
            ),
            (
                r#"<xs:element name="а"><xs:unique name="у"><xs:selector xpath="."/></xs:unique></xs:element>"#,
                "xs:unique",
            ),
        ];
        for (body, expected) in cases {
            let text = format!(
                r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" xmlns:t="urn:t">{body}</xs:schema>"#
            );
            match schema_of(&text) {
                Err(RtError::Xsd(msg)) => assert!(
                    msg.contains(expected),
                    "в тексте ошибки нет «{expected}»: {msg}"
                ),
                other => panic!("ожидалась ошибка про «{expected}», получено {other:?}"),
            }
        }
    }

    #[test]
    fn empty_schema_has_no_components_and_no_dom() {
        let s = new_schema();
        assert_eq!(text_of(&prop(&s, "ПространствоИмен")), "");
        assert_eq!(text_of(&prop(&s, "Версия")), "");
        assert_eq!(count(&prop(&s, "Компоненты")), 0);
        assert_eq!(count(&prop(&s, "ОбъявленияЭлементов")), 0);
        assert_eq!(prop(&s, "ЭлементDOM"), BslValue::Undefined);
        assert_eq!(prop(&s, "Схема"), s);
    }

    /// Набор схем: та же схема повторно проходит молча, ДРУГАЯ схема того
    /// же пространства имён — ошибка (измерено обе ветки).
    #[test]
    fn schema_set_holds_one_schema_per_namespace() {
        let set = new_schema_set();
        let first = schema(SAMPLE);
        schema_set_add(&set, std::slice::from_ref(&first)).expect("первая схема");
        schema_set_add(&set, std::slice::from_ref(&first)).expect("та же схема повторно");
        assert_eq!(set.collection_len().unwrap(), 1);
        assert_eq!(schema_set_get(&set, 0).unwrap(), first);

        let same_ns = schema(SAMPLE);
        assert!(
            matches!(schema_set_add(&set, &[same_ns]), Err(RtError::Xsd(_))),
            "вторая схема того же пространства имён — ошибка"
        );
        let other = schema(concat!(
            r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" targetNamespace="urn:другое">"#,
            r#"<xs:element name="щ" type="xs:string"/></xs:schema>"#,
        ));
        schema_set_add(&set, &[other]).expect("другое пространство имён");
        assert_eq!(set.collection_len().unwrap(), 2);
        assert!(matches!(
            schema_set_add(&set, &[BslValue::Undefined]),
            Err(RtError::Xsd(_))
        ));
        assert!(matches!(
            schema_set_get(&set, 9),
            Err(RtError::IndexOutOfBounds { .. })
        ));
    }

    #[test]
    fn expanded_name_is_a_value_not_a_reference() {
        let a = new_expanded_name("urn:t", "а");
        let b = new_expanded_name("urn:t", "а");
        assert_eq!(a, b, "равенство по содержимому");
        assert_ne!(a, new_expanded_name("urn:t", "б"));
        assert_eq!(a.to_string(), "{urn:t}а");
        // Пустой URI печатается ОДНИМ локальным именем, без скобок.
        assert_eq!(new_expanded_name("", "а").to_string(), "а");
    }

    /// Аннотация — обычная компонента: у неё есть документация и
    /// информация приложения, а имени нет.
    #[test]
    fn annotation_carries_documentation_and_appinfo() {
        let s = schema(concat!(
            r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">"#,
            r#"<xs:annotation><xs:documentation xml:lang="ru">про схему</xs:documentation>"#,
            r#"<xs:appinfo source="urn:и">данные</xs:appinfo></xs:annotation>"#,
            r#"</xs:schema>"#,
        ));
        let annotation = prop(&s, "Аннотация");
        assert_eq!(
            prop(&annotation, "ТипКомпоненты"),
            BslValue::Enum(EnumValue::XsCompAnnotation)
        );
        assert_eq!(count(&prop(&annotation, "Компоненты")), 2);
        let doc = item(&prop(&annotation, "Документация"), 0);
        assert_eq!(text_of(&prop(&doc, "Язык")), "ru");
        assert_eq!(text_of(&prop(&doc, "Источник")), "");
        assert_eq!(prop(&doc, "Контейнер"), annotation);
        let appinfo = item(&prop(&annotation, "ИнформацияДляПриложения"), 0);
        assert_eq!(text_of(&prop(&appinfo, "Источник")), "urn:и");
        assert_eq!(
            prop(&appinfo, "ТипКомпоненты"),
            BslValue::Enum(EnumValue::XsCompAppInfo)
        );
        // Имени у аннотации нет вовсе.
        assert!(matches!(
            get_property(&annotation, "Имя"),
            Err(RtError::UnknownColumn(_))
        ));
    }

    #[test]
    fn indexing_past_the_end_is_an_error() {
        let s = schema(SAMPLE);
        let elements = prop(&s, "ОбъявленияЭлементов");
        let (schema_rc, kind) = match &elements {
            BslValue::Object(o) => match &**o {
                BslObject::XsList(s, k) => (s.clone(), k.clone()),
                _ => panic!("ожидалась коллекция"),
            },
            _ => panic!("ожидалась коллекция"),
        };
        assert!(matches!(
            list_get(&schema_rc, &kind, 9),
            Err(RtError::IndexOutOfBounds { .. })
        ));
    }
}
