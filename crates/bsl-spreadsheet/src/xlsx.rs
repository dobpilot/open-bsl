//! Экспорт табличного документа в XLSX.
//!
//! Пишется НЕ то, что пишет платформа: её файл тянет за собой рисунки,
//! примечания, колонтитульные VML и стили — четырнадцать частей на документ
//! из двух ячеек. Здесь собирается минимальный пакет OOXML из шести частей,
//! который читается обратно самой 1С (`ТабличныйДокумент.Прочитать` умеет
//! xlsx — измерено).
//!
//! Минимальный он не «на глаз»: состав выяснен двусторонним опытом. Файл без
//! `xl/styles.xml` платформа отвергает с «Формат файла не поддерживается» —
//! и наш, и её собственный, из которого стили вынули. А `<dimension>`,
//! `<sheetViews>` и BOM в частях, наоборот, ни на что не влияют.
//!
//! Строки кладутся в `sharedStrings`, а не в `inlineStr`: так делает сама
//! платформа, значит её читатель этот путь заведомо поддерживает.
//!
//! Объединения всех ТРЁХ видов MXL сводятся здесь к одному: «на всю
//! таблицу» разворачивается по границам документа. Подробности — у
//! [`merge_rects`].

use std::collections::BTreeMap;
use std::fmt::Write;

use crate::document::{CellData, Color, ColumnSet, HAlign, SpreadDocData, VAlign};

/// Сборщик пакета — тонкая обёртка над `zip::ZipWriter`. Выбирать способ
/// хранения незачем: сжатие всегда deflate, для мелких частей XLSX (стили,
/// строки) раздувание пренебрежимо. Прежде обёртка жила в модуле архивов;
/// после его выноса в `bsl-zip` единственный потребитель — этот экспорт.
struct ZipWriter {
    inner: zip::ZipWriter<std::io::Cursor<Vec<u8>>>,
}

impl ZipWriter {
    fn new() -> Self {
        ZipWriter {
            inner: zip::ZipWriter::new(std::io::Cursor::new(Vec::new())),
        }
    }

    /// Добавить файл. Имя — с прямыми слэшами и без ведущего слэша, как
    /// требует формат.
    fn add(&mut self, name: &str, data: &[u8]) {
        use std::io::Write as _;
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .compression_level(Some(1))
            .last_modified_time(zip::DateTime::default());
        self.inner
            .start_file(name, options)
            .expect("zip start_file не отказывает на корректном имени");
        self.inner
            .write_all(data)
            .expect("zip write_all в память не отказывает");
    }

    /// Закрыть архив и отдать его байты.
    fn finish(self) -> Vec<u8> {
        let cursor = self.inner.finish().expect("zip finish не отказывает");
        cursor.into_inner()
    }
}

/// Экранирование для XML. Апостроф и кавычка не трогаются: они попадают
/// только в текст узлов, а не в атрибуты, которые мы формируем сами.
///
/// Табуляция превращается в ЧЕТЫРЕ ПРОБЕЛА — так делает платформа: в тексте
/// отбора отчёта было четыре табуляции, и в её выгрузке на их месте ровно
/// по четыре пробела.
fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '\t' => out.push_str("    "),
            _ => out.push(ch),
        }
    }
    out
}

/// Имя колонки в нотации A1: 0 -> `A`, 25 -> `Z`, 26 -> `AA`.
fn column_name(mut col: u32) -> String {
    let mut out = Vec::new();
    loop {
        out.push(b'A' + (col % 26) as u8);
        if col < 26 {
            break;
        }
        col = col / 26 - 1;
    }
    out.reverse();
    String::from_utf8(out).expect("буквы латиницы")
}

fn cell_ref(row: u32, col: u32) -> String {
    format!("{}{}", column_name(col), row + 1)
}

const XML_HEADER: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n";

fn content_types() -> String {
    format!(
        "{XML_HEADER}<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\">\
<Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/>\
<Default Extension=\"xml\" ContentType=\"application/xml\"/>\
<Override PartName=\"/xl/workbook.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml\"/>\
<Override PartName=\"/xl/worksheets/sheet1.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml\"/>\
<Override PartName=\"/xl/sharedStrings.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.sharedStrings+xml\"/>\
<Override PartName=\"/xl/styles.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml\"/>\
</Types>"
    )
}

fn root_rels() -> String {
    format!(
        "{XML_HEADER}<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">\
<Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument\" Target=\"xl/workbook.xml\"/>\
</Relationships>"
    )
}

fn workbook() -> String {
    format!(
        "{XML_HEADER}<workbook xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\" \
xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\">\
<sheets><sheet name=\"Лист1\" sheetId=\"1\" r:id=\"rId1\"/></sheets></workbook>"
    )
}

