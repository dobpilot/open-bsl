//! Проверки парковки приложения, независимые от BSL-семантики RunApp.

use super::*;
use bsl_rt::{
    ApplicationCompletionSink, ApplicationExit, ApplicationLauncher, ApplicationRequest,
    ApplicationResult,
};
use std::io;
use std::sync::Mutex;

#[derive(Default)]
struct Launcher {
    sink: Mutex<Option<Box<dyn ApplicationCompletionSink>>>,
    immediate: bool,
    reject: bool,
}

impl std::fmt::Debug for Launcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Launcher")
    }
}

impl ApplicationLauncher for Launcher {
    fn submit(
        &self,
        request: ApplicationRequest,
        sink: Box<dyn ApplicationCompletionSink>,
    ) -> io::Result<()> {
        assert_eq!(request.working_directory.as_deref(), Some("host-cwd"));
        if self.reject {
            return Err(io::Error::from(io::ErrorKind::PermissionDenied));
        }
        if self.immediate {
            sink.complete(Ok(ApplicationResult::Exited(ApplicationExit {
                code: Some(17),
            })));
        } else {
            *self.sink.lock().unwrap() = Some(sink);
        }
        Ok(())
    }
}

thread_local! {
    static MAPPER_CALLS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

fn map_result(
    result: io::Result<ApplicationResult>,
    _: &mut bsl_rt::RuntimeShapes,
) -> Result<BslValue, RtError> {
    MAPPER_CALLS.set(MAPPER_CALLS.get() + 1);
    let ApplicationResult::Exited(exit) =
        result.map_err(|_| RtError::IoError("completion".into()))?
    else {
        return Err(RtError::IoError("not an exit".into()));
    };
    // Политика только тестового обработчика: код без значения не подменяется 0.
    Ok(exit.code.map_or(BslValue::Undefined, |code| {
        BslValue::number_from_i64(i64::from(code))
    }))
}

fn pending(launcher: Arc<Launcher>) -> bsl_rt::PendingHostCall {
    bsl_rt::PendingHostCall::ApplicationSync {
        launcher,
        request: ApplicationRequest {
            target: bsl_rt::ApplicationTarget::from_unix_command_line("test-app 'literal *'")
                .unwrap(),
            wait_for_exit: true,
            working_directory: Some("host-cwd".into()),
        },
        mapper: map_result,
        error_mapper: |_| RtError::IoError("acceptance".into()),
    }
}

fn state() -> AsyncState {
    let mut state = AsyncState::new(
        Task {
            frames: Vec::new(),
            stack: Vec::new(),
            current_exception: None,
            completion: TaskCompletion::Root,
            quantum_remaining: 1,
        },
        1,
    );
    state.ready.clear();
    state
}

#[test]
fn application_completion_wakes_and_maps_only_on_the_execution_thread() {
    for outcome in [Ok(Some(17)), Ok(None), Err(())] {
        MAPPER_CALLS.set(0);
        let launcher = Arc::new(Launcher::default());
        let mut state = state();
        let wakes = Arc::new(AtomicU64::new(0));
        let counter = wakes.clone();
        state.host_waker = Some(Arc::new(move || {
            counter.fetch_add(1, Ordering::SeqCst);
        }));
        state
            .begin_sync_host_call(3, 7, pending(launcher.clone()))
            .unwrap();
        let wait = state.sync_wait.as_ref().unwrap();
        assert_eq!((wait.task_id, wait.dst), (3, 7));
        let index = wait.promise_id.get() as usize;
        let mut shapes = bsl_rt::RuntimeShapes::seeded(Vec::new(), Vec::new(), None);
        assert_eq!(state.drain_completions(1, false, &mut shapes).unwrap(), 0);
        assert!(
            matches!(&state.promises[index], PromiseState::Pending { waiters } if waiters.iter().copied().eq([3]))
        );
        let sink = launcher.sink.lock().unwrap().take().unwrap();
        std::thread::spawn(move || {
            sink.complete(
                outcome
                    .map(|code| ApplicationResult::Exited(ApplicationExit { code }))
                    .map_err(|_| io::Error::from(io::ErrorKind::Other)),
            );
            assert_eq!(MAPPER_CALLS.get(), 0);
        })
        .join()
        .unwrap();
        assert_eq!(wakes.load(Ordering::SeqCst), 1);
        assert_eq!(MAPPER_CALLS.get(), 0);
        assert_eq!(state.drain_completions(1, false, &mut shapes).unwrap(), 1);
        assert_eq!(MAPPER_CALLS.get(), 1);
        assert!(!state.has_pending_host_promises());
        let PromiseState::Ready(result) = &state.promises[index] else {
            panic!("not ready")
        };
        match outcome {
            Ok(Some(code)) => assert_eq!(
                result.as_ref().unwrap(),
                &BslValue::number_from_i64(i64::from(code))
            ),
            Ok(None) => assert_eq!(result.as_ref().unwrap(), &BslValue::Undefined),
            Err(()) => {
                assert!(matches!(result, Err(RtError::IoError(message)) if message == "completion"))
            }
        }
        assert_eq!(state.ready.len(), 1);
        assert!(matches!(state.ready.front(), Some(ReadyEvent::Task(3))));
    }
}

#[test]
fn immediate_application_completion_is_registered_before_delivery() {
    let launcher = Arc::new(Launcher {
        immediate: true,
        ..Default::default()
    });
    let mut state = state();
    state.begin_sync_host_call(0, 0, pending(launcher)).unwrap();
    assert!(state.sync_wait.is_some());
    let mut shapes = bsl_rt::RuntimeShapes::seeded(Vec::new(), Vec::new(), None);
    assert_eq!(state.drain_completions(1, false, &mut shapes).unwrap(), 1);
    assert!(
        matches!(&state.promises[0], PromiseState::Ready(Ok(value)) if *value == BslValue::number_from_i64(17))
    );
}

#[test]
fn refused_application_has_no_promise_or_wait() {
    let launcher = Arc::new(Launcher {
        reject: true,
        ..Default::default()
    });
    let mut state = state();
    assert!(
        matches!(state.begin_sync_host_call(0, 0, pending(launcher)), Err(RtError::IoError(message)) if message == "acceptance")
    );
    assert!(state.promises.is_empty());
    assert!(!state.has_pending_host_promises());
    assert!(state.sync_wait.is_none());
}

#[test]
fn late_application_completion_after_drop_cannot_resolve_a_new_execution() {
    let old = Arc::new(Launcher::default());
    let mut state = state();
    state
        .begin_sync_host_call(0, 0, pending(old.clone()))
        .unwrap();
    let old_token = state.token;
    drop(state);
    let current = Arc::new(Launcher::default());
    let mut state = self::state();
    state.begin_sync_host_call(0, 0, pending(current)).unwrap();
    assert_ne!(old_token, state.token);
    old.sink
        .lock()
        .unwrap()
        .take()
        .unwrap()
        .complete(Ok(ApplicationResult::Exited(ApplicationExit {
            code: Some(17),
        })));
    let mut shapes = bsl_rt::RuntimeShapes::seeded(Vec::new(), Vec::new(), None);
    assert_eq!(state.drain_completions(1, false, &mut shapes).unwrap(), 0);
    assert!(matches!(&state.promises[0], PromiseState::Pending { .. }));
}

#[derive(Debug)]
struct Probe(Arc<Launcher>);

impl bsl_rt::ObjectProtocol for Probe {
    fn type_descriptor(&self) -> &'static bsl_rt::TypeDescriptor {
        static TYPE: bsl_rt::TypeDescriptor =
            bsl_rt::TypeDescriptor::new("test", "ApplicationProbe");
        &TYPE
    }

    fn method_table(&self) -> &'static [bsl_rt::MethodDescriptor] {
        const METHODS: &[bsl_rt::MethodDescriptor] = &[bsl_rt::MethodDescriptor::suspending(
            &["Observe"],
            bsl_rt::Arity::exact(0),
            |receiver, _, _| {
                let probe = bsl_rt::receiver_of::<Probe>(receiver, "Observe")?;
                Ok(bsl_rt::CallOutcome::Pending(pending(probe.0.clone())))
            },
        )];
        METHODS
    }
}

