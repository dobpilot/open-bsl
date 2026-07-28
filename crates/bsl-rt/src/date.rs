//! Дата BSL: момент времени с разрешением 1 секунда.
//!
//! # Эпоха
//!
//! Отсчёт — СЕКУНДЫ ОТ `0001-01-01 00:00:00`, не от Unix-эпохи. Это не
//! вкусовщина, а требование семантики: в 1С диапазон дат — с `0001-01-01`
//! по `9999-12-31`, а ПУСТАЯ дата это литерал `'00010101'`, то есть НАЧАЛО
//! диапазона. От Unix-эпохи пустая дата оказалась бы отрицательным числом
//! (-62135596800), и всё, что естественно пишется как «ноль значит пусто»
//! — `ЗначениеЗаполнено`, сравнение с пустой датой, инициализация регистра
//! кадра — потребовало бы отдельной константы и ломалось бы при каждом
//! забытом её применении. При эпохе `0001-01-01` пустая дата это ровно `0`.
//!
//! # Часовых поясов нет
//!
//! Дата в 1С — НАИВНЫЙ локальный момент: ни зоны, ни смещения, ни перехода
//! на летнее время в значении не хранится. Поэтому здесь нет и внешней
//! зависимости на `chrono`: `NaiveDateTime` формально подошёл бы, но тянет
//! свою модель времени, а диапазон с года 1 и специальная пустая дата всё
//! равно потребовали бы обёртки поверх него. Календарь — алгоритм
//! гражданских дней Говарда Хиннанта (`days_from_civil`/`civil_from_days`),
//! он точен на всём пролептическом григорианском календаре и умещается в
//! десяток строк.

use std::fmt;

use crate::locale::Locale;

/// Секунд в сутках.
pub const SECONDS_PER_DAY: i64 = 86_400;

/// Смещение между эпохой Хиннанта (`1970-01-01`, относительно которой
/// работают `days_from_civil`/`civil_from_days`) и нашей (`0001-01-01`).
/// Число дней между ними, положительное.
const DAYS_FROM_0001_TO_1970: i64 = 719_162;

/// Первая секунда допустимого диапазона — она же ПУСТАЯ дата (`'00010101'`).
pub const EMPTY: i64 = 0;

/// Сколько наших секунд соответствует `1970-01-01 00:00:00`. Нужно ровно в
/// одном месте — переводе системного времени в дату (`ТекущаяДата`).
pub const UNIX_EPOCH_SECONDS: i64 = DAYS_FROM_0001_TO_1970 * SECONDS_PER_DAY;

/// Номер дня `9999-12-31`, считая `0001-01-01` нулевым: годы `1..=9999`
/// это `9999 * 365 + 2424` високосных дней, минус один за то, что счёт
/// с нуля. Константой, а не вызовом `days_from_civil`, чтобы `MAX_SECONDS`
/// оставался `const`; тест `range_ends_are_exactly_...` сверяет её с
/// алгоритмом.
const DAYS_9999_12_31: i64 = 3_652_058;

/// Последняя допустимая секунда: `9999-12-31 23:59:59`.
pub const MAX_SECONDS: i64 = DAYS_9999_12_31 * SECONDS_PER_DAY + SECONDS_PER_DAY - 1;

/// Дата как значение: секунды от `0001-01-01 00:00:00`. `Copy` и один
/// `i64` — `BslValue::Date` не увеличивает размер значения (там уже есть
/// варианты пошире).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BslDate(i64);

/// Какую компоненту даты вернуть — селектор вместо шести почти одинаковых
/// функций (`Год`, `Месяц`, ... `ДеньНедели`): все они читают одно и то же
/// разложение и отличаются только выбранным полем.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatePart {
    Year,
    Month,
    Day,
    Hour,
    Minute,
    Second,
    Weekday,
}

impl DatePart {
    /// Имя для сообщения об ошибке типа — то, что пользователь написал в
    /// коде, а не имя варианта.
    pub fn op(self) -> &'static str {
        match self {
            DatePart::Year => "Год",
            DatePart::Month => "Месяц",
            DatePart::Day => "День",
            DatePart::Hour => "Час",
            DatePart::Minute => "Минута",
            DatePart::Second => "Секунда",
            DatePart::Weekday => "ДеньНедели",
        }
    }
}

