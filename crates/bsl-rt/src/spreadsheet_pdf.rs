//! Запись табличного документа в PDF: раскладка ячеек по страницам.
//!
//! Слой формата — [`crate::pdf`], он ничего не знает ни про ячейки, ни про
//! страницы бумаги. Здесь наоборот: ни одного байта файла, только решения
//! раскладки — где кончается колонка, куда съезжает страница, на какой
//! высоте лежит базовая линия. Разделение не косметическое: писателю PDF
//! всё равно, откуда взялись координаты, а этому модулю всё равно, как
//! устроен `xref`.
//!
//! # Откуда взяты числа
//!
//! Ни одно из них не выдумано. Съёмка — `tests/conformance/pdf/
//! capture-platform-pdf-layout.bsl` на 1С:Предприятие 8.3.27.2130 (Linux
//! x86-64, 14 августа 2026 года): десяток документов, в каждом изменено
//! РОВНО ОДНО свойство, файлы закоммичены рядом со скриптом
//! (`probe-*.pdf`), а их контент-потоки разбираются командой
//!
//! ```text
//! python3 tests/conformance/pdf/dump-pdf-streams.py tests/conformance/pdf/probe-grid.pdf
//! ```
//!
//! Платформа кладёт все координаты на сетку в 1/600 дюйма (0.12 пункта):
//! A4 у неё — `595.32 x 841.92`, а не `595.28 x 841.89`, кегль 8 пунктов
//! выводится как `8.04 Tf`. Мы эту сетку НЕ воспроизводим — числа пишутся
//! как есть, — но размер страницы взят её, платформенный: так две
//! страницы сравнимы напрямую.
//!
//! Все числа ниже — те, что ЛЕЖАТ В ПОТОКЕ, то есть размеры отсечений
//! `re W n`. По горизонтали отсечения стыкуются без зазора (у первой
//! колонки `probe-colwidth` это `28.32 … 47.28`, а вторая начинается ровно
//! с 75.60), поэтому ширина отсечения и есть ширина колонки. По вертикали
//! отсечение на 0.12 пункта КОРОЧЕ строки, так что высота строки считается
//! по шагу между соседними отсечениями, и там, где это важно, названы обе
//! величины.
//!
//! Измеренное (в скобках — проба):
//!
//! * поля страницы задаются в МИЛЛИМЕТРАХ: умолчание 10 дало отступ 28.32
//!   пункта, `ПолеСлева = 30` — 85.08 (`probe-grid`, `probe-margins`);
//!   10 мм — это 28.3465 пункта, то есть перевод обычный, `72/25.4`;
//! * `ШиринаКолонки` — в ЗНАКАХ, умолчание 9; ширины 9, 5 и 40 дали 47.28,
//!   26.28 и 209.76 пункта, то есть 5.253, 5.256 и 5.244 пункта на знак
//!   (`probe-grid`, `probe-colwidth`). Взято 5.25. Разнобой в третьем знаке
//!   объясним: ПОСЛЕДНЮЮ колонку таблицы платформа укорачивает на 0.36
//!   (в `probe-grid` три колонки по 9 знаков дали 47.28, 47.28 и 46.92), и
//!   40 знаков без этой обрезки были бы 210.12 — ровно те же 5.253 на знак;
//! * `ВысотаСтроки` — в ПУНКТАХ, умолчание 0 значит «авто»: 10 дало
//!   отсечения высотой 9.84, 40 — 39.84, то есть строки 9.96 и 39.96
//!   (`probe-rowheight`; заданное значение платформа кладёт на свою сетку
//!   в 0.12 пункта). Автовысота при кегле 8 — 10.8 пункта, при 10 — 13.2,
//!   при 14 — 18.72 (`probe-grid`, `probe-font`), то есть 1.350, 1.320 и
//!   1.337 кегля: взято умолчательное 1.35;
//! * умолчательный кегль ячейки — 8 пунктов (`8.04 Tf` во всех пробах);
//! * текст отступает от левого края ячейки на 2.28 пункта, а базовая линия
//!   лежит на 2.52 пункта выше низа строки (`probe-grid`);
//! * по вертикали умолчание — НИЗ: в строке высотой 40 пунктов текст без
//!   заданного положения лёг на 2.52 выше её низа (`probe-rowheight`);
//!   `Верх` поднял базовую линию к верху строки, `Центр` отцентровал
//!   строчную коробку (`probe-align`);
//! * по горизонтали умолчание — ЛЕВО, и число ведёт себя как текст:
//!   ячейка со значением 42 и ячейка с текстом «1234» обе начались с того
//!   же отступа 2.28 (`probe-number`);
//! * сетку (`ОтображатьСетку`) платформа в PDF НЕ рисует вовсе — в
//!   потоках нет ни одной обводки, кроме заданных границ (`probe-grid`);
//! * граница ячейки — цвет `0.2 0.2 0.2`, толщина 0.75 пункта при
//!   `Толщина = 1` и 1.5 при 2; точечная — узор `[1] 0`; двойная — две
//!   параллельные линии в 1.51 пункта друг от друга (`probe-border`,
//!   `probe-line`);
//! * цвет текста выводится как `scn` перед `BT`, цвет фона — заливкой
//!   прямоугольника по ячейке (`probe-color`);
//! * объединённая область — одна ячейка на весь прямоугольник: и
//!   отсечение, и текст (`probe-merge`);
//! * страницы: документ в 120 строк лёг на две, по 72 строки на первой
//!   (72 строки по 10.8 занимают 777.6 пункта, а 73-я уже вышла бы за
//!   нижнее поле), документ в 20 колонок — на две по 11 колонок
//!   (`probe-pages`, `probe-wide`). Документ 90 x 20 дал ЧЕТЫРЕ страницы в
//!   порядке «сначала вправо, потом вниз»: 1.1, 1.12, 73.1, 73.12
//!   (`probe-big`);
//! * `ОриентацияСтраницы.Ландшафт` меняет местами стороны `/MediaBox` —
//!   `841.92 x 595.32`, поля при этом не меняются (`probe-landscape`);
//! * ПУСТОЙ документ платформа пишет как обычный файл из ОДНОЙ страницы
//!   (877 байт), а не отказывается (`probe-empty`). Поэтому пустая
//!   страница заводится и здесь: [`crate::pdf::PdfDocument::write`]
//!   документ без страниц отвергает.
//!
//! # Чего мы намеренно не повторяем
//!
//! Шрифт у нас Courier и не встраивается (почему — в обзоре
//! [`crate::pdf`]), поэтому ШИРИНА строки у нас своя: 0.6 кегля на знак
//! против пропорционального Arial у платформы. Байт в байт совпасть
//! файлы не могут и не должны; сходится текст, размер страницы и сетка
//! ячеек.
//!
//! Не воспроизводится и мелочь платформы: белый прямоугольник-подложка под
//! всей таблицей, кернинг внутри `TJ`, сдвиг обводок на полтолщины,
//! округление до 1/600 дюйма, укорочение последней колонки на 0.36 и зазор
//! отсечения — платформа отсекает ячейку на 0.12 пункта выше низа строки, а
//! мы во всю строку (отсечение графическое, на текст это не влияет). Не
//! рисуются подчёркивание и зачёркивание шрифта — их геометрия у платформы
//! не снималась, а рисовать по догадке незачем; перенос по словам
//! (`РазмещениеТекста.Переносить`) тоже не выполняется: длинный текст
//! обрезается отсечением, как у платформы обрезается текст, которому не
//! хватило соседней пустой ячейки.
//!
//! # Чем проверено
//!
//! Числа выше проверяют юнит-тесты этого модуля, а совпадение с
//! платформой — два внешних прогона `pdftotext` (poppler):
//!
//! * `probe-fit.pdf` — тот же документ, что снят с платформы, с
//!   колонками, широкими для обоих шрифтов. Извлечённые строки обязаны
//!   совпасть ПОСТРОЧНО и в том же порядке, а порядок `pdftotext` берёт
//!   из геометрии — значит совпал и он;
//! * `platform-simple.pdf` — тот же текст в колонках по 9 знаков. Здесь
//!   сверяется только СОСТАВ слов: наш Courier на 20% шире Arial, поэтому
//!   длинная ячейка физически налезает на соседнюю, и `pdftotext` читает
//!   наложившиеся колонки в другом порядке. Текст при этом не теряется —
//!   отсечение графическое.
//!
//! Перекрёстная проверка с другой стороны — в
//! `tests/conformance/measure/measure-pdf-write.bsl`: платформа своим
//! `ДокументPDF` открывает НАШ файл (лежит в скрипте шестнадцатеричной
//! строкой) и насчитывает в нём две страницы.

