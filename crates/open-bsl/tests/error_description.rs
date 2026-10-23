use open_bsl::{BslString, Engine, Value};

#[test]
fn error_description_resolves_but_does_not_return_stale_exception_text() {
    let engine = Engine::builder().build().unwrap();
    for name in ["ОписаниеОшибки", "ErrorDescription"] {
        let module = engine.compile(&format!("Возврат {name}();")).unwrap();
        assert_eq!(
            engine.new_state().run(&module).unwrap(),
            Value::Str(BslString::from_str(""))
        );
    }

    let error = engine
        .compile("Возврат ErrorDescription(1);")
        .err()
        .expect("функция не принимает аргументы");
    assert!(error.to_string().contains("аргумент"), "{error}");
}
