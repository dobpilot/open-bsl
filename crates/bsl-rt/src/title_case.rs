//! `ТРег`: титульный регистр по границам слов Unicode.
//!
//! Измерено пробой `tests/conformance/measure/measure-title-case.bsl`
//! (`measure-title-case.platform.txt`): платформа делит строку на слова по
//! правилам UAX #29 и в каждом слове переводит первую букву, имеющую регистр,
//! в титульный регистр Unicode, а остальные буквы — в строчные. Поэтому
//! `слово1слово` — одно слово, `1слово` даёт `1Слово`, `а.б` — `А.б`, а
//! `а..б` — `А..Б`. Титульный регистр отличается от прописного у диграфов
//! (`ǈ`), лигатур (`ß` → `Ss`, `ﬁ` → `Fi`) и греческих букв с
//! подписной йотой (`ᾳ` → `ᾼ`); остальное совпадает с `char::to_uppercase`.
//!
//! Классы границ сведены к тем, что меняют результат: у букв без регистра
//! (иероглифы, кана, иврит) место границы не видно, а иероглифы и кана
//! измеренно не соединяются с соседними буквами (`中b` → `中B`).

#[derive(Clone, Copy, PartialEq, Eq)]
enum WordClass {
    Letter,
    Numeric,
    MidLetter,
    MidNumLet,
    MidNum,
    ExtendNumLet,
    /// Знаки, которые продолжают предыдущий символ (UAX #29, правило WB4).
    Extend,
    Other,
}

fn word_class(c: char) -> WordClass {
    match c {
        ':' | '\u{00B7}' | '\u{0387}' | '\u{055F}' | '\u{05F4}' | '\u{2027}' | '\u{FE13}'
        | '\u{FE55}' | '\u{FF1A}' => WordClass::MidLetter,
        '.' | '\'' | '\u{2018}' | '\u{2019}' | '\u{2024}' | '\u{FE52}' | '\u{FF07}'
        | '\u{FF0E}' => WordClass::MidNumLet,
        ',' | ';' | '\u{037E}' | '\u{0589}' | '\u{060C}' | '\u{060D}' | '\u{066C}' | '\u{07F8}'
        | '\u{2044}' | '\u{FE10}' | '\u{FE14}' | '\u{FE50}' | '\u{FE54}' | '\u{FF0C}'
        | '\u{FF1B}' => WordClass::MidNum,
        '_'
        | '\u{202F}'
        | '\u{203F}'
        | '\u{2040}'
        | '\u{2054}'
        | '\u{FE33}'
        | '\u{FE34}'
        | '\u{FE4D}'..='\u{FE4F}'
        | '\u{FF3F}' => WordClass::ExtendNumLet,
        '\u{00AD}'
        | '\u{0300}'..='\u{036F}'
        | '\u{0483}'..='\u{0489}'
        | '\u{1AB0}'..='\u{1AFF}'
        | '\u{1DC0}'..='\u{1DFF}'
        | '\u{200C}'
        | '\u{200D}'
        | '\u{2060}'..='\u{2064}'
        | '\u{20D0}'..='\u{20FF}'
        | '\u{FE00}'..='\u{FE0F}'
        | '\u{FE20}'..='\u{FE2F}'
        | '\u{FEFF}' => WordClass::Extend,
        // Иероглифы и кана не входят в класс букв UAX #29.
        '\u{2E80}'..='\u{2FDF}'
        | '\u{3005}'..='\u{3007}'
        | '\u{3021}'..='\u{3029}'
        | '\u{3038}'..='\u{303B}'
        | '\u{3040}'..='\u{30FF}'
        | '\u{31F0}'..='\u{31FF}'
        | '\u{3400}'..='\u{4DBF}'
        | '\u{4E00}'..='\u{9FFF}'
        | '\u{F900}'..='\u{FAFF}'
        | '\u{FF66}'..='\u{FF9F}'
        | '\u{20000}'..='\u{3FFFF}' => WordClass::Other,
        c if c.is_numeric() => WordClass::Numeric,
        c if c.is_alphabetic() => WordClass::Letter,
        _ => WordClass::Other,
    }
}

/// Номера символов, с которых начинается новое слово.
fn word_starts(chars: &[char]) -> Vec<bool> {
    let classes: Vec<WordClass> = chars.iter().map(|&c| word_class(c)).collect();
    // Знаки WB4 прозрачны: решение о границе принимается между соседними
    // значащими символами, а сами знаки границы не открывают.
    let significant: Vec<usize> = (0..chars.len())
        .filter(|&index| index == 0 || classes[index] != WordClass::Extend)
        .collect();
    let mut starts = vec![false; chars.len()];
    for (position, &index) in significant.iter().enumerate() {
        if position == 0 {
            starts[index] = true;
            continue;
        }
        let class = |offset: isize| {
            let at = position as isize + offset;
            usize::try_from(at)
                .ok()
                .and_then(|at| significant.get(at))
                .map(|&index| classes[index])
        };
        let (before2, before, current, after) = (class(-2), class(-1), class(0), class(1));
        use WordClass::*;
        let joined = match (before, current) {
            (Some(Letter | Numeric), Some(Letter | Numeric)) => true,
            (Some(Letter), Some(MidLetter | MidNumLet)) => after == Some(Letter),
            (Some(MidLetter | MidNumLet), Some(Letter)) => before2 == Some(Letter),
            (Some(Numeric), Some(MidNum | MidNumLet)) => after == Some(Numeric),
            (Some(MidNum | MidNumLet), Some(Numeric)) => before2 == Some(Numeric),
            (Some(Letter | Numeric | ExtendNumLet), Some(ExtendNumLet)) => true,
            (Some(ExtendNumLet), Some(Letter | Numeric)) => true,
            _ => false,
        };
        starts[index] = !joined;
    }
    starts
}