/// Граница периода — тот же приём, что и `DatePart`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DateBoundary {
    StartOfDay,
    EndOfDay,
    StartOfMonth,
    EndOfMonth,
    StartOfYear,
    EndOfYear,
    StartOfWeek,
}

impl DateBoundary {
    pub fn op(self) -> &'static str {
        match self {
            DateBoundary::StartOfDay => "НачалоДня",
            DateBoundary::EndOfDay => "КонецДня",
            DateBoundary::StartOfMonth => "НачалоМесяца",
            DateBoundary::EndOfMonth => "КонецМесяца",
            DateBoundary::StartOfYear => "НачалоГода",
            DateBoundary::EndOfYear => "КонецГода",
            DateBoundary::StartOfWeek => "НачалоНедели",
        }
    }
}

/// Разложенная дата: год `1..=9999`, месяц `1..=12`, день `1..=31`,
/// время суток.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CivilDateTime {
    pub year: i64,
    pub month: u32,
    pub day: u32,
    pub hour: u32,
    pub minute: u32,
    pub second: u32,
}

impl BslDate {
    /// Пустая дата — `'00010101'`, она же ноль отсчёта (см. модульный
    /// комментарий про выбор эпохи).
    pub const fn empty() -> Self {
        BslDate(EMPTY)
    }

    pub const fn seconds(self) -> i64 {
        self.0
    }

    pub fn is_empty(self) -> bool {
        self.0 == EMPTY
    }

    /// Секунды от эпохи — с проверкой диапазона. `None` — вне
    /// `0001-01-01 .. 9999-12-31 23:59:59`.
    pub fn from_seconds(secs: i64) -> Option<Self> {
        (EMPTY..=MAX_SECONDS).contains(&secs).then_some(BslDate(secs))
    }

    /// `Дата(Год, Месяц, День[, Час, Минута, Секунда])`. `None` — любая
    /// компонента вне диапазона, включая несуществующий день месяца
    /// (`31 февраля` — ошибка, а не «переползание» на 3 марта).
    pub fn from_civil(
        year: i64,
        month: u32,
        day: u32,
        hour: u32,
        minute: u32,
        second: u32,
    ) -> Option<Self> {
        if !(1..=9999).contains(&year) || !(1..=12).contains(&month) {
            return None;
        }
        if day < 1 || day > days_in_month(year, month) {
            return None;
        }
        if hour > 23 || minute > 59 || second > 59 {
            return None;
        }
        let days = days_from_civil(year, month, day) + DAYS_FROM_0001_TO_1970;
        let secs = days * SECONDS_PER_DAY
            + hour as i64 * 3600
            + minute as i64 * 60
            + second as i64;
        BslDate::from_seconds(secs)
    }

    pub fn to_civil(self) -> CivilDateTime {
        // `div_euclid`/`rem_euclid`, а не `/` и `%`: отрицательных секунд
        // тут быть не может по конструкции, но правило "остаток всегда
        // неотрицательный" делает разложение верным без оговорок.
        let days = self.0.div_euclid(SECONDS_PER_DAY) - DAYS_FROM_0001_TO_1970;
        let rest = self.0.rem_euclid(SECONDS_PER_DAY);
        let (year, month, day) = civil_from_days(days);
        CivilDateTime {
            year,
            month,
            day,
            hour: (rest / 3600) as u32,
            minute: ((rest % 3600) / 60) as u32,
            second: (rest % 60) as u32,
        }
    }

    /// `ДеньНедели(Дата)`.
    ///
    /// НЕ ИЗМЕРЕНО(DATE.WEEKDAY_NUMBERING): нумерация. Взято
    /// `понедельник = 1 .. воскресенье = 7` — так это описано в документации
    /// 1С и так же считает `НачалоНедели` ниже; альтернатива
    /// (`воскресенье = 1`, как в некоторых СУБД) сдвинула бы обе функции
    /// разом.
    pub fn weekday(self) -> u32 {
        let days = self.0.div_euclid(SECONDS_PER_DAY);
        // `0001-01-01` — понедельник по пролептическому григорианскому
        // календарю, поэтому день 0 это уже 1.
        (days.rem_euclid(7) + 1) as u32
    }

    pub fn start_of_day(self) -> Self {
        BslDate(self.0 - self.0.rem_euclid(SECONDS_PER_DAY))
    }

