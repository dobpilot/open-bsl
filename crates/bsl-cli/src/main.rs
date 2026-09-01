//! Точка входа CLI. Разбор аргументов идёт ЧЕРЕЗ ТАБЛИЦУ [`COMMANDS`], а
//! не через `match` по строковым литералам: та же таблица печатается в
//! `--help`, поэтому команда не может быть реализована и не описана (её
//! просто не найдёт разбор) или описана и не реализована (`match` по
//! `Kind` исчерпывающий).
//!
//! Формы вызова описаны в самой таблице; здесь — только то, чего в ней не
//! видно: без аргументов запускается REPL (модуль `repl` — подсветка,
//! дополнение по Tab), а аргумент, не похожий на флаг, считается путём к
//! скрипту и исполняется целиком.

mod api_reference;
mod bytecode;
mod complete;
mod dap;
mod highlight;
mod ingest;
mod repl;
mod usemod;

use bsl_rt::BslValue;

/// Что делает команда. Отдельно от флага, потому что у команды бывает
/// сокращение (`--help`/`-h`), а поведение одно.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Help,
    EmitBytecode,
    EmitApiReference,
    RunBytecode,
    IngestMeasurements,
    Jit,
}

struct Command {
    flag: &'static str,
    /// Сокращённая форма, если есть.
    alias: Option<&'static str>,
    kind: Kind,
    /// Аргументы для строки использования; `[в квадратных]` — необязательные.
    args: &'static str,
    /// Одна строка о том, что команда делает.
    what: &'static str,
    /// Подробности под списком — то, чего в одну строку не сказать.
    details: &'static [&'static str],
}

const COMMANDS: &[Command] = &[
    Command {
        flag: "--emit-api-reference",
        alias: None,
        kind: Kind::EmitApiReference,
        args: "[выход.md]",
        what: "напечатать справочник доступного BSL API",
        details: &[
            "Без файла-выхода печатает Markdown в stdout. Имена, псевдонимы и",
            "арности берутся из runtime-дескрипторов стандартной сборки open-bsl.",
        ],
    },
    Command {
        flag: "--emit-bytecode",
        alias: None,
        kind: Kind::EmitBytecode,
        args: "<файл.bsl> [выход.bslc]",
        what: "напечатать байт-код вместо исполнения",
        details: &[
            "Без файла-выхода печатает в stdout. Формат печати и формат исполнения —",
            "один и тот же: напечатанное можно прочитать, поправить руками и запустить",
            "через --run-bytecode. Всё от `;` до конца строки — комментарий.",
        ],
    },
    Command {
        flag: "--run-bytecode",
        alias: None,
        kind: Kind::RunBytecode,
        args: "<файл.bslc> [аргументы...]",
        what: "исполнить напечатанный байт-код",
        details: &[
            "Разобранная программа исполняется тем же путём, что и только что",
            "скомпилированная. Номер формата в заголовке сверяется, чужой отвергается.",
        ],
    },
    Command {
        flag: "--ingest-measurements",
        alias: None,
        kind: Kind::IngestMeasurements,
        args: "<вывод-платформы> [measure-all.bsl]",
        what: "принять результат сеанса замеров у реальной 1С",
        details: &[
            "Раскладывает вывод в tests/conformance/measure/platform.tsv, сравнивает с",
            "нашим на том же скрипте и печатает расхождения. В КОДЕ НЕ ПРАВИТ НИЧЕГО:",
            "решение по каждому расхождению принимает человек.",
        ],
    },
    Command {
        flag: "--jit",
        alias: None,
        kind: Kind::Jit,
        args: "<файл.bsl> [аргументы...]",
        what: "исполнить скрипт, компилируя байт-код в машинный код",
        details: &[
            "Только x86-64 Linux; на других платформах ключ принимается и ничего не",
            "меняет. Компилируются не все инструкции: чего JIT не умеет (вызовы,",
            "возвраты, Выполнить, работа с объектами) — исполняет интерпретатор, и",
            "переключение туда-обратно происходит само. Семантика у обоих режимов",
            "ОДНА: нативный код зовёт те же функции, что и ветки интерпретатора,",
            "и это проверяется прогоном всего корпуса фикстур обоими путями.",
        ],
    },
    Command {
        flag: "--help",
        alias: Some("-h"),
        kind: Kind::Help,
        args: "",
        what: "эта подсказка",
        details: &[],
    },
];

