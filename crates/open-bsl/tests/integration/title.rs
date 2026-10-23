use open_bsl::{Engine, Value};

#[test]
fn title_function_checks_arity_and_follows_measured_word_boundaries() {
    let engine = Engine::builder().build().unwrap();
    assert!(engine.compile("ТРег();").is_err());
    assert!(engine.compile("Title(\"a\", \"b\");").is_err());
    // Измерено: `tests/conformance/measure/1c/measure-title-case.platform.txt`.
    let result = engine
        .new_state()
        .exec(r#"Возврат ТРег("два слова") + "|" + Title("o'neil-smith");"#)
        .unwrap();
    assert_eq!(result, Value::Str("Два Слова|O'neil-Smith".into()));
}
