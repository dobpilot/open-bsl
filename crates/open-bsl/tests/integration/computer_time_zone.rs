use open_bsl::{Engine, FixedTimeZone, Value};

#[test]
fn computer_zone_is_independent_of_the_session_zone() {
    let engine = Engine::builder().build().unwrap();
    assert!(engine.compile("Возврат ЧасовойПояс(1);").is_err());

    let computer = engine.new_state().exec("Возврат ЧасовойПояс();").unwrap();
    let mut state = engine.state_builder().zone(FixedTimeZone::UTC).build();
    assert_eq!(state.exec("Возврат TimeZone();").unwrap(), computer);
    assert_eq!(
        state.exec("Возврат ЧасовойПоясСеанса();").unwrap(),
        Value::Str("GMT+0".into())
    );
    assert_eq!(
        state
            .exec("Возврат ТипЗнч(ЧасовойПояс()) = Тип(\"Строка\");")
            .unwrap(),
        Value::Boolean(true)
    );
}
