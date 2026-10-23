use super::*;
use open_bsl::{Engine, Error, State};

fn state(id: &str) -> State {
    Engine::builder()
        .register_library(library())
        .build()
        .unwrap()
        .state_builder()
        .host_data(Rc::new(SessionData::new(
            Arc::new(MetadataCatalog::new("user-uuid", vec![]).unwrap()),
            Principal::new(id, "Проба", vec![]),
        )))
        .build()
}

#[test]
fn both_names_expose_the_acquired_uuid_type_and_value() {
    let mut state = state("a191bb4f-2a12-4108-a036-fe7ed0287ef7");
    assert_eq!(state.exec(r#"
        ПользовательПробы = ПользователиИнформационнойБазы.ТекущийПользователь();
        Возврат ПользовательПробы.УникальныйИдентификатор = Новый УникальныйИдентификатор("a191bb4f-2a12-4108-a036-fe7ed0287ef7")
            И ПользовательПробы.UUID = ПользовательПробы.УникальныйИдентификатор
            И ТипЗнч(ПользовательПробы.UUID) = Тип("УникальныйИдентификатор");
    "#).unwrap(),Value::Boolean(true));
}

#[test]
fn current_user_uuid_is_read_only() {
    for name in ["УникальныйИдентификатор", "UUID"] {
        let error=state("a191bb4f-2a12-4108-a036-fe7ed0287ef7").exec(&format!("ПользовательПробы = ПользователиИнформационнойБазы.ТекущийПользователь(); ПользовательПробы.{name} = Новый УникальныйИдентификатор;")).unwrap_err();
        assert!(
            matches!(error, Error::Runtime(RtError::PropertyReadOnly { .. })),
            "{error}"
        );
    }
}

#[test]
fn invalid_host_identifier_is_not_replaced_with_a_generated_uuid() {
    assert!(
        state("invalid-host-id")
            .exec("Возврат ПользователиИнформационнойБазы.ТекущийПользователь().UUID;")
            .is_err()
    );
}

#[test]
fn different_states_keep_their_own_principal_identifiers() {
    for id in [
        "a191bb4f-2a12-4108-a036-fe7ed0287ef7",
        "11111111-2222-3333-4444-555555555555",
    ] {
        assert_eq!(state(id).exec(&format!("Возврат ПользователиИнформационнойБазы.ТекущийПользователь().UUID = Новый УникальныйИдентификатор(\"{id}\");")).unwrap(),Value::Boolean(true));
    }
}