/// Текст `--help`. Собирается из таблицы команд, поэтому описать команду и
/// забыть — нельзя.
fn help() -> String {
    let mut out = format!(
        "bsl-cli {} — интерпретатор BSL (встроенный язык 1С:Предприятие)\n\n",
        env!("CARGO_PKG_VERSION")
    );
    out.push_str("ИСПОЛЬЗОВАНИЕ:\n");
    // Форма вызова строкой, описание — под ней с отступом. Ровные колонки
    // тут не выходят: `--ingest-measurements <вывод-платформы>` длиннее
    // половины терминала, и описание уехало бы к правому краю.
    let mut entry = |call: &str, what: &str| {
        out.push_str(&format!(
            "  {}\n      {what}\n",
            format!("bsl-cli {call}").trim_end()
        ));
    };
    entry("", "REPL: подсветка, дополнение по Tab, история");
    entry(
        "<файл.bsl> [аргументы...]",
        "исполнить скрипт целиком; аргументы — в массиве АргументыКоманднойСтроки",
    );
    for c in COMMANDS {
        let flag = match c.alias {
            Some(a) => format!("{}, {a}", c.flag),
            None => c.flag.to_string(),
        };
        let call = if c.args.is_empty() {
            flag
        } else {
            format!("{flag} {}", c.args)
        };
        entry(&call, c.what);
    }

    out.push_str("\nПОДРОБНЕЕ:\n");
    for c in COMMANDS.iter().filter(|c| !c.details.is_empty()) {
        out.push_str(&format!("  {}\n", c.flag));
        for line in c.details {
            out.push_str(&format!("    {line}\n"));
        }
    }

    // Не через `entry`: замыкание держит `out` заимствованным, а здесь
    // нужны и строка вызова, и три строки под ней. Отступы те же — два
    // пробела на форму вызова, шесть на описание.
    out.push_str("\nМОДИФИКАТОРЫ:\n");
    out.push_str("  bsl-cli --optimize[=проход,...]\n");
    out.push_str("      включить оптимизирующие проходы компилятора; без списка — все\n");
    out.push_str(&format!(
        "      проходы: {}\n",
        PASS_NAMES
            .iter()
            .map(|(n, _)| *n)
            .collect::<Vec<_>>()
            .join(", ")
    ));
    out.push_str(
        "      ни один не проходил ворота допуска, см. docs/research/performance/ssa-hotspot-analysis.md\n",
    );
    // Умолчания печатаются ИЗ ТЕХ ЖЕ констант, что читает разбор ключей:
    // подсказка, называющая другой порт, хуже отсутствующей.
    out.push_str("  bsl-cli --debug [--debug-host АДРЕС] [--debug-port ПОРТ]\n");
    out.push_str("      отлаживать по протоколу DAP: слушать и ждать редактор\n");
    out.push_str(&format!(
        "      по умолчанию {DEBUG_HOST_DEFAULT}:{DEBUG_PORT_DEFAULT}; адрес и порт без --debug — ошибка\n"
    ));
    out.push_str("      петля по умолчанию не случайно: отладчик вычисляет произвольный BSL,\n");
    out.push_str("      то есть открытый наружу порт равен удалённому запуску кода\n");
    out.push_str(
        "      несовместимо с --optimize=copy-elim: проход удаляет инструкции, а строку\n",
    );
    out.push_str("      из байт-кода не вывести — её можно только донести\n");

    out.push_str("\nОКРУЖЕНИЕ:\n");
    out.push_str("  NO_COLOR      отключает цвет в REPL (как и TERM=dumb)\n");
    out
}

