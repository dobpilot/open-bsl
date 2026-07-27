//! `Формат`/`Строка`/`Число` — язык форматных строк: числовые ключи (`ЧГ`,
//! `ЧРД`, `ЧРГ`, `ЧДЦ`) и ключи даты (`ДФ` — шаблон, `ДЛФ` — длинный
//! локальный формат). Остальные ключи (`Л`, `ЧН`, ...) сюда пока не входят.
//!
//! Само форматирование даты живёт в `bsl-rt::date` (чистая календарная
//! логика, без локали чисел), здесь только разбор форматной строки и выбор
//! между `ДФ`, `ДЛФ` и представлением по умолчанию.
//!
//! `Строка()` — это `Формат()` без явной форматной строки (значения по
//! умолчанию ниже); отдельной "простой" функции строкового представления
//! числа сознательно нет — только один путь через `format_number`.

use bsl_number::BslNumber;
use bsl_rt::BslValue;

/// Разделитель групп разрядов по умолчанию — NBSP (U+00A0), НЕ обычный
/// пробел. Глазами в исходниках не отличить, поэтому — именованная
/// константа: замер `КодСимвола` на платформе дал 160, не 32.
pub const NBSP: char = '\u{A0}';

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NumberFormat {
    /// `ЧГ` — группировать ли разряды тысяч (по умолчанию да).
    pub group: bool,
    /// `ЧРГ` — разделитель групп разрядов.
    pub group_sep: char,
    /// `ЧРД` — разделитель целой и дробной части.
    pub decimal_sep: char,
    /// `ЧДЦ` — число дробных разрядов. `None` — не заданы: дробные разряды
    /// не обрезаются (измерено: по умолчанию `ЧДЦ` не задан).
    pub frac_digits: Option<u32>,
}

impl Default for NumberFormat {
    fn default() -> Self {
        NumberFormat {
            group: true,
            group_sep: NBSP,
            decimal_sep: ',',
            frac_digits: None,
        }
    }
}

/// Форматирование ДАТЫ: `ДФ` (шаблон) и `ДЛФ` (длинный локальный формат).
/// `None` в обоих полях — формат по умолчанию: его выбирает `bsl_rt`, и он
/// сам открытый вопрос — НЕ ИЗМЕРЕНО(FMT.DATE.DEFAULT), см.
/// `bsl_rt::date::DEFAULT_PATTERN`.
///
/// Оба ключа сразу — `ДФ` выигрывает: явный шаблон конкретнее готового
/// локального формата.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DateFormat {
    /// `ДФ='дд.ММ.гггг'`.
    pub pattern: Option<String>,
    /// `ДЛФ=Д`.
    pub long: Option<String>,
}

/// Разбивает форматную строку на пары `ключ=значение` по `;`, НЕ разрезая
/// внутри одинарных кавычек: `ДФ='дд.ММ.гггг'` кавычки нужны как раз
/// потому, что шаблон может содержать что угодно, включая `;`.
fn spec_parts(spec: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    for ch in spec.chars() {
        match ch {
            '\'' => {
                in_quotes = !in_quotes;
                current.push(ch);
            }
            ';' if !in_quotes => {
                out.push(std::mem::take(&mut current));
            }
            _ => current.push(ch),
        }
    }
    out.push(current);

    out.into_iter()
        .filter_map(|part| {
            let part = part.trim();
            let (key, val) = part.split_once('=')?;
            let val = val.trim();
            // Значение может быть в одинарных кавычках — снимаем ровно
            // одну пару, содержимое отдаём как есть.
            let val = val
                .strip_prefix('\'')
                .and_then(|v| v.strip_suffix('\''))
                .unwrap_or(val);
            Some((key.trim().to_uppercase(), val.to_string()))
        })
        .collect()
}

/// Разбирает из форматной строки то, что относится к ДАТЕ. Ключи, не
/// относящиеся к дате (`ЧГ`, `ЧРД`, ...), здесь игнорируются — ровно
/// симметрично тому, как `parse_number_format` игнорирует `ДФ`.
pub fn parse_date_format(spec: &str) -> DateFormat {
    let mut fmt = DateFormat::default();
    for (key, val) in spec_parts(spec) {
        match key.as_str() {
            "ДФ" | "DF" => fmt.pattern = Some(val),
            "ДЛФ" | "DLF" => fmt.long = Some(val),
            _ => {}
        }
    }
    fmt
}

