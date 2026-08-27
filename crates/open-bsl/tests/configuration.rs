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
             Сообщить(Свежий.Ключ = \"\");\n\
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
        "завершено\nСлужебный.Сложить\nпроба\nДа\nДа\n"
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

/// Вложенное задание при пуле из ОДНОГО worker: родитель ждёт ребёнка —
/// helping-ожидание доводит ребёнка потоком родителя, deadlock нет.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn a_nested_job_completes_on_a_single_worker_pool() {
    let engine = Engine::builder()
        .common_module(
            "Служебный",
            "Функция Ребёнок(Знач х) Экспорт\n\
                 Возврат х * 2;\n\
             КонецФункции\n\
             Функция Родитель() Экспорт\n\
                 Параметры = Новый Массив;\n\
                 Параметры.Добавить(21);\n\
                 Дитя = ФоновыеЗадания.Выполнить(\"Служебный.Ребёнок\", Параметры);\n\
                 Дитя.ОжидатьЗавершенияВыполнения(30);\n\
                 Свежее = ФоновыеЗадания.НайтиПоУникальномуИдентификатору(Дитя.УникальныйИдентификатор);\n\
                 Если Свежее.Состояние <> СостояниеФоновогоЗадания.Завершено Тогда\n\
                     ВызватьИсключение \"ребёнок не завершился\";\n\
                 КонецЕсли;\n\
                 Возврат 1;\n\
             КонецФункции",
        )
        .background_jobs(open_bsl::jobs::BackgroundJobConfig {
            workers: Some(1),
            ..open_bsl::jobs::BackgroundJobConfig::default()
        })
        .build()
        .expect("движок собирается");
    let module = engine
        .compile_entry(
            "Родитель = ФоновыеЗадания.Выполнить(\"Служебный.Родитель\");\n\
             Родитель.ОжидатьЗавершенияВыполнения(60);\n\
             Свежий = ФоновыеЗадания.НайтиПоУникальномуИдентификатору(Родитель.УникальныйИдентификатор);\n\
             Если Свежий.Состояние = СостояниеФоновогоЗадания.Завершено Тогда\n\
                 Сообщить(\"родитель завершён\");\n\
             Иначе\n\
                 Сообщить(Свежий.ИнформацияОбОшибке.Подробно);\n\
             КонецЕсли;",
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
    assert_eq!(String::from_utf8(bytes).unwrap(), "родитель завершён\n");
}

/// Отмена running-задания: вечный цикл под «Попыткой» завершается
/// состоянием «Отменено», ветка `Исключение` не исполняется (измерено
/// JOB.CANCEL.CATCH), ИнформацияОбОшибке пуста; повторная отмена — no-op.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn cancelling_a_running_job_bypasses_the_exception_handler() {
    let engine = Engine::builder()
        .common_module(
            "Служебный",
            "Процедура Вечная() Экспорт\n\
                 Попытка\n\
                     Ч = 0;\n\
                     Пока Истина Цикл\n\
                         Ч = Ч + 1;\n\
                     КонецЦикла;\n\
                 Исключение\n\
                     Сообщить(\"перехвачено\");\n\
                 КонецПопытки;\n\
             КонецПроцедуры",
        )
        .build()
        .expect("движок собирается");
    let runtime = engine.job_runtime().expect("runtime");
    let rt = open_bsl::RuntimeShapes::seeded(Vec::new(), Vec::new(), None);
    let params = std::sync::Arc::new(
        open_bsl::SerializedValueGraph::capture(&[], &rt, &open_bsl::GraphLimits::default())
            .expect("снимок"),
    );
    let snapshot = runtime
        .submit_by_name("Служебный.Вечная", params.clone(), None, None)
        .expect("задание принято");
    // Дать заданию стартовать, затем отменить.
    std::thread::sleep(std::time::Duration::from_millis(50));
    runtime.cancel(snapshot.id);
    assert!(
        runtime.wait_terminal(&[snapshot.id], Some(std::time::Duration::from_secs(30))),
        "отменённое задание обязано дойти до terminal"
    );
    let done = runtime.snapshot(snapshot.id).expect("снимок");
    assert_eq!(done.state, open_bsl::JobStateDto::Canceled);
    assert!(done.error.is_none(), "отмена — не ошибка BSL");
    // Повторная отмена terminal-задания — no-op.
    runtime.cancel(snapshot.id);
    assert_eq!(
        runtime.snapshot(snapshot.id).expect("снимок").state,
        open_bsl::JobStateDto::Canceled
    );
}

