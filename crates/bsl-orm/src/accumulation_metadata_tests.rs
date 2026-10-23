use super::*;

#[test]
fn ordered_dimensions_and_resources_are_kept_separate() {
    let catalog = MetadataCatalog::new(
        "cfg",
        vec![MetadataObject::new(
            "register",
            "Обороты",
            MetadataKind::AccumulationRegister,
        )],
    )
    .unwrap()
    .with_accumulation_register_metadata(
        "register",
        Some(AccumulationRegisterKind::Turnovers),
        Some(vec![
            DocumentAttribute::new("second", "Второе", vec![]),
            DocumentAttribute::new("first", "Первое", vec![]),
        ]),
        Some(vec![DocumentAttribute::new(
            "resource",
            "Количество",
            vec![],
        )]),
    )
    .unwrap();
    let object = catalog.object_by_id("register").unwrap();
    assert_eq!(
        object.accumulation_register_kind(),
        Some(AccumulationRegisterKind::Turnovers)
    );
    assert_eq!(
        object
            .accumulation_register_dimensions()
            .unwrap()
            .iter()
            .map(DocumentAttribute::name)
            .collect::<Vec<_>>(),
        ["Второе", "Первое"]
    );
    assert_eq!(
        object.accumulation_register_resources().unwrap()[0].name(),
        "Количество"
    );
}

#[test]
fn acquired_empty_and_unknown_collections_do_not_change_an_object_clone() {
    let original = MetadataCatalog::new(
        "cfg",
        vec![MetadataObject::new(
            "register",
            "Остатки",
            MetadataKind::AccumulationRegister,
        )],
    )
    .unwrap();
    let original_object = original.object_by_id("register").unwrap().clone();
    let updated = original
        .with_accumulation_register_metadata(
            "register",
            Some(AccumulationRegisterKind::Balance),
            Some(vec![]),
            None,
        )
        .unwrap();
    let object = updated.object_by_id("register").unwrap();
    assert_eq!(
        object.accumulation_register_kind(),
        Some(AccumulationRegisterKind::Balance)
    );
    assert_eq!(object.accumulation_register_dimensions().unwrap().len(), 0);
    assert!(object.accumulation_register_resources().is_none());
    assert!(original_object.accumulation_register_kind().is_none());
    assert!(original_object.accumulation_register_dimensions().is_none());
}

#[test]
fn register_settings_reject_absent_and_other_kind_owners() {
    for owner in ["catalog", "missing"] {
        let catalog = MetadataCatalog::new(
            "cfg",
            vec![MetadataObject::new(
                "catalog",
                "Каталог",
                MetadataKind::Catalog,
            )],
        )
        .unwrap();
        assert!(matches!(
            catalog.with_accumulation_register_metadata(owner, None, None, None,),
            Err(CatalogError::UnknownAccumulationRegisterOwner(_))
        ));
    }
}
