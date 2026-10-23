use super::*;
use std::sync::Arc;

use bsl_config::{ConfigurationCodeSnapshot, ConfigurationModule, ModuleExecution, ModuleKind};
use bsl_orm::{DataAccess, MetadataCatalog, MetadataObject, Principal};
use open_bsl::{Engine, Error};

fn host_data(parameters: Rc<SessionParameters>) -> Rc<SessionData> {
    Rc::new(
        SessionData::new(
            Arc::new(MetadataCatalog::new("configuration", vec![]).unwrap()),
            Principal::new("user", "Тест", Vec::new()),
        )
        .with_extension(Rc::new(SessionRuntime {
            parameters,
            kind: SessionKind::ThinClient,
            platform_version: Arc::from(PLATFORM_VERSION),
            connection_string: None,
            connection_number: 1,
            session_number: None,
            session_started: None,
            connections: None,
            event_log_writer: None,
            client_settings: None,
            error_processing_settings: None,
            database_configuration_changed_dynamically: None,
            data_separation_common_attributes: None,
            client_displays: None,
            client_connection_speed: None,
            current_client_interface_variant: None,
            computer_name: None,
            locale_code: None,
            infobase_locale_code: None,
            system_language: None,
            external_resources_mode: None,
            background_method: None,
        })),
    )
}

fn engine(handler: &str) -> Engine {
    Engine::builder()
        .register_library(bsl_orm::library())
        .register_library(library())
        .common_module("МодульСеанса", handler)
        .build()
        .unwrap()
}

#[test]
fn session_lock_surfaces_resolve_without_fabricating_a_database_write() {
    let engine = engine("");
    let mut state = engine.state_builder().build();
    for source in [
        "Возврат Новый БлокировкаСеансов;",
        "Возврат Новый SessionsLock;",
        "Возврат ПолучитьБлокировкуСеансов();",
        "Возврат GetSessionsLock();",
        "УстановитьБлокировкуСеансов(Неопределено);",
        "SetSessionsLock(Неопределено);",
    ] {
        assert!(engine.compile(source).is_ok(), "{source}");
        assert!(
            state
                .exec(source)
                .unwrap_err()
                .to_string()
                .to_lowercase()
                .contains("блокиров")
        );
    }
}

#[test]
fn error_message_texts_constructor_uses_measured_arity_and_rejects_missing_value() {
    let engine = engine("");
    let mut state = engine.state_builder().build();
    for source in [
        "Возврат Новый ТекстыСообщенияОбОшибке(\"первый\", \"второй\");",
        "Возврат Новый ErrorMessageTexts(\"first\", \"second\");",
    ] {
        assert!(engine.compile(source).is_ok(), "{source}");
        assert!(
            state
                .exec(source)
                .unwrap_err()
                .to_string()
                .contains("ТекстыСообщенияОбОшибке")
        );
    }
    for source in [
        "Возврат Новый ТекстыСообщенияОбОшибке();",
        "Возврат Новый ТекстыСообщенияОбОшибке(\"один\");",
        "Возврат Новый ТекстыСообщенияОбОшибке(\"один\", \"два\", \"три\");",
    ] {
        assert!(engine.compile(source).is_err(), "{source}");
    }
}

fn entry(engine: &Engine, source: &str) -> open_bsl::Module {
    engine.compile(source).unwrap()
}

#[test]
fn external_component_attachment_compiles_all_forms_and_fails_explicitly() {
    let engine = engine("");
    for source in [
        r#"Возврат ПодключитьВнешнююКомпоненту("Prog.Id");"#,
        r#"Возврат ПодключитьВнешнююКомпоненту("Место", "Имя");"#,
        r#"Возврат ПодключитьВнешнююКомпоненту("Место", "Имя", Неопределено);"#,
        r#"Возврат AttachAddIn("Place", "Name", Неопределено, Неопределено);"#,
    ] {
        let module = entry(&engine, source);
        let mut state = engine.state_builder().build();
        let error = state.run(&module).unwrap_err().to_string();
        assert!(error.contains("загрузчик внешних компонент"), "{error}");
    }
    let error = engine
        .compile(
            r#"Возврат ПодключитьВнешнююКомпоненту("Место", "Имя", Неопределено, Неопределено, Неопределено);"#,
        )
        .err()
        .expect("пятый аргумент отвергается")
        .to_string();
    assert!(error.contains("принимает аргументов: 4"), "{error}");
}

#[test]
fn notification_sending_global_resolves_and_fails_explicitly_without_transport() {
    let engine = engine("");
    engine
        .compile(
            "Процедура НеВызывается()\nУ = ОтправкаДоставляемыхУведомлений;\nКонецПроцедуры\nВозврат 1;",
        )
        .unwrap();
    engine
        .compile("Процедура НеВызывается()\nУ = DeliverableNotificationSend;\nКонецПроцедуры")
        .unwrap();
    let mut state = engine.state_builder().build();
    assert_eq!(
        state
            .exec(
                "Попытка\nУ = ОтправкаДоставляемыхУведомлений;\nИсключение\nВозврат Истина;\nКонецПопытки;\nВозврат Ложь;",
            )
            .unwrap(),
        Value::Boolean(true)
    );
}

#[test]
fn crypto_tools_global_resolves_and_fails_explicitly_without_provider() {
    let engine = engine("");
    for source in [
        "Процедура НеВызывается()\nСведения = СредстваКриптографии.ПолучитьИнформациюМодуляКриптографии();\nКонецПроцедуры",
        "Процедура НеВызывается()\nСведения = CryptoTools.GetCryptoModuleInformation();\nКонецПроцедуры",
    ] {
        engine.compile(source).unwrap();
    }
    let mut state = engine.state_builder().build();
    assert_eq!(
        state
            .exec(
                "Попытка\nУ = СредстваКриптографии;\nИсключение\nВозврат Истина;\nКонецПопытки;\nВозврат Ложь;",
            )
            .unwrap(),
        Value::Boolean(true)
    );
}

#[test]
fn additional_authentication_settings_resolves_without_fabricating_policy() {
    let engine = engine("");
    for source in [
        "Процедура НеВызывается()\nНастройки = ДополнительныеНастройкиАутентификации.ПолучитьНастройкиВосстановленияПароля();\nКонецПроцедуры",
        "Процедура НеВызывается()\nSettings = AdditionalAuthenticationSettings.GetPasswordRecoverySettings();\nКонецПроцедуры",
    ] {
        engine.compile(source).unwrap();
    }
    let mut state = engine.state_builder().build();
    assert_eq!(
        state
            .exec(
                "Попытка\nНастройки = ДополнительныеНастройкиАутентификации;\nИсключение\nВозврат Истина;\nКонецПопытки;\nВозврат Ложь;",
            )
            .unwrap(),
        Value::Boolean(true)
    );
}

#[test]
fn infobase_time_zone_resolves_but_does_not_use_the_host_zone() {
    let engine = engine("");
    for name in [
        "ПолучитьЧасовойПоясИнформационнойБазы",
        "GetInfoBaseTimeZone",
    ] {
        engine
            .compile(&format!(
                "Процедура НеВызывается()\nПояс = {name}();\nКонецПроцедуры"
            ))
            .unwrap();
    }
    let mut state = engine.state_builder().build();
    assert_eq!(
        state
            .exec(
                "Попытка\nПояс = ПолучитьЧасовойПоясИнформационнойБазы();\nИсключение\nВозврат Истина;\nКонецПопытки;\nВозврат Ложь;",
            )
            .unwrap(),
        Value::Boolean(true)
    );
    assert!(
        engine
            .compile("Пояс = ПолучитьЧасовойПоясИнформационнойБазы(1);")
            .is_err()
    );
}

#[test]
fn marked_for_deletion_search_accepts_omissions_without_fabricating_results() {
    let engine = engine("");
    engine
        .compile(
            "Процедура НеВызывается()\nОтборМетаданных = Новый Массив;\nИсключения = Новый Массив;\nНайденные = НайтиПомеченныеНаУдаление(, ОтборМетаданных, Исключения);\nКонецПроцедуры",
        )
        .unwrap();
    engine
        .compile("Процедура НеВызывается()\nНайденные = findmarkedfordeletion();\nКонецПроцедуры")
        .unwrap();
    let mut state = engine.state_builder().build();
    assert_eq!(
        state
            .exec(
                "Попытка\nНайденные = НайтиПомеченныеНаУдаление();\nИсключение\nВозврат Истина;\nКонецПопытки;\nВозврат Ложь;",
            )
            .unwrap(),
        Value::Boolean(true)
    );
    assert!(
        engine
            .compile("Найденные = FindMarkedForDeletion(1, 2, 3, 4);")
            .is_err()
    );
}

#[test]
fn access_parameters_resolves_documented_forms_without_fabricating_rights() {
    let engine = engine("");
    for source in [
        "П = ПараметрыДоступа(\"Чтение\", Неопределено, \"Ссылка\");",
        "П = accessparameters(\"Read\", Неопределено, \"Ref\", Неопределено);",
    ] {
        engine
            .compile(&format!(
                "Процедура НеВызывается()\n{source}\nКонецПроцедуры"
            ))
            .unwrap();
    }
    let mut state = engine.state_builder().build();
    assert_eq!(
        state
            .exec(
                "Попытка\nП = ПараметрыДоступа(\"Чтение\", Неопределено, \"Ссылка\");\nИсключение\nВозврат Истина;\nКонецПопытки;\nВозврат Ложь;",
            )
            .unwrap(),
        Value::Boolean(true)
    );
    assert!(
        engine
            .compile("П = ПараметрыДоступа(\"Чтение\", Неопределено);")
            .is_err()
    );
    assert!(
        engine
            .compile("П = AccessParameters(1, 2, 3, 4, 5);")
            .is_err()
    );
}

#[test]
fn locale_catalog_calls_resolve_without_fabricating_platform_data() {
    let engine = engine("");
    engine
        .compile(
            "Процедура НеВызывается()\nКоды = ПолучитьДопустимыеКодыЛокализации();\nИмя = ПредставлениеКодаЛокализации(\"ru_RU\");\nКонецПроцедуры",
        )
        .unwrap();
    engine
        .compile("Коды = getavailablelocalecodes(); Имя = localecodepresentation(\"ru_RU\");")
        .unwrap();
    let mut state = engine.state_builder().build();
    for source in [
        "ПолучитьДопустимыеКодыЛокализации()",
        "LocaleCodePresentation(\"ru_RU\")",
    ] {
        let code = format!(
            "Попытка\nЗначение = {source};\nИсключение\nВозврат Истина;\nКонецПопытки;\nВозврат Ложь;"
        );
        assert_eq!(state.exec(&code).unwrap(), Value::Boolean(true));
    }
    assert!(
        engine
            .compile("Коды = GetAvailableLocaleCodes(1);")
            .is_err()
    );
    assert!(engine.compile("Имя = LocaleCodePresentation();").is_err());
}

#[test]
fn full_text_search_global_resolves_without_fabricating_a_session_index() {
    let engine = engine("");
    engine
        .compile(
            "Процедура НеВызывается()\nСписок = ПолнотекстовыйПоиск.СоздатьСписок(\"строка\", 200);\nКонецПроцедуры",
        )
        .unwrap();
    let mut state = engine.state_builder().build();
    for name in ["ПолнотекстовыйПоиск", "fulltextsearch"] {
        let source = format!(
            "Попытка\nМенеджер = {name};\nИсключение\nВозврат Истина;\nКонецПопытки;\nВозврат Ложь;"
        );
        assert_eq!(state.exec(&source).unwrap(), Value::Boolean(true));
    }
    assert_eq!(
        state
            .exec("ПолнотекстовыйПоиск = 7; Возврат ПолнотекстовыйПоиск;")
            .unwrap(),
        Value::number_from_i64(7)
    );
}

#[test]
fn server_path_separator_uses_the_session_file_system() {
    let engine = engine("");
    let mut state = engine.state_builder().build();
    for name in ["ПолучитьРазделительПутиСервера", "getserverpathseparator"]
    {
        let source = format!("Возврат {name}();");
        assert_eq!(
            state.exec(&source).unwrap(),
            Value::Str(std::path::MAIN_SEPARATOR.to_string().as_str().into())
        );
    }
    assert!(
        engine
            .compile("Возврат ПолучитьРазделительПутиСервера(1);")
            .is_err()
    );
}

#[test]
fn licensing_client_name_resolves_without_fabricating_a_database_value() {
    let engine = engine("");
    engine
        .compile(
            "Процедура НеВызывается()\nИмя = ПолучитьИмяКлиентаЛицензирования();\nКонецПроцедуры",
        )
        .unwrap();
    let mut state = engine.state_builder().build();
    for name in ["ПолучитьИмяКлиентаЛицензирования", "getlicensingclientname"]
    {
        let source = format!(
            "Попытка\nИмя = {name}();\nИсключение\nВозврат Истина;\nКонецПопытки;\nВозврат Ложь;"
        );
        assert_eq!(state.exec(&source).unwrap(), Value::Boolean(true));
    }
    assert!(
        engine
            .compile("Возврат ПолучитьИмяКлиентаЛицензирования(1);")
            .is_err()
    );
}

#[test]
fn configuration_id_resolves_without_substituting_a_fingerprint() {
    let engine = engine("");
    engine
        .compile(
            "Процедура НеВызывается()\nИД = ПолучитьИдентификаторКонфигурации();\nКонецПроцедуры",
        )
        .unwrap();
    let mut state = engine.state_builder().build();
    for name in ["ПолучитьИдентификаторКонфигурации", "getconfigurationid"]
    {
        let source = format!(
            "Попытка\nИД = {name}();\nИсключение\nВозврат Истина;\nКонецПопытки;\nВозврат Ложь;"
        );
        assert_eq!(state.exec(&source).unwrap(), Value::Boolean(true));
    }
    assert!(
        engine
            .compile("Возврат ПолучитьИдентификаторКонфигурации(1);")
            .is_err()
    );
}

#[test]
fn licensing_additional_parameter_requires_a_session_database_value() {
    let engine = engine("");
    let mut state = engine.state_builder().build();
    for name in [
        "ПолучитьДополнительныйПараметрКлиентаЛицензирования",
        "getlicensingclientadditionalparameter",
    ] {
        let source = format!(
            "Попытка\nПараметр = {name}();\nИсключение\nВозврат Истина;\nКонецПопытки;\nВозврат Ложь;"
        );
        assert_eq!(state.exec(&source).unwrap(), Value::Boolean(true));
    }
    assert!(
        engine
            .compile("Возврат ПолучитьДополнительныйПараметрКлиентаЛицензирования(1);")
            .is_err()
    );
}

#[test]
fn licensing_settings_writer_never_reports_a_no_op_success() {
    let engine = engine("");
    let mut state = engine.state_builder().build();
    for call in [
        "УстановитьНастройкиКлиентаЛицензирования()",
        "SetLicensingClientParameters(\"u\")",
        "УстановитьНастройкиКлиентаЛицензирования(\"u\", \"p\", \"x\")",
    ] {
        let source =
            format!("Попытка\n{call};\nИсключение\nВозврат Истина;\nКонецПопытки;\nВозврат Ложь;");
        assert_eq!(state.exec(&source).unwrap(), Value::Boolean(true));
    }
    assert!(
        engine
            .compile("УстановитьНастройкиКлиентаЛицензирования(1, 2, 3, 4);")
            .is_err()
    );
}

#[test]
fn settings_description_matches_the_measured_surface() {
    let engine = engine("");
    let mut state = engine.state_builder().build();

    assert_eq!(
        state
            .exec(
                r#"
                Описание = Новый ОписаниеНастроек;
                Начало = Строка(ТипЗнч(Описание)) + ":"
                    + Строка(Описание.КлючОбъекта = Неопределено) + ":"
                    + Строка(Описание.КлючНастроек = Неопределено) + ":"
                    + Строка(Описание.ДополнительныеСвойства.Количество());
                Описание.КлючОбъекта = 1;
                Описание.КлючНастроек = "key";
                Описание.Пользователь = "user";
                Описание.Представление = "caption";
                Описание.ДополнительныеСвойства.Вставить("Поле", 42);
                EnglishDescription = New SettingsDescription();
                EnglishDescription.User = "english";
                Возврат Начало + ":" + Строка(Описание.КлючОбъекта) + ":"
                    + Описание.КлючНастроек + ":" + Описание.Пользователь + ":"
                    + Описание.Представление + ":"
                    + Строка(Описание.ДополнительныеСвойства.Поле) + ":"
                    + EnglishDescription.User;
                "#,
            )
            .unwrap(),
        Value::Str("Setting description:Да:Да:0:1:key:user:caption:42:english".into())
    );
    assert!(
        state
            .exec(
                "Описание = Новый ОписаниеНастроек; Описание.ДополнительныеСвойства = Новый Структура;",
            )
            .unwrap_err()
            .to_string()
            .contains("ДополнительныеСвойства")
    );
}

#[test]
fn client_settings_match_the_measured_mutable_value() {
    let engine = engine("");
    let mut state = engine.state_builder().build();
    let value = state
        .exec(
            r#"
            Настройки = Новый НастройкиКлиентскогоПриложения;
            ПоУмолчанию = Строка(ТипЗнч(Настройки)) + "|"
                + Строка(Настройки.ОтображатьПанелиНавигацииИДействий) + "|"
                + Строка(Настройки.ОтображатьПанельРазделов) + "|"
                + Строка(Настройки.РежимОткрытияФормПриложения) + "|"
                + Строка(Настройки.ВариантИнтерфейсаКлиентскогоПриложения);
            Настройки.ОтображатьПанелиНавигацииИДействий = Ложь;
            Настройки.ОтображатьПанельРазделов = Истина;
            Настройки.РежимОткрытияФормПриложения = РежимОткрытияФормПриложения.ОтдельныеОкна;
            Настройки.ВариантИнтерфейсаКлиентскогоПриложения = ВариантИнтерфейсаКлиентскогоПриложения.Такси;
            English = New ClientSettings;
            English.ShowNavigationAndActionsPanels = False;
            English.ShowSectionsPanel = True;
            English.ApplicationFormsOpenningMode = ApplicationFormsOpenningMode.Tabs;
            English.ClientApplicationInterfaceVariant = ClientApplicationInterfaceVariant.Taxi;
            Возврат ПоУмолчанию + "|"
                + Строка(Настройки.РежимОткрытияФормПриложения) + "|"
                + Строка(Настройки.ВариантИнтерфейсаКлиентскогоПриложения) + "|"
                + Строка(ТипЗнч(English.ApplicationFormsOpenningMode)) + "|"
                + Строка(ТипЗнч(English.ClientApplicationInterfaceVariant));
            "#,
        )
        .unwrap();
    assert_eq!(
        value,
        Value::Str(
            "Client application settings|Да|Да|Tabs|Version 8.2|Separate windows|Taxi|ApplicationFormsOpenningMode|ClientApplicationInterfaceVariant"
                .into()
        )
    );

    for source in [
        r#"Настройки = Новый ClientSettings; Настройки.ShowSectionsPanel = "Да";"#,
        r#"Настройки = Новый ClientSettings; Настройки.ApplicationFormsOpenningMode = ClientApplicationInterfaceVariant.Taxi;"#,
    ] {
        assert!(state.exec(source).is_err());
    }
}

