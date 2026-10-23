use super::*;

#[test]
fn business_process_fields_keep_order_and_section_ownership() {
    let catalog = MetadataCatalog::new(
        "cfg",
        vec![MetadataObject::new(
            "process",
            "Процесс",
            MetadataKind::BusinessProcess,
        )],
    )
    .unwrap()
    .with_business_process_metadata(
        "process",
        None,
        Some(vec![
            DocumentAttribute::new("b", "Второй", vec![]),
            DocumentAttribute::new("a", "Первый", vec![]),
        ]),
        Some(vec![
            DocumentTabularSection::new("performers", "Исполнители").with_attributes(vec![
                DocumentAttribute::new("person", "Исполнитель", vec![]),
            ]),
            DocumentTabularSection::new("subjects", "Предметы").with_attributes(vec![]),
        ]),
    )
    .unwrap();
    let object = catalog.object_by_id("process").unwrap();
    assert_eq!(
        object
            .business_process_attributes()
            .unwrap()
            .iter()
            .map(DocumentAttribute::id)
            .collect::<Vec<_>>(),
        ["b", "a"]
    );
    let sections = object.business_process_tabular_sections().unwrap();
    assert_eq!(
        sections
            .iter()
            .map(DocumentTabularSection::id)
            .collect::<Vec<_>>(),
        ["performers", "subjects"]
    );
    assert_eq!(sections[0].attributes().unwrap()[0].id(), "person");
    assert!(sections[1].attributes().unwrap().is_empty());
    assert!(object.document_attributes().is_none());
    assert!(object.document_tabular_sections().is_none());
}

#[test]
fn attaching_business_metadata_preserves_unknown_empty_and_original_catalog() {
    let original = MetadataCatalog::new(
        "cfg",
        vec![
            MetadataObject::new("process", "Процесс", MetadataKind::BusinessProcess),
            MetadataObject::new("catalog", "Каталог", MetadataKind::Catalog),
        ],
    )
    .unwrap();
    let original_object = original.object_by_id("process").unwrap().clone();
    let updated = original
        .with_business_process_metadata(
            "process",
            Some(BusinessProcessNumberSettings {
                number_type: DocumentNumberType::String,
                length: 11,
                variable_length: true,
            }),
            Some(vec![]),
            None,
        )
        .unwrap();
    let object = updated.object_by_id("process").unwrap();
    assert_eq!(object.business_process_attributes().unwrap().len(), 0);
    assert!(object.business_process_tabular_sections().is_none());
    assert_eq!(
        object.business_process_number_settings().unwrap().length,
        11
    );
    assert!(original_object.business_process_attributes().is_none());
    assert!(
        updated
            .object_by_id("catalog")
            .unwrap()
            .business_process_number_settings()
            .is_none()
    );
    for owner in ["missing", "catalog"] {
        assert!(matches!(
            MetadataCatalog::new(
                "cfg",
                vec![MetadataObject::new(
                    "catalog",
                    "Каталог",
                    MetadataKind::Catalog
                )]
            )
            .unwrap()
            .with_business_process_metadata(owner, None, None, None),
            Err(CatalogError::UnknownBusinessProcessOwner(_))
        ));
    }
}

