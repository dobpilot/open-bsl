use bsl_sema::{
    ImportedFunction, ImportedModule, RExpr, RStmtKind, ResolveOptions, ResolvedLink,
    resolve_program_with,
};

fn registry() -> bsl_rt::RuntimeRegistry {
    let mut builder = bsl_rt::RuntimeBuilder::new();
    builder.register(bsl_rt::core_library());
    builder.build().unwrap()
}

fn module(alias: &str, module: u32, global: bool) -> ImportedModule {
    ImportedModule {
        alias: alias.into(),
        module,
        is_self: false,
        global,
        functions: vec![ImportedFunction {
            name: "Проверить".into(),
            chunk: 1,
            is_procedure: false,
            is_async: false,
            param_by_val: vec![],
            param_has_default: vec![],
        }],
        variables: vec![],
        receiver_methods: &[],
        component_access: None,
    }
}

#[test]
fn repeated_alias_lookups_preserve_first_match_and_unicode_folding() {
    let imports = [
        module("Служба", 1, false),
        module("СЛУЖБА", 2, false),
        module("i", 3, false),
    ];
    let syntax =
        bsl_syntax::parse("А = служба.Проверить(); Б = СЛУЖБА.Проверить(); В = ı.Проверить();")
            .unwrap();
    let resolved = resolve_program_with(
        &syntax.items,
        &ResolveOptions::new(&registry()).imports(&imports),
    )
    .unwrap();
    assert_eq!(
        resolved.links,
        [
            ResolvedLink::Function { module: 1, func: 1 },
            ResolvedLink::Function { module: 3, func: 1 },
        ],
    );
}

#[test]
fn cached_alias_does_not_override_a_later_local_assignment() {
    let imports = [module("Служба", 1, false)];
    let syntax =
        bsl_syntax::parse("А = Служба.Проверить(); Служба = 7; Б = Служба.Проверить();").unwrap();
    let resolved = resolve_program_with(
        &syntax.items,
        &ResolveOptions::new(&registry()).imports(&imports),
    )
    .unwrap();
    assert!(matches!(
        resolved.top_level.body[2].kind,
        RStmtKind::AssignLocal {
            value: RExpr::CallMethod { .. },
            ..
        }
    ));
    assert_eq!(
        resolved.links,
        [ResolvedLink::Function { module: 1, func: 1 }]
    );
}

#[test]
fn cached_alias_is_still_shadowed_by_a_parameter_of_another_function() {
    let imports = [module("Служба", 1, false)];
    let syntax = bsl_syntax::parse(
        "Функция Первая() Возврат Служба.Проверить(); КонецФункции
         Функция Вторая(Служба) Возврат Служба.Проверить(); КонецФункции",
    )
    .unwrap();
    let resolved = resolve_program_with(
        &syntax.items,
        &ResolveOptions::new(&registry()).imports(&imports),
    )
    .unwrap();
    assert!(matches!(
        resolved.functions[1].body[0].kind,
        RStmtKind::Return(Some(RExpr::CallMethod { .. }))
    ));
}

#[test]
fn repeated_global_calls_keep_the_first_import_and_own_function_priority() {
    let imports = [
        module("ПерваяСлужба", 1, true),
        module("ВтораяСлужба", 2, true),
    ];
    let syntax = bsl_syntax::parse("А = Проверить(); Б = ПРОВЕРИТЬ();").unwrap();
    let registry = registry();
    let resolved = resolve_program_with(
        &syntax.items,
        &ResolveOptions::new(&registry).imports(&imports),
    )
    .unwrap();
    assert_eq!(
        resolved.links,
        [ResolvedLink::Function { module: 1, func: 1 }]
    );
    let syntax =
        bsl_syntax::parse("Функция Проверить() Возврат 7; КонецФункции А = Проверить();").unwrap();
    assert!(
        resolve_program_with(
            &syntax.items,
            &ResolveOptions::new(&registry).imports(&imports)
        )
        .unwrap()
        .links
        .is_empty()
    );
}
