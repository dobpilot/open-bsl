use std::process::Command;

#[test]
fn structure_index_assignment_matches_native_control() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let output = Command::new(env!("CARGO_BIN_EXE_bsl-cli"))
        .arg(root.join("docs/research/fixtures/structure-index-write-native-1015/control.bsl"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let expected = include_str!(
        "../../../../docs/research/fixtures/structure-index-write-native-1015/platform.oracle"
    )
    .trim_start_matches('\u{feff}')
    .replace("\r\n", "\n");
    assert_eq!(
        String::from_utf8(output.stdout).unwrap().trim(),
        expected.trim()
    );
}
