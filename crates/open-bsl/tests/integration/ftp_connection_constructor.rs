use open_bsl::{Engine, Value};

#[test]
fn ftp_connection_constructor_resolves_documented_forms_without_transport() {
    let engine = Engine::builder().build().unwrap();
    for source in [
        "Новый FTPСоединение(\"server\")",
        "New FTPConnection(\"server\", 21, \"user\", \"password\")",
        "Новый FTPСоединение(\"server\", 21, \"user\", \"password\", Неопределено, Ложь, 0, Неопределено, Неопределено)",
    ] {
        engine
            .compile(&format!(
                "Процедура НеВызывается()\nСоединение = {source};\nКонецПроцедуры"
            ))
            .unwrap();
    }
    assert_eq!(
        engine
            .new_state()
            .exec(
                "Попытка\nСоединение = Новый FTPСоединение(\"server\");\nИсключение\nВозврат Истина;\nКонецПопытки;\nВозврат Ложь;",
            )
            .unwrap(),
        Value::Boolean(true)
    );
    assert!(engine.compile("Соединение = Новый FTPСоединение;").is_err());
    assert!(
        engine
            .compile("Соединение = Новый FTPСоединение(1, 2, 3, 4, 5, 6, 7, 8, 9, 10);")
            .is_err()
    );
}
