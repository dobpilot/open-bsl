//! Приёмочные тесты сведений об отладке: таблица строк, имена локальных и
//! отказ на сочетание с удаляющей оптимизацией
//! (`openspec/changes/bytecode-line-table`).

use bsl_compiler::{BuildOptions, CompileError, Optimizations, compile_program_with};

fn build(src: &str, opts: BuildOptions) -> Result<bsl_bytecode::Program, CompileError> {
    let parsed = bsl_syntax::parse(src).expect("разбор");
    let resolved = bsl_sema::resolve_program(&parsed.items).expect("резолвинг");
    compile_program_with(&resolved, opts)
}

fn with_debug(src: &str) -> bsl_bytecode::Program {
    build(
        src,
        BuildOptions {
            debug_info: true,
            ..BuildOptions::default()
        },
    )
    .expect("компиляция со сведениями об отладке")
}

#[test]
fn every_instruction_of_every_chunk_gets_a_line() {
    let program = with_debug("а = 1;\nб = а + 2;\nСообщить(б);\n");
    assert_eq!(program.lines.len(), program.chunks.len());
    for (chunk, lines) in program.chunks.iter().zip(&program.lines) {
        assert_eq!(lines.len(), chunk.instrs.len());
    }
}

#[test]
fn a_line_lies_inside_the_source() {
    let src = "а = 1;\nб = а + 2;\nСообщить(б);\n";
    let total = src.lines().count() as u32;
    let program = with_debug(src);
    for lines in &program.lines {
        for &l in lines {
            assert!(
                (1..=total).contains(&l),
                "строка {l} вне исходника в 1..={total}"
            );
        }
    }
}

#[test]
fn instructions_carry_the_line_of_their_own_statement() {
    // Пустые строки и комментарий сдвигают нумерацию: ошибка «считаем
    // операторы» дала бы 1, 2, 3.
    let program = with_debug("а = 1;\n\n// комментарий\nб = 2;\n\n\nв = 3;\n");
    let mut seen: Vec<u32> = program.lines[0].clone();
    seen.dedup();
    assert_eq!(seen, vec![1, 4, 7]);
}

#[test]
fn a_parameter_default_carries_the_line_it_was_declared_on() {
    // Пролог умолчания выпускается ДО тела, и его строка — строка
    // объявления параметра, а не первого оператора функции.
    let program = with_debug("Функция Ф(\n  а,\n  б = 2)\n  Возврат а;\nКонецФункции\nФ(1);");
    let body = &program.lines[1];
    // Первые инструкции чанка — пролог умолчания второго параметра.
    assert_eq!(body[0], 3, "пролог умолчания несёт строку объявления");
    assert_eq!(
        *body.last().expect("тело непусто"),
        4,
        "тело несёт свою строку"
    );
}

#[test]
fn a_body_without_return_has_as_many_lines_as_instructions() {
    // Завершающей инструкции компилятор не выпускает: возврат делает VM,
    // дойдя до конца кода. Записи для неё поэтому и не нужно.
    let program = with_debug("Функция Ф(а)\n  б = а + 1;\nКонецФункции\nФ(1);");
    assert_eq!(program.lines[1].len(), program.chunks[1].instrs.len());
}

#[test]
fn local_names_are_materialized_for_every_chunk_under_debug_info() {
    let src = "Функция Ф(а)\n  б = а + 1;\n  Возврат б;\nКонецФункции\nФ(1);";
    let with = with_debug(src);
    assert_eq!(with.chunks[1].local_names, vec!["а", "б"]);

    // Без сведений об отладке правило прежнее: имена только у чанков с
    // `Выполнить`/`Вычислить`, а у этого их нет.
    let without = build(src, BuildOptions::default()).expect("компиляция");
    assert!(without.chunks[1].local_names.is_empty());
}

#[test]
fn without_debug_info_the_table_is_empty() {
    let program = build("а = 1;\n", BuildOptions::default()).expect("компиляция");
    assert!(program.lines.is_empty());
}

