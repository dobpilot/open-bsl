use std::process::Command;

#[test]
fn internet_mail_text_type_matches_the_platform_oracle() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let script = root.join("tests/conformance/measure/1c/measure-internet-mail-text-type.bsl");
    let output = Command::new(env!("CARGO_BIN_EXE_bsl-cli"))
        .arg(script)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let actual = String::from_utf8(output.stdout).unwrap();
    assert_eq!(
        actual,
        include_str!(
            "../../../../tests/conformance/measure/1c/measure-internet-mail-text-type.platform.txt"
        )
    );
}
