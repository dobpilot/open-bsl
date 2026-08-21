//! Окружение запуска принадлежит сессии, а не процессу.
//!
//! До этого аргументы жили в `OnceLock`, часы читались прямым
//! `SystemTime::now()`, а байты идентификатора — из `thread_local`. Две
//! изолированные `State` одного `Engine` поэтому делили окружение, и
//! проверить поведение на заданном времени было нечем.

use open_bsl::{Clock, Engine, RandomSource, Value};

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
