//! Модель документа: области, ячейки, оформление, палитры.

use super::*;

pub(crate) fn bad(what: impl Into<String>) -> RtError {
    RtError::Spread(what.into())
}

/// Заголовок файла: `MOXCEL` и шесть байт версии. Снят с платформы побайтно.
pub(crate) const MXL_HEADER: &[u8] = &[
    b'M', b'O', b'X', b'C', b'E', b'L', 0x00, 0x08, 0x00, 0x01, 0x00, 0x0C, 0x00,
];

/// Подпись формата: по ней файл и опознаётся.
pub(crate) const MXL_SIGNATURE: &[u8] = b"MOXCEL";

/// UTF-8 BOM: платформа ставит его после заголовка, до первой скобки.
pub(crate) const BOM: &[u8] = &[0xEF, 0xBB, 0xBF];

/// Ширина колонки по умолчанию во внутренних единицах (`{128,72}` в шапке).
/// В единицах API это 16 знаков: множитель 8 — измерен на `ШиринаКолонки = 25`,
/// давшей 200.
pub(crate) const COL_WIDTH_SCALE: i64 = 8;

/// Высота строки: `ВысотаСтроки = 30` даёт 120, множитель 4.
pub(crate) const ROW_HEIGHT_SCALE: i64 = 4;

/// Ячейка. Текст и параметр — одно и то же место файла, различаемое кодом
/// языка: `{"#","текст"}` — текст, `{"","имя"}` — параметр (измерено на
/// `Обл.Параметр = "ПарамЯчейки"`, которое затёрло собой текст ячейки).
///
/// Читается такая ячейка ТЕКСТОМ: `.Параметр` платформа отдаёт пустым даже
/// из своего собственного файла, и сразу после присваивания — тоже. То есть
/// у обычного (не макетного) документа `Параметр` только пишется, и наше
/// пустое значение при чтении — не потеря, а её поведение.
#[derive(Debug, Default, Clone)]
pub struct CellData {
    pub text: String,
    /// Имя параметра макета; непустое — ячейка параметрическая.
    pub parameter: String,
    pub h_align: Option<HAlign>,
    pub v_align: Option<VAlign>,
    /// `РазмещениеТекста.Переносить` — бит 16384 со значением 3.
    pub wrap: bool,
    pub text_color: Option<Color>,
    pub back_color: Option<Color>,
    pub font: Option<Font>,
    pub border_left: Option<Line>,
    pub border_top: Option<Line>,
    pub border_right: Option<Line>,
    pub border_bottom: Option<Line>,
    /// `Расшифровка` — тоже ПРЕДСТАВЛЕНИЕ, и в файле она стоит ПЕРЕД
    /// текстом ячейки (бит 4 маски).
    pub detail: Option<String>,
    /// `ПараметрРасшифровки` — бит 8; в отличие от расшифровки, пишется
    /// голой строкой, без обёртки с тегом типа.
    pub detail_param: String,
    /// У ячейки задан ЧИСЛОВОЙ формат. Не выводится из текста: платформа
    /// смотрит именно на формат, а не на то, похоже ли содержимое на число
    /// (измерено — «24 466» без формата уходит в XLSX строкой, а «3 575,89»
    /// с форматом становится числом).
    pub numeric: bool,
    /// Строка формата BSL из шаблона (например, «ЧДЦ=2»). Применяется
    /// при подстановке параметров: `format_value(value, spec)` вместо
    /// формата по умолчанию.
    pub format_spec: Option<String>,
    /// `СодержитЗначение` — в файле лежит ПРЕДСТАВЛЕНИЕ значения строкой.
    /// Тип не сохраняется вовсе: и число 42, и дата, и булево уходят как
    /// `{"S","<как печатается>"}` (измерено на пяти типах).
    pub value: Option<String>,
}

impl CellData {
    /// Ячейка без текста в файл всё равно попадает — как `{0,<формат>},`,
    /// без текстового блока вовсе (измерено). Поэтому «пустых» ячеек,
    /// которые можно было бы не писать, здесь нет: раз ячейка заведена,
    /// значит платформа её тоже завела бы.
    pub(crate) fn has_value(&self) -> bool {
        self.value.is_some()
    }

    pub(crate) fn has_text(&self) -> bool {
        !self.text.is_empty() || !self.parameter.is_empty()
    }

    /// Формат ячейки. Цвета попадают в СВОЮ палитру, поэтому она передаётся
    /// сюда: порядок занятия у неё тот же, что и у форматов, — по строкам,
    /// а внутри ячейки по возрастанию бита (цвет текста раньше цвета фона).
    pub(crate) fn format(
        &self,
        colors: &mut ColorPalette,
        fonts: &mut FontPalette,
        lines: &mut LinePalette,
    ) -> Format {
        let mut f = Format::default();
        if let Some(font) = &self.font {
            f = f.with(bits::FONT, fonts.intern(font));
        }
        for (bit, line) in [
            (bits::BORDER_LEFT, self.border_left),
            (bits::BORDER_TOP, self.border_top),
            (bits::BORDER_RIGHT, self.border_right),
            (bits::BORDER_BOTTOM, self.border_bottom),
        ] {
            if let Some(l) = line {
                f = f.with(bit, lines.intern(l));
            }
        }
        if let Some(a) = self.h_align {
            f = f.with(bits::H_ALIGN, a.code());
        }
        if let Some(a) = self.v_align {
            f = f.with(bits::V_ALIGN, a.code());
        }
        if let Some(c) = self.text_color {
            f = f.with(bits::TEXT_COLOR, colors.intern(c));
        }
        if let Some(c) = self.back_color {
            f = f.with(bits::BACK_COLOR, colors.intern(c));
        }
        if self.wrap {
            f = f.with(bits::TEXT_PLACEMENT, 3);
        }
        if self.value.is_some() {
            // Маска 46137344 со значениями 1,0,0 — ровно то, что пишет
            // платформа любой ячейке со значением. Смысл каждого из трёх
            // бит по отдельности НЕ измерен, поэтому они и заведены одной
            // тройкой, а не тремя догадками.
            f = f
                .with(bits::VALUE_A, 1)
                .with(bits::VALUE_B, 0)
                .with(bits::VALUE_C, 0);
        }
        f
    }
}

