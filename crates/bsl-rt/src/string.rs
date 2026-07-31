use std::fmt;
use std::io::{self, Write};
use std::rc::Rc;

/// Строка BSL — код-юниты UTF-16, как в самой 1С (COM/Windows-строки), не
/// UTF-8. `СтрДлина` считает код-юниты, а не кодовые точки: суррогатная
/// пара (символ вне BMP, например эмодзи) даёт 2, не 1 — на UTF-8 все
/// индексные функции разъехались бы именно на таких символах.
///
/// Сравнение — по содержимому: строки в BSL, в отличие от `Массив`/
/// `Структура`, тип значения, а не ссылочный.
///
/// Интернирование коротких строк (брифом заявлено как способ свести
/// сравнение к сравнению указателей) сюда сознательно не входит — это
/// чисто оптимизационная надстройка над уже корректным посимвольным
/// сравнением ниже, и, как остальные оптимизации в этом проекте, ждёт
/// профилирования (см. план M10), а не добавляется заранее "на всякий".
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BslString(Rc<Vec<u16>>);

/// Сколько значений принимает `СтрШаблон`: `%1`..`%10`. Ограничение самой
/// 1С, а не этой реализации — поэтому константа, а не «сколько передали».
pub const MAX_TEMPLATE_ARGS: usize = 10;

/// Позиция первого вхождения `needle` в `hay`, начиная с `from`, или
/// `None`. Общий движок для `find`, `replace` и `split`: все трое раньше
/// разворачивали свой посимвольный цикл и одинаково платили за сравнение
/// срезов на каждой позиции.
///
/// Сначала быстрый пропуск до совпадения по ПЕРВОМУ код-юниту — простой
/// цикл поиска равного значения, который LLVM векторизует сам. Потом
/// дешёвая отсечка по ПОСЛЕДНЕМУ юниту: у неудачных кандидатов он не
/// совпадает почти всегда. Полное сравнение — только после обеих.
///
/// Пустая игла — `None`: у 1С поиск пустой строки ничего не находит.
fn find_from(hay: &[u16], needle: &[u16], from: usize) -> Option<usize> {
    if needle.is_empty() || needle.len() > hay.len() || from > hay.len() - needle.len() {
        return None;
    }
    let (first, last) = (needle[0], needle[needle.len() - 1]);
    let limit = hay.len() - needle.len();
    let mut start = from;
    while start <= limit {
        start += hay[start..=limit].iter().position(|u| *u == first)?;
        if hay[start + needle.len() - 1] == last && hay[start..start + needle.len()] == needle[..] {
            return Some(start);
        }
        start += 1;
    }
    None
}

/// Кодирует код-юниты UTF-16 в UTF-8 прямо в поток, без промежуточного
/// `String`. Некорректные суррогаты заменяются на U+FFFD — как в
/// `Display`.
fn encode_utf8(units: &[u16], writer: &mut impl Write) -> io::Result<()> {
    let mut out = [0u8; 1024];

    // CSV/JSON и большинство служебных строк состоят из ASCII. Для них
    // декодирование UTF-16 и проверка суррогатных пар не нужны.
    if units.iter().all(|unit| *unit <= 0x7f) {
        for chunk in units.chunks(out.len()) {
            for (dst, unit) in out.iter_mut().zip(chunk) {
                *dst = *unit as u8;
            }
            writer.write_all(&out[..chunk.len()])?;
        }
        return Ok(());
    }

    let mut used = 0;
    for decoded in char::decode_utf16(units.iter().copied()) {
        let ch = decoded.unwrap_or(char::REPLACEMENT_CHARACTER);
        let needed = ch.len_utf8();
        if used + needed > out.len() {
            writer.write_all(&out[..used])?;
            used = 0;
        }
        used += ch.encode_utf8(&mut out[used..]).len();
    }
    writer.write_all(&out[..used])
}

impl BslString {
    pub fn from_str(s: &str) -> Self {
        let units: Vec<u16> = s.encode_utf16().collect();
        Self::from_units(units)
    }

    fn from_units(units: Vec<u16>) -> Self {
        BslString(Rc::new(units))
    }

    pub fn units(&self) -> &[u16] {
        &self.0
    }

