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

use bsl_rt::{BslNumber, BslValue, RtError, RtResult};

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

    fn from_bsl_string(src: &bsl_rt::BslString) -> Self {
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
                            return Err(bad(&format!("неизвестное экранирование «\\{other}»")));
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

/// `ФорматДатыJSON` в терминах СЕРИАЛИЗАЦИИ, а не события разбора (тот —
/// [`JsonEvent`]) — отдельный тип от `EnumValue`, потому что здесь удобнее
/// работать `match`ем без соседних членов `ТипЗначенияJSON`/`ПереносСтрокJSON`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsonDateFormat {
    Iso,
    JavaScript,
    Microsoft,
}

impl JsonDateFormat {
    fn from_enum_value(v: EnumValue) -> Option<Self> {
        match v {
            EnumValue::DateFormatIso => Some(Self::Iso),
            EnumValue::DateFormatJavaScript => Some(Self::JavaScript),
            EnumValue::DateFormatMicrosoft => Some(Self::Microsoft),
            _ => None,
        }
    }

    fn to_enum_value(self) -> EnumValue {
        match self {
            Self::Iso => EnumValue::DateFormatIso,
            Self::JavaScript => EnumValue::DateFormatJavaScript,
            Self::Microsoft => EnumValue::DateFormatMicrosoft,
        }
    }
}

/// `ВариантЗаписиДатыJSON` — какой момент означает записанная дата:
/// наивное локальное время машины, то же самое со смещением от UTC
/// в тексте, либо перевод в UTC.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsonDateWritingVariant {
    Local,
    LocalOffset,
    Universal,
}

impl JsonDateWritingVariant {
    fn from_enum_value(v: EnumValue) -> Option<Self> {
        match v {
            EnumValue::DateVariantLocal => Some(Self::Local),
            EnumValue::DateVariantLocalOffset => Some(Self::LocalOffset),
            EnumValue::DateVariantUniversal => Some(Self::Universal),
            _ => None,
        }
    }

    fn to_enum_value(self) -> EnumValue {
        match self {
            Self::Local => EnumValue::DateVariantLocal,
            Self::LocalOffset => EnumValue::DateVariantLocalOffset,
            Self::Universal => EnumValue::DateVariantUniversal,
        }
    }
}

impl Default for JsonDateWritingVariant {
    /// ИЗМЕРЕНО (`JSON.SETTINGS.DATE_VARIANT_DEFAULT`): умолчание
    /// `НастройкиСериализацииJSON.ВариантЗаписиДаты` — `ЛокальнаяДата`.
    fn default() -> Self {
        Self::Local
    }
}

/// `НастройкиСериализацииJSON` — третий аргумент `ЗаписатьJSON`. В отличие
/// от [`JsonWriterSettings`] (форматирование ТЕКСТА: переносы строк,
/// отступ) управляет тем, КАК сериализуются даты и массивы внутри
/// значения.
#[derive(Debug, Clone, PartialEq)]
pub struct JsonSerializerSettings {
    pub date_format: JsonDateFormat,
    pub date_variant: JsonDateWritingVariant,
    pub arrays_as_objects: bool,
}

