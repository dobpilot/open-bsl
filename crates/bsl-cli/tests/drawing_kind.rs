use std::path::PathBuf;
use std::process::Command;

#[test]
fn drawing_kinds_match_the_platform_oracle() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let script = root.join("tests/conformance/measure/1c/measure-drawing-kind.bsl");
    let oracle = root.join("tests/conformance/measure/1c/measure-drawing-kind.platform.txt");
    let output = Command::new(env!("CARGO_BIN_EXE_bsl-cli"))
        .arg(script)
        .output()
        .expect("запуск bsl-cli");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, std::fs::read(oracle).expect("оракул 1С"));
}
