#![cfg(target_os = "linux")]

use std::os::unix::fs::PermissionsExt;
use std::{fs, path::PathBuf, process::Command};

struct Scratch {
    root: PathBuf,
    denied: PathBuf,
}

impl Scratch {
    fn new() -> Self {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "open-bsl-search-access-order-{}-{nonce}",
            std::process::id()
        ));
        let denied = root.join("denied");
        fs::create_dir_all(&denied).unwrap();
        fs::set_permissions(&denied, fs::Permissions::from_mode(0o0)).unwrap();
        Self { root, denied }
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::set_permissions(&self.denied, fs::Permissions::from_mode(0o700));
        fs::remove_dir_all(&self.root).unwrap();
    }
}

fn structural_lines(text: &str) -> Vec<&str> {
    text.trim_start_matches('\u{feff}')
        .lines()
        .filter(|line| {
            !line
                .split_once('\t')
                .is_some_and(|(id, _)| id.ends_with(".detail"))
        })
        .collect()
}

#[test]
fn search_checks_recursion_before_access_and_hides_root_denial_in_every_mode() {
    let scratch = Scratch::new();
    let original =
        include_str!("../../../tests/conformance/measure/filesystem/file-search-access-order.bsl");
    let source_text = original.replace(
        "/tmp/open-bsl-search-access-order-denied",
        scratch.denied.to_str().unwrap(),
    );
    let source = scratch.root.join("search-access-order.bsl");
    fs::write(&source, source_text).unwrap();
    let expected = structural_lines(include_str!(
        "../../../tests/conformance/measure/filesystem/file-search-access-order.platform.txt"
    ));
    assert_eq!(expected.len(), 10);

    for optimize in [false, true] {
        for bytecode in [false, true] {
            let listing = scratch.root.join("search-access-order.bslc");
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
            let actual = String::from_utf8(output.stdout).unwrap();
            assert_eq!(
                structural_lines(&actual),
                expected,
                "optimize={optimize}, bytecode={bytecode}"
            );
        }
    }
}
