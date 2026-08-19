//! `ТабличныйДокумент` и сериализация в MXL.
//!
//! # Что здесь ИЗМЕРЕНО на 8.3.27
//!
//! Формат MXL снят с платформы, а не взят из документации: серия документов,
//! отличающихся ОДНИМ свойством, записана `Записать(..., ТипФайлаТабличного\
//! Документа.MXL)` и сопоставлена попарно. Из этого следует всё, что ниже.
//!
//! MXL — НЕ двоичный формат. Это 13 байт заголовка `MOXCEL`, следом UTF-8 BOM
//! и дальше текст скобочной сериализации с переводами строк `ВК`+`ПС`.
//! Завершающего перевода строки в конце файла НЕТ.
//!
//! Строки берутся в кавычки, внутренняя кавычка УДВАИВАЕТСЯ. Запятые, скобки
//! и переводы строк внутри кавычек не экранируются никак — многострочный
//! текст ячейки лежит в файле как есть, разорванный на несколько физических
//! строк.
//!
//! # Модель
//!
//! Координаты в API 1-based (`Область(1, 1, 1, 1)` — левая верхняя ячейка), в
//! файле 0-based. Разрежённость настоящая: документ с единственной ячейкой в
//! `R3C5` пишет одну строку с индексом 2 и одну ячейку с индексом колонки 4,
//! а `ВысотаТаблицы`/`ШиринаТаблицы` при этом 3 и 5 (измерено).
//!
//! `ВысотаТаблицы` — это НЕ число заполненных строк, а номер последней
//! занятой, поэтому оно хранится отдельно от `rows` и не пересчитывается по
//! ним при удалении.
//!
//! # Области
//!
//! `ПолучитьОбласть` возвращает ТАБЛИЧНЫЙ ДОКУМЕНТ, а не область ячеек, и
//! это КОПИЯ: правка полученного на исходный не влияет (измерено). Отсюда и
//! `Вывести`, который область ячеек не принимает вовсе, — ему нужен
//! документ. Метода `ВывестиГоризонтально` у платформы НЕТ, вопреки обычным
//! примерам в сети; рядом с `Вывести` живёт `Присоединить`, и именно он
//! ставит область справа.
//!
//! Адрес принимается и числами, и строкой: `"R1C1:R2C2"`, `"R2:R3"`,
//! `"C1:C2"`, `"R1C1"`, `"C1"`. Область целых строк отдаётся во всю ширину
//! документа, целых колонок — во всю высоту, и ВИД области при этом
//! запоминается: `Объединить` на `"R2:R3"` даёт объединение строк, а не
//! прямоугольник, — то есть попадает в другой список файла.
//!
//! Именованные области заводятся ТОЛЬКО присваиванием `Область(...).Имя`:
//! у коллекции `Области` нет ни `Добавить`, ни `Удалить` — лишь `Найти`,
//! `Получить` и `Индекс`. Пустое имя область снимает, повторное имя молча
//! игнорируется (за именем остаётся первая область), а `ПолучитьОбласть`
//! понимает пересечение `"Высота|Ширина"` — то самое, чем режут ценники.
//! В отличие от `Параметр`, имя читается обратно и переживает запись в файл.
//!
//! # Рисунки
//!
//! Живут в прогоне нулей сразу за дескриптором колонок и высотой: два нуля,
//! затем признак и счётчик, затем блок записей, затем ещё четыре нуля.
//! Запись — `{{0,<номер>},2,<колонка1>,<строка1>,<смещК1>,<смещС1>,
//! <колонка2>,<строка2>,<смещК2>,<смещС2>,1,0}`.
//!
//! **Координата — это НЕ пиксели и не миллиметры**, а четверти пункта
//! (1/288 дюйма), причём разложенные на «номер колонки плюс смещение внутри
//! неё»: абсолютное положение равно `колонка × 189 + смещение` по
//! горизонтали и `строка × 45 + смещение` по вертикали. API при этом
//! говорит в МИЛЛИМЕТРАХ и округляет к ближайшей четверти пункта, поэтому
//! заданные 10 мм читаются обратно как 9,96597222222222, а 100 мм — как
//! 100,0125. Проверено на десяти значениях, все сходятся точно.
//!
//! Рисунок РАСТИТ документ: `Верх = 100` мм превращает документ 1x1 в 26x1,
//! потому что он занимает ячейки под собой.
//!
//! Типы рисунков у 8.3.27: `Прямоугольник`, `Текст`, `Картинка`,
//! `Диаграмма`, `ДиаграммаГанта`, `СводнаяДиаграмма`, `Объект`. `Линии`,
//! `Овала`, `ГруппыКартинок` и `Комментария` в перечислении НЕТ, вопреки
//! распространённым примерам. Имя по умолчанию — `D1`.
//!
//! Именованный рисунок попадает в ТОТ ЖЕ список именованных областей, но с
//! другим телом: `{2,<номер рисунка>,0}` против `{1,{<область>},0}` у
//! ячеек, — это `NamedItemDrawing` против `NamedItemCells` из
//! XML-спецификации макетов. У коллекции `Рисунки`, в отличие от
//! `Области`, есть и `Добавить`, и `Удалить`, и `Очистить`.
//!
//! # Что уже РАСШИФРОВАНО, но ещё не пишется
//!
//! Это не догадки, а замеры с тех же артефактов; они ждут своих палитр.
//! Эталоны лежат в `tests/conformance/mxl/`, воспроизводить их пока нечем.
//!
//! * **Содержимое рисунков.** Прямоугольник пишется, читается и доступен из
//!   BSL целиком, а вот текст надписи, картинка и диаграмма — это свои
//!   разделы формата, которых здесь нет.
//!
//! Геометрия рисунка ЗАЩЁЛКИВАЕТСЯ на сетку четвертей пункта: заданные
//! 10 мм читаются обратно как 9,96597222222222 — и у платформы, и у нас,
//! до последней цифры. Край ровно на границе ячейки её не занимает: рисунок
//! до начала четвёртой колонки даёт документу три колонки, а не четыре.
//!
//! Все палитры дедуплицируются: две ячейки с одинаковым оформлением делят
//! один элемент. Биты в одном формате КОМБИНИРУЮТСЯ, а значения идут по
//! возрастанию бита — `{2307,0,0,2,2}` это шрифт 0, левая граница 0,
//! горизонтальное положение 2 и цвет фона 2 в одной записи.
//!
//! # Чтение
//!
//! Разбор — зеркало записи и обязан меняться вместе с ней: разделы идут
//! подряд, без разметки, и читаются позиционно в том же порядке, в каком их
//! пишет `mxl_body`.
//!
//! Отказов больше нет: весь корпус эталонов, снятых с платформы,
//! разбирается. Тест на это и смотрит — не «не упало», а «разобралось всё».
//!
//! Мера полноты чтения — круговорот: каждый из 108
//! эталонов, которые наша ЗАПИСЬ воспроизводит побайтно, после чтения и
//! обратной записи даёт те же самые байты. Это покрывает и палитры
//! (шрифты, цвета, выравнивание, ширины и высоты), и все три списка
//! объединений, и именованные области, и настройки печати.
//!
//! # Встречная проверка
//!
//! Совпадение с эталонами доказывает только то, что снято. Поэтому
//! `tests/conformance/mxl/read-back.bsl` даёт платформе прочитать НАШИ файлы
//! и переписать их своими руками: длинный документ на 50 строк, разрежённый,
//! с несколькими объединениями, с оформлением и с текстом из кавычек,
//! запятых, скобок и переводов строк. Все пять возвращаются побайтно
//! такими же — то есть мы пишем не просто читаемый, а канонический файл.
//! Именно эта проверка и выявила порядок занятия палитры: разошлись ровно
//! номера форматов.

use std::collections::BTreeMap;

use bsl_rt::{RtError, RtResult};

fn bad(what: impl Into<String>) -> RtError {
    RtError::Spread(what.into())
}

/// Заголовок файла: `MOXCEL` и шесть байт версии. Снят с платформы побайтно.
const MXL_HEADER: &[u8] = &[
    b'M', b'O', b'X', b'C', b'E', b'L', 0x00, 0x08, 0x00, 0x01, 0x00, 0x0C, 0x00,
];

/// Подпись формата: по ней файл и опознаётся.
const MXL_SIGNATURE: &[u8] = b"MOXCEL";

/// UTF-8 BOM: платформа ставит его после заголовка, до первой скобки.
const BOM: &[u8] = &[0xEF, 0xBB, 0xBF];

/// Ширина колонки по умолчанию во внутренних единицах (`{128,72}` в шапке).
/// В единицах API это 16 знаков: множитель 8 — измерен на `ШиринаКолонки = 25`,
/// давшей 200.
const COL_WIDTH_SCALE: i64 = 8;

