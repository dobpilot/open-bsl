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

    // --- DOM -----------------------------------------------------------
    // Та же пара написаний, что у JSON и XML, но с двумя измеренными
    // неожиданностями: у «Документ  DOM» и «Комментарий  DOM» между словом
    // и `DOM` ДВА пробела, а не один. Это не описка — так печатает
    // 8.3.27 и в `Строка(ТипЗнч(...))`, и в `Строка(Тип("ДокументDOM"))`.
    // Английские написания измерены через `Тип(...)`: `DOMDocument`,
    // `DOMElement`, `DOMAttribute`, `DOMText`, `DOMComment`,
    // `DOMProcessingInstruction`, `DOMNodeList`, `DOMElementList`,
    // `DOMBuilder`, а у коллекции атрибутов — `DOMAttributeMap`
    // (`DOMNamedNodeMap` и `DOMAttributeCollection` платформа НЕ знает).
    DomBuilder,
    /// `ЗаписьDOM` — сериализатор дерева. Оба написания ИЗМЕРЕНЫ:
    /// `Тип("ЗаписьDOM")` и `Тип("DOMWriter")` дают один тип, печатающийся
    /// «Запись DOM» (справка называет его иначе — см. заголовок `dom.rs`).
    DomWriter,
    DomDocument,
    DomElement,
    DomAttribute,
    DomText,
    /// `СекцияCDATADOM` — узел, который создаёт `СоздатьСекциюCDATA`. Разбор
    /// такого узла НЕ создаёт: секция вливается в текст (измерено).
    DomCdataSection,
    DomComment,
    DomProcessingInstruction,
    DomEntityReference,
    DomNodeList,
    DomAttributeMap,
    DomElementList,
    /// Тип ЧЛЕНА перечисления `ТипУзлаDOM` — как `XmlNodeType`.
    DomNodeType,

    // --- XPath над DOM ---------------------------------------------------
    // У всех трёх представление и имя поиска — РАЗНЫЕ строки, а не одна с
    // пробелами: `Тип("РезультатXPath")` печатается «Результат DOM XPath»,
    // `Тип("ВыражениеXPath")` — «Выражение DOM XPath», а
    // `Тип("РазыменовательПространствИменDOM")` — «Разыменователь
    // пространств имен DOM XPath» (без «ё», измерено). Поэтому имена
    // поиска лежат в [`IDENTIFIERS`], как у модели схемы. Английские
    // написания — `XPathResult`, `XPathExpression`, `DOMNamespaceResolver`;
    // `РезультатXPathDOM`, `XPathResultType` и `XPathNSResolver` платформа
    // НЕ знает.
    XPathResult,
    XPathExpression,
    DomNamespaceResolver,
    /// Тип ЧЛЕНА перечисления `ТипРезультатаDOMXPath` — как `DomNodeType`.
    DomXPathResultType,

    // --- Регулярные выражения --------------------------------------------
    // Представления ИЗМЕРЕНЫ: `Тип("РезультатПоискаПоРегулярномуВыражению")`
    // печатается «Результат поиска по регулярному выражению», а
    // `Тип("ГруппаРезультатаПоискаПоРегулярномуВыражению")` — «Группа
    // результата поиска по регулярному выражению». Английские написания
    // `ResultOfSearchByRegularExpression` и
    // `ResultOfSearchByRegularExpressionGroup` находят те же типы. Второго
    // русского написания (`IDENTIFIERS`) им не нужно: от представления имя
    // отличается только пробелами и регистром, а поиск не считает значимым
    // ни то, ни другое.
    RegexMatch,
    RegexMatchGroup,
    /// Тип ЧЛЕНА перечисления `НаправлениеПоиска` — как `DomNodeType`.
    SearchDirection,

    // --- Объектная модель XML-схемы ------------------------------------
    // Написания сняты пробами (`measure-xsd.bsl`) и в двух местах не такие,
    // как подсказала бы аналогия: русского имени `ОпределениеКомплексного\
    // ТипаXS` у платформы НЕТ (тип зовётся `ОпределениеСоставногоТипаXS`), а
    // `XSParticle` печатается «Фрагмент XML Schema», то есть частица здесь
    // называется фрагментом. Английские написания — тоже пробами: у трёх
    // коллекций это `XSComponentFixedList`, `XSNamedComponentMap` и
    // `XSComponentList`, а `xs:appinfo` зовётся `XSAppInfo`
    // (`XSApplicationInformation` платформа НЕ знает).
    XmlSchema,
    XmlSchemaSet,
    XmlSchemaBuilder,
    XsElementDeclaration,
    XsAttributeDeclaration,
    XsSimpleTypeDefinition,
    XsComplexTypeDefinition,
    XsParticle,
    XsModelGroup,
    XsAttributeUse,
    XsAnnotation,
    XsDocumentation,
    XsAppInfo,
    XsLengthFacet,
    XsMinLengthFacet,
    XsMaxLengthFacet,
    XsPatternFacet,
    XsEnumerationFacet,
    XsWhitespaceFacet,
    XsTotalDigitsFacet,
    XsFractionDigitsFacet,
    XsMinInclusiveFacet,
    XsMaxInclusiveFacet,
    XsMinExclusiveFacet,
    XsMaxExclusiveFacet,
    XsComponentFixedList,
    XsNamedComponentMap,
    XsComponentList,
    XmlExpandedName,
    XmlExpandedNameList,

    // --- модель типов XDTO --------------------------------------------
    XdtoFactory,
    XdtoSerializer,
    XdtoDataObject,
    XdtoValueType,
    XdtoObjectType,
    XdtoProperty,
    XdtoPropertyCollection,
    XdtoFacet,
    XdtoFacetCollection,
    XdtoDataValue,
    XdtoList,
    XdtoSequence,
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

    // --- ТабличныйДокумент ---------------------------------------------
    // Та же пара написаний, что у остальных: тип печатается с пробелом
    // («Табличный документ»), значение — без. Измерено на 8.3.27.
    SpreadDocument,
    SpreadArea,
    SpreadDrawings,
    SpreadDrawing,
    SpreadFileType,
    DrawingKind,
    PageOrientation,

    // --- ТекстовыйДокумент ---------------------------------------------
    // Та же пара написаний: тип с пробелом («Текстовый документ»),
    // значение без. Имя типа параметров — «Параметры макета текстового
    // документа», измерено.
    TextDocument,
    TextDocParams,
    TextEncoding,

    // --- Параметры макета табличного документа ---
    SpreadDocParams,

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

    // --- ЧтениеДанных / ЗаписьДанных -------------------------------------
    /// Та же пара написаний, что у JSON и XML: имя ТИПА с пробелом («Чтение
    /// данных»), имя ЗНАЧЕНИЯ слитно («ЧтениеДанных») — измерено обеими
    /// сторонами. Отдельного `DISPLAY` им, в отличие от потоков, не нужно:
    /// поиск сминает пробелы, и `Тип("ЧтениеДанных")` находит этот же тип.
    DataReader,
    DataWriter,
    /// `РезультатЧтенияДанных` — то, что отдаёт `ЧтениеДанных.Прочитать`.
    /// Английское имя ИЗМЕРЕНО перебором: `Тип("ReadDataResult")` платформа
    /// разрешает, а `DataReadResult`, `DataReaderResult`, `DataReadingResult`
    /// и `ReadResult` — нет.
    DataReadResult,

    // --- PDF --------------------------------------------------------------
    // Три типа. У документа ПРЕДСТАВЛЕНИЕ отличается от имени в коде
    // пробелом («Документ PDF» против `ДокументPDF`) — поиск пробелов не
    // считает, поэтому отдельной строки в `IDENTIFIERS` не нужно; у
    // страницы и коллекции представление и имя совпадают. Английские
    // написания измерены: `Тип("PDFDocument")`, `Тип("PDFPage")` и
    // `Тип("PDFPagesCollection")` платформа разрешает в те же три типа.
    PdfDocument,
    PdfPage,
    PdfPagesCollection,
    // Ещё два типа и одно перечисление — вложения. Английские написания
    // измерены: `Тип("PDFAttachment")` и `Тип("PDFAttachmentCollection")`
    // платформа разрешает в те же типы, а `PDFAttachmentsCollection` не
    // знает. У перечисления `ТипСвязиВложенияPDF` английского написания
    // НЕТ (проверено перебором), поэтому обе колонки у него одинаковы.
    PdfAttachment,
    PdfAttachmentCollection,
    /// Тип ЧЛЕНА перечисления `ТипСвязиВложенияPDF`.
    PdfAttachmentRelation,

    // --- чтение архивов ---------------------------------------------------
    // ШЕСТЬ типов, а не три: на 8.3.27 рядом живут два читателя архивов со
    // своими коллекциями и элементами, и это ИЗМЕРЕНО —
    // `Тип("ЧтениеZipФайла")` печатается «Чтение ZIP файла», а
    // `Тип("ЧтениеФайлаАрхива")` — «Чтение файла архива». Английские
    // написания измерены все шесть; `ZipFileReader` и `ZIPFileReader`
    // платформа считает одним и тем же типом (у нас это выходит само:
    // поиск регистронезависим).
    ZipFileReader,
    ZipFileEntries,
    ZipFileEntry,
    ArchiveFileReader,
    ArchiveFileEntries,
    ArchiveFileEntry,
    /// Тип ЧЛЕНА перечисления `РежимВосстановленияПутейФайловZIP`.
    ZipRestorePathsMode,
    /// Тип ЧЛЕНА перечисления `ТипФайлаАрхива`.
    ArchiveFileType,

    // --- запись архивов ---------------------------------------------------
    // Писателей, как и читателей, ДВА, и они тоже разные типы: измерено, что
    // `Тип("ЗаписьZipФайла")` печатается «Запись ZIP файла», а
    // `Тип("ЗаписьФайлаАрхива")` — «Запись файла архива». Английские
    // написания измерены оба (`ZipFileWriter`, `ArchiveFileWriter`);
    // `ZipFileWrite` и `ArchiveFileWrite` платформа не знает.
    ZipFileWriter,
    ArchiveFileWriter,
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
    (TypeId::DomBuilder, "Построитель DOM", "DOMBuilder"),
    (TypeId::DomWriter, "Запись DOM", "DOMWriter"),
    // Двойной пробел ИЗМЕРЕН, см. комментарий у вариантов выше.
    (TypeId::DomDocument, "Документ  DOM", "DOMDocument"),
    (TypeId::DomElement, "Элемент DOM", "DOMElement"),
    (TypeId::DomAttribute, "Атрибут DOM", "DOMAttribute"),
    (TypeId::DomText, "Текст DOM", "DOMText"),
    // Оба написания ИЗМЕРЕНЫ: `Строка(Тип("СекцияCDATADOM"))` и
    // `Строка(Тип("DOMCDATASection"))` дают «Секция CDATA DOM», и тот же тип
    // отдаёт `ТипЗнч(Док.СоздатьСекциюCDATA("ц"))`.
    (
        TypeId::DomCdataSection,
        "Секция CDATA DOM",
        "DOMCDATASection",
    ),
    (TypeId::DomComment, "Комментарий  DOM", "DOMComment"),
    (
        TypeId::DomProcessingInstruction,
        "Инструкция обработки DOM",
        "DOMProcessingInstruction",
    ),
    (
        TypeId::DomEntityReference,
        "Ссылка на сущность DOM",
        "DOMEntityReference",
    ),
    (TypeId::DomNodeList, "Список узлов DOM", "DOMNodeList"),
    (
        TypeId::DomAttributeMap,
        "Коллекция атрибутов DOM",
        "DOMAttributeMap",
    ),
    (
        TypeId::DomElementList,
        "Список элементов DOM",
        "DOMElementList",
    ),
    (TypeId::DomNodeType, "ТипУзлаDOM", "DOMNodeType"),
    (
        TypeId::RegexMatch,
        "Результат поиска по регулярному выражению",
        "ResultOfSearchByRegularExpression",
    ),
    (
        TypeId::RegexMatchGroup,
        "Группа результата поиска по регулярному выражению",
        "ResultOfSearchByRegularExpressionGroup",
    ),
    (
        TypeId::SearchDirection,
        "НаправлениеПоиска",
        "SearchDirection",
    ),
    (TypeId::XPathResult, "Результат DOM XPath", "XPathResult"),
    (
        TypeId::XPathExpression,
        "Выражение DOM XPath",
        "XPathExpression",
    ),
    (
        TypeId::DomNamespaceResolver,
        "Разыменователь пространств имен DOM XPath",
        "DOMNamespaceResolver",
    ),
    (
        TypeId::DomXPathResultType,
        "ТипРезультатаDOMXPath",
        "DOMXPathResultType",
    ),
    (TypeId::XmlSchema, "Схема XML", "XMLSchema"),
    (TypeId::XmlSchemaSet, "Набор схем XML", "XMLSchemaSet"),
    (
        TypeId::XmlSchemaBuilder,
        "Построитель схем XML",
        "XMLSchemaBuilder",
    ),
    (
        TypeId::XsElementDeclaration,
        "Объявление элемента XML Schema",
        "XSElementDeclaration",
    ),
    (
        TypeId::XsAttributeDeclaration,
        "Объявление атрибута XML Schema",
        "XSAttributeDeclaration",
    ),
    (
        TypeId::XsSimpleTypeDefinition,
        "Определение простого типа XML Schema",
        "XSSimpleTypeDefinition",
    ),
    // «Составного», не «комплексного»: русского имени с «комплексным»
    // платформа не знает (измерено).
    (
        TypeId::XsComplexTypeDefinition,
        "Определение составного типа XML Schema",
        "XSComplexTypeDefinition",
    ),
    (TypeId::XsParticle, "Фрагмент XML Schema", "XSParticle"),
    (
        TypeId::XsModelGroup,
        "Группа модели XML Schema",
        "XSModelGroup",
    ),
    (
        TypeId::XsAttributeUse,
        "Использование атрибута XML Schema",
        "XSAttributeUse",
    ),
    (TypeId::XsAnnotation, "Аннотация XML Schema", "XSAnnotation"),
    (
        TypeId::XsDocumentation,
        "Документация XML Schema",
        "XSDocumentation",
    ),
    (
        TypeId::XsAppInfo,
        "Информация для приложения XML Schema",
        "XSAppInfo",
    ),
    (
        TypeId::XsLengthFacet,
        "Фасет длины значения XML Schema",
        "XSLengthFacet",
    ),
    (
        TypeId::XsMinLengthFacet,
        "Фасет минимальной длины значения XML Schema",
        "XSMinLengthFacet",
    ),
    (
        TypeId::XsMaxLengthFacet,
        "Фасет максимальной длины значения XML Schema",
        "XSMaxLengthFacet",
    ),
    (
        TypeId::XsPatternFacet,
        "Фасет образца значения XML Schema",
        "XSPatternFacet",
    ),
    (
        TypeId::XsEnumerationFacet,
        "Фасет перечисления значения XML Schema",
        "XSEnumerationFacet",
    ),
    (
        TypeId::XsWhitespaceFacet,
        "Фасет пробельных символов XML Schema",
        "XSWhitespaceFacet",
    ),
    (
        TypeId::XsTotalDigitsFacet,
        "Фасет общего количества разрядов значения XML Schema",
        "XSTotalDigitsFacet",
    ),
    (
        TypeId::XsFractionDigitsFacet,
        "Фасет количества разрядов дробной части значения XML Schema",
        "XSFractionDigitsFacet",
    ),
    (
        TypeId::XsMinInclusiveFacet,
        "Фасет минимального включающего значения XML Schema",
        "XSMinInclusiveFacet",
    ),
    (
        TypeId::XsMaxInclusiveFacet,
        "Фасет максимального включающего значения XML Schema",
        "XSMaxInclusiveFacet",
    ),
    (
        TypeId::XsMinExclusiveFacet,
        "Фасет минимального исключающего значения XML Schema",
        "XSMinExclusiveFacet",
    ),
    (
        TypeId::XsMaxExclusiveFacet,
        "Фасет максимального исключающего значения XML Schema",
        "XSMaxExclusiveFacet",
    ),
    (
        TypeId::XsComponentFixedList,
        "Фиксированный список компонент XML Schema",
        "XSComponentFixedList",
    ),
    (
        TypeId::XsNamedComponentMap,
        "Коллекция именованных компонент XML Schema",
        "XSNamedComponentMap",
    ),
    (
        TypeId::XsComponentList,
        "Список компонент XML Schema",
        "XSComponentList",
    ),
    (
        TypeId::XmlExpandedName,
        "Расширенное имя XML",
        "XMLExpandedName",
    ),
    (
        TypeId::XmlExpandedNameList,
        "Список расширенных имен XML",
        "XMLExpandedNameList",
    ),
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
    (TypeId::XdtoFactory, "Фабрика XDTO", "XDTOFactory"),
    // Сериализатор ИЗМЕРЕН тем же способом: `Тип("СериализаторXDTO")` и
    // `Тип("XDTOSerializer")` дают «Сериализатор XDTO».
    (
        TypeId::XdtoSerializer,
        "Сериализатор XDTO",
        "XDTOSerializer",
    ),
    (TypeId::XdtoDataObject, "Объект XDTO", "XDTODataObject"),
    (TypeId::XdtoValueType, "Тип значения XDTO", "XDTOValueType"),
    (TypeId::XdtoObjectType, "Тип объекта XDTO", "XDTOObjectType"),
    (TypeId::XdtoProperty, "Свойство XDTO", "XDTOProperty"),
    (
        TypeId::XdtoPropertyCollection,
        "Коллекция свойств XDTO",
        "XDTOPropertyCollection",
    ),
    (TypeId::XdtoFacet, "Фасет XDTO", "XDTOFacet"),
    (
        TypeId::XdtoFacetCollection,
        "Коллекция фасетов XDTO",
        "XDTOFacetCollection",
    ),
    (TypeId::XdtoDataValue, "Значение XDTO", "XDTODataValue"),
    // Множественное свойство экземпляра и последовательность его
    // элементов — оба имени и оба написания ИЗМЕРЕНЫ через `Тип("...")`:
    // «Список XDTO» и «Последовательность XDTO».
    (TypeId::XdtoList, "Список XDTO", "XDTOList"),
    (
        TypeId::XdtoSequence,
        "Последовательность XDTO",
        "XDTOSequence",
    ),
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
    (
        TypeId::SpreadDocParams,
        "Параметры макета табличного документа",
        "SpreadsheetDocumentTemplateParameters",
    ),
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
    (TypeId::DataReader, "Чтение данных", "DataReader"),
    (TypeId::DataWriter, "Запись данных", "DataWriter"),
    (
        TypeId::DataReadResult,
        "Результат чтения данных",
        "ReadDataResult",
    ),
    // Представления всех трёх ИЗМЕРЕНЫ через `Строка(Тип(...))`: у
    // документа — с пробелом, у страницы и коллекции — без.
    (TypeId::PdfDocument, "Документ PDF", "PDFDocument"),
    (TypeId::PdfPage, "СтраницаPDF", "PDFPage"),
    (
        TypeId::PdfPagesCollection,
        "КоллекцияСтраницPDF",
        "PDFPagesCollection",
    ),
    // Представления обоих ИЗМЕРЕНЫ через `Строка(Тип(...))` — они совпадают
    // с именами в коде.
    (TypeId::PdfAttachment, "ВложениеPDF", "PDFAttachment"),
    (
        TypeId::PdfAttachmentCollection,
        "КоллекцияВложенийPDF",
        "PDFAttachmentCollection",
    ),
    (
        TypeId::PdfAttachmentRelation,
        "ТипСвязиВложенияPDF",
        "ТипСвязиВложенияPDF",
    ),
    // Представления всех шести ИЗМЕРЕНЫ через `Строка(ТипЗнч(...))`.
    // Отдельной строки в `IDENTIFIERS` им не нужно: имя в коде отличается
    // от представления только пробелами, а поиск их не считает значимыми.
    (TypeId::ZipFileReader, "Чтение ZIP файла", "ZipFileReader"),
    (
        TypeId::ZipFileEntries,
        "Элементы ZIP файла",
        "ZipFileEntries",
    ),
    (TypeId::ZipFileEntry, "Элемент ZIP файла", "ZipFileEntry"),
    (
        TypeId::ArchiveFileReader,
        "Чтение файла архива",
        "ArchiveFileReader",
    ),
    (
        TypeId::ArchiveFileEntries,
        "Элементы файла архива",
        "ArchiveFileEntries",
    ),
    (
        TypeId::ArchiveFileEntry,
        "Элемент файла архива",
        "ArchiveFileEntry",
    ),
    (
        TypeId::ZipRestorePathsMode,
        "РежимВосстановленияПутейФайловZIP",
        "ZIPRestoreFilePathsMode",
    ),
    (TypeId::ArchiveFileType, "ТипФайлаАрхива", "ArchiveFileType"),
    // Представления писателей ИЗМЕРЕНЫ через `Строка(Тип(...))`.
    (TypeId::ZipFileWriter, "Запись ZIP файла", "ZipFileWriter"),
    (
        TypeId::ArchiveFileWriter,
        "Запись файла архива",
        "ArchiveFileWriter",
    ),
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
const IDENTIFIERS: &[(TypeId, &str)] = &[
    (TypeId::XPathResult, "РезультатXPath"),
    (TypeId::XPathExpression, "ВыражениеXPath"),
    (
        TypeId::DomNamespaceResolver,
        "РазыменовательПространствИменDOM",
    ),
    (TypeId::XsElementDeclaration, "ОбъявлениеЭлементаXS"),
    (TypeId::XsAttributeDeclaration, "ОбъявлениеАтрибутаXS"),
    (TypeId::XsSimpleTypeDefinition, "ОпределениеПростогоТипаXS"),
    (
        TypeId::XsComplexTypeDefinition,
        "ОпределениеСоставногоТипаXS",
    ),
    (TypeId::XsParticle, "ФрагментXS"),
    (TypeId::XsModelGroup, "ГруппаМоделиXS"),
    (TypeId::XsAttributeUse, "ИспользованиеАтрибутаXS"),
    (TypeId::XsAnnotation, "АннотацияXS"),
    (TypeId::XsDocumentation, "ДокументацияXS"),
    (TypeId::XsAppInfo, "ИнформацияДляПриложенияXS"),
    (TypeId::XsLengthFacet, "ФасетДлиныXS"),
    (TypeId::XsMinLengthFacet, "ФасетМинимальнойДлиныXS"),
    (TypeId::XsMaxLengthFacet, "ФасетМаксимальнойДлиныXS"),
    (TypeId::XsPatternFacet, "ФасетОбразцаXS"),
    (TypeId::XsEnumerationFacet, "ФасетПеречисленияXS"),
    (TypeId::XsWhitespaceFacet, "ФасетПробельныхСимволовXS"),
    (
        TypeId::XsTotalDigitsFacet,
        "ФасетОбщегоКоличестваРазрядовXS",
    ),
    (
        TypeId::XsFractionDigitsFacet,
        "ФасетКоличестваРазрядовДробнойЧастиXS",
    ),
    (
        TypeId::XsMinInclusiveFacet,
        "ФасетМинимальногоВключающегоЗначенияXS",
    ),
    (
        TypeId::XsMaxInclusiveFacet,
        "ФасетМаксимальногоВключающегоЗначенияXS",
    ),
    (
        TypeId::XsMinExclusiveFacet,
        "ФасетМинимальногоИсключающегоЗначенияXS",
    ),
    (
        TypeId::XsMaxExclusiveFacet,
        "ФасетМаксимальногоИсключающегоЗначенияXS",
    ),
    (
        TypeId::XsComponentFixedList,
        "ФиксированныйСписокКомпонентXS",
    ),
    (
        TypeId::XsNamedComponentMap,
        "КоллекцияИменованныхКомпонентXS",
    ),
    (TypeId::XsComponentList, "СписокКомпонентXS"),
];

/// Тип как ЗНАЧЕНИЕ: либо нативный тип ядра, либо тип объекта
/// компонента, названный своим дескриптором.
///
/// Второй вариант — шаг к тому, чтобы компонент не заводил себе строку в
/// закрытом `TypeId` ядра: у host-типа идентификатора там нет и быть не
/// может, а `ТипЗнч` над ним обязан работать. Официальные компоненты пока
/// сохраняют свои `TypeId` (поле `TypeDescriptor::legacy_type_id`), и до
/// их перевода `Тип("ЧтениеXML")` и `ТипЗнч(читатель)` дают ОДИН И ТОТ ЖЕ
/// нативный вариант — измеренное равенство этих двух не меняется.
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
            TypeRef::Object(descriptor) => descriptor.name.to_string(),
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
            TypeRef::Object(descriptor) => write!(f, "{}", descriptor.name),
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
        // Пробелы внутри имени не значимы: печатается «Не определено», а
        // пишут в коде обычно `Тип("Неопределено")` — одним словом, как
        // литерал языка. Обе формы обязаны находиться.
        let squash = |s: &str| s.to_uppercase().replace(' ', "");
        let key = squash(name.trim());
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
