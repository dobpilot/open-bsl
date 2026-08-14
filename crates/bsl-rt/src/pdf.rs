//! Писатель PDF: объектная модель файла, страницы и контент-потоки.
//!
//! Модуль пишет PDF с нуля — внешних крейтов в этом рабочем пространстве
//! нет, поэтому формат разобран здесь, как до того ZIP и deflate. Объём —
//! ровно тот, что понадобится табличному документу: страницы заданного
//! размера, линии, прямоугольники с заливкой и обводкой, цвета и текст с
//! обязательной кириллицей. Поверхности встроенного языка у модуля пока
//! нет: это слой формата, а не тип BSL.
//!
//! # Что делает платформа
//!
//! Решение о шрифтах принято не по документации, а по образцу, снятому с
//! 8.3.27: `ТабличныйДокумент.Записать(путь,
//! ТипФайлаТабличногоДокумента.PDF)` на четырёх строках с кириллицей,
//! латиницей и цифрами дал 8045 байт (файл лежит в дереве —
//! `tests/conformance/pdf/platform-simple.pdf`, скрипт съёмки рядом).
//! Разбор образца:
//!
//! * заголовок `%PDF-1.7`, классическая таблица `xref`, `trailer` с
//!   `/Size`, `/Root` и `/ID` из двух одинаковых 16-байтовых строк;
//! * `/Length` каждого потока — КОСВЕННАЯ ссылка на отдельный объект, а не
//!   число на месте;
//! * шрифт ОДИН и ВСТРОЕННЫЙ подмножеством: `FontFile2` — сокращённый
//!   sfnt на 8372 байта с таблицами `glyf`, `head`, `hhea`, `hmtx`,
//!   `loca`, `maxp`, `name` (без `cmap` — он при `Identity-H` не нужен);
//!   поверх него `CIDFontType2` с `/CIDToGIDMap /Identity` и `Type0` с
//!   `/Encoding /Identity-H` и `/ToUnicode` на 45 записей `bfchar`;
//! * имя шрифта — `/HPDFAA+AdwaitaSans???????`, где `HPDFAA+` — тег
//!   подмножества, а `AdwaitaSans` — системный шрифт интерфейса ЭТОЙ
//!   машины. Семь вопросительных знаков — настоящие байты 0x3F; чем они
//!   были, не измерено, но на неслучайную догадку наводит длина: у
//!   начертания «Обычный» ровно семь букв, и все — кириллические;
//! * содержимое страницы: `/DeviceRGB CS`/`cs`, `scn`/`SCN` для цвета,
//!   `q`/`Q` вокруг каждой ячейки, `re W n` для отсечения, `re B` для
//!   рамки с заливкой, текст — `BT /F1 8.04 Tf x y Td [...] TJ ET` с
//!   кернингом внутри массива `TJ`;
//! * `/MediaBox [0.00 0.00 595.32 841.92]` — A4, числа с двумя знаками.
//!
//! Съёмка повторена трижды, и два прогона подряд отличаются РОВНО в
//! `/ID`: те же 8045 байт, 56 различающихся байтов на смещениях
//! 7950..8016 — обе шестнадцатеричные строки идентификатора. Ни `/Info`,
//! ни `/CreationDate` платформа не пишет вовсе, поэтому в остальном её
//! вывод на одной машине воспроизводим до байта.
//!
//! # Что делаем мы и почему
//!
//! Путь платформы нам закрыт с двух сторон. Шрифта в дереве нет, а
//! подмножитель TrueType — это разбор и пересборка `glyf`/`loca`/`hmtx`
//! с контрольными суммами, то есть отдельная подсистема, которой в этой
//! задаче не место. И даже будь она — воспроизводимость вывода платформы
//! кончается на границе машины: встраивается подмножество ТОГО шрифта,
//! который нашёлся в системе (здесь — AdwaitaSans, шрифт интерфейса
//! GNOME), так что там, где его нет, те же четыре строки дадут другие
//! байты. От своего писателя мы, наоборот, требуем детерминизма:
//! одинаковый вход — одинаковые байты, на любой машине.
//!
//! Поэтому выбран противоположный полюс: НЕ встраивать ничего и опереться
//! на базовые четырнадцать шрифтов, которые обязан знать любой
//! просмотрщик. Из них взято семейство Courier ([`PdfFont`]) — оно
//! единственное, у которого ширина ОДНА И ТА ЖЕ для любого знака
//! (600/1000 кегля), включая те, для которых Adobe метрик не публиковала
//! вовсе: кириллицу. Это и есть решающий довод. Ширины приходится
//! объявлять самим (`/Widths`), потому что мы переопределяем кодировку, и
//! при пропорциональном Helvetica пришлось бы либо тащить в дерево таблицу
//! метрик AFM, либо выдумывать ширины кириллицы — то есть врать
//! просмотрщику о раскладке. У Courier врать не о чем, а вызывающий может
//! посчитать ширину строки точно: `длина * 0.6 * кегль`.
//!
//! Кодировка — однобайтовая поверх `/WinAnsiEncoding`: знаки ASCII
//! 0x20..0x7E остаются собой, всё остальное получает код из 128..255 в
//! порядке первого появления и попадает в `/Differences` под своим именем
//! глифа. Когда 128 мест кончаются, заводится ещё один ресурс шрифта с
//! новой кодировкой, а строка при выводе режется на отрезки по ресурсам —
//! предела на число различных знаков в документе нет. Порядок кодов —
//! порядок первого появления, поэтому вывод остаётся детерминированным.
//!
//! # Имя глифа решает, нарисуется ли кириллица
//!
//! Здесь пришлось мерить, а не рассуждать, и первый вариант оказался
//! наполовину негодным. Извлечение текста имён глифов не касается вовсе:
//! за него отвечает `/ToUnicode`, который пишется для КАЖДОГО выданного
//! кода, включая ASCII, — `pdftotext` вынимал кириллицу правильно при
//! любом из проб. А вот РИСОВАНИЕ зависит от имени целиком. Пробы (все на
//! этой машине, poppler 25.x, одна и та же строка «Накладная № 7 Latin»,
//! `pdftoppm -r 72`):
//!
//! * `/Type1 /Courier`, имена `uni0410` — кириллица НЕ НАРИСОВАНА, на
//!   странице только «7 Latin»;
//! * `/TrueType /Courier`, имена `uni0410` — то же самое, пусто;
//! * `/TrueType /CourierNew`, имена `uni0410` — то же самое, пусто;
//! * `/TrueType /DejaVuSansMono`, имена `uni0410` — нарисовано верно;
//! * `/Type1 /Courier`, имена `afii10017` — НАРИСОВАНО ВЕРНО, и обычным
//!   начертанием, и полужирным.
//!
//! Причина видна из этого же набора: подстановочный шрифт для базовых
//! четырнадцати — это Type1 (`fc-match Courier` даёт Nimbus Mono PS), и
//! глиф в нём ищется ПО ИМЕНИ. Имени `uni0410` там нет, а `afii10017`
//! есть. Вариант с DejaVu рисует лишь потому, что попал в настоящий
//! TrueType с таблицей `cmap`, то есть держится на том, какие шрифты
//! установлены в системе, — ровно та зависимость, от которой мы уходили.
//!
//! Строка `CourierNew` в этом наборе — предостережение тому, кто решит
//! «взять имя поновее»: `fc-match "Courier New"` даёт Liberation Mono,
//! настоящий TrueType с `cmap`, и всё равно НЕ НАРИСОВАНО. Через
//! fontconfig просмотрщик это имя, судя по всему, не пропускает вовсе,
//! а считает его псевдонимом базового Courier — но это уже объяснение, а
//! измерено только то, что вариант пуст.
//!
//! Отсюда правило: имя глифа берётся из Adobe Glyph List (`AGL_NAMES` —
//! кириллица, Latin-1, типографская пунктуация, валюты), и только для
//! знака, которого в списке нет, остаётся `uniXXXX`/`uXXXXX`. Такой знак
//! по-прежнему извлечётся, но может не нарисоваться — граница честная и
//! проходит там, где кончается список Adobe.
//!
//! Осознанная плата за отказ от встраивания — моноширинный вид документа:
//! все начертания Courier рисуются одной шириной, а глифы даёт
//! подстановочный шрифт просмотрщика. Замена этого слоя на встроенный
//! TrueType ничего не сломает выше: наружу видны [`PdfFont`] и точка входа
//! [`PdfDocument::text`], а не кодировка.
//!
//! # Версия формата
//!
//! Пишется `%PDF-1.4`, хотя платформа объявляет 1.7: ничего новее
//! `FlateDecode` (PDF 1.2) в выводе нет, а меньшая версия расширяет круг
//! просмотрщиков. `/ID` в трейлере не пишется — он не обязателен для
//! незашифрованного документа, а осмысленного значения у него нет ни
//! одного, которое не ломало бы детерминизм: именно на `/ID` и
//! разъезжаются два прогона платформы, других различий между ними нет.

use std::collections::BTreeMap;

use crate::deflate::deflate;
use crate::{RtError, RtResult};

// ---------------------------------------------------------------------------
// zlib поверх сырого deflate
// ---------------------------------------------------------------------------

/// Контрольная сумма Adler-32 (RFC 1950): две суммы по модулю 65521,
/// младшая — сумма байтов, старшая — сумма младших.
///
/// Модуль берётся не на каждом шаге, а на границе блока: 5552 — наибольшее
/// число байтов, при котором `s2` заведомо не переполняет `u32`.
fn adler32(data: &[u8]) -> u32 {
    const BASE: u32 = 65521;
    const NMAX: usize = 5552;
    let mut s1: u32 = 1;
    let mut s2: u32 = 0;
    for chunk in data.chunks(NMAX) {
        for &b in chunk {
            s1 += u32::from(b);
            s2 += s1;
        }
        s1 %= BASE;
        s2 %= BASE;
    }
    (s2 << 16) | s1
}

