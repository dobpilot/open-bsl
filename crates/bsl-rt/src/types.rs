//! Реестр типов: объектное представление типа (`ТипЗнч`/`Тип`).
//!
//! Имена типов в 1С ЛОКАЛИЗОВАНЫ — `Строка(Новый Массив)` даёт `Массив`, а
//! `Строка(ТипЗнч(Новый Массив))` тоже `Массив`, — поэтому у каждого типа
//! пара имён (ru/en), ровно как у ключевых слов в `bsl-syntax::keywords`.
//! Поиск по имени регистронезависим и принимает оба языка; ОБРАТНО
//! (`Строка(ТипЗнч(...))`) отдаётся всегда русское — это то, что видит
//! пользователь платформы.

/// Тип как ЗНАЧЕНИЕ (`BslValue::Type`), а не как тег `BslValue`. Список
/// намеренно совпадает с набором вариантов `BslValue`/`BslObject`: пока в
/// языке нет пользовательских типов и типов из конфигурации, множество
/// типов конечно и известно на этапе компиляции этого крейта.
///
/// `Copy` и один байт — `BslValue::Type` не должен раздувать `BslValue`
/// (см. инвариант про размер значения).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TypeId {
    Undefined,
    Null,
    Boolean,
    Number,
    String,
    Date,
    Array,
    Structure,
    Map,
    ValueTable,
    ValueTableColumns,
    ValueTableRow,
    KeyAndValue,
    /// Тип самого типа: `ТипЗнч(Тип("Число"))` -> `Тип`.
    Type,

    // --- JSON ---------------------------------------------------------
    // Имена ТИПОВ у этих шести — с пробелами («Чтение JSON»), а имена
    // ЗНАЧЕНИЙ — без («ЧтениеJSON»). Это не описка, а та же пара, что у
    // `Неопределено`/`Не определено` выше: значение и его тип печатаются
    // по-разному. Измерено на 8.3.27.
    JsonReader,
    JsonWriter,
    JsonWriterSettings,
    /// Перечисления платформы. Тип члена — само перечисление.
    JsonValueType,
    JsonLineBreak,
    JsonDateFormat,

    // --- XML ----------------------------------------------------------
    // Та же пара написаний, что и у JSON: тип печатается с пробелом
    // («Чтение XML»), значение — без («ЧтениеXML»). Измерено на 8.3.27.
    XmlReader,
    XmlWriter,
    XmlWriterSettings,
    XmlNodeType,
}

/// `(русское, английское)`. Русское — каноническое: именно оно уходит в
/// `Строка(ТипЗнч(...))`.
const NAMES: &[(TypeId, &str, &str)] = &[
    // ИЗМЕРЕНО на 8.3.27: `Строка(ТипЗнч(Неопределено))` даёт «Не
    // определено» — С ПРОБЕЛОМ, в отличие от одноимённого литерала языка.
    (TypeId::Undefined, "Не определено", "Undefined"),
    // У `Null` английское и русское написание совпадают — как и в самом
    // языке (`NULL` есть в таблице ключевых слов только одним написанием).
    (TypeId::Null, "Null", "Null"),
    (TypeId::Boolean, "Булево", "Boolean"),
    (TypeId::Number, "Число", "Number"),
    (TypeId::String, "Строка", "String"),
    (TypeId::Date, "Дата", "Date"),
    (TypeId::Array, "Массив", "Array"),
    (TypeId::Structure, "Структура", "Structure"),
    (TypeId::Map, "Соответствие", "Map"),
    (TypeId::ValueTable, "ТаблицаЗначений", "ValueTable"),
    (
        TypeId::ValueTableColumns,
        "КоллекцияКолонокТаблицыЗначений",
        "ValueTableColumnCollection",
    ),
    (TypeId::ValueTableRow, "СтрокаТаблицыЗначений", "ValueTableRow"),
    (TypeId::KeyAndValue, "КлючИЗначение", "KeyAndValue"),
    (TypeId::Type, "Тип", "Type"),
    (TypeId::JsonReader, "Чтение JSON", "JSONReader"),
    (TypeId::JsonWriter, "Запись JSON", "JSONWriter"),
    (
        TypeId::JsonWriterSettings,
        "Параметры записи JSON",
        "JSONWriterSettings",
    ),
    (TypeId::JsonValueType, "ТипЗначенияJSON", "JSONValueType"),
    (TypeId::JsonLineBreak, "ПереносСтрокJSON", "JSONLineBreak"),
    (TypeId::JsonDateFormat, "ФорматДатыJSON", "JSONDateFormat"),
    (TypeId::XmlReader, "Чтение XML", "XMLReader"),
    (TypeId::XmlWriter, "Запись XML", "XMLWriter"),
    (
        TypeId::XmlWriterSettings,
        "Параметры записи XML",
        "XMLWriterSettings",
    ),
    (TypeId::XmlNodeType, "ТипУзлаXML", "XMLNodeType"),
];

impl TypeId {
    /// Каноническое (русское) имя типа — то, что печатает `Строка()`.
    pub fn name(self) -> &'static str {
        NAMES
            .iter()
            .find(|(id, _, _)| *id == self)
            .map(|(_, ru, _)| *ru)
            .unwrap_or("Неопределено")
    }

    /// `Тип("ИмяТипа")` — разбор имени в тип. Принимает оба языка,
    /// регистронезависимо. `None` — такого типа нет (в 1С это исключение,
    /// решает вызывающий, а не эта таблица).
    pub fn lookup(name: &str) -> Option<TypeId> {
        // Пробелы внутри имени не значимы: печатается «Не определено», а
        // пишут в коде обычно `Тип("Неопределено")` — одним словом, как
        // литерал языка. Обе формы обязаны находиться.
        let squash = |s: &str| s.to_uppercase().replace(' ', "");
        let key = squash(name.trim());
        NAMES
            .iter()
            .find(|(_, ru, en)| squash(ru) == key || squash(en) == key)
            .map(|(id, _, _)| *id)
    }
}

impl std::fmt::Display for TypeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup_accepts_both_languages_case_insensitively() {
        assert_eq!(TypeId::lookup("Массив"), Some(TypeId::Array));
        assert_eq!(TypeId::lookup("массив"), Some(TypeId::Array));
        assert_eq!(TypeId::lookup("ARRAY"), Some(TypeId::Array));
        assert_eq!(TypeId::lookup("ТаблицаЗначений"), Some(TypeId::ValueTable));
        assert_eq!(TypeId::lookup("НетТакогоТипа"), None);
    }

    #[test]
    fn canonical_name_is_the_russian_one() {
        // Строка(ТипЗнч(Новый Массив)) должно совпасть со Строка(Новый Массив).
        assert_eq!(TypeId::Array.name(), "Массив");
        assert_eq!(TypeId::Number.name(), "Число");
        assert_eq!(TypeId::Type.name(), "Тип");
    }

    #[test]
    fn every_type_round_trips_through_its_own_names() {
        for (id, ru, en) in NAMES {
            assert_eq!(TypeId::lookup(ru), Some(*id), "русское имя {ru}");
            assert_eq!(TypeId::lookup(en), Some(*id), "английское имя {en}");
            assert_eq!(id.name(), *ru);
        }
    }
}