use std::collections::{BTreeMap, BTreeSet};

use crate::pdf::{PageId, PaintMode, PdfDocument, PdfFont};
use crate::spreadsheet::{CellData, HAlign, Line, LineStyle, SpreadDocData, VAlign};
use crate::RtResult;

/// A4 в пунктах — в том виде, в каком его пишет платформа (её округление
/// до 1/600 дюйма от 210 x 297 мм).
const A4_WIDTH_PT: f64 = 595.32;
const A4_HEIGHT_PT: f64 = 841.92;

/// Миллиметр в пунктах.
const MM_PT: f64 = 72.0 / 25.4;

/// Пункт на один знак `ШиринаКолонки`.
const CHAR_PT: f64 = 5.25;

/// `ШиринаКолонки` колонки, которой её не задавали.
const DEFAULT_COL_CHARS: f64 = 9.0;

/// Кегль ячейки, которой не задавали шрифт.
const DEFAULT_FONT_PT: f64 = 8.0;

/// Высота строчной коробки в долях кегля — она же автовысота строки.
const LINE_FACTOR: f64 = 1.35;

/// Отступ текста от края ячейки по горизонтали.
const PAD_PT: f64 = 2.28;

/// Насколько базовая линия выше низа строчной коробки.
const DESCENT_PT: f64 = 2.52;

/// Серый цвет границы ячейки.
const BORDER_GRAY: f64 = 0.2;

/// Толщина границы: тонкой (`Толщина = 1`) и толстой (2 и больше).
const BORDER_THIN_PT: f64 = 0.75;
const BORDER_THICK_PT: f64 = 1.5;

/// Расстояние между линиями двойной границы. Измерено 1.51 (`probe-line`:
/// две линии на 27.82 и 29.33), взято круглое 1.5 — разница 0.01 пункта
/// меньше 0.12, кванта самой платформы, то есть невидима на её же сетке.
const DOUBLE_GAP_PT: f64 = 1.5;

/// Поля страницы в МИЛЛИМЕТРАХ — в тех же единицах, в каких их принимают
/// свойства `ПолеСлева`, `ПолеСправа`, `ПолеСверху`, `ПолеСнизу`.
///
/// Умолчание 10 мм с каждой стороны измерено на пустом
/// `Новый ТабличныйДокумент`, поэтому [`Default`] написан руками: нулевые
/// поля — это не «не задано», а совсем другой документ.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PageMargins {
    pub left: f64,
    pub right: f64,
    pub top: f64,
    pub bottom: f64,
}

/// Умолчание полей в миллиметрах (измерено).
pub const DEFAULT_MARGIN_MM: f64 = 10.0;

impl Default for PageMargins {
    fn default() -> Self {
        PageMargins {
            left: DEFAULT_MARGIN_MM,
            right: DEFAULT_MARGIN_MM,
            top: DEFAULT_MARGIN_MM,
            bottom: DEFAULT_MARGIN_MM,
        }
    }
}

/// Прямоугольник ячейки на странице в пунктах: левый край, ВЕРХ, ширина и
/// высота. Верх, а не низ, потому что документ раскладывается сверху вниз,
/// а в координаты PDF (начало в левом нижнем углу) переводится один раз,
/// при выводе.
#[derive(Debug, Clone, Copy)]
struct CellBox {
    x: f64,
    top: f64,
    width: f64,
    height: f64,
}

impl CellBox {
    fn bottom(&self) -> f64 {
        self.top - self.height
    }

    fn right(&self) -> f64 {
        self.x + self.width
    }
}

/// Объединения, приведённые к одному виду: у каждого прямоугольника есть
/// угловая ячейка, которая и рисуется, и накрытые ячейки, которые
/// пропускаются.
struct Merges {
    /// Угол — правая нижняя граница объединения.
    corners: BTreeMap<(u32, u32), (u32, u32)>,
    covered: BTreeSet<(u32, u32)>,
    /// Занятый размер документа: им подпирается каждый прямоугольник,
    /// см. [`Merges::add`].
    rows: u32,
    cols: u32,
}

impl Merges {
    /// Три списка модели — прямоугольники, объединения целых строк и целых
    /// колонок — сводятся к одному набору прямоугольников по занятому
    /// размеру документа.
    fn collect(doc: &SpreadDocData) -> Merges {
        let (rows, cols) = (doc.height(), doc.width());
        let mut merges = Merges {
            corners: BTreeMap::new(),
            covered: BTreeSet::new(),
            rows,
            cols,
        };
        if rows == 0 || cols == 0 {
            return merges;
        }
        for m in doc.merges() {
            if m.unmerge {
                continue;
            }
            merges.add(m.r1, m.c1, m.r2, m.c2);
        }
        for &(r1, r2) in doc.row_merges() {
            merges.add(r1, 0, r2, cols - 1);
        }
        for &(c1, c2) in doc.col_merges() {
            merges.add(0, c1, rows - 1, c2);
        }
        merges
    }

