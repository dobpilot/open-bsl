use super::*;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug)]
struct DataAccessStub;

impl DataAccess for DataAccessStub {
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

#[derive(Debug, Default)]
struct QueryAccessStub {
    calls: RefCell<Vec<QueryCall>>,
}

type QueryCall = (String, Vec<(String, Value)>);

impl DataAccess for QueryAccessStub {
    fn constant_value(
        &self,
        _: &MetadataObject,
        _: &Principal,
        _: &mut CallContext<'_>,
    ) -> RtResult<Value> {
        Err(RtError::DynamicError("константы не поддерживаются".into()))
    }

    fn execute_query(
        &self,
        request: &QueryRequest,
        principal: &Principal,
        _: &mut CallContext<'_>,
    ) -> RtResult<QueryResultData> {
        assert_eq!(principal.id(), "user");
        self.calls.borrow_mut().push((
            request.text().to_string(),
            request
                .parameters()
                .map(|(name, value)| (name.to_string(), value.clone()))
                .collect(),
        ));
        Ok(QueryResultData::new(
            vec!["Значение".into()],
            vec![vec![Value::number_from_i64(42)]],
        )
        .unwrap())
    }
}

const APPLIED_EXTENSION_ID: [u8; 16] = [0x11; 16];
const DISABLED_EXTENSION_ID: [u8; 16] = [0x22; 16];

fn extensions() -> ConfigurationExtensionsSnapshot {
    ConfigurationExtensionsSnapshot::new(
        vec![
            ConfigurationExtension::new(
                APPLIED_EXTENSION_ID,
                "Исправление",
                "1.2",
                "hash-one",
                ExtensionSafeMode::Enabled(true),
                false,
                true,
                ConfigurationExtensionScope::InfoBase,
            )
            .unwrap(),
            ConfigurationExtension::new(
                DISABLED_EXTENSION_ID,
                "Адаптация",
                "2.0",
                "hash-two",
                ExtensionSafeMode::profile("ПрофильРасширения"),
                true,
                false,
                ConfigurationExtensionScope::DataSeparation,
            )
            .unwrap(),
        ],
        [APPLIED_EXTENSION_ID],
        [DISABLED_EXTENSION_ID],
        vec![ExtensionApplicationIssue::new(
            DISABLED_EXTENSION_ID,
            "конфликт метода",
            ExtensionApplicationIssueSeverity::Critical,
        )],
    )
    .unwrap()
}

fn catalog() -> Arc<MetadataCatalog> {
    Arc::new(
        MetadataCatalog::new(
            "configuration",
            vec![
                MetadataObject::new("role-user", "Пользователь", MetadataKind::Role),
                MetadataObject::new("role-admin", "Администратор", MetadataKind::Role),
                MetadataObject::new("catalog-versions", "Версии", MetadataKind::Catalog)
                    .with_manager_module("ВерсииМенеджер"),
                MetadataObject::new("constant-answer", "Ответ", MetadataKind::Constant)
                    .with_manager_module("ОтветМенеджер"),
                MetadataObject::new(
                    "scheduled-job-cleanup",
                    "ОчисткаДанных",
                    MetadataKind::ScheduledJob,
                )
                .with_string_property("ИмяМетода", "Обслуживание.ОчиститьДанные"),
            ],
        )
        .unwrap()
        .with_configuration(
            "Демонстрационная",
            "1.2.3",
            ScriptVariant::Russian,
            ["role-user".into()],
        )
        .unwrap()
        .with_configuration_extensions(extensions()),
    )
}

fn engine() -> open_bsl::Engine {
    open_bsl::Engine::builder()
        .register_library(library())
        .common_module(
            "ВерсииМенеджер",
            r#"
            Перем Вызовов;
            Вызовов = 0;
            Функция Проверить(Добавка) Экспорт
                Если ЭтотОбъект.Имя <> "Версии" Тогда
                    ВызватьИсключение "неверный менеджер";
                КонецЕсли;
                Вызовов = Вызовов + 1;
                Возврат Добавка + Вызовов;
            КонецФункции
            "#,
        )
        .common_module(
            "ОтветМенеджер",
            r#"
            Функция Получить() Экспорт
                Возврат 999;
            КонецФункции
            "#,
        )
        .build()
        .unwrap()
}

fn state(engine: &open_bsl::Engine) -> open_bsl::State {
    state_with_data_access(engine, None)
}

fn state_with_data_access(
    engine: &open_bsl::Engine,
    data_access: Option<Rc<dyn DataAccess>>,
) -> open_bsl::State {
    let mut session = SessionData::new(
        catalog(),
        Principal::new("user", "Тест", vec!["role-user".into()]),
    );
    if let Some(data_access) = data_access {
        session = session.with_data_access(data_access);
    }
    engine.state_builder().host_data(Rc::new(session)).build()
}

#[test]
fn query_uses_the_session_adapter_and_exposes_a_measured_linear_result() {
    let engine = engine();
    let access = Rc::new(QueryAccessStub::default());
    let mut state = state_with_data_access(&engine, Some(access.clone()));

    assert_eq!(
        state
            .exec(
                r#"
                ЗапросПробы = Новый Запрос;
                Если Строка(ТипЗнч(ЗапросПробы)) <> "Query"
                    ИЛИ ЗапросПробы.Текст <> "" Тогда
                    Возврат Ложь;
                КонецЕсли;
                ЗапросПробы.Текст = "ВЫБРАТЬ &Значение КАК Значение";
                ЗапросПробы.УстановитьПараметр("Значение", 1);
                ЗапросПробы.УстановитьПараметр("зНаЧеНиЕ", 42);
                Результат = ЗапросПробы.Выполнить();
                Если Строка(ТипЗнч(Результат)) <> "Query result"
                    ИЛИ Результат.Пустой() Тогда
                    Возврат Ложь;
                КонецЕсли;
                Выборка = Результат.Выбрать();
                Если Строка(ТипЗнч(Выборка)) <> "Selected data from query result"
                    ИЛИ Выборка.Значение <> Неопределено
                    ИЛИ НЕ Выборка.Следующий()
                    ИЛИ Выборка.Значение <> 42
                    ИЛИ Выборка.Получить(0) <> 42
                    ИЛИ Выборка[0] <> 42
                    ИЛИ Выборка.Следующий()
                    ИЛИ Выборка.Значение <> 42 Тогда
                    Возврат Ложь;
                КонецЕсли;
                Таблица = Результат.Выгрузить();
                Возврат Строка(ТипЗнч(Таблица)) = "ТаблицаЗначений"
                    И Таблица.Количество() = 1
                    И Таблица[0].Значение = 42;
                "#,
            )
            .unwrap(),
        Value::Boolean(true)
    );

    assert_eq!(
        access.calls.borrow().as_slice(),
        [(
            "ВЫБРАТЬ &Значение КАК Значение".into(),
            vec![("Значение".into(), Value::number_from_i64(42))],
        )]
    );
}

#[test]
fn query_fails_closed_without_support_and_rejects_non_linear_traversal() {
    let engine = engine();
    let mut without_access = state(&engine);
    let error = without_access
        .exec("Возврат Новый Запрос(\"ВЫБРАТЬ 1\").Выполнить();")
        .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("доступ к данным конфигурации не подключён")
    );

