use open_bsl::Engine;

fn run_in_every_mode(source: &str) {
    for optimizations in [
        bsl_compiler::Optimizations::default(),
        bsl_compiler::Optimizations::all(),
    ] {
        let engine = Engine::builder()
            .optimizations(optimizations)
            .build()
            .unwrap();
        let compiled = engine.compile(source).unwrap();
        for bytecode in [false, true] {
            let module = if bytecode {
                engine.load_bytecode(&compiled.bytecode().unwrap()).unwrap()
            } else {
                engine.compile(source).unwrap()
            };
            engine.new_state().run(&module).unwrap();
        }
    }
}

#[test]
fn str_compare_matches_the_measured_surface() {
    run_in_every_mode(
        r#"
Если СтрСравнить("Строка", "строка") <> 0
    Или СтрСравнить("а", "б") <> -1
    Или СтрСравнить("б", "а") <> 1
    Или СтрСравнить("а", "аа") <> -1
    Или СтрСравнить("е", "ё") <> -1
    Или StrCompare("a", "а") <> -1
    Или СтрСравнить(1, "1") <> 0
    Или СтрСравнить(Неопределено, "") <> 0 Тогда
    ВызватьИсключение "str-compare";
КонецЕсли;
"#,
    );
}

#[test]
fn str_compare_requires_exactly_two_arguments() {
    let engine = Engine::builder().build().unwrap();
    assert!(engine.compile("Результат = СтрСравнить(\"а\");").is_err());
    assert!(
        engine
            .compile("Результат = StrCompare(\"а\", \"б\", \"в\");")
            .is_err()
    );
}
