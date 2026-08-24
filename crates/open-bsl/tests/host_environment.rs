//! Окружение запуска принадлежит сессии, а не процессу.
//!
//! До этого аргументы жили в `OnceLock`, часы читались прямым
//! `SystemTime::now()`, а байты идентификатора — из `thread_local`. Две
//! изолированные `State` одного `Engine` поэтому делили окружение, и
//! проверить поведение на заданном времени было нечем.

use open_bsl::{Clock, Engine, RandomSource, StateBuilder, Value};

/// Неподвижные часы: любой вывод, зависящий от времени, становится
/// побайтово воспроизводимым.
struct FixedClock(i64);

impl Clock for FixedClock {
    fn unix_millis(&mut self) -> i64 {
        self.0
    }
}

/// Часы, шагающие ровно на секунду за обращение.
struct TickingClock(i64);

impl Clock for TickingClock {
    fn unix_millis(&mut self) -> i64 {
        self.0 += 1000;
        self.0
    }
}

/// Заданная последовательность байтов идентификатора.
struct Sequence(Vec<[u8; 16]>);

impl RandomSource for Sequence {
    fn fill(&mut self, buffer: &mut [u8; 16]) {
        *buffer = if self.0.is_empty() {
            [0xee; 16]
        } else {
            self.0.remove(0)
        };
    }
}

/// Все скрипты ниже возвращают СТРОКУ, собранную самим BSL: `Display` у
/// `Value` отладочный и пользовательского форматирования не воспроизводит
/// (см. `bsl_format::format_value`), а для строки он — сама строка.
fn text(value: &Value) -> String {
    match value {
        Value::Str(s) => s.to_string(),
        other => panic!("скрипт обязан вернуть строку, вернул {other:?}"),
    }
}

#[test]
fn two_states_of_one_engine_see_their_own_arguments() {
    let engine = Engine::builder().build().unwrap();
    let mut first = engine
        .state_builder()
        .arguments(vec!["раз".into(), "два".into()])
        .build();
    let mut second = engine.state_builder().arguments(vec!["три".into()]).build();

    // Порядок запусков ничего не решает: раньше первый же вызов
    // `set_command_line_args` выигрывал у всех последующих навсегда.
    let script = "а = АргументыКоманднойСтроки;\n\
                  р = \"\";\n\
                  Для Каждого э Из а Цикл\n\
                  р = р + э + \";\";\n\
                  КонецЦикла;\n\
                  Возврат р;";
    assert_eq!(text(&second.exec(script).unwrap()), "три;");
    assert_eq!(text(&first.exec(script).unwrap()), "раз;два;");
    assert_eq!(text(&second.exec(script).unwrap()), "три;");

    // Сессия без аргументов видит пустой массив, а не чужие.
    let mut bare = engine.new_state();
    assert_eq!(text(&bare.exec(script).unwrap()), "");
}

#[test]
fn a_fixed_clock_makes_the_time_reproducible() {
    let engine = Engine::builder().build().unwrap();
    // 2020-01-02 03:04:05 UTC.
    let mut state = engine
        .state_builder()
        .clock(FixedClock(1_577_934_245_000))
        .build();

    let now = "Возврат Строка(ТекущаяДата());";
    assert_eq!(text(&state.exec(now).unwrap()), "02.01.2020 3:04:05");
    // Дважды подряд — то же самое значение: часы неподвижны.
    assert_eq!(text(&state.exec(now).unwrap()), "02.01.2020 3:04:05");
    // Миллисекунды считаются от эпохи дат BSL, а не от Unix-эпохи.
    assert_eq!(
        text(&state.exec(MILLIS).unwrap()),
        "63713531045000",
        "миллисекунды считаются от эпохи дат BSL"
    );
}

/// Миллисекунды без разделителей групп: сравнивать удобнее, а вопрос теста
/// не про форматирование числа.
const MILLIS: &str = "Возврат Формат(ТекущаяУниверсальнаяДатаВМиллисекундах(), \"ЧГ=0\");";

