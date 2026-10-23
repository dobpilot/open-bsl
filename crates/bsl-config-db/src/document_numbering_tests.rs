use super::{resource, text_resource};
use bsl_config::ConfigurationCodeSnapshot;
use bsl_orm::{DocumentNumberSettings, DocumentNumberType, MetadataKind, MetadataObject};

#[test]
fn every_native_periodicity_and_shared_numerator_survives_the_loader() {
    let fixtures: &[(&str, &[u8])] = &[
        (
            "PeriodNone",
            include_bytes!("../tests/fixtures/document-numbering-scope/PeriodNone.deflate"),
        ),
        (
            "PeriodYear",
            include_bytes!("../tests/fixtures/document-numbering-scope/PeriodYear.deflate"),
        ),
        (
            "PeriodQuarter",
            include_bytes!("../tests/fixtures/document-numbering-scope/PeriodQuarter.deflate"),
        ),
        (
            "PeriodMonth",
            include_bytes!("../tests/fixtures/document-numbering-scope/PeriodMonth.deflate"),
        ),
        (
            "PeriodDay",
            include_bytes!("../tests/fixtures/document-numbering-scope/PeriodDay.deflate"),
        ),
        (
            "SharedA",
            include_bytes!("../tests/fixtures/document-numbering-scope/SharedA.deflate"),
        ),
        (
            "SharedB",
            include_bytes!("../tests/fixtures/document-numbering-scope/SharedB.deflate"),
        ),
    ];
    for row in include_str!("../tests/fixtures/document-numbering-scope/minimal-native.tsv").lines()
    {
        let cells: Vec<_> = row.split('\t').collect();
        let bytes = fixtures
            .iter()
            .find(|(name, _)| *name == cells[1])
            .unwrap()
            .1;
        // Синтетический корень ограничивает загрузку нативным описателем;
        // ожидаемые свойства получены SDK из XML, выгруженной самой платформой.
        let root = include_str!("../tests/fixtures/document-owned-attributes/isolated-root.txt")
            .replace("571e1bd9-0f64-4a05-a276-dba4fe242ad0", cells[0]);
        let resources = vec![
            text_resource("root", "{1,{11111111-1111-1111-1111-111111111111}}"),
            text_resource("11111111-1111-1111-1111-111111111111", &root),
            resource(cells[0], bytes),
        ];
        let code = ConfigurationCodeSnapshot::new("numbering-tests", vec![]).unwrap();
        let catalog = crate::load_catalog_base(&resources, &code, vec![]).unwrap();
        let object = catalog.object_by_id(cells[0]).unwrap();
        assert_eq!(object.name(), cells[1]);
        let settings = object.document_numbering_settings().unwrap();
        assert_eq!(format!("{:?}", settings.periodicity), cells[2]);
        assert_eq!(
            settings.numerator.as_deref(),
            (!cells[3].is_empty()).then_some(cells[3])
        );
        assert_ne!(settings.numerator.as_deref(), Some(object.id()));
    }
}

#[test]
fn known_number_type_does_not_fabricate_a_numbering_scope() {
    let object = MetadataObject::new("unknown", "Неизвестный", MetadataKind::Document)
        .with_document_number_settings(DocumentNumberSettings {
            number_type: DocumentNumberType::String,
            number_length: 9,
        });
    assert!(object.document_numbering_settings().is_none());
    assert!(object.document_number_settings().is_some());
}
