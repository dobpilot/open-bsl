use open_bsl::{
    ApplicationCompletionSink, ApplicationExit, ApplicationLauncher, ApplicationRequest,
    ApplicationResult, Engine, Value,
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
fn runapp_writes_local_module_and_aliased_parameters_with_all_optimizations() {
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
        let module = engine
            .compile(
                r#"
Перем КодМодуля;
Процедура Вызвать(КодСсылки)
    ЛокальныйКод = 12345;
    RunApp("'/host/app name' 'literal $HOME *' ''", "host cwd", Истина, ЛокальныйКод);
    Если ЛокальныйКод <> 17 Тогда ВызватьИсключение "local"; КонецЕсли;
    ЗапуститьПриложение("app",, Истина, КодСсылки);
    RunApp("app",, Истина, КодМодуля);
КонецПроцедуры
КодМодуля = 12345;
Код = 12345;
Вызвать(Код);
Если Код <> 17 Или КодМодуля <> 17 Тогда ВызватьИсключение "reference"; КонецЕсли;
RunApp("app",, Ложь, Код);
Если Код <> Неопределено Тогда ВызватьИсключение "no wait"; КонецЕсли;
Код = 12345;
RunApp("app",,, Код);
Если Код <> Неопределено Тогда ВызватьИсключение "omitted"; КонецЕсли;
RunApp("app",, Истина, 42);
"#,
            )
            .unwrap();
        state.run(&module).unwrap();
        let requests = launcher.0.lock().unwrap();
        assert_eq!(requests.len(), 6);
        assert_eq!(requests[0].working_directory.as_deref(), Some("host cwd"));
        let open_bsl::ApplicationTarget::Executable { program, arguments } = &requests[0].target
        else {
            panic!("document")
        };
        assert_eq!(program, "/host/app name");
        assert_eq!(
            arguments
                .iter()
                .map(open_bsl::SecretString::expose)
                .collect::<Vec<_>>(),
            ["literal $HOME *", ""]
        );
    }
}

#[test]
fn runapp_rejects_explicit_invalid_defaults_without_launch_or_output_mutation() {
    let engine = Engine::builder().build().unwrap();
    let launcher = Launcher::default();
    let mut state = engine
        .state_builder()
        .application_launcher(launcher.clone())
        .build();
    for arguments in [
        "\"app\",, Неопределено",
        "\"app\",, Null",
        "\"app\",, 0",
        "\"app\",, 1",
        "\"app\", Неопределено, Истина",
        "Неопределено,, Истина",
    ] {
        state
            .exec(&format!(
                r#"
Код = 12345;
БылаОшибка = Ложь;
Попытка RunApp({arguments}, Код); Исключение БылаОшибка = Истина; КонецПопытки;
Если Не БылаОшибка Или Код <> 12345 Тогда ВызватьИсключение "validation"; КонецЕсли;
"#
            ))
            .unwrap();
    }
    assert!(launcher.0.lock().unwrap().is_empty());
    assert!(engine.compile("Код = RunApp(\"app\");").is_err());
    assert!(engine.compile("RunApp();").is_err());
    assert!(engine.compile("RunApp(1,2,3,4,5);").is_err());
    assert!(
        engine
            .state_builder()
            .build()
            .exec("RunApp(\"app\");")
            .is_err()
    );
    assert_eq!(state.eval("42").unwrap(), Value::number_from_i64(42));
}

#[derive(Clone)]
struct Controlled {
    sink: Arc<Mutex<Option<Box<dyn ApplicationCompletionSink>>>>,
    started: std::sync::mpsc::Sender<()>,
}

impl std::fmt::Debug for Controlled {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Controlled")
    }
}

impl ApplicationLauncher for Controlled {
    fn submit(
        &self,
        _: ApplicationRequest,
        sink: Box<dyn ApplicationCompletionSink>,
    ) -> io::Result<()> {
        *self.sink.lock().unwrap() = Some(sink);
        self.started.send(()).unwrap();
        Ok(())
    }
}

fn controlled() -> (Controlled, std::sync::mpsc::Receiver<()>) {
    let (started, received) = std::sync::mpsc::channel();
    (
        Controlled {
            sink: Arc::new(Mutex::new(None)),
            started,
        },
        received,
    )
}

fn execute_wrapped(mut source: String, depth: usize) -> String {
    for _ in 0..depth {
        source = format!("Выполнить(\"{}\");", source.replace('"', "\"\""));
    }
    source
}

