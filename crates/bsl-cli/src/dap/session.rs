//! Сеанс отладки: слушатель, поток-читатель и разбор запросов.
//!
//! Рантайм однопоточен по устройству — значения держат `Rc`/`RefCell` и не
//! `Send`, — поэтому через границу потока не ходит НИ ОДНО BSL-значение.
//! Ходят кадры протокола, то есть строки. Сокет держит отдельный
//! `std::thread`, он же режет поток на кадры, а с прогоном обменивается
//! готовыми строками через канал. Тот же приём уже применён к пулу
//! фоновых заданий.
//!
//! Так решается и неприятность, которую опрос сокета из крючка не решает:
//! `pause` от редактора обязан дойти и тогда, когда скрипт крутится в
//! цикле, не порождая ни одной остановки.

use std::io::Write;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::mpsc::Receiver;

use super::wire::{FrameReader, pump, write_frame};

/// Соединение с редактором: входящие кадры приходят каналом, исходящие
/// пишутся прямо в сокет.
pub struct Connection {
    incoming: Receiver<String>,
    out: TcpStream,
    seq: i64,
}

/// Ждёт редактор на указанном адресе.
///
/// Ждёт ДО первой инструкции скрипта: отладчик, подключившийся к уже
/// доигравшей программе, бесполезен.
///
/// # Errors
///
/// Адрес занят или недоступен; соединение не установилось.
pub fn listen(addr: SocketAddr) -> Result<Connection, String> {
    let listener =
        TcpListener::bind(addr).map_err(|e| format!("отладчик не смог занять {addr}: {e}"))?;
    // Печатается фактический адрес, а не запрошенный: с портом 0 их
    // выдаёт ОС, и редактору нужен тот, что достался.
    let actual = listener
        .local_addr()
        .map_err(|e| format!("отладчик не узнал свой адрес: {e}"))?;
    eprintln!("отладчик ждёт подключения на {actual}");
    let (stream, peer) = listener
        .accept()
        .map_err(|e| format!("отладчик не принял подключение: {e}"))?;
    eprintln!("отладчик: подключён {peer}");
    let reader = stream
        .try_clone()
        .map_err(|e| format!("отладчик не смог разделить сокет: {e}"))?;
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut src = reader;
        let mut frames = FrameReader::new();
        loop {
            match pump(&mut src, &mut frames) {
                Ok(true) => {}
                // Конец потока или ошибка чтения — редактор ушёл. Канал
                // закроется сам, когда `tx` уронится вместе с потоком.
                Ok(false) | Err(_) => return,
            }
            loop {
                match frames.next_frame() {
                    Ok(Some(frame)) => {
                        if tx.send(frame).is_err() {
                            return;
                        }
                    }
                    Ok(None) => break,
                    Err(e) => {
                        eprintln!("отладчик: битый кадр — {e}");
                        return;
                    }
                }
            }
        }
    });
    Ok(Connection {
        incoming: rx,
        out: stream,
        seq: 0,
    })
}

impl Connection {
    /// Следующий запрос, если он УЖЕ пришёл. `None` — редактор молчит.
    ///
    /// Не блокирует: прогон обязан двигаться, пока никто ничего не
    /// просит. Через это доходит `pause`, посланный на ходу.
    pub fn poll_request(&mut self) -> Option<serde_json::Value> {
        let frame = self.incoming.try_recv().ok()?;
        match serde_json::from_str(&frame) {
            Ok(v) => Some(v),
            Err(e) => {
                eprintln!("отладчик: кадр не разобрался как JSON — {e}");
                None
            }
        }
    }

    /// Ждёт следующий запрос. `None` — редактор отключился.
    pub fn wait_request(&mut self) -> Option<serde_json::Value> {
        let frame = self.incoming.recv().ok()?;
        match serde_json::from_str(&frame) {
            Ok(v) => Some(v),
            Err(e) => {
                eprintln!("отладчик: кадр не разобрался как JSON — {e}");
                None
            }
        }
    }

