use open_bsl::{Engine, Value};

#[test]
fn choice_parameter_link_resolves_documented_forms_without_a_form_model() {
    let engine = Engine::builder().build().unwrap();
    for source in [
        "Новый СвязьПараметраВыбора(\"Отбор.Владелец\", \"Путь\")",
        "New ChoiceParameterLink(\"Filter.Owner\", \"Path\", Неопределено)",
    ] {
        engine
            .compile(&format!(
                "Процедура НеВызывается()\nСвязь = {source};\nКонецПроцедуры"
            ))
            .unwrap();
    }
    assert_eq!(
        engine
            .new_state()
            .exec(
                "Попытка\nСвязь = Новый СвязьПараметраВыбора(\"Отбор.Владелец\", \"Путь\");\nИсключение\nВозврат Истина;\nКонецПопытки;\nВозврат Ложь;",
            )
            .unwrap(),
        Value::Boolean(true)
    );
    assert!(
        engine
            .compile("Связь = Новый СвязьПараметраВыбора(\"Имя\");")
            .is_err()
    );
    assert!(
        engine
            .compile("Связь = Новый СвязьПараметраВыбора(1, 2, 3, 4);")
            .is_err()
    );
}