/// Строка документа. Высота задаётся не числом, а ссылкой на палитру
/// форматов, поэтому хранится в единицах API и переводится при записи.
#[derive(Debug, Default, Clone)]
pub struct RowData {
    pub height: Option<i64>,
    pub cells: BTreeMap<u32, CellData>,
}

/// Объединение ячеек: АБСОЛЮТНЫЕ границы прямоугольника, 0-based и
/// включительно.
///
/// В файле поля лежат в порядке `{колонка1, строка1, колонка2, строка2,
/// флаг}` — колонка ПЕРВАЯ, и это координаты, а не «добавочные» размеры.
/// Разница видна только когда левый верхний угол не в начале координат,
/// поэтому её легко не заметить: `Область(3, 1, 5, 1)` даёт `{0,2,0,4,0}`.
///
/// Флаг: 0 — объединение, 2 — ОТМЕНА объединения в этих ячейках (нужна,
/// чтобы разорвать объединение целых колонок в отдельной строке).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Merge {
    /// Сортировка записей в файле идёт по строке, затем по колонке, а не по
    /// порядку вызовов `Объединить` (измерено: объединения, заданные снизу
    /// вверх, легли сверху вниз), — поэтому поля объявлены в этом порядке.
    pub r1: u32,
    pub c1: u32,
    pub r2: u32,
    pub c2: u32,
    /// `Разъединить` поверх объединения целых колонок пишет не удаление, а
    /// отдельную запись с флагом 2.
    pub unmerge: bool,
}

impl Merge {
    /// Прямоугольник по абсолютным границам; порядок аргументов — как у
    /// `Область(СтрокаНач, КолонкаНач, СтрокаКон, КолонкаКон)`, но 0-based.
    pub fn new(r1: u32, c1: u32, r2: u32, c2: u32) -> Self {
        Merge {
            r1,
            c1,
            r2,
            c2,
            unmerge: false,
        }
    }

    pub(crate) fn render(&self) -> String {
        format!(
            "{},{},{},{},{}",
            self.c1,
            self.r1,
            self.c2,
            self.r2,
            u8::from(self.unmerge) * 2
        )
    }
}

/// Прямоугольник области ячеек, 0-based и включительно. Отдельный тип, а не
/// кортеж, — он живёт в `SpreadAreaObject` рядом со ссылкой на документ.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub r1: u32,
    pub c1: u32,
    pub r2: u32,
    pub c2: u32,
    /// Чем область была ЗАДАНА. `Область("R2:R3")` — это целые строки, и
    /// `Объединить` на ней даёт объединение строк, а не прямоугольник во всю
    /// ширину (измерено). Границы при этом всё равно заполнены — по ним
    /// работают текст и оформление.
    pub kind: AreaKind,
}

impl Rect {
    /// Из координат API (1-based). Перевёрнутые границы платформа принимает
    /// молча, поэтому они нормализуются, а не считаются ошибкой.
    pub fn from_api(r1: i64, c1: i64, r2: i64, c2: i64) -> Rect {
        let down = |v: i64| v.max(1) as u32 - 1;
        let (a, b) = (down(r1), down(r2));
        let (c, d) = (down(c1), down(c2));
        Rect {
            r1: a.min(b),
            c1: c.min(d),
            r2: a.max(b),
            c2: c.max(d),
            kind: AreaKind::Rect,
        }
    }
}

/// Линия границы. Стиль и толщина — единственное, что различается между
/// элементами палитры линий; остальные поля дескриптора постоянны и сняты с
/// платформы.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Line {
    pub style: LineStyle,
    /// Толщина: 1 тонкая, 2 толстая.
    pub width: i64,
}

/// Тип линии ячейки. Коды измерены перебором членов
/// `ТипЛинииЯчейкиТабличногоДокумента`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineStyle {
    Solid,
    Dotted,
    Double,
}

impl Line {
    pub fn new(style: LineStyle) -> Self {
        Line { style, width: 1 }
    }

    pub fn thick(mut self) -> Self {
        self.width = 2;
        self
    }

    pub(crate) fn render(&self) -> String {
        // GUID постоянен во всех снятых с платформы файлах — это не
        // идентификатор конкретной линии, а тег вида записи.
        format!(
            "1,\n{{4,0,\n{{0}},{},{},7,f527dc88-1d39-40b3-bcbb-d98b690ead68,0}},0",
            match self.style {
                LineStyle::Solid => 1,
                LineStyle::Dotted => 2,
                LineStyle::Double => 3,
            },
            self.width
        )
    }
}

/// Палитра линий: 0-based, живёт в ШАПКЕ файла, а не в хвосте, — в отличие
/// от палитр форматов, шрифтов и цветов.
#[derive(Debug, Default)]
pub(crate) struct LinePalette(pub(crate) Vec<Line>);

impl LinePalette {
    pub(crate) fn intern(&mut self, l: Line) -> i64 {
        match self.0.iter().position(|e| *e == l) {
            Some(i) => i as i64,
            None => {
                self.0.push(l);
                (self.0.len() - 1) as i64
            }
        }
    }

    pub(crate) fn render(&self) -> String {
        if self.0.is_empty() {
            return "{0},0,\n".to_string();
        }
        let records: Vec<String> = self.0.iter().map(Line::render).collect();
        format!("{{{},{}}},0,\n", self.0.len(), records.join(","))
    }
}

/// Цвет ячейки. В файле упакован как `B<<16 | G<<8 | R` — синий в старшем
/// байте, а не красный: `Новый Цвет(255, 0, 0)` даёт 255, а `Новый Цвет(0,
/// 0, 255)` — 16711680 (измерено).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        Color { r, g, b }
    }

    pub(crate) fn packed(self) -> i64 {
        (i64::from(self.b) << 16) | (i64::from(self.g) << 8) | i64::from(self.r)
    }
}