/// Unix-эпоха в миллисекундах от эпохи дат BSL.
const UNIX_EPOCH_MILLIS: i64 = 62_135_596_800_000;

#[test]
fn two_states_do_not_share_a_clock() {
    let engine = Engine::builder().build().unwrap();
    let mut ticking = engine.state_builder().clock(TickingClock(0)).build();
    let mut fixed = engine.state_builder().clock(FixedClock(0)).build();

    let base = UNIX_EPOCH_MILLIS;
    assert_eq!(
        text(&ticking.exec(MILLIS).unwrap()),
        (base + 1000).to_string()
    );
    // Чужой прогон между двумя нашими не сдвигает нашу последовательность.
    assert_eq!(text(&fixed.exec(MILLIS).unwrap()), base.to_string());
    assert_eq!(
        text(&ticking.exec(MILLIS).unwrap()),
        (base + 2000).to_string()
    );
}

const UUID: &str = "Возврат Строка(Новый УникальныйИдентификатор());";

#[test]
fn a_given_random_source_produces_the_expected_identifier() {
    let engine = Engine::builder().build().unwrap();
    let mut state = engine
        .state_builder()
        .random(Sequence(vec![[0x00; 16], [0xff; 16]]))
        .build();

    // Биты версии и варианта расставляет рантайм, а не источник: заданные
    // нули дают `...-4000-8000-...`, заданные единицы — `...-4fff-bfff-...`.
    assert_eq!(
        text(&state.exec(UUID).unwrap()),
        "00000000-0000-4000-8000-000000000000"
    );
    assert_eq!(
        text(&state.exec(UUID).unwrap()),
        "ffffffff-ffff-4fff-bfff-ffffffffffff"
    );
}

#[test]
fn states_do_not_consume_each_others_random_sequence() {
    let engine = Engine::builder().build().unwrap();
    let script = UUID;
    let mut first = engine
        .state_builder()
        .random(Sequence(vec![[0x11; 16], [0x22; 16]]))
        .build();
    let mut second = engine
        .state_builder()
        .random(Sequence(vec![[0x33; 16]]))
        .build();

    let a = text(&first.exec(script).unwrap());
    let b = text(&second.exec(script).unwrap());
    let c = text(&first.exec(script).unwrap());

    assert!(a.starts_with("11111111"), "{a}");
    assert!(b.starts_with("33333333"), "{b}");
    // Второй прогон первой сессии берёт СВОЙ следующий элемент, а не то,
    // что осталось после соседа.
    assert!(c.starts_with("22222222"), "{c}");
}

/// Контракт «окружение НЕ ПОПАДАЕТ в JIT» — не пожелание, а условие
/// корректности. У нативного пути `CallContext` — сток (компонентные
/// методы и свойства в stdout не пишут), а `HostIo::env` там `None`:
/// инструкция, которой окружение нужно, не «отработает медленнее», а
/// вернёт ошибку прогона. Держится это тем, что три встроенные функции
/// окружения и `Новый УникальныйИдентификатор` в JIT не компилируются
/// вовсе, и прогон уходит на них в интерпретатор.
///
/// Цикл в скрипте — не про скорость: точкой входа JIT делает только
/// достаточно длинную цепочку компилируемых инструкций, и без неё тест
/// проверил бы интерпретатор во второй раз. Арифметика внутри
/// компилируется, обращение к окружению — нет, и прогон переключается
/// между двумя режимами на каждом витке.
///
/// На не-x86-64 флаг принимается и игнорируется — тест там сводится к
/// повторному прогону интерпретатора и остаётся зелёным.
mod jit {
    use super::*;