/// Разбирает форматную строку вида `"ЧГ=0; ЧРД=."` (регистр ключей и
/// пробелы вокруг `;`/`=` не важны). Неизвестные/неприменимые к числу ключи
/// (`ДФ`, `Л`, ...) молча игнорируются — как и в самой платформе.
pub fn parse_number_format(spec: &str) -> NumberFormat {
    let mut fmt = NumberFormat::default();
    for (key, val) in spec_parts(spec) {
        let val = val.as_str();
        match key.as_str() {
            "ЧГ" => fmt.group = val != "0",
            "ЧРД" => {
                if let Some(c) = val.chars().next() {
                    fmt.decimal_sep = c;
                }
            }
            "ЧРГ" => {
                fmt.group_sep = val.chars().next().unwrap_or(' ');
            }
            "ЧДЦ" => {
                if let Ok(n) = val.parse::<u32>() {
                    fmt.frac_digits = Some(n);
                }
            }
            _ => {}
        }
    }
    fmt
}

/// Форматирует число по заданным правилам. Округление (при `ЧДЦ`) —
/// половина-вверх в decimal (`BslNumber::round_to_scale`), не через f64.
pub fn format_number(n: &BslNumber, fmt: &NumberFormat) -> String {
    let n = match fmt.frac_digits {
        Some(d) => n.round_to_scale(d as i32),
        None => n.clone(),
    };
    let canonical = n.to_canonical();
    let (sign, body): (&str, &str) = match canonical.strip_prefix('-') {
        Some(rest) => ("-", rest),
        None => ("", canonical.as_str()),
    };
    let (int_part, frac_part) = match body.split_once('.') {
        Some((i, f)) => (i, Some(f)),
        None => (body, None),
    };

    let int_part = if fmt.group {
        group_digits(int_part, fmt.group_sep)
    } else {
        int_part.to_string()
    };

    let frac_part = match (frac_part, fmt.frac_digits) {
        (Some(f), Some(d)) => {
            let mut f = f.to_string();
            while (f.len() as u32) < d {
                f.push('0');
            }
            Some(f)
        }
        (Some(f), None) => Some(f.to_string()),
        (None, Some(d)) if d > 0 => Some("0".repeat(d as usize)),
        (None, _) => None,
    };

    let mut out = String::new();
    out.push_str(sign);
    out.push_str(&int_part);
    if let Some(f) = frac_part {
        out.push(fmt.decimal_sep);
        out.push_str(&f);
    }
    out
}

fn group_digits(digits: &str, sep: char) -> String {
    let bytes = digits.as_bytes();
    let n = bytes.len();
    let mut out = String::with_capacity(n + n / 3);
    for (i, b) in bytes.iter().enumerate() {
        if i > 0 && (n - i) % 3 == 0 {
            out.push(sep);
        }
        out.push(*b as char);
    }
    out
}

/// `Строка(значение)` / `Формат(значение, spec)`. `spec = None` — правила
/// по умолчанию (`Строка()` это `Формат()` без явной форматной строки).
/// Форматная строка имеет эффект на `Число` (ключи `ЧГ`/`ЧРД`/`ЧРГ`/`ЧДЦ`)
/// и на `Дата` (`ДФ`/`ДЛФ`); у остальных типов применимых ключей пока нет,
/// их текстовое представление уже верно само по себе (`Display` в
/// `bsl-rt`: `Да`/`Нет`, `Массив`, пустая строка для `Неопределено`, ...).
pub fn format_value(v: &BslValue, spec: Option<&str>) -> String {
    match v {
        BslValue::Number(n) => {
            let fmt = spec.map(parse_number_format).unwrap_or_default();
            format_number(n, &fmt)
        }
        BslValue::Date(d) => {
            let fmt = spec.map(parse_date_format).unwrap_or_default();
            match (&fmt.pattern, &fmt.long) {
                // `ДФ` конкретнее `ДЛФ`, поэтому при обоих ключах
                // выигрывает шаблон.
                (Some(p), _) => bsl_rt::format_date_pattern(*d, p),
                (None, Some(code)) => bsl_rt::format_date_long(*d, code),
                // Без ключей даты — представление по умолчанию, то же, что
                // даёт `Строка()` (оно НЕ ИЗМЕРЕНО(FMT.DATE.DEFAULT), см.
                // `bsl_rt::date`).
                (None, None) => d.to_string(),
            }
        }
        other => other.to_string(),
    }
}

