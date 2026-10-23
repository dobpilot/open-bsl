use super::{DocumentNumberSettings, DocumentNumberType, MetadataKind, MetadataObject};

#[test]
fn document_register_associations_preserve_unknown_empty_and_owned_order() {
    use super::MetadataCatalog;
    let document = MetadataObject::new("doc", "Документ", MetadataKind::Document);
    assert!(document.document_registers().is_none());
    assert_eq!(
        document
            .clone()
            .with_document_registers(vec![])
            .document_registers(),
        Some(&[][..])
    );
    let ids = vec!["second".to_owned(), "first".to_owned()];
    assert_eq!(
        document
            .clone()
            .with_document_registers(ids.clone())
            .document_registers(),
        Some(ids.as_slice())
    );
    let catalog = MetadataCatalog::new(
        "config",
        vec![
            document,
            MetadataObject::new("first", "Первый", MetadataKind::InformationRegister),
            MetadataObject::new("second", "Второй", MetadataKind::InformationRegister),
        ],
    )
    .unwrap();
    let types = catalog.information_record_set_types().collect::<Vec<_>>();
    assert_eq!(types.len(), 2);
    assert_ne!(
        open_bsl::TypeRef::Value(std::sync::Arc::clone(types[0])),
        open_bsl::TypeRef::Value(std::sync::Arc::clone(types[1]))
    );
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<MetadataObject>();
}

#[test]
fn document_section_collections_preserve_unknown_and_empty_states() {
    use super::{DocumentTabularSection, MetadataCatalog};
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<DocumentTabularSection>();
    let object = MetadataObject::new("doc", "Документ", MetadataKind::Document);
    assert!(object.document_tabular_sections().is_none());
    assert_eq!(
        object
            .clone()
            .with_document_tabular_sections(vec![])
            .document_tabular_sections(),
        Some(&[][..])
    );
    let unknown = DocumentTabularSection::new("section", "Запасы");
    assert_eq!(unknown.id(), "section");
    assert_eq!(unknown.name(), "Запасы");
    assert!(unknown.attributes().is_none());
    let empty = unknown.clone().with_attributes(vec![]);
    assert_eq!(empty.attributes(), Some(&[][..]));
    let catalog = MetadataCatalog::new("config", vec![object.clone()]).unwrap();
    assert!(
        catalog
            .with_document_tabular_sections("missing", vec![])
            .is_err()
    );
    let known = MetadataCatalog::new("config", vec![object])
        .unwrap()
        .with_document_tabular_sections("doc", vec![unknown.clone(), empty.clone()])
        .unwrap();
    assert_eq!(
        known.objects()[0].document_tabular_sections(),
        Some(&[unknown, empty][..])
    );
}

#[test]
fn unknown_document_numbering_is_not_disabled_numbering() {
    let unknown = MetadataObject::new("doc", "Документ", MetadataKind::Document);
    assert_eq!(unknown.document_number_settings(), None);
    for number_type in [DocumentNumberType::String, DocumentNumberType::Number] {
        for number_length in [0, 9] {
            let settings = DocumentNumberSettings {
                number_type,
                number_length,
            };
            let known = unknown.clone().with_document_number_settings(settings);
            assert_eq!(known.document_number_settings(), Some(settings));
            assert_eq!(unknown.document_number_settings(), None);
        }
    }
}

#[test]
fn owned_attributes_and_filling_defaults_keep_unknown_states_and_sendable_snapshots() {
    use super::DocumentAttribute;
    use open_bsl::{
        ColumnType, GraphLimits, RuntimeShapes, SerializedValueGraph, TypeId, TypeRef, Value,
    };
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<DocumentAttribute>();
    assert_send_sync::<MetadataObject>();
    let unknown = MetadataObject::new("doc", "Документ", MetadataKind::Document);
    assert!(unknown.document_attributes().is_none());
    assert_eq!(
        unknown
            .clone()
            .with_document_attributes(vec![])
            .document_attributes(),
        Some(&[][..])
    );
    let types = vec![ColumnType {
        id: TypeRef::Native(TypeId::String),
        quals: vec!["12".into()],
    }];
    let attribute = DocumentAttribute::new("attribute", "Заполняемый", types.clone());
    assert_eq!(attribute.fill_from_filling_value(), None);
    assert!(attribute.fill_value().is_none());
    let shapes = RuntimeShapes::seeded(vec![], vec![], None);
    let graph = SerializedValueGraph::capture(
        &[Value::Str("default".into())],
        &shapes,
        &GraphLimits::default(),
    )
    .unwrap();
    let known = attribute
        .with_fill_from_filling_value(false)
        .with_fill_value(graph);
    assert_eq!(known.id(), "attribute");
    assert_eq!(known.name(), "Заполняемый");
    assert_eq!(known.types(), types);
    assert_eq!(known.fill_from_filling_value(), Some(false));
    let values = known
        .fill_value()
        .unwrap()
        .materialize(&mut RuntimeShapes::seeded(vec![], vec![], None))
        .unwrap();
    assert_eq!(values, vec![Value::Str("default".into())]);
    let object = unknown.with_document_attributes(vec![known.clone()]);
    assert_eq!(object.document_attributes(), Some(&[known][..]));
}

#[test]
fn document_checking_policies_preserve_unknown_empty_and_known_values() {
    use super::{CatalogStandardAttributeFillChecking, DocumentTabularSection, FillChecking};
    let metadata = MetadataObject::new("doc", "Проба", MetadataKind::Document);
    assert!(metadata.document_standard_fill_checking().is_none());
    let empty = metadata
        .clone()
        .with_document_standard_fill_checking(vec![]);
    assert_eq!(empty.document_standard_fill_checking(), Some(&[][..]));
    let policy = vec![CatalogStandardAttributeFillChecking {
        name: "Date".into(),
        fill_checking: FillChecking::ShowError,
    }];
    let known = metadata.with_document_standard_fill_checking(policy.clone());
    assert_eq!(
        known.document_standard_fill_checking(),
        Some(policy.as_slice())
    );
    let section = DocumentTabularSection::new("section", "Строки");
    assert_eq!(section.fill_checking(), None);
    assert_eq!(
        section
            .clone()
            .with_fill_checking(FillChecking::DontCheck)
            .fill_checking(),
        Some(FillChecking::DontCheck)
    );
    assert_eq!(
        section
            .with_fill_checking(FillChecking::ShowError)
            .fill_checking(),
        Some(FillChecking::ShowError)
    );
}
