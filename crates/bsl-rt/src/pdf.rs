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

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use crate::deflate::deflate;
use crate::{BslObject, BslValue, RtError, RtResult};

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
    /// `true` либо `false`. Писатель их не порождает; они появились ради
    /// ЧТЕНИЯ — в чужих файлах логические значения встречаются сплошь.
    Bool(bool),
    /// `null`. Им же представляется объект, которого нет в таблице
    /// перекрёстных ссылок (раздел 7.3.10).
    Null,
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
        PdfValue::Bool(true) => out.extend_from_slice(b"true"),
        PdfValue::Bool(false) => out.extend_from_slice(b"false"),
        PdfValue::Null => out.extend_from_slice(b"null"),
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

/// Страница: размер, накопленный контент-поток и глубина сохранённого
/// графического состояния.
struct Page {
    width: f64,
    height: f64,
    content: Vec<u8>,
    /// Сколько `q` выведено без парного `Q`. Незакрытая скобка — это битый
    /// файл, поэтому глубина считается и проверяется при записи.
    clip_depth: usize,
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
            clip_depth: 0,
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

    /// Штриховка последующих обводок: длины отрезков и пробелов в точках и
    /// сдвиг начала узора. Пустой узор — сплошная линия.
    ///
    /// Платформа рисует точечную границу ячейки как `[1] 0 d` при толщине
    /// 0.75 (измерено на 8.3.27, `tests/conformance/pdf/probe-line.pdf`).
    ///
    /// # Errors
    ///
    /// Длина не конечна или отрицательна, сдвиг не конечен либо страницы
    /// нет в документе.
    pub fn set_dash(&mut self, page: PageId, pattern: &[f64], phase: f64) -> RtResult<()> {
        let phase = finite("сдвиг штриховки", phase)?;
        let mut parts = Vec::with_capacity(pattern.len());
        for length in pattern {
            let length = finite("длина штриха", *length)?;
            if length < 0.0 {
                return Err(RtError::Pdf(format!(
                    "длина штриха не может быть отрицательной, получено {}",
                    fmt_real(length)
                )));
            }
            parts.push(fmt_real(length));
        }
        let page = self.page_mut(page)?;
        push_op(
            &mut page.content,
            &[format!("[{}]", parts.join(" ")), fmt_real(phase)],
            "d",
        );
        Ok(())
    }

    /// Сохранить графическое состояние и обрезать вывод прямоугольником —
    /// `q x y w h re W n`. До парного [`PdfDocument::pop_clip`] всё, что
    /// выходит за прямоугольник, на странице не появится.
    ///
    /// Отдельный примитив здесь потому, что отсечение — это решение
    /// РАСКЛАДКИ (текст не должен вылезать из ячейки), а не формата, и
    /// принимает его вызывающий; платформа обставляет отсечением каждую
    /// ячейку (измерено).
    ///
    /// # Errors
    ///
    /// Координата или размер не конечны либо страницы нет в документе.
    pub fn push_clip(
        &mut self,
        page: PageId,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    ) -> RtResult<()> {
        let box_ = [
            finite("X отсечения", x)?,
            finite("Y отсечения", y)?,
            finite("ширина отсечения", width)?,
            finite("высота отсечения", height)?,
        ];
        let page = self.page_mut(page)?;
        push_op(&mut page.content, &[], "q");
        push_op(&mut page.content, &box_.map(fmt_real), "re");
        push_op(&mut page.content, &[], "W");
        push_op(&mut page.content, &[], "n");
        page.clip_depth += 1;
        Ok(())
    }

    /// Вернуть графическое состояние, сохранённое [`PdfDocument::push_clip`].
    ///
    /// # Errors
    ///
    /// Парного `push_clip` не было либо страницы нет в документе.
    pub fn pop_clip(&mut self, page: PageId) -> RtResult<()> {
        let page = self.page_mut(page)?;
        if page.clip_depth == 0 {
            return Err(RtError::Pdf(
                "снятие отсечения без парного наложения".to_string(),
            ));
        }
        page.clip_depth -= 1;
        push_op(&mut page.content, &[], "Q");
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
    /// В документе нет ни одной страницы (`/Pages` с пустым `/Kids` —
    /// документ, который просмотрщики отвергают) либо на какой-то странице
    /// осталось незакрытое отсечение: `q` без `Q` просмотрщик считает
    /// повреждением потока.
    pub fn write(&self) -> RtResult<Vec<u8>> {
        if self.pages.is_empty() {
            return Err(RtError::Pdf(
                "в документе PDF нет ни одной страницы".to_string(),
            ));
        }
        if let Some((index, page)) = self
            .pages
            .iter()
            .enumerate()
            .find(|(_, page)| page.clip_depth != 0)
        {
            return Err(RtError::Pdf(format!(
                "на странице {} осталось {} незакрытых отсечений",
                index + 1,
                page.clip_depth
            )));
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

// ---------------------------------------------------------------------------
// Чтение контейнера
// ---------------------------------------------------------------------------
//
// Читатель разбирает ровно столько формата, сколько нужно поверхности
// `ДокументPDF`: таблицу перекрёстных ссылок (классическую и потоком),
// объекты, объектные потоки, фильтр `FlateDecode` с предикторами PNG — и
// дерево страниц. Содержимое страниц (текст, графика, шрифты) не
// разбирается вовсе: наружу видны только размер страницы и её поворот,
// потому что больше платформа про страницу и не отдаёт (измерено,
// `tests/conformance/measure/measure-pdf-read.bsl`).
//
// # Что измерено на 8.3.27 и здесь воспроизведено
//
// * размеры страницы отдаются в МИЛЛИМЕТРАХ целым числом: `/MediaBox
//   [0 0 100 200]` даёт 35 на 71 (100 пунктов — 35,28 мм, 200 — 70,56),
//   `[0 0 100.62 200]` — 35, `[0 0 100.64 200]` — 36, то есть округление
//   к ближайшему;
// * начало рамки не важно: `[10 20 110 220]` даёт те же 35 на 71;
// * видимую область задаёт `/CropBox`, а не `/MediaBox`: страница A4 с
//   `/CropBox [50 60 545.32 781.92]` отдала 175 на 255;
// * `/CropBox` ПЕРЕСЕКАЕТСЯ с `/MediaBox`: `[-100 -100 700 900]` поверх
//   A4 отдал 210 на 297, а не 282 на 353;
// * `/MediaBox` и `/CropBox` НАСЛЕДУЮТСЯ от узлов `/Pages` (страница без
//   собственной рамки под узлом с `/CropBox` отдала 175 на 255), а
//   `/Rotate` НЕ наследуется (страница под узлом с `/Rotate 90` отдала
//   ориентацию 0) — это расхождение с разделом 7.7.3.4 спецификации, где
//   наследуемым объявлен и он;
// * рамки без площади дают НОЛЬ на ноль по обеим сторонам сразу:
//   переставленная по X `[595.32 0 0 841.92]`, нулевая `[0 0 0 0]` и
//   отрицательная `[0 0 -100 -200]` — все три отдали 0 на 0;
// * страница без `/MediaBox` вообще — 216 на 279, то есть US Letter
//   (612 на 792 пункта);
// * `Ориентация` — это ЧИСЛО, значение `/Rotate`, приведённое к кратному
//   прямому углу округлением к ближайшему и затем к остатку от деления на
//   360: измерено 180 -> 180, 270 -> 270, -90 -> 270, 450 -> 90, 44 -> 0,
//   45 -> 90, 46 -> 90, 135 -> 180, 315 -> 0, 350 -> 0 (две последние —
//   ровно те половинки, на которых видно направление округления);
// * поля страницы (`ПолеСлева` и три соседа) приходят из `/TrimBox` —
//   именно его платформа кладёт в свои файлы рядом с одинаковым
//   `/BleedBox` (`probe-empty.pdf`: обе рамки [28.35 28.35 566.97 813.57]
//   при A4, то есть отступ ровно 10 мм, умолчание полей табличного
//   документа). Несимметричное правило разобрано у `margins_of`;
// * пустое дерево страниц (`/Count 0`, пустой `/Kids`) — законный файл с
//   нулём страниц, а трейлер без `/Root` — ошибка;
// * `Страницы` у документа, который ещё ничего не прочитал, — это
//   `Неопределено`, а не ошибка и не пустая коллекция;
// * инкрементальное обновление (цепочка `/Prev`) и файл с `xref`-потоком и
//   объектным потоком читаются наравне с классическим.
//
// # Враждебный вход
//
// Разбор идёт по данным пользователя, поэтому каждый обход, способный
// зациклиться, ограничен явно: цепочка `/Prev` — множеством уже виденных
// смещений, косвенные ссылки — стеком разбираемых номеров, дерево
// страниц — множеством номеров и глубиной, распаковка — пределом на
// размер выхода. Ни один битый или злонамеренный файл не должен подвесить
// разбор — он обязан кончиться `RtError` с внятным текстом.

/// Предел вложенности значений PDF (массивы и словари друг в друге) и
/// глубины дерева страниц. Настоящие файлы не подходят к нему близко.
const MAX_DEPTH: usize = 64;

/// Предел на длину цепочки таблиц перекрёстных ссылок (`/Prev` и
/// `/XRefStm`). Инкрементальных обновлений в живом файле единицы.
const MAX_XREF_SECTIONS: usize = 1024;

/// Предел на размер РАСПАКОВАННОГО потока. Без него подделанный поток
/// раздувает память: `FlateDecode` умеет отдавать гигабайты из килобайт.
const MAX_STREAM_OUT: usize = 256 << 20;

/// Предел на число страниц в дереве. Дерево обходится с множеством
/// посещённых номеров, поэтому предел нужен только для страниц, вписанных
/// в `/Kids` НЕ ссылкой, а словарём на месте.
const MAX_PAGES: usize = 1 << 20;

/// Страница разобранного файла: только то, что отдаёт платформа.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PdfPageInfo {
    width_mm: i64,
    height_mm: i64,
    rotate: i64,
    /// Поля в миллиметрах в порядке «слева, справа, сверху, снизу» —
    /// ровно том, в каком их читает [`PdfPageInfo::margin`].
    margins: [i64; 4],
}

/// Какое из четырёх полей страницы нужно.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PdfMargin {
    Left,
    Right,
    Top,
    Bottom,
}

impl PdfPageInfo {
    /// Ширина видимой области в миллиметрах.
    pub fn width_mm(&self) -> i64 {
        self.width_mm
    }

    /// Высота видимой области в миллиметрах.
    pub fn height_mm(&self) -> i64 {
        self.height_mm
    }

    /// Поворот страницы в градусах: 0, 90, 180 или 270.
    pub fn rotate(&self) -> i64 {
        self.rotate
    }

    /// Поле страницы в миллиметрах. Бывает отрицательным (измерено).
    pub fn margin(&self, which: PdfMargin) -> i64 {
        match which {
            PdfMargin::Left => self.margins[0],
            PdfMargin::Right => self.margins[1],
            PdfMargin::Top => self.margins[2],
            PdfMargin::Bottom => self.margins[3],
        }
    }
}

/// Предел на число вложений. Как и [`MAX_PAGES`], он нужен ради записей,
/// вписанных в дерево имён словарём на месте, а не ссылкой: их множество
/// посещённых номеров не ловит.
const MAX_ATTACHMENTS: usize = 1 << 16;

/// Связь вложения с документом — `/AFRelationship` файловой спецификации,
/// она же свойство `ТипСвязи`.
///
/// Членов ровно пять, и это ИЗМЕРЕНО перебором на 8.3.27: перечисление
/// называется `ТипСвязиВложенияPDF`, у него есть `Источник`/`Source`,
/// `Данные`/`Data`, `Альтернатива`/`Alternative`, `Дополнение`/`Supplement`
/// и `НеУстановлено`/`Unspecified`, а `Схема`, `ДанныеФормы` и
/// `ЗашифрованныеДанные` (они есть в таблице 43 спецификации) платформа не
/// знает. Неизвестное имя в файле читается как `НеУстановлено` — измерено
/// на `/AFRelationship /Nonsense`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PdfRelation {
    /// `/Source` — вложение является источником документа.
    Source,
    /// `/Data` — данные, по которым документ построен.
    Data,
    /// `/Alternative` — альтернативное представление.
    Alternative,
    /// `/Supplement` — дополнение к документу.
    Supplement,
    /// `/Unspecified` — связь не указана; ею же читается и неизвестное имя.
    #[default]
    Unspecified,
}

impl PdfRelation {
    /// Связь по имени из файла. Всё неизвестное — `Unspecified` (измерено).
    fn from_pdf_name(name: &str) -> PdfRelation {
        match name {
            "Source" => PdfRelation::Source,
            "Data" => PdfRelation::Data,
            "Alternative" => PdfRelation::Alternative,
            "Supplement" => PdfRelation::Supplement,
            _ => PdfRelation::Unspecified,
        }
    }

    /// Имя `/AFRelationship`, каким его пишет и платформа.
    pub fn pdf_name(self) -> &'static str {
        match self {
            PdfRelation::Source => "Source",
            PdfRelation::Data => "Data",
            PdfRelation::Alternative => "Alternative",
            PdfRelation::Supplement => "Supplement",
            PdfRelation::Unspecified => "Unspecified",
        }
    }
}

/// Вложение PDF — то, что платформа отдаёт как `ВложениеPDF`.
///
/// Наружу видны ровно четыре свойства, и все четыре ИЗМЕРЕНЫ перебором:
/// `ИмяФайла`, `ТипСодержимого`, `Содержимое` и `ТипСвязи`. Ни `Имя`, ни
/// `Описание`, ни `Размер`, ни `ДатаСоздания` платформа не знает, поэтому
/// `/Desc` и `/Params` из файла сюда не попадают вовсе.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PdfAttachment {
    name: String,
    content_type: String,
    relation: PdfRelation,
    data: Vec<u8>,
}

impl PdfAttachment {
    /// Собрать вложение из имени, типа содержимого, связи и байтов.
    pub fn new(
        name: String,
        content_type: String,
        relation: PdfRelation,
        data: Vec<u8>,
    ) -> PdfAttachment {
        PdfAttachment {
            name,
            content_type,
            relation,
            data,
        }
    }

    /// `ИмяФайла` — имя из `/UF`, а если его нет, из `/F`.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// `ТипСодержимого` — `/Subtype` потока встроенного файла. Пустая
    /// строка, если его в файле нет (измерено).
    pub fn content_type(&self) -> &str {
        &self.content_type
    }

    /// `ТипСвязи` — `/AFRelationship`.
    pub fn relation(&self) -> PdfRelation {
        self.relation
    }

    /// `Содержимое` — распакованные байты встроенного файла.
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Задать имя (свойство доступно на запись — измерено).
    pub fn set_name(&mut self, name: String) {
        self.name = name;
    }

    /// Задать тип содержимого.
    pub fn set_content_type(&mut self, content_type: String) {
        self.content_type = content_type;
    }

    /// Задать связь.
    pub fn set_relation(&mut self, relation: PdfRelation) {
        self.relation = relation;
    }

    /// Задать байты содержимого.
    pub fn set_data(&mut self, data: Vec<u8>) {
        self.data = data;
    }
}

/// Разобранный файл PDF.
///
/// Кроме страниц и вложений здесь лежат ИСХОДНЫЕ БАЙТЫ файла и всё, что
/// нужно, чтобы дописать к ним инкрементальное обновление: номер объекта
/// каталога, сам каталог, его словарь `/Names` и смещение последней
/// таблицы перекрёстных ссылок. Держать файл целиком в памяти — плата за
/// запись, которая ничего не теряет: страницы, шрифты и содержимое
/// остаются ровно теми байтами, что пришли с диска.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PdfFile {
    pages: Vec<PdfPageInfo>,
    attachments: Vec<PdfAttachment>,
    source: Vec<u8>,
    /// Номер объекта каталога. `None`, если `/Root` в трейлере — словарь на
    /// месте, а не ссылка: такой каталог инкрементальным обновлением не
    /// переписать.
    catalog_number: Option<u32>,
    catalog: Vec<(String, PdfValue)>,
    /// Разрешённый словарь каталога `/Names` без `/EmbeddedFiles`: при
    /// записи он переносится целиком, чтобы не потерять чужие деревья имён
    /// (`/Dests`, `/JavaScript`).
    names_dict: Vec<(String, PdfValue)>,
    /// Смещение таблицы перекрёстных ссылок, с которой начался разбор, —
    /// оно же `/Prev` нового обновления.
    startxref: usize,
    /// Номер, с которого можно заводить новые объекты.
    next_object: u32,
}

impl PdfFile {
    /// Разобрать байты файла.
    ///
    /// # Errors
    ///
    /// [`RtError::Pdf`] на любом входе, который не является читаемым PDF:
    /// нет заголовка `%PDF-`, нет или испорчен `startxref`, битая таблица
    /// перекрёстных ссылок, неизвестный фильтр или предиктор, цикл в
    /// ссылках, в дереве страниц или в дереве имён вложений, отсутствующий
    /// `/Root`. Отдельным текстом сообщается о зашифрованном файле:
    /// расшифровки здесь нет.
    pub fn parse(data: &[u8]) -> RtResult<PdfFile> {
        Reader::new(data)?.read_file()
    }

    /// Число страниц.
    pub fn page_count(&self) -> usize {
        self.pages.len()
    }

    /// Страница по номеру с нуля.
    pub fn page(&self, index: usize) -> Option<&PdfPageInfo> {
        self.pages.get(index)
    }

    /// Вложения в порядке обхода дерева имён.
    pub fn attachments(&self) -> &[PdfAttachment] {
        &self.attachments
    }
}

fn pdf_err(text: impl Into<String>) -> RtError {
    RtError::Pdf(text.into())
}

/// Байты ТЕКСТОВОЙ строки PDF (раздел 7.9.2.2) в строку языка: с меткой
/// `FE FF` это UTF-16BE, иначе UTF-8.
///
/// Спецификация на месте UTF-8 называет `PDFDocEncoding`, но платформа
/// читает именно UTF-8. Измерены ровно пять написаний имени (строки
/// «имя 0» — «имя 4» в `measure-pdf-attachments.platform.txt`): хекс-строка
/// `/F <6865782E747874>` возвращается как есть, «6865782e747874»;
/// восьмеричные экраны в `/UF` — тоже как есть, с обратными косыми; сырые
/// байты UTF-8 в `/F` дают «сырой.txt»; сырой UTF-16BE с меткой `FE FF` в
/// `/UF` — «шестнадцать.txt»; при обоих ключах сразу побеждает `/UF`.
/// Что платформа делает с байтами, которые не UTF-8 (например с cp1251),
/// не измерено; здесь такой вход не роняет разбор: и `from_utf16_lossy`,
/// и `from_utf8_lossy` ставят U+FFFD.
fn decode_text_string(bytes: &[u8]) -> String {
    if bytes.len() >= 2 && bytes[0] == 0xFE && bytes[1] == 0xFF {
        let units: Vec<u16> = bytes[2..]
            .chunks_exact(2)
            .map(|pair| u16::from_be_bytes([pair[0], pair[1]]))
            .collect();
        return String::from_utf16_lossy(&units);
    }
    String::from_utf8_lossy(bytes).into_owned()
}

/// Записать строку файловой спецификации СЫРЫМИ байтами: экранируются
/// только три знака, обязательных для того, чтобы строка кончилась там,
/// где надо.
///
/// Отличие от [`write_str`] измерено и существенно: разбор строк у
/// платформы неполон. Восьмеричные экраны `\ooo` она НЕ снимает, а
/// шестнадцатеричную форму `<...>` НЕ разбирает — и то и другое отдаёт
/// текстом как есть (пробы «имя 1» и «имя 0» в
/// `measure-pdf-attachments.platform.txt`). Поэтому имя, записанное
/// через `write_str`, который уводит всё непечатное в `\ooo`, вернулось
/// бы к ней как «\376\377\000i\000n\000c», а не как «inc.txt».
/// Снимает ли платформа `\\`, `\(` и `\)` — не пробовали: ни в корпусе
/// фикстур, ни в её собственном выводе нет ни одного имени с этими
/// байтами. Экранирование трёх разделителей здесь — не воспроизведение
/// замера, а синтаксическая необходимость записи, названная в первом
/// абзаце. Сама платформа пишет `/F` в UTF-8 и `/UF` в UTF-16BE, оба
/// сырыми байтами, — здесь ровно то же самое.
fn write_raw_str(out: &mut Vec<u8>, bytes: &[u8]) {
    out.push(b'(');
    for &b in bytes {
        if matches!(b, b'(' | b')' | b'\\') {
            out.push(b'\\');
        }
        out.push(b);
    }
    out.push(b')');
}

/// Занять следующий номер объекта. Проверенное сложение: номера растут от
/// того, что было в чужом файле, и упереться в потолок `u32` они обязаны
/// ошибкой, а не заворачиванием на уже занятый объект.
fn take_object_number(next: &mut u32) -> RtResult<u32> {
    let taken = *next;
    *next = next
        .checked_add(1)
        .ok_or_else(|| pdf_err("в файле кончились номера объектов"))?;
    Ok(taken)
}

/// Имя вложения в UTF-16BE с меткой порядка байтов — то, что идёт в `/UF`.
fn utf16be_with_bom(text: &str) -> Vec<u8> {
    let mut out = vec![0xFE, 0xFF];
    for unit in text.encode_utf16() {
        out.extend_from_slice(&unit.to_be_bytes());
    }
    out
}

