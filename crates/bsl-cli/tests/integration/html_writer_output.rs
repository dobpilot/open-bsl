use std::process::Command;

#[test]
fn registered_html_writer_replays_exact_native_string_output() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let directory = root.join("docs/research/fixtures/html-document");
    for name in [
        "write",
        "empty-title",
        "structure-format-en",
        "writer-type-format-en",
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_bsl-cli"))
            .arg(directory.join(format!("{name}.bsl")))
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            output.stdout,
            std::fs::read(directory.join(format!("{name}.platform.txt"))).unwrap()
        );
    }
}

#[test]
fn html_writer_constructor_aliases_and_zero_arity_use_the_same_type() {
    let engine = open_bsl::Engine::builder().build().unwrap();
    for name in ["ЗаписьHTML", "hTmLwRiTeR"] {
        let source = format!(
            "Писатель = Новый {name}; Писатель.УстановитьСтроку(); Возврат Писатель.Закрыть();"
        );
        assert_eq!(
            engine.new_state().exec(&source).unwrap(),
            open_bsl::Value::Str("".into())
        );
        assert!(
            engine
                .compile(&format!("Писатель = Новый {name}(1);"))
                .is_err()
        );
    }
    assert_eq!(
        engine
            .new_state()
            .exec("Возврат Тип(\"ЗаписьHTML\") = Тип(\"HTMLWriter\");")
            .unwrap(),
        open_bsl::Value::Boolean(true)
    );
}
