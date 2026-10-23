use super::*;

#[test]
fn document_result_keeps_its_declared_reference_type_and_native_uuid() {
    use open_sdbl::metadata::{DbNames, SchemaStorage, resolve_metadata};

    let owner: Guid = "f77edfd7-d0d4-532b-a649-399fb412f0a4".parse().unwrap();
    let uuid: Guid = "12345678-1234-4321-abcd-0123456789ab".parse().unwrap();
    let catalog = std::sync::Arc::new(
        bsl_orm::MetadataCatalog::new(
            "document-reference",
            vec![MetadataObject::new(
                owner.as_str(),
                "ЗамерДокумента",
                MetadataKind::Document,
            )],
        )
        .unwrap(),
    );
    let metadata = resolve_metadata(
        DbNames::from_entries(vec![]),
        vec![],
        SchemaStorage {
            tables: vec![],
            anomalies: vec![],
        },
        vec![],
    )
    .snapshot;
    let value = reference_from_cell(
        Cell::Bytes(uuid.to_1c_bytes().to_vec()),
        &ColumnKind::Reference {
            targets: vec![ObjectId::from_bytes(owner.to_bytes())],
            runtime_typed: false,
        },
        &metadata,
        std::sync::Arc::clone(&catalog),
    )
    .unwrap();
    assert_eq!(
        value,
        bsl_orm::configuration_reference_value(catalog, owner.as_str(), uuid.to_bytes()).unwrap()
    );
}

#[test]
fn uuid_parameters_and_results_keep_canonical_identity() {
    for id in [
        "00112233-4455-6677-8899-aabbccddeeff",
        "00000000-0000-0000-0000-000000000000",
    ] {
        let bytes = open_bsl::uuid::parse(id).unwrap();
        let value = Value::Object(Rc::new(BslObject::Uuid(bytes)));
        assert_eq!(
            parameter_value(&value).unwrap(),
            ParameterValue::Uuid(id.parse().unwrap())
        );
        assert_eq!(
            value_from_cell(Cell::Uuid(bytes), &ColumnKind::Uuid).unwrap(),
            value
        );
    }
}

#[test]
fn mismatched_cells_are_not_guessed_as_uuid() {
    assert!(value_from_cell(Cell::Bytes(vec![0; 16]), &ColumnKind::Uuid).is_err());
    assert!(
        value_from_cell(
            Cell::Uuid([0; 16]),
            &ColumnKind::Binary { length: Some(16) }
        )
        .is_err()
    );
    assert!(
        value_from_cell(
            Cell::Text("00112233-4455-6677-8899-aabbccddeeff".into()),
            &ColumnKind::Uuid
        )
        .is_err()
    );
}
