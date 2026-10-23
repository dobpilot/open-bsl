use open_bsl::{Engine, Value};

#[test]
fn number_in_words_resolves_but_never_fabricates_localized_text() {
    let engine = Engine::builder().build().unwrap();
    assert!(engine.compile("Р = ЧислоПрописью();").is_err());
    assert!(
        engine
            .compile("Р = NumberInWords(1, \"Л=ru_RU\", \"рубль\", 4);")
            .is_err()
    );
    let result = engine
        .new_state()
        .exec(
            r#"
Попытка
    Р = ЧислоПрописью(5);
Исключение
    Возврат Истина;
КонецПопытки;
Возврат Ложь;
"#,
        )
        .unwrap();
    assert_eq!(result, Value::Boolean(true));
}
