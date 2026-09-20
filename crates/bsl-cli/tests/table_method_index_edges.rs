use std::path::PathBuf;
use std::process::Command;

struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "open-bsl-table-method-index-{}-{}",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn table_method_indices_match_the_server_oracle_in_every_execution_mode() {
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/conformance/measure/filesystem/notification-table-index-edges.bsl");
    let expected = include_str!(
        "../../../tests/conformance/measure/filesystem/notification-table-index-edges.platform.txt"
    );
    let scratch = Scratch::new();

    for optimize in [false, true] {
        for bytecode in [false, true] {
            let listing = scratch.0.join("table-method-index.bslc");
            if bytecode {
                let mut emit = Command::new(env!("CARGO_BIN_EXE_bsl-cli"));
                if optimize {
                    emit.arg("--optimize");
                }
                let output = emit
                    .arg("--emit-bytecode")
                    .arg(&source)
                    .arg(&listing)
                    .output()
                    .unwrap();
                assert!(
                    output.status.success(),
                    "{}",
                    String::from_utf8_lossy(&output.stderr)
                );
            }

            let mut run = Command::new(env!("CARGO_BIN_EXE_bsl-cli"));
            if bytecode {
                run.arg("--run-bytecode").arg(&listing);
            } else {
                if optimize {
                    run.arg("--optimize");
                }
                run.arg(&source);
            }
            let output = run.output().unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert_eq!(
                String::from_utf8(output.stdout).unwrap(),
                expected,
                "optimize={optimize}, bytecode={bytecode}"
            );
        }
    }
}
