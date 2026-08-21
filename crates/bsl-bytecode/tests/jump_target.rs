//! Цели переходов помещаются в `i16`, и это проверяется, а не подразумевается.
//!
//! До этих проверок `patch_jump` усекал цель через `as i16`: в чанке на сорок
//! тысяч инструкций цель 40004 превращалась в -25532, VM получала `pc` далеко
//! за концом чанка, принимала это за нормальное завершение — и программа
//! молча заканчивалась без вывода и с нулевым кодом возврата. Неверный ответ,
//! а не ошибка.

use bsl_bytecode::{CompileError, TextError, compile_program, parse_program, write_program};

/// Модуль с ложной ветвью на `n` инструкций: `JumpIfFalse` в её начале
/// целится за `КонецЕсли`, то есть тем дальше, чем длиннее тело.
fn module_with_dead_branch(n: usize) -> String {
    let mut src = String::from("Х = 0;\nЕсли Ложь Тогда\n");
    for i in 0..n {
        src.push_str(&format!("Х = {i};\n"));
    }
    src.push_str("КонецЕсли;\nСообщить(\"после\");\n");
    src
}

fn compile(src: &str) -> Result<bsl_bytecode::Program, CompileError> {
    let parsed = bsl_syntax::parse(src).expect("разбор");
    let resolved = bsl_sema::resolve_program(&parsed.items).expect("резолвинг");
    compile_program(&resolved)
}

#[test]
fn a_chunk_within_the_limit_compiles_and_keeps_every_target_in_range() {
    let program = compile(&module_with_dead_branch(30_000)).expect("должно компилироваться");
    for chunk in &program.chunks {
        let limit = chunk.instrs.len();
        for instr in &chunk.instrs {
            let text = format!("{instr:?}");
            let Some(rest) = text.split("target: ").nth(1) else {
                continue;
            };
            let target: i64 = rest
                .trim_end_matches([' ', '}'])
                .split(&[',', ' '][..])
                .next()
                .unwrap()
                .parse()
                .unwrap();
            assert!(
                target >= 0 && target <= limit as i64,
                "цель {target} вне чанка длиной {limit}"
            );
        }
    }
}

#[test]
fn a_chunk_past_the_limit_is_a_compile_error_and_not_a_broken_program() {
    assert!(matches!(
        compile(&module_with_dead_branch(40_000)),
        Err(CompileError::JumpTargetOutOfRange)
    ));
}

#[test]
fn a_hand_edited_bytecode_target_is_rejected_before_execution() {
    // Файл байт-кода правят руками, и VM ему не верит — ровно как разметке
    // бандлов, которую парсер пересчитывает вместо чтения.
    let program = compile("Если Ложь Тогда\nСообщить(\"ветка\");\nКонецЕсли;\n").expect("сборка");
    let text = write_program(&program, None).expect("печать");
    assert!(
        parse_program(&text).is_ok(),
        "целый листинг обязан читаться"
    );

    // Подменяем цель ПЕРВОГО перехода, не полагаясь на её конкретное
    // значение: раскладка листинга — не предмет этого теста.
    let at = text
        .find("target=")
        .expect("в листинге должен быть переход");
    let end = at + text[at..].find(['\n', ' ']).unwrap_or(text.len() - at);
    for bad in ["target=-25532", "target=9999"] {
        let mut broken = text.clone();
        broken.replace_range(at..end, bad);
        assert_ne!(broken, text, "подмена цели не сработала");
        assert!(
            matches!(parse_program(&broken), Err(TextError::BadJumpTarget { .. })),
            "должно отвергаться: {bad}"
        );
    }
}

/// Наибольшая цель перехода в программе — по всем опкодам, которые её несут.
fn max_target(program: &bsl_bytecode::Program) -> i64 {
    let mut best = i64::MIN;
    for chunk in &program.chunks {
        for instr in &chunk.instrs {
            let text = format!("{instr:?}");
            if let Some(rest) = text.split("target: ").nth(1) {
                let t: i64 = rest
                    .trim_end_matches([' ', '}'])
                    .split([',', ' '])
                    .next()
                    .unwrap()
                    .parse()
                    .unwrap();
                best = best.max(t);
            }
        }
    }
    best
}

#[test]
fn the_limit_is_exactly_i16_max_and_not_a_round_number_near_it() {
    // Смещение цели от числа операторов определяется прогоном, а не
    // константой в тесте: раскладка кодогенератора — не предмет проверки,
    // предмет — точная граница.
    let probe = compile(&module_with_dead_branch(1_000)).expect("проба");
    let offset = max_target(&probe) - 1_000;

    let last_ok = i64::from(i16::MAX) - offset;
    let program = compile(&module_with_dead_branch(last_ok as usize)).expect("граница берётся");
    assert_eq!(
        max_target(&program),
        i64::from(i16::MAX),
        "проба должна упираться ровно в i16::MAX"
    );

    assert!(
        matches!(
            compile(&module_with_dead_branch(last_ok as usize + 1)),
            Err(CompileError::JumpTargetOutOfRange)
        ),
        "следующий адрес обязан быть ошибкой"
    );
}

#[test]
fn tampered_numeric_for_targets_are_rejected_for_both_opcodes() {
    // Оба цикловых опкода несут цель и оба преобразуют её в VM через
    // `as usize`, поэтому листинг с `target=-1` завершал бы программу так
    // же молча, как испорченный `JumpIfFalse`.
    let sources = [
        // тело непустое — `NumericForNext`
        (
            "NumericForNext",
            "Сумма = 0;
Для н = 1 По 5 Цикл
Сумма = Сумма + н;
КонецЦикла;
",
        ),
        // тело доказанно пустое — `NumericForNextI64`
        (
            "NumericForNextI64",
            "Для н = 1 По 5 Цикл
КонецЦикла;
",
        ),
    ];
    for (opcode, src) in sources {
        let program = compile(src).expect("сборка");
        let text = write_program(&program, None).expect("печать");
        assert!(
            text.contains(opcode),
            "листинг должен содержать {opcode}: {text}"
        );

        let at = text.find(opcode).expect("опкод в листинге");
        let field = at + text[at..].find("target=").expect("у опкода есть цель");
        let end = field
            + text[field..]
                .find(['\n', ' '])
                .unwrap_or(text.len() - field);
        for bad in ["target=-1", "target=9999"] {
            let mut broken = text.clone();
            broken.replace_range(field..end, bad);
            assert!(
                matches!(parse_program(&broken), Err(TextError::BadJumpTarget { .. })),
                "{opcode} с {bad} обязан отвергаться"
            );
        }
    }
}
