use super::*;
use std::sync::Arc;

use bsl_config::{ConfigurationCodeSnapshot, ConfigurationModule, ModuleExecution, ModuleKind};
use bsl_orm::{MetadataCatalog, Principal};
use open_bsl::{Engine, Error};

fn host_data(parameters: Rc<SessionParameters>) -> Rc<SessionData> {
    Rc::new(
        SessionData::new(
            Arc::new(MetadataCatalog::new("configuration", vec![]).unwrap()),
            Principal::new("user", "Тест", Vec::new()),
        )
        .with_extension(Rc::new(SessionRuntime::new(
            parameters,
            SessionKind::ThinClient,
        ))),
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

fn entry(engine: &Engine, source: &str) -> open_bsl::Module {
    engine.compile(source).unwrap()
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
            ],
        )
        .unwrap(),
    )
}

fn principal() -> Principal {
    Principal::new("user", "Тест", Vec::new())
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

#[test]
fn factory_validates_once_and_opens_isolated_started_sessions() {
    #[derive(Debug, PartialEq)]
    struct Connection(u8);

    let factory = SessionFactory::new(
        code_snapshot(START_HANDLER),
        metadata_catalog("configuration"),
        "мОдУлЬсЕаНсА",
        SessionKind::ThinClient,
    )
    .unwrap();
    let mut first = factory.open(principal(), Box::new(Connection(1))).unwrap();
    let second = factory.open(principal(), Box::new(Connection(2))).unwrap();

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
fn factory_rejects_incompatible_metadata_and_startup_failure() {
    assert!(matches!(
        SessionFactory::new(
            code_snapshot(START_HANDLER),
            metadata_catalog("other"),
            "МодульСеанса",
            SessionKind::WebClient,
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
    )
    .unwrap();
    let error = factory.open(principal(), Box::new(())).err().unwrap();
    assert!(error.to_string().contains("старт не выполнен"));
}

#[test]
fn every_supported_session_kind_runs_full_startup() {
    let foreground = SessionFactory::new(
        code_snapshot(START_HANDLER),
        metadata_catalog("configuration"),
        "МодульСеанса",
        SessionKind::ThinClient,
    )
    .unwrap();
    let parent = foreground.open(principal(), Box::new(())).unwrap();

    for kind in [SessionKind::WebClient, SessionKind::ThinClient] {
        let factory = SessionFactory::new(
            code_snapshot(START_HANDLER),
            metadata_catalog("configuration"),
            "МодульСеанса",
            kind,
        )
        .unwrap();
        let session = factory.open(principal(), Box::new(())).unwrap();
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
    )
    .unwrap();
    let session = factory
        .open_background(principal(), Box::new(()), &parent)
        .unwrap();
    assert_eq!(session.kind(), SessionKind::BackgroundJob);
    assert_eq!(
        session.parameters().peek("Запущен").unwrap(),
        Some(Value::number_from_i64(1))
    );
}

#[test]
fn background_session_copies_parent_parameters_and_then_diverges() {
    let foreground = SessionFactory::new(
        code_snapshot(START_HANDLER),
        metadata_catalog("configuration"),
        "МодульСеанса",
        SessionKind::ThinClient,
    )
    .unwrap();
    let background = SessionFactory::new(
        code_snapshot(START_HANDLER),
        metadata_catalog("configuration"),
        "МодульСеанса",
        SessionKind::BackgroundJob,
    )
    .unwrap();
    let parent = foreground.open(principal(), Box::new(())).unwrap();
    parent
        .parameters()
        .set("Копируемый", Value::number_from_i64(7))
        .unwrap();
    let child = background
        .open_background(principal(), Box::new(()), &parent)
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
