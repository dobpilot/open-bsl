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
            connections: None,
            event_log_writer: None,
            client_settings: None,
            error_processing_settings: None,
            database_configuration_changed_dynamically: None,
            data_separation_common_attributes: None,
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

fn entry(engine: &Engine, source: &str) -> open_bsl::Module {
    engine.compile(source).unwrap()
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

#[derive(Debug, Default)]
struct RecordingEventLog {
    entries: RefCell<Vec<EventLogEntry>>,
}

impl EventLogWriter for RecordingEventLog {
    fn write(&self, entry: EventLogEntry) -> RtResult<()> {
        self.entries.borrow_mut().push(entry);
        Ok(())
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

    fn home_page_forms(&self, _: &Principal) -> RtResult<HomePageFormsData> {
        Ok(self.forms.borrow().clone())
    }

    fn set_home_page_forms(&self, _: &Principal, forms: &HomePageFormsData) -> RtResult<()> {
        *self.forms.borrow_mut() = forms.clone();
        Ok(())
    }
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