    pub fn end_of_day(self) -> Self {
        BslDate(self.start_of_day().0 + SECONDS_PER_DAY - 1)
    }

    pub fn start_of_month(self) -> Self {
        let c = self.to_civil();
        BslDate::from_civil(c.year, c.month, 1, 0, 0, 0).unwrap_or(BslDate::empty())
    }

    pub fn end_of_month(self) -> Self {
        let c = self.to_civil();
        let last = days_in_month(c.year, c.month);
        BslDate::from_civil(c.year, c.month, last, 23, 59, 59).unwrap_or(BslDate::empty())
    }

    pub fn start_of_year(self) -> Self {
        let c = self.to_civil();
        BslDate::from_civil(c.year, 1, 1, 0, 0, 0).unwrap_or(BslDate::empty())
    }

    pub fn end_of_year(self) -> Self {
        let c = self.to_civil();
        BslDate::from_civil(c.year, 12, 31, 23, 59, 59).unwrap_or(BslDate::empty())
    }

    /// `НачалоНедели` — понедельник этой недели, 00:00:00 (см. `weekday`
    /// про то, что первым днём считается понедельник).
    pub fn start_of_week(self) -> Self {
        let back = (self.weekday() - 1) as i64;
        BslDate(self.start_of_day().0 - back * SECONDS_PER_DAY)
    }

    /// `ДобавитьМесяц(Дата, Количество)` — сдвиг на целые месяцы; время
    /// суток сохраняется.
    ///
    /// НЕ ИЗМЕРЕНО(DATE.ADD_MONTH_CLAMP): что происходит с днём, которого в
    /// целевом месяце нет. Взято ЗАЖАТИЕ в последний день месяца
    /// (31 января плюс месяц = 29 февраля в високосный год, 28 в обычный) —
    /// это и общепринятое поведение таких функций, и единственный вариант, при
    /// котором `ДобавитьМесяц` никогда не падает. Альтернатива (ошибка)
    /// заметно изменила бы код, который ходит по месяцам циклом.
    ///
    /// `None` — результат вышел за границы диапазона дат.
    pub fn add_months(self, months: i64) -> Option<Self> {
        let c = self.to_civil();
        let total = (c.year * 12 + (c.month as i64 - 1)).checked_add(months)?;
        let year = total.div_euclid(12);
        let month = (total.rem_euclid(12) + 1) as u32;
        if !(1..=9999).contains(&year) {
            return None;
        }
        let day = c.day.min(days_in_month(year, month));
        BslDate::from_civil(year, month, day, c.hour, c.minute, c.second)
    }

    /// `Дата + Число` / `Дата - Число` — сдвиг на секунды. `None` — вышли
    /// за границы диапазона.
    pub fn shift_seconds(self, secs: i64) -> Option<Self> {
        BslDate::from_seconds(self.0.checked_add(secs)?)
    }

    /// `Дата - Дата` — разница В СЕКУНДАХ.
    pub fn diff_seconds(self, other: Self) -> i64 {
        self.0 - other.0
    }

    /// Разбор литерала `'ГГГГММДД'` / `'ГГГГММДДЧЧММСС'` (лексер уже снял
    /// кавычки и проверил, что внутри только цифры и их 8 или 14 — см.
    /// `bsl-syntax::lexer`). Та же функция обслуживает и однострочную форму
    /// `Дата("20240115103000")`, где проверять длину приходится самим.
    pub fn parse_digits(digits: &str) -> Option<Self> {
        if digits.len() != 8 && digits.len() != 14 {
            return None;
        }
        if !digits.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        let part = |from: usize, len: usize| digits[from..from + len].parse::<u32>().ok();
        let year = part(0, 4)? as i64;
        let month = part(4, 2)?;
        let day = part(6, 2)?;
        let (hour, minute, second) = if digits.len() == 14 {
            (part(8, 2)?, part(10, 2)?, part(12, 2)?)
        } else {
            (0, 0, 0)
        };
        BslDate::from_civil(year, month, day, hour, minute, second)
    }
}

