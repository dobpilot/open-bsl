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
    let held = std::net::TcpListener::bind("127.0.0.1:0").expect("занятый порт");
    let port = held.local_addr().expect("адрес").port();
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_bsl-cli"))
        .arg("--debug")
        .arg("--debug-port")
        .arg(port.to_string())
        .arg("нет-такого-файла.bsl")
        .output()
        .expect("запуск");
    assert!(!out.status.success());
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains(&port.to_string()), "stderr: {err}");
    assert!(!err.contains("panicked"), "паника вместо отказа: {err}");
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
