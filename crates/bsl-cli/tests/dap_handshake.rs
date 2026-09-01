//! Сквозная проверка сеанса DAP подставным клиентом — без редактора.
//!
//! Гоняется НАСТОЯЩИЙ бинарник: отладчик — свойство запуска, и проверять
//! его в процессе теста значило бы проверять не то, что получит
//! пользователь.

use std::io::{Read, Write};
use std::net::TcpStream;

/// Порт выбирает САМ отлаживаемый процесс (`--debug-port 0`), а тест
/// узнаёт его из stderr.
///
/// Прежде тест занимал порт сам, отпускал и передавал номер потомку — и
/// в окне между «отпустил» и «потомок занял» тот же порт успевал взять
/// сосед по параллельному прогону. Тогда потомок падал, а подключение
/// уходило к чужому слушателю и обрывалось. Ловилось только под полной
/// нагрузкой набора; в одиночку тест проходил всегда.
fn port_from_stderr(err: &mut impl Read) -> u16 {
    let mut acc = String::new();
    let mut chunk = [0u8; 256];
    loop {
        let n = err.read(&mut chunk).expect("stderr отлаживаемого процесса");
        assert!(n > 0, "процесс закончился, не назвав порт: {acc}");
        acc.push_str(&String::from_utf8_lossy(&chunk[..n]));
        if let Some(rest) = acc.split_once("ждёт подключения на ") {
            let tail = rest.1;
            if let Some(line) = tail.split('\n').next()
                && let Some((_, port)) = line.trim().rsplit_once(':')
                && let Ok(port) = port.parse::<u16>()
            {
                return port;
            }
        }
    }
}

fn frame(body: &str) -> Vec<u8> {
    let mut v = format!("Content-Length: {}\r\n\r\n", body.len()).into_bytes();
    v.extend_from_slice(body.as_bytes());
    v
}

/// Разбирает поток кадров в список пар «тип, имя».
fn decode(bytes: &[u8]) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut rest = bytes;
    while let Some(i) = rest.windows(4).position(|w| w == b"\r\n\r\n") {
        let head = String::from_utf8_lossy(&rest[..i]).to_string();
        let Some(len) = head
            .split("\r\n")
            .filter_map(|l| l.split_once(':'))
            .find(|(n, _)| n.trim().eq_ignore_ascii_case("Content-Length"))
            .and_then(|(_, v)| v.trim().parse::<usize>().ok())
        else {
            break;
        };
        if rest.len() < i + 4 + len {
            break;
        }
        let body = String::from_utf8_lossy(&rest[i + 4..i + 4 + len]).to_string();
        // Разбор нарочно грубый: тесту нужны только тип и имя, а тащить
        // сюда serde ради двух полей — лишнее.
        let kind = field(&body, "\"type\":\"");
        let name = if kind == "event" {
            field(&body, "\"event\":\"")
        } else {
            field(&body, "\"command\":\"")
        };
        out.push((kind, name));
        rest = &rest[i + 4 + len..];
    }
    out
}

fn field(body: &str, key: &str) -> String {
    body.split_once(key)
        .and_then(|(_, r)| r.split_once('"'))
        .map(|(v, _)| v.to_string())
        .unwrap_or_default()
}

