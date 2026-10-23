use open_bsl::{Engine, Value};

#[test]
fn available_time_zones_resolve_but_do_not_fabricate_a_catalog() {
    let engine = Engine::builder().build().unwrap();
    assert!(
        engine
            .compile("Возврат ПолучитьДопустимыеЧасовыеПояса(1);")
            .is_err()
    );
    for source in [
        "Возврат ПолучитьДопустимыеЧасовыеПояса();",
        "Возврат GetAvailableTimeZones();",
    ] {
        assert!(engine.compile(source).is_ok(), "{source}");
        let result = engine
            .new_state()
            .exec(&format!(
                "Попытка\n{source}\nИсключение\nВозврат Истина;\nКонецПопытки;"
            ))
            .unwrap();
        assert_eq!(result, Value::Boolean(true));
    }
}
