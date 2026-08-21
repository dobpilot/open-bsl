//! Разборщик JSON: события и лексика.
//!
//! Один разборщик на двоих — `ЧтениеJSON` отдаёт его события наружу по
//! одному, а `ПрочитатьJSON` собирает из тех же событий готовое значение.

use std::rc::Rc;

use bsl_rt::{BslNumber, RtResult};

use crate::bad;

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

    pub(crate) fn from_string(src: String) -> Self {
        Self::from_utf8(Rc::from(src))
    }

    pub(crate) fn from_bsl_string(src: &bsl_rt::BslString) -> Self {
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
    /// [`bsl_rt::RtError::Json`], если на месте значения стоит то, что значением
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

#[cfg(test)]
mod tests {
    use super::*;
    use bsl_rt::BslNumber;

    fn num(s: &str) -> BslNumber {
        BslNumber::parse_canonical(s).unwrap()
    }

    fn events(text: &str) -> Vec<JsonEvent> {
        let mut p = JsonParser::new(text);
        let mut out = Vec::new();
        while let Some(e) = p.next_event().expect("разбор") {
            out.push(e);
        }
        out
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
}
