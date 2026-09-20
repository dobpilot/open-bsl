//! Проверка целостности нативного свидетельства; не приёмка CreateTempFile.

use std::collections::BTreeMap;

const VISIBILITY: &str =
    include_str!("../../../tests/conformance/measure/filesystem/file-temp-visibility.platform.txt");

#[test]
fn native_temporary_sharing_follows_the_file_across_hard_links_and_rename() {
    let oracle = include_str!(
        "../../../tests/conformance/measure/filesystem/file-temp-sharing.platform.txt"
    );
    assert_eq!(
        oracle.lines().collect::<Vec<_>>(),
        [
            "file.begin\ttemp-sharing-1",
            "temp.sharing.initial\terror",
            "temp.sharing.alias\terror",
            "temp.sharing.moved\terror",
            "temp.sharing.replacement\tread=0",
            "temp.sharing.name\toriginal",
            "temp.sharing.closed_alias\tread=2",
            "temp.sharing.closed_moved\tread=2",
            "file.end\ttemp-sharing-1",
        ]
    );
}

#[test]
fn native_temporary_file_visibility_distinguishes_metadata_from_shared_read_access() {
    let mut rows = BTreeMap::new();
    for line in VISIBILITY.trim_start_matches('\u{feff}').lines() {
        let (id, value) = line.split_once('\t').unwrap();
        assert!(rows.insert(id, value).is_none(), "повтор {id}");
    }
    assert_eq!(rows["file.begin"], "temp-visibility-1");
    assert_eq!(rows["file.end"], "temp-visibility-1");
    for case in ["default", "explicit", "zero", "one", "eight", "large"] {
        for size in [1, 8, 8191, 8192, 8193] {
            let id = format!("temp.visibility.{case}.{size}");
            let first = if size == 1 { 90 } else { 65 };
            assert_eq!(
                rows[id.as_str()],
                format!(
                    "{size}/{size}/{size}/{first}/90|read_error|Error calling constructor (ДвоичныеДанные)|cause=File shared access error '<temp>'"
                ),
                "{id}"
            );
        }
        assert_eq!(
            rows[format!("temp.visibility.{case}.closed").as_str()],
            "read=8193"
        );
    }
    assert_eq!(rows.len(), 38);
}

const PARAMETERS: &str =
    include_str!("../../../tests/conformance/measure/filesystem/file-temp-parameters.platform.txt");

#[test]
fn native_async_temporary_parameters_preserve_identity_and_separate_call_from_await_errors() {
    let oracle =
        include_str!("../../../tests/conformance/measure/filesystem/file-temp-async.platform.txt");
    let mut rows = BTreeMap::new();
    for line in oracle.lines() {
        let (id, value) = line.split_once('\t').unwrap();
        assert!(rows.insert(id, value).is_none(), "повтор {id}");
    }
    assert_eq!(rows["file.begin"], "temp-async-1");
    assert_eq!(rows["file.end"], "temp-async-1");
    let success = "promise=1|ok|file=1|repeat=1|size=0|position=0|rws|exists=1|closed_repeat=1|closed_name=same|closed_exists=1";
    let mut counts = [0; 3];
    for line in PARAMETERS
        .lines()
        .filter(|line| line.starts_with("temp.params."))
    {
        let (id, value) = line.split_once('\t').unwrap();
        let id = id.replacen("temp.params.", "temp.async.", 1);
        let (expected, category) = if id.ends_with(".extra") {
            ("promise=none|error|call", 2)
        } else if value.starts_with("error|") {
            ("promise=1|error|await|repeat=error", 1)
        } else {
            (success, 0)
        };
        assert_eq!(rows[id.as_str()], expected, "{id}");
        counts[category] += 1;
    }
    assert_eq!(counts, [32, 36, 2]);
    assert_eq!(rows.len(), 72);
}

#[test]
fn native_temporary_file_parameters_cover_both_positions_and_languages() {
    let mut rows = BTreeMap::new();
    for line in PARAMETERS.trim_start_matches('\u{feff}').lines() {
        let (id, value) = line.split_once('\t').unwrap();
        assert!(rows.insert(id, value).is_none(), "повтор {id}");
    }
    assert_eq!(rows["file.begin"], "temp-parameters-1");
    assert_eq!(rows["file.end"], "temp-parameters-1");
    assert!(!rows.contains_key("file.setup_error"));
    let success = "ok|File stream|0|0|rws|exists=1|closed_name=same|closed_exists=1";
    let accepted = [
        "zero",
        "one",
        "eight",
        "buffer",
        "limit",
        "above",
        "undefined",
    ];
    let rejected = [
        "null", "negative", "fraction", "string", "false", "true", "date", "array", "struct",
    ];
    let mut counts = [0; 2];
    for language in ["ru", "en"] {
        for case in ["default", "explicit", "extra"] {
            let id = format!("temp.params.{language}.{case}");
            let error = case == "extra";
            assert_eq!(
                rows[id.as_str()],
                if error { "error|call" } else { success }
            );
            counts[usize::from(error)] += 1;
        }
        for position in [1, 2] {
            for (cases, error) in [(&accepted[..], false), (&rejected[..], true)] {
                for case in cases {
                    let id = format!("temp.params.{language}.{position}.{case}");
                    assert_eq!(
                        rows[id.as_str()],
                        if error { "error|call" } else { success },
                        "{id}"
                    );
                    counts[usize::from(error)] += 1;
                }
            }
        }
    }
    assert_eq!(counts, [32, 38]);
    // Независимое чтение остановилось на первой записи во всех шести пробах.
    // Эти строки нельзя считать подтверждением буферизации или содержимого.
    for case in ["default", "explicit", "zero", "one", "eight", "large"] {
        assert_eq!(
            rows[format!("temp.visibility.{case}").as_str()],
            "error|Error calling constructor (ДвоичныеДанные)"
        );
    }
    assert_eq!(rows.len(), 78);
}