    let mut unsupported = state_with_data_access(&engine, Some(Rc::new(DataAccessStub)));
    let error = unsupported
        .exec("Возврат Новый Запрос(\"ВЫБРАТЬ 1\").Выполнить();")
        .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("выполнение запросов не поддерживается адаптером данных")
    );

    let mut state = state_with_data_access(&engine, Some(Rc::new(QueryAccessStub::default())));
    let error = state
        .exec("Возврат Новый Запрос(\"ВЫБРАТЬ 1\").Выполнить().Выбрать(1);")
        .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("поддерживается только прямой обход по умолчанию")
    );
}

#[test]
fn query_result_data_rejects_ambiguous_or_misaligned_rows() {
    assert!(matches!(
        QueryResultData::new(vec!["Поле".into(), "пОлЕ".into()], Vec::<Vec<Value>>::new()),
        Err(QueryResultDataError::DuplicateColumnName { name }) if name == "пОлЕ"
    ));
    assert!(matches!(
        QueryResultData::new(vec!["Поле".into()], vec![Vec::new()]),
        Err(QueryResultDataError::RowWidth {
            row: 0,
            expected: 1,
            actual: 0,
        })
    ));
}

#[test]
fn value_storage_snapshots_and_detaches_mutable_values() {
    let engine = engine();
    let mut state = state(&engine);
    assert_eq!(
        state
            .exec(
                r#"
                Сжатие = Новый СжатиеДанных;
                Исходный = Новый Массив;
                Исходный.Добавить(1);
                Хранилище = Новый ХранилищеЗначения(Исходный, Сжатие);
                Исходный[0] = 2;
                Первое = Хранилище.Получить();
                Первое[0] = 3;
                Второе = Хранилище.Получить();

                Общий = Новый Массив;
                Общий.Добавить(4);
                САлиасом = Новый Массив;
                САлиасом.Добавить(Общий);
                САлиасом.Добавить(Общий);
                Извлеченный = Новый ХранилищеЗначения(
                    САлиасом, Новый Deflation(6)).Get();
                Извлеченный[0][0] = 5;

                Возврат Строка(ТипЗнч(Сжатие)) = "Compress Data"
                    И Строка(ТипЗнч(Хранилище)) = "Value storage"
                    И Второе[0] = 1
                    И Извлеченный[1][0] = 4;
                "#,
            )
            .unwrap(),
        Value::Boolean(true)
    );
}

