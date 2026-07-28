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

/// Разделители ИЗМЕРЕНЫ на 8.3.27 через `Формат(1234.5, "Л=<код>")`:
///
/// ```text
/// ru_RU -> 1 234,5     (NBSP и запятая)
/// en_US -> 1,234.5
/// de_DE -> 1.234,5
/// fr_FR -> 1 234,5
/// ja_JP -> 1,234.5
/// ```
///
/// НЕ ИЗМЕРЕНО(FMT.LOCALE.COVERAGE): что платформа делает с кодом, которого
/// не существует. Проба на это подвесила прогон (модальное окно), и до
/// повторного замера здесь остаётся ошибка — по крайней мере она не
/// притворяется, что локаль применена.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Locale {
    #[default]
    Ru,
    En,
    De,
    Fr,
    Ja,
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
            "de" => Some(Locale::De),
            "fr" => Some(Locale::Fr),
            "ja" => Some(Locale::Ja),
            _ => None,
        }
    }

    pub fn parse_or_error(code: &str) -> Result<Self, RtError> {
        Self::parse(code).ok_or_else(|| RtError::UnsupportedLocale(code.trim().to_string()))
    }

    /// Разделитель целой и дробной части (`ЧРД` по умолчанию).
    pub fn decimal_sep(self) -> char {
        match self {
            Locale::Ru | Locale::De | Locale::Fr => ',',
            Locale::En | Locale::Ja => '.',
        }
    }

    /// Разделитель групп разрядов (`ЧРГ` по умолчанию).
    pub fn group_sep(self) -> char {
        match self {
            // Французская группировка — тоже неразрывный пробел: замер дал
            // `1 234,5`, как и у русской.
            Locale::Ru | Locale::Fr => NBSP,
            Locale::En | Locale::Ja => ',',
            Locale::De => '.',
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
            // НЕ ИЗМЕРЕНО(FMT.LOCALE.BOOLEAN) для не-английских локалей:
            // замерено только `Л=en_US` -> Yes/No. Остальные показывают
            // английский текст, а не выдуманный перевод.
            (_, true) => "Yes",
            (_, false) => "No",
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
        assert_eq!(Locale::parse("zz_ZZ"), None);
        assert!(matches!(
            Locale::parse_or_error("zz_ZZ"),
            Err(RtError::UnsupportedLocale(code)) if code == "zz_ZZ"
        ));
    }

    /// Пять локалей, чьи разделители сняты с платформы.
    #[test]
    fn measured_locales_parse() {
        for (code, want) in [
            ("de_DE", Locale::De),
            ("fr_FR", Locale::Fr),
            ("ja_JP", Locale::Ja),
        ] {
            assert_eq!(Locale::parse(code), Some(want), "{code}");
        }
    }

    #[test]
    fn russian_group_separator_is_nbsp_not_a_plain_space() {
        assert_eq!(Locale::Ru.group_sep() as u32, 160);
        assert_ne!(Locale::Ru.group_sep(), ' ');
        assert_eq!(Locale::En.group_sep(), ',');
    }
}
