//! Календарные операции значения BSL и обращения к часам хоста.

use super::BslValue;
use crate::date::{self, DateBoundary, DatePart};
use crate::{BslDate, BslNumber, HostEnv, RtError, RtResult};

impl BslValue {
    // --- Даты -------------------------------------------------------------

    fn as_date(&self, op: &'static str) -> RtResult<BslDate> {
        match self {
            BslValue::Date(d) => Ok(*d),
            _ => Err(RtError::TypeError {
                expected: "Дата",
                op,
            }),
        }
    }

    /// Компонента `Дата(...)` — целое число, помещающееся в календарный
    /// диапазон. Нецелое (`Дата(2024.5, 1, 1)`) — ошибка, а не усечение.
    fn date_part(v: &Self, op: &'static str) -> RtResult<i64> {
        v.as_number(op)?.to_i64_exact().ok_or(RtError::TypeError {
            expected: "Число (целое)",
            op,
        })
    }

    /// `Дата(Год, Месяц, День[, Час, Минута, Секунда])` — шестиместная
    /// форма, и `Дата("ГГГГММДД[ЧЧММСС]")` — строковая.
    ///
    /// Обе формы живут в одной функции, потому что в 1С это одна и та же
    /// встроенная функция с перегрузкой по типу первого аргумента, а не две
    /// разных. Опущенные `Час`/`Минута`/`Секунда` приходят сюда как
    /// `Неопределено` (см. `BuiltinFn::arity_range`) и означают ноль.
    pub fn make_date(args: &[BslValue]) -> RtResult<Self> {
        // Строковая форма: `Дата("20240115103000")`. Остальные позиции при
        // ней обязаны быть пустыми — `Дата("20240115", 2, 3)` бессмысленно.
        if let BslValue::Str(s) = &args[0] {
            if args[1..].iter().any(|a| !matches!(a, BslValue::Undefined)) {
                return Err(RtError::TypeError {
                    expected: "Дата(«ГГГГММДДЧЧММСС») без остальных аргументов",
                    op: "Дата",
                });
            }
            return BslDate::parse_digits(&s.to_string())
                .map(BslValue::Date)
                .ok_or(RtError::DateOutOfRange { op: "Дата" });
        }

        let year = Self::date_part(&args[0], "Дата")?;
        let month = Self::date_part(&args[1], "Дата")?;
        let day = Self::date_part(&args[2], "Дата")?;
        // Час/минута/секунда необязательны — опущенные значат ноль.
        let mut time = [0i64; 3];
        for (i, slot) in time.iter_mut().enumerate() {
            *slot = match &args[3 + i] {
                BslValue::Undefined => 0,
                other => Self::date_part(other, "Дата")?,
            };
        }
        let fits = |v: i64| u32::try_from(v).ok();
        let built = (|| {
            BslDate::from_civil(
                year,
                fits(month)?,
                fits(day)?,
                fits(time[0])?,
                fits(time[1])?,
                fits(time[2])?,
            )
        })();
        built
            .map(BslValue::Date)
            .ok_or(RtError::DateOutOfRange { op: "Дата" })
    }

    /// `ТекущаяДата()`.
    ///
    /// ОТКЛОНЕНИЕ, о котором надо знать: возвращается момент по UTC, а не
    /// по локальной зоне машины. Дата в 1С — наивный локальный момент без
    /// зоны (см. модуль `date`), а в `std` нет способа узнать смещение
    /// локальной зоны; тащить ради этого `chrono`/`libc` значит тащить
    /// целую модель времени с зонами, которой в типе всё равно нет.
    /// Наблюдаемо это только как сдвиг на смещение зоны, и только у
    /// `ТекущаяДата` — все остальные функции работают с датами, которые им
    /// дали.
    pub fn current_date(env: &mut HostEnv) -> RtResult<Self> {
        let secs = env.unix_millis().div_euclid(1000);
        BslDate::from_seconds(secs + date::UNIX_EPOCH_SECONDS)
            .map(BslValue::Date)
            .ok_or(RtError::DateOutOfRange {
                op: "ТекущаяДата"
            })
    }

