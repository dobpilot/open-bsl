use open_bsl::{
    ApplicationCompletionSink, ApplicationExit, ApplicationLauncher, ApplicationRequest,
    ApplicationResult, Engine,
};
use std::{
    io,
    sync::{Arc, Mutex},
};

#[derive(Debug, Clone, Default)]
struct Launcher(Arc<Mutex<Vec<ApplicationRequest>>>);

impl ApplicationLauncher for Launcher {
    fn submit(
        &self,
        request: ApplicationRequest,
        sink: Box<dyn ApplicationCompletionSink>,
    ) -> io::Result<()> {
        self.0.lock().unwrap().push(request);
        sink.complete(Ok(ApplicationResult::Exited(ApplicationExit {
            code: Some(17),
        })));
        Ok(())
    }
}

#[test]
fn async_launch_defaults_errors_and_dynamic_promises_share_the_outer_execution() {
    for optimizations in [
        bsl_compiler::Optimizations::default(),
        bsl_compiler::Optimizations::all(),
    ] {
        let engine = Engine::builder()
            .optimizations(optimizations)
            .build()
            .unwrap();
        let launcher = Launcher::default();
        let mut state = engine
            .state_builder()
            .application_launcher(launcher.clone())
            .build();
        let module = engine.compile(r#"
Асинх Процедура Проверить()
    П = RunAppAsync("app", "cwd", Истина);
    Если ТипЗнч(П) <> Тип("Обещание") Тогда ВызватьИсключение "promise"; КонецЕсли;
    Если Ждать П <> 17 Или Ждать П <> 17 Тогда ВызватьИсключение "repeat"; КонецЕсли;
    Если Ждать ЗапуститьПриложениеАсинх("app") <> Неопределено Тогда ВызватьИсключение "default"; КонецЕсли;
    Если Ждать RunAppAsync("app", , Истина) <> 17 Тогда ВызватьИсключение "omitted"; КонецЕсли;
    П = Вычислить("RunAppAsync(""app"", , Истина)");
    Если Ждать П <> 17 Тогда ВызватьИсключение "dynamic"; КонецЕсли;
    Выполнить("П = Вычислить(""RunAppAsync(""""app"""", , Истина)"");");
    Если Ждать П <> 17 Тогда ВызватьИсключение "nested"; КонецЕсли;
    Для Каждого Выражение Из СтрРазделить("RunAppAsync(Неопределено)|RunAppAsync(Null)|RunAppAsync(0)|RunAppAsync(""app"", Неопределено)|RunAppAsync(""app"", Null)|RunAppAsync(""app"", 0)|RunAppAsync(""app"", , Неопределено)|RunAppAsync(""app"", , Null)|RunAppAsync(""app"", , 0)|RunAppAsync(""app"", , 1)", "|") Цикл
        П = Вычислить(Выражение);
        ОшибкаБыла = Ложь;
        Попытка Р = Ждать П; Исключение ОшибкаБыла = Истина; КонецПопытки;
        Если Не ОшибкаБыла Тогда ВызватьИсключение "expected error"; КонецЕсли;
    КонецЦикла;
КонецПроцедуры
Проверить();
"#).unwrap();
        state.run(&module).unwrap();
        let requests = launcher.0.lock().unwrap();
        assert_eq!(requests.len(), 5);
        assert_eq!(requests[0].working_directory.as_deref(), Some("cwd"));
        assert!(requests[1..].iter().all(|r| r.working_directory.is_none()));
    }
}

#[test]
fn absent_launcher_fails_at_await_and_bad_arity_at_compile_time() {
    let engine = Engine::builder().build().unwrap();
    for source in [
        "П = RunAppAsync();",
        "П = RunAppAsync(\"app\", \"\", Истина, 0);",
    ] {
        assert!(engine.compile(source).is_err());
    }
    let module = engine
        .compile(
            r#"
Асинх Процедура Проверить()
    П = RunAppAsync("app");
    ОшибкаБыла = Ложь;
    Попытка Р = Ждать П; Исключение ОшибкаБыла = Истина; КонецПопытки;
    Если Не ОшибкаБыла Тогда ВызватьИсключение "host"; КонецЕсли;
КонецПроцедуры
Проверить();
"#,
        )
        .unwrap();
    engine.state_builder().build().run(&module).unwrap();
}

#[derive(Clone, Default)]
struct Deferred {
    sink: Arc<Mutex<Option<Box<dyn ApplicationCompletionSink>>>>,
    neighbor: Arc<std::sync::atomic::AtomicBool>,
}

impl std::fmt::Debug for Deferred {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Deferred")
    }
}

