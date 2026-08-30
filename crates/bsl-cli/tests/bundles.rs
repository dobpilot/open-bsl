//! Инварианты разметки VLIW-бандлов на всём конформанс-корпусе.
//!
//! Конформанс-прогон сравнивает только stdout, поэтому ошибочная разметка,
//! не меняющая вывод, прошла бы там зелёной. Здесь каждый скрипт корпуса
//! компилируется in-process, и разметка каждого чанка проверяется
//! независимым от построителя попарным пересчётом
//! (`bsl_bytecode::bundle::verify`): взаимная независимость членов
//! (RAW/WAW), выравнивание по целям переходов и границам `Попытка`,
//! передача управления только хвостом бандла.

use std::fs;
use std::path::{Path, PathBuf};

fn conformance_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/conformance")
}

/// Все `*.bsl` каталога, отсортированные по имени.
fn scripts_in(dir: &Path) -> Vec<PathBuf> {
    let mut entries: Vec<PathBuf> = fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("не удалось прочитать {}: {e}", dir.display()))
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("bsl"))
        .collect();
    entries.sort();
    entries
}

#[test]
fn bundle_invariants_hold_on_the_whole_corpus() {
    let mut chunks_total = 0usize;
    let mut instrs_total = 0usize;
    let mut bundles_total = 0usize;
    let mut multi = 0usize;
    let mut widest = 0usize;
    let mut skipped = 0usize;
    for dir in [
        conformance_dir().join("fixtures"),
        conformance_dir().join("measure"),
    ] {
        for script in scripts_in(&dir) {
            let src =
                fs::read_to_string(&script).unwrap_or_else(|e| panic!("{}: {e}", script.display()));
            // Скрипт, который этот интерпретатор пока не компилирует
            // (например, `measure-unsupported.bsl`), байт-кода не даёт —
            // проверять нечего, пропуск, как и в конформанс-прогоне.
            let Ok(parsed) = bsl_syntax::parse(&src) else {
                skipped += 1;
                continue;
            };
            let Ok(resolved) = bsl_sema::resolve_program(&parsed.items) else {
                skipped += 1;
                continue;
            };
            let Ok(program) = bsl_compiler::compile_program(&resolved) else {
                skipped += 1;
                continue;
            };
            for (i, chunk) in program.chunks.iter().enumerate() {
                let overlap = bsl_bytecode::analysis::module_overlap(i, program.module_vars.len());
                if let Err(e) = bsl_bytecode::bundle::verify(chunk, overlap) {
                    panic!("{}: чанк {i}: {e}", script.display());
                }
                chunks_total += 1;
                instrs_total += chunk.instrs.len();
                for &w in chunk.bundle_len() {
                    if w >= 1 {
                        bundles_total += 1;
                    }
                    if w >= 2 {
                        multi += 1;
                    }
                    widest = widest.max(w as usize);
                }
            }
        }
    }
    // Сводка видна под `--nocapture` — по ней видно, что разметка вообще
    // что-то группирует, а не выродилась в одни одиночки.
    println!(
        "бандлы: чанков {chunks_total}, инструкций {instrs_total}, \
         бандлов {bundles_total} (многочленных {multi}, максимум {widest}), \
         скриптов пропущено {skipped}"
    );
    assert!(
        multi > 0,
        "на всём корпусе не нашлось ни одного многочленного бандла — \
         либо анализ вырожден, либо разметка не заполняется"
    );
}

/// Инварианты анализа копий на том же корпусе.
///
/// Проверяет не сам ответ, а его устойчивость: живучесть, посчитанная
/// двумя порядками обхода блоков, обязана совпасть, разбиение на блоки —
/// покрывать чанк целиком, а устранимой не смеет оказаться копия
/// неточного регистра. Сводка под `--nocapture` показывает, что анализ
/// вообще что-то находит, а не выродился в сплошные отказы.
#[test]
fn copy_analysis_invariants_hold_on_the_whole_corpus() {
    let mut chunks = 0usize;
    let mut moves = 0usize;
    let mut removable = 0usize;
    let mut skipped = 0usize;
    for dir in [
        conformance_dir().join("fixtures"),
        conformance_dir().join("measure"),
    ] {
        for script in scripts_in(&dir) {
            let src =
                fs::read_to_string(&script).unwrap_or_else(|e| panic!("{}: {e}", script.display()));
            let Ok(parsed) = bsl_syntax::parse(&src) else {
                skipped += 1;
                continue;
            };
            let Ok(resolved) = bsl_sema::resolve_program(&parsed.items) else {
                skipped += 1;
                continue;
            };
            let Ok(program) = bsl_compiler::compile_program(&resolved) else {
                skipped += 1;
                continue;
            };
            for (i, chunk) in program.chunks.iter().enumerate() {
                let overlap = bsl_bytecode::analysis::module_overlap(i, program.module_vars.len());
                if let Err(e) = bsl_bytecode::analysis::verify(chunk, overlap) {
                    panic!("{}: чанк {i}: {e}", script.display());
                }
                chunks += 1;
                let flags = bsl_bytecode::analysis::removable_copies(chunk, overlap);
                for (pc, instr) in chunk.instrs.iter().enumerate() {
                    if matches!(instr, bsl_bytecode::Instr::Move { .. }) {
                        moves += 1;
                        if flags[pc] {
                            removable += 1;
                        }
                    }
                }
            }
        }
    }
    println!(
        "копии: чанков {chunks}, Move {moves}, из них устранимо {removable}, \
         скриптов пропущено {skipped}"
    );
    // Смысл утверждения зависит от того, применён ли проход. Без него
    // анализ обязан хоть что-то находить, иначе он выродился. С ним
    // корпус уже оптимизирован, и остаток обязан быть НУЛЕВЫМ: это
    // проверка того, что проход дошёл до неподвижной точки и не оставил
    // работы, которую сам же считает выполнимой.
    #[cfg(not(feature = "copyprop"))]
    assert!(
        removable > 0,
        "на всём корпусе не нашлось ни одной устранимой копии — \
         анализ вырожден"
    );
    #[cfg(feature = "copyprop")]
    assert_eq!(
        removable, 0,
        "проход оставил копии, которые сам считает устранимыми — \
         неподвижная точка не достигнута"
    );
}