/// Отмена queued-задания при отсутствии свободных workers: terminal сразу,
/// без старта.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn cancelling_a_queued_job_finishes_it_immediately() {
    let engine = Engine::builder()
        .common_module(
            "Служебный",
            "Процедура Долгая() Экспорт\n\
                 Ч = 0;\n\
                 Пока Истина Цикл\n\
                     Ч = Ч + 1;\n\
                 КонецЦикла;\n\
             КонецПроцедуры",
        )
        .background_jobs(open_bsl::jobs::BackgroundJobConfig {
            workers: Some(1),
            ..open_bsl::jobs::BackgroundJobConfig::default()
        })
        .build()
        .expect("движок собирается");
    let runtime = engine.job_runtime().expect("runtime");
    let rt = open_bsl::RuntimeShapes::seeded(Vec::new(), Vec::new(), None);
    let params = std::sync::Arc::new(
        open_bsl::SerializedValueGraph::capture(&[], &rt, &open_bsl::GraphLimits::default())
            .expect("снимок"),
    );
    let first = runtime
        .submit_by_name("Служебный.Долгая", params.clone(), None, None)
        .expect("первое принято");
    let second = runtime
        .submit_by_name("Служебный.Долгая", params, None, None)
        .expect("второе принято — в очередь");
    // Второе задание стоит в очереди за вечным первым: отмена завершает
    // его сразу.
    runtime.cancel(second.id);
    assert!(runtime.wait_terminal(&[second.id], Some(std::time::Duration::from_secs(10))));
    assert_eq!(
        runtime.snapshot(second.id).expect("снимок").state,
        open_bsl::JobStateDto::Canceled
    );
    runtime.cancel(first.id);
    assert!(runtime.wait_terminal(&[first.id], Some(std::time::Duration::from_secs(30))));
}

/// Явное завершение: вечное задание отменяется, потоки выходят до
/// deadline, повторный submit получает ловимую ошибку.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn shutdown_cancels_residents_and_rejects_new_submissions() {
    let engine = Engine::builder()
        .common_module(
            "Служебный",
            "Процедура Вечная() Экспорт\n\
                 Ч = 0;\n\
                 Пока Истина Цикл\n\
                     Ч = Ч + 1;\n\
                 КонецЦикла;\n\
             КонецПроцедуры",
        )
        .build()
        .expect("движок собирается");
    let runtime = engine.job_runtime().expect("runtime");
    let rt = open_bsl::RuntimeShapes::seeded(Vec::new(), Vec::new(), None);
    let params = std::sync::Arc::new(
        open_bsl::SerializedValueGraph::capture(&[], &rt, &open_bsl::GraphLimits::default())
            .expect("снимок"),
    );
    let snapshot = runtime
        .submit_by_name("Служебный.Вечная", params.clone(), None, None)
        .expect("задание принято");
    std::thread::sleep(std::time::Duration::from_millis(30));
    let report = runtime.shutdown(std::time::Duration::from_secs(30));
    assert_eq!(
        report.detached_workers, 0,
        "workers обязаны выйти до deadline"
    );
    assert_eq!(
        runtime.snapshot(snapshot.id).expect("снимок").state,
        open_bsl::JobStateDto::Canceled
    );
    assert!(
        runtime
            .submit_by_name("Служебный.Вечная", params, None, None)
            .is_err(),
        "закрытый runtime не принимает задания"
    );
}

