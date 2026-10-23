use super::*;

fn set(
    catalog: MetadataCatalog,
    kind: MetadataKind,
    owner: &str,
    names: &[&str],
) -> MetadataCatalog {
    let sections = names
        .iter()
        .map(|name| DocumentTabularSection::new(*name, *name))
        .collect();
    match kind {
        MetadataKind::Document => catalog.with_document_tabular_sections(owner, sections),
        MetadataKind::DataProcessor => {
            catalog.with_data_processor_metadata(owner, None, Some(sections))
        }
        MetadataKind::Report => catalog.with_report_metadata(owner, None, Some(sections)),
        MetadataKind::BusinessProcess => {
            catalog.with_business_process_metadata(owner, None, None, Some(sections))
        }
        _ => unreachable!(),
    }
    .unwrap()
}

fn get(catalog: &MetadataCatalog, kind: MetadataKind) -> Vec<Arc<ValueTypeDescriptor>> {
    match kind {
        MetadataKind::Document => catalog.document_tabular_section_types().cloned().collect(),
        MetadataKind::DataProcessor => catalog
            .data_processor_tabular_section_types()
            .cloned()
            .collect(),
        MetadataKind::Report => catalog.report_tabular_section_types().cloned().collect(),
        MetadataKind::BusinessProcess => catalog
            .business_process_tabular_section_types()
            .cloned()
            .collect(),
        _ => unreachable!(),
    }
}

#[test]
fn changing_one_owner_keeps_other_type_descriptors_and_the_original_order() {
    for kind in [
        MetadataKind::Document,
        MetadataKind::DataProcessor,
        MetadataKind::Report,
        MetadataKind::BusinessProcess,
    ] {
        let mut catalog = MetadataCatalog::new(
            "config",
            vec![
                MetadataObject::new("a", "Первый", kind),
                MetadataObject::new("b", "Второй", kind),
                MetadataObject::new("c", "Третий", kind),
            ],
        )
        .unwrap();
        // Наполнение идёт в другом порядке, чем объекты каталога.
        catalog = set(catalog, kind, "c", &["Строки"]);
        let last = get(&catalog, kind);
        catalog = set(catalog, kind, "a", &["До"]);
        catalog = set(catalog, kind, "b", &["Средняя"]);
        catalog = set(catalog, kind, "a", &["После", "Еще"]);
        let types = get(&catalog, kind);
        assert_eq!(types.len(), 8);
        assert!(types[0].names()[0].contains("Первый.После"));
        assert!(types[2].names()[0].contains("Первый.Еще"));
        assert!(types[4].names()[0].contains("Второй.Средняя"));
        assert!(Arc::ptr_eq(&types[6], &last[0]));
        assert!(Arc::ptr_eq(&types[7], &last[1]));
        let middle = Arc::clone(&types[4]);
        catalog = set(catalog, kind, "a", &[]);
        let empty_first = get(&catalog, kind);
        assert_eq!(empty_first.len(), 4);
        assert!(Arc::ptr_eq(&empty_first[0], &middle));
        assert!(Arc::ptr_eq(&empty_first[2], &last[0]));
    }
}