#[test]
fn command_interface_settings_match_the_measured_mutable_value() {
    let engine = engine("");
    let mut state = engine.state_builder().build();
    assert_eq!(
        state
            .exec(
                r#"
                Настройки = Новый НастройкиКомандногоИнтерфейса;
                ПоУмолчанию = Строка(ТипЗнч(Настройки)) + "|"
                    + Строка(Настройки.ОтображениеПанелиРазделов);
                Настройки.ОтображениеПанелиРазделов = ОтображениеПанелиРазделов.КартинкаСлеваИТекст;
                English = New CommandInterfaceSettings;
                English.SectionsPanelRepresentation = SectionsPanelRepresentation.PictureOnTopAndText;
                Возврат ПоУмолчанию + "|"
                    + Строка(Настройки.ОтображениеПанелиРазделов) + "|"
                    + Строка(English.SectionsPanelRepresentation) + "|"
                    + Строка(ТипЗнч(SectionsPanelRepresentation.Picture));
                "#,
            )
            .unwrap(),
        Value::Str(
            "Command interface settings|Icon and text|Text to the right of icon|Text below icon|SectionsPanelRepresentation"
                .into()
        )
    );
    assert!(
        state
            .exec(
                "Настройки = Новый НастройкиКомандногоИнтерфейса; Настройки.ОтображениеПанелиРазделов = ВариантИнтерфейсаКлиентскогоПриложения.Такси;",
            )
            .is_err()
    );
}

#[test]
fn client_interface_content_settings_match_the_measured_value_graph() {
    let engine = engine("");
    let mut state = engine.state_builder().build();
    assert_eq!(
        state
            .exec(
                r#"
                Настройки = Новый НастройкиИнтерфейсаКлиентскогоПриложения;
                Состав = Новый НастройкиСоставаИнтерфейсаКлиентскогоПриложения;
                Группа = Новый ГруппаНастройкиСоставаИнтерфейсаКлиентскогоПриложения;
                Элемент = Новый ЭлементНастройкиСоставаИнтерфейсаКлиентскогоПриложения("ПанельРазделов");
                Группа.Добавить(Элемент);
                Состав.Лево.Добавить(Группа);
                Настройки.УстановитьСостав(Состав);
                Полученный = Настройки.ПолучитьСостав();

                EnglishSettings = New ClientApplicationInterfaceSettings;
                EnglishContent = New ClientApplicationInterfaceContentSettings;
                EnglishGroup = New ClientApplicationInterfaceContentSettingsGroup;
                EnglishItem = New ClientApplicationInterfaceContentSettingsItem("SectionsPanel");
                EnglishGroup.Add(EnglishItem);
                EnglishContent.Right.Add(EnglishGroup);
                EnglishSettings.SetContent(EnglishContent);

                Возврат Строка(ТипЗнч(Настройки)) + "|"
                    + Строка(ТипЗнч(Полученный)) + "|"
                    + Строка(ТипЗнч(Полученный.Лево)) + "|"
                    + Строка(ТипЗнч(Группа.Получить(0))) + "|"
                    + Группа.Получить(0).Имя + "|"
                    + Строка(Полученный.Верх.Количество()) + "|"
                    + Строка(Полученный.Лево.Количество()) + "|"
                    + EnglishSettings.GetContent().Right.Get(0).Get(0).Name;
                "#,
            )
            .unwrap(),
        Value::Str(
            "Panel settings|Client application interface composition settings|Client application interface composition settings group|Client application interface composition settings item|ПанельРазделов|0|1|SectionsPanel"
                .into()
        )
    );

    for source in [
        "Настройки = Новый НастройкиИнтерфейсаКлиентскогоПриложения; Настройки.ПолучитьСостав();",
        "Настройки = Новый НастройкиИнтерфейсаКлиентскогоПриложения; Настройки.УстановитьСостав();",
        "Настройки = Новый НастройкиИнтерфейсаКлиентскогоПриложения; Настройки.УстановитьСостав(42);",
        "Группа = Новый ГруппаНастройкиСоставаИнтерфейсаКлиентскогоПриложения; Группа.Добавить(42);",
    ] {
        assert!(
            state.exec(source).is_err(),
            "исходник должен завершиться ошибкой: {source}"
        );
    }
}

#[test]
fn job_schedule_matches_the_measured_passive_surface() {
    let engine = engine("");
    let mut state = engine.state_builder().build();

    assert_eq!(
        state
            .exec(
                r#"
                Первое = Новый РасписаниеРегламентногоЗадания;
                Второе = New JobSchedule();
                ЗначенияПоУмолчанию = Строка(ТипЗнч(Первое)) + ":"
                    + Строка(Первое.ДатаНачала = Дата(1, 1, 1)) + ":"
                    + Строка(Первое.ПериодНедель) + ":"
                    + Строка(Первое.ПериодПовтораДней) + ":"
                    + Строка(Первое.ДниНедели.Количество()) + ":"
                    + Строка(Первое.Месяцы.Количество()) + ":"
                    + Строка(Первое.ДетальныеРасписанияДня.Количество());
                Первое.ДатаНачала = '20240101';
                Первое.DaysRepeatPeriod = 3;
                Первое.ДниНедели.Добавить(1);
                Первое.Месяцы.Добавить(2);
                Первое.ДетальныеРасписанияДня.Добавить(Второе);
                Второе.WeeksPeriod = 2;
                Возврат ЗначенияПоУмолчанию + ":"
                    + Строка(Первое.ДниНедели.Количество()) + ":"
                    + Строка(Первое.Месяцы.Количество()) + ":"
                    + Строка(Первое.ДетальныеРасписанияДня.Количество()) + ":"
                    + Строка(Первое.ПериодПовтораДней) + ":"
                    + Строка(Второе.ДниНедели.Количество()) + ":"
                    + Строка(Второе.ПериодНедель);
                "#,
            )
            .unwrap(),
        Value::Str("Job schedule:Да:1:0:0:0:0:1:1:1:3:0:2".into())
    );
    assert!(
        state
            .exec("Расписание = Новый РасписаниеРегламентногоЗадания; Расписание.ДатаНачала = 1;",)
            .unwrap_err()
            .to_string()
            .contains("Дата")
    );
}

#[test]
fn unsafe_operation_protection_description_is_a_mutable_boolean_value() {
    let engine = engine("");
    let mut state = engine.state_builder().build();

    assert_eq!(
        state
            .exec(
                r#"
                Описание = Новый ОписаниеЗащитыОтОпасныхДействий;
                Начальное = Описание.ПредупреждатьОбОпасныхДействиях;
                Описание.ПредупреждатьОбОпасныхДействиях = Ложь;
                Попытка
                    Описание.UnsafeOperationWarnings = 1;
                Исключение
                КонецПопытки;
                EnglishDescription = New UnsafeOperationProtectionDescription();
                EnglishDescription.UnsafeOperationWarnings = Истина;
                Возврат Строка(ТипЗнч(Описание)) + ":" + Строка(Начальное) + ":"
                    + Строка(Описание.ПредупреждатьОбОпасныхДействиях) + ":"
                    + Строка(EnglishDescription.ПредупреждатьОбОпасныхДействиях);
                "#,
            )
            .unwrap(),
        Value::Str("Unsafe operation protection description:Да:Нет:Да".into())
    );
}

#[test]
fn event_log_event_use_matches_the_measured_mutable_value() {
    let engine = engine("");
    let mut state = engine.state_builder().build();

    assert_eq!(
        state
            .exec(
                r#"
                Описание = Новый ИспользованиеСобытияЖурналаРегистрации;
                НачальноеИспользование = Описание.Использование;
                НачальноеОписание = ТипЗнч(Описание.ОписаниеИспользования);
                Описание.Использование = 0;
                Описание.ОписаниеИспользования = 42;
                English = New EventLogEventUse(False, "initial");
                English.Use = 1;
                English.UseDescription = "changed";
                Возврат Строка(ТипЗнч(Описание)) + ":" + Строка(НачальноеИспользование)
                    + ":" + Строка(НачальноеОписание) + ":" + Строка(Описание.Использование)
                    + ":" + Строка(Описание.ОписаниеИспользования) + ":"
                    + Строка(English.Use) + ":" + English.UseDescription;
                "#,
            )
            .unwrap(),
        Value::Str("Event log event use:Да:Не определено:Нет:42:Да:changed".into())
    );
    assert!(
        state
            .exec("Возврат Новый ИспользованиеСобытияЖурналаРегистрации(\"Истина\");")
            .unwrap_err()
            .to_string()
            .contains("Булево")
    );
}

#[test]
fn event_log_access_event_use_description_matches_the_measured_value() {
    let engine = engine("");
    let mut state = engine.state_builder().build();

    assert_eq!(
        state
            .exec(
                r#"
                Описание = Новый ОписаниеИспользованияСобытияДоступЖурналаРегистрации;
                Начало = Строка(ТипЗнч(Описание)) + "|" + Описание.Объект + "|"
                    + Строка(Описание.ПоляДоступа.Количество()) + "|"
                    + Строка(Описание.ПоляРегистрации.Количество());
                Доступ = Новый Массив;
                Доступ.Добавить("Поле1");
                Регистрация = Новый Массив;
                Регистрация.Добавить("Поле2");
                Описание.Объект = "Справочник.Номенклатура";
                Описание.ПоляДоступа = Доступ;
                Описание.ПоляРегистрации = Регистрация;
                English = New EventLogAccessEventUseDescription("Catalog.Products");
                English.AccessFields = New Array;
                English.LoggedFields = New Array;
                Возврат Начало + "|" + Описание.Объект + "|"
                    + Описание.ПоляДоступа[0] + "|" + Описание.ПоляРегистрации[0]
                    + "|" + English.Object;
                "#,
            )
            .unwrap(),
        Value::Str(
            "Description of event log access event use||0|0|Справочник.Номенклатура|Поле1|Поле2|Catalog.Products"
                .into()
        )
    );

    for source in [
        "Описание = Новый EventLogAccessEventUseDescription; Описание.Object = 1;",
        "Описание = Новый EventLogAccessEventUseDescription; Описание.AccessFields = \"Поле\";",
        "Описание = Новый EventLogAccessEventUseDescription; Описание.LoggedFields = 1;",
    ] {
        assert!(
            state.exec(source).is_err(),
            "исходник должен завершиться ошибкой: {source}"
        );
    }
}

#[test]
fn unset_nested_cycle_retry_clear_and_non_rollback_match_the_measurement() {
    let parameters = Rc::new(
        SessionParameters::new(
            "МодульСеанса",
            ["А", "Б", "Режим"].into_iter().map(str::to_string),
        )
        .unwrap(),
    );
    parameters
        .set("Режим", Value::Str("обычный".into()))
        .unwrap();
    let engine = engine(
        r#"
        Процедура УстановкаПараметровСеанса(Требуемые) Экспорт
            Если Требуемые = Неопределено Тогда
                Возврат;
            КонецЕсли;
            Имя = Требуемые[0];
            Если Имя = "А" Тогда
                Если ПараметрыСеанса.Режим = "цикл" Тогда
                    Х = ПараметрыСеанса.Б;
                ИначеЕсли ПараметрыСеанса.Режим = "вложенный" Тогда
                    Х = ПараметрыСеанса.Б;
                    ПараметрыСеанса.А = 10;
                ИначеЕсли ПараметрыСеанса.Режим = "вложенныйСОшибка" Тогда
                    Х = ПараметрыСеанса.Б;
                    ВызватьИсключение "после вложенной";
                ИначеЕсли ПараметрыСеанса.Режим = "ошибкаПослеЗаписи" Тогда
                    ПараметрыСеанса.А = 40;
                    ВызватьИсключение "после записи";
                ИначеЕсли ПараметрыСеанса.Режим = "неУстановлен" Тогда
                    Возврат;
                Иначе
                    ПараметрыСеанса.А = 10;
                КонецЕсли;
            ИначеЕсли Имя = "Б" Тогда
                Если ПараметрыСеанса.Режим = "цикл" Тогда
                    Х = ПараметрыСеанса.А;
                Иначе
                    ПараметрыСеанса.Б = 20;
                КонецЕсли;
            КонецЕсли;
        КонецПроцедуры
        "#,
    );
    let module = entry(
        &engine,
        r#"
        Функция ПрочитатьА() Экспорт Возврат ПараметрыСеанса.А; КонецФункции
        Функция ПрочитатьБ() Экспорт Возврат ПараметрыСеанса.Б; КонецФункции
        Процедура Режим(Значение) Экспорт ПараметрыСеанса.Режим = Значение; КонецПроцедуры
        Процедура Очистить() Экспорт ПараметрыСеанса.Очистить(); КонецПроцедуры
        "#,
    );
    let mut state = engine
        .state_builder()
        .host_data(host_data(Rc::clone(&parameters)))
        .build();
    let instance = state.instantiate(&module).unwrap();

    assert_eq!(
        instance.call(&mut state, "ПрочитатьА", vec![]).unwrap().0,
        Value::number_from_i64(10)
    );
    instance
        .call(
            &mut state,
            "Режим",
            vec![Value::Str("ошибкаПослеЗаписи".into())],
        )
        .unwrap();
    instance.call(&mut state, "Очистить", vec![]).unwrap();
    parameters
        .set("Режим", Value::Str("ошибкаПослеЗаписи".into()))
        .unwrap();
    let error = instance.call(&mut state, "ПрочитатьА", vec![]).unwrap_err();
    assert!(error.to_string().contains("после записи"));
    assert_eq!(
        parameters.peek("А").unwrap(),
        Some(Value::number_from_i64(40))
    );

    parameters.clear();
    parameters.set("Режим", Value::Str("цикл".into())).unwrap();
    let error = instance.call(&mut state, "ПрочитатьА", vec![]).unwrap_err();
    assert!(error.to_string().contains("context attribute (А)"));

    parameters.clear();
    parameters
        .set("Режим", Value::Str("неУстановлен".into()))
        .unwrap();
    for _ in 0..2 {
        let error = instance.call(&mut state, "ПрочитатьА", vec![]).unwrap_err();
        assert!(error.to_string().contains("context attribute (А)"));
    }

    parameters.clear();
    parameters
        .set("Режим", Value::Str("вложенный".into()))
        .unwrap();
    assert_eq!(
        instance.call(&mut state, "ПрочитатьА", vec![]).unwrap().0,
        Value::number_from_i64(10)
    );
    assert_eq!(
        parameters.peek("Б").unwrap(),
        Some(Value::number_from_i64(20))
    );

    parameters.clear();
    parameters
        .set("Режим", Value::Str("вложенныйСОшибка".into()))
        .unwrap();
    let error = instance.call(&mut state, "ПрочитатьА", vec![]).unwrap_err();
    assert!(error.to_string().contains("после вложенной"));
    assert_eq!(parameters.peek("А").unwrap(), None);
    assert_eq!(
        parameters.peek("Б").unwrap(),
        Some(Value::number_from_i64(20))
    );
}

#[test]
fn startup_receives_undefined() {
    fn start(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
        let parameters = Rc::clone(runtime(ctx)?.parameters());
        parameters.initialize(ctx, None)?;
        Ok(Value::Undefined)
    }
    const START: FunctionDescriptor = FunctionDescriptor {
        code: FunctionCode::new(2),
        names: &["ЗапуститьПробу"],
        arity: Arity::exact(0),
        kind: FunctionKind::Procedure,
        call: start,
    };
    const TEST_LIBRARY: LibraryDescriptor =
        LibraryDescriptor::new("bsl-session-start-test", "1").with_functions(&[START]);
    let parameters = Rc::new(SessionParameters::new("МодульСеанса", ["А".into()]).unwrap());
    let engine = Engine::builder()
        .register_library(bsl_orm::library())
        .register_library(library())
        .register_library(TEST_LIBRARY)
        .common_module(
            "МодульСеанса",
            "Процедура УстановкаПараметровСеанса(Требуемые) Экспорт Если Требуемые <> Неопределено Тогда ВызватьИсключение; КонецЕсли; ПараметрыСеанса.А = 7; КонецПроцедуры",
        )
        .build()
        .unwrap();
    let module = entry(
        &engine,
        "Процедура Старт() Экспорт ЗапуститьПробу(); КонецПроцедуры",
    );
    let mut state = engine
        .state_builder()
        .host_data(host_data(Rc::clone(&parameters)))
        .build();
    let instance = state.instantiate(&module).unwrap();
    instance.call(&mut state, "Старт", vec![]).unwrap();
    assert_eq!(
        parameters.peek("А").unwrap(),
        Some(Value::number_from_i64(7))
    );
}

#[test]
fn background_copy_inherits_values_and_then_is_isolated() {
    let parent = SessionParameters::new("МодульСеанса", ["А".into()]).unwrap();
    parent.set("А", Value::number_from_i64(1)).unwrap();
    let child = parent.copy_for_background();
    child.set("А", Value::number_from_i64(2)).unwrap();

    assert_eq!(parent.peek("А").unwrap(), Some(Value::number_from_i64(1)));
    assert_eq!(child.peek("А").unwrap(), Some(Value::number_from_i64(2)));
}

#[test]
fn missing_runtime_is_an_explicit_error() {
    let engine = Engine::builder()
        .register_library(library())
        .build()
        .unwrap();
    let error = engine
        .new_state()
        .exec("Возврат ПараметрыСеанса.А;")
        .unwrap_err();
    assert!(matches!(error, Error::Runtime(_)));
    assert!(error.to_string().contains("данные хоста"));
}

#[test]
fn refreshing_an_absent_reusable_value_cache_succeeds() {
    let engine = Engine::builder()
        .register_library(library())
        .build()
        .unwrap();
    let mut state = engine.new_state();

    assert_eq!(
        state
            .exec("ОбновитьПовторноИспользуемыеЗначения(); RefreshReusableValues(); Возврат 1;")
            .unwrap(),
        Value::number_from_i64(1)
    );
}

#[test]
fn dynamic_database_configuration_change_state_is_session_local() {
    let foreground_factory = SessionFactory::new(
        code_snapshot(START_HANDLER),
        metadata_catalog("configuration"),
        "МодульСеанса",
        SessionKind::ThinClient,
        PLATFORM_VERSION,
    )
    .unwrap();
    let mut parent = foreground_factory
        .open(
            principal(),
            resources((), 1).with_database_configuration_changed_dynamically(true),
        )
        .unwrap();
    let probe = parent
        .instantiate(
            "Функция Проверить() Экспорт Возврат КонфигурацияБазыДанныхИзмененаДинамически() = DataBaseConfigurationChangedDynamically(); КонецФункции",
        )
        .unwrap();
    assert_eq!(
        parent.call(&probe, "Проверить", vec![]).unwrap().0,
        Value::Boolean(true)
    );

    let background_factory = SessionFactory::new(
        code_snapshot(START_HANDLER),
        metadata_catalog("configuration"),
        "МодульСеанса",
        SessionKind::BackgroundJob,
        PLATFORM_VERSION,
    )
    .unwrap();
    let mut background = background_factory
        .open_background(
            principal(),
            resources((), 2).with_database_configuration_changed_dynamically(false),
            "Обработка.Проверить",
            &parent,
        )
        .unwrap();
    let probe = background
        .instantiate(
            "Функция Проверить() Экспорт Возврат КонфигурацияБазыДанныхИзмененаДинамически(); КонецФункции",
        )
        .unwrap();
    assert_eq!(
        background.call(&probe, "Проверить", vec![]).unwrap().0,
        Value::Boolean(false)
    );

    let mut missing = foreground_factory
        .open(principal(), resources((), 3))
        .unwrap();
    let probe = missing
        .instantiate(
            "Функция Проверить() Экспорт Возврат КонфигурацияБазыДанныхИзмененаДинамически(); КонецФункции",
        )
        .unwrap();
    assert!(
        missing
            .call(&probe, "Проверить", vec![])
            .unwrap_err()
            .to_string()
            .contains("не передан хостом")
    );
}

