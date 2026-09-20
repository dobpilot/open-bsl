//! Сверка корневых форм и объекта Файл с серверным oracle 1С.

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

struct WorkingDirectories(Vec<PathBuf>);

impl WorkingDirectories {
    fn create(&mut self, label: &str, nonce: u128) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "open-bsl-file-create-directory-roots-{label}-{}-{nonce}",
            std::process::id()
        ));
        std::fs::create_dir(&path).unwrap();
        self.0.push(path.clone());
        path
    }
}

impl Drop for WorkingDirectories {
    fn drop(&mut self) {
        for path in &self.0 {
            let _ = std::fs::remove_dir_all(path);
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

fn run_in(args: &[&std::ffi::OsStr], cwd: &Path) -> String {
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
fn directory_roots_and_file_objects_match_native_in_every_execution_mode() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let source = root.join("tests/conformance/measure/filesystem/file-create-directory-roots.bsl");
    let oracle = include_str!(
        "../../../tests/conformance/measure/filesystem/file-create-directory-roots.platform.txt"
    );
    let expected = canonical_output(oracle);
    assert_eq!(expected.len(), 12, "десять проб и два граничных маркера");

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
            "open-bsl-file-create-directory-roots-{}-{nonce}.bslc",
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

fn canonical_async_output(text: &str) -> Vec<String> {
    canonical_output(text)
        .into_iter()
        .map(|line| {
            line.replace("promise=Promise", "promise=Обещание")
                .replace("type=String", "type=Строка")
                .replace("type=File", "type=Файл")
        })
        .collect()
}

#[test]
fn async_directory_roots_keep_values_and_identity_in_every_execution_mode() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let source =
        root.join("tests/conformance/measure/filesystem/file-create-directory-roots-async.bsl");
    let oracle = include_str!(
        "../../../tests/conformance/measure/filesystem/file-create-directory-roots-async.platform.txt"
    );
    let expected = canonical_async_output(oracle);
    assert_eq!(expected.len(), 12, "десять проб и два граничных маркера");

    let nonce = SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let mut bytecode_files = BytecodeFiles(Vec::new());
    let mut working_directories = WorkingDirectories(Vec::new());
    for optimize in [false, true] {
        let cwd = working_directories.create(&usize::from(optimize).to_string(), nonce);
        let direct = if optimize {
            run_in(&["--optimize".as_ref(), source.as_os_str()], &cwd)
        } else {
            run_in(&[source.as_os_str()], &cwd)
        };
        assert_eq!(canonical_async_output(&direct), expected);

        let bytecode = std::env::temp_dir().join(format!(
            "open-bsl-file-create-directory-roots-async-{}-{nonce}.bslc",
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
            canonical_async_output(&run_in(
                &["--run-bytecode".as_ref(), bytecode.as_os_str()],
                &cwd,
            )),
            expected
        );
    }
}

#[test]
fn composite_relative_forms_match_native_without_losing_the_async_string() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let source = root.join(
        "tests/conformance/measure/filesystem/file-create-directory-relative-forms-async.bsl",
    );
    let oracle = include_str!(
        "../../../tests/conformance/measure/filesystem/file-create-directory-relative-forms-async.platform.txt"
    );
    let expected = canonical_output(oracle);
    assert_eq!(expected.len(), 14, "12 проб и два граничных маркера");

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
            "open-bsl-file-create-directory-relative-{}-{nonce}.bslc",
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
