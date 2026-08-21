//! `Program` — публичный тип с публичными полями, поэтому VM не вправе
//! считать её заведомо корректной.
//!
//! Проверка целей при разборе текстового байт-кода защищает только путь
//! `--run-bytecode`. Rust-клиент, собравший или подправивший `Program`
//! напрямую, до этой правки получал `Ok(Неопределено)` вместо ошибки: VM
//! переводила цель через `as usize`, а `pc` за концом чанка принимала за
//! нормальное завершение — и остаток программы молча пропадал.

use bsl_bytecode::{ArgMode, Instr, Program, compile_program};
use bsl_rt::RtError;

fn compile(src: &str) -> Program {
    let parsed = bsl_syntax::parse(src).expect("разбор");
    let resolved = bsl_sema::resolve_program(&parsed.items).expect("резолвинг");
    compile_program(&resolved).expect("кодоген")
}
/// Все переходы программы: `(номер чанка, позиция инструкции)`.
fn jumps(program: &Program) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    for (c, chunk) in program.chunks.iter().enumerate() {
        for (pc, instr) in chunk.instrs.iter().enumerate() {
            if instr.jump_target().is_some() {
                out.push((c, pc));
            }
        }
    }
    out
}

/// Имя опкода — первое слово его отладочной записи.
fn opcode_name(instr: &Instr) -> String {
    let text = format!("{instr:?}");
    text.split([' ', '{', '(']).next().unwrap_or("").to_string()
}

/// Исходник, дающий ВСЕ шесть опкодов с целью: условие с коротким
/// замыканием (`Jump`, `JumpIfFalse`, `JumpIfTrue`), цикл с телом и
/// пустой цикл (`NumericForNext`, `NumericForNextI64`) и вызов с
/// пропущенным параметром по умолчанию (`JumpIfNotSkipped`).
const ALL_SIX: &str = concat!(
    "Функция Ф(а = 3, б = 7)\n",
    "Возврат а + б;\n",
    "КонецФункции\n",
    "Сумма = 0;\n",
    "Если Истина ИЛИ Ложь Тогда\n",
    "Для н = 1 По 5 Цикл\n",
    "Сумма = Сумма + н;\n",
    "КонецЦикла;\n",
    "Для п = 1 По 3 Цикл\n",
    "КонецЦикла;\n",
    "Сумма = Сумма + Ф(1,);\n",
    "Иначе\n",
    "Сообщить(1);\n",
    "КонецЕсли;\n",
);

#[test]
fn the_sample_really_contains_every_target_carrying_opcode() {
    // Иначе следующий тест мог бы «покрывать шесть опкодов», не встретив
    // половины из них: именно так и вышло в первой редакции.
    let program = compile(ALL_SIX);
    let mut kinds: Vec<String> = jumps(&program)
        .into_iter()
        .map(|(c, pc)| opcode_name(&program.chunks[c].instrs[pc]))
        .collect();
    kinds.sort();
    kinds.dedup();
    assert_eq!(
        kinds,
        vec![
            "Jump",
            "JumpIfFalse",
            "JumpIfNotSkipped",
            "JumpIfTrue",
            "NumericForNext",
            "NumericForNextI64",
        ],
        "образец обязан содержать все шесть опкодов с целью"
    );
}

#[test]
fn every_jump_of_every_opcode_is_checked_wherever_it_sits() {
    // Портится КАЖДЫЙ переход во ВСЕХ чанках: проверка, глядящая лишь на
    // первый или лишь на нулевой чанк, прошла бы мимо остальных.
    let sites = jumps(&compile(ALL_SIX));
    assert!(sites.len() >= 6, "переходов должно быть не меньше шести");
    for (chunk, pc) in sites {
        for target in [-1, 9999] {
            let mut program = compile(ALL_SIX);
            let name = opcode_name(&program.chunks[chunk].instrs[pc]);
            match &mut program.chunks[chunk].instrs[pc] {
                Instr::Jump { target: t }
                | Instr::JumpIfFalse { target: t, .. }
                | Instr::JumpIfTrue { target: t, .. }
                | Instr::JumpIfNotSkipped { target: t, .. }
                | Instr::NumericForNext { target: t, .. }
                | Instr::NumericForNextI64 { target: t, .. } => *t = target,
                other => panic!("не переход: {other:?}"),
            }
            assert!(
                matches!(
                    bsl_vm::run_program(&program),
                    Err(RtError::InvalidBytecode(_))
                ),
                "{name} в чанке {chunk} на {pc} с целью {target} обязан быть ошибкой"
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

#[test]
fn an_empty_handler_at_the_end_unwinds_into_ordinary_termination() {
    // `handler_pc == instrs.len()` — законная цель, и проверять её надо не
    // только разбором листинга, но и ИСПОЛНЕНИЕМ: возврат строгого `>=` в
    // VM тест на разбор не сломал бы. Исключение здесь настоящее, поэтому
    // разматывание действительно приходит на `pc == len`.
    let program = compile(concat!(
        "Попытка\n",
        "ВызватьИсключение \"бум\";\n",
        "Исключение\n",
        "КонецПопытки;\n",
    ));
    let handler = program.chunks[0].exception_ranges[0].handler_pc;
    assert_eq!(
        handler,
        program.chunks[0].instrs.len(),
        "обработчик обязан указывать ровно за конец чанка"
    );
    assert!(
        bsl_vm::run_program(&program).is_ok(),
        "пустой обработчик в конце обязан ловить исключение и завершать программу"
    );
}

/// Режим `ArgMode::Default` обязывает вызванную функцию вычислить в этот
/// слот значение по умолчанию. У функции без умолчаний прологa нет, и слот
/// остался бы с тем, что вызывающий успел положить в свой временный
/// регистр раньше, — то есть результат зависел бы от чужого выражения.
/// Поэтому VM обнуляет слот при построении кадра: испорченный извне режим
/// даёт `Неопределено`, одно и то же при любом соседнем коде.
#[test]
fn a_default_mode_without_a_default_prologue_yields_undefined_not_leftovers() {
    let src = "Функция Ф(а)\nВозврат а;\nКонецФункции\nВозврат Ф(22);\n";
    // Контроль: без правки вызов возвращает переданное значение — значит
    // временный регистр действительно занят числом 22, и «Неопределено»
    // ниже берётся из обнуления, а не из пустоты.
    assert_eq!(
        bsl_vm::run_program(&compile(src)).unwrap(),
        bsl_rt::BslValue::Number(bsl_number::BslNumber::from_i64(22))
    );

    let mut program = compile(src);
    let modes = program.chunks[0]
        .instrs
        .iter()
        .find_map(|i| match i {
            Instr::Call { arg_modes, .. } => Some(*arg_modes as usize),
            _ => None,
        })
        .expect("в чанке верхнего уровня обязан быть вызов");
    program.chunks[0].call_arg_modes[modes][0] = ArgMode::Default;

    assert_eq!(
        bsl_vm::run_program(&program).unwrap(),
        bsl_rt::BslValue::Undefined
    );
}