/// Обернуть сырой поток RFC 1951 в оболочку RFC 1950 — то, что PDF
/// называет `/Filter /FlateDecode`.
///
/// Заголовок: CMF `0x78` (метод 8, окно 32 КиБ) и FLG, подобранный так,
/// чтобы `(CMF * 256 + FLG) % 31 == 0` — распаковщики эту пару проверяют.
/// Биты уровня сжатия в FLG носят справочный характер; здесь стоит `0x9C`
/// (уровень «по умолчанию»), потому что [`deflate`] всегда пишет
/// фиксированные коды. Хвост — Adler-32 РАСПАКОВАННЫХ данных, старшим
/// байтом вперёд.
fn zlib_compress(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() / 2 + 16);
    out.push(0x78);
    out.push(0x9C);
    out.extend_from_slice(&deflate(data));
    out.extend_from_slice(&adler32(data).to_be_bytes());
    out
}

// ---------------------------------------------------------------------------
// Объектная модель файла
// ---------------------------------------------------------------------------

/// Значение PDF. Ровно те виды, которые встречаются в выводе этого
/// писателя: логических значений и `null` он не порождает, а массивы и
/// словари хранят порядок, потому что от вывода требуется детерминизм.
#[derive(Debug, Clone, PartialEq)]
pub enum PdfValue {
    /// Целое число.
    Integer(i64),
    /// Вещественное число. Записывается без экспоненты — её синтаксис PDF
    /// не знает.
    Real(f64),
    /// Имя. Хранится БЕЗ ведущей косой черты, она добавляется при записи.
    Name(String),
    /// Строка в круглых скобках со скобочным экранированием.
    Str(Vec<u8>),
    /// Массив.
    Array(Vec<PdfValue>),
    /// Словарь: пары «ключ — значение» в порядке записи.
    Dict(Vec<(String, PdfValue)>),
    /// Косвенная ссылка `N 0 R`. Поколение всегда нулевое: этот писатель
    /// не обновляет файлы по месту, а собирает их целиком.
    Ref(u32),
    /// Поток: словарь и байты. `/Length` в словаре не хранится — он
    /// вычисляется и дописывается при записи, чтобы не разъехаться с
    /// данными.
    Stream {
        /// Словарь потока без `/Length`.
        dict: Vec<(String, PdfValue)>,
        /// Байты потока — уже отфильтрованные, если в словаре объявлен
        /// `/Filter`.
        data: Vec<u8>,
    },
}

/// Записать вещественное число так, как его понимает PDF: без экспоненты,
/// без хвостовых нулей и без «минус нуля».
///
/// Четырёх знаков после запятой хватает с запасом: единица пользовательских
/// координат — точка, то есть 1/72 дюйма, и 0.0001 точки не различит ни
/// один просмотрщик.
fn fmt_real(x: f64) -> String {
    let mut s = format!("{x:.4}");
    if s.contains('.') {
        while s.ends_with('0') {
            s.pop();
        }
        if s.ends_with('.') {
            s.pop();
        }
    }
    if s == "-0" {
        s.remove(0);
    }
    s
}

/// Записать имя. Знаки вне «обычных» кодируются как `#XX`, как требует
/// спецификация; имена, которые порождает этот модуль, до этого не
/// доходят, но проверка стоит на входе, а не на честном слове.
fn write_name(out: &mut Vec<u8>, name: &str) {
    out.push(b'/');
    for &b in name.as_bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'+' | b'_' | b'.') {
            out.push(b);
        } else {
            out.extend_from_slice(format!("#{b:02X}").as_bytes());
        }
    }
}

/// Записать строку в скобках. Экранируются три знака, обязательных по
/// спецификации, всё непечатное уходит в восьмеричную форму `\ooo` —
/// так файл остаётся текстом при просмотре и не зависит от того, как
/// чужой разбор поступает с сырыми байтами.
fn write_str(out: &mut Vec<u8>, bytes: &[u8]) {
    out.push(b'(');
    for &b in bytes {
        match b {
            b'(' | b')' | b'\\' => {
                out.push(b'\\');
                out.push(b);
            }
            0x20..=0x7E => out.push(b),
            _ => out.extend_from_slice(format!("\\{b:03o}").as_bytes()),
        }
    }
    out.push(b')');
}

/// Записать значение. Потоки допустимы только как тело косвенного
/// объекта — так и вызывается, из [`PdfDocument::write`].
fn write_value(out: &mut Vec<u8>, value: &PdfValue) {
    match value {
        PdfValue::Integer(n) => out.extend_from_slice(n.to_string().as_bytes()),
        PdfValue::Real(x) => out.extend_from_slice(fmt_real(*x).as_bytes()),
        PdfValue::Name(name) => write_name(out, name),
        PdfValue::Str(bytes) => write_str(out, bytes),
        PdfValue::Array(items) => {
            out.push(b'[');
            for item in items {
                out.push(b' ');
                write_value(out, item);
            }
            out.extend_from_slice(b" ]");
        }
        PdfValue::Dict(entries) => {
            out.extend_from_slice(b"<<");
            for (key, item) in entries {
                out.push(b' ');
                write_name(out, key);
                out.push(b' ');
                write_value(out, item);
            }
            out.extend_from_slice(b" >>");
        }
        PdfValue::Ref(n) => out.extend_from_slice(format!("{n} 0 R").as_bytes()),
        PdfValue::Stream { dict, data } => {
            let mut full = dict.clone();
            full.push(("Length".to_string(), PdfValue::Integer(data.len() as i64)));
            write_value(out, &PdfValue::Dict(full));
            out.extend_from_slice(b"\nstream\n");
            out.extend_from_slice(data);
            out.extend_from_slice(b"\nendstream");
        }
    }
}

// ---------------------------------------------------------------------------
// Шрифты и кодировка
// ---------------------------------------------------------------------------

/// Начертание из семейства Courier — единственного среди базовых
/// четырнадцати, где ширина знака одна для всех глифов.
///
/// Почему именно оно и почему шрифт не встраивается — в обзоре модуля.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PdfFont {
    /// `Courier`.
    Courier,
    /// `Courier-Bold`.
    CourierBold,
    /// `Courier-Oblique`.
    CourierOblique,
    /// `Courier-BoldOblique`.
    CourierBoldOblique,
}

impl PdfFont {
    /// Ширина любого знака в тысячных долях кегля. Ширина строки в точках
    /// равна `количество знаков * ADVANCE / 1000 * кегль`.
    pub const ADVANCE: f64 = 600.0;

    /// Имя шрифта для `/BaseFont`.
    pub fn base_font(self) -> &'static str {
        match self {
            PdfFont::Courier => "Courier",
            PdfFont::CourierBold => "Courier-Bold",
            PdfFont::CourierOblique => "Courier-Oblique",
            PdfFont::CourierBoldOblique => "Courier-BoldOblique",
        }
    }

    /// Ширина строки в точках при заданном кегле.
    pub fn text_width(self, text: &str, size: f64) -> f64 {
        text.chars().count() as f64 * Self::ADVANCE / 1000.0 * size
    }
}

/// Первый код, выдаваемый неанглийским знакам. Ниже лежит ASCII, который
/// берётся из `/WinAnsiEncoding` без переопределения.
const FIRST_CUSTOM_CODE: u16 = 128;

/// Первый код, для которого объявляются ширины: ниже пробела печатных
/// знаков нет.
const FIRST_ASCII_CODE: u16 = 32;

/// Один ресурс шрифта в файле: начертание плюс своя однобайтовая
/// кодировка. Когда коды кончаются, заводится следующий такой же.
struct FontPlane {
    font: PdfFont,
    /// Код — знак. Заполняется по мере ВЫВОДА: сюда попадает и ASCII,
    /// чтобы `/ToUnicode` покрывал каждый выданный код.
    by_code: Vec<Option<char>>,
    /// Знак — код, для знаков вне ASCII.
    by_char: BTreeMap<char, u8>,
    /// Следующий свободный код; `FIRST_CUSTOM_CODE + 128` означает, что
    /// план заполнен.
    next: u16,
}

impl FontPlane {
    fn new(font: PdfFont) -> Self {
        FontPlane {
            font,
            by_code: vec![None; 256],
            by_char: BTreeMap::new(),
            next: FIRST_CUSTOM_CODE,
        }
    }

    /// Код для знака в этом плане либо `None`, если места нет.
    fn code_for(&mut self, ch: char) -> Option<u8> {
        if matches!(ch, ' '..='~') {
            let code = ch as u8;
            self.by_code[code as usize] = Some(ch);
            return Some(code);
        }
        if let Some(&code) = self.by_char.get(&ch) {
            return Some(code);
        }
        if self.next > u16::from(u8::MAX) {
            return None;
        }
        let code = self.next as u8;
        self.next += 1;
        self.by_char.insert(ch, code);
        self.by_code[code as usize] = Some(ch);
        Some(code)
    }
}

/// Все ресурсы шрифтов документа в порядке появления. Порядок и есть
/// нумерация `/F1`, `/F2`, ….
struct FontBook {
    planes: Vec<FontPlane>,
}

impl FontBook {
    fn new() -> Self {
        FontBook { planes: Vec::new() }
    }

    /// Выдать знаку место: номер плана и код в нём. Знак идёт в первый
    /// план нужного начертания, который его уже знает или может принять;
    /// если такого нет — заводится новый план.
    fn encode(&mut self, font: PdfFont, ch: char) -> (usize, u8) {
        for (index, plane) in self.planes.iter_mut().enumerate() {
            if plane.font != font {
                continue;
            }
            if let Some(code) = plane.code_for(ch) {
                return (index, code);
            }
        }
        let mut plane = FontPlane::new(font);
        // Новый план пуст, поэтому место в нём есть заведомо.
        let code = plane
            .code_for(ch)
            .expect("в только что созданном плане кодировки не может не быть места");
        self.planes.push(plane);
        (self.planes.len() - 1, code)
    }

    /// Имя ресурса шрифта в словаре `/Font` страницы.
    fn resource_name(index: usize) -> String {
        format!("F{}", index + 1)
    }
}

