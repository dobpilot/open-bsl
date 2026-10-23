#![cfg(feature = "http")]

use open_bsl::{Engine, Value};

#[test]
fn linux_authorities_resolve_through_the_facade_with_native_identity() {
    let engine = Engine::builder().build().unwrap();
    for name in [
        "СертификатыУдостоверяющихЦентровLinux",
        "LinuxCertificationAuthorityCertificates",
        "linuxcertificationauthoritycertificates",
    ] {
        assert!(engine.compile(&format!("Новый {name}(1);")).is_err());
        let source = format!("Если Ложь Тогда Новый {name}(); КонецЕсли; Возврат 1;");
        assert_eq!(
            engine.new_state().exec(&source).unwrap(),
            Value::number_from_i64(1)
        );
        if cfg!(target_os = "linux") {
            let source = format!(
                "КорниПробы = Новый {name}();\n\
                 СоединениеПробы = Новый ЗащищенноеСоединениеOpenSSL(Неопределено, КорниПробы);\n\
                 Возврат ТипЗнч(КорниПробы) = Тип(\"LinuxCertificationAuthorityCertificates\")\n\
                 И ТипЗнч(КорниПробы) <> Тип(\"CertificationAuthorityCertificatesOS\")\n\
                 И ТипЗнч(СоединениеПробы) = Тип(\"OpenSSLSecureConnection\");"
            );
            assert_eq!(
                engine.new_state().exec(&source).unwrap(),
                Value::Boolean(true)
            );
        } else {
            let error = engine
                .new_state()
                .exec(&format!("Новый {name}();"))
                .unwrap_err();
            assert!(error.to_string().contains("хранилище сертификатов Linux"));
        }
    }
}
