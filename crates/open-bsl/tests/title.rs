use open_bsl::{Engine, Value};

#[test]
fn title_function_resolves_but_does_not_guess_word_boundaries() {
    let engine = Engine::builder().build().unwrap();
    assert!(engine.compile("ТРег();").is_err());
    assert!(engine.compile("Title(\"a\", \"b\");").is_err());
    engine.compile("Результат = Title(\"a\");").unwrap();

    let result = engine
        .new_state()
        .exec(
            r#"
Попытка
    Результат = ТРег("два слова");
Исключение
    Возврат Истина;
КонецПопытки;
Возврат Ложь;
"#,
        )
        .unwrap();
    assert_eq!(result, Value::Boolean(true));
}