/// Имя глифа для `/Differences`.
///
/// Сначала — СОБСТВЕННОЕ имя из Adobe Glyph List (`AGL_NAMES`), и только
/// если знака там нет — единообразная форма `uniXXXX` (для BMP) или
/// `uXXXXX` (выше BMP). Порядок именно такой, и это измерено, а не
/// выбрано по вкусу: `uni0410` подстановочный шрифт НЕ РИСУЕТ, а
/// `afii10017` рисует — подробности в обзоре модуля. Текст извлекается
/// правильно в обоих случаях, потому что за извлечение отвечает
/// `/ToUnicode`.
fn glyph_name(ch: char) -> String {
    let cp = ch as u32;
    if let Ok(index) = AGL_NAMES.binary_search_by_key(&cp, |&(cp, _)| cp) {
        return AGL_NAMES[index].1.to_string();
    }
    if cp <= 0xFFFF {
        format!("uni{cp:04X}")
    } else {
        format!("u{cp:05X}")
    }
}

/// Имена глифов из Adobe Glyph List для знаков, которые встречаются в
/// русском деловом документе: Latin-1, вся кириллица, типографская
/// пунктуация и знаки валют. Отсортировано по коду — читается двоичным
/// поиском.
///
/// Таблица не сочинена, а перенесена из списка, лежащего в системе вместе
/// с ghostscript (`gs_agl.ps`, «derived from the Adobe Glyph List, version
/// 2.0, dated September 20, 2002»). Происхождение:
///
/// ```python
/// import re
/// src = open('/usr/share/ghostscript/Resource/Init/gs_agl.ps',
///            encoding='latin1').read()
/// rev = {}
/// for name, cp in re.findall(r'^/([A-Za-z0-9_.]+) 16#([0-9A-Fa-f]{4,6})$', src, re.M):
///     rev.setdefault(int(cp, 16), []).append(name)
/// # при нескольких именах берётся afii-имя: именно ими зовут кириллицу
/// # шрифты Type1, и именно по ним подстановочный шрифт находит глиф
/// ```
///
/// Диапазоны: `U+00A0..U+00FF`, `U+0400..U+04FF`, `U+2010..U+2044`,
/// `U+20A0..U+20BF`, `U+2116`, `U+2122`.
static AGL_NAMES: &[(u32, &str)] = &[
    (0x00A0, "nbspace"),
    (0x00A1, "exclamdown"),
    (0x00A2, "cent"),
    (0x00A3, "sterling"),
    (0x00A4, "currency"),
    (0x00A5, "yen"),
    (0x00A6, "brokenbar"),
    (0x00A7, "section"),
    (0x00A8, "dieresis"),
    (0x00A9, "copyright"),
    (0x00AA, "ordfeminine"),
    (0x00AB, "guillemotleft"),
    (0x00AC, "logicalnot"),
    (0x00AD, "sfthyphen"),
    (0x00AE, "registered"),
    (0x00AF, "macron"),
    (0x00B0, "degree"),
    (0x00B1, "plusminus"),
    (0x00B2, "twosuperior"),
    (0x00B3, "threesuperior"),
    (0x00B4, "acute"),
    (0x00B5, "mu"),
    (0x00B6, "paragraph"),
    (0x00B7, "middot"),
    (0x00B8, "cedilla"),
    (0x00B9, "onesuperior"),
    (0x00BA, "ordmasculine"),
    (0x00BB, "guillemotright"),
    (0x00BC, "onequarter"),
    (0x00BD, "onehalf"),
    (0x00BE, "threequarters"),
    (0x00BF, "questiondown"),
    (0x00C0, "Agrave"),
    (0x00C1, "Aacute"),
    (0x00C2, "Acircumflex"),
    (0x00C3, "Atilde"),
    (0x00C4, "Adieresis"),
    (0x00C5, "Aring"),
    (0x00C6, "AE"),
    (0x00C7, "Ccedilla"),
    (0x00C8, "Egrave"),
    (0x00C9, "Eacute"),
    (0x00CA, "Ecircumflex"),
    (0x00CB, "Edieresis"),
    (0x00CC, "Igrave"),
    (0x00CD, "Iacute"),
    (0x00CE, "Icircumflex"),
    (0x00CF, "Idieresis"),
    (0x00D0, "Eth"),
    (0x00D1, "Ntilde"),
    (0x00D2, "Ograve"),
    (0x00D3, "Oacute"),
    (0x00D4, "Ocircumflex"),
    (0x00D5, "Otilde"),
    (0x00D6, "Odieresis"),
    (0x00D7, "multiply"),
    (0x00D8, "Oslash"),
    (0x00D9, "Ugrave"),
    (0x00DA, "Uacute"),
    (0x00DB, "Ucircumflex"),
    (0x00DC, "Udieresis"),
    (0x00DD, "Yacute"),
    (0x00DE, "Thorn"),
    (0x00DF, "germandbls"),
    (0x00E0, "agrave"),
    (0x00E1, "aacute"),
    (0x00E2, "acircumflex"),
    (0x00E3, "atilde"),
    (0x00E4, "adieresis"),
    (0x00E5, "aring"),
    (0x00E6, "ae"),
    (0x00E7, "ccedilla"),
    (0x00E8, "egrave"),
    (0x00E9, "eacute"),
    (0x00EA, "ecircumflex"),
    (0x00EB, "edieresis"),
    (0x00EC, "igrave"),
    (0x00ED, "iacute"),
    (0x00EE, "icircumflex"),
    (0x00EF, "idieresis"),
    (0x00F0, "eth"),
    (0x00F1, "ntilde"),
    (0x00F2, "ograve"),
    (0x00F3, "oacute"),
    (0x00F4, "ocircumflex"),
    (0x00F5, "otilde"),
    (0x00F6, "odieresis"),
    (0x00F7, "divide"),
    (0x00F8, "oslash"),
    (0x00F9, "ugrave"),
    (0x00FA, "uacute"),
    (0x00FB, "ucircumflex"),
    (0x00FC, "udieresis"),
    (0x00FD, "yacute"),
    (0x00FE, "thorn"),
    (0x00FF, "ydieresis"),
    (0x0401, "afii10023"),
    (0x0402, "afii10051"),
    (0x0403, "afii10052"),
    (0x0404, "afii10053"),
    (0x0405, "afii10054"),
    (0x0406, "afii10055"),
    (0x0407, "afii10056"),
    (0x0408, "afii10057"),
    (0x0409, "afii10058"),
    (0x040A, "afii10059"),
    (0x040B, "afii10060"),
    (0x040C, "afii10061"),
    (0x040E, "afii10062"),
    (0x040F, "afii10145"),
    (0x0410, "afii10017"),
    (0x0411, "afii10018"),
    (0x0412, "afii10019"),
    (0x0413, "afii10020"),
    (0x0414, "afii10021"),
    (0x0415, "afii10022"),
    (0x0416, "afii10024"),
    (0x0417, "afii10025"),
    (0x0418, "afii10026"),
    (0x0419, "afii10027"),
    (0x041A, "afii10028"),
    (0x041B, "afii10029"),
    (0x041C, "afii10030"),
    (0x041D, "afii10031"),
    (0x041E, "afii10032"),
    (0x041F, "afii10033"),
    (0x0420, "afii10034"),
    (0x0421, "afii10035"),
    (0x0422, "afii10036"),
    (0x0423, "afii10037"),
    (0x0424, "afii10038"),
    (0x0425, "afii10039"),
    (0x0426, "afii10040"),
    (0x0427, "afii10041"),
    (0x0428, "afii10042"),
    (0x0429, "afii10043"),
    (0x042A, "afii10044"),
    (0x042B, "afii10045"),
    (0x042C, "afii10046"),
    (0x042D, "afii10047"),
    (0x042E, "afii10048"),
    (0x042F, "afii10049"),
    (0x0430, "afii10065"),
    (0x0431, "afii10066"),
    (0x0432, "afii10067"),
    (0x0433, "afii10068"),
    (0x0434, "afii10069"),
    (0x0435, "afii10070"),
    (0x0436, "afii10072"),
    (0x0437, "afii10073"),
    (0x0438, "afii10074"),
    (0x0439, "afii10075"),
    (0x043A, "afii10076"),
    (0x043B, "afii10077"),
    (0x043C, "afii10078"),
    (0x043D, "afii10079"),
    (0x043E, "afii10080"),
    (0x043F, "afii10081"),
    (0x0440, "afii10082"),
    (0x0441, "afii10083"),
    (0x0442, "afii10084"),
    (0x0443, "afii10085"),
    (0x0444, "afii10086"),
    (0x0445, "afii10087"),
    (0x0446, "afii10088"),
    (0x0447, "afii10089"),
    (0x0448, "afii10090"),
    (0x0449, "afii10091"),
    (0x044A, "afii10092"),
    (0x044B, "afii10093"),
    (0x044C, "afii10094"),
    (0x044D, "afii10095"),
    (0x044E, "afii10096"),
    (0x044F, "afii10097"),
    (0x0451, "afii10071"),
    (0x0452, "afii10099"),
    (0x0453, "afii10100"),
    (0x0454, "afii10101"),
    (0x0455, "afii10102"),
    (0x0456, "afii10103"),
    (0x0457, "afii10104"),
    (0x0458, "afii10105"),
    (0x0459, "afii10106"),
    (0x045A, "afii10107"),
    (0x045B, "afii10108"),
    (0x045C, "afii10109"),
    (0x045E, "afii10110"),
    (0x045F, "afii10193"),
    (0x0460, "Omegacyrillic"),
    (0x0461, "omegacyrillic"),
    (0x0462, "afii10146"),
    (0x0463, "afii10194"),
    (0x0464, "Eiotifiedcyrillic"),
    (0x0465, "eiotifiedcyrillic"),
    (0x0466, "Yuslittlecyrillic"),
    (0x0467, "yuslittlecyrillic"),
    (0x0468, "Yuslittleiotifiedcyrillic"),
    (0x0469, "yuslittleiotifiedcyrillic"),
    (0x046A, "Yusbigcyrillic"),
    (0x046B, "yusbigcyrillic"),
    (0x046C, "Yusbigiotifiedcyrillic"),
    (0x046D, "yusbigiotifiedcyrillic"),
    (0x046E, "Ksicyrillic"),
    (0x046F, "ksicyrillic"),
    (0x0470, "Psicyrillic"),
    (0x0471, "psicyrillic"),
    (0x0472, "afii10147"),
    (0x0473, "afii10195"),
    (0x0474, "afii10148"),
    (0x0475, "afii10196"),
    (0x0476, "Izhitsadblgravecyrillic"),
    (0x0477, "izhitsadblgravecyrillic"),
    (0x0478, "Ukcyrillic"),
    (0x0479, "ukcyrillic"),
    (0x047A, "Omegaroundcyrillic"),
    (0x047B, "omegaroundcyrillic"),
    (0x047C, "Omegatitlocyrillic"),
    (0x047D, "omegatitlocyrillic"),
    (0x047E, "Otcyrillic"),
    (0x047F, "otcyrillic"),
    (0x0480, "Koppacyrillic"),
    (0x0481, "koppacyrillic"),
    (0x0482, "thousandcyrillic"),
    (0x0483, "titlocyrilliccmb"),
    (0x0484, "palatalizationcyrilliccmb"),
    (0x0485, "dasiapneumatacyrilliccmb"),
    (0x0486, "psilipneumatacyrilliccmb"),
    (0x0490, "afii10050"),
    (0x0491, "afii10098"),
    (0x0492, "Ghestrokecyrillic"),
    (0x0493, "ghestrokecyrillic"),
    (0x0494, "Ghemiddlehookcyrillic"),
    (0x0495, "ghemiddlehookcyrillic"),
    (0x0496, "Zhedescendercyrillic"),
    (0x0497, "zhedescendercyrillic"),
    (0x0498, "Zedescendercyrillic"),
    (0x0499, "zedescendercyrillic"),
    (0x049A, "Kadescendercyrillic"),
    (0x049B, "kadescendercyrillic"),
    (0x049C, "Kaverticalstrokecyrillic"),
    (0x049D, "kaverticalstrokecyrillic"),
    (0x049E, "Kastrokecyrillic"),
    (0x049F, "kastrokecyrillic"),
    (0x04A0, "Kabashkircyrillic"),
    (0x04A1, "kabashkircyrillic"),
    (0x04A2, "Endescendercyrillic"),
    (0x04A3, "endescendercyrillic"),
    (0x04A4, "Enghecyrillic"),
    (0x04A5, "enghecyrillic"),
    (0x04A6, "Pemiddlehookcyrillic"),
    (0x04A7, "pemiddlehookcyrillic"),
    (0x04A8, "Haabkhasiancyrillic"),
    (0x04A9, "haabkhasiancyrillic"),
    (0x04AA, "Esdescendercyrillic"),
    (0x04AB, "esdescendercyrillic"),
    (0x04AC, "Tedescendercyrillic"),
    (0x04AD, "tedescendercyrillic"),
    (0x04AE, "Ustraightcyrillic"),
    (0x04AF, "ustraightcyrillic"),
    (0x04B0, "Ustraightstrokecyrillic"),
    (0x04B1, "ustraightstrokecyrillic"),
    (0x04B2, "Hadescendercyrillic"),
    (0x04B3, "hadescendercyrillic"),
    (0x04B4, "Tetsecyrillic"),
    (0x04B5, "tetsecyrillic"),
    (0x04B6, "Chedescendercyrillic"),
    (0x04B7, "chedescendercyrillic"),
    (0x04B8, "Cheverticalstrokecyrillic"),
    (0x04B9, "cheverticalstrokecyrillic"),
    (0x04BA, "Shhacyrillic"),
    (0x04BB, "shhacyrillic"),
    (0x04BC, "Cheabkhasiancyrillic"),
    (0x04BD, "cheabkhasiancyrillic"),
    (0x04BE, "Chedescenderabkhasiancyrillic"),
    (0x04BF, "chedescenderabkhasiancyrillic"),
    (0x04C0, "palochkacyrillic"),
    (0x04C1, "Zhebrevecyrillic"),
    (0x04C2, "zhebrevecyrillic"),
    (0x04C3, "Kahookcyrillic"),
    (0x04C4, "kahookcyrillic"),
    (0x04C7, "Enhookcyrillic"),
    (0x04C8, "enhookcyrillic"),
    (0x04CB, "Chekhakassiancyrillic"),
    (0x04CC, "chekhakassiancyrillic"),
    (0x04D0, "Abrevecyrillic"),
    (0x04D1, "abrevecyrillic"),
    (0x04D2, "Adieresiscyrillic"),
    (0x04D3, "adieresiscyrillic"),
    (0x04D4, "Aiecyrillic"),
    (0x04D5, "aiecyrillic"),
    (0x04D6, "Iebrevecyrillic"),
    (0x04D7, "iebrevecyrillic"),
    (0x04D8, "Schwacyrillic"),
    (0x04D9, "afii10846"),
    (0x04DA, "Schwadieresiscyrillic"),
    (0x04DB, "schwadieresiscyrillic"),
    (0x04DC, "Zhedieresiscyrillic"),
    (0x04DD, "zhedieresiscyrillic"),
    (0x04DE, "Zedieresiscyrillic"),
    (0x04DF, "zedieresiscyrillic"),
    (0x04E0, "Dzeabkhasiancyrillic"),
    (0x04E1, "dzeabkhasiancyrillic"),
    (0x04E2, "Imacroncyrillic"),
    (0x04E3, "imacroncyrillic"),
    (0x04E4, "Idieresiscyrillic"),
    (0x04E5, "idieresiscyrillic"),
    (0x04E6, "Odieresiscyrillic"),
    (0x04E7, "odieresiscyrillic"),
    (0x04E8, "Obarredcyrillic"),
    (0x04E9, "obarredcyrillic"),
    (0x04EA, "Obarreddieresiscyrillic"),
    (0x04EB, "obarreddieresiscyrillic"),
    (0x04EE, "Umacroncyrillic"),
    (0x04EF, "umacroncyrillic"),
    (0x04F0, "Udieresiscyrillic"),
    (0x04F1, "udieresiscyrillic"),
    (0x04F2, "Uhungarumlautcyrillic"),
    (0x04F3, "uhungarumlautcyrillic"),
    (0x04F4, "Chedieresiscyrillic"),
    (0x04F5, "chedieresiscyrillic"),
    (0x04F8, "Yerudieresiscyrillic"),
    (0x04F9, "yerudieresiscyrillic"),
    (0x2010, "hyphentwo"),
    (0x2012, "figuredash"),
    (0x2013, "endash"),
    (0x2014, "emdash"),
    (0x2015, "afii00208"),
    (0x2016, "dblverticalbar"),
    (0x2017, "dbllowline"),
    (0x2018, "quoteleft"),
    (0x2019, "quoteright"),
    (0x201A, "quotesinglbase"),
    (0x201B, "quoteleftreversed"),
    (0x201C, "quotedblleft"),
    (0x201D, "quotedblright"),
    (0x201E, "quotedblbase"),
    (0x2020, "dagger"),
    (0x2021, "daggerdbl"),
    (0x2022, "bullet"),
    (0x2024, "onedotenleader"),
    (0x2025, "twodotenleader"),
    (0x2026, "ellipsis"),
    (0x202C, "afii61573"),
    (0x202D, "afii61574"),
    (0x202E, "afii61575"),
    (0x2030, "perthousand"),
    (0x2032, "minute"),
    (0x2033, "second"),
    (0x2035, "primereversed"),
    (0x2039, "guilsinglleft"),
    (0x203A, "guilsinglright"),
    (0x203B, "referencemark"),
    (0x203C, "exclamdbl"),
    (0x203E, "overline"),
    (0x2042, "asterism"),
    (0x2044, "fraction"),
    (0x20A1, "colonmonetary"),
    (0x20A2, "cruzeiro"),
    (0x20A3, "franc"),
    (0x20A4, "afii08941"),
    (0x20A7, "peseta"),
    (0x20A9, "won"),
    (0x20AA, "afii57636"),
    (0x20AB, "dong"),
    (0x20AC, "Euro"),
    (0x2116, "afii61352"),
    (0x2122, "trademark"),
];

