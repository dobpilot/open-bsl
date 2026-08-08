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
    ValueTableColumn,
    ValueTableRow,
    TypeDescription,
    ValueComparison,
    KeyAndValue,
    /// Непрозрачное значение внутреннего формата — служебный тип этой
    /// реализации (см. `BslObject::VstrOpaque`); в 1С не существует.
    VstrOpaque,
    /// Тип самого типа: `ТипЗнч(Тип("Число"))` -> `Тип`.
    Type,
    /// Метатип системного перечисления — `ТипЗнч(ВариантЗаписиДатыJSON)`
    /// от ГОЛОГО имени перечисления. Единственный несущий вариант: имя
    /// зависит от перечисления («ПеречислениеВариантЗаписиДатыJSON» —
    /// измерено), а плодить по варианту на каждое перечисление значило бы
    /// дублировать список `EnumKind`. В `NAMES` его нет — имя отдаёт
    /// `EnumKind::meta_ru_name`, а `Тип("Перечисление...")` такой тип НЕ
    /// находит: разрешимость этого имени через `Тип()` не измерена, и до
    /// замера честнее ошибка, чем угаданный успех.
    EnumMeta(crate::EnumKind),

    // --- JSON ---------------------------------------------------------
    // Имена ТИПОВ у этих шести — с пробелами («Чтение JSON»), а имена
    // ЗНАЧЕНИЙ — без («ЧтениеJSON»). Это не описка, а та же пара, что у
    // `Неопределено`/`Не определено` выше: значение и его тип печатаются
    // по-разному. Измерено на 8.3.27.
    JsonReader,
    JsonWriter,
    JsonWriterSettings,
    /// `НастройкиСериализацииJSON` — в отличие от `JsonWriterSettings`
    /// (форматирование текста: переносы строк, отступ) управляет тем, КАК
    /// сериализуются даты и массивы внутри `ЗаписатьJSON`.
    JsonSerializerSettings,
    /// Перечисления платформы. Тип члена — само перечисление.
    JsonValueType,
    JsonLineBreak,
    JsonDateFormat,
    JsonDateWritingVariant,

    // --- XML ----------------------------------------------------------
    // Та же пара написаний, что и у JSON: тип печатается с пробелом
    // («Чтение XML»), значение — без («ЧтениеXML»). Измерено на 8.3.27.
    XmlReader,
    XmlWriter,
    XmlWriterSettings,
    XmlNodeType,

    // --- ТабличныйДокумент ---------------------------------------------
    // Та же пара написаний, что у остальных: тип печатается с пробелом
    // («Табличный документ»), значение — без. Измерено на 8.3.27.
    SpreadDocument,
    SpreadArea,
    SpreadDrawings,
    SpreadDrawing,
    SpreadFileType,
    DrawingKind,

    // --- ТекстовыйДокумент ---------------------------------------------
    // Та же пара написаний: тип с пробелом («Текстовый документ»),
    // значение без. Имя типа параметров — «Параметры макета текстового
    // документа», измерено.
    TextDocument,
    TextDocParams,
    TextEncoding,

    // --- ДвоичныеДанные -------------------------------------------------
    // Та же пара написаний: тип печатается с пробелом («Двоичные данные»,
    // измерено пробой `BIN.TYPE`), а само ЗНАЧЕНИЕ не печатается именем
    // вовсе — оно отдаёт шестнадцатеричный дамп (см. `impl Display for
    // BslValue`).
    BinaryData,
    /// `БуферДвоичныхДанных`. Имя ТИПА с пробелами («Буфер двоичных
    /// данных»), имя ЗНАЧЕНИЯ слитно — измерено, как у соседей.
    BinaryDataBuffer,
    /// Тип ЧЛЕНА перечисления `ПорядокБайтов`. Измерено, что
    /// `ТипЗнч(ПорядокБайтов.LittleEndian)` печатается «ПорядокБайтов» —
    /// БЕЗ пробелов и без префикса `Перечисление`, который носит метатип
    /// голого имени (`TypeId::EnumMeta`).
    ByteOrder,

    // --- Потоки ---------------------------------------------------------
    /// `ПотокВПамяти` и `ФайловыйПоток` — РАЗНЫЕ типы
    /// (`Тип("ПотокВПамяти") = Тип("ФайловыйПоток")` даёт «Нет»,
    /// измерено), но печатаются они ОДИНАКОВО — «Файловый поток»; см.
    /// таблицу `DISPLAY` в этом же модуле.
    MemoryStream,
    FileStream,
    /// Тип менеджера `ФайловыеПотоки`. Имя ТИПА с пробелами («Менеджер
    /// файловых потоков»), имя ЗНАЧЕНИЯ слитно — измерено, как у соседей.
    FileStreamsManager,
    /// Типы ЧЛЕНОВ трёх перечислений потоков — как у `ByteOrder`, без
    /// пробелов и без префикса `Перечисление`.
    FileOpenMode,
    FileAccess,
    StreamPosition,
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
    (
        TypeId::ValueTableColumn,
        "КолонкаТаблицыЗначений",
        "ValueTableColumn",
    ),
    (
        TypeId::ValueTableRow,
        "СтрокаТаблицыЗначений",
        "ValueTableRow",
    ),
    (TypeId::TypeDescription, "ОписаниеТипов", "TypeDescription"),
    (
        TypeId::ValueComparison,
        "СравнениеЗначений",
        "ValueComparison",
    ),
    (TypeId::VstrOpaque, "НепрозрачноеЗначение", "OpaqueValue"),
    (TypeId::KeyAndValue, "КлючИЗначение", "KeyAndValue"),
    (TypeId::Type, "Тип", "Type"),
    (TypeId::JsonReader, "Чтение JSON", "JSONReader"),
    (TypeId::JsonWriter, "Запись JSON", "JSONWriter"),
    (
        TypeId::JsonWriterSettings,
        "Параметры записи JSON",
        "JSONWriterSettings",
    ),
    // Тип с пробелом, как и остальные объектные типы JSON выше; значение
    // («Строка(Новый НастройкиСериализацииJSON)») — без, см. `lib.rs`.
    (
        TypeId::JsonSerializerSettings,
        "Настройки сериализации JSON",
        "JSONSerializerSettings",
    ),
    (TypeId::JsonValueType, "ТипЗначенияJSON", "JSONValueType"),
    (TypeId::JsonLineBreak, "ПереносСтрокJSON", "JSONLineBreak"),
    (TypeId::JsonDateFormat, "ФорматДатыJSON", "JSONDateFormat"),
    (
        TypeId::JsonDateWritingVariant,
        "ВариантЗаписиДатыJSON",
        "JSONDateWritingVariant",
    ),
    (TypeId::XmlReader, "Чтение XML", "XMLReader"),
    (TypeId::XmlWriter, "Запись XML", "XMLWriter"),
    (
        TypeId::XmlWriterSettings,
        "Параметры записи XML",
        "XMLWriterSettings",
    ),
    (TypeId::XmlNodeType, "ТипУзлаXML", "XMLNodeType"),
    (
        TypeId::SpreadFileType,
        "ТипФайлаТабличногоДокумента",
        "SpreadsheetDocumentFileType",
    ),
    (
        TypeId::DrawingKind,
        "ТипРисункаТабличногоДокумента",
        "SpreadsheetDocumentDrawingType",
    ),
    (
        TypeId::SpreadDocument,
        "Табличный документ",
        "SpreadsheetDocument",
    ),
    (
        TypeId::SpreadArea,
        "Область ячеек табличного документа",
        "SpreadsheetDocumentRange",
    ),
    (
        TypeId::SpreadDrawings,
        "Коллекция рисунков табличного документа",
        "SpreadsheetDocumentDrawingCollection",
    ),
    (
        TypeId::SpreadDrawing,
        "Рисунок табличного документа",
        "SpreadsheetDocumentDrawing",
    ),
    (TypeId::TextDocument, "Текстовый документ", "TextDocument"),
    (
        TypeId::TextDocParams,
        "Параметры макета текстового документа",
        "TextTemplateParameters",
    ),
    (TypeId::TextEncoding, "КодировкаТекста", "TextEncoding"),
    (TypeId::BinaryData, "Двоичные данные", "BinaryData"),
    (
        TypeId::BinaryDataBuffer,
        "Буфер двоичных данных",
        "BinaryDataBuffer",
    ),
    (TypeId::ByteOrder, "ПорядокБайтов", "ByteOrder"),
    (TypeId::MemoryStream, "ПотокВПамяти", "MemoryStream"),
    (TypeId::FileStream, "ФайловыйПоток", "FileStream"),
    (
        TypeId::FileStreamsManager,
        "Менеджер файловых потоков",
        "FileStreamsManager",
    ),
    (TypeId::FileOpenMode, "РежимОткрытияФайла", "FileOpenMode"),
    (TypeId::FileAccess, "ДоступКФайлу", "FileAccess"),
    (TypeId::StreamPosition, "ПозицияВПотоке", "PositionInStream"),
];

