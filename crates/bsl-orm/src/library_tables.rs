//! Таблицы дескриптора библиотеки: конструкторы, функции, глобальные
//! свойства, типы и члены объектов, которые крейт регистрирует в рантайме.

use super::*;

const CONSTRUCTORS: &[ConstructorDescriptor] = &[
    ConstructorDescriptor {
        code: ConstructorCode::new(28),
        names: &[
            "МоментВремениСУточнениемПериода",
            "PointInTimeWithPeriodAdjustment",
        ],
        arity: Arity::range(1, 3),
        call: Some(new_adjusted_point_in_time),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(27),
        names: &[
            "НастройкаВторогоФактораАутентификации",
            "SecondAuthenticationFactorSetting",
        ],
        arity: Arity::exact(0),
        call: Some(second_factor_setting::construct),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(26),
        names: &[
            "ОписаниеОбновленияКонфигурации",
            "ConfigurationUpdateDescription",
        ],
        arity: Arity::exact(1),
        call: Some(new_configuration_update_description),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(25),
        names: &[
            "ОбработкаРасшифровкиКомпоновкиДанных",
            "DataCompositionDetailsProcess",
        ],
        arity: Arity::exact(2),
        call: Some(new_data_composition_details_process),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(24),
        names: &["ПостроительЗапроса", "QueryBuilder"],
        arity: Arity::range(0, 1),
        call: Some(new_query_builder),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(23),
        names: &[
            "ОбъектМетаданныхКонфигурация",
            "ConfigurationMetadataObject",
        ],
        arity: Arity::exact(1),
        call: Some(new_configuration_metadata_object),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(22),
        names: &["УдалениеОбъекта", "ObjectDeletion"],
        arity: Arity::exact(1),
        call: Some(new_object_deletion),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(21),
        names: &["ВыражениеСхемыЗапроса", "QuerySchemaExpression"],
        arity: Arity::exact(1),
        call: Some(new_query_schema_expression),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(20),
        names: &[
            "ПользовательскиеНастройкиКомпоновкиДанных",
            "DataCompositionUserSettings",
        ],
        arity: Arity::exact(0),
        call: Some(new_data_composition_user_settings),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(19),
        names: &["Граница", "Boundary"],
        arity: Arity::range(1, 2),
        call: Some(query_boundary::construct),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(18),
        names: &[
            "ПроцессорВыводаРезультатаКомпоновкиДанныхВТабличныйДокумент",
            "DataCompositionResultSpreadsheetDocumentOutputProcessor",
        ],
        arity: Arity::exact(0),
        call: Some(new_data_composition_spreadsheet_output),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(17),
        names: &["СхемаКомпоновкиДанных", "DataCompositionSchema"],
        arity: Arity::exact(0),
        call: Some(new_data_composition_schema),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(16),
        names: &["СтандартныйПериод", "StandardPeriod"],
        arity: Arity::range(0, 2),
        call: Some(new_standard_period),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(15),
        names: &["ОписаниеКонфигурации", "ConfigurationDescription"],
        arity: Arity::exact(1),
        call: Some(new_configuration_description),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(14),
        names: &["ОписаниеИсточникаДанных", "DataSourceDescription"],
        arity: Arity::exact(1),
        call: Some(new_data_source_description),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(13),
        names: &["ПостроительОтчета", "ReportBuilder"],
        arity: Arity::range(0, 1),
        call: Some(new_report_builder),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(1),
        names: &["Запрос", "Query"],
        arity: Arity::range(0, 1),
        call: Some(new_query),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(2),
        names: &["СжатиеДанных", "Deflation"],
        arity: Arity::range(0, 1),
        call: Some(new_deflation),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(3),
        names: &["ХранилищеЗначения", "ValueStorage"],
        arity: Arity::range(1, 2),
        call: Some(new_value_storage),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(4),
        names: &["БлокировкаДанных", "DataLock"],
        arity: Arity::exact(0),
        call: Some(new_data_lock),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(5),
        names: &["МенеджерВременныхТаблиц", "TempTablesManager"],
        arity: Arity::exact(0),
        call: Some(new_temp_tables_manager),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(6),
        names: &["СхемаЗапроса", "QuerySchema"],
        arity: Arity::exact(0),
        call: Some(new_query_schema),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(7),
        names: &["ГенераторСлучайныхПаролей", "RandomPasswordGenerator"],
        arity: Arity::exact(0),
        call: Some(new_random_password_generator),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(8),
        names: &[
            "КомпоновщикМакетаКомпоновкиДанных",
            "DataCompositionTemplateComposer",
        ],
        arity: Arity::exact(0),
        call: Some(new_data_composition_template_composer),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(9),
        names: &["ПроцессорКомпоновкиДанных", "DataCompositionProcessor"],
        arity: Arity::exact(0),
        call: Some(new_data_composition_processor),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(10),
        names: &[
            "ПроцессорВыводаРезультатаКомпоновкиДанныхВКоллекциюЗначений",
            "DataCompositionResultValueCollectionOutputProcessor",
        ],
        arity: Arity::exact(0),
        call: Some(new_data_composition_value_collection_output_processor),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(11),
        names: &[
            "КомпоновщикНастроекКомпоновкиДанных",
            "DataCompositionSettingsComposer",
        ],
        arity: Arity::exact(0),
        call: Some(new_data_composition_settings_composer),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(12),
        names: &[
            "ИсточникДоступныхНастроекКомпоновкиДанных",
            "DataCompositionAvailableSettingsSource",
        ],
        arity: Arity::range(1, 2),
        call: Some(new_data_composition_available_settings_source),
    },
];

