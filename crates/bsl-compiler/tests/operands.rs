//! Устранение безопасных копий операндов при кодогенерации.

use bsl_bytecode::Instr;
use bsl_compiler::compile_program;

fn compile(src: &str) -> bsl_bytecode::Program {
    let parsed = bsl_syntax::parse(src).expect("разбор");
    let resolved = bsl_sema::resolve_program(&parsed.items).expect("резолвинг");
    compile_program(&resolved).expect("компиляция")
}

// Свёртка констант превращает эти выражения в готовые `LoadConst`, и
// проверять на ней форму операндов бессмысленно: она проверяет выбор
// инструкций кодогеном, а не результат последующего прохода.
#[cfg_attr(
    feature = "constprop",
    ignore = "проверяет кодоген до свёртки констант"
)]
#[test]
fn a_local_next_to_a_number_is_read_without_a_move_in_both_positions() {
    let program = compile("А = 10; Б = А + 1; В = 20 - А;");
    let instrs = &program.chunks[0].instrs;

    assert!(
        !instrs
            .iter()
            .any(|instr| matches!(instr, Instr::Move { .. }))
    );
    assert!(matches!(instrs[1], Instr::AddConst { dst: 1, src: 0, .. }));
    assert!(matches!(instrs[3], Instr::Sub { dst: 2, a: 3, b: 0 }));
}

#[test]
fn a_local_is_still_copied_before_an_effectful_neighbour() {
    let program = compile(
        "Перем А;\n\
         Функция Изменить()\n\
         А = 20;\n\
         Возврат 1;\n\
         КонецФункции\n\
         А = 10;\n\
         Б = А + Изменить();",
    );
    let instrs = &program.chunks[0].instrs;

    assert!(
        instrs
            .iter()
            .any(|instr| matches!(instr, Instr::GetModuleVar { slot: 0, .. }))
    );
}

#[test]
fn local_number_equality_branches_without_a_boolean_temporary() {
    let program = compile(
        "А = 1; Если А = 1 Тогда Б = 2; КонецЕсли;\n\
         Пока 1 = А Цикл Прервать; КонецЦикла;\n\
         В = ?(А = 1, 3, 4);",
    );
    let instrs = &program.chunks[0].instrs;

    assert_eq!(
        instrs
            .iter()
            .filter(|instr| matches!(instr, Instr::JumpIfNotEqConst { src: 0, .. }))
            .count(),
        3
    );
}

#[cfg_attr(
    feature = "constprop",
    ignore = "проверяет кодоген до свёртки констант"
)]
#[test]
fn local_plus_number_uses_the_constant_directly_but_reverse_add_does_not() {
    let program = compile("А = 10; Б = А + 1; В = 20 + А;");
    let instrs = &program.chunks[0].instrs;

    assert!(matches!(instrs[1], Instr::AddConst { dst: 1, src: 0, .. }));
    assert!(
        instrs
            .iter()
            .any(|instr| matches!(instr, Instr::Add { dst: 2, a: 3, b: 0 }))
    );
}

#[test]
fn local_less_than_number_branches_without_a_boolean_temporary() {
    let program = compile("А = 0; Пока А < 10 Цикл А = А + 1; КонецЦикла;");

    assert!(
        program.chunks[0]
            .instrs
            .iter()
            .any(|instr| matches!(instr, Instr::JumpIfNotLtConst { src: 0, .. }))
    );
}
