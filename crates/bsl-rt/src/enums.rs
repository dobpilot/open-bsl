//! Члены платформенных перечислений как ЗНАЧЕНИЯ (`ТипЗначенияJSON.Строка`).
//!
//! Устроено как `types.rs`: `Copy`-тег в один байт плюс статическая таблица
//! написаний, — и по той же причине. Множество членов конечно и известно на
//! этапе компиляции этого крейта, поэтому `BslValue::Enum` не растит размер
//! значения.
//!
//! Обращение `ИмяПеречисления.Член` резолвится в КОНСТАНТУ на этапе
//! компиляции (см. `bsl-sema::resolver`), а не в чтение поля объекта: у
//! платформы перечисление — не объект с полями, и опечатка в имени члена
//! там тоже ошибка компиляции, а не рантайма (проверено: `Вычислить` с
//! несуществующим членом отвечает «Поле объекта не обнаружено», то есть
//! падает уже при компиляции фрагмента).
//!
//! Строковые представления ИЗМЕРЕНЫ на 8.3.27 и местами неожиданны: у
//! `КонецМассива` и `ИмяСвойства` в конце ЗНАЧАЩИЙ ПРОБЕЛ, а `Ничего`
//! печатается как «Нет». Не опечатка — так печатает платформа.

/// Член платформенного перечисления.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum EnumValue {
    // --- ТипЗначенияJSON ---------------------------------------------
    JsonString,
    JsonNumber,
    JsonBoolean,
    JsonNull,
    JsonObjectStart,
    JsonObjectEnd,
    JsonArrayStart,
    JsonArrayEnd,
    JsonPropertyName,
    JsonNothing,
    JsonComment,

    // --- ПереносСтрокJSON ---------------------------------------------
    LineBreakNone,
    LineBreakAuto,
    LineBreakWindows,
    LineBreakUnix,

    // --- ФорматДатыJSON -----------------------------------------------
    DateFormatIso,
    DateFormatJavaScript,
    DateFormatMicrosoft,

    // --- ТипУзлаXML ----------------------------------------------------
    // Состав ВЫЯСНЕН перебором на 8.3.27, а не взят из документации:
    // `НачалоСущности`, `Пробелы` и `ЗначащиеПробелы` платформа не знает,
    // хотя `КонецСущности` у неё есть. Отсюда и несимметричный список.
    XmlNothing,
    XmlElementStart,
    XmlElementEnd,
    XmlText,
    XmlCdataSection,
    XmlComment,
    XmlProcessingInstruction,
    XmlDeclaration,
    XmlAttribute,
    XmlEntityEnd,
    XmlEntityReference,
    XmlDocumentType,
    XmlNotation,
}

/// К какому перечислению принадлежит член — это же имя стоит слева от
/// точки в исходном тексте.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnumKind {
    JsonValueType,
    JsonLineBreak,
    JsonDateFormat,
    XmlNodeType,
}

impl EnumKind {
    /// Написания самого перечисления: `(русское, английское)`.
    const fn names(self) -> (&'static str, &'static str) {
        match self {
            EnumKind::JsonValueType => ("ТипЗначенияJSON", "JSONValueType"),
            EnumKind::JsonLineBreak => ("ПереносСтрокJSON", "JSONLineBreak"),
            EnumKind::JsonDateFormat => ("ФорматДатыJSON", "JSONDateFormat"),
            EnumKind::XmlNodeType => ("ТипУзлаXML", "XMLNodeType"),
        }
    }
}