fn main() {
    let mut args: Vec<String> = std::env::args().collect();
    // `--optimize` — модификатор, а не команда: он снимается из аргументов
    // до разбора, поэтому одинаково работает и перед именем скрипта, и
    // рядом с `--jit`. Скрипту он не достаётся.
    if let Some(i) = args
        .iter()
        .position(|a| a == "--optimize" || a.starts_with("--optimize="))
    {
        let arg = args.remove(i);
        let mask = match arg.split_once('=') {
            None => ALL_PASSES,
            Some((_, spec)) => match parse_passes(spec) {
                Ok(m) => m,
                Err(name) => {
                    eprintln!(
                        "неизвестный проход «{name}» в --optimize. Допустимы: {}",
                        PASS_NAMES
                            .iter()
                            .map(|(n, _)| *n)
                            .collect::<Vec<_>>()
                            .join(", ")
                    );
                    std::process::exit(2);
                }
            },
        };
        OPTIMIZE.store(mask, std::sync::atomic::Ordering::Relaxed);
    }
    match parse_debug_flags(&mut args) {
        Ok(endpoint) => {
            let _ = DEBUG_ENDPOINT.set(endpoint);
        }
        Err(message) => {
            eprintln!("{message}");
            std::process::exit(2);
        }
    }
    let code = match args.get(1).map(String::as_str) {
        None => {
            repl::run();
            0
        }
        // Всё, что начинается с дефиса, — команда, а не имя файла: молча
        // пытаться прочитать «--emitbytecode» как скрипт хуже, чем сказать,
        // что такой команды нет.
        Some(arg) if arg.starts_with('-') => {
            let Some(cmd) = COMMANDS
                .iter()
                .find(|c| c.flag == arg || c.alias == Some(arg))
            else {
                eprintln!("неизвестная команда «{arg}». Список команд: bsl-cli --help");
                std::process::exit(2);
            };
            run_command(cmd, &args)
        }
        Some(path) => {
            // Всё после имени скрипта — его собственные аргументы: скрипт
            // читает их массивом АргументыКоманднойСтроки.
            run_file(path, Engine::Interpreter, args[2..].to_vec());
            0
        }
    };
    // Сборка `--features counters` печатает гистограмму исполненных
    // опкодов в stderr: stdout занят выводом самого скрипта, а контракт
    // бенчмарка требует, чтобы последней строкой там были миллисекунды.
    #[cfg(feature = "counters")]
    eprint!("{}", bsl_vm::counters::report());
    std::process::exit(code);
}

fn run_command(cmd: &Command, args: &[String]) -> i32 {
    match cmd.kind {
        Kind::Help => {
            print!("{}", help());
            0
        }
        Kind::EmitBytecode => match args.get(2) {
            Some(path) => bytecode::emit(path, args.get(3).map(String::as_str)),
            None => missing_argument(cmd),
        },
        Kind::EmitApiReference => api_reference::emit(args.get(2).map(String::as_str)),
        Kind::RunBytecode => match args.get(2) {
            Some(path) => bytecode::run(path, args[3..].to_vec()),
            None => missing_argument(cmd),
        },
        Kind::IngestMeasurements => match args.get(2) {
            Some(input) => ingest::run(input, args.get(3).map(String::as_str)),
            None => missing_argument(cmd),
        },
        Kind::Jit => match args.get(2) {
            Some(path) => {
                run_file(path, Engine::Jit, args[3..].to_vec());
                0
            }
            None => missing_argument(cmd),
        },
    }
}

/// Обязательный аргумент не передан — показываем форму вызова ИМЕННО этой
/// команды, а не весь --help: человек уже знает, чего хочет.
fn missing_argument(cmd: &Command) -> i32 {
    eprintln!(
        "{}: не хватает аргумента\n  bsl-cli {} {}",
        cmd.flag, cmd.flag, cmd.args
    );
    2
}