#[test]
fn business_object_aliases_differ_from_references_and_other_configurations() {
    let make = |identity| {
        MetadataCatalog::new(
            identity,
            vec![MetadataObject::new(
                "process",
                "Процесс",
                MetadataKind::BusinessProcess,
            )],
        )
        .unwrap()
    };
    let first = make("first");
    let second = make("second");
    let first_type = first.business_process_object_types().next().unwrap();
    let second_type = second.business_process_object_types().next().unwrap();
    assert_ne!(
        TypeRef::Value(Arc::clone(first_type)),
        TypeRef::Value(Arc::clone(second_type))
    );
    let mut builder = open_bsl::Engine::builder();
    for ty in first
        .value_types()
        .chain(first.business_process_object_types())
    {
        builder = builder.register_value_type(Arc::clone(ty));
    }
    let engine = builder.build().unwrap();
    let value = engine.new_state().exec(r#"Возврат Тип("БизнесПроцессОбъект.Процесс") = Тип("businessprocessobject.Процесс") И Тип("BusinessProcessObject.Процесс") <> Тип("БизнесПроцессСсылка.Процесс");"#).unwrap();
    assert!(matches!(value, Value::Boolean(true)));
}

#[test]
fn business_checking_policy_preserves_unknown_empty_order_and_original_description() {
    let catalog = MetadataCatalog::new(
        "cfg",
        vec![MetadataObject::new(
            "process",
            "Процесс",
            MetadataKind::BusinessProcess,
        )],
    )
    .unwrap();
    let original = catalog.object_by_id("process").unwrap().clone();
    assert!(original.business_process_standard_fill_checking().is_none());
    let empty = catalog
        .with_business_process_standard_fill_checking("process", vec![])
        .unwrap();
    assert_eq!(
        empty
            .object_by_id("process")
            .unwrap()
            .business_process_standard_fill_checking(),
        Some([].as_slice())
    );
    let ordered = empty
        .with_business_process_standard_fill_checking(
            "process",
            vec![
                CatalogStandardAttributeFillChecking {
                    name: "Дата".into(),
                    fill_checking: FillChecking::ShowError,
                },
                CatalogStandardAttributeFillChecking {
                    name: "Номер".into(),
                    fill_checking: FillChecking::DontCheck,
                },
            ],
        )
        .unwrap();
    let object = ordered.object_by_id("process").unwrap();
    let settings = object.business_process_standard_fill_checking().unwrap();
    assert_eq!(
        settings
            .iter()
            .map(|setting| setting.name.as_str())
            .collect::<Vec<_>>(),
        ["Дата", "Номер"]
    );
    assert_eq!(settings[0].fill_checking, FillChecking::ShowError);
    assert_eq!(settings[1].fill_checking, FillChecking::DontCheck);
    assert!(object.catalog_standard_fill_checking().is_none());
    assert!(original.business_process_standard_fill_checking().is_none());
}

#[test]
fn business_checking_policy_rejects_missing_and_non_business_owners() {
    for owner in ["missing", "catalog"] {
        let catalog = MetadataCatalog::new(
            "cfg",
            vec![MetadataObject::new(
                "catalog",
                "Справочник",
                MetadataKind::Catalog,
            )],
        )
        .unwrap();
        assert!(
            matches!(catalog.with_business_process_standard_fill_checking(owner, vec![]), Err(CatalogError::UnknownBusinessProcessOwner(name)) if name == owner)
        );
    }
}

#[test]
fn head_task_binding_is_explicit_and_validates_both_kinds() {
    let make_catalog = || {
        MetadataCatalog::new(
            "cfg",
            vec![
                MetadataObject::new("process", "Процесс", MetadataKind::BusinessProcess),
                MetadataObject::new("task", "Задача", MetadataKind::Task),
                MetadataObject::new("catalog", "Каталог", MetadataKind::Catalog),
            ],
        )
        .unwrap()
    };
    let catalog = make_catalog();
    let original_object = catalog.object_by_id("process").unwrap().clone();
    assert!(
        catalog
            .object_by_id("process")
            .unwrap()
            .business_process_task()
            .is_none()
    );
    let updated = catalog
        .with_business_process_task("process", "task")
        .unwrap();
    assert_eq!(
        updated
            .object_by_id("process")
            .unwrap()
            .business_process_task(),
        Some("task")
    );
    assert!(original_object.business_process_task().is_none());
    for (owner, task) in [
        ("task", "task"),
        ("process", "missing"),
        ("process", "catalog"),
    ] {
        assert!(
            make_catalog()
                .with_business_process_task(owner, task)
                .is_err()
        );
    }
}
