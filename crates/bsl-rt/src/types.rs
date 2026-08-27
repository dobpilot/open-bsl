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

    /// Перечисления платформы. Тип члена — само перечисление.
    JsonValueType,
    JsonLineBreak,
    JsonEscapeCharacters,
    JsonDateFormat,
    JsonDateWritingVariant,

    XmlNodeType,

    /// Тип ЧЛЕНА перечисления `ТипУзлаDOM` — как `XmlNodeType`.
    DomNodeType,

    /// Тип ЧЛЕНА перечисления `ТипРезультатаDOMXPath` — как `DomNodeType`.
    DomXPathResultType,

    /// Тип ЧЛЕНА перечисления `НаправлениеПоиска` — как `DomNodeType`.
    SearchDirection,

    /// Типы ЧЛЕНОВ перечислений модели схемы — как `DomNodeType`.
    XsComponentType,
    XsForm,
    XsSimpleTypeVariety,
    XsModelGroupKind,
    XsDerivationMethod,
    XsValueConstraint,
    XsWhitespaceHandling,
    /// Типы ЧЛЕНОВ перечислений модели типов XDTO.
    XmlForm,
    XdtoFacetKind,

    SpreadFileType,
    DrawingKind,
    PageOrientation,

    TextEncoding,
    /// Тип члена перечисления `СпособКодированияСтроки`.
    StringEncodingMethod,
    /// Тип члена перечисления `НаправлениеСортировки`.
    SortDirection,
    BackgroundJobState,
    MessageStatus,
    ErrorCategory,
    /// Тип члена перечисления `ЧастиДаты`.
    DateFractions,
    /// Тип члена перечисления `ХешФункция`.
    HashFunction,

    // --- ДвоичныеДанные -------------------------------------------------
    // Та же пара написаний: тип печатается с пробелом («Двоичные данные»,
    // измерено пробой `BIN.TYPE`), а само ЗНАЧЕНИЕ не печатается именем
    // вовсе — оно отдаёт шестнадцатеричный дамп (см. `impl Display for
    // BslValue`).
    BinaryData,
    /// `БуферДвоичныхДанных`. Имя ТИПА с пробелами («Буфер двоичных
    /// данных»), имя ЗНАЧЕНИЯ слитно — измерено, как у соседей.
    BinaryDataBuffer,
    /// `УникальныйИдентификатор`. Представление типа измерено фикстурой
    /// `uuid`; само значение печатается канонической формой UUID, а не
    /// именем (см. `impl Display for BslValue`).
    Uuid,
    /// Тип ЧЛЕНА перечисления `ПорядокБайтов`. Измерено, что
    /// `ТипЗнч(ПорядокБайтов.LittleEndian)` печатается «ПорядокБайтов» —
    /// БЕЗ пробелов и без префикса `Перечисление`, который носит метатип
    /// голого имени (`TypeId::EnumMeta`).
    ByteOrder,

    /// Типы ЧЛЕНОВ трёх перечислений потоков — как у `ByteOrder`, без
    /// пробелов и без префикса `Перечисление`.
    FileOpenMode,
    FileAccess,
    StreamPosition,

    /// Тип ЧЛЕНА перечисления `ТипСвязиВложенияPDF`.
    PdfAttachmentRelation,

    /// Тип ЧЛЕНА перечисления `РежимВосстановленияПутейФайловZIP`.
    ZipRestorePathsMode,
    /// Тип ЧЛЕНА перечисления `ТипФайлаАрхива`.
    ArchiveFileType,

    /// Тип ЧЛЕНА перечисления `МетодСжатияZIP`.
    ZipCompressionMethod,
    /// Тип ЧЛЕНА перечисления `УровеньСжатияZIP`.
    ZipCompressionLevel,
    /// Тип ЧЛЕНА перечисления `РежимСохраненияПутейZIP`.
    ZipStorePathMode,
    /// Тип ЧЛЕНА перечисления `РежимОбработкиПодкаталоговZIP`.
    ZipSubDirProcessingMode,
    /// Тип ЧЛЕНА перечисления `МетодШифрованияZIP`.
    ZipEncryptionMethod,
    /// Тип ЧЛЕНА перечисления `КодировкаИменФайловВZipФайле`.
    ZipFileNamesEncoding,
    /// Тип ЧЛЕНА перечисления `ИспользованиеByteOrderMark`.
    ByteOrderMarkUse,
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
    // Тип с пробелом, как и остальные объектные типы JSON выше; значение
    // («Строка(Новый НастройкиСериализацииJSON)») — без, см. `lib.rs`.
    (TypeId::JsonValueType, "ТипЗначенияJSON", "JSONValueType"),
    (TypeId::JsonLineBreak, "ПереносСтрокJSON", "JSONLineBreak"),
    (
        TypeId::JsonEscapeCharacters,
        "ЭкранированиеСимволовJSON",
        "JSONCharactersEscapeMode",
    ),
    (TypeId::JsonDateFormat, "ФорматДатыJSON", "JSONDateFormat"),
    (
        TypeId::JsonDateWritingVariant,
        "ВариантЗаписиДатыJSON",
        "JSONDateWritingVariant",
    ),
    (TypeId::XmlNodeType, "ТипУзлаXML", "XMLNodeType"),
    // Двойной пробел ИЗМЕРЕН, см. комментарий у вариантов выше.
    // Оба написания ИЗМЕРЕНЫ: `Строка(Тип("СекцияCDATADOM"))` и
    // `Строка(Тип("DOMCDATASection"))` дают «Секция CDATA DOM», и тот же тип
    // отдаёт `ТипЗнч(Док.СоздатьСекциюCDATA("ц"))`.
    (TypeId::DomNodeType, "ТипУзлаDOM", "DOMNodeType"),
    (
        TypeId::SearchDirection,
        "НаправлениеПоиска",
        "SearchDirection",
    ),
    (
        TypeId::DomXPathResultType,
        "ТипРезультатаDOMXPath",
        "DOMXPathResultType",
    ),
    // «Составного», не «комплексного»: русского имени с «комплексным»
    // платформа не знает (измерено).
    (
        TypeId::XsComponentType,
        "ТипКомпонентыXS",
        "XSComponentType",
    ),
    (TypeId::XsForm, "ФормаПредставленияXS", "XSForm"),
    (
        TypeId::XsSimpleTypeVariety,
        "ВариантПростогоТипаXS",
        "XSSimpleTypeVariety",
    ),
    // Английского имени у этого перечисления НЕТ: `XSModelGroupType`
    // платформа не знает, а русское — `ВидГруппыМоделиXS` (измерено).
    (
        TypeId::XsModelGroupKind,
        "ВидГруппыМоделиXS",
        "ВидГруппыМоделиXS",
    ),
    (
        TypeId::XsDerivationMethod,
        "МетодНаследованияXS",
        "XSDerivationMethod",
    ),
    // `XSValueConstraint` платформа тоже не знает — только русское имя.
    (
        TypeId::XsValueConstraint,
        "ОграничениеЗначенияXS",
        "ОграничениеЗначенияXS",
    ),
    (
        TypeId::XsWhitespaceHandling,
        "ОбработкаПробельныхСимволовXS",
        "XSWhitespaceHandling",
    ),
    // Модель типов XDTO. Все семь представлений и оба написания каждого
    // ИЗМЕРЕНЫ через `Тип("...")`: русское имя без пробелов
    // (`ТипЗначенияXDTO`) находится и так — поиск пробелы не считает
    // значимыми, поэтому отдельной строки в `XS_IDENTIFIERS` этим типам
    // не нужно.
    // Имена фабрики и экземпляра ИЗМЕРЕНЫ так же, как остальные семь:
    // `Тип("ФабрикаXDTO")` и `Тип("XDTOFactory")` дают «Фабрика XDTO»,
    // `Тип("ОбъектXDTO")` и `Тип("XDTODataObject")` — «Объект XDTO».
    // Сериализатор ИЗМЕРЕН тем же способом: `Тип("СериализаторXDTO")` и
    // `Тип("XDTOSerializer")` дают «Сериализатор XDTO».
    // Множественное свойство экземпляра и последовательность его
    // элементов — оба имени и оба написания ИЗМЕРЕНЫ через `Тип("...")`:
    // «Список XDTO» и «Последовательность XDTO».
    (TypeId::XmlForm, "ФормаXML", "XMLForm"),
    (TypeId::XdtoFacetKind, "ВидФасетаXDTO", "XDTOFacetType"),
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
        TypeId::PageOrientation,
        "ОриентацияСтраницы",
        "PageOrientation",
    ),
    (TypeId::TextEncoding, "КодировкаТекста", "TextEncoding"),
    (
        TypeId::StringEncodingMethod,
        "StringEncodingMethod",
        "StringEncodingMethod",
    ),
    (TypeId::SortDirection, "SortDirection", "SortDirection"),
    (
        TypeId::BackgroundJobState,
        "СостояниеФоновогоЗадания",
        "BackgroundJobState",
    ),
    (TypeId::MessageStatus, "СтатусСообщения", "MessageStatus"),
    (TypeId::ErrorCategory, "КатегорияОшибки", "ErrorCategory"),
    (TypeId::DateFractions, "DateFractions", "DateFractions"),
    (TypeId::HashFunction, "HashFunction", "HashFunction"),
    (TypeId::BinaryData, "Двоичные данные", "BinaryData"),
    (
        TypeId::BinaryDataBuffer,
        "Буфер двоичных данных",
        "BinaryDataBuffer",
    ),
    // Представление — с пробелом, как у двоичных соседей (измерено
    // фикстурой `uuid`); поиск по слитному написанию работает через
    // незначимость пробелов.
    (TypeId::Uuid, "Уникальный идентификатор", "UUID"),
    (TypeId::ByteOrder, "ПорядокБайтов", "ByteOrder"),
    (TypeId::FileOpenMode, "РежимОткрытияФайла", "FileOpenMode"),
    (TypeId::FileAccess, "ДоступКФайлу", "FileAccess"),
    (TypeId::StreamPosition, "ПозицияВПотоке", "PositionInStream"),
    // Представления всех трёх ИЗМЕРЕНЫ через `Строка(Тип(...))`: у
    // документа — с пробелом, у страницы и коллекции — без.
    // Представления обоих ИЗМЕРЕНЫ через `Строка(Тип(...))` — они совпадают
    // с именами в коде.
    (
        TypeId::PdfAttachmentRelation,
        "ТипСвязиВложенияPDF",
        "ТипСвязиВложенияPDF",
    ),
    // Представления всех шести ИЗМЕРЕНЫ через `Строка(ТипЗнч(...))`.
    // Отдельной строки в `IDENTIFIERS` им не нужно: имя в коде отличается
    // от представления только пробелами, а поиск их не считает значимыми.
    (
        TypeId::ZipRestorePathsMode,
        "РежимВосстановленияПутейФайловZIP",
        "ZIPRestoreFilePathsMode",
    ),
    (TypeId::ArchiveFileType, "ТипФайлаАрхива", "ArchiveFileType"),
    // Представления писателей ИЗМЕРЕНЫ через `Строка(Тип(...))`.
    // Перечисления записи: имя типа члена — имя самого перечисления,
    // измерено на каждом (`Строка(ТипЗнч(МетодСжатияZIP.Сжатие))` даёт
    // «МетодСжатияZIP»). Английское написание `КодировкаИменФайловВZipФайле`
    // — `FileNamesEncodingInZipFile`, а не `ZipFileNamesEncodingMode`:
    // проверено перебором трёх правдоподобных вариантов.
    (
        TypeId::ZipCompressionMethod,
        "МетодСжатияZIP",
        "ZIPCompressionMethod",
    ),
    (
        TypeId::ZipCompressionLevel,
        "УровеньСжатияZIP",
        "ZIPCompressionLevel",
    ),
    (
        TypeId::ZipStorePathMode,
        "РежимСохраненияПутейZIP",
        "ZIPStorePathMode",
    ),
    (
        TypeId::ZipSubDirProcessingMode,
        "РежимОбработкиПодкаталоговZIP",
        "ZIPSubDirProcessingMode",
    ),
    (
        TypeId::ZipEncryptionMethod,
        "МетодШифрованияZIP",
        "ZIPEncryptionMethod",
    ),
    (
        TypeId::ZipFileNamesEncoding,
        "КодировкаИменФайловВZipФайле",
        "FileNamesEncodingInZipFile",
    ),
    (
        TypeId::ByteOrderMarkUse,
        "ByteOrderMarkUse",
        "ByteOrderMarkUse",
    ),
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
const DISPLAY: &[(TypeId, &str)] = &[];

