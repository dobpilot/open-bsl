use std::process::Command;

#[test]
fn zip_os_utf8_encoding_matches_the_native_enum_control() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let output = Command::new(env!("CARGO_BIN_EXE_bsl-cli"))
        .arg(root.join("docs/research/fixtures/zip-os-utf8-encoding/measure.bsl"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    // Тип имеет одно имя в двух языках интерфейса; остальные строки
    // сравниваются с нативным результатом без преобразования.
    let actual = String::from_utf8(output.stdout)
        .unwrap()
        .replace("КодировкаИменФайловВZipФайле", "FileNamesEncodingInZipFile");
    assert_eq!(
        actual.as_bytes(),
        include_bytes!(
            "../../../../docs/research/fixtures/zip-os-utf8-encoding/measure.platform.txt"
        )
    );
}