impl PdfFile {
    /// Записать документ с ЭТИМ набором вложений — инкрементальным
    /// обновлением поверх исходных байт (раздел 7.5.6).
    ///
    /// # Почему обновление, а не перезапись
    ///
    /// Платформа поступает иначе: `ДокументPDF.Записать` собирает файл
    /// заново. В снятом образце `tests/conformance/pdf/attach-platform.pdf`
    /// объекты перенумерованы с единицы подряд (с `1 0 obj` по `23 0 obj`),
    /// каталог — `14 0 obj` с `/PageMode /UseAttachments` и
    /// `/AF [ 7 0 R 8 0 R 9 0 R ]`, добавлен `/Metadata` с XMP
    /// «1C:Enterprise (8.3.27.2130)», а таблица `xref` классическая, с CRLF
    /// и одной секцией `0 24` при `/Size 24` в трейлере.
    /// Повторить это можно только удержав в памяти ВЕСЬ граф объектов
    /// файла, а читатель здесь берёт из него лишь геометрию страниц и
    /// вложения: всё остальное — шрифты, содержимое страниц, аннотации —
    /// он не разбирает и разбирать не должен. Инкрементальное обновление
    /// сохраняет их точно, потому что не трогает ни одного байта исходного
    /// файла, и ИЗМЕРЕНО, что платформа такой файл читает: собранное этим
    /// способом обновление она открыла как «страниц 1, вложений 2» и
    /// отдала оба вложения с верным содержимым.
    ///
    /// Плата — размер: байты старых встроенных файлов остаются в файле
    /// мусором, а новые пишутся заново даже для неизменённых вложений.
    /// Взамен запись не зависит от того, что читатель умеет разбирать.
    ///
    /// # Errors
    ///
    /// [`RtError::Pdf`], если `/Root` в трейлере был словарём на месте, а
    /// не ссылкой (такой каталог нечем заменить), или если номера объектов
    /// не помещаются в `u32`.
    pub fn write_with_attachments(&self, attachments: &[PdfAttachment]) -> RtResult<Vec<u8>> {
        let Some(catalog_number) = self.catalog_number else {
            return Err(pdf_err(
                "каталог документа записан словарём на месте, а не объектом: \
                 такой файл нечем обновить",
            ));
        };
        let mut out = self.source.clone();
        if !out.ends_with(b"\n") {
            out.push(b'\n');
        }
        let mut next = self.next_object;
        // Порядок записи в `/Names` — по имени, как требует раздел 7.9.6 и
        // как пишет платформа (измерено: её вывод отсортирован по
        // кодовым единицам UTF-16).
        let mut ordered: Vec<&PdfAttachment> = attachments.iter().collect();
        ordered.sort_by_key(|item| item.name.encode_utf16().collect::<Vec<u16>>());
        let mut offsets: Vec<(u32, usize)> = Vec::new();
        let mut names: Vec<(Vec<u8>, u32)> = Vec::new();
        for item in &ordered {
            let stream_number = take_object_number(&mut next)?;
            let spec_number = take_object_number(&mut next)?;
            let packed = zlib_compress(&item.data);
            offsets.push((stream_number, out.len()));
            let mut dict = vec![
                (
                    "Type".to_string(),
                    PdfValue::Name("EmbeddedFile".to_string()),
                ),
                (
                    "Filter".to_string(),
                    PdfValue::Name("FlateDecode".to_string()),
                ),
            ];
            // Пустой `/Subtype` платформа при записи заменяет на
            // `application/octet-stream` (измерено на её собственном
            // выводе), и здесь то же самое: иначе перечитанное вложение
            // меняло бы тип на пустой.
            let content_type = if item.content_type.is_empty() {
                "application/octet-stream"
            } else {
                item.content_type.as_str()
            };
            dict.push((
                "Subtype".to_string(),
                PdfValue::Name(content_type.to_string()),
            ));
            dict.push((
                "Params".to_string(),
                PdfValue::Dict(vec![(
                    "Size".to_string(),
                    PdfValue::Integer(item.data.len() as i64),
                )]),
            ));
            out.extend_from_slice(format!("{stream_number} 0 obj\n").as_bytes());
            write_value(&mut out, &PdfValue::Stream { dict, data: packed });
            out.extend_from_slice(b"\nendobj\n");

            offsets.push((spec_number, out.len()));
            out.extend_from_slice(
                format!("{spec_number} 0 obj\n<< /Type /Filespec /F ").as_bytes(),
            );
            write_raw_str(&mut out, item.name.as_bytes());
            out.extend_from_slice(b" /UF ");
            write_raw_str(&mut out, &utf16be_with_bom(&item.name));
            out.extend_from_slice(
                format!(
                    " /EF << /F {stream_number} 0 R >> /AFRelationship /{} >>\nendobj\n",
                    item.relation.pdf_name()
                )
                .as_bytes(),
            );
            names.push((utf16be_with_bom(&item.name), spec_number));
        }

        let names_number = take_object_number(&mut next)?;
        offsets.push((names_number, out.len()));
        out.extend_from_slice(format!("{names_number} 0 obj\n<< /Names [").as_bytes());
        for (key, spec_number) in &names {
            out.push(b' ');
            write_raw_str(&mut out, key);
            out.extend_from_slice(format!(" {spec_number} 0 R").as_bytes());
        }
        out.extend_from_slice(b" ] >>\nendobj\n");

        let names_dict_number = take_object_number(&mut next)?;
        offsets.push((names_dict_number, out.len()));
        let mut names_dict: Vec<(String, PdfValue)> = self
            .names_dict
            .iter()
            .filter(|(key, _)| key != "EmbeddedFiles")
            .cloned()
            .collect();
        names_dict.push(("EmbeddedFiles".to_string(), PdfValue::Ref(names_number)));
        out.extend_from_slice(format!("{names_dict_number} 0 obj\n").as_bytes());
        write_value(&mut out, &PdfValue::Dict(names_dict));
        out.extend_from_slice(b"\nendobj\n");

        let mut catalog: Vec<(String, PdfValue)> = self
            .catalog
            .iter()
            .filter(|(key, _)| key != "Names")
            .cloned()
            .collect();
        catalog.push(("Names".to_string(), PdfValue::Ref(names_dict_number)));
        offsets.push((catalog_number, out.len()));
        out.extend_from_slice(format!("{catalog_number} 0 obj\n").as_bytes());
        write_value(&mut out, &PdfValue::Dict(catalog));
        out.extend_from_slice(b"\nendobj\n");

        let xref_at = out.len();
        offsets.sort_unstable();
        out.extend_from_slice(b"xref\n");
        let mut at = 0;
        while at < offsets.len() {
            let mut end = at + 1;
            while end < offsets.len() && offsets[end].0 == offsets[end - 1].0 + 1 {
                end += 1;
            }
            out.extend_from_slice(format!("{} {}\n", offsets[at].0, end - at).as_bytes());
            for (_, offset) in &offsets[at..end] {
                out.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
            }
            at = end;
        }
        out.extend_from_slice(
            format!(
                "trailer\n<< /Size {next} /Root {catalog_number} 0 R /Prev {} >>\n\
                 startxref\n{xref_at}\n%%EOF\n",
                self.startxref
            )
            .as_bytes(),
        );
        Ok(out)
    }
}

/// Пробельные знаки PDF (таблица 1 спецификации).
fn is_ws(b: u8) -> bool {
    matches!(b, 0x00 | 0x09 | 0x0A | 0x0C | 0x0D | 0x20)
}

/// Разделители PDF (таблица 2): на них кончается любая лексема.
fn is_delim(b: u8) -> bool {
    matches!(
        b,
        b'(' | b')' | b'<' | b'>' | b'[' | b']' | b'{' | b'}' | b'/' | b'%'
    )
}

fn is_regular(b: u8) -> bool {
    !is_ws(b) && !is_delim(b)
}

/// Значение по ключу словаря. Ключи PDF регистрозависимы.
fn dict_get<'a>(dict: &'a [(String, PdfValue)], key: &str) -> Option<&'a PdfValue> {
    dict.iter().find(|(k, _)| k == key).map(|(_, v)| v)
}

/// Лексер и разборщик значений. Потоки он не разбирает — за ними нужен
/// `/Length`, а тот бывает косвенной ссылкой, то есть уже [`Reader`].
struct Lexer<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Lexer<'a> {
    fn new(data: &'a [u8], pos: usize) -> Self {
        Lexer { data, pos }
    }

    fn peek(&self) -> Option<u8> {
        self.data.get(self.pos).copied()
    }

    /// Пробелы и комментарии `%` до конца строки.
    fn skip_ws(&mut self) {
        loop {
            match self.peek() {
                Some(b) if is_ws(b) => self.pos += 1,
                Some(b'%') => {
                    while let Some(b) = self.peek() {
                        self.pos += 1;
                        if b == b'\n' || b == b'\r' {
                            break;
                        }
                    }
                }
                _ => return,
            }
        }
    }

    /// Слово из «обычных» знаков: `obj`, `endobj`, `stream`, `true`, число.
    fn token(&mut self) -> &'a [u8] {
        let start = self.pos;
        while let Some(b) = self.peek() {
            if is_regular(b) {
                self.pos += 1;
            } else {
                break;
            }
        }
        &self.data[start..self.pos]
    }

    fn expect_keyword(&mut self, word: &str) -> RtResult<()> {
        self.skip_ws();
        let tok = self.token();
        if tok == word.as_bytes() {
            Ok(())
        } else {
            Err(pdf_err(format!(
                "ожидалось «{word}», получено «{}»",
                String::from_utf8_lossy(tok)
            )))
        }
    }

    /// Целое без знака: номер объекта, поколение, длина.
    fn unsigned(&mut self) -> Option<u64> {
        self.skip_ws();
        let start = self.pos;
        while matches!(self.peek(), Some(b'0'..=b'9')) {
            self.pos += 1;
        }
        if self.pos == start {
            return None;
        }
        std::str::from_utf8(&self.data[start..self.pos])
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
    }

    fn parse_value(&mut self, depth: usize) -> RtResult<PdfValue> {
        if depth > MAX_DEPTH {
            return Err(pdf_err("слишком глубокая вложенность значений"));
        }
        self.skip_ws();
        let Some(b) = self.peek() else {
            return Err(pdf_err("значение оборвалось на конце файла"));
        };
        match b {
            b'/' => self.parse_name(),
            b'(' => self.parse_literal_string(),
            b'[' => {
                self.pos += 1;
                let mut items = Vec::new();
                loop {
                    self.skip_ws();
                    match self.peek() {
                        Some(b']') => {
                            self.pos += 1;
                            return Ok(PdfValue::Array(items));
                        }
                        None => return Err(pdf_err("массив без закрывающей скобки")),
                        _ => items.push(self.parse_value(depth + 1)?),
                    }
                }
            }
            b'<' => {
                if self.data.get(self.pos + 1) == Some(&b'<') {
                    let dict = self.parse_dict(depth)?;
                    Ok(PdfValue::Dict(dict))
                } else {
                    self.parse_hex_string()
                }
            }
            b'+' | b'-' | b'.' | b'0'..=b'9' => self.parse_number_or_ref(),
            b't' | b'f' | b'n' => {
                let tok = self.token();
                match tok {
                    b"true" => Ok(PdfValue::Bool(true)),
                    b"false" => Ok(PdfValue::Bool(false)),
                    b"null" => Ok(PdfValue::Null),
                    other => Err(pdf_err(format!(
                        "неизвестная лексема «{}»",
                        String::from_utf8_lossy(other)
                    ))),
                }
            }
            other => Err(pdf_err(format!(
                "значение не начинается ни с чего осмысленного: байт 0x{other:02X}"
            ))),
        }
    }

    fn parse_dict(&mut self, depth: usize) -> RtResult<Vec<(String, PdfValue)>> {
        if depth > MAX_DEPTH {
            return Err(pdf_err("слишком глубокая вложенность значений"));
        }
        self.pos += 2; // `<<`
        let mut out = Vec::new();
        loop {
            self.skip_ws();
            match self.peek() {
                Some(b'>') => {
                    if self.data.get(self.pos + 1) == Some(&b'>') {
                        self.pos += 2;
                        return Ok(out);
                    }
                    return Err(pdf_err("одиночная «>» в словаре"));
                }
                Some(b'/') => {
                    let PdfValue::Name(key) = self.parse_name()? else {
                        unreachable!("parse_name отдаёт только имя");
                    };
                    let value = self.parse_value(depth + 1)?;
                    out.push((key, value));
                }
                None => return Err(pdf_err("словарь без закрывающих «>>»")),
                Some(other) => {
                    return Err(pdf_err(format!(
                        "ключ словаря должен начинаться с «/», получен байт 0x{other:02X}"
                    )))
                }
            }
        }
    }

    /// Имя `/Name`. Escape-последовательности `#xx` раскрываются, поэтому
    /// `/A#20B` и `/A B` — разные имена, а `/Ко#Д` — ошибка.
    fn parse_name(&mut self) -> RtResult<PdfValue> {
        self.pos += 1; // `/`
        let mut bytes = Vec::new();
        while let Some(b) = self.peek() {
            if !is_regular(b) {
                break;
            }
            self.pos += 1;
            if b == b'#' {
                let hi = self.peek().and_then(hex_digit);
                let lo = self.data.get(self.pos + 1).copied().and_then(hex_digit);
                match (hi, lo) {
                    (Some(h), Some(l)) => {
                        bytes.push(h * 16 + l);
                        self.pos += 2;
                    }
                    _ => {
                        return Err(pdf_err(
                            "в имени после «#» ожидались две шестнадцатеричные цифры",
                        ))
                    }
                }
            } else {
                bytes.push(b);
            }
        }
        // Имена в PDF — последовательности байтов; для нас это ключи
        // словарей, все из ASCII, поэтому не-UTF-8 заменяется заменяющим
        // знаком и просто не совпадёт ни с одним известным ключом.
        Ok(PdfValue::Name(String::from_utf8_lossy(&bytes).into_owned()))
    }

    /// Строка `(...)`: экранирование обратной косой и БАЛАНС вложенных
    /// круглых скобок (раздел 7.3.4.2).
    fn parse_literal_string(&mut self) -> RtResult<PdfValue> {
        self.pos += 1; // `(`
        let mut out = Vec::new();
        let mut depth = 1usize;
        while let Some(b) = self.peek() {
            self.pos += 1;
            match b {
                b'\\' => {
                    let Some(esc) = self.peek() else {
                        return Err(pdf_err("строка оборвалась после обратной косой черты"));
                    };
                    self.pos += 1;
                    match esc {
                        b'n' => out.push(b'\n'),
                        b'r' => out.push(b'\r'),
                        b't' => out.push(b'\t'),
                        b'b' => out.push(0x08),
                        b'f' => out.push(0x0C),
                        b'(' => out.push(b'('),
                        b')' => out.push(b')'),
                        b'\\' => out.push(b'\\'),
                        // Перевод строки после косой — склейка строк.
                        b'\n' => {}
                        b'\r' => {
                            if self.peek() == Some(b'\n') {
                                self.pos += 1;
                            }
                        }
                        b'0'..=b'7' => {
                            let mut code = u32::from(esc - b'0');
                            for _ in 0..2 {
                                match self.peek() {
                                    Some(d @ b'0'..=b'7') => {
                                        code = code * 8 + u32::from(d - b'0');
                                        self.pos += 1;
                                    }
                                    _ => break,
                                }
                            }
                            out.push((code & 0xFF) as u8);
                        }
                        other => out.push(other),
                    }
                }
                b'(' => {
                    depth += 1;
                    out.push(b'(');
                }
                b')' => {
                    depth -= 1;
                    if depth == 0 {
                        return Ok(PdfValue::Str(out));
                    }
                    out.push(b')');
                }
                other => out.push(other),
            }
        }
        Err(pdf_err("строка без закрывающей круглой скобки"))
    }

    /// Строка `<...>`: шестнадцатеричная, нечётный хвост дополняется нулём.
    fn parse_hex_string(&mut self) -> RtResult<PdfValue> {
        self.pos += 1; // `<`
        let mut out = Vec::new();
        let mut half: Option<u8> = None;
        while let Some(b) = self.peek() {
            self.pos += 1;
            if b == b'>' {
                if let Some(h) = half {
                    out.push(h * 16);
                }
                return Ok(PdfValue::Str(out));
            }
            if is_ws(b) {
                continue;
            }
            let Some(d) = hex_digit(b) else {
                return Err(pdf_err(format!(
                    "в шестнадцатеричной строке недопустимый байт 0x{b:02X}"
                )));
            };
            match half.take() {
                None => half = Some(d),
                Some(h) => out.push(h * 16 + d),
            }
        }
        Err(pdf_err("шестнадцатеричная строка без закрывающей «>»"))
    }

    /// Число, а если за ним стоят второе целое и `R` — косвенная ссылка.
    fn parse_number_or_ref(&mut self) -> RtResult<PdfValue> {
        let start = self.pos;
        if matches!(self.peek(), Some(b'+') | Some(b'-')) {
            self.pos += 1;
        }
        let mut seen_dot = false;
        while let Some(b) = self.peek() {
            match b {
                b'0'..=b'9' => self.pos += 1,
                b'.' if !seen_dot => {
                    seen_dot = true;
                    self.pos += 1;
                }
                // Второй знак минуса внутри числа встречается в файлах от
                // небрежных писателей («--5»); остаток числа просто
                // отбрасывается вместе с ним.
                _ => break,
            }
        }
        let text = std::str::from_utf8(&self.data[start..self.pos])
            .map_err(|_| pdf_err("число содержит не-ASCII"))?;
        let value = if seen_dot {
            let x: f64 = text
                .parse()
                .map_err(|_| pdf_err(format!("не число: «{text}»")))?;
            return Ok(PdfValue::Real(x));
        } else {
            text.parse::<i64>()
                .map_err(|_| pdf_err(format!("не целое: «{text}»")))?
        };
        // Косвенная ссылка `N G R`: пробуем и откатываемся, если не вышло.
        if value >= 0 {
            let save = self.pos;
            let mut probe = Lexer::new(self.data, self.pos);
            if probe.unsigned().is_some() {
                probe.skip_ws();
                if probe.token() == b"R" {
                    self.pos = probe.pos;
                    // Номер объекта шире `u32` — честная ошибка, а не молча
                    // срезанная ссылка на чужой объект.
                    let number = u32::try_from(value)
                        .map_err(|_| pdf_err("слишком большой номер объекта в ссылке"))?;
                    return Ok(PdfValue::Ref(number));
                }
            }
            self.pos = save;
        }
        Ok(PdfValue::Integer(value))
    }
}

fn hex_digit(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// Где лежит объект: прямо в файле или внутри объектного потока.
#[derive(Debug, Clone, Copy)]
enum XrefLoc {
    /// Смещение от начала файла.
    Offset(usize),
    /// Номер объектного потока и порядковый номер объекта в нём.
    InStream { stream: u32, index: usize },
}

/// Разобранный объектный поток `/Type /ObjStm`: распакованные байты и
/// таблица «номер объекта — смещение внутри».
struct ObjStm {
    data: Vec<u8>,
    entries: Vec<(u32, usize)>,
}

struct Reader<'a> {
    data: &'a [u8],
    xref: std::collections::HashMap<u32, XrefLoc>,
    trailer: Vec<(String, PdfValue)>,
    cache: std::collections::HashMap<u32, PdfValue>,
    /// Стек номеров разбираемых сейчас объектов — защита от `1 0 obj 1 0 R`.
    active: Vec<u32>,
    streams: std::collections::HashMap<u32, std::rc::Rc<ObjStm>>,
    /// Смещение таблицы, с которой начался разбор: `/Prev` будущего
    /// инкрементального обновления.
    startxref: usize,
}

