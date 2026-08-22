//! Окружение запуска: то, что BSL-функция берёт не из своих аргументов, а
//! из мира вокруг, — аргументы командной строки, часы и источник
//! случайности.
//!
//! Всё это раньше лежало в состоянии ПРОЦЕССА или ПОТОКА: аргументы — в
//! `OnceLock`, часы — прямым `SystemTime::now()` из тела функции, байты
//! идентификатора — из `thread_local` с дескриптором `/dev/urandom`. Две
//! изолированные сессии одного `Engine` поэтому делили часть окружения, а
//! проверить поведение на заданном времени было нечем: тест мог только
//! сравнить результат сам с собой.
//!
//! Теперь окружение принадлежит конкретному прогону и едет в него явным
//! параметром. Реализации по умолчанию ([`HostEnv::process`]) сохраняют
//! прежнее поведение процесса бит в бит.
//!
//! Местное смещение времени — четвёртая возможность, и попадает она к
//! потребителю иначе, чем три остальные. Часы, аргументы и случайность
//! читает код ядра, у которого есть `HostEnv` напрямую; зону читает
//! перевод УЖЕ ЗАПИСАННЫХ моментов в `bsl-json` и `bsl-xml`, то есть код
//! компонентных крейтов, которому доступен только `CallContext`. Поэтому
//! зона и едет через `CallContext` — расширение ABI компонентов здесь
//! неизбежно, а половинчатый перенос завёл бы второй источник правды.
//!
//! Отсюда и разница в форме: `Clock` и `RandomSource` берут `&mut self`
//! (тестовые часы шагают, источник выдаёт последовательность) и живут в
//! `Box`, а [`TimeZone`] — чистый запрос «смещение на момент», `&self` и
//! `Rc`. Разделяемая ссылка тут не роскошь: контекст компонента строится,
//! пока `HostEnv` уже занят чем-то другим, и `&mut` не дался бы.

use std::io::Read;
use std::time::{SystemTime, UNIX_EPOCH};

/// Часы прогона: миллисекунды от Unix-эпохи в UTC.
///
/// Одна величина, а не «дата плюс время»: обе временные встроенные функции
/// выражаются через неё, а собственной модели дат интерфейсу заводить
/// незачем — она уже есть в [`crate::BslDate`].
///
/// `&mut self`, потому что тестовые часы обычно шагают: неподвижное время
/// удобно для оракула, но проверить «прошло 5 мс» на `&self` нечем.
pub trait Clock {
    fn unix_millis(&mut self) -> i64;
}

/// Источник случайных байтов для `Новый УникальныйИдентификатор()`.
///
/// Ровно шестнадцать байтов — столько в идентификаторе, и другого
/// потребителя случайности в рантайме нет. Расставление битов версии и
/// варианта сюда не входит: это чистая функция над байтами
/// (`uuid::v4_from_bytes`), и подменять её вместе с источником
/// значило бы позволить тестовой реализации выдать не-UUID.
pub trait RandomSource {
    fn fill(&mut self, buffer: &mut [u8; 16]);
}

