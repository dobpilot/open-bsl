use crate::support;

use bsl_bytecode::{
    BytecodeImage, ConfigurationProgram, ModuleId, ModuleImport, ModuleImports, ModuleProgram,
    Program, parse_image, write_image,
};

fn program(imports: ModuleImports) -> Program {
    let mut program = support::program(vec![support::chunk(vec![bsl_bytecode::Instr::Return {
        src: None,
    }])]);
    program.imports = imports;
    program
}

#[test]
fn decoding_shared_tail_and_explicit_prefix_reuses_the_first_table() {
    let common: ModuleImports = (0..4)
        .map(|index| ModuleImport {
            alias: format!("Модуль{index}"),
            module: ModuleId::new(index),
            global: index == 2,
        })
        .collect();
    let mut changed = ModuleImports::default();
    changed.extend_shared(&common, 0..1);
    changed.extend_shared(&common, 2..4);
    let image = BytecodeImage::Configuration {
        catalog: ConfigurationProgram {
            modules: [common, changed]
                .into_iter()
                .enumerate()
                .map(|(i, imports)| ModuleProgram {
                    name: format!("Программа{i}"),
                    entry_visible: true,
                    global: false,
                    program: program(imports),
                })
                .collect(),
        },
        entry: None,
    };
    let text = write_image(&image, None).unwrap();
    assert!(text.contains(".imports 1 shared-tail=2"));
    let parsed = parse_image(&text).unwrap();
    assert_eq!(write_image(&parsed, None).unwrap(), text);
    let BytecodeImage::Configuration { catalog, .. } = parsed else {
        panic!("каталог");
    };
    let first = &catalog.modules[0].program.imports;
    let second = &catalog.modules[1].program.imports;
    for (own, shared) in [(0, 0), (1, 2), (2, 3)] {
        assert!(std::ptr::eq(&second[own], &first[shared]));
    }
}
