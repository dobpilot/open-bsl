use bsl_orm::{MetadataCatalog, MetadataKind, MetadataObject, Principal, SessionData};
use open_bsl::{Engine, Value};
use std::{rc::Rc, sync::Arc};

fn state(object: MetadataObject) -> open_bsl::State {
    let catalog = Arc::new(MetadataCatalog::new("processor", vec![object]).unwrap());
    let mut builder = Engine::builder().register_library(bsl_orm::library());
    for ty in catalog.data_processor_object_types() {
        builder = builder.register_value_type(Arc::clone(ty));
    }
    builder
        .build()
        .unwrap()
        .state_builder()
        .host_data(Rc::new(SessionData::new(
            catalog,
            Principal::new("user", "Тест", vec![]),
        )))
        .build()
}

#[test]
fn processor_creation_preserves_unknown_and_verified_empty_collections() {
    let unknown = MetadataObject::new("processor", "Проба", MetadataKind::DataProcessor);
    assert!(unknown.data_processor_attributes().is_none());
    assert!(unknown.data_processor_tabular_sections().is_none());
    let error = state(unknown.clone())
        .exec("Возврат Обработки.Проба.Создать();")
        .unwrap_err();
    assert!(
        error.to_string().contains("реквизиты обработки неизвестны"),
        "{error}"
    );
    let attributes = unknown.with_data_processor_attributes(vec![]);
    assert_eq!(attributes.data_processor_attributes(), Some(&[][..]));
    let error = state(attributes.clone())
        .exec("Возврат Обработки.Проба.Создать();")
        .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("табличные части обработки неизвестны"),
        "{error}"
    );
    let complete = attributes.with_data_processor_tabular_sections(vec![]);
    assert_eq!(complete.data_processor_tabular_sections(), Some(&[][..]));
    let mut state = state(complete);
    assert_eq!(
        state
            .exec("Возврат Обработки.Проба.cReAtE().Метаданные().Имя;")
            .unwrap(),
        Value::Str("Проба".into())
    );
    assert!(state.exec("Обработки.Проба.Создать(1);").is_err());
}
