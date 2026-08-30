//! Текстовый формат со стороны РАЗБОРА: что парсер обязан принять и что
//! обязан отвергнуть.
//!
//! Программы здесь собраны руками (см. `support`), а не скомпилированы:
//! проверяется представление и его формат, а не кодоген, и заводить ради
//! этого зависимость от фронтенда незачем. Round-trip на настоящем
//! скомпилированном корпусе живёт в `bsl-compiler`.

mod support;

use bsl_bytecode::{BytecodeConst, Instr, TextError, parse_program, write_program};
use bsl_rt::{BslValue, NameId};
use support::{chunk, every_section, program, shapes};

/// Программа с одной константой — минимальный листинг, который есть что
/// портить.
fn one_const() -> bsl_bytecode::Program {
    let mut c = chunk(vec![
        Instr::LoadConst { dst: 0, k: 0 },
        Instr::Return { src: None },
    ]);
    c.consts = vec![support::konst(BslValue::number_from_i64(1))];
    program(vec![c])
}

#[test]
fn comments_and_blank_lines_are_ignored_by_the_parser() {
    let program = one_const();
    let text = write_program(&program, Some("файл.bsl")).unwrap();
    // В печати комментарии есть...
    assert!(text.contains("; исходник: файл.bsl"));
    // ...и они не мешают разобрать её обратно.
    let reparsed = parse_program(&text).unwrap();
    assert_eq!(reparsed.chunks[0].instrs, program.chunks[0].instrs);

    // Комментарий, добавленный руками, тоже не мешает.
    let hand_edited = text.replace(".code", "; моя пометка\n  .code");
    assert!(parse_program(&hand_edited).is_ok());
}

#[test]
fn a_semicolon_inside_a_string_constant_is_not_a_comment() {
    let mut c = chunk(vec![
        Instr::LoadConst { dst: 0, k: 0 },
        Instr::Return { src: None },
    ]);
    c.consts = vec![support::konst(BslValue::Str(bsl_rt::BslString::from_str(
        "а;б",
    )))];
    let program = program(vec![c]);
    let text = write_program(&program, None).unwrap();
    let reparsed = parse_program(&text).unwrap();
    assert_eq!(reparsed.chunks[0].consts, program.chunks[0].consts);
}

/// Вид объявления в листинге: печатается только у процедур, читается
/// обратно, а у ВЕРХНЕГО УРОВНЯ невозможен — у него нет объявления.
/// Последнее держится обеими границами формата: печать поля нулевому
/// чанку не даёт, разбор такое поле отвергает, поэтому рукописный
/// листинг не может завести состояние, объявленное невозможным.
#[test]
fn the_declaration_kind_is_printed_read_back_and_denied_to_the_top_level() {
    let mut procedure = chunk(vec![Instr::Return { src: None }]);
    procedure.is_procedure = true;
    let function = chunk(vec![Instr::Return { src: Some(0) }]);
    let mut program = program(vec![
        chunk(vec![Instr::Return { src: None }]),
        procedure,
        function,
    ]);
    program.function_names = vec!["П".to_string(), "Ф".to_string()];

    let text = write_program(&program, None).unwrap();
    assert_eq!(text.matches("kind=proc").count(), 1, "{text}");
    assert!(!text.contains("kind=func"), "у функции поля быть не должно");

    let reparsed = parse_program(&text).unwrap();
    let kinds: Vec<bool> = reparsed.chunks.iter().map(|c| c.is_procedure).collect();
    assert_eq!(kinds, vec![false, true, false], "верхний уровень, П, Ф");

    // Поле у нулевого чанка — ошибка разбора, а не молчаливое «ну и
    // пусть»: состояние объявлено невозможным.
    let smuggled = text.replacen(".chunk 0 params=", ".chunk 0 kind=proc params=", 1);
    match parse_program(&smuggled) {
        Err(TextError::At(_, msg)) => assert!(msg.contains("верхнего уровня"), "{msg}"),
        other => panic!("ожидалась ошибка разбора, получено {other:?}"),
    }

    // Незнакомое значение — тоже ошибка, а не «значит, функция».
    let unknown = text.replace("kind=proc", "kind=подпрограмма");
    match parse_program(&unknown) {
        Err(TextError::At(_, msg)) => assert!(msg.contains("подпрограмма"), "{msg}"),
        other => panic!("ожидалась ошибка разбора, получено {other:?}"),
    }

    // Печать нулевого чанка не зависит от флага: у верхнего уровня
    // вида объявления нет по построению, и текстового представления у
    // такого состояния тоже нет.
    let mut tampered = program.clone();
    tampered.chunks[0].is_procedure = true;
    let printed = write_program(&tampered, None).unwrap();
    assert_eq!(printed.matches("kind=proc").count(), 1, "{printed}");
    assert!(parse_program(&printed).is_ok());
}

