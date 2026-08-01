//! XML: потоковое чтение и запись.
//!
//! Устроено как `json.rs`: разборщик отдаёт события по одному, `ЧтениеXML`
//! только показывает наружу текущее, а `ЗаписьXML` копит текст и отдаёт его
//! на `Закрыть()`. Второй реализации разбора быть не должно.
//!
//! # Что здесь ИЗМЕРЕНО на 8.3.27
//!
//! Всё перечисленное снято пробами (`tests/conformance/measure/measure-xml.bsl`),
//! а не выведено из спецификации XML — платформа заметно от неё отходит:
//!
//! * узлов ОБЪЯВЛЕНИЯ и КОММЕНТАРИЯ читатель НЕ отдаёт вовсе, хотя члены
//!   `ТипУзлаXML` для них есть; инструкция обработки, наоборот, отдаётся;
//! * секция CDATA не отдельный узел, а часть СОСЕДНЕГО текста:
//!   `<а>раз<![CDATA[два]]>три</а>` — ОДИН узел «раздватри». Комментарий же
//!   текст РАЗРЫВАЕТ: `<а>раз<!--к-->два</а>` — два узла;
//! * текстовый узел целиком из пробелов выбрасывается, но пробел ВОКРУГ
//!   значащего текста сохраняется (`<а> т </а>` — узел « т »);
//! * у текстового узла `Имя` — `#text`;
//! * `<а/>` и `<а></а>` неразличимы: оба дают начало и конец элемента;
//! * битый ввод — ошибка, и пустая строка тоже: документа без корневого
//!   элемента не бывает;
//! * `Пропустить` оставляет читатель НА закрывающем теге; на нетекстовом
//!   узле пропускается остаток РОДИТЕЛЯ;
//! * запись по умолчанию с отступом в ОДИН ТАБ на уровень, но текст
//!   переводов строки вокруг себя не получает, а закрывающий тег встаёт с
//!   новой строки, только если последним в элементе шёл не текст;
//! * `ЗаписатьТекст("")` не делает ничего — элемент остаётся пустым (`<а/>`);
//! * экранируются при записи `&`, `<`, `>`, а в значении атрибута ещё и
//!   `"`; апостроф не экранируется НИГДЕ, табуляция и перевод строки уходят
//!   в атрибут как есть;
//! * `ЗаписатьТекст`/`ЗаписатьАтрибут` принимают ТОЛЬКО строку: число —
//!   ошибка.

use std::path::PathBuf;

use crate::string::BslString;
use crate::{BslValue, RtError, RtResult};

/// Ошибка разбора или записи. Текст платформы не воспроизводим — он
/// привязан к номерам строк её модуля, — поэтому своё сообщение.
fn bad(what: impl Into<String>) -> RtError {
    RtError::Xml(what.into())
}

/// Имя текстового узла. Не наша выдумка: `Ч.Имя` на тексте отдаёт именно
/// это (измерено).
pub const TEXT_NODE_NAME: &str = "#text";

/// Атрибут начального тега.
#[derive(Debug, Clone, PartialEq)]
pub struct XmlAttr {
    /// Имя как в тексте, вместе с префиксом.
    pub name: String,
    pub value: String,
}

/// Событие разбора. Пространство имён кладётся В САМО событие, а не
/// резолвится потом по стеку: к моменту, когда до свойства доберётся
/// пользователь, элемент со своими объявлениями может быть уже закрыт.
#[derive(Debug, Clone, PartialEq)]
pub enum XmlEvent {
    ElementStart {
        name: String,
        uri: String,
        attrs: Vec<XmlAttr>,
    },
    ElementEnd {
        name: String,
        uri: String,
    },
    Text(String),
    ProcessingInstruction {
        target: String,
        data: String,
    },
}

// --- Разбор -------------------------------------------------------------

/// Открытый элемент и объявленные ИМЕННО НА НЁМ префиксы.
#[derive(Debug)]
struct OpenElement {
    name: String,
    uri: String,
    /// `(префикс, URI)`; пустой префикс — объявление по умолчанию.
    ns: Vec<(String, String)>,
}

#[derive(Debug)]
pub struct XmlParser {
    src: Vec<char>,
    pos: usize,
    open: Vec<OpenElement>,
    /// Корневой элемент уже закрыт — второго документ не допускает.
    root_done: bool,
    /// `<а/>`: начало отдано, конец ждёт следующего вызова.
    pending_end: Option<(String, String)>,
}

impl XmlParser {
    pub fn new(text: &str) -> Self {
        XmlParser {
            src: text.chars().collect(),
            pos: 0,
            open: Vec::new(),
            root_done: false,
            pending_end: None,
        }
    }

    fn peek(&self) -> Option<char> {
        self.src.get(self.pos).copied()
    }

    fn starts_with(&self, s: &str) -> bool {
        s.chars()
            .enumerate()
            .all(|(i, c)| self.src.get(self.pos + i) == Some(&c))
    }

    fn skip_ws(&mut self) {
        while matches!(self.peek(), Some(c) if c.is_whitespace()) {
            self.pos += 1;
        }
    }

    /// Проглотить всё до `marker` включительно. Отсутствие маркера —
    /// незакрытая конструкция, то есть битый документ.
    fn skip_until(&mut self, marker: &str) -> RtResult<String> {
        let start = self.pos;
        while self.pos < self.src.len() {
            if self.starts_with(marker) {
                let inner: String = self.src[start..self.pos].iter().collect();
                self.pos += marker.chars().count();
                return Ok(inner);
            }
            self.pos += 1;
        }
        Err(bad(format!("не найдено закрывающее «{marker}»")))
    }

    /// Имя элемента или атрибута. XML разрешает в именах куда больше, чем
    /// ASCII, поэтому имя — это всё до пробела и до разделителя разметки.
    fn read_name(&mut self) -> RtResult<String> {
        let start = self.pos;
        while let Some(c) = self.peek() {
            if c.is_whitespace() || matches!(c, '=' | '/' | '>' | '<' | '?') {
                break;
            }
            self.pos += 1;
        }
        if self.pos == start {
            return Err(bad("ожидалось имя"));
        }
        Ok(self.src[start..self.pos].iter().collect())
    }

    /// Разбор ссылки на сущность после уже проглоченного `&`.
    fn read_entity(&mut self) -> RtResult<char> {
        let start = self.pos;
        while matches!(self.peek(), Some(c) if c != ';' && !c.is_whitespace() && c != '<') {
            self.pos += 1;
        }
        if self.peek() != Some(';') {
            return Err(bad("ссылка на сущность без «;»"));
        }
        let name: String = self.src[start..self.pos].iter().collect();
        self.pos += 1;
        if let Some(rest) = name.strip_prefix('#') {
            let code = if let Some(hex) = rest.strip_prefix('x').or_else(|| rest.strip_prefix('X'))
            {
                u32::from_str_radix(hex, 16).ok()
            } else {
                rest.parse::<u32>().ok()
            };
            return code
                .and_then(char::from_u32)
                .ok_or_else(|| bad(format!("недопустимая ссылка на символ «&{name};»")));
        }
        match name.as_str() {
            "amp" => Ok('&'),
            "lt" => Ok('<'),
            "gt" => Ok('>'),
            "quot" => Ok('"'),
            "apos" => Ok('\''),
            // Сущности, которой в XML нет, платформа не прощает:
            // `<а>&nbsp;</а>` — ошибка, а не текст как есть (измерено).
            _ => Err(bad(format!("неизвестная сущность «&{name};»"))),
        }
    }

