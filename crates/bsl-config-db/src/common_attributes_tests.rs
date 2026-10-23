use super::{resource, text_resource};
use bsl_config::ConfigurationCodeSnapshot;
use open_bsl::{TypeId, TypeRef};
use open_sdbl::metadata::{
    CommonAttributeContentEntry, CommonAttributeContentUse, CommonAttributeUsage,
    CommonAttributeUse, DbNames, SchemaStorage, parse_config_descriptors, resolve_metadata,
};

#[path = "separation_parameters_projection_tests.rs"]
mod separation_parameters_projection_tests;

fn project_usage(
    usage: Option<CommonAttributeUsage>,
) -> Result<bsl_orm::MetadataCatalog, crate::LoadError> {
    let id = "03f9d2d0-eae4-4098-a70e-4eb569b11fd9";
    let mut descriptor = parse_config_descriptors(
        id,
        include_bytes!(
            "../tests/fixtures/common-attributes/03f9d2d0-eae4-4098-a70e-4eb569b11fd9.deflate"
        ),
    )
    .unwrap()
    .into_iter()
    .find(|d| d.object_guid.as_str() == id)
    .unwrap();
    descriptor.common_attribute_usage = usage;
    let snapshot = resolve_metadata(
        DbNames::from_entries(vec![]),
        vec![descriptor],
        SchemaStorage {
            tables: vec![],
            anomalies: vec![],
        },
        vec![],
    )
    .snapshot;
    let catalog = bsl_orm::MetadataCatalog::new(
        "usage",
        vec![
            bsl_orm::MetadataObject::new(
                "9e9e2a57-8536-4983-a5c0-7841ead585e8",
                "Включённый",
                bsl_orm::MetadataKind::Catalog,
            ),
            bsl_orm::MetadataObject::new(
                "11111111-2222-4333-8444-555555555555",
                "Исключённый",
                bsl_orm::MetadataKind::Catalog,
            ),
        ],
    )
    .unwrap()
    .with_common_attribute_ids(vec![id.into()]);
    crate::project_catalog_values(catalog, &snapshot)
}

#[test]
fn usage_projection_preserves_explicit_overrides_and_known_empty() {
    let excluded = "11111111-2222-4333-8444-555555555555";
    let included = "9e9e2a57-8536-4983-a5c0-7841ead585e8";
    for usage in [
        CommonAttributeUsage {
            auto_use: CommonAttributeUse::DontUse,
            content: vec![CommonAttributeContentEntry {
                object: included.parse().unwrap(),
                usage: CommonAttributeContentUse::Use,
                conditional_separation: Some(
                    open_sdbl::metadata::CommonAttributeConditionalSeparation::Undefined,
                ),
                conditional_separation_guid: "00000000-0000-0000-0000-000000000000"
                    .parse()
                    .unwrap(),
            }],
        },
        CommonAttributeUsage {
            auto_use: CommonAttributeUse::Use,
            content: vec![CommonAttributeContentEntry {
                object: excluded.parse().unwrap(),
                usage: CommonAttributeContentUse::DontUse,
                conditional_separation: Some(
                    open_sdbl::metadata::CommonAttributeConditionalSeparation::Undefined,
                ),
                conditional_separation_guid: "00000000-0000-0000-0000-000000000000"
                    .parse()
                    .unwrap(),
            }],
        },
    ] {
        let catalog = project_usage(Some(usage)).unwrap();
        assert_eq!(
            catalog
                .object_by_id(included)
                .unwrap()
                .catalog_common_attributes()
                .unwrap()
                .len(),
            1
        );
        assert!(
            catalog
                .object_by_id(excluded)
                .unwrap()
                .catalog_common_attributes()
                .unwrap()
                .is_empty()
        );
    }
}

#[test]
fn unknown_common_usage_is_not_discarded_from_a_claimed_complete_collection() {
    let error = project_usage(None).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("неизвестно использование общего реквизита"),
        "{error}"
    );
}

#[test]
fn incomplete_root_common_collection_is_rejected_before_projection() {
    let mut resources = resources();
    let root = include_str!("../tests/fixtures/common-attributes/isolated-root.txt").replace(
        "15794563-ccec-41f6-a83c-ec5f7b9a5bc1,13,",
        "15794563-ccec-41f6-a83c-ec5f7b9a5bc1,14,",
    );
    resources[1] = text_resource("11111111-1111-1111-1111-111111111111", &root);
    let code = ConfigurationCodeSnapshot::new("common", vec![]).unwrap();
    let error = crate::load_catalog_base(&resources, &code, vec![]).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("неполная коллекция общих реквизитов"),
        "{error}"
    );
}

fn resources() -> Vec<open_sdbl_db::ConfigResource> {
    let root = include_str!("../tests/fixtures/common-attributes/isolated-root.txt");
    let mut result = vec![
        text_resource("root", "{1,{11111111-1111-1111-1111-111111111111}}"),
        text_resource("11111111-1111-1111-1111-111111111111", root),
    ];
    let directory =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/common-attributes");
    for id in include_str!("../tests/fixtures/common-attributes/demo-common-ids.txt")
        .lines()
        .chain(["9e9e2a57-8536-4983-a5c0-7841ead585e8"])
    {
        result.push(resource(
            id,
            &std::fs::read(directory.join(format!("{id}.deflate"))).unwrap(),
        ));
    }
    result
}