/// Второе русское написание — ИДЕНТИФИКАТОР — для типов, у которых
/// представление и имя в коде не совпадают.
///
/// Так устроена вся модель XML-схемы: `Строка(ТипЗнч(Объявление))` даёт
/// «Объявление элемента XML Schema», но ищется тип как
/// `Тип("ОбъявлениеЭлементаXS")` (ИЗМЕРЕНО обе стороны). У DOM такого нет —
/// там «Элемент DOM» отличается от `ЭлементDOM` только пробелом, который
/// поиск и так не считает значимым, — а вот у трёх типов XPath снова есть:
/// `Тип("РезультатXPath")` печатается «Результат DOM XPath». Печатью эта
/// таблица не служит: [`NAMES`] остаётся единственным источником
/// представлений.
const IDENTIFIERS: &[(TypeId, &str)] = &[(TypeId::ByteOrderMarkUse, "ИспользованиеByteOrderMark")];

/// Ключ поиска типа по имени: регистр и ПРОБЕЛЫ внутри имени не значимы.
/// Печатается «Не определено», а пишут в коде обычно `Тип("Неопределено")`
/// — одним словом, как литерал языка; обе формы обязаны находиться. То же
/// у типов компонентов: «Документ  DOM» с двумя пробелами и `ДокументDOM`
/// — одно имя. Единственное правило сравнения имён типов: и нативная
/// таблица, и дескрипторы компонентов ходят через него.
pub(crate) fn squash(name: &str) -> String {
    name.trim().to_uppercase().replace(' ', "")
}

