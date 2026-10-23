use std::process::Command;

#[test]
fn get_xml_type_matches_native_reader_controls() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for english in [false, true] {
        let mut source = std::fs::read_to_string(
            root.join("docs/research/fixtures/get-xml-type/measure-cases.bsl"),
        )
        .unwrap();
        if english {
            source = source.replace("ПолучитьXMLТип(", "GetXMLType(");
        }
        let path = std::env::temp_dir().join(format!(
            "open-bsl-get-xml-type-{}-{english}.bsl",
            std::process::id()
        ));
        std::fs::write(&path, source).unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_bsl-cli"))
            .arg(&path)
            .output()
            .unwrap();
        std::fs::remove_file(path).unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            output.stdout,
            include_bytes!(
                "../../../../docs/research/fixtures/get-xml-type/measure-cases.platform.txt"
            )
        );
    }
}

#[test]
fn get_xml_type_on_an_attribute_matches_the_native_control() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let output = Command::new(env!("CARGO_BIN_EXE_bsl-cli"))
        .arg(root.join("docs/research/fixtures/get-xml-type/measure-attribute.bsl"))
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
            "../../../../docs/research/fixtures/get-xml-type/measure-attribute.platform.txt"
        )
    );
}
