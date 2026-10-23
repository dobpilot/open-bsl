use super::*;

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn workers_overlap_with_a_bound_of_two_and_keep_result_order() {
    let active = std::sync::atomic::AtomicUsize::new(0);
    let peak = std::sync::atomic::AtomicUsize::new(0);
    let barrier = std::sync::Barrier::new(2);
    let results = ordered_map(&[0; 32], 2, |index, _| {
        let count = active.fetch_add(1, Ordering::SeqCst) + 1;
        peak.fetch_max(count, Ordering::SeqCst);
        barrier.wait();
        active.fetch_sub(1, Ordering::SeqCst);
        barrier.wait();
        index
    });
    assert_eq!(results, (0..32).collect::<Vec<_>>());
    assert_eq!(peak.load(Ordering::SeqCst), 2);
}

fn module(name: &str, visibility: ModuleVisibility, source: &str) -> ModuleRecipe {
    ModuleRecipe {
        name: name.into(),
        visibility,
        source: source.into(),
        imports: vec![],
    }
}

fn compile(
    modules: Vec<ModuleRecipe>,
    roots: &[&str],
    workers: usize,
    deferred: bool,
) -> Result<bsl_bytecode::ConfigurationProgram, Error> {
    let engine = Engine::builder().build().unwrap();
    super::super::compile_catalog(
        &ModuleGraphRecipe {
            modules,
            eager_init: false,
        },
        &roots
            .iter()
            .map(|name| name.to_string())
            .collect::<Vec<_>>(),
        &[],
        &[],
        &[],
        &[],
        &engine.inner.registry,
        &bsl_syntax::PreprocSymbols::new(),
        deferred,
        bsl_compiler::BuildOptions::default(),
        workers,
    )
    .map(|(catalog, _, _)| catalog)
}

fn printed(catalog: bsl_bytecode::ConfigurationProgram) -> String {
    bsl_bytecode::write_image(
        &bsl_bytecode::BytecodeImage::Configuration {
            catalog,
            entry: None,
        },
        None,
    )
    .unwrap()
}

#[test]
fn worker_counts_preserve_images_reachability_and_large_literals() {
    let mut root = module(
        "Корень",
        ModuleVisibility::Explicit,
        "Функция Запуск() Экспорт Возврат Цель.Ответ(); КонецФункции",
    );
    root.imports.push(("Цель".into(), "Зависимость".into()));
    let modules = vec![
        module(
            "НеВыбран",
            ModuleVisibility::Private,
            "Функция Ошибка() Возврат НетТакого; КонецФункции",
        ),
        root,
        module(
            "Зависимость",
            ModuleVisibility::Private,
            "Функция Ответ() Экспорт Возврат 12345678901234567890123456789012345678901234567890.123456789; КонецФункции",
        ),
    ];
    let sequential = compile(modules.clone(), &["Корень"], 1, false).unwrap();
    let parallel = compile(modules, &["Корень"], 2, false).unwrap();
    assert_eq!(
        parallel
            .modules
            .iter()
            .map(|m| m.name.as_str())
            .collect::<Vec<_>>(),
        ["Корень", "Зависимость"]
    );
    assert_eq!(printed(sequential), printed(parallel));
}

#[test]
fn workers_keep_recipe_parse_errors_and_discovery_semantic_errors_in_order() {
    let long = "Проба = 1;\n".repeat(5000);
    for syntax_error in [false, true] {
        let first = if syntax_error {
            format!("{long}Проба = ;")
        } else {
            format!("Функция Ответ() {long} Возврат НетПервого; КонецФункции")
        };
        let second = if syntax_error {
            "Проба = ;"
        } else {
            "Функция Ответ() Возврат НетВторого; КонецФункции"
        };
        let modules = vec![
            module("Первый", ModuleVisibility::Named, &first),
            module("Второй", ModuleVisibility::Named, second),
        ];
        let sequential = compile(modules.clone(), &["Первый", "Второй"], 1, false).unwrap_err();
        let parallel = compile(modules, &["Первый", "Второй"], 2, false).unwrap_err();
        assert_eq!(sequential.to_string(), parallel.to_string());
        assert!(matches!(parallel, Error::InModule { ref module, .. } if module == "Первый"));
    }
}

#[test]
fn workers_keep_explicit_imports_and_deferred_cycle_error_precedence() {
    let mut root = module(
        "Корень",
        ModuleVisibility::Named,
        "Функция Ответ() Экспорт Возврат Лишняя(1); КонецФункции",
    );
    let mut peer = module("Сосед", ModuleVisibility::Named, "");
    root.imports.push(("Сосед".into(), "Сосед".into()));
    peer.imports.push(("Корень".into(), "Корень".into()));
    let extra = module(
        "Лишний",
        ModuleVisibility::Global,
        "Функция Лишняя() Экспорт Возврат Истина; КонецФункции",
    );
    for deferred in [false, true] {
        let modules = vec![extra.clone(), root.clone(), peer.clone()];
        let first = compile(modules.clone(), &["Корень"], 1, deferred).unwrap_err();
        let second = compile(modules, &["Корень"], 2, deferred).unwrap_err();
        assert_eq!(first.to_string(), second.to_string());
        assert!(matches!(second, Error::Configuration(ref message) if message.contains("цикл")));
    }
    root.imports = vec![("Нет".into(), "Нет".into())];
    for workers in [1, 2] {
        let error = compile(vec![root.clone()], &["Корень"], workers, false).unwrap_err();
        assert!(
            matches!(error, Error::Configuration(ref message) if message.contains("импортирует неизвестный"))
        );
    }
}

#[test]
fn semantic_transfer_preserves_resolvable_conformance_sources() {
    let engine = Engine::builder().build().unwrap();
    let fixtures =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/conformance/fixtures");
    let mut checked = 0;
    let mut pending = vec![fixtures];
    while let Some(path) = pending.pop() {
        if path.is_dir() {
            pending.extend(
                std::fs::read_dir(path)
                    .unwrap()
                    .map(|entry| entry.unwrap().path()),
            );
            continue;
        }
        if path.extension().is_none_or(|extension| extension != "bsl") {
            continue;
        }
        let source = std::fs::read_to_string(&path).unwrap();
        let Ok(syntax) = bsl_syntax::parse(&source) else {
            continue;
        };
        let Ok(resolved) =
            bsl_sema::resolve_program_with_registry(&syntax.items, &engine.inner.registry)
        else {
            continue;
        };
        let expected = resolved.clone();
        assert_eq!(
            resolved.into_sendable().into_resolved(),
            expected,
            "{}",
            path.display()
        );
        checked += 1;
    }
    assert!(checked > 50, "проверено только {checked} программ");
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn worker_stack_preserves_the_parser_deep_nesting_diagnostic() {
    std::thread::Builder::new()
        .stack_size(8 * 1024 * 1024)
        .spawn(|| {
            let source = format!("А = {}1{};", "(".repeat(1000), ")".repeat(1000));
            let barrier = std::sync::Barrier::new(2);
            let errors = ordered_map(&[(), ()], 2, |_, _| {
                barrier.wait();
                bsl_syntax::parse(&source).unwrap_err()
            });
            for error in errors {
                assert!(matches!(error, bsl_syntax::Diagnostic::Parse(error)
                if matches!(error.kind, bsl_syntax::ParseErrorKind::NestingTooDeep { .. })));
            }
        })
        .unwrap()
        .join()
        .unwrap();
}