#[test]
fn application_method_parks_then_resumes_the_real_vm_or_unwinds_the_error() {
    for fails in [false, true] {
        let program = crate::tests::compile_module(
            r#"
Перем Приложение;
Попытка
    Код = Приложение.Observe();
    Возврат Код + 1;
Исключение
    Возврат 42;
КонецПопытки;
"#,
        );
        let mut builder = bsl_rt::RuntimeBuilder::new();
        builder.register(bsl_rt::core_library());
        let registry = builder.build().unwrap();
        let mut env = bsl_rt::HostEnv::process();
        let mut execution =
            crate::ProgramExecution::start_with_registry(&program, &registry, &env).unwrap();
        let launcher = Arc::new(Launcher::default());
        execution.module_state.slots[0] = BslValue::new_object(Probe(launcher.clone()));
        let mut dynamic = crate::tests::TestDynamic::bare();
        let mut poll = || {
            execution.poll_with_registry_and_io(
                &program,
                &registry,
                &mut Vec::new(),
                &mut Vec::new(),
                &mut dynamic,
                &mut env,
                1,
            )
        };
        assert!(matches!(poll().unwrap(), crate::ProgramPoll::Waiting));
        assert!(matches!(poll().unwrap(), crate::ProgramPoll::Waiting));
        launcher
            .sink
            .lock()
            .unwrap()
            .take()
            .unwrap()
            .complete(if fails {
                Err(io::Error::from(io::ErrorKind::Other))
            } else {
                Ok(ApplicationResult::Exited(ApplicationExit {
                    code: Some(17),
                }))
            });
        assert!(
            matches!(poll().unwrap(), crate::ProgramPoll::Complete(value, _) if value == BslValue::number_from_i64(if fails { 42 } else { 18 }))
        );
    }
}