/// `Число(строка)` — обратный разбор: понимает разделитель групп (по
/// умолчанию NBSP, но обычный пробел тоже принимается — терпимее, чем
/// платформа, но безопасно) и разделитель дробной части. Именно это и
/// делает возможным `Число(Строка(1000000)) = 1000000`.
pub fn parse_number(s: &str, fmt: &NumberFormat) -> Result<BslNumber, bsl_number::NumError> {
    let mut cleaned = String::with_capacity(s.len());
    for ch in s.trim().chars() {
        if ch == fmt.group_sep || ch == NBSP || ch == ' ' {
            continue;
        }
        if ch == fmt.decimal_sep {
            cleaned.push('.');
        } else {
            cleaned.push(ch);
        }
    }
    BslNumber::parse_canonical(&cleaned)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn n(s: &str) -> BslNumber {
        BslNumber::parse_canonical(s).unwrap()
    }

    #[test]
    fn default_format_groups_with_nbsp_and_comma() {
        // Строка(1000.5) -> "1 000,5" (группировка даже при дробной части).
        assert_eq!(
            format_number(&n("1000.5"), &NumberFormat::default()),
            format!("1{NBSP}000,5")
        );
        assert_eq!(
            format_number(&n("1000000"), &NumberFormat::default()),
            format!("1{NBSP}000{NBSP}000")
        );
    }

    #[test]
    fn nbsp_is_u00a0_not_a_plain_space() {
        assert_eq!(NBSP as u32, 160);
        assert_ne!(NBSP, ' ');
    }

    #[test]
    fn canonical_form_recipe_matches_to_canonical_directly() {
        // Формат(x, "ЧГ=0; ЧРД=.") — группы подавлены, разделитель точка:
        // должно совпасть с самим to_canonical (это и есть основа
        // дифф-харнесса из брифа).
        let fmt = parse_number_format("ЧГ=0; ЧРД=.");
        for s in ["1000000", "-42.5", "0.333333333333333333333333333"] {
            assert_eq!(format_number(&n(s), &fmt), n(s).to_canonical());
        }
    }

    #[test]
    fn round_trip_through_default_format_and_parse() {
        // Число(Строка(1000000)) -> 1000000: NBSP понимается обратно.
        let s = format_number(&n("1000000"), &NumberFormat::default());
        let back = parse_number(&s, &NumberFormat::default()).unwrap();
        assert_eq!(back, n("1000000"));
    }

    #[test]
    fn frac_digits_rounds_in_decimal_half_up() {
        let fmt = NumberFormat {
            frac_digits: Some(2),
            ..NumberFormat::default()
        };
        assert_eq!(format_number(&n("2.675"), &fmt), "2,68");
    }

    #[test]
    fn frac_digits_pads_with_zeros_when_value_has_fewer() {
        let fmt = NumberFormat {
            group: false,
            frac_digits: Some(2),
            ..NumberFormat::default()
        };
        assert_eq!(format_number(&n("5"), &fmt), "5,00");
    }

    #[test]
    fn negative_numbers_group_correctly() {
        assert_eq!(
            format_number(&n("-1234567"), &NumberFormat::default()),
            format!("-1{NBSP}234{NBSP}567")
        );
    }

    #[test]
    fn date_keys_survive_a_semicolon_inside_the_quoted_pattern() {
        // Ради чего разбиение на пары кавычко-осведомлённое: `;` внутри
        // шаблона — часть шаблона, а не разделитель ключей.
        let fmt = parse_date_format("ДФ='дд;ММ'");
        assert_eq!(fmt.pattern.as_deref(), Some("дд;ММ"));
        // Кавычки снимаются ровно одной парой, содержимое не трогается.
        let fmt = parse_date_format("ДФ='дд.ММ.гггг'; ЧГ=0");
        assert_eq!(fmt.pattern.as_deref(), Some("дд.ММ.гггг"));
        assert_eq!(fmt.long, None);
        // Числовые ключи из той же строки по-прежнему разбираются.
        assert!(!parse_number_format("ДФ='дд.ММ.гггг'; ЧГ=0").group);
    }

    #[test]
    fn date_format_prefers_the_explicit_pattern_over_the_long_code() {
        let d = bsl_rt::BslDate::from_civil(2024, 1, 15, 10, 30, 0).unwrap();
        let v = BslValue::Date(d);
        assert_eq!(format_value(&v, Some("ДФ='гггг'")), "2024");
        assert_eq!(format_value(&v, Some("ДЛФ=В")), "10:30:00");
        // Оба ключа сразу — шаблон конкретнее.
        assert_eq!(format_value(&v, Some("ДЛФ=В; ДФ='гггг'")), "2024");
        // Без ключей даты — представление по умолчанию, то же, что Строка().
        assert_eq!(format_value(&v, Some("ЧГ=0")), d.to_string());
        assert_eq!(format_value(&v, None), d.to_string());
    }

    #[test]
    fn non_number_values_format_via_display() {
        assert_eq!(format_value(&BslValue::Boolean(true), None), "Да");
        assert_eq!(format_value(&BslValue::Undefined, None), "");
    }
}
