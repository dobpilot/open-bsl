//! Текстовое представление байт-кода: печать (`write_program`) и разбор
//! обратно (`parse_program`).
//!
//! Формат ОДИН на оба применения — то, что человек читает глазами, то же
//! самое и исполняется. Отдельный «красивый дизассемблер» плюс отдельный
//! бинарный контейнер дали бы два представления, из которых расходиться
//! начало бы уже второе; здесь расходиться нечему.
//!
//! Читаемость держится на комментариях: всё от `;` до конца строки
//! разбор игнорирует, а печать выносит туда то, что иначе пришлось бы
//! держать в голове — текст константы рядом с её номером, имя поля рядом с
//! `NameId`, имя функции рядом с номером чанка.
//!
//! Чего в формате НЕТ намеренно:
//!
//! * `prop_cache` — это рантайм-кэш инлайн-кэширования, он восстанавливается
//!   пустым по числу инструкций (сохранять кэш означало бы сохранять и
//!   формы, на которые он ссылается, ради нулевой выгоды);
//! * версий инструкций и обратной совместимости — заголовок несёт номер
//!   формата, и загрузчик отвергает чужой номер целиком. Байт-код здесь не
//!   контракт между версиями, а способ посмотреть и прогнать то, что
//!   скомпилировал этот же бинарник.
//!
//! Единственная гарантия, которую формат обязан держать и которую проверяет
//! `round_trip_*`: печать -> разбор -> печать даёт ПОБАЙТОВО ту же строку, а
//! исполнение разобранной программы — тот же результат, что и исходной.

use std::cell::RefCell;
use std::fmt::Write as _;

use bsl_number::BslNumber;
use bsl_rt::{BslDate, BslString, BslValue, NameId, Shape, ShapeTable};

use crate::chunk::{Chunk, ExceptionRange, Program};
use crate::instr::{ArgMode, Instr};

/// Номер формата. Меняется при любой правке синтаксиса — загрузчик
/// сверяет его и отказывается угадывать.
pub const FORMAT_VERSION: u32 = 1;

/// Имена опкодов — те же строки, что печатает `write_instr` и принимает
/// `parse_instr`. Список публичен, потому что на нём держится тест
/// покрытия: корпус round-trip обязан задеть каждый опкод, иначе
/// расхождение печати и разбора обнаружится не здесь, а у пользователя.
pub const OPCODES: &[&str] = &[
    "Move",
    "GetModuleVar",
    "SetModuleVar",
    "LoadConst",
    "LoadBool",
    "LoadUndefined",
    "LoadNull",
    "LoadSkipped",
    "Add",
    "Sub",
    "Mul",
    "Div",
    "Neg",
    "Not",
    "Eq",
    "NotEq",
    "Lt",
    "Gt",
    "Le",
    "Ge",
    "Jump",
    "JumpIfFalse",
    "JumpIfTrue",
    "JumpIfNotSkipped",
    "NumericForNext",
    "NumericForNextI64",
    "Call",
    "Return",
    "GetIndex",
    "SetIndex",
    "GetProp",
    "SetProp",
    "NewArray",
    "NewStructure",
    "NewTable",
    "NewMap",
    "NewTextWriter",
    "CollectionLen",
    "Raise",
    "CallBuiltin",
    "CallMethod",
    "WriteText",
    "CloseText",
    "RunDynamic",
];

#[derive(Debug, Clone, PartialEq)]
pub enum TextError {
    /// Строка с номером и тем, что в ней не так. Номер — 1-based, как в
    /// редакторе: файл байт-кода правят руками чаще, чем кажется.
    At(usize, String),
    /// Заголовок отсутствует или несёт чужой номер формата.
    BadHeader(String),
    /// Значение, которое не переживает печать (объект в константах).
    Unrepresentable(&'static str),
}

impl std::fmt::Display for TextError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TextError::At(line, what) => write!(f, "строка {line}: {what}"),
            TextError::BadHeader(what) => write!(f, "заголовок байт-кода: {what}"),
            TextError::Unrepresentable(what) => {
                write!(f, "значение не представимо в текстовом байт-коде: {what}")
            }
        }
    }
}

impl std::error::Error for TextError {}

type Result<T> = std::result::Result<T, TextError>;

// --- Печать ---------------------------------------------------------------

/// Печатает программу. `source` — путь исходника, уходит в комментарий
/// заголовка (на разбор не влияет).
pub fn write_program(program: &Program, source: Option<&str>) -> Result<String> {
    let mut out = String::with_capacity(4096);
    writeln!(out, "bslc {FORMAT_VERSION}").unwrap();
    if let Some(src) = source {
        writeln!(out, "; исходник: {src}").unwrap();
    }

    writeln!(out, "\n.names {}", program.names.len()).unwrap();
    for (i, name) in program.names.iter().enumerate() {
        writeln!(out, "  {i} {}", quote(name)).unwrap();
    }

    writeln!(out, "\n.shapes {}", program.shapes.len()).unwrap();
    for (i, shape) in program.shapes.iter().enumerate() {
        let ids: Vec<String> = shape.names.iter().map(|n| n.index().to_string()).collect();
        writeln!(
            out,
            "  {i} [{}]  ; {}",
            ids.join(" "),
            field_names(&shape.names, program)
        )
        .unwrap();
    }

    writeln!(out, "\n.top-locals {}", program.top_level_locals.len()).unwrap();
    for (i, name) in program.top_level_locals.iter().enumerate() {
        writeln!(out, "  {i} {}", quote(name)).unwrap();
    }

    writeln!(out, "\n.module-vars {}", program.module_vars.len()).unwrap();
    for (i, name) in program.module_vars.iter().enumerate() {
        writeln!(out, "  {i} {}", quote(name)).unwrap();
    }

    writeln!(out, "\n.functions {}", program.function_names.len()).unwrap();
    for (i, name) in program.function_names.iter().enumerate() {
        // `i` -> chunks[i+1]; номер чанка в комментарии, чтобы не считать
        // в уме при чтении `Call func=N`.
        writeln!(out, "  {i} {}  ; .chunk {}", quote(name), i + 1).unwrap();
    }

    for (i, chunk) in program.chunks.iter().enumerate() {
        write_chunk(&mut out, i, chunk, program)?;
    }
    Ok(out)
}

