//! `bsl-cli <файл.bsl>` — компилирует и исполняет файл целиком.
//! `bsl-cli` без аргументов — REPL: каждая строка резолвится поверх уже
//! накопленных за сессию переменных/имён полей и исполняется как
//! отдельный чанк (`bsl_vm::run_repl_chunk`), значение (если `Возврат`
//! был) печатается.
//! `bsl-cli --ingest-measurements <файл> [measure-all.bsl]` — приём вывода
//! сеанса замеров у платформы, см. модуль `ingest`.

mod ingest;

use std::io::{self, Write};

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
        None => repl(),
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
        Ok(v) => println!("{}", bsl_format::format_value(&v, None)),
        Err(e) => {
            eprintln!("ошибка выполнения: {e}");
            std::process::exit(1);
        }
    }
}

/// Состояние REPL-сессии: растёт со строки на строку, в отличие от
/// `Выполнить`/`Вычислить` внутри уже скомпилированного скрипта (которые
/// ограничены статически размеченным окружающим кадром — см. `bsl-vm`).
/// Формы структур сюда сознательно не входят: каждая строка получает свои
/// свежие (см. doc comment на `compile_snippet`) — межстрочного разделения
/// объектов это не портит (у каждого объекта своя `Rc<Shape>` независимо
/// от того, какая `ShapeTable` её породила).
struct Session {
    locals: Vec<String>,
    names: Vec<String>,
    values: Vec<BslValue>,
}

fn repl() {
    println!("BSL REPL. Пустая строка — повтор приглашения, Ctrl+D — выход.");
    let mut session = Session {
        locals: Vec::new(),
        names: Vec::new(),
        values: Vec::new(),
    };

    loop {
        print!("bsl> ");
        if io::stdout().flush().is_err() {
            break;
        }
        let mut line = String::new();
        match io::stdin().read_line(&mut line) {
            Ok(0) => {
                println!();
                break;
            }
            Ok(_) => {}
            Err(e) => {
                eprintln!("ошибка чтения ввода: {e}");
                break;
            }
        }
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        match eval_repl_line(line, &mut session) {
            Ok(BslValue::Undefined) => {}
            Ok(v) => println!("{}", bsl_format::format_value(&v, None)),
            Err(msg) => eprintln!("Ошибка: {msg}"),
        }
    }
}

/// Разбирает+резолвит+компилирует+исполняет одну строку REPL, ЗАСЕВАЯ
/// резолвер и интернер именами, накопленными за сессию, и обновляя
/// `session` результатами — новые переменные/значения персистят для
/// следующей строки (в отличие от изолированного `Выполнить`).
fn eval_repl_line(line: &str, session: &mut Session) -> Result<BslValue, String> {
    let parsed = bsl_syntax::parse(line).map_err(|e| format!("{e:?}"))?;
    let mut stmts = Vec::with_capacity(parsed.items.len());
    for item in parsed.items {
        match item {
            bsl_syntax::Item::Stmt(s) => stmts.push(s),
            bsl_syntax::Item::VarDecl(vd) => stmts.push(bsl_syntax::Stmt::VarDecl(vd)),
            _ => {
                return Err(
                    "объявление процедур/функций в REPL пока не поддержано".to_string(),
                )
            }
        }
    }

    let (new_locals, body) =
        bsl_sema::resolve_snippet_stmts(&session.locals, &stmts).map_err(|e| format!("{e:?}"))?;
    // Формы — ВСЕГДА свежие для этой строки (см. doc comment на
    // `compile_snippet`): shape-индексы внутри `chunk` ссылаются на них, а
    // не на что-то накопленное в сессии. Раньше здесь передавался
    // `session.shapes` (всегда пустой) — падало на первой же строке,
    // создающей `Новый Структура(...)`, с индексом за границами.
    let (chunk, new_names, shapes) = bsl_bytecode::compile_snippet(&new_locals, &body, &session.names)
        .map_err(|e| format!("{e:?}"))?;

    let mut stack = session.values.clone();
    stack.resize(chunk.n_regs as usize, BslValue::Undefined);

    let (value, mut final_stack) =
        bsl_vm::run_repl_chunk(&chunk, session.names.clone(), shapes, new_locals.clone(), stack)
            .map_err(|e| e.to_string())?;
    // `final_stack` включает временные регистры этой строки (`chunk.n_regs`
    // может быть больше числа локалей) — обрезаем до настоящих локалей,
    // иначе следующая строка увидит мусор от чужих temp-регистров на месте
    // ещё не проинициализированной новой переменной.
    final_stack.truncate(new_locals.len());

    session.locals = new_locals;
    session.names = new_names;
    session.values = final_stack;
    Ok(value)
}