/// Тип как ЗНАЧЕНИЕ: либо нативный тип ядра, либо тип объекта
/// компонента, названный своим дескриптором.
///
/// Второй вариант — то, чем компонент называет свои типы: строки в
/// закрытом `TypeId` ядра у них больше нет, ни у официального типа, ни у
/// host-типа. Имена — измеренные, и живут в самом дескрипторе
/// (`type_display` для печати, `type_names` для поиска).
///
/// `Copy` в размер указателя: `BslValue::Type` не должен раздувать
/// значение (см. инвариант про размер).
#[derive(Debug, Clone, Copy)]
pub enum TypeRef {
    Native(TypeId),
    /// Тип объекта компонента. Дескрипторы — статики, но равенство здесь
    /// ПО СОДЕРЖИМОМУ (пакет и имя): так тождество не зависит от того,
    /// собрал ли линкер две копии одного дескриптора.
    Object(&'static crate::TypeDescriptor),
}

impl TypeRef {
    /// Нативный идентификатор, если тип — из закрытого реестра ядра.
    pub fn native(self) -> Option<TypeId> {
        match self {
            TypeRef::Native(id) => Some(id),
            TypeRef::Object(_) => None,
        }
    }

    /// Русское имя типа — то, что печатает `Строка(ТипЗнч(...))`.
    pub fn name(self) -> String {
        match self {
            TypeRef::Native(id) => id.name().to_string(),
            TypeRef::Object(descriptor) => descriptor.type_display.to_string(),
        }
    }
}

impl PartialEq for TypeRef {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (TypeRef::Native(a), TypeRef::Native(b)) => a == b,
            (TypeRef::Object(a), TypeRef::Object(b)) => a.package == b.package && a.name == b.name,
            _ => false,
        }
    }
}

impl Eq for TypeRef {}

impl std::hash::Hash for TypeRef {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            TypeRef::Native(id) => {
                0u8.hash(state);
                id.hash(state);
            }
            TypeRef::Object(descriptor) => {
                1u8.hash(state);
                descriptor.package.hash(state);
                descriptor.name.hash(state);
            }
        }
    }
}

impl std::fmt::Display for TypeRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TypeRef::Native(id) => write!(f, "{id}"),
            TypeRef::Object(descriptor) => write!(f, "{}", descriptor.type_display),
        }
    }
}

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
        let key = squash(name);
        NAMES
            .iter()
            .find(|(_, ru, en)| squash(ru) == key || squash(en) == key)
            .map(|(id, _, _)| *id)
            .or_else(|| {
                IDENTIFIERS
                    .iter()
                    .find(|(_, id_name)| squash(id_name) == key)
                    .map(|(id, _)| *id)
            })
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
}