/// Высота строки: `ВысотаСтроки = 30` даёт 120, множитель 4.
const ROW_HEIGHT_SCALE: i64 = 4;

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
    fn has_value(&self) -> bool {
        self.value.is_some()
    }

    fn has_text(&self) -> bool {
        !self.text.is_empty() || !self.parameter.is_empty()
    }

    /// Формат ячейки. Цвета попадают в СВОЮ палитру, поэтому она передаётся
    /// сюда: порядок занятия у неё тот же, что и у форматов, — по строкам,
    /// а внутри ячейки по возрастанию бита (цвет текста раньше цвета фона).
    fn format(
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

    fn render(&self) -> String {
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

    fn render(&self) -> String {
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
struct LinePalette(Vec<Line>);

impl LinePalette {
    fn intern(&mut self, l: Line) -> i64 {
        match self.0.iter().position(|e| *e == l) {
            Some(i) => i as i64,
            None => {
                self.0.push(l);
                (self.0.len() - 1) as i64
            }
        }
    }

    fn render(&self) -> String {
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

    fn packed(self) -> i64 {
        (i64::from(self.b) << 16) | (i64::from(self.g) << 8) | i64::from(self.r)
    }
}

/// Палитра цветов. Первые ДВА места заняты цветами стиля (`{3,3,{-1}}` и
/// `{3,3,{-3}}`), они есть даже в пустом документе, поэтому первый заданный
/// цвет получает индекс 2 (измерено).
#[derive(Debug, Default)]
struct ColorPalette {
    /// Заданные пользователем цвета — пишутся как `{3,0,{<упаковка>}}`.
    colors: Vec<Color>,
    /// Цвета СТИЛЯ — `{3,3,{<код>}}`. Их требует формат рисунка, и в
    /// палитре они занимают место наравне с обычными.
    styles: Vec<i64>,
}

impl ColorPalette {
    const DEFAULTS: usize = 2;

    fn intern(&mut self, c: Color) -> i64 {
        let i = match self.colors.iter().position(|e| *e == c) {
            Some(i) => i,
            None => {
                self.colors.push(c);
                self.colors.len() - 1
            }
        };
        (i + Self::DEFAULTS) as i64
    }

    fn intern_style(&mut self, code: i64) -> i64 {
        let i = match self.styles.iter().position(|e| *e == code) {
            Some(i) => i,
            None => {
                self.styles.push(code);
                self.styles.len() - 1
            }
        };
        (i + Self::DEFAULTS + self.colors.len()) as i64
    }

    fn render(&self) -> String {
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
    fn size(&self) -> usize {
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

    fn render(&self) -> String {
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
struct FontPalette(Vec<Font>);

impl FontPalette {
    fn intern(&mut self, f: &Font) -> i64 {
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
    fn code(self) -> i64 {
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
    fn render(&self) -> String {
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
    fn render(&self) -> String {
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
const COL_WIDTH_QP: i64 = 21;
/// Высота строки там же. Строка по умолчанию — 45, заданная в 60 пунктов —
/// 240, то есть те же четверти пункта, что и в палитре форматов.
const ROW_HEIGHT_QP: i64 = 4;
/// Высота строки по умолчанию в четвертях пункта: 11,25 пункта.
const DEFAULT_ROW_QP: i64 = 45;
/// Миллиметр в четвертях пункта: 1/288 дюйма при 25,4 мм в дюйме.
const MM_TO_QP: f64 = 288.0 / 25.4;

/// Набор колонок, каким он прочитан: имя, число колонок и ссылки на
/// форматы. Ширины подставляются позже — палитра форматов лежит в хвосте
/// файла.
type RawColumnSet = (String, u32, Vec<(u32, usize)>);

/// Разложение рисунка на сетку: границы в ячейках и четыре смещения.
type DrawingCells = (u32, u32, u32, u32, i64, i64, i64, i64);

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
const DEFAULT_COL_WIDTH: i64 = 128;

/// Состояние `ТабличныйДокумент`.
#[derive(Debug, Default, Clone)]
pub struct SpreadDocData {
    rows: BTreeMap<u32, RowData>,
    /// Ширины колонок в единицах API; только для явно заданных.
    col_widths: BTreeMap<u32, i64>,
    merges: Vec<Merge>,
    /// Объединения ЦЕЛЫХ строк (`Область("R2:R3")`) — отдельный список, в
    /// файле он идёт следом за обычными объединениями, а колонка в записи
    /// заменена на -1.
    row_merges: Vec<(u32, u32)>,
    /// Объединения ЦЕЛЫХ колонок (`Область("C2:C4")`) — третий список, и
    /// запись в нём длиннее на GUID набора колонок.
    col_merges: Vec<(u32, u32)>,
    /// Дополнительные наборы колонок и привязка строк к ним: `(строка,
    /// номер набора)`. Своей сетки у строк в модели нет — ячейки лежат в
    /// общей, — но наборы нужны, чтобы разложить документ так же, как это
    /// делает платформа при выгрузке.
    column_sets: Vec<ColumnSet>,
    row_sets: Vec<(u32, u32)>,
    /// Сколько колонок ОБЪЯВЛЕНО в основном наборе. Это не то же, что
    /// занятая ширина: в отчёте с дополнительными наборами основной набор
    /// объявлен из одной колонки, а ячейки заняли пять.
    main_columns: u32,
    /// Рисунки в порядке добавления; номер в файле — позиция плюс один.
    drawings: Vec<Drawing>,
    /// Группы строк в порядке объявления и стек ещё не закрытых.
    row_groups: Vec<RowGroup>,
    open_groups: Vec<(String, bool, u32)>,
    /// Именованные области. `BTreeMap` не только за поиск: в файле записи
    /// идут ПО АЛФАВИТУ имени, а не в порядке присваивания (измерено — и на
    /// обходе коллекции `Области` порядок тот же).
    names: BTreeMap<String, NamedArea>,
    /// Номер последней занятой строки и колонки, 1-based. Не выводятся из
    /// `rows`: платформа их не уменьшает (см. модуль).
    height: u32,
    width: u32,
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
    fn drawing_cells(&self, d: &Drawing) -> DrawingCells {
        let to_qp = |mm: f64| (mm * MM_TO_QP).round() as i64;
        let (c1, oc1) = self.split_col(to_qp(d.left));
        let (r1, or1) = self.split_row(to_qp(d.top));
        let (c2, oc2) = self.split_col(to_qp(d.left + d.width));
        let (r2, or2) = self.split_row(to_qp(d.top + d.height));
        (c1, r1, c2, r2, oc1, or1, oc2, or2)
    }

    /// Начало строки в четвертях пункта — обратная свёртка к
    /// [`Self::split_row`].
    fn row_start(&self, row: u32) -> i64 {
        (0..row)
            .map(|r| {
                self.rows
                    .get(&r)
                    .and_then(|x| x.height)
                    .map_or(DEFAULT_ROW_QP, |h| h * ROW_HEIGHT_QP)
            })
            .sum()
    }

    fn col_start(&self, col: u32) -> i64 {
        (0..col)
            .map(|c| self.col_widths.get(&c).map_or(189, |w| w * COL_WIDTH_QP))
            .sum()
    }

    /// Разложение абсолютной координаты на «номер строки и смещение внутри
    /// неё». Высоты берутся ФАКТИЧЕСКИЕ: строка с заданной высотой сдвигает
    /// всё, что ниже (измерено).
    fn split_row(&self, qp: i64) -> (u32, i64) {
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
    fn split_col(&self, qp: i64) -> (u32, i64) {
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
    fn touch(&mut self, row: u32, col: u32) {
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
    fn materialize(&mut self, r1: u32, c1: u32, r2: u32, c2: u32) {
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

// --- сериализация MXL ---------------------------------------------------

/// Строка в кавычках с удвоением внутренней кавычки — единственное
/// экранирование, которое делает платформа.
fn quoted(s: &str) -> String {
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
mod bits {
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
    fn from_code(code: i64) -> Option<Self> {
        match code {
            0 => Some(HAlign::Left),
            2 => Some(HAlign::Right),
            5 => Some(HAlign::Auto),
            6 => Some(HAlign::Center),
            _ => None,
        }
    }

    fn code(self) -> i64 {
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
    fn from_code(code: i64) -> Option<Self> {
        match code {
            0 => Some(VAlign::Top),
            8 => Some(VAlign::Bottom),
            24 => Some(VAlign::Center),
            _ => None,
        }
    }

    fn code(self) -> i64 {
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
struct Format(Vec<(u32, i64)>);

impl Format {
    fn with(mut self, bit: u32, value: i64) -> Self {
        self.0.push((bit, value));
        self.0.sort_by_key(|(b, _)| *b);
        self
    }

    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    fn render(&self) -> String {
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
struct Palette(Vec<Format>);

impl Palette {
    /// Одинаковые форматы переиспользуются, а не дублируются — измерено:
    /// две колонки одной ширины ссылаются на ОДИН элемент палитры.
    fn intern(&mut self, f: Format) -> usize {
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
fn mxl_body(doc: &SpreadDocData) -> String {
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
        .map(|(&r, row)| {
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

// --- мост к значениям BSL -------------------------------------------------

use bsl_rt::{BslNumber, BslString, BslValue, CallContext, ObjectProtocol, TypeDescriptor, TypeId};
use std::cell::RefCell;
use std::rc::Rc;

fn number(v: &BslValue, what: &str) -> RtResult<i64> {
    match v {
        BslValue::Number(n) => n
            .to_i64_exact()
            .ok_or_else(|| bad(format!("{what}: ожидалось целое число"))),
        _ => Err(bad(format!("{what}: ожидалось число"))),
    }
}

fn int_value(n: i64) -> BslValue {
    BslValue::Number(BslNumber::from_i64(n))
}

/// Число с дробной частью — им наружу отдаются поля страницы
/// (`ТипЗнч(ТабДок.ПолеСлева)` — «Число», измерено).
///
/// `from_f64` отказывает только на нечисле и бесконечности, а в поле
/// попадает лишь проверенное [`number_f64`] либо сотая доля целого из MXL,
/// поэтому запасной ноль недостижим.
fn mm_value(mm: f64) -> BslValue {
    BslValue::Number(BslNumber::from_f64(mm).unwrap_or(BslNumber::ZERO))
}

/// Прочитать число, допуская дробное: `ПолеСлева = 12.7` — законное
/// значение, а `number` требует целого.
///
/// Строка тоже принимается, и это не вольность: платформа на
/// `ПолеСлева = "10"` кладёт в свойство ЧИСЛО 10 (измерено — `ТипЗнч`
/// после присваивания даёт «Число»), на `"12,7"` — 12,7, то есть
/// разделителем дробной части служит ЗАПЯТАЯ, а на `"не число"` отвечает
/// ошибкой. Точка тоже разбирается: у платформы это не проверялось, но
/// отказывать в записи из-за разделителя было бы хуже.
fn number_f64(v: &BslValue, what: &str) -> RtResult<f64> {
    let x = match v {
        BslValue::Number(n) => n.to_f64(),
        BslValue::Str(s) => s
            .to_string()
            .trim()
            .replace(',', ".")
            .parse::<f64>()
            .map_err(|_| bad(format!("{what}: строка не преобразуется в число")))?,
        _ => return Err(bad(format!("{what}: ожидалось число"))),
    };
    if x.is_finite() {
        Ok(x)
    } else {
        Err(bad(format!("{what}: ожидалось конечное число")))
    }
}

/// `ТабличныйДокумент`.
#[derive(Debug)]
pub struct SpreadDocumentObject {
    pub(crate) data: Rc<RefCell<SpreadDocData>>,
}

/// `ОбластьЯчеекТабличногоДокумента` — ссылка на прямоугольник в документе.
#[derive(Debug)]
pub struct SpreadAreaObject {
    data: Rc<RefCell<SpreadDocData>>,
    rect: Rect,
}

/// `КоллекцияРисунковТабличногоДокумента` — окно в тот же документ.
#[derive(Debug)]
pub struct SpreadDrawingsObject {
    data: Rc<RefCell<SpreadDocData>>,
}

/// `РисунокТабличногоДокумента` — документ и номер рисунка в нём.
#[derive(Debug)]
pub struct SpreadDrawingObject {
    data: Rc<RefCell<SpreadDocData>>,
    index: usize,
}

/// `ПараметрыМакетаТабличногоДокумента` — обёртка над теми же данными.
#[derive(Debug)]
pub struct SpreadParamsObject {
    data: Rc<RefCell<SpreadDocData>>,
}

/// Данные документа у значения — и у самого документа, и у его области.
fn data(v: &BslValue) -> Option<Rc<RefCell<SpreadDocData>>> {
    let object = v.object_ref()?;
    if let Some(document) = object.downcast_ref::<SpreadDocumentObject>() {
        return Some(document.data.clone());
    }
    object
        .downcast_ref::<SpreadAreaObject>()
        .map(|area| area.data.clone())
}

fn rect(v: &BslValue) -> Option<Rect> {
    v.object_ref()
        .and_then(|object| object.downcast_ref::<SpreadAreaObject>())
        .map(|area| area.rect)
}

/// Область ячеек — получатель `Значение`, которое ВМ перехватывает.
pub fn is_area(v: &BslValue) -> bool {
    v.object_ref()
        .is_some_and(|object| object.downcast_ref::<SpreadAreaObject>().is_some())
}

/// Положить представление значения в левую верхнюю ячейку области.
///
/// # Errors
///
/// Ошибка, если получатель — не область ячеек.
pub fn set_detail(obj: &BslValue, presentation: &str) -> RtResult<()> {
    let rect = rect(obj).ok_or_else(|| bad("Расшифровка: не область ячеек"))?;
    data(obj)
        .expect("область всегда при документе")
        .borrow_mut()
        .set_cell_detail(rect.r1, rect.c1, presentation);
    Ok(())
}

/// Положить представление значения в левую верхнюю ячейку области.
///
/// # Errors
///
/// Ошибка, если получатель — не область ячеек.
pub fn set_value(obj: &BslValue, presentation: &str) -> RtResult<()> {
    let rect = rect(obj).ok_or_else(|| bad("Значение: не область ячеек"))?;
    data(obj)
        .expect("область всегда при документе")
        .borrow_mut()
        .set_cell_value(rect.r1, rect.c1, presentation);
    Ok(())
}

pub fn is_spread_document(v: &BslValue) -> bool {
    v.object_ref()
        .is_some_and(|object| object.downcast_ref::<SpreadDocumentObject>().is_some())
}

pub fn new_document() -> BslValue {
    BslValue::new_object(SpreadDocumentObject {
        data: Rc::new(RefCell::new(SpreadDocData::new())),
    })
}

/// `Область(СтрокаНач, КолонкаНач, СтрокаКон, КолонкаКон)`. Строковая
/// адресация (`"R1C1:R2C3"`) пока не поддержана — платформа её принимает,
/// но здесь она нужна отдельным разбором.
pub fn region(obj: &BslValue, args: &[BslValue]) -> RtResult<BslValue> {
    let doc = data(obj).ok_or_else(|| bad("Область: не табличный документ"))?;
    let rect = match args {
        [r1, c1, r2, c2] => Rect::from_api(
            number(r1, "Область")?,
            number(c1, "Область")?,
            number(r2, "Область")?,
            number(c2, "Область")?,
        ),
        [address] => match address {
            BslValue::Str(s) => {
                let (h, w) = {
                    let d = doc.borrow();
                    (d.height(), d.width())
                };
                parse_address(&s.to_string(), h, w)
                    .ok_or_else(|| bad(format!("Область не найдена: {s}")))?
            }
            _ => return Err(bad("Область: ожидались координаты или адрес")),
        },
        _ => return Err(bad("Область: ожидалось 1 или 4 аргумента")),
    };
    Ok(BslValue::new_object(SpreadAreaObject { data: doc, rect }))
}

/// Адрес вида `R1C1`, `R1C1:R2C3`, `R2`, `R2:R3`, `C1`, `C1:C2` — набор,
/// который принимает платформа (измерено). Отсутствие оси означает «вся»,
/// и здесь это разворачивается в границы, а не в -1: модель областей у нас
/// прямоугольная.
fn parse_address(s: &str, height: u32, width: u32) -> Option<Rect> {
    fn part(s: &str) -> Option<(Option<i64>, Option<i64>)> {
        let up = s.trim().to_uppercase();
        let (mut row, mut col) = (None, None);
        let mut chars = up.chars().peekable();
        while let Some(c) = chars.next() {
            let mut number = String::new();
            while chars.peek().is_some_and(|d| d.is_ascii_digit()) {
                number.push(chars.next()?);
            }
            let n: i64 = number.parse().ok()?;
            match c {
                'R' | 'С' => row = Some(n),
                'C' => col = Some(n),
                _ => return None,
            }
        }
        if row.is_none() && col.is_none() {
            return None;
        }
        Some((row, col))
    }
    let (start, end) = match s.split_once(':') {
        Some((a, b)) => (part(a)?, part(b)?),
        None => (part(s)?, part(s)?),
    };
    // Отсутствующая ось означает «вся», и разворачивается она по ГРАНИЦАМ
    // документа, а не в бесконечность.
    let kind = match (start.0.is_some(), start.1.is_some()) {
        (true, false) => AreaKind::Rows,
        (false, true) => AreaKind::Columns,
        _ => AreaKind::Rect,
    };
    let mut rect = Rect::from_api(
        start.0.unwrap_or(1),
        start.1.unwrap_or(1),
        end.0.unwrap_or_else(|| i64::from(height.max(1))),
        end.1.unwrap_or_else(|| i64::from(width.max(1))),
    );
    rect.kind = kind;
    Some(rect)
}

/// `ПолучитьОбласть` — КОПИЯ участка как самостоятельный документ.
pub fn get_area(obj: &BslValue, args: &[BslValue]) -> RtResult<BslValue> {
    let doc = data(obj).ok_or_else(|| bad("ПолучитьОбласть: не табличный документ"))?;
    let (h, w) = {
        let d = doc.borrow();
        (d.height(), d.width())
    };
    let rect = match args {
        [r1, c1, r2, c2] => Rect::from_api(
            number(r1, "ПолучитьОбласть")?,
            number(c1, "ПолучитьОбласть")?,
            number(r2, "ПолучитьОбласть")?,
            number(c2, "ПолучитьОбласть")?,
        ),
        [BslValue::Str(s)] => {
            let name = s.to_string();
            let d = doc.borrow();
            match d.area_named(&name) {
                Some(a) => Rect {
                    r1: a.r1,
                    c1: a.c1,
                    r2: if a.kind == AreaKind::Columns {
                        h.saturating_sub(1)
                    } else {
                        a.r2
                    },
                    c2: if a.kind == AreaKind::Rows {
                        w.saturating_sub(1)
                    } else {
                        a.c2
                    },
                    kind: a.kind,
                },
                None => parse_address(&name, h, w)
                    .ok_or_else(|| bad(format!("Область не найдена: {name}")))?,
            }
        }
        _ => return Err(bad("ПолучитьОбласть: ожидалось 1 или 4 аргумента")),
    };
    let cut = doc.borrow().extract(rect.r1, rect.c1, rect.r2, rect.c2);
    Ok(BslValue::new_object(SpreadDocumentObject {
        data: Rc::new(RefCell::new(cut)),
    }))
}

/// `Вывести(Документ)` — приёмник наращивается вниз. Область ячеек платформа
/// здесь НЕ принимает, и мы тоже.
pub fn output(target: &BslValue, args: &[BslValue]) -> RtResult<()> {
    let target = data(target).ok_or_else(|| bad("Вывести: не табличный документ"))?;
    let source = match args {
        [v] if is_spread_document(v) => data(v).expect("проверено выше"),
        _ => return Err(bad("Вывести: ожидался табличный документ")),
    };
    // Копия снимается ДО заимствования приёмника: `Вывести` самого себя —
    // законный вызов, и без копии он упёрся бы в двойное заимствование.
    let copy = source.borrow().clone();
    target.borrow_mut().append(&copy);
    Ok(())
}

/// `Прочитать(Файл)` — загрузка .mxl в СУЩЕСТВУЮЩИЙ документ, как у
/// платформы: она не отдаёт новый объект, а заменяет содержимое приёмника.
pub fn read(obj: &BslValue, args: &[BslValue]) -> RtResult<()> {
    let doc = data(obj).ok_or_else(|| bad("Прочитать: не табличный документ"))?;
    let path = match args.first() {
        Some(BslValue::Str(s)) => s.to_string(),
        _ => return Err(bad("Прочитать: ожидался путь к файлу")),
    };
    let bytes = std::fs::read(&path).map_err(|e| bad(format!("не читается {path}: {e}")))?;
    // Формат выбирается по СОДЕРЖИМОМУ, а не по расширению.
    //
    // ОТСТУПЛЕНИЕ ОТ ПЛАТФОРМЫ, намеренное. У неё `Прочитать` берёт .mxl и
    // .xlsx, а макеты живут в конфигурации и достаются `ПолучитьМакет`.
    // Конфигурации у нас нет вовсе, поэтому XML-макет читается тем же
    // методом — иначе до макетов было бы не добраться никак.
    *doc.borrow_mut() = if bytes.starts_with(MXL_SIGNATURE) {
        from_mxl_bytes(&bytes)?
    } else {
        let text = String::from_utf8(bytes)
            .map_err(|_| bad(format!("{path}: не MXL и не текст в UTF-8")))?;
        crate::template::from_template_xml(&text)?
    };
    Ok(())
}

/// `НачатьГруппуСтрок([Имя][, Сворачиваемость])`. Второй аргумент — именно
/// СВОРАЧИВАЕМОСТЬ: `Ложь` даёт свёрнутую группу (измерено).
/// `Рисунки.Добавить(ТипРисунка)`. Поддержан только прямоугольник:
/// остальные типы платформа знает, но их содержимое — отдельные разделы
/// формата, которых здесь нет.
/// `Рисунки.Добавить(ТипРисунка)`.
pub fn drawings_add(obj: &BslValue, _args: &[BslValue]) -> RtResult<BslValue> {
    let doc = match obj
        .object_ref()
        .and_then(|object| object.downcast_ref::<SpreadDrawingsObject>())
    {
        Some(drawings) => drawings.data.clone(),
        None => return Err(bad("Добавить: не коллекция рисунков")),
    };
    let number_of = doc.borrow_mut().add_drawing(0.0, 0.0, 0.0, 0.0);
    Ok(BslValue::new_object(SpreadDrawingObject {
        data: doc,
        index: number_of - 1,
    }))
}

/// Свойства рисунка. Геометрия отдаётся в миллиметрах — но НЕ теми, что
/// задали: она защёлкивается на сетку четвертей пункта, поэтому 10 мм
/// читаются обратно как 9,96597… (измерено на платформе, и здесь так же).
pub fn drawing_property(
    doc: &Rc<RefCell<SpreadDocData>>,
    i: usize,
    name: &str,
) -> RtResult<BslValue> {
    let d = doc.borrow();
    let drawing = d
        .drawings()
        .get(i)
        .ok_or_else(|| bad("рисунок уже удалён"))?;
    // Значение защёлкивается на сетку и отдаётся с четырнадцатью знаками
    // после запятой — столько же печатает платформа.
    let mm = |v: f64| -> RtResult<BslValue> {
        let qp = (v * MM_TO_QP).round() as i128;
        let mantissa = (f64::from(qp as i32) * 25.4 / 288.0 * 1e14).round() as i128;
        Ok(BslValue::Number(BslNumber::from_parts(mantissa, 14)))
    };
    match () {
        _ if name.eq_ignore_ascii_case("Имя") || name.eq_ignore_ascii_case("Name") => {
            Ok(BslValue::Str(BslString::from_str(&drawing.name)))
        }
        _ if name.eq_ignore_ascii_case("Лево") || name.eq_ignore_ascii_case("Left") => {
            mm(drawing.left)
        }
        _ if name.eq_ignore_ascii_case("Верх") || name.eq_ignore_ascii_case("Top") => {
            mm(drawing.top)
        }
        _ if name.eq_ignore_ascii_case("Ширина") || name.eq_ignore_ascii_case("Width") => {
            mm(drawing.width)
        }
        _ if name.eq_ignore_ascii_case("Высота") || name.eq_ignore_ascii_case("Height") => {
            mm(drawing.height)
        }
        _ => Err(RtError::UnknownColumn(name.to_string())),
    }
}

pub fn set_drawing_property(
    doc: &Rc<RefCell<SpreadDocData>>,
    i: usize,
    name: &str,
    val: &BslValue,
) -> RtResult<()> {
    if name.eq_ignore_ascii_case("Имя") || name.eq_ignore_ascii_case("Name") {
        let name = val.to_string();
        let mut d = doc.borrow_mut();
        if let Some(drawing) = d.drawings_mut().get_mut(i) {
            drawing.name = name;
        }
        return Ok(());
    }
    let number = match val {
        BslValue::Number(n) => n
            .to_string()
            .replace(',', ".")
            .parse::<f64>()
            .unwrap_or(0.0),
        _ => return Err(bad("геометрия рисунка: ожидалось число")),
    };
    let mut d = doc.borrow_mut();
    {
        let Some(drawing) = d.drawings_mut().get_mut(i) else {
            return Ok(());
        };
        match () {
            _ if name.eq_ignore_ascii_case("Лево") || name.eq_ignore_ascii_case("Left") => {
                drawing.left = number
            }
            _ if name.eq_ignore_ascii_case("Верх") || name.eq_ignore_ascii_case("Top") => {
                drawing.top = number
            }
            _ if name.eq_ignore_ascii_case("Ширина") || name.eq_ignore_ascii_case("Width") => {
                drawing.width = number
            }
            _ if name.eq_ignore_ascii_case("Высота") || name.eq_ignore_ascii_case("Height") => {
                drawing.height = number
            }
            _ => return Err(RtError::UnknownColumn(name.to_string())),
        }
    }
    d.refresh_drawing_bounds(i);
    Ok(())
}

pub fn begin_row_group(obj: &BslValue, args: &[BslValue]) -> RtResult<()> {
    let doc = data(obj).ok_or_else(|| bad("НачатьГруппуСтрок: не табличный документ"))?;
    let name = match args.first() {
        Some(BslValue::Str(s)) => s.to_string(),
        _ => String::new(),
    };
    let collapsed = matches!(args.get(1), Some(BslValue::Boolean(false)));
    doc.borrow_mut().begin_row_group(&name, collapsed);
    Ok(())
}

pub fn end_row_group(obj: &BslValue) -> RtResult<()> {
    data(obj)
        .ok_or_else(|| bad("ЗакончитьГруппуСтрок: не табличный документ"))?
        .borrow_mut()
        .end_row_group();
    Ok(())
}

pub fn clear(obj: &BslValue) -> RtResult<()> {
    data(obj)
        .ok_or_else(|| bad("Очистить: не табличный документ"))?
        .borrow_mut()
        .clear();
    Ok(())
}

/// `Записать(Файл [, ТипФайла])`. Без типа — MXL (измерено).
pub fn write(obj: &BslValue, args: &[BslValue]) -> RtResult<()> {
    let doc = data(obj).ok_or_else(|| bad("Записать: не табличный документ"))?;
    let path = match args.first() {
        Some(BslValue::Str(s)) => s.to_string(),
        _ => return Err(bad("Записать: ожидался путь к файлу")),
    };
    let kind = match args.get(1) {
        None => FileKind::Mxl,
        Some(BslValue::Enum(e)) => match e {
            bsl_rt::EnumValue::SpreadFileMxl => FileKind::Mxl,
            bsl_rt::EnumValue::SpreadFileTxt => FileKind::Txt,
            bsl_rt::EnumValue::SpreadFileXlsx => FileKind::Xlsx,
            bsl_rt::EnumValue::SpreadFilePdf => FileKind::Pdf,
            _ => return Err(bad("Записать: неподдерживаемый тип файла")),
        },
        Some(_) => return Err(bad("Записать: ожидался ТипФайлаТабличногоДокумента")),
    };
    let d = doc.borrow();
    write_file(&d, &path, kind)
}

pub fn merge_cells(obj: &BslValue) -> RtResult<()> {
    let rect = rect(obj).ok_or_else(|| bad("Объединить: не область ячеек"))?;
    let doc = data(obj).expect("область всегда при документе");
    let mut d = doc.borrow_mut();
    // Вид области решает, в КАКОЙ из трёх списков ляжет объединение.
    match rect.kind {
        AreaKind::Rows => d.merge_rows(rect.r1, rect.r2),
        AreaKind::Columns => d.merge_columns(rect.c1, rect.c2),
        AreaKind::Rect => d.merge(Merge::new(rect.r1, rect.c1, rect.r2, rect.c2)),
    }
    Ok(())
}

pub fn unmerge_cells(obj: &BslValue) -> RtResult<()> {
    let rect = rect(obj).ok_or_else(|| bad("Разъединить: не область ячеек"))?;
    data(obj)
        .expect("область всегда при документе")
        .borrow_mut()
        .unmerge(rect.r1, rect.c1, rect.r2, rect.c2);
    Ok(())
}

/// Чтение свойства документа или области.
pub fn get_property(obj: &BslValue, name: &str) -> RtResult<BslValue> {
    let doc = data(obj).ok_or(RtError::NotAnObject)?;
    let d = doc.borrow();
    if let Some(rect) = rect(obj) {
        return match () {
            _ if name.eq_ignore_ascii_case("Текст") || name.eq_ignore_ascii_case("Text") => {
                // У ячейки со значением `Текст` отдаёт его ПРЕДСТАВЛЕНИЕ —
                // измерено: после `Значение = 42` текст равен «42».
                let text = d
                    .cell_value(rect.r1, rect.c1)
                    .unwrap_or_else(|| d.cell_text(rect.r1, rect.c1));
                Ok(BslValue::Str(BslString::from_str(&text)))
            }
            _ if name.eq_ignore_ascii_case("Расшифровка")
                || name.eq_ignore_ascii_case("Details") =>
            {
                Ok(BslValue::Str(BslString::from_str(
                    &d.cell_detail(rect.r1, rect.c1).unwrap_or_default(),
                )))
            }
            _ if name.eq_ignore_ascii_case("ПараметрРасшифровки")
                || name.eq_ignore_ascii_case("DetailsParameter") =>
            {
                Ok(BslValue::Str(BslString::from_str(
                    &d.cell_detail_param(rect.r1, rect.c1),
                )))
            }
            _ if name.eq_ignore_ascii_case("СодержитЗначение")
                || name.eq_ignore_ascii_case("ContainsValue") =>
            {
                Ok(BslValue::Boolean(d.cell_value(rect.r1, rect.c1).is_some()))
            }
            _ if name.eq_ignore_ascii_case("Параметр")
                || name.eq_ignore_ascii_case("Parameter") =>
            {
                // У обычного документа платформа отдаёт параметр ПУСТЫМ даже
                // из своего файла (измерено), поэтому здесь тоже пусто.
                Ok(BslValue::Str(BslString::from_str("")))
            }
            _ if name.eq_ignore_ascii_case("Имя") || name.eq_ignore_ascii_case("Name") => {
                let name = d
                    .names_iter()
                    .find(|(_, a)| a.r1 == rect.r1 && a.c1 == rect.c1)
                    .map(|(n, _)| n.clone())
                    .unwrap_or_default();
                Ok(BslValue::Str(BslString::from_str(&name)))
            }
            _ => Err(RtError::UnknownColumn(name.to_string())),
        };
    }
    match () {
        _ if name.eq_ignore_ascii_case("ВысотаТаблицы")
            || name.eq_ignore_ascii_case("TableHeight") =>
        {
            Ok(int_value(i64::from(d.height())))
        }
        _ if name.eq_ignore_ascii_case("ШиринаТаблицы")
            || name.eq_ignore_ascii_case("TableWidth") =>
        {
            Ok(int_value(i64::from(d.width())))
        }
        _ if name.eq_ignore_ascii_case("ОтображатьСетку")
            || name.eq_ignore_ascii_case("ShowGrid") =>
        {
            Ok(BslValue::Boolean(d.show_grid))
        }
        _ if name.eq_ignore_ascii_case("ФиксацияСверху")
            || name.eq_ignore_ascii_case("FixedTop") =>
        {
            Ok(int_value(d.fix_top))
        }
        _ if name.eq_ignore_ascii_case("ФиксацияСлева")
            || name.eq_ignore_ascii_case("FixedLeft") =>
        {
            Ok(int_value(d.fix_left))
        }
        // Поля страницы — в миллиметрах, умолчание 10 у каждого (измерено
        // на пустом документе 8.3.27).
        // Английские написания измерены перебором в
        // `tests/conformance/measure/measure-pdf-write.bsl`: платформа
        // знает `LeftMargin` и его собратьев, а `FieldLeft`, `MarginLeft` и
        // `FieldOnLeft` отвергает — все три пробы дали ошибку.
        _ if name.eq_ignore_ascii_case("ПолеСлева") || name.eq_ignore_ascii_case("LeftMargin") => {
            Ok(mm_value(d.margins.left))
        }
        _ if name.eq_ignore_ascii_case("ПолеСправа")
            || name.eq_ignore_ascii_case("RightMargin") =>
        {
            Ok(mm_value(d.margins.right))
        }
        _ if name.eq_ignore_ascii_case("ПолеСверху") || name.eq_ignore_ascii_case("TopMargin") => {
            Ok(mm_value(d.margins.top))
        }
        _ if name.eq_ignore_ascii_case("ПолеСнизу")
            || name.eq_ignore_ascii_case("BottomMargin") =>
        {
            Ok(mm_value(d.margins.bottom))
        }
        _ if name.eq_ignore_ascii_case("ОриентацияСтраницы")
            || name.eq_ignore_ascii_case("PageOrientation") =>
        {
            Ok(BslValue::Enum(if d.landscape {
                bsl_rt::EnumValue::PageOrientationLandscape
            } else {
                bsl_rt::EnumValue::PageOrientationPortrait
            }))
        }
        _ => Err(RtError::UnknownColumn(name.to_string())),
    }
}

/// Запись свойства документа или области.
pub fn set_property(obj: &BslValue, name: &str, val: BslValue) -> RtResult<()> {
    let doc = data(obj).ok_or(RtError::NotAnObject)?;
    if let Some(rect) = rect(obj) {
        let mut d = doc.borrow_mut();
        if name.eq_ignore_ascii_case("Текст") || name.eq_ignore_ascii_case("Text") {
            let text = val.to_string();
            for r in rect.r1..=rect.r2 {
                for c in rect.c1..=rect.c2 {
                    d.set_cell_text(r, c, &text);
                }
            }
            return Ok(());
        }
        if name.eq_ignore_ascii_case("ПараметрРасшифровки")
            || name.eq_ignore_ascii_case("DetailsParameter")
        {
            d.set_cell_detail_param(rect.r1, rect.c1, &val.to_string());
            return Ok(());
        }
        if name.eq_ignore_ascii_case("СодержитЗначение")
            || name.eq_ignore_ascii_case("ContainsValue")
        {
            // Платформа держит это отдельным переключателем: пока он не
            // взведён, `Значение` не пишется вовсе (измерено — «Поле объекта
            // недоступно для записи»). Взведение само по себе кладёт в
            // ячейку пустое значение.
            if matches!(val, BslValue::Boolean(true)) {
                if d.cell_value(rect.r1, rect.c1).is_none() {
                    d.set_cell_value(rect.r1, rect.c1, "");
                }
            } else {
                d.set_cell_text(rect.r1, rect.c1, "");
            }
            return Ok(());
        }
        if name.eq_ignore_ascii_case("Параметр") || name.eq_ignore_ascii_case("Parameter") {
            d.set_cell_parameter(rect.r1, rect.c1, &val.to_string());
            return Ok(());
        }
        if name.eq_ignore_ascii_case("Имя") || name.eq_ignore_ascii_case("Name") {
            let name = val.to_string();
            if name.is_empty() {
                let old: Vec<String> = d
                    .names_iter()
                    .filter(|(_, a)| a.r1 == rect.r1 && a.c1 == rect.c1)
                    .map(|(n, _)| n.clone())
                    .collect();
                for n in old {
                    d.clear_area_name(&n);
                }
            } else {
                d.set_area_name(&name, NamedArea::rect(rect.r1, rect.c1, rect.r2, rect.c2));
            }
            return Ok(());
        }
        if name.eq_ignore_ascii_case("ШиринаКолонки") || name.eq_ignore_ascii_case("ColumnWidth")
        {
            let width = number(&val, "ШиринаКолонки")?;
            for c in rect.c1..=rect.c2 {
                d.set_col_width(c, width);
            }
            return Ok(());
        }
        if name.eq_ignore_ascii_case("ВысотаСтроки") || name.eq_ignore_ascii_case("RowHeight")
        {
            let height = number(&val, "ВысотаСтроки")?;
            for r in rect.r1..=rect.r2 {
                d.set_row_height(r, height);
            }
            return Ok(());
        }
        return Err(RtError::UnknownColumn(name.to_string()));
    }
    let mut d = doc.borrow_mut();
    if name.eq_ignore_ascii_case("ОтображатьСетку") || name.eq_ignore_ascii_case("ShowGrid")
    {
        d.show_grid = matches!(val, BslValue::Boolean(true));
        return Ok(());
    }
    if name.eq_ignore_ascii_case("ФиксацияСверху") || name.eq_ignore_ascii_case("FixedTop")
    {
        d.fix_top = number(&val, "ФиксацияСверху")?;
        return Ok(());
    }
    if name.eq_ignore_ascii_case("ФиксацияСлева") || name.eq_ignore_ascii_case("FixedLeft")
    {
        d.fix_left = number(&val, "ФиксацияСлева")?;
        return Ok(());
    }
    // Поля страницы в миллиметрах. Значение принимается как есть, без
    // ограничения снизу: платформа его тоже не поджимает (измерено —
    // `ПолеСлева = -5` читается обратно как -5, а 500 как 500). Не измерено
    // другое — как отрицательное поле ложится в РАСКЛАДКУ её печати; тихо
    // подменять пользовательское число из-за этого нельзя.
    for (ru, en, field) in [
        ("ПолеСлева", "LeftMargin", 0),
        ("ПолеСправа", "RightMargin", 1),
        ("ПолеСверху", "TopMargin", 2),
        ("ПолеСнизу", "BottomMargin", 3),
    ] {
        if name.eq_ignore_ascii_case(ru) || name.eq_ignore_ascii_case(en) {
            let mm = number_f64(&val, ru)?;
            let margins = &mut d.margins;
            match field {
                0 => margins.left = mm,
                1 => margins.right = mm,
                2 => margins.top = mm,
                _ => margins.bottom = mm,
            }
            return Ok(());
        }
    }
    if name.eq_ignore_ascii_case("ОриентацияСтраницы")
        || name.eq_ignore_ascii_case("PageOrientation")
    {
        d.landscape = match val {
            BslValue::Enum(bsl_rt::EnumValue::PageOrientationLandscape) => true,
            BslValue::Enum(bsl_rt::EnumValue::PageOrientationPortrait) => false,
            _ => return Err(bad("ОриентацияСтраницы: ожидался член ОриентацияСтраницы")),
        };
        return Ok(());
    }
    Err(RtError::UnknownColumn(name.to_string()))
}

// --- Параметры макета -----------------------------------------------------

/// Достать `Rc` данных документа из `SpreadDocParams`.
fn param_data(obj: &BslValue) -> RtResult<Rc<RefCell<SpreadDocData>>> {
    match obj
        .object_ref()
        .and_then(|object| object.downcast_ref::<SpreadParamsObject>())
    {
        Some(params) => Ok(params.data.clone()),
        None => Err(bad("Параметры: не объект параметров макета")),
    }
}

/// Чтение `Область.Параметры.Имя`. Неизвестное имя — `Неопределено`,
/// как у структуры: `ЗаполнитьЗначенияСвойств` различает «свойство есть»
/// и «свойства нет» по `has_property`, а не по значению.
pub fn get_param(obj: &BslValue, name: &str) -> RtResult<BslValue> {
    let doc = param_data(obj)?;
    let d = doc.borrow();
    let upper = name.to_uppercase();
    Ok(d.params
        .iter()
        .find(|(k, _)| **k == upper)
        .map(|(_, v)| v.clone())
        .unwrap_or(BslValue::Undefined))
}

/// `Область.Параметры.Имя = Значение` — запись в карту параметров.
pub fn set_param(obj: &BslValue, name: &str, val: BslValue) -> RtResult<()> {
    let doc = param_data(obj)?;
    let mut d = doc.borrow_mut();
    let upper = name.to_uppercase();
    if let Some(slot) = d.params.iter_mut().find(|(k, _)| **k == upper) {
        *slot.1 = val;
    } else {
        d.params.insert(upper, val);
    }
    Ok(())
}

/// Все имена параметров макета в документе — имена ячеек с непустым
/// `CellData::parameter`. Нужны `fill.rs`, чтобы различать «у приёмника
/// такое свойство есть» и «нет».
pub fn param_names(doc: &SpreadDocData) -> Vec<String> {
    let mut out = Vec::new();
    for row in doc.rows.values() {
        for cell in row.cells.values() {
            if !cell.parameter.is_empty()
                && !out
                    .iter()
                    .any(|n: &String| n.eq_ignore_ascii_case(&cell.parameter))
            {
                out.push(cell.parameter.clone());
            }
        }
    }
    out
}

/// Есть ли у документа параметр с таким именем.
pub fn has_param(doc: &SpreadDocData, name: &str) -> bool {
    let upper = name.to_uppercase();
    doc.rows.values().any(|row| {
        row.cells
            .values()
            .any(|cell| cell.parameter.to_uppercase() == upper)
    })
}

/// Применить значения параметров к ячейкам: для каждой ячейки с непустым
/// `CellData::parameter` найти значение в `params` по имени и положить
/// его представление в `CellData::text`. Вызывается из ВМ перед `Вывести`,
/// потому что форматирование значения живёт в `bsl-format`.
pub fn apply_params(obj: &BslValue, params: &[(String, String)]) -> RtResult<()> {
    if params.is_empty() {
        return Ok(());
    }
    // Имена параметров в верхнем регистре — сравнение регистронезависимо.
    let upper_params: Vec<(String, &str)> = params
        .iter()
        .map(|(name, text)| (name.to_uppercase(), text.as_str()))
        .collect();
    let doc = data(obj).ok_or_else(|| bad("Вывести: не табличный документ"))?;
    let mut d = doc.borrow_mut();
    for row in d.rows.values_mut() {
        for cell in row.cells.values_mut() {
            if cell.parameter.is_empty() {
                continue;
            }
            let param_upper = cell.parameter.to_uppercase();
            if let Some((_, text)) = upper_params.iter().find(|(name, _)| *name == param_upper) {
                cell.text = text.to_string();
            }
        }
    }
    Ok(())
}

/// Снять значения параметров из карты документа вместе со строкой формата
/// каждой параметрической ячейки: пары `(имя, значение, формат)`. Формат
/// берётся из `CellData::format_spec` — строка BSL вроде «ЧДЦ=2».
/// Вызывается из ВМ для форматирования перед `apply_params`.
pub fn take_params(obj: &BslValue) -> RtResult<Vec<(String, BslValue, Option<String>)>> {
    let doc = data(obj).ok_or_else(|| bad("Вывести: не табличный документ"))?;
    let mut d = doc.borrow_mut();
    let mut out = Vec::new();
    // Имена параметров и форматы ячеек снимаются в один проход: для каждой
    // ячейки с непустым `parameter` запоминаем имя, формат и значение из
    // карты параметров.
    let mut seen = std::collections::HashSet::new();
    for row in d.rows.values() {
        for cell in row.cells.values() {
            if cell.parameter.is_empty() {
                continue;
            }
            let upper = cell.parameter.to_uppercase();
            if !seen.insert(upper.clone()) {
                continue;
            }
            let value = d
                .params
                .iter()
                .find(|(k, _)| **k == upper)
                .map(|(_, v)| v.clone())
                .unwrap_or(BslValue::Undefined);
            out.push((cell.parameter.clone(), value, cell.format_spec.clone()));
        }
    }
    d.params.clear();
    Ok(out)
}

// --- объектный протокол -----------------------------------------------------

static DOCUMENT_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ТабличныйДокумент",
    legacy_type_id: Some(TypeId::SpreadDocument),
};

static AREA_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ОбластьЯчеекТабличногоДокумента",
    legacy_type_id: Some(TypeId::SpreadArea),
};

static DRAWINGS_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "КоллекцияРисунковТабличногоДокумента",
    legacy_type_id: Some(TypeId::SpreadDrawings),
};

static DRAWING_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "РисунокТабличногоДокумента",
    legacy_type_id: Some(TypeId::SpreadDrawing),
};

static PARAMS_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ПараметрыМакетаТабличногоДокумента",
    legacy_type_id: Some(TypeId::SpreadDocParams),
};

impl SpreadDocumentObject {
    fn as_value(&self) -> BslValue {
        BslValue::new_object(SpreadDocumentObject {
            data: self.data.clone(),
        })
    }
}

impl SpreadAreaObject {
    fn as_value(&self) -> BslValue {
        BslValue::new_object(SpreadAreaObject {
            data: self.data.clone(),
            rect: self.rect,
        })
    }
}

/// Общие методы документа и области: имена делятся получателями, а данные
/// у обоих одни (см. `data`).
fn shared_method(
    receiver: &BslValue,
    method: &str,
    arguments: &[BslValue],
) -> Option<RtResult<BslValue>> {
    let eq =
        |ru: &str, en: &str| method.eq_ignore_ascii_case(ru) || method.eq_ignore_ascii_case(en);
    if eq("Область", "Area") {
        return Some(region(receiver, arguments));
    }
    if eq("Объединить", "Merge") {
        return Some(merge_cells(receiver).map(|()| BslValue::Undefined));
    }
    if eq("Разъединить", "Unmerge") {
        return Some(unmerge_cells(receiver).map(|()| BslValue::Undefined));
    }
    None
}

impl ObjectProtocol for SpreadDocumentObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &DOCUMENT_TYPE
    }

    fn get_property(&self, name: &str, _context: &mut CallContext<'_>) -> RtResult<BslValue> {
        // `Рисунки` и `Параметры` — окна в те же данные.
        if name.eq_ignore_ascii_case("Рисунки") || name.eq_ignore_ascii_case("Drawings") {
            return Ok(BslValue::new_object(SpreadDrawingsObject {
                data: self.data.clone(),
            }));
        }
        if name.eq_ignore_ascii_case("Параметры") || name.eq_ignore_ascii_case("Parameters")
        {
            return Ok(BslValue::new_object(SpreadParamsObject {
                data: self.data.clone(),
            }));
        }
        get_property(&self.as_value(), name)
    }

    fn set_property(
        &self,
        name: &str,
        value: BslValue,
        _context: &mut CallContext<'_>,
    ) -> RtResult<()> {
        set_property(&self.as_value(), name, value)
    }

    fn call_method(
        &self,
        method: &str,
        arguments: &[BslValue],
        _context: &mut CallContext<'_>,
    ) -> RtResult<BslValue> {
        let receiver = self.as_value();
        let eq =
            |ru: &str, en: &str| method.eq_ignore_ascii_case(ru) || method.eq_ignore_ascii_case(en);
        if let Some(result) = shared_method(&receiver, method, arguments) {
            return result;
        }
        if eq("Записать", "Write") {
            write(&receiver, arguments)?;
            return Ok(BslValue::Undefined);
        }
        if eq("Прочитать", "Read") {
            read(&receiver, arguments)?;
            return Ok(BslValue::Undefined);
        }
        if eq("Вывести", "Output") {
            return output_with_params(&receiver, arguments);
        }
        if eq("ПолучитьОбласть", "GetArea") {
            return get_area(&receiver, arguments);
        }
        if eq("Очистить", "Clear") {
            clear(&receiver)?;
            return Ok(BslValue::Undefined);
        }
        if eq("НачатьГруппуСтрок", "StartRowGroup") {
            begin_row_group(&receiver, arguments)?;
            return Ok(BslValue::Undefined);
        }
        if eq("ЗакончитьГруппуСтрок", "EndRowGroup") {
            end_row_group(&receiver)?;
            return Ok(BslValue::Undefined);
        }
        Err(RtError::UnknownMethod {
            method: method.to_string(),
            receiver: DOCUMENT_TYPE.name,
        })
    }

    fn is_filled(&self) -> RtResult<bool> {
        Ok(true)
    }
}

/// `Вывести` с подстановкой параметров макета: значения из карты
/// параметров источника форматируются `bsl-format` и кладутся в текст
/// ячеек с совпадающим `CellData::parameter` — форматирование живёт
/// здесь, потому что `bsl-format` зависит от `bsl-rt`, а не наоборот.
fn output_with_params(target: &BslValue, args: &[BslValue]) -> RtResult<BslValue> {
    let Some(source) = args.first() else {
        return Err(RtError::MethodNotApplicable {
            method: "Вывести",
            receiver: DOCUMENT_TYPE.name,
        });
    };
    if is_spread_document(source) {
        let params = take_params(source)?;
        if !params.is_empty() {
            let mut formatted: Vec<(String, String)> = Vec::with_capacity(params.len());
            for (name, value, spec) in &params {
                let text = bsl_format::format_value_for_cell(value, spec.as_deref())?;
                formatted.push((name.clone(), text));
            }
            apply_params(source, &formatted)?;
        }
    }
    output(target, args)?;
    Ok(BslValue::Undefined)
}

impl ObjectProtocol for SpreadAreaObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &AREA_TYPE
    }

    fn get_property(&self, name: &str, _context: &mut CallContext<'_>) -> RtResult<BslValue> {
        get_property(&self.as_value(), name)
    }

    fn set_property(
        &self,
        name: &str,
        value: BslValue,
        _context: &mut CallContext<'_>,
    ) -> RtResult<()> {
        let receiver = self.as_value();
        // `Значение` и `Расшифровка` — значения ЛЮБОГО типа; в документ
        // уходит их представление по правилам `bsl-format` (измерено).
        if name.eq_ignore_ascii_case("Значение") || name.eq_ignore_ascii_case("Value") {
            return set_value(&receiver, &bsl_format::format_value(&value, None)?);
        }
        if name.eq_ignore_ascii_case("Расшифровка") || name.eq_ignore_ascii_case("Details")
        {
            return set_detail(&receiver, &bsl_format::format_value(&value, None)?);
        }
        set_property(&receiver, name, value)
    }

    fn call_method(
        &self,
        method: &str,
        arguments: &[BslValue],
        _context: &mut CallContext<'_>,
    ) -> RtResult<BslValue> {
        let receiver = self.as_value();
        if let Some(result) = shared_method(&receiver, method, arguments) {
            return result;
        }
        Err(RtError::UnknownMethod {
            method: method.to_string(),
            receiver: AREA_TYPE.name,
        })
    }

    fn is_filled(&self) -> RtResult<bool> {
        Ok(true)
    }
}

impl ObjectProtocol for SpreadDrawingsObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &DRAWINGS_TYPE
    }

    fn call_method(
        &self,
        method: &str,
        arguments: &[BslValue],
        _context: &mut CallContext<'_>,
    ) -> RtResult<BslValue> {
        if method.eq_ignore_ascii_case("Добавить") || method.eq_ignore_ascii_case("Add") {
            let receiver = BslValue::new_object(SpreadDrawingsObject {
                data: self.data.clone(),
            });
            return drawings_add(&receiver, arguments);
        }
        if method.eq_ignore_ascii_case("Количество") || method.eq_ignore_ascii_case("Count")
        {
            return Ok(BslValue::number_from_i64(
                self.data.borrow().drawings().len() as i64,
            ));
        }
        Err(RtError::UnknownMethod {
            method: method.to_string(),
            receiver: DRAWINGS_TYPE.name,
        })
    }

    fn collection_len(&self) -> RtResult<usize> {
        Ok(self.data.borrow().drawings().len())
    }
}

impl ObjectProtocol for SpreadDrawingObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &DRAWING_TYPE
    }

    fn get_property(&self, name: &str, _context: &mut CallContext<'_>) -> RtResult<BslValue> {
        drawing_property(&self.data, self.index, name)
    }

    fn set_property(
        &self,
        name: &str,
        value: BslValue,
        _context: &mut CallContext<'_>,
    ) -> RtResult<()> {
        set_drawing_property(&self.data, self.index, name, &value)
    }
}

impl ObjectProtocol for SpreadParamsObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &PARAMS_TYPE
    }

    fn get_property(&self, name: &str, _context: &mut CallContext<'_>) -> RtResult<BslValue> {
        get_param(&self.as_params_value(), name)
    }

    fn set_property(
        &self,
        name: &str,
        value: BslValue,
        _context: &mut CallContext<'_>,
    ) -> RtResult<()> {
        set_param(&self.as_params_value(), name, value)
    }

    // Параметры макета — источник и приёмник «ЗаполнитьЗначенияСвойств»
    // (измерено): пары отдаются в порядке текста, чужие имена приёмник
    // пропускает.
    fn fill_source_pairs(&self) -> Option<Vec<(String, BslValue)>> {
        let d = self.data.borrow();
        let names = param_names(&d);
        Some(
            names
                .into_iter()
                .map(|name| {
                    let upper = name.to_uppercase();
                    let value = d
                        .params
                        .iter()
                        .find(|(k, _)| **k == upper)
                        .map(|(_, v)| v.clone())
                        .unwrap_or(BslValue::Undefined);
                    (name, value)
                })
                .collect(),
        )
    }

    fn has_property(&self, name: &str) -> bool {
        has_param(&self.data.borrow(), name)
    }

    fn fill_property(&self, name: &str, value: BslValue) -> RtResult<bool> {
        if !self.has_property(name) {
            return Ok(false);
        }
        set_param(&self.as_params_value(), name, value)?;
        Ok(true)
    }
}

impl SpreadParamsObject {
    fn as_params_value(&self) -> BslValue {
        BslValue::new_object(SpreadParamsObject {
            data: self.data.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Пустой документ платформы — 605 байт; проверяем, что скелет совпал
    /// хотя бы по длине и по обрамлению, а точное совпадение ловит фикстура
    /// с эталонным файлом.
    #[test]
    fn empty_document_has_a_header_and_no_trailing_newline() {
        let bytes = to_mxl_bytes(&SpreadDocData::new());
        assert!(bytes.starts_with(MXL_HEADER));
        assert_eq!(&bytes[13..16], BOM);
        assert_eq!(bytes.last(), Some(&b'}'));
    }

    #[test]
    fn a_quote_in_text_is_doubled() {
        assert_eq!(quoted("ка\"вычка"), "\"ка\"\"вычка\"");
    }

    #[test]
    fn height_never_shrinks_after_clearing_a_cell() {
        let mut doc = SpreadDocData::new();
        doc.set_cell_text(2, 4, "X");
        assert_eq!((doc.height(), doc.width()), (3, 5));
        doc.set_cell_text(2, 4, "");
        assert_eq!((doc.height(), doc.width()), (3, 5));
    }

    /// `Записать` разбирает второй аргумент по перечислению, и член, в
    /// который мы писать не умеем, обязан дать ошибку, а не тихий MXL.
    /// Фикстура этого проверить не может: платформа такие форматы УМЕЕТ, и
    /// строка вышла бы разной не из-за ошибки, а из-за объёма.
    #[test]
    fn write_refuses_a_file_type_it_cannot_produce() {
        let doc = new_document();
        let path = std::env::temp_dir().join(format!("open-bsl-spread-{}.bin", std::process::id()));
        let path = BslValue::Str(BslString::from_str(&path.to_string_lossy()));
        for (kind, ok) in [
            (bsl_rt::EnumValue::SpreadFileMxl, true),
            (bsl_rt::EnumValue::SpreadFileTxt, true),
            (bsl_rt::EnumValue::SpreadFileXlsx, true),
            (bsl_rt::EnumValue::SpreadFilePdf, true),
            (bsl_rt::EnumValue::JsonBoolean, false),
        ] {
            let args = [path.clone(), BslValue::Enum(kind)];
            assert_eq!(write(&doc, &args).is_ok(), ok, "{kind:?}");
        }
        // Не член перечисления вовсе — тоже ошибка.
        assert!(write(&doc, &[path.clone(), BslValue::Boolean(true)]).is_err());
        if let BslValue::Str(s) = &path {
            std::fs::remove_file(s.to_string()).ok();
        }
    }

    /// Поля страницы и ориентация — свойства ДОКУМЕНТА: умолчания
    /// измерены (10 мм и «Портрет»), строка приводится к числу, а
    /// не-число отвергается.
    #[test]
    fn page_properties_round_trip_through_bsl() {
        let doc = new_document();
        assert_eq!(
            get_property(&doc, "ПолеСлева").unwrap().to_string(),
            "10",
            "умолчание поля"
        );
        assert!(matches!(
            get_property(&doc, "ОриентацияСтраницы").unwrap(),
            BslValue::Enum(bsl_rt::EnumValue::PageOrientationPortrait)
        ));

        set_property(&doc, "ПолеСлева", int_value(30)).unwrap();
        set_property(
            &doc,
            "BottomMargin",
            BslValue::Str(BslString::from_str("12,7")),
        )
        .unwrap();
        set_property(
            &doc,
            "ОриентацияСтраницы",
            BslValue::Enum(bsl_rt::EnumValue::PageOrientationLandscape),
        )
        .unwrap();
        assert_eq!(get_property(&doc, "LeftMargin").unwrap().to_string(), "30");
        // `Display` у `BslValue` отладочный, с точкой; запятую пользователь
        // видит через `bsl_format` — это проверяет фикстура `pdf-write`.
        assert_eq!(get_property(&doc, "ПолеСнизу").unwrap().to_string(), "12.7");
        assert!(matches!(
            get_property(&doc, "PageOrientation").unwrap(),
            BslValue::Enum(bsl_rt::EnumValue::PageOrientationLandscape)
        ));

        assert!(set_property(
            &doc,
            "ПолеСлева",
            BslValue::Str(BslString::from_str("не число"))
        )
        .is_err());
        assert!(set_property(&doc, "ОриентацияСтраницы", int_value(1)).is_err());
    }

    /// Поля переживают круг через MXL: идентификаторы пар 6..9 измерены
    /// (сверху, слева, снизу, справа) и лежат в сотых долях миллиметра.
    #[test]
    fn margins_survive_the_mxl_round_trip() {
        let mut doc = SpreadDocData::new();
        doc.set_cell_text(0, 0, "A");
        doc.margins.top = 33.0;
        doc.margins.left = 31.0;
        doc.margins.bottom = 34.0;
        doc.margins.right = 32.5;
        let bytes = to_mxl_bytes(&doc);
        let text = String::from_utf8_lossy(&bytes).to_string();
        assert!(text.contains("{\"N\",3300}"), "{text}");
        assert!(text.contains("{\"N\",3250}"), "{text}");
        let back = from_mxl_bytes(&bytes).unwrap();
        assert_eq!(back.margins, doc.margins);
        // Умолчательный документ пишется теми же 1000, что и раньше, —
        // иначе разъехался бы весь корпус MXL.
        let plain = to_mxl_bytes(&SpreadDocData::new());
        assert_eq!(
            String::from_utf8_lossy(&plain)
                .matches("{\"N\",1000}")
                .count(),
            6
        );
    }

    /// Тот же круг, но с ФАЙЛОМ ПЛАТФОРМЫ. `tests/conformance/pdf/
    /// probe-margins.mxl` записан 8.3.27 при полях 31, 32, 33 и 34 мм
    /// (съёмка `capture-platform-pdf-layout.bsl`, её вывод помнит длину:
    /// «probe-margins.mxl: Да, 1 085»), и порядок идентификаторов пар —
    /// 6 сверху, 7 слева, 8 снизу, 9 справа — прочитан именно из него.
    /// Проба закоммичена рядом со скриптом, чтобы это утверждение
    /// проверялось, а не пересказывалось.
    #[test]
    fn margins_come_back_from_the_mxl_written_by_the_platform() {
        let bytes = std::fs::read("../../tests/conformance/pdf/probe-margins.mxl")
            .expect("проба съёмки лежит рядом со скриптом");
        let doc = from_mxl_bytes(&bytes).expect("файл платформы читается");
        assert_eq!(
            doc.margins,
            crate::pdf_layout::PageMargins {
                left: 31.0,
                right: 32.0,
                top: 33.0,
                bottom: 34.0,
            }
        );
    }

    #[test]
    fn output_shifts_rows_down() {
        let mut a = SpreadDocData::new();
        a.set_cell_text(0, 0, "A");
        let mut b = SpreadDocData::new();
        b.set_cell_text(0, 0, "B");
        a.append(&b);
        assert_eq!(a.cell_text(0, 0), "A");
        assert_eq!(a.cell_text(1, 0), "B");
        assert_eq!(a.height(), 2);
    }
}

// --- разбор MXL -----------------------------------------------------------

/// Узел скобочной сериализации: либо группа, либо строка в кавычках, либо
/// «атом» — число, GUID или голое слово.
#[derive(Debug)]
enum Node {
    Group(Vec<Node>),
    Text(String),
    Atom(String),
}

impl Node {
    fn group(&self) -> RtResult<&[Node]> {
        match self {
            Node::Group(g) => Ok(g),
            _ => Err(bad("ожидалась группа")),
        }
    }

    fn text(&self) -> String {
        match self {
            Node::Text(s) => s.clone(),
            Node::Atom(s) => s.clone(),
            Node::Group(_) => String::new(),
        }
    }

    fn number(&self) -> RtResult<i64> {
        match self {
            Node::Atom(s) => s
                .parse()
                .map_err(|_| bad(format!("ожидалось число, а не «{s}»"))),
            _ => Err(bad("ожидалось число")),
        }
    }
}

/// Разбор текста в дерево. Кавычка внутри строки удвоена — единственное
/// экранирование формата, поэтому и разбор у него ровно один.
fn parse_nodes(s: &str) -> RtResult<Node> {
    let mut chars = s.chars().peekable();
    fn group(chars: &mut std::iter::Peekable<std::str::Chars>) -> RtResult<Node> {
        let mut items = Vec::new();
        let mut atom = String::new();
        let flush = |atom: &mut String, items: &mut Vec<Node>| {
            let t = atom.trim();
            if !t.is_empty() {
                items.push(Node::Atom(t.to_string()));
            }
            atom.clear();
        };
        while let Some(c) = chars.next() {
            match c {
                '{' => {
                    flush(&mut atom, &mut items);
                    items.push(group(chars)?);
                }
                '}' => {
                    flush(&mut atom, &mut items);
                    return Ok(Node::Group(items));
                }
                ',' => flush(&mut atom, &mut items),
                '"' => {
                    flush(&mut atom, &mut items);
                    let mut text = String::new();
                    loop {
                        match chars.next() {
                            Some('"') => {
                                if chars.peek() == Some(&'"') {
                                    text.push('"');
                                    chars.next();
                                } else {
                                    break;
                                }
                            }
                            Some(ch) => text.push(ch),
                            None => return Err(bad("незакрытая строка")),
                        }
                    }
                    items.push(Node::Text(text));
                }
                c if c.is_whitespace() => atom.push(' '),
                c => atom.push(c),
            }
        }
        Err(bad("незакрытая группа"))
    }
    match chars.next() {
        Some('{') => group(&mut chars),
        _ => Err(bad("файл не начинается с группы")),
    }
}

/// Курсор по плоскому списку узлов: разделы формата идут подряд и без
/// разметки, поэтому читаются позиционно — ровно в том же порядке, в каком
/// их пишет [`mxl_body`]. Эти две функции обязаны меняться вместе.
struct Cursor<'a> {
    items: &'a [Node],
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn next(&mut self) -> RtResult<&'a Node> {
        let n = self
            .items
            .get(self.pos)
            .ok_or_else(|| bad("файл кончился раньше времени"))?;
        self.pos += 1;
        Ok(n)
    }

    /// Ошибка называет ПОЗИЦИЮ и то, что там лежит: разделы читаются
    /// подряд, и без этого непонятно, на каком именно разбор сбился.
    fn number(&mut self) -> RtResult<i64> {
        let at = self.pos;
        self.next()?.number().map_err(|_| {
            bad(format!(
                "ожидалось число в позиции {at}, а там {:?}",
                self.items.get(at)
            ))
        })
    }

    fn skip(&mut self, n: usize) -> RtResult<()> {
        for _ in 0..n {
            self.next()?;
        }
        Ok(())
    }
}

/// Прочитать документ из байтов .mxl.
///
/// # Errors
///
/// Возвращает ошибку, если нет заголовка `MOXCEL`, текст не разбирается или
/// разделы идут не в том порядке, в каком их пишет платформа.
pub fn from_mxl_bytes(bytes: &[u8]) -> RtResult<SpreadDocData> {
    // Сверяется ПОДПИСЬ, а не весь заголовок: последний его байт — версия
    // формата, и она бывает разной. Файл настоящего отчёта из 1С пришёл с
    // версией 11, наши эталоны — с 12; тело при этом устроено одинаково.
    // Пишем мы всегда 12, как более новую.
    if !bytes.starts_with(MXL_SIGNATURE) {
        return Err(bad("не файл MXL: нет подписи MOXCEL"));
    }
    if bytes.len() < MXL_HEADER.len() {
        return Err(bad("файл MXL оборван на заголовке"));
    }
    let body = &bytes[MXL_HEADER.len()..];
    let body = body.strip_prefix(BOM).unwrap_or(body);
    let text = std::str::from_utf8(body).map_err(|_| bad("тело MXL не в UTF-8"))?;
    // Ровно обратное тому, что делает запись: `ВК`+`ПС` свернуть в `ПС`.
    // Иначе перевод строки ВНУТРИ текста ячейки вернулся бы удвоенным.
    let text = text.replace("\r\n", "\n");
    let root = parse_nodes(&text)?;
    let items = root.group()?;
    let mut cur = Cursor { items, pos: 0 };

    let mut doc = SpreadDocData::new();
    // Версия, языки, размеры по умолчанию, палитра линий и шесть служебных
    // групп — их содержимое мы не моделируем, но пропустить надо ровно
    // столько, сколько пишем сами.
    cur.skip(3)?; // 8, 1, 12
    cur.skip(1)?; // языки
    cur.skip(1)?; // {128,72}
                  // Палитра линий: `{<кол-во>,1,<запись>,0,1,<запись>,0,...}` — записи
                  // идут тройками, потому что каждая обёрнута парой служебных чисел.
    let line_palette = {
        let g = cur.next()?.group()?;
        let count = g.first().map_or(Ok(0), Node::number)? as usize;
        let mut out = Vec::with_capacity(count);
        for i in 0..count {
            if let Some(Node::Group(record)) = g.get(2 + i * 3) {
                out.push(Line {
                    style: match record.get(3).map_or(Ok(1), Node::number)? {
                        2 => LineStyle::Dotted,
                        3 => LineStyle::Double,
                        _ => LineStyle::Solid,
                    },
                    width: record.get(4).map_or(Ok(1), Node::number)?,
                });
            }
        }
        out
    };
    cur.skip(1)?; // счётчик следом за палитрой
    cur.skip(6)?; // шесть {0,0}
    cur.skip(2)?; // 0, 2

    // --- строки и ячейки ---------------------------------------------
    let row_count = cur.number()?;
    let mut heights: Vec<(u32, i64)> = Vec::new();
    let mut cells: Vec<(u32, u32, CellData)> = Vec::new();
    let mut row_formats: Vec<(u32, usize)> = Vec::new();
    let mut cell_formats: Vec<(u32, u32, usize)> = Vec::new();
    for _ in 0..row_count {
        let index = cur.number()? as u32;
        let row_format = cur.number()? as usize;
        if row_format > 0 {
            row_formats.push((index, row_format));
        }
        let count = cur.number()?;
        if count == 0 {
            heights.push((index, 0)); // строка есть, ячеек нет
            continue;
        }
        let mut column = cur.number()? as u32;
        for i in 0..count {
            let cell_group = cur.next()?.group()?;
            let mut cell = CellData::default();
            let mask = cell_group
                .first()
                .ok_or_else(|| bad("пустая ячейка"))?
                .number()?;
            // Поля идут по возрастанию бита, поэтому читаются курсором.
            let mut field = 2;
            if mask & 2 != 0 {
                // `{"S","<представление>"}` — тег типа и то, как значение
                // печатается. Самого значения в файле нет (измерено).
                if let Some(Node::Group(pair)) = cell_group.get(field) {
                    cell.value = Some(pair_text(pair_get(pair, 1)));
                }
                field += 1;
            }
            if mask & 4 != 0 {
                if let Some(Node::Group(pair)) = cell_group.get(field) {
                    cell.detail = Some(pair_text(pair_get(pair, 1)));
                }
                field += 1;
            }
            if mask & 8 != 0 {
                cell.detail_param = cell_group.get(field).map(Node::text).unwrap_or_default();
                field += 1;
            }
            let cell_format = cell_group.get(1).map_or(Ok(0), Node::number)?;
            if mask & 16 != 0 {
                // Текстовый блок: `{1,1,{<язык>,<текст>}}`.
                if let Some(Node::Group(block)) = cell_group.get(field) {
                    if let Some(Node::Group(pair)) = block.get(2) {
                        let lang = pair_text(pair_get(pair, 0));
                        let value = pair_text(pair_get(pair, 1));
                        if lang.is_empty() {
                            cell.parameter = value;
                        } else {
                            cell.text = value;
                        }
                    }
                }
            }
            if cell_format > 0 {
                cell_formats.push((index, column, cell_format as usize));
            }
            cells.push((index, column, cell));
            if i + 1 < count {
                column = cur.number()? as u32;
            }
        }
    }

    // --- колонки и высота ---------------------------------------------
    let columns = cur.next()?.group()?;
    let width = columns.first().map_or(Ok(0), Node::number)? as u32;
    // `{<число колонок>,0,GUID,<сколько с форматом>,<колонка>,<формат>,...}`
    let mut column_formats: Vec<(u32, usize)> = Vec::new();
    if columns.len() > 4 {
        let count = columns[3].number()? as usize;
        for i in 0..count {
            let (a, b) = (4 + i * 2, 5 + i * 2);
            if b < columns.len() {
                column_formats.push((columns[a].number()? as u32, columns[b].number()? as usize));
            }
        }
    }
    let height = cur.number()? as u32;
    // Дополнительные НАБОРЫ КОЛОНОК: счётчик, сами наборы и привязка строк
    // к ним парами «строка, набор». Своей модели у них нет — строки лежат в
    // общей сетке, — но пропустить их надо точно, иначе разъедется всё
    // дальше. В простом документе оба счётчика нулевые, и раздел выглядит
    // как два нуля; настоящий отчёт из 1С показал, что это не константа.
    let set_count = cur.number()? as usize;
    let mut raw_sets: Vec<RawColumnSet> = Vec::with_capacity(set_count);
    for _ in 0..set_count {
        let g = cur.next()?.group()?;
        let count = g.first().map_or(Ok(0), Node::number)?.max(0) as u32;
        let id = g.get(2).map(Node::text).unwrap_or_default();
        // Хвост дескриптора устроен как у основного набора: сколько колонок
        // с форматом, затем пары «колонка, формат».
        let mut formats = Vec::new();
        if g.len() > 4 {
            let count = g[3].number()?.max(0) as usize;
            for i in 0..count {
                let (a, b) = (4 + i * 2, 5 + i * 2);
                if b < g.len() {
                    formats.push((g[a].number()?.max(0) as u32, g[b].number()?.max(0) as usize));
                }
            }
        }
        raw_sets.push((id, count, formats));
    }
    let binding_count = cur.number()? as usize;
    let mut bindings = Vec::with_capacity(binding_count);
    for _ in 0..binding_count {
        let row = cur.number()?.max(0) as u32;
        let set = cur.number()?.max(0) as u32;
        bindings.push((row, set));
    }

    // Дальше ДВАЖДЫ число рисунков и сами рисунки, затем секция групп
    // строк, затем ещё три нуля.
    // Первое поле — сколько НОМЕРОВ выдано, второе — сколько рисунков
    // осталось. Различить их удалось только на документе, где рисунок
    // добавили и удалили: там `1,0`.
    cur.skip(1)?;
    let drawing_count = cur.number()? as usize;
    let mut drawing_cells_raw = Vec::with_capacity(drawing_count);
    for _ in 0..drawing_count {
        let g = cur.next()?.group()?;
        // `{{0,<формат>},2,<кол1>,<стр1>,<смещ>,<смещ>,<кол2>,<стр2>,...}`
        let num = |i: usize| -> RtResult<i64> { g.get(i).map_or(Ok(0), Node::number) };
        drawing_cells_raw.push((
            num(2)? as u32,
            num(3)? as u32,
            num(4)?,
            num(5)?,
            num(6)? as u32,
            num(7)? as u32,
            num(8)?,
            num(9)?,
        ));
    }
    let group_count = cur.number()? as usize;
    let mut groups = Vec::with_capacity(group_count);
    for _ in 0..group_count {
        let g = cur.next()?.group()?;
        let name = match g.get(3) {
            Some(Node::Group(loc)) => match loc.get(2) {
                Some(Node::Group(pair)) => pair_text(pair_get(pair, 1)),
                _ => String::new(),
            },
            _ => String::new(),
        };
        groups.push(RowGroup {
            r1: g.first().map_or(Ok(0), Node::number)?.max(0) as u32,
            r2: g.get(1).map_or(Ok(0), Node::number)?.max(0) as u32,
            level: g.get(2).map_or(Ok(0), Node::number)?.max(0) as u32,
            name,
            collapsed: g.get(4).map_or(Ok(0), Node::number)? != 0,
        });
        cur.skip(1)?; // замыкающий -1
    }
    cur.skip(3)?;

    for (r, c, cell) in cells {
        if let Some(detail) = &cell.detail {
            doc.set_cell_detail(r, c, detail);
        }
        if !cell.detail_param.is_empty() {
            doc.set_cell_detail_param(r, c, &cell.detail_param);
        }
        if let Some(presentation) = &cell.value {
            doc.set_cell_value(r, c, presentation);
        } else if !cell.text.is_empty() {
            doc.set_cell_text(r, c, &cell.text);
        } else if !cell.parameter.is_empty() {
            doc.set_cell_parameter(r, c, &cell.parameter);
        } else {
            doc.rows.entry(r).or_default().cells.entry(c).or_default();
        }
    }
    for (r, _) in heights {
        doc.rows.entry(r).or_default();
    }

    // --- три списка объединений ----------------------------------------
    for record in cur.next()?.group()?.iter().skip(1) {
        let g = record.group()?;
        if g.len() >= 5 {
            let (c1, r1, c2, r2, flag) = (
                g[0].number()? as u32,
                g[1].number()? as u32,
                g[2].number()? as u32,
                g[3].number()? as u32,
                g[4].number()?,
            );
            doc.merges.push(Merge {
                r1,
                c1,
                r2,
                c2,
                unmerge: flag == 2,
            });
        }
    }
    for record in cur.next()?.group()?.iter().skip(1) {
        let g = record.group()?;
        if g.len() >= 4 {
            doc.row_merges
                .push((g[1].number()? as u32, g[3].number()? as u32));
        }
    }
    for record in cur.next()?.group()?.iter().skip(1) {
        let g = record.group()?;
        if g.len() >= 3 {
            doc.col_merges
                .push((g[0].number()? as u32, g[2].number()? as u32));
        }
    }

    // --- именованные области --------------------------------------------
    let names = cur.next()?.group()?;
    let mut i = 1;
    while i + 1 < names.len() {
        let name = names[i].text();
        if let Node::Group(body) = &names[i + 1] {
            // `{1,{<область>},0}` — ячейки, `{2,<рисунок>,0}` — рисунок;
            // рисунки мы не моделируем и пропускаем.
            if body.first().map_or(Ok(0), Node::number)? == 1 {
                if let Some(Node::Group(area)) = body.get(1) {
                    if area.len() >= 5 {
                        let kind = match area[0].number()? {
                            1 => AreaKind::Rows,
                            2 => AreaKind::Columns,
                            _ => AreaKind::Rect,
                        };
                        let (c1, r1, c2, r2) = (
                            area[1].number()?,
                            area[2].number()?,
                            area[3].number()?,
                            area[4].number()?,
                        );
                        doc.names.insert(
                            name,
                            NamedArea {
                                kind,
                                r1: r1.max(0) as u32,
                                c1: c1.max(0) as u32,
                                r2: r2.max(0) as u32,
                                c2: c2.max(0) as u32,
                            },
                        );
                    }
                }
            }
        }
        i += 2;
    }

    // --- хвост: параметры страницы, область печати и палитры -----------
    cur.skip(1)?; // ""
    let page = cur.next()?.group()?;
    // `{0,<кол-во>,<id>,{"N",v},...}` — пары «идентификатор — значение»,
    // отсортированные по идентификатору.
    if let Some(Node::Group(list)) = page.first() {
        let mut i = 2;
        while i + 1 < list.len() {
            let id = list[i].number().unwrap_or(0);
            if let Node::Group(val) = &list[i + 1] {
                let value = val.get(1).map_or(Ok(0), Node::number)?;
                // Поля страницы лежат в сотых долях миллиметра, и порядок
                // идентификаторов свой: сверху, слева, снизу, справа.
                let mm = |v: i64| v as f64 / 100.0;
                match id {
                    1 => doc.landscape = value == 2,
                    2 => doc.print_scale = Some(value),
                    6 => doc.margins.top = mm(value),
                    7 => doc.margins.left = mm(value),
                    8 => doc.margins.bottom = mm(value),
                    9 => doc.margins.right = mm(value),
                    _ => {}
                }
            }
            i += 2;
        }
    }
    let print_area = cur.next()?.group()?;
    if print_area.first().map_or(Ok(0), Node::number)? != 0 && print_area.len() >= 5 {
        doc.print_area = Some((
            print_area[2].number()?.max(0) as u32,
            print_area[1].number()?.max(0) as u32,
            print_area[4].number()?.max(0) as u32,
            print_area[3].number()?.max(0) as u32,
        ));
    }
    cur.skip(10)?;

    let format_count = cur.number()? as usize;
    let mut palette: Vec<Vec<(u64, i64)>> = Vec::with_capacity(format_count);
    for _ in 0..format_count {
        palette.push(parse_format(cur.next()?.group()?)?);
    }
    let font_count = cur.number()? as usize;
    let mut fonts_test: Vec<Font> = Vec::with_capacity(font_count);
    for _ in 0..font_count {
        let g = cur.next()?.group()?;
        fonts_test.push(parse_font(g)?);
    }
    // Форматные строки («ЧДЦ=2» и подобные), затем описания типов и GUID —
    // все три списка переменной длины. В простом документе все три пусты, и
    // раздел выглядит как три нуля подряд; настоящий отчёт показал, что это
    // не константа.
    // Форматные строки («ЧДЦ=2» и подобные): `{1,<кол-во языков>,
    // {<язык>,<строка>},…}`. У пустой записи — ноль языков. Берём
    // русскую строку, а при её отсутствии — первую попавшуюся: как и
    // шаблон, формат хранится в `CellData::format_spec` для ВМ.
    let num_format_count = cur.number()? as usize;
    let mut num_formats: Vec<Option<String>> = Vec::with_capacity(num_format_count);
    for _ in 0..num_format_count {
        let g = cur.next()?.group()?;
        let lang_count = g.get(1).map_or(Ok(0), Node::number)?.max(0) as usize;
        let mut spec = None;
        for i in 0..lang_count {
            if let Some(Node::Group(pair)) = g.get(2 + i) {
                let lang = pair_text(pair_get(pair, 0));
                let value = pair_text(pair_get(pair, 1));
                if lang == "ru" || spec.is_none() {
                    spec = Some(value);
                }
            }
        }
        num_formats.push(spec);
    }
    let type_count = cur.number()? as usize;
    cur.skip(type_count)?;
    let guid = cur.number()? as usize;
    cur.skip(guid)?;
    let color_count = cur.number()? as usize;
    let mut colors: Vec<Option<Color>> = Vec::with_capacity(color_count);
    for _ in 0..color_count {
        let g = cur.next()?.group()?;
        // `{3,0,{<цвет>}}` — заданный цвет, `{3,3,{...}}` — цвет стиля,
        // который мы не моделируем.
        let own = g.get(1).map_or(Ok(3), Node::number)? == 0;
        let value = match g.get(2) {
            Some(Node::Group(v)) => v.first().map_or(Ok(0), Node::number)?,
            _ => 0,
        };
        colors.push(own.then_some(Color {
            r: (value & 0xFF) as u8,
            g: ((value >> 8) & 0xFF) as u8,
            b: ((value >> 16) & 0xFF) as u8,
        }));
    }
    // Палитра рисунков: счётчик и столько же групп с base64-данными.
    // В простом документе счётчик нулевой — тогда это и есть первый из
    // трёх нулей, которые писались раньше через `skip(3)`; поэтому чтение
    // счётчика здесь, а не отдельного `skip`, сохраняет порядок.
    let picture_count = cur.number()? as usize;
    cur.skip(picture_count)?;
    cur.skip(2)?; // 0,0
    cur.skip(1)?; // ""
    cur.skip(1)?; // 0
    if let Ok(Node::Group(settings)) = cur.next() {
        doc.show_grid = settings.get(4).map_or(Ok(1), Node::number)? != 0;
        doc.fix_left = settings.get(11).map_or(Ok(0), Node::number)?;
        doc.fix_top = settings.get(12).map_or(Ok(0), Node::number)?;
    }

    // --- применение форматов --------------------------------------------
    let value = |index: usize, bit: u32| -> Option<i64> {
        palette
            .get(index.checked_sub(1)?)?
            .iter()
            .find(|(b, _)| *b == u64::from(bit))
            .map(|(_, v)| *v)
    };
    for (r, fmt) in row_formats {
        if let Some(v) = value(fmt, bits::ROW_HEIGHT) {
            doc.set_row_height(r, v / ROW_HEIGHT_SCALE);
        }
    }
    for (c, fmt) in column_formats {
        if let Some(v) = value(fmt, bits::COL_WIDTH) {
            doc.set_col_width(c, v / COL_WIDTH_SCALE);
        }
    }
    for (r, c, fmt) in cell_formats {
        if let Some(v) = value(fmt, bits::H_ALIGN) {
            if let Some(a) = HAlign::from_code(v) {
                doc.set_cell_h_align(r, c, a);
            }
        }
        if let Some(v) = value(fmt, bits::V_ALIGN) {
            if let Some(a) = VAlign::from_code(v) {
                doc.set_cell_v_align(r, c, a);
            }
        }
        if value(fmt, bits::TEXT_PLACEMENT) == Some(3) {
            doc.set_cell_wrap(r, c, true);
        }
        if let Some(v) = value(fmt, bits::NUMBER_FORMAT) {
            doc.set_cell_numeric(r, c);
            if let Some(Some(spec)) = num_formats.get(v.max(0) as usize) {
                doc.set_cell_format_spec(r, c, spec);
            }
        }
        if let Some(v) = value(fmt, bits::FONT) {
            if let Some(f) = fonts_test.get(v.max(0) as usize) {
                doc.set_cell_font(r, c, f.clone());
            }
        }
        if let Some(v) = value(fmt, bits::TEXT_COLOR) {
            if let Some(Some(color)) = colors.get(v.max(0) as usize) {
                doc.set_cell_text_color(r, c, *color);
            }
        }
        for (bit, side) in [
            (bits::BORDER_LEFT, 0),
            (bits::BORDER_TOP, 1),
            (bits::BORDER_RIGHT, 2),
            (bits::BORDER_BOTTOM, 3),
        ] {
            if let Some(v) = value(fmt, bit) {
                if let Some(&line) = line_palette.get(v.max(0) as usize) {
                    doc.set_cell_border(
                        r,
                        c,
                        (side == 0).then_some(line),
                        (side == 1).then_some(line),
                        (side == 2).then_some(line),
                        (side == 3).then_some(line),
                    );
                }
            }
        }
        if let Some(v) = value(fmt, bits::BACK_COLOR) {
            if let Some(Some(color)) = colors.get(v.max(0) as usize) {
                doc.set_cell_back_color(r, c, *color);
            }
        }
    }

    // Объявленные размеры НЕ обязаны покрывать содержимое: в отчёте с
    // дополнительными наборами колонок основной набор объявлен из одной
    // колонки, а ячейки заняли пять. Своей модели у наборов нет, поэтому
    // берём большее из объявленного и занятого — иначе половина документа
    // просто исчезла бы.
    // Ширины колонок наборов лежат в той же палитре форматов, что и всё
    // остальное, поэтому наборы собираются только теперь.
    doc.column_sets = raw_sets
        .into_iter()
        .map(|(id, count, formats)| ColumnSet {
            id,
            count,
            widths: formats
                .into_iter()
                .filter_map(|(c, fmt)| Some((c, value(fmt, bits::COL_WIDTH)?)))
                .collect(),
        })
        .collect();
    doc.row_sets = bindings;

    doc.main_columns = width;
    doc.height = doc.height.max(height);
    doc.width = doc.width.max(width);
    doc.row_groups = groups;
    // Геометрия рисунков восстанавливается ПОСЛЕДНЕЙ: она считается по
    // фактическим ширинам колонок и высотам строк, а те приходят из палитры
    // форматов в хвосте файла.
    for (c1, r1, oc1, or1, c2, r2, oc2, or2) in drawing_cells_raw {
        let left = doc.col_start(c1) + oc1;
        let top = doc.row_start(r1) + or1;
        let right = doc.col_start(c2) + oc2;
        let bottom = doc.row_start(r2) + or2;
        let number_of = doc.drawings.len() + 1;
        doc.drawings.push(Drawing {
            left: left as f64 / MM_TO_QP,
            top: top as f64 / MM_TO_QP,
            width: (right - left) as f64 / MM_TO_QP,
            height: (bottom - top) as f64 / MM_TO_QP,
            name: format!("D{number_of}"),
        });
    }
    Ok(doc)
}

/// Формат — маска и значения по возрастанию установленных бит.
///
/// Маска ШИРЕ тридцати двух бит: настоящий отчёт из 1С принёс `8589954623`,
/// это тридцать четыре значащих бита. Обрезав её до `u32`, разбор считал бы
/// не то число значений и разъезжался на всём, что идёт следом.
fn parse_format(g: &[Node]) -> RtResult<Vec<(u64, i64)>> {
    let mask = g.first().map_or(Ok(0), Node::number)? as u64;
    let mut out = Vec::new();
    let mut i = 1;
    for bit in 0..64 {
        let b = 1u64 << bit;
        if mask & b != 0 {
            if let Some(n) = g.get(i) {
                out.push((b, n.number()?));
            }
            i += 1;
        }
    }
    Ok(out)
}

fn parse_font(g: &[Node]) -> RtResult<Font> {
    // Второе поле — вид записи: `0` — абсолютный шрифт с полным набором
    // полей, `2` — ссылка на другой шрифт с модификациями. У ссылки мало
    // полей и вместо кегля — группа-указатель, поэтому читаем только то,
    // что безопасно для обоих видов.
    let absolute = g.get(1).is_none_or(|n| n.number().unwrap_or(0) == 0);
    Ok(Font {
        size: if absolute {
            g.get(3).map_or(Ok(100), Node::number)? / 10
        } else {
            10
        },
        bold: g.get(7).map_or(Ok(400), Node::number)? >= 700,
        italic: g.get(8).map_or(Ok(0), Node::number)? != 0,
        underline: g.get(9).map_or(Ok(0), Node::number)? != 0,
        strikeout: g.get(10).map_or(Ok(0), Node::number)? != 0,
        face: g.get(16).map_or(String::new(), Node::text),
    })
}

fn pair_get(g: &[Node], i: usize) -> Option<&Node> {
    g.get(i)
}

fn pair_text(n: Option<&Node>) -> String {
    n.map_or(String::new(), Node::text)
}
