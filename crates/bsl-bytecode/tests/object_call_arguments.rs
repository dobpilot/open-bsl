mod support;

use bsl_bytecode::{
    ArgMode, Instr, LinkEntry, ModuleId, Program, image, parse_program, write_program,
};

fn sample() -> Program {
    let mut chunk = support::chunk(vec![
        Instr::CallObjectMethod {
            dst: 2,
            obj: 0,
            method: 0,
            base: 1,
            arg_modes: 0,
        },
        Instr::Return { src: None },
    ]);
    chunk.n_regs = 3;
    chunk.n_locals = 1;
    chunk.call_arg_modes = vec![vec![ArgMode::ByRefLocal(0)]];
    let mut program = support::program(vec![chunk]);
    program.names = vec!["Метод".into()];
    image::finalize(&mut program);
    program
}

#[test]
fn open_call_modes_survive_round_trip_and_image_validation() {
    for mode in [
        ArgMode::Value,
        ArgMode::Default,
        ArgMode::ByRefLocal(0),
        ArgMode::ByRefModuleVar(0),
        ArgMode::ByRefImportedVar(0),
        ArgMode::ByRefIndex {
            object: 1,
            index: 2,
        },
    ] {
        let mut program = sample();
        program.module_vars = vec!["М".into()];
        program.exported_module_vars = vec![false];
        program.links = vec![LinkEntry::Variable {
            module: ModuleId::new(0),
            slot: 0,
        }];
        program.chunks[0].call_arg_modes[0][0] = mode;
        image::finalize(&mut program);
        image::verify(&program).unwrap();
        let text = write_program(&program, None).unwrap();
        let restored = parse_program(&text).unwrap();
        image::verify(&restored).unwrap();
        assert_eq!(
            restored.chunks[0].call_arg_modes,
            program.chunks[0].call_arg_modes
        );
    }
}

#[test]
fn malformed_open_call_modes_are_rejected_with_or_without_bundles() {
    let mutations: &[fn(&mut Program)] = &[
        |p| p.names.clear(),
        |p| p.chunks[0].call_arg_modes.clear(),
        |p| p.chunks[0].call_arg_modes[0] = vec![ArgMode::Value; 256],
        |p| p.chunks[0].call_arg_modes[0] = vec![ArgMode::Value; 3],
        |p| {
            p.chunks[0].n_regs = 255;
            if let Instr::CallObjectMethod { base, .. } = &mut p.chunks[0].instrs[0] {
                *base = 255;
            }
            p.chunks[0].call_arg_modes[0] = vec![ArgMode::Value; 2];
        },
        |p| p.chunks[0].call_arg_modes[0][0] = ArgMode::ByRefLocal(1),
        |p| p.chunks[0].call_arg_modes[0][0] = ArgMode::ByRefLocal(255),
        |p| p.chunks[0].call_arg_modes[0][0] = ArgMode::ByRefModuleVar(0),
        |p| p.chunks[0].call_arg_modes[0][0] = ArgMode::ByRefImportedVar(0),
        |p| {
            p.chunks[0].call_arg_modes[0][0] = ArgMode::ByRefIndex {
                object: 3,
                index: 2,
            };
        },
        |p| {
            p.chunks[0].call_arg_modes[0][0] = ArgMode::ByRefImportedVar(0);
            p.links = vec![LinkEntry::Function {
                module: ModuleId::new(0),
                func: 1,
            }];
        },
    ];
    image::verify(&sample()).unwrap();
    for (index, mutate) in mutations.iter().enumerate() {
        for bundled in [false, true] {
            let mut program = sample();
            mutate(&mut program);
            if bundled {
                image::finalize(&mut program);
            } else {
                image::finalize_unbundled(&mut program);
            }
            assert!(
                image::verify(&program).is_err(),
                "mutation={index}, bundled={bundled}"
            );
        }
    }
}

#[test]
fn open_call_argument_modes_are_required_in_the_text_format() {
    let text = write_program(&sample(), None).unwrap();
    assert!(text.contains("arg_modes=0"), "{text}");
    assert!(parse_program(&text.replace(" arg_modes=0", "")).is_err());
    assert!(parse_program(&text.replace("arg_modes=0", "arg_modes=none")).is_err());
    let procedure = text.replace("CallObjectMethod", "CallObjectProcedure");
    let restored = parse_program(&procedure).unwrap();
    assert!(matches!(
        restored.chunks[0].instrs[0],
        Instr::CallObjectProcedure { arg_modes: 0, .. }
    ));
}
