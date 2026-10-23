use super::*;

#[test]
fn array_bounds_inside_load_handler_are_critical_and_preserve_the_cause() {
    let factory = factory(include_str!(
        "../../../../docs/research/fixtures/settings-storage-index-error-1019/manager.bsl"
    ));
    let mut session = factory
        .open(
            Principal::new("user", "user", vec![]),
            SessionResources::new(Box::new(()), 1),
        )
        .unwrap();
    let error = session.execute(CAUGHT_LOAD).unwrap_err();
    let SessionError::OpenBsl(Error::Runtime(RtError::CriticalConfiguration { operation, cause })) =
        error
    else {
        panic!("ожидалась критическая ошибка индекса: {error:?}");
    };
    assert_eq!(operation, "ОбработкаЗагрузки");
    assert!(
        matches!(*cause, RtError::IndexOutOfBounds { index: 0, len: 0 }),
        "{cause:?}"
    );
    assert!(session.execute("Возврат 42;").is_err());
    let mut fresh = factory
        .open(
            Principal::new("user", "user", vec![]),
            SessionResources::new(Box::new(()), 2),
        )
        .unwrap();
    fresh.execute("Возврат 42;").unwrap();
}