    fn add(&mut self, r1: u32, c1: u32, r2: u32, c2: u32) {
        // Границы прямоугольника подпираются ЗАНЯТЫМ размером документа:
        // в модель объединения можно прийти из чужого файла, где запись
        // объявлена далеко за концом документа (`from_mxl_bytes` кладёт её
        // как есть, не подпирая и не расширяя документ), а без обрезки
        // цикл ниже материализовал бы каждую накрытую клетку и запись не
        // вернулась бы вовсе. Тот же инвариант с другой стороны держит
        // `cell_box`. Угол, вышедший за документ, рисовать не на чем.
        //
        // Отказом (`RtError`) это не сделано намеренно, и это НАШЕ решение,
        // а не снятое с платформы: что она делает с таким файлом, не
        // измерено. Отказ выглядит хуже обрезки — документ, который мы
        // умеем разложить целиком, перестал бы писаться из-за числа в
        // необязательной записи.
        if r1 >= self.rows || c1 >= self.cols {
            return;
        }
        let r2 = r2.min(self.rows - 1);
        let c2 = c2.min(self.cols - 1);
        // Пересечения объединений платформа не допускает, но файл мог
        // прийти и извне: угол, уже накрытый другим объединением, второй
        // раз не заводится, иначе нарисовались бы две ячейки поверх одной.
        if self.covered.contains(&(r1, c1)) {
            return;
        }
        self.corners.insert((r1, c1), (r2, c2));
        for r in r1..=r2 {
            for c in c1..=c2 {
                if (r, c) != (r1, c1) {
                    self.covered.insert((r, c));
                    self.corners.remove(&(r, c));
                }
            }
        }
    }
}

/// Раскладка одного документа: размеры колонок и строк в пунктах, разбивка
/// на страницы и сами объединения.
struct Layout<'a> {
    doc: &'a SpreadDocData,
    col_pt: Vec<f64>,
    row_pt: Vec<f64>,
    merges: Merges,
    page_width: f64,
    page_height: f64,
    margins: PageMargins,
}

impl<'a> Layout<'a> {
    fn new(doc: &'a SpreadDocData) -> Layout<'a> {
        let (page_width, page_height) = if doc.landscape {
            (A4_HEIGHT_PT, A4_WIDTH_PT)
        } else {
            (A4_WIDTH_PT, A4_HEIGHT_PT)
        };
        let col_pt = (0..doc.width())
            .map(|c| {
                let chars = doc
                    .col_widths()
                    .get(&c)
                    .map_or(DEFAULT_COL_CHARS, |w| *w as f64);
                chars.max(0.0) * CHAR_PT
            })
            .collect();
        let row_pt = (0..doc.height()).map(|r| row_height_pt(doc, r)).collect();
        Layout {
            doc,
            col_pt,
            row_pt,
            merges: Merges::collect(doc),
            page_width,
            page_height,
            margins: doc.margins,
        }
    }

    /// Ширина области печати: страница без левого и правого полей. Поля,
    /// съевшие всю страницу, дают полосу нулевой ширины — на такой полосе
    /// колонки всё равно раскладываются, по одной на страницу.
    fn printable_width(&self) -> f64 {
        (self.page_width - (self.margins.left + self.margins.right) * MM_PT).max(0.0)
    }

    fn printable_height(&self) -> f64 {
        (self.page_height - (self.margins.top + self.margins.bottom) * MM_PT).max(0.0)
    }

    /// Прямоугольник ячейки внутри полосы страницы. Для угла объединения
    /// размер считается по всем накрытым колонкам и строкам, но не дальше
    /// границ полосы: объединение рисуется на той странице, где лежит его
    /// левый верхний угол, и обрезается по краю полосы. Продолжения на
    /// следующей странице у него нет — что делает в этом случае платформа,
    /// не измерено.
    ///
    /// Правая нижняя граница ещё и подпирается снизу самим углом: в модель
    /// объединения можно прийти из чужого файла, а перевёрнутый
    /// прямоугольник дал бы срез с началом больше конца.
    fn cell_box(&self, block: &Block, r: u32, c: u32) -> CellBox {
        let (r2, c2) = self.merges.corners.get(&(r, c)).copied().unwrap_or((r, c));
        let x = self.margins.left * MM_PT
            + self.col_pt[block.c1 as usize..c as usize]
                .iter()
                .sum::<f64>();
        let top = self.page_height
            - self.margins.top * MM_PT
            - self.row_pt[block.r1 as usize..r as usize]
                .iter()
                .sum::<f64>();
        let last_col = c2.min(block.c2).max(c);
        let last_row = r2.min(block.r2).max(r);
        CellBox {
            x,
            top,
            width: self.col_pt[c as usize..=last_col as usize].iter().sum(),
            height: self.row_pt[r as usize..=last_row as usize].iter().sum(),
        }
    }

    /// Ячейка, у которой нечего рисовать, кроме размера: ни текста, ни
    /// оформления. Такую можно накрыть отсечением соседа слева — так
    /// делает платформа (измерено на `platform-simple.pdf`: «Накладная
    /// № 7» из колонки шириной 9 знаков получила отсечение на две
    /// колонки, потому что соседняя ячейка пуста).
    fn is_blank(&self, r: u32, c: u32) -> bool {
        match self.cell(r, c) {
            None => true,
            Some(cell) => {
                cell_text(cell).is_empty()
                    && cell.back_color.is_none()
                    && cell.border_left.is_none()
                    && cell.border_right.is_none()
                    && cell.border_top.is_none()
                    && cell.border_bottom.is_none()
            }
        }
    }

    fn cell(&self, r: u32, c: u32) -> Option<&'a CellData> {
        self.doc.rows().get(&r).and_then(|row| row.cells.get(&c))
    }
}

/// Высота строки в пунктах: заданная — как есть, «авто» (0 или не задана)
/// — по самому крупному шрифту строки.
fn row_height_pt(doc: &SpreadDocData, r: u32) -> f64 {
    let row = doc.rows().get(&r);
    if let Some(height) = row.and_then(|row| row.height) {
        if height > 0 {
            return height as f64;
        }
    }
    let biggest = row
        .map(|row| {
            row.cells
                .values()
                .filter_map(|cell| cell.font.as_ref())
                .map(|font| font.size.max(1) as f64)
                .fold(DEFAULT_FONT_PT, f64::max)
        })
        .unwrap_or(DEFAULT_FONT_PT);
    biggest * LINE_FACTOR
}

