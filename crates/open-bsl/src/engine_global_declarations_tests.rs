use super::*;

fn surface(source: &str) -> bsl_sema::ImportedModule {
    let syntax = bsl_syntax::parse(source).unwrap();
    imported_module_from_syntax("Модуль", 0, &syntax.items, true)
        .map_err(|error| error.in_module("Модуль"))
        .unwrap()
}

fn conflict(entries: &[(ModuleVisibility, &str)]) -> Result<(), Error> {
    let modules = entries
        .iter()
        .enumerate()
        .map(|(index, (visibility, _))| ModuleRecipe {
            name: format!("Модуль{index}"),
            source: String::new(),
            imports: Vec::new(),
            visibility: *visibility,
        })
        .collect::<Vec<_>>();
    let surfaces = entries
        .iter()
        .map(|(_, source)| surface(source))
        .collect::<Vec<_>>();
    reject_duplicate_global_exports(&modules, &surfaces)
}

#[test]
fn global_export_conflicts_preserve_unicode_folding_and_first_diagnostic() {
    for (first, second) in [("Тест", "тЕСТ"), ("I", "ı"), ("Straße", "STRASSE")] {
        let first = format!("Процедура {first}() Экспорт КонецПроцедуры");
        let second = format!("Процедура {second}() Экспорт КонецПроцедуры");
        let Error::Configuration(message) = conflict(&[
            (ModuleVisibility::Global, &first),
            (ModuleVisibility::Global, &second),
            (ModuleVisibility::Global, &first),
        ])
        .unwrap_err() else {
            panic!("ожидалась ошибка конфигурации");
        };
        let name = surface(&second).functions[0].name.clone();
        assert_eq!(
            message,
            format!("глобальный экспорт «{name}» объявлен несколькими общими модулями")
        );
    }
}

#[test]
fn global_export_conflicts_ignore_named_private_and_same_module_duplicates() {
    let exported = "Процедура Тест() Экспорт КонецПроцедуры";
    let repeated = format!("{exported}\n{exported}");
    conflict(&[
        (ModuleVisibility::Named, exported),
        (ModuleVisibility::Private, exported),
        (ModuleVisibility::Global, &repeated),
        (ModuleVisibility::Global, "Процедура Тест() КонецПроцедуры"),
    ])
    .unwrap();
}

#[test]
fn variable_surfaces_preserve_first_spelling_slots_and_later_export() {
    let source = "Перем Скрытая; Перем I; Перем Хвост Экспорт; Перем ı Экспорт;\n\
        Перем Straße; Перем STRASSE Экспорт; Перем хвост;";
    let syntax_surface = surface(source);
    let expected = vec![("I", 1), ("Хвост", 2), ("Straße", 3)];
    let variables = |surface: &bsl_sema::ImportedModule| {
        surface
            .variables
            .iter()
            .map(|variable| (variable.name.clone(), variable.slot))
            .collect::<Vec<_>>()
    };
    assert_eq!(
        variables(&syntax_surface),
        expected
            .into_iter()
            .map(|(name, slot)| (name.to_string(), slot))
            .collect::<Vec<_>>()
    );

    let server = bsl_config::ModuleExecution::new().with_server(true);
    let snapshot = bsl_config::ConfigurationCodeSnapshot::new(
        "surfaces",
        vec![
            bsl_config::ConfigurationModule::new(
                "root",
                "Корень",
                "",
                bsl_config::ModuleKind::Session,
                server,
            ),
            bsl_config::ConfigurationModule::new(
                "variables",
                "Переменные",
                source,
                bsl_config::ModuleKind::Common,
                server,
            ),
        ],
    )
    .unwrap();
    let engine = Engine::builder()
        .configuration_source(Arc::new(bsl_config::SnapshotSource::new(snapshot)))
        .configuration_roots(["Корень"])
        .build()
        .unwrap();
    let catalog = engine.indexed.as_ref().unwrap();
    let imported = bsl_sema::ImportedModuleProvider::module_surface(catalog.as_ref(), 1).unwrap();
    assert_eq!(variables(imported), variables(&syntax_surface));
}
