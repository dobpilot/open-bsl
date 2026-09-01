//! Крючок отладчика: связывает прогон VM с сеансом DAP.
//!
//! Зовётся перед каждой инструкцией. Пока прогон не на точке останова и
//! редактор ничего не просит, крючок обязан быть дёшев: сравнение строки
//! с множеством и одна попытка чтения канала.
//!
//! На остановке он БЛОКИРУЕТ — и это не изъян, а сам механизм: пока
//! крючок не вернул управление, прогон стоит.

use std::collections::HashSet;

use open_bsl::{DebugAction, DebugPosition};

use super::session::Connection;

/// Что делать на следующей инструкции.
///
/// Три шага различаются ГЛУБИНОЙ КАДРА, а не строкой: `stepIn` идёт куда
/// угодно, `next` не заходит внутрь вызова, `stepOut` не останавливается,
/// пока не вышли. Строка при этом обязана смениться у первых двух —
/// иначе шаг вставал бы на каждой инструкции своей же строки.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// Идти до точки останова.
    Run,
    /// Следующая строка ЛЮБОЙ глубины — `stepIn`.
    StepIn,
    /// Следующая строка не глубже, чем были, — `next`.
    StepOver { depth: usize },
    /// Только выйдя из кадра — `stepOut`.
    StepOut { depth: usize },
}

pub struct Hook {
    /// Соединение РАЗДЕЛЯЕТСЯ с прогоном: крючок живёт внутри `Execution`,
    /// а события завершения шлёт вызывающий, когда `Execution` уже
    /// уронен. Разделять безопасно — здесь один поток; сокет читает
    /// другой, но у него своя половина и свой канал.
    conn: std::rc::Rc<std::cell::RefCell<Connection>>,
    /// Строки, на которых останавливаться.
    breakpoints: HashSet<u32>,
    mode: Mode,
    /// Строка, на которой стояли в прошлый раз: шаг по строке
    /// останавливается, когда она сменилась, а не на каждой инструкции.
    last_line: Option<u32>,
    stopped_once: bool,
}

impl Hook {
    #[must_use]
    pub fn new(
        conn: std::rc::Rc<std::cell::RefCell<Connection>>,
        breakpoints: HashSet<u32>,
        stop_at_entry: bool,
    ) -> Self {
        Self {
            conn,
            breakpoints,
            mode: if stop_at_entry {
                Mode::StepIn
            } else {
                Mode::Run
            },
            last_line: None,
            stopped_once: false,
        }
    }

    /// Стоим, пока редактор не скажет продолжать.
    ///
    /// Возвращает `false`, если он ушёл или попросил прекратить.
    fn stop_and_wait(&mut self, at: &DebugPosition<'_>, reason: &str) -> bool {
        self.conn.borrow_mut().event(
            "stopped",
            serde_json::json!({
                "reason": reason,
                "threadId": 1,
                "allThreadsStopped": true,
            }),
        );
        loop {
            let Some(request) = self.conn.borrow_mut().wait_request() else {
                return false;
            };
            match request["command"].as_str().unwrap_or("") {
                "continue" => {
                    self.mode = Mode::Run;
                    self.conn
                        .borrow_mut()
                        .respond(&request, serde_json::json!({"allThreadsContinued": true}));
                    return true;
                }
                step @ ("next" | "stepIn" | "stepOut") => {
                    let depth = at.frames.len();
                    self.mode = match step {
                        "next" => Mode::StepOver { depth },
                        "stepOut" => Mode::StepOut { depth },
                        _ => Mode::StepIn,
                    };
                    self.conn
                        .borrow_mut()
                        .respond(&request, serde_json::json!({}));
                    return true;
                }
                "stackTrace" => {
                    let frames: Vec<serde_json::Value> = at
                        .frames
                        .iter()
                        .rev()
                        .enumerate()
                        .map(|(i, (_, chunk, pc))| {
                            serde_json::json!({
                                "id": i,
                                "name": format!("чанк {chunk}"),
                                // Строка известна только у ТЕКУЩЕГО кадра;
                                // у остальных её пока негде взять, и ноль
                                // здесь означает «неизвестна», а не первую.
                                "line": if i == 0 { at.line.unwrap_or(0) } else { 0 },
                                "column": 0,
                                "instructionPointerReference": format!("{pc}"),
                            })
                        })
                        .collect();
                    let total = frames.len();
                    self.conn.borrow_mut().respond(
                        &request,
                        serde_json::json!({"stackFrames": frames, "totalFrames": total}),
                    );
                }
                "scopes" => {
                    // Область ровно одна — локальные кадра. Модульные
                    // переменные и глобальные — отдельная работа: их
                    // значения лежат не в кадре.
                    let id = request["arguments"]["frameId"].as_i64().unwrap_or(0);
                    self.conn.borrow_mut().respond(
                        &request,
                        serde_json::json!({"scopes": [{
                            "name": "Локальные",
                            // Ссылка на переменные — номер кадра плюс
                            // единица: ноль в DAP означает «переменных нет».
                            "variablesReference": id + 1,
                            "expensive": false,
                        }]}),
                    );
                }
                "variables" => {
                    let reference = request["arguments"]["variablesReference"]
                        .as_i64()
                        .unwrap_or(0);
                    // Кадры отдаются снаружи внутрь, а `frames` идут
                    // изнутри наружу: номер надо развернуть.
                    let from_top = usize::try_from(reference.max(1) - 1).unwrap_or(0);
                    let index = at.frames.len().saturating_sub(1).saturating_sub(from_top);
                    let vars: Vec<serde_json::Value> = at
                        .values
                        .locals(index)
                        .into_iter()
                        .map(|(name, value)| {
                            serde_json::json!({
                                "name": name,
                                // Через `format_value`: `Display` у
                                // `BslValue` отладочный и форматирования
                                // 1С не воспроизводит.
                                "value": bsl_format::format_value(&value, None)
                                    .unwrap_or_else(|_| String::from("<не отформатировано>")),
                                "variablesReference": 0,
                            })
                        })
                        .collect();
                    self.conn
                        .borrow_mut()
                        .respond(&request, serde_json::json!({"variables": vars}));
                }
                "threads" => {
                    self.conn.borrow_mut().respond(
                        &request,
                        serde_json::json!({"threads": [{"id": 1, "name": "основной"}]}),
                    );
                }
                "setBreakpoints" => {
                    self.breakpoints = collect_lines(&request);
                    self.conn.borrow_mut().respond(&request, verified(&request));
                }
                "disconnect" | "terminate" => {
                    self.conn
                        .borrow_mut()
                        .respond(&request, serde_json::json!({}));
                    return false;
                }
                other => {
                    self.conn
                        .borrow_mut()
                        .refuse(&request, &format!("«{other}» отладчик пока не умеет"));
                }
            }
        }
    }
}