/// Часовой пояс прогона: смещение от UTC в секундах на ЗАДАННЫЙ момент
/// (положительное восточнее Гринвича).
///
/// Момент, а не «текущее смещение»: зона переводит уже записанные даты, и
/// у даты 2015 года смещение своё, а не сегодняшнее. `&self`, а не
/// `&mut self`, — в отличие от [`Clock`], здесь нечему меняться от
/// обращения к обращению, и разделяемая ссылка позволяет отдать зону
/// компоненту, не забирая `HostEnv` целиком.
pub trait TimeZone {
    /// Смещение в секундах — БЕЗ обещания кратности минуте: до первого
    /// перехода база `tzdata` хранит местное солнечное время (у Москвы
    /// `+02:30:17`), и [`crate::SystemTimeZone`] его так и отдаёт.
    /// Ограничение на целые минуты есть только у [`FixedTimeZone`], где
    /// оно защищает встраивающую программу от значения, которого зона
    /// иметь не может; что делают с остатком потребители — их измеренное
    /// дело (см. `format_offset` в `bsl-json`).
    ///
    /// # Договор реализации
    ///
    /// [`TimeZone::offset_for_local`] решает обратную задачу пробами по
    /// обе стороны перехода и потому опирается на две границы, которые
    /// сам тип обеспечить не может:
    ///
    ///   * `|offset_seconds| <= `[`MAX_OFFSET_SECONDS`];
    ///   * два перехода не ближе [`MIN_TRANSITION_GAP_SECONDS`] друг к
    ///     другу.
    ///
    /// Второе условие именно такое, а не «сутки»: пробы стоят дальше
    /// самого большого смещения, и ВТОРОЙ переход, попавший между пробой
    /// и нужным переходом, подменяет кандидата. Например, зона `+24:00`
    /// до нуля, `+23:00` до тридцати часов и `+22:00` дальше на местных
    /// 23:30 имеет два согласованных решения (`+24:00` и `+23:00`, час
    /// прожит дважды), но дальняя проба видит уже `+22:00`, и правило
    /// «сперва сторона после перехода» до нужного ответа не добирается.
    ///
    /// Установленная база `tzdata` обе выдерживает с запасом: крайние
    /// значения в ней — `Asia/Manila` (`-15:56:08`) и
    /// `America/Metlakatla` (`+15:13:42`), оба это местное солнечное
    /// время до первого перехода. Реализация, которая границы нарушит,
    /// получит от `offset_for_local` смещение, не решающее уравнение, —
    /// но не панику и не расхождение с `offset_seconds`.
    fn offset_seconds(&self, unix_seconds: i64) -> i32;

    /// Смещение, действовавшее в заданный момент МЕСТНОГО времени, где
    /// `wall_seconds` — показания местных часов, посчитанные от
    /// Unix-эпохи так, будто они и есть UTC.
    ///
    /// Задача круговая: смещение зависит от момента, а момент из местного
    /// времени получается вычитанием смещения. Рядом с переходом у неё
    /// бывает два решения (час, прожитый дважды) или ни одного (час,
    /// пропущенный при переводе вперёд).
    ///
    /// ИЗМЕРЕНО на 8.3.27 в зоне Europe/Moscow, двенадцать точек в
    /// `measure-zone.platform.txt`, и ответы у двух особых часов РАЗНЫЕ:
    ///
    ///   * час, прожитый ДВАЖДЫ (`31.10.2010 02:30`), платформа относит к
    ///     ВТОРОМУ проходу — `+03:00`, смещение после перехода;
    ///   * часа, ПРОПУЩЕННОГО при переводе вперёд (`27.03.2011 02:30`), не
    ///     было вовсе, и платформа отвечает `+03:00` — смещением ДО
    ///     перехода.
    ///
    /// Обе стороны укладываются в один порядок проб: сперва смещение
    /// ПОСЛЕ перехода, затем ДО; последнее остаётся ответом и тогда,
    /// когда согласованного решения нет вовсе. Остальные десять точек
    /// однозначны: у них обе пробы дают одно и то же.
    ///
    /// Реализация по умолчанию выражает это правило через
    /// [`TimeZone::offset_seconds`] и переопределения не требует.
    fn offset_for_local(&self, wall_seconds: i64) -> i32 {
        // Кандидаты берутся ПО ОБЕ СТОРОНЫ возможного перехода, и именно
        // в пространстве МОМЕНТОВ, а не по местным часам: по договору
        // выше смещение не больше `MAX_OFFSET_SECONDS`, значит переход
        // рядом с `wall_seconds` лежит внутри этого расстояния от него, и
        // пробы на `PROBE_SECONDS` дают обе стороны. Пробовать «на месте»
        // (`offset_seconds(wall_seconds)`) нельзя: у западной зоны
        // показания часов сдвинуты в другую сторону, и такая проба берёт
        // кандидата не с той стороны — при переходе `-04:00 -> -05:00`
        // местные 08:30 получали бы `-04:00`, хотя это смещение само себя
        // не подтверждает.
        let probe = |shift: i64| self.offset_seconds(wall_seconds.saturating_add(shift));
        let after = probe(PROBE_SECONDS);
        let before = probe(-PROBE_SECONDS);
        // Порядок проб — измеренное правило: час, прожитый дважды,
        // относится ко ВТОРОМУ проходу, поэтому сторона «после» идёт
        // первой.
        let consistent = |offset: i32| {
            self.offset_seconds(wall_seconds.saturating_sub(i64::from(offset))) == offset
        };
        if consistent(after) {
            return after;
        }
        if consistent(before) {
            return before;
        }
        // Согласованного решения нет: этого местного времени не
        // существовало. Ответ — смещение до перехода.
        before
    }
}