#[test]
fn value_storage_rejects_invalid_compression_and_nonportable_values() {
    let engine = engine();
    let mut state = state(&engine);
    for source in [
        "Возврат Новый СжатиеДанных(-2);",
        "Возврат Новый СжатиеДанных(10);",
        "Возврат Новый СжатиеДанных(1.5);",
        "Возврат Новый ХранилищеЗначения(1, 1);",
        "Возврат Новый ХранилищеЗначения(Новый Запрос);",
    ] {
        assert!(state.exec(source).is_err(), "должен быть отказ: {source}");
    }
}

#[test]
fn role_available_accepts_a_name_and_a_metadata_role() {
    let engine = engine();
    let mut state = state(&engine);
    assert_eq!(
        state
            .exec("Возврат РольДоступна(\"пОлЬзОвАтЕлЬ\");")
            .unwrap(),
        Value::Boolean(true)
    );
    assert_eq!(
        state
            .exec("Возврат IsInRole(Метаданные.Roles.Администратор);")
            .unwrap(),
        Value::Boolean(false)
    );
    assert!(
        state
            .exec("Возврат РольДоступна(\"НетТакойРоли\");")
            .unwrap_err()
            .to_string()
            .contains("роль конфигурации не найдена")
    );
}

#[test]
fn configuration_metadata_exposes_name_version_language_and_main_roles() {
    let engine = engine();
    let mut state = state(&engine);

    assert_eq!(
        state
            .exec("Возврат Метаданные.Имя + \" \" + Метаданные.Версия;")
            .unwrap(),
        Value::Str("Демонстрационная 1.2.3".into())
    );
    assert_eq!(
        state
            .exec(
                "Возврат Метаданные.ВариантВстроенногоЯзыка = ВариантВстроенногоЯзыка.Русский И Метаданные.ВариантВстроенногоЯзыка = Метаданные.СвойстваОбъектов.ВариантВстроенногоЯзыка[\"Russian\"] ;"
            )
            .unwrap(),
        Value::Boolean(true)
    );
    assert_eq!(
        state
            .exec("Возврат Метаданные.ОсновныеРоли.Содержит(Метаданные.Роли.Пользователь);")
            .unwrap(),
        Value::Boolean(true)
    );
    assert_eq!(
        state
            .exec("Возврат Метаданные.DefaultRoles.Contains(Метаданные.Roles.Администратор);")
            .unwrap(),
        Value::Boolean(false)
    );
    assert_eq!(
        state
            .exec(
                r#"
                Имена = "";
                Для Каждого Задание Из Метаданные.РегламентныеЗадания Цикл
                    Имена = Имена + Задание.Имя + ":" + Задание.ИмяМетода;
                КонецЦикла;
                Возврат Имена;
                "#,
            )
            .unwrap(),
        Value::Str("ОчисткаДанных:Обслуживание.ОчиститьДанные".into())
    );
    assert_eq!(
        state
            .exec("Возврат Метаданные.Роли[0] = Метаданные.Роли.Пользователь;")
            .unwrap(),
        Value::Boolean(true)
    );
}

