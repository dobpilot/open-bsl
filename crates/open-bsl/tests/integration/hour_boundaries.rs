use open_bsl::{Engine, Value};

#[test]
fn hour_boundaries_match_the_platform_measurement() {
    let result = Engine::builder()
        .build()
        .unwrap()
        .new_state()
        .exec(
            r#"
Исходная = Дата(2024, 2, 17, 13, 24, 35);
Возврат НачалоЧаса(Исходная) = Дата(2024, 2, 17, 13, 0, 0)
    И КонецЧаса(Исходная) = Дата(2024, 2, 17, 13, 59, 59)
    И BegOfHour(Исходная) = НачалоЧаса(Исходная)
    И EndOfHour(Исходная) = КонецЧаса(Исходная)
    И НачалоЧаса(Дата(1, 1, 1)) = Дата(1, 1, 1)
    И КонецЧаса(Дата(1, 1, 1)) = Дата(1, 1, 1, 0, 59, 59);
"#,
        )
        .unwrap();
    assert_eq!(result, Value::Boolean(true));
}
