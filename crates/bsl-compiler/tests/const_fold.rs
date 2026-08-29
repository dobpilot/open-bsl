//! Приёмочные тесты свёртки констант до эмиссии инструкций
//! (`docs/ssa-hotspot-analysis.md`, раздел «Константы: что сделано и куда
//! идти»).
//!
//! Включена ровно одна свёртка — та, что в кодогене. Поздний проход над
//! готовым байт-кодом выключен намеренно: он сворачивает и то, что ранняя
//! свёртка обязана оставить (`Б = 1; А = Б + 3;` он превращает в
//! `LoadConst 4`, потому что ведёт таблицу известных регистров), и под
//! общим флагом последнее ожидание таблицы нельзя было бы даже
//! сформулировать.

use bsl_bytecode::{Chunk, Instr};
use bsl_compiler::{Optimizations, compile_program_with};

fn compile(src: &str) -> bsl_bytecode::Program {
    let parsed = bsl_syntax::parse(src).expect("разбор");
    let resolved = bsl_sema::resolve_program(&parsed.items).expect("резолвинг");
    compile_program_with(
        &resolved,
        Optimizations {
            const_fold: true,
            ..Optimizations::default()
        },
    )
    .expect("компиляция")
}

/// Единственная константа, которую грузит чанк, как целое. Утверждение о
/// единственности здесь же: свёртка обязана оставить одну загрузку, а не
/// добавить готовый результат к операндам, которые уже загружены.
fn the_only_loaded_number(chunk: &Chunk) -> i64 {
    let loads: Vec<u16> = chunk
        .instrs
        .iter()
        .filter_map(|i| match i {
            Instr::LoadConst { k, .. } => Some(*k),
            _ => None,
        })
        .collect();
    assert_eq!(
        loads.len(),
        1,
        "ожидалась одна загрузка константы, получено {:?}",
        chunk.instrs
    );
    match &chunk.consts[loads[0] as usize] {
        bsl_rt::BslValue::Number(n) => n.to_i64_exact().expect("константа не целая"),
        v => panic!("константа не число: {v:?}"),
    }
}

#[test]
fn a_constant_sum_becomes_a_single_load() {
    let program = compile("А = 2 + 3;");
    let chunk = &program.chunks[0];

    assert_eq!(chunk.instrs.len(), 1, "{:?}", chunk.instrs);
    assert_eq!(the_only_loaded_number(chunk), 5);
    // Временные регистры операндов больше не нужны: у чанка остаётся
    // только сама локальная переменная. Это вторая половина выигрыша,
    // ради которой свёртка и переехала в кодоген.
    assert_eq!(chunk.n_regs, 1);
}

#[test]
fn a_nested_constant_expression_folds_to_its_result() {
    let program = compile("А = (2 + 3) * 4;");
    let chunk = &program.chunks[0];

    assert_eq!(chunk.instrs.len(), 1, "{:?}", chunk.instrs);
    assert_eq!(the_only_loaded_number(chunk), 20);
}

#[test]
fn a_negated_constant_expression_folds_to_its_result() {
    let program = compile("А = -(2 + 3);");
    let chunk = &program.chunks[0];

    assert_eq!(chunk.instrs.len(), 1, "{:?}", chunk.instrs);
    assert_eq!(the_only_loaded_number(chunk), -5);
}

#[test]
fn a_division_by_zero_keeps_its_instruction() {
    let program = compile("А = 1 / 0;");
    let chunk = &program.chunks[0];

    assert!(
        chunk.instrs.iter().any(|i| matches!(i, Instr::Div { .. })),
        "деление на ноль обязано бросить на исполнении, а не исчезнуть \
         на компиляции: {:?}",
        chunk.instrs
    );
}

#[test]
fn a_variable_operand_leaves_the_addition_to_run_time() {
    let program = compile("Б = 1; А = Б + 3;");
    let chunk = &program.chunks[0];

    assert!(
        chunk
            .instrs
            .iter()
            .any(|i| matches!(i, Instr::AddConst { .. })),
        "слева переменная — сворачивать нечего, литерал остаётся в опкоде: \
         {:?}",
        chunk.instrs
    );
}

/// Контракт свёртки — пять арифметических операций, а не только те, что
/// попали в таблицу приёмочных тестов. `Add`, `Mul` и `Neg` проверены
/// выше; остальные три обязаны сворачиваться так же.
#[test]
fn subtraction_division_and_modulo_fold_too() {
    for (src, expected) in [("А = 7 - 2;", 5), ("А = 12 / 4;", 3), ("А = 7 % 3;", 1)] {
        let program = compile(src);
        let chunk = &program.chunks[0];

        assert_eq!(chunk.instrs.len(), 1, "{src}: {:?}", chunk.instrs);
        assert_eq!(the_only_loaded_number(chunk), expected, "{src}");
    }
}

