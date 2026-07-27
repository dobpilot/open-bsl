//! Локаль форматирования — ключ `Л` форматной строки.
//!
//! Живёт в `bsl-rt`, а не в `bsl-format`, потому что от неё зависят и имена
//! месяцев (`date.rs`), и представление булева, и разделители числа: один
//! тип на всех потребителей, иначе `Л` пришлось бы разбирать в двух местах
//! по-разному.
//!
//! Поддержаны РОВНО ДВЕ локали. Полная локализация (сотни кодов, их данные
//! о разделителях, календарях и падежах месяцев) — это ICU, а не строчка в
//! `match`; браться за неё до замеров бессмысленно. Незнакомый код —
//! внятная ошибка, не молчаливый откат к русской: молчаливый откат выдал бы
//! правдоподобный, но неверный результат, и это худший вид расхождения.

use crate::RtError;

/// Разделитель групп разрядов по умолчанию в русской локали — NBSP
/// (U+00A0), НЕ обычный пробел. Глазами в исходниках не отличить, поэтому —
/// именованная константа: замер `КодСимвола` на платформе дал 160, не 32.
pub const NBSP: char = '\u{A0}';

/// НЕ ИЗМЕРЕНО(FMT.LOCALE.COVERAGE): какие коды локалей платформа понимает
/// в ключе `Л` и что делает с незнакомым. Здесь приняты `ru`/`ru_RU` и
/// `en`/`en_US` (регистр и разделитель `-`/`_` не важны), всё остальное —
/// ошибка.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Locale {
    #[default]
    Ru,
    En,
}

impl Locale {
    /// `Л=ru_RU`, `Л=en`, `Л=EN-US`. `None` — код не поддержан.
    pub fn parse(code: &str) -> Option<Self> {
        let code = code.trim().to_lowercase().replace('-', "_");
        // Значима только языковая часть: региона, который менял бы
        // разделители внутри одного языка, здесь всё равно нет.
        match code.split('_').next()? {
            "ru" => Some(Locale::Ru),
            "en" => Some(Locale::En),
            _ => None,
        }
    }

    pub fn parse_or_error(code: &str) -> Result<Self, RtError> {
        Self::parse(code).ok_or_else(|| RtError::UnsupportedLocale(code.trim().to_string()))
    }

    /// Разделитель целой и дробной части (`ЧРД` по умолчанию).
    pub fn decimal_sep(self) -> char {
        match self {
            Locale::Ru => ',',
            Locale::En => '.',
        }
    }

    /// Разделитель групп разрядов (`ЧРГ` по умолчанию).
    pub fn group_sep(self) -> char {
        match self {
            Locale::Ru => NBSP,
            Locale::En => ',',
        }
    }

    /// Представление `Булево`.
    ///
    /// НЕ ИЗМЕРЕНО(FMT.LOCALE.BOOLEAN): как выглядит `Истина` в английской
    /// локали — `Yes`, `True` или `1`. Взято `Yes`/`No` как зеркало
    /// измеренного русского `Да`/`Нет`. Само `Да` ИЗМЕРЕНО и правкам не
    /// подлежит.
    pub fn boolean_text(self, value: bool) -> &'static str {
        match (self, value) {
            (Locale::Ru, true) => "Да",
            (Locale::Ru, false) => "Нет",
            (Locale::En, true) => "Yes",
            (Locale::En, false) => "No",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locale_codes_are_case_and_separator_insensitive() {
        for code in ["ru", "RU", "ru_RU", "ru-ru"] {
            assert_eq!(Locale::parse(code), Some(Locale::Ru), "{code}");
        }
        for code in ["en", "en_US", "EN-us"] {
            assert_eq!(Locale::parse(code), Some(Locale::En), "{code}");
        }
    }

    #[test]
    fn unsupported_locale_is_an_error_not_a_silent_fallback() {
        assert_eq!(Locale::parse("de_DE"), None);
        assert!(matches!(
            Locale::parse_or_error("de_DE"),
            Err(RtError::UnsupportedLocale(code)) if code == "de_DE"
        ));
    }

    #[test]
    fn russian_group_separator_is_nbsp_not_a_plain_space() {
        assert_eq!(Locale::Ru.group_sep() as u32, 160);
        assert_ne!(Locale::Ru.group_sep(), ' ');
        assert_eq!(Locale::En.group_sep(), ',');
    }
}
