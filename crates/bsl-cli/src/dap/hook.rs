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
    /// Строки, на которых есть код: по ним подтверждаются точки останова,
    /// пришедшие уже на остановке.
    executable: HashSet<u32>,
    /// Путь к отлаживаемому файлу — АБСОЛЮТНЫЙ.
    ///
    /// Без него редактор получает кадры со строками, но не знает, какой
    /// файл открыть: подсветки строки не будет, и отладчик выглядит
    /// сломанным при работающих точках останова. Относительный путь
    /// редактор разрешает от своей рабочей папки, а она не обязана
    /// совпадать с той, откуда запущен процесс.
    source: std::path::PathBuf,
    /// Соединение РАЗДЕЛЯЕТСЯ с прогоном: крючок живёт внутри `Execution`,
    /// а события завершения шлёт вызывающий, когда `Execution` уже
    /// уронен. Разделять безопасно — здесь один поток; сокет читает
    /// другой, но у него своя половина и свой канал.
    conn: std::rc::Rc<std::cell::RefCell<Connection>>,
    /// Строки, на которых останавливаться.
    breakpoints: HashSet<u32>,
    mode: Mode,
    /// Где стояли в прошлый раз: строка, глубина кадра и `pc`.
    ///
    /// Строки одной мало. Цикл, целиком записанный в одну строку, её не
    /// меняет — и точка останова в нём сработала бы РАЗ, а не на каждом
    /// витке. Признак «пришли сюда заново» это `pc`, не равный
    /// предыдущему плюс один, либо смена глубины кадра: и переход назад, и
    /// вызов, и возврат так отличаются от линейного хода внутри одной
    /// строки.
    last: Option<Position>,
    stopped_once: bool,
    /// Считает ли редактор строки с единицы. DAP по умолчанию ДА, но
    /// клиент вправе попросить с нуля через `initialize`; ответить не в
    /// той базе — значит показать не ту строку или получить отказ открыть
    /// кадр.
    lines_start_at_one: bool,
    /// То же про КОЛОНКИ, и это отдельный флаг: DAP передаёт
    /// `linesStartAt1` и `columnsStartAt1` независимо, и связывать их —
    /// значит отвечать не в той базе всякий раз, когда клиент выбрал
    /// разные.
    columns_start_at_one: bool,
}

impl Hook {
    #[must_use]
    pub fn new(
        conn: std::rc::Rc<std::cell::RefCell<Connection>>,
        breakpoints: HashSet<u32>,
        executable: HashSet<u32>,
        source: std::path::PathBuf,
        lines_start_at_one: bool,
        columns_start_at_one: bool,
    ) -> Self {
        Self {
            executable,
            source,
            conn,
            breakpoints,
            // Остановка на входе редактором пока не просится: она
            // приходит отдельным `stopOnEntry`, и заводить её раньше,
            // чем клиент попросит, незачем.
            mode: Mode::Run,
            last: None,
            stopped_once: false,
            lines_start_at_one,
            columns_start_at_one,
        }
    }