fn workbook_rels() -> String {
    format!(
        "{XML_HEADER}<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">\
<Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet\" Target=\"worksheets/sheet1.xml\"/>\
<Relationship Id=\"rId2\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/sharedStrings\" Target=\"sharedStrings.xml\"/>\
<Relationship Id=\"rId3\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles\" Target=\"styles.xml\"/>\
</Relationships>"
    )
}

/// Оформление одной ячейки в терминах OOXML: ссылки на шрифт, заливку и
/// рамку плюс выравнивание. Ровно то, что платформа складывает в `<xf>`.
#[derive(Debug, Default, PartialEq, Eq, Clone)]
struct CellStyle {
    font: usize,
    fill: usize,
    border: usize,
    h_align: &'static str,
    v_align: &'static str,
    wrap: bool,
}

/// Шрифт для книги. Цвет текста в OOXML живёт ВНУТРИ шрифта, а не рядом с
/// ним, — так же, как у платформы.
#[derive(Debug, PartialEq, Eq, Clone)]
struct XlsxFont {
    face: String,
    size: i64,
    bold: bool,
    italic: bool,
    underline: bool,
    strikeout: bool,
    color: Option<String>,
}

impl Default for XlsxFont {
    fn default() -> Self {
        // Умолчание платформы: Arial восьмого кегля.
        XlsxFont {
            face: "Arial".to_string(),
            size: 8,
            bold: false,
            italic: false,
            underline: false,
            strikeout: false,
            color: None,
        }
    }
}

/// Цвет в OOXML пишется как `RRGGBB` — в ПРЯМОМ порядке, в отличие от MXL,
/// где он упакован задом наперёд.
fn rgb(c: Color) -> String {
    format!("{:02X}{:02X}{:02X}", c.r, c.g, c.b)
}

/// Собранные палитры книги.
#[derive(Default)]
struct StyleBook {
    fonts: Vec<XlsxFont>,
    /// Заливки: цвет фона; два первых места заняты обязательными `none` и
    /// `gray125` — их ждёт Excel, и платформа пишет их же.
    fills: Vec<Color>,
    /// Рамки: наличие каждой из четырёх сторон.
    borders: Vec<[bool; 4]>,
    styles: Vec<CellStyle>,
    /// Номер стиля ячейки БЕЗ оформления — он один на весь документ.
    plain: Option<usize>,
}

impl StyleBook {
    /// Номер стиля ячейки. Ноль зарезервирован под «без оформления», как и
    /// у платформы: её ячейки начинаются с первого.
    fn intern(&mut self, cell: &CellData) -> usize {
        // Быстрый путь: у ячейки без оформления вообще нечего искать, а
        // таких в отчёте подавляющее большинство.
        if cell.font.is_none()
            && cell.text_color.is_none()
            && cell.back_color.is_none()
            && cell.h_align.is_none()
            && cell.v_align.is_none()
            && !cell.wrap
            && cell.border_left.is_none()
            && cell.border_top.is_none()
            && cell.border_right.is_none()
            && cell.border_bottom.is_none()
            && let Some(ready) = self.plain
        {
            return ready;
        }
        let mut font = XlsxFont::default();
        if let Some(f) = &cell.font {
            font.face = f.face.clone();
            font.size = f.size;
            font.bold = f.bold;
            font.italic = f.italic;
            font.underline = f.underline;
            font.strikeout = f.strikeout;
        }
        if let Some(c) = cell.text_color {
            font.color = Some(rgb(c));
        }
        let font = match self.fonts.iter().position(|f| *f == font) {
            Some(i) => i,
            None => {
                self.fonts.push(font);
                self.fonts.len() - 1
            }
        };
        let fill = cell
            .back_color
            .map_or(0, |c| match self.fills.iter().position(|f| *f == c) {
                Some(i) => i + 2,
                None => {
                    self.fills.push(c);
                    self.fills.len() + 1
                }
            });
        let border = [
            cell.border_left.is_some(),
            cell.border_right.is_some(),
            cell.border_top.is_some(),
            cell.border_bottom.is_some(),
        ];
        let border = if border == [false; 4] {
            0
        } else {
            match self.borders.iter().position(|b| *b == border) {
                Some(i) => i + 1,
                None => {
                    self.borders.push(border);
                    self.borders.len()
                }
            }
        };
        let style = CellStyle {
            font,
            fill,
            border,
            h_align: match cell.h_align {
                Some(HAlign::Center) => "center",
                Some(HAlign::Right) => "right",
                // `Авто` и `Лево` платформа пишет одинаково — `left`.
                _ => "left",
            },
            v_align: match cell.v_align {
                Some(VAlign::Center) => "center",
                Some(VAlign::Bottom) => "bottom",
                Some(VAlign::Top) => "top",
                None => "",
            },
            wrap: cell.wrap,
        };
        let number_of = match self.styles.iter().position(|s| *s == style) {
            Some(i) => i + 1,
            None => {
                self.styles.push(style);
                self.styles.len()
            }
        };
        if font == 0 && fill == 0 && border == 0 && self.plain.is_none() {
            self.plain = Some(number_of);
        }
        number_of
    }
}

