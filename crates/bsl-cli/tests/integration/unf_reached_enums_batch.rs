use std::process::Command;

use open_bsl::Engine;

fn assert_platform_oracle(script_name: &str, expected: &[u8]) {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let script = root.join("tests/conformance/measure/1c").join(script_name);
    let output = Command::new(env!("CARGO_BIN_EXE_bsl-cli"))
        .arg(script)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, expected);
}

#[test]
fn xml_reached_unf_enums_match_the_platform_oracle() {
    let oracle = include_bytes!(
        "../../../../tests/conformance/measure/1c/measure-unf-reached-enums-batch.platform.txt"
    );
    assert_platform_oracle("measure-unf-reached-enums-batch.bsl", oracle);

    let mut source = String::from("Процедура НеВызывается()\n");
    for line in std::str::from_utf8(oracle).unwrap().lines() {
        let expression = line.split('\t').next().unwrap();
        source.push_str("ЗначениеПеречисления = ");
        source.push_str(expression);
        source.push_str(";\n");
    }
    source.push_str("КонецПроцедуры\n");
    Engine::builder().build().unwrap().compile(&source).unwrap();
}

#[test]
fn missing_syntax_assistant_aliases_match_the_platform() {
    assert_platform_oracle(
        "measure-unf-enum-alias-gaps.bsl",
        include_bytes!(
            "../../../../tests/conformance/measure/1c/measure-unf-enum-alias-gaps.platform.txt"
        ),
    );
}
