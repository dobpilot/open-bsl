use open_bsl::{Engine, Value};

#[test]
fn font_constructor_is_resolved_but_does_not_fabricate_a_value() {
    let engine = Engine::builder().build().unwrap();
    engine.compile("Шрифт = Новый Шрифт();").unwrap();
    engine.compile("Шрифт = New Font(, 12, Истина);").unwrap();
    engine
        .compile("Исходный = Неопределено; Шрифт = Новый Шрифт(Исходный, , , Истина);")
        .unwrap();
    assert!(
        engine
            .compile("Шрифт = Новый Шрифт(1,2,3,4,5,6,7,8,9);")
            .is_err()
    );

    let result = engine
        .new_state()
        .exec(
            r#"
Попытка
    Значение = Новый Шрифт(, 12, Истина);
Исключение
    Возврат Истина;
КонецПопытки;
Возврат Ложь;
"#,
        )
        .unwrap();
    assert_eq!(result, Value::Boolean(true));
}