/// Таблица стилей — по образцу той, что пишет платформа.
///
/// БЕЗ ЭТОЙ ЧАСТИ платформа файл не открывает вовсе: `Прочитать` отвечает
/// «Формат файла не поддерживается». Проверено с двух сторон — наш файл со
/// стилями читается, а файл САМОЙ 1С, из которого стили вынули, перестаёт.
///
/// Устройство снято с её вывода: цвет текста лежит внутри `<font>`, фон —
/// отдельной заливкой `solid` с `fgColor`, рамка — набором сторон со
/// стилем `thin`, а выравнивание — прямо в `<xf>`. Первые две заливки
/// (`none` и `gray125`) обязательны и есть даже в пустой книге.
fn styles(book: &StyleBook) -> String {
    let mut out = String::from(XML_HEADER);
    out.push_str(
        "<styleSheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\">",
    );

    out.push_str(&format!("<fonts count=\"{}\">", book.fonts.len().max(1)));
    let fonts_test = if book.fonts.is_empty() {
        vec![XlsxFont::default()]
    } else {
        book.fonts.clone()
    };
    for f in &fonts_test {
        out.push_str(&format!(
            "<font><name val=\"{}\"/><charset val=\"0\"/><family val=\"2\"/>\
<b val=\"{}\"/><i val=\"{}\"/><strike val=\"{}\"/>",
            escape(&f.face),
            f.bold,
            f.italic,
            f.strikeout
        ));
        if let Some(c) = &f.color {
            out.push_str(&format!("<color rgb=\"{c}\"/>"));
        }
        out.push_str(&format!(
            "<sz val=\"{}\"/><u val=\"{}\"/></font>",
            f.size,
            if f.underline { "single" } else { "none" }
        ));
    }
    out.push_str("</fonts>");

    out.push_str(&format!("<fills count=\"{}\">", book.fills.len() + 2));
    out.push_str("<fill><patternFill patternType=\"none\"/></fill>");
    out.push_str("<fill><patternFill patternType=\"gray125\"/></fill>");
    for c in &book.fills {
        out.push_str(&format!(
            "<fill><patternFill patternType=\"solid\"><fgColor rgb=\"{}\"/>\
<bgColor auto=\"true\"/></patternFill></fill>",
            rgb(*c)
        ));
    }
    out.push_str("</fills>");

    out.push_str(&format!("<borders count=\"{}\">", book.borders.len() + 1));
    out.push_str("<border><left/><right/><top/><bottom/><diagonal/></border>");
    for b in &book.borders {
        out.push_str("<border>");
        for (present, name) in b.iter().zip(["left", "right", "top", "bottom"]) {
            if *present {
                out.push_str(&format!(
                    "<{name} style=\"thin\"><color rgb=\"000000\"/></{name}>"
                ));
            } else {
                out.push_str(&format!("<{name}/>"));
            }
        }
        out.push_str("<diagonal/></border>");
    }
    out.push_str("</borders>");

    out.push_str(
        "<cellStyleXfs count=\"1\"><xf numFmtId=\"0\" fontId=\"0\" fillId=\"0\" borderId=\"0\"/></cellStyleXfs>",
    );
    out.push_str(&format!("<cellXfs count=\"{}\">", book.styles.len() + 1));
    out.push_str("<xf numFmtId=\"0\" fontId=\"0\"/>");
    for s in &book.styles {
        out.push_str(&format!("<xf numFmtId=\"0\" fontId=\"{}\"", s.font));
        if s.fill > 0 {
            out.push_str(&format!(" fillId=\"{}\"", s.fill));
        }
        if s.border > 0 {
            out.push_str(&format!(" borderId=\"{}\"", s.border));
        }
        if s.font > 0 {
            out.push_str(" applyFont=\"true\"");
        }
        if s.fill > 0 {
            out.push_str(" applyFill=\"true\"");
        }
        if s.border > 0 {
            out.push_str(" applyBorder=\"true\"");
        }
        out.push_str(" applyAlignment=\"true\"><alignment");
        out.push_str(&format!(" horizontal=\"{}\"", s.h_align));
        if !s.v_align.is_empty() {
            out.push_str(&format!(" vertical=\"{}\"", s.v_align));
        }
        if s.wrap {
            out.push_str(" wrapText=\"1\"");
        }
        out.push_str("/></xf>");
    }
    out.push_str("</cellXfs>");
    out.push_str("<cellStyles count=\"1\"><cellStyle name=\"Обычный\" xfId=\"0\" builtinId=\"0\"/></cellStyles>");
    out.push_str("<dxfs count=\"0\"/>");
    out.push_str("</styleSheet>");
    out
}

