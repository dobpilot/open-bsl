use open_bsl::{Engine, Value};

#[test]
fn choice_parameter_constructor_resolves_and_reports_missing_value_model() {
    let engine = Engine::builder().build().unwrap();
    engine
        .compile("Значение = Новый ПараметрВыбора(\"Отбор.X\", 1);")
        .unwrap();
    engine
        .compile("Value = New ChoiceParameter(\"Filter.X\", 1);")
        .unwrap();
    assert!(
        engine
            .compile("Значение = Новый ПараметрВыбора(\"Отбор.X\");")
            .is_err()
    );

    let result = engine
        .new_state()
        .exec(
            "Попытка\nНовый ПараметрВыбора(\"Отбор.X\", 1);\nИсключение\nВозврат Истина;\nКонецПопытки;\nВозврат Ложь;",
        )
        .unwrap();
    assert_eq!(result, Value::Boolean(true));
}
