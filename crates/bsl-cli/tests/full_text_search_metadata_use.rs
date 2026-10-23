use std::process::Command;

#[test]
fn full_text_search_metadata_use_matches_the_platform_oracle() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let script =
        root.join("tests/conformance/measure/1c/measure-full-text-search-metadata-use.bsl");
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
            "../../../tests/conformance/measure/1c/measure-full-text-search-metadata-use.platform.txt"
        )
    );
}