    /// Кодирует внутренние UTF-16 код-юниты прямо в переданный UTF-8
    /// поток. Промежуточный `String` не создаётся; некорректные суррогаты,
    /// как и в `Display`, заменяются на U+FFFD.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку, полученную от [`Write::write_all`].
    pub fn write_utf8(&self, writer: &mut impl Write) -> io::Result<()> {
        encode_utf8(&self.0, writer)
    }

    /// То же, но каждый перевод строки (U+000A) выходит парой CRLF.
    ///
    /// ИЗМЕРЕНО на 8.3.27: `ЗаписьТекста.Записать("A" + Символ(10) + "B")`
    /// кладёт на диск `41 0D0A 42`. Разделителем строк ВХОДНОГО текста у
    /// объекта по умолчанию считается ПС, разделителем строк ФАЙЛА —
    /// CRLF, и при записи первый заменяется вторым. Одиночный CR под это
    /// правило не подпадает и проходит как есть (`41 0D 42`), а явный
    /// CRLF даёт `41 0D 0D0A 42`: CR прошёл, LF развернулся.
    ///
    /// Отдельным методом, а не флагом в `write_utf8`: в поток вывода
    /// (`Сообщить`) никакой замены быть не должно.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку, полученную от [`Write::write_all`].
    pub fn write_utf8_crlf(&self, writer: &mut impl Write) -> io::Result<()> {
        let mut rest: &[u16] = &self.0;
        while let Some(at) = rest.iter().position(|u| *u == 0x000a) {
            encode_utf8(&rest[..at], writer)?;
            writer.write_all(b"\r\n")?;
            rest = &rest[at + 1..];
        }
        encode_utf8(rest, writer)
    }

    /// `СтрДлина`/`StrLen` — число код-юнитов UTF-16, НЕ кодовых точек.
    pub fn len_utf16(&self) -> usize {
        self.0.len()
    }

    pub fn concat(&self, other: &Self) -> Self {
        let mut v = Vec::with_capacity(self.0.len() + other.0.len());
        v.extend_from_slice(&self.0);
        v.extend_from_slice(&other.0);
        BslString::from_units(v)
    }

    /// `self + other`, но с правом ДОПИСАТЬ НА МЕСТЕ, если на буфер
    /// `self` больше никто не смотрит.
    ///
    /// Ради этого строка и хранится как `Rc<Vec<u16>>`, а не `Rc<[u16]>`:
    /// у вектора есть ёмкость, и рост амортизируется. Сборка текста в
    /// цикле (`Текст = Текст + Кусок` — самый частый способ строить текст
    /// в прикладном BSL) из квадратичной становится линейной.
    ///
    /// Изменение чужого значения этим не вводится: `Rc::get_mut` отдаёт
    /// буфер, только когда счётчик ссылок равен единице, то есть ни одна
    /// другая переменная его не видит. `Х = Х + Х` под это не подпадает
    /// (счётчик 2) и уходит на копирующий путь.
    ///
    /// Владение сюда приходит из VM: инструкция `Add` с совпадающими
    /// приёмником и левым операндом ЗАБИРАЕТ значение из регистра, а не
    /// копирует — регистр всё равно будет перезаписан результатом.
    pub fn append(mut self, other: &Self) -> Self {
        match Rc::get_mut(&mut self.0) {
            Some(buf) => {
                buf.extend_from_slice(&other.0);
                self
            }
            None => self.concat(other),
        }
    }

    /// `Сред`/`Mid`: `start_1based` — позиция первого символа (1 = начало
    /// строки), `len` — длина в код-юнитах. Индекс, режущий суррогатную
    /// пару пополам, даёт "битую" последовательность — так же ведёт себя и
    /// сама 1С (строка внутри неё — ровно такой же буфер код-юнитов), это
    /// не отсебятина реализации.
    pub fn substring(&self, start_1based: usize, len: usize) -> Self {
        let n = self.0.len();
        if start_1based == 0 || start_1based > n {
            return BslString::from_units(Vec::new());
        }
        let start = start_1based - 1;
        let end = (start + len).min(n);
        BslString::from_units(self.0[start..end].to_vec())
    }

    pub fn left(&self, len: usize) -> Self {
        self.substring(1, len)
    }

