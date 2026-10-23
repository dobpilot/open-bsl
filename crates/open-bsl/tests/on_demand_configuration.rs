use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use open_bsl::{
    ConfigurationIndex, ConfigurationModuleDeclaration, ConfigurationSource, Engine, Value,
};
use open_bsl::{ModuleExecution, ModuleKind};

struct CountingSource {
    index: ConfigurationIndex,
    sources: HashMap<String, Arc<str>>,
    reads: Mutex<Vec<String>>,
}

impl ConfigurationSource for CountingSource {
    fn index(&self) -> &ConfigurationIndex {
        &self.index
    }

    fn module_source(&self, id: &str) -> Result<Arc<str>, String> {
        self.reads.lock().unwrap().push(id.into());
        self.sources
            .get(id)
            .cloned()
            .ok_or_else(|| format!("нельзя загружать лишний модуль «{id}»"))
    }
}

fn source() -> Arc<CountingSource> {
    let server = ModuleExecution::new().with_server(true);
    let mut modules = vec![
        ConfigurationModuleDeclaration::new("session", "МодульСеанса", ModuleKind::Session, server),
        ConfigurationModuleDeclaration::new("broker", "Посредник", ModuleKind::Common, server),
        ConfigurationModuleDeclaration::new("target", "Цель", ModuleKind::Common, server),
    ];
    modules.extend((0..32).map(|index| {
        ConfigurationModuleDeclaration::new(
            format!("unused{index}"),
            format!("Лишний{index}"),
            ModuleKind::Common,
            server,
        )
    }));
    Arc::new(CountingSource {
        index: ConfigurationIndex::new("configuration", modules).unwrap(),
        sources: [
            ("session".into(), Arc::from("Функция Старт(Имя) Экспорт Возврат Посредник.Вызвать(Имя); КонецФункции")),
            ("broker".into(), Arc::from("Функция Вызвать(Имя) Экспорт Модуль = Вычислить(Имя); Возврат Модуль.Следующее(); КонецФункции")),
            ("target".into(), Arc::from("Перем Счетчик; Счетчик = 0; Функция Следующее() Экспорт Счетчик = Счетчик + 1; Возврат Счетчик; КонецФункции")),
        ].into(),
        reads: Mutex::new(Vec::new()),
    })
}

#[derive(Debug, Default)]
struct GroupedContexts {
    groups: Mutex<Vec<Vec<String>>>,
}

impl open_bsl::ModuleContextSource for GroupedContexts {
    fn prepare_contexts(&self, modules: &[&str]) -> Result<(), String> {
        self.groups
            .lock()
            .unwrap()
            .push(modules.iter().map(|name| (*name).into()).collect());
        Ok(())
    }

    fn receiver_properties(&self, module: &str) -> Result<Vec<String>, String> {
        if !self
            .groups
            .lock()
            .unwrap()
            .iter()
            .flatten()
            .any(|name| name == module)
        {
            return Err(format!("контекст «{module}» не приобретён группой"));
        }
        Ok(Vec::new())
    }
}

#[test]
fn reached_contexts_are_acquired_at_first_body_access_without_unused_modules() {
    let mut source = source();
    Arc::get_mut(&mut source).unwrap().sources.insert("session".into(), Arc::from(
        "Функция Старт(Имя) Экспорт Возврат Посредник.Вызвать(Имя) + Цель.Следующее(); КонецФункции"
    ));
    let contexts = Arc::new(GroupedContexts::default());
    let engine = Engine::builder()
        .configuration_source(source)
        .configuration_context_source(contexts.clone())
        .configuration_roots(["МодульСеанса"])
        .build()
        .unwrap();
    let (value, _) = engine
        .state_builder()
        .build()
        .call_configuration_handler("МодульСеанса", "Старт", vec![Value::Str("Цель".into())])
        .unwrap();
    assert_eq!(value, Value::number_from_i64(3));
    let groups = contexts.groups.lock().unwrap();
    assert_eq!(
        *groups,
        [
            vec!["МодульСеанса".to_string()],
            vec!["Посредник".to_string()],
            vec!["Цель".to_string()],
        ]
    );
    assert!(
        groups
            .iter()
            .flatten()
            .all(|name| !name.starts_with("Лишний"))
    );
}

