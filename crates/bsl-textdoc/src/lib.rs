//! `ТекстовыйДокумент` и его макеты.
//!
//! # Что здесь ИЗМЕРЕНО на 8.3.27
//!
//! Всё перечисленное снято пробами (`tests/conformance/measure/measure-textdoc.bsl`).
//! Модель строк у платформы своя, и вывести её из здравого смысла нельзя:
//!
//! * пустой текст — НОЛЬ строк, но текст из одного перевода строки — ОДНА;
//!   хвостовой перевод строки лишней строки не создаёт;
//! * `ПолучитьТекст` отдаёт текст КАК БЫЛ, вместе с `ВК` (`"раз\r\nдва"`
//!   возвращается посимвольно тем же), а `ПолучитьСтроку` `ВК` уже не
//!   показывает — то есть текст хранится целиком, а строки из него
//!   выводятся;
//! * `ДобавитьСтроку` дописывает перевод строки ПОСЛЕ себя, поэтому две
//!   добавленные строки дают `"раз\nдва\n"` — и всё равно две строки;
//! * номера строк 1-based, но выход за границы — НЕ ошибка: чтение отдаёт
//!   пустую строку, замена и удаление молча ничего не делают, а вставка за
//!   концом ведёт себя как `ДобавитьСтроку`.
//!
//! # Макеты
//!
//! Области размечаются строками `#Область Имя` и `#КонецОбласти`; регистр
//! не важен ни у директивы, ни у имени, отступ перед директивой допустим.
//!
//! Область НЕ вложенная, и это измерено: у внешней области, внутри которой
//! начинается вторая, текст обрывается ПЕРЕД `#Область` вложенной. Правило
//! одно: область идёт от своей директивы до следующей директивы любого
//! вида, и включает её, только если это `#КонецОбласти`.
//!
//! `ПолучитьОбласть` отдаёт новый документ ВМЕСТЕ с маркерами, а вот
//! `Вывести` их срезает и подставляет параметры. Подстановка идёт в ПОЛЕ
//! фиксированной ширины: `[Имя]` — это пять знаков, и значение либо
//! дополняется пробелами, либо обрезается по ним (`"оченьдлинное"` в
//! `[Имя]` даёт `очень`). Незаданный параметр превращается в пробелы.

use std::cell::RefCell;
use std::rc::Rc;

use bsl_rt::{
    Arity, BslString, BslValue, CallContext, ConstructorCode, ConstructorDescriptor, EnumValue,
    LibraryDependency, LibraryDescriptor, ObjectProtocol, RtError, RtResult, TypeDescriptor,
    TypeId,
};

fn bad(what: impl Into<String>) -> RtError {
    RtError::TextDoc(what.into())
}

/// Директивы разметки макета. Английских написаний у них НЕТ — проверено:
/// `#Region`/`#EndRegion` платформа областью не считает.
const AREA_START: &str = "#ОБЛАСТЬ";
const AREA_END: &str = "#КОНЕЦОБЛАСТИ";

/// Состояние `ТекстовыйДокумент`.
#[derive(Debug, Default, Clone)]
pub struct TextDocData {
    /// Текст ЦЕЛИКОМ, как его положили. Строки выводятся отсюда, а не
    /// хранятся отдельно: иначе `ПолучитьТекст` потерял бы `ВК`, который
    /// платформа сохраняет (измерено).
    text: String,
    /// Документ получен через `ПолучитьОбласть`. Только такой источник
    /// даёт текст в `Вывести`; обычный документ даёт пустой вывод
    /// (измерено, `TEXTDOC.OUTPUT_PLAIN_DOC`).
    is_area: bool,
    /// Значения параметров макета: `(имя, значение)`. Не `HashMap`, потому
    /// что их единицы, а порядок полезен при отладке.
    params: Vec<(String, BslValue)>,
    /// Текст на момент ПЕРВОГО `ПолучитьОбласть`. Разметка областей
    /// замораживается: `УстановитьТекст` после этого проходит и текст
    /// меняет, но области ищутся всё равно по старому снимку, и новая
    /// область не находится (измерено — `TEXTDOC.SET_TEXT_AFTER_GET_AREA`
    /// проходит, `TEXTDOC.GET_AREA_AFTER_SET_TEXT` отказывает, а без
    /// предварительного взятия области та же переустановка работает).
    area_index: std::cell::RefCell<Option<String>>,
}

