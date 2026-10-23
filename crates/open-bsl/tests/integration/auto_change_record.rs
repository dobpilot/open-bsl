use open_bsl::Engine;

#[test]
fn auto_change_record_has_the_complete_measured_surface() {
    for optimizations in [
        bsl_compiler::Optimizations::default(),
        bsl_compiler::Optimizations::all(),
    ] {
        let engine = Engine::builder()
            .optimizations(optimizations)
            .build()
            .unwrap();
        let module = engine
            .compile(
                r#"
Если Строка(АвтоРегистрацияИзменений.Запретить) <> "Disable"
    Или Строка(АвтоРегистрацияИзменений.Разрешить) <> "Enable"
    Или Строка(ТипЗнч(АвтоРегистрацияИзменений.Запретить)) <> "AutoChangeRecord"
    Или AutoChangeRecord.Deny <> АвтоРегистрацияИзменений.Запретить
    Или AutoChangeRecord.Allow <> АвтоРегистрацияИзменений.Разрешить Тогда
    ВызватьИсключение "auto-change-record";
КонецЕсли;
"#,
            )
            .unwrap();
        engine.new_state().run(&module).unwrap();
    }
}
