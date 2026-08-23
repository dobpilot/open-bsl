//! REPL: чтение строки с подсветкой и дополнением, исполнение, печать.
//!
//! Редактор строки — `rustyline` (сырой режим терминала, стрелки, история,
//! Ctrl+R); BSL-специфика вся наша: `highlight` красит, `complete` решает,
//! что предлагать. Склейка с редактором — `BslHelper` ниже, и она нарочно
//! тонкая: в ней нет ни одного решения о языке, только вызовы наших чистых
//! функций.
//!
//! Если терминал не удаётся перевести в сырой режим (не tty, экзотическая
//! среда), REPL честно откатывается на построчное чтение stdin — так же,
//! как работал до появления редактора. Отсутствие подсветки не повод
//! остаться совсем без REPL.

use std::borrow::Cow;
use std::io::{self, Write};

use bsl_rt::BslValue;
use rustyline::completion::{Completer, Pair};
use rustyline::config::{ColorMode, CompletionType, Config};
use rustyline::error::ReadlineError;
use rustyline::highlight::{CmdKind, Highlighter};
use rustyline::hint::Hinter;
use rustyline::validate::Validator;
use rustyline::{Context, Editor, Helper};

use crate::complete::{self, SessionNames};
use crate::highlight;
use crate::print_value;

/// Состояние REPL-сессии: растёт со строки на строку, в отличие от
/// `Выполнить`/`Вычислить` внутри уже скомпилированного скрипта (которые
/// ограничены статически размеченным окружающим кадром — см. `bsl-vm`).
/// Формы структур сюда сознательно не входят: каждая строка получает свои
/// свежие (см. doc comment на `compile_snippet`) — межстрочного разделения
/// объектов это не портит (у каждого объекта своя `Rc<Shape>` независимо
/// от того, какая `ShapeTable` её породила).
pub struct Session {
    engine: open_bsl::Engine,
    locals: Vec<String>,
    names: Vec<String>,
    values: Vec<BslValue>,
    /// Окружение сессии: часы, случайность и (пустые) аргументы запуска.
    /// Одно на всю сессию, как и таблица имён, — иначе `ТекущаяДата` в
    /// соседних строках отвечала бы из разных окружений.
    env: bsl_rt::HostEnv,
    /// Компилятор `Выполнить`/`Вычислить`: строка REPL вправе содержать
    /// динамический код, а VM его только исполняет. Один на сессию — по
    /// той же причине, что и окружение.
    dynamic: open_bsl::DynamicCode,
}

impl Session {
    fn new() -> Self {
        let engine = crate::engine().unwrap_or_else(|e| {
            eprintln!("ошибка сборки движка: {e}");
            std::process::exit(1);
        });
        Session {
            dynamic: engine.dynamic_code(),
            engine,
            locals: Vec::new(),
            names: Vec::new(),
            values: Vec::new(),
            env: bsl_rt::HostEnv::process(),
        }
    }

    /// Имена, которые дополнение предлагает сверх встроенных: переменные
    /// этой сессии, всё, что осело в интернере (колонки таблиц, поля
    /// структур — они приходят туда при первом же обращении), и имена
    /// реестра компонентов движка — конструкторы и глобальные функции.
    fn names_for_completion(&self) -> SessionNames {
        let mut constructors = Vec::new();
        let mut functions = Vec::new();
        for library in self.engine.registry().libraries() {
            for constructor in library.constructors {
                constructors.extend(constructor.names.iter().map(|n| n.to_string()));
            }
            for function in library.functions {
                functions.extend(function.names.iter().map(|n| n.to_string()));
            }
        }
        SessionNames {
            locals: self.locals.clone(),
            fields: self.names.clone(),
            constructors,
            functions,
        }
    }
}

/// Цвет включён? Уважаем `NO_COLOR` (общее соглашение) и `TERM=dumb`.
/// Проверка идёт до создания редактора: тот же ответ уходит и в его
/// `ColorMode`, и в нашу подсветку, чтобы они не разошлись.
fn colors_enabled() -> bool {
    if std::env::var_os("NO_COLOR").is_some() {
        return false;
    }
    !matches!(std::env::var("TERM").as_deref(), Ok("dumb"))
}

/// Склейка BSL с редактором строки. Все решения о языке — в
/// `crate::highlight` и `crate::complete`; здесь только их вызовы и
/// хранение имён текущей сессии.
struct BslHelper {
    session: SessionNames,
    /// Имена глобальных функций реестра в верхнем регистре — подсветке,
    /// в отличие от дополнения, нужен регистронезависимый поиск на каждый
    /// набранный символ.
    component_fns: std::collections::HashSet<String>,
    colors: bool,
}

impl BslHelper {
    fn new(colors: bool) -> Self {
        BslHelper {
            session: SessionNames::default(),
            component_fns: std::collections::HashSet::new(),
            colors,
        }
    }
}

