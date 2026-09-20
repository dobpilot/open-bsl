//! Клиентские типы аргумента `СоздатьКаталогАсинх` по oracle 1С.

#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::SystemTime;

struct Scratch {
    directories: Vec<PathBuf>,
    files: Vec<PathBuf>,
}

impl Scratch {
    fn directory(&mut self, label: &str, nonce: u128) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "open-bsl-file-create-directory-types-{label}-{}-{nonce}",
            std::process::id()
        ));
        std::fs::create_dir(&path).unwrap();
        self.directories.push(path.clone());
        path
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        for path in &self.files {
            let _ = std::fs::remove_file(path);
        }
        for path in &self.directories {
            let _ = std::fs::remove_dir_all(path);
        }
    }
}

fn async_summary(text: &str) -> Vec<String> {
    text.trim_start_matches('\u{feff}')
        .lines()
        .map(|line| line.trim_end_matches('\r'))
        .filter(|line| line.starts_with("create.type."))
        .map(|line| {
            let (id, values) = line.split_once('\t').unwrap();
            let async_result = values.split_once("|async=").unwrap().1;
            if async_result.starts_with("ok:") {
                let same = async_result.split_once(":same=").unwrap().1;
                format!("{id}\tok:same={same}")
            } else {
                format!("{id}\t{async_result}")
            }
        })
        .collect()
}

fn run(args: &[&std::ffi::OsStr], cwd: &Path) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_bsl-cli"))
        .args(args)
        .current_dir(cwd)
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
fn async_directory_creation_formats_types_and_returns_the_original_value() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let source =
        root.join("tests/conformance/measure/filesystem/file-create-directory-types-async.bsl");
    let oracle = include_str!(
        "../../../tests/conformance/measure/filesystem/file-create-directory-types-async.platform.txt"
    );
    let expected = async_summary(oracle);
    assert_eq!(expected.len(), 20, "20 измеренных типов");

    let nonce = SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let mut scratch = Scratch {
        directories: Vec::new(),
        files: Vec::new(),
    };
    for optimize in [false, true] {
        let label = usize::from(optimize);
        let source_cwd = scratch.directory(&format!("source-{label}"), nonce);
        let direct = if optimize {
            run(&["--optimize".as_ref(), source.as_os_str()], &source_cwd)
        } else {
            run(&[source.as_os_str()], &source_cwd)
        };
        assert_eq!(async_summary(&direct), expected);

        let bytecode = std::env::temp_dir().join(format!(
            "open-bsl-file-create-directory-types-{}-{label}-{nonce}.bslc",
            std::process::id()
        ));
        scratch.files.push(bytecode.clone());
        let emit_cwd = scratch.directory(&format!("emit-{label}"), nonce);
        let emit = if optimize {
            run(
                &[
                    "--optimize".as_ref(),
                    "--emit-bytecode".as_ref(),
                    source.as_os_str(),
                    bytecode.as_os_str(),
                ],
                &emit_cwd,
            )
        } else {
            run(
                &[
                    "--emit-bytecode".as_ref(),
                    source.as_os_str(),
                    bytecode.as_os_str(),
                ],
                &emit_cwd,
            )
        };
        assert!(emit.is_empty());
        let bytecode_cwd = scratch.directory(&format!("bytecode-{label}"), nonce);
        assert_eq!(
            async_summary(&run(
                &["--run-bytecode".as_ref(), bytecode.as_os_str()],
                &bytecode_cwd,
            )),
            expected
        );
    }
}
