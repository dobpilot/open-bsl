use super::*;
use open_bsl::{Engine, HostError, UserMessageDto, UserMessageSink};

#[derive(Debug, Default)]
struct Messages(RefCell<Vec<UserMessageDto>>, Option<usize>);

impl UserMessageSink for Messages {
    fn enqueue(&self, message: &UserMessageDto) -> Result<(), HostError> {
        self.0.borrow_mut().push(message.clone());
        Ok(())
    }

    fn message_bytes_left(&self) -> Option<usize> {
        self.1
    }
}

fn check_message(language: &str, field: &str, native: &str) {
    check_message_with_attribute(language, field, native, None);
}

fn check_message_with_attribute(
    language: &str,
    field: &str,
    native: &str,
    own: Option<DocumentAttribute>,
) {
    check_message_with_budget(language, field, native, own, None);
}

fn check_message_with_budget(
    language: &str,
    field: &str,
    native: &str,
    own: Option<DocumentAttribute>,
    budget: Option<usize>,
) {
    let own_check = own.is_some();
    let attributes: Vec<_> = own.into_iter().collect();
    let metadata = MetadataObject::new("catalog", "Проба", MetadataKind::Catalog)
        .with_catalog_standard_attributes(CatalogStandardAttributes {
            code_type: CatalogCodeType::String,
            code_length: 9,
            description_length: 25,
        })
        .with_catalog_standard_fill_checking(vec![CatalogStandardAttributeFillChecking {
            name: if own_check { "Description" } else { field }.into(),
            fill_checking: FillChecking::ShowError,
        }])
        .with_catalog_attribute_names(
            attributes
                .iter()
                .map(|attribute| attribute.name().to_owned()),
        )
        .with_catalog_attributes(attributes)
        .with_catalog_tabular_section_names(Vec::<String>::new())
        .with_catalog_common_attributes(vec![]);
    let catalog = Arc::new(
        MetadataCatalog::new(
            "messages",
            vec![
                metadata,
                MetadataObject::new("language-en", "Английский", MetadataKind::Language)
                    .with_string_property("КодЯзыка", "en"),
            ],
        )
        .unwrap()
        .with_configuration(
            "Проба",
            "1.0",
            ScriptVariant::English,
            CompatibilityMode::DontUse,
            Vec::<String>::new(),
        )
        .unwrap()
        .with_primary_language("language-en")
        .unwrap(),
    );
    let mut builder = Engine::builder().register_library(library());
    for ty in catalog.transferable_value_types() {
        builder = builder.register_value_type(ty);
    }
    let engine = builder.build().unwrap();
    let messages = Rc::new(Messages(RefCell::new(vec![]), budget));
    let mut state = engine
        .state_builder()
        .host_data(Rc::new(
            SessionData::new(catalog, Principal::new("user", "Тест", vec![]))
                .with_system_language(language),
        ))
        .message_sink(messages.clone())
        .build();
    let result = state
            .exec(if own_check {
                "Проба = Справочники.Проба.СоздатьЭлемент(); Проба.Наименование = \"filled\"; Возврат Проба.ПроверитьЗаполнение();"
            } else {
                "Возврат Справочники.Проба.СоздатьЭлемент().ПроверитьЗаполнение();"
            });
    if budget == Some(0) {
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("исчерпан бюджет сообщений")
        );
        assert!(messages.0.borrow().is_empty());
        return;
    }
    assert_eq!(result.unwrap(), Value::Boolean(false));
    let messages = messages.0.borrow();
    assert_eq!(messages.len(), 1);
    let expected = |name: &str| {
        native
            .lines()
            .find_map(|line| line.strip_prefix(name))
            .unwrap()
    };
    if own_check {
        assert_eq!(messages[0].text, expected("catalog.own.message="));
        assert_eq!(messages[0].field, expected("catalog.own.field="));
    } else {
        assert_eq!(messages[0].text, expected("catalog.messages.text="));
        assert_eq!(messages[0].field, expected("catalog.messages.field="));
        assert_eq!(messages[0].data_path, expected("catalog.messages.path="));
    }
}