#[test]
fn source_preparation_follows_static_dependencies_and_loads_dynamic_target_at_first_access() {
    let source = source();
    let engine = Engine::builder()
        .configuration_source(source.clone())
        .configuration_roots(["МодульСеанса"])
        .build()
        .unwrap();
    assert_eq!(*source.reads.lock().unwrap(), ["session", "broker"]);
    let mut first = engine.state_builder().build();
    for expected in [1, 2] {
        let (value, _) = first
            .call_configuration_handler("МодульСеанса", "Старт", vec![Value::Str("Цель".into())])
            .unwrap();
        assert_eq!(value, Value::number_from_i64(expected));
    }
    let mut second = engine.state_builder().build();
    assert_eq!(
        second
            .call_configuration_handler("МодульСеанса", "Старт", vec![Value::Str("Цель".into())])
            .unwrap()
            .0,
        Value::number_from_i64(1)
    );
    assert_eq!(
        *source.reads.lock().unwrap(),
        ["session", "broker", "target"]
    );
}

#[test]
fn local_variable_with_module_name_does_not_load_that_modules_source() {
    let source = source();
    let engine = Engine::builder()
        .configuration_source(source.clone())
        .configuration_roots(["МодульСеанса"])
        .build()
        .unwrap();
    let entry = engine
        .compile_entry("Лишний0 = Новый Структура(\"Значение\", 17); Возврат Лишний0.Значение;")
        .unwrap();
    assert_eq!(
        engine.state_builder().build().run(&entry).unwrap(),
        Value::number_from_i64(17)
    );
    assert!(
        !source
            .reads
            .lock()
            .unwrap()
            .iter()
            .any(|id| id.starts_with("unused"))
    );
}

