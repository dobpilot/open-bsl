use std::process::Command;

#[test]
fn error_description_replays_native_handler_and_call_scopes() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let directory = root.join("docs/research/fixtures/error-description-context");
    for name in ["measure", "nonlocal-exits", "dynamic", "async"] {
        let output = Command::new(env!("CARGO_BIN_EXE_bsl-cli"))
            .arg(directory.join(format!("{name}.bsl")))
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{name}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            output.stdout,
            std::fs::read(directory.join(format!("{name}.platform.txt"))).unwrap(),
            "{name}"
        );
    }
}

#[test]
fn error_description_aliases_have_zero_arity_and_no_cross_state_context() {
    let engine = open_bsl::Engine::builder().build().unwrap();
    let handled = engine
        .compile("Попытка ВызватьИсключение \"state-probe\"; Исключение Возврат СтрНайти(ОписаниеОшибки(), \"state-probe\") > 0; КонецПопытки;")
        .unwrap();
    assert_eq!(
        engine.new_state().run(&handled).unwrap(),
        open_bsl::Value::Boolean(true)
    );
    for name in ["ОписаниеОшибки", "eRrOrDeScRiPtIoN"] {
        assert_eq!(
            engine
                .new_state()
                .exec(&format!("Возврат {name}();"))
                .unwrap(),
            open_bsl::Value::Str("".into())
        );
        assert!(engine.compile(&format!("{name}(1);")).is_err());
    }
}