#[test]
fn filling_message_respects_host_budget_before_enqueue() {
    check_message_with_budget("en", "Description", "", None, Some(0));
}

#[test]
fn known_unsupported_standard_check_does_not_become_success() {
    let metadata = MetadataObject::new("catalog", "Проба", MetadataKind::Catalog)
        .with_catalog_standard_attributes(CatalogStandardAttributes {
            code_type: CatalogCodeType::String,
            code_length: 9,
            description_length: 25,
        })
        .with_catalog_standard_fill_checking(vec![CatalogStandardAttributeFillChecking {
            name: "Parent".into(),
            fill_checking: FillChecking::ShowError,
        }])
        .with_catalog_attribute_names(Vec::<String>::new())
        .with_catalog_tabular_section_names(Vec::<String>::new())
        .with_catalog_common_attributes(vec![]);
    let catalog = Arc::new(MetadataCatalog::new("unsupported", vec![metadata]).unwrap());
    let mut builder = Engine::builder().register_library(library());
    for ty in catalog.transferable_value_types() {
        builder = builder.register_value_type(ty);
    }
    let engine = builder.build().unwrap();
    let mut state = engine
        .state_builder()
        .host_data(Rc::new(SessionData::new(
            catalog,
            Principal::new("user", "Тест", vec![]),
        )))
        .build();
    let error = state
        .exec("Возврат Справочники.Проба.СоздатьЭлемент().CheckFilling();")
        .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("проверка стандартного реквизита «Parent» пока не поддерживается")
    );
}

#[test]
fn own_attribute_validation_message_uses_native_localized_synonym() {
    let attribute = DocumentAttribute::new(
        "own",
        "ПроверяемаяСтрока",
        vec![open_bsl::ColumnType {
            id: TypeRef::Native(open_bsl::TypeId::String),
            quals: vec!["30".into(), "1".into()],
        }],
    )
    .with_fill_checking(FillChecking::ShowError);
    check_message_with_attribute(
        "en",
        "ПроверяемаяСтрока",
        include_str!(
            "../../../tests/conformance/measure/1c/measure-catalog-check-filling-own-attribute.platform.txt"
        ),
        Some(attribute.clone()),
    );
    check_message_with_attribute(
        "en",
        "ПроверяемаяСтрока",
        include_str!(
            "../../../tests/conformance/measure/1c/measure-catalog-check-filling-own-attribute-synonym.platform.txt"
        ),
        Some(
            attribute
                .clone()
                .with_synonym("en", "Own attribute title")
                .with_synonym("ru", "СинонимПроверяемойСтроки"),
        ),
    );
    check_message_with_attribute(
        "ru",
        "ПроверяемаяСтрока",
        include_str!(
            "../../../tests/conformance/measure/1c/measure-catalog-check-filling-own-attribute-synonym-ru.platform.txt"
        ),
        Some(
            attribute
                .with_synonym("en", "Own attribute title")
                .with_synonym("ru", "СинонимПроверяемойСтроки"),
        ),
    );
}