/// Собрать CMap `/ToUnicode` для одного плана кодировки.
///
/// Записи `bfchar` разбиты на блоки не длиннее ста — таков предел,
/// установленный спецификацией CMap.
fn to_unicode_cmap(plane: &FontPlane) -> Vec<u8> {
    let used: Vec<(u8, char)> = plane
        .by_code
        .iter()
        .enumerate()
        .filter_map(|(code, ch)| ch.map(|ch| (code as u8, ch)))
        .collect();

    let mut out = String::new();
    out.push_str("/CIDInit /ProcSet findresource begin\n");
    out.push_str("12 dict begin\nbegincmap\n");
    out.push_str("/CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def\n");
    out.push_str("/CMapName /Adobe-Identity-UCS def\n");
    out.push_str("/CMapType 2 def\n");
    out.push_str("1 begincodespacerange\n<00> <FF>\nendcodespacerange\n");
    for block in used.chunks(100) {
        out.push_str(&format!("{} beginbfchar\n", block.len()));
        for &(code, ch) in block {
            let mut utf16 = [0u16; 2];
            let units = ch.encode_utf16(&mut utf16);
            let dst: String = units.iter().map(|u| format!("{u:04X}")).collect();
            out.push_str(&format!("<{code:02X}> <{dst}>\n"));
        }
        out.push_str("endbfchar\n");
    }
    out.push_str("endcmap\nCMapName currentdict /CMap defineresource pop\nend\nend\n");
    out.into_bytes()
}

