use open_bsl::{Engine, Value};

#[test]
fn move_file_has_measured_arity_and_does_not_bypass_the_state_file_system() {
    let engine = Engine::builder().build().unwrap();
    let directory = std::env::temp_dir();
    let source = directory.join(format!("open-bsl-move-source-{}.tmp", std::process::id()));
    let destination = directory.join(format!("open-bsl-move-target-{}.tmp", std::process::id()));
    let _ = std::fs::remove_file(&destination);
    std::fs::write(&source, b"source").unwrap();

    for name in ["ПереместитьФайл", "MoveFile"] {
        let source_code = format!(
            "Попытка {name}(\"{}\", \"{}\"); Исключение Возврат Истина; КонецПопытки; Возврат Ложь;",
            source.display(),
            destination.display()
        );
        let module = engine.compile(&source_code).unwrap();
        assert_eq!(
            engine.new_state().run(&module).unwrap(),
            Value::Boolean(true)
        );
        assert_eq!(std::fs::read(&source).unwrap(), b"source");
        assert!(!destination.exists());
    }

    for call in ["ПереместитьФайл(1)", "MoveFile(1, 2, 3)"] {
        let error = engine
            .compile(&format!("{call};"))
            .err()
            .expect("неверная арность запрещена");
        assert!(error.to_string().contains("аргумент"), "{error}");
    }
    std::fs::remove_file(source).unwrap();
}