/// Таблица общих строк XLSX (`sharedStrings.xml`).
///
/// Индекс по `HashMap` делает дедупликацию O(1): на 100 000 строк
/// накладной с линейным `Vec::iter().position` уходило 55 % времени
/// выгрузки. Вектор хранит строки в порядке интернирования — он же
/// порядок индексов, которые пишутся в `<v>` ячеек.
struct SharedStrings {
    strings: Vec<String>,
    index: std::collections::HashMap<String, usize>,
}

impl SharedStrings {
    fn new() -> Self {
        Self {
            strings: Vec::new(),
            index: std::collections::HashMap::new(),
        }
    }

    /// Возвращает индекс строки в таблице, добавляя её при первом
    /// появлении. Повторное обращение к той же строке — хеш-поиск.
    fn intern(&mut self, text: String) -> usize {
        if let Some(&existing) = self.index.get(&text) {
            return existing;
        }
        let i = self.strings.len();
        self.index.insert(text.clone(), i);
        self.strings.push(text);
        i
    }
}

fn shared_strings(strings: &[String]) -> String {
    let mut out = format!(
        "{XML_HEADER}<sst xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\" count=\"{0}\" uniqueCount=\"{0}\">",
        strings.len()
    );
    for s in strings {
        // `xml:space="preserve"` обязателен: без него ведущие и хвостовые
        // пробелы ячейки теряются при чтении.
        out.push_str(&format!(
            "<si><t xml:space=\"preserve\">{}</t></si>",
            escape(s)
        ));
    }
    out.push_str("</sst>");
    out
}

/// Объединения в виде прямоугольников.
///
/// В MXL их ТРИ вида: обычные прямоугольники, объединения целых строк и
/// целых колонок. В XLSX вид один, поэтому «на всю таблицу» разворачивается
/// по границам документа: строка — во всю его ширину, колонка — во всю
/// высоту.
///
/// Отмены (`Разъединить` поверх объединения колонок) не просто
/// отбрасываются: колоночное объединение РЕЖЕТСЯ на куски, обходящие
/// отменённые строки. Иначе в книге оказалось бы объединение, которого в
/// исходном документе нет.
///
/// Excel считает пересекающиеся объединения повреждением файла, а
/// вырожденное `A1:A1` — ошибкой, поэтому и то и другое отсеивается.
fn merge_rects(doc: &SpreadDocData, layout: &Layout) -> Vec<(u32, u32, u32, u32)> {
    let (height, width) = (doc.height(), doc.width());
    if height == 0 || width == 0 {
        return Vec::new();
    }
    let (last_row, last_column) = (height - 1, width - 1);
    let mut rects: Vec<(u32, u32, u32, u32)> = Vec::new();

    for m in doc.merges().iter().filter(|m| !m.unmerge) {
        // Объединение задано в ЛОГИЧЕСКИХ колонках набора, к которому
        // привязана строка, и на физическую сетку его надо переводить так
        // же, как ячейки. Без этого объединение шапки отчёта оказывалось
        // втрое уже, чем у платформы.
        let (first, _) = layout.span(m.r1, m.c1);
        let (_, last) = layout.span(m.r1, m.c2);
        rects.push((m.r1, first, m.r2, last.max(first)));
    }
    for &(r1, r2) in doc.row_merges() {
        rects.push((r1, 0, r2.min(last_row), last_column));
    }
    for &(c1, c2) in doc.col_merges() {
        // Строки, где это объединение отменено.
        let cancelled = |r: u32| {
            doc.merges()
                .iter()
                .any(|m| m.unmerge && r >= m.r1 && r <= m.r2 && m.c1 <= c1 && m.c2 >= c2)
        };
        let mut start = None;
        for r in 0..=last_row {
            match (cancelled(r), start) {
                (false, None) => start = Some(r),
                (true, Some(chunk_start)) => {
                    rects.push((chunk_start, c1, r - 1, c2.min(last_column)));
                    start = None;
                }
                _ => {}
            }
        }
        if let Some(chunk_start) = start {
            rects.push((chunk_start, c1, last_row, c2.min(last_column)));
        }
    }

    // Проверка пересечений идёт ПО СТРОКАМ: полный перебор на ста тысячах
    // объединений — это пять миллиардов сравнений, а объединения почти все
    // однострочные, и по строке их единицы.
    let mut kept: Vec<(u32, u32, u32, u32)> = Vec::new();
    let mut by_row: BTreeMap<u32, Vec<(u32, u32)>> = BTreeMap::new();
    for (r1, c1, r2, c2) in rects {
        if r1 > r2 || c1 > c2 || (r1 == r2 && c1 == c2) {
            continue;
        }
        let overlaps = (r1..=r2).any(|r| {
            by_row
                .get(&r)
                .is_some_and(|list| list.iter().any(|&(b1, b2)| c1 <= b2 && b1 <= c2))
        });
        if !overlaps {
            for r in r1..=r2 {
                by_row.entry(r).or_default().push((c1, c2));
            }
            kept.push((r1, c1, r2, c2));
        }
    }
    kept
}

