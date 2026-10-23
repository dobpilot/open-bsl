use std::process::Command;

#[test]
fn reached_empty_date_patterns_match_the_native_receipt() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let output = Command::new(env!("CARGO_BIN_EXE_bsl-cli"))
        .arg(root.join("docs/research/fixtures/business-process-object/empty-date-format.bsl"))
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
            "../../../../docs/research/fixtures/business-process-object/empty-date-format.platform.txt"
        )
    );
}
