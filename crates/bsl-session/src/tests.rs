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
        .with_extension(Rc::new(SessionRuntime::new(
            parameters,
            SessionKind::ThinClient,
            Arc::from(PLATFORM_VERSION),
            None,
            1,
            None,
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
        .open(principal(), Box::new(Connection(1)), 101)
        .unwrap();
    let second = factory
        .open(principal(), Box::new(Connection(2)), 102)
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
        .open_with_data_access(principal(), Box::new(()), 1, Rc::new(ConstantAccess))
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

    let session = factory.open(principal(), Box::new(()), 1).unwrap();
    assert_eq!(
        session.parameters().peek("Запущен").unwrap(),
        Some(Value::Str(PLATFORM_VERSION.into()))
    );
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
        .open(principal(), Box::new(()), 1)
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
            Box::new(()),
            2,
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
    let mut missing = missing_factory.open(principal(), Box::new(()), 1).unwrap();
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
        .open(principal(), Box::new(()), 41)
        .unwrap();
    let mut second = foreground_factory
        .open(principal(), Box::new(()), 42)
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
            Box::new(()),
            43,
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
            .open(principal(), Box::new(()), number)
            .err()
            .unwrap();
        assert!(matches!(
            error,
            SessionError::InvalidConnectionNumber(value) if value == number
        ));
    }
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
    let mut session = factory.open(principal(), Box::new(()), 1).unwrap();
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
    let error = session
        .call(&module, "ИменованныйПрофиль", vec![])
        .unwrap_err();
    assert!(error.to_string().contains("не поддерживаются"));
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
    let error = factory.open(principal(), Box::new(()), 1).err().unwrap();
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
    let parent = foreground.open(principal(), Box::new(()), 1).unwrap();

    for kind in [SessionKind::WebClient, SessionKind::ThinClient] {
        let factory = SessionFactory::new(
            code_snapshot(START_HANDLER),
            metadata_catalog("configuration"),
            "МодульСеанса",
            kind,
            PLATFORM_VERSION,
        )
        .unwrap();
        let session = factory.open(principal(), Box::new(()), 2).unwrap();
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
        .open_background(principal(), Box::new(()), 3, "Обработка.Запустить", &parent)
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
        let mut session = factory.open(principal(), Box::new(()), 1).unwrap();
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
    let parent = foreground.open(principal(), Box::new(()), 1).unwrap();
    let background = SessionFactory::new(
        code_snapshot(START_HANDLER),
        metadata_catalog("configuration"),
        "МодульСеанса",
        SessionKind::BackgroundJob,
        PLATFORM_VERSION,
    )
    .unwrap();
    let mut session = background
        .open_background(principal(), Box::new(()), 2, "Обработка.Режим", &parent)
        .unwrap();
    assert_eq!(session_run_mode(&mut session), Value::Undefined);
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
        .open(principal(), Box::new(()), 1)
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
            Box::new(()),
            2,
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

    let error = background_factory
        .open_background(principal(), Box::new(()), 3, "", &foreground)
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
    let parent = foreground.open(principal(), Box::new(()), 1).unwrap();
    parent
        .parameters()
        .set("Копируемый", Value::number_from_i64(7))
        .unwrap();
    let child = background
        .open_background(principal(), Box::new(()), 2, "Обработка.Копия", &parent)
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
