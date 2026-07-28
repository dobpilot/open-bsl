//! `bsl-cli <файл.bsl>` — компилирует и исполняет файл целиком.
//! `bsl-cli` без аргументов — REPL с подсветкой и дополнением по Tab (см.
//! модуль `repl`): каждая строка резолвится поверх уже накопленных за
//! сессию переменных/имён полей и исполняется как отдельный чанк
//! (`bsl_vm::run_repl_chunk`), значение (если `Возврат` был) печатается.
//! `bsl-cli --ingest-measurements <файл> [measure-all.bsl]` — приём вывода
//! сеанса замеров у платформы, см. модуль `ingest`.

mod complete;
mod highlight;
mod ingest;
mod repl;

use bsl_rt::BslValue;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("--ingest-measurements") => {
            let Some(input) = args.get(2) else {
                eprintln!(
                    "--ingest-measurements ждёт файл с выводом платформы:\n  \
                     bsl-cli --ingest-measurements platform-output.txt [measure-all.bsl]"
                );
                std::process::exit(1);
            };
            let code = ingest::run(input, args.get(3).map(String::as_str));
            std::process::exit(code);
        }
        Some(path) => run_file(path),
        None => repl::run(),
    }
}

fn run_file(path: &str) {
    let src = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("не удалось прочитать «{path}»: {e}");
            std::process::exit(1);
        }
    };

    let program = match bsl_syntax::parse(&src) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("ошибка разбора: {e:?}");
            std::process::exit(1);
        }
    };
    let resolved = match bsl_sema::resolve_program(&program.items) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("ошибка резолвинга: {e:?}");
            std::process::exit(1);
        }
    };
    let compiled = match bsl_bytecode::compile_program(&resolved) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("ошибка компиляции: {e:?}");
            std::process::exit(1);
        }
    };
    match bsl_vm::run_program(&compiled) {
        Ok(BslValue::Undefined) => {}
        Ok(v) => print_value(&v),
        Err(e) => {
            eprintln!("ошибка выполнения: {e}");
            std::process::exit(1);
        }
    }
}

/// Печать значения, которым завершился скрипт или строка REPL.
///
/// `format_value` возвращает `Result` из-за одного-единственного случая —
/// незнакомой локали в ключе `Л`; здесь форматная строка не передаётся
/// вовсе, поэтому ветка ошибки недостижима. Молча её глотать всё равно
/// нельзя: если недостижимое случится, пусть будет видно, а не пустая
/// строка на месте результата.
pub fn print_value(v: &BslValue) {
    match bsl_format::format_value(v, None) {
        Ok(s) => println!("{s}"),
        Err(e) => eprintln!("ошибка форматирования результата: {e}"),
    }
}
