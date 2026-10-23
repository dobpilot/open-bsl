use open_bsl::Engine;

#[test]
fn data_lock_mode_has_the_complete_measured_surface() {
    let engine = Engine::builder().build().unwrap();
    let module = engine
        .compile(
            r#"
Если Строка(РежимБлокировкиДанных.Разделяемый) <> "Shared"
    Или Строка(РежимБлокировкиДанных.Исключительный) <> "Exceptional"
    Или Строка(ТипЗнч(РежимБлокировкиДанных.Разделяемый)) <> "DataLockMode"
    Или DataLockMode.Shared <> РежимБлокировкиДанных.Разделяемый
    Или DataLockMode.Exclusive <> РежимБлокировкиДанных.Исключительный Тогда
    ВызватьИсключение "data-lock-mode";
КонецЕсли;
"#,
        )
        .unwrap();
    engine.new_state().run(&module).unwrap();
}