    fn send(&mut self, mut message: serde_json::Value) {
        self.seq += 1;
        message["seq"] = self.seq.into();
        let text = message.to_string();
        if let Err(e) = write_frame(&mut self.out, &text) {
            eprintln!("отладчик: не удалось отправить кадр — {e}");
        }
    }

    /// Ответ на запрос: `success` и, если есть, тело.
    pub fn respond(&mut self, request: &serde_json::Value, body: serde_json::Value) {
        self.send(serde_json::json!({
            "type": "response",
            "request_seq": request["seq"].clone(),
            "success": true,
            "command": request["command"].clone(),
            "body": body,
        }));
    }

    /// Отказ на запрос, который отладчик не умеет.
    pub fn refuse(&mut self, request: &serde_json::Value, message: &str) {
        self.send(serde_json::json!({
            "type": "response",
            "request_seq": request["seq"].clone(),
            "success": false,
            "command": request["command"].clone(),
            "message": message,
        }));
    }

    /// Событие протокола.
    pub fn event(&mut self, event: &str, body: serde_json::Value) {
        self.send(serde_json::json!({
            "type": "event",
            "event": event,
            "body": body,
        }));
    }

    /// Досылает буферы перед выходом.
    pub fn flush(&mut self) {
        let _ = self.out.flush();
    }
}

/// Что делать прогону после обработки очередного запроса.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum After {
    /// Редактор ещё настраивается — ждать дальше.
    KeepWaiting,
    /// Пришли точки останова: запомнить и ждать дальше.
    Breakpoints(std::collections::HashSet<u32>),
    /// Редактор представился и сказал, с какой цифры считает строки.
    Initialized { lines_start_at_one: bool },
    /// `configurationDone`: пора исполнять.
    Run,
    /// Редактор ушёл или попросил закончить.
    Stop,
}

/// Обрабатывает один запрос настройки.
///
/// Выделено из цикла, чтобы разбор запроса можно было проверить без
/// сокета.
pub fn handle_setup(
    conn: &mut Connection,
    request: &serde_json::Value,
    executable: &std::collections::HashSet<u32>,
    lines_start_at_one: bool,
) -> After {
    match request["command"].as_str().unwrap_or("") {
        "initialize" => {
            conn.respond(
                request,
                serde_json::json!({
                    // Объявляется только то, что уже умеем: возможность,
                    // заявленная и не исполненная, для редактора хуже
                    // отсутствующей.
                    "supportsConfigurationDoneRequest": true,
                }),
            );
            conn.event("initialized", serde_json::json!({}));
            // База координат — выбор КЛИЕНТА. По умолчанию DAP считает с
            // единицы, но редактор вправе попросить с нуля, и ответ не в
            // той базе покажет чужую строку либо получит отказ открыть
            // кадр.
            After::Initialized {
                lines_start_at_one: request["arguments"]["linesStartAt1"]
                    .as_bool()
                    .unwrap_or(true),
            }
        }
        "launch" | "attach" => {
            conn.respond(request, serde_json::json!({}));
            After::KeepWaiting
        }
        "configurationDone" => {
            conn.respond(request, serde_json::json!({}));
            After::Run
        }
        "setBreakpoints" => {
            // Точки останова приходят ДО `configurationDone`: редактор
            // ставит их сразу после `initialized`, пока программа ещё не
            // пошла. Разрешение ОДНО: те же строки уходят и в ответ, и
            // крючку — иначе подтверждённая точка не сработала бы.
            let resolved = super::hook::resolve(request, executable, lines_start_at_one);
            conn.respond(request, resolved.answer);
            After::Breakpoints(resolved.internal)
        }
        "disconnect" | "terminate" => {
            conn.respond(request, serde_json::json!({}));
            After::Stop
        }
        "threads" => {
            // Поток ровно один и всегда: рантайм однопоточен по
            // устройству, и второго здесь быть не может.
            conn.respond(
                request,
                serde_json::json!({"threads": [{"id": 1, "name": "основной"}]}),
            );
            After::KeepWaiting
        }
        other => {
            conn.refuse(request, &format!("«{other}» отладчик пока не умеет"));
            After::KeepWaiting
        }
    }
}
