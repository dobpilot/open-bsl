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

#[test]
fn script_arguments_land_in_the_command_line_arguments_array() {
    let path = std::env::temp_dir().join("bsl-cli-test-args.bsl");
    std::fs::write(
        &path,
        "Сообщить(АргументыКоманднойСтроки.Количество());\n\
         Для Каждого Арг Из АргументыКоманднойСтроки Цикл\n\
         \tСообщить(Арг);\n\
         КонецЦикла;\n\
         Сообщить(CommandLineArguments.Количество());\n",
    )
    .unwrap();
    let script = path.to_str().unwrap();

    // Аргумент с пробелом обязан дойти одной строкой — за это отвечает
    // разбиение args[2..], а не повторный разбор по пробелам.
    let out = run(&[script, "раз", "два три"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&out.stdout), "2\nраз\nдва три\n2\n");

    // Без аргументов — пустой массив, а не ошибка.
    let out = run(&[script]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&out.stdout), "0\n0\n");

    let _ = std::fs::remove_file(&path);
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
