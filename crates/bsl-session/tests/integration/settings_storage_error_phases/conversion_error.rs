use super::*;

#[test]
fn numeric_conversion_inside_load_handler_is_critical_and_preserves_its_cause() {
    let factory = factory(include_str!(
        "../../../../../docs/research/fixtures/settings-storage-conversion-error-1017/manager.bsl"
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
        panic!("ожидалась критическая ошибка преобразования: {error:?}");
    };
    assert_eq!(operation, "ОбработкаЗагрузки");
    assert!(matches!(*cause, RtError::Num(_)), "{cause:?}");
    assert!(session.execute("Возврат 42;").is_err());
}
