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

use crate::TypeId;

/// Строка таблицы перечислений: написания, метатип, глобальные имена и
/// `TypeId` члена. Номер строки — дискриминант [`EnumKind`].
struct KindRow {
    kind: EnumKind,
    ru: &'static str,
    en: &'static str,
    meta: &'static str,
    global: &'static [&'static str],
    type_id: TypeId,
}

/// Единственное объявление всех системных перечислений. Каждое перечисление
/// задаётся один раз: имена, метатип, необязательный список глобальных имён
/// (по умолчанию русское и английское написание, `global []` — перечисление
/// не доступно по голому имени) и члены `Вариант(русское, английское, что
/// печатает Строка())`. Из объявления строятся [`EnumKind`], [`EnumValue`],
/// таблица перечислений и таблица членов; строка таблицы членов совпадает с
/// дискриминантом члена, поэтому доступ к ней — индекс, а не поиск.
macro_rules! system_enums {
    (@global $ru:literal $en:literal) => { &[$ru, $en] };
    (@global $ru:literal $en:literal [$($global:literal),*]) => { &[$($global),*] };
    (
        $(
            $(#[$kind_meta:meta])*
            $kind:ident($ru:literal, $en:literal) meta $meta:literal
                $(global [$($global:literal),*])? {
                $(
                    $(#[$value_meta:meta])*
                    $value:ident($member_ru:literal, $member_en:literal, $text:literal),
                )*
            }
        )*
    ) => {
        /// К какому перечислению принадлежит член — это же имя стоит слева от
        /// точки в исходном тексте.
        ///
        /// `Ord` нужен не самому перечислению, а несущему варианту
        /// `TypeId::EnumMeta(EnumKind)`: `TypeId` упорядочен целиком.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub enum EnumKind {
            $($(#[$kind_meta])* $kind,)*
        }

        /// Член платформенного перечисления.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub enum EnumValue {
            $($($(#[$value_meta])* $value,)*)*
        }

        const KINDS: &[KindRow] = &[$(KindRow {
            kind: EnumKind::$kind,
            ru: $ru,
            en: $en,
            meta: $meta,
            global: system_enums!(@global $ru $en $([$($global),*])?),
            type_id: TypeId::$kind,
        },)*];

        /// `(перечисление, член, русское имя члена, английское имя члена, что
        /// печатает `Строка()`)`.
        ///
        /// Пятая колонка — не производная от третьей: платформа печатает члены
        /// ЧЕЛОВЕЧЕСКИМ текстом («Начало объекта»), а не идентификатором
        /// («НачалоОбъекта»). Всё измерено, включая хвостовые пробелы.
        const MEMBERS: &[(EnumKind, EnumValue, &str, &str, &str)] = &[
            $($((EnumKind::$kind, EnumValue::$value, $member_ru, $member_en, $text),)*)*
        ];
    };
}

system_enums! {
    /// Основной язык программирования конфигурации.
    ScriptVariant("ВариантВстроенногоЯзыка", "ScriptVariant")
        meta "ПеречислениеВариантВстроенногоЯзыка" {
        ScriptVariantEnglish("Английский", "English", "Английский"),
        ScriptVariantRussian("Русский", "Russian", "Русский"),
    }
    /// Режим совместимости конфигурации с версией платформы.
    CompatibilityMode("РежимСовместимости", "CompatibilityMode")
        meta "ПеречислениеРежимСовместимости" global [] {
        // Состав типа и все представления измерены на платформе 8.3.27.2342
        // через `Метаданные.СвойстваОбъектов.РежимСовместимости`.
        CompatibilityModeVersion8_1("Версия8_1", "Version8_1", "Version8_1"),
        CompatibilityModeVersion8_2_13("Версия8_2_13", "Version8_2_13", "Version8_2_13"),
        CompatibilityModeVersion8_2_16("Версия8_2_16", "Version8_2_16", "Version8_2_16"),
        CompatibilityModeVersion8_3_1("Версия8_3_1", "Version8_3_1", "Version8_3_1"),
        CompatibilityModeVersion8_3_2("Версия8_3_2", "Version8_3_2", "Version8_3_2"),
        CompatibilityModeVersion8_3_3("Версия8_3_3", "Version8_3_3", "Version8_3_3"),
        CompatibilityModeVersion8_3_4("Версия8_3_4", "Version8_3_4", "Version8_3_4"),
        CompatibilityModeVersion8_3_5("Версия8_3_5", "Version8_3_5", "Version8_3_5"),
        CompatibilityModeVersion8_3_6("Версия8_3_6", "Version8_3_6", "Version8_3_6"),
        CompatibilityModeVersion8_3_7("Версия8_3_7", "Version8_3_7", "Version8_3_7"),
        CompatibilityModeVersion8_3_8("Версия8_3_8", "Version8_3_8", "Version8_3_8"),
        CompatibilityModeVersion8_3_9("Версия8_3_9", "Version8_3_9", "Version8_3_9"),
        CompatibilityModeVersion8_3_10("Версия8_3_10", "Version8_3_10", "Version8_3_10"),
        CompatibilityModeVersion8_3_11("Версия8_3_11", "Version8_3_11", "Version8_3_11"),
        CompatibilityModeVersion8_3_12("Версия8_3_12", "Version8_3_12", "Version8_3_12"),
        CompatibilityModeVersion8_3_13("Версия8_3_13", "Version8_3_13", "Version8_3_13"),
        CompatibilityModeVersion8_3_14("Версия8_3_14", "Version8_3_14", "Version8_3_14"),
        CompatibilityModeVersion8_3_15("Версия8_3_15", "Version8_3_15", "Version8_3_15"),
        CompatibilityModeVersion8_3_16("Версия8_3_16", "Version8_3_16", "Version8_3_16"),
        CompatibilityModeVersion8_3_17("Версия8_3_17", "Version8_3_17", "Version8_3_17"),
        CompatibilityModeVersion8_3_18("Версия8_3_18", "Version8_3_18", "Version8_3_18"),
        CompatibilityModeVersion8_3_19("Версия8_3_19", "Version8_3_19", "Version8_3_19"),
        CompatibilityModeVersion8_3_20("Версия8_3_20", "Version8_3_20", "Version8_3_20"),
        CompatibilityModeVersion8_3_21("Версия8_3_21", "Version8_3_21", "Version8_3_21"),
        CompatibilityModeVersion8_3_22("Версия8_3_22", "Version8_3_22", "Version8_3_22"),
        CompatibilityModeVersion8_3_23("Версия8_3_23", "Version8_3_23", "Version8_3_23"),
        CompatibilityModeVersion8_3_24("Версия8_3_24", "Version8_3_24", "Version8_3_24"),
        CompatibilityModeVersion8_3_25("Версия8_3_25", "Version8_3_25", "Version8_3_25"),
        CompatibilityModeVersion8_3_26("Версия8_3_26", "Version8_3_26", "Version8_3_26"),
        CompatibilityModeDontUse("НеИспользовать", "DontUse", "DontUse"),
    }
    /// Режим запуска клиентского приложения.
    // ИЗМЕРЕНО на 8.3.27 вместе с членами перечисления.
    ClientRunMode("РежимЗапускаКлиентскогоПриложения", "ClientRunMode") meta "EnumClientRunMode" {
        ClientRunModeAuto("Авто", "Auto", "Авто"),
        ClientRunModeOrdinaryApplication(
            "ОбычноеПриложение",
            "OrdinaryApplication",
            "Обычное приложение"
        ),
        ClientRunModeManagedApplication(
            "УправляемоеПриложение",
            "ManagedApplication",
            "Управляемое приложение"
        ),
    }
    /// Режим открытия форм клиентского приложения.
    // Имена типов измерены пробой `measure-client-settings.bsl`.
    ApplicationFormsOpenningMode("РежимОткрытияФормПриложения", "ApplicationFormsOpenningMode")
        meta "ApplicationFormsOpenningMode" {
        // Полный состав, оба написания, представления и типы измерены
        // на платформе 8.3.27.2342 пробой `measure-client-settings.bsl`.
        ApplicationFormsOpenningModeTabs("Закладки", "Tabs", "Tabs"),
        ApplicationFormsOpenningModeSingleWindows(
            "ОтдельныеОкна",
            "SingleWindows",
            "Separate windows"
        ),
    }
    /// Вариант интерфейса клиентского приложения.
    ClientApplicationInterfaceVariant(
        "ВариантИнтерфейсаКлиентскогоПриложения",
        "ClientApplicationInterfaceVariant"
    )
        meta "ClientApplicationInterfaceVariant" {
        ClientApplicationInterfaceVariantVersion8_2("Версия8_2", "Version8_2", "Version 8.2"),
        ClientApplicationInterfaceVariantTaxi("Такси", "Taxi", "Taxi"),
    }
    /// Способ отображения разделов приложения.
    SectionsPanelRepresentation("ОтображениеПанелиРазделов", "SectionsPanelRepresentation")
        meta "SectionsPanelRepresentation" {
        // Полный состав, оба написания, представления и тип измерены
        // на платформе 8.3.27.2342 пробой `measure-command-interface-settings.bsl`.
        SectionsPanelRepresentationPicture("Картинка", "Picture", "Icon"),
        SectionsPanelRepresentationPictureAndText(
            "КартинкаИТекст",
            "PictureAndText",
            "Icon and text"
        ),
        SectionsPanelRepresentationPictureOnTopAndText(
            "КартинкаСверхуИТекст",
            "PictureOnTopAndText",
            "Text below icon"
        ),
        SectionsPanelRepresentationPictureOnLeftAndText(
            "КартинкаСлеваИТекст",
            "PictureOnLeftAndText",
            "Text to the right of icon"
        ),
        SectionsPanelRepresentationText("Текст", "Text", "Text"),
    }
    /// Источник списка расширений конфигурации.
    // ИЗМЕРЕНО на 8.3.27 пробой `measure-configuration-extension-enums.bsl`.
    ConfigurationExtensionsSource("ИсточникРасширенийКонфигурации", "ConfigurationExtensionsSource")
        meta "EnumConfigurationExtensionsSource" {
        // Строковые представления и метатипы трёх перечислений расширений
        // измерены на платформе 8.3.27.2342 отдельной воспроизводимой пробой.
        ConfigurationExtensionsSourceDatabase("БазаДанных", "Database", "Database"),
        ConfigurationExtensionsSourceSessionApplied(
            "СеансАктивные",
            "SessionApplied",
            "SessionApplied"
        ),
        ConfigurationExtensionsSourceSessionDisabled(
            "СеансОтключенные",
            "SessionDisabled",
            "SessionDisabled"
        ),
    }
    /// Область действия расширения конфигурации.
    ConfigurationExtensionScope(
        "ОбластьДействияРасширенияКонфигурации",
        "ConfigurationExtensionScope"
    )
        meta "EnumConfigurationExtensionScope" {
        ConfigurationExtensionScopeInfoBase("ИнформационнаяБаза", "InfoBase", "Infobase"),
        ConfigurationExtensionScopeDataSeparation(
            "РазделениеДанных",
            "DataSeparation",
            "Data separation"
        ),
    }
    /// Назначение расширения из пространства свойств метаданных.
    ConfigurationExtensionPurpose(
        "НазначениеРасширенияКонфигурации",
        "ConfigurationExtensionPurpose"
    )
        meta "MetadataObjectsPropertyEnumed" global [] {
        // Назначение доступно только через свойства метаданных; состав и
        // представления измерены контролем `configuration-extension-purpose`.
        ConfigurationExtensionPurposePatch("Исправление", "Patch", "Patch"),
        ConfigurationExtensionPurposeCustomization("Адаптация", "Customization", "Customization"),
        ConfigurationExtensionPurposeAddOn("Дополнение", "AddOn", "AddOn"),
    }
    /// Важность проблемы применения расширения конфигурации.
    ConfigurationExtensionApplicationIssueSeverity(
        "ВажностьПроблемыПримененияРасширенияКонфигурации",
        "ConfigurationExtensionApplicationIssueSeverity"
    )
        meta "EnumConfigurationExtensionApplicationIssueSeverity" {
        ConfigurationExtensionApplicationIssueSeverityCritical("Критичная", "Critical", "Critical"),
        ConfigurationExtensionApplicationIssueSeverityLow("Низкая", "Low", "Low"),
        ConfigurationExtensionApplicationIssueSeverityModerate("Обычная", "Moderate", "Moderate"),
    }
    /// Уровень важности события журнала регистрации.
    // ИЗМЕРЕНО на 8.3.27 пробой `measure-event-log.bsl`.
    EventLogLevel("УровеньЖурналаРегистрации", "EventLogLevel") meta "EnumEventLogLevel" {
        // Имена, метатипы и представления измерены на платформе 8.3.27.2342
        // пробой `measure-event-log.bsl`.
        EventLogLevelNote("Примечание", "Note", "Note"),
        EventLogLevelInformation("Информация", "Information", "Information"),
        EventLogLevelWarning("Предупреждение", "Warning", "Warning"),
        EventLogLevelError("Ошибка", "Error", "Error"),
    }
    /// Связь записи журнала регистрации с текущей транзакцией.
    EventLogEntryTransactionMode(
        "РежимТранзакцииЗаписиЖурналаРегистрации",
        "EventLogEntryTransactionMode"
    )
        meta "EnumEventLogEntryTransactionMode" {
        EventLogEntryTransactionModeIndependent("Независимая", "Independent", "Independent"),
        EventLogEntryTransactionModeTransactional(
            "Транзакционная",
            "Transactional",
            "Transactional"
        ),
    }
    /// Статус транзакции записи журнала регистрации.
    EventLogEntryTransactionStatus(
        "СтатусТранзакцииЗаписиЖурналаРегистрации",
        "EventLogEntryTransactionStatus"
    )
        meta "EventLogEntryTransactionStatus" {
        // Полный состав и представления измерены на 8.3.27.2342.
        EventLogEntryTransactionStatusCommitted("Зафиксирована", "Committed", "Committed"),
        EventLogEntryTransactionStatusUnfinished("НеЗавершена", "Unfinished", "Not completed"),
        EventLogEntryTransactionStatusNotApplicable(
            "НетТранзакции",
            "NotApplicable",
            "No transaction"
        ),
        EventLogEntryTransactionStatusRolledBack("Отменена", "RolledBack", "Rolled back"),
    }
    /// Способ отправки отчёта об ошибке.
    ErrorReportingMode("РежимОтправкиИнформацииОбОшибке", "ErrorReportingMode")
        meta "ErrorReportingMode" {
        // Состав, оба написания и представления измерены на платформе 8.3.27.2342
        // пробой `measure-error-processing.bsl`.
        ErrorReportingModeAuto("Авто", "Auto", "Auto"),
        ErrorReportingModeDontSend("НеОтправлять", "DontSend", "Do not send"),
        ErrorReportingModeAskUser("СпрашиватьПользователя", "AskUser", "Ask user"),
        ErrorReportingModeSend("Отправлять", "Send", "Send"),
    }
    /// Действие над элементом данных при обмене.
    DataItemSend("ОтправкаЭлементаДанных", "DataItemSend") meta "DataItemSend" {
        // Состав, оба написания и представления измерены на платформе 8.3.27.2074
        // пробой `measure-data-item-send.bsl`.
        DataItemSendAuto("Авто", "Auto", "Auto"),
        DataItemSendDelete("Удалить", "Delete", "Delete"),
        DataItemSendIgnore("Игнорировать", "Ignore", "Ignore"),
    }
    /// Действие над полученным элементом данных при обмене.
    DataItemReceive("ПолучениеЭлементаДанных", "DataItemReceive") meta "DataItemReceive" {
        // Состав, оба написания и представления измерены на платформе 8.3.27.2074
        // пробой `measure-data-item-receive.bsl`.
        DataItemReceiveAuto("Авто", "Auto", "Auto"),
        DataItemReceiveAccept("Принять", "Accept", "Accept"),
        DataItemReceiveIgnore("Игнорировать", "Ignore", "Ignore"),
    }
    /// Режим обновления предопределённых данных.
    PredefinedDataUpdate("ОбновлениеПредопределенныхДанных", "PredefinedDataUpdate")
        meta "PredefinedDataUpdate" {
        // Состав, оба написания и представления измерены на платформе 8.3.27.2342
        // пробой `measure-infobase-predefined-data-update.bsl`.
        PredefinedDataUpdateAuto("Авто", "Auto", "Auto"),
        PredefinedDataUpdateAutoUpdate("ОбновлятьАвтоматически", "AutoUpdate", "Auto update"),
        PredefinedDataUpdateDontAutoUpdate(
            "НеОбновлятьАвтоматически",
            "DontAutoUpdate",
            "Auto update off"
        ),
    }
    /// Режим проведения документа.
    DocumentPostingMode("РежимПроведенияДокумента", "DocumentPostingMode")
        meta "DocumentPostingMode" {
        DocumentPostingModeOperational("Оперативный", "RealTime", "Real-time"),
        DocumentPostingModeNonOperational("Неоперативный", "Regular", "Regular"),
    }
    /// Режим записи документа.
    DocumentWriteMode("РежимЗаписиДокумента", "DocumentWriteMode") meta "DocumentWriteMode" {
        DocumentWriteModeWrite("Запись", "Write", "Record"),
        DocumentWriteModePosting("Проведение", "Posting", "Posting"),
        DocumentWriteModeUndoPosting("ОтменаПроведения", "UndoPosting", "Cancel posting"),
    }
    /// Способ обхода записей результата запроса.
    // ИЗМЕРЕНО на 8.3.27 пробой `measure-query-result-iteration.bsl`.
    QueryResultIteration("ОбходРезультатаЗапроса", "QueryResultIteration")
        meta "QueryResultIteration" {
        // Полный состав, оба написания, представления и тип измерены на
        // платформе 8.3.27 пробой `measure-query-result-iteration.bsl`.
        QueryResultIterationLinear("Прямой", "Linear", "Line"),
        QueryResultIterationByGroups("ПоГруппировкам", "ByGroups", "By group"),
        QueryResultIterationByGroupsWithHierarchy(
            "ПоГруппировкамСИерархией",
            "ByGroupsWithHierarchy",
            "By group with hierarchy"
        ),
    }
    /// Тип связи наборов данных компоновки данных.
    DataCompositionDataSetsLinkType(
        "ТипСвязиНаборовДанныхКомпоновкиДанных",
        "DataCompositionDataSetsLinkType"
    )
        meta "EnumDataCompositionDataSetsLinkType" {
        // Члены, алиасы, типы и представления измерены на 8.3.27.2342.
        DataCompositionDataSetsLinkTypeOuter("Внешняя", "Outer", "External"),
        DataCompositionDataSetsLinkTypeInner("Внутренняя", "Inner", "Internal"),
    }
    /// Тип соединения значений по сериям диаграммы.
    ChartValuesBySeriesConnectionType(
        "ТипСоединенияЗначенийПоСериямДиаграммы",
        "ChartValuesBySeriesConnectionType"
    )
        meta "EnumChartValuesBySeriesConnectionType" {
        ChartValuesBySeriesConnectionTypeNone("Нет", "None", "No"),
        ChartValuesBySeriesConnectionTypeEdgesConnection(
            "СоединениеКраев",
            "EdgesConnection",
            "Edge joint"
        ),
    }
    /// Расположение легенды диаграммы компоновки данных.
    DataCompositionChartLegendPlacement(
        "РасположениеЛегендыДиаграммыКомпоновкиДанных",
        "DataCompositionChartLegendPlacement"
    )
        meta "EnumDataCompositionChartLegendPlacement" {
        DataCompositionChartLegendPlacementNone("Нет", "None", "None"),
        DataCompositionChartLegendPlacementLeft("Лево", "Left", "Left"),
        DataCompositionChartLegendPlacementRight("Право", "Right", "Right"),
        DataCompositionChartLegendPlacementTop("Верх", "Top", "Top"),
        DataCompositionChartLegendPlacementBottom("Низ", "Bottom", "Bottom"),
    }
    /// Тип записи выборки результата запроса.
    QueryRecordType("ТипЗаписиЗапроса", "QueryRecordType") meta "EnumQueryRecordType" {
        QueryRecordTypeDetailRecord("ДетальнаяЗапись", "DetailRecord", "Detailed record"),
        QueryRecordTypeGroupTotal("ИтогПоГруппировке", "GroupTotal", "Total by group"),
        QueryRecordTypeTotalByHierarchy("ИтогПоИерархии", "TotalByHierarchy", "Total by hierarchy"),
        QueryRecordTypeOverall("ОбщийИтог", "Overall", "Grand total"),
    }
    /// Режим управляемой блокировки данных.
    // ИЗМЕРЕНО на 8.3.27 пробой `measure-data-lock-mode.bsl`.
    DataLockMode("РежимБлокировкиДанных", "DataLockMode") meta "DataLockMode" {
        // Полный состав, оба написания, представления и тип измерены на
        // платформе 8.3.27 пробой `measure-data-lock-mode.bsl`.
        DataLockModeShared("Разделяемый", "Shared", "Shared"),
        DataLockModeExclusive("Исключительный", "Exclusive", "Exceptional"),
    }
    /// Способ округления половины.
    RoundMode("РежимОкругления", "RoundMode") meta "RoundMode" {
        // Полный состав и представления измерены пробой `measure-round-mode.bsl`.
        RoundModeRound15as10("Окр15как10", "Round15as10", "Round 15 to 10"),
        RoundModeRound15as20("Окр15как20", "Round15as20", "Round 15 to 20"),
    }
    /// Вид числового значения.
    NumericValueType("ВидЧисловогоЗначения", "NumericValueType") meta "NumericValueType" {
        // Полный состав измерен пробой `measure-numeric-value-type.bsl`.
        NumericValueTypeCardinal("Количественное", "Cardinal", "Cardinal"),
        NumericValueTypeOrdinal("Порядковое", "Ordinal", "Ordinal"),
    }
    /// Способ отображения сообщения об ошибке.
    ErrorMessageDisplayVariant("ВариантОтображенияСообщенияОбОшибке", "ErrorMessageDisplayVariant")
        meta "ErrorMessageDisplayVariant" {
        // Полный состав и пробелы представлений измерены на 8.3.27.2342.
        ErrorMessageDisplayVariantAuto("Авто", "Auto", "Auto"),
        ErrorMessageDisplayVariantErrorMessageForUser(
            "СообщениеОбОшибкеДляПользователя",
            "ErrorMessageForUser",
            "Message about error for user "
        ),
        ErrorMessageDisplayVariantBriefErrorDescription(
            "КраткоеПредставлениеОшибки",
            "BriefErrorDescription",
            "Short error presentation "
        ),
        ErrorMessageDisplayVariantDetailErrorDescription(
            "ПодробноеПредставлениеОшибки",
            "DetailErrorDescription",
            "Detailed error presentation "
        ),
    }
    /// Режим использования параметра команды.
    CommandParameterUseMode("РежимИспользованияПараметраКоманды", "CommandParameterUseMode")
        meta "CommandParameterUseMode" {
        // Полный состав, представления и английские имена измерены на 8.3.27.2342.
        CommandParameterUseModeSingle("Одиночный", "Single", "Single"),
        CommandParameterUseModeMultiple("Множественный", "Multiple", "Multiple"),
    }
    /// Способ поиска строки при вводе по строке.
    SearchStringModeOnInputByString(
        "СпособПоискаСтрокиПриВводеПоСтроке",
        "SearchStringModeOnInputByString"
    )
        meta "SearchStringModeOnInputByString" {
        // Полный состав и представления измерены на 8.3.27.2342.
        SearchStringModeOnInputByStringBegin("Начало", "Begin", "At the beginning of line"),
        SearchStringModeOnInputByStringAnyPart("ЛюбаяЧасть", "AnyPart", "Anywhere in line"),
    }
    /// Тип файла пакета отображаемых документов.
    RepresentableDocumentBatchFileType(
        "ТипФайлаПакетаОтображаемыхДокументов",
        "RepresentableDocumentBatchFileType"
    )
        meta "RepresentableDocumentBatchFileType" {
        // Полный состав и представления измерены на 8.3.27.2342.
        RepresentableDocumentBatchFileTypeDocx("DOCX", "DOCX", "DOCX"),
        RepresentableDocumentBatchFileTypeHtml4("HTML4", "HTML4", "HTML4"),
        RepresentableDocumentBatchFileTypeHtml5("HTML5", "HTML5", "HTML5"),
        RepresentableDocumentBatchFileTypeOds("ODS", "ODS", "ODS"),
        RepresentableDocumentBatchFileTypePdf("PDF", "PDF", "PDF"),
        RepresentableDocumentBatchFileTypeTxt("TXT", "TXT", "TXT"),
        RepresentableDocumentBatchFileTypeXls("XLS", "XLS", "XLS"),
        RepresentableDocumentBatchFileTypeXlsx("XLSX", "XLSX", "XLSX"),
    }
    /// Вид шрифта.
    FontType("ВидШрифта", "FontType") meta "FontType" {
        // Полный состав и представления измерены на 8.3.27.2342.
        FontTypeWindowsFont("WindowsШрифт", "WindowsFont", "Windows font"),
        FontTypeAbsolute("Абсолютный", "Absolute", "Absolute"),
        FontTypeAutoFont("АвтоШрифт", "AutoFont", "Auto font"),
        FontTypeStyleItem("ЭлементСтиля", "StyleItem", "Style element"),
    }
    /// Вид картинки.
    PictureType("ВидКартинки", "PictureType") meta "PictureType" {
        // Полный состав и представления измерены на 8.3.27.2342.
        PictureTypeAbsolute("Абсолютная", "Absolute", "Absolute"),
        PictureTypeFromLib("ИзБиблиотеки", "FromLib", "From library"),
        PictureTypeEmpty("Пустая", "Empty", "Empty"),
    }
    /// Использование метаданных при полнотекстовом поиске.
    FullTextSearchMetadataUse(
        "ИспользованиеМетаданныхПолнотекстовогоПоиска",
        "FullTextSearchMetadataUse"
    )
        meta "FullTextSearchMetadataUse" {
        // Полный состав и представления измерены на 8.3.27.2342.
        FullTextSearchMetadataUseUse("Использовать", "Use", "Use"),
        FullTextSearchMetadataUseDontUse("НеИспользовать", "DontUse", "Do not use"),
    }
    /// Режим изменения связанного значения.
    LinkedValueChangeMode("РежимИзмененияСвязанногоЗначения", "LinkedValueChangeMode")
        meta "LinkedValueChangeMode" {
        // Полный состав и представления измерены на 8.3.27.2342.
        LinkedValueChangeModeDontChange("НеИзменять", "DontChange", "Do not change"),
        LinkedValueChangeModeClear("Очищать", "Clear", "Clear"),
    }
    /// Способ кодирования вложения интернет-почты.
    InternetMailAttachmentEncodingMode(
        "СпособКодированияИнтернетПочтовогоВложения",
        "InternetMailAttachmentEncodingMode"
    )
        meta "InternetMailAttachmentEncodingMode" {
        // Полный состав и представления измерены на 8.3.27.2342.
        InternetMailAttachmentEncodingModeMime("MIME", "MIME", "MIME"),
        InternetMailAttachmentEncodingModeUuEncode("UUEncode", "UUEncode", "UUEncode"),
    }
    /// Автоматическое сохранение данных формы в настройках.
    AutoSaveFormDataInSettings(
        "АвтоматическоеСохранениеДанныхФормыВНастройках",
        "AutoSaveFormDataInSettings"
    )
        meta "AutoSaveFormDataInSettings" {
        // Полный состав и представления измерены на 8.3.27.2342.
        AutoSaveFormDataInSettingsUse("Использовать", "Use", "Use"),
        AutoSaveFormDataInSettingsDontUse("НеИспользовать", "DontUse", "Do not use"),
    }
    /// Формат файла документа PDF.
    PDFDocumentFileType("ТипФайлаДокументаPDF", "PDFDocumentFileType") meta "PDFDocumentFileType" {
        // Полный состав и представления измерены на 8.3.27.2342.
        PDFDocumentFileTypePdf("PDF", "PDF", "PDF"),
        PDFDocumentFileTypePdfA1("PDF_A_1", "PDF_A_1", "PDF/A-1"),
        PDFDocumentFileTypePdfA2("PDF_A_2", "PDF_A_2", "PDF/A-2"),
        PDFDocumentFileTypePdfA3("PDF_A_3", "PDF_A_3", "PDF/A-3"),
    }
    /// Допустимый знак числа.
    AllowedSign("ДопустимыйЗнак", "AllowedSign") meta "AllowedSign" {
        // Полный состав и представления измерены на платформе 8.3.27.2074
        // пробой `measure-allowed-sign.bsl`.
        AllowedSignAny("Любой", "Any", "Any"),
        AllowedSignNonnegative("Неотрицательный", "Nonnegative", "Nonnegative"),
    }
    /// Режим автоматической регистрации изменений данных.
    AutoChangeRecord("АвтоРегистрацияИзменений", "AutoChangeRecord") meta "AutoChangeRecord" {
        // Полный состав, оба написания, представления и тип измерены
        // на платформе 8.3.27.2342 пробой `measure-auto-change-record.bsl`.
        AutoChangeRecordDeny("Запретить", "Deny", "Disable"),
        AutoChangeRecordAllow("Разрешить", "Allow", "Enable"),
    }
    /// Способ отображения кнопки формы.
    ButtonRepresentation("ОтображениеКнопки", "ButtonRepresentation") meta "ButtonRepresentation" {
        // Полный состав и представления измерены на платформе 8.3.27.2342
        // пробой `measure-button-representation.bsl`.
        ButtonRepresentationAuto("Авто", "Auto", "Auto"),
        ButtonRepresentationPicture("Картинка", "Picture", "Icon"),
        ButtonRepresentationPictureAndText("КартинкаИТекст", "PictureAndText", "Icon and text"),
        ButtonRepresentationText("Текст", "Text", "Text"),
    }
    /// Причина несоответствия пароля политике.
    // ИЗМЕРЕНО на 8.3.27 пробой
    // `measure-password-policy-compliance-check-result.bsl`.
    PasswordPolicyComplianceCheckResult(
        "РезультатПроверкиСоответствияПароляПолитике",
        "PasswordPolicyComplianceCheckResult"
    )
        meta "PasswordPolicyComplianceCheckResult" {
        // Полный состав, оба написания, представления и тип измерены
        // на платформе 8.3.27.2342 пробой
        // `measure-password-policy-compliance-check-result.bsl`.
        PasswordPolicyComplianceCheckResultMinLength(
            "НеСоответствуетТребованиямМинимальнойДлины",
            "DoesNotSatisfyMinLengthRequirements",
            "Does not meet the minimum length requirements"
        ),
        PasswordPolicyComplianceCheckResultReuseLimit(
            "НеСоответствуетТребованиямОграниченияПовторенияСредиПоследних",
            "DoesNotSatisfyReuseLimitRequirements",
            "Does not meet the requirements for reuse limit among recent ones"
        ),
        PasswordPolicyComplianceCheckResultCompromiseCheck(
            "НеСоответствуетТребованиямПроверкиРаскрытия",
            "DoesNotSatisfyCompromiseCheckRequirements",
            "Does not meet compromise check requirements"
        ),
        PasswordPolicyComplianceCheckResultComplexity(
            "НеСоответствуетТребованиямСложности",
            "DoesNotSatisfyComplexityRequirements",
            "Does not meet the complexity requirements"
        ),
    }
    /// Способ отображения данных в таблице формы.
    // ИЗМЕРЕНО на 8.3.27 пробой `measure-table-representation.bsl`.
    TableRepresentation("ОтображениеТаблицы", "TableRepresentation") meta "TableRepresentation" {
        // Полный состав, оба написания, представления и тип измерены
        // на платформе 8.3.27.2342 пробой `measure-table-representation.bsl`.
        TableRepresentationList("Список", "List", "List"),
        TableRepresentationHierarchicalList(
            "ИерархическийСписок",
            "HierarchicalList",
            "Hierarchical list"
        ),
        TableRepresentationTree("Дерево", "Tree", "Tree"),
    }
    /// Вид поля формы клиентского приложения.
    // ИЗМЕРЕНО на 8.3.27 пробой `measure-form-field-type.bsl`.
    FormFieldType("ВидПоляФормы", "FormFieldType") meta "FormFieldType" {
        // Полный состав, оба написания, представления и тип измерены
        // на платформе 8.3.27.2342 пробой `measure-form-field-type.bsl`.
        FormFieldTypeHtmlDocumentField(
            "ПолеHTMLДокумента",
            "HTMLDocumentField",
            "HTML document field"
        ),
        FormFieldTypePdfDocumentField("ПолеPDFДокумента", "PDFDocumentField", "PDF document field"),
        FormFieldTypeInputField("ПолеВвода", "InputField", "Input field"),
        FormFieldTypeGeographicalSchemaField(
            "ПолеГеографическойСхемы",
            "GeographicalSchemaField",
            "Geographical schema field"
        ),
        FormFieldTypeGraphicalSchemaField(
            "ПолеГрафическойСхемы",
            "GraphicalSchemaField",
            "Graphical schema field"
        ),
        FormFieldTypeDendrogramField("ПолеДендрограммы", "DendrogramField", "Dendrogram field"),
        FormFieldTypeChartField("ПолеДиаграммы", "ChartField", "Chart field"),
        FormFieldTypeGanttChartField("ПолеДиаграммыГанта", "GanttChartField", "Gantt chart field"),
        FormFieldTypeProgressBarField("ПолеИндикатора", "ProgressBarField", "Progress bar"),
        FormFieldTypeCalendarField("ПолеКалендаря", "CalendarField", "Calendar field"),
        FormFieldTypePictureField("ПолеКартинки", "PictureField", "Image field"),
        FormFieldTypeLabelField("ПолеНадписи", "LabelField", "Text box"),
        FormFieldTypeRadioButtonField("ПолеПереключателя", "RadioButtonField", "Radio buttons"),
        FormFieldTypePeriodField("ПолеПериода", "PeriodField", "Period field"),
        FormFieldTypePlannerField("ПолеПланировщика", "PlannerField", "Planner field"),
        FormFieldTypeTrackBarField("ПолеПолосыРегулирования", "TrackBarField", "Slider field"),
        FormFieldTypeSpreadsheetDocumentField(
            "ПолеТабличногоДокумента",
            "SpreadsheetDocumentField",
            "Spreadsheet document field"
        ),
        FormFieldTypeTextDocumentField(
            "ПолеТекстовогоДокумента",
            "TextDocumentField",
            "Text document field"
        ),
        FormFieldTypeCheckBoxField("ПолеФлажка", "CheckBoxField", "Checkbox"),
        FormFieldTypeFormattedDocumentField(
            "ПолеФорматированногоДокумента",
            "FormattedDocumentField",
            "Formatted document field"
        ),
    }
    /// Вид группы формы клиентского приложения.
    // ИЗМЕРЕНО на 8.3.27 пробой `measure-form-group-type.bsl`.
    FormGroupType("ВидГруппыФормы", "FormGroupType") meta "FormGroupType" {
        // Полный состав, оба написания, представления и тип измерены
        // на платформе 8.3.27.2342 пробой `measure-form-group-type.bsl`.
        FormGroupTypeButtonGroup("ГруппаКнопок", "ButtonGroup", "Button group"),
        FormGroupTypeColumnGroup("ГруппаКолонок", "ColumnGroup", "Column group"),
        FormGroupTypeCommandBar("КоманднаяПанель", "CommandBar", "Command bar"),
        FormGroupTypeContextMenu("КонтекстноеМеню", "ContextMenu", "Context menu"),
        FormGroupTypeUsualGroup("ОбычнаяГруппа", "UsualGroup", "Regular group"),
        FormGroupTypePopup("Подменю", "Popup", "Submenu"),
        FormGroupTypePage("Страница", "Page", "Page"),
        FormGroupTypePages("Страницы", "Pages", "Pages"),
    }
    /// Горизонтальное положение элемента формы.
    // ИЗМЕРЕНО на 8.3.27 пробой `measure-item-horizontal-location.bsl`.
    ItemHorizontalLocation("ГоризонтальноеПоложениеЭлемента", "ItemHorizontalLocation")
        meta "ItemHorizontalLocation" {
        // Полный состав, оба написания, представления и тип измерены
        // на платформе 8.3.27.2342 пробой `measure-item-horizontal-location.bsl`.
        ItemHorizontalLocationAuto("Авто", "Auto", "Auto"),
        ItemHorizontalLocationLeft("Лево", "Left", "Left"),
        ItemHorizontalLocationRight("Право", "Right", "Right"),
        ItemHorizontalLocationCenter("Центр", "Center", "Center"),
    }
    /// Вертикальное положение элемента формы.
    // ИЗМЕРЕНО на 8.3.27 пробой `measure-form-alignment-spacing-enums.bsl`.
    ItemVerticalAlign("ВертикальноеПоложениеЭлемента", "ItemVerticalAlign")
        meta "EnumItemVerticalAlign" {
        // Полный состав и представления измерены на платформе 8.3.27.2342
        // пробой `measure-form-alignment-spacing-enums.bsl`.
        ItemVerticalAlignAuto("Авто", "Auto", "Auto"),
        ItemVerticalAlignTop("Верх", "Top", "Top"),
        ItemVerticalAlignBottom("Низ", "Bottom", "Bottom"),
        ItemVerticalAlignCenter("Центр", "Center", "Center"),
    }
    /// Интервал между элементами формы.
    FormItemSpacing("ИнтервалМеждуЭлементамиФормы", "FormItemSpacing") meta "EnumFormItemSpacing" {
        FormItemSpacingAuto("Авто", "Auto", "Auto"),
        FormItemSpacingDouble("Двойной", "Double", "Double"),
        FormItemSpacingNone("Нет", "None", "None"),
        FormItemSpacingSingle("Одинарный", "Single", "Single"),
        FormItemSpacingHalf("Половинный", "Half", "Half"),
        FormItemSpacingOneAndHalf("Полуторный", "OneAndHalf", "One and half"),
    }
    /// Отображение фигуры кнопки формы.
    // ИЗМЕРЕНО на 8.3.27 пробой `measure-button-shape-representation.bsl`.
    ButtonShapeRepresentation("ОтображениеФигурыКнопки", "ButtonShapeRepresentation")
        meta "ButtonShapeRepresentation" {
        // Полный состав, оба написания, представления и тип измерены
        // на платформе 8.3.27.2342 пробой `measure-button-shape-representation.bsl`.
        ButtonShapeRepresentationAuto("Авто", "Auto", "Auto"),
        ButtonShapeRepresentationAlways("Всегда", "Always", "Always"),
        ButtonShapeRepresentationNone("Нет", "None", "None"),
        ButtonShapeRepresentationWhenActive("ПриАктивности", "WhenActive", "When active"),
    }
    /// Отображение обычной группы формы.
    // ИЗМЕРЕНО на 8.3.27 пробой `measure-usual-group-representation.bsl`.
    UsualGroupRepresentation("ОтображениеОбычнойГруппы", "UsualGroupRepresentation")
        meta "UsualGroupRepresentation" {
        // Полный состав, оба написания, представления и тип измерены
        // на платформе 8.3.27.2342 пробой `measure-usual-group-representation.bsl`.
        UsualGroupRepresentationNone("Нет", "None", "None"),
        UsualGroupRepresentationNormalSeparation(
            "ОбычноеВыделение",
            "NormalSeparation",
            "Normal marking"
        ),
        UsualGroupRepresentationStrongSeparation(
            "СильноеВыделение",
            "StrongSeparation",
            "Strong marking"
        ),
        UsualGroupRepresentationWeakSeparation(
            "СлабоеВыделение",
            "WeakSeparation",
            "Light marking"
        ),
    }
    /// Важность элемента формы при отображении.
    // ИЗМЕРЕНО на 8.3.27 пробой `measure-display-importance.bsl`.
    DisplayImportance("ВажностьПриОтображении", "DisplayImportance") meta "DisplayImportance" {
        // Полный состав, оба написания, представления и тип измерены
        // на платформе 8.3.27.2342 пробой `measure-display-importance.bsl`.
        DisplayImportanceAuto("Авто", "Auto", "Auto"),
        DisplayImportanceHigh("Высокая", "High", "High"),
        DisplayImportanceLow("Низкая", "Low", "Low"),
        DisplayImportanceUsual("Обычная", "Usual", "Normal"),
        DisplayImportanceVeryHigh("ОченьВысокая", "VeryHigh", "Very high"),
        DisplayImportanceVeryLow("ОченьНизкая", "VeryLow", "Very low"),
    }
    /// Формат хранения картинки.
    // ИЗМЕРЕНО на 8.3.27 пробой `measure-picture-format.bsl`.
    PictureFormat("ФорматКартинки", "PictureFormat") meta "PictureFormat" {
        // Полный состав, оба написания, представления и тип измерены
        // на платформе 8.3.27.2342 пробой `measure-picture-format.bsl`.
        PictureFormatBmp("BMP", "BMP", "BMP"),
        PictureFormatEmf("EMF", "EMF", "EMF"),
        PictureFormatGif("GIF", "GIF", "GIF"),
        PictureFormatIcon("Icon", "Icon", "Icon"),
        PictureFormatJpeg("JPEG", "JPEG", "JPEG"),
        PictureFormatPng("PNG", "PNG", "PNG"),
        PictureFormatSvg("SVG", "SVG", "SVG"),
        PictureFormatTiff("TIFF", "TIFF", "TIFF"),
        PictureFormatWmf("WMF", "WMF", "WMF"),
        PictureFormatUnknown("НеизвестныйФормат", "UnknownFormat", "Unknown format"),
    }
    /// Тип текста почтового сообщения.
    // ИЗМЕРЕНО на 8.3.27 пробой `measure-internet-mail-text-type.bsl`.
    InternetMailTextType("ТипТекстаПочтовогоСообщения", "InternetMailTextType")
        meta "InternetMailTextType" {
        // Полный состав, оба написания, представления и тип измерены
        // на платформе 8.3.27.2342 пробой `measure-internet-mail-text-type.bsl`.
        InternetMailTextTypeHtml("HTML", "HTML", "HTML"),
        InternetMailTextTypeCustomText("ПроизвольныйТекст", "CustomText", "CustomText"),
        InternetMailTextTypePlainText("ПростойТекст", "PlainText", "PlainText"),
        InternetMailTextTypeRichText("РазмеченныйТекст", "RichText", "RichText"),
    }
    /// Способ кодирования не-ASCII символов почтового сообщения.
    // ИЗМЕРЕНО на 8.3.27 пробой `measure-mail-non-ascii-encoding-mode.bsl`.
    InternetMailNonAsciiEncodingMode(
        "СпособКодированияНеASCIIСимволовИнтернетПочтовогоСообщения",
        "InternetMailMessageNonASCIISymbolsEncodingMode"
    )
        meta "InternetMailMessageNonASCIISymbolsEncodingMode" {
        // Полный состав, оба написания, представления и тип измерены
        // на платформе 8.3.27.2342 пробой `measure-mail-non-ascii-encoding-mode.bsl`.
        InternetMailNonAsciiMime("MIME", "MIME", "MIME"),
        InternetMailNonAsciiQuotedPrintable(
            "QuotedPrintable",
            "QuotedPrintable",
            "QuotedPrintable"
        ),
        InternetMailNonAsciiNone("БезКодирования", "None", "None"),
    }
    /// Важность интернет-почтового сообщения.
    // ИЗМЕРЕНО на 8.3.27 пробой `measure-mail-message-importance.bsl`.
    InternetMailMessageImportance(
        "ВажностьИнтернетПочтовогоСообщения",
        "InternetMailMessageImportance"
    )
        meta "InternetMailMessageImportance" {
        // Полный состав, оба написания, представления и тип измерены
        // на платформе 8.3.27.2342 пробой `measure-mail-message-importance.bsl`.
        InternetMailMessageImportanceHigh("Высокая", "High", "High"),
        InternetMailMessageImportanceHighest("Наивысшая", "Highest", "Highest"),
        InternetMailMessageImportanceLowest("Наименьшая", "Lowest", "Lowest"),
        InternetMailMessageImportanceLow("Низкая", "Low", "Low"),
        InternetMailMessageImportanceNormal("Обычная", "Normal", "Normal"),
    }
    /// Дополнительный способ отображения состояния.
    // ИЗМЕРЕНО на 8.3.27 пробой `measure-additional-show-mode.bsl`.
    AdditionalShowMode("ДополнительныйРежимОтображения", "AdditionalShowMode")
        meta "AdditionalShowMode" {
        // Полный состав, оба написания, представления и тип измерены
        // на платформе 8.3.27.2342 пробой `measure-additional-show-mode.bsl`.
        AdditionalShowModeDontUse("НеИспользовать", "DontUse", "Do not use"),
        AdditionalShowModeIrrelevance("Неактуальность", "Irrelevance", "Irrelevance"),
    }
    /// Режим проверки отзыва TLS-сертификата сервера.
    // ИЗМЕРЕНО на 8.3.27 пробой `measure-tls-revocation-mode.bsl`.
    ServerTlsCertificateRevocationCheckMode(
        "РежимПроверкиОтзываTLSСертификатаСервера",
        "ServerTLSCertificateRevocationCheckMode"
    )
        meta "ServerTLSCertificateRevocationCheckMode" {
        // Полный состав, оба написания, представления и тип измерены
        // на платформе 8.3.27.2342 пробой `measure-tls-revocation-mode.bsl`.
        ServerTlsCertificateRevocationCheckModeAuto("Авто", "Auto", "Auto"),
        ServerTlsCertificateRevocationCheckModeDontCheck(
            "НеПроверять",
            "DontCheck",
            "Certificate revocation check is disabled"
        ),
        ServerTlsCertificateRevocationCheckModeSoftFail(
            "Нестрогий",
            "SoftFail",
            "Soft certificate revocation check"
        ),
        ServerTlsCertificateRevocationCheckModeStrict(
            "Строгий",
            "Strict",
            "Strict certificate revocation check"
        ),
    }
    /// Область применения отбора к записям результа компоновки.
    // ИЗМЕРЕНО на 8.3.27 пробой
    // `measure-data-composition-filter-application-type.bsl`.
    DataCompositionFilterApplicationType(
        "ТипПримененияОтбораКомпоновкиДанных",
        "DataCompositionFilterApplicationType"
    )
        meta "DataCompositionFilterApplicationType" {
        // Полный состав, оба написания, представления и тип измерены
        // на платформе 8.3.27.2342 пробой
        // `measure-data-composition-filter-application-type.bsl`.
        DataCompositionFilterApplicationTypeItems("Элементы", "Items", "No hierarchy"),
        DataCompositionFilterApplicationTypeHierarchy("Иерархия", "Hierarchy", "Hierarchy"),
        DataCompositionFilterApplicationTypeHierarchyOnly(
            "ТолькоИерархия",
            "HierarchyOnly",
            "Hierarchy only"
        ),
    }
    /// Состояние элемента настройки компоновки данных.
    // ИЗМЕРЕНО на 8.3.27 пробой
    // `measure-data-composition-settings-item-view-mode.bsl`.
    DataCompositionSettingsItemState(
        "СостояниеЭлементаНастройкиКомпоновкиДанных",
        "DataCompositionSettingsItemState"
    )
        meta "EnumDataCompositionSettingsItemState" {
        // Полный состав, алиасы и представления измерены на 8.3.27.2342.
        DataCompositionSettingsItemStateEnabled("Включен", "Enabled", "Enabled"),
        DataCompositionSettingsItemStateDisabled("Отключен", "Disabled", "Disabled"),
        DataCompositionSettingsItemStateDeletedByUser(
            "УдаленПользователем",
            "DeletedByUser",
            "Deleted by user"
        ),
    }
    /// Режим отображения элемента настройки компоновки.
    DataCompositionSettingsItemViewMode(
        "РежимОтображенияЭлементаНастройкиКомпоновкиДанных",
        "DataCompositionSettingsItemViewMode"
    )
        meta "DataCompositionSettingsItemViewMode" {
        // Полный состав, оба написания, представления и тип измерены
        // на платформе 8.3.27.2342 пробой
        // `measure-data-composition-settings-item-view-mode.bsl`.
        DataCompositionSettingsItemViewModeAuto("Авто", "Auto", "Auto"),
        DataCompositionSettingsItemViewModeNormal("Обычный", "Normal", "Normal"),
        DataCompositionSettingsItemViewModeQuickAccess(
            "БыстрыйДоступ",
            "QuickAccess",
            "Quick access"
        ),
        DataCompositionSettingsItemViewModeInaccessible("Недоступный", "Inaccessible", "Disabled"),
    }
    /// Способ восстановления настроек компоновки данных.
    // ИЗМЕРЕНО на 8.3.27 пробой
    // `measure-data-composition-settings-refresh-method.bsl`.
    DataCompositionSettingsRefreshMethod(
        "СпособВосстановленияНастроекКомпоновкиДанных",
        "DataCompositionSettingsRefreshMethod"
    )
        meta "EnumDataCompositionSettingsRefreshMethod" {
        // Полный состав и представления измерены на платформе 8.3.27.2342
        // пробой `measure-data-composition-settings-refresh-method.bsl`.
        DataCompositionSettingsRefreshMethodFull("Полное", "Full", "Full"),
        DataCompositionSettingsRefreshMethodCheckAvailability(
            "ПроверятьДоступность",
            "CheckAvailability",
            "Check accessibility"
        ),
    }
    /// Направление сортировки компоновки данных.
    DataCompositionSortDirection(
        "НаправлениеСортировкиКомпоновкиДанных",
        "DataCompositionSortDirection"
    )
        meta "EnumDataCompositionSortDirection" {
        // Представления и оба написания измерены на 8.3.27.
        DataCompositionSortDirectionAsc("Возр", "Asc", "Ascending"),
        DataCompositionSortDirectionDesc("Убыв", "Desc", "Descending"),
    }
    /// Тип группировки компоновки данных.
    DataCompositionGroupType("ТипГруппировкиКомпоновкиДанных", "DataCompositionGroupType")
        meta "EnumDataCompositionGroupType" {
        // Представления и оба написания измерены на 8.3.27.
        DataCompositionGroupTypeHierarchy("Иерархия", "Hierarchy", "Hierarchy"),
        DataCompositionGroupTypeHierarchyOnly("ТолькоИерархия", "HierarchyOnly", "Hierarchy only"),
        DataCompositionGroupTypeItems("Элементы", "Items", "Elements"),
    }
    /// Расположение группировки компоновки данных.
    DataCompositionGroupPlacement(
        "РасположениеГруппировкиКомпоновкиДанных",
        "DataCompositionGroupPlacement"
    )
        meta "EnumDataCompositionGroupPlacement" {
        DataCompositionGroupPlacementEnd("Конец", "End", "End"),
        DataCompositionGroupPlacementBegin("Начало", "Begin", "Begin"),
        DataCompositionGroupPlacementBeginAndEnd("НачалоИКонец", "BeginAndEnd", "Begin and end"),
        // Члены, представления и метатип измерены на 8.3.27.2342.
        DataCompositionGroupPlacementNone("Нет", "None", "None"),
    }
    /// Расположение полей группировки компоновки данных.
    DataCompositionGroupFieldsPlacement(
        "РасположениеПолейГруппировкиКомпоновкиДанных",
        "DataCompositionGroupFieldsPlacement"
    )
        meta "EnumDataCompositionGroupFieldsPlacement" {
        // Представления и оба написания измерены на 8.3.27.
        DataCompositionGroupFieldsPlacementTogether("Вместе", "Together", "Together"),
        DataCompositionGroupFieldsPlacementSeparately("Отдельно", "Separately", "Separately"),
        DataCompositionGroupFieldsPlacementSeparatelyAndInTotalsOnly(
            "ОтдельноИТолькоВИтогах",
            "SeparatelyAndInTotalsOnly",
            "Separately and only in totals"
        ),
    }
    /// Расположение реквизитов компоновки данных.
    DataCompositionAttributesPlacement(
        "РасположениеРеквизитовКомпоновкиДанных",
        "DataCompositionAttributesPlacement"
    )
        meta "EnumDataCompositionAttributesPlacement" {
        // Представления и оба написания измерены на 8.3.27.
        DataCompositionAttributesPlacementTogether("Вместе", "Together", "Together"),
        DataCompositionAttributesPlacementWithOwnerField(
            "ВместеСВладельцем",
            "WithOwnerField",
            "With owner"
        ),
        DataCompositionAttributesPlacementSpecialPosition(
            "ВСпециальнойПозиции",
            "SpecialPosition",
            "In special position"
        ),
        DataCompositionAttributesPlacementSeparately("Отдельно", "Separately", "Separately"),
    }
    /// ВидДекорацииФормы.
    FormDecorationType("ВидДекорацииФормы", "FormDecorationType") meta "EnumFormDecorationType" {
        // Имена и представления измерены на 8.3.27 в `measure-session-enum-batch.bsl`.
        FormDecorationTypePicture("Картинка", "Picture", "Picture"),
        FormDecorationTypeLabel("Надпись", "Label", "Label"),
    }
    /// ВидПереключателя.
    RadioButtonType("ВидПереключателя", "RadioButtonType") meta "EnumRadioButtonType" {
        RadioButtonTypeAuto("Авто", "Auto", "Auto"),
        RadioButtonTypeRadioButton("Переключатель", "RadioButton", "Radio button"),
        RadioButtonTypeTumbler("Тумблер", "Tumbler", "Tumbler"),
    }
    /// ГруппировкаПодчиненныхЭлементовФормы.
    ChildFormItemsGroup("ГруппировкаПодчиненныхЭлементовФормы", "ChildFormItemsGroup")
        meta "EnumChildFormItemsGroup" {
        ChildFormItemsGroupVertical("Вертикальная", "Vertical", "Vertical"),
        ChildFormItemsGroupHorizontal("Горизонтальная", "Horizontal", "Horizontal"),
        ChildFormItemsGroupAlwaysHorizontal(
            "ГоризонтальнаяВсегда",
            "AlwaysHorizontal",
            "Always horizontal"
        ),
        ChildFormItemsGroupHorizontalIfPossible(
            "ГоризонтальнаяЕслиВозможно",
            "HorizontalIfPossible",
            "Horizontal if possible"
        ),
    }
    /// ИсторияВыбораПриВводе.
    ChoiceHistoryOnInput("ИсторияВыбораПриВводе", "ChoiceHistoryOnInput")
        meta "EnumChoiceHistoryOnInput" {
        ChoiceHistoryOnInputAuto("Авто", "Auto", "Auto"),
        ChoiceHistoryOnInputDontUse("НеИспользовать", "DontUse", "No"),
    }
    /// ОтображениеКнопкиВыбора.
    ChoiceButtonRepresentation("ОтображениеКнопкиВыбора", "ChoiceButtonRepresentation")
        meta "EnumChoiceButtonRepresentation" {
        ChoiceButtonRepresentationAuto("Авто", "Auto", "Auto"),
        ChoiceButtonRepresentationShowInDropList(
            "ОтображатьВВыпадающемСписке",
            "ShowInDropList",
            "Show in drop-down list"
        ),
        ChoiceButtonRepresentationShowInDropListAndInInputField(
            "ОтображатьВВыпадающемСпискеИВПолеВвода",
            "ShowInDropListAndInInputField",
            "Show in drop-down list and input field"
        ),
        ChoiceButtonRepresentationShowInInputField(
            "ОтображатьВПолеВвода",
            "ShowInInputField",
            "Show in input field"
        ),
    }
    /// ОтображениеУправленияОбычнойГруппы.
    UsualGroupControlRepresentation(
        "ОтображениеУправленияОбычнойГруппы",
        "UsualGroupControlRepresentation"
    )
        meta "EnumUsualGroupControlRepresentation" {
        UsualGroupControlRepresentationTitleHyperlink(
            "ГиперссылкаЗаголовка",
            "TitleHyperlink",
            "Hyperlink link"
        ),
        UsualGroupControlRepresentationPicture("Картинка", "Picture", "Picture"),
    }
    /// ПоведениеОбычнойГруппы.
    UsualGroupBehavior("ПоведениеОбычнойГруппы", "UsualGroupBehavior")
        meta "EnumUsualGroupBehavior" {
        UsualGroupBehaviorAuto("Авто", "Auto", "Auto"),
        UsualGroupBehaviorPopUp("Всплывающая", "PopUp", "Pop-up"),
        UsualGroupBehaviorUsual("Обычное", "Usual", "Regular"),
        UsualGroupBehaviorCollapsible("Свертываемая", "Collapsible", "Collapsible"),
    }
    /// ПоложениеЗаголовкаЭлементаФормы.
    FormItemTitleLocation("ПоложениеЗаголовкаЭлементаФормы", "FormItemTitleLocation")
        meta "EnumFormItemTitleLocation" {
        FormItemTitleLocationAuto("Авто", "Auto", "Auto"),
        FormItemTitleLocationTop("Верх", "Top", "Top"),
        FormItemTitleLocationLeft("Лево", "Left", "Left"),
        FormItemTitleLocationNone("Нет", "None", "None"),
        FormItemTitleLocationBottom("Низ", "Bottom", "Bottom"),
        FormItemTitleLocationRight("Право", "Right", "Right"),
    }
    /// ПоложениеКнопкиВКоманднойПанели.
    ButtonLocationInCommandBar("ПоложениеКнопкиВКоманднойПанели", "ButtonLocationInCommandBar")
        meta "EnumButtonLocationInCommandBar" {
        ButtonLocationInCommandBarAuto("Авто", "Auto", "Auto"),
        ButtonLocationInCommandBarInAdditionalSubmenu(
            "ВДополнительномПодменю",
            "InAdditionalSubmenu",
            "In additional submenu"
        ),
        ButtonLocationInCommandBarInCommandBar(
            "ВКоманднойПанели",
            "InCommandBar",
            "In command bar"
        ),
        ButtonLocationInCommandBarInCommandBarAndInAdditionalSubmenu(
            "ВКоманднойПанелиИВДополнительномПодменю",
            "InCommandBarAndInAdditionalSubmenu",
            "In the command bar and additional submenu"
        ),
    }
    /// РасположениеРесурсовКомпоновкиДанных.
    DataCompositionResourcesPlacement(
        "РасположениеРесурсовКомпоновкиДанных",
        "DataCompositionResourcesPlacement"
    )
        meta "EnumDataCompositionResourcesPlacement" {
        DataCompositionResourcesPlacementVertically("Вертикально", "Vertically", "Vertical"),
        DataCompositionResourcesPlacementHorizontally(
            "Горизонтально",
            "Horizontally",
            "Horizontal"
        ),
    }
    /// ИспользованиеУсловногоОформленияКомпоновкиДанных.
    DataCompositionConditionalAppearanceUse(
        "ИспользованиеУсловногоОформленияКомпоновкиДанных",
        "DataCompositionConditionalAppearanceUse"
    )
        meta "EnumDataCompositionConditionalAppearanceUse" {
        // Представления подтверждены нативным контролем 8.3.27.2342. Замеры:
        // `tests/conformance/measure/1c/measure-data-composition-*.platform.txt`
        // для `conditional-appearance-use`, `field-placement`,
        // `accounting-balance-type`, `balance-type` и `period-type`.
        DataCompositionConditionalAppearanceUseUse("Использовать", "Use", "Use"),
        DataCompositionConditionalAppearanceUseDontUse("НеИспользовать", "DontUse", "Do not use"),
    }
    /// РасположениеПоляКомпоновкиДанных.
    DataCompositionFieldPlacement(
        "РасположениеПоляКомпоновкиДанных",
        "DataCompositionFieldPlacement"
    )
        meta "EnumDataCompositionFieldPlacement" {
        DataCompositionFieldPlacementAuto("Авто", "Auto", "Auto"),
        DataCompositionFieldPlacementVertically("Вертикально", "Vertically", "Vertical"),
        DataCompositionFieldPlacementTogether("Вместе", "Together", "Combined"),
        DataCompositionFieldPlacementHorizontally("Горизонтально", "Horizontally", "Horizontal"),
        DataCompositionFieldPlacementSpecialColumn(
            "ОтдельнаяКолонка",
            "SpecialColumn",
            "Separate column"
        ),
    }
    /// ТипБухгалтерскогоОстаткаКомпоновкиДанных.
    DataCompositionAccountingBalanceType(
        "ТипБухгалтерскогоОстаткаКомпоновкиДанных",
        "DataCompositionAccountingBalanceType"
    )
        meta "EnumDataCompositionAccountingBalanceType" {
        DataCompositionAccountingBalanceTypeDebit("Дебет", "Debit", "Debit"),
        DataCompositionAccountingBalanceTypeCredit("Кредит", "Credit", "Credit"),
        DataCompositionAccountingBalanceTypeNone("Нет", "None", "None"),
    }
    /// ТипОстаткаКомпоновкиДанных.
    DataCompositionBalanceType("ТипОстаткаКомпоновкиДанных", "DataCompositionBalanceType")
        meta "EnumDataCompositionBalanceType" {
        DataCompositionBalanceTypeOpeningBalance(
            "НачальныйОстаток",
            "OpeningBalance",
            "Opening balance"
        ),
        DataCompositionBalanceTypeClosingBalance(
            "КонечныйОстаток",
            "ClosingBalance",
            "Closing balance"
        ),
        DataCompositionBalanceTypeNone("Нет", "None", "None"),
    }
    /// ТипПериодаКомпоновкиДанных.
    DataCompositionPeriodType("ТипПериодаКомпоновкиДанных", "DataCompositionPeriodType")
        meta "EnumDataCompositionPeriodType" {
        DataCompositionPeriodTypeMain("Основной", "Main", "Default"),
        DataCompositionPeriodTypeAdditional("Дополнительный", "Additional", "Additional"),
    }
    /// ТипДополненияПериодаКомпоновкиДанных.
    DataCompositionPeriodAdditionType(
        "ТипДополненияПериодаКомпоновкиДанных",
        "DataCompositionPeriodAdditionType"
    )
        meta "EnumDataCompositionPeriodAdditionType" {
        DataCompositionPeriodAdditionTypeNone("БезДополнения", "None", "No addition"),
        DataCompositionPeriodAdditionTypeYear("Год", "Year", "Year"),
        DataCompositionPeriodAdditionTypeYearSinceBeginOfPeriod(
            "ГодОтНачалаПериода",
            "YearSinceBeginOfPeriod",
            "Year from the beginning of the period"
        ),
        DataCompositionPeriodAdditionTypeYearSinceBeginOfPeriod445(
            "ГодОтНачалаПериода445",
            "YearSinceBeginOfPeriod445",
            "Year from the beginning of the period (445)"
        ),
        DataCompositionPeriodAdditionTypeTenDays("Декада", "TenDays", "Ten-day period"),
        DataCompositionPeriodAdditionTypeDay("День", "Day", "Day"),
        DataCompositionPeriodAdditionTypeDaySinceBeginOfPeriod(
            "ДеньОтНачалаПериода",
            "DaySinceBeginOfPeriod",
            "Day from the beginning of the period"
        ),
        DataCompositionPeriodAdditionTypeQuarter("Квартал", "Quarter", "Quarter"),
        DataCompositionPeriodAdditionTypeQuarterSinceBeginOfPeriod(
            "КварталОтНачалаПериода",
            "QuarterSinceBeginOfPeriod",
            "Quarter from the beginning of the period"
        ),
        DataCompositionPeriodAdditionTypeQuarterSinceBeginOfPeriod445(
            "КварталОтНачалаПериода445",
            "QuarterSinceBeginOfPeriod445",
            "Quarter from the beginning of the period (445)"
        ),
        DataCompositionPeriodAdditionTypeMonth("Месяц", "Month", "Month"),
        DataCompositionPeriodAdditionTypeMonthSinceBeginOfPeriod(
            "МесяцОтНачалаПериода",
            "MonthSinceBeginOfPeriod",
            "Month from the beginning of the period"
        ),
        DataCompositionPeriodAdditionTypeMonthSinceBeginOfPeriod445(
            "МесяцОтНачалаПериода445",
            "MonthSinceBeginOfPeriod445",
            "Month from the beginning of the period (445)"
        ),
        DataCompositionPeriodAdditionTypeMinute("Минута", "Minute", "Minute"),
        DataCompositionPeriodAdditionTypeMinuteSinceBeginOfPeriod(
            "МинутаОтНачалаПериода",
            "MinuteSinceBeginOfPeriod",
            "Minute from the beginning of the period"
        ),
        DataCompositionPeriodAdditionTypeWeek("Неделя", "Week", "Week"),
        DataCompositionPeriodAdditionTypeWeekSinceBeginOfPeriod(
            "НеделяОтНачалаПериода",
            "WeekSinceBeginOfPeriod",
            "Week from the beginning of the period"
        ),
        DataCompositionPeriodAdditionTypeHalfYear("Полугодие", "HalfYear", "Half year"),
        DataCompositionPeriodAdditionTypeHalfYearSinceBeginOfPeriod(
            "ПолугодиеОтНачалаПериода",
            "HalfYearSinceBeginOfPeriod",
            "Six months from the beginning of the period"
        ),
        DataCompositionPeriodAdditionTypeHalfYearSinceBeginOfPeriod445(
            "ПолугодиеОтНачалаПериода445",
            "HalfYearSinceBeginOfPeriod445",
            "Six months from the beginning of the period (445)"
        ),
        DataCompositionPeriodAdditionTypeSecond("Секунда", "Second", "Second"),
        DataCompositionPeriodAdditionTypeHour("Час", "Hour", "Hour"),
        DataCompositionPeriodAdditionTypeHourSinceBeginOfPeriod(
            "ЧасОтНачалаПериода",
            "HourSinceBeginOfPeriod",
            "Hour from the beginning of the period"
        ),
    }
    /// ГлубинаЦвета.
    ColorDepth("ГлубинаЦвета", "ColorDepth") meta "EnumColorDepth" {
        // Имена и представления измерены на 8.3.27 в `measure-next-session-enums.bsl`.
        ColorDepthBitPerPixel1("БитНаПиксел1", "BitPerPixel1", "1 bit per pixel"),
        ColorDepthBitPerPixel24("БитНаПиксел24", "BitPerPixel24", "24 bits per pixel"),
        ColorDepthBitPerPixel32("БитНаПиксел32", "BitPerPixel32", "32 bits per pixel"),
        ColorDepthBitPerPixel4("БитНаПиксел4", "BitPerPixel4", "4 bits per pixel"),
        ColorDepthBitPerPixel8("БитНаПиксел8", "BitPerPixel8", "8 bits per pixel"),
    }
    /// ОтображениеПодсказки.
    ToolTipRepresentation("ОтображениеПодсказки", "ToolTipRepresentation")
        meta "EnumToolTipRepresentation" {
        ToolTipRepresentationAuto("Авто", "Auto", "Auto"),
        ToolTipRepresentationBalloon("Всплывающая", "Balloon", "Tooltip"),
        ToolTipRepresentationButton("Кнопка", "Button", "Button"),
        ToolTipRepresentationNone("Нет", "None", "No"),
        ToolTipRepresentationShowAuto("ОтображатьАвто", "ShowAuto", "Display automatically"),
        ToolTipRepresentationShowTop("ОтображатьСверху", "ShowTop", "Display above"),
        ToolTipRepresentationShowLeft("ОтображатьСлева", "ShowLeft", "Display to the left"),
        ToolTipRepresentationShowBottom("ОтображатьСнизу", "ShowBottom", "Display below"),
        ToolTipRepresentationShowRight("ОтображатьСправа", "ShowRight", "Display to the right"),
    }
    /// ТипКлиентскогоПриложения.
    ClientApplicationType("ТипКлиентскогоПриложения", "ClientApplicationType")
        meta "EnumClientApplicationType" {
        ClientApplicationTypeWebClient("ВебКлиент", "WebClient", "Web client"),
        ClientApplicationTypeExternalConnection(
            "ВнешнееСоединение",
            "ExternalConnection",
            "External connection"
        ),
        ClientApplicationTypeMobileAppClient(
            "МобильноеПриложениеКлиент",
            "MobileAppClient",
            "Mobile application client"
        ),
        ClientApplicationTypeMobileClient("МобильныйКлиент", "MobileClient", "Mobile client"),
        ClientApplicationTypeThickClient("ТолстыйКлиент", "ThickClient", "Thick client"),
        ClientApplicationTypeThinClient("ТонкийКлиент", "ThinClient", "Thin client"),
    }
    /// Тип вывода текста компоновки данных.
    DataCompositionTextOutputType(
        "ТипВыводаТекстаКомпоновкиДанных",
        "DataCompositionTextOutputType"
    )
        meta "EnumDataCompositionTextOutputType" {
        // Полный состав и представления измерены на платформе 8.3.27.2342
        // пробой `measure-data-composition-text-output-type.bsl`.
        DataCompositionTextOutputTypeAuto("Авто", "Auto", "Auto"),
        DataCompositionTextOutputTypeOutput("Выводить", "Output", "Output"),
        DataCompositionTextOutputTypeDontOutput("НеВыводить", "DontOutput", "Do not output"),
    }
    /// Тип размещения текста компоновки данных.
    // ИЗМЕРЕНО на 8.3.27 пробой
    // `measure-data-composition-text-output-type.bsl`.
    DataCompositionTextPlacementType(
        "ТипРазмещенияТекстаКомпоновкиДанных",
        "DataCompositionTextPlacementType"
    )
        meta "EnumDataCompositionTextPlacementType" {
        DataCompositionTextPlacementTypeOverflow("Выступать", "Overflow", "Overflow"),
        // Типы, члены, алиасы и представления измерены на 8.3.27.2342.
        DataCompositionTextPlacementTypeBlock("Забивать", "Block", "Conceal"),
        DataCompositionTextPlacementTypeCut("Обрезать", "Cut", "Clip"),
        DataCompositionTextPlacementTypeWrap("Переносить", "Wrap", "Wrap"),
    }
    /// Режим масштабирования просмотра документа.
    // ИЗМЕРЕНО на 8.3.27 пробой `measure-view-scaling-mode.bsl`.
    ViewScalingMode("РежимМасштабированияПросмотра", "ViewScalingMode") meta "EnumViewScalingMode" {
        // Полный состав и представления измерены на платформе 8.3.27.2342
        // пробой `measure-view-scaling-mode.bsl`.
        ViewScalingModeAuto("Авто", "Auto", "Auto"),
        ViewScalingModeLarge("Крупный", "Large", "Large"),
        ViewScalingModeNormal("Обычный", "Normal", "Normal"),
    }
    /// Расположение итогов компоновки данных.
    // ИЗМЕРЕНО на 8.3.27 пробой
    // `measure-data-composition-total-placement.bsl`.
    DataCompositionTotalPlacement(
        "РасположениеИтоговКомпоновкиДанных",
        "DataCompositionTotalPlacement"
    )
        meta "EnumDataCompositionTotalPlacement" {
        // Полный состав и представления измерены на платформе 8.3.27.2342
        // пробой `measure-data-composition-total-placement.bsl`.
        DataCompositionTotalPlacementAuto("Авто", "Auto", "Auto"),
        DataCompositionTotalPlacementEnd("Конец", "End", "End"),
        DataCompositionTotalPlacementBegin("Начало", "Begin", "Begin"),
        DataCompositionTotalPlacementBeginAndEnd("НачалоИКонец", "BeginAndEnd", "Begin and end"),
        DataCompositionTotalPlacementNone("Нет", "None", "None"),
    }
    /// Размер картинки в области отображения.
    // ИЗМЕРЕНО на 8.3.27 пробой `measure-picture-size.bsl`.
    PictureSize("РазмерКартинки", "PictureSize") meta "EnumPictureSize" {
        // Полный состав и представления измерены на платформе 8.3.27.2342
        // пробой `measure-picture-size.bsl`.
        PictureSizeAutoSize("АвтоРазмер", "AutoSize", "Automatic size"),
        PictureSizeAutoSizeIgnoreScale(
            "АвтоРазмерБезУчетаМасштаба",
            "AutoSizeIgnoreScale",
            "Automatic size without scale"
        ),
        PictureSizeByFontSize("ПоРазмеруШрифта", "ByFontSize", "By font size"),
        PictureSizeProportionally("Пропорционально", "Proportionally", "Proportional"),
        PictureSizeStretch("Растянуть", "Stretch", "Stretch"),
        PictureSizeRealSize("РеальныйРазмер", "RealSize", "Actual size"),
        PictureSizeRealSizeIgnoreScale(
            "РеальныйРазмерБезУчетаМасштаба",
            "RealSizeIgnoreScale",
            "Actual size (no scaling)"
        ),
        PictureSizeTile("Черепица", "Tile", "Tile"),
    }
    /// Тип смещения ячеек табличного документа.
    // ИЗМЕРЕНО на 8.3.27 пробой
    // `measure-spreadsheet-document-shift-type.bsl`.
    SpreadsheetDocumentShiftType("ТипСмещенияТабличногоДокумента", "SpreadsheetDocumentShiftType")
        meta "EnumSpreadsheetDocumentShiftType" {
        // Полный состав и представления измерены на платформе 8.3.27.2342
        // пробой `measure-spreadsheet-document-shift-type.bsl`.
        SpreadsheetDocumentShiftTypeWithoutShift("БезСмещения", "WithoutShift", "No shift"),
        SpreadsheetDocumentShiftTypeVertical("ПоВертикали", "Vertical", "Vertically"),
        SpreadsheetDocumentShiftTypeHorizontal("ПоГоризонтали", "Horizontal", "Horizontally"),
    }
    /// Тип диаграммы.
    // ИЗМЕРЕНО на 8.3.27 пробой `measure-chart-type.bsl`.
    ChartType("ТипДиаграммы", "ChartType") meta "EnumChartType" {
        // Полный состав и представления измерены на платформе 8.3.27.2342
        // пробой `measure-chart-type.bsl`.
        ChartTypeStock("Биржевая", "Stock", "Stock"),
        ChartTypeOpenHighLowClose("БиржеваяСвеча", "OpenHighLowClose", "Open-High\"-Low-Close\""),
        ChartTypeConcaveSurface("ВогнутаяПоверхность", "ConcaveSurface", "Concave surface"),
        ChartTypeWaterfall("Водопад", "Waterfall", "Waterfall"),
        ChartTypeFunnel("Воронка", "Funnel", "Cone"),
        ChartTypeNormalizedFunnel("ВоронкаНормированная", "NormalizedFunnel", "Standardized cone"),
        ChartTypeNormalizedFunnel3D(
            "ВоронкаНормированнаяОбъемная",
            "NormalizedFunnel3D",
            "Standardized volumetric cone"
        ),
        ChartTypeFunnel3D("ВоронкаОбъемная", "Funnel3D", "Volumetric cone"),
        ChartTypeConvexSurface("ВыпуклаяПоверхность", "ConvexSurface", "Convex surface"),
        ChartTypeColumn("Гистограмма", "Column", "Column"),
        ChartTypeBar("ГистограммаГоризонтальная", "Bar", "Bar"),
        ChartTypeBar3D("ГистограммаГоризонтальнаяОбъемная", "Bar3D", "Bar 3D"),
        ChartTypeNormalizedColumn(
            "ГистограммаНормированная",
            "NormalizedColumn",
            "Normalized bar chart"
        ),
        ChartTypeNormalizedBar(
            "ГистограммаНормированнаяГоризонтальная",
            "NormalizedBar",
            "Normalized horizontal bar"
        ),
        ChartTypeNormalizedBar3D(
            "ГистограммаНормированнаяГоризонтальнаяОбъемная",
            "NormalizedBar3D",
            "Normalized horizontal 3D column"
        ),
        ChartTypeNormalizedColumn3D(
            "ГистограммаНормированнаяОбъемная",
            "NormalizedColumn3D",
            "Normalized 3D column"
        ),
        ChartTypeColumn3D("ГистограммаОбъемная", "Column3D", "3D column"),
        ChartTypeStackedColumn("ГистограммаСНакоплением", "StackedColumn", "Stacked column"),
        ChartTypeStackedBar("ГистограммаСНакоплениемГоризонтальная", "StackedBar", "Stacked Bar"),
        ChartTypeStackedBar3D(
            "ГистограммаСНакоплениемГоризонтальнаяОбъемная",
            "StackedBar3D",
            "Stacked bar 3D "
        ),
        ChartTypeStackedColumn3D(
            "ГистограммаСНакоплениемОбъемная",
            "StackedColumn3D",
            "Stacked column 3D"
        ),
        ChartTypeLine("График", "Line", "Line"),
        ChartTypeStep("ГрафикПоШагам", "Step", "Step"),
        ChartTypeStackedLine("ГрафикСНакоплением", "StackedLine", "Stacked graph"),
        ChartTypeArea("ГрафикСОбластями", "Area", "Area"),
        ChartTypeStackedArea("ГрафикСОбластямиИНакоплением", "StackedArea", "Stacked Area"),
        ChartTypeNormalizedArea(
            "ГрафикСОбластямиНормированный",
            "NormalizedArea",
            "Normalized area graph"
        ),
        ChartTypeShadedSurface("ЗатененнаяПоверхность", "ShadedSurface", "Shaded surface"),
        ChartTypeGauge("Измерительная", "Gauge", "Gauge"),
        ChartTypeBarGraph("Изометрическая", "BarGraph", "Bar graph"),
        ChartTypeTapeGraph("ИзометрическаяЛента", "TapeGraph", "Tape Graph"),
        ChartTypeCeilGraph("ИзометрическаяНепрерывная", "CeilGraph", "Ceil Graph"),
        ChartTypePyramidGraph("ИзометрическаяПирамида", "PyramidGraph", "Pyramid Graph"),
        ChartTypeWireframeSurface("КаркаснаяПоверхность", "WireframeSurface", "Wire-frame surface"),
        ChartTypeDonut("Кольцевая", "Donut", "Doughnut"),
        ChartTypeDonut3D("КольцеваяОбъемная", "Donut3D", "3D doughnut"),
        ChartTypePie("Круговая", "Pie", "Pie"),
        ChartTypePie3D("КруговаяОбъемная", "Pie3D", "Pie 3D"),
        ChartTypeSurface("Поверхность", "Surface", "Surface"),
        ChartTypeBubble("Пузырьковая", "Bubble", "Bubble"),
        ChartTypeRadarLine("РадарныйГрафик", "RadarLine", "Radar chart"),
        ChartTypeRadarStackedLine(
            "РадарныйГрафикСНакоплением",
            "RadarStackedLine",
            "Stacked radar chart"
        ),
        ChartTypeRadarArea("РадарныйГрафикСОбластями", "RadarArea", "Radar area chart"),
        ChartTypeRadarStackedArea(
            "РадарныйГрафикСОбластямиИНакоплением",
            "RadarStackedArea",
            "Stacked radar area chart"
        ),
        ChartTypeRadarNormalizedArea(
            "РадарныйГрафикСОбластямиНормированный",
            "RadarNormalizedArea",
            "Normalized radar area chart"
        ),
        ChartTypeHoneycomb("Сотовая", "Honeycomb", "Honeycomb"),
        ChartTypeScatter("Точечная", "Scatter", "Scatter"),
    }
    /// Ориентация подписей диаграммы.
    // ИЗМЕРЕНО на 8.3.27 пробой
    // `measure-chart-label-orientation-name.bsl`.
    ChartLabelsOrientation("ОриентацияПодписейДиаграммы", "ChartLabelsOrientation")
        meta "EnumChartLabelsOrientation"
        global ["ОриентацияПодписейДиаграммы", "ОриентацияМетокДиаграммы", "ChartLabelsOrientation"] {
        // Оба русских имени и представления измерены на платформе 8.3.27.2342
        // пробой `measure-chart-label-orientation-name.bsl`.
        ChartLabelsOrientationAuto("Авто", "Auto", "Auto"),
        ChartLabelsOrientationVertical("Вертикально", "Vertical", "Vertical"),
        ChartLabelsOrientationHorizontal("Горизонтально", "Horizontal", "Horizontal"),
        ChartLabelsOrientationCustomAngle("ПроизвольныйУголНаклона", "CustomAngle", "Custom angle"),
    }
    /// Вид включения граничного значения.
    // ИЗМЕРЕНО на 8.3.27 пробой `measure-boundary-type.bsl`.
    BoundaryType("ВидГраницы", "BoundaryType") meta "EnumBoundaryType" {
        // Полный состав и представления измерены на платформе 8.3.27.2342
        // пробой `measure-boundary-type.bsl`.
        BoundaryTypeIncluding("Включая", "Including", "Including"),
        BoundaryTypeExcluding("Исключая", "Excluding", "Excluding"),
    }
    /// Режим ограничения количества серий диаграммы.
    // ИЗМЕРЕНО на 8.3.27 пробой `measure-max-series.bsl`.
    MaxSeries("МаксимумСерий", "MaxSeries") meta "EnumMaxSeries" {
        // Полный состав и представления измерены на платформе 8.3.27.2342
        // пробой `measure-max-series.bsl`.
        MaxSeriesNotDefined("НеЗадано", "NotDefined", "Not set"),
        MaxSeriesLimited("Ограничено", "Limited", "Limited"),
        MaxSeriesPercent("Процент", "Percent", "Percent"),
    }
    /// Вид подписи к области диаграммы.
    // ИЗМЕРЕНО на 8.3.27 пробой `measure-chart-label-type.bsl`.
    ChartLabelType("ВидПодписейКДиаграмме", "ChartLabelType") meta "EnumChartLabelType" {
        // Полный состав и представления измерены на платформе 8.3.27.2342
        // пробой `measure-chart-label-type.bsl`.
        ChartLabelTypeValue("Значение", "Value", "Value"),
        ChartLabelTypeValuePercent("ЗначениеПроцент", "ValuePercent", "Value + percent"),
        ChartLabelTypeValueSize("ЗначениеРазмер", "ValueSize", "Value + size"),
        ChartLabelTypeNone("Нет", "None", "None"),
        ChartLabelTypePercent("Процент", "Percent", "Percent"),
        ChartLabelTypeSeries("Серия", "Series", "Series"),
        ChartLabelTypeSeriesValue("СерияЗначение", "SeriesValue", "Series + value"),
        ChartLabelTypeSeriesValuePercent(
            "СерияЗначениеПроцент",
            "SeriesValuePercent",
            "Series + value + percent"
        ),
        ChartLabelTypeSeriesValueSize(
            "СерияЗначениеРазмер",
            "SeriesValueSize",
            "Series + value + size"
        ),
        ChartLabelTypeSeriesPercent("СерияПроцент", "SeriesPercent", "Series + percent"),
        ChartLabelTypeSeriesSize("СерияРазмер", "SeriesSize", "Series + size"),
        ChartLabelTypeSeriesPoint("СерияТочка", "SeriesPoint", "Series + point"),
        ChartLabelTypeSeriesPointValue(
            "СерияТочкаЗначение",
            "SeriesPointValue",
            "Series + point + value"
        ),
        ChartLabelTypeSeriesPointValuePercent(
            "СерияТочкаЗначениеПроцент",
            "SeriesPointValuePercent",
            "Series + point + value + percent"
        ),
        ChartLabelTypeSeriesPointValueSize(
            "СерияТочкаЗначениеРазмер",
            "SeriesPointValueSize",
            "Series + point + value + size"
        ),
        ChartLabelTypeSeriesPointPercent(
            "СерияТочкаПроцент",
            "SeriesPointPercent",
            "Series + point + percent"
        ),
        ChartLabelTypeSeriesPointSize(
            "СерияТочкаРазмер",
            "SeriesPointSize",
            "Series + point + size"
        ),
        ChartLabelTypePoint("Точка", "Point", "Point"),
        ChartLabelTypePointValue("ТочкаЗначение", "PointValue", "Point + value"),
        ChartLabelTypePointValuePercent(
            "ТочкаЗначениеПроцент",
            "PointValuePercent",
            "Point + value + percent"
        ),
        ChartLabelTypePointValueSize(
            "ТочкаЗначениеРазмер",
            "PointValueSize",
            "Point + value + size"
        ),
        ChartLabelTypePointPercent("ТочкаПроцент", "PointPercent", "Point + percent"),
        ChartLabelTypePointSize("ТочкаРазмер", "PointSize", "Point + size"),
    }
    /// Стиль линии диаграммы.
    // ИЗМЕРЕНО на 8.3.27 пробой `measure-chart-line-type.bsl`.
    ChartLineType("ТипЛинииДиаграммы", "ChartLineType") meta "EnumChartLineType" {
        // Полный состав и представления измерены на платформе 8.3.27.2342
        // пробой `measure-chart-line-type.bsl`.
        ChartLineTypeNone("НетЛинии", "None", "None"),
        ChartLineTypeDashed("Пунктир", "Dashed", "Dashed"),
        ChartLineTypeDashDotted("ПунктирТочка", "DashDotted", "DashDotted"),
        ChartLineTypeDashDottedDotted("ПунктирТочкаТочка", "DashDottedDotted", "DotDashDotted"),
        ChartLineTypeSolid("Сплошная", "Solid", "Solid"),
        ChartLineTypeDotted("Точечная", "Dotted", "Dotted"),
    }
    /// Вид кнопки формы.
    // ИЗМЕРЕНО на 8.3.27 пробой `measure-form-button-type.bsl`.
    FormButtonType("ВидКнопкиФормы", "FormButtonType") meta "EnumFormButtonType" {
        // Полный состав и представления измерены на платформе 8.3.27.2342
        // пробой `measure-form-button-type.bsl`.
        FormButtonTypeHyperlink("Гиперссылка", "Hyperlink", "Hyperlink"),
        FormButtonTypeCommandBarHyperlink(
            "ГиперссылкаКоманднойПанели",
            "CommandBarHyperlink",
            "Command bar hyperlink"
        ),
        FormButtonTypeCommandBarButton(
            "КнопкаКоманднойПанели",
            "CommandBarButton",
            "Command bar button"
        ),
        FormButtonTypeUsualButton("ОбычнаяКнопка", "UsualButton", "Common button"),
    }
    /// Способ назначения типа при записи значения в XML.
    XmlTypeAssignment("НазначениеТипаXML", "XMLTypeAssignment") meta "XMLTypeAssignment" {
        // Полный состав и представления измерены на платформе 8.3.27.2074
        // пробой `measure-xml-type-assignment.bsl`.
        XmlTypeAssignmentImplicit("Неявное", "Implicit", "Implicit"),
        XmlTypeAssignmentExplicit("Явное", "Explicit", "Explicit"),
    }
    /// Тип операционной системы и архитектуры платформы.
    PlatformType("ТипПлатформы", "PlatformType") meta "PlatformType" {
        // Полный состав и представления измерены на платформе 8.3.27.2074
        // пробой `measure-platform-type.bsl`.
        PlatformTypeAndroidArm("Android_ARM", "Android_ARM", "Android ARM"),
        PlatformTypeAndroidArm64("Android_ARM_64", "Android_ARM_64", "Android ARM_64"),
        PlatformTypeAndroidX86("Android_x86", "Android_x86", "Android x86"),
        PlatformTypeAndroidX86_64("Android_x86_64", "Android_x86_64", "Android x86_64"),
        PlatformTypeIosArm("iOS_ARM", "iOS_ARM", "iOS ARM"),
        PlatformTypeIosArm64("iOS_ARM_64", "iOS_ARM_64", "iOS ARM_64"),
        PlatformTypeLinuxArm64("Linux_ARM64", "Linux_ARM64", "Linux ARM64"),
        PlatformTypeLinuxE2k("Linux_E2K", "Linux_E2K", "Linux E2K"),
        PlatformTypeLinuxX86("Linux_x86", "Linux_x86", "Linux x86"),
        PlatformTypeLinuxX86_64("Linux_x86_64", "Linux_x86_64", "Linux x86-64"),
        PlatformTypeMacOsX86("MacOS_x86", "MacOS_x86", "MacOS x86"),
        PlatformTypeMacOsX86_64("MacOS_x86_64", "MacOS_x86_64", "MacOS x86-64"),
        PlatformTypeWindowsX86("Windows_x86", "Windows_x86", "Windows x86"),
        PlatformTypeWindowsX86_64("Windows_x86_64", "Windows_x86_64", "Windows x86-64"),
        PlatformTypeWinRtArm("WinRT_ARM", "WinRT_ARM", "WinRT ARM"),
        PlatformTypeWinRtX86("WinRT_x86", "WinRT_x86", "WinRT x86"),
        PlatformTypeWinRtX86_64("WinRT_x86_64", "WinRT_x86_64", "WinRT x86_64"),
    }
    /// Операция сравнения в настройках системы компоновки данных.
    DataCompositionComparisonType("ВидСравненияКомпоновкиДанных", "DataCompositionComparisonType")
        meta "DataCompositionComparisonType" {
        // Состав, оба написания и представления измерены на платформе 8.3.27.2074
        // пробой `measure-data-composition-comparison-type.bsl`.
        DataCompositionComparisonBeginsWith("НачинаетсяС", "BeginsWith", "Begins with"),
        DataCompositionComparisonContains("Содержит", "Contains", "Contains"),
        DataCompositionComparisonEqual("Равно", "Equal", "Equal to"),
        DataCompositionComparisonFilled("Заполнено", "Filled", "Filled"),
        DataCompositionComparisonGreater("Больше", "Greater", "Greater than"),
        DataCompositionComparisonGreaterOrEqual(
            "БольшеИлиРавно",
            "GreaterOrEqual",
            "Greater than or equal to"
        ),
        DataCompositionComparisonInHierarchy("ВИерархии", "InHierarchy", "In group"),
        DataCompositionComparisonInList("ВСписке", "InList", "In list"),
        DataCompositionComparisonInListByHierarchy(
            "ВСпискеПоИерархии",
            "InListByHierarchy",
            "In list group"
        ),
        DataCompositionComparisonLess("Меньше", "Less", "Less than"),
        DataCompositionComparisonLessOrEqual(
            "МеньшеИлиРавно",
            "LessOrEqual",
            "Less than or equal to"
        ),
        DataCompositionComparisonLike("Подобно", "Like", "Like"),
        DataCompositionComparisonNotBeginsWith(
            "НеНачинаетсяС",
            "NotBeginsWith",
            "Does not begin with"
        ),
        DataCompositionComparisonNotContains("НеСодержит", "NotContains", "Does not contain"),
        DataCompositionComparisonNotEqual("НеРавно", "NotEqual", "Not equal"),
        DataCompositionComparisonNotFilled("НеЗаполнено", "NotFilled", "Empty"),
        DataCompositionComparisonNotInHierarchy("НеВИерархии", "NotInHierarchy", "Not in group"),
        DataCompositionComparisonNotInList("НеВСписке", "NotInList", "Not in list"),
        DataCompositionComparisonNotInListByHierarchy(
            "НеВСпискеПоИерархии",
            "NotInListByHierarchy",
            "Not in list group"
        ),
        DataCompositionComparisonNotLike("НеПодобно", "NotLike", "Not like"),
    }
    /// Операция сравнения в обычном отборе.
    ComparisonType("ВидСравнения", "ComparisonType") meta "EnumComparisonType" {
        // Состав, оба написания и представления измерены на платформе 8.3.27
        // пробой `measure-comparison-type.bsl`.
        ComparisonGreater("Больше", "Greater", "Greater than"),
        ComparisonGreaterOrEqual("БольшеИлиРавно", "GreaterOrEqual", "Greater than or equal to"),
        ComparisonInHierarchy("ВИерархии", "InHierarchy", "In group"),
        ComparisonInList("ВСписке", "InList", "In list"),
        ComparisonInListByHierarchy("ВСпискеПоИерархии", "InListByHierarchy", "In list group"),
        ComparisonInterval("Интервал", "Interval", "Range (>, <)"),
        ComparisonIntervalIncludingBounds(
            "ИнтервалВключаяГраницы",
            "IntervalIncludingBounds",
            "Range (>=, <=)"
        ),
        ComparisonIntervalIncludingLowerBound(
            "ИнтервалВключаяНачало",
            "IntervalIncludingLowerBound",
            "Range (>=, <)"
        ),
        ComparisonIntervalIncludingUpperBound(
            "ИнтервалВключаяОкончание",
            "IntervalIncludingUpperBound",
            "Range (>, <=)"
        ),
        ComparisonLess("Меньше", "Less", "Less than"),
        ComparisonLessOrEqual("МеньшеИлиРавно", "LessOrEqual", "Less than or equal to"),
        ComparisonNotInHierarchy("НеВИерархии", "NotInHierarchy", "Not in group"),
        ComparisonNotInList("НеВСписке", "NotInList", "Not in list"),
        ComparisonNotInListByHierarchy(
            "НеВСпискеПоИерархии",
            "NotInListByHierarchy",
            "Not in list group"
        ),
        ComparisonNotEqual("НеРавно", "NotEqual", "Not equal to"),
        ComparisonNotContains("НеСодержит", "NotContains", "Does not contain"),
        ComparisonEqual("Равно", "Equal", "Equal to"),
        ComparisonContains("Содержит", "Contains", "Contains"),
    }
    /// Относительный период либо произвольная стандартная дата начала.
    StandardBeginningDateVariant("ВариантСтандартнойДатыНачала", "StandardBeginningDateVariant")
        meta "StandardBeginningDateVariant" {
        // Состав, оба написания и представления измерены на платформе 8.3.27.2074
        // пробой `measure-standard-beginning-date.bsl`.
        StandardBeginningDateThisDay(
            "НачалоЭтогоДня",
            "BeginningOfThisDay",
            "Beginning of this day"
        ),
        StandardBeginningDateThisWeek(
            "НачалоЭтойНедели",
            "BeginningOfThisWeek",
            "Beginning of this week"
        ),
        StandardBeginningDateThisTenDays(
            "НачалоЭтойДекады",
            "BeginningOfThisTenDays",
            "Beginning of this ten-day period"
        ),
        StandardBeginningDateThisMonth(
            "НачалоЭтогоМесяца",
            "BeginningOfThisMonth",
            "Beginning of this month"
        ),
        StandardBeginningDateThisQuarter(
            "НачалоЭтогоКвартала",
            "BeginningOfThisQuarter",
            "Beginning of this quarter"
        ),
        StandardBeginningDateThisHalfYear(
            "НачалоЭтогоПолугодия",
            "BeginningOfThisHalfYear",
            "Beginning of this half year"
        ),
        StandardBeginningDateThisYear(
            "НачалоЭтогоГода",
            "BeginningOfThisYear",
            "Beginning of this year"
        ),
        StandardBeginningDateLastDay(
            "НачалоПрошлогоДня",
            "BeginningOfLastDay",
            "Beginning of yesterday"
        ),
        StandardBeginningDateLastWeek(
            "НачалоПрошлойНедели",
            "BeginningOfLastWeek",
            "Beginning of last week"
        ),
        StandardBeginningDateLastTenDays(
            "НачалоПрошлойДекады",
            "BeginningOfLastTenDays",
            "Beginning of last ten-day period"
        ),
        StandardBeginningDateLastMonth(
            "НачалоПрошлогоМесяца",
            "BeginningOfLastMonth",
            "Beginning of last month"
        ),
        StandardBeginningDateLastQuarter(
            "НачалоПрошлогоКвартала",
            "BeginningOfLastQuarter",
            "Beginning of last quarter"
        ),
        StandardBeginningDateLastHalfYear(
            "НачалоПрошлогоПолугодия",
            "BeginningOfLastHalfYear",
            "Beginning of last half year"
        ),
        StandardBeginningDateLastYear(
            "НачалоПрошлогоГода",
            "BeginningOfLastYear",
            "Beginning of last year"
        ),
        StandardBeginningDateNextDay(
            "НачалоСледующегоДня",
            "BeginningOfNextDay",
            "Beginning of tomorrow"
        ),
        StandardBeginningDateNextWeek(
            "НачалоСледующейНедели",
            "BeginningOfNextWeek",
            "Beginning of next week"
        ),
        StandardBeginningDateNextTenDays(
            "НачалоСледующейДекады",
            "BeginningOfNextTenDays",
            "Beginning of next ten-day period"
        ),
        StandardBeginningDateNextMonth(
            "НачалоСледующегоМесяца",
            "BeginningOfNextMonth",
            "Beginning of next month"
        ),
        StandardBeginningDateNextQuarter(
            "НачалоСледующегоКвартала",
            "BeginningOfNextQuarter",
            "Beginning of next quarter"
        ),
        StandardBeginningDateNextHalfYear(
            "НачалоСледующегоПолугодия",
            "BeginningOfNextHalfYear",
            "Beginning of next half year"
        ),
        StandardBeginningDateNextYear(
            "НачалоСледующегоГода",
            "BeginningOfNextYear",
            "Beginning of next year"
        ),
        StandardBeginningDateCustom("ПроизвольнаяДата", "Custom", "Custom date"),
    }
    JsonValueType("ТипЗначенияJSON", "JSONValueType") meta "ПеречислениеТипЗначенияJSON" {
        JsonString("Строка", "String", "Строка"),
        JsonNumber("Число", "Number", "Число"),
        JsonBoolean("Булево", "Boolean", "Булево"),
        JsonNull("Null", "Null", "Значение Null"),
        JsonObjectStart("НачалоОбъекта", "ObjectStart", "Начало объекта"),
        JsonObjectEnd("КонецОбъекта", "ObjectEnd", "Конец объекта"),
        JsonArrayStart("НачалоМассива", "ArrayStart", "Начало массива"),
        // ХВОСТОВОЙ ПРОБЕЛ — измерено, не описка.
        JsonArrayEnd("КонецМассива", "ArrayEnd", "Конец массива "),
        // И здесь тоже.
        JsonPropertyName("ИмяСвойства", "PropertyName", "Имя свойства "),
        JsonNothing("Ничего", "None", "Нет"),
        JsonComment("Комментарий", "Comment", "Комментарий"),
    }
    JsonLineBreak("ПереносСтрокJSON", "JSONLineBreak") meta "ПеречислениеПереносСтрокJSON" {
        LineBreakNone("Нет", "None", "Нет"),
        LineBreakAuto("Авто", "Auto", "Автоматически"),
        LineBreakWindows("Windows", "Windows", "Windows"),
        LineBreakUnix("Unix", "Unix", "Unix"),
    }
    /// Способ перевода Unicode-символов в `\uXXXX` при записи JSON.
    JsonEscapeCharacters("ЭкранированиеСимволовJSON", "JSONCharactersEscapeMode")
        meta "ПеречислениеЭкранированиеСимволовJSON" {
        JsonEscapeNone("Нет", "None", "Нет"),
        JsonEscapeNonAscii("СимволыВнеASCII", "CharactersOutsideASCII", "Символы вне ASCII"),
        JsonEscapeNonBmp("СимволыВнеBMP", "CharactersOutsideBMP", "Символы вне BMP"),
    }
    JsonDateFormat("ФорматДатыJSON", "JSONDateFormat") meta "ПеречислениеФорматДатыJSON" {
        DateFormatIso("ISO", "ISO", "ISO"),
        DateFormatJavaScript("JavaScript", "JavaScript", "JavaScript"),
        DateFormatMicrosoft("Microsoft", "Microsoft", "Microsoft"),
    }
    /// `ВариантЗаписиДатыJSON` — англ. написание `JSONDateWritingVariant`
    /// взято по аналогии с остальными именами перечислений JSON (`JSON` +
    /// суть без пробелов), само не измерялось отдельно; написания ЧЛЕНОВ
    /// ниже — ИЗМЕРЕНО (`JSON.DATE_VARIANT_EN_NAMES`).
    JsonDateWritingVariant("ВариантЗаписиДатыJSON", "JSONDateWritingVariant")
        meta "ПеречислениеВариантЗаписиДатыJSON" {
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
        DateVariantLocal("ЛокальнаяДата", "LocalDate", "Локальная дата"),
        DateVariantLocalOffset(
            "ЛокальнаяДатаСоСмещением",
            "LocalDateWithOffset",
            "Локальная дата со смещением"
        ),
        DateVariantUniversal("УниверсальнаяДата", "UniversalDate", "Универсальная дата"),
    }
    /// `СостояниеФоновогоЗадания` — состояния фонового задания. Русские
    /// имена членов взяты из документации главы 18; точные написания,
    /// английские aliases и представления `Строка()` фиксирует замер
    /// `JOB.API.SURFACE` реестра плана фоновых заданий.
    BackgroundJobState("СостояниеФоновогоЗадания", "BackgroundJobState")
        meta "ПеречислениеСостояниеФоновогоЗадания" {
        BackgroundJobActive("Активно", "Active", "Активно"),
        BackgroundJobCompleted("Завершено", "Completed", "Завершено"),
        BackgroundJobFailed("ЗавершеноАварийно", "Failed", "ЗавершеноАварийно"),
        BackgroundJobCanceled("Отменено", "Canceled", "Отменено"),
    }
    /// `СтатусСообщения` — статусы `Сообщить`; состав членов — по выписке
    /// синтакс-помощника 8.3.27. В управляемом режиме статус игнорируется,
    /// перечисление нужно компиляции вызовов.
    MessageStatus("СтатусСообщения", "MessageStatus") meta "ПеречислениеСтатусСообщения" {
        MessageStatusWithoutStatus("БезСтатуса", "WithoutStatus", "БезСтатуса"),
        MessageStatusImportant("Важное", "Important", "Важное"),
        MessageStatusAttention("Внимание", "Attention", "Внимание"),
        MessageStatusInformation("Информация", "Information", "Информация"),
        MessageStatusOrdinary("Обычное", "Ordinary", "Обычное"),
        MessageStatusVeryImportant("ОченьВажное", "VeryImportant", "ОченьВажное"),
    }
    /// `КатегорияОшибки` — категории `ЯвляетсяОшибкойКатегории`; состав —
    /// по выписке синтакс-помощника 8.3.27.
    ErrorCategory("КатегорияОшибки", "ErrorCategory") meta "ПеречислениеКатегорияОшибки" {
        ErrorCategoryAllErrors("ВсеОшибки", "AllErrors", "ВсеОшибки"),
        ErrorCategoryExceptionRaisedFromScript(
            "ИсключениеВызванноеИзВстроенногоЯзыка",
            "ExceptionRaisedFromScript",
            "ИсключениеВызванноеИзВстроенногоЯзыка"
        ),
        ErrorCategoryAccessViolation(
            "НарушениеПравДоступа",
            "AccessViolation",
            "НарушениеПравДоступа"
        ),
        ErrorCategoryUnsupportedFormat(
            "НеподдерживаемыйФормат",
            "UnsupportedFormat",
            "НеподдерживаемыйФормат"
        ),
        ErrorCategoryInvalidPassword("НеправильныйПароль", "InvalidPassword", "НеправильныйПароль"),
        ErrorCategoryNoPermissionToUseFunctionality(
            "ОтсутствиеРазрешенияДляИспользованияФункциональности",
            "NoPermissionToUseFunctionality",
            "ОтсутствиеРазрешенияДляИспользованияФункциональности"
        ),
        ErrorCategoryExternalDataSourceError(
            "ОшибкаВнешнегоИсточникаДанных",
            "ExternalDataSourceError",
            "ОшибкаВнешнегоИсточникаДанных"
        ),
        ErrorCategoryScriptRuntimeError(
            "ОшибкаВоВремяВыполненияВстроенногоЯзыка",
            "ScriptRuntimeError",
            "ОшибкаВоВремяВыполненияВстроенногоЯзыка"
        ),
        ErrorCategoryLocalFileAccessError(
            "ОшибкаДоступаКЛокальномуФайлу",
            "LocalFileAccessError",
            "ОшибкаДоступаКЛокальномуФайлу"
        ),
        ErrorCategoryScriptUseError(
            "ОшибкаИспользованияВстроенногоЯзыка",
            "ScriptUseError",
            "ОшибкаИспользованияВстроенногоЯзыка"
        ),
        ErrorCategoryScriptCompileError(
            "ОшибкаКомпиляцииВстроенногоЯзыка",
            "ScriptCompileError",
            "ОшибкаКомпиляцииВстроенногоЯзыка"
        ),
        ErrorCategoryConfigurationError(
            "ОшибкаКонфигурации",
            "ConfigurationError",
            "ОшибкаКонфигурации"
        ),
        ErrorCategoryDatabaseCopyError(
            "ОшибкаКопииБазыДанных",
            "DatabaseCopyError",
            "ОшибкаКопииБазыДанных"
        ),
        ErrorCategoryDataCompositionSettingsError(
            "ОшибкаНастроекКомпоновкиДанных",
            "DataCompositionSettingsError",
            "ОшибкаНастроекКомпоновкиДанных"
        ),
        ErrorCategoryGotoURLError(
            "ОшибкаПереходаПоНавигационнойСсылке",
            "GotoURLError",
            "ОшибкаПереходаПоНавигационнойСсылке"
        ),
        ErrorCategoryFullTextSearchError(
            "ОшибкаПолнотекстовогоПоиска",
            "FullTextSearchError",
            "ОшибкаПолнотекстовогоПоиска"
        ),
        ErrorCategoryDocumentConversionError(
            "ОшибкаПреобразованияДокумента",
            "DocumentConversionError",
            "ОшибкаПреобразованияДокумента"
        ),
        ErrorCategorySignatureVerificationError(
            "ОшибкаПроверкиПодписи",
            "SignatureVerificationError",
            "ОшибкаПроверкиПодписи"
        ),
        ErrorCategoryPrinterError(
            "ОшибкаРаботыСПринтером",
            "PrinterError",
            "ОшибкаРаботыСПринтером"
        ),
        ErrorCategorySpeechProcessingError(
            "ОшибкаРаботыСРечью",
            "SpeechProcessingError",
            "ОшибкаРаботыСРечью"
        ),
        ErrorCategorySessionError("ОшибкаСеанса", "SessionError", "ОшибкаСеанса"),
        ErrorCategoryNetworkError("ОшибкаСети", "NetworkError", "ОшибкаСети"),
        ErrorCategoryCollaborationSystemError(
            "ОшибкаСистемыВзаимодействия",
            "CollaborationSystemError",
            "ОшибкаСистемыВзаимодействия"
        ),
        ErrorCategoryMultimediaToolsError(
            "ОшибкаСредствМультимедиа",
            "MultimediaToolsError",
            "ОшибкаСредствМультимедиа"
        ),
        ErrorCategoryDatabaseTablespaceError(
            "ОшибкаТабличногоПространстваБазыДанных",
            "DatabaseTablespaceError",
            "ОшибкаТабличногоПространстваБазыДанных"
        ),
        ErrorCategoryStoredDataError(
            "ОшибкаХранимыхДанных",
            "StoredDataError",
            "ОшибкаХранимыхДанных"
        ),
        ErrorCategoryForcedShutdown(
            "ПринудительноеЗавершениеРаботы",
            "ForcedShutdown",
            "ПринудительноеЗавершениеРаботы"
        ),
        ErrorCategoryOtherError("ПрочаяОшибка", "OtherError", "ПрочаяОшибка"),
    }
    XmlNodeType("ТипУзлаXML", "XMLNodeType") meta "ПеречислениеТипУзлаXML" {
        // Состав ВЫЯСНЕН перебором на 8.3.27, а не взят из документации:
        // `НачалоСущности`, `Пробелы` и `ЗначащиеПробелы` платформа не знает,
        // хотя `КонецСущности` у неё есть. Отсюда и несимметричный список.
        // Пятая колонка снова не производная от третьей: платформа печатает
        // «Начало элемента», а не «НачалоЭлемента». Всё измерено.
        XmlNothing("Ничего", "None", "Ничего"),
        XmlElementStart("НачалоЭлемента", "StartElement", "Начало элемента"),
        XmlElementEnd("КонецЭлемента", "EndElement", "Конец элемента"),
        XmlText("Текст", "Text", "Текст"),
        XmlCdataSection("СекцияCDATA", "CDATASection", "Секция CDATA"),
        XmlComment("Комментарий", "Comment", "Комментарий"),
        XmlProcessingInstruction(
            "ИнструкцияОбработки",
            "ProcessingInstruction",
            "Инструкция обработки"
        ),
        XmlDeclaration("ОбъявлениеXML", "XMLDeclaration", "Объявление XML"),
        XmlAttribute("Атрибут", "Attribute", "Атрибут"),
        XmlEntityEnd("КонецСущности", "EndEntity", "Конец сущности"),
        XmlEntityReference("СсылкаНаСущность", "EntityReference", "Ссылка на сущность"),
        XmlDocumentType("ОпределениеТипаДокумента", "DocumentType", "Определение типа документа"),
        XmlNotation("Нотация", "Notation", "Нотация"),
    }
    /// `ТипУзлаDOM`. Английское написание `DOMNodeType` ИЗМЕРЕНО:
    /// `Тип("DOMNodeType") = ТипЗнч(ТипУзлаDOM.Элемент)` — «Да».
    DomNodeType("ТипУзлаDOM", "DOMNodeType") meta "ПеречислениеТипУзлаDOM" {
        // Двенадцать членов, состав ВЫЯСНЕН перебором на 8.3.27: `ТипДокумента`,
        // `Пробелы` и `ЗначащиеПробелы` платформа не знает, зато знает
        // `ОпределениеТипаДокумента`, `ФрагментДокумента`, `Нотация`,
        // `Сущность` и `СсылкаНаСущность`. Заведены все двенадцать, хотя
        // построитель создаёт узлы только шести видов: член перечисления —
        // это то, с чем сравнивают, и «нет такого члена» было бы неправдой.
        // ТипУзлаDOM. Пятая колонка снова измерена целиком: «Секция CDATA»,
        // «Инструкция обработки», «Определение типа документа», «Фрагмент
        // документа» и «Ссылка на сущность» печатаются человеческим текстом с
        // пробелами. Английские написания членов ИЗМЕРЕНЫ перебором
        // (`ТипУзлаDOM.Element` и остальные одиннадцать платформа принимает).
        DomElement("Элемент", "Element", "Элемент"),
        DomAttribute("Атрибут", "Attribute", "Атрибут"),
        DomText("Текст", "Text", "Текст"),
        DomCdataSection("СекцияCDATA", "CDATASection", "Секция CDATA"),
        DomComment("Комментарий", "Comment", "Комментарий"),
        DomProcessingInstruction(
            "ИнструкцияОбработки",
            "ProcessingInstruction",
            "Инструкция обработки"
        ),
        DomDocument("Документ", "Document", "Документ"),
        // Не `DocumentTypeDefinition`: такого члена платформа не знает —
        // измерено обоими написаниями.
        DomDocumentType("ОпределениеТипаДокумента", "DocumentType", "Определение типа документа"),
        DomDocumentFragment("ФрагментДокумента", "DocumentFragment", "Фрагмент документа"),
        DomNotation("Нотация", "Notation", "Нотация"),
        DomEntity("Сущность", "Entity", "Сущность"),
        DomEntityReference("СсылкаНаСущность", "EntityReference", "Ссылка на сущность"),
    }
    SpreadFileType("ТипФайлаТабличногоДокумента", "SpreadsheetDocumentFileType")
        meta "ПеречислениеТипФайлаТабличногоДокумента" {
        // На 8.3.27 измерено восемнадцать членов; `MXLX` среди них нет.
        // Наличие члена не обещает поддержку
        // экспортёром: `bsl-spreadsheet` отклоняет формат без кодировщика.
        SpreadFileMxl("MXL", "MXL", "MXL"),
        SpreadFileXls("XLS", "XLS", "XLS"),
        SpreadFileOds("ODS", "ODS", "ODS"),
        SpreadFileAnsiTxt("ANSITXT", "ANSITXT", "ANSITXT"),
        SpreadFileHtml5("HTML5", "HTML5", "HTML5"),
        SpreadFileDocx("DOCX", "DOCX", "DOCX"),
        SpreadFileTxt("TXT", "TXT", "TXT"),
        SpreadFileXlsx("XLSX", "XLSX", "XLSX"),
        SpreadFilePdf("PDF", "PDF", "PDF"),
        // Дополнительные члены измерены на 8.3.27 в
        // `measure-complete-spreadsheet-file-type.bsl`.
        SpreadFileHtml("HTML", "HTML", "HTML"),
        SpreadFileHtml3("HTML3", "HTML3", "HTML3"),
        SpreadFileHtml4("HTML4", "HTML4", "HTML4"),
        SpreadFileMxl7("MXL7", "MXL7", "MXL7"),
        SpreadFilePdfA1("PDF_A_1", "PDF_A_1", "PDF_A_1"),
        SpreadFilePdfA2("PDF_A_2", "PDF_A_2", "PDF_A_2"),
        SpreadFilePdfA3("PDF_A_3", "PDF_A_3", "PDF_A_3"),
        SpreadFileXls95("XLS95", "XLS95", "XLS95"),
        SpreadFileXls97("XLS97", "XLS97", "XLS97"),
    }
    DrawingKind("ТипРисункаТабличногоДокумента", "SpreadsheetDocumentDrawingType")
        meta "ПеречислениеТипРисункаТабличногоДокумента" {
        /// Единственный поддержанный вид рисунка.
        DrawingGeographicalSchema(
            "ГеографическаяСхема",
            "GeographicalSchema",
            "Geographical schema"
        ),
        DrawingGroup("Группа", "Group", "Folder"),
        DrawingDendrogram("Дендрограмма", "Dendrogram", "Dendrogram"),
        DrawingChart("Диаграмма", "Chart", "Chart"),
        DrawingGanttChart("ДиаграммаГанта", "GanttChart", "Gantt chart"),
        DrawingPicture("Картинка", "Picture", "Picture"),
        DrawingObject("Объект", "Object", "Object"),
        DrawingComment("Примечание", "Comment", "Comment"),
        DrawingLine("Прямая", "Line", "Line"),
        DrawingRectangle("Прямоугольник", "Rectangle", "Rectangle"),
        DrawingPivotChart("СводнаяДиаграмма", "PivotChart", "Pivot chart"),
        DrawingText("Текст", "Text", "Text"),
        DrawingEllipse("Эллипс", "Ellipse", "Ellipse"),
    }
    /// Способ отображения интервала диаграммы Ганта.
    // Представление корня измерено на платформе 8.3.27.2342.
    GanttChartIntervalRepresentation(
        "ОтображениеИнтервалаДиаграммыГанта",
        "GanttChartIntervalRepresentation"
    )
        meta "EnumGanttChartIntervalRepresentation" {
        // Состав и представления измерены в `measure-gantt-interval-representation.bsl`.
        // Платформа печатает `3D` и `Diamond`, а не имена членов.
        GanttChartIntervalRepresentationFlat("Плоский", "Flat", "Flat"),
        GanttChartIntervalRepresentationThreeDimensional("Объемный", "ThreeDimensional", "3D"),
        GanttChartIntervalRepresentationRhomb("Ромб", "Rhomb", "Diamond"),
        GanttChartIntervalRepresentationGradient("Градиент", "Gradient", "Gradient"),
    }
    /// Способ вертикального растягивания диаграммы Ганта.
    // Представление корня измерено на платформе 8.3.27.2342.
    GanttChartVerticalStretch("РастягиваниеПоВертикалиДиаграммыГанта", "GanttChartVerticalStretch")
        meta "EnumGanttChartVerticalStretch" {
        // Представления измерены в `measure-gantt-vertical-stretch.bsl`.
        // `НеРастягивать` печатается как `No`; строка взята из нативного замера
        // (`tests/conformance/measure/measure-gantt-vertical-stretch.platform.txt`).
        GanttChartVerticalStretchNone("НеРастягивать", "None", "No"),
        GanttChartVerticalStretchStretchRows("РастягиватьСтроки", "StretchRows", "Stretch rows"),
        GanttChartVerticalStretchStretchRowsAndData(
            "РастягиватьСтрокиИДанные",
            "StretchRowsAndData",
            "Stretch rows and data"
        ),
    }
    /// Расположение легенды диаграммы.
    // Представление корня измерено на платформе 8.3.27.2342.
    ChartLegendPlacement("РасположениеЛегендыДиаграммы", "ChartLegendPlacement")
        meta "EnumChartLegendPlacement" {
        // Состав и представления измерены в `measure-chart-legend-placement.bsl`.
        // Представление `Use coordinates` измерено, а не построено из имени.
        ChartLegendPlacementAuto("Авто", "Auto", "Auto"),
        ChartLegendPlacementBottom("Низ", "Bottom", "Bottom"),
        ChartLegendPlacementTop("Верх", "Top", "Top"),
        ChartLegendPlacementLeft("Лево", "Left", "Left"),
        ChartLegendPlacementRight("Право", "Right", "Right"),
        ChartLegendPlacementNone("Нет", "None", "None"),
        ChartLegendPlacementUseCoordinates(
            "УказываетсяРасположение",
            "UseCoordinates",
            "Use coordinates"
        ),
    }
    /// Единица шкалы времени.
    // Представление корня измерено на платформе 8.3.27.2342.
    TimeScaleUnitType("ТипЕдиницыШкалыВремени", "TimeScaleUnitType") meta "EnumTimeScaleUnitType" {
        // Состав и представления измерены в `measure-time-scale-unit-type.bsl`.
        // Все восемь представлений измерены на платформе 8.3.27.2342.
        TimeScaleUnitTypeSecond("Секунда", "Second", "Second"),
        TimeScaleUnitTypeMinute("Минута", "Minute", "Minute"),
        TimeScaleUnitTypeHour("Час", "Hour", "Hour"),
        TimeScaleUnitTypeDay("День", "Day", "Day"),
        TimeScaleUnitTypeWeek("Неделя", "Week", "Week"),
        TimeScaleUnitTypeMonth("Месяц", "Month", "Month"),
        TimeScaleUnitTypeQuarter("Квартал", "Quarter", "Quarter"),
        TimeScaleUnitTypeYear("Год", "Year", "Year"),
    }
    /// Формат дня шкалы времени.
    // Представление корня измерено на платформе 8.3.27.2342.
    TimeScaleDayFormat("ФорматДняШкалыВремени", "TimeScaleDayFormat")
        meta "EnumTimeScaleDayFormat" {
        // Представления измерены в `measure-time-scale-day-format.bsl`.
        // Представления, включая порядок частей, взяты из нативного замера
        // (`tests/conformance/measure/measure-time-scale-day-format.platform.txt`).
        TimeScaleDayFormatMonthDay("ДеньМесяца", "MonthDay", "Day of month"),
        TimeScaleDayFormatWeekDay("ДеньНедели", "WeekDay", "Day of week"),
        TimeScaleDayFormatMonthDayWeekDay(
            "ДеньМесяцаДеньНедели",
            "MonthDayWeekDay",
            "Day of month + day of week"
        ),
        TimeScaleDayFormatWeekDayMonthDay(
            "ДеньНеделиДеньМесяца",
            "WeekDayMonthDay",
            "Day of week + day of month"
        ),
    }
    /// `ОриентацияСтраницы` — свойство `ТабличныйДокумент.ОриентацияСтраницы`
    /// и пара с идентификатором 1 в параметрах страницы MXL.
    // ИЗМЕРЕНО, а не достроено по образцу: `Строка(ТипЗнч(
    // ОриентацияСтраницы))` на 8.3.27 даёт «ПеречислениеОриентацияСтраницы».
    PageOrientation("ОриентацияСтраницы", "PageOrientation") meta "ПеречислениеОриентацияСтраницы" {
        // Оба члена ИЗМЕРЕНЫ на 8.3.27: `Строка(ОриентацияСтраницы.Портрет)` —
        // «Портрет», `.Ландшафт` — «Ландшафт», метатип —
        // «ПеречислениеОриентацияСтраницы», а `PageOrientation.Landscape`
        // платформа принимает и печатает тем же русским представлением.
        PageOrientationPortrait("Портрет", "Portrait", "Портрет"),
        PageOrientationLandscape("Ландшафт", "Landscape", "Ландшафт"),
    }
    TextEncoding("КодировкаТекста", "TextEncoding") meta "ПеречислениеКодировкаТекста" {
        // Ровно пять членов, состав проверен перебором.
        // Печатаются идентификатором, без «человеческого» варианта — измерено.
        TextEncodingAnsi("ANSI", "ANSI", "ANSI"),
        TextEncodingOem("OEM", "OEM", "OEM"),
        TextEncodingUtf16("UTF16", "UTF16", "UTF16"),
        TextEncodingUtf8("UTF8", "UTF8", "UTF8"),
        TextEncodingSystem("Системная", "System", "Системная"),
    }
    /// Два режима percent-кодирования строки для query и полного URL.
    StringEncodingMethod("СпособКодированияСтроки", "СпособКодированияСтроки")
        meta "ПеречислениеСпособКодированияСтроки" {
        StringEncodingUrl("КодировкаURL", "КодировкаURL", "String URL-encoding"),
        StringEncodingUrlInUrl("URLВКодировкеURL", "URLВКодировкеURL", "URL URL-encoding"),
    }
    /// Направление сортировки коллекций; Connector использует возрастание.
    SortDirection("НаправлениеСортировки", "SortDirection")
        meta "ПеречислениеНаправлениеСортировки" {
        SortDirectionAscending("Возр", "Asc", "Ascending"),
        SortDirectionDescending("Убыв", "Desc", "Descending"),
    }
    /// Управление выводом поля формы.
    UseOutput("ИспользованиеВывода", "UseOutput") meta "UseOutput" {
        // Представления и псевдонимы измерены на 8.3.27.2342.
        UseOutputAuto("Авто", "Auto", "Auto"),
        UseOutputEnable("Разрешить", "Enable", "Enable"),
        UseOutputDisable("Запретить", "Disable", "Disable"),
    }
    /// Размещение страниц в группе формы.
    FormPagesRepresentation("ОтображениеСтраницФормы", "FormPagesRepresentation")
        meta "FormPagesRepresentation" {
        // Все члены и представления измерены на 8.3.27.2342.
        FormPagesRepresentationAuto("Авто", "Auto", "Auto"),
        FormPagesRepresentationNone("Нет", "None", "None"),
        FormPagesRepresentationSwipe("Пролистывание", "Swipe", "Swipe (touchscreen)"),
        FormPagesRepresentationTabsOnTop("ЗакладкиСверху", "TabsOnTop", "Tabs above"),
        FormPagesRepresentationTabsOnBottom("ЗакладкиСнизу", "TabsOnBottom", "Tabs below"),
        FormPagesRepresentationTabsOnLeftHorizontal(
            "ЗакладкиСлеваГоризонтально",
            "TabsOnLeftHorizontal",
            "Tabs on the left (horizontally)"
        ),
        FormPagesRepresentationTabsOnRightHorizontal(
            "ЗакладкиСправаГоризонтально",
            "TabsOnRightHorizontal",
            "Tabs on the right (horizontally)"
        ),
    }
    /// Расположение картинки на кнопке формы.
    FormButtonPictureLocation("ПоложениеКартинкиКнопкиФормы", "FormButtonPictureLocation")
        meta "FormButtonPictureLocation" {
        // Все члены и представления измерены на 8.3.27.2342.
        FormButtonPictureLocationAuto("Авто", "Auto", "Auto"),
        FormButtonPictureLocationLeft("Лево", "Left", "Left"),
        FormButtonPictureLocationRight("Право", "Right", "Right"),
    }
    /// Тип линии рисунка табличного документа.
    SpreadsheetDocumentDrawingLineType(
        "ТипЛинииРисункаТабличногоДокумента",
        "SpreadsheetDocumentDrawingLineType"
    )
        meta "SpreadsheetDocumentDrawingLineType" {
        // Все члены и представления измерены на 8.3.27.2342.
        SpreadsheetDocumentDrawingLineTypeNone("НетЛинии", "None", "None"),
        SpreadsheetDocumentDrawingLineTypeSolid("Сплошная", "Solid", "Solid"),
        SpreadsheetDocumentDrawingLineTypeDashed("Пунктир", "Dashed", "Dashed"),
        SpreadsheetDocumentDrawingLineTypeDotted("Точечная", "Dotted", "Dotted"),
        SpreadsheetDocumentDrawingLineTypeDashDotted("ПунктирТочка", "DashDotted", "DashDotted"),
        SpreadsheetDocumentDrawingLineTypeDashDottedDotted(
            "ПунктирТочкаТочка",
            "DashDottedDotted",
            "DotDashDotted"
        ),
    }
    /// Представление поля-флажка в форме.
    CheckBoxType("ВидФлажка", "CheckBoxType") meta "CheckBoxType" {
        // Все члены и представления измерены на 8.3.27.2342.
        CheckBoxTypeAuto("Авто", "Auto", "Auto"),
        CheckBoxTypeCheckBox("Флажок", "CheckBox", "Checkbox"),
        CheckBoxTypeSwitch("Выключатель", "Switch", "Switch"),
        CheckBoxTypeTumbler("Тумблер", "Tumbler", "Tumblers"),
    }
    /// Звук доставляемого уведомления.
    // Голое имя и его тип измерены отдельно от типа члена перечисления.
    DeliverableNotificationSubscriberType(
        "ТипПодписчикаДоставляемыхУведомлений",
        "DeliverableNotificationSubscriberType"
    )
        meta "EnumDeliverableNotificationSubscriberType" {
        // Представления и псевдонимы измерены на 8.3.27.2342; `APNS` печатается как `APNs`.
        DeliverableNotificationSubscriberTypeGCM("GCM", "GCM", "GCM"),
        DeliverableNotificationSubscriberTypeAPNS("APNS", "APNS", "APNs"),
        DeliverableNotificationSubscriberTypeFCM("FCM", "FCM", "FCM"),
    }
    SoundAlert("ЗвуковоеОповещение", "SoundAlert") meta "SoundAlert" {
        // Оба члена и представления измерены на 8.3.27.2342.
        SoundAlertNone("Нет", "None", "None"),
        SoundAlertDefault("ПоУмолчанию", "Default", "Default"),
    }
    /// Сквозное выравнивание элементов формы.
    ThroughAlign("СквозноеВыравнивание", "ThroughAlign") meta "ThroughAlign" {
        // Все члены и представления измерены на 8.3.27.2342.
        ThroughAlignAuto("Авто", "Auto", "Auto"),
        ThroughAlignUse("Использовать", "Use", "Use"),
        ThroughAlignDontUse("НеИспользовать", "DontUse", "Do not use"),
    }
    /// Фигура кнопки формы.
    ButtonShape("ФигураКнопки", "ButtonShape") meta "ButtonShape" {
        // Все члены и представления измерены на 8.3.27.2342.
        ButtonShapeAuto("Авто", "Auto", "Auto"),
        ButtonShapeUsual("Обычная", "Usual", "Standard"),
        ButtonShapeOval("Овал", "Oval", "Rounded"),
    }
    /// Тип группы элементов отбора компоновки данных.
    DataCompositionFilterItemsGroupType(
        "ТипГруппыЭлементовОтбораКомпоновкиДанных",
        "DataCompositionFilterItemsGroupType"
    )
        meta "DataCompositionFilterItemsGroupType" {
        // Все члены и представления измерены на 8.3.27.2342.
        DataCompositionFilterItemsGroupTypeAndGroup("ГруппаИ", "AndGroup", "AND group"),
        DataCompositionFilterItemsGroupTypeOrGroup("ГруппаИли", "OrGroup", "OR group"),
        DataCompositionFilterItemsGroupTypeNotGroup("ГруппаНе", "NotGroup", "NOT group"),
    }
    /// Клавиши для сочетаний клавиш.
    Key("Клавиша", "Key") meta "Key" {
        // Достигнутые УНФ члены и представления измерены на 8.3.27.2342.
        KeyDigit1("_1", "_1", "_1"),
        KeyDigit2("_2", "_2", "_2"),
        KeyDigit3("_3", "_3", "_3"),
        KeyDigit4("_4", "_4", "_4"),
        KeyDigit5("_5", "_5", "_5"),
        KeyDigit6("_6", "_6", "_6"),
        KeyDigit7("_7", "_7", "_7"),
        KeyDigit8("_8", "_8", "_8"),
        KeyDigit9("_9", "_9", "_9"),
        KeyNum0("Num0", "Num0", "Num0"),
        KeyNum1("Num1", "Num1", "Num1"),
        KeyNum2("Num2", "Num2", "Num2"),
        KeyNum3("Num3", "Num3", "Num3"),
        KeyNum4("Num4", "Num4", "Num4"),
        KeyNum5("Num5", "Num5", "Num5"),
        KeyNum6("Num6", "Num6", "Num6"),
        KeyNum7("Num7", "Num7", "Num7"),
        KeyNum8("Num8", "Num8", "Num8"),
        KeyNum9("Num9", "Num9", "Num9"),
        KeyNumAdd("NumAdd", "NumAdd", "NumAdd"),
        KeyNumDecimal("NumDecimal", "NumDecimal", "NumDecimal"),
        KeyNumDivide("NumDivide", "NumDivide", "NumDivide"),
        KeyNumMultiply("NumMultiply", "NumMultiply", "NumMultiply"),
        KeyNumSubtract("NumSubtract", "NumSubtract", "NumSubtract"),
        KeyA("A", "A", "A"),
        KeyS("S", "S", "S"),
        KeyV("V", "V", "V"),
    }
    /// Тип рамки элемента управления.
    ControlBorderType("ТипРамкиЭлементаУправления", "ControlBorderType") meta "ControlBorderType" {
        // Достигнутые члены и представления измерены на 8.3.27.2342.
        ControlBorderTypeWithoutBorder("БезРамки", "WithoutBorder", "No border"),
        ControlBorderTypeSingle("Одинарная", "Single", "Single"),
        ControlBorderTypeUnderline("Подчеркивание", "Underline", "Underline"),
    }
    /// Вид цвета объекта `Цвет`.
    ColorType("ВидЦвета", "ColorType") meta "ColorType" {
        // Представления и псевдонимы измерены на 1С 8.3.27.2342.
        ColorTypeAbsolute("Абсолютный", "Absolute", "Absolute"),
        ColorTypeStyleItem("ЭлементСтиля", "StyleItem", "Style element"),
        ColorTypeWebColor("WebЦвет", "WebColor", "Web color"),
        ColorTypeWindowsColor("WindowsЦвет", "WindowsColor", "Windows color"),
        ColorTypeAutoColor("АвтоЦвет", "AutoColor", "Auto color"),
    }
    /// Вид линии границы ячейки табличного документа.
    SpreadsheetDocumentCellLineType(
        "ТипЛинииЯчейкиТабличногоДокумента",
        "SpreadsheetDocumentCellLineType"
    )
        meta "SpreadsheetDocumentCellLineType" {
        // Представления и оба написания измерены на 1С 8.3.27.2342.
        SpreadsheetDocumentCellLineTypeLargeDashed("БольшойПунктир", "LargeDashed", "LargeDashed"),
        SpreadsheetDocumentCellLineTypeDouble("Двойная", "Double", "Double"),
        SpreadsheetDocumentCellLineTypeNone("НетЛинии", "None", "None"),
        SpreadsheetDocumentCellLineTypeThinDashed("РедкийПунктир", "ThinDashed", "ThinDashed"),
        SpreadsheetDocumentCellLineTypeSolid("Сплошная", "Solid", "Solid"),
        SpreadsheetDocumentCellLineTypeDotted("Точечная", "Dotted", "Dotted"),
    }
    /// Тип хранилища сертификатов криптографии.
    CryptoCertificateStoreType("ТипХранилищаСертификатовКриптографии", "CryptoCertificateStoreType")
        meta "CryptoCertificateStoreType" {
        // Представления и оба написания измерены на 1С 8.3.27.2342.
        CryptoCertificateStoreTypeRootCertificates(
            "КорневыеСертификаты",
            "RootCertificates",
            "Root certificates"
        ),
        CryptoCertificateStoreTypePersonalCertificates(
            "ПерсональныеСертификаты",
            "PersonalCertificates",
            "Personal certificates"
        ),
        CryptoCertificateStoreTypeRecipientCertificates(
            "СертификатыПолучателей",
            "RecipientCertificates",
            "Recipient certificates"
        ),
    }
    /// Обработка текстов интернет-почтового сообщения.
    InternetMailTextProcessing(
        "ОбработкаТекстаИнтернетПочтовогоСообщения",
        "InternetMailTextProcessing"
    )
        meta "InternetMailTextProcessing" {
        // Представления и псевдонимы измерены на 1С 8.3.27.2342.
        InternetMailTextProcessingDontProcess("НеОбрабатывать", "DontProcess", "DontProcess"),
        InternetMailTextProcessingProcess("Обрабатывать", "Process", "Process"),
    }
    /// Тип двусторонней печати.
    DuplexPrintingType("ТипДвустороннейПечати", "DuplexPrintingType") meta "DuplexPrintingType" {
        // Достигнутое представление измерено на 1С 8.3.27.2342.
        DuplexPrintingTypeNone("Нет", "None", "None"),
    }
    /// Порядок обработки настроек второго фактора аутентификации.
    SecondAuthenticationFactorSettingsProcessingType(
        "ТипОбработкиНастроекВторогоФактораАутентификации",
        "SecondAuthenticationFactorSettingsProcessingType"
    )
        meta "SecondAuthenticationFactorSettingsProcessingType" {
        // Оба значения и представления измерены на 1С 8.3.27.2342.
        SecondAuthenticationFactorSettingsProcessingDontUse(
            "НеИспользовать",
            "DontUse",
            "НеИспользовать"
        ),
        SecondAuthenticationFactorSettingsProcessingUseNextIfFailed(
            "ИспользоватьСледующуюПриОшибке",
            "UseNextIfFailed",
            "ИспользоватьСледующуюПриОшибке"
        ),
    }
    /// Способ перетаскивания файлов в элемент формы.
    FileDragMode("СпособПеретаскиванияФайлов", "FileDragMode") meta "FileDragMode" {
        // Полный состав и представления измерены на 1С 8.3.27.2342.
        FileDragModeAsFileRef("КакСсылкаНаФайл", "AsFileRef", "As file reference"),
        FileDragModeAsFile("КакФайл", "AsFile", "As file"),
    }
    /// Отображение группы кнопок командной панели.
    ButtonGroupRepresentation("ОтображениеГруппыКнопок", "ButtonGroupRepresentation")
        meta "ButtonGroupRepresentation" {
        // Полный состав и представления измерены на 1С 8.3.27.2342.
        ButtonGroupRepresentationAuto("Авто", "Auto", "Auto"),
        ButtonGroupRepresentationUsual("Обычное", "Usual", "Standard"),
        ButtonGroupRepresentationCompact("Компактное", "Compact", "Compact"),
    }
    /// Тип члена перечисления `ВариантСтандартногоПериода`, достигнутого в УНФ.
    StandardPeriodVariant("ВариантСтандартногоПериода", "StandardPeriodVariant")
        meta "StandardPeriodVariant" {
        // Достигнутые члены измерены на платформе 1С 8.3.27.2342.
        StandardPeriodVariantYesterday("Вчера", "Yesterday", "Yesterday"),
        StandardPeriodVariantTillEndOfThisYear(
            "ДоКонцаЭтогоГода",
            "TillEndOfThisYear",
            "Till the end of this year"
        ),
        StandardPeriodVariantTillEndOfThisQuarter(
            "ДоКонцаЭтогоКвартала",
            "TillEndOfThisQuarter",
            "Till the end of this quarter"
        ),
        StandardPeriodVariantTillEndOfThisMonth(
            "ДоКонцаЭтогоМесяца",
            "TillEndOfThisMonth",
            "Till the end of this month"
        ),
        StandardPeriodVariantTillEndOfThisHalfYear(
            "ДоКонцаЭтогоПолугодия",
            "TillEndOfThisHalfYear",
            "Till the end of this half year"
        ),
        StandardPeriodVariantTillEndOfThisTenDays(
            "ДоКонцаЭтойДекады",
            "TillEndOfThisTenDays",
            "Till the end of this ten-day period"
        ),
        StandardPeriodVariantTillEndOfThisWeek(
            "ДоКонцаЭтойНедели",
            "TillEndOfThisWeek",
            "Till the end of this week"
        ),
        StandardPeriodVariantTomorrow("Завтра", "Tomorrow", "Tomorrow"),
        StandardPeriodVariantMonth("Месяц", "Month", "Month"),
        StandardPeriodVariantLast7Days("Последние7Дней", "Last7Days", "Last 7 days"),
        StandardPeriodVariantCustom("ПроизвольныйПериод", "Custom", "Custom period"),
        StandardPeriodVariantLastTenDays("ПрошлаяДекада", "LastTenDays", "Last ten-day period"),
        StandardPeriodVariantLastTenDaysTillSameDayNumber(
            "ПрошлаяДекадаДоТакогоЖеНомераДня",
            "LastTenDaysTillSameDayNumber",
            "Last ten-day period till the same day number"
        ),
        StandardPeriodVariantLastWeek("ПрошлаяНеделя", "LastWeek", "Last week"),
        StandardPeriodVariantLastWeekTillSameWeekDay(
            "ПрошлаяНеделяДоТакогоЖеДняНедели",
            "LastWeekTillSameWeekDay",
            "Last week till the same day of the week"
        ),
        StandardPeriodVariantLastHalfYear("ПрошлоеПолугодие", "LastHalfYear", "Last half year"),
        StandardPeriodVariantLastHalfYearTillSameDate(
            "ПрошлоеПолугодиеДоТакойЖеДаты",
            "LastHalfYearTillSameDate",
            "Last half year till the same date"
        ),
        StandardPeriodVariantLastYear("ПрошлыйГод", "LastYear", "Last year"),
        StandardPeriodVariantLastYearTillSameDate(
            "ПрошлыйГодДоТакойЖеДаты",
            "LastYearTillSameDate",
            "Last year till the same date"
        ),
        StandardPeriodVariantLastQuarter("ПрошлыйКвартал", "LastQuarter", "Last quarter"),
        StandardPeriodVariantLastQuarterTillSameDate(
            "ПрошлыйКварталДоТакойЖеДаты",
            "LastQuarterTillSameDate",
            "Last quarter till the same date"
        ),
        StandardPeriodVariantLastMonth("ПрошлыйМесяц", "LastMonth", "Last month"),
        StandardPeriodVariantLastMonthTillSameDate(
            "ПрошлыйМесяцДоТакойЖеДаты",
            "LastMonthTillSameDate",
            "Last month till the same date"
        ),
        StandardPeriodVariantFromBeginningOfThisYear(
            "СНачалаЭтогоГода",
            "FromBeginningOfThisYear",
            "From the beginning of this year"
        ),
        StandardPeriodVariantFromBeginningOfThisQuarter(
            "СНачалаЭтогоКвартала",
            "FromBeginningOfThisQuarter",
            "From the beginning of this quarter"
        ),
        StandardPeriodVariantFromBeginningOfThisMonth(
            "СНачалаЭтогоМесяца",
            "FromBeginningOfThisMonth",
            "From the beginning of this month"
        ),
        StandardPeriodVariantFromBeginningOfThisHalfYear(
            "СНачалаЭтогоПолугодия",
            "FromBeginningOfThisHalfYear",
            "From the beginning of this half year"
        ),
        StandardPeriodVariantFromBeginningOfThisTenDays(
            "СНачалаЭтойДекады",
            "FromBeginningOfThisTenDays",
            "From the beginning of this ten-day period"
        ),
        StandardPeriodVariantFromBeginningOfThisWeek(
            "СНачалаЭтойНедели",
            "FromBeginningOfThisWeek",
            "From the beginning of this week"
        ),
        StandardPeriodVariantToday("Сегодня", "Today", "Today"),
        StandardPeriodVariantNextTenDays("СледующаяДекада", "NextTenDays", "Next ten-day period"),
        StandardPeriodVariantNextTenDaysTillSameDayNumber(
            "СледующаяДекадаДоТакогоЖеНомераДня",
            "NextTenDaysTillSameDayNumber",
            "Next ten-day period till the same day number"
        ),
        StandardPeriodVariantNextWeek("СледующаяНеделя", "NextWeek", "Next week"),
        StandardPeriodVariantNextWeekTillSameWeekDay(
            "СледующаяНеделяДоТакогоЖеДняНедели",
            "NextWeekTillSameWeekDay",
            "Next week till the same day of the week"
        ),
        StandardPeriodVariantNextHalfYear("СледующееПолугодие", "NextHalfYear", "Next half year"),
        StandardPeriodVariantNextHalfYearTillSameDate(
            "СледующееПолугодиеДоТакойЖеДаты",
            "NextHalfYearTillSameDate",
            "Next half year till the same date"
        ),
        StandardPeriodVariantNext7Days("Следующие7Дней", "Next7Days", "Next 7 days"),
        StandardPeriodVariantNextYear("СледующийГод", "NextYear", "Next year"),
        StandardPeriodVariantNextYearTillSameDate(
            "СледующийГодДоТакойЖеДаты",
            "NextYearTillSameDate",
            "Next year till the same date"
        ),
        StandardPeriodVariantNextQuarter("СледующийКвартал", "NextQuarter", "Next quarter"),
        StandardPeriodVariantNextQuarterTillSameDate(
            "СледующийКварталДоТакойЖеДаты",
            "NextQuarterTillSameDate",
            "Next quarter till the same date"
        ),
        StandardPeriodVariantNextMonth("СледующийМесяц", "NextMonth", "Next month"),
        StandardPeriodVariantNextMonthTillSameDate(
            "СледующийМесяцДоТакойЖеДаты",
            "NextMonthTillSameDate",
            "Next month till the same date"
        ),
        StandardPeriodVariantThisTenDays("ЭтаДекада", "ThisTenDays", "This ten-day period"),
        StandardPeriodVariantThisWeek("ЭтаНеделя", "ThisWeek", "This week"),
        StandardPeriodVariantThisHalfYear("ЭтоПолугодие", "ThisHalfYear", "This half year"),
        StandardPeriodVariantThisYear("ЭтотГод", "ThisYear", "This year"),
        StandardPeriodVariantThisQuarter("ЭтотКвартал", "ThisQuarter", "This quarter"),
        StandardPeriodVariantThisMonth("ЭтотМесяц", "ThisMonth", "This month"),
    }
    /// Тип члена перечисления `ВариантУправленияВысотойТаблицы`, достигнутого в УНФ.
    TableHeightControlVariant("ВариантУправленияВысотойТаблицы", "TableHeightControlVariant")
        meta "TableHeightControlVariant" {
        TableHeightControlVariantUseHeightInFormRows(
            "ВСтрокахФормы",
            "UseHeightInFormRows",
            "Use form row height"
        ),
    }
    /// Тип члена перечисления `ВерсияПолнотекстовогоПоиска`, достигнутого в УНФ.
    FullTextSearchVersion("ВерсияПолнотекстовогоПоиска", "FullTextSearchVersion")
        meta "FullTextSearchVersion" {
        FullTextSearchVersionVersion1("Версия1", "Version1", "Version1"),
    }
    /// Тип члена перечисления `ВертикальнаяПрокруткаФормы`, достигнутого в УНФ.
    VerticalFormScroll("ВертикальнаяПрокруткаФормы", "VerticalFormScroll")
        meta "VerticalFormScroll" {
        VerticalFormScrollUseIfNecessary(
            "ИспользоватьПриНеобходимости",
            "UseIfNecessary",
            "Use if necessary"
        ),
    }
    /// Тип члена перечисления `ВертикальноеПоложение`, достигнутого в УНФ.
    VerticalAlign("ВертикальноеПоложение", "VerticalAlign") meta "VerticalAlign" {
        VerticalAlignTop("Верх", "Top", "Align top"),
        VerticalAlignBottom("Низ", "Bottom", "Align bottom"),
        VerticalAlignCenter("Центр", "Center", "Align center"),
    }
    /// Тип члена перечисления `ВидДвиженияНакопления`, достигнутого в УНФ.
    AccumulationRecordType("ВидДвиженияНакопления", "AccumulationRecordType")
        meta "AccumulationRecordType" {
        AccumulationRecordTypeReceipt("Приход", "Receipt", "Receipt"),
        AccumulationRecordTypeExpense("Расход", "Expense", "Expense"),
    }
    /// Тип члена перечисления `ВидДвиженияБухгалтерии`, измеренный на 1С.
    // Нативная проба подтверждает тип самого перечисления
    // (`docs/research/fixtures/accounting-record-type/measure.platform.txt`).
    AccountingRecordType("ВидДвиженияБухгалтерии", "AccountingRecordType")
        meta "EnumAccountingRecordType" {
        AccountingRecordTypeDebit("Дебет", "Debit", "Debit"),
        AccountingRecordTypeCredit("Кредит", "Credit", "Credit"),
    }
    /// Положение текста относительно картинки; состав и представления измерены на 1С.
    TextPositionRelativeToPicture(
        "ПоложениеТекстаОтносительноКартинки",
        "TextPositionRelativeToPicture"
    )
        meta "EnumTextPositionRelativeToPicture" {
        TextPositionRelativeToPictureAuto("Авто", "Auto", "Auto"),
        TextPositionRelativeToPictureOnTop("Поверх", "OnTop", "On top"),
        TextPositionRelativeToPictureTop("Сверху", "Top", "Top"),
        TextPositionRelativeToPictureLeft("Слева", "Left", "Left"),
        TextPositionRelativeToPictureBottom("Снизу", "Bottom", "Bottom"),
        TextPositionRelativeToPictureRight("Справа", "Right", "Right"),
    }
    /// Тип члена перечисления `ГоризонтальноеПоложение`, достигнутого в УНФ.
    HorizontalAlign("ГоризонтальноеПоложение", "HorizontalAlign") meta "HorizontalAlign" {
        HorizontalAlignAuto("Авто", "Auto", "Auto"),
        HorizontalAlignLeft("Лево", "Left", "Align left"),
        HorizontalAlignJustify("ПоШирине", "Justify", "Justify"),
        HorizontalAlignRight("Право", "Right", "Align right"),
        HorizontalAlignCenter("Центр", "Center", "Align center"),
    }
    /// Тип члена перечисления `ГруппировкаКолонок`, достигнутого в УНФ.
    ColumnsGroup("ГруппировкаКолонок", "ColumnsGroup") meta "ColumnsGroup" {
        ColumnsGroupInCell("ВЯчейке", "InCell", "In cell"),
        ColumnsGroupVertical("Вертикальная", "Vertical", "Vertical"),
        ColumnsGroupHorizontal("Горизонтальная", "Horizontal", "Horizontal"),
    }
    /// Тип члена перечисления `ДопустимыйНомерСообщения`, достигнутого в УНФ.
    AllowedMessageNo("ДопустимыйНомерСообщения", "AllowedMessageNo") meta "AllowedMessageNo" {
        AllowedMessageNoGreater("Больший", "Greater", "Greater"),
    }
    /// Тип члена перечисления `ИспользованиеГруппИЭлементов`, достигнутого в УНФ.
    FoldersAndItemsUse("ИспользованиеГруппИЭлементов", "FoldersAndItemsUse")
        meta "FoldersAndItemsUse" {
        FoldersAndItemsUseFolders("Группы", "Folders", "Groups"),
        FoldersAndItemsUseFoldersAndItems("ГруппыИЭлементы", "FoldersAndItems", "Groups and items"),
        FoldersAndItemsUseItems("Элементы", "Items", "Items"),
    }
    /// Тип члена перечисления `ИспользованиеПараметраКомпоновкиДанных`, достигнутого в УНФ.
    DataCompositionParameterUse(
        "ИспользованиеПараметраКомпоновкиДанных",
        "DataCompositionParameterUse"
    )
        meta "DataCompositionParameterUse" {
        DataCompositionParameterUseAuto("Авто", "Auto", "Auto"),
        DataCompositionParameterUseAlways("Всегда", "Always", "Always"),
    }
    /// Тип члена перечисления `ИспользованиеПолосыПрокрутки`, достигнутого в УНФ.
    ScrollBarUse("ИспользованиеПолосыПрокрутки", "ScrollBarUse") meta "ScrollBarUse" {
        ScrollBarUseAutoUse("ИспользоватьАвтоматически", "AutoUse", "Use automatically"),
        ScrollBarUseDontUse("НеИспользовать", "DontUse", "None"),
    }
    /// Тип члена перечисления `ИспользованиеТекущейСтроки`, достигнутого в УНФ.
    CurrentRowUse("ИспользованиеТекущейСтроки", "CurrentRowUse") meta "CurrentRowUse" {
        CurrentRowUseAuto("Авто", "Auto", "Auto"),
    }
    /// Тип члена перечисления `НаправлениеПорядкаСхемыЗапроса`, достигнутого в УНФ.
    QuerySchemaOrderDirection("НаправлениеПорядкаСхемыЗапроса", "QuerySchemaOrderDirection")
        meta "QuerySchemaOrderDirection" {
        /// Порядок по возрастанию значений, измеренный на платформе 1С.
        QuerySchemaOrderDirectionAscending("ПоВозрастанию", "Ascending", "Ascending"),
        QuerySchemaOrderDirectionDescending("ПоУбыванию", "Descending", "Descending"),
    }
    /// Тип члена перечисления `НачальноеОтображениеДерева`, достигнутого в УНФ.
    InitialTreeView("НачальноеОтображениеДерева", "InitialTreeView") meta "InitialTreeView" {
        InitialTreeViewNoExpand("НеРаскрывать", "NoExpand", "Do not expand"),
        InitialTreeViewExpandAllLevels(
            "РаскрыватьВсеУровни",
            "ExpandAllLevels",
            "Expand all levels"
        ),
    }
    /// Тип члена перечисления `ОбновлениеПриИзмененииДанных`, достигнутого в УНФ.
    UpdateOnDataChange("ОбновлениеПриИзмененииДанных", "UpdateOnDataChange")
        meta "UpdateOnDataChange" {
        UpdateOnDataChangeAuto("Авто", "Auto", "Auto"),
    }
    /// Тип члена перечисления `ОбновлениеТекстаРедактирования`, достигнутого в УНФ.
    EditTextUpdate("ОбновлениеТекстаРедактирования", "EditTextUpdate") meta "EditTextUpdate" {
        EditTextUpdateDontUse("НеИспользовать", "DontUse", "Do not use"),
        EditTextUpdateOnValueChange("ПриИзмененииЗначения", "OnValueChange", "Upon value change"),
    }
    /// Тип члена перечисления `ОтображениеПредупрежденияПриРедактировании`, достигнутого в УНФ.
    WarningOnEditRepresentation(
        "ОтображениеПредупрежденияПриРедактировании",
        "WarningOnEditRepresentation"
    )
        meta "WarningOnEditRepresentation" {
        WarningOnEditRepresentationDontShow("НеОтображать", "DontShow", "Do not show"),
        WarningOnEditRepresentationShow("Отображать", "Show", "Show"),
    }
    /// Тип члена перечисления `ПоискВТаблицеПриВводе`, достигнутого в УНФ.
    SearchInTableOnInput("ПоискВТаблицеПриВводе", "SearchInTableOnInput")
        meta "SearchInTableOnInput" {
        SearchInTableOnInputDontUse("НеИспользовать", "DontUse", "Do not use"),
    }
    /// Тип члена перечисления `ПоложениеКоманднойПанелиФормы`, достигнутого в УНФ.
    FormCommandBarLabelLocation("ПоложениеКоманднойПанелиФормы", "FormCommandBarLabelLocation")
        meta "FormCommandBarLabelLocation" {
        FormCommandBarLabelLocationAuto("Авто", "Auto", "Auto"),
        FormCommandBarLabelLocationNone("Нет", "None", "None"),
        FormCommandBarLabelLocationBottom("Низ", "Bottom", "Bottom"),
    }
    /// Тип члена перечисления `ПоложениеКоманднойПанелиЭлементаФормы`, достигнутого в УНФ.
    FormItemCommandBarLabelLocation(
        "ПоложениеКоманднойПанелиЭлементаФормы",
        "FormItemCommandBarLabelLocation"
    )
        meta "FormItemCommandBarLabelLocation" {
        FormItemCommandBarLabelLocationAuto("Авто", "Auto", "Auto"),
        FormItemCommandBarLabelLocationTop("Верх", "Top", "Top"),
        FormItemCommandBarLabelLocationNone("Нет", "None", "None"),
    }
    /// Тип члена перечисления `ПоложениеПодписейШкалыЗначенийИзмерительнойДиаграммы`, достигнутого в УНФ.
    GaugeChartValuesScaleLabelsLocation(
        "ПоложениеПодписейШкалыЗначенийИзмерительнойДиаграммы",
        "GaugeChartValuesScaleLabelsLocation"
    )
        meta "GaugeChartValuesScaleLabelsLocation" {
        GaugeChartValuesScaleLabelsLocationAtScale("НаШкале", "AtScale", "On scale"),
    }
    /// Тип положения подписи, достигнутого в модуле менеджера отчёта БУХ.
    ChartLabelLocation("ПоложениеПодписейКДиаграмме", "ChartLabelLocation")
        meta "ChartLabelLocation" {
        /// Автоматическое положение подписи относительно края диаграммы.
        ChartLabelLocationEdgeAuto("КрайАвто", "EdgeAuto", "Edge auto"),
        /// Подпись с края внутри значения диаграммы.
        ChartLabelLocationEdgeInside("КрайВнутри", "EdgeInside", "Edge inside"),
    }
    /// Тип положения подписей шкалы, достигнутого в отчёте БУХ.
    ChartScaleLabelLocation("ПоложениеПодписейШкалыДиаграммы", "ChartScaleLabelLocation")
        meta "ChartScaleLabelLocation" {
        /// Отсутствие подписей шкалы диаграммы.
        ChartScaleLabelLocationNone("Нет", "None", "Do not show"),
    }
    /// Отображение в легенде диаграммы.
    ShowInChartLegend("ОтображениеВЛегендеДиаграммы", "ShowInChartLegend")
        meta "ShowInChartLegend" {
        /// Автоматическое отображение в легенде диаграммы.
        ShowInChartLegendAuto("Авто", "Auto", "Auto"),
        /// Отображение в легенде диаграммы.
        ShowInChartLegendShow("Отображать", "Show", "Show"),
        /// Отсутствие отображения в легенде диаграммы.
        ShowInChartLegendDontShow("НеОтображать", "DontShow", "Do not show"),
    }
    /// Использование расшифровки табличного документа.
    SpreadsheetDocumentDetailUse(
        "ИспользованиеРасшифровкиТабличногоДокумента",
        "SpreadsheetDocumentDetailUse"
    )
        meta "SpreadsheetDocumentDetailUse" {
        /// Расшифровка табличного документа без обработки.
        SpreadsheetDocumentDetailUseWithoutProcessing(
            "БезОбработки",
            "WithoutProcessing",
            "No processing"
        ),
        /// Расшифровка отдельной ячейки табличного документа.
        SpreadsheetDocumentDetailUseCell("Ячейка", "Cell", "Cell"),
    }
    /// Тип члена перечисления `ПоложениеСостоянияПросмотра`, достигнутого в УНФ.
    ViewStatusLocation("ПоложениеСостоянияПросмотра", "ViewStatusLocation")
        meta "ViewStatusLocation" {
        ViewStatusLocationTop("Верх", "Top", "Top"),
    }
    /// Тип члена перечисления `ПоложениеСтрокиПоиска`, достигнутого в УНФ.
    SearchStringLocation("ПоложениеСтрокиПоиска", "SearchStringLocation")
        meta "SearchStringLocation" {
        SearchStringLocationAuto("Авто", "Auto", "Auto"),
        SearchStringLocationNone("Нет", "None", "None"),
    }
    /// Тип члена перечисления `ПоложениеУправленияПоиском`, достигнутого в УНФ.
    SearchControlLocation("ПоложениеУправленияПоиском", "SearchControlLocation")
        meta "SearchControlLocation" {
        SearchControlLocationNone("Нет", "None", "None"),
    }
    /// Тип члена перечисления `РежимАвтоВремя`, достигнутого в УНФ.
    AutoTimeMode("РежимАвтоВремя", "AutoTimeMode") meta "AutoTimeMode" {
        AutoTimeModeLast("Последним", "Last", "Last"),
        AutoTimeModeCurrentOrLast("ТекущееИлиПоследним", "CurrentOrLast", "Current or last"),
    }
    /// Тип члена перечисления `РежимВводаСтрокТаблицы`, достигнутого в УНФ.
    TableRowInputMode("РежимВводаСтрокТаблицы", "TableRowInputMode") meta "TableRowInputMode" {
        TableRowInputModeEndOfList("ВКонецСписка", "EndOfList", "End of list"),
    }
    /// Тип члена перечисления `РежимВключенияСертификатовКриптографии`, достигнутого в УНФ.
    CryptoCertificateIncludeMode(
        "РежимВключенияСертификатовКриптографии",
        "CryptoCertificateIncludeMode"
    )
        meta "CryptoCertificateIncludeMode" {
        CryptoCertificateIncludeModeIncludeWholeChain(
            "ВключатьПолнуюЦепочку",
            "IncludeWholeChain",
            "Include full chain"
        ),
        CryptoCertificateIncludeModeIncludeSubjectCertificate(
            "ВключатьСертификатСубъекта",
            "IncludeSubjectCertificate",
            "Include subject certificate"
        ),
        CryptoCertificateIncludeModeDontInclude("НеВключать", "DontInclude", "Do not include"),
    }
    /// Тип члена перечисления `РежимВыделенияСтрокиТаблицы`, достигнутого в УНФ.
    TableRowSelectionMode("РежимВыделенияСтрокиТаблицы", "TableRowSelectionMode")
        meta "TableRowSelectionMode" {
        TableRowSelectionModeRow("Строка", "Row", "String"),
        TableRowSelectionModeCell("Ячейка", "Cell", "Cell"),
    }
    /// Тип члена перечисления `РежимВыделенияТаблицы`, достигнутого в УНФ.
    TableSelectionMode("РежимВыделенияТаблицы", "TableSelectionMode") meta "TableSelectionMode" {
        TableSelectionModeSingleRow("Одиночный", "SingleRow", "Single"),
    }
    /// Тип члена перечисления `РежимЗамещения`, достигнутого в УНФ.
    ReplacementMode("РежимЗамещения", "ReplacementMode") meta "ReplacementMode" {
        ReplacementModeAppend("Добавление", "Append", "Add"),
        ReplacementModeReplace("Замещение", "Replace", "Replace"),
        ReplacementModeUpdate("Обновление", "Update", "Update"),
        ReplacementModeMerge("Слияние", "Merge", "Merge"),
        ReplacementModeDelete("Удаление", "Delete", "Delete"),
    }
    /// Тип члена перечисления `РежимИзмененияРазмера`, достигнутого в УНФ.
    SizeChangeMode("РежимИзмененияРазмера", "SizeChangeMode") meta "SizeChangeMode" {
        SizeChangeModeQuickChange("БыстроеИзменение", "QuickChange", "Quick change"),
        SizeChangeModeNormal("Обычный", "Normal", "Normal"),
    }
    /// Тип члена перечисления `РежимОткрытияОкнаФормы`, достигнутого в УНФ.
    FormWindowOpeningMode("РежимОткрытияОкнаФормы", "FormWindowOpeningMode")
        meta "FormWindowOpeningMode" {
        FormWindowOpeningModeLockWholeInterface(
            "БлокироватьВесьИнтерфейс",
            "LockWholeInterface",
            "Block the entire interface"
        ),
        FormWindowOpeningModeLockOwnerWindow(
            "БлокироватьОкноВладельца",
            "LockOwnerWindow",
            "Lock parent window"
        ),
        FormWindowOpeningModeIndependent("Независимый", "Independent", "Independent"),
    }
    /// Тип члена перечисления `РежимОтображенияНастроекКомпоновкиДанных`, достигнутого в УНФ.
    DataCompositionSettingsViewMode(
        "РежимОтображенияНастроекКомпоновкиДанных",
        "DataCompositionSettingsViewMode"
    )
        meta "DataCompositionSettingsViewMode" {
        DataCompositionSettingsViewModeQuickAccess("БыстрыйДоступ", "QuickAccess", "Quick access"),
        DataCompositionSettingsViewModeAll("Все", "All", "All"),
    }
    /// Тип члена перечисления `РежимОтображенияРезультатаОтчета`, достигнутого в УНФ.
    ReportResultViewMode("РежимОтображенияРезультатаОтчета", "ReportResultViewMode")
        meta "ReportResultViewMode" {
        ReportResultViewModeAuto("Авто", "Auto", "Auto"),
        ReportResultViewModeCompact("Компактный", "Compact", "Compact"),
        ReportResultViewModeDefault("Обычный", "Default", "Default"),
    }
    /// Тип члена перечисления `РежимПолнотекстовогоПоиска`, достигнутого в УНФ.
    FullTextSearchMode("РежимПолнотекстовогоПоиска", "FullTextSearchMode")
        meta "FullTextSearchMode" {
        FullTextSearchModeDisable("Запретить", "Disable", "Disable"),
        FullTextSearchModeEnable("Разрешить", "Enable", "Enable"),
    }
    /// Тип члена перечисления `РежимПроверкиСертификатаКриптографии`, достигнутого в УНФ.
    CryptoCertificateCheckMode("РежимПроверкиСертификатаКриптографии", "CryptoCertificateCheckMode")
        meta "CryptoCertificateCheckMode" {
        CryptoCertificateCheckModeIgnoreTimeValidity(
            "ИгнорироватьВремяДействия",
            "IgnoreTimeValidity",
            "Ignore action time"
        ),
        CryptoCertificateCheckModeIgnoreSignatureValidity(
            "ИгнорироватьДействительностьПодписи",
            "IgnoreSignatureValidity",
            "Ignore signature validity"
        ),
        CryptoCertificateCheckModeIgnoreCertificateRevocationStatus(
            "ИгнорироватьПроверкуВСпискеОтозванныхСертификатов",
            "IgnoreCertificateRevocationStatus",
            "Ignore check in revoked certificates list"
        ),
        CryptoCertificateCheckModeAllowTestCertificates(
            "РазрешитьТестовыеСертификаты",
            "AllowTestCertificates",
            "Enable test"
        ),
    }
    /// Тип члена перечисления `РежимРедактированияКолонки`, достигнутого в УНФ.
    ColumnEditMode("РежимРедактированияКолонки", "ColumnEditMode") meta "ColumnEditMode" {
        ColumnEditModeEnter("Вход", "Enter", "Enter"),
        ColumnEditModeEnterOnInput("ВходПриВводе", "EnterOnInput", "Enter on input"),
        ColumnEditModeDirectly("Непосредственно", "Directly", "Directly"),
    }
    /// Тип члена перечисления `СпособВосстановленияПароляПользователяИнформационнойБазы`, достигнутого в УНФ.
    InfoBaseUserPasswordRecoveryMethod(
        "СпособВосстановленияПароляПользователяИнформационнойБазы",
        "InfoBaseUserPasswordRecoveryMethod"
    )
        meta "InfoBaseUserPasswordRecoveryMethod" {
        InfoBaseUserPasswordRecoveryMethodSendVerificationCodeBySetParameters(
            "ОтправкаКодаПодтвержденияПоЗаданнымПараметрам",
            "SendVerificationCodeBySetParameters",
            "Send confirmation code with the specified parameters"
        ),
        InfoBaseUserPasswordRecoveryMethodSendVerificationCodeByStandardService(
            "ОтправкаКодаПодтвержденияЧерезСтандартныйСервис",
            "SendVerificationCodeByStandardService",
            "Send confirmation code using standard service"
        ),
    }
    /// Тип члена перечисления `ТипЗаполненияОбластиТабличногоДокумента`, достигнутого в УНФ.
    SpreadsheetDocumentAreaFillType(
        "ТипЗаполненияОбластиТабличногоДокумента",
        "SpreadsheetDocumentAreaFillType"
    )
        meta "SpreadsheetDocumentAreaFillType" {
        SpreadsheetDocumentAreaFillTypeParameter("Параметр", "Parameter", "Parameter"),
        SpreadsheetDocumentAreaFillTypeText("Текст", "Text", "Text"),
        SpreadsheetDocumentAreaFillTypeTemplate("Шаблон", "Template", "Template"),
    }
    /// Тип члена перечисления `ТипОбъединенияСхемыЗапроса`, достигнутого в УНФ.
    QuerySchemaUnionType("ТипОбъединенияСхемыЗапроса", "QuerySchemaUnionType")
        meta "QuerySchemaUnionType" {
        QuerySchemaUnionTypeUnion("Объединить", "Union", "Union"),
        QuerySchemaUnionTypeUnionAll("ОбъединитьВсе", "UnionAll", "Union all"),
    }
    /// Тип члена перечисления `ТипПодписиКриптографии`, достигнутого в УНФ.
    CryptoSignatureType("ТипПодписиКриптографии", "CryptoSignatureType")
        meta "CryptoSignatureType" {
        CryptoSignatureTypeCAdESAv2("CAdESAv2", "CAdESAv2", "CAdES-A v2"),
        CryptoSignatureTypeCAdESAv3("CAdESAv3", "CAdESAv3", "CAdES-A v3"),
        CryptoSignatureTypeCAdESBES("CAdESBES", "CAdESBES", "CAdES-BES"),
        CryptoSignatureTypeCAdESC("CAdESC", "CAdESC", "CAdES-C"),
        CryptoSignatureTypeCAdEST("CAdEST", "CAdEST", "CAdES-T"),
        CryptoSignatureTypeCAdESXLong("CAdESXLong", "CAdESXLong", "CAdES-X Long"),
        CryptoSignatureTypeCAdESXLongType1(
            "CAdESXLongType1",
            "CAdESXLongType1",
            "CAdES-X Long Type 1"
        ),
        CryptoSignatureTypeCAdESXLongType2(
            "CAdESXLongType2",
            "CAdESXLongType2",
            "CAdES-X Long Type 2"
        ),
        CryptoSignatureTypeCAdESXType1("CAdESXType1", "CAdESXType1", "CAdES-X Type 1"),
        CryptoSignatureTypeCAdESXType2("CAdESXType2", "CAdESXType2", "CAdES-X Type 2"),
        CryptoSignatureTypeCMS("CMS", "CMS", "CMS"),
    }
    /// Тип члена перечисления `ТипРазмещенияТекстаТабличногоДокумента`, достигнутого в УНФ.
    SpreadsheetDocumentTextPlacementType(
        "ТипРазмещенияТекстаТабличногоДокумента",
        "SpreadsheetDocumentTextPlacementType"
    )
        meta "SpreadsheetDocumentTextPlacementType" {
        SpreadsheetDocumentTextPlacementTypeCut("Обрезать", "Cut", "Clip"),
        SpreadsheetDocumentTextPlacementTypeWrap("Переносить", "Wrap", "Wrap"),
    }
    /// Тип члена перечисления `ТипСоединенияСхемыЗапроса`, достигнутого в УНФ.
    QuerySchemaJoinType("ТипСоединенияСхемыЗапроса", "QuerySchemaJoinType")
        meta "QuerySchemaJoinType" {
        QuerySchemaJoinTypeInner("Внутреннее", "Inner", "Inner"),
        QuerySchemaJoinTypeLeftOuter("ЛевоеВнешнее", "LeftOuter", "Left outer"),
        QuerySchemaJoinTypeRightOuter("ПравоеВнешнее", "RightOuter", "Right outer"),
    }
    /// Тип члена перечисления `ТипФормыОтчета`, достигнутого в УНФ.
    ReportFormType("ТипФормыОтчета", "ReportFormType") meta "ReportFormType" {
        ReportFormTypeSettings("Настройка", "Settings", "Settings"),
        ReportFormTypeMain("Основная", "Main", "Default"),
    }
    /// Тип члена перечисления `ТипЭлементаФорматированногоДокумента`, достигнутого в УНФ.
    FormattedDocumentItemType("ТипЭлементаФорматированногоДокумента", "FormattedDocumentItemType")
        meta "FormattedDocumentItemType" {
        FormattedDocumentItemTypeText("Текст", "Text", "Text"),
        FormattedDocumentItemTypeLineFeed("ПереводСтроки", "LineFeed", "LineFeed"),
        FormattedDocumentItemTypePicture("Картинка", "Picture", "Picture"),
    }
    /// Тип члена перечисления `ФиксацияВТаблице`, достигнутого в УНФ.
    FixingInTable("ФиксацияВТаблице", "FixingInTable") meta "FixingInTable" {
        FixingInTableLeft("Лево", "Left", "Left"),
        FixingInTableNone("Нет", "None", "No"),
        FixingInTableRight("Право", "Right", "Right"),
    }
    /// Скорость соединения текущего клиента.
    ClientConnectionSpeed("СкоростьКлиентскогоСоединения", "ClientConnectionSpeed")
        meta "EnumClientConnectionSpeed" {
        // Имена и представления измерены на 8.3.27 пробой
        // `measure-client-connection-speed.bsl`.
        ClientConnectionSpeedLow("Низкая", "Low", "Low"),
        ClientConnectionSpeedNormal("Обычная", "Normal", "Normal"),
    }
    /// Тип области ячеек табличного документа.
    SpreadsheetDocumentCellAreaType(
        "ТипОбластиЯчеекТабличногоДокумента",
        "SpreadsheetDocumentCellAreaType"
    )
        meta "EnumSpreadsheetDocumentCellAreaType" {
        // Все четыре имени и представления измерены пробой
        // `measure-spreadsheet-cell-area-type.bsl` на 8.3.27.
        SpreadsheetDocumentCellAreaTypeTable("Таблица", "Table", "Table"),
        SpreadsheetDocumentCellAreaTypeRows("Строки", "Rows", "Rows"),
        SpreadsheetDocumentCellAreaTypeColumns("Колонки", "Columns", "Columns"),
        SpreadsheetDocumentCellAreaTypeRectangle("Прямоугольник", "Rectangle", "Rectangle"),
    }
    /// Тип подключения внешней компоненты.
    // НЕ ИЗМЕРЕНО(ENUM.ADDIN_CONNECTION_TYPE.PRESENTATION):
    // перечисление доступно только в толстом клиенте.
    AddInConnectionType("ТипПодключенияВнешнейКомпоненты", "AddInConnectionType")
        meta "EnumAddInConnectionType" {
        AddInConnectionTypeIsolated("Изолированно", "Isolated", "Isolated"),
        AddInConnectionTypeNotIsolated("НеИзолированно", "NotIsolated", "NotIsolated"),
    }
    /// Проверка заполнения реквизита.
    FillChecking("ПроверкаЗаполнения", "FillChecking") meta "EnumFillChecking" {
        // Представления `Yes` и `No` измерены пробой `measure-fill-checking.bsl`.
        FillCheckingShowError("ВыдаватьОшибку", "ShowError", "Yes"),
        FillCheckingDontCheck("НеПроверять", "DontCheck", "No"),
    }
    /// Выбор групп и элементов иерархических данных.
    FoldersAndItems("ГруппыИЭлементы", "FoldersAndItems") meta "EnumFoldersAndItems" {
        // Имена и представления измерены пробой `measure-folders-and-items.bsl`.
        FoldersAndItemsAuto("Авто", "Auto", "Auto"),
        FoldersAndItemsFolders("Группы", "Folders", "Folders"),
        FoldersAndItemsBoth("ГруппыИЭлементы", "FoldersAndItems", "FoldersAndItems"),
        FoldersAndItemsItems("Элементы", "Items", "Items"),
    }
    /// Протокол интернет-почты.
    /// Устаревшие способы аутентификации, используемые исходниками 1С.
    XmlCanonicalizationType("ТипКаноническогоXML", "XMLCanonicalizationType")
        meta "EnumXMLCanonicalizationType" {
        // Состав и представления измерены пробой `xml-canonicalization-type/measure.bsl`.
        // Все значения, aliases и представления измерены контролем `mail-authentication`.
        // Нативный эталон: `xml-canonicalization-type/measure.platform.txt`.
        XmlCanonicalizationTypeCanonical("КаноническийXML", "XMLCanonicalization", "Canonical XML"),
        XmlCanonicalizationTypeCanonicalWithComments(
            "КаноническийXMLСКомментариями",
            "XMLCanonicalizationWithComments",
            "Canonical XML with comments"
        ),
        XmlCanonicalizationTypeExclusive(
            "ИсключающийКаноническийXML",
            "XMLExclusiveCanonicalization",
            "Exclusive canonical XML"
        ),
        XmlCanonicalizationTypeExclusiveWithComments(
            "ИсключающийКаноническийXMLСКомментариями",
            "XMLExclusiveCanonicalizationWithComments",
            "Exclusive canonical XML with comments"
        ),
        XmlCanonicalizationTypeCanonical11(
            "КаноническийXML1_1",
            "XMLCanonicalization1_1",
            "Canonical XML 1.1"
        ),
        XmlCanonicalizationTypeCanonical11WithComments(
            "КаноническийXML1_1СКомментариями",
            "XMLCanonicalization1_1WithComments",
            "Canonical XML 1.1 with comments"
        ),
    }
    Pop3AuthenticationMode("СпособPOP3Аутентификации", "POP3AuthenticationMode")
        meta "EnumPOP3AuthenticationMode" {
        Pop3AuthenticationModeApop("APOP", "APOP", "APOP"),
        Pop3AuthenticationModeCramMd5("CramMD5", "CramMD5", "CramMD5"),
        Pop3AuthenticationModeGeneral("Обычная", "General", "General"),
    }
    SmtpAuthenticationMode("СпособSMTPАутентификации", "SMTPAuthenticationMode")
        meta "EnumSMTPAuthenticationMode" {
        SmtpAuthenticationModeCramMd5("CramMD5", "CramMD5", "CramMD5"),
        SmtpAuthenticationModeLogin("Login", "Login", "Login"),
        SmtpAuthenticationModePlain("Plain", "Plain", "Plain"),
        SmtpAuthenticationModeNone("БезАутентификации", "None", "None"),
        SmtpAuthenticationModeDefault("ПоУмолчанию", "Default", "Default"),
    }
    InternetMailProtocol("ПротоколИнтернетПочты", "InternetMailProtocol")
        meta "EnumInternetMailProtocol" {
        // Состав и представления измерены пробой `measure-internet-mail-protocol.bsl`.
        InternetMailProtocolImap("IMAP", "IMAP", "IMAP"),
        InternetMailProtocolPop3("POP3", "POP3", "POP3"),
        InternetMailProtocolSmtp("SMTP", "SMTP", "SMTP"),
    }
    /// Действие при обработке расшифровки компоновки данных.
    DataCompositionDetailsProcessingAction(
        "ДействиеОбработкиРасшифровкиКомпоновкиДанных",
        "DataCompositionDetailsProcessingAction"
    )
        meta "EnumDataCompositionDetailsProcessingAction" {
        // Представления, включая пробел после `Open value `, измерены на 1С.
        DataCompositionDetailsProcessingActionNone("Нет", "None", "None"),
        DataCompositionDetailsProcessingActionOpenValue(
            "ОткрытьЗначение",
            "OpenValue",
            "Open value "
        ),
        DataCompositionDetailsProcessingActionFilter("Отфильтровать", "Filter", "Filter"),
        DataCompositionDetailsProcessingActionApplyAppearance(
            "Оформить",
            "ApplyAppearance",
            "Apply appearance"
        ),
        DataCompositionDetailsProcessingActionDrillDown("Расшифровать", "DrillDown", "Drill down"),
        DataCompositionDetailsProcessingActionGroup("Сгруппировать", "Group", "Group"),
        DataCompositionDetailsProcessingActionOrder("Упорядочить", "Order", "Order"),
    }
    /// Какие части даты сохраняет `КвалификаторыДаты`.
    DateFractions("ЧастиДаты", "DateFractions") meta "ПеречислениеЧастиДаты" {
        DateFractionsTime("Время", "Time", "Time"),
        DateFractionsDate("Дата", "Date", "Date"),
        DateFractionsDateTime("ДатаВремя", "DateTime", "Date and time"),
    }
    /// Ограничение длины для `КвалификаторыСтроки`.
    AllowedLength("ДопустимаяДлина", "AllowedLength") meta "AllowedLength" {
        AllowedLengthVariable("Переменная", "Variable", "Variable-length"),
        AllowedLengthFixed("Фиксированная", "Fixed", "Fixed-length"),
    }
    /// Алгоритм объекта `ХешированиеДанных`.
    HashFunction("ХешФункция", "HashFunction") meta "ПеречислениеХешФункция" {
        HashCrc32("CRC32", "CRC32", "CRC32 hash function"),
        HashMd5("MD5", "MD5", "MD5 hash function"),
        HashPbkdf2Sha256("PBKDF2SHA256", "PBKDF2SHA256", "PBKDF2 SHA-256 hash function"),
        HashSha1("SHA1", "SHA1", "SHA-1 hash function"),
        HashSha256("SHA256", "SHA256", "SHA-256 hash function"),
        HashSha512("SHA512", "SHA512", "SHA-512 hash function"),
    }
    /// `ПорядокБайтов` — порядок байтов многобайтового целого в
    /// `БуферДвоичныхДанных`. Английское написание `ByteOrder` ИЗМЕРЕНО
    /// (`ByteOrder.LittleEndian` платформа принимает и считает равным
    /// русскому написанию), метатип — «ПеречислениеПорядокБайтов», тоже
    /// измерено.
    // Здесь префикс не предположение по образцу, а ИЗМЕРЕНО:
    // `Строка(ПорядокБайтов)` и `Строка(ТипЗнч(ПорядокБайтов))`
    // оба дают «ПеречислениеПорядокБайтов».
    ByteOrder("ПорядокБайтов", "ByteOrder") meta "ПеречислениеПорядокБайтов" {
        // Ровно два члена, и оба пишутся ОДИНАКОВО на обоих языках: русских
        // `Прямой`/`Обратный` платформа не знает (проверено перебором).
        // Третья и четвёртая колонки СОВПАДАЮТ: у этого перечисления русских
        // написаний членов нет вовсе, а печатается член через дефис и со
        // строчной второй частью — «Little-endian», не «LittleEndian». Всё
        // измерено на 8.3.27.
        ByteOrderLittle("LittleEndian", "LittleEndian", "Little-endian"),
        ByteOrderBig("BigEndian", "BigEndian", "Big-endian"),
    }
    /// `РежимОткрытияФайла` — второй аргумент `Новый ФайловыйПоток`.
    /// Английское написание `FileOpenMode` и написания всех шести членов
    /// ИЗМЕРЕНЫ (`FileOpenMode.Open = РежимОткрытияФайла.Открыть` —
    /// «Да»), метатип — «ПеречислениеРежимОткрытияФайла», тоже измерено.
    // Тоже ИЗМЕРЕНО, а не достроено по образцу: фикстура
    // `binary-streams` печатает и голое имя, и его `ТипЗнч()` для
    // всех трёх перечислений потоков.
    FileOpenMode("РежимОткрытияФайла", "FileOpenMode") meta "ПеречислениеРежимОткрытияФайла" {
        // Шесть членов; состав и написания измерены полной таблицей «режим x
        // доступ» (см. заголовок `crate::stream`).
        // Пятая колонка у потоковых перечислений снова не производная от
        // третьей: составные члены платформа печатает раздельно и со строчной
        // второй частью — «Создать новый», «Открыть или создать», «Чтение и
        // запись». Всё измерено фикстурой `binary-streams`.
        FileOpenModeOpen("Открыть", "Open", "Открыть"),
        FileOpenModeOpenOrCreate("ОткрытьИлиСоздать", "OpenOrCreate", "Открыть или создать"),
        FileOpenModeCreate("Создать", "Create", "Создать"),
        FileOpenModeCreateNew("СоздатьНовый", "CreateNew", "Создать новый"),
        FileOpenModeTruncate("Обрезать", "Truncate", "Обрезать"),
        FileOpenModeAppend("Дописать", "Append", "Дописать"),
    }
    /// `ДоступКФайлу` — третий аргумент `Новый ФайловыйПоток`. Написание
    /// члена `ЧтениеИЗапись` по-английски — `ReadAndWrite`, а не
    /// `ReadWrite`: измерено.
    FileAccess("ДоступКФайлу", "FileAccess") meta "ПеречислениеДоступКФайлу" {
        FileAccessRead("Чтение", "Read", "Чтение"),
        FileAccessWrite("Запись", "Write", "Запись"),
        FileAccessReadAndWrite("ЧтениеИЗапись", "ReadAndWrite", "Чтение и запись"),
    }
    /// `ПозицияВПотоке` — точка отсчёта у `Перейти`. Английское написание
    /// самого перечисления — `PositionInStream`, а не `StreamPosition`:
    /// измерено.
    StreamPosition("ПозицияВПотоке", "PositionInStream") meta "ПеречислениеПозицияВПотоке" {
        StreamPositionBegin("Начало", "Begin", "Начало"),
        StreamPositionCurrent("Текущая", "Current", "Текущая"),
        StreamPositionEnd("Конец", "End", "Конец"),
    }
    // --- перечисления объектной модели XML-схемы ------------------------
    // Английские написания САМИХ перечислений измерены через `Тип(...)`:
    // `XSComponentType`, `XSForm`, `XSSimpleTypeVariety`,
    // `XSDerivationMethod`, `XSWhitespaceHandling` — есть, а вот у
    // `ВидГруппыМоделиXS` и `ОграничениеЗначенияXS` английского имени НЕТ
    // (`XSModelGroupType` и `XSValueConstraint` платформа не знает), и
    // тогда вторым написанием стоит то же русское.
    /// `ТипКомпонентыXS` — вид компоненты схемы; его отдаёт свойство
    /// `ТипКомпоненты` у каждой компоненты.
    // Перечисления модели схемы — тот же префикс по образцу, под
    // тем же маркером `JSON.ENUM.BARE_NAME`: голое имя ни у одного
    // из них не мерилось.
    XsComponentType("ТипКомпонентыXS", "XSComponentType") meta "ПеречислениеТипКомпонентыXS" {
        // Заведены ВСЕ члены, которые платформа признала (перебором имён из
        // словаря `xml2.so`), а не только те, что отдаёт наша модель: член,
        // который есть у платформы, обязан сравниваться и здесь.
        // Английские написания ЧЛЕНОВ ИЗМЕРЕНЫ перебором всех сорока семи:
        // платформа принимает `ФормаПредставленияXS.Qualified`,
        // `ТипКомпонентыXS.MinInclusiveFacet` и так далее. Пятая колонка — то,
        // что печатает `Строка()`: у `ТипКомпонентыXS` это человеческий текст с
        // пробелами, у остальных совпадает с русским идентификатором, кроме
        // `ПоУмолчанию` -> «По умолчанию».
        XsCompSchema("Схема", "Schema", "Схема"),
        XsCompElementDeclaration("ОбъявлениеЭлемента", "ElementDeclaration", "Объявление элемента"),
        XsCompAttributeDeclaration(
            "ОбъявлениеАтрибута",
            "AttributeDeclaration",
            "Объявление атрибута"
        ),
        XsCompSimpleTypeDefinition(
            "ОпределениеПростогоТипа",
            "SimpleTypeDefinition",
            "Определение простого типа"
        ),
        XsCompComplexTypeDefinition(
            "ОпределениеСоставногоТипа",
            "ComplexTypeDefinition",
            "Определение составного типа"
        ),
        XsCompParticle("Фрагмент", "Particle", "Фрагмент"),
        XsCompModelGroup("ГруппаМодели", "ModelGroup", "Группа модели"),
        XsCompAttributeUse("ИспользованиеАтрибута", "AttributeUse", "Использование атрибута"),
        XsCompModelGroupDefinition(
            "ОпределениеГруппыМодели",
            "ModelGroupDefinition",
            "Определение группы модели"
        ),
        XsCompAttributeGroupDefinition(
            "ОпределениеГруппыАтрибутов",
            "AttributeGroupDefinition",
            "Определение группы атрибутов"
        ),
        XsCompNotationDeclaration("ОбъявлениеНотации", "NotationDeclaration", "Объявление нотации"),
        XsCompAnnotation("Аннотация", "Annotation", "Аннотация"),
        XsCompDocumentation("Документация", "Documentation", "Документация"),
        // Имя члена — `ИнформацияПриложения`, хотя тип называется
        // `ИнформацияДляПриложенияXS`: `ТипКомпонентыXS.ИнформацияДляПриложения`
        // платформа отвергает (измерено).
        XsCompAppInfo("ИнформацияПриложения", "AppInfo", "Информация приложения"),
        XsCompWildcard("Маска", "Wildcard", "Маска"),
        XsCompImport("Импорт", "Import", "Импорт"),
        XsCompInclude("Включение", "Include", "Включение"),
        XsCompRedefine("Переопределение", "Redefine", "Переопределение"),
        XsCompLengthFacet("ФасетДлины", "LengthFacet", "Фасет длины"),
        XsCompMinLengthFacet("ФасетМинимальнойДлины", "MinLengthFacet", "Фасет минимальной длины"),
        XsCompMaxLengthFacet(
            "ФасетМаксимальнойДлины",
            "MaxLengthFacet",
            "Фасет максимальной длины"
        ),
        XsCompPatternFacet("ФасетОбразца", "PatternFacet", "Фасет образца"),
        XsCompEnumerationFacet("ФасетПеречисления", "EnumerationFacet", "Фасет перечисления"),
        XsCompWhitespaceFacet(
            "ФасетПробельныхСимволов",
            "WhitespaceFacet",
            "Фасет пробельных символов"
        ),
        XsCompTotalDigitsFacet(
            "ФасетОбщегоКоличестваРазрядов",
            "TotalDigitsFacet",
            "Фасет общего количества разрядов"
        ),
        XsCompFractionDigitsFacet(
            "ФасетКоличестваРазрядовДробнойЧасти",
            "FractionDigitsFacet",
            "Фасет количества разрядов дробной части"
        ),
        // Четыре граничных фасета названы у платформы ИНАЧЕ, чем остальные:
        // прилагательное впереди («МинимальноВключающийФасет»), а не «Фасет…».
        XsCompMinInclusiveFacet(
            "МинимальноВключающийФасет",
            "MinInclusiveFacet",
            "Минимально включающий фасет"
        ),
        XsCompMaxInclusiveFacet(
            "МаксимальноВключающийФасет",
            "MaxInclusiveFacet",
            "Максимально включающий фасет"
        ),
        XsCompMinExclusiveFacet(
            "МинимальноИсключающийФасет",
            "MinExclusiveFacet",
            "Минимально исключающий фасет"
        ),
        XsCompMaxExclusiveFacet(
            "МаксимальноИсключающийФасет",
            "MaxExclusiveFacet",
            "Максимально исключающий фасет"
        ),
        XsCompIdentityConstraintDefinition(
            "ОпределениеОграниченияИдентичности",
            "IdentityConstraintDefinition",
            "Определение ограничения идентичности"
        ),
        XsCompXPathDefinition("ОпределениеXPath", "XPathDefinition", "Определение XPath"),
    }
    /// `ФормаПредставленияXS` — `elementFormDefault`/`form`.
    XsForm("ФормаПредставленияXS", "XSForm") meta "ПеречислениеФормаПредставленияXS" {
        XsFormQualified("Квалифицированная", "Qualified", "Квалифицированная"),
        XsFormUnqualified("Неквалифицированная", "Unqualified", "Неквалифицированная"),
    }
    /// `ВариантПростогоТипаXS` — `Атомарная`/`Список`/`Объединение`.
    XsSimpleTypeVariety("ВариантПростогоТипаXS", "XSSimpleTypeVariety")
        meta "ПеречислениеВариантПростогоТипаXS" {
        XsVarietyAtomic("Атомарная", "Atomic", "Атомарная"),
        XsVarietyList("Список", "List", "Список"),
        XsVarietyUnion("Объединение", "Union", "Объединение"),
    }
    /// `ВидГруппыМоделиXS` — `Последовательность`/`Выбор`/`Все`.
    XsModelGroupKind("ВидГруппыМоделиXS", "ВидГруппыМоделиXS")
        meta "ПеречислениеВидГруппыМоделиXS" {
        XsGroupSequence("Последовательность", "Sequence", "Последовательность"),
        XsGroupChoice("Выбор", "Choice", "Выбор"),
        XsGroupAll("Все", "All", "Все"),
    }
    /// `МетодНаследованияXS` — `Расширение`/`Ограничение`.
    XsDerivationMethod("МетодНаследованияXS", "XSDerivationMethod")
        meta "ПеречислениеМетодНаследованияXS" {
        XsDerivationExtension("Расширение", "Extension", "Расширение"),
        XsDerivationRestriction("Ограничение", "Restriction", "Ограничение"),
    }
    /// `ОграничениеЗначенияXS` — вид значения по умолчанию у объявления.
    XsValueConstraint("ОграничениеЗначенияXS", "ОграничениеЗначенияXS")
        meta "ПеречислениеОграничениеЗначенияXS" {
        // Печатается с ПРОБЕЛОМ — «По умолчанию», хотя пишется слитно.
        XsConstraintDefault("ПоУмолчанию", "Default", "По умолчанию"),
        XsConstraintFixed("Фиксированное", "Fixed", "Фиксированное"),
    }
    /// `ОбработкаПробельныхСимволовXS` — значение фасета `whiteSpace`.
    XsWhitespaceHandling("ОбработкаПробельныхСимволовXS", "XSWhitespaceHandling")
        meta "ПеречислениеОбработкаПробельныхСимволовXS" {
        XsWhitespacePreserve("Сохранять", "Preserve", "Сохранять"),
        XsWhitespaceReplace("Заменять", "Replace", "Заменять"),
        XsWhitespaceCollapse("Сворачивать", "Collapse", "Сворачивать"),
    }
    /// `ФормаXML` — `Свойство.Форма` модели типов XDTO: `Элемент`,
    /// `Атрибут`, `Текст`. Английские написания есть и у самого
    /// перечисления (`XMLForm`), и у всех трёх членов (измерено).
    XmlForm("ФормаXML", "XMLForm") meta "ПеречислениеФормаXML" {
        XmlFormElement("Элемент", "Element", "Элемент"),
        XmlFormAttribute("Атрибут", "Attribute", "Атрибут"),
        XmlFormText("Текст", "Text", "Текст"),
    }
    /// `ВидФасетаXDTO` — `Фасет.Вид`. Русские написания членов измерены
    /// поимённо и НЕ выводятся из представлений: у члена с
    /// представлением «Минимальная длина» имя `МинДлина`, а
    /// `МинимальнаяДлина` платформа отвергает.
    XdtoFacetKind("ВидФасетаXDTO", "XDTOFacetType") meta "ПеречислениеВидФасетаXDTO" {
        // `ВидФасетаXDTO`: русские написания членов НЕ выводятся из
        // представлений — они короче и местами неожиданны (`МинДлина` при
        // представлении «Минимальная длина», `РазрядовВсего` при «Количество
        // разрядов»). Каждое проверено отдельной пробой, как и то, что
        // «длинные» написания (`МинимальнаяДлина`, `ОбщееКоличествоРазрядов`,
        // `МинимальноеВключающееЗначение`) платформа отвергает.
        XdtoFacetLength("Длина", "Length", "Длина"),
        XdtoFacetMinLength("МинДлина", "MinLength", "Минимальная длина"),
        XdtoFacetMaxLength("МаксДлина", "MaxLength", "Максимальная длина"),
        XdtoFacetPattern("Образец", "Pattern", "Образец"),
        XdtoFacetEnumeration("Перечисление", "Enumeration", "Перечисление"),
        XdtoFacetWhiteSpace("ПробельныеСимволы", "WhiteSpace", "Пробельные символы"),
        XdtoFacetTotalDigits("РазрядовВсего", "TotalDigits", "Количество разрядов"),
        XdtoFacetFractionDigits(
            "РазрядовДробнойЧасти",
            "FractionDigits",
            "Количество разрядов дробной части"
        ),
        XdtoFacetMinInclusive("МинВключающее", "MinInclusive", "Минимальное включающее значение"),
        XdtoFacetMaxInclusive("МаксВключающее", "MaxInclusive", "Максимальное включающее значение"),
        XdtoFacetMinExclusive("МинИсключающее", "MinExclusive", "Минимальное исключающее значение"),
        XdtoFacetMaxExclusive(
            "МаксИсключающее",
            "MaxExclusive",
            "Максимальное исключающее значение"
        ),
    }
    /// `ТипРезультатаDOMXPath` — вид результата `ВычислитьВыражениеXPath`;
    /// его же принимает необязательный четвёртый аргумент. Английское
    /// написание `DOMXPathResultType` ИЗМЕРЕНО (`XPathResultType` и
    /// `ТипРезультатаXPathDOM` платформа не знает).
    // ИЗМЕРЕНО, а не достроено по образцу: `Строка(ТипРезультатаDOMXPath)`
    // и `Строка(ТипЗнч(ТипРезультатаDOMXPath))` оба дают
    // «ПеречислениеТипРезультатаDOMXPath».
    DomXPathResultType("ТипРезультатаDOMXPath", "DOMXPathResultType")
        meta "ПеречислениеТипРезультатаDOMXPath" {
        // Все десять членов и оба их написания ИЗМЕРЕНЫ перебором на 8.3.27:
        // `МножествоУзлов`, `НаборУзлов`, `Узел`, `Итератор`, `Снимок` и
        // `NodeSet` платформа НЕ знает, а вот `Любой`/`Any`,
        // `НеупорядоченныйИтераторУзлов`/`UnorderedNodeIterator` и остальные
        // восемь — знает.
        // ТипРезультатаDOMXPath. Пятая колонка измерена целиком: шесть узловых
        // членов печатаются человеческим текстом («Неупорядоченный итератор
        // узлов»), а четыре простых — как пишутся. Английские написания
        // измерены перебором: `ТипРезультатаDOMXPath.Any` и остальные девять
        // платформа принимает и считает равными русским.
        XPathAny("Любой", "Any", "Любой"),
        XPathNumber("Число", "Number", "Число"),
        XPathString("Строка", "String", "Строка"),
        XPathBoolean("Булево", "Boolean", "Булево"),
        XPathUnorderedNodeIterator(
            "НеупорядоченныйИтераторУзлов",
            "UnorderedNodeIterator",
            "Неупорядоченный итератор узлов"
        ),
        XPathOrderedNodeIterator(
            "УпорядоченныйИтераторУзлов",
            "OrderedNodeIterator",
            "Упорядоченный итератор узлов"
        ),
        XPathUnorderedNodeSnapshot(
            "НеупорядоченныйСнимокУзлов",
            "UnorderedNodeSnapshot",
            "Неупорядоченный снимок узлов"
        ),
        XPathOrderedNodeSnapshot(
            "УпорядоченныйСнимокУзлов",
            "OrderedNodeSnapshot",
            "Упорядоченный снимок узлов"
        ),
        XPathAnyUnorderedNode(
            "ЛюбойНеупорядоченныйУзел",
            "AnyUnorderedNode",
            "Любой неупорядоченный узел"
        ),
        XPathFirstOrderedNode(
            "ПервыйУпорядоченныйУзел",
            "FirstOrderedNode",
            "Первый упорядоченный узел"
        ),
    }
    /// `НаправлениеПоиска` — третий аргумент
    /// `СтрНайтиПоРегулярномуВыражению`. Английское написание
    /// `SearchDirection` ИЗМЕРЕНО, как и то, что `Тип("НаправлениеПоиска")`
    /// и `Тип("SearchDirection")` дают один и тот же тип.
    // Тоже ИЗМЕРЕНО: `Строка(ТипЗнч(НаправлениеПоиска))` даёт
    // «ПеречислениеНаправлениеПоиска».
    SearchDirection("НаправлениеПоиска", "SearchDirection") meta "ПеречислениеНаправлениеПоиска" {
        // Оба члена и оба их написания ИЗМЕРЕНЫ на 8.3.27:
        // `НаправлениеПоиска.СНачала` печатается «С начала», `.СКонца` — «С
        // конца», а `SearchDirection.FromBegin`/`.FromEnd` платформа принимает
        // и считает теми же значениями. Третьего члена нет: `.СЛева` — «Поле
        // объекта не обнаружено».
        // НаправлениеПоиска. Пятая колонка измерена: имя пишется слитно
        // (`СНачала`), а печатается с пробелом («С начала»).
        SearchFromBegin("СНачала", "FromBegin", "С начала"),
        SearchFromEnd("СКонца", "FromEnd", "С конца"),
    }
    // --- перечисления чтения архивов -------------------------------------
    /// `РежимВосстановленияПутейФайловZIP` — необязательный аргумент
    /// `Извлечь` и `ИзвлечьВсе`. Английское написание
    /// `ZIPRestoreFilePathsMode` ИЗМЕРЕНО (`ZIPRestorePathsMode` платформа
    /// не знает), как и то, что членов ровно два.
    // Тот же префикс по образцу и под тем же маркером
    // `JSON.ENUM.BARE_NAME`: голое имя ни у одного из двух не
    // мерилось.
    ZipRestorePathsMode("РежимВосстановленияПутейФайловZIP", "ZIPRestoreFilePathsMode")
        meta "ПеречислениеРежимВосстановленияПутейФайловZIP" {
        // Членов ровно два: `.НетТакого` — «Поле объекта не обнаружено»
        // (измерено).
        // Представления обоих режимов ИЗМЕРЕНЫ и, как обычно, не выводятся из
        // имён: у `НеВосстанавливать` печатается «Не восстанавливать пути».
        RestorePaths("Восстанавливать", "Restore", "Восстанавливать пути"),
        DontRestorePaths("НеВосстанавливать", "DontRestore", "Не восстанавливать пути"),
    }
    /// Отдельное перечисление восстановления путей архива, начиная с 8.3.26.
    // Английское представление измерено и при русском интерфейсе.
    ArchiveFileRestorePathsMode(
        "РежимВосстановленияПутейФайлаАрхива",
        "ArchiveFileRestoreFilePathsMode"
    )
        meta "EnumArchiveFileRestoreFilePathsMode" {
        // Новый тип архива не равен ZIP-перечислению; измерено на 8.3.27.
        ArchiveRestorePaths("Восстанавливать", "Restore", "Восстанавливать пути"),
        ArchiveDontRestorePaths("НеВосстанавливать", "DontRestore", "Не восстанавливать пути"),
    }
    /// `ТипСвязиВложенияPDF` — четвёртый аргумент
    /// `КоллекцияВложенийPDF.Добавить` и свойство `ВложениеPDF.ТипСвязи`.
    /// Английского написания у самого перечисления НЕТ:
    /// `PDFAttachmentRelationship`, `СвязьВложенияPDF` и
    /// `ОтношениеВложенияPDF` платформа не знает (проверено перебором), а
    /// у членов оно есть.
    PdfAttachmentRelation("ТипСвязиВложенияPDF", "ТипСвязиВложенияPDF")
        meta "ПеречислениеТипСвязиВложенияPDF" {
        // Пять членов, снятых перебором. Спецификация PDF (таблица 43) знает
        // ещё `Schema`, `FormData` и `EncryptedPayload` — платформа их НЕ
        // знает, поэтому их нет и здесь: член перечисления, которого у неё
        // нет, был бы неправдой.
        // Члены `ТипСвязиВложенияPDF`. Английские написания есть у всех пяти,
        // и представление снова измерено: у `НеУстановлено` печатается «Не
        // указано» — ни «НеУстановлено», ни «Unspecified».
        PdfRelationSource("Источник", "Source", "Источник"),
        PdfRelationData("Данные", "Data", "Данные"),
        PdfRelationAlternative("Альтернатива", "Alternative", "Альтернатива"),
        PdfRelationSupplement("Дополнение", "Supplement", "Дополнение"),
        PdfRelationUnspecified("НеУстановлено", "Unspecified", "Не указано"),
    }
    /// `ТипФайлаАрхива` — третий аргумент `Новый ЧтениеФайлаАрхива`.
    /// Английское написание `ArchiveFileType` ИЗМЕРЕНО. Членов семь, и
    /// читаем мы ровно один из них: остальные шесть заведены для того,
    /// чтобы `ЧтениеФайлаАрхива(файл, , ТипФайлаАрхива.TAR)` отвечал
    /// «формат не поддерживается», а не разбирал TAR как ZIP.
    ArchiveFileType("ТипФайлаАрхива", "ArchiveFileType") meta "ПеречислениеТипФайлаАрхива" {
        // Семь членов, снятых перебором; из них поддержан только `Zip`.
        // У членов `ТипФайлаАрхива` написание латинское и на обоих языках
        // одинаковое; представление снова измерено — у `SevenZip` печатается
        // «Тип архива 7Z», а не «7Zip». `Deflate64` и `SevenZ` платформа не
        // знает — проверено перебором.
        ArchiveTypeZip("Zip", "Zip", "Тип архива ZIP"),
        ArchiveTypeBzip2("BZIP2", "BZIP2", "Тип архива BZIP2"),
        ArchiveTypeGzip("GZIP", "GZIP", "Тип архива GZIP"),
        ArchiveTypeRar("RAR", "RAR", "Тип архива RAR"),
        ArchiveTypeSevenZip("SevenZip", "SevenZip", "Тип архива 7Z"),
        ArchiveTypeTar("TAR", "TAR", "Тип архива TAR"),
        ArchiveTypeXz("XZ", "XZ", "Тип архива XZ"),
    }
    // --- перечисления записи архивов ---------------------------------------
    /// `МетодСжатияZIP` — четвёртый аргумент `Новый ЗаписьZipФайла`.
    /// Английское написание `ZIPCompressionMethod` ИЗМЕРЕНО.
    ZipCompressionMethod("МетодСжатияZIP", "ZIPCompressionMethod")
        meta "ПеречислениеМетодСжатияZIP" {
        // Три члена; `Deflate64`, `LZMA` и `PPMd` — «Поле объекта не
        // обнаружено» (проверено перебором).
        // Перечисления записи. Пятая колонка везде измерена и нигде не
        // выводится из имени: у `Сжатие` печатается «Метод сжатия сжатием», у
        // `BZIP2` — «Метод сжатия BZIP» (без двойки), у `ОбрабатыватьРекурсивно`
        // — просто «Обрабатывать».
        ZipMethodDeflate("Сжатие", "Deflate", "Метод сжатия сжатием"),
        ZipMethodCopy("Копирование", "Copy", "Метод сжатия копированием"),
        ZipMethodBzip2("BZIP2", "BZip2", "Метод сжатия BZIP"),
    }
    /// `УровеньСжатияZIP` — пятый аргумент того же конструктора.
    ZipCompressionLevel("УровеньСжатияZIP", "ZIPCompressionLevel")
        meta "ПеречислениеУровеньСжатияZIP" {
        // Ровно три: ни `БезСжатия`, ни `Нет`, ни `Никакой`, ни `Отсутствует`,
        // ни `Быстрый` платформа не знает.
        ZipLevelMinimal("Минимальный", "Minimum", "Минимальный уровень сжатия"),
        ZipLevelOptimal("Оптимальный", "Optimal", "Оптимальный уровень сжатия"),
        ZipLevelMaximal("Максимальный", "Maximum", "Максимальный уровень сжатия"),
    }
    /// `РежимСохраненияПутейZIP` — второй аргумент `Добавить`.
    ZipStorePathMode("РежимСохраненияПутейZIP", "ZIPStorePathMode")
        meta "ПеречислениеРежимСохраненияПутейZIP" {
        // Имя третьего члена — `НеСохранятьПути`, а не `НеСохранять`
        // (измерено: второе — «Поле объекта не обнаружено»).
        ZipStoreRelativePath(
            "СохранятьОтносительныеПути",
            "StoreRelativePath",
            "Сохранять относительные пути"
        ),
        ZipStoreFullPath("СохранятьПолныеПути", "StoreFullPath", "Сохранять полные пути"),
        ZipDontStorePath("НеСохранятьПути", "DontStorePath", "Не сохранять пути"),
    }
    /// `РежимОбработкиПодкаталоговZIP` — третий аргумент `Добавить`.
    ZipSubDirProcessingMode("РежимОбработкиПодкаталоговZIP", "ZIPSubDirProcessingMode")
        meta "ПеречислениеРежимОбработкиПодкаталоговZIP" {
        ZipDontProcessSubdirs("НеОбрабатывать", "DontProcess", "Не обрабатывать"),
        ZipProcessSubdirsRecursively(
            "ОбрабатыватьРекурсивно",
            "ProcessRecursively",
            "Обрабатывать"
        ),
    }
    /// `МетодШифрованияZIP` — шестой аргумент конструктора; шифрования
    /// здесь нет, члены нужны для отказа.
    ZipEncryptionMethod("МетодШифрованияZIP", "ZIPEncryptionMethod")
        meta "ПеречислениеМетодШифрованияZIP" {
        // Четыре члена, представление каждого измерено — `AES192` отдельной
        // пробой, потому что между 128 и 256 он не подразумевается. Шифрования
        // здесь нет ни одного: члены заведены для честного отказа вместо
        // молчаливо открытого архива.
        ZipEncryptionAes128("AES128", "AES128", "Шифрование методом AES 128 бит"),
        ZipEncryptionAes192("AES192", "AES192", "Шифрование методом AES 192 бит"),
        ZipEncryptionAes256("AES256", "AES256", "Шифрование методом AES 256 бит"),
        ZipEncryptionZip20("Zip20", "Zip20", "Шифрование методом ZIP 2.0"),
    }
    /// `КодировкаИменФайловВZipФайле` — седьмой аргумент конструктора.
    // Английское написание тут не по образцу соседей, а измерено:
    // `Тип("FileNamesEncodingInZipFile")` даёт это перечисление, а
    // `ZipFileNamesEncodingMode`, `ZIPFileNamesEncodingMode` и
    // `ZipFileNamesEncoding` — «Тип не определен».
    ZipFileNamesEncoding("КодировкаИменФайловВZipФайле", "FileNamesEncodingInZipFile")
        meta "ПеречислениеКодировкаИменФайловВZipФайле" {
        // Три члена: `КодировкаОС`, `OEM` и `ANSI` платформа не знает.
        ZipNamesAuto("Авто", "Auto", "Авто"),
        ZipNamesUtf8("UTF8", "Utf8", "UTF8"),
        ZipNamesOsWithUtf8(
            "КодировкаОСДополнительноUTF8",
            "OSEncodingWithUTF8",
            "OS encoding with UTF8"
        ),
    }
    /// Формат сохранения форматированного документа.
    FormattedDocumentFileType("ТипФайлаФорматированногоДокумента", "FormattedDocumentFileType")
        meta "EnumFormattedDocumentFileType" {
        /// Форматы файла форматированного документа, измеренные на 1С.
        FormattedDocumentAnsiTxt("ANSITXT", "ANSITXT", "ANSITXT"),
        FormattedDocumentHtml("HTML", "HTML", "HTML"),
        FormattedDocumentPdf("PDF", "PDF", "PDF"),
        FormattedDocumentTxt("TXT", "TXT", "TXT"),
    }
    /// Управляет сигнатурой при записи строкового HTTP-тела.
    ByteOrderMarkUse("ИспользованиеByteOrderMark", "ByteOrderMarkUse")
        meta "ПеречислениеИспользованиеByteOrderMark" {
        ByteOrderMarkUse("Использовать", "Use", "Использовать"),
        ByteOrderMarkDoNotUse("НеИспользовать", "DoNotUse", "Не использовать"),
    }
    /// Вид бухгалтерского счёта.
    // Представление типа измерено пробой `measure-account-type.bsl`.
    AccountType("ВидСчета", "AccountType") meta "AccountType" {
        // Полный состав, псевдонимы и представления измерены на 8.3.27.2342
        // пробой `measure-account-type.bsl`; текст не выводится из имени члена.
        AccountTypeActive("Активный", "Active", "Assets"),
        AccountTypePassive("Пассивный", "Passive", "Liabilities"),
        AccountTypeActivePassive("АктивноПассивный", "ActivePassive", "Assets/Liabilities"),
    }
    /// Положение картинки внутри элемента графической схемы.
    GraphicalSchemaItemPictureLocation(
        "ПоложениеКартинкиЭлементаГрафическойСхемы",
        "GraphicalSchemaItemPictureLocation"
    )
        meta "ПеречислениеПоложениеКартинкиЭлементаГрафическойСхемы" {
        // Полный состав измерен на 1С 8.3.27.2342.
        // Представления и псевдонимы сняты пробой `graphical-schema/picture-location.bsl`.
        GraphicalSchemaItemPictureLocationTop("Верх", "Top", "Верх"),
        GraphicalSchemaItemPictureLocationLeft("Лево", "Left", "Лево"),
        GraphicalSchemaItemPictureLocationBottom("Низ", "Bottom", "Низ"),
        GraphicalSchemaItemPictureLocationRight("Право", "Right", "Право"),
        GraphicalSchemaItemPictureLocationCenter("Центр", "Center", "Центр"),
    }
    /// `ФигурыГрафическойСхемы` — системное перечисление графической схемы.
    GraphicalSchemaShapes("ФигурыГрафическойСхемы", "GraphicalSchemaShapes")
        meta "ПеречислениеФигурыГрафическойСхемы" {
        // Полные наборы графических констант измерены на 1С 8.3.27.2342.
        // Полные представления сняты пробами `graphical-schema/shapes.bsl` и `line-enums.bsl`.
        GraphicalSchemaShapesBlock("Блок", "Block", "Блок"),
        GraphicalSchemaShapesDocument("Документ", "Document", "Документ"),
        GraphicalSchemaShapesNone("Нет", "None", "Нет"),
        GraphicalSchemaShapesFolder("Папка", "Folder", "Папка"),
        GraphicalSchemaShapesVerticalBrackets(
            "СкобкиВертикальные",
            "VerticalBrackets",
            "Скобки вертикальные"
        ),
        GraphicalSchemaShapesHorizontalBrackets(
            "СкобкиГоризонтальные",
            "HorizontalBrackets",
            "Скобки горизонтальные"
        ),
        GraphicalSchemaShapesUpArrow("СтрелкаВверх", "UpArrow", "Стрелка вверх"),
        GraphicalSchemaShapesUpDownArrow("СтрелкаВверхВниз", "UpDownArrow", "Стрелка вверх-вниз"),
        GraphicalSchemaShapesLeftArrow("СтрелкаВлево", "LeftArrow", "Стрелка влево"),
        GraphicalSchemaShapesLeftRightArrow(
            "СтрелкаВлевоВправо",
            "LeftRightArrow",
            "Стрелка влево-вправо"
        ),
        GraphicalSchemaShapesDownArrow("СтрелкаВниз", "DownArrow", "Стрелка вниз"),
        GraphicalSchemaShapesRightArrow("СтрелкаВправо", "RightArrow", "Стрелка вправо"),
        GraphicalSchemaShapesFile("Файл", "File", "Файл"),
        GraphicalSchemaShapesEllipse("Эллипс", "Ellipse", "Эллипс"),
    }
    /// `ТипСоединительнойЛинии` — системное перечисление графической схемы.
    ConnectorLineType("ТипСоединительнойЛинии", "ConnectorLineType")
        meta "ПеречислениеТипСоединительнойЛинии" {
        ConnectorLineTypeNone("НетЛинии", "None", "Нет линии"),
        ConnectorLineTypeDashed("Пунктир", "Dashed", "Пунктирная"),
        ConnectorLineTypeDashDotted("ПунктирТочка", "DashDotted", "Пунктирно-точечная"),
        ConnectorLineTypeDashDottedDotted(
            "ПунктирТочкаТочка",
            "DashDottedDotted",
            "Точечно-пунктирно-точечная"
        ),
        ConnectorLineTypeSolid("Сплошная", "Solid", "Сплошная"),
        ConnectorLineTypeDotted("Точечная", "Dotted", "Точечная"),
    }
    /// `ПоложениеТекстаСоединительнойЛинии` — системное перечисление графической схемы.
    ConnectorTextLocation("ПоложениеТекстаСоединительнойЛинии", "ConnectorTextLocation")
        meta "ПеречислениеПоложениеТекстаСоединительнойЛинии" {
        ConnectorTextLocationFirstSegment("ПервыйСегмент", "FirstSegment", "Первый сегмент"),
        ConnectorTextLocationMiddle("СерединаЛинии", "Middle", "Середина линии"),
    }
    /// `СтильСтрелки` — системное перечисление графической схемы.
    ArrowStyle("СтильСтрелки", "ArrowStyle") meta "ПеречислениеСтильСтрелки" {
        ArrowStyleFilled("Заполненная", "Filled", "Заполненная"),
        ArrowStyleBlank("Незаполненная", "Blank", "Незаполненная"),
        ArrowStyleNone("Нет", "None", "Нет"),
    }
    /// РазделениеДанныхОбщегоРеквизита.
    CommonAttributeDataSeparation(
        "РазделениеДанныхОбщегоРеквизита",
        "CommonAttributeDataSeparation"
    )
        meta "EnumCommonAttributeDataSeparation" global [] {
        // Значения подтверждены measure-common-attribute-property-enums на 8.3.27.2342.
        CommonAttributeDataSeparationDontUse("НеИспользовать", "DontUse", "DontUse"),
        CommonAttributeDataSeparationSeparate("Разделять", "Separate", "Separate"),
    }
    /// АвтоИспользованиеОбщегоРеквизита.
    CommonAttributeAutoUse("АвтоИспользованиеОбщегоРеквизита", "CommonAttributeAutoUse")
        meta "EnumCommonAttributeAutoUse" global [] {
        CommonAttributeAutoUseDontUse("НеИспользовать", "DontUse", "DontUse"),
        CommonAttributeAutoUseUse("Использовать", "Use", "Use"),
    }
    /// ИспользованиеОбщегоРеквизита.
    CommonAttributeUse("ИспользованиеОбщегоРеквизита", "CommonAttributeUse")
        meta "EnumCommonAttributeUse" global [] {
        CommonAttributeUseAuto("Авто", "Auto", "Auto"),
        CommonAttributeUseDontUse("НеИспользовать", "DontUse", "DontUse"),
        CommonAttributeUseUse("Использовать", "Use", "Use"),
    }
    /// Тип узора табличного документа.
    SpreadsheetDocumentPatternType("ТипУзораТабличногоДокумента", "SpreadsheetDocumentPatternType")
        meta "EnumSpreadsheetDocumentPatternType" {
        /// Отсутствие узора табличного документа.
        SpreadsheetDocumentPatternTypeWithoutPattern("БезУзора", "WithoutPattern", "No pattern"),
    }
    /// Тип расположения области построения диаграммы.
    ChartPlotAreaPlacement("РасположениеОбластиПостроенияДиаграммы", "ChartPlotAreaPlacement")
        meta "EnumChartPlotAreaPlacement" {
        /// Расположение области построения по координатам.
        ChartPlotAreaPlacementUseCoordinates(
            "УказываетсяРасположение",
            "UseCoordinates",
            "Use coordinates"
        ),
    }
    /// Алгоритм подписи токена доступа.
    AccessTokenSignAlgorithm("АлгоритмПодписиТокенаДоступа", "AccessTokenSignAlgorithm")
        meta "EnumAccessTokenSignAlgorithm" {
        /// Измеренный алгоритм подписи токена доступа HS256.
        AccessTokenSignAlgorithmHs256("HS256", "HS256", "HS256"),
    }
}

impl EnumKind {
    fn row(self) -> &'static KindRow {
        &KINDS[self as usize]
    }

    /// Каноническое английское имя системного перечисления.
    #[must_use]
    pub const fn english_name(self) -> &'static str {
        KINDS[self as usize].en
    }

    /// Имя МЕТАТИПА перечисления — то, что печатают `Строка()` и
    /// `Строка(ТипЗнч())` от ГОЛОГО имени перечисления как выражения.
    /// Для `ВариантЗаписиДатыJSON` ИЗМЕРЕНО:
    /// `ПеречислениеВариантЗаписиДатыJSON` — префикс `Перечисление` плюс
    /// русское написание, слитно. `НЕ ИЗМЕРЕНО(JSON.ENUM.BARE_NAME)`:
    /// распространение того же префикса на остальные перечисления и
    /// английское написание метатипа — предположение по образцу.
    pub const fn meta_ru_name(self) -> &'static str {
        KINDS[self as usize].meta
    }

    /// Русское написание самого перечисления — то, что стоит слева от
    /// точки в исходном тексте (`ВариантЗаписиДатыJSON.ЛокальнаяДата`) и то
    /// же, что печатает `EnumValue::enum_name` для любого его члена. Нужно
    /// снаружи модуля для `BslValue::EnumType` — голого имени перечисления
    /// как выражения (`Вычислить("ВариантЗаписиДатыJSON")`).
    pub fn ru_name(self) -> &'static str {
        self.row().ru
    }

    /// `TypeId` членов перечисления — для `ТипЗнч()` члена и голого имени
    /// перечисления как выражения.
    pub(crate) fn type_id(self) -> TypeId {
        self.row().type_id
    }
}

/// Имена всех перечислений — для автодополнения REPL и для резолвера,
/// который по левой части точки решает, перечисление это или переменная.
fn enum_names() -> impl Iterator<Item = (&'static str, EnumKind)> {
    KINDS
        .iter()
        .flat_map(|row| row.global.iter().map(|name| (*name, row.kind)))
}

/// Перечисление по имени слева от точки. Регистронезависимо и на обоих
/// языках, как и всё остальное в языке.
pub fn lookup_enum(name: &str) -> Option<EnumKind> {
    static INDEX: std::sync::OnceLock<std::collections::HashMap<String, EnumKind>> =
        std::sync::OnceLock::new();
    // Поиск вызывается и для обычных идентификаторов при разрешении имён.
    // Повторная свёртка всех постоянных написаний здесь не нужна.
    INDEX
        .get_or_init(|| {
            let mut index = std::collections::HashMap::new();
            for (name, kind) in enum_names() {
                index.entry(name.to_uppercase()).or_insert(kind);
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
    fn row(
        self,
    ) -> &'static (
        EnumKind,
        EnumValue,
        &'static str,
        &'static str,
        &'static str,
    ) {
        &MEMBERS[self as usize]
    }

    /// Что печатает `Строка()`. Измерено на платформе.
    pub fn display_text(self) -> &'static str {
        self.row().4
    }

    /// ИДЕНТИФИКАТОР члена (`НачалоОбъекта`), в отличие от
    /// [`display_text`](Self::display_text) («Начало объекта»). Нужен
    /// текстовому формату байт-кода: печатать туда человеческий текст
    /// нельзя — его не разобрать обратно.
    pub fn member_name(self) -> &'static str {
        self.row().2
    }

    /// Каноническое английское имя члена системного перечисления.
    #[must_use]
    pub fn english_name(self) -> &'static str {
        self.row().3
    }

    /// К какому перечислению принадлежит член.
    pub fn kind(self) -> EnumKind {
        self.row().0
    }

    /// Имя ЗНАЧЕНИЯ для `type_name` — имя самого перечисления без
    /// пробелов, в отличие от имени ТИПА в `types.rs`.
    pub fn enum_name(self) -> &'static str {
        self.kind().row().ru
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
