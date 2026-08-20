//! Таблица диапазонов десятичных цифр (общая категория `Nd`).
//!
//! Единственный оставшийся потребитель — [`decimal_digit_value`],
//! грамматика строки замены: сами шаблоны теперь исполняет `fancy-regex`,
//! и `\p{Nd}`/`\p{L}` печатаются рендерером в его синтаксисе. Таблица
//! не берётся из `char::is_numeric`: то свойство ШИРЕ категории — оно
//! включает `Nl` и `No` (римская `Ⅻ`, надстрочная `²`), а платформа
//! цифрой после `$` считает именно `Nd`.
//!
//! Таблица СГЕНЕРИРОВАНА. Unicode 16.0.0 (`unicodedata` из CPython
//! 3.14.6). Команда генерации — ровно этот скрипт, его вывод вставлен
//! ниже без правок:
//!
//! ```python
//! import unicodedata
//! def ranges(pred):
//!     out, start, prev = [], None, None
//!     for cp in range(0x110000):
//!         if pred(cp):
//!             if start is None: start = prev = cp
//!             elif cp == prev + 1: prev = cp
//!             else: out.append((start, prev)); start = prev = cp
//!     if start is not None: out.append((start, prev))
//!     return out
//! cat = lambda cp: unicodedata.category(chr(cp))
//! print("DIGIT_RANGES",
//!       [(hex(a), hex(b)) for a, b in ranges(lambda cp: cat(cp) == "Nd")])
//! ```
//!
//! Таблица отсортирована по началу диапазона, диапазоны не пересекаются —
//! `partition_point` в [`decimal_digit_value`] полагается на это.
//! Инвариант проверяется тестом `the_ranges_are_sorted_and_disjoint`.

/// Числовое значение десятичной цифры — любой, не только ASCII.
///
/// Нужно грамматике строки замены: измерено, что после `$` платформа
/// считает цифрой и арабо-индийскую единицу U+0661 (`$` + U+0661 при двух
/// группах подставляет первую группу), а римскую единицу U+2170 категории
/// Nl — уже нет.
///
/// Значение считается как `(cp - начало) % 10`: каждый блок Nd — это ровно
/// десять точек подряд от нуля до девятки, а склеенные соседние блоки в
/// [`DIGIT_RANGES`] сохраняют это по построению. Инвариант «длина каждого
/// диапазона кратна десяти» проверяет тест
/// `every_digit_range_is_whole_blocks_of_ten`.
pub(crate) fn decimal_digit_value(cp: u32) -> Option<u32> {
    let index = DIGIT_RANGES.partition_point(|(from, _)| *from <= cp);
    let (from, to) = *DIGIT_RANGES.get(index.checked_sub(1)?)?;
    if cp > to {
        return None;
    }
    Some((cp - from) % 10)
}