    /// Текстовый прогон до следующей разметки. `None` — прогон выброшен как
    /// целиком пробельный (измерено: такой узел платформа не отдаёт).
    ///
    /// Секция CDATA не прерывает прогон, а вливается в него: измерено, что
    /// `раз<![CDATA[два]]>три` — ОДИН узел.
    fn read_text_run(&mut self) -> RtResult<Option<String>> {
        let mut out = String::new();
        loop {
            while let Some(c) = self.peek() {
                if c == '<' {
                    break;
                }
                if c == '&' {
                    self.pos += 1;
                    out.push(self.read_entity()?);
                } else {
                    out.push(c);
                    self.pos += 1;
                }
            }
            if self.starts_with("<![CDATA[") {
                self.pos += "<![CDATA[".chars().count();
                out.push_str(&self.skip_until("]]>")?);
                continue;
            }
            break;
        }
        if self.open.is_empty() {
            // Вне корневого элемента текста быть не может; пробелы —
            // могут (перевод строки в конце файла — обычное дело).
            if out.trim().is_empty() {
                return Ok(None);
            }
            return Err(bad("текст вне корневого элемента"));
        }
        // Пробельный прогон выбрасывается независимо от того, пришёл он
        // из обычного текста или из секции CDATA: измерено, что
        // `<а><![CDATA[ ]]></а>` узла не даёт — явная выписка секции
        // значащей его НЕ делает.
        if out.trim().is_empty() {
            return Ok(None);
        }
        Ok(Some(out))
    }

    /// Объявления `xmlns` начального тега — в область видимости, остальное
    /// — в атрибуты.
    fn split_namespaces(attrs: &[XmlAttr]) -> Vec<(String, String)> {
        let mut ns = Vec::new();
        for a in attrs {
            if let Some(prefix) = a.name.strip_prefix("xmlns:") {
                ns.push((prefix.to_string(), a.value.clone()));
            } else if a.name == "xmlns" {
                ns.push((String::new(), a.value.clone()));
            }
        }
        ns
    }

    /// URI по префиксу — поиск от вершины стека вниз, как того требует
    /// область видимости XML.
    fn resolve_prefix(&self, prefix: &str) -> String {
        for el in self.open.iter().rev() {
            for (p, uri) in &el.ns {
                if p == prefix {
                    return uri.clone();
                }
            }
        }
        String::new()
    }

    fn read_attributes(&mut self) -> RtResult<Vec<XmlAttr>> {
        let mut attrs = Vec::new();
        loop {
            self.skip_ws();
            match self.peek() {
                Some('>') | Some('/') | None => return Ok(attrs),
                _ => {}
            }
            let name = self.read_name()?;
            self.skip_ws();
            if self.peek() != Some('=') {
                return Err(bad(format!("у атрибута «{name}» нет значения")));
            }
            self.pos += 1;
            self.skip_ws();
            let quote = match self.peek() {
                Some(q @ ('"' | '\'')) => q,
                _ => return Err(bad(format!("значение атрибута «{name}» не в кавычках"))),
            };
            self.pos += 1;
            let mut value = String::new();
            loop {
                match self.peek() {
                    None => return Err(bad(format!("значение атрибута «{name}» не закрыто"))),
                    Some(c) if c == quote => {
                        self.pos += 1;
                        break;
                    }
                    Some('&') => {
                        self.pos += 1;
                        value.push(self.read_entity()?);
                    }
                    Some(c) => {
                        value.push(c);
                        self.pos += 1;
                    }
                }
            }
            attrs.push(XmlAttr { name, value });
        }
    }

    /// Следующее событие; `None` — документ кончился.
    ///
    /// # Errors
    ///
    /// [`RtError::Xml`] на битой разметке: незакрытый элемент, чужой
    /// закрывающий тег, второй корень, текст вне корня.
    pub fn read(&mut self) -> RtResult<Option<XmlEvent>> {
        if let Some((name, uri)) = self.pending_end.take() {
            self.open.pop();
            if self.open.is_empty() {
                self.root_done = true;
            }
            return Ok(Some(XmlEvent::ElementEnd { name, uri }));
        }
        loop {
            if self.pos >= self.src.len() {
                if let Some(open) = self.open.last() {
                    return Err(bad(format!("элемент «{}» не закрыт", open.name)));
                }
                if !self.root_done {
                    return Err(bad("в документе нет корневого элемента"));
                }
                return Ok(None);
            }
            // Секция CDATA — не самостоятельный узел, а НАЧАЛО текстового
            // прогона: проверка обязана стоять до общей ветки `<!`, иначе
            // секция уедет в разбор объявления типа документа.
            if self.peek() != Some('<') || self.starts_with("<![CDATA[") {
                if let Some(text) = self.read_text_run()? {
                    return Ok(Some(XmlEvent::Text(text)));
                }
                continue;
            }
            // Объявление XML читатель не отдаёт — измерено. Спецификация
            // держит имя `xml` за платформой, так что это не инструкция
            // обработки.
            if self.starts_with("<?xml") {
                self.pos += 2;
                self.skip_until("?>")?;
                continue;
            }
            if self.starts_with("<?") {
                self.pos += 2;
                let target = self.read_name()?;
                self.skip_ws();
                let data = self.skip_until("?>")?;
                return Ok(Some(XmlEvent::ProcessingInstruction {
                    target,
                    data: data.trim_end().to_string(),
                }));
            }
            // Комментарий не отдаётся, но текст вокруг себя РАЗРЫВАЕТ:
            // выход из `read_text_run` уже произошёл, а новый прогон
            // начнётся после `-->` (измерено).
            if self.starts_with("<!--") {
                self.pos += 4;
                self.skip_until("-->")?;
                continue;
            }
            // Объявление типа документа узлом не отдаётся — измерено на
            // `<!DOCTYPE а><а/>`, где видны только начало и конец `а`.
            if self.starts_with("<!") {
                self.pos += 2;
                self.skip_until(">")?;
                continue;
            }
            if self.starts_with("</") {
                self.pos += 2;
                let name = self.read_name()?;
                self.skip_ws();
                if self.peek() != Some('>') {
                    return Err(bad(format!("закрывающий тег «{name}» не закрыт")));
                }
                self.pos += 1;
                let open = self
                    .open
                    .pop()
                    .ok_or_else(|| bad(format!("закрывающий тег «{name}» без открывающего")))?;
                if open.name != name {
                    return Err(bad(format!(
                        "закрывающий тег «{name}» не совпадает с открытым «{}»",
                        open.name
                    )));
                }
                if self.open.is_empty() {
                    self.root_done = true;
                }
                return Ok(Some(XmlEvent::ElementEnd {
                    name,
                    uri: open.uri,
                }));
            }
            // Начальный тег.
            if self.root_done {
                return Err(bad("в документе больше одного корневого элемента"));
            }
            self.pos += 1;
            let name = self.read_name()?;
            let attrs = self.read_attributes()?;
            let empty = match self.peek() {
                Some('/') => {
                    self.pos += 1;
                    if self.peek() != Some('>') {
                        return Err(bad(format!("тег «{name}» не закрыт")));
                    }
                    self.pos += 1;
                    true
                }
                Some('>') => {
                    self.pos += 1;
                    false
                }
                _ => return Err(bad(format!("тег «{name}» не закрыт"))),
            };
            let ns = Self::split_namespaces(&attrs);
            self.open.push(OpenElement {
                name: name.clone(),
                uri: String::new(),
                ns,
            });
            // Префикс резолвится ПОСЛЕ помещения в стек: элемент вправе
            // пользоваться префиксом, который сам же и объявил.
            let prefix = prefix_of(&name);
            let uri = self.resolve_prefix(prefix);
            if let Some(top) = self.open.last_mut() {
                top.uri = uri.clone();
            }
            if empty {
                self.pending_end = Some((name.clone(), uri.clone()));
            }
            return Ok(Some(XmlEvent::ElementStart { name, uri, attrs }));
        }
    }

