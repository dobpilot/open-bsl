use super::super::test_support::{engine, number, params};
use super::super::{BackgroundJobConfig, runtime_for_engine};
use super::*;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

#[derive(Default)]
struct Probe {
    builds: AtomicUsize,
    idle: AtomicBool,
    entered: AtomicBool,
    panic_on_build: AtomicBool,
    held: Mutex<bool>,
    released: Condvar,
}

static PROBES: Mutex<Vec<(usize, Arc<Probe>)>> = Mutex::new(Vec::new());

fn probe(shared: &Arc<JobRuntimeShared>) -> Option<Arc<Probe>> {
    PROBES
        .lock()
        .unwrap()
        .iter()
        .find(|(key, _)| *key == Arc::as_ptr(shared) as usize)
        .map(|(_, probe)| Arc::clone(probe))
}

pub(super) fn on_build(shared: &Arc<JobRuntimeShared>) {
    if let Some(probe) = probe(shared) {
        probe.builds.fetch_add(1, Ordering::SeqCst);
        probe.entered.store(true, Ordering::SeqCst);
        let mut held = probe.held.lock().unwrap();
        while *held {
            held = probe.released.wait(held).unwrap();
        }
        drop(held);
        assert!(
            !probe.panic_on_build.swap(false, Ordering::SeqCst),
            "контролируемый сбой построения каталога"
        );
    }
}

pub(super) fn on_idle(shared: &Arc<JobRuntimeShared>) {
    if let Some(probe) = probe(shared) {
        probe.idle.store(true, Ordering::SeqCst);
    }
}

#[test]
fn an_idle_worker_does_not_construct_a_configuration_engine() {
    let runtime = runtime_for_engine(&engine(), BackgroundJobConfig::default()).unwrap();
    let probe = Arc::new(Probe::default());
    let key = Arc::as_ptr(&runtime.shared) as usize;
    PROBES.lock().unwrap().push((key, Arc::clone(&probe)));
    let shared = Arc::clone(&runtime.shared);
    let worker = std::thread::spawn(move || worker_main(&shared, &AtomicBool::new(false)));
    let deadline = Instant::now() + Duration::from_secs(30);
    while !probe.idle.load(Ordering::SeqCst) {
        assert!(Instant::now() < deadline, "worker не дошёл до ожидания");
        std::thread::yield_now();
    }
    let builds = probe.builds.load(Ordering::SeqCst);
    runtime.shutdown(Duration::from_secs(30));
    worker.join().unwrap();
    PROBES.lock().unwrap().retain(|(id, _)| *id != key);
    assert_eq!(builds, 0, "пустой worker не должен разбирать каталог");
}

fn wait_until(mut condition: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(30);
    while !condition() {
        assert!(Instant::now() < deadline, "условие не выполнено");
        std::thread::yield_now();
    }
}

fn setup(held: bool) -> (super::super::JobRuntime, Arc<Probe>, usize) {
    let runtime = runtime_for_engine(
        &engine(),
        BackgroundJobConfig {
            workers: Some(1),
            ..BackgroundJobConfig::default()
        },
    )
    .unwrap();
    let probe = Arc::new(Probe::default());
    *probe.held.lock().unwrap() = held;
    let key = Arc::as_ptr(&runtime.shared) as usize;
    PROBES.lock().unwrap().push((key, Arc::clone(&probe)));
    (runtime, probe, key)
}

fn submit(runtime: &super::super::JobRuntime) -> JobId {
    let target =
        super::super::resolve_target(engine().catalog().unwrap(), "Служебный.Сложить").unwrap();
    runtime
        .submit(
            "Служебный.Сложить",
            target,
            params(&[number(20), number(22)]),
            None,
            None,
            None,
            0,
        )
        .unwrap()
        .id
}

fn release(probe: &Probe) {
    *probe.held.lock().unwrap() = false;
    probe.released.notify_all();
}

#[test]
fn one_worker_reuses_its_engine_for_two_jobs() {
    let (runtime, probe, key) = setup(false);
    for _ in 0..2 {
        let id = submit(&runtime);
        assert!(
            runtime
                .wait_terminal(&[id], Some(Duration::from_secs(30)))
                .unwrap()
        );
        assert_eq!(runtime.snapshot(id).unwrap().state, JobStateDto::Completed);
    }
    runtime.shutdown(Duration::from_secs(30));
    assert_eq!(probe.builds.load(Ordering::SeqCst), 1);
    PROBES.lock().unwrap().retain(|(id, _)| *id != key);
}