/// Чем исполнять скрипт. По умолчанию — интерпретатором; JIT включается
/// только ключом, и никогда сам.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Engine {
    Interpreter,
    Jit,
}

fn run_file(path: &str, engine: Engine, arguments: Vec<String>) {
    // Отладчик ждёт редактор ДО первой инструкции: подключившийся к уже
    // доигравшей программе бесполезен.
    let debug = match debug_endpoint() {
        None => None,
        Some(addr) => match dap::session::listen(addr) {
            Ok(conn) => Some(std::rc::Rc::new(std::cell::RefCell::new(conn))),
            Err(message) => {
                eprintln!("{message}");
                std::process::exit(1);
            }
        },
    };
    let mut breakpoints = std::collections::HashSet::new();
    if let Some(shared) = debug.as_ref() {
        let mut conn = shared.borrow_mut();
        loop {
            let Some(request) = conn.wait_request() else {
                // Редактор ушёл, не досказав: исполнять нечего.
                eprintln!("отладчик: редактор отключился до запуска");
                std::process::exit(1);
            };
            match dap::session::handle_setup(&mut conn, &request) {
                dap::session::After::KeepWaiting => {}
                dap::session::After::Breakpoints(lines) => breakpoints = lines,
                dap::session::After::Run => break,
                dap::session::After::Stop => {
                    conn.flush();
                    std::process::exit(0);
                }
            }
        }
    }
    let src = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("не удалось прочитать «{path}»: {e}");
            std::process::exit(1);
        }
    };

    // Шапка с `//@используй` включает конфигурационный путь: файловый
    // граф превращается в рецепт каталога, main компилируется как entry.
    let directives = match usemod::parse_directives(&src) {
        Ok(directives) => directives,
        Err(e) => {
            eprintln!("{path}: {e}");
            std::process::exit(1);
        }
    };
    let host = if directives.is_empty() {
        match crate::engine() {
            Ok(engine) => engine,
            Err(e) => {
                eprintln!("ошибка сборки движка: {e}");
                std::process::exit(1);
            }
        }
    } else {
        let recipe = match usemod::load_graph(std::path::Path::new(path), &src) {
            Ok(recipe) => recipe,
            Err(e) => {
                eprintln!("{e}");
                std::process::exit(1);
            }
        };
        match open_bsl::Engine::builder()
            .optimizations(optimizations())
            .configuration(recipe)
            .build()
        {
            Ok(engine) => engine,
            Err(e) => {
                eprintln!("ошибка сборки конфигурации: {e}");
                std::process::exit(1);
            }
        }
    };
    let module = match host.compile_entry(&src) {
        Ok(m) => m,
        Err(open_bsl::Error::Parse(e)) => {
            eprintln!("ошибка разбора: {e}");
            std::process::exit(1);
        }
        Err(open_bsl::Error::Semantic(e)) => {
            eprintln!("ошибка резолвинга: {e}");
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("ошибка компиляции: {e}");
            std::process::exit(1);
        }
    };
    let mut state = host
        .state_builder()
        .jit(matches!(engine, Engine::Jit))
        .arguments(arguments)
        .message_sink(std::rc::Rc::new(StdoutMessageSink))
        .build();
    // Отладочный прогон идёт через `start` + собственный цикл: крючок
    // ставится на ЗАПУСК, а `run` его не принимает и принимать не должен —
    // отладка свойство запуска, а не встраивания.
    let outcome = match debug.as_ref() {
        None => state.run(&module),
        Some(shared) => {
            let hook = dap::hook::Hook::new(shared.clone(), breakpoints.clone(), false);
            match state.start(&module) {
                Err(e) => Err(e),
                Ok(mut execution) => {
                    execution.set_debug_hook(Box::new(hook));
                    loop {
                        match execution.poll(usize::MAX) {
                            Ok(open_bsl::ExecutionPoll::Complete(v)) => break Ok(v),
                            Ok(_) => continue,
                            Err(e) => break Err(e),
                        }
                    }
                }
            }
        }
    };
    // Редактору обязательно сказать, что программа кончилась: без
    // `terminated` он ждёт вечно, а без `exited` не покажет код возврата.
    // Отправляется и на успехе, и на ошибке — «доиграла» и «упала» для
    // протокола одинаково конец.
    let code = match &outcome {
        Ok(_) => 0,
        Err(_) => 1,
    };
    if let Some(shared) = debug.as_ref() {
        let mut conn = shared.borrow_mut();
        conn.event("exited", serde_json::json!({ "exitCode": code }));
        conn.event("terminated", serde_json::json!({}));
        conn.flush();
    }
    match outcome {
        Ok(BslValue::Undefined) => {}
        Ok(v) => print_value(&v),
        Err(open_bsl::Error::Runtime(e)) => {
            eprintln!("ошибка выполнения: {e}");
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("ошибка выполнения: {e}");
            std::process::exit(1);
        }
    }
}

