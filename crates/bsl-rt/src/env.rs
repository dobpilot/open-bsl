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
//! Чего здесь СОЗНАТЕЛЬНО нет — местного смещения времени. Его читает не
//! путь часов (`ТекущаяДата` считает от Unix-эпохи и смещения не
//! применяет), а перевод УЖЕ ЗАПИСАННЫХ моментов в `bsl-json` и
//! `bsl-xml`, то есть код компонентных крейтов, которому доступен только
//! `CallContext`. Перенести смещение сюда — значит расширить ABI
//! компонентов; сделать это наполовину — значит завести второй источник
//! правды о зоне. Поэтому смещение остаётся возможностью процесса (см.
//! модуль `tz`), и это записано, а не умолчано.

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