impl<'a> Reader<'a> {
    fn new(data: &'a [u8]) -> RtResult<Reader<'a>> {
        // Заголовок разрешено смещать от начала файла (раздел 7.5.2), но
        // не дальше первого килобайта.
        let head = &data[..data.len().min(1024)];
        if !head.windows(5).any(|w| w == b"%PDF-") {
            return Err(pdf_err("это не PDF: в начале файла нет «%PDF-»"));
        }
        let mut reader = Reader {
            data,
            xref: std::collections::HashMap::new(),
            trailer: Vec::new(),
            cache: std::collections::HashMap::new(),
            active: Vec::new(),
            streams: std::collections::HashMap::new(),
            startxref: 0,
        };
        let start = reader.find_startxref()?;
        reader.startxref = start;
        reader.load_xref_chain(start)?;
        if dict_get(&reader.trailer, "Encrypt").is_some() {
            return Err(pdf_err(
                "документ зашифрован: чтение зашифрованных PDF не поддерживается",
            ));
        }
        Ok(reader)
    }

    /// Последний `startxref` файла и стоящее за ним смещение.
    fn find_startxref(&self) -> RtResult<usize> {
        let tail_from = self.data.len().saturating_sub(2048);
        let tail = &self.data[tail_from..];
        let at = tail
            .windows(9)
            .rposition(|w| w == b"startxref")
            .ok_or_else(|| pdf_err("в конце файла нет «startxref»"))?;
        let mut lexer = Lexer::new(self.data, tail_from + at + 9);
        let offset = lexer
            .unsigned()
            .ok_or_else(|| pdf_err("после «startxref» нет смещения"))?;
        let offset = usize::try_from(offset).unwrap_or(usize::MAX);
        if offset >= self.data.len() {
            return Err(pdf_err(format!(
                "«startxref» указывает за пределы файла: {offset}"
            )));
        }
        Ok(offset)
    }

    /// Вся цепочка таблиц: сама таблица, её `/XRefStm` (гибридные файлы) и
    /// `/Prev` (инкрементальные обновления). Порядок обхода — от НОВОГО к
    /// старому, и запись побеждает первая: так и работает обновление.
    fn load_xref_chain(&mut self, start: usize) -> RtResult<()> {
        let mut seen = std::collections::HashSet::new();
        let mut pending = std::collections::VecDeque::new();
        pending.push_back(start);
        let mut sections = 0usize;
        while let Some(offset) = pending.pop_front() {
            if !seen.insert(offset) {
                continue;
            }
            sections += 1;
            if sections > MAX_XREF_SECTIONS {
                return Err(pdf_err(
                    "слишком длинная цепочка таблиц перекрёстных ссылок",
                ));
            }
            let section = self.load_xref_section(offset)?;
            for key in ["XRefStm", "Prev"] {
                if let Some(PdfValue::Integer(prev)) = dict_get(&section, key) {
                    if let Ok(prev) = usize::try_from(*prev) {
                        if prev < self.data.len() {
                            pending.push_back(prev);
                        }
                    }
                }
            }
            for (key, value) in section {
                if dict_get(&self.trailer, &key).is_none() {
                    self.trailer.push((key, value));
                }
            }
        }
        Ok(())
    }

    /// Одна таблица: классическая `xref` либо `xref`-поток. Отдаёт трейлер.
    fn load_xref_section(&mut self, offset: usize) -> RtResult<Vec<(String, PdfValue)>> {
        let mut lexer = Lexer::new(self.data, offset);
        lexer.skip_ws();
        if self.data[lexer.pos..].starts_with(b"xref") {
            lexer.pos += 4;
            return self.load_classic_xref(lexer);
        }
        self.load_xref_stream(offset)
    }

    fn load_classic_xref(&mut self, mut lexer: Lexer<'a>) -> RtResult<Vec<(String, PdfValue)>> {
        loop {
            lexer.skip_ws();
            if self.data[lexer.pos..].starts_with(b"trailer") {
                lexer.pos += 7;
                lexer.skip_ws();
                if lexer.peek() != Some(b'<') {
                    return Err(pdf_err("после «trailer» нет словаря"));
                }
                return lexer.parse_dict(0);
            }
            let first = lexer
                .unsigned()
                .ok_or_else(|| pdf_err("в таблице xref ожидался номер первого объекта"))?;
            let count = lexer
                .unsigned()
                .ok_or_else(|| pdf_err("в таблице xref ожидалось число записей"))?;
            if count > u64::try_from(self.data.len()).unwrap_or(u64::MAX) {
                return Err(pdf_err(
                    "в таблице xref объявлено больше записей, чем байт в файле",
                ));
            }
            for i in 0..count {
                let position = lexer
                    .unsigned()
                    .ok_or_else(|| pdf_err("запись xref без смещения"))?;
                let _generation = lexer
                    .unsigned()
                    .ok_or_else(|| pdf_err("запись xref без номера поколения"))?;
                lexer.skip_ws();
                let kind = lexer.token();
                // Сложение проверенное: `first` и `count` пришли из файла,
                // и их сумма обязана быть номером объекта, а не переполнением.
                let number = first
                    .checked_add(i)
                    .and_then(|number| u32::try_from(number).ok())
                    .ok_or_else(|| pdf_err("слишком большой номер объекта в таблице xref"))?;
                match kind {
                    b"n" => {
                        if let Ok(position) = usize::try_from(position) {
                            self.xref.entry(number).or_insert(XrefLoc::Offset(position));
                        }
                    }
                    b"f" => {
                        // Свободная запись: объекта нет. Занимать место в
                        // таблице ей всё равно нужно — иначе более старая
                        // секция вернула бы удалённый объект к жизни.
                        self.xref.entry(number).or_insert(XrefLoc::Offset(0));
                    }
                    other => {
                        return Err(pdf_err(format!(
                            "запись xref помечена «{}», а не «n» или «f»",
                            String::from_utf8_lossy(other)
                        )))
                    }
                }
            }
        }
    }

    fn load_xref_stream(&mut self, offset: usize) -> RtResult<Vec<(String, PdfValue)>> {
        let object = self.parse_object_at(offset)?;
        let PdfValue::Stream { dict, data } = object else {
            return Err(pdf_err(
                "«startxref» указывает не на таблицу xref и не на xref-поток",
            ));
        };
        let bytes = self.decode_stream(&dict, &data)?;
        let widths = match dict_get(&dict, "W") {
            Some(PdfValue::Array(items)) => items
                .iter()
                .map(|v| match v {
                    PdfValue::Integer(n) if *n >= 0 && *n <= 8 => Ok(*n as usize),
                    _ => Err(pdf_err("в /W xref-потока ожидались небольшие целые")),
                })
                .collect::<RtResult<Vec<_>>>()?,
            _ => return Err(pdf_err("в xref-потоке нет массива /W")),
        };
        if widths.len() < 3 {
            return Err(pdf_err("в /W xref-потока меньше трёх полей"));
        }
        let size = match dict_get(&dict, "Size") {
            Some(PdfValue::Integer(n)) if *n >= 0 => *n,
            _ => return Err(pdf_err("в xref-потоке нет /Size")),
        };
        let index: Vec<i64> = match dict_get(&dict, "Index") {
            Some(PdfValue::Array(items)) => items
                .iter()
                .map(|v| match v {
                    PdfValue::Integer(n) => Ok(*n),
                    _ => Err(pdf_err("в /Index xref-потока ожидались целые")),
                })
                .collect::<RtResult<Vec<_>>>()?,
            _ => vec![0, size],
        };
        let record = widths.iter().sum::<usize>();
        if record == 0 {
            return Err(pdf_err("в /W xref-потока все поля нулевой ширины"));
        }
        let mut at = 0usize;
        for pair in index.chunks(2) {
            let (&first, &count) = match pair {
                [first, count] => (first, count),
                _ => return Err(pdf_err("в /Index xref-потока нечётное число значений")),
            };
            for i in 0..count.max(0) {
                if at + record > bytes.len() {
                    return Err(pdf_err("xref-поток короче, чем объявлено в /Index"));
                }
                let mut field = [0u64; 3];
                let mut cursor = at;
                for (slot, width) in field.iter_mut().zip(widths.iter().copied()) {
                    let mut value = 0u64;
                    for _ in 0..width {
                        value = (value << 8) | u64::from(bytes[cursor]);
                        cursor += 1;
                    }
                    *slot = value;
                }
                at += record;
                // Нулевая ширина первого поля означает тип 1 (раздел 7.5.8.2).
                let kind = if widths[0] == 0 { 1 } else { field[0] };
                // Номер за пределами `u32` — запись просто пропускается, а
                // вот сумма за пределами `i64` означает мусорный `/Index`:
                // продолжать цикл после неё нечего, следующая же итерация
                // складывала бы `i64::MAX + 1`.
                let Some(number) = first.checked_add(i) else {
                    return Err(pdf_err(
                        "слишком большой номер объекта в /Index xref-потока",
                    ));
                };
                let Ok(number) = u32::try_from(number) else {
                    continue;
                };
                match kind {
                    1 => {
                        if let Ok(position) = usize::try_from(field[1]) {
                            self.xref.entry(number).or_insert(XrefLoc::Offset(position));
                        }
                    }
                    2 => {
                        if let (Ok(stream), Ok(index)) =
                            (u32::try_from(field[1]), usize::try_from(field[2]))
                        {
                            self.xref
                                .entry(number)
                                .or_insert(XrefLoc::InStream { stream, index });
                        }
                    }
                    // Тип 0 — свободный объект; всё остальное спецификация
                    // велит считать типом 0 (раздел 7.5.8.3).
                    _ => {
                        self.xref.entry(number).or_insert(XrefLoc::Offset(0));
                    }
                }
            }
        }
        Ok(dict)
    }

    /// Объект по номеру. Отсутствующий объект — это `null` (раздел 7.3.10),
    /// а не ошибка: так ведут себя все читатели, и так файл с испорченной
    /// таблицей всё же отдаёт то, что в нём уцелело.
    fn object(&mut self, number: u32) -> RtResult<PdfValue> {
        if let Some(value) = self.cache.get(&number) {
            return Ok(value.clone());
        }
        if self.active.contains(&number) {
            return Err(pdf_err(format!(
                "циклическая косвенная ссылка на объект {number}"
            )));
        }
        if self.active.len() > MAX_DEPTH {
            return Err(pdf_err("слишком глубокая цепочка косвенных ссылок"));
        }
        let Some(location) = self.xref.get(&number).copied() else {
            return Ok(PdfValue::Null);
        };
        self.active.push(number);
        let parsed = match location {
            XrefLoc::Offset(0) => Ok(PdfValue::Null),
            XrefLoc::Offset(offset) => self.parse_object_at(offset),
            XrefLoc::InStream { stream, index } => self.object_from_stream(stream, index),
        };
        self.active.pop();
        let value = parsed?;
        self.cache.insert(number, value.clone());
        Ok(value)
    }

    /// Значение, каким его видит потребитель: косвенная ссылка разыменована.
    fn resolve(&mut self, value: &PdfValue) -> RtResult<PdfValue> {
        match value {
            PdfValue::Ref(number) => self.object(*number),
            other => Ok(other.clone()),
        }
    }

    /// Разобрать `N G obj ... endobj` по смещению.
    fn parse_object_at(&mut self, offset: usize) -> RtResult<PdfValue> {
        if offset >= self.data.len() {
            return Err(pdf_err(format!(
                "объект за пределами файла: смещение {offset}"
            )));
        }
        let mut lexer = Lexer::new(self.data, offset);
        lexer
            .unsigned()
            .ok_or_else(|| pdf_err("объект не начинается с номера"))?;
        lexer
            .unsigned()
            .ok_or_else(|| pdf_err("у объекта нет номера поколения"))?;
        lexer.expect_keyword("obj")?;
        let value = lexer.parse_value(0)?;
        lexer.skip_ws();
        if !self.data[lexer.pos..].starts_with(b"stream") {
            return Ok(value);
        }
        let PdfValue::Dict(dict) = value else {
            return Err(pdf_err("перед «stream» должен стоять словарь"));
        };
        lexer.pos += 6;
        // После `stream` идёт CRLF или LF, но не одиночный CR (раздел 7.3.8.1).
        if self.data.get(lexer.pos) == Some(&b'\r') {
            lexer.pos += 1;
        }
        if self.data.get(lexer.pos) == Some(&b'\n') {
            lexer.pos += 1;
        }
        let start = lexer.pos;
        let declared = match dict_get(&dict, "Length") {
            Some(PdfValue::Integer(n)) if *n >= 0 => usize::try_from(*n).ok(),
            Some(PdfValue::Ref(number)) => match self.object(*number) {
                Ok(PdfValue::Integer(n)) if n >= 0 => usize::try_from(n).ok(),
                _ => None,
            },
            _ => None,
        };
        let end = match declared {
            Some(length) if self.ends_with_endstream(start, length) => start + length,
            // `/Length` соврал или его нет вовсе — ищем `endstream`. Платформа
            // такие файлы читает, и потерять из-за одного неверного числа
            // весь документ было бы хуже, чем довериться разделителю.
            _ => self.data[start..]
                .windows(9)
                .position(|w| w == b"endstream")
                .map(|at| start + at)
                .ok_or_else(|| pdf_err("поток без «endstream»"))?,
        };
        let mut data = self.data[start..end].to_vec();
        // Перевод строки перед `endstream` в поток не входит.
        if declared.is_none() || Some(end - start) != declared {
            if data.last() == Some(&b'\n') {
                data.pop();
            }
            if data.last() == Some(&b'\r') {
                data.pop();
            }
        }
        Ok(PdfValue::Stream { dict, data })
    }

    /// Стоит ли `endstream` там, где обещал `/Length`.
    fn ends_with_endstream(&self, start: usize, length: usize) -> bool {
        let Some(end) = start.checked_add(length) else {
            return false;
        };
        if end > self.data.len() {
            return false;
        }
        let mut lexer = Lexer::new(self.data, end);
        lexer.skip_ws();
        self.data[lexer.pos..].starts_with(b"endstream")
    }

    /// Объект из объектного потока `/Type /ObjStm`.
    fn object_from_stream(&mut self, stream: u32, index: usize) -> RtResult<PdfValue> {
        let objstm = self.object_stream(stream)?;
        let Some(&(_, offset)) = objstm.entries.get(index) else {
            return Err(pdf_err(format!(
                "в объектном потоке {stream} нет объекта с порядковым номером {index}"
            )));
        };
        if offset >= objstm.data.len() {
            return Err(pdf_err(format!(
                "объект {index} объектного потока {stream} лежит за его концом"
            )));
        }
        Lexer::new(&objstm.data, offset).parse_value(0)
    }

    fn object_stream(&mut self, number: u32) -> RtResult<std::rc::Rc<ObjStm>> {
        if let Some(found) = self.streams.get(&number) {
            return Ok(found.clone());
        }
        // Объектный поток лежит в файле напрямую: тип 2 внутри типа 2
        // спецификация запрещает, и защита от цикла тут именно эта.
        let location = self.xref.get(&number).copied();
        let Some(XrefLoc::Offset(offset)) = location else {
            return Err(pdf_err(format!(
                "объектный поток {number} не найден в таблице перекрёстных ссылок"
            )));
        };
        let PdfValue::Stream { dict, data } = self.parse_object_at(offset)? else {
            return Err(pdf_err(format!("объект {number} — не поток")));
        };
        let bytes = self.decode_stream(&dict, &data)?;
        let count = match dict_get(&dict, "N") {
            Some(PdfValue::Integer(n)) if *n >= 0 => usize::try_from(*n).unwrap_or(0),
            _ => return Err(pdf_err(format!("в объектном потоке {number} нет /N"))),
        };
        let first = match dict_get(&dict, "First") {
            Some(PdfValue::Integer(n)) if *n >= 0 => usize::try_from(*n).unwrap_or(usize::MAX),
            _ => return Err(pdf_err(format!("в объектном потоке {number} нет /First"))),
        };
        if first > bytes.len() {
            return Err(pdf_err(format!(
                "/First объектного потока {number} указывает за его конец"
            )));
        }
        let mut head = Lexer::new(&bytes, 0);
        let mut entries = Vec::with_capacity(count.min(4096));
        for _ in 0..count {
            let (Some(object), Some(offset)) = (head.unsigned(), head.unsigned()) else {
                return Err(pdf_err(format!(
                    "заголовок объектного потока {number} короче объявленного /N"
                )));
            };
            if head.pos > first {
                return Err(pdf_err(format!(
                    "заголовок объектного потока {number} заходит за /First"
                )));
            }
            let object = u32::try_from(object)
                .map_err(|_| pdf_err("слишком большой номер объекта в объектном потоке"))?;
            let offset = usize::try_from(offset).unwrap_or(usize::MAX);
            entries.push((object, first.saturating_add(offset)));
        }
        let objstm = std::rc::Rc::new(ObjStm {
            data: bytes,
            entries,
        });
        self.streams.insert(number, objstm.clone());
        Ok(objstm)
    }

    /// Снять с потока фильтры. Поддержан один — `FlateDecode`; всякий
    /// другой отвергается ПО ИМЕНИ, чтобы отказ был понятен.
    fn decode_stream(&mut self, dict: &[(String, PdfValue)], raw: &[u8]) -> RtResult<Vec<u8>> {
        let filters = match dict_get(dict, "Filter") {
            None | Some(PdfValue::Null) => Vec::new(),
            Some(PdfValue::Name(name)) => vec![name.clone()],
            Some(PdfValue::Array(items)) => {
                let mut out = Vec::with_capacity(items.len());
                for item in items {
                    match self.resolve(item)? {
                        PdfValue::Name(name) => out.push(name),
                        _ => return Err(pdf_err("в /Filter потока ожидались имена фильтров")),
                    }
                }
                out
            }
            Some(PdfValue::Ref(number)) => match self.object(*number)? {
                PdfValue::Name(name) => vec![name],
                PdfValue::Array(items) => {
                    let mut out = Vec::with_capacity(items.len());
                    for item in items {
                        match self.resolve(&item)? {
                            PdfValue::Name(name) => out.push(name),
                            _ => return Err(pdf_err("в /Filter потока ожидались имена фильтров")),
                        }
                    }
                    out
                }
                _ => return Err(pdf_err("/Filter потока — не имя и не массив имён")),
            },
            Some(_) => return Err(pdf_err("/Filter потока — не имя и не массив имён")),
        };
        let parms = match dict_get(dict, "DecodeParms").or_else(|| dict_get(dict, "DP")) {
            None => Vec::new(),
            Some(value) => match self.resolve(value)? {
                PdfValue::Null => Vec::new(),
                PdfValue::Dict(d) => vec![Some(d)],
                PdfValue::Array(items) => {
                    let mut out = Vec::with_capacity(items.len());
                    for item in items {
                        match self.resolve(&item)? {
                            PdfValue::Dict(d) => out.push(Some(d)),
                            _ => out.push(None),
                        }
                    }
                    out
                }
                _ => Vec::new(),
            },
        };
        let mut data = raw.to_vec();
        for (i, filter) in filters.iter().enumerate() {
            match filter.as_str() {
                "FlateDecode" | "Fl" => data = inflate_pdf_stream(&data)?,
                other => {
                    return Err(pdf_err(format!(
                        "фильтр потока «{other}» не поддерживается"
                    )))
                }
            }
            if let Some(Some(parm)) = parms.get(i) {
                let parm = parm.clone();
                data = self.apply_predictor(&parm, data)?;
            }
        }
        Ok(data)
    }

    /// Снять предиктор PNG (раздел 7.4.4.4). Предиктор TIFF (2) и всё
    /// неизвестное отвергаются с НОМЕРОМ в тексте ошибки.
    fn apply_predictor(
        &mut self,
        parms: &[(String, PdfValue)],
        data: Vec<u8>,
    ) -> RtResult<Vec<u8>> {
        let number = |key: &str, default: i64| -> RtResult<i64> {
            match dict_get(parms, key) {
                None | Some(PdfValue::Null) => Ok(default),
                Some(PdfValue::Integer(n)) => Ok(*n),
                Some(_) => Err(pdf_err(format!("/{key} предиктора — не целое"))),
            }
        };
        let predictor = number("Predictor", 1)?;
        if predictor <= 1 {
            return Ok(data);
        }
        if predictor < 10 {
            return Err(pdf_err(format!(
                "предиктор {predictor} не поддерживается: здесь снимаются только предикторы PNG (10..15)"
            )));
        }
        let colors = number("Colors", 1)?;
        let bits = number("BitsPerComponent", 8)?;
        let columns = number("Columns", 1)?;
        if colors <= 0 || bits <= 0 || columns <= 0 {
            return Err(pdf_err("параметры предиктора должны быть положительными"));
        }
        // Все три числа пришли из файла, поэтому цепочка проверена до
        // конца, вместе с округлением битов вверх до байтов: без этого
        // `+ 7` у `colors * bits` под `i64::MAX` переполняется само.
        let pixel_bits = colors
            .checked_mul(bits)
            .ok_or_else(|| pdf_err("слишком большая строка предиктора"))?;
        let row_bits = pixel_bits
            .checked_mul(columns)
            .ok_or_else(|| pdf_err("слишком большая строка предиктора"))?;
        let row_len = row_bits
            .checked_add(7)
            .and_then(|bits| usize::try_from(bits / 8).ok())
            .ok_or_else(|| pdf_err("слишком большая строка предиктора"))?;
        let step = pixel_bits
            .checked_add(7)
            .and_then(|bits| usize::try_from(bits / 8).ok())
            .ok_or_else(|| pdf_err("слишком большой шаг предиктора"))?
            .max(1);
        // Память выделяется по `row_len`, то есть по числу ИЗ ФАЙЛА, —
        // значит его нужно зажать самими данными. У правильного потока
        // каждая строка занимает в них `1 + row_len` байт, так что ни один
        // корректный файл этим не задет, а `data` после `inflate` уже
        // ограничена `MAX_STREAM_OUT`. Обрезанный до неполной строки поток
        // превращается из «строки, добитой нулями» в ошибку — так и надо.
        if data.is_empty() {
            return Ok(data);
        }
        if row_len > data.len() {
            return Err(pdf_err("строка предиктора длиннее самих данных"));
        }
        let mut out = Vec::with_capacity(data.len());
        let mut previous = vec![0u8; row_len];
        let mut at = 0usize;
        while at < data.len() {
            let kind = data[at];
            at += 1;
            let end = (at + row_len).min(data.len());
            let mut row = data[at..end].to_vec();
            row.resize(row_len, 0);
            at = end;
            for i in 0..row_len {
                let left = if i >= step { row[i - step] } else { 0 };
                let up = previous[i];
                let up_left = if i >= step { previous[i - step] } else { 0 };
                row[i] = match kind {
                    0 => row[i],
                    1 => row[i].wrapping_add(left),
                    2 => row[i].wrapping_add(up),
                    3 => row[i].wrapping_add(((u16::from(left) + u16::from(up)) / 2) as u8),
                    4 => row[i].wrapping_add(paeth(left, up, up_left)),
                    other => {
                        return Err(pdf_err(format!(
                            "неизвестный тип строки предиктора PNG: {other}"
                        )))
                    }
                };
            }
            out.extend_from_slice(&row);
            previous = row;
        }
        Ok(out)
    }

    /// Всё, что берётся из файла: дерево страниц `/Root` -> `/Pages` ->
    /// `/Kids`, дерево имён вложений и то, что понадобится инкрементальной
    /// записи.
    fn read_file(mut self) -> RtResult<PdfFile> {
        let root_value = match dict_get(&self.trailer, "Root").cloned() {
            Some(value) => value,
            None => return Err(pdf_err("в трейлере нет /Root")),
        };
        let catalog_number = match root_value {
            PdfValue::Ref(number) => Some(number),
            _ => None,
        };
        let PdfValue::Dict(root) = self.resolve(&root_value)? else {
            return Err(pdf_err("/Root указывает не на словарь каталога"));
        };
        let pages = match dict_get(&root, "Pages").cloned() {
            Some(value) => self.resolve(&value)?,
            None => return Err(pdf_err("в каталоге нет /Pages")),
        };
        let mut out = Vec::new();
        let mut seen = std::collections::HashSet::new();
        self.walk_pages(&pages, Inherited::default(), 0, &mut seen, &mut out)?;
        let attachments = self.read_attachments(&root)?;
        let names_dict = match dict_get(&root, "Names").cloned() {
            Some(value) => match self.resolve(&value)? {
                PdfValue::Dict(dict) => dict,
                _ => Vec::new(),
            },
            None => Vec::new(),
        };
        // Номер первого свободного объекта. `/Size` трейлера — это заявка
        // файла, таблица — то, что в нём есть на самом деле; берётся
        // максимум, потому что новый объект не должен наступить ни на одно
        // из двух даже во лгущем файле.
        let mut next_object = match dict_get(&self.trailer, "Size") {
            Some(PdfValue::Integer(size)) => u32::try_from(*size).unwrap_or(1),
            _ => 1,
        };
        for number in self.xref.keys() {
            next_object = next_object.max(number.saturating_add(1));
        }
        if let Some(number) = catalog_number {
            next_object = next_object.max(number.saturating_add(1));
        }
        Ok(PdfFile {
            pages: out,
            attachments,
            source: self.data.to_vec(),
            catalog_number,
            catalog: root,
            names_dict,
            startxref: self.startxref,
            next_object: next_object.max(1),
        })
    }

    /// Вложения: `/Root` -> `/Names` -> `/EmbeddedFiles` -> дерево имён.
    ///
    /// Одноимённые записи схлопываются, и побеждает ПОСЛЕДНЯЯ — измерено на
    /// файле с двумя записями `dup.txt`: платформа отдаёт одно вложение с
    /// содержимым второй.
    fn read_attachments(&mut self, root: &[(String, PdfValue)]) -> RtResult<Vec<PdfAttachment>> {
        let Some(names) = dict_get(root, "Names").cloned() else {
            return Ok(Vec::new());
        };
        let PdfValue::Dict(names) = self.resolve(&names)? else {
            return Ok(Vec::new());
        };
        let Some(embedded) = dict_get(&names, "EmbeddedFiles").cloned() else {
            return Ok(Vec::new());
        };
        let mut out: Vec<PdfAttachment> = Vec::new();
        let mut seen = std::collections::HashSet::new();
        self.walk_name_tree(&embedded, 0, &mut seen, &mut out)?;
        let mut unique: Vec<PdfAttachment> = Vec::with_capacity(out.len());
        for item in out {
            match unique.iter_mut().find(|kept| kept.name == item.name) {
                Some(kept) => *kept = item,
                None => unique.push(item),
            }
        }
        Ok(unique)
    }

    /// Обход дерева имён (раздел 7.9.6): узел либо промежуточный с
    /// `/Kids`, либо лист с `/Names`. `/Limits` не читается вовсе — он
    /// нужен для ДВОИЧНОГО поиска по дереву, а нам нужны все записи
    /// подряд, и доверять ему в чужом файле незачем.
    ///
    /// Вход враждебный, поэтому ограничений три: множество посещённых
    /// номеров объектов, предел глубины и предел числа записей.
    fn walk_name_tree(
        &mut self,
        node: &PdfValue,
        depth: usize,
        seen: &mut std::collections::HashSet<u32>,
        out: &mut Vec<PdfAttachment>,
    ) -> RtResult<()> {
        if depth > MAX_DEPTH {
            return Err(pdf_err("слишком глубокое дерево имён вложений"));
        }
        let node = match node {
            PdfValue::Ref(number) => {
                if !seen.insert(*number) {
                    return Err(pdf_err(format!(
                        "цикл в дереве имён вложений: объект {number} встретился дважды"
                    )));
                }
                self.object(*number)?
            }
            other => other.clone(),
        };
        let PdfValue::Dict(node) = node else {
            return Err(pdf_err("узел дерева имён вложений — не словарь"));
        };
        if let Some(kids) = dict_get(&node, "Kids").cloned() {
            let PdfValue::Array(kids) = self.resolve(&kids)? else {
                return Err(pdf_err("/Kids дерева имён вложений — не массив"));
            };
            for kid in kids {
                self.walk_name_tree(&kid, depth + 1, seen, out)?;
            }
            return Ok(());
        }
        let Some(names) = dict_get(&node, "Names").cloned() else {
            // Лист без `/Names` и без `/Kids` — пустое дерево имён, а не
            // поломка: так выглядит документ, у которого вложения удалили.
            return Ok(());
        };
        let PdfValue::Array(names) = self.resolve(&names)? else {
            return Err(pdf_err("/Names дерева имён вложений — не массив"));
        };
        if names.len() % 2 != 0 {
            return Err(pdf_err(
                "в /Names дерева имён вложений нечётное число элементов",
            ));
        }
        for pair in names.chunks(2) {
            if out.len() >= MAX_ATTACHMENTS {
                return Err(pdf_err("в дереве имён вложений слишком много записей"));
            }
            // Ключ дерева не читается: имя вложения платформа берёт из
            // самой файловой спецификации (измерено — запись с ключом
            // «a-key» и `/F (a-file.txt)` отдала «a-file.txt»).
            if let Some(attachment) = self.filespec(&pair[1])? {
                out.push(attachment);
            }
        }
        Ok(())
    }

    /// Одна файловая спецификация (раздел 7.11.3). `None` — запись, которую
    /// платформа молча пропускает: без `/EF`, с висящей ссылкой в нём или
    /// вовсе без имени.
    fn filespec(&mut self, value: &PdfValue) -> RtResult<Option<PdfAttachment>> {
        let PdfValue::Dict(spec) = self.resolve(value)? else {
            return Ok(None);
        };
        // `/UF` — текстовая строка (UTF-16BE с меткой или UTF-8), `/F` —
        // строка файловой спецификации; при обеих побеждает `/UF`. Всё
        // измерено: файл с `/F (b-f.txt)` и `/UF` отдал имя из `/UF`.
        let name = match self.string_value(&spec, "UF")? {
            Some(bytes) => decode_text_string(&bytes),
            None => match self.string_value(&spec, "F")? {
                Some(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
                None => return Ok(None),
            },
        };
        let relation = match dict_get(&spec, "AFRelationship").cloned() {
            Some(value) => match self.resolve(&value)? {
                PdfValue::Name(name) => PdfRelation::from_pdf_name(&name),
                _ => PdfRelation::Unspecified,
            },
            None => PdfRelation::Unspecified,
        };
        let Some(ef) = dict_get(&spec, "EF").cloned() else {
            return Ok(None);
        };
        let PdfValue::Dict(ef) = self.resolve(&ef)? else {
            return Ok(None);
        };
        // В `/EF` спецификация разрешает те же ключи, что и в самой
        // файловой спецификации; берётся первый попавшийся поток.
        let mut stream = None;
        for key in ["F", "UF", "DOS", "Mac", "Unix"] {
            let Some(value) = dict_get(&ef, key).cloned() else {
                continue;
            };
            if let PdfValue::Stream { dict, data } = self.resolve(&value)? {
                stream = Some((dict, data));
                break;
            }
        }
        let Some((dict, data)) = stream else {
            return Ok(None);
        };
        let content_type = match dict_get(&dict, "Subtype") {
            Some(PdfValue::Name(name)) => name.clone(),
            _ => String::new(),
        };
        // Фильтр, которого мы не умеем (`/LZWDecode`), НЕ должен уносить
        // весь документ: измерено, что платформа такой файл читает и
        // вложение из него показывает. Отдаются сырые байты потока —
        // единственное, что у нас есть; потерять вместе с ними страницы
        // и остальные вложения было бы заметно хуже.
        let data = match self.decode_stream(&dict, &data) {
            Ok(decoded) => decoded,
            Err(_) => data,
        };
        Ok(Some(PdfAttachment {
            name,
            content_type,
            relation,
            data,
        }))
    }

    /// Строковое значение словаря, в том числе через косвенную ссылку.
    fn string_value(
        &mut self,
        dict: &[(String, PdfValue)],
        key: &str,
    ) -> RtResult<Option<Vec<u8>>> {
        let Some(value) = dict_get(dict, key).cloned() else {
            return Ok(None);
        };
        match self.resolve(&value)? {
            PdfValue::Str(bytes) => Ok(Some(bytes)),
            _ => Ok(None),
        }
    }

    fn walk_pages(
        &mut self,
        node: &PdfValue,
        inherited: Inherited,
        depth: usize,
        seen: &mut std::collections::HashSet<u32>,
        out: &mut Vec<PdfPageInfo>,
    ) -> RtResult<()> {
        if depth > MAX_DEPTH {
            return Err(pdf_err("слишком глубокое дерево страниц"));
        }
        let node = match node {
            PdfValue::Ref(number) => {
                if !seen.insert(*number) {
                    return Err(pdf_err(format!(
                        "цикл в дереве страниц: объект {number} встретился дважды"
                    )));
                }
                self.object(*number)?
            }
            other => other.clone(),
        };
        let PdfValue::Dict(node) = node else {
            return Err(pdf_err("узел дерева страниц — не словарь"));
        };
        let inherited = Inherited {
            media: self.rect(&node, "MediaBox")?.or(inherited.media),
            crop: self.rect(&node, "CropBox")?.or(inherited.crop),
        };
        let kids = match dict_get(&node, "Kids").cloned() {
            Some(value) => self.resolve(&value)?,
            None => PdfValue::Null,
        };
        if let PdfValue::Array(kids) = kids {
            for kid in kids {
                self.walk_pages(&kid, inherited, depth + 1, seen, out)?;
            }
            return Ok(());
        }
        if out.len() >= MAX_PAGES {
            return Err(pdf_err("в дереве страниц слишком много листьев"));
        }
        // Лист. `/Type /Page` не проверяется: файлы от небрежных писателей
        // его теряют, а решает всё равно отсутствие `/Kids`.
        //
        // `/TrimBox` берётся ТОЛЬКО со самой страницы: он, как и `/Rotate`,
        // не наследуется (измерено — страница под узлом с `/TrimBox` отдала
        // нулевые поля).
        let trim = self.rect(&node, "TrimBox")?;
        let rotate = rotate_of(&node);
        let visible = visible_rect(inherited);
        out.push(PdfPageInfo {
            width_mm: pt_to_mm(extent(visible).0),
            height_mm: pt_to_mm(extent(visible).1),
            rotate,
            margins: margins_of(visible, trim),
        });
        Ok(())
    }

    /// Прямоугольник из словаря: четыре числа, каждое может быть косвенным.
    fn rect(&mut self, dict: &[(String, PdfValue)], key: &str) -> RtResult<Option<[f64; 4]>> {
        let Some(value) = dict_get(dict, key).cloned() else {
            return Ok(None);
        };
        let value = self.resolve(&value)?;
        let PdfValue::Array(items) = value else {
            return Ok(None);
        };
        if items.len() != 4 {
            return Ok(None);
        }
        let mut out = [0.0f64; 4];
        for (slot, item) in out.iter_mut().zip(items.iter()) {
            *slot = match self.resolve(item)? {
                PdfValue::Integer(n) => n as f64,
                PdfValue::Real(x) if x.is_finite() => x,
                _ => return Ok(None),
            };
        }
        Ok(Some(out))
    }
}

/// Наследуемые от узлов `/Pages` атрибуты страницы. `/Rotate` здесь нет
/// намеренно: платформа его НЕ наследует (измерено).
#[derive(Debug, Clone, Copy, Default)]
struct Inherited {
    media: Option<[f64; 4]>,
    crop: Option<[f64; 4]>,
}

/// Видимая рамка страницы в пунктах: `/CropBox`, пересечённый с
/// `/MediaBox`, либо один `/MediaBox`. Рамки нет вовсе — US Letter
/// (измерено: страница без `/MediaBox` отдала 216 на 279 мм).
fn visible_rect(inherited: Inherited) -> [f64; 4] {
    const LETTER: [f64; 4] = [0.0, 0.0, 612.0, 792.0];
    let media = inherited.media.unwrap_or(LETTER);
    match inherited.crop {
        Some(crop) => [
            crop[0].max(media[0]),
            crop[1].max(media[1]),
            crop[2].min(media[2]),
            crop[3].min(media[3]),
        ],
        None => media,
    }
}

/// Ширина и высота рамки в пунктах. Рамка без площади — ноль на ноль ПО
/// ОБЕИМ сторонам сразу, а не по одной: измерено на переставленной,
/// нулевой и отрицательной рамках.
fn extent(rect: [f64; 4]) -> (f64, f64) {
    let width = rect[2] - rect[0];
    let height = rect[3] - rect[1];
    // Условие написано положительно, чтобы NaN (разность бесконечностей)
    // отсеивался вместе с нулевой и отрицательной площадью.
    if width.is_finite() && height.is_finite() && width > 0.0 && height > 0.0 {
        (width, height)
    } else {
        (0.0, 0.0)
    }
}

/// Поля страницы в миллиметрах из `/TrimBox`.
///
/// ИЗМЕРЕНО на одиннадцати страницах, и правило вышло несимметричным:
///
/// * поле берётся ТОЛЬКО из `/TrimBox`; `/BleedBox` и `/ArtBox` не годятся
///   (страницы с одним из них отдали нули), а когда объявлены и `/TrimBox`,
///   и `/BleedBox`, побеждает `/TrimBox`;
/// * левое и ВЕРХНЕЕ поля — это АБСОЛЮТНЫЕ координаты левого нижнего угла
///   `/TrimBox`, а не отступы от `/MediaBox`: страница со смещённым началом
///   `[10 20 605.32 861.92]` и `/TrimBox [95.04 90.87 ...]` отдала 34 и 32,
///   а не 30 и 25;
/// * правое и НИЖНЕЕ — расстояния до дальних краёв ВИДИМОЙ рамки, то есть
///   `/CropBox`, если он есть: та же страница с `/CropBox` отдала 18 и 18;
/// * ось Y перевёрнута относительно PDF: «верхнее» поле отмеряется снизу
///   рамки, «нижнее» — сверху. Так платформа свои файлы и ПИШЕТ, поэтому
///   круг замыкается: `probe-margins.pdf`, записанный с полями 30/10/25/5,
///   читается обратно как 30/10/25/5;
/// * поля не поджимаются: `/TrimBox` шире `/MediaBox` даёт отрицательные
///   значения (-18, -37, -18, -20);
/// * `/TrimBox` НЕ наследуется от узлов `/Pages`.
fn margins_of(visible: [f64; 4], trim: Option<[f64; 4]>) -> [i64; 4] {
    let Some(trim) = trim else {
        return [0; 4];
    };
    [
        pt_to_mm(trim[0]),
        pt_to_mm(visible[2] - trim[2]),
        pt_to_mm(trim[1]),
        pt_to_mm(visible[3] - trim[3]),
    ]
}

/// Пункты в миллиметры с округлением к ближайшему — ровно так печатает
/// платформа (измерено: 100 пт -> 35, 200 пт -> 71, 100,62 -> 35,
/// 100,64 -> 36).
fn pt_to_mm(pt: f64) -> i64 {
    if !pt.is_finite() {
        return 0;
    }
    (pt * 25.4 / 72.0).round() as i64
}

/// `/Rotate` страницы, приведённый к кратному прямому углу.
///
/// Порядок действий измерен на десяти значениях: сначала остаток от
/// деления на 360, затем округление к ближайшему кратному 90, затем ещё
/// раз остаток (иначе 350 дало бы 360, а не 0).
fn rotate_of(page: &[(String, PdfValue)]) -> i64 {
    let raw = match dict_get(page, "Rotate") {
        Some(PdfValue::Integer(n)) => *n as f64,
        Some(PdfValue::Real(x)) if x.is_finite() => *x,
        _ => return 0,
    };
    let wrapped = raw % 360.0;
    let quarters = (wrapped / 90.0).round();
    if !quarters.is_finite() {
        return 0;
    }
    ((quarters as i64) * 90).rem_euclid(360)
}

fn paeth(left: u8, up: u8, up_left: u8) -> u8 {
    let p = i32::from(left) + i32::from(up) - i32::from(up_left);
    let (dl, du, dul) = (
        (p - i32::from(left)).abs(),
        (p - i32::from(up)).abs(),
        (p - i32::from(up_left)).abs(),
    );
    if dl <= du && dl <= dul {
        left
    } else if du <= dul {
        up
    } else {
        up_left
    }
}

/// Распаковать `FlateDecode`. Поток в PDF обёрнут по RFC 1950 (заголовок
/// zlib и adler32 в хвосте), но файлы от небрежных писателей встречаются и
/// с сырым deflate, поэтому обёртка распознаётся, а при неудаче делается
/// вторая попытка без неё.
fn inflate_pdf_stream(data: &[u8]) -> RtResult<Vec<u8>> {
    if data.len() >= 2 {
        let (cmf, flg) = (data[0], data[1]);
        let looks_zlib = cmf & 0x0F == 8 && (u16::from(cmf) * 256 + u16::from(flg)) % 31 == 0;
        if looks_zlib {
            if let Ok(out) = crate::inflate::inflate(&data[2..], MAX_STREAM_OUT) {
                return Ok(out);
            }
        }
    }
    crate::inflate::inflate(data, MAX_STREAM_OUT)
}

// ---------------------------------------------------------------------------
// Поверхность встроенного языка: `ДокументPDF`
// ---------------------------------------------------------------------------
//
// Всё, что здесь есть, снято с 8.3.27 скриптом
// `tests/conformance/measure/measure-pdf-read.bsl`; его вывод лежит рядом
// в `.platform.txt` и сверяется построчно.
//
// * `Новый ДокументPDF` — БЕЗ аргументов: и путь, и `ДвоичныеДанные` в
//   конструкторе платформа отвергает;
// * `Прочитать(ИмяФайла)` берёт только имя файла: `ДвоичныеДанные`
//   отвергнуты, вызов без аргументов — ошибка;
// * `Страницы` до первого чтения — `Неопределено`, и после НЕУДАЧНОГО
//   чтения снова `Неопределено`: документ возвращается в непрочитанное
//   состояние, а ошибку в нём даёт уже `Количество()`;
// * коллекция `КоллекцияСтраницPDF` умеет `Количество()`, `Получить(i)`,
//   `Индекс(Страница)`, `[i]` и `Для Каждого`; `Получить` вне диапазона
//   отдаёт `Неопределено` (и на 99, и на -1), а `[i]` — ошибку;
// * `КоличествоСтраниц` у документа НЕТ (ошибка);
// * страница только ЧИТАЕТСЯ: присваивание в `Ширина` — ошибка.
//
// Вложения сняты тем же способом (скрипт задачи —
// `tests/conformance/measure/measure-pdf-attachments.bsl`), и устроены они
// НЕ так, как страницы:
//
// * `Вложения` есть ВСЕГДА, в том числе до `Прочитать`: это пустая
//   `КоллекцияВложенийPDF`, а не `Неопределено`, и в неё уже можно
//   добавлять;
// * `Новый КоллекцияВложенийPDF` платформа строит, а `Новый ВложениеPDF` —
//   нет: вложение появляется только через `Добавить`;
// * коллекция умеет `Количество()`, `Получить(i)`, `[i]`, `Индекс(Влож)`,
//   `Найти(Имя)`, `Добавить(Имя, Данные[, ТипСодержимого[, ТипСвязи]])`,
//   `Удалить(i)`, `Очистить()` и `Для Каждого`; `Вставить` не пробовали, а
//   потому и не заявляем — здесь его нет до замера;
// * у вложения ровно четыре свойства — `ИмяФайла`, `ТипСодержимого`,
//   `Содержимое`, `ТипСвязи`, — и все ЧЕТЫРЕ ПИШУТСЯ (в отличие от
//   страницы, которая только читается);
// * `Записать(ИмяФайла[, Пароль])` у документа — процедура;
// * членов про ЭЛЕКТРОННУЮ ПОДПИСЬ у «ДокументPDF» на 8.3.27 НЕТ вовсе:
//   пробовано шесть написаний — чтения свойств `ЭлектронныеПодписи` и
//   `Подписи`, вызовы `ПолучитьЭлектронныеПодписи()`,
//   `ДобавитьЭлектроннуюПодпись()`, `ПроверитьЭлектроннуюПодпись()` и
//   `Подписать()`, — и все шесть кончаются исключением (текст ошибки
//   скрипт не печатает, только «нет» из ветки `Исключение`); четырёх типов
//   `ПодписьPDF`, `ЭлектроннаяПодписьPDF`, `PDFSignature` и
//   `КоллекцияПодписейPDF` платформа не знает. Поэтому здесь их тоже нет:
//   честная ошибка «нет такого метода» приходит сама, из общих таблиц, и
//   заводить ради неё пустую заглушку значило бы придумать платформе
//   поверхность, которой у неё нет.

/// Состояние `ДокументPDF`: разобранный файл либо ничего, если чтения ещё
/// не было или последнее чтение не удалось.
///
/// Вложения живут ОТДЕЛЬНО от разобранного файла, и это не украшение:
/// измерено, что `Вложения` у свежего документа — пустая коллекция, в
/// которую можно добавлять ещё до `Прочитать`. Коллекция и её элементы
/// держат тот же `Rc`, поэтому `Док.Вложения = Док.Вложения` — «Да»
/// (измерено), а `Прочитать` заменяет СОДЕРЖИМОЕ вектора, из-за чего уже
/// полученная коллекция видит новые вложения.
#[derive(Debug, Default)]
pub struct PdfDocState {
    file: Option<PdfFile>,
    attachments: Rc<RefCell<Vec<PdfAttachment>>>,
}

impl PdfDocState {
    /// Разобранный файл, если чтение было и удалось.
    pub fn file(&self) -> Option<&PdfFile> {
        self.file.as_ref()
    }
}

/// `Новый ДокументPDF` — пустой документ без источника.
pub fn new_pdf_document() -> BslValue {
    BslValue::Object(Rc::new(BslObject::PdfDocument(Rc::new(RefCell::new(
        PdfDocState::default(),
    )))))
}

/// `Новый КоллекцияВложенийPDF` — коллекция сама по себе, без документа.
///
/// Платформа такой конструктор ЗНАЕТ (измерено), хотя присоединить готовую
/// коллекцию к документу нечем: `Вложения` только читается. Значит, всё,
/// что с ней можно делать, — наполнять и разглядывать; ровно это здесь и
/// получается, потому что коллекция документа отличается от отдельной лишь
/// тем, кто ещё держит тот же `Rc`.
pub fn new_pdf_attachments() -> BslValue {
    BslValue::Object(Rc::new(BslObject::PdfAttachments(Rc::new(RefCell::new(
        Vec::new(),
    )))))
}

/// Состояние за значением любого из трёх видов — документа, коллекции и
/// страницы. У всех трёх оно ОБЩЕЕ: коллекция и страница — окна в тот же
/// документ, а не снимки.
fn doc_state<'a>(
    value: &'a BslValue,
    method: &'static str,
) -> RtResult<&'a Rc<RefCell<PdfDocState>>> {
    match value {
        BslValue::Object(o) => match &**o {
            BslObject::PdfDocument(state)
            | BslObject::PdfPages(state)
            | BslObject::PdfPage(state, _) => Ok(state),
            _ => Err(RtError::MethodNotApplicable {
                method,
                receiver: value.type_name(),
            }),
        },
        _ => Err(RtError::MethodNotApplicable {
            method,
            receiver: value.type_name(),
        }),
    }
}

