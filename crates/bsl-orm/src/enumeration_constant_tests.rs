use super::*;

#[test]
fn configuration_reference_materializes_the_existing_enumeration_carrier() {
    let identity = "40ae3f99-8ef0-4819-8dff-e6701aab4cf4";
    let catalog = Arc::new(
        MetadataCatalog::new(
            "enum-constant",
            vec![MetadataObject::new(
                "enum",
                "EnumLegacyProbe",
                MetadataKind::Enumeration,
            )],
        )
        .unwrap()
        .with_enumeration_values(vec![ConfigurationEnumValue::new("enum", identity, "First")])
        .unwrap(),
    );
    for (uuid, member) in [
        (open_bsl::uuid::parse(identity).unwrap(), identity),
        ([0; 16], ""),
    ] {
        let expected = enumeration_reference_value(Arc::clone(&catalog), "enum", member).unwrap();
        let actual = configuration_reference_value(Arc::clone(&catalog), "enum", uuid).unwrap();
        assert_eq!(actual, expected);
        assert_eq!(
            actual.object_ref().unwrap().value_type(),
            expected.object_ref().unwrap().value_type()
        );
    }
}
