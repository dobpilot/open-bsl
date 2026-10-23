mod support;

use bsl_bytecode::{Instr, image, parse_program, write_program};
use bsl_rt::BslValue;

#[test]
fn fatal_module_compilation_instruction_round_trips_and_bounds_its_register() {
    let mut chunk = support::chunk(vec![
        Instr::LoadConst { dst: 0, k: 0 },
        Instr::RaiseCompilationError { src: 0 },
    ]);
    chunk.consts = vec![support::konst(BslValue::Str("ошибка компиляции".into()))];
    let mut program = support::program(vec![chunk]);
    image::finalize(&mut program);
    image::verify(&program).unwrap();
    let text = write_program(&program, None).unwrap();
    assert!(text.contains("RaiseCompilationError src=0"));
    assert_eq!(
        write_program(&parse_program(&text).unwrap(), None).unwrap(),
        text
    );
    let invalid = parse_program(&text.replace(
        "RaiseCompilationError src=0",
        "RaiseCompilationError src=255",
    ))
    .unwrap();
    assert!(image::verify(&invalid).is_err());
    program.chunks[0].instrs[1] = Instr::RaiseCompilationError { src: 1 };
    assert!(image::verify(&program).is_err());
}