impl Completer for BslHelper {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Pair>)> {
        let (start, names) = complete::candidates(line, pos, &self.session);
        let pairs = names
            .into_iter()
            .map(|name| Pair {
                display: name.clone(),
                replacement: name,
            })
            .collect();
        Ok((start, pairs))
    }
}

impl Highlighter for BslHelper {
    fn highlight<'l>(&self, line: &'l str, _pos: usize) -> Cow<'l, str> {
        if !self.colors || line.is_empty() {
            return Cow::Borrowed(line);
        }
        Cow::Owned(highlight::colorize(line, &self.component_fns))
    }

    fn highlight_prompt<'b, 's: 'b, 'p: 'b>(
        &'s self,
        prompt: &'p str,
        _default: bool,
    ) -> Cow<'b, str> {
        if !self.colors {
            return Cow::Borrowed(prompt);
        }
        Cow::Owned(format!("\x1b[34m{prompt}\x1b[0m"))
    }

    /// Перекрашивать на каждое изменение строки. Дешевле, чем кажется:
    /// строка ввода короткая, а разбор в `highlight::pieces` — один проход
    /// по символам без аллокаций сверх результата.
    fn highlight_char(&self, _line: &str, _pos: usize, _kind: CmdKind) -> bool {
        self.colors
    }
}

/// Подсказок «серым по ходу набора» нет: единственное, чем их можно было бы
/// наполнить, — история, а она и так доступна стрелкой вверх.
impl Hinter for BslHelper {
    type Hint = String;
}

/// Ввод всегда однострочный: многострочные конструкции (`Если ... КонецЕсли`)
/// в REPL пишутся в одну строку, как и во фрагментах `Выполнить`.
impl Validator for BslHelper {}

impl Helper for BslHelper {}

pub fn run() {
    let colors = colors_enabled();
    let config = Config::builder()
        // Список вариантов по Tab, а не циклический перебор: имён много,
        // и увидеть их сразу полезнее, чем перебирать по одному.
        .completion_type(CompletionType::List)
        .color_mode(if colors {
            ColorMode::Enabled
        } else {
            ColorMode::Disabled
        })
        .build();

    let mut editor: Editor<BslHelper, _> = match Editor::with_config(config) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("редактор строки недоступен ({e}), читаю stdin построчно");
            return run_plain();
        }
    };
    editor.set_helper(Some(BslHelper::new(colors)));

    println!("BSL REPL. Tab — дополнение, стрелка вверх — история, Ctrl+D — выход.");
    let mut session = Session::new();
    // Имена реестра известны до первой строки — дополнение и подсветка
    // компонентов не должны ждать первого исполнения.
    if let Some(helper) = editor.helper_mut() {
        helper.session = session.names_for_completion();
        helper.component_fns = helper
            .session
            .functions
            .iter()
            .map(|n| n.to_uppercase())
            .collect();
    }

    loop {
        match editor.readline("bsl> ") {
            Ok(line) => {
                if line.trim().is_empty() {
                    continue;
                }
                // История пополняется ДО исполнения: строку, уронившую
                // интерпретатор, чаще всего и хочется поднять стрелкой,
                // чтобы поправить.
                let _ = editor.add_history_entry(line.as_str());
                eval_and_print(line.trim(), &mut session);
                if let Some(helper) = editor.helper_mut() {
                    helper.session = session.names_for_completion();
                    helper.component_fns = helper
                        .session
                        .functions
                        .iter()
                        .map(|n| n.to_uppercase())
                        .collect();
                }
            }
            // Ctrl+C — бросить набранную строку, но не сессию.
            Err(ReadlineError::Interrupted) => continue,
            Err(ReadlineError::Eof) => break,
            Err(e) => {
                eprintln!("ошибка чтения ввода: {e}");
                break;
            }
        }
    }
}

/// Запасной REPL без редактора строки — ровно тот, что был до rustyline.
fn run_plain() {
    println!("BSL REPL. Пустая строка — повтор приглашения, Ctrl+D — выход.");
    let mut session = Session::new();
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
        eval_and_print(line, &mut session);
    }
}

fn eval_and_print(line: &str, session: &mut Session) {
    match eval_repl_line(line, session) {
        Ok(BslValue::Undefined) => {}
        Ok(v) => print_value(&v),
        Err(msg) => eprintln!("Ошибка: {msg}"),
    }
}

