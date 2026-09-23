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
    // --- ВариантВстроенногоЯзыка ---------------------------------------
    ScriptVariantEnglish,
    ScriptVariantRussian,

    // --- РежимСовместимости ------------------------------------------
    CompatibilityModeVersion8_1,
    CompatibilityModeVersion8_2_13,
    CompatibilityModeVersion8_2_16,
    CompatibilityModeVersion8_3_1,
    CompatibilityModeVersion8_3_2,
    CompatibilityModeVersion8_3_3,
    CompatibilityModeVersion8_3_4,
    CompatibilityModeVersion8_3_5,
    CompatibilityModeVersion8_3_6,
    CompatibilityModeVersion8_3_7,
    CompatibilityModeVersion8_3_8,
    CompatibilityModeVersion8_3_9,
    CompatibilityModeVersion8_3_10,
    CompatibilityModeVersion8_3_11,
    CompatibilityModeVersion8_3_12,
    CompatibilityModeVersion8_3_13,
    CompatibilityModeVersion8_3_14,
    CompatibilityModeVersion8_3_15,
    CompatibilityModeVersion8_3_16,
    CompatibilityModeVersion8_3_17,
    CompatibilityModeVersion8_3_18,
    CompatibilityModeVersion8_3_19,
    CompatibilityModeVersion8_3_20,
    CompatibilityModeVersion8_3_21,
    CompatibilityModeVersion8_3_22,
    CompatibilityModeVersion8_3_23,
    CompatibilityModeVersion8_3_24,
    CompatibilityModeVersion8_3_25,
    CompatibilityModeVersion8_3_26,
    CompatibilityModeDontUse,

    // --- РежимЗапускаКлиентскогоПриложения ---------------------------
    ClientRunModeAuto,
    ClientRunModeOrdinaryApplication,
    ClientRunModeManagedApplication,

    // --- Расширения конфигурации --------------------------------------
    ConfigurationExtensionsSourceDatabase,
    ConfigurationExtensionsSourceSessionApplied,
    ConfigurationExtensionsSourceSessionDisabled,
    ConfigurationExtensionScopeInfoBase,
    ConfigurationExtensionScopeDataSeparation,
    ConfigurationExtensionApplicationIssueSeverityCritical,
    ConfigurationExtensionApplicationIssueSeverityLow,
    ConfigurationExtensionApplicationIssueSeverityModerate,

    // --- Журнал регистрации ------------------------------------------
    EventLogLevelNote,
    EventLogLevelInformation,
    EventLogLevelWarning,
    EventLogLevelError,
    EventLogEntryTransactionModeIndependent,
    EventLogEntryTransactionModeTransactional,

    // --- РежимОтправкиИнформацииОбОшибке -----------------------------
    ErrorReportingModeAuto,
    ErrorReportingModeDontSend,
    ErrorReportingModeAskUser,
    ErrorReportingModeSend,

    // --- ОтправкаЭлементаДанных --------------------------------------
    DataItemSendAuto,
    DataItemSendDelete,
    DataItemSendIgnore,

    // --- ПолучениеЭлементаДанных -------------------------------------
    DataItemReceiveAuto,
    DataItemReceiveAccept,
    DataItemReceiveIgnore,

    // --- ОбновлениеПредопределенныхДанных ----------------------------
    PredefinedDataUpdateAuto,
    PredefinedDataUpdateAutoUpdate,
    PredefinedDataUpdateDontAutoUpdate,

    // --- РежимПроведенияДокумента ------------------------------------
    DocumentPostingModeOperational,
    DocumentPostingModeNonOperational,

    // --- ВидСравненияКомпоновкиДанных ---------------------------------
    DataCompositionComparisonBeginsWith,
    DataCompositionComparisonContains,
    DataCompositionComparisonEqual,
    DataCompositionComparisonFilled,
    DataCompositionComparisonGreater,
    DataCompositionComparisonGreaterOrEqual,
    DataCompositionComparisonInHierarchy,
    DataCompositionComparisonInList,
    DataCompositionComparisonInListByHierarchy,
    DataCompositionComparisonLess,
    DataCompositionComparisonLessOrEqual,
    DataCompositionComparisonLike,
    DataCompositionComparisonNotBeginsWith,
    DataCompositionComparisonNotContains,
    DataCompositionComparisonNotEqual,
    DataCompositionComparisonNotFilled,
    DataCompositionComparisonNotInHierarchy,
    DataCompositionComparisonNotInList,
    DataCompositionComparisonNotInListByHierarchy,
    DataCompositionComparisonNotLike,

    // --- ВариантСтандартнойДатыНачала ---------------------------------
    StandardBeginningDateThisDay,
    StandardBeginningDateThisWeek,
    StandardBeginningDateThisTenDays,
    StandardBeginningDateThisMonth,
    StandardBeginningDateThisQuarter,
    StandardBeginningDateThisHalfYear,
    StandardBeginningDateThisYear,
    StandardBeginningDateLastDay,
    StandardBeginningDateLastWeek,
    StandardBeginningDateLastTenDays,
    StandardBeginningDateLastMonth,
    StandardBeginningDateLastQuarter,
    StandardBeginningDateLastHalfYear,
    StandardBeginningDateLastYear,
    StandardBeginningDateNextDay,
    StandardBeginningDateNextWeek,
    StandardBeginningDateNextTenDays,
    StandardBeginningDateNextMonth,
    StandardBeginningDateNextQuarter,
    StandardBeginningDateNextHalfYear,
    StandardBeginningDateNextYear,
    StandardBeginningDateCustom,

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

    // --- ЭкранированиеСимволовJSON -----------------------------------
    JsonEscapeNone,
    JsonEscapeNonAscii,
    JsonEscapeNonBmp,

    // --- ФорматДатыJSON -----------------------------------------------
    DateFormatIso,
    DateFormatJavaScript,
    DateFormatMicrosoft,

    // --- ВариантЗаписиДатыJSON ------------------------------------------
    DateVariantLocal,
    DateVariantLocalOffset,
    DateVariantUniversal,

    // --- ТипФайлаТабличногоДокумента -----------------------------------
    // Членов у платформы четырнадцать (MXL, MXL7, XLS, XLS95, XLSX, ODS,
    // TXT, ANSITXT, HTML, HTML3, HTML4, HTML5, PDF, DOCX; `MXLX` НЕТ —
    // измерено перебором). Здесь заведены форматы записи и значения, которые
    // модуль сеанса читает как данные. Наличие члена не обещает поддержку
    // экспортёром: `bsl-spreadsheet` отклоняет формат без кодировщика.
    SpreadFileMxl,
    SpreadFileXls,
    SpreadFileOds,
    SpreadFileAnsiTxt,
    SpreadFileHtml5,
    SpreadFileDocx,
    SpreadFileTxt,
    SpreadFileXlsx,
    SpreadFilePdf,
    /// Единственный поддержанный вид рисунка.
    DrawingRectangle,

    // --- ОриентацияСтраницы ---------------------------------------------
    // Оба члена ИЗМЕРЕНЫ на 8.3.27: `Строка(ОриентацияСтраницы.Портрет)` —
    // «Портрет», `.Ландшафт` — «Ландшафт», метатип —
    // «ПеречислениеОриентацияСтраницы», а `PageOrientation.Landscape`
    // платформа принимает и печатает тем же русским представлением.
    PageOrientationPortrait,
    PageOrientationLandscape,

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

    // --- ТипУзлаDOM ----------------------------------------------------
    // Двенадцать членов, состав ВЫЯСНЕН перебором на 8.3.27: `ТипДокумента`,
    // `Пробелы` и `ЗначащиеПробелы` платформа не знает, зато знает
    // `ОпределениеТипаДокумента`, `ФрагментДокумента`, `Нотация`,
    // `Сущность` и `СсылкаНаСущность`. Заведены все двенадцать, хотя
    // построитель создаёт узлы только шести видов: член перечисления —
    // это то, с чем сравнивают, и «нет такого члена» было бы неправдой.
    DomElement,
    DomAttribute,
    DomText,
    DomCdataSection,
    DomComment,
    DomProcessingInstruction,
    DomDocument,
    DomDocumentType,
    DomDocumentFragment,
    DomNotation,
    DomEntity,
    DomEntityReference,

    // --- КодировкаТекста ------------------------------------------------
    // Ровно пять членов, состав проверен перебором.
    TextEncodingAnsi,
    TextEncodingOem,
    TextEncodingUtf16,
    TextEncodingUtf8,
    TextEncodingSystem,

    // --- СпособКодированияСтроки ----------------------------------------
    StringEncodingUrl,
    StringEncodingUrlInUrl,

    // --- СостояниеФоновогоЗадания ---------------------------------------
    BackgroundJobActive,
    BackgroundJobCompleted,
    BackgroundJobFailed,
    BackgroundJobCanceled,
    MessageStatusWithoutStatus,
    MessageStatusImportant,
    MessageStatusAttention,
    MessageStatusInformation,
    MessageStatusOrdinary,
    MessageStatusVeryImportant,
    ErrorCategoryAllErrors,
    ErrorCategoryExceptionRaisedFromScript,
    ErrorCategoryAccessViolation,
    ErrorCategoryUnsupportedFormat,
    ErrorCategoryInvalidPassword,
    ErrorCategoryNoPermissionToUseFunctionality,
    ErrorCategoryExternalDataSourceError,
    ErrorCategoryScriptRuntimeError,
    ErrorCategoryLocalFileAccessError,
    ErrorCategoryScriptUseError,
    ErrorCategoryScriptCompileError,
    ErrorCategoryConfigurationError,
    ErrorCategoryDatabaseCopyError,
    ErrorCategoryDataCompositionSettingsError,
    ErrorCategoryGotoURLError,
    ErrorCategoryFullTextSearchError,
    ErrorCategoryDocumentConversionError,
    ErrorCategorySignatureVerificationError,
    ErrorCategoryPrinterError,
    ErrorCategorySpeechProcessingError,
    ErrorCategorySessionError,
    ErrorCategoryNetworkError,
    ErrorCategoryCollaborationSystemError,
    ErrorCategoryMultimediaToolsError,
    ErrorCategoryDatabaseTablespaceError,
    ErrorCategoryStoredDataError,
    ErrorCategoryForcedShutdown,
    ErrorCategoryOtherError,
    // --- НаправлениеСортировки ------------------------------------------
    SortDirectionAscending,

    // --- ЧастиДаты -------------------------------------------------------
    DateFractionsDateTime,

    // --- ДопустимаяДлина -------------------------------------------------
    AllowedLengthVariable,
    AllowedLengthFixed,

    // --- ХешФункция -----------------------------------------------------
    HashMd5,
    HashSha1,
    HashSha256,

    // --- ПорядокБайтов --------------------------------------------------
    // Ровно два члена, и оба пишутся ОДИНАКОВО на обоих языках: русских
    // `Прямой`/`Обратный` платформа не знает (проверено перебором).
    ByteOrderLittle,
    ByteOrderBig,

    // --- РежимОткрытияФайла ---------------------------------------------
    // Шесть членов; состав и написания измерены полной таблицей «режим x
    // доступ» (см. заголовок `crate::stream`).
    FileOpenModeOpen,
    FileOpenModeOpenOrCreate,
    FileOpenModeCreate,
    FileOpenModeCreateNew,
    FileOpenModeTruncate,
    FileOpenModeAppend,

    // --- ДоступКФайлу ------------------------------------------------------
    FileAccessRead,
    FileAccessWrite,
    FileAccessReadAndWrite,

    // --- ПозицияВПотоке ----------------------------------------------------
    StreamPositionBegin,
    StreamPositionCurrent,
    StreamPositionEnd,

    // --- ТипКомпонентыXS ---------------------------------------------------
    // Заведены ВСЕ члены, которые платформа признала (перебором имён из
    // словаря `xml2.so`), а не только те, что отдаёт наша модель: член,
    // который есть у платформы, обязан сравниваться и здесь.
    XsCompSchema,
    XsCompElementDeclaration,
    XsCompAttributeDeclaration,
    XsCompSimpleTypeDefinition,
    XsCompComplexTypeDefinition,
    XsCompParticle,
    XsCompModelGroup,
    XsCompAttributeUse,
    XsCompModelGroupDefinition,
    XsCompAttributeGroupDefinition,
    XsCompNotationDeclaration,
    XsCompAnnotation,
    XsCompDocumentation,
    XsCompAppInfo,
    XsCompWildcard,
    XsCompImport,
    XsCompInclude,
    XsCompRedefine,
    XsCompLengthFacet,
    XsCompMinLengthFacet,
    XsCompMaxLengthFacet,
    XsCompPatternFacet,
    XsCompEnumerationFacet,
    XsCompWhitespaceFacet,
    XsCompTotalDigitsFacet,
    XsCompFractionDigitsFacet,
    XsCompMinInclusiveFacet,
    XsCompMaxInclusiveFacet,
    XsCompMinExclusiveFacet,
    XsCompMaxExclusiveFacet,
    XsCompIdentityConstraintDefinition,
    XsCompXPathDefinition,

    // --- ФормаПредставленияXS ---------------------------------------------
    XsFormQualified,
    XsFormUnqualified,

    // --- ФормаXML (модель типов XDTO) ---------------------------------
    XmlFormElement,
    XmlFormAttribute,
    XmlFormText,

    // --- ВидФасетаXDTO -------------------------------------------------
    XdtoFacetLength,
    XdtoFacetMinLength,
    XdtoFacetMaxLength,
    XdtoFacetPattern,
    XdtoFacetEnumeration,
    XdtoFacetWhiteSpace,
    XdtoFacetTotalDigits,
    XdtoFacetFractionDigits,
    XdtoFacetMinInclusive,
    XdtoFacetMaxInclusive,
    XdtoFacetMinExclusive,
    XdtoFacetMaxExclusive,

    // --- ВариантПростогоТипаXS --------------------------------------------
    XsVarietyAtomic,
    XsVarietyList,
    XsVarietyUnion,

    // --- ВидГруппыМоделиXS -------------------------------------------------
    XsGroupSequence,
    XsGroupChoice,
    XsGroupAll,

    // --- МетодНаследованияXS -----------------------------------------------
    XsDerivationExtension,
    XsDerivationRestriction,

    // --- ОграничениеЗначенияXS ---------------------------------------------
    XsConstraintDefault,
    XsConstraintFixed,

    // --- ОбработкаПробельныхСимволовXS -------------------------------------
    XsWhitespacePreserve,
    XsWhitespaceReplace,
    XsWhitespaceCollapse,

    // --- ТипРезультатаDOMXPath ---------------------------------------------
    // Все десять членов и оба их написания ИЗМЕРЕНЫ перебором на 8.3.27:
    // `МножествоУзлов`, `НаборУзлов`, `Узел`, `Итератор`, `Снимок` и
    // `NodeSet` платформа НЕ знает, а вот `Любой`/`Any`,
    // `НеупорядоченныйИтераторУзлов`/`UnorderedNodeIterator` и остальные
    // восемь — знает.
    XPathAny,
    XPathNumber,
    XPathString,
    XPathBoolean,
    XPathUnorderedNodeIterator,
    XPathOrderedNodeIterator,
    XPathUnorderedNodeSnapshot,
    XPathOrderedNodeSnapshot,
    XPathAnyUnorderedNode,
    XPathFirstOrderedNode,

    // --- НаправлениеПоиска -------------------------------------------------
    // Оба члена и оба их написания ИЗМЕРЕНЫ на 8.3.27:
    // `НаправлениеПоиска.СНачала` печатается «С начала», `.СКонца` — «С
    // конца», а `SearchDirection.FromBegin`/`.FromEnd` платформа принимает
    // и считает теми же значениями. Третьего члена нет: `.СЛева` — «Поле
    // объекта не обнаружено».
    SearchFromBegin,
    SearchFromEnd,

    // --- РежимВосстановленияПутейФайловZIP ----------------------------------
    // Членов ровно два: `.НетТакого` — «Поле объекта не обнаружено»
    // (измерено).
    RestorePaths,
    DontRestorePaths,

    // --- ТипСвязиВложенияPDF ------------------------------------------------
    // Пять членов, снятых перебором. Спецификация PDF (таблица 43) знает
    // ещё `Schema`, `FormData` и `EncryptedPayload` — платформа их НЕ
    // знает, поэтому их нет и здесь: член перечисления, которого у неё
    // нет, был бы неправдой.
    PdfRelationSource,
    PdfRelationData,
    PdfRelationAlternative,
    PdfRelationSupplement,
    PdfRelationUnspecified,

    // --- ТипФайлаАрхива -----------------------------------------------------
    // Семь членов, снятых перебором; из них поддержан только `Zip`.
    ArchiveTypeZip,
    ArchiveTypeBzip2,
    ArchiveTypeGzip,
    ArchiveTypeRar,
    ArchiveTypeSevenZip,
    ArchiveTypeTar,
    ArchiveTypeXz,

    // --- МетодСжатияZIP -----------------------------------------------------
    // Три члена; `Deflate64`, `LZMA` и `PPMd` — «Поле объекта не
    // обнаружено» (проверено перебором).
    ZipMethodDeflate,
    ZipMethodCopy,
    ZipMethodBzip2,

    // --- УровеньСжатияZIP ---------------------------------------------------
    // Ровно три: ни `БезСжатия`, ни `Нет`, ни `Никакой`, ни `Отсутствует`,
    // ни `Быстрый` платформа не знает.
    ZipLevelMinimal,
    ZipLevelOptimal,
    ZipLevelMaximal,

    // --- РежимСохраненияПутейZIP --------------------------------------------
    // Имя третьего члена — `НеСохранятьПути`, а не `НеСохранять`
    // (измерено: второе — «Поле объекта не обнаружено»).
    ZipStoreRelativePath,
    ZipStoreFullPath,
    ZipDontStorePath,

    // --- РежимОбработкиПодкаталоговZIP ---------------------------------------
    ZipDontProcessSubdirs,
    ZipProcessSubdirsRecursively,

    // --- МетодШифрованияZIP --------------------------------------------------
    // Четыре члена, представление каждого измерено — `AES192` отдельной
    // пробой, потому что между 128 и 256 он не подразумевается. Шифрования
    // здесь нет ни одного: члены заведены для честного отказа вместо
    // молчаливо открытого архива.
    ZipEncryptionAes128,
    ZipEncryptionAes192,
    ZipEncryptionAes256,
    ZipEncryptionZip20,

    // --- КодировкаИменФайловВZipФайле -----------------------------------------
    // Два члена: `КодировкаОС`, `OEM` и `ANSI` платформа не знает.
    ZipNamesAuto,
    ZipNamesUtf8,

    // --- ИспользованиеByteOrderMark ------------------------------------
    ByteOrderMarkUse,
    ByteOrderMarkDoNotUse,
}

