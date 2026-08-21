//! Поверхность `ЧтениеXML`/`ЗаписьXML`/`ПараметрыЗаписиXML`.
//!
//! Парсерное и писательское ядро живёт в `crate::core` — им пользуется и
//! шаблонный разбор MXL; здесь объекты встроенного языка поверх него.

use std::cell::RefCell;
use std::rc::Rc;

use std::path::PathBuf;

use crate::core::{
    COMMENT_NODE_NAME, TEXT_NODE_NAME, XmlAttr, XmlEvent, XmlParser, XmlWriter, XmlWriterSettings,
    local_of, prefix_of,
};
use bsl_rt::{
    BslNumber, BslString, BslValue, CallContext, EnumValue, MethodCode, MethodDescriptor,
    ObjectProtocol, RtError, RtResult, TypeDescriptor, TypeId,
};

fn bad(what: impl Into<String>) -> RtError {
    RtError::Xml(what.into())
}

/// Состояние `ЧтениеXML`.
///
/// Кроме текущего события хранится ещё две вещи, и обе — из-за того, что у
/// платформы курсор один на две сущности. `attr_cursor` — позиция обхода
/// `ПрочитатьАтрибут`: пока он стоит на атрибуте, `ТипУзла`, `Имя` и
/// `Значение` показывают АТРИБУТ, а не сам элемент (измерено). `depth` —
/// глубина открытых элементов на момент текущего события: по ней
/// `Пропустить` понимает, до какого закрывающего тега глотать, и она обязана
/// быть снята ДО того, как разборщик уйдёт вперёд.
#[derive(Debug, Default)]
pub struct XmlReaderState {
    pub parser: Option<XmlParser>,
    pub current: Option<XmlEvent>,
    pub attr_cursor: Option<usize>,
    pub depth: usize,
}

impl XmlReaderState {
    /// Свежее состояние над готовым разборщиком.
    pub fn over(parser: XmlParser) -> Self {
        XmlReaderState {
            parser: Some(parser),
            current: None,
            attr_cursor: None,
            depth: 0,
        }
    }

    /// Атрибуты текущего узла; у неэлементного узла их нет.
    pub fn attrs(&self) -> &[XmlAttr] {
        match &self.current {
            Some(XmlEvent::ElementStart { attrs, .. }) => attrs.as_slice(),
            _ => &[],
        }
    }

    /// Атрибут, на котором стоит курсор `ПрочитатьАтрибут`.
    pub fn current_attr(&self) -> Option<&XmlAttr> {
        self.attr_cursor.and_then(|i| self.attrs().get(i))
    }
}

// --- Склейка с объектами BSL --------------------------------------------

/// `ЧтениеXML`.
#[derive(Debug)]
pub struct XmlReaderObject {
    pub(crate) state: Rc<RefCell<XmlReaderState>>,
}

/// `ЗаписьXML` — до `УстановитьСтроку`/`ОткрытьФайл` писателя нет.
#[derive(Debug)]
pub struct XmlWriterObject {
    writer: Rc<RefCell<Option<XmlWriter>>>,
}

/// `ПараметрыЗаписиXML` — неизменяемый набор настроек.
#[derive(Debug)]
pub struct XmlWriterSettingsObject {
    settings: XmlWriterSettings,
}

fn as_reader(v: &dyn ObjectProtocol) -> RtResult<&RefCell<XmlReaderState>> {
    match v.downcast_ref::<XmlReaderObject>() {
        Some(reader) => Ok(&reader.state),
        _ => Err(not_applicable(v)),
    }
}

fn as_writer(v: &dyn ObjectProtocol) -> RtResult<&RefCell<Option<XmlWriter>>> {
    match v.downcast_ref::<XmlWriterObject>() {
        Some(writer) => Ok(&writer.writer),
        _ => Err(not_applicable(v)),
    }
}