fn write_chunk(out: &mut String, index: usize, chunk: &Chunk, program: &Program) -> Result<()> {
    let what = if index == 0 {
        "верхний уровень".to_string()
    } else {
        match program.function_names.get(index - 1) {
            Some(name) => name.clone(),
            None => "процедура/функция".to_string(),
        }
    };
    let modes: Vec<&str> = chunk
        .param_by_val
        .iter()
        .map(|by_val| if *by_val { "value" } else { "byref" })
        .collect();
    writeln!(
        out,
        "\n.chunk {index} params={} locals={} regs={} argmodes=[{}]  ; {what}",
        chunk.n_params,
        chunk.n_locals,
        chunk.n_regs,
        modes.join(",")
    )
    .unwrap();

    writeln!(out, "  .consts {}", chunk.consts.len()).unwrap();
    for (k, v) in chunk.consts.iter().enumerate() {
        writeln!(out, "    {k} {}", write_const(v)?).unwrap();
    }

    writeln!(out, "  .argmodes {}", chunk.call_arg_modes.len()).unwrap();
    for (i, modes) in chunk.call_arg_modes.iter().enumerate() {
        let modes: Vec<String> = modes
            .iter()
            .map(|m| match m {
                ArgMode::Value => "value".to_string(),
                ArgMode::ByRefLocal(slot) => format!("byref:{slot}"),
            })
            .collect();
        writeln!(out, "    {i} [{}]", modes.join(" ")).unwrap();
    }

    writeln!(out, "  .handlers {}", chunk.exception_ranges.len()).unwrap();
    for (i, r) in chunk.exception_ranges.iter().enumerate() {
        writeln!(
            out,
            "    {i} {} {} {}  ; Попытка {}..{} -> обработчик {}",
            r.start_pc, r.end_pc, r.handler_pc, r.start_pc, r.end_pc, r.handler_pc
        )
        .unwrap();
    }

    writeln!(out, "  .localnames {}", chunk.local_names.len()).unwrap();
    for (i, name) in chunk.local_names.iter().enumerate() {
        writeln!(out, "    {i} {}", quote(name)).unwrap();
    }

    writeln!(out, "  .code {}", chunk.instrs.len()).unwrap();
    for (pc, instr) in chunk.instrs.iter().enumerate() {
        let text = write_instr(instr);
        match instr_comment(instr, chunk, program) {
            Some(c) => writeln!(out, "    {pc:04} {text}  ; {c}").unwrap(),
            None => writeln!(out, "    {pc:04} {text}").unwrap(),
        }
    }
    Ok(())
}

/// Подсказка справа от инструкции: то, что иначе пришлось бы искать по
/// таблицам глазами.
fn instr_comment(instr: &Instr, chunk: &Chunk, program: &Program) -> Option<String> {
    match instr {
        Instr::LoadConst { k, .. } => chunk
            .consts
            .get(*k as usize)
            .map(|v| format!("= {}", short_value(v))),
        Instr::GetProp { name, .. } | Instr::SetProp { name, .. } => {
            program.names.get(name.index()).map(|n| format!(".{n}"))
        }
        Instr::NewStructure { shape, .. } => program
            .shapes
            .get(*shape as usize)
            .map(|s| format!("поля: {}", field_names(&s.names, program))),
        Instr::GetModuleVar { slot, .. } | Instr::SetModuleVar { slot, .. } => program
            .module_vars
            .get(*slot as usize)
            .map(|n| format!("модульная {n}")),
        Instr::Call { func, .. } => Some(match program.function_names.get(*func as usize - 1) {
            Some(name) => format!("-> {name} (.chunk {func})"),
            None => format!("-> .chunk {func}"),
        }),
        _ => None,
    }
}

fn field_names(ids: &[NameId], program: &Program) -> String {
    ids.iter()
        .map(|id| program.names.get(id.index()).cloned().unwrap_or_default())
        .collect::<Vec<_>>()
        .join(", ")
}

/// Короткое представление значения для комментария (не для разбора).
fn short_value(v: &BslValue) -> String {
    match v {
        BslValue::Str(s) => {
            let s = s.to_string();
            if s.chars().count() > 40 {
                format!("{:.40}…", s)
            } else {
                format!("{s:?}")
            }
        }
        other => other.to_string(),
    }
}

fn write_const(v: &BslValue) -> Result<String> {
    Ok(match v {
        BslValue::Undefined => "Неопределено".to_string(),
        BslValue::Null => "Null".to_string(),
        BslValue::Boolean(b) => format!("Булево {}", if *b { "Истина" } else { "Ложь" }),
        // Каноническая форма числа — та же, что у оракула замеров: точка
        // как разделитель, без группировки, без потери разрядов.
        BslValue::Number(n) => format!("Число {}", n.to_canonical()),
        BslValue::Str(s) => format!("Строка {}", quote(&s.to_string())),
        // Секундами от эпохи, а не `дд.ММ.гггг`: представление даты по
        // умолчанию — само по себе открытый вопрос (FMT.DATE.DEFAULT), и
        // байт-код не должен от него зависеть.
        BslValue::Date(d) => format!("Дата {}", d.seconds()),
        BslValue::Type(_) => return Err(TextError::Unrepresentable("Тип")),
        BslValue::Object(_) => return Err(TextError::Unrepresentable("объект")),
        BslValue::Skipped => "Пропущено".to_string(),
    })
}