/// Остаток от деления на ноль отказывает так же, как деление, и по той же
/// причине обязан дожить до исполнения.
#[test]
fn a_modulo_by_zero_keeps_its_instruction() {
    let program = compile("А = 1 % 0;");
    let chunk = &program.chunks[0];

    assert!(
        chunk.instrs.iter().any(|i| matches!(i, Instr::Mod { .. })),
        "остаток по нулю обязан бросить на исполнении: {:?}",
        chunk.instrs
    );
}

/// Худший случай для `fold_const` — цепочка УНАРНЫХ минусов, а не
/// бинарная. У бинарной каждый уровень занимает временный регистр, и её
/// обрывает лимит кадра; у унарной операнд компилируется в тот же `dst`,
/// `alloc_temp` не вызывается, и глубину не ограничивает ничто. Именно на
/// ней обход и был квадратичным, пока `fold_const` не завёл память об
/// отказах.
///
/// Здесь проверяется результат, а не сложность: сложность доказывает
/// счётчик заходов в модульном тесте
/// `folding_visits_grow_linearly_with_expression_depth`, потому что
/// снаружи крейта её не видно, а порог по времени краснел бы от
/// посторонней нагрузки.
///
/// Глубина 120 — не осторожность, а чужой предел: фронтенд рекурсивен, и
/// в потоке с двумя мегабайтами стека (таком, как у обычного `#[test]`)
/// разбор с резолвингом переполняют стек уже на 200 уровнях. Замерено,
/// что происходит это одинаково со свёрткой и без неё.
#[test]
fn a_deep_unary_chain_folds_to_one_load() {
    let deep = format!("А = {}1;", "- ".repeat(120));
    let program = compile(&deep);
    let chunk = &program.chunks[0];

    assert_eq!(chunk.instrs.len(), 1, "{:?}", chunk.instrs);
    // Чётное число минусов — знак сохраняется.
    assert_eq!(the_only_loaded_number(chunk), 1);
}

/// Та же цепочка, но с переменной в основании: свернуть нечего, и обход
/// поддерева повторяется на каждом уровне впустую. Байт-код обязан выйти
/// побайтно тем же, что и без свёртки.
#[test]
fn a_deep_unary_chain_rooted_at_a_variable_compiles_unfolded() {
    let src = format!("Б = 1;\nА = {}Б;", "- ".repeat(120));

    let parsed = bsl_syntax::parse(&src).expect("разбор");
    let resolved = bsl_sema::resolve_program(&parsed.items).expect("резолвинг");
    let folded = compile_program_with(
        &resolved,
        Optimizations {
            const_fold: true,
            ..Optimizations::default()
        },
    )
    .expect("компиляция со свёрткой");
    let plain = bsl_compiler::compile_program(&resolved).expect("компиляция без свёртки");

    assert_eq!(
        folded.chunks[0].instrs, plain.chunks[0].instrs,
        "сворачивать нечего — байт-код обязан совпасть с обычным"
    );
}

/// То же самое на БИНАРНОЙ цепочке. Она проверяет другой путь кодогена —
/// тот, где каждый уровень занимает временный регистр, — и потому не
/// заменяет унарную проверку выше, а дополняет её. Границу самого лимита
/// кадра закрепляет модульный тест
/// `a_binary_chain_compiles_to_the_frame_limit_and_refuses_past_it`.
#[test]
fn a_deep_chain_rooted_at_a_variable_compiles_unfolded() {
    let src = format!("Б = 1;\nА = Б{};", " + 1".repeat(120));

    let parsed = bsl_syntax::parse(&src).expect("разбор");
    let resolved = bsl_sema::resolve_program(&parsed.items).expect("резолвинг");
    let folded = compile_program_with(
        &resolved,
        Optimizations {
            const_fold: true,
            ..Optimizations::default()
        },
    )
    .expect("компиляция со свёрткой");
    let plain = bsl_compiler::compile_program(&resolved).expect("компиляция без свёртки");

    assert_eq!(
        folded.chunks[0].instrs, plain.chunks[0].instrs,
        "сворачивать нечего — байт-код обязан совпасть с обычным"
    );
}

/// Ворота допуска свёртка не проходила, поэтому обычная сборка обязана
/// эмитить ровно то же, что и до неё.
#[test]
fn folding_is_off_unless_it_is_asked_for() {
    let parsed = bsl_syntax::parse("А = 2 + 3;").expect("разбор");
    let resolved = bsl_sema::resolve_program(&parsed.items).expect("резолвинг");
    let program = bsl_compiler::compile_program(&resolved).expect("компиляция");

    assert!(
        program.chunks[0]
            .instrs
            .iter()
            .any(|i| matches!(i, Instr::Add { .. })),
        "по умолчанию свёртки быть не должно: {:?}",
        program.chunks[0].instrs
    );
}
