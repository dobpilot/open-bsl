//! Таблицы дескриптора библиотеки: конструкторы, функции, глобальные
//! свойства, типы и члены объектов, которые крейт регистрирует в рантайме.

use super::*;

const CONSTRUCTORS: &[ConstructorDescriptor] = &[
    ConstructorDescriptor {
        code: ConstructorCode::new(24),
        names: &["АдминистрированиеСервера", "ServerAdministration"],
        arity: Arity::range(0, 2),
        call: Some(construct_server_administration),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(23),
        names: &[
            "СовместноеИспользованиеПриложенийСистемыВзаимодействия",
            "CollaborationSystemApplicationLinks",
        ],
        arity: Arity::exact(0),
        call: Some(construct_collaboration_application),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(22),
        names: &[
            "КоллекцияИдентификаторовПриложенийСистемыВзаимодействия",
            "CollaborationSystemApplicationIDCollection",
        ],
        arity: Arity::exact(0),
        call: Some(construct_collaboration_application),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(21),
        names: &[
            "ИдентификаторСообщенияСистемыВзаимодействия",
            "CollaborationSystemMessageID",
        ],
        arity: Arity::exact(1),
        call: Some(construct_collaboration_application),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(20),
        names: &[
            "ИдентификаторПриложенияСистемыВзаимодействия",
            "CollaborationSystemApplicationID",
        ],
        arity: Arity::exact(1),
        call: Some(construct_collaboration_application),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(19),
        names: &["ИнтернетПочтовоеСообщение", "InternetMailMessage"],
        arity: Arity::exact(0),
        call: Some(construct_internet_mail_message),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(18),
        names: &[
            "КонтекстОбсужденияСистемыВзаимодействия",
            "CollaborationSystemConversationContext",
        ],
        arity: Arity::exact(1),
        call: Some(construct_collaboration_conversation_context),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(17),
        names: &[
            "ИдентификаторОбсужденияСистемыВзаимодействия",
            "CollaborationSystemConversationID",
        ],
        arity: Arity::exact(1),
        call: Some(construct_collaboration_conversation_id),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(15),
        names: &[
            "ОтборОбсужденийСистемыВзаимодействия",
            "CollaborationSystemConversationsFilter",
        ],
        arity: Arity::exact(0),
        call: Some(construct_collaboration_filter),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(16),
        names: &[
            "ОтборСообщенийСистемыВзаимодействия",
            "CollaborationSystemMessagesFilter",
        ],
        arity: Arity::exact(0),
        call: Some(construct_collaboration_filter),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(1),
        names: &["СистемнаяИнформация", "SystemInfo"],
        arity: Arity::exact(0),
        call: Some(construct_system_information),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(2),
        names: &["НастройкиНачальнойСтраницы", "HomePageSettings"],
        arity: Arity::exact(0),
        call: Some(construct_home_page_settings),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(3),
        names: &["СоставФормНачальнойСтраницы", "HomePageForms"],
        arity: Arity::exact(0),
        call: Some(construct_home_page_forms),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(4),
        names: &["ОписаниеНастроек", "SettingsDescription"],
        arity: Arity::exact(0),
        call: Some(construct_settings_description),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(5),
        names: &["РасписаниеРегламентногоЗадания", "JobSchedule"],
        arity: Arity::exact(0),
        call: Some(construct_job_schedule),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(6),
        names: &[
            "ОписаниеЗащитыОтОпасныхДействий",
            "UnsafeOperationProtectionDescription",
        ],
        arity: Arity::exact(0),
        call: Some(construct_unsafe_operation_protection_description),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(7),
        names: &["ИспользованиеСобытияЖурналаРегистрации", "EventLogEventUse"],
        arity: Arity::range(0, 2),
        call: Some(construct_event_log_event_use),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(8),
        names: &["НастройкиКлиентскогоПриложения", "ClientSettings"],
        arity: Arity::exact(0),
        call: Some(construct_client_settings),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(9),
        names: &["НастройкиКомандногоИнтерфейса", "CommandInterfaceSettings"],
        arity: Arity::exact(0),
        call: Some(construct_command_interface_settings),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(10),
        names: &[
            "НастройкиИнтерфейсаКлиентскогоПриложения",
            "ClientApplicationInterfaceSettings",
        ],
        arity: Arity::exact(0),
        call: Some(construct_client_application_interface_settings),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(11),
        names: &[
            "НастройкиСоставаИнтерфейсаКлиентскогоПриложения",
            "ClientApplicationInterfaceContentSettings",
        ],
        arity: Arity::exact(0),
        call: Some(construct_client_application_interface_content_settings),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(12),
        names: &[
            "ГруппаНастройкиСоставаИнтерфейсаКлиентскогоПриложения",
            "ClientApplicationInterfaceContentSettingsGroup",
        ],
        arity: Arity::exact(0),
        call: Some(construct_client_application_interface_content_settings_group),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(13),
        names: &[
            "ЭлементНастройкиСоставаИнтерфейсаКлиентскогоПриложения",
            "ClientApplicationInterfaceContentSettingsItem",
        ],
        arity: Arity::exact(1),
        call: Some(construct_client_application_interface_content_settings_item),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(14),
        names: &[
            "ОписаниеИспользованияСобытияДоступЖурналаРегистрации",
            "EventLogAccessEventUseDescription",
        ],
        arity: Arity::range(0, 1),
        call: Some(construct_event_log_access_event_use_description),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(25),
        names: &["ЭлементПланировщика", "PlannerItem"],
        arity: Arity::exact(0),
        call: Some(planner_item::construct),
    },
];

