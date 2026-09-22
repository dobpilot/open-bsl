use open_bsl::{Engine, Value};

#[test]
fn date_literal_without_seconds_keeps_hours_and_minutes() {
    for optimizations in [
        bsl_compiler::Optimizations::default(),
        bsl_compiler::Optimizations::all(),
    ] {
        let engine = Engine::builder()
            .optimizations(optimizations)
            .build()
            .unwrap();
        let result = engine
            .new_state()
            .exec(
                r#"
Значение = '202401151230';
Если Год(Значение) = 2024 И Месяц(Значение) = 1 И День(Значение) = 15
    И Час(Значение) = 12 И Минута(Значение) = 30 И Секунда(Значение) = 0 Тогда
    Возврат "ok";
КонецЕсли;
Возврат "неверно";
"#,
            )
            .unwrap();
        let Value::Str(result) = result else {
            panic!("ожидалась строка");
        };
        assert_eq!(result.to_string(), "ok");
    }
}

#[test]
fn invalid_minute_in_date_literal_remains_an_error() {
    let engine = Engine::builder().build().unwrap();
    let error = engine
        .new_state()
        .exec("Возврат '202401151260';")
        .unwrap_err();
    assert!(error.to_string().contains("некорректный литерал даты"));
}

#[test]
fn date_literal_ignores_non_digit_separators() {
    let engine = Engine::builder().build().unwrap();
    let result = engine
        .new_state()
        .exec("Возврат '2024аб01!!15' = '20240115';")
        .unwrap();
    assert_eq!(result, Value::Boolean(true));
}