impl Default for JsonSerializerSettings {
    fn default() -> Self {
        JsonSerializerSettings {
            date_format: JsonDateFormat::Iso,
            date_variant: JsonDateWritingVariant::default(),
            arrays_as_objects: false,
        }
    }
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
    /// `ЗаписьJSON.ПроверятьСтруктуру`. Умолчание `true` — ИЗМЕРЕНО
    /// (`JSON.WRITE.CHECK_STRUCTURE_DEFAULT`). Свойство читается и
    /// пишется, но ни на одну из известных проверок структуры документа
    /// (`end_object`/`end_array`/`begin_property_name`/`value` ниже) не
    /// влияет — ИЗМЕРЕНО, что все они безусловны что при `Истина`, что при
    /// `Ложь`. `НЕ ИЗМЕРЕНО(JSON.WRITE.CHECK_STRUCTURE_OFF)`: что тогда
    /// вообще отключает `Ложь` (если хоть что-то) — открытый вопрос,
    /// пробуются новые кандидаты (см. реестр).
    check_structure: bool,
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
            check_structure: true,
        }
    }

    pub fn to_file(path: std::path::PathBuf, settings: JsonWriterSettings) -> Self {
        let mut w = Self::to_string_target(settings);
        w.path = Some(path);
        w
    }

    pub fn check_structure(&self) -> bool {
        self.check_structure
    }

    pub fn set_check_structure(&mut self, v: bool) {
        self.check_structure = v;
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
    ///
    /// ИЗМЕРЕНО: эта проверка тоже БЕЗУСЛОВНАЯ — `ПроверятьСтруктуру = Ложь`
    /// её не снимает (см. `JSON.WRITE.CHECK_STRUCTURE_OFF`: пооперационная
    /// проба дала «ошибка» и здесь, и у `end_array`/`begin_property_name`,
    /// то есть ни одна из известных проверок настройкой не управляется).
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
    /// [`RtError::Json`], если открытого массива нет — БЕЗУСЛОВНО, см.
    /// `end_object`.
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
    ///
    /// ИЗМЕРЕНО: проверка контекста БЕЗУСЛОВНАЯ, см. `end_object`.
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
    fn property_name_bsl(&mut self, name: &bsl_rt::BslString) -> RtResult<()> {
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
    /// [`RtError::Json`], если значение пишется в объект без имени
    /// свойства, либо тип значения записать нечем.
    ///
    /// ИЗМЕРЕНО: эта проверка, как и все прочие проверки структуры
    /// документа (`end_object`/`end_array`/`begin_property_name`), —
    /// БЕЗУСЛОВНАЯ; `ПроверятьСтруктуру = Ложь` не снимает ни одну из них
    /// (`JSON.WRITE.CHECK_STRUCTURE_OFF`, пооперационная проба). Что же
    /// именно отключает эта настройка (если хоть что-то), остаётся
    /// открытым вопросом — свойство при этом читаемо и записываемо.
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
fn escape_bsl_string_into(out: &mut String, s: &bsl_rt::BslString) {
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

use std::cell::RefCell;

use bsl_rt::{
    Arity, BslObject, CallContext, ConstructorCode, ConstructorDescriptor, EnumValue, FunctionCode,
    FunctionDescriptor, FunctionKind, LibraryDescriptor, MethodCode, MethodDescriptor,
    ObjectDowncast, ObjectProtocol, StructureStorage, TypeDescriptor, TypeId,
    local_date_from_utc_seconds, pseudo_unix_seconds,
};

#[derive(Debug, Default)]
struct JsonReaderState {
    parser: Option<JsonParser>,
    current: Option<JsonEvent>,
}

#[derive(Debug, Default)]
struct JsonReaderObject {
    state: Rc<RefCell<JsonReaderState>>,
}

#[derive(Debug, Default)]
struct JsonWriterObject {
    /// Состояние за `Rc<RefCell>`: обработчики достают его из получателя
    /// ссылкой через `as_writer`, обёртка значения не пересобирается.
    writer: Rc<RefCell<Option<JsonWriter>>>,
}

#[derive(Debug, Clone)]
struct JsonWriterSettingsObject(JsonWriterSettings);

#[derive(Debug, Default)]
struct JsonSerializerSettingsObject(Rc<RefCell<JsonSerializerSettings>>);

static READER_TYPE: TypeDescriptor = TypeDescriptor {
    package: env!("CARGO_PKG_NAME"),
    name: "ЧтениеJSON",
    legacy_type_id: Some(TypeId::JsonReader),
};
static WRITER_TYPE: TypeDescriptor = TypeDescriptor {
    package: env!("CARGO_PKG_NAME"),
    name: "ЗаписьJSON",
    legacy_type_id: Some(TypeId::JsonWriter),
};
static WRITER_SETTINGS_TYPE: TypeDescriptor = TypeDescriptor {
    package: env!("CARGO_PKG_NAME"),
    name: "ПараметрыЗаписиJSON",
    legacy_type_id: Some(TypeId::JsonWriterSettings),
};
static SERIALIZER_SETTINGS_TYPE: TypeDescriptor = TypeDescriptor {
    package: env!("CARGO_PKG_NAME"),
    name: "НастройкиСериализацииJSON",
    legacy_type_id: Some(TypeId::JsonSerializerSettings),
};

fn as_reader(v: &dyn ObjectProtocol) -> RtResult<&std::cell::RefCell<JsonReaderState>> {
    v.downcast_ref::<JsonReaderObject>()
        .map(|reader| reader.state.as_ref())
        .ok_or_else(|| not_applicable(v, "ЧтениеJSON"))
}

fn as_writer(v: &dyn ObjectProtocol) -> RtResult<&std::cell::RefCell<Option<JsonWriter>>> {
    v.downcast_ref::<JsonWriterObject>()
        .map(|writer| writer.writer.as_ref())
        .ok_or_else(|| not_applicable(v, "ЗаписьJSON"))
}

fn not_applicable(v: &dyn ObjectProtocol, _expected: &str) -> RtError {
    RtError::MethodNotApplicable {
        method: "метод JSON",
        receiver: v.type_descriptor().name,
    }
}

/// Объект за значением аргумента: не-объект получает ту же ошибку «метод
/// JSON не применим», что и объект чужого типа.
fn arg_object(v: &BslValue) -> RtResult<&dyn ObjectProtocol> {
    v.object_ref()
        .map(bsl_rt::ObjectRef::as_dyn)
        .ok_or_else(|| RtError::MethodNotApplicable {
            method: "метод JSON",
            receiver: v.type_name(),
        })
}

/// Настройки из аргумента `УстановитьСтроку([Параметры])`. Отсутствующий
/// аргумент — умолчание (переносы есть, отступа нет).
fn settings_from(arg: Option<&BslValue>) -> RtResult<JsonWriterSettings> {
    match arg {
        None | Some(BslValue::Undefined) => Ok(JsonWriterSettings::default()),
        Some(value) => value
            .object_ref()
            .and_then(|object| object.downcast_ref::<JsonWriterSettingsObject>())
            .map(|settings| settings.0.clone())
            .ok_or(RtError::TypeError {
                expected: "ПараметрыЗаписиJSON",
                op: "УстановитьСтроку",
            }),
    }
}

/// Настройки из третьего аргумента `ЗаписатьJSON(Запись, Значение,
/// [Настройки])`. Отсутствующий аргумент — умолчания
/// [`JsonSerializerSettings::default`].
///
/// # Errors
///
/// [`RtError::TypeError`], если аргумент задан и не `НастройкиСериализацииJSON`.
pub fn serializer_settings_from(arg: Option<&BslValue>) -> RtResult<JsonSerializerSettings> {
    match arg {
        None | Some(BslValue::Undefined) => Ok(JsonSerializerSettings::default()),
        Some(value) => value
            .object_ref()
            .and_then(|object| object.downcast_ref::<JsonSerializerSettingsObject>())
            .map(|settings| settings.0.borrow().clone())
            .ok_or(RtError::TypeError {
                expected: "НастройкиСериализацииJSON",
                op: "ЗаписатьJSON",
            }),
    }
}

/// `НастройкиСериализацииJSON.<поле>` (чтение) — три свойства, все
/// читаемые и записываемые (см. `set_serializer_setting`).
///
/// Все шесть написаний (рус./англ. на каждое из трёх свойств) — ИЗМЕРЕНО
/// (`JSON.SETTINGS.PROPERTY_NAMES`): `ФорматСериализацииДаты` (с «ы»!) /
/// `DateSerializationFormat`, `ВариантЗаписиДаты` / `DateWritingVariant`,
/// `СериализовыватьМассивыКакОбъекты` / `SerializeArraysAsObjects`. Статья
/// 16.2.3.2 приводит `ФорматСериализацииДат` (без «ы») — это ОПЕЧАТКА
/// статьи, живая 8.3.27 такое имя отвергает («Поле объекта не обнаружено»,
/// снято прогоном фикстуры `json-dates`), поэтому здесь оно НЕ принимается.
///
/// # Errors
///
/// [`RtError::UnknownColumn`] на неизвестном имени; [`RtError::NotAnObject`],
/// если получатель не `НастройкиСериализацииJSON`.
pub fn get_serializer_setting(obj: &dyn ObjectProtocol, name: &str) -> RtResult<BslValue> {
    let settings = obj
        .downcast_ref::<JsonSerializerSettingsObject>()
        .ok_or(RtError::NotAnObject)?;
    let s = settings.0.borrow();
    if name.eq_ignore_ascii_case("ФорматСериализацииДаты")
        || name.eq_ignore_ascii_case("DateSerializationFormat")
    {
        Ok(BslValue::Enum(s.date_format.to_enum_value()))
    } else if name.eq_ignore_ascii_case("ВариантЗаписиДаты")
        || name.eq_ignore_ascii_case("DateWritingVariant")
    {
        Ok(BslValue::Enum(s.date_variant.to_enum_value()))
    } else if name.eq_ignore_ascii_case("СериализовыватьМассивыКакОбъекты")
        || name.eq_ignore_ascii_case("SerializeArraysAsObjects")
    {
        Ok(BslValue::Boolean(s.arrays_as_objects))
    } else {
        Err(RtError::UnknownColumn(name.to_string()))
    }
}

/// `НастройкиСериализацииJSON.<поле> = Значение` (запись).
///
/// # Errors
///
/// [`RtError::TypeError`] на значении не того типа; [`RtError::UnknownColumn`]
/// на неизвестном имени; [`RtError::NotAnObject`], если получатель не
/// `НастройкиСериализацииJSON`.
pub fn set_serializer_setting(obj: &dyn ObjectProtocol, name: &str, val: BslValue) -> RtResult<()> {
    let settings = obj
        .downcast_ref::<JsonSerializerSettingsObject>()
        .ok_or(RtError::NotAnObject)?;
    if name.eq_ignore_ascii_case("ФорматСериализацииДаты")
        || name.eq_ignore_ascii_case("DateSerializationFormat")
    {
        let BslValue::Enum(e) = val else {
            return Err(RtError::TypeError {
                expected: "ФорматДатыJSON",
                op: "ФорматСериализацииДаты",
            });
        };
        let format = JsonDateFormat::from_enum_value(e).ok_or(RtError::TypeError {
            expected: "ФорматДатыJSON",
            op: "ФорматСериализацииДаты",
        })?;
        settings.0.borrow_mut().date_format = format;
        Ok(())
    } else if name.eq_ignore_ascii_case("ВариантЗаписиДаты")
        || name.eq_ignore_ascii_case("DateWritingVariant")
    {
        let BslValue::Enum(e) = val else {
            return Err(RtError::TypeError {
                expected: "ВариантЗаписиДатыJSON",
                op: "ВариантЗаписиДаты",
            });
        };
        let variant = JsonDateWritingVariant::from_enum_value(e).ok_or(RtError::TypeError {
            expected: "ВариантЗаписиДатыJSON",
            op: "ВариантЗаписиДаты",
        })?;
        settings.0.borrow_mut().date_variant = variant;
        Ok(())
    } else if name.eq_ignore_ascii_case("СериализовыватьМассивыКакОбъекты")
        || name.eq_ignore_ascii_case("SerializeArraysAsObjects")
    {
        let BslValue::Boolean(b) = val else {
            return Err(RtError::TypeError {
                expected: "Булево",
                op: "СериализовыватьМассивыКакОбъекты",
            });
        };
        settings.0.borrow_mut().arrays_as_objects = b;
        Ok(())
    } else {
        Err(RtError::UnknownColumn(name.to_string()))
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
pub fn set_string(obj: &dyn ObjectProtocol, args: &[BslValue]) -> RtResult<()> {
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
pub fn open_file(obj: &dyn ObjectProtocol, args: &[BslValue]) -> RtResult<()> {
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
pub fn read(obj: &dyn ObjectProtocol) -> RtResult<bool> {
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
pub fn skip(obj: &dyn ObjectProtocol) -> RtResult<()> {
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
pub fn current_value_type(obj: &dyn ObjectProtocol) -> RtResult<BslValue> {
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
pub fn current_value(obj: &dyn ObjectProtocol) -> RtResult<BslValue> {
    let reader = as_reader(obj)?;
    let state = reader.borrow();
    match &state.current {
        Some(JsonEvent::PropertyName(s)) | Some(JsonEvent::Str(s)) => {
            Ok(BslValue::Str(bsl_rt::BslString::from_str(s)))
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
fn with_writer_cell<T>(
    cell: &RefCell<Option<JsonWriter>>,
    f: impl FnOnce(&mut JsonWriter) -> RtResult<T>,
) -> RtResult<T> {
    let mut slot = cell.borrow_mut();
    let Some(writer) = slot.as_mut() else {
        return Err(RtError::TypeError {
            expected: "назначенный приёмник (УстановитьСтроку/ОткрытьФайл)",
            op: "запись JSON",
        });
    };
    f(writer)
}

fn with_writer<T>(
    obj: &dyn ObjectProtocol,
    f: impl FnOnce(&mut JsonWriter) -> RtResult<T>,
) -> RtResult<T> {
    with_writer_cell(as_writer(obj)?, f)
}

fn close_writer_cell(cell: &RefCell<Option<JsonWriter>>) -> RtResult<BslValue> {
    let mut slot = cell.borrow_mut();
    let Some(writer) = slot.as_mut() else {
        return Ok(BslValue::Str(bsl_rt::BslString::from_str("")));
    };
    let text = writer.finish()?;
    *slot = None;
    Ok(BslValue::Str(bsl_rt::BslString::from_utf8_string(text)))
}

/// # Errors
///
/// Ошибку структуры документа либо отсутствия приёмника.
pub fn write_start_object(obj: &dyn ObjectProtocol) -> RtResult<()> {
    with_writer(obj, JsonWriter::begin_object)
}

/// # Errors
///
/// См. [`write_start_object`].
pub fn write_end_object(obj: &dyn ObjectProtocol) -> RtResult<()> {
    with_writer(obj, JsonWriter::end_object)
}

/// # Errors
///
/// См. [`write_start_object`].
pub fn write_start_array(obj: &dyn ObjectProtocol) -> RtResult<()> {
    with_writer(obj, JsonWriter::begin_array)
}

/// # Errors
///
/// См. [`write_start_object`].
pub fn write_end_array(obj: &dyn ObjectProtocol) -> RtResult<()> {
    with_writer(obj, JsonWriter::end_array)
}

/// # Errors
///
/// [`RtError::TypeError`], если имя не строка.
pub fn write_property_name(obj: &dyn ObjectProtocol, args: &[BslValue]) -> RtResult<()> {
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
pub fn write_value(obj: &dyn ObjectProtocol, args: &[BslValue]) -> RtResult<()> {
    let value = args.first().cloned().unwrap_or(BslValue::Undefined);
    with_writer(obj, |w| w.value(&value))
}

/// `ЗаписьJSON.Закрыть()` -> текст (для строкового приёмника) либо пустая
/// строка (для файлового).
///
/// # Errors
///
/// [`RtError::IoError`] при неудачной записи файла.
pub fn close_writer(obj: &dyn ObjectProtocol) -> RtResult<BslValue> {
    close_writer_cell(as_writer(obj)?)
}

/// Получатель — `ЗаписьJSON`? Нужно `BslValue::close_object`, чтобы
/// развести одноимённый `Закрыть` у двух разных объектов.
pub fn is_json_writer(v: &BslValue) -> bool {
    v.object_ref()
        .and_then(|object| object.downcast_ref::<JsonWriterObject>())
        .is_some()
}

/// `ЗаписьJSON.ПроверятьСтруктуру` (чтение).
///
/// # Errors
///
/// [`RtError::TypeError`], если приёмник ещё не назначен — то же условие,
/// что и у остальных методов записи.
pub fn get_check_structure(obj: &dyn ObjectProtocol) -> RtResult<BslValue> {
    with_writer(obj, |w| Ok(BslValue::Boolean(w.check_structure())))
}

/// `ЗаписьJSON.ПроверятьСтруктуру` (запись).
///
/// # Errors
///
/// [`RtError::TypeError`], если значение не `Булево` либо приёмник ещё не
/// назначен.
pub fn set_check_structure(obj: &dyn ObjectProtocol, val: BslValue) -> RtResult<()> {
    let BslValue::Boolean(b) = val else {
        return Err(RtError::TypeError {
            expected: "Булево",
            op: "ПроверятьСтруктуру",
        });
    };
    with_writer(obj, |w| {
        w.set_check_structure(b);
        Ok(())
    })
}

fn method_is(name: &str, russian: &str, english: &str) -> bool {
    name.eq_ignore_ascii_case(russian) || name.eq_ignore_ascii_case(english)
}

fn exact_method_arity(_name: &str, arguments: &[BslValue], count: usize) -> RtResult<()> {
    if arguments.len() == count {
        Ok(())
    } else {
        Err(RtError::MethodNotApplicable {
            method: "метод JSON",
            receiver: "JSON",
        })
    }
}

/// Создаёт ненастроенный `ЧтениеJSON`.
pub fn new_json_reader() -> BslValue {
    BslValue::new_object(JsonReaderObject {
        state: Rc::new(RefCell::new(JsonReaderState::default())),
    })
}

/// Создаёт ненастроенный `ЗаписьJSON`.
pub fn new_json_writer() -> BslValue {
    BslValue::new_object(JsonWriterObject {
        writer: Rc::new(RefCell::new(None)),
    })
}

/// Создаёт `ПараметрыЗаписиJSON` из нуля, одного или двух аргументов.
///
/// # Errors
///
/// Ошибка арности или [`RtError::TypeError`], если перенос строк
/// или строка отступа имеют неверный тип.
pub fn new_json_writer_settings(arguments: &[BslValue]) -> RtResult<BslValue> {
    if arguments.len() > 2 {
        return Err(RtError::MethodNotApplicable {
            method: "Новый ПараметрыЗаписиJSON",
            receiver: WRITER_SETTINGS_TYPE.name,
        });
    }
    let line_break = arguments.first().unwrap_or(&BslValue::Undefined);
    let indent = arguments.get(1).unwrap_or(&BslValue::Undefined);
    let line_break = match line_break {
        BslValue::Undefined => JsonLineBreak::Auto,
        BslValue::Enum(EnumValue::LineBreakNone) => JsonLineBreak::None,
        BslValue::Enum(EnumValue::LineBreakAuto) => JsonLineBreak::Auto,
        BslValue::Enum(EnumValue::LineBreakWindows) => JsonLineBreak::Windows,
        BslValue::Enum(EnumValue::LineBreakUnix) => JsonLineBreak::Unix,
        _ => {
            return Err(RtError::TypeError {
                expected: "ПереносСтрокJSON",
                op: "Новый ПараметрыЗаписиJSON",
            });
        }
    };
    let indent = match indent {
        BslValue::Undefined => String::new(),
        BslValue::Str(value) => value.to_string(),
        _ => {
            return Err(RtError::TypeError {
                expected: "Строка",
                op: "Новый ПараметрыЗаписиJSON",
            });
        }
    };
    Ok(BslValue::new_object(JsonWriterSettingsObject(
        JsonWriterSettings { line_break, indent },
    )))
}

/// Создаёт `НастройкиСериализацииJSON` с платформенными умолчаниями.
pub fn new_json_serializer_settings() -> BslValue {
    BslValue::new_object(JsonSerializerSettingsObject(Rc::new(RefCell::new(
        JsonSerializerSettings::default(),
    ))))
}

impl ObjectProtocol for JsonReaderObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &READER_TYPE
    }

    fn get_property(&self, name: &str, _context: &mut CallContext<'_>) -> RtResult<BslValue> {
        if method_is(name, "ТипТекущегоЗначения", "CurrentValueType") {
            current_value_type(self.as_dyn())
        } else if method_is(name, "ТекущееЗначение", "CurrentValue") {
            current_value(self.as_dyn())
        } else {
            Err(RtError::UnknownColumn(name.to_string()))
        }
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        READER_METHODS
    }

    fn is_filled(&self) -> RtResult<bool> {
        Ok(true)
    }
}

// Обработчики статической таблицы читателя: получатель приходит от
// вызывающего (VM отдаёт исходное значение — без пересборки обёртки на
// каждый вызов), проверки арности — прежние, из веток `method_is`.
fn reader_set_string(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    exact_method_arity("УстановитьСтроку", arguments, 1)?;
    set_string(receiver, arguments).map(|()| BslValue::Undefined)
}

fn reader_open_file(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    exact_method_arity("ОткрытьФайл", arguments, 1)?;
    open_file(receiver, arguments).map(|()| BslValue::Undefined)
}

fn reader_read(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    exact_method_arity("Прочитать", arguments, 0)?;
    read(receiver).map(BslValue::Boolean)
}

fn reader_skip(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    exact_method_arity("Пропустить", arguments, 0)?;
    skip(receiver).map(|()| BslValue::Undefined)
}

const READER_METHODS: &[MethodDescriptor] = &[
    MethodDescriptor {
        code: MethodCode::new(1),
        names: &["УстановитьСтроку", "SetString"],
        call: reader_set_string,
    },
    MethodDescriptor {
        code: MethodCode::new(2),
        names: &["ОткрытьФайл", "OpenFile"],
        call: reader_open_file,
    },
    MethodDescriptor {
        code: MethodCode::new(3),
        names: &["Прочитать", "Read"],
        call: reader_read,
    },
    MethodDescriptor {
        code: MethodCode::new(4),
        names: &["Пропустить", "Skip"],
        call: reader_skip,
    },
];

// Обработчики статической таблицы писателя: тела — прежние ветки
// `invoke`, состояние достаётся из получателя через `as_writer`.
fn writer_set_string(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    if arguments.len() > 1 {
        return Err(RtError::MethodNotApplicable {
            method: "метод JSON",
            receiver: WRITER_TYPE.name,
        });
    }
    *as_writer(receiver)?.borrow_mut() = Some(JsonWriter::to_string_target(settings_from(
        arguments.first(),
    )?));
    Ok(BslValue::Undefined)
}

fn writer_open_file(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    if !(1..=2).contains(&arguments.len()) {
        return Err(RtError::MethodNotApplicable {
            method: "метод JSON",
            receiver: WRITER_TYPE.name,
        });
    }
    let BslValue::Str(path) = &arguments[0] else {
        return Err(RtError::TypeError {
            expected: "Строка",
            op: "ОткрытьФайл",
        });
    };
    *as_writer(receiver)?.borrow_mut() = Some(JsonWriter::to_file(
        std::path::PathBuf::from(path.to_string()),
        settings_from(arguments.get(1))?,
    ));
    Ok(BslValue::Undefined)
}

fn writer_close(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    exact_method_arity("Закрыть", arguments, 0)?;
    close_writer_cell(as_writer(receiver)?)
}

fn writer_start_object(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    exact_method_arity("ЗаписатьНачалоОбъекта", arguments, 0)?;
    with_writer_cell(as_writer(receiver)?, JsonWriter::begin_object).map(|()| BslValue::Undefined)
}

fn writer_end_object(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    exact_method_arity("ЗаписатьКонецОбъекта", arguments, 0)?;
    with_writer_cell(as_writer(receiver)?, JsonWriter::end_object).map(|()| BslValue::Undefined)
}

fn writer_start_array(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    exact_method_arity("ЗаписатьНачалоМассива", arguments, 0)?;
    with_writer_cell(as_writer(receiver)?, JsonWriter::begin_array).map(|()| BslValue::Undefined)
}

fn writer_end_array(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    exact_method_arity("ЗаписатьКонецМассива", arguments, 0)?;
    with_writer_cell(as_writer(receiver)?, JsonWriter::end_array).map(|()| BslValue::Undefined)
}

fn writer_property_name(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    exact_method_arity("ЗаписатьИмяСвойства", arguments, 1)?;
    let BslValue::Str(property) = &arguments[0] else {
        return Err(RtError::TypeError {
            expected: "Строка",
            op: "ЗаписатьИмяСвойства",
        });
    };
    with_writer_cell(as_writer(receiver)?, |writer| {
        writer.property_name_bsl(property)
    })
    .map(|()| BslValue::Undefined)
}

fn writer_value(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    exact_method_arity("ЗаписатьЗначение", arguments, 1)?;
    with_writer_cell(as_writer(receiver)?, |writer| writer.value(&arguments[0]))
        .map(|()| BslValue::Undefined)
}

const WRITER_METHODS: &[MethodDescriptor] = &[
    MethodDescriptor {
        code: MethodCode::new(1),
        names: &["УстановитьСтроку", "SetString"],
        call: writer_set_string,
    },
    MethodDescriptor {
        code: MethodCode::new(2),
        names: &["ОткрытьФайл", "OpenFile"],
        call: writer_open_file,
    },
    MethodDescriptor {
        code: MethodCode::new(3),
        names: &["Закрыть", "Close"],
        call: writer_close,
    },
    MethodDescriptor {
        code: MethodCode::new(4),
        names: &["ЗаписатьНачалоОбъекта", "WriteStartObject"],
        call: writer_start_object,
    },
    MethodDescriptor {
        code: MethodCode::new(5),
        names: &["ЗаписатьКонецОбъекта", "WriteEndObject"],
        call: writer_end_object,
    },
    MethodDescriptor {
        code: MethodCode::new(6),
        names: &["ЗаписатьНачалоМассива", "WriteStartArray"],
        call: writer_start_array,
    },
    MethodDescriptor {
        code: MethodCode::new(7),
        names: &["ЗаписатьКонецМассива", "WriteEndArray"],
        call: writer_end_array,
    },
    MethodDescriptor {
        code: MethodCode::new(8),
        names: &["ЗаписатьИмяСвойства", "WritePropertyName"],
        call: writer_property_name,
    },
    MethodDescriptor {
        code: MethodCode::new(9),
        names: &["ЗаписатьЗначение", "WriteValue"],
        call: writer_value,
    },
];

impl ObjectProtocol for JsonWriterObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &WRITER_TYPE
    }

    fn get_property(&self, name: &str, _context: &mut CallContext<'_>) -> RtResult<BslValue> {
        if method_is(name, "ПроверятьСтруктуру", "CheckStructure") {
            with_writer_cell(&self.writer, |writer| {
                Ok(BslValue::Boolean(writer.check_structure()))
            })
        } else {
            Err(RtError::UnknownColumn(name.to_string()))
        }
    }

    fn set_property(
        &self,
        name: &str,
        value: BslValue,
        _context: &mut CallContext<'_>,
    ) -> RtResult<()> {
        if method_is(name, "ПроверятьСтруктуру", "CheckStructure") {
            let BslValue::Boolean(check) = value else {
                return Err(RtError::TypeError {
                    expected: "Булево",
                    op: "ПроверятьСтруктуру",
                });
            };
            with_writer_cell(&self.writer, |writer| {
                writer.set_check_structure(check);
                Ok(())
            })
        } else {
            Err(RtError::UnknownColumn(name.to_string()))
        }
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        WRITER_METHODS
    }

    fn is_filled(&self) -> RtResult<bool> {
        Ok(true)
    }
}

impl ObjectProtocol for JsonWriterSettingsObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &WRITER_SETTINGS_TYPE
    }

    fn is_filled(&self) -> RtResult<bool> {
        Ok(true)
    }
}

impl ObjectProtocol for JsonSerializerSettingsObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &SERIALIZER_SETTINGS_TYPE
    }

    fn get_property(&self, name: &str, _context: &mut CallContext<'_>) -> RtResult<BslValue> {
        get_serializer_setting(self.as_dyn(), name)
    }

    fn set_property(
        &self,
        name: &str,
        value: BslValue,
        _context: &mut CallContext<'_>,
    ) -> RtResult<()> {
        set_serializer_setting(self.as_dyn(), name, value)
    }

    fn is_filled(&self) -> RtResult<bool> {
        Ok(true)
    }
}

fn callback_name(
    name_arg: Option<&BslValue>,
    module_arg: Option<&BslValue>,
    op: &'static str,
) -> RtResult<Option<String>> {
    let name = match name_arg {
        None | Some(BslValue::Undefined) => return Ok(None),
        Some(BslValue::Str(value)) => value.to_string(),
        Some(_) => {
            return Err(RtError::TypeError {
                expected: "Строка",
                op,
            });
        }
    };
    if name.is_empty() || matches!(module_arg, None | Some(BslValue::Undefined)) {
        Ok(None)
    } else {
        Ok(Some(name))
    }
}

fn name_list_arg(
    argument: Option<&BslValue>,
    runtime: &RuntimeShapes,
    op: &'static str,
) -> RtResult<Vec<String>> {
    let Some(list) = argument else {
        return Ok(Vec::new());
    };
    if matches!(list, BslValue::Undefined) {
        return Ok(Vec::new());
    }
    let length = list.collection_len().map_err(|_| RtError::TypeError {
        expected: "Массив",
        op,
    })?;
    let mut names = Vec::with_capacity(length);
    for index in 0..length {
        let item = list.get_index(&BslValue::number_from_i64(index as i64), &runtime.names)?;
        if let BslValue::Str(value) = item {
            names.push(value.to_string());
        }
    }
    Ok(names)
}

/// Общая реализация `ПрочитатьJSON` для нового и legacy-байткода.
///
/// # Errors
///
/// Ошибка типа, разбора JSON или вызова функции восстановления.
pub fn read_json_builtin(
    arguments: &[BslValue],
    runtime: &mut RuntimeShapes,
    call: Option<JsonCallByName<'_>>,
) -> RtResult<BslValue> {
    let as_map = match arguments.get(1) {
        None | Some(BslValue::Undefined) => false,
        Some(BslValue::Boolean(value)) => *value,
        Some(_) => {
            return Err(RtError::TypeError {
                expected: "Булево",
                op: "ПрочитатьJSON(ВозвращатьСоответствие)",
            });
        }
    };
    let date_names = name_list_arg(
        arguments.get(2),
        runtime,
        "ПрочитатьJSON(ИменаСвойствСоЗначениямиДата)",
    )?;
    let date_format =
        optional_date_format_from_arg(arguments.get(3), "ПрочитатьJSON(ОжидаемыйФорматДаты)")?;
    let name = callback_name(
        arguments.get(4),
        arguments.get(5),
        "ПрочитатьJSON(ИмяФункцииВосстановления)",
    )?;
    let restore = match (name, call) {
        (None, _) => None,
        (Some(name), Some(call)) => Some(JsonRestoreFn {
            name,
            extra: arguments.get(6).cloned().unwrap_or(BslValue::Undefined),
            property_names: name_list_arg(
                arguments.get(7),
                runtime,
                "ПрочитатьJSON(ИменаСвойствДляФункцииВосстановления)",
            )?,
            call,
        }),
        (Some(_), None) => {
            return Err(RtError::Json(
                "ПрочитатьJSON: функция восстановления требует исполняющей VM".to_string(),
            ));
        }
    };
    read_json(
        &arguments[0],
        as_map,
        &date_names,
        date_format,
        restore,
        runtime,
    )
}

/// Общая реализация `ЗаписатьJSON` для нового и legacy-байткода.
///
/// # Errors
///
/// Ошибка типа, записи JSON или вызова функции преобразования.
pub fn write_json_builtin(
    arguments: &[BslValue],
    runtime: &mut RuntimeShapes,
    call: Option<JsonCallByName<'_>>,
) -> RtResult<BslValue> {
    let settings = serializer_settings_from(arguments.get(2))?;
    let name = callback_name(
        arguments.get(3),
        arguments.get(4),
        "ЗаписатьJSON(ИмяФункцииПреобразования)",
    )?;
    let convert = match (name, call) {
        (None, _) => None,
        (Some(name), Some(call)) => Some(JsonConvertFn {
            name,
            extra: arguments.get(5).cloned().unwrap_or(BslValue::Undefined),
            call,
        }),
        (Some(_), None) => {
            return Err(RtError::Json(
                "ЗаписатьJSON: функция преобразования требует исполняющей VM".to_string(),
            ));
        }
    };
    write_json(&arguments[0], &arguments[1], &settings, convert, runtime)?;
    Ok(BslValue::Undefined)
}

// --- ПрочитатьJSON / ЗаписатьJSON ---------------------------------------

use bsl_rt::RuntimeShapes;

/// Подготовленные имена свойств в пределах одного `ПрочитатьJSON`.
///
/// Ключ хранит точное написание из JSON: разный регистр может дать
/// две записи в этом кэше, но `NameInterner` всё равно вернёт им один
/// регистронезависимый `NameId`. Это сохраняет семантику и не требует
/// Unicode-нормализации на каждом повторе одной и той же схемы.
type JsonKeyCache = HashMap<Box<str>, bsl_rt::NameId>;

/// Итоговые формы объектов, уже встреченные в текущем документе.
///
/// Первый объект каждой схемы строится обычными `structure_insert`: так
/// сохраняются порог переходов и деградация в словарь. Повторный объект
/// получает ту же форму и готовые слоты сразу, без прохода по цепочке
/// промежуточных форм для каждого поля.
type JsonShapeCache = HashMap<Vec<bsl_rt::NameId>, Rc<bsl_rt::Shape>>;

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
///
/// `ПрочитатьДатуJSON`/`ЗаписатьДатуJSON` (см. `read_json_date`/
/// `write_json_date` ниже) — ИСКЛЮЧЕНИЕ из этого правила, а не отказ от
/// него: смещение машины (`bsl_rt::tz`) там нужно самой сутью функций
/// (варианты `ЛокальнаяДатаСоСмещением`/`УниверсальнаяДата`
/// `ВариантЗаписиДатыJSON` описаны платформой именно через часовой пояс
/// машины), это явно заказанная этим этапом способность, а не тихое
/// распространение зоны на весь модуль `json`. `ИменаСвойствСоЗначениямиДата`
/// здесь по-прежнему без сдвига.
fn parse_json_date(text: &str) -> Option<bsl_rt::BslDate> {
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
    bsl_rt::BslDate::from_civil(year, month, day, hour, minute, second)
}

// --- ЗаписатьДатуJSON / ПрочитатьДатуJSON -------------------------------
//
// Единственное место в крейте, где дата интерпретируется относительно
// часового пояса МАШИНЫ (`bsl_rt::tz`), а не как наивный набор полей: сама
// суть `ВариантЗаписиДатыJSON` — «локальная», «локальная со смещением»,
// «универсальная» — это часовой пояс. `BslDate`, переданная сюда, читается
// как НАИВНОЕ ЛОКАЛЬНОЕ время машины: `ЗаписатьДатуJSON` с вариантом
// `УниверсальнаяДата` вычитает офсет машины, `ПрочитатьДатуJSON` с
// UTC-моментом (`Z`, JavaScript, Microsoft) его прибавляет — симметрично,
// так что `ПрочитатьДатуJSON(ЗаписатьДатуJSON(Д, ...), ...) = Д` на машине
// с одним и тем же смещением в оба конца.

fn unix_to_bsl_date(unix_seconds: i64, op: &'static str) -> RtResult<bsl_rt::BslDate> {
    unix_seconds
        .checked_add(bsl_rt::UNIX_EPOCH_SECONDS)
        .and_then(bsl_rt::BslDate::from_seconds)
        .ok_or(RtError::DateOutOfRange { op })
}

/// `+ЧЧ:ММ`/`-ЧЧ:ММ` — знак и величина смещения в секундах.
fn format_offset(offset_seconds: i32) -> String {
    let sign = if offset_seconds < 0 { '-' } else { '+' };
    let abs = offset_seconds.unsigned_abs();
    format!("{sign}{:02}:{:02}", abs / 3600, (abs % 3600) / 60)
}

/// Содержимое ЗНАЧЕНИЯ даты — без кавычек JSON, их ставит вызывающий
/// (`write_json_date` отдаёт эту строку как есть, `serialize` заворачивает
/// её как обычную строку через `JsonWriter::value`).
///
/// # Errors
///
/// ИЗМЕРЕНО (`JSON.WRITE_DATE.NON_ISO_LOCAL_ERROR`): платформа отвергает
/// сочетание не-ISO формата с не-универсальным вариантом записи — факт
/// исключения подтверждён, точный текст платформы снять не удалось (см.
/// `Anchor` в реестре), здесь [`RtError::Json`] с СОБСТВЕННЫМ текстом по
/// смыслу статьи. [`RtError::DateOutOfRange`], если перевод в UTC
/// вычитанием смещения машины ушёл за границы `0001-01-01..9999-12-31`
/// (пустые и предельные даты около границ диапазона).
fn format_json_date(
    date: bsl_rt::BslDate,
    format: JsonDateFormat,
    variant: JsonDateWritingVariant,
) -> RtResult<String> {
    if format != JsonDateFormat::Iso && variant != JsonDateWritingVariant::Universal {
        return Err(RtError::Json(
            "формат даты, отличный от ISO, поддержан только для варианта записи \
             УниверсальнаяДата (JSONDateWritingVariant.UniversalDate)"
                .to_string(),
        ));
    }
    match variant {
        JsonDateWritingVariant::Local => {
            let c = date.to_civil();
            Ok(format!(
                "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}",
                c.year, c.month, c.day, c.hour, c.minute, c.second
            ))
        }
        JsonDateWritingVariant::LocalOffset => {
            let c = date.to_civil();
            let offset = bsl_rt::local_offset_seconds(pseudo_unix_seconds(date));
            Ok(format!(
                "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}{}",
                c.year,
                c.month,
                c.day,
                c.hour,
                c.minute,
                c.second,
                format_offset(offset)
            ))
        }
        JsonDateWritingVariant::Universal => {
            let pseudo = pseudo_unix_seconds(date);
            let offset = bsl_rt::local_offset_seconds(pseudo);
            let utc_unix = pseudo - i64::from(offset);
            // ИЗМЕРЕНО: `ЗаписатьДатуJSON(Дата(1,1,1), ISO, УниверсальнаяДата)`
            // на платформе даёт `0001-01-01T00:00:00Z`, а не ошибку — хотя
            // вычитание смещения машины уводит псевдо-момент НИЖЕ пола
            // диапазона (`0001-01-01`) для положительных (восточных)
            // смещений. Платформа клампит результат к полу, а не падает;
            // тот же приём применён здесь. Поведение для дат ВБЛИЗИ пола,
            // но не равных ему, замером не подтверждено — фикстура несёт
            // пробу на этот случай.
            let floor = pseudo_unix_seconds(bsl_rt::BslDate::empty());
            let utc_unix = utc_unix.max(floor);
            match format {
                JsonDateFormat::Iso => {
                    let utc_date = unix_to_bsl_date(utc_unix, "ЗаписатьДатуJSON")?;
                    let c = utc_date.to_civil();
                    Ok(format!(
                        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
                        c.year, c.month, c.day, c.hour, c.minute, c.second
                    ))
                }
                JsonDateFormat::JavaScript => Ok(format!("new Date({})", utc_unix * 1000)),
                // ИЗМЕРЕНО: платформа пишет момент БЕЗ обратных косых —
                // `/Date(мс)/`, а не `\/Date(мс)\/`, как ошибочно
                // предполагалось раньше (до замера).
                JsonDateFormat::Microsoft => Ok(format!("/Date({})/", utc_unix * 1000)),
            }
        }
    }
}

/// `ЗаписатьДатуJSON(Дата, Формат[, Вариант])` -> `Строка`.
///
/// # Errors
///
/// [`RtError::TypeError`] на аргументах не тех типов; иначе см.
/// `format_json_date`.
pub fn write_json_date(
    date: &BslValue,
    format: &BslValue,
    variant: &BslValue,
) -> RtResult<BslValue> {
    let BslValue::Date(d) = date else {
        return Err(RtError::TypeError {
            expected: "Дата",
            op: "ЗаписатьДатуJSON",
        });
    };
    let BslValue::Enum(format_enum) = format else {
        return Err(RtError::TypeError {
            expected: "ФорматДатыJSON",
            op: "ЗаписатьДатуJSON",
        });
    };
    let format = JsonDateFormat::from_enum_value(*format_enum).ok_or(RtError::TypeError {
        expected: "ФорматДатыJSON",
        op: "ЗаписатьДатуJSON",
    })?;
    let variant = match variant {
        BslValue::Undefined => JsonDateWritingVariant::default(),
        BslValue::Enum(e) => {
            JsonDateWritingVariant::from_enum_value(*e).ok_or(RtError::TypeError {
                expected: "ВариантЗаписиДатыJSON",
                op: "ЗаписатьДатуJSON",
            })?
        }
        _ => {
            return Err(RtError::TypeError {
                expected: "ВариантЗаписиДатыJSON",
                op: "ЗаписатьДатуJSON",
            });
        }
    };
    let text = format_json_date(*d, format, variant)?;
    Ok(BslValue::Str(bsl_rt::BslString::from_str(&text)))
}

/// ИЗМЕРЕНО (`JSON.READ_DATE.BAD_FORMAT_TEXT`): факт исключения на
/// неразобравшемся представлении подтверждён, точный текст платформы снять
/// не удалось (см. `Anchor` в реестре — `КраткоеПредставлениеОшибки`
/// внутри `Вычислить` не видит контекст чужого исключения). Текст ниже —
/// «Представление даты имеет неверный формат» — СОБСТВЕННЫЙ, по смыслу
/// платформенных сообщений об ошибках разбора (`XXXИзСтроки`).
fn bad_date_representation() -> RtError {
    RtError::Json("Представление даты имеет неверный формат".to_string())
}

fn utc_millis_to_local_date(ms: i64) -> RtResult<bsl_rt::BslDate> {
    local_date_from_utc_seconds(ms.div_euclid(1000), "ПрочитатьДатуJSON")
}

/// `+ЧЧ:ММ`/`-ЧЧ:ММ` -> секунды. `None` — не разобралось.
fn parse_offset(s: &str) -> Option<i32> {
    let (h, m) = s.split_once(':')?;
    let h: i32 = h.parse().ok()?;
    let m: i32 = m.parse().ok()?;
    Some(h * 3600 + m * 60)
}

/// Хвост ISO-представления после времени суток: маркер зоны или явное
/// смещение (в секундах, уже со знаком).
enum IsoTail {
    Utc,
    Offset(i32),
}

/// Отделяет от ISO-строки суффикс зоны. Знак смещения ищется ПОСЛЕ `T`:
/// до неё дефисы принадлежат самой дате (`2024-03-04`), а время суток
/// `+`/`-` не содержит вовсе.
fn split_iso_tail(text: &str) -> (&str, Option<IsoTail>) {
    if let Some(body) = text.strip_suffix('Z') {
        return (body, Some(IsoTail::Utc));
    }
    if let Some(t_pos) = text.find('T') {
        let after_time = &text[t_pos + 1..];
        if let Some(rel) = after_time.find(['+', '-']) {
            let sign = after_time.as_bytes()[rel] as char;
            if let Some(magnitude) = parse_offset(&after_time[rel + 1..]) {
                let secs = if sign == '-' { -magnitude } else { magnitude };
                return (&text[..t_pos + 1 + rel], Some(IsoTail::Offset(secs)));
            }
        }
    }
    (text, None)
}

/// Разбор ISO-представления `ПрочитатьДатуJSON` во всех трёх вариантах
/// записи: без зоны (локальное время как есть), `Z` (UTC) и явное
/// смещение.
fn parse_iso_json_date(text: &str) -> Option<bsl_rt::BslDate> {
    let (body, tail) = split_iso_tail(text);
    let (date_part, time_part) = body.split_once('T')?;
    let mut dp = date_part.split('-');
    let year: i64 = dp.next()?.parse().ok()?;
    let month: u32 = dp.next()?.parse().ok()?;
    let day: u32 = dp.next()?.parse().ok()?;
    let mut tp = time_part.split(':');
    let hour: u32 = tp.next()?.parse().ok()?;
    let minute: u32 = tp.next()?.parse().ok()?;
    let second: u32 = tp.next()?.split('.').next()?.parse().ok()?;
    let wall = bsl_rt::BslDate::from_civil(year, month, day, hour, minute, second)?;
    match tail {
        // Без указания зоны — то, что и хранится: локальное время машины
        // как есть, без пересчёта (симметрично `Local` при записи).
        None => Some(wall),
        Some(IsoTail::Utc) => utc_millis_to_local_date(pseudo_unix_seconds(wall) * 1000).ok(),
        Some(IsoTail::Offset(off_secs)) => {
            let utc_unix = pseudo_unix_seconds(wall) - i64::from(off_secs);
            utc_millis_to_local_date(utc_unix * 1000).ok()
        }
    }
}

fn parse_json_date_by_format(text: &str, format: JsonDateFormat) -> Option<bsl_rt::BslDate> {
    match format {
        JsonDateFormat::Iso => parse_iso_json_date(text),
        JsonDateFormat::JavaScript => {
            let inner = text.strip_prefix("new Date(")?.strip_suffix(')')?;
            let ms: i64 = inner.trim().parse().ok()?;
            utc_millis_to_local_date(ms).ok()
        }
        // ИЗМЕРЕНО: платформа кидает исключение на написании с обратными
        // косыми (`\/Date(...)\/ `) — до замера здесь принимались оба
        // написания «на всякий случай», это оказалось лишним снисхождением.
        // Единственная принимаемая форма — `/Date(мс)/ `. Закрывающая
        // скобка идёт ПЕРЕД хвостовой чертой, поэтому в хвосте снимается
        // `)/ ` целиком, а не только сама черта.
        JsonDateFormat::Microsoft => {
            let inner = text.strip_prefix("/Date(")?.strip_suffix(")/")?;
            let ms: i64 = inner.trim().parse().ok()?;
            utc_millis_to_local_date(ms).ok()
        }
    }
}

/// `ПрочитатьДатуJSON(Строка, Формат)` -> `Дата`.
///
/// # Errors
///
/// [`RtError::TypeError`] на аргументах не тех типов;
/// [`RtError::Json`] (см. `bad_date_representation`) на строке, не
/// разобравшейся в заданном формате.
pub fn read_json_date(text: &BslValue, format: &BslValue) -> RtResult<BslValue> {
    let BslValue::Str(s) = text else {
        return Err(RtError::TypeError {
            expected: "Строка",
            op: "ПрочитатьДатуJSON",
        });
    };
    let BslValue::Enum(format_enum) = format else {
        return Err(RtError::TypeError {
            expected: "ФорматДатыJSON",
            op: "ПрочитатьДатуJSON",
        });
    };
    let format = JsonDateFormat::from_enum_value(*format_enum).ok_or(RtError::TypeError {
        expected: "ФорматДатыJSON",
        op: "ПрочитатьДатуJSON",
    })?;
    let text = s.to_string();
    let date = parse_json_date_by_format(&text, format).ok_or_else(bad_date_representation)?;
    Ok(BslValue::Date(date))
}

/// Вызов функции модуля ПО ИМЕНИ — канал из исполняющей VM в рантайм.
///
/// `bsl-rt` не видит `bsl-vm` (зависимость идёт в обратную сторону), поэтому
/// колбэки `ПрочитатьJSON`/`ЗаписатьJSON` приходят сюда сверху замыканием
/// над `bsl_vm::call_module_function`. Аргументы уходят по значению, а
/// возвращается ПАРА: значение `Возврат` и финальные значения слотов
/// параметров — второй элемент нужен ровно затем, чтобы прочитать `Отказ`
/// функции преобразования (запись в параметр без `Знач` наблюдаема только
/// так, см. `call_module_function`).
pub type JsonCallByName<'a> =
    &'a mut dyn FnMut(&str, Vec<BslValue>) -> RtResult<(BslValue, Vec<BslValue>)>;

/// Функция преобразования `ЗаписатьJSON` (статья 16.2).
///
/// Сигнатура на платформе — РОВНО четыре параметра
/// `(Свойство, Значение, ДополнительныеПараметры, Отказ)`; другое их число —
/// ошибка вызова («В методе X количество параметров 2. Ожидаемое
/// количество - 4»), и у нас её даёт `call_module_function` своим текстом.
pub struct JsonConvertFn<'a> {
    /// `ИмяФункцииПреобразования` — четвёртый аргумент `ЗаписатьJSON`.
    pub name: String,
    /// `ДополнительныеПараметрыФункцииПреобразования` — шестой аргумент;
    /// уходит в функцию третьим параметром как есть.
    pub extra: BslValue,
    /// Как позвать функцию модуля по имени.
    pub call: JsonCallByName<'a>,
}

/// Функция восстановления `ПрочитатьJSON` (статья 16.2).
///
/// Сигнатура на платформе — РОВНО три параметра
/// `(Свойство, Значение, ДополнительныеПараметры)`.
pub struct JsonRestoreFn<'a> {
    /// `ИмяФункцииВосстановления` — пятый аргумент `ПрочитатьJSON`.
    pub name: String,
    /// `ДополнительныеПараметрыФункцииВосстановления` — седьмой аргумент.
    pub extra: BslValue,
    /// `ИменаСвойствДляФункцииВосстановления` — восьмой аргумент.
    ///
    /// ИЗМЕРЕНО: пустой список (как и отсутствующий аргумент) означает «для
    /// каждого значения документа», а непустой сужает вызовы до
    /// перечисленных имён свойств НА ЛЮБОЙ ГЛУБИНЕ и заодно отменяет вызов
    /// на корне документа. Сравнение регистрозависимое — см.
    /// `JsonRestoreFn::applies_to`.
    pub property_names: Vec<String>,
    /// Как позвать функцию модуля по имени.
    pub call: JsonCallByName<'a>,
}

impl JsonRestoreFn<'_> {
    /// Зовётся ли функция восстановления для значения, лежащего под именем
    /// `property` (`None` — элемент массива или корень документа)?
    ///
    /// ИЗМЕРЕНО на 8.3.27, документ `{"а":1,"б":2,"в":{"б":3,"г":4}}`:
    /// * без списка имён функция получает ВСЕ значения — включая элементы
    ///   массивов (`Свойство = Неопределено`) и сам корень;
    /// * со списком `["б"]` — ровно два вызова, оба на свойстве `б`
    ///   (внешнем и вложенном), и НИ ОДНОГО на корне;
    /// * со списком `["БЭ"]` против свойства `бэ` — ни одного вызова,
    ///   то есть сравнение РЕГИСТРОЗАВИСИМОЕ (как и у
    ///   `ИменаСвойствСоЗначениямиДата`, см. [`is_date_property`]).
    fn applies_to(&self, property: Option<&str>) -> bool {
        if self.property_names.is_empty() {
            return true;
        }
        property.is_some_and(|p| self.property_names.iter().any(|n| n == p))
    }
}

/// Всё, что при сборке значения не меняется от узла к узлу.
///
/// Отдельной структурой, потому что без неё `build_value` пришлось бы
/// тащить девять параметров сквозь рекурсию, а функция восстановления
/// добавляет десятый — и повторное `&mut`-заимствование каждого из них на
/// каждом уровне.
struct BuildCtx<'a, 'c> {
    as_map: bool,
    date_names: &'a [String],
    date_format: Option<JsonDateFormat>,
    /// Функция восстановления или `None`, если она не задана.
    restore: Option<JsonRestoreFn<'c>>,
    rt: &'a mut RuntimeShapes,
    cache: JsonBuildCache,
}

impl BuildCtx<'_, '_> {
    /// Нужно ли звать функцию восстановления на значении под именем
    /// `property`.
    fn restores(&self, property: Option<&str>) -> bool {
        self.restore
            .as_ref()
            .is_some_and(|r| r.applies_to(property))
    }

    /// Вызов функции восстановления на уже собранном значении.
    ///
    /// # Errors
    ///
    /// Ошибку самого вызова (нет такой функции, не то число параметров) и
    /// любое исключение изнутри функции — ИЗМЕРЕНО, что платформа их не
    /// глотает, а выпускает наружу из `ПрочитатьJSON`.
    fn call_restore(&mut self, property: Option<&str>, value: BslValue) -> RtResult<BslValue> {
        let Some(restore) = self.restore.as_mut() else {
            return Ok(value);
        };
        let args = vec![property_arg(property), value, restore.extra.clone()];
        let (returned, _) = (restore.call)(&restore.name, args)?;
        Ok(returned)
    }
}

/// Первый параметр колбэка: имя свойства или `Неопределено`.
///
/// ИЗМЕРЕНО: `Неопределено` приходит и для элемента массива, и для
/// верхнего уровня документа — платформа не выдумывает им ни индекса, ни
/// пустой строки.
fn property_arg(property: Option<&str>) -> BslValue {
    match property {
        Some(p) => BslValue::Str(bsl_rt::BslString::from_str(p)),
        None => BslValue::Undefined,
    }
}

/// `ПрочитатьJSON(Чтение[, ВозвращатьСоответствие[, ИменаСвойствСоЗначениямиДата
/// [, ОжидаемыйФорматДаты[, ИмяФункцииВосстановления, ...]]]])`.
///
/// `date_format` — четвёртый аргумент платформы: `None`, если он не задан
/// (тогда разбор `ИменаСвойствСоЗначениямиДата` идёт по старому правилу —
/// см. `optional_date_format_from_arg`). `restore` — функция восстановления
/// (пятый-восьмой аргументы), см. [`JsonRestoreFn`].
///
/// # Errors
///
/// [`RtError::Json`] на битом вводе, на ключе, который не может быть именем
/// поля структуры, либо (при заданном `date_format`) на значении из
/// `ИменаСвойствСоЗначениямиДата`, не разобравшемся в этом формате
/// (см. `bad_date_representation`, `JSON.READ_DATE.BAD_FORMAT_TEXT`);
/// ошибку вызова функции восстановления и любое исключение из неё.
pub fn read_json(
    reader: &BslValue,
    as_map: bool,
    date_names: &[String],
    date_format: Option<JsonDateFormat>,
    restore: Option<JsonRestoreFn<'_>>,
    rt: &mut RuntimeShapes,
) -> RtResult<BslValue> {
    // Первое событие читается здесь же: `ПрочитатьJSON` забирает документ
    // с текущей позиции целиком, и вызывать перед ним `Прочитать()` не
    // требуется.
    let cell = as_reader(arg_object(reader)?)?;
    // Разборщик ВЫНИМАЕТСЯ из ячейки на всё время сборки, а не держится
    // заимствованным: функция восстановления — это пользовательский код,
    // и он волен потрогать тот же самый `ЧтениеJSON`. С `borrow_mut()`
    // такой повторный вход был бы паникой `RefCell` мимо `Попытка`; без
    // разборщика в ячейке он упирается в обычную перехватываемую ошибку
    // «нет назначенного источника». Платформа в этом месте тоже отвечает
    // ошибкой («Недопустимое состояние потока чтения JSON»), а не молча
    // продолжает, — текст у нас свой, как и для остальных ошибок JSON.
    let (first, mut parser) = {
        let mut state = cell.borrow_mut();
        let Some(mut parser) = state.parser.take() else {
            return Err(RtError::TypeError {
                expected: "назначенный источник (УстановитьСтроку/ОткрытьФайл)",
                op: "ПрочитатьJSON",
            });
        };
        // Текущее событие, если на него уже встали ручным `Прочитать()`,
        // иначе следующее: `ПрочитатьJSON` работает в обоих сценариях.
        let first = match state.current.take() {
            Some(e) => Ok(Some(e)),
            None => parser.next_event(),
        };
        match first {
            Ok(first) => (first, parser),
            Err(e) => {
                state.parser = Some(parser);
                return Err(e);
            }
        }
    };

    let mut ctx = BuildCtx {
        as_map,
        date_names,
        date_format,
        restore,
        rt,
        cache: JsonBuildCache::default(),
    };
    let built = match first {
        None => Ok(BslValue::Undefined),
        Some(first) => build_value(first, &mut parser, None, &mut ctx, 0),
    };
    // Разборщик возвращается на место при любом исходе: после ошибки
    // `ЧтениеJSON` обязан остаться тем же объектом, у которого можно
    // спросить `Закрыть()`.
    cell.borrow_mut().parser = Some(parser);
    built
}

/// `ОжидаемыйФорматДаты` — четвёртый аргумент `ПрочитатьJSON`.
///
/// Отсутствует (`Неопределено`) — `None`: разбор
/// `ИменаСвойствСоЗначениямиДата` по умолчанию — ISO, но через СТАРЫЙ
/// парсер без сдвига зоны (`parse_json_date`, не
/// `parse_json_date_by_format`) — это отдельное, ранее измеренное
/// намеренное отклонение (см. doc comment на `parse_json_date`), не
/// тронутое добавлением этого аргумента. ИЗМЕРЕНО: разбор СТРОГИЙ даже без
/// явного формата — представление, не разобравшееся как ISO, даёт то же
/// исключение, что и при явном формате (до замера здесь был тихий фолбэк
/// в строку).
///
/// Задан — платформа проверяет представление СТРОГО под этот формат;
/// несовпадение (в том числе значение вовсе не строка — статья приводит
/// пример с числом при `ФорматДатыJSON.JavaScript`) — исключение с тем же
/// текстом, что и у `ПрочитатьДатуJSON` (`JSON.READ_DATE.BAD_FORMAT_TEXT`).
///
/// ИЗМЕРЕНО и НЕ ВОСПРОИЗВОДИТСЯ: платформа различает ПРОПУЩЕННЫЙ аргумент
/// и явно переданное `Неопределено`. `ПрочитатьJSON(Ч, Ложь, , , "Имя")`
/// работает, а `ПрочитатьJSON(Ч, Ложь, Неопределено, Неопределено, "Имя")`
/// падает с «Несоответствие типов (параметр номер '4')» — то есть
/// `ОжидаемыйФорматДаты` принимает только `ФорматДатыJSON`, но
/// необязательность проверяет по факту передачи. Здесь этого различия нет:
/// резолвер добивает необязательные позиции встроенного вызова именно
/// `Неопределено` (см. `call_builtin_with_format`), так что оба написания
/// приходят сюда одинаковыми, и отвергать `Неопределено` значило бы сломать
/// все вызовы с пропущенным форматом. Это НЕ открытый вопрос — поведение
/// платформы известно; воспроизвести его нечем без отдельного маркера
/// «аргумент не передавали» в байт-коде встроенных вызовов.
///
/// # Errors
///
/// [`RtError::TypeError`], если аргумент задан и не `ФорматДатыJSON`.
pub fn optional_date_format_from_arg(
    arg: Option<&BslValue>,
    op: &'static str,
) -> RtResult<Option<JsonDateFormat>> {
    match arg {
        None | Some(BslValue::Undefined) => Ok(None),
        Some(BslValue::Enum(e)) => {
            JsonDateFormat::from_enum_value(*e)
                .map(Some)
                .ok_or(RtError::TypeError {
                    expected: "ФорматДатыJSON",
                    op,
                })
        }
        Some(_) => Err(RtError::TypeError {
            expected: "ФорматДатыJSON",
            op,
        }),
    }
}

/// `ПрочитатьЗначениеJSON(Строка)` -> значение — обратная операция к
/// `ЗаписатьЗначениеJSON`, поверх того же `build_value`, которым разбирает
/// и `ПрочитатьJSON`.
///
/// ИЗМЕРЕНО (`JSON.VALUE.READ_KIND`): объект JSON превращается в
/// `Структура`, а не в `Соответствие` — тот же выбор по умолчанию, что и у
/// `ПрочитатьJSON` без второго аргумента (`JSON.DESERIALIZE.DEFAULT_TYPE`).
///
/// ИЗМЕРЕНО: пустая строка — тоже исключение, а не тихое `Неопределено`
/// (снято прогоном фикстуры `json-dates`; до замера здесь было наоборот).
///
/// # Errors
///
/// [`RtError::TypeError`], если аргумент не строка; [`RtError::Json`] на
/// пустой строке (в ней нет ни одного события разбора); иначе — см.
/// [`read_json`].
pub fn read_json_value(text: &BslValue, rt: &mut RuntimeShapes) -> RtResult<BslValue> {
    let BslValue::Str(s) = text else {
        return Err(RtError::TypeError {
            expected: "Строка",
            op: "ПрочитатьЗначениеJSON",
        });
    };
    let mut parser = JsonParser::from_bsl_string(s);
    let Some(first) = parser.next_event()? else {
        return Err(RtError::Json(
            "пустая строка не представляет значение JSON".to_string(),
        ));
    };
    // Функции восстановления у `ПрочитатьЗначениеJSON` нет вовсе — у
    // платформы такого параметра здесь не существует.
    let mut ctx = BuildCtx {
        as_map: false,
        date_names: &[],
        date_format: None,
        restore: None,
        rt,
        cache: JsonBuildCache::default(),
    };
    build_value(first, &mut parser, None, &mut ctx, 0)
}

/// Проверяет и интернирует имя один раз за разбор документа.
fn json_field_id(
    name: &str,
    rt: &mut RuntimeShapes,
    cache: &mut JsonKeyCache,
) -> RtResult<bsl_rt::NameId> {
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
    let mut positions: Option<HashMap<bsl_rt::NameId, usize>> = None;
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

/// Свойство `property` перечислено в `ИменаСвойствСоЗначениямиДата`?
///
/// ИЗМЕРЕНО: сравнение РЕГИСТРОЗАВИСИМОЕ — вопреки общему правилу языка,
/// где идентификаторы регистр не различают. Проба: документ
/// `{"создано":"2014-05-10T13:14:15"}` со списком `["создано"]` даёт `Дата`,
/// а с `["СОЗДАНО"]` — по-прежнему `Строка`. Это имена свойств ДОКУМЕНТА,
/// а не идентификаторы BSL, и платформа обращается с ними как с ключами
/// JSON. До замера здесь стояло `to_uppercase` на обеих сторонах.
fn is_date_property(property: Option<&str>, date_names: &[String]) -> bool {
    property.is_some_and(|p| date_names.iter().any(|n| n == p))
}

/// Сборка значения из события и продолжения потока с вызовом функции
/// восстановления на готовом результате.
///
/// ИЗМЕРЕНО, что порядок вызовов — ОБРАТНЫЙ (сначала дети, потом родитель).
/// Документ
/// `{"чис":1,"стр":"т","лог":true,"нул":null,"об":{"вчис":2,"вмас":[7,8]},
/// "мас":[3,"ф",false,null,{"мчис":4},[5,6]]}` даёт ровно двадцать вызовов
/// в порядке `чис`, `стр`, `лог`, `нул`, `вчис`, элемент, элемент, `вмас`,
/// `об`, элемент, элемент, элемент, элемент, `мчис`, элемент (объект),
/// элемент, элемент, элемент (массив), `мас`, корень: значение приходит в
/// функцию уже собранным из УЖЕ восстановленных детей, а её результат
/// становится тем, что увидит родитель. Поэтому вызов стоит здесь, на
/// выходе, а не в родительских ветках.
fn build_value(
    event: JsonEvent,
    parser: &mut JsonParser,
    property: Option<&str>,
    ctx: &mut BuildCtx<'_, '_>,
    depth: usize,
) -> RtResult<BslValue> {
    let restores = ctx.restores(property);
    let value = build_raw_value(event, parser, property, restores, ctx, depth)?;
    if restores {
        ctx.call_restore(property, value)
    } else {
        Ok(value)
    }
}

/// Сборка значения из события и продолжения потока — без функции
/// восстановления.
///
/// `property` — имя свойства, под которым это значение лежит у родителя:
/// по нему решается, превращать ли строку в дату
/// (`ИменаСвойствСоЗначениямиДата`). `date_format` — четвёртый аргумент
/// `ПрочитатьJSON` (см. `optional_date_format_from_arg`): `None` — старое
/// правило (только ISO, неудача молча оставляет строку), `Some(fmt)` —
/// значение обязано разобраться СТРОГО под этот формат.
///
/// `restores` — будет ли на ЭТОМ значении вызвана функция восстановления.
/// ИЗМЕРЕНО, что она отменяет разбор даты: документ
/// `{"создано":"2014-05-10T13:14:15","прочее":1}` с
/// `ИменаСвойствСоЗначениямиДата = ["создано"]` даёт `Дата` без функции
/// восстановления и с функцией, суженной списком до `прочее`, — но
/// `Строка` (сырое представление), если функция зовётся и на `создано`.
/// То есть функция восстановления имеет приоритет над списком дат, а не
/// получает уже готовую дату.
fn build_raw_value(
    event: JsonEvent,
    parser: &mut JsonParser,
    property: Option<&str>,
    restores: bool,
    ctx: &mut BuildCtx<'_, '_>,
    depth: usize,
) -> RtResult<BslValue> {
    // Как и в `serialize`: рекурсия возможна только на контейнерах, на
    // скалярных событиях предел не срабатывает никогда.
    if depth > MAX_JSON_DEPTH && matches!(event, JsonEvent::ObjectStart | JsonEvent::ArrayStart) {
        return Err(RtError::StackOverflow {
            what: "слишком глубокая вложенность документа при чтении JSON",
        });
    }
    // Разбор даты отменяется, если это значение уходит в функцию
    // восстановления (измерено, см. doc comment).
    let is_date = !restores && is_date_property(property, ctx.date_names);
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
                        let v = build_value(value_event, parser, Some(&name), ctx, depth + 1)?;
                        keys.push(name);
                        values.push(v);
                    }
                    _ => break,
                }
            }
            if ctx.as_map {
                let map = BslValue::new_map();
                for (k, v) in keys.into_iter().zip(values) {
                    map.map_insert(BslValue::Str(bsl_rt::BslString::from_str(&k)), v)?;
                }
                Ok(map)
            } else {
                build_json_structure(keys, values, ctx.rt, &mut ctx.cache)
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
                let v = build_value(next, parser, None, ctx, depth + 1)?;
                items.push_element(v)?;
            }
            Ok(items)
        }
        JsonEvent::Str(s) => {
            // Дата — только если имя свойства перечислено. JSON типа даты
            // не знает, а гадать по виду строки платформа не берётся, и мы
            // тоже: «2024-03-04» может быть просто строкой.
            //
            // ИЗМЕРЕНО: платформа кидает исключение и БЕЗ явного формата
            // (четвёртого аргумента `ПрочитатьJSON`), если значение не
            // разбирается, — до замера здесь был тихий фолбэк в строку, и
            // фикстура `json-dates` упала ровно на этой пробе (была без
            // `Попытка`). Формат по умолчанию — ISO, но именно СТАРЫЙ
            // парсер без сдвига зоны (`parse_json_date`), а не
            // `parse_json_date_by_format(..., Iso)`: сдвиг `Z`/явного
            // смещения в локальное время машины для ЭТОГО (более раннего)
            // пути — отдельное, ранее измеренное намеренное отклонение
            // (см. doc comment на `parse_json_date`), эта правка его не
            // трогает — меняется только СТРОГОСТЬ (ошибка вместо тихого
            // фолбэка), не арифметика разбора.
            if is_date {
                let d = match ctx.date_format {
                    Some(fmt) => parse_json_date_by_format(&s, fmt),
                    None => parse_json_date(&s),
                }
                .ok_or_else(bad_date_representation)?;
                return Ok(BslValue::Date(d));
            }
            Ok(BslValue::Str(bsl_rt::BslString::from_str(&s)))
        }
        // Число/булево на месте объявленного имени даты — заведомо не
        // текстовое представление ни одного из трёх форматов (у всех троих
        // момент кодируется строкой), поэтому та же ошибка, что и у
        // несоответствующей строки. Статья приводит именно такой пример —
        // число вместо `"new Date(...)"` — при ЯВНО заданном формате;
        // ИЗМЕРЕНО, что без него платформа тоже не прощает (см. выше), так
        // что здесь проверка больше не зависит от `date_format.is_some()`.
        JsonEvent::Number(n) => {
            if is_date {
                return Err(bad_date_representation());
            }
            Ok(BslValue::Number(n))
        }
        JsonEvent::Boolean(b) => {
            if is_date {
                return Err(bad_date_representation());
            }
            Ok(BslValue::Boolean(b))
        }
        // ИЗМЕРЕНО: `null` становится `Неопределено`, а НЕ `Null`. `null`
        // не проверяется на соответствие формату даты даже при явном
        // формате: это осмысленное «нет значения», а не мусор на месте
        // даты, и статья не даёт для него примера — расширять список
        // отвергаемых значений домыслом не стоит.
        JsonEvent::Null => Ok(BslValue::Undefined),
        JsonEvent::PropertyName(s) => Ok(BslValue::Str(bsl_rt::BslString::from_str(&s))),
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
/// Предел вложенности данных при записи и документа при чтении JSON.
/// И `serialize`, и `build_value` рекурсивны, поэтому глубина входа
/// напрямую расходует стек Rust; без предела циклическая структура в
/// `ЗаписатьJSON` (массив, содержащий сам себя) и документ вида `[[[[…`
/// в `ПрочитатьJSON` валят процесс переполнением стека вместо
/// перехватываемой ошибки.
// НЕ ИЗМЕРЕНО(JSON.MAX_DEPTH) — какую глубину допускает платформа и что
// она делает с циклической структурой в `ЗаписатьJSON`; циклический зонд
// намеренно не ставится — если платформа на нём падает, он уносит весь
// сеанс замеров. Замер даёт нижнюю границу: 400 уровней обязаны работать.
const MAX_JSON_DEPTH: usize = 500;

/// `ЗаписатьJSON(Запись, Значение[, Настройки[, ИмяФункцииПреобразования, ...]])`.
///
/// `convert` — функция преобразования (четвёртый-шестой аргументы
/// платформы), см. [`JsonConvertFn`]. ИЗМЕРЕНО, что её имя само по себе НЕ
/// ошибка на входе и что зовётся она ЛЕНИВО — только там, где встретилось
/// значение, которое сериализовать нечем: `ЗаписатьJSON(Запись, 1,
/// Неопределено, "ИмяФункции", ЭтотОбъект)` пишет `1`, ни разу не позвав
/// функцию, и то же самое верно для `Дата` (её платформа сериализует сама).
///
/// # Errors
///
/// См. `serialize`.
pub fn write_json(
    writer: &BslValue,
    value: &BslValue,
    settings: &JsonSerializerSettings,
    convert: Option<JsonConvertFn<'_>>,
    rt: &RuntimeShapes,
) -> RtResult<()> {
    let cell = as_writer(arg_object(writer)?)?;
    // Приёмник, как и разборщик в `read_json`, ВЫНИМАЕТСЯ из ячейки на
    // время записи: функция преобразования — пользовательский код, который
    // волен позвать `ЗаписатьJSON` на том же самом объекте, а `borrow_mut()`
    // поперёк такого повторного входа был бы паникой `RefCell` мимо
    // `Попытка`. Платформа отвечает на этот случай ошибкой («Неверный
    // порядок записи JSON»), не паникой, — здесь получится своя ошибка про
    // отсутствие назначенного приёмника.
    let Some(mut w) = cell.borrow_mut().take() else {
        return Err(RtError::TypeError {
            expected: "назначенный приёмник (УстановитьСтроку/ОткрытьФайл)",
            op: "ЗаписатьJSON",
        });
    };
    let mut ctx = SerializeCtx {
        settings,
        single_value_mode: false,
        convert,
        rt,
    };
    let written = write_top_level(&mut w, value, &mut ctx);
    // Приёмник возвращается на место при любом исходе: `Закрыть()` после
    // ошибки обязан работать (измерено на отказе функции преобразования на
    // верхнем уровне — там документ пуст, а `Закрыть()` отдаёт пустую
    // строку).
    *cell.borrow_mut() = Some(w);
    written
}

/// Верхний уровень документа. Отдельной функцией из-за `Отказ`: ИЗМЕРЕНО,
/// что отказ функции преобразования на САМОМ значении не пишет вообще
/// ничего — `ЗаписатьJSON(Запись, ТаблицаЗначений, , "Отказная", ЭтотОбъект)`
/// с последующим `Закрыть()` даёт пустую строку, а не `null` и не ошибку.
fn write_top_level(
    w: &mut JsonWriter,
    value: &BslValue,
    ctx: &mut SerializeCtx<'_, '_>,
) -> RtResult<()> {
    match prepare(value, None, ctx)? {
        Prepared::Skip => Ok(()),
        Prepared::AsIs => serialize(w, value, ctx, 0),
        Prepared::Converted(v) => serialize_converted(w, &v, ctx, 0),
    }
}

/// `ЗаписатьЗначениеJSON(Значение)` — сериализация ОДНОГО значения в
/// строку поверх того же `serialize`, что и `ЗаписатьJSON`, но с
/// `single_value_mode = true`: дата, в том числе вложенная, — исключение
/// (см. обзор задачи в плане реализации, раздел «Этап 0»), и ИЗМЕРЕНО, что
/// `Соответствие` — тоже (`ЗаписатьJSON` с `Соответствие` работает и
/// измерен отдельно, значит отличие — в самой функции
/// `ЗаписатьЗначениеJSON`, не в объектной технике сериализации вообще).
///
/// # Errors
///
/// См. `serialize`; дополнительно [`RtError::TypeError`] на `Дата` или
/// `Соответствие` в любой позиции дерева значения.
pub fn write_json_value(value: &BslValue, rt: &RuntimeShapes) -> RtResult<BslValue> {
    let mut w = JsonWriter::to_string_target(JsonWriterSettings::default());
    let mut ctx = SerializeCtx {
        settings: &JsonSerializerSettings::default(),
        single_value_mode: true,
        // `ЗаписатьЗначениеJSON` не берёт функцию преобразования вовсе —
        // у платформы такого параметра здесь нет.
        convert: None,
        rt,
    };
    serialize(&mut w, value, &mut ctx, 0)?;
    Ok(BslValue::Str(bsl_rt::BslString::from_utf8_string(
        w.finish()?,
    )))
}

/// Ошибка на значении, которое `serialize` сериализовать не умеет
/// (`JSON.SERIALIZE.UNSUPPORTED_TYPE`).
///
/// ИЗМЕРЕНО, что имя функции преобразования само по себе эту ошибку НЕ
/// меняет: без `МодульФункцииПреобразования` платформа функцию не ищет
/// вовсе и отвечает тем же «Значение содержит данные недопустимых типов»,
/// что и без имени. Поэтому текст здесь один на все случаи, когда звать
/// оказалось некого.
fn unsupported_value_error() -> RtError {
    RtError::TypeError {
        expected: "значение, представимое в JSON",
        op: "ЗаписатьJSON",
    }
}

/// Всё, что при сериализации не меняется от узла к узлу.
struct SerializeCtx<'a, 'c> {
    settings: &'a JsonSerializerSettings,
    /// `ЗаписатьЗначениеJSON`: у него свои запреты (`Дата`, `Соответствие`)
    /// и функции преобразования не бывает.
    single_value_mode: bool,
    convert: Option<JsonConvertFn<'c>>,
    rt: &'a RuntimeShapes,
}

/// Что писать на месте очередного значения.
enum Prepared {
    /// Само значение: функция преобразования либо не нужна (значение
    /// сериализуемо), либо не задана.
    AsIs,
    /// Результат функции преобразования.
    Converted(BslValue),
    /// `Отказ = Истина`: значение молча выпадает из документа.
    Skip,
}

/// Значения, у которых нет собственного представления в JSON, — ровно те,
/// на которых платформа зовёт функцию преобразования.
///
/// Матч по `BslValue` исчерпывающий намеренно: новый вариант ЗНАЧЕНИЯ обязан
/// решить здесь, сериализуем он сам или уходит в функцию преобразования, —
/// иначе он молча попал бы в «сериализуемые» и упал бы уже в `serialize`.
/// На `BslObject` эта защита НЕ распространяется: ветка написана негативным
/// `matches!`, поэтому новый вариант объекта компилятор здесь не остановит —
/// он по умолчанию попадёт в «несериализуемые», то есть в функцию
/// преобразования. Умолчание консервативное и совпадает с тем, что давал
/// прежний `_ => Err(unsupported_value_error(..))` в `serialize`, но
/// проверить его на новом варианте придётся глазами.
fn needs_convert(value: &BslValue) -> bool {
    match value {
        BslValue::Str(_)
        | BslValue::Number(_)
        | BslValue::Boolean(_)
        | BslValue::Undefined
        | BslValue::Null
        | BslValue::Date(_) => false,
        BslValue::Object(o) => !matches!(
            &**o,
            BslObject::Array(_) | BslObject::Structure(_) | BslObject::Map(_)
        ),
        BslValue::Type(_) | BslValue::Enum(_) | BslValue::EnumType(_) | BslValue::Skipped => true,
    }
}

/// Прочтение параметра `Отказ` из финального слота функции преобразования.
///
/// ИЗМЕРЕНО на 8.3.27 четырнадцатью пробами: платформа читает `Отказ` по
/// ОБЫЧНЫМ правилам условия языка, а значение, которое к условию не
/// приводится, отказом не считает. Отказом обернулись `Истина`, `1`, `-1` и
/// строка `"да"`; НЕ обернулись `Ложь`, `0`, `""`, `"   "`, `"абв"`,
/// `Неопределено`, `Null`, пустая и непустая дата, пустой и непустой
/// массив, `Тип("Строка")`. Это ровно [`BslValue::as_condition`] с
/// подавленной ошибкой — включая её нетривиальную часть про строки
/// (истинны только слова «Да»/«Истина»/«True», а не «непустая строка»).
fn refused(final_params: &[BslValue]) -> bool {
    final_params
        .get(3)
        .and_then(|v| v.as_condition().ok())
        .unwrap_or(false)
}

/// Готовит значение к записи в позиции `property`: решает, нужна ли функция
/// преобразования, и зовёт её.
///
/// # Errors
///
/// Ошибку вызова функции (нет такой, не то число параметров) и любое
/// исключение изнутри неё — ИЗМЕРЕНО, что платформа их не глотает.
fn prepare(
    value: &BslValue,
    property: Option<&str>,
    ctx: &mut SerializeCtx<'_, '_>,
) -> RtResult<Prepared> {
    if !needs_convert(value) {
        return Ok(Prepared::AsIs);
    }
    let Some(convert) = ctx.convert.as_mut() else {
        // Звать некого — ошибку выдаст сам `serialize`, чтобы точка отказа
        // была одна.
        return Ok(Prepared::AsIs);
    };
    let args = vec![
        property_arg(property),
        value.clone(),
        convert.extra.clone(),
        BslValue::Boolean(false),
    ];
    let (returned, final_params) = (convert.call)(&convert.name, args)?;
    if refused(&final_params) {
        return Ok(Prepared::Skip);
    }
    Ok(Prepared::Converted(returned))
}

/// Запись значения, УЖЕ прошедшего функцию преобразования.
///
/// ИЗМЕРЕНО, что второй раз на том же месте платформа функцию не зовёт:
/// функция, возвращающая снова `ТаблицаЗначений`, вызывается ровно один раз
/// и запись падает обычной ошибкой типа. При этом возвращённый КОНТЕЙНЕР
/// обходится как обычно — функция, возвращающая `Структура("вложенное",
/// ТаблицаЗначений)`, вызывается на каждом следующем уровне вложенности.
/// Отсюда и разделение: подавляется вызов ровно на этой позиции, а не во
/// всём поддереве.
fn serialize_converted(
    w: &mut JsonWriter,
    value: &BslValue,
    ctx: &mut SerializeCtx<'_, '_>,
    depth: usize,
) -> RtResult<()> {
    if needs_convert(value) {
        return Err(unsupported_value_error());
    }
    serialize(w, value, ctx, depth)
}

/// Записывает один элемент контейнера, пропуская его при отказе.
///
/// `property` — имя, под которым элемент лежит (`None` для элемента
/// массива); `name` — имя свойства, которое надо написать ПЕРЕД значением,
/// но только если значение в документ попадёт: ИЗМЕРЕНО, что отказ убирает
/// свойство целиком (`{"а": ТаблицаЗначений}` -> `{}`), а не оставляет его
/// с `null`.
fn serialize_member(
    w: &mut JsonWriter,
    name: Option<&str>,
    value: &BslValue,
    ctx: &mut SerializeCtx<'_, '_>,
    depth: usize,
) -> RtResult<()> {
    let prepared = prepare(value, name, ctx)?;
    if matches!(prepared, Prepared::Skip) {
        return Ok(());
    }
    if let Some(name) = name {
        w.property_name(name)?;
    }
    match prepared {
        Prepared::AsIs => serialize(w, value, ctx, depth),
        Prepared::Converted(v) => serialize_converted(w, &v, ctx, depth),
        Prepared::Skip => unreachable!("отказ обработан выше"),
    }
}

/// Общее ядро `ЗаписатьJSON`/`ЗаписатьЗначениеJSON`.
///
/// # Errors
///
/// [`RtError::TypeError`] на значении, которое сериализовать нечем
/// (`ТаблицаЗначений` и прочие объекты, а при `single_value_mode` — ещё и
/// любая `Дата`/`Соответствие`) — измерено, платформа тоже отвергает;
/// [`RtError::StackOverflow`] на слишком глубокой вложенности (см.
/// `MAX_JSON_DEPTH`); ошибку [`format_json_date`] на настройках даты,
/// запрещающих сочетание формата и варианта записи; ошибку вызова функции
/// преобразования и любое исключение из неё.
fn serialize(
    w: &mut JsonWriter,
    value: &BslValue,
    ctx: &mut SerializeCtx<'_, '_>,
    depth: usize,
) -> RtResult<()> {
    // Рекурсия возможна только на контейнерах, поэтому на скалярах предел
    // не срабатывает никогда; проверка стоит одного сравнения `depth`.
    if depth > MAX_JSON_DEPTH && matches!(value, BslValue::Object(_)) {
        return Err(RtError::StackOverflow {
            what: "слишком глубокая вложенность данных при записи JSON \
                   (возможна циклическая ссылка)",
        });
    }
    match value {
        BslValue::Str(_) | BslValue::Number(_) | BslValue::Boolean(_) => w.value(value),
        // `Неопределено` и `Null` отдельным ЗаписатьЗначение платформа не
        // принимает, но в составе сериализуемого значения они обязаны во
        // что-то превращаться — в `null`.
        BslValue::Undefined | BslValue::Null => {
            w.literal("null");
            Ok(())
        }
        BslValue::Date(d) => {
            if ctx.single_value_mode {
                return Err(RtError::TypeError {
                    expected: "значение без Даты (ЗаписатьЗначениеJSON её не сериализует)",
                    op: "ЗаписатьЗначениеJSON",
                });
            }
            // ИЗМЕРЕНО: Microsoft-формат пишется БЕЗ обратных косых
            // (`/Date(мс)/`), так что в содержимом нет символов, которые
            // экранирование JSON вообще трогает (см. `format_json_date`) —
            // обычный `JsonWriter::value` (с проверкой контекста и
            // стандартным экранированием строки) безопасен для всех трёх
            // форматов даты. Какой вид вложенная Microsoft-дата примет в
            // ДОКУМЕНТЕ через `НастройкиСериализацииJSON`, замерит фикстура
            // (проба уже есть в `json-dates.bsl`).
            let content =
                format_json_date(*d, ctx.settings.date_format, ctx.settings.date_variant)?;
            w.value(&BslValue::Str(bsl_rt::BslString::from_str(&content)))
        }
        BslValue::Object(o) => match &**o {
            BslObject::Array(items) => {
                // Снимок до записи: элемент может оказаться тем же
                // массивом, а `RefCell` вложенного заимствования не
                // переживёт.
                let snapshot: Vec<BslValue> = items.borrow().clone();
                if ctx.settings.arrays_as_objects {
                    // `СериализовыватьМассивыКакОбъекты`: индексы уходят
                    // строковыми именами свойств `"0"`, `"1"`, ...
                    // ИЗМЕРЕНО, что и функция преобразования получает в
                    // `Свойство` этот самый индекс строкой (`"1"`), а не
                    // `Неопределено`, как для настоящего элемента массива:
                    // она видит документ таким, каким он ПИШЕТСЯ.
                    w.begin_object()?;
                    for (i, item) in snapshot.iter().enumerate() {
                        serialize_member(w, Some(&i.to_string()), item, ctx, depth + 1)?;
                    }
                    w.end_object()
                } else {
                    w.begin_array()?;
                    for item in &snapshot {
                        serialize_member(w, None, item, ctx, depth + 1)?;
                    }
                    w.end_array()
                }
            }
            BslObject::Structure(s) => {
                let entries: Vec<(String, BslValue)> = {
                    let s = s.borrow();
                    (0..s.len())
                        .filter_map(|i| s.entry_at(i))
                        .filter_map(|(id, v)| ctx.rt.names.name(id).map(|n| (n.to_string(), v)))
                        .collect()
                };
                w.begin_object()?;
                for (name, v) in &entries {
                    serialize_member(w, Some(name), v, ctx, depth + 1)?;
                }
                w.end_object()
            }
            BslObject::Map(data) => {
                // ИЗМЕРЕНО: `ЗаписатьЗначениеJSON(Соответствие)` — исключение
                // на платформе, вопреки таблице сериализуемых типов из статьи
                // 16.2.1 (снято прогоном фикстуры `json-dates`); `ЗаписатьJSON`
                // с `Соответствие` при этом работает и измерен отдельно
                // (`JSON.SERIALIZE.NESTED`) — отличие именно в
                // `ЗаписатьЗначениеJSON`, а не в объектной технике сериализации.
                if ctx.single_value_mode {
                    return Err(RtError::TypeError {
                        expected: "значение без Соответствия (ЗаписатьЗначениеJSON его не сериализует)",
                        op: "ЗаписатьЗначениеJSON",
                    });
                }
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
                            });
                        }
                    };
                    // Функция преобразования зовётся только на ЗНАЧЕНИИ —
                    // ИЗМЕРЕНО, что несериализуемый КЛЮЧ до неё не доходит
                    // («Недопустимый тип значения ключа элемента
                    // соответствия»), даже когда функция задана.
                    serialize_member(w, Some(&name), v, ctx, depth + 1)?;
                }
                w.end_object()
            }
            _ => Err(unsupported_value_error()),
        },
        _ => Err(unsupported_value_error()),
    }
}

