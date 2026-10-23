use std::process::Command;

#[test]
fn reached_text_extraction_forms_compile_and_fail_catchably() {
    let script = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/text_extraction_reached.bsl"
    );
    let output = Command::new(env!("CARGO_BIN_EXE_bsl-cli"))
        .arg(script)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"CAUGHT_0\nCAUGHT_1\n");
}
