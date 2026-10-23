use open_bsl::{Engine, Value};

#[test]
fn global_xml_type_resolves_without_reusing_the_value_mapping() {
    let engine = Engine::builder().build().unwrap();
    for name in ["XMLТип", "xmltype"] {
        engine
            .compile(&format!(
                "Процедура НеВызывается(ТипДокумента)\nТипXML = {name}(ТипДокумента);\nКонецПроцедуры"
            ))
            .unwrap();
    }
    assert_eq!(
        engine
            .new_state()
            .exec(
                "Попытка\nТипXML = XMLТип(Тип(\"Строка\"));\nИсключение\nВозврат Истина;\nКонецПопытки;\nВозврат Ложь;",
            )
            .unwrap(),
        Value::Boolean(true)
    );
    assert!(engine.compile("ТипXML = XMLТип();").is_err());
    assert!(engine.compile("ТипXML = XMLТип(1, 2);").is_err());
}
