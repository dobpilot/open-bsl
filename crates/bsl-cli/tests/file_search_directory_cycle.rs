//! Циклическая ссылка на каталог сверяется с серверным oracle 1С.

#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::SystemTime;

struct BytecodeFiles(Vec<PathBuf>);

impl Drop for BytecodeFiles {
    fn drop(&mut self) {
        for path in &self.0 {
            let _ = std::fs::remove_file(path);
        }
    }
}

fn canonical_output(text: &str) -> Vec<&str> {
    text.trim_start_matches('\u{feff}')
        .lines()
        .map(|line| line.trim_end_matches('\r'))
        .filter(|line| !line.starts_with("context."))
        .collect()
}

fn run(args: &[&std::ffi::OsStr]) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_bsl-cli"))
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

#[test]
fn directory_cycle_matches_native_in_every_execution_mode() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let source = root.join("tests/conformance/measure/filesystem/file-search-directory-cycle.bsl");
    let oracle = include_str!(
        "../../../tests/conformance/measure/filesystem/file-search-directory-cycle.platform.txt"
    );
    let expected = canonical_output(oracle);
    assert_eq!(expected.len(), 3, "одна проба и два граничных маркера");

    let nonce = SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let mut bytecode_files = BytecodeFiles(Vec::new());
    for optimize in [false, true] {
        let direct = if optimize {
            run(&["--optimize".as_ref(), source.as_os_str()])
        } else {
            run(&[source.as_os_str()])
        };
        assert_eq!(canonical_output(&direct), expected);

        let bytecode = std::env::temp_dir().join(format!(
            "open-bsl-file-search-directory-cycle-{}-{nonce}.bslc",
            usize::from(optimize)
        ));
        bytecode_files.0.push(bytecode.clone());
        let emit = if optimize {
            run(&[
                "--optimize".as_ref(),
                "--emit-bytecode".as_ref(),
                source.as_os_str(),
                bytecode.as_os_str(),
            ])
        } else {
            run(&[
                "--emit-bytecode".as_ref(),
                source.as_os_str(),
                bytecode.as_os_str(),
            ])
        };
        assert!(emit.is_empty());
        assert_eq!(
            canonical_output(&run(&["--run-bytecode".as_ref(), bytecode.as_os_str()])),
            expected
        );
    }
}