fn component_read_json(
    context: &mut CallContext<'_>,
    arguments: &[BslValue],
) -> RtResult<BslValue> {
    let (runtime, stdout, stderr, caller) = context.execution_parts();
    match caller {
        Some(caller) => {
            let mut call = |name: &str, values: Vec<BslValue>| {
                caller(name, values, &mut *stdout, &mut *stderr)
            };
            read_json_builtin(arguments, runtime, Some(&mut call))
        }
        None => read_json_builtin(arguments, runtime, None),
    }
}

fn component_write_json(
    context: &mut CallContext<'_>,
    arguments: &[BslValue],
) -> RtResult<BslValue> {
    let (runtime, stdout, stderr, caller) = context.execution_parts();
    match caller {
        Some(caller) => {
            let mut call = |name: &str, values: Vec<BslValue>| {
                caller(name, values, &mut *stdout, &mut *stderr)
            };
            write_json_builtin(arguments, runtime, Some(&mut call))
        }
        None => write_json_builtin(arguments, runtime, None),
    }
}

fn component_write_json_date(
    _context: &mut CallContext<'_>,
    arguments: &[BslValue],
) -> RtResult<BslValue> {
    write_json_date(
        &arguments[0],
        &arguments[1],
        arguments.get(2).unwrap_or(&BslValue::Undefined),
    )
}

