use open_bsl::{Engine, Value};

#[test]
fn string_with_number_is_resolved_but_never_fabricates_a_plural_form() {
    let engine = Engine::builder().build().unwrap();
    assert!(engine.compile("Р = СтрокаСЧислом();").is_err());
    assert!(
        engine
            .compile(
                "Р = СтрокаСЧислом(\"формы\", 2, ВидЧисловогоЗначения.Количественное, \"L=ru\", 1);"
            )
            .is_err()
    );
    let result = engine
        .new_state()
        .exec(
            r#"
Попытка
    Р = StringWithNumber(";день;;;дня;дней", 2, NumericValueType.Cardinal);
Исключение
    Возврат Истина;
КонецПопытки;
Возврат Ложь;
"#,
        )
        .unwrap();
    assert_eq!(result, Value::Boolean(true));
}
