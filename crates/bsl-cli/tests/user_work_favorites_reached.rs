use std::process::Command;

#[test]
fn reached_user_work_favorites_types_compile_and_fail_catchably() {
    let script = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/user_work_favorites_reached.bsl"
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
    assert_eq!(output.stdout, b"CAUGHT_COLLECTION\nCAUGHT_ITEM\n");
}
