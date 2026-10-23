use open_bsl::{Engine, Value};

#[test]
fn form_data_to_value_resolves_and_fails_explicitly_without_form_data() {
    let engine = Engine::builder().build().unwrap();
    engine
        .compile(
            "Процедура НеВызывается()\nЗначение = ДанныеФормыВЗначение(Неопределено, Тип(\"Структура\"));\nКонецПроцедуры\nВозврат 1;",
        )
        .unwrap();
    engine
        .compile("Значение = FormDataToValue(Неопределено, Тип(\"Структура\"));")
        .unwrap();
    assert!(
        engine
            .compile("Значение = ДанныеФормыВЗначение();")
            .is_err()
    );
    assert!(
        engine
            .compile("Значение = FormDataToValue(1, 2, 3);")
            .is_err()
    );
    assert_eq!(
        engine
            .new_state()
            .exec(
                "Попытка\nДанныеФормыВЗначение(Неопределено, Тип(\"Структура\"));\nИсключение\nВозврат Истина;\nКонецПопытки;\nВозврат Ложь;",
            )
            .unwrap(),
        Value::Boolean(true)
    );
}
