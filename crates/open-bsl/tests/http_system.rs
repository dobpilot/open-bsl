#![cfg(all(feature = "http", not(target_arch = "wasm32")))]

use std::thread;
use std::time::{Duration, Instant};

use open_bsl::{Engine, ExecutionPoll};

#[path = "../../../tests/support/http_system.rs"]
mod support;

#[test]
fn system_http_runs_end_to_end_through_state_in_interpreter_and_jit() {
    let engine = Engine::builder().build().unwrap();
    for jit in [false, true] {
        let (port, observed, server) = support::start_server();
        let module = engine.compile(&support::source(port)).unwrap();
        engine
            .state_builder()
            .jit(jit)
            .build()
            .run(&module)
            .unwrap();
        server.join().unwrap();
        support::assert_requests(&observed);
    }
}

#[test]
fn system_http_runs_end_to_end_through_pollable_execution() {
    let (port, observed, server) = support::start_server();
    let engine = Engine::builder().build().unwrap();
    let module = engine.compile(&support::source(port)).unwrap();
    let mut state = engine.new_state();
    let mut execution = state.start(&module).unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        match execution.poll(1).unwrap() {
            ExecutionPoll::Complete(_) => break,
            ExecutionPoll::Runnable | ExecutionPoll::Waiting => {
                assert!(Instant::now() < deadline, "pollable HTTP-запуск завис");
                thread::yield_now();
            }
        }
    }
    server.join().unwrap();
    support::assert_requests(&observed);
}
