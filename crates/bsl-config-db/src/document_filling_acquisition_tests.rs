use super::{resource, text_resource};
use bsl_config::ConfigurationCodeSnapshot;
use open_sdbl::metadata::{
    DbNameEntry, DbNames, SchemaStorage, parse_config_descriptors, resolve_metadata,
};

#[test]
fn acquired_event_section_fields_match_native_ownership_and_order() {
    let descriptors = parse_config_descriptors(
        "571e1bd9-0f64-4a05-a276-dba4fe242ad0",
        include_bytes!("../tests/fixtures/document-owned-attributes/event.deflate"),
    )
    .unwrap();
    let rows =
        include_str!("../tests/fixtures/document-owned-attributes/event-section-fields.native.tsv")
            .lines()
            .map(|line| line.split('\t').collect::<Vec<_>>())
            .collect::<Vec<_>>();
    assert_eq!(rows.len(), 37);
    let mut sections = std::collections::HashSet::new();
    for row in &rows {
        let section = descriptors
            .iter()
            .find(|descriptor| descriptor.object_guid.as_str() == row[0])
            .unwrap();
        assert_eq!(section.name, row[1]);
        let fields = section.tabular_section_attributes.as_ref().unwrap();
        let index = row[2].parse::<usize>().unwrap();
        assert_eq!(fields[index].as_str(), row[3]);
        let field = descriptors
            .iter()
            .find(|descriptor| descriptor.object_guid == fields[index])
            .unwrap();
        assert_eq!(field.name, row[4]);
        assert!(field.declared_types.is_some(), "{}", field.name);
        if sections.insert(row[0]) {
            assert_eq!(
                fields.len(),
                rows.iter().filter(|other| other[0] == row[0]).count()
            );
        }
    }
    assert_eq!(sections.len(), 5);
    let metadata = resolve_metadata(
        DbNames::from_entries(vec![]),
        descriptors,
        SchemaStorage {
            tables: vec![],
            anomalies: vec![],
        },
        vec![],
    )
    .snapshot;
    let catalog = bsl_orm::MetadataCatalog::new(
        "native-event-sections",
        vec![bsl_orm::MetadataObject::new(
            "571e1bd9-0f64-4a05-a276-dba4fe242ad0",
            "Событие",
            bsl_orm::MetadataKind::Document,
        )],
    )
    .unwrap();
    let catalog = crate::document_attributes::project(catalog, &metadata).unwrap();
    let sections = catalog.objects()[0].document_tabular_sections().unwrap();
    for section in sections {
        let expected = rows
            .iter()
            .filter(|row| row[0] == section.id())
            .map(|row| (row[3], row[4]))
            .collect::<Vec<_>>();
        let actual = section
            .attributes()
            .unwrap()
            .iter()
            .map(|field| (field.id(), field.name()))
            .collect::<Vec<_>>();
        assert_eq!(actual, expected);
    }
}

#[test]
fn native_enumeration_filling_value_survives_acquisition_and_projection() {
    let event = "571e1bd9-0f64-4a05-a276-dba4fe242ad0";
    let enumeration = "19430845-e6b2-4a2b-944a-75142d4914c0";
    let event_bytes = include_bytes!("../tests/fixtures/document-owned-attributes/event.deflate");
    let enumeration_bytes = include_bytes!(
        "../tests/fixtures/document-owned-attributes/importance-enumeration.deflate"
    );
    let mut descriptors = parse_config_descriptors(event, event_bytes).unwrap();
    descriptors.extend(parse_config_descriptors(enumeration, enumeration_bytes).unwrap());
    let metadata = resolve_metadata(
        // Физическая идентичность взята из соответствующего DbNames УНФ.
        DbNames::from_entries(vec![DbNameEntry {
            guid: enumeration.parse().unwrap(),
            alias: "Enum".into(),
            number: 9292,
        }]),
        descriptors,
        SchemaStorage {
            tables: vec![],
            anomalies: vec![],
        },
        vec![],
    )
    .snapshot;
    assert!(
        !metadata.values().is_empty(),
        "значения перечисления отсутствуют в снимке: {:?}",
        metadata.objects()
    );
    let resources = vec![
        text_resource("root", "{1,{11111111-1111-1111-1111-111111111111}}"),
        text_resource(
            "11111111-1111-1111-1111-111111111111",
            include_str!(
                "../tests/fixtures/document-owned-attributes/isolated-enumeration-root.txt"
            ),
        ),
        resource(event, event_bytes),
        resource(enumeration, enumeration_bytes),
    ];
    let code = ConfigurationCodeSnapshot::new("isolated-root", vec![]).unwrap();
    let catalog =
        crate::load_catalog_base_with_metadata(&resources, &code, vec![], Some(&metadata)).unwrap();
    let catalog = std::sync::Arc::new(crate::project_catalog_values(catalog, &metadata).unwrap());
    let importance = catalog
        .object_by_id(event)
        .unwrap()
        .document_attributes()
        .unwrap()
        .iter()
        .find(|attribute| attribute.name() == "Важность")
        .unwrap();
    let mut builder = open_bsl::Engine::builder().register_library(bsl_orm::library());
    for ty in catalog.transferable_value_types() {
        builder = builder.register_value_type(ty);
    }
    let engine = builder.build().unwrap();
    let mut shapes = open_bsl::RuntimeShapes::seeded(vec![], vec![], Some(engine.registry()));
    let actual = importance
        .fill_value()
        .expect("нативный DesignTimeRef должен сохраниться после проекции")
        .materialize(&mut shapes)
        .unwrap();
    // DesignTimeRef и идентичности взяты из штатной XML, не из SDK.
    let ty = engine
        .registry()
        .value_types()
        .iter()
        .find(|ty| ty.names().contains(&"EnumRef.ВариантыВажности".to_owned()))
        .unwrap();
    let expected = ty
        .materialize(b"e2942d8d-433f-497b-8b7e-c999fbc88b44")
        .unwrap();
    assert_eq!(actual, vec![expected]);
}
