use open_bsl::{Engine, Value};

#[test]
fn numeric_value_type_matches_the_platform_measurement() {
    let result = Engine::builder()
        .build()
        .unwrap()
        .new_state()
        .exec(
            r#"
Возврат Строка(ТипЗнч(ВидЧисловогоЗначения.Количественное)) + "|"
    + Строка(ВидЧисловогоЗначения.Количественное) + "|"
    + Строка(NumericValueType.Ordinal) + "|"
    + ?(NumericValueType.Cardinal = ВидЧисловогоЗначения.Количественное, "true", "false");
"#,
        )
        .unwrap();
    let Value::Str(value) = result else {
        panic!("ожидалась строка");
    };
    assert_eq!(
        value.to_string(),
        "ВидЧисловогоЗначения|Количественное|Порядковое|true"
    );
}