    /// Глубина открытых элементов — по ней `Пропустить` понимает, где
    /// остановиться.
    pub fn depth(&self) -> usize {
        self.open.len()
    }
}

/// Часть имени до двоеточия; без двоеточия префикса нет.
pub fn prefix_of(name: &str) -> &str {
    match name.split_once(':') {
        Some((p, _)) => p,
        None => "",
    }
}

/// Часть имени после двоеточия.
pub fn local_of(name: &str) -> &str {
    match name.split_once(':') {
        Some((_, l)) => l,
        None => name,
    }
}

// --- Запись -------------------------------------------------------------

/// `ПараметрыЗаписиXML(Кодировка, Версия, ИспользоватьОтступ)`.
///
/// Третий параметр гасит И перевод строки, И отступ разом — измерено:
/// `<а><б/></а>` в одну строку.
#[derive(Debug, Clone)]
pub struct XmlWriterSettings {
    /// `None` — не писать `encoding` в объявлении. Именно так ведёт себя
    /// `УстановитьСтроку()` без параметров, тогда как `ОткрытьФайл`
    /// подставляет UTF-8 (измерено).
    pub encoding: Option<String>,
    pub version: String,
    pub indent: bool,
}

impl Default for XmlWriterSettings {
    fn default() -> Self {
        XmlWriterSettings {
            encoding: None,
            version: "1.0".to_string(),
            indent: true,
        }
    }
}

/// Открытый элемент писателя.
#[derive(Debug)]
struct OpenTag {
    name: String,
    /// Последним в этом элементе шёл текст: тогда закрывающий тег встаёт
    /// вплотную, без перевода строки (измерено).
    last_was_text: bool,
}

#[derive(Debug)]
pub struct XmlWriter {
    out: String,
    settings: XmlWriterSettings,
    stack: Vec<OpenTag>,
    /// Начальный тег написан, но не закрыт `>`: можно ещё дописать атрибут
    /// или схлопнуть элемент в `<а/>`.
    pending_start: bool,
    /// Корневой элемент закрыт — второго документ не допускает.
    root_done: bool,
    path: Option<PathBuf>,
}

impl XmlWriter {
    pub fn to_string_target(settings: XmlWriterSettings) -> Self {
        XmlWriter {
            out: String::new(),
            settings,
            stack: Vec::new(),
            pending_start: false,
            root_done: false,
            path: None,
        }
    }

    pub fn to_file(path: PathBuf, settings: XmlWriterSettings) -> Self {
        let mut w = Self::to_string_target(settings);
        w.path = Some(path);
        w
    }

    pub fn is_file_target(&self) -> bool {
        self.path.is_some()
    }

    /// Перевод строки и отступ по глубине. Один таб на уровень — измерено.
    fn newline(&mut self, depth: usize) {
        if !self.settings.indent {
            return;
        }
        self.out.push('\n');
        for _ in 0..depth {
            self.out.push('\t');
        }
    }

    /// Дописать `>` у висящего начального тега: содержимое элемента
    /// начинается.
    fn close_pending(&mut self) {
        if self.pending_start {
            self.out.push('>');
            self.pending_start = false;
        }
    }

    fn mark_content(&mut self, text_like: bool) {
        if let Some(top) = self.stack.last_mut() {
            top.last_was_text = text_like;
        }
    }

    /// # Errors
    ///
    /// [`RtError::Xml`], если объявление пишется не первым.
    pub fn write_declaration(&mut self) -> RtResult<()> {
        if !self.out.is_empty() || self.pending_start {
            return Err(bad("объявление XML должно идти первым"));
        }
        self.out.push_str("<?xml version=\"");
        self.out.push_str(&self.settings.version);
        self.out.push('"');
        if let Some(enc) = self.settings.encoding.clone() {
            self.out.push_str(" encoding=\"");
            self.out.push_str(&enc);
            self.out.push('"');
        }
        self.out.push_str("?>");
        // Перевод строки после объявления — часть форматирования, поэтому
        // подчиняется тому же флагу (измерено при включённом отступе).
        if self.settings.indent {
            self.out.push('\n');
        }
        Ok(())
    }

    /// # Errors
    ///
    /// [`RtError::Xml`], если корневой элемент уже закрыт.
    pub fn write_start_element(&mut self, name: &str) -> RtResult<()> {
        if self.root_done {
            return Err(bad("корневой элемент уже записан"));
        }
        self.close_pending();
        if !self.stack.is_empty() {
            let depth = self.stack.len();
            self.newline(depth);
        }
        self.mark_content(false);
        self.out.push('<');
        self.out.push_str(name);
        self.pending_start = true;
        self.stack.push(OpenTag {
            name: name.to_string(),
            last_was_text: false,
        });
        Ok(())
    }

    /// # Errors
    ///
    /// [`RtError::Xml`], если начальный тег уже закрыт: после текста или
    /// вложенного элемента атрибут дописать нельзя (измерено — ошибка).
    pub fn write_attribute(&mut self, name: &str, value: &str) -> RtResult<()> {
        if !self.pending_start {
            return Err(bad("атрибут вне начального тега"));
        }
        self.out.push(' ');
        self.out.push_str(name);
        self.out.push_str("=\"");
        escape_attr(&mut self.out, value);
        self.out.push('"');
        Ok(())
    }

