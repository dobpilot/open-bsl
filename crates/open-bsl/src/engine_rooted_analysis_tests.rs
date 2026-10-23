use super::*;

fn component(
    _: &mut bsl_rt::CallContext<'_>,
    _: &[bsl_rt::BslValue],
) -> bsl_rt::RtResult<bsl_rt::BslValue> {
    Ok(bsl_rt::BslValue::Undefined)
}

const FUNCTIONS: &[bsl_rt::FunctionDescriptor] = &[bsl_rt::FunctionDescriptor {
    code: bsl_rt::FunctionCode::new(1),
    names: &["ПолучитьФабрикуПробы"],
    arity: bsl_rt::Arity::exact(0),
    kind: bsl_rt::FunctionKind::Function,
    call: Some(component),
}];
const GLOBALS: &[bsl_rt::GlobalDescriptor] = &[bsl_rt::GlobalDescriptor {
    names: &["ФабрикаПробы"],
    function: bsl_rt::FunctionCode::new(1),
}];

fn recipe(name: &str, visibility: ModuleVisibility, source: &str) -> ModuleRecipe {
    ModuleRecipe {
        name: name.into(),
        visibility,
        source: source.into(),
        imports: vec![],
    }
}

fn build(modules: Vec<ModuleRecipe>, rooted: bool, optimize: bool, debug: bool) -> Engine {
    let mut builder = Engine::builder()
        .register_library(
            LibraryDescriptor::new("rooted-reuse-test", "1")
                .with_functions(FUNCTIONS)
                .with_globals(GLOBALS),
        )
        .configuration(ModuleGraphRecipe {
            modules,
            eager_init: false,
        })
        .configuration_component_module("ФабрикаПробы", "Менеджер", "ЧастныйМенеджер")
        .debug_info(debug)
        .optimizations(if optimize {
            bsl_compiler::Optimizations::all()
        } else {
            bsl_compiler::Optimizations::default()
        });
    if rooted {
        builder = builder.configuration_roots(["Корень"]);
    }
    builder.build().unwrap()
}

fn image(engine: &Engine) -> String {
    bsl_bytecode::write_image(
        &bsl_bytecode::BytecodeImage::Configuration {
            catalog: engine.catalog().unwrap().clone(),
            entry: None,
        },
        None,
    )
    .unwrap()
}

#[test]
fn reused_rooted_bodies_match_pruned_compilation_with_compact_module_ids() {
    let mut root = recipe(
        "Корень",
        ModuleVisibility::Explicit,
        r#"
Функция Запуск() Экспорт
    СсылкаМодуля = Данные;
    Данные.Общая = СсылкаМодуля.Прочитать() + Добавка();
    Если Ложь Тогда Получатель = ФабрикаПробы.Менеджер; КонецЕсли;
    Попытка
        Лишнее = Вычислить("Недостижимый.Ответ()");
    Исключение
        Возврат Вычислить("Псевдоним.Прочитать()");
    КонецПопытки;
    ВызватьИсключение "лишний модуль стал доступен";
КонецФункции
"#,
    );
    root.imports.push(("Псевдоним".into(), "Данные".into()));
    let data = recipe(
        "Данные",
        ModuleVisibility::Named,
        "Перем Общая Экспорт; Функция Прочитать() Экспорт Возврат Общая; КонецФункции Общая = 40;",
    );
    let global = recipe(
        "Глобальный",
        ModuleVisibility::Global,
        "Функция Добавка() Экспорт Возврат 2; КонецФункции",
    );
    let manager = recipe(
        "ЧастныйМенеджер",
        ModuleVisibility::Private,
        "Функция Ответ() Экспорт Возврат 99; КонецФункции",
    );
    let pruned = vec![data.clone(), global.clone(), manager.clone(), root.clone()];
    let full = vec![
        recipe(
            "ПервыйЛишний",
            ModuleVisibility::Private,
            "Функция Ошибка() Экспорт Возврат НетТакого; КонецФункции",
        ),
        data,
        recipe(
            "СреднийЛишний",
            ModuleVisibility::Private,
            "Функция Ошибка() Экспорт Возврат НетТакого; КонецФункции",
        ),
        global,
        manager,
        root,
        recipe(
            "Недостижимый",
            ModuleVisibility::Named,
            "Функция Ответ() Экспорт Возврат НетТакого; КонецФункции",
        ),
    ];
    for optimize in [false, true] {
        for debug in [false, true] {
            // Проходы, переставляющие инструкции, несовместимы с отладочной сборкой.
            if optimize && debug {
                continue;
            }
            let reused = build(full.clone(), true, optimize, debug);
            let ordinary = build(pruned.clone(), false, optimize, debug);
            assert_eq!(image(&reused), image(&ordinary));
            let catalog = reused.catalog().unwrap();
            assert_eq!(
                catalog
                    .modules
                    .iter()
                    .map(|m| m.name.as_str())
                    .collect::<Vec<_>>(),
                ["Данные", "Глобальный", "ЧастныйМенеджер", "Корень"]
            );
            assert!(
                catalog.modules.iter().all(|m| m
                    .program
                    .imports
                    .iter()
                    .all(|i| !i.alias.is_empty()))
            );
            for engine in [reused, ordinary] {
                let entry = engine.compile_entry("Возврат Корень.Запуск();").unwrap();
                assert_eq!(
                    engine.new_state().run(&entry).unwrap(),
                    crate::Value::number_from_i64(42)
                );
            }
        }
    }
}