/// Типы, чьё ПРЕДСТАВЛЕНИЕ не совпадает с именем, по которому тип ищется.
///
/// Единственный такой случай — два потока: ИЗМЕРЕНО на 8.3.27, что
/// `Строка(Тип("ПотокВПамяти"))` и `Строка(Тип("ФайловыйПоток"))` оба дают
/// «Файловый поток», хотя сами типы РАЗНЫЕ (`Тип("ПотокВПамяти") =
/// Тип("ФайловыйПоток")` — «Нет»). Одно имя на два типа в [`NAMES`] не
/// помещается: там русская колонка служит и поиском, и печатью. Обратная
/// сторона совпадения — `Тип("Файловый поток")` находит `FileStream`
/// (пробелы при поиске не значимы), а `MemoryStream` через своё
/// представление уже не находится; так же несимметрично ведёт себя и
/// платформа.
const DISPLAY: &[(TypeId, &str)] = &[
    (TypeId::MemoryStream, "Файловый поток"),
    (TypeId::FileStream, "Файловый поток"),
];

impl TypeId {
    /// Каноническое (русское) имя типа — то, что печатает `Строка()`.
    pub fn name(self) -> &'static str {
        if let TypeId::EnumMeta(kind) = self {
            return kind.meta_ru_name();
        }
        if let Some((_, text)) = DISPLAY.iter().find(|(id, _)| *id == self) {
            return text;
        }
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
            // Типы с отдельным представлением печатаются не своим именем
            // поиска — это измеренное свойство платформы, см. `DISPLAY`.
            if !DISPLAY.iter().any(|(d, _)| d == id) {
                assert_eq!(id.name(), *ru);
            }
        }
    }

    /// Два потока — разные типы с ОДНИМ представлением. Измерено; ровно это
    /// и закрепляет фикстура `binary-streams`.
    #[test]
    fn both_streams_print_the_same_name_but_stay_different_types() {
        assert_ne!(TypeId::MemoryStream, TypeId::FileStream);
        assert_eq!(TypeId::MemoryStream.name(), "Файловый поток");
        assert_eq!(TypeId::FileStream.name(), "Файловый поток");
        assert_eq!(TypeId::lookup("ПотокВПамяти"), Some(TypeId::MemoryStream));
        assert_eq!(TypeId::lookup("MemoryStream"), Some(TypeId::MemoryStream));
        assert_eq!(TypeId::lookup("ФайловыйПоток"), Some(TypeId::FileStream));
        assert_eq!(TypeId::lookup("FileStream"), Some(TypeId::FileStream));
    }
}