const CONSTRUCTOR_TYPES: &[(ConstructorCode, &TypeDescriptor)] = &[
    (ConstructorCode::new(19), &query_boundary::TYPE),
    (ConstructorCode::new(28), &ADJUSTED_POINT_IN_TIME_TYPE),
    (ConstructorCode::new(27), &second_factor_setting::TYPE),
    (ConstructorCode::new(1), &QUERY_TYPE),
    (ConstructorCode::new(2), &DEFLATION_TYPE),
    (ConstructorCode::new(3), &VALUE_STORAGE_TYPE),
    (ConstructorCode::new(4), &DATA_LOCK_TYPE),
    (ConstructorCode::new(5), &TEMP_TABLES_MANAGER_TYPE),
    (ConstructorCode::new(6), &QUERY_SCHEMA_TYPE),
    (ConstructorCode::new(7), &RANDOM_PASSWORD_GENERATOR_TYPE),
    (
        ConstructorCode::new(8),
        &DATA_COMPOSITION_TEMPLATE_COMPOSER_TYPE,
    ),
    (ConstructorCode::new(9), &DATA_COMPOSITION_PROCESSOR_TYPE),
    (
        ConstructorCode::new(10),
        &DATA_COMPOSITION_VALUE_COLLECTION_OUTPUT_PROCESSOR_TYPE,
    ),
    (
        ConstructorCode::new(11),
        &DATA_COMPOSITION_SETTINGS_COMPOSER_TYPE,
    ),
    (
        ConstructorCode::new(12),
        &DATA_COMPOSITION_AVAILABLE_SETTINGS_SOURCE_TYPE,
    ),
    (
        ConstructorCode::new(25),
        &DATA_COMPOSITION_DETAILS_PROCESS_TYPE,
    ),
    (
        ConstructorCode::new(26),
        &CONFIGURATION_UPDATE_DESCRIPTION_TYPE,
    ),
];

