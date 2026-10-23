use std::process::Command;

#[test]
fn xml_canonicalization_matches_the_native_string_control() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let output = Command::new(env!("CARGO_BIN_EXE_bsl-cli"))
        .arg(root.join("docs/research/fixtures/xml-canonicalization/measure.bsl"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        output.stdout,
        include_bytes!("../../../docs/research/fixtures/xml-canonicalization/measure.platform.txt")
    );
}

#[test]
fn xml_canonicalization_matches_the_remaining_native_controls() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for name in [
        "arguments",
        "io",
        "extended",
        "edge-arguments",
        "filter-conversion",
        "empty-document",
        "base-resolution",
        "base-edges",
        "xpath-result",
        "output-errors",
    ] {
        let directory = root.join("docs/research/fixtures/xml-canonicalization");
        let scratch = std::env::temp_dir().join(format!(
            "open-bsl-canonical-control-{}-{name}",
            std::process::id()
        ));
        std::fs::create_dir(&scratch).unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_bsl-cli"))
            .current_dir(&scratch)
            .arg(directory.join(format!("measure-{name}.bsl")))
            .output()
            .unwrap();
        std::fs::remove_dir_all(&scratch).unwrap();
        assert!(
            output.status.success(),
            "{name}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let expected =
            std::fs::read(directory.join(format!("measure-{name}.platform.txt"))).unwrap();
        if name == "io" {
            // Переносы Base64 относятся к представлению двоичных данных,
            // здесь сравниваются канонические байты файла и потока.
            let actual = String::from_utf8(output.stdout)
                .unwrap()
                .replace("\r\n", "");
            let expected = String::from_utf8(expected).unwrap().replace("\r\n", "");
            assert_eq!(actual, expected, "{name}");
        } else if name == "filter-conversion" {
            // Среднее поле показывает локализованную Строка(значение),
            // а контракт канонизации закреплён последним полем.
            let fields = |bytes: Vec<u8>| -> Vec<(String, String)> {
                String::from_utf8(bytes)
                    .unwrap()
                    .lines()
                    .map(|line| {
                        let mut fields = line.split('\t');
                        let id = fields.next().unwrap().to_owned();
                        fields.next().unwrap();
                        (id, fields.next().unwrap().to_owned())
                    })
                    .collect()
            };
            assert_eq!(fields(output.stdout), fields(expected), "{name}");
        } else {
            assert_eq!(output.stdout, expected, "{name}");
        }
    }
}