impl TextDocData {
    pub fn set_text(&mut self, text: &str) {
        self.text = text.to_string();
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn is_area(&self) -> bool {
        self.is_area
    }

    /// Строки документа. `ВК` в конце строки снимается: `ПолучитьСтроку` на
    /// тексте с `ВК`+`ПС` отдаёт строку без `ВК` (измерено).
    ///
    /// Пустой текст даёт НОЛЬ строк, а одинокий перевод строки — одну.
    pub fn lines(&self) -> Vec<&str> {
        Self::split_lines(&self.text)
    }

    fn split_lines(text: &str) -> Vec<&str> {
        if text.is_empty() {
            return Vec::new();
        }
        let mut parts: Vec<&str> = text.split('\n').collect();
        // Хвостовой перевод строки лишней строки не создаёт.
        if parts.last() == Some(&"") {
            parts.pop();
        }
        parts
            .into_iter()
            .map(|l| l.strip_suffix('\r').unwrap_or(l))
            .collect()
    }

    pub fn line_count(&self) -> usize {
        self.lines().len()
    }

    /// Номера 1-based; выход за границы — пустая строка, а не ошибка.
    pub fn line(&self, number: i64) -> String {
        if number < 1 {
            return String::new();
        }
        self.lines()
            .get(number as usize - 1)
            .map_or(String::new(), |s| (*s).to_string())
    }

    /// Дописать строку. Перевод строки ставится и ПЕРЕД (если текст его ещё
    /// не имеет), и ПОСЛЕ — измерено: `"раз"` плюс `ДобавитьСтроку("два")`
    /// даёт `"раз\nдва\n"`.
    pub fn add_line(&mut self, line: &str) {
        if !self.text.is_empty() && !self.text.ends_with('\n') {
            self.text.push('\n');
        }
        self.text.push_str(line);
        self.text.push('\n');
    }

    /// Пересобрать текст из строк, сохранив, был ли в конце перевод строки.
    fn rebuild(&mut self, lines: &[String]) {
        let trailing = self.text.ends_with('\n');
        self.text = lines.join("\n");
        if trailing && !self.text.is_empty() {
            self.text.push('\n');
        }
    }

    /// Вставка ЗА концом ведёт себя как `ДобавитьСтроку` — измерено на
    /// `ВставитьСтроку(9, ...)` в документе из одной строки.
    pub fn insert_line(&mut self, number: i64, line: &str) {
        let mut lines: Vec<String> = self.lines().iter().map(|s| (*s).to_string()).collect();
        let idx = if number < 1 { 0 } else { number as usize - 1 };
        if idx >= lines.len() {
            self.add_line(line);
            return;
        }
        lines.insert(idx, line.to_string());
        self.rebuild(&lines);
    }

    /// Замена за границами — молча ничего (измерено).
    pub fn replace_line(&mut self, number: i64, line: &str) {
        let mut lines: Vec<String> = self.lines().iter().map(|s| (*s).to_string()).collect();
        if number < 1 || number as usize > lines.len() {
            return;
        }
        lines[number as usize - 1] = line.to_string();
        self.rebuild(&lines);
    }

    /// Удаление за границами — тоже молча ничего.
    pub fn delete_line(&mut self, number: i64) {
        let mut lines: Vec<String> = self.lines().iter().map(|s| (*s).to_string()).collect();
        if number < 1 || number as usize > lines.len() {
            return;
        }
        lines.remove(number as usize - 1);
        self.rebuild(&lines);
    }

    pub fn clear(&mut self) {
        self.text.clear();
        self.params.clear();
    }

    /// Имя области в строке-директиве, если строка ею является.
    fn area_name(line: &str) -> Option<&str> {
        let t = line.trim_start();
        let upper = t.to_uppercase();
        let rest = upper.strip_prefix(AREA_START)?;
        // `#ОбластьЧтоТо` без пробела областью не считается.
        if !rest.is_empty() && !rest.starts_with(' ') && !rest.starts_with('\t') {
            return None;
        }
        Some(t[AREA_START.len()..].trim())
    }

    fn is_area_end(line: &str) -> bool {
        line.trim().to_uppercase() == AREA_END
    }

    fn is_directive(line: &str) -> bool {
        Self::area_name(line).is_some() || Self::is_area_end(line)
    }

    /// `ПолучитьОбласть(Имя)` -> новый документ С МАРКЕРАМИ.
    ///
    /// # Errors
    ///
    /// [`RtError::TextDoc`], если области с таким именем нет.
    pub fn area(&self, name: &str) -> RtResult<TextDocData> {
        // Первое обращение фиксирует разметку; дальше ищем по снимку.
        let mut index = self.area_index.borrow_mut();
        let source = index.get_or_insert_with(|| self.text.clone()).clone();
        drop(index);
        let lines = Self::split_lines(&source);
        let wanted = name.trim().to_uppercase();
        let start = lines
            .iter()
            .position(|l| Self::area_name(l).is_some_and(|n| n.to_uppercase() == wanted));
        let Some(start) = start else {
            return Err(bad(format!("область «{name}» в тексте не найдена")));
        };
        // Область идёт до следующей директивы ЛЮБОГО вида и включает её,
        // только если это `#КонецОбласти` (измерено на вложенных областях:
        // у внешней текст обрывается перед `#Область` внутренней).
        let mut end = lines.len();
        for (i, l) in lines.iter().enumerate().skip(start + 1) {
            if Self::is_area_end(l) {
                end = i + 1;
                break;
            }
            if Self::area_name(l).is_some() {
                end = i;
                break;
            }
        }
        let mut text = lines[start..end].join("\n");
        // Перевод строки в конце — только если область не упирается в конец
        // документа (измерено: у последней области его нет).
        if end < lines.len() {
            text.push('\n');
        }
        Ok(TextDocData {
            text,
            is_area: true,
            params: Vec::new(),
            area_index: std::cell::RefCell::new(None),
        })
    }

    /// Имена всех параметров, встречающихся в тексте: `[Имя]`.
    pub fn parameter_names(&self) -> Vec<String> {
        let mut out = Vec::new();
        for (name, _, _) in self.placeholders() {
            if !out.iter().any(|n: &String| n.eq_ignore_ascii_case(&name)) {
                out.push(name);
            }
        }
        out
    }

    /// Плейсхолдеры текста: `(имя, начало, ширина в символах)`.
    fn placeholders(&self) -> Vec<(String, usize, usize)> {
        let chars: Vec<char> = self.text.chars().collect();
        let mut out = Vec::new();
        let mut i = 0;
        while i < chars.len() {
            if chars[i] != '[' {
                i += 1;
                continue;
            }
            let Some(close) = chars[i + 1..].iter().position(|c| *c == ']') else {
                break;
            };
            let end = i + 1 + close;
            let name: String = chars[i + 1..end]
                .iter()
                .collect::<String>()
                .trim()
                .to_string();
            // Перевод строки внутри скобок — это не плейсхолдер.
            if name.is_empty() || name.contains('\n') {
                i += 1;
                continue;
            }
            out.push((name, i, end - i + 1));
            i = end + 1;
        }
        out
    }

    /// # Errors
    ///
    /// [`RtError::TextDoc`], если такого параметра в тексте нет: платформа
    /// на присваивание несуществующему имени отвечает ошибкой (измерено).
    pub fn set_parameter(&mut self, name: &str, value: BslValue) -> RtResult<()> {
        if !self
            .parameter_names()
            .iter()
            .any(|n| n.eq_ignore_ascii_case(name))
        {
            return Err(bad(format!("параметра «{name}» в макете нет")));
        }
        if let Some(slot) = self
            .params
            .iter_mut()
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
        {
            slot.1 = value;
        } else {
            self.params.push((name.to_string(), value));
        }
        Ok(())
    }

    pub fn parameter(&self, name: &str) -> Option<&BslValue> {
        self.params
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
            .map(|(_, v)| v)
    }

    /// Текст области БЕЗ маркеров и с подставленными параметрами.
    ///
    /// Значения приходят уже отформатированными: форматирование живёт в
    /// `bsl-format`, а он зависит от этого крейта, не наоборот, — поэтому
    /// строки готовит вызывающий (`bsl-vm`).
    pub fn render(&self, formatted: &[(String, String)]) -> String {
        let chars: Vec<char> = self.text.chars().collect();
        let mut out = String::new();
        let mut last = 0;
        for (name, start, width) in self.placeholders() {
            out.extend(&chars[last..start]);
            let value = formatted
                .iter()
                .find(|(n, _)| n.eq_ignore_ascii_case(&name))
                .map_or("", |(_, v)| v.as_str());
            // Поле фиксированной ширины: значение дополняется пробелами или
            // обрезается по ширине самого плейсхолдера (измерено).
            let mut written = 0;
            for c in value.chars().take(width) {
                out.push(c);
                written += 1;
            }
            for _ in written..width {
                out.push(' ');
            }
            last = start + width;
        }
        out.extend(&chars[last..]);

        // Маркеры области в вывод не идут. Пустой источник даёт пустой
        // вывод, а не одинокий перевод строки (измерено на `Вывести`
        // обычного документа).
        if out.is_empty() {
            return String::new();
        }
        let mut result = String::new();
        for line in out.split('\n') {
            if Self::is_directive(line) {
                continue;
            }
            if line.is_empty() && result.ends_with('\n') {
                continue;
            }
            // Ведущая угловая скобка заменяется ПРОБЕЛОМ — измерено
            // (`TEXTDOC.ANGLE_LINE_*`). В конфигураторе `<...>` означает
            // строку, которая выводится, только если ей есть что выводить;
            // на документе, собранном в рантайме, от этой конструкции
            // остаётся лишь гашение первого знака — саму условность
            // платформа здесь не применяет, что видно по тому, что строка
            // с пустым параметром всё равно выведена.
            if let Some(rest) = line.strip_prefix('<') {
                result.push(' ');
                result.push_str(rest);
            } else {
                result.push_str(line);
            }
            result.push('\n');
        }
        result
    }
}

// --- Объекты компонента --------------------------------------------------

static DOCUMENT_TYPE: TypeDescriptor = TypeDescriptor {
    package: env!("CARGO_PKG_NAME"),
    name: "ТекстовыйДокумент",
    legacy_type_id: Some(TypeId::TextDocument),
};

static PARAMS_TYPE: TypeDescriptor = TypeDescriptor {
    package: env!("CARGO_PKG_NAME"),
    name: "ПараметрыМакетаТекстовогоДокумента",
    legacy_type_id: Some(TypeId::TextDocParams),
};

#[derive(Debug, Clone, Default)]
struct TextDocument {
    data: Rc<RefCell<TextDocData>>,
}

#[derive(Debug, Clone)]
struct TextDocParams {
    data: Rc<RefCell<TextDocData>>,
}

fn need_str(arg: Option<&BslValue>, op: &'static str) -> RtResult<String> {
    match arg {
        Some(BslValue::Str(value)) => Ok(value.to_string()),
        _ => Err(RtError::TypeError {
            expected: "Строка",
            op,
        }),
    }
}

fn need_number(arg: Option<&BslValue>, op: &'static str) -> RtResult<i64> {
    match arg {
        Some(BslValue::Number(value)) => value.to_i64_exact().ok_or(RtError::BadIndex),
        _ => Err(RtError::TypeError {
            expected: "Число",
            op,
        }),
    }
}

fn wrong_method(name: &str, receiver: &'static str) -> RtError {
    RtError::UnknownMethod {
        method: name.to_string(),
        receiver,
    }
}

fn exact_arity(
    name: &str,
    arguments: &[BslValue],
    expected: usize,
    receiver: &'static str,
) -> RtResult<()> {
    if arguments.len() == expected {
        Ok(())
    } else {
        Err(wrong_method(name, receiver))
    }
}

fn range_arity(
    name: &str,
    arguments: &[BslValue],
    min: usize,
    max: usize,
    receiver: &'static str,
) -> RtResult<()> {
    if (min..=max).contains(&arguments.len()) {
        Ok(())
    } else {
        Err(wrong_method(name, receiver))
    }
}

fn encoding_arg(arg: Option<&BslValue>) -> RtResult<bsl_rt::encoding::Encoding> {
    use bsl_rt::encoding::Encoding;

    match arg {
        None | Some(BslValue::Undefined) => Ok(Encoding::Utf8),
        Some(BslValue::Str(value)) => Encoding::by_name(&value.to_string()),
        Some(BslValue::Enum(value)) => match value {
            EnumValue::TextEncodingAnsi | EnumValue::TextEncodingSystem => {
                Ok(Encoding::Windows1251)
            }
            EnumValue::TextEncodingOem => Ok(Encoding::Cp866),
            EnumValue::TextEncodingUtf16 => Ok(Encoding::Utf16Le),
            EnumValue::TextEncodingUtf8 => Ok(Encoding::Utf8),
            _ => Err(RtError::TypeError {
                expected: "КодировкаТекста",
                op: "кодировка файла",
            }),
        },
        Some(_) => Err(RtError::TypeError {
            expected: "КодировкаТекста либо Строка",
            op: "кодировка файла",
        }),
    }
}

impl TextDocument {
    fn get_area(&self, arguments: &[BslValue]) -> RtResult<BslValue> {
        if arguments.len() != 1 {
            return Err(bad("ПолучитьОбласть принимает ровно одно имя области"));
        }
        let name = need_str(arguments.first(), "ПолучитьОбласть")?;
        let area = self.data.borrow().area(&name)?;
        Ok(BslValue::new_object(TextDocument {
            data: Rc::new(RefCell::new(area)),
        }))
    }