const FUNCTIONS: &[FunctionDescriptor] = &[
    FunctionDescriptor {
        code: FunctionCode::new(74),
        names: &["ПолучитьКритерииОтбораORM"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(filter_criteria),
    },
    FunctionDescriptor {
        code: FunctionCode::new(73),
        names: &["WSСсылки", "WSReferences"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(ws_references),
    },
    FunctionDescriptor {
        code: FunctionCode::new(72),
        names: &["ПолучитьРазмерДанныхБазыДанных", "GetDatabaseDataSize"],
        arity: Arity::range(0, 3),
        kind: FunctionKind::Function,
        call: Some(get_database_data_size),
    },
    FunctionDescriptor {
        code: FunctionCode::new(71),
        names: &["УдалитьОбъекты", "DeleteObjects"],
        arity: Arity::range(1, 6),
        kind: FunctionKind::Procedure,
        call: Some(delete_objects),
    },
    FunctionDescriptor {
        code: FunctionCode::new(69),
        names: &["ПолучитьДанныеВыбора", "GetChoiceData"],
        arity: Arity::exact(2),
        kind: FunctionKind::Function,
        call: Some(get_choice_data),
    },
    FunctionDescriptor {
        code: FunctionCode::new(68),
        names: &[
            "УстановитьСоставСтандартногоИнтерфейсаOData",
            "SetStandardODataInterfaceContent",
        ],
        arity: Arity::exact(1),
        kind: FunctionKind::Procedure,
        call: Some(set_standard_odata_interface_content),
    },
    FunctionDescriptor {
        code: FunctionCode::new(67),
        names: &[
            "ПолучитьСоставСтандартногоИнтерфейсаOData",
            "GetStandardODataInterfaceContent",
        ],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(standard_odata_interface_content),
    },
    FunctionDescriptor {
        code: FunctionCode::new(66),
        names: &["ПолноеИмяПользователя", "UserFullName"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(user_full_name),
    },
    FunctionDescriptor {
        code: FunctionCode::new(64),
        names: &["УдалитьДанныеИнформационнойБазы", "EraseInfoBaseData"],
        arity: Arity::exact(0),
        kind: FunctionKind::Procedure,
        call: Some(erase_infobase_data),
    },
    FunctionDescriptor {
        code: FunctionCode::new(63),
        names: &["РазблокироватьДанныеДляРедактирования", "UnlockDataForEdit"],
        arity: Arity::range(0, 2),
        kind: FunctionKind::Procedure,
        call: Some(unlock_data_for_edit),
    },
    FunctionDescriptor {
        code: FunctionCode::new(62),
        names: &["ЗаблокироватьДанныеДляРедактирования", "LockDataForEdit"],
        arity: Arity::range(1, 3),
        kind: FunctionKind::Procedure,
        call: Some(lock_data_for_edit),
    },
    FunctionDescriptor {
        code: FunctionCode::new(1),
        names: &["ПолучитьМетаданныеORM"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(metadata),
    },
    FunctionDescriptor {
        code: FunctionCode::new(2),
        names: &["ПолучитьСправочникиORM"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(catalogs),
    },
    FunctionDescriptor {
        code: FunctionCode::new(3),
        names: &["ПолучитьРегистрыСведенийORM"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(information_registers),
    },
    FunctionDescriptor {
        code: FunctionCode::new(4),
        names: &["ПолучитьКонстантыORM"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(constants),
    },
    FunctionDescriptor {
        code: FunctionCode::new(5),
        names: &["РольДоступна", "IsInRole"],
        arity: Arity::exact(1),
        kind: FunctionKind::Function,
        call: Some(role_available),
    },
    FunctionDescriptor {
        code: FunctionCode::new(6),
        names: &["ПолучитьРасширенияКонфигурацииORM"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(configuration_extensions),
    },
    FunctionDescriptor {
        code: FunctionCode::new(7),
        names: &["ПолучитьПользователейИнформационнойБазыORM"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(info_base_users),
    },
    FunctionDescriptor {
        code: FunctionCode::new(8),
        names: &["ПолучитьПеречисленияORM"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(enumerations),
    },
    FunctionDescriptor {
        code: FunctionCode::new(9),
        names: &["ПолучитьБиблиотекуКартинокORM"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(picture_library),
    },
    FunctionDescriptor {
        code: FunctionCode::new(10),
        names: &["КонфигурацияИзменена", "ConfigurationChanged"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(configuration_changed),
    },
    FunctionDescriptor {
        code: FunctionCode::new(11),
        names: &["ПолучитьПланыОбменаORM"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(exchange_plans),
    },
    FunctionDescriptor {
        code: FunctionCode::new(12),
        names: &["НачатьТранзакцию", "BeginTransaction"],
        arity: Arity::exact(0),
        kind: FunctionKind::Procedure,
        call: Some(begin_transaction),
    },
    FunctionDescriptor {
        code: FunctionCode::new(13),
        names: &["ЗафиксироватьТранзакцию", "CommitTransaction"],
        arity: Arity::exact(0),
        kind: FunctionKind::Procedure,
        call: Some(commit_transaction),
    },
    FunctionDescriptor {
        code: FunctionCode::new(14),
        names: &["ОтменитьТранзакцию", "RollbackTransaction"],
        arity: Arity::exact(0),
        kind: FunctionKind::Procedure,
        call: Some(rollback_transaction),
    },
    FunctionDescriptor {
        code: FunctionCode::new(15),
        names: &["ПравоДоступа", "AccessRight"],
        arity: Arity::range(2, 4),
        kind: FunctionKind::Function,
        call: Some(access_right),
    },
    FunctionDescriptor {
        code: FunctionCode::new(16),
        names: &["ИмяПользователя", "UserName"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(user_name),
    },
    FunctionDescriptor {
        code: FunctionCode::new(17),
        names: &["ПредопределенноеЗначение", "PredefinedValue"],
        arity: Arity::exact(1),
        kind: FunctionKind::Function,
        call: Some(predefined_value),
    },
    FunctionDescriptor {
        code: FunctionCode::new(18),
        names: &["МонопольныйРежим", "ExclusiveMode"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(exclusive_mode),
    },
    FunctionDescriptor {
        code: FunctionCode::new(19),
        names: &["УстановитьМонопольныйРежим", "SetExclusiveMode"],
        arity: Arity::exact(1),
        kind: FunctionKind::Procedure,
        call: Some(set_exclusive_mode),
    },
    FunctionDescriptor {
        code: FunctionCode::new(20),
        names: &["ПолучитьСервисыИнтеграцииORM"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(integration_services),
    },
    FunctionDescriptor {
        code: FunctionCode::new(21),
        names: &[
            "ПолучитьОбновлениеПредопределенныхДанныхИнформационнойБазы",
            "GetInfoBasePredefinedData",
        ],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(info_base_predefined_data_update),
    },
    FunctionDescriptor {
        code: FunctionCode::new(22),
        names: &[
            "УстановитьОбновлениеПредопределенныхДанныхИнформационнойБазы",
            "SetInfoBasePredefinedDataUpdate",
        ],
        arity: Arity::exact(1),
        kind: FunctionKind::Procedure,
        call: Some(set_info_base_predefined_data_update),
    },
    FunctionDescriptor {
        code: FunctionCode::new(23),
        names: &[
            "ИнициализироватьПредопределенныеДанные",
            "InitializePredefinedData",
        ],
        arity: Arity::exact(0),
        kind: FunctionKind::Procedure,
        call: Some(initialize_predefined_data),
    },
    FunctionDescriptor {
        code: FunctionCode::new(24),
        names: &["ПолучитьФункциональнуюОпцию", "GetFunctionalOption"],
        arity: Arity::range(1, 2),
        kind: FunctionKind::Function,
        call: Some(functional_option),
    },
    FunctionDescriptor {
        code: FunctionCode::new(25),
        names: &["ТранзакцияАктивна", "TransactionActive"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(transaction_active),
    },
    FunctionDescriptor {
        code: FunctionCode::new(26),
        names: &["ТекущийЯзык", "CurrentLanguage"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(current_language),
    },
    FunctionDescriptor {
        code: FunctionCode::new(27),
        names: &["НайтиПоСсылкам", "FindByRef"],
        arity: Arity::range(1, 4),
        kind: FunctionKind::Function,
        call: Some(find_by_ref),
    },
    FunctionDescriptor {
        code: FunctionCode::new(28),
        names: &["ПолучитьДокументыORM"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(documents),
    },
    FunctionDescriptor {
        code: FunctionCode::new(29),
        names: &["ПолучитьЖурналыДокументовORM"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(document_journals),
    },
    FunctionDescriptor {
        code: FunctionCode::new(30),
        names: &["ПолучитьОтчетыORM"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(reports),
    },
    FunctionDescriptor {
        code: FunctionCode::new(31),
        names: &["ПолучитьОбработкиORM"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(data_processors),
    },
    FunctionDescriptor {
        code: FunctionCode::new(32),
        names: &["ПолучитьРегистрыНакопленияORM"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(accumulation_registers),
    },
    FunctionDescriptor {
        code: FunctionCode::new(33),
        names: &["ПолучитьРегистрыБухгалтерииORM"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(accounting_registers),
    },
    FunctionDescriptor {
        code: FunctionCode::new(34),
        names: &["ПолучитьРегистрыРасчетаORM"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(calculation_registers),
    },
    FunctionDescriptor {
        code: FunctionCode::new(35),
        names: &["ПолучитьПланыСчетовORM"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(charts_of_accounts),
    },
    FunctionDescriptor {
        code: FunctionCode::new(36),
        names: &["ПолучитьПланыВидовХарактеристикORM"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(charts_of_characteristic_types),
    },
    FunctionDescriptor {
        code: FunctionCode::new(37),
        names: &["ПолучитьПланыВидовРасчетаORM"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(charts_of_calculation_types),
    },
    FunctionDescriptor {
        code: FunctionCode::new(38),
        names: &["ПолучитьБизнесПроцессыORM"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(business_processes),
    },
    FunctionDescriptor {
        code: FunctionCode::new(39),
        names: &["ПолучитьЗадачиORM"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(tasks),
    },
    FunctionDescriptor {
        code: FunctionCode::new(40),
        names: &["ПолучитьХранилищаНастроекORM"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(settings_storages),
    },
    FunctionDescriptor {
        code: FunctionCode::new(41),
        names: &["ПолучитьПоследовательностиORM"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(sequences),
    },
    FunctionDescriptor {
        code: FunctionCode::new(42),
        names: &[
            "ПолучитьПолноеИмяПредопределенногоЗначения",
            "GetPredefinedValueFullName",
        ],
        arity: Arity::exact(1),
        kind: FunctionKind::Function,
        call: Some(predefined_value_full_name),
    },
    FunctionDescriptor {
        code: FunctionCode::new(43),
        names: &["ПолучитьНавигационнуюСсылку", "GetURL"],
        arity: Arity::range(1, 4),
        kind: FunctionKind::Function,
        call: Some(navigation_link),
    },
    FunctionDescriptor {
        code: FunctionCode::new(44),
        names: &["ПолучитьПолитикиПаролейПользователейORM"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(user_password_policies),
    },
    FunctionDescriptor {
        code: FunctionCode::new(45),
        names: &[
            "ПолучитьМинимальнуюДлинуПаролейПользователей",
            "GetUserPasswordMinLength",
        ],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(info_base_user_password_minimum_length),
    },
    FunctionDescriptor {
        code: FunctionCode::new(46),
        names: &["ПолучитьБлокировкуАутентификацииORM"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(authentication_lock_manager),
    },
    FunctionDescriptor {
        code: FunctionCode::new(47),
        names: &[
            "ПолучитьПроверкуСложностиПаролейПользователей",
            "GetUserPasswordStrengthCheck",
        ],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(info_base_user_password_strength_check),
    },
    FunctionDescriptor {
        code: FunctionCode::new(48),
        names: &[
            "ПолучитьМаксимальныйСрокДействияПаролейПользователей",
            "GetUserPasswordMaxEffectivePeriod",
        ],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(user_password_max_effective_period),
    },
    FunctionDescriptor {
        code: FunctionCode::new(49),
        names: &[
            "ПолучитьМинимальныйСрокДействияПаролейПользователей",
            "GetUserPasswordMinEffectivePeriod",
        ],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(user_password_min_effective_period),
    },
    FunctionDescriptor {
        code: FunctionCode::new(50),
        names: &[
            "ПолучитьОграничениеПовторенияПаролейПользователейСредиПоследних",
            "GetUserPasswordReuseLimit",
        ],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(user_password_reuse_limit),
    },
    FunctionDescriptor {
        code: FunctionCode::new(51),
        names: &[
            "ПолучитьСрокПредупрежденияОбИстеченииСрокаДействияПаролейПользователей",
            "GetUserPasswordExpirationNotificationPeriod",
        ],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(user_password_expiration_notification_period),
    },
    FunctionDescriptor {
        code: FunctionCode::new(52),
        names: &[
            "УстановитьПроверкуСложностиПаролейПользователей",
            "SetUserPasswordStrengthCheck",
        ],
        arity: Arity::exact(1),
        kind: FunctionKind::Procedure,
        call: Some(set_user_password_strength_check),
    },
    FunctionDescriptor {
        code: FunctionCode::new(53),
        names: &[
            "УстановитьМинимальнуюДлинуПаролейПользователей",
            "SetUserPasswordMinLength",
        ],
        arity: Arity::exact(1),
        kind: FunctionKind::Procedure,
        call: Some(set_user_password_minimum_length),
    },
    FunctionDescriptor {
        code: FunctionCode::new(54),
        names: &[
            "УстановитьМаксимальныйСрокДействияПаролейПользователей",
            "SetUserPasswordMaxEffectivePeriod",
        ],
        arity: Arity::exact(1),
        kind: FunctionKind::Procedure,
        call: Some(set_user_password_max_effective_period),
    },
    FunctionDescriptor {
        code: FunctionCode::new(55),
        names: &[
            "УстановитьМинимальныйСрокДействияПаролейПользователей",
            "SetUserPasswordMinEffectivePeriod",
        ],
        arity: Arity::exact(1),
        kind: FunctionKind::Procedure,
        call: Some(set_user_password_min_effective_period),
    },
    FunctionDescriptor {
        code: FunctionCode::new(56),
        names: &[
            "УстановитьОграничениеПовторенияПаролейПользователейСредиПоследних",
            "SetUserPasswordReuseLimit",
        ],
        arity: Arity::exact(1),
        kind: FunctionKind::Procedure,
        call: Some(set_user_password_reuse_limit),
    },
    FunctionDescriptor {
        code: FunctionCode::new(57),
        names: &[
            "УстановитьСрокПредупрежденияОбИстеченииСрокаДействияПаролейПользователей",
            "SetUserPasswordExpirationNotificationPeriod",
        ],
        arity: Arity::exact(1),
        kind: FunctionKind::Procedure,
        call: Some(set_user_password_expiration_notification_period),
    },
    FunctionDescriptor {
        code: FunctionCode::new(58),
        names: &["ПолучитьОбщийМакет", "GetCommonTemplate"],
        arity: Arity::exact(1),
        kind: FunctionKind::Function,
        call: Some(common_template),
    },
    FunctionDescriptor {
        code: FunctionCode::new(59),
        names: &["ВыполнитьПроверкуПравДоступа", "VerifyAccessRights"],
        arity: Arity::range(2, 3),
        kind: FunctionKind::Procedure,
        call: Some(verify_access_rights),
    },
    FunctionDescriptor {
        code: FunctionCode::new(60),
        names: &["ПолучитьВнешниеОбработкиORM"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(external_data_processors),
    },
    FunctionDescriptor {
        code: FunctionCode::new(61),
        names: &["ПолучитьВнешниеОтчетыORM"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(external_reports),
    },
    FunctionDescriptor {
        code: FunctionCode::new(65),
        names: &["ПолучитьИсториюДанныхORM"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(data_history),
    },
    FunctionDescriptor {
        code: FunctionCode::new(70),
        names: &["ПолучитьШаблоныНастроекВторогоФактораАутентификацииORM"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(second_authentication_factor_settings_templates),
    },
];

const GLOBALS: &[GlobalDescriptor] = &[
    GlobalDescriptor {
        names: &["КритерииОтбора", "FilterCriteria"],
        function: FunctionCode::new(74),
    },
    GlobalDescriptor {
        names: &["WSСсылки", "WSReferences"],
        function: FunctionCode::new(73),
    },
    GlobalDescriptor {
        names: &[
            "ШаблоныНастроекВторогоФактораАутентификации",
            "SecondAuthenticationFactorSettingsTemplates",
        ],
        function: FunctionCode::new(70),
    },
    GlobalDescriptor {
        names: &["ВнешниеОбработки", "ExternalDataProcessors"],
        function: FunctionCode::new(60),
    },
    GlobalDescriptor {
        names: &["ВнешниеОтчеты", "ExternalReports"],
        function: FunctionCode::new(61),
    },
    GlobalDescriptor {
        names: &["ИсторияДанных", "DataHistory"],
        function: FunctionCode::new(65),
    },
    GlobalDescriptor {
        names: &["Метаданные", "Metadata"],
        function: FunctionCode::new(1),
    },
    GlobalDescriptor {
        names: &["Справочники", "Catalogs"],
        function: FunctionCode::new(2),
    },
    GlobalDescriptor {
        names: &["Документы", "Documents"],
        function: FunctionCode::new(28),
    },
    GlobalDescriptor {
        names: &["ЖурналыДокументов", "DocumentJournals"],
        function: FunctionCode::new(29),
    },
    GlobalDescriptor {
        names: &["Отчеты", "Reports"],
        function: FunctionCode::new(30),
    },
    GlobalDescriptor {
        names: &["Обработки", "DataProcessors"],
        function: FunctionCode::new(31),
    },
    GlobalDescriptor {
        names: &["РегистрыНакопления", "AccumulationRegisters"],
        function: FunctionCode::new(32),
    },
    GlobalDescriptor {
        names: &["РегистрыБухгалтерии", "AccountingRegisters"],
        function: FunctionCode::new(33),
    },
    GlobalDescriptor {
        names: &["РегистрыРасчета", "CalculationRegisters"],
        function: FunctionCode::new(34),
    },
    GlobalDescriptor {
        names: &["ПланыСчетов", "ChartsOfAccounts"],
        function: FunctionCode::new(35),
    },
    GlobalDescriptor {
        names: &["ПланыВидовХарактеристик", "ChartsOfCharacteristicTypes"],
        function: FunctionCode::new(36),
    },
    GlobalDescriptor {
        names: &["ПланыВидовРасчета", "ChartsOfCalculationTypes"],
        function: FunctionCode::new(37),
    },
    GlobalDescriptor {
        names: &["БизнесПроцессы", "BusinessProcesses"],
        function: FunctionCode::new(38),
    },
    GlobalDescriptor {
        names: &["Задачи", "Tasks"],
        function: FunctionCode::new(39),
    },
    GlobalDescriptor {
        names: &["ХранилищаНастроек", "SettingsStorages"],
        function: FunctionCode::new(40),
    },
    GlobalDescriptor {
        names: &["Последовательности", "Sequences"],
        function: FunctionCode::new(41),
    },
    GlobalDescriptor {
        names: &["РегистрыСведений", "InformationRegisters"],
        function: FunctionCode::new(3),
    },
    GlobalDescriptor {
        names: &["Константы", "Constants"],
        function: FunctionCode::new(4),
    },
    GlobalDescriptor {
        names: &["РасширенияКонфигурации", "ConfigurationExtensions"],
        function: FunctionCode::new(6),
    },
    GlobalDescriptor {
        names: &["ПользователиИнформационнойБазы", "InfoBaseUsers"],
        function: FunctionCode::new(7),
    },
    GlobalDescriptor {
        names: &["ПолитикиПаролейПользователей", "UserPasswordPolicies"],
        function: FunctionCode::new(44),
    },
    GlobalDescriptor {
        names: &["БлокировкаАутентификации", "AuthenticationLock"],
        function: FunctionCode::new(46),
    },
    GlobalDescriptor {
        names: &["Перечисления", "Enums"],
        function: FunctionCode::new(8),
    },
    GlobalDescriptor {
        names: &["ПланыОбмена", "ExchangePlans"],
        function: FunctionCode::new(11),
    },
    GlobalDescriptor {
        names: &["БиблиотекаКартинок", "PictureLib"],
        function: FunctionCode::new(9),
    },
    GlobalDescriptor {
        names: &["СервисыИнтеграции", "IntegrationServices"],
        function: FunctionCode::new(20),
    },
];

const TYPES: &[&TypeDescriptor] = &[
    &query_boundary::TYPE,
    &second_factor_setting::TYPE,
    &METADATA_TYPE,
    &METADATA_COLLECTION_TYPE,
    &METADATA_VALUE_TYPE,
    &MAIN_ROLES_TYPE,
    &METADATA_OBJECT_PROPERTIES_TYPE,
    &COMPATIBILITY_MODE_PROPERTY_TYPE,
    &MANAGERS_TYPE,
    &MANAGER_TYPE,
    &ENUMERATION_REFERENCE_TYPE,
    &route_points::COLLECTION_TYPE,
    &route_points::REFERENCE_TYPE,
    &CONFIGURATION_REFERENCE_TYPE,
    &catalog_object::TYPE,
    &characteristic_object::TYPE,
    &document_object::TYPE,
    &business_process_object::TYPE,
    &data_processor_object::TYPE,
    &data_processor_object::REPORT_TYPE,
    &document_movements::TYPE,
    &document_movements::RECORD_SET_TYPE,
    &document_tabular_section::TYPE,
    &document_tabular_section::ROW_TYPE,
    &catalog_object::EXCHANGE_TYPE,
    &catalog_object::RECIPIENTS_TYPE,
    &PICTURE_LIBRARY_TYPE,
    &CONFIGURATION_EXTENSIONS_MANAGER_TYPE,
    &INFO_BASE_USERS_MANAGER_TYPE,
    &INFO_BASE_USER_TYPE,
    &USER_PASSWORD_POLICIES_MANAGER_TYPE,
    &USER_PASSWORD_POLICY_TYPE,
    &AUTHENTICATION_LOCK_MANAGER_TYPE,
    &AUTHENTICATION_LOCK_SETTINGS_TYPE,
    &RANDOM_PASSWORD_GENERATOR_TYPE,
    &INTEGRATION_SERVICES_MANAGER_TYPE,
    &CONFIGURATION_EXTENSION_TYPE,
    &EXTENSION_APPLICATION_ISSUE_TYPE,
    &QUERY_TYPE,
    &QUERY_SCHEMA_TYPE,
    &DATA_COMPOSITION_SETTINGS_COMPOSER_TYPE,
    &DATA_COMPOSITION_AVAILABLE_SETTINGS_SOURCE_TYPE,
    &DATA_COMPOSITION_DETAILS_PROCESS_TYPE,
    &CONFIGURATION_UPDATE_DESCRIPTION_TYPE,
    &DATA_COMPOSITION_TEMPLATE_COMPOSER_TYPE,
    &DATA_COMPOSITION_PROCESSOR_TYPE,
    &DATA_COMPOSITION_VALUE_COLLECTION_OUTPUT_PROCESSOR_TYPE,
    &TEMP_TABLES_MANAGER_TYPE,
    &QUERY_RESULT_TYPE,
    &QUERY_SELECTION_TYPE,
    &catalog_selection::TYPE,
    &DEFLATION_TYPE,
    &VALUE_STORAGE_TYPE,
    &DATA_LOCK_TYPE,
    &DATA_LOCK_ITEM_TYPE,
    &common_attribute_content::COLLECTION_TYPE,
    &common_attribute_content::ITEM_TYPE,
    &ADJUSTED_POINT_IN_TIME_TYPE,
];

const API_MEMBERS: &[ObjectMembersDescriptor] = &[
    ObjectMembersDescriptor::new(&query_boundary::TYPE).with_properties(query_boundary::PROPERTIES),
    ObjectMembersDescriptor::new(&common_attribute_content::COLLECTION_TYPE)
        .with_methods(common_attribute_content::METHODS),
    ObjectMembersDescriptor::new(&common_attribute_content::ITEM_TYPE)
        .with_properties(common_attribute_content::ITEM_PROPERTIES),
    ObjectMembersDescriptor::new(&second_factor_setting::TYPE)
        .with_properties(second_factor_setting::PROPERTIES),
    ObjectMembersDescriptor::new(&METADATA_TYPE)
        .with_methods(metadata_find_by_type::METHODS)
        .with_properties(METADATA_ROOT_PROPERTIES)
        .with_dynamic_properties(),
    ObjectMembersDescriptor::new(&METADATA_COLLECTION_TYPE)
        .with_methods(METADATA_COLLECTION_METHODS)
        .with_dynamic_properties(),
    ObjectMembersDescriptor::new(&METADATA_VALUE_TYPE)
        .with_methods(METADATA_VALUE_METHODS)
        .with_properties(METADATA_VALUE_PROPERTIES)
        .with_dynamic_properties(),
    ObjectMembersDescriptor::new(&MAIN_ROLES_TYPE).with_methods(MAIN_ROLES_METHODS),
    ObjectMembersDescriptor::new(&METADATA_OBJECT_PROPERTIES_TYPE)
        .with_properties(METADATA_OBJECT_PROPERTIES),
    ObjectMembersDescriptor::new(&COMPATIBILITY_MODE_PROPERTY_TYPE),
    ObjectMembersDescriptor::new(&MANAGERS_TYPE).with_dynamic_properties(),
    ObjectMembersDescriptor::new(&MANAGER_TYPE)
        .with_methods(MANAGER_PUBLIC_METHODS)
        .with_properties(BUSINESS_PROCESS_MANAGER_PROPERTIES),
    ObjectMembersDescriptor::new(&ENUMERATION_REFERENCE_TYPE)
        .with_methods(ENUMERATION_REFERENCE_METHODS),
    ObjectMembersDescriptor::new(&route_points::COLLECTION_TYPE)
        .with_methods(route_points::METHODS)
        .with_dynamic_properties(),
    ObjectMembersDescriptor::new(&route_points::REFERENCE_TYPE),
    ObjectMembersDescriptor::new(&CONFIGURATION_REFERENCE_TYPE)
        .with_methods(CONFIGURATION_REFERENCE_METHODS),
    ObjectMembersDescriptor::new(&data_processor_object::REPORT_TYPE)
        .with_methods(data_processor_object::METHODS)
        .with_properties(data_processor_object::REPORT_PROPERTIES),
    ObjectMembersDescriptor::new(&data_processor_object::TYPE)
        .with_methods(data_processor_object::METHODS),
    ObjectMembersDescriptor::new(&business_process_object::TYPE)
        .with_methods(business_process_object::METHODS)
        .with_properties(business_process_object::PROPERTIES),
    ObjectMembersDescriptor::new(&document_object::TYPE)
        .with_methods(document_object::METHODS)
        .with_properties(document_object::PROPERTIES),
    ObjectMembersDescriptor::new(&document_tabular_section::TYPE)
        .with_methods(document_tabular_section::METHODS),
    ObjectMembersDescriptor::new(&document_movements::TYPE)
        .with_methods(document_movements::METHODS)
        .with_dynamic_properties(),
    ObjectMembersDescriptor::new(&document_movements::RECORD_SET_TYPE)
        .with_methods(document_movements::RECORD_SET_METHODS)
        .with_properties(document_movements::RECORD_SET_PROPERTIES),
    ObjectMembersDescriptor::new(&document_tabular_section::ROW_TYPE)
        .with_properties(document_tabular_section::ROW_PROPERTIES)
        .with_dynamic_properties(),
    ObjectMembersDescriptor::new(&characteristic_object::TYPE)
        .with_methods(characteristic_object::METHODS)
        .with_properties(characteristic_object::PROPERTIES),
    ObjectMembersDescriptor::new(&catalog_object::TYPE)
        .with_methods(catalog_object::METHODS)
        .with_properties(catalog_object::PROPERTIES),
    ObjectMembersDescriptor::new(&catalog_object::EXCHANGE_TYPE)
        .with_properties(catalog_object::EXCHANGE_PROPERTIES),
    ObjectMembersDescriptor::new(&catalog_object::RECIPIENTS_TYPE)
        .with_properties(catalog_object::RECIPIENTS_PROPERTIES),
    ObjectMembersDescriptor::new(&PICTURE_LIBRARY_TYPE).with_dynamic_properties(),
    ObjectMembersDescriptor::new(&CONFIGURATION_EXTENSIONS_MANAGER_TYPE)
        .with_methods(CONFIGURATION_EXTENSIONS_MANAGER_METHODS),
    ObjectMembersDescriptor::new(&INFO_BASE_USERS_MANAGER_TYPE)
        .with_methods(INFO_BASE_USERS_MANAGER_METHODS),
    ObjectMembersDescriptor::new(&INFO_BASE_USER_TYPE).with_properties(INFO_BASE_USER_PROPERTIES),
    ObjectMembersDescriptor::new(&USER_PASSWORD_POLICIES_MANAGER_TYPE)
        .with_methods(USER_PASSWORD_POLICIES_MANAGER_METHODS),
    ObjectMembersDescriptor::new(&USER_PASSWORD_POLICY_TYPE)
        .with_properties(USER_PASSWORD_POLICY_PROPERTIES),
    ObjectMembersDescriptor::new(&AUTHENTICATION_LOCK_MANAGER_TYPE)
        .with_methods(AUTHENTICATION_LOCK_MANAGER_METHODS),
    ObjectMembersDescriptor::new(&AUTHENTICATION_LOCK_SETTINGS_TYPE)
        .with_properties(AUTHENTICATION_LOCK_SETTINGS_PROPERTIES),
    ObjectMembersDescriptor::new(&RANDOM_PASSWORD_GENERATOR_TYPE)
        .with_methods(RANDOM_PASSWORD_GENERATOR_METHODS),
    ObjectMembersDescriptor::new(&INTEGRATION_SERVICES_MANAGER_TYPE)
        .with_methods(INTEGRATION_SERVICES_MANAGER_METHODS)
        .with_dynamic_properties(),
    ObjectMembersDescriptor::new(&CONFIGURATION_EXTENSION_TYPE)
        .with_properties(CONFIGURATION_EXTENSION_PROPERTIES),
    ObjectMembersDescriptor::new(&EXTENSION_APPLICATION_ISSUE_TYPE)
        .with_properties(EXTENSION_APPLICATION_ISSUE_PROPERTIES),
    ObjectMembersDescriptor::new(&QUERY_TYPE)
        .with_methods(QUERY_METHODS)
        .with_properties(QUERY_PROPERTIES),
    ObjectMembersDescriptor::new(&TEMP_TABLES_MANAGER_TYPE)
        .with_methods(TEMP_TABLES_MANAGER_METHODS),
    ObjectMembersDescriptor::new(&QUERY_RESULT_TYPE).with_methods(QUERY_RESULT_METHODS),
    ObjectMembersDescriptor::new(&catalog_selection::TYPE)
        .with_methods(catalog_selection::METHODS)
        .with_dynamic_properties(),
    ObjectMembersDescriptor::new(&QUERY_SELECTION_TYPE)
        .with_methods(QUERY_SELECTION_METHODS)
        .with_dynamic_properties(),
    ObjectMembersDescriptor::new(&VALUE_STORAGE_TYPE).with_methods(VALUE_STORAGE_METHODS),
    ObjectMembersDescriptor::new(&DATA_LOCK_TYPE).with_methods(DATA_LOCK_METHODS),
    ObjectMembersDescriptor::new(&DATA_LOCK_ITEM_TYPE),
    ObjectMembersDescriptor::new(&DATA_COMPOSITION_TEMPLATE_COMPOSER_TYPE)
        .with_methods(DATA_COMPOSITION_TEMPLATE_COMPOSER_METHODS),
    ObjectMembersDescriptor::new(&DATA_COMPOSITION_PROCESSOR_TYPE)
        .with_methods(DATA_COMPOSITION_PROCESSOR_METHODS),
    ObjectMembersDescriptor::new(&DATA_COMPOSITION_VALUE_COLLECTION_OUTPUT_PROCESSOR_TYPE)
        .with_methods(DATA_COMPOSITION_VALUE_COLLECTION_OUTPUT_PROCESSOR_METHODS)
        .with_properties(DATA_COMPOSITION_VALUE_COLLECTION_OUTPUT_PROCESSOR_PROPERTIES),
];

const OBJECT_MEMBER_GROUPS: &[&[ObjectMembersDescriptor]] = &[API_MEMBERS];

pub(crate) const LIBRARY: LibraryDescriptor =
    LibraryDescriptor::new("bsl-orm", env!("CARGO_PKG_VERSION"))
        .with_functions(FUNCTIONS)
        .with_globals(GLOBALS)
        .with_constructors(CONSTRUCTORS)
        .with_constructor_types(CONSTRUCTOR_TYPES)
        .with_types(TYPES)
        .with_object_member_groups(OBJECT_MEMBER_GROUPS);
