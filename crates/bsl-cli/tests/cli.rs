//! Проверки самого интерфейса командной строки: подсказка, неизвестная
//! команда, нехватка аргумента, обычный запуск скрипта.
//!
//! Подпроцессом, а не вызовом функций: разбор аргументов и коды возврата
//! — ровно то, что видит человек в терминале, и проверять их изнутри
//! процесса бессмысленно (`std::process::exit` не вернётся).

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn bsl_cli() -> &'static str {
    env!("CARGO_BIN_EXE_bsl-cli")
}

fn run(args: &[&str]) -> Output {
    Command::new(bsl_cli())
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("не удалось запустить bsl-cli {args:?}: {e}"))
}

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/conformance/fixtures")
        .join(name)
}

#[test]
fn help_is_printed_to_stdout_with_a_zero_exit_code() {
    for flag in ["--help", "-h"] {
        let out = run(&[flag]);
        assert!(out.status.success(), "{flag}: код возврата {}", out.status);
        let text = String::from_utf8_lossy(&out.stdout);
        assert!(text.contains("ИСПОЛЬЗОВАНИЕ"), "{flag}: {text}");
        // Каждая команда названа — список собирается из той же таблицы,
        // по которой идёт разбор (см. `COMMANDS` в main.rs).
        for expected in [
            "--emit-bytecode",
            "--run-bytecode",
            "--ingest-measurements",
            "--help",
        ] {
            assert!(
                text.contains(expected),
                "{flag}: в подсказке нет {expected}"
            );
        }
        // Подсказка — это не ошибка: stderr пуст.
        assert!(out.stderr.is_empty(), "{flag}: {:?}", out.stderr);
    }
}

#[test]
fn an_unknown_command_is_an_error_pointing_at_help() {
    let out = run(&["--такой-команды-нет"]);
    assert!(!out.status.success(), "неизвестная команда прошла молча");
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("--такой-команды-нет"), "{err}");
    assert!(err.contains("--help"), "{err}");
    // Ошибка идёт в stderr, чтобы `bsl-cli ... | что-то` не получил её на вход.
    assert!(out.stdout.is_empty());
}

#[test]
fn a_command_without_its_argument_shows_that_commands_usage() {
    for (flag, expected) in [
        ("--emit-bytecode", "<файл.bsl>"),
        ("--run-bytecode", "<файл.bslc>"),
        ("--ingest-measurements", "<вывод-платформы>"),
    ] {
        let out = run(&[flag]);
        assert!(!out.status.success(), "{flag}: прошло без аргумента");
        let err = String::from_utf8_lossy(&out.stderr);
        assert!(err.contains(flag), "{flag}: {err}");
        assert!(err.contains(expected), "{flag}: нет формы вызова: {err}");
    }
}

/// Регрессия на разбор аргументов: путь к скрипту не должен спутаться с
/// командой, а команда — с путём.
#[test]
fn a_script_path_still_runs_the_script() {
    let path = fixture("arithmetic.bsl");
    let out = run(&[path.to_str().unwrap()]);
    assert!(
        out.status.success(),
        "скрипт не исполнился: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let expected = std::fs::read_to_string(fixture("arithmetic.expected")).unwrap();
    let actual = String::from_utf8_lossy(&out.stdout);
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n')
    );
}
