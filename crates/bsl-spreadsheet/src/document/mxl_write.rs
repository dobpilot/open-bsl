//! Сериализация модели в MXL.

use super::*;

// --- сериализация MXL ---------------------------------------------------

/// Строка в кавычках с удвоением внутренней кавычки — единственное
/// экранирование, которое делает платформа.
pub(crate) fn quoted(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for ch in s.chars() {
        if ch == '"' {
            out.push('"');
        }
        out.push(ch);
    }
    out.push('"');
    out
}

/// Биты маски формата. Формат — это НЕ запись с фиксированными полями, а
/// набор «бит + значение»: `{257,0,6}` — это шрифт (бит 1) со значением 0 и
/// горизонтальное положение (бит 256) со значением 6 в ОДНОМ элементе
/// палитры. Измерено на ячейке, которой задали и шрифт, и выравнивание.
///
/// Значения бит сняты по одному свойству за раз. Часть из них пока не
/// используется: шрифты, границы и цвета требуют СВОИХ палитр, которых ещё
/// нет, — но сами номера бит уже измерены, и терять их, чтобы унять
/// компилятор, было бы потерей результата замера.
#[allow(dead_code)]
pub(crate) mod bits {
    /// Индекс в палитре шрифтов.
    pub const FONT: u32 = 1;
    /// Индексы в палитре линий.
    pub const BORDER_LEFT: u32 = 2;
    pub const BORDER_TOP: u32 = 4;
    pub const BORDER_RIGHT: u32 = 8;
    pub const BORDER_BOTTOM: u32 = 16;
    /// Высота строки, единицы — четверти пункта API.
    pub const ROW_HEIGHT: u32 = 64;
    /// Ширина колонки, единицы — восьмые доли знака API.
    pub const COL_WIDTH: u32 = 128;
    pub const H_ALIGN: u32 = 256;
    pub const V_ALIGN: u32 = 512;
    /// Индексы в палитре цветов.
    pub const TEXT_COLOR: u32 = 1024;
    pub const BACK_COLOR: u32 = 2048;
    pub const TEXT_PLACEMENT: u32 = 16384;
    /// У ячейки есть числовой формат. Найден сравнением масок: этот бит
    /// стоит ровно у денежных форматов отчёта и ни у одного текстового.
    pub const NUMBER_FORMAT: u32 = 1 << 24;
    /// Строка СКРЫТА. Взводится не пользователем, а свёрнутой группой:
    /// `НачатьГруппуСтрок(Имя, Ложь)` даёт своим строкам формат `{131072,1}`
    /// (измерено).
    pub const ROW_HIDDEN: u32 = 1 << 17;
    /// Три бита ячейки со значением; вместе дают маску 46137344.
    pub const VALUE_A: u32 = 1 << 22;
    pub const VALUE_B: u32 = 1 << 23;
    pub const VALUE_C: u32 = 1 << 25;
}

/// `ГоризонтальноеПоложение`. Коды измерены перебором членов перечисления;
/// они не по порядку объявления, поэтому выводить их из имени нельзя.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HAlign {
    Auto,
    Left,
    Center,
    Right,
}

impl HAlign {
    pub(crate) fn from_code(code: i64) -> Option<Self> {
        match code {
            0 => Some(HAlign::Left),
            2 => Some(HAlign::Right),
            5 => Some(HAlign::Auto),
            6 => Some(HAlign::Center),
            _ => None,
        }
    }

    pub(crate) fn code(self) -> i64 {
        match self {
            HAlign::Left => 0,
            HAlign::Right => 2,
            HAlign::Auto => 5,
            HAlign::Center => 6,
        }
    }
}

/// `ВертикальноеПоложение`. Члена `Авто` у него НЕТ — платформа отвечает
/// «Поле объекта не обнаружено» (измерено).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VAlign {
    Top,
    Center,
    Bottom,
}

impl VAlign {
    pub(crate) fn from_code(code: i64) -> Option<Self> {
        match code {
            0 => Some(VAlign::Top),
            8 => Some(VAlign::Bottom),
            24 => Some(VAlign::Center),
            _ => None,
        }
    }

    pub(crate) fn code(self) -> i64 {
        match self {
            VAlign::Top => 0,
            VAlign::Bottom => 8,
            VAlign::Center => 24,
        }
    }
}

