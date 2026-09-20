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
            let (id, value) = line.split_once('\t')?;
            if id.starts_with("file.") {
                return Some(line.to_owned());
            }
            Some(format!(
                "{id}\t{}",
                if value.starts_with("ok") {
                    "ok"
                } else {
                    "error"
                }
            ))
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
fn measured_notification_name_types_agree_in_every_execution_mode() {
    let root = std::env::temp_dir().join(format!(
        "open-bsl-notification-name-types-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&root).unwrap();
    let scratch = Scratch(root);
    let script = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/conformance/measure/filesystem/notification-name-types.bsl");
    let expected = canonical_output(include_str!(
        "../../../tests/conformance/measure/filesystem/notification-name-types.platform.txt"
    ));
    assert_eq!(expected.len(), 46, "44 пробы и два маркера");

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