/// Наибольшее смещение от UTC, которое [`TimeZone`] вправе вернуть, и
/// заодно наименьшее расстояние между двумя переходами (см. договор у
/// [`TimeZone::offset_seconds`]). Сутки с запасом: крайние значения
/// установленной базы `tzdata` — около шестнадцати часов.
pub const MAX_OFFSET_SECONDS: i32 = 24 * 3600;

/// Насколько далеко от местного времени берутся пробы. Строго больше
/// [`MAX_OFFSET_SECONDS`], иначе обе пробы могли бы лечь по одну сторону
/// перехода.
const PROBE_SECONDS: i64 = MAX_OFFSET_SECONDS as i64 + 3600;

/// Наименьшее расстояние между двумя переходами, при котором проба
/// заведомо видит смещение ПО ТУ СТОРОНУ нужного перехода, а не следующее
/// за ним: проба уходит дальше самого большого смещения (`PROBE_SECONDS`
/// в этом модуле), а сам переход может лежать в [`MAX_OFFSET_SECONDS`] от
/// местного времени, и второй переход обязан быть дальше их суммы. База `tzdata` держит это с огромным запасом —
/// переходы в ней разделены месяцами.
pub const MIN_TRANSITION_GAP_SECONDS: i64 = PROBE_SECONDS + MAX_OFFSET_SECONDS as i64;

/// Неподвижное смещение — зона для встраивающей программы и для тестов:
/// `FixedTimeZone::new(3 * 3600)` это UTC+3 круглый год, без переходов.
///
/// Поле закрыто, и это не церемония. Смещение печатается как `+ЧЧ:ММ`
/// (даты JSON, лексические формы XDTO), то есть у записи ровно два
/// разряда на часы и минутная точность: `FixedTimeZone(1)` напечаталось
/// бы как `+00:00`, вычитая при этом целую секунду, а `i32::MAX` дал бы
/// `+596523:14` — форму, которой в ISO 8601 не существует. Проверяющий
/// конструктор делает такие значения непредставимыми.
pub struct FixedTimeZone(i32);

impl FixedTimeZone {
    /// Гринвич: зона, в которой местное время и есть UTC.
    pub const UTC: FixedTimeZone = FixedTimeZone(0);

    /// Смещение в секундах: целое число минут, не дальше ±14:00 от UTC —
    /// предел, за который не выходит ни одна зона базы `tzdata`.
    ///
    /// `None` — значение вне этих границ.
    #[must_use]
    pub const fn new(offset_seconds: i32) -> Option<FixedTimeZone> {
        if offset_seconds % 60 == 0 && offset_seconds.abs() <= 14 * 3600 {
            Some(FixedTimeZone(offset_seconds))
        } else {
            None
        }
    }
}

impl TimeZone for FixedTimeZone {
    fn offset_seconds(&self, _unix_seconds: i64) -> i32 {
        self.0
    }
}

/// Часы процесса.
pub struct SystemClock;

impl Clock for SystemClock {
    fn unix_millis(&mut self) -> i64 {
        // `unwrap_or(0)` — прежнее поведение: часы до 1970 года на этой
        // платформе означают сломанные часы, а не момент времени.
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .ok()
            .and_then(|d| i64::try_from(d.as_millis()).ok())
            .unwrap_or(0)
    }
}

/// Случайность процесса: `/dev/urandom`, а без него — ключи `RandomState`.
///
/// Дескриптор держится открытым: он дешевле открывать один раз, чем на
/// каждый идентификатор. Раньше он жил в `thread_local`; теперь это поле
/// окружения, и время его жизни совпадает со временем жизни прогона.
/// Криптографическая стойкость не обещается: УИД платформы —
/// идентификатор обмена, а не секрет.
#[derive(Default)]
pub struct SystemRandom {
    urandom: Option<std::fs::File>,
}

impl SystemRandom {
    fn read_urandom(&mut self, buffer: &mut [u8; 16]) -> bool {
        if self.urandom.is_none() {
            self.urandom = std::fs::File::open("/dev/urandom").ok();
        }
        match self.urandom.as_mut() {
            Some(file) => file.read_exact(buffer).is_ok(),
            None => false,
        }
    }