/// Что рисуется в ячейке: представление значения, а при его отсутствии —
/// текст. Тот же порядок, что у свойства `Текст` области (измерено:
/// после `Значение = 42` текст равен «42»).
fn cell_text(cell: &CellData) -> &str {
    match &cell.value {
        Some(value) => value,
        None => &cell.text,
    }
}

/// Полоса документа, попавшая на одну страницу: строки `r1..=r2` и колонки
/// `c1..=c2`. Пустой документ полос не даёт вовсе, и тогда страница
/// заводится пустая.
struct Block {
    r1: u32,
    r2: u32,
    c1: u32,
    c2: u32,
}

/// Разбить размеры на полосы, помещающиеся в `limit`. Полоса всегда
/// непуста: элемент, который не влезает и в одиночку, занимает свою полосу
/// целиком — иначе разбивка не сошлась бы вовсе.
fn split(sizes: &[f64], limit: f64) -> Vec<(u32, u32)> {
    let mut out = Vec::new();
    let mut start = 0usize;
    let mut used = 0.0;
    for (index, size) in sizes.iter().enumerate() {
        if index > start && used + size > limit {
            out.push((start as u32, index as u32 - 1));
            start = index;
            used = 0.0;
        }
        used += size;
    }
    if start < sizes.len() {
        out.push((start as u32, sizes.len() as u32 - 1));
    }
    out
}

/// Начертание Courier по флагам шрифта ячейки.
fn pdf_font(cell: Option<&CellData>) -> (PdfFont, f64) {
    match cell.and_then(|cell| cell.font.as_ref()) {
        None => (PdfFont::Courier, DEFAULT_FONT_PT),
        Some(font) => {
            let face = match (font.bold, font.italic) {
                (false, false) => PdfFont::Courier,
                (true, false) => PdfFont::CourierBold,
                (false, true) => PdfFont::CourierOblique,
                (true, true) => PdfFont::CourierBoldOblique,
            };
            (face, font.size.max(1) as f64)
        }
    }
}

/// Толщина линии в пунктах по толщине из модели: 1 — тонкая, всё, что
/// больше, — толстая (измерены обе).
fn line_width_pt(line: Line) -> f64 {
    if line.width >= 2 {
        BORDER_THICK_PT
    } else {
        BORDER_THIN_PT
    }
}

/// Разбить текст ячейки на строки. Перевод строки этот слой раскладывает
/// сам (писатель PDF его не принимает), табуляция заменяется пробелом:
/// табуляция в PDF — не знак, а решение о раскладке, и делать его за
/// пользователя молча правильнее, чем отказывать в записи всего документа.
fn text_lines(text: &str) -> Vec<String> {
    text.replace("\r\n", "\n")
        .replace('\r', "\n")
        .split('\n')
        .map(|line| line.replace('\t', " "))
        .collect()
}

/// Записать табличный документ в PDF.
///
/// # Errors
///
/// Ошибку возвращает слой формата: нерисуемый знак в тексте ячейки
/// (управляющий, кроме перевода строки и табуляции) или размер страницы,
/// от которого не осталось ничего положительного.
pub fn to_pdf_bytes(doc: &SpreadDocData) -> RtResult<Vec<u8>> {
    let layout = Layout::new(doc);
    let mut pdf = PdfDocument::new();

    let row_blocks = split(&layout.row_pt, layout.printable_height());
    let col_blocks = split(&layout.col_pt, layout.printable_width());
    if row_blocks.is_empty() || col_blocks.is_empty() {
        // Пустой документ платформа пишет одностраничным файлом (измерено).
        pdf.add_page(layout.page_width, layout.page_height)?;
        return pdf.write();
    }

    // Порядок страниц измерен на документе 90 x 20: сначала вправо, потом
    // вниз.
    for &(r1, r2) in &row_blocks {
        for &(c1, c2) in &col_blocks {
            let block = Block { r1, r2, c1, c2 };
            let page = pdf.add_page(layout.page_width, layout.page_height)?;
            Painter {
                pdf: &mut pdf,
                page,
                layout: &layout,
                block: &block,
            }
            .block()?;
        }
    }
    pdf.write()
}

/// Кисть: страница, полоса документа на ней и раскладка, по которой всё
/// это считается. Отдельный тип, а не пяток аргументов у каждой функции
/// рисования, — вниз по вызовам идут одни и те же четыре вещи.
struct Painter<'a> {
    pdf: &'a mut PdfDocument,
    page: PageId,
    layout: &'a Layout<'a>,
    block: &'a Block,
}