/// Словарь шрифта для одного плана кодировки.
fn font_dict(plane: &FontPlane, to_unicode: u32) -> PdfValue {
    // Ширины объявляются явно: кодировка переопределена, и полагаться на
    // встроенные метрики базового шрифта для собственных имён глифов
    // нельзя. У Courier все они равны.
    let widths = (FIRST_ASCII_CODE..=u16::from(u8::MAX))
        .map(|_| PdfValue::Real(PdfFont::ADVANCE))
        .collect();

    // `/Differences` — только коды поверх WinAnsi. Пишутся пробегами:
    // число, затем имена подряд идущих кодов.
    let mut differences: Vec<PdfValue> = Vec::new();
    let mut expected: Option<u16> = None;
    for (code, ch) in plane.by_code.iter().enumerate() {
        let (code, ch) = match ch {
            Some(ch) if code as u16 >= FIRST_CUSTOM_CODE => (code as u16, *ch),
            Some(_) | None => continue,
        };
        if expected != Some(code) {
            differences.push(PdfValue::Integer(i64::from(code)));
        }
        differences.push(PdfValue::Name(glyph_name(ch)));
        expected = Some(code + 1);
    }

    let encoding = PdfValue::Dict(vec![
        ("Type".to_string(), PdfValue::Name("Encoding".to_string())),
        (
            "BaseEncoding".to_string(),
            PdfValue::Name("WinAnsiEncoding".to_string()),
        ),
        ("Differences".to_string(), PdfValue::Array(differences)),
    ]);

    PdfValue::Dict(vec![
        ("Type".to_string(), PdfValue::Name("Font".to_string())),
        ("Subtype".to_string(), PdfValue::Name("Type1".to_string())),
        (
            "BaseFont".to_string(),
            PdfValue::Name(plane.font.base_font().to_string()),
        ),
        (
            "FirstChar".to_string(),
            PdfValue::Integer(i64::from(FIRST_ASCII_CODE)),
        ),
        (
            "LastChar".to_string(),
            PdfValue::Integer(i64::from(u8::MAX)),
        ),
        ("Widths".to_string(), PdfValue::Array(widths)),
        ("Encoding".to_string(), encoding),
        ("ToUnicode".to_string(), PdfValue::Ref(to_unicode)),
    ])
}

// ---------------------------------------------------------------------------
// Страницы и примитивы
// ---------------------------------------------------------------------------

/// Как закрашивать контур: заливкой, обводкой или тем и другим.
/// Соответствует операторам `f`, `S` и `B`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaintMode {
    /// Только заливка — оператор `f`.
    Fill,
    /// Только обводка — оператор `S`.
    Stroke,
    /// Заливка и поверх неё обводка — оператор `B`.
    FillAndStroke,
}

impl PaintMode {
    fn operator(self) -> &'static str {
        match self {
            PaintMode::Fill => "f",
            PaintMode::Stroke => "S",
            PaintMode::FillAndStroke => "B",
        }
    }
}

/// Номер страницы в документе. Выдаётся [`PdfDocument::add_page`] и годится
/// только для того документа, который его выдал.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageId(usize);

/// Страница: размер и накопленный контент-поток.
struct Page {
    width: f64,
    height: f64,
    content: Vec<u8>,
}

/// Проверка входного числа: NaN и бесконечность в файл не попадают — их
/// синтаксис PDF не знает, и просмотрщик получил бы битый документ.
fn finite(what: &'static str, value: f64) -> RtResult<f64> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(RtError::Pdf(format!(
            "{what}: ожидалось конечное число, получено {value}"
        )))
    }
}

/// Составляющая цвета: конечное число от нуля до единицы.
fn component(what: &'static str, value: f64) -> RtResult<f64> {
    let value = finite(what, value)?;
    if (0.0..=1.0).contains(&value) {
        Ok(value)
    } else {
        Err(RtError::Pdf(format!(
            "{what}: составляющая цвета вне диапазона 0..1, получено {value}"
        )))
    }
}

/// Документ: страницы, их содержимое и общая на всех книга шрифтов.
///
/// Порядок работы — набрать страницы примитивами и один раз вызвать
/// [`PdfDocument::write`]. Писатель однопроходный, но собирает объекты в
/// памяти: смещения нужны для таблицы `xref`.
pub struct PdfDocument {
    pages: Vec<Page>,
    fonts: FontBook,
}

impl Default for PdfDocument {
    fn default() -> Self {
        Self::new()
    }
}

impl PdfDocument {
    /// Пустой документ без страниц.
    pub fn new() -> Self {
        PdfDocument {
            pages: Vec::new(),
            fonts: FontBook::new(),
        }
    }

    /// Добавить страницу заданного размера в точках (1/72 дюйма). A4 —
    /// это 595.28 на 841.89; платформа для того же A4 пишет 595.32 на
    /// 841.92, округляя по-своему.
    ///
    /// # Errors
    ///
    /// Размер не конечен или не положителен.
    pub fn add_page(&mut self, width: f64, height: f64) -> RtResult<PageId> {
        let width = finite("ширина страницы", width)?;
        let height = finite("высота страницы", height)?;
        if width <= 0.0 || height <= 0.0 {
            return Err(RtError::Pdf(format!(
                "размер страницы должен быть положительным, получено {} на {}",
                fmt_real(width),
                fmt_real(height)
            )));
        }
        self.pages.push(Page {
            width,
            height,
            content: Vec::new(),
        });
        Ok(PageId(self.pages.len() - 1))
    }

    /// Число страниц.
    pub fn page_count(&self) -> usize {
        self.pages.len()
    }

    fn page_mut(&mut self, page: PageId) -> RtResult<&mut Page> {
        self.pages.get_mut(page.0).ok_or_else(|| {
            RtError::Pdf(format!("страницы с номером {} в документе нет", page.0 + 1))
        })
    }

    /// Толщина линии в точках для последующих обводок.
    ///
    /// # Errors
    ///
    /// Толщина не конечна или отрицательна.
    pub fn set_line_width(&mut self, page: PageId, width: f64) -> RtResult<()> {
        let width = finite("толщина линии", width)?;
        if width < 0.0 {
            return Err(RtError::Pdf(format!(
                "толщина линии не может быть отрицательной, получено {}",
                fmt_real(width)
            )));
        }
        let page = self.page_mut(page)?;
        push_op(&mut page.content, &[fmt_real(width)], "w");
        Ok(())
    }

    /// Цвет обводки в `/DeviceRGB`, составляющие от 0 до 1.
    ///
    /// # Errors
    ///
    /// Составляющая не конечна или вне диапазона 0..1.
    pub fn set_stroke_color(&mut self, page: PageId, r: f64, g: f64, b: f64) -> RtResult<()> {
        let rgb = [
            component("красная составляющая цвета обводки", r)?,
            component("зелёная составляющая цвета обводки", g)?,
            component("синяя составляющая цвета обводки", b)?,
        ];
        let page = self.page_mut(page)?;
        push_op(&mut page.content, &rgb.map(fmt_real), "RG");
        Ok(())
    }

    /// Цвет заливки в `/DeviceRGB`, составляющие от 0 до 1.
    ///
    /// # Errors
    ///
    /// Составляющая не конечна или вне диапазона 0..1.
    pub fn set_fill_color(&mut self, page: PageId, r: f64, g: f64, b: f64) -> RtResult<()> {
        let rgb = [
            component("красная составляющая цвета заливки", r)?,
            component("зелёная составляющая цвета заливки", g)?,
            component("синяя составляющая цвета заливки", b)?,
        ];
        let page = self.page_mut(page)?;
        push_op(&mut page.content, &rgb.map(fmt_real), "rg");
        Ok(())
    }

    /// Отрезок из точки в точку текущими толщиной и цветом обводки.
    ///
    /// Начало координат PDF — в ЛЕВОМ НИЖНЕМ углу страницы, ось Y растёт
    /// вверх; пересчёт из «сверху вниз» — дело вызывающего.
    ///
    /// # Errors
    ///
    /// Координата не конечна или страницы нет в документе.
    pub fn line(&mut self, page: PageId, x1: f64, y1: f64, x2: f64, y2: f64) -> RtResult<()> {
        let coords = [
            finite("X начала отрезка", x1)?,
            finite("Y начала отрезка", y1)?,
            finite("X конца отрезка", x2)?,
            finite("Y конца отрезка", y2)?,
        ];
        let page = self.page_mut(page)?;
        push_op(
            &mut page.content,
            &[fmt_real(coords[0]), fmt_real(coords[1])],
            "m",
        );
        push_op(
            &mut page.content,
            &[fmt_real(coords[2]), fmt_real(coords[3])],
            "l",
        );
        push_op(&mut page.content, &[], "S");
        Ok(())
    }

    /// Прямоугольник по левому нижнему углу, ширине и высоте. Ширина и
    /// высота могут быть отрицательными — оператор `re` это допускает.
    ///
    /// # Errors
    ///
    /// Координата или размер не конечны, либо страницы нет в документе.
    pub fn rect(
        &mut self,
        page: PageId,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        mode: PaintMode,
    ) -> RtResult<()> {
        let box_ = [
            finite("X прямоугольника", x)?,
            finite("Y прямоугольника", y)?,
            finite("ширина прямоугольника", width)?,
            finite("высота прямоугольника", height)?,
        ];
        let page = self.page_mut(page)?;
        push_op(&mut page.content, &box_.map(fmt_real), "re");
        push_op(&mut page.content, &[], mode.operator());
        Ok(())
    }