    pub fn right(&self, len: usize) -> Self {
        let n = self.0.len();
        let take = len.min(n);
        BslString::from_units(self.0[n - take..].to_vec())
    }

    /// Через `char` (декодируя код-юниты), не по код-юниту напрямую —
    /// иначе смена регистра могла бы разломать суррогатные пары.
    pub fn to_uppercase(&self) -> Self {
        BslString::from_str(&self.to_string().to_uppercase())
    }

    pub fn to_lowercase(&self) -> Self {
        BslString::from_str(&self.to_string().to_lowercase())
    }

    pub fn trim(&self) -> Self {
        BslString::from_str(self.to_string().trim())
    }

    /// `СокрЛ`/`TrimL` — обрезка только слева.
    pub fn trim_start(&self) -> Self {
        BslString::from_str(self.to_string().trim_start())
    }

    /// `СокрП`/`TrimR` — обрезка только справа.
    pub fn trim_end(&self) -> Self {
        BslString::from_str(self.to_string().trim_end())
    }

    /// `СтрНайти`/`StrFind` — позиция первого вхождения, 1-based, `0` если
    /// не найдено. Поиск и позиция — В КОД-ЮНИТАХ UTF-16, тех же, что
    /// считает `len_utf16`: `СтрНайти` обязан возвращать число, которое
    /// можно без пересчёта скормить в `Сред`/`Лев` (инвариант индексации).
    /// Поэтому сравнение идёт по срезам `[u16]`, а не по UTF-8 `str`, где
    /// байтовая позиция не совпала бы с юнитной для кириллицы.
    ///
    /// Пустая подстрока — `0` (не найдено), а не `1`: у 1С поиск пустой
    /// строки ничего не находит.
    pub fn find(&self, needle: &Self) -> usize {
        match find_from(&self.0, &needle.0, 0) {
            Some(at) => at + 1,
            None => 0,
        }
    }

    /// `СтрЗаменить`/`StrReplace` — все вхождения. Пустой `from` —
    /// возврат исходной строки без изменений (иначе замена вставляла бы
    /// `to` между каждой парой символов бесконечно).
    pub fn replace(&self, from: &Self, to: &Self) -> Self {
        if from.0.is_empty() {
            return self.clone();
        }
        let mut out: Vec<u16> = Vec::with_capacity(self.0.len());
        let mut i = 0;
        while let Some(at) = find_from(&self.0, &from.0, i) {
            out.extend_from_slice(&self.0[i..at]);
            out.extend_from_slice(&to.0);
            i = at + from.0.len();
        }
        out.extend_from_slice(&self.0[i..]);
        BslString::from_units(out)
    }

    /// `СтрРазделить`/`StrSplit` — по разделителю, пустые куски
    /// СОХРАНЯЮТСЯ (у 1С третий аргумент `ВключатьПустые` по умолчанию
    /// `Истина`; самого аргумента здесь пока нет). Пустой разделитель —
    /// вся строка одним элементом.
    pub fn split(&self, sep: &Self) -> Vec<Self> {
        if sep.0.is_empty() {
            return vec![self.clone()];
        }
        let mut parts = Vec::new();
        let mut start = 0;
        while let Some(at) = find_from(&self.0, &sep.0, start) {
            parts.push(BslString::from_units(self.0[start..at].to_vec()));
            start = at + sep.0.len();
        }
        parts.push(BslString::from_units(self.0[start..].to_vec()));
        parts
    }

    /// `СтрСоединить`/`StrConcat`.
    pub fn join(parts: &[Self], sep: &Self) -> Self {
        let mut out: Vec<u16> = Vec::new();
        for (i, p) in parts.iter().enumerate() {
            if i > 0 {
                out.extend_from_slice(&sep.0);
            }
            out.extend_from_slice(&p.0);
        }
        BslString::from_units(out)
    }

