use std::process::Command;

#[test]
fn html_dom_string_output_matches_native_platform_controls() {
    for name in [
        "write",
        "structure-format-en",
        "empty-title",
        "writer-type-format-en",
    ] {
        check_control(name);
    }
}

#[test]
fn html_dom_tree_and_node_types_match_native_platform_controls() {
    check_control("tree-contract");
}

fn check_control(name: &str) {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let directory = root.join("docs/research/fixtures/html-document");
    let output = Command::new(env!("CARGO_BIN_EXE_bsl-cli"))
        .arg(directory.join(format!("{name}.bsl")))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{name}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        output.stdout,
        std::fs::read(directory.join(format!("{name}.platform.txt"))).unwrap(),
        "{name}"
    );
}
