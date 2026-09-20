//! Целостность нативного свидетельства, не проверка реализации Begin в VM.

use std::collections::BTreeMap;

const RAW: &str =
    include_str!("../../../tests/conformance/measure/filesystem/file-begin-arity.platform.txt");

#[test]
fn native_begin_arity_distinguishes_call_errors_from_delayed_operation_errors() {
    let mut rows = BTreeMap::new();
    for line in RAW.trim_start_matches('\u{feff}').lines() {
        let (id, value) = line.split_once('\t').unwrap();
        assert!(rows.insert(id, value).is_none(), "повтор {id}");
    }
    assert_eq!(rows["file.begin"], "file-begin-arity-1");
    assert_eq!(rows["file.end"], "file-begin-arity-1");
    assert_eq!(rows["begin.ready"], "1");
    assert_eq!(rows["begin.expected"], "36");
    assert_eq!(rows["begin.delivered"], "36");
    assert_eq!(rows["begin.valid"], "26");
    assert!(!rows.contains_key("file.setup_error"));
    assert!(!rows.contains_key("file.cleanup_error"));

    let mut calls = 0;
    let mut callbacks = 0;
    let mut call_errors = 0;
    let mut callback_errors = 0;
    for (method, result_type, mode) in [
        ("exists", "Boolean", 0),
        ("isdir", "Boolean", 0),
        ("isfile", "Boolean", 0),
        ("hidden", "Boolean", 0),
        ("readonly", "Boolean", 0),
        ("size", "Number", 0),
        ("time", "Date", 0),
        ("utc", "Date", 0),
        ("sethidden", "", 1),
        ("setreadonly", "", 1),
        ("settime", "", 1),
        ("setutc", "", 1),
        ("init", "File", 2),
    ] {
        for language in ["ru", "en"] {
            let cases: &[&str] = if mode == 0 {
                &["valid", "none", "extra"]
            } else {
                &["valid", "none", "extra", "missing_value", "bad_value"]
            };
            for case in cases {
                let prefix = format!("begin.{method}.{language}.{case}");
                let call_id = format!("{prefix}.call");
                let callback_id = format!("{prefix}.callback");
                calls += 1;
                if ["none", "extra", "missing_value"].contains(case) {
                    call_errors += 1;
                    assert_eq!(rows[call_id.as_str()], "error|path_changed=0", "{prefix}");
                    assert!(!rows.contains_key(callback_id.as_str()), "{prefix}");
                    let detail = format!("{call_id}.detail");
                    assert_eq!(
                        rows[detail.as_str()],
                        if *case == "extra" {
                            "Too many actual parameters"
                        } else {
                            "Not enough actual parameters"
                        },
                        "{prefix}"
                    );
                    continue;
                }
                assert_eq!(
                    rows[call_id.as_str()],
                    if mode == 2 {
                        "returned|path_changed=1"
                    } else {
                        "returned|path_changed=0"
                    },
                    "{prefix}"
                );
                assert!(!rows.contains_key(format!("{call_id}.detail").as_str()));
                callbacks += 1;
                if *case == "bad_value" && mode == 1 {
                    callback_errors += 1;
                    assert_eq!(
                        rows[callback_id.as_str()],
                        "error|returned=1|standard=1",
                        "{prefix}"
                    );
                    assert_eq!(
                        rows[format!("{callback_id}.detail").as_str()],
                        "Type mismatch (parameter number '1')",
                        "{prefix}"
                    );
                } else {
                    let expected = if mode == 1 {
                        "ok|argc=1|returned=1".to_owned()
                    } else {
                        format!("ok|argc=2|returned=1|type={result_type}")
                    };
                    assert_eq!(rows[callback_id.as_str()], expected, "{prefix}");
                    assert!(!rows.contains_key(format!("{callback_id}.detail").as_str()));
                }
            }
        }
    }
    assert_eq!(
        (calls, call_errors, callbacks, callback_errors),
        (98, 62, 36, 8)
    );
    // Все строки распределены: события, детали ошибок, контекст и маркеры.
    assert_eq!(
        rows.len(),
        calls + call_errors + callbacks + callback_errors + 7
    );
}