/// Разбирает+резолвит+компилирует+исполняет одну строку REPL, ЗАСЕВАЯ
/// резолвер и интернер именами, накопленными за сессию, и обновляя
/// `session` результатами — новые переменные и значения сохраняются для
/// следующей строки (в отличие от изолированного `Выполнить`).
fn eval_repl_line(line: &str, session: &mut Session) -> Result<BslValue, String> {
    let parsed = bsl_syntax::parse(line).map_err(|e| format!("{e}"))?;
    let mut stmts = Vec::with_capacity(parsed.items.len());
    for item in parsed.items {
        match item {
            bsl_syntax::Item::Stmt(s) => stmts.push(s),
            bsl_syntax::Item::VarDecl(vd) => stmts.push(bsl_syntax::Stmt::VarDecl(vd)),
            _ => return Err("объявление процедур/функций в REPL пока не поддержано".to_string()),
        }
    }

    let (new_locals, body, requirements) = bsl_sema::resolve_repl_stmts_with_registry(
        &session.locals,
        &[],
        &stmts,
        &[],
        session.engine.registry(),
    )
    .map_err(|e| format!("{e}"))?;
    // Формы — ВСЕГДА свежие для этой строки (см. doc comment на
    // `compile_snippet`): shape-индексы внутри `chunk` ссылаются на них, а
    // не на что-то накопленное в сессии. Раньше здесь передавался
    // `session.shapes` (всегда пустой) — падало на первой же строке,
    // создающей `Новый Структура(...)`, с индексом за границами.
    let (chunk, new_names, shapes) = bsl_bytecode::compile_snippet_with_requirements(
        &new_locals,
        &body,
        &session.names,
        &[],
        &requirements,
    )
    .map_err(|e| format!("{e}"))?;

    let mut stack = session.values.clone();
    stack.resize(chunk.n_regs as usize, BslValue::Undefined);

    // В VM уходит именно `new_names`, а НЕ `session.names`: чанк
    // скомпилирован против расширенной таблицы, и `NameId` для поля, впервые
    // упомянутого этой же строкой (`т.Колонки`), в старую таблицу не
    // попадает. Раньше сюда передавалась старая — и первая же строка с
    // новым именем поля падала с `InvalidBytecode`.
    let (value, mut final_stack) = bsl_vm::run_repl_chunk_with_registry(
        &chunk,
        new_names.clone(),
        shapes,
        new_locals.clone(),
        stack,
        requirements.clone(),
        session.engine.registry(),
        &mut session.dynamic,
        &mut session.env,
    )
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Дополнение обязано видеть переменные, объявленные в этой же сессии,
    /// — иначе от него в REPL толку мало. Проверяем через настоящий прогон
    /// строки, а не подсовыванием имён руками.
    #[test]
    fn session_variables_become_completion_candidates() {
        let mut session = Session::new();
        eval_repl_line("таблицаИтогов = 1;", &mut session).unwrap();
        let names = session.names_for_completion();
        assert!(names.locals.contains(&"таблицаИтогов".to_string()));

        let (_, found) = complete::candidates("таблиц", "таблиц".len(), &names);
        assert!(found.contains(&"таблицаИтогов".to_string()), "{found:?}");
    }

    /// Имена полей попадают в интернер при первом обращении, и после этого
    /// дополнение после точки предлагает их наравне с методами.
    #[test]
    fn structure_fields_become_member_candidates() {
        let mut session = Session::new();
        eval_repl_line("с = Новый Структура(\"колонкаЦены\", 1);", &mut session).unwrap();
        let names = session.names_for_completion();
        let (_, found) = complete::candidates("с.колон", "с.колон".len(), &names);
        assert!(found.contains(&"колонкаЦены".to_string()), "{found:?}");
    }

    /// Регрессия: имя поля, впервые встреченное ЭТОЙ строкой, попадает в
    /// таблицу имён чанка — и в VM обязана уехать та же таблица, иначе
    /// `NameId` указывает за её конец. Ловилось на самой обычной строке
    /// REPL (`т.Колонки.Добавить(...)`) и выглядело как `InvalidBytecode`.
    #[test]
    fn a_field_name_introduced_by_this_very_line_resolves() {
        let mut session = Session::new();
        eval_repl_line("т = Новый ТаблицаЗначений;", &mut session).unwrap();
        eval_repl_line("т.Колонки.Добавить(\"цена\");", &mut session).unwrap();
        eval_repl_line("с = т.Добавить();", &mut session).unwrap();
        eval_repl_line("с.цена = 42;", &mut session).unwrap();
        let v = eval_repl_line("Возврат т.Итог(\"цена\");", &mut session).unwrap();
        assert_eq!(bsl_format::format_value(&v, None).unwrap(), "42");
    }

    #[test]
    fn a_failing_line_does_not_poison_the_session() {
        let mut session = Session::new();
        eval_repl_line("х = 2;", &mut session).unwrap();
        assert!(eval_repl_line("х = х / 0;", &mut session).is_err());
        // Значение осталось прежним, сессия жива.
        let v = eval_repl_line("Возврат х;", &mut session).unwrap();
        assert_eq!(bsl_format::format_value(&v, None).unwrap(), "2");
    }
}