#[test]
fn cancellation_during_catalog_construction_does_not_resurrect_the_job() {
    let (runtime, probe, key) = setup(true);
    let id = submit(&runtime);
    wait_until(|| probe.entered.load(Ordering::SeqCst));
    runtime.cancel(id).unwrap();
    assert_eq!(runtime.snapshot(id).unwrap().state, JobStateDto::Canceled);
    release(&probe);
    wait_until(|| probe.idle.load(Ordering::SeqCst));
    assert_eq!(runtime.snapshot(id).unwrap().state, JobStateDto::Canceled);
    runtime.shutdown(Duration::from_secs(30));
    PROBES.lock().unwrap().retain(|(id, _)| *id != key);
}

#[test]
fn shutdown_during_catalog_construction_does_not_resurrect_the_job() {
    let (runtime, probe, key) = setup(true);
    let id = submit(&runtime);
    wait_until(|| probe.entered.load(Ordering::SeqCst));
    let report = runtime.shutdown(Duration::ZERO);
    assert_eq!(report.detached_workers, 1);
    assert_eq!(runtime.snapshot(id).unwrap().state, JobStateDto::Canceled);
    release(&probe);
    // Состояние Closed остаётся и после возврата построенного движка.
    wait_until(|| Arc::strong_count(&runtime.shared) == 1);
    assert_eq!(
        runtime.shared.registry.lock().unwrap().state,
        RuntimeState::Closed
    );
    assert_eq!(runtime.snapshot(id).unwrap().state, JobStateDto::Canceled);
    PROBES.lock().unwrap().retain(|(id, _)| *id != key);
}

#[test]
fn a_failed_cold_catalog_breaks_the_runtime_and_finishes_queued_jobs() {
    let (mut runtime, probe, key) = setup(true);
    Arc::get_mut(&mut runtime.shared).unwrap().recipe.image_text =
        Arc::new("повреждённый образ".into());
    let first = submit(&runtime);
    wait_until(|| probe.entered.load(Ordering::SeqCst));
    let second = submit(&runtime);
    release(&probe);
    wait_until(|| runtime.shared.registry.lock().unwrap().state == RuntimeState::Broken);
    for id in [first, second] {
        assert_eq!(runtime.snapshot(id).unwrap().state, JobStateDto::Failed);
    }
    assert_eq!(probe.builds.load(Ordering::SeqCst), 1);
    runtime.shutdown(Duration::from_secs(30));
    PROBES.lock().unwrap().retain(|(id, _)| *id != key);
}

#[test]
fn a_failed_catalog_after_shutdown_preserves_closed_and_canceled() {
    let (mut runtime, probe, key) = setup(true);
    Arc::get_mut(&mut runtime.shared).unwrap().recipe.image_text =
        Arc::new("повреждённый образ".into());
    let id = submit(&runtime);
    wait_until(|| probe.entered.load(Ordering::SeqCst));
    runtime.shutdown(Duration::ZERO);
    release(&probe);
    wait_until(|| Arc::strong_count(&runtime.shared) == 1);
    assert_eq!(
        runtime.shared.registry.lock().unwrap().state,
        RuntimeState::Closed
    );
    assert_eq!(runtime.snapshot(id).unwrap().state, JobStateDto::Canceled);
    PROBES.lock().unwrap().retain(|(id, _)| *id != key);
}

#[test]
fn a_panicking_cold_build_does_not_leave_the_claimed_job_queued() {
    let (runtime, probe, key) = setup(false);
    probe.panic_on_build.store(true, Ordering::SeqCst);
    let first = submit(&runtime);
    assert!(
        runtime
            .wait_terminal(&[first], Some(Duration::from_secs(30)))
            .unwrap()
    );
    let failed = runtime.snapshot(first).unwrap();
    assert_eq!(failed.state, JobStateDto::Failed);
    // Настоящий supervisor восстанавливает worker после первой паники.
    let second = submit(&runtime);
    assert!(
        runtime
            .wait_terminal(&[second], Some(Duration::from_secs(30)))
            .unwrap()
    );
    assert_eq!(
        runtime.snapshot(second).unwrap().state,
        JobStateDto::Completed
    );
    assert_eq!(probe.builds.load(Ordering::SeqCst), 2);
    runtime.shutdown(Duration::from_secs(30));
    PROBES.lock().unwrap().retain(|(id, _)| *id != key);
}