const CONSTRUCTOR_TYPES: &[(ConstructorCode, &TypeDescriptor)] = &[
    (ConstructorCode::new(1), &SYSTEM_INFORMATION_TYPE),
    (ConstructorCode::new(2), &HOME_PAGE_SETTINGS_TYPE),
    (ConstructorCode::new(3), &HOME_PAGE_FORMS_TYPE),
    (ConstructorCode::new(4), &SETTINGS_DESCRIPTION_TYPE),
    (ConstructorCode::new(5), &JOB_SCHEDULE_TYPE),
    (
        ConstructorCode::new(6),
        &UNSAFE_OPERATION_PROTECTION_DESCRIPTION_TYPE,
    ),
    (ConstructorCode::new(7), &EVENT_LOG_EVENT_USE_TYPE),
    (ConstructorCode::new(8), &CLIENT_SETTINGS_TYPE),
    (ConstructorCode::new(9), &COMMAND_INTERFACE_SETTINGS_TYPE),
    (
        ConstructorCode::new(10),
        &CLIENT_APPLICATION_INTERFACE_SETTINGS_TYPE,
    ),
    (
        ConstructorCode::new(11),
        &CLIENT_APPLICATION_INTERFACE_CONTENT_SETTINGS_TYPE,
    ),
    (
        ConstructorCode::new(12),
        &CLIENT_APPLICATION_INTERFACE_CONTENT_SETTINGS_GROUP_TYPE,
    ),
    (
        ConstructorCode::new(13),
        &CLIENT_APPLICATION_INTERFACE_CONTENT_SETTINGS_ITEM_TYPE,
    ),
    (
        ConstructorCode::new(14),
        &EVENT_LOG_ACCESS_EVENT_USE_DESCRIPTION_TYPE,
    ),
    (ConstructorCode::new(25), &planner_item::TYPE),
    (
        ConstructorCode::new(17),
        &COLLABORATION_CONVERSATION_ID_TYPE,
    ),
];