#[test]
fn native_demo_complete_common_collection_preserves_applied_string_qualifiers() {
    let resources = resources();
    let code = ConfigurationCodeSnapshot::new("common", vec![]).unwrap();
    let catalog = crate::load_catalog_base(&resources, &code, vec![]).unwrap();
    assert_eq!(catalog.common_attribute_ids().unwrap().len(), 13);
    let descriptors = resources
        .iter()
        .filter(|r| {
            r.file_name.len() == 36 && r.file_name != "11111111-1111-1111-1111-111111111111"
        })
        .flat_map(|r| parse_config_descriptors(&r.file_name, &r.compressed).unwrap())
        .collect();
    let snapshot = resolve_metadata(
        DbNames::from_entries(vec![]),
        descriptors,
        SchemaStorage {
            tables: vec![],
            anomalies: vec![],
        },
        vec![],
    )
    .snapshot;
    let catalog = crate::project_catalog_values(catalog, &snapshot).unwrap();
    let object = catalog
        .object_by_id("9e9e2a57-8536-4983-a5c0-7841ead585e8")
        .unwrap();
    let attribute = object
        .catalog_common_attributes()
        .unwrap()
        .iter()
        .find(|a| a.name() == "НаименованиеЯзык1")
        .unwrap();
    assert_eq!(attribute.id(), "03f9d2d0-eae4-4098-a70e-4eb569b11fd9");
    assert_eq!(attribute.types()[0].id, TypeRef::Native(TypeId::String));
    assert_eq!(attribute.types()[0].quals, ["150", "1"]);
    assert!(
        !object
            .catalog_attribute_names()
            .unwrap()
            .iter()
            .any(|name| name == "НаименованиеЯзык1")
    );
}

#[test]
fn native_demo_root_common_attributes_become_complete_metadata_objects() {
    let resources = resources();
    let code = ConfigurationCodeSnapshot::new("common-root", vec![]).unwrap();
    let catalog = crate::load_catalog_base(&resources, &code, vec![]).unwrap();
    let ids = catalog.common_attribute_ids().unwrap();
    assert_eq!(ids.len(), 13);
    let objects = catalog
        .objects()
        .iter()
        .filter(|object| object.kind() == bsl_orm::MetadataKind::CommonAttribute)
        .collect::<Vec<_>>();
    assert_eq!(
        objects.iter().map(|object| object.id()).collect::<Vec<_>>(),
        ids.iter().map(String::as_str).collect::<Vec<_>>()
    );
    assert_eq!(
        catalog
            .object_by_id("03f9d2d0-eae4-4098-a70e-4eb569b11fd9")
            .unwrap()
            .name(),
        "НаименованиеЯзык1"
    );
}

#[test]
fn native_common_attribute_auto_use_is_projected_from_the_sdk_descriptor() {
    let code = ConfigurationCodeSnapshot::new("native-auto", vec![]).unwrap();
    let catalog = crate::load_catalog_base(&resources(), &code, vec![]).unwrap();
    let engine = open_bsl::Engine::builder()
        .register_library(bsl_orm::library())
        .build()
        .unwrap();
    let mut state = engine
        .state_builder()
        .host_data(std::rc::Rc::new(bsl_orm::SessionData::new(
            std::sync::Arc::new(catalog),
            bsl_orm::Principal::new("user", "Проба", vec![]),
        )))
        .build();
    assert_eq!(state.exec("Возврат Метаданные.ОбщиеРеквизиты.НаименованиеЯзык1.AutoUse = Метаданные.СвойстваОбъектов.АвтоИспользованиеОбщегоРеквизита.НеИспользовать;").unwrap(), open_bsl::Value::Boolean(true));
}

#[test]
fn native_disabled_common_attribute_settings_do_not_establish_separate_mode() {
    let resources = resources();
    let id = "03f9d2d0-eae4-4098-a70e-4eb569b11fd9";
    let resource = resources
        .iter()
        .find(|resource| resource.file_name == id)
        .unwrap();
    let descriptor = parse_config_descriptors(id, &resource.compressed)
        .unwrap()
        .into_iter()
        .find(|descriptor| descriptor.object_guid.as_str() == id)
        .unwrap();
    // Нативный XML и контроль платформы задают DontUse, хотя SDK возвращает настройки.
    assert!(descriptor.separation.is_some());
    let code = ConfigurationCodeSnapshot::new("native-separation", vec![]).unwrap();
    let catalog = crate::load_catalog_base(&resources, &code, vec![]).unwrap();
    let engine = open_bsl::Engine::builder()
        .register_library(bsl_orm::library())
        .build()
        .unwrap();
    let mut state = engine
        .state_builder()
        .host_data(std::rc::Rc::new(bsl_orm::SessionData::new(
            std::sync::Arc::new(catalog),
            bsl_orm::Principal::new("user", "Проба", vec![]),
        )))
        .build();
    let error = state
        .exec("Возврат Метаданные.ОбщиеРеквизиты.НаименованиеЯзык1.DataSeparation;")
        .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("режим разделения общего реквизита неизвестен"),
        "{error}"
    );
}