/// Палитра цветов. Первые ДВА места заняты цветами стиля (`{3,3,{-1}}` и
/// `{3,3,{-3}}`), они есть даже в пустом документе, поэтому первый заданный
/// цвет получает индекс 2 (измерено).
#[derive(Debug, Default)]
pub(crate) struct ColorPalette {
    /// Заданные пользователем цвета — пишутся как `{3,0,{<упаковка>}}`.
    pub(crate) colors: Vec<Color>,
    /// Цвета СТИЛЯ — `{3,3,{<код>}}`. Их требует формат рисунка, и в
    /// палитре они занимают место наравне с обычными.
    pub(crate) styles: Vec<i64>,
}

impl ColorPalette {
    const DEFAULTS: usize = 2;

    pub(crate) fn intern(&mut self, c: Color) -> i64 {
        let i = match self.colors.iter().position(|e| *e == c) {
            Some(i) => i,
            None => {
                self.colors.push(c);
                self.colors.len() - 1
            }
        };
        (i + Self::DEFAULTS) as i64
    }

    pub(crate) fn intern_style(&mut self, code: i64) -> i64 {
        let i = match self.styles.iter().position(|e| *e == code) {
            Some(i) => i,
            None => {
                self.styles.push(code);
                self.styles.len() - 1
            }
        };
        (i + Self::DEFAULTS + self.colors.len()) as i64
    }

    pub(crate) fn render(&self) -> String {
        let mut records = vec!["{3,3,\n{-1}\n}".to_string(), "{3,3,\n{-3}\n}".to_string()];
        records.extend(
            self.colors
                .iter()
                .map(|c| format!("{{3,0,\n{{{}}}\n}}", c.packed())),
        );
        records.extend(self.styles.iter().map(|s| format!("{{3,3,\n{{{s}}}\n}}")));
        records.join(",\n")
    }

    /// Размер вместе с предзаданными цветами стиля — именно он пишется в
    /// файл. `is_empty` здесь бессмысленно: пустой палитры не бывает.
    pub(crate) fn size(&self) -> usize {
        self.colors.len() + self.styles.len() + Self::DEFAULTS
    }
}

/// Шрифт ячейки. Кегль в файле умножен на десять, насыщенность записана
/// числом (400 обычная, 700 полужирная), а начертания — тремя отдельными
/// флагами. Остальные поля дескриптора при задании через `Новый Шрифт`
/// постоянны, и их значения сняты с платформы.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Font {
    pub face: String,
    /// Кегль в пунктах.
    pub size: i64,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strikeout: bool,
}

impl Font {
    pub fn new(face: &str, size: i64) -> Self {
        Font {
            face: face.to_string(),
            size,
            bold: false,
            italic: false,
            underline: false,
            strikeout: false,
        }
    }

    pub fn bold(mut self) -> Self {
        self.bold = true;
        self
    }

    pub fn italic(mut self) -> Self {
        self.italic = true;
        self
    }

    pub fn underline(mut self) -> Self {
        self.underline = true;
        self
    }

    pub fn strikeout(mut self) -> Self {
        self.strikeout = true;
        self
    }

    pub(crate) fn render(&self) -> String {
        format!(
            "{{7,0,703,{},0,0,0,{},{},{},{},0,0,0,0,34,{},1,100}}",
            self.size * 10,
            if self.bold { 700 } else { 400 },
            u8::from(self.italic),
            u8::from(self.underline),
            u8::from(self.strikeout),
            quoted(&self.face),
        )
    }
}

/// Палитра шрифтов: 0-based, без предзаданных элементов — в отличие от
/// палитры цветов.
#[derive(Debug, Default)]
pub(crate) struct FontPalette(pub(crate) Vec<Font>);

impl FontPalette {
    pub(crate) fn intern(&mut self, f: &Font) -> i64 {
        match self.0.iter().position(|e| e == f) {
            Some(i) => i as i64,
            None => {
                self.0.push(f.clone());
                (self.0.len() - 1) as i64
            }
        }
    }
}

/// Вид именованной области. Тип пишется числом первым полем записи, и он же
/// решает, какие координаты значащие: у строк колонки заменены на -1, у
/// колонок -1 стоит вместо строк.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AreaKind {
    /// Прямоугольник, тип 3.
    Rect,
    /// Целые строки, тип 1.
    Rows,
    /// Целые колонки, тип 2.
    Columns,
}

impl AreaKind {
    pub(crate) fn code(self) -> i64 {
        match self {
            AreaKind::Rows => 1,
            AreaKind::Columns => 2,
            AreaKind::Rect => 3,
        }
    }
}

/// Именованная область. Задаётся присваиванием `Область(...).Имя = "..."` —
/// у коллекции `Области` методов изменения нет вовсе (измерено: `Добавить`,
/// `Вставить`, `Удалить`, `Очистить` не существуют, есть только `Найти`,
/// `Получить` и `Индекс`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NamedArea {
    pub kind: AreaKind,
    pub r1: u32,
    pub c1: u32,
    pub r2: u32,
    pub c2: u32,
}

impl NamedArea {
    pub fn rect(r1: u32, c1: u32, r2: u32, c2: u32) -> Self {
        NamedArea {
            kind: AreaKind::Rect,
            r1,
            c1,
            r2,
            c2,
        }
    }

    pub fn rows(r1: u32, r2: u32) -> Self {
        NamedArea {
            kind: AreaKind::Rows,
            r1,
            c1: 0,
            r2,
            c2: 0,
        }
    }

    pub fn columns(c1: u32, c2: u32) -> Self {
        NamedArea {
            kind: AreaKind::Columns,
            r1: 0,
            c1,
            r2: 0,
            c2,
        }
    }

    /// Координаты идут в том же порядке, что и у объединений — колонка
    /// первой; незначащая ось заменяется на -1.
    pub(crate) fn render(&self) -> String {
        let (c1, c2) = match self.kind {
            AreaKind::Rows => (-1, -1),
            _ => (self.c1 as i64, self.c2 as i64),
        };
        let (r1, r2) = match self.kind {
            AreaKind::Columns => (-1, -1),
            _ => (self.r1 as i64, self.r2 as i64),
        };
        format!(
            "{},{},{},{},{},00000000-0000-0000-0000-000000000000",
            self.kind.code(),
            c1,
            r1,
            c2,
            r2
        )
    }
}

