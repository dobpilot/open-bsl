use open_bsl::{Engine, Value};

#[test]
fn deliverable_notification_resolves_without_inventing_a_value() {
    let engine = Engine::builder().build().unwrap();
    engine
        .compile(
            "Процедура НеВызывается()\nУ = Новый ДоставляемоеУведомление;\nКонецПроцедуры\nВозврат 1;",
        )
        .unwrap();
    engine
        .compile("У = Новый DeliverableNotification();")
        .unwrap();
    assert!(
        engine
            .compile("У = Новый ДоставляемоеУведомление(1);")
            .is_err()
    );
    assert_eq!(
        engine
            .new_state()
            .exec(
                "Попытка\nНовый ДоставляемоеУведомление();\nИсключение\nВозврат Истина;\nКонецПопытки;\nВозврат Ложь;",
            )
            .unwrap(),
        Value::Boolean(true)
    );
}