#[test]
fn debug_info_with_a_removing_pass_is_refused() {
    let err = build(
        "а = 1;\n",
        BuildOptions {
            debug_info: true,
            optimizations: Optimizations {
                copy_elim: true,
                ..Optimizations::default()
            },
        },
    )
    .expect_err("сочетание обязано отвергаться");
    assert!(matches!(err, CompileError::DebugInfoWithRemovingPass));
}

#[test]
fn debug_info_with_a_non_removing_pass_is_allowed() {
    let program = build(
        "а = 1 + 2;\n",
        BuildOptions {
            debug_info: true,
            optimizations: Optimizations {
                const_fold: true,
                ..Optimizations::default()
            },
        },
    )
    .expect("неудаляющий проход совместим со сведениями об отладке");
    for (chunk, lines) in program.chunks.iter().zip(&program.lines) {
        assert_eq!(lines.len(), chunk.instrs.len());
    }
}

/// Фрагмент `Выполнить`/`Вычислить` считает строки от начала СВОЕГО текста.
///
/// Это решение изменения `bytecode-line-table`, а не следствие реализации:
/// текст фрагмента — значение времени исполнения и может не лежать ни в
/// одном файле, поэтому наложить его строки на файл с вызовом нельзя.
#[test]
fn a_fragment_counts_lines_from_its_own_text() {
    let request = bsl_bytecode::DynamicRequest {
        source: "б = 1;\nв = 2;",
        debug_info: true,
        kind: bsl_bytecode::DynamicKind::Execute,
        scope: bsl_bytecode::DynamicScope {
            program: bsl_bytecode::DynamicScope::ROOT,
            chunk: 0,
        },
        caller_is_async: false,
        locals: &[],
        module_vars: &[],
        functions: &[],
        names: &[],
        requirements: &[bsl_bytecode::LibraryRequirement::bsl_rt()],
    };
    let unit = bsl_compiler::compile_dynamic_snippet(
        &request,
        None,
        &bsl_syntax::PreprocSymbols::new(),
        std::num::NonZeroU64::new(1).expect("не ноль"),
    )
    .expect("компиляция фрагмента");
    assert_eq!(unit.lines.len(), unit.chunk.instrs.len());
    // Обе строки — собственные строки фрагмента, 1 и 2, а не строка
    // вызова в файле.
    let mut seen = unit.lines.clone();
    seen.dedup();
    assert_eq!(seen, vec![1, 2]);
}

#[test]
fn a_fragment_without_debug_info_carries_no_lines() {
    let request = bsl_bytecode::DynamicRequest {
        source: "б = 1;",
        debug_info: false,
        kind: bsl_bytecode::DynamicKind::Execute,
        scope: bsl_bytecode::DynamicScope {
            program: bsl_bytecode::DynamicScope::ROOT,
            chunk: 0,
        },
        caller_is_async: false,
        locals: &[],
        module_vars: &[],
        functions: &[],
        names: &[],
        requirements: &[bsl_bytecode::LibraryRequirement::bsl_rt()],
    };
    let unit = bsl_compiler::compile_dynamic_snippet(
        &request,
        None,
        &bsl_syntax::PreprocSymbols::new(),
        std::num::NonZeroU64::new(1).expect("не ноль"),
    )
    .expect("компиляция фрагмента");
    assert!(unit.lines.is_empty());
}

/// Образ со сведениями об отладке собирается НЕПУЧКОВАННЫМ.
///
/// Бандл исполняется одним заходом диспетчера, и остановиться внутри него
/// негде: точка останова на втором члене сработала бы только вместе с
/// первым и всеми остальными.
#[test]
fn a_debug_image_carries_no_bundle_markup() {
    let src = "а = 1;\nб = 2;\nв = а + б;\nСообщить(в);\n";
    let plain = build(src, BuildOptions::default()).expect("без отладки");
    // Предусловие: без отладки разметка ЕСТЬ — иначе тест ниже проходил
    // бы и на программе, которую нечем пучковать.
    assert!(
        plain.chunks.iter().any(|c| !c.bundle_len().is_empty()),
        "предусловие: обычный образ размечен бандлами"
    );

    let dbg = with_debug(src);
    for chunk in &dbg.chunks {
        assert!(
            chunk.bundle_len().is_empty(),
            "у отладочного образа осталась разметка бандлов"
        );
    }
}