/// К какому перечислению принадлежит член — это же имя стоит слева от
/// точки в исходном тексте.
///
/// `Ord` нужен не самому перечислению, а несущему варианту
/// `TypeId::EnumMeta(EnumKind)`: `TypeId` упорядочен целиком.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum EnumKind {
    /// Основной язык программирования конфигурации.
    ScriptVariant,
    /// Режим совместимости конфигурации с версией платформы.
    CompatibilityMode,
    /// Режим запуска клиентского приложения.
    ClientRunMode,
    /// Источник списка расширений конфигурации.
    ConfigurationExtensionsSource,
    /// Область действия расширения конфигурации.
    ConfigurationExtensionScope,
    /// Важность проблемы применения расширения конфигурации.
    ConfigurationExtensionApplicationIssueSeverity,
    /// Уровень важности события журнала регистрации.
    EventLogLevel,
    /// Связь записи журнала регистрации с текущей транзакцией.
    EventLogEntryTransactionMode,
    /// Способ отправки отчёта об ошибке.
    ErrorReportingMode,
    /// Действие над элементом данных при обмене.
    DataItemSend,
    /// Действие над полученным элементом данных при обмене.
    DataItemReceive,
    /// Режим обновления предопределённых данных.
    PredefinedDataUpdate,
    /// Режим проведения документа.
    DocumentPostingMode,
    /// Операция сравнения в настройках системы компоновки данных.
    DataCompositionComparisonType,
    /// Относительный период либо произвольная стандартная дата начала.
    StandardBeginningDateVariant,
    JsonValueType,
    JsonLineBreak,
    /// Способ перевода Unicode-символов в `\uXXXX` при записи JSON.
    JsonEscapeCharacters,
    JsonDateFormat,
    /// `ВариантЗаписиДатыJSON` — англ. написание `JSONDateWritingVariant`
    /// взято по аналогии с остальными именами перечислений JSON (`JSON` +
    /// суть без пробелов), само не измерялось отдельно; написания ЧЛЕНОВ
    /// ниже — ИЗМЕРЕНО (`JSON.DATE_VARIANT_EN_NAMES`).
    JsonDateWritingVariant,
    /// `СостояниеФоновогоЗадания` — состояния фонового задания. Русские
    /// имена членов взяты из документации главы 18; точные написания,
    /// английские aliases и представления `Строка()` фиксирует замер
    /// `JOB.API.SURFACE` реестра плана фоновых заданий.
    BackgroundJobState,
    /// `СтатусСообщения` — статусы `Сообщить`; состав членов — по выписке
    /// синтакс-помощника 8.3.27. В управляемом режиме статус игнорируется,
    /// перечисление нужно компиляции вызовов.
    MessageStatus,
    /// `КатегорияОшибки` — категории `ЯвляетсяОшибкойКатегории`; состав —
    /// по выписке синтакс-помощника 8.3.27.
    ErrorCategory,
    XmlNodeType,
    /// `ТипУзлаDOM`. Английское написание `DOMNodeType` ИЗМЕРЕНО:
    /// `Тип("DOMNodeType") = ТипЗнч(ТипУзлаDOM.Элемент)` — «Да».
    DomNodeType,
    SpreadFileType,
    DrawingKind,
    /// `ОриентацияСтраницы` — свойство `ТабличныйДокумент.ОриентацияСтраницы`
    /// и пара с идентификатором 1 в параметрах страницы MXL.
    PageOrientation,
    TextEncoding,
    /// Два режима percent-кодирования строки для query и полного URL.
    StringEncodingMethod,
    /// Направление сортировки коллекций; Connector использует возрастание.
    SortDirection,
    /// Какие части даты сохраняет `КвалификаторыДаты`.
    DateFractions,
    /// Ограничение длины для `КвалификаторыСтроки`.
    AllowedLength,
    /// Алгоритм объекта `ХешированиеДанных`.
    HashFunction,
    /// `ПорядокБайтов` — порядок байтов многобайтового целого в
    /// `БуферДвоичныхДанных`. Английское написание `ByteOrder` ИЗМЕРЕНО
    /// (`ByteOrder.LittleEndian` платформа принимает и считает равным
    /// русскому написанию), метатип — «ПеречислениеПорядокБайтов», тоже
    /// измерено.
    ByteOrder,
    /// `РежимОткрытияФайла` — второй аргумент `Новый ФайловыйПоток`.
    /// Английское написание `FileOpenMode` и написания всех шести членов
    /// ИЗМЕРЕНЫ (`FileOpenMode.Open = РежимОткрытияФайла.Открыть` —
    /// «Да»), метатип — «ПеречислениеРежимОткрытияФайла», тоже измерено.
    FileOpenMode,
    /// `ДоступКФайлу` — третий аргумент `Новый ФайловыйПоток`. Написание
    /// члена `ЧтениеИЗапись` по-английски — `ReadAndWrite`, а не
    /// `ReadWrite`: измерено.
    FileAccess,
    /// `ПозицияВПотоке` — точка отсчёта у `Перейти`. Английское написание
    /// самого перечисления — `PositionInStream`, а не `StreamPosition`:
    /// измерено.
    StreamPosition,

    // --- перечисления объектной модели XML-схемы ------------------------
    // Английские написания САМИХ перечислений измерены через `Тип(...)`:
    // `XSComponentType`, `XSForm`, `XSSimpleTypeVariety`,
    // `XSDerivationMethod`, `XSWhitespaceHandling` — есть, а вот у
    // `ВидГруппыМоделиXS` и `ОграничениеЗначенияXS` английского имени НЕТ
    // (`XSModelGroupType` и `XSValueConstraint` платформа не знает), и
    // тогда вторым написанием стоит то же русское.
    /// `ТипКомпонентыXS` — вид компоненты схемы; его отдаёт свойство
    /// `ТипКомпоненты` у каждой компоненты.
    XsComponentType,
    /// `ФормаПредставленияXS` — `elementFormDefault`/`form`.
    XsForm,
    /// `ВариантПростогоТипаXS` — `Атомарная`/`Список`/`Объединение`.
    XsSimpleTypeVariety,
    /// `ВидГруппыМоделиXS` — `Последовательность`/`Выбор`/`Все`.
    XsModelGroupKind,
    /// `МетодНаследованияXS` — `Расширение`/`Ограничение`.
    XsDerivationMethod,
    /// `ОграничениеЗначенияXS` — вид значения по умолчанию у объявления.
    XsValueConstraint,
    /// `ОбработкаПробельныхСимволовXS` — значение фасета `whiteSpace`.
    XsWhitespaceHandling,
    /// `ФормаXML` — `Свойство.Форма` модели типов XDTO: `Элемент`,
    /// `Атрибут`, `Текст`. Английские написания есть и у самого
    /// перечисления (`XMLForm`), и у всех трёх членов (измерено).
    XmlForm,
    /// `ВидФасетаXDTO` — `Фасет.Вид`. Русские написания членов измерены
    /// поимённо и НЕ выводятся из представлений: у члена с
    /// представлением «Минимальная длина» имя `МинДлина`, а
    /// `МинимальнаяДлина` платформа отвергает.
    XdtoFacetKind,
    /// `ТипРезультатаDOMXPath` — вид результата `ВычислитьВыражениеXPath`;
    /// его же принимает необязательный четвёртый аргумент. Английское
    /// написание `DOMXPathResultType` ИЗМЕРЕНО (`XPathResultType` и
    /// `ТипРезультатаXPathDOM` платформа не знает).
    DomXPathResultType,
    /// `НаправлениеПоиска` — третий аргумент
    /// `СтрНайтиПоРегулярномуВыражению`. Английское написание
    /// `SearchDirection` ИЗМЕРЕНО, как и то, что `Тип("НаправлениеПоиска")`
    /// и `Тип("SearchDirection")` дают один и тот же тип.
    SearchDirection,

    // --- перечисления чтения архивов -------------------------------------
    /// `РежимВосстановленияПутейФайловZIP` — необязательный аргумент
    /// `Извлечь` и `ИзвлечьВсе`. Английское написание
    /// `ZIPRestoreFilePathsMode` ИЗМЕРЕНО (`ZIPRestorePathsMode` платформа
    /// не знает), как и то, что членов ровно два.
    ZipRestorePathsMode,
    /// `ТипСвязиВложенияPDF` — четвёртый аргумент
    /// `КоллекцияВложенийPDF.Добавить` и свойство `ВложениеPDF.ТипСвязи`.
    /// Английского написания у самого перечисления НЕТ:
    /// `PDFAttachmentRelationship`, `СвязьВложенияPDF` и
    /// `ОтношениеВложенияPDF` платформа не знает (проверено перебором), а
    /// у членов оно есть.
    PdfAttachmentRelation,
    /// `ТипФайлаАрхива` — третий аргумент `Новый ЧтениеФайлаАрхива`.
    /// Английское написание `ArchiveFileType` ИЗМЕРЕНО. Членов семь, и
    /// читаем мы ровно один из них: остальные шесть заведены для того,
    /// чтобы `ЧтениеФайлаАрхива(файл, , ТипФайлаАрхива.TAR)` отвечал
    /// «формат не поддерживается», а не разбирал TAR как ZIP.
    ArchiveFileType,

    // --- перечисления записи архивов ---------------------------------------
    /// `МетодСжатияZIP` — четвёртый аргумент `Новый ЗаписьZipФайла`.
    /// Английское написание `ZIPCompressionMethod` ИЗМЕРЕНО.
    ZipCompressionMethod,
    /// `УровеньСжатияZIP` — пятый аргумент того же конструктора.
    ZipCompressionLevel,
    /// `РежимСохраненияПутейZIP` — второй аргумент `Добавить`.
    ZipStorePathMode,
    /// `РежимОбработкиПодкаталоговZIP` — третий аргумент `Добавить`.
    ZipSubDirProcessingMode,
    /// `МетодШифрованияZIP` — шестой аргумент конструктора; шифрования
    /// здесь нет, члены нужны для отказа.
    ZipEncryptionMethod,
    /// `КодировкаИменФайловВZipФайле` — седьмой аргумент конструктора.
    ZipFileNamesEncoding,
    /// Управляет сигнатурой при записи строкового HTTP-тела.
    ByteOrderMarkUse,
}