/// Документ ли это (для развилок в общих таблицах методов).
pub fn is_pdf_document(value: &BslValue) -> bool {
    matches!(value, BslValue::Object(o) if matches!(&**o, BslObject::PdfDocument(_)))
}

/// Коллекция страниц ли это.
pub fn is_pdf_pages(value: &BslValue) -> bool {
    matches!(value, BslValue::Object(o) if matches!(&**o, BslObject::PdfPages(_)))
}

/// Коллекция вложений ли это.
pub fn is_pdf_attachments(value: &BslValue) -> bool {
    matches!(value, BslValue::Object(o) if matches!(&**o, BslObject::PdfAttachments(_)))
}

/// Вектор вложений за коллекцией или за самим вложением.
fn attachments_of<'a>(
    value: &'a BslValue,
    method: &'static str,
) -> RtResult<&'a Rc<RefCell<Vec<PdfAttachment>>>> {
    match value {
        BslValue::Object(o) => match &**o {
            BslObject::PdfAttachments(items) | BslObject::PdfAttachment(items, _) => Ok(items),
            _ => Err(RtError::MethodNotApplicable {
                method,
                receiver: value.type_name(),
            }),
        },
        _ => Err(RtError::MethodNotApplicable {
            method,
            receiver: value.type_name(),
        }),
    }
}

/// `ДокументPDF.Прочитать(ИмяФайла[, Пароль])`.
///
/// Пароль ПРИНИМАЕТСЯ И НЕ ИСПОЛЬЗУЕТСЯ, как у `Новый ЧтениеZipФайла`:
/// расшифровки здесь нет, и зашифрованный файл отвергается независимо от
/// того, назвали пароль или нет. Платформа с паролем такой файл читает
/// (измерено: `Прочитать(файл, "secret")` на RC4-40 отдало одну страницу),
/// и это единственное объявленное расхождение скрипта замеров чтения.
/// Симметричная граница на записи — в [`fn@write`].
///
/// Источником может быть ТОЛЬКО имя файла: `ДвоичныеДанные` платформа
/// отвергает (измерено). Принимает ли она ПОТОК, выяснить этой оснасткой
/// нельзя — на такой пробе платформа показывает модальное окно, то есть
/// немой таймаут, и в реестр открытых вопросов её не занести: одна строка
/// в `measure-all.bsl` унесла бы весь сеанс замеров. Поэтому здесь
/// принимается ровно то написание, которое измерено, — строка.
///
/// # Errors
///
/// [`RtError::Pdf`], если аргументов нет вовсе, если первым передано не имя
/// файла, если файл не читается с диска или не разбирается как PDF. При
/// любой ошибке документ возвращается в НЕПРОЧИТАННОЕ состояние — измерено,
/// что после неудачного чтения платформа снова отдаёт на `Страницы`
/// `Неопределено`, а ошибку даёт уже `Количество()` на нём. Вложения при
/// этом тоже забываются: измерено, что после неудачного чтения коллекция
/// пуста, а удачное чтение заменяет её содержимым нового файла.
pub fn read(obj: &BslValue, args: &[BslValue]) -> RtResult<()> {
    let state = doc_state(obj, "Прочитать")?;
    // Сначала забываем прежнее, потом читаем: иначе неудача оставила бы
    // документ с чужими страницами. Вложения забываются тем же движением —
    // уже полученная коллекция при этом остаётся той же самой, меняется
    // только её содержимое.
    {
        let mut state = state.borrow_mut();
        state.file = None;
        state.attachments.borrow_mut().clear();
    }
    let (Some(source), true) = (args.first(), args.len() <= 2) else {
        return Err(pdf_err(
            "ДокументPDF.Прочитать ожидает имя файла и необязательный пароль",
        ));
    };
    let BslValue::Str(name) = source else {
        return Err(pdf_err(format!(
            "ДокументPDF.Прочитать ожидает имя файла строкой, получено «{}»",
            source.type_name()
        )));
    };
    let name = name.to_string();
    let bytes = std::fs::read(&name)
        .map_err(|e| pdf_err(format!("не удалось прочитать файл «{name}»: {e}")))?;
    let file = PdfFile::parse(&bytes)?;
    {
        let mut state = state.borrow_mut();
        state.attachments.borrow_mut().clone_from(&file.attachments);
        state.file = Some(file);
    }
    Ok(())
}

