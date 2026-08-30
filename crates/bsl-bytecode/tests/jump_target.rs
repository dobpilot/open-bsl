//! Цель перехода в правленом руками листинге — недостоверный вход.
//!
//! VM преобразует её в `pc` простым `as usize`, поэтому цель за концом
//! чанка уводила исполнение туда же, VM принимала это за нормальное
//! завершение — и программа с `ВызватьИсключение` заканчивалась без вывода
//! и с нулевым кодом возврата. Неверный ответ, а не ошибка. То же и с
//! обработчиком `Попытка`: это тоже цель передачи управления, только со
//! стороны разматывания, и доверия ему ровно столько же.
//!
//! Программы здесь собраны руками: предмет проверки — разбор листинга.
//! Что сам кодоген упирается в границу `i16` ошибкой, проверяется в
//! `bsl-compiler`.

mod support;

use bsl_bytecode::{ExceptionRange, Instr, TextError, image, parse_program, write_program};
use bsl_number::BslNumber;
use bsl_rt::{BslValue, RtError};
use support::{chunk, program};

/// Все ВОСЕМЬ опкодов, несущих цель, в одном чанке — по одному на каждый
/// вариант `Instr::jump_target`. Цели законные и указывают за последнюю
/// инструкцию: это нормальное завершение чанка.
fn every_target_carrying_opcode() -> Vec<(&'static str, Instr)> {
    let target = 8;
    vec![
        ("Jump", Instr::Jump { target }),
        ("JumpIfFalse", Instr::JumpIfFalse { cond: 0, target }),
        ("JumpIfTrue", Instr::JumpIfTrue { cond: 0, target }),
        (
            "JumpIfNotEqConst",
            Instr::JumpIfNotEqConst {
                src: 0,
                k: 0,
                target,
            },
        ),
        (
            "JumpIfNotLtConst",
            Instr::JumpIfNotLtConst {
                src: 0,
                k: 0,
                target,
            },
        ),
        (
            "JumpIfNotSkipped",
            Instr::JumpIfNotSkipped { src: 0, target },
        ),
        (
            "NumericForNext",
            Instr::NumericForNext {
                counter: 0,
                bound: 1,
                target,
            },
        ),
        (
            "NumericForNextI64",
            Instr::NumericForNextI64 {
                counter: 0,
                bound: 1,
                target,
            },
        ),
    ]
}

fn jumps() -> bsl_bytecode::Program {
    let mut c = chunk(
        every_target_carrying_opcode()
            .into_iter()
            .map(|(_, instr)| instr)
            .collect(),
    );
    c.n_regs = 2;
    c.consts
        .push(support::konst(BslValue::Number(BslNumber::from_i64(1))));
    program(vec![c])
}

/// Подменяет значение поля `target` в первой строке листинга, где после
/// `token` встречается это поле.
fn tamper(text: &str, token: &str, bad: &str) -> String {
    let at = text.find(token).unwrap_or_else(|| panic!("нет {token}"));
    let field = at + text[at..].find("target=").expect("у опкода есть цель");
    let end = field
        + text[field..]
            .find(['\n', ' '])
            .unwrap_or(text.len() - field);
    let mut broken = text.to_string();
    broken.replace_range(field..end, bad);
    assert_ne!(broken, text, "подмена цели не сработала");
    broken
}

#[test]
fn a_hand_edited_bytecode_target_is_rejected_before_execution() {
    let text = write_program(&jumps(), None).expect("печать");
    assert!(
        parse_program(&text).is_ok(),
        "целый листинг обязан читаться"
    );

    for bad in ["target=-25532", "target=9999"] {
        assert!(
            matches!(
                parse_program(&tamper(&text, "target=", bad)),
                Err(TextError::BadJumpTarget { .. })
            ),
            "должно отвергаться: {bad}"
        );
    }
}