fn code_snapshot(handler: &str) -> ConfigurationCodeSnapshot {
    ConfigurationCodeSnapshot::new(
        "configuration",
        vec![ConfigurationModule::new(
            "session",
            "МодульСеанса",
            handler,
            ModuleKind::Session,
            ModuleExecution::new()
                .with_server(true)
                .with_external_connection(true),
        )],
    )
    .unwrap()
}

fn metadata_catalog(identity: &str) -> Arc<MetadataCatalog> {
    Arc::new(
        MetadataCatalog::new(
            identity,
            vec![
                bsl_orm::MetadataObject::new(
                    "started",
                    "Запущен",
                    bsl_orm::MetadataKind::SessionParameter,
                ),
                bsl_orm::MetadataObject::new(
                    "copied",
                    "Копируемый",
                    bsl_orm::MetadataKind::SessionParameter,
                ),
                bsl_orm::MetadataObject::new(
                    "constant-answer",
                    "Ответ",
                    bsl_orm::MetadataKind::Constant,
                ),
            ],
        )
        .unwrap(),
    )
}

fn principal() -> Principal {
    Principal::new("user", "Тест", Vec::new())
}

fn resources<T: 'static>(connection: T, connection_number: u64) -> SessionResources {
    SessionResources::new(Box::new(connection), connection_number)
}

#[test]
fn startup_reaches_only_the_used_manager_module_and_keeps_its_session_instance() {
    let code = ConfigurationCodeSnapshot::new("configuration", vec![
        ConfigurationModule::new(
            "session", "МодульСеанса",
            "Процедура УстановкаПараметровСеанса(Имена) ПараметрыСеанса.Запущен = Catalogs.Версии.Следующий(); КонецПроцедуры",
            ModuleKind::Session, ModuleExecution::new().with_server(true),
        ),
        ConfigurationModule::new(
            "manager", "Справочник.Версии.МодульМенеджера",
            "Перем Счетчик; Счетчик = 0; Функция Следующий() Экспорт Если ЭтотОбъект.Имя <> \"Версии\" Тогда ВызватьИсключение \"неверный получатель\"; КонецЕсли; Счетчик = Счетчик + 1; Возврат Счетчик; КонецФункции",
            ModuleKind::Manager, ModuleExecution::new().with_server(true),
        ).with_owner("catalog"),
        ConfigurationModule::new(
            "unused", "Справочник.Неиспользуемый.МодульМенеджера",
            "Функция Ошибка() Экспорт Возврат НеизвестноеИмя; КонецФункции",
            ModuleKind::Manager, ModuleExecution::new().with_server(true),
        ).with_owner("unused-catalog"),
    ]).unwrap();
    let catalog = Arc::new(
        MetadataCatalog::new(
            "configuration",
            vec![
                MetadataObject::new("started", "Запущен", MetadataKind::SessionParameter),
                MetadataObject::new("catalog", "Версии", MetadataKind::Catalog)
                    .with_manager_module("Справочник.Версии.МодульМенеджера"),
                MetadataObject::new("unused-catalog", "Неиспользуемый", MetadataKind::Catalog)
                    .with_manager_module("Справочник.Неиспользуемый.МодульМенеджера"),
            ],
        )
        .unwrap(),
    );
    let factory = SessionFactory::new(
        code,
        catalog,
        "МодульСеанса",
        SessionKind::ThinClient,
        PLATFORM_VERSION,
    )
    .unwrap();
    let mut first = factory.open(principal(), resources((), 1)).unwrap();
    let mut second = factory.open(principal(), resources((), 2)).unwrap();
    let source = "Функция Следующий() Экспорт Возврат Справочники.Версии.Следующий(); КонецФункции";
    let first_module = first.instantiate(source).unwrap();
    let second_module = second.instantiate(source).unwrap();
    assert_eq!(
        first.call(&first_module, "Следующий", vec![]).unwrap().0,
        Value::number_from_i64(2)
    );
    assert_eq!(
        first.call(&first_module, "Следующий", vec![]).unwrap().0,
        Value::number_from_i64(3)
    );
    assert_eq!(
        second.call(&second_module, "Следующий", vec![]).unwrap().0,
        Value::number_from_i64(2)
    );
}

#[test]
fn variables_shadow_manager_globals_without_reaching_the_manager_module() {
    for handler in [
        "Перем Справочники; Процедура УстановкаПараметровСеанса(Имена) Справочники = Новый Структура(\"Версии\", 7); ПараметрыСеанса.Запущен = Справочники.Версии; КонецПроцедуры",
        "Процедура УстановкаПараметровСеанса(Имена) Перем Справочники; Справочники = Новый Структура(\"Версии\", 7); ПараметрыСеанса.Запущен = Справочники.Версии; КонецПроцедуры",
    ] {
        let code = ConfigurationCodeSnapshot::new(
            "configuration",
            vec![
                ConfigurationModule::new(
                    "session",
                    "МодульСеанса",
                    handler,
                    ModuleKind::Session,
                    ModuleExecution::new().with_server(true),
                ),
                ConfigurationModule::new(
                    "manager",
                    "Справочник.Версии.МодульМенеджера",
                    "Функция Ошибка() Экспорт Возврат НеизвестноеИмя; КонецФункции",
                    ModuleKind::Manager,
                    ModuleExecution::new().with_server(true),
                )
                .with_owner("catalog"),
            ],
        )
        .unwrap();
        let catalog = Arc::new(
            MetadataCatalog::new(
                "configuration",
                vec![
                    MetadataObject::new("started", "Запущен", MetadataKind::SessionParameter),
                    MetadataObject::new("catalog", "Версии", MetadataKind::Catalog)
                        .with_manager_module("Справочник.Версии.МодульМенеджера"),
                ],
            )
            .unwrap(),
        );
        let factory = SessionFactory::new(
            code,
            catalog,
            "МодульСеанса",
            SessionKind::ThinClient,
            PLATFORM_VERSION,
        )
        .unwrap();
        let mut session = factory.open(principal(), resources((), 1)).unwrap();
        let module = session
            .instantiate("Функция Значение() Экспорт Возврат ПараметрыСеанса.Запущен; КонецФункции")
            .unwrap();
        assert_eq!(
            session.call(&module, "Значение", vec![]).unwrap().0,
            Value::number_from_i64(7)
        );
    }
}

#[test]
fn script_arguments_belong_to_the_opened_session() {
    let factory = SessionFactory::new(
        code_snapshot(START_HANDLER),
        metadata_catalog("configuration"),
        "МодульСеанса",
        SessionKind::ThinClient,
        PLATFORM_VERSION,
    )
    .unwrap();
    let mut first = factory
        .open(
            principal(),
            resources((), 1).with_arguments(vec!["первый".into()]),
        )
        .unwrap();
    let mut second = factory
        .open(
            principal(),
            resources((), 2).with_arguments(vec!["второй".into()]),
        )
        .unwrap();
    let source = "Функция Аргумент() Экспорт Возврат АргументыКоманднойСтроки[0]; КонецФункции";
    for (session, expected) in [(&mut first, "первый"), (&mut second, "второй")] {
        let module = session.instantiate(source).unwrap();
        assert_eq!(
            session.call(&module, "Аргумент", Vec::new()).unwrap().0,
            Value::Str(expected.into())
        );
    }
}

#[test]
fn client_displays_are_owned_by_one_session_and_absent_in_background() {
    let foreground = SessionFactory::new(
        code_snapshot(START_HANDLER),
        metadata_catalog("configuration"),
        "МодульСеанса",
        SessionKind::WebClient,
        PLATFORM_VERSION,
    )
    .unwrap();
    let mut first = foreground
        .open(
            principal(),
            resources((), 1).with_client_displays([
                ClientDisplayInformation::new(1920, 1080, 96),
                ClientDisplayInformation::new(1280, 800, 120),
            ]),
        )
        .unwrap();
    let probe = first
        .instantiate(
            r#"
            Функция Экраны() Экспорт
                Список = GetClientDisplaysInformation();
                Возврат Список.Количество() = 2
                    И Строка(ТипЗнч(Список[0])) = "ClientDisplayInformation"
                    И Список[0].Width = 1920
                    И Список[0].Высота = 1080
                    И Список[0].DPI = 96
                    И Список[1].DPI = 120;
            КонецФункции
            "#,
        )
        .unwrap();
    let (value, _) = first.call(&probe, "Экраны", Vec::new()).unwrap();
    assert_eq!(value, Value::Boolean(true));

    let mut second = foreground.open(principal(), resources((), 2)).unwrap();
    let probe = second
        .instantiate(
            "Функция Экраны() Экспорт Возврат ПолучитьИнформациюЭкрановКлиента(); КонецФункции",
        )
        .unwrap();
    assert!(second.call(&probe, "Экраны", Vec::new()).is_err());

    let background = SessionFactory::new(
        code_snapshot(START_HANDLER),
        metadata_catalog("configuration"),
        "МодульСеанса",
        SessionKind::BackgroundJob,
        PLATFORM_VERSION,
    )
    .unwrap();
    let mut child = background
        .open_background(
            principal(),
            resources((), 3).with_client_displays([ClientDisplayInformation::new(1, 1, 1)]),
            "Обработка.Проверка",
            &first,
        )
        .unwrap();
    let probe = child
        .instantiate(
            "Функция Экраны() Экспорт Возврат ПолучитьИнформациюЭкрановКлиента(); КонецФункции",
        )
        .unwrap();
    assert_eq!(
        child.call(&probe, "Экраны", Vec::new()).unwrap().0,
        Value::Undefined
    );
}

#[test]
fn computer_name_uses_only_the_current_sessions_host_value() {
    let foreground = SessionFactory::new(
        code_snapshot(START_HANDLER),
        metadata_catalog("configuration"),
        "МодульСеанса",
        SessionKind::WebClient,
        PLATFORM_VERSION,
    )
    .unwrap();
    let mut parent = foreground
        .open(
            principal(),
            resources((), 1).with_computer_name("parent-host"),
        )
        .unwrap();
    let probe = parent
        .instantiate(
            "Функция Имя() Экспорт Возврат ИмяКомпьютера() + \"|\" + ComputerName(); КонецФункции",
        )
        .unwrap();
    assert_eq!(
        parent.call(&probe, "Имя", Vec::new()).unwrap().0,
        Value::Str("parent-host|parent-host".into())
    );

    let mut missing = foreground.open(principal(), resources((), 2)).unwrap();
    let probe = missing
        .instantiate("Функция Имя() Экспорт Возврат ИмяКомпьютера(); КонецФункции")
        .unwrap();
    assert!(missing.call(&probe, "Имя", Vec::new()).is_err());

    let background = SessionFactory::new(
        code_snapshot(START_HANDLER),
        metadata_catalog("configuration"),
        "МодульСеанса",
        SessionKind::BackgroundJob,
        PLATFORM_VERSION,
    )
    .unwrap();
    let mut child = background
        .open_background(
            principal(),
            resources((), 3).with_computer_name("child-host"),
            "Обработка.Проверка",
            &parent,
        )
        .unwrap();
    let probe = child
        .instantiate("Функция Имя() Экспорт Возврат ComputerName(); КонецФункции")
        .unwrap();
    assert_eq!(
        child.call(&probe, "Имя", Vec::new()).unwrap().0,
        Value::Str("child-host".into())
    );
}

#[test]
fn current_locale_code_uses_only_the_current_sessions_host_value() {
    let foreground = SessionFactory::new(
        code_snapshot(START_HANDLER),
        metadata_catalog("configuration"),
        "МодульСеанса",
        SessionKind::ThinClient,
        PLATFORM_VERSION,
    )
    .unwrap();
    let mut parent = foreground
        .open(principal(), resources((), 1).with_locale_code("ru_RU"))
        .unwrap();
    let probe = parent
        .instantiate(
            "Функция Код() Экспорт Возврат ТекущийКодЛокализации() + \"|\" + CurrentLocaleCode(); КонецФункции",
        )
        .unwrap();
    assert_eq!(
        parent.call(&probe, "Код", Vec::new()).unwrap().0,
        Value::Str("ru_RU|ru_RU".into())
    );

    let mut missing = foreground.open(principal(), resources((), 2)).unwrap();
    let probe = missing
        .instantiate("Функция Код() Экспорт Возврат ТекущийКодЛокализации(); КонецФункции")
        .unwrap();
    let error = missing
        .call(&probe, "Код", Vec::new())
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("код локализации не передан хостом"),
        "{error}"
    );

    let background = SessionFactory::new(
        code_snapshot(START_HANDLER),
        metadata_catalog("configuration"),
        "МодульСеанса",
        SessionKind::BackgroundJob,
        PLATFORM_VERSION,
    )
    .unwrap();
    let mut child = background
        .open_background(
            principal(),
            resources((), 3).with_locale_code("en_US_POSIX"),
            "Обработка.Проверка",
            &parent,
        )
        .unwrap();
    let probe = child
        .instantiate("Функция Код() Экспорт Возврат CurrentLocaleCode(); КонецФункции")
        .unwrap();
    assert_eq!(
        child.call(&probe, "Код", Vec::new()).unwrap().0,
        Value::Str("en_US_POSIX".into())
    );
}

#[test]
fn infobase_locale_code_is_distinct_and_explicit_for_each_session() {
    let foreground = SessionFactory::new(
        code_snapshot(START_HANDLER),
        metadata_catalog("configuration"),
        "МодульСеанса",
        SessionKind::ThinClient,
        PLATFORM_VERSION,
    )
    .unwrap();
    let mut parent = foreground
        .open(
            principal(),
            resources((), 1)
                .with_infobase_locale_code("ru_RU")
                .with_locale_code("en_US_POSIX"),
        )
        .unwrap();
    let probe = parent
        .instantiate(
            "Функция Код() Экспорт Возврат КодЛокализацииИнформационнойБазы() + \"|\" + CurrentLocaleCode() + \"|\" + InfoBaseLocaleCode(); КонецФункции",
        )
        .unwrap();
    assert_eq!(
        parent.call(&probe, "Код", Vec::new()).unwrap().0,
        Value::Str("ru_RU|en_US_POSIX|ru_RU".into())
    );

    let mut missing = foreground.open(principal(), resources((), 2)).unwrap();
    let probe = missing
        .instantiate("Функция Код() Экспорт Возврат InfoBaseLocaleCode(); КонецФункции")
        .unwrap();
    let error = missing
        .call(&probe, "Код", Vec::new())
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("код локализации информационной базы не передан хостом"),
        "{error}"
    );

    let background = SessionFactory::new(
        code_snapshot(START_HANDLER),
        metadata_catalog("configuration"),
        "МодульСеанса",
        SessionKind::BackgroundJob,
        PLATFORM_VERSION,
    )
    .unwrap();
    let mut child = background
        .open_background(principal(), resources((), 3), "Обработка.Проверка", &parent)
        .unwrap();
    let probe = child
        .instantiate("Функция Код() Экспорт Возврат InfoBaseLocaleCode(); КонецФункции")
        .unwrap();
    let error = child
        .call(&probe, "Код", Vec::new())
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("код локализации информационной базы не передан хостом"),
        "{error}"
    );
}

#[test]
fn current_system_language_uses_only_the_current_sessions_host_value() {
    let foreground = SessionFactory::new(
        code_snapshot(START_HANDLER),
        metadata_catalog("configuration"),
        "МодульСеанса",
        SessionKind::ThinClient,
        PLATFORM_VERSION,
    )
    .unwrap();
    let mut parent = foreground
        .open(principal(), resources((), 1).with_system_language("en"))
        .unwrap();
    let probe = parent
        .instantiate(
            "Функция Язык() Экспорт Возврат ТекущийЯзыкСистемы() + \"|\" + CurrentSystemLanguage(); КонецФункции",
        )
        .unwrap();
    assert_eq!(
        parent.call(&probe, "Язык", Vec::new()).unwrap().0,
        Value::Str("en|en".into())
    );

    let mut missing = foreground.open(principal(), resources((), 2)).unwrap();
    let probe = missing
        .instantiate("Функция Язык() Экспорт Возврат ТекущийЯзыкСистемы(); КонецФункции")
        .unwrap();
    let error = missing
        .call(&probe, "Язык", Vec::new())
        .unwrap_err()
        .to_string();
    assert!(error.contains("язык системы не передан хостом"), "{error}");

    let background = SessionFactory::new(
        code_snapshot(START_HANDLER),
        metadata_catalog("configuration"),
        "МодульСеанса",
        SessionKind::BackgroundJob,
        PLATFORM_VERSION,
    )
    .unwrap();
    let mut child = background
        .open_background(
            principal(),
            resources((), 3).with_system_language("ru"),
            "Обработка.Проверка",
            &parent,
        )
        .unwrap();
    let probe = child
        .instantiate("Функция Язык() Экспорт Возврат CurrentSystemLanguage(); КонецФункции")
        .unwrap();
    assert_eq!(
        child.call(&probe, "Язык", Vec::new()).unwrap().0,
        Value::Str("ru".into())
    );
}

#[test]
fn external_resources_mode_defaults_to_d_and_is_session_local() {
    let factory = SessionFactory::new(
        code_snapshot(START_HANDLER),
        metadata_catalog("configuration"),
        "МодульСеанса",
        SessionKind::ThinClient,
        PLATFORM_VERSION,
    )
    .unwrap();
    let mut alternative = factory
        .open(
            principal(),
            resources((), 1).with_external_resources_mode(ExternalResourcesMode::Alternative),
        )
        .unwrap();
    let probe = alternative
        .instantiate(
            "Функция Режим() Экспорт Возврат ПолучитьРежимВнешнихРесурсов() + \"|\" + GetExternalResourcesMode(); КонецФункции",
        )
        .unwrap();
    assert_eq!(
        alternative.call(&probe, "Режим", Vec::new()).unwrap().0,
        Value::Str("A|A".into())
    );

    let mut default = factory.open(principal(), resources((), 2)).unwrap();
    let probe = default
        .instantiate("Функция Режим() Экспорт Возврат GetExternalResourcesMode(); КонецФункции")
        .unwrap();
    assert_eq!(
        default.call(&probe, "Режим", Vec::new()).unwrap().0,
        Value::Str("D".into())
    );
}