/// Формат по умолчанию для `Строка(Дата)`.
///
/// ИЗМЕРЕНО на платформе 8.3.27:
///
/// ```text
/// Строка(Дата(2024,1,15,10,30,0)) -> 15.01.2024 10:30:00
/// Строка(Дата(2024,1,15))         -> 15.01.2024 0:00:00
/// Строка('00010101')              -> 01.01.0001 0:00:00
/// ```
///
/// Время печатается ВСЕГДА, даже нулевое, — но час БЕЗ ведущего нуля
/// (`Ч`, не `ЧЧ`), тогда как минуты и секунды с ним. До замера здесь
/// стояло `ЧЧ`, и полуночные даты расходились с платформой на один
/// символ.
pub const DEFAULT_PATTERN: &str = "дд.ММ.гггг Ч:мм:сс";

impl fmt::Display for BslDate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", format_pattern(*self, DEFAULT_PATTERN, Locale::default()))
    }
}

/// Високосность — пролептический григорианский календарь на всём диапазоне
/// (в 1С дата с года 1 существует, юлианского участка нет).
pub fn is_leap_year(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

pub fn days_in_month(year: i64, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

/// Дней от `1970-01-01` до `y-m-d` (алгоритм Хиннанта). Работает для любого
/// года, в том числе до 1970 (результат отрицательный) — на нашем диапазоне
/// это годы 1..1969.
fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400; // [0, 399]
    let m = m as i64;
    let d = d as i64;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1; // [0, 365]
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy; // [0, 146096]
    era * 146_097 + doe - 719_468
}

/// Обратное к `days_from_civil`.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = doy - (153 * mp + 2) / 5 + 1; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 }; // [1, 12]
    (if m <= 2 { y + 1 } else { y }, m as u32, d as u32)
}

const MONTHS_FULL_RU: [&str; 12] = [
    "января",
    "февраля",
    "марта",
    "апреля",
    "мая",
    "июня",
    "июля",
    "августа",
    "сентября",
    "октября",
    "ноября",
    "декабря",
];

const MONTHS_SHORT_RU: [&str; 12] = [
    "янв", "фев", "мар", "апр", "мая", "июн", "июл", "авг", "сен", "окт", "ноя", "дек",
];

const WEEKDAYS_FULL_RU: [&str; 7] = [
    "понедельник",
    "вторник",
    "среда",
    "четверг",
    "пятница",
    "суббота",
    "воскресенье",
];

const WEEKDAYS_SHORT_RU: [&str; 7] = ["пн", "вт", "ср", "чт", "пт", "сб", "вс"];

/// Английские имена — часть ключа `Л` (см. `Locale`). Русские формы стоят
/// в РОДИТЕЛЬНОМ падеже («15 января»), английские — в именительном:
/// падежей у них нет, и брать для `MMMM` что-то кроме «January» негде.
const MONTHS_FULL_EN: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

const MONTHS_SHORT_EN: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

const WEEKDAYS_FULL_EN: [&str; 7] = [
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
    "Sunday",
];

const WEEKDAYS_SHORT_EN: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

fn months_full(locale: Locale) -> &'static [&'static str; 12] {
    match locale {
        Locale::Ru => &MONTHS_FULL_RU,
        Locale::En => &MONTHS_FULL_EN,
    }
}

fn months_short(locale: Locale) -> &'static [&'static str; 12] {
    match locale {
        Locale::Ru => &MONTHS_SHORT_RU,
        Locale::En => &MONTHS_SHORT_EN,
    }
}

fn weekdays_full(locale: Locale) -> &'static [&'static str; 7] {
    match locale {
        Locale::Ru => &WEEKDAYS_FULL_RU,
        Locale::En => &WEEKDAYS_FULL_EN,
    }
}

fn weekdays_short(locale: Locale) -> &'static [&'static str; 7] {
    match locale {
        Locale::Ru => &WEEKDAYS_SHORT_RU,
        Locale::En => &WEEKDAYS_SHORT_EN,
    }
}

