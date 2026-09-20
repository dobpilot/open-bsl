#![cfg(target_os = "linux")]

use std::{path::PathBuf, process::Command};

struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "open-bsl-search-sfile-directory-{}-{nonce}",
            std::process::id()
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn sfile_directory_matches_the_server_oracle_in_both_formats() {
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/conformance/measure/filesystem/file-search-sfile-directory.bsl");
    let expected = include_str!(
        "../../../tests/conformance/measure/filesystem/file-search-sfile-directory.platform.txt"
    );
    let scratch = Scratch::new();

    for optimize in [false, true] {
        for bytecode in [false, true] {
            let listing = scratch.0.join("search-sfile-directory.bslc");
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
