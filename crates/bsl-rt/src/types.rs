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
    ScriptVariant,
    CompatibilityMode,
    ClientRunMode,
    /// Тип члена `РежимОткрытияФормПриложения`.
    ApplicationFormsOpenningMode,
    /// Тип члена `ВариантИнтерфейсаКлиентскогоПриложения`.
    ClientApplicationInterfaceVariant,
    /// Тип члена `ОтображениеПанелиРазделов`.
    SectionsPanelRepresentation,
    ConfigurationExtensionsSource,
    ConfigurationExtensionScope,
    /// Тип назначения расширения; поиск по имени на платформе недоступен.
    ConfigurationExtensionPurpose,
    ConfigurationExtensionApplicationIssueSeverity,
    EventLogLevel,
    EventLogEntryTransactionMode,
    EventLogEntryTransactionStatus,
    ErrorReportingMode,
    DataItemSend,
    DataItemReceive,
    PredefinedDataUpdate,
    DocumentPostingMode,
    DocumentWriteMode,
    QueryResultIteration,
    /// Тип члена перечисления `ТипСвязиНаборовДанныхКомпоновкиДанных`.
    DataCompositionDataSetsLinkType,
    /// Тип члена перечисления `ТипСоединенияЗначенийПоСериямДиаграммы`.
    ChartValuesBySeriesConnectionType,
    /// Тип члена перечисления `РасположениеЛегендыДиаграммыКомпоновкиДанных`.
    DataCompositionChartLegendPlacement,
    /// Тип члена перечисления `ТипЗаписиЗапроса`.
    QueryRecordType,
    DataLockMode,
    RoundMode,
    NumericValueType,
    ErrorMessageDisplayVariant,
    CommandParameterUseMode,
    SearchStringModeOnInputByString,
    RepresentableDocumentBatchFileType,
    FontType,
    PictureType,
    FullTextSearchMetadataUse,
    LinkedValueChangeMode,
    InternetMailAttachmentEncodingMode,
    AutoSaveFormDataInSettings,
    PDFDocumentFileType,
    AllowedSign,
    /// Тип члена `АвтоРегистрацияИзменений`.
    AutoChangeRecord,
    /// Тип члена `ОтображениеКнопки`.
    ButtonRepresentation,
    /// Тип члена `РезультатПроверкиСоответствияПароляПолитике`.
    PasswordPolicyComplianceCheckResult,
    /// Тип члена `ОтображениеТаблицы`.
    TableRepresentation,
    /// Тип члена `ВидПоляФормы`.
    FormFieldType,
    FormGroupType,
    ItemHorizontalLocation,
    ItemVerticalAlign,
    FormItemSpacing,
    ButtonShapeRepresentation,
    UsualGroupRepresentation,
    DisplayImportance,
    PictureFormat,
    InternetMailTextType,
    InternetMailNonAsciiEncodingMode,
    InternetMailMessageImportance,
    /// Тип члена `ДополнительныйРежимОтображения`.
    AdditionalShowMode,
    /// Тип члена `РежимПроверкиОтзываTLSСертификатаСервера`.
    ServerTlsCertificateRevocationCheckMode,
    /// Тип члена `ТипПримененияОтбораКомпоновкиДанных`.
    DataCompositionFilterApplicationType,
    /// Тип члена перечисления `СостояниеЭлементаНастройкиКомпоновкиДанных`.
    DataCompositionSettingsItemState,
    /// Тип члена `РежимОтображенияЭлементаНастройкиКомпоновкиДанных`.
    DataCompositionSettingsItemViewMode,
    /// Тип члена `СпособВосстановленияНастроекКомпоновкиДанных`.
    DataCompositionSettingsRefreshMethod,
    /// Тип члена `ТипВыводаТекстаКомпоновкиДанных`.
    DataCompositionTextOutputType,
    /// Тип члена перечисления `ТипРазмещенияТекстаКомпоновкиДанных`.
    DataCompositionTextPlacementType,
    /// Тип члена `РежимМасштабированияПросмотра`.
    ViewScalingMode,
    /// Тип члена `РасположениеИтоговКомпоновкиДанных`.
    DataCompositionTotalPlacement,
    /// Тип члена `РазмерКартинки`.
    PictureSize,
    /// Тип члена `ТипСмещенияТабличногоДокумента`.
    SpreadsheetDocumentShiftType,
    /// Тип члена `ТипДиаграммы`.
    ChartType,
    /// Тип члена `ОриентацияПодписейДиаграммы`.
    ChartLabelsOrientation,
    /// Тип члена `ВидГраницы`.
    BoundaryType,
    /// Тип члена `МаксимумСерий`.
    MaxSeries,
    /// Тип члена `ВидПодписейКДиаграмме`.
    ChartLabelType,
    /// Тип члена `ТипЛинииДиаграммы`.
    ChartLineType,
    /// Тип члена `ВидКнопкиФормы`.
    FormButtonType,
    XmlTypeAssignment,
    PlatformType,
    DataCompositionComparisonType,
    ComparisonType,
    ClientConnectionSpeed,
    SpreadsheetDocumentCellAreaType,
    AddInConnectionType,
    FillChecking,
    FoldersAndItems,
    XmlCanonicalizationType,
    Pop3AuthenticationMode,
    SmtpAuthenticationMode,
    InternetMailProtocol,
    DataCompositionDetailsProcessingAction,
    StandardBeginningDateVariant,
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
    /// Тип члена перечисления отображения интервала диаграммы Ганта.
    GanttChartIntervalRepresentation,
    /// Тип члена перечисления вертикального растягивания диаграммы Ганта.
    GanttChartVerticalStretch,
    /// Тип члена перечисления расположения легенды диаграммы.
    ChartLegendPlacement,
    /// Тип члена перечисления единиц шкалы времени.
    TimeScaleUnitType,
    /// Тип члена перечисления форматов дня шкалы времени.
    TimeScaleDayFormat,
    PageOrientation,

    TextEncoding,
    /// Тип члена перечисления `СпособКодированияСтроки`.
    StringEncodingMethod,
    /// Тип члена перечисления `НаправлениеСортировки`.
    SortDirection,
    /// Тип члена перечисления `ИспользованиеВывода`.
    UseOutput,
    /// Тип члена перечисления `ОтображениеСтраницФормы`.
    FormPagesRepresentation,
    /// Тип члена перечисления `ПоложениеКартинкиКнопкиФормы`.
    FormButtonPictureLocation,
    /// Тип члена перечисления `ТипЛинииРисункаТабличногоДокумента`.
    SpreadsheetDocumentDrawingLineType,
    /// Тип члена перечисления `ВидФлажка`.
    CheckBoxType,
    /// Тип члена перечисления `ЗвуковоеОповещение`.
    DeliverableNotificationSubscriberType,
    SoundAlert,
    /// Тип члена перечисления `СквозноеВыравнивание`.
    ThroughAlign,
    /// Тип члена перечисления `ФигураКнопки`.
    ButtonShape,
    /// Тип члена перечисления `ТипГруппыЭлементовОтбораКомпоновкиДанных`.
    DataCompositionFilterItemsGroupType,
    /// Тип члена перечисления `Клавиша`.
    Key,
    /// Тип члена перечисления `ТипРамкиЭлементаУправления`.
    ControlBorderType,
    /// Тип члена перечисления `ВидЦвета`.
    ColorType,
    /// Тип члена перечисления `ТипЛинииЯчейкиТабличногоДокумента`.
    SpreadsheetDocumentCellLineType,
    /// Тип члена перечисления `ТипХранилищаСертификатовКриптографии`.
    CryptoCertificateStoreType,
    /// Тип члена перечисления `ОбработкаТекстаИнтернетПочтовогоСообщения`.
    InternetMailTextProcessing,
    /// Тип члена перечисления `ТипДвустороннейПечати`.
    DuplexPrintingType,
    SecondAuthenticationFactorSettingsProcessingType,
    /// Тип члена перечисления `СпособПеретаскиванияФайлов`.
    FileDragMode,
    /// Тип члена перечисления `ОтображениеГруппыКнопок`.
    ButtonGroupRepresentation,
    /// Тип члена перечисления `ВариантСтандартногоПериода`, достигнутого в УНФ.
    StandardPeriodVariant,
    /// Тип члена перечисления `ВариантУправленияВысотойТаблицы`, достигнутого в УНФ.
    TableHeightControlVariant,
    /// Тип члена перечисления `ВерсияПолнотекстовогоПоиска`, достигнутого в УНФ.
    FullTextSearchVersion,
    /// Тип члена перечисления `ВертикальнаяПрокруткаФормы`, достигнутого в УНФ.
    VerticalFormScroll,
    /// Тип члена перечисления `ВертикальноеПоложение`, достигнутого в УНФ.
    VerticalAlign,
    /// Тип члена перечисления `ВидДвиженияНакопления`, достигнутого в УНФ.
    AccumulationRecordType,
    /// Тип значения `ВидДвиженияБухгалтерии`, измеренный на 1С.
    AccountingRecordType,
    /// Тип положения текста относительно картинки, измеренный на 1С.
    TextPositionRelativeToPicture,
    /// Тип члена перечисления `ГоризонтальноеПоложение`, достигнутого в УНФ.
    HorizontalAlign,
    /// Тип члена перечисления `ГруппировкаКолонок`, достигнутого в УНФ.
    ColumnsGroup,
    /// Тип члена перечисления `ДопустимыйНомерСообщения`, достигнутого в УНФ.
    AllowedMessageNo,
    /// Тип члена перечисления `ИспользованиеГруппИЭлементов`, достигнутого в УНФ.
    FoldersAndItemsUse,
    /// Тип члена перечисления `ИспользованиеПараметраКомпоновкиДанных`, достигнутого в УНФ.
    DataCompositionParameterUse,
    /// Тип члена перечисления `ИспользованиеПолосыПрокрутки`, достигнутого в УНФ.
    ScrollBarUse,
    /// Тип члена перечисления `ИспользованиеТекущейСтроки`, достигнутого в УНФ.
    CurrentRowUse,
    /// Тип члена перечисления `НаправлениеПорядкаСхемыЗапроса`, достигнутого в УНФ.
    QuerySchemaOrderDirection,
    /// Тип члена перечисления `НачальноеОтображениеДерева`, достигнутого в УНФ.
    InitialTreeView,
    /// Тип члена перечисления `ОбновлениеПриИзмененииДанных`, достигнутого в УНФ.
    UpdateOnDataChange,
    /// Тип члена перечисления `ОбновлениеТекстаРедактирования`, достигнутого в УНФ.
    EditTextUpdate,
    /// Тип члена перечисления `ОтображениеПредупрежденияПриРедактировании`, достигнутого в УНФ.
    WarningOnEditRepresentation,
    /// Тип члена перечисления `ПоискВТаблицеПриВводе`, достигнутого в УНФ.
    SearchInTableOnInput,
    /// Тип члена перечисления `ПоложениеКоманднойПанелиФормы`, достигнутого в УНФ.
    FormCommandBarLabelLocation,
    /// Тип члена перечисления `ПоложениеКоманднойПанелиЭлементаФормы`, достигнутого в УНФ.
    FormItemCommandBarLabelLocation,
    /// Тип члена перечисления `ПоложениеПодписейШкалыЗначенийИзмерительнойДиаграммы`, достигнутого в УНФ.
    GaugeChartValuesScaleLabelsLocation,
    /// Тип положения подписи, достигнутого в модуле менеджера отчёта БУХ.
    ChartLabelLocation,
    /// Тип положения подписей шкалы, достигнутого в отчёте БУХ.
    ChartScaleLabelLocation,
    /// Отображение в легенде диаграммы.
    ShowInChartLegend,
    /// Тип использования расшифровки табличного документа.
    SpreadsheetDocumentDetailUse,
    /// Тип члена перечисления `ПоложениеСостоянияПросмотра`, достигнутого в УНФ.
    ViewStatusLocation,
    /// Тип члена перечисления `ПоложениеСтрокиПоиска`, достигнутого в УНФ.
    SearchStringLocation,
    /// Тип члена перечисления `ПоложениеУправленияПоиском`, достигнутого в УНФ.
    SearchControlLocation,
    /// Тип члена перечисления `РежимАвтоВремя`, достигнутого в УНФ.
    AutoTimeMode,
    /// Тип члена перечисления `РежимВводаСтрокТаблицы`, достигнутого в УНФ.
    TableRowInputMode,
    /// Тип члена перечисления `РежимВключенияСертификатовКриптографии`, достигнутого в УНФ.
    CryptoCertificateIncludeMode,
    /// Тип члена перечисления `РежимВыделенияСтрокиТаблицы`, достигнутого в УНФ.
    TableRowSelectionMode,
    /// Тип члена перечисления `РежимВыделенияТаблицы`, достигнутого в УНФ.
    TableSelectionMode,
    /// Тип члена перечисления `РежимЗамещения`, достигнутого в УНФ.
    ReplacementMode,
    /// Тип члена перечисления `РежимИзмененияРазмера`, достигнутого в УНФ.
    SizeChangeMode,
    /// Тип члена перечисления `РежимОткрытияОкнаФормы`, достигнутого в УНФ.
    FormWindowOpeningMode,
    /// Тип члена перечисления `РежимОтображенияНастроекКомпоновкиДанных`, достигнутого в УНФ.
    DataCompositionSettingsViewMode,
    /// Тип члена перечисления `РежимОтображенияРезультатаОтчета`, достигнутого в УНФ.
    ReportResultViewMode,
    /// Тип члена перечисления `РежимПолнотекстовогоПоиска`, достигнутого в УНФ.
    FullTextSearchMode,
    /// Тип члена перечисления `РежимПроверкиСертификатаКриптографии`, достигнутого в УНФ.
    CryptoCertificateCheckMode,
    /// Тип члена перечисления `РежимРедактированияКолонки`, достигнутого в УНФ.
    ColumnEditMode,
    /// Тип члена перечисления `СпособВосстановленияПароляПользователяИнформационнойБазы`, достигнутого в УНФ.
    InfoBaseUserPasswordRecoveryMethod,
    /// Тип члена перечисления `ТипЗаполненияОбластиТабличногоДокумента`, достигнутого в УНФ.
    SpreadsheetDocumentAreaFillType,
    /// Тип члена перечисления `ТипОбъединенияСхемыЗапроса`, достигнутого в УНФ.
    QuerySchemaUnionType,
    /// Тип члена перечисления `ТипПодписиКриптографии`, достигнутого в УНФ.
    CryptoSignatureType,
    /// Тип члена перечисления `ТипРазмещенияТекстаТабличногоДокумента`, достигнутого в УНФ.
    SpreadsheetDocumentTextPlacementType,
    /// Тип члена перечисления `ТипСоединенияСхемыЗапроса`, достигнутого в УНФ.
    QuerySchemaJoinType,
    /// Тип члена перечисления `ТипФормыОтчета`, достигнутого в УНФ.
    ReportFormType,
    /// Тип члена перечисления `ТипЭлементаФорматированногоДокумента`, достигнутого в УНФ.
    FormattedDocumentItemType,
    /// Тип члена перечисления `ФиксацияВТаблице`, достигнутого в УНФ.
    FixingInTable,
    /// Тип члена перечисления `НаправлениеСортировкиКомпоновкиДанных`.
    DataCompositionSortDirection,
    /// Тип члена перечисления `ТипГруппировкиКомпоновкиДанных`.
    DataCompositionGroupType,
    /// Тип члена перечисления `РасположениеГруппировкиКомпоновкиДанных`.
    DataCompositionGroupPlacement,
    /// Тип члена перечисления `РасположениеПолейГруппировкиКомпоновкиДанных`.
    DataCompositionGroupFieldsPlacement,
    /// Тип члена перечисления `РасположениеРеквизитовКомпоновкиДанных`.
    DataCompositionAttributesPlacement,
    /// Тип члена перечисления `ВидДекорацииФормы`.
    FormDecorationType,
    /// Тип члена перечисления `ВидПереключателя`.
    RadioButtonType,
    /// Тип члена перечисления `ГруппировкаПодчиненныхЭлементовФормы`.
    ChildFormItemsGroup,
    /// Тип члена перечисления `ИсторияВыбораПриВводе`.
    ChoiceHistoryOnInput,
    /// Тип члена перечисления `ОтображениеКнопкиВыбора`.
    ChoiceButtonRepresentation,
    /// Тип члена перечисления `ОтображениеУправленияОбычнойГруппы`.
    UsualGroupControlRepresentation,
    /// Тип члена перечисления `ПоведениеОбычнойГруппы`.
    UsualGroupBehavior,
    /// Тип члена перечисления `ПоложениеЗаголовкаЭлементаФормы`.
    FormItemTitleLocation,
    /// Тип члена перечисления `ПоложениеКнопкиВКоманднойПанели`.
    ButtonLocationInCommandBar,
    /// Тип члена перечисления `РасположениеРесурсовКомпоновкиДанных`.
    DataCompositionResourcesPlacement,
    /// Тип члена перечисления `ИспользованиеУсловногоОформленияКомпоновкиДанных`.
    DataCompositionConditionalAppearanceUse,
    /// Тип члена перечисления `РасположениеПоляКомпоновкиДанных`.
    DataCompositionFieldPlacement,
    /// Тип члена перечисления `ТипБухгалтерскогоОстаткаКомпоновкиДанных`.
    DataCompositionAccountingBalanceType,
    /// Тип члена перечисления `ТипОстаткаКомпоновкиДанных`.
    DataCompositionBalanceType,
    /// Тип члена перечисления `ТипПериодаКомпоновкиДанных`.
    DataCompositionPeriodType,
    /// Тип члена перечисления `ТипДополненияПериодаКомпоновкиДанных`.
    DataCompositionPeriodAdditionType,
    /// Тип члена перечисления `ГлубинаЦвета`.
    ColorDepth,
    /// Тип члена перечисления `ОтображениеПодсказки`.
    ToolTipRepresentation,
    /// Тип члена перечисления `ТипКлиентскогоПриложения`.
    ClientApplicationType,
    BackgroundJobState,
    MessageStatus,
    ErrorCategory,
    /// Тип члена перечисления `ЧастиДаты`.
    DateFractions,
    /// Тип члена перечисления `ДопустимаяДлина`.
    AllowedLength,
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
    /// Режим восстановления путей нового API архивов.
    ArchiveFileRestorePathsMode,
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
    /// Тип файла форматированного документа.
    FormattedDocumentFileType,
    /// Тип ЧЛЕНА перечисления `ИспользованиеByteOrderMark`.
    ByteOrderMarkUse,
    /// Тип члена перечисления `ВидСчета`.
    AccountType,
    /// Измеренный тип положения картинки элемента графической схемы.
    GraphicalSchemaItemPictureLocation,
    /// Измеренный тип `ФигурыГрафическойСхемы`.
    GraphicalSchemaShapes,
    /// Измеренный тип `ТипСоединительнойЛинии`.
    ConnectorLineType,
    /// Измеренный тип `ПоложениеТекстаСоединительнойЛинии`.
    ConnectorTextLocation,
    /// Измеренный тип `СтильСтрелки`.
    ArrowStyle,
    /// Тип значения перечислимого свойства `РазделениеДанныхОбщегоРеквизита`.
    CommonAttributeDataSeparation,
    /// Тип значения перечислимого свойства `АвтоИспользованиеОбщегоРеквизита`.
    CommonAttributeAutoUse,
    /// Тип значения перечислимого свойства `ИспользованиеОбщегоРеквизита`.
    CommonAttributeUse,
    /// Тип узора табличного документа.
    SpreadsheetDocumentPatternType,
    /// Тип расположения области построения диаграммы.
    ChartPlotAreaPlacement,
    /// Алгоритм подписи токена доступа.
    AccessTokenSignAlgorithm,
}

