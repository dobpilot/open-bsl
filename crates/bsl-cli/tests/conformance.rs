//! Раннер для tests/conformance/fixtures/ (задача 2 ревью): для каждого
//! `*.bsl` рядом с `*.expected` гоняет `bsl-cli <файл>` как отдельный
//! процесс и построчно сравнивает его stdout с оракулом.
//!
//! Подпроцесс, а не вызов пайплайна `bsl_syntax::parse -> ... ->
//! bsl_vm::run_program` в этом же процессе, потому что `Сообщить`/`Message`
//! пишет напрямую в stdout через `println!` (см. `bsl-vm/src/lib.rs`) — в
//! процессе теста это некуда перехватить без OS-уровневой подмены fd 1.
//! `CARGO_BIN_EXE_bsl-cli` cargo выставляет сам для интеграционных тестов
//! пакета, который собирает этот бинарник.
//!
//! Фикстура без пары `.expected` молча пропускается — это не ошибка, а
//! способ держать в репозитории заготовки (`n-body-precision.bsl` и
//! `n-body-smoke.bsl` сейчас), для которых пока нет платформенного оракула.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/conformance/fixtures")
}

/// Построчный дифф, показывающий первую точку расхождения (плюс контекст),
/// а не всё содержимое обоих файлов сразу.
fn line_diff(expected: &str, actual: &str) -> String {
    let exp_lines: Vec<&str> = expected.lines().collect();
    let act_lines: Vec<&str> = actual.lines().collect();
    let mut out = String::new();
    let max = exp_lines.len().max(act_lines.len());
    for i in 0..max {
        let e = exp_lines.get(i).copied();
        let a = act_lines.get(i).copied();
        if e != a {
            out.push_str(&format!(
                "  строка {}: ожидалось {:?}, получено {:?}\n",
                i + 1,
                e.unwrap_or("<конец файла>"),
                a.unwrap_or("<конец файла>"),
            ));
        }
    }
    if exp_lines.len() != act_lines.len() {
        out.push_str(&format!(
            "  разное число строк: ожидалось {}, получено {}\n",
            exp_lines.len(),
            act_lines.len()
        ));
    }
    out
}

#[test]
fn conformance_fixtures_match_oracle_output() {
    let dir = fixtures_dir();
    let bsl_cli = env!("CARGO_BIN_EXE_bsl-cli");

    let mut checked = Vec::new();
    let mut skipped = Vec::new();
    let mut failures = Vec::new();

    let mut entries: Vec<PathBuf> = fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("не удалось прочитать {}: {e}", dir.display()))
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("bsl"))
        .collect();
    entries.sort();

    for fixture in entries {
        let expected_path = fixture.with_extension("expected");
        let name = fixture.file_name().unwrap().to_string_lossy().into_owned();
        if !expected_path.exists() {
            skipped.push(name);
            continue;
        }
        checked.push(name.clone());

        let expected = fs::read_to_string(&expected_path)
            .unwrap_or_else(|e| panic!("не удалось прочитать {}: {e}", expected_path.display()));

        let output = Command::new(bsl_cli)
            .arg(&fixture)
            .output()
            .unwrap_or_else(|e| panic!("не удалось запустить bsl-cli на {name}: {e}"));

        let actual = String::from_utf8_lossy(&output.stdout).into_owned();

        if actual.trim_end_matches('\n') != expected.trim_end_matches('\n') {
            let mut msg = format!("{name}: stdout разошёлся с {}\n", expected_path.display());
            msg.push_str(&line_diff(&expected, &actual));
            if !output.status.success() {
                msg.push_str(&format!(
                    "  процесс завершился с {}; stderr:\n{}\n",
                    output.status,
                    String::from_utf8_lossy(&output.stderr)
                ));
            }
            failures.push(msg);
        }
    }

    assert!(
        !checked.is_empty(),
        "ни одной фикстуры с парой .expected не найдено в {}",
        dir.display()
    );
    if !skipped.is_empty() {
        eprintln!("пропущено (нет .expected): {}", skipped.join(", "));
    }

    assert!(
        failures.is_empty(),
        "{} фикстур(а) разошлись с оракулом:\n\n{}",
        failures.len(),
        failures.join("\n")
    );
}
