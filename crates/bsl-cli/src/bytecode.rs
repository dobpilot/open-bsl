//! `--emit-bytecode` и `--run-bytecode`: посмотреть байт-код вместо
//! исполнения и исполнить посмотренное.
//!
//! Обе команды ходят через `bsl_bytecode::text` — один формат и на печать,
//! и на разбор, так что напечатанный файл не «отчёт о байт-коде», а сам
//! байт-код: его можно прочитать, поправить руками и запустить.
//!
//! Полезно ровно там, где иначе приходится гадать: что компилятор сделал с
//! коротким замыканием, куда встал `NumericForNextI64` вместо общего пути,
//! сколько регистров съел кадр, попала ли форма структуры в таблицу.

use std::path::Path;

/// Компилирует и печатает байт-код: в файл, если путь задан, иначе в
/// stdout. Возвращает код возврата процесса.
pub fn emit(source: &str, out: Option<&str>) -> i32 {
    let program = match compile(source) {
        Ok(p) => p,
        Err(code) => return code,
    };
    let text = match bsl_bytecode::write_program(&program, Some(source)) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("не удалось напечатать байт-код: {e}");
            return 1;
        }
    };
    match out {
        None => {
            print!("{text}");
            0
        }
        Some(path) => match std::fs::write(path, &text) {
            Ok(()) => {
                eprintln!("байт-код записан в {path}");
                0
            }
            Err(e) => {
                eprintln!("не удалось записать «{path}»: {e}");
                1
            }
        },
    }
}

/// Читает напечатанный байт-код и исполняет его — тем же
/// `bsl_vm::run_program`, что и обычный запуск: разобранная программа
/// ничем не отличается от только что скомпилированной.
pub fn run(path: &str) -> i32 {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("не удалось прочитать «{path}»: {e}");
            return 1;
        }
    };
    let program = match bsl_bytecode::parse_program(&text) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{}: {e}", Path::new(path).display());
            return 1;
        }
    };
    match bsl_vm::run_program(&program) {
        Ok(bsl_rt::BslValue::Undefined) => 0,
        Ok(v) => {
            crate::print_value(&v);
            0
        }
        Err(e) => {
            eprintln!("ошибка выполнения: {e}");
            1
        }
    }
}

/// Тот же путь компиляции, что и у обычного запуска файла, — до VM.
fn compile(path: &str) -> Result<bsl_bytecode::Program, i32> {
    let src = std::fs::read_to_string(path).map_err(|e| {
        eprintln!("не удалось прочитать «{path}»: {e}");
        1
    })?;
    let program = bsl_syntax::parse(&src).map_err(|e| {
        eprintln!("ошибка разбора: {e:?}");
        1
    })?;
    let resolved = bsl_sema::resolve_program(&program.items).map_err(|e| {
        eprintln!("ошибка резолвинга: {e:?}");
        1
    })?;
    bsl_bytecode::compile_program(&resolved).map_err(|e| {
        eprintln!("ошибка компиляции: {e:?}");
        1
    })
}
