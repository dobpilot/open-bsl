use super::*;
use bsl_orm::{FunctionalOptionLocation as Location, MetadataObject};
use open_sdbl::metadata::{
    DbNameEntry, DbNames, SchemaStorage, parse_config_descriptors, resolve_metadata,
};

pub(crate) fn native(
    profile: &str,
    mutate: impl FnOnce(&mut Vec<ConfigDescriptor>),
) -> (MetadataCatalog, MetadataSnapshot) {
    let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/functional-options")
        .join(profile);
    let mut descriptors = std::fs::read_to_string(base.join("resources.txt"))
        .unwrap()
        .lines()
        .flat_map(|id| {
            parse_config_descriptors(
                id,
                &std::fs::read(base.join(format!("{id}.deflate"))).unwrap(),
            )
            .unwrap()
        })
        .collect::<Vec<_>>();
    let identity = descriptors
        .iter()
        .find(|descriptor| {
            descriptor
                .configuration_functional_option_parameters
                .is_some()
        })
        .unwrap()
        .object_guid
        .as_str()
        .to_owned();
    let entries = if profile == "native" {
        vec![DbNameEntry {
            guid: "9893e2d6-f3f8-4d73-bb06-19bf26d216ab".parse().unwrap(),
            alias: "Const".into(),
            number: 1,
        }]
    } else {
        vec![]
    };
    let names = DbNames::from_entries(entries);
    let metadata = resolve_metadata(
        names.clone(),
        descriptors.clone(),
        SchemaStorage {
            tables: vec![],
            anomalies: vec![],
        },
        vec![],
    )
    .snapshot;
    let mut objects = Vec::<MetadataObject>::new();
    for descriptor in metadata
        .descriptors()
        .iter()
        .filter(|descriptor| descriptor.functional_option_location_guid.is_some())
    {
        objects.push(MetadataObject::new(
            descriptor.object_guid.as_str(),
            descriptor.name.as_str(),
            MetadataKind::FunctionalOption,
        ));
        let (owner, kind) = match descriptor.functional_option_location.as_ref().unwrap() {
            SourceLocation::Constant(id) => (id, MetadataKind::Constant),
            SourceLocation::CatalogAttribute { owner, .. } => (owner, MetadataKind::Catalog),
            SourceLocation::InformationRegisterResource { owner, .. } => {
                (owner, MetadataKind::InformationRegister)
            }
            SourceLocation::Absent => continue,
        };
        if !objects.iter().any(|object| object.id() == owner.as_str()) {
            let source = metadata
                .descriptors()
                .iter()
                .find(|source| &source.object_guid == owner)
                .unwrap();
            objects.push(MetadataObject::new(
                owner.as_str(),
                source.name.as_str(),
                kind,
            ));
        }
    }
    let catalog = MetadataCatalog::new(identity, objects).unwrap();
    mutate(&mut descriptors);
    let metadata = resolve_metadata(
        names,
        descriptors,
        SchemaStorage {
            tables: vec![],
            anomalies: vec![],
        },
        vec![],
    )
    .snapshot;
    (catalog, metadata)
}