/// Временное хранилище через границу задания: job читает адрес родителя
/// как Неопределено, его запись публикуется только на terminal, после
/// завершения родитель видит новое значение (измерено
/// JOB.TEMP.CLIENT_SERVER).
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn temp_storage_stages_job_writes_until_terminal() {
    let engine = Engine::builder()
        .common_module(
            "Служебный",
            "Процедура Записать(Знач Адрес) Экспорт\n\
                 Чужое = ПолучитьИзВременногоХранилища(Адрес);\n\
                 Если Чужое <> Неопределено Тогда\n\
                     ВызватьИсключение \"адрес родителя обязан читаться как Неопределено\";\n\
                 КонецЕсли;\n\
                 ПоместитьВоВременноеХранилище(\"из-задания\", Адрес);\n\
             КонецПроцедуры",
        )
        .build()
        .expect("движок собирается");
    let module = engine
        .compile_entry(
            "Адрес = ПоместитьВоВременноеХранилище(\"из-родителя\");\n\
             Параметры = Новый Массив;\n\
             Параметры.Добавить(Адрес);\n\
             Задание = ФоновыеЗадания.Выполнить(\"Служебный.Записать\", Параметры);\n\
             Задание.ОжидатьЗавершенияВыполнения(30);\n\
             Свежее = ФоновыеЗадания.НайтиПоУникальномуИдентификатору(Задание.УникальныйИдентификатор);\n\
             Если Свежее.Состояние <> СостояниеФоновогоЗадания.Завершено Тогда\n\
                 Сообщить(\"задание не завершилось\");\n\
             КонецЕсли;\n\
             Сообщить(ПолучитьИзВременногоХранилища(Адрес));",
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
    assert_eq!(String::from_utf8(bytes).unwrap(), "из-задания\n");
}

/// Локальная семантика хранилища: адрес возвращается и перечитывается,
/// повторная запись по адресу сохраняет адрес, удаление делает чтение
/// Неопределено, кривой адрес читается как Неопределено.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn temp_storage_local_round_trip_and_delete() {
    let engine = Engine::builder()
        .common_module(
            "Служебный",
            "Функция Пусто() Экспорт\n Возврат 0;\nКонецФункции",
        )
        .build()
        .expect("движок собирается");
    let module = engine
        .compile_entry(
            "Адрес = ПоместитьВоВременноеХранилище(41);\n\
             Адрес2 = ПоместитьВоВременноеХранилище(42, Адрес);\n\
             Сообщить(Адрес = Адрес2);\n\
             Сообщить(ПолучитьИзВременногоХранилища(Адрес));\n\
             УдалитьИзВременногоХранилища(Адрес);\n\
             Сообщить(ПолучитьИзВременногоХранилища(Адрес) = Неопределено);\n\
             Сообщить(ПолучитьИзВременногоХранилища(\"мусор\") = Неопределено);",
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
    assert_eq!(String::from_utf8(bytes).unwrap(), "Да\n42\nДа\nДа\n");
}

/// Сообщения пользователю: Сообщить внутри задания попадает в FIFO-историю,
/// читается через ПолучитьСообщенияПользователю (свойство Текст), а не в
/// stdout родителя; режим удаления забирает префикс.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn job_messages_are_recorded_and_drained() {
    let engine = Engine::builder()
        .common_module(
            "Служебный",
            "Процедура Говорит() Экспорт\n\
                 Сообщить(\"первое\");\n\
                 Сообщить(\"второе\");\n\
             КонецПроцедуры",
        )
        .build()
        .expect("движок собирается");
    let module = engine
        .compile_entry(
            "Задание = ФоновыеЗадания.Выполнить(\"Служебный.Говорит\");\n\
             Задание.ОжидатьЗавершенияВыполнения(30);\n\
             Сообщения = Задание.ПолучитьСообщенияПользователю();\n\
             Сообщить(Сообщения.Количество());\n\
             Для Каждого Сообщение Из Сообщения Цикл\n\
                 Сообщить(Сообщение.Текст);\n\
             КонецЦикла;",
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
        "2\nпервое\nвторое\n",
        "сообщения задания не должны утекать в stdout родителя"
    );
}

/// ИЗМЕРЕНО (JOB.EXECUTE.VALIDATION): лишние аргументы принимаются при
/// Выполнить, а задание завершается аварийно уже асинхронно.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn extra_arguments_fail_the_job_asynchronously() {
    let engine = Engine::builder()
        .common_module("Служебный", "Процедура Пустая() Экспорт\nКонецПроцедуры")
        .build()
        .expect("движок собирается");
    let runtime = engine.job_runtime().expect("runtime");
    let rt = open_bsl::RuntimeShapes::seeded(Vec::new(), Vec::new(), None);
    let values = [
        open_bsl::Value::number_from_i64(1),
        open_bsl::Value::number_from_i64(2),
    ];
    let params = std::sync::Arc::new(
        open_bsl::SerializedValueGraph::capture(&values, &rt, &open_bsl::GraphLimits::default())
            .expect("снимок"),
    );
    let snapshot = runtime
        .submit_by_name("Служебный.Пустая", params, None, None)
        .expect("submit обязан принять лишние аргументы — отказ асинхронный");
    assert!(runtime.wait_terminal(&[snapshot.id], Some(std::time::Duration::from_secs(30))));
    let done = runtime.snapshot(snapshot.id).expect("снимок");
    assert_eq!(done.state, open_bsl::JobStateDto::Failed);
    assert!(done.error.is_some(), "нужен текст об арности");
}

