mod support;

use bsl_bytecode::{ConfigurationProgram, Instr, ModuleId, ModuleImport, ModuleProgram, image};

fn catalog() -> ConfigurationProgram {
    ConfigurationProgram {
        modules: ["Первый", "Второй"]
            .into_iter()
            .map(|name| ModuleProgram {
                name: name.into(),
                program: support::program(vec![support::chunk(vec![Instr::Return { src: None }])]),
            })
            .collect(),
    }
}

fn import(alias: &str, module: u32) -> ModuleImport {
    ModuleImport {
        alias: alias.into(),
        module: ModuleId::new(module),
    }
}

#[test]
fn declared_imports_round_trip_without_static_links() {
    let mut catalog = catalog();
    catalog.modules[0].program.imports = vec![import("Данные", 1), import("Alias", 1)];
    image::verify_configuration(&catalog, None).unwrap();
    let text = bsl_bytecode::write_image(
        &bsl_bytecode::BytecodeImage::Configuration {
            catalog: catalog.clone(),
            entry: None,
        },
        None,
    )
    .unwrap();
    assert!(text.starts_with("bslc 35\n"));
    assert!(text.contains(".imports 2"));
    let bsl_bytecode::BytecodeImage::Configuration {
        catalog: parsed, ..
    } = bsl_bytecode::parse_image(&text).unwrap()
    else {
        panic!("каталог");
    };
    assert_eq!(
        parsed.modules[0].program.imports,
        catalog.modules[0].program.imports
    );
    assert!(parsed.modules[0].program.links.is_empty());
    assert!(matches!(
        bsl_bytecode::parse_image(&text.replacen("bslc 35", "bslc 34", 1)),
        Err(bsl_bytecode::TextError::BadHeader(_))
    ));
    for (from, to) in [
        ("module=1", "module=-1"),
        ("module=1", "module=4294967296"),
        (".imports 2", ".imports 3"),
    ] {
        assert!(bsl_bytecode::parse_image(&text.replacen(from, to, 1)).is_err());
    }
}

#[test]
fn import_names_targets_and_unused_cycles_are_verified() {
    for imports in [
        vec![import("", 1)],
        vec![import("ДаНнЫе", 1), import("данные", 1)],
        vec![import("Данные", 2)],
        vec![import("Данные", 0)],
    ] {
        let mut catalog = catalog();
        catalog.modules[0].program.imports = imports;
        assert!(image::verify_configuration(&catalog, None).is_err());
    }
    let mut catalog = catalog();
    catalog.modules[0].program.imports = vec![import("Второй", 1)];
    catalog.modules[1].program.imports = vec![import("Первый", 0)];
    assert!(image::verify_configuration(&catalog, None).is_err());
}

#[test]
fn a_dynamic_program_is_checked_against_the_catalog_export_surface() {
    let mut catalog = catalog();
    let target = &mut catalog.modules[1].program;
    target
        .chunks
        .push(support::chunk(vec![Instr::Return { src: None }]));
    target.function_names.push("Метод".into());
    target.exported_functions.push(false);
    let mut fragment = support::program(vec![support::chunk(vec![Instr::Return { src: None }])]);
    fragment.imports = vec![import("Данные", 1)];
    fragment.links = vec![bsl_bytecode::LinkEntry::Function {
        module: ModuleId::new(1),
        func: 1,
    }];
    assert!(image::verify_linked_program(&fragment, &catalog, Some(ModuleId::new(0))).is_err());
    catalog.modules[1].program.exported_functions[0] = true;
    image::verify_linked_program(&fragment, &catalog, Some(ModuleId::new(0))).unwrap();
    fragment.links[0] = bsl_bytecode::LinkEntry::Function {
        module: ModuleId::new(1),
        func: 9,
    };
    assert!(image::verify_linked_program(&fragment, &catalog, Some(ModuleId::new(0))).is_err());
}
