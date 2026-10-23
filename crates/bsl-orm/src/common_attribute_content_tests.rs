use super::*;

fn catalog() -> MetadataCatalog {
    MetadataCatalog::new(
        "content",
        vec![
            MetadataObject::new("common", "НаименованиеЯзык1", MetadataKind::CommonAttribute),
            MetadataObject::new("owner", "ВладелецПробы", MetadataKind::Catalog),
            MetadataObject::new("condition", "ЗначениеОпцииПробы", MetadataKind::Constant),
            MetadataObject::new("target", "ЗамерВременныхТаблиц", MetadataKind::Catalog),
        ],
    )
    .unwrap()
    .with_common_attribute_ids(vec!["common".into()])
}

fn entries() -> Vec<CommonAttributeContentEntry> {
    vec![
        CommonAttributeContentEntry {
            metadata_id: "owner".into(),
            usage: EnumValue::CommonAttributeUseAuto,
            conditional_separation: Some(CommonAttributeCondition::Undefined),
        },
        CommonAttributeContentEntry {
            metadata_id: "condition".into(),
            usage: EnumValue::CommonAttributeUseAuto,
            conditional_separation: Some(CommonAttributeCondition::Undefined),
        },
        CommonAttributeContentEntry {
            metadata_id: "target".into(),
            usage: EnumValue::CommonAttributeUseUse,
            conditional_separation: Some(CommonAttributeCondition::Constant("condition".into())),
        },
    ]
}

fn state(catalog: MetadataCatalog) -> open_bsl::State {
    let engine = open_bsl::Engine::builder()
        .register_library(library())
        .build()
        .unwrap();
    engine
        .state_builder()
        .host_data(Rc::new(SessionData::new(
            Arc::new(catalog),
            Principal::new("user", "Проба", vec![]),
        )))
        .build()
}

#[test]
fn complete_content_is_visible_through_the_metadata_property() {
    let mut state = state(
        catalog()
            .with_common_attribute_content("common", entries())
            .unwrap(),
    );
    assert_eq!(
        state
            .exec("Возврат Метаданные.ОбщиеРеквизиты.НаименованиеЯзык1.Состав.Количество();")
            .unwrap(),
        Value::number_from_i64(3)
    );
}