    fn read_file(&self, arguments: &[BslValue]) -> RtResult<BslValue> {
        range_arity("Прочитать", arguments, 1, 2, DOCUMENT_TYPE.name)?;
        let path = need_str(arguments.first(), "Прочитать")?;
        let encoding = encoding_arg(arguments.get(1))?;
        let bytes = std::fs::read(&path).map_err(|error| RtError::IoError(error.to_string()))?;
        self.data.borrow_mut().set_text(&encoding.decode(&bytes));
        Ok(BslValue::Undefined)
    }

    fn write_file(&self, arguments: &[BslValue]) -> RtResult<BslValue> {
        range_arity("Записать", arguments, 1, 2, DOCUMENT_TYPE.name)?;
        let path = need_str(arguments.first(), "Записать")?;
        let encoding = encoding_arg(arguments.get(1))?;
        let data = self.data.borrow();
        std::fs::write(&path, encoding.encode(data.text()))
            .map_err(|error| RtError::IoError(error.to_string()))?;
        Ok(BslValue::Undefined)
    }

    fn output(&self, arguments: &[BslValue], context: &mut CallContext<'_>) -> RtResult<BslValue> {
        exact_arity("Вывести", arguments, 1, DOCUMENT_TYPE.name)?;
        let source = arguments[0]
            .object_ref()
            .and_then(|object| object.downcast_ref::<TextDocument>())
            .ok_or_else(|| RtError::MethodNotApplicable {
                method: "Вывести",
                receiver: arguments[0].type_name(),
            })?;
        let source = source.data.borrow();
        if !source.is_area() {
            return Ok(BslValue::Undefined);
        }
        let mut formatted = Vec::with_capacity(source.params.len());
        for (name, value) in &source.params {
            formatted.push((name.clone(), context.format_value(value, None)?));
        }
        let rendered = source.render(&formatted);
        drop(source);

        let mut target = self.data.borrow_mut();
        if !target.text.is_empty() && !target.text.ends_with('\n') {
            target.text.push('\n');
        }
        target.text.push_str(&rendered);
        Ok(BslValue::Undefined)
    }
}

impl ObjectProtocol for TextDocument {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &DOCUMENT_TYPE
    }

