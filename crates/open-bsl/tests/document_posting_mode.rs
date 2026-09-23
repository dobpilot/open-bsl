use open_bsl::Engine;

#[test]
fn document_posting_mode_has_the_complete_measured_surface() {
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
Если Строка(РежимПроведенияДокумента.Оперативный) <> "Real-time"
    Или Строка(РежимПроведенияДокумента.Неоперативный) <> "Regular"
    Или Строка(ТипЗнч(РежимПроведенияДокумента.Оперативный)) <> "DocumentPostingMode"
    Или DocumentPostingMode.RealTime <> РежимПроведенияДокумента.Оперативный
    Или DocumentPostingMode.Regular <> РежимПроведенияДокумента.Неоперативный Тогда
    ВызватьИсключение "document-posting-mode";
КонецЕсли;
"#,
            )
            .unwrap();
        engine.new_state().run(&module).unwrap();
    }
}