fn write_instr(instr: &Instr) -> String {
    match instr {
        Instr::Move { dst, src } => format!("Move dst={dst} src={src}"),
        Instr::GetModuleVar { dst, slot } => format!("GetModuleVar dst={dst} slot={slot}"),
        Instr::SetModuleVar { slot, src } => format!("SetModuleVar slot={slot} src={src}"),
        Instr::LoadConst { dst, k } => format!("LoadConst dst={dst} k={k}"),
        Instr::LoadBool { dst, val } => format!("LoadBool dst={dst} val={val}"),
        Instr::LoadUndefined { dst } => format!("LoadUndefined dst={dst}"),
        Instr::LoadNull { dst } => format!("LoadNull dst={dst}"),
        Instr::LoadSkipped { dst } => format!("LoadSkipped dst={dst}"),
        Instr::Add { dst, a, b } => format!("Add dst={dst} a={a} b={b}"),
        Instr::Sub { dst, a, b } => format!("Sub dst={dst} a={a} b={b}"),
        Instr::Mul { dst, a, b } => format!("Mul dst={dst} a={a} b={b}"),
        Instr::Div { dst, a, b } => format!("Div dst={dst} a={a} b={b}"),
        Instr::Neg { dst, src } => format!("Neg dst={dst} src={src}"),
        Instr::Not { dst, src } => format!("Not dst={dst} src={src}"),
        Instr::Eq { dst, a, b } => format!("Eq dst={dst} a={a} b={b}"),
        Instr::NotEq { dst, a, b } => format!("NotEq dst={dst} a={a} b={b}"),
        Instr::Lt { dst, a, b } => format!("Lt dst={dst} a={a} b={b}"),
        Instr::Gt { dst, a, b } => format!("Gt dst={dst} a={a} b={b}"),
        Instr::Le { dst, a, b } => format!("Le dst={dst} a={a} b={b}"),
        Instr::Ge { dst, a, b } => format!("Ge dst={dst} a={a} b={b}"),
        Instr::Jump { target } => format!("Jump target={target}"),
        Instr::JumpIfFalse { cond, target } => format!("JumpIfFalse cond={cond} target={target}"),
        Instr::JumpIfTrue { cond, target } => format!("JumpIfTrue cond={cond} target={target}"),
        Instr::JumpIfNotSkipped { src, target } => {
            format!("JumpIfNotSkipped src={src} target={target}")
        }
        Instr::NumericForNext {
            counter,
            bound,
            target,
        } => format!("NumericForNext counter={counter} bound={bound} target={target}"),
        Instr::NumericForNextI64 {
            counter,
            bound,
            target,
        } => format!("NumericForNextI64 counter={counter} bound={bound} target={target}"),
        Instr::Call {
            func,
            base,
            arg_modes,
            ret,
        } => format!("Call func={func} base={base} arg_modes={arg_modes} ret={ret}"),
        Instr::Return { src } => match src {
            Some(src) => format!("Return src={src}"),
            None => "Return".to_string(),
        },
        Instr::GetIndex { dst, obj, idx } => format!("GetIndex dst={dst} obj={obj} idx={idx}"),
        Instr::SetIndex { obj, idx, src } => format!("SetIndex obj={obj} idx={idx} src={src}"),
        Instr::GetProp { dst, obj, name } => {
            format!("GetProp dst={dst} obj={obj} name={}", name.index())
        }
        Instr::SetProp { obj, name, src } => {
            format!("SetProp obj={obj} name={} src={src}", name.index())
        }
        Instr::NewArray { dst, base, count } => {
            format!("NewArray dst={dst} base={base} count={count}")
        }
        Instr::NewStructure {
            dst,
            shape,
            base,
            count,
        } => format!("NewStructure dst={dst} shape={shape} base={base} count={count}"),
        Instr::NewTable { dst } => format!("NewTable dst={dst}"),
        Instr::NewMap { dst } => format!("NewMap dst={dst}"),
        Instr::NewTextWriter { dst, path } => format!("NewTextWriter dst={dst} path={path}"),
        Instr::CollectionLen { dst, obj } => format!("CollectionLen dst={dst} obj={obj}"),
        Instr::Raise { src } => match src {
            Some(src) => format!("Raise src={src}"),
            None => "Raise".to_string(),
        },
        Instr::CallBuiltin {
            dst,
            builtin,
            base,
            count,
        } => format!(
            "CallBuiltin dst={dst} builtin={} base={base} count={count}",
            builtin_name(*builtin)
        ),
        Instr::CallMethod {
            dst,
            obj,
            method,
            base,
            count,
        } => format!(
            "CallMethod dst={dst} obj={obj} method={} base={base} count={count}",
            method_name(*method)
        ),
        Instr::WriteText { dst, obj, src } => format!("WriteText dst={dst} obj={obj} src={src}"),
        Instr::CloseText { dst, obj } => format!("CloseText dst={dst} obj={obj}"),
        Instr::RunDynamic { src, dst, is_eval } => {
            format!("RunDynamic src={src} dst={dst} is_eval={is_eval}")
        }
    }
}

/// Имя встроенной функции для печати — ПЕРВОЕ написание из таблицы
/// `bsl_rt::BUILTIN_FN_NAMES` (там же, откуда их берёт автодополнение).
/// Разбор идёт обратно через `BuiltinFn::lookup`, так что таблица —
/// единственный источник и здесь.
fn builtin_name(f: bsl_rt::BuiltinFn) -> &'static str {
    bsl_rt::BUILTIN_FN_NAMES
        .iter()
        .find(|(_, v)| *v == f)
        .map(|(n, _)| *n)
        .unwrap_or("?")
}

fn method_name(m: bsl_rt::BuiltinMethod) -> &'static str {
    bsl_rt::BUILTIN_METHOD_NAMES
        .iter()
        .find(|(_, v)| *v == m)
        .map(|(n, _)| *n)
        .unwrap_or("?")
}

/// Строковый литерал с экранированием. `;` внутри кавычек не начинает
/// комментарий — разбор снимает комментарии уже после выделения строки.
fn quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            other => out.push(other),
        }
    }
    out.push('"');
    out
}

// --- Разбор ---------------------------------------------------------------

/// Одна значащая строка файла: номер (для сообщений) и текст без
/// комментария.
struct Line {
    no: usize,
    text: String,
}

/// Снимает комментарии (`;` вне кавычек) и пустые строки.
fn significant_lines(src: &str) -> Vec<Line> {
    let mut out = Vec::new();
    for (i, raw) in src.lines().enumerate() {
        let mut text = String::with_capacity(raw.len());
        let mut in_quotes = false;
        let mut escaped = false;
        for c in raw.chars() {
            if in_quotes {
                text.push(c);
                if escaped {
                    escaped = false;
                } else if c == '\\' {
                    escaped = true;
                } else if c == '"' {
                    in_quotes = false;
                }
                continue;
            }
            match c {
                '"' => {
                    in_quotes = true;
                    text.push(c);
                }
                ';' => break,
                other => text.push(other),
            }
        }
        let text = text.trim().to_string();
        if !text.is_empty() {
            out.push(Line { no: i + 1, text });
        }
    }
    out
}

struct Reader {
    lines: Vec<Line>,
    pos: usize,
}

impl Reader {
    fn next(&mut self) -> Option<&Line> {
        let line = self.lines.get(self.pos)?;
        self.pos += 1;
        Some(line)
    }

    fn expect(&mut self, what: &str) -> Result<(usize, String)> {
        match self.next() {
            Some(l) => Ok((l.no, l.text.clone())),
            None => Err(TextError::At(
                self.lines.last().map(|l| l.no).unwrap_or(0),
                format!("файл кончился, ожидалось: {what}"),
            )),
        }
    }

    /// `.директива N` — возвращает N.
    fn directive(&mut self, name: &str) -> Result<usize> {
        let (no, text) = self.expect(name)?;
        let mut parts = text.split_whitespace();
        match (parts.next(), parts.next()) {
            (Some(d), Some(n)) if d == name => n
                .parse::<usize>()
                .map_err(|_| TextError::At(no, format!("{name}: «{n}» не число"))),
            _ => Err(TextError::At(no, format!("ожидалась директива {name} N"))),
        }
    }
}