    fn get_property(&self, name: &str, _context: &mut CallContext<'_>) -> RtResult<BslValue> {
        if name.eq_ignore_ascii_case("Параметры") || name.eq_ignore_ascii_case("Parameters")
        {
            Ok(BslValue::new_object(TextDocParams {
                data: self.data.clone(),
            }))
        } else {
            Err(RtError::UnknownColumn(name.to_string()))
        }
    }

    fn call_method(
        &self,
        name: &str,
        arguments: &[BslValue],
        context: &mut CallContext<'_>,
    ) -> RtResult<BslValue> {
        if name.eq_ignore_ascii_case("УстановитьТекст") || name.eq_ignore_ascii_case("SetText")
        {
            exact_arity(name, arguments, 1, DOCUMENT_TYPE.name)?;
            let text = need_str(arguments.first(), "УстановитьТекст")?;
            self.data.borrow_mut().set_text(&text);
            Ok(BslValue::Undefined)
        } else if name.eq_ignore_ascii_case("ПолучитьТекст") || name.eq_ignore_ascii_case("GetText")
        {
            exact_arity(name, arguments, 0, DOCUMENT_TYPE.name)?;
            Ok(BslValue::Str(BslString::from_str(
                self.data.borrow().text(),
            )))
        } else if name.eq_ignore_ascii_case("КоличествоСтрок")
            || name.eq_ignore_ascii_case("LineCount")
        {
            exact_arity(name, arguments, 0, DOCUMENT_TYPE.name)?;
            Ok(BslValue::number_from_i64(
                self.data.borrow().line_count() as i64
            ))
        } else if name.eq_ignore_ascii_case("ПолучитьСтроку")
            || name.eq_ignore_ascii_case("GetLine")
        {
            exact_arity(name, arguments, 1, DOCUMENT_TYPE.name)?;
            let number = need_number(arguments.first(), "ПолучитьСтроку")?;
            Ok(BslValue::Str(BslString::from_str(
                &self.data.borrow().line(number),
            )))
        } else if name.eq_ignore_ascii_case("ДобавитьСтроку")
            || name.eq_ignore_ascii_case("AddLine")
        {
            exact_arity(name, arguments, 1, DOCUMENT_TYPE.name)?;
            let line = need_str(arguments.first(), "ДобавитьСтроку")?;
            self.data.borrow_mut().add_line(&line);
            Ok(BslValue::Undefined)
        } else if name.eq_ignore_ascii_case("ВставитьСтроку")
            || name.eq_ignore_ascii_case("InsertLine")
        {
            exact_arity(name, arguments, 2, DOCUMENT_TYPE.name)?;
            let number = need_number(arguments.first(), "ВставитьСтроку")?;
            let line = need_str(arguments.get(1), "ВставитьСтроку")?;
            self.data.borrow_mut().insert_line(number, &line);
            Ok(BslValue::Undefined)
        } else if name.eq_ignore_ascii_case("ЗаменитьСтроку")
            || name.eq_ignore_ascii_case("ReplaceLine")
        {
            exact_arity(name, arguments, 2, DOCUMENT_TYPE.name)?;
            let number = need_number(arguments.first(), "ЗаменитьСтроку")?;
            let line = need_str(arguments.get(1), "ЗаменитьСтроку")?;
            self.data.borrow_mut().replace_line(number, &line);
            Ok(BslValue::Undefined)
        } else if name.eq_ignore_ascii_case("УдалитьСтроку")
            || name.eq_ignore_ascii_case("DeleteLine")
        {
            exact_arity(name, arguments, 1, DOCUMENT_TYPE.name)?;
            let number = need_number(arguments.first(), "УдалитьСтроку")?;
            self.data.borrow_mut().delete_line(number);
            Ok(BslValue::Undefined)
        } else if name.eq_ignore_ascii_case("Очистить") || name.eq_ignore_ascii_case("Clear")
        {
            exact_arity(name, arguments, 0, DOCUMENT_TYPE.name)?;
            self.data.borrow_mut().clear();
            Ok(BslValue::Undefined)
        } else if name.eq_ignore_ascii_case("Прочитать") || name.eq_ignore_ascii_case("Read")
        {
            self.read_file(arguments)
        } else if name.eq_ignore_ascii_case("Записать") || name.eq_ignore_ascii_case("Write")
        {
            self.write_file(arguments)
        } else if name.eq_ignore_ascii_case("ПолучитьОбласть")
            || name.eq_ignore_ascii_case("GetArea")
        {
            self.get_area(arguments)
        } else if name.eq_ignore_ascii_case("Вывести") || name.eq_ignore_ascii_case("Output")
        {
            self.output(arguments, context)
        } else {
            Err(wrong_method(name, DOCUMENT_TYPE.name))
        }
    }

