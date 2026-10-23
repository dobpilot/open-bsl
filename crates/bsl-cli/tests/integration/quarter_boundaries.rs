use std::process::Command;

use open_bsl::Engine;

#[test]
fn quarter_boundaries_match_the_platform_oracle() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let script = root.join("tests/conformance/measure/1c/measure-quarter-boundaries.bsl");
    let output = Command::new(env!("CARGO_BIN_EXE_bsl-cli"))
        .arg(script)
        .output()
        .expect("запуск bsl-cli");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        output.stdout,
        include_bytes!(
            "../../../../tests/conformance/measure/1c/measure-quarter-boundaries.platform.txt"
        )
    );
}

#[test]
fn quarter_boundaries_require_one_date() {
    let engine = Engine::builder().build().unwrap();
    assert!(engine.compile("Результат = НачалоКвартала();").is_err());
    assert!(
        engine
            .compile("Result = EndOfQuarter(Date(2024, 1, 1), 2);")
            .is_err()
    );
}
