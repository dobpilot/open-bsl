use super::*;

/// Метаданные из типовой конфигурации лежат в неотслеживаемом каталоге
/// `private/`: без них тесты, которые их читают, пропускаются.
fn private_fixture(relative: &str) -> Option<Vec<u8>> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../private/fixtures")
        .join(relative);
    match std::fs::read(&path) {
        Ok(bytes) => Some(bytes),
        Err(_) => {
            eprintln!("пропуск: нет приватной фикстуры {}", path.display());
            None
        }
    }
}

fn native_types() -> Vec<ConfigDeclaredType> {
    open_sdbl::metadata::parse_config_descriptors(
        "4062674a-4a81-494d-82bc-4984cd7c7903",
        include_bytes!("../../../docs/research/fixtures/file-configuration-native-1022/native-generic-reference/config.deflate"),
    ).unwrap().into_iter().find(|d| d.name == "ЗначениеПустойСсылки").unwrap().declared_types.unwrap()
}

fn catalog(extra: Vec<bsl_orm::MetadataObject>) -> MetadataCatalog {
    let mut objects = vec![
        bsl_orm::MetadataObject::new("first", "Первый", MetadataKind::Catalog),
        bsl_orm::MetadataObject::new("second", "Второй", MetadataKind::Catalog),
        bsl_orm::MetadataObject::new("document", "Документ", MetadataKind::Document),
        bsl_orm::MetadataObject::new("enum", "Перечисление", MetadataKind::Enumeration),
    ];
    objects.extend(extra);
    MetadataCatalog::new("native-reference-set", objects).unwrap()
}

fn types(catalog: &MetadataCatalog) -> HashMap<&str, Arc<ValueTypeDescriptor>> {
    catalog
        .value_types()
        .flat_map(|ty| {
            ty.names()
                .iter()
                .map(move |name| (name.as_str(), Arc::clone(ty)))
        })
        .collect()
}

#[test]
fn native_families_use_full_catalog_without_any_reached_guid_descriptors() {
    let catalog = catalog(Vec::new());
    let types = types(&catalog);
    let references = References::for_catalog(&catalog, &types);
    assert!(references.by_guid.is_empty());
    let native = native_types();
    assert_eq!(native.len(), 8);
    let columns = references.columns(&native).unwrap();
    assert_eq!(columns.len(), 3);
    assert!(columns.iter().all(|column| column.quals.is_empty()));
    assert_eq!(
        columns
            .iter()
            .map(|column| column.id.name())
            .collect::<HashSet<_>>(),
        HashSet::from([
            "Первый".to_string(),
            "Второй".to_string(),
            "Документ".to_string()
        ])
    );
    let repeated = native.iter().chain(&native).cloned().collect::<Vec<_>>();
    assert_eq!(references.columns(&repeated).unwrap(), columns);
}

#[test]
fn unresolved_member_does_not_leave_a_successful_partial_family_union() {
    use std::str::FromStr;
    let catalog = catalog(Vec::new());
    let types = types(&catalog);
    let references = References::for_catalog(&catalog, &types);
    let mut native = native_types();
    native.push(ConfigDeclaredType {
        tag: "#".into(),
        reference: Some(
            open_sdbl::metadata::Guid::from_str("dddddddd-dddd-4ddd-8ddd-dddddddddddd").unwrap(),
        ),
        qualifiers: Vec::new(),
    });
    assert!(references.columns(&native).is_none());
}

#[test]
fn missing_concrete_reference_descriptor_invalidates_the_declared_family() {
    let catalog = catalog(vec![bsl_orm::MetadataObject::new(
        "calculation",
        "Расчет",
        MetadataKind::ChartOfCalculationTypes,
    )]);
    let mut types = types(&catalog);
    types.retain(|name, _| !name.ends_with(".Расчет"));
    let references = References::for_catalog(&catalog, &types);
    assert!(references.columns(&native_types()).is_none());
}

#[test]
fn selected_object_projection_reuses_full_catalog_reference_families() {
    use open_sdbl::metadata::{DbNames, SchemaStorage, parse_config_descriptors, resolve_metadata};
    let owner = || {
        bsl_orm::MetadataObject::new(
            "4062674a-4a81-494d-82bc-4984cd7c7903",
            "ИдентификаторыОбъектовМетаданных",
            MetadataKind::Catalog,
        )
    };
    let complete = catalog(vec![owner()]);
    let families = reference_families(&complete);
    let selected = MetadataCatalog::new("native-reference-set", vec![owner()]).unwrap();
    let descriptors = parse_config_descriptors("4062674a-4a81-494d-82bc-4984cd7c7903", include_bytes!("../../../docs/research/fixtures/file-configuration-native-1022/native-generic-reference/config.deflate")).unwrap();
    let snapshot = resolve_metadata(
        DbNames::from_entries(Vec::new()),
        descriptors,
        SchemaStorage {
            tables: Vec::new(),
            anomalies: Vec::new(),
        },
        Vec::new(),
    )
    .snapshot;
    let projected =
        project_runtime_with_references(selected, snapshot.runtime_facts(), true, Some(&families))
            .unwrap();
    let attributes = projected
        .object_by_id("4062674a-4a81-494d-82bc-4984cd7c7903")
        .unwrap()
        .catalog_attributes()
        .unwrap();
    let field = attributes
        .iter()
        .find(|field| field.name() == "ЗначениеПустойСсылки")
        .unwrap();
    assert_eq!(field.types().len(), 4);
    assert!(
        field
            .types()
            .iter()
            .any(|column| column.id.name() == "Второй")
    );
    assert!(
        field
            .types()
            .iter()
            .all(|column| column.id.name() != "Перечисление")
    );
}

