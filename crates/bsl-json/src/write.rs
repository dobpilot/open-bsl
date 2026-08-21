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
    pub(crate) fn property_name_bsl(&mut self, name: &bsl_rt::BslString) -> RtResult<()> {
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
}