const FUNCTIONS: &[FunctionDescriptor] = &[
    FunctionDescriptor {
        code: FunctionCode::new(61),
        names: &["КодЛокализацииИнформационнойБазы", "InfoBaseLocaleCode"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(infobase_locale_code),
    },
    FunctionDescriptor {
        code: FunctionCode::new(60),
        names: &[
            "ТекущийВариантИнтерфейсаКлиентскогоПриложения",
            "ClientApplicationInterfaceCurrentVariant",
        ],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(current_client_interface_variant),
    },
    FunctionDescriptor {
        code: FunctionCode::new(58),
        names: &["ПолучитьБлокировкуСеансов", "GetSessionsLock"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(get_sessions_lock),
    },
    FunctionDescriptor {
        code: FunctionCode::new(59),
        names: &["УстановитьБлокировкуСеансов", "SetSessionsLock"],
        arity: Arity::exact(1),
        kind: FunctionKind::Procedure,
        call: Some(set_sessions_lock),
    },
    FunctionDescriptor {
        code: FunctionCode::new(57),
        names: &[
            "ДополнительныеНастройкиАутентификации",
            "AdditionalAuthenticationSettings",
        ],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(additional_authentication_settings),
    },
    FunctionDescriptor {
        code: FunctionCode::new(56),
        names: &[
            "УстановитьНастройкиКлиентаЛицензирования",
            "SetLicensingClientParameters",
        ],
        arity: Arity::range(0, 3),
        kind: FunctionKind::Procedure,
        call: Some(set_licensing_client_parameters),
    },
    FunctionDescriptor {
        code: FunctionCode::new(55),
        names: &[
            "ПолучитьДополнительныйПараметрКлиентаЛицензирования",
            "GetLicensingClientAdditionalParameter",
        ],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(get_licensing_client_additional_parameter),
    },
    FunctionDescriptor {
        code: FunctionCode::new(54),
        names: &["ПолучитьИдентификаторКонфигурации", "GetConfigurationID"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(get_configuration_id),
    },
    FunctionDescriptor {
        code: FunctionCode::new(53),
        names: &["ПолучитьИмяКлиентаЛицензирования", "GetLicensingClientName"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(get_licensing_client_name),
    },
    FunctionDescriptor {
        code: FunctionCode::new(52),
        names: &["ПолучитьРазделительПутиСервера", "GetServerPathSeparator"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(get_server_path_separator),
    },
    FunctionDescriptor {
        code: FunctionCode::new(51),
        names: &["ПолнотекстовыйПоиск", "FullTextSearch"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(full_text_search),
    },
    FunctionDescriptor {
        code: FunctionCode::new(50),
        names: &["ПредставлениеКодаЛокализации", "LocaleCodePresentation"],
        arity: Arity::exact(1),
        kind: FunctionKind::Function,
        call: Some(locale_code_presentation),
    },
    FunctionDescriptor {
        code: FunctionCode::new(49),
        names: &[
            "ПолучитьДопустимыеКодыЛокализации",
            "GetAvailableLocaleCodes",
        ],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(available_locale_codes),
    },
    FunctionDescriptor {
        code: FunctionCode::new(48),
        names: &["ПараметрыДоступа", "AccessParameters"],
        arity: Arity::range(3, 4),
        kind: FunctionKind::Function,
        call: Some(access_parameters),
    },
    FunctionDescriptor {
        code: FunctionCode::new(47),
        names: &["НайтиПомеченныеНаУдаление", "FindMarkedForDeletion"],
        arity: Arity::range(0, 3),
        kind: FunctionKind::Function,
        call: Some(find_marked_for_deletion),
    },
    FunctionDescriptor {
        code: FunctionCode::new(39),
        names: &["ПолучитьИсториюРаботыПользователяСеанса"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(user_work_history),
    },
    FunctionDescriptor {
        code: FunctionCode::new(38),
        names: &["ПолучитьХранилищеВариантовОтчетовСеанса"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(reports_variants_storage),
    },
    FunctionDescriptor {
        code: FunctionCode::new(35),
        names: &["ВыгрузитьЖурналРегистрации", "UnloadEventLog"],
        arity: Arity::range(1, 5),
        kind: FunctionKind::Procedure,
        call: Some(unload_event_log),
    },
    FunctionDescriptor {
        code: FunctionCode::new(34),
        names: &[
            "ПолучитьНавигационнуюСсылкуИнформационнойБазы",
            "GetInfoBaseURL",
        ],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(infobase_url),
    },
    FunctionDescriptor {
        code: FunctionCode::new(33),
        names: &[
            "ПолучитьСкоростьКлиентскогоСоединения",
            "GetClientConnectionSpeed",
        ],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(client_connection_speed),
    },
    FunctionDescriptor {
        code: FunctionCode::new(32),
        names: &["НомерСеансаИнформационнойБазы", "InfoBaseSessionNumber"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(infobase_session_number),
    },
    FunctionDescriptor {
        code: FunctionCode::new(31),
        names: &["СистемаВзаимодействия", "CollaborationSystem"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(collaboration_system),
    },
    FunctionDescriptor {
        code: FunctionCode::new(30),
        names: &["РегламентныеЗадания", "ScheduledJobs"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(scheduled_jobs),
    },
    FunctionDescriptor {
        code: FunctionCode::new(29),
        names: &["ПолучитьСеансыИнформационнойБазы", "GetInfoBaseSessions"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(infobase_sessions),
    },
    FunctionDescriptor {
        code: FunctionCode::new(28),
        names: &["ИмяКомпьютера", "ComputerName"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(computer_name),
    },
    FunctionDescriptor {
        code: FunctionCode::new(27),
        names: &[
            "ПолучитьИнформациюЭкрановКлиента",
            "GetClientDisplaysInformation",
        ],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(client_displays_information),
    },
    FunctionDescriptor {
        code: FunctionCode::new(1),
        names: &["ПолучитьПараметрыСеанса"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(parameters),
    },
    FunctionDescriptor {
        code: FunctionCode::new(2),
        names: &["ТекущийРежимЗапуска", "CurrentRunMode"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(current_run_mode),
    },
    FunctionDescriptor {
        code: FunctionCode::new(3),
        names: &[
            "СтрокаСоединенияИнформационнойБазы",
            "InfoBaseConnectionString",
        ],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(infobase_connection_string),
    },
    FunctionDescriptor {
        code: FunctionCode::new(4),
        names: &[
            "ПолучитьТекущийСеансИнформационнойБазы",
            "GetCurrentInfoBaseSession",
        ],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(current_infobase_session),
    },
    FunctionDescriptor {
        code: FunctionCode::new(5),
        names: &[
            "НомерСоединенияИнформационнойБазы",
            "InfoBaseConnectionNumber",
        ],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(infobase_connection_number),
    },
    FunctionDescriptor {
        code: FunctionCode::new(6),
        names: &["УстановитьБезопасныйРежим", "SetSafeMode"],
        arity: Arity::exact(1),
        kind: FunctionKind::Procedure,
        call: Some(set_safe_mode),
    },
    FunctionDescriptor {
        code: FunctionCode::new(7),
        names: &["БезопасныйРежим", "SafeMode"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(safe_mode),
    },
    FunctionDescriptor {
        code: FunctionCode::new(8),
        names: &["УстановитьПривилегированныйРежим", "SetPrivilegedMode"],
        arity: Arity::exact(1),
        kind: FunctionKind::Procedure,
        call: Some(set_privileged_mode),
    },
    FunctionDescriptor {
        code: FunctionCode::new(9),
        names: &["ПривилегированныйРежим", "PrivilegedMode"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(privileged_mode),
    },
    FunctionDescriptor {
        code: FunctionCode::new(10),
        names: &[
            "ПолучитьСоединенияИнформационнойБазы",
            "GetInfoBaseConnections",
        ],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(infobase_connections),
    },
    FunctionDescriptor {
        code: FunctionCode::new(14),
        names: &[
            "УстановитьОтключениеБезопасногоРежима",
            "SetSafeModeDisabled",
        ],
        arity: Arity::exact(1),
        kind: FunctionKind::Procedure,
        call: Some(set_safe_mode_disabled),
    },
    FunctionDescriptor {
        code: FunctionCode::new(15),
        names: &[
            "ОбновитьПовторноИспользуемыеЗначения",
            "RefreshReusableValues",
        ],
        arity: Arity::exact(0),
        kind: FunctionKind::Procedure,
        call: Some(refresh_reusable_values),
    },
    FunctionDescriptor {
        code: FunctionCode::new(16),
        names: &["ПолучитьОбработкуОшибокСеанса"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(error_processing),
    },
    FunctionDescriptor {
        code: FunctionCode::new(17),
        names: &[
            "КонфигурацияБазыДанныхИзмененаДинамически",
            "DataBaseConfigurationChangedDynamically",
        ],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(database_configuration_changed_dynamically),
    },
    FunctionDescriptor {
        code: FunctionCode::new(18),
        names: &["ПолучитьОтключениеБезопасногоРежима", "GetSafeModeDisabled"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(get_safe_mode_disabled),
    },
    FunctionDescriptor {
        code: FunctionCode::new(11),
        names: &["ЗаписьЖурналаРегистрации", "WriteLogEvent"],
        arity: Arity::range(1, 6),
        kind: FunctionKind::Procedure,
        call: Some(write_log_event),
    },
    FunctionDescriptor {
        code: FunctionCode::new(12),
        names: &["ПолучитьХранилищеОбщихНастроек"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(common_settings_storage),
    },
    FunctionDescriptor {
        code: FunctionCode::new(13),
        names: &["ПолучитьХранилищеСистемныхНастроек"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(system_settings_storage),
    },
    FunctionDescriptor {
        code: FunctionCode::new(19),
        names: &["ПолучитьХранилищеНастроекДанныхФорм"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(form_data_settings_storage),
    },
    FunctionDescriptor {
        code: FunctionCode::new(25),
        names: &[
            "ПолучитьСтандартноеХранилищеПользовательскихНастроекОтчетов",
            "GetStandardReportsUserSettingsStorage",
        ],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(reports_user_settings_storage),
    },
    FunctionDescriptor {
        code: FunctionCode::new(26),
        names: &[
            "ПолучитьСтандартноеХранилищеПользовательскихНастроекДинамическихСписков",
            "GetStandardDynamicListsUserSettingsStorage",
        ],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(dynamic_lists_user_settings_storage),
    },
    FunctionDescriptor {
        code: FunctionCode::new(20),
        names: &[
            "УстановитьБезопасныйРежимРазделенияДанных",
            "SetDataSeparationSafeMode",
        ],
        arity: Arity::exact(2),
        kind: FunctionKind::Procedure,
        call: Some(set_data_separation_safe_mode),
    },
    FunctionDescriptor {
        code: FunctionCode::new(36),
        names: &["БезопасныйРежимРазделенияДанных", "DataSeparationSafeMode"],
        arity: Arity::exact(1),
        kind: FunctionKind::Function,
        call: Some(data_separation_safe_mode),
    },
    FunctionDescriptor {
        code: FunctionCode::new(21),
        names: &["ПодключитьВнешнююКомпоненту", "AttachAddIn"],
        arity: Arity::range(1, 4),
        kind: FunctionKind::Function,
        call: Some(attach_add_in),
    },
    FunctionDescriptor {
        code: FunctionCode::new(22),
        names: &[
            "ПолучитьИспользованиеСобытияЖурналаРегистрации",
            "GetEventLogEventUse",
        ],
        arity: Arity::exact(1),
        kind: FunctionKind::Function,
        call: Some(get_event_log_event_use),
    },
    FunctionDescriptor {
        code: FunctionCode::new(23),
        names: &[
            "УстановитьИспользованиеСобытияЖурналаРегистрации",
            "SetEventLogEventUse",
        ],
        arity: Arity::exact(2),
        kind: FunctionKind::Procedure,
        call: Some(set_event_log_event_use),
    },
    FunctionDescriptor {
        code: FunctionCode::new(24),
        names: &[
            "ПолучитьИспользованиеЖурналаРегистрации",
            "GetEventLogUsing",
        ],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(get_event_log_using),
    },
    FunctionDescriptor {
        code: FunctionCode::new(40),
        names: &["КлиентскоеПриложение", "ClientApplication"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(client_application),
    },
    FunctionDescriptor {
        code: FunctionCode::new(41),
        names: &["ТекущийКодЛокализации", "CurrentLocaleCode"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(current_locale_code),
    },
    FunctionDescriptor {
        code: FunctionCode::new(42),
        names: &["ТекущийЯзыкСистемы", "CurrentSystemLanguage"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(current_system_language),
    },
    FunctionDescriptor {
        code: FunctionCode::new(43),
        names: &["ПолучитьРежимВнешнихРесурсов", "GetExternalResourcesMode"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(get_external_resources_mode),
    },
    FunctionDescriptor {
        code: FunctionCode::new(44),
        names: &[
            "ОтправкаДоставляемыхУведомлений",
            "DeliverableNotificationSend",
        ],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(deliverable_notification_send),
    },
    FunctionDescriptor {
        code: FunctionCode::new(45),
        names: &["СредстваКриптографии", "CryptoTools"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(crypto_tools),
    },
    FunctionDescriptor {
        code: FunctionCode::new(46),
        names: &[
            "ПолучитьЧасовойПоясИнформационнойБазы",
            "GetInfoBaseTimeZone",
        ],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(get_infobase_time_zone),
    },
];

const GLOBALS: &[GlobalDescriptor] = &[
    GlobalDescriptor {
        names: &[
            "ДополнительныеНастройкиАутентификации",
            "AdditionalAuthenticationSettings",
        ],
        function: FunctionCode::new(57),
    },
    GlobalDescriptor {
        names: &["ПолнотекстовыйПоиск", "FullTextSearch"],
        function: FunctionCode::new(51),
    },
    GlobalDescriptor {
        names: &["СредстваКриптографии", "CryptoTools"],
        function: FunctionCode::new(45),
    },
    GlobalDescriptor {
        names: &[
            "ОтправкаДоставляемыхУведомлений",
            "DeliverableNotificationSend",
        ],
        function: FunctionCode::new(44),
    },
    GlobalDescriptor {
        names: &["КлиентскоеПриложение", "ClientApplication"],
        function: FunctionCode::new(40),
    },
    GlobalDescriptor {
        names: &["ИсторияРаботыПользователя", "UserWorkHistory"],
        function: FunctionCode::new(39),
    },
    GlobalDescriptor {
        names: &["ХранилищеВариантовОтчетов", "ReportsVariantsStorage"],
        function: FunctionCode::new(38),
    },
    GlobalDescriptor {
        names: &["СистемаВзаимодействия", "CollaborationSystem"],
        function: FunctionCode::new(31),
    },
    GlobalDescriptor {
        names: &["РегламентныеЗадания", "ScheduledJobs"],
        function: FunctionCode::new(30),
    },
    GlobalDescriptor {
        names: &["ПараметрыСеанса", "SessionParameters"],
        function: FunctionCode::new(1),
    },
    GlobalDescriptor {
        names: &["ХранилищеОбщихНастроек", "CommonSettingsStorage"],
        function: FunctionCode::new(12),
    },
    GlobalDescriptor {
        names: &["ХранилищеСистемныхНастроек", "SystemSettingsStorage"],
        function: FunctionCode::new(13),
    },
    GlobalDescriptor {
        names: &["ХранилищеНастроекДанныхФорм", "FormDataSettingsStorage"],
        function: FunctionCode::new(19),
    },
    GlobalDescriptor {
        names: &[
            "ХранилищеПользовательскихНастроекОтчетов",
            "ReportsUserSettingsStorage",
        ],
        function: FunctionCode::new(25),
    },
    GlobalDescriptor {
        names: &[
            "ХранилищеПользовательскихНастроекДинамическихСписков",
            "DynamicListsUserSettingsStorage",
        ],
        function: FunctionCode::new(26),
    },
    GlobalDescriptor {
        names: &["ОбработкаОшибок", "ErrorProcessing"],
        function: FunctionCode::new(16),
    },
];

const TYPES: &[&TypeDescriptor] = &[
    &COLLABORATION_CONVERSATION_ID_TYPE,
    &PARAMETERS_TYPE,
    &SYSTEM_INFORMATION_TYPE,
    &INFOBASE_SESSION_TYPE,
    &INFOBASE_CONNECTION_TYPE,
    &STANDARD_SETTINGS_STORAGE_TYPE,
    &STANDARD_SETTINGS_SELECTION_TYPE,
    &HOME_PAGE_SETTINGS_TYPE,
    &HOME_PAGE_FORMS_TYPE,
    &ERROR_PROCESSING_MANAGER_TYPE,
    &ERROR_PROCESSING_SETTINGS_TYPE,
    &SETTINGS_DESCRIPTION_TYPE,
    &JOB_SCHEDULE_TYPE,
    &UNSAFE_OPERATION_PROTECTION_DESCRIPTION_TYPE,
    &EVENT_LOG_EVENT_USE_TYPE,
    &EVENT_LOG_ACCESS_EVENT_USE_DESCRIPTION_TYPE,
    &CLIENT_SETTINGS_TYPE,
    &CLIENT_APPLICATION_TYPE,
    &COMMAND_INTERFACE_SETTINGS_TYPE,
    &CLIENT_APPLICATION_INTERFACE_SETTINGS_TYPE,
    &CLIENT_APPLICATION_INTERFACE_CONTENT_SETTINGS_TYPE,
    &CLIENT_APPLICATION_INTERFACE_CONTENT_SETTINGS_GROUP_TYPE,
    &CLIENT_APPLICATION_INTERFACE_CONTENT_SETTINGS_ITEM_TYPE,
    &CLIENT_DISPLAY_INFORMATION_TYPE,
    &planner_item::TYPE,
];

const API_MEMBERS: &[ObjectMembersDescriptor] = &[
    ObjectMembersDescriptor::new(&PARAMETERS_TYPE)
        .with_methods(PARAMETERS_METHODS)
        .with_dynamic_properties(),
    ObjectMembersDescriptor::new(&SYSTEM_INFORMATION_TYPE)
        .with_properties(SYSTEM_INFORMATION_PROPERTIES),
    ObjectMembersDescriptor::new(&INFOBASE_SESSION_TYPE)
        .with_methods(INFOBASE_SESSION_METHODS)
        .with_properties(INFOBASE_SESSION_PROPERTIES),
    ObjectMembersDescriptor::new(&INFOBASE_CONNECTION_TYPE)
        .with_properties(INFOBASE_CONNECTION_PROPERTIES),
    ObjectMembersDescriptor::new(&STANDARD_SETTINGS_STORAGE_TYPE)
        .with_methods(STANDARD_SETTINGS_STORAGE_METHODS),
    ObjectMembersDescriptor::new(&STANDARD_SETTINGS_SELECTION_TYPE)
        .with_methods(STANDARD_SETTINGS_SELECTION_METHODS)
        .with_properties(STANDARD_SETTINGS_SELECTION_PROPERTIES),
    ObjectMembersDescriptor::new(&HOME_PAGE_SETTINGS_TYPE).with_methods(HOME_PAGE_SETTINGS_METHODS),
    ObjectMembersDescriptor::new(&HOME_PAGE_FORMS_TYPE).with_properties(HOME_PAGE_FORMS_PROPERTIES),
    ObjectMembersDescriptor::new(&ERROR_PROCESSING_MANAGER_TYPE)
        .with_methods(ERROR_PROCESSING_MANAGER_METHODS),
    ObjectMembersDescriptor::new(&ERROR_PROCESSING_SETTINGS_TYPE)
        .with_properties(ERROR_PROCESSING_SETTINGS_PROPERTIES),
    ObjectMembersDescriptor::new(&SETTINGS_DESCRIPTION_TYPE)
        .with_properties(SETTINGS_DESCRIPTION_PROPERTIES),
    ObjectMembersDescriptor::new(&JOB_SCHEDULE_TYPE).with_properties(JOB_SCHEDULE_PROPERTIES),
    ObjectMembersDescriptor::new(&UNSAFE_OPERATION_PROTECTION_DESCRIPTION_TYPE)
        .with_properties(UNSAFE_OPERATION_PROTECTION_DESCRIPTION_PROPERTIES),
    ObjectMembersDescriptor::new(&EVENT_LOG_EVENT_USE_TYPE)
        .with_properties(EVENT_LOG_EVENT_USE_PROPERTIES),
    ObjectMembersDescriptor::new(&EVENT_LOG_ACCESS_EVENT_USE_DESCRIPTION_TYPE)
        .with_properties(EVENT_LOG_ACCESS_EVENT_USE_DESCRIPTION_PROPERTIES),
    ObjectMembersDescriptor::new(&CLIENT_SETTINGS_TYPE).with_properties(CLIENT_SETTINGS_PROPERTIES),
    ObjectMembersDescriptor::new(&CLIENT_APPLICATION_TYPE).with_methods(CLIENT_APPLICATION_METHODS),
    ObjectMembersDescriptor::new(&COMMAND_INTERFACE_SETTINGS_TYPE)
        .with_properties(COMMAND_INTERFACE_SETTINGS_PROPERTIES),
    ObjectMembersDescriptor::new(&CLIENT_APPLICATION_INTERFACE_SETTINGS_TYPE)
        .with_methods(CLIENT_APPLICATION_INTERFACE_SETTINGS_METHODS),
    ObjectMembersDescriptor::new(&CLIENT_APPLICATION_INTERFACE_CONTENT_SETTINGS_TYPE)
        .with_properties(CLIENT_APPLICATION_INTERFACE_CONTENT_SETTINGS_PROPERTIES),
    ObjectMembersDescriptor::new(&CLIENT_APPLICATION_INTERFACE_CONTENT_SETTINGS_GROUP_TYPE)
        .with_methods(CLIENT_APPLICATION_INTERFACE_CONTENT_SETTINGS_GROUP_METHODS),
    ObjectMembersDescriptor::new(&CLIENT_APPLICATION_INTERFACE_CONTENT_SETTINGS_ITEM_TYPE)
        .with_properties(CLIENT_APPLICATION_INTERFACE_CONTENT_SETTINGS_ITEM_PROPERTIES),
    ObjectMembersDescriptor::new(&CLIENT_DISPLAY_INFORMATION_TYPE)
        .with_properties(CLIENT_DISPLAY_INFORMATION_PROPERTIES),
    ObjectMembersDescriptor::new(&planner_item::TYPE).with_properties(planner_item::PROPERTIES),
];

const OBJECT_MEMBER_GROUPS: &[&[ObjectMembersDescriptor]] = &[API_MEMBERS];

pub(crate) const LIBRARY: LibraryDescriptor =
    LibraryDescriptor::new("bsl-session", env!("CARGO_PKG_VERSION"))
        .with_functions(FUNCTIONS)
        .with_constructors(CONSTRUCTORS)
        .with_constructor_types(CONSTRUCTOR_TYPES)
        .with_globals(GLOBALS)
        .with_types(TYPES)
        .with_object_member_groups(OBJECT_MEMBER_GROUPS);
