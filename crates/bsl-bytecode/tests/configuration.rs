//! Конфигурационный образ: round-trip текстового формата и периметр
//! `verify_configuration`. Программы собраны руками — см. `support`.

mod support;

use bsl_bytecode::image::{self, verify, verify_configuration};
use bsl_bytecode::{
    ArgMode, BytecodeImage, ConfigurationProgram, EntryId, EntryProgram, Instr, LinkEntry,
    ModuleId, ModuleProgram, Program, parse_image, parse_program, write_image,
};

/// Модуль «Служебный»: экспортная функция `Удвоить`, экспортная переменная
/// `Счётчик` и неэкспортная `Скрытая`.
fn service_module() -> Program {
    let top = support::chunk(vec![Instr::Return { src: None }]);
    let mut callee = support::chunk(vec![Instr::Return { src: Some(0) }]);
    callee.n_params = 1;
    callee.n_locals = 1;
    callee.param_by_val = vec![false];
    callee.param_has_default = vec![false];
    let mut p = support::program(vec![top, callee]);
    p.function_names = vec!["Удвоить".to_string()];
    p.exported_functions = vec![true];
    p.module_vars = vec!["Счётчик".to_string(), "Скрытая".to_string()];
    p.exported_module_vars = vec![true, false];
    recompute_bundles(&mut p);
    p
}

/// Модуль «Клиент»: читает, вызывает и пишет символы «Служебного» через
/// таблицу связей.
fn client_module() -> Program {
    let mut top = support::chunk(vec![
        Instr::GetImportedVar {
            dst: 0,
            link_slot: 1,
        },
        Instr::CallImported {
            link_slot: 0,
            base: 0,
            arg_modes: 0,
            ret: 0,
        },
        Instr::SetImportedVar {
            link_slot: 1,
            src: 0,
        },
        Instr::Return { src: None },
    ]);
    top.call_arg_modes = vec![vec![ArgMode::Value]];
    let mut p = support::program(vec![top]);
    p.links = vec![
        LinkEntry::Function {
            module: ModuleId::new(0),
            func: 1,
        },
        LinkEntry::Variable {
            module: ModuleId::new(0),
            slot: 0,
        },
    ];
    recompute_bundles(&mut p);
    p
}

fn entry_program() -> EntryProgram {
    let mut top = support::chunk(vec![
        Instr::CallImported {
            link_slot: 0,
            base: 0,
            arg_modes: 0,
            ret: 0,
        },
        Instr::Return { src: None },
    ]);
    top.call_arg_modes = vec![vec![ArgMode::Value]];
    let mut p = support::program(vec![top]);
    p.links = vec![LinkEntry::Function {
        module: ModuleId::new(0),
        func: 1,
    }];
    recompute_bundles(&mut p);
    EntryProgram {
        id: EntryId::new(7),
        program: p,
    }
}

fn recompute_bundles(p: &mut Program) {
    image::finalize(p);
}

fn catalog() -> ConfigurationProgram {
    ConfigurationProgram {
        modules: vec![
            ModuleProgram {
                name: "Служебный".to_string(),
                program: service_module(),
            },
            ModuleProgram {
                name: "Клиент".to_string(),
                program: client_module(),
            },
        ],
    }
}

#[test]
fn a_configuration_image_round_trips_byte_identical() {
    let image = BytecodeImage::Configuration {
        catalog: catalog(),
        entry: Some(entry_program()),
    };
    let first = write_image(&image, Some("тест.bsl")).unwrap();
    let reparsed = parse_image(&first).unwrap_or_else(|e| panic!("{first}\nошибка: {e}"));
    let second = write_image(&reparsed, Some("тест.bsl")).unwrap();
    assert_eq!(first, second);
    let BytecodeImage::Configuration { catalog, entry } = reparsed else {
        panic!("разобрался не в конфигурацию");
    };
    assert_eq!(catalog.modules.len(), 2);
    assert_eq!(catalog.modules[0].name, "Служебный");
    assert_eq!(entry.as_ref().map(|e| e.id), Some(EntryId::new(7)));
}

#[test]
fn a_configuration_image_without_entry_round_trips() {
    let image = BytecodeImage::Configuration {
        catalog: catalog(),
        entry: None,
    };
    let first = write_image(&image, None).unwrap();
    let reparsed = parse_image(&first).unwrap();
    let second = write_image(&reparsed, None).unwrap();
    assert_eq!(first, second);
    let BytecodeImage::Configuration { entry, .. } = reparsed else {
        panic!("разобрался не в конфигурацию");
    };
    assert!(entry.is_none());
}

#[test]
fn a_single_program_image_round_trips_through_parse_image() {
    let image = BytecodeImage::Program(support::every_section());
    let first = write_image(&image, None).unwrap();
    let BytecodeImage::Program(program) = parse_image(&first).unwrap() else {
        panic!("одиночная программа разобралась в конфигурацию");
    };
    let second = write_image(&BytecodeImage::Program(program), None).unwrap();
    assert_eq!(first, second);
}