#[test]
fn client_connection_speed_is_explicit_and_isolated_per_session() {
    let code = code_snapshot(START_HANDLER);
    let foreground = SessionFactory::new(
        code.clone(),
        metadata_catalog("configuration"),
        "МодульСеанса",
        SessionKind::WebClient,
        PLATFORM_VERSION,
    )
    .unwrap();
    let source =
        "Функция Скорость() Экспорт Возврат ПолучитьСкоростьКлиентскогоСоединения(); КонецФункции";
    let mut low = foreground
        .open(
            principal(),
            resources((), 51).with_client_connection_speed(ClientConnectionSpeed::Low),
        )
        .unwrap();
    let module = low.instantiate(source).unwrap();
    assert_eq!(
        low.call(&module, "Скорость", Vec::new()).unwrap().0,
        Value::Enum(EnumValue::ClientConnectionSpeedLow)
    );

    let mut normal = foreground
        .open(
            principal(),
            resources((), 52).with_client_connection_speed(ClientConnectionSpeed::Normal),
        )
        .unwrap();
    let module = normal.instantiate(source).unwrap();
    assert_eq!(
        normal.call(&module, "Скорость", Vec::new()).unwrap().0,
        Value::Enum(EnumValue::ClientConnectionSpeedNormal)
    );

    let mut missing = foreground.open(principal(), resources((), 53)).unwrap();
    let module = missing.instantiate(source).unwrap();
    assert!(
        missing
            .call(&module, "Скорость", Vec::new())
            .unwrap_err()
            .to_string()
            .contains("не передана хостом")
    );

    let background = SessionFactory::new(
        code,
        metadata_catalog("configuration"),
        "МодульСеанса",
        SessionKind::BackgroundJob,
        PLATFORM_VERSION,
    )
    .unwrap();
    let mut child = background
        .open_background(principal(), resources((), 54), "Обработка.Проверка", &low)
        .unwrap();
    let module = child.instantiate(source).unwrap();
    assert!(child.call(&module, "Скорость", Vec::new()).is_err());
}

#[test]
fn current_client_interface_variant_is_explicit_per_session() {
    let code = code_snapshot(START_HANDLER);
    let foreground = SessionFactory::new(
        code.clone(),
        metadata_catalog("configuration"),
        "МодульСеанса",
        SessionKind::ThinClient,
        PLATFORM_VERSION,
    )
    .unwrap();
    let source = "Функция Вариант() Экспорт Возврат ТекущийВариантИнтерфейсаКлиентскогоПриложения() = ClientApplicationInterfaceCurrentVariant(); КонецФункции";
    let mut taxi = foreground
        .open(
            principal(),
            resources((), 61).with_current_client_interface_variant(
                EnumValue::ClientApplicationInterfaceVariantTaxi,
            ),
        )
        .unwrap();
    let module = taxi.instantiate(source).unwrap();
    assert_eq!(
        taxi.call(&module, "Вариант", Vec::new()).unwrap().0,
        Value::Boolean(true)
    );
    let module = taxi
        .instantiate("Функция Вариант() Экспорт Возврат ТекущийВариантИнтерфейсаКлиентскогоПриложения(); КонецФункции")
        .unwrap();
    assert_eq!(
        taxi.call(&module, "Вариант", Vec::new()).unwrap().0,
        Value::Enum(EnumValue::ClientApplicationInterfaceVariantTaxi)
    );

    let mut old = foreground
        .open(
            principal(),
            resources((), 62).with_current_client_interface_variant(
                EnumValue::ClientApplicationInterfaceVariantVersion8_2,
            ),
        )
        .unwrap();
    let module = old
        .instantiate("Функция Вариант() Экспорт Возврат ClientApplicationInterfaceCurrentVariant(); КонецФункции")
        .unwrap();
    assert_eq!(
        old.call(&module, "Вариант", Vec::new()).unwrap().0,
        Value::Enum(EnumValue::ClientApplicationInterfaceVariantVersion8_2)
    );

    let mut missing = foreground.open(principal(), resources((), 63)).unwrap();
    let module = missing.instantiate(source).unwrap();
    assert!(
        missing
            .call(&module, "Вариант", Vec::new())
            .unwrap_err()
            .to_string()
            .contains("не передан хостом")
    );
    assert!(matches!(
        foreground
            .open(
                principal(),
                resources((), 64)
                    .with_current_client_interface_variant(EnumValue::ClientConnectionSpeedLow,),
            )
            .err(),
        Some(SessionError::InvalidClientInterfaceVariant)
    ));

    let background = SessionFactory::new(
        code,
        metadata_catalog("configuration"),
        "МодульСеанса",
        SessionKind::BackgroundJob,
        PLATFORM_VERSION,
    )
    .unwrap();
    let mut child = background
        .open_background(
            principal(),
            resources((), 65).with_current_client_interface_variant(
                EnumValue::ClientApplicationInterfaceVariantTaxi,
            ),
            "Обработка.Проверка",
            &taxi,
        )
        .unwrap();
    let module = child
        .instantiate("Функция Вариант() Экспорт Возврат ClientApplicationInterfaceCurrentVariant(); КонецФункции")
        .unwrap();
    assert_eq!(
        child.call(&module, "Вариант", Vec::new()).unwrap().0,
        Value::Enum(EnumValue::ClientApplicationInterfaceVariantTaxi)
    );
    let mut child_without_variant = background
        .open_background(principal(), resources((), 66), "Обработка.Проверка", &taxi)
        .unwrap();
    let module = child_without_variant
        .instantiate("Функция Вариант() Экспорт Возврат ClientApplicationInterfaceCurrentVariant(); КонецФункции")
        .unwrap();
    assert!(
        child_without_variant
            .call(&module, "Вариант", Vec::new())
            .is_err()
    );
}

#[test]
fn infobase_session_roster_never_fabricates_a_partial_array() {
    let factory = SessionFactory::new(
        code_snapshot(START_HANDLER),
        metadata_catalog("configuration"),
        "МодульСеанса",
        SessionKind::WebClient,
        PLATFORM_VERSION,
    )
    .unwrap();
    let mut session = factory.open(principal(), resources((), 1)).unwrap();
    let probe = session
        .instantiate(
            "Функция Сеансы() Экспорт Попытка ПолучитьСеансыИнформационнойБазы(); Исключение Возврат Истина; КонецПопытки; Возврат Ложь; КонецФункции",
        )
        .unwrap();
    assert_eq!(
        session.call(&probe, "Сеансы", Vec::new()).unwrap().0,
        Value::Boolean(true)
    );
    let alias = session
        .instantiate("Функция Сеансы() Экспорт Возврат GetInfoBaseSessions(); КонецФункции")
        .unwrap();
    assert!(session.call(&alias, "Сеансы", Vec::new()).is_err());
}

#[test]
fn scheduled_jobs_global_is_catchably_unsupported() {
    let factory = SessionFactory::new(
        code_snapshot(START_HANDLER),
        metadata_catalog("configuration"),
        "МодульСеанса",
        SessionKind::WebClient,
        PLATFORM_VERSION,
    )
    .unwrap();
    let mut session = factory.open(principal(), resources((), 1)).unwrap();
    let probe = session
        .instantiate(
            "Функция Проверка() Экспорт Попытка Задания = РегламентныеЗадания; Исключение Возврат Истина; КонецПопытки; Возврат Ложь; КонецФункции",
        )
        .unwrap();
    assert_eq!(
        session.call(&probe, "Проверка", Vec::new()).unwrap().0,
        Value::Boolean(true)
    );
}

#[test]
fn collaboration_system_global_is_catchably_unsupported() {
    let factory = SessionFactory::new(
        code_snapshot(START_HANDLER),
        metadata_catalog("configuration"),
        "МодульСеанса",
        SessionKind::WebClient,
        PLATFORM_VERSION,
    )
    .unwrap();
    let mut session = factory.open(principal(), resources((), 1)).unwrap();
    let probe = session
        .instantiate(
            "Функция Проверка() Экспорт Попытка Сервис = СистемаВзаимодействия; Исключение Возврат Истина; КонецПопытки; Возврат Ложь; КонецФункции",
        )
        .unwrap();
    assert_eq!(
        session.call(&probe, "Проверка", Vec::new()).unwrap().0,
        Value::Boolean(true)
    );
}

#[test]
fn collaboration_filter_constructor_is_catchably_unsupported() {
    let factory = SessionFactory::new(
        code_snapshot(START_HANDLER),
        metadata_catalog("configuration"),
        "МодульСеанса",
        SessionKind::WebClient,
        PLATFORM_VERSION,
    )
    .unwrap();
    let mut session = factory.open(principal(), resources((), 1)).unwrap();
    let probe = session
        .instantiate(
            "Функция Проверка() Экспорт Попытка Отбор = Новый ОтборОбсужденийСистемыВзаимодействия; Исключение Возврат Истина; КонецПопытки; Возврат Ложь; КонецФункции",
        )
        .unwrap();
    assert_eq!(
        session.call(&probe, "Проверка", Vec::new()).unwrap().0,
        Value::Boolean(true)
    );
    let messages = session
        .instantiate(
            "Функция Проверка() Экспорт Попытка Отбор = New CollaborationSystemMessagesFilter(); Исключение Возврат Истина; КонецПопытки; Возврат Ложь; КонецФункции",
        )
        .unwrap();
    assert_eq!(
        session.call(&messages, "Проверка", Vec::new()).unwrap().0,
        Value::Boolean(true)
    );
}

#[test]
fn collaboration_conversation_id_constructor_is_catchably_unsupported() {
    let factory = SessionFactory::new(
        code_snapshot(START_HANDLER),
        metadata_catalog("configuration"),
        "МодульСеанса",
        SessionKind::WebClient,
        PLATFORM_VERSION,
    )
    .unwrap();
    let mut session = factory.open(principal(), resources((), 1)).unwrap();
    let probe = session
        .instantiate(
            "Функция Проверка() Экспорт Попытка Идентификатор = Новый ИдентификаторОбсужденияСистемыВзаимодействия(\"x\"); Исключение Возврат Истина; КонецПопытки; Возврат Ложь; КонецФункции",
        )
        .unwrap();
    assert_eq!(
        session.call(&probe, "Проверка", Vec::new()).unwrap().0,
        Value::Boolean(true)
    );
    let english = session
        .instantiate(
            "Function Check() Export Try Identifier = New CollaborationSystemConversationID(\"x\"); Except Return True; EndTry; Return False; EndFunction",
        )
        .unwrap();
    assert_eq!(
        session.call(&english, "Check", Vec::new()).unwrap().0,
        Value::Boolean(true)
    );
}

#[test]
fn collaboration_conversation_context_constructor_is_catchably_unsupported() {
    let factory = SessionFactory::new(
        code_snapshot(START_HANDLER),
        metadata_catalog("configuration"),
        "МодульСеанса",
        SessionKind::WebClient,
        PLATFORM_VERSION,
    )
    .unwrap();
    let mut session = factory.open(principal(), resources((), 1)).unwrap();
    let probe = session
        .instantiate(
            "Функция Проверка() Экспорт Попытка Контекст = Новый КонтекстОбсужденияСистемыВзаимодействия(\"e1cib/data/Catalog.X\"); Исключение Возврат Истина; КонецПопытки; Возврат Ложь; КонецФункции",
        )
        .unwrap();
    assert_eq!(
        session.call(&probe, "Проверка", Vec::new()).unwrap().0,
        Value::Boolean(true)
    );
}

#[test]
fn collaboration_application_constructors_keep_measured_arity_and_fail_explicitly() {
    let factory = SessionFactory::new(
        code_snapshot(START_HANDLER),
        metadata_catalog("configuration"),
        "МодульСеанса",
        SessionKind::WebClient,
        PLATFORM_VERSION,
    )
    .unwrap();
    let mut session = factory.open(principal(), resources((), 1)).unwrap();
    for expression in [
        "Новый ИдентификаторПриложенияСистемыВзаимодействия(\"x\")",
        "New CollaborationSystemApplicationID(\"x\")",
        "Новый ИдентификаторСообщенияСистемыВзаимодействия(\"x\")",
        "New CollaborationSystemMessageID(\"x\")",
        "Новый КоллекцияИдентификаторовПриложенийСистемыВзаимодействия",
        "New CollaborationSystemApplicationIDCollection",
        "Новый СовместноеИспользованиеПриложенийСистемыВзаимодействия",
        "New CollaborationSystemApplicationLinks",
    ] {
        let source = format!(
            "Функция Проверка() Экспорт Попытка Значение = {expression}; Исключение Возврат Истина; КонецПопытки; Возврат Ложь; КонецФункции"
        );
        let module = session.instantiate(&source).unwrap();
        assert_eq!(
            session.call(&module, "Проверка", Vec::new()).unwrap().0,
            Value::Boolean(true),
            "{expression}"
        );
    }
    assert!(
        session
            .instantiate(
                "Функция Проверка() Экспорт Возврат Новый ИдентификаторПриложенияСистемыВзаимодействия; КонецФункции"
            )
            .is_err()
    );
}

#[test]
fn internet_mail_message_constructor_is_catchably_unsupported() {
    let factory = SessionFactory::new(
        code_snapshot(START_HANDLER),
        metadata_catalog("configuration"),
        "МодульСеанса",
        SessionKind::WebClient,
        PLATFORM_VERSION,
    )
    .unwrap();
    let mut session = factory.open(principal(), resources((), 1)).unwrap();
    let probe = session
        .instantiate(
            "Функция Проверка() Экспорт Попытка Сообщение = Новый ИнтернетПочтовоеСообщение; Исключение Возврат Истина; КонецПопытки; Возврат Ложь; КонецФункции",
        )
        .unwrap();
    assert_eq!(
        session.call(&probe, "Проверка", Vec::new()).unwrap().0,
        Value::Boolean(true)
    );
    let english = session
        .instantiate(
            "Function Check() Export Try Message = New InternetMailMessage(); Except Return True; EndTry; Return False; EndFunction",
        )
        .unwrap();
    assert_eq!(
        session.call(&english, "Check", Vec::new()).unwrap().0,
        Value::Boolean(true)
    );
}

#[derive(Debug, Default)]
struct RecordingEventLog {
    entries: RefCell<Vec<EventLogEntry>>,
    event_use_reads: RefCell<Vec<String>>,
    event_use_writes: RefCell<Vec<(String, bool, Value)>>,
}

impl EventLogWriter for RecordingEventLog {
    fn write(&self, entry: EventLogEntry) -> RtResult<()> {
        self.entries.borrow_mut().push(entry);
        Ok(())
    }

    fn event_use(&self, event_name: &str) -> RtResult<EventLogEventUseData> {
        self.event_use_reads.borrow_mut().push(event_name.into());
        Ok(EventLogEventUseData::new(false, Value::number_from_i64(42)))
    }

    fn set_event_use(&self, event_name: &str, use_data: &EventLogEventUseData) -> RtResult<()> {
        self.event_use_writes.borrow_mut().push((
            event_name.into(),
            use_data.use_event(),
            use_data.description().clone(),
        ));
        Ok(())
    }
}

#[derive(Debug)]
struct LevelEventLog(Vec<EventLogLevel>);

impl EventLogWriter for LevelEventLog {
    fn write(&self, _: EventLogEntry) -> RtResult<()> {
        Ok(())
    }

    fn enabled_levels(&self) -> RtResult<Vec<EventLogLevel>> {
        Ok(self.0.clone())
    }
}

#[derive(Debug)]
struct ConstantAccess;

impl DataAccess for ConstantAccess {
    fn constant_value(
        &self,
        metadata: &MetadataObject,
        principal: &Principal,
        _: &mut CallContext<'_>,
    ) -> RtResult<Value> {
        assert_eq!(metadata.id(), "constant-answer");
        assert_eq!(principal.id(), "user");
        Ok(Value::number_from_i64(42))
    }
}

fn candidate(id: &str, name: &str, can_authenticate: bool) -> PrincipalCandidate {
    PrincipalCandidate::new(
        Principal::new(id, name, vec![format!("Роль{name}")]),
        can_authenticate,
    )
}

#[test]
fn principal_selection_accepts_case_insensitive_name_and_exact_id() {
    let candidates = [
        candidate("first-id", "Первый", true),
        candidate("second-id", "Второй", true),
    ];

    assert_eq!(
        select_principal(&candidates, Some("пЕрВыЙ"), Vec::new())
            .unwrap()
            .id(),
        "first-id"
    );
    assert_eq!(
        select_principal(&candidates, Some("second-id"), Vec::new())
            .unwrap()
            .name(),
        "Второй"
    );
}

#[test]
fn principal_selection_rejects_forbidden_unknown_and_ambiguous_users() {
    let candidates = [
        candidate("first-id", "Первый", true),
        candidate("second-id", "Второй", true),
        candidate("blocked-id", "Запрещённый", false),
    ];

    assert!(matches!(
        select_principal(&candidates, Some("Запрещённый"), Vec::new()),
        Err(PrincipalSelectionError::Forbidden(name)) if name == "Запрещённый"
    ));
    assert!(matches!(
        select_principal(&candidates, Some("НетТакого"), Vec::new()),
        Err(PrincipalSelectionError::Unknown(name)) if name == "НетТакого"
    ));
    assert!(matches!(
        select_principal(&candidates, None, Vec::new()),
        Err(PrincipalSelectionError::Ambiguous(names))
            if names == ["Первый", "Второй"]
    ));
}

#[test]
fn principal_selection_automatically_uses_the_only_eligible_user() {
    let candidates = [
        candidate("allowed-id", "Разрешённый", true),
        candidate("blocked-id", "Запрещённый", false),
    ];

    let selected = select_principal(&candidates, None, Vec::new()).unwrap();
    assert_eq!(selected.id(), "allowed-id");
    assert_eq!(selected.name(), "Разрешённый");
}

#[test]
fn empty_user_list_creates_empty_principal_with_main_roles() {
    let selected = select_principal(&[], None, ["ОсновнаяРоль".to_string()]).unwrap();

    assert_eq!(selected.id(), "");
    assert_eq!(selected.name(), "");
    assert_eq!(selected.role_ids().collect::<Vec<_>>(), ["ОсновнаяРоль"]);
}

const START_HANDLER: &str = r#"
    Перем ЧислоЗапусков;
    ЧислоЗапусков = 0;
    Процедура УстановкаПараметровСеанса(Требуемые) Экспорт
        Если Требуемые = Неопределено Тогда
            ЧислоЗапусков = ЧислоЗапусков + 1;
            ПараметрыСеанса.Запущен = ЧислоЗапусков;
        ИначеЕсли Требуемые[0] = "Копируемый" Тогда
            ПараметрыСеанса.Копируемый = 2;
        КонецЕсли;
    КонецПроцедуры
"#;
const PLATFORM_VERSION: &str = "8.3.27.2342";

#[test]
fn startup_calls_a_non_exported_session_handler() {
    let factory = SessionFactory::new(
        code_snapshot(
            "Процедура УстановкаПараметровСеанса(Требуемые) Если Требуемые = Неопределено Тогда ПараметрыСеанса.Запущен = 1; КонецЕсли; КонецПроцедуры",
        ),
        metadata_catalog("configuration"),
        "МодульСеанса",
        SessionKind::ThinClient,
        PLATFORM_VERSION,
    )
    .unwrap();
    let session = factory.open(principal(), resources((), 1)).unwrap();
    assert_eq!(
        session.parameters().peek("Запущен").unwrap(),
        Some(Value::number_from_i64(1))
    );
}

