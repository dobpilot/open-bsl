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
        .filter_map(|line| {
            let line = line.trim_end_matches('\r');
            if line.starts_with("context.") {
                return None;
            }
            let Some((id, value)) = line.split_once('\t') else {
                return Some(line.to_owned());
            };
            if !id.starts_with("ctor.more.") {
                return Some(line.to_owned());
            }
            assert!(value.starts_with("ok|equals=1|"), "{line}");
            if id == "ctor.more.value_list" {
                assert_eq!(
                    value,
                    "ok|equals=1|input=|full=|parent=|name=|base=|extension="
                );
            }
            Some(format!("{id}\tok|equals=1"))
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
fn measured_object_arguments_use_their_string_in_every_execution_mode() {
    let root = std::env::temp_dir().join(format!(
        "open-bsl-file-constructor-more-types-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&root).unwrap();
    let scratch = Scratch(root);
    let script = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/conformance/measure/filesystem/file-constructor-more-types.bsl");
    let expected = canonical_output(include_str!(
        "../../../tests/conformance/measure/filesystem/file-constructor-more-types.platform.txt"
    ));
    assert_eq!(expected.len(), 13, "11 проб и два граничных маркера");

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