    /// Пустая строка не делает НИЧЕГО: элемент остаётся пустым и
    /// схлопывается в `<а/>` (измерено).
    ///
    /// # Errors
    ///
    /// [`RtError::Xml`], если открытого элемента нет.
    pub fn write_text(&mut self, text: &str) -> RtResult<()> {
        if text.is_empty() {
            return Ok(());
        }
        if self.stack.is_empty() {
            return Err(bad("текст вне элемента"));
        }
        self.close_pending();
        escape_text(&mut self.out, text);
        self.mark_content(true);
        Ok(())
    }

    /// # Errors
    ///
    /// [`RtError::Xml`], если открытого элемента нет.
    pub fn write_end_element(&mut self) -> RtResult<()> {
        let Some(top) = self.stack.pop() else {
            return Err(bad("ЗаписатьКонецЭлемента без открытого элемента"));
        };
        if self.pending_start {
            self.out.push_str("/>");
            self.pending_start = false;
        } else {
            if !top.last_was_text {
                let depth = self.stack.len();
                self.newline(depth);
            }
            self.out.push_str("</");
            self.out.push_str(&top.name);
            self.out.push('>');
        }
        if self.stack.is_empty() {
            self.root_done = true;
        }
        self.mark_content(false);
        Ok(())
    }

    /// # Errors
    ///
    /// Не отказывает; `Result` — ради единообразия с остальными методами.
    pub fn write_comment(&mut self, text: &str) -> RtResult<()> {
        self.close_pending();
        let depth = self.stack.len();
        self.newline(depth);
        self.out.push_str("<!--");
        self.out.push_str(text);
        self.out.push_str("-->");
        self.mark_content(false);
        Ok(())
    }

    /// # Errors
    ///
    /// Не отказывает; `Result` — ради единообразия.
    pub fn write_processing_instruction(&mut self, target: &str, data: &str) -> RtResult<()> {
        self.close_pending();
        let depth = self.stack.len();
        self.newline(depth);
        self.out.push_str("<?");
        self.out.push_str(target);
        if !data.is_empty() {
            self.out.push(' ');
            self.out.push_str(data);
        }
        self.out.push_str("?>");
        self.mark_content(false);
        Ok(())
    }

    /// Секция CDATA ведёт себя ДВОЙСТВЕННО, и это измерено: отступ перед
    /// собой она получает как узел, а закрывающий тег после неё встаёт
    /// вплотную, как после текста.
    ///
    /// # Errors
    ///
    /// Не отказывает; `Result` — ради единообразия.
    pub fn write_cdata(&mut self, text: &str) -> RtResult<()> {
        self.close_pending();
        let depth = self.stack.len();
        self.newline(depth);
        self.out.push_str("<![CDATA[");
        self.out.push_str(text);
        self.out.push_str("]]>");
        self.mark_content(true);
        Ok(())
    }

    /// Зеркало `write_cdata`: отступа перед собой НЕ получает, а после себя
    /// закрывающий тег с новой строки оставляет (измерено).
    ///
    /// # Errors
    ///
    /// Не отказывает; `Result` — ради единообразия.
    pub fn write_raw(&mut self, text: &str) -> RtResult<()> {
        self.close_pending();
        self.out.push_str(text);
        self.mark_content(false);
        Ok(())
    }

    /// Отдать накопленное. Незакрытые элементы НЕ дописываются: висящий
    /// начальный тег закрывается одним `>`, и на этом всё (измерено —
    /// `<а>`, а не `<а/>` и не `<а></а>`).
    pub fn finish(&mut self) -> String {
        self.close_pending();
        self.stack.clear();
        std::mem::take(&mut self.out)
    }

    pub fn take_path(&mut self) -> Option<PathBuf> {
        self.path.take()
    }
}

/// Экранирование текста узла: апостроф и кавычка остаются как есть
/// (измерено).
fn escape_text(out: &mut String, s: &str) {
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            c => out.push(c),
        }
    }
}

/// Экранирование значения атрибута: к набору текста добавляется кавычка, но
/// НЕ апостроф; табуляция и перевод строки уходят как есть (измерено).
fn escape_attr(out: &mut String, s: &str) {
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            c => out.push(c),
        }
    }
}

// --- Склейка с объектами BSL --------------------------------------------
//
// Как и у JSON: методы живут здесь, наружу уходят через `builtin.rs`.

use crate::object::{BslObject, XmlReaderState};
use crate::EnumValue;

fn as_reader(v: &BslValue) -> RtResult<&std::cell::RefCell<XmlReaderState>> {
    match v {
        BslValue::Object(o) => match &**o {
            BslObject::XmlReader(state) => Ok(state),
            _ => Err(not_applicable(v)),
        },
        _ => Err(not_applicable(v)),
    }
}

fn as_writer(v: &BslValue) -> RtResult<&std::cell::RefCell<Option<XmlWriter>>> {
    match v {
        BslValue::Object(o) => match &**o {
            BslObject::XmlWriter(state) => Ok(state),
            _ => Err(not_applicable(v)),
        },
        _ => Err(not_applicable(v)),
    }
}

fn not_applicable(v: &BslValue) -> RtError {
    RtError::MethodNotApplicable {
        method: "метод XML",
        receiver: v.type_name(),
    }
}

pub fn is_xml_reader(v: &BslValue) -> bool {
    matches!(v, BslValue::Object(o) if matches!(&**o, BslObject::XmlReader(_)))
}