#[test]
fn parse_program_rejects_a_configuration_image() {
    let image = BytecodeImage::Configuration {
        catalog: catalog(),
        entry: None,
    };
    let text = write_image(&image, None).unwrap();
    let error = parse_program(&text).expect_err("конфигурация не одиночная программа");
    assert!(
        error.to_string().contains("parse_image"),
        "не та ошибка: {error}"
    );
}

#[test]
fn the_valid_catalog_passes_verification() {
    let entry = entry_program();
    verify_configuration(&catalog(), Some(&entry)).unwrap();
}

#[test]
fn every_module_passes_the_single_program_perimeter() {
    for module in catalog().modules {
        verify(&module.program).unwrap();
    }
}

#[test]
fn a_link_to_a_non_exported_function_is_rejected() {
    let mut catalog = catalog();
    catalog.modules[0].program.exported_functions[0] = false;
    let error = verify_configuration(&catalog, None).expect_err("цель неэкспортна");
    assert!(error.to_string().contains("неэкспортный"), "{error}");
}

#[test]
fn a_link_to_a_non_exported_variable_is_rejected() {
    let mut catalog = catalog();
    catalog.modules[1].program.links[1] = LinkEntry::Variable {
        module: ModuleId::new(0),
        slot: 1,
    };
    let error = verify_configuration(&catalog, None).expect_err("переменная неэкспортна");
    assert!(error.to_string().contains("неэкспортную"), "{error}");
}

#[test]
fn a_link_past_the_catalog_is_rejected() {
    let mut catalog = catalog();
    catalog.modules[1].program.links[0] = LinkEntry::Function {
        module: ModuleId::new(9),
        func: 1,
    };
    let error = verify_configuration(&catalog, None).expect_err("модуль вне каталога");
    assert!(error.to_string().contains("мимо каталога"), "{error}");
}

#[test]
fn a_self_link_is_rejected() {
    let mut catalog = catalog();
    catalog.modules[1].program.links[0] = LinkEntry::Function {
        module: ModuleId::new(1),
        func: 1,
    };
    // Собственный чанк 1 у «Клиента» отсутствует, но проверка на
    // собственный модуль стоит раньше и срабатывает первой.
    let error = verify_configuration(&catalog, None).expect_err("связь на себя");
    assert!(error.to_string().contains("собственный модуль"), "{error}");
}

#[test]
fn an_import_cycle_is_rejected() {
    let mut catalog = catalog();
    // «Служебный» получает связь на функцию «Клиента» — но у «Клиента» нет
    // экспортных функций, поэтому цикл строится через переменную.
    catalog.modules[1].program.module_vars = vec!["Обратная".to_string()];
    catalog.modules[1].program.exported_module_vars = vec![true];
    catalog.modules[0].program.links = vec![LinkEntry::Variable {
        module: ModuleId::new(1),
        slot: 0,
    }];
    let error = verify_configuration(&catalog, None).expect_err("цикл импортов");
    assert!(error.to_string().contains("цикл"), "{error}");
}

#[test]
fn an_arity_mismatch_across_modules_is_rejected() {
    let mut catalog = catalog();
    // Ноль аргументов против одного параметра `Удвоить`: локальная
    // геометрия чанка верна, расхождение видно только через каталог.
    catalog.modules[1].program.chunks[0].call_arg_modes = vec![Vec::new()];
    let error = verify_configuration(&catalog, None).expect_err("арность разошлась");
    assert!(error.to_string().contains("параметров"), "{error}");
}

#[test]
fn a_by_ref_import_against_znach_is_rejected() {
    let mut catalog = catalog();
    catalog.modules[0].program.chunks[1].param_by_val = vec![true];
    catalog.modules[1].program.chunks[0].n_locals = 1;
    catalog.modules[1].program.chunks[0].call_arg_modes = vec![vec![ArgMode::ByRefLocal(0)]];
    let error = verify_configuration(&catalog, None).expect_err("Знач по ссылке");
    assert!(error.to_string().contains("Знач"), "{error}");
}

#[test]
fn duplicate_module_names_differing_only_in_case_are_rejected() {
    let mut catalog = catalog();
    catalog.modules[1].name = "СЛУЖЕБНЫЙ".to_string();
    catalog.modules[1].program.links.clear();
    catalog.modules[1].program.chunks[0] = {
        // Один чанк без модульных переменных: разметку ему ставит та же
        // единая точка, что и всем остальным.
        let mut one = Program {
            chunks: vec![support::chunk(vec![Instr::Return { src: None }])],
            ..support::program(Vec::new())
        };
        image::finalize(&mut one);
        one.chunks.remove(0)
    };
    let error = verify_configuration(&catalog, None).expect_err("дубль имени");
    assert!(error.to_string().contains("регистра"), "{error}");
}
