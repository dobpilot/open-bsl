//! Единая точка сборки образа и отказ устаревшей разметке бандлов.
//!
//! Разметка бандлов — производная таблица: VM исполняет бандл целиком за
//! одну диспетчеризацию, доверяя доказанной независимости членов. Значит
//! таблица, посчитанная ДО правки инструкций, описывает уже не тот чанк и
//! даёт не отказ, а молча неверный ответ. Проверка образа обязана такой
//! образ отвергать, а сборщики — не считать таблицу каждый сам.

mod support;

use bsl_bytecode::{Instr, Program, analysis, bundle, image};
use support::{chunk, program};

/// Образ, который `verify` проходит целиком.
fn valid_program() -> Program {
    let mut p = program(vec![
        chunk(vec![
            Instr::LoadConst { dst: 0, k: 0 },
            Instr::LoadConst { dst: 1, k: 0 },
            Instr::Return { src: None },
        ]),
        chunk(vec![Instr::Return { src: None }]),
    ]);
    p.chunks[0].consts = vec![support::konst(bsl_rt::BslValue::number_from_i64(1))];
    p.chunks[0].n_regs = 2;
    p.module_vars = vec!["Общая".to_string()];
    p.exported_module_vars = vec![true];
    p.function_names = vec!["Ф".to_string()];
    p.exported_functions = vec![true];
    p
}

/// Разметка от прежней редакции инструкций отвергается.
///
/// Проверки, которым нужно ИСПОРТИТЬ производную таблицу, живут теперь
/// внутри крейта (`image::tests`): снаружи такой записи больше нет.
/// Здесь остаётся то, что портит образ законными средствами — правкой
/// инструкций после финализации.
#[test]
fn a_stale_bundle_table_is_rejected() {
    let mut p = valid_program();
    image::finalize(&mut p);
    p.chunks[1].instrs = vec![
        Instr::LoadConst { dst: 0, k: 0 },
        Instr::LoadConst { dst: 1, k: 0 },
        Instr::Return { src: None },
    ];
    p.chunks[1].consts = vec![support::konst(bsl_rt::BslValue::number_from_i64(1))];
    p.chunks[1].n_regs = 2;
    assert!(
        image::verify(&p).is_err(),
        "устаревшая разметка прошла проверку"
    );
}

/// Финализация вычисляет пересечение модульных слотов ИЗНУТРИ образа: у
/// вызывающего не остаётся способа передать чужое.
#[test]
fn finalize_computes_the_overlap_from_the_image_itself() {
    let mut p = valid_program();
    assert!(p.chunks.iter().all(|c| c.bundle_len().is_empty()));

    image::finalize(&mut p);

    for (i, c) in p.chunks.iter().enumerate() {
        assert_eq!(
            c.bundle_len(),
            bundle::compute(c, analysis::module_overlap(i, p.module_vars.len())),
            "чанк {i}: финализация посчитала разметку иначе"
        );
    }
}

/// Одиночный чанк финализируется своей операцией — той, что не знает о
/// пересечении модульных слотов и знать не должна.
#[test]
fn a_lone_chunk_has_its_own_finalization() {
    let mut c = chunk(vec![
        Instr::LoadConst { dst: 0, k: 0 },
        Instr::LoadConst { dst: 1, k: 0 },
        Instr::Return { src: None },
    ]);
    c.consts = vec![support::konst(bsl_rt::BslValue::number_from_i64(1))];
    c.n_regs = 2;
    assert!(c.bundle_len().is_empty());

    image::finalize_lone_chunk(&mut c);

    assert_eq!(c.bundle_len(), bundle::compute(&c, None));
    assert_eq!(c.prop_cache().len(), c.instrs.len());
    assert_eq!(c.method_cache().len(), c.instrs.len());
}