#[test]
fn scalar_attribute_checks_match_native_results_and_message_order() {
    use open_bsl::{ColumnType, TypeId};
    let attributes = [
        (
            "ПроверяемаяСтрока",
            TypeId::String,
            vec!["30".into(), "1".into()],
        ),
        (
            "ПроверяемоеЧисло",
            TypeId::Number,
            vec!["10".into(), "2".into(), "0".into()],
        ),
        ("ПроверяемоеБулево", TypeId::Boolean, vec![]),
        ("ПроверяемаяДата", TypeId::Date, vec![]),
    ]
    .into_iter()
    .enumerate()
    .map(|(index, (name, ty, quals))| {
        DocumentAttribute::new(
            format!("own-{index}"),
            name,
            vec![ColumnType {
                id: TypeRef::Native(ty),
                quals,
            }],
        )
        .with_fill_checking(FillChecking::ShowError)
    })
    .collect::<Vec<_>>();
    let metadata = MetadataObject::new("catalog", "Проба", MetadataKind::Catalog)
        .with_catalog_standard_attributes(CatalogStandardAttributes {
            code_type: CatalogCodeType::String,
            code_length: 9,
            description_length: 25,
        })
        .with_catalog_standard_fill_checking(vec![CatalogStandardAttributeFillChecking {
            name: "Description".into(),
            fill_checking: FillChecking::ShowError,
        }])
        .with_catalog_attribute_names(
            attributes
                .iter()
                .map(|attribute| attribute.name().to_owned()),
        )
        .with_catalog_attributes(attributes)
        .with_catalog_tabular_section_names(Vec::<String>::new())
        .with_catalog_common_attributes(vec![]);
    let catalog = Arc::new(
        MetadataCatalog::new(
            "scalars",
            vec![
                metadata,
                MetadataObject::new("language-en", "Английский", MetadataKind::Language)
                    .with_string_property("КодЯзыка", "en"),
            ],
        )
        .unwrap()
        .with_configuration(
            "Проба",
            "1.0",
            ScriptVariant::English,
            CompatibilityMode::DontUse,
            Vec::<String>::new(),
        )
        .unwrap()
        .with_primary_language("language-en")
        .unwrap(),
    );
    let mut builder = Engine::builder().register_library(library());
    for ty in catalog.transferable_value_types() {
        builder = builder.register_value_type(ty);
    }
    let engine = builder.build().unwrap();
    let messages = Rc::new(Messages::default());
    let mut state = engine
        .state_builder()
        .host_data(Rc::new(
            SessionData::new(catalog, Principal::new("user", "Тест", vec![]))
                .with_system_language("en"),
        ))
        .message_sink(messages.clone())
        .build();
    let result = state
        .exec(
            r#"
        Проба = Справочники.Проба.СоздатьЭлемент();
        Проба.Наименование = "filled";
        Проба.ПроверяемаяСтрока = "filled";
        Итог = ?(Проба.ПроверитьЗаполнение(), "TRUE", "FALSE");
        Проба.ПроверяемоеЧисло = 1;
        Проба.ПроверяемаяДата = Дата(2026, 10, 2);
        Итог = Итог + "|" + ?(Проба.ПроверитьЗаполнение(), "TRUE", "FALSE");
        Проба.ПроверяемоеБулево = Истина;
        Возврат Итог + "|" + ?(Проба.ПроверитьЗаполнение(), "TRUE", "FALSE");
    "#,
        )
        .unwrap();
    let native = include_str!(
        "../../../tests/conformance/measure/1c/measure-catalog-check-filling-scalars.platform.txt"
    );
    let expected = [
        "catalog.scalars.empty=",
        "catalog.scalars.false=",
        "catalog.scalars.true=",
    ]
    .map(|prefix| {
        native
            .lines()
            .find_map(|line| line.strip_prefix(prefix))
            .unwrap()
    })
    .join("|");
    assert_eq!(open_bsl::format_value(&result, None).unwrap(), expected);
    let fields = messages
        .0
        .borrow()
        .iter()
        .map(|message| message.field.clone())
        .collect::<Vec<_>>();
    assert_eq!(
        fields,
        native
            .lines()
            .filter_map(|line| line.strip_prefix("catalog.scalars.empty.field="))
            .collect::<Vec<_>>()
    );
}

#[test]
fn standard_validation_messages_match_native_text_field_and_path() {
    check_message(
        "en",
        "Description",
        include_str!(
            "../../../tests/conformance/measure/1c/measure-catalog-check-filling-messages.platform.txt"
        ),
    );
    check_message(
        "ru",
        "Description",
        include_str!(
            "../../../tests/conformance/measure/1c/measure-catalog-check-filling-messages-ru.platform.txt"
        ),
    );
    check_message(
        "en",
        "Code",
        include_str!(
            "../../../tests/conformance/measure/1c/measure-catalog-check-filling-messages-code.platform.txt"
        ),
    );
    check_message(
        "ru",
        "Code",
        include_str!(
            "../../../tests/conformance/measure/1c/measure-catalog-check-filling-messages-code-ru.platform.txt"
        ),
    );
}
