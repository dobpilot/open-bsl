use std::process::Command;

#[test]
fn legacy_xml_canonicalization_matches_the_native_algorithm_control() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let output = Command::new(env!("CARGO_BIN_EXE_bsl-cli"))
        .arg(root.join("docs/research/fixtures/legacy-xml-canonicalization/measure.bsl"))
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
            "../../../docs/research/fixtures/legacy-xml-canonicalization/measure.platform.txt"
        )
    );
}

#[test]
fn legacy_xml_canonicalization_matches_native_lifecycle_and_arity() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let directory = root.join("docs/research/fixtures/legacy-xml-canonicalization");
    for name in ["contract", "arity", "defaults"] {
        let output = Command::new(env!("CARGO_BIN_EXE_bsl-cli"))
            .arg(directory.join(format!("measure-{name}.bsl")))
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{name}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            output.stdout,
            std::fs::read(directory.join(format!("measure-{name}.platform.txt"))).unwrap(),
            "{name}"
        );
    }
}