/// Строки из запроса `setBreakpoints`.
pub fn collect_lines(request: &serde_json::Value) -> HashSet<u32> {
    request["arguments"]["breakpoints"]
        .as_array()
        .map(|bps| {
            bps.iter()
                .filter_map(|b| b["line"].as_u64())
                .map(|l| u32::try_from(l).unwrap_or(u32::MAX))
                .collect()
        })
        .unwrap_or_default()
}

/// Подтверждение точек останова: отвечаем той же строкой, которую
/// попросили.
pub fn verified(request: &serde_json::Value) -> serde_json::Value {
    let list: Vec<serde_json::Value> = request["arguments"]["breakpoints"]
        .as_array()
        .map(|bps| {
            bps.iter()
                .map(|b| serde_json::json!({"verified": true, "line": b["line"].clone()}))
                .collect()
        })
        .unwrap_or_default();
    serde_json::json!({ "breakpoints": list })
}

impl open_bsl::DebugHook for Hook {
    fn before_instruction(&mut self, at: &DebugPosition<'_>) -> DebugAction {
        let Some(line) = at.line else {
            // Образ без таблицы строк: останавливаться не на чем.
            return DebugAction::Continue;
        };
        let changed = self.last_line != Some(line);
        let depth = at.frames.len();
        let stepping = !matches!(self.mode, Mode::Run);
        let stop = match self.mode {
            Mode::StepIn => changed || !self.stopped_once,
            // Не глубже, чем были: вызов из этой строки проходится
            // насквозь, а возврат наружу останавливает.
            Mode::StepOver { depth: from } => changed && depth <= from,
            // Только выйдя: пока глубина та же или больше, идём дальше.
            Mode::StepOut { depth: from } => depth < from,
            Mode::Run => changed && self.breakpoints.contains(&line),
        };
        self.last_line = Some(line);
        if !stop {
            // На ходу редактор всё равно может попросить остановиться, и
            // просьба обязана дойти, даже если скрипт крутится в цикле,
            // не порождая ни одной остановки.
            if self.paused_by_request() {
                return self.wait(at, "pause");
            }
            return DebugAction::Continue;
        }
        self.stopped_once = true;
        let reason = if stepping { "step" } else { "breakpoint" };
        self.wait(at, reason)
    }
}

impl Hook {
    /// Просил ли редактор паузу, пока прогон шёл.
    ///
    /// Читается НЕ блокируя: пока никто не просит, шаг стоит одну попытку
    /// чтения канала.
    fn paused_by_request(&mut self) -> bool {
        let Some(request) = self.conn.borrow_mut().poll_request() else {
            return false;
        };
        if request["command"].as_str() == Some("pause") {
            self.conn
                .borrow_mut()
                .respond(&request, serde_json::json!({}));
            return true;
        }
        // Прочее на ходу отвечать нечем: кадр не остановлен.
        let other = request["command"].as_str().unwrap_or("").to_string();
        self.conn
            .borrow_mut()
            .refuse(&request, &format!("«{other}» доступен только на остановке"));
        false
    }

    fn wait(&mut self, at: &DebugPosition<'_>, reason: &str) -> DebugAction {
        if self.stop_and_wait(at, reason) {
            DebugAction::Continue
        } else {
            DebugAction::Terminate
        }
    }
}