    /// Текст с левым краем в точке `(x, y)`: `y` — базовая линия, а не
    /// верх строки.
    ///
    /// Строка кодируется по правилам, описанным в обзоре модуля: знаки
    /// раскладываются по ресурсам шрифта, и на каждый ресурс выводится
    /// свой `Tj`. Кегль — в точках.
    ///
    /// # Errors
    ///
    /// Координата не конечна, кегль не положителен, страницы нет в
    /// документе или в строке есть управляющий знак: перевод строки и
    /// табуляцию этот слой не раскладывает — где начинается новая строка,
    /// решает вызывающий.
    pub fn text(
        &mut self,
        page: PageId,
        x: f64,
        y: f64,
        font: PdfFont,
        size: f64,
        text: &str,
    ) -> RtResult<()> {
        let x = finite("X текста", x)?;
        let y = finite("Y текста", y)?;
        let size = finite("кегль", size)?;
        if size <= 0.0 {
            return Err(RtError::Pdf(format!(
                "кегль должен быть положительным, получено {}",
                fmt_real(size)
            )));
        }
        if page.0 >= self.pages.len() {
            return Err(RtError::Pdf(format!(
                "страницы с номером {} в документе нет",
                page.0 + 1
            )));
        }
        if text.is_empty() {
            return Ok(());
        }

        // Строка проверяется ЦЕЛИКОМ до того, как знакам начнут выдаваться
        // коды: иначе отказ на середине оставил бы в книге шрифтов коды
        // для знаков, которые так и не нарисованы.
        for ch in text.chars() {
            if ch.is_control() {
                return Err(RtError::Pdf(format!(
                    "управляющий знак U+{:04X} в тексте PDF: разбивать текст на строки должен вызывающий",
                    ch as u32
                )));
            }
        }

        // Отрезки строки по ресурсам шрифта: подряд идущие знаки одного
        // ресурса выводятся одним `Tj`.
        let mut runs: Vec<(usize, Vec<u8>)> = Vec::new();
        for ch in text.chars() {
            let (plane, code) = self.fonts.encode(font, ch);
            match runs.last_mut() {
                Some((last, bytes)) if *last == plane => bytes.push(code),
                _ => runs.push((plane, vec![code])),
            }
        }

        let content = &mut self.pages[page.0].content;
        content.extend_from_slice(b"BT\n");
        let mut current: Option<usize> = None;
        for (index, (plane, bytes)) in runs.iter().enumerate() {
            if current != Some(*plane) {
                write_name(content, &FontBook::resource_name(*plane));
                content.push(b' ');
                content.extend_from_slice(fmt_real(size).as_bytes());
                content.extend_from_slice(b" Tf\n");
                current = Some(*plane);
            }
            if index == 0 {
                // Позиционирование ставится один раз, перед первым
                // отрезком: `BT` сбросил текстовую матрицу, а дальше она
                // двигается сама, по объявленным ширинам.
                push_op(content, &[fmt_real(x), fmt_real(y)], "Td");
            }
            write_str(content, bytes);
            content.extend_from_slice(b" Tj\n");
        }
        content.extend_from_slice(b"ET\n");
        Ok(())
    }

    /// Собрать файл целиком.
    ///
    /// Нумерация объектов выводится из числа страниц и ресурсов шрифта, а
    /// не раздаётся по ходу дела: 1 — каталог, 2 — узел страниц, затем на
    /// каждую страницу пара «страница, контент-поток», затем на каждый
    /// ресурс шрифта пара «шрифт, `/ToUnicode`». Поэтому одинаковый вход
    /// даёт одинаковые байты.
    ///
    /// # Errors
    ///
    /// В документе нет ни одной страницы: `/Pages` с пустым `/Kids` —
    /// документ, который просмотрщики отвергают.
    pub fn write(&self) -> RtResult<Vec<u8>> {
        if self.pages.is_empty() {
            return Err(RtError::Pdf(
                "в документе PDF нет ни одной страницы".to_string(),
            ));
        }

        let pages = self.pages.len();
        let page_obj = |i: usize| (3 + 2 * i) as u32;
        let content_obj = |i: usize| (4 + 2 * i) as u32;
        let font_obj = |j: usize| (3 + 2 * pages + 2 * j) as u32;
        let to_unicode_obj = |j: usize| (4 + 2 * pages + 2 * j) as u32;

        // Словарь `/Font` одинаков для всех страниц: ресурсов немного, а
        // раздельный учёт «какой план на какой странице» ничего не даёт.
        let font_entries: Vec<(String, PdfValue)> = (0..self.fonts.planes.len())
            .map(|j| (FontBook::resource_name(j), PdfValue::Ref(font_obj(j))))
            .collect();

        let mut objects: Vec<PdfValue> = Vec::with_capacity(2 + 2 * pages);

        objects.push(PdfValue::Dict(vec![
            ("Type".to_string(), PdfValue::Name("Catalog".to_string())),
            ("Pages".to_string(), PdfValue::Ref(2)),
        ]));
        objects.push(PdfValue::Dict(vec![
            ("Type".to_string(), PdfValue::Name("Pages".to_string())),
            (
                "Kids".to_string(),
                PdfValue::Array((0..pages).map(|i| PdfValue::Ref(page_obj(i))).collect()),
            ),
            ("Count".to_string(), PdfValue::Integer(pages as i64)),
        ]));

        for (i, page) in self.pages.iter().enumerate() {
            let mut resources = vec![(
                "ProcSet".to_string(),
                PdfValue::Array(vec![
                    PdfValue::Name("PDF".to_string()),
                    PdfValue::Name("Text".to_string()),
                ]),
            )];
            if !font_entries.is_empty() {
                resources.push(("Font".to_string(), PdfValue::Dict(font_entries.clone())));
            }
            objects.push(PdfValue::Dict(vec![
                ("Type".to_string(), PdfValue::Name("Page".to_string())),
                ("Parent".to_string(), PdfValue::Ref(2)),
                (
                    "MediaBox".to_string(),
                    PdfValue::Array(vec![
                        PdfValue::Integer(0),
                        PdfValue::Integer(0),
                        PdfValue::Real(page.width),
                        PdfValue::Real(page.height),
                    ]),
                ),
                ("Contents".to_string(), PdfValue::Ref(content_obj(i))),
                ("Resources".to_string(), PdfValue::Dict(resources)),
            ]));
            objects.push(PdfValue::Stream {
                dict: vec![(
                    "Filter".to_string(),
                    PdfValue::Name("FlateDecode".to_string()),
                )],
                data: zlib_compress(&page.content),
            });
        }

        for (j, plane) in self.fonts.planes.iter().enumerate() {
            objects.push(font_dict(plane, to_unicode_obj(j)));
            objects.push(PdfValue::Stream {
                dict: vec![(
                    "Filter".to_string(),
                    PdfValue::Name("FlateDecode".to_string()),
                )],
                data: zlib_compress(&to_unicode_cmap(plane)),
            });
        }

        // Заголовок: версия и обязательная по спецификации строка-маркер
        // из байтов старше 0x7F — по ней инструменты, работающие с файлом
        // как с текстом, понимают, что он двоичный.
        let mut out: Vec<u8> = Vec::new();
        out.extend_from_slice(b"%PDF-1.4\n");
        out.extend_from_slice(&[b'%', 0xE2, 0xE3, 0xCF, 0xD3, b'\n']);

        let mut offsets: Vec<usize> = Vec::with_capacity(objects.len());
        for (index, object) in objects.iter().enumerate() {
            offsets.push(out.len());
            out.extend_from_slice(format!("{} 0 obj\n", index + 1).as_bytes());
            write_value(&mut out, object);
            out.extend_from_slice(b"\nendobj\n");
        }

        // Классическая таблица xref: нулевая запись свободная, дальше по
        // одной на объект, каждая РОВНО в двадцать байтов.
        let xref_offset = out.len();
        out.extend_from_slice(format!("xref\n0 {}\n", objects.len() + 1).as_bytes());
        out.extend_from_slice(b"0000000000 65535 f \n");
        for offset in &offsets {
            out.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
        }
        out.extend_from_slice(b"trailer\n");
        write_value(
            &mut out,
            &PdfValue::Dict(vec![
                (
                    "Size".to_string(),
                    PdfValue::Integer(objects.len() as i64 + 1),
                ),
                ("Root".to_string(), PdfValue::Ref(1)),
            ]),
        );
        out.extend_from_slice(format!("\nstartxref\n{xref_offset}\n%%EOF\n").as_bytes());
        Ok(out)
    }
}

