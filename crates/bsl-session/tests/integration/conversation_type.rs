use open_bsl::{Engine, Value};

#[test]
fn conversation_type_lookup_does_not_require_a_service_or_construct_a_value() {
    let engine = Engine::builder()
        .register_library(bsl_session::library())
        .build()
        .unwrap();
    let mut state = engine.state_builder().build();
    assert_eq!(
        state.exec("Возврат Тип(\"ИдентификаторОбсужденияСистемыВзаимодействия\") = Тип(\"CollaborationSystemConversationID\");").unwrap(),
        Value::Boolean(true)
    );
    assert_eq!(
        state
            .exec("Возврат Строка(Тип(\"ИдентификаторОбсужденияСистемыВзаимодействия\"));")
            .unwrap(),
        Value::Str("ИдентификаторОбсужденияСистемыВзаимодействия".into())
    );
    assert_eq!(
        state
            .exec("Возврат ТипЗнч(Неопределено) = Тип(\"CollaborationSystemConversationID\");")
            .unwrap(),
        Value::Boolean(false)
    );
    assert_eq!(
        state.exec("Попытка ИдентификаторПробы = Новый ИдентификаторОбсужденияСистемыВзаимодействия(\"x\"); Исключение Возврат Истина; КонецПопытки; Возврат Ложь;").unwrap(),
        Value::Boolean(true)
    );
    let error = state
        .exec("Возврат Новый CollaborationSystemConversationID(\"x\");")
        .unwrap_err();
    assert!(
        error.to_string().contains(
            "идентификатор обсуждения системы взаимодействия не поддерживается этим хостом"
        )
    );
}
