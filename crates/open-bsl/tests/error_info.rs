use open_bsl::Engine;

#[test]
fn error_info_is_a_stable_snapshot_in_interpreter_and_jit() {
    let engine = Engine::builder().build().unwrap();
    let module = engine
        .compile(
            "Сохраненная = Неопределено;\n\
             Попытка\n\
               ВызватьИсключение \"connector-boom\";\n\
             Исключение\n\
               Сохраненная = ИнформацияОбОшибке();\n\
             КонецПопытки;\n\
             Если ТипЗнч(Сохраненная) <> Тип(\"ИнформацияОбОшибке\") Тогда\n\
               ВызватьИсключение \"type\";\n\
             КонецЕсли;\n\
             Текст = ПодробноеПредставлениеОшибки(Сохраненная);\n\
             Если СтрНайти(Текст, \"connector-boom\") = 0 Тогда\n\
               ВызватьИсключение \"detail\";\n\
             КонецЕсли;\n\
             Возврат Текст;",
        )
        .unwrap();

    for jit in [false, true] {
        let value = engine
            .state_builder()
            .jit(jit)
            .build()
            .run(&module)
            .unwrap();
        assert_eq!(value.to_string(), "connector-boom");
    }
}

#[test]
fn error_info_outside_a_handler_has_the_measured_default() {
    let engine = Engine::builder().build().unwrap();
    let module = engine
        .compile("Возврат ПодробноеПредставлениеОшибки(ИнформацияОбОшибке());")
        .unwrap();

    assert_eq!(
        engine.new_state().run(&module).unwrap().to_string(),
        "Unexpected error"
    );
}
