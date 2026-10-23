use std::process::Command;

use bsl_rt::BuiltinFn;

#[test]
fn base64_binary_data_matches_the_platform_oracle() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let script = root.join("tests/conformance/measure/1c/measure-base64-binary-data.bsl");
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
            "../../../tests/conformance/measure/1c/measure-base64-binary-data.platform.txt"
        )
    );
}

#[test]
fn base64_binary_data_name_and_arity_are_exact() {
    let function = BuiltinFn::lookup("getbinarydatafrombase64binarydata").unwrap();
    assert_eq!(function, BuiltinFn::GetBinaryDataFromBase64BinaryData);
    assert_eq!(function.arity_range(), (1, 1));
}