/// `(перечисление, член, русское имя члена, английское имя члена, что
/// печатает `Строка()`)`.
///
/// Пятая колонка — не производная от третьей: платформа печатает члены
/// ЧЕЛОВЕЧЕСКИМ текстом («Начало объекта»), а не идентификатором
/// («НачалоОбъекта»). Всё измерено, включая хвостовые пробелы.
const MEMBERS: &[(EnumKind, EnumValue, &str, &str, &str)] = &[
    (
        EnumKind::JsonValueType,
        EnumValue::JsonString,
        "Строка",
        "String",
        "Строка",
    ),
    (
        EnumKind::JsonValueType,
        EnumValue::JsonNumber,
        "Число",
        "Number",
        "Число",
    ),
    (
        EnumKind::JsonValueType,
        EnumValue::JsonBoolean,
        "Булево",
        "Boolean",
        "Булево",
    ),
    (
        EnumKind::JsonValueType,
        EnumValue::JsonNull,
        "Null",
        "Null",
        "Значение Null",
    ),
    (
        EnumKind::JsonValueType,
        EnumValue::JsonObjectStart,
        "НачалоОбъекта",
        "ObjectStart",
        "Начало объекта",
    ),
    (
        EnumKind::JsonValueType,
        EnumValue::JsonObjectEnd,
        "КонецОбъекта",
        "ObjectEnd",
        "Конец объекта",
    ),
    (
        EnumKind::JsonValueType,
        EnumValue::JsonArrayStart,
        "НачалоМассива",
        "ArrayStart",
        "Начало массива",
    ),
    // ХВОСТОВОЙ ПРОБЕЛ — измерено, не описка.
    (
        EnumKind::JsonValueType,
        EnumValue::JsonArrayEnd,
        "КонецМассива",
        "ArrayEnd",
        "Конец массива ",
    ),
    // И здесь тоже.
    (
        EnumKind::JsonValueType,
        EnumValue::JsonPropertyName,
        "ИмяСвойства",
        "PropertyName",
        "Имя свойства ",
    ),
    (
        EnumKind::JsonValueType,
        EnumValue::JsonNothing,
        "Ничего",
        "None",
        "Нет",
    ),
    (
        EnumKind::JsonValueType,
        EnumValue::JsonComment,
        "Комментарий",
        "Comment",
        "Комментарий",
    ),
    (
        EnumKind::JsonLineBreak,
        EnumValue::LineBreakNone,
        "Нет",
        "None",
        "Нет",
    ),
    (
        EnumKind::JsonLineBreak,
        EnumValue::LineBreakAuto,
        "Авто",
        "Auto",
        "Автоматически",
    ),
    (
        EnumKind::JsonLineBreak,
        EnumValue::LineBreakWindows,
        "Windows",
        "Windows",
        "Windows",
    ),
    (
        EnumKind::JsonLineBreak,
        EnumValue::LineBreakUnix,
        "Unix",
        "Unix",
        "Unix",
    ),
    (
        EnumKind::JsonDateFormat,
        EnumValue::DateFormatIso,
        "ISO",
        "ISO",
        "ISO",
    ),
    (
        EnumKind::JsonDateFormat,
        EnumValue::DateFormatJavaScript,
        "JavaScript",
        "JavaScript",
        "JavaScript",
    ),
    // Пятая колонка снова не производная от третьей: платформа печатает
    // «Начало элемента», а не «НачалоЭлемента». Всё измерено.
    (
        EnumKind::XmlNodeType,
        EnumValue::XmlNothing,
        "Ничего",
        "None",
        "Ничего",
    ),
    (
        EnumKind::XmlNodeType,
        EnumValue::XmlElementStart,
        "НачалоЭлемента",
        "StartElement",
        "Начало элемента",
    ),
    (
        EnumKind::XmlNodeType,
        EnumValue::XmlElementEnd,
        "КонецЭлемента",
        "EndElement",
        "Конец элемента",
    ),
    (
        EnumKind::XmlNodeType,
        EnumValue::XmlText,
        "Текст",
        "Text",
        "Текст",
    ),
    (
        EnumKind::XmlNodeType,
        EnumValue::XmlCdataSection,
        "СекцияCDATA",
        "CDATASection",
        "Секция CDATA",
    ),
    (
        EnumKind::XmlNodeType,
        EnumValue::XmlComment,
        "Комментарий",
        "Comment",
        "Комментарий",
    ),
    (
        EnumKind::XmlNodeType,
        EnumValue::XmlProcessingInstruction,
        "ИнструкцияОбработки",
        "ProcessingInstruction",
        "Инструкция обработки",
    ),
    (
        EnumKind::XmlNodeType,
        EnumValue::XmlDeclaration,
        "ОбъявлениеXML",
        "XMLDeclaration",
        "Объявление XML",
    ),
    (
        EnumKind::XmlNodeType,
        EnumValue::XmlAttribute,
        "Атрибут",
        "Attribute",
        "Атрибут",
    ),
    (
        EnumKind::XmlNodeType,
        EnumValue::XmlEntityEnd,
        "КонецСущности",
        "EndEntity",
        "Конец сущности",
    ),
    (
        EnumKind::XmlNodeType,
        EnumValue::XmlEntityReference,
        "СсылкаНаСущность",
        "EntityReference",
        "Ссылка на сущность",
    ),
    (
        EnumKind::XmlNodeType,
        EnumValue::XmlDocumentType,
        "ОпределениеТипаДокумента",
        "DocumentType",
        "Определение типа документа",
    ),
    (
        EnumKind::XmlNodeType,
        EnumValue::XmlNotation,
        "Нотация",
        "Notation",
        "Нотация",
    ),
    (
        EnumKind::JsonDateFormat,
        EnumValue::DateFormatMicrosoft,
        "Microsoft",
        "Microsoft",
        "Microsoft",
    ),
];

