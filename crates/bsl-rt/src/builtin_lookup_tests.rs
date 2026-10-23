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

#[test]
fn declared_only_builtins_raise_the_uniform_error() {
    let mut env = HostEnv::process();
    let mut seen = Vec::new();
    for &(_, function) in BUILTIN_FN_NAMES {
        if !function.is_declared_only() || seen.contains(&function) {
            continue;
        }
        seen.push(function);
        let args = vec![BslValue::Undefined; function.arity_range().0];
        let result = match function.host_effect() {
            Some(HostEffect::Env) => call_builtin_env_with_args(function, &args, &mut env),
            _ => call_builtin_fn(function, &args),
        };
        assert_eq!(
            format!("{result:?}"),
            format!(
                "{:?}",
                Err::<BslValue, _>(crate::declared_only_error(function.name()))
            ),
            "{}",
            function.name()
        );
    }
    assert!(!seen.is_empty());
    assert!(!BuiltinFn::StrLen.is_declared_only());
}