/// Документированная поверхность по синтакс-помощнику: отбор
/// ПолучитьФоновыеЗадания по ИмяМетода/Состояние/Ключ, менеджерное
/// ожидание возвращает МАССИВ обновлённых заданий, запись по удалённому
/// адресу хранилища — исключение. Ожидание — формой БЕЗ таймаута: она
/// детерминирована (до всех terminal), а форма с таймаутом возвращается
/// уже до первого изменения, и отбор по Завершено гонялся бы со вторым
/// заданием (падало под нагрузкой полного прогона workspace).
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn the_documented_manager_surface_works_end_to_end() {
    let engine = Engine::builder()
        .common_module(
            "Служебный",
            "Процедура Пустая() Экспорт\nКонецПроцедуры\n\
             Процедура Другая() Экспорт\nКонецПроцедуры",
        )
        .build()
        .expect("движок собирается");
    let module = engine
        .compile_entry(
            "Одно = ФоновыеЗадания.Выполнить(\"Служебный.Пустая\", Неопределено, \"К-1\");\n\
             Два = ФоновыеЗадания.Выполнить(\"Служебный.Другая\");\n\
             Пачка = Новый Массив;\n\
             Пачка.Добавить(Одно);\n\
             Пачка.Добавить(Два);\n\
             Свежие = ФоновыеЗадания.ОжидатьЗавершенияВыполнения(Пачка);\n\
             Сообщить(ТипЗнч(Свежие));\n\
             Сообщить(Свежие.Количество());\n\
             Отбор = Новый Структура(\"ИмяМетода\", \"Служебный.Пустая\");\n\
             Сообщить(ФоновыеЗадания.ПолучитьФоновыеЗадания(Отбор).Количество());\n\
             ОтборКлюч = Новый Структура(\"Ключ\", \"К-1\");\n\
             Сообщить(ФоновыеЗадания.ПолучитьФоновыеЗадания(ОтборКлюч).Количество());\n\
             ОтборСост = Новый Структура(\"Состояние\", СостояниеФоновогоЗадания.Завершено);\n\
             Сообщить(ФоновыеЗадания.ПолучитьФоновыеЗадания(ОтборСост).Количество() >= 2);\n\
             Адрес = ПоместитьВоВременноеХранилище(7);\n\
             УдалитьИзВременногоХранилища(Адрес);\n\
             Попытка\n\
                 ПоместитьВоВременноеХранилище(8, Адрес);\n\
                 Сообщить(\"запись прошла\");\n\
             Исключение\n\
                 Сообщить(\"запись отвергнута\");\n\
             КонецПопытки;",
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
        "Массив\n2\n1\n1\nДа\nзапись отвергнута\n"
    );
}

/// Задание, припаркованное на синхронном HTTP, освобождает свой worker:
/// пул из одного потока успевает довести соседнее задание до конца, пока
/// транспорт держит ответ, — синхронное ожидание внутри worker запрещено
/// планом этапа 5, и это его сквозная проверка.
#[cfg(all(not(target_arch = "wasm32"), feature = "http"))]
#[test]
fn a_parked_sync_http_job_frees_its_worker() {
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("порт");
    let port = listener.local_addr().expect("адрес").port();
    let (accepted_sender, accepted_receiver) = std::sync::mpsc::channel::<()>();
    let (release_sender, release_receiver) = std::sync::mpsc::channel::<()>();
    let server = std::thread::spawn(move || {
        let (mut socket, _) = listener.accept().expect("accept");
        let mut buffer = [0u8; 4096];
        let _ = socket.read(&mut buffer);
        let _ = accepted_sender.send(());
        let _ = release_receiver.recv();
        let _ = socket
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\nConnection: close\r\n\r\nready");
    });

    let engine = Engine::builder()
        .common_module(
            "Служебный",
            "Процедура Медленно(Знач Порт) Экспорт\n\
                 Соединение = Новый HTTPСоединение(\"127.0.0.1\", Порт);\n\
                 Ответ = Соединение.Получить(Новый HTTPЗапрос(\"/\"));\n\
                 Сообщить(Ответ.ПолучитьТелоКакСтроку());\n\
             КонецПроцедуры\n\
             Процедура Быстро() Экспорт\n\
                 Сообщить(\"быстро\");\n\
             КонецПроцедуры",
        )
        .background_jobs(open_bsl::jobs::BackgroundJobConfig {
            workers: Some(1),
            ..Default::default()
        })
        .build()
        .expect("движок собирается");
    let runtime = engine.job_runtime().expect("runtime");
    let rt = open_bsl::RuntimeShapes::seeded(Vec::new(), Vec::new(), None);
    let port_params = [open_bsl::Value::number_from_i64(i64::from(port))];
    let params = std::sync::Arc::new(
        open_bsl::SerializedValueGraph::capture(
            &port_params,
            &rt,
            &open_bsl::GraphLimits::default(),
        )
        .expect("снимок"),
    );
    let slow = runtime
        .submit_by_name("Служебный.Медленно", params, None, None)
        .expect("submit медленного");
    assert!(
        accepted_receiver
            .recv_timeout(std::time::Duration::from_secs(10))
            .is_ok(),
        "запрос обязан дойти до сервера"
    );

    let empty = std::sync::Arc::new(
        open_bsl::SerializedValueGraph::capture(&[], &rt, &open_bsl::GraphLimits::default())
            .expect("снимок"),
    );
    let fast = runtime
        .submit_by_name("Служебный.Быстро", empty, None, None)
        .expect("submit быстрого");
    // Единственный worker при припаркованном соседе доводит второе
    // задание: если бы синхронный HTTP блокировал поток, ожидание ниже
    // истекло бы.
    assert!(
        runtime.wait_terminal(&[fast.id], Some(std::time::Duration::from_secs(10))),
        "быстрое задание обязано завершиться, пока медленное припарковано"
    );
    assert_eq!(
        runtime.snapshot(fast.id).expect("снимок").state,
        open_bsl::JobStateDto::Completed
    );
    assert_eq!(
        runtime.snapshot(slow.id).expect("снимок").state,
        open_bsl::JobStateDto::Running,
        "медленное задание всё ещё ждёт транспорт"
    );

    release_sender.send(()).expect("сервер жив");
    assert!(
        runtime.wait_terminal(&[slow.id], Some(std::time::Duration::from_secs(10))),
        "после ответа транспорта задание обязано завершиться"
    );
    let done = runtime.snapshot(slow.id).expect("снимок");
    assert_eq!(done.state, open_bsl::JobStateDto::Completed);
    assert_eq!(done.messages.as_slice(), ["ready".to_string()]);
    server.join().expect("сервер завершился");
}