impl EnumKind {
    /// Написания самого перечисления: `(русское, английское)`.
    const fn names(self) -> (&'static str, &'static str) {
        match self {
            EnumKind::ScriptVariant => ("ВариантВстроенногоЯзыка", "ScriptVariant"),
            EnumKind::CompatibilityMode => ("РежимСовместимости", "CompatibilityMode"),
            EnumKind::ClientRunMode => ("РежимЗапускаКлиентскогоПриложения", "ClientRunMode"),
            EnumKind::ConfigurationExtensionsSource => (
                "ИсточникРасширенийКонфигурации",
                "ConfigurationExtensionsSource",
            ),
            EnumKind::ConfigurationExtensionScope => (
                "ОбластьДействияРасширенияКонфигурации",
                "ConfigurationExtensionScope",
            ),
            EnumKind::ConfigurationExtensionApplicationIssueSeverity => (
                "ВажностьПроблемыПримененияРасширенияКонфигурации",
                "ConfigurationExtensionApplicationIssueSeverity",
            ),
            EnumKind::EventLogLevel => ("УровеньЖурналаРегистрации", "EventLogLevel"),
            EnumKind::EventLogEntryTransactionMode => (
                "РежимТранзакцииЗаписиЖурналаРегистрации",
                "EventLogEntryTransactionMode",
            ),
            EnumKind::ErrorReportingMode => {
                ("РежимОтправкиИнформацииОбОшибке", "ErrorReportingMode")
            }
            EnumKind::DataItemSend => ("ОтправкаЭлементаДанных", "DataItemSend"),
            EnumKind::DataItemReceive => ("ПолучениеЭлементаДанных", "DataItemReceive"),
            EnumKind::PredefinedDataUpdate => {
                ("ОбновлениеПредопределенныхДанных", "PredefinedDataUpdate")
            }
            EnumKind::DocumentPostingMode => ("РежимПроведенияДокумента", "DocumentPostingMode"),
            EnumKind::DataCompositionComparisonType => (
                "ВидСравненияКомпоновкиДанных",
                "DataCompositionComparisonType",
            ),
            EnumKind::StandardBeginningDateVariant => (
                "ВариантСтандартнойДатыНачала",
                "StandardBeginningDateVariant",
            ),
            EnumKind::JsonValueType => ("ТипЗначенияJSON", "JSONValueType"),
            EnumKind::JsonLineBreak => ("ПереносСтрокJSON", "JSONLineBreak"),
            EnumKind::JsonEscapeCharacters => {
                ("ЭкранированиеСимволовJSON", "JSONCharactersEscapeMode")
            }
            EnumKind::JsonDateFormat => ("ФорматДатыJSON", "JSONDateFormat"),
            EnumKind::JsonDateWritingVariant => ("ВариантЗаписиДатыJSON", "JSONDateWritingVariant"),
            EnumKind::XmlNodeType => ("ТипУзлаXML", "XMLNodeType"),
            EnumKind::DomNodeType => ("ТипУзлаDOM", "DOMNodeType"),
            EnumKind::SpreadFileType => {
                ("ТипФайлаТабличногоДокумента", "SpreadsheetDocumentFileType")
            }
            EnumKind::DrawingKind => (
                "ТипРисункаТабличногоДокумента",
                "SpreadsheetDocumentDrawingType",
            ),
            EnumKind::PageOrientation => ("ОриентацияСтраницы", "PageOrientation"),
            EnumKind::TextEncoding => ("КодировкаТекста", "TextEncoding"),
            EnumKind::StringEncodingMethod => {
                ("СпособКодированияСтроки", "СпособКодированияСтроки")
            }
            EnumKind::SortDirection => ("НаправлениеСортировки", "SortDirection"),
            EnumKind::BackgroundJobState => ("СостояниеФоновогоЗадания", "BackgroundJobState"),
            EnumKind::MessageStatus => ("СтатусСообщения", "MessageStatus"),
            EnumKind::ErrorCategory => ("КатегорияОшибки", "ErrorCategory"),
            EnumKind::DateFractions => ("ЧастиДаты", "DateFractions"),
            EnumKind::AllowedLength => ("ДопустимаяДлина", "AllowedLength"),
            EnumKind::HashFunction => ("ХешФункция", "HashFunction"),
            EnumKind::ByteOrder => ("ПорядокБайтов", "ByteOrder"),
            EnumKind::FileOpenMode => ("РежимОткрытияФайла", "FileOpenMode"),
            EnumKind::FileAccess => ("ДоступКФайлу", "FileAccess"),
            EnumKind::StreamPosition => ("ПозицияВПотоке", "PositionInStream"),
            EnumKind::XsComponentType => ("ТипКомпонентыXS", "XSComponentType"),
            EnumKind::XsForm => ("ФормаПредставленияXS", "XSForm"),
            EnumKind::XsSimpleTypeVariety => ("ВариантПростогоТипаXS", "XSSimpleTypeVariety"),
            EnumKind::XsModelGroupKind => ("ВидГруппыМоделиXS", "ВидГруппыМоделиXS"),
            EnumKind::XsDerivationMethod => ("МетодНаследованияXS", "XSDerivationMethod"),
            EnumKind::XsValueConstraint => ("ОграничениеЗначенияXS", "ОграничениеЗначенияXS"),
            EnumKind::XsWhitespaceHandling => {
                ("ОбработкаПробельныхСимволовXS", "XSWhitespaceHandling")
            }
            EnumKind::XmlForm => ("ФормаXML", "XMLForm"),
            EnumKind::XdtoFacetKind => ("ВидФасетаXDTO", "XDTOFacetType"),
            EnumKind::DomXPathResultType => ("ТипРезультатаDOMXPath", "DOMXPathResultType"),
            EnumKind::SearchDirection => ("НаправлениеПоиска", "SearchDirection"),
            EnumKind::ZipRestorePathsMode => (
                "РежимВосстановленияПутейФайловZIP",
                "ZIPRestoreFilePathsMode",
            ),
            EnumKind::PdfAttachmentRelation => ("ТипСвязиВложенияPDF", "ТипСвязиВложенияPDF"),
            EnumKind::ArchiveFileType => ("ТипФайлаАрхива", "ArchiveFileType"),
            EnumKind::ZipCompressionMethod => ("МетодСжатияZIP", "ZIPCompressionMethod"),
            EnumKind::ZipCompressionLevel => ("УровеньСжатияZIP", "ZIPCompressionLevel"),
            EnumKind::ZipStorePathMode => ("РежимСохраненияПутейZIP", "ZIPStorePathMode"),
            EnumKind::ZipSubDirProcessingMode => {
                ("РежимОбработкиПодкаталоговZIP", "ZIPSubDirProcessingMode")
            }
            EnumKind::ZipEncryptionMethod => ("МетодШифрованияZIP", "ZIPEncryptionMethod"),
            // Английское написание тут не по образцу соседей, а измерено:
            // `Тип("FileNamesEncodingInZipFile")` даёт это перечисление, а
            // `ZipFileNamesEncodingMode`, `ZIPFileNamesEncodingMode` и
            // `ZipFileNamesEncoding` — «Тип не определен».
            EnumKind::ZipFileNamesEncoding => {
                ("КодировкаИменФайловВZipФайле", "FileNamesEncodingInZipFile")
            }
            EnumKind::ByteOrderMarkUse => ("ИспользованиеByteOrderMark", "ByteOrderMarkUse"),
        }
    }

