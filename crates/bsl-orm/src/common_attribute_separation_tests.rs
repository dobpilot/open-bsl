use super::*;

fn catalog() -> MetadataCatalog {
    MetadataCatalog::new(
        "separation",
        vec![
            MetadataObject::new("common", "Разделитель", MetadataKind::CommonAttribute)
                .with_common_attribute_data_separation(true),
            MetadataObject::new("value", "Значение", MetadataKind::SessionParameter),
            MetadataObject::new("other", "НеПараметр", MetadataKind::Catalog),
        ],
    )
    .unwrap()
}

#[test]
fn separation_bindings_validate_owner_and_parameter_targets() {
    for (owner, target) in [
        ("missing", "value"),
        ("other", "value"),
        ("common", "missing"),
        ("common", "other"),
    ] {
        assert!(matches!(
            catalog().with_common_attribute_separation_parameters(
                owner,
                CommonAttributeSeparationParameters {
                    value_parameter: Some(target.into()),
                    use_parameter: None,
                    value_types: None,
                }
            ),
            Err(CatalogError::InvalidCommonAttributeSeparation(_))
        ));
    }
}

#[test]
fn unknown_and_known_empty_types_remain_distinct() {
    for types in [None, Some(vec![])] {
        let catalog = catalog()
            .with_common_attribute_separation_parameters(
                "common",
                CommonAttributeSeparationParameters {
                    value_parameter: Some("value".into()),
                    use_parameter: None,
                    value_types: types.clone(),
                },
            )
            .unwrap();
        let settings = catalog
            .object_by_id("common")
            .unwrap()
            .common_attribute_separation_parameters()
            .unwrap();
        assert_eq!(settings.value_types, types);
        assert_eq!(settings.use_parameter, None);
    }
}