    /// Разбиение на СТРОКИ (в смысле "строк текста"): разделитель — `LF`,
    /// предшествующий ему `CR` съедается вместе с ним (`CRLF` — один
    /// перевод, не два). Одинокий `CR` разделителем НЕ считается — иначе
    /// строка с `CR` внутри давала бы разное число строк на разных
    /// платформах, а исходники BSL приходят и с той, и с другой
    /// разметкой.
    fn lines(&self) -> Vec<&[u16]> {
        const LF: u16 = 0x000A;
        const CR: u16 = 0x000D;
        let mut out = Vec::new();
        let mut start = 0;
        for i in 0..self.0.len() {
            if self.0[i] == LF {
                let end = if i > start && self.0[i - 1] == CR { i - 1 } else { i };
                out.push(&self.0[start..end]);
                start = i + 1;
            }
        }
        out.push(&self.0[start..]);
        out
    }

    /// `СтрЧислоСтрок`/`StrLineCount` — пустая строка — это одна строка
    /// (не ноль): текст без переводов строки состоит из одной строки.
    pub fn line_count(&self) -> usize {
        self.lines().len()
    }

    /// `СтрПолучитьСтроку`/`StrGetLine` — 1-based; номер вне диапазона
    /// даёт пустую строку (как `Сред` за границей), а не ошибку.
    pub fn line_at(&self, n_1based: usize) -> Self {
        if n_1based == 0 {
            return BslString::from_units(Vec::new());
        }
        match self.lines().get(n_1based - 1) {
            Some(l) => BslString::from_units(l.to_vec()),
            None => BslString::from_units(Vec::new()),
        }
    }

    /// `Символ`/`Char` — код -> строка. Код за пределами BMP даёт ДВА
    /// код-юнита (суррогатную пару), то есть `СтрДлина(Символ(128512)) = 2`
    /// — прямое следствие инварианта "длина в код-юнитах".
    pub fn from_char_code(code: u32) -> Option<Self> {
        let ch = char::from_u32(code)?;
        let mut buf = [0u16; 2];
        Some(BslString::from_units(ch.encode_utf16(&mut buf).to_vec()))
    }

    /// `КодСимвола`/`CharCode` — код символа на позиции `pos_1based`
    /// (позиция — в код-юнитах, как везде). `None` — позиция вне строки.
    ///
    /// `КодСимвола`/`CharCode` — код символа на позиции `pos_1based`
    /// (позиция — в код-юнитах, как везде).
    ///
    /// ИЗМЕРЕНО на 8.3.27: `КодСимвола("")` даёт **-1**, а не ошибку. То же
    /// самое здесь означает `None` — вызывающий превращает его в -1.
    /// Суррогатных пар в замере не оказалось вовсе: `Символ(128512)` на
    /// платформе возвращает ПУСТУЮ строку (см. `from_code`), поэтому
    /// вопроса «код-юнит или кодовая точка» на ней просто не существует.
    pub fn char_code_at(&self, pos_1based: usize) -> Option<u32> {
        if pos_1based == 0 {
            return None;
        }
        let i = pos_1based - 1;
        let unit = *self.0.get(i)? as u32;
        const HIGH: std::ops::Range<u32> = 0xD800..0xDC00;
        const LOW: std::ops::Range<u32> = 0xDC00..0xE000;
        if HIGH.contains(&unit) {
            if let Some(&next) = self.0.get(i + 1) {
                if LOW.contains(&(next as u32)) {
                    return Some(0x10000 + ((unit - 0xD800) << 10) + (next as u32 - 0xDC00));
                }
            }
        }
        Some(unit)
    }

