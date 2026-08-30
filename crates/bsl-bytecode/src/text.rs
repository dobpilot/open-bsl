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

use std::fmt::Write as _;

use bsl_number::BslNumber;
use bsl_rt::{BslDate, BslString, BslValue, LibraryRequirement, NameId, Shape, ShapeTable};

use crate::chunk::{Chunk, ExceptionRange, Program};
use crate::configuration::{LinkEntry, ModuleId};
use crate::instr::{ArgMode, Instr};

/// Номер формата. Меняется при любой правке синтаксиса — загрузчик
/// сверяет его и отказывается угадывать.
pub const FORMAT_VERSION: u32 = 28;

/// Имена опкодов — те же строки, что печатает `write_instr` и принимает
/// `parse_instr`. Список публичен, потому что на нём держится тест
/// покрытия: корпус round-trip обязан задеть каждый опкод, иначе
/// расхождение печати и разбора обнаружится не здесь, а у пользователя.
/// Единый источник имён опкодов: макрос порождает и список [`OPCODES`]
/// (его сверяет разбор), и [`Instr::opcode`] (его зовёт печать). Имя опкода
/// совпадает с идентификатором варианта `Instr`; сам `enum` остаётся
/// рукописным со всеми измеренными комментариями, а несовпадение имени в
/// этом списке с вариантом `Instr` — ошибка сборки (`Instr::$name`), не
/// молчаливое расхождение. Разбор (`parse_instr`) рукописный и связан с
/// этим источником круговым тестом формата (`text_round_trip`).
macro_rules! opcodes {
    ($($name:ident),+ $(,)?) => {
        pub const OPCODES: &[&str] = &[$(stringify!($name)),+];

        /// Число опкодов: размер гистограммы счётчиков исполнения.
        pub const OPCODE_COUNT: usize = OPCODES.len();

        impl Instr {
            /// Имя опкода — то же, что печатает формат и принимает разбор.
            pub fn opcode(&self) -> &'static str {
                match self {
                    $(Instr::$name { .. } => stringify!($name),)+
                }
            }

            /// Порядковый номер опкода — позиция в [`OPCODES`].
            ///
            /// Нужен гистограмме исполненных опкодов в счётчиках. Индекс
            /// порождается тем же макросом, что и имена: вспомогательное
            /// перечисление нумерует варианты в том же порядке, поэтому
            /// второго рукописного `match` по всем опкодам не появляется,
            /// а новый опкод получает индекс сам.
            pub fn opcode_index(&self) -> usize {
                #[allow(dead_code, non_camel_case_types)]
                enum Idx { $($name,)+ }
                match self {
                    $(Instr::$name { .. } => Idx::$name as usize,)+
                }
            }
        }
    };
}

