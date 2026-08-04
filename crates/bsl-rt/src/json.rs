//! JSON: потоковое чтение и запись.
//!
//! Один разборщик на двоих: `ЧтениеJSON` отдаёт его события наружу по
//! одному, а `ПрочитатьJSON` собирает из тех же событий готовое значение.
//! Второй реализации разбора в проекте быть не должно — иначе потоковый и
//! целиком-режим разъедутся на первом же краевом случае.
//!
//! # Что здесь ИЗМЕРЕНО на 8.3.27
//!
//! Всё, что ниже, снято пробами (`tests/conformance/measure/measure-json.bsl`),
//! а не выведено из спецификации JSON — платформа местами от неё отходит:
//!
//! * экранируются только `"`, `\`, ПС и ВК; прямая косая НЕ экранируется,
//!   остальные символы до `0x20` уходят как `\uXXXX` ЗАГЛАВНЫМИ шестнадцатеричными
//!   (табуляция уходит как `\u0009`, а НЕ как `\t`);
//! * не-ASCII (кириллица) пишется как есть;
//! * `ЗаписатьЗначение` принимает ТОЛЬКО строку, число и булево: `Null`,
//!   `Неопределено` и `Дата` дают ошибку типа;
//! * числа пишутся точной десятичной записью с точкой (`1/3` уходит всеми
//!   27 знаками), без разделителей групп;
//! * разборщик СНИСХОДИТЕЛЕН к битому вводу: пропущенное значение,
//!   висячая запятая, отсутствующее двоеточие и незакрытый объект
//!   принимаются молча, ошибку даёт только мусор на месте самого значения.

use std::collections::HashMap;
use std::rc::Rc;

use bsl_number::BslNumber;

use crate::{BslValue, RtError, RtResult};

/// Событие разбора — ровно члены платформенного `ТипЗначенияJSON`, кроме
/// `Комментарий` (комментариев в JSON нет, а расширения платформы мы не
/// читаем).
#[derive(Debug, Clone, PartialEq)]
pub enum JsonEvent {
    ObjectStart,
    ObjectEnd,
    ArrayStart,
    ArrayEnd,
    PropertyName(String),
    Str(String),
    Number(BslNumber),
    Boolean(bool),
    Null,
}

/// Ошибка разбора. Текст платформы мы не воспроизводим (он привязан к её
/// номерам строк), поэтому своё сообщение.
fn bad(what: &str) -> RtError {
    RtError::Json(format!("некорректный JSON: {what}"))
}

// --- Разбор -------------------------------------------------------------

/// Где мы сейчас находимся. Нужно, чтобы отличить имя свойства от строкового
/// значения: лексически это одно и то же.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Ctx {
    /// Внутри объекта, следующий значимый токен — имя свойства.
    ObjectName,
    /// Внутри объекта, имя прочитано, следующий токен — значение.
    ObjectValue,
    /// Внутри массива.
    Array,
}

#[derive(Debug)]
pub struct JsonParser {
    src: Rc<str>,
    /// Байтовая позиция в `src`. Синтакс JSON размечен ASCII-байтами,
    /// а переход по Unicode-пробелам всегда учитывает `len_utf8`, поэтому
    /// позиция остаётся на границе символа.
    pos: usize,
    stack: Vec<Ctx>,
    /// Верхнеуровневое значение уже прочитано — дальше только `Ничего`.
    finished: bool,
}

impl JsonParser {
    pub fn new(text: &str) -> Self {
        Self::from_utf8(Rc::from(text))
    }

    fn from_string(src: String) -> Self {
        Self::from_utf8(Rc::from(src))
    }

    fn from_bsl_string(src: &crate::BslString) -> Self {
        Self::from_utf8(src.shared_utf8())
    }

    fn from_utf8(src: Rc<str>) -> Self {
        JsonParser {
            src,
            pos: 0,
            stack: Vec::new(),
            finished: false,
        }
    }

    fn current_char(&self) -> Option<char> {
        self.src.get(self.pos..)?.chars().next()
    }

    fn skip_ws(&mut self) {
        while let Some(c) = self.current_char() {
            if c.is_whitespace() {
                self.pos += c.len_utf8();
            } else {
                break;
            }
        }
    }

    fn peek(&mut self) -> Option<char> {
        self.skip_ws();
        self.current_char()
    }

