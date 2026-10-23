use open_bsl::Engine;

#[test]
fn type_description_copy_forms_resolve_and_fail_explicitly() {
    let engine = Engine::builder().build().unwrap();
    for source in [
        "Возврат Новый ОписаниеТипов(Неопределено, Неопределено, Неопределено, Неопределено, Неопределено, Неопределено);",
        "Возврат New TypeDescription(Undefined, Undefined, Undefined, Undefined, Undefined, Undefined, Undefined);",
    ] {
        engine.compile(source).unwrap();
        assert!(
            engine
                .new_state()
                .exec(source)
                .unwrap_err()
                .to_string()
                .contains("копирование ОписаниеТипов не реализовано")
        );
    }
    assert!(
        engine
            .compile("Возврат Новый ОписаниеТипов(1, 2, 3, 4, 5, 6, 7, 8);")
            .err()
            .unwrap()
            .to_string()
            .contains("принимает аргументов: 7")
    );
}

#[test]
fn type_description_adjust_value_rejects_two_arguments() {
    let engine = Engine::builder().build().unwrap();
    let error = engine
        .new_state()
        .exec("Возврат (Новый ОписаниеТипов(\"Число\")).ПривестиЗначение(1, 2);")
        .unwrap_err()
        .to_string();
    assert!(error.contains("не больше одного аргумента"), "{error}");
}
