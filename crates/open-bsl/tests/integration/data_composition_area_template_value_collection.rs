use open_bsl::{Engine, Value};

#[test]
fn value_collection_area_template_resolves_but_does_not_fabricate_a_value() {
    let engine = Engine::builder().build().unwrap();
    engine
        .compile("Значение = Новый МакетКоллекцииЗначенийОбластиКомпоновкиДанных;")
        .unwrap();
    engine
        .compile("Value = New DataCompositionAreaTemplateValueCollection;")
        .unwrap();
    assert!(
        engine
            .compile("Значение = Новый МакетКоллекцииЗначенийОбластиКомпоновкиДанных(1);")
            .is_err()
    );

    let result = engine
        .new_state()
        .exec(
            "Попытка\nНовый МакетКоллекцииЗначенийОбластиКомпоновкиДанных;\nИсключение\nВозврат Истина;\nКонецПопытки;\nВозврат Ложь;",
        )
        .unwrap();
    assert_eq!(result, Value::Boolean(true));
}