/// Адрес и порт отладчика, если он запрошен: ставятся один раз в [`main`]
/// по ключам `--debug*` и читаются точками сборки движка.
///
/// `None` — отладка не запрошена. Ключ, как и `--optimize`, модификатор
/// обычного запуска, а не команда: он сочетается и с `--jit`, и с прямым
/// запуском файла, поэтому в таблицу `COMMANDS` не входит.
static DEBUG_ENDPOINT: std::sync::OnceLock<Option<std::net::SocketAddr>> =
    std::sync::OnceLock::new();

/// Умолчания отладчика.
///
/// Петля, а не все интерфейсы: отладчик вычисляет произвольный BSL, то
/// есть открытый наружу порт равен удалённому исполнению кода без
/// аутентификации. Сервер отладки платформы (`dbgs` из 8.3.27) слушает
/// `0.0.0.0` — это измеренное расхождение, принятое сознательно.
///
/// Порт 1550 занят: измерено, что именно его берёт `dbgs`, а
/// совместимость с платформой — цель проекта, а не повод с ней
/// столкнуться.
const DEBUG_HOST_DEFAULT: &str = "127.0.0.1";
const DEBUG_PORT_DEFAULT: u16 = 4711;

/// Выбор оптимизаций процесса: ставится один раз в [`main`] по ключу
/// `--optimize` и читается всеми точками сборки движка. Ключ — модификатор
/// обычного запуска, а не команда, поэтому в таблицу `COMMANDS` он не
/// входит: он сочетается и с `--jit`, и с `--emit-bytecode`.
static OPTIMIZE: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);

/// Имена проходов для `--optimize=список` и их биты. Единый источник:
/// разбор ключа, сообщение об ошибке и строка `--help` читают эту таблицу,
/// поэтому проход не может быть выбираемым, но не описанным.
const PASS_NAMES: &[(&str, u8)] = &[
    ("const-fold", 1),
    ("const-prop", 2),
    ("copy-elim", 4),
    ("ssa-const", 8),
    ("ssa-regalloc", 16),
];

/// Маска, которую даёт голый `--optimize`.
///
/// `ssa-regalloc` (16) в неё НЕ входит: его модель исключительных рёбер
/// неполна, и на чанке с `Попытка` он себя отключает. Проход с известной
/// дырой в модели включается поимённо — иначе голый ключ обещал бы
/// больше, чем проход умеет.
const ALL_PASSES: u8 = 1 | 2 | 4 | 8;