#[test]
fn content_preserves_native_lookup_iteration_and_condition_identity() {
    let mut state = state(
        catalog()
            .with_common_attribute_content("common", entries())
            .unwrap(),
    );
    assert_eq!(state.exec(r#"
        СоставПробы = Метаданные.ОбщиеРеквизиты.НаименованиеЯзык1.Состав;
        Найденный = СоставПробы.Найти("замервременныхтаблиц");
        Если Найденный.Метаданные <> Метаданные.Справочники.ЗамерВременныхТаблиц Тогда Возврат Ложь; КонецЕсли;
        Если Найденный.УсловноеРазделение <> Метаданные.Константы.ЗначениеОпцииПробы Тогда Возврат Ложь; КонецЕсли;
        Если СоставПробы.Найти("Справочник.ЗамерВременныхТаблиц") <> Неопределено Тогда Возврат Ложь; КонецЕсли;
        Если СоставПробы.Содержит("ЗамерВременныхТаблиц") Тогда Возврат Ложь; КонецЕсли;
        Если Не СоставПробы.Содержит(Найденный.Метаданные) Тогда Возврат Ложь; КонецЕсли;
        Если ТипЗнч(СоставПробы) <> Тип("CommonAttributeContent") Тогда Возврат Ложь; КонецЕсли;
        Если ТипЗнч(Найденный) <> Тип("CommonAttributeContentItem") Тогда Возврат Ложь; КонецЕсли;
        СчетчикПробы = 0;
        Для Каждого ЭлементПробы Из СоставПробы Цикл
            Если ЭлементПробы <> СоставПробы.Найти(ЭлементПробы.Метаданные) Тогда Возврат Ложь; КонецЕсли;
            СчетчикПробы = СчетчикПробы + 1;
        КонецЦикла;
        Возврат СчетчикПробы = 3;
    "#).unwrap(), Value::Boolean(true));
}

#[test]
fn unknown_content_is_distinct_from_a_known_empty_content() {
    let mut unknown = state(catalog());
    assert!(
        unknown
            .exec("Возврат Метаданные.ОбщиеРеквизиты.НаименованиеЯзык1.Состав;")
            .is_err()
    );
    let mut empty = state(
        catalog()
            .with_common_attribute_content("common", vec![])
            .unwrap(),
    );
    assert_eq!(
        empty
            .exec("Возврат Метаданные.ОбщиеРеквизиты.НаименованиеЯзык1.Состав.Количество();")
            .unwrap(),
        Value::number_from_i64(0)
    );
}

#[test]
fn content_rejects_indexing_invalid_arguments_and_property_writes() {
    let mut state = state(
        catalog()
            .with_common_attribute_content("common", entries())
            .unwrap(),
    );
    for expression in [
        "СоставПробы[0]",
        "СоставПробы.Получить(0)",
        "СоставПробы.Найти()",
        "СоставПробы.Найти(1)",
        "СоставПробы.Содержит()",
        "СоставПробы.Содержит(Ложь)",
        "СоставПробы.Найти(Неопределено)",
        "СоставПробы.Содержит(Неопределено)",
        "СоставПробы.Найти(\"x\", \"y\")",
    ] {
        assert!(state.exec(&format!("СоставПробы = Метаданные.ОбщиеРеквизиты.НаименованиеЯзык1.Состав; Возврат {expression};")).is_err(), "{expression}");
    }
    assert!(state.exec("Метаданные.ОбщиеРеквизиты.НаименованиеЯзык1.Состав.Найти(\"ЗамерВременныхТаблиц\").Использование = Неопределено;").is_err());
}

#[test]
fn unknown_item_condition_does_not_erase_known_membership() {
    let mut content = entries();
    content[2].conditional_separation = None;
    let mut state = state(
        catalog()
            .with_common_attribute_content("common", content)
            .unwrap(),
    );
    assert_eq!(
        state
            .exec("Возврат Метаданные.ОбщиеРеквизиты.НаименованиеЯзык1.Состав.Количество();")
            .unwrap(),
        Value::number_from_i64(3)
    );
    assert!(state.exec("Возврат Метаданные.ОбщиеРеквизиты.НаименованиеЯзык1.Состав.Найти(\"ЗамерВременныхТаблиц\").УсловноеРазделение;").is_err());
}

#[test]
fn host_content_construction_checks_member_and_condition_targets() {
    let mut missing = entries();
    missing[0].metadata_id = "missing".into();
    assert!(
        catalog()
            .with_common_attribute_content("common", missing)
            .is_err()
    );
    let mut wrong_condition = entries();
    wrong_condition[2].conditional_separation =
        Some(CommonAttributeCondition::Constant("owner".into()));
    assert!(
        catalog()
            .with_common_attribute_content("common", wrong_condition)
            .is_err()
    );
    assert!(
        catalog()
            .with_common_attribute_content("owner", entries())
            .is_err()
    );
}

#[test]
fn native_content_surface_replays_without_rewriting_the_oracle() {
    let catalog = catalog()
        .with_common_attribute_content("common", entries())
        .unwrap()
        .with_configuration(
            "Замер",
            "1",
            ScriptVariant::English,
            CompatibilityMode::Version8_3_24,
            [],
        )
        .unwrap();
    let mut state = state(catalog);
    let source = include_str!(
        "../../../tests/conformance/measure/1c/measure-common-attribute-content-surface.bsl"
    )
    .replace("Сообщить(", "РезультатыПробы.Добавить(");
    let source = format!(
        "Перем РезультатыПробы; РезультатыПробы = Новый Массив; {source} Возврат СтрСоединить(РезультатыПробы, Символы.ПС);"
    );
    assert_eq!(state.exec(&source).unwrap(), Value::Str(include_str!("../../../tests/conformance/measure/1c/measure-common-attribute-content-surface.platform.txt").trim_end().replace("\tYes", "\tДа").replace("\tNo", "\tНет").as_str().into()));
}

#[test]
fn native_content_lookup_types_and_identity_replay() {
    let mut state = state(
        catalog()
            .with_common_attribute_content("common", entries())
            .unwrap(),
    );
    let source = include_str!(
        "../../../tests/conformance/measure/1c/measure-common-attribute-content-lookup.bsl"
    )
    .replace("Сообщить(", "РезультатыПробы.Добавить(");
    let source = format!(
        "Перем РезультатыПробы; РезультатыПробы = Новый Массив; {source} Возврат СтрСоединить(РезультатыПробы, Символы.ПС);"
    );
    let result = state.exec(&source).unwrap();
    let Value::Str(result) = result else {
        panic!("строка результатов")
    };
    let result = result.to_string();
    // Платформа добавляет адрес формы в описание ошибки; контракт закрепляет отказ по арности.
    for id in [
        "CONTENT_LOOKUP.CONTAINS_EMPTY_REASON",
        "CONTENT_LOOKUP.FIND_EMPTY_REASON",
    ] {
        let line = result.lines().find(|line| line.starts_with(id)).unwrap();
        assert!(!line.ends_with("\taccepted"));
    }
    let meaningful = |text: &str| {
        text.lines()
            .filter(|line| !line.contains("_EMPTY_REASON\t"))
            .map(|line| line.replace("\tYes", "\tДа").replace("\tNo", "\tНет"))
            .collect::<Vec<_>>()
    };
    assert_eq!(
        meaningful(&result),
        meaningful(include_str!(
            "../../../tests/conformance/measure/1c/measure-common-attribute-content-lookup.platform.txt"
        ))
    );
}

#[test]
fn global_condition_preserves_native_constant_identity_and_unknown_absence() {
    let configured = catalog()
        .with_common_attribute_condition(
            "common",
            CommonAttributeCondition::Constant("condition".into()),
        )
        .unwrap();
    let mut state_with_condition = state(configured);
    assert_eq!(state_with_condition.exec("Возврат Метаданные.ОбщиеРеквизиты.НаименованиеЯзык1.ConditionalSeparation = Метаданные.Константы.ЗначениеОпцииПробы;").unwrap(), Value::Boolean(true));
    let mut absent = state(
        catalog()
            .with_common_attribute_condition("common", CommonAttributeCondition::Undefined)
            .unwrap(),
    );
    assert_eq!(
        absent
            .exec("Возврат Метаданные.ОбщиеРеквизиты.НаименованиеЯзык1.УсловноеРазделение;")
            .unwrap(),
        Value::Undefined
    );
    let mut unknown = state(catalog());
    assert!(
        unknown
            .exec("Возврат Метаданные.ОбщиеРеквизиты.НаименованиеЯзык1.УсловноеРазделение;")
            .is_err()
    );
    assert!(
        catalog()
            .with_common_attribute_condition(
                "common",
                CommonAttributeCondition::Constant("target".into())
            )
            .is_err()
    );
}

#[test]
fn global_condition_is_read_only_and_separate_from_item_conditions() {
    let catalog = catalog()
        .with_common_attribute_content("common", entries())
        .unwrap()
        .with_common_attribute_condition("common", CommonAttributeCondition::Undefined)
        .unwrap();
    let mut state = state(catalog);
    assert_eq!(state.exec(r#"
        ОбщийРеквизитПробы = Метаданные.ОбщиеРеквизиты.НаименованиеЯзык1;
        Возврат ОбщийРеквизитПробы.УсловноеРазделение = Неопределено
            И ОбщийРеквизитПробы.Состав.Найти("ЗамерВременныхТаблиц").УсловноеРазделение = Метаданные.Константы.ЗначениеОпцииПробы;
    "#).unwrap(), Value::Boolean(true));
    assert!(
        state
            .exec("Метаданные.ОбщиеРеквизиты.НаименованиеЯзык1.УсловноеРазделение = Неопределено;")
            .is_err()
    );
    assert!(
        state
            .exec("Возврат Метаданные.Справочники.ЗамерВременныхТаблиц.ConditionalSeparation;")
            .is_err()
    );
}
