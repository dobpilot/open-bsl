//! Чтение макета табличного документа из `Template.xml`.
//!
//! Это НЕ тот формат, в котором платформа сохраняет документ методом
//! `Записать`. `Template.xml` — представление макета в выгрузке
//! конфигурации: тот же табличный документ, но разложенный по XML и с
//! другими соглашениями. Отсюда и отдельный модуль: общего кода у него с
//! разбором MXL нет, кроме конечной модели.
//!
//! # Откуда эталон
//!
//! Сочинять `Template.xml` от руки не пришлось: `СериализаторXDTO.ЗаписатьXML`
//! на платформе выдаёт табличный документ ровно в этом формате. Значит и
//! проверять разбор можно её собственным выводом — файл лежит в
//! `tests/conformance/mxl/template-basic.xml`, а породивший его скрипт рядом.
//!
//! Эта сверка сразу поймала три расхождения с описанием формата:
//!
//! * корень называется `SpreadsheetDocument`, а не `document` — принимаются
//!   оба;
//! * имя параметра приходит НЕ элементом `<parameter>`, а пустым
//!   `<v8:lang/>` в локализованной строке, ровно как в MXL;
//! * ширины и высоты в палитре — во ВНУТРЕННИХ единицах, тех же, что в MXL
//!   (восьмые доли знака и четверти пункта), а не в единицах API.
//!
//! Формат описан в спецификации `1c-spreadsheet-spec.md`, и её ключевые
//! утверждения проверялись здесь построчно:
//!
//! * все индексы в документе 0-based, КРОМЕ палитры форматов — она 1-based,
//!   и индекс 0 значит «формат не задан»;
//! * `<i>` у ячейки необязателен: без него колонка равна предыдущей плюс
//!   один, а первая ячейка без `<i>` идёт в колонку 0;
//! * `<indexTo>` у строки задаёт ДИАПАЗОН строк с одинаковым содержимым;
//! * тип заполнения ячейки живёт не в ней, а в её формате: `Parameter` —
//!   значение подставляется программно, `Template` — текст с `[Имя]`,
//!   `Text` — статический текст;
//! * `<merge>` с `<r>-1</r>` — объединение по всем строкам набора колонок,
//!   а `<verticalUnmerge>` его в отдельных строках разрывает.
//!
//! Чего здесь НЕТ и не подразумевается: рисунков, картинок, дополнительных
//! наборов колонок с их UUID. Наборы читаются, но строки, привязанные к
//! неосновному набору, кладутся в ту же сетку — своей модели у них пока
//! нет, и это лучше, чем потерять их молча.

use crate::spreadsheet::{AreaKind, HAlign, Merge, NamedArea, SpreadDocData, VAlign};
use crate::xml::{XmlEvent, XmlParser};
use crate::{RtError, RtResult};

fn bad(what: impl Into<String>) -> RtError {
    RtError::Spread(what.into())
}

/// Элемент палитры форматов. Значащие свойства зависят от того, кто на него
/// ссылается: у колонки — ширина, у строки — высота, у ячейки — всё
/// остальное.
#[derive(Debug, Default, Clone)]
struct Format {
    width: Option<i64>,
    height: Option<i64>,
    h_align: Option<HAlign>,
    v_align: Option<VAlign>,
    wrap: bool,
    fill: Option<String>,
    /// Строка формата BSL из вложенного `<format>`: «ЧДЦ=2» и т. п.
    /// Берётся из первой локализации (`v8:content`), как и текст ячейки.
    number_format: Option<String>,
}

/// Один разобранный элемент дерева: имя, текст и дети. `Template.xml`
/// невелик и читается целиком в память — так разбор описывается прямо по
/// спецификации, разделами, а не машиной состояний.
#[derive(Debug, Default)]
struct El {
    name: String,
    text: String,
    children: Vec<El>,
}