#[test]
fn native_uuid_session_parameter_projects_to_the_existing_scalar_type() {
    let descriptors = open_sdbl::metadata::parse_config_descriptors(
        "53ee3eb6-d2d1-4fb3-8a78-1fa4543850a1",
        include_bytes!("../../../docs/research/fixtures/file-configuration-native-1022/native-uuid/config.deflate"),
    ).unwrap();
    let parameter = descriptors
        .iter()
        .find(|d| d.name == "КлючДанныхПовторногоИспользования")
        .unwrap();
    let declared = parameter.declared_types.as_ref().unwrap();
    let columns = References::new()
        .columns(declared)
        .unwrap_or_else(|| panic!("{declared:?}"));
    assert_eq!(
        columns,
        [ColumnType {
            id: TypeRef::Native(TypeId::Uuid),
            quals: Vec::new()
        }]
    );
}

#[test]
fn native_calculation_chart_reference_descriptor_participates_in_its_family() {
    let xml = include_str!(
        "../../../docs/research/fixtures/file-configuration-native-1022/native-calculation-reference/Chart.xml"
    );
    assert!(xml.contains("ChartOfCalculationTypesRef.Начисления"));
    let catalog = catalog(vec![bsl_orm::MetadataObject::new(
        "calculation",
        "Начисления",
        MetadataKind::ChartOfCalculationTypes,
    )]);
    let types = types(&catalog);
    let english = types.get("ChartOfCalculationTypesRef.Начисления").unwrap();
    let russian = types.get("ПланВидовРасчетаСсылка.Начисления").unwrap();
    assert!(Arc::ptr_eq(english, russian));
    let columns = References::for_catalog(&catalog, &types)
        .columns(&native_types())
        .unwrap();
    assert_eq!(columns.len(), 4);
    assert!(
        columns
            .iter()
            .any(|c| c.id == TypeRef::Value(Arc::clone(english)))
    );
}

#[test]
fn native_buh_catalog_attribute_keeps_its_declared_reference_type() {
    let Some(buh_target) =
        private_fixture("file-configuration-native-1022/native-buh-catalog-field/target.deflate")
    else {
        return;
    };
    let Some(buh_field_owner) =
        private_fixture("file-configuration-native-1022/native-buh-catalog-field/owner.deflate")
    else {
        return;
    };
    let target = open_sdbl::metadata::parse_config_header_bounded(
        "d5ada3af-5fe4-470a-a52d-888d88c9234f",
        &buh_target,
        32 * 1024 * 1024,
    )
    .unwrap()
    .unwrap();
    assert_eq!(
        target.reference_type,
        open_sdbl::metadata::ConfigTypeIdentity::Known(
            "7c6a7e8d-9204-47b1-8619-ea42fa2be95b".parse().unwrap()
        )
    );
    let owner = open_sdbl::metadata::parse_config_descriptors(
        "b1f594d2-00a1-43fd-bb21-df062df93840",
        &buh_field_owner,
    )
    .unwrap();
    let field = owner
        .iter()
        .find(|d| d.object_guid.as_str() == "0b1d2dd5-8ba1-4314-bf59-f8fb0e2141aa")
        .unwrap();
    let declared = field
        .declared_types
        .as_ref()
        .expect("native field Pattern must not be lost");
    assert_eq!(declared.len(), 1);
    assert_eq!(
        declared[0].reference.as_ref().unwrap().as_str(),
        "7c6a7e8d-9204-47b1-8619-ea42fa2be95b"
    );
}

#[test]
fn native_buh_defined_reference_projects_to_its_concrete_catalog_type() {
    let Some(buh_owner) = private_fixture(
        "file-configuration-native-1022/native-buh-defined-reference/owner.deflate",
    ) else {
        return;
    };
    let Some(buh_definition) = private_fixture(
        "file-configuration-native-1022/native-buh-defined-reference/definition.deflate",
    ) else {
        return;
    };
    let Some(buh_target) =
        private_fixture("file-configuration-native-1022/native-buh-catalog-field/target.deflate")
    else {
        return;
    };
    use open_sdbl::metadata::{DbNames, SchemaStorage, parse_config_descriptors, resolve_metadata};
    let owner = "bffeceba-fe82-4593-9d34-edc03d99fa44";
    let definition = "28a25822-2306-409a-a7c8-6f7e42bf25e8";
    let target = "d5ada3af-5fe4-470a-a52d-888d88c9234f";
    let mut descriptors = Vec::new();
    for (id, bytes) in [
        (owner, buh_owner.as_slice()),
        (definition, buh_definition.as_slice()),
        (target, buh_target.as_slice()),
    ] {
        descriptors.extend(parse_config_descriptors(id, bytes).unwrap());
    }
    let snapshot = resolve_metadata(
        DbNames::from_entries(Vec::new()),
        descriptors,
        SchemaStorage {
            tables: Vec::new(),
            anomalies: Vec::new(),
        },
        Vec::new(),
    )
    .snapshot;
    let model = MetadataCatalog::new(
        "native-buh-defined-reference",
        vec![
            bsl_orm::MetadataObject::new(owner, "Пользователи", MetadataKind::Catalog),
            bsl_orm::MetadataObject::new(target, "ПодразделенияОрганизаций", MetadataKind::Catalog),
        ],
    )
    .unwrap();
    let projected = project(model, &snapshot).unwrap();
    let field = projected
        .object_by_id(owner)
        .unwrap()
        .catalog_attributes()
        .unwrap()
        .iter()
        .find(|a| a.name() == "Подразделение")
        .unwrap();
    assert_eq!(
        field.types().len(),
        1,
        "{:?}",
        snapshot
            .descriptors()
            .iter()
            .filter(|d| d.name == "Подразделение")
            .collect::<Vec<_>>()
    );
    assert_eq!(field.types()[0].id.name(), "ПодразделенияОрганизаций");
}