fn component_read_json_date(
    _context: &mut CallContext<'_>,
    arguments: &[BslValue],
) -> RtResult<BslValue> {
    read_json_date(&arguments[0], &arguments[1])
}

fn component_write_json_value(
    context: &mut CallContext<'_>,
    arguments: &[BslValue],
) -> RtResult<BslValue> {
    write_json_value(&arguments[0], context.runtime_shapes())
}

fn component_read_json_value(
    context: &mut CallContext<'_>,
    arguments: &[BslValue],
) -> RtResult<BslValue> {
    read_json_value(&arguments[0], context.runtime_shapes())
}

fn construct_reader(_context: &mut CallContext<'_>, _arguments: &[BslValue]) -> RtResult<BslValue> {
    Ok(new_json_reader())
}

fn construct_writer(_context: &mut CallContext<'_>, _arguments: &[BslValue]) -> RtResult<BslValue> {
    Ok(new_json_writer())
}

fn construct_writer_settings(
    _context: &mut CallContext<'_>,
    arguments: &[BslValue],
) -> RtResult<BslValue> {
    new_json_writer_settings(arguments)
}

fn construct_serializer_settings(
    _context: &mut CallContext<'_>,
    _arguments: &[BslValue],
) -> RtResult<BslValue> {
    Ok(new_json_serializer_settings())
}

