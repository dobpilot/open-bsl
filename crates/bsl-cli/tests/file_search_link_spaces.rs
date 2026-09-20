//! Реальное дерево сочетает пробельные имена, ссылки и оборванные ссылки.

#![cfg(unix)]

use std::{fs, os::unix::fs::symlink, path::PathBuf, process::Command, time::SystemTime};

struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

fn canonical_output(text: &str) -> Vec<&str> {
    text.trim_start_matches('\u{feff}')
        .lines()
        .map(str::trim_end)
        .filter(|line| !line.starts_with("context."))
        .collect()
}

fn run_cli(args: &[&std::ffi::OsStr]) -> String {
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
fn links_with_space_names_match_native_in_every_execution_mode() {
    let root = std::env::temp_dir().join(format!(
        "open-bsl-find-link-spaces-test-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&root).unwrap();
    let scratch = Scratch(root);
    for directory in [
        "targets/linked target",
        "late space/plain dir",
        "first space/plain dir",
    ] {
        fs::create_dir_all(scratch.0.join(directory)).unwrap();
    }
    for file in [
        "targets/linked target/inside space.txt",
        "targets/linked target/inside.bin",
        "late space/after space.txt",
        "late space/middle.txt",
        "late space/plain dir/child space.txt",
        "late space/before.txt",
        "first space/after space.txt",
        "first space/middle.txt",
        "first space/plain dir/child space.txt",
        "first space/before.txt",
    ] {
        fs::write(scratch.0.join(file), b"preserved").unwrap();
    }
    symlink(
        "../targets/linked target",
        scratch.0.join("late space/space link dir"),
    )
    .unwrap();
    symlink(
        "absent target",
        scratch.0.join("late space/broken link.txt"),
    )
    .unwrap();
    symlink(
        "absent target",
        scratch.0.join("first space/broken link.txt"),
    )
    .unwrap();
    symlink(
        "../targets/linked target",
        scratch.0.join("first space/space link dir"),
    )
    .unwrap();

    let original =
        include_str!("../../../tests/conformance/measure/filesystem/file-search-link-spaces.bsl");
    let assignment = original
        .lines()
        .find(|line| line.trim_start().starts_with("КореньПробельныхСсылок = "))
        .unwrap();
    let replacement = format!(
        "КореньПробельныхСсылок = \"{}\";",
        scratch.0.to_str().unwrap().replace('"', "\"\"")
    );
    let source = original.replacen(assignment, &replacement, 1);
    let script = scratch.0.join("probe.bsl");
    fs::write(&script, source).unwrap();
    let expected = canonical_output(include_str!(
        "../../../tests/conformance/measure/filesystem/file-search-link-spaces.platform.txt"
    ));
    assert_eq!(expected.len(), 66, "64 пробы и два граничных маркера");

    for optimize in [false, true] {
        let mut source_args: Vec<&std::ffi::OsStr> = Vec::new();
        if optimize {
            source_args.push("--optimize".as_ref());
        }
        source_args.push(script.as_os_str());
        assert_eq!(canonical_output(&run_cli(&source_args)), expected);

        let bytecode = scratch.0.join(if optimize {
            "probe-optimized.bslc"
        } else {
            "probe.bslc"
        });
        let mut emit_args: Vec<&std::ffi::OsStr> = Vec::new();
        if optimize {
            emit_args.push("--optimize".as_ref());
        }
        emit_args.extend([
            "--emit-bytecode".as_ref(),
            script.as_os_str(),
            bytecode.as_os_str(),
        ]);
        run_cli(&emit_args);
        assert_eq!(
            canonical_output(&run_cli(&["--run-bytecode".as_ref(), bytecode.as_os_str()])),
            expected
        );
    }
}