#[test]
fn unselected_syntax_errors_still_fail_before_unknown_root_diagnostics() {
    let error = Engine::builder()
        .configuration(ModuleGraphRecipe {
            modules: vec![recipe("Сломанный", ModuleVisibility::Private, "х = ;")],
            eager_init: false,
        })
        .configuration_roots(["НетКорня"])
        .build()
        .err()
        .unwrap();
    assert!(matches!(error, Error::InModule { source, .. } if matches!(*source, Error::Parse(_))));
}

#[test]
fn deferred_arity_errors_keep_the_diagnostic_of_the_pruned_environment() {
    let root = recipe(
        "Корень",
        ModuleVisibility::Named,
        "Функция Запуск() Экспорт Возврат ЛишняяФункция(1); КонецФункции",
    );
    let extra = recipe(
        "Лишний",
        ModuleVisibility::Global,
        "Функция ЛишняяФункция() Экспорт Возврат Истина; КонецФункции",
    );
    let make = |rooted| {
        let mut builder = Engine::builder()
            .defer_common_module_name_errors(true)
            .configuration(ModuleGraphRecipe {
                modules: if rooted {
                    vec![extra.clone(), root.clone()]
                } else {
                    vec![root.clone()]
                },
                eager_init: false,
            });
        if rooted {
            builder = builder.configuration_roots(["Корень"]);
        }
        builder.build().unwrap()
    };
    assert_eq!(image(&make(true)), image(&make(false)));
}

#[test]
fn import_cycles_still_precede_errors_from_rechecking_a_pruned_environment() {
    let mut root = recipe(
        "Корень",
        ModuleVisibility::Named,
        "Функция Запуск() Экспорт Возврат ЛишняяФункция(1); КонецФункции",
    );
    root.imports.push(("Сосед".into(), "Сосед".into()));
    let mut peer = recipe("Сосед", ModuleVisibility::Named, "");
    peer.imports.push(("Корень".into(), "Корень".into()));
    let extra = recipe(
        "Лишний",
        ModuleVisibility::Global,
        "Функция ЛишняяФункция() Экспорт Возврат Истина; КонецФункции",
    );
    let error = Engine::builder()
        .configuration(ModuleGraphRecipe {
            modules: vec![extra, root, peer],
            eager_init: false,
        })
        .configuration_roots(["Корень"])
        .build()
        .err()
        .unwrap();
    assert!(
        matches!(error, Error::Configuration(ref message) if message.contains("содержит цикл")),
        "{error}"
    );
}