const FUNCTIONS: &[FunctionDescriptor] = &[
    FunctionDescriptor {
        code: FunctionCode::new(1),
        names: &["ПрочитатьJSON", "ReadJSON"],
        arity: Arity::range(1, 8),
        kind: FunctionKind::Function,
        call: component_read_json,
    },
    FunctionDescriptor {
        code: FunctionCode::new(2),
        names: &["ЗаписатьJSON", "WriteJSON"],
        arity: Arity::range(2, 6),
        kind: FunctionKind::Procedure,
        call: component_write_json,
    },
    FunctionDescriptor {
        code: FunctionCode::new(3),
        names: &["ЗаписатьДатуJSON", "WriteJSONDate"],
        arity: Arity::range(2, 3),
        kind: FunctionKind::Function,
        call: component_write_json_date,
    },
    FunctionDescriptor {
        code: FunctionCode::new(4),
        names: &["ПрочитатьДатуJSON", "ReadJSONDate"],
        arity: Arity::exact(2),
        kind: FunctionKind::Function,
        call: component_read_json_date,
    },
    FunctionDescriptor {
        code: FunctionCode::new(5),
        names: &["ЗаписатьЗначениеJSON", "WriteJSONValue"],
        arity: Arity::exact(1),
        kind: FunctionKind::Function,
        call: component_write_json_value,
    },
    FunctionDescriptor {
        code: FunctionCode::new(6),
        names: &["ПрочитатьЗначениеJSON", "ReadJSONValue"],
        arity: Arity::exact(1),
        kind: FunctionKind::Function,
        call: component_read_json_value,
    },
];

