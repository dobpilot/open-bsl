use std::process::Command;

#[test]
fn all_files_mask_matches_the_local_platform_oracle() {
    if !cfg!(unix) {
        return;
    }
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let script = root.join("tests/conformance/measure/1c/measure-get-all-files-mask.bsl");
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
        include_bytes!(
            "../../../tests/conformance/measure/1c/measure-get-all-files-mask.platform.txt"
        )
    );
}
