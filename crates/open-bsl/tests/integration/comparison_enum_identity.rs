use open_bsl::{Engine, Value};

#[test]
fn filter_and_composition_comparisons_keep_distinct_value_and_type_identity() {
    let engine = Engine::builder().build().unwrap();
    let result = engine
        .new_state()
        .exec(
            r#"
            Возврат ВидСравнения.Равно = ComparisonType.Equal
                И ВидСравненияКомпоновкиДанных.Равно = DataCompositionComparisonType.Equal
                И ВидСравнения.Равно <> ВидСравненияКомпоновкиДанных.Равно
                И ТипЗнч(ВидСравнения.Равно) <> ТипЗнч(ВидСравненияКомпоновкиДанных.Равно);
            "#,
        )
        .unwrap();
    assert_eq!(result, Value::Boolean(true));
}
