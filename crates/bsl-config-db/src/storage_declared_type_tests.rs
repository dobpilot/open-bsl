use super::*;

#[test]
fn native_storage_declaration_is_identified_by_its_platform_type() {
    let descriptors = open_sdbl::metadata::parse_config_descriptors(
        "571e1bd9-0f64-4a05-a276-dba4fe242ad0",
        include_bytes!("../tests/fixtures/document-owned-attributes/event.deflate"),
    )
    .unwrap();
    let types = descriptors
        .iter()
        .find(|descriptor| descriptor.name == "КартинкиHTML")
        .unwrap()
        .declared_types
        .as_ref()
        .unwrap();
    assert_eq!(types.len(), 1);
    assert!(is_value_storage_declared_type(&types[0]));
    let mut wrong_tag = types[0].clone();
    wrong_tag.tag = "S".into();
    assert!(!is_value_storage_declared_type(&wrong_tag));
    let mut unrelated_type = types[0].clone();
    unrelated_type.reference = Some("51e7a0d2-530b-11d4-b98a-008048da3034".parse().unwrap());
    assert!(!is_value_storage_declared_type(&unrelated_type));
    let mut unknown_type = types[0].clone();
    unknown_type.reference = None;
    assert!(!is_value_storage_declared_type(&unknown_type));
}
