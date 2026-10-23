use open_bsl::{Engine, Value};

#[test]
fn end_of_week_resolves_but_requires_the_session_calendar() {
    let engine = Engine::builder().build().unwrap();
    engine
        .compile("Значение = КонецНедели(Дата(2024, 1, 1));")
        .unwrap();
    engine
        .compile("Value = EndOfWeek(Date(2024, 1, 1));")
        .unwrap();
    assert!(engine.compile("Значение = КонецНедели();").is_err());

    let result = engine
        .new_state()
        .exec(
            "Попытка\nЗначение = КонецНедели(Дата(2024, 1, 1));\nИсключение\nВозврат Истина;\nКонецПопытки;\nВозврат Ложь;",
        )
        .unwrap();
    assert_eq!(result, Value::Boolean(true));
}

#[test]
fn week_of_year_resolves_but_requires_the_session_calendar() {
    let engine = Engine::builder().build().unwrap();
    engine
        .compile("Значение = НеделяГода(Дата(2024, 1, 1));")
        .unwrap();
    engine
        .compile("Value = WeekOfYear(Date(2024, 1, 1));")
        .unwrap();
    assert!(engine.compile("Значение = НеделяГода();").is_err());

    let result = engine
        .new_state()
        .exec(
            "Попытка\nЗначение = НеделяГода(Дата(2024, 1, 1));\nИсключение\nВозврат Истина;\nКонецПопытки;\nВозврат Ложь;",
        )
        .unwrap();
    assert_eq!(result, Value::Boolean(true));
}
