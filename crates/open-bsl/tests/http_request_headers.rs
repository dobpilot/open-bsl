#![cfg(feature = "http")]

use open_bsl::{Engine, Value};

#[test]
fn http_request_headers_accept_only_a_map_and_no_third_argument() {
    let engine = Engine::builder().build().unwrap();
    assert!(
        engine
            .compile("Новый HTTPЗапрос(\"/probe\", Новый Соответствие, 1);")
            .is_err()
    );
    let result = engine
        .new_state()
        .exec(
            "Попытка\nНовый HTTPЗапрос(\"/probe\", 1);\nИсключение\nВозврат Истина;\nКонецПопытки;\nВозврат Ложь;",
        )
        .unwrap();
    assert_eq!(result, Value::Boolean(true));
}
