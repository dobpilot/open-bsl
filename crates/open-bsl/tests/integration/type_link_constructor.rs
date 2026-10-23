use open_bsl::{Engine, Value};

#[test]
fn type_link_constructor_resolves_documented_forms_without_fabricating_a_link() {
    let engine = Engine::builder().build().unwrap();
    for source in [
        "Новый СвязьПоТипу",
        "New TypeLink(\"Источник.Свойство\")",
        "Новый СвязьПоТипу(\"Источник.Свойство\", 0)",
    ] {
        engine
            .compile(&format!(
                "Процедура НеВызывается()\nСвязь = {source};\nКонецПроцедуры"
            ))
            .unwrap();
    }
    assert_eq!(
        engine
            .new_state()
            .exec(
                "Попытка\nСвязь = Новый СвязьПоТипу;\nИсключение\nВозврат Истина;\nКонецПопытки;\nВозврат Ложь;",
            )
            .unwrap(),
        Value::Boolean(true)
    );
    assert!(
        engine
            .compile("Связь = Новый СвязьПоТипу(1, 2, 3);")
            .is_err()
    );
}