pub(crate) const DIGIT_RANGES: &[(u32, u32)] = &[
    (0x0030, 0x0039),
    (0x0660, 0x0669),
    (0x06F0, 0x06F9),
    (0x07C0, 0x07C9),
    (0x0966, 0x096F),
    (0x09E6, 0x09EF),
    (0x0A66, 0x0A6F),
    (0x0AE6, 0x0AEF),
    (0x0B66, 0x0B6F),
    (0x0BE6, 0x0BEF),
    (0x0C66, 0x0C6F),
    (0x0CE6, 0x0CEF),
    (0x0D66, 0x0D6F),
    (0x0DE6, 0x0DEF),
    (0x0E50, 0x0E59),
    (0x0ED0, 0x0ED9),
    (0x0F20, 0x0F29),
    (0x1040, 0x1049),
    (0x1090, 0x1099),
    (0x17E0, 0x17E9),
    (0x1810, 0x1819),
    (0x1946, 0x194F),
    (0x19D0, 0x19D9),
    (0x1A80, 0x1A89),
    (0x1A90, 0x1A99),
    (0x1B50, 0x1B59),
    (0x1BB0, 0x1BB9),
    (0x1C40, 0x1C49),
    (0x1C50, 0x1C59),
    (0xA620, 0xA629),
    (0xA8D0, 0xA8D9),
    (0xA900, 0xA909),
    (0xA9D0, 0xA9D9),
    (0xA9F0, 0xA9F9),
    (0xAA50, 0xAA59),
    (0xABF0, 0xABF9),
    (0xFF10, 0xFF19),
    (0x104A0, 0x104A9),
    (0x10D30, 0x10D39),
    (0x10D40, 0x10D49),
    (0x11066, 0x1106F),
    (0x110F0, 0x110F9),
    (0x11136, 0x1113F),
    (0x111D0, 0x111D9),
    (0x112F0, 0x112F9),
    (0x11450, 0x11459),
    (0x114D0, 0x114D9),
    (0x11650, 0x11659),
    (0x116C0, 0x116C9),
    (0x116D0, 0x116E3),
    (0x11730, 0x11739),
    (0x118E0, 0x118E9),
    (0x11950, 0x11959),
    (0x11BF0, 0x11BF9),
    (0x11C50, 0x11C59),
    (0x11D50, 0x11D59),
    (0x11DA0, 0x11DA9),
    (0x11F50, 0x11F59),
    (0x16130, 0x16139),
    (0x16A60, 0x16A69),
    (0x16AC0, 0x16AC9),
    (0x16B50, 0x16B59),
    (0x16D70, 0x16D79),
    (0x1CCF0, 0x1CCF9),
    (0x1D7CE, 0x1D7FF),
    (0x1E140, 0x1E149),
    (0x1E2F0, 0x1E2F9),
    (0x1E4F0, 0x1E4F9),
    (0x1E5F1, 0x1E5FA),
    (0x1E950, 0x1E959),
    (0x1FBF0, 0x1FBF9),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_digit_range_is_whole_blocks_of_ten() {
        // На этом держится `decimal_digit_value`: склеенные диапазоны
        // (например 0x116D0..0x116E3) обязаны оставаться целым числом
        // десятичных блоков, иначе остаток от деления соврёт.
        for (from, to) in DIGIT_RANGES {
            assert_eq!(
                (to - from + 1) % 10,
                0,
                "диапазон {from:#x}..={to:#x} не кратен десяти"
            );
        }
        assert_eq!(decimal_digit_value(u32::from(b'7')), Some(7));
        assert_eq!(decimal_digit_value(0x0661), Some(1));
        assert_eq!(decimal_digit_value(0x2170), None);
        assert_eq!(decimal_digit_value(u32::from(b'x')), None);
    }

    #[test]
    fn the_ranges_are_sorted_and_disjoint() {
        let mut prev_end: Option<u32> = None;
        for &(from, to) in DIGIT_RANGES {
            assert!(from <= to, "перевёрнутый диапазон {from:#X}..{to:#X}");
            if let Some(end) = prev_end {
                assert!(
                    end < from,
                    "диапазоны идут не по возрастанию или сливаются: {end:#X} и {from:#X}"
                );
            }
            prev_end = Some(to);
        }
    }

    #[test]
    fn the_edges_of_the_ranges_answer_correctly() {
        // Края блоков и их соседи: `/` перед `0`, `:` после `9`; арабо-
        // индийская тройка внутри своего блока, надстрочная двойка (`No`)
        // и римская двенадцать (`Nl`) — вне категории; последняя точка
        // Unicode — вне последнего диапазона.
        assert_eq!(decimal_digit_value(u32::from(b'0')), Some(0));
        assert_eq!(decimal_digit_value(u32::from(b'9')), Some(9));
        assert_eq!(decimal_digit_value(0x0663), Some(3));
        assert_eq!(decimal_digit_value(u32::from(b'/')), None);
        assert_eq!(decimal_digit_value(u32::from(b':')), None);
        assert_eq!(decimal_digit_value(0x00B2), None);
        assert_eq!(decimal_digit_value(0x216B), None);
        assert_eq!(decimal_digit_value(0x10_FFFF), None);
    }
}
