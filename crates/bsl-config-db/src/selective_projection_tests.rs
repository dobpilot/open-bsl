use super::*;
use bsl_orm::{ConfigurationEnumValue, MetadataObject};
use open_sdbl::metadata::{DbNames, SchemaStorage, parse_config_descriptors, resolve_metadata};

#[test]
fn selected_projection_preserves_the_native_enumeration_filling_graph() {
    let event = "571e1bd9-0f64-4a05-a276-dba4fe242ad0";
    let enumeration = "19430845-e6b2-4a2b-944a-75142d4914c0";
    let member = "e2942d8d-433f-497b-8b7e-c999fbc88b44";
    let mut descriptors = parse_config_descriptors(
        event,
        include_bytes!("../tests/fixtures/document-owned-attributes/event.deflate"),
    )
    .unwrap();
    descriptors.extend(
        parse_config_descriptors(
            enumeration,
            include_bytes!(
                "../tests/fixtures/document-owned-attributes/importance-enumeration.deflate"
            ),
        )
        .unwrap(),
    );
    let metadata = resolve_metadata(
        DbNames::from_entries(vec![]),
        descriptors,
        SchemaStorage {
            tables: vec![],
            anomalies: vec![],
        },
        vec![],
    )
    .snapshot;
    let catalog = || {
        MetadataCatalog::new(
            "native-filling",
            vec![
                MetadataObject::new(event, "Событие", MetadataKind::Document),
                MetadataObject::new(enumeration, "ВариантыВажности", MetadataKind::Enumeration),
                MetadataObject::new("unused", "Лишний", MetadataKind::Catalog),
            ],
        )
        .unwrap()
        .with_enumeration_values(vec![ConfigurationEnumValue::new(
            enumeration,
            member,
            "Обычная",
        )])
        .unwrap()
    };
    let full = Arc::new(project(catalog(), &metadata).unwrap());
    let selected = project_selected(catalog(), &metadata).unwrap();
    let mut builder = Engine::builder().register_library(bsl_orm::library());
    for ty in full.transferable_value_types() {
        builder = builder.register_value_type(ty);
    }
    let engine = builder.build().unwrap();
    let mut shapes = RuntimeShapes::seeded(vec![], vec![], Some(engine.registry()));
    let values = [&*full, &selected].map(|catalog| {
        catalog
            .object_by_id(event)
            .unwrap()
            .document_attributes()
            .unwrap()
            .iter()
            .find(|attribute| attribute.name() == "Важность")
            .unwrap()
            .fill_value()
            .expect("нативное значение заполнения должно сохраниться")
            .materialize(&mut shapes)
            .unwrap()
    });
    assert_eq!(values[0], values[1]);
    assert_ne!(values[0], vec![open_bsl::Value::Undefined]);
}
