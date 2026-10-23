mod support;

use bsl_bytecode::{
    BytecodeImage, ConfigurationProgram, Instr, ModuleProgram, image, parse_image, write_image,
    write_program,
};
use bsl_rt::NameId;
use std::{rc::Rc, sync::Arc};

fn catalog(count: usize) -> BytecodeImage {
    let mut program = support::program(vec![support::chunk(vec![Instr::Return { src: None }])]);
    program.names = (0..4096)
        .map(|i| format!("ИмяОбщегоКаталога{i}"))
        .collect::<Vec<_>>()
        .into();
    program.shapes = support::shapes(&[NameId::from_index(4095)]);
    image::finalize(&mut program);
    BytecodeImage::Configuration {
        catalog: ConfigurationProgram {
            modules: (0..count)
                .map(|i| ModuleProgram {
                    name: format!("Модуль{i}"),
                    entry_visible: true,
                    global: false,
                    program: program.clone(),
                })
                .collect(),
        },
        entry: None,
    }
}

#[test]
fn repeated_tables_are_written_once_and_parse_as_shared_immutable_tables() {
    let one = write_image(&catalog(1), None).unwrap();
    let text = write_image(&catalog(100), None).unwrap();
    assert_eq!(text.matches(".names 4096").count(), 1);
    assert_eq!(text.matches(".tables shared").count(), 99);
    assert!(text.len() < one.len() + 100 * 1024);
    let decoded = parse_image(&text).unwrap();
    let BytecodeImage::Configuration { catalog, .. } = &decoded else {
        panic!("configuration expected")
    };
    for module in &catalog.modules {
        image::verify(&module.program).unwrap();
        assert!(Arc::ptr_eq(
            &catalog.modules[0].program.names,
            &module.program.names
        ));
        assert!(Rc::ptr_eq(
            &catalog.modules[0].program.shapes[0],
            &module.program.shapes[0]
        ));
    }
    assert_eq!(write_image(&decoded, None).unwrap(), text);
}

#[test]
fn a_table_reference_requires_the_first_configuration_module() {
    let BytecodeImage::Configuration { catalog, .. } = catalog(1) else {
        unreachable!()
    };
    let text = write_program(&catalog.modules[0].program, None).unwrap();
    let begin = text.find(".names ").unwrap();
    let end = text.find(".top-locals ").unwrap();
    let referenced = format!("{}.tables shared\n{}", &text[..begin], &text[end..]);
    assert!(parse_image(&referenced).is_err());
    let text = write_image(
        &BytecodeImage::Configuration {
            catalog,
            entry: None,
        },
        None,
    )
    .unwrap();
    let begin = text.find(".names ").unwrap();
    let end = text.find(".top-locals ").unwrap();
    let referenced = format!("{}.tables shared\n{}", &text[..begin], &text[end..]);
    assert!(parse_image(&referenced).is_err());
}

#[test]
fn independent_tables_do_not_change_the_base_of_later_references() {
    let mut image = catalog(3);
    let BytecodeImage::Configuration { catalog, .. } = &mut image else {
        unreachable!()
    };
    let mut names = catalog.modules[1].program.names.to_vec();
    names[0] = "ДругоеИмя".into();
    catalog.modules[1].program.names = names.into();
    let text = write_image(&image, None).unwrap();
    assert_eq!(text.matches(".names 4096").count(), 2);
    assert_eq!(text.matches(".tables shared").count(), 1);
    let decoded = parse_image(&text).unwrap();
    let BytecodeImage::Configuration { catalog, .. } = &decoded else {
        unreachable!()
    };
    assert_eq!(catalog.modules[1].program.names[0], "ДругоеИмя");
    assert!(Arc::ptr_eq(
        &catalog.modules[0].program.names,
        &catalog.modules[2].program.names
    ));
    assert!(!Arc::ptr_eq(
        &catalog.modules[0].program.names,
        &catalog.modules[1].program.names
    ));
    assert_eq!(write_image(&decoded, None).unwrap(), text);
}

#[test]
fn a_different_shape_table_is_not_replaced_by_the_first_modules_table() {
    let mut image = catalog(2);
    let BytecodeImage::Configuration { catalog, .. } = &mut image else {
        unreachable!()
    };
    catalog.modules[1].program.shapes = support::shapes(&[NameId::from_index(1)]);
    let text = write_image(&image, None).unwrap();
    assert_eq!(text.matches(".names 4096").count(), 2);
    assert!(!text.contains(".tables shared"));
    let decoded = parse_image(&text).unwrap();
    let BytecodeImage::Configuration { catalog, .. } = &decoded else {
        unreachable!()
    };
    assert_eq!(
        catalog.modules[1].program.shapes[0].names,
        [NameId::from_index(1)]
    );
    assert_eq!(write_image(&decoded, None).unwrap(), text);
}