#[test]
fn selective_parameter_initialization_reenters_the_private_session_handler() {
    let factory = SessionFactory::new(
        code_snapshot(
            "Перем Вызовы;\nВызовы = 0;\nПроцедура УстановкаПараметровСеанса(Требуемые)\nВызовы = Вызовы + 1;\nЕсли Требуемые = Неопределено Тогда\nПараметрыСеанса.Запущен = ПараметрыСеанса.Копируемый;\nИначе\nПараметрыСеанса.Копируемый = Вызовы;\nКонецЕсли;\nКонецПроцедуры",
        ),
        metadata_catalog("configuration"),
        "МодульСеанса",
        SessionKind::ThinClient,
        PLATFORM_VERSION,
    )
    .unwrap();
    let session = factory.open(principal(), resources((), 1)).unwrap();
    assert_eq!(
        session.parameters().peek("Запущен").unwrap(),
        Some(Value::number_from_i64(2))
    );
}

#[test]
fn factory_validates_once_and_opens_isolated_started_sessions() {
    #[derive(Debug, PartialEq)]
    struct Connection(u8);

    let factory = SessionFactory::new(
        code_snapshot(START_HANDLER),
        metadata_catalog("configuration"),
        "мОдУлЬсЕаНсА",
        SessionKind::ThinClient,
        PLATFORM_VERSION,
    )
    .unwrap();
    let mut first = factory
        .open(principal(), resources(Connection(1), 101))
        .unwrap();
    let second = factory
        .open(principal(), resources(Connection(2), 102))
        .unwrap();

    assert_eq!(
        first.parameters().peek("Запущен").unwrap(),
        Some(Value::number_from_i64(1))
    );
    assert_eq!(first.connection::<Connection>(), Some(&Connection(1)));
    assert_eq!(second.connection::<Connection>(), Some(&Connection(2)));
    first
        .parameters()
        .set("Запущен", Value::number_from_i64(9))
        .unwrap();
    assert_eq!(
        second.parameters().peek("Запущен").unwrap(),
        Some(Value::number_from_i64(1))
    );

    let module = first
        .instantiate("Функция Значение() Экспорт Возврат ПараметрыСеанса.Запущен; КонецФункции")
        .unwrap();
    assert_eq!(
        first.call(&module, "Значение", vec![]).unwrap().0,
        Value::number_from_i64(9)
    );
}

#[test]
fn factory_compiles_session_module_without_external_connection_branches() {
    let source = r#"
        Процедура УстановкаПараметровСеанса(Требуемые) Экспорт
            #Если ВнешнееСоединение Тогда
                НеизвестнаяВеткаВнешнегоСоединения();
            #КонецЕсли
            ПараметрыСеанса.Запущен = Истина;
        КонецПроцедуры
    "#;

    assert!(
        SessionFactory::new(
            code_snapshot(source),
            metadata_catalog("configuration"),
            "МодульСеанса",
            SessionKind::WebClient,
            PLATFORM_VERSION,
        )
        .is_ok()
    );
}

#[test]
fn factory_selects_only_server_common_modules_for_session_startup() {
    let code = ConfigurationCodeSnapshot::new(
        "configuration",
        vec![
            ConfigurationModule::new(
                "session",
                "МодульСеанса",
                "Процедура УстановкаПараметровСеанса(Требуемые) Экспорт Если КонтекстноеЗначение() <> 2 Тогда ВызватьИсключение \"не сервер\"; КонецЕсли; КонецПроцедуры",
                ModuleKind::Session,
                ModuleExecution::new().with_server(true),
            ),
            ConfigurationModule::new(
                "client",
                "КлиентскийГлобальный",
                "Функция КонтекстноеЗначение() Экспорт Возврат 1; КонецФункции",
                ModuleKind::Common,
                ModuleExecution::new().with_client(true).with_global(true),
            ),
            ConfigurationModule::new(
                "server",
                "СерверныйГлобальный",
                "Функция КонтекстноеЗначение() Экспорт Возврат 2; КонецФункции",
                ModuleKind::Common,
                ModuleExecution::new().with_server(true).with_global(true),
            ),
        ],
    )
    .unwrap();
    let factory = SessionFactory::new(
        code,
        Arc::new(MetadataCatalog::new("configuration", vec![]).unwrap()),
        "МодульСеанса",
        SessionKind::ThinClient,
        PLATFORM_VERSION,
    )
    .unwrap();

    factory.open(principal(), resources((), 1)).unwrap();
}

#[test]
fn factory_still_rejects_duplicate_server_global_exports() {
    let mut modules = vec![ConfigurationModule::new(
        "session",
        "МодульСеанса",
        "Процедура УстановкаПараметровСеанса(Требуемые) Экспорт КонецПроцедуры",
        ModuleKind::Session,
        ModuleExecution::new().with_server(true),
    )];
    for (id, name) in [("one", "Первый"), ("two", "Второй")] {
        modules.push(ConfigurationModule::new(
            id,
            name,
            "Функция ОдинаковыйЭкспорт() Экспорт Возврат 1; КонецФункции",
            ModuleKind::Common,
            ModuleExecution::new().with_server(true).with_global(true),
        ));
    }
    let Err(error) = SessionFactory::new(
        ConfigurationCodeSnapshot::new("configuration", modules).unwrap(),
        Arc::new(MetadataCatalog::new("configuration", vec![]).unwrap()),
        "МодульСеанса",
        SessionKind::ThinClient,
        PLATFORM_VERSION,
    ) else {
        panic!("ожидался конфликт серверных глобальных экспортов");
    };

    assert!(
        error
            .to_string()
            .contains("объявлен несколькими общими модулями")
    );
}

#[test]
fn startup_reads_configuration_data_through_its_session_adapter() {
    let factory = SessionFactory::new(
        code_snapshot(
            "Процедура УстановкаПараметровСеанса(Требуемые) Экспорт ПараметрыСеанса.Запущен = Константы.Ответ.Получить(); КонецПроцедуры",
        ),
        metadata_catalog("configuration"),
        "МодульСеанса",
        SessionKind::ThinClient,
        PLATFORM_VERSION,
    )
    .unwrap();

    let session = factory
        .open(
            principal(),
            resources((), 1).with_data_access(Rc::new(ConstantAccess)),
        )
        .unwrap();
    assert_eq!(
        session.parameters().peek("Запущен").unwrap(),
        Some(Value::number_from_i64(42))
    );
}

#[test]
fn startup_reads_the_factory_platform_version() {
    let factory = SessionFactory::new(
        code_snapshot(
            "Процедура УстановкаПараметровСеанса(Требуемые) Экспорт Инфо = Новый СистемнаяИнформация; ПараметрыСеанса.Запущен = Инфо.ВерсияПриложения; КонецПроцедуры",
        ),
        metadata_catalog("configuration"),
        "МодульСеанса",
        SessionKind::ThinClient,
        PLATFORM_VERSION,
    )
    .unwrap();

    let session = factory.open(principal(), resources((), 1)).unwrap();
    assert_eq!(
        session.parameters().peek("Запущен").unwrap(),
        Some(Value::Str(PLATFORM_VERSION.into()))
    );
}

#[test]
fn infobase_url_resolves_but_does_not_guess_from_connection_string() {
    let factory = SessionFactory::new(
        code_snapshot(START_HANDLER),
        metadata_catalog("configuration"),
        "МодульСеанса",
        SessionKind::WebClient,
        PLATFORM_VERSION,
    )
    .unwrap()
    .with_connection_string("Srvr=cluster;Ref=demo;")
    .unwrap();
    let mut session = factory.open(principal(), resources((), 1)).unwrap();
    let module = session
        .instantiate(
            "Функция Проверить() Экспорт Возврат ПолучитьНавигационнуюСсылкуИнформационнойБазы(); КонецФункции",
        )
        .unwrap();
    let error = session.call(&module, "Проверить", vec![]).unwrap_err();
    assert!(error.to_string().contains("не предоставлена хостом"));
}

#[test]
fn connection_string_is_exact_session_data_and_missing_value_fails() {
    const FOREGROUND: &str = "File=/srv/1c/demo;Usr=Тест;";
    const BACKGROUND: &str = "Srvr=cluster;Ref=demo;";

    let foreground_factory = SessionFactory::new(
        code_snapshot(START_HANDLER),
        metadata_catalog("configuration"),
        "МодульСеанса",
        SessionKind::ThinClient,
        PLATFORM_VERSION,
    )
    .unwrap()
    .with_connection_string(FOREGROUND)
    .unwrap();
    let mut foreground = foreground_factory
        .open(principal(), resources((), 1))
        .unwrap();
    let probe = foreground
        .instantiate(
            "Функция Значение() Экспорт Возврат СтрокаСоединенияИнформационнойБазы(); КонецФункции",
        )
        .unwrap();
    assert_eq!(
        foreground.call(&probe, "Значение", vec![]).unwrap().0,
        Value::Str(FOREGROUND.into())
    );

    let background_factory = SessionFactory::new(
        code_snapshot(START_HANDLER),
        metadata_catalog("configuration"),
        "МодульСеанса",
        SessionKind::BackgroundJob,
        PLATFORM_VERSION,
    )
    .unwrap()
    .with_connection_string(BACKGROUND)
    .unwrap();
    let mut background = background_factory
        .open_background(
            principal(),
            resources((), 2),
            "Обработка.ФоновыйМетод",
            &foreground,
        )
        .unwrap();
    let probe = background
        .instantiate("Функция Значение() Экспорт Возврат InfoBaseConnectionString(); КонецФункции")
        .unwrap();
    assert_eq!(
        background.call(&probe, "Значение", vec![]).unwrap().0,
        Value::Str(BACKGROUND.into())
    );

    let missing_factory = SessionFactory::new(
        code_snapshot(START_HANDLER),
        metadata_catalog("configuration"),
        "МодульСеанса",
        SessionKind::WebClient,
        PLATFORM_VERSION,
    )
    .unwrap();
    let mut missing = missing_factory.open(principal(), resources((), 1)).unwrap();
    let probe = missing
        .instantiate(
            "Функция Значение() Экспорт Возврат СтрокаСоединенияИнформационнойБазы(); КонецФункции",
        )
        .unwrap();
    let error = missing.call(&probe, "Значение", vec![]).unwrap_err();
    assert!(error.to_string().contains("не передана хостом"));

    assert!(matches!(
        SessionFactory::new(
            code_snapshot(START_HANDLER),
            metadata_catalog("configuration"),
            "МодульСеанса",
            SessionKind::WebClient,
            PLATFORM_VERSION,
        )
        .unwrap()
        .with_connection_string(""),
        Err(SessionError::EmptyConnectionString)
    ));
}

#[test]
fn connection_number_belongs_to_the_concrete_session_connection() {
    let code = code_snapshot(
        "Процедура УстановкаПараметровСеанса(Требуемые) Экспорт ПараметрыСеанса.Запущен = НомерСоединенияИнформационнойБазы(); КонецПроцедуры",
    );
    let foreground_factory = SessionFactory::new(
        code.clone(),
        metadata_catalog("configuration"),
        "МодульСеанса",
        SessionKind::ThinClient,
        PLATFORM_VERSION,
    )
    .unwrap();
    let mut first = foreground_factory
        .open(principal(), resources((), 41))
        .unwrap();
    let mut second = foreground_factory
        .open(principal(), resources((), 42))
        .unwrap();

    for (session, expected) in [(&mut first, 41), (&mut second, 42)] {
        assert_eq!(
            session.parameters().peek("Запущен").unwrap(),
            Some(Value::number_from_i64(expected))
        );
        let probe = session
            .instantiate("Функция Номер() Экспорт Возврат InfoBaseConnectionNumber(); КонецФункции")
            .unwrap();
        assert_eq!(
            session.call(&probe, "Номер", vec![]).unwrap().0,
            Value::number_from_i64(expected)
        );
    }

    let background_factory = SessionFactory::new(
        code,
        metadata_catalog("configuration"),
        "МодульСеанса",
        SessionKind::BackgroundJob,
        PLATFORM_VERSION,
    )
    .unwrap();
    let background = background_factory
        .open_background(
            principal(),
            resources((), 43),
            "Обработка.ФоновыйМетод",
            &first,
        )
        .unwrap();
    assert_eq!(
        background.parameters().peek("Запущен").unwrap(),
        Some(Value::number_from_i64(43))
    );
    assert_eq!(
        first.parameters().peek("Запущен").unwrap(),
        Some(Value::number_from_i64(41))
    );
}

#[test]
fn connection_number_must_be_positive_and_representable_in_bsl() {
    let factory = SessionFactory::new(
        code_snapshot(START_HANDLER),
        metadata_catalog("configuration"),
        "МодульСеанса",
        SessionKind::ThinClient,
        PLATFORM_VERSION,
    )
    .unwrap();

    for number in [0, i64::MAX as u64 + 1] {
        let error = factory
            .open(principal(), resources((), number))
            .err()
            .unwrap();
        assert!(matches!(
            error,
            SessionError::InvalidConnectionNumber(value) if value == number
        ));
    }
}

#[test]
fn session_number_is_distinct_from_connection_and_isolated_per_session() {
    let code = code_snapshot(START_HANDLER);
    let foreground = SessionFactory::new(
        code.clone(),
        metadata_catalog("configuration"),
        "МодульСеанса",
        SessionKind::ThinClient,
        PLATFORM_VERSION,
    )
    .unwrap();
    let mut first = foreground
        .open(principal(), resources((), 41).with_session_number(71))
        .unwrap();
    let mut second = foreground
        .open(principal(), resources((), 42).with_session_number(72))
        .unwrap();
    let source = "Функция Проверить() Экспорт Возврат InfoBaseSessionNumber() = ПолучитьТекущийСеансИнформационнойБазы().SessionNumber И InfoBaseSessionNumber() <> InfoBaseConnectionNumber(); КонецФункции";
    for (session, expected) in [(&mut first, 71), (&mut second, 72)] {
        let module = session.instantiate(source).unwrap();
        assert_eq!(
            session.call(&module, "Проверить", Vec::new()).unwrap().0,
            Value::Boolean(true)
        );
        let module = session
            .instantiate(
                "Функция Номер() Экспорт Возврат НомерСеансаИнформационнойБазы(); КонецФункции",
            )
            .unwrap();
        assert_eq!(
            session.call(&module, "Номер", Vec::new()).unwrap().0,
            Value::number_from_i64(expected)
        );
    }

    let background = SessionFactory::new(
        code,
        metadata_catalog("configuration"),
        "МодульСеанса",
        SessionKind::BackgroundJob,
        PLATFORM_VERSION,
    )
    .unwrap();
    let mut child = background
        .open_background(
            principal(),
            resources((), 43).with_session_number(73),
            "Обработка.ФоновыйМетод",
            &first,
        )
        .unwrap();
    let module = child.instantiate(source).unwrap();
    assert_eq!(
        child.call(&module, "Проверить", Vec::new()).unwrap().0,
        Value::Boolean(true)
    );
    let module = child
        .instantiate(
            "Функция Номер() Экспорт Возврат НомерСеансаИнформационнойБазы(); КонецФункции",
        )
        .unwrap();
    assert_eq!(
        child.call(&module, "Номер", Vec::new()).unwrap().0,
        Value::number_from_i64(73)
    );
}

#[test]
fn absent_or_invalid_session_number_is_never_synthesized() {
    let factory = SessionFactory::new(
        code_snapshot(START_HANDLER),
        metadata_catalog("configuration"),
        "МодульСеанса",
        SessionKind::ThinClient,
        PLATFORM_VERSION,
    )
    .unwrap();
    let mut session = factory.open(principal(), resources((), 41)).unwrap();
    for source in [
        "Функция Номер() Экспорт Возврат НомерСеансаИнформационнойБазы(); КонецФункции",
        "Функция Номер() Экспорт Возврат ПолучитьТекущийСеансИнформационнойБазы().НомерСеанса; КонецФункции",
    ] {
        let module = session.instantiate(source).unwrap();
        assert!(session.call(&module, "Номер", Vec::new()).is_err());
    }
    for number in [0, i64::MAX as u64 + 1] {
        let error = factory
            .open(principal(), resources((), 41).with_session_number(number))
            .err()
            .unwrap();
        assert!(matches!(
            error,
            SessionError::InvalidSessionNumber(value) if value == number
        ));
    }
}

#[test]
fn authorized_connection_snapshot_is_exposed_with_both_language_names() {
    let factory = SessionFactory::new(
        code_snapshot(START_HANDLER),
        metadata_catalog("configuration"),
        "МодульСеанса",
        SessionKind::ThinClient,
        PLATFORM_VERSION,
    )
    .unwrap();
    let mut session = factory
        .open(
            principal(),
            resources((), 41).with_connections([
                InfoBaseConnection::new(7, "BackgroundJob", "server"),
                InfoBaseConnection::new(41, "ThinClient", "client"),
            ]),
        )
        .unwrap();
    let module = session
        .instantiate(
            r#"
            Функция Сведения() Экспорт
                Соединения = ПолучитьСоединенияИнформационнойБазы();
                Текущее = Соединения[1];
                Возврат Строка(ТипЗнч(Текущее)) + "|"
                    + Строка(Текущее.НомерСоединения) + "|"
                    + Текущее.ApplicationName + "|"
                    + Текущее.ИмяКомпьютера + "|"
                    + Строка(GetInfoBaseConnections().Count());
            КонецФункции

            Функция НовыйМассив() Экспорт
                Соединения = ПолучитьСоединенияИнформационнойБазы();
                Соединения.Очистить();
                Возврат GetInfoBaseConnections().Количество();
            КонецФункции
            "#,
        )
        .unwrap();

    assert_eq!(
        session.call(&module, "Сведения", vec![]).unwrap().0,
        Value::Str("Active connection|41|ThinClient|client|2".into())
    );
    assert_eq!(
        session.call(&module, "НовыйМассив", vec![]).unwrap().0,
        Value::number_from_i64(2)
    );
}

#[test]
fn unavailable_connection_snapshot_fails_explicitly() {
    let factory = SessionFactory::new(
        code_snapshot(START_HANDLER),
        metadata_catalog("configuration"),
        "МодульСеанса",
        SessionKind::ThinClient,
        PLATFORM_VERSION,
    )
    .unwrap();
    let mut session = factory.open(principal(), resources((), 41)).unwrap();
    let module = session
        .instantiate(
            "Функция Соединения() Экспорт Возврат ПолучитьСоединенияИнформационнойБазы(); КонецФункции",
        )
        .unwrap();

    let error = session.call(&module, "Соединения", vec![]).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("снимок соединений информационной базы недоступен")
    );
}

#[test]
fn invalid_connection_snapshots_fail_before_configuration_startup() {
    let factory = SessionFactory::new(
        code_snapshot(
            "Процедура УстановкаПараметровСеанса(Требуемые) Экспорт ВызватьИсключение \"startup ran\"; КонецПроцедуры",
        ),
        metadata_catalog("configuration"),
        "МодульСеанса",
        SessionKind::ThinClient,
        PLATFORM_VERSION,
    )
    .unwrap();

    let missing = factory
        .open(
            principal(),
            resources((), 41).with_connections([InfoBaseConnection::new(
                7,
                "BackgroundJob",
                "server",
            )]),
        )
        .err()
        .unwrap();
    assert!(matches!(
        missing,
        SessionError::MissingCurrentConnection(41)
    ));

    for number in [0, i64::MAX as u64 + 1] {
        let invalid = factory
            .open(
                principal(),
                resources((), 41).with_connections([
                    InfoBaseConnection::new(41, "ThinClient", "client"),
                    InfoBaseConnection::new(number, "BackgroundJob", "server"),
                ]),
            )
            .err()
            .unwrap();
        assert!(matches!(
            invalid,
            SessionError::InvalidListedConnectionNumber(value) if value == number
        ));
    }
}