/// Имена всех перечислений — для автодополнения REPL и для резолвера,
/// который по левой части точки решает, перечисление это или переменная.
pub const ENUM_NAMES: &[(&str, EnumKind)] = &[
    ("ТипЗначенияJSON", EnumKind::JsonValueType),
    ("JSONValueType", EnumKind::JsonValueType),
    ("ПереносСтрокJSON", EnumKind::JsonLineBreak),
    ("JSONLineBreak", EnumKind::JsonLineBreak),
    ("ФорматДатыJSON", EnumKind::JsonDateFormat),
    ("JSONDateFormat", EnumKind::JsonDateFormat),
    ("ТипУзлаXML", EnumKind::XmlNodeType),
    ("XMLNodeType", EnumKind::XmlNodeType),
];

/// Перечисление по имени слева от точки. Регистронезависимо и на обоих
/// языках, как и всё остальное в языке.
pub fn lookup_enum(name: &str) -> Option<EnumKind> {
    let upper = name.to_uppercase();
    ENUM_NAMES
        .iter()
        .find(|(n, _)| n.to_uppercase() == upper)
        .map(|(_, k)| *k)
}

/// Член перечисления по имени справа от точки.
pub fn lookup_member(kind: EnumKind, member: &str) -> Option<EnumValue> {
    let upper = member.to_uppercase();
    MEMBERS
        .iter()
        .find(|(k, _, ru, en, _)| {
            *k == kind && (ru.to_uppercase() == upper || en.to_uppercase() == upper)
        })
        .map(|(_, v, ..)| *v)
}

/// Все члены перечисления — для автодополнения после точки.
pub fn members_of(kind: EnumKind) -> impl Iterator<Item = &'static str> {
    MEMBERS
        .iter()
        .filter(move |(k, ..)| *k == kind)
        .map(|(_, _, ru, ..)| *ru)
}

impl EnumValue {
    /// Что печатает `Строка()`. Измерено на платформе.
    pub fn display_text(self) -> &'static str {
        MEMBERS
            .iter()
            .find(|(_, v, ..)| *v == self)
            .map(|(.., text)| *text)
            .unwrap_or("?")
    }

    /// ИДЕНТИФИКАТОР члена (`НачалоОбъекта`), в отличие от
    /// [`display_text`](Self::display_text) («Начало объекта»). Нужен
    /// текстовому формату байт-кода: печатать туда человеческий текст
    /// нельзя — его не разобрать обратно.
    pub fn member_name(self) -> &'static str {
        MEMBERS
            .iter()
            .find(|(_, v, ..)| *v == self)
            .map(|(_, _, ru, ..)| *ru)
            .unwrap_or("?")
    }

    /// К какому перечислению принадлежит член.
    pub fn kind(self) -> EnumKind {
        MEMBERS
            .iter()
            .find(|(_, v, ..)| *v == self)
            .map(|(k, ..)| *k)
            .expect("член обязан быть в таблице")
    }

    /// Имя ЗНАЧЕНИЯ для `type_name` — имя самого перечисления без
    /// пробелов, в отличие от имени ТИПА в `types.rs`.
    pub fn enum_name(self) -> &'static str {
        MEMBERS
            .iter()
            .find(|(_, v, ..)| *v == self)
            .map(|(k, ..)| k.names().0)
            .unwrap_or("?")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_member_resolves_from_both_languages() {
        for (kind, value, ru, en, _) in MEMBERS {
            assert_eq!(lookup_member(*kind, ru), Some(*value), "{ru}");
            assert_eq!(lookup_member(*kind, en), Some(*value), "{en}");
            assert_eq!(
                lookup_member(*kind, &ru.to_uppercase()),
                Some(*value),
                "регистр не значим"
            );
        }
    }

    #[test]
    fn enum_names_resolve_case_insensitively() {
        assert_eq!(
            lookup_enum("типзначенияjson"),
            Some(EnumKind::JsonValueType)
        );
        assert_eq!(lookup_enum("JSONLineBreak"), Some(EnumKind::JsonLineBreak));
        assert_eq!(lookup_enum("НетТакого"), None);
    }

    /// Хвостовые пробелы — измеренная особенность платформы, и она обязана
    /// пережить любую «уборку» таблицы.
    #[test]
    fn measured_display_text_keeps_its_trailing_spaces() {
        assert_eq!(EnumValue::JsonArrayEnd.display_text(), "Конец массива ");
        assert_eq!(EnumValue::JsonPropertyName.display_text(), "Имя свойства ");
        assert_eq!(EnumValue::JsonNothing.display_text(), "Нет");
        assert_eq!(EnumValue::JsonNull.display_text(), "Значение Null");
        assert_eq!(EnumValue::LineBreakAuto.display_text(), "Автоматически");
    }

    #[test]
    fn a_member_of_one_enum_is_not_found_in_another() {
        // `Нет` есть у ПереносСтрокJSON, но не у ТипЗначенияJSON.
        assert!(lookup_member(EnumKind::JsonLineBreak, "Нет").is_some());
        assert!(lookup_member(EnumKind::JsonValueType, "Нет").is_none());
    }
}
