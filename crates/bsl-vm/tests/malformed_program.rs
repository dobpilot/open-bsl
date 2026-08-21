//! `Program` — публичный тип с публичными полями, поэтому VM не вправе
//! считать её заведомо корректной.
//!
//! Проверка целей при разборе текстового байт-кода защищает только путь
//! `--run-bytecode`. Rust-клиент, собравший или подправивший `Program`
//! напрямую, до этой правки получал `Ok(Неопределено)` вместо ошибки: VM
//! переводила цель через `as usize`, а `pc` за концом чанка принимала за
//! нормальное завершение — и остаток программы молча пропадал.

use bsl_bytecode::{Instr, Program, compile_program};
use bsl_rt::RtError;

fn compile(src: &str) -> Program {
    let parsed = bsl_syntax::parse(src).expect("разбор");
    let resolved = bsl_sema::resolve_program(&parsed.items).expect("резолвинг");
    compile_program(&resolved).expect("кодоген")
}

/// Ставит цель `target` первой инструкции-переходу в нулевом чанке.
fn break_first_jump(program: &mut Program, target: i16) {
    for instr in &mut program.chunks[0].instrs {
        match instr {
            Instr::Jump { target: t }
            | Instr::JumpIfFalse { target: t, .. }
            | Instr::JumpIfTrue { target: t, .. }
            | Instr::JumpIfNotSkipped { target: t, .. }
            | Instr::NumericForNext { target: t, .. }
            | Instr::NumericForNextI64 { target: t, .. } => {
                *t = target;
                return;
            }
            _ => {}
        }
    }
    panic!("в чанке нет ни одного перехода");
}

#[test]
fn a_jump_out_of_the_chunk_is_invalid_bytecode_and_not_a_silent_success() {
    for target in [-1, 9999] {
        let mut program = compile("Если Ложь Тогда\nСообщить(1);\nКонецЕсли;\n");
        break_first_jump(&mut program, target);
        assert!(
            matches!(
                bsl_vm::run_program(&program),
                Err(RtError::InvalidBytecode(_))
            ),
            "цель {target} обязана быть ошибкой"
        );
    }
}

#[test]
fn a_handler_out_of_the_chunk_is_invalid_bytecode_too() {
    let mut program = compile(
        "Попытка\nВызватьИсключение \"бум\";\nИсключение\nСообщить(\"поймано\");\nКонецПопытки;\n",
    );
    program.chunks[0].exception_ranges[0].handler_pc = 9999;
    assert!(matches!(
        bsl_vm::run_program(&program),
        Err(RtError::InvalidBytecode(_))
    ));
}

#[test]
fn a_correct_program_still_runs() {
    let program = compile("Если Ложь Тогда\nСообщить(1);\nКонецЕсли;\n");
    assert!(bsl_vm::run_program(&program).is_ok());
}