pub fn is_xml_writer(v: &BslValue) -> bool {
    matches!(v, BslValue::Object(o) if matches!(&**o, BslObject::XmlWriter(_)))
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

/// Настройки из аргумента `УстановитьСтроку([Параметры])`.
fn settings_from(arg: Option<&BslValue>) -> RtResult<XmlWriterSettings> {
    match arg {
        None | Some(BslValue::Undefined) => Ok(XmlWriterSettings::default()),
        Some(BslValue::Object(o)) => match &**o {
            BslObject::XmlWriterSettings(s) => Ok(s.clone()),
            _ => Err(RtError::TypeError {
                expected: "ПараметрыЗаписиXML",
                op: "УстановитьСтроку",
            }),
        },
        Some(_) => Err(RtError::TypeError {
            expected: "ПараметрыЗаписиXML",
            op: "УстановитьСтроку",
        }),
    }
}

/// `ЧтениеXML.УстановитьСтроку(Текст)` / `ЗаписьXML.УстановитьСтроку([Параметры])`.
///
/// # Errors
///
/// [`RtError::TypeError`], если получатель не объект XML либо аргумент не
/// того типа.
pub fn set_string(obj: &BslValue, args: &[BslValue]) -> RtResult<()> {
    if let Ok(reader) = as_reader(obj) {
        let text = need_str(args.first(), "УстановитьСтроку")?;
        *reader.borrow_mut() = XmlReaderState::over(XmlParser::new(&text));
        return Ok(());
    }
    let writer = as_writer(obj)?;
    *writer.borrow_mut() = Some(XmlWriter::to_string_target(settings_from(args.first())?));
    Ok(())
}

/// `ОткрытьФайл(Имя)` у обоих объектов XML.
///
/// # Errors
///
/// [`RtError::IoError`], если файл не читается; [`RtError::TypeError`] при
/// неверном аргументе.
pub fn open_file(obj: &BslValue, args: &[BslValue]) -> RtResult<()> {
    let path = need_str(args.first(), "ОткрытьФайл")?;
    if let Ok(reader) = as_reader(obj) {
        let text = std::fs::read_to_string(&path).map_err(|e| RtError::IoError(e.to_string()))?;
        // Платформа терпит сигнатуру UTF-8 в начале файла, а разборщику
        // она видна как символ перед `<` — снимаем.
        let text = text.strip_prefix('\u{feff}').unwrap_or(&text).to_string();
        *reader.borrow_mut() = XmlReaderState::over(XmlParser::new(&text));
        return Ok(());
    }
    let writer = as_writer(obj)?;
    // У файлового приёмника объявление получает `encoding` — измерено на
    // содержимом записанного файла.
    let settings = XmlWriterSettings {
        encoding: Some("UTF-8".to_string()),
        ..XmlWriterSettings::default()
    };
    *writer.borrow_mut() = Some(XmlWriter::to_file(PathBuf::from(path), settings));
    Ok(())
}

/// Разобрать следующий узел. Курсор атрибутов при этом сбрасывается.
///
/// # Errors
///
/// [`RtError::Xml`] на битой разметке.
pub fn read(obj: &BslValue) -> RtResult<BslValue> {
    let reader = as_reader(obj)?;
    let mut state = reader.borrow_mut();
    let Some(parser) = state.parser.as_mut() else {
        return Err(bad("источник для ЧтениеXML не задан"));
    };
    let event = parser.read()?;
    state.attr_cursor = None;
    match event {
        Some(e) => {
            state.depth = state.parser.as_ref().map_or(0, XmlParser::depth);
            state.current = Some(e);
            Ok(BslValue::Boolean(true))
        }
        None => {
            state.current = None;
            Ok(BslValue::Boolean(false))
        }
    }
}

/// `Пропустить()` — проглотить остаток текущего элемента и встать НА его
/// закрывающий тег (измерено; на нетекстовом узле пропускается остаток
/// родителя).
///
/// # Errors
///
/// [`RtError::Xml`] на битой разметке или если пропускать нечего.
pub fn skip(obj: &BslValue) -> RtResult<()> {
    let reader = as_reader(obj)?;
    let mut state = reader.borrow_mut();
    // Глубина снимается ДО заимствования разборщика: после первого же
    // `read` она уже другая, а нужна та, что была на текущем узле.
    let depth = state.depth;
    if depth == 0 {
        return Err(bad("Пропустить вне элемента"));
    }
    let target = depth - 1;
    let Some(parser) = state.parser.as_mut() else {
        return Err(bad("источник для ЧтениеXML не задан"));
    };
    loop {
        let Some(event) = parser.read()? else {
            state.current = None;
            state.depth = 0;
            return Ok(());
        };
        let now = parser.depth();
        if matches!(event, XmlEvent::ElementEnd { .. }) && now == target {
            state.current = Some(event);
            state.depth = now;
            state.attr_cursor = None;
            return Ok(());
        }
    }
}

/// `ПрочитатьАтрибут()` — курсор по атрибутам текущего элемента.
///
/// # Errors
///
/// [`RtError::MethodNotApplicable`], если получатель не `ЧтениеXML`.
pub fn read_attribute(obj: &BslValue) -> RtResult<BslValue> {
    let reader = as_reader(obj)?;
    let mut state = reader.borrow_mut();
    let count = state.attrs().len();
    let next = match state.attr_cursor {
        None => 0,
        Some(i) => i + 1,
    };
    if next >= count {
        state.attr_cursor = Some(count);
        return Ok(BslValue::Boolean(false));
    }
    state.attr_cursor = Some(next);
    Ok(BslValue::Boolean(true))
}

/// `ПерейтиКСодержимому()` -> член `ТипУзлаXML`.
///
/// # Errors
///
/// [`RtError::Xml`] на битой разметке.
pub fn move_to_content(obj: &BslValue) -> RtResult<BslValue> {
    loop {
        {
            let reader = as_reader(obj)?;
            let state = reader.borrow();
            if matches!(
                state.current,
                Some(XmlEvent::ElementStart { .. })
                    | Some(XmlEvent::ElementEnd { .. })
                    | Some(XmlEvent::Text(_))
            ) {
                drop(state);
                return node_type(obj);
            }
        }
        if read(obj)? == BslValue::Boolean(false) {
            return Ok(BslValue::Enum(EnumValue::XmlNothing));
        }
    }
}

/// `ТипУзла` — член `ТипУзлаXML`.
///
/// # Errors
///
/// [`RtError::MethodNotApplicable`], если получатель не `ЧтениеXML`.
pub fn node_type(obj: &BslValue) -> RtResult<BslValue> {
    let reader = as_reader(obj)?;
    let state = reader.borrow();
    if state.attr_cursor.is_some_and(|i| i < state.attrs().len()) {
        return Ok(BslValue::Enum(EnumValue::XmlAttribute));
    }
    let v = match &state.current {
        None => EnumValue::XmlNothing,
        Some(XmlEvent::ElementStart { .. }) => EnumValue::XmlElementStart,
        Some(XmlEvent::ElementEnd { .. }) => EnumValue::XmlElementEnd,
        Some(XmlEvent::Text(_)) => EnumValue::XmlText,
        Some(XmlEvent::ProcessingInstruction { .. }) => EnumValue::XmlProcessingInstruction,
    };
    Ok(BslValue::Enum(v))
}

/// `Имя` текущего узла; у текста это `#text` (измерено).
///
/// # Errors
///
/// [`RtError::MethodNotApplicable`], если получатель не `ЧтениеXML`.
pub fn name(obj: &BslValue) -> RtResult<BslValue> {
    let reader = as_reader(obj)?;
    let state = reader.borrow();
    if let Some(a) = state.current_attr() {
        return Ok(BslValue::Str(BslString::from_str(&a.name)));
    }
    let s = match &state.current {
        None => String::new(),
        Some(XmlEvent::ElementStart { name, .. }) | Some(XmlEvent::ElementEnd { name, .. }) => {
            name.clone()
        }
        Some(XmlEvent::Text(_)) => TEXT_NODE_NAME.to_string(),
        Some(XmlEvent::ProcessingInstruction { target, .. }) => target.clone(),
    };
    Ok(BslValue::Str(BslString::from_str(&s)))
}

/// `Значение` текущего узла; у элемента оно пустое (измерено).
///
/// # Errors
///
/// [`RtError::MethodNotApplicable`], если получатель не `ЧтениеXML`.
pub fn value(obj: &BslValue) -> RtResult<BslValue> {
    let reader = as_reader(obj)?;
    let state = reader.borrow();
    if let Some(a) = state.current_attr() {
        return Ok(BslValue::Str(BslString::from_str(&a.value)));
    }
    let s = match &state.current {
        Some(XmlEvent::Text(t)) => t.clone(),
        Some(XmlEvent::ProcessingInstruction { data, .. }) => data.clone(),
        _ => String::new(),
    };
    Ok(BslValue::Str(BslString::from_str(&s)))
}

/// `ЛокальноеИмя` — имя без префикса.
///
/// # Errors
///
/// [`RtError::MethodNotApplicable`], если получатель не `ЧтениеXML`.
pub fn local_name(obj: &BslValue) -> RtResult<BslValue> {
    let full = name(obj)?;
    let BslValue::Str(s) = &full else {
        return Ok(full);
    };
    Ok(BslValue::Str(BslString::from_str(local_of(&s.to_string()))))
}

/// `Префикс` — часть имени до двоеточия.
///
/// # Errors
///
/// [`RtError::MethodNotApplicable`], если получатель не `ЧтениеXML`.
pub fn prefix(obj: &BslValue) -> RtResult<BslValue> {
    let full = name(obj)?;
    let BslValue::Str(s) = &full else {
        return Ok(full);
    };
    Ok(BslValue::Str(BslString::from_str(prefix_of(
        &s.to_string(),
    ))))
}

/// `URIПространстваИмен` текущего элемента.
///
/// # Errors
///
/// [`RtError::MethodNotApplicable`], если получатель не `ЧтениеXML`.
pub fn namespace_uri(obj: &BslValue) -> RtResult<BslValue> {
    let reader = as_reader(obj)?;
    let state = reader.borrow();
    let s = match &state.current {
        Some(XmlEvent::ElementStart { uri, .. }) | Some(XmlEvent::ElementEnd { uri, .. }) => {
            uri.clone()
        }
        _ => String::new(),
    };
    Ok(BslValue::Str(BslString::from_str(&s)))
}

/// `КоличествоАтрибутов()`.
///
/// # Errors
///
/// [`RtError::MethodNotApplicable`], если получатель не `ЧтениеXML`.
pub fn attribute_count(obj: &BslValue) -> RtResult<BslValue> {
    let reader = as_reader(obj)?;
    let state = reader.borrow();
    Ok(BslValue::Number(bsl_number::BslNumber::from_i64(
        state.attrs().len() as i64,
    )))
}

/// `ИмяАтрибута(Индекс)`. Индекс за границей — `Неопределено`, как и у
/// `ЗначениеАтрибута` (у которого это измерено).
///
/// # Errors
///
/// [`RtError::BadIndex`], если индекс не целое неотрицательное.
pub fn attribute_name(obj: &BslValue, args: &[BslValue]) -> RtResult<BslValue> {
    let reader = as_reader(obj)?;
    let state = reader.borrow();
    let idx = index_arg(args.first())?;
    // Индекс за границей списка даёт `Неопределено` — измерено отдельно
    // от `ЗначениеАтрибута`, у обоих одинаково.
    Ok(state.attrs().get(idx).map_or(BslValue::Undefined, |a| {
        BslValue::Str(BslString::from_str(&a.name))
    }))
}

/// `ЗначениеАтрибута(ИмяЛибоИндекс)` -> значение либо `Неопределено`
/// (измерено: у отсутствующего атрибута тип результата — «Не определено»).
///
/// # Errors
///
/// [`RtError::TypeError`], если аргумент не строка и не число.
pub fn attribute_value(obj: &BslValue, args: &[BslValue]) -> RtResult<BslValue> {
    let reader = as_reader(obj)?;
    let state = reader.borrow();
    match args.first() {
        Some(BslValue::Str(s)) => {
            let wanted = s.to_string();
            Ok(state
                .attrs()
                .iter()
                .find(|a| a.name == wanted)
                .map_or(BslValue::Undefined, |a| {
                    BslValue::Str(BslString::from_str(&a.value))
                }))
        }
        Some(BslValue::Number(_)) => {
            let idx = index_arg(args.first())?;
            Ok(state.attrs().get(idx).map_or(BslValue::Undefined, |a| {
                BslValue::Str(BslString::from_str(&a.value))
            }))
        }
        _ => Err(RtError::TypeError {
            expected: "Строка либо Число",
            op: "ЗначениеАтрибута",
        }),
    }
}

fn index_arg(arg: Option<&BslValue>) -> RtResult<usize> {
    match arg {
        Some(BslValue::Number(n)) => {
            let i = n.to_i64_exact().ok_or(RtError::BadIndex)?;
            usize::try_from(i).map_err(|_| RtError::BadIndex)
        }
        _ => Err(RtError::TypeError {
            expected: "Число",
            op: "индекс атрибута",
        }),
    }
}

// --- Методы записи ------------------------------------------------------

fn with_writer<R>(obj: &BslValue, f: impl FnOnce(&mut XmlWriter) -> RtResult<R>) -> RtResult<R> {
    let writer = as_writer(obj)?;
    let mut slot = writer.borrow_mut();
    let w = slot
        .as_mut()
        .ok_or_else(|| bad("приёмник для ЗаписьXML не задан"))?;
    f(w)
}

/// `ЗаписатьОбъявлениеXML()`.
///
/// # Errors
///
/// [`RtError::Xml`], если объявление пишется не первым.
pub fn write_declaration(obj: &BslValue) -> RtResult<()> {
    with_writer(obj, XmlWriter::write_declaration)
}

/// `ЗаписатьНачалоЭлемента(Имя)`.
///
/// # Errors
///
/// [`RtError::TypeError`], если имя не строка; [`RtError::Xml`], если
/// корневой элемент уже записан.
pub fn write_start_element(obj: &BslValue, args: &[BslValue]) -> RtResult<()> {
    let name = need_str(args.first(), "ЗаписатьНачалоЭлемента")?;
    with_writer(obj, |w| w.write_start_element(&name))
}

/// `ЗаписатьКонецЭлемента()`.
///
/// # Errors
///
/// [`RtError::Xml`], если открытого элемента нет.
pub fn write_end_element(obj: &BslValue) -> RtResult<()> {
    with_writer(obj, XmlWriter::write_end_element)
}

/// `ЗаписатьАтрибут(Имя, Значение)` — оба только строки (измерено: число
/// даёт ошибку).
///
/// # Errors
///
/// [`RtError::TypeError`] на нестроковом аргументе; [`RtError::Xml`], если
/// начальный тег уже закрыт.
pub fn write_attribute(obj: &BslValue, args: &[BslValue]) -> RtResult<()> {
    let name = need_str(args.first(), "ЗаписатьАтрибут")?;
    let value = need_str(args.get(1), "ЗаписатьАтрибут")?;
    with_writer(obj, |w| w.write_attribute(&name, &value))
}

/// `ЗаписатьТекст(Текст)`.
///
/// # Errors
///
/// [`RtError::TypeError`], если аргумент не строка.
pub fn write_text(obj: &BslValue, args: &[BslValue]) -> RtResult<()> {
    let text = need_str(args.first(), "ЗаписатьТекст")?;
    with_writer(obj, |w| w.write_text(&text))
}

/// `ЗаписатьКомментарий(Текст)`.
///
/// # Errors
///
/// [`RtError::TypeError`], если аргумент не строка.
pub fn write_comment(obj: &BslValue, args: &[BslValue]) -> RtResult<()> {
    let text = need_str(args.first(), "ЗаписатьКомментарий")?;
    with_writer(obj, |w| w.write_comment(&text))
}

/// `ЗаписатьСекциюCDATA(Текст)`.
///
/// # Errors
///
/// [`RtError::TypeError`], если аргумент не строка.
pub fn write_cdata(obj: &BslValue, args: &[BslValue]) -> RtResult<()> {
    let text = need_str(args.first(), "ЗаписатьСекциюCDATA")?;
    with_writer(obj, |w| w.write_cdata(&text))
}

/// `ЗаписатьИнструкциюОбработки(Имя, Текст)`.
///
/// # Errors
///
/// [`RtError::TypeError`], если аргумент не строка.
pub fn write_processing_instruction(obj: &BslValue, args: &[BslValue]) -> RtResult<()> {
    let target = need_str(args.first(), "ЗаписатьИнструкциюОбработки")?;
    let data = need_str(args.get(1), "ЗаписатьИнструкциюОбработки")?;
    with_writer(obj, |w| w.write_processing_instruction(&target, &data))
}

/// `ЗаписатьБезОбработки(Текст)`.
///
/// # Errors
///
/// [`RtError::TypeError`], если аргумент не строка.
pub fn write_raw(obj: &BslValue, args: &[BslValue]) -> RtResult<()> {
    let text = need_str(args.first(), "ЗаписатьБезОбработки")?;
    with_writer(obj, |w| w.write_raw(&text))
}

/// `ЗаписьXML.Закрыть()` -> текст для строкового приёмника либо пустая
/// строка для файлового. Второй вызов подряд отдаёт пустую строку —
/// измерено.
///
/// # Errors
///
/// [`RtError::IoError`], если файл не записался.
pub fn close_writer(obj: &BslValue) -> RtResult<BslValue> {
    let writer = as_writer(obj)?;
    let mut slot = writer.borrow_mut();
    let Some(w) = slot.as_mut() else {
        return Ok(BslValue::Str(BslString::from_str("")));
    };
    let text = w.finish();
    if let Some(path) = w.take_path() {
        std::fs::write(&path, text.as_bytes()).map_err(|e| RtError::IoError(e.to_string()))?;
        *slot = None;
        return Ok(BslValue::Str(BslString::from_str("")));
    }
    *slot = None;
    Ok(BslValue::Str(BslString::from_str(&text)))
}

/// `ЧтениеXML.Закрыть()` — источник отпускается, объект остаётся годным для
/// нового `УстановитьСтроку`.
///
/// # Errors
///
/// [`RtError::MethodNotApplicable`], если получатель не `ЧтениеXML`.
pub fn close_reader(obj: &BslValue) -> RtResult<BslValue> {
    let reader = as_reader(obj)?;
    *reader.borrow_mut() = XmlReaderState::default();
    Ok(BslValue::Undefined)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn events(text: &str) -> Vec<XmlEvent> {
        let mut p = XmlParser::new(text);
        let mut out = Vec::new();
        while let Some(e) = p.read().expect("разбор") {
            out.push(e);
        }
        out
    }

    fn start(name: &str) -> XmlEvent {
        XmlEvent::ElementStart {
            name: name.into(),
            uri: String::new(),
            attrs: Vec::new(),
        }
    }

    fn end(name: &str) -> XmlEvent {
        XmlEvent::ElementEnd {
            name: name.into(),
            uri: String::new(),
        }
    }

    #[test]
    fn empty_element_is_indistinguishable_from_a_pair_of_tags() {
        // Замеры XML.READ.EMPTY_ELEMENT и XML.READ.EMPTY_PAIR.
        assert_eq!(events("<а/>"), vec![start("а"), end("а")]);
        assert_eq!(events("<а></а>"), events("<а/>"));
    }

    #[test]
    fn declaration_and_comment_are_not_reported_but_a_comment_splits_text() {
        // Замеры XML.READ.DECLARATION, XML.READ.COMMENT и
        // XML.READ.TEXT_SPLIT_BY_COMMENT.
        assert_eq!(
            events("<?xml version=\"1.0\"?><а/>"),
            vec![start("а"), end("а")]
        );
        assert_eq!(events("<а><!-- сюда --></а>"), vec![start("а"), end("а")]);
        assert_eq!(
            events("<а>раз<!--к-->два</а>"),
            vec![
                start("а"),
                XmlEvent::Text("раз".into()),
                XmlEvent::Text("два".into()),
                end("а"),
            ]
        );
    }

    #[test]
    fn cdata_merges_into_the_surrounding_text_run() {
        // Замер XML.READ.TEXT_TWICE_SPLIT: ОДИН узел, а не три. Именно
        // этим секция отличается от комментария.
        assert_eq!(
            events("<а>раз<![CDATA[два]]>три</а>"),
            vec![start("а"), XmlEvent::Text("раздватри".into()), end("а")]
        );
        // Замер XML.READ.CDATA: секция в начале содержимого — тот же текст.
        assert_eq!(
            events("<а><![CDATA[<не разметка>]]></а>"),
            vec![start("а"), XmlEvent::Text("<не разметка>".into()), end("а"),]
        );
    }

    #[test]
    fn whitespace_only_text_is_dropped_but_padding_survives() {
        // Замеры XML.READ.WHITESPACE_BETWEEN и XML.READ.TEXT_PADDED.
        assert_eq!(
            events("<а>  <б/>  </а>"),
            vec![start("а"), start("б"), end("б"), end("а")]
        );
        assert_eq!(
            events("<а> т </а>"),
            vec![start("а"), XmlEvent::Text(" т ".into()), end("а")]
        );
    }

    #[test]
    fn entities_and_character_references_are_decoded() {
        // Замеры XML.READ.ENTITIES и XML.READ.CHAR_REF.
        assert_eq!(
            events("<а>&amp;&lt;&gt;&quot;&apos;</а>"),
            vec![start("а"), XmlEvent::Text("&<>\"'".into()), end("а")]
        );
        assert_eq!(
            events("<а>&#65;&#x42;</а>"),
            vec![start("а"), XmlEvent::Text("AB".into()), end("а")]
        );
    }

    #[test]
    fn namespace_declaration_stays_an_attribute_and_resolves_the_prefix() {
        // Замеры XML.READ.NS_ATTR_COUNT и XML.READ.NAME_PARTS_NS: объявление
        // видно среди атрибутов И при этом резолвит префикс своего же
        // элемента.
        let ev = events("<п:а xmlns:п=\"http://прим\" х=\"1\">т</п:а>");
        let XmlEvent::ElementStart { name, uri, attrs } = &ev[0] else {
            panic!("ожидалось начало элемента: {ev:?}");
        };
        assert_eq!(name, "п:а");
        assert_eq!(uri, "http://прим");
        assert_eq!(attrs.len(), 2);
        assert_eq!(attrs[0].name, "xmlns:п");
        assert_eq!(attrs[1].name, "х");
        assert_eq!(local_of(name), "а");
        assert_eq!(prefix_of(name), "п");
    }

    #[test]
    fn broken_markup_is_an_error() {
        // Замеры XML.READ.UNCLOSED, GARBAGE, EMPTY_STRING и TWO_ROOTS.
        for text in ["<а><б></а>", "не разметка", "", "<а/><б/>"] {
            let mut p = XmlParser::new(text);
            let mut failed = false;
            loop {
                match p.read() {
                    Ok(Some(_)) => continue,
                    Ok(None) => break,
                    Err(_) => {
                        failed = true;
                        break;
                    }
                }
            }
            assert!(failed, "битый ввод принят молча: {text:?}");
        }
    }

    fn write(f: impl FnOnce(&mut XmlWriter)) -> String {
        let mut w = XmlWriter::to_string_target(XmlWriterSettings::default());
        f(&mut w);
        w.finish()
    }

    #[test]
    fn default_formatting_indents_elements_but_not_text() {
        // Замеры XML.WRITE.DEFAULT_FORMAT и XML.WRITE.DEEP_INDENT.
        let out = write(|w| {
            w.write_start_element("а").unwrap();
            w.write_start_element("б").unwrap();
            w.write_text("т").unwrap();
            w.write_end_element().unwrap();
            w.write_end_element().unwrap();
        });
        assert_eq!(out, "<а>\n\t<б>т</б>\n</а>");

        let deep = write(|w| {
            w.write_start_element("а").unwrap();
            w.write_start_element("б").unwrap();
            w.write_start_element("в").unwrap();
            w.write_end_element().unwrap();
            w.write_end_element().unwrap();
            w.write_end_element().unwrap();
        });
        assert_eq!(deep, "<а>\n\t<б>\n\t\t<в/>\n\t</б>\n</а>");
    }

    #[test]
    fn mixed_content_keeps_the_closing_tag_tight_after_text() {
        // Замер XML.WRITE.MIXED.
        let out = write(|w| {
            w.write_start_element("а").unwrap();
            w.write_text("т").unwrap();
            w.write_start_element("б").unwrap();
            w.write_end_element().unwrap();
            w.write_text("у").unwrap();
            w.write_end_element().unwrap();
        });
        assert_eq!(out, "<а>т\n\t<б/>у</а>");
    }

    #[test]
    fn cdata_and_raw_are_mirror_images_of_each_other() {
        // Замеры XML.WRITE.CDATA_SECTION и XML.WRITE.RAW: секция получает
        // отступ перед собой, но не перевод строки после; у записи без
        // обработки ровно наоборот.
        let cdata = write(|w| {
            w.write_start_element("а").unwrap();
            w.write_cdata("<не разметка>").unwrap();
            w.write_end_element().unwrap();
        });
        assert_eq!(cdata, "<а>\n\t<![CDATA[<не разметка>]]></а>");

        let raw = write(|w| {
            w.write_start_element("а").unwrap();
            w.write_raw("<сырое/>").unwrap();
            w.write_end_element().unwrap();
        });
        assert_eq!(raw, "<а><сырое/>\n</а>");
    }

    #[test]
    fn empty_text_leaves_the_element_collapsed() {
        // Замер XML.WRITE.TEXT_EMPTY.
        let out = write(|w| {
            w.write_start_element("а").unwrap();
            w.write_text("").unwrap();
            w.write_end_element().unwrap();
        });
        assert_eq!(out, "<а/>");
    }

    #[test]
    fn escaping_differs_between_text_and_attribute() {
        // Замеры XML.WRITE.ESCAPE_TEXT и XML.WRITE.ESCAPE_ATTR: апостроф не
        // экранируется нигде, кавычка — только в атрибуте.
        let text = write(|w| {
            w.write_start_element("а").unwrap();
            w.write_text("&<>\"'").unwrap();
            w.write_end_element().unwrap();
        });
        assert_eq!(text, "<а>&amp;&lt;&gt;\"'</а>");

        let attr = write(|w| {
            w.write_start_element("а").unwrap();
            w.write_attribute("х", "&<>\"'").unwrap();
            w.write_end_element().unwrap();
        });
        assert_eq!(attr, "<а х=\"&amp;&lt;&gt;&quot;'\"/>");
    }

    #[test]
    fn unclosed_element_is_not_completed_on_finish() {
        // Замер XML.WRITE.UNCLOSED: именно `<а>`, а не `<а/>`.
        let out = write(|w| {
            w.write_start_element("а").unwrap();
        });
        assert_eq!(out, "<а>");
    }

    #[test]
    fn structure_violations_are_rejected() {
        // Замеры XML.WRITE.UNBALANCED_END, ATTR_AFTER_TEXT, TWO_ROOTS и
        // DECL_LATE.
        let mut w = XmlWriter::to_string_target(XmlWriterSettings::default());
        assert!(w.write_end_element().is_err());

        let mut w = XmlWriter::to_string_target(XmlWriterSettings::default());
        w.write_start_element("а").unwrap();
        w.write_text("т").unwrap();
        assert!(w.write_attribute("х", "1").is_err());

        let mut w = XmlWriter::to_string_target(XmlWriterSettings::default());
        w.write_start_element("а").unwrap();
        w.write_end_element().unwrap();
        assert!(w.write_start_element("б").is_err());

        let mut w = XmlWriter::to_string_target(XmlWriterSettings::default());
        w.write_start_element("а").unwrap();
        assert!(w.write_declaration().is_err());
    }

    #[test]
    fn indent_flag_removes_every_line_break() {
        // Замеры XML.WRITE.SETTINGS_NO_INDENT и XML.WRITE.DECL_NO_INDENT.
        let settings = XmlWriterSettings {
            encoding: Some("UTF-8".to_string()),
            version: "1.0".to_string(),
            indent: false,
        };
        let mut w = XmlWriter::to_string_target(settings);
        w.write_declaration().unwrap();
        w.write_start_element("а").unwrap();
        w.write_start_element("б").unwrap();
        w.write_end_element().unwrap();
        w.write_end_element().unwrap();
        assert_eq!(
            w.finish(),
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?><а><б/></а>"
        );
    }

    #[test]
    fn round_trip_survives_escaping() {
        // Замер XML.ROUND_TRIP.
        let text = write(|w| {
            w.write_start_element("а").unwrap();
            w.write_attribute("х", "1").unwrap();
            w.write_text("т&т").unwrap();
            w.write_end_element().unwrap();
        });
        assert_eq!(text, "<а х=\"1\">т&amp;т</а>");
        let ev = events(&text);
        assert_eq!(ev[1], XmlEvent::Text("т&т".into()));
    }
}
