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

use crate::string::BslString;
use crate::{BslValue, RtError, RtResult};

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
    /// Документ получен через `ПолучитьОбласть`. `Вывести` принимает
    /// ТОЛЬКО такой: обычный документ в него передать — ошибка (измерено).
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
        let start = lines.iter().position(|l| {
            Self::area_name(l).is_some_and(|n| n.to_uppercase() == wanted)
        });
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

// --- Склейка с объектами BSL --------------------------------------------

use crate::object::BslObject;

fn as_doc(v: &BslValue) -> RtResult<&std::rc::Rc<std::cell::RefCell<TextDocData>>> {
    match v {
        BslValue::Object(o) => match &**o {
            BslObject::TextDocument(d) | BslObject::TextDocParams(d) => Ok(d),
            _ => Err(RtError::MethodNotApplicable {
                method: "метод ТекстовыйДокумент",
                receiver: v.type_name(),
            }),
        },
        _ => Err(RtError::MethodNotApplicable {
            method: "метод ТекстовыйДокумент",
            receiver: v.type_name(),
        }),
    }
}

pub fn is_text_document(v: &BslValue) -> bool {
    matches!(v, BslValue::Object(o) if matches!(&**o, BslObject::TextDocument(_)))
}

fn need_str(arg: Option<&BslValue>, op: &'static str) -> RtResult<String> {
    match arg {
        Some(BslValue::Str(s)) => Ok(s.to_string()),
        _ => Err(RtError::TypeError {
            expected: "Строка",
            op,
        }),
    }
}

fn need_number(arg: Option<&BslValue>, op: &'static str) -> RtResult<i64> {
    match arg {
        Some(BslValue::Number(n)) => n.to_i64_exact().ok_or(RtError::BadIndex),
        _ => Err(RtError::TypeError {
            expected: "Число",
            op,
        }),
    }
}

/// # Errors
///
/// [`RtError::TypeError`], если получатель не документ либо аргумент не строка.
pub fn set_text(obj: &BslValue, args: &[BslValue]) -> RtResult<()> {
    let text = need_str(args.first(), "УстановитьТекст")?;
    as_doc(obj)?.borrow_mut().set_text(&text);
    Ok(())
}

/// # Errors
///
/// [`RtError::MethodNotApplicable`], если получатель не документ.
pub fn get_text(obj: &BslValue) -> RtResult<BslValue> {
    Ok(BslValue::Str(BslString::from_str(
        as_doc(obj)?.borrow().text(),
    )))
}

/// # Errors
///
/// [`RtError::MethodNotApplicable`], если получатель не документ.
pub fn line_count(obj: &BslValue) -> RtResult<BslValue> {
    Ok(BslValue::Number(bsl_number::BslNumber::from_i64(
        as_doc(obj)?.borrow().line_count() as i64,
    )))
}

/// # Errors
///
/// [`RtError::TypeError`], если номер не число.
pub fn get_line(obj: &BslValue, args: &[BslValue]) -> RtResult<BslValue> {
    let n = need_number(args.first(), "ПолучитьСтроку")?;
    Ok(BslValue::Str(BslString::from_str(
        &as_doc(obj)?.borrow().line(n),
    )))
}

/// # Errors
///
/// [`RtError::TypeError`], если аргумент не строка.
pub fn add_line(obj: &BslValue, args: &[BslValue]) -> RtResult<()> {
    let line = need_str(args.first(), "ДобавитьСтроку")?;
    as_doc(obj)?.borrow_mut().add_line(&line);
    Ok(())
}

/// # Errors
///
/// [`RtError::TypeError`] при неверных аргументах.
pub fn insert_line(obj: &BslValue, args: &[BslValue]) -> RtResult<()> {
    let n = need_number(args.first(), "ВставитьСтроку")?;
    let line = need_str(args.get(1), "ВставитьСтроку")?;
    as_doc(obj)?.borrow_mut().insert_line(n, &line);
    Ok(())
}

/// # Errors
///
/// [`RtError::TypeError`] при неверных аргументах.
pub fn replace_line(obj: &BslValue, args: &[BslValue]) -> RtResult<()> {
    let n = need_number(args.first(), "ЗаменитьСтроку")?;
    let line = need_str(args.get(1), "ЗаменитьСтроку")?;
    as_doc(obj)?.borrow_mut().replace_line(n, &line);
    Ok(())
}

/// # Errors
///
/// [`RtError::TypeError`], если номер не число.
pub fn delete_line(obj: &BslValue, args: &[BslValue]) -> RtResult<()> {
    let n = need_number(args.first(), "УдалитьСтроку")?;
    as_doc(obj)?.borrow_mut().delete_line(n);
    Ok(())
}

/// # Errors
///
/// [`RtError::MethodNotApplicable`], если получатель не документ.
pub fn clear(obj: &BslValue) -> RtResult<()> {
    as_doc(obj)?.borrow_mut().clear();
    Ok(())
}

