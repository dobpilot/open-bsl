use super::*;

#[test]
fn unknown_and_known_empty_stay_distinct() {
    assert!(snapshot(&[]).is_some());
    let principal = attach(Principal::new("id", "name", vec![]), None);
    assert_eq!(principal.name(), "name");
}

#[test]
fn string_bindings_survive_materialization() {
    let graph = snapshot(&[("НаименованиеЯзык1".into(), "own-separator-value".into())]).unwrap();
    let mut shapes = RuntimeShapes::seeded(vec![], vec![], None);
    let roots = graph.materialize(&mut shapes).unwrap();
    let value = roots[0]
        .get_field(shapes.names.intern("НаименованиеЯзык1"))
        .unwrap();
    assert_eq!(value, Value::Str("own-separator-value".into()));
}

#[test]
fn duplicate_folded_names_never_silently_replace_a_binding() {
    assert!(snapshot(&[("Имя".into(), "one".into()), ("имя".into(), "two".into())]).is_none());
}

fn read_count(bindings: Option<&[(String, String)]>) -> Result<Value, open_bsl::Error> {
    let engine = open_bsl::Engine::builder()
        .register_library(bsl_orm::library())
        .build()
        .unwrap();
    let catalog = std::sync::Arc::new(
        bsl_orm::MetadataCatalog::new("user-separation-adapter", vec![]).unwrap(),
    );
    let principal = attach(Principal::new("id", "name", vec![]), bindings);
    let mut state = engine
        .state_builder()
        .host_data(std::rc::Rc::new(bsl_orm::SessionData::new(
            catalog, principal,
        )))
        .build();
    state.exec("Возврат ПользователиИнформационнойБазы.ТекущийПользователь().РазделениеДанных.Количество();")
}

#[test]
fn projected_principal_exposes_known_bindings_and_rejects_unknown() {
    assert!(read_count(None).is_err());
    assert_eq!(
        read_count(Some(&[])).unwrap(),
        Value::Number(open_bsl::BslNumber::from_i64(0))
    );
    assert_eq!(
        read_count(Some(&[(
            "НаименованиеЯзык1".into(),
            "own-separator-value".into()
        )]))
        .unwrap(),
        Value::Number(open_bsl::BslNumber::from_i64(1))
    );
}
