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

    // --- Режим открытия форм приложения ------------------------------
    ApplicationFormsOpenningModeTabs,
    ApplicationFormsOpenningModeSingleWindows,

    // --- Вариант интерфейса клиентского приложения -------------------
    ClientApplicationInterfaceVariantVersion8_2,
    ClientApplicationInterfaceVariantTaxi,

    // --- Отображение панели разделов ---------------------------------
    SectionsPanelRepresentationPicture,
    SectionsPanelRepresentationPictureAndText,
    SectionsPanelRepresentationPictureOnTopAndText,
    SectionsPanelRepresentationPictureOnLeftAndText,
    SectionsPanelRepresentationText,

    // --- Расширения конфигурации --------------------------------------
    ConfigurationExtensionsSourceDatabase,
    ConfigurationExtensionsSourceSessionApplied,
    ConfigurationExtensionsSourceSessionDisabled,
    ConfigurationExtensionScopeInfoBase,
    ConfigurationExtensionScopeDataSeparation,
    ConfigurationExtensionPurposePatch,
    ConfigurationExtensionPurposeCustomization,
    ConfigurationExtensionPurposeAddOn,
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
    EventLogEntryTransactionStatusCommitted,
    EventLogEntryTransactionStatusUnfinished,
    EventLogEntryTransactionStatusNotApplicable,
    EventLogEntryTransactionStatusRolledBack,

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

    // --- РежимЗаписиДокумента ---------------------------------------
    DocumentWriteModeWrite,
    DocumentWriteModePosting,
    DocumentWriteModeUndoPosting,

    // --- ОбходРезультатаЗапроса --------------------------------------
    QueryResultIterationLinear,
    QueryResultIterationByGroups,
    QueryResultIterationByGroupsWithHierarchy,

    // --- ТипСвязиНаборовДанныхКомпоновкиДанных ------------------------
    DataCompositionDataSetsLinkTypeOuter,
    DataCompositionDataSetsLinkTypeInner,

    // --- ТипСоединенияЗначенийПоСериямДиаграммы -----------------------
    ChartValuesBySeriesConnectionTypeNone,
    ChartValuesBySeriesConnectionTypeEdgesConnection,

    // --- РасположениеЛегендыДиаграммыКомпоновкиДанных -----------------
    DataCompositionChartLegendPlacementNone,
    DataCompositionChartLegendPlacementLeft,
    DataCompositionChartLegendPlacementRight,
    DataCompositionChartLegendPlacementTop,
    DataCompositionChartLegendPlacementBottom,

    // --- ТипЗаписиЗапроса ------------------------------------------
    QueryRecordTypeDetailRecord,
    QueryRecordTypeGroupTotal,
    QueryRecordTypeTotalByHierarchy,
    QueryRecordTypeOverall,

    // --- РежимБлокировкиДанных ---------------------------------------
    DataLockModeShared,
    DataLockModeExclusive,

    // --- РежимОкругления ---------------------------------------------
    RoundModeRound15as10,
    RoundModeRound15as20,

    // --- ВидЧисловогоЗначения ---------------------------------------
    NumericValueTypeCardinal,
    NumericValueTypeOrdinal,

    // --- ВариантОтображенияСообщенияОбОшибке -------------------------
    ErrorMessageDisplayVariantAuto,
    ErrorMessageDisplayVariantErrorMessageForUser,
    ErrorMessageDisplayVariantBriefErrorDescription,
    ErrorMessageDisplayVariantDetailErrorDescription,

    // --- РежимИспользованияПараметраКоманды --------------------------
    CommandParameterUseModeSingle,
    CommandParameterUseModeMultiple,

    // --- СпособПоискаСтрокиПриВводеПоСтроке --------------------------
    SearchStringModeOnInputByStringBegin,
    SearchStringModeOnInputByStringAnyPart,

    // --- ТипФайлаПакетаОтображаемыхДокументов ------------------------
    RepresentableDocumentBatchFileTypeDocx,
    RepresentableDocumentBatchFileTypeHtml4,
    RepresentableDocumentBatchFileTypeHtml5,
    RepresentableDocumentBatchFileTypeOds,
    RepresentableDocumentBatchFileTypePdf,
    RepresentableDocumentBatchFileTypeTxt,
    RepresentableDocumentBatchFileTypeXls,
    RepresentableDocumentBatchFileTypeXlsx,

    // --- ВидШрифта --------------------------------------------------
    FontTypeWindowsFont,
    FontTypeAbsolute,
    FontTypeAutoFont,
    FontTypeStyleItem,

    // --- ВидКартинки ------------------------------------------------
    PictureTypeAbsolute,
    PictureTypeFromLib,
    PictureTypeEmpty,

    // --- ИспользованиеМетаданныхПолнотекстовогоПоиска --------------
    FullTextSearchMetadataUseUse,
    FullTextSearchMetadataUseDontUse,

    // --- РежимИзмененияСвязанногоЗначения ---------------------------
    LinkedValueChangeModeDontChange,
    LinkedValueChangeModeClear,

    // --- СпособКодированияИнтернетПочтовогоВложения ----------------
    InternetMailAttachmentEncodingModeMime,
    InternetMailAttachmentEncodingModeUuEncode,

    // --- АвтоматическоеСохранениеДанныхФормыВНастройках -------------
    AutoSaveFormDataInSettingsUse,
    AutoSaveFormDataInSettingsDontUse,

    // --- ТипФайлаДокументаPDF ----------------------------------------
    PDFDocumentFileTypePdf,
    PDFDocumentFileTypePdfA1,
    PDFDocumentFileTypePdfA2,
    PDFDocumentFileTypePdfA3,

    // --- ДопустимыйЗнак ---------------------------------------------
    AllowedSignAny,
    AllowedSignNonnegative,

    // --- АвтоРегистрацияИзменений ----------------------------------
    AutoChangeRecordDeny,
    AutoChangeRecordAllow,

    // --- ОтображениеКнопки -----------------------------------------
    ButtonRepresentationAuto,
    ButtonRepresentationPicture,
    ButtonRepresentationPictureAndText,
    ButtonRepresentationText,

    // --- РезультатПроверкиСоответствияПароляПолитике -----------------
    PasswordPolicyComplianceCheckResultMinLength,
    PasswordPolicyComplianceCheckResultReuseLimit,
    PasswordPolicyComplianceCheckResultCompromiseCheck,
    PasswordPolicyComplianceCheckResultComplexity,

    // --- ОтображениеТаблицы ------------------------------------------
    TableRepresentationList,
    TableRepresentationHierarchicalList,
    TableRepresentationTree,

    // --- ВидПоляФормы -------------------------------------------------
    FormFieldTypeHtmlDocumentField,
    FormFieldTypePdfDocumentField,
    FormFieldTypeInputField,
    FormFieldTypeGeographicalSchemaField,
    FormFieldTypeGraphicalSchemaField,
    FormFieldTypeDendrogramField,
    FormFieldTypeChartField,
    FormFieldTypeGanttChartField,
    FormFieldTypeProgressBarField,
    FormFieldTypeCalendarField,
    FormFieldTypePictureField,
    FormFieldTypeLabelField,
    FormFieldTypeRadioButtonField,
    FormFieldTypePeriodField,
    FormFieldTypePlannerField,
    FormFieldTypeTrackBarField,
    FormFieldTypeSpreadsheetDocumentField,
    FormFieldTypeTextDocumentField,
    FormFieldTypeCheckBoxField,
    FormFieldTypeFormattedDocumentField,

    // --- ВидГруппыФормы -----------------------------------------------
    FormGroupTypeButtonGroup,
    FormGroupTypeColumnGroup,
    FormGroupTypeCommandBar,
    FormGroupTypeContextMenu,
    FormGroupTypeUsualGroup,
    FormGroupTypePopup,
    FormGroupTypePage,
    FormGroupTypePages,

    // --- ГоризонтальноеПоложениеЭлемента ------------------------------
    ItemHorizontalLocationAuto,
    ItemHorizontalLocationLeft,
    ItemHorizontalLocationRight,
    ItemHorizontalLocationCenter,

    // --- ВертикальноеПоложениеЭлемента --------------------------------
    ItemVerticalAlignAuto,
    ItemVerticalAlignTop,
    ItemVerticalAlignBottom,
    ItemVerticalAlignCenter,

    // --- ИнтервалМеждуЭлементамиФормы ---------------------------------
    FormItemSpacingAuto,
    FormItemSpacingDouble,
    FormItemSpacingNone,
    FormItemSpacingSingle,
    FormItemSpacingHalf,
    FormItemSpacingOneAndHalf,

    // --- ОтображениеФигурыКнопки --------------------------------------
    ButtonShapeRepresentationAuto,
    ButtonShapeRepresentationAlways,
    ButtonShapeRepresentationNone,
    ButtonShapeRepresentationWhenActive,

    // --- ОтображениеОбычнойГруппы -------------------------------------
    UsualGroupRepresentationNone,
    UsualGroupRepresentationNormalSeparation,
    UsualGroupRepresentationStrongSeparation,
    UsualGroupRepresentationWeakSeparation,

    // --- ВажностьПриОтображении ---------------------------------------
    DisplayImportanceAuto,
    DisplayImportanceHigh,
    DisplayImportanceLow,
    DisplayImportanceUsual,
    DisplayImportanceVeryHigh,
    DisplayImportanceVeryLow,

    // --- ФорматКартинки ------------------------------------------------
    PictureFormatBmp,
    PictureFormatEmf,
    PictureFormatGif,
    PictureFormatIcon,
    PictureFormatJpeg,
    PictureFormatPng,
    PictureFormatSvg,
    PictureFormatTiff,
    PictureFormatWmf,
    PictureFormatUnknown,

    // --- ТипТекстаПочтовогоСообщения ----------------------------------
    InternetMailTextTypeHtml,
    InternetMailTextTypeCustomText,
    InternetMailTextTypePlainText,
    InternetMailTextTypeRichText,

    // --- СпособКодированияНеASCIIСимволовИнтернетПочтовогоСообщения ----
    InternetMailNonAsciiMime,
    InternetMailNonAsciiQuotedPrintable,
    InternetMailNonAsciiNone,

    // --- ВажностьИнтернетПочтовогоСообщения ----------------------------
    InternetMailMessageImportanceHigh,
    InternetMailMessageImportanceHighest,
    InternetMailMessageImportanceLowest,
    InternetMailMessageImportanceLow,
    InternetMailMessageImportanceNormal,

    // --- ДополнительныйРежимОтображения ------------------------------
    AdditionalShowModeDontUse,
    AdditionalShowModeIrrelevance,

    // --- РежимПроверкиОтзываTLSСертификатаСервера -------------------
    ServerTlsCertificateRevocationCheckModeAuto,
    ServerTlsCertificateRevocationCheckModeDontCheck,
    ServerTlsCertificateRevocationCheckModeSoftFail,
    ServerTlsCertificateRevocationCheckModeStrict,

    // --- ТипПримененияОтбораКомпоновкиДанных ---------------------
    DataCompositionFilterApplicationTypeItems,
    DataCompositionFilterApplicationTypeHierarchy,
    DataCompositionFilterApplicationTypeHierarchyOnly,

    // --- СостояниеЭлементаНастройкиКомпоновкиДанных ----------------
    DataCompositionSettingsItemStateEnabled,
    DataCompositionSettingsItemStateDisabled,
    DataCompositionSettingsItemStateDeletedByUser,

    // --- РежимОтображенияЭлементаНастройкиКомпоновкиДанных ---------
    DataCompositionSettingsItemViewModeAuto,
    DataCompositionSettingsItemViewModeNormal,
    DataCompositionSettingsItemViewModeQuickAccess,
    DataCompositionSettingsItemViewModeInaccessible,

    // --- СпособВосстановленияНастроекКомпоновкиДанных ---------------
    DataCompositionSettingsRefreshMethodFull,
    DataCompositionSettingsRefreshMethodCheckAvailability,

    // --- НаправлениеСортировкиКомпоновкиДанных ----------------------
    DataCompositionSortDirectionAsc,
    DataCompositionSortDirectionDesc,

    // --- ТипГруппировкиКомпоновкиДанных ------------------------------
    DataCompositionGroupTypeHierarchy,
    DataCompositionGroupTypeHierarchyOnly,
    DataCompositionGroupTypeItems,

    // --- РасположениеГруппировкиКомпоновкиДанных -------------------
    DataCompositionGroupPlacementEnd,
    DataCompositionGroupPlacementBegin,
    DataCompositionGroupPlacementBeginAndEnd,
    DataCompositionGroupPlacementNone,

    // --- РасположениеПолейГруппировкиКомпоновкиДанных --------------
    DataCompositionGroupFieldsPlacementTogether,
    DataCompositionGroupFieldsPlacementSeparately,
    DataCompositionGroupFieldsPlacementSeparatelyAndInTotalsOnly,

    // --- РасположениеРеквизитовКомпоновкиДанных ---------------------
    DataCompositionAttributesPlacementTogether,
    DataCompositionAttributesPlacementWithOwnerField,
    DataCompositionAttributesPlacementSpecialPosition,
    DataCompositionAttributesPlacementSeparately,

    // --- ВидДекорацииФормы ------------------------------------------------
    FormDecorationTypePicture,
    FormDecorationTypeLabel,

    // --- ВидПереключателя ------------------------------------------------
    RadioButtonTypeAuto,
    RadioButtonTypeRadioButton,
    RadioButtonTypeTumbler,

    // --- ГруппировкаПодчиненныхЭлементовФормы ------------------------------------------------
    ChildFormItemsGroupVertical,
    ChildFormItemsGroupHorizontal,
    ChildFormItemsGroupAlwaysHorizontal,
    ChildFormItemsGroupHorizontalIfPossible,

    // --- ИсторияВыбораПриВводе ------------------------------------------------
    ChoiceHistoryOnInputAuto,
    ChoiceHistoryOnInputDontUse,

    // --- ОтображениеКнопкиВыбора ------------------------------------------------
    ChoiceButtonRepresentationAuto,
    ChoiceButtonRepresentationShowInDropList,
    ChoiceButtonRepresentationShowInDropListAndInInputField,
    ChoiceButtonRepresentationShowInInputField,

    // --- ОтображениеУправленияОбычнойГруппы ------------------------------------------------
    UsualGroupControlRepresentationTitleHyperlink,
    UsualGroupControlRepresentationPicture,

    // --- ПоведениеОбычнойГруппы ------------------------------------------------
    UsualGroupBehaviorAuto,
    UsualGroupBehaviorPopUp,
    UsualGroupBehaviorUsual,
    UsualGroupBehaviorCollapsible,

    // --- ПоложениеЗаголовкаЭлементаФормы ------------------------------------------------
    FormItemTitleLocationAuto,
    FormItemTitleLocationTop,
    FormItemTitleLocationLeft,
    FormItemTitleLocationNone,
    FormItemTitleLocationBottom,
    FormItemTitleLocationRight,

    // --- ПоложениеКнопкиВКоманднойПанели ------------------------------------------------
    ButtonLocationInCommandBarAuto,
    ButtonLocationInCommandBarInAdditionalSubmenu,
    ButtonLocationInCommandBarInCommandBar,
    ButtonLocationInCommandBarInCommandBarAndInAdditionalSubmenu,

    // --- РасположениеРесурсовКомпоновкиДанных ------------------------------------------------
    DataCompositionResourcesPlacementVertically,
    DataCompositionResourcesPlacementHorizontally,

    // Представления подтверждены нативным контролем 8.3.27.2342.
    DataCompositionConditionalAppearanceUseUse,
    DataCompositionConditionalAppearanceUseDontUse,
    DataCompositionFieldPlacementAuto,
    DataCompositionFieldPlacementVertically,
    DataCompositionFieldPlacementTogether,
    DataCompositionFieldPlacementHorizontally,
    DataCompositionFieldPlacementSpecialColumn,
    DataCompositionAccountingBalanceTypeDebit,
    DataCompositionAccountingBalanceTypeCredit,
    DataCompositionAccountingBalanceTypeNone,
    DataCompositionBalanceTypeOpeningBalance,
    DataCompositionBalanceTypeClosingBalance,
    DataCompositionBalanceTypeNone,
    DataCompositionPeriodTypeMain,
    DataCompositionPeriodTypeAdditional,

    // --- ТипДополненияПериодаКомпоновкиДанных ------------------------------------------------
    DataCompositionPeriodAdditionTypeNone,
    DataCompositionPeriodAdditionTypeYear,
    DataCompositionPeriodAdditionTypeYearSinceBeginOfPeriod,
    DataCompositionPeriodAdditionTypeYearSinceBeginOfPeriod445,
    DataCompositionPeriodAdditionTypeTenDays,
    DataCompositionPeriodAdditionTypeDay,
    DataCompositionPeriodAdditionTypeDaySinceBeginOfPeriod,
    DataCompositionPeriodAdditionTypeQuarter,
    DataCompositionPeriodAdditionTypeQuarterSinceBeginOfPeriod,
    DataCompositionPeriodAdditionTypeQuarterSinceBeginOfPeriod445,
    DataCompositionPeriodAdditionTypeMonth,
    DataCompositionPeriodAdditionTypeMonthSinceBeginOfPeriod,
    DataCompositionPeriodAdditionTypeMonthSinceBeginOfPeriod445,
    DataCompositionPeriodAdditionTypeMinute,
    DataCompositionPeriodAdditionTypeMinuteSinceBeginOfPeriod,
    DataCompositionPeriodAdditionTypeWeek,
    DataCompositionPeriodAdditionTypeWeekSinceBeginOfPeriod,
    DataCompositionPeriodAdditionTypeHalfYear,
    DataCompositionPeriodAdditionTypeHalfYearSinceBeginOfPeriod,
    DataCompositionPeriodAdditionTypeHalfYearSinceBeginOfPeriod445,
    DataCompositionPeriodAdditionTypeSecond,
    DataCompositionPeriodAdditionTypeHour,
    DataCompositionPeriodAdditionTypeHourSinceBeginOfPeriod,

    // --- ГлубинаЦвета ------------------------------------------------
    ColorDepthBitPerPixel1,
    ColorDepthBitPerPixel24,
    ColorDepthBitPerPixel32,
    ColorDepthBitPerPixel4,
    ColorDepthBitPerPixel8,

    // --- ОтображениеПодсказки ------------------------------------------------
    ToolTipRepresentationAuto,
    ToolTipRepresentationBalloon,
    ToolTipRepresentationButton,
    ToolTipRepresentationNone,
    ToolTipRepresentationShowAuto,
    ToolTipRepresentationShowTop,
    ToolTipRepresentationShowLeft,
    ToolTipRepresentationShowBottom,
    ToolTipRepresentationShowRight,

    // --- ТипКлиентскогоПриложения ------------------------------------------------
    ClientApplicationTypeWebClient,
    ClientApplicationTypeExternalConnection,
    ClientApplicationTypeMobileAppClient,
    ClientApplicationTypeMobileClient,
    ClientApplicationTypeThickClient,
    ClientApplicationTypeThinClient,

    // --- ТипРазмещенияТекстаКомпоновкиДанных -----------------------
    DataCompositionTextPlacementTypeOverflow,
    DataCompositionTextPlacementTypeBlock,
    DataCompositionTextPlacementTypeCut,
    DataCompositionTextPlacementTypeWrap,

    // --- ТипВыводаТекстаКомпоновкиДанных ----------------------------
    DataCompositionTextOutputTypeAuto,
    DataCompositionTextOutputTypeOutput,
    DataCompositionTextOutputTypeDontOutput,

    // --- РежимМасштабированияПросмотра -------------------------------
    ViewScalingModeAuto,
    ViewScalingModeLarge,
    ViewScalingModeNormal,

    // --- РасположениеИтоговКомпоновкиДанных --------------------------
    DataCompositionTotalPlacementAuto,
    DataCompositionTotalPlacementEnd,
    DataCompositionTotalPlacementBegin,
    DataCompositionTotalPlacementBeginAndEnd,
    DataCompositionTotalPlacementNone,

    // --- РазмерКартинки ----------------------------------------------
    PictureSizeAutoSize,
    PictureSizeAutoSizeIgnoreScale,
    PictureSizeByFontSize,
    PictureSizeProportionally,
    PictureSizeStretch,
    PictureSizeRealSize,
    PictureSizeRealSizeIgnoreScale,
    PictureSizeTile,

    // --- ТипСмещенияТабличногоДокумента ------------------------------
    SpreadsheetDocumentShiftTypeWithoutShift,
    SpreadsheetDocumentShiftTypeVertical,
    SpreadsheetDocumentShiftTypeHorizontal,

    // --- ТипДиаграммы -------------------------------------------------
    ChartTypeStock,
    ChartTypeOpenHighLowClose,
    ChartTypeConcaveSurface,
    ChartTypeWaterfall,
    ChartTypeFunnel,
    ChartTypeNormalizedFunnel,
    ChartTypeNormalizedFunnel3D,
    ChartTypeFunnel3D,
    ChartTypeConvexSurface,
    ChartTypeColumn,
    ChartTypeBar,
    ChartTypeBar3D,
    ChartTypeNormalizedColumn,
    ChartTypeNormalizedBar,
    ChartTypeNormalizedBar3D,
    ChartTypeNormalizedColumn3D,
    ChartTypeColumn3D,
    ChartTypeStackedColumn,
    ChartTypeStackedBar,
    ChartTypeStackedBar3D,
    ChartTypeStackedColumn3D,
    ChartTypeLine,
    ChartTypeStep,
    ChartTypeStackedLine,
    ChartTypeArea,
    ChartTypeStackedArea,
    ChartTypeNormalizedArea,
    ChartTypeShadedSurface,
    ChartTypeGauge,
    ChartTypeBarGraph,
    ChartTypeTapeGraph,
    ChartTypeCeilGraph,
    ChartTypePyramidGraph,
    ChartTypeWireframeSurface,
    ChartTypeDonut,
    ChartTypeDonut3D,
    ChartTypePie,
    ChartTypePie3D,
    ChartTypeSurface,
    ChartTypeBubble,
    ChartTypeRadarLine,
    ChartTypeRadarStackedLine,
    ChartTypeRadarArea,
    ChartTypeRadarStackedArea,
    ChartTypeRadarNormalizedArea,
    ChartTypeHoneycomb,
    ChartTypeScatter,

    // --- ОриентацияПодписейДиаграммы ---------------------------------
    ChartLabelsOrientationAuto,
    ChartLabelsOrientationVertical,
    ChartLabelsOrientationHorizontal,
    ChartLabelsOrientationCustomAngle,

    // --- ВидГраницы --------------------------------------------------
    BoundaryTypeIncluding,
    BoundaryTypeExcluding,

    // --- МаксимумСерий ----------------------------------------------
    MaxSeriesNotDefined,
    MaxSeriesLimited,
    MaxSeriesPercent,

    // --- ВидПодписейКДиаграмме ---------------------------------------
    ChartLabelTypeValue,
    ChartLabelTypeValuePercent,
    ChartLabelTypeValueSize,
    ChartLabelTypeNone,
    ChartLabelTypePercent,
    ChartLabelTypeSeries,
    ChartLabelTypeSeriesValue,
    ChartLabelTypeSeriesValuePercent,
    ChartLabelTypeSeriesValueSize,
    ChartLabelTypeSeriesPercent,
    ChartLabelTypeSeriesSize,
    ChartLabelTypeSeriesPoint,
    ChartLabelTypeSeriesPointValue,
    ChartLabelTypeSeriesPointValuePercent,
    ChartLabelTypeSeriesPointValueSize,
    ChartLabelTypeSeriesPointPercent,
    ChartLabelTypeSeriesPointSize,
    ChartLabelTypePoint,
    ChartLabelTypePointValue,
    ChartLabelTypePointValuePercent,
    ChartLabelTypePointValueSize,
    ChartLabelTypePointPercent,
    ChartLabelTypePointSize,

    // --- ТипЛинииДиаграммы ------------------------------------------
    ChartLineTypeNone,
    ChartLineTypeDashed,
    ChartLineTypeDashDotted,
    ChartLineTypeDashDottedDotted,
    ChartLineTypeSolid,
    ChartLineTypeDotted,

    // --- ВидКнопкиФормы ---------------------------------------------
    FormButtonTypeHyperlink,
    FormButtonTypeCommandBarHyperlink,
    FormButtonTypeCommandBarButton,
    FormButtonTypeUsualButton,

    // --- НазначениеТипаXML -------------------------------------------
    XmlTypeAssignmentImplicit,
    XmlTypeAssignmentExplicit,

    // --- ТипПлатформы ------------------------------------------------
    PlatformTypeAndroidArm,
    PlatformTypeAndroidArm64,
    PlatformTypeAndroidX86,
    PlatformTypeAndroidX86_64,
    PlatformTypeIosArm,
    PlatformTypeIosArm64,
    PlatformTypeLinuxArm64,
    PlatformTypeLinuxE2k,
    PlatformTypeLinuxX86,
    PlatformTypeLinuxX86_64,
    PlatformTypeMacOsX86,
    PlatformTypeMacOsX86_64,
    PlatformTypeWindowsX86,
    PlatformTypeWindowsX86_64,
    PlatformTypeWinRtArm,
    PlatformTypeWinRtX86,
    PlatformTypeWinRtX86_64,

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

    // --- ВидСравнения -------------------------------------------------
    ComparisonGreater,
    ComparisonGreaterOrEqual,
    ComparisonInHierarchy,
    ComparisonInList,
    ComparisonInListByHierarchy,
    ComparisonInterval,
    ComparisonIntervalIncludingBounds,
    ComparisonIntervalIncludingLowerBound,
    ComparisonIntervalIncludingUpperBound,
    ComparisonLess,
    ComparisonLessOrEqual,
    ComparisonNotInHierarchy,
    ComparisonNotInList,
    ComparisonNotInListByHierarchy,
    ComparisonNotEqual,
    ComparisonNotContains,
    ComparisonEqual,
    ComparisonContains,

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
    // На 8.3.27 измерено восемнадцать членов; `MXLX` среди них нет.
    // Наличие члена не обещает поддержку
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
    SpreadFileHtml,
    SpreadFileHtml3,
    SpreadFileHtml4,
    SpreadFileMxl7,
    SpreadFilePdfA1,
    SpreadFilePdfA2,
    SpreadFilePdfA3,
    SpreadFileXls95,
    SpreadFileXls97,
    /// Единственный поддержанный вид рисунка.
    DrawingGeographicalSchema,
    DrawingGroup,
    DrawingDendrogram,
    DrawingChart,
    DrawingGanttChart,
    DrawingPicture,
    DrawingObject,
    DrawingComment,
    DrawingLine,
    DrawingRectangle,
    DrawingPivotChart,
    DrawingText,
    DrawingEllipse,

    // Состав и представления измерены в `measure-gantt-interval-representation.bsl`.
    GanttChartIntervalRepresentationFlat,
    GanttChartIntervalRepresentationThreeDimensional,
    GanttChartIntervalRepresentationRhomb,
    GanttChartIntervalRepresentationGradient,

    // Представления измерены в `measure-gantt-vertical-stretch.bsl`.
    GanttChartVerticalStretchNone,
    GanttChartVerticalStretchStretchRows,
    GanttChartVerticalStretchStretchRowsAndData,

    // Состав и представления измерены в `measure-chart-legend-placement.bsl`.
    ChartLegendPlacementAuto,
    ChartLegendPlacementBottom,
    ChartLegendPlacementTop,
    ChartLegendPlacementLeft,
    ChartLegendPlacementRight,
    ChartLegendPlacementNone,
    ChartLegendPlacementUseCoordinates,

    // Состав и представления измерены в `measure-time-scale-unit-type.bsl`.
    TimeScaleUnitTypeSecond,
    TimeScaleUnitTypeMinute,
    TimeScaleUnitTypeHour,
    TimeScaleUnitTypeDay,
    TimeScaleUnitTypeWeek,
    TimeScaleUnitTypeMonth,
    TimeScaleUnitTypeQuarter,
    TimeScaleUnitTypeYear,

    // Представления измерены в `measure-time-scale-day-format.bsl`.
    TimeScaleDayFormatMonthDay,
    TimeScaleDayFormatWeekDay,
    TimeScaleDayFormatMonthDayWeekDay,
    TimeScaleDayFormatWeekDayMonthDay,

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
    SortDirectionDescending,

    // --- ИспользованиеВывода ------------------------------------------
    UseOutputAuto,
    UseOutputEnable,
    UseOutputDisable,

    // --- ОтображениеСтраницФормы --------------------------------------
    FormPagesRepresentationAuto,
    FormPagesRepresentationNone,
    FormPagesRepresentationSwipe,
    FormPagesRepresentationTabsOnTop,
    FormPagesRepresentationTabsOnBottom,
    FormPagesRepresentationTabsOnLeftHorizontal,
    FormPagesRepresentationTabsOnRightHorizontal,

    // --- ПоложениеКартинкиКнопкиФормы ---------------------------------
    FormButtonPictureLocationAuto,
    FormButtonPictureLocationLeft,
    FormButtonPictureLocationRight,

    // --- ТипЛинииРисункаТабличногоДокумента --------------------------
    SpreadsheetDocumentDrawingLineTypeNone,
    SpreadsheetDocumentDrawingLineTypeSolid,
    SpreadsheetDocumentDrawingLineTypeDashed,
    SpreadsheetDocumentDrawingLineTypeDotted,
    SpreadsheetDocumentDrawingLineTypeDashDotted,
    SpreadsheetDocumentDrawingLineTypeDashDottedDotted,

    // --- ВидФлажка ----------------------------------------------------
    CheckBoxTypeAuto,
    CheckBoxTypeCheckBox,
    CheckBoxTypeSwitch,
    CheckBoxTypeTumbler,

    // --- ТипПодписчикаДоставляемыхУведомлений -------------------------
    DeliverableNotificationSubscriberTypeGCM,
    DeliverableNotificationSubscriberTypeAPNS,
    DeliverableNotificationSubscriberTypeFCM,

    // --- ЗвуковоеОповещение ------------------------------------------
    SoundAlertNone,
    SoundAlertDefault,

    // --- СквозноеВыравнивание ----------------------------------------
    ThroughAlignAuto,
    ThroughAlignUse,
    ThroughAlignDontUse,

    // --- ФигураКнопки ------------------------------------------------
    ButtonShapeAuto,
    ButtonShapeUsual,
    ButtonShapeOval,

    // --- ТипГруппыЭлементовОтбораКомпоновкиДанных --------------------
    DataCompositionFilterItemsGroupTypeAndGroup,
    DataCompositionFilterItemsGroupTypeOrGroup,
    DataCompositionFilterItemsGroupTypeNotGroup,

    // --- Клавиша -----------------------------------------------------
    KeyDigit1,
    KeyDigit2,
    KeyDigit3,
    KeyDigit4,
    KeyDigit5,
    KeyDigit6,
    KeyDigit7,
    KeyDigit8,
    KeyDigit9,
    KeyNum0,
    KeyNum1,
    KeyNum2,
    KeyNum3,
    KeyNum4,
    KeyNum5,
    KeyNum6,
    KeyNum7,
    KeyNum8,
    KeyNum9,
    KeyNumAdd,
    KeyNumDecimal,
    KeyNumDivide,
    KeyNumMultiply,
    KeyNumSubtract,
    KeyA,
    KeyS,
    KeyV,

    // --- ТипРамкиЭлементаУправления ----------------------------------
    ControlBorderTypeWithoutBorder,
    ControlBorderTypeSingle,
    ControlBorderTypeUnderline,

    // --- ВидЦвета -----------------------------------------------------
    ColorTypeAbsolute,
    ColorTypeStyleItem,
    ColorTypeWebColor,
    ColorTypeWindowsColor,
    ColorTypeAutoColor,

    // --- ТипЛинииЯчейкиТабличногоДокумента ----------------------------
    SpreadsheetDocumentCellLineTypeLargeDashed,
    SpreadsheetDocumentCellLineTypeDouble,
    SpreadsheetDocumentCellLineTypeNone,
    SpreadsheetDocumentCellLineTypeThinDashed,
    SpreadsheetDocumentCellLineTypeSolid,
    SpreadsheetDocumentCellLineTypeDotted,

    // --- ТипХранилищаСертификатовКриптографии -------------------------
    CryptoCertificateStoreTypeRootCertificates,
    CryptoCertificateStoreTypePersonalCertificates,
    CryptoCertificateStoreTypeRecipientCertificates,

    // --- ОбработкаТекстаИнтернетПочтовогоСообщения ---------------------
    InternetMailTextProcessingDontProcess,
    InternetMailTextProcessingProcess,

    // --- ТипДвустороннейПечати ---------------------------------------
    DuplexPrintingTypeNone,

    // --- ТипОбработкиНастроекВторогоФактораАутентификации -------------
    SecondAuthenticationFactorSettingsProcessingDontUse,
    SecondAuthenticationFactorSettingsProcessingUseNextIfFailed,

    // --- СпособПеретаскиванияФайлов ----------------------------------
    FileDragModeAsFileRef,
    FileDragModeAsFile,

    // --- ОтображениеГруппыКнопок -------------------------------------
    ButtonGroupRepresentationAuto,
    ButtonGroupRepresentationUsual,
    ButtonGroupRepresentationCompact,

    // --- Члены перечислений, достигнутые в УНФ ----------------------
    StandardPeriodVariantYesterday,
    StandardPeriodVariantTillEndOfThisYear,
    StandardPeriodVariantTillEndOfThisQuarter,
    StandardPeriodVariantTillEndOfThisMonth,
    StandardPeriodVariantTillEndOfThisHalfYear,
    StandardPeriodVariantTillEndOfThisTenDays,
    StandardPeriodVariantTillEndOfThisWeek,
    StandardPeriodVariantTomorrow,
    StandardPeriodVariantMonth,
    StandardPeriodVariantLast7Days,
    StandardPeriodVariantCustom,
    StandardPeriodVariantLastTenDays,
    StandardPeriodVariantLastTenDaysTillSameDayNumber,
    StandardPeriodVariantLastWeek,
    StandardPeriodVariantLastWeekTillSameWeekDay,
    StandardPeriodVariantLastHalfYear,
    StandardPeriodVariantLastHalfYearTillSameDate,
    StandardPeriodVariantLastYear,
    StandardPeriodVariantLastYearTillSameDate,
    StandardPeriodVariantLastQuarter,
    StandardPeriodVariantLastQuarterTillSameDate,
    StandardPeriodVariantLastMonth,
    StandardPeriodVariantLastMonthTillSameDate,
    StandardPeriodVariantFromBeginningOfThisYear,
    StandardPeriodVariantFromBeginningOfThisQuarter,
    StandardPeriodVariantFromBeginningOfThisMonth,
    StandardPeriodVariantFromBeginningOfThisHalfYear,
    StandardPeriodVariantFromBeginningOfThisTenDays,
    StandardPeriodVariantFromBeginningOfThisWeek,
    StandardPeriodVariantToday,
    StandardPeriodVariantNextTenDays,
    StandardPeriodVariantNextTenDaysTillSameDayNumber,
    StandardPeriodVariantNextWeek,
    StandardPeriodVariantNextWeekTillSameWeekDay,
    StandardPeriodVariantNextHalfYear,
    StandardPeriodVariantNextHalfYearTillSameDate,
    StandardPeriodVariantNext7Days,
    StandardPeriodVariantNextYear,
    StandardPeriodVariantNextYearTillSameDate,
    StandardPeriodVariantNextQuarter,
    StandardPeriodVariantNextQuarterTillSameDate,
    StandardPeriodVariantNextMonth,
    StandardPeriodVariantNextMonthTillSameDate,
    StandardPeriodVariantThisTenDays,
    StandardPeriodVariantThisWeek,
    StandardPeriodVariantThisHalfYear,
    StandardPeriodVariantThisYear,
    StandardPeriodVariantThisQuarter,
    StandardPeriodVariantThisMonth,
    TableHeightControlVariantUseHeightInFormRows,
    FullTextSearchVersionVersion1,
    VerticalFormScrollUseIfNecessary,
    VerticalAlignTop,
    VerticalAlignBottom,
    VerticalAlignCenter,
    AccumulationRecordTypeReceipt,
    AccumulationRecordTypeExpense,
    AccountingRecordTypeDebit,
    AccountingRecordTypeCredit,
    TextPositionRelativeToPictureAuto,
    TextPositionRelativeToPictureOnTop,
    TextPositionRelativeToPictureTop,
    TextPositionRelativeToPictureLeft,
    TextPositionRelativeToPictureBottom,
    TextPositionRelativeToPictureRight,
    HorizontalAlignAuto,
    HorizontalAlignLeft,
    HorizontalAlignJustify,
    HorizontalAlignRight,
    HorizontalAlignCenter,
    ColumnsGroupInCell,
    ColumnsGroupVertical,
    ColumnsGroupHorizontal,
    AllowedMessageNoGreater,
    FoldersAndItemsUseFolders,
    FoldersAndItemsUseFoldersAndItems,
    FoldersAndItemsUseItems,
    DataCompositionParameterUseAuto,
    DataCompositionParameterUseAlways,
    ScrollBarUseAutoUse,
    ScrollBarUseDontUse,
    CurrentRowUseAuto,
    /// Порядок по возрастанию значений, измеренный на платформе 1С.
    QuerySchemaOrderDirectionAscending,
    QuerySchemaOrderDirectionDescending,
    InitialTreeViewNoExpand,
    InitialTreeViewExpandAllLevels,
    UpdateOnDataChangeAuto,
    EditTextUpdateDontUse,
    EditTextUpdateOnValueChange,
    WarningOnEditRepresentationDontShow,
    WarningOnEditRepresentationShow,
    SearchInTableOnInputDontUse,
    FormCommandBarLabelLocationAuto,
    FormCommandBarLabelLocationNone,
    FormCommandBarLabelLocationBottom,
    FormItemCommandBarLabelLocationAuto,
    FormItemCommandBarLabelLocationTop,
    FormItemCommandBarLabelLocationNone,
    GaugeChartValuesScaleLabelsLocationAtScale,
    /// Автоматическое положение подписи относительно края диаграммы.
    ChartLabelLocationEdgeAuto,
    /// Подпись с края внутри значения диаграммы.
    ChartLabelLocationEdgeInside,
    /// Отсутствие подписей шкалы диаграммы.
    ChartScaleLabelLocationNone,
    /// Автоматическое отображение в легенде диаграммы.
    ShowInChartLegendAuto,
    /// Отображение в легенде диаграммы.
    ShowInChartLegendShow,
    /// Отсутствие отображения в легенде диаграммы.
    ShowInChartLegendDontShow,
    /// Расшифровка табличного документа без обработки.
    SpreadsheetDocumentDetailUseWithoutProcessing,
    /// Расшифровка отдельной ячейки табличного документа.
    SpreadsheetDocumentDetailUseCell,
    ViewStatusLocationTop,
    SearchStringLocationAuto,
    SearchStringLocationNone,
    SearchControlLocationNone,
    AutoTimeModeLast,
    AutoTimeModeCurrentOrLast,
    TableRowInputModeEndOfList,
    CryptoCertificateIncludeModeIncludeWholeChain,
    CryptoCertificateIncludeModeIncludeSubjectCertificate,
    CryptoCertificateIncludeModeDontInclude,
    TableRowSelectionModeRow,
    TableRowSelectionModeCell,
    TableSelectionModeSingleRow,
    ReplacementModeAppend,
    ReplacementModeReplace,
    ReplacementModeUpdate,
    ReplacementModeMerge,
    ReplacementModeDelete,
    SizeChangeModeQuickChange,
    SizeChangeModeNormal,
    FormWindowOpeningModeLockWholeInterface,
    FormWindowOpeningModeLockOwnerWindow,
    FormWindowOpeningModeIndependent,
    DataCompositionSettingsViewModeQuickAccess,
    DataCompositionSettingsViewModeAll,
    ReportResultViewModeAuto,
    ReportResultViewModeCompact,
    ReportResultViewModeDefault,
    FullTextSearchModeDisable,
    FullTextSearchModeEnable,
    CryptoCertificateCheckModeIgnoreTimeValidity,
    CryptoCertificateCheckModeIgnoreSignatureValidity,
    CryptoCertificateCheckModeIgnoreCertificateRevocationStatus,
    CryptoCertificateCheckModeAllowTestCertificates,
    ColumnEditModeEnter,
    ColumnEditModeEnterOnInput,
    ColumnEditModeDirectly,
    InfoBaseUserPasswordRecoveryMethodSendVerificationCodeBySetParameters,
    InfoBaseUserPasswordRecoveryMethodSendVerificationCodeByStandardService,
    SpreadsheetDocumentAreaFillTypeParameter,
    SpreadsheetDocumentAreaFillTypeText,
    SpreadsheetDocumentAreaFillTypeTemplate,
    QuerySchemaUnionTypeUnion,
    QuerySchemaUnionTypeUnionAll,
    CryptoSignatureTypeCAdESAv2,
    CryptoSignatureTypeCAdESAv3,
    CryptoSignatureTypeCAdESBES,
    CryptoSignatureTypeCAdESC,
    CryptoSignatureTypeCAdEST,
    CryptoSignatureTypeCAdESXLong,
    CryptoSignatureTypeCAdESXLongType1,
    CryptoSignatureTypeCAdESXLongType2,
    CryptoSignatureTypeCAdESXType1,
    CryptoSignatureTypeCAdESXType2,
    CryptoSignatureTypeCMS,
    SpreadsheetDocumentTextPlacementTypeCut,
    SpreadsheetDocumentTextPlacementTypeWrap,
    QuerySchemaJoinTypeInner,
    QuerySchemaJoinTypeLeftOuter,
    QuerySchemaJoinTypeRightOuter,
    ReportFormTypeSettings,
    ReportFormTypeMain,
    FormattedDocumentItemTypeText,
    FormattedDocumentItemTypeLineFeed,
    FormattedDocumentItemTypePicture,
    FixingInTableLeft,
    FixingInTableNone,
    FixingInTableRight,

    // --- СкоростьКлиентскогоСоединения ---------------------------------
    ClientConnectionSpeedLow,
    ClientConnectionSpeedNormal,

    // --- ТипОбластиЯчеекТабличногоДокумента ----------------------------
    SpreadsheetDocumentCellAreaTypeTable,
    SpreadsheetDocumentCellAreaTypeRows,
    SpreadsheetDocumentCellAreaTypeColumns,
    SpreadsheetDocumentCellAreaTypeRectangle,

    // --- ТипПодключенияВнешнейКомпоненты -------------------------------
    AddInConnectionTypeIsolated,
    AddInConnectionTypeNotIsolated,

    // --- ПроверкаЗаполнения --------------------------------------------
    FillCheckingShowError,
    FillCheckingDontCheck,

    // --- ГруппыИЭлементы -----------------------------------------------
    FoldersAndItemsAuto,
    FoldersAndItemsFolders,
    FoldersAndItemsBoth,
    FoldersAndItemsItems,

    // --- ПротоколИнтернетПочты ----------------------------------------
    // Состав и представления измерены пробой `xml-canonicalization-type/measure.bsl`.
    XmlCanonicalizationTypeCanonical,
    XmlCanonicalizationTypeCanonicalWithComments,
    XmlCanonicalizationTypeExclusive,
    XmlCanonicalizationTypeExclusiveWithComments,
    XmlCanonicalizationTypeCanonical11,
    XmlCanonicalizationTypeCanonical11WithComments,
    Pop3AuthenticationModeApop,
    Pop3AuthenticationModeCramMd5,
    Pop3AuthenticationModeGeneral,
    SmtpAuthenticationModeCramMd5,
    SmtpAuthenticationModeLogin,
    SmtpAuthenticationModePlain,
    SmtpAuthenticationModeNone,
    SmtpAuthenticationModeDefault,
    InternetMailProtocolImap,
    InternetMailProtocolPop3,
    InternetMailProtocolSmtp,

    // --- ДействиеОбработкиРасшифровкиКомпоновкиДанных -----------------
    DataCompositionDetailsProcessingActionNone,
    DataCompositionDetailsProcessingActionOpenValue,
    DataCompositionDetailsProcessingActionFilter,
    DataCompositionDetailsProcessingActionApplyAppearance,
    DataCompositionDetailsProcessingActionDrillDown,
    DataCompositionDetailsProcessingActionGroup,
    DataCompositionDetailsProcessingActionOrder,

    // --- ЧастиДаты -------------------------------------------------------
    DateFractionsTime,
    DateFractionsDate,
    DateFractionsDateTime,

    // --- ДопустимаяДлина -------------------------------------------------
    AllowedLengthVariable,
    AllowedLengthFixed,

    // --- ХешФункция -----------------------------------------------------
    HashCrc32,
    HashMd5,
    HashPbkdf2Sha256,
    HashSha1,
    HashSha256,
    HashSha512,

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

    // Новый тип архива не равен ZIP-перечислению; измерено на 8.3.27.
    ArchiveRestorePaths,
    ArchiveDontRestorePaths,

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
    // Три члена: `КодировкаОС`, `OEM` и `ANSI` платформа не знает.
    ZipNamesAuto,
    ZipNamesUtf8,
    ZipNamesOsWithUtf8,
    /// Форматы файла форматированного документа, измеренные на 1С.
    FormattedDocumentAnsiTxt,
    FormattedDocumentHtml,
    FormattedDocumentPdf,
    FormattedDocumentTxt,

    // --- ИспользованиеByteOrderMark ------------------------------------
    ByteOrderMarkUse,
    ByteOrderMarkDoNotUse,

    // --- ВидСчета ------------------------------------------------------
    AccountTypeActive,
    AccountTypePassive,
    AccountTypeActivePassive,

    // Полный состав измерен на 1С 8.3.27.2342.
    GraphicalSchemaItemPictureLocationTop,
    GraphicalSchemaItemPictureLocationLeft,
    GraphicalSchemaItemPictureLocationBottom,
    GraphicalSchemaItemPictureLocationRight,
    GraphicalSchemaItemPictureLocationCenter,

    // Полные наборы графических констант измерены на 1С 8.3.27.2342.
    GraphicalSchemaShapesBlock,
    GraphicalSchemaShapesDocument,
    GraphicalSchemaShapesNone,
    GraphicalSchemaShapesFolder,
    GraphicalSchemaShapesVerticalBrackets,
    GraphicalSchemaShapesHorizontalBrackets,
    GraphicalSchemaShapesUpArrow,
    GraphicalSchemaShapesUpDownArrow,
    GraphicalSchemaShapesLeftArrow,
    GraphicalSchemaShapesLeftRightArrow,
    GraphicalSchemaShapesDownArrow,
    GraphicalSchemaShapesRightArrow,
    GraphicalSchemaShapesFile,
    GraphicalSchemaShapesEllipse,
    ConnectorLineTypeNone,
    ConnectorLineTypeDashed,
    ConnectorLineTypeDashDotted,
    ConnectorLineTypeDashDottedDotted,
    ConnectorLineTypeSolid,
    ConnectorLineTypeDotted,
    ConnectorTextLocationFirstSegment,
    ConnectorTextLocationMiddle,
    ArrowStyleFilled,
    ArrowStyleBlank,
    ArrowStyleNone,
    // Значения подтверждены measure-common-attribute-property-enums на 8.3.27.2342.
    CommonAttributeDataSeparationDontUse,
    CommonAttributeDataSeparationSeparate,
    CommonAttributeAutoUseDontUse,
    CommonAttributeAutoUseUse,
    CommonAttributeUseAuto,
    CommonAttributeUseDontUse,
    CommonAttributeUseUse,
    /// Отсутствие узора табличного документа.
    SpreadsheetDocumentPatternTypeWithoutPattern,
    /// Расположение области построения по координатам.
    ChartPlotAreaPlacementUseCoordinates,
    /// Измеренный алгоритм подписи токена доступа HS256.
    AccessTokenSignAlgorithmHs256,
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
    /// Режим открытия форм клиентского приложения.
    ApplicationFormsOpenningMode,
    /// Вариант интерфейса клиентского приложения.
    ClientApplicationInterfaceVariant,
    /// Способ отображения разделов приложения.
    SectionsPanelRepresentation,
    /// Источник списка расширений конфигурации.
    ConfigurationExtensionsSource,
    /// Область действия расширения конфигурации.
    ConfigurationExtensionScope,
    /// Назначение расширения из пространства свойств метаданных.
    ConfigurationExtensionPurpose,
    /// Важность проблемы применения расширения конфигурации.
    ConfigurationExtensionApplicationIssueSeverity,
    /// Уровень важности события журнала регистрации.
    EventLogLevel,
    /// Связь записи журнала регистрации с текущей транзакцией.
    EventLogEntryTransactionMode,
    /// Статус транзакции записи журнала регистрации.
    EventLogEntryTransactionStatus,
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
    /// Режим записи документа.
    DocumentWriteMode,
    /// Способ обхода записей результата запроса.
    QueryResultIteration,
    /// Тип связи наборов данных компоновки данных.
    DataCompositionDataSetsLinkType,
    /// Тип соединения значений по сериям диаграммы.
    ChartValuesBySeriesConnectionType,
    /// Расположение легенды диаграммы компоновки данных.
    DataCompositionChartLegendPlacement,
    /// Тип записи выборки результата запроса.
    QueryRecordType,
    /// Режим управляемой блокировки данных.
    DataLockMode,
    /// Способ округления половины.
    RoundMode,
    /// Вид числового значения.
    NumericValueType,
    /// Способ отображения сообщения об ошибке.
    ErrorMessageDisplayVariant,
    /// Режим использования параметра команды.
    CommandParameterUseMode,
    /// Способ поиска строки при вводе по строке.
    SearchStringModeOnInputByString,
    /// Тип файла пакета отображаемых документов.
    RepresentableDocumentBatchFileType,
    /// Вид шрифта.
    FontType,
    /// Вид картинки.
    PictureType,
    /// Использование метаданных при полнотекстовом поиске.
    FullTextSearchMetadataUse,
    /// Режим изменения связанного значения.
    LinkedValueChangeMode,
    /// Способ кодирования вложения интернет-почты.
    InternetMailAttachmentEncodingMode,
    /// Автоматическое сохранение данных формы в настройках.
    AutoSaveFormDataInSettings,
    /// Формат файла документа PDF.
    PDFDocumentFileType,
    /// Допустимый знак числа.
    AllowedSign,
    /// Режим автоматической регистрации изменений данных.
    AutoChangeRecord,
    /// Способ отображения кнопки формы.
    ButtonRepresentation,
    /// Причина несоответствия пароля политике.
    PasswordPolicyComplianceCheckResult,
    /// Способ отображения данных в таблице формы.
    TableRepresentation,
    /// Вид поля формы клиентского приложения.
    FormFieldType,
    /// Вид группы формы клиентского приложения.
    FormGroupType,
    /// Горизонтальное положение элемента формы.
    ItemHorizontalLocation,
    /// Вертикальное положение элемента формы.
    ItemVerticalAlign,
    /// Интервал между элементами формы.
    FormItemSpacing,
    /// Отображение фигуры кнопки формы.
    ButtonShapeRepresentation,
    /// Отображение обычной группы формы.
    UsualGroupRepresentation,
    /// Важность элемента формы при отображении.
    DisplayImportance,
    /// Формат хранения картинки.
    PictureFormat,
    /// Тип текста почтового сообщения.
    InternetMailTextType,
    /// Способ кодирования не-ASCII символов почтового сообщения.
    InternetMailNonAsciiEncodingMode,
    /// Важность интернет-почтового сообщения.
    InternetMailMessageImportance,
    /// Дополнительный способ отображения состояния.
    AdditionalShowMode,
    /// Режим проверки отзыва TLS-сертификата сервера.
    ServerTlsCertificateRevocationCheckMode,
    /// Область применения отбора к записям результа компоновки.
    DataCompositionFilterApplicationType,
    /// Состояние элемента настройки компоновки данных.
    DataCompositionSettingsItemState,
    /// Режим отображения элемента настройки компоновки.
    DataCompositionSettingsItemViewMode,
    /// Способ восстановления настроек компоновки данных.
    DataCompositionSettingsRefreshMethod,
    /// Направление сортировки компоновки данных.
    DataCompositionSortDirection,
    /// Тип группировки компоновки данных.
    DataCompositionGroupType,
    /// Расположение группировки компоновки данных.
    DataCompositionGroupPlacement,
    /// Расположение полей группировки компоновки данных.
    DataCompositionGroupFieldsPlacement,
    /// Расположение реквизитов компоновки данных.
    DataCompositionAttributesPlacement,
    /// ВидДекорацииФормы.
    FormDecorationType,
    /// ВидПереключателя.
    RadioButtonType,
    /// ГруппировкаПодчиненныхЭлементовФормы.
    ChildFormItemsGroup,
    /// ИсторияВыбораПриВводе.
    ChoiceHistoryOnInput,
    /// ОтображениеКнопкиВыбора.
    ChoiceButtonRepresentation,
    /// ОтображениеУправленияОбычнойГруппы.
    UsualGroupControlRepresentation,
    /// ПоведениеОбычнойГруппы.
    UsualGroupBehavior,
    /// ПоложениеЗаголовкаЭлементаФормы.
    FormItemTitleLocation,
    /// ПоложениеКнопкиВКоманднойПанели.
    ButtonLocationInCommandBar,
    /// РасположениеРесурсовКомпоновкиДанных.
    DataCompositionResourcesPlacement,
    /// ИспользованиеУсловногоОформленияКомпоновкиДанных.
    DataCompositionConditionalAppearanceUse,
    /// РасположениеПоляКомпоновкиДанных.
    DataCompositionFieldPlacement,
    /// ТипБухгалтерскогоОстаткаКомпоновкиДанных.
    DataCompositionAccountingBalanceType,
    /// ТипОстаткаКомпоновкиДанных.
    DataCompositionBalanceType,
    /// ТипПериодаКомпоновкиДанных.
    DataCompositionPeriodType,
    /// ТипДополненияПериодаКомпоновкиДанных.
    DataCompositionPeriodAdditionType,
    /// ГлубинаЦвета.
    ColorDepth,
    /// ОтображениеПодсказки.
    ToolTipRepresentation,
    /// ТипКлиентскогоПриложения.
    ClientApplicationType,
    /// Тип вывода текста компоновки данных.
    DataCompositionTextOutputType,
    /// Тип размещения текста компоновки данных.
    DataCompositionTextPlacementType,
    /// Режим масштабирования просмотра документа.
    ViewScalingMode,
    /// Расположение итогов компоновки данных.
    DataCompositionTotalPlacement,
    /// Размер картинки в области отображения.
    PictureSize,
    /// Тип смещения ячеек табличного документа.
    SpreadsheetDocumentShiftType,
    /// Тип диаграммы.
    ChartType,
    /// Ориентация подписей диаграммы.
    ChartLabelsOrientation,
    /// Вид включения граничного значения.
    BoundaryType,
    /// Режим ограничения количества серий диаграммы.
    MaxSeries,
    /// Вид подписи к области диаграммы.
    ChartLabelType,
    /// Стиль линии диаграммы.
    ChartLineType,
    /// Вид кнопки формы.
    FormButtonType,
    /// Способ назначения типа при записи значения в XML.
    XmlTypeAssignment,
    /// Тип операционной системы и архитектуры платформы.
    PlatformType,
    /// Операция сравнения в настройках системы компоновки данных.
    DataCompositionComparisonType,
    /// Операция сравнения в обычном отборе.
    ComparisonType,
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
    /// Способ отображения интервала диаграммы Ганта.
    GanttChartIntervalRepresentation,
    /// Способ вертикального растягивания диаграммы Ганта.
    GanttChartVerticalStretch,
    /// Расположение легенды диаграммы.
    ChartLegendPlacement,
    /// Единица шкалы времени.
    TimeScaleUnitType,
    /// Формат дня шкалы времени.
    TimeScaleDayFormat,
    /// `ОриентацияСтраницы` — свойство `ТабличныйДокумент.ОриентацияСтраницы`
    /// и пара с идентификатором 1 в параметрах страницы MXL.
    PageOrientation,
    TextEncoding,
    /// Два режима percent-кодирования строки для query и полного URL.
    StringEncodingMethod,
    /// Направление сортировки коллекций; Connector использует возрастание.
    SortDirection,
    /// Управление выводом поля формы.
    UseOutput,
    /// Размещение страниц в группе формы.
    FormPagesRepresentation,
    /// Расположение картинки на кнопке формы.
    FormButtonPictureLocation,
    /// Тип линии рисунка табличного документа.
    SpreadsheetDocumentDrawingLineType,
    /// Представление поля-флажка в форме.
    CheckBoxType,
    /// Звук доставляемого уведомления.
    DeliverableNotificationSubscriberType,
    SoundAlert,
    /// Сквозное выравнивание элементов формы.
    ThroughAlign,
    /// Фигура кнопки формы.
    ButtonShape,
    /// Тип группы элементов отбора компоновки данных.
    DataCompositionFilterItemsGroupType,
    /// Клавиши для сочетаний клавиш.
    Key,
    /// Тип рамки элемента управления.
    ControlBorderType,
    /// Вид цвета объекта `Цвет`.
    ColorType,
    /// Вид линии границы ячейки табличного документа.
    SpreadsheetDocumentCellLineType,
    /// Тип хранилища сертификатов криптографии.
    CryptoCertificateStoreType,
    /// Обработка текстов интернет-почтового сообщения.
    InternetMailTextProcessing,
    /// Тип двусторонней печати.
    DuplexPrintingType,
    /// Порядок обработки настроек второго фактора аутентификации.
    SecondAuthenticationFactorSettingsProcessingType,
    /// Способ перетаскивания файлов в элемент формы.
    FileDragMode,
    /// Отображение группы кнопок командной панели.
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
    /// Тип члена перечисления `ВидДвиженияБухгалтерии`, измеренный на 1С.
    AccountingRecordType,
    /// Положение текста относительно картинки; состав и представления измерены на 1С.
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
    /// Использование расшифровки табличного документа.
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
    /// Скорость соединения текущего клиента.
    ClientConnectionSpeed,
    /// Тип области ячеек табличного документа.
    SpreadsheetDocumentCellAreaType,
    /// Тип подключения внешней компоненты.
    AddInConnectionType,
    /// Проверка заполнения реквизита.
    FillChecking,
    /// Выбор групп и элементов иерархических данных.
    FoldersAndItems,
    /// Протокол интернет-почты.
    /// Устаревшие способы аутентификации, используемые исходниками 1С.
    XmlCanonicalizationType,
    Pop3AuthenticationMode,
    SmtpAuthenticationMode,
    InternetMailProtocol,
    /// Действие при обработке расшифровки компоновки данных.
    DataCompositionDetailsProcessingAction,
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
    /// Отдельное перечисление восстановления путей архива, начиная с 8.3.26.
    ArchiveFileRestorePathsMode,
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
    /// Формат сохранения форматированного документа.
    FormattedDocumentFileType,
    /// Управляет сигнатурой при записи строкового HTTP-тела.
    ByteOrderMarkUse,
    /// Вид бухгалтерского счёта.
    AccountType,
    /// Положение картинки внутри элемента графической схемы.
    GraphicalSchemaItemPictureLocation,
    /// `ФигурыГрафическойСхемы` — системное перечисление графической схемы.
    GraphicalSchemaShapes,
    /// `ТипСоединительнойЛинии` — системное перечисление графической схемы.
    ConnectorLineType,
    /// `ПоложениеТекстаСоединительнойЛинии` — системное перечисление графической схемы.
    ConnectorTextLocation,
    /// `СтильСтрелки` — системное перечисление графической схемы.
    ArrowStyle,
    /// РазделениеДанныхОбщегоРеквизита.
    CommonAttributeDataSeparation,
    /// АвтоИспользованиеОбщегоРеквизита.
    CommonAttributeAutoUse,
    /// ИспользованиеОбщегоРеквизита.
    CommonAttributeUse,
    /// Тип узора табличного документа.
    SpreadsheetDocumentPatternType,
    /// Тип расположения области построения диаграммы.
    ChartPlotAreaPlacement,
    /// Алгоритм подписи токена доступа.
    AccessTokenSignAlgorithm,
}

impl EnumKind {
    /// Написания самого перечисления: `(русское, английское)`.
    const fn names(self) -> (&'static str, &'static str) {
        match self {
            EnumKind::ScriptVariant => ("ВариантВстроенногоЯзыка", "ScriptVariant"),
            EnumKind::CompatibilityMode => ("РежимСовместимости", "CompatibilityMode"),
            EnumKind::ClientRunMode => ("РежимЗапускаКлиентскогоПриложения", "ClientRunMode"),
            EnumKind::ApplicationFormsOpenningMode => (
                "РежимОткрытияФормПриложения",
                "ApplicationFormsOpenningMode",
            ),
            EnumKind::ClientApplicationInterfaceVariant => (
                "ВариантИнтерфейсаКлиентскогоПриложения",
                "ClientApplicationInterfaceVariant",
            ),
            EnumKind::SectionsPanelRepresentation => {
                ("ОтображениеПанелиРазделов", "SectionsPanelRepresentation")
            }
            EnumKind::ConfigurationExtensionsSource => (
                "ИсточникРасширенийКонфигурации",
                "ConfigurationExtensionsSource",
            ),
            EnumKind::ConfigurationExtensionScope => (
                "ОбластьДействияРасширенияКонфигурации",
                "ConfigurationExtensionScope",
            ),
            EnumKind::ConfigurationExtensionPurpose => (
                "НазначениеРасширенияКонфигурации",
                "ConfigurationExtensionPurpose",
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
            EnumKind::EventLogEntryTransactionStatus => (
                "СтатусТранзакцииЗаписиЖурналаРегистрации",
                "EventLogEntryTransactionStatus",
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
            EnumKind::DocumentWriteMode => ("РежимЗаписиДокумента", "DocumentWriteMode"),
            EnumKind::QueryResultIteration => ("ОбходРезультатаЗапроса", "QueryResultIteration"),
            EnumKind::DataCompositionDataSetsLinkType => (
                "ТипСвязиНаборовДанныхКомпоновкиДанных",
                "DataCompositionDataSetsLinkType",
            ),
            EnumKind::ChartValuesBySeriesConnectionType => (
                "ТипСоединенияЗначенийПоСериямДиаграммы",
                "ChartValuesBySeriesConnectionType",
            ),
            EnumKind::DataCompositionChartLegendPlacement => (
                "РасположениеЛегендыДиаграммыКомпоновкиДанных",
                "DataCompositionChartLegendPlacement",
            ),
            EnumKind::QueryRecordType => ("ТипЗаписиЗапроса", "QueryRecordType"),
            EnumKind::DataLockMode => ("РежимБлокировкиДанных", "DataLockMode"),
            EnumKind::AccountType => ("ВидСчета", "AccountType"),
            EnumKind::RoundMode => ("РежимОкругления", "RoundMode"),
            EnumKind::NumericValueType => ("ВидЧисловогоЗначения", "NumericValueType"),
            EnumKind::ErrorMessageDisplayVariant => (
                "ВариантОтображенияСообщенияОбОшибке",
                "ErrorMessageDisplayVariant",
            ),
            EnumKind::CommandParameterUseMode => (
                "РежимИспользованияПараметраКоманды",
                "CommandParameterUseMode",
            ),
            EnumKind::SearchStringModeOnInputByString => (
                "СпособПоискаСтрокиПриВводеПоСтроке",
                "SearchStringModeOnInputByString",
            ),
            EnumKind::RepresentableDocumentBatchFileType => (
                "ТипФайлаПакетаОтображаемыхДокументов",
                "RepresentableDocumentBatchFileType",
            ),
            EnumKind::FontType => ("ВидШрифта", "FontType"),
            EnumKind::PictureType => ("ВидКартинки", "PictureType"),
            EnumKind::FullTextSearchMetadataUse => (
                "ИспользованиеМетаданныхПолнотекстовогоПоиска",
                "FullTextSearchMetadataUse",
            ),
            EnumKind::LinkedValueChangeMode => {
                ("РежимИзмененияСвязанногоЗначения", "LinkedValueChangeMode")
            }
            EnumKind::InternetMailAttachmentEncodingMode => (
                "СпособКодированияИнтернетПочтовогоВложения",
                "InternetMailAttachmentEncodingMode",
            ),
            EnumKind::AutoSaveFormDataInSettings => (
                "АвтоматическоеСохранениеДанныхФормыВНастройках",
                "AutoSaveFormDataInSettings",
            ),
            EnumKind::PDFDocumentFileType => ("ТипФайлаДокументаPDF", "PDFDocumentFileType"),
            EnumKind::AllowedSign => ("ДопустимыйЗнак", "AllowedSign"),
            EnumKind::AutoChangeRecord => ("АвтоРегистрацияИзменений", "AutoChangeRecord"),
            EnumKind::ButtonRepresentation => ("ОтображениеКнопки", "ButtonRepresentation"),
            EnumKind::PasswordPolicyComplianceCheckResult => (
                "РезультатПроверкиСоответствияПароляПолитике",
                "PasswordPolicyComplianceCheckResult",
            ),
            EnumKind::TableRepresentation => ("ОтображениеТаблицы", "TableRepresentation"),
            EnumKind::FormFieldType => ("ВидПоляФормы", "FormFieldType"),
            EnumKind::FormGroupType => ("ВидГруппыФормы", "FormGroupType"),
            EnumKind::ItemHorizontalLocation => {
                ("ГоризонтальноеПоложениеЭлемента", "ItemHorizontalLocation")
            }
            EnumKind::ItemVerticalAlign => ("ВертикальноеПоложениеЭлемента", "ItemVerticalAlign"),
            EnumKind::FormItemSpacing => ("ИнтервалМеждуЭлементамиФормы", "FormItemSpacing"),
            EnumKind::ButtonShapeRepresentation => {
                ("ОтображениеФигурыКнопки", "ButtonShapeRepresentation")
            }
            EnumKind::UsualGroupRepresentation => {
                ("ОтображениеОбычнойГруппы", "UsualGroupRepresentation")
            }
            EnumKind::DisplayImportance => ("ВажностьПриОтображении", "DisplayImportance"),
            EnumKind::PictureFormat => ("ФорматКартинки", "PictureFormat"),
            EnumKind::InternetMailTextType => {
                ("ТипТекстаПочтовогоСообщения", "InternetMailTextType")
            }
            EnumKind::InternetMailNonAsciiEncodingMode => (
                "СпособКодированияНеASCIIСимволовИнтернетПочтовогоСообщения",
                "InternetMailMessageNonASCIISymbolsEncodingMode",
            ),
            EnumKind::InternetMailMessageImportance => (
                "ВажностьИнтернетПочтовогоСообщения",
                "InternetMailMessageImportance",
            ),
            EnumKind::AdditionalShowMode => {
                ("ДополнительныйРежимОтображения", "AdditionalShowMode")
            }
            EnumKind::ServerTlsCertificateRevocationCheckMode => (
                "РежимПроверкиОтзываTLSСертификатаСервера",
                "ServerTLSCertificateRevocationCheckMode",
            ),
            EnumKind::DataCompositionFilterApplicationType => (
                "ТипПримененияОтбораКомпоновкиДанных",
                "DataCompositionFilterApplicationType",
            ),
            EnumKind::DataCompositionSettingsItemState => (
                "СостояниеЭлементаНастройкиКомпоновкиДанных",
                "DataCompositionSettingsItemState",
            ),
            EnumKind::DataCompositionSettingsItemViewMode => (
                "РежимОтображенияЭлементаНастройкиКомпоновкиДанных",
                "DataCompositionSettingsItemViewMode",
            ),
            EnumKind::DataCompositionSettingsRefreshMethod => (
                "СпособВосстановленияНастроекКомпоновкиДанных",
                "DataCompositionSettingsRefreshMethod",
            ),
            EnumKind::DataCompositionTextPlacementType => (
                "ТипРазмещенияТекстаКомпоновкиДанных",
                "DataCompositionTextPlacementType",
            ),
            EnumKind::DataCompositionTextOutputType => (
                "ТипВыводаТекстаКомпоновкиДанных",
                "DataCompositionTextOutputType",
            ),
            EnumKind::ViewScalingMode => ("РежимМасштабированияПросмотра", "ViewScalingMode"),
            EnumKind::DataCompositionTotalPlacement => (
                "РасположениеИтоговКомпоновкиДанных",
                "DataCompositionTotalPlacement",
            ),
            EnumKind::PictureSize => ("РазмерКартинки", "PictureSize"),
            EnumKind::SpreadsheetDocumentShiftType => (
                "ТипСмещенияТабличногоДокумента",
                "SpreadsheetDocumentShiftType",
            ),
            EnumKind::ChartType => ("ТипДиаграммы", "ChartType"),
            EnumKind::ChartLabelsOrientation => {
                ("ОриентацияПодписейДиаграммы", "ChartLabelsOrientation")
            }
            EnumKind::BoundaryType => ("ВидГраницы", "BoundaryType"),
            EnumKind::MaxSeries => ("МаксимумСерий", "MaxSeries"),
            EnumKind::ChartLabelType => ("ВидПодписейКДиаграмме", "ChartLabelType"),
            EnumKind::ChartLineType => ("ТипЛинииДиаграммы", "ChartLineType"),
            EnumKind::FormButtonType => ("ВидКнопкиФормы", "FormButtonType"),
            EnumKind::XmlTypeAssignment => ("НазначениеТипаXML", "XMLTypeAssignment"),
            EnumKind::PlatformType => ("ТипПлатформы", "PlatformType"),
            EnumKind::DataCompositionComparisonType => (
                "ВидСравненияКомпоновкиДанных",
                "DataCompositionComparisonType",
            ),
            EnumKind::ComparisonType => ("ВидСравнения", "ComparisonType"),
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
            EnumKind::GanttChartIntervalRepresentation => (
                "ОтображениеИнтервалаДиаграммыГанта",
                "GanttChartIntervalRepresentation",
            ),
            EnumKind::ChartLegendPlacement => {
                ("РасположениеЛегендыДиаграммы", "ChartLegendPlacement")
            }
            EnumKind::GanttChartVerticalStretch => (
                "РастягиваниеПоВертикалиДиаграммыГанта",
                "GanttChartVerticalStretch",
            ),
            EnumKind::TimeScaleUnitType => ("ТипЕдиницыШкалыВремени", "TimeScaleUnitType"),
            EnumKind::TimeScaleDayFormat => ("ФорматДняШкалыВремени", "TimeScaleDayFormat"),
            EnumKind::PageOrientation => ("ОриентацияСтраницы", "PageOrientation"),
            EnumKind::TextEncoding => ("КодировкаТекста", "TextEncoding"),
            EnumKind::StringEncodingMethod => {
                ("СпособКодированияСтроки", "СпособКодированияСтроки")
            }
            EnumKind::SortDirection => ("НаправлениеСортировки", "SortDirection"),
            EnumKind::UseOutput => ("ИспользованиеВывода", "UseOutput"),
            EnumKind::FormPagesRepresentation => {
                ("ОтображениеСтраницФормы", "FormPagesRepresentation")
            }
            EnumKind::GraphicalSchemaShapes => ("ФигурыГрафическойСхемы", "GraphicalSchemaShapes"),
            EnumKind::ConnectorLineType => ("ТипСоединительнойЛинии", "ConnectorLineType"),
            EnumKind::ConnectorTextLocation => (
                "ПоложениеТекстаСоединительнойЛинии",
                "ConnectorTextLocation",
            ),
            EnumKind::ArrowStyle => ("СтильСтрелки", "ArrowStyle"),
            EnumKind::GraphicalSchemaItemPictureLocation => (
                "ПоложениеКартинкиЭлементаГрафическойСхемы",
                "GraphicalSchemaItemPictureLocation",
            ),
            EnumKind::FormButtonPictureLocation => {
                ("ПоложениеКартинкиКнопкиФормы", "FormButtonPictureLocation")
            }
            EnumKind::SpreadsheetDocumentDrawingLineType => (
                "ТипЛинииРисункаТабличногоДокумента",
                "SpreadsheetDocumentDrawingLineType",
            ),
            EnumKind::CheckBoxType => ("ВидФлажка", "CheckBoxType"),
            EnumKind::DeliverableNotificationSubscriberType => (
                "ТипПодписчикаДоставляемыхУведомлений",
                "DeliverableNotificationSubscriberType",
            ),
            EnumKind::SoundAlert => ("ЗвуковоеОповещение", "SoundAlert"),
            EnumKind::ThroughAlign => ("СквозноеВыравнивание", "ThroughAlign"),
            EnumKind::ButtonShape => ("ФигураКнопки", "ButtonShape"),
            EnumKind::DataCompositionFilterItemsGroupType => (
                "ТипГруппыЭлементовОтбораКомпоновкиДанных",
                "DataCompositionFilterItemsGroupType",
            ),
            EnumKind::Key => ("Клавиша", "Key"),
            EnumKind::ControlBorderType => ("ТипРамкиЭлементаУправления", "ControlBorderType"),
            EnumKind::ColorType => ("ВидЦвета", "ColorType"),
            EnumKind::SpreadsheetDocumentCellLineType => (
                "ТипЛинииЯчейкиТабличногоДокумента",
                "SpreadsheetDocumentCellLineType",
            ),
            EnumKind::CryptoCertificateStoreType => (
                "ТипХранилищаСертификатовКриптографии",
                "CryptoCertificateStoreType",
            ),
            EnumKind::InternetMailTextProcessing => (
                "ОбработкаТекстаИнтернетПочтовогоСообщения",
                "InternetMailTextProcessing",
            ),
            EnumKind::DuplexPrintingType => ("ТипДвустороннейПечати", "DuplexPrintingType"),
            EnumKind::SecondAuthenticationFactorSettingsProcessingType => (
                "ТипОбработкиНастроекВторогоФактораАутентификации",
                "SecondAuthenticationFactorSettingsProcessingType",
            ),
            EnumKind::FileDragMode => ("СпособПеретаскиванияФайлов", "FileDragMode"),
            EnumKind::ButtonGroupRepresentation => {
                ("ОтображениеГруппыКнопок", "ButtonGroupRepresentation")
            }
            EnumKind::StandardPeriodVariant => {
                ("ВариантСтандартногоПериода", "StandardPeriodVariant")
            }
            EnumKind::TableHeightControlVariant => (
                "ВариантУправленияВысотойТаблицы",
                "TableHeightControlVariant",
            ),
            EnumKind::FullTextSearchVersion => {
                ("ВерсияПолнотекстовогоПоиска", "FullTextSearchVersion")
            }
            EnumKind::VerticalFormScroll => ("ВертикальнаяПрокруткаФормы", "VerticalFormScroll"),
            EnumKind::VerticalAlign => ("ВертикальноеПоложение", "VerticalAlign"),
            EnumKind::AccumulationRecordType => ("ВидДвиженияНакопления", "AccumulationRecordType"),
            EnumKind::AccountingRecordType => ("ВидДвиженияБухгалтерии", "AccountingRecordType"),
            EnumKind::TextPositionRelativeToPicture => (
                "ПоложениеТекстаОтносительноКартинки",
                "TextPositionRelativeToPicture",
            ),
            EnumKind::HorizontalAlign => ("ГоризонтальноеПоложение", "HorizontalAlign"),
            EnumKind::ColumnsGroup => ("ГруппировкаКолонок", "ColumnsGroup"),
            EnumKind::AllowedMessageNo => ("ДопустимыйНомерСообщения", "AllowedMessageNo"),
            EnumKind::FoldersAndItemsUse => ("ИспользованиеГруппИЭлементов", "FoldersAndItemsUse"),
            EnumKind::DataCompositionParameterUse => (
                "ИспользованиеПараметраКомпоновкиДанных",
                "DataCompositionParameterUse",
            ),
            EnumKind::ScrollBarUse => ("ИспользованиеПолосыПрокрутки", "ScrollBarUse"),
            EnumKind::CurrentRowUse => ("ИспользованиеТекущейСтроки", "CurrentRowUse"),
            EnumKind::QuerySchemaOrderDirection => (
                "НаправлениеПорядкаСхемыЗапроса",
                "QuerySchemaOrderDirection",
            ),
            EnumKind::InitialTreeView => ("НачальноеОтображениеДерева", "InitialTreeView"),
            EnumKind::UpdateOnDataChange => ("ОбновлениеПриИзмененииДанных", "UpdateOnDataChange"),
            EnumKind::EditTextUpdate => ("ОбновлениеТекстаРедактирования", "EditTextUpdate"),
            EnumKind::WarningOnEditRepresentation => (
                "ОтображениеПредупрежденияПриРедактировании",
                "WarningOnEditRepresentation",
            ),
            EnumKind::SearchInTableOnInput => ("ПоискВТаблицеПриВводе", "SearchInTableOnInput"),
            EnumKind::FormCommandBarLabelLocation => (
                "ПоложениеКоманднойПанелиФормы",
                "FormCommandBarLabelLocation",
            ),
            EnumKind::FormItemCommandBarLabelLocation => (
                "ПоложениеКоманднойПанелиЭлементаФормы",
                "FormItemCommandBarLabelLocation",
            ),
            EnumKind::GaugeChartValuesScaleLabelsLocation => (
                "ПоложениеПодписейШкалыЗначенийИзмерительнойДиаграммы",
                "GaugeChartValuesScaleLabelsLocation",
            ),
            EnumKind::ShowInChartLegend => ("ОтображениеВЛегендеДиаграммы", "ShowInChartLegend"),
            EnumKind::AccessTokenSignAlgorithm => {
                ("АлгоритмПодписиТокенаДоступа", "AccessTokenSignAlgorithm")
            }
            EnumKind::ChartPlotAreaPlacement => (
                "РасположениеОбластиПостроенияДиаграммы",
                "ChartPlotAreaPlacement",
            ),
            EnumKind::SpreadsheetDocumentPatternType => (
                "ТипУзораТабличногоДокумента",
                "SpreadsheetDocumentPatternType",
            ),
            EnumKind::ChartLabelLocation => ("ПоложениеПодписейКДиаграмме", "ChartLabelLocation"),
            EnumKind::ChartScaleLabelLocation => {
                ("ПоложениеПодписейШкалыДиаграммы", "ChartScaleLabelLocation")
            }
            EnumKind::SpreadsheetDocumentDetailUse => (
                "ИспользованиеРасшифровкиТабличногоДокумента",
                "SpreadsheetDocumentDetailUse",
            ),
            EnumKind::ViewStatusLocation => ("ПоложениеСостоянияПросмотра", "ViewStatusLocation"),
            EnumKind::SearchStringLocation => ("ПоложениеСтрокиПоиска", "SearchStringLocation"),
            EnumKind::SearchControlLocation => {
                ("ПоложениеУправленияПоиском", "SearchControlLocation")
            }
            EnumKind::AutoTimeMode => ("РежимАвтоВремя", "AutoTimeMode"),
            EnumKind::TableRowInputMode => ("РежимВводаСтрокТаблицы", "TableRowInputMode"),
            EnumKind::CryptoCertificateIncludeMode => (
                "РежимВключенияСертификатовКриптографии",
                "CryptoCertificateIncludeMode",
            ),
            EnumKind::TableRowSelectionMode => {
                ("РежимВыделенияСтрокиТаблицы", "TableRowSelectionMode")
            }
            EnumKind::TableSelectionMode => ("РежимВыделенияТаблицы", "TableSelectionMode"),
            EnumKind::ReplacementMode => ("РежимЗамещения", "ReplacementMode"),
            EnumKind::SizeChangeMode => ("РежимИзмененияРазмера", "SizeChangeMode"),
            EnumKind::FormWindowOpeningMode => ("РежимОткрытияОкнаФормы", "FormWindowOpeningMode"),
            EnumKind::DataCompositionSettingsViewMode => (
                "РежимОтображенияНастроекКомпоновкиДанных",
                "DataCompositionSettingsViewMode",
            ),
            EnumKind::ReportResultViewMode => {
                ("РежимОтображенияРезультатаОтчета", "ReportResultViewMode")
            }
            EnumKind::FullTextSearchMode => ("РежимПолнотекстовогоПоиска", "FullTextSearchMode"),
            EnumKind::CryptoCertificateCheckMode => (
                "РежимПроверкиСертификатаКриптографии",
                "CryptoCertificateCheckMode",
            ),
            EnumKind::ColumnEditMode => ("РежимРедактированияКолонки", "ColumnEditMode"),
            EnumKind::InfoBaseUserPasswordRecoveryMethod => (
                "СпособВосстановленияПароляПользователяИнформационнойБазы",
                "InfoBaseUserPasswordRecoveryMethod",
            ),
            EnumKind::SpreadsheetDocumentAreaFillType => (
                "ТипЗаполненияОбластиТабличногоДокумента",
                "SpreadsheetDocumentAreaFillType",
            ),
            EnumKind::QuerySchemaUnionType => {
                ("ТипОбъединенияСхемыЗапроса", "QuerySchemaUnionType")
            }
            EnumKind::CryptoSignatureType => ("ТипПодписиКриптографии", "CryptoSignatureType"),
            EnumKind::SpreadsheetDocumentTextPlacementType => (
                "ТипРазмещенияТекстаТабличногоДокумента",
                "SpreadsheetDocumentTextPlacementType",
            ),
            EnumKind::QuerySchemaJoinType => ("ТипСоединенияСхемыЗапроса", "QuerySchemaJoinType"),
            EnumKind::ReportFormType => ("ТипФормыОтчета", "ReportFormType"),
            EnumKind::FormattedDocumentItemType => (
                "ТипЭлементаФорматированногоДокумента",
                "FormattedDocumentItemType",
            ),
            EnumKind::FixingInTable => ("ФиксацияВТаблице", "FixingInTable"),
            EnumKind::DataCompositionSortDirection => (
                "НаправлениеСортировкиКомпоновкиДанных",
                "DataCompositionSortDirection",
            ),
            EnumKind::DataCompositionGroupType => {
                ("ТипГруппировкиКомпоновкиДанных", "DataCompositionGroupType")
            }
            EnumKind::DataCompositionGroupPlacement => (
                "РасположениеГруппировкиКомпоновкиДанных",
                "DataCompositionGroupPlacement",
            ),
            EnumKind::DataCompositionGroupFieldsPlacement => (
                "РасположениеПолейГруппировкиКомпоновкиДанных",
                "DataCompositionGroupFieldsPlacement",
            ),
            EnumKind::DataCompositionAttributesPlacement => (
                "РасположениеРеквизитовКомпоновкиДанных",
                "DataCompositionAttributesPlacement",
            ),
            EnumKind::FormDecorationType => ("ВидДекорацииФормы", "FormDecorationType"),
            EnumKind::RadioButtonType => ("ВидПереключателя", "RadioButtonType"),
            EnumKind::ChildFormItemsGroup => (
                "ГруппировкаПодчиненныхЭлементовФормы",
                "ChildFormItemsGroup",
            ),
            EnumKind::ChoiceHistoryOnInput => ("ИсторияВыбораПриВводе", "ChoiceHistoryOnInput"),
            EnumKind::ChoiceButtonRepresentation => {
                ("ОтображениеКнопкиВыбора", "ChoiceButtonRepresentation")
            }
            EnumKind::UsualGroupControlRepresentation => (
                "ОтображениеУправленияОбычнойГруппы",
                "UsualGroupControlRepresentation",
            ),
            EnumKind::UsualGroupBehavior => ("ПоведениеОбычнойГруппы", "UsualGroupBehavior"),
            EnumKind::FormItemTitleLocation => {
                ("ПоложениеЗаголовкаЭлементаФормы", "FormItemTitleLocation")
            }
            EnumKind::ButtonLocationInCommandBar => (
                "ПоложениеКнопкиВКоманднойПанели",
                "ButtonLocationInCommandBar",
            ),
            EnumKind::DataCompositionResourcesPlacement => (
                "РасположениеРесурсовКомпоновкиДанных",
                "DataCompositionResourcesPlacement",
            ),
            EnumKind::CommonAttributeDataSeparation => (
                "РазделениеДанныхОбщегоРеквизита",
                "CommonAttributeDataSeparation",
            ),
            EnumKind::CommonAttributeAutoUse => {
                ("АвтоИспользованиеОбщегоРеквизита", "CommonAttributeAutoUse")
            }
            EnumKind::CommonAttributeUse => ("ИспользованиеОбщегоРеквизита", "CommonAttributeUse"),
            EnumKind::DataCompositionConditionalAppearanceUse => (
                "ИспользованиеУсловногоОформленияКомпоновкиДанных",
                "DataCompositionConditionalAppearanceUse",
            ),
            EnumKind::DataCompositionFieldPlacement => (
                "РасположениеПоляКомпоновкиДанных",
                "DataCompositionFieldPlacement",
            ),
            EnumKind::DataCompositionAccountingBalanceType => (
                "ТипБухгалтерскогоОстаткаКомпоновкиДанных",
                "DataCompositionAccountingBalanceType",
            ),
            EnumKind::DataCompositionBalanceType => {
                ("ТипОстаткаКомпоновкиДанных", "DataCompositionBalanceType")
            }
            EnumKind::DataCompositionPeriodType => {
                ("ТипПериодаКомпоновкиДанных", "DataCompositionPeriodType")
            }
            EnumKind::DataCompositionPeriodAdditionType => (
                "ТипДополненияПериодаКомпоновкиДанных",
                "DataCompositionPeriodAdditionType",
            ),
            EnumKind::ColorDepth => ("ГлубинаЦвета", "ColorDepth"),
            EnumKind::ToolTipRepresentation => ("ОтображениеПодсказки", "ToolTipRepresentation"),
            EnumKind::ClientApplicationType => {
                ("ТипКлиентскогоПриложения", "ClientApplicationType")
            }
            EnumKind::ClientConnectionSpeed => {
                ("СкоростьКлиентскогоСоединения", "ClientConnectionSpeed")
            }
            EnumKind::SpreadsheetDocumentCellAreaType => (
                "ТипОбластиЯчеекТабличногоДокумента",
                "SpreadsheetDocumentCellAreaType",
            ),
            EnumKind::AddInConnectionType => {
                ("ТипПодключенияВнешнейКомпоненты", "AddInConnectionType")
            }
            EnumKind::FillChecking => ("ПроверкаЗаполнения", "FillChecking"),
            EnumKind::FoldersAndItems => ("ГруппыИЭлементы", "FoldersAndItems"),
            EnumKind::XmlCanonicalizationType => ("ТипКаноническогоXML", "XMLCanonicalizationType"),
            EnumKind::Pop3AuthenticationMode => {
                ("СпособPOP3Аутентификации", "POP3AuthenticationMode")
            }
            EnumKind::SmtpAuthenticationMode => {
                ("СпособSMTPАутентификации", "SMTPAuthenticationMode")
            }
            EnumKind::InternetMailProtocol => ("ПротоколИнтернетПочты", "InternetMailProtocol"),
            EnumKind::DataCompositionDetailsProcessingAction => (
                "ДействиеОбработкиРасшифровкиКомпоновкиДанных",
                "DataCompositionDetailsProcessingAction",
            ),
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
            EnumKind::ArchiveFileRestorePathsMode => (
                "РежимВосстановленияПутейФайлаАрхива",
                "ArchiveFileRestoreFilePathsMode",
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
            EnumKind::FormattedDocumentFileType => (
                "ТипФайлаФорматированногоДокумента",
                "FormattedDocumentFileType",
            ),
            EnumKind::ByteOrderMarkUse => ("ИспользованиеByteOrderMark", "ByteOrderMarkUse"),
        }
    }

    /// Каноническое английское имя системного перечисления.
    #[must_use]
    pub const fn english_name(self) -> &'static str {
        self.names().1
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
            // Имена типов измерены пробой `measure-client-settings.bsl`.
            EnumKind::ApplicationFormsOpenningMode => "ApplicationFormsOpenningMode",
            EnumKind::ClientApplicationInterfaceVariant => "ClientApplicationInterfaceVariant",
            EnumKind::SectionsPanelRepresentation => "SectionsPanelRepresentation",
            // ИЗМЕРЕНО на 8.3.27 пробой `measure-configuration-extension-enums.bsl`.
            EnumKind::ConfigurationExtensionsSource => "EnumConfigurationExtensionsSource",
            EnumKind::ConfigurationExtensionScope => "EnumConfigurationExtensionScope",
            EnumKind::ConfigurationExtensionPurpose => "MetadataObjectsPropertyEnumed",
            EnumKind::ConfigurationExtensionApplicationIssueSeverity => {
                "EnumConfigurationExtensionApplicationIssueSeverity"
            }
            // ИЗМЕРЕНО на 8.3.27 пробой `measure-event-log.bsl`.
            EnumKind::EventLogLevel => "EnumEventLogLevel",
            EnumKind::EventLogEntryTransactionMode => "EnumEventLogEntryTransactionMode",
            EnumKind::EventLogEntryTransactionStatus => "EventLogEntryTransactionStatus",
            EnumKind::ErrorReportingMode => "ErrorReportingMode",
            EnumKind::DataItemSend => "DataItemSend",
            EnumKind::DataItemReceive => "DataItemReceive",
            EnumKind::PredefinedDataUpdate => "PredefinedDataUpdate",
            EnumKind::DocumentPostingMode => "DocumentPostingMode",
            EnumKind::DocumentWriteMode => "DocumentWriteMode",
            // ИЗМЕРЕНО на 8.3.27 пробой `measure-query-result-iteration.bsl`.
            EnumKind::QueryResultIteration => "QueryResultIteration",
            EnumKind::DataCompositionDataSetsLinkType => "EnumDataCompositionDataSetsLinkType",
            EnumKind::ChartValuesBySeriesConnectionType => "EnumChartValuesBySeriesConnectionType",
            EnumKind::DataCompositionChartLegendPlacement => {
                "EnumDataCompositionChartLegendPlacement"
            }
            EnumKind::QueryRecordType => "EnumQueryRecordType",
            // ИЗМЕРЕНО на 8.3.27 пробой `measure-data-lock-mode.bsl`.
            EnumKind::DataLockMode => "DataLockMode",
            // Представление типа измерено пробой `measure-account-type.bsl`.
            EnumKind::AccountType => "AccountType",
            EnumKind::RoundMode => "RoundMode",
            EnumKind::NumericValueType => "NumericValueType",
            EnumKind::ErrorMessageDisplayVariant => "ErrorMessageDisplayVariant",
            EnumKind::CommandParameterUseMode => "CommandParameterUseMode",
            EnumKind::SearchStringModeOnInputByString => "SearchStringModeOnInputByString",
            EnumKind::RepresentableDocumentBatchFileType => "RepresentableDocumentBatchFileType",
            EnumKind::FontType => "FontType",
            EnumKind::PictureType => "PictureType",
            EnumKind::FullTextSearchMetadataUse => "FullTextSearchMetadataUse",
            EnumKind::LinkedValueChangeMode => "LinkedValueChangeMode",
            EnumKind::InternetMailAttachmentEncodingMode => "InternetMailAttachmentEncodingMode",
            EnumKind::AutoSaveFormDataInSettings => "AutoSaveFormDataInSettings",
            EnumKind::PDFDocumentFileType => "PDFDocumentFileType",
            EnumKind::AllowedSign => "AllowedSign",
            EnumKind::AutoChangeRecord => "AutoChangeRecord",
            EnumKind::ButtonRepresentation => "ButtonRepresentation",
            // ИЗМЕРЕНО на 8.3.27 пробой
            // `measure-password-policy-compliance-check-result.bsl`.
            EnumKind::PasswordPolicyComplianceCheckResult => "PasswordPolicyComplianceCheckResult",
            // ИЗМЕРЕНО на 8.3.27 пробой `measure-table-representation.bsl`.
            EnumKind::TableRepresentation => "TableRepresentation",
            // ИЗМЕРЕНО на 8.3.27 пробой `measure-form-field-type.bsl`.
            EnumKind::FormFieldType => "FormFieldType",
            // ИЗМЕРЕНО на 8.3.27 пробой `measure-form-group-type.bsl`.
            EnumKind::FormGroupType => "FormGroupType",
            // ИЗМЕРЕНО на 8.3.27 пробой `measure-item-horizontal-location.bsl`.
            EnumKind::ItemHorizontalLocation => "ItemHorizontalLocation",
            // ИЗМЕРЕНО на 8.3.27 пробой `measure-form-alignment-spacing-enums.bsl`.
            EnumKind::ItemVerticalAlign => "EnumItemVerticalAlign",
            EnumKind::FormItemSpacing => "EnumFormItemSpacing",
            // ИЗМЕРЕНО на 8.3.27 пробой `measure-button-shape-representation.bsl`.
            EnumKind::ButtonShapeRepresentation => "ButtonShapeRepresentation",
            // ИЗМЕРЕНО на 8.3.27 пробой `measure-usual-group-representation.bsl`.
            EnumKind::UsualGroupRepresentation => "UsualGroupRepresentation",
            // ИЗМЕРЕНО на 8.3.27 пробой `measure-display-importance.bsl`.
            EnumKind::DisplayImportance => "DisplayImportance",
            // ИЗМЕРЕНО на 8.3.27 пробой `measure-picture-format.bsl`.
            EnumKind::PictureFormat => "PictureFormat",
            // ИЗМЕРЕНО на 8.3.27 пробой `measure-internet-mail-text-type.bsl`.
            EnumKind::InternetMailTextType => "InternetMailTextType",
            // ИЗМЕРЕНО на 8.3.27 пробой `measure-mail-non-ascii-encoding-mode.bsl`.
            EnumKind::InternetMailNonAsciiEncodingMode => {
                "InternetMailMessageNonASCIISymbolsEncodingMode"
            }
            // ИЗМЕРЕНО на 8.3.27 пробой `measure-mail-message-importance.bsl`.
            EnumKind::InternetMailMessageImportance => "InternetMailMessageImportance",
            // ИЗМЕРЕНО на 8.3.27 пробой `measure-additional-show-mode.bsl`.
            EnumKind::AdditionalShowMode => "AdditionalShowMode",
            // ИЗМЕРЕНО на 8.3.27 пробой `measure-tls-revocation-mode.bsl`.
            EnumKind::ServerTlsCertificateRevocationCheckMode => {
                "ServerTLSCertificateRevocationCheckMode"
            }
            // ИЗМЕРЕНО на 8.3.27 пробой
            // `measure-data-composition-filter-application-type.bsl`.
            EnumKind::DataCompositionFilterApplicationType => {
                "DataCompositionFilterApplicationType"
            }
            // ИЗМЕРЕНО на 8.3.27 пробой
            // `measure-data-composition-settings-item-view-mode.bsl`.
            EnumKind::DataCompositionSettingsItemState => "EnumDataCompositionSettingsItemState",
            EnumKind::DataCompositionSettingsItemViewMode => "DataCompositionSettingsItemViewMode",
            // ИЗМЕРЕНО на 8.3.27 пробой
            // `measure-data-composition-settings-refresh-method.bsl`.
            EnumKind::DataCompositionSettingsRefreshMethod => {
                "EnumDataCompositionSettingsRefreshMethod"
            }
            // ИЗМЕРЕНО на 8.3.27 пробой
            // `measure-data-composition-text-output-type.bsl`.
            EnumKind::DataCompositionTextPlacementType => "EnumDataCompositionTextPlacementType",
            EnumKind::DataCompositionTextOutputType => "EnumDataCompositionTextOutputType",
            // ИЗМЕРЕНО на 8.3.27 пробой `measure-view-scaling-mode.bsl`.
            EnumKind::ViewScalingMode => "EnumViewScalingMode",
            // ИЗМЕРЕНО на 8.3.27 пробой
            // `measure-data-composition-total-placement.bsl`.
            EnumKind::DataCompositionTotalPlacement => "EnumDataCompositionTotalPlacement",
            // ИЗМЕРЕНО на 8.3.27 пробой `measure-picture-size.bsl`.
            EnumKind::PictureSize => "EnumPictureSize",
            // ИЗМЕРЕНО на 8.3.27 пробой
            // `measure-spreadsheet-document-shift-type.bsl`.
            EnumKind::SpreadsheetDocumentShiftType => "EnumSpreadsheetDocumentShiftType",
            // ИЗМЕРЕНО на 8.3.27 пробой `measure-chart-type.bsl`.
            EnumKind::ChartType => "EnumChartType",
            // ИЗМЕРЕНО на 8.3.27 пробой
            // `measure-chart-label-orientation-name.bsl`.
            EnumKind::ChartLabelsOrientation => "EnumChartLabelsOrientation",
            // ИЗМЕРЕНО на 8.3.27 пробой `measure-boundary-type.bsl`.
            EnumKind::BoundaryType => "EnumBoundaryType",
            // ИЗМЕРЕНО на 8.3.27 пробой `measure-max-series.bsl`.
            EnumKind::MaxSeries => "EnumMaxSeries",
            // ИЗМЕРЕНО на 8.3.27 пробой `measure-chart-label-type.bsl`.
            EnumKind::ChartLabelType => "EnumChartLabelType",
            // ИЗМЕРЕНО на 8.3.27 пробой `measure-chart-line-type.bsl`.
            EnumKind::ChartLineType => "EnumChartLineType",
            // ИЗМЕРЕНО на 8.3.27 пробой `measure-form-button-type.bsl`.
            EnumKind::FormButtonType => "EnumFormButtonType",
            EnumKind::XmlTypeAssignment => "XMLTypeAssignment",
            EnumKind::PlatformType => "PlatformType",
            EnumKind::DataCompositionComparisonType => "DataCompositionComparisonType",
            EnumKind::ComparisonType => "EnumComparisonType",
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
            // Представление корня измерено на платформе 8.3.27.2342.
            EnumKind::TimeScaleDayFormat => "EnumTimeScaleDayFormat",
            // Представление корня измерено на платформе 8.3.27.2342.
            EnumKind::TimeScaleUnitType => "EnumTimeScaleUnitType",
            // Представление корня измерено на платформе 8.3.27.2342.
            EnumKind::GanttChartIntervalRepresentation => "EnumGanttChartIntervalRepresentation",
            // Представление корня измерено на платформе 8.3.27.2342.
            EnumKind::GanttChartVerticalStretch => "EnumGanttChartVerticalStretch",
            // Представление корня измерено на платформе 8.3.27.2342.
            EnumKind::ChartLegendPlacement => "EnumChartLegendPlacement",
            // ИЗМЕРЕНО, а не достроено по образцу: `Строка(ТипЗнч(
            // ОриентацияСтраницы))` на 8.3.27 даёт «ПеречислениеОриентацияСтраницы».
            EnumKind::PageOrientation => "ПеречислениеОриентацияСтраницы",
            EnumKind::TextEncoding => "ПеречислениеКодировкаТекста",
            EnumKind::StringEncodingMethod => "ПеречислениеСпособКодированияСтроки",
            EnumKind::SortDirection => "ПеречислениеНаправлениеСортировки",
            EnumKind::UseOutput => "UseOutput",
            EnumKind::FormPagesRepresentation => "FormPagesRepresentation",
            EnumKind::GraphicalSchemaShapes => "ПеречислениеФигурыГрафическойСхемы",
            EnumKind::ConnectorLineType => "ПеречислениеТипСоединительнойЛинии",
            EnumKind::ConnectorTextLocation => "ПеречислениеПоложениеТекстаСоединительнойЛинии",
            EnumKind::ArrowStyle => "ПеречислениеСтильСтрелки",
            EnumKind::GraphicalSchemaItemPictureLocation => {
                "ПеречислениеПоложениеКартинкиЭлементаГрафическойСхемы"
            }
            EnumKind::FormButtonPictureLocation => "FormButtonPictureLocation",
            EnumKind::SpreadsheetDocumentDrawingLineType => "SpreadsheetDocumentDrawingLineType",
            EnumKind::CheckBoxType => "CheckBoxType",
            EnumKind::DeliverableNotificationSubscriberType => {
                // Голое имя и его тип измерены отдельно от типа члена перечисления.
                "EnumDeliverableNotificationSubscriberType"
            }
            EnumKind::SoundAlert => "SoundAlert",
            EnumKind::ThroughAlign => "ThroughAlign",
            EnumKind::ButtonShape => "ButtonShape",
            EnumKind::DataCompositionFilterItemsGroupType => "DataCompositionFilterItemsGroupType",
            EnumKind::Key => "Key",
            EnumKind::ControlBorderType => "ControlBorderType",
            EnumKind::ColorType => "ColorType",
            EnumKind::SpreadsheetDocumentCellLineType => "SpreadsheetDocumentCellLineType",
            EnumKind::CryptoCertificateStoreType => "CryptoCertificateStoreType",
            EnumKind::InternetMailTextProcessing => "InternetMailTextProcessing",
            EnumKind::DuplexPrintingType => "DuplexPrintingType",
            EnumKind::SecondAuthenticationFactorSettingsProcessingType => {
                "SecondAuthenticationFactorSettingsProcessingType"
            }
            EnumKind::FileDragMode => "FileDragMode",
            EnumKind::ButtonGroupRepresentation => "ButtonGroupRepresentation",
            EnumKind::StandardPeriodVariant => "StandardPeriodVariant",
            EnumKind::TableHeightControlVariant => "TableHeightControlVariant",
            EnumKind::FullTextSearchVersion => "FullTextSearchVersion",
            EnumKind::VerticalFormScroll => "VerticalFormScroll",
            EnumKind::VerticalAlign => "VerticalAlign",
            EnumKind::AccumulationRecordType => "AccumulationRecordType",
            // Нативная проба подтверждает тип самого перечисления.
            EnumKind::AccountingRecordType => "EnumAccountingRecordType",
            EnumKind::TextPositionRelativeToPicture => "EnumTextPositionRelativeToPicture",
            EnumKind::HorizontalAlign => "HorizontalAlign",
            EnumKind::ColumnsGroup => "ColumnsGroup",
            EnumKind::AllowedMessageNo => "AllowedMessageNo",
            EnumKind::FoldersAndItemsUse => "FoldersAndItemsUse",
            EnumKind::DataCompositionParameterUse => "DataCompositionParameterUse",
            EnumKind::ScrollBarUse => "ScrollBarUse",
            EnumKind::CurrentRowUse => "CurrentRowUse",
            EnumKind::QuerySchemaOrderDirection => "QuerySchemaOrderDirection",
            EnumKind::InitialTreeView => "InitialTreeView",
            EnumKind::UpdateOnDataChange => "UpdateOnDataChange",
            EnumKind::EditTextUpdate => "EditTextUpdate",
            EnumKind::WarningOnEditRepresentation => "WarningOnEditRepresentation",
            EnumKind::SearchInTableOnInput => "SearchInTableOnInput",
            EnumKind::FormCommandBarLabelLocation => "FormCommandBarLabelLocation",
            EnumKind::FormItemCommandBarLabelLocation => "FormItemCommandBarLabelLocation",
            EnumKind::GaugeChartValuesScaleLabelsLocation => "GaugeChartValuesScaleLabelsLocation",
            EnumKind::ShowInChartLegend => "ShowInChartLegend",
            EnumKind::AccessTokenSignAlgorithm => "EnumAccessTokenSignAlgorithm",
            EnumKind::ChartPlotAreaPlacement => "EnumChartPlotAreaPlacement",
            EnumKind::SpreadsheetDocumentPatternType => "EnumSpreadsheetDocumentPatternType",
            EnumKind::ChartLabelLocation => "ChartLabelLocation",
            EnumKind::ChartScaleLabelLocation => "ChartScaleLabelLocation",
            EnumKind::SpreadsheetDocumentDetailUse => "SpreadsheetDocumentDetailUse",
            EnumKind::ViewStatusLocation => "ViewStatusLocation",
            EnumKind::SearchStringLocation => "SearchStringLocation",
            EnumKind::SearchControlLocation => "SearchControlLocation",
            EnumKind::AutoTimeMode => "AutoTimeMode",
            EnumKind::TableRowInputMode => "TableRowInputMode",
            EnumKind::CryptoCertificateIncludeMode => "CryptoCertificateIncludeMode",
            EnumKind::TableRowSelectionMode => "TableRowSelectionMode",
            EnumKind::TableSelectionMode => "TableSelectionMode",
            EnumKind::ReplacementMode => "ReplacementMode",
            EnumKind::SizeChangeMode => "SizeChangeMode",
            EnumKind::FormWindowOpeningMode => "FormWindowOpeningMode",
            EnumKind::DataCompositionSettingsViewMode => "DataCompositionSettingsViewMode",
            EnumKind::ReportResultViewMode => "ReportResultViewMode",
            EnumKind::FullTextSearchMode => "FullTextSearchMode",
            EnumKind::CryptoCertificateCheckMode => "CryptoCertificateCheckMode",
            EnumKind::ColumnEditMode => "ColumnEditMode",
            EnumKind::InfoBaseUserPasswordRecoveryMethod => "InfoBaseUserPasswordRecoveryMethod",
            EnumKind::SpreadsheetDocumentAreaFillType => "SpreadsheetDocumentAreaFillType",
            EnumKind::QuerySchemaUnionType => "QuerySchemaUnionType",
            EnumKind::CryptoSignatureType => "CryptoSignatureType",
            EnumKind::SpreadsheetDocumentTextPlacementType => {
                "SpreadsheetDocumentTextPlacementType"
            }
            EnumKind::QuerySchemaJoinType => "QuerySchemaJoinType",
            EnumKind::ReportFormType => "ReportFormType",
            EnumKind::FormattedDocumentItemType => "FormattedDocumentItemType",
            EnumKind::FixingInTable => "FixingInTable",
            EnumKind::DataCompositionSortDirection => "EnumDataCompositionSortDirection",
            EnumKind::DataCompositionGroupType => "EnumDataCompositionGroupType",
            EnumKind::DataCompositionGroupPlacement => "EnumDataCompositionGroupPlacement",
            EnumKind::DataCompositionGroupFieldsPlacement => {
                "EnumDataCompositionGroupFieldsPlacement"
            }
            EnumKind::DataCompositionAttributesPlacement => {
                "EnumDataCompositionAttributesPlacement"
            }
            EnumKind::FormDecorationType => "EnumFormDecorationType",
            EnumKind::RadioButtonType => "EnumRadioButtonType",
            EnumKind::ChildFormItemsGroup => "EnumChildFormItemsGroup",
            EnumKind::ChoiceHistoryOnInput => "EnumChoiceHistoryOnInput",
            EnumKind::ChoiceButtonRepresentation => "EnumChoiceButtonRepresentation",
            EnumKind::UsualGroupControlRepresentation => "EnumUsualGroupControlRepresentation",
            EnumKind::UsualGroupBehavior => "EnumUsualGroupBehavior",
            EnumKind::FormItemTitleLocation => "EnumFormItemTitleLocation",
            EnumKind::ButtonLocationInCommandBar => "EnumButtonLocationInCommandBar",
            EnumKind::DataCompositionResourcesPlacement => "EnumDataCompositionResourcesPlacement",
            EnumKind::CommonAttributeDataSeparation => "EnumCommonAttributeDataSeparation",
            EnumKind::CommonAttributeAutoUse => "EnumCommonAttributeAutoUse",
            EnumKind::CommonAttributeUse => "EnumCommonAttributeUse",
            EnumKind::DataCompositionConditionalAppearanceUse => {
                "EnumDataCompositionConditionalAppearanceUse"
            }
            EnumKind::DataCompositionFieldPlacement => "EnumDataCompositionFieldPlacement",
            EnumKind::DataCompositionAccountingBalanceType => {
                "EnumDataCompositionAccountingBalanceType"
            }
            EnumKind::DataCompositionBalanceType => "EnumDataCompositionBalanceType",
            EnumKind::DataCompositionPeriodType => "EnumDataCompositionPeriodType",
            EnumKind::DataCompositionPeriodAdditionType => "EnumDataCompositionPeriodAdditionType",
            EnumKind::ColorDepth => "EnumColorDepth",
            EnumKind::ToolTipRepresentation => "EnumToolTipRepresentation",
            EnumKind::ClientApplicationType => "EnumClientApplicationType",
            EnumKind::ClientConnectionSpeed => "EnumClientConnectionSpeed",
            EnumKind::SpreadsheetDocumentCellAreaType => "EnumSpreadsheetDocumentCellAreaType",
            // НЕ ИЗМЕРЕНО(ENUM.ADDIN_CONNECTION_TYPE.PRESENTATION):
            // перечисление доступно только в толстом клиенте.
            EnumKind::AddInConnectionType => "EnumAddInConnectionType",
            EnumKind::FillChecking => "EnumFillChecking",
            EnumKind::FoldersAndItems => "EnumFoldersAndItems",
            EnumKind::XmlCanonicalizationType => "EnumXMLCanonicalizationType",
            EnumKind::Pop3AuthenticationMode => "EnumPOP3AuthenticationMode",
            EnumKind::SmtpAuthenticationMode => "EnumSMTPAuthenticationMode",
            EnumKind::InternetMailProtocol => "EnumInternetMailProtocol",
            EnumKind::DataCompositionDetailsProcessingAction => {
                "EnumDataCompositionDetailsProcessingAction"
            }
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
            // Английское представление измерено и при русском интерфейсе.
            EnumKind::ArchiveFileRestorePathsMode => "EnumArchiveFileRestoreFilePathsMode",
            EnumKind::PdfAttachmentRelation => "ПеречислениеТипСвязиВложенияPDF",
            EnumKind::ArchiveFileType => "ПеречислениеТипФайлаАрхива",
            EnumKind::ZipCompressionMethod => "ПеречислениеМетодСжатияZIP",
            EnumKind::ZipCompressionLevel => "ПеречислениеУровеньСжатияZIP",
            EnumKind::ZipStorePathMode => "ПеречислениеРежимСохраненияПутейZIP",
            EnumKind::ZipSubDirProcessingMode => "ПеречислениеРежимОбработкиПодкаталоговZIP",
            EnumKind::ZipEncryptionMethod => "ПеречислениеМетодШифрованияZIP",
            EnumKind::ZipFileNamesEncoding => "ПеречислениеКодировкаИменФайловВZipФайле",
            EnumKind::FormattedDocumentFileType => "EnumFormattedDocumentFileType",
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
        EnumKind::AccessTokenSignAlgorithm,
        EnumValue::AccessTokenSignAlgorithmHs256,
        "HS256",
        "HS256",
        "HS256",
    ),
    (
        EnumKind::ChartPlotAreaPlacement,
        EnumValue::ChartPlotAreaPlacementUseCoordinates,
        "УказываетсяРасположение",
        "UseCoordinates",
        "Use coordinates",
    ),
    (
        EnumKind::SpreadsheetDocumentPatternType,
        EnumValue::SpreadsheetDocumentPatternTypeWithoutPattern,
        "БезУзора",
        "WithoutPattern",
        "No pattern",
    ),
    // Полные представления сняты пробами `graphical-schema/shapes.bsl` и `line-enums.bsl`.
    (
        EnumKind::GraphicalSchemaShapes,
        EnumValue::GraphicalSchemaShapesBlock,
        "Блок",
        "Block",
        "Блок",
    ),
    (
        EnumKind::GraphicalSchemaShapes,
        EnumValue::GraphicalSchemaShapesDocument,
        "Документ",
        "Document",
        "Документ",
    ),
    (
        EnumKind::GraphicalSchemaShapes,
        EnumValue::GraphicalSchemaShapesNone,
        "Нет",
        "None",
        "Нет",
    ),
    (
        EnumKind::GraphicalSchemaShapes,
        EnumValue::GraphicalSchemaShapesFolder,
        "Папка",
        "Folder",
        "Папка",
    ),
    (
        EnumKind::GraphicalSchemaShapes,
        EnumValue::GraphicalSchemaShapesVerticalBrackets,
        "СкобкиВертикальные",
        "VerticalBrackets",
        "Скобки вертикальные",
    ),
    (
        EnumKind::GraphicalSchemaShapes,
        EnumValue::GraphicalSchemaShapesHorizontalBrackets,
        "СкобкиГоризонтальные",
        "HorizontalBrackets",
        "Скобки горизонтальные",
    ),
    (
        EnumKind::GraphicalSchemaShapes,
        EnumValue::GraphicalSchemaShapesUpArrow,
        "СтрелкаВверх",
        "UpArrow",
        "Стрелка вверх",
    ),
    (
        EnumKind::GraphicalSchemaShapes,
        EnumValue::GraphicalSchemaShapesUpDownArrow,
        "СтрелкаВверхВниз",
        "UpDownArrow",
        "Стрелка вверх-вниз",
    ),
    (
        EnumKind::GraphicalSchemaShapes,
        EnumValue::GraphicalSchemaShapesLeftArrow,
        "СтрелкаВлево",
        "LeftArrow",
        "Стрелка влево",
    ),
    (
        EnumKind::GraphicalSchemaShapes,
        EnumValue::GraphicalSchemaShapesLeftRightArrow,
        "СтрелкаВлевоВправо",
        "LeftRightArrow",
        "Стрелка влево-вправо",
    ),
    (
        EnumKind::GraphicalSchemaShapes,
        EnumValue::GraphicalSchemaShapesDownArrow,
        "СтрелкаВниз",
        "DownArrow",
        "Стрелка вниз",
    ),
    (
        EnumKind::GraphicalSchemaShapes,
        EnumValue::GraphicalSchemaShapesRightArrow,
        "СтрелкаВправо",
        "RightArrow",
        "Стрелка вправо",
    ),
    (
        EnumKind::GraphicalSchemaShapes,
        EnumValue::GraphicalSchemaShapesFile,
        "Файл",
        "File",
        "Файл",
    ),
    (
        EnumKind::GraphicalSchemaShapes,
        EnumValue::GraphicalSchemaShapesEllipse,
        "Эллипс",
        "Ellipse",
        "Эллипс",
    ),
    (
        EnumKind::ConnectorLineType,
        EnumValue::ConnectorLineTypeNone,
        "НетЛинии",
        "None",
        "Нет линии",
    ),
    (
        EnumKind::ConnectorLineType,
        EnumValue::ConnectorLineTypeDashed,
        "Пунктир",
        "Dashed",
        "Пунктирная",
    ),
    (
        EnumKind::ConnectorLineType,
        EnumValue::ConnectorLineTypeDashDotted,
        "ПунктирТочка",
        "DashDotted",
        "Пунктирно-точечная",
    ),
    (
        EnumKind::ConnectorLineType,
        EnumValue::ConnectorLineTypeDashDottedDotted,
        "ПунктирТочкаТочка",
        "DashDottedDotted",
        "Точечно-пунктирно-точечная",
    ),
    (
        EnumKind::ConnectorLineType,
        EnumValue::ConnectorLineTypeSolid,
        "Сплошная",
        "Solid",
        "Сплошная",
    ),
    (
        EnumKind::ConnectorLineType,
        EnumValue::ConnectorLineTypeDotted,
        "Точечная",
        "Dotted",
        "Точечная",
    ),
    (
        EnumKind::ConnectorTextLocation,
        EnumValue::ConnectorTextLocationFirstSegment,
        "ПервыйСегмент",
        "FirstSegment",
        "Первый сегмент",
    ),
    (
        EnumKind::ConnectorTextLocation,
        EnumValue::ConnectorTextLocationMiddle,
        "СерединаЛинии",
        "Middle",
        "Середина линии",
    ),
    (
        EnumKind::ArrowStyle,
        EnumValue::ArrowStyleFilled,
        "Заполненная",
        "Filled",
        "Заполненная",
    ),
    (
        EnumKind::ArrowStyle,
        EnumValue::ArrowStyleBlank,
        "Незаполненная",
        "Blank",
        "Незаполненная",
    ),
    (
        EnumKind::ArrowStyle,
        EnumValue::ArrowStyleNone,
        "Нет",
        "None",
        "Нет",
    ),
    // Представления и псевдонимы сняты пробой `graphical-schema/picture-location.bsl`.
    (
        EnumKind::GraphicalSchemaItemPictureLocation,
        EnumValue::GraphicalSchemaItemPictureLocationTop,
        "Верх",
        "Top",
        "Верх",
    ),
    (
        EnumKind::GraphicalSchemaItemPictureLocation,
        EnumValue::GraphicalSchemaItemPictureLocationLeft,
        "Лево",
        "Left",
        "Лево",
    ),
    (
        EnumKind::GraphicalSchemaItemPictureLocation,
        EnumValue::GraphicalSchemaItemPictureLocationBottom,
        "Низ",
        "Bottom",
        "Низ",
    ),
    (
        EnumKind::GraphicalSchemaItemPictureLocation,
        EnumValue::GraphicalSchemaItemPictureLocationRight,
        "Право",
        "Right",
        "Право",
    ),
    (
        EnumKind::GraphicalSchemaItemPictureLocation,
        EnumValue::GraphicalSchemaItemPictureLocationCenter,
        "Центр",
        "Center",
        "Центр",
    ),
    // Полный состав, псевдонимы и представления измерены на 8.3.27.2342
    // пробой `measure-account-type.bsl`; текст не выводится из имени члена.
    (
        EnumKind::AccountType,
        EnumValue::AccountTypeActive,
        "Активный",
        "Active",
        "Assets",
    ),
    (
        EnumKind::AccountType,
        EnumValue::AccountTypePassive,
        "Пассивный",
        "Passive",
        "Liabilities",
    ),
    (
        EnumKind::AccountType,
        EnumValue::AccountTypeActivePassive,
        "АктивноПассивный",
        "ActivePassive",
        "Assets/Liabilities",
    ),
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
    // Полный состав и представления измерены на 8.3.27.2342.
    (
        EnumKind::EventLogEntryTransactionStatus,
        EnumValue::EventLogEntryTransactionStatusCommitted,
        "Зафиксирована",
        "Committed",
        "Committed",
    ),
    (
        EnumKind::EventLogEntryTransactionStatus,
        EnumValue::EventLogEntryTransactionStatusUnfinished,
        "НеЗавершена",
        "Unfinished",
        "Not completed",
    ),
    (
        EnumKind::EventLogEntryTransactionStatus,
        EnumValue::EventLogEntryTransactionStatusNotApplicable,
        "НетТранзакции",
        "NotApplicable",
        "No transaction",
    ),
    (
        EnumKind::EventLogEntryTransactionStatus,
        EnumValue::EventLogEntryTransactionStatusRolledBack,
        "Отменена",
        "RolledBack",
        "Rolled back",
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
    (
        EnumKind::DocumentWriteMode,
        EnumValue::DocumentWriteModeWrite,
        "Запись",
        "Write",
        "Record",
    ),
    (
        EnumKind::DocumentWriteMode,
        EnumValue::DocumentWriteModePosting,
        "Проведение",
        "Posting",
        "Posting",
    ),
    (
        EnumKind::DocumentWriteMode,
        EnumValue::DocumentWriteModeUndoPosting,
        "ОтменаПроведения",
        "UndoPosting",
        "Cancel posting",
    ),
    // Полный состав, оба написания, представления и тип измерены на
    // платформе 8.3.27 пробой `measure-query-result-iteration.bsl`.
    (
        EnumKind::QueryResultIteration,
        EnumValue::QueryResultIterationLinear,
        "Прямой",
        "Linear",
        "Line",
    ),
    (
        EnumKind::QueryResultIteration,
        EnumValue::QueryResultIterationByGroups,
        "ПоГруппировкам",
        "ByGroups",
        "By group",
    ),
    (
        EnumKind::QueryResultIteration,
        EnumValue::QueryResultIterationByGroupsWithHierarchy,
        "ПоГруппировкамСИерархией",
        "ByGroupsWithHierarchy",
        "By group with hierarchy",
    ),
    // Члены, алиасы, типы и представления измерены на 8.3.27.2342.
    (
        EnumKind::DataCompositionDataSetsLinkType,
        EnumValue::DataCompositionDataSetsLinkTypeOuter,
        "Внешняя",
        "Outer",
        "External",
    ),
    (
        EnumKind::DataCompositionDataSetsLinkType,
        EnumValue::DataCompositionDataSetsLinkTypeInner,
        "Внутренняя",
        "Inner",
        "Internal",
    ),
    (
        EnumKind::ChartValuesBySeriesConnectionType,
        EnumValue::ChartValuesBySeriesConnectionTypeNone,
        "Нет",
        "None",
        "No",
    ),
    (
        EnumKind::ChartValuesBySeriesConnectionType,
        EnumValue::ChartValuesBySeriesConnectionTypeEdgesConnection,
        "СоединениеКраев",
        "EdgesConnection",
        "Edge joint",
    ),
    (
        EnumKind::DataCompositionChartLegendPlacement,
        EnumValue::DataCompositionChartLegendPlacementNone,
        "Нет",
        "None",
        "None",
    ),
    (
        EnumKind::DataCompositionChartLegendPlacement,
        EnumValue::DataCompositionChartLegendPlacementLeft,
        "Лево",
        "Left",
        "Left",
    ),
    (
        EnumKind::DataCompositionChartLegendPlacement,
        EnumValue::DataCompositionChartLegendPlacementRight,
        "Право",
        "Right",
        "Right",
    ),
    (
        EnumKind::DataCompositionChartLegendPlacement,
        EnumValue::DataCompositionChartLegendPlacementTop,
        "Верх",
        "Top",
        "Top",
    ),
    (
        EnumKind::DataCompositionChartLegendPlacement,
        EnumValue::DataCompositionChartLegendPlacementBottom,
        "Низ",
        "Bottom",
        "Bottom",
    ),
    (
        EnumKind::QueryRecordType,
        EnumValue::QueryRecordTypeDetailRecord,
        "ДетальнаяЗапись",
        "DetailRecord",
        "Detailed record",
    ),
    (
        EnumKind::QueryRecordType,
        EnumValue::QueryRecordTypeGroupTotal,
        "ИтогПоГруппировке",
        "GroupTotal",
        "Total by group",
    ),
    (
        EnumKind::QueryRecordType,
        EnumValue::QueryRecordTypeTotalByHierarchy,
        "ИтогПоИерархии",
        "TotalByHierarchy",
        "Total by hierarchy",
    ),
    (
        EnumKind::QueryRecordType,
        EnumValue::QueryRecordTypeOverall,
        "ОбщийИтог",
        "Overall",
        "Grand total",
    ),
    // Полный состав, оба написания, представления и тип измерены на
    // платформе 8.3.27 пробой `measure-data-lock-mode.bsl`.
    (
        EnumKind::DataLockMode,
        EnumValue::DataLockModeShared,
        "Разделяемый",
        "Shared",
        "Shared",
    ),
    (
        EnumKind::DataLockMode,
        EnumValue::DataLockModeExclusive,
        "Исключительный",
        "Exclusive",
        "Exceptional",
    ),
    // Полный состав и представления измерены пробой `measure-round-mode.bsl`.
    (
        EnumKind::RoundMode,
        EnumValue::RoundModeRound15as10,
        "Окр15как10",
        "Round15as10",
        "Round 15 to 10",
    ),
    (
        EnumKind::RoundMode,
        EnumValue::RoundModeRound15as20,
        "Окр15как20",
        "Round15as20",
        "Round 15 to 20",
    ),
    // Полный состав измерен пробой `measure-numeric-value-type.bsl`.
    (
        EnumKind::NumericValueType,
        EnumValue::NumericValueTypeCardinal,
        "Количественное",
        "Cardinal",
        "Cardinal",
    ),
    (
        EnumKind::NumericValueType,
        EnumValue::NumericValueTypeOrdinal,
        "Порядковое",
        "Ordinal",
        "Ordinal",
    ),
    // Полный состав и пробелы представлений измерены на 8.3.27.2342.
    (
        EnumKind::ErrorMessageDisplayVariant,
        EnumValue::ErrorMessageDisplayVariantAuto,
        "Авто",
        "Auto",
        "Auto",
    ),
    (
        EnumKind::ErrorMessageDisplayVariant,
        EnumValue::ErrorMessageDisplayVariantErrorMessageForUser,
        "СообщениеОбОшибкеДляПользователя",
        "ErrorMessageForUser",
        "Message about error for user ",
    ),
    (
        EnumKind::ErrorMessageDisplayVariant,
        EnumValue::ErrorMessageDisplayVariantBriefErrorDescription,
        "КраткоеПредставлениеОшибки",
        "BriefErrorDescription",
        "Short error presentation ",
    ),
    (
        EnumKind::ErrorMessageDisplayVariant,
        EnumValue::ErrorMessageDisplayVariantDetailErrorDescription,
        "ПодробноеПредставлениеОшибки",
        "DetailErrorDescription",
        "Detailed error presentation ",
    ),
    // Полный состав, представления и английские имена измерены на 8.3.27.2342.
    (
        EnumKind::CommandParameterUseMode,
        EnumValue::CommandParameterUseModeSingle,
        "Одиночный",
        "Single",
        "Single",
    ),
    (
        EnumKind::CommandParameterUseMode,
        EnumValue::CommandParameterUseModeMultiple,
        "Множественный",
        "Multiple",
        "Multiple",
    ),
    // Полный состав и представления измерены на 8.3.27.2342.
    (
        EnumKind::SearchStringModeOnInputByString,
        EnumValue::SearchStringModeOnInputByStringBegin,
        "Начало",
        "Begin",
        "At the beginning of line",
    ),
    (
        EnumKind::SearchStringModeOnInputByString,
        EnumValue::SearchStringModeOnInputByStringAnyPart,
        "ЛюбаяЧасть",
        "AnyPart",
        "Anywhere in line",
    ),
    // Полный состав и представления измерены на 8.3.27.2342.
    (
        EnumKind::RepresentableDocumentBatchFileType,
        EnumValue::RepresentableDocumentBatchFileTypeDocx,
        "DOCX",
        "DOCX",
        "DOCX",
    ),
    (
        EnumKind::RepresentableDocumentBatchFileType,
        EnumValue::RepresentableDocumentBatchFileTypeHtml4,
        "HTML4",
        "HTML4",
        "HTML4",
    ),
    (
        EnumKind::RepresentableDocumentBatchFileType,
        EnumValue::RepresentableDocumentBatchFileTypeHtml5,
        "HTML5",
        "HTML5",
        "HTML5",
    ),
    (
        EnumKind::RepresentableDocumentBatchFileType,
        EnumValue::RepresentableDocumentBatchFileTypeOds,
        "ODS",
        "ODS",
        "ODS",
    ),
    (
        EnumKind::RepresentableDocumentBatchFileType,
        EnumValue::RepresentableDocumentBatchFileTypePdf,
        "PDF",
        "PDF",
        "PDF",
    ),
    (
        EnumKind::RepresentableDocumentBatchFileType,
        EnumValue::RepresentableDocumentBatchFileTypeTxt,
        "TXT",
        "TXT",
        "TXT",
    ),
    (
        EnumKind::RepresentableDocumentBatchFileType,
        EnumValue::RepresentableDocumentBatchFileTypeXls,
        "XLS",
        "XLS",
        "XLS",
    ),
    (
        EnumKind::RepresentableDocumentBatchFileType,
        EnumValue::RepresentableDocumentBatchFileTypeXlsx,
        "XLSX",
        "XLSX",
        "XLSX",
    ),
    // Полный состав и представления измерены на 8.3.27.2342.
    (
        EnumKind::FontType,
        EnumValue::FontTypeWindowsFont,
        "WindowsШрифт",
        "WindowsFont",
        "Windows font",
    ),
    (
        EnumKind::FontType,
        EnumValue::FontTypeAbsolute,
        "Абсолютный",
        "Absolute",
        "Absolute",
    ),
    (
        EnumKind::FontType,
        EnumValue::FontTypeAutoFont,
        "АвтоШрифт",
        "AutoFont",
        "Auto font",
    ),
    (
        EnumKind::FontType,
        EnumValue::FontTypeStyleItem,
        "ЭлементСтиля",
        "StyleItem",
        "Style element",
    ),
    // Полный состав и представления измерены на 8.3.27.2342.
    (
        EnumKind::PictureType,
        EnumValue::PictureTypeAbsolute,
        "Абсолютная",
        "Absolute",
        "Absolute",
    ),
    (
        EnumKind::PictureType,
        EnumValue::PictureTypeFromLib,
        "ИзБиблиотеки",
        "FromLib",
        "From library",
    ),
    (
        EnumKind::PictureType,
        EnumValue::PictureTypeEmpty,
        "Пустая",
        "Empty",
        "Empty",
    ),
    // Полный состав и представления измерены на 8.3.27.2342.
    (
        EnumKind::FullTextSearchMetadataUse,
        EnumValue::FullTextSearchMetadataUseUse,
        "Использовать",
        "Use",
        "Use",
    ),
    (
        EnumKind::FullTextSearchMetadataUse,
        EnumValue::FullTextSearchMetadataUseDontUse,
        "НеИспользовать",
        "DontUse",
        "Do not use",
    ),
    // Полный состав и представления измерены на 8.3.27.2342.
    (
        EnumKind::LinkedValueChangeMode,
        EnumValue::LinkedValueChangeModeDontChange,
        "НеИзменять",
        "DontChange",
        "Do not change",
    ),
    (
        EnumKind::LinkedValueChangeMode,
        EnumValue::LinkedValueChangeModeClear,
        "Очищать",
        "Clear",
        "Clear",
    ),
    // Полный состав и представления измерены на 8.3.27.2342.
    (
        EnumKind::InternetMailAttachmentEncodingMode,
        EnumValue::InternetMailAttachmentEncodingModeMime,
        "MIME",
        "MIME",
        "MIME",
    ),
    (
        EnumKind::InternetMailAttachmentEncodingMode,
        EnumValue::InternetMailAttachmentEncodingModeUuEncode,
        "UUEncode",
        "UUEncode",
        "UUEncode",
    ),
    // Полный состав и представления измерены на 8.3.27.2342.
    (
        EnumKind::AutoSaveFormDataInSettings,
        EnumValue::AutoSaveFormDataInSettingsUse,
        "Использовать",
        "Use",
        "Use",
    ),
    (
        EnumKind::AutoSaveFormDataInSettings,
        EnumValue::AutoSaveFormDataInSettingsDontUse,
        "НеИспользовать",
        "DontUse",
        "Do not use",
    ),
    // Полный состав и представления измерены на 8.3.27.2342.
    (
        EnumKind::PDFDocumentFileType,
        EnumValue::PDFDocumentFileTypePdf,
        "PDF",
        "PDF",
        "PDF",
    ),
    (
        EnumKind::PDFDocumentFileType,
        EnumValue::PDFDocumentFileTypePdfA1,
        "PDF_A_1",
        "PDF_A_1",
        "PDF/A-1",
    ),
    (
        EnumKind::PDFDocumentFileType,
        EnumValue::PDFDocumentFileTypePdfA2,
        "PDF_A_2",
        "PDF_A_2",
        "PDF/A-2",
    ),
    (
        EnumKind::PDFDocumentFileType,
        EnumValue::PDFDocumentFileTypePdfA3,
        "PDF_A_3",
        "PDF_A_3",
        "PDF/A-3",
    ),
    // Полный состав и представления измерены на платформе 8.3.27.2074
    // пробой `measure-allowed-sign.bsl`.
    (
        EnumKind::AllowedSign,
        EnumValue::AllowedSignAny,
        "Любой",
        "Any",
        "Any",
    ),
    (
        EnumKind::AllowedSign,
        EnumValue::AllowedSignNonnegative,
        "Неотрицательный",
        "Nonnegative",
        "Nonnegative",
    ),
    // Полный состав, оба написания, представления и тип измерены
    // на платформе 8.3.27.2342 пробой `measure-auto-change-record.bsl`.
    (
        EnumKind::AutoChangeRecord,
        EnumValue::AutoChangeRecordDeny,
        "Запретить",
        "Deny",
        "Disable",
    ),
    (
        EnumKind::AutoChangeRecord,
        EnumValue::AutoChangeRecordAllow,
        "Разрешить",
        "Allow",
        "Enable",
    ),
    // Полный состав и представления измерены на платформе 8.3.27.2342
    // пробой `measure-button-representation.bsl`.
    (
        EnumKind::ButtonRepresentation,
        EnumValue::ButtonRepresentationAuto,
        "Авто",
        "Auto",
        "Auto",
    ),
    (
        EnumKind::ButtonRepresentation,
        EnumValue::ButtonRepresentationPicture,
        "Картинка",
        "Picture",
        "Icon",
    ),
    (
        EnumKind::ButtonRepresentation,
        EnumValue::ButtonRepresentationPictureAndText,
        "КартинкаИТекст",
        "PictureAndText",
        "Icon and text",
    ),
    (
        EnumKind::ButtonRepresentation,
        EnumValue::ButtonRepresentationText,
        "Текст",
        "Text",
        "Text",
    ),
    // Полный состав, оба написания, представления и тип измерены
    // на платформе 8.3.27.2342 пробой
    // `measure-password-policy-compliance-check-result.bsl`.
    (
        EnumKind::PasswordPolicyComplianceCheckResult,
        EnumValue::PasswordPolicyComplianceCheckResultMinLength,
        "НеСоответствуетТребованиямМинимальнойДлины",
        "DoesNotSatisfyMinLengthRequirements",
        "Does not meet the minimum length requirements",
    ),
    (
        EnumKind::PasswordPolicyComplianceCheckResult,
        EnumValue::PasswordPolicyComplianceCheckResultReuseLimit,
        "НеСоответствуетТребованиямОграниченияПовторенияСредиПоследних",
        "DoesNotSatisfyReuseLimitRequirements",
        "Does not meet the requirements for reuse limit among recent ones",
    ),
    (
        EnumKind::PasswordPolicyComplianceCheckResult,
        EnumValue::PasswordPolicyComplianceCheckResultCompromiseCheck,
        "НеСоответствуетТребованиямПроверкиРаскрытия",
        "DoesNotSatisfyCompromiseCheckRequirements",
        "Does not meet compromise check requirements",
    ),
    (
        EnumKind::PasswordPolicyComplianceCheckResult,
        EnumValue::PasswordPolicyComplianceCheckResultComplexity,
        "НеСоответствуетТребованиямСложности",
        "DoesNotSatisfyComplexityRequirements",
        "Does not meet the complexity requirements",
    ),
    // Полный состав, оба написания, представления и тип измерены
    // на платформе 8.3.27.2342 пробой `measure-table-representation.bsl`.
    (
        EnumKind::TableRepresentation,
        EnumValue::TableRepresentationList,
        "Список",
        "List",
        "List",
    ),
    (
        EnumKind::TableRepresentation,
        EnumValue::TableRepresentationHierarchicalList,
        "ИерархическийСписок",
        "HierarchicalList",
        "Hierarchical list",
    ),
    (
        EnumKind::TableRepresentation,
        EnumValue::TableRepresentationTree,
        "Дерево",
        "Tree",
        "Tree",
    ),
    // Полный состав, оба написания, представления и тип измерены
    // на платформе 8.3.27.2342 пробой `measure-form-field-type.bsl`.
    (
        EnumKind::FormFieldType,
        EnumValue::FormFieldTypeHtmlDocumentField,
        "ПолеHTMLДокумента",
        "HTMLDocumentField",
        "HTML document field",
    ),
    (
        EnumKind::FormFieldType,
        EnumValue::FormFieldTypePdfDocumentField,
        "ПолеPDFДокумента",
        "PDFDocumentField",
        "PDF document field",
    ),
    (
        EnumKind::FormFieldType,
        EnumValue::FormFieldTypeInputField,
        "ПолеВвода",
        "InputField",
        "Input field",
    ),
    (
        EnumKind::FormFieldType,
        EnumValue::FormFieldTypeGeographicalSchemaField,
        "ПолеГеографическойСхемы",
        "GeographicalSchemaField",
        "Geographical schema field",
    ),
    (
        EnumKind::FormFieldType,
        EnumValue::FormFieldTypeGraphicalSchemaField,
        "ПолеГрафическойСхемы",
        "GraphicalSchemaField",
        "Graphical schema field",
    ),
    (
        EnumKind::FormFieldType,
        EnumValue::FormFieldTypeDendrogramField,
        "ПолеДендрограммы",
        "DendrogramField",
        "Dendrogram field",
    ),
    (
        EnumKind::FormFieldType,
        EnumValue::FormFieldTypeChartField,
        "ПолеДиаграммы",
        "ChartField",
        "Chart field",
    ),
    (
        EnumKind::FormFieldType,
        EnumValue::FormFieldTypeGanttChartField,
        "ПолеДиаграммыГанта",
        "GanttChartField",
        "Gantt chart field",
    ),
    (
        EnumKind::FormFieldType,
        EnumValue::FormFieldTypeProgressBarField,
        "ПолеИндикатора",
        "ProgressBarField",
        "Progress bar",
    ),
    (
        EnumKind::FormFieldType,
        EnumValue::FormFieldTypeCalendarField,
        "ПолеКалендаря",
        "CalendarField",
        "Calendar field",
    ),
    (
        EnumKind::FormFieldType,
        EnumValue::FormFieldTypePictureField,
        "ПолеКартинки",
        "PictureField",
        "Image field",
    ),
    (
        EnumKind::FormFieldType,
        EnumValue::FormFieldTypeLabelField,
        "ПолеНадписи",
        "LabelField",
        "Text box",
    ),
    (
        EnumKind::FormFieldType,
        EnumValue::FormFieldTypeRadioButtonField,
        "ПолеПереключателя",
        "RadioButtonField",
        "Radio buttons",
    ),
    (
        EnumKind::FormFieldType,
        EnumValue::FormFieldTypePeriodField,
        "ПолеПериода",
        "PeriodField",
        "Period field",
    ),
    (
        EnumKind::FormFieldType,
        EnumValue::FormFieldTypePlannerField,
        "ПолеПланировщика",
        "PlannerField",
        "Planner field",
    ),
    (
        EnumKind::FormFieldType,
        EnumValue::FormFieldTypeTrackBarField,
        "ПолеПолосыРегулирования",
        "TrackBarField",
        "Slider field",
    ),
    (
        EnumKind::FormFieldType,
        EnumValue::FormFieldTypeSpreadsheetDocumentField,
        "ПолеТабличногоДокумента",
        "SpreadsheetDocumentField",
        "Spreadsheet document field",
    ),
    (
        EnumKind::FormFieldType,
        EnumValue::FormFieldTypeTextDocumentField,
        "ПолеТекстовогоДокумента",
        "TextDocumentField",
        "Text document field",
    ),
    (
        EnumKind::FormFieldType,
        EnumValue::FormFieldTypeCheckBoxField,
        "ПолеФлажка",
        "CheckBoxField",
        "Checkbox",
    ),
    (
        EnumKind::FormFieldType,
        EnumValue::FormFieldTypeFormattedDocumentField,
        "ПолеФорматированногоДокумента",
        "FormattedDocumentField",
        "Formatted document field",
    ),
    // Полный состав, оба написания, представления и тип измерены
    // на платформе 8.3.27.2342 пробой `measure-form-group-type.bsl`.
    (
        EnumKind::FormGroupType,
        EnumValue::FormGroupTypeButtonGroup,
        "ГруппаКнопок",
        "ButtonGroup",
        "Button group",
    ),
    (
        EnumKind::FormGroupType,
        EnumValue::FormGroupTypeColumnGroup,
        "ГруппаКолонок",
        "ColumnGroup",
        "Column group",
    ),
    (
        EnumKind::FormGroupType,
        EnumValue::FormGroupTypeCommandBar,
        "КоманднаяПанель",
        "CommandBar",
        "Command bar",
    ),
    (
        EnumKind::FormGroupType,
        EnumValue::FormGroupTypeContextMenu,
        "КонтекстноеМеню",
        "ContextMenu",
        "Context menu",
    ),
    (
        EnumKind::FormGroupType,
        EnumValue::FormGroupTypeUsualGroup,
        "ОбычнаяГруппа",
        "UsualGroup",
        "Regular group",
    ),
    (
        EnumKind::FormGroupType,
        EnumValue::FormGroupTypePopup,
        "Подменю",
        "Popup",
        "Submenu",
    ),
    (
        EnumKind::FormGroupType,
        EnumValue::FormGroupTypePage,
        "Страница",
        "Page",
        "Page",
    ),
    (
        EnumKind::FormGroupType,
        EnumValue::FormGroupTypePages,
        "Страницы",
        "Pages",
        "Pages",
    ),
    // Полный состав, оба написания, представления и тип измерены
    // на платформе 8.3.27.2342 пробой `measure-item-horizontal-location.bsl`.
    (
        EnumKind::ItemHorizontalLocation,
        EnumValue::ItemHorizontalLocationAuto,
        "Авто",
        "Auto",
        "Auto",
    ),
    (
        EnumKind::ItemHorizontalLocation,
        EnumValue::ItemHorizontalLocationLeft,
        "Лево",
        "Left",
        "Left",
    ),
    (
        EnumKind::ItemHorizontalLocation,
        EnumValue::ItemHorizontalLocationRight,
        "Право",
        "Right",
        "Right",
    ),
    (
        EnumKind::ItemHorizontalLocation,
        EnumValue::ItemHorizontalLocationCenter,
        "Центр",
        "Center",
        "Center",
    ),
    // Полный состав и представления измерены на платформе 8.3.27.2342
    // пробой `measure-form-alignment-spacing-enums.bsl`.
    (
        EnumKind::ItemVerticalAlign,
        EnumValue::ItemVerticalAlignAuto,
        "Авто",
        "Auto",
        "Auto",
    ),
    (
        EnumKind::ItemVerticalAlign,
        EnumValue::ItemVerticalAlignTop,
        "Верх",
        "Top",
        "Top",
    ),
    (
        EnumKind::ItemVerticalAlign,
        EnumValue::ItemVerticalAlignBottom,
        "Низ",
        "Bottom",
        "Bottom",
    ),
    (
        EnumKind::ItemVerticalAlign,
        EnumValue::ItemVerticalAlignCenter,
        "Центр",
        "Center",
        "Center",
    ),
    (
        EnumKind::FormItemSpacing,
        EnumValue::FormItemSpacingAuto,
        "Авто",
        "Auto",
        "Auto",
    ),
    (
        EnumKind::FormItemSpacing,
        EnumValue::FormItemSpacingDouble,
        "Двойной",
        "Double",
        "Double",
    ),
    (
        EnumKind::FormItemSpacing,
        EnumValue::FormItemSpacingNone,
        "Нет",
        "None",
        "None",
    ),
    (
        EnumKind::FormItemSpacing,
        EnumValue::FormItemSpacingSingle,
        "Одинарный",
        "Single",
        "Single",
    ),
    (
        EnumKind::FormItemSpacing,
        EnumValue::FormItemSpacingHalf,
        "Половинный",
        "Half",
        "Half",
    ),
    (
        EnumKind::FormItemSpacing,
        EnumValue::FormItemSpacingOneAndHalf,
        "Полуторный",
        "OneAndHalf",
        "One and half",
    ),
    // Полный состав, оба написания, представления и тип измерены
    // на платформе 8.3.27.2342 пробой `measure-button-shape-representation.bsl`.
    (
        EnumKind::ButtonShapeRepresentation,
        EnumValue::ButtonShapeRepresentationAuto,
        "Авто",
        "Auto",
        "Auto",
    ),
    (
        EnumKind::ButtonShapeRepresentation,
        EnumValue::ButtonShapeRepresentationAlways,
        "Всегда",
        "Always",
        "Always",
    ),
    (
        EnumKind::ButtonShapeRepresentation,
        EnumValue::ButtonShapeRepresentationNone,
        "Нет",
        "None",
        "None",
    ),
    (
        EnumKind::ButtonShapeRepresentation,
        EnumValue::ButtonShapeRepresentationWhenActive,
        "ПриАктивности",
        "WhenActive",
        "When active",
    ),
    // Полный состав, оба написания, представления и тип измерены
    // на платформе 8.3.27.2342 пробой `measure-usual-group-representation.bsl`.
    (
        EnumKind::UsualGroupRepresentation,
        EnumValue::UsualGroupRepresentationNone,
        "Нет",
        "None",
        "None",
    ),
    (
        EnumKind::UsualGroupRepresentation,
        EnumValue::UsualGroupRepresentationNormalSeparation,
        "ОбычноеВыделение",
        "NormalSeparation",
        "Normal marking",
    ),
    (
        EnumKind::UsualGroupRepresentation,
        EnumValue::UsualGroupRepresentationStrongSeparation,
        "СильноеВыделение",
        "StrongSeparation",
        "Strong marking",
    ),
    (
        EnumKind::UsualGroupRepresentation,
        EnumValue::UsualGroupRepresentationWeakSeparation,
        "СлабоеВыделение",
        "WeakSeparation",
        "Light marking",
    ),
    // Полный состав, оба написания, представления и тип измерены
    // на платформе 8.3.27.2342 пробой `measure-display-importance.bsl`.
    (
        EnumKind::DisplayImportance,
        EnumValue::DisplayImportanceAuto,
        "Авто",
        "Auto",
        "Auto",
    ),
    (
        EnumKind::DisplayImportance,
        EnumValue::DisplayImportanceHigh,
        "Высокая",
        "High",
        "High",
    ),
    (
        EnumKind::DisplayImportance,
        EnumValue::DisplayImportanceLow,
        "Низкая",
        "Low",
        "Low",
    ),
    (
        EnumKind::DisplayImportance,
        EnumValue::DisplayImportanceUsual,
        "Обычная",
        "Usual",
        "Normal",
    ),
    (
        EnumKind::DisplayImportance,
        EnumValue::DisplayImportanceVeryHigh,
        "ОченьВысокая",
        "VeryHigh",
        "Very high",
    ),
    (
        EnumKind::DisplayImportance,
        EnumValue::DisplayImportanceVeryLow,
        "ОченьНизкая",
        "VeryLow",
        "Very low",
    ),
    // Полный состав, оба написания, представления и тип измерены
    // на платформе 8.3.27.2342 пробой `measure-picture-format.bsl`.
    (
        EnumKind::PictureFormat,
        EnumValue::PictureFormatBmp,
        "BMP",
        "BMP",
        "BMP",
    ),
    (
        EnumKind::PictureFormat,
        EnumValue::PictureFormatEmf,
        "EMF",
        "EMF",
        "EMF",
    ),
    (
        EnumKind::PictureFormat,
        EnumValue::PictureFormatGif,
        "GIF",
        "GIF",
        "GIF",
    ),
    (
        EnumKind::PictureFormat,
        EnumValue::PictureFormatIcon,
        "Icon",
        "Icon",
        "Icon",
    ),
    (
        EnumKind::PictureFormat,
        EnumValue::PictureFormatJpeg,
        "JPEG",
        "JPEG",
        "JPEG",
    ),
    (
        EnumKind::PictureFormat,
        EnumValue::PictureFormatPng,
        "PNG",
        "PNG",
        "PNG",
    ),
    (
        EnumKind::PictureFormat,
        EnumValue::PictureFormatSvg,
        "SVG",
        "SVG",
        "SVG",
    ),
    (
        EnumKind::PictureFormat,
        EnumValue::PictureFormatTiff,
        "TIFF",
        "TIFF",
        "TIFF",
    ),
    (
        EnumKind::PictureFormat,
        EnumValue::PictureFormatWmf,
        "WMF",
        "WMF",
        "WMF",
    ),
    (
        EnumKind::PictureFormat,
        EnumValue::PictureFormatUnknown,
        "НеизвестныйФормат",
        "UnknownFormat",
        "Unknown format",
    ),
    // Полный состав, оба написания, представления и тип измерены
    // на платформе 8.3.27.2342 пробой `measure-internet-mail-text-type.bsl`.
    (
        EnumKind::InternetMailTextType,
        EnumValue::InternetMailTextTypeHtml,
        "HTML",
        "HTML",
        "HTML",
    ),
    (
        EnumKind::InternetMailTextType,
        EnumValue::InternetMailTextTypeCustomText,
        "ПроизвольныйТекст",
        "CustomText",
        "CustomText",
    ),
    (
        EnumKind::InternetMailTextType,
        EnumValue::InternetMailTextTypePlainText,
        "ПростойТекст",
        "PlainText",
        "PlainText",
    ),
    (
        EnumKind::InternetMailTextType,
        EnumValue::InternetMailTextTypeRichText,
        "РазмеченныйТекст",
        "RichText",
        "RichText",
    ),
    // Полный состав, оба написания, представления и тип измерены
    // на платформе 8.3.27.2342 пробой `measure-mail-non-ascii-encoding-mode.bsl`.
    (
        EnumKind::InternetMailNonAsciiEncodingMode,
        EnumValue::InternetMailNonAsciiMime,
        "MIME",
        "MIME",
        "MIME",
    ),
    (
        EnumKind::InternetMailNonAsciiEncodingMode,
        EnumValue::InternetMailNonAsciiQuotedPrintable,
        "QuotedPrintable",
        "QuotedPrintable",
        "QuotedPrintable",
    ),
    (
        EnumKind::InternetMailNonAsciiEncodingMode,
        EnumValue::InternetMailNonAsciiNone,
        "БезКодирования",
        "None",
        "None",
    ),
    // Полный состав, оба написания, представления и тип измерены
    // на платформе 8.3.27.2342 пробой `measure-mail-message-importance.bsl`.
    (
        EnumKind::InternetMailMessageImportance,
        EnumValue::InternetMailMessageImportanceHigh,
        "Высокая",
        "High",
        "High",
    ),
    (
        EnumKind::InternetMailMessageImportance,
        EnumValue::InternetMailMessageImportanceHighest,
        "Наивысшая",
        "Highest",
        "Highest",
    ),
    (
        EnumKind::InternetMailMessageImportance,
        EnumValue::InternetMailMessageImportanceLowest,
        "Наименьшая",
        "Lowest",
        "Lowest",
    ),
    (
        EnumKind::InternetMailMessageImportance,
        EnumValue::InternetMailMessageImportanceLow,
        "Низкая",
        "Low",
        "Low",
    ),
    (
        EnumKind::InternetMailMessageImportance,
        EnumValue::InternetMailMessageImportanceNormal,
        "Обычная",
        "Normal",
        "Normal",
    ),
    // Полный состав, оба написания, представления и тип измерены
    // на платформе 8.3.27.2342 пробой `measure-additional-show-mode.bsl`.
    (
        EnumKind::AdditionalShowMode,
        EnumValue::AdditionalShowModeDontUse,
        "НеИспользовать",
        "DontUse",
        "Do not use",
    ),
    (
        EnumKind::AdditionalShowMode,
        EnumValue::AdditionalShowModeIrrelevance,
        "Неактуальность",
        "Irrelevance",
        "Irrelevance",
    ),
    // Полный состав, оба написания, представления и тип измерены
    // на платформе 8.3.27.2342 пробой `measure-tls-revocation-mode.bsl`.
    (
        EnumKind::ServerTlsCertificateRevocationCheckMode,
        EnumValue::ServerTlsCertificateRevocationCheckModeAuto,
        "Авто",
        "Auto",
        "Auto",
    ),
    (
        EnumKind::ServerTlsCertificateRevocationCheckMode,
        EnumValue::ServerTlsCertificateRevocationCheckModeDontCheck,
        "НеПроверять",
        "DontCheck",
        "Certificate revocation check is disabled",
    ),
    (
        EnumKind::ServerTlsCertificateRevocationCheckMode,
        EnumValue::ServerTlsCertificateRevocationCheckModeSoftFail,
        "Нестрогий",
        "SoftFail",
        "Soft certificate revocation check",
    ),
    (
        EnumKind::ServerTlsCertificateRevocationCheckMode,
        EnumValue::ServerTlsCertificateRevocationCheckModeStrict,
        "Строгий",
        "Strict",
        "Strict certificate revocation check",
    ),
    // Полный состав, оба написания, представления и тип измерены
    // на платформе 8.3.27.2342 пробой
    // `measure-data-composition-filter-application-type.bsl`.
    (
        EnumKind::DataCompositionFilterApplicationType,
        EnumValue::DataCompositionFilterApplicationTypeItems,
        "Элементы",
        "Items",
        "No hierarchy",
    ),
    (
        EnumKind::DataCompositionFilterApplicationType,
        EnumValue::DataCompositionFilterApplicationTypeHierarchy,
        "Иерархия",
        "Hierarchy",
        "Hierarchy",
    ),
    (
        EnumKind::DataCompositionFilterApplicationType,
        EnumValue::DataCompositionFilterApplicationTypeHierarchyOnly,
        "ТолькоИерархия",
        "HierarchyOnly",
        "Hierarchy only",
    ),
    // Полный состав, алиасы и представления измерены на 8.3.27.2342.
    (
        EnumKind::DataCompositionSettingsItemState,
        EnumValue::DataCompositionSettingsItemStateEnabled,
        "Включен",
        "Enabled",
        "Enabled",
    ),
    (
        EnumKind::DataCompositionSettingsItemState,
        EnumValue::DataCompositionSettingsItemStateDisabled,
        "Отключен",
        "Disabled",
        "Disabled",
    ),
    (
        EnumKind::DataCompositionSettingsItemState,
        EnumValue::DataCompositionSettingsItemStateDeletedByUser,
        "УдаленПользователем",
        "DeletedByUser",
        "Deleted by user",
    ),
    // Полный состав, оба написания, представления и тип измерены
    // на платформе 8.3.27.2342 пробой
    // `measure-data-composition-settings-item-view-mode.bsl`.
    (
        EnumKind::DataCompositionSettingsItemViewMode,
        EnumValue::DataCompositionSettingsItemViewModeAuto,
        "Авто",
        "Auto",
        "Auto",
    ),
    (
        EnumKind::DataCompositionSettingsItemViewMode,
        EnumValue::DataCompositionSettingsItemViewModeNormal,
        "Обычный",
        "Normal",
        "Normal",
    ),
    (
        EnumKind::DataCompositionSettingsItemViewMode,
        EnumValue::DataCompositionSettingsItemViewModeQuickAccess,
        "БыстрыйДоступ",
        "QuickAccess",
        "Quick access",
    ),
    (
        EnumKind::DataCompositionSettingsItemViewMode,
        EnumValue::DataCompositionSettingsItemViewModeInaccessible,
        "Недоступный",
        "Inaccessible",
        "Disabled",
    ),
    // Полный состав и представления измерены на платформе 8.3.27.2342
    // пробой `measure-data-composition-settings-refresh-method.bsl`.
    (
        EnumKind::DataCompositionSettingsRefreshMethod,
        EnumValue::DataCompositionSettingsRefreshMethodFull,
        "Полное",
        "Full",
        "Full",
    ),
    (
        EnumKind::DataCompositionSettingsRefreshMethod,
        EnumValue::DataCompositionSettingsRefreshMethodCheckAvailability,
        "ПроверятьДоступность",
        "CheckAvailability",
        "Check accessibility",
    ),
    // Типы, члены, алиасы и представления измерены на 8.3.27.2342.
    (
        EnumKind::DataCompositionTextPlacementType,
        EnumValue::DataCompositionTextPlacementTypeBlock,
        "Забивать",
        "Block",
        "Conceal",
    ),
    (
        EnumKind::DataCompositionTextPlacementType,
        EnumValue::DataCompositionTextPlacementTypeCut,
        "Обрезать",
        "Cut",
        "Clip",
    ),
    (
        EnumKind::DataCompositionTextPlacementType,
        EnumValue::DataCompositionTextPlacementTypeWrap,
        "Переносить",
        "Wrap",
        "Wrap",
    ),
    (
        EnumKind::DataCompositionTextPlacementType,
        EnumValue::DataCompositionTextPlacementTypeOverflow,
        "Выступать",
        "Overflow",
        "Overflow",
    ),
    // Полный состав и представления измерены на платформе 8.3.27.2342
    // пробой `measure-data-composition-text-output-type.bsl`.
    (
        EnumKind::DataCompositionTextOutputType,
        EnumValue::DataCompositionTextOutputTypeAuto,
        "Авто",
        "Auto",
        "Auto",
    ),
    (
        EnumKind::DataCompositionTextOutputType,
        EnumValue::DataCompositionTextOutputTypeOutput,
        "Выводить",
        "Output",
        "Output",
    ),
    (
        EnumKind::DataCompositionTextOutputType,
        EnumValue::DataCompositionTextOutputTypeDontOutput,
        "НеВыводить",
        "DontOutput",
        "Do not output",
    ),
    // Полный состав и представления измерены на платформе 8.3.27.2342
    // пробой `measure-view-scaling-mode.bsl`.
    (
        EnumKind::ViewScalingMode,
        EnumValue::ViewScalingModeAuto,
        "Авто",
        "Auto",
        "Auto",
    ),
    (
        EnumKind::ViewScalingMode,
        EnumValue::ViewScalingModeLarge,
        "Крупный",
        "Large",
        "Large",
    ),
    (
        EnumKind::ViewScalingMode,
        EnumValue::ViewScalingModeNormal,
        "Обычный",
        "Normal",
        "Normal",
    ),
    // Полный состав и представления измерены на платформе 8.3.27.2342
    // пробой `measure-data-composition-total-placement.bsl`.
    (
        EnumKind::DataCompositionTotalPlacement,
        EnumValue::DataCompositionTotalPlacementAuto,
        "Авто",
        "Auto",
        "Auto",
    ),
    (
        EnumKind::DataCompositionTotalPlacement,
        EnumValue::DataCompositionTotalPlacementEnd,
        "Конец",
        "End",
        "End",
    ),
    (
        EnumKind::DataCompositionTotalPlacement,
        EnumValue::DataCompositionTotalPlacementBegin,
        "Начало",
        "Begin",
        "Begin",
    ),
    (
        EnumKind::DataCompositionTotalPlacement,
        EnumValue::DataCompositionTotalPlacementBeginAndEnd,
        "НачалоИКонец",
        "BeginAndEnd",
        "Begin and end",
    ),
    (
        EnumKind::DataCompositionTotalPlacement,
        EnumValue::DataCompositionTotalPlacementNone,
        "Нет",
        "None",
        "None",
    ),
    // Полный состав и представления измерены на платформе 8.3.27.2342
    // пробой `measure-picture-size.bsl`.
    (
        EnumKind::PictureSize,
        EnumValue::PictureSizeAutoSize,
        "АвтоРазмер",
        "AutoSize",
        "Automatic size",
    ),
    (
        EnumKind::PictureSize,
        EnumValue::PictureSizeAutoSizeIgnoreScale,
        "АвтоРазмерБезУчетаМасштаба",
        "AutoSizeIgnoreScale",
        "Automatic size without scale",
    ),
    (
        EnumKind::PictureSize,
        EnumValue::PictureSizeByFontSize,
        "ПоРазмеруШрифта",
        "ByFontSize",
        "By font size",
    ),
    (
        EnumKind::PictureSize,
        EnumValue::PictureSizeProportionally,
        "Пропорционально",
        "Proportionally",
        "Proportional",
    ),
    (
        EnumKind::PictureSize,
        EnumValue::PictureSizeStretch,
        "Растянуть",
        "Stretch",
        "Stretch",
    ),
    (
        EnumKind::PictureSize,
        EnumValue::PictureSizeRealSize,
        "РеальныйРазмер",
        "RealSize",
        "Actual size",
    ),
    (
        EnumKind::PictureSize,
        EnumValue::PictureSizeRealSizeIgnoreScale,
        "РеальныйРазмерБезУчетаМасштаба",
        "RealSizeIgnoreScale",
        "Actual size (no scaling)",
    ),
    (
        EnumKind::PictureSize,
        EnumValue::PictureSizeTile,
        "Черепица",
        "Tile",
        "Tile",
    ),
    // Полный состав и представления измерены на платформе 8.3.27.2342
    // пробой `measure-spreadsheet-document-shift-type.bsl`.
    (
        EnumKind::SpreadsheetDocumentShiftType,
        EnumValue::SpreadsheetDocumentShiftTypeWithoutShift,
        "БезСмещения",
        "WithoutShift",
        "No shift",
    ),
    (
        EnumKind::SpreadsheetDocumentShiftType,
        EnumValue::SpreadsheetDocumentShiftTypeVertical,
        "ПоВертикали",
        "Vertical",
        "Vertically",
    ),
    (
        EnumKind::SpreadsheetDocumentShiftType,
        EnumValue::SpreadsheetDocumentShiftTypeHorizontal,
        "ПоГоризонтали",
        "Horizontal",
        "Horizontally",
    ),
    // Полный состав и представления измерены на платформе 8.3.27.2342
    // пробой `measure-chart-type.bsl`.
    (
        EnumKind::ChartType,
        EnumValue::ChartTypeStock,
        "Биржевая",
        "Stock",
        "Stock",
    ),
    (
        EnumKind::ChartType,
        EnumValue::ChartTypeOpenHighLowClose,
        "БиржеваяСвеча",
        "OpenHighLowClose",
        "Open-High\"-Low-Close\"",
    ),
    (
        EnumKind::ChartType,
        EnumValue::ChartTypeConcaveSurface,
        "ВогнутаяПоверхность",
        "ConcaveSurface",
        "Concave surface",
    ),
    (
        EnumKind::ChartType,
        EnumValue::ChartTypeWaterfall,
        "Водопад",
        "Waterfall",
        "Waterfall",
    ),
    (
        EnumKind::ChartType,
        EnumValue::ChartTypeFunnel,
        "Воронка",
        "Funnel",
        "Cone",
    ),
    (
        EnumKind::ChartType,
        EnumValue::ChartTypeNormalizedFunnel,
        "ВоронкаНормированная",
        "NormalizedFunnel",
        "Standardized cone",
    ),
    (
        EnumKind::ChartType,
        EnumValue::ChartTypeNormalizedFunnel3D,
        "ВоронкаНормированнаяОбъемная",
        "NormalizedFunnel3D",
        "Standardized volumetric cone",
    ),
    (
        EnumKind::ChartType,
        EnumValue::ChartTypeFunnel3D,
        "ВоронкаОбъемная",
        "Funnel3D",
        "Volumetric cone",
    ),
    (
        EnumKind::ChartType,
        EnumValue::ChartTypeConvexSurface,
        "ВыпуклаяПоверхность",
        "ConvexSurface",
        "Convex surface",
    ),
    (
        EnumKind::ChartType,
        EnumValue::ChartTypeColumn,
        "Гистограмма",
        "Column",
        "Column",
    ),
    (
        EnumKind::ChartType,
        EnumValue::ChartTypeBar,
        "ГистограммаГоризонтальная",
        "Bar",
        "Bar",
    ),
    (
        EnumKind::ChartType,
        EnumValue::ChartTypeBar3D,
        "ГистограммаГоризонтальнаяОбъемная",
        "Bar3D",
        "Bar 3D",
    ),
    (
        EnumKind::ChartType,
        EnumValue::ChartTypeNormalizedColumn,
        "ГистограммаНормированная",
        "NormalizedColumn",
        "Normalized bar chart",
    ),
    (
        EnumKind::ChartType,
        EnumValue::ChartTypeNormalizedBar,
        "ГистограммаНормированнаяГоризонтальная",
        "NormalizedBar",
        "Normalized horizontal bar",
    ),
    (
        EnumKind::ChartType,
        EnumValue::ChartTypeNormalizedBar3D,
        "ГистограммаНормированнаяГоризонтальнаяОбъемная",
        "NormalizedBar3D",
        "Normalized horizontal 3D column",
    ),
    (
        EnumKind::ChartType,
        EnumValue::ChartTypeNormalizedColumn3D,
        "ГистограммаНормированнаяОбъемная",
        "NormalizedColumn3D",
        "Normalized 3D column",
    ),
    (
        EnumKind::ChartType,
        EnumValue::ChartTypeColumn3D,
        "ГистограммаОбъемная",
        "Column3D",
        "3D column",
    ),
    (
        EnumKind::ChartType,
        EnumValue::ChartTypeStackedColumn,
        "ГистограммаСНакоплением",
        "StackedColumn",
        "Stacked column",
    ),
    (
        EnumKind::ChartType,
        EnumValue::ChartTypeStackedBar,
        "ГистограммаСНакоплениемГоризонтальная",
        "StackedBar",
        "Stacked Bar",
    ),
    (
        EnumKind::ChartType,
        EnumValue::ChartTypeStackedBar3D,
        "ГистограммаСНакоплениемГоризонтальнаяОбъемная",
        "StackedBar3D",
        "Stacked bar 3D ",
    ),
    (
        EnumKind::ChartType,
        EnumValue::ChartTypeStackedColumn3D,
        "ГистограммаСНакоплениемОбъемная",
        "StackedColumn3D",
        "Stacked column 3D",
    ),
    (
        EnumKind::ChartType,
        EnumValue::ChartTypeLine,
        "График",
        "Line",
        "Line",
    ),
    (
        EnumKind::ChartType,
        EnumValue::ChartTypeStep,
        "ГрафикПоШагам",
        "Step",
        "Step",
    ),
    (
        EnumKind::ChartType,
        EnumValue::ChartTypeStackedLine,
        "ГрафикСНакоплением",
        "StackedLine",
        "Stacked graph",
    ),
    (
        EnumKind::ChartType,
        EnumValue::ChartTypeArea,
        "ГрафикСОбластями",
        "Area",
        "Area",
    ),
    (
        EnumKind::ChartType,
        EnumValue::ChartTypeStackedArea,
        "ГрафикСОбластямиИНакоплением",
        "StackedArea",
        "Stacked Area",
    ),
    (
        EnumKind::ChartType,
        EnumValue::ChartTypeNormalizedArea,
        "ГрафикСОбластямиНормированный",
        "NormalizedArea",
        "Normalized area graph",
    ),
    (
        EnumKind::ChartType,
        EnumValue::ChartTypeShadedSurface,
        "ЗатененнаяПоверхность",
        "ShadedSurface",
        "Shaded surface",
    ),
    (
        EnumKind::ChartType,
        EnumValue::ChartTypeGauge,
        "Измерительная",
        "Gauge",
        "Gauge",
    ),
    (
        EnumKind::ChartType,
        EnumValue::ChartTypeBarGraph,
        "Изометрическая",
        "BarGraph",
        "Bar graph",
    ),
    (
        EnumKind::ChartType,
        EnumValue::ChartTypeTapeGraph,
        "ИзометрическаяЛента",
        "TapeGraph",
        "Tape Graph",
    ),
    (
        EnumKind::ChartType,
        EnumValue::ChartTypeCeilGraph,
        "ИзометрическаяНепрерывная",
        "CeilGraph",
        "Ceil Graph",
    ),
    (
        EnumKind::ChartType,
        EnumValue::ChartTypePyramidGraph,
        "ИзометрическаяПирамида",
        "PyramidGraph",
        "Pyramid Graph",
    ),
    (
        EnumKind::ChartType,
        EnumValue::ChartTypeWireframeSurface,
        "КаркаснаяПоверхность",
        "WireframeSurface",
        "Wire-frame surface",
    ),
    (
        EnumKind::ChartType,
        EnumValue::ChartTypeDonut,
        "Кольцевая",
        "Donut",
        "Doughnut",
    ),
    (
        EnumKind::ChartType,
        EnumValue::ChartTypeDonut3D,
        "КольцеваяОбъемная",
        "Donut3D",
        "3D doughnut",
    ),
    (
        EnumKind::ChartType,
        EnumValue::ChartTypePie,
        "Круговая",
        "Pie",
        "Pie",
    ),
    (
        EnumKind::ChartType,
        EnumValue::ChartTypePie3D,
        "КруговаяОбъемная",
        "Pie3D",
        "Pie 3D",
    ),
    (
        EnumKind::ChartType,
        EnumValue::ChartTypeSurface,
        "Поверхность",
        "Surface",
        "Surface",
    ),
    (
        EnumKind::ChartType,
        EnumValue::ChartTypeBubble,
        "Пузырьковая",
        "Bubble",
        "Bubble",
    ),
    (
        EnumKind::ChartType,
        EnumValue::ChartTypeRadarLine,
        "РадарныйГрафик",
        "RadarLine",
        "Radar chart",
    ),
    (
        EnumKind::ChartType,
        EnumValue::ChartTypeRadarStackedLine,
        "РадарныйГрафикСНакоплением",
        "RadarStackedLine",
        "Stacked radar chart",
    ),
    (
        EnumKind::ChartType,
        EnumValue::ChartTypeRadarArea,
        "РадарныйГрафикСОбластями",
        "RadarArea",
        "Radar area chart",
    ),
    (
        EnumKind::ChartType,
        EnumValue::ChartTypeRadarStackedArea,
        "РадарныйГрафикСОбластямиИНакоплением",
        "RadarStackedArea",
        "Stacked radar area chart",
    ),
    (
        EnumKind::ChartType,
        EnumValue::ChartTypeRadarNormalizedArea,
        "РадарныйГрафикСОбластямиНормированный",
        "RadarNormalizedArea",
        "Normalized radar area chart",
    ),
    (
        EnumKind::ChartType,
        EnumValue::ChartTypeHoneycomb,
        "Сотовая",
        "Honeycomb",
        "Honeycomb",
    ),
    (
        EnumKind::ChartType,
        EnumValue::ChartTypeScatter,
        "Точечная",
        "Scatter",
        "Scatter",
    ),
    // Оба русских имени и представления измерены на платформе 8.3.27.2342
    // пробой `measure-chart-label-orientation-name.bsl`.
    (
        EnumKind::ChartLabelsOrientation,
        EnumValue::ChartLabelsOrientationAuto,
        "Авто",
        "Auto",
        "Auto",
    ),
    (
        EnumKind::ChartLabelsOrientation,
        EnumValue::ChartLabelsOrientationVertical,
        "Вертикально",
        "Vertical",
        "Vertical",
    ),
    (
        EnumKind::ChartLabelsOrientation,
        EnumValue::ChartLabelsOrientationHorizontal,
        "Горизонтально",
        "Horizontal",
        "Horizontal",
    ),
    (
        EnumKind::ChartLabelsOrientation,
        EnumValue::ChartLabelsOrientationCustomAngle,
        "ПроизвольныйУголНаклона",
        "CustomAngle",
        "Custom angle",
    ),
    // Полный состав и представления измерены на платформе 8.3.27.2342
    // пробой `measure-boundary-type.bsl`.
    (
        EnumKind::BoundaryType,
        EnumValue::BoundaryTypeIncluding,
        "Включая",
        "Including",
        "Including",
    ),
    (
        EnumKind::BoundaryType,
        EnumValue::BoundaryTypeExcluding,
        "Исключая",
        "Excluding",
        "Excluding",
    ),
    // Полный состав и представления измерены на платформе 8.3.27.2342
    // пробой `measure-max-series.bsl`.
    (
        EnumKind::MaxSeries,
        EnumValue::MaxSeriesNotDefined,
        "НеЗадано",
        "NotDefined",
        "Not set",
    ),
    (
        EnumKind::MaxSeries,
        EnumValue::MaxSeriesLimited,
        "Ограничено",
        "Limited",
        "Limited",
    ),
    (
        EnumKind::MaxSeries,
        EnumValue::MaxSeriesPercent,
        "Процент",
        "Percent",
        "Percent",
    ),
    // Полный состав и представления измерены на платформе 8.3.27.2342
    // пробой `measure-chart-label-type.bsl`.
    (
        EnumKind::ChartLabelType,
        EnumValue::ChartLabelTypeValue,
        "Значение",
        "Value",
        "Value",
    ),
    (
        EnumKind::ChartLabelType,
        EnumValue::ChartLabelTypeValuePercent,
        "ЗначениеПроцент",
        "ValuePercent",
        "Value + percent",
    ),
    (
        EnumKind::ChartLabelType,
        EnumValue::ChartLabelTypeValueSize,
        "ЗначениеРазмер",
        "ValueSize",
        "Value + size",
    ),
    (
        EnumKind::ChartLabelType,
        EnumValue::ChartLabelTypeNone,
        "Нет",
        "None",
        "None",
    ),
    (
        EnumKind::ChartLabelType,
        EnumValue::ChartLabelTypePercent,
        "Процент",
        "Percent",
        "Percent",
    ),
    (
        EnumKind::ChartLabelType,
        EnumValue::ChartLabelTypeSeries,
        "Серия",
        "Series",
        "Series",
    ),
    (
        EnumKind::ChartLabelType,
        EnumValue::ChartLabelTypeSeriesValue,
        "СерияЗначение",
        "SeriesValue",
        "Series + value",
    ),
    (
        EnumKind::ChartLabelType,
        EnumValue::ChartLabelTypeSeriesValuePercent,
        "СерияЗначениеПроцент",
        "SeriesValuePercent",
        "Series + value + percent",
    ),
    (
        EnumKind::ChartLabelType,
        EnumValue::ChartLabelTypeSeriesValueSize,
        "СерияЗначениеРазмер",
        "SeriesValueSize",
        "Series + value + size",
    ),
    (
        EnumKind::ChartLabelType,
        EnumValue::ChartLabelTypeSeriesPercent,
        "СерияПроцент",
        "SeriesPercent",
        "Series + percent",
    ),
    (
        EnumKind::ChartLabelType,
        EnumValue::ChartLabelTypeSeriesSize,
        "СерияРазмер",
        "SeriesSize",
        "Series + size",
    ),
    (
        EnumKind::ChartLabelType,
        EnumValue::ChartLabelTypeSeriesPoint,
        "СерияТочка",
        "SeriesPoint",
        "Series + point",
    ),
    (
        EnumKind::ChartLabelType,
        EnumValue::ChartLabelTypeSeriesPointValue,
        "СерияТочкаЗначение",
        "SeriesPointValue",
        "Series + point + value",
    ),
    (
        EnumKind::ChartLabelType,
        EnumValue::ChartLabelTypeSeriesPointValuePercent,
        "СерияТочкаЗначениеПроцент",
        "SeriesPointValuePercent",
        "Series + point + value + percent",
    ),
    (
        EnumKind::ChartLabelType,
        EnumValue::ChartLabelTypeSeriesPointValueSize,
        "СерияТочкаЗначениеРазмер",
        "SeriesPointValueSize",
        "Series + point + value + size",
    ),
    (
        EnumKind::ChartLabelType,
        EnumValue::ChartLabelTypeSeriesPointPercent,
        "СерияТочкаПроцент",
        "SeriesPointPercent",
        "Series + point + percent",
    ),
    (
        EnumKind::ChartLabelType,
        EnumValue::ChartLabelTypeSeriesPointSize,
        "СерияТочкаРазмер",
        "SeriesPointSize",
        "Series + point + size",
    ),
    (
        EnumKind::ChartLabelType,
        EnumValue::ChartLabelTypePoint,
        "Точка",
        "Point",
        "Point",
    ),
    (
        EnumKind::ChartLabelType,
        EnumValue::ChartLabelTypePointValue,
        "ТочкаЗначение",
        "PointValue",
        "Point + value",
    ),
    (
        EnumKind::ChartLabelType,
        EnumValue::ChartLabelTypePointValuePercent,
        "ТочкаЗначениеПроцент",
        "PointValuePercent",
        "Point + value + percent",
    ),
    (
        EnumKind::ChartLabelType,
        EnumValue::ChartLabelTypePointValueSize,
        "ТочкаЗначениеРазмер",
        "PointValueSize",
        "Point + value + size",
    ),
    (
        EnumKind::ChartLabelType,
        EnumValue::ChartLabelTypePointPercent,
        "ТочкаПроцент",
        "PointPercent",
        "Point + percent",
    ),
    (
        EnumKind::ChartLabelType,
        EnumValue::ChartLabelTypePointSize,
        "ТочкаРазмер",
        "PointSize",
        "Point + size",
    ),
    // Полный состав и представления измерены на платформе 8.3.27.2342
    // пробой `measure-chart-line-type.bsl`.
    (
        EnumKind::ChartLineType,
        EnumValue::ChartLineTypeNone,
        "НетЛинии",
        "None",
        "None",
    ),
    (
        EnumKind::ChartLineType,
        EnumValue::ChartLineTypeDashed,
        "Пунктир",
        "Dashed",
        "Dashed",
    ),
    (
        EnumKind::ChartLineType,
        EnumValue::ChartLineTypeDashDotted,
        "ПунктирТочка",
        "DashDotted",
        "DashDotted",
    ),
    (
        EnumKind::ChartLineType,
        EnumValue::ChartLineTypeDashDottedDotted,
        "ПунктирТочкаТочка",
        "DashDottedDotted",
        "DotDashDotted",
    ),
    (
        EnumKind::ChartLineType,
        EnumValue::ChartLineTypeSolid,
        "Сплошная",
        "Solid",
        "Solid",
    ),
    (
        EnumKind::ChartLineType,
        EnumValue::ChartLineTypeDotted,
        "Точечная",
        "Dotted",
        "Dotted",
    ),
    // Полный состав и представления измерены на платформе 8.3.27.2342
    // пробой `measure-form-button-type.bsl`.
    (
        EnumKind::FormButtonType,
        EnumValue::FormButtonTypeHyperlink,
        "Гиперссылка",
        "Hyperlink",
        "Hyperlink",
    ),
    (
        EnumKind::FormButtonType,
        EnumValue::FormButtonTypeCommandBarHyperlink,
        "ГиперссылкаКоманднойПанели",
        "CommandBarHyperlink",
        "Command bar hyperlink",
    ),
    (
        EnumKind::FormButtonType,
        EnumValue::FormButtonTypeCommandBarButton,
        "КнопкаКоманднойПанели",
        "CommandBarButton",
        "Command bar button",
    ),
    (
        EnumKind::FormButtonType,
        EnumValue::FormButtonTypeUsualButton,
        "ОбычнаяКнопка",
        "UsualButton",
        "Common button",
    ),
    // Полный состав и представления измерены на платформе 8.3.27.2074
    // пробой `measure-xml-type-assignment.bsl`.
    (
        EnumKind::XmlTypeAssignment,
        EnumValue::XmlTypeAssignmentImplicit,
        "Неявное",
        "Implicit",
        "Implicit",
    ),
    (
        EnumKind::XmlTypeAssignment,
        EnumValue::XmlTypeAssignmentExplicit,
        "Явное",
        "Explicit",
        "Explicit",
    ),
    // Полный состав и представления измерены на платформе 8.3.27.2074
    // пробой `measure-platform-type.bsl`.
    (
        EnumKind::PlatformType,
        EnumValue::PlatformTypeAndroidArm,
        "Android_ARM",
        "Android_ARM",
        "Android ARM",
    ),
    (
        EnumKind::PlatformType,
        EnumValue::PlatformTypeAndroidArm64,
        "Android_ARM_64",
        "Android_ARM_64",
        "Android ARM_64",
    ),
    (
        EnumKind::PlatformType,
        EnumValue::PlatformTypeAndroidX86,
        "Android_x86",
        "Android_x86",
        "Android x86",
    ),
    (
        EnumKind::PlatformType,
        EnumValue::PlatformTypeAndroidX86_64,
        "Android_x86_64",
        "Android_x86_64",
        "Android x86_64",
    ),
    (
        EnumKind::PlatformType,
        EnumValue::PlatformTypeIosArm,
        "iOS_ARM",
        "iOS_ARM",
        "iOS ARM",
    ),
    (
        EnumKind::PlatformType,
        EnumValue::PlatformTypeIosArm64,
        "iOS_ARM_64",
        "iOS_ARM_64",
        "iOS ARM_64",
    ),
    (
        EnumKind::PlatformType,
        EnumValue::PlatformTypeLinuxArm64,
        "Linux_ARM64",
        "Linux_ARM64",
        "Linux ARM64",
    ),
    (
        EnumKind::PlatformType,
        EnumValue::PlatformTypeLinuxE2k,
        "Linux_E2K",
        "Linux_E2K",
        "Linux E2K",
    ),
    (
        EnumKind::PlatformType,
        EnumValue::PlatformTypeLinuxX86,
        "Linux_x86",
        "Linux_x86",
        "Linux x86",
    ),
    (
        EnumKind::PlatformType,
        EnumValue::PlatformTypeLinuxX86_64,
        "Linux_x86_64",
        "Linux_x86_64",
        "Linux x86-64",
    ),
    (
        EnumKind::PlatformType,
        EnumValue::PlatformTypeMacOsX86,
        "MacOS_x86",
        "MacOS_x86",
        "MacOS x86",
    ),
    (
        EnumKind::PlatformType,
        EnumValue::PlatformTypeMacOsX86_64,
        "MacOS_x86_64",
        "MacOS_x86_64",
        "MacOS x86-64",
    ),
    (
        EnumKind::PlatformType,
        EnumValue::PlatformTypeWindowsX86,
        "Windows_x86",
        "Windows_x86",
        "Windows x86",
    ),
    (
        EnumKind::PlatformType,
        EnumValue::PlatformTypeWindowsX86_64,
        "Windows_x86_64",
        "Windows_x86_64",
        "Windows x86-64",
    ),
    (
        EnumKind::PlatformType,
        EnumValue::PlatformTypeWinRtArm,
        "WinRT_ARM",
        "WinRT_ARM",
        "WinRT ARM",
    ),
    (
        EnumKind::PlatformType,
        EnumValue::PlatformTypeWinRtX86,
        "WinRT_x86",
        "WinRT_x86",
        "WinRT x86",
    ),
    (
        EnumKind::PlatformType,
        EnumValue::PlatformTypeWinRtX86_64,
        "WinRT_x86_64",
        "WinRT_x86_64",
        "WinRT x86_64",
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
    // Состав, оба написания и представления измерены на платформе 8.3.27
    // пробой `measure-comparison-type.bsl`.
    (
        EnumKind::ComparisonType,
        EnumValue::ComparisonGreater,
        "Больше",
        "Greater",
        "Greater than",
    ),
    (
        EnumKind::ComparisonType,
        EnumValue::ComparisonGreaterOrEqual,
        "БольшеИлиРавно",
        "GreaterOrEqual",
        "Greater than or equal to",
    ),
    (
        EnumKind::ComparisonType,
        EnumValue::ComparisonInHierarchy,
        "ВИерархии",
        "InHierarchy",
        "In group",
    ),
    (
        EnumKind::ComparisonType,
        EnumValue::ComparisonInList,
        "ВСписке",
        "InList",
        "In list",
    ),
    (
        EnumKind::ComparisonType,
        EnumValue::ComparisonInListByHierarchy,
        "ВСпискеПоИерархии",
        "InListByHierarchy",
        "In list group",
    ),
    (
        EnumKind::ComparisonType,
        EnumValue::ComparisonInterval,
        "Интервал",
        "Interval",
        "Range (>, <)",
    ),
    (
        EnumKind::ComparisonType,
        EnumValue::ComparisonIntervalIncludingBounds,
        "ИнтервалВключаяГраницы",
        "IntervalIncludingBounds",
        "Range (>=, <=)",
    ),
    (
        EnumKind::ComparisonType,
        EnumValue::ComparisonIntervalIncludingLowerBound,
        "ИнтервалВключаяНачало",
        "IntervalIncludingLowerBound",
        "Range (>=, <)",
    ),
    (
        EnumKind::ComparisonType,
        EnumValue::ComparisonIntervalIncludingUpperBound,
        "ИнтервалВключаяОкончание",
        "IntervalIncludingUpperBound",
        "Range (>, <=)",
    ),
    (
        EnumKind::ComparisonType,
        EnumValue::ComparisonLess,
        "Меньше",
        "Less",
        "Less than",
    ),
    (
        EnumKind::ComparisonType,
        EnumValue::ComparisonLessOrEqual,
        "МеньшеИлиРавно",
        "LessOrEqual",
        "Less than or equal to",
    ),
    (
        EnumKind::ComparisonType,
        EnumValue::ComparisonNotInHierarchy,
        "НеВИерархии",
        "NotInHierarchy",
        "Not in group",
    ),
    (
        EnumKind::ComparisonType,
        EnumValue::ComparisonNotInList,
        "НеВСписке",
        "NotInList",
        "Not in list",
    ),
    (
        EnumKind::ComparisonType,
        EnumValue::ComparisonNotInListByHierarchy,
        "НеВСпискеПоИерархии",
        "NotInListByHierarchy",
        "Not in list group",
    ),
    (
        EnumKind::ComparisonType,
        EnumValue::ComparisonNotEqual,
        "НеРавно",
        "NotEqual",
        "Not equal to",
    ),
    (
        EnumKind::ComparisonType,
        EnumValue::ComparisonNotContains,
        "НеСодержит",
        "NotContains",
        "Does not contain",
    ),
    (
        EnumKind::ComparisonType,
        EnumValue::ComparisonEqual,
        "Равно",
        "Equal",
        "Equal to",
    ),
    (
        EnumKind::ComparisonType,
        EnumValue::ComparisonContains,
        "Содержит",
        "Contains",
        "Contains",
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
    // Полный состав, оба написания, представления и типы измерены
    // на платформе 8.3.27.2342 пробой `measure-client-settings.bsl`.
    (
        EnumKind::ApplicationFormsOpenningMode,
        EnumValue::ApplicationFormsOpenningModeTabs,
        "Закладки",
        "Tabs",
        "Tabs",
    ),
    (
        EnumKind::ApplicationFormsOpenningMode,
        EnumValue::ApplicationFormsOpenningModeSingleWindows,
        "ОтдельныеОкна",
        "SingleWindows",
        "Separate windows",
    ),
    (
        EnumKind::ClientApplicationInterfaceVariant,
        EnumValue::ClientApplicationInterfaceVariantVersion8_2,
        "Версия8_2",
        "Version8_2",
        "Version 8.2",
    ),
    (
        EnumKind::ClientApplicationInterfaceVariant,
        EnumValue::ClientApplicationInterfaceVariantTaxi,
        "Такси",
        "Taxi",
        "Taxi",
    ),
    // Полный состав, оба написания, представления и тип измерены
    // на платформе 8.3.27.2342 пробой `measure-command-interface-settings.bsl`.
    (
        EnumKind::SectionsPanelRepresentation,
        EnumValue::SectionsPanelRepresentationPicture,
        "Картинка",
        "Picture",
        "Icon",
    ),
    (
        EnumKind::SectionsPanelRepresentation,
        EnumValue::SectionsPanelRepresentationPictureAndText,
        "КартинкаИТекст",
        "PictureAndText",
        "Icon and text",
    ),
    (
        EnumKind::SectionsPanelRepresentation,
        EnumValue::SectionsPanelRepresentationPictureOnTopAndText,
        "КартинкаСверхуИТекст",
        "PictureOnTopAndText",
        "Text below icon",
    ),
    (
        EnumKind::SectionsPanelRepresentation,
        EnumValue::SectionsPanelRepresentationPictureOnLeftAndText,
        "КартинкаСлеваИТекст",
        "PictureOnLeftAndText",
        "Text to the right of icon",
    ),
    (
        EnumKind::SectionsPanelRepresentation,
        EnumValue::SectionsPanelRepresentationText,
        "Текст",
        "Text",
        "Text",
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
    // Назначение доступно только через свойства метаданных; состав и
    // представления измерены контролем `configuration-extension-purpose`.
    (
        EnumKind::ConfigurationExtensionPurpose,
        EnumValue::ConfigurationExtensionPurposePatch,
        "Исправление",
        "Patch",
        "Patch",
    ),
    (
        EnumKind::ConfigurationExtensionPurpose,
        EnumValue::ConfigurationExtensionPurposeCustomization,
        "Адаптация",
        "Customization",
        "Customization",
    ),
    (
        EnumKind::ConfigurationExtensionPurpose,
        EnumValue::ConfigurationExtensionPurposeAddOn,
        "Дополнение",
        "AddOn",
        "AddOn",
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
    // Дополнительные члены измерены на 8.3.27 в
    // `measure-complete-spreadsheet-file-type.bsl`.
    (
        EnumKind::SpreadFileType,
        EnumValue::SpreadFileHtml,
        "HTML",
        "HTML",
        "HTML",
    ),
    (
        EnumKind::SpreadFileType,
        EnumValue::SpreadFileHtml3,
        "HTML3",
        "HTML3",
        "HTML3",
    ),
    (
        EnumKind::SpreadFileType,
        EnumValue::SpreadFileHtml4,
        "HTML4",
        "HTML4",
        "HTML4",
    ),
    (
        EnumKind::SpreadFileType,
        EnumValue::SpreadFileMxl7,
        "MXL7",
        "MXL7",
        "MXL7",
    ),
    (
        EnumKind::SpreadFileType,
        EnumValue::SpreadFilePdfA1,
        "PDF_A_1",
        "PDF_A_1",
        "PDF_A_1",
    ),
    (
        EnumKind::SpreadFileType,
        EnumValue::SpreadFilePdfA2,
        "PDF_A_2",
        "PDF_A_2",
        "PDF_A_2",
    ),
    (
        EnumKind::SpreadFileType,
        EnumValue::SpreadFilePdfA3,
        "PDF_A_3",
        "PDF_A_3",
        "PDF_A_3",
    ),
    (
        EnumKind::SpreadFileType,
        EnumValue::SpreadFileXls95,
        "XLS95",
        "XLS95",
        "XLS95",
    ),
    (
        EnumKind::SpreadFileType,
        EnumValue::SpreadFileXls97,
        "XLS97",
        "XLS97",
        "XLS97",
    ),
    (
        EnumKind::DrawingKind,
        EnumValue::DrawingGeographicalSchema,
        "ГеографическаяСхема",
        "GeographicalSchema",
        "Geographical schema",
    ),
    (
        EnumKind::DrawingKind,
        EnumValue::DrawingGroup,
        "Группа",
        "Group",
        "Folder",
    ),
    (
        EnumKind::DrawingKind,
        EnumValue::DrawingDendrogram,
        "Дендрограмма",
        "Dendrogram",
        "Dendrogram",
    ),
    (
        EnumKind::DrawingKind,
        EnumValue::DrawingChart,
        "Диаграмма",
        "Chart",
        "Chart",
    ),
    (
        EnumKind::DrawingKind,
        EnumValue::DrawingGanttChart,
        "ДиаграммаГанта",
        "GanttChart",
        "Gantt chart",
    ),
    (
        EnumKind::DrawingKind,
        EnumValue::DrawingPicture,
        "Картинка",
        "Picture",
        "Picture",
    ),
    (
        EnumKind::DrawingKind,
        EnumValue::DrawingObject,
        "Объект",
        "Object",
        "Object",
    ),
    (
        EnumKind::DrawingKind,
        EnumValue::DrawingComment,
        "Примечание",
        "Comment",
        "Comment",
    ),
    (
        EnumKind::DrawingKind,
        EnumValue::DrawingLine,
        "Прямая",
        "Line",
        "Line",
    ),
    (
        EnumKind::DrawingKind,
        EnumValue::DrawingRectangle,
        "Прямоугольник",
        "Rectangle",
        "Rectangle",
    ),
    (
        EnumKind::DrawingKind,
        EnumValue::DrawingPivotChart,
        "СводнаяДиаграмма",
        "PivotChart",
        "Pivot chart",
    ),
    (
        EnumKind::DrawingKind,
        EnumValue::DrawingText,
        "Текст",
        "Text",
        "Text",
    ),
    (
        EnumKind::DrawingKind,
        EnumValue::DrawingEllipse,
        "Эллипс",
        "Ellipse",
        "Ellipse",
    ),
    // Платформа печатает `3D` и `Diamond`, а не имена членов.
    (
        EnumKind::GanttChartIntervalRepresentation,
        EnumValue::GanttChartIntervalRepresentationFlat,
        "Плоский",
        "Flat",
        "Flat",
    ),
    (
        EnumKind::GanttChartIntervalRepresentation,
        EnumValue::GanttChartIntervalRepresentationThreeDimensional,
        "Объемный",
        "ThreeDimensional",
        "3D",
    ),
    (
        EnumKind::GanttChartIntervalRepresentation,
        EnumValue::GanttChartIntervalRepresentationRhomb,
        "Ромб",
        "Rhomb",
        "Diamond",
    ),
    (
        EnumKind::GanttChartIntervalRepresentation,
        EnumValue::GanttChartIntervalRepresentationGradient,
        "Градиент",
        "Gradient",
        "Gradient",
    ),
    // Представление `Use coordinates` измерено, а не построено из имени.
    (
        EnumKind::ChartLegendPlacement,
        EnumValue::ChartLegendPlacementAuto,
        "Авто",
        "Auto",
        "Auto",
    ),
    (
        EnumKind::ChartLegendPlacement,
        EnumValue::ChartLegendPlacementBottom,
        "Низ",
        "Bottom",
        "Bottom",
    ),
    (
        EnumKind::ChartLegendPlacement,
        EnumValue::ChartLegendPlacementTop,
        "Верх",
        "Top",
        "Top",
    ),
    (
        EnumKind::ChartLegendPlacement,
        EnumValue::ChartLegendPlacementLeft,
        "Лево",
        "Left",
        "Left",
    ),
    (
        EnumKind::ChartLegendPlacement,
        EnumValue::ChartLegendPlacementRight,
        "Право",
        "Right",
        "Right",
    ),
    (
        EnumKind::ChartLegendPlacement,
        EnumValue::ChartLegendPlacementNone,
        "Нет",
        "None",
        "None",
    ),
    (
        EnumKind::ChartLegendPlacement,
        EnumValue::ChartLegendPlacementUseCoordinates,
        "УказываетсяРасположение",
        "UseCoordinates",
        "Use coordinates",
    ),
    // `НеРастягивать` печатается как `No`; строка взята из нативного замера.
    (
        EnumKind::GanttChartVerticalStretch,
        EnumValue::GanttChartVerticalStretchNone,
        "НеРастягивать",
        "None",
        "No",
    ),
    (
        EnumKind::GanttChartVerticalStretch,
        EnumValue::GanttChartVerticalStretchStretchRows,
        "РастягиватьСтроки",
        "StretchRows",
        "Stretch rows",
    ),
    (
        EnumKind::GanttChartVerticalStretch,
        EnumValue::GanttChartVerticalStretchStretchRowsAndData,
        "РастягиватьСтрокиИДанные",
        "StretchRowsAndData",
        "Stretch rows and data",
    ),
    // Все восемь представлений измерены на платформе 8.3.27.2342.
    (
        EnumKind::TimeScaleUnitType,
        EnumValue::TimeScaleUnitTypeSecond,
        "Секунда",
        "Second",
        "Second",
    ),
    (
        EnumKind::TimeScaleUnitType,
        EnumValue::TimeScaleUnitTypeMinute,
        "Минута",
        "Minute",
        "Minute",
    ),
    (
        EnumKind::TimeScaleUnitType,
        EnumValue::TimeScaleUnitTypeHour,
        "Час",
        "Hour",
        "Hour",
    ),
    (
        EnumKind::TimeScaleUnitType,
        EnumValue::TimeScaleUnitTypeDay,
        "День",
        "Day",
        "Day",
    ),
    (
        EnumKind::TimeScaleUnitType,
        EnumValue::TimeScaleUnitTypeWeek,
        "Неделя",
        "Week",
        "Week",
    ),
    (
        EnumKind::TimeScaleUnitType,
        EnumValue::TimeScaleUnitTypeMonth,
        "Месяц",
        "Month",
        "Month",
    ),
    (
        EnumKind::TimeScaleUnitType,
        EnumValue::TimeScaleUnitTypeQuarter,
        "Квартал",
        "Quarter",
        "Quarter",
    ),
    (
        EnumKind::TimeScaleUnitType,
        EnumValue::TimeScaleUnitTypeYear,
        "Год",
        "Year",
        "Year",
    ),
    // Представления, включая порядок частей, взяты из нативного замера.
    (
        EnumKind::TimeScaleDayFormat,
        EnumValue::TimeScaleDayFormatMonthDay,
        "ДеньМесяца",
        "MonthDay",
        "Day of month",
    ),
    (
        EnumKind::TimeScaleDayFormat,
        EnumValue::TimeScaleDayFormatWeekDay,
        "ДеньНедели",
        "WeekDay",
        "Day of week",
    ),
    (
        EnumKind::TimeScaleDayFormat,
        EnumValue::TimeScaleDayFormatMonthDayWeekDay,
        "ДеньМесяцаДеньНедели",
        "MonthDayWeekDay",
        "Day of month + day of week",
    ),
    (
        EnumKind::TimeScaleDayFormat,
        EnumValue::TimeScaleDayFormatWeekDayMonthDay,
        "ДеньНеделиДеньМесяца",
        "WeekDayMonthDay",
        "Day of week + day of month",
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
        "Asc",
        "Ascending",
    ),
    (
        EnumKind::SortDirection,
        EnumValue::SortDirectionDescending,
        "Убыв",
        "Desc",
        "Descending",
    ),
    // Представления и псевдонимы измерены на 8.3.27.2342.
    (
        EnumKind::UseOutput,
        EnumValue::UseOutputAuto,
        "Авто",
        "Auto",
        "Auto",
    ),
    (
        EnumKind::UseOutput,
        EnumValue::UseOutputEnable,
        "Разрешить",
        "Enable",
        "Enable",
    ),
    (
        EnumKind::UseOutput,
        EnumValue::UseOutputDisable,
        "Запретить",
        "Disable",
        "Disable",
    ),
    // Все члены и представления измерены на 8.3.27.2342.
    (
        EnumKind::FormPagesRepresentation,
        EnumValue::FormPagesRepresentationAuto,
        "Авто",
        "Auto",
        "Auto",
    ),
    (
        EnumKind::FormPagesRepresentation,
        EnumValue::FormPagesRepresentationNone,
        "Нет",
        "None",
        "None",
    ),
    (
        EnumKind::FormPagesRepresentation,
        EnumValue::FormPagesRepresentationSwipe,
        "Пролистывание",
        "Swipe",
        "Swipe (touchscreen)",
    ),
    (
        EnumKind::FormPagesRepresentation,
        EnumValue::FormPagesRepresentationTabsOnTop,
        "ЗакладкиСверху",
        "TabsOnTop",
        "Tabs above",
    ),
    (
        EnumKind::FormPagesRepresentation,
        EnumValue::FormPagesRepresentationTabsOnBottom,
        "ЗакладкиСнизу",
        "TabsOnBottom",
        "Tabs below",
    ),
    (
        EnumKind::FormPagesRepresentation,
        EnumValue::FormPagesRepresentationTabsOnLeftHorizontal,
        "ЗакладкиСлеваГоризонтально",
        "TabsOnLeftHorizontal",
        "Tabs on the left (horizontally)",
    ),
    (
        EnumKind::FormPagesRepresentation,
        EnumValue::FormPagesRepresentationTabsOnRightHorizontal,
        "ЗакладкиСправаГоризонтально",
        "TabsOnRightHorizontal",
        "Tabs on the right (horizontally)",
    ),
    // Все члены и представления измерены на 8.3.27.2342.
    (
        EnumKind::FormButtonPictureLocation,
        EnumValue::FormButtonPictureLocationAuto,
        "Авто",
        "Auto",
        "Auto",
    ),
    (
        EnumKind::FormButtonPictureLocation,
        EnumValue::FormButtonPictureLocationLeft,
        "Лево",
        "Left",
        "Left",
    ),
    (
        EnumKind::FormButtonPictureLocation,
        EnumValue::FormButtonPictureLocationRight,
        "Право",
        "Right",
        "Right",
    ),
    // Все члены и представления измерены на 8.3.27.2342.
    (
        EnumKind::SpreadsheetDocumentDrawingLineType,
        EnumValue::SpreadsheetDocumentDrawingLineTypeNone,
        "НетЛинии",
        "None",
        "None",
    ),
    (
        EnumKind::SpreadsheetDocumentDrawingLineType,
        EnumValue::SpreadsheetDocumentDrawingLineTypeSolid,
        "Сплошная",
        "Solid",
        "Solid",
    ),
    (
        EnumKind::SpreadsheetDocumentDrawingLineType,
        EnumValue::SpreadsheetDocumentDrawingLineTypeDashed,
        "Пунктир",
        "Dashed",
        "Dashed",
    ),
    (
        EnumKind::SpreadsheetDocumentDrawingLineType,
        EnumValue::SpreadsheetDocumentDrawingLineTypeDotted,
        "Точечная",
        "Dotted",
        "Dotted",
    ),
    (
        EnumKind::SpreadsheetDocumentDrawingLineType,
        EnumValue::SpreadsheetDocumentDrawingLineTypeDashDotted,
        "ПунктирТочка",
        "DashDotted",
        "DashDotted",
    ),
    (
        EnumKind::SpreadsheetDocumentDrawingLineType,
        EnumValue::SpreadsheetDocumentDrawingLineTypeDashDottedDotted,
        "ПунктирТочкаТочка",
        "DashDottedDotted",
        "DotDashDotted",
    ),
    // Все члены и представления измерены на 8.3.27.2342.
    (
        EnumKind::CheckBoxType,
        EnumValue::CheckBoxTypeAuto,
        "Авто",
        "Auto",
        "Auto",
    ),
    (
        EnumKind::CheckBoxType,
        EnumValue::CheckBoxTypeCheckBox,
        "Флажок",
        "CheckBox",
        "Checkbox",
    ),
    (
        EnumKind::CheckBoxType,
        EnumValue::CheckBoxTypeSwitch,
        "Выключатель",
        "Switch",
        "Switch",
    ),
    (
        EnumKind::CheckBoxType,
        EnumValue::CheckBoxTypeTumbler,
        "Тумблер",
        "Tumbler",
        "Tumblers",
    ),
    // Представления и псевдонимы измерены на 8.3.27.2342; `APNS` печатается как `APNs`.
    (
        EnumKind::DeliverableNotificationSubscriberType,
        EnumValue::DeliverableNotificationSubscriberTypeGCM,
        "GCM",
        "GCM",
        "GCM",
    ),
    (
        EnumKind::DeliverableNotificationSubscriberType,
        EnumValue::DeliverableNotificationSubscriberTypeAPNS,
        "APNS",
        "APNS",
        "APNs",
    ),
    (
        EnumKind::DeliverableNotificationSubscriberType,
        EnumValue::DeliverableNotificationSubscriberTypeFCM,
        "FCM",
        "FCM",
        "FCM",
    ),
    // Оба члена и представления измерены на 8.3.27.2342.
    (
        EnumKind::SoundAlert,
        EnumValue::SoundAlertNone,
        "Нет",
        "None",
        "None",
    ),
    (
        EnumKind::SoundAlert,
        EnumValue::SoundAlertDefault,
        "ПоУмолчанию",
        "Default",
        "Default",
    ),
    // Все члены и представления измерены на 8.3.27.2342.
    (
        EnumKind::ThroughAlign,
        EnumValue::ThroughAlignAuto,
        "Авто",
        "Auto",
        "Auto",
    ),
    (
        EnumKind::ThroughAlign,
        EnumValue::ThroughAlignUse,
        "Использовать",
        "Use",
        "Use",
    ),
    (
        EnumKind::ThroughAlign,
        EnumValue::ThroughAlignDontUse,
        "НеИспользовать",
        "DontUse",
        "Do not use",
    ),
    // Все члены и представления измерены на 8.3.27.2342.
    (
        EnumKind::ButtonShape,
        EnumValue::ButtonShapeAuto,
        "Авто",
        "Auto",
        "Auto",
    ),
    (
        EnumKind::ButtonShape,
        EnumValue::ButtonShapeUsual,
        "Обычная",
        "Usual",
        "Standard",
    ),
    (
        EnumKind::ButtonShape,
        EnumValue::ButtonShapeOval,
        "Овал",
        "Oval",
        "Rounded",
    ),
    // Все члены и представления измерены на 8.3.27.2342.
    (
        EnumKind::DataCompositionFilterItemsGroupType,
        EnumValue::DataCompositionFilterItemsGroupTypeAndGroup,
        "ГруппаИ",
        "AndGroup",
        "AND group",
    ),
    (
        EnumKind::DataCompositionFilterItemsGroupType,
        EnumValue::DataCompositionFilterItemsGroupTypeOrGroup,
        "ГруппаИли",
        "OrGroup",
        "OR group",
    ),
    (
        EnumKind::DataCompositionFilterItemsGroupType,
        EnumValue::DataCompositionFilterItemsGroupTypeNotGroup,
        "ГруппаНе",
        "NotGroup",
        "NOT group",
    ),
    // Достигнутые УНФ члены и представления измерены на 8.3.27.2342.
    (EnumKind::Key, EnumValue::KeyDigit1, "_1", "_1", "_1"),
    (EnumKind::Key, EnumValue::KeyDigit2, "_2", "_2", "_2"),
    (EnumKind::Key, EnumValue::KeyDigit3, "_3", "_3", "_3"),
    (EnumKind::Key, EnumValue::KeyDigit4, "_4", "_4", "_4"),
    (EnumKind::Key, EnumValue::KeyDigit5, "_5", "_5", "_5"),
    (EnumKind::Key, EnumValue::KeyDigit6, "_6", "_6", "_6"),
    (EnumKind::Key, EnumValue::KeyDigit7, "_7", "_7", "_7"),
    (EnumKind::Key, EnumValue::KeyDigit8, "_8", "_8", "_8"),
    (EnumKind::Key, EnumValue::KeyDigit9, "_9", "_9", "_9"),
    (EnumKind::Key, EnumValue::KeyNum0, "Num0", "Num0", "Num0"),
    (EnumKind::Key, EnumValue::KeyNum1, "Num1", "Num1", "Num1"),
    (EnumKind::Key, EnumValue::KeyNum2, "Num2", "Num2", "Num2"),
    (EnumKind::Key, EnumValue::KeyNum3, "Num3", "Num3", "Num3"),
    (EnumKind::Key, EnumValue::KeyNum4, "Num4", "Num4", "Num4"),
    (EnumKind::Key, EnumValue::KeyNum5, "Num5", "Num5", "Num5"),
    (EnumKind::Key, EnumValue::KeyNum6, "Num6", "Num6", "Num6"),
    (EnumKind::Key, EnumValue::KeyNum7, "Num7", "Num7", "Num7"),
    (EnumKind::Key, EnumValue::KeyNum8, "Num8", "Num8", "Num8"),
    (EnumKind::Key, EnumValue::KeyNum9, "Num9", "Num9", "Num9"),
    (
        EnumKind::Key,
        EnumValue::KeyNumAdd,
        "NumAdd",
        "NumAdd",
        "NumAdd",
    ),
    (
        EnumKind::Key,
        EnumValue::KeyNumDecimal,
        "NumDecimal",
        "NumDecimal",
        "NumDecimal",
    ),
    (
        EnumKind::Key,
        EnumValue::KeyNumDivide,
        "NumDivide",
        "NumDivide",
        "NumDivide",
    ),
    (
        EnumKind::Key,
        EnumValue::KeyNumMultiply,
        "NumMultiply",
        "NumMultiply",
        "NumMultiply",
    ),
    (
        EnumKind::Key,
        EnumValue::KeyNumSubtract,
        "NumSubtract",
        "NumSubtract",
        "NumSubtract",
    ),
    (EnumKind::Key, EnumValue::KeyA, "A", "A", "A"),
    (EnumKind::Key, EnumValue::KeyS, "S", "S", "S"),
    (EnumKind::Key, EnumValue::KeyV, "V", "V", "V"),
    // Достигнутые члены и представления измерены на 8.3.27.2342.
    (
        EnumKind::ControlBorderType,
        EnumValue::ControlBorderTypeWithoutBorder,
        "БезРамки",
        "WithoutBorder",
        "No border",
    ),
    (
        EnumKind::ControlBorderType,
        EnumValue::ControlBorderTypeSingle,
        "Одинарная",
        "Single",
        "Single",
    ),
    (
        EnumKind::ControlBorderType,
        EnumValue::ControlBorderTypeUnderline,
        "Подчеркивание",
        "Underline",
        "Underline",
    ),
    // Представления и псевдонимы измерены на 1С 8.3.27.2342.
    (
        EnumKind::ColorType,
        EnumValue::ColorTypeAbsolute,
        "Абсолютный",
        "Absolute",
        "Absolute",
    ),
    (
        EnumKind::ColorType,
        EnumValue::ColorTypeStyleItem,
        "ЭлементСтиля",
        "StyleItem",
        "Style element",
    ),
    (
        EnumKind::ColorType,
        EnumValue::ColorTypeWebColor,
        "WebЦвет",
        "WebColor",
        "Web color",
    ),
    (
        EnumKind::ColorType,
        EnumValue::ColorTypeWindowsColor,
        "WindowsЦвет",
        "WindowsColor",
        "Windows color",
    ),
    (
        EnumKind::ColorType,
        EnumValue::ColorTypeAutoColor,
        "АвтоЦвет",
        "AutoColor",
        "Auto color",
    ),
    // Представления и оба написания измерены на 1С 8.3.27.2342.
    (
        EnumKind::SpreadsheetDocumentCellLineType,
        EnumValue::SpreadsheetDocumentCellLineTypeLargeDashed,
        "БольшойПунктир",
        "LargeDashed",
        "LargeDashed",
    ),
    (
        EnumKind::SpreadsheetDocumentCellLineType,
        EnumValue::SpreadsheetDocumentCellLineTypeDouble,
        "Двойная",
        "Double",
        "Double",
    ),
    (
        EnumKind::SpreadsheetDocumentCellLineType,
        EnumValue::SpreadsheetDocumentCellLineTypeNone,
        "НетЛинии",
        "None",
        "None",
    ),
    (
        EnumKind::SpreadsheetDocumentCellLineType,
        EnumValue::SpreadsheetDocumentCellLineTypeThinDashed,
        "РедкийПунктир",
        "ThinDashed",
        "ThinDashed",
    ),
    (
        EnumKind::SpreadsheetDocumentCellLineType,
        EnumValue::SpreadsheetDocumentCellLineTypeSolid,
        "Сплошная",
        "Solid",
        "Solid",
    ),
    (
        EnumKind::SpreadsheetDocumentCellLineType,
        EnumValue::SpreadsheetDocumentCellLineTypeDotted,
        "Точечная",
        "Dotted",
        "Dotted",
    ),
    // Представления и оба написания измерены на 1С 8.3.27.2342.
    (
        EnumKind::CryptoCertificateStoreType,
        EnumValue::CryptoCertificateStoreTypeRootCertificates,
        "КорневыеСертификаты",
        "RootCertificates",
        "Root certificates",
    ),
    (
        EnumKind::CryptoCertificateStoreType,
        EnumValue::CryptoCertificateStoreTypePersonalCertificates,
        "ПерсональныеСертификаты",
        "PersonalCertificates",
        "Personal certificates",
    ),
    (
        EnumKind::CryptoCertificateStoreType,
        EnumValue::CryptoCertificateStoreTypeRecipientCertificates,
        "СертификатыПолучателей",
        "RecipientCertificates",
        "Recipient certificates",
    ),
    // Представления и псевдонимы измерены на 1С 8.3.27.2342.
    (
        EnumKind::InternetMailTextProcessing,
        EnumValue::InternetMailTextProcessingDontProcess,
        "НеОбрабатывать",
        "DontProcess",
        "DontProcess",
    ),
    (
        EnumKind::InternetMailTextProcessing,
        EnumValue::InternetMailTextProcessingProcess,
        "Обрабатывать",
        "Process",
        "Process",
    ),
    // Достигнутое представление измерено на 1С 8.3.27.2342.
    (
        EnumKind::DuplexPrintingType,
        EnumValue::DuplexPrintingTypeNone,
        "Нет",
        "None",
        "None",
    ),
    // Оба значения и представления измерены на 1С 8.3.27.2342.
    (
        EnumKind::SecondAuthenticationFactorSettingsProcessingType,
        EnumValue::SecondAuthenticationFactorSettingsProcessingDontUse,
        "НеИспользовать",
        "DontUse",
        "НеИспользовать",
    ),
    (
        EnumKind::SecondAuthenticationFactorSettingsProcessingType,
        EnumValue::SecondAuthenticationFactorSettingsProcessingUseNextIfFailed,
        "ИспользоватьСледующуюПриОшибке",
        "UseNextIfFailed",
        "ИспользоватьСледующуюПриОшибке",
    ),
    // Полный состав и представления измерены на 1С 8.3.27.2342.
    (
        EnumKind::FileDragMode,
        EnumValue::FileDragModeAsFileRef,
        "КакСсылкаНаФайл",
        "AsFileRef",
        "As file reference",
    ),
    (
        EnumKind::FileDragMode,
        EnumValue::FileDragModeAsFile,
        "КакФайл",
        "AsFile",
        "As file",
    ),
    // Полный состав и представления измерены на 1С 8.3.27.2342.
    (
        EnumKind::ButtonGroupRepresentation,
        EnumValue::ButtonGroupRepresentationAuto,
        "Авто",
        "Auto",
        "Auto",
    ),
    (
        EnumKind::ButtonGroupRepresentation,
        EnumValue::ButtonGroupRepresentationUsual,
        "Обычное",
        "Usual",
        "Standard",
    ),
    (
        EnumKind::ButtonGroupRepresentation,
        EnumValue::ButtonGroupRepresentationCompact,
        "Компактное",
        "Compact",
        "Compact",
    ),
    // Достигнутые члены измерены на платформе 1С 8.3.27.2342.
    (
        EnumKind::StandardPeriodVariant,
        EnumValue::StandardPeriodVariantYesterday,
        "Вчера",
        "Yesterday",
        "Yesterday",
    ),
    (
        EnumKind::StandardPeriodVariant,
        EnumValue::StandardPeriodVariantTillEndOfThisYear,
        "ДоКонцаЭтогоГода",
        "TillEndOfThisYear",
        "Till the end of this year",
    ),
    (
        EnumKind::StandardPeriodVariant,
        EnumValue::StandardPeriodVariantTillEndOfThisQuarter,
        "ДоКонцаЭтогоКвартала",
        "TillEndOfThisQuarter",
        "Till the end of this quarter",
    ),
    (
        EnumKind::StandardPeriodVariant,
        EnumValue::StandardPeriodVariantTillEndOfThisMonth,
        "ДоКонцаЭтогоМесяца",
        "TillEndOfThisMonth",
        "Till the end of this month",
    ),
    (
        EnumKind::StandardPeriodVariant,
        EnumValue::StandardPeriodVariantTillEndOfThisHalfYear,
        "ДоКонцаЭтогоПолугодия",
        "TillEndOfThisHalfYear",
        "Till the end of this half year",
    ),
    (
        EnumKind::StandardPeriodVariant,
        EnumValue::StandardPeriodVariantTillEndOfThisTenDays,
        "ДоКонцаЭтойДекады",
        "TillEndOfThisTenDays",
        "Till the end of this ten-day period",
    ),
    (
        EnumKind::StandardPeriodVariant,
        EnumValue::StandardPeriodVariantTillEndOfThisWeek,
        "ДоКонцаЭтойНедели",
        "TillEndOfThisWeek",
        "Till the end of this week",
    ),
    (
        EnumKind::StandardPeriodVariant,
        EnumValue::StandardPeriodVariantTomorrow,
        "Завтра",
        "Tomorrow",
        "Tomorrow",
    ),
    (
        EnumKind::StandardPeriodVariant,
        EnumValue::StandardPeriodVariantMonth,
        "Месяц",
        "Month",
        "Month",
    ),
    (
        EnumKind::StandardPeriodVariant,
        EnumValue::StandardPeriodVariantLast7Days,
        "Последние7Дней",
        "Last7Days",
        "Last 7 days",
    ),
    (
        EnumKind::StandardPeriodVariant,
        EnumValue::StandardPeriodVariantCustom,
        "ПроизвольныйПериод",
        "Custom",
        "Custom period",
    ),
    (
        EnumKind::StandardPeriodVariant,
        EnumValue::StandardPeriodVariantLastTenDays,
        "ПрошлаяДекада",
        "LastTenDays",
        "Last ten-day period",
    ),
    (
        EnumKind::StandardPeriodVariant,
        EnumValue::StandardPeriodVariantLastTenDaysTillSameDayNumber,
        "ПрошлаяДекадаДоТакогоЖеНомераДня",
        "LastTenDaysTillSameDayNumber",
        "Last ten-day period till the same day number",
    ),
    (
        EnumKind::StandardPeriodVariant,
        EnumValue::StandardPeriodVariantLastWeek,
        "ПрошлаяНеделя",
        "LastWeek",
        "Last week",
    ),
    (
        EnumKind::StandardPeriodVariant,
        EnumValue::StandardPeriodVariantLastWeekTillSameWeekDay,
        "ПрошлаяНеделяДоТакогоЖеДняНедели",
        "LastWeekTillSameWeekDay",
        "Last week till the same day of the week",
    ),
    (
        EnumKind::StandardPeriodVariant,
        EnumValue::StandardPeriodVariantLastHalfYear,
        "ПрошлоеПолугодие",
        "LastHalfYear",
        "Last half year",
    ),
    (
        EnumKind::StandardPeriodVariant,
        EnumValue::StandardPeriodVariantLastHalfYearTillSameDate,
        "ПрошлоеПолугодиеДоТакойЖеДаты",
        "LastHalfYearTillSameDate",
        "Last half year till the same date",
    ),
    (
        EnumKind::StandardPeriodVariant,
        EnumValue::StandardPeriodVariantLastYear,
        "ПрошлыйГод",
        "LastYear",
        "Last year",
    ),
    (
        EnumKind::StandardPeriodVariant,
        EnumValue::StandardPeriodVariantLastYearTillSameDate,
        "ПрошлыйГодДоТакойЖеДаты",
        "LastYearTillSameDate",
        "Last year till the same date",
    ),
    (
        EnumKind::StandardPeriodVariant,
        EnumValue::StandardPeriodVariantLastQuarter,
        "ПрошлыйКвартал",
        "LastQuarter",
        "Last quarter",
    ),
    (
        EnumKind::StandardPeriodVariant,
        EnumValue::StandardPeriodVariantLastQuarterTillSameDate,
        "ПрошлыйКварталДоТакойЖеДаты",
        "LastQuarterTillSameDate",
        "Last quarter till the same date",
    ),
    (
        EnumKind::StandardPeriodVariant,
        EnumValue::StandardPeriodVariantLastMonth,
        "ПрошлыйМесяц",
        "LastMonth",
        "Last month",
    ),
    (
        EnumKind::StandardPeriodVariant,
        EnumValue::StandardPeriodVariantLastMonthTillSameDate,
        "ПрошлыйМесяцДоТакойЖеДаты",
        "LastMonthTillSameDate",
        "Last month till the same date",
    ),
    (
        EnumKind::StandardPeriodVariant,
        EnumValue::StandardPeriodVariantFromBeginningOfThisYear,
        "СНачалаЭтогоГода",
        "FromBeginningOfThisYear",
        "From the beginning of this year",
    ),
    (
        EnumKind::StandardPeriodVariant,
        EnumValue::StandardPeriodVariantFromBeginningOfThisQuarter,
        "СНачалаЭтогоКвартала",
        "FromBeginningOfThisQuarter",
        "From the beginning of this quarter",
    ),
    (
        EnumKind::StandardPeriodVariant,
        EnumValue::StandardPeriodVariantFromBeginningOfThisMonth,
        "СНачалаЭтогоМесяца",
        "FromBeginningOfThisMonth",
        "From the beginning of this month",
    ),
    (
        EnumKind::StandardPeriodVariant,
        EnumValue::StandardPeriodVariantFromBeginningOfThisHalfYear,
        "СНачалаЭтогоПолугодия",
        "FromBeginningOfThisHalfYear",
        "From the beginning of this half year",
    ),
    (
        EnumKind::StandardPeriodVariant,
        EnumValue::StandardPeriodVariantFromBeginningOfThisTenDays,
        "СНачалаЭтойДекады",
        "FromBeginningOfThisTenDays",
        "From the beginning of this ten-day period",
    ),
    (
        EnumKind::StandardPeriodVariant,
        EnumValue::StandardPeriodVariantFromBeginningOfThisWeek,
        "СНачалаЭтойНедели",
        "FromBeginningOfThisWeek",
        "From the beginning of this week",
    ),
    (
        EnumKind::StandardPeriodVariant,
        EnumValue::StandardPeriodVariantToday,
        "Сегодня",
        "Today",
        "Today",
    ),
    (
        EnumKind::StandardPeriodVariant,
        EnumValue::StandardPeriodVariantNextTenDays,
        "СледующаяДекада",
        "NextTenDays",
        "Next ten-day period",
    ),
    (
        EnumKind::StandardPeriodVariant,
        EnumValue::StandardPeriodVariantNextTenDaysTillSameDayNumber,
        "СледующаяДекадаДоТакогоЖеНомераДня",
        "NextTenDaysTillSameDayNumber",
        "Next ten-day period till the same day number",
    ),
    (
        EnumKind::StandardPeriodVariant,
        EnumValue::StandardPeriodVariantNextWeek,
        "СледующаяНеделя",
        "NextWeek",
        "Next week",
    ),
    (
        EnumKind::StandardPeriodVariant,
        EnumValue::StandardPeriodVariantNextWeekTillSameWeekDay,
        "СледующаяНеделяДоТакогоЖеДняНедели",
        "NextWeekTillSameWeekDay",
        "Next week till the same day of the week",
    ),
    (
        EnumKind::StandardPeriodVariant,
        EnumValue::StandardPeriodVariantNextHalfYear,
        "СледующееПолугодие",
        "NextHalfYear",
        "Next half year",
    ),
    (
        EnumKind::StandardPeriodVariant,
        EnumValue::StandardPeriodVariantNextHalfYearTillSameDate,
        "СледующееПолугодиеДоТакойЖеДаты",
        "NextHalfYearTillSameDate",
        "Next half year till the same date",
    ),
    (
        EnumKind::StandardPeriodVariant,
        EnumValue::StandardPeriodVariantNext7Days,
        "Следующие7Дней",
        "Next7Days",
        "Next 7 days",
    ),
    (
        EnumKind::StandardPeriodVariant,
        EnumValue::StandardPeriodVariantNextYear,
        "СледующийГод",
        "NextYear",
        "Next year",
    ),
    (
        EnumKind::StandardPeriodVariant,
        EnumValue::StandardPeriodVariantNextYearTillSameDate,
        "СледующийГодДоТакойЖеДаты",
        "NextYearTillSameDate",
        "Next year till the same date",
    ),
    (
        EnumKind::StandardPeriodVariant,
        EnumValue::StandardPeriodVariantNextQuarter,
        "СледующийКвартал",
        "NextQuarter",
        "Next quarter",
    ),
    (
        EnumKind::StandardPeriodVariant,
        EnumValue::StandardPeriodVariantNextQuarterTillSameDate,
        "СледующийКварталДоТакойЖеДаты",
        "NextQuarterTillSameDate",
        "Next quarter till the same date",
    ),
    (
        EnumKind::StandardPeriodVariant,
        EnumValue::StandardPeriodVariantNextMonth,
        "СледующийМесяц",
        "NextMonth",
        "Next month",
    ),
    (
        EnumKind::StandardPeriodVariant,
        EnumValue::StandardPeriodVariantNextMonthTillSameDate,
        "СледующийМесяцДоТакойЖеДаты",
        "NextMonthTillSameDate",
        "Next month till the same date",
    ),
    (
        EnumKind::StandardPeriodVariant,
        EnumValue::StandardPeriodVariantThisTenDays,
        "ЭтаДекада",
        "ThisTenDays",
        "This ten-day period",
    ),
    (
        EnumKind::StandardPeriodVariant,
        EnumValue::StandardPeriodVariantThisWeek,
        "ЭтаНеделя",
        "ThisWeek",
        "This week",
    ),
    (
        EnumKind::StandardPeriodVariant,
        EnumValue::StandardPeriodVariantThisHalfYear,
        "ЭтоПолугодие",
        "ThisHalfYear",
        "This half year",
    ),
    (
        EnumKind::StandardPeriodVariant,
        EnumValue::StandardPeriodVariantThisYear,
        "ЭтотГод",
        "ThisYear",
        "This year",
    ),
    (
        EnumKind::StandardPeriodVariant,
        EnumValue::StandardPeriodVariantThisQuarter,
        "ЭтотКвартал",
        "ThisQuarter",
        "This quarter",
    ),
    (
        EnumKind::StandardPeriodVariant,
        EnumValue::StandardPeriodVariantThisMonth,
        "ЭтотМесяц",
        "ThisMonth",
        "This month",
    ),
    (
        EnumKind::TableHeightControlVariant,
        EnumValue::TableHeightControlVariantUseHeightInFormRows,
        "ВСтрокахФормы",
        "UseHeightInFormRows",
        "Use form row height",
    ),
    (
        EnumKind::FullTextSearchVersion,
        EnumValue::FullTextSearchVersionVersion1,
        "Версия1",
        "Version1",
        "Version1",
    ),
    (
        EnumKind::VerticalFormScroll,
        EnumValue::VerticalFormScrollUseIfNecessary,
        "ИспользоватьПриНеобходимости",
        "UseIfNecessary",
        "Use if necessary",
    ),
    (
        EnumKind::VerticalAlign,
        EnumValue::VerticalAlignTop,
        "Верх",
        "Top",
        "Align top",
    ),
    (
        EnumKind::VerticalAlign,
        EnumValue::VerticalAlignBottom,
        "Низ",
        "Bottom",
        "Align bottom",
    ),
    (
        EnumKind::VerticalAlign,
        EnumValue::VerticalAlignCenter,
        "Центр",
        "Center",
        "Align center",
    ),
    (
        EnumKind::TextPositionRelativeToPicture,
        EnumValue::TextPositionRelativeToPictureAuto,
        "Авто",
        "Auto",
        "Auto",
    ),
    (
        EnumKind::TextPositionRelativeToPicture,
        EnumValue::TextPositionRelativeToPictureOnTop,
        "Поверх",
        "OnTop",
        "On top",
    ),
    (
        EnumKind::TextPositionRelativeToPicture,
        EnumValue::TextPositionRelativeToPictureTop,
        "Сверху",
        "Top",
        "Top",
    ),
    (
        EnumKind::TextPositionRelativeToPicture,
        EnumValue::TextPositionRelativeToPictureLeft,
        "Слева",
        "Left",
        "Left",
    ),
    (
        EnumKind::TextPositionRelativeToPicture,
        EnumValue::TextPositionRelativeToPictureBottom,
        "Снизу",
        "Bottom",
        "Bottom",
    ),
    (
        EnumKind::TextPositionRelativeToPicture,
        EnumValue::TextPositionRelativeToPictureRight,
        "Справа",
        "Right",
        "Right",
    ),
    (
        EnumKind::AccountingRecordType,
        EnumValue::AccountingRecordTypeDebit,
        "Дебет",
        "Debit",
        "Debit",
    ),
    (
        EnumKind::AccountingRecordType,
        EnumValue::AccountingRecordTypeCredit,
        "Кредит",
        "Credit",
        "Credit",
    ),
    (
        EnumKind::AccumulationRecordType,
        EnumValue::AccumulationRecordTypeReceipt,
        "Приход",
        "Receipt",
        "Receipt",
    ),
    (
        EnumKind::AccumulationRecordType,
        EnumValue::AccumulationRecordTypeExpense,
        "Расход",
        "Expense",
        "Expense",
    ),
    (
        EnumKind::HorizontalAlign,
        EnumValue::HorizontalAlignAuto,
        "Авто",
        "Auto",
        "Auto",
    ),
    (
        EnumKind::HorizontalAlign,
        EnumValue::HorizontalAlignLeft,
        "Лево",
        "Left",
        "Align left",
    ),
    (
        EnumKind::HorizontalAlign,
        EnumValue::HorizontalAlignJustify,
        "ПоШирине",
        "Justify",
        "Justify",
    ),
    (
        EnumKind::HorizontalAlign,
        EnumValue::HorizontalAlignRight,
        "Право",
        "Right",
        "Align right",
    ),
    (
        EnumKind::HorizontalAlign,
        EnumValue::HorizontalAlignCenter,
        "Центр",
        "Center",
        "Align center",
    ),
    (
        EnumKind::ColumnsGroup,
        EnumValue::ColumnsGroupInCell,
        "ВЯчейке",
        "InCell",
        "In cell",
    ),
    (
        EnumKind::ColumnsGroup,
        EnumValue::ColumnsGroupVertical,
        "Вертикальная",
        "Vertical",
        "Vertical",
    ),
    (
        EnumKind::ColumnsGroup,
        EnumValue::ColumnsGroupHorizontal,
        "Горизонтальная",
        "Horizontal",
        "Horizontal",
    ),
    (
        EnumKind::AllowedMessageNo,
        EnumValue::AllowedMessageNoGreater,
        "Больший",
        "Greater",
        "Greater",
    ),
    (
        EnumKind::FoldersAndItemsUse,
        EnumValue::FoldersAndItemsUseFolders,
        "Группы",
        "Folders",
        "Groups",
    ),
    (
        EnumKind::FoldersAndItemsUse,
        EnumValue::FoldersAndItemsUseFoldersAndItems,
        "ГруппыИЭлементы",
        "FoldersAndItems",
        "Groups and items",
    ),
    (
        EnumKind::FoldersAndItemsUse,
        EnumValue::FoldersAndItemsUseItems,
        "Элементы",
        "Items",
        "Items",
    ),
    (
        EnumKind::DataCompositionParameterUse,
        EnumValue::DataCompositionParameterUseAuto,
        "Авто",
        "Auto",
        "Auto",
    ),
    (
        EnumKind::DataCompositionParameterUse,
        EnumValue::DataCompositionParameterUseAlways,
        "Всегда",
        "Always",
        "Always",
    ),
    (
        EnumKind::ScrollBarUse,
        EnumValue::ScrollBarUseAutoUse,
        "ИспользоватьАвтоматически",
        "AutoUse",
        "Use automatically",
    ),
    (
        EnumKind::ScrollBarUse,
        EnumValue::ScrollBarUseDontUse,
        "НеИспользовать",
        "DontUse",
        "None",
    ),
    (
        EnumKind::CurrentRowUse,
        EnumValue::CurrentRowUseAuto,
        "Авто",
        "Auto",
        "Auto",
    ),
    (
        EnumKind::QuerySchemaOrderDirection,
        EnumValue::QuerySchemaOrderDirectionAscending,
        "ПоВозрастанию",
        "Ascending",
        "Ascending",
    ),
    (
        EnumKind::QuerySchemaOrderDirection,
        EnumValue::QuerySchemaOrderDirectionDescending,
        "ПоУбыванию",
        "Descending",
        "Descending",
    ),
    (
        EnumKind::InitialTreeView,
        EnumValue::InitialTreeViewNoExpand,
        "НеРаскрывать",
        "NoExpand",
        "Do not expand",
    ),
    (
        EnumKind::InitialTreeView,
        EnumValue::InitialTreeViewExpandAllLevels,
        "РаскрыватьВсеУровни",
        "ExpandAllLevels",
        "Expand all levels",
    ),
    (
        EnumKind::UpdateOnDataChange,
        EnumValue::UpdateOnDataChangeAuto,
        "Авто",
        "Auto",
        "Auto",
    ),
    (
        EnumKind::EditTextUpdate,
        EnumValue::EditTextUpdateDontUse,
        "НеИспользовать",
        "DontUse",
        "Do not use",
    ),
    (
        EnumKind::EditTextUpdate,
        EnumValue::EditTextUpdateOnValueChange,
        "ПриИзмененииЗначения",
        "OnValueChange",
        "Upon value change",
    ),
    (
        EnumKind::WarningOnEditRepresentation,
        EnumValue::WarningOnEditRepresentationDontShow,
        "НеОтображать",
        "DontShow",
        "Do not show",
    ),
    (
        EnumKind::WarningOnEditRepresentation,
        EnumValue::WarningOnEditRepresentationShow,
        "Отображать",
        "Show",
        "Show",
    ),
    (
        EnumKind::SearchInTableOnInput,
        EnumValue::SearchInTableOnInputDontUse,
        "НеИспользовать",
        "DontUse",
        "Do not use",
    ),
    (
        EnumKind::FormCommandBarLabelLocation,
        EnumValue::FormCommandBarLabelLocationAuto,
        "Авто",
        "Auto",
        "Auto",
    ),
    (
        EnumKind::FormCommandBarLabelLocation,
        EnumValue::FormCommandBarLabelLocationNone,
        "Нет",
        "None",
        "None",
    ),
    (
        EnumKind::FormCommandBarLabelLocation,
        EnumValue::FormCommandBarLabelLocationBottom,
        "Низ",
        "Bottom",
        "Bottom",
    ),
    (
        EnumKind::FormItemCommandBarLabelLocation,
        EnumValue::FormItemCommandBarLabelLocationAuto,
        "Авто",
        "Auto",
        "Auto",
    ),
    (
        EnumKind::FormItemCommandBarLabelLocation,
        EnumValue::FormItemCommandBarLabelLocationTop,
        "Верх",
        "Top",
        "Top",
    ),
    (
        EnumKind::FormItemCommandBarLabelLocation,
        EnumValue::FormItemCommandBarLabelLocationNone,
        "Нет",
        "None",
        "None",
    ),
    (
        EnumKind::SpreadsheetDocumentDetailUse,
        EnumValue::SpreadsheetDocumentDetailUseWithoutProcessing,
        "БезОбработки",
        "WithoutProcessing",
        "No processing",
    ),
    (
        EnumKind::SpreadsheetDocumentDetailUse,
        EnumValue::SpreadsheetDocumentDetailUseCell,
        "Ячейка",
        "Cell",
        "Cell",
    ),
    (
        EnumKind::ShowInChartLegend,
        EnumValue::ShowInChartLegendAuto,
        "Авто",
        "Auto",
        "Auto",
    ),
    (
        EnumKind::ShowInChartLegend,
        EnumValue::ShowInChartLegendShow,
        "Отображать",
        "Show",
        "Show",
    ),
    (
        EnumKind::ShowInChartLegend,
        EnumValue::ShowInChartLegendDontShow,
        "НеОтображать",
        "DontShow",
        "Do not show",
    ),
    (
        EnumKind::ChartScaleLabelLocation,
        EnumValue::ChartScaleLabelLocationNone,
        "Нет",
        "None",
        "Do not show",
    ),
    (
        EnumKind::ChartLabelLocation,
        EnumValue::ChartLabelLocationEdgeAuto,
        "КрайАвто",
        "EdgeAuto",
        "Edge auto",
    ),
    (
        EnumKind::ChartLabelLocation,
        EnumValue::ChartLabelLocationEdgeInside,
        "КрайВнутри",
        "EdgeInside",
        "Edge inside",
    ),
    (
        EnumKind::GaugeChartValuesScaleLabelsLocation,
        EnumValue::GaugeChartValuesScaleLabelsLocationAtScale,
        "НаШкале",
        "AtScale",
        "On scale",
    ),
    (
        EnumKind::ViewStatusLocation,
        EnumValue::ViewStatusLocationTop,
        "Верх",
        "Top",
        "Top",
    ),
    (
        EnumKind::SearchStringLocation,
        EnumValue::SearchStringLocationAuto,
        "Авто",
        "Auto",
        "Auto",
    ),
    (
        EnumKind::SearchStringLocation,
        EnumValue::SearchStringLocationNone,
        "Нет",
        "None",
        "None",
    ),
    (
        EnumKind::SearchControlLocation,
        EnumValue::SearchControlLocationNone,
        "Нет",
        "None",
        "None",
    ),
    (
        EnumKind::AutoTimeMode,
        EnumValue::AutoTimeModeLast,
        "Последним",
        "Last",
        "Last",
    ),
    (
        EnumKind::AutoTimeMode,
        EnumValue::AutoTimeModeCurrentOrLast,
        "ТекущееИлиПоследним",
        "CurrentOrLast",
        "Current or last",
    ),
    (
        EnumKind::TableRowInputMode,
        EnumValue::TableRowInputModeEndOfList,
        "ВКонецСписка",
        "EndOfList",
        "End of list",
    ),
    (
        EnumKind::CryptoCertificateIncludeMode,
        EnumValue::CryptoCertificateIncludeModeIncludeWholeChain,
        "ВключатьПолнуюЦепочку",
        "IncludeWholeChain",
        "Include full chain",
    ),
    (
        EnumKind::CryptoCertificateIncludeMode,
        EnumValue::CryptoCertificateIncludeModeIncludeSubjectCertificate,
        "ВключатьСертификатСубъекта",
        "IncludeSubjectCertificate",
        "Include subject certificate",
    ),
    (
        EnumKind::CryptoCertificateIncludeMode,
        EnumValue::CryptoCertificateIncludeModeDontInclude,
        "НеВключать",
        "DontInclude",
        "Do not include",
    ),
    (
        EnumKind::TableRowSelectionMode,
        EnumValue::TableRowSelectionModeRow,
        "Строка",
        "Row",
        "String",
    ),
    (
        EnumKind::TableRowSelectionMode,
        EnumValue::TableRowSelectionModeCell,
        "Ячейка",
        "Cell",
        "Cell",
    ),
    (
        EnumKind::TableSelectionMode,
        EnumValue::TableSelectionModeSingleRow,
        "Одиночный",
        "SingleRow",
        "Single",
    ),
    (
        EnumKind::ReplacementMode,
        EnumValue::ReplacementModeAppend,
        "Добавление",
        "Append",
        "Add",
    ),
    (
        EnumKind::ReplacementMode,
        EnumValue::ReplacementModeReplace,
        "Замещение",
        "Replace",
        "Replace",
    ),
    (
        EnumKind::ReplacementMode,
        EnumValue::ReplacementModeUpdate,
        "Обновление",
        "Update",
        "Update",
    ),
    (
        EnumKind::ReplacementMode,
        EnumValue::ReplacementModeMerge,
        "Слияние",
        "Merge",
        "Merge",
    ),
    (
        EnumKind::ReplacementMode,
        EnumValue::ReplacementModeDelete,
        "Удаление",
        "Delete",
        "Delete",
    ),
    (
        EnumKind::SizeChangeMode,
        EnumValue::SizeChangeModeQuickChange,
        "БыстроеИзменение",
        "QuickChange",
        "Quick change",
    ),
    (
        EnumKind::SizeChangeMode,
        EnumValue::SizeChangeModeNormal,
        "Обычный",
        "Normal",
        "Normal",
    ),
    (
        EnumKind::FormWindowOpeningMode,
        EnumValue::FormWindowOpeningModeLockWholeInterface,
        "БлокироватьВесьИнтерфейс",
        "LockWholeInterface",
        "Block the entire interface",
    ),
    (
        EnumKind::FormWindowOpeningMode,
        EnumValue::FormWindowOpeningModeLockOwnerWindow,
        "БлокироватьОкноВладельца",
        "LockOwnerWindow",
        "Lock parent window",
    ),
    (
        EnumKind::FormWindowOpeningMode,
        EnumValue::FormWindowOpeningModeIndependent,
        "Независимый",
        "Independent",
        "Independent",
    ),
    (
        EnumKind::DataCompositionSettingsViewMode,
        EnumValue::DataCompositionSettingsViewModeQuickAccess,
        "БыстрыйДоступ",
        "QuickAccess",
        "Quick access",
    ),
    (
        EnumKind::DataCompositionSettingsViewMode,
        EnumValue::DataCompositionSettingsViewModeAll,
        "Все",
        "All",
        "All",
    ),
    (
        EnumKind::ReportResultViewMode,
        EnumValue::ReportResultViewModeAuto,
        "Авто",
        "Auto",
        "Auto",
    ),
    (
        EnumKind::ReportResultViewMode,
        EnumValue::ReportResultViewModeCompact,
        "Компактный",
        "Compact",
        "Compact",
    ),
    (
        EnumKind::ReportResultViewMode,
        EnumValue::ReportResultViewModeDefault,
        "Обычный",
        "Default",
        "Default",
    ),
    (
        EnumKind::FullTextSearchMode,
        EnumValue::FullTextSearchModeDisable,
        "Запретить",
        "Disable",
        "Disable",
    ),
    (
        EnumKind::FullTextSearchMode,
        EnumValue::FullTextSearchModeEnable,
        "Разрешить",
        "Enable",
        "Enable",
    ),
    (
        EnumKind::CryptoCertificateCheckMode,
        EnumValue::CryptoCertificateCheckModeIgnoreTimeValidity,
        "ИгнорироватьВремяДействия",
        "IgnoreTimeValidity",
        "Ignore action time",
    ),
    (
        EnumKind::CryptoCertificateCheckMode,
        EnumValue::CryptoCertificateCheckModeIgnoreSignatureValidity,
        "ИгнорироватьДействительностьПодписи",
        "IgnoreSignatureValidity",
        "Ignore signature validity",
    ),
    (
        EnumKind::CryptoCertificateCheckMode,
        EnumValue::CryptoCertificateCheckModeIgnoreCertificateRevocationStatus,
        "ИгнорироватьПроверкуВСпискеОтозванныхСертификатов",
        "IgnoreCertificateRevocationStatus",
        "Ignore check in revoked certificates list",
    ),
    (
        EnumKind::CryptoCertificateCheckMode,
        EnumValue::CryptoCertificateCheckModeAllowTestCertificates,
        "РазрешитьТестовыеСертификаты",
        "AllowTestCertificates",
        "Enable test",
    ),
    (
        EnumKind::ColumnEditMode,
        EnumValue::ColumnEditModeEnter,
        "Вход",
        "Enter",
        "Enter",
    ),
    (
        EnumKind::ColumnEditMode,
        EnumValue::ColumnEditModeEnterOnInput,
        "ВходПриВводе",
        "EnterOnInput",
        "Enter on input",
    ),
    (
        EnumKind::ColumnEditMode,
        EnumValue::ColumnEditModeDirectly,
        "Непосредственно",
        "Directly",
        "Directly",
    ),
    (
        EnumKind::InfoBaseUserPasswordRecoveryMethod,
        EnumValue::InfoBaseUserPasswordRecoveryMethodSendVerificationCodeBySetParameters,
        "ОтправкаКодаПодтвержденияПоЗаданнымПараметрам",
        "SendVerificationCodeBySetParameters",
        "Send confirmation code with the specified parameters",
    ),
    (
        EnumKind::InfoBaseUserPasswordRecoveryMethod,
        EnumValue::InfoBaseUserPasswordRecoveryMethodSendVerificationCodeByStandardService,
        "ОтправкаКодаПодтвержденияЧерезСтандартныйСервис",
        "SendVerificationCodeByStandardService",
        "Send confirmation code using standard service",
    ),
    (
        EnumKind::SpreadsheetDocumentAreaFillType,
        EnumValue::SpreadsheetDocumentAreaFillTypeParameter,
        "Параметр",
        "Parameter",
        "Parameter",
    ),
    (
        EnumKind::SpreadsheetDocumentAreaFillType,
        EnumValue::SpreadsheetDocumentAreaFillTypeText,
        "Текст",
        "Text",
        "Text",
    ),
    (
        EnumKind::SpreadsheetDocumentAreaFillType,
        EnumValue::SpreadsheetDocumentAreaFillTypeTemplate,
        "Шаблон",
        "Template",
        "Template",
    ),
    (
        EnumKind::QuerySchemaUnionType,
        EnumValue::QuerySchemaUnionTypeUnion,
        "Объединить",
        "Union",
        "Union",
    ),
    (
        EnumKind::QuerySchemaUnionType,
        EnumValue::QuerySchemaUnionTypeUnionAll,
        "ОбъединитьВсе",
        "UnionAll",
        "Union all",
    ),
    (
        EnumKind::CryptoSignatureType,
        EnumValue::CryptoSignatureTypeCAdESAv2,
        "CAdESAv2",
        "CAdESAv2",
        "CAdES-A v2",
    ),
    (
        EnumKind::CryptoSignatureType,
        EnumValue::CryptoSignatureTypeCAdESAv3,
        "CAdESAv3",
        "CAdESAv3",
        "CAdES-A v3",
    ),
    (
        EnumKind::CryptoSignatureType,
        EnumValue::CryptoSignatureTypeCAdESBES,
        "CAdESBES",
        "CAdESBES",
        "CAdES-BES",
    ),
    (
        EnumKind::CryptoSignatureType,
        EnumValue::CryptoSignatureTypeCAdESC,
        "CAdESC",
        "CAdESC",
        "CAdES-C",
    ),
    (
        EnumKind::CryptoSignatureType,
        EnumValue::CryptoSignatureTypeCAdEST,
        "CAdEST",
        "CAdEST",
        "CAdES-T",
    ),
    (
        EnumKind::CryptoSignatureType,
        EnumValue::CryptoSignatureTypeCAdESXLong,
        "CAdESXLong",
        "CAdESXLong",
        "CAdES-X Long",
    ),
    (
        EnumKind::CryptoSignatureType,
        EnumValue::CryptoSignatureTypeCAdESXLongType1,
        "CAdESXLongType1",
        "CAdESXLongType1",
        "CAdES-X Long Type 1",
    ),
    (
        EnumKind::CryptoSignatureType,
        EnumValue::CryptoSignatureTypeCAdESXLongType2,
        "CAdESXLongType2",
        "CAdESXLongType2",
        "CAdES-X Long Type 2",
    ),
    (
        EnumKind::CryptoSignatureType,
        EnumValue::CryptoSignatureTypeCAdESXType1,
        "CAdESXType1",
        "CAdESXType1",
        "CAdES-X Type 1",
    ),
    (
        EnumKind::CryptoSignatureType,
        EnumValue::CryptoSignatureTypeCAdESXType2,
        "CAdESXType2",
        "CAdESXType2",
        "CAdES-X Type 2",
    ),
    (
        EnumKind::CryptoSignatureType,
        EnumValue::CryptoSignatureTypeCMS,
        "CMS",
        "CMS",
        "CMS",
    ),
    (
        EnumKind::SpreadsheetDocumentTextPlacementType,
        EnumValue::SpreadsheetDocumentTextPlacementTypeCut,
        "Обрезать",
        "Cut",
        "Clip",
    ),
    (
        EnumKind::SpreadsheetDocumentTextPlacementType,
        EnumValue::SpreadsheetDocumentTextPlacementTypeWrap,
        "Переносить",
        "Wrap",
        "Wrap",
    ),
    (
        EnumKind::QuerySchemaJoinType,
        EnumValue::QuerySchemaJoinTypeInner,
        "Внутреннее",
        "Inner",
        "Inner",
    ),
    (
        EnumKind::QuerySchemaJoinType,
        EnumValue::QuerySchemaJoinTypeLeftOuter,
        "ЛевоеВнешнее",
        "LeftOuter",
        "Left outer",
    ),
    (
        EnumKind::QuerySchemaJoinType,
        EnumValue::QuerySchemaJoinTypeRightOuter,
        "ПравоеВнешнее",
        "RightOuter",
        "Right outer",
    ),
    (
        EnumKind::ReportFormType,
        EnumValue::ReportFormTypeSettings,
        "Настройка",
        "Settings",
        "Settings",
    ),
    (
        EnumKind::ReportFormType,
        EnumValue::ReportFormTypeMain,
        "Основная",
        "Main",
        "Default",
    ),
    (
        EnumKind::FormattedDocumentItemType,
        EnumValue::FormattedDocumentItemTypeText,
        "Текст",
        "Text",
        "Text",
    ),
    (
        EnumKind::FormattedDocumentItemType,
        EnumValue::FormattedDocumentItemTypeLineFeed,
        "ПереводСтроки",
        "LineFeed",
        "LineFeed",
    ),
    (
        EnumKind::FormattedDocumentItemType,
        EnumValue::FormattedDocumentItemTypePicture,
        "Картинка",
        "Picture",
        "Picture",
    ),
    (
        EnumKind::FixingInTable,
        EnumValue::FixingInTableLeft,
        "Лево",
        "Left",
        "Left",
    ),
    (
        EnumKind::FixingInTable,
        EnumValue::FixingInTableNone,
        "Нет",
        "None",
        "No",
    ),
    (
        EnumKind::FixingInTable,
        EnumValue::FixingInTableRight,
        "Право",
        "Right",
        "Right",
    ),
    // Представления и оба написания измерены на 8.3.27.
    (
        EnumKind::DataCompositionSortDirection,
        EnumValue::DataCompositionSortDirectionAsc,
        "Возр",
        "Asc",
        "Ascending",
    ),
    (
        EnumKind::DataCompositionSortDirection,
        EnumValue::DataCompositionSortDirectionDesc,
        "Убыв",
        "Desc",
        "Descending",
    ),
    // Представления и оба написания измерены на 8.3.27.
    (
        EnumKind::DataCompositionGroupType,
        EnumValue::DataCompositionGroupTypeHierarchy,
        "Иерархия",
        "Hierarchy",
        "Hierarchy",
    ),
    (
        EnumKind::DataCompositionGroupType,
        EnumValue::DataCompositionGroupTypeHierarchyOnly,
        "ТолькоИерархия",
        "HierarchyOnly",
        "Hierarchy only",
    ),
    (
        EnumKind::DataCompositionGroupType,
        EnumValue::DataCompositionGroupTypeItems,
        "Элементы",
        "Items",
        "Elements",
    ),
    // Члены, представления и метатип измерены на 8.3.27.2342.
    (
        EnumKind::DataCompositionGroupPlacement,
        EnumValue::DataCompositionGroupPlacementNone,
        "Нет",
        "None",
        "None",
    ),
    (
        EnumKind::DataCompositionGroupPlacement,
        EnumValue::DataCompositionGroupPlacementBegin,
        "Начало",
        "Begin",
        "Begin",
    ),
    (
        EnumKind::DataCompositionGroupPlacement,
        EnumValue::DataCompositionGroupPlacementEnd,
        "Конец",
        "End",
        "End",
    ),
    (
        EnumKind::DataCompositionGroupPlacement,
        EnumValue::DataCompositionGroupPlacementBeginAndEnd,
        "НачалоИКонец",
        "BeginAndEnd",
        "Begin and end",
    ),
    // Представления и оба написания измерены на 8.3.27.
    (
        EnumKind::DataCompositionGroupFieldsPlacement,
        EnumValue::DataCompositionGroupFieldsPlacementTogether,
        "Вместе",
        "Together",
        "Together",
    ),
    (
        EnumKind::DataCompositionGroupFieldsPlacement,
        EnumValue::DataCompositionGroupFieldsPlacementSeparately,
        "Отдельно",
        "Separately",
        "Separately",
    ),
    (
        EnumKind::DataCompositionGroupFieldsPlacement,
        EnumValue::DataCompositionGroupFieldsPlacementSeparatelyAndInTotalsOnly,
        "ОтдельноИТолькоВИтогах",
        "SeparatelyAndInTotalsOnly",
        "Separately and only in totals",
    ),
    // Представления и оба написания измерены на 8.3.27.
    (
        EnumKind::DataCompositionAttributesPlacement,
        EnumValue::DataCompositionAttributesPlacementTogether,
        "Вместе",
        "Together",
        "Together",
    ),
    (
        EnumKind::DataCompositionAttributesPlacement,
        EnumValue::DataCompositionAttributesPlacementWithOwnerField,
        "ВместеСВладельцем",
        "WithOwnerField",
        "With owner",
    ),
    (
        EnumKind::DataCompositionAttributesPlacement,
        EnumValue::DataCompositionAttributesPlacementSpecialPosition,
        "ВСпециальнойПозиции",
        "SpecialPosition",
        "In special position",
    ),
    (
        EnumKind::DataCompositionAttributesPlacement,
        EnumValue::DataCompositionAttributesPlacementSeparately,
        "Отдельно",
        "Separately",
        "Separately",
    ),
    // Имена и представления измерены на 8.3.27 в `measure-session-enum-batch.bsl`.
    (
        EnumKind::FormDecorationType,
        EnumValue::FormDecorationTypePicture,
        "Картинка",
        "Picture",
        "Picture",
    ),
    (
        EnumKind::FormDecorationType,
        EnumValue::FormDecorationTypeLabel,
        "Надпись",
        "Label",
        "Label",
    ),
    (
        EnumKind::RadioButtonType,
        EnumValue::RadioButtonTypeAuto,
        "Авто",
        "Auto",
        "Auto",
    ),
    (
        EnumKind::RadioButtonType,
        EnumValue::RadioButtonTypeRadioButton,
        "Переключатель",
        "RadioButton",
        "Radio button",
    ),
    (
        EnumKind::RadioButtonType,
        EnumValue::RadioButtonTypeTumbler,
        "Тумблер",
        "Tumbler",
        "Tumbler",
    ),
    (
        EnumKind::ChildFormItemsGroup,
        EnumValue::ChildFormItemsGroupVertical,
        "Вертикальная",
        "Vertical",
        "Vertical",
    ),
    (
        EnumKind::ChildFormItemsGroup,
        EnumValue::ChildFormItemsGroupHorizontal,
        "Горизонтальная",
        "Horizontal",
        "Horizontal",
    ),
    (
        EnumKind::ChildFormItemsGroup,
        EnumValue::ChildFormItemsGroupAlwaysHorizontal,
        "ГоризонтальнаяВсегда",
        "AlwaysHorizontal",
        "Always horizontal",
    ),
    (
        EnumKind::ChildFormItemsGroup,
        EnumValue::ChildFormItemsGroupHorizontalIfPossible,
        "ГоризонтальнаяЕслиВозможно",
        "HorizontalIfPossible",
        "Horizontal if possible",
    ),
    (
        EnumKind::ChoiceHistoryOnInput,
        EnumValue::ChoiceHistoryOnInputAuto,
        "Авто",
        "Auto",
        "Auto",
    ),
    (
        EnumKind::ChoiceHistoryOnInput,
        EnumValue::ChoiceHistoryOnInputDontUse,
        "НеИспользовать",
        "DontUse",
        "No",
    ),
    (
        EnumKind::ChoiceButtonRepresentation,
        EnumValue::ChoiceButtonRepresentationAuto,
        "Авто",
        "Auto",
        "Auto",
    ),
    (
        EnumKind::ChoiceButtonRepresentation,
        EnumValue::ChoiceButtonRepresentationShowInDropList,
        "ОтображатьВВыпадающемСписке",
        "ShowInDropList",
        "Show in drop-down list",
    ),
    (
        EnumKind::ChoiceButtonRepresentation,
        EnumValue::ChoiceButtonRepresentationShowInDropListAndInInputField,
        "ОтображатьВВыпадающемСпискеИВПолеВвода",
        "ShowInDropListAndInInputField",
        "Show in drop-down list and input field",
    ),
    (
        EnumKind::ChoiceButtonRepresentation,
        EnumValue::ChoiceButtonRepresentationShowInInputField,
        "ОтображатьВПолеВвода",
        "ShowInInputField",
        "Show in input field",
    ),
    (
        EnumKind::UsualGroupControlRepresentation,
        EnumValue::UsualGroupControlRepresentationTitleHyperlink,
        "ГиперссылкаЗаголовка",
        "TitleHyperlink",
        "Hyperlink link",
    ),
    (
        EnumKind::UsualGroupControlRepresentation,
        EnumValue::UsualGroupControlRepresentationPicture,
        "Картинка",
        "Picture",
        "Picture",
    ),
    (
        EnumKind::UsualGroupBehavior,
        EnumValue::UsualGroupBehaviorAuto,
        "Авто",
        "Auto",
        "Auto",
    ),
    (
        EnumKind::UsualGroupBehavior,
        EnumValue::UsualGroupBehaviorPopUp,
        "Всплывающая",
        "PopUp",
        "Pop-up",
    ),
    (
        EnumKind::UsualGroupBehavior,
        EnumValue::UsualGroupBehaviorUsual,
        "Обычное",
        "Usual",
        "Regular",
    ),
    (
        EnumKind::UsualGroupBehavior,
        EnumValue::UsualGroupBehaviorCollapsible,
        "Свертываемая",
        "Collapsible",
        "Collapsible",
    ),
    (
        EnumKind::FormItemTitleLocation,
        EnumValue::FormItemTitleLocationAuto,
        "Авто",
        "Auto",
        "Auto",
    ),
    (
        EnumKind::FormItemTitleLocation,
        EnumValue::FormItemTitleLocationTop,
        "Верх",
        "Top",
        "Top",
    ),
    (
        EnumKind::FormItemTitleLocation,
        EnumValue::FormItemTitleLocationLeft,
        "Лево",
        "Left",
        "Left",
    ),
    (
        EnumKind::FormItemTitleLocation,
        EnumValue::FormItemTitleLocationNone,
        "Нет",
        "None",
        "None",
    ),
    (
        EnumKind::FormItemTitleLocation,
        EnumValue::FormItemTitleLocationBottom,
        "Низ",
        "Bottom",
        "Bottom",
    ),
    (
        EnumKind::FormItemTitleLocation,
        EnumValue::FormItemTitleLocationRight,
        "Право",
        "Right",
        "Right",
    ),
    (
        EnumKind::ButtonLocationInCommandBar,
        EnumValue::ButtonLocationInCommandBarAuto,
        "Авто",
        "Auto",
        "Auto",
    ),
    (
        EnumKind::ButtonLocationInCommandBar,
        EnumValue::ButtonLocationInCommandBarInAdditionalSubmenu,
        "ВДополнительномПодменю",
        "InAdditionalSubmenu",
        "In additional submenu",
    ),
    (
        EnumKind::ButtonLocationInCommandBar,
        EnumValue::ButtonLocationInCommandBarInCommandBar,
        "ВКоманднойПанели",
        "InCommandBar",
        "In command bar",
    ),
    (
        EnumKind::ButtonLocationInCommandBar,
        EnumValue::ButtonLocationInCommandBarInCommandBarAndInAdditionalSubmenu,
        "ВКоманднойПанелиИВДополнительномПодменю",
        "InCommandBarAndInAdditionalSubmenu",
        "In the command bar and additional submenu",
    ),
    (
        EnumKind::DataCompositionResourcesPlacement,
        EnumValue::DataCompositionResourcesPlacementVertically,
        "Вертикально",
        "Vertically",
        "Vertical",
    ),
    (
        EnumKind::DataCompositionResourcesPlacement,
        EnumValue::DataCompositionResourcesPlacementHorizontally,
        "Горизонтально",
        "Horizontally",
        "Horizontal",
    ),
    (
        EnumKind::CommonAttributeDataSeparation,
        EnumValue::CommonAttributeDataSeparationDontUse,
        "НеИспользовать",
        "DontUse",
        "DontUse",
    ),
    (
        EnumKind::CommonAttributeDataSeparation,
        EnumValue::CommonAttributeDataSeparationSeparate,
        "Разделять",
        "Separate",
        "Separate",
    ),
    (
        EnumKind::CommonAttributeAutoUse,
        EnumValue::CommonAttributeAutoUseDontUse,
        "НеИспользовать",
        "DontUse",
        "DontUse",
    ),
    (
        EnumKind::CommonAttributeAutoUse,
        EnumValue::CommonAttributeAutoUseUse,
        "Использовать",
        "Use",
        "Use",
    ),
    (
        EnumKind::CommonAttributeUse,
        EnumValue::CommonAttributeUseAuto,
        "Авто",
        "Auto",
        "Auto",
    ),
    (
        EnumKind::CommonAttributeUse,
        EnumValue::CommonAttributeUseDontUse,
        "НеИспользовать",
        "DontUse",
        "DontUse",
    ),
    (
        EnumKind::CommonAttributeUse,
        EnumValue::CommonAttributeUseUse,
        "Использовать",
        "Use",
        "Use",
    ),
    (
        EnumKind::DataCompositionConditionalAppearanceUse,
        EnumValue::DataCompositionConditionalAppearanceUseUse,
        "Использовать",
        "Use",
        "Use",
    ),
    (
        EnumKind::DataCompositionConditionalAppearanceUse,
        EnumValue::DataCompositionConditionalAppearanceUseDontUse,
        "НеИспользовать",
        "DontUse",
        "Do not use",
    ),
    (
        EnumKind::DataCompositionFieldPlacement,
        EnumValue::DataCompositionFieldPlacementAuto,
        "Авто",
        "Auto",
        "Auto",
    ),
    (
        EnumKind::DataCompositionFieldPlacement,
        EnumValue::DataCompositionFieldPlacementVertically,
        "Вертикально",
        "Vertically",
        "Vertical",
    ),
    (
        EnumKind::DataCompositionFieldPlacement,
        EnumValue::DataCompositionFieldPlacementTogether,
        "Вместе",
        "Together",
        "Combined",
    ),
    (
        EnumKind::DataCompositionFieldPlacement,
        EnumValue::DataCompositionFieldPlacementHorizontally,
        "Горизонтально",
        "Horizontally",
        "Horizontal",
    ),
    (
        EnumKind::DataCompositionFieldPlacement,
        EnumValue::DataCompositionFieldPlacementSpecialColumn,
        "ОтдельнаяКолонка",
        "SpecialColumn",
        "Separate column",
    ),
    (
        EnumKind::DataCompositionAccountingBalanceType,
        EnumValue::DataCompositionAccountingBalanceTypeDebit,
        "Дебет",
        "Debit",
        "Debit",
    ),
    (
        EnumKind::DataCompositionAccountingBalanceType,
        EnumValue::DataCompositionAccountingBalanceTypeCredit,
        "Кредит",
        "Credit",
        "Credit",
    ),
    (
        EnumKind::DataCompositionAccountingBalanceType,
        EnumValue::DataCompositionAccountingBalanceTypeNone,
        "Нет",
        "None",
        "None",
    ),
    (
        EnumKind::DataCompositionBalanceType,
        EnumValue::DataCompositionBalanceTypeOpeningBalance,
        "НачальныйОстаток",
        "OpeningBalance",
        "Opening balance",
    ),
    (
        EnumKind::DataCompositionBalanceType,
        EnumValue::DataCompositionBalanceTypeClosingBalance,
        "КонечныйОстаток",
        "ClosingBalance",
        "Closing balance",
    ),
    (
        EnumKind::DataCompositionBalanceType,
        EnumValue::DataCompositionBalanceTypeNone,
        "Нет",
        "None",
        "None",
    ),
    (
        EnumKind::DataCompositionPeriodType,
        EnumValue::DataCompositionPeriodTypeMain,
        "Основной",
        "Main",
        "Default",
    ),
    (
        EnumKind::DataCompositionPeriodType,
        EnumValue::DataCompositionPeriodTypeAdditional,
        "Дополнительный",
        "Additional",
        "Additional",
    ),
    (
        EnumKind::DataCompositionPeriodAdditionType,
        EnumValue::DataCompositionPeriodAdditionTypeNone,
        "БезДополнения",
        "None",
        "No addition",
    ),
    (
        EnumKind::DataCompositionPeriodAdditionType,
        EnumValue::DataCompositionPeriodAdditionTypeYear,
        "Год",
        "Year",
        "Year",
    ),
    (
        EnumKind::DataCompositionPeriodAdditionType,
        EnumValue::DataCompositionPeriodAdditionTypeYearSinceBeginOfPeriod,
        "ГодОтНачалаПериода",
        "YearSinceBeginOfPeriod",
        "Year from the beginning of the period",
    ),
    (
        EnumKind::DataCompositionPeriodAdditionType,
        EnumValue::DataCompositionPeriodAdditionTypeYearSinceBeginOfPeriod445,
        "ГодОтНачалаПериода445",
        "YearSinceBeginOfPeriod445",
        "Year from the beginning of the period (445)",
    ),
    (
        EnumKind::DataCompositionPeriodAdditionType,
        EnumValue::DataCompositionPeriodAdditionTypeTenDays,
        "Декада",
        "TenDays",
        "Ten-day period",
    ),
    (
        EnumKind::DataCompositionPeriodAdditionType,
        EnumValue::DataCompositionPeriodAdditionTypeDay,
        "День",
        "Day",
        "Day",
    ),
    (
        EnumKind::DataCompositionPeriodAdditionType,
        EnumValue::DataCompositionPeriodAdditionTypeDaySinceBeginOfPeriod,
        "ДеньОтНачалаПериода",
        "DaySinceBeginOfPeriod",
        "Day from the beginning of the period",
    ),
    (
        EnumKind::DataCompositionPeriodAdditionType,
        EnumValue::DataCompositionPeriodAdditionTypeQuarter,
        "Квартал",
        "Quarter",
        "Quarter",
    ),
    (
        EnumKind::DataCompositionPeriodAdditionType,
        EnumValue::DataCompositionPeriodAdditionTypeQuarterSinceBeginOfPeriod,
        "КварталОтНачалаПериода",
        "QuarterSinceBeginOfPeriod",
        "Quarter from the beginning of the period",
    ),
    (
        EnumKind::DataCompositionPeriodAdditionType,
        EnumValue::DataCompositionPeriodAdditionTypeQuarterSinceBeginOfPeriod445,
        "КварталОтНачалаПериода445",
        "QuarterSinceBeginOfPeriod445",
        "Quarter from the beginning of the period (445)",
    ),
    (
        EnumKind::DataCompositionPeriodAdditionType,
        EnumValue::DataCompositionPeriodAdditionTypeMonth,
        "Месяц",
        "Month",
        "Month",
    ),
    (
        EnumKind::DataCompositionPeriodAdditionType,
        EnumValue::DataCompositionPeriodAdditionTypeMonthSinceBeginOfPeriod,
        "МесяцОтНачалаПериода",
        "MonthSinceBeginOfPeriod",
        "Month from the beginning of the period",
    ),
    (
        EnumKind::DataCompositionPeriodAdditionType,
        EnumValue::DataCompositionPeriodAdditionTypeMonthSinceBeginOfPeriod445,
        "МесяцОтНачалаПериода445",
        "MonthSinceBeginOfPeriod445",
        "Month from the beginning of the period (445)",
    ),
    (
        EnumKind::DataCompositionPeriodAdditionType,
        EnumValue::DataCompositionPeriodAdditionTypeMinute,
        "Минута",
        "Minute",
        "Minute",
    ),
    (
        EnumKind::DataCompositionPeriodAdditionType,
        EnumValue::DataCompositionPeriodAdditionTypeMinuteSinceBeginOfPeriod,
        "МинутаОтНачалаПериода",
        "MinuteSinceBeginOfPeriod",
        "Minute from the beginning of the period",
    ),
    (
        EnumKind::DataCompositionPeriodAdditionType,
        EnumValue::DataCompositionPeriodAdditionTypeWeek,
        "Неделя",
        "Week",
        "Week",
    ),
    (
        EnumKind::DataCompositionPeriodAdditionType,
        EnumValue::DataCompositionPeriodAdditionTypeWeekSinceBeginOfPeriod,
        "НеделяОтНачалаПериода",
        "WeekSinceBeginOfPeriod",
        "Week from the beginning of the period",
    ),
    (
        EnumKind::DataCompositionPeriodAdditionType,
        EnumValue::DataCompositionPeriodAdditionTypeHalfYear,
        "Полугодие",
        "HalfYear",
        "Half year",
    ),
    (
        EnumKind::DataCompositionPeriodAdditionType,
        EnumValue::DataCompositionPeriodAdditionTypeHalfYearSinceBeginOfPeriod,
        "ПолугодиеОтНачалаПериода",
        "HalfYearSinceBeginOfPeriod",
        "Six months from the beginning of the period",
    ),
    (
        EnumKind::DataCompositionPeriodAdditionType,
        EnumValue::DataCompositionPeriodAdditionTypeHalfYearSinceBeginOfPeriod445,
        "ПолугодиеОтНачалаПериода445",
        "HalfYearSinceBeginOfPeriod445",
        "Six months from the beginning of the period (445)",
    ),
    (
        EnumKind::DataCompositionPeriodAdditionType,
        EnumValue::DataCompositionPeriodAdditionTypeSecond,
        "Секунда",
        "Second",
        "Second",
    ),
    (
        EnumKind::DataCompositionPeriodAdditionType,
        EnumValue::DataCompositionPeriodAdditionTypeHour,
        "Час",
        "Hour",
        "Hour",
    ),
    (
        EnumKind::DataCompositionPeriodAdditionType,
        EnumValue::DataCompositionPeriodAdditionTypeHourSinceBeginOfPeriod,
        "ЧасОтНачалаПериода",
        "HourSinceBeginOfPeriod",
        "Hour from the beginning of the period",
    ),
    // Имена и представления измерены на 8.3.27 в `measure-next-session-enums.bsl`.
    (
        EnumKind::ColorDepth,
        EnumValue::ColorDepthBitPerPixel1,
        "БитНаПиксел1",
        "BitPerPixel1",
        "1 bit per pixel",
    ),
    (
        EnumKind::ColorDepth,
        EnumValue::ColorDepthBitPerPixel24,
        "БитНаПиксел24",
        "BitPerPixel24",
        "24 bits per pixel",
    ),
    (
        EnumKind::ColorDepth,
        EnumValue::ColorDepthBitPerPixel32,
        "БитНаПиксел32",
        "BitPerPixel32",
        "32 bits per pixel",
    ),
    (
        EnumKind::ColorDepth,
        EnumValue::ColorDepthBitPerPixel4,
        "БитНаПиксел4",
        "BitPerPixel4",
        "4 bits per pixel",
    ),
    (
        EnumKind::ColorDepth,
        EnumValue::ColorDepthBitPerPixel8,
        "БитНаПиксел8",
        "BitPerPixel8",
        "8 bits per pixel",
    ),
    (
        EnumKind::ToolTipRepresentation,
        EnumValue::ToolTipRepresentationAuto,
        "Авто",
        "Auto",
        "Auto",
    ),
    (
        EnumKind::ToolTipRepresentation,
        EnumValue::ToolTipRepresentationBalloon,
        "Всплывающая",
        "Balloon",
        "Tooltip",
    ),
    (
        EnumKind::ToolTipRepresentation,
        EnumValue::ToolTipRepresentationButton,
        "Кнопка",
        "Button",
        "Button",
    ),
    (
        EnumKind::ToolTipRepresentation,
        EnumValue::ToolTipRepresentationNone,
        "Нет",
        "None",
        "No",
    ),
    (
        EnumKind::ToolTipRepresentation,
        EnumValue::ToolTipRepresentationShowAuto,
        "ОтображатьАвто",
        "ShowAuto",
        "Display automatically",
    ),
    (
        EnumKind::ToolTipRepresentation,
        EnumValue::ToolTipRepresentationShowTop,
        "ОтображатьСверху",
        "ShowTop",
        "Display above",
    ),
    (
        EnumKind::ToolTipRepresentation,
        EnumValue::ToolTipRepresentationShowLeft,
        "ОтображатьСлева",
        "ShowLeft",
        "Display to the left",
    ),
    (
        EnumKind::ToolTipRepresentation,
        EnumValue::ToolTipRepresentationShowBottom,
        "ОтображатьСнизу",
        "ShowBottom",
        "Display below",
    ),
    (
        EnumKind::ToolTipRepresentation,
        EnumValue::ToolTipRepresentationShowRight,
        "ОтображатьСправа",
        "ShowRight",
        "Display to the right",
    ),
    (
        EnumKind::ClientApplicationType,
        EnumValue::ClientApplicationTypeWebClient,
        "ВебКлиент",
        "WebClient",
        "Web client",
    ),
    (
        EnumKind::ClientApplicationType,
        EnumValue::ClientApplicationTypeExternalConnection,
        "ВнешнееСоединение",
        "ExternalConnection",
        "External connection",
    ),
    (
        EnumKind::ClientApplicationType,
        EnumValue::ClientApplicationTypeMobileAppClient,
        "МобильноеПриложениеКлиент",
        "MobileAppClient",
        "Mobile application client",
    ),
    (
        EnumKind::ClientApplicationType,
        EnumValue::ClientApplicationTypeMobileClient,
        "МобильныйКлиент",
        "MobileClient",
        "Mobile client",
    ),
    (
        EnumKind::ClientApplicationType,
        EnumValue::ClientApplicationTypeThickClient,
        "ТолстыйКлиент",
        "ThickClient",
        "Thick client",
    ),
    (
        EnumKind::ClientApplicationType,
        EnumValue::ClientApplicationTypeThinClient,
        "ТонкийКлиент",
        "ThinClient",
        "Thin client",
    ),
    // Имена и представления измерены на 8.3.27 пробой
    // `measure-client-connection-speed.bsl`.
    (
        EnumKind::ClientConnectionSpeed,
        EnumValue::ClientConnectionSpeedLow,
        "Низкая",
        "Low",
        "Low",
    ),
    (
        EnumKind::ClientConnectionSpeed,
        EnumValue::ClientConnectionSpeedNormal,
        "Обычная",
        "Normal",
        "Normal",
    ),
    // Все четыре имени и представления измерены пробой
    // `measure-spreadsheet-cell-area-type.bsl` на 8.3.27.
    (
        EnumKind::SpreadsheetDocumentCellAreaType,
        EnumValue::SpreadsheetDocumentCellAreaTypeTable,
        "Таблица",
        "Table",
        "Table",
    ),
    (
        EnumKind::SpreadsheetDocumentCellAreaType,
        EnumValue::SpreadsheetDocumentCellAreaTypeRows,
        "Строки",
        "Rows",
        "Rows",
    ),
    (
        EnumKind::SpreadsheetDocumentCellAreaType,
        EnumValue::SpreadsheetDocumentCellAreaTypeColumns,
        "Колонки",
        "Columns",
        "Columns",
    ),
    (
        EnumKind::SpreadsheetDocumentCellAreaType,
        EnumValue::SpreadsheetDocumentCellAreaTypeRectangle,
        "Прямоугольник",
        "Rectangle",
        "Rectangle",
    ),
    (
        EnumKind::AddInConnectionType,
        EnumValue::AddInConnectionTypeIsolated,
        "Изолированно",
        "Isolated",
        "Isolated",
    ),
    (
        EnumKind::AddInConnectionType,
        EnumValue::AddInConnectionTypeNotIsolated,
        "НеИзолированно",
        "NotIsolated",
        "NotIsolated",
    ),
    // Представления `Yes` и `No` измерены пробой `measure-fill-checking.bsl`.
    (
        EnumKind::FillChecking,
        EnumValue::FillCheckingShowError,
        "ВыдаватьОшибку",
        "ShowError",
        "Yes",
    ),
    (
        EnumKind::FillChecking,
        EnumValue::FillCheckingDontCheck,
        "НеПроверять",
        "DontCheck",
        "No",
    ),
    // Имена и представления измерены пробой `measure-folders-and-items.bsl`.
    (
        EnumKind::FoldersAndItems,
        EnumValue::FoldersAndItemsAuto,
        "Авто",
        "Auto",
        "Auto",
    ),
    (
        EnumKind::FoldersAndItems,
        EnumValue::FoldersAndItemsFolders,
        "Группы",
        "Folders",
        "Folders",
    ),
    (
        EnumKind::FoldersAndItems,
        EnumValue::FoldersAndItemsBoth,
        "ГруппыИЭлементы",
        "FoldersAndItems",
        "FoldersAndItems",
    ),
    (
        EnumKind::FoldersAndItems,
        EnumValue::FoldersAndItemsItems,
        "Элементы",
        "Items",
        "Items",
    ),
    // Все значения, aliases и представления измерены контролем `mail-authentication`.
    // Нативный эталон: `xml-canonicalization-type/measure.platform.txt`.
    (
        EnumKind::XmlCanonicalizationType,
        EnumValue::XmlCanonicalizationTypeCanonical,
        "КаноническийXML",
        "XMLCanonicalization",
        "Canonical XML",
    ),
    (
        EnumKind::XmlCanonicalizationType,
        EnumValue::XmlCanonicalizationTypeCanonicalWithComments,
        "КаноническийXMLСКомментариями",
        "XMLCanonicalizationWithComments",
        "Canonical XML with comments",
    ),
    (
        EnumKind::XmlCanonicalizationType,
        EnumValue::XmlCanonicalizationTypeExclusive,
        "ИсключающийКаноническийXML",
        "XMLExclusiveCanonicalization",
        "Exclusive canonical XML",
    ),
    (
        EnumKind::XmlCanonicalizationType,
        EnumValue::XmlCanonicalizationTypeExclusiveWithComments,
        "ИсключающийКаноническийXMLСКомментариями",
        "XMLExclusiveCanonicalizationWithComments",
        "Exclusive canonical XML with comments",
    ),
    (
        EnumKind::XmlCanonicalizationType,
        EnumValue::XmlCanonicalizationTypeCanonical11,
        "КаноническийXML1_1",
        "XMLCanonicalization1_1",
        "Canonical XML 1.1",
    ),
    (
        EnumKind::XmlCanonicalizationType,
        EnumValue::XmlCanonicalizationTypeCanonical11WithComments,
        "КаноническийXML1_1СКомментариями",
        "XMLCanonicalization1_1WithComments",
        "Canonical XML 1.1 with comments",
    ),
    (
        EnumKind::Pop3AuthenticationMode,
        EnumValue::Pop3AuthenticationModeApop,
        "APOP",
        "APOP",
        "APOP",
    ),
    (
        EnumKind::Pop3AuthenticationMode,
        EnumValue::Pop3AuthenticationModeCramMd5,
        "CramMD5",
        "CramMD5",
        "CramMD5",
    ),
    (
        EnumKind::Pop3AuthenticationMode,
        EnumValue::Pop3AuthenticationModeGeneral,
        "Обычная",
        "General",
        "General",
    ),
    (
        EnumKind::SmtpAuthenticationMode,
        EnumValue::SmtpAuthenticationModeCramMd5,
        "CramMD5",
        "CramMD5",
        "CramMD5",
    ),
    (
        EnumKind::SmtpAuthenticationMode,
        EnumValue::SmtpAuthenticationModeLogin,
        "Login",
        "Login",
        "Login",
    ),
    (
        EnumKind::SmtpAuthenticationMode,
        EnumValue::SmtpAuthenticationModePlain,
        "Plain",
        "Plain",
        "Plain",
    ),
    (
        EnumKind::SmtpAuthenticationMode,
        EnumValue::SmtpAuthenticationModeNone,
        "БезАутентификации",
        "None",
        "None",
    ),
    (
        EnumKind::SmtpAuthenticationMode,
        EnumValue::SmtpAuthenticationModeDefault,
        "ПоУмолчанию",
        "Default",
        "Default",
    ),
    // Состав и представления измерены пробой `measure-internet-mail-protocol.bsl`.
    (
        EnumKind::InternetMailProtocol,
        EnumValue::InternetMailProtocolImap,
        "IMAP",
        "IMAP",
        "IMAP",
    ),
    (
        EnumKind::InternetMailProtocol,
        EnumValue::InternetMailProtocolPop3,
        "POP3",
        "POP3",
        "POP3",
    ),
    (
        EnumKind::InternetMailProtocol,
        EnumValue::InternetMailProtocolSmtp,
        "SMTP",
        "SMTP",
        "SMTP",
    ),
    // Представления, включая пробел после `Open value `, измерены на 1С.
    (
        EnumKind::DataCompositionDetailsProcessingAction,
        EnumValue::DataCompositionDetailsProcessingActionNone,
        "Нет",
        "None",
        "None",
    ),
    (
        EnumKind::DataCompositionDetailsProcessingAction,
        EnumValue::DataCompositionDetailsProcessingActionOpenValue,
        "ОткрытьЗначение",
        "OpenValue",
        "Open value ",
    ),
    (
        EnumKind::DataCompositionDetailsProcessingAction,
        EnumValue::DataCompositionDetailsProcessingActionFilter,
        "Отфильтровать",
        "Filter",
        "Filter",
    ),
    (
        EnumKind::DataCompositionDetailsProcessingAction,
        EnumValue::DataCompositionDetailsProcessingActionApplyAppearance,
        "Оформить",
        "ApplyAppearance",
        "Apply appearance",
    ),
    (
        EnumKind::DataCompositionDetailsProcessingAction,
        EnumValue::DataCompositionDetailsProcessingActionDrillDown,
        "Расшифровать",
        "DrillDown",
        "Drill down",
    ),
    (
        EnumKind::DataCompositionDetailsProcessingAction,
        EnumValue::DataCompositionDetailsProcessingActionGroup,
        "Сгруппировать",
        "Group",
        "Group",
    ),
    (
        EnumKind::DataCompositionDetailsProcessingAction,
        EnumValue::DataCompositionDetailsProcessingActionOrder,
        "Упорядочить",
        "Order",
        "Order",
    ),
    (
        EnumKind::DateFractions,
        EnumValue::DateFractionsTime,
        "Время",
        "Time",
        "Time",
    ),
    (
        EnumKind::DateFractions,
        EnumValue::DateFractionsDate,
        "Дата",
        "Date",
        "Date",
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
        EnumValue::HashCrc32,
        "CRC32",
        "CRC32",
        "CRC32 hash function",
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
        EnumValue::HashPbkdf2Sha256,
        "PBKDF2SHA256",
        "PBKDF2SHA256",
        "PBKDF2 SHA-256 hash function",
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
        EnumKind::HashFunction,
        EnumValue::HashSha512,
        "SHA512",
        "SHA512",
        "SHA-512 hash function",
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
    (
        EnumKind::ArchiveFileRestorePathsMode,
        EnumValue::ArchiveRestorePaths,
        "Восстанавливать",
        "Restore",
        "Восстанавливать пути",
    ),
    (
        EnumKind::ArchiveFileRestorePathsMode,
        EnumValue::ArchiveDontRestorePaths,
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
        EnumKind::ZipFileNamesEncoding,
        EnumValue::ZipNamesOsWithUtf8,
        "КодировкаОСДополнительноUTF8",
        "OSEncodingWithUTF8",
        "OS encoding with UTF8",
    ),
    (
        EnumKind::FormattedDocumentFileType,
        EnumValue::FormattedDocumentAnsiTxt,
        "ANSITXT",
        "ANSITXT",
        "ANSITXT",
    ),
    (
        EnumKind::FormattedDocumentFileType,
        EnumValue::FormattedDocumentHtml,
        "HTML",
        "HTML",
        "HTML",
    ),
    (
        EnumKind::FormattedDocumentFileType,
        EnumValue::FormattedDocumentPdf,
        "PDF",
        "PDF",
        "PDF",
    ),
    (
        EnumKind::FormattedDocumentFileType,
        EnumValue::FormattedDocumentTxt,
        "TXT",
        "TXT",
        "TXT",
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
    (
        "АлгоритмПодписиТокенаДоступа",
        EnumKind::AccessTokenSignAlgorithm,
    ),
    (
        "AccessTokenSignAlgorithm",
        EnumKind::AccessTokenSignAlgorithm,
    ),
    (
        "РасположениеОбластиПостроенияДиаграммы",
        EnumKind::ChartPlotAreaPlacement,
    ),
    ("ChartPlotAreaPlacement", EnumKind::ChartPlotAreaPlacement),
    (
        "ТипУзораТабличногоДокумента",
        EnumKind::SpreadsheetDocumentPatternType,
    ),
    (
        "SpreadsheetDocumentPatternType",
        EnumKind::SpreadsheetDocumentPatternType,
    ),
    ("ФигурыГрафическойСхемы", EnumKind::GraphicalSchemaShapes),
    ("GraphicalSchemaShapes", EnumKind::GraphicalSchemaShapes),
    ("ТипСоединительнойЛинии", EnumKind::ConnectorLineType),
    ("ConnectorLineType", EnumKind::ConnectorLineType),
    (
        "ПоложениеТекстаСоединительнойЛинии",
        EnumKind::ConnectorTextLocation,
    ),
    ("ConnectorTextLocation", EnumKind::ConnectorTextLocation),
    ("СтильСтрелки", EnumKind::ArrowStyle),
    ("ArrowStyle", EnumKind::ArrowStyle),
    (
        "ПоложениеКартинкиЭлементаГрафическойСхемы",
        EnumKind::GraphicalSchemaItemPictureLocation,
    ),
    (
        "GraphicalSchemaItemPictureLocation",
        EnumKind::GraphicalSchemaItemPictureLocation,
    ),
    ("ВариантВстроенногоЯзыка", EnumKind::ScriptVariant),
    ("ScriptVariant", EnumKind::ScriptVariant),
    ("РежимЗапускаКлиентскогоПриложения", EnumKind::ClientRunMode),
    ("ClientRunMode", EnumKind::ClientRunMode),
    (
        "РежимОткрытияФормПриложения",
        EnumKind::ApplicationFormsOpenningMode,
    ),
    (
        "ApplicationFormsOpenningMode",
        EnumKind::ApplicationFormsOpenningMode,
    ),
    (
        "ВариантИнтерфейсаКлиентскогоПриложения",
        EnumKind::ClientApplicationInterfaceVariant,
    ),
    (
        "ClientApplicationInterfaceVariant",
        EnumKind::ClientApplicationInterfaceVariant,
    ),
    (
        "ОтображениеПанелиРазделов",
        EnumKind::SectionsPanelRepresentation,
    ),
    (
        "SectionsPanelRepresentation",
        EnumKind::SectionsPanelRepresentation,
    ),
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
        "СтатусТранзакцииЗаписиЖурналаРегистрации",
        EnumKind::EventLogEntryTransactionStatus,
    ),
    (
        "EventLogEntryTransactionStatus",
        EnumKind::EventLogEntryTransactionStatus,
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
    ("РежимЗаписиДокумента", EnumKind::DocumentWriteMode),
    ("DocumentWriteMode", EnumKind::DocumentWriteMode),
    ("ОбходРезультатаЗапроса", EnumKind::QueryResultIteration),
    (
        "ТипСвязиНаборовДанныхКомпоновкиДанных",
        EnumKind::DataCompositionDataSetsLinkType,
    ),
    (
        "DataCompositionDataSetsLinkType",
        EnumKind::DataCompositionDataSetsLinkType,
    ),
    (
        "ТипСоединенияЗначенийПоСериямДиаграммы",
        EnumKind::ChartValuesBySeriesConnectionType,
    ),
    (
        "ChartValuesBySeriesConnectionType",
        EnumKind::ChartValuesBySeriesConnectionType,
    ),
    (
        "РасположениеЛегендыДиаграммыКомпоновкиДанных",
        EnumKind::DataCompositionChartLegendPlacement,
    ),
    (
        "DataCompositionChartLegendPlacement",
        EnumKind::DataCompositionChartLegendPlacement,
    ),
    ("ТипЗаписиЗапроса", EnumKind::QueryRecordType),
    ("QueryRecordType", EnumKind::QueryRecordType),
    ("QueryResultIteration", EnumKind::QueryResultIteration),
    ("РежимБлокировкиДанных", EnumKind::DataLockMode),
    ("DataLockMode", EnumKind::DataLockMode),
    ("ВидСчета", EnumKind::AccountType),
    ("AccountType", EnumKind::AccountType),
    ("РежимОкругления", EnumKind::RoundMode),
    ("RoundMode", EnumKind::RoundMode),
    ("ВидЧисловогоЗначения", EnumKind::NumericValueType),
    ("NumericValueType", EnumKind::NumericValueType),
    (
        "ВариантОтображенияСообщенияОбОшибке",
        EnumKind::ErrorMessageDisplayVariant,
    ),
    (
        "ErrorMessageDisplayVariant",
        EnumKind::ErrorMessageDisplayVariant,
    ),
    (
        "РежимИспользованияПараметраКоманды",
        EnumKind::CommandParameterUseMode,
    ),
    ("CommandParameterUseMode", EnumKind::CommandParameterUseMode),
    (
        "СпособПоискаСтрокиПриВводеПоСтроке",
        EnumKind::SearchStringModeOnInputByString,
    ),
    (
        "SearchStringModeOnInputByString",
        EnumKind::SearchStringModeOnInputByString,
    ),
    (
        "ТипФайлаПакетаОтображаемыхДокументов",
        EnumKind::RepresentableDocumentBatchFileType,
    ),
    (
        "RepresentableDocumentBatchFileType",
        EnumKind::RepresentableDocumentBatchFileType,
    ),
    ("ВидШрифта", EnumKind::FontType),
    ("FontType", EnumKind::FontType),
    ("ВидКартинки", EnumKind::PictureType),
    ("PictureType", EnumKind::PictureType),
    (
        "ИспользованиеМетаданныхПолнотекстовогоПоиска",
        EnumKind::FullTextSearchMetadataUse,
    ),
    (
        "FullTextSearchMetadataUse",
        EnumKind::FullTextSearchMetadataUse,
    ),
    (
        "РежимИзмененияСвязанногоЗначения",
        EnumKind::LinkedValueChangeMode,
    ),
    ("LinkedValueChangeMode", EnumKind::LinkedValueChangeMode),
    (
        "СпособКодированияИнтернетПочтовогоВложения",
        EnumKind::InternetMailAttachmentEncodingMode,
    ),
    (
        "InternetMailAttachmentEncodingMode",
        EnumKind::InternetMailAttachmentEncodingMode,
    ),
    (
        "АвтоматическоеСохранениеДанныхФормыВНастройках",
        EnumKind::AutoSaveFormDataInSettings,
    ),
    (
        "AutoSaveFormDataInSettings",
        EnumKind::AutoSaveFormDataInSettings,
    ),
    ("ТипФайлаДокументаPDF", EnumKind::PDFDocumentFileType),
    ("PDFDocumentFileType", EnumKind::PDFDocumentFileType),
    ("ДопустимыйЗнак", EnumKind::AllowedSign),
    ("AllowedSign", EnumKind::AllowedSign),
    ("АвтоРегистрацияИзменений", EnumKind::AutoChangeRecord),
    ("AutoChangeRecord", EnumKind::AutoChangeRecord),
    ("ОтображениеКнопки", EnumKind::ButtonRepresentation),
    ("ButtonRepresentation", EnumKind::ButtonRepresentation),
    (
        "РезультатПроверкиСоответствияПароляПолитике",
        EnumKind::PasswordPolicyComplianceCheckResult,
    ),
    (
        "PasswordPolicyComplianceCheckResult",
        EnumKind::PasswordPolicyComplianceCheckResult,
    ),
    ("ОтображениеТаблицы", EnumKind::TableRepresentation),
    ("TableRepresentation", EnumKind::TableRepresentation),
    ("ВидПоляФормы", EnumKind::FormFieldType),
    ("FormFieldType", EnumKind::FormFieldType),
    ("ВидГруппыФормы", EnumKind::FormGroupType),
    ("FormGroupType", EnumKind::FormGroupType),
    (
        "ГоризонтальноеПоложениеЭлемента",
        EnumKind::ItemHorizontalLocation,
    ),
    ("ItemHorizontalLocation", EnumKind::ItemHorizontalLocation),
    ("ВертикальноеПоложениеЭлемента", EnumKind::ItemVerticalAlign),
    ("ItemVerticalAlign", EnumKind::ItemVerticalAlign),
    ("ИнтервалМеждуЭлементамиФормы", EnumKind::FormItemSpacing),
    ("FormItemSpacing", EnumKind::FormItemSpacing),
    (
        "ОтображениеФигурыКнопки",
        EnumKind::ButtonShapeRepresentation,
    ),
    (
        "ButtonShapeRepresentation",
        EnumKind::ButtonShapeRepresentation,
    ),
    (
        "ОтображениеОбычнойГруппы",
        EnumKind::UsualGroupRepresentation,
    ),
    (
        "UsualGroupRepresentation",
        EnumKind::UsualGroupRepresentation,
    ),
    ("ВажностьПриОтображении", EnumKind::DisplayImportance),
    ("DisplayImportance", EnumKind::DisplayImportance),
    ("ФорматКартинки", EnumKind::PictureFormat),
    ("PictureFormat", EnumKind::PictureFormat),
    (
        "ТипТекстаПочтовогоСообщения",
        EnumKind::InternetMailTextType,
    ),
    ("InternetMailTextType", EnumKind::InternetMailTextType),
    (
        "СпособКодированияНеASCIIСимволовИнтернетПочтовогоСообщения",
        EnumKind::InternetMailNonAsciiEncodingMode,
    ),
    (
        "InternetMailMessageNonASCIISymbolsEncodingMode",
        EnumKind::InternetMailNonAsciiEncodingMode,
    ),
    (
        "ВажностьИнтернетПочтовогоСообщения",
        EnumKind::InternetMailMessageImportance,
    ),
    (
        "InternetMailMessageImportance",
        EnumKind::InternetMailMessageImportance,
    ),
    (
        "ДополнительныйРежимОтображения",
        EnumKind::AdditionalShowMode,
    ),
    ("AdditionalShowMode", EnumKind::AdditionalShowMode),
    (
        "РежимПроверкиОтзываTLSСертификатаСервера",
        EnumKind::ServerTlsCertificateRevocationCheckMode,
    ),
    (
        "ServerTLSCertificateRevocationCheckMode",
        EnumKind::ServerTlsCertificateRevocationCheckMode,
    ),
    (
        "ТипПримененияОтбораКомпоновкиДанных",
        EnumKind::DataCompositionFilterApplicationType,
    ),
    (
        "DataCompositionFilterApplicationType",
        EnumKind::DataCompositionFilterApplicationType,
    ),
    (
        "СостояниеЭлементаНастройкиКомпоновкиДанных",
        EnumKind::DataCompositionSettingsItemState,
    ),
    (
        "DataCompositionSettingsItemState",
        EnumKind::DataCompositionSettingsItemState,
    ),
    (
        "РежимОтображенияЭлементаНастройкиКомпоновкиДанных",
        EnumKind::DataCompositionSettingsItemViewMode,
    ),
    (
        "DataCompositionSettingsItemViewMode",
        EnumKind::DataCompositionSettingsItemViewMode,
    ),
    (
        "СпособВосстановленияНастроекКомпоновкиДанных",
        EnumKind::DataCompositionSettingsRefreshMethod,
    ),
    (
        "DataCompositionSettingsRefreshMethod",
        EnumKind::DataCompositionSettingsRefreshMethod,
    ),
    (
        "ТипВыводаТекстаКомпоновкиДанных",
        EnumKind::DataCompositionTextOutputType,
    ),
    (
        "ТипРазмещенияТекстаКомпоновкиДанных",
        EnumKind::DataCompositionTextPlacementType,
    ),
    (
        "DataCompositionTextPlacementType",
        EnumKind::DataCompositionTextPlacementType,
    ),
    (
        "DataCompositionTextOutputType",
        EnumKind::DataCompositionTextOutputType,
    ),
    ("РежимМасштабированияПросмотра", EnumKind::ViewScalingMode),
    ("ViewScalingMode", EnumKind::ViewScalingMode),
    (
        "РасположениеИтоговКомпоновкиДанных",
        EnumKind::DataCompositionTotalPlacement,
    ),
    (
        "DataCompositionTotalPlacement",
        EnumKind::DataCompositionTotalPlacement,
    ),
    ("РазмерКартинки", EnumKind::PictureSize),
    ("PictureSize", EnumKind::PictureSize),
    (
        "ТипСмещенияТабличногоДокумента",
        EnumKind::SpreadsheetDocumentShiftType,
    ),
    (
        "SpreadsheetDocumentShiftType",
        EnumKind::SpreadsheetDocumentShiftType,
    ),
    ("ТипДиаграммы", EnumKind::ChartType),
    ("ChartType", EnumKind::ChartType),
    (
        "ОриентацияПодписейДиаграммы",
        EnumKind::ChartLabelsOrientation,
    ),
    ("ОриентацияМетокДиаграммы", EnumKind::ChartLabelsOrientation),
    ("ChartLabelsOrientation", EnumKind::ChartLabelsOrientation),
    ("ВидГраницы", EnumKind::BoundaryType),
    ("BoundaryType", EnumKind::BoundaryType),
    ("МаксимумСерий", EnumKind::MaxSeries),
    ("MaxSeries", EnumKind::MaxSeries),
    ("ВидПодписейКДиаграмме", EnumKind::ChartLabelType),
    ("ChartLabelType", EnumKind::ChartLabelType),
    ("ТипЛинииДиаграммы", EnumKind::ChartLineType),
    ("ChartLineType", EnumKind::ChartLineType),
    ("ВидКнопкиФормы", EnumKind::FormButtonType),
    ("FormButtonType", EnumKind::FormButtonType),
    ("НазначениеТипаXML", EnumKind::XmlTypeAssignment),
    ("XMLTypeAssignment", EnumKind::XmlTypeAssignment),
    ("ТипПлатформы", EnumKind::PlatformType),
    ("PlatformType", EnumKind::PlatformType),
    (
        "ВидСравненияКомпоновкиДанных",
        EnumKind::DataCompositionComparisonType,
    ),
    (
        "DataCompositionComparisonType",
        EnumKind::DataCompositionComparisonType,
    ),
    ("ВидСравнения", EnumKind::ComparisonType),
    ("ComparisonType", EnumKind::ComparisonType),
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
    ("ИспользованиеВывода", EnumKind::UseOutput),
    ("UseOutput", EnumKind::UseOutput),
    ("ОтображениеСтраницФормы", EnumKind::FormPagesRepresentation),
    ("FormPagesRepresentation", EnumKind::FormPagesRepresentation),
    (
        "ПоложениеКартинкиКнопкиФормы",
        EnumKind::FormButtonPictureLocation,
    ),
    (
        "FormButtonPictureLocation",
        EnumKind::FormButtonPictureLocation,
    ),
    (
        "ТипЛинииРисункаТабличногоДокумента",
        EnumKind::SpreadsheetDocumentDrawingLineType,
    ),
    (
        "SpreadsheetDocumentDrawingLineType",
        EnumKind::SpreadsheetDocumentDrawingLineType,
    ),
    ("ВидФлажка", EnumKind::CheckBoxType),
    ("CheckBoxType", EnumKind::CheckBoxType),
    (
        "ТипПодписчикаДоставляемыхУведомлений",
        EnumKind::DeliverableNotificationSubscriberType,
    ),
    (
        "DeliverableNotificationSubscriberType",
        EnumKind::DeliverableNotificationSubscriberType,
    ),
    ("ЗвуковоеОповещение", EnumKind::SoundAlert),
    ("SoundAlert", EnumKind::SoundAlert),
    ("СквозноеВыравнивание", EnumKind::ThroughAlign),
    ("ThroughAlign", EnumKind::ThroughAlign),
    ("ФигураКнопки", EnumKind::ButtonShape),
    ("ButtonShape", EnumKind::ButtonShape),
    (
        "ТипГруппыЭлементовОтбораКомпоновкиДанных",
        EnumKind::DataCompositionFilterItemsGroupType,
    ),
    (
        "DataCompositionFilterItemsGroupType",
        EnumKind::DataCompositionFilterItemsGroupType,
    ),
    ("Клавиша", EnumKind::Key),
    ("Key", EnumKind::Key),
    ("ТипРамкиЭлементаУправления", EnumKind::ControlBorderType),
    ("ControlBorderType", EnumKind::ControlBorderType),
    ("ВидЦвета", EnumKind::ColorType),
    ("ColorType", EnumKind::ColorType),
    (
        "ТипЛинииЯчейкиТабличногоДокумента",
        EnumKind::SpreadsheetDocumentCellLineType,
    ),
    (
        "SpreadsheetDocumentCellLineType",
        EnumKind::SpreadsheetDocumentCellLineType,
    ),
    (
        "ТипХранилищаСертификатовКриптографии",
        EnumKind::CryptoCertificateStoreType,
    ),
    (
        "CryptoCertificateStoreType",
        EnumKind::CryptoCertificateStoreType,
    ),
    (
        "ОбработкаТекстаИнтернетПочтовогоСообщения",
        EnumKind::InternetMailTextProcessing,
    ),
    (
        "InternetMailTextProcessing",
        EnumKind::InternetMailTextProcessing,
    ),
    ("ТипДвустороннейПечати", EnumKind::DuplexPrintingType),
    ("DuplexPrintingType", EnumKind::DuplexPrintingType),
    (
        "ТипОбработкиНастроекВторогоФактораАутентификации",
        EnumKind::SecondAuthenticationFactorSettingsProcessingType,
    ),
    (
        "SecondAuthenticationFactorSettingsProcessingType",
        EnumKind::SecondAuthenticationFactorSettingsProcessingType,
    ),
    ("СпособПеретаскиванияФайлов", EnumKind::FileDragMode),
    ("FileDragMode", EnumKind::FileDragMode),
    (
        "ОтображениеГруппыКнопок",
        EnumKind::ButtonGroupRepresentation,
    ),
    (
        "ButtonGroupRepresentation",
        EnumKind::ButtonGroupRepresentation,
    ),
    (
        "ВариантСтандартногоПериода",
        EnumKind::StandardPeriodVariant,
    ),
    ("StandardPeriodVariant", EnumKind::StandardPeriodVariant),
    (
        "ВариантУправленияВысотойТаблицы",
        EnumKind::TableHeightControlVariant,
    ),
    (
        "TableHeightControlVariant",
        EnumKind::TableHeightControlVariant,
    ),
    (
        "ВерсияПолнотекстовогоПоиска",
        EnumKind::FullTextSearchVersion,
    ),
    ("FullTextSearchVersion", EnumKind::FullTextSearchVersion),
    ("ВертикальнаяПрокруткаФормы", EnumKind::VerticalFormScroll),
    ("VerticalFormScroll", EnumKind::VerticalFormScroll),
    ("ВертикальноеПоложение", EnumKind::VerticalAlign),
    ("VerticalAlign", EnumKind::VerticalAlign),
    ("ВидДвиженияБухгалтерии", EnumKind::AccountingRecordType),
    ("AccountingRecordType", EnumKind::AccountingRecordType),
    (
        "ПоложениеТекстаОтносительноКартинки",
        EnumKind::TextPositionRelativeToPicture,
    ),
    (
        "TextPositionRelativeToPicture",
        EnumKind::TextPositionRelativeToPicture,
    ),
    ("ВидДвиженияНакопления", EnumKind::AccumulationRecordType),
    ("AccumulationRecordType", EnumKind::AccumulationRecordType),
    ("ГоризонтальноеПоложение", EnumKind::HorizontalAlign),
    ("HorizontalAlign", EnumKind::HorizontalAlign),
    ("ГруппировкаКолонок", EnumKind::ColumnsGroup),
    ("ColumnsGroup", EnumKind::ColumnsGroup),
    ("ДопустимыйНомерСообщения", EnumKind::AllowedMessageNo),
    ("AllowedMessageNo", EnumKind::AllowedMessageNo),
    ("ИспользованиеГруппИЭлементов", EnumKind::FoldersAndItemsUse),
    ("FoldersAndItemsUse", EnumKind::FoldersAndItemsUse),
    (
        "ИспользованиеПараметраКомпоновкиДанных",
        EnumKind::DataCompositionParameterUse,
    ),
    (
        "DataCompositionParameterUse",
        EnumKind::DataCompositionParameterUse,
    ),
    ("ИспользованиеПолосыПрокрутки", EnumKind::ScrollBarUse),
    ("ScrollBarUse", EnumKind::ScrollBarUse),
    ("ИспользованиеТекущейСтроки", EnumKind::CurrentRowUse),
    ("CurrentRowUse", EnumKind::CurrentRowUse),
    (
        "НаправлениеПорядкаСхемыЗапроса",
        EnumKind::QuerySchemaOrderDirection,
    ),
    (
        "QuerySchemaOrderDirection",
        EnumKind::QuerySchemaOrderDirection,
    ),
    ("НачальноеОтображениеДерева", EnumKind::InitialTreeView),
    ("InitialTreeView", EnumKind::InitialTreeView),
    ("ОбновлениеПриИзмененииДанных", EnumKind::UpdateOnDataChange),
    ("UpdateOnDataChange", EnumKind::UpdateOnDataChange),
    ("ОбновлениеТекстаРедактирования", EnumKind::EditTextUpdate),
    ("EditTextUpdate", EnumKind::EditTextUpdate),
    (
        "ОтображениеПредупрежденияПриРедактировании",
        EnumKind::WarningOnEditRepresentation,
    ),
    (
        "WarningOnEditRepresentation",
        EnumKind::WarningOnEditRepresentation,
    ),
    ("ПоискВТаблицеПриВводе", EnumKind::SearchInTableOnInput),
    ("SearchInTableOnInput", EnumKind::SearchInTableOnInput),
    (
        "ПоложениеКоманднойПанелиФормы",
        EnumKind::FormCommandBarLabelLocation,
    ),
    (
        "FormCommandBarLabelLocation",
        EnumKind::FormCommandBarLabelLocation,
    ),
    (
        "ПоложениеКоманднойПанелиЭлементаФормы",
        EnumKind::FormItemCommandBarLabelLocation,
    ),
    (
        "FormItemCommandBarLabelLocation",
        EnumKind::FormItemCommandBarLabelLocation,
    ),
    (
        "ПоложениеПодписейШкалыЗначенийИзмерительнойДиаграммы",
        EnumKind::GaugeChartValuesScaleLabelsLocation,
    ),
    (
        "GaugeChartValuesScaleLabelsLocation",
        EnumKind::GaugeChartValuesScaleLabelsLocation,
    ),
    ("ПоложениеПодписейКДиаграмме", EnumKind::ChartLabelLocation),
    ("ChartLabelLocation", EnumKind::ChartLabelLocation),
    ("ОтображениеВЛегендеДиаграммы", EnumKind::ShowInChartLegend),
    ("ShowInChartLegend", EnumKind::ShowInChartLegend),
    (
        "ПоложениеПодписейШкалыДиаграммы",
        EnumKind::ChartScaleLabelLocation,
    ),
    ("ChartScaleLabelLocation", EnumKind::ChartScaleLabelLocation),
    (
        "ИспользованиеРасшифровкиТабличногоДокумента",
        EnumKind::SpreadsheetDocumentDetailUse,
    ),
    (
        "SpreadsheetDocumentDetailUse",
        EnumKind::SpreadsheetDocumentDetailUse,
    ),
    ("ПоложениеСостоянияПросмотра", EnumKind::ViewStatusLocation),
    ("ViewStatusLocation", EnumKind::ViewStatusLocation),
    ("ПоложениеСтрокиПоиска", EnumKind::SearchStringLocation),
    ("SearchStringLocation", EnumKind::SearchStringLocation),
    (
        "ПоложениеУправленияПоиском",
        EnumKind::SearchControlLocation,
    ),
    ("SearchControlLocation", EnumKind::SearchControlLocation),
    ("РежимАвтоВремя", EnumKind::AutoTimeMode),
    ("AutoTimeMode", EnumKind::AutoTimeMode),
    ("РежимВводаСтрокТаблицы", EnumKind::TableRowInputMode),
    ("TableRowInputMode", EnumKind::TableRowInputMode),
    (
        "РежимВключенияСертификатовКриптографии",
        EnumKind::CryptoCertificateIncludeMode,
    ),
    (
        "CryptoCertificateIncludeMode",
        EnumKind::CryptoCertificateIncludeMode,
    ),
    (
        "РежимВыделенияСтрокиТаблицы",
        EnumKind::TableRowSelectionMode,
    ),
    ("TableRowSelectionMode", EnumKind::TableRowSelectionMode),
    ("РежимВыделенияТаблицы", EnumKind::TableSelectionMode),
    ("TableSelectionMode", EnumKind::TableSelectionMode),
    ("РежимЗамещения", EnumKind::ReplacementMode),
    ("ReplacementMode", EnumKind::ReplacementMode),
    ("РежимИзмененияРазмера", EnumKind::SizeChangeMode),
    ("SizeChangeMode", EnumKind::SizeChangeMode),
    ("РежимОткрытияОкнаФормы", EnumKind::FormWindowOpeningMode),
    ("FormWindowOpeningMode", EnumKind::FormWindowOpeningMode),
    (
        "РежимОтображенияНастроекКомпоновкиДанных",
        EnumKind::DataCompositionSettingsViewMode,
    ),
    (
        "DataCompositionSettingsViewMode",
        EnumKind::DataCompositionSettingsViewMode,
    ),
    (
        "РежимОтображенияРезультатаОтчета",
        EnumKind::ReportResultViewMode,
    ),
    ("ReportResultViewMode", EnumKind::ReportResultViewMode),
    ("РежимПолнотекстовогоПоиска", EnumKind::FullTextSearchMode),
    ("FullTextSearchMode", EnumKind::FullTextSearchMode),
    (
        "РежимПроверкиСертификатаКриптографии",
        EnumKind::CryptoCertificateCheckMode,
    ),
    (
        "CryptoCertificateCheckMode",
        EnumKind::CryptoCertificateCheckMode,
    ),
    ("РежимРедактированияКолонки", EnumKind::ColumnEditMode),
    ("ColumnEditMode", EnumKind::ColumnEditMode),
    (
        "СпособВосстановленияПароляПользователяИнформационнойБазы",
        EnumKind::InfoBaseUserPasswordRecoveryMethod,
    ),
    (
        "InfoBaseUserPasswordRecoveryMethod",
        EnumKind::InfoBaseUserPasswordRecoveryMethod,
    ),
    (
        "ТипЗаполненияОбластиТабличногоДокумента",
        EnumKind::SpreadsheetDocumentAreaFillType,
    ),
    (
        "SpreadsheetDocumentAreaFillType",
        EnumKind::SpreadsheetDocumentAreaFillType,
    ),
    ("ТипОбъединенияСхемыЗапроса", EnumKind::QuerySchemaUnionType),
    ("QuerySchemaUnionType", EnumKind::QuerySchemaUnionType),
    ("ТипПодписиКриптографии", EnumKind::CryptoSignatureType),
    ("CryptoSignatureType", EnumKind::CryptoSignatureType),
    (
        "ТипРазмещенияТекстаТабличногоДокумента",
        EnumKind::SpreadsheetDocumentTextPlacementType,
    ),
    (
        "SpreadsheetDocumentTextPlacementType",
        EnumKind::SpreadsheetDocumentTextPlacementType,
    ),
    ("ТипСоединенияСхемыЗапроса", EnumKind::QuerySchemaJoinType),
    ("QuerySchemaJoinType", EnumKind::QuerySchemaJoinType),
    ("ТипФормыОтчета", EnumKind::ReportFormType),
    ("ReportFormType", EnumKind::ReportFormType),
    (
        "ТипЭлементаФорматированногоДокумента",
        EnumKind::FormattedDocumentItemType,
    ),
    (
        "FormattedDocumentItemType",
        EnumKind::FormattedDocumentItemType,
    ),
    ("ФиксацияВТаблице", EnumKind::FixingInTable),
    ("FixingInTable", EnumKind::FixingInTable),
    (
        "НаправлениеСортировкиКомпоновкиДанных",
        EnumKind::DataCompositionSortDirection,
    ),
    (
        "DataCompositionSortDirection",
        EnumKind::DataCompositionSortDirection,
    ),
    (
        "ТипГруппировкиКомпоновкиДанных",
        EnumKind::DataCompositionGroupType,
    ),
    (
        "DataCompositionGroupType",
        EnumKind::DataCompositionGroupType,
    ),
    (
        "РасположениеГруппировкиКомпоновкиДанных",
        EnumKind::DataCompositionGroupPlacement,
    ),
    (
        "DataCompositionGroupPlacement",
        EnumKind::DataCompositionGroupPlacement,
    ),
    (
        "РасположениеПолейГруппировкиКомпоновкиДанных",
        EnumKind::DataCompositionGroupFieldsPlacement,
    ),
    (
        "DataCompositionGroupFieldsPlacement",
        EnumKind::DataCompositionGroupFieldsPlacement,
    ),
    (
        "РасположениеРеквизитовКомпоновкиДанных",
        EnumKind::DataCompositionAttributesPlacement,
    ),
    (
        "DataCompositionAttributesPlacement",
        EnumKind::DataCompositionAttributesPlacement,
    ),
    ("ВидДекорацииФормы", EnumKind::FormDecorationType),
    ("FormDecorationType", EnumKind::FormDecorationType),
    ("ВидПереключателя", EnumKind::RadioButtonType),
    ("RadioButtonType", EnumKind::RadioButtonType),
    (
        "ГруппировкаПодчиненныхЭлементовФормы",
        EnumKind::ChildFormItemsGroup,
    ),
    ("ChildFormItemsGroup", EnumKind::ChildFormItemsGroup),
    ("ИсторияВыбораПриВводе", EnumKind::ChoiceHistoryOnInput),
    ("ChoiceHistoryOnInput", EnumKind::ChoiceHistoryOnInput),
    (
        "ОтображениеКнопкиВыбора",
        EnumKind::ChoiceButtonRepresentation,
    ),
    (
        "ChoiceButtonRepresentation",
        EnumKind::ChoiceButtonRepresentation,
    ),
    (
        "ОтображениеУправленияОбычнойГруппы",
        EnumKind::UsualGroupControlRepresentation,
    ),
    (
        "UsualGroupControlRepresentation",
        EnumKind::UsualGroupControlRepresentation,
    ),
    ("ПоведениеОбычнойГруппы", EnumKind::UsualGroupBehavior),
    ("UsualGroupBehavior", EnumKind::UsualGroupBehavior),
    (
        "ПоложениеЗаголовкаЭлементаФормы",
        EnumKind::FormItemTitleLocation,
    ),
    ("FormItemTitleLocation", EnumKind::FormItemTitleLocation),
    (
        "ПоложениеКнопкиВКоманднойПанели",
        EnumKind::ButtonLocationInCommandBar,
    ),
    (
        "ButtonLocationInCommandBar",
        EnumKind::ButtonLocationInCommandBar,
    ),
    (
        "РасположениеРесурсовКомпоновкиДанных",
        EnumKind::DataCompositionResourcesPlacement,
    ),
    (
        "DataCompositionResourcesPlacement",
        EnumKind::DataCompositionResourcesPlacement,
    ),
    (
        "ИспользованиеУсловногоОформленияКомпоновкиДанных",
        EnumKind::DataCompositionConditionalAppearanceUse,
    ),
    (
        "DataCompositionConditionalAppearanceUse",
        EnumKind::DataCompositionConditionalAppearanceUse,
    ),
    (
        "РасположениеПоляКомпоновкиДанных",
        EnumKind::DataCompositionFieldPlacement,
    ),
    (
        "DataCompositionFieldPlacement",
        EnumKind::DataCompositionFieldPlacement,
    ),
    (
        "ТипБухгалтерскогоОстаткаКомпоновкиДанных",
        EnumKind::DataCompositionAccountingBalanceType,
    ),
    (
        "DataCompositionAccountingBalanceType",
        EnumKind::DataCompositionAccountingBalanceType,
    ),
    (
        "ТипОстаткаКомпоновкиДанных",
        EnumKind::DataCompositionBalanceType,
    ),
    (
        "DataCompositionBalanceType",
        EnumKind::DataCompositionBalanceType,
    ),
    (
        "ТипПериодаКомпоновкиДанных",
        EnumKind::DataCompositionPeriodType,
    ),
    (
        "DataCompositionPeriodType",
        EnumKind::DataCompositionPeriodType,
    ),
    (
        "ТипДополненияПериодаКомпоновкиДанных",
        EnumKind::DataCompositionPeriodAdditionType,
    ),
    (
        "DataCompositionPeriodAdditionType",
        EnumKind::DataCompositionPeriodAdditionType,
    ),
    ("ГлубинаЦвета", EnumKind::ColorDepth),
    ("ColorDepth", EnumKind::ColorDepth),
    ("ОтображениеПодсказки", EnumKind::ToolTipRepresentation),
    ("ToolTipRepresentation", EnumKind::ToolTipRepresentation),
    ("ТипКлиентскогоПриложения", EnumKind::ClientApplicationType),
    ("ClientApplicationType", EnumKind::ClientApplicationType),
    ("СостояниеФоновогоЗадания", EnumKind::BackgroundJobState),
    ("BackgroundJobState", EnumKind::BackgroundJobState),
    ("СтатусСообщения", EnumKind::MessageStatus),
    ("MessageStatus", EnumKind::MessageStatus),
    ("КатегорияОшибки", EnumKind::ErrorCategory),
    ("ErrorCategory", EnumKind::ErrorCategory),
    ("BackgroundJobState", EnumKind::BackgroundJobState),
    ("SortDirection", EnumKind::SortDirection),
    (
        "СкоростьКлиентскогоСоединения",
        EnumKind::ClientConnectionSpeed,
    ),
    ("ClientConnectionSpeed", EnumKind::ClientConnectionSpeed),
    (
        "ТипОбластиЯчеекТабличногоДокумента",
        EnumKind::SpreadsheetDocumentCellAreaType,
    ),
    (
        "SpreadsheetDocumentCellAreaType",
        EnumKind::SpreadsheetDocumentCellAreaType,
    ),
    (
        "ТипПодключенияВнешнейКомпоненты",
        EnumKind::AddInConnectionType,
    ),
    ("AddInConnectionType", EnumKind::AddInConnectionType),
    ("ПроверкаЗаполнения", EnumKind::FillChecking),
    ("FillChecking", EnumKind::FillChecking),
    ("ГруппыИЭлементы", EnumKind::FoldersAndItems),
    ("FoldersAndItems", EnumKind::FoldersAndItems),
    ("ТипКаноническогоXML", EnumKind::XmlCanonicalizationType),
    ("XMLCanonicalizationType", EnumKind::XmlCanonicalizationType),
    ("СпособPOP3Аутентификации", EnumKind::Pop3AuthenticationMode),
    ("POP3AuthenticationMode", EnumKind::Pop3AuthenticationMode),
    ("СпособSMTPАутентификации", EnumKind::SmtpAuthenticationMode),
    ("SMTPAuthenticationMode", EnumKind::SmtpAuthenticationMode),
    ("ПротоколИнтернетПочты", EnumKind::InternetMailProtocol),
    ("InternetMailProtocol", EnumKind::InternetMailProtocol),
    (
        "ДействиеОбработкиРасшифровкиКомпоновкиДанных",
        EnumKind::DataCompositionDetailsProcessingAction,
    ),
    (
        "DataCompositionDetailsProcessingAction",
        EnumKind::DataCompositionDetailsProcessingAction,
    ),
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
    ("ФорматДняШкалыВремени", EnumKind::TimeScaleDayFormat),
    ("TimeScaleDayFormat", EnumKind::TimeScaleDayFormat),
    ("ТипЕдиницыШкалыВремени", EnumKind::TimeScaleUnitType),
    ("TimeScaleUnitType", EnumKind::TimeScaleUnitType),
    (
        "РастягиваниеПоВертикалиДиаграммыГанта",
        EnumKind::GanttChartVerticalStretch,
    ),
    (
        "GanttChartVerticalStretch",
        EnumKind::GanttChartVerticalStretch,
    ),
    (
        "РасположениеЛегендыДиаграммы",
        EnumKind::ChartLegendPlacement,
    ),
    ("ChartLegendPlacement", EnumKind::ChartLegendPlacement),
    (
        "ОтображениеИнтервалаДиаграммыГанта",
        EnumKind::GanttChartIntervalRepresentation,
    ),
    (
        "GanttChartIntervalRepresentation",
        EnumKind::GanttChartIntervalRepresentation,
    ),
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
    (
        "РежимВосстановленияПутейФайлаАрхива",
        EnumKind::ArchiveFileRestorePathsMode,
    ),
    (
        "ArchiveFileRestoreFilePathsMode",
        EnumKind::ArchiveFileRestorePathsMode,
    ),
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
    (
        "ТипФайлаФорматированногоДокумента",
        EnumKind::FormattedDocumentFileType,
    ),
    (
        "FormattedDocumentFileType",
        EnumKind::FormattedDocumentFileType,
    ),
];

/// Перечисление по имени слева от точки. Регистронезависимо и на обоих
/// языках, как и всё остальное в языке.
pub fn lookup_enum(name: &str) -> Option<EnumKind> {
    static INDEX: std::sync::OnceLock<std::collections::HashMap<String, EnumKind>> =
        std::sync::OnceLock::new();
    // Поиск вызывается и для обычных идентификаторов при разрешении имён.
    // Повторная свёртка всех постоянных написаний здесь не нужна.
    INDEX
        .get_or_init(|| {
            let mut index = std::collections::HashMap::with_capacity(ENUM_NAMES.len());
            for (name, kind) in ENUM_NAMES {
                index.entry(name.to_uppercase()).or_insert(*kind);
            }
            index
        })
        .get(&name.to_uppercase())
        .copied()
}

/// Член перечисления по имени справа от точки.
pub fn lookup_member(kind: EnumKind, member: &str) -> Option<EnumValue> {
    // Членов `Ascending`/`Descending` у `НаправлениеСортировки` нет:
    // `measure-sort-direction-alias.platform.txt`.
    static INDEX: std::sync::OnceLock<std::collections::HashMap<(EnumKind, String), EnumValue>> =
        std::sync::OnceLock::new();
    INDEX
        .get_or_init(|| {
            let mut index = std::collections::HashMap::with_capacity(MEMBERS.len() * 2);
            for (kind, value, ru, en, _) in MEMBERS {
                index.entry((*kind, ru.to_uppercase())).or_insert(*value);
                index.entry((*kind, en.to_uppercase())).or_insert(*value);
            }
            index
        })
        .get(&(kind, member.to_uppercase()))
        .copied()
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

    /// Каноническое английское имя члена системного перечисления.
    #[must_use]
    pub fn english_name(self) -> &'static str {
        MEMBERS
            .iter()
            .find(|(_, value, ..)| *value == self)
            .map(|(_, _, _, english, _)| *english)
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
#[path = "enum_lookup_tests.rs"]
mod lookup_tests;

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

    #[test]
    fn date_fractions_match_the_measured_platform_surface() {
        assert_eq!(
            lookup_member(EnumKind::DateFractions, "Time"),
            Some(EnumValue::DateFractionsTime)
        );
        assert_eq!(
            lookup_member(EnumKind::DateFractions, "Дата"),
            Some(EnumValue::DateFractionsDate)
        );
        assert_eq!(
            lookup_member(EnumKind::DateFractions, "DateTime"),
            Some(EnumValue::DateFractionsDateTime)
        );
        assert_eq!(EnumValue::DateFractionsTime.display_text(), "Time");
        assert_eq!(EnumValue::DateFractionsDate.display_text(), "Date");
        assert_eq!(
            EnumValue::DateFractionsDateTime.display_text(),
            "Date and time"
        );
    }

    #[test]
    fn auto_change_record_matches_the_measured_platform_surface() {
        assert_eq!(
            lookup_member(EnumKind::AutoChangeRecord, "Deny"),
            Some(EnumValue::AutoChangeRecordDeny)
        );
        assert_eq!(
            lookup_member(EnumKind::AutoChangeRecord, "Разрешить"),
            Some(EnumValue::AutoChangeRecordAllow)
        );
        assert_eq!(EnumValue::AutoChangeRecordDeny.display_text(), "Disable");
        assert_eq!(EnumValue::AutoChangeRecordAllow.display_text(), "Enable");
        assert_eq!(
            EnumKind::AutoChangeRecord.meta_ru_name(),
            "AutoChangeRecord"
        );
    }

    #[test]
    fn data_lock_mode_matches_the_measured_platform_surface() {
        assert_eq!(
            lookup_member(EnumKind::DataLockMode, "Shared"),
            Some(EnumValue::DataLockModeShared)
        );
        assert_eq!(
            lookup_member(EnumKind::DataLockMode, "Исключительный"),
            Some(EnumValue::DataLockModeExclusive)
        );
        assert_eq!(EnumValue::DataLockModeShared.display_text(), "Shared");
        assert_eq!(
            EnumValue::DataLockModeExclusive.display_text(),
            "Exceptional"
        );
        assert_eq!(EnumKind::DataLockMode.meta_ru_name(), "DataLockMode");
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