/// Похоже ли содержимое ячейки на ЧИСЛО в записи 1С — и если да, то на
/// какое.
///
/// Платформа при выгрузке в XLSX превращает такой текст в настоящее число:
/// «1 000,01» уходит в книгу как `1000.01` с типом `n`, а не строкой
/// (измерено на её собственной выгрузке реального отчёта — 22 ячейки из
/// 111). Без этого в Excel по колонке не посчитать сумму.
///
/// Разбор нарочно узкий: разделитель групп — пробел или неразрывный пробел,
/// дробная часть отделяется ЗАПЯТОЙ. Точка не принимается, иначе датой
/// «01.01.2020» подавилось бы всё остальное.
fn as_number(text: &str) -> Option<String> {
    let ungrouped: String = text
        .chars()
        .filter(|c| *c != ' ' && *c != '\u{a0}')
        .collect();
    let (sign, body) = match ungrouped.strip_prefix('-') {
        Some(t) => ("-", t),
        None => ("", ungrouped.as_str()),
    };
    if body.is_empty() {
        return None;
    }
    let (integer, fraction) = match body.split_once(',') {
        Some((a, b)) => (a, Some(b)),
        None => (body, None),
    };
    if celaya_bad(integer) || fraction.is_some_and(celaya_bad) {
        return None;
    }
    // Хвостовые нули дробной части платформа убирает: «736,80» уходит как
    // `736.8`, а «498,00» — как `498` (измерено на её выгрузке).
    let fraction = fraction
        .map(|d| d.trim_end_matches('0'))
        .filter(|d| !d.is_empty());
    Some(match fraction {
        Some(d) => format!("{sign}{integer}.{d}"),
        None => format!("{sign}{integer}"),
    })
}

fn celaya_bad(s: &str) -> bool {
    s.is_empty() || !s.chars().all(|c| c.is_ascii_digit())
}

/// Раскладка логической ячейки на ФИЗИЧЕСКУЮ сетку.
///
/// Когда строки документа разложены по разным наборам колонок, «вторая
/// колонка» в шапке и в табличной части — это разные места. Платформа
/// сводит их в одну сетку по объединению границ, и логическая ячейка
/// занимает несколько физических колонок; отсюда и объединения в её
/// выгрузке. Здесь то же самое.
///
/// Возвращает `(первая физическая колонка, последняя)`. Без наборов сетка
/// совпадает с логической, и ячейка занимает ровно одну колонку.
struct Layout {
    grid: Vec<i64>,
    /// Границы каждого набора — считаются один раз, а не на ячейку.
    bounds: Vec<Vec<i64>>,
    /// Строка -> номер набора.
    strings: std::collections::HashMap<u32, usize>,
}

impl Layout {
    /// Раскладка считается ОДИН раз на документ.
    ///
    /// Раньше и сетка, и границы набора, и поиск привязки строки делались
    /// на каждую ячейку — на полумиллионе ячеек и полусотне тысяч привязок
    /// это давало почти всё время выгрузки.
    fn new(doc: &SpreadDocData) -> Self {
        Layout {
            grid: doc.physical_grid(),
            bounds: doc
                .column_sets()
                .iter()
                .map(ColumnSet::boundaries)
                .collect(),
            strings: doc
                .row_bindings()
                .iter()
                .map(|&(r, set_no)| (r, set_no as usize))
                .collect(),
        }
    }

    fn span(&self, row: u32, col: u32) -> (u32, u32) {
        if self.grid.is_empty() {
            return (col, col);
        }
        let Some(bounds) = self
            .strings
            .get(&row)
            .and_then(|&set_no| self.bounds.get(set_no))
        else {
            return (col, col);
        };
        let (Some(&left), Some(&right)) = (bounds.get(col as usize), bounds.get(col as usize + 1))
        else {
            return (col, col);
        };
        let first = self.grid.partition_point(|&x| x < left) as u32;
        let last = (self.grid.partition_point(|&x| x < right) as u32).saturating_sub(1);
        (first, last.max(first))
    }
}