const CONSTRUCTORS: &[ConstructorDescriptor] = &[
    ConstructorDescriptor {
        code: ConstructorCode::new(1),
        names: &["ЧтениеJSON", "JSONReader"],
        arity: Arity::exact(0),
        call: construct_reader,
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(2),
        names: &["ЗаписьJSON", "JSONWriter"],
        arity: Arity::exact(0),
        call: construct_writer,
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(3),
        names: &["ПараметрыЗаписиJSON", "JSONWriterSettings"],
        arity: Arity::range(0, 2),
        call: construct_writer_settings,
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(4),
        names: &["НастройкиСериализацииJSON", "JSONSerializerSettings"],
        arity: Arity::exact(0),
        call: construct_serializer_settings,
    },
];

/// Дескриптор статически подключаемого JSON-компонента.
pub const fn library() -> LibraryDescriptor {
    LibraryDescriptor {
        package: env!("CARGO_PKG_NAME"),
        version: env!("CARGO_PKG_VERSION"),
        // Ядро в зависимостях не объявляется: реестр включает его в
        // требования любой программы (`RuntimeRegistry::requirements_for`).
        dependencies: &[],
        functions: FUNCTIONS,
        constructors: CONSTRUCTORS,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn published_component_codes_are_stable() {
        let descriptor = library();
        assert_eq!(
            descriptor
                .functions
                .iter()
                .map(|function| function.code.get())
                .collect::<Vec<_>>(),
            [1, 2, 3, 4, 5, 6]
        );
        assert_eq!(
            descriptor
                .constructors
                .iter()
                .map(|constructor| constructor.code.get())
                .collect::<Vec<_>>(),
            [1, 2, 3, 4]
        );
    }

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

    /// Контекст сборки без функции восстановления — то, чем был
    /// `build_value` до появления колбэков.
    fn plain_build_ctx(rt: &mut RuntimeShapes) -> BuildCtx<'_, 'static> {
        BuildCtx {
            as_map: false,
            date_names: &[],
            date_format: None,
            restore: None,
            rt,
            cache: JsonBuildCache::default(),
        }
    }

    /// Контекст сериализации без функции преобразования.
    fn plain_serialize_ctx<'a>(
        settings: &'a JsonSerializerSettings,
        rt: &'a RuntimeShapes,
    ) -> SerializeCtx<'a, 'static> {
        SerializeCtx {
            settings,
            single_value_mode: false,
            convert: None,
            rt,
        }
    }

    // НЕ ИЗМЕРЕНО(JSON.MAX_DEPTH) — тесты фиксируют ВЫБРАННОЕ поведение:
    // перехватываемая ошибка вместо переполнения стека процесса; предел
    // платформы не замерен.
    #[test]
    fn too_deep_json_document_is_an_error_not_a_crash() {
        let depth = MAX_JSON_DEPTH + 100;
        let text = format!("{}1{}", "[".repeat(depth), "]".repeat(depth));
        let mut rt = RuntimeShapes::seeded(Vec::new(), Vec::new());
        let mut parser = JsonParser::new(&text);
        let first = parser.next_event().unwrap().unwrap();
        let e =
            build_value(first, &mut parser, None, &mut plain_build_ctx(&mut rt), 0).unwrap_err();
        assert!(matches!(e, RtError::StackOverflow { .. }), "{e:?}");
    }

    #[test]
    fn json_document_below_the_depth_limit_still_reads() {
        // 400 уровней — нижняя граница из замера: обязана работать.
        let depth = 400;
        let text = format!("{}1{}", "[".repeat(depth), "]".repeat(depth));
        let mut rt = RuntimeShapes::seeded(Vec::new(), Vec::new());
        let mut parser = JsonParser::new(&text);
        let first = parser.next_event().unwrap().unwrap();
        build_value(first, &mut parser, None, &mut plain_build_ctx(&mut rt), 0)
            .expect("глубина ниже предела обязана читаться");
    }

    #[test]
    fn cyclic_value_in_write_json_is_an_error_not_a_crash() {
        // Массив, содержащий сам себя, — бесконечная глубина: без предела
        // `serialize` рекурсировал бы до переполнения стека процесса.
        let arr = BslValue::new_array(Vec::new());
        arr.push_element(arr.clone()).unwrap();
        let rt = RuntimeShapes::seeded(Vec::new(), Vec::new());
        let mut w = JsonWriter::to_string_target(settings_from(None).unwrap());
        let settings = JsonSerializerSettings::default();
        let e = serialize(&mut w, &arr, &mut plain_serialize_ctx(&settings, &rt), 0).unwrap_err();
        assert!(matches!(e, RtError::StackOverflow { .. }), "{e:?}");
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
        let field_count = bsl_rt::MAX_SHAPE_TRANSITIONS as usize + 1;
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
        let source = bsl_rt::BslString::from_str("[1]");
        let mut parser = JsonParser::from_bsl_string(&source);

        let changed = source.append(&bsl_rt::BslString::from_str("мусор"));
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
        w.value(&BslValue::Str(bsl_rt::BslString::from_str("текст")))
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
            w.value(&BslValue::Str(bsl_rt::BslString::from_str(
                "\"\\/\n\tЁж\u{1}",
            )))
        });
        assert_eq!(s, "\"\\\"\\\\/\\n\\u0009Ёж\\u0001\"");

        let s = write_compact(|w| {
            w.value(&BslValue::Str(bsl_rt::BslString::from_str(
                "\r\u{8}\u{c}\u{b}",
            )))
        });
        assert_eq!(s, "\"\\r\\u0008\\u000C\\u000B\"");

        let isolated_surrogate = bsl_rt::BslString::from_str("😀").left(1);
        let s = write_compact(|w| w.value(&BslValue::Str(isolated_surrogate)));
        assert_eq!(s, "\"�\"");
    }

    #[test]
    fn bsl_property_name_uses_the_same_escaping_rules() {
        let name = bsl_rt::BslString::from_str("имя\"\t😀");
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
            BslValue::Date(bsl_rt::BslDate::from_seconds(0).unwrap()),
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

    // --- ПроверятьСтруктуру -------------------------------------------

    #[test]
    fn check_structure_defaults_to_true() {
        // ИЗМЕРЕНО (JSON.WRITE.CHECK_STRUCTURE_DEFAULT): «Да».
        let w = JsonWriter::to_string_target(JsonWriterSettings::default());
        assert!(w.check_structure());
    }

    #[test]
    fn disabling_check_structure_does_not_lift_any_known_check() {
        // ИЗМЕРЕНО (`JSON.WRITE.CHECK_STRUCTURE_OFF`): все известные
        // проверки структуры документа остаются ошибками даже при
        // `ПроверятьСтруктуру = Ложь` — ни одна из них этой настройкой не
        // управляется (что она отключает — открытый вопрос).
        let mut w = JsonWriter::to_string_target(settings_from(None).unwrap());
        w.set_check_structure(false);
        w.begin_object().unwrap();
        assert!(w.value(&BslValue::Number(num("1"))).is_err());

        let mut w2 = JsonWriter::to_string_target(settings_from(None).unwrap());
        w2.set_check_structure(false);
        assert!(w2.end_array().is_err());
        assert!(w2.end_object().is_err());

        let mut w3 = JsonWriter::to_string_target(settings_from(None).unwrap());
        w3.set_check_structure(false);
        assert!(w3.property_name("х").is_err());
    }

    // --- ЗаписатьДатуJSON / ПрочитатьДатуJSON --------------------------

    fn civil(y: i64, m: u32, d: u32, h: u32, mi: u32, s: u32) -> bsl_rt::BslDate {
        bsl_rt::BslDate::from_civil(y, m, d, h, mi, s).unwrap()
    }

    #[test]
    fn write_json_date_local_variant_is_iso_without_zone() {
        let d = civil(2014, 5, 10, 13, 14, 15);
        let s = format_json_date(d, JsonDateFormat::Iso, JsonDateWritingVariant::Local).unwrap();
        assert_eq!(s, "2014-05-10T13:14:15");
    }

    #[test]
    fn write_json_date_local_offset_variant_appends_the_machine_offset() {
        let d = civil(2014, 5, 10, 13, 14, 15);
        let s =
            format_json_date(d, JsonDateFormat::Iso, JsonDateWritingVariant::LocalOffset).unwrap();
        let offset = bsl_rt::local_offset_seconds(pseudo_unix_seconds(d));
        assert_eq!(s, format!("2014-05-10T13:14:15{}", format_offset(offset)));
    }

    #[test]
    fn write_json_date_universal_variant_covers_all_three_formats() {
        let d = civil(2014, 5, 10, 13, 14, 15);
        let pseudo = pseudo_unix_seconds(d);
        let offset = bsl_rt::local_offset_seconds(pseudo);
        let utc_unix = pseudo - i64::from(offset);

        let iso = format_json_date(d, JsonDateFormat::Iso, JsonDateWritingVariant::Universal)
            .expect("универсальный ISO обязан построиться");
        let utc_civil = unix_to_bsl_date(utc_unix, "test").unwrap().to_civil();
        assert_eq!(
            iso,
            format!(
                "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
                utc_civil.year,
                utc_civil.month,
                utc_civil.day,
                utc_civil.hour,
                utc_civil.minute,
                utc_civil.second
            )
        );

        let js = format_json_date(
            d,
            JsonDateFormat::JavaScript,
            JsonDateWritingVariant::Universal,
        )
        .unwrap();
        assert_eq!(js, format!("new Date({})", utc_unix * 1000));

        // ИЗМЕРЕНО: БЕЗ обратных косых — `/Date(мс)/`, не `\/Date(мс)\/`,
        // как ошибочно предполагалось до замера.
        let ms = format_json_date(
            d,
            JsonDateFormat::Microsoft,
            JsonDateWritingVariant::Universal,
        )
        .unwrap();
        assert_eq!(ms, format!("/Date({})/", utc_unix * 1000));
        assert!(!ms.contains('\\'), "обратных косых быть не должно: {ms}");
    }

    /// Документ, а не только `format_json_date`, обязан нести дату слово в
    /// слово (в кавычках) — раз в содержимом больше нет символов,
    /// требующих экранирования JSON, обычный `JsonWriter::value` ничего в
    /// нём не меняет.
    #[test]
    fn nested_microsoft_date_matches_the_standalone_content_in_the_document() {
        let mut rt = RuntimeShapes::seeded(Vec::new(), Vec::new());
        let id = rt.names.intern("д");
        let d = civil(2014, 5, 10, 13, 14, 15);
        let structure = BslValue::new_structure(rt.shapes.empty(), Vec::new());
        structure
            .structure_insert(id, BslValue::Date(d), &mut rt.shapes)
            .unwrap();

        let settings = JsonSerializerSettings {
            date_format: JsonDateFormat::Microsoft,
            date_variant: JsonDateWritingVariant::Universal,
            arrays_as_objects: false,
        };
        let content = format_json_date(d, settings.date_format, settings.date_variant).unwrap();

        let mut w = JsonWriter::to_string_target(settings_from(None).unwrap());
        serialize(
            &mut w,
            &structure,
            &mut plain_serialize_ctx(&settings, &rt),
            0,
        )
        .unwrap();
        let text = w.finish().unwrap();

        assert_eq!(text, format!("{{\n\"д\": \"{content}\"\n}}"));
        assert!(
            !text.contains('\\'),
            "обратных косых быть не должно: {text}"
        );
    }

    #[test]
    fn non_iso_format_requires_the_universal_variant() {
        // Замер JSON.WRITE_DATE.NON_ISO_LOCAL_ERROR: выбранное поведение —
        // ошибка, а не тихая подстановка ISO.
        let d = civil(2024, 1, 1, 0, 0, 0);
        assert!(
            format_json_date(d, JsonDateFormat::JavaScript, JsonDateWritingVariant::Local).is_err()
        );
        assert!(
            format_json_date(
                d,
                JsonDateFormat::Microsoft,
                JsonDateWritingVariant::LocalOffset
            )
            .is_err()
        );
        assert!(
            format_json_date(
                d,
                JsonDateFormat::JavaScript,
                JsonDateWritingVariant::Universal
            )
            .is_ok()
        );
    }

    #[test]
    fn empty_date_formats_without_error_in_local_variants() {
        // Граница диапазона: пустая дата, оба варианта без пересчёта в UTC.
        let d = bsl_rt::BslDate::empty();
        assert_eq!(
            format_json_date(d, JsonDateFormat::Iso, JsonDateWritingVariant::Local).unwrap(),
            "0001-01-01T00:00:00"
        );
        assert!(
            format_json_date(d, JsonDateFormat::Iso, JsonDateWritingVariant::LocalOffset).is_ok()
        );
    }

    /// ИЗМЕРЕНО: `ЗаписатьДатуJSON(Дата(1,1,1), ISO, УниверсальнаяДата)` на
    /// платформе даёт `0001-01-01T00:00:00Z`, а не ошибку — вычитание
    /// смещения машины клампится к полу диапазона.
    #[test]
    fn universal_variant_of_the_empty_date_clamps_to_the_floor_instead_of_erroring() {
        let d = bsl_rt::BslDate::empty();
        let text = format_json_date(d, JsonDateFormat::Iso, JsonDateWritingVariant::Universal)
            .expect("клампится, а не падает");
        assert!(
            text.starts_with("0001-01-01T") && text.ends_with('Z'),
            "{text}"
        );
        // На восточном (неотрицательном) смещении платформа измерена ТОЧНО
        // на полу; при отрицательном смещении вычитание и так не уходит за
        // пол, кламп там — no-op, и точное значение не измерено.
        let offset = bsl_rt::local_offset_seconds(pseudo_unix_seconds(d));
        if offset >= 0 {
            assert_eq!(text, "0001-01-01T00:00:00Z");
        }
    }

    #[test]
    fn read_json_date_parses_iso_without_zone() {
        let d = civil(2014, 5, 10, 13, 14, 15);
        assert_eq!(
            parse_json_date_by_format("2014-05-10T13:14:15", JsonDateFormat::Iso),
            Some(d)
        );
    }

    #[test]
    fn read_json_date_parses_iso_with_z_as_a_utc_moment() {
        // ИЗМЕРЕНО по примеру статьи: момент UTC переводится в локальное
        // время МАШИНЫ — то, что и делает `utc_millis_to_local_date`.
        let utc = civil(2014, 5, 10, 9, 14, 15);
        let expected = utc_millis_to_local_date(pseudo_unix_seconds(utc) * 1000).unwrap();
        assert_eq!(
            parse_json_date_by_format("2014-05-10T09:14:15Z", JsonDateFormat::Iso),
            Some(expected)
        );
    }

    #[test]
    fn read_json_date_parses_javascript_and_microsoft_examples_from_the_article() {
        // "new Date(1411464940000)" -> 23.09.2014 13:35:40 при UTC+4
        // (пример из статьи) — здесь проверяется не конкретная зона (она
        // зависит от машины), а то, что оба формата дают ОДИН И ТОТ ЖЕ
        // момент. Microsoft — БЕЗ обратных косых (см. `format_json_date`).
        let ms = 1_411_464_940_000i64;
        let expected = utc_millis_to_local_date(ms).unwrap();
        assert_eq!(
            parse_json_date_by_format("new Date(1411464940000)", JsonDateFormat::JavaScript),
            Some(expected)
        );
        assert_eq!(
            parse_json_date_by_format("/Date(1411464940000)/", JsonDateFormat::Microsoft),
            Some(expected)
        );
    }

    /// ИЗМЕРЕНО: платформа кидает исключение на написании с обратными
    /// косыми — до замера здесь снисходительно принимались оба написания,
    /// это оказалось лишним. Единственная принимаемая форма — без них.
    #[test]
    fn read_json_date_rejects_microsoft_format_with_backslashes() {
        assert_eq!(
            parse_json_date_by_format("\\/Date(1411464940000)\\/", JsonDateFormat::Microsoft),
            None
        );
    }

    #[test]
    fn read_json_date_round_trips_every_writing_variant() {
        let d = civil(2014, 5, 10, 13, 14, 15);
        for variant in [
            JsonDateWritingVariant::Local,
            JsonDateWritingVariant::LocalOffset,
            JsonDateWritingVariant::Universal,
        ] {
            let text = format_json_date(d, JsonDateFormat::Iso, variant).unwrap();
            let back = parse_json_date_by_format(&text, JsonDateFormat::Iso);
            assert_eq!(back, Some(d), "вариант {variant:?}, текст {text:?}");
        }
    }

    #[test]
    fn read_json_date_rejects_garbage_in_every_format() {
        assert_eq!(
            parse_json_date_by_format("совсем не дата", JsonDateFormat::Iso),
            None
        );
        assert_eq!(
            parse_json_date_by_format("new Date(не число)", JsonDateFormat::JavaScript),
            None
        );
        assert_eq!(
            parse_json_date_by_format("Date(123)", JsonDateFormat::Microsoft),
            None,
            "без ведущей `\\/` — не микрософтовский формат"
        );
    }

    /// Округление краёв: доли миллисекунды НИЖЕ секунды отбрасываются к
    /// РАНЕЕ идущей секунде (`div_euclid`, а не усечение к нулю) —
    /// симметрично и на положительной, и на отрицательной стороне эпохи.
    #[test]
    fn millisecond_fraction_rounds_toward_the_earlier_second_on_both_sides_of_the_epoch() {
        assert_eq!(
            parse_json_date_by_format("new Date(999)", JsonDateFormat::JavaScript),
            parse_json_date_by_format("new Date(0)", JsonDateFormat::JavaScript)
        );
        assert_eq!(
            parse_json_date_by_format("new Date(-1)", JsonDateFormat::JavaScript),
            parse_json_date_by_format("new Date(-1000)", JsonDateFormat::JavaScript)
        );
    }

    #[test]
    fn write_json_date_rejects_wrong_argument_types() {
        let format = BslValue::Enum(EnumValue::DateFormatIso);
        assert!(write_json_date(&BslValue::Undefined, &format, &BslValue::Undefined).is_err());
        let date = BslValue::Date(bsl_rt::BslDate::empty());
        assert!(write_json_date(&date, &BslValue::Undefined, &BslValue::Undefined).is_err());
    }

    #[test]
    fn read_json_date_reports_the_chosen_error_text() {
        // Замер JSON.READ_DATE.BAD_FORMAT_TEXT — фиксирует ВЫБРАННЫЙ текст.
        let text = BslValue::Str(bsl_rt::BslString::from_str("мусор"));
        let format = BslValue::Enum(EnumValue::DateFormatIso);
        let e = read_json_date(&text, &format).unwrap_err();
        assert_eq!(e.to_string(), "Представление даты имеет неверный формат");
    }

    // --- НастройкиСериализацииJSON --------------------------------------

    #[test]
    fn serializer_settings_default_is_iso_local_and_arrays_stay_arrays() {
        let s = JsonSerializerSettings::default();
        assert_eq!(s.date_format, JsonDateFormat::Iso);
        assert_eq!(s.date_variant, JsonDateWritingVariant::Local);
        assert!(!s.arrays_as_objects);
    }

    #[test]
    fn serializer_setting_accessors_round_trip_every_field() {
        let obj = new_json_serializer_settings();
        let obj = obj.object_ref().unwrap().as_dyn();
        set_serializer_setting(
            obj,
            "ФорматСериализацииДаты",
            BslValue::Enum(EnumValue::DateFormatMicrosoft),
        )
        .unwrap();
        set_serializer_setting(
            obj,
            "ВариантЗаписиДаты",
            BslValue::Enum(EnumValue::DateVariantUniversal),
        )
        .unwrap();
        set_serializer_setting(
            obj,
            "СериализовыватьМассивыКакОбъекты",
            BslValue::Boolean(true),
        )
        .unwrap();

        assert_eq!(
            get_serializer_setting(obj, "DateSerializationFormat").unwrap(),
            BslValue::Enum(EnumValue::DateFormatMicrosoft)
        );
        assert_eq!(
            get_serializer_setting(obj, "DateWritingVariant").unwrap(),
            BslValue::Enum(EnumValue::DateVariantUniversal)
        );
        assert_eq!(
            get_serializer_setting(obj, "SerializeArraysAsObjects").unwrap(),
            BslValue::Boolean(true)
        );
        assert!(get_serializer_setting(obj, "НетТакогоСвойства").is_err());
    }

    /// ИЗМЕРЕНО: статья 16.2.3.2 приводит `ФорматСериализацииДат` (без
    /// «ы») — опечатка статьи, живая 8.3.27 такое имя отвергает.
    #[test]
    fn the_article_typo_spelling_of_date_format_property_is_rejected() {
        let obj = new_json_serializer_settings();
        let obj = obj.object_ref().unwrap().as_dyn();
        assert!(get_serializer_setting(obj, "ФорматСериализацииДат").is_err());
    }

    // --- ЗаписатьЗначениеJSON / ПрочитатьЗначениеJSON -------------------

    #[test]
    fn write_json_value_rejects_date_at_top_level_and_nested() {
        let rt = RuntimeShapes::seeded(Vec::new(), Vec::new());
        let e = write_json_value(&BslValue::Date(bsl_rt::BslDate::empty()), &rt).unwrap_err();
        assert!(matches!(e, RtError::TypeError { .. }));

        let mut rt2 = RuntimeShapes::seeded(Vec::new(), Vec::new());
        let id = rt2.names.intern("д");
        let s = BslValue::new_structure(rt2.shapes.empty(), Vec::new());
        s.structure_insert(
            id,
            BslValue::Date(bsl_rt::BslDate::empty()),
            &mut rt2.shapes,
        )
        .unwrap();
        let e2 = write_json_value(&s, &rt2).unwrap_err();
        assert!(matches!(e2, RtError::TypeError { .. }));
    }

    /// ИЗМЕРЕНО: `ЗаписатьЗначениеJSON(Соответствие)` — исключение, тогда
    /// как `ЗаписатьJSON` с тем же `Соответствие` по-прежнему работает.
    #[test]
    fn write_json_value_rejects_a_map_while_write_json_still_accepts_it() {
        let rt = RuntimeShapes::seeded(Vec::new(), Vec::new());
        let map = BslValue::new_map();
        map.map_insert(
            BslValue::Str(bsl_rt::BslString::from_str("ключ")),
            BslValue::Number(num("1")),
        )
        .unwrap();

        let e = write_json_value(&map, &rt).unwrap_err();
        assert!(matches!(e, RtError::TypeError { .. }));

        let mut w = JsonWriter::to_string_target(settings_from(None).unwrap());
        let settings = JsonSerializerSettings::default();
        serialize(&mut w, &map, &mut plain_serialize_ctx(&settings, &rt), 0)
            .expect("ЗаписатьJSON по-прежнему сериализует Соответствие");
    }

    /// Без функции преобразования несериализуемый тип даёт обычную ошибку
    /// типа — `JSON.SERIALIZE.UNSUPPORTED_TYPE`, поведение не изменилось.
    #[test]
    fn write_json_without_a_convert_function_keeps_the_plain_type_error() {
        let rt = RuntimeShapes::seeded(Vec::new(), Vec::new());
        let writer = new_json_writer();
        set_string(writer.object_ref().unwrap().as_dyn(), &[]).unwrap();
        let table = BslValue::new_table();
        let e = write_json(
            &writer,
            &table,
            &JsonSerializerSettings::default(),
            None,
            &rt,
        )
        .unwrap_err();
        assert!(matches!(e, RtError::TypeError { .. }), "{e:?}");
    }

    /// ИЗМЕРЕНО: пустая строка в `ПрочитатьЗначениеJSON` — исключение, а
    /// не тихое `Неопределено`.
    #[test]
    fn read_json_value_rejects_an_empty_string() {
        let mut rt = RuntimeShapes::seeded(Vec::new(), Vec::new());
        let e =
            read_json_value(&BslValue::Str(bsl_rt::BslString::from_str("")), &mut rt).unwrap_err();
        assert!(matches!(e, RtError::Json(_)));
    }

    #[test]
    fn write_and_read_json_value_round_trip_scalars_and_structures() {
        let mut rt = RuntimeShapes::seeded(Vec::new(), Vec::new());
        let s = write_json_value(&BslValue::Number(num("42")), &rt).unwrap();
        assert_eq!(s, BslValue::Str(bsl_rt::BslString::from_str("42")));

        let id = rt.names.intern("а");
        let structure = BslValue::new_structure(rt.shapes.empty(), Vec::new());
        structure
            .structure_insert(id, BslValue::Number(num("1")), &mut rt.shapes)
            .unwrap();
        let text = write_json_value(&structure, &rt).unwrap();
        let BslValue::Str(text) = text else {
            panic!("ожидалась строка")
        };
        let back = read_json_value(&BslValue::Str(text), &mut rt).unwrap();
        // ИЗМЕРЕНО (JSON.VALUE.READ_KIND): «Структура».
        assert_eq!(back.type_name(), "Структура");
    }
}
