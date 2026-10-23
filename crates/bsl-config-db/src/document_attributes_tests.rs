use super::*;
use open_sdbl::metadata::{DbNames, SchemaStorage, parse_config_descriptors, resolve_metadata};

fn snapshot() -> MetadataSnapshot {
    let mut descriptors = parse_config_descriptors(
        "571e1bd9-0f64-4a05-a276-dba4fe242ad0",
        include_bytes!("../tests/fixtures/document-owned-attributes/event.deflate"),
    )
    .unwrap();
    descriptors.extend(
        parse_config_descriptors(
            "51c403a3-e062-4f0d-b39c-11989c0ffe2b",
            include_bytes!("../tests/fixtures/document-owned-attributes/basis.deflate"),
        )
        .unwrap(),
    );
    resolve_metadata(
        DbNames::from_entries(vec![]),
        descriptors,
        SchemaStorage {
            tables: vec![],
            anomalies: vec![],
        },
        vec![],
    )
    .snapshot
}

#[test]
fn native_document_sections_keep_their_owned_order_without_guessing_row_fields() {
    let catalog = MetadataCatalog::new(
        "native-event",
        vec![bsl_orm::MetadataObject::new(
            "571e1bd9-0f64-4a05-a276-dba4fe242ad0",
            "Событие",
            MetadataKind::Document,
        )],
    )
    .unwrap();
    let catalog = project(catalog, &snapshot()).unwrap();
    let sections = catalog.objects()[0].document_tabular_sections().unwrap();
    // Идентичности и порядок сверены с собственным ChildObjects нативной XML.
    let expected = [
        ("0d63eba3-9057-4617-bc37-8ac5e4f0b84f", "ДокументыОснования"),
        ("80bbb358-f129-4b0d-9001-73a6a9334a92", "УдалитьУчастники"),
        (
            "61529d5a-a23e-494d-83d4-7d3b056da8a9",
            "ДополнительныеРеквизиты",
        ),
        ("6d37013d-64bb-42ac-8702-79f46a4c606a", "РесурсыПредприятия"),
        ("8525b5d3-0604-4dc7-a6a2-a017d74e768d", "Участники"),
    ];
    assert_eq!(
        sections
            .iter()
            .map(|section| (section.id(), section.name()))
            .collect::<Vec<_>>(),
        expected
    );
    assert!(
        sections
            .iter()
            .all(|section| section.attributes().is_some())
    );
}

#[test]
fn native_members_and_known_primitive_defaults_survive_projection() {
    let catalog = MetadataCatalog::new(
        "native-event",
        vec![bsl_orm::MetadataObject::new(
            "571e1bd9-0f64-4a05-a276-dba4fe242ad0",
            "Событие",
            MetadataKind::Document,
        )],
    )
    .unwrap();
    let catalog = project(catalog, &snapshot()).unwrap();
    let attributes = catalog.objects()[0].document_attributes().unwrap();
    assert_eq!(attributes.len(), 26);
    let boolean = attributes
        .iter()
        .find(|attribute| attribute.name() == "КонтактыПеренесены")
        .unwrap();
    assert_eq!(boolean.types()[0].id, TypeRef::Native(TypeId::Boolean));
    let mut shapes = RuntimeShapes::seeded(vec![], vec![], None);
    assert_eq!(
        boolean
            .fill_value()
            .unwrap()
            .materialize(&mut shapes)
            .unwrap(),
        vec![open_bsl::Value::Boolean(false)]
    );
    let storage = attributes
        .iter()
        .find(|attribute| attribute.name() == "КартинкиHTML")
        .unwrap();
    assert!(matches!(storage.types()[0].id, TypeRef::Object(ty) if ty.name == "ХранилищеЗначения"));
    // Владельцев ссылок в этой ограниченной фикстуре нет. Полный список
    // реквизитов сохраняется, но недоступный тип и значение не угадываются.
    let enumeration = attributes
        .iter()
        .find(|attribute| attribute.name() == "Важность")
        .unwrap();
    assert!(enumeration.types().is_empty());
    assert!(enumeration.fill_value().is_none());
}

