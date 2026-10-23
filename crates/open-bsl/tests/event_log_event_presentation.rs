use open_bsl::{Engine, Value};

#[test]
fn event_log_presentation_has_the_measured_arity_and_fails_explicitly() {
    let engine = Engine::builder().build().unwrap();
    assert!(
        engine
            .compile("Возврат ПредставлениеСобытияЖурналаРегистрации();")
            .is_err()
    );
    assert!(
        engine
            .compile("Возврат ПредставлениеСобытияЖурналаРегистрации(1, 2);")
            .is_err()
    );
    assert_eq!(
        engine
            .new_state()
            .exec(
                r#"
Попытка
    Р = EventLogEventPresentation("_$_Session");
Исключение
    Возврат Истина;
КонецПопытки;
Возврат Ложь;
"#,
            )
            .unwrap(),
        Value::Boolean(true)
    );
}
