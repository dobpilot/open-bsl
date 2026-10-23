use bsl_orm::{MetadataCatalog, MetadataKind, MetadataObject};

#[test]
fn explicit_reference_presentation_preserves_identity_names_and_existing_catalog() {
    let owner = MetadataObject::new("task-id", "ЗамерЗадачи", MetadataKind::Task);
    let original = MetadataCatalog::new("configuration", vec![owner.clone()]).unwrap();
    let presented = MetadataCatalog::new(
        "configuration",
        vec![owner.with_reference_type_presentation("Замер задачи")],
    )
    .unwrap();
    let before = original.value_types().next().unwrap();
    let after = presented.value_types().next().unwrap();
    assert_eq!(before.display(), "ЗамерЗадачи");
    assert_eq!(after.display(), "Замер задачи");
    assert_eq!(before.package(), after.package());
    assert_eq!(before.identity(), after.identity());
    assert_eq!(before.names(), after.names());
    assert_eq!(
        after.names(),
        ["ЗадачаСсылка.ЗамерЗадачи", "TaskRef.ЗамерЗадачи"]
    );
}