/// Группа строк. Уровень вложенности хранится ЧИСЛОМ, а не деревом: в
/// файле он так и лежит, и плоский список с уровнем ближе к тому, что
/// пишет платформа, чем восстановленная иерархия.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RowGroup {
    pub r1: u32,
    pub r2: u32,
    /// 0 — внешняя, 1 — вложенная в неё и так далее (измерено).
    pub level: u32,
    pub name: String,
    /// `НачатьГруппуСтрок(Имя, Ложь)` — группа свёрнута.
    pub collapsed: bool,
}

impl RowGroup {
    pub(crate) fn render(&self) -> String {
        // Имя — та же локализованная строка, что и у ячеек; пустое имя
        // пишется как `{1,0}`, без пары «язык, текст» (измерено).
        let name = if self.name.is_empty() {
            "{1,0}".to_string()
        } else {
            format!("{{1,1,\n{{\"#\",{}}}\n}}", quoted(&self.name))
        };
        format!(
            "\n{{{},{},{},\n{},{},0}},-1",
            self.r1,
            self.r2,
            self.level,
            name,
            u8::from(self.collapsed)
        )
    }
}

/// Ширина колонки в единицах ГЕОМЕТРИИ РИСУНКОВ — четвертях пункта.
/// Измерено: колонка по умолчанию (`ШиринаКолонки` читается как 9) занимает
/// 189, а заданная в 50 знаков — 1050. И то и другое даёт 21 на знак.
pub(crate) const COL_WIDTH_QP: i64 = 21;
/// Высота строки там же. Строка по умолчанию — 45, заданная в 60 пунктов —
/// 240, то есть те же четверти пункта, что и в палитре форматов.
pub(crate) const ROW_HEIGHT_QP: i64 = 4;
/// Высота строки по умолчанию в четвертях пункта: 11,25 пункта.
pub(crate) const DEFAULT_ROW_QP: i64 = 45;
/// Миллиметр в четвертях пункта: 1/288 дюйма при 25,4 мм в дюйме.
pub(crate) const MM_TO_QP: f64 = 288.0 / 25.4;

/// Набор колонок, каким он прочитан: имя, число колонок и ссылки на
/// форматы. Ширины подставляются позже — палитра форматов лежит в хвосте
/// файла.
pub(crate) type RawColumnSet = (String, u32, Vec<(u32, usize)>);

/// Разложение рисунка на сетку: границы в ячейках и четыре смещения.
pub(crate) type DrawingCells = (u32, u32, u32, u32, i64, i64, i64, i64);

/// Рисунок. Пока только прямоугольник: остальные типы платформа знает, но
/// их содержимое (текст, картинка, диаграмма) — отдельные разделы, которых
/// здесь нет.
#[derive(Debug, Clone, PartialEq)]
pub struct Drawing {
    /// Границы в МИЛЛИМЕТРАХ, как их задаёт API.
    pub left: f64,
    pub top: f64,
    pub width: f64,
    pub height: f64,
    pub name: String,
}

/// Дополнительный НАБОР КОЛОНОК со своей сеткой.
///
/// В одном документе их бывает несколько, и каждая строка привязана к
/// своему: у шапки отчёта одна разбивка, у табличной части другая. Ширины
/// хранятся во ВНУТРЕННИХ единицах (восьмых долях знака) — в единицах API
/// они дробные, и округление тут потеряло бы саму разбивку.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnSet {
    pub id: String,
    pub count: u32,
    pub widths: BTreeMap<u32, i64>,
}

impl ColumnSet {
    /// Границы колонок набора нарастающим итогом, включая нулевую.
    /// Колонка без своей ширины занимает умолчание из шапки файла.
    pub fn boundaries(&self) -> Vec<i64> {
        let mut out = vec![0];
        let mut x = 0;
        for c in 0..self.count {
            x += self.widths.get(&c).copied().unwrap_or(DEFAULT_COL_WIDTH);
            out.push(x);
        }
        out
    }
}

/// Ширина колонки по умолчанию во внутренних единицах — первое число шапки
/// `{128,72}`.
pub(crate) const DEFAULT_COL_WIDTH: i64 = 128;