#[test]
fn background_session_uses_its_own_connection_snapshot() {
    let foreground_factory = SessionFactory::new(
        code_snapshot(START_HANDLER),
        metadata_catalog("configuration"),
        "МодульСеанса",
        SessionKind::ThinClient,
        PLATFORM_VERSION,
    )
    .unwrap();
    let mut foreground = foreground_factory
        .open(
            principal(),
            resources((), 41).with_connections([InfoBaseConnection::new(
                41,
                "ThinClient",
                "client",
            )]),
        )
        .unwrap();
    let background_factory = SessionFactory::new(
        code_snapshot(START_HANDLER),
        metadata_catalog("configuration"),
        "МодульСеанса",
        SessionKind::BackgroundJob,
        PLATFORM_VERSION,
    )
    .unwrap();
    let mut background = background_factory
        .open_background(
            principal(),
            resources((), 43).with_connections([
                InfoBaseConnection::new(41, "ThinClient", "client"),
                InfoBaseConnection::new(43, "BackgroundJob", "server"),
            ]),
            "Обработка.ФоновыйМетод",
            &foreground,
        )
        .unwrap();
    let foreground_probe = foreground
        .instantiate(
            "Функция Количество() Экспорт Возврат ПолучитьСоединенияИнформационнойБазы().Количество(); КонецФункции",
        )
        .unwrap();
    let background_probe = background
        .instantiate(
            "Функция ПоследнийНомер() Экспорт С = ПолучитьСоединенияИнформационнойБазы(); Возврат С[1].НомерСоединения; КонецФункции",
        )
        .unwrap();

    assert_eq!(
        foreground
            .call(&foreground_probe, "Количество", vec![])
            .unwrap()
            .0,
        Value::number_from_i64(1)
    );
    assert_eq!(
        background
            .call(&background_probe, "ПоследнийНомер", vec![])
            .unwrap()
            .0,
        Value::number_from_i64(43)
    );
}

#[test]
fn safe_and_privileged_modes_follow_the_measured_frame_scope() {
    let factory = SessionFactory::new(
        code_snapshot(START_HANDLER),
        metadata_catalog("configuration"),
        "МодульСеанса",
        SessionKind::ThinClient,
        PLATFORM_VERSION,
    )
    .unwrap();
    let mut session = factory.open(principal(), resources((), 1)).unwrap();
    let module = session
        .instantiate(
            r#"
            Функция СнятьБезопасныйРежимРодителя()
                Попытка
                    УстановитьБезопасныйРежим(Ложь);
                    Возврат "NO_ERROR";
                Исключение
                    Возврат "ERROR";
                КонецПопытки;
            КонецФункции

            Функция БезопасныйРодитель() Экспорт
                УстановитьБезопасныйРежим(Истина);
                Результат = СнятьБезопасныйРежимРодителя() + ":"
                    + Строка(БезопасныйРежим());
                УстановитьБезопасныйРежим(Ложь);
                Возврат Результат;
            КонецФункции

            Функция БезопасныйСчетчик() Экспорт
                УстановитьБезопасныйРежим(Истина);
                УстановитьБезопасныйРежим(Истина);
                УстановитьБезопасныйРежим(Ложь);
                Первое = БезопасныйРежим();
                УстановитьБезопасныйРежим(Ложь);
                Возврат Строка(Первое) + ":" + Строка(БезопасныйРежим());
            КонецФункции

            Функция СнятьПривилегированныйРежимРодителя()
                Попытка
                    УстановитьПривилегированныйРежим(Ложь);
                    Возврат "NO_ERROR";
                Исключение
                    Возврат "ERROR";
                КонецПопытки;
            КонецФункции

            Функция ПривилегированныйРодитель() Экспорт
                УстановитьПривилегированныйРежим(Истина);
                Результат = СнятьПривилегированныйРежимРодителя() + ":"
                    + Строка(ПривилегированныйРежим());
                УстановитьПривилегированныйРежим(Ложь);
                Возврат Результат;
            КонецФункции

            Функция ПривилегированныйСчетчик() Экспорт
                УстановитьПривилегированныйРежим(Истина);
                УстановитьПривилегированныйРежим(Истина);
                УстановитьПривилегированныйРежим(Ложь);
                Первое = ПривилегированныйРежим();
                УстановитьПривилегированныйРежим(Ложь);
                Возврат Строка(Первое) + ":" + Строка(ПривилегированныйРежим());
            КонецФункции

            Процедура БезопасныйСбой()
                УстановитьБезопасныйРежим(Истина);
                ВызватьИсключение "сбой";
            КонецПроцедуры

            Процедура БезопасныйВозврат()
                УстановитьБезопасныйРежим(Истина);
            КонецПроцедуры

            Функция ПослеБезопасногоВозврата() Экспорт
                БезопасныйВозврат();
                Возврат БезопасныйРежим();
            КонецФункции

            Процедура ПривилегированныйСбой()
                УстановитьПривилегированныйРежим(Истина);
                ВызватьИсключение "сбой";
            КонецПроцедуры

            Процедура ПривилегированныйВозврат()
                УстановитьПривилегированныйРежим(Истина);
            КонецПроцедуры

            Функция ПослеПривилегированногоВозврата() Экспорт
                ПривилегированныйВозврат();
                Возврат ПривилегированныйРежим();
            КонецФункции

            Функция ПослеПривилегированногоСбоя() Экспорт
                Попытка
                    ПривилегированныйСбой();
                Исключение
                КонецПопытки;
                Возврат ПривилегированныйРежим();
            КонецФункции

            Функция ПослеБезопасногоСбоя() Экспорт
                Попытка
                    БезопасныйСбой();
                Исключение
                КонецПопытки;
                Возврат БезопасныйРежим();
            КонецФункции

            Функция ПривилегированныйВБезопасном() Экспорт
                УстановитьБезопасныйРежим(Истина);
                УстановитьПривилегированныйРежим(Истина);
                ВБезопасном = ПривилегированныйРежим();
                УстановитьБезопасныйРежим(Ложь);
                ПослеБезопасного = ПривилегированныйРежим();
                Возврат Строка(ВБезопасном) + ":" + Строка(ПослеБезопасного);
            КонецФункции

            Функция ПодавлениеБезопасногоСбалансировано() Экспорт
                УстановитьБезопасныйРежим(Истина);
                УстановитьОтключениеБезопасногоРежима(Истина);
                ПриПодавлении = БезопасныйРежим();
                УстановитьОтключениеБезопасногоРежима(Ложь);
                ПослеБалансировки = БезопасныйРежим();
                УстановитьБезопасныйРежим(Ложь);
                Возврат Строка(ПриПодавлении) + ":" + Строка(ПослеБалансировки);
            КонецФункции

            Функция СнятьПодавлениеРодителя()
                Попытка
                    УстановитьОтключениеБезопасногоРежима(Ложь);
                    Возврат "NO_ERROR";
                Исключение
                    Возврат "ERROR";
                КонецПопытки;
            КонецФункции

            Функция ПодавлениеБезопасногоРодителя() Экспорт
                УстановитьБезопасныйРежим(Истина);
                УстановитьОтключениеБезопасногоРежима(Истина);
                Результат = СнятьПодавлениеРодителя() + ":" + Строка(БезопасныйРежим());
                УстановитьОтключениеБезопасногоРежима(Ложь);
                УстановитьБезопасныйРежим(Ложь);
                Возврат Результат;
            КонецФункции

            Функция ПодавлениеСВозвратом()
                УстановитьОтключениеБезопасногоРежима(Истина);
                Возврат БезопасныйРежим();
            КонецФункции

            Функция РодительПослеВозвратаПодавления() Экспорт
                УстановитьБезопасныйРежим(Истина);
                Вложенный = ПодавлениеСВозвратом();
                Родитель = БезопасныйРежим();
                УстановитьБезопасныйРежим(Ложь);
                Возврат Строка(Вложенный) + ":" + Строка(Родитель);
            КонецФункции

            Функция ПривилегированныйПриПодавлении() Экспорт
                УстановитьБезопасныйРежим(Истина);
                УстановитьПривилегированныйРежим(Истина);
                УстановитьОтключениеБезопасногоРежима(Истина);
                ПриПодавлении = Строка(БезопасныйРежим()) + ":"
                    + Строка(ПривилегированныйРежим());
                УстановитьОтключениеБезопасногоРежима(Ложь);
                ПослеБалансировки = ПривилегированныйРежим();
                УстановитьБезопасныйРежим(Ложь);
                УстановитьПривилегированныйРежим(Ложь);
                Возврат ПриПодавлении + ":" + Строка(ПослеБалансировки);
            КонецФункции

            Функция ПрочитатьПодавлениеВложенное()
                Возврат ПолучитьОтключениеБезопасногоРежима();
            КонецФункции

            Функция ПрочитатьПодавление() Экспорт
                УстановитьОтключениеБезопасногоРежима(Истина);
                До = ПолучитьОтключениеБезопасногоРежима();
                Вложенное = ПрочитатьПодавлениеВложенное();
                УстановитьОтключениеБезопасногоРежима(Ложь);
                После = GetSafeModeDisabled();
                Возврат Строка(До) + ":" + Строка(Вложенное) + ":" + Строка(После);
            КонецФункции

            Процедура ИменованныйПрофиль() Экспорт
                SetSafeMode("Профиль");
            КонецПроцедуры
            "#,
        )
        .unwrap();

    assert_eq!(
        session
            .call(&module, "БезопасныйРодитель", vec![])
            .unwrap()
            .0,
        Value::Str("ERROR:Да".into())
    );
    assert_eq!(
        session
            .call(&module, "БезопасныйСчетчик", vec![])
            .unwrap()
            .0,
        Value::Str("Да:Нет".into())
    );
    assert_eq!(
        session
            .call(&module, "ПривилегированныйРодитель", vec![])
            .unwrap()
            .0,
        Value::Str("ERROR:Да".into())
    );
    assert_eq!(
        session
            .call(&module, "ПривилегированныйСчетчик", vec![])
            .unwrap()
            .0,
        Value::Str("Да:Нет".into())
    );
    assert_eq!(
        session
            .call(&module, "ПослеБезопасногоСбоя", vec![])
            .unwrap()
            .0,
        Value::Boolean(false)
    );
    assert_eq!(
        session
            .call(&module, "ПослеБезопасногоВозврата", vec![])
            .unwrap()
            .0,
        Value::Boolean(false)
    );
    assert_eq!(
        session
            .call(&module, "ПослеПривилегированногоСбоя", vec![])
            .unwrap()
            .0,
        Value::Boolean(false)
    );
    assert_eq!(
        session
            .call(&module, "ПослеПривилегированногоВозврата", vec![])
            .unwrap()
            .0,
        Value::Boolean(false)
    );
    assert_eq!(
        session
            .call(&module, "ПривилегированныйВБезопасном", vec![])
            .unwrap()
            .0,
        Value::Str("Нет:Да".into())
    );
    assert_eq!(
        session
            .call(&module, "ПодавлениеБезопасногоСбалансировано", vec![])
            .unwrap()
            .0,
        Value::Str("Нет:Нет".into())
    );
    assert_eq!(
        session
            .call(&module, "ПодавлениеБезопасногоРодителя", vec![])
            .unwrap()
            .0,
        Value::Str("ERROR:Нет".into())
    );
    assert_eq!(
        session
            .call(&module, "РодительПослеВозвратаПодавления", vec![])
            .unwrap()
            .0,
        Value::Str("Нет:Да".into())
    );
    assert_eq!(
        session
            .call(&module, "ПривилегированныйПриПодавлении", vec![])
            .unwrap()
            .0,
        Value::Str("Нет:Да:Да".into())
    );
    assert_eq!(
        session
            .call(&module, "ПрочитатьПодавление", vec![])
            .unwrap()
            .0,
        Value::Str("Да:Нет:Нет".into())
    );
    let error = session
        .call(&module, "ИменованныйПрофиль", vec![])
        .unwrap_err();
    assert!(error.to_string().contains("не поддерживаются"));
}

#[test]
fn data_separation_safe_mode_validates_session_names_and_frame_balance() {
    let factory = SessionFactory::new(
        code_snapshot(START_HANDLER),
        metadata_catalog("configuration"),
        "МодульСеанса",
        SessionKind::ThinClient,
        PLATFORM_VERSION,
    )
    .unwrap();
    let mut session = factory
        .open(
            principal(),
            resources((), 1).with_data_separation_common_attributes(["Разделитель".to_string()]),
        )
        .unwrap();
    let module = session
        .instantiate(
            r#"
            Функция СнятьВложенно()
                Попытка
                    SetDataSeparationSafeMode("Разделитель", Ложь);
                    Возврат "NO_ERROR";
                Исключение
                    Возврат "ERROR";
                КонецПопытки;
            КонецФункции

            Функция Сбалансировать() Экспорт
                УстановитьБезопасныйРежимРазделенияДанных("Разделитель", Истина);
                Вложенный = СнятьВложенно();
                УстановитьБезопасныйРежимРазделенияДанных("разделитель", Ложь);
                Возврат Вложенный;
            КонецФункции

            Процедура ОставитьВКадре()
                УстановитьБезопасныйРежимРазделенияДанных("Разделитель", Истина);
            КонецПроцедуры

            Функция ПроверитьПослеВозврата() Экспорт
                ОставитьВКадре();
                Попытка
                    УстановитьБезопасныйРежимРазделенияДанных("Разделитель", Ложь);
                    Возврат "NO_ERROR";
                Исключение
                    Возврат "ERROR";
                КонецПопытки;
            КонецФункции

            Процедура Неизвестный() Экспорт
                УстановитьБезопасныйРежимРазделенияДанных("Чужой", Истина);
            КонецПроцедуры
            "#,
        )
        .unwrap();

    assert_eq!(
        session.call(&module, "Сбалансировать", vec![]).unwrap().0,
        Value::Str("ERROR".into())
    );
    assert_eq!(
        session
            .call(&module, "ПроверитьПослеВозврата", vec![])
            .unwrap()
            .0,
        Value::Str("ERROR".into())
    );
    assert!(
        session
            .call(&module, "Неизвестный", vec![])
            .unwrap_err()
            .to_string()
            .contains("не найден")
    );

    let mut without_names = factory.open(principal(), resources((), 2)).unwrap();
    let module = without_names
        .instantiate(
            "Процедура Проверить() Экспорт УстановитьБезопасныйРежимРазделенияДанных(\"Разделитель\", Истина); КонецПроцедуры",
        )
        .unwrap();
    let error = without_names
        .call(&module, "Проверить", vec![])
        .unwrap_err();
    assert!(error.to_string().contains("не переданы хостом"));
}

#[test]
fn data_separation_safe_mode_getter_requires_an_authoritative_zone_baseline() {
    let factory = SessionFactory::new(
        code_snapshot(START_HANDLER),
        metadata_catalog("configuration"),
        "МодульСеанса",
        SessionKind::ThinClient,
        PLATFORM_VERSION,
    )
    .unwrap();
    let mut session = factory
        .open(
            principal(),
            resources((), 1).with_data_separation_common_attributes(["Разделитель".to_string()]),
        )
        .unwrap();
    let module = session
        .instantiate(
            "Функция Проверить(Имя) Экспорт Возврат DataSeparationSafeMode(Имя); КонецФункции",
        )
        .unwrap();
    let error = session
        .call(&module, "Проверить", vec![Value::Str("Разделитель".into())])
        .unwrap_err();
    assert!(error.to_string().contains("зоны не передан хостом"));
    let error = session
        .call(&module, "Проверить", vec![Value::Str("Чужой".into())])
        .unwrap_err();
    assert!(error.to_string().contains("не найден"));

    for source in [
        "Возврат БезопасныйРежимРазделенияДанных();",
        "Возврат DataSeparationSafeMode(\"Разделитель\", Истина);",
    ] {
        let error = session.instantiate(source).err().expect("неверная арность");
        assert!(error.to_string().contains("аргумент"), "{error}");
    }

    let module = session
        .instantiate(
            "Процедура ПроверитьПорядок() Экспорт SetDataSeparationSafeMode(\"Чужой\", \"bad\"); КонецПроцедуры",
        )
        .unwrap();
    let error = session
        .call(&module, "ПроверитьПорядок", vec![])
        .unwrap_err();
    assert!(error.to_string().contains("Булево"));
}

#[test]
fn factory_rejects_incompatible_metadata_and_startup_failure() {
    assert!(matches!(
        SessionFactory::new(
            code_snapshot(START_HANDLER),
            metadata_catalog("configuration"),
            "МодульСеанса",
            SessionKind::WebClient,
            "",
        ),
        Err(SessionError::EmptyPlatformVersion)
    ));
    assert!(matches!(
        SessionFactory::new(
            code_snapshot(START_HANDLER),
            metadata_catalog("other"),
            "МодульСеанса",
            SessionKind::WebClient,
            PLATFORM_VERSION,
        ),
        Err(SessionError::ConfigurationIdentityMismatch { .. })
    ));

    let factory = SessionFactory::new(
        code_snapshot(
            "Процедура УстановкаПараметровСеанса(Требуемые) Экспорт ВызватьИсключение \"старт не выполнен\"; КонецПроцедуры",
        ),
        metadata_catalog("configuration"),
        "МодульСеанса",
        SessionKind::WebClient,
        PLATFORM_VERSION,
    )
    .unwrap();
    let error = factory.open(principal(), resources((), 1)).err().unwrap();
    assert!(error.to_string().contains("старт не выполнен"));
}

#[test]
fn every_supported_session_kind_runs_full_startup() {
    let foreground = SessionFactory::new(
        code_snapshot(START_HANDLER),
        metadata_catalog("configuration"),
        "МодульСеанса",
        SessionKind::ThinClient,
        PLATFORM_VERSION,
    )
    .unwrap();
    let parent = foreground.open(principal(), resources((), 1)).unwrap();

    for kind in [SessionKind::WebClient, SessionKind::ThinClient] {
        let factory = SessionFactory::new(
            code_snapshot(START_HANDLER),
            metadata_catalog("configuration"),
            "МодульСеанса",
            kind,
            PLATFORM_VERSION,
        )
        .unwrap();
        let session = factory.open(principal(), resources((), 2)).unwrap();
        assert_eq!(session.kind(), kind);
        assert_eq!(
            session.parameters().peek("Запущен").unwrap(),
            Some(Value::number_from_i64(1))
        );
    }

    let factory = SessionFactory::new(
        code_snapshot(START_HANDLER),
        metadata_catalog("configuration"),
        "МодульСеанса",
        SessionKind::BackgroundJob,
        PLATFORM_VERSION,
    )
    .unwrap();
    let session = factory
        .open_background(
            principal(),
            resources((), 3),
            "Обработка.Запустить",
            &parent,
        )
        .unwrap();
    assert_eq!(session.kind(), SessionKind::BackgroundJob);
    assert_eq!(
        session.parameters().peek("Запущен").unwrap(),
        Some(Value::number_from_i64(1))
    );
}

fn session_run_mode(session: &mut Session) -> Value {
    let module = session
        .instantiate("Функция Режим() Экспорт Возврат CurrentRunMode(); КонецФункции")
        .unwrap();
    module
        .call(&mut session.state, "Режим", Vec::new())
        .unwrap()
        .0
}