/// Отмена задания, припаркованного на синхронном HTTP: спящий без таймера
/// worker просыпается по отмене, poll возвращает Canceled, не дожидаясь
/// ответа транспорта, а сброс execution отменяет сам запрос.
#[cfg(all(not(target_arch = "wasm32"), feature = "http"))]
#[test]
fn cancelling_a_parked_sync_http_job_does_not_wait_for_the_transport() {
    use std::io::Read;
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("порт");
    let port = listener.local_addr().expect("адрес").port();
    let (accepted_sender, accepted_receiver) = std::sync::mpsc::channel::<()>();
    let (shutdown_sender, shutdown_receiver) = std::sync::mpsc::channel::<()>();
    let server = std::thread::spawn(move || {
        let (mut socket, _) = listener.accept().expect("accept");
        let mut buffer = [0u8; 4096];
        let _ = socket.read(&mut buffer);
        let _ = accepted_sender.send(());
        // Ответ намеренно не отправляется: отмена не должна его ждать.
        // Поток живёт до закрытия канала в конце теста.
        let _ = shutdown_receiver.recv();
    });

    let engine = Engine::builder()
        .common_module(
            "Служебный",
            "Процедура Висеть(Знач Порт) Экспорт\n\
                 Соединение = Новый HTTPСоединение(\"127.0.0.1\", Порт);\n\
                 Соединение.Получить(Новый HTTPЗапрос(\"/\"));\n\
             КонецПроцедуры",
        )
        .background_jobs(open_bsl::jobs::BackgroundJobConfig {
            workers: Some(1),
            ..Default::default()
        })
        .build()
        .expect("движок собирается");
    let runtime = engine.job_runtime().expect("runtime");
    let rt = open_bsl::RuntimeShapes::seeded(Vec::new(), Vec::new(), None);
    let port_params = [open_bsl::Value::number_from_i64(i64::from(port))];
    let params = std::sync::Arc::new(
        open_bsl::SerializedValueGraph::capture(
            &port_params,
            &rt,
            &open_bsl::GraphLimits::default(),
        )
        .expect("снимок"),
    );
    let hung = runtime
        .submit_by_name("Служебный.Висеть", params, None, None)
        .expect("submit");
    assert!(
        accepted_receiver
            .recv_timeout(std::time::Duration::from_secs(10))
            .is_ok(),
        "запрос обязан дойти до сервера"
    );

    runtime.cancel(hung.id);
    assert!(
        runtime.wait_terminal(&[hung.id], Some(std::time::Duration::from_secs(10))),
        "отмена не должна ждать ответа транспорта"
    );
    assert_eq!(
        runtime.snapshot(hung.id).expect("снимок").state,
        open_bsl::JobStateDto::Canceled
    );
    assert!(
        runtime.snapshot(hung.id).expect("снимок").error.is_none(),
        "отмена — не ошибка BSL"
    );
    drop(engine);
    drop(shutdown_sender);
    server.join().expect("сервер завершился");
}