/// `ДокументPDF.Записать(ИмяФайла[, Пароль])` — процедура (измерено:
/// обращение к ней как к функции платформа отвергает).
///
/// Пишется ИНКРЕМЕНТАЛЬНОЕ ОБНОВЛЕНИЕ поверх прочитанных байт, см.
/// [`PdfFile::write_with_attachments`]; поэтому документ, который ничего не
/// читал, записать нечем. Платформа в этом случае тоже отвечает ошибкой,
/// хотя и оставляет на диске файл с одной пустой страницей A4 — этого
/// последнего мы не делаем сознательно: пустую страницу неоткуда взять, а
/// придумать её значило бы записать не тот документ, который просили.
///
/// # Errors
///
/// [`RtError::Pdf`], если аргументов нет или их больше двух, если имя файла
/// не строка, если документ ещё ничего не прочитал, если задан непустой
/// пароль (шифрования здесь нет) или если файл не записался на диск.
pub fn write(obj: &BslValue, args: &[BslValue]) -> RtResult<()> {
    let state = doc_state(obj, "Записать")?;
    let (Some(target), true) = (args.first(), args.len() <= 2) else {
        return Err(pdf_err(
            "ДокументPDF.Записать ожидает имя файла и необязательный пароль",
        ));
    };
    let BslValue::Str(name) = target else {
        return Err(pdf_err(format!(
            "ДокументPDF.Записать ожидает имя файла строкой, получено «{}»",
            target.type_name()
        )));
    };
    // Пароль ПРИНИМАЕТСЯ, но работать с ним нечем: шифрования здесь нет ни
    // на чтении, ни на записи. Платформа на незашифрованном документе
    // отвечает на любой непустой пароль «Неверный пароль», то есть тоже
    // ошибкой.
    if let Some(password) = args.get(1) {
        let empty = match password {
            BslValue::Str(text) => text.len_utf16() == 0,
            BslValue::Undefined => true,
            _ => false,
        };
        if !empty {
            return Err(pdf_err(
                "ДокументPDF.Записать: шифрование PDF не поддерживается, пароль неприменим",
            ));
        }
    }
    let state = state.borrow();
    let file = state.file.as_ref().ok_or_else(|| {
        pdf_err("ДокументPDF.Записать: документ ничего не прочитал, записывать нечего")
    })?;
    let bytes = file.write_with_attachments(&state.attachments.borrow())?;
    let name = name.to_string();
    std::fs::write(&name, bytes)
        .map_err(|e| pdf_err(format!("не удалось записать файл «{name}»: {e}")))?;
    Ok(())
}

/// Свойство `ДокументPDF.Страницы` (`Pages`).
///
/// До первого чтения — и после НЕУДАЧНОГО чтения — свойство отдаёт
/// `Неопределено`, а не ошибку и не пустую коллекцию (измерено:
/// `ТипЗнч(Док.Страницы)` у свежего документа — «Не определено»). Ошибку
/// в этом состоянии даёт уже `Количество()`, потому что метода у
/// `Неопределено` нет.
///
/// # Errors
///
/// [`RtError::UnknownColumn`] на любом другом имени свойства: других
/// свойств у документа нет (измерено — `КоличествоСтраниц` платформа не
/// знает).
pub fn document_property(obj: &BslValue, name: &str) -> RtResult<BslValue> {
    if name.eq_ignore_ascii_case("Вложения") || name.eq_ignore_ascii_case("Attachments") {
        // В отличие от `Страницы`, коллекция вложений есть и до чтения:
        // измерено, что у свежего документа это `КоллекцияВложенийPDF` с
        // нулём элементов, а не `Неопределено`.
        let state = doc_state(obj, "Вложения")?;
        let items = state.borrow().attachments.clone();
        return Ok(BslValue::Object(Rc::new(BslObject::PdfAttachments(items))));
    }
    if !name.eq_ignore_ascii_case("Страницы") && !name.eq_ignore_ascii_case("Pages") {
        return Err(RtError::UnknownColumn(name.to_string()));
    }
    let state = doc_state(obj, "Страницы")?;
    if state.borrow().file.is_none() {
        return Ok(BslValue::Undefined);
    }
    Ok(BslValue::Object(Rc::new(BslObject::PdfPages(
        state.clone(),
    ))))
}

/// Число вложений — общий путь `Количество()` и `Для Каждого`.
///
/// # Errors
///
/// [`RtError::MethodNotApplicable`], если получатель — не коллекция
/// вложений и не вложение.
pub fn attachment_count(obj: &BslValue) -> RtResult<usize> {
    Ok(attachments_of(obj, "Количество")?.borrow().len())
}

/// `Вложения[Номер]` — вне диапазона ОШИБКА (измерено: `Вложения[99]`
/// платформа отвергает).
///
/// # Errors
///
/// [`RtError::IndexOutOfBounds`], если такого вложения нет.
pub fn attachment_at(obj: &BslValue, index: usize) -> RtResult<BslValue> {
    let items = attachments_of(obj, "Получить")?;
    let len = items.borrow().len();
    if index >= len {
        return Err(RtError::IndexOutOfBounds {
            index: index as i64,
            len,
        });
    }
    Ok(BslValue::Object(Rc::new(BslObject::PdfAttachment(
        items.clone(),
        index,
    ))))
}

/// `Вложения.Получить(Номер)` — вне диапазона `Неопределено`, и на 99, и
/// на -1 (измерено), ровно как у страниц.
///
/// # Errors
///
/// [`RtError::TypeError`], если номер не число.
pub fn attachment_get(obj: &BslValue, index: &BslValue) -> RtResult<BslValue> {
    let len = attachment_count(obj)?;
    let BslValue::Number(number) = index else {
        return Err(RtError::TypeError {
            expected: "Число",
            op: "КоллекцияВложенийPDF.Получить",
        });
    };
    let Some(number) = number.to_i64_exact() else {
        return Ok(BslValue::Undefined);
    };
    match usize::try_from(number) {
        Ok(i) if i < len => attachment_at(obj, i),
        _ => Ok(BslValue::Undefined),
    }
}

/// `Вложения.Индекс(Вложение)` — номер в этой же коллекции.
///
/// Строже, чем у страниц: чужое ЗНАЧЕНИЕ (число, массив) платформа
/// отвергает ошибкой типа, а не отдаёт -1 (измерено). Вложение из другой
/// коллекции — как раз тот случай, когда -1 законно.
///
/// # Errors
///
/// [`RtError::TypeError`], если аргумент не `ВложениеPDF`.
pub fn attachment_index_of(obj: &BslValue, item: &BslValue) -> RtResult<BslValue> {
    let items = attachments_of(obj, "Индекс")?;
    let len = items.borrow().len();
    let found = match item {
        BslValue::Object(o) => match &**o {
            BslObject::PdfAttachment(other, index) if Rc::ptr_eq(items, other) && *index < len => {
                *index as i64
            }
            BslObject::PdfAttachment(..) => -1,
            _ => {
                return Err(RtError::TypeError {
                    expected: "ВложениеPDF",
                    op: "КоллекцияВложенийPDF.Индекс",
                })
            }
        },
        _ => {
            return Err(RtError::TypeError {
                expected: "ВложениеPDF",
                op: "КоллекцияВложенийPDF.Индекс",
            })
        }
    };
    Ok(BslValue::Number(bsl_number::BslNumber::from_i64(found)))
}

/// Имя вложения из аргумента: строка как есть, число — своим
/// представлением, пустое имя — ошибка.
///
/// Так меряется платформа, и правило у `Найти` и `Добавить` одно:
/// `Найти(1)` отвечает `Неопределено` (то есть 1 стало именем «1» и не
/// нашлось), а `Найти("")` и `Добавить("", Данные)` — «Несоответствие
/// типов (параметр номер '1')».
fn attachment_name_arg(value: &BslValue, op: &'static str) -> RtResult<String> {
    let name = match value {
        BslValue::Str(text) => text.to_string(),
        // Целое число платформа принимает и превращает в имя (измерено:
        // после `ИмяФайла = 1` свойство отдаёт «1»). Дробное отвергается
        // здесь же: у него представление зависит от разделителя, и
        // придумывать его имени файла незачем.
        BslValue::Number(number) => match number.to_i64_exact() {
            Some(number) => number.to_string(),
            None => {
                return Err(RtError::TypeError {
                    expected: "Строка",
                    op,
                })
            }
        },
        _ => {
            return Err(RtError::TypeError {
                expected: "Строка",
                op,
            })
        }
    };
    if name.is_empty() {
        return Err(RtError::TypeError {
            expected: "непустое имя файла",
            op,
        });
    }
    Ok(name)
}

/// `Вложения.Найти(Имя)` — вложение с таким `ИмяФайла` либо
/// `Неопределено`. Аргумент ровно один (измерено).
///
/// # Errors
///
/// [`RtError::TypeError`], если имя пустое или не приводится к строке.
pub fn attachment_find(obj: &BslValue, args: &[BslValue]) -> RtResult<BslValue> {
    let items = attachments_of(obj, "Найти")?;
    let [name] = args else {
        return Err(pdf_err(
            "КоллекцияВложенийPDF.Найти ожидает ровно одно имя файла",
        ));
    };
    // Нестроковое и нечисловое значение платформа не бракует, а просто не
    // находит (измерено на `Найти(Новый Массив)` — «Не определено»).
    let name = match name {
        BslValue::Str(_) | BslValue::Number(_) => {
            attachment_name_arg(name, "КоллекцияВложенийPDF.Найти")?
        }
        _ => return Ok(BslValue::Undefined),
    };
    let found = items.borrow().iter().position(|item| item.name == name);
    match found {
        Some(index) => Ok(BslValue::Object(Rc::new(BslObject::PdfAttachment(
            items.clone(),
            index,
        )))),
        None => Ok(BslValue::Undefined),
    }
}

/// `Вложения.Добавить(Имя, Данные[, ТипСодержимого[, ТипСвязи]])` —
/// процедура.
///
/// Одноимённое вложение СНИМАЕТСЯ, а новое дописывается В КОНЕЦ — то
/// есть коллекция ведёт себя как дерево имён, которым она и станет при
/// записи. Измерено: после `Добавить("а")`, `Добавить("б")` и повторного
/// `Добавить("а")` вложений двое, и порядок «б», «а», причём у «а» новые
/// тип содержимого и данные.
///
/// # Errors
///
/// [`RtError::Pdf`], если аргументов меньше двух или больше четырёх;
/// [`RtError::TypeError`], если имя пустое, если данные не
/// `ДвоичныеДанные`, если тип содержимого не строка или связь — не член
/// `ТипСвязиВложенияPDF`.
pub fn attachment_add(obj: &BslValue, args: &[BslValue]) -> RtResult<()> {
    let items = attachments_of(obj, "Добавить")?;
    if args.len() < 2 || args.len() > 4 {
        return Err(pdf_err(
            "КоллекцияВложенийPDF.Добавить ожидает имя файла, данные и \
             необязательные тип содержимого и тип связи",
        ));
    }
    let name = attachment_name_arg(&args[0], "КоллекцияВложенийPDF.Добавить")?;
    let data = match &args[1] {
        BslValue::Object(o) => match &**o {
            BslObject::BinaryData(bytes) => bytes.to_vec(),
            _ => {
                return Err(RtError::TypeError {
                    expected: "ДвоичныеДанные",
                    op: "КоллекцияВложенийPDF.Добавить",
                })
            }
        },
        _ => {
            return Err(RtError::TypeError {
                expected: "ДвоичныеДанные",
                op: "КоллекцияВложенийPDF.Добавить",
            })
        }
    };
    let content_type = match args.get(2) {
        None | Some(BslValue::Undefined) => String::new(),
        Some(BslValue::Str(text)) => text.to_string(),
        Some(_) => {
            return Err(RtError::TypeError {
                expected: "Строка",
                op: "КоллекцияВложенийPDF.Добавить",
            })
        }
    };
    let relation = match args.get(3) {
        None => PdfRelation::Unspecified,
        Some(value) => relation_of(value, "КоллекцияВложенийPDF.Добавить")?,
    };
    let mut items = items.borrow_mut();
    let fresh = PdfAttachment {
        name,
        content_type,
        relation,
        data,
    };
    items.retain(|item| item.name != fresh.name);
    items.push(fresh);
    Ok(())
}

/// Член перечисления `ТипСвязиВложенияPDF` из значения языка.
fn relation_of(value: &BslValue, op: &'static str) -> RtResult<PdfRelation> {
    let BslValue::Enum(member) = value else {
        return Err(RtError::TypeError {
            expected: "ТипСвязиВложенияPDF",
            op,
        });
    };
    match member {
        crate::enums::EnumValue::PdfRelationSource => Ok(PdfRelation::Source),
        crate::enums::EnumValue::PdfRelationData => Ok(PdfRelation::Data),
        crate::enums::EnumValue::PdfRelationAlternative => Ok(PdfRelation::Alternative),
        crate::enums::EnumValue::PdfRelationSupplement => Ok(PdfRelation::Supplement),
        crate::enums::EnumValue::PdfRelationUnspecified => Ok(PdfRelation::Unspecified),
        _ => Err(RtError::TypeError {
            expected: "ТипСвязиВложенияPDF",
            op,
        }),
    }
}

/// Член перечисления по связи — обратное [`relation_of`].
fn relation_enum(relation: PdfRelation) -> crate::enums::EnumValue {
    match relation {
        PdfRelation::Source => crate::enums::EnumValue::PdfRelationSource,
        PdfRelation::Data => crate::enums::EnumValue::PdfRelationData,
        PdfRelation::Alternative => crate::enums::EnumValue::PdfRelationAlternative,
        PdfRelation::Supplement => crate::enums::EnumValue::PdfRelationSupplement,
        PdfRelation::Unspecified => crate::enums::EnumValue::PdfRelationUnspecified,
    }
}

/// `Вложения.Удалить(Номер)` либо `Удалить(Вложение)`.
///
/// Номер ВНЕ ДИАПАЗОНА не ошибка: измерено, что и `Удалить(99)`, и
/// `Удалить(-1)` удаляют ПОСЛЕДНЕЕ вложение, а не жалуются.
///
/// Что делает платформа с ПУСТОЙ коллекцией, этой оснасткой не выяснить:
/// `Удалить(0)` на пустой она встречает модальным окном, то есть немым
/// таймаутом, и `Попытка` его не ловит (проверено отдельной пробой —
/// вывод обрывается ровно перед вызовом). Строке в реестре открытых
/// вопросов там не место: она унесла бы весь сеанс замеров. Здесь пустая
/// коллекция остаётся пустой — это единственное продолжение правила
/// «номер вне диапазона удаляет последнее» на случай, когда последнего
/// нет.
///
/// # Errors
///
/// [`RtError::Pdf`], если аргумент не один;
/// [`RtError::TypeError`], если это не число и не `ВложениеPDF`.
pub fn attachment_delete(obj: &BslValue, args: &[BslValue]) -> RtResult<()> {
    let items = attachments_of(obj, "Удалить")?;
    let [what] = args else {
        return Err(pdf_err(
            "КоллекцияВложенийPDF.Удалить ожидает ровно один аргумент",
        ));
    };
    let len = items.borrow().len();
    if len == 0 {
        return Ok(());
    }
    let index = match what {
        BslValue::Number(number) => match number.to_i64_exact() {
            Some(number) => match usize::try_from(number) {
                Ok(index) if index < len => index,
                _ => len - 1,
            },
            None => len - 1,
        },
        BslValue::Object(o) => match &**o {
            BslObject::PdfAttachment(other, index) if Rc::ptr_eq(items, other) && *index < len => {
                *index
            }
            BslObject::PdfAttachment(..) => {
                return Err(RtError::TypeError {
                    expected: "ВложениеPDF этой же коллекции",
                    op: "КоллекцияВложенийPDF.Удалить",
                })
            }
            _ => {
                return Err(RtError::TypeError {
                    expected: "Число или ВложениеPDF",
                    op: "КоллекцияВложенийPDF.Удалить",
                })
            }
        },
        _ => {
            return Err(RtError::TypeError {
                expected: "Число или ВложениеPDF",
                op: "КоллекцияВложенийPDF.Удалить",
            })
        }
    };
    items.borrow_mut().remove(index);
    Ok(())
}

/// `Вложения.Очистить()` — аргументов не берёт (измерено).
///
/// # Errors
///
/// [`RtError::MethodNotApplicable`], если получатель не коллекция вложений.
pub fn attachment_clear(obj: &BslValue) -> RtResult<()> {
    attachments_of(obj, "Очистить")?.borrow_mut().clear();
    Ok(())
}

/// Свойства `ВложениеPDF`. Все четыре имени и все четыре английских
/// синонима измерены: `FileName`, `MIMEType`, `Content` и
/// `RelationshipType`. Перебором проверено и то, чего у платформы НЕТ:
/// `ContentType`, `MediaType`, `Mime`, `Relationship`, `AFRelationship`,
/// `Relation`, `Имя`, `Описание`, `Размер` и `Данные`.
///
/// # Errors
///
/// [`RtError::Pdf`], если вложение уже удалено из коллекции;
/// [`RtError::UnknownColumn`] на неизвестном имени.
pub fn attachment_property(obj: &BslValue, name: &str) -> RtResult<BslValue> {
    let (items, index) = attachment_slot(obj, name)?;
    let items = items.borrow();
    let item = items
        .get(index)
        .ok_or_else(|| pdf_err("вложение уже удалено из коллекции"))?;
    if name.eq_ignore_ascii_case("ИмяФайла") || name.eq_ignore_ascii_case("FileName") {
        return Ok(BslValue::Str(crate::BslString::from_str(&item.name)));
    }
    if name.eq_ignore_ascii_case("ТипСодержимого") || name.eq_ignore_ascii_case("MIMEType")
    {
        return Ok(BslValue::Str(crate::BslString::from_str(
            &item.content_type,
        )));
    }
    if name.eq_ignore_ascii_case("Содержимое") || name.eq_ignore_ascii_case("Content") {
        return Ok(BslValue::binary_data_of(item.data.clone()));
    }
    if name.eq_ignore_ascii_case("ТипСвязи") || name.eq_ignore_ascii_case("RelationshipType")
    {
        return Ok(BslValue::Enum(relation_enum(item.relation)));
    }
    Err(RtError::UnknownColumn(name.to_string()))
}

/// Присваивание в свойство `ВложениеPDF`. Пишутся ВСЕ ЧЕТЫРЕ (измерено —
/// в отличие от страницы, которая только читается).
///
/// # Errors
///
/// [`RtError::Pdf`], если вложение уже удалено из коллекции;
/// [`RtError::TypeError`] на значении не того типа;
/// [`RtError::UnknownColumn`] на неизвестном имени.
pub fn set_attachment_property(obj: &BslValue, name: &str, value: &BslValue) -> RtResult<()> {
    let (items, index) = attachment_slot(obj, name)?;
    let mut items = items.borrow_mut();
    let item = items
        .get_mut(index)
        .ok_or_else(|| pdf_err("вложение уже удалено из коллекции"))?;
    if name.eq_ignore_ascii_case("ИмяФайла") || name.eq_ignore_ascii_case("FileName") {
        // Число платформа принимает и превращает в строку (измерено:
        // после `ИмяФайла = 1` свойство отдаёт «1»).
        item.name = attachment_name_arg(value, "ВложениеPDF.ИмяФайла")?;
        return Ok(());
    }
    if name.eq_ignore_ascii_case("ТипСодержимого") || name.eq_ignore_ascii_case("MIMEType")
    {
        let BslValue::Str(text) = value else {
            return Err(RtError::TypeError {
                expected: "Строка",
                op: "ВложениеPDF.ТипСодержимого",
            });
        };
        item.content_type = text.to_string();
        return Ok(());
    }
    if name.eq_ignore_ascii_case("Содержимое") || name.eq_ignore_ascii_case("Content") {
        let BslValue::Object(o) = value else {
            return Err(RtError::TypeError {
                expected: "ДвоичныеДанные",
                op: "ВложениеPDF.Содержимое",
            });
        };
        let BslObject::BinaryData(bytes) = &**o else {
            return Err(RtError::TypeError {
                expected: "ДвоичныеДанные",
                op: "ВложениеPDF.Содержимое",
            });
        };
        item.data = bytes.to_vec();
        return Ok(());
    }
    if name.eq_ignore_ascii_case("ТипСвязи") || name.eq_ignore_ascii_case("RelationshipType")
    {
        item.relation = relation_of(value, "ВложениеPDF.ТипСвязи")?;
        return Ok(());
    }
    Err(RtError::UnknownColumn(name.to_string()))
}

/// Вектор и номер за значением `ВложениеPDF`.
fn attachment_slot<'a>(
    obj: &'a BslValue,
    name: &str,
) -> RtResult<(&'a Rc<RefCell<Vec<PdfAttachment>>>, usize)> {
    let BslValue::Object(o) = obj else {
        return Err(RtError::UnknownColumn(name.to_string()));
    };
    let BslObject::PdfAttachment(items, index) = &**o else {
        return Err(RtError::UnknownColumn(name.to_string()));
    };
    Ok((items, *index))
}

/// Число страниц — общий путь `Количество()` и `Для Каждого`.
///
/// # Errors
///
/// [`RtError::Pdf`], если документ ничего не прочитал.
pub fn page_count(obj: &BslValue) -> RtResult<usize> {
    let state = doc_state(obj, "Количество")?;
    let state = state.borrow();
    let file = state
        .file
        .as_ref()
        .ok_or_else(|| pdf_err("у ДокументPDF ещё нет страниц: сначала нужен Прочитать"))?;
    Ok(file.page_count())
}

/// `Страницы.Получить(Номер)` — вне диапазона отдаёт `Неопределено`, и на
/// 99, и на -1 (измерено). Этим он и отличается от `Страницы[Номер]`.
///
/// # Errors
///
/// [`RtError::Pdf`], если документ ничего не прочитал;
/// [`RtError::TypeError`], если номер не число.
pub fn page_get(obj: &BslValue, index: &BslValue) -> RtResult<BslValue> {
    let count = page_count(obj)?;
    let BslValue::Number(number) = index else {
        return Err(RtError::TypeError {
            expected: "Число",
            op: "КоллекцияСтраницPDF.Получить",
        });
    };
    let Some(number) = number.to_i64_exact() else {
        return Ok(BslValue::Undefined);
    };
    match usize::try_from(number) {
        Ok(i) if i < count => page_at(obj, i),
        _ => Ok(BslValue::Undefined),
    }
}

