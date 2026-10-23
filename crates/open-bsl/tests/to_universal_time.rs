use open_bsl::{Engine, FixedTimeZone, Value};

#[test]
fn local_to_universal_time_uses_each_states_zone() {
    let engine = Engine::builder().build().unwrap();
    let source = "Возврат УниверсальноеВремя(Дата(2024, 1, 2, 12, 0, 0));";
    let east = engine
        .state_builder()
        .zone(FixedTimeZone::new(3 * 3600).unwrap())
        .build()
        .exec(source)
        .unwrap();
    let utc = engine
        .state_builder()
        .zone(FixedTimeZone::UTC)
        .build()
        .exec(source)
        .unwrap();
    let expected_east = engine
        .new_state()
        .exec("Возврат Дата(2024, 1, 2, 9, 0, 0);")
        .unwrap();
    let expected_utc = engine
        .new_state()
        .exec("Возврат Дата(2024, 1, 2, 12, 0, 0);")
        .unwrap();
    assert_eq!(east, expected_east);
    assert_eq!(utc, expected_utc);
    assert_ne!(east, utc);
    assert!(matches!(east, Value::Date(_)));
}

#[test]
fn time_zone_conversion_preserves_fractional_ticks_through_the_facade() {
    let engine = Engine::builder()
        .common_module(
            "Время",
            "Функция Перевести(Местная) Экспорт Возврат УниверсальноеВремя(Местная); КонецФункции",
        )
        .configuration_roots(["Время"])
        .build()
        .unwrap();
    let mut state = engine
        .state_builder()
        .zone(FixedTimeZone::new(3 * 3600).unwrap())
        .build();
    let Value::Date(base) = state.exec("Возврат Дата(2024, 1, 2, 12, 0, 0);").unwrap()
    else {
        panic!("ожидалась дата")
    };
    let local = open_bsl::BslDate::from_ticks(base.ticks() + 123).unwrap();
    let (utc, _) = state
        .call_configuration_handler(
            "Время",
            "Перевести",
            vec![Value::Date(local)],
            open_bsl::ConfigurationHandlerEntryMode::Inherit,
        )
        .unwrap();
    let Value::Date(utc) = utc else {
        panic!("ожидалась дата UTC")
    };
    assert_eq!(utc, local.shift_seconds(-3 * 3600).unwrap());
    assert_eq!(utc.ticks() % 10_000, local.ticks() % 10_000);
}