/// Разбирает список проходов через запятую. `Err` несёт нераспознанное имя:
/// молча игнорировать опечатку нельзя — прогон замера прочитался бы как
/// «проход включён», а на деле не включался.
fn parse_passes(spec: &str) -> Result<u8, String> {
    let mut mask = 0u8;
    for name in spec.split(',') {
        let name = name.trim();
        let (_, bit) = PASS_NAMES
            .iter()
            .find(|(n, _)| *n == name)
            .ok_or_else(|| name.to_string())?;
        mask |= bit;
    }
    Ok(mask)
}

/// Снимает `--debug`, `--debug-host` и `--debug-port` из аргументов.
///
/// Возвращает адрес, на котором слушать, либо `None`, если отладка не
/// запрошена.
///
/// # Errors
///
/// Адрес или порт БЕЗ `--debug`; отсутствующее значение у ключа; порт не
/// число; адрес, который не разбирается. Ключ, который приняли и не
/// исполнили, — дефект, а не удобство, поэтому `--debug-port` в одиночку
/// это отказ, а не молчаливый обычный запуск.
fn parse_debug_flags(args: &mut Vec<String>) -> Result<Option<std::net::SocketAddr>, String> {
    let mut on = false;
    let mut host = DEBUG_HOST_DEFAULT.to_string();
    let mut port = DEBUG_PORT_DEFAULT;
    let mut host_given = false;
    let mut port_given = false;
    let mut i = 1;
    while i < args.len() {
        let value_of = |args: &Vec<String>, i: usize, name: &str| -> Result<String, String> {
            match args[i].split_once('=') {
                Some((_, v)) if !v.is_empty() => Ok(v.to_string()),
                Some(_) => Err(format!("{name} без значения")),
                None => args
                    .get(i + 1)
                    .filter(|v| !v.starts_with('-'))
                    .cloned()
                    .ok_or_else(|| format!("{name} без значения")),
            }
        };
        let head = args[i].split('=').next().unwrap_or("").to_string();
        let taken = match head.as_str() {
            "--debug" => {
                on = true;
                1
            }
            "--debug-host" => {
                host = value_of(args, i, "--debug-host")?;
                host_given = true;
                if args[i].contains('=') { 1 } else { 2 }
            }
            "--debug-port" => {
                let raw = value_of(args, i, "--debug-port")?;
                port = raw
                    .parse::<u16>()
                    .map_err(|_| format!("--debug-port: «{raw}» не номер порта"))?;
                port_given = true;
                if args[i].contains('=') { 1 } else { 2 }
            }
            _ => {
                i += 1;
                continue;
            }
        };
        args.drain(i..i + taken);
    }
    if !on {
        if host_given || port_given {
            return Err(
                "--debug-host и --debug-port задают, ГДЕ слушать, но саму отладку включает \
                 --debug: без него ключ приняли бы и не исполнили"
                    .to_string(),
            );
        }
        return Ok(None);
    }
    let addr = format!("{host}:{port}");
    use std::net::ToSocketAddrs;
    addr.to_socket_addrs()
        .map_err(|e| format!("--debug-host/--debug-port: «{addr}» не разбирается как адрес: {e}"))?
        .next()
        .ok_or_else(|| format!("--debug-host: «{host}» не дал ни одного адреса"))
        .map(Some)
}

#[cfg(test)]
mod debug_flag_tests {
    use super::{DEBUG_PORT_DEFAULT, parse_debug_flags};

    fn args(rest: &[&str]) -> Vec<String> {
        std::iter::once("bsl-cli")
            .chain(rest.iter().copied())
            .map(str::to_string)
            .collect()
    }

    #[test]
    fn without_any_flag_debugging_is_off() {
        let mut a = args(&["скрипт.bsl"]);
        assert_eq!(parse_debug_flags(&mut a).unwrap(), None);
        assert_eq!(a, args(&["скрипт.bsl"]));
    }

