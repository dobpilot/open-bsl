use open_bsl::{Engine, Value};

#[test]
fn xml_reader_settings_resolves_documented_arities_and_fails_explicitly_when_called() {
    let engine = Engine::builder().build().unwrap();
    for source in [
        "П = Новый ПараметрыЧтенияXML();",
        "П = Новый XMLReaderSettings(\"1.0\");",
        "П = Новый XMLReaderSettings(1,2,3,4,5,6,7,8,9,10,11);",
    ] {
        engine.compile(source).unwrap();
    }
    assert!(
        engine
            .compile("П = Новый ПараметрыЧтенияXML(1,2,3,4,5,6,7,8,9,10,11,12);")
            .is_err()
    );
    assert_eq!(
        engine
            .new_state()
            .exec(
                "Попытка\nНовый ПараметрыЧтенияXML();\nИсключение\nВозврат Истина;\nКонецПопытки;\nВозврат Ложь;",
            )
            .unwrap(),
        Value::Boolean(true)
    );
}