impl Painter<'_> {
    /// Нарисовать всю полосу документа на своей странице.
    fn block(&mut self) -> RtResult<()> {
        for r in self.block.r1..=self.block.r2 {
            for c in self.block.c1..=self.block.c2 {
                if self.layout.merges.covered.contains(&(r, c)) {
                    continue;
                }
                let Some(cell) = self.layout.cell(r, c) else {
                    continue;
                };
                let area = self.layout.cell_box(self.block, r, c);
                self.cell(cell, area, r, c)?;
            }
        }
        Ok(())
    }

    /// Нарисовать одну ячейку: фон, границы и текст — именно в этом
    /// порядке, иначе фон закрасил бы текст.
    fn cell(&mut self, cell: &CellData, area: CellBox, r: u32, c: u32) -> RtResult<()> {
        if let Some(color) = cell.back_color {
            self.pdf
                .set_fill_color(self.page, unit(color.r), unit(color.g), unit(color.b))?;
            self.pdf.rect(
                self.page,
                area.x,
                area.bottom(),
                area.width,
                area.height,
                PaintMode::Fill,
            )?;
            self.pdf.set_fill_color(self.page, 0.0, 0.0, 0.0)?;
        }

        self.borders(cell, area)?;

        let text = cell_text(cell);
        if text.is_empty() {
            return Ok(());
        }
        let (font, size) = pdf_font(Some(cell));
        let line_height = size * LINE_FACTOR;
        let lines = text_lines(text);

        // Отсечение: ячейка плюс подряд идущие ПУСТЫЕ соседи справа, и
        // только пока текст не помещается (измерено — см. обзор модуля).
        let widest = lines
            .iter()
            .map(|line| font.text_width(line, size))
            .fold(0.0, f64::max);
        let mut clip_right = area.right();
        let mut next = c + 1;
        while widest + 2.0 * PAD_PT > clip_right - area.x
            && next <= self.block.c2
            && !self.layout.merges.covered.contains(&(r, next))
            && self.layout.is_blank(r, next)
        {
            clip_right += self.layout.col_pt[next as usize];
            next += 1;
        }

        self.pdf.push_clip(
            self.page,
            area.x,
            area.bottom(),
            clip_right - area.x,
            area.height,
        )?;
        if let Some(color) = cell.text_color {
            self.pdf
                .set_fill_color(self.page, unit(color.r), unit(color.g), unit(color.b))?;
        }

        // Блок строк ставится по вертикальному положению целиком, а строки
        // внутри него идут сверху вниз.
        let block_height = line_height * lines.len() as f64;
        let block_top = match cell.v_align.unwrap_or(VAlign::Bottom) {
            VAlign::Top => area.top,
            VAlign::Center => area.top - (area.height - block_height) / 2.0,
            VAlign::Bottom => area.bottom() + block_height,
        };
        for (index, line) in lines.iter().enumerate() {
            if line.is_empty() {
                continue;
            }
            let baseline = block_top - line_height * (index as f64 + 1.0) + DESCENT_PT;
            let width = font.text_width(line, size);
            let x = match cell.h_align.unwrap_or(HAlign::Auto) {
                HAlign::Auto | HAlign::Left => area.x + PAD_PT,
                HAlign::Center => area.x + (area.width - width) / 2.0,
                HAlign::Right => area.right() - PAD_PT - width,
            };
            self.pdf.text(self.page, x, baseline, font, size, line)?;
        }

        if cell.text_color.is_some() {
            self.pdf.set_fill_color(self.page, 0.0, 0.0, 0.0)?;
        }
        self.pdf.pop_clip(self.page)
    }

    /// Четыре стороны рамки. Каждая сторона рисуется своей ячейкой:
    /// соседи, объявившие общую границу оба, нарисуют её дважды в одном и
    /// том же месте — платформа делает так же.
    fn borders(&mut self, cell: &CellData, area: CellBox) -> RtResult<()> {
        let sides = [
            (cell.border_left, (area.x, area.bottom(), area.x, area.top)),
            (cell.border_top, (area.x, area.top, area.right(), area.top)),
            (
                cell.border_right,
                (area.right(), area.bottom(), area.right(), area.top),
            ),
            (
                cell.border_bottom,
                (area.x, area.bottom(), area.right(), area.bottom()),
            ),
        ];
        let mut painted = false;
        for (line, (x1, y1, x2, y2)) in sides {
            let Some(line) = line else {
                continue;
            };
            if !painted {
                self.pdf
                    .set_stroke_color(self.page, BORDER_GRAY, BORDER_GRAY, BORDER_GRAY)?;
                painted = true;
            }
            self.pdf.set_line_width(self.page, line_width_pt(line))?;
            match line.style {
                LineStyle::Solid => {
                    self.pdf.set_dash(self.page, &[], 0.0)?;
                    self.pdf.line(self.page, x1, y1, x2, y2)?;
                }
                LineStyle::Dotted => {
                    self.pdf.set_dash(self.page, &[1.0], 0.0)?;
                    self.pdf.line(self.page, x1, y1, x2, y2)?;
                    self.pdf.set_dash(self.page, &[], 0.0)?;
                }
                LineStyle::Double => {
                    // Две параллельные линии по обе стороны от края
                    // ячейки: сдвиг идёт поперёк линии, поэтому
                    // вертикальная сторона расходится по X, а
                    // горизонтальная — по Y.
                    self.pdf.set_dash(self.page, &[], 0.0)?;
                    let half = DOUBLE_GAP_PT / 2.0;
                    let (dx, dy) = if (x1 - x2).abs() < f64::EPSILON {
                        (half, 0.0)
                    } else {
                        (0.0, half)
                    };
                    self.pdf
                        .line(self.page, x1 - dx, y1 - dy, x2 - dx, y2 - dy)?;
                    self.pdf
                        .line(self.page, x1 + dx, y1 + dy, x2 + dx, y2 + dy)?;
                }
            }
        }
        if painted {
            self.pdf.set_stroke_color(self.page, 0.0, 0.0, 0.0)?;
        }
        Ok(())
    }
}