#[test]
fn a_detached_parked_error_is_reported_after_the_caller_continues() {
    let program = crate::tests::compile_module(
        r#"
Перем Приложение;
Асинх Процедура Работа()
    КодРаботы = Приложение.Observe();
КонецПроцедуры
Работа();
Сообщить("caller continued");
Возврат 17;
"#,
    );
    let mut builder = bsl_rt::RuntimeBuilder::new();
    builder.register(bsl_rt::core_library());
    let registry = builder.build().unwrap();
    let mut env = bsl_rt::HostEnv::process();
    let mut execution =
        crate::ProgramExecution::start_with_registry(&program, &registry, &env).unwrap();
    let launcher = Arc::new(Launcher::default());
    execution.module_state.slots[0] = BslValue::new_object(Probe(launcher.clone()));
    let mut dynamic = crate::tests::TestDynamic::bare();
    let mut output = Vec::new();
    let mut poll = || {
        execution.poll_with_registry_and_io(
            &program,
            &registry,
            &mut output,
            &mut Vec::new(),
            &mut dynamic,
            &mut env,
            1,
        )
    };
    // Синхронная host-операция замораживает также вызывающего.
    assert!(matches!(poll().unwrap(), crate::ProgramPoll::Waiting));
    launcher
        .sink
        .lock()
        .unwrap()
        .take()
        .unwrap()
        .complete(Err(io::Error::from(io::ErrorKind::Other)));
    assert!(matches!(poll(), Err(RtError::IoError(ref text)) if text == "completion"));
    assert_eq!(output, b"caller continued\n");
}

#[test]
fn blocking_component_path_uses_the_application_mappers() {
    use bsl_rt::ObjectProtocol;
    for reject in [false, true] {
        let probe = Probe(Arc::new(Launcher {
            immediate: true,
            reject,
            ..Default::default()
        }));
        let mut shapes = bsl_rt::RuntimeShapes::seeded(Vec::new(), Vec::new(), None);
        let mut context = bsl_rt::CallContext::minimal(&mut shapes, |_, _| unreachable!());
        let result = bsl_rt::call_method_from_table(
            probe.method_table(),
            "ApplicationProbe",
            &probe,
            "Observe",
            &[],
            &mut context,
        );
        if reject {
            assert!(matches!(result, Err(RtError::IoError(message)) if message == "acceptance"));
        } else {
            assert_eq!(result.unwrap(), BslValue::number_from_i64(17));
        }
    }
}

#[test]
fn canceling_a_dynamic_async_task_after_root_return_is_not_success() {
    let program = crate::tests::compile_module(
        r#"
Перем Приложение;
Асинх Функция Работник()
    Сумма = 0;
    Для Номер = 1 По 3 Цикл Сумма = Сумма + Номер; КонецЦикла;
    Выполнить("Приложение.Observe();");
    Возврат 17;
КонецФункции
Работник();
"#,
    );
    let mut builder = bsl_rt::RuntimeBuilder::new();
    builder.register(bsl_rt::core_library());
    let registry = builder.build().unwrap();
    let mut env = bsl_rt::HostEnv::process();
    let mut execution = crate::ProgramExecution::start_with_registry_and_scheduler(
        &program,
        &registry,
        &env,
        SchedulerConfig {
            safe_points_per_quantum: 1,
        },
    )
    .unwrap();
    execution.force_scheduled = true;
    let canceled = Arc::new(std::sync::atomic::AtomicBool::new(false));
    execution.set_cancel_flag(canceled.clone());
    let launcher = Arc::new(Launcher::default());
    execution.module_state.slots[0] = BslValue::new_object(Probe(launcher.clone()));
    let mut dynamic = crate::tests::TestDynamic::bare();
    let first = execution
        .poll_with_registry_and_io(
            &program,
            &registry,
            &mut Vec::new(),
            &mut Vec::new(),
            &mut dynamic,
            &mut env,
            16,
        )
        .unwrap();
    assert!(matches!(first, crate::ProgramPoll::Waiting));
    assert!(
        execution.root_result.is_some(),
        "основное тело должно уже завершиться"
    );
    canceled.store(true, Ordering::SeqCst);
    let result = execution.poll_with_registry_and_io(
        &program,
        &registry,
        &mut Vec::new(),
        &mut Vec::new(),
        &mut dynamic,
        &mut env,
        16,
    );
    assert!(
        matches!(result, Err(RtError::Canceled)),
        "отмена потерялась при завершении async-задачи: {result:?}"
    );
    drop(execution);
    launcher
        .sink
        .lock()
        .unwrap()
        .take()
        .unwrap()
        .complete(Ok(ApplicationResult::Exited(ApplicationExit {
            code: Some(17),
        })));
}
