//! Запись JSON: настройки, экранирование, потоковый писатель.

use bsl_rt::{BslValue, RtError, RtResult};

use bsl_rt::EnumValue;

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

/// `ЭкранированиеСимволовJSON` — общий режим экранирования Unicode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsonEscapeMode {
    None,
    NonAscii,
    NonBmp,
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

/// Полная измеренная форма `ПараметрыЗаписиJSON`.
#[derive(Debug, Clone)]
pub struct JsonWriterSettings {
    pub line_break: JsonLineBreak,
    pub indent: String,
    pub use_double_quotes: bool,
    pub escape_mode: JsonEscapeMode,
    pub escape_angle_brackets: bool,
    pub escape_line_separators: bool,
    pub escape_ampersand: bool,
    pub escape_single_quotes: bool,
    pub escape_slash: bool,
}

/// `ФорматДатыJSON` в терминах СЕРИАЛИЗАЦИИ, а не события разбора (тот —
/// [`crate::JsonEvent`]) — отдельный тип от `EnumValue`, потому что здесь удобнее
/// работать `match`ем без соседних членов `ТипЗначенияJSON`/`ПереносСтрокJSON`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsonDateFormat {
    Iso,
    JavaScript,
    Microsoft,
}

impl JsonDateFormat {
    pub(crate) fn from_enum_value(v: EnumValue) -> Option<Self> {
        match v {
            EnumValue::DateFormatIso => Some(Self::Iso),
            EnumValue::DateFormatJavaScript => Some(Self::JavaScript),
            EnumValue::DateFormatMicrosoft => Some(Self::Microsoft),
            _ => None,
        }
    }