    /// Стоим, пока редактор не скажет продолжать.
    ///
    /// Возвращает `false`, если он ушёл или попросил прекратить.
    fn stop_and_wait(&mut self, at: &mut DebugPosition<'_>, reason: &str) -> bool {
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
                    let total = at.frames.len();
                    let base = self.lines_start_at_one;
                    let col_base = self.columns_start_at_one;
                    let mut frames: Vec<serde_json::Value> = Vec::with_capacity(total);
                    for i in 0..total {
                        // Кадры отдаются изнутри наружу, как ждёт редактор,
                        // а `at.frames` идут снаружи внутрь.
                        let index = total - 1 - i;
                        let (module, chunk, pc) = at.frames[index];
                        // Строка КАЖДОГО кадра, а не только текущего:
                        // спрашивается лениво, на остановке.
                        let line = at.values.line_of(index).unwrap_or(0);
                        // Источник — только у кадров КОРНЕВОГО модуля:
                        // чанки чужих модулей каталога лежат в других
                        // файлах, и подсунуть им этот значило бы отправить
                        // редактор не туда.
                        let source = if module == open_bsl::ROOT_MODULE {
                            serde_json::json!({
                                "name": self
                                    .source
                                    .file_name()
                                    .map_or_else(String::new, |n| n.to_string_lossy().into_owned()),
                                "path": self.source.display().to_string(),
                            })
                        } else {
                            serde_json::Value::Null
                        };
                        frames.push(serde_json::json!({
                            "id": i,
                            "name": format!("чанк {chunk}"),
                            "source": source,
                            "line": adjust_line(line, base),
                            // Колонок в таблице нет — в образ уходит строка,
                            // и первая колонка в выбранной базе честнее
                            // выдуманного смещения.
                            "column": u32::from(col_base),
                            "instructionPointerReference": format!("{pc}"),
                        }));
                    }
                    self.conn.borrow_mut().respond(
                        &request,
                        serde_json::json!({"stackFrames": frames, "totalFrames": total}),
                    );
                }
                "evaluate" => {
                    // Кадр выбирает редактор: смотреть переменную
                    // вызывающего, стоя во вложенном вызове, — обычное
                    // дело, и обычный `Вычислить` так не умеет.
                    let from_top = usize::try_from(
                        request["arguments"]["frameId"].as_i64().unwrap_or(0).max(0),
                    )
                    .unwrap_or(0);
                    let index = at.frames.len().saturating_sub(1).saturating_sub(from_top);
                    let source = request["arguments"]["expression"]
                        .as_str()
                        .unwrap_or("")
                        .to_string();
                    match at.values.evaluate(index, &source) {
                        Ok(value) => {
                            let text = bsl_format::format_value(&value, None)
                                .unwrap_or_else(|_| String::from("<не отформатировано>"));
                            self.conn.borrow_mut().respond(
                                &request,
                                serde_json::json!({"result": text, "variablesReference": 0}),
                            );
                        }
                        Err(message) => {
                            self.conn.borrow_mut().refuse(&request, &message);
                        }
                    }
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
                    let resolved = resolve(&request, &self.executable, self.lines_start_at_one);
                    self.breakpoints = resolved.internal;
                    self.conn.borrow_mut().respond(&request, resolved.answer);
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

/// Переводит строку из внутренней нумерации (с единицы) в базу редактора.
///
/// DAP по умолчанию считает с единицы, но клиент вправе попросить с нуля
/// в `initialize`. Ответ не в той базе — это либо чужая строка, либо
/// отказ редактора открыть кадр.
#[must_use]
pub fn adjust_line(line: u32, start_at_one: bool) -> u32 {
    if start_at_one {
        line
    } else {
        line.saturating_sub(1)
    }
}

/// Где стоял прогон на прошлом вызове крючка.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Position {
    line: u32,
    depth: usize,
    pc: usize,
}

/// Разрешение точек останова: ОДИН ответ на оба вопроса.
///
/// Раньше строки для крючка собирались одной функцией, а ответ редактору
/// строился другой, и они расходились: точка на комментарии
/// подтверждалась как строка 4, а хранилась как 2 — редактор рисовал
/// подтверждённую точку, на которой прогон не останавливался никогда.
/// Поэтому здесь одна функция и один результат.
pub struct Resolved {
    /// Строки во ВНУТРЕННЕЙ нумерации (с единицы) — по ним крючок и
    /// сравнивает.
    pub internal: HashSet<u32>,
    /// Ответ редактору, уже в ЕГО системе координат.
    pub answer: serde_json::Value,
}

/// Переводит строку из системы координат редактора во внутреннюю.
///
/// Внутри всё считается с единицы: так строит таблицу компилятор. DAP по
/// умолчанию тоже, но клиент вправе попросить с нуля в `initialize`.
#[must_use]
pub fn to_internal(line: i64, start_at_one: bool) -> u32 {
    let shifted = if start_at_one { line } else { line + 1 };
    u32::try_from(shifted.max(0)).unwrap_or(u32::MAX)
}

/// Обратный перевод — во внутренней нумерации наружу.
#[must_use]
pub fn to_client(line: u32, start_at_one: bool) -> u32 {
    if start_at_one {
        line
    } else {
        line.saturating_sub(1)
    }
}

/// Разрешает точки останова запроса по фактическим строкам образа.
///
/// Строка, на которой нет ни одной инструкции — пустая, комментарий,
/// `КонецЕсли`, — недостижима. Запрос сдвигается ВНИЗ к ближайшей
/// исполняемой: редактор ставит точку на объявление или на комментарий
/// над кодом, а сдвиг вверх увёл бы остановку в уже выполненное. Если
/// ниже кода нет — точка возвращается неподтверждённой с причиной, как и
/// требует DAP.
#[must_use]
pub fn resolve(
    request: &serde_json::Value,
    executable: &HashSet<u32>,
    start_at_one: bool,
) -> Resolved {
    let mut internal = HashSet::new();
    let mut list = Vec::new();
    if let Some(bps) = request["arguments"]["breakpoints"].as_array() {
        for b in bps {
            let asked_client = b["line"].as_i64().unwrap_or(0);
            let asked = to_internal(asked_client, start_at_one);
            match nearest_executable(asked, executable) {
                Some(actual) => {
                    internal.insert(actual);
                    list.push(serde_json::json!({
                        "verified": true,
                        "line": to_client(actual, start_at_one),
                    }));
                }
                None => list.push(serde_json::json!({
                    "verified": false,
                    "line": asked_client,
                    "message": "на этой строке нет исполняемого кода",
                })),
            }
        }
    }
    Resolved {
        internal,
        answer: serde_json::json!({ "breakpoints": list }),
    }
}

/// Ближайшая исполняемая строка НЕ ВЫШЕ запрошенной.
fn nearest_executable(asked: u32, executable: &HashSet<u32>) -> Option<u32> {
    executable.iter().copied().filter(|&l| l >= asked).min()
}

impl open_bsl::DebugHook for Hook {
    fn before_instruction(&mut self, at: &mut DebugPosition<'_>) -> DebugAction {
        let Some(line) = at.line else {
            // Образ без таблицы строк: останавливаться не на чем.
            return DebugAction::Continue;
        };
        let depth = at.frames.len();
        let pc = at.frames.last().map_or(0, |&(_, _, pc)| pc);
        let here = Position { line, depth, pc };
        let changed = self.last.map(|p| p.line) != Some(line);
        // «Пришли заново» — смена строки, смена глубины кадра либо переход
        // НАЗАД внутри той же строки.
        //
        // Направление здесь существенно, и «любой нелинейный переход» —
        // признак неверный. Внутри одной строки бывают переходы ВПЕРЁД:
        // короткое замыкание `Ложь И Истина`, `Если … Тогда … КонецЕсли`
        // в одну строку. Это продолжение текущего прохода, а не новый
        // виток, и остановка на них была бы второй подряд на той же
        // строке. Назад прыгает back-edge — вот он новый виток и есть.
        let re_entered = match self.last {
            None => true,
            Some(prev) => prev.line != line || prev.depth != depth || pc <= prev.pc,
        };
        let stepping = !matches!(self.mode, Mode::Run);
        let stop = match self.mode {
            Mode::StepIn => changed || !self.stopped_once,
            // Не глубже, чем были: вызов из этой строки проходится
            // насквозь, а возврат наружу останавливает.
            Mode::StepOver { depth: from } => changed && depth <= from,
            // Только выйдя: пока глубина та же или больше, идём дальше.
            Mode::StepOut { depth: from } => depth < from,
            // Не `changed`, а `re_entered`: цикл в одну строку иначе
            // останавливался бы однажды вместо каждого витка.
            Mode::Run => re_entered && self.breakpoints.contains(&line),
        };
        self.last = Some(here);
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

    fn wait(&mut self, at: &mut DebugPosition<'_>, reason: &str) -> DebugAction {
        if self.stop_and_wait(at, reason) {
            DebugAction::Continue
        } else {
            DebugAction::Terminate
        }
    }
}
