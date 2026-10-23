use open_bsl::Engine;

#[test]
fn additional_show_mode_has_the_complete_measured_surface() {
    let engine = Engine::builder().build().unwrap();
    let module = engine
        .compile(
            r#"
Если Строка(ДополнительныйРежимОтображения.НеИспользовать) <> "Не использовать"
    Или Строка(ДополнительныйРежимОтображения.Неактуальность) <> "Неактуальность"
    Или Строка(ТипЗнч(ДополнительныйРежимОтображения.НеИспользовать)) <> "ДополнительныйРежимОтображения"
    Или AdditionalShowMode.DontUse <> ДополнительныйРежимОтображения.НеИспользовать
    Или AdditionalShowMode.Irrelevance <> ДополнительныйРежимОтображения.Неактуальность Тогда
    ВызватьИсключение "additional-show-mode";
КонецЕсли;
"#,
        )
        .unwrap();
    engine.new_state().run(&module).unwrap();
}
