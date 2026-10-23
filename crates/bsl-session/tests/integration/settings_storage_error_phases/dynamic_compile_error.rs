use super::*;

#[test]
fn expression_compilation_inside_the_handler_is_execution_failure_not_preparation() {
    let factory = factory(include_str!(
        "../../../../../docs/research/fixtures/settings-storage-dynamic-compile-error-1018/manager.bsl"
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
        panic!("ожидался критический отказ тела обработчика: {error:?}");
    };
    assert_eq!(operation, "ОбработкаЗагрузки");
    assert!(matches!(*cause, RtError::DynamicError(_)), "{cause:?}");
    assert!(
        cause.to_string().contains("НеСуществующаяФункцияПробы"),
        "{cause:?}"
    );
    assert!(session.execute("Возврат 42;").is_err());
}