    /// Тот же скрипт под обоими режимами: расхождение — это и есть
    /// поломка контракта, поэтому сравниваются они между собой, а
    /// результат вдобавок сверяется с ожидаемым (иначе «сломаны
    /// одинаково» прошло бы как успех).
    fn both_modes(script: &str, configure: impl Fn(StateBuilder) -> StateBuilder, expected: &str) {
        let engine = Engine::builder().build().unwrap();
        let mut interpreted = configure(engine.state_builder().jit(false)).build();
        let mut compiled = configure(engine.state_builder().jit(true)).build();

        let a = text(&interpreted.exec(script).unwrap());
        let b = text(&compiled.exec(script).unwrap());
        assert_eq!(a, b, "режимы разошлись");
        assert_eq!(a, expected, "интерпретатор");
    }

    /// Тело с достаточной цепочкой арифметики, чтобы чанк попал в JIT:
    /// внутренний цикл даёт сумму нечётных чисел, то есть 40*40.
    const WARM: &str = "итог = \"\";\n\
                        Для к = 1 По 3 Цикл\n\
                        н = 0;\n\
                        Для ж = 1 По 40 Цикл\n\
                        н = н + ж * 2 - 1;\n\
                        КонецЦикла;\n\
                        итог = итог + Формат(н, \"ЧГ=0\") + \"=\" + ПРОБА + \";\";\n\
                        КонецЦикла;\n\
                        Возврат итог;";

    fn warm_with(probe: &str) -> String {
        WARM.replace("ПРОБА", probe)
    }

    #[test]
    fn a_fixed_clock_survives_a_jit_run() {
        both_modes(
            &warm_with("Формат(ТекущаяУниверсальнаяДатаВМиллисекундах(), \"ЧГ=0\")"),
            |b| b.clock(TickingClock(0)),
            // Часы шагают на секунду за обращение: три витка — три
            // разных значения, и все они от эпохи дат BSL.
            &format!(
                "1600={};1600={};1600={};",
                UNIX_EPOCH_MILLIS + 1000,
                UNIX_EPOCH_MILLIS + 2000,
                UNIX_EPOCH_MILLIS + 3000
            ),
        );
    }

    #[test]
    fn the_arguments_survive_a_jit_run() {
        both_modes(
            &warm_with("АргументыКоманднойСтроки[0]"),
            |b| b.arguments(vec!["раз".into(), "два".into()]),
            "1600=раз;1600=раз;1600=раз;",
        );
    }

    #[test]
    fn a_given_random_source_survives_a_jit_run() {
        both_modes(
            &warm_with("Строка(Новый УникальныйИдентификатор())"),
            |b| b.random(Sequence(vec![[0x11; 16], [0x22; 16], [0x33; 16]])),
            "1600=11111111-1111-4111-9111-111111111111;\
             1600=22222222-2222-4222-a222-222222222222;\
             1600=33333333-3333-4333-b333-333333333333;",
        );
    }
}

/// Часовой пояс — четвёртая возможность окружения, и попадает к
/// потребителю не так, как три остальные: его читает не ядро, а КОД
/// КОМПОНЕНТА, которому доступен только `CallContext`. Поэтому зона едет
/// туда ссылкой, а не через `HostEnv`, и проверка тут не «часы отвечают
/// заданным», а «две сессии одного движка толкуют один и тот же момент в
/// разных зонах».
mod zone {
    use super::*;
    use open_bsl::FixedTimeZone;

    /// Дата со СМЕЩЕНИЕМ: единственная запись, в которой зона видна в
    /// выводе целиком. Момент взят до всякого перехода на летнее время,
    /// чтобы неподвижная зона была честной моделью.
    const WRITE_WITH_OFFSET: &str = "Д = Дата(2014, 5, 10, 13, 14, 15);\n\
         Возврат ЗаписатьДатуJSON(Д, ФорматДатыJSON.ISO, \
         ВариантЗаписиДатыJSON.ЛокальнаяДатаСоСмещением);";

