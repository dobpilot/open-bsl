use open_bsl::{Engine, Value};

#[test]
fn internet_mail_constructor_resolves_and_fails_inside_try() {
    let engine = Engine::builder().build().unwrap();
    engine.compile("Значение = Новый ИнтернетПочта();").unwrap();
    engine.compile("Значение = New InternetMail();").unwrap();
    assert!(
        engine
            .compile("Значение = Новый ИнтернетПочта(1);")
            .is_err()
    );

    let result = engine
        .new_state()
        .exec(
            "Попытка\nНовый ИнтернетПочта();\nИсключение\nВозврат Истина;\nКонецПопытки;\nВозврат Ложь;",
        )
        .unwrap();
    assert_eq!(result, Value::Boolean(true));
}

#[test]
fn internet_mail_profile_constructor_resolves_and_fails_inside_try() {
    let engine = Engine::builder().build().unwrap();
    engine
        .compile("Значение = Новый ИнтернетПочтовыйПрофиль();")
        .unwrap();
    engine
        .compile("Значение = New InternetMailProfile();")
        .unwrap();
    assert!(
        engine
            .compile("Значение = Новый ИнтернетПочтовыйПрофиль(1);")
            .is_err()
    );

    let result = engine
        .new_state()
        .exec(
            "Попытка\nНовый ИнтернетПочтовыйПрофиль();\nИсключение\nВозврат Истина;\nКонецПопытки;\nВозврат Ложь;",
        )
        .unwrap();
    assert_eq!(result, Value::Boolean(true));
}
