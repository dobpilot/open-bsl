use super::*;
use open_bsl::{Engine, RuntimeShapes, State, value_from_string_internal};

fn snapshot(text: &str) -> SerializedValueGraph {
    let mut shapes = RuntimeShapes::seeded(vec![], vec![], None);
    let value = value_from_string_internal(text, &mut shapes).unwrap();
    SerializedValueGraph::capture(&[value], &shapes, &GraphLimits::default()).unwrap()
}

fn state(principal: Principal) -> State {
    let engine = Engine::builder()
        .register_library(library())
        .build()
        .unwrap();
    engine
        .state_builder()
        .host_data(Rc::new(SessionData::new(
            Arc::new(MetadataCatalog::new("user-separation", vec![]).unwrap()),
            principal,
        )))
        .build()
}

#[test]
fn unknown_user_separation_does_not_become_an_empty_structure() {
    let error = state(Principal::new("user", "Проба", vec![]))
        .exec("Возврат ПользователиИнформационнойБазы.ТекущийПользователь().РазделениеДанных;")
        .unwrap_err();
    assert!(error.to_string().contains("не предоставлено источником"));
}

#[test]
fn known_empty_user_separation_is_a_structure_with_both_property_names() {
    let principal = Principal::new("user", "Проба", vec![]).with_data_separation(snapshot(
        r##"{"#",4238019d-7e49-4fc9-91db-b6b951d5cf8e,{0}}"##,
    ));
    assert_eq!(
        state(principal).exec(
            "ПользовательПробы = ПользователиИнформационнойБазы.ТекущийПользователь(); Возврат ТипЗнч(ПользовательПробы.РазделениеДанных) = Тип(\"Структура\") И ПользовательПробы.DataSeparation.Количество() = 0;"
        ).unwrap(),
        Value::Boolean(true)
    );
}

#[test]
fn user_object_mutation_does_not_change_the_principal_or_another_state() {
    let principal = Principal::new("user", "Проба", vec![]).with_data_separation(snapshot(
        r##"{"#",4238019d-7e49-4fc9-91db-b6b951d5cf8e,{1,{{"S","НаименованиеЯзык1"},{"S","own-separator-value"}}}}"##,
    ));
    let mut first = state(principal.clone());
    let mut second = state(principal);
    assert_eq!(
        first.exec(
            r#"ПользовательПробы = ПользователиИнформационнойБазы.ТекущийПользователь();
            СтруктураПробы = ПользовательПробы.РазделениеДанных;
            СтруктураПробы.Вставить("НаименованиеЯзык1", "changed");
            Возврат ПользовательПробы.DataSeparation.НаименованиеЯзык1 = "changed"
                И ПользователиИнформационнойБазы.ТекущийПользователь().РазделениеДанных.НаименованиеЯзык1 = "own-separator-value";"#,
        ).unwrap(),
        Value::Boolean(true)
    );
    assert_eq!(
        second.exec("Возврат ПользователиИнформационнойБазы.ТекущийПользователь().РазделениеДанных.НаименованиеЯзык1;").unwrap(),
        Value::Str("own-separator-value".into())
    );
}

#[test]
fn invalid_user_snapshot_root_count_and_type_fail_explicitly() {
    let shapes = RuntimeShapes::seeded(vec![], vec![], None);
    for roots in [
        vec![],
        vec![Value::Undefined],
        vec![Value::Undefined, Value::Undefined],
    ] {
        let graph =
            SerializedValueGraph::capture(&roots, &shapes, &GraphLimits::default()).unwrap();
        assert!(
            state(Principal::new("user", "Проба", vec![]).with_data_separation(graph))
                .exec(
                    "Возврат ПользователиИнформационнойБазы.ТекущийПользователь().DataSeparation;"
                )
                .is_err()
        );
    }
}