/// `Страницы[Номер]` — вне диапазона ОШИБКА (измерено: `Страницы[-1]`
/// платформа отвергает, хотя `Получить(-1)` отдаёт `Неопределено`).
///
/// # Errors
///
/// [`RtError::Pdf`], если документ ничего не прочитал;
/// [`RtError::IndexOutOfBounds`], если такой страницы нет.
pub fn page_at(obj: &BslValue, index: usize) -> RtResult<BslValue> {
    let count = page_count(obj)?;
    if index >= count {
        return Err(RtError::IndexOutOfBounds {
            index: index as i64,
            len: count,
        });
    }
    let state = doc_state(obj, "Получить")?;
    Ok(BslValue::Object(Rc::new(BslObject::PdfPage(
        state.clone(),
        index,
    ))))
}

/// `Страницы.Индекс(Страница)` — номер страницы в этой же коллекции, и
/// `-1`, если страница чужая. Номер измерен: `Индекс(Страницы[1])` — 1.
///
/// # Errors
///
/// [`RtError::Pdf`], если документ ничего не прочитал.
pub fn page_index_of(obj: &BslValue, page: &BslValue) -> RtResult<BslValue> {
    let count = page_count(obj)?;
    let state = doc_state(obj, "Индекс")?;
    let found = match page {
        BslValue::Object(o) => match &**o {
            BslObject::PdfPage(other, index) if Rc::ptr_eq(state, other) && *index < count => {
                *index as i64
            }
            _ => -1,
        },
        _ => -1,
    };
    Ok(BslValue::Number(bsl_number::BslNumber::from_i64(found)))
}