#[test]
fn a_session_handshakes_runs_the_script_and_reports_the_end() {
    let dir = std::env::temp_dir().join(format!("bsl-dap-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("каталог");
    let script = dir.join("привет.bsl");
    std::fs::write(&script, "Сообщить(\"привет\");\n").expect("скрипт");

    let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_bsl-cli"))
        .arg("--debug")
        // Ноль — «выбери сам»: гонки за номер тогда нет вовсе.
        .arg("--debug-port")
        .arg("0")
        .arg(&script)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("запуск bsl-cli");

    // Порт известен ПОСЛЕ того, как процесс уже слушает: он печатает его,
    // связавшись, поэтому опрашивать соединением ничего не нужно.
    let port = port_from_stderr(child.stderr.as_mut().expect("stderr"));
    let mut sock = TcpStream::connect(("127.0.0.1", port)).expect("подключение к отладчику");

    for req in [
        r#"{"seq":1,"type":"request","command":"initialize"}"#,
        r#"{"seq":2,"type":"request","command":"launch"}"#,
        r#"{"seq":3,"type":"request","command":"configurationDone"}"#,
    ] {
        sock.write_all(&frame(req)).expect("запрос");
    }

    let mut got = Vec::new();
    let mut chunk = [0u8; 4096];
    loop {
        match sock.read(&mut chunk) {
            Ok(0) | Err(_) => break,
            Ok(n) => {
                got.extend_from_slice(&chunk[..n]);
                if decode(&got).iter().any(|(_, n)| n == "terminated") {
                    break;
                }
            }
        }
    }
    let seen = decode(&got);
    let out = child.wait_with_output().expect("завершение");

    assert!(
        seen.contains(&("response".into(), "initialize".into())),
        "{seen:?}"
    );
    assert!(
        seen.contains(&("event".into(), "initialized".into())),
        "{seen:?}"
    );
    assert!(
        seen.contains(&("response".into(), "configurationDone".into())),
        "{seen:?}"
    );
    // Без этих двух редактор ждал бы вечно.
    assert!(
        seen.contains(&("event".into(), "exited".into())),
        "{seen:?}"
    );
    assert!(
        seen.contains(&("event".into(), "terminated".into())),
        "{seen:?}"
    );
    // Скрипт при этом действительно исполнился, а не только поговорил.
    assert!(
        String::from_utf8_lossy(&out.stdout).contains("привет"),
        "stdout: {}",
        String::from_utf8_lossy(&out.stdout)
    );
    // И редактор увидел тот же вывод у себя: `Сообщить` дублируется
    // событием `output`, не переставая писать в stdout.
    assert!(
        seen.contains(&("event".into(), "output".into())),
        "нет события output: {seen:?}"
    );
    assert!(
        String::from_utf8_lossy(&got).contains("привет"),
        "в событии output нет текста"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn an_address_flag_without_debug_is_refused() {
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_bsl-cli"))
        .arg("--debug-port")
        .arg("4711")
        .arg("нет-такого-файла.bsl")
        .output()
        .expect("запуск");
    assert!(!out.status.success(), "ключ без --debug обязан отказывать");
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("--debug"),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn a_busy_port_is_refused_clearly_and_does_not_panic() {
    // Скрипт настоящий: компиляция идёт ДО прослушивания, и на
    // несуществующем файле до попытки занять порт дело бы не дошло.
    let dir = std::env::temp_dir().join(format!("bsl-dap-busy-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("каталог");
    let script = dir.join("занято.bsl");
    std::fs::write(&script, "а = 1;\n").expect("скрипт");
    let held = std::net::TcpListener::bind("127.0.0.1:0").expect("занятый порт");
    let port = held.local_addr().expect("адрес").port();
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_bsl-cli"))
        .arg("--debug")
        .arg("--debug-port")
        .arg(port.to_string())
        .arg(&script)
        .output()
        .expect("запуск");
    assert!(!out.status.success());
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains(&port.to_string()), "stderr: {err}");
    assert!(!err.contains("panicked"), "паника вместо отказа: {err}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// Точка останова обязана сработать ДО инструкций своей строки.
///
/// Проверяется не событием, а выводом: в момент `stopped` первая строка
/// уже напечатана, а вторая — ещё нет. Отладчик, останавливающийся после,
/// показывал бы состояние, которого пользователь не просил.
#[test]
fn a_breakpoint_stops_before_its_line_runs() {
    let dir = std::env::temp_dir().join(format!("bsl-dap-bp-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("каталог");
    let script = dir.join("три.bsl");
    std::fs::write(
        &script,
        "Сообщить(\"раз\");\nСообщить(\"два\");\nСообщить(\"три\");\n",
    )
    .expect("скрипт");

    let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_bsl-cli"))
        .arg("--debug")
        .arg("--debug-port")
        .arg("0")
        .arg(&script)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("запуск bsl-cli");
    let port = port_from_stderr(child.stderr.as_mut().expect("stderr"));
    let mut sock = TcpStream::connect(("127.0.0.1", port)).expect("подключение");

    sock.write_all(&frame(
        r#"{"seq":1,"type":"request","command":"initialize"}"#,
    ))
    .expect("initialize");
    sock.write_all(&frame(
        r#"{"seq":2,"type":"request","command":"setBreakpoints","arguments":{"breakpoints":[{"line":2}]}}"#,
    ))
    .expect("setBreakpoints");
    sock.write_all(&frame(
        r#"{"seq":3,"type":"request","command":"configurationDone"}"#,
    ))
    .expect("configurationDone");

    // Ждём остановку.
    let mut got = Vec::new();
    let mut chunk = [0u8; 4096];
    loop {
        let n = sock.read(&mut chunk).expect("чтение");
        assert!(n > 0, "соединение закрылось до остановки");
        got.extend_from_slice(&chunk[..n]);
        if decode(&got).iter().any(|(_, n)| n == "stopped") {
            break;
        }
    }

    // Пока стоим — читаем то, что успело напечататься. `Сообщить` пишет в
    // stdout сразу, без буферизации до конца программы.
    let mut out = child.stdout.take().expect("stdout");
    // Копятся БАЙТЫ: кириллица многобайтная, и складывать прочитанное
    // как `char` значило бы рвать её посередине.
    let mut raw: Vec<u8> = Vec::new();
    let mut byte = [0u8; 1];
    while !raw.contains(&b'\n') {
        let n = out.read(&mut byte).expect("stdout");
        assert!(n > 0, "программа кончилась, не напечатав ни строки");
        raw.push(byte[0]);
    }
    let printed = String::from_utf8_lossy(&raw).to_string();
    assert!(printed.contains("раз"), "напечатано: {printed:?}");
    assert!(
        !printed.contains("два"),
        "остановка пришла ПОСЛЕ строки 2: {printed:?}"
    );

    sock.write_all(&frame(
        r#"{"seq":4,"type":"request","command":"continue","arguments":{"threadId":1}}"#,
    ))
    .expect("continue");
    let mut tail = String::new();
    out.read_to_string(&mut tail).expect("остаток stdout");
    assert!(
        tail.contains("два") && tail.contains("три"),
        "хвост: {tail:?}"
    );
    let _ = child.wait();
    let _ = std::fs::remove_dir_all(&dir);
}

/// Сеанс со скриптом: ставит точку останова, на первой остановке шлёт
/// `step`, дальше продолжает. Возвращает строки всех остановок.
fn stop_lines(script: &std::path::Path, breakpoint: u32, step: &str) -> Vec<i64> {
    let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_bsl-cli"))
        .arg("--debug")
        .arg("--debug-port")
        .arg("0")
        .arg(script)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("запуск");
    let port = port_from_stderr(child.stderr.as_mut().expect("stderr"));
    let mut sock = TcpStream::connect(("127.0.0.1", port)).expect("подключение");
    let mut seq = 0;
    let mut send = |sock: &mut TcpStream, body: String| {
        seq += 1;
        let body = body.replace("SEQ", &seq.to_string());
        sock.write_all(&frame(&body)).expect("запрос");
    };
    send(
        &mut sock,
        r#"{"seq":SEQ,"type":"request","command":"initialize"}"#.into(),
    );
    send(
        &mut sock,
        format!(
            r#"{{"seq":SEQ,"type":"request","command":"setBreakpoints","arguments":{{"breakpoints":[{{"line":{breakpoint}}}]}}}}"#
        ),
    );
    send(
        &mut sock,
        r#"{"seq":SEQ,"type":"request","command":"configurationDone"}"#.into(),
    );

    let mut lines = Vec::new();
    let mut buf = Vec::new();
    let mut chunk = [0u8; 4096];
    let mut stops = 0;
    let mut done = 0;
    loop {
        let n = match sock.read(&mut chunk) {
            Ok(0) | Err(_) => break,
            Ok(n) => n,
        };
        buf.extend_from_slice(&chunk[..n]);
        let seen = decode(&buf);
        while done < seen.len() {
            let (kind, name) = &seen[done];
            done += 1;
            if kind == "event" && name == "stopped" {
                stops += 1;
                send(
                    &mut sock,
                    r#"{"seq":SEQ,"type":"request","command":"stackTrace","arguments":{"threadId":1}}"#.into(),
                );
            } else if kind == "response" && name == "stackTrace" {
                lines.push(line_of_top_frame(&buf));
                let next = if stops == 1 { step } else { "continue" };
                send(
                    &mut sock,
                    format!(
                        r#"{{"seq":SEQ,"type":"request","command":"{next}","arguments":{{"threadId":1}}}}"#
                    ),
                );
            } else if kind == "event" && name == "terminated" {
                let _ = child.wait();
                return lines;
            }
        }
    }
    let _ = child.wait();
    lines
}

/// Строка верхнего кадра из ПОСЛЕДНЕГО ответа `stackTrace` в потоке.
fn line_of_top_frame(buf: &[u8]) -> i64 {
    let text = String::from_utf8_lossy(buf);
    let last = text.rfind("\"stackFrames\"").expect("ответ stackTrace");
    text[last..]
        .split_once("\"line\":")
        .and_then(|(_, r)| r.split(|c: char| !c.is_ascii_digit()).next())
        .and_then(|d| d.parse().ok())
        .expect("строка кадра")
}

/// Три шага различаются глубиной кадра, а не строкой.
#[test]
fn step_over_skips_a_call_while_step_in_enters_it() {
    let dir = std::env::temp_dir().join(format!("bsl-dap-step-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("каталог");
    let script = dir.join("шаги.bsl");
    std::fs::write(
        &script,
        "Процедура Внутри()\n    а = 1;\n    б = 2;\nКонецПроцедуры\n\nВнутри();\nСообщить(\"после\");\n",
    )
    .expect("скрипт");

    // Точка останова на вызове (строка 6).
    let over = stop_lines(&script, 6, "next");
    let into = stop_lines(&script, 6, "stepIn");
    assert_eq!(over.first(), Some(&6), "обе остановки первой — на вызове");
    assert_eq!(into.first(), Some(&6));
    // `next` проходит вызов насквозь и встаёт на следующей строке файла;
    // `stepIn` заходит внутрь процедуры. Если бы они не различались,
    // вторые остановки совпали бы.
    assert_eq!(over.get(1), Some(&7), "next обязан пройти вызов: {over:?}");
    assert_eq!(into.get(1), Some(&2), "stepIn обязан войти: {into:?}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// На остановке редактор видит локальные переменные кадра с их
/// значениями.
#[test]
fn a_stopped_frame_shows_its_locals() {
    let dir = std::env::temp_dir().join(format!("bsl-dap-vars-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("каталог");
    let script = dir.join("переменные.bsl");
    std::fs::write(&script, "а = 41;\nб = а + 1;\nСообщить(б);\n").expect("скрипт");

    let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_bsl-cli"))
        .arg("--debug")
        .arg("--debug-port")
        .arg("0")
        .arg(&script)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("запуск");
    let port = port_from_stderr(child.stderr.as_mut().expect("stderr"));
    let mut sock = TcpStream::connect(("127.0.0.1", port)).expect("подключение");

    for req in [
        r#"{"seq":1,"type":"request","command":"initialize"}"#,
        r#"{"seq":2,"type":"request","command":"setBreakpoints","arguments":{"breakpoints":[{"line":3}]}}"#,
        r#"{"seq":3,"type":"request","command":"configurationDone"}"#,
    ] {
        sock.write_all(&frame(req)).expect("запрос");
    }

    let mut buf = Vec::new();
    let mut chunk = [0u8; 4096];
    let mut asked_scopes = false;
    let mut asked_vars = false;
    loop {
        let n = sock.read(&mut chunk).expect("чтение");
        assert!(n > 0, "соединение закрылось раньше времени");
        buf.extend_from_slice(&chunk[..n]);
        let seen = decode(&buf);
        if !asked_scopes && seen.iter().any(|(_, n)| n == "stopped") {
            asked_scopes = true;
            sock.write_all(&frame(
                r#"{"seq":4,"type":"request","command":"scopes","arguments":{"frameId":0}}"#,
            ))
            .expect("scopes");
        }
        if asked_scopes && !asked_vars && seen.iter().any(|(_, n)| n == "scopes") {
            asked_vars = true;
            sock.write_all(&frame(
                r#"{"seq":5,"type":"request","command":"variables","arguments":{"variablesReference":1}}"#,
            ))
            .expect("variables");
        }
        if asked_vars && seen.iter().filter(|(_, n)| n == "variables").count() > 0 {
            break;
        }
    }

    let text = String::from_utf8_lossy(&buf);
    // Именно МАССИВ, а не поле `command` того же ответа: ключи
    // сериализуются по алфавиту, и `"command":"variables"` идёт позже.
    let body = &text[text.find("\"variables\":[").expect("тело ответа variables")..];
    // Имена из `local_names`, материализованных сборкой со сведениями об
    // отладке; значения — через `format_value`, а не `Display`.
    assert!(body.contains("\"а\""), "нет переменной а: {body}");
    assert!(body.contains("\"41\""), "у а не то значение: {body}");
    assert!(body.contains("\"б\""), "нет переменной б: {body}");
    assert!(body.contains("\"42\""), "у б не то значение: {body}");

    sock.write_all(&frame(
        r#"{"seq":6,"type":"request","command":"continue","arguments":{"threadId":1}}"#,
    ))
    .expect("continue");
    let _ = child.wait();
    let _ = std::fs::remove_dir_all(&dir);
}

/// Отладка и оптимизация, УДАЛЯЮЩАЯ инструкции, несовместимы, и отказ
/// обязан дойти до пользователя через ключи, а не только через API.
#[test]
fn debug_with_a_removing_pass_is_refused_from_the_command_line() {
    let dir = std::env::temp_dir().join(format!("bsl-dap-opt-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("каталог");
    let script = dir.join("простой.bsl");
    std::fs::write(&script, "а = 1;\n").expect("скрипт");

    // Голый `--optimize` включает и `copy-elim` — значит, отказ.
    for flags in [
        vec!["--debug", "--optimize"],
        vec!["--debug", "--optimize=copy-elim"],
        vec!["--debug", "--optimize=ssa-regalloc"],
    ] {
        let out = std::process::Command::new(env!("CARGO_BIN_EXE_bsl-cli"))
            .args(&flags)
            .arg("--debug-port")
            .arg("0")
            .arg(&script)
            .output()
            .expect("запуск");
        assert!(!out.status.success(), "{flags:?} обязано отказывать");
        let err = String::from_utf8_lossy(&out.stderr);
        assert!(
            err.contains("несовместим с этой оптимизацией"),
            "{flags:?}: непонятное сообщение: {err}"
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// `evaluate` считает В ВЫБРАННОМ кадре, а не только в текущем.
///
/// Проверяется одноимённой переменной: `х` во вложенном вызове и `х`
/// снаружи — разные переменные с разными значениями. Отладчик,
/// вычисляющий всегда в верхнем кадре, дал бы на оба запроса одно и то
/// же.
#[test]
fn evaluate_answers_in_the_frame_the_editor_chose() {
    let dir = std::env::temp_dir().join(format!("bsl-dap-eval-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("каталог");
    let script = dir.join("кадры.bsl");
    std::fs::write(
        &script,
        "Процедура Внутри()\n    х = 7;\n    Сообщить(х);\nКонецПроцедуры\n\nх = 100;\nВнутри();\n",
    )
    .expect("скрипт");

    let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_bsl-cli"))
        .arg("--debug")
        .arg("--debug-port")
        .arg("0")
        .arg(&script)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("запуск");
    let port = port_from_stderr(child.stderr.as_mut().expect("stderr"));
    let mut sock = TcpStream::connect(("127.0.0.1", port)).expect("подключение");

    for req in [
        r#"{"seq":1,"type":"request","command":"initialize"}"#,
        r#"{"seq":2,"type":"request","command":"setBreakpoints","arguments":{"breakpoints":[{"line":3}]}}"#,
        r#"{"seq":3,"type":"request","command":"configurationDone"}"#,
    ] {
        sock.write_all(&frame(req)).expect("запрос");
    }

    let mut buf = Vec::new();
    let mut chunk = [0u8; 4096];
    let mut asked = 0;
    loop {
        let n = sock.read(&mut chunk).expect("чтение");
        assert!(n > 0, "соединение закрылось раньше времени");
        buf.extend_from_slice(&chunk[..n]);
        let seen = decode(&buf);
        if asked == 0 && seen.iter().any(|(_, n)| n == "stopped") {
            asked = 1;
            sock.write_all(&frame(
                r#"{"seq":4,"type":"request","command":"evaluate","arguments":{"expression":"х","frameId":0}}"#,
            ))
            .expect("evaluate текущего");
        }
        let answers = seen.iter().filter(|(_, n)| n == "evaluate").count();
        if asked == 1 && answers >= 1 {
            asked = 2;
            sock.write_all(&frame(
                r#"{"seq":5,"type":"request","command":"evaluate","arguments":{"expression":"х","frameId":1}}"#,
            ))
            .expect("evaluate родительского");
        }
        if answers >= 2 {
            break;
        }
    }

    let text = String::from_utf8_lossy(&buf);
    let results: Vec<&str> = text
        .match_indices("\"result\":\"")
        .map(|(i, _)| {
            let rest = &text[i + "\"result\":\"".len()..];
            &rest[..rest.find('"').expect("конец значения")]
        })
        .collect();
    assert_eq!(results.len(), 2, "два ответа evaluate: {results:?}");
    assert_eq!(results[0], "7", "в текущем кадре х = 7: {results:?}");
    assert_eq!(
        results[1], "100",
        "в родительском кадре х = 100: {results:?}"
    );

    sock.write_all(&frame(
        r#"{"seq":6,"type":"request","command":"continue","arguments":{"threadId":1}}"#,
    ))
    .expect("continue");
    let _ = child.wait();
    let _ = std::fs::remove_dir_all(&dir);
}

/// `pause` доходит до прогона, который крутится БЕЗ точек останова.
///
/// Опрос сокета только на остановках этого бы не дал: остановок здесь нет
/// ни одной, и редактор ждал бы конца цикла.
#[test]
fn pause_reaches_a_run_that_never_stops_on_its_own() {
    let dir = std::env::temp_dir().join(format!("bsl-dap-pause-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("каталог");
    let script = dir.join("цикл.bsl");
    // Цикл длинный намеренно: пауза обязана прийти в середину, а не после.
    std::fs::write(
        &script,
        // Переменная НЕ `и`: это ключевое слово `И`, и скрипт с таким
        // именем не компилируется — грабля записана в `AGENTS.md`.
        "с = 0;\nДля ш = 1 По 20000000 Цикл\n    с = с + 1;\nКонецЦикла;\nСообщить(с);\n",
    )
    .expect("скрипт");

    let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_bsl-cli"))
        .arg("--debug")
        .arg("--debug-port")
        .arg("0")
        .arg(&script)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("запуск");
    let port = port_from_stderr(child.stderr.as_mut().expect("stderr"));
    let mut sock = TcpStream::connect(("127.0.0.1", port)).expect("подключение");

    for req in [
        r#"{"seq":1,"type":"request","command":"initialize"}"#,
        // Точек останова НЕТ — прогон сам не остановится ни разу.
        r#"{"seq":2,"type":"request","command":"configurationDone"}"#,
        r#"{"seq":3,"type":"request","command":"pause","arguments":{"threadId":1}}"#,
    ] {
        sock.write_all(&frame(req)).expect("запрос");
    }

    let mut buf = Vec::new();
    let mut chunk = [0u8; 4096];
    loop {
        let n = sock.read(&mut chunk).expect("чтение");
        assert!(n > 0, "прогон кончился, не остановившись по просьбе");
        buf.extend_from_slice(&chunk[..n]);
        let seen = decode(&buf);
        if seen.iter().any(|(_, n)| n == "stopped") {
            break;
        }
        // Программа не должна успеть доиграть: если пришло `terminated`,
        // значит пауза не дошла.
        assert!(
            !seen.iter().any(|(_, n)| n == "terminated"),
            "программа доиграла, не заметив pause: {seen:?}"
        );
    }
    let text = String::from_utf8_lossy(&buf);
    assert!(
        text.contains("\"reason\":\"pause\""),
        "остановка не по просьбе: {text}"
    );

    // Прекращаем: досчитывать двадцать миллионов итераций тесту незачем.
    sock.write_all(&frame(
        r#"{"seq":4,"type":"request","command":"terminate"}"#,
    ))
    .expect("terminate");
    let _ = child.wait();
    let _ = std::fs::remove_dir_all(&dir);
}

/// Считает остановки на точке останова, при желании — под `--jit`.
fn count_stops(script: &std::path::Path, line: u32, jit: bool) -> usize {
    let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_bsl-cli"));
    cmd.arg("--debug");
    if jit {
        cmd.arg("--jit");
    }
    let mut child = cmd
        .arg("--debug-port")
        .arg("0")
        .arg(script)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("запуск");
    let port = port_from_stderr(child.stderr.as_mut().expect("stderr"));
    let mut sock = TcpStream::connect(("127.0.0.1", port)).expect("подключение");
    let mut seq = 3;
    for req in [
        r#"{"seq":1,"type":"request","command":"initialize"}"#.to_string(),
        format!(
            r#"{{"seq":2,"type":"request","command":"setBreakpoints","arguments":{{"breakpoints":[{{"line":{line}}}]}}}}"#
        ),
        r#"{"seq":3,"type":"request","command":"configurationDone"}"#.to_string(),
    ] {
        sock.write_all(&frame(&req)).expect("запрос");
    }
    let mut buf = Vec::new();
    let mut chunk = [0u8; 4096];
    let mut answered = 0;
    loop {
        let n = match sock.read(&mut chunk) {
            Ok(0) | Err(_) => break,
            Ok(n) => n,
        };
        buf.extend_from_slice(&chunk[..n]);
        let seen = decode(&buf);
        let stops = seen.iter().filter(|(_, n)| n == "stopped").count();
        while answered < stops {
            answered += 1;
            seq += 1;
            sock.write_all(&frame(&format!(
                r#"{{"seq":{seq},"type":"request","command":"continue","arguments":{{"threadId":1}}}}"#
            )))
            .expect("continue");
        }
        if seen.iter().any(|(_, n)| n == "terminated") {
            break;
        }
    }
    let _ = child.wait();
    decode(&buf).iter().filter(|(_, n)| n == "stopped").count()
}

/// Точка останова в теле цикла срабатывает на КАЖДОЙ итерации.
///
/// Ловит сразу две поломки, обе найденные ревью: быстрый back-edge
/// numeric-for завершал итерацию, не доходя до крючка, и строка внешнего
/// оператора не восстанавливалась после вложенного блока — из-за чего
/// тело и back-edge несли одну строку, она не менялась, и остановка
/// случалась ровно одна.
#[test]
fn a_breakpoint_in_a_loop_body_fires_every_iteration() {
    let dir = std::env::temp_dir().join(format!("bsl-dap-loop-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("каталог");
    let script = dir.join("витки.bsl");
    std::fs::write(
        &script,
        "с = 0;\nДля ш = 1 По 5 Цикл\n    с = с + 1;\nКонецЦикла;\nСообщить(с);\n",
    )
    .expect("скрипт");
    assert_eq!(count_stops(&script, 3, false), 5, "витков пять");
    // И то же самое с JIT: нативный путь исполнял целые куски чанка, не
    // возвращаясь во внешний цикл, и точка не срабатывала НИ РАЗУ.
    assert_eq!(count_stops(&script, 3, true), 5, "с --jit тоже пять");
    let _ = std::fs::remove_dir_all(&dir);
}
