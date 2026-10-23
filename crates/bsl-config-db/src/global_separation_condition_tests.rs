use bsl_orm::{CommonAttributeCondition, MetadataCatalog, MetadataKind, MetadataObject};
use open_sdbl::metadata::{DbNames, SchemaStorage, parse_config_descriptors, resolve_metadata};

const OWNER: &str = "03f9d2d0-eae4-4098-a70e-4eb569b11fd9";
const CONDITION: &str = "9893e2d6-f3f8-4d73-bb06-19bf26d216ab";

fn descriptors(absent: bool) -> Vec<open_sdbl::metadata::ConfigDescriptor> {
    let attribute = if absent {
        include_bytes!("../tests/fixtures/global-separation-condition/no-condition.deflate")
            .as_slice()
    } else {
        include_bytes!("../tests/fixtures/global-separation-condition/03f9d2d0-eae4-4098-a70e-4eb569b11fd9.deflate").as_slice()
    };
    let mut descriptors = parse_config_descriptors(OWNER, attribute).unwrap();
    descriptors.extend(parse_config_descriptors(CONDITION, include_bytes!("../tests/fixtures/global-separation-condition/9893e2d6-f3f8-4d73-bb06-19bf26d216ab.deflate")).unwrap());
    descriptors
}

fn project(
    descriptors: Vec<open_sdbl::metadata::ConfigDescriptor>,
    kind: MetadataKind,
) -> MetadataCatalog {
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
    let catalog = MetadataCatalog::new(
        "native",
        vec![
            MetadataObject::new(OWNER, "Разделитель", MetadataKind::CommonAttribute),
            MetadataObject::new(CONDITION, "ЗначениеОпцииПробы", kind),
        ],
    )
    .unwrap();
    crate::document_attributes::project(catalog, &metadata).unwrap()
}

#[test]
fn native_global_condition_projects_boolean_identity_and_explicit_absence() {
    let catalog = project(descriptors(false), MetadataKind::Constant);
    assert_eq!(
        catalog
            .object_by_id(OWNER)
            .unwrap()
            .common_attribute_condition(),
        Some(&CommonAttributeCondition::Constant(CONDITION.into()))
    );
    let catalog = project(descriptors(true), MetadataKind::Constant);
    assert_eq!(
        catalog
            .object_by_id(OWNER)
            .unwrap()
            .common_attribute_condition(),
        Some(&CommonAttributeCondition::Undefined)
    );
}

#[test]
fn unknown_or_wrong_condition_targets_do_not_invent_boolean_metadata() {
    let mut missing = descriptors(false);
    missing.retain(|d| d.object_guid.as_str() != CONDITION);
    let mut unknown_type = descriptors(false);
    unknown_type
        .iter_mut()
        .find(|d| d.object_guid.as_str() == CONDITION)
        .unwrap()
        .declared_types = None;
    let mut wrong_type = descriptors(false);
    wrong_type
        .iter_mut()
        .find(|d| d.object_guid.as_str() == CONDITION)
        .unwrap()
        .declared_types
        .as_mut()
        .unwrap()[0]
        .tag = "S".into();
    for catalog in [
        project(missing, MetadataKind::Constant),
        project(wrong_type, MetadataKind::Constant),
        project(unknown_type, MetadataKind::Constant),
        project(descriptors(false), MetadataKind::Catalog),
    ] {
        assert!(
            catalog
                .object_by_id(OWNER)
                .unwrap()
                .common_attribute_condition()
                .is_none()
        );
    }
}
