use open_bsl::Engine;

#[test]
fn data_composition_parameter_matches_the_measured_value_behavior() {
    let engine = Engine::builder().build().unwrap();
    let module = engine
        .compile(
            r#"
Параметр1 = Новый ПараметрКомпоновкиДанных("Период");
Параметр2 = New DataCompositionParameter("период");
Параметр3 = Новый ПараметрКомпоновкиДанных("Организация");
Если Строка(ТипЗнч(Параметр1)) <> "Data composition parameter"
    Или Строка(Параметр1) <> "Период"
    Или Не ЗначениеЗаполнено(Параметр1)
    Или Параметр1 <> Параметр2
    Или Параметр1 = Параметр3 Тогда
    ВызватьИсключение "data-composition-parameter";
КонецЕсли;
"#,
        )
        .unwrap();
    engine.new_state().run(&module).unwrap();
}

#[test]
fn data_composition_parameter_requires_its_measured_name_argument() {
    let engine = Engine::builder().build().unwrap();
    assert!(
        engine
            .compile("Параметр = Новый ПараметрКомпоновкиДанных();")
            .is_err()
    );
}

#[test]
fn empty_data_composition_parameter_matches_the_native_receipt() {
    let engine = Engine::builder().build().unwrap();
    let result = engine
        .new_state()
        .exec(
            r#"
            Параметр = Новый ПараметрКомпоновкиДанных("");
            Возврат Строка(Параметр) = ""
                И Не ЗначениеЗаполнено(Параметр)
                И Параметр = New DataCompositionParameter("");
            "#,
        )
        .unwrap();
    assert_eq!(result, open_bsl::Value::Boolean(true));
}
