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
///
/// Образец `every_section` для этого не годится: он собран для печати и
/// кругового разбора и `verify` не проходит по посторонней причине —
/// поэтому проверка «отвергнуто» на нём была бы пустой, что здесь уже и
/// случилось однажды.
fn valid_program() -> Program {
    let mut p = program(vec![
        chunk(vec![
            Instr::LoadConst { dst: 0, k: 0 },
            Instr::LoadConst { dst: 1, k: 0 },
            Instr::Return { src: None },
        ]),
        chunk(vec![Instr::Return { src: None }]),
    ]);
    p.chunks[0].consts = vec![bsl_rt::BslValue::number_from_i64(1)];
    p.chunks[0].n_regs = 2;
    p.module_vars = vec!["Общая".to_string()];
    p.exported_module_vars = vec![true];
    p.function_names = vec!["Ф".to_string()];
    p.exported_functions = vec![true];
    p
}

/// Прежде всего: образец обязан проходить проверку. Без этого любой тест
/// «отвергнуто» ниже доказывал бы не то.
#[test]
fn the_sample_image_is_valid_to_begin_with() {
    let mut p = valid_program();
    image::finalize(&mut p);
    image::verify(&p).expect("повторная проверка согласованного образа");
}

/// Разметка от прежней редакции инструкций отвергается.
#[test]
fn a_stale_bundle_table_is_rejected() {
    let mut p = valid_program();
    image::finalize(&mut p);
    // Инструкции чанка 1 меняются ПОСЛЕ того, как разметка посчитана.
    p.chunks[1].instrs = vec![
        Instr::LoadConst { dst: 0, k: 0 },
        Instr::LoadConst { dst: 1, k: 0 },
        Instr::Return { src: None },
    ];
    p.chunks[1].consts = vec![bsl_rt::BslValue::number_from_i64(1)];
    p.chunks[1].n_regs = 2;
    assert!(
        image::verify(&p).is_err(),
        "устаревшая разметка прошла проверку"
    );
}

/// Разметка, посчитанная с чужим пересечением модульных слотов, тоже
/// отвергается: пересечение меняет вывод об алиасинге, и бандл собрался бы
/// из зависимых инструкций.
#[test]
fn a_bundle_table_computed_with_the_wrong_overlap_is_rejected() {
    let mut p = valid_program();
    // Смысл пересечения — в том, что модульный слот чанка 0 И ЕСТЬ
    // регистр кадра с тем же номером. Поэтому запись в регистр 0 и чтение
    // модульного слота 0 зависимы с пересечением и независимы без него:
    // ровно на этом ответы `compute` и расходятся.
    p.chunks[0].instrs = vec![
        Instr::LoadConst { dst: 0, k: 0 },
        Instr::GetModuleVar { dst: 1, slot: 0 },
        Instr::Return { src: None },
    ];
    p.chunks[0].consts = vec![bsl_rt::BslValue::number_from_i64(1)];
    p.chunks[0].n_regs = 2;
    image::finalize(&mut p);

    let wrong = bundle::compute(&p.chunks[0], None);
    assert_ne!(
        wrong, p.chunks[0].bundle_len,
        "образец не различает пересечение модульных слотов — тест ничего не проверяет"
    );
    p.chunks[0].bundle_len = wrong;
    assert!(
        image::verify(&p).is_err(),
        "разметка с чужим пересечением модульных слотов прошла проверку"
    );
}

/// Пустая разметка — законный отказ от бандлов, а не устаревшая таблица:
/// VM исполняет такой чанк поинструкционно. Фрагмент `Вычислить` идёт
/// именно так, и проверка не смеет его отвергать.
#[test]
fn an_empty_bundle_table_is_a_legitimate_opt_out() {
    let mut p = valid_program();
    image::finalize(&mut p);
    for c in &mut p.chunks {
        c.bundle_len.clear();
    }
    image::verify(&p).expect("пустая разметка обязана оставаться законной");
}

/// Финализация вычисляет пересечение модульных слотов ИЗНУТРИ образа: у
/// вызывающего не остаётся способа передать чужое.
#[test]
fn finalize_computes_the_overlap_from_the_image_itself() {
    let mut p = valid_program();
    assert!(p.chunks.iter().all(|c| c.bundle_len.is_empty()));

    image::finalize(&mut p);

    for (i, c) in p.chunks.iter().enumerate() {
        assert_eq!(
            c.bundle_len,
            bundle::compute(c, analysis::module_overlap(i, p.module_vars.len())),
            "чанк {i}: финализация посчитала разметку иначе"
        );
    }
}
