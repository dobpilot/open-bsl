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
        eager_init: false,
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

/// Нележивая инициализация (`eager_init`): тело модуля выполняется до
/// первой инструкции entry, даже если entry не касается его символов.
#[test]
fn eager_init_runs_module_bodies_before_the_entry() {
    use std::io::Write;
    use std::rc::Rc;
    use std::sync::Mutex;

    #[derive(Clone, Default)]
    struct Capture(Rc<Mutex<Vec<u8>>>);
    impl Write for Capture {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    let run = |eager: bool| -> String {
        let recipe = open_bsl::ModuleGraphRecipe {
            modules: vec![open_bsl::ModuleRecipe {
                name: "Служебный".to_string(),
                source: "Сообщить(\"из тела\");".to_string(),
                imports: Vec::new(),
            }],
            eager_init: eager,
        };
        let engine = Engine::builder()
            .configuration(recipe)
            .build()
            .expect("движок собирается");
        // Entry не касается символов модуля: разница выводов — ровно
        // нележивая инициализация.
        let module = engine
            .compile_entry("Сообщить(\"из entry\");")
            .expect("entry компилируется");
        let capture = Capture::default();
        let mut state = engine.state_builder().stdout(capture.clone()).build();
        state.run(&module).expect("прогон завершается");
        let bytes = capture.0.lock().unwrap().clone();
        String::from_utf8(bytes).unwrap()
    };

    assert_eq!(run(true), "из тела\nиз entry\n");
    assert_eq!(run(false), "из entry\n");
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
        eager_init: false,
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

/// Runtime фоновых заданий: клоны движка разделяют его, задание доходит
/// до Completed через публичный путь Engine::job_runtime.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn engine_clones_share_the_job_runtime() {
    use std::sync::Arc;

    let engine = Engine::builder()
        .common_module(
            "Служебный",
            "Функция Эхо(Знач х) Экспорт\n Возврат х;\nКонецФункции",
        )
        .build()
        .expect("движок собирается");
    let clone = engine.clone();
    let runtime = engine.job_runtime().expect("runtime создаётся");
    let from_clone = clone.job_runtime().expect("у клона тот же runtime");
    assert!(
        Arc::ptr_eq(&runtime, &from_clone),
        "клоны разделяют runtime"
    );

    let rt = open_bsl::RuntimeShapes::seeded(Vec::new(), Vec::new(), None);
    let params = Arc::new(
        open_bsl::SerializedValueGraph::capture(
            &[open_bsl::Value::number_from_i64(7)],
            &rt,
            &open_bsl::GraphLimits::default(),
        )
        .expect("снимок"),
    );
    let snapshot = runtime
        .submit_by_name("Служебный.Эхо", params, None, None)
        .expect("задание принято");
    assert!(runtime.wait_terminal(&[snapshot.id], Some(std::time::Duration::from_secs(30))));
    assert_eq!(
        runtime.snapshot(snapshot.id).expect("снимок").state,
        open_bsl::JobStateDto::Completed
    );
    let done = runtime.snapshot(snapshot.id).expect("снимок");
    assert!(done.begin.is_some() && done.end.is_some(), "метки времени");
}

/// BSL-поверхность фоновых заданий: запуск, ожидание, снимок и свойства —
/// целиком из BSL-кода через голое имя `ФоновыеЗадания`.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn the_bsl_surface_runs_and_inspects_a_background_job() {
    let engine = Engine::builder()
        .common_module(
            "Служебный",
            "Функция Сложить(Знач а, Знач б) Экспорт\n\
                 Возврат а + б;\n\
             КонецФункции",
        )
        .build()
        .expect("движок собирается");
    let module = engine
        .compile_entry(
            "Параметры = Новый Массив;\n\
             Параметры.Добавить(2);\n\
             Параметры.Добавить(40);\n\
             Задание = ФоновыеЗадания.Выполнить(\"Служебный.Сложить\", Параметры, Неопределено, \"проба\");\n\
             Задание.ОжидатьЗавершенияВыполнения(30);\n\
             Свежий = ФоновыеЗадания.НайтиПоУникальномуИдентификатору(Задание.УникальныйИдентификатор);\n\
             Если Свежий.Состояние = СостояниеФоновогоЗадания.Завершено Тогда\n\
                 Сообщить(\"завершено\");\n\
             Иначе\n\
                 Сообщить(Свежий.Состояние);\n\
             КонецЕсли;\n\
             Сообщить(Свежий.ИмяМетода);\n\
             Сообщить(Свежий.Наименование);\n\
             Сообщить(Свежий.Параметры.Количество());\n\
             Сообщить(ЗначениеЗаполнено(Свежий.Начало) И ЗначениеЗаполнено(Свежий.Конец));",
        )
        .expect("entry компилируется");

    use std::io::Write;
    use std::rc::Rc;
    use std::sync::Mutex;
    #[derive(Clone, Default)]
    struct Capture(Rc<Mutex<Vec<u8>>>);
    impl Write for Capture {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let capture = Capture::default();
    let mut state = engine.state_builder().stdout(capture.clone()).build();
    state.run(&module).expect("прогон завершается");
    let bytes = capture.0.lock().unwrap().clone();
    assert_eq!(
        String::from_utf8(bytes).unwrap(),
        "завершено\nСлужебный.Сложить\nпроба\n2\nДа\n"
    );
}

/// Без каталога сервис не внедряется: голое имя отвечает ловимой ошибкой
/// возможности, а не паникой.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn without_a_catalog_the_manager_raises_a_catchable_error() {
    let engine = Engine::builder().build().expect("движок без каталога");
    let module = engine
        .compile(
            "Попытка\n\
                 М = ФоновыеЗадания;\n\
                 Сообщить(\"дожили\");\n\
             Исключение\n\
                 Сообщить(\"поймана\");\n\
             КонецПопытки;",
        )
        .expect("компилируется");
    let mut state = engine.new_state();
    state.run(&module).expect("ловимая ошибка перехвачена");
}
