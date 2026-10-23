use open_bsl::{Engine, Value};

#[test]
fn collection_template_constructors_keep_measured_names_and_arity() {
    let engine = Engine::builder().build().unwrap();
    for (russian, english) in [
        (
            "МакетЗаголовкаКоллекцииЗначенийОбластиКомпоновкиДанных",
            "DataCompositionAreaTemplateValueCollectionHeader",
        ),
        ("МакетКомпоновкиДанных", "DataCompositionTemplate"),
    ] {
        engine
            .compile(&format!("Значение = Новый {russian};"))
            .unwrap();
        engine.compile(&format!("Value = New {english};")).unwrap();
        assert!(
            engine
                .compile(&format!("Значение = Новый {russian}(1);"))
                .is_err()
        );
        let result = engine
            .new_state()
            .exec(&format!(
                "Попытка\nНовый {russian};\nИсключение\nВозврат Истина;\nКонецПопытки;\nВозврат Ложь;"
            ))
            .unwrap();
        assert_eq!(result, Value::Boolean(true));
    }
}
