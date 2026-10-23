use super::*;
use crate::functional_options::{project, tests::native};
use bsl_orm::{FunctionalOptionLocation, MetadataKind};
use open_bsl::Value;
use std::sync::Arc;

#[test]
fn catalog_boolean_partial_read_uses_a_single_matching_true_row() {
    let (catalog, metadata) = native("catalog-native", |_| {});
    let catalog = project(catalog, &metadata).unwrap();
    let option = catalog
        .object(MetadataKind::FunctionalOption, "ОпцияСправочника")
        .unwrap();
    let plan = prepare(
        &metadata,
        &catalog,
        option.functional_option_settings().unwrap(),
        &[],
    )
    .unwrap();
    assert!(!plan.complete);
    assert!(plan.query.contains("О.ОпцияБулево = ИСТИНА"));
    assert!(plan.query.contains("НЕ О.ЭтоГруппа"));
    assert!(plan.parameters.is_empty());
    assert_eq!(plan.value(&[]), Value::Boolean(false));
    assert_eq!(
        plan.value(&[vec![Value::Boolean(true)].into()]),
        Value::Boolean(true)
    );
}

#[test]
fn numeric_catalog_option_refuses_partial_parameters_before_reading() {
    let (catalog, metadata) = native("catalog-native", |_| {});
    let catalog = project(catalog, &metadata).unwrap();
    let option = catalog
        .object(MetadataKind::FunctionalOption, "ЧисловаяОпцияСправочника")
        .unwrap();
    assert!(
        prepare(
            &metadata,
            &catalog,
            option.functional_option_settings().unwrap(),
            &[]
        )
        .is_err()
    );
}

#[test]
fn a_full_catalog_reference_retains_group_null_and_empty_result_false() {
    let (catalog, metadata) = native("catalog-native", |_| {});
    let catalog = Arc::new(project(catalog, &metadata).unwrap());
    let option = catalog
        .object(MetadataKind::FunctionalOption, "ЧисловаяОпцияСправочника")
        .unwrap();
    let settings = option.functional_option_settings().unwrap();
    let Some(FunctionalOptionLocation::CatalogAttribute { owner, .. }) = &settings.location else {
        panic!("источник должен быть реквизитом");
    };
    let reference = bsl_orm::catalog_reference_value(Arc::clone(&catalog), owner, [0; 16]).unwrap();
    let plan = prepare(
        &metadata,
        &catalog,
        settings,
        &[("объектопции", &reference)],
    )
    .unwrap();
    assert!(plan.complete);
    assert_eq!(plan.parameters.len(), 1);
    assert!(
        plan.query
            .contains("ВЫБОР КОГДА НЕ О.ЭтоГруппа ТОГДА О.ЧислоПробы ИНАЧЕ NULL")
    );
    assert_eq!(plan.value(&[]), Value::Boolean(false));
    assert_eq!(plan.value(&[vec![Value::Null].into()]), Value::Null);
    let number = Value::Number(open_bsl::BslNumber::from_i64(20));
    assert_eq!(plan.value(&[vec![number.clone()].into()]), number);
}

#[test]
fn the_register_uses_all_acquired_dimensions_to_establish_completeness() {
    let (catalog, metadata) = native("register-native", |_| {});
    let catalog = project(catalog, &metadata).unwrap();
    let option = catalog
        .object(MetadataKind::FunctionalOption, "ОпцияРегистра")
        .unwrap();
    let one = Value::Number(open_bsl::BslNumber::from_i64(1));
    let partial = prepare(
        &metadata,
        &catalog,
        option.functional_option_settings().unwrap(),
        &[("ПараметрПервый", &one)],
    )
    .unwrap();
    assert!(!partial.complete);
    assert_eq!(partial.parameters.len(), 1);
    assert!(partial.query.contains("О.ПервыйКлюч = &ПараметрОпции0"));
    assert!(partial.query.contains("О.ПризнакОпции = ИСТИНА"));
    let full = prepare(
        &metadata,
        &catalog,
        option.functional_option_settings().unwrap(),
        &[("ПараметрПервый", &one), ("ПараметрВторой", &one)],
    )
    .unwrap();
    assert!(full.complete);
    assert_eq!(full.parameters.len(), 2);
    assert_eq!(
        full.value(&[vec![Value::Boolean(false)].into()]),
        Value::Boolean(false)
    );
}

#[test]
fn numeric_register_partial_read_is_an_error_even_with_one_matching_row() {
    let (catalog, metadata) = native("register-native", |_| {});
    let catalog = project(catalog, &metadata).unwrap();
    let option = catalog
        .object(MetadataKind::FunctionalOption, "ЧисловаяОпцияРегистра")
        .unwrap();
    let one = Value::Number(open_bsl::BslNumber::from_i64(1));
    assert!(
        prepare(
            &metadata,
            &catalog,
            option.functional_option_settings().unwrap(),
            &[("ПараметрПервый", &one)]
        )
        .is_err()
    );
    let full = prepare(
        &metadata,
        &catalog,
        option.functional_option_settings().unwrap(),
        &[("ПараметрПервый", &one), ("ПараметрВторой", &one)],
    )
    .unwrap();
    assert_eq!(full.value(&[]), Value::Boolean(false));
}

#[test]
fn unknown_register_profile_or_collections_cannot_return_false_successfully() {
    for missing in 0..4 {
        let (catalog, metadata) = native("register-native", |descriptors| {
            let owner = descriptors
                .iter_mut()
                .find(|descriptor| {
                    descriptor.object_guid.as_str() == "c34d5785-6f04-45c7-9555-088544ddc489"
                })
                .unwrap();
            match missing {
                0 => owner.information_register_write_mode = None,
                1 => owner.information_register_periodicity = None,
                2 => owner.information_register_dimensions = None,
                3 => owner.information_register_resources = None,
                _ => unreachable!(),
            }
        });
        let catalog = project(catalog, &metadata).unwrap();
        let option = catalog
            .object(MetadataKind::FunctionalOption, "ОпцияРегистра")
            .unwrap();
        assert!(
            prepare(
                &metadata,
                &catalog,
                option.functional_option_settings().unwrap(),
                &[]
            )
            .is_err(),
            "unknown field {missing}"
        );
    }
}

#[test]
fn unknown_catalog_attribute_use_cannot_be_guessed() {
    let (catalog, metadata) = native("catalog-native", |descriptors| {
        for descriptor in descriptors {
            descriptor.catalog_attribute_use = None;
        }
    });
    let catalog = project(catalog, &metadata).unwrap();
    let option = catalog
        .object(MetadataKind::FunctionalOption, "ОпцияСправочника")
        .unwrap();
    assert!(
        prepare(
            &metadata,
            &catalog,
            option.functional_option_settings().unwrap(),
            &[]
        )
        .is_err()
    );
}

#[test]
fn an_unrelated_option_parameter_is_ignored_without_value_conversion() {
    let (catalog, metadata) = native("catalog-native", |_| {});
    let catalog = project(catalog, &metadata).unwrap();
    let option = catalog
        .object(MetadataKind::FunctionalOption, "ОпцияСправочника")
        .unwrap();
    let settings = option.functional_option_settings().unwrap();
    let empty = prepare(&metadata, &catalog, settings, &[]).unwrap();
    let unrelated = prepare(
        &metadata,
        &catalog,
        settings,
        &[("НеизвестныйПараметр", &Value::Undefined)],
    )
    .unwrap();
    assert_eq!(empty.query, unrelated.query);
    assert!(unrelated.parameters.is_empty());
    assert!(!unrelated.complete);
}
