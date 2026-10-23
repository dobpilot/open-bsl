use open_bsl::Engine;

#[test]
fn document_write_mode_has_the_complete_measured_surface() {
    let engine = Engine::builder().build().unwrap();
    let module = engine
        .compile(
            r#"
Если Строка(РежимЗаписиДокумента.Запись) <> "Record"
    Или Строка(РежимЗаписиДокумента.Проведение) <> "Posting"
    Или Строка(РежимЗаписиДокумента.ОтменаПроведения) <> "Cancel posting"
    Или Строка(ТипЗнч(РежимЗаписиДокумента.Запись)) <> "DocumentWriteMode"
    Или DocumentWriteMode.Write <> РежимЗаписиДокумента.Запись
    Или DocumentWriteMode.Posting <> РежимЗаписиДокумента.Проведение
    Или DocumentWriteMode.UndoPosting <> РежимЗаписиДокумента.ОтменаПроведения Тогда
    ВызватьИсключение "document-write-mode";
КонецЕсли;
"#,
        )
        .unwrap();
    engine.new_state().run(&module).unwrap();
}
