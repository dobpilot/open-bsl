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
    /// `ЧЦ` — общее число разрядов (целые вместе с дробными).
    ///
    /// ИЗМЕРЕНО на 8.3.27, и модель оказалась совсем не той, что была
    /// выбрана до замера:
    ///
    /// ```text
    /// Формат(123.456, "ЧЦ=5") -> 123      дробных 0, не 123,46
    /// Формат(123.456, "ЧЦ=7") -> 123      ЧЦ дробную часть НЕ возвращает
    /// Формат(1.5,     "ЧЦ=1") -> 2        округление до целого
    /// Формат(123456,  "ЧЦ=3") -> 999      переполнение поля — все девятки
    /// Формат(1.5, "ЧЦ=5; ЧДЦ=2") -> 1,50  дробных ровно ЧДЦ
    /// ```
    ///
    /// То есть: дробных разрядов ровно `ЧДЦ` (по умолчанию НОЛЬ, если задан
    /// `ЧЦ`), остальные разряды достаются целой части, а если она туда не
    /// влезает — поле заполняется девятками. До замера здесь стоял
    /// «потолок дробной части», и он не совпадал ни в одной точке.
    pub total_digits: Option<u32>,
    /// `ЧН` — что печатать вместо нуля.
    ///
    /// ИЗМЕРЕНО на 8.3.27:
    ///
    /// ```text
    /// Формат(0, "ЧГ=0")            -> ""        ноль по умолчанию ПУСТОЙ
    /// Формат(0)                    -> ""
    /// Строка(0)                    -> "0"       а вот Строка печатает ноль
    /// Формат(0, "ЧН=X")            -> "X"
    /// Формат(0, "ЧН=")             -> "0"       пустое значение ключа
    /// Формат(0.004, "ЧДЦ=2; ЧН=X") -> "0,00"    к округлённому нулю НЕ применяется
    /// ```
    ///
    /// Отсюда две неожиданности, обе были у нас неверны: `Формат` без `ЧН`
    /// печатает ноль ПУСТОЙ СТРОКОЙ (а `Строка` — нет), и `ЧН=` с пустым
    /// значением означает «печатать как обычно», а не «печатать пусто».
    pub zero_text: Option<String>,
    /// `ЧВН` — выводить ведущие нули.
    ///
    /// `НЕ ИЗМЕРЕНО(FMT.NUM.LEADING_ZEROS)`: до какой ширины дополнять.
    /// Взято «до `ЧЦ` минус дробные разряды», то есть без `ЧЦ` ключ не
    /// делает ничего; альтернатива — своя ширина у самого `ЧВН`.
    pub leading_zeros: bool,
    /// Печатать ли ноль пустой строкой. ИЗМЕРЕНО, что это свойство самой
    /// ФУНКЦИИ, а не форматной строки: `Формат(0, "ЧГ=0")` даёт пустую
    /// строку, а `Строка(0)` — «0». Поэтому флаг ставится не ключом, а тем,
    /// кто строит формат: `Формат` — да, `Строка` — нет.
    pub blank_zero: bool,
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
            blank_zero: false,
            leading_zeros: false,
            shift: 0,
        }
    }
}

/// Форматирование БУЛЕВА: `БИ`/`БЛ` — тексты для истины и лжи, `Л` —
/// локаль, дающая их умолчания.
///
/// `НЕ ИЗМЕРЕНО(FMT.BOOLEAN.TRUE_TEXT)` и
/// `НЕ ИЗМЕРЕНО(FMT.BOOLEAN.FALSE_TEXT)`: что происходит, когда задан только
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
/// `None` в обоих полях — формат по умолчанию, он ИЗМЕРЕН и зафиксирован
/// в `bsl_rt::date::DEFAULT_PATTERN`.
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