impl El {
    fn child(&self, name: &str) -> Option<&El> {
        self.children.iter().find(|c| c.name == name)
    }

    fn text_of(&self, name: &str) -> Option<&str> {
        self.child(name).map(|c| c.text.trim())
    }

    fn number(&self, name: &str) -> Option<i64> {
        self.text_of(name).and_then(|t| t.parse().ok())
    }

    fn children_named<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a El> {
        self.children.iter().filter(move |c| c.name == name)
    }

    /// Локализованная строка: `<tl><v8:item><v8:lang/><v8:content>…`.
    ///
    /// Возвращает пару «содержимое, это параметр». Язык здесь не про язык:
    /// ПУСТОЙ `<v8:lang/>` означает, что в ячейке имя параметра, а не текст
    /// — ровно то же соглашение, что и в MXL (измерено на выводе платформы,
    /// где `Обл.Параметр = "Номенклатура"` дал именно пустой lang).
    fn localized(&self) -> (String, bool) {
        for item in &self.children {
            if let Some(c) = item.child("content") {
                return (
                    c.text.clone(),
                    item.text_of("lang").unwrap_or("").is_empty(),
                );
            }
            let (text, parameter) = item.localized();
            if !text.is_empty() {
                return (text, parameter);
            }
        }
        (String::new(), false)
    }

    /// Первое `v8:content` из локализованной строки — для строки формата
    /// числа, где язык не важен (берётся русское написание).
    fn first_content(&self) -> Option<&str> {
        for item in &self.children {
            if let Some(c) = item.child("content") {
                return Some(c.text.trim());
            }
            if let Some(s) = item.first_content() {
                return Some(s);
            }
        }
        None
    }
}

/// Разбор XML в дерево. Пространства имён отбрасываются: в этом формате имя
/// элемента однозначно и без префикса.
fn parse_tree(text: &str) -> RtResult<El> {
    let mut parser = XmlParser::new(text);
    let mut stack: Vec<El> = vec![El::default()];
    while let Some(event) = parser.read()? {
        match event {
            XmlEvent::ElementStart { name, .. } => stack.push(El {
                // Префикс пространства имён отбрасывается: в этом формате
                // локального имени достаточно, а `v8:content` и `content` —
                // одно и то же поле.
                name: name.rsplit(':').next().unwrap_or(&name).to_string(),
                ..El::default()
            }),
            XmlEvent::ElementEnd { .. } => {
                let done = stack.pop().ok_or_else(|| bad("лишний закрывающий тег"))?;
                stack
                    .last_mut()
                    .ok_or_else(|| bad("лишний закрывающий тег"))?
                    .children
                    .push(done);
            }
            // Комментарии сюда не приходят: разборщик отдаёт их только с
            // включённым `set_report_comments`, а макет читается обычным
            // способом.
            XmlEvent::Comment(_) => {}
            XmlEvent::Text(t) => {
                if let Some(top) = stack.last_mut() {
                    top.text.push_str(&t);
                }
            }
            XmlEvent::ProcessingInstruction { .. } => {}
            // Внутренний формат макета сущностей не объявляет, поэтому
            // ссылка на них здесь — испорченный файл, а не потеря данных.
            XmlEvent::EntityReference { name } => {
                return Err(bad(format!("ссылка на сущность «&{name};» в макете")))
            }
        }
    }
    let root = stack.pop().ok_or_else(|| bad("пустой документ"))?;
    // Корень называется по-разному: `document` в выгрузке конфигурации и
    // `SpreadsheetDocument` в сериализации через `СериализаторXDTO`.
    // Содержимое при этом одно и то же (сверено с выводом платформы).
    root.children
        .into_iter()
        .find(|c| c.name == "document" || c.name == "SpreadsheetDocument")
        .ok_or_else(|| bad("нет корневого элемента макета"))
}