#[test]
fn tampering_is_rejected_for_each_of_the_eight_target_carrying_opcodes() {
    // По одному опкоду на каждый вариант `Instr::jump_target`: удаление
    // любого из них оттуда должно ломать именно свою строку этого теста.
    let text = write_program(&jumps(), None).expect("печать");
    for (opcode, _) in every_target_carrying_opcode() {
        // Имя ищется С ПРОБЕЛОМ: «Jump» — префикс «JumpIfFalse», и без
        // этого случай `Jump` подменял бы цель условного перехода,
        // молча проверяя не тот опкод.
        let token = format!("{opcode} ");
        assert!(text.contains(&token), "листинг должен содержать {opcode}");
        for bad in ["target=-1", "target=9999"] {
            assert!(
                matches!(
                    parse_program(&tamper(&text, &token, bad)),
                    Err(TextError::BadJumpTarget { .. })
                ),
                "{opcode} с {bad} обязан отвергаться"
            );
        }
    }
}

#[test]
fn fused_equality_jump_rejects_an_invalid_register_and_constant() {
    let make = |src, k| {
        let mut c = chunk(vec![Instr::JumpIfNotEqConst { src, k, target: 1 }]);
        c.consts
            .push(support::konst(BslValue::Number(BslNumber::from_i64(1))));
        program(vec![c])
    };

    assert!(matches!(
        image::verify(&make(1, 0)),
        Err(RtError::InvalidBytecode(
            "регистр условного перехода выходит за кадр"
        ))
    ));
    assert!(matches!(
        image::verify(&make(0, 1)),
        Err(RtError::InvalidBytecode(
            "номер константы условного перехода вне таблицы чанка"
        ))
    ));
}

#[test]
fn a_tampered_exception_handler_is_rejected_before_execution() {
    let mut c = chunk(vec![
        Instr::LoadUndefined { dst: 0 },
        Instr::Return { src: None },
    ]);
    c.exception_ranges = vec![ExceptionRange {
        start_pc: 0,
        end_pc: 1,
        handler_pc: 1,
    }];
    let text = write_program(&program(vec![c]), None).expect("печать");
    assert!(
        parse_program(&text).is_ok(),
        "целый листинг обязан читаться"
    );

    let at = text.find(".handlers").expect("в листинге есть обработчики");
    let line_start = at + text[at..].find('\n').expect("строка после директивы") + 1;
    let line_end = line_start + text[line_start..].find('\n').unwrap_or(0);
    let line = text[line_start..line_end].to_string();
    let head = line.split(';').next().unwrap_or(&line).trim().to_string();
    let mut nums: Vec<&str> = head.split_whitespace().collect();
    assert_eq!(nums.len(), 4, "строка обработчика: «{head}»");

    for bad in ["9999", "-1"] {
        nums[3] = bad;
        let broken = format!(
            "{}    {}\n{}",
            &text[..line_start],
            nums.join(" "),
            &text[line_end + 1..]
        );
        assert!(
            parse_program(&broken).is_err(),
            "обработчик {bad} обязан отвергаться"
        );
    }
}

#[test]
fn an_empty_handler_at_the_end_of_the_chunk_is_legal() {
    // `handler_pc == instrs.len()` — законная цель: пустой обработчик в
    // конце уводит управление за последнюю инструкцию, то есть в обычное
    // завершение, ровно как переход с целью `limit`. Строгое `>=` в
    // проверке однажды отвергло такую программу целиком.
    let mut c = chunk(vec![Instr::Return { src: None }]);
    c.exception_ranges = vec![ExceptionRange {
        start_pc: 0,
        end_pc: 1,
        handler_pc: 1,
    }];
    let program = program(vec![c]);
    assert_eq!(
        program.chunks[0].exception_ranges[0].handler_pc,
        program.chunks[0].instrs.len(),
        "обработчик обязан указывать ровно за конец чанка"
    );
    let text = write_program(&program, None).expect("печать");
    assert!(
        parse_program(&text).is_ok(),
        "собственный листинг обязан читаться обратно"
    );
}