/// Форматирование даты по шаблону `ДФ` (`дд.ММ.гггг`, `d MMMM yyyy`, ...).
///
/// Буквы шаблона — и русские, и латинские, как принимает и сама 1С.
/// ВАЖНО про регистр: `М`/`M` — МЕСЯЦ, `м`/`m` — МИНУТА; `ч`/`h` — час в
/// 12-часовом виде, `Ч`/`H` — в 24-часовом. Это единственное место во всём
/// проекте, где регистр буквы значим, поэтому оно тут написано словами.
///
/// Всё, что не буква шаблона, копируется как есть; текст в одинарных
/// кавычках копируется буквально даже если состоит из букв шаблона.
///
/// НЕ ИЗМЕРЕНО(DATE.PATTERN_LETTERS): точный набор букв и их значения
/// сверх перечисленных (`К` — квартал, `в` — до/после полудня и т.п.).
/// Реализовано то, что нужно перечисленным в брифе шаблонам.
pub fn format_pattern(date: BslDate, pattern: &str, locale: Locale) -> String {
    let c = date.to_civil();
    let chars: Vec<char> = pattern.chars().collect();
    let mut out = String::with_capacity(pattern.len() + 8);
    let mut i = 0;
    while i < chars.len() {
        let ch = chars[i];
        if ch == '\'' {
            // Литеральный кусок в одинарных кавычках.
            i += 1;
            while i < chars.len() && chars[i] != '\'' {
                out.push(chars[i]);
                i += 1;
            }
            i += 1;
            continue;
        }
        let mut run = 1;
        while i + run < chars.len() && chars[i + run] == ch {
            run += 1;
        }
        let consumed = match ch {
            'д' | 'd' => {
                match run {
                    1 => out.push_str(&c.day.to_string()),
                    2 => out.push_str(&format!("{:02}", c.day)),
                    3 => out.push_str(weekdays_short(locale)[(date.weekday() - 1) as usize]),
                    _ => out.push_str(weekdays_full(locale)[(date.weekday() - 1) as usize]),
                }
                run.min(4)
            }
            'М' | 'M' => {
                match run {
                    1 => out.push_str(&c.month.to_string()),
                    2 => out.push_str(&format!("{:02}", c.month)),
                    3 => out.push_str(months_short(locale)[(c.month - 1) as usize]),
                    _ => out.push_str(months_full(locale)[(c.month - 1) as usize]),
                }
                run.min(4)
            }
            'г' | 'y' => {
                match run {
                    1 => out.push_str(&c.year.to_string()),
                    2 => out.push_str(&format!("{:02}", c.year % 100)),
                    _ => out.push_str(&format!("{:04}", c.year)),
                }
                run.min(4)
            }
            'Ч' | 'H' => {
                push_number(&mut out, c.hour, run);
                run.min(2)
            }
            'ч' | 'h' => {
                let h12 = match c.hour % 12 {
                    0 => 12,
                    other => other,
                };
                push_number(&mut out, h12, run);
                run.min(2)
            }
            'м' | 'm' => {
                push_number(&mut out, c.minute, run);
                run.min(2)
            }
            'с' | 's' => {
                push_number(&mut out, c.second, run);
                run.min(2)
            }
            _ => {
                out.push(ch);
                1
            }
        };
        i += consumed;
    }
    out
}

fn push_number(out: &mut String, value: u32, width: usize) {
    if width >= 2 {
        out.push_str(&format!("{value:02}"));
    } else {
        out.push_str(&value.to_string());
    }
}

