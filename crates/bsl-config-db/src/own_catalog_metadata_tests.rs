use super::*;
use bsl_orm::{
    CatalogCodeType, CatalogHierarchyType, CatalogStandardAttributes, MetadataKind, MetadataObject,
};

const OWN: &str = "413c18db-d586-5dd5-8f97-51b903e903a3";
const NATIVE: &[u8] = include_bytes!(
    "../../../docs/research/fixtures/catalog-own-extension-db-1017/cfe-outer/0/413c18db-d586-5dd5-8f97-51b903e903a3"
);

#[test]
fn acquired_own_catalog_fields_reach_the_object_without_losing_its_source_policy() {
    let raw = [resource(OWN, NATIVE)];
    let resources = Resources::new(&raw).unwrap();
    let object = MetadataObject::new(OWN, "CP_ЗамерКода", MetadataKind::Catalog)
        .with_own_extension_object_module("Собственный", true);
    let object = crate::project_own_catalog_metadata(object, &resources).unwrap();
    assert_eq!(
        object.catalog_standard_attributes(),
        Some(CatalogStandardAttributes {
            code_type: CatalogCodeType::String,
            code_length: 9,
            description_length: 25,
        })
    );
    assert_eq!(object.catalog_hierarchical(), Some(true));
    assert_eq!(
        object.catalog_hierarchy_type(),
        Some(CatalogHierarchyType::FoldersAndItems)
    );
    assert_eq!(object.catalog_owners(), Some([].as_slice()));
    assert_eq!(
        object.catalog_attribute_names().unwrap(),
        ["ИскомоеЗначение"]
    );
    assert_eq!(object.catalog_tabular_section_names(), Some([].as_slice()));
    assert_eq!(object.object_module(), Some("Собственный"));
    assert_eq!(object.own_extension_object_module_safe_mode(), Some(true));
}

#[test]
fn unknown_sdk_catalog_fields_do_not_become_default_settings() {
    let descriptors = open_sdbl::metadata::parse_config_descriptors(OWN, NATIVE).unwrap();
    let mut owner = descriptors
        .iter()
        .find(|d| d.object_guid.as_str() == OWN)
        .unwrap()
        .clone();
    owner.catalog_standard_attributes = None;
    owner.catalog_hierarchical = None;
    owner.catalog_hierarchy_type = None;
    owner.catalog_owners = None;
    owner.catalog_attributes = None;
    owner.catalog_tabular_sections = None;
    let object = crate::project_catalog_descriptor_fields(
        MetadataObject::new(OWN, "CP_ЗамерКода", MetadataKind::Catalog),
        &owner,
        &descriptors,
    )
    .unwrap();
    assert_eq!(object.catalog_standard_attributes(), None);
    assert_eq!(object.catalog_hierarchical(), None);
    assert_eq!(object.catalog_hierarchy_type(), None);
    assert_eq!(object.catalog_owners(), None);
    assert_eq!(object.catalog_attribute_names(), None);
    assert_eq!(object.catalog_tabular_section_names(), None);
}

#[test]
fn own_catalog_projection_rejects_corruption_and_does_not_read_other_kinds() {
    let raw = [resource(OWN, b"not a deflate resource")];
    let resources = Resources::new(&raw).unwrap();
    assert!(
        crate::project_own_catalog_metadata(
            MetadataObject::new(OWN, "Повреждённый", MetadataKind::Catalog),
            &resources,
        )
        .is_err()
    );
    let object = crate::project_own_catalog_metadata(
        MetadataObject::new(OWN, "ДругойВид", MetadataKind::Document),
        &resources,
    )
    .unwrap();
    assert_eq!(object.kind(), MetadataKind::Document);
    assert_eq!(object.name(), "ДругойВид");
}
