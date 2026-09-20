//! Реальное дерево и исходный oracle 1С для поиска пробельных имён.
//! Порядок внутри результата зависит от host; здесь сравнивается мультимножество.
//! Порядок обхода и сохранение дубликатов отдельно проверены виртуальной ФС.

#![cfg(unix)]

use std::{fs, path::PathBuf, process::Command, time::SystemTime};

struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

fn canonical_output(text: &str) -> Vec<String> {
    text.trim_start_matches('\u{feff}')
        .lines()
        .filter(|line| !line.starts_with("context."))
        .map(|line| {
            let (id, value) = line.trim_end_matches('\r').split_once('\t').unwrap();
            if value.starts_with("ok|count=") {
                let mut fields: Vec<_> = value.split('|').collect();
                fields[2..].sort_unstable();
                format!("{id}\t{}", fields.join("|"))
            } else {
                line.trim_end_matches('\r').to_owned()
            }
        })
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
fn synchronous_and_async_search_match_native_space_names_in_every_execution_mode() {
    let root = std::env::temp_dir().join(format!(
        "open-bsl-search-spaces-test-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&root).unwrap();
    let scratch = Scratch(root);
    for directory in [
        "alone/padded ",
        "middle/padded ",
        "twins/padded ",
        "twins/padded",
        "file",
        "file-twins/padded",
        "leading/ padded",
        "tab/padded\t",
    ] {
        fs::create_dir_all(scratch.0.join(directory)).unwrap();
    }
    let files = [
        "alone/padded /raw-only.txt",
        "middle/padded /raw-only.txt",
        "middle/before.txt",
        "middle/after.txt",
        "twins/padded /raw-only.txt",
        "twins/padded/trimmed-only.txt",
        "file/padded ",
        "file/before.txt",
        "file/after.txt",
        "file-twins/padded ",
        "file-twins/padded/trimmed-only.txt",
        "leading/ padded/leading-only.txt",
        "tab/padded\t/tab-only.txt",
    ];
    for path in files {
        fs::write(scratch.0.join(path), b"preserved").unwrap();
    }
    for (source, oracle) in [
        (
            include_str!("../../../tests/conformance/measure/filesystem/file-search-spaces.bsl"),
            include_str!(
                "../../../tests/conformance/measure/filesystem/file-search-spaces.platform.txt"
            ),
        ),
        (
            include_str!(
                "../../../tests/conformance/measure/filesystem/file-search-spaces-async.bsl"
            ),
            include_str!(
                "../../../tests/conformance/measure/filesystem/file-search-spaces-async.platform.txt"
            ),
        ),
    ] {
        let assignment = source
            .lines()
            .find(|line| line.trim_start().starts_with("КореньПоискаПробелов = "))
            .unwrap();
        let replacement = format!(
            "КореньПоискаПробелов = \"{}\";",
            scratch.0.to_str().unwrap().replace('"', "\"\"")
        );
        let source = source.replacen(assignment, &replacement, 1);
        let script = scratch.0.join("probe.bsl");
        let bytecode = scratch.0.join("probe.bslc");
        fs::write(&script, source).unwrap();
        let expected = canonical_output(oracle);
        assert_eq!(expected.len(), 128, "126 проб и два граничных маркера");
        for args in [
            vec![script.as_os_str()],
            vec!["--optimize".as_ref(), script.as_os_str()],
        ] {
            assert_eq!(canonical_output(&run_cli(&args)), expected);
        }
        run_cli(&[
            "--emit-bytecode".as_ref(),
            script.as_os_str(),
            bytecode.as_os_str(),
        ]);
        assert_eq!(
            canonical_output(&run_cli(&["--run-bytecode".as_ref(), bytecode.as_os_str()])),
            expected
        );
    }
    for path in files {
        assert_eq!(fs::read(scratch.0.join(path)).unwrap(), b"preserved");
    }
}