    #[test]
    fn bare_debug_listens_on_the_loopback_default() {
        let mut a = args(&["--debug", "скрипт.bsl"]);
        let addr = parse_debug_flags(&mut a).unwrap().expect("адрес");
        assert_eq!(addr.ip().to_string(), "127.0.0.1");
        assert_eq!(addr.port(), DEBUG_PORT_DEFAULT);
        assert_eq!(addr.port(), 4711);
        // Ключ — модификатор: скрипту он не достаётся.
        assert_eq!(a, args(&["скрипт.bsl"]));
    }

    #[test]
    fn host_and_port_are_taken_in_both_spellings() {
        for spelling in [
            vec!["--debug", "--debug-host", "0.0.0.0", "--debug-port", "5005"],
            vec!["--debug", "--debug-host=0.0.0.0", "--debug-port=5005"],
        ] {
            let mut a = args(&spelling);
            let addr = parse_debug_flags(&mut a).unwrap().expect("адрес");
            assert_eq!(addr.to_string(), "0.0.0.0:5005", "{spelling:?}");
            assert_eq!(a, args(&[]), "{spelling:?}");
        }
    }

    #[test]
    fn an_address_without_debug_is_refused() {
        // Ключ, который приняли и не исполнили, — дефект: без `--debug`
        // отладки не будет, и молча запустить скрипт как обычно нельзя.
        for lone in [
            vec!["--debug-port", "5005", "скрипт.bsl"],
            vec!["--debug-host", "0.0.0.0", "скрипт.bsl"],
        ] {
            let mut a = args(&lone);
            assert!(parse_debug_flags(&mut a).is_err(), "{lone:?}");
        }
    }

    #[test]
    fn a_non_numeric_port_is_refused() {
        let mut a = args(&["--debug", "--debug-port", "много"]);
        assert!(parse_debug_flags(&mut a).is_err());
    }

    #[test]
    fn a_port_out_of_range_is_refused() {
        let mut a = args(&["--debug", "--debug-port", "70000"]);
        assert!(parse_debug_flags(&mut a).is_err());
    }

    #[test]
    fn a_flag_without_its_value_is_refused() {
        for lone in [
            vec!["--debug", "--debug-port"],
            vec!["--debug", "--debug-host"],
            vec!["--debug", "--debug-port="],
        ] {
            let mut a = args(&lone);
            assert!(parse_debug_flags(&mut a).is_err(), "{lone:?}");
        }
    }

    #[test]
    fn the_help_names_the_flags_and_their_actual_defaults() {
        // Подсказка, называющая не тот порт, хуже отсутствующей, поэтому
        // умолчания в ней берутся из тех же констант, что и в разборе, а
        // тест сверяет, что они туда действительно попали.
        let help = super::help();
        for flag in ["--debug", "--debug-host", "--debug-port"] {
            assert!(help.contains(flag), "в --help нет {flag}");
        }
        assert!(
            help.contains(&format!(
                "{}:{}",
                super::DEBUG_HOST_DEFAULT,
                super::DEBUG_PORT_DEFAULT
            )),
            "в --help нет умолчаний"
        );
    }

    #[test]
    fn an_unresolvable_host_is_refused() {
        let mut a = args(&["--debug", "--debug-host", "хост.которого.нет.invalid"]);
        assert!(parse_debug_flags(&mut a).is_err());
    }
}

/// Выбранные проходы компилятора.
pub fn optimizations() -> bsl_compiler::Optimizations {
    let mask = OPTIMIZE.load(std::sync::atomic::Ordering::Relaxed);
    bsl_compiler::Optimizations {
        const_fold: mask & 1 != 0,
        ssa_const: mask & 8 != 0,
        ssa_regalloc: mask & 16 != 0,
        const_prop: mask & 2 != 0,
        copy_elim: mask & 4 != 0,
    }
}

/// Движок со всеми компонентами по умолчанию — общий для запуска файла,
/// байт-кода и REPL.
pub fn engine() -> Result<open_bsl::Engine, open_bsl::Error> {
    open_bsl::Engine::builder()
        .optimizations(optimizations())
        // Со сведениями об отладке — только когда отладка запрошена: без
        // таблицы строк отладчику нечего сказать редактору, а платить за
        // неё размером образа при обычном запуске незачем.
        .debug_info(debug_endpoint().is_some())
        .build()
}