/// Элемент палитры форматов: набор пар «бит — значение», отсортированный по
/// возрастанию бита. Порядок значений в файле именно такой (измерено на
/// `{257,0,6}`), поэтому он и держится сортировкой, а не порядком задания.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Format(Vec<(u32, i64)>);

impl Format {
    pub(crate) fn with(mut self, bit: u32, value: i64) -> Self {
        self.0.push((bit, value));
        self.0.sort_by_key(|(b, _)| *b);
        self
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub(crate) fn render(&self) -> String {
        let mask: u32 = self.0.iter().map(|(b, _)| b).sum();
        let mut out = format!("{{{mask}");
        for (_, v) in &self.0 {
            out.push_str(&format!(",{v}"));
        }
        out.push('}');
        out
    }
}

/// Палитра форматов документа: 1-based, индекс 0 — «формат не задан».
#[derive(Debug, Default)]
pub(crate) struct Palette(Vec<Format>);

impl Palette {
    /// Одинаковые форматы переиспользуются, а не дублируются — измерено:
    /// две колонки одной ширины ссылаются на ОДИН элемент палитры.
    pub(crate) fn intern(&mut self, f: Format) -> usize {
        if f.is_empty() {
            return 0;
        }
        if let Some(i) = self.0.iter().position(|e| *e == f) {
            return i + 1;
        }
        self.0.push(f);
        self.0.len()
    }
}

/// Текст MXL целиком (без заголовка и BOM), с `ПС` в качестве разделителя —
/// в `ВК`+`ПС` он превращается один раз, при записи в файл.
pub(crate) fn mxl_body(doc: &SpreadDocData) -> String {
    let mut palette = Palette::default();
    let mut colors = ColorPalette::default();
    let mut fonts = FontPalette::default();
    let mut lines = LinePalette::default();

    // Порядок занятия палитры определяет НОМЕРА форматов, а значит и байты
    // файла. Он снят встречной проверкой: платформа прочитала наш документ и
    // переписала его своими руками, разойдясь с нами ровно в нумерации, —
    // обход идёт по строкам (формат строки, затем форматы её ячеек), и лишь
    // ПОСЛЕ всех строк заводятся форматы колонок.
    let mut row_formats: BTreeMap<u32, usize> = BTreeMap::new();
    let mut cell_formats: BTreeMap<(u32, u32), usize> = BTreeMap::new();
    // Строка свёрнутой группы прячется битом формата, а не флагом у самой
    // группы, поэтому формат строки собирается из двух источников.
    let hidden = |r: u32| {
        doc.row_groups
            .iter()
            .any(|g| g.collapsed && r >= g.r1 && r <= g.r2)
    };
    for (&r, row) in &doc.rows {
        let mut f = Format::default();
        if let Some(h) = row.height {
            f = f.with(bits::ROW_HEIGHT, h * ROW_HEIGHT_SCALE);
        }
        if hidden(r) {
            f = f.with(bits::ROW_HIDDEN, 1);
        }
        if !f.is_empty() {
            row_formats.insert(r, palette.intern(f));
        }
        for (&c, cell) in &row.cells {
            let f = cell.format(&mut colors, &mut fonts, &mut lines);
            if !f.is_empty() {
                cell_formats.insert((r, c), palette.intern(f));
            }
        }
    }
    let col_formats: BTreeMap<u32, usize> = doc
        .col_widths
        .iter()
        .map(|(&c, &w)| {
            (
                c,
                palette.intern(Format::default().with(bits::COL_WIDTH, w * COL_WIDTH_SCALE)),
            )
        })
        .collect();

    // Ширины колонок ДОПОЛНИТЕЛЬНЫХ наборов тоже живут в палитре форматов,
    // и в файле платформы они идут последними.
    let mut set_formats: BTreeMap<(String, u32, i64), usize> = BTreeMap::new();
    for set in &doc.column_sets {
        for (&c, &w) in &set.widths {
            let index = palette.intern(Format::default().with(bits::COL_WIDTH, w));
            set_formats.insert((set.id.clone(), c, w), index);
        }
    }

    // Формат рисунка занимает палитру ПОСЛЕ форматов колонок — измерено на
    // документе, где есть и заданная ширина колонки, и рисунок.
    let drawing_format = if doc.drawings.is_empty() {
        0
    } else {
        colors.intern_style(-10);
        palette.intern(
            Format::default()
                .with(bits::V_ALIGN, 0)
                .with(bits::BACK_COLOR, 2),
        )
    };

    let mut out = String::new();
    out.push_str("{8,1,12,\n");
    out.push_str("{\"#\",\"\",1,1,\"#\",\"Язык по умолчанию\",\"Язык по умолчанию\",0},\n");
    out.push_str("{128,72},\n");
    out.push_str(&lines.render());
    for _ in 0..5 {
        out.push_str("{0,0},\n");
    }

    // --- строки и ячейки ---------------------------------------------
    // Пишутся ВСЕ заведённые строки и ячейки, включая пустые: объединение
    // заводит их намеренно, и платформа их сохраняет (`{0,0},` — ячейка без
    // маски и без формата).
    let filled: Vec<(&u32, &RowData)> = doc.rows.iter().collect();
    // Заголовок строки — `индекс, формат, число ячеек, индекс первой
    // колонки`. Физические переводы строк расставлены не по логическим
    // границам: заголовок ПЕРВОЙ строки продолжает строку файла со счётчиком
    // строк, а заголовок каждой следующей приписан к последней ячейке
    // предыдущей — ровно так пишет платформа, и байтовое совпадение с ней
    // держится на этом.
    // Индекс первой колонки в заголовке ЕСТЬ только когда ячейки есть:
    // строка, у которой задана лишь высота, пишется тремя полями (измерено).
    let header = |r: u32, cells: &[(u32, &CellData)]| {
        let fmt = row_formats.get(&r).copied().unwrap_or(0);
        match cells.first() {
            Some((c, _)) => format!("{},{},{},{},", r, fmt, cells.len(), c),
            None => format!("{},{},0,", r, fmt),
        }
    };
    let content: Vec<(u32, Vec<(u32, &CellData)>)> = filled
        .iter()
        .map(|&(&r, row)| {
            let cells = row
                .cells
                .iter()
                .map(|(&c, cell)| (c, cell))
                .collect::<Vec<_>>();
            (r, cells)
        })
        .collect();

    out.push_str(&format!("{{0,0}},0,2,{},", content.len()));
    // Заголовки строк БЕЗ ячеек прицепляются к тому же месту, что и заголовок
    // следующей содержательной строки, — своей позиции у них нет.
    let mut idx = 0;
    while idx < content.len() {
        let (r, cells) = &content[idx];
        out.push_str(&header(*r, cells));
        if !cells.is_empty() {
            break;
        }
        idx += 1;
    }
    out.push('\n');

    while idx < content.len() {
        let (r, cells) = &content[idx];
        let (r, cells) = (*r, cells.clone());
        for (j, (c, cell)) in cells.iter().enumerate() {
            let fmt = cell_formats.get(&(r, *c)).copied().unwrap_or(0);
            // Маска складывается из того, что у ячейки задано; порядок
            // полей в записи — по возрастанию бита, как и у форматов.
            let mut mask = 0;
            if cell.value.is_some() {
                mask |= 2;
            }
            if cell.detail.is_some() {
                mask |= 4;
            }
            if !cell.detail_param.is_empty() {
                mask |= 8;
            }
            if cell.has_text() {
                mask |= 16;
            }
            if mask & 12 != 0 {
                out.push_str(&format!("{{{mask},{fmt},"));
                if let Some(detail) = &cell.detail {
                    out.push_str(&format!("\n{{\"S\",{}}},", quoted(detail)));
                }
                if !cell.detail_param.is_empty() {
                    out.push_str(&format!("{},", quoted(&cell.detail_param)));
                }
                if cell.has_text() {
                    let (lang, value) = if cell.parameter.is_empty() {
                        ("#", &cell.text)
                    } else {
                        ("", &cell.parameter)
                    };
                    out.push_str("\n{1,1,\n");
                    out.push_str(&format!("{{{},{}}}\n", quoted(lang), quoted(value)));
                    out.push_str("},0");
                }
                out.push_str("},");
            } else if let Some(presentation) = &cell.value {
                // Маска 2 вместо 16, и вместо локализованной строки — пара
                // «тег типа, представление». Хвостового нуля тут НЕТ.
                out.push_str(&format!("{{2,{fmt},\n"));
                out.push_str(&format!("{{\"S\",{}}}\n", quoted(presentation)));
                out.push_str("},");
            } else if cell.has_text() {
                let (lang, value) = if cell.parameter.is_empty() {
                    ("#", &cell.text)
                } else {
                    ("", &cell.parameter)
                };
                out.push_str(&format!("{{16,{fmt},\n{{1,1,\n"));
                out.push_str(&format!("{{{},{}}}\n", quoted(lang), quoted(value)));
                out.push_str("},0},");
            } else {
                // Ячейка с одним оформлением: текстового блока нет вовсе.
                out.push_str(&format!("{{0,{fmt}}},"));
            }
            if let Some((next, _)) = cells.get(j + 1) {
                // Внутри строки перед каждой следующей ячейкой стоит её
                // индекс колонки.
                out.push_str(&format!("{},", next));
            } else {
                idx += 1;
                while idx < content.len() {
                    let (r2, cells2) = &content[idx];
                    out.push_str(&header(*r2, cells2));
                    if !cells2.is_empty() {
                        break;
                    }
                    idx += 1;
                }
            }
            out.push('\n');
        }
    }

    // --- колонки -------------------------------------------------------
    let mut cols = format!("{{{},0,00000000-0000-0000-0000-000000000000,", doc.width);
    if col_formats.is_empty() {
        cols.push_str("0}");
    } else {
        cols.push_str(&format!("{}", col_formats.len()));
        for (&c, &f) in &col_formats {
            cols.push_str(&format!(",{},{}", c, f));
        }
        cols.push('}');
    }
    // Между высотой и объединениями — четыре нуля, затем секция ГРУПП
    // СТРОК (счётчик и блоки, каждый со своим замыкающим -1), затем ещё
    // три нуля. Рисунки живут в этих же четырёх полях, но их мы не пишем.
    out.push_str(&format!("{},{},", cols, doc.height));
    if doc.column_sets.is_empty() {
        out.push_str("0,");
    } else {
        out.push_str(&format!("{},\n", doc.column_sets.len()));
        let sets: Vec<String> = doc
            .column_sets
            .iter()
            .map(|set_no| {
                let refs: Vec<(u32, usize)> = set_no
                    .widths
                    .iter()
                    .filter_map(|(&c, &w)| Some((c, *set_formats.get(&(set_no.id.clone(), c, w))?)))
                    .collect();
                let mut s = format!("{{{},0,{},", set_no.count, set_no.id);
                if refs.is_empty() {
                    s.push_str("0}");
                } else {
                    s.push_str(&format!("{}", refs.len()));
                    for (c, fmt) in refs {
                        s.push_str(&format!(",{c},{fmt}"));
                    }
                    s.push('}');
                }
                s
            })
            .collect();
        out.push_str(&sets.join(",\n"));
        out.push_str(",\n");
    }
    if doc.row_sets.is_empty() {
        out.push_str("0,");
    } else {
        out.push_str(&format!("{}", doc.row_sets.len()));
        for (r, set_no) in &doc.row_sets {
            out.push_str(&format!(",{r},{set_no}"));
        }
        out.push(',');
    }
    if doc.drawings.is_empty() {
        out.push_str("0,0,");
    } else {
        // Первое поле — сколько номеров выдано, второе — сколько рисунков
        // сейчас. У нас они совпадают: удаления рисунков нет.
        out.push_str(&format!("{0},{0},", doc.drawings.len()));
        let records: Vec<String> = doc
            .drawings
            .iter()
            .enumerate()
            .map(|(i, d)| {
                let (c1, r1, c2, r2, oc1, or1, oc2, or2) = doc.drawing_cells(d);
                format!(
                    "\n{{\n{{0,{drawing_format}}},2,{c1},{r1},{oc1},{or1},{c2},{r2},{oc2},{or2},{},0}}",
                    i + 1
                )
            })
            .collect();
        out.push_str(&records.join(","));
        out.push(',');
    }
    if doc.row_groups.is_empty() {
        out.push_str("0,");
    } else {
        out.push_str(&format!("{},", doc.row_groups.len()));
        let blocks: Vec<String> = doc.row_groups.iter().map(RowGroup::render).collect();
        out.push_str(&blocks.join(","));
        out.push(',');
    }
    out.push_str("0,0,0,\n");

    // --- объединения ---------------------------------------------------
    // Три СПИСКА подряд: обычные прямоугольники, объединения целых строк
    // (колонка в записи -1) и объединения целых колонок (строка -1, и запись
    // длиннее на GUID набора колонок). Записи отсортированы по строке, затем
    // по колонке — не по порядку вызовов `Объединить`.
    let list = |records: Vec<String>| {
        if records.is_empty() {
            return "{0},\n".to_string();
        }
        let mut out = format!("{{{},\n", records.len());
        out.push_str(&records.join(",\n"));
        out.push_str("\n},\n");
        out
    };

    let mut merges = doc.merges.clone();
    merges.sort();
    out.push_str(&list(
        merges
            .iter()
            .map(|m| format!("{{{}}}", m.render()))
            .collect(),
    ));

    let mut row_merges = doc.row_merges.clone();
    row_merges.sort();
    out.push_str(&list(
        row_merges
            .iter()
            .map(|(r1, r2)| format!("{{-1,{r1},-1,{r2},0}}"))
            .collect(),
    ));

    let mut col_merges = doc.col_merges.clone();
    col_merges.sort();
    out.push_str(&list(
        col_merges
            .iter()
            .map(|(c1, c2)| format!("{{{c1},-1,{c2},-1,0,00000000-0000-0000-0000-000000000000}}"))
            .collect(),
    ));

    // --- именованные области ------------------------------------------
    // Четвёртый список подряд. Запись — имя и тело `{1,{<область>},0}`;
    // счётчик впереди, разделитель между записями — запятая.
    if doc.names.is_empty() {
        out.push_str("{0},\"\",\n");
    } else {
        out.push_str(&format!("{{{},", doc.names.len()));
        let records: Vec<String> = doc
            .names
            .iter()
            .map(|(name, area)| format!("{},\n{{1,\n{{{}}},0}}", quoted(name), area.render()))
            .collect();
        out.push_str(&records.join(","));
        out.push_str("\n},\"\",\n");
    }

    // --- параметры страницы ---------------------------------------------
    // Это СПИСОК пар «идентификатор — значение», отсортированный по
    // идентификатору, а не запись с фиксированными полями: `ОриентацияСтраницы
    // .Ландшафт` добавляет пару `1 -> 2`, `МасштабПечати = 80` — пару `2 -> 80`,
    // а шесть пар `6..11` есть всегда (измерено).
    //
    // Четыре из них — ПОЛЯ СТРАНИЦЫ в сотых долях миллиметра, и порядок
    // идентификаторов не тот, что у свойств: документ с полями 33, 31, 34
    // и 32 мм (сверху, слева, снизу, справа) дал `6 -> 3300, 7 -> 3100,
    // 8 -> 3400, 9 -> 3200` (измерено на 8.3.27). Что значат пары 10 и 11,
    // не измерено: у документа с любыми полями обе равны 1000.
    let mm100 = |mm: f64| (mm * 100.0).round() as i64;
    let mut page: Vec<(i64, i64)> = Vec::new();
    if doc.landscape {
        page.push((1, 2));
    }
    if let Some(scale) = doc.print_scale {
        page.push((2, scale));
    }
    page.push((6, mm100(doc.margins.top)));
    page.push((7, mm100(doc.margins.left)));
    page.push((8, mm100(doc.margins.bottom)));
    page.push((9, mm100(doc.margins.right)));
    page.push((10, 1000));
    page.push((11, 1000));
    out.push_str(&format!("{{\n{{0,{},", page.len()));
    for (i, (id, value)) in page.iter().enumerate() {
        out.push_str(&format!("{id},\n{{\"N\",{value}}}"));
        if i + 1 < page.len() {
            out.push(',');
        }
    }
    out.push_str("\n}\n},\n");

    // --- область печати ---------------------------------------------------
    match doc.print_area {
        Some((r1, c1, r2, c2)) => {
            out.push_str(&format!(
                "{{3,{r1},{c1},{r2},{c2},00000000-0000-0000-0000-000000000000}},"
            ));
        }
        None => out.push_str("{0,-1,-1,-1,-1,00000000-0000-0000-0000-000000000000},"),
    }

    // --- палитра форматов ------------------------------------------------
    out.push_str("0,0,0,0,0,0,0,1,0,1,");
    out.push_str(&format!("{},", palette.0.len()));
    if !palette.0.is_empty() {
        // Каждый формат — на своей физической строке; хвост палитры
        // продолжает строку ПОСЛЕДНЕГО (измерено на документе с двумя
        // форматами).
        out.push('\n');
        for f in &palette.0 {
            out.push_str(&format!("{},\n", f.render()));
        }
        out.pop();
    }
    // Ноль шрифтов, три служебных нуля и РАЗМЕР палитры цветов — она есть
    // всегда, минимум из двух цветов стиля.
    out.push_str(&format!("{},", fonts.0.len()));
    if !fonts.0.is_empty() {
        out.push('\n');
        for font in &fonts.0 {
            out.push_str(&format!("{},\n", font.render()));
        }
        out.pop();
    }
    // Между шрифтами и цветами — служебный ноль, число ОПИСАНИЙ ТИПОВ и
    // число GUID. У документа с ячейками-значениями и того и другого по
    // одному, и они одни на весь документ, сколько бы таких ячеек ни было
    // (измерено на документе с двумя).
    let has_values = doc
        .rows
        .values()
        .any(|row| row.cells.values().any(CellData::has_value));
    if has_values {
        out.push_str("0,1,\n{\"Pattern\",\n{\"S\"}\n},1,381ed624-9217-4e63-85db-c4c3cb87daae,");
        out.push_str(&format!("{},\n", colors.size()));
    } else {
        out.push_str(&format!("0,0,0,{},\n", colors.size()));
    }
    out.push_str(&colors.render());
    out.push_str(",0,0,0,\"\",0,\n");

    // --- настройки печати и отображения ------------------------------------
    // Двадцать девять полей; значащие из них измерены по одному: 5-е —
    // `ОтображатьСетку`, 12-е — `ФиксацияСлева`, 13-е — `ФиксацияСверху`
    // ЧИСЛОМ (а не флагом), 27-е — признак, что фиксация сверху задана.
    // Признака для фиксации СЛЕВА нет: при `ФиксацияСлева = 2` 27-е поле
    // осталось нулём.
    let grid = i64::from(doc.show_grid);
    let fix_top = doc.fix_top.max(0);
    let fix_left = doc.fix_left.max(0);
    out.push_str(&format!(
        "{{3,0,0,100,{grid},1,0,1,1,0,0,{fix_left},{fix_top},0,0,0,0,0,0,0,0,\"\",0,0,0,0,{},0,0}},\n",
        i64::from(fix_top > 0),
    ));
    out.push_str("{0},0,0,0,1,0,0,0}");
    out
}

/// Байты файла целиком: заголовок, BOM и текст с `ВК`+`ПС`.
pub fn to_mxl_bytes(doc: &SpreadDocData) -> Vec<u8> {
    let body = mxl_body(doc).replace('\n', "\r\n");
    let mut out = Vec::with_capacity(MXL_HEADER.len() + BOM.len() + body.len());
    out.extend_from_slice(MXL_HEADER);
    out.extend_from_slice(BOM);
    out.extend_from_slice(body.as_bytes());
    out
}

/// Текстовое представление документа — `ТипФайлаТабличногоДокумента.TXT`.
/// Платформа пишет UTF-8 с BOM и `ВК`+`ПС`, колонки разделяет табуляцией
/// (измерено: документ из двух строк дал 27 байт «Привет»/«Число»).
pub fn to_txt_bytes(doc: &SpreadDocData) -> Vec<u8> {
    let mut text = String::new();
    for r in 0..doc.height {
        if r > 0 {
            text.push_str("\r\n");
        }
        if let Some(row) = doc.rows.get(&r) {
            let last = row.cells.keys().next_back().copied().unwrap_or(0);
            for c in 0..=last {
                if c > 0 {
                    text.push('\t');
                }
                text.push_str(&doc.cell_text(r, c));
            }
        }
    }
    let mut out = Vec::from(BOM);
    out.extend_from_slice(text.as_bytes());
    out
}

/// Записать документ в файл. Тип выбирается по члену
/// `ТипФайлаТабличногоДокумента`; без него — MXL (измерено: `Записать(путь)`
/// без второго аргумента дал файл, побайтно равный записи с MXL).
pub fn write_file(doc: &SpreadDocData, path: &str, kind: FileKind) -> RtResult<()> {
    let bytes = match kind {
        FileKind::Mxl => to_mxl_bytes(doc),
        FileKind::Txt => to_txt_bytes(doc),
        FileKind::Xlsx => crate::xlsx::to_xlsx_bytes(doc),
        FileKind::Pdf => crate::pdf_layout::to_pdf_bytes(doc)?,
    };
    std::fs::write(path, bytes).map_err(|e| bad(format!("не удалось записать {path}: {e}")))
}

/// Форматы, в которые этот интерпретатор умеет писать. Остальные члены
/// `ТипФайлаТабличногоДокумента` платформа знает, а мы — нет, и попытка
/// записи в них должна быть ошибкой, а не тихим MXL.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileKind {
    Mxl,
    Txt,
    Xlsx,
    Pdf,
}