    #[test]
    fn two_states_of_one_engine_see_their_own_zone() {
        let engine = Engine::builder().build().unwrap();
        let mut east = engine
            .state_builder()
            .zone(FixedTimeZone::new(3 * 3600).expect("допустимое смещение"))
            .build();
        let mut west = engine
            .state_builder()
            .zone(FixedTimeZone::new(-5 * 3600 - 1800).expect("допустимое смещение"))
            .build();

        // Порядок запусков ничего не решает: зона не процессная и не
        // «первая победила», как было с кэшем `/etc/localtime`.
        assert_eq!(
            text(&east.exec(WRITE_WITH_OFFSET).unwrap()),
            "2014-05-10T13:14:15+03:00"
        );
        assert_eq!(
            text(&west.exec(WRITE_WITH_OFFSET).unwrap()),
            "2014-05-10T13:14:15-05:30"
        );
        assert_eq!(
            text(&east.exec(WRITE_WITH_OFFSET).unwrap()),
            "2014-05-10T13:14:15+03:00"
        );
    }

    /// Универсальная запись вычитает смещение — значит зона видна и там,
    /// где её самой в выводе нет.
    #[test]
    fn the_zone_shifts_the_universal_variant() {
        let engine = Engine::builder().build().unwrap();
        let script = "Д = Дата(2014, 5, 10, 13, 14, 15);\n\
                      Возврат ЗаписатьДатуJSON(Д, ФорматДатыJSON.ISO, \
                      ВариантЗаписиДатыJSON.УниверсальнаяДата);";
        let mut utc = engine
            .state_builder()
            .zone(FixedTimeZone::new(0).expect("допустимое смещение"))
            .build();
        let mut east = engine
            .state_builder()
            .zone(FixedTimeZone::new(3 * 3600).expect("допустимое смещение"))
            .build();
        assert_eq!(text(&utc.exec(script).unwrap()), "2014-05-10T13:14:15Z");
        assert_eq!(text(&east.exec(script).unwrap()), "2014-05-10T10:14:15Z");
    }

    /// Обратный ход: строка с явным поясом читается в местное время СВОЕЙ
    /// сессии. Без зоны прогона обе сессии дали бы одно и то же.
    #[test]
    fn reading_a_dated_string_lands_in_the_sessions_own_zone() {
        let engine = Engine::builder().build().unwrap();
        let script = "Д = ПрочитатьДатуJSON(\"2014-05-10T13:14:15Z\", ФорматДатыJSON.ISO);\n\
                      Возврат Формат(Д, \"ДФ=yyyy-MM-dd HH:mm:ss\");";
        let mut utc = engine
            .state_builder()
            .zone(FixedTimeZone::new(0).expect("допустимое смещение"))
            .build();
        let mut east = engine
            .state_builder()
            .zone(FixedTimeZone::new(3 * 3600).expect("допустимое смещение"))
            .build();
        assert_eq!(text(&utc.exec(script).unwrap()), "2014-05-10 13:14:15");
        assert_eq!(text(&east.exec(script).unwrap()), "2014-05-10 16:14:15");
    }