/// Длинные локальные форматы `ДЛФ`.
///
/// НЕ ИЗМЕРЕНО(DATE.LONG_FORMAT_CODES) целиком: и набор кодов, и то, как
/// каждый из них выглядит в
/// русской локали. Реализованы четыре, названные в брифе, плюс `ДВ` как
/// очевидная комбинация. Неизвестный код отдаёт формат по умолчанию, а не
/// падает — форматная строка в 1С вообще прощает неизвестные ключи.
pub fn format_long(date: BslDate, code: &str, locale: Locale) -> String {
    let c = date.to_civil();
    match code.trim().to_uppercase().as_str() {
        "Д" | "D" => format_pattern(date, "дд.ММ.гггг", locale),
        "ДД" | "DD" => format!(
            "{} {} {} г.",
            c.day,
            months_full(locale)[(c.month - 1) as usize],
            c.year
        ),
        "ДДД" | "DDD" => format!(
            "{}, {} {} {} г.",
            weekdays_full(locale)[(date.weekday() - 1) as usize],
            c.day,
            months_full(locale)[(c.month - 1) as usize],
            c.year
        ),
        "В" | "T" => format_pattern(date, "ЧЧ:мм:сс", locale),
        "ДВ" | "DT" => format_pattern(date, "дд.ММ.гггг ЧЧ:мм:сс", locale),
        _ => format_pattern(date, DEFAULT_PATTERN, locale),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i64, m: u32, day: u32) -> BslDate {
        BslDate::from_civil(y, m, day, 0, 0, 0).unwrap()
    }

    #[test]
    fn empty_date_is_zero_not_a_negative_unix_offset() {
        // Смысл выбора эпохи: '00010101' — ровно ноль, а не -62135596800.
        assert_eq!(BslDate::parse_digits("00010101").unwrap(), BslDate::empty());
        assert_eq!(BslDate::empty().seconds(), 0);
        assert!(BslDate::empty().is_empty());
        assert!(!d(2024, 1, 15).is_empty());
    }

    #[test]
    fn civil_round_trips_across_the_whole_supported_range() {
        for (y, m, day) in [
            (1, 1, 1),
            (1, 12, 31),
            (1582, 10, 15),
            (1900, 2, 28),
            (1970, 1, 1),
            (2000, 2, 29),
            (2024, 2, 29),
            (2024, 12, 31),
            (9999, 12, 31),
        ] {
            let date = BslDate::from_civil(y, m, day, 12, 34, 56).unwrap();
            let c = date.to_civil();
            assert_eq!((c.year, c.month, c.day), (y, m, day), "{y}-{m}-{day}");
            assert_eq!((c.hour, c.minute, c.second), (12, 34, 56));
        }
    }

    #[test]
    fn range_ends_are_exactly_the_first_and_last_representable_second() {
        assert_eq!(BslDate::from_seconds(-1), None);
        assert_eq!(BslDate::from_seconds(MAX_SECONDS + 1), None);
        let last = BslDate::from_seconds(MAX_SECONDS).unwrap().to_civil();
        assert_eq!(
            (last.year, last.month, last.day, last.hour, last.minute, last.second),
            (9999, 12, 31, 23, 59, 59)
        );
        assert_eq!(BslDate::from_civil(10000, 1, 1, 0, 0, 0), None);
        assert_eq!(BslDate::from_civil(0, 1, 1, 0, 0, 0), None);
    }

    #[test]
    fn impossible_calendar_dates_are_rejected_not_rolled_over() {
        assert_eq!(BslDate::from_civil(2023, 2, 29, 0, 0, 0), None);
        assert!(BslDate::from_civil(2024, 2, 29, 0, 0, 0).is_some());
        assert_eq!(BslDate::from_civil(2024, 4, 31, 0, 0, 0), None);
        assert_eq!(BslDate::from_civil(2024, 13, 1, 0, 0, 0), None);
        assert_eq!(BslDate::from_civil(2024, 1, 1, 24, 0, 0), None);
    }

    #[test]
    fn difference_of_two_dates_is_in_seconds() {
        assert_eq!(d(2024, 1, 15).diff_seconds(d(2024, 1, 14)), SECONDS_PER_DAY);
        assert_eq!(d(2024, 3, 1).diff_seconds(d(2024, 2, 29)), SECONDS_PER_DAY);
        // Високосный год длиннее обычного ровно на сутки.
        assert_eq!(
            d(2025, 1, 1).diff_seconds(d(2024, 1, 1)) / SECONDS_PER_DAY,
            366
        );
        assert_eq!(
            d(2024, 1, 1).diff_seconds(d(2023, 1, 1)) / SECONDS_PER_DAY,
            365
        );
    }

    #[test]
    fn weekday_is_monday_first_and_agrees_with_start_of_week() {
        // 15 января 2024 — понедельник.
        assert_eq!(d(2024, 1, 15).weekday(), 1);
        assert_eq!(d(2024, 1, 21).weekday(), 7); // воскресенье
        for day in 15..=21 {
            assert_eq!(
                d(2024, 1, day).start_of_week(),
                d(2024, 1, 15),
                "день {day} той же недели"
            );
        }
    }

    #[test]
    fn period_boundaries_keep_the_last_second_inside_the_period() {
        let dt = BslDate::from_civil(2024, 2, 17, 13, 45, 30).unwrap();
        assert_eq!(dt.start_of_day(), d(2024, 2, 17));
        assert_eq!(
            dt.end_of_day(),
            BslDate::from_civil(2024, 2, 17, 23, 59, 59).unwrap()
        );
        assert_eq!(dt.start_of_month(), d(2024, 2, 1));
        assert_eq!(
            dt.end_of_month(),
            BslDate::from_civil(2024, 2, 29, 23, 59, 59).unwrap()
        );
        assert_eq!(dt.start_of_year(), d(2024, 1, 1));
        assert_eq!(
            dt.end_of_year(),
            BslDate::from_civil(2024, 12, 31, 23, 59, 59).unwrap()
        );
        // Конец периода на секунду раньше начала следующего — не на день.
        assert_eq!(dt.end_of_month().shift_seconds(1).unwrap(), d(2024, 3, 1));
    }

    #[test]
    fn add_months_clamps_the_day_into_the_target_month() {
        // 31 января + 1 месяц: зажатие в последний день февраля
        // (НЕ ИЗМЕРЕНО(DATE.ADD_MONTH_CLAMP), см. doc comment — фиксируем
        // выбранное поведение).
        assert_eq!(d(2024, 1, 31).add_months(1).unwrap(), d(2024, 2, 29));
        assert_eq!(d(2023, 1, 31).add_months(1).unwrap(), d(2023, 2, 28));
        assert_eq!(d(2024, 1, 31).add_months(2).unwrap(), d(2024, 3, 31));
        // Отрицательный сдвиг и переход через границу года.
        assert_eq!(d(2024, 1, 15).add_months(-1).unwrap(), d(2023, 12, 15));
        assert_eq!(d(2024, 12, 15).add_months(1).unwrap(), d(2025, 1, 15));
        // Время суток сохраняется.
        let dt = BslDate::from_civil(2024, 1, 15, 10, 30, 0).unwrap();
        assert_eq!(
            dt.add_months(1).unwrap(),
            BslDate::from_civil(2024, 2, 15, 10, 30, 0).unwrap()
        );
        // Выход за диапазон — None, а не паника.
        assert_eq!(d(9999, 12, 1).add_months(1), None);
    }

    #[test]
    fn literal_digits_parse_in_both_lengths_and_reject_garbage() {
        assert_eq!(BslDate::parse_digits("20240115").unwrap(), d(2024, 1, 15));
        assert_eq!(
            BslDate::parse_digits("20240115103000").unwrap(),
            BslDate::from_civil(2024, 1, 15, 10, 30, 0).unwrap()
        );
        assert_eq!(BslDate::parse_digits("2024011"), None);
        assert_eq!(BslDate::parse_digits("20240230"), None);
    }

    #[test]
    fn pattern_formatting_distinguishes_month_from_minute_by_case() {
        let dt = BslDate::from_civil(2024, 1, 15, 9, 5, 3).unwrap();
        assert_eq!(format_pattern(dt, "дд.ММ.гггг", Locale::Ru), "15.01.2024");
        // `ММ` — месяц, `мм` — минута: разный регистр, разное значение.
        assert_eq!(format_pattern(dt, "ММ мм", Locale::Ru), "01 05");
        assert_eq!(format_pattern(dt, "ЧЧ:мм:сс", Locale::Ru), "09:05:03");
        // `ч` — 12-часовой: 9 утра остаётся 9, полночь становится 12.
        assert_eq!(format_pattern(dt, "чч", Locale::Ru), "09");
        let midnight = BslDate::from_civil(2024, 1, 15, 0, 0, 0).unwrap();
        assert_eq!(format_pattern(midnight, "чч", Locale::Ru), "12");
        assert_eq!(format_pattern(midnight, "ЧЧ", Locale::Ru), "00");
        // Латинский шаблон принимается наравне с русским.
        assert_eq!(format_pattern(dt, "dd.MM.yyyy", Locale::Ru), "15.01.2024");
        assert_eq!(format_pattern(dt, "yy", Locale::Ru), "24");
        // Имена месяца и дня недели.
        assert_eq!(format_pattern(dt, "д МММ гггг", Locale::Ru), "15 янв 2024");
        assert_eq!(format_pattern(dt, "д ММММ гггг", Locale::Ru), "15 января 2024");
        assert_eq!(format_pattern(dt, "дддд", Locale::Ru), "понедельник");
        // Кавычки защищают буквы шаблона от подстановки.
        assert_eq!(format_pattern(dt, "гггг 'год'", Locale::Ru), "2024 год");
    }

    #[test]
    fn default_display_is_the_documented_default_pattern() {
        let dt = BslDate::from_civil(2024, 1, 15, 10, 30, 0).unwrap();
        assert_eq!(dt.to_string(), "15.01.2024 10:30:00");
        assert_eq!(BslDate::empty().to_string(), "01.01.0001 0:00:00");
    }
}