    /// Запятые и двоеточия ПРОГЛАТЫВАЮТСЯ как необязательные разделители, а
    /// не требуются: измерено, что `{"а" 1}` и `[1,]` платформа принимает
    /// (`JSON.READ.MALFORMED`). Отдельного состояния «ждём запятую» поэтому
    /// нет — оно бы только выдумывало строгость, которой у платформы нет.
    fn skip_separators(&mut self) {
        while let Some(c) = self.peek() {
            if c == ',' || c == ':' {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    /// Следующее событие; `None` — поток кончился (`ТипЗначенияJSON.Ничего`).
    ///
    /// # Errors
    ///
    /// [`RtError::Json`], если на месте значения стоит то, что значением
    /// быть не может.
    pub fn next_event(&mut self) -> RtResult<Option<JsonEvent>> {
        self.skip_separators();
        let Some(c) = self.peek() else {
            // Незакрытый объект концом ввода не считается ошибкой —
            // измерено на `{"а":1`.
            self.stack.clear();
            self.finished = true;
            return Ok(None);
        };
        if self.finished && self.stack.is_empty() {
            return Ok(None);
        }

        match self.stack.last().copied() {
            Some(Ctx::ObjectName) => {
                if c == '}' {
                    self.pos += 1;
                    self.stack.pop();
                    self.close_value();
                    return Ok(Some(JsonEvent::ObjectEnd));
                }
                if c != '"' {
                    return Err(bad("на месте имени свойства не строка"));
                }
                let name = self.read_string()?;
                *self
                    .stack
                    .last_mut()
                    .expect("контекст объекта только что был") = Ctx::ObjectValue;
                Ok(Some(JsonEvent::PropertyName(name)))
            }
            Some(Ctx::ObjectValue) => {
                // Пропущенное значение (`{"а":}`) — не ошибка: объект просто
                // закрывается. Измерено.
                if c == '}' {
                    self.pos += 1;
                    self.stack.pop();
                    self.close_value();
                    return Ok(Some(JsonEvent::ObjectEnd));
                }
                *self
                    .stack
                    .last_mut()
                    .expect("контекст объекта только что был") = Ctx::ObjectName;
                self.read_value(c)
            }
            Some(Ctx::Array) => {
                if c == ']' {
                    self.pos += 1;
                    self.stack.pop();
                    self.close_value();
                    return Ok(Some(JsonEvent::ArrayEnd));
                }
                self.read_value(c)
            }
            None => {
                if self.finished {
                    return Ok(None);
                }
                self.read_value(c)
            }
        }
    }

    /// Верхнеуровневое значение закрыто — дальше поток пуст.
    fn close_value(&mut self) {
        if self.stack.is_empty() {
            self.finished = true;
        }
    }

    fn read_value(&mut self, c: char) -> RtResult<Option<JsonEvent>> {
        match c {
            '{' => {
                self.pos += 1;
                self.stack.push(Ctx::ObjectName);
                Ok(Some(JsonEvent::ObjectStart))
            }
            '[' => {
                self.pos += 1;
                self.stack.push(Ctx::Array);
                Ok(Some(JsonEvent::ArrayStart))
            }
            '"' => {
                let s = self.read_string()?;
                self.close_value();
                Ok(Some(JsonEvent::Str(s)))
            }
            't' | 'f' | 'n' => {
                let word = self.read_word();
                let ev = match word.as_str() {
                    "true" => JsonEvent::Boolean(true),
                    "false" => JsonEvent::Boolean(false),
                    "null" => JsonEvent::Null,
                    other => return Err(bad(&format!("неизвестное слово «{other}»"))),
                };
                self.close_value();
                Ok(Some(ev))
            }
            c if c == '-' || c == '+' || c.is_ascii_digit() => {
                let n = self.read_number()?;
                self.close_value();
                Ok(Some(JsonEvent::Number(n)))
            }
            other => Err(bad(&format!("значение не может начинаться с «{other}»"))),
        }
    }

    fn read_word(&mut self) -> String {
        let start = self.pos;
        while matches!(self.src.as_bytes().get(self.pos), Some(c) if c.is_ascii_alphabetic()) {
            self.pos += 1;
        }
        self.src[start..self.pos].to_string()
    }

    /// Строка вместе со снятием экранирования.
    ///
    /// Обычная строка без `\` уже является готовым UTF-8-срезом: её
    /// незачем перегонять через `Vec<u16>` и обратно. На первом escape
    /// включается медленный UTF-16-путь: он сохраняет прежнюю семантику
    /// `\uXXXX`, включая суррогатные пары и замену одиночного суррогата.
    fn read_string(&mut self) -> RtResult<String> {
        if self.src.as_bytes().get(self.pos) != Some(&b'"') {
            return Err(bad("ожидалась строка"));
        }
        self.pos += 1;
        let start = self.pos;
        let mut scan = start;
        while let Some(&byte) = self.src.as_bytes().get(scan) {
            match byte {
                b'"' => {
                    let out = self.src[start..scan].to_string();
                    self.pos = scan + 1;
                    return Ok(out);
                }
                b'\\' => {
                    self.pos = start;
                    return self.read_escaped_string();
                }
                _ => scan += 1,
            }
        }
        // Незакрытая строка обрывом ввода и заканчивается: измерено.
        self.pos = self.src.len();
        Ok(self.src[start..].to_string())
    }

    /// Медленный путь строки, в которой встретилось экранирование.
    fn read_escaped_string(&mut self) -> RtResult<String> {
        let mut units: Vec<u16> = Vec::new();
        // Незакрытая строка обрывом ввода и заканчивается: разборщик
        // платформы к этому снисходителен, поэтому цикл просто кончается,
        // а не жалуется.
        while let Some(c) = self.current_char() {
            self.pos += c.len_utf8();
            match c {
                '"' => break,
                '\\' => {
                    let Some(esc) = self.current_char() else {
                        break;
                    };
                    self.pos += esc.len_utf8();
                    match esc {
                        '"' => units.push(u16::from(b'"')),
                        '\\' => units.push(u16::from(b'\\')),
                        '/' => units.push(u16::from(b'/')),
                        'b' => units.push(0x08),
                        'f' => units.push(0x0C),
                        'n' => units.push(u16::from(b'\n')),
                        'r' => units.push(u16::from(b'\r')),
                        't' => units.push(u16::from(b'\t')),
                        'u' => {
                            let mut v: u16 = 0;
                            for _ in 0..4 {
                                let Some(h) = self.current_char() else {
                                    return Err(bad("оборванная последовательность \\u"));
                                };
                                let d = h
                                    .to_digit(16)
                                    .ok_or_else(|| bad("в \\u не шестнадцатеричная цифра"))?;
                                v = v.wrapping_mul(16).wrapping_add(d as u16);
                                self.pos += h.len_utf8();
                            }
                            units.push(v);
                        }
                        other => {
                            return Err(bad(&format!("неизвестное экранирование «\\{other}»")))
                        }
                    }
                }
                c => {
                    let mut buf = [0u16; 2];
                    units.extend_from_slice(c.encode_utf16(&mut buf));
                }
            }
        }
        Ok(String::from_utf16_lossy(&units))
    }

    /// Число вместе с экспонентой. `BslNumber::parse_canonical` экспоненты
    /// не знает (в BSL таких литералов нет), поэтому она разворачивается
    /// здесь сдвигом десятичной точки — точно, без промежуточного `f64`:
    /// весь смысл `bsl-number` в том, что `1e-3` это ровно `0.001`.
    fn read_number(&mut self) -> RtResult<BslNumber> {
        let start = self.pos;
        if matches!(self.src.as_bytes().get(self.pos), Some(b'-') | Some(b'+')) {
            self.pos += 1;
        }
        while matches!(self.src.as_bytes().get(self.pos), Some(c) if c.is_ascii_digit() || *c == b'.')
        {
            self.pos += 1;
        }
        // Текст МАНТИССЫ снимается до разбора экспоненты: иначе `1e3`
        // уехало бы в `parse_canonical` целиком, вместе с показателем.
        let mantissa_end = self.pos;
        let mut exponent: i32 = 0;
        if matches!(self.src.as_bytes().get(self.pos), Some(b'e') | Some(b'E')) {
            let save = self.pos;
            self.pos += 1;
            let neg = match self.src.as_bytes().get(self.pos) {
                Some(b'-') => {
                    self.pos += 1;
                    true
                }
                Some(b'+') => {
                    self.pos += 1;
                    false
                }
                _ => false,
            };
            let digits_start = self.pos;
            while matches!(self.src.as_bytes().get(self.pos), Some(c) if c.is_ascii_digit()) {
                self.pos += 1;
            }
            if self.pos == digits_start {
                // `1e` без цифр — не экспонента; откатываемся и оставляем
                // `e` следующему токену.
                self.pos = save;
            } else {
                let v: i32 = self.src[digits_start..self.pos]
                    .parse()
                    .map_err(|_| bad("слишком большая экспонента"))?;
                exponent = if neg { -v } else { v };
            }
        }
        let text = &self.src[start..mantissa_end];
        if exponent == 0 {
            BslNumber::parse_canonical(text).map_err(|_| bad(&format!("число «{text}»")))
        } else {
            let base = shift_decimal_point(text, exponent)?;
            BslNumber::parse_canonical(&base).map_err(|_| bad(&format!("число «{text}»")))
        }
    }
}

/// Сдвиг десятичной точки в текстовой записи числа на `exp` разрядов —
/// разворачивание экспоненты без потери точности.
fn shift_decimal_point(text: &str, exp: i32) -> RtResult<String> {
    if exp == 0 {
        return Ok(text.to_string());
    }
    let (sign, body) = match text.strip_prefix('-') {
        Some(rest) => ("-", rest),
        None => ("", text.strip_prefix('+').unwrap_or(text)),
    };
    let (int_part, frac_part) = match body.split_once('.') {
        Some((a, b)) => (a, b),
        None => (body, ""),
    };
    let digits: String = format!("{int_part}{frac_part}");
    // Позиция точки, считая от начала строки цифр.
    let point = int_part.len() as i32 + exp;
    let mut out = String::from(sign);
    if point <= 0 {
        out.push_str("0.");
        for _ in 0..-point {
            out.push('0');
        }
        out.push_str(&digits);
    } else if (point as usize) >= digits.len() {
        out.push_str(&digits);
        for _ in 0..(point as usize - digits.len()) {
            out.push('0');
        }
    } else {
        out.push_str(&digits[..point as usize]);
        out.push('.');
        out.push_str(&digits[point as usize..]);
    }
    Ok(out)
}

// --- Запись -------------------------------------------------------------

/// `ПереносСтрокJSON`. `Авто` — то, что принято в системе; измерено, что на
/// Linux платформа даёт ПС, то есть то же, что `Unix`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsonLineBreak {
    None,
    Auto,
    Windows,
    Unix,
}

impl JsonLineBreak {
    fn text(self) -> &'static str {
        match self {
            JsonLineBreak::None => "",
            JsonLineBreak::Windows => "\r\n",
            JsonLineBreak::Auto | JsonLineBreak::Unix => "\n",
        }
    }
}

/// `ПараметрыЗаписиJSON`. Полная сигнатура платформы длиннее (ключи
/// экранирования, кавычки), здесь — та часть, что измерена и наблюдаема в
/// выводе.
#[derive(Debug, Clone)]
pub struct JsonWriterSettings {
    pub line_break: JsonLineBreak,
    pub indent: String,
}

impl Default for JsonWriterSettings {
    /// Умолчание `УстановитьСтроку()` без аргументов: переносы есть,
    /// отступа НЕТ. Измерено (`JSON.WRITE.DEFAULT_FORMAT` дал
    /// `{ПС}"а": 1,{ПС}"б": "текст"{ПС}}` — ни таба, ни пробелов перед
    /// именем).
    fn default() -> Self {
        JsonWriterSettings {
            line_break: JsonLineBreak::Auto,
            indent: String::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum WCtx {
    Object,
    Array,
}

/// Состояние `ЗаписьJSON`.
#[derive(Debug)]
pub struct JsonWriter {
    out: String,
    settings: JsonWriterSettings,
    stack: Vec<WCtx>,
    /// В текущем контейнере уже что-то записано — значит, перед следующим
    /// членом нужна запятая.
    has_member: Vec<bool>,
    /// Имя свойства записано, ждём значение.
    awaiting_value: bool,
    /// Куда уйдёт результат: `None` — в строку (`УстановитьСтроку`).
    path: Option<std::path::PathBuf>,
}

impl JsonWriter {
    pub fn to_string_target(settings: JsonWriterSettings) -> Self {
        JsonWriter {
            out: String::new(),
            settings,
            stack: Vec::new(),
            has_member: Vec::new(),
            awaiting_value: false,
            path: None,
        }
    }

    pub fn to_file(path: std::path::PathBuf, settings: JsonWriterSettings) -> Self {
        let mut w = Self::to_string_target(settings);
        w.path = Some(path);
        w
    }

    fn pretty(&self) -> bool {
        self.settings.line_break != JsonLineBreak::None
    }

    /// Перевод строки и отступ по текущей глубине.
    fn newline(&mut self, depth: usize) {
        if !self.pretty() {
            return;
        }
        self.out.push_str(self.settings.line_break.text());
        for _ in 0..depth {
            self.out.push_str(&self.settings.indent);
        }
    }

    /// Разделитель перед очередным членом контейнера.
    fn before_member(&mut self) {
        if self.awaiting_value {
            // Значение после имени свойства: отступ уже поставлен именем.
            return;
        }
        if let Some(last) = self.has_member.last_mut() {
            let had = *last;
            *last = true;
            if had {
                self.out.push(',');
            }
            let depth = self.stack.len();
            self.newline(depth);
        }
    }

    /// # Errors
    ///
    /// [`RtError::Json`], если запись нарушает структуру документа.
    pub fn begin_object(&mut self) -> RtResult<()> {
        self.before_member();
        self.out.push('{');
        self.stack.push(WCtx::Object);
        self.has_member.push(false);
        self.awaiting_value = false;
        Ok(())
    }

    /// # Errors
    ///
    /// [`RtError::Json`], если открытого объекта нет.
    pub fn end_object(&mut self) -> RtResult<()> {
        if self.stack.last() != Some(&WCtx::Object) {
            return Err(RtError::Json(
                "ЗаписатьКонецОбъекта без открытого объекта".to_string(),
            ));
        }
        let had = self.has_member.pop().unwrap_or(false);
        self.stack.pop();
        if had {
            let depth = self.stack.len();
            self.newline(depth);
        }
        self.out.push('}');
        Ok(())
    }

    /// # Errors
    ///
    /// Не возвращает ошибок; `Result` — ради единообразия с остальными
    /// методами записи.
    pub fn begin_array(&mut self) -> RtResult<()> {
        self.before_member();
        self.out.push('[');
        self.stack.push(WCtx::Array);
        self.has_member.push(false);
        self.awaiting_value = false;
        Ok(())
    }

    /// # Errors
    ///
    /// [`RtError::Json`], если открытого массива нет.
    pub fn end_array(&mut self) -> RtResult<()> {
        if self.stack.last() != Some(&WCtx::Array) {
            return Err(RtError::Json(
                "ЗаписатьКонецМассива без открытого массива".to_string(),
            ));
        }
        let had = self.has_member.pop().unwrap_or(false);
        self.stack.pop();
        if had {
            let depth = self.stack.len();
            self.newline(depth);
        }
        self.out.push(']');
        Ok(())
    }

    /// Открывает имя свойства и ставит начальную кавычку.
    fn begin_property_name(&mut self) -> RtResult<()> {
        if self.stack.last() != Some(&WCtx::Object) {
            return Err(RtError::Json("ЗаписатьИмяСвойства вне объекта".to_string()));
        }
        self.before_member();
        self.out.push('"');
        Ok(())
    }

    /// Закрывает имя свойства и переводит автомат в режим ожидания
    /// значения.
    fn finish_property_name(&mut self) {
        self.out.push('"');
        self.out.push(':');
        // Пробел после двоеточия ставится ТОЛЬКО в форматированном режиме:
        // измерено, что `ПереносСтрокJSON.Нет` даёт `{"а":1}` без пробела,
        // а умолчание — `"а": 1` с пробелом.
        if self.pretty() {
            self.out.push(' ');
        }
        self.awaiting_value = true;
    }

    /// # Errors
    ///
    /// [`RtError::Json`], если мы не внутри объекта.
    pub fn property_name(&mut self, name: &str) -> RtResult<()> {
        self.begin_property_name()?;
        escape_into(&mut self.out, name);
        self.finish_property_name();
        Ok(())
    }

    /// То же имя свойства, но прямо из внутреннего UTF-16 `BslString`.
    fn property_name_bsl(&mut self, name: &crate::BslString) -> RtResult<()> {
        self.begin_property_name()?;
        escape_bsl_string_into(&mut self.out, name);
        self.finish_property_name();
        Ok(())
    }

    /// Готовый текст значения (строка уже с кавычками и экранированием).
    fn raw_value(&mut self, text: &str) {
        self.before_member();
        self.out.push_str(text);
        self.awaiting_value = false;
    }

    /// # Errors
    ///
    /// [`RtError::Json`], если значение пишется в объект без имени свойства,
    /// либо тип значения записать нечем.
    pub fn value(&mut self, v: &BslValue) -> RtResult<()> {
        if self.stack.last() == Some(&WCtx::Object) && !self.awaiting_value {
            return Err(RtError::Json(
                "значение в объекте без имени свойства".to_string(),
            ));
        }
        // ИЗМЕРЕНО, и это контринтуитивно: `Неопределено` записывается как
        // `null`, а вот `Null` и `Дата` платформа ОТВЕРГАЕТ несоответствием
        // типов (`JSON.WRITE.VALUE_UNDEFINED` против `VALUE_NULL` и
        // `VALUE_DATE` — три отдельные пробы, по одному вызову в каждой).
        // Логику в этом искать не стоит: `Null` в 1С — значение из базы
        // данных, а не «пусто» языка, и в JSON его отображение платформа,
        // видимо, считает неоднозначным.
        if !matches!(
            v,
            BslValue::Undefined | BslValue::Str(_) | BslValue::Number(_) | BslValue::Boolean(_)
        ) {
            return Err(RtError::TypeError {
                expected: "Строка, Число или Булево",
                op: "ЗаписатьЗначение",
            });
        }

        self.before_member();
        match v {
            BslValue::Undefined => self.out.push_str("null"),
            BslValue::Str(s) => {
                self.out.push('"');
                escape_bsl_string_into(&mut self.out, s);
                self.out.push('"');
            }
            BslValue::Number(n) => self.out.push_str(&n.to_canonical()),
            BslValue::Boolean(true) => self.out.push_str("true"),
            BslValue::Boolean(false) => self.out.push_str("false"),
            _ => unreachable!("типы проверены выше"),
        }
        self.awaiting_value = false;
        Ok(())
    }

    /// Значение, которое умеет писать только `ЗаписатьJSON`: `null` из
    /// `Неопределено`/`Null` и дата строкой.
    pub fn literal(&mut self, text: &str) {
        self.raw_value(text);
    }

    pub fn is_string_target(&self) -> bool {
        self.path.is_none()
    }

    pub fn text(&self) -> &str {
        &self.out
    }

    /// `Закрыть()`. Для строкового приёмника отдаёт накопленный текст, для
    /// файлового — пишет его на диск и отдаёт пустую строку.
    ///
    /// # Errors
    ///
    /// [`RtError::IoError`], если файл не записался.
    pub fn finish(&mut self) -> RtResult<String> {
        match self.path.take() {
            None => Ok(std::mem::take(&mut self.out)),
            Some(path) => {
                std::fs::write(&path, self.out.as_bytes())
                    .map_err(|e| RtError::IoError(format!("{}: {e}", path.display())))?;
                self.out.clear();
                Ok(String::new())
            }
        }
    }
}

/// Экранирование по измеренному набору правил (см. обзор модуля).
fn escape_into(out: &mut String, s: &str) {
    for ch in s.chars() {
        escape_char_into(out, ch);
    }
}

/// Экранирует `BslString` без промежуточного UTF-8 `String`.
fn escape_bsl_string_into(out: &mut String, s: &crate::BslString) {
    let units = s.units();
    let mut pos = 0;
    while let Some(&unit) = units.get(pos) {
        match unit {
            0x22 => out.push_str("\\\""),
            0x5c => out.push_str("\\\\"),
            0x0a => out.push_str("\\n"),
            0x0d => out.push_str("\\r"),
            0x00..=0x1f => escape_control_into(out, unit as u8),
            0x20..=0x7f => out.push(unit as u8 as char),
            0xd800..=0xdbff => {
                let Some(&low @ 0xdc00..=0xdfff) = units.get(pos + 1) else {
                    out.push(char::REPLACEMENT_CHARACTER);
                    pos += 1;
                    continue;
                };
                let scalar =
                    0x1_0000 + ((u32::from(unit) - 0xd800) << 10) + (u32::from(low) - 0xdc00);
                out.push(char::from_u32(scalar).expect("суррогатная пара всегда валидна"));
                pos += 2;
                continue;
            }
            0xdc00..=0xdfff => out.push(char::REPLACEMENT_CHARACTER),
            _ => out.push(char::from_u32(u32::from(unit)).expect("BMP-юнит вне суррогатов")),
        }
        pos += 1;
    }
}

#[inline(always)]
fn escape_char_into(out: &mut String, ch: char) {
    match ch {
        '"' => out.push_str("\\\""),
        '\\' => out.push_str("\\\\"),
        '\n' => out.push_str("\\n"),
        '\r' => out.push_str("\\r"),
        c if (c as u32) < 0x20 => escape_control_into(out, c as u8),
        c => out.push(c),
    }
}

/// У всех экранируемых здесь символов старшие два hex-разряда нулевые.
#[inline(always)]
fn escape_control_into(out: &mut String, byte: u8) {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    out.push_str("\\u00");
    out.push(HEX[usize::from(byte >> 4)] as char);
    out.push(HEX[usize::from(byte & 0x0f)] as char);
}

// --- Склейка с объектами BSL --------------------------------------------
//
// Методы `ЧтениеJSON`/`ЗаписьJSON` живут здесь, а не в `lib.rs`: там
// `BslValue` уже на две с лишним тысячи строк, а семантика JSON целиком
// принадлежит этому модулю. Наружу они уходят через `builtin.rs`, как и
// методы таблицы значений через `table.rs`.

use crate::object::{BslObject, JsonReaderState, StructureStorage};
use crate::EnumValue;

fn as_reader(v: &BslValue) -> RtResult<&std::cell::RefCell<JsonReaderState>> {
    match v {
        BslValue::Object(o) => match &**o {
            BslObject::JsonReader(state) => Ok(state),
            _ => Err(not_applicable(v, "ЧтениеJSON")),
        },
        _ => Err(not_applicable(v, "ЧтениеJSON")),
    }
}

fn as_writer(v: &BslValue) -> RtResult<&std::cell::RefCell<Option<JsonWriter>>> {
    match v {
        BslValue::Object(o) => match &**o {
            BslObject::JsonWriter(state) => Ok(state),
            _ => Err(not_applicable(v, "ЗаписьJSON")),
        },
        _ => Err(not_applicable(v, "ЗаписьJSON")),
    }
}

fn not_applicable(v: &BslValue, _expected: &str) -> RtError {
    RtError::MethodNotApplicable {
        method: "метод JSON",
        receiver: v.type_name(),
    }
}

/// Настройки из аргумента `УстановитьСтроку([Параметры])`. Отсутствующий
/// аргумент — умолчание (переносы есть, отступа нет).
fn settings_from(arg: Option<&BslValue>) -> RtResult<JsonWriterSettings> {
    match arg {
        None | Some(BslValue::Undefined) => Ok(JsonWriterSettings::default()),
        Some(BslValue::Object(o)) => match &**o {
            BslObject::JsonWriterSettings(s) => Ok(s.clone()),
            _ => Err(RtError::TypeError {
                expected: "ПараметрыЗаписиJSON",
                op: "УстановитьСтроку",
            }),
        },
        Some(_) => Err(RtError::TypeError {
            expected: "ПараметрыЗаписиJSON",
            op: "УстановитьСтроку",
        }),
    }
}

/// `ЧтениеJSON.УстановитьСтроку(Текст)` / `ЗаписьJSON.УстановитьСтроку([Параметры])`.
///
/// Один метод на два объекта — как и в самой платформе: смысл выбирается
/// по получателю, а не по имени.
///
/// # Errors
///
/// [`RtError::TypeError`], если получатель не объект JSON либо аргумент не
/// того типа.
pub fn set_string(obj: &BslValue, args: &[BslValue]) -> RtResult<()> {
    if let Ok(reader) = as_reader(obj) {
        let BslValue::Str(text) = args.first().unwrap_or(&BslValue::Undefined) else {
            return Err(RtError::TypeError {
                expected: "Строка",
                op: "УстановитьСтроку",
            });
        };
        *reader.borrow_mut() = JsonReaderState {
            parser: Some(JsonParser::from_bsl_string(text)),
            current: None,
        };
        return Ok(());
    }
    let writer = as_writer(obj)?;
    *writer.borrow_mut() = Some(JsonWriter::to_string_target(settings_from(args.first())?));
    Ok(())
}

/// `ОткрытьФайл(Имя[, Параметры])`. У читателя файл загружается целиком:
/// разборщик и так держит весь текст (`ЧтениеJSON` в платформе потоковый,
/// но наблюдаемой разницы это не даёт, кроме памяти на большом файле).
///
/// # Errors
///
/// [`RtError::IoError`], если файл не читается; [`RtError::TypeError`] при
/// неверных аргументах.
pub fn open_file(obj: &BslValue, args: &[BslValue]) -> RtResult<()> {
    let BslValue::Str(path) = args.first().unwrap_or(&BslValue::Undefined) else {
        return Err(RtError::TypeError {
            expected: "Строка",
            op: "ОткрытьФайл",
        });
    };
    let path = path.to_string();
    if let Ok(reader) = as_reader(obj) {
        let text =
            std::fs::read_to_string(&path).map_err(|e| RtError::IoError(format!("{path}: {e}")))?;
        // Метка порядка байтов в начале файла — не часть документа.
        let text = text.strip_prefix('\u{feff}').unwrap_or(&text).to_string();
        *reader.borrow_mut() = JsonReaderState {
            parser: Some(JsonParser::from_string(text)),
            current: None,
        };
        return Ok(());
    }
    let writer = as_writer(obj)?;
    *writer.borrow_mut() = Some(JsonWriter::to_file(
        std::path::PathBuf::from(path),
        settings_from(args.get(1))?,
    ));
    Ok(())
}

/// `ЧтениеJSON.Прочитать()` -> `Булево`: удалось ли перейти к следующему
/// элементу.
///
/// # Errors
///
/// [`RtError::Json`] на битом вводе; [`RtError::TypeError`], если источник
/// ещё не назначен.
pub fn read(obj: &BslValue) -> RtResult<bool> {
    let reader = as_reader(obj)?;
    let mut state = reader.borrow_mut();
    let Some(parser) = state.parser.as_mut() else {
        return Err(RtError::TypeError {
            expected: "назначенный источник (УстановитьСтроку/ОткрытьФайл)",
            op: "Прочитать",
        });
    };
    let event = parser.next_event()?;
    let advanced = event.is_some();
    state.current = event;
    Ok(advanced)
}

/// `ЧтениеJSON.Пропустить()` — проглотить текущее значение целиком вместе
/// с вложенными. Имя именно такое: `ПропуститьЗначение` платформа не знает
/// (измерено — «Метод объекта не обнаружен»).
///
/// # Errors
///
/// [`RtError::Json`] на битом вводе.
pub fn skip(obj: &BslValue) -> RtResult<()> {
    let reader = as_reader(obj)?;
    let mut state = reader.borrow_mut();
    if state.parser.is_none() {
        return Err(RtError::TypeError {
            expected: "назначенный источник (УстановитьСтроку/ОткрытьФайл)",
            op: "Пропустить",
        });
    }
    // Глубина считается от ТЕКУЩЕГО события: если стоим на начале
    // контейнера, дочитываем до его конца; на скаляре — делать нечего.
    let mut depth = match state.current {
        Some(JsonEvent::ObjectStart) | Some(JsonEvent::ArrayStart) => 1,
        _ => 0,
    };
    while depth > 0 {
        let parser = state.parser.as_mut().expect("наличие проверено выше");
        let Some(event) = parser.next_event()? else {
            break;
        };
        match event {
            JsonEvent::ObjectStart | JsonEvent::ArrayStart => depth += 1,
            JsonEvent::ObjectEnd | JsonEvent::ArrayEnd => depth -= 1,
            _ => {}
        }
        state.current = Some(event);
    }
    // ИЗМЕРЕНО (`JSON.READ.SKIP`): после пропуска читатель стоит на
    // СЛЕДУЮЩЕМ элементе, а не на закрывающей скобке пропущенного. То есть
    // `Пропустить` дочитывает значение и делает ещё один шаг — иначе
    // вызывающий цикл увидел бы лишний `КонецМассива`, которого у
    // платформы нет.
    let parser = state.parser.as_mut().expect("наличие проверено выше");
    let next = parser.next_event()?;
    state.current = next;
    Ok(())
}

/// `ЧтениеJSON.ТипТекущегоЗначения` — член `ТипЗначенияJSON`.
pub fn current_value_type(obj: &BslValue) -> RtResult<BslValue> {
    let reader = as_reader(obj)?;
    let state = reader.borrow();
    let v = match &state.current {
        None => EnumValue::JsonNothing,
        Some(JsonEvent::ObjectStart) => EnumValue::JsonObjectStart,
        Some(JsonEvent::ObjectEnd) => EnumValue::JsonObjectEnd,
        Some(JsonEvent::ArrayStart) => EnumValue::JsonArrayStart,
        Some(JsonEvent::ArrayEnd) => EnumValue::JsonArrayEnd,
        Some(JsonEvent::PropertyName(_)) => EnumValue::JsonPropertyName,
        Some(JsonEvent::Str(_)) => EnumValue::JsonString,
        Some(JsonEvent::Number(_)) => EnumValue::JsonNumber,
        Some(JsonEvent::Boolean(_)) => EnumValue::JsonBoolean,
        Some(JsonEvent::Null) => EnumValue::JsonNull,
    };
    Ok(BslValue::Enum(v))
}

/// `ЧтениеJSON.ТекущееЗначение`.
///
/// # Errors
///
/// [`RtError::Json`], если у текущего элемента значения нет (мы стоим на
/// скобке). ИЗМЕРЕНО: платформа отвечает ровно так же — «Текущее значение
/// JSON не может быть получено», — а не `Неопределено`.
pub fn current_value(obj: &BslValue) -> RtResult<BslValue> {
    let reader = as_reader(obj)?;
    let state = reader.borrow();
    match &state.current {
        Some(JsonEvent::PropertyName(s)) | Some(JsonEvent::Str(s)) => {
            Ok(BslValue::Str(crate::BslString::from_str(s)))
        }
        Some(JsonEvent::Number(n)) => Ok(BslValue::Number(n.clone())),
        Some(JsonEvent::Boolean(b)) => Ok(BslValue::Boolean(*b)),
        // JSON `null` при чтении становится `Неопределено`, а не `Null` —
        // измерено на `ПрочитатьJSON` (`JSON.DESERIALIZE.VALUE_TYPES`).
        Some(JsonEvent::Null) => Ok(BslValue::Undefined),
        _ => Err(RtError::Json(
            "Текущее значение JSON не может быть получено".to_string(),
        )),
    }
}

/// Общая часть всех пишущих методов: достать writer и применить операцию.
fn with_writer<T>(obj: &BslValue, f: impl FnOnce(&mut JsonWriter) -> RtResult<T>) -> RtResult<T> {
    let cell = as_writer(obj)?;
    let mut slot = cell.borrow_mut();
    let Some(writer) = slot.as_mut() else {
        return Err(RtError::TypeError {
            expected: "назначенный приёмник (УстановитьСтроку/ОткрытьФайл)",
            op: "запись JSON",
        });
    };
    f(writer)
}

/// # Errors
///
/// Ошибку структуры документа либо отсутствия приёмника.
pub fn write_start_object(obj: &BslValue) -> RtResult<()> {
    with_writer(obj, JsonWriter::begin_object)
}

/// # Errors
///
/// См. [`write_start_object`].
pub fn write_end_object(obj: &BslValue) -> RtResult<()> {
    with_writer(obj, JsonWriter::end_object)
}

/// # Errors
///
/// См. [`write_start_object`].
pub fn write_start_array(obj: &BslValue) -> RtResult<()> {
    with_writer(obj, JsonWriter::begin_array)
}

/// # Errors
///
/// См. [`write_start_object`].
pub fn write_end_array(obj: &BslValue) -> RtResult<()> {
    with_writer(obj, JsonWriter::end_array)
}

/// # Errors
///
/// [`RtError::TypeError`], если имя не строка.
pub fn write_property_name(obj: &BslValue, args: &[BslValue]) -> RtResult<()> {
    let BslValue::Str(name) = args.first().unwrap_or(&BslValue::Undefined) else {
        return Err(RtError::TypeError {
            expected: "Строка",
            op: "ЗаписатьИмяСвойства",
        });
    };
    with_writer(obj, |w| w.property_name_bsl(name))
}

/// # Errors
///
/// [`RtError::TypeError`] на типе, который платформа не принимает
/// (`Null`, `Неопределено`, `Дата`).
pub fn write_value(obj: &BslValue, args: &[BslValue]) -> RtResult<()> {
    let value = args.first().cloned().unwrap_or(BslValue::Undefined);
    with_writer(obj, |w| w.value(&value))
}

/// `ЗаписьJSON.Закрыть()` -> текст (для строкового приёмника) либо пустая
/// строка (для файлового).
///
/// # Errors
///
/// [`RtError::IoError`] при неудачной записи файла.
pub fn close_writer(obj: &BslValue) -> RtResult<BslValue> {
    let cell = as_writer(obj)?;
    let mut slot = cell.borrow_mut();
    let Some(writer) = slot.as_mut() else {
        return Ok(BslValue::Str(crate::BslString::from_str("")));
    };
    let text = writer.finish()?;
    *slot = None;
    Ok(BslValue::Str(crate::BslString::from_utf8_string(text)))
}

/// Получатель — `ЗаписьJSON`? Нужно `BslValue::close_object`, чтобы
/// развести одноимённый `Закрыть` у двух разных объектов.
pub fn is_json_writer(v: &BslValue) -> bool {
    matches!(v, BslValue::Object(o) if matches!(&**o, BslObject::JsonWriter(_)))
}

// --- ПрочитатьJSON / ЗаписатьJSON ---------------------------------------

use crate::runtime_shapes::RuntimeShapes;

/// Подготовленные имена свойств в пределах одного `ПрочитатьJSON`.
///
/// Ключ хранит точное написание из JSON: разный регистр может дать
/// две записи в этом кэше, но `NameInterner` всё равно вернёт им один
/// регистронезависимый `NameId`. Это сохраняет семантику и не требует
/// Unicode-нормализации на каждом повторе одной и той же схемы.
type JsonKeyCache = HashMap<Box<str>, crate::NameId>;

/// Итоговые формы объектов, уже встреченные в текущем документе.
///
/// Первый объект каждой схемы строится обычными `structure_insert`: так
/// сохраняются порог переходов и деградация в словарь. Повторный объект
/// получает ту же форму и готовые слоты сразу, без прохода по цепочке
/// промежуточных форм для каждого поля.
type JsonShapeCache = HashMap<Vec<crate::NameId>, Rc<crate::Shape>>;

#[derive(Default)]
struct JsonBuildCache {
    keys: JsonKeyCache,
    shapes: JsonShapeCache,
}

/// Имя свойства годится в поле структуры? Платформа отвергает ключ,
/// который не является идентификатором (измерено: `{"не имя":1}` при
/// разборе в структуру — ошибка), поэтому проверка обязана быть здесь, а
/// не «как получится»: интернер-то примет любую строку.
fn is_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first.is_alphabetic() || first == '_') {
        return false;
    }
    chars.all(|c| c.is_alphanumeric() || c == '_')
}

/// Разбирает дату из строки JSON. Поддержаны формы ISO без зоны и с `Z`.
///
/// ОТКЛОНЕНИЕ, СОЗНАТЕЛЬНОЕ: платформа на `...Z` переводит момент в
/// ЛОКАЛЬНОЕ время (измерено — `05:06:07Z` вернулось как `8:06:07` в зоне
/// UTC+3), а здесь суффикс только распознаётся, сдвига нет. Причина не в
/// лени: у движка вообще нет понятия часового пояса — `ТекущаяДата` тоже
/// отдаёт UTC (давнее задокументированное отклонение), и вводить смещение
/// ради одной функции значило бы завести полузону, о которой не знает
/// остальной рантайм.
fn parse_json_date(text: &str) -> Option<crate::BslDate> {
    let body = text.strip_suffix('Z').unwrap_or(text);
    let (date, time) = match body.split_once('T') {
        Some((d, t)) => (d, t),
        None => (body, "00:00:00"),
    };
    let mut dp = date.split('-');
    let year: i64 = dp.next()?.parse().ok()?;
    let month: u32 = dp.next()?.parse().ok()?;
    let day: u32 = dp.next()?.parse().ok()?;
    let mut tp = time.split(':');
    let hour: u32 = tp.next().unwrap_or("0").parse().ok()?;
    let minute: u32 = tp.next().unwrap_or("0").parse().ok()?;
    // Дробные секунды отбрасываются: у даты 1С разрешение — секунда.
    let sec_text = tp.next().unwrap_or("0");
    let second: u32 = sec_text.split('.').next()?.parse().ok()?;
    crate::BslDate::from_civil(year, month, day, hour, minute, second)
}

/// `ПрочитатьJSON(Чтение[, ВозвращатьСоответствие[, ИменаСвойствСоЗначениямиДата]])`.
///
/// # Errors
///
/// [`RtError::Json`] на битом вводе или на ключе, который не может быть
/// именем поля структуры.
pub fn read_json(
    reader: &BslValue,
    as_map: bool,
    date_names: &[String],
    rt: &mut RuntimeShapes,
) -> RtResult<BslValue> {
    // Первое событие читается здесь же: `ПрочитатьJSON` забирает документ
    // с текущей позиции целиком, и вызывать перед ним `Прочитать()` не
    // требуется.
    let cell = as_reader(reader)?;
    let mut state = cell.borrow_mut();
    if state.parser.is_none() {
        return Err(RtError::TypeError {
            expected: "назначенный источник (УстановитьСтроку/ОткрытьФайл)",
            op: "ПрочитатьJSON",
        });
    }
    // Текущее событие, если на него уже встали ручным `Прочитать()`, иначе
    // следующее: `ПрочитатьJSON` работает в обоих сценариях.
    let first = match state.current.take() {
        Some(e) => Some(e),
        None => state
            .parser
            .as_mut()
            .expect("наличие проверено выше")
            .next_event()?,
    };
    let Some(first) = first else {
        return Ok(BslValue::Undefined);
    };
    let parser = state.parser.as_mut().expect("наличие проверено выше");
    let mut cache = JsonBuildCache::default();
    build_value(first, parser, as_map, date_names, None, rt, &mut cache)
}

/// Проверяет и интернирует имя один раз за разбор документа.
fn json_field_id(
    name: &str,
    rt: &mut RuntimeShapes,
    cache: &mut JsonKeyCache,
) -> RtResult<crate::NameId> {
    if let Some(&id) = cache.get(name) {
        return Ok(id);
    }
    if !is_identifier(name) {
        return Err(RtError::Json(format!(
            "ключ «{name}» не может быть именем свойства структуры"
        )));
    }
    let id = rt.names.intern(name);
    cache.insert(name.into(), id);
    Ok(id)
}

/// Собирает JSON-объект в `Структура`.
///
/// Дублирующееся имя перезаписывает прежний слот, но не меняет
/// его позицию. Это та же семантика, что у последовательных
/// `Структура.Вставить`.
fn build_json_structure(
    keys: Vec<String>,
    values: Vec<BslValue>,
    rt: &mut RuntimeShapes,
    cache: &mut JsonBuildCache,
) -> RtResult<BslValue> {
    // На типовых коротких схемах линейный поиск дешевле ещё
    // одной таблицы и её хэширования. Длинная схема переходит на
    // индекс, чтобы дубликаты не превратили большой объект в O(n²).
    const LINEAR_LOOKUP_LIMIT: usize = 16;

    let mut names = Vec::with_capacity(keys.len());
    let mut slots = Vec::with_capacity(values.len());
    let mut positions: Option<HashMap<crate::NameId, usize>> = None;
    for (key, value) in keys.into_iter().zip(values) {
        let id = json_field_id(&key, rt, &mut cache.keys)?;
        let old_slot = match &positions {
            Some(index) => index.get(&id).copied(),
            None => names.iter().position(|&known| known == id),
        };
        if let Some(slot) = old_slot {
            slots[slot] = value;
        } else {
            let slot = names.len();
            names.push(id);
            slots.push(value);
            if let Some(index) = positions.as_mut() {
                index.insert(id, slot);
            } else if names.len() == LINEAR_LOOKUP_LIMIT + 1 {
                positions = Some(
                    names
                        .iter()
                        .copied()
                        .enumerate()
                        .map(|(slot, name)| (name, slot))
                        .collect(),
                );
            }
        }
    }

    if let Some(shape) = cache.shapes.get(names.as_slice()) {
        return Ok(BslValue::new_structure(shape.clone(), slots));
    }

    let empty = rt.shapes.empty();
    let object = BslValue::new_structure(empty, Vec::new());
    for (&id, value) in names.iter().zip(slots) {
        object.structure_insert(id, value, &mut rt.shapes)?;
    }

    // Словарную структуру не кэшируем: прямое создание с
    // произвольными именами вернуло бы её в таблицу бессрочных форм.
    let built_shape = match &object {
        BslValue::Object(value) => match &**value {
            BslObject::Structure(storage) => match &*storage.borrow() {
                StructureStorage::Shaped { shape, .. } => Some(shape.clone()),
                StructureStorage::Dictionary { .. } => None,
            },
            _ => None,
        },
        _ => None,
    };
    if let Some(shape) = built_shape {
        cache.shapes.insert(names, shape);
    }
    Ok(object)
}

/// Сборка значения из события и продолжения потока.
///
/// `property` — имя свойства, под которым это значение лежит у родителя:
/// по нему решается, превращать ли строку в дату
/// (`ИменаСвойствСоЗначениямиДата`).
fn build_value(
    event: JsonEvent,
    parser: &mut JsonParser,
    as_map: bool,
    date_names: &[String],
    property: Option<&str>,
    rt: &mut RuntimeShapes,
    cache: &mut JsonBuildCache,
) -> RtResult<BslValue> {
    match event {
        JsonEvent::ObjectStart => {
            let mut keys: Vec<String> = Vec::new();
            let mut values: Vec<BslValue> = Vec::new();
            loop {
                let Some(next) = parser.next_event()? else {
                    break;
                };
                match next {
                    JsonEvent::ObjectEnd => break,
                    JsonEvent::PropertyName(name) => {
                        let Some(value_event) = parser.next_event()? else {
                            break;
                        };
                        // Конец объекта на месте значения — пропущенное
                        // значение (`{"а":}`), разборщик к такому
                        // снисходителен.
                        if value_event == JsonEvent::ObjectEnd {
                            break;
                        }
                        let v = build_value(
                            value_event,
                            parser,
                            as_map,
                            date_names,
                            Some(&name),
                            rt,
                            cache,
                        )?;
                        keys.push(name);
                        values.push(v);
                    }
                    _ => break,
                }
            }
            if as_map {
                let map = BslValue::new_map();
                for (k, v) in keys.into_iter().zip(values) {
                    map.map_insert(BslValue::Str(crate::BslString::from_str(&k)), v)?;
                }
                Ok(map)
            } else {
                build_json_structure(keys, values, rt, cache)
            }
        }
        JsonEvent::ArrayStart => {
            let items = BslValue::new_array(Vec::new());
            loop {
                let Some(next) = parser.next_event()? else {
                    break;
                };
                if next == JsonEvent::ArrayEnd {
                    break;
                }
                let v = build_value(next, parser, as_map, date_names, None, rt, cache)?;
                items.push_element(v)?;
            }
            Ok(items)
        }
        JsonEvent::Str(s) => {
            // Дата — только если имя свойства перечислено. JSON типа даты
            // не знает, а гадать по виду строки платформа не берётся, и мы
            // тоже: «2024-03-04» может быть просто строкой.
            let wanted = property.is_some_and(|p| {
                date_names
                    .iter()
                    .any(|n| n.to_uppercase() == p.to_uppercase())
            });
            if wanted {
                if let Some(d) = parse_json_date(&s) {
                    return Ok(BslValue::Date(d));
                }
            }
            Ok(BslValue::Str(crate::BslString::from_str(&s)))
        }
        JsonEvent::Number(n) => Ok(BslValue::Number(n)),
        JsonEvent::Boolean(b) => Ok(BslValue::Boolean(b)),
        // ИЗМЕРЕНО: `null` становится `Неопределено`, а НЕ `Null`.
        JsonEvent::Null => Ok(BslValue::Undefined),
        JsonEvent::PropertyName(s) => Ok(BslValue::Str(crate::BslString::from_str(&s))),
        JsonEvent::ObjectEnd | JsonEvent::ArrayEnd => Ok(BslValue::Undefined),
    }
}

/// `ЗаписатьJSON(Запись, Значение)`.
///
/// # Errors
///
/// [`RtError::TypeError`] на значении, которое сериализовать нечем
/// (`ТаблицаЗначений` и прочие объекты) — измерено, платформа тоже
/// отвергает.
pub fn write_json(writer: &BslValue, value: &BslValue, rt: &RuntimeShapes) -> RtResult<()> {
    let cell = as_writer(writer)?;
    let mut slot = cell.borrow_mut();
    let Some(w) = slot.as_mut() else {
        return Err(RtError::TypeError {
            expected: "назначенный приёмник (УстановитьСтроку/ОткрытьФайл)",
            op: "ЗаписатьJSON",
        });
    };
    serialize(w, value, rt)
}

fn serialize(w: &mut JsonWriter, value: &BslValue, rt: &RuntimeShapes) -> RtResult<()> {
    match value {
        BslValue::Str(_) | BslValue::Number(_) | BslValue::Boolean(_) => w.value(value),
        // `Неопределено` и `Null` отдельным ЗаписатьЗначение платформа не
        // принимает, но в составе сериализуемого значения они обязаны во
        // что-то превращаться — в `null`.
        BslValue::Undefined | BslValue::Null => {
            w.literal("null");
            Ok(())
        }
        // ИЗМЕРЕНО: дата уходит строкой ISO без зоны, включая пустую
        // (`0001-01-01T00:00:00`).
        BslValue::Date(d) => {
            let c = d.to_civil();
            let text = format!(
                "\"{:04}-{:02}-{:02}T{:02}:{:02}:{:02}\"",
                c.year, c.month, c.day, c.hour, c.minute, c.second
            );
            w.literal(&text);
            Ok(())
        }
        BslValue::Object(o) => match &**o {
            BslObject::Array(items) => {
                // Снимок до записи: элемент может оказаться тем же
                // массивом, а `RefCell` вложенного заимствования не
                // переживёт.
                let snapshot: Vec<BslValue> = items.borrow().clone();
                w.begin_array()?;
                for item in &snapshot {
                    serialize(w, item, rt)?;
                }
                w.end_array()
            }
            BslObject::Structure(s) => {
                let entries: Vec<(String, BslValue)> = {
                    let s = s.borrow();
                    (0..s.len())
                        .filter_map(|i| s.entry_at(i))
                        .filter_map(|(id, v)| rt.names.name(id).map(|n| (n.to_string(), v)))
                        .collect()
                };
                w.begin_object()?;
                for (name, v) in &entries {
                    w.property_name(name)?;
                    serialize(w, v, rt)?;
                }
                w.end_object()
            }
            BslObject::Map(data) => {
                let entries: Vec<(BslValue, BslValue)> = {
                    let d = data.borrow();
                    (0..d.len()).filter_map(|i| d.entry_at(i)).collect()
                };
                w.begin_object()?;
                for (k, v) in &entries {
                    // Ключ соответствия может быть любым значением, а имя
                    // свойства JSON — только строкой. Числовой ключ
                    // печатается своим строковым видом, остальное —
                    // ошибка типа.
                    let name = match k {
                        BslValue::Str(s) => s.to_string(),
                        BslValue::Number(n) => n.to_canonical(),
                        _ => {
                            return Err(RtError::TypeError {
                                expected: "Строка или Число в ключе Соответствия",
                                op: "ЗаписатьJSON",
                            })
                        }
                    };
                    w.property_name(&name)?;
                    serialize(w, v, rt)?;
                }
                w.end_object()
            }
            _ => Err(RtError::TypeError {
                expected: "значение, представимое в JSON",
                op: "ЗаписатьJSON",
            }),
        },
        _ => Err(RtError::TypeError {
            expected: "значение, представимое в JSON",
            op: "ЗаписатьJSON",
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn events(text: &str) -> Vec<JsonEvent> {
        let mut p = JsonParser::new(text);
        let mut out = Vec::new();
        while let Some(e) = p.next_event().expect("разбор") {
            out.push(e);
        }
        out
    }

    fn num(s: &str) -> BslNumber {
        BslNumber::parse_canonical(s).unwrap()
    }

    #[test]
    fn json_key_cache_reuses_exact_spelling_and_preserves_case_insensitivity() {
        let mut rt = RuntimeShapes::seeded(Vec::new(), Vec::new());
        let mut cache = JsonKeyCache::new();

        let first = json_field_id("Поле", &mut rt, &mut cache).unwrap();
        let repeated = json_field_id("Поле", &mut rt, &mut cache).unwrap();
        let other_case = json_field_id("поле", &mut rt, &mut cache).unwrap();

        assert_eq!(first, repeated);
        assert_eq!(first, other_case);
        assert_eq!(cache.len(), 2, "кэш различает точные написания");
    }

    #[test]
    fn invalid_json_key_is_not_cached() {
        let mut rt = RuntimeShapes::seeded(Vec::new(), Vec::new());
        let mut cache = JsonKeyCache::new();

        let error = json_field_id("не имя", &mut rt, &mut cache).unwrap_err();

        assert!(matches!(error, RtError::Json(_)));
        assert!(cache.is_empty());
    }

    #[test]
    fn repeated_json_schema_reuses_shape_and_duplicate_overwrites_slot() {
        let mut rt = RuntimeShapes::seeded(Vec::new(), Vec::new());
        let mut cache = JsonBuildCache::default();

        let first = build_json_structure(
            vec!["Поле".into(), "поле".into(), "Второе".into()],
            vec![
                BslValue::Number(num("1")),
                BslValue::Number(num("2")),
                BslValue::Number(num("3")),
            ],
            &mut rt,
            &mut cache,
        )
        .unwrap();
        let (first_shape, first_slots) = match &first {
            BslValue::Object(value) => match &**value {
                BslObject::Structure(storage) => match &*storage.borrow() {
                    StructureStorage::Shaped { shape, slots } => (shape.clone(), slots.clone()),
                    StructureStorage::Dictionary { .. } => panic!("ожидалась форма"),
                },
                _ => panic!("ожидалась структура"),
            },
            _ => panic!("ожидался объект"),
        };
        assert_eq!(first_shape.names.len(), 2);
        assert_eq!(
            first_slots,
            vec![BslValue::Number(num("2")), BslValue::Number(num("3"))]
        );
        assert_eq!(cache.shapes.len(), 1);

        let second = build_json_structure(
            vec!["поле".into(), "ВТОРОЕ".into()],
            vec![BslValue::Number(num("4")), BslValue::Number(num("5"))],
            &mut rt,
            &mut cache,
        )
        .unwrap();
        let second_shape = match &second {
            BslValue::Object(value) => match &**value {
                BslObject::Structure(storage) => match &*storage.borrow() {
                    StructureStorage::Shaped { shape, .. } => shape.clone(),
                    StructureStorage::Dictionary { .. } => panic!("ожидалась форма"),
                },
                _ => panic!("ожидалась структура"),
            },
            _ => panic!("ожидался объект"),
        };
        assert!(Rc::ptr_eq(&first_shape, &second_shape));
    }

    #[test]
    fn oversized_json_schema_stays_dictionary_and_is_not_cached() {
        let mut rt = RuntimeShapes::seeded(Vec::new(), Vec::new());
        let mut cache = JsonBuildCache::default();
        let field_count = crate::MAX_SHAPE_TRANSITIONS as usize + 1;
        let keys = (0..field_count).map(|i| format!("f{i}")).collect();
        let values = (0..field_count)
            .map(|i| BslValue::Number(num(&i.to_string())))
            .collect();

        let object = build_json_structure(keys, values, &mut rt, &mut cache).unwrap();

        let BslValue::Object(value) = &object else {
            panic!("ожидался объект");
        };
        let BslObject::Structure(storage) = &**value else {
            panic!("ожидалась структура");
        };
        assert!(matches!(
            &*storage.borrow(),
            StructureStorage::Dictionary { .. }
        ));
        assert!(cache.shapes.is_empty());
    }

    #[test]
    fn event_sequence_matches_the_platform() {
        // Замер JSON.READ.EVENT_SEQUENCE.
        assert_eq!(
            events(r#"{"а":1,"б":[true,null]}"#),
            vec![
                JsonEvent::ObjectStart,
                JsonEvent::PropertyName("а".into()),
                JsonEvent::Number(num("1")),
                JsonEvent::PropertyName("б".into()),
                JsonEvent::ArrayStart,
                JsonEvent::Boolean(true),
                JsonEvent::Null,
                JsonEvent::ArrayEnd,
                JsonEvent::ObjectEnd,
            ]
        );
    }

    /// Снисходительность разборщика — измерена, а не выдумана: платформа
    /// принимает всё это молча и отдаёт ровно столько же событий.
    #[test]
    fn malformed_input_is_tolerated_the_way_the_platform_tolerates_it() {
        assert_eq!(events(r#"{"а":}"#).len(), 3, "пропущенное значение");
        assert_eq!(events("[1,]").len(), 3, "висячая запятая");
        assert_eq!(events(r#"{"а" 1}"#).len(), 4, "нет двоеточия");
        assert_eq!(events(r#"{"а":1"#).len(), 3, "незакрытый объект");
        // А вот мусор на месте значения — ошибка.
        let mut p = JsonParser::new("нечто");
        assert!(p.next_event().is_err());
    }

    #[test]
    fn escapes_are_unescaped_including_surrogate_pairs() {
        let JsonEvent::Str(s) = &events(r#"["Ё\t\"\\/"]"#)[1] else {
            panic!("ожидалась строка");
        };
        assert_eq!(s, "Ё\t\"\\/");

        // U+1F600, записанный двумя `\u`, обязан собраться в один символ.
        let JsonEvent::Str(s) = &events(r#"["\uD83D\uDE00"]"#)[1] else {
            panic!("ожидалась строка");
        };
        assert_eq!(s, "😀");

        let JsonEvent::Str(s) = &events(r#"["\uD83D"]"#)[1] else {
            panic!("ожидалась строка");
        };
        assert_eq!(s, "\u{fffd}", "одиночный суррогат заменяется");
    }

    #[test]
    fn unescaped_utf8_and_tolerated_truncation_use_the_fast_path() {
        assert_eq!(
            events(r#"["Кириллица 😀"]"#)[1],
            JsonEvent::Str("Кириллица 😀".into())
        );
        assert_eq!(
            events(" \"незакрытая"),
            vec![JsonEvent::Str("незакрытая".into())]
        );
        assert_eq!(
            events("\"abc\\"),
            vec![JsonEvent::Str("abc".into())],
            "оборванный escape проглатывается как и прежде"
        );
    }

    #[test]
    fn parser_keeps_the_bsl_string_snapshot_assigned_to_it() {
        let source = crate::BslString::from_str("[1]");
        let mut parser = JsonParser::from_bsl_string(&source);

        let changed = source.append(&crate::BslString::from_str("мусор"));
        assert_eq!(&*changed.shared_utf8(), "[1]мусор");

        let mut parsed = Vec::new();
        while let Some(event) = parser.next_event().unwrap() {
            parsed.push(event);
        }
        assert_eq!(
            parsed,
            vec![
                JsonEvent::ArrayStart,
                JsonEvent::Number(num("1")),
                JsonEvent::ArrayEnd,
            ]
        );
    }

    #[test]
    fn exponents_are_expanded_exactly() {
        assert_eq!(events("1e3"), vec![JsonEvent::Number(num("1000"))]);
        assert_eq!(events("1.5e-3"), vec![JsonEvent::Number(num("0.0015"))]);
        assert_eq!(events("-2E2"), vec![JsonEvent::Number(num("-200"))]);
        // Точность не теряется через f64: 0.1 + 0.2 тут ни при чём, но
        // экспонента не должна вводить двоичную дробь.
        assert_eq!(
            events("1e-27"),
            vec![JsonEvent::Number(num("0.000000000000000000000000001"))]
        );
    }

    fn write_compact(f: impl FnOnce(&mut JsonWriter) -> RtResult<()>) -> String {
        let mut w = JsonWriter::to_string_target(JsonWriterSettings {
            line_break: JsonLineBreak::None,
            indent: String::new(),
        });
        f(&mut w).expect("запись");
        w.finish().expect("закрытие")
    }

    #[test]
    fn compact_output_has_no_space_after_the_colon() {
        // Замер JSON.WRITE.NO_LINE_BREAKS.
        let s = write_compact(|w| {
            w.begin_object()?;
            w.property_name("а")?;
            w.value(&BslValue::Number(num("1")))?;
            w.end_object()
        });
        assert_eq!(s, r#"{"а":1}"#);
    }

    #[test]
    fn default_settings_break_lines_without_indenting() {
        // Замер JSON.WRITE.DEFAULT_FORMAT.
        let mut w = JsonWriter::to_string_target(JsonWriterSettings::default());
        w.begin_object().unwrap();
        w.property_name("а").unwrap();
        w.value(&BslValue::Number(num("1"))).unwrap();
        w.property_name("б").unwrap();
        w.value(&BslValue::Str(crate::BslString::from_str("текст")))
            .unwrap();
        w.end_object().unwrap();
        assert_eq!(w.finish().unwrap(), "{\n\"а\": 1,\n\"б\": \"текст\"\n}");
    }

    #[test]
    fn indent_repeats_per_depth() {
        // Замер JSON.WRITE.INDENT_NESTED.
        let mut w = JsonWriter::to_string_target(JsonWriterSettings {
            line_break: JsonLineBreak::Auto,
            indent: "__".to_string(),
        });
        w.begin_object().unwrap();
        w.property_name("а").unwrap();
        w.begin_object().unwrap();
        w.property_name("б").unwrap();
        w.value(&BslValue::Number(num("1"))).unwrap();
        w.end_object().unwrap();
        w.end_object().unwrap();
        assert_eq!(w.finish().unwrap(), "{\n__\"а\": {\n____\"б\": 1\n__}\n}");
    }

    #[test]
    fn escaping_follows_the_measured_rules() {
        // Замеры JSON.WRITE.ESCAPES и JSON.WRITE.CONTROL_CHARS.
        let s = write_compact(|w| {
            w.value(&BslValue::Str(crate::BslString::from_str(
                "\"\\/\n\tЁж\u{1}",
            )))
        });
        assert_eq!(s, "\"\\\"\\\\/\\n\\u0009Ёж\\u0001\"");

        let s = write_compact(|w| {
            w.value(&BslValue::Str(crate::BslString::from_str(
                "\r\u{8}\u{c}\u{b}",
            )))
        });
        assert_eq!(s, "\"\\r\\u0008\\u000C\\u000B\"");

        let isolated_surrogate = crate::BslString::from_str("😀").left(1);
        let s = write_compact(|w| w.value(&BslValue::Str(isolated_surrogate)));
        assert_eq!(s, "\"�\"");
    }

    #[test]
    fn bsl_property_name_uses_the_same_escaping_rules() {
        let name = crate::BslString::from_str("имя\"\t😀");
        let s = write_compact(|w| {
            w.begin_object()?;
            w.property_name_bsl(&name)?;
            w.value(&BslValue::Number(num("1")))?;
            w.end_object()
        });

        assert_eq!(s, "{\"имя\\\"\\u0009😀\":1}");
    }

    #[test]
    fn numbers_keep_exact_decimal_form() {
        // Замер JSON.WRITE.NUMBERS: 1/3 уходит всеми 27 знаками, точкой.
        let s = write_compact(|w| {
            w.begin_array()?;
            w.value(&BslValue::Number(num("1.5")))?;
            w.value(&BslValue::Number(num("-2")))?;
            w.value(&BslValue::Number(num("0.333333333333333333333333333")))?;
            w.end_array()
        });
        assert_eq!(s, "[1.5,-2,0.333333333333333333333333333]");
    }

    /// ИЗМЕРЕНО, и это единственное место, где платформа ведёт себя
    /// непоследовательно: `Неопределено` пишется как `null`, а `Null` и
    /// `Дата` отвергаются. Три отдельные пробы, по одному вызову в каждой.
    #[test]
    fn undefined_is_written_as_null_while_null_and_date_are_rejected() {
        let s = write_compact(|w| w.value(&BslValue::Undefined));
        assert_eq!(s, "null");

        for v in [
            BslValue::Null,
            BslValue::Date(crate::BslDate::from_seconds(0).unwrap()),
        ] {
            let mut w = JsonWriter::to_string_target(JsonWriterSettings::default());
            assert!(w.value(&v).is_err(), "{v:?} не должно записываться");
        }
    }

    #[test]
    fn a_value_in_an_object_without_a_name_is_rejected() {
        let mut w = JsonWriter::to_string_target(JsonWriterSettings::default());
        w.begin_object().unwrap();
        assert!(w.value(&BslValue::Number(num("1"))).is_err());
    }
}
