use open_bsl::{Engine, Value};

#[test]
fn period_presentation_resolves_but_never_fabricates_localized_text() {
    let engine = Engine::builder().build().unwrap();
    assert!(engine.compile("Р = ПредставлениеПериода(1);").is_err());
    assert!(
        engine
            .compile("Р = PeriodPresentation(1, 2, \"Л=ru_RU\", 3);")
            .is_err()
    );
    let result = engine
        .new_state()
        .exec(
            r#"
Попытка
    Р = ПредставлениеПериода(Дата(2024, 1, 1), Дата(2024, 1, 31));
Исключение
    Возврат Истина;
КонецПопытки;
Возврат Ложь;
"#,
        )
        .unwrap();
    assert_eq!(result, Value::Boolean(true));
}