/// `(русское, английское)`. Русское — каноническое: именно оно уходит в
/// `Строка(ТипЗнч(...))`.
const NAMES: &[(TypeId, &str, &str)] = &[
    (
        TypeId::AccessTokenSignAlgorithm,
        "АлгоритмПодписиТокенаДоступа",
        "AccessTokenSignAlgorithm",
    ),
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
    (
        TypeId::ScriptVariant,
        "ВариантВстроенногоЯзыка",
        "ScriptVariant",
    ),
    (
        TypeId::ClientRunMode,
        "РежимЗапускаКлиентскогоПриложения",
        "ClientRunMode",
    ),
    (
        TypeId::ApplicationFormsOpenningMode,
        "РежимОткрытияФормПриложения",
        "ApplicationFormsOpenningMode",
    ),
    (
        TypeId::ClientApplicationInterfaceVariant,
        "ВариантИнтерфейсаКлиентскогоПриложения",
        "ClientApplicationInterfaceVariant",
    ),
    (
        TypeId::SectionsPanelRepresentation,
        "ОтображениеПанелиРазделов",
        "SectionsPanelRepresentation",
    ),
    (
        TypeId::ConfigurationExtensionsSource,
        "ИсточникРасширенийКонфигурации",
        "ConfigurationExtensionsSource",
    ),
    (
        TypeId::ConfigurationExtensionScope,
        "ОбластьДействияРасширенияКонфигурации",
        "ConfigurationExtensionScope",
    ),
    (
        TypeId::ConfigurationExtensionApplicationIssueSeverity,
        "ВажностьПроблемыПримененияРасширенияКонфигурации",
        "ConfigurationExtensionApplicationIssueSeverity",
    ),
    (
        TypeId::EventLogLevel,
        "УровеньЖурналаРегистрации",
        "EventLogLevel",
    ),
    (
        TypeId::EventLogEntryTransactionMode,
        "РежимТранзакцииЗаписиЖурналаРегистрации",
        "EventLogEntryTransactionMode",
    ),
    (
        TypeId::EventLogEntryTransactionStatus,
        "СтатусТранзакцииЗаписиЖурналаРегистрации",
        "EventLogEntryTransactionStatus",
    ),
    (
        TypeId::ErrorReportingMode,
        "РежимОтправкиИнформацииОбОшибке",
        "ErrorReportingMode",
    ),
    (
        TypeId::DataItemSend,
        "ОтправкаЭлементаДанных",
        "DataItemSend",
    ),
    (
        TypeId::DataItemReceive,
        "ПолучениеЭлементаДанных",
        "DataItemReceive",
    ),
    (
        TypeId::PredefinedDataUpdate,
        "ОбновлениеПредопределенныхДанных",
        "PredefinedDataUpdate",
    ),
    (
        TypeId::DocumentPostingMode,
        "РежимПроведенияДокумента",
        "DocumentPostingMode",
    ),
    (
        TypeId::DocumentWriteMode,
        "РежимЗаписиДокумента",
        "DocumentWriteMode",
    ),
    (
        TypeId::DataCompositionDataSetsLinkType,
        "ТипСвязиНаборовДанныхКомпоновкиДанных",
        "DataCompositionDataSetsLinkType",
    ),
    (
        TypeId::ChartValuesBySeriesConnectionType,
        "ТипСоединенияЗначенийПоСериямДиаграммы",
        "ChartValuesBySeriesConnectionType",
    ),
    (
        TypeId::DataCompositionChartLegendPlacement,
        "РасположениеЛегендыДиаграммыКомпоновкиДанных",
        "DataCompositionChartLegendPlacement",
    ),
    (
        TypeId::QueryRecordType,
        "ТипЗаписиЗапроса",
        "QueryRecordType",
    ),
    (
        TypeId::QueryResultIteration,
        "ОбходРезультатаЗапроса",
        "QueryResultIteration",
    ),
    (
        TypeId::DataLockMode,
        "РежимБлокировкиДанных",
        "DataLockMode",
    ),
    (TypeId::AccountType, "ВидСчета", "AccountType"),
    (TypeId::RoundMode, "РежимОкругления", "RoundMode"),
    (
        TypeId::NumericValueType,
        "ВидЧисловогоЗначения",
        "NumericValueType",
    ),
    (
        TypeId::ErrorMessageDisplayVariant,
        "ВариантОтображенияСообщенияОбОшибке",
        "ErrorMessageDisplayVariant",
    ),
    (
        TypeId::CommandParameterUseMode,
        "РежимИспользованияПараметраКоманды",
        "CommandParameterUseMode",
    ),
    (
        TypeId::SearchStringModeOnInputByString,
        "СпособПоискаСтрокиПриВводеПоСтроке",
        "SearchStringModeOnInputByString",
    ),
    (
        TypeId::RepresentableDocumentBatchFileType,
        "ТипФайлаПакетаОтображаемыхДокументов",
        "RepresentableDocumentBatchFileType",
    ),
    (TypeId::FontType, "ВидШрифта", "FontType"),
    (TypeId::PictureType, "ВидКартинки", "PictureType"),
    (
        TypeId::FullTextSearchMetadataUse,
        "ИспользованиеМетаданныхПолнотекстовогоПоиска",
        "FullTextSearchMetadataUse",
    ),
    (
        TypeId::LinkedValueChangeMode,
        "РежимИзмененияСвязанногоЗначения",
        "LinkedValueChangeMode",
    ),
    (
        TypeId::InternetMailAttachmentEncodingMode,
        "СпособКодированияИнтернетПочтовогоВложения",
        "InternetMailAttachmentEncodingMode",
    ),
    (
        TypeId::AutoSaveFormDataInSettings,
        "АвтоматическоеСохранениеДанныхФормыВНастройках",
        "AutoSaveFormDataInSettings",
    ),
    (
        TypeId::PDFDocumentFileType,
        "ТипФайлаДокументаPDF",
        "PDFDocumentFileType",
    ),
    (TypeId::AllowedSign, "ДопустимыйЗнак", "AllowedSign"),
    (
        TypeId::AutoChangeRecord,
        "АвтоРегистрацияИзменений",
        "AutoChangeRecord",
    ),
    (
        TypeId::ButtonRepresentation,
        "ОтображениеКнопки",
        "ButtonRepresentation",
    ),
    (
        TypeId::PasswordPolicyComplianceCheckResult,
        "РезультатПроверкиСоответствияПароляПолитике",
        "PasswordPolicyComplianceCheckResult",
    ),
    (
        TypeId::TableRepresentation,
        "ОтображениеТаблицы",
        "TableRepresentation",
    ),
    (TypeId::FormFieldType, "ВидПоляФормы", "FormFieldType"),
    (TypeId::FormGroupType, "ВидГруппыФормы", "FormGroupType"),
    (
        TypeId::ItemHorizontalLocation,
        "ГоризонтальноеПоложениеЭлемента",
        "ItemHorizontalLocation",
    ),
    (
        TypeId::ItemVerticalAlign,
        "ВертикальноеПоложениеЭлемента",
        "ItemVerticalAlign",
    ),
    (
        TypeId::FormItemSpacing,
        "ИнтервалМеждуЭлементамиФормы",
        "FormItemSpacing",
    ),
    (
        TypeId::ButtonShapeRepresentation,
        "ОтображениеФигурыКнопки",
        "ButtonShapeRepresentation",
    ),
    (
        TypeId::UsualGroupRepresentation,
        "ОтображениеОбычнойГруппы",
        "UsualGroupRepresentation",
    ),
    (
        TypeId::DisplayImportance,
        "ВажностьПриОтображении",
        "DisplayImportance",
    ),
    (TypeId::PictureFormat, "ФорматКартинки", "PictureFormat"),
    (
        TypeId::InternetMailTextType,
        "ТипТекстаПочтовогоСообщения",
        "InternetMailTextType",
    ),
    (
        TypeId::InternetMailNonAsciiEncodingMode,
        "СпособКодированияНеASCIIСимволовИнтернетПочтовогоСообщения",
        "InternetMailMessageNonASCIISymbolsEncodingMode",
    ),
    (
        TypeId::InternetMailMessageImportance,
        "ВажностьИнтернетПочтовогоСообщения",
        "InternetMailMessageImportance",
    ),
    (
        TypeId::AdditionalShowMode,
        "ДополнительныйРежимОтображения",
        "AdditionalShowMode",
    ),
    (
        TypeId::ServerTlsCertificateRevocationCheckMode,
        "РежимПроверкиОтзываTLSСертификатаСервера",
        "ServerTLSCertificateRevocationCheckMode",
    ),
    (
        TypeId::DataCompositionFilterApplicationType,
        "ТипПримененияОтбораКомпоновкиДанных",
        "DataCompositionFilterApplicationType",
    ),
    (
        TypeId::DataCompositionSettingsItemState,
        "СостояниеЭлементаНастройкиКомпоновкиДанных",
        "DataCompositionSettingsItemState",
    ),
    (
        TypeId::DataCompositionSettingsItemViewMode,
        "РежимОтображенияЭлементаНастройкиКомпоновкиДанных",
        "DataCompositionSettingsItemViewMode",
    ),
    (
        TypeId::DataCompositionSettingsRefreshMethod,
        "СпособВосстановленияНастроекКомпоновкиДанных",
        "DataCompositionSettingsRefreshMethod",
    ),
    (
        TypeId::DataCompositionTextPlacementType,
        "ТипРазмещенияТекстаКомпоновкиДанных",
        "DataCompositionTextPlacementType",
    ),
    (
        TypeId::DataCompositionTextOutputType,
        "ТипВыводаТекстаКомпоновкиДанных",
        "DataCompositionTextOutputType",
    ),
    (
        TypeId::ViewScalingMode,
        "РежимМасштабированияПросмотра",
        "ViewScalingMode",
    ),
    (
        TypeId::DataCompositionTotalPlacement,
        "РасположениеИтоговКомпоновкиДанных",
        "DataCompositionTotalPlacement",
    ),
    (TypeId::PictureSize, "РазмерКартинки", "PictureSize"),
    (
        TypeId::SpreadsheetDocumentShiftType,
        "ТипСмещенияТабличногоДокумента",
        "SpreadsheetDocumentShiftType",
    ),
    (TypeId::ChartType, "ТипДиаграммы", "ChartType"),
    (
        TypeId::ChartLabelsOrientation,
        "ОриентацияПодписейДиаграммы",
        "ChartLabelsOrientation",
    ),
    (TypeId::BoundaryType, "ВидГраницы", "BoundaryType"),
    (TypeId::MaxSeries, "МаксимумСерий", "MaxSeries"),
    (
        TypeId::ChartLabelType,
        "ВидПодписейКДиаграмме",
        "ChartLabelType",
    ),
    (TypeId::ChartLineType, "ТипЛинииДиаграммы", "ChartLineType"),
    (TypeId::FormButtonType, "ВидКнопкиФормы", "FormButtonType"),
    (
        TypeId::XmlTypeAssignment,
        "НазначениеТипаXML",
        "XMLTypeAssignment",
    ),
    (TypeId::PlatformType, "ТипПлатформы", "PlatformType"),
    (
        TypeId::DataCompositionComparisonType,
        "ВидСравненияКомпоновкиДанных",
        "DataCompositionComparisonType",
    ),
    (TypeId::ComparisonType, "ВидСравнения", "ComparisonType"),
    (
        TypeId::ClientConnectionSpeed,
        "СкоростьКлиентскогоСоединения",
        "ClientConnectionSpeed",
    ),
    (
        TypeId::SpreadsheetDocumentCellAreaType,
        "ТипОбластиЯчеекТабличногоДокумента",
        "SpreadsheetDocumentCellAreaType",
    ),
    (
        TypeId::AddInConnectionType,
        "ТипПодключенияВнешнейКомпоненты",
        "AddInConnectionType",
    ),
    (TypeId::FillChecking, "ПроверкаЗаполнения", "FillChecking"),
    (
        TypeId::FoldersAndItems,
        "ГруппыИЭлементы",
        "FoldersAndItems",
    ),
    (
        TypeId::XmlCanonicalizationType,
        "ТипКаноническогоXML",
        "XMLCanonicalizationType",
    ),
    (
        TypeId::Pop3AuthenticationMode,
        "СпособPOP3Аутентификации",
        "POP3AuthenticationMode",
    ),
    (
        TypeId::SmtpAuthenticationMode,
        "СпособSMTPАутентификации",
        "SMTPAuthenticationMode",
    ),
    (
        TypeId::InternetMailProtocol,
        "ПротоколИнтернетПочты",
        "InternetMailProtocol",
    ),
    (
        TypeId::DataCompositionDetailsProcessingAction,
        "ДействиеОбработкиРасшифровкиКомпоновкиДанных",
        "DataCompositionDetailsProcessingAction",
    ),
    (
        TypeId::StandardBeginningDateVariant,
        "ВариантСтандартнойДатыНачала",
        "StandardBeginningDateVariant",
    ),
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
        TypeId::GanttChartIntervalRepresentation,
        "ОтображениеИнтервалаДиаграммыГанта",
        "GanttChartIntervalRepresentation",
    ),
    (
        TypeId::ChartLegendPlacement,
        "РасположениеЛегендыДиаграммы",
        "ChartLegendPlacement",
    ),
    (
        TypeId::GanttChartVerticalStretch,
        "РастягиваниеПоВертикалиДиаграммыГанта",
        "GanttChartVerticalStretch",
    ),
    (
        TypeId::TimeScaleUnitType,
        "ТипЕдиницыШкалыВремени",
        "TimeScaleUnitType",
    ),
    (
        TypeId::TimeScaleDayFormat,
        "ФорматДняШкалыВремени",
        "TimeScaleDayFormat",
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
    (TypeId::UseOutput, "UseOutput", "UseOutput"),
    (
        TypeId::FormPagesRepresentation,
        "FormPagesRepresentation",
        "FormPagesRepresentation",
    ),
    (
        TypeId::GraphicalSchemaShapes,
        "ФигурыГрафическойСхемы",
        "GraphicalSchemaShapes",
    ),
    (
        TypeId::ConnectorLineType,
        "ТипСоединительнойЛинии",
        "ConnectorLineType",
    ),
    (
        TypeId::ConnectorTextLocation,
        "ПоложениеТекстаСоединительнойЛинии",
        "ConnectorTextLocation",
    ),
    (TypeId::ArrowStyle, "СтильСтрелки", "ArrowStyle"),
    (
        TypeId::GraphicalSchemaItemPictureLocation,
        "ПоложениеКартинкиЭлементаГрафическойСхемы",
        "GraphicalSchemaItemPictureLocation",
    ),
    (
        TypeId::FormButtonPictureLocation,
        "FormButtonPictureLocation",
        "FormButtonPictureLocation",
    ),
    (
        TypeId::SpreadsheetDocumentDrawingLineType,
        "SpreadsheetDocumentDrawingLineType",
        "SpreadsheetDocumentDrawingLineType",
    ),
    (TypeId::CheckBoxType, "CheckBoxType", "CheckBoxType"),
    (
        TypeId::DeliverableNotificationSubscriberType,
        "DeliverableNotificationSubscriberType",
        "DeliverableNotificationSubscriberType",
    ),
    (TypeId::SoundAlert, "SoundAlert", "SoundAlert"),
    (TypeId::ThroughAlign, "ThroughAlign", "ThroughAlign"),
    (TypeId::ButtonShape, "ButtonShape", "ButtonShape"),
    (
        TypeId::DataCompositionFilterItemsGroupType,
        "DataCompositionFilterItemsGroupType",
        "DataCompositionFilterItemsGroupType",
    ),
    (TypeId::Key, "Key", "Key"),
    (
        TypeId::ControlBorderType,
        "ControlBorderType",
        "ControlBorderType",
    ),
    (TypeId::ColorType, "ColorType", "ColorType"),
    (
        TypeId::SpreadsheetDocumentCellLineType,
        "SpreadsheetDocumentCellLineType",
        "SpreadsheetDocumentCellLineType",
    ),
    (
        TypeId::CryptoCertificateStoreType,
        "CryptoCertificateStoreType",
        "CryptoCertificateStoreType",
    ),
    (
        TypeId::InternetMailTextProcessing,
        "InternetMailTextProcessing",
        "InternetMailTextProcessing",
    ),
    (
        TypeId::DuplexPrintingType,
        "DuplexPrintingType",
        "DuplexPrintingType",
    ),
    (
        TypeId::SecondAuthenticationFactorSettingsProcessingType,
        "ТипОбработкиНастроекВторогоФактораАутентификации",
        "SecondAuthenticationFactorSettingsProcessingType",
    ),
    (TypeId::FileDragMode, "FileDragMode", "FileDragMode"),
    (
        TypeId::ButtonGroupRepresentation,
        "ButtonGroupRepresentation",
        "ButtonGroupRepresentation",
    ),
    (
        TypeId::StandardPeriodVariant,
        "StandardPeriodVariant",
        "StandardPeriodVariant",
    ),
    (
        TypeId::TableHeightControlVariant,
        "TableHeightControlVariant",
        "TableHeightControlVariant",
    ),
    (
        TypeId::FullTextSearchVersion,
        "FullTextSearchVersion",
        "FullTextSearchVersion",
    ),
    (
        TypeId::VerticalFormScroll,
        "VerticalFormScroll",
        "VerticalFormScroll",
    ),
    (TypeId::VerticalAlign, "VerticalAlign", "VerticalAlign"),
    (
        TypeId::AccountingRecordType,
        "ВидДвиженияБухгалтерии",
        "AccountingRecordType",
    ),
    (
        TypeId::TextPositionRelativeToPicture,
        "ПоложениеТекстаОтносительноКартинки",
        "TextPositionRelativeToPicture",
    ),
    (
        TypeId::AccumulationRecordType,
        "AccumulationRecordType",
        "AccumulationRecordType",
    ),
    (
        TypeId::HorizontalAlign,
        "HorizontalAlign",
        "HorizontalAlign",
    ),
    (TypeId::ColumnsGroup, "ColumnsGroup", "ColumnsGroup"),
    (
        TypeId::AllowedMessageNo,
        "AllowedMessageNo",
        "AllowedMessageNo",
    ),
    (
        TypeId::FoldersAndItemsUse,
        "FoldersAndItemsUse",
        "FoldersAndItemsUse",
    ),
    (
        TypeId::DataCompositionParameterUse,
        "DataCompositionParameterUse",
        "DataCompositionParameterUse",
    ),
    (TypeId::ScrollBarUse, "ScrollBarUse", "ScrollBarUse"),
    (TypeId::CurrentRowUse, "CurrentRowUse", "CurrentRowUse"),
    (
        TypeId::QuerySchemaOrderDirection,
        "НаправлениеПорядкаСхемыЗапроса",
        "QuerySchemaOrderDirection",
    ),
    (
        TypeId::InitialTreeView,
        "InitialTreeView",
        "InitialTreeView",
    ),
    (
        TypeId::UpdateOnDataChange,
        "UpdateOnDataChange",
        "UpdateOnDataChange",
    ),
    (TypeId::EditTextUpdate, "EditTextUpdate", "EditTextUpdate"),
    (
        TypeId::WarningOnEditRepresentation,
        "WarningOnEditRepresentation",
        "WarningOnEditRepresentation",
    ),
    (
        TypeId::SearchInTableOnInput,
        "SearchInTableOnInput",
        "SearchInTableOnInput",
    ),
    (
        TypeId::FormCommandBarLabelLocation,
        "FormCommandBarLabelLocation",
        "FormCommandBarLabelLocation",
    ),
    (
        TypeId::FormItemCommandBarLabelLocation,
        "FormItemCommandBarLabelLocation",
        "FormItemCommandBarLabelLocation",
    ),
    (
        TypeId::GaugeChartValuesScaleLabelsLocation,
        "GaugeChartValuesScaleLabelsLocation",
        "GaugeChartValuesScaleLabelsLocation",
    ),
    (
        TypeId::ShowInChartLegend,
        "ОтображениеВЛегендеДиаграммы",
        "ShowInChartLegend",
    ),
    (
        TypeId::ChartPlotAreaPlacement,
        "РасположениеОбластиПостроенияДиаграммы",
        "ChartPlotAreaPlacement",
    ),
    (
        TypeId::SpreadsheetDocumentPatternType,
        "ТипУзораТабличногоДокумента",
        "SpreadsheetDocumentPatternType",
    ),
    (
        TypeId::ChartScaleLabelLocation,
        "ПоложениеПодписейШкалыДиаграммы",
        "ChartScaleLabelLocation",
    ),
    (
        TypeId::SpreadsheetDocumentDetailUse,
        "ИспользованиеРасшифровкиТабличногоДокумента",
        "SpreadsheetDocumentDetailUse",
    ),
    (
        TypeId::ChartLabelLocation,
        "ПоложениеПодписейКДиаграмме",
        "ChartLabelLocation",
    ),
    (
        TypeId::ViewStatusLocation,
        "ViewStatusLocation",
        "ViewStatusLocation",
    ),
    (
        TypeId::SearchStringLocation,
        "SearchStringLocation",
        "SearchStringLocation",
    ),
    (
        TypeId::SearchControlLocation,
        "SearchControlLocation",
        "SearchControlLocation",
    ),
    (TypeId::AutoTimeMode, "AutoTimeMode", "AutoTimeMode"),
    (
        TypeId::TableRowInputMode,
        "TableRowInputMode",
        "TableRowInputMode",
    ),
    (
        TypeId::CryptoCertificateIncludeMode,
        "CryptoCertificateIncludeMode",
        "CryptoCertificateIncludeMode",
    ),
    (
        TypeId::TableRowSelectionMode,
        "TableRowSelectionMode",
        "TableRowSelectionMode",
    ),
    (
        TypeId::TableSelectionMode,
        "TableSelectionMode",
        "TableSelectionMode",
    ),
    (
        TypeId::ReplacementMode,
        "ReplacementMode",
        "ReplacementMode",
    ),
    (TypeId::SizeChangeMode, "SizeChangeMode", "SizeChangeMode"),
    (
        TypeId::FormWindowOpeningMode,
        "FormWindowOpeningMode",
        "FormWindowOpeningMode",
    ),
    (
        TypeId::DataCompositionSettingsViewMode,
        "DataCompositionSettingsViewMode",
        "DataCompositionSettingsViewMode",
    ),
    (
        TypeId::ReportResultViewMode,
        "ReportResultViewMode",
        "ReportResultViewMode",
    ),
    (
        TypeId::FullTextSearchMode,
        "FullTextSearchMode",
        "FullTextSearchMode",
    ),
    (
        TypeId::CryptoCertificateCheckMode,
        "CryptoCertificateCheckMode",
        "CryptoCertificateCheckMode",
    ),
    (TypeId::ColumnEditMode, "ColumnEditMode", "ColumnEditMode"),
    (
        TypeId::InfoBaseUserPasswordRecoveryMethod,
        "InfoBaseUserPasswordRecoveryMethod",
        "InfoBaseUserPasswordRecoveryMethod",
    ),
    (
        TypeId::SpreadsheetDocumentAreaFillType,
        "SpreadsheetDocumentAreaFillType",
        "SpreadsheetDocumentAreaFillType",
    ),
    (
        TypeId::QuerySchemaUnionType,
        "QuerySchemaUnionType",
        "QuerySchemaUnionType",
    ),
    (
        TypeId::CryptoSignatureType,
        "CryptoSignatureType",
        "CryptoSignatureType",
    ),
    (
        TypeId::SpreadsheetDocumentTextPlacementType,
        "SpreadsheetDocumentTextPlacementType",
        "SpreadsheetDocumentTextPlacementType",
    ),
    (
        TypeId::QuerySchemaJoinType,
        "QuerySchemaJoinType",
        "QuerySchemaJoinType",
    ),
    (TypeId::ReportFormType, "ReportFormType", "ReportFormType"),
    (
        TypeId::FormattedDocumentItemType,
        "ТипЭлементаФорматированногоДокумента",
        "FormattedDocumentItemType",
    ),
    (TypeId::FixingInTable, "FixingInTable", "FixingInTable"),
    (
        TypeId::DataCompositionSortDirection,
        "DataCompositionSortDirection",
        "DataCompositionSortDirection",
    ),
    (
        TypeId::DataCompositionGroupType,
        "DataCompositionGroupType",
        "DataCompositionGroupType",
    ),
    (
        TypeId::DataCompositionGroupPlacement,
        "РасположениеГруппировкиКомпоновкиДанных",
        "DataCompositionGroupPlacement",
    ),
    (
        TypeId::DataCompositionGroupFieldsPlacement,
        "DataCompositionGroupFieldsPlacement",
        "DataCompositionGroupFieldsPlacement",
    ),
    (
        TypeId::DataCompositionAttributesPlacement,
        "DataCompositionAttributesPlacement",
        "DataCompositionAttributesPlacement",
    ),
    (
        TypeId::FormDecorationType,
        "FormDecorationType",
        "FormDecorationType",
    ),
    (
        TypeId::RadioButtonType,
        "RadioButtonType",
        "RadioButtonType",
    ),
    (
        TypeId::ChildFormItemsGroup,
        "ChildFormItemsGroup",
        "ChildFormItemsGroup",
    ),
    (
        TypeId::ChoiceHistoryOnInput,
        "ChoiceHistoryOnInput",
        "ChoiceHistoryOnInput",
    ),
    (
        TypeId::ChoiceButtonRepresentation,
        "ChoiceButtonRepresentation",
        "ChoiceButtonRepresentation",
    ),
    (
        TypeId::UsualGroupControlRepresentation,
        "UsualGroupControlRepresentation",
        "UsualGroupControlRepresentation",
    ),
    (
        TypeId::UsualGroupBehavior,
        "UsualGroupBehavior",
        "UsualGroupBehavior",
    ),
    (
        TypeId::FormItemTitleLocation,
        "FormItemTitleLocation",
        "FormItemTitleLocation",
    ),
    (
        TypeId::ButtonLocationInCommandBar,
        "ButtonLocationInCommandBar",
        "ButtonLocationInCommandBar",
    ),
    (
        TypeId::DataCompositionResourcesPlacement,
        "DataCompositionResourcesPlacement",
        "DataCompositionResourcesPlacement",
    ),
    (
        TypeId::CommonAttributeDataSeparation,
        "Common attribute data separation",
        "РазделениеДанныхОбщегоРеквизита",
    ),
    (
        TypeId::CommonAttributeAutoUse,
        "Use a common attribute automatically",
        "АвтоИспользованиеОбщегоРеквизита",
    ),
    (
        TypeId::CommonAttributeUse,
        "Use a common attribute",
        "ИспользованиеОбщегоРеквизита",
    ),
    (
        TypeId::DataCompositionConditionalAppearanceUse,
        "DataCompositionConditionalAppearanceUse",
        "ИспользованиеУсловногоОформленияКомпоновкиДанных",
    ),
    (
        TypeId::DataCompositionFieldPlacement,
        "DataCompositionFieldPlacement",
        "РасположениеПоляКомпоновкиДанных",
    ),
    (
        TypeId::DataCompositionAccountingBalanceType,
        "DataCompositionAccountingBalanceType",
        "ТипБухгалтерскогоОстаткаКомпоновкиДанных",
    ),
    (
        TypeId::DataCompositionBalanceType,
        "DataCompositionBalanceType",
        "ТипОстаткаКомпоновкиДанных",
    ),
    (
        TypeId::DataCompositionPeriodType,
        "DataCompositionPeriodType",
        "ТипПериодаКомпоновкиДанных",
    ),
    (
        TypeId::DataCompositionPeriodAdditionType,
        "DataCompositionPeriodAdditionType",
        "DataCompositionPeriodAdditionType",
    ),
    (TypeId::ColorDepth, "ColorDepth", "ColorDepth"),
    (
        TypeId::ToolTipRepresentation,
        "ToolTipRepresentation",
        "ToolTipRepresentation",
    ),
    (
        TypeId::ClientApplicationType,
        "ClientApplicationType",
        "ClientApplicationType",
    ),
    (
        TypeId::BackgroundJobState,
        "СостояниеФоновогоЗадания",
        "BackgroundJobState",
    ),
    (TypeId::MessageStatus, "СтатусСообщения", "MessageStatus"),
    (TypeId::ErrorCategory, "КатегорияОшибки", "ErrorCategory"),
    (TypeId::DateFractions, "DateFractions", "DateFractions"),
    (TypeId::AllowedLength, "AllowedLength", "AllowedLength"),
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
    // Нативное представление нового типа английское и в русском интерфейсе.
    (
        TypeId::ArchiveFileRestorePathsMode,
        "ArchiveFileRestoreFilePathsMode",
        "РежимВосстановленияПутейФайлаАрхива",
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
        TypeId::FormattedDocumentFileType,
        "ТипФайлаФорматированногоДокумента",
        "FormattedDocumentFileType",
    ),
    (
        TypeId::ByteOrderMarkUse,
        "ByteOrderMarkUse",
        "ByteOrderMarkUse",
    ),
];

