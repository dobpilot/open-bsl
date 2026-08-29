//! Перестановка базы одноаргументного вызова на источник копии
//! (`docs/ssa-hotspot-analysis.md`, «Ворота шага 6»).
//!
//! Копия в окно аргументов — та самая, которую план считал неустранимой
//! без распределения регистров. Устранима она там, где вызов окно ТОЛЬКО
//! ЧИТАЕТ: тогда базой может быть любой регистр, и переставить её на
//! источник дешевле, чем копировать значение в окно.

use bsl_bytecode::Instr;
use bsl_compiler::{Optimizations, compile_program_with};

fn compile(src: &str) -> bsl_bytecode::Program {
    let parsed = bsl_syntax::parse(src).expect("разбор");
    let resolved = bsl_sema::resolve_program(&parsed.items).expect("резолвинг");
    compile_program_with(
        &resolved,
        Optimizations {
            copy_elim: true,
            ..Optimizations::default()
        },
    )
    .expect("компиляция")
}

/// Метод базового рантайма читает окно через `CallArgs::load`, то есть
/// копирует его до исполнения. Значит и копия получателя, и копия
/// единственного аргумента снимаются: остаётся один опкод.
#[test]
fn a_single_argument_method_reads_its_argument_where_it_lives() {
    let program = compile("М = Новый Массив;\nХ = 5;\nМ.Добавить(Х);");
    let chunk = &program.chunks[0];

    assert!(
        !chunk.instrs.iter().any(|i| matches!(i, Instr::Move { .. })),
        "копий остаться не должно: {:?}",
        chunk.instrs
    );
    assert!(
        chunk.instrs.iter().any(|i| matches!(
            i,
            Instr::CallMethod {
                obj: 0,
                base: 1,
                count: 1,
                ..
            }
        )),
        "вызов обязан читать получателя и аргумент по месту: {:?}",
        chunk.instrs
    );
}

/// А вот у вызова функции BSL копия обязана остаться, и это не
/// перестраховка. Слот окна СТАНОВИТСЯ параметром вызванной функции через
/// `ParamSlot`, то есть окно и есть та приватная копия, которой требует
/// `Знач`. Переставь базу на переменную вызывающего — и присваивание
/// внутри функции запишет прямо в неё.
#[test]
fn a_bsl_call_keeps_the_copy_that_makes_znach_private() {
    let program = compile(
        "Функция Испортить(Знач Х)\n\
         \tХ = 99;\n\
         \tВозврат Х;\n\
         КонецФункции\n\
         А = 1;\n\
         Испортить(А);",
    );
    let chunk = &program.chunks[0];

    assert!(
        chunk.instrs.iter().any(|i| matches!(i, Instr::Move { .. })),
        "копия аргумента `Знач` обязана остаться: {:?}",
        chunk.instrs
    );
}

/// Окно шире одного регистра не трогается: оно обязано быть непрерывным,
/// а источники соседних аргументов лежат где придётся.
#[test]
fn a_two_argument_call_keeps_its_window() {
    let program = compile("Х = 1;\nУ = 2;\nСообщить(Строка(Х) + Строка(У));");
    let chunk = &program.chunks[0];

    assert!(
        chunk.instrs.iter().any(|i| matches!(
            i,
            Instr::CallBuiltin { count, .. } if *count >= 2
        )),
        "ожидался вызов с окном шире одного регистра: {:?}",
        chunk.instrs
    );
}