/// Составляющая цвета из байта модели в долю единицы, как её ждёт PDF.
fn unit(component: u8) -> f64 {
    f64::from(component) / 255.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spreadsheet::{from_mxl_bytes, to_mxl_bytes, Color, Font, Merge};

    /// Число так, как его пишет слой формата: четыре знака после запятой
    /// без хвостовых нулей. Ожидания тестов считаются той же арифметикой,
    /// что и раскладка, — иначе они проверяли бы форматирование, а не
    /// геометрию.
    fn num(x: f64) -> String {
        let mut s = format!("{x:.4}");
        if s.contains('.') {
            while s.ends_with('0') {
                s.pop();
            }
            if s.ends_with('.') {
                s.pop();
            }
        }
        s
    }

    fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
        haystack
            .windows(needle.len())
            .position(|window| window == needle)
    }

    /// Сколько в файле страниц: по числу словарей страницы. Узел `/Pages`
    /// под это не подходит — у него `/Type /Pages`.
    fn page_count(pdf: &[u8]) -> usize {
        String::from_utf8_lossy(pdf)
            .to_string()
            .matches("/Type /Page /Parent")
            .count()
    }

    /// Распакованный контент-поток страницы с заданным номером. Потоки в
    /// файле идут по страницам, а `/ToUnicode` отличается от них тем, что
    /// операторов рисования в нём нет.
    fn page_stream(pdf: &[u8], index: usize) -> String {
        let mut streams = Vec::new();
        let mut at = 0;
        while let Some(found) = find(&pdf[at..], b"stream\n") {
            let start = at + found + b"stream\n".len();
            let Some(found) = find(&pdf[start..], b"\nendstream") else {
                break;
            };
            let end = start + found;
            // Оболочка zlib: два байта заголовка и четыре Adler-32.
            if end > start + 6 {
                if let Ok(data) = crate::inflate::inflate(&pdf[start + 2..end - 4], 1 << 22) {
                    streams.push(String::from_utf8_lossy(&data).to_string());
                }
            }
            at = end + b"\nendstream".len();
        }
        streams
            .into_iter()
            .filter(|s| s.contains(" re\n") || s.contains(" Td\n"))
            .nth(index)
            .expect("в файле нет потока страницы с таким номером")
    }

    /// Документ из съёмки `capture-platform-pdf-layout.bsl`, проба
    /// `probe-fit.pdf`: тот же текст, что у `platform-simple.pdf`, но
    /// колонки шире — чтобы ни одна ячейка не вылезала за свою колонку ни
    /// у нас, ни у платформы.
    fn fitting_invoice() -> SpreadDocData {
        let mut doc = SpreadDocData::new();
        doc.set_col_width(0, 24);
        doc.set_col_width(1, 12);
        doc.set_cell_text(0, 0, "Накладная № 7");
        doc.set_cell_text(1, 0, "Товар");
        doc.set_cell_text(1, 1, "Цена");
        doc.set_cell_text(2, 0, "Гвоздь строительный");
        doc.set_cell_text(2, 1, "123.45");
        doc.set_cell_text(3, 0, "Latin ASCII text");
        doc.set_cell_text(3, 1, "67890");
        doc
    }

    /// Тот же документ с умолчательными ширинами — как в
    /// `capture-platform-pdf.bsl`, с которого снят `platform-simple.pdf`.
    fn invoice() -> SpreadDocData {
        let mut doc = fitting_invoice();
        doc.set_col_width(0, 9);
        doc.set_col_width(1, 9);
        doc
    }

    #[test]
    fn pdf_page_is_a4_and_landscape_swaps_it() {
        let mut doc = SpreadDocData::new();
        doc.set_cell_text(0, 0, "X");
        let text = String::from_utf8_lossy(&to_pdf_bytes(&doc).unwrap()).to_string();
        assert!(text.contains("/MediaBox [ 0 0 595.32 841.92 ]"), "{text}");
        doc.landscape = true;
        let text = String::from_utf8_lossy(&to_pdf_bytes(&doc).unwrap()).to_string();
        assert!(text.contains("/MediaBox [ 0 0 841.92 595.32 ]"), "{text}");
    }

    /// Умолчания измерены: левое поле 10 мм — это 28.3465 пункта, колонка
    /// шириной 9 знаков — 47.25, автовысота строки при кегле 8 — 10.8.
    /// Отсюда и место текста: отступ 2.28 от края и базовая линия на 2.52
    /// выше низа строки.
    #[test]
    fn pdf_default_grid_matches_the_measured_geometry() {
        let mut doc = SpreadDocData::new();
        doc.set_cell_text(0, 0, "A");
        doc.set_cell_text(1, 0, "B");
        let content = page_stream(&to_pdf_bytes(&doc).unwrap(), 0);
        let left = 10.0 * MM_PT;
        let top = A4_HEIGHT_PT - 10.0 * MM_PT;
        assert!(
            content.contains(&format!(
                "{} {} Td",
                num(left + PAD_PT),
                num(top - 10.8 + DESCENT_PT)
            )),
            "первая строка: {content}"
        );
        assert!(
            content.contains(&format!(
                "{} {} Td",
                num(left + PAD_PT),
                num(top - 21.6 + DESCENT_PT)
            )),
            "вторая строка ниже ровно на автовысоту: {content}"
        );
        assert!(
            content.contains(&format!(
                "{} {} {} 10.8 re",
                num(left),
                num(top - 10.8),
                num(9.0 * CHAR_PT)
            )),
            "отсечение по прямоугольнику ячейки: {content}"
        );
    }

    /// Поля страницы — в миллиметрах, и они сдвигают начало таблицы.
    #[test]
    fn pdf_margins_are_millimetres() {
        let mut doc = SpreadDocData::new();
        doc.set_cell_text(0, 0, "A");
        doc.margins.left = 30.0;
        doc.margins.top = 25.0;
        let content = page_stream(&to_pdf_bytes(&doc).unwrap(), 0);
        assert!(
            content.contains(&format!(
                "{} {} Td",
                num(30.0 * MM_PT + PAD_PT),
                num(A4_HEIGHT_PT - 25.0 * MM_PT - 10.8 + DESCENT_PT)
            )),
            "{content}"
        );
    }

    /// Разбивка на страницы: 120 строк умолчательной высоты не помещаются
    /// на A4 с полями по 10 мм — платформа положила на первую страницу 72
    /// строки, столько же кладём и мы.
    #[test]
    fn pdf_rows_that_do_not_fit_start_a_new_page() {
        let mut doc = SpreadDocData::new();
        for r in 0..120 {
            doc.set_cell_text(r, 0, &format!("строка {}", r + 1));
        }
        let bytes = to_pdf_bytes(&doc).unwrap();
        assert_eq!(page_count(&bytes), 2);
        assert_eq!(page_stream(&bytes, 0).matches(" Td\n").count(), 72);
        assert_eq!(page_stream(&bytes, 1).matches(" Td\n").count(), 48);
    }

    /// Слишком широкий документ рвётся и по колонкам, и по строкам, а
    /// порядок страниц — «сначала вправо, потом вниз» (измерено на
    /// документе 90 x 20: 1.1, 1.12, 73.1, 73.12).
    #[test]
    fn pdf_wide_document_splits_by_columns_first() {
        let mut doc = SpreadDocData::new();
        for r in 0..90 {
            for c in 0..20 {
                doc.set_cell_text(r, c, &format!("{}.{}", r + 1, c + 1));
            }
        }
        let bytes = to_pdf_bytes(&doc).unwrap();
        assert_eq!(page_count(&bytes), 4);
        for (index, first) in ["(1.1) Tj", "(1.12) Tj", "(73.1) Tj", "(73.12) Tj"]
            .into_iter()
            .enumerate()
        {
            let content = page_stream(&bytes, index);
            assert!(content.contains(first), "страница {index}: {content}");
        }
    }

    /// Объединение рисуется одной ячейкой на весь прямоугольник, а
    /// накрытые ячейки пропускаются.
    #[test]
    fn pdf_merge_draws_one_cell_for_the_whole_rectangle() {
        let mut doc = SpreadDocData::new();
        doc.set_cell_text(0, 0, "шапка");
        doc.set_cell_text(0, 1, "затрётся");
        doc.set_cell_text(1, 0, "низ");
        doc.merge(Merge::new(0, 0, 0, 1));
        let content = page_stream(&to_pdf_bytes(&doc).unwrap(), 0);
        assert_eq!(content.matches("Tj\n").count(), 2, "{content}");
        assert!(
            content.contains(&format!(" {} 10.8 re", num(18.0 * CHAR_PT))),
            "отсечение угла — на обе колонки: {content}"
        );
    }

    /// Объединение, объявленное далеко за концом документа, подпирается его
    /// занятым размером. Прийти такая запись может только из чужого файла:
    /// `Объединить` в модели расширяет документ под прямоугольник, а
    /// `from_mxl_bytes` кладёт запись как есть. Без обрезки раскладка
    /// материализовала бы миллион на миллион накрытых клеток и не
    /// вернулась бы вовсе.
    #[test]
    fn pdf_merge_declared_far_beyond_the_document_is_clamped() {
        let mut doc = SpreadDocData::new();
        doc.set_cell_text(0, 0, "шапка");
        doc.set_cell_text(1, 1, "низ");
        doc.merge(Merge::new(0, 0, 0, 1));

        // Запись объединения в файле — `{КолонкаНач, СтрокаНач, КолонкаКон,
        // СтрокаКон, флаг}`; подменяется правый нижний угол, размеры самого
        // документа остаются прежними, 2 x 2.
        let mut mxl = to_mxl_bytes(&doc);
        let record = b"{0,0,1,0,0}";
        let at = find(&mxl, record).expect("запись объединения в файле");
        mxl.splice(
            at..at + record.len(),
            b"{0,0,1000000,1000000,0}".iter().copied(),
        );
        let hostile = from_mxl_bytes(&mxl).expect("файл с чужим объединением читается");
        assert_eq!(
            (hostile.height(), hostile.width()),
            (2, 2),
            "документ должен остаться 2 x 2"
        );

        let bytes =
            to_pdf_bytes(&hostile).expect("такой файл обязан записаться, а не подвесить запись");
        assert_eq!(page_count(&bytes), 1, "страница ровно одна");
        let content = page_stream(&bytes, 0);
        assert!(
            content.contains(&format!(" {} {} re", num(18.0 * CHAR_PT), num(2.0 * 10.8))),
            "объединение обрезано по занятому размеру, две колонки и две строки: {content}"
        );
    }

    /// Выравнивание по горизонтали: центр и право считаются по ширине
    /// строки, а `Авто` ведёт себя как «лево» — измерено, число вправо не
    /// отъезжает.
    #[test]
    fn pdf_alignment_moves_the_text_inside_the_cell() {
        let mut doc = SpreadDocData::new();
        for c in 0..3 {
            doc.set_cell_text(0, c, "abc");
        }
        doc.set_cell_h_align(0, 1, HAlign::Center);
        doc.set_cell_h_align(0, 2, HAlign::Right);
        let content = page_stream(&to_pdf_bytes(&doc).unwrap(), 0);
        let width = PdfFont::Courier.text_width("abc", DEFAULT_FONT_PT);
        let col = 9.0 * CHAR_PT;
        let left = 10.0 * MM_PT;
        assert!(
            content.contains(&format!("{} ", num(left + PAD_PT))),
            "лево: {content}"
        );
        assert!(
            content.contains(&format!("{} ", num(left + col + (col - width) / 2.0))),
            "центр: {content}"
        );
        assert!(
            content.contains(&format!("{} ", num(left + 3.0 * col - PAD_PT - width))),
            "право: {content}"
        );
    }

    /// Вертикальное положение: умолчание — низ, `Верх` поднимает базовую
    /// линию к верху ячейки, `Центр` ставит строчную коробку посередине.
    #[test]
    fn pdf_vertical_alignment_follows_the_measured_defaults() {
        let mut doc = SpreadDocData::new();
        for c in 0..3 {
            doc.set_cell_text(0, c, "x");
        }
        doc.set_row_height(0, 40);
        doc.set_cell_v_align(0, 1, VAlign::Top);
        doc.set_cell_v_align(0, 2, VAlign::Center);
        let content = page_stream(&to_pdf_bytes(&doc).unwrap(), 0);
        let top = A4_HEIGHT_PT - 10.0 * MM_PT;
        let bottom = top - 40.0;
        let line = DEFAULT_FONT_PT * LINE_FACTOR;
        assert!(
            content.contains(&format!(" {} Td", num(bottom + DESCENT_PT))),
            "низ: {content}"
        );
        assert!(
            content.contains(&format!(" {} Td", num(top - line + DESCENT_PT))),
            "верх: {content}"
        );
        assert!(
            content.contains(&format!(
                " {} Td",
                num(top - (40.0 - line) / 2.0 - line + DESCENT_PT)
            )),
            "центр: {content}"
        );
    }

    /// Шрифт ячейки доходит до файла начертанием и кеглем, а автовысота
    /// строки считается по самому крупному шрифту в ней.
    #[test]
    fn pdf_cell_font_reaches_the_stream() {
        let mut doc = SpreadDocData::new();
        doc.set_cell_text(0, 0, "жирный");
        doc.set_cell_font(0, 0, Font::new("Arial", 14).bold());
        doc.set_cell_text(1, 0, "обычный");
        let bytes = to_pdf_bytes(&doc).unwrap();
        let content = page_stream(&bytes, 0);
        assert!(content.contains(" 14 Tf"), "{content}");
        assert!(content.contains(" 8 Tf"), "{content}");
        let text = String::from_utf8_lossy(&bytes).to_string();
        assert!(text.contains("/BaseFont /Courier-Bold"), "{text}");
        let top = A4_HEIGHT_PT - 10.0 * MM_PT;
        assert!(
            content.contains(&format!(
                " {} Td",
                num(top - 14.0 * LINE_FACTOR - DEFAULT_FONT_PT * LINE_FACTOR + DESCENT_PT)
            )),
            "строка с крупным шрифтом выше умолчательной: {content}"
        );
    }

    /// Границы: цвет и толщина измерены, точечная линия получает узор, а
    /// двойная — две линии вместо одной.
    #[test]
    fn pdf_borders_use_the_measured_colour_and_widths() {
        let mut doc = SpreadDocData::new();
        doc.set_cell_text(0, 0, "рамка");
        doc.set_cell_border(0, 0, Some(Line::new(LineStyle::Solid)), None, None, None);
        doc.set_cell_border(
            1,
            0,
            None,
            Some(Line::new(LineStyle::Solid).thick()),
            None,
            None,
        );
        doc.set_cell_border(2, 0, None, None, Some(Line::new(LineStyle::Dotted)), None);
        doc.set_cell_border(3, 0, None, None, None, Some(Line::new(LineStyle::Double)));
        let content = page_stream(&to_pdf_bytes(&doc).unwrap(), 0);
        assert!(content.contains("0.2 0.2 0.2 RG"), "{content}");
        assert!(content.contains("0.75 w"), "{content}");
        assert!(content.contains("1.5 w"), "{content}");
        assert!(content.contains("[1] 0 d"), "{content}");
        assert_eq!(
            content.matches("l\nS\n").count(),
            5,
            "три одиночные стороны и двойная в два отрезка: {content}"
        );
    }

    /// Цвета текста и фона доходят до потока и не остаются включёнными на
    /// следующую ячейку.
    #[test]
    fn pdf_cell_colours_are_reset_after_the_cell() {
        let mut doc = SpreadDocData::new();
        doc.set_cell_text(0, 0, "цвет");
        doc.set_cell_text_color(0, 0, Color::new(255, 0, 0));
        doc.set_cell_back_color(0, 0, Color::new(0, 0, 255));
        let content = page_stream(&to_pdf_bytes(&doc).unwrap(), 0);
        assert!(content.contains("1 0 0 rg"), "{content}");
        assert!(content.contains("0 0 1 rg"), "{content}");
        assert_eq!(content.matches("0 0 0 rg").count(), 2, "{content}");
    }

    /// Отсечение накрывает соседа справа, если тот пуст, а текст не влез —
    /// ровно так делает платформа.
    #[test]
    fn pdf_long_text_spills_over_empty_neighbours_only() {
        let mut doc = SpreadDocData::new();
        doc.set_cell_text(0, 0, "очень длинный текст ячейки");
        doc.set_cell_text(1, 0, "очень длинный текст ячейки");
        doc.set_cell_text(1, 1, "занято");
        let content = page_stream(&to_pdf_bytes(&doc).unwrap(), 0);
        assert!(
            content.contains(&format!(" {} 10.8 re", num(18.0 * CHAR_PT))),
            "пустой сосед: {content}"
        );
        assert!(
            content.contains(&format!(" {} 10.8 re", num(9.0 * CHAR_PT))),
            "занятый сосед: {content}"
        );
    }

    /// Пустой документ — это одностраничный файл, а не отказ (измерено:
    /// платформа пишет 877 байт с одной страницей).
    #[test]
    fn pdf_empty_document_is_one_empty_page() {
        let bytes = to_pdf_bytes(&SpreadDocData::new()).unwrap();
        assert!(bytes.starts_with(b"%PDF-"));
        assert_eq!(page_count(&bytes), 1);
    }

    /// Перевод строки внутри ячейки раскладывает этот слой: писателю PDF
    /// он не достаётся вовсе, а табуляция превращается в пробел.
    #[test]
    fn pdf_multiline_cell_becomes_separate_lines() {
        let mut doc = SpreadDocData::new();
        doc.set_cell_text(0, 0, "первая\nвторая\tс табуляцией");
        doc.set_row_height(0, 40);
        let content = page_stream(&to_pdf_bytes(&doc).unwrap(), 0);
        assert_eq!(content.matches("Tj\n").count(), 2, "{content}");
        assert_eq!(content.matches(" Td\n").count(), 2, "{content}");
        assert!(!content.contains('\t'), "{content}");
    }

    /// Знак, который слой формата рисовать не берётся, обязан стать
    /// `RtError`, а не паникой и не молчаливой потерей.
    #[test]
    fn pdf_control_character_in_a_cell_is_an_error() {
        let mut doc = SpreadDocData::new();
        doc.set_cell_text(0, 0, "звонок\u{7}внутри");
        let err = to_pdf_bytes(&doc).unwrap_err();
        assert!(format!("{err}").contains("U+0007"), "{err}");
    }

    /// Извлечь текст `pdftotext`'ом. `None` — извлекателя нет в `PATH`.
    fn extract(path: &std::path::Path) -> Option<Vec<String>> {
        let out = std::process::Command::new("pdftotext")
            .args(["-enc", "UTF-8"])
            .arg(path)
            .arg("-")
            .output()
            .ok()?;
        assert!(
            out.status.success(),
            "pdftotext: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        Some(
            String::from_utf8_lossy(&out.stdout)
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty() && *line != "\u{c}")
                .map(str::to_string)
                .collect(),
        )
    }

    fn write_temp(name: &str, bytes: &[u8]) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("open-bsl-pdf-cmp-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        std::fs::write(&path, bytes).unwrap();
        path
    }

    /// ВНЕШНЯЯ перекрёстная проверка раскладки: тот же документ, что снят
    /// с платформы в `tests/conformance/pdf/probe-fit.pdf`, обязан дать
    /// ТОТ ЖЕ извлечённый текст — построчно и в том же порядке.
    ///
    /// Порядок здесь не украшение: `pdftotext` строит его по геометрии, а
    /// не по порядку в потоке, — значит совпадение строк означает, что
    /// колонки, строки и переносы легли туда же, куда у платформы.
    ///
    /// Без `pdftotext` в `PATH` тест печатает пометку и проходит —
    /// прецедент тот же, что у `table_compare2` в `bsl-cli`.
    #[test]
    fn pdf_text_matches_the_platform_file() {
        let ours = write_temp("fit.pdf", &to_pdf_bytes(&fitting_invoice()).unwrap());
        let Some(mine) = extract(&ours) else {
            println!("ПРОПУЩЕНО: pdftotext не найден в PATH — сверка с платформой не выполнена");
            return;
        };
        let theirs = extract(std::path::Path::new(
            "../../tests/conformance/pdf/probe-fit.pdf",
        ))
        .unwrap();
        println!(
            "pdftotext ИСПОЛНЕН на probe-fit.pdf\nнаш:        {mine:?}\nплатформа:  {theirs:?}"
        );
        assert_eq!(mine, theirs);
        std::fs::remove_file(&ours).ok();
    }

    /// Тот же документ, но с УМОЛЧАТЕЛЬНЫМИ ширинами колонок — это
    /// `platform-simple.pdf`, снятый ещё на прошлой задаче.
    ///
    /// Здесь сверяется не порядок строк, а состав слов, и вот почему:
    /// наш моноширинный Courier на 20% шире платформенного Arial, поэтому
    /// «Гвоздь строительный» в колонке шириной 9 знаков физически налезает
    /// на соседнюю ячейку. Текст от этого не теряется — отсечение
    /// графическое, — но `pdftotext` начинает читать наложившиеся колонки
    /// в другом порядке и рвёт строку по словам. Требовать здесь
    /// построчного совпадения значило бы требовать чужих метрик шрифта;
    /// состав же слов обязан сойтись — ни одного лишнего, ни одного
    /// потерянного.
    #[test]
    fn pdf_narrow_columns_keep_every_word() {
        let ours = write_temp("simple.pdf", &to_pdf_bytes(&invoice()).unwrap());
        let Some(mine) = extract(&ours) else {
            println!("ПРОПУЩЕНО: pdftotext не найден в PATH — сверка с платформой не выполнена");
            return;
        };
        let theirs = extract(std::path::Path::new(
            "../../tests/conformance/pdf/platform-simple.pdf",
        ))
        .unwrap();
        let words = |lines: Vec<String>| {
            let mut out: Vec<String> = lines
                .iter()
                .flat_map(|line| line.split_whitespace())
                .map(str::to_string)
                .collect();
            out.sort();
            out
        };
        let (mine, theirs) = (words(mine), words(theirs));
        println!("pdftotext ИСПОЛНЕН на platform-simple.pdf\nнаш:        {mine:?}\nплатформа:  {theirs:?}");
        assert_eq!(mine, theirs);
        std::fs::remove_file(&ours).ok();
    }
}
