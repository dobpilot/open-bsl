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
fn str_occurrence_count_matches_the_measured_surface() {
    run_in_every_mode(
        r#"
Если СтрЧислоВхождений("абабаб", "аб") <> 3
    Или СтрЧислоВхождений("аааа", "аа") <> 2
    Или СтрЧислоВхождений("абабаба", "аба") <> 2
    Или СтрЧислоВхождений("аб", "") <> 0
    Или СтрЧислоВхождений("АаА", "а") <> 1
    Или StrOccurrenceCount("абаб", "аб") <> 2
    Или СтрЧислоВхождений(111, "1") <> 3
    Или СтрЧислоВхождений("111", 1) <> 3
    Или СтрЧислоВхождений(Неопределено, "а") <> 0 Тогда
    ВызватьИсключение "str-occurrence-count";
КонецЕсли;
"#,
    );
}

#[test]
fn str_occurrence_count_requires_exactly_two_arguments() {
    let engine = Engine::builder().build().unwrap();
    assert!(
        engine
            .compile("Результат = СтрЧислоВхождений(\"а\");")
            .is_err()
    );
    assert!(
        engine
            .compile("Результат = StrOccurrenceCount(\"а\", \"б\", \"в\");")
            .is_err()
    );
}