fn not_applicable(v: &dyn ObjectProtocol) -> RtError {
    RtError::MethodNotApplicable {
        method: "метод XML",
        receiver: v.type_descriptor().name,
    }
}

/// Объект за значением аргумента: мост из мира значений, где читатель или
/// писатель приходит аргументом глобальной функции (`ПрочитатьXML`,
/// `ЗаписатьXML`), а не получателем метода. Не-объект получает ту же
/// ошибку «метод XML не применим», что и объект чужого типа.
pub(crate) fn arg_object(v: &BslValue) -> RtResult<&dyn ObjectProtocol> {
    v.object_ref()
        .map(bsl_rt::ObjectRef::as_dyn)
        .ok_or_else(|| RtError::MethodNotApplicable {
            method: "метод XML",
            receiver: v.type_name(),
        })
}

pub fn is_xml_reader(v: &BslValue) -> bool {
    v.object_ref()
        .is_some_and(|object| object.downcast_ref::<XmlReaderObject>().is_some())
}

pub fn is_xml_writer(v: &BslValue) -> bool {
    v.object_ref()
        .is_some_and(|object| object.downcast_ref::<XmlWriterObject>().is_some())
}

/// `Новый ЧтениеXML` — читатель без источника.
pub fn new_xml_reader() -> BslValue {
    BslValue::new_object(XmlReaderObject {
        state: Rc::new(RefCell::new(XmlReaderState::default())),
    })
}

/// `Новый ЗаписьXML` — писатель без приёмника.
pub fn new_xml_writer() -> BslValue {
    BslValue::new_object(XmlWriterObject {
        writer: Rc::new(RefCell::new(None)),
    })
}

/// `Новый ПараметрыЗаписиXML([Кодировка][, Версия][, ИспользоватьОтступ])`.
pub fn new_xml_writer_settings(settings: XmlWriterSettings) -> BslValue {
    BslValue::new_object(XmlWriterSettingsObject { settings })
}

