use super::{
    CatalogCodeType, CatalogDefaultPresentation, CatalogError, CatalogStandardAttributes,
    MetadataCatalog, MetadataKind, MetadataObject,
};

#[test]
fn unknown_standard_settings_are_not_numeric_zero_settings() {
    let unknown = MetadataObject::new("unknown", "Неизвестный", MetadataKind::Catalog);
    assert_eq!(unknown.catalog_standard_attributes(), None);
    assert_eq!(unknown.catalog_default_presentation(), None);
    assert_eq!(unknown.object_module(), None);
    let settings = CatalogStandardAttributes {
        code_type: CatalogCodeType::Number,
        code_length: 9,
        description_length: 0,
    };
    let known = MetadataObject::new("known", "Известный", MetadataKind::Catalog)
        .with_catalog_standard_attributes(settings)
        .with_catalog_default_presentation(CatalogDefaultPresentation::AsDescription)
        .with_object_module("Справочник.Известный.МодульОбъекта");
    let catalog = MetadataCatalog::new("configuration", vec![unknown, known]).unwrap();
    let known = catalog.object_by_id("known").unwrap();
    assert_eq!(known.catalog_standard_attributes(), Some(settings));
    assert_eq!(
        known.catalog_default_presentation(),
        Some(CatalogDefaultPresentation::AsDescription)
    );
    assert_eq!(
        known.object_module(),
        Some("Справочник.Известный.МодульОбъекта")
    );
    assert_eq!(
        catalog
            .object_by_id("unknown")
            .unwrap()
            .catalog_standard_attributes(),
        None
    );
}

#[test]
fn object_module_binding_is_independent_of_manager_and_rejects_an_empty_name() {
    let object = MetadataObject::new("catalog", "Тест", MetadataKind::Catalog)
        .with_manager_module("Менеджер")
        .with_object_module("Объект");
    assert_eq!(object.manager_module(), Some("Менеджер"));
    assert_eq!(object.object_module(), Some("Объект"));
    let error =
        MetadataCatalog::new("configuration", vec![object.with_object_module("")]).unwrap_err();
    assert!(matches!(error, CatalogError::EmptyObjectModule { object } if object == "catalog"));
}

#[test]
fn catalog_object_type_is_distinct_from_reference_and_has_no_transfer_codec() {
    use open_bsl::{Engine, TypeRef, Value};
    use std::sync::Arc;
    let catalog = MetadataCatalog::new(
        "configuration",
        vec![MetadataObject::new(
            "catalog",
            "ЗамерВременныхТаблиц",
            MetadataKind::Catalog,
        )],
    )
    .unwrap();
    let object_type = catalog.catalog_object_types().next().unwrap();
    let reference_type = catalog.value_types().next().unwrap();
    assert_ne!(
        TypeRef::Value(Arc::clone(object_type)),
        TypeRef::Value(Arc::clone(reference_type))
    );
    assert!(!object_type.has_materializer());
    assert_eq!(
        object_type.display(),
        "Справочник объект: ЗамерВременныхТаблиц"
    );
    let engine = Engine::builder()
        .register_value_type(Arc::clone(object_type))
        .register_value_type(Arc::clone(reference_type))
        .build()
        .unwrap();
    let module = engine.compile("Возврат Тип(\"CatalogObject.ЗамерВременныхТаблиц\") = Тип(\"справочникобъект.замервременныхтаблиц\") И Тип(\"CatalogObject.ЗамерВременныхТаблиц\") <> Тип(\"CatalogRef.ЗамерВременныхТаблиц\");").unwrap();
    assert_eq!(
        engine.new_state().run(&module).unwrap(),
        Value::Boolean(true)
    );
}

#[test]
fn acquired_sections_distinguish_unknown_empty_and_owned_names() {
    let unknown = MetadataObject::new("catalog", "Тест", MetadataKind::Catalog);
    assert_eq!(unknown.catalog_tabular_section_names(), None);
    let empty = unknown
        .clone()
        .with_catalog_tabular_section_names([] as [&str; 0]);
    assert_eq!(empty.catalog_tabular_section_names(), Some([].as_slice()));
    let known = unknown.with_catalog_tabular_section_names(["Состав", "Дополнительно"]);
    assert_eq!(
        known.catalog_tabular_section_names().unwrap(),
        ["Состав", "Дополнительно"]
    );
    assert_eq!(known.catalog_attribute_names(), None);
}
