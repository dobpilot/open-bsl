use open_bsl::{Engine, Value};

#[test]
fn range_constructor_accepts_measured_forms_without_fabricating_a_value() {
    let engine = Engine::builder().build().unwrap();
    for source in [
        "Значение = Новый Диапазон(1);",
        "Значение = Новый Диапазон(1, 3);",
        "Значение = Новый Диапазон(, Дата(2024, 1, 31));",
        "Значение = Новый Диапазон(Дата(2024, 1, 1), );",
        "Value = New Range(1, 3);",
    ] {
        engine.compile(source).unwrap();
    }
    for source in [
        "Значение = Новый Диапазон();",
        "Value = New Range(1, 2, 3);",
    ] {
        assert!(engine.compile(source).is_err());
    }

    let result = engine
        .new_state()
        .exec(
            "Попытка\nНовый Диапазон(1, 3);\nИсключение\nВозврат Истина;\nКонецПопытки;\nВозврат Ложь;",
        )
        .unwrap();
    assert_eq!(result, Value::Boolean(true));
}