#[test]
fn every_expanded_member_binds_to_its_exact_native_owner_type() {
    let rows: Vec<_> =
        include_str!("../../../docs/research/unf-event-basis-reference-identities.tsv")
            .lines()
            .skip(1)
            .map(|line| line.split('\t').collect::<Vec<_>>())
            .collect();
    let objects = rows
        .iter()
        .map(|row| {
            bsl_orm::MetadataObject::new(
                row[1],
                row[3],
                if row[0] == "Document" {
                    MetadataKind::Document
                } else {
                    MetadataKind::Catalog
                },
            )
        })
        .collect();
    let catalog = Arc::new(MetadataCatalog::new("native-basis", objects).unwrap());
    let mut references = References::new();
    for row in &rows {
        let kind = if row[0] == "Document" {
            MetadataKind::Document
        } else {
            MetadataKind::Catalog
        };
        let name = format!(
            "{}.{}",
            if kind == MetadataKind::Document {
                "DocumentRef"
            } else {
                "CatalogRef"
            },
            row[3]
        );
        let ty = catalog
            .value_types()
            .find(|ty| ty.names().contains(&name))
            .unwrap();
        references.insert(row[2], (kind, Arc::clone(ty)));
    }
    let mut builder = Engine::builder().register_library(bsl_orm::library());
    for ty in catalog.transferable_value_types() {
        builder = builder.register_value_type(ty);
    }
    let engine = builder.build().unwrap();
    let shapes = RuntimeShapes::seeded(vec![], vec![], Some(engine.registry()));
    let metadata = snapshot();
    let descriptor = metadata
        .descriptors()
        .iter()
        .find(|descriptor| {
            descriptor.object_guid.as_str() == "e350fdab-d2d1-45e8-bd75-a2ee410245f1"
        })
        .unwrap();
    let attribute = attribute(descriptor, &references, &engine, &shapes).unwrap();
    assert_eq!(attribute.types().len(), 56);
    for (column, row) in attribute.types().iter().zip(rows) {
        let TypeRef::Value(ty) = &column.id else {
            panic!("конкретный тип ссылки")
        };
        assert!(ty.names().contains(&format!(
            "{}.{}",
            if row[0] == "Document" {
                "DocumentRef"
            } else {
                "CatalogRef"
            },
            row[3]
        )));
    }
}

#[test]
fn catalog_attachment_rejects_unknown_and_non_document_owners() {
    for owner in ["missing", "catalog"] {
        let catalog = MetadataCatalog::new(
            "configuration",
            vec![bsl_orm::MetadataObject::new(
                "catalog",
                "Проба",
                MetadataKind::Catalog,
            )],
        )
        .unwrap();
        assert!(matches!(
            catalog.with_document_attributes(owner, vec![]),
            Err(bsl_orm::CatalogError::UnknownDocumentOwner(_))
        ));
    }
}

#[test]
fn acquired_enumeration_default_uses_the_existing_transferable_graph() {
    let catalog = Arc::new(
        MetadataCatalog::new(
            "native-default",
            vec![bsl_orm::MetadataObject::new(
                "enum",
                "ВариантыВажности",
                MetadataKind::Enumeration,
            )],
        )
        .unwrap()
        .with_enumeration_values(vec![bsl_orm::ConfigurationEnumValue::new(
            "enum",
            "e2942d8d-433f-497b-8b7e-c999fbc88b44",
            "Обычная",
        )])
        .unwrap(),
    );
    let declaration = catalog.value_types().next().unwrap().clone();
    let references = References::from(HashMap::from([(
        "981c7e3c-694b-4328-8937-217bb303823c",
        (MetadataKind::Enumeration, declaration.clone()),
    )]));
    let mut builder = Engine::builder().register_library(bsl_orm::library());
    for ty in catalog.transferable_value_types() {
        builder = builder.register_value_type(ty);
    }
    let engine = builder.build().unwrap();
    let mut shapes = RuntimeShapes::seeded(vec![], vec![], Some(engine.registry()));
    let metadata = snapshot();
    let descriptor = metadata
        .descriptors()
        .iter()
        .find(|descriptor| descriptor.name == "Важность")
        .unwrap();
    let attribute = attribute(descriptor, &references, &engine, &shapes).unwrap();
    let value = attribute
        .fill_value()
        .unwrap()
        .materialize(&mut shapes)
        .unwrap()
        .pop()
        .unwrap();
    assert_eq!(
        value.type_of().unwrap(),
        open_bsl::Value::Type(TypeRef::Value(declaration))
    );
    let expected = engine
        .registry()
        .value_types()
        .iter()
        .find(|ty| ty.names().contains(&"EnumRef.ВариантыВажности".to_owned()))
        .unwrap()
        .materialize(b"e2942d8d-433f-497b-8b7e-c999fbc88b44")
        .unwrap();
    assert_eq!(value, expected);
}