fn parse_index(no: usize, got: &str, expected: usize) -> Result<()> {
    let idx: usize = got
        .parse()
        .map_err(|_| TextError::At(no, format!("«{got}» не индекс")))?;
    if idx != expected {
        return Err(TextError::At(
            no,
            format!("индекс {idx}, а по порядку ожидался {expected}"),
        ));
    }
    Ok(())
}

/// Снимает строковый литерал с начала `text`, возвращает его и остаток.
fn unquote(no: usize, text: &str) -> Result<(String, String)> {
    let mut chars = text.char_indices();
    match chars.next() {
        Some((_, '"')) => {}
        _ => return Err(TextError::At(no, format!("ожидалась строка: «{text}»"))),
    }
    let mut out = String::new();
    let mut escaped = false;
    for (i, c) in chars {
        if escaped {
            out.push(match c {
                'n' => '\n',
                'r' => '\r',
                't' => '\t',
                other => other,
            });
            escaped = false;
            continue;
        }
        match c {
            '\\' => escaped = true,
            '"' => return Ok((out, text[i + 1..].trim().to_string())),
            other => out.push(other),
        }
    }
    Err(TextError::At(no, "незакрытая строка".to_string()))
}

pub fn parse_program(src: &str) -> Result<Program> {
    let lines = significant_lines(src);
    let mut r = Reader { lines, pos: 0 };

    let (no, header) = r
        .expect("заголовок")
        .map_err(|_| TextError::BadHeader("файл пуст".to_string()))?;
    let mut parts = header.split_whitespace();
    match (parts.next(), parts.next()) {
        (Some("bslc"), Some(v)) if v.parse::<u32>() == Ok(FORMAT_VERSION) => {}
        (Some("bslc"), Some(v)) => {
            return Err(TextError::BadHeader(format!(
                "версия формата {v}, этот бинарник понимает только {FORMAT_VERSION}"
            )))
        }
        _ => {
            return Err(TextError::BadHeader(format!(
                "строка {no}: ожидалось «bslc {FORMAT_VERSION}»"
            )))
        }
    }

    // Имена.
    let n = r.directive(".names")?;
    let mut names = Vec::with_capacity(n);
    for i in 0..n {
        let (no, text) = r.expect("имя")?;
        let (idx, rest) = text
            .split_once(char::is_whitespace)
            .ok_or_else(|| TextError::At(no, "ожидалось «N \"имя\"»".to_string()))?;
        parse_index(no, idx, i)?;
        let (name, tail) = unquote(no, rest.trim())?;
        if !tail.is_empty() {
            return Err(TextError::At(no, format!("лишнее после имени: «{tail}»")));
        }
        names.push(name);
    }

    // Формы. Интернируются в свежую таблицу В ТОМ ЖЕ ПОРЯДКЕ, поэтому
    // индексы форм совпадают с исходными — на них ссылается NewStructure.
    let n = r.directive(".shapes")?;
    let mut shapes = ShapeTable::new();
    for i in 0..n {
        let (no, text) = r.expect("форму")?;
        let (idx, rest) = text
            .split_once(char::is_whitespace)
            .ok_or_else(|| TextError::At(no, "ожидалось «N [поля]»".to_string()))?;
        parse_index(no, idx, i)?;
        let ids = parse_id_list(no, rest.trim())?;
        let ids: Vec<NameId> = ids.into_iter().map(NameId::from_index).collect();
        let interned = shapes.intern(&ids);
        if interned as usize != i {
            return Err(TextError::At(
                no,
                format!("форма {i} совпала с уже описанной формой {interned}"),
            ));
        }
    }
    let shapes: Vec<std::rc::Rc<Shape>> = shapes.into_shapes();

    // Локали верхнего уровня.
    let n = r.directive(".top-locals")?;
    let mut top_level_locals = Vec::with_capacity(n);
    for i in 0..n {
        let (no, text) = r.expect("имя локали")?;
        let (idx, rest) = text
            .split_once(char::is_whitespace)
            .ok_or_else(|| TextError::At(no, "ожидалось «N \"имя\"»".to_string()))?;
        parse_index(no, idx, i)?;
        let (name, _) = unquote(no, rest.trim())?;
        top_level_locals.push(name);
    }

    let n = r.directive(".module-vars")?;
    let mut module_vars = Vec::with_capacity(n);
    for i in 0..n {
        let (no, text) = r.expect("имя переменной модуля")?;
        let (idx, rest) = text
            .split_once(char::is_whitespace)
            .ok_or_else(|| TextError::At(no, "ожидалось «N \"имя\"»".to_string()))?;
        parse_index(no, idx, i)?;
        let (name, _) = unquote(no, rest.trim())?;
        module_vars.push(name);
    }

    let n = r.directive(".functions")?;
    let mut function_names = Vec::with_capacity(n);
    for i in 0..n {
        let (no, text) = r.expect("имя функции")?;
        let (idx, rest) = text
            .split_once(char::is_whitespace)
            .ok_or_else(|| TextError::At(no, "ожидалось «N \"имя\"»".to_string()))?;
        parse_index(no, idx, i)?;
        let (name, _) = unquote(no, rest.trim())?;
        function_names.push(name);
    }

    // Чанки — до конца файла.
    let mut chunks = Vec::new();
    while r.pos < r.lines.len() {
        chunks.push(parse_chunk(&mut r, chunks.len())?);
    }
    if chunks.is_empty() {
        return Err(TextError::At(0, "нет ни одного .chunk".to_string()));
    }

    Ok(Program {
        chunks,
        names,
        shapes,
        top_level_locals,
        function_names,
        module_vars,
        module_base: 0,
    })
}

fn parse_id_list(no: usize, text: &str) -> Result<Vec<u32>> {
    let inner = text
        .strip_prefix('[')
        .and_then(|t| t.strip_suffix(']'))
        .ok_or_else(|| TextError::At(no, format!("ожидался список в скобках: «{text}»")))?;
    inner
        .split_whitespace()
        .map(|t| {
            t.parse::<u32>()
                .map_err(|_| TextError::At(no, format!("«{t}» не число")))
        })
        .collect()
}

