//! Поиск функций сохраняет первый вариант в общей таблице написаний.

use super::*;

fn linear_lookup(name: &str) -> Option<BuiltinFn> {
    let upper = name.to_uppercase();
    BUILTIN_FN_NAMES
        .iter()
        .find(|(name, _)| name.to_uppercase() == upper)
        .map(|(_, function)| *function)
}

#[test]
fn function_lookup_matches_every_original_alias_and_case_variant() {
    for (name, _) in BUILTIN_FN_NAMES {
        for spelling in [name.to_string(), name.to_lowercase(), name.to_uppercase()] {
            assert_eq!(BuiltinFn::lookup(&spelling), linear_lookup(&spelling));
        }
    }
    for name in [" Строка", "Строка ", "", "НетТакойФункции"] {
        assert_eq!(BuiltinFn::lookup(name), None);
    }
}