#[test]
fn a_foreign_or_missing_header_is_rejected() {
    assert!(matches!(
        parse_program("bslc 999\n.names 0\n"),
        Err(TextError::BadHeader(_))
    ));
    assert!(matches!(parse_program(""), Err(TextError::BadHeader(_))));
    assert!(matches!(
        parse_program("что-то другое\n"),
        Err(TextError::BadHeader(_))
    ));
}

#[test]
fn a_corrupted_body_names_the_line() {
    let text = write_program(&one_const(), None).unwrap();

    // Неизвестный опкод.
    let broken = text.replace("LoadConst", "ЛюбиРаботу");
    match parse_program(&broken) {
        Err(TextError::At(line, msg)) => {
            assert!(msg.contains("ЛюбиРаботу"), "{msg}");
            assert!(line > 0);
        }
        other => panic!("ожидалась ошибка с номером строки, получено {other:?}"),
    }

    // Сбитая нумерация инструкций — признак ручной правки, при которой
    // прыжки поехали бы молча.
    let broken = text.replace("0000 ", "0007 ");
    assert!(matches!(parse_program(&broken), Err(TextError::At(..))));

    // Обрыв файла на середине.
    let broken: String = text.lines().take(4).collect::<Vec<_>>().join("\n");
    assert!(parse_program(&broken).is_err());
}

/// Объект в таблице констант печатью отвергается, а не печатается
/// неверно.
///
/// Положить его туда можно ровно одним входом — `BytecodeConst::transient`,
/// и существует тот вход ради фоновых заданий, где таблица констант
/// служит транспортом аргументов, а программа не печатается никогда.
/// Этот тест — вторая половина той же договорённости: если такая
/// программа всё же дойдёт до печати, печать откажет.
#[test]
fn objects_in_constants_are_refused_rather_than_printed_wrong() {
    let mut program = one_const();
    program.chunks[0]
        .consts
        .push(BytecodeConst::transient(BslValue::new_array(Vec::new())));
    assert!(matches!(
        write_program(&program, None),
        Err(TextError::Unrepresentable(_))
    ));
}

/// Программа-образец со всеми непустыми секциями — та же, на которой
/// проверяются счётчики, — обязана печататься и читаться обратно без
/// потерь. Без этого испорченные варианты в соседних тестах ничего не
/// доказывали бы: отвергнуть можно и целый листинг.
#[test]
fn a_hand_built_program_survives_print_and_parse() {
    let program = every_section();
    let text = write_program(&program, None).unwrap();
    let reparsed =
        parse_program(&text).unwrap_or_else(|e| panic!("образец обязан читаться: {e}\n{text}"));
    assert_eq!(
        write_program(&reparsed, None).unwrap(),
        text,
        "round-trip разошёлся"
    );
    assert_eq!(reparsed.names, program.names);
    assert_eq!(reparsed.module_vars, program.module_vars);
    assert_eq!(reparsed.function_names, program.function_names);
    assert_eq!(reparsed.chunks[0].consts, program.chunks[0].consts);
    assert_eq!(
        reparsed.chunks[0].exception_ranges,
        program.chunks[0].exception_ranges
    );
    assert_eq!(
        reparsed.chunks[0].local_names,
        program.chunks[0].local_names
    );
    // Производные поля разбор пересчитывает, а не читает.
    assert!(reparsed.chunks[0].touches_objects);
    assert_eq!(
        reparsed.chunks[0].prop_cache.len(),
        reparsed.chunks[0].instrs.len()
    );
    assert_eq!(reparsed.shapes.len(), 1);
    assert_eq!(
        reparsed.shapes[0].names,
        shapes(&[NameId::from_index(0)])[0].names
    );
}