#[test]
fn shared_entry_tables_round_trip_and_unknown_references_are_rejected() {
    let mut image = catalog(2);
    let BytecodeImage::Configuration { catalog, entry } = &mut image else {
        unreachable!()
    };
    *entry = Some(bsl_bytecode::EntryProgram {
        id: bsl_bytecode::EntryId::new(7),
        program: catalog.modules[0].program.clone(),
    });
    let text = write_image(&image, None).unwrap();
    assert_eq!(text.matches(".tables shared").count(), 2);
    let decoded = parse_image(&text).unwrap();
    let BytecodeImage::Configuration {
        catalog,
        entry: Some(entry),
    } = &decoded
    else {
        unreachable!()
    };
    assert!(Arc::ptr_eq(
        &catalog.modules[0].program.names,
        &entry.program.names
    ));
    assert!(Rc::ptr_eq(
        &catalog.modules[0].program.shapes[0],
        &entry.program.shapes[0]
    ));
    assert_eq!(write_image(&decoded, None).unwrap(), text);
    for directive in [
        ".tables other",
        ".tables shared extra",
        ".tables",
        ".tables 0",
    ] {
        assert!(
            parse_image(&text.replacen(".tables shared", directive, 1)).is_err(),
            "{directive}"
        );
    }
}

#[test]
fn common_import_suffixes_are_not_repeated_after_different_local_prefixes() {
    let mut original = catalog(100);
    let BytecodeImage::Configuration { catalog, .. } = &mut original else {
        unreachable!()
    };
    for (index, module) in catalog.modules.iter_mut().enumerate() {
        module.program.imports = std::iter::once(bsl_bytecode::ModuleImport {
            alias: format!("Локальный{index}"),
            module: bsl_bytecode::ModuleId::new(0),
            global: false,
        })
        .chain((0..4096).map(|i| bsl_bytecode::ModuleImport {
            alias: format!("ОбщийМодуль{i}"),
            module: bsl_bytecode::ModuleId::new(0),
            global: i % 2 == 0,
        }))
        .collect();
    }
    let text = write_image(&original, None).unwrap();
    let BytecodeImage::Configuration { catalog, .. } = &original else {
        unreachable!()
    };
    assert_eq!(text.matches(".imports 4097").count(), 1);
    assert_eq!(text.matches(".imports 1 shared-tail=4096").count(), 99);
    let decoded = parse_image(&text).unwrap();
    let BytecodeImage::Configuration {
        catalog: actual, ..
    } = &decoded
    else {
        unreachable!()
    };
    for (expected, actual) in catalog.modules.iter().zip(&actual.modules) {
        assert_eq!(actual.program.imports, expected.program.imports);
        image::verify(&actual.program).unwrap();
    }
    assert_eq!(write_image(&decoded, None).unwrap(), text);
}

#[test]
fn invalid_import_suffixes_and_explicit_counts_are_rejected() {
    let mut original = catalog(2);
    let BytecodeImage::Configuration { catalog, .. } = &mut original else {
        unreachable!()
    };
    for module in &mut catalog.modules {
        module.program.imports = vec![bsl_bytecode::ModuleImport {
            alias: "Общий".into(),
            module: bsl_bytecode::ModuleId::new(0),
            global: true,
        }]
        .into();
    }
    let standalone = write_program(&catalog.modules[0].program, None).unwrap();
    assert!(
        parse_image(&standalone.replacen(".imports 1", ".imports 0 shared-tail=1", 1)).is_err()
    );
    let text = write_image(&original, None).unwrap();
    assert!(parse_image(&text.replacen(".imports 1", ".imports 0 shared-tail=1", 1)).is_err());
    assert!(text.contains(".imports 0 shared-tail=1"));
    for invalid in [
        ".imports 0 shared-tail=0",
        ".imports 0 shared-tail=2",
        ".imports 0 shared-tail=18446744073709551615",
        ".imports 0 shared-tail=-1",
        ".imports 0 shared-tail=x",
        ".imports 0 shared-tail=1 extra",
        ".imports 18446744073709551615 shared-tail=1",
    ] {
        assert!(
            parse_image(&text.replacen(".imports 0 shared-tail=1", invalid, 1)).is_err(),
            "{invalid}"
        );
    }
}

#[test]
fn an_independent_import_set_does_not_rebase_the_next_shared_suffix() {
    let mut original = catalog(3);
    let BytecodeImage::Configuration { catalog, .. } = &mut original else {
        unreachable!()
    };
    for (index, module) in catalog.modules.iter_mut().enumerate() {
        module.program.imports = vec![bsl_bytecode::ModuleImport {
            alias: format!("Локальный{index}"),
            module: bsl_bytecode::ModuleId::new(0),
            global: false,
        }]
        .into();
        if index != 1 {
            module.program.imports.push(bsl_bytecode::ModuleImport {
                alias: "Общий".into(),
                module: bsl_bytecode::ModuleId::new(0),
                global: true,
            });
        }
    }
    let text = write_image(&original, None).unwrap();
    assert_eq!(text.matches(".imports 1 shared-tail=1").count(), 1);
    let decoded = parse_image(&text).unwrap();
    assert_eq!(write_image(&decoded, None).unwrap(), text);
    let BytecodeImage::Configuration { catalog, .. } = decoded else {
        unreachable!()
    };
    assert_eq!(catalog.modules[1].program.imports.len(), 1);
    assert_eq!(catalog.modules[2].program.imports[0].alias, "Локальный2");
    assert_eq!(catalog.modules[2].program.imports[1].alias, "Общий");
}
