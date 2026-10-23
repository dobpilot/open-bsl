use super::Engine;

#[test]
fn ordinary_state_construction_and_execution_do_not_prepare_background_runtime() {
    let engine = Engine::builder()
        .common_module(
            "Служебный",
            "Функция Эхо(Знач Х) Экспорт Возврат Х; КонецФункции",
        )
        .build()
        .unwrap();
    let module = engine.compile_entry("Возврат Служебный.Эхо(7);").unwrap();
    let mut state = engine.new_state();
    assert!(engine.inner.job_runtime.get().is_none());
    state.run(&module).unwrap();
    assert!(engine.inner.job_runtime.get().is_none());
}

#[test]
fn first_background_access_prepares_the_runtime_shared_by_engine_clones() {
    let engine = Engine::builder()
        .common_module("Служебный", "Процедура Работа() Экспорт КонецПроцедуры")
        .build()
        .unwrap();
    let clone = engine.clone();
    let module = engine.compile_entry("Возврат ФоновыеЗадания;").unwrap();
    let mut first = engine.new_state();
    let mut second = clone.new_state();
    assert!(engine.inner.job_runtime.get().is_none());
    first.run(&module).unwrap();
    let runtime = engine.job_runtime().unwrap();
    second.run(&module).unwrap();
    assert!(std::sync::Arc::ptr_eq(
        &runtime,
        &clone.job_runtime().unwrap()
    ));
    runtime.shutdown(std::time::Duration::from_secs(2));
}

#[test]
fn failed_deferred_preparation_keeps_the_missing_capability_error() {
    let engine = Engine::builder()
        .common_module("Служебный", "Процедура Работа() Экспорт КонецПроцедуры")
        .build()
        .unwrap();
    assert!(
        engine
            .inner
            .job_runtime
            .set(Err("ошибка подготовки".into()))
            .is_ok()
    );
    let module = engine.compile_entry("Возврат ФоновыеЗадания;").unwrap();
    let mut state = engine.new_state();
    let error = state.run(&module).unwrap_err();
    assert!(matches!(
        error,
        crate::Error::Runtime(bsl_rt::RtError::CapabilityMissing {
            capability: bsl_rt::Capability::BackgroundJobs,
            ..
        })
    ));
}
