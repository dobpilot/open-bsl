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

const LOAD: &str = r#"ХранилищаНастроек.ЗамерХранилища.Загрузить("probe");"#;

/// По справке `Загрузить` вызывает `ОбработкаЗагрузки`, поэтому ошибка
/// обработчика выходит из `Загрузить` ловимым исключением с исходной
/// причиной, а сеанс остаётся рабочим (сознательное отклонение от 8.3.27,
/// где платформа требует перезапуска).
pub(super) fn assert_handler_error_is_catchable(
    factory: &SessionFactory,
    cause: impl Fn(&RtError) -> bool,
) {
    let mut session = factory
        .open(
            Principal::new("user", "user", vec![]),
            SessionResources::new(Box::new(()), 1),
        )
        .unwrap();
    let probe = session
        .instantiate(&format!(
            "Функция Проба() Экспорт {CAUGHT_LOAD} КонецФункции"
        ))
        .unwrap();
    assert_eq!(
        session.call(&probe, "Проба", vec![]).unwrap().0,
        Value::Str("caught".into())
    );
    let error = session.execute(LOAD).unwrap_err();
    let SessionError::OpenBsl(Error::Runtime(error)) = error else {
        panic!("ожидалась ошибка исполнения: {error:?}");
    };
    assert!(cause(&error), "{error:?}");
    session.execute("Возврат 42;").unwrap();
}

#[test]
fn arithmetic_error_inside_load_handler_is_catchable_with_its_cause() {
    let factory = factory(include_str!(
        "../../../../docs/research/fixtures/settings-storage-load/manager-division-error.bsl"
    ));
    assert_handler_error_is_catchable(&factory, |error| matches!(error, RtError::Num(_)));
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
        "../../../../docs/research/fixtures/approved-sdk-production-integration-1016/settings-storage-runtime-error-classes-1017/manager-english-handler.bsl"
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
                "../../../../docs/research/fixtures/approved-sdk-production-integration-1016/settings-storage-runtime-error-classes-1017/dual-handlers-russian/manager.bsl"
            ),
        ),
        (
            ScriptVariant::Russian,
            include_str!(
                "../../../../docs/research/fixtures/approved-sdk-production-integration-1016/settings-storage-runtime-error-classes-1017/dual-handlers-russian-reverse/manager.bsl"
            ),
        ),
        (
            ScriptVariant::English,
            include_str!(
                "../../../../docs/research/fixtures/approved-sdk-production-integration-1016/settings-storage-runtime-error-classes-1017/dual-handlers-english/manager.bsl"
            ),
        ),
        (
            ScriptVariant::English,
            include_str!(
                "../../../../docs/research/fixtures/approved-sdk-production-integration-1016/settings-storage-runtime-error-classes-1017/dual-handlers-english-reverse/manager.bsl"
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
fn english_handler_failure_is_catchable_with_its_original_cause() {
    let source = r#"Процедура LoadProcessing(КлючОбъекта, КлючНастроек, Настройки, ОписаниеНастроек, ИмяПользователяИБ)
        ВызватьИсключение "english-handler-probe";
    КонецПроцедуры"#;
    assert_handler_error_is_catchable(
        &factory(source),
        |error| matches!(error, RtError::Raised(Value::Str(text)) if text.to_string() == "english-handler-probe"),
    );
}

#[test]
fn absent_handler_preserves_the_original_diagnostic() {
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