#[test]
fn runtime_created_fields_do_not_collide_with_names_of_a_later_module() {
    let mut source = source();
    let data = Arc::get_mut(&mut source).unwrap();
    data.sources.insert(
        "session".into(),
        Arc::from(
            "Функция Старт(Имя) Экспорт
            Временная = Новый Структура; Временная.Вставить(\"ТолькоВоВремяИсполнения\", 1);
            Возврат Посредник.Вызвать(Имя);
         КонецФункции",
        ),
    );
    data.sources.insert(
        "target".into(),
        Arc::from(
            "Функция Следующее() Экспорт
            Результат = Новый Структура(\"ТолькоВПозднемМодуле\", 17);
            Возврат Результат[\"ТолькоВПозднемМодуле\"];
         КонецФункции",
        ),
    );
    let engine = Engine::builder()
        .configuration_source(source)
        .configuration_roots(["МодульСеанса"])
        .build()
        .unwrap();
    let result = engine
        .new_state()
        .call_configuration_handler("МодульСеанса", "Старт", vec![Value::Str("Цель".into())])
        .unwrap()
        .0;
    assert_eq!(result, Value::number_from_i64(17));
}

#[test]
fn delayed_module_values_keep_their_names_across_handler_calls() {
    let mut source = source();
    let data = Arc::get_mut(&mut source).unwrap();
    data.sources.insert(
        "session".into(),
        Arc::from(
            "Функция Старт(Имя) Экспорт
            Если Имя = \"Первый\" Тогда
                Временная = Новый Структура; Временная.Вставить(\"ТолькоВоВремяИсполнения\", 1);
            КонецЕсли;
            Возврат Посредник.Вызвать(\"Цель\");
         КонецФункции",
        ),
    );
    data.sources.insert("target".into(), Arc::from(
        "Перем Данные;
         Функция Следующее() Экспорт
            Если Данные = Неопределено Тогда Данные = Новый Структура(\"ПозднееПоле\", 17); КонецЕсли;
            Возврат Данные[\"ПозднееПоле\"];
         КонецФункции"));
    let engine = Engine::builder()
        .configuration_source(source.clone())
        .configuration_roots(["МодульСеанса"])
        .build()
        .unwrap();
    let mut session = engine.new_state();
    for name in ["Первый", "Другой"] {
        assert_eq!(
            session
                .call_configuration_handler("МодульСеанса", "Старт", vec![Value::Str(name.into())])
                .unwrap()
                .0,
            Value::number_from_i64(17)
        );
    }
    assert_eq!(
        source
            .reads
            .lock()
            .unwrap()
            .iter()
            .filter(|id| id.as_str() == "target")
            .count(),
        1
    );
}

#[test]
fn invalid_unused_body_is_deferred_and_its_reached_error_is_catchable() {
    let mut source = source();
    let data = Arc::get_mut(&mut source).unwrap();
    data.sources.insert(
        "target".into(),
        Arc::from("Функция Следующее() Экспорт Возврат +; КонецФункции"),
    );
    let engine = Engine::builder()
        .configuration_source(source.clone())
        .configuration_roots(["МодульСеанса"])
        .build()
        .unwrap();
    assert_eq!(*source.reads.lock().unwrap(), ["session", "broker"]);
    let entry = engine
        .compile_entry(
            "Попытка Посредник.Вызвать(\"Цель\"); Исключение Возврат \"caught\"; КонецПопытки;",
        )
        .unwrap();
    assert_eq!(
        engine.new_state().run(&entry).unwrap(),
        Value::Str("caught".into())
    );
}

#[test]
fn explicit_import_cycles_are_rejected_without_reading_source() {
    let mut source = source();
    let data = Arc::get_mut(&mut source).unwrap();
    let mut modules = data.index.modules().to_vec();
    modules[1] = modules[1]
        .clone()
        .with_imports(vec![open_bsl::ModuleImport::new("Цель", "target")]);
    modules[2] = modules[2]
        .clone()
        .with_imports(vec![open_bsl::ModuleImport::new("Посредник", "broker")]);
    data.index = ConfigurationIndex::new("configuration", modules).unwrap();
    let result = Engine::builder()
        .configuration_source(source.clone())
        .configuration_roots(["МодульСеанса"])
        .build();
    assert!(
        matches!(result, Err(open_bsl::Error::Configuration(message))
        if message == "граф импортов общих модулей содержит цикл")
    );
    assert!(source.reads.lock().unwrap().is_empty());
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn background_worker_prepares_its_target_without_reading_unused_modules() {
    let source = source();
    let engine = Engine::builder()
        .configuration_source(source.clone())
        .configuration_roots(["МодульСеанса"])
        .background_jobs(open_bsl::jobs::BackgroundJobConfig {
            workers: Some(1),
            ..Default::default()
        })
        .build()
        .unwrap();
    assert_eq!(*source.reads.lock().unwrap(), ["session", "broker"]);
    let entry = engine
        .compile_entry(
            "Задание = ФоновыеЗадания.Выполнить(\"Цель.Следующее\");
         Задание = Задание.ОжидатьЗавершенияВыполнения();
         Возврат Строка(Задание.Состояние);",
        )
        .unwrap();
    assert_eq!(
        engine.new_state().run(&entry).unwrap(),
        Value::Str("Завершено".into())
    );
    assert!(
        !source
            .reads
            .lock()
            .unwrap()
            .iter()
            .any(|id| id.starts_with("unused"))
    );
}

#[test]
fn semantic_dependency_cycles_share_stable_module_identities() {
    let mut source = source();
    let data = Arc::get_mut(&mut source).unwrap();
    data.sources.insert(
        "broker".into(),
        Arc::from(
            "Функция Вызвать(Имя) Экспорт Возврат Цель.Следующее(); КонецФункции
         Функция Другое() Экспорт Возврат 17; КонецФункции",
        ),
    );
    data.sources.insert(
        "target".into(),
        Arc::from("Функция Следующее() Экспорт Возврат Посредник.Другое(); КонецФункции"),
    );
    let engine = Engine::builder()
        .configuration_source(source.clone())
        .configuration_roots(["МодульСеанса"])
        .build()
        .unwrap();
    assert_eq!(
        engine
            .new_state()
            .call_configuration_handler("МодульСеанса", "Старт", vec![Value::Undefined])
            .unwrap()
            .0,
        Value::number_from_i64(17)
    );
    assert_eq!(
        *source.reads.lock().unwrap(),
        ["session", "broker", "target"]
    );
}

#[test]
fn conflicting_global_exports_fail_before_any_body_is_prepared() {
    let mut source = source();
    let data = Arc::get_mut(&mut source).unwrap();
    let global = ModuleExecution::new().with_server(true).with_global(true);
    let mut modules = data.index.modules().to_vec();
    modules[1] =
        ConfigurationModuleDeclaration::new("broker", "Посредник", ModuleKind::Common, global);
    modules[2] = ConfigurationModuleDeclaration::new("target", "Цель", ModuleKind::Common, global);
    data.index = ConfigurationIndex::new("configuration", modules).unwrap();
    data.sources.insert(
        "broker".into(),
        Arc::from("Функция Совпадение() Экспорт Возврат +; КонецФункции"),
    );
    data.sources.insert(
        "target".into(),
        Arc::from("Функция СОВПАДЕНИЕ() Экспорт Возврат +; КонецФункции"),
    );
    let result = Engine::builder()
        .configuration_source(source.clone())
        .configuration_roots(["МодульСеанса"])
        .build();
    assert!(matches!(result, Err(open_bsl::Error::Configuration(_))));
    assert_eq!(*source.reads.lock().unwrap(), ["broker", "target"]);
}

#[test]
fn whole_catalog_preparation_still_parses_unreached_sources_unlike_the_indexed_path() {
    let source = source();
    let modules = source
        .index
        .modules()
        .iter()
        .map(|module| open_bsl::ModuleRecipe {
            name: module.name().into(),
            source: source
                .sources
                .get(module.id())
                .map(|text| text.to_string())
                .unwrap_or_else(|| {
                    "Функция НеИспользована() Экспорт Возврат +; КонецФункции".into()
                }),
            visibility: if module.kind() == ModuleKind::Session {
                open_bsl::ModuleVisibility::Explicit
            } else {
                open_bsl::ModuleVisibility::Named
            },
            imports: vec![],
        })
        .collect();
    assert!(
        Engine::builder()
            .configuration(open_bsl::ModuleGraphRecipe {
                modules,
                eager_init: false
            })
            .configuration_roots(["МодульСеанса"])
            .build()
            .is_err()
    );
    assert!(
        Engine::builder()
            .configuration_source(source.clone())
            .configuration_roots(["МодульСеанса"])
            .build()
            .is_ok()
    );
    assert_eq!(*source.reads.lock().unwrap(), ["session", "broker"]);
}

#[test]
fn dynamically_constructed_call_prepares_only_its_actual_module() {
    let mut source = source();
    Arc::get_mut(&mut source).unwrap().sources.insert(
        "broker".into(),
        Arc::from(
            "Функция Вызвать(Имя) Экспорт Возврат Вычислить(Имя + \".Следующее()\"); КонецФункции",
        ),
    );
    let engine = Engine::builder()
        .configuration_source(source.clone())
        .configuration_roots(["МодульСеанса"])
        .build()
        .unwrap();
    assert_eq!(*source.reads.lock().unwrap(), ["session", "broker"]);
    let mut state = engine.new_state();
    for expected in [1, 2] {
        assert_eq!(
            state
                .call_configuration_handler(
                    "МодульСеанса",
                    "Старт",
                    vec![Value::Str("Цель".into())]
                )
                .unwrap()
                .0,
            Value::number_from_i64(expected)
        );
    }
    assert_eq!(
        *source.reads.lock().unwrap(),
        ["session", "broker", "target"]
    );
}

#[test]
fn declaration_only_source_is_not_executed_and_export_chunk_positions_stay_stable() {
    struct Protected {
        index: ConfigurationIndex,
        bodies: Mutex<usize>,
    }
    impl ConfigurationSource for Protected {
        fn index(&self) -> &ConfigurationIndex {
            &self.index
        }
        fn module_source(&self, _: &str) -> Result<Arc<str>, String> {
            *self.bodies.lock().unwrap() += 1;
            Ok(Arc::from(
                "Процедура Частный() КонецПроцедуры Функция Значение() Экспорт Возврат 73; КонецФункции",
            ))
        }
        fn module_declarations(
            &self,
            _: &str,
        ) -> Result<open_bsl::ModuleDeclarationSource, String> {
            Ok(open_bsl::ModuleDeclarationSource::Declarations(Arc::from(
                "Процедура Частный() КонецПроцедуры Функция Значение() Экспорт КонецФункции",
            )))
        }
    }
    let source = Arc::new(Protected {
        index: ConfigurationIndex::new(
            "configuration",
            vec![ConfigurationModuleDeclaration::new(
                "global",
                "Глобальный",
                ModuleKind::Common,
                ModuleExecution::new().with_server(true).with_global(true),
            )],
        )
        .unwrap(),
        bodies: Mutex::new(0),
    });
    let engine = Engine::builder()
        .configuration_source(source.clone())
        .build()
        .unwrap();
    assert_eq!(*source.bodies.lock().unwrap(), 0);
    let entry = engine.compile_entry("Возврат Значение();").unwrap();
    assert_eq!(
        engine.new_state().run(&entry).unwrap(),
        Value::number_from_i64(73)
    );
    assert_eq!(*source.bodies.lock().unwrap(), 1);
    assert_eq!(
        engine.new_state().run(&entry).unwrap(),
        Value::number_from_i64(73)
    );
    assert_eq!(*source.bodies.lock().unwrap(), 1);
}