#[test]
fn acquired_missing_enumeration_default_keeps_the_transferable_identity() {
    let catalog = Arc::new(
        MetadataCatalog::new(
            "native-default",
            vec![bsl_orm::MetadataObject::new(
                "enum",
                "ВариантыВажности",
                MetadataKind::Enumeration,
            )],
        )
        .unwrap(),
    );
    let declaration = catalog.value_types().next().unwrap().clone();
    let references = References::from(HashMap::from([(
        "981c7e3c-694b-4328-8937-217bb303823c",
        (MetadataKind::Enumeration, declaration.clone()),
    )]));
    let mut builder = Engine::builder().register_library(bsl_orm::library());
    for ty in catalog.transferable_value_types() {
        builder = builder.register_value_type(ty);
    }
    let engine = builder.build().unwrap();
    let mut shapes = RuntimeShapes::seeded(vec![], vec![], Some(engine.registry()));
    let metadata = snapshot();
    let descriptor = metadata
        .descriptors()
        .iter()
        .find(|descriptor| descriptor.name == "Важность")
        .unwrap();
    let attribute = attribute(descriptor, &references, &engine, &shapes).unwrap();
    let value = attribute
        .fill_value()
        .unwrap()
        .materialize(&mut shapes)
        .unwrap()
        .pop()
        .unwrap();
    assert_eq!(
        value.type_of().unwrap(),
        open_bsl::Value::Type(TypeRef::Value(declaration))
    );
    let expected = engine
        .registry()
        .value_types()
        .iter()
        .find(|ty| ty.names().contains(&"EnumRef.ВариантыВажности".to_owned()))
        .unwrap()
        .materialize(b"e2942d8d-433f-497b-8b7e-c999fbc88b44")
        .unwrap();
    assert_eq!(value, expected);
}

fn characteristic_snapshot(update: impl FnOnce(&mut Vec<ConfigDescriptor>)) -> MetadataSnapshot {
    let mut descriptors = parse_config_descriptors(
        "d5d8567e-ef0b-4eba-962c-1db134b6a551",
        include_bytes!("../../../docs/research/fixtures/characteristic-manager-attributes/d5d8567e-ef0b-4eba-962c-1db134b6a551.config.deflate"),
    )
    .unwrap();
    update(&mut descriptors);
    resolve_metadata(
        DbNames::from_entries(vec![]),
        descriptors,
        SchemaStorage {
            tables: vec![],
            anomalies: vec![],
        },
        vec![],
    )
    .snapshot
}

fn characteristic_catalog() -> MetadataCatalog {
    MetadataCatalog::new(
        "native-characteristic",
        vec![bsl_orm::MetadataObject::new(
            "d5d8567e-ef0b-4eba-962c-1db134b6a551",
            "ЗамерХарактеристик",
            MetadataKind::ChartOfCharacteristicTypes,
        )],
    )
    .unwrap()
}

#[test]
fn native_characteristic_settings_and_own_attributes_reach_orm() {
    let catalog = project(characteristic_catalog(), &characteristic_snapshot(|_| {})).unwrap();
    let owner = &catalog.objects()[0];
    let settings = owner.characteristic_settings().unwrap();
    assert_eq!((settings.code_length, settings.description_length), (9, 25));
    assert_eq!(settings.value_types.len(), 4);
    let string = settings
        .value_types
        .iter()
        .find(|ty| ty.id == TypeRef::Native(TypeId::String))
        .unwrap();
    assert_eq!(string.quals, ["100", "1"]);
    let attributes = owner.characteristic_attributes().unwrap();
    assert_eq!(attributes.len(), 1);
    assert_eq!(attributes[0].name(), "УникальныйКодДляПоля");
    assert_eq!(attributes[0].types()[0].quals, ["12", "1"]);
}

#[test]
fn characteristic_projection_preserves_unknown_and_known_empty_metadata() {
    for known in [false, true] {
        let metadata = characteristic_snapshot(|descriptors| {
            let owner = &mut descriptors[0];
            owner.characteristic_settings = None;
            owner.characteristic_attributes = known.then(Vec::new);
        });
        let catalog = project(characteristic_catalog(), &metadata).unwrap();
        let owner = &catalog.objects()[0];
        assert!(owner.characteristic_settings().is_none());
        assert_eq!(
            owner.characteristic_attributes().map(<[_]>::len),
            known.then_some(0)
        );
    }
}