fn parse_chunk(r: &mut Reader, expected_index: usize) -> Result<Chunk> {
    let (no, text) = r.expect(".chunk")?;
    let mut parts = text.split_whitespace();
    if parts.next() != Some(".chunk") {
        return Err(TextError::At(no, format!("ожидался .chunk: «{text}»")));
    }
    let idx = parts
        .next()
        .ok_or_else(|| TextError::At(no, "у .chunk нет номера".to_string()))?;
    parse_index(no, idx, expected_index)?;
    let fields = key_values(no, parts)?;
    let n_params = field_u8(&fields, no, "params")?;
    let n_locals = field_u8(&fields, no, "locals")?;
    let n_regs = field_u8(&fields, no, "regs")?;
    // `argmodes=[value byref]` — режимы ПАРАМЕТРОВ этой функции (не
    // аргументов её вызовов, те в `.argmodes` ниже).
    let param_by_val: Vec<bool> = match field(&fields, no, "argmodes") {
        Ok(list) => {
            let inner = list
                .strip_prefix('[')
                .and_then(|t| t.strip_suffix(']'))
                .ok_or_else(|| TextError::At(no, format!("ожидался список: «{list}»")))?;
            inner
                .split(',')
                .filter(|t| !t.is_empty())
                .map(|t| match t {
                    "value" => Ok(true),
                    "byref" => Ok(false),
                    other => Err(TextError::At(no, format!("режим параметра «{other}»"))),
                })
                .collect::<Result<_>>()?
        }
        Err(_) => Vec::new(),
    };

    let n = r.directive(".consts")?;
    let mut consts = Vec::with_capacity(n);
    for i in 0..n {
        let (no, text) = r.expect("константу")?;
        let (idx, rest) = text
            .split_once(char::is_whitespace)
            .ok_or_else(|| TextError::At(no, "ожидалось «N значение»".to_string()))?;
        parse_index(no, idx, i)?;
        consts.push(parse_const(no, rest.trim())?);
    }

    let n = r.directive(".argmodes")?;
    let mut call_arg_modes = Vec::with_capacity(n);
    for i in 0..n {
        let (no, text) = r.expect("режимы аргументов")?;
        let (idx, rest) = text
            .split_once(char::is_whitespace)
            .ok_or_else(|| TextError::At(no, "ожидалось «N [режимы]»".to_string()))?;
        parse_index(no, idx, i)?;
        let rest = rest.trim();
        let inner = rest
            .strip_prefix('[')
            .and_then(|t| t.strip_suffix(']'))
            .ok_or_else(|| TextError::At(no, format!("ожидался список: «{rest}»")))?;
        let mut modes = Vec::new();
        for token in inner.split_whitespace() {
            modes.push(match token {
                "value" => ArgMode::Value,
                t => match t.strip_prefix("byref:") {
                    Some(slot) => ArgMode::ByRefLocal(
                        slot.parse()
                            .map_err(|_| TextError::At(no, format!("«{slot}» не слот")))?,
                    ),
                    None => return Err(TextError::At(no, format!("неизвестный режим «{t}»"))),
                },
            });
        }
        call_arg_modes.push(modes);
    }

    let n = r.directive(".handlers")?;
    let mut exception_ranges = Vec::with_capacity(n);
    for i in 0..n {
        let (no, text) = r.expect("обработчик")?;
        let nums: Vec<&str> = text.split_whitespace().collect();
        if nums.len() != 4 {
            return Err(TextError::At(
                no,
                "ожидалось «N начало конец обработчик»".to_string(),
            ));
        }
        parse_index(no, nums[0], i)?;
        let num = |s: &str| {
            s.parse::<usize>()
                .map_err(|_| TextError::At(no, format!("«{s}» не число")))
        };
        exception_ranges.push(ExceptionRange {
            start_pc: num(nums[1])?,
            end_pc: num(nums[2])?,
            handler_pc: num(nums[3])?,
        });
    }

    let n = r.directive(".localnames")?;
    let mut local_names = Vec::with_capacity(n);
    for i in 0..n {
        let (no, text) = r.expect("имя слота")?;
        let (idx, rest) = text
            .split_once(char::is_whitespace)
            .ok_or_else(|| TextError::At(no, "ожидалось «N \"имя\"»".to_string()))?;
        parse_index(no, idx, i)?;
        let (name, _) = unquote(no, rest.trim())?;
        local_names.push(name);
    }

    let n = r.directive(".code")?;
    let mut instrs = Vec::with_capacity(n);
    for i in 0..n {
        let (no, text) = r.expect("инструкцию")?;
        let (pc, rest) = text
            .split_once(char::is_whitespace)
            .ok_or_else(|| TextError::At(no, "ожидалось «PC Опкод ...»".to_string()))?;
        // Адрес печатается с ведущими нулями (`0007`) — `parse::<usize>`
        // их и так понимает, отдельная чистка не нужна. Сверка адреса с
        // позицией не формальность: прыжки в этом формате абсолютные, и
        // строка, вставленная руками в середину `.code`, сдвинула бы все
        // цели молча.
        parse_index(no, pc, i)?;
        instrs.push(parse_instr(no, rest.trim())?);
    }

    Ok(Chunk {
        param_by_val,
        prop_cache: (0..instrs.len()).map(|_| RefCell::new(None)).collect(),
        instrs,
        consts,
        call_arg_modes,
        exception_ranges,
        n_params,
        n_locals,
        n_regs,
        local_names,
    })
}

fn parse_const(no: usize, text: &str) -> Result<BslValue> {
    let (tag, rest) = match text.split_once(char::is_whitespace) {
        Some((tag, rest)) => (tag, rest.trim()),
        None => (text, ""),
    };
    Ok(match tag {
        "Неопределено" => BslValue::Undefined,
        "Null" => BslValue::Null,
        "Пропущено" => BslValue::Skipped,
        "Булево" => match rest {
            "Истина" => BslValue::Boolean(true),
            "Ложь" => BslValue::Boolean(false),
            other => return Err(TextError::At(no, format!("«{other}» не Истина/Ложь"))),
        },
        "Число" => BslValue::Number(
            BslNumber::parse_canonical(rest)
                .map_err(|e| TextError::At(no, format!("число «{rest}»: {e}")))?,
        ),
        "Строка" => BslValue::Str(BslString::from_str(&unquote(no, rest)?.0)),
        "Дата" => {
            let secs: i64 = rest
                .parse()
                .map_err(|_| TextError::At(no, format!("«{rest}» не секунды")))?;
            BslValue::Date(
                BslDate::from_seconds(secs)
                    .ok_or_else(|| TextError::At(no, format!("дата вне диапазона: {secs}")))?,
            )
        }
        other => return Err(TextError::At(no, format!("неизвестный тип «{other}»"))),
    })
}