/// Типы, чьё ПРЕДСТАВЛЕНИЕ не совпадает с именем, по которому тип ищется.
///
/// Представления сняты в закреплённом окружении замеров (русский интерфейс и
/// вариант языка): `tests/conformance/measure/1c/measure-presentations-ru.platform.txt`.
/// Строки без пробы в ней оставлены прежними измерениями.
const DISPLAY: &[(TypeId, &str)] = &[
    // Измерен в `measure-extension-purpose-presentations`.
    (TypeId::CompatibilityMode, "Режим совместимости"),
    // Измерены в `measure-common-attribute-property-enums`.
    (
        TypeId::CommonAttributeDataSeparation,
        "Разделение данных общего реквизита",
    ),
    (
        TypeId::CommonAttributeAutoUse,
        "Автоматическое использование общего реквизита",
    ),
    (TypeId::CommonAttributeUse, "Использование общего реквизита"),
    // Двухстрочное представление измерено в `measure-extension-purpose-presentations`.
    (
        TypeId::ConfigurationExtensionPurpose,
        "Назначение расширения конфигурации\nНазначение расширения конфигурации",
    ),
    (
        TypeId::DataCompositionSortDirection,
        "НаправлениеСортировкиКомпоновкиДанных",
    ),
    (TypeId::ColorDepth, "ГлубинаЦвета"),
    (TypeId::ToolTipRepresentation, "ОтображениеПодсказки"),
    (TypeId::ClientApplicationType, "ТипКлиентскогоПриложения"),
    (
        TypeId::DataCompositionGroupType,
        "ТипГруппировкиКомпоновкиДанных",
    ),
    (TypeId::FormDecorationType, "ВидДекорацииФормы"),
    (TypeId::RadioButtonType, "ВидПереключателя"),
    (
        TypeId::ChildFormItemsGroup,
        "ГруппировкаПодчиненныхЭлементовФормы",
    ),
    (TypeId::ChoiceHistoryOnInput, "ИсторияВыбораПриВводе"),
    (
        TypeId::ChoiceButtonRepresentation,
        "ОтображениеКнопкиВыбора",
    ),
    (
        TypeId::UsualGroupControlRepresentation,
        "ОтображениеУправленияОбычнойГруппы",
    ),
    (TypeId::UsualGroupBehavior, "ПоведениеОбычнойГруппы"),
    (
        TypeId::FormItemTitleLocation,
        "ПоложениеЗаголовкаЭлементаФормы",
    ),
    (
        TypeId::ButtonLocationInCommandBar,
        "ПоложениеКнопкиВКоманднойПанели",
    ),
    (
        TypeId::DataCompositionResourcesPlacement,
        "РасположениеРесурсовКомпоновкиДанных",
    ),
    (
        TypeId::DataCompositionPeriodType,
        "ТипПериодаКомпоновкиДанных",
    ),
    (
        TypeId::DataCompositionPeriodAdditionType,
        "ТипДополненияПериодаКомпоновкиДанных",
    ),
    (
        TypeId::DataCompositionGroupFieldsPlacement,
        "РасположениеПолейГруппировкиКомпоновкиДанных",
    ),
    (
        TypeId::DataCompositionAttributesPlacement,
        "РасположениеРеквизитовКомпоновкиДанных",
    ),
    (TypeId::ValueTable, "Таблица значений"),
    (
        TypeId::ValueTableColumns,
        "Коллекция колонок таблицы значений",
    ),
    (TypeId::ValueTableColumn, "Колонка таблицы значений"),
    (TypeId::ValueTableRow, "Строка таблицы значений"),
    (TypeId::TypeDescription, "Описание типов"),
    (TypeId::KeyAndValue, "Ключ и значение"),
    (TypeId::StringEncodingMethod, "СпособКодированияСтроки"),
    (TypeId::SortDirection, "НаправлениеСортировки"),
    (TypeId::UseOutput, "ИспользованиеВывода"),
    (TypeId::FormPagesRepresentation, "ОтображениеСтраницФормы"),
    (
        TypeId::FormButtonPictureLocation,
        "ПоложениеКартинкиКнопкиФормы",
    ),
    (
        TypeId::SpreadsheetDocumentDrawingLineType,
        "ТипЛинииРисункаТабличногоДокумента",
    ),
    (TypeId::CheckBoxType, "ВидФлажка"),
    (
        TypeId::DeliverableNotificationSubscriberType,
        "ТипПодписчикаДоставляемыхУведомлений",
    ),
    (TypeId::SoundAlert, "ЗвуковоеОповещение"),
    (TypeId::ThroughAlign, "СквозноеВыравнивание"),
    (TypeId::ButtonShape, "ФигураКнопки"),
    (
        TypeId::DataCompositionFilterItemsGroupType,
        "ТипГруппыЭлементовОтбораКомпоновкиДанных",
    ),
    (TypeId::Key, "Клавиша"),
    (TypeId::ControlBorderType, "ТипРамкиЭлементаУправления"),
    (TypeId::ColorType, "ВидЦвета"),
    (
        TypeId::SpreadsheetDocumentCellLineType,
        "ТипЛинииЯчейкиТабличногоДокумента",
    ),
    (
        TypeId::CryptoCertificateStoreType,
        "ТипХранилищаСертификатовКриптографии",
    ),
    (
        TypeId::InternetMailTextProcessing,
        "ОбработкаТекстаИнтернетПочтовогоСообщения",
    ),
    (TypeId::DuplexPrintingType, "ТипДвустороннейПечати"),
    (TypeId::FileDragMode, "СпособПеретаскиванияФайлов"),
    (TypeId::ButtonGroupRepresentation, "ОтображениеГруппыКнопок"),
    (TypeId::StandardPeriodVariant, "ВариантСтандартногоПериода"),
    (
        TypeId::TableHeightControlVariant,
        "ВариантУправленияВысотойТаблицы",
    ),
    (TypeId::FullTextSearchVersion, "ВерсияПолнотекстовогоПоиска"),
    (TypeId::VerticalFormScroll, "ВертикальнаяПрокруткаФормы"),
    (TypeId::VerticalAlign, "ВертикальноеПоложение"),
    (TypeId::AccumulationRecordType, "ВидДвиженияНакопления"),
    (TypeId::HorizontalAlign, "ГоризонтальноеПоложение"),
    (TypeId::ColumnsGroup, "ГруппировкаКолонок"),
    (TypeId::AllowedMessageNo, "ДопустимыйНомерСообщения"),
    (TypeId::FoldersAndItemsUse, "ИспользованиеГруппИЭлементов"),
    (
        TypeId::DataCompositionParameterUse,
        "ИспользованиеПараметраКомпоновкиДанных",
    ),
    (TypeId::ScrollBarUse, "ИспользованиеПолосыПрокрутки"),
    (TypeId::CurrentRowUse, "ИспользованиеТекущейСтроки"),
    (TypeId::InitialTreeView, "НачальноеОтображениеДерева"),
    (TypeId::UpdateOnDataChange, "ОбновлениеПриИзмененииДанных"),
    (TypeId::EditTextUpdate, "ОбновлениеТекстаРедактирования"),
    (
        TypeId::WarningOnEditRepresentation,
        "ОтображениеПредупрежденияПриРедактировании",
    ),
    (TypeId::SearchInTableOnInput, "ПоискВТаблицеПриВводе"),
    (
        TypeId::FormCommandBarLabelLocation,
        "ПоложениеКоманднойПанелиФормы",
    ),
    (
        TypeId::FormItemCommandBarLabelLocation,
        "ПоложениеКоманднойПанелиЭлементаФормы",
    ),
    (
        TypeId::GaugeChartValuesScaleLabelsLocation,
        "ПоложениеПодписейШкалыЗначенийИзмерительнойДиаграммы",
    ),
    (TypeId::ViewStatusLocation, "ПоложениеСостоянияПросмотра"),
    (TypeId::SearchStringLocation, "ПоложениеСтрокиПоиска"),
    (TypeId::SearchControlLocation, "ПоложениеУправленияПоиском"),
    (TypeId::AutoTimeMode, "РежимАвтоВремя"),
    (TypeId::TableRowInputMode, "РежимВводаСтрокТаблицы"),
    (
        TypeId::CryptoCertificateIncludeMode,
        "РежимВключенияСертификатовКриптографии",
    ),
    (TypeId::TableRowSelectionMode, "РежимВыделенияСтрокиТаблицы"),
    (TypeId::TableSelectionMode, "РежимВыделенияТаблицы"),
    (TypeId::ReplacementMode, "РежимЗамещения"),
    (TypeId::SizeChangeMode, "РежимИзмененияРазмера"),
    (TypeId::FormWindowOpeningMode, "РежимОткрытияОкнаФормы"),
    (
        TypeId::DataCompositionSettingsViewMode,
        "РежимОтображенияНастроекКомпоновкиДанных",
    ),
    (
        TypeId::ReportResultViewMode,
        "РежимОтображенияРезультатаОтчета",
    ),
    (TypeId::FullTextSearchMode, "РежимПолнотекстовогоПоиска"),
    (
        TypeId::CryptoCertificateCheckMode,
        "РежимПроверкиСертификатаКриптографии",
    ),
    (TypeId::ColumnEditMode, "РежимРедактированияКолонки"),
    (
        TypeId::InfoBaseUserPasswordRecoveryMethod,
        "СпособВосстановленияПароляПользователяИнформационнойБазы",
    ),
    (
        TypeId::SpreadsheetDocumentAreaFillType,
        "ТипЗаполненияОбластиТабличногоДокумента",
    ),
    (TypeId::QuerySchemaUnionType, "ТипОбъединенияСхемыЗапроса"),
    (TypeId::CryptoSignatureType, "ТипПодписиКриптографии"),
    (
        TypeId::SpreadsheetDocumentTextPlacementType,
        "ТипРазмещенияТекстаТабличногоДокумента",
    ),
    (TypeId::QuerySchemaJoinType, "ТипСоединенияСхемыЗапроса"),
    (TypeId::ReportFormType, "ТипФормыОтчета"),
    (TypeId::FixingInTable, "ФиксацияВТаблице"),
    (
        TypeId::DataCompositionConditionalAppearanceUse,
        "ИспользованиеУсловногоОформленияКомпоновкиДанных",
    ),
    (
        TypeId::DataCompositionFieldPlacement,
        "РасположениеПоляКомпоновкиДанных",
    ),
    (
        TypeId::DataCompositionAccountingBalanceType,
        "ТипБухгалтерскогоОстаткаКомпоновкиДанных",
    ),
    (
        TypeId::DataCompositionBalanceType,
        "ТипОстаткаКомпоновкиДанных",
    ),
    (TypeId::DateFractions, "ЧастиДаты"),
    (TypeId::AllowedLength, "ДопустимаяДлина"),
    (TypeId::HashFunction, "ХешФункция"),
    (
        TypeId::ArchiveFileRestorePathsMode,
        "РежимВосстановленияПутейФайлаАрхива",
    ),
    (TypeId::ByteOrderMarkUse, "ИспользованиеByteOrderMark"),
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
/// Владеющая ссылка на тип занимает тот же размер, что статическая:
/// `BslValue::Type` не должен раздувать значение (см. инвариант про размер).
#[derive(Debug, Clone)]
pub enum TypeRef {
    Native(TypeId),
    /// Тип объекта компонента. Дескрипторы — статики, но равенство здесь
    /// ПО СОДЕРЖИМОМУ (пакет и имя): так тождество не зависит от того,
    /// собрал ли линкер две копии одного дескриптора.
    Object(&'static crate::TypeDescriptor),
    /// Тип значения с описанием, принадлежащим хосту и движку.
    Value(std::sync::Arc<crate::ValueTypeDescriptor>),
}

impl TypeRef {
    /// Нативный идентификатор, если тип — из закрытого реестра ядра.
    pub fn native(&self) -> Option<TypeId> {
        match self {
            TypeRef::Native(id) => Some(*id),
            TypeRef::Object(_) | TypeRef::Value(_) => None,
        }
    }

    /// Русское имя типа — то, что печатает `Строка(ТипЗнч(...))`.
    pub fn name(&self) -> String {
        match self {
            TypeRef::Native(id) => id.name().to_string(),
            TypeRef::Object(descriptor) => descriptor.type_display.to_string(),
            TypeRef::Value(descriptor) => descriptor.display().to_string(),
        }
    }

    /// Длина печатного имени в байтах без его материализации: описание
    /// неизменно, поэтому длина известна заранее. Нужна
    /// тем, кто списывает бюджет памяти до аллокации.
    #[must_use]
    pub fn display_len(&self) -> usize {
        match self {
            TypeRef::Native(id) => id.name().len(),
            TypeRef::Object(descriptor) => descriptor.type_display.len(),
            TypeRef::Value(descriptor) => descriptor.display().len(),
        }
    }
}

impl PartialEq for TypeRef {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (TypeRef::Native(a), TypeRef::Native(b)) => a == b,
            (TypeRef::Object(a), TypeRef::Object(b)) => a.package == b.package && a.name == b.name,
            (TypeRef::Value(a), TypeRef::Value(b)) => {
                a.package() == b.package() && a.identity() == b.identity()
            }
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
            TypeRef::Value(descriptor) => {
                2u8.hash(state);
                descriptor.package().hash(state);
                descriptor.identity().hash(state);
            }
        }
    }
}

impl std::fmt::Display for TypeRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TypeRef::Native(id) => write!(f, "{id}"),
            TypeRef::Object(descriptor) => write!(f, "{}", descriptor.type_display),
            TypeRef::Value(descriptor) => f.write_str(descriptor.display()),
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
        static INDEX: std::sync::OnceLock<std::collections::HashMap<String, TypeId>> =
            std::sync::OnceLock::new();
        // Таблицы неизменяемы: нормализация всех их написаний на каждый
        // запрос превращала проверку типов большой конфигурации в основной
        // расход CPU. Первый встретившийся ключ сохраняет прежний приоритет
        // `NAMES`, а дополнительные идентификаторы остаются запасным путём.
        INDEX
            .get_or_init(|| {
                let mut index = std::collections::HashMap::new();
                for (id, ru, en) in NAMES {
                    index.entry(squash(ru)).or_insert(*id);
                    index.entry(squash(en)).or_insert(*id);
                }
                for (id, name) in IDENTIFIERS {
                    index.entry(squash(name)).or_insert(*id);
                }
                index
            })
            .get(&squash(name))
            .copied()
    }
}

impl std::fmt::Display for TypeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

#[cfg(test)]
#[path = "type_lookup_tests.rs"]
mod lookup_tests;

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