    /// Запасной источник без `/dev/urandom`: два независимых `RandomState`
    /// приходят со случайными ключами от ОС, и их хеши дают шестнадцать
    /// байтов.
    fn hash_random_state(buffer: &mut [u8; 16]) {
        use std::collections::hash_map::RandomState;
        use std::hash::{BuildHasher, Hasher};
        for half in 0..2 {
            let mut hasher = RandomState::new().build_hasher();
            hasher.write_u64(half as u64);
            buffer[half * 8..][..8].copy_from_slice(&hasher.finish().to_le_bytes());
        }
    }
}

impl RandomSource for SystemRandom {
    fn fill(&mut self, buffer: &mut [u8; 16]) {
        if !self.read_urandom(buffer) {
            Self::hash_random_state(buffer);
        }
    }
}

/// Окружение одного прогона.
///
/// Принадлежит вызывающему (`open_bsl::State`, `bsl-cli`), а не процессу:
/// две сессии одного движка могут видеть разные аргументы, разное время и
/// разную последовательность случайных байтов, в каком угодно порядке
/// запусков.
pub struct HostEnv {
    arguments: Vec<String>,
    clock: Box<dyn Clock>,
    random: Box<dyn RandomSource>,
    zone: std::rc::Rc<dyn TimeZone>,
}

impl HostEnv {
    /// Окружение процесса: пустой список аргументов, системные часы и
    /// системный источник случайности.
    #[must_use]
    pub fn process() -> Self {
        HostEnv {
            arguments: Vec::new(),
            clock: Box::new(SystemClock),
            random: Box::new(SystemRandom::default()),
            zone: std::rc::Rc::new(crate::tz::SystemTimeZone::new()),
        }
    }

    /// Аргументы, которые скрипт увидит в `АргументыКоманднойСтроки`.
    #[must_use]
    pub fn with_arguments(mut self, arguments: Vec<String>) -> Self {
        self.arguments = arguments;
        self
    }

    #[must_use]
    pub fn with_clock(mut self, clock: impl Clock + 'static) -> Self {
        self.clock = Box::new(clock);
        self
    }

    #[must_use]
    pub fn with_random(mut self, random: impl RandomSource + 'static) -> Self {
        self.random = Box::new(random);
        self
    }

    #[must_use]
    pub fn with_zone(mut self, zone: impl TimeZone + 'static) -> Self {
        self.zone = std::rc::Rc::new(zone);
        self
    }

    /// Зона прогона отдельной ссылкой: её забирает `CallContext`
    /// компонента, пока сам `HostEnv` занят чем-то ещё.
    #[must_use]
    pub fn zone(&self) -> std::rc::Rc<dyn TimeZone> {
        std::rc::Rc::clone(&self.zone)
    }

    #[must_use]
    pub fn arguments(&self) -> &[String] {
        &self.arguments
    }

    pub fn unix_millis(&mut self) -> i64 {
        self.clock.unix_millis()
    }

    pub fn fill_random(&mut self, buffer: &mut [u8; 16]) {
        self.random.fill(buffer);
    }
}

