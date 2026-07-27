//! `Формат`/`Строка`/`Число` — язык форматных строк.
//!
//! Числовые ключи: `ЧГ` (группировать разряды), `ЧРГ`/`ЧРД` (разделители),
//! `ЧДЦ` (дробных разрядов), `ЧЦ` (всего разрядов), `ЧН` (текст на нуле),
//! `ЧВН` (ведущие нули), `ЧС` (сдвиг разрядов). Булевы: `БИ`/`БЛ`. Даты:
//! `ДФ` (шаблон) и `ДЛФ` (длинный локальный формат). Плюс `Л` — локаль,
//! которая задаёт УМОЛЧАНИЯ для разделителей, представления булева и имён
//! месяцев; явный ключ всегда сильнее локали.
//!
//! Ни один ключ, кроме `ЧГ`/`ЧРД`/`ЧРГ`/`ЧДЦ` и `ДФ`/`ДЛФ`, с платформой
//! не сверялся: каждый из них помечен `FMT.*` в реестре открытых вопросов
//! (`bsl_rt::open_questions`) и имеет строку в
//! `tests/conformance/measure/measure-all.bsl`. Реализация сделана по
//! здравому смыслу и синтакс-помощнику — это заготовка под замер, а не
//! знание.
//!
//! Само форматирование даты живёт в `bsl-rt::date` (чистая календарная
//! логика), здесь только разбор форматной строки и выбор между `ДФ`, `ДЛФ`
//! и представлением по умолчанию.
//!
//! `Строка()` — это `Формат()` без явной форматной строки (значения по
//! умолчанию ниже); отдельной "простой" функции строкового представления
//! числа сознательно нет — только один путь через `format_number`.

use bsl_number::BslNumber;
use bsl_rt::{BslValue, Locale, RtError, RtResult};

/// Разделитель групп разрядов русской локали — NBSP (U+00A0), НЕ обычный
/// пробел (замер `КодСимвола` на платформе дал 160, не 32). Определение
/// живёт в `bsl-rt`, потому что от локали зависят и разделители, и имена
/// месяцев; здесь — реэкспорт, чтобы `bsl_format::NBSP` продолжал значить
/// то же, что и раньше.
pub use bsl_rt::NBSP;

#[derive(Debug, Clone, PartialEq)]
pub struct NumberFormat {
    /// `Л` — локаль; задаёт умолчания для разделителей.
    pub locale: Locale,
    /// `ЧГ` — группировать ли разряды тысяч (по умолчанию да).
    pub group: bool,
    /// `ЧРГ` — разделитель групп разрядов.
    pub group_sep: char,
    /// `ЧРД` — разделитель целой и дробной части.
    pub decimal_sep: char,
    /// `ЧДЦ` — число дробных разрядов. `None` — не заданы: дробные разряды
    /// не обрезаются (измерено: по умолчанию `ЧДЦ` не задан).
    pub frac_digits: Option<u32>,
    /// `ЧЦ` — общее число разрядов.
    ///
    /// НЕ ИЗМЕРЕНО(FMT.NUM.TOTAL_DIGITS): считает ли платформа разряды
    /// целой части вместе с дробными (взято ДА), и что она делает, когда
    /// одна целая часть уже длиннее `ЧЦ` — печатает как есть (взято),
    /// заполняет поле звёздочками или обрезает старшие разряды. Здесь `ЧЦ`
    /// работает как потолок дробной части: `Формат(123.456, "ЧЦ=5")` даёт
    /// `123,46`, а `Формат(123456, "ЧЦ=3")` — целое без изменений.
    pub total_digits: Option<u32>,
    /// `ЧН` — что печатать вместо нуля.
    ///
    /// НЕ ИЗМЕРЕНО(FMT.NUM.ZERO_TEXT): применяется ли `ЧН` к значению,
    /// ставшему нулём ПОСЛЕ округления по `ЧДЦ` (взято ДА: проверка идёт по
    /// уже отформатированному числу), и что значит `ЧН=` без значения
    /// (взята пустая строка).
    pub zero_text: Option<String>,
    /// `ЧВН` — выводить ведущие нули.
    ///
    /// НЕ ИЗМЕРЕНО(FMT.NUM.LEADING_ZEROS): до какой ширины дополнять.
    /// Взято «до `ЧЦ` минус дробные разряды», то есть без `ЧЦ` ключ не
    /// делает ничего; альтернатива — своя ширина у самого `ЧВН`.
    pub leading_zeros: bool,
    /// `ЧС` — сдвиг разрядов.
    ///
    /// НЕ ИЗМЕРЕНО(FMT.NUM.SHIFT): направление сдвига и знак. Взято
    /// «`ЧС=n` делит на `10^n`» (`Формат(1234, "ЧС=3")` -> `1,234`),
    /// отрицательное `n` умножает. Сдвиг делается ТОЧНЫМ умножением на
    /// `10^-n`, а не делением: деление обрезало бы результат до 27 знаков.
    pub shift: i32,
}

