use open_bsl::{Engine, Value};

#[test]
fn overflowing_gmt_names_fail_as_catchable_errors() {
    let engine = Engine::builder().build().unwrap();
    for function in ["СмещениеСтандартногоВремени", "DaylightTimeOffset"]
    {
        for zone in ["GMT+-2147483648", "GMT--2147483648", "GMT+-1000000"] {
            let source = format!(
                "Попытка\n Возврат {function}(\"{zone}\", Дата(2024, 1, 1));\n Исключение\n Возврат Истина;\n КонецПопытки;"
            );
            assert_eq!(
                engine.new_state().exec(&source).unwrap(),
                Value::Boolean(true),
                "{function}: {zone}"
            );
        }
    }
}
