use open_bsl::{Clock, Engine, FixedTimeZone, TimeZone, Value};

struct TestClock(i64);

impl Clock for TestClock {
    fn unix_millis(&mut self) -> i64 {
        self.0
    }
}

struct NamedZone {
    current: i32,
    named: i32,
}

impl TimeZone for NamedZone {
    fn offset_seconds(&self, _: i64) -> i32 {
        self.current + 3600
    }

    fn standard_offset_seconds(&self, _: i64) -> i32 {
        self.current
    }

    fn named_standard_offset_seconds(&self, name: &str, _: i64) -> Option<i32> {
        (name == "Test/Named").then_some(self.named)
    }
}

#[test]
fn standard_time_offset_uses_each_states_zone_and_clock() {
    let engine = Engine::builder().build().unwrap();
    let mut first = engine
        .state_builder()
        .clock(TestClock(1_704_110_400_000))
        .zone(NamedZone {
            current: 3 * 3600,
            named: -5 * 3600,
        })
        .build();
    let mut second = engine
        .state_builder()
        .clock(TestClock(1_704_110_400_000))
        .zone(NamedZone {
            current: 4 * 3600,
            named: 3600,
        })
        .build();

    let script = r#"
Возврат СмещениеСтандартногоВремени()
    + StandardTimeOffset("Test/Named", "20240715120000");
"#;
    assert_eq!(
        first.exec(script).unwrap(),
        Value::number_from_i64(-2 * 3600)
    );
    assert_eq!(
        second.exec(script).unwrap(),
        Value::number_from_i64(5 * 3600)
    );
}

#[test]
fn standard_time_offset_supports_gmt_and_rejects_invalid_inputs() {
    let engine = Engine::builder().build().unwrap();
    let mut state = engine
        .state_builder()
        .clock(TestClock(1_704_110_400_000))
        .zone(FixedTimeZone::UTC)
        .build();

    assert_eq!(
        state
            .exec("Возврат СмещениеСтандартногоВремени(\"GMT-05:30\", Дата(2024, 1, 1));")
            .unwrap(),
        Value::number_from_i64(-5 * 3600 - 30 * 60)
    );
    for source in [
        "Возврат СмещениеСтандартногоВремени(\"Invalid/Zone\");",
        "Возврат СмещениеСтандартногоВремени(1);",
        "Возврат СмещениеСтандартногоВремени(\"GMT+3\", 1);",
        "Возврат СмещениеСтандартногоВремени(\"GMT+3\", \"not-a-date\");",
    ] {
        assert!(state.exec(source).is_err(), "{source}");
    }
}

#[cfg(target_os = "linux")]
#[test]
fn system_time_zone_resolves_iana_standard_offsets() {
    let engine = Engine::builder().build().unwrap();
    let mut state = engine.new_state();
    let value = state
        .exec(
            r#"
Возврат СмещениеСтандартногоВремени("America/New_York", Дата(2024, 7, 15))
    + СмещениеСтандартногоВремени("Europe/Berlin", Дата(2024, 7, 15));
"#,
        )
        .unwrap();
    assert_eq!(value, Value::number_from_i64(-4 * 3600));
}
