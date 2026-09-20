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
    let universal = "Возврат Строка(ТекущаяУниверсальнаяДата());";
    assert_eq!(text(&state.exec(universal).unwrap()), "02.01.2020 3:04:05");
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

/// Проверки того, что внедрённые сервисы окружения доходят до встроенных
/// функций и компонентов одного запуска.
///
mod injected_services {
    use super::*;

    fn run_with(script: &str, configure: impl Fn(StateBuilder) -> StateBuilder, expected: &str) {
        let engine = Engine::builder().build().unwrap();
        let mut state = configure(engine.state_builder()).build();
        assert_eq!(text(&state.exec(script).unwrap()), expected);
    }

    /// Тело повторяет обращение к сервису окружения на нескольких витках:
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
    fn a_fixed_clock_is_used_by_the_run() {
        run_with(
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
    fn the_arguments_are_used_by_the_run() {
        run_with(
            &warm_with("АргументыКоманднойСтроки[0]"),
            |b| b.arguments(vec!["раз".into(), "два".into()]),
            "1600=раз;1600=раз;1600=раз;",
        );
    }

    #[test]
    fn a_given_random_source_is_used_by_the_run() {
        run_with(
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

    #[test]
    fn the_zone_is_used_by_component_functions() {
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
        let mut state = engine
            .state_builder()
            .zone(FixedTimeZone::new(3 * 3600).expect("допустимое смещение"))
            .build();
        assert_eq!(text(&state.exec(script).unwrap()), expected);
    }
}

/// Файловая система — четвёртая возможность прогона. Через неё идут и
/// операции «файл целиком» (`ЗначениеВФайл`, `ЗначениеИзФайла`, `Новый
/// ДвоичныеДанные(путь)`), и компонентные объекты: файловая возможность
/// приходит им в контексте вызова (ABI-G), а не прямым `std::fs`.
mod files {
    use super::*;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::rc::Rc;

    /// Файловая система в памяти — та самая тестовая реализация, ради
    /// которой возможность и заводилась: до неё проверить чтение и запись
    /// можно было только через настоящий диск и временный каталог.
    #[derive(Debug, Default, Clone)]
    struct MemoryFiles(
        Rc<RefCell<HashMap<String, Vec<u8>>>>,
        Rc<RefCell<HashMap<String, i64>>>,
        Rc<RefCell<HashMap<String, bool>>>,
        Rc<RefCell<HashMap<String, bool>>>,
    );

    #[test]
    fn file_hidden_is_a_host_attribute_not_a_filename_convention() {
        let engine = Engine::builder().build().unwrap();
        let files = MemoryFiles::default();
        files
            .0
            .borrow_mut()
            .insert("visible-name".into(), b"ABC".to_vec());
        let mut state = engine.state_builder().files(files.clone()).build();
        assert_eq!(
            text(
                &state
                    .exec(
                        r#"
            Ф = Новый Файл("visible-name");
            Попытка А = Ф.GetHidden(); Исключение Возврат "unknown"; КонецПопытки;
        "#
                    )
                    .unwrap()
            ),
            "unknown"
        );
        assert_eq!(text(&state.exec(r#"
            Ф = Новый Файл("visible-name");
            Ф.УстановитьНевидимость(Истина);
            Если Не Ф.GetHidden() Тогда ВызватьИсключение; КонецЕсли;
            Ф.SetHidden(Ложь);
            Если Ф.ПолучитьНевидимость() Тогда ВызватьИсключение; КонецЕсли;
            Попытка Ф.SetHidden(Неопределено); Исключение Возврат Строка(Не Ф.GetHidden()); КонецПопытки;
            Возврат "not rejected";
        "#).unwrap()), "Да");
        assert_eq!(files.3.borrow().get("visible-name"), Some(&false));
        assert_eq!(files.0.borrow().get("visible-name").unwrap(), b"ABC");
        assert_eq!(files.0.borrow().len(), 1);
        assert!(
            state
                .exec(r#"Ф = Новый Файл("visible-name"); Возврат Ф.SetHidden(Истина);"#)
                .is_err()
        );
        assert_eq!(files.3.borrow().get("visible-name"), Some(&false));
        assert!(
            state
                .exec(r#"Ф = Новый Файл("missing"); Ф.SetHidden(Истина);"#)
                .is_err()
        );
        assert!(!files.0.borrow().contains_key("missing"));
    }

    #[test]
    fn file_read_only_uses_current_host_attributes_and_measured_arguments() {
        let engine = Engine::builder().build().unwrap();
        let files = MemoryFiles::default();
        files
            .0
            .borrow_mut()
            .insert("attribute".into(), b"ABC".to_vec());
        let mut state = engine.state_builder().files(files.clone()).build();
        assert_eq!(
            text(
                &state
                    .exec(
                        r#"
            Ф = Новый Файл("attribute");
            Попытка А = Ф.GetReadOnly(); Исключение Возврат "unknown"; КонецПопытки;
        "#
                    )
                    .unwrap()
            ),
            "unknown"
        );
        assert_eq!(text(&state.exec(r#"
            Ф = Новый Файл("attribute");
            Ф.УстановитьТолькоЧтение(1);
            Если Не Ф.GetReadOnly() Тогда ВызватьИсключение; КонецЕсли;
            Ф.SetReadOnly("Ложь");
            Если Ф.ПолучитьТолькоЧтение() Тогда ВызватьИсключение; КонецЕсли;
            Ф.SetReadOnly(Истина);
            Попытка Ф.SetReadOnly(Неопределено); Исключение Возврат Строка(Ф.GetReadOnly()); КонецПопытки;
            Возврат "not rejected";
        "#).unwrap()), "Да");
        assert_eq!(files.2.borrow().get("attribute"), Some(&true));
        assert_eq!(files.0.borrow().get("attribute").unwrap(), b"ABC");
        assert!(
            state
                .exec(r#"Ф = Новый Файл("attribute"); Возврат Ф.SetReadOnly(Ложь);"#)
                .is_err()
        );
        assert_eq!(files.2.borrow().get("attribute"), Some(&true));
        assert!(
            state
                .exec(r#"Ф = Новый Файл("missing"); Ф.SetReadOnly(Ложь);"#)
                .is_err()
        );
        assert!(!files.0.borrow().contains_key("missing"));
    }

    #[test]
    fn file_times_use_host_metadata_and_the_session_zone() {
        let engine = Engine::builder().build().unwrap();
        let files = MemoryFiles::default();
        files.0.borrow_mut().insert("time".into(), b"ABC".to_vec());
        let mut state = engine
            .state_builder()
            .files(files.clone())
            .zone(open_bsl::FixedTimeZone::new(10800).unwrap())
            .build();
        assert_eq!(text(&state.exec(r#"
            Ф = Новый Файл("time");
            Ф.УстановитьВремяИзменения(Дата(2020, 1, 15, 12, 34, 56));
            Если Ф.GetModificationUniversalTime() <> Дата(2020, 1, 15, 9, 34, 56) Тогда ВызватьИсключение; КонецЕсли;
            Если Ф.GetModificationTime() <> Дата(2020, 1, 15, 12, 34, 56) Тогда ВызватьИсключение; КонецЕсли;
            Ф.SetModificationUniversalTime("20210314151617");
            Если Ф.ПолучитьУниверсальноеВремяИзменения() <> Дата(2021, 3, 14, 15, 16, 17) Тогда ВызватьИсключение; КонецЕсли;
            Если Ф.ПолучитьВремяИзменения() <> Дата(2021, 3, 14, 18, 16, 17) Тогда ВызватьИсключение; КонецЕсли;
            Ф.SetModificationTime("20210314151617");
            Возврат Строка(Ф.ПолучитьУниверсальноеВремяИзменения() = Дата(2021, 3, 14, 12, 16, 17));
        "#).unwrap()), "Да");
        assert_eq!(files.1.borrow().get("time"), Some(&1_615_724_177));
        assert_eq!(files.0.borrow().get("time").unwrap(), b"ABC");
        for method in [
            "УстановитьВремяИзменения",
            "SetModificationTime",
            "УстановитьУниверсальноеВремяИзменения",
            "SetModificationUniversalTime",
        ] {
            for argument in ["17", "Неопределено"] {
                let script = format!(
                    "Ф = Новый Файл(\"time\"); Попытка Ф.{method}({argument}); Исключение Возврат \"caught\"; КонецПопытки;"
                );
                assert_eq!(text(&state.exec(&script).unwrap()), "caught");
                assert_eq!(files.1.borrow().get("time"), Some(&1_615_724_177));
            }
        }
        files.1.borrow_mut().clear();
        for method in ["GetModificationTime", "GetModificationUniversalTime"] {
            let script = format!(
                "Ф = Новый Файл(\"time\"); Попытка Д = Ф.{method}(); Исключение Возврат \"unknown\"; КонецПопытки;"
            );
            assert_eq!(text(&state.exec(&script).unwrap()), "unknown");
        }
        // Число mtime и сдвиг getter взяты из file-time-edges.host/platform.txt.
        assert_eq!(
            text(
                &state
                    .exec(
                        r#"
            Ф = Новый Файл("time");
            Ф.SetModificationUniversalTime(Дата(1969, 12, 31, 23, 59, 59));
            Если Ф.GetModificationTime() <> Дата(1, 1, 1) Тогда ВызватьИсключение; КонецЕсли;
            Возврат Строка(Ф.GetModificationUniversalTime() - Дата(1969, 12, 31, 23, 59, 58) = 0.8384);
        "#
                    )
                    .unwrap()
            ),
            "Да"
        );
        assert_eq!(files.1.borrow().get("time"), Some(&1_844_674_407_370_954));
        assert!(
            state
                .exec(r#"Ф = Новый Файл("time"); Возврат Ф.SetModificationTime(Дата(2020, 1, 1));"#)
                .is_err()
        );
        assert_eq!(files.1.borrow().get("time"), Some(&1_844_674_407_370_954));
    }

    #[test]
    fn explicit_undefined_recursion_differs_from_an_omitted_argument() {
        let engine = Engine::builder().build().unwrap();
        let mut state = engine.state_builder().files(MemoryFiles::default()).build();
        for name in ["НайтиФайлы", "FindFiles"] {
            assert_eq!(
                text(
                    &state
                        .exec(&format!(
                            "Возврат Строка({name}(\"missing\", , ).Количество());"
                        ))
                        .unwrap()
                ),
                "0"
            );
            assert_eq!(
                text(
                    &state
                        .exec(&format!(
                            "Возврат Строка({name}(\"missing\").Количество());"
                        ))
                        .unwrap()
                ),
                "0"
            );
            assert_eq!(text(&state.exec(&format!(
                "Попытка Р = {name}(\"missing\", Неопределено, Неопределено); Исключение Возврат \"error\"; КонецПопытки; Возврат \"accepted\";"
            )).unwrap()), "error");
        }
    }

    #[test]
    fn find_files_returns_objects_bound_to_the_session() {
        let engine = Engine::builder().build().unwrap();
        let files = MemoryFiles::default();
        files.0.borrow_mut().insert("probe".into(), b"ABC".to_vec());
        let mut state = engine.state_builder().files(files.clone()).build();
        let result = state
            .exec(
                r#"
            Р = НайтиФайлы("probe");
            Ф = Р[0];
            Д = ПолучитьДвоичныеДанныеИзСтроки("ABCDE", КодировкаТекста.UTF8, Ложь);
            Д.Записать("probe");
            Возврат Ф.FullName + "|" + Строка(Ф.Size()) + "|"
                + Строка(FindFiles("missing", Неопределено, Ложь).Count());
        "#,
            )
            .unwrap();
        assert_eq!(text(&result), "probe|5|0");
        for source in [
            "Возврат НайтиФайлы();",
            "Возврат FindFiles(\"x\", \"*\", Истина, 1);",
        ] {
            assert!(state.exec(source).is_err());
        }
    }

    #[test]
    fn file_object_observes_updates_in_its_host_filesystem() {
        let engine = Engine::builder().build().unwrap();
        let files = MemoryFiles::default();
        let mut state = engine.state_builder().files(files.clone()).build();
        let result = state
            .exec(
                "Ф = Новый Файл(\"probe\");\n\
             Р = Строка(Ф.Существует());\n\
             Д = ПолучитьДвоичныеДанныеИзСтроки(\"ABC\", КодировкаТекста.UTF8, Ложь);\n\
             Д.Записать(\"probe\");\n\
             Р = Р + \"|\" + Строка(Ф.Существует()) + \"|\" + Строка(Ф.Размер());\n\
             Р = Р + \"|\" + Строка(Ф.ЭтоФайл()) + \"|\" + Строка(Ф.ЭтоКаталог());\n\
             Д = ПолучитьДвоичныеДанныеИзСтроки(\"ABCDE\", КодировкаТекста.UTF8, Ложь);\n\
             Д.Записать(\"probe\"); Р = Р + \"|\" + Строка(Ф.Size());\n\
             УдалитьФайлы(\"probe\");\n\
             Р = Р + \"|\" + Строка(Ф.Существует());\n\
             Попытка Ф.Размер(); Исключение Р = Р + \"|missing\"; КонецПопытки;\n\
             Возврат Р;",
            )
            .unwrap();
        assert_eq!(text(&result), "Нет|Да|3|Да|Нет|5|Нет|missing");
        assert!(files.0.borrow().is_empty());
    }

    #[test]
    fn file_io_paths_are_normalized_before_the_session_host() {
        let engine = Engine::builder().build().unwrap();
        let files = MemoryFiles::default();
        let mut state = engine.state_builder().files(files.clone()).build();
        let result = state
            .exec(
                r#"
                Д = ПолучитьДвоичныеДанныеИзСтроки("ABC", КодировкаТекста.UTF8, Ложь);
                ПутьНуль = "leaf" + Символ(0) + "tail";
                Д.Записать(ПутьНуль);
                ФайлНуль = Новый Файл(ПутьНуль);
                ПутьТаб = "tab" + Символ(9);
                Д.Записать(ПутьТаб);
                ФайлТаб = Новый Файл(ПутьТаб);
                ПрочитаноНуль = Новый ДвоичныеДанные(ПутьНуль);
                ПрочитаноТаб = Новый ДвоичныеДанные(ПутьТаб);
                Возврат Строка(СтрДлина(ФайлНуль.ПолноеИмя)) + "|"
                    + Строка(ФайлНуль.Существует()) + "|" + Строка(ФайлНуль.Размер()) + "|"
                    + Строка(ФайлТаб.ПолноеИмя = "tab") + "|"
                    + Строка(ФайлТаб.Существует()) + "|" + Строка(ФайлТаб.Размер()) + "|"
                    + Строка(ПрочитаноНуль.Размер()) + "|" + Строка(ПрочитаноТаб.Размер());
            "#,
            )
            .unwrap();
        assert_eq!(text(&result), "9|Да|3|Да|Да|3|3|3");
        assert_eq!(
            *files.0.borrow(),
            HashMap::from([
                ("leaf".to_owned(), b"ABC".to_vec()),
                ("tab".to_owned(), b"ABC".to_vec()),
            ])
        );
    }

    #[test]
    fn file_object_has_the_measured_path_properties_and_english_aliases() {
        let engine = Engine::builder().build().unwrap();
        let mut state = engine.state_builder().files(MemoryFiles::default()).build();
        let result = state.exec(
            "Ф = Новый File(\"каталог/имя.tar.gz\");\n\
             Возврат Ф.Name + \"|\" + Ф.BaseName + \"|\" + Ф.Extension + \"|\" + Ф.Path + \"|\" + Ф.FullName;",
        ).unwrap();
        assert_eq!(
            text(&result),
            "имя.tar.gz|имя.tar|.gz|каталог/|каталог/имя.tar.gz"
        );
        for (path, expression, expected) in [
            ("", "Ф.FullName", ""),
            (".hidden", "Ф.BaseName + \"|\" + Ф.Extension", "|.hidden"),
            ("каталог/", "Ф.Name + \"|\" + Ф.Path", "каталог|"),
            ("каталог/../имя.txt", "Ф.FullName", "имя.txt"),
            (
                "каталог\\имя.txt",
                "Ф.Name + \"|\" + Ф.Path",
                "имя.txt|каталог/",
            ),
        ] {
            let script = format!("Ф = Новый Файл(\"{path}\"); Возврат {expression};");
            assert_eq!(text(&state.exec(&script).unwrap()), expected);
        }
        for property in ["Имя", "ИмяБезРасширения", "Расширение", "Путь", "ПолноеИмя"]
        {
            let script = format!(
                "Ф = Новый Файл(\"x\"); Попытка Ф.{property} = \"y\"; Исключение Возврат \"readonly\"; КонецПопытки; Возврат \"writable\";"
            );
            assert_eq!(text(&state.exec(&script).unwrap()), "readonly");
        }
    }

    #[test]
    fn file_constructor_accepts_omitted_and_formatted_arguments() {
        let engine = Engine::builder().build().unwrap();
        let files = MemoryFiles::default();
        let mut state = engine.state_builder().files(files.clone()).build();
        for (constructor, expected) in [
            ("Новый Файл()", ""),
            ("New File()", ""),
            ("Новый Файл(Неопределено)", ""),
            ("Новый File(Null)", ""),
            ("Новый Файл(123)", "123"),
            ("Новый Файл(\"a/../../b\")", "b"),
            ("Новый Файл(\"///\")", "///"),
        ] {
            let result = state
                .exec(&format!("Ф = {constructor}; Возврат Ф.FullName;"))
                .unwrap();
            assert_eq!(text(&result), expected);
        }
        assert!(files.0.borrow().is_empty());
    }

    #[test]
    fn temporary_directory_comes_from_the_session_host() {
        let engine = Engine::builder().build().unwrap();
        let mut state = engine.state_builder().files(MemoryFiles::default()).build();
        for name in ["КаталогВременныхФайлов", "TempFilesDir"] {
            assert_eq!(
                text(&state.exec(&format!("Возврат {name}();")).unwrap()),
                "виртуальный-temp/"
            );
            assert!(engine.compile(&format!("Возврат {name}(1);")).is_err());
        }
    }

    #[test]
    fn temporary_name_extensions_are_normalized_before_the_host_call() {
        let engine = Engine::builder().build().unwrap();
        let files = MemoryFiles::default();
        let mut state = engine.state_builder().files(files.clone()).build();
        for (argument, suffix) in [
            ("", ".tmp"),
            ("Неопределено", ".tmp"),
            ("\"\"", ""),
            ("\"xml\"", ".xml"),
            ("\".xml\"", ".xml"),
            ("\"..tar.gz\"", "..tar.gz"),
            ("\"a/b\"", ".a/b"),
            ("\"a\\b\"", ".a\\b"),
        ] {
            for name in ["ПолучитьИмяВременногоФайла", "GetTempFileName"]
            {
                let value = state.exec(&format!("Возврат {name}({argument});")).unwrap();
                let result = text(&value);
                assert!(result.starts_with("виртуальный-"));
                let tail = result.strip_prefix("виртуальный-").unwrap();
                assert_eq!(&tail[2..], suffix, "{name}({argument})");
            }
        }
        assert!(files.0.borrow().is_empty());
        assert!(state.exec("Возврат GetTempFileName(17);").is_err());
    }

    #[test]
    fn create_directory_uses_the_session_host_and_remains_a_procedure() {
        let engine = Engine::builder().build().unwrap();
        let path = std::env::temp_dir().join(format!(
            "open-bsl-virtual-directory-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let path_bsl = path.to_string_lossy().replace('"', "\"\"");
        let mut state = engine.state_builder().files(MemoryFiles::default()).build();
        for name in ["СоздатьКаталог", "CreateDirectory"] {
            state.exec(&format!("{name}(\"{path_bsl}\");")).unwrap();
            assert!(
                !path.exists(),
                "виртуальная ФС не должна создавать каталог ОС"
            );
            assert!(engine.compile(&format!("Возврат {name}(\"x\");")).is_err());
        }
    }

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

        fn set_modified(&self, path: &str, seconds: i64) -> std::io::Result<()> {
            if !self.0.borrow().contains_key(path) {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "нет файла",
                ));
            }
            self.1.borrow_mut().insert(path.into(), seconds);
            Ok(())
        }

        fn set_read_only(&self, path: &str, value: bool) -> std::io::Result<()> {
            if !self.0.borrow().contains_key(path) {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "нет файла",
                ));
            }
            self.2.borrow_mut().insert(path.into(), value);
            Ok(())
        }

        fn set_hidden(&self, path: &str, value: bool) -> std::io::Result<()> {
            if !self.0.borrow().contains_key(path) {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "нет файла",
                ));
            }
            self.3.borrow_mut().insert(path.into(), value);
            Ok(())
        }

        // Размер известен из содержимого виртуального файла; дескриптор
        // потока для запроса метаданных не нужен.
        fn metadata(&self, path: &str) -> std::io::Result<open_bsl::FileMetadata> {
            self.0
                .borrow()
                .get(path)
                .map(|data| {
                    let metadata = open_bsl::FileMetadata::file(self.1.borrow().get(path).copied())
                        .with_size(data.len() as u64);
                    let metadata = match self.2.borrow().get(path) {
                        Some(value) => metadata.with_read_only(*value),
                        None => metadata,
                    };
                    match self.3.borrow().get(path) {
                        Some(value) => metadata.with_hidden(*value),
                        None => metadata,
                    }
                })
                .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "нет файла"))
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

        fn temporary_directory(&self) -> std::io::Result<String> {
            Ok("виртуальный-temp".to_owned())
        }

        fn temporary_path(&self, suffix: &str, entropy: &[u8; 16]) -> std::io::Result<String> {
            Ok(format!("виртуальный-{:02X}{suffix}", entropy[0]))
        }

        fn path_separator(&self) -> std::io::Result<String> {
            Ok("/".to_string())
        }

        fn remove_path(&self, path: &str) -> std::io::Result<()> {
            let prefix = format!("{path}/");
            self.0
                .borrow_mut()
                .retain(|candidate, _| candidate != path && !candidate.starts_with(&prefix));
            Ok(())
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

    /// `Новый ДвоичныеДанные(путь)` получает файловую систему через
    /// обычную границу конструктора компонента.
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

    /// Временный путь, `ДвоичныеДанные.Записать` и удаление остаются внутри
    /// файловой системы сессии.
    #[test]
    fn temporary_binary_file_stays_in_the_session_file_system() {
        let engine = Engine::builder().build().unwrap();
        let module = engine
            .compile(
                "п = ПолучитьИмяВременногоФайла(\".bin\");\n\
                 существовал = Истина;\n\
                 Попытка\n\
                     до = Новый ДвоичныеДанные(п);\n\
                 Исключение\n\
                     существовал = Ложь;\n\
                 КонецПопытки;\n\
                 данные = ПолучитьДвоичныеДанныеИзСтроки(\"A\", КодировкаТекста.UTF8, Ложь);\n\
                 данные.Записать(п);\n\
                 данные = ПолучитьДвоичныеДанныеИзСтроки(\"BC\", КодировкаТекста.UTF8, Ложь);\n\
                 данные.Записать(п);\n\
                 итог = ПолучитьHexСтрокуИзДвоичныхДанных(Новый ДвоичныеДанные(п));\n\
                 УдалитьФайлы(п);\n\
                 УдалитьФайлы(п);\n\
                 Возврат Строка(Прав(п, 4) = \".bin\") + \"|\"\n\
                     + ПолучитьРазделительПути() + \"|\" + итог + \"|\"\n\
                     + Строка(существовал);",
            )
            .unwrap();

        let files = MemoryFiles::default();
        let value = engine
            .state_builder()
            .files(files.clone())
            .build()
            .run(&module)
            .unwrap();

        assert_eq!(text(&value), "Да|/|4243|Нет");
        assert!(files.0.borrow().is_empty());
    }

    /// `ЗаписьJSON.ОткрытьФайл`/`ЧтениеJSON.ОткрытьФайл` идут через файловую
    /// систему СЕССИИ (ABI-G), а не мимо неё на диск: объект берёт ФС при
    /// построении и держит её, потому что сама запись происходит в
    /// `Закрыть()`.
    #[test]
    fn json_open_file_round_trips_through_the_sessions_file_system() {
        let engine = Engine::builder().build().unwrap();
        let disk = MemoryFiles::default();
        let mut state = engine.state_builder().files(disk.clone()).build();

        state
            .exec(
                "З = Новый ЗаписьJSON;\n\
                 З.ОткрытьФайл(\"данные.json\");\n\
                 ЗаписатьJSON(З, Новый Структура(\"а\", 1));\n\
                 З.Закрыть();",
            )
            .unwrap();

        // Документ лёг в файловую систему сессии, а не на настоящий диск.
        assert!(
            disk.0.borrow().contains_key("данные.json"),
            "ЗаписьJSON должна писать в ФС сессии"
        );
        assert!(!std::path::Path::new("данные.json").exists());

        // И читается он оттуда же — через ту же ФС сессии.
        let read = state
            .exec(
                "Ч = Новый ЧтениеJSON;\n\
                 Ч.ОткрытьФайл(\"данные.json\");\n\
                 Стр = ПрочитатьJSON(Ч);\n\
                 Возврат Формат(Стр.а, \"ЧГ=0\");",
            )
            .unwrap();
        assert_eq!(text(&read), "1");

        state
            .exec(
                "З = Новый ЗаписьJSON;\n\
                 З.ОткрытьФайл(\"край.json \" );\n\
                 ЗаписатьJSON(З, Новый Структура(\"а\", 2));\n\
                 З.Закрыть();",
            )
            .unwrap();
        assert!(disk.0.borrow().contains_key("край.json"));
        assert!(!disk.0.borrow().contains_key("край.json "));
        let read = state
            .exec(
                "Ч = Новый ЧтениеJSON;\n\
                 Ч.ОткрытьФайл(\"край.json\" + Символ(0) + \"ignored\");\n\
                 Стр = ПрочитатьJSON(Ч);\n\
                 Возврат Формат(Стр.а, \"ЧГ=0\");",
            )
            .unwrap();
        assert_eq!(text(&read), "2");
    }

    /// Ошибка файловой системы — ловимое `Попыткой` исключение, а не
    /// паника, и в ОБЕ стороны: чтение отсутствующего файла и отказ
    /// записи. Реализация возвращает `io::Error`, рантайм переводит его в
    /// `RtError`, как и раньше у `std::fs`.
    #[test]
    fn a_file_system_failure_is_a_catchable_error_both_ways() {
        /// Система, у которой нет ни одного файла и запись всегда
        /// отказывает: у отказа записи своя ветка перевода ошибки.
        #[derive(Debug)]
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
            "п = КаталогВременныхФайлов();",
            "СоздатьКаталог(\"куда-нибудь\");",
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

    /// Заданная файловая система работает тремя путями: две встроенные
    /// функции и конструктор `ДвоичныеДанные` через `CreateObject`.
    #[test]
    fn the_file_system_is_used_by_all_file_operations() {
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
        let disk = MemoryFiles::default();
        disk.0
            .borrow_mut()
            .insert("двоичное".to_string(), vec![7; 5]);
        let mut state = engine.state_builder().files(disk.clone()).build();
        assert_eq!(text(&state.exec(script).unwrap()), "1600/5;1600/5;1600/5;");
        // Файл лёг в заданную систему, а не на диск.
        assert!(disk.0.borrow().contains_key("виток.txt"));
        assert!(!std::path::Path::new("виток.txt").exists());
    }

    /// Отказ файловой системы при ЗАКРЫТИИ `ЗаписьТекста` — ловимая ошибка
    /// (ABI-G, сквозная проверка канала через `StateBuilder::files`).
    /// Дескриптор пришёл из файловой системы СЕССИИ
    /// на построении (`NewTextWriter`, интерпретаторный путь), а неудачное
    /// `Закрыть()` ловится `Попыткой`, как любая ошибка рантайма.
    #[test]
    fn a_failing_close_from_the_session_file_system_is_catchable() {
        use std::io::{self, Read, Seek, SeekFrom, Write};

        fn denied(what: &str) -> io::Error {
            io::Error::new(io::ErrorKind::PermissionDenied, what.to_string())
        }

        #[derive(Debug)]
        struct FailingHandle;
        impl Read for FailingHandle {
            fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
                Ok(0)
            }
        }
        impl Seek for FailingHandle {
            fn seek(&mut self, _pos: SeekFrom) -> io::Result<u64> {
                Ok(0)
            }
        }
        impl Write for FailingHandle {
            fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
                Err(denied("запись запрещена"))
            }
            fn flush(&mut self) -> io::Result<()> {
                Err(denied("сброс запрещён"))
            }
        }
        impl open_bsl::FileHandle for FailingHandle {
            fn len(&self) -> io::Result<u64> {
                Ok(0)
            }
            fn close(&mut self) -> io::Result<()> {
                Err(denied("закрытие запрещено"))
            }
        }

        #[derive(Debug)]
        struct FailOnClose;
        impl open_bsl::FileSystem for FailOnClose {
            fn read(&self, path: &str) -> io::Result<Vec<u8>> {
                Err(io::Error::new(io::ErrorKind::NotFound, path.to_string()))
            }
            fn write(&self, _path: &str, _data: &[u8]) -> io::Result<()> {
                Err(denied("запись запрещена"))
            }
            fn metadata(&self, path: &str) -> io::Result<open_bsl::FileMetadata> {
                Err(io::Error::new(io::ErrorKind::NotFound, path.to_string()))
            }
            fn read_dir<'fs>(
                &'fs self,
                path: &str,
            ) -> io::Result<Box<dyn Iterator<Item = io::Result<open_bsl::DirEntry>> + 'fs>>
            {
                Err(io::Error::new(io::ErrorKind::NotFound, path.to_string()))
            }
            fn create_dir_all(&self, _path: &str) -> io::Result<()> {
                Ok(())
            }
            fn open(
                &self,
                _path: &str,
                _options: open_bsl::FileOpenOptions,
            ) -> io::Result<Box<dyn open_bsl::FileHandle>> {
                Ok(Box::new(FailingHandle))
            }
        }

        let engine = Engine::builder().build().unwrap();
        let script = "Попытка\n\
                      \tзп = Новый ЗаписьТекста(\"вых.txt\");\n\
                      \tзп.Записать(\"x\");\n\
                      \tзп.Закрыть();\n\
                      \tВозврат \"не поймано\";\n\
                      Исключение\n\
                      \tВозврат \"поймано\";\n\
                      КонецПопытки;";
        let mut state = engine.state_builder().files(FailOnClose).build();
        assert_eq!(text(&state.exec(script).unwrap()), "поймано");
    }
}