fn is_cased(c: char) -> bool {
    c.is_lowercase() || c.is_uppercase() || title_special(c).is_some()
}

/// Титульные формы, отличающиеся от `char::to_uppercase` (`UnicodeData.txt`
/// и `SpecialCasing.txt`).
fn title_special(c: char) -> Option<&'static str> {
    Some(match c {
        'ß' => "Ss",
        'ﬀ' => "Ff",
        'ﬁ' => "Fi",
        'ﬂ' => "Fl",
        'ﬃ' => "Ffi",
        'ﬄ' => "Ffl",
        'ﬅ' | 'ﬆ' => "St",
        'և' => "Եւ",
        'ﬓ' => "Մն",
        'ﬔ' => "Մե",
        'ﬕ' => "Մի",
        'ﬖ' => "Վն",
        'ﬗ' => "Մխ",
        'Ǆ' | 'ǅ' | 'ǆ' => "ǅ",
        'Ǉ' | 'ǈ' | 'ǉ' => "ǈ",
        'Ǌ' | 'ǋ' | 'ǌ' => "ǋ",
        'Ǳ' | 'ǲ' | 'ǳ' => "ǲ",
        'ᾲ' => "Ὰ\u{0345}",
        'ᾴ' => "Ά\u{0345}",
        'ᾷ' => "Α\u{0342}\u{0345}",
        'ῂ' => "Ὴ\u{0345}",
        'ῄ' => "Ή\u{0345}",
        'ῇ' => "Η\u{0342}\u{0345}",
        'ῲ' => "Ὼ\u{0345}",
        'ῴ' => "Ώ\u{0345}",
        'ῷ' => "Ω\u{0342}\u{0345}",
        _ => return greek_iota_title(c),
    })
}

/// Греческие буквы с подписной йотой: титульная форма — та же буква с
/// прописной основой (`ᾳ` → `ᾼ`), а не пара `ΑΙ` прописного регистра.
fn greek_iota_title(c: char) -> Option<&'static str> {
    const TITLES: [&str; 51] = [
        "ᾈ", "ᾉ", "ᾊ", "ᾋ", "ᾌ", "ᾍ", "ᾎ", "ᾏ", "ᾈ", "ᾉ", "ᾊ", "ᾋ", "ᾌ", "ᾍ", "ᾎ", "ᾏ", "ᾘ", "ᾙ",
        "ᾚ", "ᾛ", "ᾜ", "ᾝ", "ᾞ", "ᾟ", "ᾘ", "ᾙ", "ᾚ", "ᾛ", "ᾜ", "ᾝ", "ᾞ", "ᾟ", "ᾨ", "ᾩ", "ᾪ", "ᾫ",
        "ᾬ", "ᾭ", "ᾮ", "ᾯ", "ᾨ", "ᾩ", "ᾪ", "ᾫ", "ᾬ", "ᾭ", "ᾮ", "ᾯ", "ᾼ", "ῌ", "ῼ",
    ];
    let index = match c {
        '\u{1F80}'..='\u{1FAF}' => c as usize - 0x1F80,
        'ᾳ' | 'ᾼ' => 48,
        'ῃ' | 'ῌ' => 49,
        'ῳ' | 'ῼ' => 50,
        _ => return None,
    };
    Some(TITLES[index])
}

fn push_title(out: &mut String, c: char) {
    match title_special(c) {
        Some(title) => out.push_str(title),
        None => out.extend(c.to_uppercase()),
    }
}

/// Переводит строку в титульный регистр по правилам `ТРег`.
pub fn title_case(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let starts = word_starts(&chars);
    let mut out = String::with_capacity(text.len());
    let mut word_start = 0;
    while word_start < chars.len() {
        let word_end = (word_start + 1..chars.len())
            .find(|&index| starts[index])
            .unwrap_or(chars.len());
        let word = &chars[word_start..word_end];
        match word.iter().position(|&c| is_cased(c)) {
            Some(first) => {
                out.extend(&word[..first]);
                push_title(&mut out, word[first]);
                // Строчные считаются по всему слову: окончательная сигма
                // зависит от букв перед ней (`ΟΔΟΣ` → `Οδος`). Отображение
                // в строчные контекстно только у сигмы и меняет один символ
                // на один, поэтому префикс первой буквы отрезается по числу
                // символов её строчной формы.
                let tail: String = word[first..].iter().collect();
                let skip = word[first].to_lowercase().count();
                out.extend(tail.to_lowercase().chars().skip(skip));
            }
            None => out.extend(word),
        }
        word_start = word_end;
    }
    out
}

#[cfg(test)]
#[path = "title_case_tests.rs"]
mod tests;
