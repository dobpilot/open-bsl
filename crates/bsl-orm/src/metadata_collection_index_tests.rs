use super::*;

fn identity(collection: &MetadataCollection, index: i64) -> Arc<str> {
    let value = collection
        .get_index(&Value::number_from_i64(index))
        .unwrap();
    Arc::clone(
        &value
            .object_ref()
            .unwrap()
            .downcast_ref::<MetadataValue>()
            .unwrap()
            .object_id,
    )
}

#[test]
fn indexed_collection_keeps_catalog_order_between_unrelated_kinds() {
    let mut objects = (0..1000)
        .map(|index| {
            MetadataObject::new(
                format!("doc{index}"),
                format!("Документ{index}"),
                MetadataKind::Document,
            )
        })
        .collect::<Vec<_>>();
    objects.insert(7, MetadataObject::new("b", "Второй", MetadataKind::Catalog));
    objects.push(MetadataObject::new("a", "Первый", MetadataKind::Catalog));
    let collection = MetadataCollection {
        catalog: Arc::new(MetadataCatalog::new("index", objects).unwrap()),
        kind: MetadataKind::Catalog,
        subsystem_parent: None,
    };
    assert_eq!(collection.collection_len().unwrap(), 2);
    assert_eq!(identity(&collection, 0).as_ref(), "b");
    assert_eq!(identity(&collection, 1).as_ref(), "a");
    assert_eq!(collection.find("пЕРВЫЙ").unwrap().id(), "a");
    assert!(matches!(
        collection.get_index(&Value::number_from_i64(2)),
        Err(RtError::IndexOutOfBounds { index: 2, len: 2 })
    ));
    assert!(matches!(
        collection.get_index(&Value::number_from_i64(-1)),
        Err(RtError::BadIndex)
    ));
    assert!(matches!(
        collection.get_index(&Value::Boolean(false)),
        Err(RtError::BadIndex)
    ));
}

#[test]
fn indexed_subsystem_collections_keep_parent_scope_and_empty_children() {
    let catalog = Arc::new(
        MetadataCatalog::new(
            "subsystems",
            vec![
                MetadataObject::new("root", "Корень", MetadataKind::Subsystem),
                MetadataObject::new("child", "Дочерняя", MetadataKind::Subsystem)
                    .with_subsystem_parent("root"),
                MetadataObject::new("other", "Другой", MetadataKind::Subsystem),
            ],
        )
        .unwrap(),
    );
    for (parent, expected) in [
        (None, vec!["root", "other"]),
        (Some("root"), vec!["child"]),
        (Some("child"), vec![]),
    ] {
        let collection = MetadataCollection {
            catalog: Arc::clone(&catalog),
            kind: MetadataKind::Subsystem,
            subsystem_parent: parent.map(Arc::from),
        };
        assert_eq!(collection.collection_len().unwrap(), expected.len());
        for (index, id) in expected.iter().enumerate() {
            assert_eq!(identity(&collection, index as i64).as_ref(), *id);
        }
    }
}

#[test]
fn indexed_common_attributes_keep_explicit_order_and_replacement() {
    let catalog = MetadataCatalog::new(
        "common",
        vec![
            MetadataObject::new("one", "Первый", MetadataKind::CommonAttribute),
            MetadataObject::new("two", "Второй", MetadataKind::CommonAttribute),
        ],
    )
    .unwrap();
    let collection = |catalog| MetadataCollection {
        catalog: Arc::new(catalog),
        kind: MetadataKind::CommonAttribute,
        subsystem_parent: None,
    };
    assert_eq!(collection(catalog).collection_len().unwrap(), 0);
    let catalog = MetadataCatalog::new(
        "common",
        vec![
            MetadataObject::new("one", "Первый", MetadataKind::CommonAttribute),
            MetadataObject::new("two", "Второй", MetadataKind::CommonAttribute),
        ],
    )
    .unwrap()
    .with_common_attribute_ids(vec![
        "two".into(),
        "missing".into(),
        "one".into(),
        "two".into(),
    ]);
    let collection = collection(catalog);
    assert_eq!(collection.collection_len().unwrap(), 3);
    assert_eq!(identity(&collection, 0).as_ref(), "two");
    assert_eq!(identity(&collection, 1).as_ref(), "one");
    assert_eq!(identity(&collection, 2).as_ref(), "two");
    let catalog = Arc::try_unwrap(collection.catalog)
        .unwrap()
        .with_common_attribute_ids(vec!["one".into()]);
    let collection = MetadataCollection {
        catalog: Arc::new(catalog),
        kind: MetadataKind::CommonAttribute,
        subsystem_parent: None,
    };
    assert_eq!(collection.collection_len().unwrap(), 1);
    assert_eq!(identity(&collection, 0).as_ref(), "one");
}