/// Свойства `СтраницаPDF`. Все восемь имён и оба языка измерены.
///
/// # Errors
///
/// [`RtError::Pdf`], если документ успел забыть прочитанное;
/// [`RtError::UnknownColumn`] на неизвестном имени.
pub fn page_property(obj: &BslValue, name: &str) -> RtResult<BslValue> {
    let BslValue::Object(o) = obj else {
        return Err(RtError::UnknownColumn(name.to_string()));
    };
    let BslObject::PdfPage(state, index) = &**o else {
        return Err(RtError::UnknownColumn(name.to_string()));
    };
    let number = |value: i64| Ok(BslValue::Number(bsl_number::BslNumber::from_i64(value)));
    let state = state.borrow();
    let page = state
        .file
        .as_ref()
        .and_then(|file| file.page(*index))
        .ok_or_else(|| pdf_err("страница относится к документу, который уже перечитан"))?;
    if name.eq_ignore_ascii_case("Номер") || name.eq_ignore_ascii_case("Number") {
        // Номер СТРАНИЦЫ с единицы, в отличие от номера в коллекции
        // (измерено: `Страницы[0].Номер` — 1).
        return number(*index as i64 + 1);
    }
    if name.eq_ignore_ascii_case("Ширина") || name.eq_ignore_ascii_case("Width") {
        return number(page.width_mm());
    }
    if name.eq_ignore_ascii_case("Высота") || name.eq_ignore_ascii_case("Height") {
        return number(page.height_mm());
    }
    if name.eq_ignore_ascii_case("Ориентация") || name.eq_ignore_ascii_case("Orientation")
    {
        // Ориентация — ЧИСЛО, а не член перечисления (измерено:
        // `ТипЗнч(Страницы[0].Ориентация)` — «Число»).
        return number(page.rotate());
    }
    // Поля страницы приходят из `/TrimBox`; правило — в `margins_of`.
    for (ru, en, which) in [
        ("ПолеСлева", "LeftMargin", PdfMargin::Left),
        ("ПолеСправа", "RightMargin", PdfMargin::Right),
        ("ПолеСверху", "TopMargin", PdfMargin::Top),
        ("ПолеСнизу", "BottomMargin", PdfMargin::Bottom),
    ] {
        if name.eq_ignore_ascii_case(ru) || name.eq_ignore_ascii_case(en) {
            return number(page.margin(which));
        }
    }
    Err(RtError::UnknownColumn(name.to_string()))
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

    // --- чтение контейнера --------------------------------------------

    /// Собрать классический файл из готовых тел объектов: заголовок,
    /// объекты подряд, таблица `xref` и трейлер. Ровно то, что делает
    /// [`PdfDocument::write`], только без содержимого страниц — здесь
    /// проверяется КОНТЕЙНЕР.
    fn build_classic(objects: &[(u32, Vec<u8>)], trailer_extra: &str) -> Vec<u8> {
        let mut out = b"%PDF-1.4\n%\xe2\xe3\xcf\xd3\n".to_vec();
        let size = objects.iter().map(|(n, _)| *n).max().unwrap_or(0) + 1;
        let mut offsets = vec![0usize; size as usize];
        for (number, body) in objects {
            offsets[*number as usize] = out.len();
            out.extend_from_slice(format!("{number} 0 obj\n").as_bytes());
            out.extend_from_slice(body);
            out.extend_from_slice(b"\nendobj\n");
        }
        let xref = out.len();
        out.extend_from_slice(format!("xref\n0 {size}\n0000000000 65535 f \n").as_bytes());
        for offset in offsets.iter().skip(1) {
            out.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
        }
        out.extend_from_slice(
            format!("trailer\n<< /Size {size} /Root 1 0 R{trailer_extra} >>\nstartxref\n{xref}\n%%EOF\n")
                .as_bytes(),
        );
        out
    }

    fn empty_content() -> Vec<u8> {
        b"<< /Length 0 >>\nstream\n\nendstream".to_vec()
    }

    /// Однолистовой файл с заданным словарём страницы.
    fn one_page(page: &str) -> Vec<u8> {
        build_classic(
            &[
                (1, b"<< /Type /Catalog /Pages 2 0 R >>".to_vec()),
                (2, b"<< /Type /Pages /Kids [ 3 0 R ] /Count 1 >>".to_vec()),
                (3, page.as_bytes().to_vec()),
                (4, empty_content()),
            ],
            "",
        )
    }

    /// Собственный вывод писателя обязан читаться собственным читателем:
    /// число страниц и их размеры сходятся с тем, что было задано.
    #[test]
    fn pdf_reader_reads_the_writer_output() {
        let mut doc = PdfDocument::new();
        let first = doc.add_page(595.32, 841.92).unwrap();
        doc.text(first, 40.0, 800.0, PdfFont::Courier, 12.0, "Накладная № 7")
            .unwrap();
        let second = doc.add_page(841.92, 595.32).unwrap();
        doc.rect(second, 10.0, 10.0, 100.0, 50.0, PaintMode::Stroke)
            .unwrap();
        let bytes = doc.write().unwrap();

        let file = PdfFile::parse(&bytes).expect("свой же вывод обязан читаться");
        assert_eq!(file.page_count(), 2);
        assert_eq!(file.page(0).unwrap().width_mm(), 210);
        assert_eq!(file.page(0).unwrap().height_mm(), 297);
        assert_eq!(file.page(1).unwrap().width_mm(), 297);
        assert_eq!(file.page(1).unwrap().height_mm(), 210);
        assert_eq!(file.page(0).unwrap().rotate(), 0);
    }

    /// ВСЕ закоммиченные файлы, снятые с платформы, обязаны читаться, и
    /// число страниц в каждом — совпасть с тем, ради чего файл снимали
    /// (происхождение всех — `tests/conformance/pdf/capture-platform-pdf*.bsl`).
    #[test]
    fn pdf_reader_reads_every_committed_platform_file() {
        let expected: &[(&str, usize)] = &[
            // Три файла задачи о вложениях: снятый с платформы и два
            // собранных НАШИМ писателем поверх чужой основы (см.
            // `make-open-bsl-attachments.bsl`).
            ("attach-platform.pdf", 1),
            ("attach-open-bsl.pdf", 1),
            ("attach-open-bsl-xrefstream.pdf", 2),
            ("platform-simple.pdf", 1),
            ("probe-align.pdf", 1),
            ("probe-big.pdf", 4),
            ("probe-border.pdf", 1),
            ("probe-color.pdf", 1),
            ("probe-colwidth.pdf", 1),
            ("probe-empty.pdf", 1),
            ("probe-fit.pdf", 1),
            ("probe-font.pdf", 1),
            ("probe-grid.pdf", 1),
            ("probe-landscape.pdf", 1),
            ("probe-line.pdf", 1),
            ("probe-margins.pdf", 1),
            ("probe-merge.pdf", 1),
            ("probe-number.pdf", 1),
            ("probe-numeric.pdf", 1),
            ("probe-pages.pdf", 2),
            ("probe-rowheight.pdf", 1),
            ("probe-wide.pdf", 2),
        ];
        let dir = PathBuf::from("../../tests/conformance/pdf");
        let mut checked = 0usize;
        for (name, pages) in expected {
            let bytes = std::fs::read(dir.join(name)).unwrap_or_else(|e| {
                panic!("файл {name} обязан лежать в дереве: {e}");
            });
            let file = PdfFile::parse(&bytes)
                .unwrap_or_else(|e| panic!("платформенный файл {name} обязан читаться: {e:?}"));
            assert_eq!(file.page_count(), *pages, "число страниц в {name}");
            for i in 0..file.page_count() {
                let page = file.page(i).unwrap();
                assert!(
                    page.width_mm() > 0 && page.height_mm() > 0,
                    "страница {i} файла {name} без размера"
                );
            }
            checked += 1;
        }
        // Ни один файл каталога не должен остаться неучтённым: новый
        // снимок с платформы обязан попасть в таблицу выше.
        let on_disk = std::fs::read_dir(&dir)
            .expect("каталог со снимками обязан существовать")
            .filter_map(Result::ok)
            .filter(|e| e.path().extension().is_some_and(|x| x == "pdf"))
            .count();
        assert_eq!(checked, on_disk, "в каталоге появился неучтённый PDF");
    }

    /// Альбомный лист платформы: `probe-landscape.pdf` снят с документа,
    /// у которого `ОриентацияСтраницы = Ландшафт`.
    #[test]
    fn pdf_reader_sees_the_platform_landscape_page() {
        let bytes = std::fs::read("../../tests/conformance/pdf/probe-landscape.pdf").unwrap();
        let file = PdfFile::parse(&bytes).unwrap();
        let page = file.page(0).unwrap();
        assert_eq!((page.width_mm(), page.height_mm()), (297, 210));
    }

    /// Размеры отдаются в миллиметрах с округлением к ближайшему, начало
    /// рамки не важно (измерено, см. обзор модуля).
    #[test]
    fn pdf_reader_converts_points_to_millimetres() {
        let cases: &[(&str, i64, i64)] = &[
            ("[ 0 0 100 200 ]", 35, 71),
            ("[ 10 20 110 220 ]", 35, 71),
            ("[ 0 0 100.62 200 ]", 35, 71),
            ("[ 0 0 100.64 200 ]", 36, 71),
            ("[ 0 0 595.32 841.92 ]", 210, 297),
        ];
        for (rect, width, height) in cases {
            let pdf = one_page(&format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox {rect} /Contents 4 0 R >>"
            ));
            let file = PdfFile::parse(&pdf).unwrap();
            let page = file.page(0).unwrap();
            assert_eq!(
                (page.width_mm(), page.height_mm()),
                (*width, *height),
                "рамка {rect}"
            );
        }
    }

    /// Рамка без площади — ноль на ноль по обеим сторонам сразу, а страница
    /// без рамки вовсе — US Letter (измерено).
    #[test]
    fn pdf_reader_gives_zero_for_degenerate_boxes() {
        for rect in ["[ 595.32 0 0 841.92 ]", "[ 0 0 0 0 ]", "[ 0 0 -100 -200 ]"] {
            let pdf = one_page(&format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox {rect} /Contents 4 0 R >>"
            ));
            let file = PdfFile::parse(&pdf).unwrap();
            let page = file.page(0).unwrap();
            assert_eq!((page.width_mm(), page.height_mm()), (0, 0), "рамка {rect}");
        }
        let pdf = one_page("<< /Type /Page /Parent 2 0 R /Contents 4 0 R >>");
        let file = PdfFile::parse(&pdf).unwrap();
        let page = file.page(0).unwrap();
        assert_eq!((page.width_mm(), page.height_mm()), (216, 279));
    }

    /// `/CropBox` задаёт видимую область и ПЕРЕСЕКАЕТСЯ с `/MediaBox`;
    /// обе рамки наследуются от узла `/Pages`, а `/Rotate` — нет
    /// (измерено).
    #[test]
    fn pdf_reader_inherits_boxes_but_not_rotation() {
        let pdf = build_classic(
            &[
                (1, b"<< /Type /Catalog /Pages 2 0 R >>".to_vec()),
                (
                    2,
                    b"<< /Type /Pages /Kids [ 3 0 R 4 0 R 5 0 R ] /Count 3 \
                       /MediaBox [ 0 0 595.32 841.92 ] /CropBox [ 50 60 545.32 781.92 ] \
                       /Rotate 90 >>"
                        .to_vec(),
                ),
                (
                    3,
                    b"<< /Type /Page /Parent 2 0 R /Contents 6 0 R >>".to_vec(),
                ),
                (
                    4,
                    b"<< /Type /Page /Parent 2 0 R /MediaBox [ 0 0 595.32 841.92 ] \
                       /CropBox [ -100 -100 700 900 ] /Contents 6 0 R >>"
                        .to_vec(),
                ),
                (
                    5,
                    b"<< /Type /Page /Parent 2 0 R /MediaBox [ 0 0 595.32 841.92 ] \
                       /Rotate 180 /Contents 6 0 R >>"
                        .to_vec(),
                ),
                (6, empty_content()),
            ],
            "",
        );
        let file = PdfFile::parse(&pdf).unwrap();
        assert_eq!(file.page_count(), 3);
        // Унаследованный `/CropBox`.
        let first = file.page(0).unwrap();
        assert_eq!((first.width_mm(), first.height_mm()), (175, 255));
        // Наследуется, но `/Rotate` узла на страницу НЕ переходит.
        assert_eq!(first.rotate(), 0);
        // `/CropBox` шире `/MediaBox` — берётся пересечение.
        let second = file.page(1).unwrap();
        assert_eq!((second.width_mm(), second.height_mm()), (210, 297));
        // Собственный `/Rotate` страницы виден.
        assert_eq!(file.page(2).unwrap().rotate(), 180);
    }

    /// `/Rotate` приводится к кратному прямому углу — все десять значений,
    /// снятых с платформы.
    #[test]
    fn pdf_reader_normalises_rotation_like_the_platform() {
        let cases: &[(&str, i64)] = &[
            ("0", 0),
            ("90", 90),
            ("180", 180),
            ("270", 270),
            ("-90", 270),
            ("450", 90),
            ("44", 0),
            ("45", 90),
            ("46", 90),
            ("135", 180),
            ("315", 0),
            ("350", 0),
        ];
        for (raw, expected) in cases {
            let pdf = one_page(&format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [ 0 0 595.32 841.92 ] \
                 /Rotate {raw} /Contents 4 0 R >>"
            ));
            let file = PdfFile::parse(&pdf).unwrap();
            assert_eq!(file.page(0).unwrap().rotate(), *expected, "/Rotate {raw}");
        }
    }

    /// Инкрементальное обновление: вторая таблица `xref` с `/Prev`
    /// перекрывает объект 2 и добавляет объект 5.
    #[test]
    fn pdf_reader_follows_the_prev_chain() {
        let base = build_classic(
            &[
                (1, b"<< /Type /Catalog /Pages 2 0 R >>".to_vec()),
                (
                    2,
                    b"<< /Type /Pages /Kids [ 3 0 R ] /Count 1 /MediaBox [ 0 0 595.32 841.92 ] >>"
                        .to_vec(),
                ),
                (
                    3,
                    b"<< /Type /Page /Parent 2 0 R /Contents 4 0 R >>".to_vec(),
                ),
                (4, empty_content()),
            ],
            "",
        );
        let previous = find(&base, b"xref\n0 ").expect("в базовом файле есть таблица");
        let mut out = base.clone();
        let five = out.len();
        out.extend_from_slice(
            b"5 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [ 0 0 841.92 595.32 ] \
              /Contents 4 0 R >>\nendobj\n",
        );
        let two = out.len();
        out.extend_from_slice(
            b"2 0 obj\n<< /Type /Pages /Kids [ 3 0 R 5 0 R ] /Count 2 \
              /MediaBox [ 0 0 595.32 841.92 ] >>\nendobj\n",
        );
        let xref = out.len();
        out.extend_from_slice(
            format!(
                "xref\n0 1\n0000000000 65535 f \n2 1\n{two:010} 00000 n \n5 1\n{five:010} 00000 n \n\
                 trailer\n<< /Size 6 /Root 1 0 R /Prev {previous} >>\nstartxref\n{xref}\n%%EOF\n"
            )
            .as_bytes(),
        );

        let file = PdfFile::parse(&out).expect("инкрементальное обновление обязано читаться");
        assert_eq!(file.page_count(), 2);
        assert_eq!(file.page(0).unwrap().width_mm(), 210);
        assert_eq!(file.page(1).unwrap().width_mm(), 297);
        // Базовый файл в одиночку по-прежнему даёт одну страницу — значит
        // прочиталась именно цепочка, а не хвост.
        assert_eq!(PdfFile::parse(&base).unwrap().page_count(), 1);
    }

    /// Файл PDF 1.5: `xref`-поток с предиктором PNG «Up» и объектный поток,
    /// в котором лежат каталог, узел `/Pages` и обе страницы.
    ///
    /// Собран байтами прямо здесь по разделам 7.5.7 и 7.5.8 спецификации:
    /// ни qpdf, ни pikepdf на машине нет, а снимать такой файл с платформы
    /// нечем — её писатель кладёт классическую таблицу.
    #[test]
    fn pdf_reader_reads_xref_stream_with_object_stream() {
        let inner: &[(u32, &str)] = &[
            (1, "<< /Type /Catalog /Pages 2 0 R >>"),
            (
                2,
                "<< /Type /Pages /Kids [ 3 0 R 4 0 R ] /Count 2 /MediaBox [ 0 0 595.32 841.92 ] >>",
            ),
            (3, "<< /Type /Page /Parent 2 0 R /Contents 6 0 R >>"),
            (
                4,
                "<< /Type /Page /Parent 2 0 R /MediaBox [ 0 0 841.92 595.32 ] /Contents 6 0 R >>",
            ),
        ];
        let mut head = String::new();
        let mut body = String::new();
        for (number, text) in inner {
            head.push_str(&format!("{number} {} ", body.len()));
            body.push_str(text);
            body.push(' ');
        }
        let first = head.len();
        let objstm_plain = format!("{head}{body}").into_bytes();
        let objstm = zlib_compress(&objstm_plain);

        let mut out = b"%PDF-1.5\n%\xe2\xe3\xcf\xd3\n".to_vec();
        let at_five = out.len();
        out.extend_from_slice(
            format!(
                "5 0 obj\n<< /Type /ObjStm /N {} /First {first} /Filter /FlateDecode /Length {} >>\nstream\n",
                inner.len(),
                objstm.len()
            )
            .as_bytes(),
        );
        out.extend_from_slice(&objstm);
        out.extend_from_slice(b"\nendstream\nendobj\n");
        let at_six = out.len();
        out.extend_from_slice(b"6 0 obj\n");
        out.extend_from_slice(&empty_content());
        out.extend_from_slice(b"\nendobj\n");
        let at_xref = out.len();

        // Записи шириной [1 4 2]; строки предсказаны фильтром PNG «Up»
        // (тип 2), то есть каждая строка — разность с предыдущей.
        let mut rows: Vec<[u8; 7]> = Vec::new();
        let mut push = |kind: u8, second: u32, third: u16| {
            let mut row = [0u8; 7];
            row[0] = kind;
            row[1..5].copy_from_slice(&second.to_be_bytes());
            row[5..7].copy_from_slice(&third.to_be_bytes());
            rows.push(row);
        };
        push(0, 0, 65535);
        for index in 0..inner.len() {
            push(2, 5, index as u16);
        }
        push(1, at_five as u32, 0);
        push(1, at_six as u32, 0);
        push(1, at_xref as u32, 0);
        let mut predicted = Vec::new();
        let mut previous = [0u8; 7];
        for row in &rows {
            predicted.push(2u8);
            for i in 0..7 {
                predicted.push(row[i].wrapping_sub(previous[i]));
            }
            previous = *row;
        }
        let xref_stream = zlib_compress(&predicted);
        out.extend_from_slice(
            format!(
                "7 0 obj\n<< /Type /XRef /Size 8 /W [ 1 4 2 ] /Root 1 0 R /Filter /FlateDecode \
                 /DecodeParms << /Predictor 12 /Columns 7 >> /Length {} >>\nstream\n",
                xref_stream.len()
            )
            .as_bytes(),
        );
        out.extend_from_slice(&xref_stream);
        out.extend_from_slice(b"\nendstream\nendobj\n");
        out.extend_from_slice(format!("startxref\n{at_xref}\n%%EOF\n").as_bytes());

        let file = PdfFile::parse(&out).expect("xref-поток с ObjStm обязан читаться");
        assert_eq!(file.page_count(), 2);
        assert_eq!(file.page(0).unwrap().width_mm(), 210);
        assert_eq!(file.page(1).unwrap().width_mm(), 297);
    }

    /// Зашифрованный файл — ЧЕСТНЫЙ ОТКАЗ, а не попытка прочитать мусор:
    /// расшифровки здесь нет.
    #[test]
    fn pdf_reader_refuses_an_encrypted_file() {
        let pdf = build_classic(
            &[
                (1, b"<< /Type /Catalog /Pages 2 0 R >>".to_vec()),
                (
                    2,
                    b"<< /Type /Pages /Kids [ 3 0 R ] /Count 1 /MediaBox [ 0 0 595.32 841.92 ] >>"
                        .to_vec(),
                ),
                (
                    3,
                    b"<< /Type /Page /Parent 2 0 R /Contents 4 0 R >>".to_vec(),
                ),
                (4, empty_content()),
                (
                    5,
                    b"<< /Filter /Standard /V 1 /R 2 /O <00> /U <00> /P -1 >>".to_vec(),
                ),
            ],
            " /Encrypt 5 0 R",
        );
        let error = PdfFile::parse(&pdf).expect_err("зашифрованный файл читаться не должен");
        let RtError::Pdf(text) = error else {
            panic!("ожидалась ошибка PDF");
        };
        assert!(text.contains("зашифрован"), "текст ошибки: {text}");
    }

    /// Неподдержанный фильтр отвергается ПО ИМЕНИ — чтобы из текста было
    /// видно, чего не хватает.
    #[test]
    fn pdf_reader_names_the_unsupported_filter() {
        let inner = b"<< /Type /Catalog /Pages 2 0 R >>";
        let pdf = build_classic(
            &[
                (
                    1,
                    format!(
                        "<< /Type /ObjStm /N 1 /First 4 /Filter /LZWDecode /Length {} >>\nstream\n{}\nendstream",
                        inner.len() + 4,
                        format_args!("1 0 {}", String::from_utf8_lossy(inner))
                    )
                    .into_bytes(),
                ),
                (2, b"<< /Type /Pages /Kids [ ] /Count 0 >>".to_vec()),
            ],
            "",
        );
        // Каталог здесь — сам объект 1, то есть словарь потока; разбор
        // упрётся в фильтр только если добраться до потока, поэтому файл
        // собран так, чтобы `/Root` вёл именно в него.
        let error = PdfFile::parse(&pdf);
        // Каталог-поток — вырожденный случай; важно, что разбор кончается
        // ошибкой, а не паникой.
        assert!(error.is_err());

        // А вот прямая проверка фильтра: поток страницы с /LZWDecode.
        let mut reader = Reader::new(&pdf).unwrap();
        let dict = vec![(
            "Filter".to_string(),
            PdfValue::Name("LZWDecode".to_string()),
        )];
        let error = reader
            .decode_stream(&dict, b"whatever")
            .expect_err("LZW не поддержан");
        let RtError::Pdf(text) = error else {
            panic!("ожидалась ошибка PDF");
        };
        assert!(text.contains("LZWDecode"), "текст ошибки: {text}");
    }

    /// Битые входы: разбор обязан кончиться ошибкой, а не паникой и не
    /// зависанием.
    #[test]
    fn pdf_reader_rejects_broken_input_without_hanging() {
        let good = one_page(
            "<< /Type /Page /Parent 2 0 R /MediaBox [ 0 0 595.32 841.92 ] /Contents 4 0 R >>",
        );
        let cases: Vec<(&str, Vec<u8>)> = vec![
            ("пусто", Vec::new()),
            ("не PDF", b"just some bytes".to_vec()),
            (
                "заголовок без содержимого",
                b"%PDF-1.4\nnonsense\n".to_vec(),
            ),
            ("обрезан на половине", good[..good.len() / 2].to_vec()),
            ("обрезан на четверти", good[..good.len() / 4].to_vec()),
            ("startxref в никуда", {
                let mut bytes = good.clone();
                let at = find(&bytes, b"startxref\n").unwrap();
                bytes.truncate(at);
                bytes.extend_from_slice(b"startxref\n999999999\n%%EOF\n");
                bytes
            }),
            (
                "трейлер без /Root",
                build_classic(
                    &[(1, b"<< /Type /Pages /Kids [ ] /Count 0 >>".to_vec())],
                    "",
                )
                .split(|b| *b == b'\n')
                .map(|line| {
                    if line.starts_with(b"<< /Size") {
                        b"<< /Size 2 >>".to_vec()
                    } else {
                        line.to_vec()
                    }
                })
                .collect::<Vec<_>>()
                .join(&b'\n'),
            ),
            (
                "цикл в дереве страниц",
                build_classic(
                    &[
                        (1, b"<< /Type /Catalog /Pages 2 0 R >>".to_vec()),
                        (2, b"<< /Type /Pages /Kids [ 3 0 R ] /Count 1 >>".to_vec()),
                        (3, b"<< /Type /Pages /Kids [ 2 0 R ] /Count 1 >>".to_vec()),
                    ],
                    "",
                ),
            ),
            (
                "циклическая косвенная ссылка",
                build_classic(
                    &[
                        (1, b"<< /Type /Catalog /Pages 2 0 R >>".to_vec()),
                        (2, b"2 0 R".to_vec()),
                    ],
                    "",
                ),
            ),
            (
                "словарь без закрытия",
                build_classic(
                    &[
                        (1, b"<< /Type /Catalog /Pages 2 0 R".to_vec()),
                        (2, b"<< /Type /Pages /Kids [ ] /Count 0 >>".to_vec()),
                    ],
                    "",
                ),
            ),
        ];
        for (what, bytes) in cases {
            match PdfFile::parse(&bytes) {
                Err(RtError::Pdf(text)) => {
                    assert!(!text.is_empty(), "у ошибки «{what}» пустой текст");
                }
                Err(other) => panic!("«{what}»: ожидалась ошибка PDF, получено {other:?}"),
                Ok(file) => panic!("«{what}» разобрался в {} страниц", file.page_count()),
            }
        }
    }

    /// Мусор на входе не должен уводить разбор в панику ни на одном
    /// префиксе живого файла: грубая, но действенная проверка на то, что
    /// каждый обрыв обработан.
    #[test]
    fn pdf_reader_survives_every_prefix_of_a_good_file() {
        let good = one_page(
            "<< /Type /Page /Parent 2 0 R /MediaBox [ 0 0 595.32 841.92 ] /Contents 4 0 R >>",
        );
        for cut in 0..good.len() {
            // Результат не важен: важно, что вызов вернулся.
            let _ = PdfFile::parse(&good[..cut]);
        }
        // И то же самое с одним испорченным байтом в каждой позиции.
        for at in 0..good.len() {
            let mut bytes = good.clone();
            bytes[at] = b'#';
            let _ = PdfFile::parse(&bytes);
        }
    }

    /// Числа, которые проба кладёт в словари вместо правильных. `None` —
    /// вычисленное значение, то есть законный файл.
    #[derive(Debug, Default, Clone, Copy)]
    struct Boundary<'a> {
        /// `/N` объектного потока.
        n: Option<&'a str>,
        /// `/First` объектного потока.
        first: Option<&'a str>,
        /// `/Length` объектного потока.
        objstm_length: Option<&'a str>,
        /// `/Size` xref-потока.
        size: Option<&'a str>,
        /// Целиком ключ `/Index [ ... ]`: по умолчанию его нет вовсе и
        /// действует умолчание `[0 /Size]` (раздел 7.5.8.2).
        index: Option<&'a str>,
        /// Целиком массив `/W`.
        widths: Option<&'a str>,
        /// Целиком словарь `/DecodeParms` xref-потока.
        parms: Option<&'a str>,
        /// `/Length` xref-потока.
        xref_length: Option<&'a str>,
    }

    /// Собрать файл PDF 1.5 «объектный поток плюс xref-поток» байтами по
    /// разделам 7.5.7 и 7.5.8 спецификации, подставив в числовые ключи то,
    /// что просит проба. Без подстановок файл законный и читается — значит
    /// в пробе ломает именно подставленное число, а не сборка.
    fn build_boundary_pdf(boundary: Boundary<'_>) -> Vec<u8> {
        let pick = |over: Option<&str>, computed: String| -> String {
            over.map(str::to_string).unwrap_or(computed)
        };
        // Каталог и узел `/Pages` лежат внутри объектного потока 1,
        // xref-поток — объект 4.
        let inner: &[(u32, &str)] = &[
            (2, "<< /Type /Catalog /Pages 3 0 R >>"),
            (3, "<< /Type /Pages /Kids [ ] /Count 0 >>"),
        ];
        let mut head = String::new();
        let mut body = String::new();
        for (number, text) in inner {
            head.push_str(&format!("{number} {} ", body.len()));
            body.push_str(text);
            body.push(' ');
        }
        let first = head.len();
        let objstm = zlib_compress(format!("{head}{body}").as_bytes());

        let mut out = b"%PDF-1.5\n%\xe2\xe3\xcf\xd3\n".to_vec();
        let at_objstm = out.len();
        out.extend_from_slice(
            format!(
                "1 0 obj\n<< /Type /ObjStm /N {} /First {} /Filter /FlateDecode /Length {} >>\nstream\n",
                pick(boundary.n, inner.len().to_string()),
                pick(boundary.first, first.to_string()),
                pick(boundary.objstm_length, objstm.len().to_string()),
            )
            .as_bytes(),
        );
        out.extend_from_slice(&objstm);
        out.extend_from_slice(b"\nendstream\nendobj\n");
        let at_xref = out.len();

        // Записи шириной [1 4 2], строки предсказаны фильтром PNG «Up».
        let mut rows: Vec<[u8; 7]> = Vec::new();
        let mut push = |kind: u8, second: u32, third: u16| {
            let mut row = [0u8; 7];
            row[0] = kind;
            row[1..5].copy_from_slice(&second.to_be_bytes());
            row[5..7].copy_from_slice(&third.to_be_bytes());
            rows.push(row);
        };
        push(0, 0, 65535);
        push(1, at_objstm as u32, 0);
        push(2, 1, 0);
        push(2, 1, 1);
        push(1, at_xref as u32, 0);
        let mut predicted = Vec::new();
        let mut previous = [0u8; 7];
        for row in &rows {
            predicted.push(2u8);
            for i in 0..7 {
                predicted.push(row[i].wrapping_sub(previous[i]));
            }
            previous = *row;
        }
        let xref_stream = zlib_compress(&predicted);
        out.extend_from_slice(
            format!(
                "4 0 obj\n<< /Type /XRef /Size {} {} /W {} /Root 2 0 R /Filter /FlateDecode \
                 /DecodeParms {} /Length {} >>\nstream\n",
                pick(boundary.size, rows.len().to_string()),
                boundary.index.unwrap_or(""),
                pick(boundary.widths, "[ 1 4 2 ]".to_string()),
                pick(boundary.parms, "<< /Predictor 12 /Columns 7 >>".to_string()),
                pick(boundary.xref_length, xref_stream.len().to_string()),
            )
            .as_bytes(),
        );
        out.extend_from_slice(&xref_stream);
        out.extend_from_slice(b"\nendstream\nendobj\n");
        out.extend_from_slice(format!("startxref\n{at_xref}\n%%EOF\n").as_bytes());
        out
    }

    /// Граничные числа ПРЯМО ИЗ ФАЙЛА в каждом числовом ключе разбора:
    /// разбор обязан вернуться с `RtError`, а не переполнить арифметику и
    /// не выделить память по объявленному в файле размеру. Проверки
    /// префиксов и однобайтовых порч этого не ловят — двадцатизначное
    /// число из хорошего файла так не получить, поэтому словари здесь
    /// собираются байтами.
    ///
    /// Случаи, помеченные как обязанные упасть, — ровно те, где иначе
    /// сложение уходит за `i64` или строка предиктора выделяется по
    /// объявленным гигабайтам; там `Ok` означал бы, что защита снята.
    #[test]
    fn pdf_reader_rejects_boundary_numbers_in_dictionaries() {
        assert_eq!(
            PdfFile::parse(&build_boundary_pdf(Boundary::default()))
                .expect("базовый файл пробы обязан читаться")
                .page_count(),
            0
        );

        let cases: &[(&str, Boundary, bool)] = &[
            (
                "первый номер /Index у i64::MAX",
                Boundary {
                    index: Some("/Index [ 9223372036854775807 2 ]"),
                    ..Boundary::default()
                },
                true,
            ),
            (
                "первый номер /Index на единицу ниже i64::MAX",
                Boundary {
                    index: Some("/Index [ 9223372036854775806 2 ]"),
                    ..Boundary::default()
                },
                true,
            ),
            (
                "первый номер /Index за u32",
                Boundary {
                    index: Some("/Index [ 4294967296 2 ]"),
                    ..Boundary::default()
                },
                true,
            ),
            (
                "первый номер /Index на 10^11",
                Boundary {
                    index: Some("/Index [ 100000000000 1 ]"),
                    ..Boundary::default()
                },
                true,
            ),
            (
                "число записей /Index у i64::MAX",
                Boundary {
                    index: Some("/Index [ 0 9223372036854775807 ]"),
                    ..Boundary::default()
                },
                true,
            ),
            (
                "ширина поля /W у i64::MAX",
                Boundary {
                    widths: Some("[ 9223372036854775807 4 2 ]"),
                    ..Boundary::default()
                },
                true,
            ),
            (
                "лишнее поле /W на 10^11",
                Boundary {
                    widths: Some("[ 1 4 2 100000000000 ]"),
                    ..Boundary::default()
                },
                true,
            ),
            (
                "/Size у i64::MAX",
                Boundary {
                    size: Some("9223372036854775807"),
                    ..Boundary::default()
                },
                true,
            ),
            (
                "/Size на 10^11",
                Boundary {
                    size: Some("100000000000"),
                    ..Boundary::default()
                },
                true,
            ),
            (
                "/N объектного потока у i64::MAX",
                Boundary {
                    n: Some("9223372036854775807"),
                    ..Boundary::default()
                },
                true,
            ),
            (
                "/N объектного потока за u32",
                Boundary {
                    n: Some("4294967296"),
                    ..Boundary::default()
                },
                true,
            ),
            (
                "/First объектного потока у i64::MAX",
                Boundary {
                    first: Some("9223372036854775807"),
                    ..Boundary::default()
                },
                true,
            ),
            (
                "/First объектного потока на 10^11",
                Boundary {
                    first: Some("100000000000"),
                    ..Boundary::default()
                },
                true,
            ),
            (
                "/Colors предиктора у i64::MAX",
                Boundary {
                    parms: Some(
                        "<< /Predictor 12 /Colors 9223372036854775807 /BitsPerComponent 1 /Columns 1 >>",
                    ),
                    ..Boundary::default()
                },
                true,
            ),
            (
                "/Columns предиктора на 10^11",
                Boundary {
                    parms: Some("<< /Predictor 12 /Columns 100000000000 >>"),
                    ..Boundary::default()
                },
                true,
            ),
            (
                "/Columns предиктора у i64::MAX",
                Boundary {
                    parms: Some("<< /Predictor 12 /Columns 9223372036854775807 >>"),
                    ..Boundary::default()
                },
                true,
            ),
            (
                "/BitsPerComponent предиктора у i64::MAX",
                Boundary {
                    parms: Some("<< /Predictor 12 /BitsPerComponent 9223372036854775807 /Columns 7 >>"),
                    ..Boundary::default()
                },
                true,
            ),
            (
                "/BitsPerComponent предиктора за u32",
                Boundary {
                    parms: Some("<< /Predictor 12 /BitsPerComponent 4294967296 /Columns 7 >>"),
                    ..Boundary::default()
                },
                true,
            ),
            (
                "/Colors предиктора за u32",
                Boundary {
                    parms: Some("<< /Predictor 12 /Colors 4294967296 /Columns 7 >>"),
                    ..Boundary::default()
                },
                true,
            ),
            // Номер предиктора больше десяти — это по-прежнему строки PNG
            // (раздел 7.4.4.4), поэтому такой файл имеет право прочитаться.
            (
                "/Predictor у i64::MAX",
                Boundary {
                    parms: Some("<< /Predictor 9223372036854775807 /Columns 7 >>"),
                    ..Boundary::default()
                },
                false,
            ),
            // Соврамший `/Length` спасает поиск `endstream`, и это
            // измеренное поведение платформы, а не недосмотр.
            (
                "/Length объектного потока у i64::MAX",
                Boundary {
                    objstm_length: Some("9223372036854775807"),
                    ..Boundary::default()
                },
                false,
            ),
            (
                "/Length xref-потока на 10^11",
                Boundary {
                    xref_length: Some("100000000000"),
                    ..Boundary::default()
                },
                false,
            ),
        ];

        for (what, boundary, must_fail) in cases {
            match PdfFile::parse(&build_boundary_pdf(*boundary)) {
                Err(RtError::Pdf(text)) => {
                    assert!(!text.is_empty(), "у ошибки «{what}» пустой текст");
                }
                Err(other) => panic!("«{what}»: ожидалась ошибка PDF, получено {other:?}"),
                Ok(file) => assert!(
                    !must_fail,
                    "«{what}» разобрался в {} страниц",
                    file.page_count()
                ),
            }
        }
    }

    /// Пустое дерево страниц — законный файл с нулём страниц (измерено).
    #[test]
    fn pdf_reader_accepts_an_empty_page_tree() {
        let pdf = build_classic(
            &[
                (1, b"<< /Type /Catalog /Pages 2 0 R >>".to_vec()),
                (2, b"<< /Type /Pages /Kids [ ] /Count 0 >>".to_vec()),
            ],
            "",
        );
        let file = PdfFile::parse(&pdf).expect("пустое дерево — законный файл");
        assert_eq!(file.page_count(), 0);
    }

    /// `/Length` косвенной ссылкой — так пишет сама платформа
    /// (`platform-simple.pdf`), и так же обязан читаться собранный здесь
    /// файл; соврамший `/Length` не должен рушить разбор.
    #[test]
    fn pdf_reader_takes_length_from_an_indirect_object_and_survives_a_wrong_one() {
        let pdf = build_classic(
            &[
                (1, b"<< /Type /Catalog /Pages 2 0 R >>".to_vec()),
                (
                    2,
                    b"<< /Type /Pages /Kids [ 3 0 R ] /Count 1 /MediaBox [ 0 0 595.32 841.92 ] >>"
                        .to_vec(),
                ),
                (
                    3,
                    b"<< /Type /Page /Parent 2 0 R /Contents 4 0 R >>".to_vec(),
                ),
                (4, b"<< /Length 5 0 R >>\nstream\nq Q\nendstream".to_vec()),
                (5, b"3".to_vec()),
            ],
            "",
        );
        assert_eq!(PdfFile::parse(&pdf).unwrap().page_count(), 1);

        // Подмена РАВНОЙ ДЛИНЫ: «/Length 5 0 R» и «/Length 99999» — по
        // тринадцать байтов, иначе поехали бы смещения в таблице xref и
        // проба мерила бы не то.
        let mut lying = pdf.clone();
        let at = find(&lying, b"/Length 5 0 R").expect("поток с косвенной длиной");
        lying[at..at + 13].copy_from_slice(b"/Length 99999");
        assert_eq!(PdfFile::parse(&lying).unwrap().page_count(), 1);
    }

    /// Синтаксис значений: имена с `#`, строки со скобками и escape'ами,
    /// шестнадцатеричные строки, логические значения и `null`.
    #[test]
    fn pdf_reader_parses_the_value_syntax() {
        let source =
            "[ /A#20B (в (скобках) \\) и \\101) <48656C6C6F> <4a4> true false null 1 2 R -3.5 ]"
                .as_bytes();
        let value = Lexer::new(source, 0).parse_value(0).unwrap();
        let PdfValue::Array(items) = value else {
            panic!("ожидался массив");
        };
        assert_eq!(items[0], PdfValue::Name("A B".to_string()));
        assert_eq!(
            items[1],
            PdfValue::Str("в (скобках) ) и A".as_bytes().to_vec())
        );
        assert_eq!(items[2], PdfValue::Str(b"Hello".to_vec()));
        // Нечётный хвост шестнадцатеричной строки дополняется нулём.
        assert_eq!(items[3], PdfValue::Str(vec![0x4A, 0x40]));
        assert_eq!(items[4], PdfValue::Bool(true));
        assert_eq!(items[5], PdfValue::Bool(false));
        assert_eq!(items[6], PdfValue::Null);
        assert_eq!(items[7], PdfValue::Ref(1));
        assert_eq!(items[8], PdfValue::Real(-3.5));
        assert_eq!(items.len(), 9);
    }

    /// КРУГ ЗАМЫКАЕТСЯ на файле самой платформы: `probe-margins.pdf` она
    /// записала из документа с полями 30, 10, 25 и 5 мм
    /// (`capture-platform-pdf-layout.bsl`, строки 241..244), и читатель
    /// обязан вернуть ровно их.
    #[test]
    fn pdf_reader_reads_back_the_platform_margins() {
        let bytes = std::fs::read("../../tests/conformance/pdf/probe-margins.pdf").unwrap();
        let file = PdfFile::parse(&bytes).unwrap();
        let page = file.page(0).unwrap();
        assert_eq!(page.margin(PdfMargin::Left), 30);
        assert_eq!(page.margin(PdfMargin::Right), 10);
        assert_eq!(page.margin(PdfMargin::Top), 25);
        assert_eq!(page.margin(PdfMargin::Bottom), 5);
        // А умолчание полей табличного документа — 10 мм со всех сторон.
        let empty = std::fs::read("../../tests/conformance/pdf/probe-empty.pdf").unwrap();
        let empty = PdfFile::parse(&empty).unwrap();
        let page = empty.page(0).unwrap();
        assert_eq!(
            [
                page.margin(PdfMargin::Left),
                page.margin(PdfMargin::Right),
                page.margin(PdfMargin::Top),
                page.margin(PdfMargin::Bottom),
            ],
            [10, 10, 10, 10]
        );
    }

    /// Правило полей целиком, все одиннадцать снятых страниц.
    #[test]
    fn pdf_reader_takes_margins_from_the_trim_box_only() {
        let a4 = "/MediaBox [ 0 0 595.32 841.92 ]";
        let shift = "/MediaBox [ 10 20 605.32 861.92 ]";
        let cases: &[(&str, [i64; 4])] = &[
            // Обе рамки объявлены и совпадают — как пишет платформа.
            (
                "/TrimBox [ 85.04 70.87 566.97 827.75 ] /BleedBox [ 85.04 70.87 566.97 827.75 ]",
                [30, 10, 25, 5],
            ),
            ("/TrimBox [ 85.04 70.87 566.97 827.75 ]", [30, 10, 25, 5]),
            // Один `/BleedBox` полей не даёт, как и один `/ArtBox`.
            ("/BleedBox [ 85.04 70.87 566.97 827.75 ]", [0, 0, 0, 0]),
            ("/ArtBox [ 85.04 70.87 566.97 827.75 ]", [0, 0, 0, 0]),
            // Рамки разные — побеждает `/TrimBox`.
            (
                "/TrimBox [ 56.7 56.7 538.62 785.22 ] /BleedBox [ 14.17 14.17 581.15 827.75 ]",
                [20, 20, 20, 20],
            ),
            // `/CropBox` двигает ДАЛЬНИЕ края, но не ближние.
            (
                "/CropBox [ 50 60 545.32 781.92 ] /TrimBox [ 85.04 70.87 495.32 731.92 ]",
                [30, 18, 25, 18],
            ),
            // Поля не поджимаются к нулю.
            ("/TrimBox [ -50 -50 700 900 ]", [-18, -37, -18, -20]),
            ("/TrimBox [ 0 0 595.32 841.92 ]", [0, 0, 0, 0]),
        ];
        for (extra, expected) in cases {
            let pdf = one_page(&format!(
                "<< /Type /Page /Parent 2 0 R {a4} {extra} /Contents 4 0 R >>"
            ));
            let file = PdfFile::parse(&pdf).unwrap();
            let page = file.page(0).unwrap();
            let got = [
                page.margin(PdfMargin::Left),
                page.margin(PdfMargin::Right),
                page.margin(PdfMargin::Top),
                page.margin(PdfMargin::Bottom),
            ];
            assert_eq!(got, *expected, "страница с {extra}");
        }

        // Смещённое начало `/MediaBox`: левое и верхнее поля — АБСОЛЮТНЫЕ
        // координаты угла `/TrimBox`, а не отступы от рамки.
        let shifted: &[(&str, [i64; 4])] = &[
            ("/TrimBox [ 95.04 90.87 576.97 847.75 ]", [34, 10, 32, 5]),
            (
                "/CropBox [ 60 80 555.32 801.92 ] /TrimBox [ 95.04 90.87 505.32 751.92 ]",
                [34, 18, 32, 18],
            ),
        ];
        for (extra, expected) in shifted {
            let pdf = one_page(&format!(
                "<< /Type /Page /Parent 2 0 R {shift} {extra} /Contents 4 0 R >>"
            ));
            let file = PdfFile::parse(&pdf).unwrap();
            let page = file.page(0).unwrap();
            let got = [
                page.margin(PdfMargin::Left),
                page.margin(PdfMargin::Right),
                page.margin(PdfMargin::Top),
                page.margin(PdfMargin::Bottom),
            ];
            assert_eq!(got, *expected, "смещённая страница с {extra}");
        }

        // `/TrimBox` НЕ наследуется от узла `/Pages` — как и `/Rotate`.
        let pdf = build_classic(
            &[
                (1, b"<< /Type /Catalog /Pages 2 0 R >>".to_vec()),
                (
                    2,
                    b"<< /Type /Pages /Kids [ 3 0 R ] /Count 1 /MediaBox [ 0 0 595.32 841.92 ] \
                       /TrimBox [ 85.04 70.87 566.97 827.75 ] >>"
                        .to_vec(),
                ),
                (
                    3,
                    b"<< /Type /Page /Parent 2 0 R /Contents 4 0 R >>".to_vec(),
                ),
                (4, empty_content()),
            ],
            "",
        );
        let file = PdfFile::parse(&pdf).unwrap();
        assert_eq!(file.page(0).unwrap().margin(PdfMargin::Left), 0);
    }

    /// Все пять фильтров строк PNG, снимаемые предиктором, — и отказ с
    /// НОМЕРОМ на предикторе TIFF и на неизвестном типе строки.
    #[test]
    fn pdf_reader_undoes_every_png_row_filter() {
        // Три строки по четыре байта, шаг предсказания — один байт.
        let rows: [[u8; 4]; 3] = [[10, 20, 30, 40], [11, 22, 33, 44], [200, 100, 50, 25]];
        for kind in 0u8..=4 {
            let mut encoded = Vec::new();
            let mut previous = [0u8; 4];
            for row in &rows {
                encoded.push(kind);
                for i in 0..4 {
                    let left = if i >= 1 { row[i - 1] } else { 0 };
                    let up = previous[i];
                    let up_left = if i >= 1 { previous[i - 1] } else { 0 };
                    let predictor = match kind {
                        0 => 0,
                        1 => left,
                        2 => up,
                        3 => ((u16::from(left) + u16::from(up)) / 2) as u8,
                        _ => paeth(left, up, up_left),
                    };
                    encoded.push(row[i].wrapping_sub(predictor));
                }
                previous = *row;
            }
            let empty = one_page("<< /Type /Page /Parent 2 0 R /Contents 4 0 R >>");
            let mut reader = Reader::new(&empty).unwrap();
            let parms = vec![
                ("Predictor".to_string(), PdfValue::Integer(12)),
                ("Columns".to_string(), PdfValue::Integer(4)),
            ];
            let decoded = reader.apply_predictor(&parms, encoded).unwrap();
            let expected: Vec<u8> = rows.iter().flatten().copied().collect();
            assert_eq!(decoded, expected, "фильтр строки {kind}");
        }

        let empty = one_page("<< /Type /Page /Parent 2 0 R /Contents 4 0 R >>");
        let mut reader = Reader::new(&empty).unwrap();
        let tiff = vec![("Predictor".to_string(), PdfValue::Integer(2))];
        let error = reader
            .apply_predictor(&tiff, vec![0; 4])
            .expect_err("предиктор TIFF не поддержан");
        let RtError::Pdf(text) = error else {
            panic!("ожидалась ошибка PDF");
        };
        assert!(
            text.contains('2'),
            "номер предиктора обязан быть в тексте: {text}"
        );

        let png = vec![
            ("Predictor".to_string(), PdfValue::Integer(12)),
            ("Columns".to_string(), PdfValue::Integer(4)),
        ];
        let error = reader
            .apply_predictor(&png, vec![9, 0, 0, 0, 0])
            .expect_err("тип строки 9 у PNG не определён");
        let RtError::Pdf(text) = error else {
            panic!("ожидалась ошибка PDF");
        };
        assert!(
            text.contains('9'),
            "номер типа строки обязан быть в тексте: {text}"
        );
    }

    // -----------------------------------------------------------------
    // Вложения
    // -----------------------------------------------------------------

    /// Однолистовой файл с деревом имён вложений: `spec` — тело
    /// `/EmbeddedFiles`, `objects` — всё остальное, что ему нужно.
    fn with_attachments(tree: &str, objects: &[(u32, Vec<u8>)]) -> Vec<u8> {
        let mut all = vec![
            (
                1,
                format!("<< /Type /Catalog /Pages 2 0 R /Names << /EmbeddedFiles {tree} >> >>")
                    .into_bytes(),
            ),
            (
                2,
                b"<< /Type /Pages /Kids [ 3 0 R ] /Count 1 /MediaBox [ 0 0 595.32 841.92 ] >>"
                    .to_vec(),
            ),
            (
                3,
                b"<< /Type /Page /Parent 2 0 R /Contents 4 0 R >>".to_vec(),
            ),
            (4, empty_content()),
        ];
        all.extend_from_slice(objects);
        all.sort_by_key(|(number, _)| *number);
        build_classic(&all, "")
    }

    /// Пара объектов «поток встроенного файла + файловая спецификация»
    /// под номерами `number` и `number + 1`.
    fn filespec_pair(number: u32, head: &str, data: &[u8]) -> Vec<(u32, Vec<u8>)> {
        let mut stream = format!(
            "<< /Type /EmbeddedFile /Subtype /text#2Fplain /Length {} >>\nstream\n",
            data.len()
        )
        .into_bytes();
        stream.extend_from_slice(data);
        stream.extend_from_slice(b"\nendstream");
        vec![
            (number, stream),
            (
                number + 1,
                format!("<< /Type /Filespec {head} /EF << /F {number} 0 R >> >>").into_bytes(),
            ),
        ]
    }

    /// Файл с xref-ПОТОКОМ вместо классической таблицы: объекты пишутся
    /// подряд, а таблица — поток `/XRef` шириной `[1 4 2]` без предиктора.
    fn build_xref_stream(objects: &[(u32, Vec<u8>)]) -> Vec<u8> {
        let mut out = b"%PDF-1.5\n%\xe2\xe3\xcf\xd3\n".to_vec();
        let size = objects.iter().map(|(n, _)| *n).max().unwrap_or(0) + 2;
        let mut offsets = vec![0usize; size as usize];
        for (number, body) in objects {
            offsets[*number as usize] = out.len();
            out.extend_from_slice(format!("{number} 0 obj\n").as_bytes());
            out.extend_from_slice(body);
            out.extend_from_slice(b"\nendobj\n");
        }
        let at_xref = out.len();
        let table_number = size - 1;
        offsets[table_number as usize] = at_xref;
        let mut rows = Vec::new();
        for (index, offset) in offsets.iter().enumerate() {
            if index == 0 {
                rows.push(0u8);
                rows.extend_from_slice(&0u32.to_be_bytes());
                rows.extend_from_slice(&65535u16.to_be_bytes());
                continue;
            }
            rows.push(1u8);
            rows.extend_from_slice(&(*offset as u32).to_be_bytes());
            rows.extend_from_slice(&0u16.to_be_bytes());
        }
        let packed = zlib_compress(&rows);
        out.extend_from_slice(
            format!(
                "{table_number} 0 obj\n<< /Type /XRef /Size {size} /W [ 1 4 2 ] /Root 1 0 R \
                 /Filter /FlateDecode /Length {} >>\nstream\n",
                packed.len()
            )
            .as_bytes(),
        );
        out.extend_from_slice(&packed);
        out.extend_from_slice(b"\nendstream\nendobj\n");
        out.extend_from_slice(format!("startxref\n{at_xref}\n%%EOF\n").as_bytes());
        out
    }

    fn names_tree(entries: &[(&str, u32)]) -> String {
        let mut out = String::from("<< /Names [");
        for (name, number) in entries {
            out.push_str(&format!(" ({name}) {number} 0 R"));
        }
        out.push_str(" ] >>");
        out
    }

    /// Круг «добавить — записать — прочитать своим же читателем» на ОБОИХ
    /// видах таблицы перекрёстных ссылок: инкрементальное обновление
    /// дописывается и поверх классической `xref`, и поверх xref-потока, а
    /// страницы исходного файла при этом остаются на месте.
    #[test]
    fn pdf_attachments_round_trip_over_both_xref_kinds() {
        let mut objects = filespec_pair(10, "/F (первое.txt)", "было".as_bytes());
        let tree = names_tree(&[("первое.txt", 11)]);
        let classic = with_attachments(&tree, &objects);

        objects.sort_by_key(|(number, _)| *number);
        let mut stream_objects = vec![
            (
                1,
                format!("<< /Type /Catalog /Pages 2 0 R /Names << /EmbeddedFiles {tree} >> >>")
                    .into_bytes(),
            ),
            (
                2,
                b"<< /Type /Pages /Kids [ 3 0 R ] /Count 1 /MediaBox [ 0 0 595.32 841.92 ] >>"
                    .to_vec(),
            ),
            (
                3,
                b"<< /Type /Page /Parent 2 0 R /Contents 4 0 R >>".to_vec(),
            ),
            (4, empty_content()),
        ];
        stream_objects.extend(objects);
        stream_objects.sort_by_key(|(number, _)| *number);
        let with_stream = build_xref_stream(&stream_objects);

        for (kind, base) in [("классическая xref", classic), ("xref-поток", with_stream)]
        {
            let file = PdfFile::parse(&base).unwrap_or_else(|e| panic!("{kind}: {e:?}"));
            assert_eq!(file.page_count(), 1, "{kind}: страница");
            assert_eq!(file.attachments().len(), 1, "{kind}: вложение из основы");

            let mut attachments = file.attachments().to_vec();
            attachments.push(PdfAttachment::new(
                "второе.bin".to_string(),
                "application/octet-stream".to_string(),
                PdfRelation::Data,
                vec![0, 1, 2, 250, 251, 252],
            ));
            let updated = file
                .write_with_attachments(&attachments)
                .unwrap_or_else(|e| panic!("{kind}: запись {e:?}"));
            // Исходные байты остаются началом файла: обновление ТОЛЬКО
            // дописывает.
            assert!(updated.starts_with(&base), "{kind}: основа переписана");

            let back =
                PdfFile::parse(&updated).unwrap_or_else(|e| panic!("{kind}: перечтение {e:?}"));
            assert_eq!(back.page_count(), 1, "{kind}: страница после записи");
            let names: Vec<&str> = back.attachments().iter().map(|a| a.name()).collect();
            // Порядок — по имени: так пишется дерево имён.
            assert_eq!(names, ["второе.bin", "первое.txt"], "{kind}: имена");
            assert_eq!(
                back.attachments()[0].data(),
                &[0, 1, 2, 250, 251, 252],
                "{kind}"
            );
            assert_eq!(
                back.attachments()[0].relation(),
                PdfRelation::Data,
                "{kind}"
            );
            assert_eq!(
                back.attachments()[0].content_type(),
                "application/octet-stream",
                "{kind}"
            );
            assert_eq!(back.attachments()[1].data(), "было".as_bytes(), "{kind}");

            // И ещё круг: обновление поверх обновления.
            let mut again = back.attachments().to_vec();
            again.retain(|item| item.name() != "первое.txt");
            let twice = back
                .write_with_attachments(&again)
                .expect("второе обновление");
            let last = PdfFile::parse(&twice).expect("перечтение второго обновления");
            assert_eq!(last.attachments().len(), 1, "{kind}: после удаления");
            assert_eq!(last.attachments()[0].name(), "второе.bin", "{kind}");
            assert_eq!(last.page_count(), 1, "{kind}: страница цела");
        }
    }

    /// Дерево имён с промежуточными узлами `/Kids` обходится целиком, а
    /// `/Limits` при этом не читается вовсе — нам нужны все записи.
    #[test]
    fn pdf_attachments_walk_a_name_tree_with_kids() {
        let mut objects = Vec::new();
        objects.extend(filespec_pair(10, "/F (a.txt)", b"A"));
        objects.extend(filespec_pair(12, "/F (b.txt)", b"B"));
        objects.extend(filespec_pair(14, "/F (c.txt)", b"C"));
        objects.push((
            30,
            b"<< /Limits [ (a.txt) (b.txt) ] /Names [ (a.txt) 11 0 R (b.txt) 13 0 R ] >>".to_vec(),
        ));
        objects.push((
            31,
            b"<< /Limits [ (c.txt) (c.txt) ] /Names [ (c.txt) 15 0 R ] >>".to_vec(),
        ));
        objects.push((32, b"<< /Kids [ 30 0 R 31 0 R ] >>".to_vec()));
        let bytes = with_attachments("32 0 R", &objects);

        let file = PdfFile::parse(&bytes).expect("дерево с /Kids обязано читаться");
        let names: Vec<&str> = file.attachments().iter().map(|a| a.name()).collect();
        assert_eq!(names, ["a.txt", "b.txt", "c.txt"]);
        assert_eq!(file.attachments()[2].data(), b"C");
    }

    /// Битое дерево имён — `RtError` с внятным текстом, а не паника и не
    /// молчаливый пропуск. Вход враждебный: цикл, лишняя глубина,
    /// нечётный `/Names`, не тот тип узла.
    #[test]
    fn pdf_attachments_reject_a_broken_name_tree() {
        let cycle = with_attachments("32 0 R", &[(32, b"<< /Kids [ 32 0 R ] >>".to_vec())]);
        let not_a_dict = with_attachments("32 0 R", &[(32, b"[ 1 2 3 ]".to_vec())]);
        let odd = with_attachments(
            "32 0 R",
            &[(32, "<< /Names [ (одно) ] >>".as_bytes().to_vec())],
        );
        let kids_not_array = with_attachments("32 0 R", &[(32, b"<< /Kids 7 >>".to_vec())]);
        let names_not_array = with_attachments("32 0 R", &[(32, b"<< /Names 7 >>".to_vec())]);

        // Глубина: цепочка `/Kids` длиннее предела.
        let mut deep: Vec<(u32, Vec<u8>)> = Vec::new();
        let depth = MAX_DEPTH as u32 + 5;
        for i in 0..depth {
            deep.push((
                100 + i,
                format!("<< /Kids [ {} 0 R ] >>", 100 + i + 1).into_bytes(),
            ));
        }
        deep.push((100 + depth, b"<< /Names [ ] >>".to_vec()));
        let too_deep = with_attachments("100 0 R", &deep);

        for (what, bytes) in [
            ("цикл", cycle),
            ("узел не словарь", not_a_dict),
            ("нечётный /Names", odd),
            ("/Kids не массив", kids_not_array),
            ("/Names не массив", names_not_array),
            ("слишком глубоко", too_deep),
        ] {
            let err = PdfFile::parse(&bytes)
                .expect_err(&format!("{what}: разбор обязан кончиться ошибкой"));
            let RtError::Pdf(text) = err else {
                panic!("{what}: ожидался RtError::Pdf, получено {err:?}");
            };
            assert!(!text.is_empty(), "{what}: пустой текст ошибки");
        }
    }

    /// Записи без данных платформа молча пропускает — и мы тоже: без
    /// `/EF`, с висящей ссылкой в нём и без имени вовсе.
    #[test]
    fn pdf_attachments_skip_entries_without_data_or_name() {
        let mut objects = Vec::new();
        objects.push((11, b"<< /Type /Filespec /F (noef.txt) >>".to_vec()));
        objects.push((
            13,
            b"<< /Type /Filespec /F (dangling.txt) /EF << /F 99 0 R >> >>".to_vec(),
        ));
        objects.extend(filespec_pair(
            14,
            "/Type /Filespec",
            "безымянное".as_bytes(),
        ));
        objects.extend(filespec_pair(16, "/F (живое.txt)", "живое".as_bytes()));
        let tree = names_tree(&[("noef", 11), ("dangling", 13), ("noname", 15), ("ok", 17)]);
        let bytes = with_attachments(&tree, &objects);

        let file = PdfFile::parse(&bytes).expect("файл с битыми записями обязан читаться");
        let names: Vec<&str> = file.attachments().iter().map(|a| a.name()).collect();
        assert_eq!(names, ["живое.txt"]);
    }

    /// Имя берётся из `/UF`, а без него из `/F`; байты со знаком порядка
    /// FE FF читаются как UTF-16BE, остальные — как UTF-8. Одноимённые
    /// записи схлопываются, и побеждает ПОСЛЕДНЯЯ.
    #[test]
    fn pdf_attachments_decode_names_and_collapse_duplicates() {
        let utf16: String = "юникод.txt"
            .encode_utf16()
            .map(|unit| {
                let [hi, lo] = unit.to_be_bytes();
                format!("\\{hi:03o}\\{lo:03o}")
            })
            .collect();
        let mut objects = Vec::new();
        objects.extend(filespec_pair(10, "/F (только-f.txt)", b"1"));
        objects.extend(filespec_pair(12, "/F (не-это.txt) /UF (это.txt)", b"2"));
        objects.extend(filespec_pair(14, &format!("/UF (\\376\\377{utf16})"), b"3"));
        objects.extend(filespec_pair(16, "/F (дубль.txt)", "первое".as_bytes()));
        objects.extend(filespec_pair(18, "/F (дубль.txt)", "второе".as_bytes()));
        let tree = names_tree(&[("a", 11), ("b", 13), ("c", 15), ("d", 17), ("e", 19)]);
        let bytes = with_attachments(&tree, &objects);

        let file = PdfFile::parse(&bytes).expect("файл обязан читаться");
        let names: Vec<&str> = file.attachments().iter().map(|a| a.name()).collect();
        assert_eq!(
            names,
            ["только-f.txt", "это.txt", "юникод.txt", "дубль.txt"]
        );
        assert_eq!(file.attachments()[3].data(), "второе".as_bytes());
    }

    /// Поток встроенного файла с фильтром, которого мы не умеем, не
    /// уносит документ: вложение остаётся, содержимое — сырые байты.
    #[test]
    fn pdf_attachments_survive_an_unsupported_filter() {
        let objects = vec![
            (
                10,
                b"<< /Type /EmbeddedFile /Filter /LZWDecode /Length 3 >>\nstream\n\x80\x0b\x60\nendstream"
                    .to_vec(),
            ),
            (
                11,
                b"<< /Type /Filespec /F (lzw.txt) /EF << /F 10 0 R >> >>".to_vec(),
            ),
        ];
        let bytes = with_attachments(&names_tree(&[("lzw", 11)]), &objects);

        let file = PdfFile::parse(&bytes).expect("неизвестный фильтр не должен ронять документ");
        assert_eq!(file.page_count(), 1);
        assert_eq!(file.attachments().len(), 1);
        assert_eq!(file.attachments()[0].data(), b"\x80\x0b\x60");
    }

    /// Связь читается из `/AFRelationship`, а неизвестное имя — это
    /// `НеУстановлено` (измерено на `/AFRelationship /Nonsense`).
    #[test]
    fn pdf_attachments_read_the_relationship() {
        let mut objects = Vec::new();
        objects.extend(filespec_pair(
            10,
            "/F (s.txt) /AFRelationship /Source",
            b"1",
        ));
        objects.extend(filespec_pair(12, "/F (d.txt) /AFRelationship /Data", b"2"));
        objects.extend(filespec_pair(
            14,
            "/F (a.txt) /AFRelationship /Alternative",
            b"3",
        ));
        objects.extend(filespec_pair(
            16,
            "/F (u.txt) /AFRelationship /Supplement",
            b"4",
        ));
        objects.extend(filespec_pair(
            18,
            "/F (n.txt) /AFRelationship /Nonsense",
            b"5",
        ));
        objects.extend(filespec_pair(20, "/F (empty.txt)", b"6"));
        let tree = names_tree(&[
            ("a", 11),
            ("b", 13),
            ("c", 15),
            ("d", 17),
            ("e", 19),
            ("f", 21),
        ]);
        let bytes = with_attachments(&tree, &objects);

        let file = PdfFile::parse(&bytes).expect("файл обязан читаться");
        let relations: Vec<PdfRelation> = file.attachments().iter().map(|a| a.relation()).collect();
        assert_eq!(
            relations,
            [
                PdfRelation::Source,
                PdfRelation::Data,
                PdfRelation::Alternative,
                PdfRelation::Supplement,
                PdfRelation::Unspecified,
                PdfRelation::Unspecified,
            ]
        );
    }

    /// Имена в записанном дереве экранируются РОВНО тремя знаками и
    /// уходят сырыми байтами: `/F` в UTF-8, `/UF` в UTF-16BE. Иначе
    /// платформа, которая не снимает восьмеричные экраны, прочитала бы
    /// вместо имени его запись.
    #[test]
    fn pdf_attachments_are_written_with_raw_name_bytes() {
        let base = with_attachments("32 0 R", &[(32, b"<< /Names [ ] >>".to_vec())]);
        let file = PdfFile::parse(&base).expect("основа обязана читаться");
        let written = file
            .write_with_attachments(&[PdfAttachment::new(
                "имя (со скобкой).txt".to_string(),
                "text/plain".to_string(),
                PdfRelation::Unspecified,
                b"data".to_vec(),
            )])
            .expect("запись");

        assert!(
            find(&written, "имя \\(со скобкой\\).txt".as_bytes()).is_some(),
            "имя обязано быть записано сырыми байтами UTF-8 со скобочным экранированием"
        );
        assert!(
            find(&written, &[0xFE, 0xFF, 0x04, 0x38, 0x04, 0x3C, 0x04, 0x4F]).is_some(),
            "/UF обязан быть UTF-16BE с меткой порядка байтов"
        );
        assert!(
            find(&written, b"\\376\\377").is_none(),
            "восьмеричных экранов в имени быть не должно"
        );
        let back = PdfFile::parse(&written).expect("перечтение");
        assert_eq!(back.attachments()[0].name(), "имя (со скобкой).txt");
    }

    /// Шестнадцатеричная строка в фикстуре `pdf-attachments.bsl` — это
    /// РОВНО байты снятого с платформы `attach-platform.pdf`.
    ///
    /// Копия нужна оснастке (конформанс-раннер запускает фикстуру из
    /// каталога крейта, платформенный — из корня репозитория, и одному
    /// относительному пути с обоими не сойтись), но копия, которая молча
    /// разъехалась с оригиналом, — это фикстура, проверяющая не то.
    #[test]
    fn pdf_attachments_fixture_hex_matches_the_captured_file() {
        let fixture =
            std::fs::read_to_string("../../tests/conformance/fixtures/pdf-attachments.bsl")
                .expect("фикстура вложений обязана лежать в дереве");
        let mut hex = String::new();
        for line in fixture.lines() {
            if !line.starts_with("ШестнПлатформа") {
                continue;
            }
            let mut parts = line.split('"');
            parts.next();
            for (index, part) in parts.enumerate() {
                if index % 2 == 0 {
                    hex.push_str(part);
                }
            }
        }
        assert!(!hex.is_empty(), "в фикстуре не нашлось строки с байтами");
        let bytes: Vec<u8> = hex
            .as_bytes()
            .chunks(2)
            .map(|pair| {
                u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16)
                    .expect("в строке фикстуры обязаны быть только шестнадцатеричные цифры")
            })
            .collect();
        let captured = std::fs::read("../../tests/conformance/pdf/attach-platform.pdf")
            .expect("снимок платформы обязан лежать в дереве");
        assert_eq!(
            bytes, captured,
            "байты в фикстуре разошлись со снимком attach-platform.pdf"
        );
    }
}