fn worksheet(
    doc: &SpreadDocData,
    strings: &mut SharedStrings,
    book: &mut StyleBook,
    layout: &Layout,
) -> String {
    let mut out = String::from(XML_HEADER);
    out.push_str("<worksheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\">");

    // Ширины колонок. В XLSX они в знаках, как и в API табличного документа,
    // поэтому переводить ничего не нужно.
    let widths = doc.col_widths();
    if !widths.is_empty() {
        out.push_str("<cols>");
        for (&c, &w) in widths {
            out.push_str(&format!(
                "<col min=\"{0}\" max=\"{0}\" width=\"{w}\" customWidth=\"1\"/>",
                c + 1
            ));
        }
        out.push_str("</cols>");
    }

    out.push_str("<sheetData>");
    // Объединения, которые рождает сама раскладка: логическая ячейка шире
    // одной физической колонки.
    let mut spans: Vec<(u32, u32, u32)> = Vec::new();
    for (&r, row) in doc.rows() {
        // Что попадает в книгу: ПРЕДСТАВЛЕНИЕ ячейки. У ячейки со значением
        // это его печатный вид, у параметрической — имя параметра. Так
        // делает и платформа: число 1000,5 уходит в `sharedStrings` строкой
        // «1 000,5», а не числом (измерено на её собственном экспорте).
        let cells: Vec<(u32, String, usize, bool)> = row
            .cells
            .iter()
            .filter_map(|(&c, cell)| {
                let text = cell
                    .value
                    .clone()
                    .filter(|v| !v.is_empty())
                    .or_else(|| Some(cell.text.clone()).filter(|s| !s.is_empty()))
                    .or_else(|| Some(cell.parameter.clone()).filter(|s| !s.is_empty()))?;
                Some((c, text, book.intern(cell), cell.numeric))
            })
            .collect();
        if cells.is_empty() && row.height.is_none() {
            continue;
        }
        let _ = match row.height {
            Some(h) => write!(out, "<row r=\"{}\" ht=\"{h}\" customHeight=\"1\">", r + 1),
            None => write!(out, "<row r=\"{}\">", r + 1),
        };
        for (c, text, style, is_numeric) in cells {
            let (first, last) = layout.span(r, c);
            if last > first {
                spans.push((r, first, last));
            }
            let c = first;
            // В число превращается только то, у чего ЕСТЬ числовой формат.
            if let Some(number) = as_number(&text).filter(|_| is_numeric) {
                let _ = write!(
                    out,
                    "<c r=\"{}\" s=\"{style}\"><v>{number}</v></c>",
                    cell_ref(r, c)
                );
                continue;
            }
            let index = strings.intern(text);
            let _ = write!(
                out,
                "<c r=\"{}\" s=\"{style}\" t=\"s\"><v>{index}</v></c>",
                cell_ref(r, c)
            );
        }
        out.push_str("</row>");
    }
    out.push_str("</sheetData>");

    let mut merges = merge_rects(doc, layout);
    // Развороты добавляются к обычным объединениям, но не вытесняют их:
    // пересечения отсеются там же, где и всегда.
    for (r, c1, c2) in spans {
        if !merges
            .iter()
            .any(|&(a1, b1, a2, b2)| r <= a2 && a1 <= r && c1 <= b2 && b1 <= c2)
        {
            merges.push((r, c1, r, c2));
        }
    }
    merges.sort_unstable();
    if !merges.is_empty() {
        out.push_str(&format!("<mergeCells count=\"{}\">", merges.len()));
        for (r1, c1, r2, c2) in merges {
            let _ = write!(
                out,
                "<mergeCell ref=\"{}:{}\"/>",
                cell_ref(r1, c1),
                cell_ref(r2, c2)
            );
        }
        out.push_str("</mergeCells>");
    }

    out.push_str("</worksheet>");
    out
}

