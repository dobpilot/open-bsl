use open_bsl::{Engine, FixedTimeZone, Value};

#[test]
fn daylight_offset_aliases_and_input_errors_match_native() {
    let engine = Engine::builder().build().unwrap();
    let mut state = engine.state_builder().zone(FixedTimeZone::UTC).build();
    for name in ["СмещениеЛетнегоВремени", "DaylightTimeOffset"] {
        for args in [
            "",
            "\"GMT+3\"",
            "\"GMT-05:30\", Дата(2024, 1, 15)",
            "Неопределено, Неопределено",
        ] {
            assert_eq!(
                state.exec(&format!("Возврат {name}({args});")).unwrap(),
                Value::number_from_i64(0)
            );
        }
        for args in [
            "\"Invalid/Zone\"",
            "1",
            "\"GMT+3\", 1",
            "\"GMT+3\", Новый Массив",
            "\"GMT+3\", \"not-a-date\"",
            "Неопределено, Неопределено, Неопределено",
        ] {
            assert!(
                state.exec(&format!("Возврат {name}({args});")).is_err(),
                "{name}({args})"
            );
        }
    }
}

#[cfg(target_os = "linux")]
#[test]
fn daylight_offset_reuses_system_regional_rules_matching_native_dates() {
    let engine = Engine::builder().build().unwrap();
    let mut state = engine.new_state();
    for name in ["СмещениеЛетнегоВремени", "DaylightTimeOffset"] {
        for zone in ["America/New_York", "Europe/Berlin"] {
            for (date, expected) in [
                ("Дата(2024, 1, 15, 12)", 0),
                ("Дата(2024, 7, 15, 12)", 3600),
                ("\"20240715120000\"", 3600),
            ] {
                assert_eq!(
                    state
                        .exec(&format!("Возврат {name}(\"{zone}\", {date});"))
                        .unwrap(),
                    Value::number_from_i64(expected)
                );
            }
        }
    }
}