/// Дописать оператор контент-потока: операнды через пробел, затем сам
/// оператор и перевод строки.
fn push_op(content: &mut Vec<u8>, operands: &[String], operator: &str) {
    for operand in operands {
        content.extend_from_slice(operand.as_bytes());
        content.push(b' ');
    }
    content.extend_from_slice(operator.as_bytes());
    content.push(b'\n');
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inflate::inflate;
    use std::path::PathBuf;
    use std::process::Command;

    /// Разобрать поток по имени объекта: вернуть распакованные байты.
    fn stream_data(pdf: &[u8], object: u32) -> Vec<u8> {
        let head = format!("{object} 0 obj");
        let start = find(pdf, head.as_bytes()).expect("объект должен быть в файле");
        let marker = find(&pdf[start..], b"stream\n").expect("у объекта должен быть поток") + start;
        let data_start = marker + b"stream\n".len();
        let data_end =
            find(&pdf[data_start..], b"\nendstream").expect("поток должен кончаться") + data_start;
        let zlib = &pdf[data_start..data_end];
        // Снять оболочку RFC 1950: два байта заголовка и четыре Adler-32.
        assert_eq!(zlib[0], 0x78, "CMF");
        inflate(&zlib[2..zlib.len() - 4], 1 << 20).expect("поток должен распаковываться")
    }

    fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
        haystack
            .windows(needle.len())
            .position(|window| window == needle)
    }

    /// Простой документ на все примитивы сразу.
    fn sample() -> PdfDocument {
        let mut doc = PdfDocument::new();
        let page = doc.add_page(595.28, 841.89).unwrap();
        doc.set_line_width(page, 0.75).unwrap();
        doc.set_stroke_color(page, 0.0, 0.0, 0.0).unwrap();
        doc.set_fill_color(page, 0.9, 0.9, 0.85).unwrap();
        doc.rect(page, 40.0, 700.0, 200.0, 60.0, PaintMode::FillAndStroke)
            .unwrap();
        doc.line(page, 40.0, 690.0, 400.0, 690.0).unwrap();
        doc.set_fill_color(page, 0.0, 0.0, 0.0).unwrap();
        doc.text(
            page,
            50.0,
            730.0,
            PdfFont::CourierBold,
            14.0,
            "Накладная № 7",
        )
        .unwrap();
        doc.text(page, 50.0, 710.0, PdfFont::Courier, 10.0, "Гвоздь 123.45")
            .unwrap();
        doc.text(
            page,
            50.0,
            660.0,
            PdfFont::Courier,
            10.0,
            "Latin ASCII 67890",
        )
        .unwrap();
        doc
    }

    /// Разбор таблицы `xref` СВОЕГО вывода: смещения должны указывать
    /// ровно на заголовки объектов, а трейлер — разрешаться в каталог.
    #[test]
    fn pdf_xref_points_at_every_object() {
        let pdf = sample().write().unwrap();
        assert!(pdf.starts_with(b"%PDF-1.4\n"), "заголовок");

        let startxref = find(&pdf, b"startxref\n").expect("должен быть startxref");
        let tail = String::from_utf8_lossy(&pdf[startxref + b"startxref\n".len()..]);
        let xref_offset: usize = tail.lines().next().unwrap().trim().parse().unwrap();
        assert_eq!(&pdf[xref_offset..xref_offset + 4], b"xref");

        let header_end = find(&pdf[xref_offset..], b"\n").unwrap() + xref_offset + 1;
        let count_line = String::from_utf8_lossy(&pdf[header_end..]);
        let count: usize = count_line
            .lines()
            .next()
            .unwrap()
            .split_whitespace()
            .nth(1)
            .unwrap()
            .parse()
            .unwrap();
        let table = header_end + count_line.lines().next().unwrap().len() + 1;

        // Записи ровно по двадцать байтов, первая — свободная.
        assert_eq!(&pdf[table..table + 20], b"0000000000 65535 f \n");
        for object in 1..count {
            let entry = &pdf[table + 20 * object..table + 20 * (object + 1)];
            assert_eq!(entry.len(), 20, "запись xref обязана быть 20 байтов");
            assert_eq!(entry[19], b'\n');
            let offset: usize = String::from_utf8_lossy(&entry[..10]).parse().unwrap();
            let expected = format!("{object} 0 obj");
            assert_eq!(
                &pdf[offset..offset + expected.len()],
                expected.as_bytes(),
                "смещение объекта {object} должно указывать на его заголовок"
            );
        }

        // Трейлер разрешается в каталог, каталог — в узел страниц.
        let trailer = find(&pdf, b"trailer\n").expect("должен быть trailer");
        let trailer_text = String::from_utf8_lossy(&pdf[trailer..startxref]);
        assert!(trailer_text.contains("/Root 1 0 R"), "{trailer_text}");
        assert!(
            trailer_text.contains(&format!("/Size {}", count)),
            "{trailer_text}"
        );
        let catalog = find(&pdf, b"1 0 obj").unwrap();
        let catalog_text = String::from_utf8_lossy(&pdf[catalog..catalog + 80]);
        assert!(catalog_text.contains("/Type /Catalog"), "{catalog_text}");
        assert!(catalog_text.contains("/Pages 2 0 R"), "{catalog_text}");
    }

    /// `/Length` каждого потока должен совпадать с числом байтов между
    /// `stream` и `endstream` — иначе просмотрщик читает мимо.
    #[test]
    fn pdf_stream_lengths_match_the_bytes() {
        let pdf = sample().write().unwrap();
        let mut checked = 0;
        let mut at = 0;
        while let Some(found) = find(&pdf[at..], b"stream\n") {
            let marker = at + found;
            // `endstream` тоже содержит «stream», пропускаем его.
            if marker >= 3 && &pdf[marker - 3..marker] == b"end" {
                at = marker + 1;
                continue;
            }
            let data_start = marker + b"stream\n".len();
            let data_end = find(&pdf[data_start..], b"\nendstream").unwrap() + data_start;

            let dict_start = pdf[..marker].windows(2).rposition(|w| w == b"<<").unwrap();
            let dict = String::from_utf8_lossy(&pdf[dict_start..marker]);
            let length: usize = dict
                .split("/Length")
                .nth(1)
                .unwrap()
                .split_whitespace()
                .next()
                .unwrap()
                .parse()
                .unwrap();
            assert_eq!(length, data_end - data_start, "/Length потока при {dict}");
            checked += 1;
            at = data_end;
        }
        assert!(
            checked >= 2,
            "потоков должно быть хотя бы два, а не {checked}"
        );
    }

    /// Adler-32 — сверка с эталоном. Происхождение (CPython 3.14,
    /// zlib 1.3.1):
    ///
    /// ```python
    /// import zlib
    /// print(zlib.adler32(b""), zlib.adler32(b"a"),
    ///       zlib.adler32("Накладная № 7".encode()),
    ///       zlib.adler32(bytes(range(256)) * 40))
    /// # 1 6422626 3674410983 4101369118
    /// ```
    ///
    /// Последний случай длиной 10240 байт нужен ради границы блока: суммы
    /// берутся по модулю раз в 5552 байта, и ошибка в этой арифметике
    /// видна только на входе длиннее одного блока.
    #[test]
    fn pdf_adler32_matches_zlib() {
        assert_eq!(adler32(b""), 1);
        assert_eq!(adler32(b"a"), 6_422_626);
        assert_eq!(adler32("Накладная № 7".as_bytes()), 3_674_410_983);
        let long: Vec<u8> = (0..=255u8).cycle().take(256 * 40).collect();
        assert_eq!(adler32(&long), 4_101_369_118);
    }

    /// Оболочка RFC 1950 вокруг нашего deflate: проверочная пара сходится,
    /// хвост — Adler-32, а сами данные распаковываются нашим же inflate.
    #[test]
    fn pdf_flate_streams_round_trip() {
        let text = "Накладная № 7; Гвоздь строительный; 123.45; Latin ASCII\n".repeat(20);
        let wrapped = zlib_compress(text.as_bytes());
        let check = (u16::from(wrapped[0]) << 8) | u16::from(wrapped[1]);
        assert_eq!(check % 31, 0, "проверочная пара CMF/FLG");
        assert_eq!(wrapped[0], 0x78);
        let tail = u32::from_be_bytes(wrapped[wrapped.len() - 4..].try_into().unwrap());
        assert_eq!(tail, adler32(text.as_bytes()));
        let back = inflate(&wrapped[2..wrapped.len() - 4], 1 << 20).unwrap();
        assert_eq!(back, text.as_bytes());

        // То же самое, но на настоящем контент-потоке документа: объект 4
        // — поток первой страницы.
        let pdf = sample().write().unwrap();
        let content = stream_data(&pdf, 4);
        assert!(String::from_utf8_lossy(&content).contains("BT"));
    }

    /// Примитивы должны доходить до потока операторами, а не «примерно».
    #[test]
    fn pdf_primitives_reach_the_content_stream() {
        let pdf = sample().write().unwrap();
        let content = String::from_utf8(stream_data(&pdf, 4)).unwrap();
        assert!(content.contains("0.75 w\n"), "{content}");
        assert!(content.contains("0 0 0 RG\n"), "{content}");
        assert!(content.contains("0.9 0.9 0.85 rg\n"), "{content}");
        assert!(content.contains("40 700 200 60 re\nB\n"), "{content}");
        assert!(content.contains("40 690 m\n400 690 l\nS\n"), "{content}");
        assert!(content.contains("BT\n"), "{content}");
        assert!(content.contains(" 14 Tf\n"), "{content}");
        assert!(content.contains("50 730 Td\n"), "{content}");
        assert!(content.contains("ET\n"), "{content}");
    }

    /// Кодировка: кириллица получает коды из 128.., ASCII остаётся собой,
    /// и каждый выданный код описан в `/ToUnicode`.
    #[test]
    fn pdf_cyrillic_codes_are_declared_in_tounicode() {
        let mut doc = PdfDocument::new();
        let page = doc.add_page(200.0, 200.0).unwrap();
        doc.text(page, 10.0, 100.0, PdfFont::Courier, 12.0, "Аz")
            .unwrap();
        let pdf = doc.write().unwrap();
        let content = String::from_utf8(stream_data(&pdf, 4)).unwrap();
        // «А» — первый неанглийский знак, значит код 128 = \200
        // восьмеричным; «z» остаётся собой.
        assert!(content.contains("(\\200z) Tj"), "{content}");

        let font = String::from_utf8_lossy(&pdf[find(&pdf, b"5 0 obj").unwrap()..]).to_string();
        assert!(font.contains("/BaseFont /Courier"), "{font}");
        // Имя из AGL, а не `uni0410`: по `uni0410` подстановочный шрифт
        // глифа не находит и кириллица не рисуется — см. обзор модуля.
        assert!(font.contains("/Differences [ 128 /afii10017 ]"), "{font}");
        assert!(font.contains("/BaseEncoding /WinAnsiEncoding"), "{font}");

        let cmap = String::from_utf8(stream_data(&pdf, 6)).unwrap();
        assert!(cmap.contains("<80> <0410>"), "{cmap}");
        assert!(cmap.contains("<7A> <007A>"), "{cmap}");
    }

    /// Когда 128 мест кончаются, заводится второй ресурс шрифта, и строка
    /// режется на отрезки по ресурсам.
    #[test]
    fn pdf_encoding_overflows_into_a_second_font_resource() {
        let mut doc = PdfDocument::new();
        let page = doc.add_page(200.0, 200.0).unwrap();
        // 200 различных неанглийских знаков подряд — больше, чем 128 мест
        // одного плана.
        let many: String = (0..200u32)
            .map(|i| char::from_u32(0x0400 + i).unwrap())
            .collect();
        doc.text(page, 10.0, 100.0, PdfFont::Courier, 8.0, &many)
            .unwrap();
        assert_eq!(doc.fonts.planes.len(), 2);
        let pdf = doc.write().unwrap();
        let content = String::from_utf8(stream_data(&pdf, 4)).unwrap();
        assert!(content.contains("/F1 8 Tf"), "{content}");
        assert!(content.contains("/F2 8 Tf"), "{content}");
        // Позиционирование ставится один раз, до первого отрезка.
        assert_eq!(content.matches(" Td\n").count(), 1, "{content}");
    }

    /// Ошибки — это `RtError`, а не паника: испорченный вход API назван
    /// словами.
    #[test]
    fn pdf_bad_input_is_an_error_not_a_panic() {
        let mut doc = PdfDocument::new();
        assert!(doc.write().is_err(), "документ без страниц");
        assert!(doc.add_page(f64::NAN, 100.0).is_err());
        assert!(doc.add_page(0.0, 100.0).is_err());
        let page = doc.add_page(100.0, 100.0).unwrap();
        assert!(doc.line(page, f64::INFINITY, 0.0, 1.0, 1.0).is_err());
        assert!(doc.set_fill_color(page, 1.5, 0.0, 0.0).is_err());
        assert!(doc.set_line_width(page, -1.0).is_err());
        assert!(doc
            .text(page, 0.0, 0.0, PdfFont::Courier, 0.0, "а")
            .is_err());
        assert!(
            doc.text(page, 0.0, 0.0, PdfFont::Courier, 10.0, "две\nстроки")
                .is_err(),
            "перевод строки должен раскладывать вызывающий"
        );
        // Отказ на середине строки не должен оставлять следа: коды знакам
        // «две» не выданы, книга шрифтов пуста, ресурсов шрифта в файле
        // нет.
        assert!(doc.fonts.planes.is_empty(), "отказ не должен ничего копить");
        // Номер страницы из чужого документа — тоже ошибка, а не паника.
        assert!(doc
            .rect(PageId(7), 0.0, 0.0, 1.0, 1.0, PaintMode::Fill)
            .is_err());
        assert!(doc
            .text(PageId(7), 0.0, 0.0, PdfFont::Courier, 10.0, "а")
            .is_err());
    }

    /// Вывод детерминирован: тот же вход — те же байты.
    #[test]
    fn pdf_output_is_deterministic() {
        assert_eq!(sample().write().unwrap(), sample().write().unwrap());
    }

    /// Документ без единой строки текста: ресурсов шрифта нет, и словарь
    /// `/Font` в страницу не попадает вовсе — пустой `<< >>` там был бы
    /// мусором. Нумерация объектов при этом обязана остаться связной.
    #[test]
    fn pdf_document_without_text_has_no_font_resources() {
        let mut doc = PdfDocument::new();
        let page = doc.add_page(100.0, 100.0).unwrap();
        doc.rect(page, 10.0, 10.0, 80.0, 80.0, PaintMode::Stroke)
            .unwrap();
        let pdf = doc.write().unwrap();
        let text = String::from_utf8_lossy(&pdf).to_string();
        assert!(!text.contains("/Font"), "{text}");
        assert!(text.contains("/Size 5"), "каталог, узел, страница, поток");
        assert!(find(&pdf, b"4 0 obj").is_some());
        assert!(find(&pdf, b"5 0 obj").is_none());
        let content = String::from_utf8(stream_data(&pdf, 4)).unwrap();
        assert_eq!(content, "10 10 80 80 re\nS\n");
    }

    /// Таблица имён должна быть отсортирована — по ней идёт двоичный
    /// поиск, и разъехавшийся порядок молча давал бы `uniXXXX` вместо
    /// собственного имени, то есть нерисуемую кириллицу.
    #[test]
    fn pdf_agl_table_is_sorted_and_covers_cyrillic() {
        assert!(
            AGL_NAMES.windows(2).all(|pair| pair[0].0 < pair[1].0),
            "таблица AGL обязана быть строго возрастающей"
        );
        assert_eq!(glyph_name('А'), "afii10017");
        assert_eq!(glyph_name('я'), "afii10097");
        assert_eq!(glyph_name('Ё'), "afii10023");
        assert_eq!(glyph_name('№'), "afii61352");
        assert_eq!(glyph_name('«'), "guillemotleft");
        // Знака вне списка Adobe не существует — остаётся общая форма.
        assert_eq!(glyph_name('\u{4E2D}'), "uni4E2D");
        assert_eq!(glyph_name('\u{1F600}'), "u1F600");
    }

    /// Чёрные пиксели в однобитной картинке PBM (`P4`): заголовок,
    /// размеры, дальше упакованные строки, единица — чёрный.
    fn ink(pbm: &[u8]) -> usize {
        let mut parts = pbm.splitn(3, |&b| b == b'\n');
        assert_eq!(parts.next(), Some(&b"P4"[..]), "ожидался PBM P4");
        let size = String::from_utf8_lossy(parts.next().unwrap()).to_string();
        let mut wh = size.split_whitespace();
        let width: usize = wh.next().unwrap().parse().unwrap();
        let height: usize = wh.next().unwrap().parse().unwrap();
        let data = parts.next().unwrap();
        assert_eq!(data.len(), width.div_ceil(8) * height, "размер растра");
        data.iter().map(|b| b.count_ones() as usize).sum()
    }

    /// ВНЕШНЯЯ проверка рисования: страница, на которой нет ничего кроме
    /// КИРИЛЛИЦЫ, обязана содержать краску.
    ///
    /// Это регрессия на самую дорогую ошибку этого модуля: с именами
    /// глифов `uniXXXX` текст прекрасно ИЗВЛЕКАЛСЯ, но подстановочный
    /// шрифт не находил по ним глифов, и страница выходила пустой. Тест с
    /// `pdftotext` такую поломку не ловит — он смотрит в `/ToUnicode`,
    /// а не в растр.
    #[test]
    fn pdf_cyrillic_is_actually_painted() {
        let dir = std::env::temp_dir().join(format!("open-bsl-pdf-ink-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();

        let render = |name: &str, text: &str| -> Option<usize> {
            let mut doc = PdfDocument::new();
            let page = doc.add_page(300.0, 100.0).unwrap();
            if !text.is_empty() {
                doc.text(page, 20.0, 40.0, PdfFont::Courier, 18.0, text)
                    .unwrap();
            }
            let pdf = dir.join(format!("{name}.pdf"));
            std::fs::write(&pdf, doc.write().unwrap()).unwrap();
            let run = Command::new("pdftoppm")
                .args(["-mono", "-r", "72", "-singlefile"])
                .arg(&pdf)
                .arg(dir.join(name))
                .output()
                .ok()?;
            assert!(
                run.status.success(),
                "pdftoppm: {}",
                String::from_utf8_lossy(&run.stderr)
            );
            let pbm = std::fs::read(dir.join(format!("{name}.pbm"))).unwrap();
            Some(ink(&pbm))
        };

        let Some(empty) = render("empty", "") else {
            println!("ПРОПУЩЕНО: pdftoppm не найден в PATH — рисование не проверено");
            return;
        };
        assert_eq!(empty, 0, "пустая страница обязана быть без краски");

        let cyrillic = render("cyrillic", "Гвоздь").unwrap();
        println!("pdftoppm ИСПОЛНЕН: чёрных пикселей на кириллической странице {cyrillic}");
        assert!(
            cyrillic > 100,
            "кириллица не нарисована: краски {cyrillic} — проверьте имена глифов в /Differences"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    /// ВНЕШНЯЯ проверка: `pdftotext` из poppler должен вынуть из нашего
    /// файла ровно тот текст, что в него положен, — и кириллицу, и
    /// латиницу, и цифры. Это и есть настоящий критерий правильности
    /// `/ToUnicode`: разбор своего вывода своими же руками ничего не
    /// доказывает.
    ///
    /// Без `pdftotext` в `PATH` тест печатает пометку и проходит —
    /// прецедент тот же, что у `table_compare2` в `bsl-cli`.
    #[test]
    fn pdf_pdftotext_extracts_the_written_text() {
        let dir = std::env::temp_dir().join(format!("open-bsl-pdf-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path: PathBuf = dir.join("extract.pdf");
        let txt: PathBuf = dir.join("extract.txt");

        let mut doc = PdfDocument::new();
        let page = doc.add_page(595.28, 841.89).unwrap();
        let lines = [
            "Накладная № 7 от 14.08.2026",
            "Гвоздь строительный 123.45",
            "Latin ASCII text 67890",
            "ЁЙЦУКЕНГШЩЗХЪ ёйцукенгшщзхъ",
        ];
        for (i, line) in lines.iter().enumerate() {
            doc.text(
                page,
                40.0,
                800.0 - 20.0 * i as f64,
                PdfFont::Courier,
                11.0,
                line,
            )
            .unwrap();
        }
        std::fs::write(&path, doc.write().unwrap()).unwrap();

        let run = Command::new("pdftotext")
            .arg("-enc")
            .arg("UTF-8")
            .arg(&path)
            .arg(&txt)
            .output();
        let run = match run {
            Ok(run) => run,
            Err(err) => {
                println!("ПРОПУЩЕНО: pdftotext не найден в PATH ({err}) — извлечение текста не проверено");
                return;
            }
        };
        assert!(
            run.status.success(),
            "pdftotext вернул {:?}: {}",
            run.status.code(),
            String::from_utf8_lossy(&run.stderr)
        );
        let extracted = std::fs::read_to_string(&txt).unwrap();
        println!("pdftotext ИСПОЛНЕН, извлечено:\n{extracted}");
        for line in lines {
            assert!(
                extracted.contains(line),
                "pdftotext не нашёл строку «{line}» в:\n{extracted}"
            );
        }

        // Заодно число страниц по данным самого poppler: разбираем строку,
        // а не сравниваем с её точной раскладкой по колонкам.
        if let Ok(info) = Command::new("pdfinfo").arg(&path).output() {
            let info = String::from_utf8_lossy(&info.stdout).to_string();
            let pages = info
                .lines()
                .find_map(|line| line.strip_prefix("Pages:"))
                .map(str::trim);
            assert_eq!(pages, Some("1"), "{info}");
        }

        std::fs::remove_file(&path).ok();
        std::fs::remove_file(&txt).ok();
        std::fs::remove_dir(&dir).ok();
    }
}