#[test]
fn native_sources_and_complete_bindings_are_projected_without_config_decoding() {
    for (profile, count) in [("native", 0), ("catalog-native", 1), ("register-native", 2)] {
        let (catalog, metadata) = native(profile, |_| {});
        let catalog = project(catalog, &metadata).unwrap();
        let options = catalog
            .objects()
            .iter()
            .filter(|object| object.kind() == MetadataKind::FunctionalOption)
            .collect::<Vec<_>>();
        assert_eq!(options.len(), 2);
        for option in options {
            let settings = option.functional_option_settings().unwrap();
            assert!(settings.privileged_get_mode.is_some());
            let parameters = settings.parameters.as_ref().unwrap();
            assert_eq!(parameters.len(), count, "{profile}");
            let descriptor = metadata
                .descriptors()
                .iter()
                .find(|descriptor| descriptor.object_guid.as_str() == option.id())
                .unwrap();
            let expected_flag = descriptor.functional_option_privileged_get_mode.unwrap();
            assert_eq!(settings.privileged_get_mode, Some(expected_flag));
            match (&settings.location, profile) {
                (Some(Location::Constant(id)), "native") => {
                    assert_eq!(id.as_ref(), "9893e2d6-f3f8-4d73-bb06-19bf26d216ab")
                }
                (Some(Location::CatalogAttribute { owner, .. }), "catalog-native") => {
                    assert_eq!(owner.as_ref(), "a7177622-2cb3-5bfd-a0d7-1ae7a4a9f4d5");
                    assert_eq!(parameters[0].name.as_ref(), "ОбъектОпции");
                    assert_eq!(&parameters[0].target_id, owner);
                }
                (Some(Location::InformationRegisterResource { owner, .. }), "register-native") => {
                    assert_eq!(owner.as_ref(), "c34d5785-6f04-45c7-9555-088544ddc489");
                    assert_eq!(
                        parameters
                            .iter()
                            .map(|parameter| parameter.name.as_ref())
                            .collect::<Vec<_>>(),
                        ["ПараметрПервый", "ПараметрВторой"]
                    );
                    for parameter in parameters {
                        let dimension = metadata
                            .descriptors()
                            .iter()
                            .find(|descriptor| {
                                descriptor.object_guid.as_str() == parameter.target_id.as_ref()
                            })
                            .unwrap();
                        assert_eq!(dimension.resource_guid.as_str(), owner.as_ref());
                        assert_eq!(
                            dimension.field_purpose,
                            Some(ConfigFieldPurpose::InformationRegisterDimension)
                        );
                    }
                }
                source => panic!("неожиданный нативный источник {source:?}"),
            }
        }
    }
}

#[test]
fn an_unknown_parameter_use_keeps_the_whole_binding_list_unknown() {
    let (catalog, metadata) = native("register-native", |descriptors| {
        let parameter = descriptors
            .iter_mut()
            .find(|descriptor| descriptor.name == "ПараметрВторой")
            .unwrap();
        parameter.functional_option_parameter_use = None;
    });
    let catalog = project(catalog, &metadata).unwrap();
    for option in catalog
        .objects()
        .iter()
        .filter(|object| object.kind() == MetadataKind::FunctionalOption)
    {
        assert!(
            option
                .functional_option_settings()
                .unwrap()
                .parameters
                .is_none()
        );
    }
}

#[test]
fn a_known_source_cannot_target_an_owner_of_the_wrong_kind() {
    let (catalog, metadata) = native("native", |_| {});
    let objects = catalog
        .objects()
        .iter()
        .map(|object| {
            if object.kind() == MetadataKind::Constant {
                MetadataObject::new(object.id(), object.name(), MetadataKind::Catalog)
            } else {
                object.clone()
            }
        })
        .collect();
    let catalog = MetadataCatalog::new(catalog.identity(), objects).unwrap();
    assert!(
        project(catalog, &metadata)
            .unwrap_err()
            .to_string()
            .contains("неверного владельца")
    );
}

#[test]
fn missing_root_membership_does_not_publish_source_bindings_as_empty() {
    let (catalog, metadata) = native("catalog-native", |descriptors| {
        let root = descriptors
            .iter_mut()
            .find(|descriptor| {
                descriptor
                    .configuration_functional_option_parameters
                    .is_some()
            })
            .unwrap();
        root.configuration_functional_option_parameters = None;
    });
    let catalog = project(catalog, &metadata).unwrap();
    for option in catalog
        .objects()
        .iter()
        .filter(|object| object.kind() == MetadataKind::FunctionalOption)
    {
        let settings = option.functional_option_settings().unwrap();
        assert!(settings.location.is_some());
        assert!(settings.parameters.is_none());
    }
}

#[test]
fn unknown_source_and_known_absence_remain_distinct() {
    for absent in [false, true] {
        let (catalog, metadata) = native("native", |descriptors| {
            let option = descriptors
                .iter_mut()
                .find(|descriptor| descriptor.functional_option_privileged_get_mode == Some(false))
                .unwrap();
            option.functional_option_location_guid =
                absent.then(|| "00000000-0000-0000-0000-000000000000".parse().unwrap());
            option.functional_option_location = None;
        });
        let catalog = project(catalog, &metadata).unwrap();
        let settings = catalog
            .object(MetadataKind::FunctionalOption, "ОпцияПробы")
            .unwrap()
            .functional_option_settings()
            .unwrap();
        assert_eq!(settings.privileged_get_mode, Some(false));
        if absent {
            assert_eq!(settings.location, Some(Location::Absent));
            assert!(settings.parameters.as_ref().unwrap().is_empty());
        } else {
            assert!(settings.location.is_none());
            assert!(settings.parameters.is_none());
        }
    }
}
