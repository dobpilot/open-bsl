use bsl_orm::{MetadataCatalog, MetadataKind, MetadataObject, Principal, SessionData, library};
use open_bsl::{BslObject, Engine, Value, format_value};
use std::{rc::Rc, sync::Arc};

fn state(catalog: MetadataCatalog) -> open_bsl::State {
    Engine::builder()
        .register_library(library())
        .build()
        .unwrap()
        .state_builder()
        .host_data(Rc::new(SessionData::new(
            Arc::new(catalog),
            Principal::new("user", "Проба", vec![]),
        )))
        .build()
}

#[test]
fn a_verified_empty_common_attribute_list_is_available_at_the_metadata_root() {
    let catalog = MetadataCatalog::new("empty", vec![])
        .unwrap()
        .with_common_attribute_ids(vec![]);
    assert_eq!(
        state(catalog)
            .exec("Возврат Метаданные.ОбщиеРеквизиты.Количество();")
            .unwrap(),
        Value::number_from_i64(0)
    );
}

#[test]
fn acquired_common_attribute_collection_replays_native_basic_operations() {
    let id = "03f9d2d0-eae4-4098-a70e-4eb569b11fd9";
    let catalog = MetadataCatalog::new(
        "native-common",
        vec![MetadataObject::new(
            id,
            "НаименованиеЯзык1",
            MetadataKind::CommonAttribute,
        )],
    )
    .unwrap()
    .with_common_attribute_ids(vec![id.into()]);
    let source = include_str!(
        "../../../../tests/conformance/measure/1c/measure-metadata-common-attributes.bsl"
    )
    .replace("Сообщить(", "РезультатыПробы.Добавить(");
    let result = state(catalog)
        .exec(&format!(
            "Перем РезультатыПробы; РезультатыПробы = Новый Массив; {source} Возврат РезультатыПробы;"
        ))
        .unwrap();
    let Value::Object(result) = result else {
        panic!("массив результатов")
    };
    let BslObject::Array(result) = result.as_ref() else {
        panic!("массив результатов")
    };
    let lines = result
        .borrow()
        .iter()
        .map(|value| format_value(value, None).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(lines.len(), 10);
    assert_eq!(
        lines.join("\n"),
        include_str!(
            "../../../../tests/conformance/measure/1c/measure-metadata-common-attributes.platform.txt"
        )
        .trim_end()
    );
}

#[test]
fn unknown_missing_and_wrong_kind_common_attribute_members_fail_explicitly() {
    let unknown = MetadataCatalog::new("unknown", vec![]).unwrap();
    let missing = MetadataCatalog::new("missing", vec![])
        .unwrap()
        .with_common_attribute_ids(vec!["absent".into()]);
    let wrong_kind = MetadataCatalog::new(
        "wrong-kind",
        vec![MetadataObject::new(
            "catalog",
            "СправочникПробы",
            MetadataKind::Catalog,
        )],
    )
    .unwrap()
    .with_common_attribute_ids(vec!["catalog".into()]);
    for catalog in [unknown, missing, wrong_kind] {
        let error = state(catalog)
            .exec("Возврат Метаданные.ОбщиеРеквизиты;")
            .unwrap_err();
        assert!(
            error.to_string().contains("состав общих реквизитов"),
            "{error}"
        );
    }
}

#[test]
fn common_attribute_iteration_uses_root_ids_instead_of_object_storage_order() {
    let catalog = MetadataCatalog::new(
        "ordered",
        vec![
            MetadataObject::new("a", "Альфа", MetadataKind::CommonAttribute),
            MetadataObject::new("b", "Бета", MetadataKind::CommonAttribute),
        ],
    )
    .unwrap()
    .with_common_attribute_ids(vec!["b".into(), "a".into()]);
    let result = state(catalog).exec(r#"Имена = ""; Для Каждого Элемент Из Метаданные.ОбщиеРеквизиты Цикл Имена = Имена + Элемент.Имя + ";"; КонецЦикла; Возврат Имена;"#).unwrap();
    assert_eq!(result, Value::Str("Бета;Альфа;".into()));
}

#[test]
fn common_attribute_property_enumerations_replay_native_identities_and_scope() {
    let source = include_str!(
        "../../../../tests/conformance/measure/1c/measure-common-attribute-property-enums.bsl"
    )
    .replace("Сообщить(", "РезультатыПробы.Добавить(");
    let result = state(MetadataCatalog::new("native-enums", vec![]).unwrap())
        .exec(&format!("Перем РезультатыПробы; РезультатыПробы = Новый Массив; {source} Возврат РезультатыПробы;"))
        .unwrap();
    let Value::Object(result) = result else {
        panic!("массив результатов")
    };
    let BslObject::Array(result) = result.as_ref() else {
        panic!("массив результатов")
    };
    let lines = result
        .borrow()
        .iter()
        .map(|value| format_value(value, None).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(lines.len(), 32);
    assert_eq!(lines.join("\n"), include_str!("../../../../tests/conformance/measure/1c/measure-common-attribute-property-enums.platform.txt").trim_end());
}

#[test]
fn unknown_common_attribute_automatic_use_is_not_a_disabled_setting() {
    let catalog = MetadataCatalog::new(
        "unknown-auto",
        vec![MetadataObject::new(
            "common",
            "Проба",
            MetadataKind::CommonAttribute,
        )],
    )
    .unwrap()
    .with_common_attribute_ids(vec!["common".into()]);
    let error = state(catalog)
        .exec("Возврат Метаданные.ОбщиеРеквизиты.Проба.АвтоИспользование;")
        .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("автоиспользование общего реквизита неизвестно"),
        "{error}"
    );
}

#[test]
fn known_common_attribute_automatic_use_preserves_both_native_identities() {
    for (enabled, member) in [(false, "НеИспользовать"), (true, "Использовать")]
    {
        let catalog = MetadataCatalog::new(
            "known-auto",
            vec![
                MetadataObject::new("common", "Проба", MetadataKind::CommonAttribute)
                    .with_common_attribute_auto_use(enabled),
            ],
        )
        .unwrap()
        .with_common_attribute_ids(vec!["common".into()]);
        let mut state = state(catalog);
        for alias in ["АвтоИспользование", "AutoUse", "аВтОиСпОлЬзОвАнИе"]
        {
            assert_eq!(state.exec(&format!("Возврат Метаданные.ОбщиеРеквизиты.Проба.{alias} = Метаданные.СвойстваОбъектов.АвтоИспользованиеОбщегоРеквизита.{member};")).unwrap(), Value::Boolean(true));
        }
        assert!(
            state
                .exec("Метаданные.ОбщиеРеквизиты.Проба.AutoUse = Неопределено;")
                .is_err()
        );
    }
}

#[test]
fn common_attribute_automatic_use_does_not_intercept_other_metadata_properties() {
    let catalog = MetadataCatalog::new(
        "other-auto",
        vec![
            MetadataObject::new("catalog", "Проба", MetadataKind::Catalog)
                .with_string_property("AutoUse", "english")
                .with_string_property("АвтоИспользование", "russian")
                .with_string_property("DataSeparation", "custom-mode"),
        ],
    )
    .unwrap();
    let mut state = state(catalog);
    assert_eq!(
        state
            .exec("Возврат Метаданные.Справочники.Проба.AutoUse;")
            .unwrap(),
        Value::Str("english".into())
    );
    assert_eq!(
        state
            .exec("Возврат Метаданные.Справочники.Проба.АвтоИспользование;")
            .unwrap(),
        Value::Str("russian".into())
    );
    assert_eq!(
        state
            .exec("Возврат Метаданные.Справочники.Проба.DataSeparation;")
            .unwrap(),
        Value::Str("custom-mode".into())
    );
}

#[test]
fn unknown_common_attribute_separation_is_not_a_disabled_mode() {
    let catalog = MetadataCatalog::new(
        "unknown-separation",
        vec![MetadataObject::new(
            "common",
            "Проба",
            MetadataKind::CommonAttribute,
        )],
    )
    .unwrap()
    .with_common_attribute_ids(vec!["common".into()]);
    let error = state(catalog)
        .exec("Возврат Метаданные.ОбщиеРеквизиты.Проба.РазделениеДанных;")
        .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("режим разделения общего реквизита неизвестен"),
        "{error}"
    );
}

#[test]
fn explicit_common_attribute_separation_preserves_native_modes_and_is_read_only() {
    for (enabled, member) in [(false, "НеИспользовать"), (true, "Разделять")]
    {
        let catalog = MetadataCatalog::new(
            "known-separation",
            vec![
                MetadataObject::new("common", "Проба", MetadataKind::CommonAttribute)
                    .with_common_attribute_data_separation(enabled),
            ],
        )
        .unwrap()
        .with_common_attribute_ids(vec!["common".into()]);
        let mut state = state(catalog);
        for alias in ["РазделениеДанных", "DataSeparation", "рАзДеЛеНиЕдАнНыХ"]
        {
            assert_eq!(state.exec(&format!("Возврат Метаданные.ОбщиеРеквизиты.Проба.{alias} = Метаданные.СвойстваОбъектов.РазделениеДанныхОбщегоРеквизита.{member};")).unwrap(), Value::Boolean(true));
        }
        assert!(
            state
                .exec("Метаданные.ОбщиеРеквизиты.Проба.DataSeparation = Неопределено;")
                .is_err()
        );
    }
}

#[test]
fn metadata_full_names_replay_both_native_script_variants() {
    for (variant, source, expected) in [
        (
            bsl_orm::ScriptVariant::English,
            include_str!("../../../../tests/conformance/measure/1c/measure-metadata-full-name.bsl"),
            include_str!(
                "../../../../tests/conformance/measure/1c/measure-metadata-full-name.platform.txt"
            ),
        ),
        (
            bsl_orm::ScriptVariant::Russian,
            include_str!(
                "../../../../tests/conformance/measure/1c/measure-metadata-full-name-russian-script.bsl"
            ),
            include_str!(
                "../../../../tests/conformance/measure/1c/measure-metadata-full-name-russian-script.platform.txt"
            ),
        ),
    ] {
        let catalog = MetadataCatalog::new(
            "full-names",
            vec![
                MetadataObject::new("common", "НаименованиеЯзык1", MetadataKind::CommonAttribute),
                MetadataObject::new("catalog", "ЗамерВременныхТаблиц", MetadataKind::Catalog),
                MetadataObject::new("owner", "ВладелецПробы", MetadataKind::Catalog),
            ],
        )
        .unwrap()
        .with_common_attribute_ids(vec!["common".into()])
        .with_configuration(
            "Проба",
            "",
            variant,
            bsl_orm::CompatibilityMode::Version8_3_24,
            [],
        )
        .unwrap();
        let source = source.replace("Сообщить(", "РезультатыПробы.Добавить(");
        let result = state(catalog)
            .exec(&format!("Перем РезультатыПробы; РезультатыПробы = Новый Массив; {source} Возврат РезультатыПробы;"))
            .unwrap();
        let Value::Object(result) = result else {
            panic!("массив результатов")
        };
        let BslObject::Array(result) = result.as_ref() else {
            panic!("массив результатов")
        };
        let lines = result
            .borrow()
            .iter()
            .map(|value| format_value(value, None).unwrap())
            .collect::<Vec<_>>();
        assert_eq!(lines.len(), 8);
        // Представление варианта языка зависит от языка нативного сеанса;
        // этот контроль сравнивает полные имена и границы результата.
        let full_names = |text: &str| {
            text.lines()
                .filter(|line| !line.starts_with("METADATA_FULL_NAME.SCRIPT_VARIANT\t"))
                .collect::<Vec<_>>()
                .join("\n")
        };
        assert_eq!(full_names(&lines.join("\n")), full_names(expected));
    }
}

#[test]
fn metadata_full_name_does_not_guess_an_unknown_script_variant() {
    let catalog = MetadataCatalog::new(
        "unknown-language",
        vec![MetadataObject::new(
            "catalog",
            "Проба",
            MetadataKind::Catalog,
        )],
    )
    .unwrap();
    let error = state(catalog)
        .exec("Возврат Метаданные.Справочники.Проба.ПолноеИмя();")
        .unwrap_err();
    assert!(
        error.to_string().contains("вариант встроенного языка"),
        "{error}"
    );
}

#[test]
fn metadata_full_name_rejects_an_unimplemented_kind() {
    let catalog = MetadataCatalog::new(
        "unsupported-kind",
        vec![MetadataObject::new("report", "Проба", MetadataKind::Report)],
    )
    .unwrap()
    .with_configuration(
        "Проба",
        "",
        bsl_orm::ScriptVariant::Russian,
        bsl_orm::CompatibilityMode::Version8_3_24,
        [],
    )
    .unwrap();
    let error = state(catalog)
        .exec("Возврат Метаданные.Отчеты.Проба.FullName();")
        .unwrap_err();
    assert!(
        error.to_string().contains("пока не поддерживается"),
        "{error}"
    );
}

#[test]
fn constant_and_option_full_names_replay_both_native_script_variants() {
    for (variant, source, expected) in [
        (
            bsl_orm::ScriptVariant::English,
            include_str!(
                "../../../../tests/conformance/measure/1c/measure-metadata-option-full-name.bsl"
            ),
            include_str!(
                "../../../../tests/conformance/measure/1c/measure-metadata-option-full-name.platform.txt"
            ),
        ),
        (
            bsl_orm::ScriptVariant::Russian,
            include_str!(
                "../../../../tests/conformance/measure/1c/measure-metadata-option-full-name-russian-script.bsl"
            ),
            include_str!(
                "../../../../tests/conformance/measure/1c/measure-metadata-option-full-name-russian-script.platform.txt"
            ),
        ),
    ] {
        let catalog = MetadataCatalog::new(
            "constant-option-full-names",
            vec![
                MetadataObject::new("constant", "ЗначениеОпцииПробы", MetadataKind::Constant),
                MetadataObject::new("option", "ОпцияПробы", MetadataKind::FunctionalOption),
            ],
        )
        .unwrap()
        .with_configuration(
            "Проба",
            "",
            variant,
            bsl_orm::CompatibilityMode::Version8_3_24,
            [],
        )
        .unwrap();
        let source = source.replace("Сообщить(", "РезультатыПробы.Добавить(");
        let result = state(catalog)
            .exec(&format!("Перем РезультатыПробы; РезультатыПробы = Новый Массив; {source} Возврат РезультатыПробы;"))
            .unwrap();
        let Value::Object(result) = result else {
            panic!("массив результатов")
        };
        let BslObject::Array(result) = result.as_ref() else {
            panic!("массив результатов")
        };
        let lines = result
            .borrow()
            .iter()
            .map(|value| format_value(value, None).unwrap())
            .collect::<Vec<_>>();
        assert_eq!(lines.len(), 6);
        assert_eq!(lines.join("\n"), expected.trim_end());
    }
}
