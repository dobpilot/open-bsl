#![cfg(feature = "http")]

use open_bsl::{Engine, Value};

#[test]
fn dormant_windows_certificate_constructors_compile_without_a_store_adapter() {
    let engine = Engine::builder().build().unwrap();
    let value = engine.new_state().exec(
        "Если Ложь Тогда\nНовый СертификатКлиентаWindows();\nНовый WindowsClientCertificate(Неопределено);\nКонецЕсли;\nВозврат 1;",
    ).unwrap();
    assert_eq!(value, Value::number_from_i64(1));
    assert!(
        engine
            .compile("Новый WindowsClientCertificate(1, 2);")
            .is_err()
    );
}

#[test]
fn windows_certificate_execution_reports_the_missing_store_adapter() {
    let engine = Engine::builder().build().unwrap();
    for source in [
        "Новый СертификатКлиентаWindows();",
        "Новый WindowsClientCertificate(Неопределено);",
    ] {
        let error = engine.new_state().exec(source).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("Windows: объявлено без реализации"),
            "{error}"
        );
    }
}

#[test]
fn windows_authority_certificates_require_a_store_adapter() {
    let engine = Engine::builder().build().unwrap();
    for name in [
        "СертификатыУдостоверяющихЦентровWindows",
        "WindowsCertificationAuthorityCertificates",
    ] {
        let source = format!("Если Ложь Тогда Новый {name}(); КонецЕсли; Возврат 1;");
        assert_eq!(
            engine.new_state().exec(&source).unwrap(),
            Value::number_from_i64(1)
        );
        assert!(engine.compile(&format!("Новый {name}(1);")).is_err());
        let error = engine
            .new_state()
            .exec(&format!("Новый {name}();"))
            .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("Windows: объявлено без реализации"),
            "{error}"
        );
    }
}
