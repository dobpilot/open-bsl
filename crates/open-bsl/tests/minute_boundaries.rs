use open_bsl::{Engine, Value};

#[test]
fn minute_boundaries_match_the_complete_native_receipt() {
    let engine = Engine::builder().build().unwrap();
    let result = engine
        .new_state()
        .exec(
            r#"
            Исходная = Дата(2024, 2, 17, 13, 24, 35);
            Возврат НачалоМинуты(Исходная) = Дата(2024, 2, 17, 13, 24, 0)
                И КонецМинуты(Исходная) = Дата(2024, 2, 17, 13, 24, 59)
                И BegOfMinute(Исходная) = НачалоМинуты(Исходная)
                И EndOfMinute(Исходная) = КонецМинуты(Исходная)
                И НачалоМинуты(Дата(1, 1, 1)) = Дата(1, 1, 1)
                И КонецМинуты(Дата(1, 1, 1)) = Дата(1, 1, 1, 0, 0, 59);
            "#,
        )
        .unwrap();
    assert_eq!(result, Value::Boolean(true));
}
