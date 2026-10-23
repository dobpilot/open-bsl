use bsl_config::{ConfigurationCodeSnapshot, ConfigurationModule, ModuleExecution, ModuleKind};
use bsl_orm::{MetadataCatalog, MetadataKind, MetadataObject, Principal};
use bsl_session::{SessionError, SessionFactory, SessionKind, SessionResources};
use open_bsl::{Error, RtError, Value};
use std::sync::Arc;

fn factory() -> SessionFactory {
    factory_with_session(
        "Процедура УстановкаПараметровСеанса(Имена) КонецПроцедуры Функция СвязатьПробу() Экспорт Возврат ХранилищаНастроек.ЗамерХранилища.ЗагрузитьВКонтексте(\"probe\"); КонецФункции",
    )
}

fn factory_with_session(session_source: &str) -> SessionFactory {
    let manager = "ХранилищеНастроек.ЗамерХранилища.МодульМенеджера";
    let object = MetadataObject::new("probe", "ЗамерХранилища", MetadataKind::SettingsStorage)
        .with_manager_module(manager);
    let code = ConfigurationCodeSnapshot::new(
        "critical-settings-load",
        vec![
            ConfigurationModule::new(
                "session",
                "МодульСеанса",
                session_source,
                ModuleKind::Session,
                ModuleExecution::new().with_server(true),
            ),
            ConfigurationModule::new(
                "manager",
                manager,
                include_str!("../../../docs/research/fixtures/settings-storage-load/manager-handler-error.bsl"),
                ModuleKind::Manager,
                ModuleExecution::new().with_server(true),
            )
            .with_owner("probe"),
        ],
    )
    .unwrap();
    SessionFactory::new(
        code,
        Arc::new(MetadataCatalog::new("critical-settings-load", vec![object]).unwrap()),
        "МодульСеанса",
        SessionKind::ThinClient,
        "8.3.27.2342",
    )
    .unwrap()
}

#[test]
fn raised_load_handler_error_bypasses_bsl_catch_and_ends_only_its_session() {
    let factory = factory();
    let open = || {
        factory
            .open(
                Principal::new("user", "user", vec![]),
                SessionResources::new(Box::new(()), 1),
            )
            .unwrap()
    };
    let mut control = open();
    let direct = control
        .instantiate("Функция Проверить() Экспорт ХранилищаНастроек.ЗамерХранилища.Загрузить(\"failure\"); КонецФункции")
        .unwrap();
    let error = control.call(&direct, "Проверить", vec![]).unwrap_err();
    assert!(
        error.to_string().contains("settings-load-probe"),
        "{error:?}"
    );
    let mut failed = open();
    let code = failed
        .instantiate(
            r#"Функция Сбой() Экспорт
                Попытка
                    ХранилищаНастроек.ЗамерХранилища.Загрузить("failure");
                Исключение
                    Возврат "caught";
                КонецПопытки;
                Возврат "missed";
            КонецФункции
            Функция Исправный() Экспорт Возврат 42; КонецФункции"#,
        )
        .unwrap();
    let error = failed.call(&code, "Сбой", vec![]).unwrap_err();
    assert!(error.to_string().contains("settings-load-probe"), "{error}");
    let SessionError::OpenBsl(Error::Runtime(RtError::CriticalConfiguration { operation, cause })) =
        error
    else {
        panic!("ожидалась критическая ошибка обработчика");
    };
    assert_eq!(operation, "ОбработкаЗагрузки");
    assert!(
        matches!(*cause, RtError::Raised(Value::Str(ref text)) if text.to_string() == "settings-load-probe")
    );
    assert!(failed.call(&code, "Исправный", vec![]).is_err());
    assert!(failed.instantiate("Возврат 42;").is_err());
    assert!(failed.execute("Возврат 42;").is_err());

    let mut fresh = open();
    let code = fresh
        .instantiate(
            r#"Функция Проверить() Экспорт
                Попытка ВызватьИсключение "обычная ошибка";
                Исключение КонецПопытки;
                Возврат ХранилищаНастроек.ЗамерХранилища.Загрузить("object");
            КонецФункции"#,
        )
        .unwrap();
    assert_eq!(
        fresh.call(&code, "Проверить", vec![]).unwrap().0,
        Value::Str("object|undefined|undefined|user".into())
    );
}

#[test]
fn raised_load_handler_error_prevents_session_startup() {
    let factory = factory_with_session(
        r#"Процедура УстановкаПараметровСеанса(Имена)
        Попытка ХранилищаНастроек.ЗамерХранилища.Загрузить("failure");
        Исключение КонецПопытки;
    КонецПроцедуры"#,
    );
    let error = factory
        .open(
            Principal::new("user", "user", vec![]),
            SessionResources::new(Box::new(()), 1),
        )
        .err()
        .unwrap();
    assert!(
        matches!(
            error,
            SessionError::OpenBsl(Error::Runtime(RtError::CriticalConfiguration { .. }))
        ),
        "{error:?}"
    );
}
