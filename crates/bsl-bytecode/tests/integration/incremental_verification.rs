use crate::support;

use std::sync::Arc;

use bsl_bytecode::{Instr, ModuleId, ModuleImport, ModuleImports, image};

fn program() -> bsl_bytecode::Program {
    let mut program = support::program(vec![support::chunk(vec![Instr::Return { src: None }])]);
    program.names = vec!["Поле".into()].into();
    image::finalize(&mut program);
    program
}

#[test]
fn changing_a_previously_verified_names_table_requires_a_new_validation() {
    let mut program = program();
    let mut cache = image::VerificationCache::default();
    cache.verify(&program).unwrap();
    cache.verify(&program).unwrap();
    Arc::make_mut(&mut program.names)[0] = "Другое".into();
    cache.verify(&program).unwrap();
    program.names = vec!["Другое".into(), "ДРУГОЕ".into()].into();
    assert!(cache.verify(&program).is_err());
}

#[test]
fn caching_shared_tables_does_not_skip_local_checks() {
    let mut program = program();
    let mut cache = image::VerificationCache::default();
    cache.verify(&program).unwrap();
    program.function_names = vec!["НесуществующееТело".into()];
    assert!(cache.verify(&program).is_err());
}

#[test]
fn excluded_bad_aliases_do_not_change_the_uncached_verdict() {
    let common: ModuleImports = vec![
        ModuleImport {
            alias: String::new(),
            module: ModuleId::new(0),
            global: false,
        },
        ModuleImport {
            alias: "Корректное".into(),
            module: ModuleId::new(1),
            global: false,
        },
    ]
    .into();
    let mut program = program();
    program.imports.extend_shared(&common, 1..2);
    image::verify(&program).unwrap();
    let mut cache = image::VerificationCache::default();
    cache.verify(&program).unwrap();
    program.imports.extend_shared(&common, 1..2);
    assert!(image::verify(&program).is_err());
    assert!(cache.verify(&program).is_err());
}