/// Разобранные пары форматной строки кэшируются: `Формат` в горячих
/// циклах зовётся с одними и теми же литералами, и повторный разбор — с
/// `to_uppercase` кириллических ключей на каждый вызов — был заметен в
/// профиле. Кэш потоко-локальный и ограничен по числу записей: форматные
/// строки в реальном коде — литералы, их единицы; при переполнении кэш
/// просто очищается.
const SPEC_CACHE_CAP: usize = 128;

/// Пары `ключ=значение` одной форматной строки, разделяемые кэшем.
type SpecParts = std::rc::Rc<Vec<(String, String)>>;

thread_local! {
    static SPEC_CACHE: std::cell::RefCell<std::collections::HashMap<String, SpecParts>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
}

fn spec_parts(spec: &str) -> SpecParts {
    SPEC_CACHE.with(|cache| {
        if let Some(hit) = cache.borrow().get(spec) {
            return hit.clone();
        }
        let parts = std::rc::Rc::new(spec_parts_uncached(spec));
        let mut cache = cache.borrow_mut();
        if cache.len() >= SPEC_CACHE_CAP {
            cache.clear();
        }
        cache.insert(spec.to_string(), parts.clone());
        parts
    })
}

/// Разбивает форматную строку на пары `ключ=значение` по `;`, НЕ разрезая
/// внутри одинарных кавычек: `ДФ='дд.ММ.гггг'` кавычки нужны как раз
/// потому, что шаблон может содержать что угодно, включая `;`.
fn spec_parts_uncached(spec: &str) -> Vec<(String, String)> {
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
/// Ключ и откат ИЗМЕРЕНЫ: `Л=<код>` задаёт локаль, отсутствие ключа и
/// незнакомый код — русская.
fn parse_locale(parts: &[(String, String)]) -> Locale {
    for (key, val) in parts {
        if key == "Л" || key == "L" {
            return Locale::parse_or_default(val);
        }
    }
    Locale::default()
}

/// Разбирает из форматной строки то, что относится к ДАТЕ. Ключи, не
/// относящиеся к дате (`ЧГ`, `ЧРД`, ...), здесь игнорируются — ровно
/// симметрично тому, как `parse_number_format` игнорирует `ДФ`. Кроме `Л`:
/// она общая для всех типов.
pub fn parse_date_format(spec: &str) -> RtResult<DateFormat> {
    let parts = spec_parts(spec);
    let mut fmt = DateFormat {
        locale: parse_locale(&parts),
        ..DateFormat::default()
    };
    for (key, val) in parts.iter() {
        match key.as_str() {
            "ДФ" | "DF" => fmt.pattern = Some(val.clone()),
            "ДЛФ" | "DLF" => fmt.long = Some(val.clone()),
            _ => {}
        }
    }
    Ok(fmt)
}

/// Разбирает булевы ключи `БИ`/`БЛ` (плюс общую `Л`).
pub fn parse_boolean_format(spec: &str) -> RtResult<BooleanFormat> {
    let parts = spec_parts(spec);
    let mut fmt = BooleanFormat {
        locale: parse_locale(&parts),
        true_text: None,
        false_text: None,
    };
    for (key, val) in parts.iter() {
        match key.as_str() {
            "БИ" | "BT" => fmt.true_text = Some(val.clone()),
            "БЛ" | "BF" => fmt.false_text = Some(val.clone()),
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
    let mut fmt = NumberFormat::for_locale(parse_locale(&parts));
    fmt.blank_zero = true;
    for (key, val) in parts.iter() {
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

/// Сколько дробных разрядов останется в итоге: `ЧДЦ`, если задан; ноль,
/// если задан `ЧЦ` (измерено: `Формат(123.456, "ЧЦ=7")` -> `123`, дробная
/// часть при `ЧЦ` без `ЧДЦ` не печатается вовсе); иначе — сколько есть.
fn effective_frac_digits(n: &BslNumber, fmt: &NumberFormat) -> Option<u32> {
    match (fmt.frac_digits, fmt.total_digits) {
        (Some(d), _) => Some(d),
        (None, Some(_)) => Some(0),
        (None, None) => {
            let _ = n;
            None
        }
    }
}

/// Влезает ли целая часть в поле шириной `ЧЦ` минус дробные разряды.
/// Не влезает — платформа заполняет ВСЁ поле девятками (`Формат(123456,
/// "ЧЦ=3")` -> `999`), а не печатает число как есть.
fn overflow_nines(int_digits: usize, total: u32, frac: u32) -> Option<String> {
    let room = total.saturating_sub(frac) as usize;
    if int_digits <= room {
        return None;
    }
    Some("9".repeat(room.max(1)))
}

/// Форматирует число по заданным правилам. Порядок операций важен и
/// зафиксирован здесь: сдвиг `ЧС` -> округление `ЧДЦ` -> потолок `ЧЦ` ->
/// подстановка `ЧН` на нуле -> ведущие нули `ЧВН` -> группировка.
/// Округление — половина-вверх в decimal (`BslNumber::round_to_scale`), не
/// через f64.
pub fn format_number(n: &BslNumber, fmt: &NumberFormat) -> RtResult<String> {
    let shifted = shift_digits(n, fmt.shift)?;

    // Подстановка вместо нуля смотрит на ИСХОДНОЕ значение, а не на
    // округлённое: измерено, что `Формат(0.004, "ЧДЦ=2; ЧН=X")` печатает
    // `0,00`, то есть ноль после округления заменой не считается.
    if shifted.is_zero() {
        match &fmt.zero_text {
            // `ЧН=X` — печатаем X.
            Some(text) if !text.is_empty() => return Ok(text.clone()),
            // `ЧН=` без значения — печатаем ноль как обычно (измерено).
            Some(_) => {}
            // Без `ЧН`: у `Формат` ноль пустой, у `Строка` — нет.
            None if fmt.blank_zero => return Ok(String::new()),
            None => {}
        }
    }

    let frac_digits = effective_frac_digits(&shifted, fmt);
    let rounded = match frac_digits {
        Some(d) => shifted.round_to_scale(d as i32),
        None => shifted,
    };

    let canonical = rounded.to_canonical();
    let (sign, body): (&str, &str) = match canonical.strip_prefix('-') {
        Some(rest) => ("-", rest),
        None => ("", canonical.as_str()),
    };
    let (int_part, frac_part) = match body.split_once('.') {
        Some((i, f)) => (i, Some(f)),
        None => (body, None),
    };

    if let Some(total) = fmt.total_digits
        && let Some(nines) = overflow_nines(int_part.len(), total, frac_digits.unwrap_or(0))
    {
        let frac = frac_digits.unwrap_or(0) as usize;
        let mut out = String::new();
        out.push_str(sign);
        out.push_str(&nines);
        if frac > 0 {
            out.push(fmt.decimal_sep);
            out.push_str(&"9".repeat(frac));
        }
        return Ok(out);
    }

    let frac_part = match (frac_part, frac_digits) {
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
    if fmt.leading_zeros
        && let Some(total) = fmt.total_digits
    {
        let used_by_frac = frac_part.as_ref().map_or(0, |f| f.len() as u32);
        let width = total.saturating_sub(used_by_frac) as usize;
        while int_part.len() < width {
            int_part.insert(0, '0');
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
        if i > 0 && (n - i).is_multiple_of(3) {
            out.push(sep);
        }
        out.push(*b as char);
    }
    out
}

/// `Строка(значение)` / `Формат(значение, spec)`. `spec = None` — правила
/// по умолчанию (`Строка()` — это `Формат()` без явной форматной строки).
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
                // даёт `Строка()` (измерено, см. `bsl_rt::date`).
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
        // `Строка(Null)` — ПУСТАЯ строка, как и `Строка(Неопределено)`
        // (измерено, `CONCAT.RIGHT.NULL`). Через `Display` сюда попадало бы
        // отладочное «Null»: он для человека за отладчиком, а не для
        // пользовательского вывода.
        BslValue::Null => Ok(String::new()),
        other => Ok(other.to_string()),
    }
}

/// Форматирование значения для подстановки в ячейку табличного документа.
/// Отличается от [`format_value`] тем, что ноль НЕ пустеет: семантика
/// `Строка`, а не `Формат`. Спецификация формата ячейки (`ЧДЦ=2` и т. п.)
/// применяется, но `ЧН` (пустой ноль) принудительно отключён.
pub fn format_value_for_cell(v: &BslValue, spec: Option<&str>) -> RtResult<String> {
    match v {
        BslValue::Number(n) => {
            let mut fmt = match spec {
                Some(s) => parse_number_format(s)?,
                None => NumberFormat::default(),
            };
            fmt.blank_zero = false;
            format_number(n, &fmt)
        }
        other => format_value(other, spec),
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

    /// ИЗМЕРЕНО на 8.3.27, и обе неожиданности здесь: `Формат` печатает
    /// ноль ПУСТОЙ строкой, а `ЧН=` с пустым значением означает «печатать
    /// как обычно».
    #[test]
    fn zero_prints_empty_from_format_and_zero_from_string() {
        assert_eq!(f("0", "ЧГ=0"), "");
        assert_eq!(f("0", "ЧН=пусто"), "пусто");
        assert_eq!(f("0", "ЧН="), "0");
        assert_eq!(f("1", "ЧН=пусто"), "1");
        // Ноль ПОСЛЕ округления заменой не считается — смотрим на исходное.
        assert_eq!(f("0.004", "ЧДЦ=2; ЧН=пусто"), "0,00");
        // А `Строка()` печатает ноль как ноль.
        assert_eq!(fv(&BslValue::Number(n("0")), None), "0");
    }

    /// ИЗМЕРЕНО. `ЧЦ` — ОБЩЕЕ число разрядов: дробных ровно `ЧДЦ` (по
    /// умолчанию ноль), остальное целой части, переполнение — девятки на
    /// всё поле.
    #[test]
    fn total_digits_is_a_field_width_and_overflow_fills_it_with_nines() {
        assert_eq!(f("123.456", "ЧГ=0; ЧЦ=5"), "123");
        assert_eq!(f("123.456", "ЧГ=0; ЧЦ=7"), "123");
        assert_eq!(f("1.5", "ЧГ=0; ЧЦ=1"), "2");
        assert_eq!(f("123456", "ЧГ=0; ЧЦ=3"), "999");
        assert_eq!(f("1.5", "ЧГ=0; ЧЦ=5; ЧДЦ=2"), "1,50");
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

    /// Разделители ИЗМЕРЕНЫ на 8.3.27 по всем пяти локалям.
    #[test]
    fn measured_locales_use_their_measured_separators() {
        assert_eq!(f("1234.5", "Л=ru_RU"), format!("1{NBSP}234,5"));
        assert_eq!(f("1234.5", "Л=en_US"), "1,234.5");
        assert_eq!(f("1234.5", "Л=de_DE"), "1.234,5");
        assert_eq!(f("1234.5", "Л=fr_FR"), format!("1{NBSP}234,5"));
        assert_eq!(f("1234.5", "Л=ja_JP"), "1,234.5");
    }

    /// ИЗМЕРЕНО: несуществующий код — не ошибка, а откат к русской
    /// (`Формат(1234.5, "Л=zz_ZZ")` даёт `1 234,5`). До замера здесь была
    /// ошибка «чтобы не притворяться» — платформа притворяется.
    #[test]
    fn an_unknown_locale_falls_back_to_russian() {
        assert_eq!(f("1234.5", "Л=zz_ZZ"), format!("1{NBSP}234,5"));
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
