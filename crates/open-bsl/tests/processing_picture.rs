use open_bsl::{Engine, Value};

#[test]
fn processing_picture_resolves_documented_arity_without_inventing_a_value() {
    let engine = Engine::builder().build().unwrap();
    for source in [
        "Значение = Новый ОбрабатываемаяКартинка();",
        "Value = New ProcessingPicture(Undefined);",
        "Value = New ProcessingPicture(Undefined, True);",
        "Value = New ProcessingPicture(Undefined, True, Undefined);",
    ] {
        engine.compile(source).unwrap();
    }
    assert!(
        engine
            .compile("Значение = Новый ОбрабатываемаяКартинка(1, 2, 3, 4);")
            .is_err()
    );
    assert_eq!(
        engine
            .new_state()
            .exec(
                "Попытка\nНовый ОбрабатываемаяКартинка();\nИсключение\nВозврат Истина;\nКонецПопытки;\nВозврат Ложь;",
            )
            .unwrap(),
        Value::Boolean(true)
    );
}
