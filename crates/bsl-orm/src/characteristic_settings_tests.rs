use super::{CharacteristicSettings, MetadataKind, MetadataObject};

#[test]
fn acquired_characteristic_settings_distinguish_unknown_from_known_empty_types() {
    let object = MetadataObject::new("probe", "Проба", MetadataKind::ChartOfCharacteristicTypes);
    assert!(object.characteristic_settings().is_none());
    let configured = object
        .clone()
        .with_characteristic_settings(CharacteristicSettings {
            code_length: 9,
            description_length: 25,
            value_types: vec![],
        });
    let settings = configured.characteristic_settings().unwrap();
    assert_eq!((settings.code_length, settings.description_length), (9, 25));
    assert!(settings.value_types.is_empty());
    assert!(object.characteristic_settings().is_none());
    assert_eq!(configured.clone().characteristic_settings(), Some(settings));
}

#[test]
fn characteristic_types_have_their_own_reference_and_object_names() {
    let catalog = super::MetadataCatalog::new(
        "configuration",
        vec![MetadataObject::new(
            "probe",
            "Проба",
            MetadataKind::ChartOfCharacteristicTypes,
        )],
    )
    .unwrap();
    let references: Vec<_> = catalog.value_types().collect();
    assert_eq!(references.len(), 1);
    assert_eq!(
        references[0].identity(),
        "configuration/characteristic-ref/probe"
    );
    assert_eq!(
        references[0].names(),
        &[
            "ПланВидовХарактеристикСсылка.Проба",
            "ChartOfCharacteristicTypesRef.Проба"
        ]
    );
    let objects: Vec<_> = catalog.characteristic_object_types().collect();
    assert_eq!(objects.len(), 1);
    assert_eq!(
        objects[0].identity(),
        "configuration/characteristic-object/probe"
    );
    assert_eq!(
        objects[0].names(),
        &[
            "ПланВидовХарактеристикОбъект.Проба",
            "ChartOfCharacteristicTypesObject.Проба"
        ]
    );
    assert_eq!(catalog.catalog_object_types().count(), 0);
    assert_eq!(catalog.document_object_types().count(), 0);
}

#[test]
fn characteristic_attributes_keep_unknown_and_known_empty_lists_distinct() {
    let object = MetadataObject::new("probe", "Проба", MetadataKind::ChartOfCharacteristicTypes);
    assert!(object.characteristic_attributes().is_none());
    let configured = object.clone().with_characteristic_attributes(vec![]);
    assert_eq!(configured.characteristic_attributes(), Some([].as_slice()));
    assert!(object.characteristic_attributes().is_none());
}
