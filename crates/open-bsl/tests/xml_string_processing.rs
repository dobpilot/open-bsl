use open_bsl::Engine;

#[test]
fn xml_string_processing_manager_matches_the_measured_positions() {
    let engine = Engine::builder().build().unwrap();
    let module = engine
        .compile(
            r#"
Если Строка(ТипЗнч(ОбработкаСтрокиXML)) <> "XML string processing manager"
    Или Строка(ОбработкаСтрокиXML) <> "XMLStringProcessingManager"
    Или ОбработкаСтрокиXML.НайтиНедопустимыеСимволыXML("ABC") <> 0
    Или ОбработкаСтрокиXML.НайтиНедопустимыеСимволыXML("A" + Символ(1) + "B") <> 2
    Или ОбработкаСтрокиXML.НайтиНедопустимыеСимволыXML("A" + Символ(1) + "B", 3) <> 0
    Или ОбработкаСтрокиXML.НайтиНедопустимыеСимволыXML("😀" + Символ(1)) <> 3
    Или XMLStringProcessing.FindDisallowedXMLCharacters("A" + Символ(1) + "B", 2) <> 2 Тогда
    ВызватьИсключение "xml-string-processing";
КонецЕсли;
"#,
        )
        .unwrap();
    engine.new_state().run(&module).unwrap();
}

#[test]
fn xml_string_processing_manager_rejects_measured_invalid_arities() {
    let engine = Engine::builder().build().unwrap();
    for source in [
        "Результат = ОбработкаСтрокиXML.НайтиНедопустимыеСимволыXML();",
        "Результат = ОбработкаСтрокиXML.НайтиНедопустимыеСимволыXML(\"ABC\", 1, 1);",
    ] {
        if let Ok(module) = engine.compile(source) {
            assert!(engine.new_state().run(&module).is_err());
        }
    }
}
