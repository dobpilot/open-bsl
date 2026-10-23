use open_bsl::{Engine, Value};

#[test]
fn service_and_certificate_constructors_resolve_without_fabricated_values() {
    let engine = Engine::builder().build().unwrap();
    for source in [
        "Новый WSОпределения(\"file.wsdl\")",
        "Новый WSProxy(Неопределено, \"urn:example\", \"Service\", \"Endpoint\")",
        "Новый СертификатКриптографии()",
        "Новый WSDefinitions(\"file.wsdl\")",
        "Новый CryptoCertificate(Неопределено)",
    ] {
        engine
            .compile(&format!(
                "Процедура НеВызывается()\nЗначение = {source};\nКонецПроцедуры\nВозврат 1;"
            ))
            .unwrap();
        assert_eq!(
            engine
                .new_state()
                .exec(&format!(
                    "Попытка\nЗначение = {source};\nИсключение\nВозврат Истина;\nКонецПопытки;\nВозврат Ложь;"
                ))
                .unwrap(),
            Value::Boolean(true)
        );
    }
    for source in [
        "Новый WSОпределения()",
        "Новый WSПрокси(1, 2, 3)",
        "Новый СертификатКриптографии(1, 2)",
    ] {
        assert!(engine.compile(&format!("Значение = {source};")).is_err());
    }
}
