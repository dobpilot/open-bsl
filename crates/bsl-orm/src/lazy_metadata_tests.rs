use std::sync::atomic::{AtomicUsize, Ordering};

use super::*;

#[test]
fn indexed_member_validation_keeps_folded_names_scoped_to_the_owner() {
    let catalog = || {
        MetadataCatalog::new(
            "members",
            vec![
                MetadataObject::new("one", "Первое", MetadataKind::Enumeration),
                MetadataObject::new("two", "Второе", MetadataKind::Enumeration),
            ],
        )
        .unwrap()
    };
    let values = vec![
        ConfigurationEnumValue::new("one", "a", "Значение"),
        ConfigurationEnumValue::new("two", "b", "ЗНАЧЕНИЕ"),
    ];
    assert!(catalog().with_enumeration_values(values.clone()).is_ok());
    let mut duplicate = values;
    duplicate.push(ConfigurationEnumValue::new("one", "c", "зНАЧЕНИЕ"));
    assert!(matches!(
        catalog().with_enumeration_values(duplicate),
        Err(CatalogError::DuplicateEnumerationValue { owner, name })
            if owner == "Первое" && name == "зНАЧЕНИЕ"
    ));
}

#[derive(Debug, Default)]
struct Details {
    loads: AtomicUsize,
}

impl MetadataObjectSource for Details {
    fn load_object(&self, id: &str) -> Result<MetadataObject, String> {
        self.loads.fetch_add(1, Ordering::Relaxed);
        if id == "broken" {
            return Err("ошибка адресного чтения".into());
        }
        assert_eq!(id, "target");
        Ok(MetadataObject::new("target", "Цель", MetadataKind::Catalog)
            .with_manager_module("Справочник.Цель.МодульМенеджера"))
    }
}

fn catalog(source: Arc<Details>) -> Arc<MetadataCatalog> {
    Arc::new(
        MetadataCatalog::new(
            "configuration",
            vec![
                MetadataObject::new("target", "Цель", MetadataKind::Catalog),
                MetadataObject::new("unused", "Лишний", MetadataKind::Catalog),
                MetadataObject::new("broken", "Поврежденный", MetadataKind::Catalog),
            ],
        )
        .unwrap()
        .with_object_source(source),
    )
}

#[test]
fn metadata_collection_discovery_does_not_materialize_objects() {
    let source = Arc::new(Details::default());
    let catalog = catalog(source.clone());
    let collection = MetadataCollection {
        catalog,
        kind: MetadataKind::Catalog,
        subsystem_parent: None,
    };
    assert_eq!(
        collection
            .members()
            .map(MetadataObject::name)
            .collect::<Vec<_>>(),
        ["Цель", "Лишний", "Поврежденный"]
    );
    assert_eq!(source.loads.load(Ordering::Relaxed), 0);
}

#[test]
fn metadata_value_materializes_only_its_target_and_shares_immutable_details() {
    let source = Arc::new(Details::default());
    let catalog = catalog(source.clone());
    let value = metadata_object_value(&catalog, catalog.object_by_id("target").unwrap());
    let mut shapes = open_bsl::RuntimeShapes::seeded(Vec::new(), Vec::new(), None);
    let mut context = CallContext::minimal(&mut shapes, open_bsl::format_value);
    assert_eq!(
        value
            .object_ref()
            .unwrap()
            .get_property("Имя", &mut context)
            .unwrap(),
        Value::Str("Цель".into())
    );
    let first = catalog
        .materialized_object_by_id("target")
        .unwrap()
        .unwrap();
    let shared = catalog.clone();
    assert!(std::ptr::eq(
        first,
        shared.materialized_object_by_id("target").unwrap().unwrap()
    ));
    assert_eq!(
        first.manager_module(),
        Some("Справочник.Цель.МодульМенеджера")
    );
    assert_eq!(source.loads.load(Ordering::Relaxed), 1);
}

#[test]
fn metadata_read_failure_is_distinct_from_an_unknown_identity() {
    let source = Arc::new(Details::default());
    let catalog = catalog(source.clone());
    assert!(
        catalog
            .materialized_object_by_id("absent")
            .unwrap()
            .is_none()
    );
    assert!(
        matches!(catalog.materialized_object_by_id("broken"), Err(RtError::DynamicError(message)) if message == "ошибка адресного чтения")
    );
    assert_eq!(source.loads.load(Ordering::Relaxed), 1);
}

#[test]
fn section_type_discovery_preserves_unknown_attributes_without_materializing_details() {
    let source = Arc::new(Details::default());
    let catalog = MetadataCatalog::new(
        "configuration",
        vec![
            MetadataObject::new("target", "Проба", MetadataKind::Document)
                .with_document_tabular_sections(vec![DocumentTabularSection::new(
                    "items",
                    "Товары",
                )]),
        ],
    )
    .unwrap()
    .with_object_source(source.clone());
    let mut builder = open_bsl::Engine::builder();
    for ty in catalog.document_tabular_section_types() {
        builder = builder.register_value_type(ty.clone());
    }
    let engine = builder.build().unwrap();
    let entry = engine.compile(
        "Возврат Тип(\"ДокументТабличнаяЧасть.Проба.Товары\") = Тип(\"DocumentTabularSection.Проба.Товары\");"
    ).unwrap();
    assert_eq!(
        engine.new_state().run(&entry).unwrap(),
        Value::Boolean(true)
    );
    let section = &catalog.objects()[0].document_tabular_sections().unwrap()[0];
    assert!(section.attributes().is_none());
    assert_eq!(source.loads.load(Ordering::Relaxed), 0);
}

#[test]
fn a_large_discovery_index_does_not_load_unrequested_object_details() {
    let source = Arc::new(Details::default());
    let mut objects = vec![MetadataObject::new("target", "Цель", MetadataKind::Catalog)];
    objects.extend((0..1000).map(|index| {
        MetadataObject::new(
            format!("unused{index}"),
            format!("Лишний{index}"),
            MetadataKind::Catalog,
        )
    }));
    let catalog = MetadataCatalog::new("configuration", objects)
        .unwrap()
        .with_object_source(source.clone());
    assert_eq!(catalog.objects().len(), 1001);
    assert_eq!(source.loads.load(Ordering::Relaxed), 0);
    assert!(
        catalog
            .materialized_object_by_id("target")
            .unwrap()
            .is_some()
    );
    assert_eq!(source.loads.load(Ordering::Relaxed), 1);
}
