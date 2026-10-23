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
        CompatibilityModeVersion8_1("Версия8_1", "Version8_1", "Версия8_1"),
        CompatibilityModeVersion8_2_13("Версия8_2_13", "Version8_2_13", "Версия8_2_13"),
        CompatibilityModeVersion8_2_16("Версия8_2_16", "Version8_2_16", "Версия8_2_16"),
        CompatibilityModeVersion8_3_1("Версия8_3_1", "Version8_3_1", "Версия8_3_1"),
        CompatibilityModeVersion8_3_2("Версия8_3_2", "Version8_3_2", "Версия8_3_2"),
        CompatibilityModeVersion8_3_3("Версия8_3_3", "Version8_3_3", "Версия8_3_3"),
        CompatibilityModeVersion8_3_4("Версия8_3_4", "Version8_3_4", "Версия8_3_4"),
        CompatibilityModeVersion8_3_5("Версия8_3_5", "Version8_3_5", "Версия8_3_5"),
        CompatibilityModeVersion8_3_6("Версия8_3_6", "Version8_3_6", "Версия8_3_6"),
        CompatibilityModeVersion8_3_7("Версия8_3_7", "Version8_3_7", "Версия8_3_7"),
        CompatibilityModeVersion8_3_8("Версия8_3_8", "Version8_3_8", "Версия8_3_8"),
        CompatibilityModeVersion8_3_9("Версия8_3_9", "Version8_3_9", "Версия8_3_9"),
        CompatibilityModeVersion8_3_10("Версия8_3_10", "Version8_3_10", "Версия8_3_10"),
        CompatibilityModeVersion8_3_11("Версия8_3_11", "Version8_3_11", "Версия8_3_11"),
        CompatibilityModeVersion8_3_12("Версия8_3_12", "Version8_3_12", "Версия8_3_12"),
        CompatibilityModeVersion8_3_13("Версия8_3_13", "Version8_3_13", "Версия8_3_13"),
        CompatibilityModeVersion8_3_14("Версия8_3_14", "Version8_3_14", "Версия8_3_14"),
        CompatibilityModeVersion8_3_15("Версия8_3_15", "Version8_3_15", "Версия8_3_15"),
        CompatibilityModeVersion8_3_16("Версия8_3_16", "Version8_3_16", "Версия8_3_16"),
        CompatibilityModeVersion8_3_17("Версия8_3_17", "Version8_3_17", "Версия8_3_17"),
        CompatibilityModeVersion8_3_18("Версия8_3_18", "Version8_3_18", "Версия8_3_18"),
        CompatibilityModeVersion8_3_19("Версия8_3_19", "Version8_3_19", "Версия8_3_19"),
        CompatibilityModeVersion8_3_20("Версия8_3_20", "Version8_3_20", "Версия8_3_20"),
        CompatibilityModeVersion8_3_21("Версия8_3_21", "Version8_3_21", "Версия8_3_21"),
        CompatibilityModeVersion8_3_22("Версия8_3_22", "Version8_3_22", "Версия8_3_22"),
        CompatibilityModeVersion8_3_23("Версия8_3_23", "Version8_3_23", "Версия8_3_23"),
        CompatibilityModeVersion8_3_24("Версия8_3_24", "Version8_3_24", "Версия8_3_24"),
        CompatibilityModeVersion8_3_25("Версия8_3_25", "Version8_3_25", "Версия8_3_25"),
        CompatibilityModeVersion8_3_26("Версия8_3_26", "Version8_3_26", "Версия8_3_26"),
        CompatibilityModeDontUse("НеИспользовать", "DontUse", "НеИспользовать"),
    }
    /// Режим запуска клиентского приложения.
    // ИЗМЕРЕНО на 8.3.27 вместе с членами перечисления.
    ClientRunMode("РежимЗапускаКлиентскогоПриложения", "ClientRunMode") meta "ПеречислениеРежимЗапускаКлиентскогоПриложения" {
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
        meta "ПеречислениеРежимОткрытияФормПриложения" {
        // Полный состав, оба написания, представления и типы измерены
        // на платформе 8.3.27.2342 пробой `measure-client-settings.bsl`.
        ApplicationFormsOpenningModeTabs("Закладки", "Tabs", "Закладки"),
        ApplicationFormsOpenningModeSingleWindows(
            "ОтдельныеОкна",
            "SingleWindows",
            "Отдельные окна"
        ),
    }
    /// Вариант интерфейса клиентского приложения.
    ClientApplicationInterfaceVariant(
        "ВариантИнтерфейсаКлиентскогоПриложения",
        "ClientApplicationInterfaceVariant"
    )
        meta "ПеречислениеВариантИнтерфейсаКлиентскогоПриложения" {
        ClientApplicationInterfaceVariantVersion8_2("Версия8_2", "Version8_2", "Версия 8.2"),
        ClientApplicationInterfaceVariantTaxi("Такси", "Taxi", "Такси"),
    }
    /// Способ отображения разделов приложения.
    SectionsPanelRepresentation("ОтображениеПанелиРазделов", "SectionsPanelRepresentation")
        meta "ПеречислениеОтображениеПанелиРазделов" {
        // Полный состав, оба написания, представления и тип измерены
        // на платформе 8.3.27.2342 пробой `measure-command-interface-settings.bsl`.
        SectionsPanelRepresentationPicture("Картинка", "Picture", "Картинка"),
        SectionsPanelRepresentationPictureAndText(
            "КартинкаИТекст",
            "PictureAndText",
            "Картинка и текст"
        ),
        SectionsPanelRepresentationPictureOnTopAndText(
            "КартинкаСверхуИТекст",
            "PictureOnTopAndText",
            "Текст под картинкой"
        ),
        SectionsPanelRepresentationPictureOnLeftAndText(
            "КартинкаСлеваИТекст",
            "PictureOnLeftAndText",
            "Текст справа от картинки"
        ),
        SectionsPanelRepresentationText("Текст", "Text", "Текст"),
    }
    /// Источник списка расширений конфигурации.
    // ИЗМЕРЕНО на 8.3.27 пробой `measure-configuration-extension-enums.bsl`.
    ConfigurationExtensionsSource("ИсточникРасширенийКонфигурации", "ConfigurationExtensionsSource")
        meta "ПеречислениеИсточникРасширенийКонфигурации" {
        // Строковые представления и метатипы трёх перечислений расширений
        // измерены на платформе 8.3.27.2342 отдельной воспроизводимой пробой.
        ConfigurationExtensionsSourceDatabase("БазаДанных", "Database", "БазаДанных"),
        ConfigurationExtensionsSourceSessionApplied(
            "СеансАктивные",
            "SessionApplied",
            "СеансАктивные"
        ),
        ConfigurationExtensionsSourceSessionDisabled(
            "СеансОтключенные",
            "SessionDisabled",
            "СеансОтключенные"
        ),
    }
    /// Область действия расширения конфигурации.
    ConfigurationExtensionScope(
        "ОбластьДействияРасширенияКонфигурации",
        "ConfigurationExtensionScope"
    )
        meta "ПеречислениеОбластьДействияРасширенияКонфигурации" {
        ConfigurationExtensionScopeInfoBase("ИнформационнаяБаза", "InfoBase", "Информационная база"),
        ConfigurationExtensionScopeDataSeparation(
            "РазделениеДанных",
            "DataSeparation",
            "Разделение данных"
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
        ConfigurationExtensionPurposePatch("Исправление", "Patch", "Исправление"),
        ConfigurationExtensionPurposeCustomization("Адаптация", "Customization", "Адаптация"),
        ConfigurationExtensionPurposeAddOn("Дополнение", "AddOn", "Дополнение"),
    }
    /// Важность проблемы применения расширения конфигурации.
    ConfigurationExtensionApplicationIssueSeverity(
        "ВажностьПроблемыПримененияРасширенияКонфигурации",
        "ConfigurationExtensionApplicationIssueSeverity"
    )
        meta "ПеречислениеВажностьПроблемыПримененияРасширенияКонфигурации" {
        ConfigurationExtensionApplicationIssueSeverityCritical("Критичная", "Critical", "Критичная"),
        ConfigurationExtensionApplicationIssueSeverityLow("Низкая", "Low", "Низкая"),
        ConfigurationExtensionApplicationIssueSeverityModerate("Обычная", "Moderate", "Обычная"),
    }
    /// Уровень важности события журнала регистрации.
    // ИЗМЕРЕНО на 8.3.27 пробой `measure-event-log.bsl`.
    EventLogLevel("УровеньЖурналаРегистрации", "EventLogLevel") meta "ПеречислениеУровеньЖурналаРегистрации" {
        // Имена, метатипы и представления измерены на платформе 8.3.27.2342
        // пробой `measure-event-log.bsl`.
        EventLogLevelNote("Примечание", "Note", "Примечание"),
        EventLogLevelInformation("Информация", "Information", "Информация"),
        EventLogLevelWarning("Предупреждение", "Warning", "Предупреждение"),
        EventLogLevelError("Ошибка", "Error", "Ошибка"),
    }
    /// Связь записи журнала регистрации с текущей транзакцией.
    EventLogEntryTransactionMode(
        "РежимТранзакцииЗаписиЖурналаРегистрации",
        "EventLogEntryTransactionMode"
    )
        meta "ПеречислениеРежимТранзакцииЗаписиЖурналаРегистрации" {
        EventLogEntryTransactionModeIndependent("Независимая", "Independent", "Независимая"),
        EventLogEntryTransactionModeTransactional(
            "Транзакционная",
            "Transactional",
            "Транзакционная"
        ),
    }
    /// Статус транзакции записи журнала регистрации.
    EventLogEntryTransactionStatus(
        "СтатусТранзакцииЗаписиЖурналаРегистрации",
        "EventLogEntryTransactionStatus"
    )
        meta "ПеречислениеСтатусТранзакцииЗаписиЖурналаРегистрации" {
        // Полный состав и представления измерены на 8.3.27.2342.
        EventLogEntryTransactionStatusCommitted("Зафиксирована", "Committed", "Зафиксирована"),
        EventLogEntryTransactionStatusUnfinished("НеЗавершена", "Unfinished", "Не завершена"),
        EventLogEntryTransactionStatusNotApplicable(
            "НетТранзакции",
            "NotApplicable",
            "Нет транзакции"
        ),
        EventLogEntryTransactionStatusRolledBack("Отменена", "RolledBack", "Отменена"),
    }
    /// Способ отправки отчёта об ошибке.
    ErrorReportingMode("РежимОтправкиИнформацииОбОшибке", "ErrorReportingMode")
        meta "ПеречислениеРежимОтправкиИнформацииОбОшибке" {
        // Состав, оба написания и представления измерены на платформе 8.3.27.2342
        // пробой `measure-error-processing.bsl`.
        ErrorReportingModeAuto("Авто", "Auto", "Авто"),
        ErrorReportingModeDontSend("НеОтправлять", "DontSend", "Не отправлять"),
        ErrorReportingModeAskUser("СпрашиватьПользователя", "AskUser", "Спрашивать пользователя"),
        ErrorReportingModeSend("Отправлять", "Send", "Отправлять"),
    }
    /// Действие над элементом данных при обмене.
    DataItemSend("ОтправкаЭлементаДанных", "DataItemSend") meta "ПеречислениеОтправкаЭлементаДанных" {
        // Состав, оба написания и представления измерены на платформе 8.3.27.2074
        // пробой `measure-data-item-send.bsl`.
        DataItemSendAuto("Авто", "Auto", "Авто"),
        DataItemSendDelete("Удалить", "Delete", "Удалить"),
        DataItemSendIgnore("Игнорировать", "Ignore", "Игнорировать"),
    }
    /// Действие над полученным элементом данных при обмене.
    DataItemReceive("ПолучениеЭлементаДанных", "DataItemReceive") meta "ПеречислениеПолучениеЭлементаДанных" {
        // Состав, оба написания и представления измерены на платформе 8.3.27.2074
        // пробой `measure-data-item-receive.bsl`.
        DataItemReceiveAuto("Авто", "Auto", "Авто"),
        DataItemReceiveAccept("Принять", "Accept", "Принять"),
        DataItemReceiveIgnore("Игнорировать", "Ignore", "Игнорировать"),
    }
    /// Режим обновления предопределённых данных.
    PredefinedDataUpdate("ОбновлениеПредопределенныхДанных", "PredefinedDataUpdate")
        meta "ПеречислениеОбновлениеПредопределенныхДанных" {
        // Состав, оба написания и представления измерены на платформе 8.3.27.2342
        // пробой `measure-infobase-predefined-data-update.bsl`.
        PredefinedDataUpdateAuto("Авто", "Auto", "Авто"),
        PredefinedDataUpdateAutoUpdate("ОбновлятьАвтоматически", "AutoUpdate", "Обновлять автоматически"),
        PredefinedDataUpdateDontAutoUpdate(
            "НеОбновлятьАвтоматически",
            "DontAutoUpdate",
            "Не обновлять автоматически"
        ),
    }
    /// Режим проведения документа.
    DocumentPostingMode("РежимПроведенияДокумента", "DocumentPostingMode")
        meta "ПеречислениеРежимПроведенияДокумента" {
        DocumentPostingModeOperational("Оперативный", "RealTime", "Оперативный"),
        DocumentPostingModeNonOperational("Неоперативный", "Regular", "Неоперативный"),
    }
    /// Режим записи документа.
    DocumentWriteMode("РежимЗаписиДокумента", "DocumentWriteMode") meta "ПеречислениеРежимЗаписиДокумента" {
        DocumentWriteModeWrite("Запись", "Write", "Запись"),
        DocumentWriteModePosting("Проведение", "Posting", "Проведение"),
        DocumentWriteModeUndoPosting("ОтменаПроведения", "UndoPosting", "Отмена проведения"),
    }
    /// Способ обхода записей результата запроса.
    // ИЗМЕРЕНО на 8.3.27 пробой `measure-query-result-iteration.bsl`.
    QueryResultIteration("ОбходРезультатаЗапроса", "QueryResultIteration")
        meta "ПеречислениеОбходРезультатаЗапроса" {
        // Полный состав, оба написания, представления и тип измерены на
        // платформе 8.3.27 пробой `measure-query-result-iteration.bsl`.
        QueryResultIterationLinear("Прямой", "Linear", "Прямой"),
        QueryResultIterationByGroups("ПоГруппировкам", "ByGroups", "По группировкам"),
        QueryResultIterationByGroupsWithHierarchy(
            "ПоГруппировкамСИерархией",
            "ByGroupsWithHierarchy",
            "По группировкам с иерархией"
        ),
    }
    /// Тип связи наборов данных компоновки данных.
    DataCompositionDataSetsLinkType(
        "ТипСвязиНаборовДанныхКомпоновкиДанных",
        "DataCompositionDataSetsLinkType"
    )
        meta "ПеречислениеТипСвязиНаборовДанныхКомпоновкиДанных" {
        // Члены, алиасы, типы и представления измерены на 8.3.27.2342.
        DataCompositionDataSetsLinkTypeOuter("Внешняя", "Outer", "Внешняя"),
        DataCompositionDataSetsLinkTypeInner("Внутренняя", "Inner", "Внутренняя"),
    }
    /// Тип соединения значений по сериям диаграммы.
    ChartValuesBySeriesConnectionType(
        "ТипСоединенияЗначенийПоСериямДиаграммы",
        "ChartValuesBySeriesConnectionType"
    )
        meta "ПеречислениеТипСоединенияЗначенийПоСериямДиаграммы" {
        ChartValuesBySeriesConnectionTypeNone("Нет", "None", "Нет"),
        ChartValuesBySeriesConnectionTypeEdgesConnection(
            "СоединениеКраев",
            "EdgesConnection",
            "Соединение краев"
        ),
    }
    /// Расположение легенды диаграммы компоновки данных.
    DataCompositionChartLegendPlacement(
        "РасположениеЛегендыДиаграммыКомпоновкиДанных",
        "DataCompositionChartLegendPlacement"
    )
        meta "ПеречислениеРасположениеЛегендыДиаграммыКомпоновкиДанных" {
        DataCompositionChartLegendPlacementNone("Нет", "None", "Нет"),
        DataCompositionChartLegendPlacementLeft("Лево", "Left", "Лево"),
        DataCompositionChartLegendPlacementRight("Право", "Right", "Право"),
        DataCompositionChartLegendPlacementTop("Верх", "Top", "Верх"),
        DataCompositionChartLegendPlacementBottom("Низ", "Bottom", "Низ"),
    }
    /// Тип записи выборки результата запроса.
    QueryRecordType("ТипЗаписиЗапроса", "QueryRecordType") meta "ПеречислениеТипЗаписиЗапроса" {
        QueryRecordTypeDetailRecord("ДетальнаяЗапись", "DetailRecord", "Детальная запись"),
        QueryRecordTypeGroupTotal("ИтогПоГруппировке", "GroupTotal", "Итог по группировке"),
        QueryRecordTypeTotalByHierarchy("ИтогПоИерархии", "TotalByHierarchy", "Итог по иерархии"),
        QueryRecordTypeOverall("ОбщийИтог", "Overall", "Общий итог"),
    }
    /// Режим управляемой блокировки данных.
    // ИЗМЕРЕНО на 8.3.27 пробой `measure-data-lock-mode.bsl`.
    DataLockMode("РежимБлокировкиДанных", "DataLockMode") meta "ПеречислениеРежимБлокировкиДанных" {
        // Полный состав, оба написания, представления и тип измерены на
        // платформе 8.3.27 пробой `measure-data-lock-mode.bsl`.
        DataLockModeShared("Разделяемый", "Shared", "Разделяемый"),
        DataLockModeExclusive("Исключительный", "Exclusive", "Исключительный"),
    }
    /// Способ округления половины.
    RoundMode("РежимОкругления", "RoundMode") meta "ПеречислениеРежимОкругления" {
        // Полный состав и представления измерены пробой `measure-round-mode.bsl`.
        RoundModeRound15as10("Окр15как10", "Round15as10", "Округлять 15 как 10"),
        RoundModeRound15as20("Окр15как20", "Round15as20", "Округлять 15 как 20"),
    }
    /// Вид числового значения.
    NumericValueType("ВидЧисловогоЗначения", "NumericValueType") meta "ПеречислениеВидЧисловогоЗначения" {
        // Полный состав измерен пробой `measure-numeric-value-type.bsl`.
        NumericValueTypeCardinal("Количественное", "Cardinal", "Количественное"),
        NumericValueTypeOrdinal("Порядковое", "Ordinal", "Порядковое"),
    }
    /// Способ отображения сообщения об ошибке.
    ErrorMessageDisplayVariant("ВариантОтображенияСообщенияОбОшибке", "ErrorMessageDisplayVariant")
        meta "ПеречислениеВариантОтображенияСообщенияОбОшибке" {
        // Полный состав и пробелы представлений измерены на 8.3.27.2342.
        ErrorMessageDisplayVariantAuto("Авто", "Auto", "Авто"),
        ErrorMessageDisplayVariantErrorMessageForUser(
            "СообщениеОбОшибкеДляПользователя",
            "ErrorMessageForUser",
            "Сообщение об ошибке для пользователя "
        ),
        ErrorMessageDisplayVariantBriefErrorDescription(
            "КраткоеПредставлениеОшибки",
            "BriefErrorDescription",
            "Краткое представление ошибки "
        ),
        ErrorMessageDisplayVariantDetailErrorDescription(
            "ПодробноеПредставлениеОшибки",
            "DetailErrorDescription",
            "Подробное представление ошибки "
        ),
    }
    /// Режим использования параметра команды.
    CommandParameterUseMode("РежимИспользованияПараметраКоманды", "CommandParameterUseMode")
        meta "ПеречислениеРежимИспользованияПараметраКоманды" {
        // Полный состав, представления и английские имена измерены на 8.3.27.2342.
        CommandParameterUseModeSingle("Одиночный", "Single", "Одиночный"),
        CommandParameterUseModeMultiple("Множественный", "Multiple", "Множественный"),
    }
    /// Способ поиска строки при вводе по строке.
    SearchStringModeOnInputByString(
        "СпособПоискаСтрокиПриВводеПоСтроке",
        "SearchStringModeOnInputByString"
    )
        meta "ПеречислениеСпособПоискаСтрокиПриВводеПоСтроке" {
        // Полный состав и представления измерены на 8.3.27.2342.
        SearchStringModeOnInputByStringBegin("Начало", "Begin", "Начало"),
        SearchStringModeOnInputByStringAnyPart("ЛюбаяЧасть", "AnyPart", "Любая часть"),
    }
    /// Тип файла пакета отображаемых документов.
    RepresentableDocumentBatchFileType(
        "ТипФайлаПакетаОтображаемыхДокументов",
        "RepresentableDocumentBatchFileType"
    )
        meta "ПеречислениеТипФайлаПакетаОтображаемыхДокументов" {
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
    FontType("ВидШрифта", "FontType") meta "ПеречислениеВидШрифта" {
        // Полный состав и представления измерены на 8.3.27.2342.
        FontTypeWindowsFont("WindowsШрифт", "WindowsFont", "Windows шрифт"),
        FontTypeAbsolute("Абсолютный", "Absolute", "Абсолютный"),
        FontTypeAutoFont("АвтоШрифт", "AutoFont", "Авто шрифт"),
        FontTypeStyleItem("ЭлементСтиля", "StyleItem", "Элемент стиля"),
    }
    /// Вид картинки.
    PictureType("ВидКартинки", "PictureType") meta "ПеречислениеВидКартинки" {
        // Полный состав и представления измерены на 8.3.27.2342.
        PictureTypeAbsolute("Абсолютная", "Absolute", "Абсолютная"),
        PictureTypeFromLib("ИзБиблиотеки", "FromLib", "Из библиотеки"),
        PictureTypeEmpty("Пустая", "Empty", "Пустая"),
    }
    /// Использование метаданных при полнотекстовом поиске.
    FullTextSearchMetadataUse(
        "ИспользованиеМетаданныхПолнотекстовогоПоиска",
        "FullTextSearchMetadataUse"
    )
        meta "ПеречислениеИспользованиеМетаданныхПолнотекстовогоПоиска" {
        // Полный состав и представления измерены на 8.3.27.2342.
        FullTextSearchMetadataUseUse("Использовать", "Use", "Использовать"),
        FullTextSearchMetadataUseDontUse("НеИспользовать", "DontUse", "Не использовать"),
    }
    /// Режим изменения связанного значения.
    LinkedValueChangeMode("РежимИзмененияСвязанногоЗначения", "LinkedValueChangeMode")
        meta "ПеречислениеРежимИзмененияСвязанногоЗначения" {
        // Полный состав и представления измерены на 8.3.27.2342.
        LinkedValueChangeModeDontChange("НеИзменять", "DontChange", "Не изменять"),
        LinkedValueChangeModeClear("Очищать", "Clear", "Очищать"),
    }
    /// Способ кодирования вложения интернет-почты.
    InternetMailAttachmentEncodingMode(
        "СпособКодированияИнтернетПочтовогоВложения",
        "InternetMailAttachmentEncodingMode"
    )
        meta "ПеречислениеСпособКодированияИнтернетПочтовогоВложения" {
        // Полный состав и представления измерены на 8.3.27.2342.
        InternetMailAttachmentEncodingModeMime("MIME", "MIME", "MIME"),
        InternetMailAttachmentEncodingModeUuEncode("UUEncode", "UUEncode", "UUEncode"),
    }
    /// Автоматическое сохранение данных формы в настройках.
    AutoSaveFormDataInSettings(
        "АвтоматическоеСохранениеДанныхФормыВНастройках",
        "AutoSaveFormDataInSettings"
    )
        meta "ПеречислениеАвтоматическоеСохранениеДанныхФормыВНастройках" {
        // Полный состав и представления измерены на 8.3.27.2342.
        AutoSaveFormDataInSettingsUse("Использовать", "Use", "Использовать"),
        AutoSaveFormDataInSettingsDontUse("НеИспользовать", "DontUse", "Не использовать"),
    }
    /// Формат файла документа PDF.
    PDFDocumentFileType("ТипФайлаДокументаPDF", "PDFDocumentFileType") meta "ПеречислениеТипФайлаДокументаPDF" {
        // Полный состав и представления измерены на 8.3.27.2342.
        PDFDocumentFileTypePdf("PDF", "PDF", "PDF"),
        PDFDocumentFileTypePdfA1("PDF_A_1", "PDF_A_1", "PDF/A-1"),
        PDFDocumentFileTypePdfA2("PDF_A_2", "PDF_A_2", "PDF/A-2"),
        PDFDocumentFileTypePdfA3("PDF_A_3", "PDF_A_3", "PDF/A-3"),
    }
    /// Допустимый знак числа.
    AllowedSign("ДопустимыйЗнак", "AllowedSign") meta "ПеречислениеДопустимыйЗнак" {
        // Полный состав и представления измерены на платформе 8.3.27.2074
        // пробой `measure-allowed-sign.bsl`.
        AllowedSignAny("Любой", "Any", "Любой"),
        AllowedSignNonnegative("Неотрицательный", "Nonnegative", "Неотрицательный"),
    }
    /// Режим автоматической регистрации изменений данных.
    AutoChangeRecord("АвтоРегистрацияИзменений", "AutoChangeRecord") meta "ПеречислениеАвтоРегистрацияИзменений" {
        // Полный состав, оба написания, представления и тип измерены
        // на платформе 8.3.27.2342 пробой `measure-auto-change-record.bsl`.
        AutoChangeRecordDeny("Запретить", "Deny", "Запретить"),
        AutoChangeRecordAllow("Разрешить", "Allow", "Разрешить"),
    }
    /// Способ отображения кнопки формы.
    ButtonRepresentation("ОтображениеКнопки", "ButtonRepresentation") meta "ПеречислениеОтображениеКнопки" {
        // Полный состав и представления измерены на платформе 8.3.27.2342
        // пробой `measure-button-representation.bsl`.
        ButtonRepresentationAuto("Авто", "Auto", "Авто"),
        ButtonRepresentationPicture("Картинка", "Picture", "Картинка"),
        ButtonRepresentationPictureAndText("КартинкаИТекст", "PictureAndText", "Картинка и текст"),
        ButtonRepresentationText("Текст", "Text", "Текст"),
    }
    /// Причина несоответствия пароля политике.
    // ИЗМЕРЕНО на 8.3.27 пробой
    // `measure-password-policy-compliance-check-result.bsl`.
    PasswordPolicyComplianceCheckResult(
        "РезультатПроверкиСоответствияПароляПолитике",
        "PasswordPolicyComplianceCheckResult"
    )
        meta "ПеречислениеРезультатПроверкиСоответствияПароляПолитике" {
        // Полный состав, оба написания, представления и тип измерены
        // на платформе 8.3.27.2342 пробой
        // `measure-password-policy-compliance-check-result.bsl`.
        PasswordPolicyComplianceCheckResultMinLength(
            "НеСоответствуетТребованиямМинимальнойДлины",
            "DoesNotSatisfyMinLengthRequirements",
            "Не соответствует требованиям минимальной длины"
        ),
        PasswordPolicyComplianceCheckResultReuseLimit(
            "НеСоответствуетТребованиямОграниченияПовторенияСредиПоследних",
            "DoesNotSatisfyReuseLimitRequirements",
            "Не соответствует требованиям ограничения повторения среди последних"
        ),
        PasswordPolicyComplianceCheckResultCompromiseCheck(
            "НеСоответствуетТребованиямПроверкиРаскрытия",
            "DoesNotSatisfyCompromiseCheckRequirements",
            "Не соответствует требованиям проверки раскрытия"
        ),
        PasswordPolicyComplianceCheckResultComplexity(
            "НеСоответствуетТребованиямСложности",
            "DoesNotSatisfyComplexityRequirements",
            "Не соответствует требованиям сложности"
        ),
    }
    /// Способ отображения данных в таблице формы.
    // ИЗМЕРЕНО на 8.3.27 пробой `measure-table-representation.bsl`.
    TableRepresentation("ОтображениеТаблицы", "TableRepresentation") meta "ПеречислениеОтображениеТаблицы" {
        // Полный состав, оба написания, представления и тип измерены
        // на платформе 8.3.27.2342 пробой `measure-table-representation.bsl`.
        TableRepresentationList("Список", "List", "Список"),
        TableRepresentationHierarchicalList(
            "ИерархическийСписок",
            "HierarchicalList",
            "Иерархический список"
        ),
        TableRepresentationTree("Дерево", "Tree", "Дерево"),
    }
    /// Вид поля формы клиентского приложения.
    // ИЗМЕРЕНО на 8.3.27 пробой `measure-form-field-type.bsl`.
    FormFieldType("ВидПоляФормы", "FormFieldType") meta "ПеречислениеВидПоляФормы" {
        // Полный состав, оба написания, представления и тип измерены
        // на платформе 8.3.27.2342 пробой `measure-form-field-type.bsl`.
        FormFieldTypeHtmlDocumentField(
            "ПолеHTMLДокумента",
            "HTMLDocumentField",
            "Поле HTML документа"
        ),
        FormFieldTypePdfDocumentField("ПолеPDFДокумента", "PDFDocumentField", "Поле PDF документа"),
        FormFieldTypeInputField("ПолеВвода", "InputField", "Поле ввода"),
        FormFieldTypeGeographicalSchemaField(
            "ПолеГеографическойСхемы",
            "GeographicalSchemaField",
            "Поле географической схемы"
        ),
        FormFieldTypeGraphicalSchemaField(
            "ПолеГрафическойСхемы",
            "GraphicalSchemaField",
            "Поле графической схемы"
        ),
        FormFieldTypeDendrogramField("ПолеДендрограммы", "DendrogramField", "Поле дендрограммы"),
        FormFieldTypeChartField("ПолеДиаграммы", "ChartField", "Поле диаграммы"),
        FormFieldTypeGanttChartField("ПолеДиаграммыГанта", "GanttChartField", "Поле диаграммы Ганта"),
        FormFieldTypeProgressBarField("ПолеИндикатора", "ProgressBarField", "Поле индикатора"),
        FormFieldTypeCalendarField("ПолеКалендаря", "CalendarField", "Поле календаря"),
        FormFieldTypePictureField("ПолеКартинки", "PictureField", "Поле картинки"),
        FormFieldTypeLabelField("ПолеНадписи", "LabelField", "Поле надписи"),
        FormFieldTypeRadioButtonField("ПолеПереключателя", "RadioButtonField", "Поле переключателя"),
        FormFieldTypePeriodField("ПолеПериода", "PeriodField", "Поле периода"),
        FormFieldTypePlannerField("ПолеПланировщика", "PlannerField", "Поле данных планировщика"),
        FormFieldTypeTrackBarField("ПолеПолосыРегулирования", "TrackBarField", "Поле полосы регулирования"),
        FormFieldTypeSpreadsheetDocumentField(
            "ПолеТабличногоДокумента",
            "SpreadsheetDocumentField",
            "Поле табличного документа"
        ),
        FormFieldTypeTextDocumentField(
            "ПолеТекстовогоДокумента",
            "TextDocumentField",
            "Поле текстового документа"
        ),
        FormFieldTypeCheckBoxField("ПолеФлажка", "CheckBoxField", "Поле флажка"),
        FormFieldTypeFormattedDocumentField(
            "ПолеФорматированногоДокумента",
            "FormattedDocumentField",
            "Поле форматированного документа"
        ),
    }
    /// Вид группы формы клиентского приложения.
    // ИЗМЕРЕНО на 8.3.27 пробой `measure-form-group-type.bsl`.
    FormGroupType("ВидГруппыФормы", "FormGroupType") meta "ПеречислениеВидГруппыФормы" {
        // Полный состав, оба написания, представления и тип измерены
        // на платформе 8.3.27.2342 пробой `measure-form-group-type.bsl`.
        FormGroupTypeButtonGroup("ГруппаКнопок", "ButtonGroup", "Группа кнопок"),
        FormGroupTypeColumnGroup("ГруппаКолонок", "ColumnGroup", "Группа колонок"),
        FormGroupTypeCommandBar("КоманднаяПанель", "CommandBar", "Командная панель"),
        FormGroupTypeContextMenu("КонтекстноеМеню", "ContextMenu", "Контекстное меню"),
        FormGroupTypeUsualGroup("ОбычнаяГруппа", "UsualGroup", "Обычная группа"),
        FormGroupTypePopup("Подменю", "Popup", "Подменю"),
        FormGroupTypePage("Страница", "Page", "Страница"),
        FormGroupTypePages("Страницы", "Pages", "Страницы"),
    }
    /// Горизонтальное положение элемента формы.
    // ИЗМЕРЕНО на 8.3.27 пробой `measure-item-horizontal-location.bsl`.
    ItemHorizontalLocation("ГоризонтальноеПоложениеЭлемента", "ItemHorizontalLocation")
        meta "ПеречислениеГоризонтальноеПоложениеЭлемента" {
        // Полный состав, оба написания, представления и тип измерены
        // на платформе 8.3.27.2342 пробой `measure-item-horizontal-location.bsl`.
        ItemHorizontalLocationAuto("Авто", "Auto", "Авто"),
        ItemHorizontalLocationLeft("Лево", "Left", "Лево"),
        ItemHorizontalLocationRight("Право", "Right", "Право"),
        ItemHorizontalLocationCenter("Центр", "Center", "Центр"),
    }
    /// Вертикальное положение элемента формы.
    // ИЗМЕРЕНО на 8.3.27 пробой `measure-form-alignment-spacing-enums.bsl`.
    ItemVerticalAlign("ВертикальноеПоложениеЭлемента", "ItemVerticalAlign")
        meta "ПеречислениеВертикальноеПоложениеЭлемента" {
        // Полный состав и представления измерены на платформе 8.3.27.2342
        // пробой `measure-form-alignment-spacing-enums.bsl`.
        ItemVerticalAlignAuto("Авто", "Auto", "Авто"),
        ItemVerticalAlignTop("Верх", "Top", "Верх"),
        ItemVerticalAlignBottom("Низ", "Bottom", "Низ"),
        ItemVerticalAlignCenter("Центр", "Center", "Центр"),
    }
    /// Интервал между элементами формы.
    FormItemSpacing("ИнтервалМеждуЭлементамиФормы", "FormItemSpacing") meta "ПеречислениеИнтервалМеждуЭлементамиФормы" {
        FormItemSpacingAuto("Авто", "Auto", "Авто"),
        FormItemSpacingDouble("Двойной", "Double", "Двойной"),
        FormItemSpacingNone("Нет", "None", "Нет"),
        FormItemSpacingSingle("Одинарный", "Single", "Одинарный"),
        FormItemSpacingHalf("Половинный", "Half", "Половинный"),
        FormItemSpacingOneAndHalf("Полуторный", "OneAndHalf", "Полуторный"),
    }
    /// Отображение фигуры кнопки формы.
    // ИЗМЕРЕНО на 8.3.27 пробой `measure-button-shape-representation.bsl`.
    ButtonShapeRepresentation("ОтображениеФигурыКнопки", "ButtonShapeRepresentation")
        meta "ПеречислениеОтображениеФигурыКнопки" {
        // Полный состав, оба написания, представления и тип измерены
        // на платформе 8.3.27.2342 пробой `measure-button-shape-representation.bsl`.
        ButtonShapeRepresentationAuto("Авто", "Auto", "Авто"),
        ButtonShapeRepresentationAlways("Всегда", "Always", "Всегда"),
        ButtonShapeRepresentationNone("Нет", "None", "Нет"),
        ButtonShapeRepresentationWhenActive("ПриАктивности", "WhenActive", "При активности"),
    }
    /// Отображение обычной группы формы.
    // ИЗМЕРЕНО на 8.3.27 пробой `measure-usual-group-representation.bsl`.
    UsualGroupRepresentation("ОтображениеОбычнойГруппы", "UsualGroupRepresentation")
        meta "ПеречислениеОтображениеОбычнойГруппы" {
        // Полный состав, оба написания, представления и тип измерены
        // на платформе 8.3.27.2342 пробой `measure-usual-group-representation.bsl`.
        UsualGroupRepresentationNone("Нет", "None", "Нет"),
        UsualGroupRepresentationNormalSeparation(
            "ОбычноеВыделение",
            "NormalSeparation",
            "Обычное выделение"
        ),
        UsualGroupRepresentationStrongSeparation(
            "СильноеВыделение",
            "StrongSeparation",
            "Сильное выделение"
        ),
        UsualGroupRepresentationWeakSeparation(
            "СлабоеВыделение",
            "WeakSeparation",
            "Слабое выделение"
        ),
    }
    /// Важность элемента формы при отображении.
    // ИЗМЕРЕНО на 8.3.27 пробой `measure-display-importance.bsl`.
    DisplayImportance("ВажностьПриОтображении", "DisplayImportance") meta "ПеречислениеВажностьПриОтображении" {
        // Полный состав, оба написания, представления и тип измерены
        // на платформе 8.3.27.2342 пробой `measure-display-importance.bsl`.
        DisplayImportanceAuto("Авто", "Auto", "Авто"),
        DisplayImportanceHigh("Высокая", "High", "Высокая"),
        DisplayImportanceLow("Низкая", "Low", "Низкая"),
        DisplayImportanceUsual("Обычная", "Usual", "Обычная"),
        DisplayImportanceVeryHigh("ОченьВысокая", "VeryHigh", "Очень высокая"),
        DisplayImportanceVeryLow("ОченьНизкая", "VeryLow", "Очень низкая"),
    }
    /// Формат хранения картинки.
    // ИЗМЕРЕНО на 8.3.27 пробой `measure-picture-format.bsl`.
    PictureFormat("ФорматКартинки", "PictureFormat") meta "ПеречислениеФорматКартинки" {
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
        PictureFormatUnknown("НеизвестныйФормат", "UnknownFormat", "Неизвестный формат"),
    }
    /// Тип текста почтового сообщения.
    // ИЗМЕРЕНО на 8.3.27 пробой `measure-internet-mail-text-type.bsl`.
    InternetMailTextType("ТипТекстаПочтовогоСообщения", "InternetMailTextType")
        meta "ПеречислениеТипТекстаПочтовогоСообщения" {
        // Полный состав, оба написания, представления и тип измерены
        // на платформе 8.3.27.2342 пробой `measure-internet-mail-text-type.bsl`.
        InternetMailTextTypeHtml("HTML", "HTML", "HTML"),
        InternetMailTextTypeCustomText("ПроизвольныйТекст", "CustomText", "ПроизвольныйТекст"),
        InternetMailTextTypePlainText("ПростойТекст", "PlainText", "ПростойТекст"),
        InternetMailTextTypeRichText("РазмеченныйТекст", "RichText", "РазмеченныйТекст"),
    }
    /// Способ кодирования не-ASCII символов почтового сообщения.
    // ИЗМЕРЕНО на 8.3.27 пробой `measure-mail-non-ascii-encoding-mode.bsl`.
    InternetMailNonAsciiEncodingMode(
        "СпособКодированияНеASCIIСимволовИнтернетПочтовогоСообщения",
        "InternetMailMessageNonASCIISymbolsEncodingMode"
    )
        meta "ПеречислениеСпособКодированияНеASCIIСимволовИнтернетПочтовогоСообщения" {
        // Полный состав, оба написания, представления и тип измерены
        // на платформе 8.3.27.2342 пробой `measure-mail-non-ascii-encoding-mode.bsl`.
        InternetMailNonAsciiMime("MIME", "MIME", "MIME"),
        InternetMailNonAsciiQuotedPrintable(
            "QuotedPrintable",
            "QuotedPrintable",
            "QuotedPrintable"
        ),
        InternetMailNonAsciiNone("БезКодирования", "None", "БезКодирования"),
    }
    /// Важность интернет-почтового сообщения.
    // ИЗМЕРЕНО на 8.3.27 пробой `measure-mail-message-importance.bsl`.
    InternetMailMessageImportance(
        "ВажностьИнтернетПочтовогоСообщения",
        "InternetMailMessageImportance"
    )
        meta "ПеречислениеВажностьИнтернетПочтовогоСообщения" {
        // Полный состав, оба написания, представления и тип измерены
        // на платформе 8.3.27.2342 пробой `measure-mail-message-importance.bsl`.
        InternetMailMessageImportanceHigh("Высокая", "High", "Высокая"),
        InternetMailMessageImportanceHighest("Наивысшая", "Highest", "Наивысшая"),
        InternetMailMessageImportanceLowest("Наименьшая", "Lowest", "Наименьшая"),
        InternetMailMessageImportanceLow("Низкая", "Low", "Низкая"),
        InternetMailMessageImportanceNormal("Обычная", "Normal", "Обычная"),
    }
    /// Дополнительный способ отображения состояния.
    // ИЗМЕРЕНО на 8.3.27 пробой `measure-additional-show-mode.bsl`.
    AdditionalShowMode("ДополнительныйРежимОтображения", "AdditionalShowMode")
        meta "ПеречислениеДополнительныйРежимОтображения" {
        // Полный состав, оба написания, представления и тип измерены
        // на платформе 8.3.27.2342 пробой `measure-additional-show-mode.bsl`.
        AdditionalShowModeDontUse("НеИспользовать", "DontUse", "Не использовать"),
        AdditionalShowModeIrrelevance("Неактуальность", "Irrelevance", "Неактуальность"),
    }
    /// Режим проверки отзыва TLS-сертификата сервера.
    // ИЗМЕРЕНО на 8.3.27 пробой `measure-tls-revocation-mode.bsl`.
    ServerTlsCertificateRevocationCheckMode(
        "РежимПроверкиОтзываTLSСертификатаСервера",
        "ServerTLSCertificateRevocationCheckMode"
    )
        meta "ПеречислениеРежимПроверкиОтзываTLSСертификатаСервера" {
        // Полный состав, оба написания, представления и тип измерены
        // на платформе 8.3.27.2342 пробой `measure-tls-revocation-mode.bsl`.
        ServerTlsCertificateRevocationCheckModeAuto("Авто", "Auto", "Авто"),
        ServerTlsCertificateRevocationCheckModeDontCheck(
            "НеПроверять",
            "DontCheck",
            "Проверка отзыва сертификатов выключена"
        ),
        ServerTlsCertificateRevocationCheckModeSoftFail(
            "Нестрогий",
            "SoftFail",
            "Мягкая проверка отзыва сертификатов"
        ),
        ServerTlsCertificateRevocationCheckModeStrict(
            "Строгий",
            "Strict",
            "Строгая проверка отзыва сертификатов"
        ),
    }
    /// Область применения отбора к записям результа компоновки.
    // ИЗМЕРЕНО на 8.3.27 пробой
    // `measure-data-composition-filter-application-type.bsl`.
    DataCompositionFilterApplicationType(
        "ТипПримененияОтбораКомпоновкиДанных",
        "DataCompositionFilterApplicationType"
    )
        meta "ПеречислениеТипПримененияОтбораКомпоновкиДанных" {
        // Полный состав, оба написания, представления и тип измерены
        // на платформе 8.3.27.2342 пробой
        // `measure-data-composition-filter-application-type.bsl`.
        DataCompositionFilterApplicationTypeItems("Элементы", "Items", "Обычное"),
        DataCompositionFilterApplicationTypeHierarchy("Иерархия", "Hierarchy", "После группировки или для иерархии"),
        DataCompositionFilterApplicationTypeHierarchyOnly(
            "ТолькоИерархия",
            "HierarchyOnly",
            "Только иерархия"
        ),
    }
    /// Состояние элемента настройки компоновки данных.
    // ИЗМЕРЕНО на 8.3.27 пробой
    // `measure-data-composition-settings-item-view-mode.bsl`.
    DataCompositionSettingsItemState(
        "СостояниеЭлементаНастройкиКомпоновкиДанных",
        "DataCompositionSettingsItemState"
    )
        meta "ПеречислениеСостояниеЭлементаНастройкиКомпоновкиДанных" {
        // Полный состав, алиасы и представления измерены на 8.3.27.2342.
        DataCompositionSettingsItemStateEnabled("Включен", "Enabled", "Включена"),
        DataCompositionSettingsItemStateDisabled("Отключен", "Disabled", "Отключена"),
        DataCompositionSettingsItemStateDeletedByUser(
            "УдаленПользователем",
            "DeletedByUser",
            "Удалена пользователем"
        ),
    }
    /// Режим отображения элемента настройки компоновки.
    DataCompositionSettingsItemViewMode(
        "РежимОтображенияЭлементаНастройкиКомпоновкиДанных",
        "DataCompositionSettingsItemViewMode"
    )
        meta "ПеречислениеРежимОтображенияЭлементаНастройкиКомпоновкиДанных" {
        // Полный состав, оба написания, представления и тип измерены
        // на платформе 8.3.27.2342 пробой
        // `measure-data-composition-settings-item-view-mode.bsl`.
        DataCompositionSettingsItemViewModeAuto("Авто", "Auto", "Авто"),
        DataCompositionSettingsItemViewModeNormal("Обычный", "Normal", "Обычный"),
        DataCompositionSettingsItemViewModeQuickAccess(
            "БыстрыйДоступ",
            "QuickAccess",
            "Быстрый доступ"
        ),
        DataCompositionSettingsItemViewModeInaccessible("Недоступный", "Inaccessible", "Недоступный"),
    }
    /// Способ восстановления настроек компоновки данных.
    // ИЗМЕРЕНО на 8.3.27 пробой
    // `measure-data-composition-settings-refresh-method.bsl`.
    DataCompositionSettingsRefreshMethod(
        "СпособВосстановленияНастроекКомпоновкиДанных",
        "DataCompositionSettingsRefreshMethod"
    )
        meta "ПеречислениеСпособВосстановленияНастроекКомпоновкиДанных" {
        // Полный состав и представления измерены на платформе 8.3.27.2342
        // пробой `measure-data-composition-settings-refresh-method.bsl`.
        DataCompositionSettingsRefreshMethodFull("Полное", "Full", "Полное"),
        DataCompositionSettingsRefreshMethodCheckAvailability(
            "ПроверятьДоступность",
            "CheckAvailability",
            "Проверять доступность"
        ),
    }
    /// Направление сортировки компоновки данных.
    DataCompositionSortDirection(
        "НаправлениеСортировкиКомпоновкиДанных",
        "DataCompositionSortDirection"
    )
        meta "ПеречислениеНаправлениеСортировкиКомпоновкиДанных" {
        // Представления и оба написания измерены на 8.3.27.
        DataCompositionSortDirectionAsc("Возр", "Asc", "По возрастанию"),
        DataCompositionSortDirectionDesc("Убыв", "Desc", "По убыванию"),
    }
    /// Тип группировки компоновки данных.
    DataCompositionGroupType("ТипГруппировкиКомпоновкиДанных", "DataCompositionGroupType")
        meta "ПеречислениеТипГруппировкиКомпоновкиДанных" {
        // Представления и оба написания измерены на 8.3.27.
        DataCompositionGroupTypeHierarchy("Иерархия", "Hierarchy", "Иерархия"),
        DataCompositionGroupTypeHierarchyOnly("ТолькоИерархия", "HierarchyOnly", "Только иерархия"),
        DataCompositionGroupTypeItems("Элементы", "Items", "Без иерархии"),
    }
    /// Расположение группировки компоновки данных.
    DataCompositionGroupPlacement(
        "РасположениеГруппировкиКомпоновкиДанных",
        "DataCompositionGroupPlacement"
    )
        meta "ПеречислениеРасположениеГруппировкиКомпоновкиДанных" {
        DataCompositionGroupPlacementEnd("Конец", "End", "Конец"),
        DataCompositionGroupPlacementBegin("Начало", "Begin", "Начало"),
        DataCompositionGroupPlacementBeginAndEnd("НачалоИКонец", "BeginAndEnd", "Начало и конец"),
        // Члены, представления и метатип измерены на 8.3.27.2342.
        DataCompositionGroupPlacementNone("Нет", "None", "Нет"),
    }
    /// Расположение полей группировки компоновки данных.
    DataCompositionGroupFieldsPlacement(
        "РасположениеПолейГруппировкиКомпоновкиДанных",
        "DataCompositionGroupFieldsPlacement"
    )
        meta "ПеречислениеРасположениеПолейГруппировкиКомпоновкиДанных" {
        // Представления и оба написания измерены на 8.3.27.
        DataCompositionGroupFieldsPlacementTogether("Вместе", "Together", "Вместе"),
        DataCompositionGroupFieldsPlacementSeparately("Отдельно", "Separately", "Отдельно"),
        DataCompositionGroupFieldsPlacementSeparatelyAndInTotalsOnly(
            "ОтдельноИТолькоВИтогах",
            "SeparatelyAndInTotalsOnly",
            "Отдельно и только в итогах"
        ),
    }
    /// Расположение реквизитов компоновки данных.
    DataCompositionAttributesPlacement(
        "РасположениеРеквизитовКомпоновкиДанных",
        "DataCompositionAttributesPlacement"
    )
        meta "ПеречислениеРасположениеРеквизитовКомпоновкиДанных" {
        // Представления и оба написания измерены на 8.3.27.
        DataCompositionAttributesPlacementTogether("Вместе", "Together", "Вместе"),
        DataCompositionAttributesPlacementWithOwnerField(
            "ВместеСВладельцем",
            "WithOwnerField",
            "Вместе с владельцем"
        ),
        DataCompositionAttributesPlacementSpecialPosition(
            "ВСпециальнойПозиции",
            "SpecialPosition",
            "В специальной позиции"
        ),
        DataCompositionAttributesPlacementSeparately("Отдельно", "Separately", "Отдельно"),
    }
    /// ВидДекорацииФормы.
    FormDecorationType("ВидДекорацииФормы", "FormDecorationType") meta "ПеречислениеВидДекорацииФормы" {
        // Имена и представления измерены на 8.3.27 в `measure-session-enum-batch.bsl`.
        FormDecorationTypePicture("Картинка", "Picture", "Картинка"),
        FormDecorationTypeLabel("Надпись", "Label", "Надпись"),
    }
    /// ВидПереключателя.
    RadioButtonType("ВидПереключателя", "RadioButtonType") meta "ПеречислениеВидПереключателя" {
        RadioButtonTypeAuto("Авто", "Auto", "Авто"),
        RadioButtonTypeRadioButton("Переключатель", "RadioButton", "Переключатель"),
        RadioButtonTypeTumbler("Тумблер", "Tumbler", "Тумблер"),
    }
    /// ГруппировкаПодчиненныхЭлементовФормы.
    ChildFormItemsGroup("ГруппировкаПодчиненныхЭлементовФормы", "ChildFormItemsGroup")
        meta "ПеречислениеГруппировкаПодчиненныхЭлементовФормы" {
        ChildFormItemsGroupVertical("Вертикальная", "Vertical", "Вертикальная"),
        ChildFormItemsGroupHorizontal("Горизонтальная", "Horizontal", "Горизонтальная"),
        ChildFormItemsGroupAlwaysHorizontal(
            "ГоризонтальнаяВсегда",
            "AlwaysHorizontal",
            "Горизонтальная всегда"
        ),
        ChildFormItemsGroupHorizontalIfPossible(
            "ГоризонтальнаяЕслиВозможно",
            "HorizontalIfPossible",
            "Горизонтальная если возможно"
        ),
    }
    /// ИсторияВыбораПриВводе.
    ChoiceHistoryOnInput("ИсторияВыбораПриВводе", "ChoiceHistoryOnInput")
        meta "ПеречислениеИсторияВыбораПриВводе" {
        ChoiceHistoryOnInputAuto("Авто", "Auto", "Авто"),
        ChoiceHistoryOnInputDontUse("НеИспользовать", "DontUse", "Не использовать"),
    }
    /// ОтображениеКнопкиВыбора.
    ChoiceButtonRepresentation("ОтображениеКнопкиВыбора", "ChoiceButtonRepresentation")
        meta "ПеречислениеОтображениеКнопкиВыбора" {
        ChoiceButtonRepresentationAuto("Авто", "Auto", "Авто"),
        ChoiceButtonRepresentationShowInDropList(
            "ОтображатьВВыпадающемСписке",
            "ShowInDropList",
            "Отображать в выпадающем списке"
        ),
        ChoiceButtonRepresentationShowInDropListAndInInputField(
            "ОтображатьВВыпадающемСпискеИВПолеВвода",
            "ShowInDropListAndInInputField",
            "Отображать в выпадающем списке и в поле ввода"
        ),
        ChoiceButtonRepresentationShowInInputField(
            "ОтображатьВПолеВвода",
            "ShowInInputField",
            "Отображать в поле ввода"
        ),
    }
    /// ОтображениеУправленияОбычнойГруппы.
    UsualGroupControlRepresentation(
        "ОтображениеУправленияОбычнойГруппы",
        "UsualGroupControlRepresentation"
    )
        meta "ПеречислениеОтображениеУправленияОбычнойГруппы" {
        UsualGroupControlRepresentationTitleHyperlink(
            "ГиперссылкаЗаголовка",
            "TitleHyperlink",
            "Гиперссылка заголовка"
        ),
        UsualGroupControlRepresentationPicture("Картинка", "Picture", "Картинка"),
    }
    /// ПоведениеОбычнойГруппы.
    UsualGroupBehavior("ПоведениеОбычнойГруппы", "UsualGroupBehavior")
        meta "ПеречислениеПоведениеОбычнойГруппы" {
        UsualGroupBehaviorAuto("Авто", "Auto", "Авто"),
        UsualGroupBehaviorPopUp("Всплывающая", "PopUp", "Всплывающая"),
        UsualGroupBehaviorUsual("Обычное", "Usual", "Обычное"),
        UsualGroupBehaviorCollapsible("Свертываемая", "Collapsible", "Свертываемая"),
    }
    /// ПоложениеЗаголовкаЭлементаФормы.
    FormItemTitleLocation("ПоложениеЗаголовкаЭлементаФормы", "FormItemTitleLocation")
        meta "ПеречислениеПоложениеЗаголовкаЭлементаФормы" {
        FormItemTitleLocationAuto("Авто", "Auto", "Авто"),
        FormItemTitleLocationTop("Верх", "Top", "Верх"),
        FormItemTitleLocationLeft("Лево", "Left", "Лево"),
        FormItemTitleLocationNone("Нет", "None", "Нет"),
        FormItemTitleLocationBottom("Низ", "Bottom", "Низ"),
        FormItemTitleLocationRight("Право", "Right", "Право"),
    }
    /// ПоложениеКнопкиВКоманднойПанели.
    ButtonLocationInCommandBar("ПоложениеКнопкиВКоманднойПанели", "ButtonLocationInCommandBar")
        meta "ПеречислениеПоложениеКнопкиВКоманднойПанели" {
        ButtonLocationInCommandBarAuto("Авто", "Auto", "Авто"),
        ButtonLocationInCommandBarInAdditionalSubmenu(
            "ВДополнительномПодменю",
            "InAdditionalSubmenu",
            "В дополнительном подменю"
        ),
        ButtonLocationInCommandBarInCommandBar(
            "ВКоманднойПанели",
            "InCommandBar",
            "В командной панели"
        ),
        ButtonLocationInCommandBarInCommandBarAndInAdditionalSubmenu(
            "ВКоманднойПанелиИВДополнительномПодменю",
            "InCommandBarAndInAdditionalSubmenu",
            "В командной панели и дополнительном подменю"
        ),
    }
    /// РасположениеРесурсовКомпоновкиДанных.
    DataCompositionResourcesPlacement(
        "РасположениеРесурсовКомпоновкиДанных",
        "DataCompositionResourcesPlacement"
    )
        meta "ПеречислениеРасположениеРесурсовКомпоновкиДанных" {
        DataCompositionResourcesPlacementVertically("Вертикально", "Vertically", "Вертикально"),
        DataCompositionResourcesPlacementHorizontally(
            "Горизонтально",
            "Horizontally",
            "Горизонтально"
        ),
    }
    /// ИспользованиеУсловногоОформленияКомпоновкиДанных.
    DataCompositionConditionalAppearanceUse(
        "ИспользованиеУсловногоОформленияКомпоновкиДанных",
        "DataCompositionConditionalAppearanceUse"
    )
        meta "ПеречислениеИспользованиеУсловногоОформленияКомпоновкиДанных" {
        // Представления подтверждены нативным контролем 8.3.27.2342. Замеры:
        // `tests/conformance/measure/1c/measure-data-composition-*.platform.txt`
        // для `conditional-appearance-use`, `field-placement`,
        // `accounting-balance-type`, `balance-type` и `period-type`.
        DataCompositionConditionalAppearanceUseUse("Использовать", "Use", "Использовать"),
        DataCompositionConditionalAppearanceUseDontUse("НеИспользовать", "DontUse", "Не использовать"),
    }
    /// РасположениеПоляКомпоновкиДанных.
    DataCompositionFieldPlacement(
        "РасположениеПоляКомпоновкиДанных",
        "DataCompositionFieldPlacement"
    )
        meta "ПеречислениеРасположениеПоляКомпоновкиДанных" {
        DataCompositionFieldPlacementAuto("Авто", "Auto", "Авто"),
        DataCompositionFieldPlacementVertically("Вертикально", "Vertically", "Вертикально"),
        DataCompositionFieldPlacementTogether("Вместе", "Together", "Вместе"),
        DataCompositionFieldPlacementHorizontally("Горизонтально", "Horizontally", "Горизонтально"),
        DataCompositionFieldPlacementSpecialColumn(
            "ОтдельнаяКолонка",
            "SpecialColumn",
            "В отдельной колонке"
        ),
    }
    /// ТипБухгалтерскогоОстаткаКомпоновкиДанных.
    DataCompositionAccountingBalanceType(
        "ТипБухгалтерскогоОстаткаКомпоновкиДанных",
        "DataCompositionAccountingBalanceType"
    )
        meta "ПеречислениеТипБухгалтерскогоОстаткаКомпоновкиДанных" {
        DataCompositionAccountingBalanceTypeDebit("Дебет", "Debit", "Дебет"),
        DataCompositionAccountingBalanceTypeCredit("Кредит", "Credit", "Кредит"),
        DataCompositionAccountingBalanceTypeNone("Нет", "None", "Нет"),
    }
    /// ТипОстаткаКомпоновкиДанных.
    DataCompositionBalanceType("ТипОстаткаКомпоновкиДанных", "DataCompositionBalanceType")
        meta "ПеречислениеТипОстаткаКомпоновкиДанных" {
        DataCompositionBalanceTypeOpeningBalance(
            "НачальныйОстаток",
            "OpeningBalance",
            "Начальный остаток"
        ),
        DataCompositionBalanceTypeClosingBalance(
            "КонечныйОстаток",
            "ClosingBalance",
            "Конечный остаток"
        ),
        DataCompositionBalanceTypeNone("Нет", "None", "Нет"),
    }
    /// ТипПериодаКомпоновкиДанных.
    DataCompositionPeriodType("ТипПериодаКомпоновкиДанных", "DataCompositionPeriodType")
        meta "ПеречислениеТипПериодаКомпоновкиДанных" {
        DataCompositionPeriodTypeMain("Основной", "Main", "Основной"),
        DataCompositionPeriodTypeAdditional("Дополнительный", "Additional", "Дополнительный"),
    }
    /// ТипДополненияПериодаКомпоновкиДанных.
    DataCompositionPeriodAdditionType(
        "ТипДополненияПериодаКомпоновкиДанных",
        "DataCompositionPeriodAdditionType"
    )
        meta "ПеречислениеТипДополненияПериодаКомпоновкиДанных" {
        DataCompositionPeriodAdditionTypeNone("БезДополнения", "None", "Без дополнения"),
        DataCompositionPeriodAdditionTypeYear("Год", "Year", "Год"),
        DataCompositionPeriodAdditionTypeYearSinceBeginOfPeriod(
            "ГодОтНачалаПериода",
            "YearSinceBeginOfPeriod",
            "Год с начала периода"
        ),
        DataCompositionPeriodAdditionTypeYearSinceBeginOfPeriod445(
            "ГодОтНачалаПериода445",
            "YearSinceBeginOfPeriod445",
            "Год с начала периода (445)"
        ),
        DataCompositionPeriodAdditionTypeTenDays("Декада", "TenDays", "Декада"),
        DataCompositionPeriodAdditionTypeDay("День", "Day", "День"),
        DataCompositionPeriodAdditionTypeDaySinceBeginOfPeriod(
            "ДеньОтНачалаПериода",
            "DaySinceBeginOfPeriod",
            "День с начала периода"
        ),
        DataCompositionPeriodAdditionTypeQuarter("Квартал", "Quarter", "Квартал"),
        DataCompositionPeriodAdditionTypeQuarterSinceBeginOfPeriod(
            "КварталОтНачалаПериода",
            "QuarterSinceBeginOfPeriod",
            "Квартал с начала периода"
        ),
        DataCompositionPeriodAdditionTypeQuarterSinceBeginOfPeriod445(
            "КварталОтНачалаПериода445",
            "QuarterSinceBeginOfPeriod445",
            "Квартал с начала периода (445)"
        ),
        DataCompositionPeriodAdditionTypeMonth("Месяц", "Month", "Месяц"),
        DataCompositionPeriodAdditionTypeMonthSinceBeginOfPeriod(
            "МесяцОтНачалаПериода",
            "MonthSinceBeginOfPeriod",
            "Месяц с начала периода"
        ),
        DataCompositionPeriodAdditionTypeMonthSinceBeginOfPeriod445(
            "МесяцОтНачалаПериода445",
            "MonthSinceBeginOfPeriod445",
            "Месяц с начала периода (445)"
        ),
        DataCompositionPeriodAdditionTypeMinute("Минута", "Minute", "Минута"),
        DataCompositionPeriodAdditionTypeMinuteSinceBeginOfPeriod(
            "МинутаОтНачалаПериода",
            "MinuteSinceBeginOfPeriod",
            "Минута с начала периода"
        ),
        DataCompositionPeriodAdditionTypeWeek("Неделя", "Week", "Неделя"),
        DataCompositionPeriodAdditionTypeWeekSinceBeginOfPeriod(
            "НеделяОтНачалаПериода",
            "WeekSinceBeginOfPeriod",
            "Неделя с начала периода"
        ),
        DataCompositionPeriodAdditionTypeHalfYear("Полугодие", "HalfYear", "Полугодие"),
        DataCompositionPeriodAdditionTypeHalfYearSinceBeginOfPeriod(
            "ПолугодиеОтНачалаПериода",
            "HalfYearSinceBeginOfPeriod",
            "Полугодие с начала периода"
        ),
        DataCompositionPeriodAdditionTypeHalfYearSinceBeginOfPeriod445(
            "ПолугодиеОтНачалаПериода445",
            "HalfYearSinceBeginOfPeriod445",
            "Полугодие с начала периода (445)"
        ),
        DataCompositionPeriodAdditionTypeSecond("Секунда", "Second", "Секунда"),
        DataCompositionPeriodAdditionTypeHour("Час", "Hour", "Час"),
        DataCompositionPeriodAdditionTypeHourSinceBeginOfPeriod(
            "ЧасОтНачалаПериода",
            "HourSinceBeginOfPeriod",
            "Час с начала периода"
        ),
    }
    /// ГлубинаЦвета.
    ColorDepth("ГлубинаЦвета", "ColorDepth") meta "ПеречислениеГлубинаЦвета" {
        // Имена и представления измерены на 8.3.27 в `measure-next-session-enums.bsl`.
        ColorDepthBitPerPixel1("БитНаПиксел1", "BitPerPixel1", "1 бит на пикель"),
        ColorDepthBitPerPixel24("БитНаПиксел24", "BitPerPixel24", "24 бита на пикель"),
        ColorDepthBitPerPixel32("БитНаПиксел32", "BitPerPixel32", "32 бита на пикель"),
        ColorDepthBitPerPixel4("БитНаПиксел4", "BitPerPixel4", "4 бита на пикель"),
        ColorDepthBitPerPixel8("БитНаПиксел8", "BitPerPixel8", "8 бит на пикель"),
    }
    /// ОтображениеПодсказки.
    ToolTipRepresentation("ОтображениеПодсказки", "ToolTipRepresentation")
        meta "ПеречислениеОтображениеПодсказки" {
        ToolTipRepresentationAuto("Авто", "Auto", "Авто"),
        ToolTipRepresentationBalloon("Всплывающая", "Balloon", "Всплывающая подсказка"),
        ToolTipRepresentationButton("Кнопка", "Button", "Кнопка"),
        ToolTipRepresentationNone("Нет", "None", "Нет"),
        ToolTipRepresentationShowAuto("ОтображатьАвто", "ShowAuto", "Отображать авто"),
        ToolTipRepresentationShowTop("ОтображатьСверху", "ShowTop", "Отображать сверху"),
        ToolTipRepresentationShowLeft("ОтображатьСлева", "ShowLeft", "Отображать слева"),
        ToolTipRepresentationShowBottom("ОтображатьСнизу", "ShowBottom", "Отображать снизу"),
        ToolTipRepresentationShowRight("ОтображатьСправа", "ShowRight", "Отображать справа"),
    }
    /// ТипКлиентскогоПриложения.
    ClientApplicationType("ТипКлиентскогоПриложения", "ClientApplicationType")
        meta "ПеречислениеТипКлиентскогоПриложения" {
        ClientApplicationTypeWebClient("ВебКлиент", "WebClient", "Веб клиент"),
        ClientApplicationTypeExternalConnection(
            "ВнешнееСоединение",
            "ExternalConnection",
            "Внешнее соединение"
        ),
        ClientApplicationTypeMobileAppClient(
            "МобильноеПриложениеКлиент",
            "MobileAppClient",
            "Мобильное приложение клиент"
        ),
        ClientApplicationTypeMobileClient("МобильныйКлиент", "MobileClient", "Мобильный клиент"),
        ClientApplicationTypeThickClient("ТолстыйКлиент", "ThickClient", "Толстый клиент"),
        ClientApplicationTypeThinClient("ТонкийКлиент", "ThinClient", "Тонкий клиент"),
    }
    /// Тип вывода текста компоновки данных.
    DataCompositionTextOutputType(
        "ТипВыводаТекстаКомпоновкиДанных",
        "DataCompositionTextOutputType"
    )
        meta "ПеречислениеТипВыводаТекстаКомпоновкиДанных" {
        // Полный состав и представления измерены на платформе 8.3.27.2342
        // пробой `measure-data-composition-text-output-type.bsl`.
        DataCompositionTextOutputTypeAuto("Авто", "Auto", "Авто"),
        DataCompositionTextOutputTypeOutput("Выводить", "Output", "Выводить"),
        DataCompositionTextOutputTypeDontOutput("НеВыводить", "DontOutput", "Не выводить"),
    }
    /// Тип размещения текста компоновки данных.
    // ИЗМЕРЕНО на 8.3.27 пробой
    // `measure-data-composition-text-output-type.bsl`.
    DataCompositionTextPlacementType(
        "ТипРазмещенияТекстаКомпоновкиДанных",
        "DataCompositionTextPlacementType"
    )
        meta "ПеречислениеТипРазмещенияТекстаКомпоновкиДанных" {
        DataCompositionTextPlacementTypeOverflow("Выступать", "Overflow", "Выступать"),
        // Типы, члены, алиасы и представления измерены на 8.3.27.2342.
        DataCompositionTextPlacementTypeBlock("Забивать", "Block", "Забивать"),
        DataCompositionTextPlacementTypeCut("Обрезать", "Cut", "Обрезать"),
        DataCompositionTextPlacementTypeWrap("Переносить", "Wrap", "Переносить"),
    }
    /// Режим масштабирования просмотра документа.
    // ИЗМЕРЕНО на 8.3.27 пробой `measure-view-scaling-mode.bsl`.
    ViewScalingMode("РежимМасштабированияПросмотра", "ViewScalingMode") meta "ПеречислениеРежимМасштабированияПросмотра" {
        // Полный состав и представления измерены на платформе 8.3.27.2342
        // пробой `measure-view-scaling-mode.bsl`.
        ViewScalingModeAuto("Авто", "Auto", "Авто"),
        ViewScalingModeLarge("Крупный", "Large", "Крупный"),
        ViewScalingModeNormal("Обычный", "Normal", "Обычный"),
    }
    /// Расположение итогов компоновки данных.
    // ИЗМЕРЕНО на 8.3.27 пробой
    // `measure-data-composition-total-placement.bsl`.
    DataCompositionTotalPlacement(
        "РасположениеИтоговКомпоновкиДанных",
        "DataCompositionTotalPlacement"
    )
        meta "ПеречислениеРасположениеИтоговКомпоновкиДанных" {
        // Полный состав и представления измерены на платформе 8.3.27.2342
        // пробой `measure-data-composition-total-placement.bsl`.
        DataCompositionTotalPlacementAuto("Авто", "Auto", "Авто"),
        DataCompositionTotalPlacementEnd("Конец", "End", "Конец"),
        DataCompositionTotalPlacementBegin("Начало", "Begin", "Начало"),
        DataCompositionTotalPlacementBeginAndEnd("НачалоИКонец", "BeginAndEnd", "Начало и конец"),
        DataCompositionTotalPlacementNone("Нет", "None", "Нет"),
    }
    /// Размер картинки в области отображения.
    // ИЗМЕРЕНО на 8.3.27 пробой `measure-picture-size.bsl`.
    PictureSize("РазмерКартинки", "PictureSize") meta "ПеречислениеРазмерКартинки" {
        // Полный состав и представления измерены на платформе 8.3.27.2342
        // пробой `measure-picture-size.bsl`.
        PictureSizeAutoSize("АвтоРазмер", "AutoSize", "Автоматический размер"),
        PictureSizeAutoSizeIgnoreScale(
            "АвтоРазмерБезУчетаМасштаба",
            "AutoSizeIgnoreScale",
            "Автоматический размер без учета масштаба"
        ),
        PictureSizeByFontSize("ПоРазмеруШрифта", "ByFontSize", "По размеру шрифта"),
        PictureSizeProportionally("Пропорционально", "Proportionally", "Пропорционально"),
        PictureSizeStretch("Растянуть", "Stretch", "Растянуть"),
        PictureSizeRealSize("РеальныйРазмер", "RealSize", "Реальный размер"),
        PictureSizeRealSizeIgnoreScale(
            "РеальныйРазмерБезУчетаМасштаба",
            "RealSizeIgnoreScale",
            "Реальный размер без учета масштаба"
        ),
        PictureSizeTile("Черепица", "Tile", "Черепица"),
    }
    /// Тип смещения ячеек табличного документа.
    // ИЗМЕРЕНО на 8.3.27 пробой
    // `measure-spreadsheet-document-shift-type.bsl`.
    SpreadsheetDocumentShiftType("ТипСмещенияТабличногоДокумента", "SpreadsheetDocumentShiftType")
        meta "ПеречислениеТипСмещенияТабличногоДокумента" {
        // Полный состав и представления измерены на платформе 8.3.27.2342
        // пробой `measure-spreadsheet-document-shift-type.bsl`.
        SpreadsheetDocumentShiftTypeWithoutShift("БезСмещения", "WithoutShift", "Без смещения"),
        SpreadsheetDocumentShiftTypeVertical("ПоВертикали", "Vertical", "По вертикали"),
        SpreadsheetDocumentShiftTypeHorizontal("ПоГоризонтали", "Horizontal", "По горизонтали"),
    }
    /// Тип диаграммы.
    // ИЗМЕРЕНО на 8.3.27 пробой `measure-chart-type.bsl`.
    ChartType("ТипДиаграммы", "ChartType") meta "ПеречислениеТипДиаграммы" {
        // Полный состав и представления измерены на платформе 8.3.27.2342
        // пробой `measure-chart-type.bsl`.
        ChartTypeStock("Биржевая", "Stock", "Биржевая"),
        ChartTypeOpenHighLowClose("БиржеваяСвеча", "OpenHighLowClose", "Биржевая \"свеча\""),
        ChartTypeConcaveSurface("ВогнутаяПоверхность", "ConcaveSurface", "Вогнутая поверхность"),
        ChartTypeWaterfall("Водопад", "Waterfall", "Водопад"),
        ChartTypeFunnel("Воронка", "Funnel", "Воронка"),
        ChartTypeNormalizedFunnel("ВоронкаНормированная", "NormalizedFunnel", "Воронка нормированная"),
        ChartTypeNormalizedFunnel3D(
            "ВоронкаНормированнаяОбъемная",
            "NormalizedFunnel3D",
            "Воронка нормированная объемная"
        ),
        ChartTypeFunnel3D("ВоронкаОбъемная", "Funnel3D", "Воронка объемная"),
        ChartTypeConvexSurface("ВыпуклаяПоверхность", "ConvexSurface", "Выпуклая поверхность"),
        ChartTypeColumn("Гистограмма", "Column", "Гистограмма"),
        ChartTypeBar("ГистограммаГоризонтальная", "Bar", "Гистограмма горизонтальная"),
        ChartTypeBar3D("ГистограммаГоризонтальнаяОбъемная", "Bar3D", "Гистограмма горизонтальная объемная"),
        ChartTypeNormalizedColumn(
            "ГистограммаНормированная",
            "NormalizedColumn",
            "Гистограмма нормированная"
        ),
        ChartTypeNormalizedBar(
            "ГистограммаНормированнаяГоризонтальная",
            "NormalizedBar",
            "Гистограмма нормированная горизонтальная"
        ),
        ChartTypeNormalizedBar3D(
            "ГистограммаНормированнаяГоризонтальнаяОбъемная",
            "NormalizedBar3D",
            "Гистограмма нормированная горизонтальная объемная"
        ),
        ChartTypeNormalizedColumn3D(
            "ГистограммаНормированнаяОбъемная",
            "NormalizedColumn3D",
            "Гистограмма нормированная объемная"
        ),
        ChartTypeColumn3D("ГистограммаОбъемная", "Column3D", "Гистограмма объемная"),
        ChartTypeStackedColumn("ГистограммаСНакоплением", "StackedColumn", "Гистограмма с накоплением"),
        ChartTypeStackedBar("ГистограммаСНакоплениемГоризонтальная", "StackedBar", "Гистограмма с накоплением горизонтальная"),
        ChartTypeStackedBar3D(
            "ГистограммаСНакоплениемГоризонтальнаяОбъемная",
            "StackedBar3D",
            "Гистограмма с накоплением горизонтальная объемная "
        ),
        ChartTypeStackedColumn3D(
            "ГистограммаСНакоплениемОбъемная",
            "StackedColumn3D",
            "Гистограмма с накоплением объемная"
        ),
        ChartTypeLine("График", "Line", "График"),
        ChartTypeStep("ГрафикПоШагам", "Step", "График по шагам"),
        ChartTypeStackedLine("ГрафикСНакоплением", "StackedLine", "График с накоплением"),
        ChartTypeArea("ГрафикСОбластями", "Area", "График с областями"),
        ChartTypeStackedArea("ГрафикСОбластямиИНакоплением", "StackedArea", "График с областями и накоплением"),
        ChartTypeNormalizedArea(
            "ГрафикСОбластямиНормированный",
            "NormalizedArea",
            "График с областями нормированный"
        ),
        ChartTypeShadedSurface("ЗатененнаяПоверхность", "ShadedSurface", "Затененная поверхность"),
        ChartTypeGauge("Измерительная", "Gauge", "Измерительная"),
        ChartTypeBarGraph("Изометрическая", "BarGraph", "Изометрическая обычная"),
        ChartTypeTapeGraph("ИзометрическаяЛента", "TapeGraph", "Изометрическая лента"),
        ChartTypeCeilGraph("ИзометрическаяНепрерывная", "CeilGraph", "Изометрическая непрерывная"),
        ChartTypePyramidGraph("ИзометрическаяПирамида", "PyramidGraph", "Изометрическая пирамида"),
        ChartTypeWireframeSurface("КаркаснаяПоверхность", "WireframeSurface", "Каркасная поверхность"),
        ChartTypeDonut("Кольцевая", "Donut", "Кольцевая"),
        ChartTypeDonut3D("КольцеваяОбъемная", "Donut3D", "Кольцевая объемная"),
        ChartTypePie("Круговая", "Pie", "Круговая"),
        ChartTypePie3D("КруговаяОбъемная", "Pie3D", "Круговая объемная"),
        ChartTypeSurface("Поверхность", "Surface", "Поверхность"),
        ChartTypeBubble("Пузырьковая", "Bubble", "Пузырьковая"),
        ChartTypeRadarLine("РадарныйГрафик", "RadarLine", "Радарный график"),
        ChartTypeRadarStackedLine(
            "РадарныйГрафикСНакоплением",
            "RadarStackedLine",
            "Радарный график с накоплением"
        ),
        ChartTypeRadarArea("РадарныйГрафикСОбластями", "RadarArea", "Радарный график с областями"),
        ChartTypeRadarStackedArea(
            "РадарныйГрафикСОбластямиИНакоплением",
            "RadarStackedArea",
            "Радарный график с областями и накоплением"
        ),
        ChartTypeRadarNormalizedArea(
            "РадарныйГрафикСОбластямиНормированный",
            "RadarNormalizedArea",
            "Радарный график с областями нормированный"
        ),
        ChartTypeHoneycomb("Сотовая", "Honeycomb", "Сотовая"),
        ChartTypeScatter("Точечная", "Scatter", "Точечная"),
    }
    /// Ориентация подписей диаграммы.
    // ИЗМЕРЕНО на 8.3.27 пробой
    // `measure-chart-label-orientation-name.bsl`.
    ChartLabelsOrientation("ОриентацияПодписейДиаграммы", "ChartLabelsOrientation")
        meta "ПеречислениеОриентацияПодписейДиаграммы"
        global ["ОриентацияПодписейДиаграммы", "ОриентацияМетокДиаграммы", "ChartLabelsOrientation"] {
        // Оба русских имени и представления измерены на платформе 8.3.27.2342
        // пробой `measure-chart-label-orientation-name.bsl`.
        ChartLabelsOrientationAuto("Авто", "Auto", "Авто"),
        ChartLabelsOrientationVertical("Вертикально", "Vertical", "Вертикально"),
        ChartLabelsOrientationHorizontal("Горизонтально", "Horizontal", "Горизонтально"),
        ChartLabelsOrientationCustomAngle("ПроизвольныйУголНаклона", "CustomAngle", "Произвольный угол"),
    }
    /// Вид включения граничного значения.
    // ИЗМЕРЕНО на 8.3.27 пробой `measure-boundary-type.bsl`.
    BoundaryType("ВидГраницы", "BoundaryType") meta "ПеречислениеВидГраницы" {
        // Полный состав и представления измерены на платформе 8.3.27.2342
        // пробой `measure-boundary-type.bsl`.
        BoundaryTypeIncluding("Включая", "Including", "Включая"),
        BoundaryTypeExcluding("Исключая", "Excluding", "Исключая"),
    }
    /// Режим ограничения количества серий диаграммы.
    // ИЗМЕРЕНО на 8.3.27 пробой `measure-max-series.bsl`.
    MaxSeries("МаксимумСерий", "MaxSeries") meta "ПеречислениеМаксимумСерий" {
        // Полный состав и представления измерены на платформе 8.3.27.2342
        // пробой `measure-max-series.bsl`.
        MaxSeriesNotDefined("НеЗадано", "NotDefined", "Не задано"),
        MaxSeriesLimited("Ограничено", "Limited", "Ограничено"),
        MaxSeriesPercent("Процент", "Percent", "Процент"),
    }
    /// Вид подписи к области диаграммы.
    // ИЗМЕРЕНО на 8.3.27 пробой `measure-chart-label-type.bsl`.
    ChartLabelType("ВидПодписейКДиаграмме", "ChartLabelType") meta "ПеречислениеВидПодписейКДиаграмме" {
        // Полный состав и представления измерены на платформе 8.3.27.2342
        // пробой `measure-chart-label-type.bsl`.
        ChartLabelTypeValue("Значение", "Value", "Значение"),
        ChartLabelTypeValuePercent("ЗначениеПроцент", "ValuePercent", "Значение + процент"),
        ChartLabelTypeValueSize("ЗначениеРазмер", "ValueSize", "Значение + размер"),
        ChartLabelTypeNone("Нет", "None", "Нет"),
        ChartLabelTypePercent("Процент", "Percent", "Процент"),
        ChartLabelTypeSeries("Серия", "Series", "Серия"),
        ChartLabelTypeSeriesValue("СерияЗначение", "SeriesValue", "Серия + значение"),
        ChartLabelTypeSeriesValuePercent(
            "СерияЗначениеПроцент",
            "SeriesValuePercent",
            "Серия + значение + процент"
        ),
        ChartLabelTypeSeriesValueSize(
            "СерияЗначениеРазмер",
            "SeriesValueSize",
            "Cерия + значение + размер"
        ),
        ChartLabelTypeSeriesPercent("СерияПроцент", "SeriesPercent", "Серия + процент"),
        ChartLabelTypeSeriesSize("СерияРазмер", "SeriesSize", "Cерия + размер"),
        ChartLabelTypeSeriesPoint("СерияТочка", "SeriesPoint", "Серия + точка"),
        ChartLabelTypeSeriesPointValue(
            "СерияТочкаЗначение",
            "SeriesPointValue",
            "Серия + точка + значение"
        ),
        ChartLabelTypeSeriesPointValuePercent(
            "СерияТочкаЗначениеПроцент",
            "SeriesPointValuePercent",
            "Серия + точка + значение + процент"
        ),
        ChartLabelTypeSeriesPointValueSize(
            "СерияТочкаЗначениеРазмер",
            "SeriesPointValueSize",
            "Серия + точка + значение + размер"
        ),
        ChartLabelTypeSeriesPointPercent(
            "СерияТочкаПроцент",
            "SeriesPointPercent",
            "Серия + точка + процент"
        ),
        ChartLabelTypeSeriesPointSize(
            "СерияТочкаРазмер",
            "SeriesPointSize",
            "Серия + точка + размер"
        ),
        ChartLabelTypePoint("Точка", "Point", "Точка"),
        ChartLabelTypePointValue("ТочкаЗначение", "PointValue", "Точка + значение"),
        ChartLabelTypePointValuePercent(
            "ТочкаЗначениеПроцент",
            "PointValuePercent",
            "Точка + значение + процент"
        ),
        ChartLabelTypePointValueSize(
            "ТочкаЗначениеРазмер",
            "PointValueSize",
            "Точка + значение + размер"
        ),
        ChartLabelTypePointPercent("ТочкаПроцент", "PointPercent", "Точка + процент"),
        ChartLabelTypePointSize("ТочкаРазмер", "PointSize", "Точка + размер"),
    }
    /// Стиль линии диаграммы.
    // ИЗМЕРЕНО на 8.3.27 пробой `measure-chart-line-type.bsl`.
    ChartLineType("ТипЛинииДиаграммы", "ChartLineType") meta "ПеречислениеТипЛинииДиаграммы" {
        // Полный состав и представления измерены на платформе 8.3.27.2342
        // пробой `measure-chart-line-type.bsl`.
        ChartLineTypeNone("НетЛинии", "None", "Нет линии"),
        ChartLineTypeDashed("Пунктир", "Dashed", "Пунктирная"),
        ChartLineTypeDashDotted("ПунктирТочка", "DashDotted", "Пунктирно-точечная"),
        ChartLineTypeDashDottedDotted("ПунктирТочкаТочка", "DashDottedDotted", "Точечно-пунктирно-точечная"),
        ChartLineTypeSolid("Сплошная", "Solid", "Сплошная"),
        ChartLineTypeDotted("Точечная", "Dotted", "Точечная"),
    }
    /// Вид кнопки формы.
    // ИЗМЕРЕНО на 8.3.27 пробой `measure-form-button-type.bsl`.
    FormButtonType("ВидКнопкиФормы", "FormButtonType") meta "ПеречислениеВидКнопкиФормы" {
        // Полный состав и представления измерены на платформе 8.3.27.2342
        // пробой `measure-form-button-type.bsl`.
        FormButtonTypeHyperlink("Гиперссылка", "Hyperlink", "Гиперссылка"),
        FormButtonTypeCommandBarHyperlink(
            "ГиперссылкаКоманднойПанели",
            "CommandBarHyperlink",
            "Гиперссылка командной панели"
        ),
        FormButtonTypeCommandBarButton(
            "КнопкаКоманднойПанели",
            "CommandBarButton",
            "Кнопка командной панели"
        ),
        FormButtonTypeUsualButton("ОбычнаяКнопка", "UsualButton", "Обычная кнопка"),
    }
    /// Способ назначения типа при записи значения в XML.
    XmlTypeAssignment("НазначениеТипаXML", "XMLTypeAssignment") meta "ПеречислениеНазначениеТипаXML" {
        // Полный состав и представления измерены на платформе 8.3.27.2074
        // пробой `measure-xml-type-assignment.bsl`.
        XmlTypeAssignmentImplicit("Неявное", "Implicit", "Неявное"),
        XmlTypeAssignmentExplicit("Явное", "Explicit", "Явное"),
    }
    /// Тип операционной системы и архитектуры платформы.
    PlatformType("ТипПлатформы", "PlatformType") meta "ПеречислениеТипПлатформы" {
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
        meta "ПеречислениеВидСравненияКомпоновкиДанных" {
        // Состав, оба написания и представления измерены на платформе 8.3.27.2074
        // пробой `measure-data-composition-comparison-type.bsl`.
        DataCompositionComparisonBeginsWith("НачинаетсяС", "BeginsWith", "Начинается с"),
        DataCompositionComparisonContains("Содержит", "Contains", "Содержит"),
        DataCompositionComparisonEqual("Равно", "Equal", "Равно"),
        DataCompositionComparisonFilled("Заполнено", "Filled", "Заполнено"),
        DataCompositionComparisonGreater("Больше", "Greater", "Больше"),
        DataCompositionComparisonGreaterOrEqual(
            "БольшеИлиРавно",
            "GreaterOrEqual",
            "Больше или равно"
        ),
        DataCompositionComparisonInHierarchy("ВИерархии", "InHierarchy", "В группе"),
        DataCompositionComparisonInList("ВСписке", "InList", "В списке"),
        DataCompositionComparisonInListByHierarchy(
            "ВСпискеПоИерархии",
            "InListByHierarchy",
            "В группе из списка"
        ),
        DataCompositionComparisonLess("Меньше", "Less", "Меньше"),
        DataCompositionComparisonLessOrEqual(
            "МеньшеИлиРавно",
            "LessOrEqual",
            "Меньше или равно"
        ),
        DataCompositionComparisonLike("Подобно", "Like", "Соответствует шаблону"),
        DataCompositionComparisonNotBeginsWith(
            "НеНачинаетсяС",
            "NotBeginsWith",
            "Не начинается с"
        ),
        DataCompositionComparisonNotContains("НеСодержит", "NotContains", "Не содержит"),
        DataCompositionComparisonNotEqual("НеРавно", "NotEqual", "Не равно"),
        DataCompositionComparisonNotFilled("НеЗаполнено", "NotFilled", "Не заполнено"),
        DataCompositionComparisonNotInHierarchy("НеВИерархии", "NotInHierarchy", "Не в группе"),
        DataCompositionComparisonNotInList("НеВСписке", "NotInList", "Не в списке"),
        DataCompositionComparisonNotInListByHierarchy(
            "НеВСпискеПоИерархии",
            "NotInListByHierarchy",
            "Не в группе из списка"
        ),
        DataCompositionComparisonNotLike("НеПодобно", "NotLike", "Не соответствует шаблону"),
    }
    /// Операция сравнения в обычном отборе.
    ComparisonType("ВидСравнения", "ComparisonType") meta "ПеречислениеВидСравнения" {
        // Состав, оба написания и представления измерены на платформе 8.3.27
        // пробой `measure-comparison-type.bsl`.
        ComparisonGreater("Больше", "Greater", "Больше"),
        ComparisonGreaterOrEqual("БольшеИлиРавно", "GreaterOrEqual", "Больше или равно"),
        ComparisonInHierarchy("ВИерархии", "InHierarchy", "В группе"),
        ComparisonInList("ВСписке", "InList", "В списке"),
        ComparisonInListByHierarchy("ВСпискеПоИерархии", "InListByHierarchy", "В группе из списка"),
        ComparisonInterval("Интервал", "Interval", "Интервал (>, <)"),
        ComparisonIntervalIncludingBounds(
            "ИнтервалВключаяГраницы",
            "IntervalIncludingBounds",
            "Интервал (>=, <=)"
        ),
        ComparisonIntervalIncludingLowerBound(
            "ИнтервалВключаяНачало",
            "IntervalIncludingLowerBound",
            "Интервал (>=, <)"
        ),
        ComparisonIntervalIncludingUpperBound(
            "ИнтервалВключаяОкончание",
            "IntervalIncludingUpperBound",
            "Интервал (>, <=)"
        ),
        ComparisonLess("Меньше", "Less", "Меньше"),
        ComparisonLessOrEqual("МеньшеИлиРавно", "LessOrEqual", "Меньше или равно"),
        ComparisonNotInHierarchy("НеВИерархии", "NotInHierarchy", "Не в группе"),
        ComparisonNotInList("НеВСписке", "NotInList", "Не в списке"),
        ComparisonNotInListByHierarchy(
            "НеВСпискеПоИерархии",
            "NotInListByHierarchy",
            "Не в группе из списка"
        ),
        ComparisonNotEqual("НеРавно", "NotEqual", "Не равно"),
        ComparisonNotContains("НеСодержит", "NotContains", "Не содержит"),
        ComparisonEqual("Равно", "Equal", "Равно"),
        ComparisonContains("Содержит", "Contains", "Содержит"),
    }
    /// Относительный период либо произвольная стандартная дата начала.
    StandardBeginningDateVariant("ВариантСтандартнойДатыНачала", "StandardBeginningDateVariant")
        meta "ПеречислениеВариантСтандартнойДатыНачала" {
        // Состав, оба написания и представления измерены на платформе 8.3.27.2074
        // пробой `measure-standard-beginning-date.bsl`.
        StandardBeginningDateThisDay(
            "НачалоЭтогоДня",
            "BeginningOfThisDay",
            "Начало этого дня"
        ),
        StandardBeginningDateThisWeek(
            "НачалоЭтойНедели",
            "BeginningOfThisWeek",
            "Начало этой недели"
        ),
        StandardBeginningDateThisTenDays(
            "НачалоЭтойДекады",
            "BeginningOfThisTenDays",
            "Начало этой декады"
        ),
        StandardBeginningDateThisMonth(
            "НачалоЭтогоМесяца",
            "BeginningOfThisMonth",
            "Начало этого месяца"
        ),
        StandardBeginningDateThisQuarter(
            "НачалоЭтогоКвартала",
            "BeginningOfThisQuarter",
            "Начало этого квартала"
        ),
        StandardBeginningDateThisHalfYear(
            "НачалоЭтогоПолугодия",
            "BeginningOfThisHalfYear",
            "Начало этого полугодия"
        ),
        StandardBeginningDateThisYear(
            "НачалоЭтогоГода",
            "BeginningOfThisYear",
            "Начало этого года"
        ),
        StandardBeginningDateLastDay(
            "НачалоПрошлогоДня",
            "BeginningOfLastDay",
            "Начало вчерашнего дня"
        ),
        StandardBeginningDateLastWeek(
            "НачалоПрошлойНедели",
            "BeginningOfLastWeek",
            "Начало прошлой недели"
        ),
        StandardBeginningDateLastTenDays(
            "НачалоПрошлойДекады",
            "BeginningOfLastTenDays",
            "Начало прошлой декады"
        ),
        StandardBeginningDateLastMonth(
            "НачалоПрошлогоМесяца",
            "BeginningOfLastMonth",
            "Начало прошлого месяца"
        ),
        StandardBeginningDateLastQuarter(
            "НачалоПрошлогоКвартала",
            "BeginningOfLastQuarter",
            "Начало прошлого квартала"
        ),
        StandardBeginningDateLastHalfYear(
            "НачалоПрошлогоПолугодия",
            "BeginningOfLastHalfYear",
            "Начало прошлого полугодия"
        ),
        StandardBeginningDateLastYear(
            "НачалоПрошлогоГода",
            "BeginningOfLastYear",
            "Начало прошлого года"
        ),
        StandardBeginningDateNextDay(
            "НачалоСледующегоДня",
            "BeginningOfNextDay",
            "Начало завтрашнего дня"
        ),
        StandardBeginningDateNextWeek(
            "НачалоСледующейНедели",
            "BeginningOfNextWeek",
            "Начало следующей недели"
        ),
        StandardBeginningDateNextTenDays(
            "НачалоСледующейДекады",
            "BeginningOfNextTenDays",
            "Начало следующей декады"
        ),
        StandardBeginningDateNextMonth(
            "НачалоСледующегоМесяца",
            "BeginningOfNextMonth",
            "Начало следующего месяца"
        ),
        StandardBeginningDateNextQuarter(
            "НачалоСледующегоКвартала",
            "BeginningOfNextQuarter",
            "Начало следующего квартала"
        ),
        StandardBeginningDateNextHalfYear(
            "НачалоСледующегоПолугодия",
            "BeginningOfNextHalfYear",
            "Начало следующего полугодия"
        ),
        StandardBeginningDateNextYear(
            "НачалоСледующегоГода",
            "BeginningOfNextYear",
            "Начало следующего года"
        ),
        StandardBeginningDateCustom("ПроизвольнаяДата", "Custom", "Произвольная дата"),
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
        JsonEscapeNonAscii("СимволыВнеASCII", "CharactersOutsideASCII", "СимволыВнеASCII"),
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
        BackgroundJobActive("Активно", "Active", "Задание выполняется"),
        BackgroundJobCompleted("Завершено", "Completed", "Задание выполнено"),
        BackgroundJobFailed("ЗавершеноАварийно", "Failed", "Задание завершено с ошибками"),
        BackgroundJobCanceled("Отменено", "Canceled", "Задание отменено пользователем"),
    }
    /// `СтатусСообщения` — статусы `Сообщить`; состав членов — по выписке
    /// синтакс-помощника 8.3.27. В управляемом режиме статус игнорируется,
    /// перечисление нужно компиляции вызовов.
    MessageStatus("СтатусСообщения", "MessageStatus") meta "ПеречислениеСтатусСообщения" {
        MessageStatusWithoutStatus("БезСтатуса", "WithoutStatus", "Без cтатуса"),
        MessageStatusImportant("Важное", "Important", "Важное"),
        MessageStatusAttention("Внимание", "Attention", "Внимание"),
        MessageStatusInformation("Информация", "Information", "Информация"),
        MessageStatusOrdinary("Обычное", "Ordinary", "Обычное"),
        MessageStatusVeryImportant("ОченьВажное", "VeryImportant", "Очень важное"),
    }
    /// `КатегорияОшибки` — категории `ЯвляетсяОшибкойКатегории`; состав —
    /// по выписке синтакс-помощника 8.3.27.
    ErrorCategory("КатегорияОшибки", "ErrorCategory") meta "ПеречислениеКатегорияОшибки" {
        ErrorCategoryAllErrors("ВсеОшибки", "AllErrors", "Все ошибки"),
        ErrorCategoryExceptionRaisedFromScript(
            "ИсключениеВызванноеИзВстроенногоЯзыка",
            "ExceptionRaisedFromScript",
            "Исключение, вызванное из встроенного языка"
        ),
        ErrorCategoryAccessViolation(
            "НарушениеПравДоступа",
            "AccessViolation",
            "Нарушение прав доступа"
        ),
        ErrorCategoryUnsupportedFormat(
            "НеподдерживаемыйФормат",
            "UnsupportedFormat",
            "Неподдерживаемый формат"
        ),
        ErrorCategoryInvalidPassword("НеправильныйПароль", "InvalidPassword", "Неправильный пароль"),
        ErrorCategoryNoPermissionToUseFunctionality(
            "ОтсутствиеРазрешенияДляИспользованияФункциональности",
            "NoPermissionToUseFunctionality",
            "Отсутствие разрешения для использования функциональности"
        ),
        ErrorCategoryExternalDataSourceError(
            "ОшибкаВнешнегоИсточникаДанных",
            "ExternalDataSourceError",
            "Ошибка внешнего источника данных"
        ),
        ErrorCategoryScriptRuntimeError(
            "ОшибкаВоВремяВыполненияВстроенногоЯзыка",
            "ScriptRuntimeError",
            "Ошибка во время выполнения встроенного языка"
        ),
        ErrorCategoryLocalFileAccessError(
            "ОшибкаДоступаКЛокальномуФайлу",
            "LocalFileAccessError",
            "Ошибка доступа к локальному файлу"
        ),
        ErrorCategoryScriptUseError(
            "ОшибкаИспользованияВстроенногоЯзыка",
            "ScriptUseError",
            "Ошибка использования встроенного языка"
        ),
        ErrorCategoryScriptCompileError(
            "ОшибкаКомпиляцииВстроенногоЯзыка",
            "ScriptCompileError",
            "Ошибка компиляции встроенного языка"
        ),
        ErrorCategoryConfigurationError(
            "ОшибкаКонфигурации",
            "ConfigurationError",
            "Ошибка конфигурации"
        ),
        ErrorCategoryDatabaseCopyError(
            "ОшибкаКопииБазыДанных",
            "DatabaseCopyError",
            "Ошибка копии базы данных"
        ),
        ErrorCategoryDataCompositionSettingsError(
            "ОшибкаНастроекКомпоновкиДанных",
            "DataCompositionSettingsError",
            "Ошибка настроек компоновки данных"
        ),
        ErrorCategoryGotoURLError(
            "ОшибкаПереходаПоНавигационнойСсылке",
            "GotoURLError",
            "Ошибка перехода по навигационной ссылке"
        ),
        ErrorCategoryFullTextSearchError(
            "ОшибкаПолнотекстовогоПоиска",
            "FullTextSearchError",
            "Ошибка полнотекстового поиска"
        ),
        ErrorCategoryDocumentConversionError(
            "ОшибкаПреобразованияДокумента",
            "DocumentConversionError",
            "Ошибка преобразования документа"
        ),
        ErrorCategorySignatureVerificationError(
            "ОшибкаПроверкиПодписи",
            "SignatureVerificationError",
            "Ошибка проверки подписи"
        ),
        ErrorCategoryPrinterError(
            "ОшибкаРаботыСПринтером",
            "PrinterError",
            "Ошибка работы с принтером"
        ),
        ErrorCategorySpeechProcessingError(
            "ОшибкаРаботыСРечью",
            "SpeechProcessingError",
            "Ошибка системы работы с речью"
        ),
        ErrorCategorySessionError("ОшибкаСеанса", "SessionError", "Ошибка сеанса"),
        ErrorCategoryNetworkError("ОшибкаСети", "NetworkError", "Ошибка сети"),
        ErrorCategoryCollaborationSystemError(
            "ОшибкаСистемыВзаимодействия",
            "CollaborationSystemError",
            "Ошибка системы взаимодействия"
        ),
        ErrorCategoryMultimediaToolsError(
            "ОшибкаСредствМультимедиа",
            "MultimediaToolsError",
            "Ошибка cредств мультимедиа"
        ),
        ErrorCategoryDatabaseTablespaceError(
            "ОшибкаТабличногоПространстваБазыДанных",
            "DatabaseTablespaceError",
            "Ошибка табличного пространства базы данных"
        ),
        ErrorCategoryStoredDataError(
            "ОшибкаХранимыхДанных",
            "StoredDataError",
            "Ошибка хранимых данных"
        ),
        ErrorCategoryForcedShutdown(
            "ПринудительноеЗавершениеРаботы",
            "ForcedShutdown",
            "Принудительное завершение работы"
        ),
        ErrorCategoryOtherError("ПрочаяОшибка", "OtherError", "Прочая ошибка"),
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
            "Географическая схема"
        ),
        DrawingGroup("Группа", "Group", "Группа"),
        DrawingDendrogram("Дендрограмма", "Dendrogram", "Дендрограмма"),
        DrawingChart("Диаграмма", "Chart", "Диаграмма"),
        DrawingGanttChart("ДиаграммаГанта", "GanttChart", "Диаграмма Ганта"),
        DrawingPicture("Картинка", "Picture", "Картинка"),
        DrawingObject("Объект", "Object", "Объект"),
        DrawingComment("Примечание", "Comment", "Примечание"),
        DrawingLine("Прямая", "Line", "Прямая"),
        DrawingRectangle("Прямоугольник", "Rectangle", "Прямоугольник"),
        DrawingPivotChart("СводнаяДиаграмма", "PivotChart", "Сводная диаграмма"),
        DrawingText("Текст", "Text", "Текст"),
        DrawingEllipse("Эллипс", "Ellipse", "Эллипс"),
    }
    /// Способ отображения интервала диаграммы Ганта.
    // Представление корня измерено на платформе 8.3.27.2342.
    GanttChartIntervalRepresentation(
        "ОтображениеИнтервалаДиаграммыГанта",
        "GanttChartIntervalRepresentation"
    )
        meta "ПеречислениеОтображениеИнтервалаДиаграммыГанта" {
        // Состав и представления измерены в `measure-gantt-interval-representation.bsl`.
        // Платформа печатает `3D` и `Diamond`, а не имена членов.
        GanttChartIntervalRepresentationFlat("Плоский", "Flat", "Плоский"),
        GanttChartIntervalRepresentationThreeDimensional("Объемный", "ThreeDimensional", "Объемный"),
        GanttChartIntervalRepresentationRhomb("Ромб", "Rhomb", "Ромб"),
        GanttChartIntervalRepresentationGradient("Градиент", "Gradient", "Градиент"),
    }
    /// Способ вертикального растягивания диаграммы Ганта.
    // Представление корня измерено на платформе 8.3.27.2342.
    GanttChartVerticalStretch("РастягиваниеПоВертикалиДиаграммыГанта", "GanttChartVerticalStretch")
        meta "ПеречислениеРастягиваниеПоВертикалиДиаграммыГанта" {
        // Представления измерены в `measure-gantt-vertical-stretch.bsl`.
        // `НеРастягивать` печатается как `No`; строка взята из нативного замера
        // (`tests/conformance/measure/measure-gantt-vertical-stretch.platform.txt`).
        GanttChartVerticalStretchNone("НеРастягивать", "None", "Не растягивать"),
        GanttChartVerticalStretchStretchRows("РастягиватьСтроки", "StretchRows", "Растягивать строки"),
        GanttChartVerticalStretchStretchRowsAndData(
            "РастягиватьСтрокиИДанные",
            "StretchRowsAndData",
            "Растягивать строки и данные"
        ),
    }
    /// Расположение легенды диаграммы.
    // Представление корня измерено на платформе 8.3.27.2342.
    ChartLegendPlacement("РасположениеЛегендыДиаграммы", "ChartLegendPlacement")
        meta "ПеречислениеРасположениеЛегендыДиаграммы" {
        // Состав и представления измерены в `measure-chart-legend-placement.bsl`.
        // Представление `Use coordinates` измерено, а не построено из имени.
        ChartLegendPlacementAuto("Авто", "Auto", "Авто"),
        ChartLegendPlacementBottom("Низ", "Bottom", "Низ"),
        ChartLegendPlacementTop("Верх", "Top", "Верх"),
        ChartLegendPlacementLeft("Лево", "Left", "Лево"),
        ChartLegendPlacementRight("Право", "Right", "Право"),
        ChartLegendPlacementNone("Нет", "None", "Нет"),
        ChartLegendPlacementUseCoordinates(
            "УказываетсяРасположение",
            "UseCoordinates",
            "Указываются координаты"
        ),
    }
    /// Единица шкалы времени.
    // Представление корня измерено на платформе 8.3.27.2342.
    TimeScaleUnitType("ТипЕдиницыШкалыВремени", "TimeScaleUnitType") meta "ПеречислениеТипЕдиницыШкалыВремени" {
        // Состав и представления измерены в `measure-time-scale-unit-type.bsl`.
        // Все восемь представлений измерены на платформе 8.3.27.2342.
        TimeScaleUnitTypeSecond("Секунда", "Second", "Секунда"),
        TimeScaleUnitTypeMinute("Минута", "Minute", "Минута"),
        TimeScaleUnitTypeHour("Час", "Hour", "Час"),
        TimeScaleUnitTypeDay("День", "Day", "День"),
        TimeScaleUnitTypeWeek("Неделя", "Week", "Неделя"),
        TimeScaleUnitTypeMonth("Месяц", "Month", "Месяц"),
        TimeScaleUnitTypeQuarter("Квартал", "Quarter", "Квартал"),
        TimeScaleUnitTypeYear("Год", "Year", "Год"),
    }
    /// Формат дня шкалы времени.
    // Представление корня измерено на платформе 8.3.27.2342.
    TimeScaleDayFormat("ФорматДняШкалыВремени", "TimeScaleDayFormat")
        meta "ПеречислениеФорматДняШкалыВремени" {
        // Представления измерены в `measure-time-scale-day-format.bsl`.
        // Представления, включая порядок частей, взяты из нативного замера
        // (`tests/conformance/measure/measure-time-scale-day-format.platform.txt`).
        TimeScaleDayFormatMonthDay("ДеньМесяца", "MonthDay", "День месяца"),
        TimeScaleDayFormatWeekDay("ДеньНедели", "WeekDay", "День недели"),
        TimeScaleDayFormatMonthDayWeekDay(
            "ДеньМесяцаДеньНедели",
            "MonthDayWeekDay",
            "День месяца + день недели"
        ),
        TimeScaleDayFormatWeekDayMonthDay(
            "ДеньНеделиДеньМесяца",
            "WeekDayMonthDay",
            "День недели + день месяца"
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
        StringEncodingUrl("КодировкаURL", "КодировкаURL", "URL-кодировка строки"),
        StringEncodingUrlInUrl("URLВКодировкеURL", "URLВКодировкеURL", "URL-кодировка URL"),
    }
    /// Направление сортировки коллекций; Connector использует возрастание.
    SortDirection("НаправлениеСортировки", "SortDirection")
        meta "ПеречислениеНаправлениеСортировки" {
        SortDirectionAscending("Возр", "Asc", "По возрастанию"),
        SortDirectionDescending("Убыв", "Desc", "По убыванию"),
    }
    /// Управление выводом поля формы.
    UseOutput("ИспользованиеВывода", "UseOutput") meta "ПеречислениеИспользованиеВывода" {
        // Представления и псевдонимы измерены на 8.3.27.2342.
        UseOutputAuto("Авто", "Auto", "Авто"),
        UseOutputEnable("Разрешить", "Enable", "Разрешить"),
        UseOutputDisable("Запретить", "Disable", "Запретить"),
    }
    /// Размещение страниц в группе формы.
    FormPagesRepresentation("ОтображениеСтраницФормы", "FormPagesRepresentation")
        meta "ПеречислениеОтображениеСтраницФормы" {
        // Все члены и представления измерены на 8.3.27.2342.
        FormPagesRepresentationAuto("Авто", "Auto", "Авто"),
        FormPagesRepresentationNone("Нет", "None", "Нет"),
        FormPagesRepresentationSwipe("Пролистывание", "Swipe", "Пролистывание (сенсорный экран)"),
        FormPagesRepresentationTabsOnTop("ЗакладкиСверху", "TabsOnTop", "Закладки сверху"),
        FormPagesRepresentationTabsOnBottom("ЗакладкиСнизу", "TabsOnBottom", "Закладки снизу"),
        FormPagesRepresentationTabsOnLeftHorizontal(
            "ЗакладкиСлеваГоризонтально",
            "TabsOnLeftHorizontal",
            "Закладки слева (горизонтально)"
        ),
        FormPagesRepresentationTabsOnRightHorizontal(
            "ЗакладкиСправаГоризонтально",
            "TabsOnRightHorizontal",
            "Закладки справа (горизонтально)"
        ),
    }
    /// Расположение картинки на кнопке формы.
    FormButtonPictureLocation("ПоложениеКартинкиКнопкиФормы", "FormButtonPictureLocation")
        meta "ПеречислениеПоложениеКартинкиКнопкиФормы" {
        // Все члены и представления измерены на 8.3.27.2342.
        FormButtonPictureLocationAuto("Авто", "Auto", "Авто"),
        FormButtonPictureLocationLeft("Лево", "Left", "Лево"),
        FormButtonPictureLocationRight("Право", "Right", "Право"),
    }
    /// Тип линии рисунка табличного документа.
    SpreadsheetDocumentDrawingLineType(
        "ТипЛинииРисункаТабличногоДокумента",
        "SpreadsheetDocumentDrawingLineType"
    )
        meta "ПеречислениеТипЛинииРисункаТабличногоДокумента" {
        // Все члены и представления измерены на 8.3.27.2342.
        SpreadsheetDocumentDrawingLineTypeNone("НетЛинии", "None", "Нет линии"),
        SpreadsheetDocumentDrawingLineTypeSolid("Сплошная", "Solid", "Сплошная"),
        SpreadsheetDocumentDrawingLineTypeDashed("Пунктир", "Dashed", "Пунктирная"),
        SpreadsheetDocumentDrawingLineTypeDotted("Точечная", "Dotted", "Точечная"),
        SpreadsheetDocumentDrawingLineTypeDashDotted("ПунктирТочка", "DashDotted", "Пунктирно-точечная"),
        SpreadsheetDocumentDrawingLineTypeDashDottedDotted(
            "ПунктирТочкаТочка",
            "DashDottedDotted",
            "Точечно-пунктирно-точечная"
        ),
    }
    /// Представление поля-флажка в форме.
    CheckBoxType("ВидФлажка", "CheckBoxType") meta "ПеречислениеВидФлажка" {
        // Все члены и представления измерены на 8.3.27.2342.
        CheckBoxTypeAuto("Авто", "Auto", "Авто"),
        CheckBoxTypeCheckBox("Флажок", "CheckBox", "Флажок"),
        CheckBoxTypeSwitch("Выключатель", "Switch", "Выключатель"),
        CheckBoxTypeTumbler("Тумблер", "Tumbler", "Тумблер"),
    }
    /// Звук доставляемого уведомления.
    // Голое имя и его тип измерены отдельно от типа члена перечисления.
    DeliverableNotificationSubscriberType(
        "ТипПодписчикаДоставляемыхУведомлений",
        "DeliverableNotificationSubscriberType"
    )
        meta "ПеречислениеТипПодписчикаДоставляемыхУведомлений" {
        // Представления и псевдонимы измерены на 8.3.27.2342; `APNS` печатается как `APNs`.
        DeliverableNotificationSubscriberTypeGCM("GCM", "GCM", "GCM"),
        DeliverableNotificationSubscriberTypeAPNS("APNS", "APNS", "APNS"),
        DeliverableNotificationSubscriberTypeFCM("FCM", "FCM", "FCM"),
    }
    SoundAlert("ЗвуковоеОповещение", "SoundAlert") meta "ПеречислениеЗвуковоеОповещение" {
        // Оба члена и представления измерены на 8.3.27.2342.
        SoundAlertNone("Нет", "None", "Нет"),
        SoundAlertDefault("ПоУмолчанию", "Default", "По умолчанию"),
    }
    /// Сквозное выравнивание элементов формы.
    ThroughAlign("СквозноеВыравнивание", "ThroughAlign") meta "ПеречислениеСквозноеВыравнивание" {
        // Все члены и представления измерены на 8.3.27.2342.
        ThroughAlignAuto("Авто", "Auto", "Авто"),
        ThroughAlignUse("Использовать", "Use", "Использовать"),
        ThroughAlignDontUse("НеИспользовать", "DontUse", "Не использовать"),
    }
    /// Фигура кнопки формы.
    ButtonShape("ФигураКнопки", "ButtonShape") meta "ПеречислениеФигураКнопки" {
        // Все члены и представления измерены на 8.3.27.2342.
        ButtonShapeAuto("Авто", "Auto", "Авто"),
        ButtonShapeUsual("Обычная", "Usual", "Обычная"),
        ButtonShapeOval("Овал", "Oval", "Овальная"),
    }
    /// Тип группы элементов отбора компоновки данных.
    DataCompositionFilterItemsGroupType(
        "ТипГруппыЭлементовОтбораКомпоновкиДанных",
        "DataCompositionFilterItemsGroupType"
    )
        meta "ПеречислениеТипГруппыЭлементовОтбораКомпоновкиДанных" {
        // Все члены и представления измерены на 8.3.27.2342.
        DataCompositionFilterItemsGroupTypeAndGroup("ГруппаИ", "AndGroup", "Группа И"),
        DataCompositionFilterItemsGroupTypeOrGroup("ГруппаИли", "OrGroup", "Группа Или"),
        DataCompositionFilterItemsGroupTypeNotGroup("ГруппаНе", "NotGroup", "Группа Не"),
    }
    /// Клавиши для сочетаний клавиш.
    Key("Клавиша", "Key") meta "ПеречислениеКлавиша" {
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
    ControlBorderType("ТипРамкиЭлементаУправления", "ControlBorderType") meta "ПеречислениеТипРамкиЭлементаУправления" {
        // Достигнутые члены и представления измерены на 8.3.27.2342.
        ControlBorderTypeWithoutBorder("БезРамки", "WithoutBorder", "Без рамки"),
        ControlBorderTypeSingle("Одинарная", "Single", "Одинарная"),
        ControlBorderTypeUnderline("Подчеркивание", "Underline", "Подчеркивание"),
    }
    /// Вид цвета объекта `Цвет`.
    ColorType("ВидЦвета", "ColorType") meta "ПеречислениеВидЦвета" {
        // Представления и псевдонимы измерены на 1С 8.3.27.2342.
        ColorTypeAbsolute("Абсолютный", "Absolute", "Абсолютный"),
        ColorTypeStyleItem("ЭлементСтиля", "StyleItem", "Элемент стиля"),
        ColorTypeWebColor("WebЦвет", "WebColor", "Web цвет"),
        ColorTypeWindowsColor("WindowsЦвет", "WindowsColor", "Windows цвет"),
        ColorTypeAutoColor("АвтоЦвет", "AutoColor", "Авто цвет"),
    }
    /// Вид линии границы ячейки табличного документа.
    SpreadsheetDocumentCellLineType(
        "ТипЛинииЯчейкиТабличногоДокумента",
        "SpreadsheetDocumentCellLineType"
    )
        meta "ПеречислениеТипЛинииЯчейкиТабличногоДокумента" {
        // Представления и оба написания измерены на 1С 8.3.27.2342.
        SpreadsheetDocumentCellLineTypeLargeDashed("БольшойПунктир", "LargeDashed", "Большой пунктир"),
        SpreadsheetDocumentCellLineTypeDouble("Двойная", "Double", "Двойная"),
        SpreadsheetDocumentCellLineTypeNone("НетЛинии", "None", "Нет линии"),
        SpreadsheetDocumentCellLineTypeThinDashed("РедкийПунктир", "ThinDashed", "Редкий пунктир"),
        SpreadsheetDocumentCellLineTypeSolid("Сплошная", "Solid", "Сплошная"),
        SpreadsheetDocumentCellLineTypeDotted("Точечная", "Dotted", "Точечная"),
    }
    /// Тип хранилища сертификатов криптографии.
    CryptoCertificateStoreType("ТипХранилищаСертификатовКриптографии", "CryptoCertificateStoreType")
        meta "ПеречислениеТипХранилищаСертификатовКриптографии" {
        // Представления и оба написания измерены на 1С 8.3.27.2342.
        CryptoCertificateStoreTypeRootCertificates(
            "КорневыеСертификаты",
            "RootCertificates",
            "Корневые сертификаты"
        ),
        CryptoCertificateStoreTypePersonalCertificates(
            "ПерсональныеСертификаты",
            "PersonalCertificates",
            "Персональные сертификаты"
        ),
        CryptoCertificateStoreTypeRecipientCertificates(
            "СертификатыПолучателей",
            "RecipientCertificates",
            "Сертификаты получателей"
        ),
    }
    /// Обработка текстов интернет-почтового сообщения.
    InternetMailTextProcessing(
        "ОбработкаТекстаИнтернетПочтовогоСообщения",
        "InternetMailTextProcessing"
    )
        meta "ПеречислениеОбработкаТекстаИнтернетПочтовогоСообщения" {
        // Представления и псевдонимы измерены на 1С 8.3.27.2342.
        InternetMailTextProcessingDontProcess("НеОбрабатывать", "DontProcess", "НеОбрабатывать"),
        InternetMailTextProcessingProcess("Обрабатывать", "Process", "Обрабатывать"),
    }
    /// Тип двусторонней печати.
    DuplexPrintingType("ТипДвустороннейПечати", "DuplexPrintingType") meta "ПеречислениеТипДвустороннейПечати" {
        // Достигнутое представление измерено на 1С 8.3.27.2342.
        DuplexPrintingTypeNone("Нет", "None", "Нет"),
    }
    /// Порядок обработки настроек второго фактора аутентификации.
    SecondAuthenticationFactorSettingsProcessingType(
        "ТипОбработкиНастроекВторогоФактораАутентификации",
        "SecondAuthenticationFactorSettingsProcessingType"
    )
        meta "ПеречислениеТипОбработкиНастроекВторогоФактораАутентификации" {
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
    FileDragMode("СпособПеретаскиванияФайлов", "FileDragMode") meta "ПеречислениеСпособПеретаскиванияФайлов" {
        // Полный состав и представления измерены на 1С 8.3.27.2342.
        FileDragModeAsFileRef("КакСсылкаНаФайл", "AsFileRef", "Как ссылка на файл"),
        FileDragModeAsFile("КакФайл", "AsFile", "Как файл"),
    }
    /// Отображение группы кнопок командной панели.
    ButtonGroupRepresentation("ОтображениеГруппыКнопок", "ButtonGroupRepresentation")
        meta "ПеречислениеОтображениеГруппыКнопок" {
        // Полный состав и представления измерены на 1С 8.3.27.2342.
        ButtonGroupRepresentationAuto("Авто", "Auto", "Авто"),
        ButtonGroupRepresentationUsual("Обычное", "Usual", "Обычное"),
        ButtonGroupRepresentationCompact("Компактное", "Compact", "Компактное"),
    }
    /// Тип члена перечисления `ВариантСтандартногоПериода`, достигнутого в УНФ.
    StandardPeriodVariant("ВариантСтандартногоПериода", "StandardPeriodVariant")
        meta "ПеречислениеВариантСтандартногоПериода" {
        // Достигнутые члены измерены на платформе 1С 8.3.27.2342.
        StandardPeriodVariantYesterday("Вчера", "Yesterday", "Вчера"),
        StandardPeriodVariantTillEndOfThisYear(
            "ДоКонцаЭтогоГода",
            "TillEndOfThisYear",
            "До конца этого года"
        ),
        StandardPeriodVariantTillEndOfThisQuarter(
            "ДоКонцаЭтогоКвартала",
            "TillEndOfThisQuarter",
            "До конца этого квартала"
        ),
        StandardPeriodVariantTillEndOfThisMonth(
            "ДоКонцаЭтогоМесяца",
            "TillEndOfThisMonth",
            "До конца этого месяца"
        ),
        StandardPeriodVariantTillEndOfThisHalfYear(
            "ДоКонцаЭтогоПолугодия",
            "TillEndOfThisHalfYear",
            "До конца этого полугодия"
        ),
        StandardPeriodVariantTillEndOfThisTenDays(
            "ДоКонцаЭтойДекады",
            "TillEndOfThisTenDays",
            "До конца этой декады"
        ),
        StandardPeriodVariantTillEndOfThisWeek(
            "ДоКонцаЭтойНедели",
            "TillEndOfThisWeek",
            "До конца этой недели"
        ),
        StandardPeriodVariantTomorrow("Завтра", "Tomorrow", "Завтра"),
        StandardPeriodVariantMonth("Месяц", "Month", "Месяц"),
        StandardPeriodVariantLast7Days("Последние7Дней", "Last7Days", "Последние 7 дней"),
        StandardPeriodVariantCustom("ПроизвольныйПериод", "Custom", "Произвольный период"),
        StandardPeriodVariantLastTenDays("ПрошлаяДекада", "LastTenDays", "Прошлая декада"),
        StandardPeriodVariantLastTenDaysTillSameDayNumber(
            "ПрошлаяДекадаДоТакогоЖеНомераДня",
            "LastTenDaysTillSameDayNumber",
            "Прошлая декада, до такого же номера дня"
        ),
        StandardPeriodVariantLastWeek("ПрошлаяНеделя", "LastWeek", "Прошлая неделя"),
        StandardPeriodVariantLastWeekTillSameWeekDay(
            "ПрошлаяНеделяДоТакогоЖеДняНедели",
            "LastWeekTillSameWeekDay",
            "Прошлая неделя, до такого же дня недели"
        ),
        StandardPeriodVariantLastHalfYear("ПрошлоеПолугодие", "LastHalfYear", "Прошлое полугодие"),
        StandardPeriodVariantLastHalfYearTillSameDate(
            "ПрошлоеПолугодиеДоТакойЖеДаты",
            "LastHalfYearTillSameDate",
            "Прошлое полугодие, до такой же даты"
        ),
        StandardPeriodVariantLastYear("ПрошлыйГод", "LastYear", "Прошлый год"),
        StandardPeriodVariantLastYearTillSameDate(
            "ПрошлыйГодДоТакойЖеДаты",
            "LastYearTillSameDate",
            "Прошлый год, до такой же даты"
        ),
        StandardPeriodVariantLastQuarter("ПрошлыйКвартал", "LastQuarter", "Прошлый квартал"),
        StandardPeriodVariantLastQuarterTillSameDate(
            "ПрошлыйКварталДоТакойЖеДаты",
            "LastQuarterTillSameDate",
            "Прошлый квартал, до такой же даты"
        ),
        StandardPeriodVariantLastMonth("ПрошлыйМесяц", "LastMonth", "Прошлый месяц"),
        StandardPeriodVariantLastMonthTillSameDate(
            "ПрошлыйМесяцДоТакойЖеДаты",
            "LastMonthTillSameDate",
            "Прошлый месяц, до такой же даты"
        ),
        StandardPeriodVariantFromBeginningOfThisYear(
            "СНачалаЭтогоГода",
            "FromBeginningOfThisYear",
            "С начала этого года"
        ),
        StandardPeriodVariantFromBeginningOfThisQuarter(
            "СНачалаЭтогоКвартала",
            "FromBeginningOfThisQuarter",
            "С начала этого квартала"
        ),
        StandardPeriodVariantFromBeginningOfThisMonth(
            "СНачалаЭтогоМесяца",
            "FromBeginningOfThisMonth",
            "С начала этого месяца"
        ),
        StandardPeriodVariantFromBeginningOfThisHalfYear(
            "СНачалаЭтогоПолугодия",
            "FromBeginningOfThisHalfYear",
            "С начала этого полугодия"
        ),
        StandardPeriodVariantFromBeginningOfThisTenDays(
            "СНачалаЭтойДекады",
            "FromBeginningOfThisTenDays",
            "С начала этой декады"
        ),
        StandardPeriodVariantFromBeginningOfThisWeek(
            "СНачалаЭтойНедели",
            "FromBeginningOfThisWeek",
            "С начала этой недели"
        ),
        StandardPeriodVariantToday("Сегодня", "Today", "Сегодня"),
        StandardPeriodVariantNextTenDays("СледующаяДекада", "NextTenDays", "Следующая декада"),
        StandardPeriodVariantNextTenDaysTillSameDayNumber(
            "СледующаяДекадаДоТакогоЖеНомераДня",
            "NextTenDaysTillSameDayNumber",
            "Следующая декада, до такого же номера дня"
        ),
        StandardPeriodVariantNextWeek("СледующаяНеделя", "NextWeek", "Следующая неделя"),
        StandardPeriodVariantNextWeekTillSameWeekDay(
            "СледующаяНеделяДоТакогоЖеДняНедели",
            "NextWeekTillSameWeekDay",
            "Следующая неделя, до такого же дня недели"
        ),
        StandardPeriodVariantNextHalfYear("СледующееПолугодие", "NextHalfYear", "Следующее полугодие"),
        StandardPeriodVariantNextHalfYearTillSameDate(
            "СледующееПолугодиеДоТакойЖеДаты",
            "NextHalfYearTillSameDate",
            "Следующее полугодие, до такой же даты"
        ),
        StandardPeriodVariantNext7Days("Следующие7Дней", "Next7Days", "Следующие 7 дней"),
        StandardPeriodVariantNextYear("СледующийГод", "NextYear", "Следующий год"),
        StandardPeriodVariantNextYearTillSameDate(
            "СледующийГодДоТакойЖеДаты",
            "NextYearTillSameDate",
            "Следующий год, до такой же даты"
        ),
        StandardPeriodVariantNextQuarter("СледующийКвартал", "NextQuarter", "Следующий квартал"),
        StandardPeriodVariantNextQuarterTillSameDate(
            "СледующийКварталДоТакойЖеДаты",
            "NextQuarterTillSameDate",
            "Следующий квартал, до такой же даты"
        ),
        StandardPeriodVariantNextMonth("СледующийМесяц", "NextMonth", "Следующий месяц"),
        StandardPeriodVariantNextMonthTillSameDate(
            "СледующийМесяцДоТакойЖеДаты",
            "NextMonthTillSameDate",
            "Следующий месяц, до такой же даты"
        ),
        StandardPeriodVariantThisTenDays("ЭтаДекада", "ThisTenDays", "Эта декада"),
        StandardPeriodVariantThisWeek("ЭтаНеделя", "ThisWeek", "Эта неделя"),
        StandardPeriodVariantThisHalfYear("ЭтоПолугодие", "ThisHalfYear", "Это полугодие"),
        StandardPeriodVariantThisYear("ЭтотГод", "ThisYear", "Этот год"),
        StandardPeriodVariantThisQuarter("ЭтотКвартал", "ThisQuarter", "Этот квартал"),
        StandardPeriodVariantThisMonth("ЭтотМесяц", "ThisMonth", "Этот месяц"),
    }
    /// Тип члена перечисления `ВариантУправленияВысотойТаблицы`, достигнутого в УНФ.
    TableHeightControlVariant("ВариантУправленияВысотойТаблицы", "TableHeightControlVariant")
        meta "ПеречислениеВариантУправленияВысотойТаблицы" {
        TableHeightControlVariantUseHeightInFormRows(
            "ВСтрокахФормы",
            "UseHeightInFormRows",
            "В строках формы"
        ),
    }
    /// Тип члена перечисления `ВерсияПолнотекстовогоПоиска`, достигнутого в УНФ.
    FullTextSearchVersion("ВерсияПолнотекстовогоПоиска", "FullTextSearchVersion")
        meta "ПеречислениеВерсияПолнотекстовогоПоиска" {
        FullTextSearchVersionVersion1("Версия1", "Version1", "Версия1"),
    }
    /// Тип члена перечисления `ВертикальнаяПрокруткаФормы`, достигнутого в УНФ.
    VerticalFormScroll("ВертикальнаяПрокруткаФормы", "VerticalFormScroll")
        meta "ПеречислениеВертикальнаяПрокруткаФормы" {
        VerticalFormScrollUseIfNecessary(
            "ИспользоватьПриНеобходимости",
            "UseIfNecessary",
            "Использовать при необходимости"
        ),
    }
    /// Тип члена перечисления `ВертикальноеПоложение`, достигнутого в УНФ.
    VerticalAlign("ВертикальноеПоложение", "VerticalAlign") meta "ПеречислениеВертикальноеПоложение" {
        VerticalAlignTop("Верх", "Top", "Прижать вверх"),
        VerticalAlignBottom("Низ", "Bottom", "Прижать вниз"),
        VerticalAlignCenter("Центр", "Center", "Центрировать"),
    }
    /// Тип члена перечисления `ВидДвиженияНакопления`, достигнутого в УНФ.
    AccumulationRecordType("ВидДвиженияНакопления", "AccumulationRecordType")
        meta "ПеречислениеВидДвиженияНакопления" {
        AccumulationRecordTypeReceipt("Приход", "Receipt", "Приход"),
        AccumulationRecordTypeExpense("Расход", "Expense", "Расход"),
    }
    /// Тип члена перечисления `ВидДвиженияБухгалтерии`, измеренный на 1С.
    // Нативная проба подтверждает тип самого перечисления
    // (`docs/research/fixtures/accounting-record-type/measure.platform.txt`).
    AccountingRecordType("ВидДвиженияБухгалтерии", "AccountingRecordType")
        meta "ПеречислениеВидДвиженияБухгалтерии" {
        AccountingRecordTypeDebit("Дебет", "Debit", "Дебет"),
        AccountingRecordTypeCredit("Кредит", "Credit", "Кредит"),
    }
    /// Положение текста относительно картинки; состав и представления измерены на 1С.
    TextPositionRelativeToPicture(
        "ПоложениеТекстаОтносительноКартинки",
        "TextPositionRelativeToPicture"
    )
        meta "ПеречислениеПоложениеТекстаОтносительноКартинки" {
        TextPositionRelativeToPictureAuto("Авто", "Auto", "Авто"),
        TextPositionRelativeToPictureOnTop("Поверх", "OnTop", "Поверх"),
        TextPositionRelativeToPictureTop("Сверху", "Top", "Сверху"),
        TextPositionRelativeToPictureLeft("Слева", "Left", "Слева"),
        TextPositionRelativeToPictureBottom("Снизу", "Bottom", "Снизу"),
        TextPositionRelativeToPictureRight("Справа", "Right", "Справа"),
    }
    /// Тип члена перечисления `ГоризонтальноеПоложение`, достигнутого в УНФ.
    HorizontalAlign("ГоризонтальноеПоложение", "HorizontalAlign") meta "ПеречислениеГоризонтальноеПоложение" {
        HorizontalAlignAuto("Авто", "Auto", "Автоматически"),
        HorizontalAlignLeft("Лево", "Left", "Прижать влево"),
        HorizontalAlignJustify("ПоШирине", "Justify", "По ширине"),
        HorizontalAlignRight("Право", "Right", "Прижать вправо"),
        HorizontalAlignCenter("Центр", "Center", "Центрировать"),
    }
    /// Тип члена перечисления `ГруппировкаКолонок`, достигнутого в УНФ.
    ColumnsGroup("ГруппировкаКолонок", "ColumnsGroup") meta "ПеречислениеГруппировкаКолонок" {
        ColumnsGroupInCell("ВЯчейке", "InCell", "В ячейке"),
        ColumnsGroupVertical("Вертикальная", "Vertical", "Вертикальная"),
        ColumnsGroupHorizontal("Горизонтальная", "Horizontal", "Горизонтальная"),
    }
    /// Тип члена перечисления `ДопустимыйНомерСообщения`, достигнутого в УНФ.
    AllowedMessageNo("ДопустимыйНомерСообщения", "AllowedMessageNo") meta "ПеречислениеДопустимыйНомерСообщения" {
        AllowedMessageNoGreater("Больший", "Greater", "Больший"),
    }
    /// Тип члена перечисления `ИспользованиеГруппИЭлементов`, достигнутого в УНФ.
    FoldersAndItemsUse("ИспользованиеГруппИЭлементов", "FoldersAndItemsUse")
        meta "ПеречислениеИспользованиеГруппИЭлементов" {
        FoldersAndItemsUseFolders("Группы", "Folders", "Группы"),
        FoldersAndItemsUseFoldersAndItems("ГруппыИЭлементы", "FoldersAndItems", "Группы и элементы"),
        FoldersAndItemsUseItems("Элементы", "Items", "Элементы"),
    }
    /// Тип члена перечисления `ИспользованиеПараметраКомпоновкиДанных`, достигнутого в УНФ.
    DataCompositionParameterUse(
        "ИспользованиеПараметраКомпоновкиДанных",
        "DataCompositionParameterUse"
    )
        meta "ПеречислениеИспользованиеПараметраКомпоновкиДанных" {
        DataCompositionParameterUseAuto("Авто", "Auto", "Авто"),
        DataCompositionParameterUseAlways("Всегда", "Always", "Всегда"),
    }
    /// Тип члена перечисления `ИспользованиеПолосыПрокрутки`, достигнутого в УНФ.
    ScrollBarUse("ИспользованиеПолосыПрокрутки", "ScrollBarUse") meta "ПеречислениеИспользованиеПолосыПрокрутки" {
        ScrollBarUseAutoUse("ИспользоватьАвтоматически", "AutoUse", "Использовать автоматически"),
        ScrollBarUseDontUse("НеИспользовать", "DontUse", "Не использовать"),
    }
    /// Тип члена перечисления `ИспользованиеТекущейСтроки`, достигнутого в УНФ.
    CurrentRowUse("ИспользованиеТекущейСтроки", "CurrentRowUse") meta "ПеречислениеИспользованиеТекущейСтроки" {
        CurrentRowUseAuto("Авто", "Auto", "Авто"),
    }
    /// Тип члена перечисления `НаправлениеПорядкаСхемыЗапроса`, достигнутого в УНФ.
    QuerySchemaOrderDirection("НаправлениеПорядкаСхемыЗапроса", "QuerySchemaOrderDirection")
        meta "ПеречислениеНаправлениеПорядкаСхемыЗапроса" {
        /// Порядок по возрастанию значений, измеренный на платформе 1С.
        QuerySchemaOrderDirectionAscending("ПоВозрастанию", "Ascending", "По возрастанию"),
        QuerySchemaOrderDirectionDescending("ПоУбыванию", "Descending", "По убыванию"),
    }
    /// Тип члена перечисления `НачальноеОтображениеДерева`, достигнутого в УНФ.
    InitialTreeView("НачальноеОтображениеДерева", "InitialTreeView") meta "ПеречислениеНачальноеОтображениеДерева" {
        InitialTreeViewNoExpand("НеРаскрывать", "NoExpand", "Не раскрывать"),
        InitialTreeViewExpandAllLevels(
            "РаскрыватьВсеУровни",
            "ExpandAllLevels",
            "Раскрывать все уровни"
        ),
    }
    /// Тип члена перечисления `ОбновлениеПриИзмененииДанных`, достигнутого в УНФ.
    UpdateOnDataChange("ОбновлениеПриИзмененииДанных", "UpdateOnDataChange")
        meta "ПеречислениеОбновлениеПриИзмененииДанных" {
        UpdateOnDataChangeAuto("Авто", "Auto", "Авто"),
    }
    /// Тип члена перечисления `ОбновлениеТекстаРедактирования`, достигнутого в УНФ.
    EditTextUpdate("ОбновлениеТекстаРедактирования", "EditTextUpdate") meta "ПеречислениеОбновлениеТекстаРедактирования" {
        EditTextUpdateDontUse("НеИспользовать", "DontUse", "Не использовать"),
        EditTextUpdateOnValueChange("ПриИзмененииЗначения", "OnValueChange", "При изменении значения"),
    }
    /// Тип члена перечисления `ОтображениеПредупрежденияПриРедактировании`, достигнутого в УНФ.
    WarningOnEditRepresentation(
        "ОтображениеПредупрежденияПриРедактировании",
        "WarningOnEditRepresentation"
    )
        meta "ПеречислениеОтображениеПредупрежденияПриРедактировании" {
        WarningOnEditRepresentationDontShow("НеОтображать", "DontShow", "Не отображать"),
        WarningOnEditRepresentationShow("Отображать", "Show", "Отображать"),
    }
    /// Тип члена перечисления `ПоискВТаблицеПриВводе`, достигнутого в УНФ.
    SearchInTableOnInput("ПоискВТаблицеПриВводе", "SearchInTableOnInput")
        meta "ПеречислениеПоискВТаблицеПриВводе" {
        SearchInTableOnInputDontUse("НеИспользовать", "DontUse", "Не использовать"),
    }
    /// Тип члена перечисления `ПоложениеКоманднойПанелиФормы`, достигнутого в УНФ.
    FormCommandBarLabelLocation("ПоложениеКоманднойПанелиФормы", "FormCommandBarLabelLocation")
        meta "ПеречислениеПоложениеКоманднойПанелиФормы" {
        FormCommandBarLabelLocationAuto("Авто", "Auto", "Авто"),
        FormCommandBarLabelLocationNone("Нет", "None", "Нет"),
        FormCommandBarLabelLocationBottom("Низ", "Bottom", "Низ"),
    }
    /// Тип члена перечисления `ПоложениеКоманднойПанелиЭлементаФормы`, достигнутого в УНФ.
    FormItemCommandBarLabelLocation(
        "ПоложениеКоманднойПанелиЭлементаФормы",
        "FormItemCommandBarLabelLocation"
    )
        meta "ПеречислениеПоложениеКоманднойПанелиЭлементаФормы" {
        FormItemCommandBarLabelLocationAuto("Авто", "Auto", "Авто"),
        FormItemCommandBarLabelLocationTop("Верх", "Top", "Верх"),
        FormItemCommandBarLabelLocationNone("Нет", "None", "Нет"),
    }
    /// Тип члена перечисления `ПоложениеПодписейШкалыЗначенийИзмерительнойДиаграммы`, достигнутого в УНФ.
    GaugeChartValuesScaleLabelsLocation(
        "ПоложениеПодписейШкалыЗначенийИзмерительнойДиаграммы",
        "GaugeChartValuesScaleLabelsLocation"
    )
        meta "ПеречислениеПоложениеПодписейШкалыЗначенийИзмерительнойДиаграммы" {
        GaugeChartValuesScaleLabelsLocationAtScale("НаШкале", "AtScale", "На шкале"),
    }
    /// Тип положения подписи, достигнутого в модуле менеджера отчёта БУХ.
    ChartLabelLocation("ПоложениеПодписейКДиаграмме", "ChartLabelLocation")
        meta "ПеречислениеПоложениеПодписейКДиаграмме" {
        /// Автоматическое положение подписи относительно края диаграммы.
        ChartLabelLocationEdgeAuto("КрайАвто", "EdgeAuto", "Край авто"),
        /// Подпись с края внутри значения диаграммы.
        ChartLabelLocationEdgeInside("КрайВнутри", "EdgeInside", "Край внутри"),
    }
    /// Тип положения подписей шкалы, достигнутого в отчёте БУХ.
    ChartScaleLabelLocation("ПоложениеПодписейШкалыДиаграммы", "ChartScaleLabelLocation")
        meta "ПеречислениеПоложениеПодписейШкалыДиаграммы" {
        /// Отсутствие подписей шкалы диаграммы.
        ChartScaleLabelLocationNone("Нет", "None", "Не отображать"),
    }
    /// Отображение в легенде диаграммы.
    ShowInChartLegend("ОтображениеВЛегендеДиаграммы", "ShowInChartLegend")
        meta "ПеречислениеОтображениеВЛегендеДиаграммы" {
        /// Автоматическое отображение в легенде диаграммы.
        ShowInChartLegendAuto("Авто", "Auto", "Авто"),
        /// Отображение в легенде диаграммы.
        ShowInChartLegendShow("Отображать", "Show", "Отображать"),
        /// Отсутствие отображения в легенде диаграммы.
        ShowInChartLegendDontShow("НеОтображать", "DontShow", "Не отображать"),
    }
    /// Использование расшифровки табличного документа.
    SpreadsheetDocumentDetailUse(
        "ИспользованиеРасшифровкиТабличногоДокумента",
        "SpreadsheetDocumentDetailUse"
    )
        meta "ПеречислениеИспользованиеРасшифровкиТабличногоДокумента" {
        /// Расшифровка табличного документа без обработки.
        SpreadsheetDocumentDetailUseWithoutProcessing(
            "БезОбработки",
            "WithoutProcessing",
            "Без обработки"
        ),
        /// Расшифровка отдельной ячейки табличного документа.
        SpreadsheetDocumentDetailUseCell("Ячейка", "Cell", "Ячейка"),
    }
    /// Тип члена перечисления `ПоложениеСостоянияПросмотра`, достигнутого в УНФ.
    ViewStatusLocation("ПоложениеСостоянияПросмотра", "ViewStatusLocation")
        meta "ПеречислениеПоложениеСостоянияПросмотра" {
        ViewStatusLocationTop("Верх", "Top", "Верх"),
    }
    /// Тип члена перечисления `ПоложениеСтрокиПоиска`, достигнутого в УНФ.
    SearchStringLocation("ПоложениеСтрокиПоиска", "SearchStringLocation")
        meta "ПеречислениеПоложениеСтрокиПоиска" {
        SearchStringLocationAuto("Авто", "Auto", "Авто"),
        SearchStringLocationNone("Нет", "None", "Нет"),
    }
    /// Тип члена перечисления `ПоложениеУправленияПоиском`, достигнутого в УНФ.
    SearchControlLocation("ПоложениеУправленияПоиском", "SearchControlLocation")
        meta "ПеречислениеПоложениеУправленияПоиском" {
        SearchControlLocationNone("Нет", "None", "Нет"),
    }
    /// Тип члена перечисления `РежимАвтоВремя`, достигнутого в УНФ.
    AutoTimeMode("РежимАвтоВремя", "AutoTimeMode") meta "ПеречислениеРежимАвтоВремя" {
        AutoTimeModeLast("Последним", "Last", "Последним"),
        AutoTimeModeCurrentOrLast("ТекущееИлиПоследним", "CurrentOrLast", "Текущее или последним"),
    }
    /// Тип члена перечисления `РежимВводаСтрокТаблицы`, достигнутого в УНФ.
    TableRowInputMode("РежимВводаСтрокТаблицы", "TableRowInputMode") meta "ПеречислениеРежимВводаСтрокТаблицы" {
        TableRowInputModeEndOfList("ВКонецСписка", "EndOfList", "В конце списка"),
    }
    /// Тип члена перечисления `РежимВключенияСертификатовКриптографии`, достигнутого в УНФ.
    CryptoCertificateIncludeMode(
        "РежимВключенияСертификатовКриптографии",
        "CryptoCertificateIncludeMode"
    )
        meta "ПеречислениеРежимВключенияСертификатовКриптографии" {
        CryptoCertificateIncludeModeIncludeWholeChain(
            "ВключатьПолнуюЦепочку",
            "IncludeWholeChain",
            "Включать полную цепочку"
        ),
        CryptoCertificateIncludeModeIncludeSubjectCertificate(
            "ВключатьСертификатСубъекта",
            "IncludeSubjectCertificate",
            "Включать сертификат субъекта"
        ),
        CryptoCertificateIncludeModeDontInclude("НеВключать", "DontInclude", "Не включать"),
    }
    /// Тип члена перечисления `РежимВыделенияСтрокиТаблицы`, достигнутого в УНФ.
    TableRowSelectionMode("РежимВыделенияСтрокиТаблицы", "TableRowSelectionMode")
        meta "ПеречислениеРежимВыделенияСтрокиТаблицы" {
        TableRowSelectionModeRow("Строка", "Row", "Строка"),
        TableRowSelectionModeCell("Ячейка", "Cell", "Ячейка"),
    }
    /// Тип члена перечисления `РежимВыделенияТаблицы`, достигнутого в УНФ.
    TableSelectionMode("РежимВыделенияТаблицы", "TableSelectionMode") meta "ПеречислениеРежимВыделенияТаблицы" {
        TableSelectionModeSingleRow("Одиночный", "SingleRow", "Одиночный"),
    }
    /// Тип члена перечисления `РежимЗамещения`, достигнутого в УНФ.
    ReplacementMode("РежимЗамещения", "ReplacementMode") meta "ПеречислениеРежимЗамещения" {
        ReplacementModeAppend("Добавление", "Append", "Добавление"),
        ReplacementModeReplace("Замещение", "Replace", "Замещение"),
        ReplacementModeUpdate("Обновление", "Update", "Обновление"),
        ReplacementModeMerge("Слияние", "Merge", "Слияние"),
        ReplacementModeDelete("Удаление", "Delete", "Удаление"),
    }
    /// Тип члена перечисления `РежимИзмененияРазмера`, достигнутого в УНФ.
    SizeChangeMode("РежимИзмененияРазмера", "SizeChangeMode") meta "ПеречислениеРежимИзмененияРазмера" {
        SizeChangeModeQuickChange("БыстроеИзменение", "QuickChange", "Быстрое изменение"),
        SizeChangeModeNormal("Обычный", "Normal", "Обычный"),
    }
    /// Тип члена перечисления `РежимОткрытияОкнаФормы`, достигнутого в УНФ.
    FormWindowOpeningMode("РежимОткрытияОкнаФормы", "FormWindowOpeningMode")
        meta "ПеречислениеРежимОткрытияОкнаФормы" {
        FormWindowOpeningModeLockWholeInterface(
            "БлокироватьВесьИнтерфейс",
            "LockWholeInterface",
            "Блокировать весь интерфейс"
        ),
        FormWindowOpeningModeLockOwnerWindow(
            "БлокироватьОкноВладельца",
            "LockOwnerWindow",
            "Блокировать окно владельца"
        ),
        FormWindowOpeningModeIndependent("Независимый", "Independent", "Независимый"),
    }
    /// Тип члена перечисления `РежимОтображенияНастроекКомпоновкиДанных`, достигнутого в УНФ.
    DataCompositionSettingsViewMode(
        "РежимОтображенияНастроекКомпоновкиДанных",
        "DataCompositionSettingsViewMode"
    )
        meta "ПеречислениеРежимОтображенияНастроекКомпоновкиДанных" {
        DataCompositionSettingsViewModeQuickAccess("БыстрыйДоступ", "QuickAccess", "Быстрый доступ"),
        DataCompositionSettingsViewModeAll("Все", "All", "Все"),
    }
    /// Тип члена перечисления `РежимОтображенияРезультатаОтчета`, достигнутого в УНФ.
    ReportResultViewMode("РежимОтображенияРезультатаОтчета", "ReportResultViewMode")
        meta "ПеречислениеРежимОтображенияРезультатаОтчета" {
        ReportResultViewModeAuto("Авто", "Auto", "Авто"),
        ReportResultViewModeCompact("Компактный", "Compact", "Компактный"),
        ReportResultViewModeDefault("Обычный", "Default", "Обычный"),
    }
    /// Тип члена перечисления `РежимПолнотекстовогоПоиска`, достигнутого в УНФ.
    FullTextSearchMode("РежимПолнотекстовогоПоиска", "FullTextSearchMode")
        meta "ПеречислениеРежимПолнотекстовогоПоиска" {
        FullTextSearchModeDisable("Запретить", "Disable", "Запретить"),
        FullTextSearchModeEnable("Разрешить", "Enable", "Разрешить"),
    }
    /// Тип члена перечисления `РежимПроверкиСертификатаКриптографии`, достигнутого в УНФ.
    CryptoCertificateCheckMode("РежимПроверкиСертификатаКриптографии", "CryptoCertificateCheckMode")
        meta "ПеречислениеРежимПроверкиСертификатаКриптографии" {
        CryptoCertificateCheckModeIgnoreTimeValidity(
            "ИгнорироватьВремяДействия",
            "IgnoreTimeValidity",
            "Игнорировать время действия"
        ),
        CryptoCertificateCheckModeIgnoreSignatureValidity(
            "ИгнорироватьДействительностьПодписи",
            "IgnoreSignatureValidity",
            "Игнорировать действительность подписи"
        ),
        CryptoCertificateCheckModeIgnoreCertificateRevocationStatus(
            "ИгнорироватьПроверкуВСпискеОтозванныхСертификатов",
            "IgnoreCertificateRevocationStatus",
            "Игнорировать проверку в списке отозванных сертификатов"
        ),
        CryptoCertificateCheckModeAllowTestCertificates(
            "РазрешитьТестовыеСертификаты",
            "AllowTestCertificates",
            "Разрешить тестовые"
        ),
    }
    /// Тип члена перечисления `РежимРедактированияКолонки`, достигнутого в УНФ.
    ColumnEditMode("РежимРедактированияКолонки", "ColumnEditMode") meta "ПеречислениеРежимРедактированияКолонки" {
        ColumnEditModeEnter("Вход", "Enter", "Вход"),
        ColumnEditModeEnterOnInput("ВходПриВводе", "EnterOnInput", "Вход при вводе"),
        ColumnEditModeDirectly("Непосредственно", "Directly", "Непосредственно"),
    }
    /// Тип члена перечисления `СпособВосстановленияПароляПользователяИнформационнойБазы`, достигнутого в УНФ.
    InfoBaseUserPasswordRecoveryMethod(
        "СпособВосстановленияПароляПользователяИнформационнойБазы",
        "InfoBaseUserPasswordRecoveryMethod"
    )
        meta "ПеречислениеСпособВосстановленияПароляПользователяИнформационнойБазы" {
        InfoBaseUserPasswordRecoveryMethodSendVerificationCodeBySetParameters(
            "ОтправкаКодаПодтвержденияПоЗаданнымПараметрам",
            "SendVerificationCodeBySetParameters",
            "Отправка кода подтверждения по заданным параметрам"
        ),
        InfoBaseUserPasswordRecoveryMethodSendVerificationCodeByStandardService(
            "ОтправкаКодаПодтвержденияЧерезСтандартныйСервис",
            "SendVerificationCodeByStandardService",
            "Отправка кода подтверждения через стандартный сервис"
        ),
    }
    /// Тип члена перечисления `ТипЗаполненияОбластиТабличногоДокумента`, достигнутого в УНФ.
    SpreadsheetDocumentAreaFillType(
        "ТипЗаполненияОбластиТабличногоДокумента",
        "SpreadsheetDocumentAreaFillType"
    )
        meta "ПеречислениеТипЗаполненияОбластиТабличногоДокумента" {
        SpreadsheetDocumentAreaFillTypeParameter("Параметр", "Parameter", "Параметр"),
        SpreadsheetDocumentAreaFillTypeText("Текст", "Text", "Текст"),
        SpreadsheetDocumentAreaFillTypeTemplate("Шаблон", "Template", "Шаблон"),
    }
    /// Тип члена перечисления `ТипОбъединенияСхемыЗапроса`, достигнутого в УНФ.
    QuerySchemaUnionType("ТипОбъединенияСхемыЗапроса", "QuerySchemaUnionType")
        meta "ПеречислениеТипОбъединенияСхемыЗапроса" {
        QuerySchemaUnionTypeUnion("Объединить", "Union", "Объединить"),
        QuerySchemaUnionTypeUnionAll("ОбъединитьВсе", "UnionAll", "Объединить все"),
    }
    /// Тип члена перечисления `ТипПодписиКриптографии`, достигнутого в УНФ.
    CryptoSignatureType("ТипПодписиКриптографии", "CryptoSignatureType")
        meta "ПеречислениеТипПодписиКриптографии" {
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
        meta "ПеречислениеТипРазмещенияТекстаТабличногоДокумента" {
        SpreadsheetDocumentTextPlacementTypeCut("Обрезать", "Cut", "Обрезать"),
        SpreadsheetDocumentTextPlacementTypeWrap("Переносить", "Wrap", "Переносить"),
    }
    /// Тип члена перечисления `ТипСоединенияСхемыЗапроса`, достигнутого в УНФ.
    QuerySchemaJoinType("ТипСоединенияСхемыЗапроса", "QuerySchemaJoinType")
        meta "ПеречислениеТипСоединенияСхемыЗапроса" {
        QuerySchemaJoinTypeInner("Внутреннее", "Inner", "Внутреннее"),
        QuerySchemaJoinTypeLeftOuter("ЛевоеВнешнее", "LeftOuter", "Левое внешнее"),
        QuerySchemaJoinTypeRightOuter("ПравоеВнешнее", "RightOuter", "Правое внешнее"),
    }
    /// Тип члена перечисления `ТипФормыОтчета`, достигнутого в УНФ.
    ReportFormType("ТипФормыОтчета", "ReportFormType") meta "ПеречислениеТипФормыОтчета" {
        ReportFormTypeSettings("Настройка", "Settings", "Настройка"),
        ReportFormTypeMain("Основная", "Main", "Основная"),
    }
    /// Тип члена перечисления `ТипЭлементаФорматированногоДокумента`, достигнутого в УНФ.
    FormattedDocumentItemType("ТипЭлементаФорматированногоДокумента", "FormattedDocumentItemType")
        meta "ПеречислениеТипЭлементаФорматированногоДокумента" {
        FormattedDocumentItemTypeText("Текст", "Text", "Текст"),
        FormattedDocumentItemTypeLineFeed("ПереводСтроки", "LineFeed", "ПереводСтроки"),
        FormattedDocumentItemTypePicture("Картинка", "Picture", "Картинка"),
    }
    /// Тип члена перечисления `ФиксацияВТаблице`, достигнутого в УНФ.
    FixingInTable("ФиксацияВТаблице", "FixingInTable") meta "ПеречислениеФиксацияВТаблице" {
        FixingInTableLeft("Лево", "Left", "Лево"),
        FixingInTableNone("Нет", "None", "Нет"),
        FixingInTableRight("Право", "Right", "Право"),
    }
    /// Скорость соединения текущего клиента.
    ClientConnectionSpeed("СкоростьКлиентскогоСоединения", "ClientConnectionSpeed")
        meta "ПеречислениеСкоростьКлиентскогоСоединения" {
        // Имена и представления измерены на 8.3.27 пробой
        // `measure-client-connection-speed.bsl`.
        ClientConnectionSpeedLow("Низкая", "Low", "Низкая"),
        ClientConnectionSpeedNormal("Обычная", "Normal", "Обычная"),
    }
    /// Тип области ячеек табличного документа.
    SpreadsheetDocumentCellAreaType(
        "ТипОбластиЯчеекТабличногоДокумента",
        "SpreadsheetDocumentCellAreaType"
    )
        meta "ПеречислениеТипОбластиЯчеекТабличногоДокумента" {
        // Все четыре имени и представления измерены пробой
        // `measure-spreadsheet-cell-area-type.bsl` на 8.3.27.
        SpreadsheetDocumentCellAreaTypeTable("Таблица", "Table", "Таблица"),
        SpreadsheetDocumentCellAreaTypeRows("Строки", "Rows", "Строки"),
        SpreadsheetDocumentCellAreaTypeColumns("Колонки", "Columns", "Колонки"),
        SpreadsheetDocumentCellAreaTypeRectangle("Прямоугольник", "Rectangle", "Прямоугольник"),
    }
    /// Тип подключения внешней компоненты.
    // НЕ ИЗМЕРЕНО(ENUM.ADDIN_CONNECTION_TYPE.PRESENTATION):
    // перечисление доступно только в толстом клиенте.
    AddInConnectionType("ТипПодключенияВнешнейКомпоненты", "AddInConnectionType")
        meta "ПеречислениеТипПодключенияВнешнейКомпоненты" {
        AddInConnectionTypeIsolated("Изолированно", "Isolated", "Изолированно"),
        AddInConnectionTypeNotIsolated("НеИзолированно", "NotIsolated", "НеИзолированно"),
    }
    /// Проверка заполнения реквизита.
    FillChecking("ПроверкаЗаполнения", "FillChecking") meta "ПеречислениеПроверкаЗаполнения" {
        // Представления `Yes` и `No` измерены пробой `measure-fill-checking.bsl`.
        FillCheckingShowError("ВыдаватьОшибку", "ShowError", "Выдавать ошибку"),
        FillCheckingDontCheck("НеПроверять", "DontCheck", "Не проверять"),
    }
    /// Выбор групп и элементов иерархических данных.
    FoldersAndItems("ГруппыИЭлементы", "FoldersAndItems") meta "ПеречислениеГруппыИЭлементы" {
        // Имена и представления измерены пробой `measure-folders-and-items.bsl`.
        FoldersAndItemsAuto("Авто", "Auto", "Авто"),
        FoldersAndItemsFolders("Группы", "Folders", "Группы"),
        FoldersAndItemsBoth("ГруппыИЭлементы", "FoldersAndItems", "ГруппыИЭлементы"),
        FoldersAndItemsItems("Элементы", "Items", "Элементы"),
    }
    /// Протокол интернет-почты.
    /// Устаревшие способы аутентификации, используемые исходниками 1С.
    XmlCanonicalizationType("ТипКаноническогоXML", "XMLCanonicalizationType")
        meta "ПеречислениеТипКаноническогоXML" {
        // Состав и представления измерены пробой `xml-canonicalization-type/measure.bsl`.
        // Все значения, aliases и представления измерены контролем `mail-authentication`.
        // Нативный эталон: `xml-canonicalization-type/measure.platform.txt`.
        XmlCanonicalizationTypeCanonical("КаноническийXML", "XMLCanonicalization", "Канонический XML"),
        XmlCanonicalizationTypeCanonicalWithComments(
            "КаноническийXMLСКомментариями",
            "XMLCanonicalizationWithComments",
            "Канонический XML с комментариями"
        ),
        XmlCanonicalizationTypeExclusive(
            "ИсключающийКаноническийXML",
            "XMLExclusiveCanonicalization",
            "Исключающий канонический XML"
        ),
        XmlCanonicalizationTypeExclusiveWithComments(
            "ИсключающийКаноническийXMLСКомментариями",
            "XMLExclusiveCanonicalizationWithComments",
            "Исключающий канонический XML с комментариями"
        ),
        XmlCanonicalizationTypeCanonical11(
            "КаноническийXML1_1",
            "XMLCanonicalization1_1",
            "Канонический XML 1.1"
        ),
        XmlCanonicalizationTypeCanonical11WithComments(
            "КаноническийXML1_1СКомментариями",
            "XMLCanonicalization1_1WithComments",
            "Канонический XML 1.1 с комментариями"
        ),
    }
    Pop3AuthenticationMode("СпособPOP3Аутентификации", "POP3AuthenticationMode")
        meta "ПеречислениеСпособPOP3Аутентификации" {
        Pop3AuthenticationModeApop("APOP", "APOP", "APOP"),
        Pop3AuthenticationModeCramMd5("CramMD5", "CramMD5", "CramMD5"),
        Pop3AuthenticationModeGeneral("Обычная", "General", "Обычная"),
    }
    SmtpAuthenticationMode("СпособSMTPАутентификации", "SMTPAuthenticationMode")
        meta "ПеречислениеСпособSMTPАутентификации" {
        SmtpAuthenticationModeCramMd5("CramMD5", "CramMD5", "CramMD5"),
        SmtpAuthenticationModeLogin("Login", "Login", "Login"),
        SmtpAuthenticationModePlain("Plain", "Plain", "Plain"),
        SmtpAuthenticationModeNone("БезАутентификации", "None", "БезАутентификации"),
        SmtpAuthenticationModeDefault("ПоУмолчанию", "Default", "ПоУмолчанию"),
    }
    InternetMailProtocol("ПротоколИнтернетПочты", "InternetMailProtocol")
        meta "ПеречислениеПротоколИнтернетПочты" {
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
        meta "ПеречислениеДействиеОбработкиРасшифровкиКомпоновкиДанных" {
        // Представления, включая пробел после `Open value `, измерены на 1С.
        DataCompositionDetailsProcessingActionNone("Нет", "None", "Нет"),
        DataCompositionDetailsProcessingActionOpenValue(
            "ОткрытьЗначение",
            "OpenValue",
            "Открыть значение "
        ),
        DataCompositionDetailsProcessingActionFilter("Отфильтровать", "Filter", "Отфильтровать"),
        DataCompositionDetailsProcessingActionApplyAppearance(
            "Оформить",
            "ApplyAppearance",
            "Оформить"
        ),
        DataCompositionDetailsProcessingActionDrillDown("Расшифровать", "DrillDown", "Расшифровать"),
        DataCompositionDetailsProcessingActionGroup("Сгруппировать", "Group", "Сгруппировать"),
        DataCompositionDetailsProcessingActionOrder("Упорядочить", "Order", "Упорядочить"),
    }
    /// Какие части даты сохраняет `КвалификаторыДаты`.
    DateFractions("ЧастиДаты", "DateFractions") meta "ПеречислениеЧастиДаты" {
        DateFractionsTime("Время", "Time", "Время"),
        DateFractionsDate("Дата", "Date", "Дата"),
        DateFractionsDateTime("ДатаВремя", "DateTime", "Дата и время"),
    }
    /// Ограничение длины для `КвалификаторыСтроки`.
    AllowedLength("ДопустимаяДлина", "AllowedLength") meta "ПеречислениеДопустимаяДлина" {
        AllowedLengthVariable("Переменная", "Variable", "Переменная"),
        AllowedLengthFixed("Фиксированная", "Fixed", "Фиксированная"),
    }
    /// Алгоритм объекта `ХешированиеДанных`.
    HashFunction("ХешФункция", "HashFunction") meta "ПеречислениеХешФункция" {
        HashCrc32("CRC32", "CRC32", "Хеш функция CRC32"),
        HashMd5("MD5", "MD5", "Хеш функция MD5"),
        HashPbkdf2Sha256("PBKDF2SHA256", "PBKDF2SHA256", "Хеш функция PBKDF2 SHA-256"),
        HashSha1("SHA1", "SHA1", "Хеш функция SHA1"),
        HashSha256("SHA256", "SHA256", "Хеш функция SHA256"),
        HashSha512("SHA512", "SHA512", "Хеш функция SHA512"),
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
        meta "ПеречислениеРежимВосстановленияПутейФайлаАрхива" {
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
            "Кодировка ОС дополнительно UTF8"
        ),
    }
    /// Формат сохранения форматированного документа.
    FormattedDocumentFileType("ТипФайлаФорматированногоДокумента", "FormattedDocumentFileType")
        meta "ПеречислениеТипФайлаФорматированногоДокумента" {
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
    AccountType("ВидСчета", "AccountType") meta "ПеречислениеВидСчета" {
        // Полный состав, псевдонимы и представления измерены на 8.3.27.2342
        // пробой `measure-account-type.bsl`; текст не выводится из имени члена.
        AccountTypeActive("Активный", "Active", "Активный"),
        AccountTypePassive("Пассивный", "Passive", "Пассивный"),
        AccountTypeActivePassive("АктивноПассивный", "ActivePassive", "Активный/Пассивный"),
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
        // Значения подтверждены measure-common-attribute-property-enums на 8.3.27.2342:
        // представление совпадает с русским идентификатором.
        CommonAttributeDataSeparationDontUse("НеИспользовать", "DontUse", "НеИспользовать"),
        CommonAttributeDataSeparationSeparate("Разделять", "Separate", "Разделять"),
    }
    /// АвтоИспользованиеОбщегоРеквизита.
    CommonAttributeAutoUse("АвтоИспользованиеОбщегоРеквизита", "CommonAttributeAutoUse")
        meta "EnumCommonAttributeAutoUse" global [] {
        CommonAttributeAutoUseDontUse("НеИспользовать", "DontUse", "НеИспользовать"),
        CommonAttributeAutoUseUse("Использовать", "Use", "Использовать"),
    }
    /// ИспользованиеОбщегоРеквизита.
    CommonAttributeUse("ИспользованиеОбщегоРеквизита", "CommonAttributeUse")
        meta "EnumCommonAttributeUse" global [] {
        CommonAttributeUseAuto("Авто", "Auto", "Авто"),
        CommonAttributeUseDontUse("НеИспользовать", "DontUse", "НеИспользовать"),
        CommonAttributeUseUse("Использовать", "Use", "Использовать"),
    }
    /// Тип узора табличного документа.
    SpreadsheetDocumentPatternType("ТипУзораТабличногоДокумента", "SpreadsheetDocumentPatternType")
        meta "ПеречислениеТипУзораТабличногоДокумента" {
        /// Отсутствие узора табличного документа.
        SpreadsheetDocumentPatternTypeWithoutPattern("БезУзора", "WithoutPattern", "Без узора"),
    }
    /// Тип расположения области построения диаграммы.
    ChartPlotAreaPlacement("РасположениеОбластиПостроенияДиаграммы", "ChartPlotAreaPlacement")
        meta "ПеречислениеРасположениеОбластиПостроенияДиаграммы" {
        /// Расположение области построения по координатам.
        ChartPlotAreaPlacementUseCoordinates(
            "УказываетсяРасположение",
            "UseCoordinates",
            "Указываются координаты"
        ),
    }
    /// Алгоритм подписи токена доступа.
    AccessTokenSignAlgorithm("АлгоритмПодписиТокенаДоступа", "AccessTokenSignAlgorithm")
        meta "ПеречислениеАлгоритмПодписиТокенаДоступа" {
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
        assert_eq!(
            EnumKind::ClientRunMode.meta_ru_name(),
            "ПеречислениеРежимЗапускаКлиентскогоПриложения"
        );
    }

    #[test]
    fn configuration_extension_enums_match_the_measured_platform_surface() {
        assert_eq!(
            lookup_member(EnumKind::ConfigurationExtensionsSource, "SessionApplied"),
            Some(EnumValue::ConfigurationExtensionsSourceSessionApplied)
        );
        assert_eq!(
            EnumValue::ConfigurationExtensionScopeDataSeparation.display_text(),
            "Разделение данных"
        );
        assert_eq!(
            EnumValue::ConfigurationExtensionApplicationIssueSeverityModerate.display_text(),
            "Обычная"
        );
        assert_eq!(
            EnumKind::ConfigurationExtensionsSource.meta_ru_name(),
            "ПеречислениеИсточникРасширенийКонфигурации"
        );
        assert_eq!(
            EnumKind::ConfigurationExtensionScope.meta_ru_name(),
            "ПеречислениеОбластьДействияРасширенияКонфигурации"
        );
        assert_eq!(
            EnumKind::ConfigurationExtensionApplicationIssueSeverity.meta_ru_name(),
            "ПеречислениеВажностьПроблемыПримененияРасширенияКонфигурации"
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
        assert_eq!(EnumValue::DateFractionsTime.display_text(), "Время");
        assert_eq!(EnumValue::DateFractionsDate.display_text(), "Дата");
        assert_eq!(
            EnumValue::DateFractionsDateTime.display_text(),
            "Дата и время"
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
        assert_eq!(EnumValue::AutoChangeRecordDeny.display_text(), "Запретить");
        assert_eq!(EnumValue::AutoChangeRecordAllow.display_text(), "Разрешить");
        assert_eq!(
            EnumKind::AutoChangeRecord.meta_ru_name(),
            "ПеречислениеАвтоРегистрацияИзменений"
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
        assert_eq!(EnumValue::DataLockModeShared.display_text(), "Разделяемый");
        assert_eq!(
            EnumValue::DataLockModeExclusive.display_text(),
            "Исключительный"
        );
        assert_eq!(
            EnumKind::DataLockMode.meta_ru_name(),
            "ПеречислениеРежимБлокировкиДанных"
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