impl Default for NumberFormat {
    fn default() -> Self {
        NumberFormat::for_locale(Locale::default())
    }
}

impl NumberFormat {
    /// Умолчания локали: разделители берутся из неё, остальное — общее.
    pub fn for_locale(locale: Locale) -> Self {
        NumberFormat {
            locale,
            group: true,
            group_sep: locale.group_sep(),
            decimal_sep: locale.decimal_sep(),
            frac_digits: None,
            total_digits: None,
            zero_text: None,
            leading_zeros: false,
            shift: 0,
        }
    }
}

/// Форматирование БУЛЕВА: `БИ`/`БЛ` — тексты для истины и лжи, `Л` —
/// локаль, дающая их умолчания.
///
/// НЕ ИЗМЕРЕНО(FMT.BOOLEAN.TRUE_TEXT) и
/// НЕ ИЗМЕРЕНО(FMT.BOOLEAN.FALSE_TEXT): что происходит, когда задан только
/// один из двух ключей. Взято «второй остаётся локальным умолчанием» —
/// `Формат(Ложь, "БИ=ага")` печатает `Нет`.
#[derive(Debug, Clone, PartialEq)]
pub struct BooleanFormat {
    pub locale: Locale,
    pub true_text: Option<String>,
    pub false_text: Option<String>,
}

