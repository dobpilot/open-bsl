use open_bsl::Engine;

#[test]
fn empty_picture_matches_the_measured_value_behavior() {
    let engine = Engine::builder().build().unwrap();
    let module = engine
        .compile(
            r#"
Картинка1 = Новый Картинка();
Картинка2 = New Picture();
Если Строка(ТипЗнч(Картинка1)) <> "Картинка"
    Или Строка(Картинка1) <> "Картинка"
    Или ЗначениеЗаполнено(Картинка1)
    Или Картинка1 <> Картинка2 Тогда
    ВызватьИсключение "empty-picture";
КонецЕсли;
"#,
        )
        .unwrap();
    engine.new_state().run(&module).unwrap();
}