    /// Имя МЕТАТИПА перечисления — то, что печатают `Строка()` и
    /// `Строка(ТипЗнч())` от ГОЛОГО имени перечисления как выражения.
    /// Для `ВариантЗаписиДатыJSON` ИЗМЕРЕНО:
    /// `ПеречислениеВариантЗаписиДатыJSON` — префикс `Перечисление` плюс
    /// русское написание, слитно. `НЕ ИЗМЕРЕНО(JSON.ENUM.BARE_NAME)`:
    /// распространение того же префикса на остальные перечисления и
    /// английское написание метатипа — предположение по образцу.
    pub const fn meta_ru_name(self) -> &'static str {
        match self {
            EnumKind::ScriptVariant => "ПеречислениеВариантВстроенногоЯзыка",
            EnumKind::CompatibilityMode => "ПеречислениеРежимСовместимости",
            // ИЗМЕРЕНО на 8.3.27 вместе с членами перечисления.
            EnumKind::ClientRunMode => "EnumClientRunMode",
            // ИЗМЕРЕНО на 8.3.27 пробой `measure-configuration-extension-enums.bsl`.
            EnumKind::ConfigurationExtensionsSource => "EnumConfigurationExtensionsSource",
            EnumKind::ConfigurationExtensionScope => "EnumConfigurationExtensionScope",
            EnumKind::ConfigurationExtensionApplicationIssueSeverity => {
                "EnumConfigurationExtensionApplicationIssueSeverity"
            }
            // ИЗМЕРЕНО на 8.3.27 пробой `measure-event-log.bsl`.
            EnumKind::EventLogLevel => "EnumEventLogLevel",
            EnumKind::EventLogEntryTransactionMode => "EnumEventLogEntryTransactionMode",
            EnumKind::ErrorReportingMode => "ErrorReportingMode",
            EnumKind::DataItemSend => "DataItemSend",
            EnumKind::DataItemReceive => "DataItemReceive",
            EnumKind::PredefinedDataUpdate => "PredefinedDataUpdate",
            EnumKind::DocumentPostingMode => "DocumentPostingMode",
            EnumKind::DataCompositionComparisonType => "DataCompositionComparisonType",
            EnumKind::StandardBeginningDateVariant => "StandardBeginningDateVariant",
            EnumKind::JsonValueType => "ПеречислениеТипЗначенияJSON",
            EnumKind::JsonLineBreak => "ПеречислениеПереносСтрокJSON",
            EnumKind::JsonEscapeCharacters => "ПеречислениеЭкранированиеСимволовJSON",
            EnumKind::JsonDateFormat => "ПеречислениеФорматДатыJSON",
            EnumKind::JsonDateWritingVariant => "ПеречислениеВариантЗаписиДатыJSON",
            EnumKind::XmlNodeType => "ПеречислениеТипУзлаXML",
            EnumKind::DomNodeType => "ПеречислениеТипУзлаDOM",
            EnumKind::SpreadFileType => "ПеречислениеТипФайлаТабличногоДокумента",
            EnumKind::DrawingKind => "ПеречислениеТипРисункаТабличногоДокумента",
            // ИЗМЕРЕНО, а не достроено по образцу: `Строка(ТипЗнч(
            // ОриентацияСтраницы))` на 8.3.27 даёт «ПеречислениеОриентацияСтраницы».
            EnumKind::PageOrientation => "ПеречислениеОриентацияСтраницы",
            EnumKind::TextEncoding => "ПеречислениеКодировкаТекста",
            EnumKind::StringEncodingMethod => "ПеречислениеСпособКодированияСтроки",
            EnumKind::SortDirection => "ПеречислениеНаправлениеСортировки",
            EnumKind::BackgroundJobState => "ПеречислениеСостояниеФоновогоЗадания",
            EnumKind::MessageStatus => "ПеречислениеСтатусСообщения",
            EnumKind::ErrorCategory => "ПеречислениеКатегорияОшибки",
            EnumKind::DateFractions => "ПеречислениеЧастиДаты",
            EnumKind::AllowedLength => "AllowedLength",
            EnumKind::HashFunction => "ПеречислениеХешФункция",
            // Здесь префикс не предположение по образцу, а ИЗМЕРЕНО:
            // `Строка(ПорядокБайтов)` и `Строка(ТипЗнч(ПорядокБайтов))`
            // оба дают «ПеречислениеПорядокБайтов».
            EnumKind::ByteOrder => "ПеречислениеПорядокБайтов",
            // Тоже ИЗМЕРЕНО, а не достроено по образцу: фикстура
            // `binary-streams` печатает и голое имя, и его `ТипЗнч()` для
            // всех трёх перечислений потоков.
            EnumKind::FileOpenMode => "ПеречислениеРежимОткрытияФайла",
            EnumKind::FileAccess => "ПеречислениеДоступКФайлу",
            EnumKind::StreamPosition => "ПеречислениеПозицияВПотоке",
            // Перечисления модели схемы — тот же префикс по образцу, под
            // тем же маркером `JSON.ENUM.BARE_NAME`: голое имя ни у одного
            // из них не мерилось.
            EnumKind::XsComponentType => "ПеречислениеТипКомпонентыXS",
            EnumKind::XsForm => "ПеречислениеФормаПредставленияXS",
            EnumKind::XsSimpleTypeVariety => "ПеречислениеВариантПростогоТипаXS",
            EnumKind::XsModelGroupKind => "ПеречислениеВидГруппыМоделиXS",
            EnumKind::XsDerivationMethod => "ПеречислениеМетодНаследованияXS",
            EnumKind::XsValueConstraint => "ПеречислениеОграничениеЗначенияXS",
            EnumKind::XsWhitespaceHandling => "ПеречислениеОбработкаПробельныхСимволовXS",
            EnumKind::XmlForm => "ПеречислениеФормаXML",
            EnumKind::XdtoFacetKind => "ПеречислениеВидФасетаXDTO",
            // ИЗМЕРЕНО, а не достроено по образцу: `Строка(ТипРезультатаDOMXPath)`
            // и `Строка(ТипЗнч(ТипРезультатаDOMXPath))` оба дают
            // «ПеречислениеТипРезультатаDOMXPath».
            EnumKind::DomXPathResultType => "ПеречислениеТипРезультатаDOMXPath",
            // Тоже ИЗМЕРЕНО: `Строка(ТипЗнч(НаправлениеПоиска))` даёт
            // «ПеречислениеНаправлениеПоиска».
            EnumKind::SearchDirection => "ПеречислениеНаправлениеПоиска",
            // Тот же префикс по образцу и под тем же маркером
            // `JSON.ENUM.BARE_NAME`: голое имя ни у одного из двух не
            // мерилось.
            EnumKind::ZipRestorePathsMode => "ПеречислениеРежимВосстановленияПутейФайловZIP",
            EnumKind::PdfAttachmentRelation => "ПеречислениеТипСвязиВложенияPDF",
            EnumKind::ArchiveFileType => "ПеречислениеТипФайлаАрхива",
            EnumKind::ZipCompressionMethod => "ПеречислениеМетодСжатияZIP",
            EnumKind::ZipCompressionLevel => "ПеречислениеУровеньСжатияZIP",
            EnumKind::ZipStorePathMode => "ПеречислениеРежимСохраненияПутейZIP",
            EnumKind::ZipSubDirProcessingMode => "ПеречислениеРежимОбработкиПодкаталоговZIP",
            EnumKind::ZipEncryptionMethod => "ПеречислениеМетодШифрованияZIP",
            EnumKind::ZipFileNamesEncoding => "ПеречислениеКодировкаИменФайловВZipФайле",
            EnumKind::ByteOrderMarkUse => "ПеречислениеИспользованиеByteOrderMark",
        }
    }

    /// Русское написание самого перечисления — то, что стоит слева от
    /// точки в исходном тексте (`ВариантЗаписиДатыJSON.ЛокальнаяДата`) и то
    /// же, что печатает `EnumValue::enum_name` для любого его члена. Нужно
    /// снаружи модуля для `BslValue::EnumType` — голого имени перечисления
    /// как выражения (`Вычислить("ВариантЗаписиДатыJSON")`).
    pub fn ru_name(self) -> &'static str {
        self.names().0
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
        EnumKind::ScriptVariant,
        EnumValue::ScriptVariantEnglish,
        "Английский",
        "English",
        "Английский",
    ),
    (
        EnumKind::ScriptVariant,
        EnumValue::ScriptVariantRussian,
        "Русский",
        "Russian",
        "Русский",
    ),
    // Состав типа и все представления измерены на платформе 8.3.27.2342
    // через `Метаданные.СвойстваОбъектов.РежимСовместимости`.
    (
        EnumKind::CompatibilityMode,
        EnumValue::CompatibilityModeVersion8_1,
        "Версия8_1",
        "Version8_1",
        "Version8_1",
    ),
    (
        EnumKind::CompatibilityMode,
        EnumValue::CompatibilityModeVersion8_2_13,
        "Версия8_2_13",
        "Version8_2_13",
        "Version8_2_13",
    ),
    (
        EnumKind::CompatibilityMode,
        EnumValue::CompatibilityModeVersion8_2_16,
        "Версия8_2_16",
        "Version8_2_16",
        "Version8_2_16",
    ),
    (
        EnumKind::CompatibilityMode,
        EnumValue::CompatibilityModeVersion8_3_1,
        "Версия8_3_1",
        "Version8_3_1",
        "Version8_3_1",
    ),
    (
        EnumKind::CompatibilityMode,
        EnumValue::CompatibilityModeVersion8_3_2,
        "Версия8_3_2",
        "Version8_3_2",
        "Version8_3_2",
    ),
    (
        EnumKind::CompatibilityMode,
        EnumValue::CompatibilityModeVersion8_3_3,
        "Версия8_3_3",
        "Version8_3_3",
        "Version8_3_3",
    ),
    (
        EnumKind::CompatibilityMode,
        EnumValue::CompatibilityModeVersion8_3_4,
        "Версия8_3_4",
        "Version8_3_4",
        "Version8_3_4",
    ),
    (
        EnumKind::CompatibilityMode,
        EnumValue::CompatibilityModeVersion8_3_5,
        "Версия8_3_5",
        "Version8_3_5",
        "Version8_3_5",
    ),
    (
        EnumKind::CompatibilityMode,
        EnumValue::CompatibilityModeVersion8_3_6,
        "Версия8_3_6",
        "Version8_3_6",
        "Version8_3_6",
    ),
    (
        EnumKind::CompatibilityMode,
        EnumValue::CompatibilityModeVersion8_3_7,
        "Версия8_3_7",
        "Version8_3_7",
        "Version8_3_7",
    ),
    (
        EnumKind::CompatibilityMode,
        EnumValue::CompatibilityModeVersion8_3_8,
        "Версия8_3_8",
        "Version8_3_8",
        "Version8_3_8",
    ),
    (
        EnumKind::CompatibilityMode,
        EnumValue::CompatibilityModeVersion8_3_9,
        "Версия8_3_9",
        "Version8_3_9",
        "Version8_3_9",
    ),
    (
        EnumKind::CompatibilityMode,
        EnumValue::CompatibilityModeVersion8_3_10,
        "Версия8_3_10",
        "Version8_3_10",
        "Version8_3_10",
    ),
    (
        EnumKind::CompatibilityMode,
        EnumValue::CompatibilityModeVersion8_3_11,
        "Версия8_3_11",
        "Version8_3_11",
        "Version8_3_11",
    ),
    (
        EnumKind::CompatibilityMode,
        EnumValue::CompatibilityModeVersion8_3_12,
        "Версия8_3_12",
        "Version8_3_12",
        "Version8_3_12",
    ),
    (
        EnumKind::CompatibilityMode,
        EnumValue::CompatibilityModeVersion8_3_13,
        "Версия8_3_13",
        "Version8_3_13",
        "Version8_3_13",
    ),
    (
        EnumKind::CompatibilityMode,
        EnumValue::CompatibilityModeVersion8_3_14,
        "Версия8_3_14",
        "Version8_3_14",
        "Version8_3_14",
    ),
    (
        EnumKind::CompatibilityMode,
        EnumValue::CompatibilityModeVersion8_3_15,
        "Версия8_3_15",
        "Version8_3_15",
        "Version8_3_15",
    ),
    (
        EnumKind::CompatibilityMode,
        EnumValue::CompatibilityModeVersion8_3_16,
        "Версия8_3_16",
        "Version8_3_16",
        "Version8_3_16",
    ),
    (
        EnumKind::CompatibilityMode,
        EnumValue::CompatibilityModeVersion8_3_17,
        "Версия8_3_17",
        "Version8_3_17",
        "Version8_3_17",
    ),
    (
        EnumKind::CompatibilityMode,
        EnumValue::CompatibilityModeVersion8_3_18,
        "Версия8_3_18",
        "Version8_3_18",
        "Version8_3_18",
    ),
    (
        EnumKind::CompatibilityMode,
        EnumValue::CompatibilityModeVersion8_3_19,
        "Версия8_3_19",
        "Version8_3_19",
        "Version8_3_19",
    ),
    (
        EnumKind::CompatibilityMode,
        EnumValue::CompatibilityModeVersion8_3_20,
        "Версия8_3_20",
        "Version8_3_20",
        "Version8_3_20",
    ),
    (
        EnumKind::CompatibilityMode,
        EnumValue::CompatibilityModeVersion8_3_21,
        "Версия8_3_21",
        "Version8_3_21",
        "Version8_3_21",
    ),
    (
        EnumKind::CompatibilityMode,
        EnumValue::CompatibilityModeVersion8_3_22,
        "Версия8_3_22",
        "Version8_3_22",
        "Version8_3_22",
    ),
    (
        EnumKind::CompatibilityMode,
        EnumValue::CompatibilityModeVersion8_3_23,
        "Версия8_3_23",
        "Version8_3_23",
        "Version8_3_23",
    ),
    (
        EnumKind::CompatibilityMode,
        EnumValue::CompatibilityModeVersion8_3_24,
        "Версия8_3_24",
        "Version8_3_24",
        "Version8_3_24",
    ),
    (
        EnumKind::CompatibilityMode,
        EnumValue::CompatibilityModeVersion8_3_25,
        "Версия8_3_25",
        "Version8_3_25",
        "Version8_3_25",
    ),
    (
        EnumKind::CompatibilityMode,
        EnumValue::CompatibilityModeVersion8_3_26,
        "Версия8_3_26",
        "Version8_3_26",
        "Version8_3_26",
    ),
    (
        EnumKind::CompatibilityMode,
        EnumValue::CompatibilityModeDontUse,
        "НеИспользовать",
        "DontUse",
        "DontUse",
    ),
    // Имена, метатипы и представления измерены на платформе 8.3.27.2342
    // пробой `measure-event-log.bsl`.
    (
        EnumKind::EventLogLevel,
        EnumValue::EventLogLevelNote,
        "Примечание",
        "Note",
        "Note",
    ),
    (
        EnumKind::EventLogLevel,
        EnumValue::EventLogLevelInformation,
        "Информация",
        "Information",
        "Information",
    ),
    (
        EnumKind::EventLogLevel,
        EnumValue::EventLogLevelWarning,
        "Предупреждение",
        "Warning",
        "Warning",
    ),
    (
        EnumKind::EventLogLevel,
        EnumValue::EventLogLevelError,
        "Ошибка",
        "Error",
        "Error",
    ),
    (
        EnumKind::EventLogEntryTransactionMode,
        EnumValue::EventLogEntryTransactionModeIndependent,
        "Независимая",
        "Independent",
        "Independent",
    ),
    (
        EnumKind::EventLogEntryTransactionMode,
        EnumValue::EventLogEntryTransactionModeTransactional,
        "Транзакционная",
        "Transactional",
        "Transactional",
    ),
    // Состав, оба написания и представления измерены на платформе 8.3.27.2342
    // пробой `measure-error-processing.bsl`.
    (
        EnumKind::ErrorReportingMode,
        EnumValue::ErrorReportingModeAuto,
        "Авто",
        "Auto",
        "Auto",
    ),
    (
        EnumKind::ErrorReportingMode,
        EnumValue::ErrorReportingModeDontSend,
        "НеОтправлять",
        "DontSend",
        "Do not send",
    ),
    (
        EnumKind::ErrorReportingMode,
        EnumValue::ErrorReportingModeAskUser,
        "СпрашиватьПользователя",
        "AskUser",
        "Ask user",
    ),
    (
        EnumKind::ErrorReportingMode,
        EnumValue::ErrorReportingModeSend,
        "Отправлять",
        "Send",
        "Send",
    ),
    // Состав, оба написания и представления измерены на платформе 8.3.27.2074
    // пробой `measure-data-item-send.bsl`.
    (
        EnumKind::DataItemSend,
        EnumValue::DataItemSendAuto,
        "Авто",
        "Auto",
        "Auto",
    ),
    (
        EnumKind::DataItemSend,
        EnumValue::DataItemSendDelete,
        "Удалить",
        "Delete",
        "Delete",
    ),
    (
        EnumKind::DataItemSend,
        EnumValue::DataItemSendIgnore,
        "Игнорировать",
        "Ignore",
        "Ignore",
    ),
    // Состав, оба написания и представления измерены на платформе 8.3.27.2074
    // пробой `measure-data-item-receive.bsl`.
    (
        EnumKind::DataItemReceive,
        EnumValue::DataItemReceiveAuto,
        "Авто",
        "Auto",
        "Auto",
    ),
    (
        EnumKind::DataItemReceive,
        EnumValue::DataItemReceiveAccept,
        "Принять",
        "Accept",
        "Accept",
    ),
    (
        EnumKind::DataItemReceive,
        EnumValue::DataItemReceiveIgnore,
        "Игнорировать",
        "Ignore",
        "Ignore",
    ),
    // Состав, оба написания и представления измерены на платформе 8.3.27.2342
    // пробой `measure-infobase-predefined-data-update.bsl`.
    (
        EnumKind::PredefinedDataUpdate,
        EnumValue::PredefinedDataUpdateAuto,
        "Авто",
        "Auto",
        "Auto",
    ),
    (
        EnumKind::PredefinedDataUpdate,
        EnumValue::PredefinedDataUpdateAutoUpdate,
        "ОбновлятьАвтоматически",
        "AutoUpdate",
        "Auto update",
    ),
    (
        EnumKind::PredefinedDataUpdate,
        EnumValue::PredefinedDataUpdateDontAutoUpdate,
        "НеОбновлятьАвтоматически",
        "DontAutoUpdate",
        "Auto update off",
    ),
    (
        EnumKind::DocumentPostingMode,
        EnumValue::DocumentPostingModeOperational,
        "Оперативный",
        "RealTime",
        "Real-time",
    ),
    (
        EnumKind::DocumentPostingMode,
        EnumValue::DocumentPostingModeNonOperational,
        "Неоперативный",
        "Regular",
        "Regular",
    ),
    // Состав, оба написания и представления измерены на платформе 8.3.27.2074
    // пробой `measure-data-composition-comparison-type.bsl`.
    (
        EnumKind::DataCompositionComparisonType,
        EnumValue::DataCompositionComparisonBeginsWith,
        "НачинаетсяС",
        "BeginsWith",
        "Begins with",
    ),
    (
        EnumKind::DataCompositionComparisonType,
        EnumValue::DataCompositionComparisonContains,
        "Содержит",
        "Contains",
        "Contains",
    ),
    (
        EnumKind::DataCompositionComparisonType,
        EnumValue::DataCompositionComparisonEqual,
        "Равно",
        "Equal",
        "Equal to",
    ),
    (
        EnumKind::DataCompositionComparisonType,
        EnumValue::DataCompositionComparisonFilled,
        "Заполнено",
        "Filled",
        "Filled",
    ),
    (
        EnumKind::DataCompositionComparisonType,
        EnumValue::DataCompositionComparisonGreater,
        "Больше",
        "Greater",
        "Greater than",
    ),
    (
        EnumKind::DataCompositionComparisonType,
        EnumValue::DataCompositionComparisonGreaterOrEqual,
        "БольшеИлиРавно",
        "GreaterOrEqual",
        "Greater than or equal to",
    ),
    (
        EnumKind::DataCompositionComparisonType,
        EnumValue::DataCompositionComparisonInHierarchy,
        "ВИерархии",
        "InHierarchy",
        "In group",
    ),
    (
        EnumKind::DataCompositionComparisonType,
        EnumValue::DataCompositionComparisonInList,
        "ВСписке",
        "InList",
        "In list",
    ),
    (
        EnumKind::DataCompositionComparisonType,
        EnumValue::DataCompositionComparisonInListByHierarchy,
        "ВСпискеПоИерархии",
        "InListByHierarchy",
        "In list group",
    ),
    (
        EnumKind::DataCompositionComparisonType,
        EnumValue::DataCompositionComparisonLess,
        "Меньше",
        "Less",
        "Less than",
    ),
    (
        EnumKind::DataCompositionComparisonType,
        EnumValue::DataCompositionComparisonLessOrEqual,
        "МеньшеИлиРавно",
        "LessOrEqual",
        "Less than or equal to",
    ),
    (
        EnumKind::DataCompositionComparisonType,
        EnumValue::DataCompositionComparisonLike,
        "Подобно",
        "Like",
        "Like",
    ),
    (
        EnumKind::DataCompositionComparisonType,
        EnumValue::DataCompositionComparisonNotBeginsWith,
        "НеНачинаетсяС",
        "NotBeginsWith",
        "Does not begin with",
    ),
    (
        EnumKind::DataCompositionComparisonType,
        EnumValue::DataCompositionComparisonNotContains,
        "НеСодержит",
        "NotContains",
        "Does not contain",
    ),
    (
        EnumKind::DataCompositionComparisonType,
        EnumValue::DataCompositionComparisonNotEqual,
        "НеРавно",
        "NotEqual",
        "Not equal",
    ),
    (
        EnumKind::DataCompositionComparisonType,
        EnumValue::DataCompositionComparisonNotFilled,
        "НеЗаполнено",
        "NotFilled",
        "Empty",
    ),
    (
        EnumKind::DataCompositionComparisonType,
        EnumValue::DataCompositionComparisonNotInHierarchy,
        "НеВИерархии",
        "NotInHierarchy",
        "Not in group",
    ),
    (
        EnumKind::DataCompositionComparisonType,
        EnumValue::DataCompositionComparisonNotInList,
        "НеВСписке",
        "NotInList",
        "Not in list",
    ),
    (
        EnumKind::DataCompositionComparisonType,
        EnumValue::DataCompositionComparisonNotInListByHierarchy,
        "НеВСпискеПоИерархии",
        "NotInListByHierarchy",
        "Not in list group",
    ),
    (
        EnumKind::DataCompositionComparisonType,
        EnumValue::DataCompositionComparisonNotLike,
        "НеПодобно",
        "NotLike",
        "Not like",
    ),
    // Состав, оба написания и представления измерены на платформе 8.3.27.2074
    // пробой `measure-standard-beginning-date.bsl`.
    (
        EnumKind::StandardBeginningDateVariant,
        EnumValue::StandardBeginningDateThisDay,
        "НачалоЭтогоДня",
        "BeginningOfThisDay",
        "Beginning of this day",
    ),
    (
        EnumKind::StandardBeginningDateVariant,
        EnumValue::StandardBeginningDateThisWeek,
        "НачалоЭтойНедели",
        "BeginningOfThisWeek",
        "Beginning of this week",
    ),
    (
        EnumKind::StandardBeginningDateVariant,
        EnumValue::StandardBeginningDateThisTenDays,
        "НачалоЭтойДекады",
        "BeginningOfThisTenDays",
        "Beginning of this ten-day period",
    ),
    (
        EnumKind::StandardBeginningDateVariant,
        EnumValue::StandardBeginningDateThisMonth,
        "НачалоЭтогоМесяца",
        "BeginningOfThisMonth",
        "Beginning of this month",
    ),
    (
        EnumKind::StandardBeginningDateVariant,
        EnumValue::StandardBeginningDateThisQuarter,
        "НачалоЭтогоКвартала",
        "BeginningOfThisQuarter",
        "Beginning of this quarter",
    ),
    (
        EnumKind::StandardBeginningDateVariant,
        EnumValue::StandardBeginningDateThisHalfYear,
        "НачалоЭтогоПолугодия",
        "BeginningOfThisHalfYear",
        "Beginning of this half year",
    ),
    (
        EnumKind::StandardBeginningDateVariant,
        EnumValue::StandardBeginningDateThisYear,
        "НачалоЭтогоГода",
        "BeginningOfThisYear",
        "Beginning of this year",
    ),
    (
        EnumKind::StandardBeginningDateVariant,
        EnumValue::StandardBeginningDateLastDay,
        "НачалоПрошлогоДня",
        "BeginningOfLastDay",
        "Beginning of yesterday",
    ),
    (
        EnumKind::StandardBeginningDateVariant,
        EnumValue::StandardBeginningDateLastWeek,
        "НачалоПрошлойНедели",
        "BeginningOfLastWeek",
        "Beginning of last week",
    ),
    (
        EnumKind::StandardBeginningDateVariant,
        EnumValue::StandardBeginningDateLastTenDays,
        "НачалоПрошлойДекады",
        "BeginningOfLastTenDays",
        "Beginning of last ten-day period",
    ),
    (
        EnumKind::StandardBeginningDateVariant,
        EnumValue::StandardBeginningDateLastMonth,
        "НачалоПрошлогоМесяца",
        "BeginningOfLastMonth",
        "Beginning of last month",
    ),
    (
        EnumKind::StandardBeginningDateVariant,
        EnumValue::StandardBeginningDateLastQuarter,
        "НачалоПрошлогоКвартала",
        "BeginningOfLastQuarter",
        "Beginning of last quarter",
    ),
    (
        EnumKind::StandardBeginningDateVariant,
        EnumValue::StandardBeginningDateLastHalfYear,
        "НачалоПрошлогоПолугодия",
        "BeginningOfLastHalfYear",
        "Beginning of last half year",
    ),
    (
        EnumKind::StandardBeginningDateVariant,
        EnumValue::StandardBeginningDateLastYear,
        "НачалоПрошлогоГода",
        "BeginningOfLastYear",
        "Beginning of last year",
    ),
    (
        EnumKind::StandardBeginningDateVariant,
        EnumValue::StandardBeginningDateNextDay,
        "НачалоСледующегоДня",
        "BeginningOfNextDay",
        "Beginning of tomorrow",
    ),
    (
        EnumKind::StandardBeginningDateVariant,
        EnumValue::StandardBeginningDateNextWeek,
        "НачалоСледующейНедели",
        "BeginningOfNextWeek",
        "Beginning of next week",
    ),
    (
        EnumKind::StandardBeginningDateVariant,
        EnumValue::StandardBeginningDateNextTenDays,
        "НачалоСледующейДекады",
        "BeginningOfNextTenDays",
        "Beginning of next ten-day period",
    ),
    (
        EnumKind::StandardBeginningDateVariant,
        EnumValue::StandardBeginningDateNextMonth,
        "НачалоСледующегоМесяца",
        "BeginningOfNextMonth",
        "Beginning of next month",
    ),
    (
        EnumKind::StandardBeginningDateVariant,
        EnumValue::StandardBeginningDateNextQuarter,
        "НачалоСледующегоКвартала",
        "BeginningOfNextQuarter",
        "Beginning of next quarter",
    ),
    (
        EnumKind::StandardBeginningDateVariant,
        EnumValue::StandardBeginningDateNextHalfYear,
        "НачалоСледующегоПолугодия",
        "BeginningOfNextHalfYear",
        "Beginning of next half year",
    ),
    (
        EnumKind::StandardBeginningDateVariant,
        EnumValue::StandardBeginningDateNextYear,
        "НачалоСледующегоГода",
        "BeginningOfNextYear",
        "Beginning of next year",
    ),
    (
        EnumKind::StandardBeginningDateVariant,
        EnumValue::StandardBeginningDateCustom,
        "ПроизвольнаяДата",
        "Custom",
        "Custom date",
    ),
    (
        EnumKind::ClientRunMode,
        EnumValue::ClientRunModeAuto,
        "Авто",
        "Auto",
        "Авто",
    ),
    (
        EnumKind::ClientRunMode,
        EnumValue::ClientRunModeOrdinaryApplication,
        "ОбычноеПриложение",
        "OrdinaryApplication",
        "Обычное приложение",
    ),
    (
        EnumKind::ClientRunMode,
        EnumValue::ClientRunModeManagedApplication,
        "УправляемоеПриложение",
        "ManagedApplication",
        "Управляемое приложение",
    ),
    // Строковые представления и метатипы трёх перечислений расширений
    // измерены на платформе 8.3.27.2342 отдельной воспроизводимой пробой.
    (
        EnumKind::ConfigurationExtensionsSource,
        EnumValue::ConfigurationExtensionsSourceDatabase,
        "БазаДанных",
        "Database",
        "Database",
    ),
    (
        EnumKind::ConfigurationExtensionsSource,
        EnumValue::ConfigurationExtensionsSourceSessionApplied,
        "СеансАктивные",
        "SessionApplied",
        "SessionApplied",
    ),
    (
        EnumKind::ConfigurationExtensionsSource,
        EnumValue::ConfigurationExtensionsSourceSessionDisabled,
        "СеансОтключенные",
        "SessionDisabled",
        "SessionDisabled",
    ),
    (
        EnumKind::ConfigurationExtensionScope,
        EnumValue::ConfigurationExtensionScopeInfoBase,
        "ИнформационнаяБаза",
        "InfoBase",
        "Infobase",
    ),
    (
        EnumKind::ConfigurationExtensionScope,
        EnumValue::ConfigurationExtensionScopeDataSeparation,
        "РазделениеДанных",
        "DataSeparation",
        "Data separation",
    ),
    (
        EnumKind::ConfigurationExtensionApplicationIssueSeverity,
        EnumValue::ConfigurationExtensionApplicationIssueSeverityCritical,
        "Критичная",
        "Critical",
        "Critical",
    ),
    (
        EnumKind::ConfigurationExtensionApplicationIssueSeverity,
        EnumValue::ConfigurationExtensionApplicationIssueSeverityLow,
        "Низкая",
        "Low",
        "Low",
    ),
    (
        EnumKind::ConfigurationExtensionApplicationIssueSeverity,
        EnumValue::ConfigurationExtensionApplicationIssueSeverityModerate,
        "Обычная",
        "Moderate",
        "Moderate",
    ),
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
        EnumKind::JsonEscapeCharacters,
        EnumValue::JsonEscapeNone,
        "Нет",
        "None",
        "Нет",
    ),
    (
        EnumKind::JsonEscapeCharacters,
        EnumValue::JsonEscapeNonAscii,
        "СимволыВнеASCII",
        "CharactersOutsideASCII",
        "Символы вне ASCII",
    ),
    (
        EnumKind::JsonEscapeCharacters,
        EnumValue::JsonEscapeNonBmp,
        "СимволыВнеBMP",
        "CharactersOutsideBMP",
        "Символы вне BMP",
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
    (
        EnumKind::SpreadFileType,
        EnumValue::SpreadFileMxl,
        "MXL",
        "MXL",
        "MXL",
    ),
    (
        EnumKind::SpreadFileType,
        EnumValue::SpreadFileXls,
        "XLS",
        "XLS",
        "XLS",
    ),
    (
        EnumKind::SpreadFileType,
        EnumValue::SpreadFileOds,
        "ODS",
        "ODS",
        "ODS",
    ),
    (
        EnumKind::SpreadFileType,
        EnumValue::SpreadFileAnsiTxt,
        "ANSITXT",
        "ANSITXT",
        "ANSITXT",
    ),
    (
        EnumKind::SpreadFileType,
        EnumValue::SpreadFileHtml5,
        "HTML5",
        "HTML5",
        "HTML5",
    ),
    (
        EnumKind::SpreadFileType,
        EnumValue::SpreadFileDocx,
        "DOCX",
        "DOCX",
        "DOCX",
    ),
    (
        EnumKind::SpreadFileType,
        EnumValue::SpreadFileTxt,
        "TXT",
        "TXT",
        "TXT",
    ),
    (
        EnumKind::SpreadFileType,
        EnumValue::SpreadFileXlsx,
        "XLSX",
        "XLSX",
        "XLSX",
    ),
    (
        EnumKind::SpreadFileType,
        EnumValue::SpreadFilePdf,
        "PDF",
        "PDF",
        "PDF",
    ),
    (
        EnumKind::DrawingKind,
        EnumValue::DrawingRectangle,
        "Прямоугольник",
        "Rectangle",
        "Прямоугольник",
    ),
    (
        EnumKind::PageOrientation,
        EnumValue::PageOrientationPortrait,
        "Портрет",
        "Portrait",
        "Портрет",
    ),
    (
        EnumKind::PageOrientation,
        EnumValue::PageOrientationLandscape,
        "Ландшафт",
        "Landscape",
        "Ландшафт",
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
    // ТипУзлаDOM. Пятая колонка снова измерена целиком: «Секция CDATA»,
    // «Инструкция обработки», «Определение типа документа», «Фрагмент
    // документа» и «Ссылка на сущность» печатаются человеческим текстом с
    // пробелами. Английские написания членов ИЗМЕРЕНЫ перебором
    // (`ТипУзлаDOM.Element` и остальные одиннадцать платформа принимает).
    (
        EnumKind::DomNodeType,
        EnumValue::DomElement,
        "Элемент",
        "Element",
        "Элемент",
    ),
    (
        EnumKind::DomNodeType,
        EnumValue::DomAttribute,
        "Атрибут",
        "Attribute",
        "Атрибут",
    ),
    (
        EnumKind::DomNodeType,
        EnumValue::DomText,
        "Текст",
        "Text",
        "Текст",
    ),
    (
        EnumKind::DomNodeType,
        EnumValue::DomCdataSection,
        "СекцияCDATA",
        "CDATASection",
        "Секция CDATA",
    ),
    (
        EnumKind::DomNodeType,
        EnumValue::DomComment,
        "Комментарий",
        "Comment",
        "Комментарий",
    ),
    (
        EnumKind::DomNodeType,
        EnumValue::DomProcessingInstruction,
        "ИнструкцияОбработки",
        "ProcessingInstruction",
        "Инструкция обработки",
    ),
    (
        EnumKind::DomNodeType,
        EnumValue::DomDocument,
        "Документ",
        "Document",
        "Документ",
    ),
    (
        EnumKind::DomNodeType,
        EnumValue::DomDocumentType,
        "ОпределениеТипаДокумента",
        // Не `DocumentTypeDefinition`: такого члена платформа не знает —
        // измерено обоими написаниями.
        "DocumentType",
        "Определение типа документа",
    ),
    (
        EnumKind::DomNodeType,
        EnumValue::DomDocumentFragment,
        "ФрагментДокумента",
        "DocumentFragment",
        "Фрагмент документа",
    ),
    (
        EnumKind::DomNodeType,
        EnumValue::DomNotation,
        "Нотация",
        "Notation",
        "Нотация",
    ),
    (
        EnumKind::DomNodeType,
        EnumValue::DomEntity,
        "Сущность",
        "Entity",
        "Сущность",
    ),
    (
        EnumKind::DomNodeType,
        EnumValue::DomEntityReference,
        "СсылкаНаСущность",
        "EntityReference",
        "Ссылка на сущность",
    ),
    // Печатаются идентификатором, без «человеческого» варианта — измерено.
    (
        EnumKind::TextEncoding,
        EnumValue::TextEncodingAnsi,
        "ANSI",
        "ANSI",
        "ANSI",
    ),
    (
        EnumKind::TextEncoding,
        EnumValue::TextEncodingOem,
        "OEM",
        "OEM",
        "OEM",
    ),
    (
        EnumKind::TextEncoding,
        EnumValue::TextEncodingUtf16,
        "UTF16",
        "UTF16",
        "UTF16",
    ),
    (
        EnumKind::TextEncoding,
        EnumValue::TextEncodingUtf8,
        "UTF8",
        "UTF8",
        "UTF8",
    ),
    (
        EnumKind::TextEncoding,
        EnumValue::TextEncodingSystem,
        "Системная",
        "System",
        "Системная",
    ),
    (
        EnumKind::StringEncodingMethod,
        EnumValue::StringEncodingUrl,
        "КодировкаURL",
        "КодировкаURL",
        "String URL-encoding",
    ),
    (
        EnumKind::StringEncodingMethod,
        EnumValue::StringEncodingUrlInUrl,
        "URLВКодировкеURL",
        "URLВКодировкеURL",
        "URL URL-encoding",
    ),
    (
        EnumKind::BackgroundJobState,
        EnumValue::BackgroundJobActive,
        "Активно",
        "Active",
        "Активно",
    ),
    (
        EnumKind::BackgroundJobState,
        EnumValue::BackgroundJobCompleted,
        "Завершено",
        "Completed",
        "Завершено",
    ),
    (
        EnumKind::BackgroundJobState,
        EnumValue::BackgroundJobFailed,
        "ЗавершеноАварийно",
        "Failed",
        "ЗавершеноАварийно",
    ),
    (
        EnumKind::BackgroundJobState,
        EnumValue::BackgroundJobCanceled,
        "Отменено",
        "Canceled",
        "Отменено",
    ),
    (
        EnumKind::MessageStatus,
        EnumValue::MessageStatusWithoutStatus,
        "БезСтатуса",
        "WithoutStatus",
        "БезСтатуса",
    ),
    (
        EnumKind::MessageStatus,
        EnumValue::MessageStatusImportant,
        "Важное",
        "Important",
        "Важное",
    ),
    (
        EnumKind::MessageStatus,
        EnumValue::MessageStatusAttention,
        "Внимание",
        "Attention",
        "Внимание",
    ),
    (
        EnumKind::MessageStatus,
        EnumValue::MessageStatusInformation,
        "Информация",
        "Information",
        "Информация",
    ),
    (
        EnumKind::MessageStatus,
        EnumValue::MessageStatusOrdinary,
        "Обычное",
        "Ordinary",
        "Обычное",
    ),
    (
        EnumKind::MessageStatus,
        EnumValue::MessageStatusVeryImportant,
        "ОченьВажное",
        "VeryImportant",
        "ОченьВажное",
    ),
    (
        EnumKind::ErrorCategory,
        EnumValue::ErrorCategoryAllErrors,
        "ВсеОшибки",
        "AllErrors",
        "ВсеОшибки",
    ),
    (
        EnumKind::ErrorCategory,
        EnumValue::ErrorCategoryExceptionRaisedFromScript,
        "ИсключениеВызванноеИзВстроенногоЯзыка",
        "ExceptionRaisedFromScript",
        "ИсключениеВызванноеИзВстроенногоЯзыка",
    ),
    (
        EnumKind::ErrorCategory,
        EnumValue::ErrorCategoryAccessViolation,
        "НарушениеПравДоступа",
        "AccessViolation",
        "НарушениеПравДоступа",
    ),
    (
        EnumKind::ErrorCategory,
        EnumValue::ErrorCategoryUnsupportedFormat,
        "НеподдерживаемыйФормат",
        "UnsupportedFormat",
        "НеподдерживаемыйФормат",
    ),
    (
        EnumKind::ErrorCategory,
        EnumValue::ErrorCategoryInvalidPassword,
        "НеправильныйПароль",
        "InvalidPassword",
        "НеправильныйПароль",
    ),
    (
        EnumKind::ErrorCategory,
        EnumValue::ErrorCategoryNoPermissionToUseFunctionality,
        "ОтсутствиеРазрешенияДляИспользованияФункциональности",
        "NoPermissionToUseFunctionality",
        "ОтсутствиеРазрешенияДляИспользованияФункциональности",
    ),
    (
        EnumKind::ErrorCategory,
        EnumValue::ErrorCategoryExternalDataSourceError,
        "ОшибкаВнешнегоИсточникаДанных",
        "ExternalDataSourceError",
        "ОшибкаВнешнегоИсточникаДанных",
    ),
    (
        EnumKind::ErrorCategory,
        EnumValue::ErrorCategoryScriptRuntimeError,
        "ОшибкаВоВремяВыполненияВстроенногоЯзыка",
        "ScriptRuntimeError",
        "ОшибкаВоВремяВыполненияВстроенногоЯзыка",
    ),
    (
        EnumKind::ErrorCategory,
        EnumValue::ErrorCategoryLocalFileAccessError,
        "ОшибкаДоступаКЛокальномуФайлу",
        "LocalFileAccessError",
        "ОшибкаДоступаКЛокальномуФайлу",
    ),
    (
        EnumKind::ErrorCategory,
        EnumValue::ErrorCategoryScriptUseError,
        "ОшибкаИспользованияВстроенногоЯзыка",
        "ScriptUseError",
        "ОшибкаИспользованияВстроенногоЯзыка",
    ),
    (
        EnumKind::ErrorCategory,
        EnumValue::ErrorCategoryScriptCompileError,
        "ОшибкаКомпиляцииВстроенногоЯзыка",
        "ScriptCompileError",
        "ОшибкаКомпиляцииВстроенногоЯзыка",
    ),
    (
        EnumKind::ErrorCategory,
        EnumValue::ErrorCategoryConfigurationError,
        "ОшибкаКонфигурации",
        "ConfigurationError",
        "ОшибкаКонфигурации",
    ),
    (
        EnumKind::ErrorCategory,
        EnumValue::ErrorCategoryDatabaseCopyError,
        "ОшибкаКопииБазыДанных",
        "DatabaseCopyError",
        "ОшибкаКопииБазыДанных",
    ),
    (
        EnumKind::ErrorCategory,
        EnumValue::ErrorCategoryDataCompositionSettingsError,
        "ОшибкаНастроекКомпоновкиДанных",
        "DataCompositionSettingsError",
        "ОшибкаНастроекКомпоновкиДанных",
    ),
    (
        EnumKind::ErrorCategory,
        EnumValue::ErrorCategoryGotoURLError,
        "ОшибкаПереходаПоНавигационнойСсылке",
        "GotoURLError",
        "ОшибкаПереходаПоНавигационнойСсылке",
    ),
    (
        EnumKind::ErrorCategory,
        EnumValue::ErrorCategoryFullTextSearchError,
        "ОшибкаПолнотекстовогоПоиска",
        "FullTextSearchError",
        "ОшибкаПолнотекстовогоПоиска",
    ),
    (
        EnumKind::ErrorCategory,
        EnumValue::ErrorCategoryDocumentConversionError,
        "ОшибкаПреобразованияДокумента",
        "DocumentConversionError",
        "ОшибкаПреобразованияДокумента",
    ),
    (
        EnumKind::ErrorCategory,
        EnumValue::ErrorCategorySignatureVerificationError,
        "ОшибкаПроверкиПодписи",
        "SignatureVerificationError",
        "ОшибкаПроверкиПодписи",
    ),
    (
        EnumKind::ErrorCategory,
        EnumValue::ErrorCategoryPrinterError,
        "ОшибкаРаботыСПринтером",
        "PrinterError",
        "ОшибкаРаботыСПринтером",
    ),
    (
        EnumKind::ErrorCategory,
        EnumValue::ErrorCategorySpeechProcessingError,
        "ОшибкаРаботыСРечью",
        "SpeechProcessingError",
        "ОшибкаРаботыСРечью",
    ),
    (
        EnumKind::ErrorCategory,
        EnumValue::ErrorCategorySessionError,
        "ОшибкаСеанса",
        "SessionError",
        "ОшибкаСеанса",
    ),
    (
        EnumKind::ErrorCategory,
        EnumValue::ErrorCategoryNetworkError,
        "ОшибкаСети",
        "NetworkError",
        "ОшибкаСети",
    ),
    (
        EnumKind::ErrorCategory,
        EnumValue::ErrorCategoryCollaborationSystemError,
        "ОшибкаСистемыВзаимодействия",
        "CollaborationSystemError",
        "ОшибкаСистемыВзаимодействия",
    ),
    (
        EnumKind::ErrorCategory,
        EnumValue::ErrorCategoryMultimediaToolsError,
        "ОшибкаСредствМультимедиа",
        "MultimediaToolsError",
        "ОшибкаСредствМультимедиа",
    ),
    (
        EnumKind::ErrorCategory,
        EnumValue::ErrorCategoryDatabaseTablespaceError,
        "ОшибкаТабличногоПространстваБазыДанных",
        "DatabaseTablespaceError",
        "ОшибкаТабличногоПространстваБазыДанных",
    ),
    (
        EnumKind::ErrorCategory,
        EnumValue::ErrorCategoryStoredDataError,
        "ОшибкаХранимыхДанных",
        "StoredDataError",
        "ОшибкаХранимыхДанных",
    ),
    (
        EnumKind::ErrorCategory,
        EnumValue::ErrorCategoryForcedShutdown,
        "ПринудительноеЗавершениеРаботы",
        "ForcedShutdown",
        "ПринудительноеЗавершениеРаботы",
    ),
    (
        EnumKind::ErrorCategory,
        EnumValue::ErrorCategoryOtherError,
        "ПрочаяОшибка",
        "OtherError",
        "ПрочаяОшибка",
    ),
    (
        EnumKind::SortDirection,
        EnumValue::SortDirectionAscending,
        "Возр",
        "Возр",
        "Ascending",
    ),
    (
        EnumKind::DateFractions,
        EnumValue::DateFractionsDateTime,
        "ДатаВремя",
        "DateTime",
        "Date and time",
    ),
    (
        EnumKind::AllowedLength,
        EnumValue::AllowedLengthVariable,
        "Переменная",
        "Variable",
        "Variable-length",
    ),
    (
        EnumKind::AllowedLength,
        EnumValue::AllowedLengthFixed,
        "Фиксированная",
        "Fixed",
        "Fixed-length",
    ),
    (
        EnumKind::HashFunction,
        EnumValue::HashMd5,
        "MD5",
        "MD5",
        "MD5 hash function",
    ),
    (
        EnumKind::HashFunction,
        EnumValue::HashSha1,
        "SHA1",
        "SHA1",
        "SHA-1 hash function",
    ),
    (
        EnumKind::HashFunction,
        EnumValue::HashSha256,
        "SHA256",
        "SHA256",
        "SHA-256 hash function",
    ),
    (
        EnumKind::JsonDateFormat,
        EnumValue::DateFormatMicrosoft,
        "Microsoft",
        "Microsoft",
        "Microsoft",
    ),
    // Английские написания членов — ИЗМЕРЕНО (`JSON.DATE_VARIANT_EN_NAMES`,
    // детализированная проба по кандидатам): `LocalDate`,
    // `LocalDateWithOffset`, `UniversalDate`. Первый прогон (равенство всех
    // трёх разом) дал общий `<ошибка>` из-за `LocalDateTimeOffset` —
    // предположения по аналогии, которое здесь ошибочно стояло вместо
    // измеренного `LocalDateWithOffset`.
    //
    // Пятая колонка (текст `Строка()`) — ИЗМЕРЕНО для всех трёх членов:
    // «Локальная дата», «Локальная дата со смещением», «Универсальная
    // дата» (раздельно, со строчными буквами — НЕ слитное написание
    // идентификатора, как ошибочно предполагалось до первого замера).
    (
        EnumKind::JsonDateWritingVariant,
        EnumValue::DateVariantLocal,
        "ЛокальнаяДата",
        "LocalDate",
        "Локальная дата",
    ),
    (
        EnumKind::JsonDateWritingVariant,
        EnumValue::DateVariantLocalOffset,
        "ЛокальнаяДатаСоСмещением",
        "LocalDateWithOffset",
        "Локальная дата со смещением",
    ),
    (
        EnumKind::JsonDateWritingVariant,
        EnumValue::DateVariantUniversal,
        "УниверсальнаяДата",
        "UniversalDate",
        "Универсальная дата",
    ),
    // Третья и четвёртая колонки СОВПАДАЮТ: у этого перечисления русских
    // написаний членов нет вовсе, а печатается член через дефис и со
    // строчной второй частью — «Little-endian», не «LittleEndian». Всё
    // измерено на 8.3.27.
    (
        EnumKind::ByteOrder,
        EnumValue::ByteOrderLittle,
        "LittleEndian",
        "LittleEndian",
        "Little-endian",
    ),
    (
        EnumKind::ByteOrder,
        EnumValue::ByteOrderBig,
        "BigEndian",
        "BigEndian",
        "Big-endian",
    ),
    // Пятая колонка у потоковых перечислений снова не производная от
    // третьей: составные члены платформа печатает раздельно и со строчной
    // второй частью — «Создать новый», «Открыть или создать», «Чтение и
    // запись». Всё измерено фикстурой `binary-streams`.
    (
        EnumKind::FileOpenMode,
        EnumValue::FileOpenModeOpen,
        "Открыть",
        "Open",
        "Открыть",
    ),
    (
        EnumKind::FileOpenMode,
        EnumValue::FileOpenModeOpenOrCreate,
        "ОткрытьИлиСоздать",
        "OpenOrCreate",
        "Открыть или создать",
    ),
    (
        EnumKind::FileOpenMode,
        EnumValue::FileOpenModeCreate,
        "Создать",
        "Create",
        "Создать",
    ),
    (
        EnumKind::FileOpenMode,
        EnumValue::FileOpenModeCreateNew,
        "СоздатьНовый",
        "CreateNew",
        "Создать новый",
    ),
    (
        EnumKind::FileOpenMode,
        EnumValue::FileOpenModeTruncate,
        "Обрезать",
        "Truncate",
        "Обрезать",
    ),
    (
        EnumKind::FileOpenMode,
        EnumValue::FileOpenModeAppend,
        "Дописать",
        "Append",
        "Дописать",
    ),
    (
        EnumKind::FileAccess,
        EnumValue::FileAccessRead,
        "Чтение",
        "Read",
        "Чтение",
    ),
    (
        EnumKind::FileAccess,
        EnumValue::FileAccessWrite,
        "Запись",
        "Write",
        "Запись",
    ),
    (
        EnumKind::FileAccess,
        EnumValue::FileAccessReadAndWrite,
        "ЧтениеИЗапись",
        "ReadAndWrite",
        "Чтение и запись",
    ),
    (
        EnumKind::StreamPosition,
        EnumValue::StreamPositionBegin,
        "Начало",
        "Begin",
        "Начало",
    ),
    (
        EnumKind::StreamPosition,
        EnumValue::StreamPositionCurrent,
        "Текущая",
        "Current",
        "Текущая",
    ),
    (
        EnumKind::StreamPosition,
        EnumValue::StreamPositionEnd,
        "Конец",
        "End",
        "Конец",
    ),
    // --- перечисления модели схемы -----------------------------------------
    // Английские написания ЧЛЕНОВ ИЗМЕРЕНЫ перебором всех сорока семи:
    // платформа принимает `ФормаПредставленияXS.Qualified`,
    // `ТипКомпонентыXS.MinInclusiveFacet` и так далее. Пятая колонка — то,
    // что печатает `Строка()`: у `ТипКомпонентыXS` это человеческий текст с
    // пробелами, у остальных совпадает с русским идентификатором, кроме
    // `ПоУмолчанию` -> «По умолчанию».
    (
        EnumKind::XsComponentType,
        EnumValue::XsCompSchema,
        "Схема",
        "Schema",
        "Схема",
    ),
    (
        EnumKind::XsComponentType,
        EnumValue::XsCompElementDeclaration,
        "ОбъявлениеЭлемента",
        "ElementDeclaration",
        "Объявление элемента",
    ),
    (
        EnumKind::XsComponentType,
        EnumValue::XsCompAttributeDeclaration,
        "ОбъявлениеАтрибута",
        "AttributeDeclaration",
        "Объявление атрибута",
    ),
    (
        EnumKind::XsComponentType,
        EnumValue::XsCompSimpleTypeDefinition,
        "ОпределениеПростогоТипа",
        "SimpleTypeDefinition",
        "Определение простого типа",
    ),
    (
        EnumKind::XsComponentType,
        EnumValue::XsCompComplexTypeDefinition,
        "ОпределениеСоставногоТипа",
        "ComplexTypeDefinition",
        "Определение составного типа",
    ),
    (
        EnumKind::XsComponentType,
        EnumValue::XsCompParticle,
        "Фрагмент",
        "Particle",
        "Фрагмент",
    ),
    (
        EnumKind::XsComponentType,
        EnumValue::XsCompModelGroup,
        "ГруппаМодели",
        "ModelGroup",
        "Группа модели",
    ),
    (
        EnumKind::XsComponentType,
        EnumValue::XsCompAttributeUse,
        "ИспользованиеАтрибута",
        "AttributeUse",
        "Использование атрибута",
    ),
    (
        EnumKind::XsComponentType,
        EnumValue::XsCompModelGroupDefinition,
        "ОпределениеГруппыМодели",
        "ModelGroupDefinition",
        "Определение группы модели",
    ),
    (
        EnumKind::XsComponentType,
        EnumValue::XsCompAttributeGroupDefinition,
        "ОпределениеГруппыАтрибутов",
        "AttributeGroupDefinition",
        "Определение группы атрибутов",
    ),
    (
        EnumKind::XsComponentType,
        EnumValue::XsCompNotationDeclaration,
        "ОбъявлениеНотации",
        "NotationDeclaration",
        "Объявление нотации",
    ),
    (
        EnumKind::XsComponentType,
        EnumValue::XsCompAnnotation,
        "Аннотация",
        "Annotation",
        "Аннотация",
    ),
    (
        EnumKind::XsComponentType,
        EnumValue::XsCompDocumentation,
        "Документация",
        "Documentation",
        "Документация",
    ),
    // Имя члена — `ИнформацияПриложения`, хотя тип называется
    // `ИнформацияДляПриложенияXS`: `ТипКомпонентыXS.ИнформацияДляПриложения`
    // платформа отвергает (измерено).
    (
        EnumKind::XsComponentType,
        EnumValue::XsCompAppInfo,
        "ИнформацияПриложения",
        "AppInfo",
        "Информация приложения",
    ),
    (
        EnumKind::XsComponentType,
        EnumValue::XsCompWildcard,
        "Маска",
        "Wildcard",
        "Маска",
    ),
    (
        EnumKind::XsComponentType,
        EnumValue::XsCompImport,
        "Импорт",
        "Import",
        "Импорт",
    ),
    (
        EnumKind::XsComponentType,
        EnumValue::XsCompInclude,
        "Включение",
        "Include",
        "Включение",
    ),
    (
        EnumKind::XsComponentType,
        EnumValue::XsCompRedefine,
        "Переопределение",
        "Redefine",
        "Переопределение",
    ),
    (
        EnumKind::XsComponentType,
        EnumValue::XsCompLengthFacet,
        "ФасетДлины",
        "LengthFacet",
        "Фасет длины",
    ),
    (
        EnumKind::XsComponentType,
        EnumValue::XsCompMinLengthFacet,
        "ФасетМинимальнойДлины",
        "MinLengthFacet",
        "Фасет минимальной длины",
    ),
    (
        EnumKind::XsComponentType,
        EnumValue::XsCompMaxLengthFacet,
        "ФасетМаксимальнойДлины",
        "MaxLengthFacet",
        "Фасет максимальной длины",
    ),
    (
        EnumKind::XsComponentType,
        EnumValue::XsCompPatternFacet,
        "ФасетОбразца",
        "PatternFacet",
        "Фасет образца",
    ),
    (
        EnumKind::XsComponentType,
        EnumValue::XsCompEnumerationFacet,
        "ФасетПеречисления",
        "EnumerationFacet",
        "Фасет перечисления",
    ),
    (
        EnumKind::XsComponentType,
        EnumValue::XsCompWhitespaceFacet,
        "ФасетПробельныхСимволов",
        "WhitespaceFacet",
        "Фасет пробельных символов",
    ),
    (
        EnumKind::XsComponentType,
        EnumValue::XsCompTotalDigitsFacet,
        "ФасетОбщегоКоличестваРазрядов",
        "TotalDigitsFacet",
        "Фасет общего количества разрядов",
    ),
    (
        EnumKind::XsComponentType,
        EnumValue::XsCompFractionDigitsFacet,
        "ФасетКоличестваРазрядовДробнойЧасти",
        "FractionDigitsFacet",
        "Фасет количества разрядов дробной части",
    ),
    // Четыре граничных фасета названы у платформы ИНАЧЕ, чем остальные:
    // прилагательное впереди («МинимальноВключающийФасет»), а не «Фасет…».
    (
        EnumKind::XsComponentType,
        EnumValue::XsCompMinInclusiveFacet,
        "МинимальноВключающийФасет",
        "MinInclusiveFacet",
        "Минимально включающий фасет",
    ),
    (
        EnumKind::XsComponentType,
        EnumValue::XsCompMaxInclusiveFacet,
        "МаксимальноВключающийФасет",
        "MaxInclusiveFacet",
        "Максимально включающий фасет",
    ),
    (
        EnumKind::XsComponentType,
        EnumValue::XsCompMinExclusiveFacet,
        "МинимальноИсключающийФасет",
        "MinExclusiveFacet",
        "Минимально исключающий фасет",
    ),
    (
        EnumKind::XsComponentType,
        EnumValue::XsCompMaxExclusiveFacet,
        "МаксимальноИсключающийФасет",
        "MaxExclusiveFacet",
        "Максимально исключающий фасет",
    ),
    (
        EnumKind::XsComponentType,
        EnumValue::XsCompIdentityConstraintDefinition,
        "ОпределениеОграниченияИдентичности",
        "IdentityConstraintDefinition",
        "Определение ограничения идентичности",
    ),
    (
        EnumKind::XsComponentType,
        EnumValue::XsCompXPathDefinition,
        "ОпределениеXPath",
        "XPathDefinition",
        "Определение XPath",
    ),
    (
        EnumKind::XsForm,
        EnumValue::XsFormQualified,
        "Квалифицированная",
        "Qualified",
        "Квалифицированная",
    ),
    (
        EnumKind::XsForm,
        EnumValue::XsFormUnqualified,
        "Неквалифицированная",
        "Unqualified",
        "Неквалифицированная",
    ),
    (
        EnumKind::XsSimpleTypeVariety,
        EnumValue::XsVarietyAtomic,
        "Атомарная",
        "Atomic",
        "Атомарная",
    ),
    (
        EnumKind::XsSimpleTypeVariety,
        EnumValue::XsVarietyList,
        "Список",
        "List",
        "Список",
    ),
    (
        EnumKind::XsSimpleTypeVariety,
        EnumValue::XsVarietyUnion,
        "Объединение",
        "Union",
        "Объединение",
    ),
    (
        EnumKind::XsModelGroupKind,
        EnumValue::XsGroupSequence,
        "Последовательность",
        "Sequence",
        "Последовательность",
    ),
    (
        EnumKind::XsModelGroupKind,
        EnumValue::XsGroupChoice,
        "Выбор",
        "Choice",
        "Выбор",
    ),
    (
        EnumKind::XsModelGroupKind,
        EnumValue::XsGroupAll,
        "Все",
        "All",
        "Все",
    ),
    (
        EnumKind::XsDerivationMethod,
        EnumValue::XsDerivationExtension,
        "Расширение",
        "Extension",
        "Расширение",
    ),
    (
        EnumKind::XsDerivationMethod,
        EnumValue::XsDerivationRestriction,
        "Ограничение",
        "Restriction",
        "Ограничение",
    ),
    // Печатается с ПРОБЕЛОМ — «По умолчанию», хотя пишется слитно.
    (
        EnumKind::XsValueConstraint,
        EnumValue::XsConstraintDefault,
        "ПоУмолчанию",
        "Default",
        "По умолчанию",
    ),
    (
        EnumKind::XsValueConstraint,
        EnumValue::XsConstraintFixed,
        "Фиксированное",
        "Fixed",
        "Фиксированное",
    ),
    (
        EnumKind::XsWhitespaceHandling,
        EnumValue::XsWhitespacePreserve,
        "Сохранять",
        "Preserve",
        "Сохранять",
    ),
    (
        EnumKind::XsWhitespaceHandling,
        EnumValue::XsWhitespaceReplace,
        "Заменять",
        "Replace",
        "Заменять",
    ),
    (
        EnumKind::XsWhitespaceHandling,
        EnumValue::XsWhitespaceCollapse,
        "Сворачивать",
        "Collapse",
        "Сворачивать",
    ),
    (
        EnumKind::XmlForm,
        EnumValue::XmlFormElement,
        "Элемент",
        "Element",
        "Элемент",
    ),
    (
        EnumKind::XmlForm,
        EnumValue::XmlFormAttribute,
        "Атрибут",
        "Attribute",
        "Атрибут",
    ),
    (
        EnumKind::XmlForm,
        EnumValue::XmlFormText,
        "Текст",
        "Text",
        "Текст",
    ),
    // `ВидФасетаXDTO`: русские написания членов НЕ выводятся из
    // представлений — они короче и местами неожиданны (`МинДлина` при
    // представлении «Минимальная длина», `РазрядовВсего` при «Количество
    // разрядов»). Каждое проверено отдельной пробой, как и то, что
    // «длинные» написания (`МинимальнаяДлина`, `ОбщееКоличествоРазрядов`,
    // `МинимальноеВключающееЗначение`) платформа отвергает.
    (
        EnumKind::XdtoFacetKind,
        EnumValue::XdtoFacetLength,
        "Длина",
        "Length",
        "Длина",
    ),
    (
        EnumKind::XdtoFacetKind,
        EnumValue::XdtoFacetMinLength,
        "МинДлина",
        "MinLength",
        "Минимальная длина",
    ),
    (
        EnumKind::XdtoFacetKind,
        EnumValue::XdtoFacetMaxLength,
        "МаксДлина",
        "MaxLength",
        "Максимальная длина",
    ),
    (
        EnumKind::XdtoFacetKind,
        EnumValue::XdtoFacetPattern,
        "Образец",
        "Pattern",
        "Образец",
    ),
    (
        EnumKind::XdtoFacetKind,
        EnumValue::XdtoFacetEnumeration,
        "Перечисление",
        "Enumeration",
        "Перечисление",
    ),
    (
        EnumKind::XdtoFacetKind,
        EnumValue::XdtoFacetWhiteSpace,
        "ПробельныеСимволы",
        "WhiteSpace",
        "Пробельные символы",
    ),
    (
        EnumKind::XdtoFacetKind,
        EnumValue::XdtoFacetTotalDigits,
        "РазрядовВсего",
        "TotalDigits",
        "Количество разрядов",
    ),
    (
        EnumKind::XdtoFacetKind,
        EnumValue::XdtoFacetFractionDigits,
        "РазрядовДробнойЧасти",
        "FractionDigits",
        "Количество разрядов дробной части",
    ),
    (
        EnumKind::XdtoFacetKind,
        EnumValue::XdtoFacetMinInclusive,
        "МинВключающее",
        "MinInclusive",
        "Минимальное включающее значение",
    ),
    (
        EnumKind::XdtoFacetKind,
        EnumValue::XdtoFacetMaxInclusive,
        "МаксВключающее",
        "MaxInclusive",
        "Максимальное включающее значение",
    ),
    (
        EnumKind::XdtoFacetKind,
        EnumValue::XdtoFacetMinExclusive,
        "МинИсключающее",
        "MinExclusive",
        "Минимальное исключающее значение",
    ),
    (
        EnumKind::XdtoFacetKind,
        EnumValue::XdtoFacetMaxExclusive,
        "МаксИсключающее",
        "MaxExclusive",
        "Максимальное исключающее значение",
    ),
    // ТипРезультатаDOMXPath. Пятая колонка измерена целиком: шесть узловых
    // членов печатаются человеческим текстом («Неупорядоченный итератор
    // узлов»), а четыре простых — как пишутся. Английские написания
    // измерены перебором: `ТипРезультатаDOMXPath.Any` и остальные девять
    // платформа принимает и считает равными русским.
    (
        EnumKind::DomXPathResultType,
        EnumValue::XPathAny,
        "Любой",
        "Any",
        "Любой",
    ),
    (
        EnumKind::DomXPathResultType,
        EnumValue::XPathNumber,
        "Число",
        "Number",
        "Число",
    ),
    (
        EnumKind::DomXPathResultType,
        EnumValue::XPathString,
        "Строка",
        "String",
        "Строка",
    ),
    (
        EnumKind::DomXPathResultType,
        EnumValue::XPathBoolean,
        "Булево",
        "Boolean",
        "Булево",
    ),
    (
        EnumKind::DomXPathResultType,
        EnumValue::XPathUnorderedNodeIterator,
        "НеупорядоченныйИтераторУзлов",
        "UnorderedNodeIterator",
        "Неупорядоченный итератор узлов",
    ),
    (
        EnumKind::DomXPathResultType,
        EnumValue::XPathOrderedNodeIterator,
        "УпорядоченныйИтераторУзлов",
        "OrderedNodeIterator",
        "Упорядоченный итератор узлов",
    ),
    (
        EnumKind::DomXPathResultType,
        EnumValue::XPathUnorderedNodeSnapshot,
        "НеупорядоченныйСнимокУзлов",
        "UnorderedNodeSnapshot",
        "Неупорядоченный снимок узлов",
    ),
    (
        EnumKind::DomXPathResultType,
        EnumValue::XPathOrderedNodeSnapshot,
        "УпорядоченныйСнимокУзлов",
        "OrderedNodeSnapshot",
        "Упорядоченный снимок узлов",
    ),
    (
        EnumKind::DomXPathResultType,
        EnumValue::XPathAnyUnorderedNode,
        "ЛюбойНеупорядоченныйУзел",
        "AnyUnorderedNode",
        "Любой неупорядоченный узел",
    ),
    (
        EnumKind::DomXPathResultType,
        EnumValue::XPathFirstOrderedNode,
        "ПервыйУпорядоченныйУзел",
        "FirstOrderedNode",
        "Первый упорядоченный узел",
    ),
    // НаправлениеПоиска. Пятая колонка измерена: имя пишется слитно
    // (`СНачала`), а печатается с пробелом («С начала»).
    (
        EnumKind::SearchDirection,
        EnumValue::SearchFromBegin,
        "СНачала",
        "FromBegin",
        "С начала",
    ),
    (
        EnumKind::SearchDirection,
        EnumValue::SearchFromEnd,
        "СКонца",
        "FromEnd",
        "С конца",
    ),
    // Представления обоих режимов ИЗМЕРЕНЫ и, как обычно, не выводятся из
    // имён: у `НеВосстанавливать` печатается «Не восстанавливать пути».
    (
        EnumKind::ZipRestorePathsMode,
        EnumValue::RestorePaths,
        "Восстанавливать",
        "Restore",
        "Восстанавливать пути",
    ),
    (
        EnumKind::ZipRestorePathsMode,
        EnumValue::DontRestorePaths,
        "НеВосстанавливать",
        "DontRestore",
        "Не восстанавливать пути",
    ),
    // Члены `ТипСвязиВложенияPDF`. Английские написания есть у всех пяти,
    // и представление снова измерено: у `НеУстановлено` печатается «Не
    // указано» — ни «НеУстановлено», ни «Unspecified».
    (
        EnumKind::PdfAttachmentRelation,
        EnumValue::PdfRelationSource,
        "Источник",
        "Source",
        "Источник",
    ),
    (
        EnumKind::PdfAttachmentRelation,
        EnumValue::PdfRelationData,
        "Данные",
        "Data",
        "Данные",
    ),
    (
        EnumKind::PdfAttachmentRelation,
        EnumValue::PdfRelationAlternative,
        "Альтернатива",
        "Alternative",
        "Альтернатива",
    ),
    (
        EnumKind::PdfAttachmentRelation,
        EnumValue::PdfRelationSupplement,
        "Дополнение",
        "Supplement",
        "Дополнение",
    ),
    (
        EnumKind::PdfAttachmentRelation,
        EnumValue::PdfRelationUnspecified,
        "НеУстановлено",
        "Unspecified",
        "Не указано",
    ),
    // У членов `ТипФайлаАрхива` написание латинское и на обоих языках
    // одинаковое; представление снова измерено — у `SevenZip` печатается
    // «Тип архива 7Z», а не «7Zip». `Deflate64` и `SevenZ` платформа не
    // знает — проверено перебором.
    (
        EnumKind::ArchiveFileType,
        EnumValue::ArchiveTypeZip,
        "Zip",
        "Zip",
        "Тип архива ZIP",
    ),
    (
        EnumKind::ArchiveFileType,
        EnumValue::ArchiveTypeBzip2,
        "BZIP2",
        "BZIP2",
        "Тип архива BZIP2",
    ),
    (
        EnumKind::ArchiveFileType,
        EnumValue::ArchiveTypeGzip,
        "GZIP",
        "GZIP",
        "Тип архива GZIP",
    ),
    (
        EnumKind::ArchiveFileType,
        EnumValue::ArchiveTypeRar,
        "RAR",
        "RAR",
        "Тип архива RAR",
    ),
    (
        EnumKind::ArchiveFileType,
        EnumValue::ArchiveTypeSevenZip,
        "SevenZip",
        "SevenZip",
        "Тип архива 7Z",
    ),
    (
        EnumKind::ArchiveFileType,
        EnumValue::ArchiveTypeTar,
        "TAR",
        "TAR",
        "Тип архива TAR",
    ),
    (
        EnumKind::ArchiveFileType,
        EnumValue::ArchiveTypeXz,
        "XZ",
        "XZ",
        "Тип архива XZ",
    ),
    // Перечисления записи. Пятая колонка везде измерена и нигде не
    // выводится из имени: у `Сжатие` печатается «Метод сжатия сжатием», у
    // `BZIP2` — «Метод сжатия BZIP» (без двойки), у `ОбрабатыватьРекурсивно`
    // — просто «Обрабатывать».
    (
        EnumKind::ZipCompressionMethod,
        EnumValue::ZipMethodDeflate,
        "Сжатие",
        "Deflate",
        "Метод сжатия сжатием",
    ),
    (
        EnumKind::ZipCompressionMethod,
        EnumValue::ZipMethodCopy,
        "Копирование",
        "Copy",
        "Метод сжатия копированием",
    ),
    (
        EnumKind::ZipCompressionMethod,
        EnumValue::ZipMethodBzip2,
        "BZIP2",
        "BZip2",
        "Метод сжатия BZIP",
    ),
    (
        EnumKind::ZipCompressionLevel,
        EnumValue::ZipLevelMinimal,
        "Минимальный",
        "Minimum",
        "Минимальный уровень сжатия",
    ),
    (
        EnumKind::ZipCompressionLevel,
        EnumValue::ZipLevelOptimal,
        "Оптимальный",
        "Optimal",
        "Оптимальный уровень сжатия",
    ),
    (
        EnumKind::ZipCompressionLevel,
        EnumValue::ZipLevelMaximal,
        "Максимальный",
        "Maximum",
        "Максимальный уровень сжатия",
    ),
    (
        EnumKind::ZipStorePathMode,
        EnumValue::ZipStoreRelativePath,
        "СохранятьОтносительныеПути",
        "StoreRelativePath",
        "Сохранять относительные пути",
    ),
    (
        EnumKind::ZipStorePathMode,
        EnumValue::ZipStoreFullPath,
        "СохранятьПолныеПути",
        "StoreFullPath",
        "Сохранять полные пути",
    ),
    (
        EnumKind::ZipStorePathMode,
        EnumValue::ZipDontStorePath,
        "НеСохранятьПути",
        "DontStorePath",
        "Не сохранять пути",
    ),
    (
        EnumKind::ZipSubDirProcessingMode,
        EnumValue::ZipDontProcessSubdirs,
        "НеОбрабатывать",
        "DontProcess",
        "Не обрабатывать",
    ),
    (
        EnumKind::ZipSubDirProcessingMode,
        EnumValue::ZipProcessSubdirsRecursively,
        "ОбрабатыватьРекурсивно",
        "ProcessRecursively",
        "Обрабатывать",
    ),
    (
        EnumKind::ZipEncryptionMethod,
        EnumValue::ZipEncryptionAes128,
        "AES128",
        "AES128",
        "Шифрование методом AES 128 бит",
    ),
    (
        EnumKind::ZipEncryptionMethod,
        EnumValue::ZipEncryptionAes192,
        "AES192",
        "AES192",
        "Шифрование методом AES 192 бит",
    ),
    (
        EnumKind::ZipEncryptionMethod,
        EnumValue::ZipEncryptionAes256,
        "AES256",
        "AES256",
        "Шифрование методом AES 256 бит",
    ),
    (
        EnumKind::ZipEncryptionMethod,
        EnumValue::ZipEncryptionZip20,
        "Zip20",
        "Zip20",
        "Шифрование методом ZIP 2.0",
    ),
    (
        EnumKind::ZipFileNamesEncoding,
        EnumValue::ZipNamesAuto,
        "Авто",
        "Auto",
        "Авто",
    ),
    (
        EnumKind::ZipFileNamesEncoding,
        EnumValue::ZipNamesUtf8,
        "UTF8",
        "Utf8",
        "UTF8",
    ),
    (
        EnumKind::ByteOrderMarkUse,
        EnumValue::ByteOrderMarkUse,
        "Использовать",
        "Use",
        "Использовать",
    ),
    (
        EnumKind::ByteOrderMarkUse,
        EnumValue::ByteOrderMarkDoNotUse,
        "НеИспользовать",
        "DoNotUse",
        "Не использовать",
    ),
];

/// Имена всех перечислений — для автодополнения REPL и для резолвера,
/// который по левой части точки решает, перечисление это или переменная.
pub const ENUM_NAMES: &[(&str, EnumKind)] = &[
    ("ВариантВстроенногоЯзыка", EnumKind::ScriptVariant),
    ("ScriptVariant", EnumKind::ScriptVariant),
    ("РежимЗапускаКлиентскогоПриложения", EnumKind::ClientRunMode),
    ("ClientRunMode", EnumKind::ClientRunMode),
    (
        "ИсточникРасширенийКонфигурации",
        EnumKind::ConfigurationExtensionsSource,
    ),
    (
        "ConfigurationExtensionsSource",
        EnumKind::ConfigurationExtensionsSource,
    ),
    (
        "ОбластьДействияРасширенияКонфигурации",
        EnumKind::ConfigurationExtensionScope,
    ),
    (
        "ConfigurationExtensionScope",
        EnumKind::ConfigurationExtensionScope,
    ),
    (
        "ВажностьПроблемыПримененияРасширенияКонфигурации",
        EnumKind::ConfigurationExtensionApplicationIssueSeverity,
    ),
    (
        "ConfigurationExtensionApplicationIssueSeverity",
        EnumKind::ConfigurationExtensionApplicationIssueSeverity,
    ),
    ("УровеньЖурналаРегистрации", EnumKind::EventLogLevel),
    ("EventLogLevel", EnumKind::EventLogLevel),
    (
        "РежимТранзакцииЗаписиЖурналаРегистрации",
        EnumKind::EventLogEntryTransactionMode,
    ),
    (
        "EventLogEntryTransactionMode",
        EnumKind::EventLogEntryTransactionMode,
    ),
    (
        "РежимОтправкиИнформацииОбОшибке",
        EnumKind::ErrorReportingMode,
    ),
    ("ErrorReportingMode", EnumKind::ErrorReportingMode),
    ("ОтправкаЭлементаДанных", EnumKind::DataItemSend),
    ("DataItemSend", EnumKind::DataItemSend),
    ("ПолучениеЭлементаДанных", EnumKind::DataItemReceive),
    ("DataItemReceive", EnumKind::DataItemReceive),
    (
        "ОбновлениеПредопределенныхДанных",
        EnumKind::PredefinedDataUpdate,
    ),
    ("PredefinedDataUpdate", EnumKind::PredefinedDataUpdate),
    ("РежимПроведенияДокумента", EnumKind::DocumentPostingMode),
    ("DocumentPostingMode", EnumKind::DocumentPostingMode),
    (
        "ВидСравненияКомпоновкиДанных",
        EnumKind::DataCompositionComparisonType,
    ),
    (
        "DataCompositionComparisonType",
        EnumKind::DataCompositionComparisonType,
    ),
    (
        "ВариантСтандартнойДатыНачала",
        EnumKind::StandardBeginningDateVariant,
    ),
    (
        "StandardBeginningDateVariant",
        EnumKind::StandardBeginningDateVariant,
    ),
    ("ТипЗначенияJSON", EnumKind::JsonValueType),
    ("JSONValueType", EnumKind::JsonValueType),
    ("ПереносСтрокJSON", EnumKind::JsonLineBreak),
    ("JSONLineBreak", EnumKind::JsonLineBreak),
    ("ЭкранированиеСимволовJSON", EnumKind::JsonEscapeCharacters),
    ("JSONCharactersEscapeMode", EnumKind::JsonEscapeCharacters),
    ("ФорматДатыJSON", EnumKind::JsonDateFormat),
    ("JSONDateFormat", EnumKind::JsonDateFormat),
    ("ВариантЗаписиДатыJSON", EnumKind::JsonDateWritingVariant),
    ("JSONDateWritingVariant", EnumKind::JsonDateWritingVariant),
    ("ТипУзлаXML", EnumKind::XmlNodeType),
    ("XMLNodeType", EnumKind::XmlNodeType),
    ("ТипУзлаDOM", EnumKind::DomNodeType),
    ("DOMNodeType", EnumKind::DomNodeType),
    ("КодировкаТекста", EnumKind::TextEncoding),
    ("TextEncoding", EnumKind::TextEncoding),
    ("СпособКодированияСтроки", EnumKind::StringEncodingMethod),
    ("НаправлениеСортировки", EnumKind::SortDirection),
    ("СостояниеФоновогоЗадания", EnumKind::BackgroundJobState),
    ("BackgroundJobState", EnumKind::BackgroundJobState),
    ("СтатусСообщения", EnumKind::MessageStatus),
    ("MessageStatus", EnumKind::MessageStatus),
    ("КатегорияОшибки", EnumKind::ErrorCategory),
    ("ErrorCategory", EnumKind::ErrorCategory),
    ("BackgroundJobState", EnumKind::BackgroundJobState),
    ("SortDirection", EnumKind::SortDirection),
    ("ЧастиДаты", EnumKind::DateFractions),
    ("DateFractions", EnumKind::DateFractions),
    ("ДопустимаяДлина", EnumKind::AllowedLength),
    ("AllowedLength", EnumKind::AllowedLength),
    ("ХешФункция", EnumKind::HashFunction),
    ("HashFunction", EnumKind::HashFunction),
    ("ТипФайлаТабличногоДокумента", EnumKind::SpreadFileType),
    ("SpreadsheetDocumentFileType", EnumKind::SpreadFileType),
    ("ТипРисункаТабличногоДокумента", EnumKind::DrawingKind),
    ("SpreadsheetDocumentDrawingType", EnumKind::DrawingKind),
    ("ОриентацияСтраницы", EnumKind::PageOrientation),
    ("PageOrientation", EnumKind::PageOrientation),
    ("ПорядокБайтов", EnumKind::ByteOrder),
    ("ByteOrder", EnumKind::ByteOrder),
    ("РежимОткрытияФайла", EnumKind::FileOpenMode),
    ("FileOpenMode", EnumKind::FileOpenMode),
    ("ДоступКФайлу", EnumKind::FileAccess),
    ("FileAccess", EnumKind::FileAccess),
    ("ПозицияВПотоке", EnumKind::StreamPosition),
    ("PositionInStream", EnumKind::StreamPosition),
    ("ТипКомпонентыXS", EnumKind::XsComponentType),
    ("XSComponentType", EnumKind::XsComponentType),
    ("ФормаПредставленияXS", EnumKind::XsForm),
    ("XSForm", EnumKind::XsForm),
    ("ВариантПростогоТипаXS", EnumKind::XsSimpleTypeVariety),
    ("XSSimpleTypeVariety", EnumKind::XsSimpleTypeVariety),
    ("ВидГруппыМоделиXS", EnumKind::XsModelGroupKind),
    ("МетодНаследованияXS", EnumKind::XsDerivationMethod),
    ("XSDerivationMethod", EnumKind::XsDerivationMethod),
    ("ОграничениеЗначенияXS", EnumKind::XsValueConstraint),
    (
        "ОбработкаПробельныхСимволовXS",
        EnumKind::XsWhitespaceHandling,
    ),
    ("XSWhitespaceHandling", EnumKind::XsWhitespaceHandling),
    ("ФормаXML", EnumKind::XmlForm),
    ("XMLForm", EnumKind::XmlForm),
    ("ВидФасетаXDTO", EnumKind::XdtoFacetKind),
    ("XDTOFacetType", EnumKind::XdtoFacetKind),
    ("ТипРезультатаDOMXPath", EnumKind::DomXPathResultType),
    ("DOMXPathResultType", EnumKind::DomXPathResultType),
    ("НаправлениеПоиска", EnumKind::SearchDirection),
    ("SearchDirection", EnumKind::SearchDirection),
    (
        "РежимВосстановленияПутейФайловZIP",
        EnumKind::ZipRestorePathsMode,
    ),
    ("ZIPRestoreFilePathsMode", EnumKind::ZipRestorePathsMode),
    ("ТипСвязиВложенияPDF", EnumKind::PdfAttachmentRelation),
    ("ТипФайлаАрхива", EnumKind::ArchiveFileType),
    ("ArchiveFileType", EnumKind::ArchiveFileType),
    ("МетодСжатияZIP", EnumKind::ZipCompressionMethod),
    ("ZIPCompressionMethod", EnumKind::ZipCompressionMethod),
    ("УровеньСжатияZIP", EnumKind::ZipCompressionLevel),
    ("ZIPCompressionLevel", EnumKind::ZipCompressionLevel),
    ("РежимСохраненияПутейZIP", EnumKind::ZipStorePathMode),
    ("ZIPStorePathMode", EnumKind::ZipStorePathMode),
    (
        "РежимОбработкиПодкаталоговZIP",
        EnumKind::ZipSubDirProcessingMode,
    ),
    ("ZIPSubDirProcessingMode", EnumKind::ZipSubDirProcessingMode),
    ("МетодШифрованияZIP", EnumKind::ZipEncryptionMethod),
    ("ZIPEncryptionMethod", EnumKind::ZipEncryptionMethod),
    (
        "КодировкаИменФайловВZipФайле",
        EnumKind::ZipFileNamesEncoding,
    ),
    ("FileNamesEncodingInZipFile", EnumKind::ZipFileNamesEncoding),
    ("ИспользованиеByteOrderMark", EnumKind::ByteOrderMarkUse),
    ("ByteOrderMarkUse", EnumKind::ByteOrderMarkUse),
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
    fn client_run_mode_matches_the_measured_platform_surface() {
        assert_eq!(
            lookup_member(EnumKind::ClientRunMode, "ManagedApplication"),
            Some(EnumValue::ClientRunModeManagedApplication)
        );
        assert_eq!(EnumValue::ClientRunModeAuto.display_text(), "Авто");
        assert_eq!(
            EnumValue::ClientRunModeOrdinaryApplication.display_text(),
            "Обычное приложение"
        );
        assert_eq!(
            EnumValue::ClientRunModeManagedApplication.display_text(),
            "Управляемое приложение"
        );
        assert_eq!(EnumKind::ClientRunMode.meta_ru_name(), "EnumClientRunMode");
    }

    #[test]
    fn configuration_extension_enums_match_the_measured_platform_surface() {
        assert_eq!(
            lookup_member(EnumKind::ConfigurationExtensionsSource, "SessionApplied"),
            Some(EnumValue::ConfigurationExtensionsSourceSessionApplied)
        );
        assert_eq!(
            EnumValue::ConfigurationExtensionScopeDataSeparation.display_text(),
            "Data separation"
        );
        assert_eq!(
            EnumValue::ConfigurationExtensionApplicationIssueSeverityModerate.display_text(),
            "Moderate"
        );
        assert_eq!(
            EnumKind::ConfigurationExtensionsSource.meta_ru_name(),
            "EnumConfigurationExtensionsSource"
        );
        assert_eq!(
            EnumKind::ConfigurationExtensionScope.meta_ru_name(),
            "EnumConfigurationExtensionScope"
        );
        assert_eq!(
            EnumKind::ConfigurationExtensionApplicationIssueSeverity.meta_ru_name(),
            "EnumConfigurationExtensionApplicationIssueSeverity"
        );
    }

    /// Составные члены потоковых перечислений печатаются раздельно и со
    /// строчной второй частью — измерено, и «уборка» таблицы в слитное
    /// написание идентификатора его бы сломала.
    #[test]
    fn stream_enum_members_print_as_human_text() {
        assert_eq!(
            EnumValue::FileOpenModeCreateNew.display_text(),
            "Создать новый"
        );
        assert_eq!(
            EnumValue::FileOpenModeOpenOrCreate.display_text(),
            "Открыть или создать"
        );
        assert_eq!(
            EnumValue::FileAccessReadAndWrite.display_text(),
            "Чтение и запись"
        );
        assert_eq!(EnumValue::StreamPositionBegin.display_text(), "Начало");
        // Английские написания самих перечислений — не калька с русского.
        assert_eq!(
            lookup_enum("PositionInStream"),
            Some(EnumKind::StreamPosition)
        );
        assert_eq!(
            lookup_member(EnumKind::FileAccess, "ReadAndWrite"),
            Some(EnumValue::FileAccessReadAndWrite)
        );
    }

    #[test]
    fn a_member_of_one_enum_is_not_found_in_another() {
        // `Нет` есть у ПереносСтрокJSON, но не у ТипЗначенияJSON.
        assert!(lookup_member(EnumKind::JsonLineBreak, "Нет").is_some());
        assert!(lookup_member(EnumKind::JsonValueType, "Нет").is_none());
    }
}