#[test]
fn missing_configuration_properties_and_invalid_main_roles_fail_explicitly() {
    let missing = Arc::new(
        MetadataCatalog::new(
            "configuration",
            vec![MetadataObject::new(
                "role-user",
                "Пользователь",
                MetadataKind::Role,
            )],
        )
        .unwrap(),
    );
    let engine = engine();
    let mut state = engine
        .state_builder()
        .host_data(Rc::new(SessionData::new(
            missing,
            Principal::new("user", "Тест", Vec::new()),
        )))
        .build();
    assert!(
        state
            .exec("Возврат Метаданные.Имя;")
            .unwrap_err()
            .to_string()
            .contains("свойства конфигурации отсутствуют")
    );

    let error = MetadataCatalog::new(
        "configuration",
        vec![MetadataObject::new(
            "role-user",
            "Пользователь",
            MetadataKind::Role,
        )],
    )
    .unwrap()
    .with_configuration(
        "Конфигурация",
        "1",
        ScriptVariant::Russian,
        ["missing".into()],
    )
    .unwrap_err();
    assert_eq!(error, CatalogError::UnknownMainRole("missing".into()));
}

#[test]
fn manager_export_runs_in_the_session_module_instance() {
    let engine = engine();
    let entry = engine
        .compile(
            r#"
            Функция Проверка() Экспорт
                Возврат Справочники.Версии.Проверить(10);
            КонецФункции
            "#,
        )
        .unwrap();
    let mut state = state(&engine);
    let instance = state.instantiate(&entry).unwrap();
    assert_eq!(
        instance.call(&mut state, "Проверка", vec![]).unwrap().0,
        Value::number_from_i64(11)
    );
    assert_eq!(
        instance.call(&mut state, "Проверка", vec![]).unwrap().0,
        Value::number_from_i64(12)
    );
}

#[test]
fn platform_manager_method_wins_over_same_named_module_export() {
    let engine = engine();
    let mut state = state_with_data_access(&engine, Some(Rc::new(DataAccessStub)));
    assert_eq!(
        state.exec("Возврат Константы.Ответ.Получить();").unwrap(),
        Value::number_from_i64(42)
    );
}

#[test]
fn platform_manager_method_does_not_fall_back_when_data_access_is_missing() {
    let engine = engine();
    let mut state = state(&engine);
    let error = state
        .exec("Возврат Константы.Ответ.Получить();")
        .unwrap_err()
        .to_string();
    assert!(error.contains("доступ к данным конфигурации не подключён"));
    assert!(!error.contains("999"));
}

#[test]
fn unknown_manager_member_fails_explicitly() {
    let engine = engine();
    let mut state = state(&engine);
    assert!(
        state
            .exec("Возврат Константы.НетТакойКонстанты;")
            .unwrap_err()
            .to_string()
            .contains("НетТакойКонстанты")
    );
    assert!(
        state
            .exec("Возврат Справочники.Версии.НетТакогоМетода();")
            .unwrap_err()
            .to_string()
            .contains("НетТакогоМетода")
    );
}

