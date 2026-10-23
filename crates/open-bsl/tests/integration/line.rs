use open_bsl::{Engine, Value};

#[test]
fn line_constructor_resolves_documented_arities_without_fabricating_a_value() {
    let engine = Engine::builder().build().unwrap();
    for source in [
        "Значение = Новый Линия(Неопределено);",
        "Value = New Line(Undefined, 1);",
        "Value = New Line(Undefined, 1, False);",
    ] {
        engine.compile(source).unwrap();
    }
    for source in ["Значение = Новый Линия();", "Value = New Line(1, 2, 3, 4);"] {
        assert!(engine.compile(source).is_err());
    }

    let result = engine
        .new_state()
        .exec(
            "Попытка\nНовый Линия(Неопределено);\nИсключение\nВозврат Истина;\nКонецПопытки;\nВозврат Ложь;",
        )
        .unwrap();
    assert_eq!(result, Value::Boolean(true));
}
