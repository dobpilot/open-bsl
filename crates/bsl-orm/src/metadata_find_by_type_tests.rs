use super::*;

fn state() -> open_bsl::State {
    let catalog = Arc::new(
        MetadataCatalog::new(
            "find-types",
            vec![
                MetadataObject::new("enum", "EnumLegacyProbe", MetadataKind::Enumeration),
                MetadataObject::new("catalog", "ВладелецПробы", MetadataKind::Catalog),
            ],
        )
        .unwrap(),
    );
    let mut builder = open_bsl::Engine::builder().register_library(library());
    for ty in catalog.transferable_value_types() {
        builder = builder.register_value_type(ty);
    }
    for ty in catalog
        .catalog_object_types()
        .chain(catalog.catalog_selection_types())
    {
        builder = builder.register_value_type(Arc::clone(ty));
    }
    builder
        .build()
        .unwrap()
        .state_builder()
        .host_data(Rc::new(SessionData::new(
            catalog,
            Principal::new("user", "Тест", vec![]),
        )))
        .build()
}

#[test]
fn metadata_find_by_type_reuses_acquired_owner_identity() {
    let mut state = state();
    for (name, owner) in [
        ("EnumRef.EnumLegacyProbe", "Перечисления.EnumLegacyProbe"),
        ("CatalogRef.ВладелецПробы", "Справочники.ВладелецПробы"),
        ("CatalogObject.ВладелецПробы", "Справочники.ВладелецПробы"),
        (
            "CatalogSelection.ВладелецПробы",
            "Справочники.ВладелецПробы",
        ),
    ] {
        for method in ["НайтиПоТипу", "fInDbYtYpE"] {
            assert_eq!(
                state
                    .exec(&format!(
                        r#"Возврат Метаданные.{method}(Тип("{name}")) = Метаданные.{owner};"#
                    ))
                    .unwrap(),
                Value::Boolean(true),
                "{method}: {name}"
            );
        }
    }
}

#[test]
fn metadata_find_by_type_distinguishes_primitive_and_invalid_arguments() {
    let mut state = state();
    for name in [
        "Число",
        "Строка",
        "Булево",
        "Дата",
        "Массив",
        "УникальныйИдентификатор",
    ] {
        assert_eq!(
            state
                .exec(&format!(
                    r#"Возврат Метаданные.НайтиПоТипу(Тип("{name}"));"#
                ))
                .unwrap(),
            Value::Undefined
        );
    }
    let error = state
        .exec("Возврат Метаданные.НайтиПоТипу(42);")
        .unwrap_err();
    assert!(
        matches!(error, open_bsl::Error::Runtime(RtError::TypeError { .. })),
        "{error}"
    );
}

#[test]
fn an_owned_type_without_a_supported_owner_mapping_is_not_reported_as_absent() {
    let error = state()
        .exec("Возврат Метаданные.НайтиПоТипу(ТипЗнч(Справочники.ВладелецПробы));")
        .unwrap_err();
    assert!(
        matches!(error, open_bsl::Error::Runtime(RtError::DynamicError(ref text)) if text.contains("не поддерживается"))
    );
}
