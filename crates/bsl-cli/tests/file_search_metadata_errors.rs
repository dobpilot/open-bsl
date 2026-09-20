//! Реальные `PermissionDenied` и `FilesystemLoop` сверяются с oracle 1С.

#![cfg(unix)]

use std::{
    fs,
    os::unix::{fs::PermissionsExt, fs::symlink},
    path::PathBuf,
    process::Command,
    time::SystemTime,
};

struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let denied = self.0.join("permission/denied dir");
        fs::set_permissions(&denied, fs::Permissions::from_mode(0o700)).unwrap();
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
fn measured_metadata_errors_match_native_in_every_execution_mode() {
    let root = std::env::temp_dir().join(format!(
        "open-bsl-find-metadata-errors-test-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&root).unwrap();
    let scratch = Scratch(root);
    fs::create_dir_all(scratch.0.join("permission/denied dir")).unwrap();
    fs::create_dir(scratch.0.join("loop")).unwrap();
    for file in [
        "permission/before.txt",
        "permission/denied dir/inside.txt",
        "permission/after.txt",
        "loop/before.txt",
        "loop/after.txt",
    ] {
        fs::write(scratch.0.join(file), b"preserved").unwrap();
    }
    symlink("error loop", scratch.0.join("loop/error loop")).unwrap();
    fs::set_permissions(
        scratch.0.join("permission/denied dir"),
        fs::Permissions::from_mode(0o0),
    )
    .unwrap();

    let original = include_str!(
        "../../../tests/conformance/measure/filesystem/file-search-metadata-errors.bsl"
    );
    let assignment = original
        .lines()
        .find(|line| line.trim_start().starts_with("КореньОшибокМетаданных = "))
        .unwrap();
    let replacement = format!(
        "КореньОшибокМетаданных = \"{}\";",
        scratch.0.to_str().unwrap().replace('"', "\"\"")
    );
    let source = original.replacen(assignment, &replacement, 1);
    let script = scratch.0.join("probe.bsl");
    fs::write(&script, source).unwrap();
    let expected = canonical_output(include_str!(
        "../../../tests/conformance/measure/filesystem/file-search-metadata-errors.platform.txt"
    ));
    assert_eq!(expected.len(), 26, "24 пробы и два граничных маркера");

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
