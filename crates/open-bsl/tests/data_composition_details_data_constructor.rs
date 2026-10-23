use open_bsl::{Engine, Value};

#[test]
fn data_composition_details_data_resolves_without_fabricated_value() {
    let engine = Engine::builder().build().unwrap();
    for source in [
        "Новый ДанныеРасшифровкиКомпоновкиДанных",
        "Новый datacompositiondetailsdata()",
    ] {
        engine
            .compile(&format!(
                "Процедура НеВызывается()\nЗначение = {source};\nКонецПроцедуры\nВозврат 1;"
            ))
            .unwrap();
        assert_eq!(
            engine
                .new_state()
                .exec(&format!(
                    "Попытка\nЗначение = {source};\nИсключение\nВозврат Истина;\nКонецПопытки;\nВозврат Ложь;"
                ))
                .unwrap(),
            Value::Boolean(true)
        );
    }
    assert!(
        engine
            .compile("Значение = Новый DataCompositionDetailsData(1);")
            .is_err()
    );
}