    /// `СтрШаблон`/`StrTemplate` — подстановка `%1`..`%10`. `%%` обозначает
    /// литеральный `%`. Номер читается жадно до двух цифр, поэтому `%10` — это десятый
    /// параметр, а не первый со следующей за ним нулём.
    ///
    /// Номер вне `1..=values.len()` подставляется пустой строкой, а не
    /// падает: у шаблона и списка значений разные места происхождения
    /// (часто — конфигурация и код), и рассинхрон не повод ронять скрипт.
    pub fn template(&self, values: &[Self]) -> Self {
        const PERCENT: u16 = b'%' as u16;
        let src = &self.0;
        let mut out: Vec<u16> = Vec::with_capacity(src.len());
        let mut i = 0;
        while i < src.len() {
            if src[i] != PERCENT {
                out.push(src[i]);
                i += 1;
                continue;
            }
            match src.get(i + 1) {
                Some(&PERCENT) => {
                    out.push(PERCENT);
                    i += 2;
                }
                Some(&d) if digit(d).is_some() => {
                    let mut n = digit(d).unwrap() as usize;
                    let mut used = 2;
                    if let Some(&d2) = src.get(i + 2) {
                        if let Some(v) = digit(d2) {
                            // Только до `%10`: у 1С шаблон принимает ровно
                            // десять значений, `%11` — это `%1` и текст `1`.
                            let wide = n * 10 + v as usize;
                            if wide <= MAX_TEMPLATE_ARGS {
                                n = wide;
                                used = 3;
                            }
                        }
                    }
                    if n >= 1 {
                        if let Some(v) = values.get(n - 1) {
                            out.extend_from_slice(&v.0);
                        }
                    }
                    i += used;
                }
                // `%` не перед цифрой и не перед `%` — сам по себе символ.
                _ => {
                    out.push(PERCENT);
                    i += 1;
                }
            }
        }
        BslString::from_units(out)
    }
}

fn digit(unit: u16) -> Option<u32> {
    (0x0030..=0x0039)
        .contains(&unit)
        .then(|| unit as u32 - 0x0030)
}

impl fmt::Display for BslString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", String::from_utf16_lossy(&self.0))
    }
}

