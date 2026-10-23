use open_bsl::{Engine, Value};

#[test]
fn com_object_constructor_is_resolved_and_fails_inside_try() {
    let engine = Engine::builder().build().unwrap();
    assert!(engine.compile("Значение = Новый COMОбъект();").is_err());
    assert!(
        engine
            .compile("Значение = Новый COMОбъект(\"Приложение\", \"Сервер\", \"Лишний\");")
            .is_err()
    );
    engine
        .compile("Значение = New COMObject(\"Приложение\", \"Сервер\");")
        .unwrap();

    let result = engine
        .new_state()
        .exec(
            r#"
Попытка
    Значение = Новый COMОбъект("Приложение");
Исключение
    Возврат Истина;
КонецПопытки;
Возврат Ложь;
"#,
        )
        .unwrap();
    assert_eq!(result, Value::Boolean(true));
}
