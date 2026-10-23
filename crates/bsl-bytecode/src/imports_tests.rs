use super::*;
use crate::ModuleId;

fn entry(index: u32) -> ModuleImport {
    ModuleImport {
        alias: format!("Модуль{index}"),
        module: ModuleId::new(index),
        global: index == 2,
    }
}

#[test]
fn omitted_self_and_owned_aliases_keep_order_and_share_backing() {
    let common: ModuleImports = (0..8).map(entry).collect();
    let mut entries: Vec<_> = common.iter().cloned().collect();
    entries.remove(3);
    let alias = ModuleImport {
        alias: "Псевдоним".into(),
        ..entry(3)
    };
    entries.insert(0, alias);
    let actual = ModuleImports::sharing(entries.clone(), &common);
    assert_eq!(actual.iter().cloned().collect::<Vec<_>>(), entries);
    assert_eq!(
        actual.iter().rev().cloned().collect::<Vec<_>>(),
        entries.into_iter().rev().collect::<Vec<_>>()
    );
    assert_eq!(actual.parts.len(), 3);
    for part in &actual.parts[1..] {
        assert!(Arc::ptr_eq(&part.entries, &common.parts[0].entries));
    }
    let cloned = actual.clone();
    for (left, right) in cloned.parts.iter().zip(&actual.parts) {
        assert!(Arc::ptr_eq(&left.entries, &right.entries));
    }
    assert_eq!(cloned, actual);
}

#[test]
fn shared_ranges_cross_parts_and_do_not_mutate_the_source() {
    let common: ModuleImports = (0..8).map(entry).collect();
    let mut source = ModuleImports::default();
    source.extend_shared(&common, 0..2);
    source.extend_shared(&common, 4..8);
    let mut actual = ModuleImports::default();
    actual.extend_shared(&source, 1..5);
    assert_eq!(
        actual.iter().map(|e| e.module.index()).collect::<Vec<_>>(),
        [1, 4, 5, 6]
    );
    actual.push(entry(10));
    assert_eq!(source.len(), 6);
    assert_eq!(actual[4], entry(10));
    assert!(ModuleImports::default().is_empty());
}

#[test]
fn equal_entries_with_different_global_flags_remain_distinct() {
    let common: ModuleImports = vec![entry(2)].into();
    let mut different = entry(2);
    different.global = false;
    let actual = ModuleImports::sharing([different.clone(), entry(2), entry(2)], &common);
    assert_eq!(actual[0], different);
    assert_eq!(actual.len(), 3);
    assert_eq!(actual.parts.len(), 3);
    assert!(!Arc::ptr_eq(
        &actual.parts[0].entries,
        &common.parts[0].entries
    ));
    assert!(Arc::ptr_eq(
        &actual.parts[1].entries,
        &common.parts[0].entries
    ));
}
