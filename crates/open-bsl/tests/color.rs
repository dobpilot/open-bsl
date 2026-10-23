use open_bsl::{Engine, Value};

#[test]
fn color_constructors_resolve_without_fabricating_a_value() {
    let engine = Engine::builder().build().unwrap();
    engine.compile("Значение = Новый Цвет();").unwrap();
    engine.compile("Value = New Color(255, 0, 0);").unwrap();
    assert!(engine.compile("Значение = Новый Цвет(255);").is_err());

    let result = engine
        .new_state()
        .exec("Попытка\nНовый Цвет(255, 0, 0);\nИсключение\nВозврат Истина;\nКонецПопытки;\nВозврат Ложь;")
        .unwrap();
    assert_eq!(result, Value::Boolean(true));
}