/// Настройки из аргументов конструктора — прежние проверки типов.
///
/// # Errors
///
/// [`RtError::TypeError`], если аргумент не того типа.
pub fn writer_settings_from_args(
    encoding: &BslValue,
    version: &BslValue,
    indent: &BslValue,
) -> RtResult<BslValue> {
    let mut settings = XmlWriterSettings::default();
    match encoding {
        BslValue::Undefined => {}
        BslValue::Str(s) => settings.encoding = Some(s.to_string()),
        _ => {
            return Err(RtError::TypeError {
                expected: "Строка",
                op: "Новый ПараметрыЗаписиXML",
            });
        }
    }
    match version {
        BslValue::Undefined => {}
        BslValue::Str(s) => settings.version = s.to_string(),
        _ => {
            return Err(RtError::TypeError {
                expected: "Строка",
                op: "Новый ПараметрыЗаписиXML",
            });
        }
    }
    match indent {
        BslValue::Undefined => {}
        BslValue::Boolean(b) => settings.indent = *b,
        _ => {
            return Err(RtError::TypeError {
                expected: "Булево",
                op: "Новый ПараметрыЗаписиXML",
            });
        }
    }
    Ok(new_xml_writer_settings(settings))
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
        Some(value @ BslValue::Object(_)) => match value
            .object_ref()
            .and_then(|object| object.downcast_ref::<XmlWriterSettingsObject>())
        {
            Some(settings) => Ok(settings.settings.clone()),
            None => Err(RtError::TypeError {
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
pub fn set_string(obj: &dyn ObjectProtocol, args: &[BslValue]) -> RtResult<()> {
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
pub fn open_file(obj: &dyn ObjectProtocol, args: &[BslValue]) -> RtResult<()> {
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
pub fn read(obj: &dyn ObjectProtocol) -> RtResult<BslValue> {
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
pub fn skip(obj: &dyn ObjectProtocol) -> RtResult<()> {
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
pub fn read_attribute(obj: &dyn ObjectProtocol) -> RtResult<BslValue> {
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
pub fn move_to_content(obj: &dyn ObjectProtocol) -> RtResult<BslValue> {
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
pub fn node_type(obj: &dyn ObjectProtocol) -> RtResult<BslValue> {
    node_type_from(as_reader(obj)?)
}

fn node_type_from(reader: &RefCell<XmlReaderState>) -> RtResult<BslValue> {
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
        // Недостижимо: комментарии разборщик отдаёт только построителю
        // DOM, а тот не оставляет их в состоянии читателя. Ветка написана
        // явно, чтобы `match` оставался исчерпывающим.
        Some(XmlEvent::Comment(_)) => EnumValue::XmlComment,
        Some(XmlEvent::EntityReference { .. }) => EnumValue::XmlEntityReference,
    };
    Ok(BslValue::Enum(v))
}

/// `Имя` текущего узла; у текста это `#text` (измерено).
///
/// # Errors
///
/// [`RtError::MethodNotApplicable`], если получатель не `ЧтениеXML`.
/// Обёртка над [`name_from`] для получателя-значения. Продуктовые пути
/// ходят через `get_property`/таблицу методов; снаружи модуля имя узла
/// читает только тест round-trip XDTO.
#[cfg(test)]
pub fn name(obj: &dyn ObjectProtocol) -> RtResult<BslValue> {
    name_from(as_reader(obj)?)
}

fn name_from(reader: &RefCell<XmlReaderState>) -> RtResult<BslValue> {
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
        // Недостижимо — см. `node_type`.
        Some(XmlEvent::Comment(_)) => COMMENT_NODE_NAME.to_string(),
        // У ссылки на сущность `Имя` — имя сущности, а `Значение` пусто
        // (измерено).
        Some(XmlEvent::EntityReference { name }) => name.clone(),
    };
    Ok(BslValue::Str(BslString::from_str(&s)))
}

/// `Значение` текущего узла; у элемента оно пустое (измерено).
///
/// # Errors
///
/// [`RtError::MethodNotApplicable`], если получатель не `ЧтениеXML`.
fn value_from(reader: &RefCell<XmlReaderState>) -> RtResult<BslValue> {
    let state = reader.borrow();
    if let Some(a) = state.current_attr() {
        return Ok(BslValue::Str(BslString::from_str(&a.value)));
    }
    let s = match &state.current {
        Some(XmlEvent::Text(t)) => t.clone(),
        Some(XmlEvent::ProcessingInstruction { data, .. }) => data.clone(),
        // Недостижимо — см. `node_type`; ответ дан по образцу дерева DOM,
        // где значение комментария и есть его текст.
        Some(XmlEvent::Comment(t)) => t.clone(),
        // У элемента значения нет (измерено), у ссылки на сущность — тоже:
        // `Значение` на ней пусто, хотя текст замены известен.
        Some(XmlEvent::ElementStart { .. })
        | Some(XmlEvent::ElementEnd { .. })
        | Some(XmlEvent::EntityReference { .. })
        | None => String::new(),
    };
    Ok(BslValue::Str(BslString::from_str(&s)))
}

/// `ЛокальноеИмя` — имя без префикса.
///
/// # Errors
///
/// [`RtError::MethodNotApplicable`], если получатель не `ЧтениеXML`.
fn local_name_from(reader: &RefCell<XmlReaderState>) -> RtResult<BslValue> {
    let full = name_from(reader)?;
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
fn prefix_from(reader: &RefCell<XmlReaderState>) -> RtResult<BslValue> {
    let full = name_from(reader)?;
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
fn namespace_uri_from(reader: &RefCell<XmlReaderState>) -> RtResult<BslValue> {
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
pub fn attribute_count(obj: &dyn ObjectProtocol) -> RtResult<BslValue> {
    let reader = as_reader(obj)?;
    let state = reader.borrow();
    Ok(BslValue::Number(BslNumber::from_i64(
        state.attrs().len() as i64
    )))
}

/// `ИмяАтрибута(Индекс)`. Индекс за границей — `Неопределено`, как и у
/// `ЗначениеАтрибута` (у которого это измерено).
///
/// # Errors
///
/// [`RtError::BadIndex`], если индекс не целое неотрицательное.
pub fn attribute_name(obj: &dyn ObjectProtocol, args: &[BslValue]) -> RtResult<BslValue> {
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
pub fn attribute_value(obj: &dyn ObjectProtocol, args: &[BslValue]) -> RtResult<BslValue> {
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

/// Доступ к писателю получателя. `pub(crate)`, потому что тем же писателем
/// пишет дерево DOM (`dom::write`): второго сериализатора XML в рантайме нет.
pub fn with_writer<R>(
    obj: &dyn ObjectProtocol,
    f: impl FnOnce(&mut XmlWriter) -> RtResult<R>,
) -> RtResult<R> {
    let writer = as_writer(obj)?;
    let mut slot = writer.borrow_mut();
    let w = slot
        .as_mut()
        .ok_or_else(|| bad("приёмник для ЗаписьXML не задан"))?;
    f(w)
}

/// Доступ к состоянию читателя. `pub(crate)` по той же причине, что и
/// [`with_writer`]: разбор XML в экземпляры XDTO (`xdto::factory_read_xml`)
/// идёт по СОБЫТИЯМ того же `ЧтениеXML`, и второго разборщика для этого
/// заводить не за чем. Читатель после вызова остаётся там, куда его
/// подвинул `f`, — позиция наблюдаема из BSL.
///
/// # Errors
///
/// [`RtError::MethodNotApplicable`], если получатель не `ЧтениеXML`, плюс
/// всё, чем ответит `f`.
pub fn with_reader<R>(
    obj: &dyn ObjectProtocol,
    f: impl FnOnce(&mut XmlReaderState) -> RtResult<R>,
) -> RtResult<R> {
    let reader = as_reader(obj)?;
    let mut state = reader.borrow_mut();
    f(&mut state)
}

/// `ЗаписатьОбъявлениеXML()`.
///
/// # Errors
///
/// [`RtError::Xml`], если объявление пишется не первым.
pub fn write_declaration(obj: &dyn ObjectProtocol) -> RtResult<()> {
    with_writer(obj, XmlWriter::write_declaration)
}

/// `ЗаписатьНачалоЭлемента(Имя)`.
///
/// # Errors
///
/// [`RtError::TypeError`], если имя не строка; [`RtError::Xml`], если
/// корневой элемент уже записан.
pub fn write_start_element(obj: &dyn ObjectProtocol, args: &[BslValue]) -> RtResult<()> {
    let name = need_str(args.first(), "ЗаписатьНачалоЭлемента")?;
    with_writer(obj, |w| w.write_start_element(&name))
}

/// `ЗаписатьКонецЭлемента()`.
///
/// # Errors
///
/// [`RtError::Xml`], если открытого элемента нет.
pub fn write_end_element(obj: &dyn ObjectProtocol) -> RtResult<()> {
    with_writer(obj, XmlWriter::write_end_element)
}

/// `ЗаписатьАтрибут(Имя, Значение)` — оба только строки (измерено: число
/// даёт ошибку).
///
/// # Errors
///
/// [`RtError::TypeError`] на нестроковом аргументе; [`RtError::Xml`], если
/// начальный тег уже закрыт.
pub fn write_attribute(obj: &dyn ObjectProtocol, args: &[BslValue]) -> RtResult<()> {
    let name = need_str(args.first(), "ЗаписатьАтрибут")?;
    let value = need_str(args.get(1), "ЗаписатьАтрибут")?;
    with_writer(obj, |w| w.write_attribute(&name, &value))
}

/// `ЗаписатьТекст(Текст)`.
///
/// # Errors
///
/// [`RtError::TypeError`], если аргумент не строка.
pub fn write_text(obj: &dyn ObjectProtocol, args: &[BslValue]) -> RtResult<()> {
    let text = need_str(args.first(), "ЗаписатьТекст")?;
    with_writer(obj, |w| w.write_text(&text))
}

/// `ЗаписатьКомментарий(Текст)`.
///
/// # Errors
///
/// [`RtError::TypeError`], если аргумент не строка.
pub fn write_comment(obj: &dyn ObjectProtocol, args: &[BslValue]) -> RtResult<()> {
    let text = need_str(args.first(), "ЗаписатьКомментарий")?;
    with_writer(obj, |w| w.write_comment(&text))
}

/// `ЗаписатьСекциюCDATA(Текст)`.
///
/// # Errors
///
/// [`RtError::TypeError`], если аргумент не строка.
pub fn write_cdata(obj: &dyn ObjectProtocol, args: &[BslValue]) -> RtResult<()> {
    let text = need_str(args.first(), "ЗаписатьСекциюCDATA")?;
    with_writer(obj, |w| w.write_cdata(&text))
}

/// `ЗаписатьИнструкциюОбработки(Имя, Текст)`.
///
/// # Errors
///
/// [`RtError::TypeError`], если аргумент не строка.
pub fn write_processing_instruction(obj: &dyn ObjectProtocol, args: &[BslValue]) -> RtResult<()> {
    let target = need_str(args.first(), "ЗаписатьИнструкциюОбработки")?;
    let data = need_str(args.get(1), "ЗаписатьИнструкциюОбработки")?;
    with_writer(obj, |w| w.write_processing_instruction(&target, &data))
}

/// `ЗаписатьБезОбработки(Текст)`.
///
/// # Errors
///
/// [`RtError::TypeError`], если аргумент не строка.
pub fn write_raw(obj: &dyn ObjectProtocol, args: &[BslValue]) -> RtResult<()> {
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
pub fn close_writer(obj: &dyn ObjectProtocol) -> RtResult<BslValue> {
    let writer = as_writer(obj)?;
    let mut slot = writer.borrow_mut();
    let Some(w) = slot.as_mut() else {
        return Ok(BslValue::Str(BslString::from_str("")));
    };
    let text = w.finish();
    if let Some(path) = w.take_path() {
        // Файл платформа начинает сигнатурой UTF-8 — измерено побайтным
        // сличением выгрузки `edata_writer` (первые три байта EF BB BF).
        let mut bytes = Vec::with_capacity(3 + text.len());
        bytes.extend_from_slice(b"\xef\xbb\xbf");
        bytes.extend_from_slice(text.as_bytes());
        std::fs::write(&path, bytes).map_err(|e| RtError::IoError(e.to_string()))?;
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
pub fn close_reader(obj: &dyn ObjectProtocol) -> RtResult<BslValue> {
    let reader = as_reader(obj)?;
    *reader.borrow_mut() = XmlReaderState::default();
    Ok(BslValue::Undefined)
}

// --- объектный протокол -----------------------------------------------------

static READER_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ЧтениеXML",
    legacy_type_id: Some(TypeId::XmlReader),
};

static WRITER_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ЗаписьXML",
    legacy_type_id: Some(TypeId::XmlWriter),
};

static SETTINGS_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ПараметрыЗаписиXML",
    legacy_type_id: Some(TypeId::XmlWriterSettings),
};

impl ObjectProtocol for XmlReaderObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &READER_TYPE
    }

    fn get_property(&self, prop: &str, _context: &mut CallContext<'_>) -> RtResult<BslValue> {
        let is =
            |ru: &str, en: &str| prop.eq_ignore_ascii_case(ru) || prop.eq_ignore_ascii_case(en);
        // Читатель помнит текущий узел, а свойства показывают его с разных
        // сторон (прежний арм общей таблицы свойств). Внутренние
        // `*_from`-функции работают прямо над состоянием: свойство
        // читается на каждом узле обхода, и пересборка Rc-обёртки ради
        // обратного даункаста стоила бы аллокации на каждое чтение.
        if is("ТипУзла", "NodeType") {
            node_type_from(&self.state)
        } else if is("Имя", "Name") {
            name_from(&self.state)
        } else if is("Значение", "Value") {
            value_from(&self.state)
        } else if is("ЛокальноеИмя", "LocalName") {
            local_name_from(&self.state)
        } else if is("Префикс", "Prefix") {
            prefix_from(&self.state)
        } else if is("URIПространстваИмен", "NamespaceURI") {
            namespace_uri_from(&self.state)
        } else {
            Err(RtError::UnknownColumn(prop.to_string()))
        }
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        READER_METHODS
    }
}

// Обработчики статических таблиц читателя и писателя XML: получатель
// приходит от вызывающего, пары имён — прежние ветки `eq`-цепочек.
// `УстановитьСтроку` и `ОткрытьФайл` общие: свободные функции сами
// различают читателя и писателя по состоянию получателя.
fn xml_set_string(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    set_string(receiver, arguments)?;
    Ok(BslValue::Undefined)
}

fn xml_open_file(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    open_file(receiver, arguments)?;
    Ok(BslValue::Undefined)
}

fn reader_read(
    receiver: &dyn ObjectProtocol,
    _arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    read(receiver)
}

fn reader_skip(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    // Верхнюю границу арности проверяет получатель: `Пропустить(5)` не
    // должен пройти молча (см. прежний арм).
    if !arguments.is_empty() {
        return Err(RtError::MethodNotApplicable {
            method: "Пропустить",
            receiver: READER_TYPE.name,
        });
    }
    skip(receiver)?;
    Ok(BslValue::Undefined)
}

fn reader_read_attribute(
    receiver: &dyn ObjectProtocol,
    _arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    read_attribute(receiver)
}

fn reader_attribute_count(
    receiver: &dyn ObjectProtocol,
    _arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    attribute_count(receiver)
}

fn reader_attribute_name(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    attribute_name(receiver, arguments)
}

fn reader_attribute_value(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    attribute_value(receiver, arguments)
}

fn reader_move_to_content(
    receiver: &dyn ObjectProtocol,
    _arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    move_to_content(receiver)
}

fn reader_close(
    receiver: &dyn ObjectProtocol,
    _arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    close_reader(receiver)
}

const READER_METHODS: &[MethodDescriptor] = &[
    MethodDescriptor {
        code: MethodCode::new(1),
        names: &["УстановитьСтроку", "SetString"],
        call: xml_set_string,
    },
    MethodDescriptor {
        code: MethodCode::new(2),
        names: &["ОткрытьФайл", "OpenFile"],
        call: xml_open_file,
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
    MethodDescriptor {
        code: MethodCode::new(5),
        names: &["ПрочитатьАтрибут", "ReadAttribute"],
        call: reader_read_attribute,
    },
    MethodDescriptor {
        code: MethodCode::new(6),
        names: &["КоличествоАтрибутов", "AttributeCount"],
        call: reader_attribute_count,
    },
    MethodDescriptor {
        code: MethodCode::new(7),
        names: &["ИмяАтрибута", "AttributeName"],
        call: reader_attribute_name,
    },
    MethodDescriptor {
        code: MethodCode::new(8),
        names: &["ЗначениеАтрибута", "AttributeValue"],
        call: reader_attribute_value,
    },
    MethodDescriptor {
        code: MethodCode::new(9),
        names: &["ПерейтиКСодержимому", "MoveToContent"],
        call: reader_move_to_content,
    },
    MethodDescriptor {
        code: MethodCode::new(10),
        names: &["Закрыть", "Close"],
        call: reader_close,
    },
];

impl ObjectProtocol for XmlWriterObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &WRITER_TYPE
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        WRITER_METHODS
    }
}

fn writer_declaration(
    receiver: &dyn ObjectProtocol,
    _arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    write_declaration(receiver)?;
    Ok(BslValue::Undefined)
}

fn writer_start_element(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    write_start_element(receiver, arguments)?;
    Ok(BslValue::Undefined)
}

fn writer_end_element(
    receiver: &dyn ObjectProtocol,
    _arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    write_end_element(receiver)?;
    Ok(BslValue::Undefined)
}

fn writer_attribute(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    write_attribute(receiver, arguments)?;
    Ok(BslValue::Undefined)
}

fn writer_text(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    write_text(receiver, arguments)?;
    Ok(BslValue::Undefined)
}

fn writer_comment(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    write_comment(receiver, arguments)?;
    Ok(BslValue::Undefined)
}

fn writer_cdata(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    write_cdata(receiver, arguments)?;
    Ok(BslValue::Undefined)
}

fn writer_processing_instruction(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    write_processing_instruction(receiver, arguments)?;
    Ok(BslValue::Undefined)
}

fn writer_raw(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    write_raw(receiver, arguments)?;
    Ok(BslValue::Undefined)
}

fn writer_close(
    receiver: &dyn ObjectProtocol,
    _arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    close_writer(receiver)
}

const WRITER_METHODS: &[MethodDescriptor] = &[
    MethodDescriptor {
        code: MethodCode::new(1),
        names: &["УстановитьСтроку", "SetString"],
        call: xml_set_string,
    },
    MethodDescriptor {
        code: MethodCode::new(2),
        names: &["ОткрытьФайл", "OpenFile"],
        call: xml_open_file,
    },
    MethodDescriptor {
        code: MethodCode::new(3),
        names: &["ЗаписатьОбъявлениеXML", "WriteXMLDeclaration"],
        call: writer_declaration,
    },
    MethodDescriptor {
        code: MethodCode::new(4),
        names: &["ЗаписатьНачалоЭлемента", "WriteStartElement"],
        call: writer_start_element,
    },
    MethodDescriptor {
        code: MethodCode::new(5),
        names: &["ЗаписатьКонецЭлемента", "WriteEndElement"],
        call: writer_end_element,
    },
    MethodDescriptor {
        code: MethodCode::new(6),
        names: &["ЗаписатьАтрибут", "WriteAttribute"],
        call: writer_attribute,
    },
    MethodDescriptor {
        code: MethodCode::new(7),
        names: &["ЗаписатьТекст", "WriteText"],
        call: writer_text,
    },
    MethodDescriptor {
        code: MethodCode::new(8),
        names: &["ЗаписатьКомментарий", "WriteComment"],
        call: writer_comment,
    },
    MethodDescriptor {
        code: MethodCode::new(9),
        names: &["ЗаписатьСекциюCDATA", "WriteCDATASection"],
        call: writer_cdata,
    },
    MethodDescriptor {
        code: MethodCode::new(10),
        names: &["ЗаписатьИнструкциюОбработки", "WriteProcessingInstruction"],
        call: writer_processing_instruction,
    },
    MethodDescriptor {
        code: MethodCode::new(11),
        names: &["ЗаписатьБезОбработки", "WriteRaw"],
        call: writer_raw,
    },
    MethodDescriptor {
        code: MethodCode::new(12),
        names: &["Закрыть", "Close"],
        call: writer_close,
    },
];

impl ObjectProtocol for XmlWriterSettingsObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &SETTINGS_TYPE
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn method_codes_are_static_and_dense() {
        for table in [super::READER_METHODS, super::WRITER_METHODS] {
            let codes = table
                .iter()
                .map(|method| method.code.get())
                .collect::<Vec<_>>();
            assert_eq!(codes, (1..=table.len() as u16).collect::<Vec<_>>());
        }
    }
}
