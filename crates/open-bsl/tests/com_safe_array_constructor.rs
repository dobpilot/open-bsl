use open_bsl::{Engine, Value};

#[test]
fn com_safe_array_resolves_variable_dimensions_without_fabricating_com() {
    let engine = Engine::builder().build().unwrap();
    for source in [
        "Новый COMSafeArray(Неопределено)",
        "Новый COMSafeArray(\"VT_VARIANT\", 2)",
        "Новый COMSafeArray(\"VT_I4\", 2, 3)",
    ] {
        engine
            .compile(&format!(
                "Процедура НеВызывается()\nДанные = {source};\nКонецПроцедуры"
            ))
            .unwrap();
    }
    assert_eq!(
        engine
            .new_state()
            .exec(
                "Попытка\nДанные = Новый COMSafeArray(\"VT_VARIANT\", 2);\nИсключение\nВозврат Истина;\nКонецПопытки;\nВозврат Ложь;",
            )
            .unwrap(),
        Value::Boolean(true)
    );
    assert!(engine.compile("Данные = Новый COMSafeArray();").is_err());
}