#[test]
fn current_run_mode_matches_every_session_kind() {
    for kind in [SessionKind::WebClient, SessionKind::ThinClient] {
        let factory = SessionFactory::new(
            code_snapshot(START_HANDLER),
            metadata_catalog("configuration"),
            "МодульСеанса",
            kind,
            PLATFORM_VERSION,
        )
        .unwrap();
        let mut session = factory.open(principal(), resources((), 1)).unwrap();
        assert_eq!(
            session_run_mode(&mut session),
            Value::Enum(EnumValue::ClientRunModeManagedApplication)
        );
    }

    let foreground = SessionFactory::new(
        code_snapshot(START_HANDLER),
        metadata_catalog("configuration"),
        "МодульСеанса",
        SessionKind::ThinClient,
        PLATFORM_VERSION,
    )
    .unwrap();
    let parent = foreground.open(principal(), resources((), 1)).unwrap();
    let background = SessionFactory::new(
        code_snapshot(START_HANDLER),
        metadata_catalog("configuration"),
        "МодульСеанса",
        SessionKind::BackgroundJob,
        PLATFORM_VERSION,
    )
    .unwrap();
    let mut session = background
        .open_background(principal(), resources((), 2), "Обработка.Режим", &parent)
        .unwrap();
    assert_eq!(session_run_mode(&mut session), Value::Undefined);
}

#[test]
fn client_application_type_belongs_to_the_current_session() {
    let foreground_factory = SessionFactory::new(
        code_snapshot(START_HANDLER),
        metadata_catalog("configuration"),
        "МодульСеанса",
        SessionKind::ThinClient,
        PLATFORM_VERSION,
    )
    .unwrap();
    let mut parent = foreground_factory
        .open(principal(), resources((), 1))
        .unwrap();
    let type_probe = parent
        .instantiate(
            "Функция ТипОбъекта() Экспорт Возврат Строка(ТипЗнч(КлиентскоеПриложение)); КонецФункции",
        )
        .unwrap();
    assert_eq!(
        parent.call(&type_probe, "ТипОбъекта", vec![]).unwrap().0,
        Value::Str("Client application".into())
    );
    assert_eq!(
        client_application_type_value(&mut parent),
        Value::Enum(EnumValue::ClientApplicationTypeThinClient)
    );

    let background_factory = SessionFactory::new(
        code_snapshot(START_HANDLER),
        metadata_catalog("configuration"),
        "МодульСеанса",
        SessionKind::BackgroundJob,
        PLATFORM_VERSION,
    )
    .unwrap();
    let mut background = background_factory
        .open_background(principal(), resources((), 2), "Обработка.Проба", &parent)
        .unwrap();
    assert_eq!(
        client_application_type_value(&mut background),
        Value::Undefined
    );

    let web_factory = SessionFactory::new(
        code_snapshot(START_HANDLER),
        metadata_catalog("configuration"),
        "МодульСеанса",
        SessionKind::WebClient,
        PLATFORM_VERSION,
    )
    .unwrap();
    let mut web = web_factory.open(principal(), resources((), 3)).unwrap();
    assert_eq!(
        client_application_type_value(&mut web),
        Value::Enum(EnumValue::ClientApplicationTypeWebClient)
    );
    assert_eq!(
        client_application_type_value(&mut parent),
        Value::Enum(EnumValue::ClientApplicationTypeThinClient)
    );
}

fn client_application_type_value(session: &mut Session) -> Value {
    let module = session
        .instantiate(
            "Функция ТипПриложения() Экспорт Возврат ClientApplication.ApplicationType(); КонецФункции",
        )
        .unwrap();
    session.call(&module, "ТипПриложения", vec![]).unwrap().0
}

#[test]
fn current_infobase_session_exposes_only_the_current_background_job() {
    let foreground_factory = SessionFactory::new(
        code_snapshot(START_HANDLER),
        metadata_catalog("configuration"),
        "МодульСеанса",
        SessionKind::ThinClient,
        PLATFORM_VERSION,
    )
    .unwrap();
    let mut foreground = foreground_factory
        .open(principal(), resources((), 1))
        .unwrap();
    let foreground_probe = foreground
        .instantiate(
            r#"
            Функция Контекст() Экспорт
                Сеанс = ПолучитьТекущийСеансИнформационнойБазы();
                Возврат Строка(ТипЗнч(Сеанс)) + "|"
                    + Строка(ТипЗнч(Сеанс.ПолучитьФоновоеЗадание()));
            КонецФункции
            "#,
        )
        .unwrap();
    assert_eq!(
        foreground
            .call(&foreground_probe, "Контекст", vec![])
            .unwrap()
            .0,
        Value::Str("Active session|Не определено".into())
    );

    let background_factory = SessionFactory::new(
        code_snapshot(START_HANDLER),
        metadata_catalog("configuration"),
        "МодульСеанса",
        SessionKind::BackgroundJob,
        PLATFORM_VERSION,
    )
    .unwrap();
    let mut background = background_factory
        .open_background(
            principal(),
            resources((), 2),
            "Обработка.ФоновыйМетод",
            &foreground,
        )
        .unwrap();
    let background_probe = background
        .instantiate(
            r#"
            Функция Контекст() Экспорт
                Сеанс = GetCurrentInfoBaseSession();
                Задание = Сеанс.GetBackgroundJob();
                Возврат Строка(ТипЗнч(Сеанс)) + "|"
                    + Строка(ТипЗнч(Задание)) + "|" + Задание.MethodName;
            КонецФункции
            "#,
        )
        .unwrap();
    assert_eq!(
        background
            .call(&background_probe, "Контекст", vec![])
            .unwrap()
            .0,
        Value::Str("Active session|Background job|Обработка.ФоновыйМетод".into())
    );
    let unsupported_probe = background
        .instantiate(
            "Функция Состояние() Экспорт Возврат ПолучитьТекущийСеансИнформационнойБазы().ПолучитьФоновоеЗадание().Состояние; КонецФункции",
        )
        .unwrap();
    assert!(
        background
            .call(&unsupported_probe, "Состояние", vec![])
            .unwrap_err()
            .to_string()
            .contains("Состояние")
    );

    let error = background_factory
        .open_background(principal(), resources((), 3), "", &foreground)
        .err()
        .unwrap();
    assert!(matches!(error, SessionError::EmptyBackgroundMethod));
}

#[test]
fn background_session_copies_parent_parameters_and_then_diverges() {
    let foreground = SessionFactory::new(
        code_snapshot(START_HANDLER),
        metadata_catalog("configuration"),
        "МодульСеанса",
        SessionKind::ThinClient,
        PLATFORM_VERSION,
    )
    .unwrap();
    let background = SessionFactory::new(
        code_snapshot(START_HANDLER),
        metadata_catalog("configuration"),
        "МодульСеанса",
        SessionKind::BackgroundJob,
        PLATFORM_VERSION,
    )
    .unwrap();
    let parent = foreground.open(principal(), resources((), 1)).unwrap();
    parent
        .parameters()
        .set("Копируемый", Value::number_from_i64(7))
        .unwrap();
    let child = background
        .open_background(principal(), resources((), 2), "Обработка.Копия", &parent)
        .unwrap();
    assert_eq!(
        child.parameters().peek("Копируемый").unwrap(),
        Some(Value::number_from_i64(7))
    );
    child
        .parameters()
        .set("Копируемый", Value::number_from_i64(8))
        .unwrap();
    assert_eq!(
        parent.parameters().peek("Копируемый").unwrap(),
        Some(Value::number_from_i64(7))
    );
}

#[test]
fn event_log_uses_the_current_session_writer_and_measured_enums() {
    let factory = SessionFactory::new(
        code_snapshot(START_HANDLER),
        metadata_catalog("configuration"),
        "МодульСеанса",
        SessionKind::ThinClient,
        PLATFORM_VERSION,
    )
    .unwrap();
    let writer = Rc::new(RecordingEventLog::default());
    let mut session = factory
        .open(
            principal(),
            resources((), 1).with_event_log_writer(writer.clone()),
        )
        .unwrap();
    let probe = session
        .instantiate(
            r#"
            Процедура Записать() Экспорт
                ЗаписьЖурналаРегистрации("minimal");
                WriteLogEvent("full", EventLogLevel.Error, , 42, "comment",
                    EventLogEntryTransactionMode.Transactional);
            КонецПроцедуры

            Функция Перечисления() Экспорт
                Возврат Строка(ТипЗнч(УровеньЖурналаРегистрации)) + "|"
                    + Строка(УровеньЖурналаРегистрации.Примечание) + "|"
                    + Строка(УровеньЖурналаРегистрации.Информация) + "|"
                    + Строка(УровеньЖурналаРегистрации.Предупреждение) + "|"
                    + Строка(УровеньЖурналаРегистрации.Ошибка) + "|"
                    + Строка(ТипЗнч(EventLogEntryTransactionMode)) + "|"
                    + Строка(EventLogEntryTransactionMode.Independent) + "|"
                    + Строка(EventLogEntryTransactionMode.Transactional);
            КонецФункции
            "#,
        )
        .unwrap();
    assert_eq!(
        session.call(&probe, "Перечисления", vec![]).unwrap().0,
        Value::Str(
            "EnumEventLogLevel|Note|Information|Warning|Error|EnumEventLogEntryTransactionMode|Independent|Transactional"
                .into()
        )
    );
    session.call(&probe, "Записать", vec![]).unwrap();

    let entries = writer.entries.borrow();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].event_name(), "minimal");
    assert_eq!(entries[0].level(), EventLogLevel::Note);
    assert_eq!(entries[0].metadata(), &Value::Undefined);
    assert_eq!(entries[0].data(), &Value::Undefined);
    assert_eq!(entries[0].comment(), "");
    assert_eq!(
        entries[0].transaction_mode(),
        EventLogTransactionMode::Independent
    );
    assert_eq!(entries[1].event_name(), "full");
    assert_eq!(entries[1].level(), EventLogLevel::Error);
    assert_eq!(entries[1].data(), &Value::number_from_i64(42));
    assert_eq!(entries[1].comment(), "comment");
    assert_eq!(
        entries[1].transaction_mode(),
        EventLogTransactionMode::Transactional
    );
}

#[test]
fn event_log_event_use_operations_delegate_to_the_session_writer() {
    let factory = SessionFactory::new(
        code_snapshot(START_HANDLER),
        metadata_catalog("configuration"),
        "МодульСеанса",
        SessionKind::ThinClient,
        PLATFORM_VERSION,
    )
    .unwrap();
    let writer = Rc::new(RecordingEventLog::default());
    let mut session = factory
        .open(
            principal(),
            resources((), 1).with_event_log_writer(writer.clone()),
        )
        .unwrap();
    let probe = session
        .instantiate(
            r#"
            Функция Обновить() Экспорт
                Настройка = GetEventLogEventUse("event");
                Исходное = Строка(Настройка.Use) + ":" + Строка(Настройка.UseDescription);
                Настройка.Использование = Истина;
                Настройка.ОписаниеИспользования = "changed";
                УстановитьИспользованиеСобытияЖурналаРегистрации("event", Настройка);
                Возврат Исходное;
            КонецФункции
            "#,
        )
        .unwrap();

    assert_eq!(
        session.call(&probe, "Обновить", vec![]).unwrap().0,
        Value::Str("Нет:42".into())
    );
    assert_eq!(&*writer.event_use_reads.borrow(), &["event"]);
    assert_eq!(
        &*writer.event_use_writes.borrow(),
        &[("event".into(), true, Value::Str("changed".into()))]
    );
}

#[test]
fn event_log_using_reads_each_session_adapter() {
    let factory = SessionFactory::new(
        code_snapshot(START_HANDLER),
        metadata_catalog("configuration"),
        "МодульСеанса",
        SessionKind::ThinClient,
        PLATFORM_VERSION,
    )
    .unwrap();
    let mut first = factory
        .open(
            principal(),
            resources((), 1).with_event_log_writer(Rc::new(LevelEventLog(vec![
                EventLogLevel::Information,
                EventLogLevel::Error,
            ]))),
        )
        .unwrap();
    let mut second = factory
        .open(
            principal(),
            resources((), 2)
                .with_event_log_writer(Rc::new(LevelEventLog(vec![EventLogLevel::Warning]))),
        )
        .unwrap();
    let source = r#"
        Функция Проверить() Экспорт
            Уровни = ПолучитьИспользованиеЖурналаРегистрации();
            EnglishLevels = GetEventLogUsing();
            Возврат Строка(Уровни.Количество()) + "|"
                + Строка(Уровни.Получить(0)) + "|"
                + Строка(EnglishLevels.Получить(EnglishLevels.Количество() - 1));
        КонецФункции
    "#;
    let first_probe = first.instantiate(source).unwrap();
    let second_probe = second.instantiate(source).unwrap();
    assert_eq!(
        first.call(&first_probe, "Проверить", vec![]).unwrap().0,
        Value::Str("2|Information|Error".into())
    );
    assert_eq!(
        second.call(&second_probe, "Проверить", vec![]).unwrap().0,
        Value::Str("1|Warning|Warning".into())
    );

    let mut missing = factory.open(principal(), resources((), 3)).unwrap();
    let missing_probe = missing.instantiate(source).unwrap();
    assert!(
        missing
            .call(&missing_probe, "Проверить", vec![])
            .unwrap_err()
            .to_string()
            .contains("журнал регистрации не подключён")
    );
}

#[test]
fn event_log_fails_explicitly_without_a_session_writer() {
    let factory = SessionFactory::new(
        code_snapshot(START_HANDLER),
        metadata_catalog("configuration"),
        "МодульСеанса",
        SessionKind::ThinClient,
        PLATFORM_VERSION,
    )
    .unwrap();
    let mut session = factory.open(principal(), resources((), 1)).unwrap();
    let probe = session
        .instantiate(
            "Процедура Записать() Экспорт ЗаписьЖурналаРегистрации(\"event\"); КонецПроцедуры",
        )
        .unwrap();
    assert!(
        session
            .call(&probe, "Записать", vec![])
            .unwrap_err()
            .to_string()
            .contains("журнал регистрации не подключён")
    );
}

#[test]
fn event_log_export_has_measured_arity_and_fails_explicitly() {
    let factory = SessionFactory::new(
        code_snapshot(START_HANDLER),
        metadata_catalog("configuration"),
        "МодульСеанса",
        SessionKind::ThinClient,
        PLATFORM_VERSION,
    )
    .unwrap();
    let mut session = factory.open(principal(), resources((), 1)).unwrap();
    let guarded = session
        .instantiate(
            "Функция Проверить() Экспорт Попытка ВыгрузитьЖурналРегистрации(Неопределено); Исключение Возврат Истина; КонецПопытки; Возврат Ложь; КонецФункции",
        )
        .unwrap();
    assert_eq!(
        session.call(&guarded, "Проверить", vec![]).unwrap().0,
        Value::Boolean(true)
    );
    let english = session
        .instantiate(
            "Function Check() Export Try UnloadEventLog(Undefined); Except Return True; EndTry; Return False; EndFunction",
        )
        .unwrap();
    assert_eq!(
        session.call(&english, "Check", vec![]).unwrap().0,
        Value::Boolean(true)
    );
    assert!(
        session
            .instantiate(
                "Процедура Проверить() Экспорт ВыгрузитьЖурналРегистрации(); КонецПроцедуры"
            )
            .is_err()
    );
    assert!(
        session
            .instantiate("Процедура Проверить() Экспорт ВыгрузитьЖурналРегистрации(Неопределено, Неопределено, Неопределено, Неопределено, Неопределено, Неопределено); КонецПроцедуры")
            .is_err()
    );
}

#[test]
fn error_processing_uses_the_session_owned_current_user_settings() {
    let factory = SessionFactory::new(
        code_snapshot(START_HANDLER),
        metadata_catalog("configuration"),
        "МодульСеанса",
        SessionKind::ThinClient,
        PLATFORM_VERSION,
    )
    .unwrap();
    let settings = ErrorProcessingSettingsData::new(ErrorReportingMode::Auto, "");
    let mut session = factory
        .open(
            principal(),
            resources((), 1).with_error_processing_settings(settings),
        )
        .unwrap();
    let probe = session
        .instantiate(
            r#"
            Функция Проверить() Экспорт
                Настройки = ErrorProcessing.GetUserSettings();
                Попытка
                    ВызватьИсключение "Пробная ошибка";
                Исключение
                    Ошибка = ИнформацияОбОшибке();
                    Кратко = ОбработкаОшибок.КраткоеПредставлениеОшибки(Ошибка)
                        = Ошибка.Описание;
                    Подробно = ErrorProcessing.DetailErrorDescription(Ошибка)
                        = ПодробноеПредставлениеОшибки(Ошибка);
                КонецПопытки;
                Возврат Строка(ТипЗнч(Настройки)) + ":"
                    + Строка(Настройки.ОтправлятьОтчетНаКлиенте) + ":"
                    + Настройки.ErrorProcessingServiceAddress + ":"
                    + Строка(Кратко) + ":" + Строка(Подробно);
            КонецФункции
            "#,
        )
        .unwrap();
    assert_eq!(
        session.call(&probe, "Проверить", vec![]).unwrap().0,
        Value::Str("Settings of error processing on the client:Auto::Да:Да".into())
    );

    let mut missing = factory.open(principal(), resources((), 2)).unwrap();
    let probe = missing
        .instantiate(
            "Функция Проверить() Экспорт Возврат ОбработкаОшибок.ПолучитьНастройкиПользователя(); КонецФункции",
        )
        .unwrap();
    assert!(
        missing
            .call(&probe, "Проверить", vec![])
            .unwrap_err()
            .to_string()
            .contains("не переданы хостом")
    );
}

#[derive(Debug, Default)]
struct RecordingClientSettings {
    loads: RefCell<Vec<RecordedSettingsLoad>>,
    saves: RefCell<Vec<RecordedSettingsSave>>,
    selections: RefCell<Vec<(String, ClientSettingsStorage, bool)>>,
    lists: RefCell<Vec<RecordedSettingsList>>,
    descriptions: RefCell<Vec<RecordedSettingsDescription>>,
    deletions: RefCell<Vec<RecordedSettingsDelete>>,
    forms: RefCell<HomePageFormsData>,
}

#[derive(Debug)]
struct RecordedSettingsLoad {
    principal: String,
    storage: ClientSettingsStorage,
    object_key: String,
    settings_key: String,
    user_name: Option<String>,
}

#[derive(Debug)]
struct RecordedSettingsSave {
    principal: String,
    storage: ClientSettingsStorage,
    object_key: String,
    settings_key: String,
    settings: Value,
}

#[derive(Debug)]
struct RecordedSettingsList {
    principal: String,
    storage: ClientSettingsStorage,
    object_key: String,
    user_name: Option<String>,
}

#[derive(Debug)]
struct RecordedSettingsDescription {
    principal: String,
    storage: ClientSettingsStorage,
    object_key: String,
    settings_key: String,
    user_name: Option<String>,
}

#[derive(Debug)]
struct RecordedSettingsDelete {
    principal: String,
    storage: ClientSettingsStorage,
    object_key: Option<String>,
    settings_key: Option<String>,
    user_name: Option<String>,
}

