use open_bsl::{Engine, Value};

#[test]
fn copy_form_data_resolves_without_fabricating_a_boolean_result() {
    let engine = Engine::builder().build().unwrap();
    for name in ["КопироватьДанныеФормы", "copyformdata"] {
        engine
            .compile(&format!(
                "Процедура НеВызывается()\nУспех = {name}(Неопределено, Неопределено);\nКонецПроцедуры"
            ))
            .unwrap();
    }
    assert_eq!(
        engine
            .new_state()
            .exec(
                "Попытка\nУспех = КопироватьДанныеФормы(Неопределено, Неопределено);\nИсключение\nВозврат Истина;\nКонецПопытки;\nВозврат Ложь;",
            )
            .unwrap(),
        Value::Boolean(true)
    );
    assert!(engine.compile("Успех = CopyFormData(1);").is_err());
    assert!(engine.compile("Успех = CopyFormData(1, 2, 3);").is_err());
}
