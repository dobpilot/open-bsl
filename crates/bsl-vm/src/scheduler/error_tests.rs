//! Порядок отложенных ошибок и границы владения очередью.

use super::*;

fn state() -> AsyncState {
    AsyncState::new(
        Task {
            frames: Vec::new(),
            stack: Vec::new(),
            current_exception: None,
            completion: TaskCompletion::Root,
            quantum_remaining: 1,
        },
        1,
    )
}

#[test]
fn detached_error_keeps_its_place_before_later_promise_waiters() {
    let mut state = state();
    state
        .defer_detached_error(RtError::IoError("first".into()))
        .unwrap();
    let (promise, _) = state.new_promise().unwrap();
    let PromiseState::Pending { waiters } = &mut state.promises[0] else {
        panic!()
    };
    waiters.extend([1, 2]);
    state
        .resolve_promise(promise, Ok(BslValue::Undefined))
        .unwrap();
    assert!(matches!(state.ready.pop_front(), Some(ReadyEvent::Task(0))));
    assert!(
        matches!(state.ready.pop_front(), Some(ReadyEvent::Error(error))
        if matches!(*error, RtError::IoError(ref text) if text == "first"))
    );
    assert!(matches!(state.ready.pop_front(), Some(ReadyEvent::Task(1))));
    assert!(matches!(state.ready.pop_front(), Some(ReadyEvent::Task(2))));
    assert!(state.ready.is_empty());
}

#[test]
fn cancellation_and_invalid_bytecode_are_not_deferred() {
    let mut state = state();
    assert!(matches!(
        state.defer_detached_error(RtError::Canceled),
        Err(RtError::Canceled)
    ));
    assert!(matches!(
        state.defer_detached_error(RtError::InvalidBytecode("broken")),
        Err(RtError::InvalidBytecode("broken"))
    ));
    assert_eq!(state.ready.len(), 1);
}

#[test]
fn synchronous_wait_keeps_other_tasks_and_errors_frozen_in_order() {
    let mut state = state();
    state
        .defer_detached_error(RtError::IoError("first".into()))
        .unwrap();
    state.ready.push_back(ReadyEvent::Task(2));
    state.sync_wait = Some(SyncWait {
        task_id: 2,
        promise_id: PromiseId::new(0),
        dst: 0,
    });
    assert!(matches!(
        take_frozen_ready(&mut state),
        Some(ReadyEvent::Task(2))
    ));
    assert!(take_frozen_ready(&mut state).is_none());
    assert!(matches!(state.ready.pop_front(), Some(ReadyEvent::Task(0))));
    assert!(matches!(
        state.ready.pop_front(),
        Some(ReadyEvent::Error(_))
    ));
    assert!(state.ready.is_empty());
}

#[test]
fn dropping_the_queue_releases_values_owned_by_errors() {
    let mut state = state();
    let value = BslValue::new_array(Vec::new());
    let BslValue::Object(object) = &value else {
        panic!()
    };
    let weak = std::rc::Rc::downgrade(object);
    state.defer_detached_error(RtError::Raised(value)).unwrap();
    assert!(weak.upgrade().is_some());
    drop(state);
    assert!(weak.upgrade().is_none());
}

#[test]
fn debug_poll_does_not_run_an_old_task_woken_by_a_completion() {
    let program = crate::tests::compile_module("Возврат 42;");
    let mut builder = bsl_rt::RuntimeBuilder::new();
    builder.register(bsl_rt::core_library());
    let registry = builder.build().unwrap();
    let mut env = bsl_rt::HostEnv::process();
    let mut execution =
        crate::ProgramExecution::start_with_registry(&program, &registry, &env).unwrap();
    let mut expression =
        crate::ProgramExecution::start_with_registry(&program, &registry, &env).unwrap();
    let task = expression.async_state.tasks[0].take().unwrap();
    let id = execution.async_state.insert_task(task);
    execution.debug_task_floor = Some(id);
    // Старая задача уже попала в FIFO после завершения host, но её BSL
    // не должен продолжиться во время вычисления выражения.
    execution.async_state.ready.push_back(ReadyEvent::Task(id));
    let result = execution
        .poll_with_registry_and_io(
            &program,
            &registry,
            &mut Vec::new(),
            &mut Vec::new(),
            &mut crate::tests::TestDynamic::bare(),
            &mut env,
            usize::MAX,
        )
        .unwrap();
    assert!(
        matches!(result, crate::ProgramPoll::Complete(value, _) if value == BslValue::number_from_i64(42))
    );
    assert!(execution.async_state.tasks[0].is_some());
    assert!(matches!(
        execution.async_state.ready.front(),
        Some(ReadyEvent::Task(0))
    ));
}

#[test]
fn discarding_debug_root_keeps_other_waiters_and_errors() {
    let mut state = state();
    let (promise_id, _) = state.new_promise().unwrap();
    let PromiseState::Pending { waiters } = &mut state.promises[0] else {
        panic!()
    };
    waiters.extend([0, 1]);
    state.sync_wait = Some(SyncWait {
        task_id: 0,
        promise_id,
        dst: 0,
    });
    state
        .defer_detached_error(RtError::IoError("keep".into()))
        .unwrap();
    state.discard_debug_task(0);
    assert!(state.tasks[0].is_none());
    assert!(state.sync_wait.is_none());
    assert_eq!(state.ready.len(), 1);
    assert!(matches!(state.ready.front(), Some(ReadyEvent::Error(_))));
    let PromiseState::Pending { waiters } = &state.promises[0] else {
        panic!()
    };
    assert_eq!(*waiters, VecDeque::from([1]));
}
