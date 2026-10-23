use bsl_config::{ConfigurationCodeSnapshot, ConfigurationModule, ModuleExecution, ModuleKind};
use bsl_orm::{
    CompatibilityMode, MetadataCatalog, MetadataKind, MetadataObject, Principal, ScriptVariant,
};
use bsl_session::{SessionError, SessionFactory, SessionKind, SessionResources};
use open_bsl::{Error, RtError, Value};
use std::sync::Arc;

fn factory(manager_source: &str) -> SessionFactory {
    factory_with_variant(manager_source, ScriptVariant::Russian)
}

fn factory_with_variant(manager_source: &str, variant: ScriptVariant) -> SessionFactory {
    let manager = "ХранилищеНастроек.ЗамерХранилища.МодульМенеджера";
    let object = MetadataObject::new("probe", "ЗамерХранилища", MetadataKind::SettingsStorage)
        .with_manager_module(manager);
    // Экспорт оставляет менеджер в статическом графе пустого сеансового модуля.
    // Измеренное тело и обработчик при этом не изменяются.
    let source = format!(
        "Функция ЗагрузитьВКонтексте() Экспорт Возврат Загрузить(\"probe\"); КонецФункции\n{manager_source}"
    );
    let code = ConfigurationCodeSnapshot::new(
        "settings-error-phases",
        vec![
            ConfigurationModule::new(
                "session",
                "МодульСеанса",
                "Процедура УстановкаПараметровСеанса(Имена) КонецПроцедуры Функция СвязатьПробу() Экспорт Возврат ХранилищаНастроек.ЗамерХранилища.ЗагрузитьВКонтексте(); КонецФункции",
                ModuleKind::Session,
                ModuleExecution::new().with_server(true),
            ),
            ConfigurationModule::new(
                "manager",
                manager,
                source,
                ModuleKind::Manager,
                ModuleExecution::new().with_server(true),
            )
            .with_owner("probe"),
        ],
    )
    .unwrap();
    SessionFactory::new(
        code,
        Arc::new(
            MetadataCatalog::new("settings-error-phases", vec![object])
                .unwrap()
                .with_configuration(
                    "Проба",
                    "1",
                    variant,
                    CompatibilityMode::DontUse,
                    Vec::new(),
                )
                .unwrap(),
        ),
        "МодульСеанса",
        SessionKind::ThinClient,
        "8.3.27.2342",
    )
    .unwrap()
}

const CAUGHT_LOAD: &str = r#"
Попытка
    ХранилищаНастроек.ЗамерХранилища.Загрузить("probe");
Исключение
    Возврат "caught";
КонецПопытки;
Возврат "missed";
"#;

#[test]
fn arithmetic_error_inside_load_handler_is_critical_and_preserves_its_cause() {
    let factory = factory(include_str!(
        "../../../docs/research/fixtures/settings-storage-load/manager-division-error.bsl"
    ));
    let mut session = factory
        .open(
            Principal::new("user", "user", vec![]),
            SessionResources::new(Box::new(()), 1),
        )
        .unwrap();
    let error = session.execute(CAUGHT_LOAD).unwrap_err();
    let SessionError::OpenBsl(Error::Runtime(RtError::CriticalConfiguration { operation, cause })) =
        error
    else {
        panic!("ожидалась критическая ошибка: {error:?}");
    };
    assert_eq!(operation, "ОбработкаЗагрузки");
    assert!(matches!(*cause, RtError::Num(_)), "{cause:?}");
    assert!(session.execute("Возврат 42;").is_err());
}