impl BooleanFormat {
    pub fn text(&self, value: bool) -> String {
        let explicit = if value {
            self.true_text.as_deref()
        } else {
            self.false_text.as_deref()
        };
        explicit
            .map(str::to_string)
            .unwrap_or_else(|| self.locale.boolean_text(value).to_string())
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
    /// `Л` — от неё зависят имена месяцев и дней недели.
    pub locale: Locale,
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

/// Локаль форматной строки: ключ `Л`. Разбирается ПЕРВЫМ проходом, до
/// остальных ключей — она задаёт умолчания, поверх которых те ложатся.
///
/// НЕ ИЗМЕРЕНО(FMT.LOCALE.KEY): понимает ли платформа этот ключ под именем
/// `Л`/`L` и что означает пустое значение. Взято: `Л=<код>` — локаль,
/// отсутствие ключа — русская.
fn parse_locale(parts: &[(String, String)]) -> RtResult<Locale> {
    for (key, val) in parts {
        if key == "Л" || key == "L" {
            return Locale::parse_or_error(val);
        }
    }
    Ok(Locale::default())
}

/// Разбирает из форматной строки то, что относится к ДАТЕ. Ключи, не
/// относящиеся к дате (`ЧГ`, `ЧРД`, ...), здесь игнорируются — ровно
/// симметрично тому, как `parse_number_format` игнорирует `ДФ`. Кроме `Л`:
/// она общая для всех типов.
pub fn parse_date_format(spec: &str) -> RtResult<DateFormat> {
    let parts = spec_parts(spec);
    let mut fmt = DateFormat {
        locale: parse_locale(&parts)?,
        ..DateFormat::default()
    };
    for (key, val) in parts {
        match key.as_str() {
            "ДФ" | "DF" => fmt.pattern = Some(val),
            "ДЛФ" | "DLF" => fmt.long = Some(val),
            _ => {}
        }
    }
    Ok(fmt)
}

/// Разбирает булевы ключи `БИ`/`БЛ` (плюс общую `Л`).
pub fn parse_boolean_format(spec: &str) -> RtResult<BooleanFormat> {
    let parts = spec_parts(spec);
    let mut fmt = BooleanFormat {
        locale: parse_locale(&parts)?,
        true_text: None,
        false_text: None,
    };
    for (key, val) in parts {
        match key.as_str() {
            "БИ" | "BT" => fmt.true_text = Some(val),
            "БЛ" | "BF" => fmt.false_text = Some(val),
            _ => {}
        }
    }
    Ok(fmt)
}

/// Разбирает форматную строку вида `"ЧГ=0; ЧРД=."` (регистр ключей и
/// пробелы вокруг `;`/`=` не важны). Неизвестные/неприменимые к числу ключи
/// (`ДФ`, `БИ`, ...) молча игнорируются — как и в самой платформе.
///
/// Ошибка возможна ровно одна — незнакомая локаль в `Л`.
pub fn parse_number_format(spec: &str) -> RtResult<NumberFormat> {
    let parts = spec_parts(spec);
    let mut fmt = NumberFormat::for_locale(parse_locale(&parts)?);
    for (key, val) in &parts {
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
            "ЧЦ" => {
                if let Ok(n) = val.parse::<u32>() {
                    fmt.total_digits = Some(n);
                }
            }
            // `ЧН=` без значения — пустая строка вместо нуля; это самое
            // частое применение ключа (пустая ячейка вместо «0»).
            "ЧН" => fmt.zero_text = Some(val.to_string()),
            // `ЧВН` без значения — включение: ключ сам по себе уже флаг.
            "ЧВН" => fmt.leading_zeros = val != "0",
            "ЧС" => {
                if let Ok(n) = val.parse::<i32>() {
                    fmt.shift = n;
                }
            }
            _ => {}
        }
    }
    Ok(fmt)
}

/// Сдвиг разрядов `ЧС` — ТОЧНЫМ умножением на степень десяти, не делением
/// (деление обрезало бы результат до `DIV_SCALE` знаков и превратило бы
/// сдвиг в потерю данных).
fn shift_digits(n: &BslNumber, shift: i32) -> RtResult<BslNumber> {
    const MAX_SHIFT: i32 = 30;
    if shift == 0 {
        return Ok(n.clone());
    }
    if !(-MAX_SHIFT..=MAX_SHIFT).contains(&shift) {
        // Больше тридцати разрядов сдвига — почти наверняка опечатка в
        // форматной строке, а не намерение; молча вернуть исходное число
        // хуже, чем сказать об этом.
        return Err(RtError::TypeError {
            expected: "ЧС в пределах ±30",
            op: "Формат",
        });
    }
    let factor = if shift > 0 {
        // 10^-shift: мантисса 1 с масштабом shift.
        BslNumber::from_parts(1, shift)
    } else {
        BslNumber::from_parts(10i128.pow((-shift) as u32), 0)
    };
    Ok(n.mul(&factor)?)
}

/// `ЧЦ` как потолок ОБЩЕГО числа разрядов: сколько остаётся дробной части
/// после того, как целая заняла своё. Целая часть не обрезается — см.
/// метку на `NumberFormat::total_digits`.
fn limit_total_digits(n: &BslNumber, total: u32) -> BslNumber {
    let canonical = n.to_canonical();
    let body = canonical.strip_prefix('-').unwrap_or(&canonical);
    let int_len = body.split('.').next().unwrap_or("0").len() as u32;
    let allowed_frac = total.saturating_sub(int_len);
    let scale = n.scale().max(0) as u32;
    if allowed_frac >= scale {
        return n.clone();
    }
    n.round_to_scale(allowed_frac as i32)
}

/// Форматирует число по заданным правилам. Порядок операций важен и
/// зафиксирован здесь: сдвиг `ЧС` -> округление `ЧДЦ` -> потолок `ЧЦ` ->
/// подстановка `ЧН` на нуле -> ведущие нули `ЧВН` -> группировка.
/// Округление — половина-вверх в decimal (`BslNumber::round_to_scale`), не
/// через f64.
pub fn format_number(n: &BslNumber, fmt: &NumberFormat) -> RtResult<String> {
    let mut n = shift_digits(n, fmt.shift)?;
    if let Some(d) = fmt.frac_digits {
        n = n.round_to_scale(d as i32);
    }
    if let Some(total) = fmt.total_digits {
        n = limit_total_digits(&n, total);
    }
    // `ЧН` проверяется по ЗНАЧЕНИЮ после округления: `Формат(0.004,
    // "ЧДЦ=2; ЧН=пусто")` печатает `пусто`, а не `0,00`.
    if n.is_zero() {
        if let Some(text) = &fmt.zero_text {
            return Ok(text.clone());
        }
    }

    let canonical = n.to_canonical();
    let (sign, body): (&str, &str) = match canonical.strip_prefix('-') {
        Some(rest) => ("-", rest),
        None => ("", canonical.as_str()),
    };
    let (int_part, frac_part) = match body.split_once('.') {
        Some((i, f)) => (i, Some(f)),
        None => (body, None),
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

    // Ведущие нули добиваются ДО группировки: иначе разделитель встал бы
    // по границам исходного, а не дополненного числа.
    let mut int_part = int_part.to_string();
    if fmt.leading_zeros {
        if let Some(total) = fmt.total_digits {
            let used_by_frac = frac_part.as_ref().map_or(0, |f| f.len() as u32);
            let width = total.saturating_sub(used_by_frac) as usize;
            while int_part.len() < width {
                int_part.insert(0, '0');
            }
        }
    }
    let int_part = if fmt.group {
        group_digits(&int_part, fmt.group_sep)
    } else {
        int_part
    };

    let mut out = String::new();
    out.push_str(sign);
    out.push_str(&int_part);
    if let Some(f) = frac_part {
        out.push(fmt.decimal_sep);
        out.push_str(&f);
    }
    Ok(out)
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
/// Форматная строка имеет эффект на `Число` (`ЧГ`/`ЧРД`/`ЧРГ`/`ЧДЦ`/`ЧЦ`/
/// `ЧН`/`ЧВН`/`ЧС`), на `Дата` (`ДФ`/`ДЛФ`) и на `Булево` (`БИ`/`БЛ`); `Л`
/// действует на все три. У остальных типов применимых ключей нет, их
/// текстовое представление уже верно само по себе (`Display` в `bsl-rt`:
/// `Массив`, пустая строка для `Неопределено`, ...).
///
/// Ошибка — единственная: незнакомая локаль в `Л`.
pub fn format_value(v: &BslValue, spec: Option<&str>) -> RtResult<String> {
    match v {
        BslValue::Number(n) => {
            let fmt = match spec {
                Some(s) => parse_number_format(s)?,
                None => NumberFormat::default(),
            };
            format_number(n, &fmt)
        }
        BslValue::Date(d) => {
            let fmt = match spec {
                Some(s) => parse_date_format(s)?,
                None => DateFormat::default(),
            };
            Ok(match (&fmt.pattern, &fmt.long) {
                // `ДФ` конкретнее `ДЛФ`, поэтому при обоих ключах
                // выигрывает шаблон.
                (Some(p), _) => bsl_rt::format_date_pattern(*d, p, fmt.locale),
                (None, Some(code)) => bsl_rt::format_date_long(*d, code, fmt.locale),
                // Без ключей даты — представление по умолчанию, то же, что
                // даёт `Строка()` (оно НЕ ИЗМЕРЕНО(FMT.DATE.DEFAULT), см.
                // `bsl_rt::date`).
                (None, None) => {
                    bsl_rt::format_date_pattern(*d, bsl_rt::DEFAULT_DATE_PATTERN, fmt.locale)
                }
            })
        }
        BslValue::Boolean(b) => {
            let fmt = match spec {
                Some(s) => parse_boolean_format(s)?,
                None => BooleanFormat {
                    locale: Locale::default(),
                    true_text: None,
                    false_text: None,
                },
            };
            Ok(fmt.text(*b))
        }
        other => Ok(other.to_string()),
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

    /// Форматирование числа по разобранной строке — то, что делает
    /// `Формат`, но короче на два `unwrap`.
    fn f(value: &str, spec: &str) -> String {
        let fmt = parse_number_format(spec).unwrap();
        format_number(&n(value), &fmt).unwrap()
    }

    fn fv(v: &BslValue, spec: Option<&str>) -> String {
        format_value(v, spec).unwrap()
    }

    #[test]
    fn default_format_groups_with_nbsp_and_comma() {
        // Строка(1000.5) -> "1 000,5" (группировка даже при дробной части).
        assert_eq!(
            format_number(&n("1000.5"), &NumberFormat::default()).unwrap(),
            format!("1{NBSP}000,5")
        );
        assert_eq!(
            format_number(&n("1000000"), &NumberFormat::default()).unwrap(),
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
        for s in ["1000000", "-42.5", "0.333333333333333333333333333"] {
            assert_eq!(f(s, "ЧГ=0; ЧРД=."), n(s).to_canonical());
        }
    }

    #[test]
    fn round_trip_through_default_format_and_parse() {
        // Число(Строка(1000000)) -> 1000000: NBSP понимается обратно.
        let s = format_number(&n("1000000"), &NumberFormat::default()).unwrap();
        let back = parse_number(&s, &NumberFormat::default()).unwrap();
        assert_eq!(back, n("1000000"));
    }

    #[test]
    fn frac_digits_rounds_in_decimal_half_up() {
        assert_eq!(f("2.675", "ЧДЦ=2"), "2,68");
    }

    #[test]
    fn frac_digits_pads_with_zeros_when_value_has_fewer() {
        assert_eq!(f("5", "ЧГ=0; ЧДЦ=2"), "5,00");
    }

    #[test]
    fn negative_numbers_group_correctly() {
        assert_eq!(
            format_number(&n("-1234567"), &NumberFormat::default()).unwrap(),
            format!("-1{NBSP}234{NBSP}567")
        );
    }

    #[test]
    fn total_digits_caps_the_fraction_and_leaves_the_integer_alone() {
        // НЕ ИЗМЕРЕНО(FMT.NUM.TOTAL_DIGITS): фиксируем ВЫБРАННОЕ — `ЧЦ`
        // считает все разряды вместе, лишние уходят из дробной части.
        assert_eq!(f("123.456", "ЧГ=0; ЧЦ=5"), "123,46");
        assert_eq!(f("123.456", "ЧГ=0; ЧЦ=3"), "123");
        // Целая часть длиннее `ЧЦ` — печатается как есть, а не обрезается.
        assert_eq!(f("123456", "ЧГ=0; ЧЦ=3"), "123456");
        // `ЧДЦ` применяется раньше `ЧЦ`, поэтому более жёсткий из двух и
        // побеждает.
        assert_eq!(f("123.456", "ЧГ=0; ЧДЦ=1; ЧЦ=5"), "123,5");
    }

    #[test]
    fn zero_text_replaces_the_whole_number() {
        // НЕ ИЗМЕРЕНО(FMT.NUM.ZERO_TEXT).
        assert_eq!(f("0", "ЧН=пусто"), "пусто");
        assert_eq!(f("0", "ЧН="), "");
        assert_eq!(f("1", "ЧН=пусто"), "1");
        // Ноль ПОСЛЕ округления — тоже ноль.
        assert_eq!(f("0.004", "ЧДЦ=2; ЧН=пусто"), "пусто");
        // Без `ЧН` ноль печатается как ноль.
        assert_eq!(f("0", "ЧГ=0"), "0");
    }

    #[test]
    fn leading_zeros_pad_up_to_the_total_digits() {
        // НЕ ИЗМЕРЕНО(FMT.NUM.LEADING_ZEROS): ширина берётся из `ЧЦ`.
        assert_eq!(f("42", "ЧГ=0; ЧЦ=5; ЧВН=1"), "00042");
        assert_eq!(f("1.5", "ЧГ=0; ЧЦ=5; ЧДЦ=2; ЧВН=1"), "001,50");
        // Без `ЧЦ` ключ не делает ничего — дополнять не до чего.
        assert_eq!(f("42", "ЧГ=0; ЧВН=1"), "42");
        // `ЧВН=0` — выключение.
        assert_eq!(f("42", "ЧГ=0; ЧЦ=5; ЧВН=0"), "42");
    }

    #[test]
    fn shift_moves_the_decimal_point_exactly() {
        // НЕ ИЗМЕРЕНО(FMT.NUM.SHIFT): `ЧС=n` делит на 10^n.
        assert_eq!(f("1234", "ЧГ=0; ЧРД=.; ЧС=3"), "1.234");
        assert_eq!(f("1234", "ЧГ=0; ЧС=-2"), "123400");
        // Сдвиг ТОЧНЫЙ: 27 знаков деления тут ничего не обрезают.
        assert_eq!(
            f("1", "ЧГ=0; ЧРД=.; ЧС=30"),
            format!("0.{}1", "0".repeat(29))
        );
    }

    #[test]
    fn shift_beyond_thirty_digits_is_an_error_not_a_silent_noop() {
        let fmt = parse_number_format("ЧС=99").unwrap();
        assert!(format_number(&n("1"), &fmt).is_err());
    }

    #[test]
    fn locale_sets_the_separators_and_explicit_keys_win() {
        assert_eq!(f("1234.5", "Л=en"), "1,234.5");
        assert_eq!(f("1234.5", "Л=ru"), format!("1{NBSP}234,5"));
        // Явный ключ сильнее локали.
        assert_eq!(f("1234.5", "Л=en; ЧРД=,"), "1,234,5");
        assert_eq!(f("1234.5", "Л=en; ЧГ=0"), "1234.5");
    }

    #[test]
    fn unsupported_locale_is_an_error() {
        // НЕ ИЗМЕРЕНО(FMT.LOCALE.COVERAGE): поддержаны только ru и en, всё
        // остальное — внятная ошибка, а не молчаливый откат к русской.
        assert!(matches!(
            parse_number_format("Л=de_DE"),
            Err(RtError::UnsupportedLocale(_))
        ));
        assert!(matches!(
            format_value(&BslValue::Boolean(true), Some("Л=fr")),
            Err(RtError::UnsupportedLocale(_))
        ));
    }

    #[test]
    fn boolean_keys_override_the_locale_text() {
        let t = BslValue::Boolean(true);
        let fls = BslValue::Boolean(false);
        assert_eq!(fv(&t, None), "Да");
        assert_eq!(fv(&fls, None), "Нет");
        assert_eq!(fv(&t, Some("БИ=ага")), "ага");
        // Задан только `БИ` — ложь остаётся локальной.
        assert_eq!(fv(&fls, Some("БИ=ага")), "Нет");
        assert_eq!(fv(&fls, Some("БЛ=неа")), "неа");
        assert_eq!(fv(&t, Some("Л=en")), "Yes");
        assert_eq!(fv(&fls, Some("Л=en")), "No");
        // Явный текст сильнее локали.
        assert_eq!(fv(&t, Some("Л=en; БИ=Истина")), "Истина");
    }

    #[test]
    fn locale_switches_the_month_and_weekday_names() {
        let d = bsl_rt::BslDate::from_civil(2024, 1, 15, 10, 30, 0).unwrap();
        let v = BslValue::Date(d);
        assert_eq!(fv(&v, Some("ДФ='д ММММ гггг'")), "15 января 2024");
        assert_eq!(fv(&v, Some("Л=en; ДФ='д ММММ гггг'")), "15 January 2024");
        assert_eq!(fv(&v, Some("ДФ='дддд'")), "понедельник");
        assert_eq!(fv(&v, Some("Л=en; ДФ='дддд'")), "Monday");
        // Представление по умолчанию цифровое — от локали не зависит.
        assert_eq!(fv(&v, Some("Л=en")), fv(&v, None));
    }

    #[test]
    fn date_keys_survive_a_semicolon_inside_the_quoted_pattern() {
        // Ради чего разбиение на пары кавычко-осведомлённое: `;` внутри
        // шаблона — часть шаблона, а не разделитель ключей.
        let fmt = parse_date_format("ДФ='дд;ММ'").unwrap();
        assert_eq!(fmt.pattern.as_deref(), Some("дд;ММ"));
        // Кавычки снимаются ровно одной парой, содержимое не трогается.
        let fmt = parse_date_format("ДФ='дд.ММ.гггг'; ЧГ=0").unwrap();
        assert_eq!(fmt.pattern.as_deref(), Some("дд.ММ.гггг"));
        assert_eq!(fmt.long, None);
        // Числовые ключи из той же строки по-прежнему разбираются.
        assert!(!parse_number_format("ДФ='дд.ММ.гггг'; ЧГ=0").unwrap().group);
    }

    #[test]
    fn date_format_prefers_the_explicit_pattern_over_the_long_code() {
        let d = bsl_rt::BslDate::from_civil(2024, 1, 15, 10, 30, 0).unwrap();
        let v = BslValue::Date(d);
        assert_eq!(fv(&v, Some("ДФ='гггг'")), "2024");
        assert_eq!(fv(&v, Some("ДЛФ=В")), "10:30:00");
        // Оба ключа сразу — шаблон конкретнее.
        assert_eq!(fv(&v, Some("ДЛФ=В; ДФ='гггг'")), "2024");
        // Без ключей даты — представление по умолчанию, то же, что Строка().
        assert_eq!(fv(&v, Some("ЧГ=0")), d.to_string());
        assert_eq!(fv(&v, None), d.to_string());
    }

    #[test]
    fn non_number_values_format_via_display() {
        assert_eq!(fv(&BslValue::Boolean(true), None), "Да");
        assert_eq!(fv(&BslValue::Undefined, None), "");
    }
}
