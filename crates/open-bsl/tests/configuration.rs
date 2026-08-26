//! Конфигурация фасада: общие модули, entry с импортами, изоляция
//! сессионных экземпляров между запусками.

use open_bsl::Engine;

fn engine_with_service() -> Engine {
    Engine::builder()
        .common_module(
            "Служебный",
            "Перем Счётчик Экспорт;\n\
             Функция Удвоить(Знач х) Экспорт\n\
                 Счётчик = Счётчик + 1;\n\
                 Возврат х * 2;\n\
             КонецФункции\n\
             Счётчик = 100;",
        )
        .build()
        .expect("движок с каталогом собирается")
}

#[test]
fn an_entry_calls_the_common_module_through_the_facade() {
    let engine = engine_with_service();
    let module = engine
        .compile_entry(
            "Служебный.Счётчик = Служебный.Удвоить(Служебный.Счётчик) + 1;\n\
             Возврат Служебный.Счётчик;",
        )
        .expect("entry компилируется");
    let mut state = engine.new_state();
    let value = state.run(&module).expect("прогон завершается");
    assert_eq!(open_bsl::format_value(&value, None).unwrap(), "201");
}

/// Каждый запуск получает свежие экземпляры модулей: инициализация
/// выполняется заново, состояние прошлого запуска не протекает.
#[test]
fn each_run_gets_fresh_module_instances() {
    let engine = engine_with_service();
    let module = engine
        .compile_entry(
            "Служебный.Счётчик = Служебный.Счётчик + 7;\n\
             Возврат Служебный.Счётчик;",
        )
        .expect("entry компилируется");
    let mut state = engine.new_state();
    let first = state.run(&module).expect("первый прогон");
    let second = state.run(&module).expect("второй прогон");
    assert_eq!(open_bsl::format_value(&first, None).unwrap(), "107");
    assert_eq!(open_bsl::format_value(&second, None).unwrap(), "107");
}

/// Импорты между общими модулями: клиентский модуль каталога зовёт
/// служебный, entry зовёт клиентский.
#[test]
fn common_modules_import_each_other() {
    let recipe = open_bsl::ModuleGraphRecipe {
        modules: vec![
            open_bsl::ModuleRecipe {
                name: "Служебный".to_string(),
                source: "Функция База() Экспорт\n Возврат 20;\nКонецФункции".to_string(),
                imports: Vec::new(),
            },
            open_bsl::ModuleRecipe {
                name: "Клиент".to_string(),
                source:
                    "Функция Сумма(Знач д) Экспорт\n Возврат Служебный.База() + д;\nКонецФункции"
                        .to_string(),
                imports: vec![("Служебный".to_string(), "Служебный".to_string())],
            },
        ],
    };
    let engine = Engine::builder()
        .configuration(recipe)
        .build()
        .expect("движок с графом собирается");
    let module = engine
        .compile_entry("Возврат Клиент.Сумма(3);")
        .expect("entry компилируется");
    let mut state = engine.new_state();
    let value = state.run(&module).expect("прогон завершается");
    assert_eq!(open_bsl::format_value(&value, None).unwrap(), "23");
}

/// Цикл импортов отвергается на сборке движка, а не в рантайме.
#[test]
fn an_import_cycle_is_a_build_error() {
    let recipe = open_bsl::ModuleGraphRecipe {
        modules: vec![
            open_bsl::ModuleRecipe {
                name: "А".to_string(),
                source: "Функция Ф() Экспорт\n Возврат 1;\nКонецФункции".to_string(),
                imports: vec![("Б".to_string(), "Б".to_string())],
            },
            open_bsl::ModuleRecipe {
                name: "Б".to_string(),
                source: "Функция Г() Экспорт\n Возврат 2;\nКонецФункции".to_string(),
                imports: vec![("А".to_string(), "А".to_string())],
            },
        ],
    };
    let Err(error) = Engine::builder().configuration(recipe).build() else {
        panic!("цикл импортов должен отвергаться");
    };
    assert!(error.to_string().contains("цикл"), "не та ошибка: {error}");
}

/// Неэкспортный метод не виден снаружи модуля.
#[test]
fn a_non_exported_function_is_invisible_to_the_entry() {
    let engine = Engine::builder()
        .common_module("Служебный", "Функция Скрытая()\n Возврат 1;\nКонецФункции")
        .build()
        .expect("движок собирается");
    let Err(error) = engine.compile_entry("Возврат Служебный.Скрытая();")
    else {
        panic!("неэкспортная функция не должна быть видна");
    };
    assert!(
        error.to_string().contains("Служебный.Скрытая"),
        "не та ошибка: {error}"
    );
}