/// Кодировка из второго аргумента: член `КодировкаТекста` либо строка с
/// названием. Без аргумента — UTF-8, как у платформы.
///
/// # Errors
///
/// [`RtError::TextDoc`], если название не поддержано;
/// [`RtError::TypeError`], если аргумент не строка и не член перечисления.
fn encoding_arg(arg: Option<&BslValue>) -> RtResult<crate::encoding::Encoding> {
    use crate::encoding::Encoding;
    match arg {
        None | Some(BslValue::Undefined) => Ok(Encoding::Utf8),
        Some(BslValue::Str(s)) => Encoding::by_name(&s.to_string()),
        Some(BslValue::Enum(e)) => match e {
            // ANSI и «Системная» — кодовая страница системы. На машине
            // замеров это windows-1251 (проверено дампом записанного
            // файла); на системе с другой локалью платформа взяла бы
            // другую, и вот эта зависимость у нас не воспроизведена.
            crate::EnumValue::TextEncodingAnsi | crate::EnumValue::TextEncodingSystem => {
                Ok(Encoding::Windows1251)
            }
            crate::EnumValue::TextEncodingOem => Ok(Encoding::Cp866),
            crate::EnumValue::TextEncodingUtf16 => Ok(Encoding::Utf16Le),
            crate::EnumValue::TextEncodingUtf8 => Ok(Encoding::Utf8),
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

/// `Прочитать(Путь[, Кодировка])`.
///
/// # Errors
///
/// [`RtError::IoError`], если файл не читается.
pub fn read_file(obj: &BslValue, args: &[BslValue]) -> RtResult<()> {
    let path = need_str(args.first(), "Прочитать")?;
    let encoding = encoding_arg(args.get(1))?;
    let bytes = std::fs::read(&path).map_err(|e| RtError::IoError(e.to_string()))?;
    as_doc(obj)?.borrow_mut().set_text(&encoding.decode(&bytes));
    Ok(())
}

/// `Записать(Путь[, Кодировка])`.
///
/// # Errors
///
/// [`RtError::IoError`], если файл не записывается;
/// [`RtError::TextDoc`], если кодировка не поддержана.
pub fn write_file(obj: &BslValue, args: &[BslValue]) -> RtResult<()> {
    let path = need_str(args.first(), "Записать")?;
    let encoding = encoding_arg(args.get(1))?;
    let doc = as_doc(obj)?.borrow();
    std::fs::write(&path, encoding.encode(doc.text()))
        .map_err(|e| RtError::IoError(e.to_string()))?;
    Ok(())
}

/// `ПолучитьОбласть(Имя)`.
///
/// # Errors
///
/// [`RtError::TextDoc`], если области нет.
pub fn get_area(obj: &BslValue, args: &[BslValue]) -> RtResult<BslValue> {
    // Арность проверяется здесь, а не резолвером: у платформы лишний
    // аргумент — ошибка РАНТАЙМА (форма `ПолучитьОбласть(2, 3)` от
    // табличного документа не работает и здесь), и ловится она `Попытка`.
    if args.len() != 1 {
        return Err(bad("ПолучитьОбласть принимает ровно одно имя области"));
    }
    let name = need_str(args.first(), "ПолучитьОбласть")?;
    let area = as_doc(obj)?.borrow().area(&name)?;
    Ok(BslValue::Object(std::rc::Rc::new(BslObject::TextDocument(
        std::rc::Rc::new(std::cell::RefCell::new(area)),
    ))))
}

/// Данные для `Вывести`: сам текст области и её параметры. Форматирование
/// значений делает `bsl-vm` — здесь его негде взять.
///
/// Источник, который НЕ область, ошибкой не считается: платформа на
/// `Вывести` обычного документа отвечает пустым выводом, а не отказом
/// (измерено, `TEXTDOC.OUTPUT_PLAIN_DOC`). Первый замер говорил обратное,
/// но там сама проба была написана с переменной `И` — ключевым словом, — и
/// отказ приходил от разбора, а не от `Вывести`.
///
/// # Errors
///
/// [`RtError::MethodNotApplicable`], если источник вообще не документ.
pub fn area_for_output(source: &BslValue) -> RtResult<(TextDocData, Vec<(String, BslValue)>)> {
    let doc = as_doc(source)?.borrow();
    if !doc.is_area() {
        return Ok((TextDocData::default(), Vec::new()));
    }
    Ok((doc.clone(), doc.params.clone()))
}

/// Дописать готовый текст в конец документа-приёмника.
///
/// # Errors
///
/// [`RtError::MethodNotApplicable`], если приёмник не документ.
pub fn append_rendered(target: &BslValue, rendered: &str) -> RtResult<()> {
    let doc = as_doc(target)?;
    let mut doc = doc.borrow_mut();
    if !doc.text.is_empty() && !doc.text.ends_with('\n') {
        doc.text.push('\n');
    }
    doc.text.push_str(rendered);
    Ok(())
}

/// `Параметры.Имя = Значение` — присваивание идёт сюда.
///
/// # Errors
///
/// [`RtError::TextDoc`], если такого параметра в макете нет.
pub fn set_parameter(obj: &BslValue, name: &str, value: BslValue) -> RtResult<()> {
    as_doc(obj)?.borrow_mut().set_parameter(name, value)
}

/// Чтение `Параметры.Имя`.
///
/// # Errors
///
/// [`RtError::TextDoc`], если такого параметра в макете нет.
pub fn get_parameter(obj: &BslValue, name: &str) -> RtResult<BslValue> {
    let doc = as_doc(obj)?.borrow();
    if !doc
        .parameter_names()
        .iter()
        .any(|n| n.eq_ignore_ascii_case(name))
    {
        return Err(bad(format!("параметра «{name}» в макете нет")));
    }
    Ok(doc.parameter(name).cloned().unwrap_or(BslValue::Undefined))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(text: &str) -> TextDocData {
        let mut d = TextDocData::default();
        d.set_text(text);
        d
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
            d.area("Т").unwrap().render(&[("А".to_string(), "х".to_string())]),
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