/// `ключ=значение ...` в карту. Порядок полей в инструкции значения не
/// имеет — читается по имени, как оно и печатается.
fn key_values<'a>(
    no: usize,
    parts: impl Iterator<Item = &'a str>,
) -> Result<Vec<(String, String)>> {
    let mut out = Vec::new();
    for part in parts {
        let (k, v) = part.split_once('=').ok_or_else(|| {
            TextError::At(no, format!("ожидалось «ключ=значение», а не «{part}»"))
        })?;
        out.push((k.to_string(), v.to_string()));
    }
    Ok(out)
}

fn field<'a>(fields: &'a [(String, String)], no: usize, key: &str) -> Result<&'a str> {
    fields
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.as_str())
        .ok_or_else(|| TextError::At(no, format!("нет поля «{key}»")))
}

fn field_u8(fields: &[(String, String)], no: usize, key: &str) -> Result<u8> {
    field(fields, no, key)?
        .parse()
        .map_err(|_| TextError::At(no, format!("поле «{key}»: не число 0..255")))
}

fn field_u16(fields: &[(String, String)], no: usize, key: &str) -> Result<u16> {
    field(fields, no, key)?
        .parse()
        .map_err(|_| TextError::At(no, format!("поле «{key}»: не число 0..65535")))
}

fn field_i16(fields: &[(String, String)], no: usize, key: &str) -> Result<i16> {
    field(fields, no, key)?
        .parse()
        .map_err(|_| TextError::At(no, format!("поле «{key}»: не число -32768..32767")))
}

fn field_bool(fields: &[(String, String)], no: usize, key: &str) -> Result<bool> {
    match field(fields, no, key)? {
        "true" => Ok(true),
        "false" => Ok(false),
        other => Err(TextError::At(
            no,
            format!("поле «{key}»: «{other}» не true/false"),
        )),
    }
}

fn field_name(fields: &[(String, String)], no: usize, key: &str) -> Result<NameId> {
    Ok(NameId::from_index(field_u16(fields, no, key)? as u32))
}

