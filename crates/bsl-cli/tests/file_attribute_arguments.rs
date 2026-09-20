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
            "open-bsl-attribute-oracle-{}-{nonce}",
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

fn measured_results(text: &str) -> Vec<&str> {
    text.trim_start_matches('\u{feff}')
        .lines()
        .filter(|line| {
            // Полные сообщения сохранены в raw oracle. Здесь проверяются
            // принятие аргумента, стадия ошибки и состояние, не её локализация.
            !line.starts_with("context.")
                && !line
                    .split_once('\t')
                    .is_some_and(|(id, _)| id.ends_with(".detail"))
        })
        .collect()
}

#[test]
fn attribute_arguments_match_sync_and_async_oracles_in_both_formats() {
    for (name, raw) in [
        (
            "file-attribute-arguments.bsl",
            include_str!(
                "../../../tests/conformance/measure/filesystem/file-attribute-arguments.platform.txt"
            ),
        ),
        (
            "file-attribute-arguments-async.bsl",
            include_str!(
                "../../../tests/conformance/measure/filesystem/file-attribute-arguments-async.platform.txt"
            ),
        ),
    ] {
        assert_oracle(name, raw, 226);
    }
}

#[test]
fn attribute_error_order_matches_the_client_oracle_in_both_formats() {
    assert_oracle(
        "file-attribute-access-order.bsl",
        include_str!(
            "../../../tests/conformance/measure/filesystem/file-attribute-access-order.platform.txt"
        ),
        18,
    );
}

#[test]
fn stream_resize_edges_match_the_server_oracle_in_both_formats() {
    assert_oracle(
        "stream-resize-edges.bsl",
        include_str!(
            "../../../tests/conformance/measure/filesystem/stream-resize-edges.platform.txt"
        ),
        18,
    );
}

#[test]
fn recursion_arguments_match_the_sync_oracle_in_both_formats() {
    assert_oracle(
        "file-search-recursion-arguments.bsl",
        include_str!(
            "../../../tests/conformance/measure/filesystem/file-search-recursion-arguments.platform.txt"
        ),
        58,
    );
}

#[test]
fn recursion_arguments_match_the_async_oracle_in_both_formats() {
    assert_oracle(
        "file-search-recursion-arguments-async.bsl",
        include_str!(
            "../../../tests/conformance/measure/filesystem/file-search-recursion-arguments-async.platform.txt"
        ),
        58,
    );
}

#[test]
fn file_date_precision_matches_the_server_oracle_in_both_formats() {
    assert_oracle(
        "file-date-precision.bsl",
        include_str!(
            "../../../tests/conformance/measure/filesystem/file-date-precision.platform.txt"
        ),
        170,
    );
}

#[test]
fn file_date_components_match_the_server_oracle_in_both_formats() {
    assert_oracle(
        "file-date-components.bsl",
        include_str!(
            "../../../tests/conformance/measure/filesystem/file-date-components.platform.txt"
        ),
        44,
    );
}

#[test]
fn file_time_string_arguments_match_the_server_oracle_in_both_formats() {
    assert_oracle(
        "file-time-string-arguments.bsl",
        include_str!(
            "../../../tests/conformance/measure/filesystem/file-time-string-arguments.platform.txt"
        ),
        80,
    );
}

fn normalized_zero_date_results(text: &str) -> Vec<String> {
    measured_results(text)
        .into_iter()
        .filter(|line| {
            !line.starts_with("constructor.zero_fields\t")
                && !line.starts_with("local_min.string\t")
                && !line.starts_with("local_min.format\t")
                && !line.starts_with("utc_min.string\t")
                && !line.starts_with("utc_min.format\t")
        })
        .map(|line| {
            line.replace("Строка:", "String:")
                .replace("Число:", "Number:")
                .replace("Булево:", "Boolean:")
                .replace('\u{a0}', "")
                .replace(',', ".")
        })
        .collect()
}

#[test]
fn wrapped_file_dates_match_the_measured_semantics_in_both_formats() {
    let name = "file-zero-date.bsl";
    let raw =
        include_str!("../../../tests/conformance/measure/filesystem/file-zero-date.platform.txt");
    let expected = normalized_zero_date_results(raw);
    assert_eq!(expected.len(), 62);
    let scratch = Scratch::new();
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/conformance/measure/filesystem")
        .join(name);
    for optimize in [false, true] {
        for bytecode in [false, true] {
            let listing = scratch.0.join("zero-date.bslc");
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
                normalized_zero_date_results(&actual),
                expected,
                "{name}, optimize={optimize}, bytecode={bytecode}"
            );
        }
    }
}

#[test]
fn wrapped_file_date_order_matches_the_server_oracle_in_both_formats() {
    assert_oracle(
        "file-zero-date-order.bsl",
        include_str!(
            "../../../tests/conformance/measure/filesystem/file-zero-date-order.platform.txt"
        ),
        8,
    );
}

#[test]
fn wrapped_file_dates_match_the_async_server_oracle_in_both_formats() {
    assert_oracle(
        "file-zero-date-async.bsl",
        include_str!(
            "../../../tests/conformance/measure/filesystem/file-zero-date-async.platform.txt"
        ),
        6,
    );
}

#[test]
fn wrapped_file_dates_match_the_begin_server_oracle_in_both_formats() {
    assert_oracle(
        "file-zero-date-begin.bsl",
        include_str!(
            "../../../tests/conformance/measure/filesystem/file-zero-date-begin.platform.txt"
        ),
        6,
    );
}

#[test]
fn wrapped_file_date_boundaries_match_the_server_oracle_in_both_formats() {
    assert_number_oracle(
        "file-zero-date-week.bsl",
        include_str!(
            "../../../tests/conformance/measure/filesystem/file-zero-date-week.platform.txt"
        ),
        38,
    );
}

#[test]
fn date_shift_limits_match_the_server_oracle_in_both_formats() {
    assert_number_oracle(
        "date-shift-limits.bsl",
        include_str!(
            "../../../tests/conformance/measure/filesystem/date-shift-limits.platform.txt"
        ),
        16,
    );
}

fn assert_oracle(name: &str, raw: &str, rows: usize) {
    assert_oracle_impl(name, raw, rows, false);
}

fn assert_number_oracle(name: &str, raw: &str, rows: usize) {
    assert_oracle_impl(name, raw, rows, true);
}

fn assert_oracle_impl(name: &str, raw: &str, rows: usize, normalize_numbers: bool) {
    let scratch = Scratch::new();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/conformance/measure/filesystem");
    let source = root.join(name);
    let normalize = |line: &str| {
        if normalize_numbers {
            line.replace('\u{a0}', "").replace(',', ".")
        } else {
            line.to_owned()
        }
    };
    let expected: Vec<_> = measured_results(raw).into_iter().map(normalize).collect();
    assert_eq!(expected.len(), rows, "{name}");
    for optimize in [false, true] {
        for bytecode in [false, true] {
            let listing = scratch.0.join("probe.bslc");
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
            let actual: Vec<_> = measured_results(&actual)
                .into_iter()
                .map(normalize)
                .collect();
            assert_eq!(actual.len(), expected.len(), "{name}");
            for (actual, expected) in actual.iter().zip(&expected) {
                assert_eq!(
                    actual, expected,
                    "{name}, optimize={optimize}, bytecode={bytecode}"
                );
            }
        }
    }
}
