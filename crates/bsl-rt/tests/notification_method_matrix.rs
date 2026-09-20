use std::collections::HashSet;

const SCRIPT: &str = include_str!(
    "../../../tests/conformance/measure/filesystem/notification-core-methods-client.bsl"
);
const RAW: &str = include_str!(
    "../../../tests/conformance/measure/filesystem/notification-core-methods-client.platform.txt"
);

#[test]
fn measured_names_cover_the_complete_builtin_method_table() {
    let names = SCRIPT
        .lines()
        .find_map(|line| {
            line.strip_prefix("ИменаМатрицы = СтрРазделить(\"")
                .and_then(|line| line.strip_suffix("\", \",\");"))
        })
        .unwrap();
    let expected: Vec<_> = bsl_rt::BUILTIN_METHOD_NAMES
        .iter()
        .map(|(name, _)| *name)
        .collect();
    assert_eq!(names.split(',').collect::<Vec<_>>(), expected);
    let native_names = RAW
        .lines()
        .find_map(|line| line.strip_prefix("methods\t"))
        .unwrap();
    assert_eq!(native_names, names);
}

#[test]
fn native_matrix_preserves_receiver_errors_separately_from_missing_methods() {
    assert!(RAW.lines().any(|line| line == "canary\tok"));
    assert!(
        RAW.lines()
            .any(|line| line == "file.end\tnotification-core-methods-client-1")
    );
    assert!(!RAW.contains("setup_error"));
    let names: Vec<_> = RAW
        .lines()
        .find_map(|line| line.strip_prefix("methods\t"))
        .unwrap()
        .split(',')
        .collect();
    let mut rows = HashSet::new();
    let mut counts = [0; 3];
    for line in RAW.lines() {
        let Some((id, value)) = line.split_once('\t') else {
            continue;
        };
        let Some((_, mask)) = value.split_once('|') else {
            continue;
        };
        assert!(rows.insert(id), "повтор строки {id}");
        assert_eq!(mask.len(), names.len(), "{id}");
        for (name, bit) in names.iter().zip(mask.bytes()) {
            match bit {
                b'1' => counts[0] += 1,
                b'0' => counts[1] += 1,
                b'?' => {
                    assert!(["table.index", "uuid"].contains(&id));
                    let expected =
                        format!("{id}.error.{name}\tType mismatch (parameter number '2')");
                    assert!(RAW.lines().any(|line| line == expected), "{expected}");
                    counts[2] += 1;
                }
                _ => panic!("неизвестный результат {id}: {mask}"),
            }
        }
    }
    assert_eq!(rows.len(), 18);
    assert_eq!(counts, [181, 1083, 158]);
}
