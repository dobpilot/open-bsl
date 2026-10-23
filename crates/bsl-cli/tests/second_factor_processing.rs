use std::process::Command;

#[test]
fn second_factor_processing_matches_the_native_platform_receipt() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let output = Command::new(env!("CARGO_BIN_EXE_bsl-cli"))
        .arg(root.join("docs/research/fixtures/second-factor-template-global/processing-type.bsl"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        output.stdout,
        include_bytes!(
            "../../../docs/research/fixtures/second-factor-template-global/processing-type.platform.txt"
        )
    );
}