#[test]
fn native_dbnames_classification_projects_both_common_attribute_modes() {
    let resources = resources();
    let descriptors = include_str!("../tests/fixtures/common-attributes/demo-common-ids.txt")
        .lines()
        .flat_map(|id| {
            let resource = resources
                .iter()
                .find(|resource| resource.file_name == id)
                .unwrap();
            parse_config_descriptors(id, &resource.compressed).unwrap()
        })
        .collect();
    let db_names = open_sdbl::metadata::parse_db_names(include_bytes!(
        "../tests/fixtures/common-attributes/demo-db-names-1009.deflate"
    ))
    .unwrap();
    let metadata = resolve_metadata(
        db_names,
        descriptors,
        SchemaStorage {
            tables: vec![],
            anomalies: vec![],
        },
        vec![],
    )
    .snapshot;
    let code = ConfigurationCodeSnapshot::new("native-physical-mode", vec![]).unwrap();
    let catalog =
        crate::load_catalog_base_with_metadata(&resources, &code, vec![], Some(&metadata)).unwrap();
    let observations = catalog
        .common_attribute_ids()
        .unwrap()
        .iter()
        .map(|id| {
            let object = catalog.object_by_id(id).unwrap();
            (
                object.name().to_owned(),
                matches!(
                    id.as_str(),
                    "530a3164-4ef1-4b3b-8269-13764ef4bf15" | "6df2bb92-558c-4453-9de4-e4176e8f93dc"
                ),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(observations.len(), 13);
    let engine = open_bsl::Engine::builder()
        .register_library(bsl_orm::library())
        .build()
        .unwrap();
    let mut state = engine
        .state_builder()
        .host_data(std::rc::Rc::new(bsl_orm::SessionData::new(
            std::sync::Arc::new(catalog),
            bsl_orm::Principal::new("user", "Проба", vec![]),
        )))
        .build();
    for (name, enabled) in observations {
        let member = if enabled {
            "Разделять"
        } else {
            "НеИспользовать"
        };
        assert_eq!(state.exec(&format!("Возврат Метаданные.ОбщиеРеквизиты.{name}.DataSeparation = Метаданные.СвойстваОбъектов.РазделениеДанныхОбщегоРеквизита.{member};")).unwrap(), open_bsl::Value::Boolean(true), "{name}");
    }
}

#[test]
fn missing_and_ambiguous_common_attribute_field_modes_remain_unknown() {
    let id = "03f9d2d0-eae4-4098-a70e-4eb569b11fd9";
    for entries in [
        vec![],
        (1..=2)
            .map(|number| open_sdbl::metadata::DbNameEntry {
                guid: id.parse().unwrap(),
                alias: "Fld".into(),
                number,
            })
            .collect(),
    ] {
        let metadata = resolve_metadata(
            DbNames::from_entries(entries),
            vec![],
            SchemaStorage {
                tables: vec![],
                anomalies: vec![],
            },
            vec![],
        )
        .snapshot;
        let code = ConfigurationCodeSnapshot::new("unknown-field-mode", vec![]).unwrap();
        let catalog =
            crate::load_catalog_base_with_metadata(&resources(), &code, vec![], Some(&metadata))
                .unwrap();
        let engine = open_bsl::Engine::builder()
            .register_library(bsl_orm::library())
            .build()
            .unwrap();
        let mut state = engine
            .state_builder()
            .host_data(std::rc::Rc::new(bsl_orm::SessionData::new(
                std::sync::Arc::new(catalog),
                bsl_orm::Principal::new("user", "Проба", vec![]),
            )))
            .build();
        let error = state
            .exec("Возврат Метаданные.ОбщиеРеквизиты.НаименованиеЯзык1.DataSeparation;")
            .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("режим разделения общего реквизита неизвестен"),
            "{error}"
        );
    }
}

#[test]
fn explicit_auto_content_preserves_the_acquired_auto_use_setting() {
    let included = "9e9e2a57-8536-4983-a5c0-7841ead585e8";
    for (auto_use, count) in [
        (CommonAttributeUse::Use, 1),
        (CommonAttributeUse::DontUse, 0),
    ] {
        let catalog = project_usage(Some(CommonAttributeUsage {
            auto_use,
            content: vec![CommonAttributeContentEntry {
                object: included.parse().unwrap(),
                usage: CommonAttributeContentUse::Auto,
                conditional_separation: Some(
                    open_sdbl::metadata::CommonAttributeConditionalSeparation::Undefined,
                ),
                conditional_separation_guid: "00000000-0000-0000-0000-000000000000"
                    .parse()
                    .unwrap(),
            }],
        }))
        .unwrap();
        assert_eq!(
            catalog
                .object_by_id(included)
                .unwrap()
                .catalog_common_attributes()
                .unwrap()
                .len(),
            count
        );
    }
}
