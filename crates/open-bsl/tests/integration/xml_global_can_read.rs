use open_bsl::{Engine, Value};

#[test]
fn global_can_read_xml_resolves_but_does_not_guess_readability() {
    let engine = Engine::builder().build().unwrap();
    for name in ["ВозможностьЧтенияXML", "canreadxml"] {
        engine
            .compile(&format!(
                "Процедура НеВызывается(Чтение)\nЕсли {name}(Чтение) Тогда\nВозврат;\nКонецЕсли;\nКонецПроцедуры"
            ))
            .unwrap();
    }
    assert_eq!(
        engine
            .new_state()
            .exec(
                "Попытка\nР = ВозможностьЧтенияXML(Неопределено);\nИсключение\nВозврат Истина;\nКонецПопытки;\nВозврат Ложь;",
            )
            .unwrap(),
        Value::Boolean(true)
    );
    assert!(engine.compile("Р = CanReadXML();").is_err());
    assert!(engine.compile("Р = CanReadXML(1, 2);").is_err());
}
