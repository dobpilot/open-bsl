use open_bsl::{Engine, Value};

#[test]
fn measured_text_writer_constructor_shapes_resolve() {
    let engine = Engine::builder().build().unwrap();
    let unopened = engine
        .compile("Писатель = Новый ЗаписьТекста(); Писатель.Закрыть(); Возврат Истина;")
        .unwrap();
    assert_eq!(
        engine.new_state().run(&unopened).unwrap(),
        Value::Boolean(true)
    );

    for constructor in [
        "Новый ЗаписьТекста(\"x\", \"UTF-16\")",
        "Новый TextWriter(\"x\", \"UTF-8\", \"\\n\")",
        "Новый ЗаписьТекста(\"x\", , , Истина)",
    ] {
        let module = engine
            .compile(&format!(
                "Попытка З = {constructor}; Исключение Возврат Истина; КонецПопытки; Возврат Ложь;"
            ))
            .unwrap();
        assert_eq!(
            engine.new_state().run(&module).unwrap(),
            Value::Boolean(true)
        );
    }

    let error = engine
        .compile("Возврат Новый ЗаписьТекста(1, 2, 3, 4, 5, 6);")
        .err()
        .expect("шесть аргументов запрещены");
    assert!(error.to_string().contains("6"), "{error}");
}

#[test]
fn two_argument_utf8_writer_uses_the_state_file_system() {
    let path = std::env::temp_dir().join(format!(
        "open-bsl-text-writer-two-arguments-{}.txt",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path);
    let engine = Engine::builder().build().unwrap();
    let source = format!(
        "Писатель = Новый TextWriter(\"{}\", КодировкаТекста.UTF8); Писатель.Записать(\"А\"); Писатель.Закрыть();",
        path.display()
    );
    let module = engine.compile(&source).unwrap();
    engine.new_state().run(&module).unwrap();
    assert_eq!(
        std::fs::read(&path).unwrap(),
        [0xef, 0xbb, 0xbf, 0xd0, 0x90]
    );
    std::fs::remove_file(path).unwrap();
}

#[test]
fn unsupported_writer_options_fail_before_file_creation() {
    let path = std::env::temp_dir().join(format!(
        "open-bsl-text-writer-unsupported-{}.txt",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path);
    let engine = Engine::builder().build().unwrap();
    for options in ["\"UTF-16\"", "\"UTF-8\", \"\\n\""] {
        let source = format!(
            "Попытка Писатель = Новый ЗаписьТекста(\"{}\", {options}); Исключение Возврат Истина; КонецПопытки; Возврат Ложь;",
            path.display()
        );
        let module = engine.compile(&source).unwrap();
        assert_eq!(
            engine.new_state().run(&module).unwrap(),
            Value::Boolean(true)
        );
        assert!(!path.exists(), "отказ не должен создавать файл");
    }
}