opcodes! {
    Move, GetModuleVar, SetModuleVar, LoadConst, LoadBool, LoadUndefined, LoadNull,
    Add, AddConst, Sub, Mul, Div, Mod, Neg, Not,
    Eq, NotEq, Lt, Gt, Le, Ge, Jump,
    JumpIfFalse, JumpIfTrue, JumpIfNotEqConst, JumpIfNotLtConst, JumpIfNotSkipped, NumericForNext,
    NumericForNextI64, Call, CallImported, Await, Return,
    GetImportedVar, SetImportedVar,
    GetIndex, SetIndex, GetProp, SetProp, CreateObject, NewArray, NewStructure,
    NewTable, NewTypeDescription, NewValueComparison, NewMap, NewTextWriter,
    CollectionLen, Raise, CallBuiltin, CallComponent, CallMethod,
    RunDynamic, CallObjectMethod, GetObjectProp, SetObjectProp,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TextError {
    /// Строка с номером и тем, что в ней не так. Номер — 1-based, как в
    /// редакторе: файл байт-кода правят руками чаще, чем кажется.
    At(usize, String),
    /// Заголовок отсутствует или несёт чужой номер формата.
    BadHeader(String),
    /// Манифест компонентов пуст, неупорядочен или содержит дубликаты.
    InvalidRequirements(String),
    /// Значение, которое не переживает печать (объект в константах).
    Unrepresentable(&'static str),
    /// Цель перехода вне чанка: номер чанка, `pc` инструкции и сама цель.
    BadJumpTarget {
        chunk: usize,
        pc: usize,
        target: i16,
    },
    /// Диапазон `Попытка` ссылается за пределы чанка либо вывернут наизнанку.
    BadExceptionRange { chunk: usize, what: &'static str },
    /// `Call` ссылается не на функцию: номер чанка, `pc` инструкции и сам
    /// номер. Нумерация в `Call` начинается с единицы, потому что
    /// `function_names[i]` — это `chunks[i+1]`, а нулевой чанк (верхний
    /// уровень) не вызывает никто.
    BadCallTarget { chunk: usize, pc: usize, func: u16 },
    /// Импортный опкод ссылается мимо таблицы связей либо на запись
    /// чужого вида (функция вместо переменной или наоборот).
    BadLinkTarget {
        chunk: usize,
        pc: usize,
        link_slot: u16,
    },
}

impl std::fmt::Display for TextError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TextError::At(line, what) => write!(f, "строка {line}: {what}"),
            TextError::BadHeader(what) => write!(f, "заголовок байт-кода: {what}"),
            TextError::InvalidRequirements(what) => {
                write!(f, "требования компонентов: {what}")
            }
            TextError::BadExceptionRange { chunk, what } => {
                write!(f, "чанк {chunk}: {what}")
            }
            TextError::BadJumpTarget { chunk, pc, target } => write!(
                f,
                "чанк {chunk}, инструкция {pc}: цель перехода {target} вне чанка"
            ),
            TextError::BadCallTarget { chunk, pc, func } => write!(
                f,
                "чанк {chunk}, инструкция {pc}: вызов {func} не ссылается на функцию"
            ),
            TextError::BadLinkTarget {
                chunk,
                pc,
                link_slot,
            } => write!(
                f,
                "чанк {chunk}, инструкция {pc}: связь {link_slot} отсутствует или чужого вида"
            ),
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
///
/// # Errors
///
/// - [`TextError::InvalidRequirements`] — манифест компонентов пуст,
///   неупорядочен или содержит дубликаты.
/// - [`TextError::Unrepresentable`] — константа программы не имеет
///   текстового представления в формате байт-кода.
/// - [`TextError::BadCallTarget`] — `Call` ссылается не на функцию.
pub fn write_program(program: &Program, source: Option<&str>) -> Result<String> {
    let mut out = String::with_capacity(4096);
    writeln!(out, "bslc {FORMAT_VERSION}").unwrap();
    if let Some(src) = source {
        writeln!(out, "; исходник: {src}").unwrap();
    }
    write_program_body(&mut out, program)?;
    Ok(out)
}

/// Полный конфигурационный либо одиночный образ. Одиночная программа
/// печатается ровно как [`write_program`]; конфигурация — заголовком
/// `.configuration`, за которым идут модули каталога (`.module N "имя"`) и,
/// если есть, transient entry (`.entry id=N`). Тело каждого модуля — те же
/// секции, что у одиночной программы.
///
/// # Errors
///
/// Те же, что у [`write_program`], для каждого модуля образа.
pub fn write_image(image: &crate::BytecodeImage, source: Option<&str>) -> Result<String> {
    match image {
        crate::BytecodeImage::Program(program) => write_program(program, source),
        crate::BytecodeImage::Configuration { catalog, entry } => {
            let mut out = String::with_capacity(4096 * (catalog.modules.len() + 1));
            writeln!(out, "bslc {FORMAT_VERSION}").unwrap();
            if let Some(src) = source {
                writeln!(out, "; исходник: {src}").unwrap();
            }
            writeln!(
                out,
                "\n.configuration modules={} entry={}",
                catalog.modules.len(),
                if entry.is_some() { "yes" } else { "no" }
            )
            .unwrap();
            for (i, module) in catalog.modules.iter().enumerate() {
                writeln!(out, "\n.module {i} {}", quote(&module.name)).unwrap();
                write_program_body(&mut out, &module.program)?;
            }
            if let Some(entry) = entry {
                writeln!(out, "\n.entry id={}", entry.id.get()).unwrap();
                write_program_body(&mut out, &entry.program)?;
            }
            Ok(out)
        }
    }
}

fn write_program_body(out: &mut String, program: &Program) -> Result<()> {
    validate_requirements(&program.requirements).map_err(TextError::InvalidRequirements)?;
    let out = &mut *out;

    writeln!(out, "\n.requires {}", program.requirements.len()).unwrap();
    for (i, requirement) in program.requirements.iter().enumerate() {
        writeln!(
            out,
            "  {i} {} {}",
            quote(&requirement.package),
            quote(&requirement.version)
        )
        .unwrap();
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
        let export = export_suffix(&program.exported_module_vars, i);
        writeln!(out, "  {i} {}{export}", quote(name)).unwrap();
    }

    writeln!(out, "\n.functions {}", program.function_names.len()).unwrap();
    for (i, name) in program.function_names.iter().enumerate() {
        // `i` -> chunks[i+1]; номер чанка в комментарии, чтобы не считать
        // в уме при чтении `Call func=N`.
        let export = export_suffix(&program.exported_functions, i);
        writeln!(out, "  {i} {}{export}  ; .chunk {}", quote(name), i + 1).unwrap();
    }

    writeln!(out, "\n.links {}", program.links.len()).unwrap();
    for (i, link) in program.links.iter().enumerate() {
        match link {
            LinkEntry::Function { module, func } => {
                writeln!(out, "  {i} fn module={} func={func}", module.index()).unwrap();
            }
            LinkEntry::Variable { module, slot } => {
                writeln!(out, "  {i} var module={} slot={slot}", module.index()).unwrap();
            }
        }
    }

    for (i, chunk) in program.chunks.iter().enumerate() {
        write_chunk(out, i, chunk, program)?;
    }
    Ok(())
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
    let defaults: Vec<&str> = chunk
        .param_has_default
        .iter()
        .map(|has| if *has { "yes" } else { "no" })
        .collect();
    // `kind=proc` пишется ТОЛЬКО у процедур: у функции это умолчание, а
    // у `chunks[0]` вида объявления попросту нет — верхний уровень никто
    // не вызывает, и приписывать ему `kind=func` значило бы утверждать
    // лишнее. У нулевого чанка поле не печатается НИКОГДА, даже если флаг
    // почему-то взведён: у верхнего уровня вида объявления нет по
    // построению (`signatures` в VM отображает имена только на
    // `chunks[1..]`), поэтому текстового представления у такого состояния
    // тоже быть не должно. Разбор его отвергает — см. `parse_chunk`.
    let kind = if index != 0 && chunk.is_procedure {
        " kind=proc"
    } else {
        ""
    };
    let async_modifier = if index != 0 && chunk.is_async {
        " async=true"
    } else {
        ""
    };
    writeln!(
        out,
        "\n.chunk {index} params={} locals={} regs={} argmodes=[{}] defaults=[{}]{kind}{async_modifier}  ; {what}",
        chunk.n_params,
        chunk.n_locals,
        chunk.n_regs,
        modes.join(","),
        defaults.join(",")
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
                ArgMode::ByRefModuleVar(slot) => format!("bymodvar:{slot}"),
                ArgMode::ByRefImportedVar(slot) => format!("byimport:{slot}"),
                ArgMode::Default => "default".to_string(),
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
        // Начало многочленного бандла помечается отдельной строкой-
        // комментарием: для парсера её не существует, а при разборе
        // разметка пересчитывается в ту же (см. `parse_program`).
        if let Some(&w) = chunk.bundle_len.get(pc)
            && w >= 2
        {
            writeln!(out, "    ; бандл {w}").unwrap();
        }
        // Номер вызываемой функции проверяется ЗДЕСЬ, до печати: дальше
        // комментатор листинга индексирует таблицу функций без запасного
        // пути. Целым номер считается, только если он проходит ОБЕ таблицы,
        // которые связывает: `function_names[func - 1]` — подпись, которую
        // печатает комментатор, а `chunks[func]` — тело, которое будет
        // вызвано (VM проверяет именно его, см. `Instr::Call` в `bsl-vm`).
        // Порознь эти границы не совпадают: программу с одним именем в
        // таблице функций, но без соответствующего чанка, собрать можно, и
        // напечатанный листинг разобрался бы в программу, которую VM
        // отвергает уже на исполнении. Ноль — ссылка на верхний уровень,
        // которого не вызывает никто.
        // Той же строгости заслуживает и таблица связей: печатать инструкцию,
        // ссылающуюся мимо `links` или на запись чужого вида, значит выпустить
        // листинг, который разбор примет, а проверка образа отвергнет.
        let link_kind_ok = |slot: u16, want_function: bool| match program.links.get(slot as usize) {
            Some(LinkEntry::Function { .. }) => want_function,
            Some(LinkEntry::Variable { .. }) => !want_function,
            None => false,
        };
        match instr {
            Instr::CallImported { link_slot, .. } if !link_kind_ok(*link_slot, true) => {
                return Err(TextError::BadLinkTarget {
                    chunk: index,
                    pc,
                    link_slot: *link_slot,
                });
            }
            Instr::GetImportedVar { link_slot, .. } | Instr::SetImportedVar { link_slot, .. }
                if !link_kind_ok(*link_slot, false) =>
            {
                return Err(TextError::BadLinkTarget {
                    chunk: index,
                    pc,
                    link_slot: *link_slot,
                });
            }
            _ => {}
        }
        if let Instr::Call { func, .. } = instr
            && (*func == 0
                || *func as usize > program.function_names.len()
                || *func as usize >= program.chunks.len())
        {
            return Err(TextError::BadCallTarget {
                chunk: index,
                pc,
                func: *func,
            });
        }
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
        Instr::CallObjectMethod { method, .. } => program
            .names
            .get(*method as usize)
            .map(|name| format!(".{name}")),
        Instr::GetObjectProp { name, .. } | Instr::SetObjectProp { name, .. } => program
            .names
            .get(*name as usize)
            .map(|name| format!(".{name}")),
        Instr::NewStructure { shape, .. } => program
            .shapes
            .get(*shape as usize)
            .map(|s| format!("поля: {}", field_names(&s.names, program))),
        Instr::GetModuleVar { slot, .. } | Instr::SetModuleVar { slot, .. } => program
            .module_vars
            .get(*slot as usize)
            .map(|n| format!("модульная {n}")),
        // Номер уже проверен в `write_chunk`, поэтому индексация прямая:
        // решение о том, что считать целым номером функции, принимается в
        // одном месте, а не размазывается по запасным веткам печати.
        Instr::Call { func, .. } => Some(format!(
            "-> {} (.chunk {func})",
            program.function_names[*func as usize - 1]
        )),
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
            // Обрезанное значение печатается ЧЕРЕЗ `{:?}`, как и целое:
            // комментарий обязан уместиться в ОДНУ строку, а перевод строки
            // внутри константы разрывал её и ломал обратный разбор
            // листинга (`--run-bytecode`) — комментарии парсер отбрасывает
            // построчно.
            let shown: String = s.chars().take(40).collect();
            if shown.chars().count() < s.chars().count() {
                format!("{shown:?}…")
            } else {
                format!("{s:?}")
            }
        }
        other => other.to_string(),
    }
}

/// Представимая в текстовом формате константа.
///
/// Существует ради того, чтобы классификация вариантов `BslValue` была
/// в проекте ОДНА. Прежде их было две: печать разбирала `BslValue`
/// исчерпывающе, а проверяемое преобразование константы отвергало два
/// варианта и принимало остальные ветвью-заглушкой. Новый вариант
/// `BslValue` заставил бы обновить печать и НЕ заставил бы обновить
/// преобразование — и, объяви печать его непредставимым, преобразование
/// молча приняло бы его, вернув ровно тот дефект, ради которого
/// проверяемое преобразование и заводилось.
pub(crate) enum ConstKind<'a> {
    Undefined,
    Null,
    Boolean(bool),
    Number(&'a BslNumber),
    Str(&'a BslString),
    Date(&'a BslDate),
    Enum(&'a bsl_rt::EnumValue),
    EnumType(&'a bsl_rt::EnumKind),
}

/// Единственная классификация представимости значения как константы
/// байт-кода.
///
/// Разбор ИСЧЕРПЫВАЮЩИЙ, без ветви-заглушки: новый вариант `BslValue`
/// обязан быть ошибкой сборки здесь, а не молча попасть в
/// «представимо». Ошибка несёт то же слово, которое видит пользователь в
/// `TextError::Unrepresentable`.
pub(crate) fn classify_const(v: &BslValue) -> std::result::Result<ConstKind<'_>, &'static str> {
    match v {
        BslValue::Undefined => Ok(ConstKind::Undefined),
        BslValue::Null => Ok(ConstKind::Null),
        BslValue::Boolean(b) => Ok(ConstKind::Boolean(*b)),
        BslValue::Number(n) => Ok(ConstKind::Number(n)),
        BslValue::Str(s) => Ok(ConstKind::Str(s)),
        BslValue::Date(d) => Ok(ConstKind::Date(d)),
        // Член перечисления — константа времени компиляции
        // (`ТипЗначенияJSON.ИмяСвойства` резолвится в `LoadConst`), значит
        // и в текстовом формате он обязан быть представим, иначе
        // напечатанный байт-код перестал бы исполняться.
        BslValue::Enum(e) => Ok(ConstKind::Enum(e)),
        // Голое имя перечисления — та же логика, что у члена: константа
        // времени компиляции (`RExpr::EnumTypeRef`).
        BslValue::EnumType(k) => Ok(ConstKind::EnumType(k)),
        BslValue::Type(_) => Err("Тип"),
        BslValue::Object(_) => Err("объект"),
    }
}

fn write_const(v: &BslValue) -> Result<String> {
    let kind = classify_const(v).map_err(TextError::Unrepresentable)?;
    Ok(match kind {
        ConstKind::Undefined => "Неопределено".to_string(),
        ConstKind::Null => "Null".to_string(),
        ConstKind::Boolean(b) => format!("Булево {}", if b { "Истина" } else { "Ложь" }),
        // Каноническая форма числа — та же, что у оракула замеров: точка
        // как разделитель, без группировки, без потери разрядов.
        ConstKind::Number(n) => format!("Число {}", n.to_canonical()),
        ConstKind::Str(s) => format!("Строка {}", quote(&s.to_string())),
        // Секундами от эпохи, а не `дд.ММ.гггг`: представление даты по
        // умолчанию — само по себе открытый вопрос (FMT.DATE.DEFAULT), и
        // байт-код не должен от него зависеть.
        ConstKind::Date(d) => format!("Дата {}", d.seconds()),
        ConstKind::Enum(e) => format!("Перечисление {}.{}", e.enum_name(), e.member_name()),
        // Отдельный тег, а не «Перечисление» без точки: `Перечисление X.Y`
        // уже занят членом, и без точки текст разбирался бы неоднозначно.
        ConstKind::EnumType(k) => format!("ТипПеречисления {}", k.ru_name()),
    })
}

fn write_instr(instr: &Instr) -> String {
    let op = instr.opcode();
    match instr {
        Instr::Move { dst, src } => format!("{op} dst={dst} src={src}"),
        Instr::GetModuleVar { dst, slot } => format!("{op} dst={dst} slot={slot}"),
        Instr::SetModuleVar { slot, src } => format!("{op} slot={slot} src={src}"),
        Instr::LoadConst { dst, k } => format!("{op} dst={dst} k={k}"),
        Instr::LoadBool { dst, val } => format!("{op} dst={dst} val={val}"),
        Instr::LoadUndefined { dst } => format!("{op} dst={dst}"),
        Instr::LoadNull { dst } => format!("{op} dst={dst}"),
        Instr::Add { dst, a, b } => format!("{op} dst={dst} a={a} b={b}"),
        Instr::AddConst { dst, src, k } => format!("{op} dst={dst} src={src} k={k}"),
        Instr::Sub { dst, a, b } => format!("{op} dst={dst} a={a} b={b}"),
        Instr::Mul { dst, a, b } => format!("{op} dst={dst} a={a} b={b}"),
        Instr::Div { dst, a, b } => format!("{op} dst={dst} a={a} b={b}"),
        Instr::Mod { dst, a, b } => format!("{op} dst={dst} a={a} b={b}"),
        Instr::Neg { dst, src } => format!("{op} dst={dst} src={src}"),
        Instr::Not { dst, src } => format!("{op} dst={dst} src={src}"),
        Instr::Eq { dst, a, b } => format!("{op} dst={dst} a={a} b={b}"),
        Instr::NotEq { dst, a, b } => format!("{op} dst={dst} a={a} b={b}"),
        Instr::Lt { dst, a, b } => format!("{op} dst={dst} a={a} b={b}"),
        Instr::Gt { dst, a, b } => format!("{op} dst={dst} a={a} b={b}"),
        Instr::Le { dst, a, b } => format!("{op} dst={dst} a={a} b={b}"),
        Instr::Ge { dst, a, b } => format!("{op} dst={dst} a={a} b={b}"),
        Instr::Jump { target } => format!("{op} target={target}"),
        Instr::JumpIfFalse { cond, target } => format!("{op} cond={cond} target={target}"),
        Instr::JumpIfTrue { cond, target } => format!("{op} cond={cond} target={target}"),
        Instr::JumpIfNotEqConst { src, k, target } => {
            format!("{op} src={src} k={k} target={target}")
        }
        Instr::JumpIfNotLtConst { src, k, target } => {
            format!("{op} src={src} k={k} target={target}")
        }
        Instr::JumpIfNotSkipped { src, target } => {
            format!("{op} src={src} target={target}")
        }
        Instr::NumericForNext {
            counter,
            bound,
            target,
        } => format!("{op} counter={counter} bound={bound} target={target}"),
        Instr::NumericForNextI64 {
            counter,
            bound,
            target,
        } => format!("{op} counter={counter} bound={bound} target={target}"),
        Instr::Call {
            func,
            base,
            arg_modes,
            ret,
        } => format!("{op} func={func} base={base} arg_modes={arg_modes} ret={ret}"),
        Instr::CallImported {
            link_slot,
            base,
            arg_modes,
            ret,
        } => format!("{op} link={link_slot} base={base} arg_modes={arg_modes} ret={ret}"),
        Instr::GetImportedVar { dst, link_slot } => format!("{op} dst={dst} link={link_slot}"),
        Instr::SetImportedVar { link_slot, src } => format!("{op} link={link_slot} src={src}"),
        Instr::Await { dst, promise } => format!("{op} dst={dst} promise={promise}"),
        Instr::Return { src } => match src {
            Some(src) => format!("{op} src={src}"),
            None => op.to_string(),
        },
        Instr::GetIndex { dst, obj, idx } => format!("{op} dst={dst} obj={obj} idx={idx}"),
        Instr::SetIndex { obj, idx, src } => format!("{op} obj={obj} idx={idx} src={src}"),
        Instr::GetProp { dst, obj, name } => {
            format!("{op} dst={dst} obj={obj} name={}", name.index())
        }
        Instr::SetProp { obj, name, src } => {
            format!("{op} obj={obj} name={} src={src}", name.index())
        }
        Instr::CreateObject {
            dst,
            library,
            constructor,
            base,
            count,
        } => format!("{op} dst={dst} lib={library} ctor={constructor} base={base} count={count}"),
        Instr::NewArray { dst, base, count } => {
            format!("{op} dst={dst} base={base} count={count}")
        }
        Instr::NewStructure {
            dst,
            shape,
            base,
            count,
        } => format!("{op} dst={dst} shape={shape} base={base} count={count}"),
        Instr::NewTable { dst } => format!("{op} dst={dst}"),
        Instr::NewTypeDescription { dst, names } => {
            format!("{op} dst={dst} names={names}")
        }
        Instr::NewValueComparison { dst } => format!("{op} dst={dst}"),
        Instr::NewMap { dst } => format!("{op} dst={dst}"),
        Instr::NewTextWriter { dst, path } => format!("{op} dst={dst} path={path}"),
        Instr::CollectionLen { dst, obj } => format!("{op} dst={dst} obj={obj}"),
        Instr::Raise { src } => match src {
            Some(src) => format!("{op} src={src}"),
            None => op.to_string(),
        },
        Instr::CallBuiltin {
            dst,
            builtin,
            base,
            count,
        } => format!(
            "{op} dst={dst} builtin={} base={base} count={count}",
            builtin_name(*builtin)
        ),
        Instr::CallComponent {
            dst,
            library,
            function,
            base,
            count,
        } => {
            format!("{op} dst={dst} lib={library} fn={function} base={base} count={count}")
        }
        Instr::CallMethod {
            dst,
            obj,
            method,
            base,
            count,
        } => format!(
            "{op} dst={dst} obj={obj} method={} base={base} count={count}",
            builtin_method_name(*method)
        ),
        Instr::CallObjectMethod {
            dst,
            obj,
            method,
            base,
            count,
        } => format!(
            "{op} dst={dst} obj={obj} method={} base={base} count={count}",
            method
        ),
        Instr::GetObjectProp { dst, obj, name } => {
            format!("{op} dst={dst} obj={obj} name={name}")
        }
        Instr::SetObjectProp { obj, name, src } => {
            format!("{op} obj={obj} name={name} src={src}")
        }
        Instr::RunDynamic { src, dst, is_eval } => {
            format!("{op} src={src} dst={dst} is_eval={is_eval}")
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

fn builtin_method_name(method: bsl_rt::BuiltinMethod) -> &'static str {
    bsl_rt::BUILTIN_METHOD_NAMES
        .iter()
        .find_map(|(name, candidate)| (*candidate == method).then_some(*name))
        .expect("каждый BuiltinMethod обязан иметь имя в единой таблице")
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
    ///
    /// Счётчик проверяется по числу ОСТАВШИХСЯ строк: каждая запись секции
    /// занимает хотя бы одну, поэтому больше их быть не может. Без этой
    /// границы число из листинга уходило прямо в `Vec::with_capacity`, и
    /// `.handlers 18446744073709551615` валил процесс «capacity overflow»
    /// с кодом 101 вместо `TextError` — одинаково в debug и в release.
    /// Листинг правят руками, доверия ему столько же, сколько целям
    /// переходов.
    fn directive(&mut self, name: &str) -> Result<usize> {
        let (no, text) = self.expect(name)?;
        let mut parts = text.split_whitespace();
        let n = match (parts.next(), parts.next()) {
            (Some(d), Some(n)) if d == name => n
                .parse::<usize>()
                .map_err(|_| TextError::At(no, format!("{name}: «{n}» не число")))?,
            _ => return Err(TextError::At(no, format!("ожидалась директива {name} N"))),
        };
        let left = self.lines.len() - self.pos;
        if n > left {
            return Err(TextError::At(
                no,
                format!("{name}: записей {n}, а строк осталось {left}"),
            ));
        }
        Ok(n)
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

/// Разбирает текстовое представление программы байт-кода.
///
/// # Errors
///
/// Возвращает [`TextError`], если заголовок, секция, опкод, операнд или индекс не соответствует
/// текстовому формату.
pub fn parse_program(src: &str) -> Result<Program> {
    let mut r = reader_after_header(src)?;
    if peek_directive(&r).is_some_and(|d| d == ".configuration") {
        return Err(TextError::BadHeader(
            "это конфигурационный образ: его читает parse_image".to_string(),
        ));
    }
    parse_program_body(&mut r)
}

/// Разбирает одиночный либо конфигурационный образ — обратная сторона
/// [`write_image`]. Гарантия побайтового round-trip относится к паре
/// `write_image`/`parse_image` так же, как к `write_program`/`parse_program`.
///
/// # Errors
///
/// Те же, что у [`parse_program`], плюс ошибки структуры конфигурации
/// (`.module` вне `.configuration`, число модулей, отсутствующий entry).
pub fn parse_image(src: &str) -> Result<crate::BytecodeImage> {
    let mut r = reader_after_header(src)?;
    if !peek_directive(&r).is_some_and(|d| d == ".configuration") {
        return parse_program_body(&mut r).map(crate::BytecodeImage::Program);
    }
    let (no, text) = r.expect(".configuration")?;
    let mut parts = text.split_whitespace();
    let _ = parts.next();
    let field = |token: Option<&str>, name: &str| -> Result<String> {
        token
            .and_then(|t| t.strip_prefix(name))
            .and_then(|t| t.strip_prefix('='))
            .map(str::to_string)
            .ok_or_else(|| TextError::At(no, format!("ожидалось {name}= в .configuration")))
    };
    let module_count: usize = field(parts.next(), "modules")?
        .parse()
        .map_err(|_| TextError::At(no, "modules= не число".to_string()))?;
    let has_entry = match field(parts.next(), "entry")?.as_str() {
        "yes" => true,
        "no" => false,
        other => {
            return Err(TextError::At(
                no,
                format!("entry= ожидает yes или no, получено «{other}»"),
            ));
        }
    };

    let mut modules = Vec::with_capacity(module_count.min(r.lines.len()));
    for i in 0..module_count {
        let (no, text) = r.expect(".module")?;
        let rest = text
            .strip_prefix(".module")
            .ok_or_else(|| TextError::At(no, format!("ожидался .module, получено «{text}»")))?;
        let (idx, rest) = rest
            .trim()
            .split_once(char::is_whitespace)
            .ok_or_else(|| TextError::At(no, "ожидалось «.module N \"имя\"»".to_string()))?;
        parse_index(no, idx, i)?;
        let (name, tail) = unquote(no, rest.trim())?;
        if !tail.trim().is_empty() {
            return Err(TextError::At(
                no,
                format!("лишнее после имени модуля: «{tail}»"),
            ));
        }
        let program = parse_program_body(&mut r)?;
        modules.push(crate::ModuleProgram { name, program });
    }
    let catalog = crate::ConfigurationProgram { modules };

    let entry = if has_entry {
        let (no, text) = r.expect(".entry")?;
        let rest = text
            .strip_prefix(".entry")
            .ok_or_else(|| TextError::At(no, format!("ожидался .entry, получено «{text}»")))?;
        let id: u64 = rest
            .trim()
            .strip_prefix("id=")
            .ok_or_else(|| TextError::At(no, "ожидалось «.entry id=N»".to_string()))?
            .parse()
            .map_err(|_| TextError::At(no, "id entry не число".to_string()))?;
        let program = parse_program_body(&mut r)?;
        Some(crate::EntryProgram {
            id: crate::EntryId::new(id),
            program,
        })
    } else {
        None
    };
    if r.pos < r.lines.len() {
        let line = &r.lines[r.pos];
        return Err(TextError::At(
            line.no,
            format!("лишнее после конца образа: «{}»", line.text),
        ));
    }
    Ok(crate::BytecodeImage::Configuration { catalog, entry })
}

/// Общий заголовок обеих форм: `bslc N` с точной сверкой версии.
fn reader_after_header(src: &str) -> Result<Reader> {
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
            )));
        }
        _ => {
            return Err(TextError::BadHeader(format!(
                "строка {no}: ожидалось «bslc {FORMAT_VERSION}»"
            )));
        }
    }
    Ok(r)
}

/// Первая директива впереди, если она есть, — без продвижения позиции.
fn peek_directive(r: &Reader) -> Option<&str> {
    r.lines
        .get(r.pos)
        .map(|line| line.text.split_whitespace().next().unwrap_or(""))
}

/// Начинается ли строка с граничной директивы конфигурации: тело модуля
/// заканчивается там, где начинается следующий модуль либо entry.
fn at_image_boundary(r: &Reader) -> bool {
    matches!(peek_directive(r), Some(".module") | Some(".entry"))
}

fn parse_program_body(r: &mut Reader) -> Result<Program> {
    // Точные версии runtime-компонентов. Индекс строки используется
    // инструкциями как локальный номер библиотеки, поэтому порядок
    // проверяется так же строго, как индексы имён и форм.
    let n = r.directive(".requires")?;
    let mut requirements = Vec::with_capacity(n);
    for i in 0..n {
        let (no, text) = r.expect("требование компонента")?;
        let (idx, rest) = text
            .split_once(char::is_whitespace)
            .ok_or_else(|| TextError::At(no, "ожидалось «N \"пакет\" \"версия\"»".to_string()))?;
        parse_index(no, idx, i)?;
        let (package, rest) = unquote(no, rest.trim())?;
        let (version, tail) = unquote(no, rest.trim())?;
        if !tail.is_empty() {
            return Err(TextError::At(
                no,
                format!("лишнее после версии компонента: «{tail}»"),
            ));
        }
        requirements.push(LibraryRequirement::new(package, version));
    }
    validate_requirements(&requirements).map_err(TextError::InvalidRequirements)?;

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
    let mut exported_module_vars = Vec::with_capacity(n);
    for i in 0..n {
        let (no, text) = r.expect("имя переменной модуля")?;
        let (idx, rest) = text
            .split_once(char::is_whitespace)
            .ok_or_else(|| TextError::At(no, "ожидалось «N \"имя\"»".to_string()))?;
        parse_index(no, idx, i)?;
        let (name, tail) = unquote(no, rest.trim())?;
        module_vars.push(name);
        exported_module_vars.push(parse_export_flag(no, &tail)?);
    }

    let n = r.directive(".functions")?;
    let mut function_names = Vec::with_capacity(n);
    let mut exported_functions = Vec::with_capacity(n);
    for i in 0..n {
        let (no, text) = r.expect("имя функции")?;
        let (idx, rest) = text
            .split_once(char::is_whitespace)
            .ok_or_else(|| TextError::At(no, "ожидалось «N \"имя\"»".to_string()))?;
        parse_index(no, idx, i)?;
        let (name, tail) = unquote(no, rest.trim())?;
        function_names.push(name);
        exported_functions.push(parse_export_flag(no, &tail)?);
    }

    let n = r.directive(".links")?;
    let mut links = Vec::with_capacity(n);
    for i in 0..n {
        let (no, text) = r.expect("запись таблицы связей")?;
        let mut parts = text.split_whitespace();
        let idx = parts
            .next()
            .ok_or_else(|| TextError::At(no, "ожидалось «N fn|var ...»".to_string()))?;
        parse_index(no, idx, i)?;
        let kind = parts
            .next()
            .ok_or_else(|| TextError::At(no, "ожидался вид связи fn|var".to_string()))?;
        let fields: Vec<&str> = parts.collect();
        let field = |name: &str| -> Result<u32> {
            fields
                .iter()
                .find_map(|f| f.strip_prefix(name).and_then(|f| f.strip_prefix('=')))
                .ok_or_else(|| TextError::At(no, format!("нет поля {name}=")))?
                .parse::<u32>()
                .map_err(|_| TextError::At(no, format!("поле {name} не число")))
        };
        let module = ModuleId::new(field("module")?);
        let narrow = |value: u32, what: &str| -> Result<u16> {
            u16::try_from(value).map_err(|_| TextError::At(no, format!("{what} шире u16")))
        };
        links.push(match kind {
            "fn" => LinkEntry::Function {
                module,
                func: narrow(field("func")?, "func")?,
            },
            "var" => LinkEntry::Variable {
                module,
                slot: narrow(field("slot")?, "slot")?,
            },
            other => {
                return Err(TextError::At(
                    no,
                    format!("неизвестный вид связи «{other}»"),
                ));
            }
        });
    }

    // Чанки — до конца тела: конец файла либо граница следующего модуля
    // конфигурации.
    let mut chunks = Vec::new();
    while r.pos < r.lines.len() && !at_image_boundary(r) {
        chunks.push(parse_chunk(r, chunks.len())?);
    }
    if chunks.is_empty() {
        return Err(TextError::At(0, "нет ни одного .chunk".to_string()));
    }
    // Цели переходов из файла доверия не заслуживают ровно так же, как
    // разметка бандлов. `pc` за концом чанка VM принимает за нормальное
    // завершение, поэтому битая цель дала бы не диагностику, а МОЛЧА
    // неверный ответ: программа закончилась бы без вывода и с нулевым кодом
    // возврата. Проверка стоит один проход здесь, а не по ветвлению на
    // каждом переходе, — горячий цикл её не замечает.
    for (i, chunk) in chunks.iter().enumerate() {
        let limit = chunk.instrs.len();
        // Обработчик `Попытка` — тоже цель передачи управления, только со
        // стороны разматывания, и доверия ему ровно столько же. С целью за
        // концом чанка VM уводила `pc` туда же, принимала это за нормальное
        // завершение — и программа с `ВызватьИсключение` заканчивалась без
        // вывода и с нулевым кодом, молча проглотив исключение.
        for range in &chunk.exception_ranges {
            let what = if range.start_pc > range.end_pc {
                Some("начало диапазона «Попытка» больше конца")
            } else if range.end_pc > limit {
                Some("конец диапазона «Попытка» за концом чанка")
            } else if range.handler_pc > limit {
                Some("обработчик «Попытка» за концом чанка")
            } else {
                None
            };
            if let Some(what) = what {
                return Err(TextError::BadExceptionRange { chunk: i, what });
            }
        }
        for (pc, instr) in chunk.instrs.iter().enumerate() {
            let Some(target) = instr.jump_target() else {
                continue;
            };
            // `target == limit` законно: так выглядит выход за последнюю
            // инструкцию чанка, то есть нормальное завершение.
            if target < 0 || target as usize > limit {
                return Err(TextError::BadJumpTarget {
                    chunk: i,
                    pc,
                    target,
                });
            }
        }
    }

    let mut program = Program {
        requirements,
        chunks,
        names,
        shapes,
        top_level_locals,
        function_names,
        exported_functions,
        module_vars,
        exported_module_vars,
        links,
    };
    // Разметка бандлов — производная таблица (как `prop_cache`): из файла
    // не читается, а пересчитывается из уже разобранного. Обязана дать то
    // же, что у компилятора — это держит побайтовый round-trip вместе с
    // пометками `; бандл N` в листинге.
    crate::image::finalize(&mut program);
    Ok(program)
}

/// Хвост строки секции имён: ` export` у экспортного элемента, пусто у
/// обычного. Печать и разбор договорились об одном написании флага, чтобы
/// листинг ходил по кругу побайтово.
fn export_suffix(flags: &[bool], index: usize) -> &'static str {
    if flags.get(index).copied().unwrap_or(false) {
        " export"
    } else {
        ""
    }
}

/// Разбирает необязательный флаг `export` после имени в секциях
/// `.module-vars` и `.functions`. Комментарий (`; ...`) флагом не
/// считается; любой другой токен — ошибка формата, а не молчаливый
/// пропуск.
fn parse_export_flag(no: usize, tail: &str) -> Result<bool> {
    let tail = tail.trim();
    let payload = match tail.split_once(';') {
        Some((before, _)) => before.trim(),
        None => tail,
    };
    match payload {
        "" => Ok(false),
        "export" => Ok(true),
        other => Err(TextError::At(
            no,
            format!("после имени ожидался флаг «export» или комментарий, получено «{other}»"),
        )),
    }
}

fn validate_requirements(requirements: &[LibraryRequirement]) -> std::result::Result<(), String> {
    if requirements.is_empty() {
        return Err("нет обязательной записи bsl-rt".to_string());
    }
    if requirements.len() > u8::MAX as usize + 1 {
        return Err("локальный индекс библиотеки не помещается в u8".to_string());
    }
    let core = &requirements[0];
    if core.package != bsl_rt::PACKAGE_NAME {
        return Err(format!(
            "нулевой записью должен быть {}, получен {}",
            bsl_rt::PACKAGE_NAME,
            core.package
        ));
    }
    for requirement in requirements {
        if requirement.package.is_empty() || requirement.version.is_empty() {
            return Err("имя пакета и версия не могут быть пустыми".to_string());
        }
    }
    for pair in requirements[1..].windows(2) {
        if pair[0].package >= pair[1].package {
            return Err(format!(
                "пакеты после bsl-rt должны быть уникальны и отсортированы: {} перед {}",
                pair[0].package, pair[1].package
            ));
        }
    }
    if requirements[1..]
        .iter()
        .any(|requirement| requirement.package == bsl_rt::PACKAGE_NAME)
    {
        return Err("bsl-rt указан более одного раза".to_string());
    }
    Ok(())
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
    // `defaults=[yes no]` — есть ли у параметра значение по умолчанию.
    // Нужен фрагменту `Выполнить`: без него он не отличит опущенный
    // хвостовой необязательный аргумент от пропущенного обязательного.
    let param_has_default: Vec<bool> = match field(&fields, no, "defaults") {
        Ok(list) => {
            let inner = list
                .strip_prefix('[')
                .and_then(|t| t.strip_suffix(']'))
                .ok_or_else(|| TextError::At(no, format!("ожидался список: «{list}»")))?;
            inner
                .split(',')
                .filter(|t| !t.is_empty())
                .map(|t| match t {
                    "yes" => Ok(true),
                    "no" => Ok(false),
                    other => Err(TextError::At(no, format!("признак умолчания «{other}»"))),
                })
                .collect::<Result<_>>()?
        }
        Err(_) => Vec::new(),
    };
    // `kind=proc` у процедуры, отсутствие поля — функция либо верхний
    // уровень (см. печать выше). `kind=func` разбирается тоже: листинг
    // бывает и рукописным, и явное «функция» в нём законно. У `.chunk 0`
    // поля не бывает ни в каком виде — у верхнего уровня нет объявления,
    // и рукописный листинг не должен уметь завести это состояние.
    let is_procedure = match field(&fields, no, "kind") {
        Ok(_) if expected_index == 0 => {
            return Err(TextError::At(
                no,
                "у верхнего уровня нет вида объявления".to_string(),
            ));
        }
        Ok("proc") => true,
        Ok("func") => false,
        Ok(other) => return Err(TextError::At(no, format!("вид объявления «{other}»"))),
        Err(_) => false,
    };
    let is_async = match field(&fields, no, "async") {
        Ok(_) if expected_index == 0 => {
            return Err(TextError::At(
                no,
                "верхний уровень не может быть асинхронным методом".to_string(),
            ));
        }
        Ok(value) => match value {
            "true" => true,
            "false" => false,
            other => {
                return Err(TextError::At(
                    no,
                    format!("признак асинхронного метода «{other}»"),
                ));
            }
        },
        Err(_) => false,
    };

    let n = r.directive(".consts")?;
    let mut consts = Vec::with_capacity(n);
    for i in 0..n {
        let (no, text) = r.expect("константу")?;
        let (idx, rest) = text
            .split_once(char::is_whitespace)
            .ok_or_else(|| TextError::At(no, "ожидалось «N значение»".to_string()))?;
        parse_index(no, idx, i)?;
        // Разбор строит константы из ТЕКСТА, то есть по построению
        // получает только представимые: непредставимого тега в формате
        // просто нет. Проверяемое преобразование здесь — тавтология, и
        // отказ его недостижим; но второго входа в таблицу констát быть
        // не должно.
        let value = parse_const(no, rest.trim())?;
        let value = crate::BytecodeConst::new(value)
            .map_err(|_| TextError::At(no, "константа непредставима".to_string()))?;
        consts.push(value);
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
                "default" => ArgMode::Default,
                t if t.starts_with("bymodvar:") => ArgMode::ByRefModuleVar(
                    t["bymodvar:".len()..]
                        .parse()
                        .map_err(|_| TextError::At(no, format!("«{t}» не слот модуля")))?,
                ),
                t if t.starts_with("byimport:") => ArgMode::ByRefImportedVar(
                    t["byimport:".len()..]
                        .parse()
                        .map_err(|_| TextError::At(no, format!("«{t}» не номер связи")))?,
                ),
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

    // Производные таблицы — `touches_objects`, оба инлайн-кэша и
    // разметка — здесь не заполняются: их ставит `image::finalize` в
    // конце `parse_program`. Единственный писатель на весь крейт.
    Ok(Chunk {
        touches_objects: false,
        param_has_default,
        is_procedure,
        is_async,
        param_by_val,
        prop_cache: Vec::new(),
        method_cache: Vec::new(),
        instrs,
        consts,
        call_arg_modes,
        exception_ranges,
        n_params,
        n_locals,
        n_regs,
        local_names,
        // Пересчитывается в `parse_program`: разметке из файла VM не
        // верит, единственный производитель — `bundle::compute`.
        bundle_len: Vec::new(),
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
        "Перечисление" => {
            let (enum_name, member) = rest
                .split_once('.')
                .ok_or_else(|| TextError::At(no, format!("«{rest}» не вида Перечисление.Член")))?;
            let kind = bsl_rt::lookup_enum(enum_name)
                .ok_or_else(|| TextError::At(no, format!("нет перечисления «{enum_name}»")))?;
            let value = bsl_rt::lookup_member(kind, member).ok_or_else(|| {
                TextError::At(no, format!("нет члена «{member}» у «{enum_name}»"))
            })?;
            BslValue::Enum(value)
        }
        "ТипПеречисления" => {
            let kind = bsl_rt::lookup_enum(rest)
                .ok_or_else(|| TextError::At(no, format!("нет перечисления «{rest}»")))?;
            BslValue::EnumType(kind)
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
        "Add" => Instr::Add {
            dst: dst(&f)?,
            a: a(&f)?,
            b: b(&f)?,
        },
        "AddConst" => Instr::AddConst {
            dst: dst(&f)?,
            src: src(&f)?,
            k: field_u16(&f, no, "k")?,
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
        "Mod" => Instr::Mod {
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
        "JumpIfNotEqConst" => Instr::JumpIfNotEqConst {
            src: src(&f)?,
            k: field_u16(&f, no, "k")?,
            target: target(&f)?,
        },
        "JumpIfNotLtConst" => Instr::JumpIfNotLtConst {
            src: src(&f)?,
            k: field_u16(&f, no, "k")?,
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
        "CallImported" => Instr::CallImported {
            link_slot: field_u16(&f, no, "link")?,
            base: base(&f)?,
            arg_modes: field_u16(&f, no, "arg_modes")?,
            ret: field_u8(&f, no, "ret")?,
        },
        "GetImportedVar" => Instr::GetImportedVar {
            dst: dst(&f)?,
            link_slot: field_u16(&f, no, "link")?,
        },
        "SetImportedVar" => Instr::SetImportedVar {
            link_slot: field_u16(&f, no, "link")?,
            src: src(&f)?,
        },
        "Await" => Instr::Await {
            dst: dst(&f)?,
            promise: field_u8(&f, no, "promise")?,
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
        "CreateObject" => Instr::CreateObject {
            dst: dst(&f)?,
            library: field_u8(&f, no, "lib")?,
            constructor: field_u16(&f, no, "ctor")?,
            base: base(&f)?,
            count: count(&f)?,
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
        "NewTypeDescription" => Instr::NewTypeDescription {
            dst: dst(&f)?,
            names: field_u8(&f, no, "names")?,
        },
        "NewValueComparison" => Instr::NewValueComparison { dst: dst(&f)? },
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
        "CallComponent" => Instr::CallComponent {
            dst: dst(&f)?,
            library: field_u8(&f, no, "lib")?,
            function: field_u16(&f, no, "fn")?,
            base: base(&f)?,
            count: count(&f)?,
        },
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
        "CallObjectMethod" => Instr::CallObjectMethod {
            dst: dst(&f)?,
            obj: obj(&f)?,
            method: field_u16(&f, no, "method")?,
            base: base(&f)?,
            count: count(&f)?,
        },
        "GetObjectProp" => Instr::GetObjectProp {
            dst: dst(&f)?,
            obj: obj(&f)?,
            name: field_u16(&f, no, "name")?,
        },
        "SetObjectProp" => Instr::SetObjectProp {
            obj: obj(&f)?,
            name: field_u16(&f, no, "name")?,
            src: src(&f)?,
        },
        "RunDynamic" => Instr::RunDynamic {
            src: src(&f)?,
            dst: dst(&f)?,
            is_eval: field_bool(&f, no, "is_eval")?,
        },
        other => return Err(TextError::At(no, format!("неизвестный опкод «{other}»"))),
    })
}
