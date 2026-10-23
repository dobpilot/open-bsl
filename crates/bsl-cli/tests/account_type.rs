use std::process::Command;

#[test]
fn account_type_members_match_the_platform_oracle() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let script = root.join("tests/conformance/measure/1c/measure-account-type.bsl");
    let output = Command::new(env!("CARGO_BIN_EXE_bsl-cli"))
        .arg(script)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        output.stdout,
        include_bytes!("../../../tests/conformance/measure/1c/measure-account-type.platform.txt")
    );
}

#[test]
fn account_type_preserves_variable_priority_and_unknown_member_errors() {
    let engine = open_bsl::Engine::builder().build().unwrap();
    for source in [
        "Перем ВидСчета; ВидСчета = 17; Возврат ВидСчета;",
        "Функция Проверить() Перем AccountType; AccountType = 17; Возврат AccountType; КонецФункции Возврат Проверить();",
    ] {
        let module = engine.compile(source).unwrap();
        assert_eq!(
            engine.new_state().run(&module).unwrap(),
            open_bsl::Value::number_from_i64(17)
        );
    }
    for source in [
        "Возврат ВидСчета.Несуществующий;",
        "Возврат AccountType.NotAMember;",
    ] {
        assert!(engine.compile(source).is_err());
    }
}