fn parse_instr(no: usize, text: &str) -> Result<Instr> {
    let mut parts = text.split_whitespace();
    let op = parts
        .next()
        .ok_or_else(|| TextError::At(no, "пустая инструкция".to_string()))?;
    if !OPCODES.contains(&op) {
        return Err(TextError::At(no, format!("неизвестный опкод «{op}»")));
    }
    let f = key_values(no, parts)?;
    let dst = |f: &[(String, String)]| field_u8(f, no, "dst");
    let src = |f: &[(String, String)]| field_u8(f, no, "src");
    let a = |f: &[(String, String)]| field_u8(f, no, "a");
    let b = |f: &[(String, String)]| field_u8(f, no, "b");
    let obj = |f: &[(String, String)]| field_u8(f, no, "obj");
    let base = |f: &[(String, String)]| field_u8(f, no, "base");
    let count = |f: &[(String, String)]| field_u8(f, no, "count");
    let cond = |f: &[(String, String)]| field_u8(f, no, "cond");
    let target = |f: &[(String, String)]| field_i16(f, no, "target");

    Ok(match op {
        "Move" => Instr::Move {
            dst: dst(&f)?,
            src: src(&f)?,
        },
        "GetModuleVar" => Instr::GetModuleVar {
            dst: dst(&f)?,
            slot: field_u16(&f, no, "slot")?,
        },
        "SetModuleVar" => Instr::SetModuleVar {
            slot: field_u16(&f, no, "slot")?,
            src: src(&f)?,
        },
        "LoadConst" => Instr::LoadConst {
            dst: dst(&f)?,
            k: field_u16(&f, no, "k")?,
        },
        "LoadBool" => Instr::LoadBool {
            dst: dst(&f)?,
            val: field_bool(&f, no, "val")?,
        },
        "LoadUndefined" => Instr::LoadUndefined { dst: dst(&f)? },
        "LoadNull" => Instr::LoadNull { dst: dst(&f)? },
        "LoadSkipped" => Instr::LoadSkipped { dst: dst(&f)? },
        "Add" => Instr::Add {
            dst: dst(&f)?,
            a: a(&f)?,
            b: b(&f)?,
        },
        "Sub" => Instr::Sub {
            dst: dst(&f)?,
            a: a(&f)?,
            b: b(&f)?,
        },
        "Mul" => Instr::Mul {
            dst: dst(&f)?,
            a: a(&f)?,
            b: b(&f)?,
        },
        "Div" => Instr::Div {
            dst: dst(&f)?,
            a: a(&f)?,
            b: b(&f)?,
        },
        "Neg" => Instr::Neg {
            dst: dst(&f)?,
            src: src(&f)?,
        },
        "Not" => Instr::Not {
            dst: dst(&f)?,
            src: src(&f)?,
        },
        "Eq" => Instr::Eq {
            dst: dst(&f)?,
            a: a(&f)?,
            b: b(&f)?,
        },
        "NotEq" => Instr::NotEq {
            dst: dst(&f)?,
            a: a(&f)?,
            b: b(&f)?,
        },
        "Lt" => Instr::Lt {
            dst: dst(&f)?,
            a: a(&f)?,
            b: b(&f)?,
        },
        "Gt" => Instr::Gt {
            dst: dst(&f)?,
            a: a(&f)?,
            b: b(&f)?,
        },
        "Le" => Instr::Le {
            dst: dst(&f)?,
            a: a(&f)?,
            b: b(&f)?,
        },
        "Ge" => Instr::Ge {
            dst: dst(&f)?,
            a: a(&f)?,
            b: b(&f)?,
        },
        "Jump" => Instr::Jump {
            target: target(&f)?,
        },
        "JumpIfFalse" => Instr::JumpIfFalse {
            cond: cond(&f)?,
            target: target(&f)?,
        },
        "JumpIfTrue" => Instr::JumpIfTrue {
            cond: cond(&f)?,
            target: target(&f)?,
        },
        "JumpIfNotSkipped" => Instr::JumpIfNotSkipped {
            src: src(&f)?,
            target: target(&f)?,
        },
        "NumericForNext" => Instr::NumericForNext {
            counter: field_u8(&f, no, "counter")?,
            bound: field_u8(&f, no, "bound")?,
            target: target(&f)?,
        },
        "NumericForNextI64" => Instr::NumericForNextI64 {
            counter: field_u8(&f, no, "counter")?,
            bound: field_u8(&f, no, "bound")?,
            target: target(&f)?,
        },
        "Call" => Instr::Call {
            func: field_u16(&f, no, "func")?,
            base: base(&f)?,
            arg_modes: field_u16(&f, no, "arg_modes")?,
            ret: field_u8(&f, no, "ret")?,
        },
        "Return" => Instr::Return {
            src: match field(&f, no, "src") {
                Ok(_) => Some(src(&f)?),
                Err(_) => None,
            },
        },
        "GetIndex" => Instr::GetIndex {
            dst: dst(&f)?,
            obj: obj(&f)?,
            idx: field_u8(&f, no, "idx")?,
        },
        "SetIndex" => Instr::SetIndex {
            obj: obj(&f)?,
            idx: field_u8(&f, no, "idx")?,
            src: src(&f)?,
        },
        "GetProp" => Instr::GetProp {
            dst: dst(&f)?,
            obj: obj(&f)?,
            name: field_name(&f, no, "name")?,
        },
        "SetProp" => Instr::SetProp {
            obj: obj(&f)?,
            name: field_name(&f, no, "name")?,
            src: src(&f)?,
        },
        "NewArray" => Instr::NewArray {
            dst: dst(&f)?,
            base: base(&f)?,
            count: count(&f)?,
        },
        "NewStructure" => Instr::NewStructure {
            dst: dst(&f)?,
            shape: field_u16(&f, no, "shape")?,
            base: base(&f)?,
            count: count(&f)?,
        },
        "NewTable" => Instr::NewTable { dst: dst(&f)? },
        "NewMap" => Instr::NewMap { dst: dst(&f)? },
        "NewTextWriter" => Instr::NewTextWriter {
            dst: dst(&f)?,
            path: field_u8(&f, no, "path")?,
        },
        "CollectionLen" => Instr::CollectionLen {
            dst: dst(&f)?,
            obj: obj(&f)?,
        },
        "Raise" => Instr::Raise {
            src: match field(&f, no, "src") {
                Ok(_) => Some(src(&f)?),
                Err(_) => None,
            },
        },
        "CallBuiltin" => {
            let name = field(&f, no, "builtin")?;
            Instr::CallBuiltin {
                dst: dst(&f)?,
                builtin: bsl_rt::BuiltinFn::lookup(name).ok_or_else(|| {
                    TextError::At(no, format!("неизвестная встроенная функция «{name}»"))
                })?,
                base: base(&f)?,
                count: count(&f)?,
            }
        }
        "CallMethod" => {
            let name = field(&f, no, "method")?;
            Instr::CallMethod {
                dst: dst(&f)?,
                obj: obj(&f)?,
                method: bsl_rt::BuiltinMethod::lookup(name)
                    .ok_or_else(|| TextError::At(no, format!("неизвестный метод «{name}»")))?,
                base: base(&f)?,
                count: count(&f)?,
            }
        }
        "WriteText" => Instr::WriteText {
            dst: dst(&f)?,
            obj: obj(&f)?,
            src: src(&f)?,
        },
        "CloseText" => Instr::CloseText {
            dst: dst(&f)?,
            obj: obj(&f)?,
        },
        "RunDynamic" => Instr::RunDynamic {
            src: src(&f)?,
            dst: dst(&f)?,
            is_eval: field_bool(&f, no, "is_eval")?,
        },
        other => return Err(TextError::At(no, format!("неизвестный опкод «{other}»"))),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Корпус, на котором проверяется round-trip. Он же — покрытие
    /// опкодов: тест ниже требует, чтобы КАЖДЫЙ опкод из `OPCODES`
    /// встретился хотя бы раз, иначе расхождение печати и разбора
    /// обнаружилось бы уже у пользователя.
    const CORPUS: &[&str] = &[
        // Арифметика, сравнения, логика, унарные.
        "х = 1 + 2 * 3 - 4 / 5;\nу = -х;\nз = Не (х > 1 И х < 10 ИЛИ х = 5);\n\
         р = х <> 1; р = х >= 1; р = х <= 1; р = х = 1;\nк = у;\n",
        // Литералы всех видов, включая дату и строку с кавычками.
        "а = Истина; б = Ложь; в = Неопределено; г = Null;\n\
         д = '20240115103000'; е = \"строка с \"\"кавычками\"\" и ;\";\n",
        // Ветвление и оба вида короткого замыкания.
        "Если 1 > 0 Тогда х = 1; ИначеЕсли 2 > 0 Тогда х = 2; Иначе х = 3; КонецЕсли;\n",
        // Числовой цикл с телом и заведомо пустой (для NumericForNextI64).
        "Для ном = 1 По 10 Цикл с = ном; КонецЦикла;\nДля пуст = 1 По 10 Цикл КонецЦикла;\n",
        // Цикл по коллекции: CollectionLen, GetIndex.
        "м = Новый Массив; м.Добавить(1); Для Каждого э Из м Цикл ч = э; КонецЦикла;\n",
        // Коллекции: NewArray/NewStructure/NewTable/NewMap, Get/SetIndex, Get/SetProp.
        "м = Новый Массив(3);\nм[0] = 5;\nз = м[0];\n\
         с = Новый Структура(\"поле,ещё\", 1, 2);\nс.поле = с.ещё;\n\
         т = Новый ТаблицаЗначений;\nсо = Новый Соответствие;\n\
         со.Вставить(\"к\", 1);\nн = м.Количество();\n",
        // Пользовательские функции: Call, Return, параметры по ссылке и
        // по значению, значение по умолчанию и пропущенный аргумент.
        "Функция Ф(а, Знач б = 7, в = 9)\n  а = а + б + в;\n  Возврат а;\nКонецФункции\n\
         Процедура П()\n  Возврат;\nКонецПроцедуры\n\
         х = 1;\nу = Ф(х, , 3);\nП();\n",
        // Исключения: обработчик и обе формы ВызватьИсключение.
        "Попытка\n  ВызватьИсключение \"беда\";\nИсключение\n  ВызватьИсключение;\nКонецПопытки;\n",
        // Встроенные функции и методы объектов.
        "х = Sqrt(2);\nс = Формат(1/3, \"ЧГ=0\");\n\
         т = Новый ТаблицаЗначений;\nт.Колонки.Добавить(\"ц\");\nт.Свернуть(\"ц\");\n",
        // Динамическое исполнение — обе формы.
        "х = 1;\nВыполнить(\"х = 2\");\nу = Вычислить(\"х + 1\");\n",
        // Переменные уровня модуля: чтение и запись из процедуры.
        "Перем Общая;\n\
         Процедура Пишет()\n  Общая = 1;\nКонецПроцедуры\n\
         Функция Читает()\n  Возврат Общая;\nКонецФункции\n\
         Общая = 0;\nПишет();\nх = Читает();\n",
        // Запись текста: NewTextWriter и оба горячих пути.
        "з = Новый ЗаписьТекста(\"/dev/null\");\nз.Записать(\"строка\");\nз.Закрыть();\n",
    ];

    fn compile(src: &str) -> Program {
        let parsed = bsl_syntax::parse(src).unwrap_or_else(|e| panic!("{src}\nparse: {e:?}"));
        let resolved = bsl_sema::resolve_program(&parsed.items)
            .unwrap_or_else(|e| panic!("{src}\nsema: {e:?}"));
        crate::compile_program(&resolved).unwrap_or_else(|e| panic!("{src}\ncompile: {e:?}"))
    }

    /// ГЛАВНЫЙ инвариант формата: печать -> разбор -> печать даёт ту же
    /// строку. Побайтово, а не «эквивалентно»: любое расхождение здесь
    /// значит, что часть программы потерялась при одном из переходов.
    #[test]
    fn round_trip_through_text_is_byte_identical() {
        for src in CORPUS {
            let program = compile(src);
            let first = write_program(&program, None).unwrap();
            let reparsed = parse_program(&first)
                .unwrap_or_else(|e| panic!("исходник:\n{src}\nбайт-код:\n{first}\nошибка: {e}"));
            let second = write_program(&reparsed, None).unwrap();
            assert_eq!(first, second, "round-trip разошёлся на:\n{src}");
        }
    }

    /// Разобранная программа совпадает с исходной по СУЩЕСТВУ, а не только
    /// по печати: инструкции, константы, режимы аргументов, обработчики и
    /// размеры кадров.
    #[test]
    fn reparsed_program_matches_the_original_structurally() {
        for src in CORPUS {
            let a = compile(src);
            let b = parse_program(&write_program(&a, None).unwrap()).unwrap();
            assert_eq!(a.names, b.names, "{src}");
            assert_eq!(a.top_level_locals, b.top_level_locals, "{src}");
            assert_eq!(a.shapes.len(), b.shapes.len(), "{src}");
            for (x, y) in a.shapes.iter().zip(&b.shapes) {
                assert_eq!(x.names, y.names, "поля формы разошлись: {src}");
            }
            assert_eq!(a.chunks.len(), b.chunks.len(), "{src}");
            for (x, y) in a.chunks.iter().zip(&b.chunks) {
                assert_eq!(x.instrs, y.instrs, "{src}");
                assert_eq!(x.consts, y.consts, "{src}");
                assert_eq!(x.call_arg_modes, y.call_arg_modes, "{src}");
                assert_eq!(x.exception_ranges, y.exception_ranges, "{src}");
                assert_eq!(
                    (x.n_params, x.n_locals, x.n_regs),
                    (y.n_params, y.n_locals, y.n_regs)
                );
                assert_eq!(x.local_names, y.local_names, "{src}");
                // Кэш инлайн-кэширования не сохраняется, но обязан быть
                // размером с код — иначе VM промахнётся по индексу.
                assert_eq!(y.prop_cache.len(), y.instrs.len(), "{src}");
            }
        }
    }

    #[test]
    fn the_corpus_covers_every_opcode() {
        let mut seen: Vec<&str> = Vec::new();
        for src in CORPUS {
            let text = write_program(&compile(src), None).unwrap();
            for op in OPCODES {
                // Опкод — первое слово инструкции; ищем по границе, чтобы
                // `Eq` не «покрылся» строкой `NotEq`.
                if text
                    .lines()
                    .filter_map(|l| l.split_whitespace().nth(1))
                    .any(|word| word == *op)
                    && !seen.contains(op)
                {
                    seen.push(op);
                }
            }
        }
        let missing: Vec<&&str> = OPCODES.iter().filter(|op| !seen.contains(op)).collect();
        assert!(
            missing.is_empty(),
            "корпус не задевает опкоды: {missing:?}. Добавьте на них исходник — \
             иначе печать и разбор для них ничем не проверены."
        );
    }

    #[test]
    fn comments_and_blank_lines_are_ignored_by_the_parser() {
        let program = compile("х = 1 + 2;\n");
        let text = write_program(&program, Some("файл.bsl")).unwrap();
        // В печати комментарии есть...
        assert!(text.contains("; исходник: файл.bsl"));
        // ...и они не мешают разобрать её обратно.
        let reparsed = parse_program(&text).unwrap();
        assert_eq!(reparsed.chunks[0].instrs, program.chunks[0].instrs);

        // Комментарий, добавленный руками, тоже не мешает.
        let hand_edited = text.replace(".code", "; моя пометка\n  .code");
        assert!(parse_program(&hand_edited).is_ok());
    }

    #[test]
    fn a_semicolon_inside_a_string_constant_is_not_a_comment() {
        let program = compile("с = \"а;б\";\n");
        let text = write_program(&program, None).unwrap();
        let reparsed = parse_program(&text).unwrap();
        assert_eq!(reparsed.chunks[0].consts, program.chunks[0].consts);
    }

    #[test]
    fn a_foreign_or_missing_header_is_rejected() {
        assert!(matches!(
            parse_program("bslc 999\n.names 0\n"),
            Err(TextError::BadHeader(_))
        ));
        assert!(matches!(parse_program(""), Err(TextError::BadHeader(_))));
        assert!(matches!(
            parse_program("что-то другое\n"),
            Err(TextError::BadHeader(_))
        ));
    }

    #[test]
    fn a_corrupted_body_names_the_line() {
        let text = write_program(&compile("х = 1;\n"), None).unwrap();

        // Неизвестный опкод.
        let broken = text.replace("LoadConst", "ЛюбиРаботу");
        match parse_program(&broken) {
            Err(TextError::At(line, msg)) => {
                assert!(msg.contains("ЛюбиРаботу"), "{msg}");
                assert!(line > 0);
            }
            other => panic!("ожидалась ошибка с номером строки, получено {other:?}"),
        }

        // Сбитая нумерация инструкций — признак ручной правки, при которой
        // прыжки поехали бы молча.
        let broken = text.replace("0000 ", "0007 ");
        assert!(matches!(parse_program(&broken), Err(TextError::At(..))));

        // Обрыв файла на середине.
        let broken: String = text.lines().take(4).collect::<Vec<_>>().join("\n");
        assert!(parse_program(&broken).is_err());
    }

    #[test]
    fn objects_in_constants_are_refused_rather_than_printed_wrong() {
        let mut program = compile("х = 1;\n");
        program.chunks[0]
            .consts
            .push(BslValue::new_array(Vec::new()));
        assert!(matches!(
            write_program(&program, None),
            Err(TextError::Unrepresentable(_))
        ));
    }
}
