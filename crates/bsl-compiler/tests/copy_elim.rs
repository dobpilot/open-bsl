//! Перестановка базы одноаргументного вызова на источник копии
//! (`docs/research/performance/ssa-hotspot-analysis.md`, «Ворота шага 6»).
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
///
/// Утверждать «есть вызов с двумя аргументами» бессмысленно — это прошло
/// бы и после того, как база уехала на локаль, а окно развалилось.
/// Проверяются два разных исхода на одном исходнике: у однорегистровых
/// окон `Строка(Х)` и `Строка(У)` база указывает ПРЯМО на локали, а у
/// двухрегистрового окна `Сообщить` она остаётся во временных регистрах.
#[test]
fn a_wide_window_stays_in_temporaries_while_narrow_ones_reach_the_locals() {
    let program = compile("Х = 1;\nУ = 2;\nСообщить(Строка(Х) + Строка(У));");
    let chunk = &program.chunks[0];
    let n_locals = chunk.n_locals;

    let windows: Vec<(u8, u8)> = chunk
        .instrs
        .iter()
        .filter_map(|i| match i {
            Instr::CallBuiltin { base, count, .. } => Some((*base, *count)),
            _ => None,
        })
        .collect();

    let narrow: Vec<u8> = windows
        .iter()
        .filter(|(_, c)| *c == 1)
        .map(|(b, _)| *b)
        .collect();
    assert_eq!(
        narrow.len(),
        2,
        "ожидались два вызова `Строка`: {windows:?}"
    );
    for base in narrow {
        assert!(
            base < n_locals,
            "однорегистровое окно обязано читать локаль напрямую: база {base}, локалей {n_locals}"
        );
    }

    let wide: Vec<(u8, u8)> = windows.into_iter().filter(|(_, c)| *c >= 2).collect();
    assert_eq!(
        wide.len(),
        1,
        "ожидался один вызов с широким окном: {wide:?}"
    );
    assert!(
        wide[0].0 >= n_locals,
        "широкое окно обязано остаться во временных регистрах: база {}, локалей {n_locals}",
        wide[0].0
    );
}