impl Default for HostEnv {
    fn default() -> Self {
        Self::process()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Источник, выдающий заданную последовательность, — та самая
    /// тестовая реализация, ради которой интерфейс и заведён.
    struct Sequence(Vec<[u8; 16]>);

    impl RandomSource for Sequence {
        fn fill(&mut self, buffer: &mut [u8; 16]) {
            *buffer = if self.0.is_empty() {
                [0; 16]
            } else {
                self.0.remove(0)
            };
        }
    }

    struct Ticking(i64);

    impl Clock for Ticking {
        fn unix_millis(&mut self) -> i64 {
            self.0 += 1000;
            self.0
        }
    }

    /// Разрешение МЕСТНОГО времени в смещение на синтетической зоне с
    /// одним переходом: в полдень 2 января часы уходят с +03 на +04.
    ///
    /// Проверяются оба особых часа, а не только «где-то рядом»: с
    /// переводом вперёд пропадает час местного времени, с переводом назад
    /// он проживается дважды. Правило снято с платформы (см. doc comment
    /// на `TimeZone::offset_for_local`).
    #[test]
    fn a_local_time_around_a_transition_resolves_the_measured_way() {
        /// Переход в момент `at`: до него `from`, после — `to`.
        struct Switch {
            at: i64,
            from: i32,
            to: i32,
        }

        impl TimeZone for Switch {
            fn offset_seconds(&self, unix_seconds: i64) -> i32 {
                if unix_seconds < self.at {
                    self.from
                } else {
                    self.to
                }
            }
        }

        // Перевод ВПЕРЁД (+03 -> +04) в 12:00 UTC: местные 15:00–15:59
        // второго января не существовали.
        let forward = Switch {
            at: 12 * 3600,
            from: 3 * 3600,
            to: 4 * 3600,
        };
        // Задолго до и задолго после — однозначно.
        assert_eq!(forward.offset_for_local(0), 3 * 3600);
        assert_eq!(forward.offset_for_local(20 * 3600), 4 * 3600);
        // Пропущенный час: ответ — смещение ДО перехода.
        assert_eq!(forward.offset_for_local(15 * 3600 + 1800), 3 * 3600);

        // Перевод НАЗАД (+04 -> +03) в том же месте: местные 15:00–15:59
        // прожиты дважды, и ответ — смещение ПОСЛЕ перехода.
        let back = Switch {
            at: 12 * 3600,
            from: 4 * 3600,
            to: 3 * 3600,
        };
        assert_eq!(back.offset_for_local(0), 4 * 3600);
        assert_eq!(back.offset_for_local(20 * 3600), 3 * 3600);
        assert_eq!(back.offset_for_local(15 * 3600 + 1800), 3 * 3600);

        // ЗАПАДНЫЕ зоны — та же задача с другим знаком, и решается она
        // тем же правилом только потому, что кандидаты берутся по обе
        // стороны перехода в пространстве моментов. Проба «на месте»
        // здесь брала кандидата не с той стороны: у однозначного 08:30
        // ниже она давала бы -04:00, которое само себя не подтверждает.
        let west_back = Switch {
            at: 12 * 3600,
            from: -4 * 3600,
            to: -5 * 3600,
        };
        // Однозначное местное время: единственное согласованное решение.
        assert_eq!(west_back.offset_for_local(8 * 3600 + 1800), -5 * 3600);
        assert_eq!(west_back.offset_for_local(0), -4 * 3600);
        assert_eq!(west_back.offset_for_local(20 * 3600), -5 * 3600);
        // Местные 07:00–07:59 прожиты дважды — второй проход.
        assert_eq!(west_back.offset_for_local(7 * 3600 + 1800), -5 * 3600);

        let west_forward = Switch {
            at: 12 * 3600,
            from: -5 * 3600,
            to: -4 * 3600,
        };
        // Местных 07:00–07:59 не было — смещение до перехода.
        assert_eq!(west_forward.offset_for_local(7 * 3600 + 1800), -5 * 3600);
        assert_eq!(west_forward.offset_for_local(0), -5 * 3600);
        assert_eq!(west_forward.offset_for_local(20 * 3600), -4 * 3600);

        // Крайние значения установленной базы `tzdata` — местное
        // солнечное время до первого перехода: `Asia/Manila` −15:56:08 и
        // `America/Metlakatla` +15:13:42. Оба дальше суток от нуля не
        // уходят, и договор их держит.
        for (from, to) in [(-57368, -8 * 3600), (54822, -8 * 3600)] {
            let zone = Switch {
                at: 12 * 3600,
                from,
                to,
            };
            assert!(from.abs() <= MAX_OFFSET_SECONDS, "договор нарушен: {from}");
            // Задолго до перехода — солнечное время, задолго после — зона.
            assert_eq!(zone.offset_for_local(-5 * 86400), from);
            assert_eq!(zone.offset_for_local(5 * 86400), to);
        }

        // ДВА перехода подряд: пока они дальше `MIN_TRANSITION_GAP_SECONDS`
        // друг от друга, правило работает на каждом из них по отдельности.
        // Ближе — договор нарушен, и ответ уже ничем не обещан.
        struct TwoSwitches {
            first: i64,
            second: i64,
        }

        impl TimeZone for TwoSwitches {
            fn offset_seconds(&self, unix_seconds: i64) -> i32 {
                if unix_seconds < self.first {
                    4 * 3600
                } else if unix_seconds < self.second {
                    3 * 3600
                } else {
                    2 * 3600
                }
            }
        }

        let gap = MIN_TRANSITION_GAP_SECONDS + 3600;
        let two = TwoSwitches {
            first: 0,
            second: gap,
        };
        // Час, прожитый дважды у ПЕРВОГО перехода (местные 03:00–03:59),
        // и он же у ВТОРОГО: оба разрешаются в смещение после своего
        // перехода, как измерено.
        assert_eq!(two.offset_for_local(3 * 3600 + 1800), 3 * 3600);
        assert_eq!(two.offset_for_local(gap + 2 * 3600 + 1800), 2 * 3600);
        // И однозначные точки между переходами и по краям.
        assert_eq!(two.offset_for_local(-5 * 86400), 4 * 3600);
        assert_eq!(two.offset_for_local(gap / 2), 3 * 3600);
        assert_eq!(two.offset_for_local(gap + 5 * 86400), 2 * 3600);

        // Неподвижная зона отвечает собой в любой момент, включая края
        // диапазона: подпись публичная и принимает весь `i64`, поэтому
        // вычитания внутри насыщающие, а не проверяемые только в debug.
        let fixed = FixedTimeZone::new(-5 * 3600 - 1800).expect("допустимое смещение");
        for wall in [0, 12 * 3600, -3 * 86400, 5 * 86400, i64::MIN, i64::MAX] {
            assert_eq!(fixed.offset_for_local(wall), -5 * 3600 - 1800);
        }
        for zone in [
            FixedTimeZone::new(14 * 3600).expect("допустимое смещение"),
            FixedTimeZone::new(-14 * 3600).expect("допустимое смещение"),
        ] {
            let _ = zone.offset_for_local(i64::MIN);
            let _ = zone.offset_for_local(i64::MAX);
        }
    }

    /// Конструктор зоны отсекает то, что нельзя записать как `+ЧЧ:ММ`.
    #[test]
    fn a_fixed_zone_takes_whole_minutes_within_fourteen_hours() {
        for good in [
            0,
            60,
            -60,
            3 * 3600,
            -5 * 3600 - 1800,
            14 * 3600,
            -14 * 3600,
        ] {
            let zone = FixedTimeZone::new(good).expect("допустимое смещение");
            assert_eq!(zone.offset_seconds(0), good);
        }
        for bad in [
            1,
            -1,
            59,
            3601,
            14 * 3600 + 60,
            -14 * 3600 - 60,
            i32::MAX,
            i32::MIN,
        ] {
            assert!(FixedTimeZone::new(bad).is_none(), "принято негодное: {bad}");
        }
    }

    #[test]
    fn a_given_random_sequence_comes_back_in_order() {
        let mut env = HostEnv::process().with_random(Sequence(vec![[1; 16], [2; 16]]));
        let mut buffer = [0u8; 16];
        env.fill_random(&mut buffer);
        assert_eq!(buffer, [1; 16]);
        env.fill_random(&mut buffer);
        assert_eq!(buffer, [2; 16]);
    }

    #[test]
    fn a_test_clock_advances_on_its_own_terms() {
        let mut env = HostEnv::process().with_clock(Ticking(0));
        assert_eq!(env.unix_millis(), 1000);
        assert_eq!(env.unix_millis(), 2000);
    }

    #[test]
    fn arguments_belong_to_the_environment_that_was_given_them() {
        let env = HostEnv::process().with_arguments(vec!["а".into(), "б".into()]);
        assert_eq!(env.arguments(), ["а", "б"]);
        assert!(HostEnv::process().arguments().is_empty());
    }

    /// Системный источник обязан давать разные байты: совпадение двух
    /// подряд — событие порядка 2^-128, то есть сломанный источник, а не
    /// невезение.
    #[test]
    fn the_process_random_source_does_not_repeat_itself() {
        let mut env = HostEnv::process();
        let (mut first, mut second) = ([0u8; 16], [0u8; 16]);
        env.fill_random(&mut first);
        env.fill_random(&mut second);
        assert_ne!(first, second);
    }

    /// Часы процесса идут вперёд от Unix-эпохи, а не отвечают нулём:
    /// `unwrap_or(0)` в них — обработка сломанных часов, а не норма.
    #[test]
    fn the_process_clock_is_past_the_unix_epoch() {
        // 2020-01-01 в миллисекундах — заведомо в прошлом.
        assert!(SystemClock.unix_millis() > 1_577_836_800_000);
    }
}
