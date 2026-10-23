use std::process::Command;

#[test]
fn reached_xsl_transform_name_compiles_and_fails_catchably() {
    let script = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/xsl_transform_reached.bsl"
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
    assert_eq!(output.stdout, b"CAUGHT\n");
}