/// Состояние `ТабличныйДокумент`.
#[derive(Debug, Default, Clone)]
pub struct SpreadDocData {
    pub(crate) rows: BTreeMap<u32, RowData>,
    /// Ширины колонок в единицах API; только для явно заданных.
    pub(crate) col_widths: BTreeMap<u32, i64>,
    pub(crate) merges: Vec<Merge>,
    /// Объединения ЦЕЛЫХ строк (`Область("R2:R3")`) — отдельный список, в
    /// файле он идёт следом за обычными объединениями, а колонка в записи
    /// заменена на -1.
    pub(crate) row_merges: Vec<(u32, u32)>,
    /// Объединения ЦЕЛЫХ колонок (`Область("C2:C4")`) — третий список, и
    /// запись в нём длиннее на GUID набора колонок.
    pub(crate) col_merges: Vec<(u32, u32)>,
    /// Дополнительные наборы колонок и привязка строк к ним: `(строка,
    /// номер набора)`. Своей сетки у строк в модели нет — ячейки лежат в
    /// общей, — но наборы нужны, чтобы разложить документ так же, как это
    /// делает платформа при выгрузке.
    pub(crate) column_sets: Vec<ColumnSet>,
    pub(crate) row_sets: Vec<(u32, u32)>,
    /// Сколько колонок ОБЪЯВЛЕНО в основном наборе. Это не то же, что
    /// занятая ширина: в отчёте с дополнительными наборами основной набор
    /// объявлен из одной колонки, а ячейки заняли пять.
    pub(crate) main_columns: u32,
    /// Рисунки в порядке добавления; номер в файле — позиция плюс один.
    pub(crate) drawings: Vec<Drawing>,
    /// Группы строк в порядке объявления и стек ещё не закрытых.
    pub(crate) row_groups: Vec<RowGroup>,
    pub(crate) open_groups: Vec<(String, bool, u32)>,
    /// Именованные области. `BTreeMap` не только за поиск: в файле записи
    /// идут ПО АЛФАВИТУ имени, а не в порядке присваивания (измерено — и на
    /// обходе коллекции `Области` порядок тот же).
    pub(crate) names: BTreeMap<String, NamedArea>,
    /// Номер последней занятой строки и колонки, 1-based. Не выводятся из
    /// `rows`: платформа их не уменьшает (см. модуль).
    pub(crate) height: u32,
    pub(crate) width: u32,
    /// `ОтображатьСетку`; по умолчанию Истина.
    pub show_grid: bool,
    pub fix_top: i64,
    pub fix_left: i64,
    /// `МасштабПечати` в процентах; `None` — не задан (в файле тогда нет
    /// пары с идентификатором 2 в списке параметров страницы).
    pub print_scale: Option<i64>,
    /// `ОриентацияСтраницы.Ландшафт` — пара с идентификатором 1 и значением 2.
    pub landscape: bool,
    /// Поля страницы в миллиметрах (`ПолеСлева` и три соседа). В MXL это
    /// пары `6..9` списка параметров страницы в сотых долях миллиметра,
    /// причём порядок идентификаторов свой — сверху, слева, снизу, справа
    /// (измерено на 8.3.27, файл `tests/conformance/pdf/probe-margins.mxl`);
    /// чтение возвращает их обратно. Что значат соседние пары 10 и 11, не
    /// измерено — они всегда пишутся как 1000. Те же поля доходят и до
    /// записи в PDF.
    pub margins: crate::pdf_layout::PageMargins,
    /// `ОбластьПечати` — прямоугольник 0-based; `None` пишется как
    /// `{0,-1,-1,-1,-1,...}`.
    pub print_area: Option<(u32, u32, u32, u32)>,
    /// Значения параметров макета: `Область.Параметры.Имя = Значение`.
    /// Ключи — имена параметров в верхнем регистре (сравнение
    /// регистронезависимо, как всё в BSL). Подстановка в ячейки с
    /// непустым `CellData::parameter` идёт при `Вывести`.
    pub params: BTreeMap<String, BslValue>,
}

