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
    let (engine, module) = match compile(source) {
        Ok(pair) => pair,
        Err(code) => return code,
    };
    let text = match engine.image_bytecode(&module, Some(source)) {
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

/// Читает напечатанный байт-код и исполняет его — тем же путём через
/// фасад (`Engine`/`State`), что и обычный запуск: разобранная программа
/// ничем не отличается от только что скомпилированной.
pub fn run(path: &str, arguments: Vec<String>) -> i32 {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("не удалось прочитать «{path}»: {e}");
            return 1;
        }
    };
    // Образ может быть и одиночной программой, и конфигурацией с entry:
    // `--run-bytecode` требует entry, каталог без него — ошибка команды.
    let image = match bsl_bytecode::parse_image(&text) {
        Ok(image) => image,
        Err(e) => {
            eprintln!("{}: {e}", Path::new(path).display());
            return 1;
        }
    };
    let (engine, module) = match image {
        bsl_bytecode::BytecodeImage::Program(_) => {
            let engine = match crate::engine() {
                Ok(engine) => engine,
                Err(e) => {
                    eprintln!("ошибка сборки движка: {e}");
                    return 1;
                }
            };
            match engine.load_bytecode(&text) {
                Ok(m) => (engine, m),
                Err(e) => {
                    eprintln!("{}: {e}", Path::new(path).display());
                    return 1;
                }
            }
        }
        bsl_bytecode::BytecodeImage::Configuration { catalog, entry } => {
            let Some(entry) = entry else {
                eprintln!(
                    "{}: конфигурационный образ без entry исполнить нечем",
                    Path::new(path).display()
                );
                return 1;
            };
            // Расширение CLI: как и прогон исходника с `//@используй`,
            // тела модулей выполняются до entry.
            let engine = match open_bsl::Engine::builder()
                .configuration_image(catalog, true)
                .build()
            {
                Ok(engine) => engine,
                Err(e) => {
                    eprintln!("ошибка сборки конфигурации: {e}");
                    return 1;
                }
            };
            match engine.load_entry(entry) {
                Ok(m) => (engine, m),
                Err(e) => {
                    eprintln!("{}: {e}", Path::new(path).display());
                    return 1;
                }
            }
        }
    };
    let mut state = engine
        .state_builder()
        .arguments(arguments)
        .message_sink(std::rc::Rc::new(crate::StdoutMessageSink))
        .build();
    match state.run(&module) {
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

/// Тот же путь компиляции, что и у обычного запуска файла, — до VM:
/// шапка с `//@используй` включает конфигурационный путь.
fn compile(path: &str) -> Result<(open_bsl::Engine, open_bsl::Module), i32> {
    let src = std::fs::read_to_string(path).map_err(|e| {
        eprintln!("не удалось прочитать «{path}»: {e}");
        1
    })?;
    let directives = crate::usemod::parse_directives(&src).map_err(|e| {
        eprintln!("{path}: {e}");
        1
    })?;
    let engine = if directives.is_empty() {
        crate::engine().map_err(|e| {
            eprintln!("ошибка сборки движка: {e}");
            1
        })?
    } else {
        let recipe = crate::usemod::load_graph(Path::new(path), &src).map_err(|e| {
            eprintln!("{e}");
            1
        })?;
        open_bsl::Engine::builder()
            .configuration(recipe)
            .build()
            .map_err(|e| {
                eprintln!("ошибка сборки конфигурации: {e}");
                1
            })?
    };
    let module = engine.compile_entry(&src).map_err(|e| {
        match e {
            open_bsl::Error::Parse(e) => eprintln!("ошибка разбора: {e}"),
            open_bsl::Error::Semantic(e) => eprintln!("ошибка резолвинга: {e}"),
            other => eprintln!("ошибка компиляции: {other}"),
        }
        1
    })?;
    Ok((engine, module))
}