impl ClientSettingsAdapter for RecordingClientSettings {
    fn load(
        &self,
        principal: &Principal,
        request: &ClientSettingsLoadRequest,
    ) -> RtResult<Option<Value>> {
        self.loads.borrow_mut().push(RecordedSettingsLoad {
            principal: principal.name().to_string(),
            storage: request.storage(),
            object_key: request.object_key().to_string(),
            settings_key: request.settings_key().to_string(),
            user_name: request.user_name().map(str::to_string),
        });
        Ok((request.object_key() == "present").then(|| Value::number_from_i64(41)))
    }

    fn save(&self, principal: &Principal, request: &ClientSettingsSaveRequest) -> RtResult<()> {
        self.saves.borrow_mut().push(RecordedSettingsSave {
            principal: principal.name().to_string(),
            storage: request.storage(),
            object_key: request.object_key().to_string(),
            settings_key: request.settings_key().to_string(),
            settings: request.settings().clone(),
        });
        Ok(())
    }

    fn select(
        &self,
        principal: &Principal,
        request: &ClientSettingsSelectRequest,
    ) -> RtResult<Vec<ClientSettingsEntry>> {
        self.selections
            .borrow_mut()
            .push((
                principal.name().to_string(),
                request.storage(),
                matches!(request.filter(), Value::Object(object) if matches!(object.as_ref(), BslObject::Structure(_))),
            ));
        Ok(vec![ClientSettingsEntry::new(
            "object",
            "settings",
            principal.name(),
            Value::number_from_i64(42),
        )])
    }

    fn list(
        &self,
        principal: &Principal,
        request: &ClientSettingsListRequest,
    ) -> RtResult<Vec<String>> {
        self.lists.borrow_mut().push(RecordedSettingsList {
            principal: principal.name().to_string(),
            storage: request.storage(),
            object_key: request.object_key().to_string(),
            user_name: request.user_name().map(str::to_string),
        });
        Ok(vec!["first".to_string(), "second".to_string()])
    }

    fn description(
        &self,
        principal: &Principal,
        request: &ClientSettingsDescriptionRequest,
    ) -> RtResult<ClientSettingsDescription> {
        self.descriptions
            .borrow_mut()
            .push(RecordedSettingsDescription {
                principal: principal.name().to_string(),
                storage: request.storage(),
                object_key: request.object_key().to_string(),
                settings_key: request.settings_key().to_string(),
                user_name: request.user_name().map(str::to_string),
            });
        Ok(ClientSettingsDescription::new(
            Value::Str("object".into()),
            Value::Str("settings".into()),
            Value::Str(principal.name().into()),
            Value::Str("caption".into()),
            Value::Undefined,
        ))
    }

    fn delete(&self, principal: &Principal, request: &ClientSettingsDeleteRequest) -> RtResult<()> {
        self.deletions.borrow_mut().push(RecordedSettingsDelete {
            principal: principal.name().to_string(),
            storage: request.storage(),
            object_key: request.object_key().map(str::to_string),
            settings_key: request.settings_key().map(str::to_string),
            user_name: request.user_name().map(str::to_string),
        });
        Ok(())
    }

    fn home_page_forms(&self, _: &Principal) -> RtResult<HomePageFormsData> {
        Ok(self.forms.borrow().clone())
    }

    fn set_home_page_forms(&self, _: &Principal, forms: &HomePageFormsData) -> RtResult<()> {
        *self.forms.borrow_mut() = forms.clone();
        Ok(())
    }
}

#[test]
fn special_settings_storages_use_the_adapter_of_the_current_session() {
    let factory = SessionFactory::new(
        code_snapshot(START_HANDLER),
        metadata_catalog("configuration"),
        "МодульСеанса",
        SessionKind::ThinClient,
        PLATFORM_VERSION,
    )
    .unwrap();
    let adapter = Rc::new(RecordingClientSettings::default());
    let mut session = factory
        .open(
            principal(),
            resources((), 1).with_client_settings_adapter(adapter.clone()),
        )
        .unwrap();
    let probe = session
        .instantiate(
            r#"
            Функция ПроверитьСписок() Экспорт
                Список = ReportsUserSettingsStorage.GetList("report", "Другой");
                Возврат Строка(ТипЗнч(Список)) + "|" + Список[0].Value
                    + "|" + Список[1].Значение;
            КонецФункции

            Функция ПроверитьОписание() Экспорт
                Описание = ХранилищеПользовательскихНастроекДинамическихСписков
                    .ПолучитьОписание("list", "variant", "Другой");
                Возврат Описание.ObjectKey + "|" + Описание.КлючНастроек
                    + "|" + Описание.User + "|" + Описание.Представление;
            КонецФункции

            Процедура УдалитьВсе() Экспорт
                GetStandardReportsUserSettingsStorage().Delete(, , );
            КонецПроцедуры
            "#,
        )
        .unwrap();

    assert_eq!(
        session.call(&probe, "ПроверитьСписок", vec![]).unwrap().0,
        Value::Str("Value list|first|second".into())
    );
    assert_eq!(
        session.call(&probe, "ПроверитьОписание", vec![]).unwrap().0,
        Value::Str("object|settings|Тест|caption".into())
    );
    session.call(&probe, "УдалитьВсе", vec![]).unwrap();

    let lists = adapter.lists.borrow();
    assert_eq!(lists.len(), 1);
    assert_eq!(lists[0].principal, "Тест");
    assert_eq!(lists[0].storage, ClientSettingsStorage::ReportsUser);
    assert_eq!(lists[0].object_key, "report");
    assert_eq!(lists[0].user_name.as_deref(), Some("Другой"));

    let descriptions = adapter.descriptions.borrow();
    assert_eq!(descriptions.len(), 1);
    assert_eq!(descriptions[0].principal, "Тест");
    assert_eq!(
        descriptions[0].storage,
        ClientSettingsStorage::DynamicListsUser
    );
    assert_eq!(descriptions[0].object_key, "list");
    assert_eq!(descriptions[0].settings_key, "variant");
    assert_eq!(descriptions[0].user_name.as_deref(), Some("Другой"));

    let deletions = adapter.deletions.borrow();
    assert_eq!(deletions.len(), 1);
    assert_eq!(deletions[0].principal, "Тест");
    assert_eq!(deletions[0].storage, ClientSettingsStorage::ReportsUser);
    assert_eq!(deletions[0].object_key, None);
    assert_eq!(deletions[0].settings_key, None);
    assert_eq!(deletions[0].user_name, None);
}

#[test]
fn report_variants_storage_and_user_history_fail_without_configured_services() {
    let factory = SessionFactory::new(
        code_snapshot(START_HANDLER),
        metadata_catalog("configuration"),
        "МодульСеанса",
        SessionKind::ThinClient,
        PLATFORM_VERSION,
    )
    .unwrap();
    let mut session = factory.open(principal(), resources((), 1)).unwrap();
    let probe = session
        .instantiate(
            r#"
            Функция Список() Экспорт
                Возврат ХранилищеВариантовОтчетов.ПолучитьСписок("Отчет.Продажи");
            КонецФункции
            Функция История() Экспорт
                Возврат UserWorkHistory;
            КонецФункции
            "#,
        )
        .unwrap();

    let error = session.call(&probe, "Список", vec![]).unwrap_err();
    assert!(error.to_string().contains("хранилище вариантов отчетов"));

    let error = session.call(&probe, "История", vec![]).unwrap_err();
    assert!(error.to_string().contains("история работы пользователя"));
}

#[test]
fn client_settings_use_the_current_principal_and_distinguish_stores() {
    let factory = SessionFactory::new(
        code_snapshot(START_HANDLER),
        metadata_catalog("configuration"),
        "МодульСеанса",
        SessionKind::ThinClient,
        PLATFORM_VERSION,
    )
    .unwrap();
    let adapter = Rc::new(RecordingClientSettings::default());
    let mut session = factory
        .open(
            principal(),
            resources((), 1).with_client_settings_adapter(adapter.clone()),
        )
        .unwrap();
    let probe = session
        .instantiate(
            r#"
            Функция ЗагрузитьПрисутствующую() Экспорт
                Возврат commonsettingsstorage.LOAD("present");
            КонецФункции

            Функция ЗагрузитьОтсутствующую() Экспорт
                Возврат ХранилищеОбщихНастроек.Загрузить("missing");
            КонецФункции

            Функция ЗагрузитьДляПользователя() Экспорт
                Возврат ХранилищеОбщихНастроек.Загрузить(
                    "present", "key", , "Другой");
            КонецФункции

            Процедура СохранитьНастройку() Экспорт
                ХранилищеСистемныхНастроек.Сохранить("object", , 7);
            КонецПроцедуры

            Функция ЗагрузитьНастройкиФормы() Экспорт
                Возврат formdatasettingsstorage.LOAD("present");
            КонецФункции
            "#,
        )
        .unwrap();

    assert_eq!(
        session
            .call(&probe, "ЗагрузитьПрисутствующую", vec![])
            .unwrap()
            .0,
        Value::number_from_i64(41)
    );
    assert_eq!(
        session
            .call(&probe, "ЗагрузитьОтсутствующую", vec![])
            .unwrap()
            .0,
        Value::Undefined
    );
    assert_eq!(
        session
            .call(&probe, "ЗагрузитьДляПользователя", vec![])
            .unwrap()
            .0,
        Value::number_from_i64(41)
    );
    session.call(&probe, "СохранитьНастройку", vec![]).unwrap();
    assert_eq!(
        session
            .call(&probe, "ЗагрузитьНастройкиФормы", vec![])
            .unwrap()
            .0,
        Value::number_from_i64(41)
    );

    let loads = adapter.loads.borrow();
    assert_eq!(loads.len(), 4);
    assert_eq!(loads[0].principal, "Тест");
    assert_eq!(loads[0].storage, ClientSettingsStorage::Common);
    assert_eq!(loads[0].object_key, "present");
    assert_eq!(loads[0].settings_key, "");
    assert_eq!(loads[0].user_name, None);
    assert_eq!(loads[2].settings_key, "key");
    assert_eq!(loads[2].user_name.as_deref(), Some("Другой"));
    assert_eq!(loads[3].principal, "Тест");
    assert_eq!(loads[3].storage, ClientSettingsStorage::FormData);
    let saves = adapter.saves.borrow();
    assert_eq!(saves.len(), 1);
    assert_eq!(saves[0].principal, "Тест");
    assert_eq!(saves[0].storage, ClientSettingsStorage::System);
    assert_eq!(saves[0].object_key, "object");
    assert_eq!(saves[0].settings_key, "");
    assert_eq!(saves[0].settings, Value::number_from_i64(7));
}

#[test]
fn client_settings_selection_matches_the_measured_cursor() {
    let factory = SessionFactory::new(
        code_snapshot(START_HANDLER),
        metadata_catalog("configuration"),
        "МодульСеанса",
        SessionKind::WebClient,
        PLATFORM_VERSION,
    )
    .unwrap();
    let adapter = Rc::new(RecordingClientSettings::default());
    let mut session = factory
        .open(
            principal(),
            resources((), 1).with_client_settings_adapter(adapter.clone()),
        )
        .unwrap();
    let probe = session
        .instantiate(
            r#"
            Функция Пройти() Экспорт
                Отбор = Новый Структура("КлючОбъекта", "object");
                Выборка = ХранилищеСистемныхНастроек.Выбрать(Отбор);
                ОшибкаДо = Ложь;
                Попытка
                    Значение = Выборка.КлючОбъекта;
                Исключение
                    ОшибкаДо = Истина;
                КонецПопытки;
                Первый = Выборка.Следующий();
                СтрокаДанных = Выборка.ObjectKey + "|" + Выборка.SettingsKey
                    + "|" + Выборка.User + "|" + Строка(Выборка.Settings);
                Конец = Выборка.Next();
                ОшибкаПосле = Ложь;
                Попытка
                    Значение = Выборка.КлючОбъекта;
                Исключение
                    ОшибкаПосле = Истина;
                КонецПопытки;
                Возврат Строка(ОшибкаДо) + "|" + Строка(Первый) + "|"
                    + СтрокаДанных + "|" + Строка(Конец) + "|" + Строка(ОшибкаПосле);
            КонецФункции

            Функция Типы() Экспорт
                Возврат Строка(ТипЗнч(ХранилищеСистемныхНастроек)) + "|"
                    + Строка(ТипЗнч(ХранилищеСистемныхНастроек.Выбрать())) + "|"
                    + Строка(ТипЗнч(Новый НастройкиНачальнойСтраницы)) + "|"
                    + Строка(ТипЗнч(Новый СоставФормНачальнойСтраницы));
            КонецФункции
            "#,
        )
        .unwrap();

    assert_eq!(
        session.call(&probe, "Пройти", vec![]).unwrap().0,
        Value::Str("Да|Да|object|settings|Тест|42|Нет|Да".into())
    );
    assert_eq!(
        session.call(&probe, "Типы", vec![]).unwrap().0,
        Value::Str(
            "Standard settings storage|Standard settings storage Selected data|Home page settings|Home page forms"
                .into()
        )
    );
    assert_eq!(
        adapter.selections.borrow().as_slice(),
        &[
            ("Тест".to_string(), ClientSettingsStorage::System, true,),
            ("Тест".to_string(), ClientSettingsStorage::System, false,),
        ]
    );
}

#[test]
fn home_page_forms_round_trip_through_the_session_adapter() {
    let factory = SessionFactory::new(
        code_snapshot(START_HANDLER),
        metadata_catalog("configuration"),
        "МодульСеанса",
        SessionKind::ThinClient,
        PLATFORM_VERSION,
    )
    .unwrap();
    let adapter = Rc::new(RecordingClientSettings::default());
    let mut session = factory
        .open(
            principal(),
            resources((), 1).with_client_settings_adapter(adapter.clone()),
        )
        .unwrap();
    let probe = session
        .instantiate(
            r#"
            Процедура Записать() Экспорт
                Состав = Новый СоставФормНачальнойСтраницы;
                Состав.ЛеваяКолонка.Добавить("ОбщаяФорма.Левая");
                Состав.RightColumn.Add("ОбщаяФорма.Правая");
                Настройки = Новый HomePageSettings;
                Настройки.SetForms(Состав);
            КонецПроцедуры

            Функция Прочитать() Экспорт
                Состав = Новый НастройкиНачальнойСтраницы;
                Формы = Состав.ПолучитьСоставФорм();
                Возврат Формы.LeftColumn[0] + "|" + Формы.ПраваяКолонка[0];
            КонецФункции
            "#,
        )
        .unwrap();

    session.call(&probe, "Записать", vec![]).unwrap();
    assert_eq!(
        adapter.forms.borrow().left(),
        &[Arc::<str>::from("ОбщаяФорма.Левая")]
    );
    assert_eq!(
        adapter.forms.borrow().right(),
        &[Arc::<str>::from("ОбщаяФорма.Правая")]
    );
    assert_eq!(
        session.call(&probe, "Прочитать", vec![]).unwrap().0,
        Value::Str("ОбщаяФорма.Левая|ОбщаяФорма.Правая".into())
    );
}

#[test]
fn client_settings_are_not_implicitly_shared_between_sessions() {
    let factory = SessionFactory::new(
        code_snapshot(START_HANDLER),
        metadata_catalog("configuration"),
        "МодульСеанса",
        SessionKind::ThinClient,
        PLATFORM_VERSION,
    )
    .unwrap();
    let adapter = Rc::new(RecordingClientSettings::default());
    let first = factory
        .open(
            principal(),
            resources((), 1).with_client_settings_adapter(adapter),
        )
        .unwrap();
    let mut second = factory.open(principal(), resources((), 2)).unwrap();
    let probe = second
        .instantiate(
            "Функция Прочитать() Экспорт Возврат ХранилищеОбщихНастроек.Загрузить(\"present\"); КонецФункции",
        )
        .unwrap();
    let error = second.call(&probe, "Прочитать", vec![]).unwrap_err();
    assert!(error.to_string().contains("провайдер клиентских настроек"));
    drop(first);
}

#[test]
fn background_session_does_not_inherit_the_parent_client_settings_adapter() {
    let foreground_factory = SessionFactory::new(
        code_snapshot(START_HANDLER),
        metadata_catalog("configuration"),
        "МодульСеанса",
        SessionKind::ThinClient,
        PLATFORM_VERSION,
    )
    .unwrap();
    let background_factory = SessionFactory::new(
        code_snapshot(START_HANDLER),
        metadata_catalog("configuration"),
        "МодульСеанса",
        SessionKind::BackgroundJob,
        PLATFORM_VERSION,
    )
    .unwrap();
    let parent = foreground_factory
        .open(
            principal(),
            resources((), 1)
                .with_client_settings_adapter(Rc::new(RecordingClientSettings::default())),
        )
        .unwrap();
    let mut child = background_factory
        .open_background(
            principal(),
            resources((), 2),
            "Обработка.ФоновыйМетод",
            &parent,
        )
        .unwrap();
    let probe = child
        .instantiate(
            "Функция Прочитать() Экспорт Возврат ХранилищеОбщихНастроек.Загрузить(\"present\"); КонецФункции",
        )
        .unwrap();

    let error = child.call(&probe, "Прочитать", vec![]).unwrap_err();
    assert!(error.to_string().contains("провайдер клиентских настроек"));
}

#[test]
fn invalid_home_page_forms_do_not_reach_the_adapter() {
    let factory = SessionFactory::new(
        code_snapshot(START_HANDLER),
        metadata_catalog("configuration"),
        "МодульСеанса",
        SessionKind::ThinClient,
        PLATFORM_VERSION,
    )
    .unwrap();
    let adapter = Rc::new(RecordingClientSettings::default());
    let mut session = factory
        .open(
            principal(),
            resources((), 1).with_client_settings_adapter(adapter.clone()),
        )
        .unwrap();
    let probe = session
        .instantiate(
            "Процедура Записать() Экспорт (Новый НастройкиНачальнойСтраницы).УстановитьСоставФорм(42); КонецПроцедуры",
        )
        .unwrap();
    assert!(session.call(&probe, "Записать", vec![]).is_err());
    assert_eq!(*adapter.forms.borrow(), HomePageFormsData::default());
}

#[test]
fn private_startup_handler_reenters_for_each_supported_session_kind() {
    for kind in [
        SessionKind::WebClient,
        SessionKind::ThinClient,
        SessionKind::BackgroundJob,
    ] {
        let factory = SessionFactory::new(
            code_snapshot(
                "Перем Вызовы; Вызовы = 0; Процедура УстановкаПараметровСеанса(Требуемые) Вызовы = Вызовы + 1; Если Требуемые = Неопределено Тогда ПараметрыСеанса.Запущен = ПараметрыСеанса.Копируемый; Иначе ПараметрыСеанса.Копируемый = Вызовы; КонецЕсли; КонецПроцедуры",
            ),
            metadata_catalog("configuration"),
            "МодульСеанса",
            kind,
            PLATFORM_VERSION,
        )
        .unwrap();
        let session = factory.open(principal(), resources((), 1)).unwrap();
        assert_eq!(session.kind(), kind);
        for name in ["Запущен", "Копируемый"] {
            assert_eq!(
                session.parameters().peek(name).unwrap(),
                Some(Value::number_from_i64(2)),
                "{kind:?}: {name}"
            );
        }
    }
}