impl SpreadDocData {
    pub fn new() -> Self {
        SpreadDocData {
            show_grid: true,
            ..Default::default()
        }
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    /// `НачатьГруппуСтрок`. Границы строк закрепляются при закрытии: до
    /// него неизвестно, сколько строк в группу попадёт.
    pub fn begin_row_group(&mut self, name: &str, collapsed: bool) {
        self.open_groups
            .push((name.to_string(), collapsed, self.height));
    }

    /// `ЗакончитьГруппуСтрок`. Лишний вызов игнорируется, как и у платформы
    /// — она на него не ругается.
    pub fn end_row_group(&mut self) {
        let Some((name, collapsed, start)) = self.open_groups.pop() else {
            return;
        };
        let level = self.open_groups.len() as u32;
        self.row_groups.push(RowGroup {
            r1: start,
            r2: self.height.saturating_sub(1).max(start),
            level,
            name,
            collapsed,
        });
        // Порядок в файле — по строке начала, затем по уровню: сначала
        // внешняя, потом вложенная (измерено на паре вложенных).
        self.row_groups.sort_by_key(|g| (g.r1, g.level));
    }

    /// Добавить прямоугольник. Имя по умолчанию — `D<номер>`, как у
    /// платформы (измерено).
    pub fn add_drawing(&mut self, left: f64, top: f64, width: f64, height: f64) -> usize {
        let number_of = self.drawings.len() + 1;
        let drawing = Drawing {
            left,
            top,
            width,
            height,
            name: format!("D{number_of}"),
        };
        self.drawings.push(drawing);
        self.refresh_drawing_bounds(number_of - 1);
        number_of
    }

    pub fn drawings(&self) -> &[Drawing] {
        &self.drawings
    }

    pub fn drawings_mut(&mut self) -> &mut Vec<Drawing> {
        &mut self.drawings
    }

    /// Пересчитать занятую область после правки геометрии рисунка: он
    /// растит документ, и это должно случиться сразу, а не при записи.
    pub fn refresh_drawing_bounds(&mut self, i: usize) {
        let Some(d) = self.drawings.get(i).cloned() else {
            return;
        };
        let (c1, r1, c2, r2, _, _, oc2, or2) = self.drawing_cells(&d);
        // Край РОВНО на границе ячейки её не занимает: рисунок шириной до
        // начала четвёртой колонки даёт документу три колонки, а не четыре
        // (измерено). Нулевой по размеру рисунок при этом всё равно стоит
        // в своей ячейке, поэтому вычитание только когда край дальше начала.
        let edge = |end: u32, start: u32, offset: i64| {
            if offset == 0 && end > start {
                end - 1
            } else {
                end
            }
        };
        self.touch(edge(r2, r1, or2), edge(c2, c1, oc2));
    }

    /// Разложение рисунка на сетку: `(колонка1, строка1, колонка2, строка2,
    /// и четыре смещения)`.
    pub(crate) fn drawing_cells(&self, d: &Drawing) -> DrawingCells {
        let to_qp = |mm: f64| (mm * MM_TO_QP).round() as i64;
        let (c1, oc1) = self.split_col(to_qp(d.left));
        let (r1, or1) = self.split_row(to_qp(d.top));
        let (c2, oc2) = self.split_col(to_qp(d.left + d.width));
        let (r2, or2) = self.split_row(to_qp(d.top + d.height));
        (c1, r1, c2, r2, oc1, or1, oc2, or2)
    }

    /// Начало строки в четвертях пункта — обратная свёртка к
    /// [`Self::split_row`].
    pub(crate) fn row_start(&self, row: u32) -> i64 {
        (0..row)
            .map(|r| {
                self.rows
                    .get(&r)
                    .and_then(|x| x.height)
                    .map_or(DEFAULT_ROW_QP, |h| h * ROW_HEIGHT_QP)
            })
            .sum()
    }

    pub(crate) fn col_start(&self, col: u32) -> i64 {
        (0..col)
            .map(|c| self.col_widths.get(&c).map_or(189, |w| w * COL_WIDTH_QP))
            .sum()
    }

    /// Разложение абсолютной координаты на «номер строки и смещение внутри
    /// неё». Высоты берутся ФАКТИЧЕСКИЕ: строка с заданной высотой сдвигает
    /// всё, что ниже (измерено).
    pub(crate) fn split_row(&self, qp: i64) -> (u32, i64) {
        let mut rest = qp;
        let mut row = 0u32;
        loop {
            let height = self
                .rows
                .get(&row)
                .and_then(|r| r.height)
                .map_or(DEFAULT_ROW_QP, |h| h * ROW_HEIGHT_QP);
            if rest < height {
                return (row, rest);
            }
            rest -= height;
            row += 1;
        }
    }

    /// То же по горизонтали.
    pub(crate) fn split_col(&self, qp: i64) -> (u32, i64) {
        let mut rest = qp;
        let mut column = 0u32;
        loop {
            let width = self
                .col_widths
                .get(&column)
                .map_or(189, |w| w * COL_WIDTH_QP);
            if rest < width {
                return (column, rest);
            }
            rest -= width;
            column += 1;
        }
    }

    pub fn row_groups(&self) -> &[RowGroup] {
        &self.row_groups
    }

    /// Объединения ЦЕЛЫХ строк: пары «первая, последняя».
    pub fn column_sets(&self) -> &[ColumnSet] {
        &self.column_sets
    }

    /// Привязки строк к наборам как есть.
    pub fn row_bindings(&self) -> &[(u32, u32)] {
        &self.row_sets
    }

    /// Набор колонок, по которому разложена строка.
    pub fn row_set(&self, row: u32) -> Option<&ColumnSet> {
        let number_of = self.row_sets.iter().find(|(r, _)| *r == row)?.1;
        self.column_sets.get(number_of as usize)
    }

    /// ФИЗИЧЕСКАЯ сетка документа — объединение границ всех наборов.
    ///
    /// Так же поступает платформа при выгрузке: она сводит разные разбивки
    /// в одну сетку по общим границам, отчего логическая ячейка занимает
    /// несколько физических колонок. На отчёте с наборами из 1, 2, 1 и 5
    /// колонок это даёт ровно восемь физических — как в её собственном
    /// XLSX.
    pub fn physical_grid(&self) -> Vec<i64> {
        if self.column_sets.is_empty() {
            return Vec::new();
        }
        let mut bounds: Vec<i64> = self
            .column_sets
            .iter()
            .flat_map(ColumnSet::boundaries)
            .collect();
        // Основной набор тоже участвует — по ОБЪЯВЛЕННОМУ числу колонок.
        let mut x = 0;
        bounds.push(0);
        for c in 0..self.main_columns {
            x += self
                .col_widths
                .get(&c)
                .map_or(DEFAULT_COL_WIDTH, |w| w * COL_WIDTH_SCALE);
            bounds.push(x);
        }
        bounds.sort_unstable();
        bounds.dedup();
        bounds
    }

    pub fn row_merges(&self) -> &[(u32, u32)] {
        &self.row_merges
    }

    /// Объединения ЦЕЛЫХ колонок.
    pub fn col_merges(&self) -> &[(u32, u32)] {
        &self.col_merges
    }

    pub fn merges(&self) -> &[Merge] {
        &self.merges
    }

    pub fn rows(&self) -> &BTreeMap<u32, RowData> {
        &self.rows
    }

    pub fn col_widths(&self) -> &BTreeMap<u32, i64> {
        &self.col_widths
    }

    /// Размеры задаются напрямую при чтении макета: там они объявлены, а не
    /// выводятся из содержимого.
    pub fn set_height(&mut self, height: u32) {
        self.height = self.height.max(height);
    }

    pub fn set_width(&mut self, width: u32) {
        self.width = self.width.max(width);
    }

    /// Расширить занятую область под ячейку (0-based).
    pub(crate) fn touch(&mut self, row: u32, col: u32) {
        self.height = self.height.max(row + 1);
        self.width = self.width.max(col + 1);
    }

    /// Текст ячейки; координаты 0-based. Отсутствующая ячейка — пустая
    /// строка, а не ошибка.
    pub fn cell_text(&self, row: u32, col: u32) -> String {
        self.rows
            .get(&row)
            .and_then(|r| r.cells.get(&col))
            .map_or(String::new(), |c| c.text.clone())
    }

    pub fn set_cell_text(&mut self, row: u32, col: u32, text: &str) {
        self.touch(row, col);
        let cell = self
            .rows
            .entry(row)
            .or_default()
            .cells
            .entry(col)
            .or_default();
        cell.text = text.to_string();
        cell.parameter.clear();
    }

    pub fn cell_parameter(&self, row: u32, col: u32) -> String {
        self.rows
            .get(&row)
            .and_then(|r| r.cells.get(&col))
            .map_or(String::new(), |c| c.parameter.clone())
    }

    pub fn set_cell_parameter(&mut self, row: u32, col: u32, name: &str) {
        self.touch(row, col);
        let cell = self
            .rows
            .entry(row)
            .or_default()
            .cells
            .entry(col)
            .or_default();
        cell.parameter = name.to_string();
        cell.text.clear();
    }

    /// Оформление ячейки: выравнивание и перенос. Ячейка заводится, даже
    /// если текста в ней нет, — платформа такую в файл пишет.
    pub fn set_cell_h_align(&mut self, row: u32, col: u32, align: HAlign) {
        self.touch(row, col);
        self.rows
            .entry(row)
            .or_default()
            .cells
            .entry(col)
            .or_default()
            .h_align = Some(align);
    }

    pub fn set_cell_v_align(&mut self, row: u32, col: u32, align: VAlign) {
        self.touch(row, col);
        self.rows
            .entry(row)
            .or_default()
            .cells
            .entry(col)
            .or_default()
            .v_align = Some(align);
    }

    pub fn set_cell_font(&mut self, row: u32, col: u32, font: Font) {
        self.touch(row, col);
        self.rows
            .entry(row)
            .or_default()
            .cells
            .entry(col)
            .or_default()
            .font = Some(font);
    }

    /// Границы ячейки. Стороны задаются по одной: `Обвести` с ОДНИМ
    /// аргументом ставит только левую (измерено), поэтому общего метода
    /// «обвести всё» здесь нет — он был бы догадкой.
    pub fn set_cell_border(
        &mut self,
        row: u32,
        col: u32,
        left: Option<Line>,
        top: Option<Line>,
        right: Option<Line>,
        bottom: Option<Line>,
    ) {
        self.touch(row, col);
        let cell = self
            .rows
            .entry(row)
            .or_default()
            .cells
            .entry(col)
            .or_default();
        if left.is_some() {
            cell.border_left = left;
        }
        if top.is_some() {
            cell.border_top = top;
        }
        if right.is_some() {
            cell.border_right = right;
        }
        if bottom.is_some() {
            cell.border_bottom = bottom;
        }
    }

    /// Положить в ячейку ЗНАЧЕНИЕ. Принимается уже готовое представление:
    /// форматирование живёт в `bsl-format`, который зависит от этого крейта,
    /// а не наоборот — как и у параметров текстового макета.
    pub fn set_cell_value(&mut self, row: u32, col: u32, presentation: &str) {
        self.touch(row, col);
        let cell = self
            .rows
            .entry(row)
            .or_default()
            .cells
            .entry(col)
            .or_default();
        cell.value = Some(presentation.to_string());
        cell.text.clear();
        cell.parameter.clear();
    }

    pub fn cell_value(&self, row: u32, col: u32) -> Option<String> {
        self.rows.get(&row)?.cells.get(&col)?.value.clone()
    }

    pub fn set_cell_detail(&mut self, row: u32, col: u32, presentation: &str) {
        self.touch(row, col);
        self.rows
            .entry(row)
            .or_default()
            .cells
            .entry(col)
            .or_default()
            .detail = Some(presentation.to_string());
    }

    pub fn set_cell_detail_param(&mut self, row: u32, col: u32, name: &str) {
        self.touch(row, col);
        self.rows
            .entry(row)
            .or_default()
            .cells
            .entry(col)
            .or_default()
            .detail_param = name.to_string();
    }

    /// Помечена ли ячейка числовым форматом.
    pub fn cell_numeric(&self, row: u32, col: u32) -> bool {
        self.rows
            .get(&row)
            .and_then(|r| r.cells.get(&col))
            .is_some_and(|c| c.numeric)
    }

    pub fn cell_detail(&self, row: u32, col: u32) -> Option<String> {
        self.rows.get(&row)?.cells.get(&col)?.detail.clone()
    }

    pub fn cell_detail_param(&self, row: u32, col: u32) -> String {
        self.rows
            .get(&row)
            .and_then(|r| r.cells.get(&col))
            .map(|c| c.detail_param.clone())
            .unwrap_or_default()
    }

    pub fn set_cell_text_color(&mut self, row: u32, col: u32, color: Color) {
        self.touch(row, col);
        self.rows
            .entry(row)
            .or_default()
            .cells
            .entry(col)
            .or_default()
            .text_color = Some(color);
    }

    pub fn set_cell_back_color(&mut self, row: u32, col: u32, color: Color) {
        self.touch(row, col);
        self.rows
            .entry(row)
            .or_default()
            .cells
            .entry(col)
            .or_default()
            .back_color = Some(color);
    }

    /// Пометить ячейку как имеющую числовой формат.
    pub fn set_cell_numeric(&mut self, row: u32, col: u32) {
        self.touch(row, col);
        self.rows
            .entry(row)
            .or_default()
            .cells
            .entry(col)
            .or_default()
            .numeric = true;
    }

    /// Строка формата BSL ячейки из шаблона.
    pub fn cell_format_spec(&self, row: u32, col: u32) -> Option<&str> {
        self.rows
            .get(&row)
            .and_then(|r| r.cells.get(&col))
            .and_then(|c| c.format_spec.as_deref())
    }

    /// Задать строку формата BSL ячейки.
    pub fn set_cell_format_spec(&mut self, row: u32, col: u32, spec: &str) {
        self.touch(row, col);
        self.rows
            .entry(row)
            .or_default()
            .cells
            .entry(col)
            .or_default()
            .format_spec = Some(spec.to_string());
    }

    pub fn set_cell_wrap(&mut self, row: u32, col: u32, wrap: bool) {
        self.touch(row, col);
        self.rows
            .entry(row)
            .or_default()
            .cells
            .entry(col)
            .or_default()
            .wrap = wrap;
    }

    pub fn set_row_height(&mut self, row: u32, height: i64) {
        self.touch(row, 0);
        self.rows.entry(row).or_default().height = Some(height);
    }

    pub fn set_col_width(&mut self, col: u32, width: i64) {
        self.col_widths.insert(col, width);
    }

    /// Объединение не только заводит запись, но и МЕНЯЕТ содержимое: в углу
    /// появляется ячейка (пустая, если её там не было), а все накрытые
    /// удаляются вместе с текстом (измерено: после объединения трёх ячеек с
    /// текстом читается только первый). Строки при этом остаются: строка,
    /// потерявшая все свои ячейки, пишется заголовком с нулём ячеек.
    ///
    /// Повторное объединение того же места записи не удваивает (измерено).
    pub fn merge(&mut self, m: Merge) {
        self.touch(m.r2, m.c2);
        for r in m.r1..=m.r2 {
            if let Some(row) = self.rows.get_mut(&r) {
                row.cells
                    .retain(|&c, _| c < m.c1 || c > m.c2 || (r == m.r1 && c == m.c1));
            }
        }
        self.rows
            .entry(m.r1)
            .or_default()
            .cells
            .entry(m.c1)
            .or_default();
        if !self.merges.contains(&m) {
            self.merges.push(m);
        }
    }

    /// `Разъединить` убирает запись целиком (измерено: список становится
    /// пустым), а не помечает её флагом. Флаг отмены нужен только поверх
    /// объединения целых колонок, и его ставит [`Self::unmerge_cells`].
    pub fn unmerge(&mut self, r1: u32, c1: u32, r2: u32, c2: u32) {
        self.merges
            .retain(|m| !(m.r1 == r1 && m.c1 == c1 && m.r2 == r2 && m.c2 == c2 && !m.unmerge));
        self.materialize(r1, c1, r2, c2);
    }

    /// Отмена объединения в отдельных ячейках — запись с флагом 2.
    pub fn unmerge_cells(&mut self, r1: u32, c1: u32, r2: u32, c2: u32) {
        let m = Merge {
            unmerge: true,
            ..Merge::new(r1, c1, r2, c2)
        };
        if !self.merges.contains(&m) {
            self.merges.push(m);
        }
        self.materialize(r1, c1, r2, c2);
    }

    /// Завести пустые ячейки во всём прямоугольнике. Так ведёт себя
    /// `Разъединить`: после него ячейки остаются, и ширина таблицы уже не
    /// сокращается (измерено).
    pub(crate) fn materialize(&mut self, r1: u32, c1: u32, r2: u32, c2: u32) {
        self.touch(r2, c2);
        for r in r1..=r2 {
            let row = self.rows.entry(r).or_default();
            for c in c1..=c2 {
                row.cells.entry(c).or_default();
            }
        }
    }

    /// Присвоить области имя. Пустое имя ИМЯ СНИМАЕТ — область исчезает из
    /// коллекции (измерено). Повторное имя, уже занятое другой областью,
    /// молча игнорируется: за именем остаётся та область, что была первой.
    pub fn set_area_name(&mut self, name: &str, area: NamedArea) {
        if name.is_empty() {
            return;
        }
        self.names.entry(name.to_string()).or_insert(area);
    }

    /// Снять имя. Отдельным методом, потому что в API это то же присваивание
    /// `Имя = ""`, а по смыслу — удаление.
    pub fn clear_area_name(&mut self, name: &str) {
        self.names.remove(name);
    }

    pub fn area_named(&self, name: &str) -> Option<&NamedArea> {
        self.names.get(name)
    }

    /// Обход именованных областей — нужен, чтобы по прямоугольнику найти
    /// его имя (`Область(...).Имя` читается обратно, измерено).
    pub fn names_iter(&self) -> impl Iterator<Item = (&String, &NamedArea)> {
        self.names.iter()
    }

    /// Объединение целых строк: `Область("R2:R3").Объединить()`. Ячейка
    /// заводится только в ПЕРВОЙ строке диапазона.
    pub fn merge_rows(&mut self, r1: u32, r2: u32) {
        self.touch(r2, 0);
        self.rows.entry(r1).or_default().cells.entry(0).or_default();
        if !self.row_merges.contains(&(r1, r2)) {
            self.row_merges.push((r1, r2));
        }
    }

    /// Объединение целых колонок: `Область("C2:C4").Объединить()`. Ячейка в
    /// первой колонке диапазона заводится в каждой УЖЕ существующей строке.
    pub fn merge_columns(&mut self, c1: u32, c2: u32) {
        self.touch(0, c2);
        for row in self.rows.values_mut() {
            row.cells.entry(c1).or_default();
        }
        if !self.col_merges.contains(&(c1, c2)) {
            self.col_merges.push((c1, c2));
        }
    }

    pub fn clear(&mut self) {
        let grid = self.show_grid;
        *self = SpreadDocData::new();
        self.show_grid = grid;
    }

    /// `Вывести(Документ)` — приписать содержимое снизу, со сдвигом строк на
    /// текущую высоту приёмника. Колонки не сдвигаются.
    pub fn append(&mut self, other: &SpreadDocData) {
        let shift = self.height;
        for (&r, row) in &other.rows {
            let target = self.rows.entry(r + shift).or_default();
            if row.height.is_some() {
                target.height = row.height;
            }
            for (&c, cell) in &row.cells {
                target.cells.insert(c, cell.clone());
            }
        }
        for m in &other.merges {
            let moved = Merge {
                r1: m.r1 + shift,
                r2: m.r2 + shift,
                ..*m
            };
            if !self.merges.contains(&moved) {
                self.merges.push(moved);
            }
        }
        for &(r1, r2) in &other.row_merges {
            let moved = (r1 + shift, r2 + shift);
            if !self.row_merges.contains(&moved) {
                self.row_merges.push(moved);
            }
        }
        for c in &other.col_merges {
            if !self.col_merges.contains(c) {
                self.col_merges.push(*c);
            }
        }
        for (&c, &w) in &other.col_widths {
            self.col_widths.entry(c).or_insert(w);
        }
        self.height += other.height;
        self.width = self.width.max(other.width);
    }

    /// Копия прямоугольного участка как самостоятельного документа —
    /// основа `ПолучитьОбласть`. Координаты 0-based, включительно.
    pub fn extract(&self, r1: u32, c1: u32, r2: u32, c2: u32) -> SpreadDocData {
        let mut out = SpreadDocData::new();
        out.show_grid = self.show_grid;
        for (&r, row) in self.rows.range(r1..=r2) {
            let mut copy = RowData {
                height: row.height,
                cells: BTreeMap::new(),
            };
            for (&c, cell) in row.cells.range(c1..=c2) {
                copy.cells.insert(c - c1, cell.clone());
            }
            out.rows.insert(r - r1, copy);
        }
        for (&c, &w) in self.col_widths.range(c1..=c2) {
            out.col_widths.insert(c - c1, w);
        }
        for m in &self.merges {
            if m.r1 >= r1 && m.r1 <= r2 && m.c1 >= c1 && m.c1 <= c2 {
                out.merges.push(Merge {
                    r1: m.r1 - r1,
                    c1: m.c1 - c1,
                    r2: m.r2.min(r2) - r1,
                    c2: m.c2.min(c2) - c1,
                    ..*m
                });
            }
        }
        out.height = r2.saturating_sub(r1) + 1;
        out.width = c2.saturating_sub(c1) + 1;
        out
    }
}