fn normal_load(factory: &SessionFactory) -> Value {
    let mut session = factory
        .open(
            Principal::new("user", "user", vec![]),
            SessionResources::new(Box::new(()), 1),
        )
        .unwrap();
    let module = session.instantiate(r#"Функция Прочитать() Экспорт Возврат SettingsStorages.ЗамерХранилища.Load("probe"); КонецФункции"#).unwrap();
    session.call(&module, "Прочитать", vec![]).unwrap().0
}

#[test]
fn english_handler_and_its_letter_case_match_the_native_inner_catch_control() {
    let source = include_str!(
        "../../../docs/research/fixtures/approved-sdk-production-integration-1016/settings-storage-runtime-error-classes-1017/manager-english-handler.bsl"
    );
    for handler in ["LoadProcessing", "lOaDpRoCeSsInG"] {
        let source = source.replace("LoadProcessing", handler);
        assert_eq!(
            normal_load(&factory(&source)),
            Value::Str("caught-inside".into())
        );
    }
}

#[test]
fn russian_handler_wins_for_both_script_variants_and_both_definition_orders() {
    let profiles = [
        (
            ScriptVariant::Russian,
            include_str!(
                "../../../docs/research/fixtures/approved-sdk-production-integration-1016/settings-storage-runtime-error-classes-1017/dual-handlers-russian/manager.bsl"
            ),
        ),
        (
            ScriptVariant::Russian,
            include_str!(
                "../../../docs/research/fixtures/approved-sdk-production-integration-1016/settings-storage-runtime-error-classes-1017/dual-handlers-russian-reverse/manager.bsl"
            ),
        ),
        (
            ScriptVariant::English,
            include_str!(
                "../../../docs/research/fixtures/approved-sdk-production-integration-1016/settings-storage-runtime-error-classes-1017/dual-handlers-english/manager.bsl"
            ),
        ),
        (
            ScriptVariant::English,
            include_str!(
                "../../../docs/research/fixtures/approved-sdk-production-integration-1016/settings-storage-runtime-error-classes-1017/dual-handlers-english-reverse/manager.bsl"
            ),
        ),
    ];
    for (variant, source) in profiles {
        assert_eq!(
            normal_load(&factory_with_variant(source, variant)),
            Value::Str("russian".into())
        );
    }
}

#[test]
fn english_handler_failure_retains_the_critical_policy_and_original_cause() {
    let source = r#"Процедура LoadProcessing(КлючОбъекта, КлючНастроек, Настройки, ОписаниеНастроек, ИмяПользователяИБ)
        ВызватьИсключение "english-handler-probe";
    КонецПроцедуры"#;
    let factory = factory(source);
    let mut session = factory
        .open(
            Principal::new("user", "user", vec![]),
            SessionResources::new(Box::new(()), 1),
        )
        .unwrap();
    let error = session.execute(CAUGHT_LOAD).unwrap_err();
    let SessionError::OpenBsl(Error::Runtime(RtError::CriticalConfiguration { operation, cause })) =
        error
    else {
        panic!("ожидался критический отказ английского обработчика: {error:?}");
    };
    assert_eq!(operation, "ОбработкаЗагрузки");
    assert!(
        matches!(*cause, RtError::Raised(Value::Str(ref text)) if text.to_string() == "english-handler-probe")
    );
    assert!(session.execute("Возврат 42;").is_err());
}

#[test]
fn absent_handler_preserves_the_original_diagnostic_without_critical_promotion() {
    let factory = factory("Процедура Другая() КонецПроцедуры");
    let mut session = factory
        .open(
            Principal::new("user", "user", vec![]),
            SessionResources::new(Box::new(()), 1),
        )
        .unwrap();
    let error = session
        .execute(r#"ХранилищаНастроек.ЗамерХранилища.Загрузить("probe");"#)
        .unwrap_err();
    let expected = "Процедура или функция «ОбработкаЗагрузки» в модуле «ХранилищеНастроек.ЗамерХранилища.МодульМенеджера» не найдена";
    assert!(
        matches!(error, SessionError::OpenBsl(Error::Runtime(RtError::DynamicError(ref text))) if text == expected),
        "{error:?}"
    );
    session.execute("Возврат 42;").unwrap();
}

#[path = "settings_storage_error_phases/property_error.rs"]
mod property_error;

#[path = "settings_storage_error_phases/conversion_error.rs"]
mod conversion_error;

#[path = "settings_storage_error_phases/dynamic_compile_error.rs"]
mod dynamic_compile_error;

#[path = "settings_storage_error_phases/index_error.rs"]
mod index_error;
