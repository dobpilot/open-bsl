//! Понижение символических `Goto` в существующий `Instr::Jump`.

use bsl_bytecode::Instr;
use bsl_compiler::{CompileError, compile_program};

fn compile(src: &str) -> Result<bsl_bytecode::Program, CompileError> {
    let parsed = bsl_syntax::parse(src).expect("разбор");
    let resolved = bsl_sema::resolve_program(&parsed.items).expect("резолвинг");
    compile_program(&resolved)
}

#[test]
fn forward_backward_and_end_labels_patch_absolute_jump_targets() {
    let program = compile(
        "Goto ~Вперёд;\n\
         ~Назад: Goto ~Конец;\n\
         ~Вперёд: Goto ~Назад;\n\
         ~Конец:;",
    )
    .expect("компиляция");
    assert_eq!(
        program.chunks[0].instrs,
        vec![
            Instr::Jump { target: 2 },
            Instr::Jump { target: 3 },
            Instr::Jump { target: 1 },
        ]
    );
}

#[test]
fn adjacent_labels_may_share_one_pc() {
    let program = compile("Goto ~А; ~А:; ~Б:; Goto ~Б;").expect("компиляция");
    assert_eq!(
        program.chunks[0].instrs,
        vec![Instr::Jump { target: 1 }, Instr::Jump { target: 1 }]
    );
}

fn long_goto(n: usize) -> String {
    let mut src = String::from("Goto ~Конец;\n");
    for i in 0..n {
        src.push_str(&format!("Х = {i};\n"));
    }
    src.push_str("~Конец:;\n");
    src
}

#[test]
fn goto_uses_the_checked_i16_target_conversion() {
    compile(&long_goto(30_000)).expect("цель в диапазоне");
    assert!(matches!(
        compile(&long_goto(40_000)),
        Err(CompileError::JumpTargetOutOfRange)
    ));
}
