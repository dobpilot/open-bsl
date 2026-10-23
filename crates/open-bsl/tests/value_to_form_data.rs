use open_bsl::{Engine, Value};

#[test]
fn value_to_form_data_resolves_as_a_procedure_without_fabricating_form_data() {
    let engine = Engine::builder().build().unwrap();
    for name in ["ЗначениеВДанныеФормы", "valuetoformdata"] {
        engine
            .compile(&format!(
                "Процедура НеВызывается()\n{name}(Неопределено, Неопределено);\nКонецПроцедуры"
            ))
            .unwrap();
    }
    assert_eq!(
        engine
            .new_state()
            .exec(
                "Попытка\nЗначениеВДанныеФормы(Неопределено, Неопределено);\nИсключение\nВозврат Истина;\nКонецПопытки;\nВозврат Ложь;",
            )
            .unwrap(),
        Value::Boolean(true)
    );
    assert!(
        engine
            .compile("Значение = ValueToFormData(Неопределено, Неопределено);")
            .is_err()
    );
    assert!(
        engine
            .compile("ЗначениеВДанныеФормы(Неопределено);")
            .is_err()
    );
}