/// Байты .xlsx целиком.
pub fn to_xlsx_bytes(doc: &SpreadDocData) -> Vec<u8> {
    // Лист собирается ПЕРВЫМ: он же наполняет таблицу общих строк.
    let mut strings = SharedStrings::new();
    let mut book = StyleBook::default();
    let layout = Layout::new(doc);
    let sheet = worksheet(doc, &mut strings, &mut book, &layout);

    let mut zip = ZipWriter::new();
    zip.add("[Content_Types].xml", content_types().as_bytes());
    zip.add("_rels/.rels", root_rels().as_bytes());
    zip.add("xl/workbook.xml", workbook().as_bytes());
    zip.add("xl/_rels/workbook.xml.rels", workbook_rels().as_bytes());
    zip.add("xl/worksheets/sheet1.xml", sheet.as_bytes());
    zip.add("xl/styles.xml", styles(&book).as_bytes());
    zip.add(
        "xl/sharedStrings.xml",
        shared_strings(&strings.strings).as_bytes(),
    );
    zip.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn column_names() {
        assert_eq!(column_name(0), "A");
        assert_eq!(column_name(25), "Z");
        assert_eq!(column_name(26), "AA");
        assert_eq!(column_name(701), "ZZ");
        assert_eq!(column_name(702), "AAA");
    }

    #[test]
    fn cell_ref_is_one_based_by_row() {
        assert_eq!(cell_ref(0, 0), "A1");
        assert_eq!(cell_ref(4, 2), "C5");
    }

    /// Ячейка со значением и параметрическая тоже попадают в книгу — иначе
    /// отчёт с числами выгрузился бы пустым.
    #[test]
    fn values_and_parameters_reach_the_workbook() {
        let mut doc = SpreadDocData::new();
        doc.set_cell_text(0, 0, "Товар");
        doc.set_cell_value(0, 1, "1 000,5");
        doc.set_cell_parameter(1, 0, "Сумма");
        let mut strings = SharedStrings::new();
        worksheet(
            &doc,
            &mut strings,
            &mut StyleBook::default(),
            &Layout::new(&doc),
        );
        // Без числового формата значение остаётся СТРОКОЙ — ровно так же
        // повела себя платформа на таком же документе.
        assert_eq!(
            strings.strings,
            vec![
                "Товар".to_string(),
                "1 000,5".to_string(),
                "Сумма".to_string()
            ]
        );
    }

    /// А с числовым форматом — числом.
    #[test]
    fn a_number_format_makes_the_cell_numeric() {
        let mut doc = SpreadDocData::new();
        doc.set_cell_value(0, 0, "1 000,5");
        doc.set_cell_numeric(0, 0);
        let sheet = worksheet(
            &doc,
            &mut SharedStrings::new(),
            &mut StyleBook::default(),
            &Layout::new(&doc),
        );
        assert!(sheet.contains("<v>1000.5</v>"), "должно уйти числом");
    }

    #[test]
    fn shared_strings_are_deduplicated() {
        let mut doc = SpreadDocData::new();
        doc.set_cell_text(0, 0, "повтор");
        doc.set_cell_text(1, 0, "повтор");
        doc.set_cell_text(2, 0, "другое");
        let mut strings = SharedStrings::new();
        worksheet(
            &doc,
            &mut strings,
            &mut StyleBook::default(),
            &Layout::new(&doc),
        );
        assert_eq!(
            strings.strings,
            vec!["повтор".to_string(), "другое".to_string()]
        );
    }

    /// Одинаковое оформление — ОДИН стиль на обе ячейки, как у платформы:
    /// в её файле седьмая ячейка ссылалась на тот же `s`, что и вторая.
    #[test]
    fn equal_styling_reuses_the_style() {
        let mut doc = SpreadDocData::new();
        doc.set_cell_text(0, 0, "жирная");
        doc.set_cell_font(0, 0, crate::document::Font::new("Arial", 14).bold());
        doc.set_cell_text(1, 0, "снова жирная");
        doc.set_cell_font(1, 0, crate::document::Font::new("Arial", 14).bold());
        let mut book = StyleBook::default();
        let sheet = worksheet(
            &doc,
            &mut SharedStrings::new(),
            &mut book,
            &Layout::new(&doc),
        );
        assert_eq!(book.styles.len(), 1, "оформление должно совпасть");
        assert_eq!(sheet.matches("s=\"1\"").count(), 2);
    }

    /// Цвет в OOXML — в прямом порядке `RRGGBB`, в отличие от MXL.
    /// Что считается числом, а что нет. Дата и номер отправления обязаны
    /// остаться строками.
    /// Разметка листа на НАСТОЯЩЕМ отчёте: денежные ячейки числами, а
    /// объединения — на физической сетке, как в выгрузке платформы.
    #[test]
    fn a_real_report_lays_out_like_the_platform() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/conformance/mxl/report-real.mxl");
        let bytes = std::fs::read(&path).expect("нет эталона отчёта");
        let doc = crate::document::from_mxl_bytes(&bytes).expect("отчёт не разобрался");
        let sheet = worksheet(
            &doc,
            &mut SharedStrings::new(),
            &mut StyleBook::default(),
            &Layout::new(&doc),
        );
        assert!(sheet.contains("<v>1000.01</v>"), "сумма должна быть числом");
        assert!(sheet.contains("<v>22000.22</v>"), "итог должен быть числом");
        assert!(
            sheet.contains("<mergeCell ref=\"A5:G5\"/>"),
            "итог по организации"
        );
        assert!(
            sheet.contains("<mergeCell ref=\"A6:C6\"/>"),
            "строка таблицы"
        );
        assert!(sheet.contains("<mergeCell ref=\"D6:E6\"/>"));
    }

    #[test]
    fn number_recognition() {
        assert_eq!(as_number("1 000,01").as_deref(), Some("1000.01"));
        assert_eq!(as_number("1\u{a0}000,01").as_deref(), Some("1000.01"));
        assert_eq!(as_number("42").as_deref(), Some("42"));
        assert_eq!(
            as_number("736,80").as_deref(),
            Some("736.8"),
            "хвостовой ноль"
        );
        assert_eq!(
            as_number("498,00").as_deref(),
            Some("498"),
            "нулевая дробная часть"
        );
        assert_eq!(as_number("-3,5").as_deref(), Some("-3.5"));
        assert_eq!(as_number("01.01.2020 9:03:25"), None, "дата — не число");
        assert_eq!(as_number("NN100000000"), None, "номер — не число");
        assert_eq!(as_number("Итого"), None);
        assert_eq!(as_number(""), None);
        assert_eq!(as_number("1,"), None, "оборванная дробная часть");
    }

    #[test]
    fn color_is_in_direct_order() {
        assert_eq!(rgb(Color::new(255, 255, 0)), "FFFF00");
        assert_eq!(rgb(Color::new(0, 0, 255)), "0000FF");
    }

    /// Рамка заводит запись в `<borders>`, а ячейка ссылается на неё.
    #[test]
    fn border_reaches_the_workbook() {
        let mut doc = SpreadDocData::new();
        doc.set_cell_text(0, 0, "в рамке");
        let line = crate::document::Line::new(crate::document::LineStyle::Solid);
        doc.set_cell_border(0, 0, Some(line), None, None, Some(line));
        let mut book = StyleBook::default();
        worksheet(
            &doc,
            &mut SharedStrings::new(),
            &mut book,
            &Layout::new(&doc),
        );
        assert_eq!(book.borders, [[true, false, false, true]]);
        let xml = styles(&book);
        assert!(xml.contains("<left style=\"thin\"><color rgb=\"000000\"/></left>"));
        assert!(xml.contains("<right/>"));
    }

    #[test]
    fn special_characters_are_escaped() {
        assert_eq!(escape("a<b>&c"), "a&lt;b&gt;&amp;c");
        assert_eq!(escape("а\tб"), "а    б", "табуляция — четыре пробела");
    }

    /// Состав частей — не косметика: без `xl/styles.xml` платформа файл не
    /// открывает (измерено), поэтому его отсутствие должно ронять тест, а не
    /// обнаруживаться на живом отчёте.
    #[test]
    fn the_archive_has_every_required_part() {
        let mut doc = SpreadDocData::new();
        doc.set_cell_text(0, 0, "ячейка");
        let bytes = to_xlsx_bytes(&doc);
        for part in [
            "[Content_Types].xml",
            "_rels/.rels",
            "xl/workbook.xml",
            "xl/_rels/workbook.xml.rels",
            "xl/worksheets/sheet1.xml",
            "xl/styles.xml",
            "xl/sharedStrings.xml",
        ] {
            assert!(
                bytes.windows(part.len()).any(|w| w == part.as_bytes()),
                "в архиве нет части {part}"
            );
        }
    }

    #[test]
    fn merges_are_carried_into_ooxml() {
        let mut doc = SpreadDocData::new();
        doc.set_cell_text(0, 0, "шапка");
        doc.merge(crate::document::Merge::new(0, 0, 0, 2));
        let mut strings = SharedStrings::new();
        let sheet = worksheet(
            &doc,
            &mut strings,
            &mut StyleBook::default(),
            &Layout::new(&doc),
        );
        assert!(sheet.contains("<mergeCell ref=\"A1:C1\"/>"));
    }

    /// Объединение целых строк разворачивается во всю ШИРИНУ документа.
    #[test]
    fn row_merge_is_expanded() {
        let mut doc = SpreadDocData::new();
        doc.set_cell_text(0, 0, "A");
        doc.set_cell_text(3, 4, "Z");
        doc.merge_rows(1, 2);
        assert_eq!(merge_rects(&doc, &Layout::new(&doc)), [(1, 0, 2, 4)]);
    }

    /// Объединение целых колонок — во всю ВЫСОТУ.
    #[test]
    fn column_merge_is_expanded() {
        let mut doc = SpreadDocData::new();
        doc.set_cell_text(0, 0, "A");
        doc.set_cell_text(3, 4, "Z");
        doc.merge_columns(1, 2);
        assert_eq!(merge_rects(&doc, &Layout::new(&doc)), [(0, 1, 3, 2)]);
    }

    /// Отменённая строка РЕЖЕТ колоночное объединение на два куска, а не
    /// отбрасывается.
    #[test]
    fn cancellation_splits_a_column_merge() {
        let mut doc = SpreadDocData::new();
        doc.set_cell_text(0, 0, "A");
        doc.set_cell_text(4, 2, "Z");
        doc.merge_columns(0, 2);
        doc.unmerge_cells(2, 0, 2, 2);
        assert_eq!(
            merge_rects(&doc, &Layout::new(&doc)),
            [(0, 0, 1, 2), (3, 0, 4, 2)]
        );
    }

    /// Пересекающиеся объединения Excel считает повреждением файла, поэтому
    /// второе отбрасывается.
    #[test]
    fn overlaps_never_reach_the_workbook() {
        let mut doc = SpreadDocData::new();
        doc.set_cell_text(0, 0, "A");
        doc.set_cell_text(2, 2, "Z");
        doc.merge(crate::document::Merge::new(0, 0, 1, 1));
        doc.merge(crate::document::Merge::new(1, 1, 2, 2));
        assert_eq!(merge_rects(&doc, &Layout::new(&doc)), [(0, 0, 1, 1)]);
    }

    /// Объединение из одной ячейки в XLSX недопустимо.
    #[test]
    fn degenerate_merge_is_skipped() {
        let mut doc = SpreadDocData::new();
        doc.set_cell_text(0, 0, "A");
        doc.merge(crate::document::Merge::new(0, 0, 0, 0));
        assert!(merge_rects(&doc, &Layout::new(&doc)).is_empty());
    }
}