    pub(crate) fn to_enum_value(self) -> EnumValue {
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
    pub(crate) fn from_enum_value(v: EnumValue) -> Option<Self> {
        match v {
            EnumValue::DateVariantLocal => Some(Self::Local),
            EnumValue::DateVariantLocalOffset => Some(Self::LocalOffset),
            EnumValue::DateVariantUniversal => Some(Self::Universal),
            _ => None,
        }
    }

    pub(crate) fn to_enum_value(self) -> EnumValue {
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
            use_double_quotes: true,
            escape_mode: JsonEscapeMode::None,
            escape_angle_brackets: false,
            escape_line_separators: true,
            escape_ampersand: false,
            escape_single_quotes: false,
            escape_slash: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum WCtx {
    Object,
    Array,
}

/// Состояние КАДРА контейнера — одно перечисление вместо параллельных
/// `has_member`/`awaiting_value`. «Член уже записан» (`AfterValue`) и «ждём
/// значение после имени» (`AwaitingValue`) не могут быть истинны
/// одновременно по построению: прежде их держали два булевых поля рядом, и
/// невозможная комбинация «члена нет, но значение ждём» была представима.
#[derive(Debug, Clone, Copy, PartialEq)]
enum FrameState {
    /// Контейнер пуст — ни одного члена ещё не записано.
    Empty,
    /// Последним записан член (значение либо закрытый контейнер): перед
    /// следующим нужна запятая.
    AfterValue,
    /// В объекте записано имя свойства, ждём его значение: следующий член
    /// идёт без запятой и без переноса — отступ уже поставлен именем.
    AwaitingValue,
}

/// Кадр стека контейнеров: что открыто и в каком оно состоянии.
#[derive(Debug, Clone, Copy)]
struct Frame {
    ctx: WCtx,
    state: FrameState,
}

/// Куда уйдёт результат записи — ОДИН приёмник вместо параллельных
/// `path`/`files`, где было представимо «строка, но с путём» и «файл, но без
/// файловой системы». Файловый приёмник ВСЕГДА несёт и путь, и файловую
/// систему сессии, поэтому недостижимая комбинация исчезает вместе с
/// `expect` на неё.
#[derive(Debug)]
enum WriterSink {
    /// `УстановитьСтроку`: `Закрыть()` отдаёт накопленный текст.
    String,
    /// `ОткрытьФайл`: `Закрыть()` пишет документ файловой системой сессии.
    /// Оба поля берутся у объекта при `ОткрытьФайл` и хранятся, потому что
    /// запись идёт в `Закрыть()`, который под JIT нативен и контекста уже не
    /// несёт (ABI-G).
    File {
        path: std::path::PathBuf,
        files: std::rc::Rc<dyn bsl_rt::FileSystem>,
    },
}

/// Состояние `ЗаписьJSON`.
#[derive(Debug)]
pub struct JsonWriter {
    out: String,
    settings: JsonWriterSettings,
    /// Стек открытых контейнеров: каждый несёт своё состояние
    /// (`FrameState`), чем и заменяет прежние параллельные `has_member` и
    /// `awaiting_value`.
    stack: Vec<Frame>,
    sink: WriterSink,
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
            sink: WriterSink::String,
            check_structure: true,
        }
    }

    pub fn to_file(
        path: std::path::PathBuf,
        settings: JsonWriterSettings,
        files: std::rc::Rc<dyn bsl_rt::FileSystem>,
    ) -> Self {
        let mut w = Self::to_string_target(settings);
        w.sink = WriterSink::File { path, files };
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

    /// Разделитель перед очередным членом контейнера — по СОСТОЯНИЮ верхнего
    /// кадра. Само состояние проставляет вызывающий после записи через
    /// `mark_value_written`/`mark_awaiting_value`.
    fn before_member(&mut self) {
        let Some(&Frame { state, .. }) = self.stack.last() else {
            return;
        };
        match state {
            // Значение после имени свойства: отступ уже поставлен именем.
            FrameState::AwaitingValue => {}
            FrameState::AfterValue => {
                self.out.push(',');
                self.newline(self.stack.len());
            }
            FrameState::Empty => self.newline(self.stack.len()),
        }
    }

    /// Верхний кадр: член записан (значение либо закрытый контейнер).
    fn mark_value_written(&mut self) {
        if let Some(top) = self.stack.last_mut() {
            top.state = FrameState::AfterValue;
        }
    }

    /// Верхний кадр: записано имя свойства, ждём значение.
    fn mark_awaiting_value(&mut self) {
        if let Some(top) = self.stack.last_mut() {
            top.state = FrameState::AwaitingValue;
        }
    }

    /// # Errors
    ///
    /// [`RtError::Json`], если запись нарушает структуру документа.
    pub fn begin_object(&mut self) -> RtResult<()> {
        self.before_member();
        self.out.push('{');
        // Открытый контейнер — это ЗНАЧЕНИЕ родителя: помечаем родителя, а
        // затем кладём собственный пустой кадр.
        self.mark_value_written();
        self.stack.push(Frame {
            ctx: WCtx::Object,
            state: FrameState::Empty,
        });
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
        let Some(&Frame { ctx, state }) = self.stack.last() else {
            return Err(RtError::Json(
                "ЗаписатьКонецОбъекта без открытого объекта".to_string(),
            ));
        };
        if ctx != WCtx::Object {
            return Err(RtError::Json(
                "ЗаписатьКонецОбъекта без открытого объекта".to_string(),
            ));
        }
        self.stack.pop();
        if state != FrameState::Empty {
            self.newline(self.stack.len());
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
        self.mark_value_written();
        self.stack.push(Frame {
            ctx: WCtx::Array,
            state: FrameState::Empty,
        });
        Ok(())
    }

    /// # Errors
    ///
    /// [`RtError::Json`], если открытого массива нет — БЕЗУСЛОВНО, см.
    /// `end_object`.
    pub fn end_array(&mut self) -> RtResult<()> {
        let Some(&Frame { ctx, state }) = self.stack.last() else {
            return Err(RtError::Json(
                "ЗаписатьКонецМассива без открытого массива".to_string(),
            ));
        };
        if ctx != WCtx::Array {
            return Err(RtError::Json(
                "ЗаписатьКонецМассива без открытого массива".to_string(),
            ));
        }
        self.stack.pop();
        if state != FrameState::Empty {
            self.newline(self.stack.len());
        }
        self.out.push(']');
        Ok(())
    }

    /// Открывает имя свойства и ставит начальную кавычку.
    ///
    /// ИЗМЕРЕНО: проверка контекста БЕЗУСЛОВНАЯ, см. `end_object`.
    fn begin_property_name(&mut self) -> RtResult<()> {
        if self.stack.last().map(|f| f.ctx) != Some(WCtx::Object) {
            return Err(RtError::Json("ЗаписатьИмяСвойства вне объекта".to_string()));
        }
        self.before_member();
        self.out.push(self.quote());
        Ok(())
    }

    /// Закрывает имя свойства и переводит автомат в режим ожидания
    /// значения.
    fn finish_property_name(&mut self) {
        self.out.push(self.quote());
        self.out.push(':');
        // Пробел после двоеточия ставится ТОЛЬКО в форматированном режиме:
        // измерено, что `ПереносСтрокJSON.Нет` даёт `{"а":1}` без пробела,
        // а умолчание — `"а": 1` с пробелом.
        if self.pretty() {
            self.out.push(' ');
        }
        self.mark_awaiting_value();
    }

    /// # Errors
    ///
    /// [`RtError::Json`], если мы не внутри объекта.
    pub fn property_name(&mut self, name: &str) -> RtResult<()> {
        self.begin_property_name()?;
        escape_into(&mut self.out, name, &self.settings);
        self.finish_property_name();
        Ok(())
    }

    /// То же имя свойства, но прямо из внутреннего UTF-16 `BslString`.
    pub(crate) fn property_name_bsl(&mut self, name: &bsl_rt::BslString) -> RtResult<()> {
        self.begin_property_name()?;
        escape_bsl_string_into(&mut self.out, name, &self.settings);
        self.finish_property_name();
        Ok(())
    }

    /// Готовый текст значения (строка уже с кавычками и экранированием).
    fn raw_value(&mut self, text: &str) {
        self.before_member();
        self.out.push_str(text);
        self.mark_value_written();
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
        if let Some(top) = self.stack.last()
            && top.ctx == WCtx::Object
            && top.state != FrameState::AwaitingValue
        {
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
                self.out.push(self.quote());
                escape_bsl_string_into(&mut self.out, s, &self.settings);
                self.out.push(self.quote());
            }
            BslValue::Number(n) => self.out.push_str(&n.to_canonical()),
            BslValue::Boolean(true) => self.out.push_str("true"),
            BslValue::Boolean(false) => self.out.push_str("false"),
            _ => unreachable!("типы проверены выше"),
        }
        self.mark_value_written();
        Ok(())
    }

    /// Значение, которое умеет писать только `ЗаписатьJSON`: `null` из
    /// `Неопределено`/`Null` и дата строкой.
    pub fn literal(&mut self, text: &str) {
        self.raw_value(text);
    }

    fn quote(&self) -> char {
        if self.settings.use_double_quotes {
            '"'
        } else {
            '\''
        }
    }

    pub fn is_string_target(&self) -> bool {
        matches!(self.sink, WriterSink::String)
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
        // Клонируем путь и (дёшево, счётчик ссылок) файловую систему, чтобы
        // не держать заимствование `self.sink` во время записи и снятия
        // приёмника ниже.
        let (path, files) = match &self.sink {
            WriterSink::String => return Ok(std::mem::take(&mut self.out)),
            WriterSink::File { path, files } => (path.clone(), files.clone()),
        };
        // НЕ ИЗМЕРЕНО(JSON.WRITE.CLOSE_IO_FAIL): как ведёт себя писатель JSON
        // платформы после отказа ФС в `Закрыть()`. Здесь выбрано: писатель
        // остаётся файловым и повторный `Закрыть()` пробует снова, а не
        // отдаёт документ текстом. Поведение второго `Закрыть()` платформы
        // не снято (см. `measure-all.bsl`).
        // Приёмник становится строковым (и буфер чистится) ТОЛЬКО после
        // успеха: `?` ниже выходит раньше присваивания, оставляя `sink`
        // файловым. Прежде путь снимался ДО `write`, и на отказе ФС писатель
        // молча становился строковым — повторный `Закрыть()` отдавал весь
        // документ текстом, тем самым ответом, который рядом означает
        // успешную запись в файл.
        files
            .write(&path.to_string_lossy(), self.out.as_bytes())
            .map_err(|e| RtError::IoError(format!("{}: {e}", path.display())))?;
        self.out.clear();
        self.sink = WriterSink::String;
        Ok(String::new())
    }
}

/// Экранирование по измеренному набору правил (см. обзор модуля).
fn escape_into(out: &mut String, s: &str, settings: &JsonWriterSettings) {
    let units = s.encode_utf16().collect::<Vec<_>>();
    escape_utf16_into(out, &units, settings);
}

/// Экранирует `BslString` без промежуточного UTF-8 `String`.
fn escape_bsl_string_into(out: &mut String, s: &bsl_rt::BslString, settings: &JsonWriterSettings) {
    escape_utf16_into(out, s.units(), settings);
}

fn escape_utf16_into(out: &mut String, units: &[u16], settings: &JsonWriterSettings) {
    let mut pos = 0;
    while let Some(&unit) = units.get(pos) {
        let should_escape = match settings.escape_mode {
            JsonEscapeMode::None => false,
            JsonEscapeMode::NonAscii => unit > 0x7f,
            JsonEscapeMode::NonBmp => matches!(unit, 0xd800..=0xdfff),
        } || settings.escape_angle_brackets && matches!(unit, 0x3c | 0x3e)
            || settings.escape_line_separators && matches!(unit, 0x2028 | 0x2029)
            || settings.escape_ampersand && unit == 0x26
            || settings.escape_single_quotes && unit == 0x27;
        if should_escape {
            escape_unit_into(out, unit);
            pos += 1;
            continue;
        }
        match unit {
            0x22 if settings.use_double_quotes => out.push_str("\\\""),
            0x27 if !settings.use_double_quotes => out.push_str("\\'"),
            0x5c => out.push_str("\\\\"),
            0x2f if settings.escape_slash => out.push_str("\\/"),
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
fn escape_unit_into(out: &mut String, unit: u16) {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    out.push_str("\\u");
    for shift in [12, 8, 4, 0] {
        out.push(HEX[usize::from((unit >> shift) & 0x0f)] as char);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::objects::settings_from;
    use bsl_rt::BslNumber;

    fn num(s: &str) -> BslNumber {
        BslNumber::parse_canonical(s).unwrap()
    }

    fn write_compact(f: impl FnOnce(&mut JsonWriter) -> RtResult<()>) -> String {
        let mut w = JsonWriter::to_string_target(JsonWriterSettings {
            line_break: JsonLineBreak::None,
            indent: String::new(),
            ..JsonWriterSettings::default()
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
    fn full_writer_settings_apply_every_measured_escape_flag() {
        let mut settings = JsonWriterSettings {
            line_break: JsonLineBreak::None,
            escape_angle_brackets: true,
            escape_line_separators: true,
            escape_ampersand: true,
            escape_single_quotes: true,
            escape_slash: true,
            ..JsonWriterSettings::default()
        };
        let mut writer = JsonWriter::to_string_target(settings.clone());
        writer.begin_object().unwrap();
        writer.property_name("name").unwrap();
        writer
            .value(&BslValue::Str(bsl_rt::BslString::from_str(
                "<&'/\u{2028}\u{2029}",
            )))
            .unwrap();
        writer.end_object().unwrap();
        assert_eq!(
            writer.finish().unwrap(),
            r#"{"name":"\u003C\u0026\u0027\/\u2028\u2029"}"#
        );

        settings.use_double_quotes = false;
        settings.escape_single_quotes = true;
        let mut writer = JsonWriter::to_string_target(settings);
        writer.begin_object().unwrap();
        writer.property_name("name").unwrap();
        writer
            .value(&BslValue::Str(bsl_rt::BslString::from_str("value")))
            .unwrap();
        writer.end_object().unwrap();
        assert_eq!(writer.finish().unwrap(), "{'name':'value'}");
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
            ..JsonWriterSettings::default()
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

    /// Отказ записи в файл на `Закрыть()` не делает писатель строковым.
    /// Путь-директория: `fs::write` туда не проходит. Прежде `path.take()`
    /// шёл до записи, и повторный `Закрыть()` отдавал документ («42»)
    /// текстом; теперь он снова падает — писатель остаётся файловым.
    #[test]
    fn a_failed_file_write_keeps_the_writer_a_file_writer_for_retry() {
        let dir = std::env::temp_dir();
        let mut w = JsonWriter::to_file(
            dir,
            JsonWriterSettings::default(),
            std::rc::Rc::new(bsl_rt::SystemFileSystem),
        );
        w.value(&BslValue::Number(num("42")))
            .expect("запись значения");
        assert!(
            matches!(w.finish(), Err(RtError::IoError(_))),
            "запись в каталог обязана упасть"
        );
        match w.finish() {
            Err(RtError::IoError(_)) => {}
            other => panic!("повторный Закрыть обязан снова упасть, а не {other:?}"),
        }
    }
}
