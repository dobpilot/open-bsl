use open_sdbl::metadata::{
    DbNameEntry, DbNames, SchemaStorage, parse_config_descriptors, parse_config_predefined_values,
    resolve_metadata_with_predefined_values,
};

#[test]
fn native_predefined_names_project_the_decoded_snapshot_without_rereading_resources() {
    let rows = include_str!("../tests/fixtures/catalog-predefined-items/order-kinds.native.tsv")
        .lines()
        .map(|line| line.split('\t').collect::<Vec<_>>())
        .collect::<Vec<_>>();
    let owner = rows[0][1];
    let descriptors = parse_config_descriptors(
        owner,
        include_bytes!("../tests/fixtures/catalog-predefined-items/order-kinds.deflate"),
    )
    .unwrap();
    let predefined = parse_config_predefined_values(
        &format!("{owner}.1c"),
        include_bytes!("../tests/fixtures/catalog-predefined-items/order-kinds.1c.deflate"),
    )
    .unwrap();
    assert_eq!(predefined.len(), 1);
    assert_eq!(predefined[0].value_guid.as_str(), rows[1][1]);
    assert_eq!(predefined[0].name, rows[1][2]);
    let snapshot = resolve_metadata_with_predefined_values(
        DbNames::from_entries(vec![DbNameEntry {
            guid: owner.parse().unwrap(),
            alias: "Reference".into(),
            number: 1,
        }]),
        descriptors,
        predefined,
        SchemaStorage {
            tables: vec![],
            anomalies: vec![],
        },
        vec![],
    )
    .snapshot;
    let names = super::catalog_predefined_names(Some(&snapshot));
    assert_eq!(names.len(), 1);
    assert_eq!(names[owner], [rows[1][2]]);
    assert!(super::catalog_predefined_names(None).is_empty());
}

#[test]
fn characteristic_predefined_names_are_kept_from_the_acquired_snapshot() {
    let owner = "d5d8567e-ef0b-4eba-962c-1db134b6a551";
    let descriptors = parse_config_descriptors(
        owner,
        include_bytes!("../../../docs/research/fixtures/characteristic-manager-create/d5d8567e-ef0b-4eba-962c-1db134b6a551.config.deflate"),
    ).unwrap();
    let snapshot = resolve_metadata_with_predefined_values(
        DbNames::from_entries(vec![DbNameEntry {
            guid: owner.parse().unwrap(),
            alias: "Chrc".into(),
            number: 1,
        }]),
        descriptors,
        vec![open_sdbl::metadata::ConfigPredefinedValue {
            owner_guid: owner.parse().unwrap(),
            value_guid: "9b6065e7-092d-4be9-bed8-5b580763721c".parse().unwrap(),
            name: "Основной".into(),
            source: open_sdbl::metadata::PredefinedSource::ChartOfCharacteristicTypes,
        }],
        SchemaStorage {
            tables: vec![],
            anomalies: vec![],
        },
        vec![],
    )
    .snapshot;
    assert_eq!(
        super::catalog_predefined_names(Some(&snapshot))[owner],
        ["Основной"]
    );
}