    fn is_filled(&self) -> RtResult<bool> {
        Ok(true)
    }
}

impl ObjectProtocol for TextDocParams {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &PARAMS_TYPE
    }

    fn get_property(&self, name: &str, _context: &mut CallContext<'_>) -> RtResult<BslValue> {
        let data = self.data.borrow();
        if !data
            .parameter_names()
            .iter()
            .any(|candidate| candidate.eq_ignore_ascii_case(name))
        {
            return Err(bad(format!("параметра «{name}» в макете нет")));
        }
        Ok(data.parameter(name).cloned().unwrap_or(BslValue::Undefined))
    }

    fn set_property(
        &self,
        name: &str,
        value: BslValue,
        _context: &mut CallContext<'_>,
    ) -> RtResult<()> {
        self.data.borrow_mut().set_parameter(name, value)
    }

    fn is_filled(&self) -> RtResult<bool> {
        Ok(true)
    }
}

/// Создаёт пустой ТекстовыйДокумент.
pub fn new_text_document() -> BslValue {
    BslValue::new_object(TextDocument::default())
}

fn construct(_context: &mut CallContext<'_>, arguments: &[BslValue]) -> RtResult<BslValue> {
    if arguments.is_empty() {
        Ok(new_text_document())
    } else {
        Err(wrong_method("Новый ТекстовыйДокумент", DOCUMENT_TYPE.name))
    }
}

