use open_bsl::{Engine, Value};

#[test]
fn form_attribute_constructor_resolves_and_fails_inside_try() {
    let engine = Engine::builder().build().unwrap();
    engine
        .compile("Реквизит = Новый РеквизитФормы(\"Имя\", Новый ОписаниеТипов());")
        .unwrap();
    engine
        .compile("Attribute = New FormAttribute(\"Name\", New TypeDescription());")
        .unwrap();
    assert!(
        engine
            .compile("Реквизит = Новый РеквизитФормы(\"Имя\");")
            .is_err()
    );

    let result = engine
        .new_state()
        .exec(
            "Попытка\nНовый РеквизитФормы(\"Имя\", Новый ОписаниеТипов());\nИсключение\nВозврат Истина;\nКонецПопытки;\nВозврат Ложь;",
        )
        .unwrap();
    assert_eq!(result, Value::Boolean(true));
}