fn h_align(s: &str) -> Option<HAlign> {
    match s {
        "Left" => Some(HAlign::Left),
        "Center" => Some(HAlign::Center),
        "Right" => Some(HAlign::Right),
        "Auto" => Some(HAlign::Auto),
        _ => None,
    }
}

fn v_align(s: &str) -> Option<VAlign> {
    match s {
        "Top" => Some(VAlign::Top),
        "Center" => Some(VAlign::Center),
        "Bottom" => Some(VAlign::Bottom),
        _ => None,
    }
}

/// Прочитать макет.
///
/// # Errors
///
/// Возвращает ошибку, если это не XML, если нет корневого `<document>` или
/// если разметка оборвана.
pub fn from_template_xml(text: &str) -> RtResult<SpreadDocData> {
    // Платформа пишет XML с BOM и без объявления; для разборщика BOM — это
    // текст перед корневым элементом, то есть ошибка. Снимаем его здесь, а
    // не в разборщике: он общий и обязан оставаться строгим.
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let root = parse_tree(text)?;
    let mut doc = SpreadDocData::new();

    // --- палитра форматов, 1-based ------------------------------------
    let formats: Vec<Format> = root
        .children_named("format")
        .map(|f| Format {
            // Единицы ВНУТРЕННИЕ, как в MXL: ширина в восьмых долях знака,
            // высота в четвертях пункта. Проверено на выводе платформы, где
            // `ШиринаКолонки = 30` дала 240, а `ВысотаСтроки = 25` — 100.
            width: f.number("width").map(|w| w / 8),
            height: f.number("height").map(|h| h / 4),
            h_align: f.text_of("horizontalAlignment").and_then(h_align),
            v_align: f.text_of("verticalAlignment").and_then(v_align),
            wrap: f.text_of("textPlacement") == Some("Wrap"),
            fill: f.text_of("fillType").map(str::to_string),
            number_format: f
                .child("format")
                .and_then(El::first_content)
                .map(str::to_string),
        })
        .collect();
    let format = |index: Option<i64>| -> Option<&Format> {
        formats.get(usize::try_from(index?).ok()?.checked_sub(1)?)
    };

    // --- колонки -------------------------------------------------------
    // Основной набор — первый `<columns>` без `<id>`; дополнительные имеют
    // UUID и своей сетки у нас нет (см. модуль).
    if let Some(set) = root
        .children_named("columns")
        .find(|c| c.child("id").is_none())
    {
        for item in set.children_named("columnsItem") {
            let index = item.number("index").unwrap_or(0).max(0) as u32;
            if let Some(width) = item
                .child("column")
                .and_then(|c| format(c.number("formatIndex")))
                .and_then(|f| f.width)
            {
                doc.set_col_width(index, width);
            }
        }
    }

    // --- строки и ячейки -----------------------------------------------
    for item in root.children_named("rowsItem") {
        let from_row = item.number("index").unwrap_or(0).max(0) as u32;
        let to_row = item.number("indexTo").map_or(from_row, |v| v.max(0) as u32);
        let Some(row) = item.child("row") else {
            continue;
        };
        let height = format(row.number("formatIndex")).and_then(|f| f.height);
        // Диапазон `index..indexTo` — строки с ОДИНАКОВЫМ содержимым.
        for r in from_row..=to_row.max(from_row) {
            if let Some(h) = height {
                doc.set_row_height(r, h);
            }
            let mut column: Option<u32> = None;
            for group in row.children_named("c") {
                // `<i>` необязателен: без него колонка равна предыдущей + 1.
                let current = match group.number("i") {
                    Some(i) => i.max(0) as u32,
                    None => column.map_or(0, |c| c + 1),
                };
                column = Some(current);
                let Some(content) = group.child("c") else {
                    continue;
                };
                let fmt = format(content.number("f"));
                // Имя параметра приходит двумя путями: отдельным элементом
                // `<parameter>` (так описано в спецификации) либо пустым
                // языком в локализованной строке (так пишет платформа).
                let parameter = content.text_of("parameter").unwrap_or("").to_string();
                let (text, by_lang) = content.child("tl").map(El::localized).unwrap_or_default();
                let by_format = fmt.and_then(|f| f.fill.as_deref()) == Some("Parameter");
                if !parameter.is_empty() {
                    doc.set_cell_parameter(r, current, &parameter);
                } else if !text.is_empty() && (by_lang || by_format) {
                    doc.set_cell_parameter(r, current, &text);
                } else if !text.is_empty() {
                    doc.set_cell_text(r, current, &text);
                }
                if let Some(f) = fmt {
                    if let Some(a) = f.h_align {
                        doc.set_cell_h_align(r, current, a);
                    }
                    if let Some(a) = f.v_align {
                        doc.set_cell_v_align(r, current, a);
                    }
                    if f.wrap {
                        doc.set_cell_wrap(r, current, true);
                    }
                    if let Some(ref spec) = f.number_format {
                        doc.set_cell_format_spec(r, current, spec);
                        doc.set_cell_numeric(r, current);
                    }
                }
            }
        }
    }

    // --- объединения ----------------------------------------------------
    for m in root.children_named("merge") {
        let r = m.number("r").unwrap_or(0);
        let c = m.number("c").unwrap_or(0).max(0) as u32;
        // Размер — «добавочные» строки и колонки, а не границы: `w` это
        // сколько ЕЩЁ колонок войдёт (спецификация, раздел «Объединения»).
        let w = m.number("w").unwrap_or(0).max(0) as u32;
        let h = m.number("h").unwrap_or(0).max(0) as u32;
        if r < 0 {
            // `-1` — объединение по всем строкам набора колонок.
            doc.merge_columns(c, c + w);
        } else {
            let r = r as u32;
            doc.merge(Merge::new(r, c, r + h, c + w));
        }
    }
    for u in root.children_named("verticalUnmerge") {
        let r = u.number("r").unwrap_or(0).max(0) as u32;
        let c = u.number("c").unwrap_or(0).max(0) as u32;
        let w = u.number("w").unwrap_or(0).max(0) as u32;
        doc.unmerge_cells(r, c, r, c + w);
    }

    // --- именованные области ---------------------------------------------
    for item in root.children_named("namedItem") {
        let Some(name) = item.text_of("name") else {
            continue;
        };
        // `NamedItemDrawing` — именованный рисунок; рисунков у нас нет.
        let Some(area) = item.child("area") else {
            continue;
        };
        let (r1, r2) = (
            area.number("beginRow").unwrap_or(-1),
            area.number("endRow").unwrap_or(-1),
        );
        let (c1, c2) = (
            area.number("beginColumn").unwrap_or(-1),
            area.number("endColumn").unwrap_or(-1),
        );
        let kind = match area.text_of("type") {
            Some("Rows") => AreaKind::Rows,
            Some("Columns") => AreaKind::Columns,
            _ => AreaKind::Rect,
        };
        let area = match kind {
            AreaKind::Rows => NamedArea::rows(r1.max(0) as u32, r2.max(0) as u32),
            AreaKind::Columns => NamedArea::columns(c1.max(0) as u32, c2.max(0) as u32),
            AreaKind::Rect => NamedArea::rect(
                r1.max(0) as u32,
                c1.max(0) as u32,
                r2.max(0) as u32,
                c2.max(0) as u32,
            ),
        };
        doc.set_area_name(name, area);
    }

    // `<height>` — общее число строк документа; ширина берётся из размера
    // основного набора колонок.
    if let Some(h) = root.number("height") {
        doc.set_height(h.max(0) as u32);
    }
    if let Some(w) = root
        .children_named("columns")
        .find(|c| c.child("id").is_none())
        .and_then(|c| c.number("size"))
    {
        doc.set_width(w.max(0) as u32);
    }
    Ok(doc)
}