const CONSTRUCTORS: &[ConstructorDescriptor] = &[ConstructorDescriptor {
    code: ConstructorCode::new(1),
    names: &["ТекстовыйДокумент", "TextDocument"],
    arity: Arity::exact(0),
    call: construct,
}];

/// Дескриптор статически подключаемого компонента текстовых документов.
pub const fn library() -> LibraryDescriptor {
    LibraryDescriptor {
        package: env!("CARGO_PKG_NAME"),
        version: env!("CARGO_PKG_VERSION"),
        dependencies: &[LibraryDependency {
            package: bsl_rt::PACKAGE_NAME,
            version: bsl_rt::PACKAGE_VERSION,
        }],
        functions: &[],
        constructors: CONSTRUCTORS,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(text: &str) -> TextDocData {
        let mut d = TextDocData::default();
        d.set_text(text);
        d
    }

    #[test]
    fn constructor_code_is_static() {
        assert_eq!(library().constructors.len(), 1);
        assert_eq!(library().constructors[0].code.get(), 1);
    }

    /// Модель строк платформы: пустой текст — ноль строк, одинокий перевод
    /// строки — одна, хвостовой перевод лишней не создаёт. Замеры
    /// `TEXTDOC.COUNT_*`.
    #[test]
    fn line_count_follows_the_measured_model() {
        assert_eq!(doc("").line_count(), 0);
        assert_eq!(doc("\n").line_count(), 1);
        assert_eq!(doc("раз").line_count(), 1);
        assert_eq!(doc("раз\n").line_count(), 1);
        assert_eq!(doc("раз\nдва").line_count(), 2);
        assert_eq!(doc("раз\r\nдва").line_count(), 2);
    }

    /// Текст сохраняется КАК БЫЛ, а `ВК` в строке не показывается. Замеры
    /// `TEXTDOC.CRLF_ROUND_TRIP` и `TEXTDOC.GET_LINE_CRLF`.
    #[test]
    fn text_keeps_carriage_returns_but_lines_do_not() {
        let d = doc("раз\r\nдва");
        assert_eq!(d.text(), "раз\r\nдва");
        assert_eq!(d.line(1), "раз");
        assert_eq!(d.line(2), "два");
    }

    /// Выход за границы — не ошибка. Замеры `TEXTDOC.GET_LINE_ZERO`,
    /// `..._BEYOND`, `TEXTDOC.REPLACE_BEYOND`, `TEXTDOC.DELETE_BEYOND`.
    #[test]
    fn out_of_range_line_numbers_are_silent() {
        let mut d = doc("раз");
        assert_eq!(d.line(0), "");
        assert_eq!(d.line(5), "");
        d.replace_line(9, "девять");
        assert_eq!(d.text(), "раз");
        d.delete_line(9);
        assert_eq!(d.text(), "раз");
        // А вот вставка за концом ведёт себя как ДобавитьСтроку.
        d.insert_line(9, "девять");
        assert_eq!(d.text(), "раз\nдевять\n");
    }

    /// `ДобавитьСтроку` ставит перевод строки и до, и после. Замеры
    /// `TEXTDOC.ADD_LINE`, `TEXTDOC.ADD_AFTER_SET`.
    #[test]
    fn add_line_terminates_every_line() {
        let mut d = TextDocData::default();
        d.add_line("раз");
        d.add_line("два");
        assert_eq!(d.text(), "раз\nдва\n");
        assert_eq!(d.line_count(), 2);

        let mut d = doc("раз");
        d.add_line("два");
        assert_eq!(d.text(), "раз\nдва\n");
        assert_eq!(d.line_count(), 2);
    }

    /// Область отдаётся С маркерами и обрывается на следующей директиве.
    /// Замеры `TEXTDOC.AREA_SUBSET`, `AREA_TWO`, `AREA_NESTED*`.
    #[test]
    fn area_runs_until_the_next_directive() {
        let d = doc("до\n#Область Тело\nвнутри\n#КонецОбласти\nпосле");
        assert_eq!(
            d.area("Тело").unwrap().text(),
            "#Область Тело\nвнутри\n#КонецОбласти\n"
        );

        // Последняя область перевода строки в конце не получает.
        let d = doc("#Область А\nпервая\n#КонецОбласти\n#Область Б\nвторая\n#КонецОбласти");
        assert_eq!(
            d.area("Б").unwrap().text(),
            "#Область Б\nвторая\n#КонецОбласти"
        );

        // Вложения НЕТ: внешняя обрывается перед внутренней.
        let d = doc("#Область Внешняя\nа\n#Область Внутренняя\nб\n#КонецОбласти\n#КонецОбласти");
        assert_eq!(d.area("Внешняя").unwrap().text(), "#Область Внешняя\nа\n");
        assert_eq!(
            d.area("Внутренняя").unwrap().text(),
            "#Область Внутренняя\nб\n#КонецОбласти\n"
        );
    }

    /// Регистр и отступ директиве не мешают, а английских написаний нет.
    /// Замеры `TEXTDOC.AREA_DIRECTIVE_CASE`, `AREA_INDENTED`, `GET_AREA_EN`.
    #[test]
    fn directives_are_case_insensitive_and_may_be_indented() {
        assert!(doc("#область Т\nтело\n#конецобласти").area("т").is_ok());
        assert!(doc("  #Область Т\nтело\n  #КонецОбласти").area("Т").is_ok());
        assert!(doc("#Region Head\nhi\n#EndRegion").area("Head").is_err());
        assert!(doc("просто текст").area("НетТакой").is_err());
    }

    /// Подстановка идёт в поле ФИКСИРОВАННОЙ ширины, маркеры срезаются.
    /// Замеры `TEXTDOC.PARAM_ON_OUTPUT`, `PARAM_MISSING`, `PARAM_LONGER`,
    /// `PARAM_REPEATED`, `OUTPUT_KEEPS_LINES`.
    #[test]
    fn output_pads_or_truncates_to_the_placeholder_width() {
        let d = doc("#Область Т\nпривет, [Имя]\n#КонецОбласти");
        let area = d.area("Т").unwrap();
        assert_eq!(
            area.render(&[("Имя".to_string(), "мир".to_string())]),
            "привет, мир  \n"
        );
        // Незаданный параметр — пробелы по ширине плейсхолдера.
        assert_eq!(area.render(&[]), "привет,      \n");

        // Значение длиннее поля обрезается.
        let d = doc("#Область Т\n[Имя]|\n#КонецОбласти");
        assert_eq!(
            d.area("Т")
                .unwrap()
                .render(&[("Имя".to_string(), "оченьдлинноезначение".to_string())]),
            "очень|\n"
        );

        // Один и тот же параметр подставляется в каждое вхождение.
        let d = doc("#Область Т\n[А]-[А]\n#КонецОбласти");
        assert_eq!(
            d.area("Т")
                .unwrap()
                .render(&[("А".to_string(), "х".to_string())]),
            "х  -х  \n"
        );

        // Строк в выводе столько же, сколько в теле области.
        let d = doc("#Область Т\nпервая\nвторая\n#КонецОбласти");
        assert_eq!(d.area("Т").unwrap().render(&[]), "первая\nвторая\n");
    }

    /// Имена параметров берутся из текста, чужое имя не принимается.
    /// Замер `TEXTDOC.PARAM_UNKNOWN_NAME`.
    #[test]
    fn only_parameters_present_in_the_text_can_be_set() {
        let mut d = doc("привет, [Имя]");
        assert_eq!(d.parameter_names(), vec!["Имя".to_string()]);
        assert!(d.set_parameter("Имя", BslValue::Undefined).is_ok());
        assert!(d.set_parameter("НетТакого", BslValue::Undefined).is_err());
    }

    /// Пробелы внутри поля задают ширину, а не входят в имя параметра.
    #[test]
    fn parameter_names_ignore_padding_spaces() {
        let mut d = doc("#Область Т\n[Сумма       ]|\n#КонецОбласти")
            .area("Т")
            .unwrap();
        assert_eq!(d.parameter_names(), vec!["Сумма".to_string()]);
        d.set_parameter("Сумма", BslValue::Undefined).unwrap();
        assert_eq!(
            d.render(&[("Сумма".to_string(), "123".to_string())]),
            "123           |\n"
        );
    }
}