    /// Фабрика XDTO запоминает зону ТОГО ПРОГОНА, в котором построена,
    /// и толкует в ней лексические формы с поясом.
    ///
    /// Проверяется через настоящую границу — `СоздатьФабрикуXDTO` в
    /// скрипте, то есть `State` -> VM -> `CallContext::zone_rc` -> модель,
    /// — а не прямым вызовом построителя модели: потеря зоны на любом
    /// звене этой цепочки обязана тест уронить.
    #[test]
    fn an_xdto_factory_keeps_the_zone_of_the_run_that_built_it() {
        let schema = std::env::temp_dir().join("open-bsl-zone-factory.xsd");
        std::fs::write(
            &schema,
            "<?xml version=\"1.0\"?>\n\
             <xs:schema xmlns:xs=\"http://www.w3.org/2001/XMLSchema\" \
             targetNamespace=\"urn:z\">\n\
             <xs:simpleType name=\"Момент\">\n\
             <xs:restriction base=\"xs:dateTime\"/>\n\
             </xs:simpleType>\n\
             </xs:schema>\n",
        )
        .expect("схема пишется во временный файл");

        let script = format!(
            "ф = СоздатьФабрикуXDTO(\"{}\");\n\
             з = ф.Создать(ф.Тип(\"urn:z\", \"Момент\"), \"2026-08-12T18:41:17Z\");\n\
             Возврат Формат(з.Значение, \"ДФ=HH:mm:ss\");",
            schema.to_string_lossy()
        );

        let engine = Engine::builder().build().unwrap();
        let mut east = engine
            .state_builder()
            .zone(FixedTimeZone::new(3 * 3600).expect("допустимое смещение"))
            .build();
        let mut utc = engine.state_builder().zone(FixedTimeZone::UTC).build();

        // Тот же момент, две фабрики, два ответа — измеренный пересчёт
        // (`facets::apply_zone`), но зона теперь из сессии.
        assert_eq!(text(&east.exec(&script).unwrap()), "21:41:17");
        assert_eq!(text(&utc.exec(&script).unwrap()), "18:41:17");

        let _ = std::fs::remove_file(&schema);
    }

    /// Тот же контракт, что у остальных возможностей окружения, — под
    /// JIT.
    ///
    /// Работает он здесь не потому, что зона доезжает до нативного пути,
    /// а потому, что доезжать ей некуда: `ЗаписатьДатуJSON` — ГЛОБАЛЬНАЯ
    /// функция компонента, а из компонентного JIT компилирует только
    /// объектные опкоды. Сам нативный путь зоны не получает — известное
    /// ограничение, закреплённое тестом
    /// `under_the_jit_a_host_reader_of_the_zone_gets_an_error` в
    /// `embedding.rs`, там же и измерение, почему так.
    #[test]
    fn the_zone_survives_a_jit_run() {
        let engine = Engine::builder().build().unwrap();
        let script = "итог = \"\";\n\
                      Для к = 1 По 3 Цикл\n\
                      н = 0;\n\
                      Для ж = 1 По 40 Цикл\n\
                      н = н + ж * 2 - 1;\n\
                      КонецЦикла;\n\
                      Д = Дата(2014, 5, 10, 13, 14, 15);\n\
                      итог = итог + Формат(н, \"ЧГ=0\") + \"=\" \
                      + ЗаписатьДатуJSON(Д, ФорматДатыJSON.ISO, \
                      ВариантЗаписиДатыJSON.ЛокальнаяДатаСоСмещением) + \";\";\n\
                      КонецЦикла;\n\
                      Возврат итог;";
        let expected = "1600=2014-05-10T13:14:15+03:00;\
                        1600=2014-05-10T13:14:15+03:00;\
                        1600=2014-05-10T13:14:15+03:00;";
        for jit in [false, true] {
            let mut state = engine
                .state_builder()
                .jit(jit)
                .zone(FixedTimeZone::new(3 * 3600).expect("допустимое смещение"))
                .build();
            assert_eq!(text(&state.exec(script).unwrap()), expected, "jit={jit}");
        }
    }
}

/// Файловая система — четвёртая возможность прогона, и пока она накрывает
/// только операции «файл целиком»: `ЗначениеВФайл`, `ЗначениеИзФайла` и
/// `Новый ДвоичныеДанные(путь)`. Компонентные объекты ходят в `std::fs`
/// напрямую — вторая и третья волны, см. обзор `bsl_rt::FileSystem`.
mod files {
    use super::*;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::rc::Rc;

    /// Файловая система в памяти — та самая тестовая реализация, ради
    /// которой возможность и заводилась: до неё проверить чтение и запись
    /// можно было только через настоящий диск и временный каталог.
    #[derive(Default, Clone)]
    struct MemoryFiles(Rc<RefCell<HashMap<String, Vec<u8>>>>);