#[test]
fn characteristic_projection_resolves_its_own_reference_type_and_rejects_unknown_types() {
    let metadata = characteristic_snapshot(|descriptors| {
        let reference = descriptors[0].object_reference_type.clone();
        let ty = &mut descriptors[0]
            .characteristic_settings
            .as_mut()
            .unwrap()
            .value_types[0];
        ty.tag = "#".into();
        ty.reference = reference;
        ty.qualifiers.clear();
    });
    let catalog = project(characteristic_catalog(), &metadata).unwrap();
    assert!(matches!(
        catalog.objects()[0]
            .characteristic_settings()
            .unwrap()
            .value_types[0]
            .id,
        TypeRef::Value(_)
    ));
    let unknown = characteristic_snapshot(|descriptors| {
        descriptors[0]
            .characteristic_settings
            .as_mut()
            .unwrap()
            .value_types[0]
            .tag = "unknown".into();
    });
    assert!(project(characteristic_catalog(), &unknown).is_err());
}

#[test]
fn native_business_process_and_task_references_reach_characteristic_types_and_facade_lookup() {
    let controls = [
        (
            "6db4bbe0-ce4c-4389-ad25-9a10c1857192",
            "ЗамерБизнесПроцесса",
            MetadataKind::BusinessProcess,
            "1be70b29-c26d-47ec-9cae-9806560af3dd",
            include_bytes!("../../../docs/research/fixtures/demo-business-task-reference-types/6db4bbe0-ce4c-4389-ad25-9a10c1857192.native.deflate").as_slice(),
        ),
        (
            "2adee501-a0c9-4849-9fd0-a3d10679d4f7",
            "ЗамерЗадачи",
            MetadataKind::Task,
            "bfac82af-f7c8-4c14-93bf-8cbd4824cb3c",
            include_bytes!("../../../docs/research/fixtures/demo-business-task-reference-types/2adee501-a0c9-4849-9fd0-a3d10679d4f7.native.deflate").as_slice(),
        ),
    ];
    let mut descriptors = Vec::new();
    for (id, _, _, reference, bytes) in controls {
        let acquired = parse_config_descriptors(id, bytes).unwrap();
        let owner = acquired
            .iter()
            .find(|descriptor| descriptor.object_guid.as_str() == id)
            .unwrap();
        assert_eq!(
            owner.object_reference_type.as_ref().unwrap().as_str(),
            reference
        );
        descriptors.extend(acquired);
    }
    let mut objects = controls
        .iter()
        .map(|(id, name, kind, _, _)| bsl_orm::MetadataObject::new(*id, *name, *kind))
        .collect::<Vec<_>>();
    objects.extend(characteristic_catalog().objects().iter().cloned());
    let mut characteristic = characteristic_snapshot(|_| {}).descriptors().to_vec();
    characteristic[0]
        .characteristic_settings
        .as_mut()
        .unwrap()
        .value_types = controls
        .iter()
        .map(|(_, _, _, reference, _)| ConfigDeclaredType {
            tag: "#".into(),
            reference: Some(reference.parse().unwrap()),
            qualifiers: vec![],
        })
        .collect();
    descriptors.extend(characteristic);
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
    let catalog = Arc::new(
        project(
            MetadataCatalog::new("native-business-task", objects).unwrap(),
            &metadata,
        )
        .unwrap(),
    );
    let configured = catalog
        .object_by_id("d5d8567e-ef0b-4eba-962c-1db134b6a551")
        .unwrap()
        .characteristic_settings()
        .unwrap();
    assert_eq!(configured.value_types.len(), 2);
    assert_ne!(configured.value_types[0].id, configured.value_types[1].id);
    let mut builder = Engine::builder();
    for ty in catalog.value_types() {
        builder = builder.register_value_type(Arc::clone(ty));
    }
    let engine = builder.build().unwrap();
    let module = engine.compile(r#"Возврат Тип("БизнесПроцессСсылка.ЗамерБизнесПроцесса") = Тип("BusinessProcessRef.ЗамерБизнесПроцесса")
И Тип("ЗадачаСсылка.ЗамерЗадачи") = Тип("TaskRef.ЗамерЗадачи")
И Тип("TaskRef.ЗамерЗадачи") <> Тип("BusinessProcessRef.ЗамерБизнесПроцесса");"#).unwrap();
    assert_eq!(
        engine.state_builder().build().run(&module).unwrap(),
        open_bsl::Value::Boolean(true)
    );
}