impl fmt::Debug for BslString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn surrogate_pair_counts_as_two_code_units() {
        // брифом явно указано: суррогатная пара даёт 2.
        let s = BslString::from_str("😀");
        assert_eq!(s.len_utf16(), 2);
        let s = BslString::from_str("a😀b");
        assert_eq!(s.len_utf16(), 4);
    }

    #[test]
    fn bmp_characters_count_one_per_code_unit() {
        assert_eq!(BslString::from_str("привет").len_utf16(), 6);
        assert_eq!(BslString::from_str("").len_utf16(), 0);
    }

    #[test]
    fn direct_utf8_write_matches_lossy_conversion_without_allocating_a_string() {
        let source = format!("{}😀конец", "я".repeat(600));
        let mut bytes = Vec::new();
        BslString::from_str(&source).write_utf8(&mut bytes).unwrap();
        assert_eq!(bytes, source.as_bytes());

        let malformed = BslString::from_units(vec![0xD800, b'x' as u16]);
        bytes.clear();
        malformed.write_utf8(&mut bytes).unwrap();
        assert_eq!(String::from_utf8(bytes).unwrap(), "\u{FFFD}x");
    }

    #[test]
    fn concat_matches_plus_operator_semantics() {
        let a = BslString::from_str("Привет, ");
        let b = BslString::from_str("мир!");
        assert_eq!(a.concat(&b).to_string(), "Привет, мир!");
    }

    #[test]
    fn left_right_mid_are_1_based_code_unit_slices() {
        let s = BslString::from_str("Привет");
        assert_eq!(s.left(3).to_string(), "При");
        assert_eq!(s.right(3).to_string(), "вет");
        assert_eq!(s.substring(2, 3).to_string(), "рив");
    }

    #[test]
    fn equality_is_by_content_not_identity() {
        let a = BslString::from_str("x");
        let b = BslString::from_str("x");
        assert_eq!(a, b);
    }

    #[test]
    fn case_conversion_does_not_break_on_non_ascii() {
        assert_eq!(BslString::from_str("привет").to_uppercase().to_string(), "ПРИВЕТ");
        assert_eq!(BslString::from_str("ПРИВЕТ").to_lowercase().to_string(), "привет");
    }

    fn s(t: &str) -> BslString {
        BslString::from_str(t)
    }

    #[test]
    fn find_is_1_based_in_utf16_units_not_utf8_bytes() {
        // Ключевое: у кириллицы байтовая позиция вдвое больше юнитной —
        // "вг" в "абвгд" начинается с 3-го СИМВОЛА и 5-го БАЙТА.
        assert_eq!(s("абвгд").find(&s("вг")), 3);
        assert_eq!(s("абвгд").find(&s("а")), 1);
        assert_eq!(s("абвгд").find(&s("д")), 5);
        assert_eq!(s("абвгд").find(&s("яя")), 0);
        assert_eq!(s("абв").find(&s("")), 0);
        assert_eq!(s("").find(&s("а")), 0);
    }

    #[test]
    fn find_agrees_with_mid_on_the_position_it_returns() {
        // Смысл единиц измерения: результат `СтрНайти` можно скормить в
        // `Сред` без пересчёта.
        let hay = s("а😀бвг");
        let pos = hay.find(&s("бв"));
        assert_eq!(hay.substring(pos, 2).to_string(), "бв");
    }

    #[test]
    fn replace_handles_all_occurrences_and_empty_needle() {
        assert_eq!(s("а-б-в").replace(&s("-"), &s("+")).to_string(), "а+б+в");
        assert_eq!(s("ааа").replace(&s("аа"), &s("б")).to_string(), "ба");
        assert_eq!(s("абв").replace(&s(""), &s("x")).to_string(), "абв");
        assert_eq!(s("абв").replace(&s("б"), &s("")).to_string(), "ав");
    }

    #[test]
    fn split_keeps_empty_pieces_and_round_trips_through_join() {
        let parts = s("а,,б").split(&s(","));
        let texts: Vec<String> = parts.iter().map(|p| p.to_string()).collect();
        assert_eq!(texts, vec!["а", "", "б"]);
        assert_eq!(BslString::join(&parts, &s(",")).to_string(), "а,,б");

        // Пустой разделитель — вся строка одним куском.
        assert_eq!(s("абв").split(&s("")).len(), 1);
        // Строка без разделителя — тоже один кусок, а не ноль.
        assert_eq!(s("абв").split(&s(",")).len(), 1);
        assert_eq!(s("").split(&s(",")).len(), 1);
    }

    #[test]
    fn line_helpers_treat_crlf_as_one_break_and_empty_text_as_one_line() {
        assert_eq!(s("а\r\nб\nв").line_count(), 3);
        assert_eq!(s("а\r\nб\nв").line_at(1).to_string(), "а");
        assert_eq!(s("а\r\nб\nв").line_at(2).to_string(), "б");
        assert_eq!(s("а\r\nб\nв").line_at(3).to_string(), "в");
        assert_eq!(s("").line_count(), 1);
        // Вне диапазона — пустая строка, не паника.
        assert_eq!(s("а").line_at(0).to_string(), "");
        assert_eq!(s("а").line_at(9).to_string(), "");
    }

    #[test]
    fn char_and_char_code_round_trip_including_outside_bmp() {
        assert_eq!(BslString::from_char_code(160).unwrap().len_utf16(), 1);
        assert_eq!(s("абв").char_code_at(2), Some('б' as u32));
        assert_eq!(s("абв").char_code_at(9), None);
        assert_eq!(s("абв").char_code_at(0), None);

        // Суррогатная пара: длина 2 (инвариант), но код — полная кодовая
        // точка, чтобы Символ/КодСимвола сходились туда-обратно.
        let emoji = BslString::from_char_code(128512).unwrap();
        assert_eq!(emoji.len_utf16(), 2);
        assert_eq!(emoji.char_code_at(1), Some(128512));
    }

    #[test]
    fn template_substitutes_by_number_and_escapes_double_percent() {
        let vals = vec![s("раз"), s("два")];
        assert_eq!(s("%1 и %2").template(&vals).to_string(), "раз и два");
        assert_eq!(s("100%%").template(&vals).to_string(), "100%");
        // Номер без значения — пусто, не паника.
        assert_eq!(s("[%9]").template(&vals).to_string(), "[]");
        // `%` не перед цифрой остаётся собой.
        assert_eq!(s("%а").template(&vals).to_string(), "%а");
        // Двузначный номер читается жадно только до %10.
        let ten: Vec<BslString> = (1..=10).map(|i| s(&i.to_string())).collect();
        assert_eq!(s("%10").template(&ten).to_string(), "10");
        assert_eq!(s("%11").template(&ten).to_string(), "11");
    }

    #[test]
    fn one_sided_trims_do_not_touch_the_other_side() {
        assert_eq!(s("  а  ").trim_start().to_string(), "а  ");
        assert_eq!(s("  а  ").trim_end().to_string(), "  а");
        assert_eq!(s("  а  ").trim().to_string(), "а");
    }
}
