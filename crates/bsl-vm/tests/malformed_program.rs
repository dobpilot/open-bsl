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
/// Ставит цель `target` переходу номер `nth` в нулевом чанке.
/// `None` — переходов столько не нашлось.
fn break_jump(program: &mut Program, nth: usize, target: i16) -> Option<()> {
    let mut seen = 0;
    for instr in &mut program.chunks[0].instrs {
        match instr {
            Instr::Jump { target: t }
            | Instr::JumpIfFalse { target: t, .. }
            | Instr::JumpIfTrue { target: t, .. }
            | Instr::JumpIfNotSkipped { target: t, .. }
            | Instr::NumericForNext { target: t, .. }
            | Instr::NumericForNextI64 { target: t, .. } => {
                if seen == nth {
                    *t = target;
                    return Some(());
                }
                seen += 1;
            }
            _ => {}
        }
    }
    None
}

#[test]
fn every_jump_in_the_chunk_is_checked_not_only_the_first() {
    // Портится КАЖДЫЙ переход по очереди: проверка, глядящая лишь на
    // первый, прошла бы мимо остальных пяти опкодов с целью.
    let src = concat!(
        "Сумма = 0;\n",
        "Если Истина ИЛИ Ложь Тогда\n",
        "Для н = 1 По 5 Цикл\n",
        "Сумма = Сумма + н;\n",
        "КонецЦикла;\n",
        "Иначе\n",
        "Сообщить(1);\n",
        "КонецЕсли;\n",
    );
    for nth in 0..16 {
        for target in [-1, 9999] {
            let mut program = compile(src);
            if break_jump(&mut program, nth, target).is_none() {
                assert!(nth > 0, "в чанке не нашлось ни одного перехода");
                return;
            }
            assert!(
                matches!(
                    bsl_vm::run_program(&program),
                    Err(RtError::InvalidBytecode(_))
                ),
                "переход {nth} с целью {target} обязан быть ошибкой"
            );
        }
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