/// Адрес отладчика, если он запрошен ключами.
#[must_use]
pub fn debug_endpoint() -> Option<std::net::SocketAddr> {
    DEBUG_ENDPOINT.get().copied().flatten()
}

/// Sink сообщений CLI: очередь перед stdout процесса — короткая, потому
/// что stdout и есть представление CLI. `enqueue` не блокируется дольше
/// самой записи в поток; отказ записи отдаётся как backpressure без
/// скрытых повторов. Байты и порядок совпадают с прежним прямым путём
/// `Сообщить` -> stdout — conformance-вывод не меняется.
pub struct StdoutMessageSink;

impl bsl_rt::UserMessageSink for StdoutMessageSink {
    fn enqueue(&self, message: &bsl_rt::UserMessageDto) -> Result<(), bsl_rt::HostError> {
        use std::io::Write;
        let stdout = std::io::stdout();
        let mut lock = stdout.lock();
        writeln!(lock, "{}", message.text).map_err(|error| {
            bsl_rt::HostError::new(bsl_rt::HostErrorCode::HostBackpressure, error.to_string())
        })
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Подсказка собирается из таблицы, но проверить, что в неё попала
    /// КАЖДАЯ команда, всё равно стоит: рендер легко сломать так, что
    /// часть таблицы просто не напечатается.
    #[test]
    fn help_mentions_every_command() {
        let text = help();
        for c in COMMANDS {
            assert!(text.contains(c.flag), "в --help нет {}", c.flag);
            assert!(text.contains(c.what), "в --help нет описания {}", c.flag);
            if let Some(alias) = c.alias {
                assert!(text.contains(alias), "в --help нет сокращения {alias}");
            }
            for line in c.details {
                assert!(text.contains(line), "в --help нет подробностей {}", c.flag);
            }
        }
        // Формы без флага — REPL и запуск скрипта — тоже описаны.
        assert!(text.contains("<файл.bsl>"));
        assert!(text.contains("REPL"));
        // Версия — из Cargo.toml, а не переписанная руками.
        assert!(text.contains(env!("CARGO_PKG_VERSION")));
    }

    #[test]
    fn command_table_is_well_formed() {
        for c in COMMANDS {
            assert!(
                c.flag.starts_with("--"),
                "{}: флаг без двух дефисов",
                c.flag
            );
            assert!(!c.what.is_empty(), "{}: нет описания", c.flag);
            if let Some(alias) = c.alias {
                assert!(alias.starts_with('-'), "{alias}: сокращение без дефиса");
            }
        }
        // Флаги и сокращения не повторяются — иначе разбор нашёл бы первый,
        // а человек читал бы второй.
        let mut all: Vec<&str> = COMMANDS
            .iter()
            .flat_map(|c| std::iter::once(c.flag).chain(c.alias))
            .collect();
        let before = all.len();
        all.sort_unstable();
        all.dedup();
        assert_eq!(before, all.len(), "повтор флага в таблице команд");
    }

    /// Разбор аргументов и подсказка ходят по одной таблице — значит, поиск
    /// по ней обязан находить ровно то, что напечатано в подсказке.
    #[test]
    fn every_advertised_flag_resolves_to_a_command() {
        for c in COMMANDS {
            for flag in std::iter::once(c.flag).chain(c.alias) {
                let found = COMMANDS
                    .iter()
                    .find(|x| x.flag == flag || x.alias == Some(flag));
                assert_eq!(
                    found.map(|x| x.kind),
                    Some(c.kind),
                    "{flag} не резолвится в свою команду"
                );
            }
        }
        assert!(
            COMMANDS
                .iter()
                .all(|c| c.flag != "--нет-такой" && c.alias != Some("--нет-такой"))
        );
    }
}
