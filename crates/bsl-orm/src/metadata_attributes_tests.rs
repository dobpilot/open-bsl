use super::*;
use open_bsl::{Engine, State};

fn state(attributes: Option<Vec<DocumentAttribute>>) -> State {
    let mut document = MetadataObject::new("doc", "ДокументПробы", MetadataKind::Document);
    if let Some(attributes) = attributes {
        document = document.with_document_attributes(attributes);
    }
    let catalog =
        Arc::new(MetadataCatalog::new("metadata-attribute-state", vec![document]).unwrap());
    let engine = Engine::builder()
        .register_library(library())
        .build()
        .unwrap();
    engine
        .state_builder()
        .host_data(Rc::new(SessionData::new(
            catalog,
            Principal::new("user", "Тест", vec![]),
        )))
        .build()
}

fn attribute() -> DocumentAttribute {
    DocumentAttribute::new(
        "attribute",
        "РеквизитПробы",
        vec![open_bsl::ColumnType::plain(TypeRef::Native(
            open_bsl::TypeId::Boolean,
        ))],
    )
}

#[test]
fn unknown_collection_does_not_become_an_empty_collection() {
    let source = "Возврат Метаданные.Документы.ДокументПробы.Реквизиты.Количество();";
    assert!(state(None).exec(source).is_err());
    assert_eq!(
        state(Some(vec![])).exec(source).unwrap(),
        Value::number_from_i64(0)
    );
}

#[test]
fn unknown_filling_fields_and_invalid_graph_roots_fail_explicitly() {
    let flag = "Возврат Метаданные.Документы.ДокументПробы.Реквизиты.РеквизитПробы.ЗаполнятьИзДанныхЗаполнения;";
    let value =
        "Возврат Метаданные.Документы.ДокументПробы.Реквизиты.РеквизитПробы.ЗначениеЗаполнения;";
    let mut missing = state(Some(vec![attribute()]));
    assert!(missing.exec(flag).is_err());
    assert!(missing.exec(value).is_err());
    let mut disabled = state(Some(vec![attribute().with_fill_from_filling_value(false)]));
    assert_eq!(disabled.exec(flag).unwrap(), Value::Boolean(false));
    let shapes = open_bsl::RuntimeShapes::seeded(vec![], vec![], None);
    for roots in [vec![], vec![Value::Undefined, Value::Boolean(true)]] {
        let graph =
            SerializedValueGraph::capture(&roots, &shapes, &GraphLimits::default()).unwrap();
        assert!(
            state(Some(vec![attribute().with_fill_value(graph)]))
                .exec(value)
                .is_err()
        );
    }
    let graph =
        SerializedValueGraph::capture(&[Value::Undefined], &shapes, &GraphLimits::default())
            .unwrap();
    assert_eq!(
        state(Some(vec![attribute().with_fill_value(graph)]))
            .exec(value)
            .unwrap(),
        Value::Undefined
    );
}

#[test]
fn attribute_metadata_properties_are_read_only_and_preserve_the_catalog() {
    let mut session = state(Some(vec![attribute().with_fill_from_filling_value(false)]));
    for (property, value) in [
        ("Имя", "\"Изменено\""),
        ("ЗаполнятьИзДанныхЗаполнения", "Истина"),
        ("ЗначениеЗаполнения", "Неопределено"),
    ] {
        assert!(session.exec(&format!("Метаданные.Документы.ДокументПробы.Реквизиты.РеквизитПробы.{property} = {value};")).is_err());
    }
    assert_eq!(
        session
            .exec("Возврат Метаданные.Документы.ДокументПробы.Реквизиты.РеквизитПробы.Имя;")
            .unwrap(),
        Value::Str("РеквизитПробы".into())
    );
    assert_eq!(session.exec("Возврат Метаданные.Документы.ДокументПробы.Реквизиты.РеквизитПробы.ЗаполнятьИзДанныхЗаполнения;").unwrap(), Value::Boolean(false));
}
