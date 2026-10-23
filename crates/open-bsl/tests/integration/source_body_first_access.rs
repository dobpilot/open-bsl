use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use open_bsl::{
    ConfigurationIndex, ConfigurationModuleDeclaration, ConfigurationSource, Engine,
    ModuleDeclarationSource, ModuleExecution, ModuleKind, Value,
};

struct Source {
    index: ConfigurationIndex,
    bodies: HashMap<String, Arc<str>>,
    declarations: HashMap<String, Arc<str>>,
    reads: Mutex<Vec<String>>,
}

impl ConfigurationSource for Source {
    fn index(&self) -> &ConfigurationIndex {
        &self.index
    }

    fn module_source(&self, id: &str) -> Result<Arc<str>, String> {
        self.reads.lock().unwrap().push(id.into());
        self.bodies
            .get(id)
            .cloned()
            .ok_or_else(|| format!("нет тела «{id}»"))
    }

    fn module_declarations(&self, id: &str) -> Result<ModuleDeclarationSource, String> {
        self.declarations
            .get(id)
            .cloned()
            .map(ModuleDeclarationSource::Declarations)
            .ok_or_else(|| format!("нет объявлений «{id}»"))
    }
}

fn source(parent: &str, child: &str, child_declarations: &str) -> Arc<Source> {
    let server = ModuleExecution::new().with_server(true);
    Arc::new(Source {
        index: ConfigurationIndex::new(
            "first-access",
            vec![
                ConfigurationModuleDeclaration::new(
                    "parent",
                    "Родитель",
                    ModuleKind::Common,
                    server,
                ),
                ConfigurationModuleDeclaration::new("child", "Потомок", ModuleKind::Common, server),
            ],
        )
        .unwrap(),
        bodies: [
            ("parent".into(), Arc::from(parent)),
            ("child".into(), Arc::from(child)),
        ]
        .into(),
        declarations: [
            ("parent".into(), Arc::from(parent)),
            ("child".into(), Arc::from(child_declarations)),
        ]
        .into(),
        reads: Mutex::new(Vec::new()),
    })
}

fn engine(source: Arc<Source>) -> Engine {
    Engine::builder()
        .configuration_source(source)
        .configuration_roots(["Родитель"])
        .defer_common_module_name_errors(true)
        .build()
        .unwrap()
}

#[test]
fn unused_child_body_stays_unprepared_until_the_actual_call() {
    let source = source(
        "Функция Исправный() Экспорт Возврат 42; КонецФункции Процедура Дочерний() Экспорт Потомок.Проверить(); КонецПроцедуры",
        "Процедура Проверить() Экспорт ЗначениеПробы = Новый НамеренноНеизвестныйТипДляЗамера; КонецПроцедуры",
        "Процедура Проверить() Экспорт КонецПроцедуры",
    );
    let engine = engine(source.clone());
    assert_eq!(*source.reads.lock().unwrap(), ["parent"]);
    let good = engine
        .compile_entry("Возврат Родитель.Исправный();")
        .unwrap();
    let mut state = engine.new_state();
    assert_eq!(
        state.run(&good).unwrap(),
        Value::Number(open_bsl::BslNumber::from_i64(42))
    );
    assert_eq!(*source.reads.lock().unwrap(), ["parent"]);
    let bad = engine.compile_entry("Родитель.Дочерний();").unwrap();
    let error = state.run(&bad).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("НамеренноНеизвестныйТипДляЗамера")
    );
    assert_eq!(*source.reads.lock().unwrap(), ["parent", "child"]);
}

#[test]
fn cyclic_declarations_allow_real_code_and_keep_session_state_isolated() {
    let source = source(
        "Функция Следующее() Экспорт Возврат Потомок.Следующее(); КонецФункции Процедура Назад() Экспорт КонецПроцедуры",
        "Перем Счетчик; Функция Следующее() Экспорт Счетчик = Счетчик + 1; Возврат Счетчик; КонецФункции Процедура НеИспользуемая() Экспорт Родитель.Назад(); КонецПроцедуры Счетчик = 0;",
        "Функция Следующее() Экспорт КонецФункции Процедура НеИспользуемая() Экспорт КонецПроцедуры",
    );
    let engine = engine(source.clone());
    assert_eq!(*source.reads.lock().unwrap(), ["parent"]);
    let entry = engine
        .compile_entry(
            "Возврат Строка(Родитель.Следующее()) + \"/\" + Строка(Родитель.Следующее());",
        )
        .unwrap();
    let mut first = engine.new_state();
    assert_eq!(first.run(&entry).unwrap(), Value::Str("1/2".into()));
    let mut second = engine.new_state();
    assert_eq!(second.run(&entry).unwrap(), Value::Str("1/2".into()));
    assert_eq!(*source.reads.lock().unwrap(), ["parent", "child"]);
}

#[test]
fn inconsistent_declared_parameter_modes_are_rejected_before_execution() {
    let source = source(
        "Процедура Проверить() Экспорт ПараметрПробы = 1; Потомок.Проверить(ПараметрПробы); КонецПроцедуры",
        "Процедура Проверить(ПараметрПробы) Экспорт КонецПроцедуры",
        "Процедура Проверить(Знач ПараметрПробы) Экспорт КонецПроцедуры",
    );
    let engine = engine(source.clone());
    assert_eq!(*source.reads.lock().unwrap(), ["parent"]);
    let entry = engine.compile_entry("Родитель.Проверить();").unwrap();
    let error = engine.new_state().run(&entry).unwrap_err();
    assert!(error.to_string().contains("объявлен"));
}

#[test]
fn late_exported_variable_keeps_by_reference_arguments_and_defaults() {
    let source = source(
        "Функция Проверить() Экспорт Потомок.Изменить(Потомок.Счетчик); Возврат Потомок.Счетчик + Потомок.Умолчание(); КонецФункции",
        "Перем Счетчик Экспорт; Процедура Изменить(Параметр) Экспорт Параметр = Параметр + 7; КонецПроцедуры Функция Умолчание(Параметр = 9) Экспорт Возврат Параметр; КонецФункции Счетчик = 5;",
        "Перем Счетчик Экспорт; Процедура Изменить(Параметр) Экспорт КонецПроцедуры Функция Умолчание(Параметр = 9) Экспорт КонецФункции",
    );
    let engine = engine(source.clone());
    assert_eq!(*source.reads.lock().unwrap(), ["parent"]);
    let entry = engine
        .compile_entry("Возврат Родитель.Проверить();")
        .unwrap();
    assert_eq!(
        engine.new_state().run(&entry).unwrap(),
        Value::Number(open_bsl::BslNumber::from_i64(21))
    );
    assert_eq!(*source.reads.lock().unwrap(), ["parent", "child"]);
}