    /// `ТекущаяУниверсальнаяДата()` — текущий момент UTC как наивная дата
    /// BSL. Источник времени принадлежит прогону и потому подменяется тем
    /// же `Clock`, что и две соседние функции времени.
    pub fn current_universal_date(env: &mut HostEnv) -> RtResult<Self> {
        let secs = env.unix_millis().div_euclid(1000);
        BslDate::from_seconds(secs + date::UNIX_EPOCH_SECONDS)
            .map(BslValue::Date)
            .ok_or(RtError::DateOutOfRange {
                op: "ТекущаяУниверсальнаяДата",
            })
    }

    /// `ТекущаяУниверсальнаяДатаВМиллисекундах()` — целое число
    /// миллисекунд от Unix-эпохи в UTC.
    pub fn current_universal_date_in_milliseconds(env: &mut HostEnv) -> RtResult<Self> {
        let millis = env.unix_millis();
        // Отсчёт — от эпохи дат BSL (0001-01-01 UTC), не от 1970-го:
        // ИЗМЕРЕНО на 8.3.27, платформа печатает ~63.9e12.
        let millis = millis
            .checked_add(crate::date::UNIX_EPOCH_SECONDS * 1000)
            .ok_or(RtError::DateOutOfRange {
                op: "ТекущаяУниверсальнаяДатаВМиллисекундах",
            })?;
        Ok(BslValue::Number(BslNumber::from_i64(millis)))
    }

    /// `Год`/`Месяц`/`День`/`Час`/`Минута`/`Секунда`/`ДеньНедели` — все
    /// возвращают `Число`, поэтому одна функция с селектором вместо шести
    /// почти одинаковых.
    pub fn date_component(&self, part: DatePart) -> RtResult<Self> {
        let d = self.as_date(part.op())?;
        let c = d.to_civil();
        let n = match part {
            DatePart::Year => c.year,
            DatePart::Month => c.month as i64,
            DatePart::Day => c.day as i64,
            DatePart::Hour => c.hour as i64,
            DatePart::Minute => c.minute as i64,
            DatePart::Second => c.second as i64,
            DatePart::Weekday => d.weekday() as i64,
        };
        Ok(BslValue::Number(BslNumber::from_i64(n)))
    }

    /// `НачалоДня`/`КонецДня`/`НачалоМесяца`/... — тоже один селектор:
    /// все семь границ отличаются только тем, что округляют.
    pub fn date_boundary(&self, which: DateBoundary) -> RtResult<Self> {
        let d = self.as_date(which.op())?;
        Ok(BslValue::Date(match which {
            DateBoundary::StartOfDay => d.start_of_day(),
            DateBoundary::EndOfDay => d.end_of_day(),
            DateBoundary::StartOfMonth => d.start_of_month(),
            DateBoundary::EndOfMonth => d.end_of_month(),
            DateBoundary::StartOfYear => d.start_of_year(),
            DateBoundary::EndOfYear => d.end_of_year(),
            DateBoundary::StartOfWeek => d.start_of_week(),
        }))
    }

    /// `ДобавитьМесяц(Дата, Количество)` — про зажатие дня см.
    /// `BslDate::add_months` (там же пометка `НЕ ИЗМЕРЕНО(DATE.ADD_MONTH_CLAMP)`).
    pub fn add_month(&self, count: &Self) -> RtResult<Self> {
        let d = self.as_date("ДобавитьМесяц")?;
        let n = Self::date_part(count, "ДобавитьМесяц")?;
        d.add_months(n)
            .map(BslValue::Date)
            .ok_or(RtError::DateOutOfRange {
                op: "ДобавитьМесяц",
            })
    }
}