#[test]
fn configuration_extensions_filter_each_complete_source() {
    let engine = engine();
    let mut state = state(&engine);
    assert_eq!(
        state
            .exec(
                r#"
                Активные = РасширенияКонфигурации.Получить(
                    Новый Структура("Имя", "иСпРаВлЕнИе"),
                    ИсточникРасширенийКонфигурации.СеансАктивные);
                Если Активные.Количество() <> 1 Тогда
                    Возврат "wrong applied count";
                КонецЕсли;
                Расширение = Активные[0];
                ПоИдентификатору = ConfigurationExtensions.Get(
                    Новый Структура("UUID", Расширение.UUID),
                    ConfigurationExtensionsSource.Database);
                Возврат Расширение.Name + ":" + Расширение.Version + ":"
                    + Расширение.HashSum + ":" + Строка(Расширение.SafeMode) + ":"
                    + Строка(Расширение.UsedInDistributedInfoBase) + ":"
                    + Строка(Расширение.Active) + ":" + Строка(Расширение.Scope)
                    + ":" + Строка(ПоИдентификатору.Количество());
                "#,
            )
            .unwrap(),
        Value::Str("Исправление:1.2:hash-one:Да:Нет:Да:Infobase:1".into())
    );

    assert_eq!(
        state
            .exec(
                "Возврат РасширенияКонфигурации.Получить(, ИсточникРасширенийКонфигурации.СеансОтключенные).Количество();"
            )
            .unwrap(),
        Value::number_from_i64(1)
    );
}

#[test]
fn configuration_extension_issues_refer_to_the_same_extension_snapshot() {
    let engine = engine();
    let mut state = state(&engine);
    assert_eq!(
        state
            .exec(
                r#"
                Проблема = РасширенияКонфигурации
                    .ПолучитьИнформациюОПроблемахПримененияВСеансе()[0];
                Отключенное = РасширенияКонфигурации.Получить(
                    , ИсточникРасширенийКонфигурации.СеансОтключенные)[0];
                Возврат Проблема.Расширение = Отключенное
                    И Проблема.Описание = "конфликт метода"
                    И Проблема.Важность = ВажностьПроблемыПримененияРасширенияКонфигурации.Критичная;
                "#,
            )
            .unwrap(),
        Value::Boolean(true)
    );
}

#[test]
fn complete_empty_and_missing_extension_snapshots_are_distinct() {
    let empty_catalog = Arc::new(
        MetadataCatalog::new("configuration", Vec::new())
            .unwrap()
            .with_configuration_extensions(
                ConfigurationExtensionsSnapshot::new(
                    Vec::new(),
                    Vec::new(),
                    Vec::new(),
                    Vec::new(),
                )
                .unwrap(),
            ),
    );
    let missing_catalog = Arc::new(MetadataCatalog::new("configuration", Vec::new()).unwrap());
    let engine = engine();
    let make_state = |catalog| {
        engine
            .state_builder()
            .host_data(Rc::new(SessionData::new(
                catalog,
                Principal::new("user", "Тест", Vec::new()),
            )))
            .build()
    };

    let mut empty_state = make_state(empty_catalog);
    assert_eq!(
        empty_state
            .exec("Возврат РасширенияКонфигурации.Получить().Количество();")
            .unwrap(),
        Value::number_from_i64(0)
    );
    assert_eq!(
        empty_state
            .exec(
                "Возврат РасширенияКонфигурации.ПолучитьИнформациюОПроблемахПримененияВСеансе().Количество();"
            )
            .unwrap(),
        Value::number_from_i64(0)
    );

    let mut missing_state = make_state(missing_catalog);
    assert!(
        missing_state
            .exec("Возврат РасширенияКонфигурации.Получить();")
            .unwrap_err()
            .to_string()
            .contains("не загружены для текущего сеанса")
    );
}

#[test]
fn extension_snapshot_rejects_dangling_references() {
    let extension = ConfigurationExtension::new(
        APPLIED_EXTENSION_ID,
        "Исправление",
        "1",
        "hash",
        ExtensionSafeMode::Enabled(false),
        false,
        true,
        ConfigurationExtensionScope::InfoBase,
    )
    .unwrap();
    assert_eq!(
        ConfigurationExtensionsSnapshot::new(
            vec![extension.clone()],
            [DISABLED_EXTENSION_ID],
            Vec::new(),
            Vec::new(),
        )
        .unwrap_err(),
        CatalogError::UnknownExtensionIdentity
    );
    assert_eq!(
        ConfigurationExtensionsSnapshot::new(
            vec![extension],
            Vec::new(),
            Vec::new(),
            vec![ExtensionApplicationIssue::new(
                DISABLED_EXTENSION_ID,
                "нет расширения",
                ExtensionApplicationIssueSeverity::Low,
            )],
        )
        .unwrap_err(),
        CatalogError::UnknownIssueExtension
    );
}
