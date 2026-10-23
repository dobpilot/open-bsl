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
                include_str!("../../../../docs/research/fixtures/settings-storage-load/manager-handler-error.bsl"),
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

// По справке ошибка `ОбработкаЗагрузки` выходит из `Загрузить` ловимым
// исключением; критический отказ 8.3.27 сознательно не воспроизводится.
#[test]
fn raised_load_handler_error_is_catchable_and_keeps_the_session() {
    let factory = factory();
    let mut session = factory
        .open(
            Principal::new("user", "user", vec![]),
            SessionResources::new(Box::new(()), 1),
        )
        .unwrap();
    let direct = session
        .instantiate("Функция Проверить() Экспорт ХранилищаНастроек.ЗамерХранилища.Загрузить(\"failure\"); КонецФункции")
        .unwrap();
    let error = session.call(&direct, "Проверить", vec![]).unwrap_err();
    assert!(
        matches!(
            error,
            SessionError::OpenBsl(Error::Runtime(RtError::Raised(Value::Str(ref text))))
                if text.to_string() == "settings-load-probe"
        ),
        "{error:?}"
    );
    let code = session
        .instantiate(
            r#"Функция Сбой() Экспорт
                Попытка
                    ХранилищаНастроек.ЗамерХранилища.Загрузить("failure");
                Исключение
                    Возврат "caught:" + ОписаниеОшибки();
                КонецПопытки;
                Возврат "missed";
            КонецФункции
            Функция Исправный() Экспорт Возврат ХранилищаНастроек.ЗамерХранилища.Загрузить("object"); КонецФункции"#,
        )
        .unwrap();
    let Value::Str(caught) = session.call(&code, "Сбой", vec![]).unwrap().0 else {
        panic!("ожидалась строка");
    };
    let caught = caught.to_string();
    assert!(
        caught.starts_with("caught:") && caught.contains("settings-load-probe"),
        "{caught}"
    );
    assert_eq!(
        session.call(&code, "Исправный", vec![]).unwrap().0,
        Value::Str("object|undefined|undefined|user".into())
    );
    session.execute("Возврат 42;").unwrap();
}

#[test]
fn caught_load_handler_error_does_not_prevent_session_startup() {
    let factory = factory_with_session(
        r#"Процедура УстановкаПараметровСеанса(Имена)
        Попытка ХранилищаНастроек.ЗамерХранилища.Загрузить("failure");
        Исключение КонецПопытки;
    КонецПроцедуры"#,
    );
    factory
        .open(
            Principal::new("user", "user", vec![]),
            SessionResources::new(Box::new(()), 1),
        )
        .unwrap();
}