impl ApplicationLauncher for Deferred {
    fn submit(
        &self,
        request: ApplicationRequest,
        sink: Box<dyn ApplicationCompletionSink>,
    ) -> io::Result<()> {
        if matches!(&request.target, open_bsl::ApplicationTarget::Executable { program, .. } if program == "neighbor")
        {
            self.neighbor
                .store(true, std::sync::atomic::Ordering::SeqCst);
            sink.complete(Ok(ApplicationResult::Exited(ApplicationExit {
                code: Some(0),
            })));
        } else {
            *self.sink.lock().unwrap() = Some(sink);
        }
        Ok(())
    }
}

#[test]
fn a_failed_parked_dynamic_call_keeps_local_writes_and_propagates_the_error() {
    for nested in [false, true] {
        let fragment = "ЛокальнаяПробы = 9; ЗапуститьПриложение(\"app\", , Истина);";
        let fragment = if nested {
            format!("Выполнить(\"{}\");", fragment.replace('"', "\"\""))
        } else {
            fragment.into()
        };
        let source = format!(
            "Функция Проверить() ЛокальнаяПробы = 1; ОшибкаБыла = Ложь;
             Попытка Выполнить(\"{}\"); Исключение ОшибкаБыла = Истина; КонецПопытки;
             Если Не ОшибкаБыла Тогда ВызватьИсключение \"потеря ошибки\"; КонецЕсли;
             Возврат ЛокальнаяПробы; КонецФункции Возврат Проверить();",
            fragment.replace('"', "\"\"")
        );
        for optimizations in [
            bsl_compiler::Optimizations::default(),
            bsl_compiler::Optimizations::all(),
        ] {
            let engine = Engine::builder()
                .optimizations(optimizations)
                .build()
                .unwrap();
            let compiled = engine.compile(&source).unwrap();
            for bytecode in [false, true] {
                let module = if bytecode {
                    engine.load_bytecode(&compiled.bytecode().unwrap()).unwrap()
                } else {
                    engine.compile(&source).unwrap()
                };
                let launcher = Deferred::default();
                let mut state = engine
                    .state_builder()
                    .application_launcher(launcher.clone())
                    .build();
                let mut execution = state.start(&module).unwrap();
                for _ in 0..100 {
                    let poll = execution.poll(16).unwrap();
                    assert!(!matches!(poll, open_bsl::ExecutionPoll::Complete(_)));
                    if matches!(poll, open_bsl::ExecutionPoll::Waiting) {
                        break;
                    }
                }
                launcher
                    .sink
                    .lock()
                    .unwrap()
                    .take()
                    .expect("операция припаркована у host")
                    .complete(Err(io::ErrorKind::PermissionDenied.into()));
                let mut result = None;
                for _ in 0..100 {
                    if let open_bsl::ExecutionPoll::Complete(value) = execution.poll(16).unwrap() {
                        result = Some(value);
                        break;
                    }
                }
                assert_eq!(result, Some(open_bsl::Value::number_from_i64(9)));
            }
        }
    }
}

#[test]
fn application_promise_yields_to_neighbors_and_maps_deferred_results() {
    for (result, expected) in [
        (
            Ok(ApplicationResult::Exited(ApplicationExit {
                code: Some(17),
            })),
            "17",
        ),
        (
            Ok(ApplicationResult::Exited(ApplicationExit {
                code: Some(127),
            })),
            "127",
        ),
        (
            Err(io::Error::from(io::ErrorKind::NotFound)),
            "Неопределено",
        ),
        (Ok(ApplicationResult::Started), "error"),
        (Ok(ApplicationResult::DocumentOpened), "error"),
        (
            Ok(ApplicationResult::Exited(ApplicationExit { code: None })),
            "error",
        ),
        (
            Err(io::Error::from(io::ErrorKind::PermissionDenied)),
            "error",
        ),
    ] {
        let engine = Engine::builder().build().unwrap();
        let launcher = Deferred::default();
        let mut state = engine
            .state_builder()
            .application_launcher(launcher.clone())
            .build();
        let assertion = if expected == "error" {
            "Если Не ОшибкаБыла Тогда ВызватьИсключение \"expected error\"; КонецЕсли;".into()
        } else {
            format!(
                "Если ОшибкаБыла Или Р <> {expected} Тогда ВызватьИсключение \"result\"; КонецЕсли;"
            )
        };
        let module = engine
            .compile(&format!(
                r#"
Асинх Процедура Длинная()
    П = Вычислить("RunAppAsync(""app"", , Истина)");
    Для Повтор = 1 По 2 Цикл
        ОшибкаБыла = Ложь;
        Попытка Р = Ждать П; Исключение ОшибкаБыла = Истина; КонецПопытки;
        {assertion}
    КонецЦикла;
КонецПроцедуры
Асинх Процедура Соседняя()
    Р = Ждать RunAppAsync("neighbor", , Истина);
КонецПроцедуры
Длинная();
Соседняя();
"#
            ))
            .unwrap();
        let mut execution = state.start(&module).unwrap();
        for _ in 0..10 {
            assert!(!matches!(
                execution.poll(16).unwrap(),
                open_bsl::ExecutionPoll::Complete(_)
            ));
            if launcher.neighbor.load(std::sync::atomic::Ordering::SeqCst) {
                break;
            }
        }
        assert!(
            launcher.neighbor.load(std::sync::atomic::Ordering::SeqCst),
            "ожидание приложения заморозило соседа"
        );
        launcher
            .sink
            .lock()
            .unwrap()
            .take()
            .unwrap()
            .complete(result);
        let mut complete = false;
        for _ in 0..100 {
            if matches!(
                execution.poll(16).unwrap(),
                open_bsl::ExecutionPoll::Complete(_)
            ) {
                complete = true;
                break;
            }
        }
        assert!(complete, "доставленное завершение не пробудило обещание");
    }
}

#[derive(Debug)]
struct Refused;

impl ApplicationLauncher for Refused {
    fn submit(
        &self,
        _request: ApplicationRequest,
        _sink: Box<dyn ApplicationCompletionSink>,
    ) -> io::Result<()> {
        Err(io::Error::other("secret launch command"))
    }
}

#[test]
fn submit_refusal_is_an_await_error_and_does_not_leak_host_text() {
    let engine = Engine::builder().build().unwrap();
    let mut state = engine.state_builder().application_launcher(Refused).build();
    let module = engine
        .compile(
            r#"
Асинх Процедура Проверить()
    П = RunAppAsync("app", , Истина);
    ОшибкаБыла = Ложь;
    Попытка Р = Ждать П; Исключение
        ОшибкаБыла = Истина;
        ОшибкаЗапуска = ИнформацияОбОшибке();
        Если СтрНайти(ОшибкаЗапуска.Описание, "secret") > 0 Тогда ВызватьИсключение "leak"; КонецЕсли;
    КонецПопытки;
    Если Не ОшибкаБыла Тогда ВызватьИсключение "submit"; КонецЕсли;
КонецПроцедуры
Проверить();
"#,
        )
        .unwrap();
    state.run(&module).unwrap();
}
