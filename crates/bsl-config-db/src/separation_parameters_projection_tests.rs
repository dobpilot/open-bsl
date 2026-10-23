use super::*;
use bsl_orm::{MetadataCatalog, MetadataKind, MetadataObject};

#[test]
fn native_separator_bindings_and_types_reach_the_runtime_catalog() {
    let resources = resources();
    let descriptors = include_str!("../tests/fixtures/common-attributes/demo-common-ids.txt")
        .lines()
        .flat_map(|id| {
            let resource = resources
                .iter()
                .find(|resource| resource.file_name == id)
                .unwrap();
            parse_config_descriptors(id, &resource.compressed).unwrap()
        })
        .collect();
    let metadata = resolve_metadata(
        open_sdbl::metadata::parse_db_names(include_bytes!(
            "../tests/fixtures/common-attributes/demo-db-names-1009.deflate"
        ))
        .unwrap(),
        descriptors,
        SchemaStorage {
            tables: vec![],
            anomalies: vec![],
        },
        vec![],
    )
    .snapshot;
    let separator = metadata.separators().next().unwrap();
    let descriptor = metadata
        .descriptors()
        .iter()
        .find(|descriptor| descriptor.object_guid == separator.guid)
        .unwrap();
    let settings = descriptor.separation.as_ref().unwrap();
    let value_id = settings.value_parameter.as_ref().unwrap().to_string();
    let use_id = settings.use_parameter.as_ref().unwrap().to_string();
    let catalog = MetadataCatalog::new(
        "bindings",
        vec![
            MetadataObject::new(
                separator.guid.to_string(),
                "Разделитель",
                MetadataKind::CommonAttribute,
            )
            .with_common_attribute_data_separation(true),
            MetadataObject::new(&value_id, "Значение", MetadataKind::SessionParameter),
            MetadataObject::new(&use_id, "Использование", MetadataKind::SessionParameter),
        ],
    )
    .unwrap();
    let catalog = crate::document_attributes::project(catalog, &metadata).unwrap();
    let projected = catalog
        .object_by_id(separator.guid.as_str())
        .unwrap()
        .common_attribute_separation_parameters()
        .unwrap();
    assert_eq!(
        projected.value_parameter.as_deref(),
        Some(value_id.as_str())
    );
    assert_eq!(projected.use_parameter.as_deref(), Some(use_id.as_str()));
    let types = projected.value_types.as_ref().unwrap();
    assert_eq!(types.len(), 1);
    assert_eq!(types[0].id, TypeRef::Native(TypeId::Number));
}

#[test]
fn native_complete_content_preserves_order_auto_and_constant_identity() {
    use bsl_orm::{CommonAttributeCondition, MetadataCatalog, MetadataKind, MetadataObject};
    let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/common-content-condition");
    let descriptors = std::fs::read_to_string(base.join("resources.txt"))
        .unwrap()
        .lines()
        .flat_map(|id| {
            parse_config_descriptors(
                id,
                &std::fs::read(base.join(format!("{id}.deflate"))).unwrap(),
            )
            .unwrap()
        })
        .collect();
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
    let owner = "03f9d2d0-eae4-4098-a70e-4eb569b11fd9";
    let acquired = metadata
        .descriptors()
        .iter()
        .find(|descriptor| descriptor.object_guid.as_str() == owner)
        .unwrap()
        .common_attribute_content
        .as_ref()
        .unwrap();
    assert_eq!(acquired.len(), 3);
    let objects = || {
        let mut objects = vec![MetadataObject::new(
            owner,
            "Разделитель",
            MetadataKind::CommonAttribute,
        )];
        objects.extend(acquired.iter().enumerate().map(|(index, entry)| {
            MetadataObject::new(
                entry.object.to_string(),
                format!("Участник{index}"),
                if index == 1 {
                    MetadataKind::Constant
                } else {
                    MetadataKind::Catalog
                },
            )
        }));
        objects
    };
    let catalog = crate::document_attributes::project(
        MetadataCatalog::new("native", objects()).unwrap(),
        &metadata,
    )
    .unwrap();
    let projected = catalog
        .object_by_id(owner)
        .unwrap()
        .common_attribute_content()
        .unwrap();
    assert_eq!(
        projected
            .iter()
            .map(|entry| entry.metadata_id.as_str())
            .collect::<Vec<_>>(),
        acquired
            .iter()
            .map(|entry| entry.object.as_str())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        projected[0].usage,
        open_bsl::EnumValue::CommonAttributeUseAuto
    );
    assert_eq!(
        projected[2].conditional_separation,
        Some(CommonAttributeCondition::Constant(
            acquired[1].object.to_string()
        ))
    );
    let mut missing = objects();
    missing.pop();
    let catalog = crate::document_attributes::project(
        MetadataCatalog::new("incomplete", missing).unwrap(),
        &metadata,
    )
    .unwrap();
    assert!(
        catalog
            .object_by_id(owner)
            .unwrap()
            .common_attribute_content()
            .is_none()
    );
}