    impl open_bsl::FileSystem for MemoryFiles {
        fn read(&self, path: &str) -> std::io::Result<Vec<u8>> {
            self.0.borrow().get(path).cloned().ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::NotFound, format!("нет файла {path}"))
            })
        }

        fn write(&self, path: &str, data: &[u8]) -> std::io::Result<()> {
            self.0.borrow_mut().insert(path.to_string(), data.to_vec());
            Ok(())
        }

        // Эти тесты работают только с «файлом целиком»; операции с
        // метаданными и дескрипторами не задействованы.
        fn metadata(&self, path: &str) -> std::io::Result<open_bsl::FileMetadata> {
            unsupported(path)
        }

        fn read_dir<'fs>(
            &'fs self,
            path: &str,
        ) -> std::io::Result<Box<dyn Iterator<Item = std::io::Result<open_bsl::DirEntry>> + 'fs>>
        {
            unsupported(path)
        }

        fn create_dir_all(&self, _path: &str) -> std::io::Result<()> {
            Ok(())
        }

        fn open(
            &self,
            path: &str,
            _options: open_bsl::FileOpenOptions,
        ) -> std::io::Result<Box<dyn open_bsl::FileHandle>> {
            unsupported(path)
        }
    }

    fn unsupported<T>(path: &str) -> std::io::Result<T> {
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            format!("операция не поддержана тестовой ФС: {path}"),
        ))
    }

    #[test]
    fn two_states_write_into_their_own_file_systems() {
        let engine = Engine::builder().build().unwrap();
        let first = MemoryFiles::default();
        let second = MemoryFiles::default();
        let mut a = engine.state_builder().files(first.clone()).build();
        let mut b = engine.state_builder().files(second.clone()).build();

        a.exec("ЗначениеВФайл(\"общий.txt\", \"из первой\");")
            .unwrap();
        b.exec("ЗначениеВФайл(\"общий.txt\", \"из второй\");")
            .unwrap();

        // Одно и то же имя, две сессии — и ни одна не видит чужой файл.
        assert_eq!(
            text(&a.exec("Возврат ЗначениеИзФайла(\"общий.txt\");").unwrap()),
            "из первой"
        );
        assert_eq!(
            text(&b.exec("Возврат ЗначениеИзФайла(\"общий.txt\");").unwrap()),
            "из второй"
        );
        // И на настоящем диске не осталось ничего.
        assert!(!std::path::Path::new("общий.txt").exists());
    }

    /// `Новый ДвоичныеДанные(путь)` — отдельный опкод VM, а не встроенная
    /// функция, поэтому проверяется своим прогоном.
    #[test]
    fn binary_data_reads_through_the_sessions_file_system() {
        let engine = Engine::builder().build().unwrap();
        let disk = MemoryFiles::default();
        disk.0
            .borrow_mut()
            .insert("данные.bin".to_string(), vec![1, 2, 3, 250]);
        let mut state = engine.state_builder().files(disk).build();

        assert_eq!(
            text(
                &state
                    .exec(
                        "д = Новый ДвоичныеДанные(\"данные.bin\");\n\
                         Возврат Формат(д.Размер(), \"ЧГ=0\");"
                    )
                    .unwrap()
            ),
            "4"
        );
    }

    /// Ошибка файловой системы — ловимое `Попыткой` исключение, а не
    /// паника, и в ОБЕ стороны: чтение отсутствующего файла и отказ
    /// записи. Реализация возвращает `io::Error`, рантайм переводит его в
    /// `RtError`, как и раньше у `std::fs`.
    #[test]
    fn a_file_system_failure_is_a_catchable_error_both_ways() {
        /// Система, у которой нет ни одного файла и запись всегда
        /// отказывает: у отказа записи своя ветка перевода ошибки.
        struct Broken;

        impl open_bsl::FileSystem for Broken {
            fn read(&self, path: &str) -> std::io::Result<Vec<u8>> {
                Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("нет файла {path}"),
                ))
            }

            fn write(&self, _path: &str, _data: &[u8]) -> std::io::Result<()> {
                Err(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "только чтение",
                ))
            }

            fn metadata(&self, _path: &str) -> std::io::Result<open_bsl::FileMetadata> {
                Err(std::io::Error::new(std::io::ErrorKind::NotFound, "нет"))
            }

            fn read_dir<'fs>(
                &'fs self,
                _path: &str,
            ) -> std::io::Result<Box<dyn Iterator<Item = std::io::Result<open_bsl::DirEntry>> + 'fs>>
            {
                Err(std::io::Error::new(std::io::ErrorKind::NotFound, "нет"))
            }

            fn create_dir_all(&self, _path: &str) -> std::io::Result<()> {
                Err(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "только чтение",
                ))
            }

            fn open(
                &self,
                _path: &str,
                _options: open_bsl::FileOpenOptions,
            ) -> std::io::Result<Box<dyn open_bsl::FileHandle>> {
                Err(std::io::Error::new(std::io::ErrorKind::NotFound, "нет"))
            }
        }

        let engine = Engine::builder().build().unwrap();
        let catch = |probe: &str| {
            format!(
                "Попытка\n\
                 {probe}\n\
                 Исключение\n\
                 Возврат \"поймано\";\n\
                 КонецПопытки;\n\
                 Возврат \"не поймано\";"
            )
        };
        for probe in [
            "з = ЗначениеИзФайла(\"нет-такого\");",
            "ЗначениеВФайл(\"куда-нибудь\", \"текст\");",
            "д = Новый ДвоичныеДанные(\"нет-такого\");",
        ] {
            let mut state = engine.state_builder().files(Broken).build();
            assert_eq!(
                text(&state.exec(&catch(probe)).unwrap()),
                "поймано",
                "{probe}"
            );
        }
    }

    /// Заданная файловая система работает и ПОД JIT — всеми ТРЕМЯ
    /// путями: две встроенные функции и отдельный опкод
    /// `Новый ДвоичныеДанные`. Держится это тем, что нативный путь ни
    /// одного из них не компилирует, как и функции окружения; тест
    /// закрепляет публичный контракт, а не устройство списка исключений,
    /// поэтому опкод обязан быть в скрипте, а не только в комментарии.
    #[test]
    fn the_file_system_survives_a_jit_run() {
        let engine = Engine::builder().build().unwrap();
        let script = "итог = \"\";\n\
                      Для к = 1 По 3 Цикл\n\
                      н = 0;\n\
                      Для ж = 1 По 40 Цикл\n\
                      н = н + ж * 2 - 1;\n\
                      КонецЦикла;\n\
                      ЗначениеВФайл(\"виток.txt\", Формат(н, \"ЧГ=0\"));\n\
                      д = Новый ДвоичныеДанные(\"двоичное\");\n\
                      итог = итог + ЗначениеИзФайла(\"виток.txt\")\n\
                      + \"/\" + Формат(д.Размер(), \"ЧГ=0\") + \";\";\n\
                      КонецЦикла;\n\
                      Возврат итог;";
        for jit in [false, true] {
            let disk = MemoryFiles::default();
            disk.0
                .borrow_mut()
                .insert("двоичное".to_string(), vec![7; 5]);
            let mut state = engine.state_builder().jit(jit).files(disk.clone()).build();
            assert_eq!(
                text(&state.exec(script).unwrap()),
                "1600/5;1600/5;1600/5;",
                "jit={jit}"
            );
            // Файл лёг в заданную систему, а не на диск.
            assert!(disk.0.borrow().contains_key("виток.txt"), "jit={jit}");
            assert!(!std::path::Path::new("виток.txt").exists());
        }
    }
}
