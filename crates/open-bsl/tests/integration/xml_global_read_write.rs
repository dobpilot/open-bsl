use open_bsl::{Engine, Value};

#[test]
fn global_xml_write_forms_resolve_without_fabricating_output() {
    let engine = Engine::builder().build().unwrap();
    for name in ["ЗаписатьXML", "writexml"] {
        for arguments in [
            "Неопределено, Неопределено",
            "Неопределено, Неопределено, Неопределено",
            "Неопределено, Неопределено, Неопределено, Неопределено",
            "Неопределено, Неопределено, Неопределено, Неопределено, Неопределено",
        ] {
            engine
                .compile(&format!(
                    "Процедура НеВызывается()\n{name}({arguments});\nКонецПроцедуры"
                ))
                .unwrap();
        }
    }
    assert_eq!(
        engine
            .new_state()
            .exec(
                "Попытка\nЗаписатьXML(Неопределено, Неопределено);\nИсключение\nВозврат Истина;\nКонецПопытки;\nВозврат Ложь;",
            )
            .unwrap(),
        Value::Boolean(true)
    );
    assert!(engine.compile("Р = WriteXML(1, 2);").is_err());
    assert!(engine.compile("WriteXML(1);").is_err());
    assert!(engine.compile("WriteXML(1, 2, 3, 4, 5, 6);").is_err());
}

#[test]
fn global_xml_read_forms_resolve_without_fabricating_a_value() {
    let engine = Engine::builder().build().unwrap();
    for source in [
        "ReadXML(Неопределено)",
        "ПрочитатьXML(Неопределено, Неопределено)",
    ] {
        engine
            .compile(&format!(
                "Процедура НеВызывается()\nР = {source};\nКонецПроцедуры"
            ))
            .unwrap();
    }
    assert_eq!(
        engine
            .new_state()
            .exec(
                "Попытка\nР = ПрочитатьXML(Неопределено);\nИсключение\nВозврат Истина;\nКонецПопытки;\nВозврат Ложь;",
            )
            .unwrap(),
        Value::Boolean(true)
    );
    assert!(engine.compile("Р = ReadXML();").is_err());
    assert!(engine.compile("Р = ReadXML(1, 2, 3);").is_err());
}