#[test]
fn nested_dynamic_waits_restore_locals_aliases_and_module_state_on_success_and_error() {
    for depth in [1, 2, 5] {
        for fails in [false, true] {
            let engine = Engine::builder().build().unwrap();
            let (launcher, _started) = controlled();
            let mut state = engine
                .state_builder()
                .application_launcher(launcher.clone())
                .build();
            let inner = execute_wrapped(
                "МодульноеЗначение = 9; RunApp(\"app\",, Истина, ЛокальныйКод);".into(),
                depth,
            );
            let expected = if fails { 12345 } else { 17 };
            let module = engine.compile(&format!(r#"
Перем МодульноеЗначение;
Процедура Проверить(КодСсылки)
    ЛокальныйКод = 12345;
    Попытка {inner} Исключение КонецПопытки;
    КодСсылки = ЛокальныйКод;
КонецПроцедуры
МодульноеЗначение = 0;
Результат = 12345;
Проверить(Результат);
Если Результат <> {expected} Или МодульноеЗначение <> 9 Тогда ВызватьИсключение "copyback"; КонецЕсли;
"#)).unwrap();
            let mut execution = state.start(&module).unwrap();
            assert_eq!(
                execution.poll(16).unwrap(),
                open_bsl::ExecutionPoll::Waiting
            );
            assert_eq!(
                execution.poll(16).unwrap(),
                open_bsl::ExecutionPoll::Waiting
            );
            launcher
                .sink
                .lock()
                .unwrap()
                .take()
                .unwrap()
                .complete(if fails {
                    Err(io::Error::other("probe"))
                } else {
                    Ok(ApplicationResult::Exited(ApplicationExit {
                        code: Some(17),
                    }))
                });
            assert!(matches!(
                execution.poll(16).unwrap(),
                open_bsl::ExecutionPoll::Complete(_)
            ));
        }
    }
}

#[test]
fn eval_can_park_in_a_module_function_and_return_value_and_reference_updates() {
    let engine = Engine::builder().build().unwrap();
    let (launcher, _started) = controlled();
    let mut state = engine
        .state_builder()
        .application_launcher(launcher.clone())
        .build();
    let module = engine
        .compile(
            r#"
Функция Дождаться(Код)
    RunApp("app",, Истина, Код);
    Возврат Код + 1;
КонецФункции
Код = 12345;
Ответ = Вычислить("Вычислить(""Дождаться(Код)"")");
Если Код <> 17 Или Ответ <> 18 Тогда ВызватьИсключение "eval"; КонецЕсли;
"#,
        )
        .unwrap();
    let mut execution = state.start(&module).unwrap();
    assert_eq!(
        execution.poll(16).unwrap(),
        open_bsl::ExecutionPoll::Waiting
    );
    launcher
        .sink
        .lock()
        .unwrap()
        .take()
        .unwrap()
        .complete(Ok(ApplicationResult::Exited(ApplicationExit {
            code: Some(17),
        })));
    assert!(matches!(
        execution.poll(16).unwrap(),
        open_bsl::ExecutionPoll::Complete(_)
    ));
}

#[test]
fn a_parked_dynamic_call_freezes_other_tasks_of_the_same_execution() {
    let engine = Engine::builder().build().unwrap();
    let (launcher, _started) = controlled();
    let mut state = engine
        .state_builder()
        .application_launcher(launcher.clone())
        .safe_points_per_quantum(1)
        .build();
    let module = engine.compile(r#"
Перем Маркер;
Асинх Процедура Сосед()
    Сумма = 0;
    Для Номер = 1 По 100 Цикл Сумма = Сумма + Номер; КонецЦикла;
    Маркер = 1;
КонецПроцедуры
Маркер = 0;
Сосед();
Выполнить("Код = 0; RunApp(""app"",, Истина, Код); Если Маркер <> 0 Тогда ВызватьИсключение ""not frozen""; КонецЕсли;");
"#).unwrap();
    let mut execution = state.start(&module).unwrap();
    assert_eq!(
        execution.poll(16).unwrap(),
        open_bsl::ExecutionPoll::Waiting
    );
    for _ in 0..3 {
        assert_eq!(
            execution.poll(16).unwrap(),
            open_bsl::ExecutionPoll::Waiting
        );
    }
    launcher
        .sink
        .lock()
        .unwrap()
        .take()
        .unwrap()
        .complete(Ok(ApplicationResult::Exited(ApplicationExit {
            code: Some(17),
        })));
    assert!(matches!(
        execution.poll(16).unwrap(),
        open_bsl::ExecutionPoll::Complete(_)
    ));
}

#[test]
fn runapp_evaluates_every_argument_once_in_source_order() {
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
        state
            .exec(
                r#"
Перем Порядок;
Функция Аргумент(Знач Номер, Знач Значение)
    Порядок = Порядок * 10 + Номер;
    Возврат Значение;
КонецФункции
Порядок = 0;
RunApp(Аргумент(1, "app"), Аргумент(2, "cwd"), Аргумент(3, Истина), Аргумент(4, 42));
Если Порядок <> 1234 Тогда ВызватьИсключение "argument order"; КонецЕсли;
"#,
            )
            .unwrap();
        assert_eq!(launcher.0.lock().unwrap().len(), 1);
    }
}

#[test]
fn runapp_delivers_deferred_results_and_preserves_output_on_errors() {
    for optimizations in [
        bsl_compiler::Optimizations::default(),
        bsl_compiler::Optimizations::all(),
    ] {
        for (outcome, expected) in [
            (
                Ok(ApplicationResult::Exited(ApplicationExit {
                    code: Some(17),
                })),
                17,
            ),
            (Err(io::Error::from(io::ErrorKind::NotFound)), 127),
            (Err(io::Error::other("secret command")), 12345),
            (Ok(ApplicationResult::Started), 12345),
            (Ok(ApplicationResult::DocumentOpened), 12345),
            (
                Ok(ApplicationResult::Exited(ApplicationExit { code: None })),
                12345,
            ),
        ] {
            let engine = Engine::builder()
                .optimizations(optimizations)
                .build()
                .unwrap();
            let (launcher, _started) = controlled();
            let mut state = engine
                .state_builder()
                .application_launcher(launcher.clone())
                .build();
            let module = engine
                .compile(&format!(
                    r#"
Код = 12345;
Попытка RunApp("app",, Истина, Код); Исключение КонецПопытки;
Если Код <> {expected} Тогда ВызватьИсключение "output"; КонецЕсли;
"#
                ))
                .unwrap();
            let mut execution = state.start(&module).unwrap();
            assert_eq!(
                execution.poll(16).unwrap(),
                open_bsl::ExecutionPoll::Waiting
            );
            launcher
                .sink
                .lock()
                .unwrap()
                .take()
                .unwrap()
                .complete(outcome);
            assert!(matches!(
                execution.poll(16).unwrap(),
                open_bsl::ExecutionPoll::Complete(_)
            ));
        }
    }
}

#[test]
fn runapp_without_wait_and_dropped_waiter_do_not_own_the_application() {
    let engine = Engine::builder().build().unwrap();
    for (wait, depth) in [false, true]
        .into_iter()
        .flat_map(|wait| [0, 3].map(move |depth| (wait, depth)))
    {
        let (launcher, _started) = controlled();
        let mut state = engine
            .state_builder()
            .application_launcher(launcher.clone())
            .build();
        let module = engine
            .compile(&execute_wrapped(
                format!(
                    "RunApp(\"app\",, {});",
                    if wait { "Истина" } else { "Ложь" }
                ),
                depth,
            ))
            .unwrap();
        let mut execution = state.start(&module).unwrap();
        let result = execution.poll(16).unwrap();
        // Без ожидания приложения всё равно нужно подтверждение запуска.
        assert_eq!(result, open_bsl::ExecutionPoll::Waiting);
        drop(execution);
        drop(state);
        launcher
            .sink
            .lock()
            .unwrap()
            .take()
            .unwrap()
            .complete(Ok(ApplicationResult::Exited(ApplicationExit {
                code: Some(17),
            })));
        assert_eq!(
            engine.state_builder().build().eval("42").unwrap(),
            Value::number_from_i64(42)
        );
    }
}

#[test]
fn runapp_writes_imported_variables_and_dynamic_output() {
    for optimizations in [
        bsl_compiler::Optimizations::default(),
        bsl_compiler::Optimizations::all(),
    ] {
        let engine = Engine::builder()
            .optimizations(optimizations)
            .common_module("Данные", "Перем Код Экспорт;")
            .build()
            .unwrap();
        let mut state = engine
            .state_builder()
            .application_launcher(Launcher::default())
            .build();
        let module = engine
            .compile_entry(
                r#"
Данные.Код = 12345;
RunApp("app",, Истина, Данные.Код);
Если Данные.Код <> 17 Тогда ВызватьИсключение "import"; КонецЕсли;
Код = 12345;
Выполнить("RunApp(""app"",, Истина, Код);");
Если Код <> 17 Тогда ВызватьИсключение "dynamic"; КонецЕсли;
"#,
            )
            .unwrap();
        state.run(&module).unwrap();
    }
}

#[test]
fn launch_confirmation_parks_without_waiting_for_application_exit() {
    for depth in [0, 3] {
        for result in [
            ApplicationResult::Started,
            ApplicationResult::DocumentOpened,
        ] {
            let engine = Engine::builder().build().unwrap();
            let (launcher, _started) = controlled();
            let mut state = engine
                .state_builder()
                .application_launcher(launcher.clone())
                .build();
            let source = execute_wrapped("Код = 123; RunApp(\"document\",, Ложь, Код); Если Код <> Неопределено Тогда ВызватьИсключение \"confirmation\"; КонецЕсли;".into(), depth);
            let module = engine.compile(&source).unwrap();
            let mut execution = state.start(&module).unwrap();
            assert_eq!(
                execution.poll(16).unwrap(),
                open_bsl::ExecutionPoll::Waiting
            );
            launcher
                .sink
                .lock()
                .unwrap()
                .take()
                .unwrap()
                .complete(Ok(result));
            assert!(matches!(
                execution.poll(16).unwrap(),
                open_bsl::ExecutionPoll::Complete(_)
            ));
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn runapp_wait_frees_a_single_worker_and_survives_cancellation_and_shutdown() {
    use open_bsl::jobs::{BackgroundJobConfig, BackgroundStateFactory};
    use std::time::Duration;
    struct Profile(Controlled);
    impl BackgroundStateFactory for Profile {
        fn configure(
            &self,
            builder: open_bsl::StateBuilder,
        ) -> Result<open_bsl::StateBuilder, String> {
            Ok(builder.application_launcher(self.0.clone()))
        }
    }
    for (action, depth, wait) in ["complete", "cancel", "shutdown"]
        .into_iter()
        .flat_map(|action| [0, 1, 3].map(move |depth| (action, depth)))
        .flat_map(|(action, depth)| [false, true].map(move |wait| (action, depth, wait)))
    {
        let (launcher, started) = controlled();
        let mut builder = Engine::builder()
            .background_jobs(BackgroundJobConfig {
                workers: Some(1),
                ..Default::default()
            })
            .common_module(
                "Работы",
                &r#"
Процедура Длинная() Экспорт
    Код = 12345;
    RunApp("app",, Истина, Код);
    Если Код <> 17 Тогда ВызватьИсключение "exit"; КонецЕсли;
КонецПроцедуры
Процедура Короткая() Экспорт
КонецПроцедуры
"#
                .replace(
                    "RunApp(\"app\",, Истина, Код);",
                    &execute_wrapped(
                        format!(
                            "RunApp(\"app\",, {}, Код);",
                            if wait { "Истина" } else { "Ложь" }
                        ),
                        depth,
                    ),
                )
                .replace(
                    "Код <> 17",
                    if wait {
                        "Код <> 17"
                    } else {
                        "Код <> Неопределено"
                    },
                ),
            );
        let profile = builder.register_host_profile(Arc::new(Profile(launcher.clone())));
        let engine = builder.build().unwrap();
        let runtime = engine.job_runtime().unwrap();
        let mut state = engine
            .state_builder()
            .host_profile(profile)
            .unwrap()
            .build();
        state
            .run(
                &engine
                    .compile_entry("ФоновыеЗадания.Выполнить(\"Работы.Длинная\");")
                    .unwrap(),
            )
            .unwrap();
        started.recv_timeout(Duration::from_secs(5)).unwrap();
        let slow = runtime
            .snapshots()
            .into_iter()
            .find(|job| job.method_name == "Работы.Длинная")
            .unwrap();
        state
            .run(
                &engine
                    .compile_entry("ФоновыеЗадания.Выполнить(\"Работы.Короткая\");")
                    .unwrap(),
            )
            .unwrap();
        let quick = runtime
            .snapshots()
            .into_iter()
            .find(|job| job.method_name == "Работы.Короткая")
            .unwrap();
        let quick_done = runtime
            .wait_terminal(&[quick.id], Some(Duration::from_secs(2)))
            .unwrap();
        if action == "cancel" {
            runtime.cancel(slow.id).unwrap();
        }
        if action == "shutdown" {
            assert_eq!(runtime.shutdown(Duration::from_secs(2)).detached_workers, 0);
        }
        let canceled = if action == "shutdown" {
            runtime.snapshot(slow.id).unwrap().state == open_bsl::JobStateDto::Canceled
        } else {
            action != "complete"
                && runtime
                    .wait_terminal(&[slow.id], Some(Duration::from_secs(2)))
                    .unwrap()
        };
        launcher
            .sink
            .lock()
            .unwrap()
            .take()
            .unwrap()
            .complete(Ok(ApplicationResult::Exited(ApplicationExit {
                code: Some(17),
            })));
        assert!(quick_done, "единственный worker занят ожиданием приложения");
        assert_eq!(
            runtime.snapshot(quick.id).unwrap().state,
            open_bsl::JobStateDto::Completed
        );
        assert!(
            action == "shutdown"
                || runtime
                    .wait_terminal(&[slow.id], Some(Duration::from_secs(5)))
                    .unwrap()
        );
        if action == "complete" {
            assert_eq!(
                runtime.snapshot(slow.id).unwrap().state,
                open_bsl::JobStateDto::Completed
            );
        } else {
            assert!(canceled);
            assert_eq!(
                runtime.snapshot(slow.id).unwrap().state,
                open_bsl::JobStateDto::Canceled
            );
        }
    }
}
