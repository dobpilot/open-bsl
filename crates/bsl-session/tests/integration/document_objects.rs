use bsl_config::{ConfigurationCodeSnapshot, ConfigurationModule, ModuleExecution, ModuleKind};
use bsl_orm::{
    DocumentNumberSettings, DocumentNumberType, MetadataCatalog, MetadataKind, MetadataObject,
    Principal,
};
use bsl_session::{Session, SessionFactory, SessionKind, SessionResources};
use open_bsl::{BslObject, Value, format_value};
use std::sync::Arc;

fn factory(settings: Option<DocumentNumberSettings>, source: Option<&str>) -> SessionFactory {
    factory_attributes(settings, source, vec![])
}

fn factory_attributes(
    settings: Option<DocumentNumberSettings>,
    source: Option<&str>,
    attributes: Vec<bsl_orm::DocumentAttribute>,
) -> SessionFactory {
    let mut object = MetadataObject::new("doc", "ЗамерДокумента", MetadataKind::Document)
        .with_document_attributes(attributes);
    if let Some(settings) = settings {
        object = object.with_document_number_settings(settings);
    }
    let mut modules = vec![ConfigurationModule::new(
        "session",
        "МодульСеанса",
        "Процедура УстановкаПараметровСеанса(Имена) КонецПроцедуры Функция СоздатьПробу() Экспорт Возврат Документы.ЗамерДокумента.СоздатьДокумент(); КонецФункции",
        ModuleKind::Session,
        ModuleExecution::new().with_server(true),
    )];
    if let Some(source) = source {
        object = object.with_object_module("Документ.ЗамерДокумента.МодульОбъекта");
        modules.push(
            ConfigurationModule::new(
                "object",
                "Документ.ЗамерДокумента.МодульОбъекта",
                source,
                ModuleKind::Object,
                ModuleExecution::new().with_server(true),
            )
            .with_owner("doc"),
        );
    }
    SessionFactory::new(
        ConfigurationCodeSnapshot::new("document-test", modules).unwrap(),
        Arc::new(MetadataCatalog::new("document-test", vec![object]).unwrap()),
        "МодульСеанса",
        SessionKind::ThinClient,
        "8.3.27.2342",
    )
    .unwrap()
}

fn session(factory: &SessionFactory) -> Session {
    // Окружение эталонов: русский интерфейс и локаль сеанса.
    factory
        .open(
            Principal::new("user", "Тест", vec![]),
            SessionResources::new(Box::new(()), 1)
                .with_system_language("ru")
                .with_locale_code("ru"),
        )
        .unwrap()
}

fn replay(session: &mut Session, source: &str) -> String {
    // Меняется только приёмник сообщений: вычисления остаются нативным сценарием.
    let source = source.replace("Сообщить(", "РезультатыПробы.Добавить(");
    let module = session.instantiate(&format!(
        "Функция Проверить() Экспорт РезультатыПробы = Новый Массив;\n{source}\nВозврат РезультатыПробы; КонецФункции"
    )).unwrap();
    let Value::Object(result) = session.call(&module, "Проверить", vec![]).unwrap().0
    else {
        panic!("массив результатов");
    };
    let BslObject::Array(result) = result.as_ref() else {
        panic!("массив результатов");
    };
    result
        .borrow()
        .iter()
        .map(|value| format_value(value, None).unwrap())
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn measured_document_body_and_assignment_match_native_oracles() {
    let variants = [
        (
            DocumentNumberType::String,
            9,
            include_str!(
                "../../../../tests/conformance/measure/1c/measure-document-object-properties.bsl"
            ),
            include_str!(
                "../../../../tests/conformance/measure/1c/measure-document-object-properties.platform.txt"
            ),
            include_str!(
                "../../../../tests/conformance/measure/1c/measure-document-object-assignment-string.bsl"
            ),
            include_str!(
                "../../../../tests/conformance/measure/1c/measure-document-object-assignment-string.platform.txt"
            ),
        ),
        (
            DocumentNumberType::Number,
            9,
            include_str!(
                "../../../../tests/conformance/measure/1c/measure-document-object-properties-number.bsl"
            ),
            include_str!(
                "../../../../tests/conformance/measure/1c/measure-document-object-properties-number.platform.txt"
            ),
            include_str!(
                "../../../../tests/conformance/measure/1c/measure-document-object-assignment-number.bsl"
            ),
            include_str!(
                "../../../../tests/conformance/measure/1c/measure-document-object-assignment-number.platform.txt"
            ),
        ),
        (
            DocumentNumberType::String,
            0,
            include_str!(
                "../../../../tests/conformance/measure/1c/measure-document-object-properties-disabled.bsl"
            ),
            include_str!(
                "../../../../tests/conformance/measure/1c/measure-document-object-properties-disabled.platform.txt"
            ),
            include_str!(
                "../../../../tests/conformance/measure/1c/measure-document-object-assignment-disabled.bsl"
            ),
            include_str!(
                "../../../../tests/conformance/measure/1c/measure-document-object-assignment-disabled.platform.txt"
            ),
        ),
    ];
    for (number_type, number_length, body, body_oracle, assignment, assignment_oracle) in variants {
        let factory = factory(
            Some(DocumentNumberSettings {
                number_type,
                number_length,
            }),
            Some(include_str!(
                "../../../../tests/conformance/measure/1c/document-object-properties-module.bsl"
            )),
        );
        let mut session = session(&factory);
        assert_eq!(replay(&mut session, body), body_oracle.trim());
        assert_eq!(replay(&mut session, assignment), assignment_oracle.trim());
    }
}

#[test]
fn document_objects_retain_separate_module_variables() {
    let factory = factory(
        Some(DocumentNumberSettings {
            number_type: DocumentNumberType::String,
            number_length: 9,
        }),
        Some(
            "Перем Счетчик; Функция Следующий() Экспорт Счетчик = Счетчик + 1; Возврат Счетчик; КонецФункции Счетчик = 0;",
        ),
    );
    let source = "Первый = Документы.ЗамерДокумента.CreateDocument(); Второй = Документы.ЗамерДокумента.СоздатьДокумент(); Сообщить(Строка(Первый.Следующий())); Сообщить(Строка(Второй.Следующий())); Сообщить(Строка(Первый.Следующий()));";
    for _ in 0..2 {
        assert_eq!(replay(&mut session(&factory), source), "1\n1\n2");
    }
}

#[test]
fn unknown_number_settings_do_not_create_a_guessed_document() {
    let factory = factory(None, None);
    let mut session = session(&factory);
    let module = session.instantiate("Функция Проверить() Экспорт Возврат Документы.ЗамерДокумента.CreateDocument(); КонецФункции").unwrap();
    assert!(
        session
            .call(&module, "Проверить", vec![])
            .unwrap_err()
            .to_string()
            .contains("настройки номера документа неизвестны")
    );
}

#[test]
fn creation_alias_type_and_arity_use_the_document_context() {
    let factory = factory(
        Some(DocumentNumberSettings {
            number_type: DocumentNumberType::String,
            number_length: 9,
        }),
        None,
    );
    let mut session = session(&factory);
    assert_eq!(
        replay(
            &mut session,
            "Объект = Документы.ЗамерДокумента.cReAtEdOcUmEnT(); Сообщить(Строка(ТипЗнч(Объект) = Тип(\"ДокументОбъект.ЗамерДокумента\"))); Сообщить(Строка(ТипЗнч(Объект) = Тип(\"DocumentObject.ЗамерДокумента\")));"
        ),
        "Да\nДа"
    );
    let module = session.instantiate("Функция Проверить() Экспорт Возврат Документы.ЗамерДокумента.СоздатьДокумент(17); КонецФункции").unwrap();
    assert!(session.call(&module, "Проверить", vec![]).is_err());
}

#[test]
fn failed_document_body_does_not_return_an_object() {
    let factory = factory(
        Some(DocumentNumberSettings {
            number_type: DocumentNumberType::String,
            number_length: 9,
        }),
        Some("Номер = \"body\";"),
    );
    let mut session = session(&factory);
    let module = session.instantiate("Функция Проверить() Экспорт Возврат Документы.ЗамерДокумента.СоздатьДокумент(); КонецФункции").unwrap();
    for _ in 0..2 {
        assert!(
            session
                .call(&module, "Проверить", vec![])
                .unwrap_err()
                .to_string()
                .contains("Документ не выбран")
        );
    }
}

#[test]
fn additional_number_and_date_conversions_match_native() {
    let factory = factory(
        Some(DocumentNumberSettings {
            number_type: DocumentNumberType::Number,
            number_length: 9,
        }),
        None,
    );
    let actual = replay(
        &mut session(&factory),
        include_str!(
            "../../../../tests/conformance/measure/1c/measure-document-conversion-number.bsl"
        ),
    );
    assert_eq!(
        actual,
        include_str!(
            "../../../../tests/conformance/measure/1c/measure-document-conversion-number.platform.txt"
        )
        .trim()
    );
}

#[test]
fn document_body_flag_assignments_match_native() {
    let factory = factory(
        Some(DocumentNumberSettings {
            number_type: DocumentNumberType::String,
            number_length: 9,
        }),
        Some(include_str!(
            "../../../../tests/conformance/measure/1c/document-body-flags-module.bsl"
        )),
    );
    assert_eq!(
        replay(
            &mut session(&factory),
            include_str!(
                "../../../../tests/conformance/measure/1c/measure-document-body-flags.bsl"
            )
        ),
        include_str!(
            "../../../../tests/conformance/measure/1c/measure-document-body-flags.platform.txt"
        )
        .trim()
    );
}

fn filling_attributes() -> Vec<bsl_orm::DocumentAttribute> {
    use open_bsl::{ColumnType, GraphLimits, RuntimeShapes, SerializedValueGraph, TypeId, TypeRef};
    let graph = SerializedValueGraph::capture(
        &[Value::Str("default".into())],
        &RuntimeShapes::seeded(vec![], vec![], None),
        &GraphLimits::default(),
    )
    .unwrap();
    [("Заполняемый", true), ("НеЗаполняемый", false)]
        .into_iter()
        .map(|(name, enabled)| {
            bsl_orm::DocumentAttribute::new(
                name,
                name,
                vec![ColumnType {
                    id: TypeRef::Native(TypeId::String),
                    quals: vec!["12".into()],
                }],
            )
            .with_fill_from_filling_value(enabled)
            .with_fill_value(graph.clone())
        })
        .collect()
}

fn primitive_attributes() -> Vec<bsl_orm::DocumentAttribute> {
    use open_bsl::{
        BslDate, ColumnType, GraphLimits, RuntimeShapes, SerializedValueGraph, TypeId, TypeRef,
    };
    let shapes = RuntimeShapes::seeded(vec![], vec![], None);
    [
        ("ФлагПробы", TypeId::Boolean, Value::Boolean(true)),
        (
            "ДатаПробы",
            TypeId::Date,
            Value::Date(BslDate::parse_digits("20260102030405").unwrap()),
        ),
    ]
    .into_iter()
    .map(|(name, id, default)| {
        bsl_orm::DocumentAttribute::new(name, name, vec![ColumnType::plain(TypeRef::Native(id))])
            .with_fill_from_filling_value(true)
            .with_fill_value(
                SerializedValueGraph::capture(&[default], &shapes, &GraphLimits::default())
                    .unwrap(),
            )
    })
    .collect()
}

#[test]
fn boolean_and_datetime_attributes_match_native_filling_and_conversion() {
    let factory = factory_attributes(
        Some(DocumentNumberSettings {
            number_type: DocumentNumberType::String,
            number_length: 9,
        }),
        Some(include_str!(
            "../../../../tests/conformance/measure/1c/document-fill-primitives-module.bsl"
        )),
        primitive_attributes(),
    );
    for (script, oracle) in [
        (
            include_str!(
                "../../../../tests/conformance/measure/1c/measure-document-fill-primitives.bsl"
            ),
            include_str!(
                "../../../../tests/conformance/measure/1c/measure-document-fill-primitives.platform.txt"
            ),
        ),
        (
            include_str!(
                "../../../../tests/conformance/measure/1c/measure-document-attribute-conversion.bsl"
            ),
            include_str!(
                "../../../../tests/conformance/measure/1c/measure-document-attribute-conversion.platform.txt"
            ),
        ),
    ] {
        assert_eq!(replay(&mut session(&factory), script), oracle.trim());
    }
}

#[test]
fn catalog_reference_and_string_union_attributes_match_native() {
    use open_bsl::{ColumnType, GraphLimits, RuntimeShapes, SerializedValueGraph, TypeId, TypeRef};
    let target = MetadataObject::new("target", "ЗамерВременныхТаблиц", MetadataKind::Catalog);
    let document = MetadataObject::new("doc", "ЗамерДокумента", MetadataKind::Document)
        .with_document_number_settings(DocumentNumberSettings {
            number_type: DocumentNumberType::String,
            number_length: 9,
        })
        .with_object_module("Документ.ЗамерДокумента.МодульОбъекта");
    let declarations =
        MetadataCatalog::new("document-test", vec![target.clone(), document.clone()]).unwrap();
    let reference_type = declarations
        .value_types()
        .find(|ty| ty.names()[0] == "СправочникСсылка.ЗамерВременныхТаблиц")
        .unwrap()
        .clone();
    let reference = ColumnType::plain(TypeRef::Value(reference_type));
    let shapes = RuntimeShapes::seeded(vec![], vec![], None);
    let attributes = [
        ("СсылкаПробы", vec![reference.clone()], Value::Undefined),
        (
            "СоставнойПробы",
            vec![
                reference,
                ColumnType {
                    id: TypeRef::Native(TypeId::String),
                    quals: vec!["12".into()],
                },
            ],
            Value::Str("default".into()),
        ),
    ]
    .into_iter()
    .map(|(name, types, default)| {
        bsl_orm::DocumentAttribute::new(name, name, types)
            .with_fill_from_filling_value(true)
            .with_fill_value(
                SerializedValueGraph::capture(&[default], &shapes, &GraphLimits::default())
                    .unwrap(),
            )
    })
    .collect();
    let catalog = Arc::new(
        MetadataCatalog::new(
            "document-test",
            vec![target, document.with_document_attributes(attributes)],
        )
        .unwrap(),
    );
    let code = ConfigurationCodeSnapshot::new(
        "document-test",
        vec![
            ConfigurationModule::new(
                "session",
                "МодульСеанса",
                "Процедура УстановкаПараметровСеанса(Имена) КонецПроцедуры Функция СоздатьПробу() Экспорт Возврат Документы.ЗамерДокумента.СоздатьДокумент(); КонецФункции",
                ModuleKind::Session,
                ModuleExecution::new().with_server(true),
            ),
            ConfigurationModule::new(
                "object",
                "Документ.ЗамерДокумента.МодульОбъекта",
                include_str!(
                    "../../../../tests/conformance/measure/1c/document-fill-references-module.bsl"
                ),
                ModuleKind::Object,
                ModuleExecution::new().with_server(true),
            )
            .with_owner("doc"),
        ],
    )
    .unwrap();
    let factory = SessionFactory::new(
        code,
        catalog,
        "МодульСеанса",
        SessionKind::ThinClient,
        "8.3.27.2342",
    )
    .unwrap();
    let script = include_str!(
        "../../../../tests/conformance/measure/1c/measure-document-fill-references.bsl"
    );
    let oracle = include_str!(
        "../../../../tests/conformance/measure/1c/measure-document-fill-references.platform.txt"
    );
    assert_eq!(replay(&mut session(&factory), script), oracle.trim());
}

#[test]
fn owned_string_filling_matches_all_measured_handler_forms() {
    let cases = [
        (
            include_str!(
                "../../../../tests/conformance/measure/1c/document-fill-attributes-module.bsl"
            ),
            include_str!(
                "../../../../tests/conformance/measure/1c/measure-document-fill-attributes.bsl"
            ),
            include_str!(
                "../../../../tests/conformance/measure/1c/measure-document-fill-attributes.platform.txt"
            ),
        ),
        (
            include_str!(
                "../../../../tests/conformance/measure/1c/document-fill-attributes-three-module.bsl"
            ),
            include_str!(
                "../../../../tests/conformance/measure/1c/measure-document-fill-attributes-three.bsl"
            ),
            include_str!(
                "../../../../tests/conformance/measure/1c/measure-document-fill-attributes-three.platform.txt"
            ),
        ),
        (
            include_str!(
                "../../../../tests/conformance/measure/1c/document-fill-attributes-empty-module.bsl"
            ),
            include_str!(
                "../../../../tests/conformance/measure/1c/measure-document-fill-attributes-empty.bsl"
            ),
            include_str!(
                "../../../../tests/conformance/measure/1c/measure-document-fill-attributes-empty.platform.txt"
            ),
        ),
        (
            include_str!(
                "../../../../tests/conformance/measure/1c/document-fill-attributes-empty-two-module.bsl"
            ),
            include_str!(
                "../../../../tests/conformance/measure/1c/measure-document-fill-attributes-empty-two.bsl"
            ),
            include_str!(
                "../../../../tests/conformance/measure/1c/measure-document-fill-attributes-empty-two.platform.txt"
            ),
        ),
        (
            include_str!(
                "../../../../tests/conformance/measure/1c/document-fill-attributes-no-handler-module.bsl"
            ),
            include_str!(
                "../../../../tests/conformance/measure/1c/measure-document-fill-attributes-no-handler.bsl"
            ),
            include_str!(
                "../../../../tests/conformance/measure/1c/measure-document-fill-attributes-no-handler.platform.txt"
            ),
        ),
        (
            include_str!(
                "../../../../tests/conformance/measure/1c/document-fill-attributes-no-handler-module.bsl"
            ),
            include_str!(
                "../../../../tests/conformance/measure/1c/measure-document-fill-explicit.bsl"
            ),
            include_str!(
                "../../../../tests/conformance/measure/1c/measure-document-fill-explicit.platform.txt"
            ),
        ),
        (
            include_str!(
                "../../../../tests/conformance/measure/1c/document-attribute-body-module.bsl"
            ),
            include_str!(
                "../../../../tests/conformance/measure/1c/measure-document-attribute-body.bsl"
            ),
            include_str!(
                "../../../../tests/conformance/measure/1c/measure-document-attribute-body.platform.txt"
            ),
        ),
        (
            include_str!(
                "../../../../tests/conformance/measure/1c/document-fill-whitespace-module.bsl"
            ),
            include_str!(
                "../../../../tests/conformance/measure/1c/measure-document-fill-whitespace.bsl"
            ),
            include_str!(
                "../../../../tests/conformance/measure/1c/measure-document-fill-whitespace.platform.txt"
            ),
        ),
    ];
    for (source, script, oracle) in cases {
        let factory = factory_attributes(
            Some(DocumentNumberSettings {
                number_type: DocumentNumberType::String,
                number_length: 9,
            }),
            Some(source),
            filling_attributes(),
        );
        assert_eq!(replay(&mut session(&factory), script), oracle.trim());
    }
}

#[test]
fn standard_filling_uses_the_data_returned_by_the_handler() {
    let factory = factory_attributes(
        Some(DocumentNumberSettings {
            number_type: DocumentNumberType::String,
            number_length: 9,
        }),
        Some(include_str!(
            "../../../../tests/conformance/measure/1c/document-fill-replace-module.bsl"
        )),
        filling_attributes(),
    );
    assert_eq!(
        replay(
            &mut session(&factory),
            include_str!(
                "../../../../tests/conformance/measure/1c/measure-document-fill-replace.bsl"
            ),
        ),
        include_str!(
            "../../../../tests/conformance/measure/1c/measure-document-fill-replace.platform.txt"
        )
        .trim()
    );
}

#[test]
fn document_exchange_and_additional_properties_match_native() {
    let factory = factory(
        Some(DocumentNumberSettings {
            number_type: DocumentNumberType::String,
            number_length: 9,
        }),
        Some(include_str!(
            "../../../../tests/conformance/measure/1c/document-object-context-module.bsl"
        )),
    );
    let script = include_str!(
        "../../../../tests/conformance/measure/1c/measure-document-object-context.bsl"
    );
    let oracle = include_str!(
        "../../../../tests/conformance/measure/1c/measure-document-object-context.platform.txt"
    );
    let mut first = session(&factory);
    assert_eq!(replay(&mut first, script), oracle.trim());
    assert_eq!(replay(&mut session(&factory), script), oracle.trim());
}

#[test]
fn filling_event_data_and_text_match_native() {
    for (source, script, oracle) in [
        (
            include_str!("../../../../tests/conformance/measure/1c/document-fill-module.bsl"),
            include_str!("../../../../tests/conformance/measure/1c/measure-document-fill.bsl"),
            include_str!(
                "../../../../tests/conformance/measure/1c/measure-document-fill.platform.txt"
            ),
        ),
        (
            include_str!("../../../../tests/conformance/measure/1c/document-fill-three-module.bsl"),
            include_str!(
                "../../../../tests/conformance/measure/1c/measure-document-fill-three.bsl"
            ),
            include_str!(
                "../../../../tests/conformance/measure/1c/measure-document-fill-three.platform.txt"
            ),
        ),
    ] {
        let factory = factory(
            Some(DocumentNumberSettings {
                number_type: DocumentNumberType::String,
                number_length: 9,
            }),
            Some(source),
        );
        let actual = replay(&mut session(&factory), script);
        assert_eq!(actual, oracle.trim());
    }
}

#[test]
fn owned_attribute_storage_and_filling_are_isolated_between_objects_and_states() {
    let factory = factory_attributes(
        Some(DocumentNumberSettings {
            number_type: DocumentNumberType::String,
            number_length: 9,
        }),
        None,
        filling_attributes(),
    );
    let source = "Первый = Документы.ЗамерДокумента.СоздатьДокумент(); Второй = Документы.ЗамерДокумента.СоздатьДокумент(); Первый.ЗАПОЛНЯЕМЫЙ = \"first\"; Второй.fIlL(Неопределено); Сообщить(Первый.Заполняемый); Сообщить(Второй.Заполняемый);";
    for _ in 0..2 {
        assert_eq!(replay(&mut session(&factory), source), "first\ndefault");
    }
}

#[test]
fn filling_reports_unknown_defaults_and_preserves_an_existing_fill_export() {
    let settings = Some(DocumentNumberSettings {
        number_type: DocumentNumberType::String,
        number_length: 9,
    });
    let mut attributes = filling_attributes();
    attributes[0] =
        bsl_orm::DocumentAttribute::new("unknown", "Заполняемый", attributes[0].types().to_vec());
    let missing = factory_attributes(settings, None, attributes);
    let mut missing_session = session(&missing);
    let module = missing_session.instantiate("Функция Проверить() Экспорт Объект = Документы.ЗамерДокумента.СоздатьДокумент(); Объект.Заполнить(Неопределено); КонецФункции").unwrap();
    assert!(
        missing_session
            .call(&module, "Проверить", vec![])
            .unwrap_err()
            .to_string()
            .contains("настройка заполнения реквизита")
    );
    let exported = factory_attributes(
        settings,
        Some("Функция Заполнить() Экспорт Возврат 23; КонецФункции"),
        filling_attributes(),
    );
    assert_eq!(
        replay(
            &mut session(&exported),
            "Объект = Документы.ЗамерДокумента.СоздатьДокумент(); Сообщить(Строка(Объект.Заполнить()));"
        ),
        "23"
    );
}

fn enumeration_document_factory(configured_default: bool) -> SessionFactory {
    use open_bsl::{ColumnType, GraphLimits, RuntimeShapes, SerializedValueGraph, TypeId, TypeRef};
    let target = MetadataObject::new("enum", "ЗамерПеречисления", MetadataKind::Enumeration);
    let document = MetadataObject::new("doc", "ЗамерДокумента", MetadataKind::Document)
        .with_document_number_settings(DocumentNumberSettings {
            number_type: DocumentNumberType::String,
            number_length: 9,
        });
    let members = vec![
        bsl_orm::ConfigurationEnumValue::new("enum", "low", "Низкая")
            .with_synonym("ru", "Первое значение"),
        // Синоним из XML пробы `document-attribute-enum.xml`.
        bsl_orm::ConfigurationEnumValue::new("enum", "normal", "Обычная")
            .with_synonym("ru", "Второе значение"),
        bsl_orm::ConfigurationEnumValue::new("enum", "high", "Высокая")
            .with_synonym("ru", "Третье значение"),
    ];
    let declarations = Arc::new(
        MetadataCatalog::new("document-enum-test", vec![target.clone(), document.clone()])
            .unwrap()
            .with_enumeration_values(members.clone())
            .unwrap(),
    );
    let ty = declarations
        .value_types()
        .find(|ty| ty.names()[0] == "ПеречислениеСсылка.ЗамерПеречисления")
        .unwrap()
        .clone();
    let reference = ColumnType::plain(TypeRef::Value(ty));
    let mut builder = open_bsl::Engine::builder();
    let mut enumeration_default = Value::Undefined;
    for ty in declarations.transferable_value_types() {
        if ty.names()[0] == "ПеречислениеСсылка.ЗамерПеречисления"
        {
            enumeration_default = ty.materialize(b"normal").unwrap();
        }
        builder = builder.register_value_type(ty);
    }
    let engine = builder.build().unwrap();
    let shapes = RuntimeShapes::seeded(vec![], vec![], Some(engine.registry()));
    let attributes = [
        ("СсылкаПробы", vec![reference.clone()], Value::Undefined),
        (
            "СоставнойПробы",
            vec![
                reference,
                ColumnType {
                    id: TypeRef::Native(TypeId::String),
                    quals: vec!["12".into()],
                },
            ],
            Value::Str("default".into()),
        ),
    ]
    .into_iter()
    .map(|(name, types, default)| {
        let default = if configured_default {
            enumeration_default.clone()
        } else {
            default
        };
        bsl_orm::DocumentAttribute::new(name, name, types)
            .with_fill_from_filling_value(true)
            .with_fill_value(
                SerializedValueGraph::capture(&[default], &shapes, &GraphLimits::default())
                    .unwrap(),
            )
    })
    .collect();
    let catalog = Arc::new(
        MetadataCatalog::new(
            "document-enum-test",
            vec![
                target,
                document.with_document_attributes(attributes),
                MetadataObject::new("language-ru", "Русский", MetadataKind::Language)
                    .with_string_property("КодЯзыка", "ru"),
            ],
        )
        .unwrap()
        .with_enumeration_values(members)
        .unwrap()
        .with_configuration(
            "Проба",
            "1.0",
            bsl_orm::ScriptVariant::Russian,
            bsl_orm::CompatibilityMode::DontUse,
            Vec::<String>::new(),
        )
        .unwrap()
        .with_primary_language("language-ru")
        .unwrap(),
    );
    let code = ConfigurationCodeSnapshot::new("document-enum-test", vec![ConfigurationModule::new(
        "session", "МодульСеанса", "Процедура УстановкаПараметровСеанса(Имена) КонецПроцедуры Функция СоздатьПробу() Экспорт Возврат Документы.ЗамерДокумента.СоздатьДокумент(); КонецФункции",
        ModuleKind::Session, ModuleExecution::new().with_server(true),
    )]).unwrap();
    SessionFactory::new(
        code,
        catalog,
        "МодульСеанса",
        SessionKind::ThinClient,
        "8.3.27.2342",
    )
    .unwrap()
}

#[test]
fn enumeration_and_string_union_document_attributes_match_native() {
    let factory = enumeration_document_factory(false);
    for (script, oracle) in [
        (
            include_str!(
                "../../../../tests/conformance/measure/1c/measure-document-fill-enumerations.bsl"
            ),
            include_str!(
                "../../../../tests/conformance/measure/1c/measure-document-fill-enumerations.platform.txt"
            ),
        ),
        (
            include_str!(
                "../../../../tests/conformance/measure/1c/measure-document-fill-enumerations-preserve.bsl"
            ),
            include_str!(
                "../../../../tests/conformance/measure/1c/measure-document-fill-enumerations-preserve.platform.txt"
            ),
        ),
    ] {
        assert_eq!(replay(&mut session(&factory), script), oracle.trim());
    }
}

#[test]
fn configured_enumeration_filling_value_matches_native_through_the_graph() {
    let factory = enumeration_document_factory(true);
    let script = include_str!(
        "../../../../tests/conformance/measure/1c/measure-document-fill-enumerations-default.bsl"
    );
    let oracle = include_str!(
        "../../../../tests/conformance/measure/1c/measure-document-fill-enumerations-default.platform.txt"
    );
    assert_eq!(replay(&mut session(&factory), script), oracle.trim());
    assert_eq!(replay(&mut session(&factory), script), oracle.trim());
}

#[test]
fn document_reference_and_string_union_attributes_match_native() {
    use open_bsl::{ColumnType, GraphLimits, RuntimeShapes, SerializedValueGraph, TypeId, TypeRef};
    let document = MetadataObject::new("doc", "ЗамерДокумента", MetadataKind::Document)
        .with_document_number_settings(DocumentNumberSettings {
            number_type: DocumentNumberType::String,
            number_length: 9,
        });
    let declarations = MetadataCatalog::new("document-ref-test", vec![document.clone()]).unwrap();
    let ty = declarations
        .value_types()
        .find(|ty| ty.names()[0] == "ДокументСсылка.ЗамерДокумента")
        .unwrap()
        .clone();
    let reference = ColumnType::plain(TypeRef::Value(ty));
    let shapes = RuntimeShapes::seeded(vec![], vec![], None);
    let attributes = [
        ("СсылкаПробы", vec![reference.clone()], Value::Undefined),
        (
            "СоставнойПробы",
            vec![
                reference,
                ColumnType {
                    id: TypeRef::Native(TypeId::String),
                    quals: vec!["12".into()],
                },
            ],
            Value::Str("default".into()),
        ),
    ]
    .into_iter()
    .map(|(name, types, default)| {
        bsl_orm::DocumentAttribute::new(name, name, types)
            .with_fill_from_filling_value(true)
            .with_fill_value(
                SerializedValueGraph::capture(&[default], &shapes, &GraphLimits::default())
                    .unwrap(),
            )
    })
    .collect();
    let catalog = Arc::new(
        MetadataCatalog::new(
            "document-ref-test",
            vec![document.with_document_attributes(attributes)],
        )
        .unwrap(),
    );
    let code = ConfigurationCodeSnapshot::new("document-ref-test", vec![ConfigurationModule::new(
        "session", "МодульСеанса", "Процедура УстановкаПараметровСеанса(Имена) КонецПроцедуры Функция СоздатьПробу() Экспорт Возврат Документы.ЗамерДокумента.СоздатьДокумент(); КонецФункции",
        ModuleKind::Session, ModuleExecution::new().with_server(true),
    )]).unwrap();
    let factory = SessionFactory::new(
        code,
        catalog,
        "МодульСеанса",
        SessionKind::ThinClient,
        "8.3.27.2342",
    )
    .unwrap();
    for (script, oracle) in [
        (
            include_str!(
                "../../../../tests/conformance/measure/1c/measure-document-fill-document-references.bsl"
            ),
            include_str!(
                "../../../../tests/conformance/measure/1c/measure-document-fill-document-references.platform.txt"
            ),
        ),
        (
            include_str!(
                "../../../../tests/conformance/measure/1c/measure-document-fill-document-references-preserve.bsl"
            ),
            include_str!(
                "../../../../tests/conformance/measure/1c/measure-document-fill-document-references-preserve.platform.txt"
            ),
        ),
        (
            include_str!("../../../../tests/conformance/measure/1c/measure-document-get-ref.bsl"),
            include_str!(
                "../../../../tests/conformance/measure/1c/measure-document-get-ref.platform.txt"
            ),
        ),
    ] {
        assert_eq!(replay(&mut session(&factory), script), oracle.trim());
    }
}

fn mixed_document_factory(reversed: bool) -> SessionFactory {
    use open_bsl::{ColumnType, GraphLimits, RuntimeShapes, SerializedValueGraph, TypeId, TypeRef};
    let target = MetadataObject::new("catalog", "ЗамерВременныхТаблиц", MetadataKind::Catalog);
    let document = MetadataObject::new("doc", "ЗамерДокумента", MetadataKind::Document)
        .with_document_number_settings(DocumentNumberSettings {
            number_type: DocumentNumberType::String,
            number_length: 9,
        });
    let declarations = MetadataCatalog::new(
        "document-mixed-test",
        vec![target.clone(), document.clone()],
    )
    .unwrap();
    let mut references: Vec<_> = [
        "ДокументСсылка.ЗамерДокумента",
        "СправочникСсылка.ЗамерВременныхТаблиц",
    ]
    .into_iter()
    .map(|name| {
        ColumnType::plain(TypeRef::Value(
            declarations
                .value_types()
                .find(|ty| ty.names()[0] == name)
                .unwrap()
                .clone(),
        ))
    })
    .collect();
    if reversed {
        references.reverse();
    }
    let shapes = RuntimeShapes::seeded(vec![], vec![], None);
    let mut with_string = references.clone();
    with_string.push(ColumnType {
        id: TypeRef::Native(TypeId::String),
        quals: vec!["12".into()],
    });
    let attributes = [
        ("СсылкаПробы", references, Value::Undefined),
        ("СоставнойПробы", with_string, Value::Str("default".into())),
    ]
    .into_iter()
    .map(|(name, types, default)| {
        bsl_orm::DocumentAttribute::new(name, name, types)
            .with_fill_from_filling_value(true)
            .with_fill_value(
                SerializedValueGraph::capture(&[default], &shapes, &GraphLimits::default())
                    .unwrap(),
            )
    })
    .collect();
    let catalog = Arc::new(
        MetadataCatalog::new(
            "document-mixed-test",
            vec![target, document.with_document_attributes(attributes)],
        )
        .unwrap(),
    );
    let code = ConfigurationCodeSnapshot::new("document-mixed-test", vec![ConfigurationModule::new(
        "session", "МодульСеанса", "Процедура УстановкаПараметровСеанса(Имена) КонецПроцедуры Функция СоздатьПробу() Экспорт Возврат Документы.ЗамерДокумента.СоздатьДокумент(); КонецФункции",
        ModuleKind::Session, ModuleExecution::new().with_server(true),
    )]).unwrap();
    SessionFactory::new(
        code,
        catalog,
        "МодульСеанса",
        SessionKind::ThinClient,
        "8.3.27.2342",
    )
    .unwrap()
}

#[test]
fn mixed_document_and_catalog_reference_attributes_match_native() {
    for (reversed, script, oracle) in [
        (
            false,
            include_str!(
                "../../../../tests/conformance/measure/1c/measure-document-fill-mixed-references.bsl"
            ),
            include_str!(
                "../../../../tests/conformance/measure/1c/measure-document-fill-mixed-references.platform.txt"
            ),
        ),
        (
            true,
            include_str!(
                "../../../../tests/conformance/measure/1c/measure-document-fill-mixed-references-reversed.bsl"
            ),
            include_str!(
                "../../../../tests/conformance/measure/1c/measure-document-fill-mixed-references-reversed.platform.txt"
            ),
        ),
    ] {
        let factory = mixed_document_factory(reversed);
        assert_eq!(replay(&mut session(&factory), script), oracle.trim());
    }
}

#[test]
fn all_unf_event_basis_reference_members_match_native() {
    use open_bsl::{ColumnType, GraphLimits, RuntimeShapes, SerializedValueGraph, TypeRef};
    // Читается только фиксированная декларация типов эталона, не произвольный XML.
    let xml =
        include_str!("../../../../tests/conformance/measure/1c/unf-event-basis-defined-type.xml");
    let targets: Vec<_> = xml
        .split("<v8:Type>")
        .skip(1)
        .map(|part| {
            let value = part.split_once("</v8:Type>").unwrap().0;
            let (kind, name) = value.strip_prefix("cfg:").unwrap().split_once('.').unwrap();
            // Синонимы шаблонов, из которых проба собирает объекты.
            let (kind, synonym) = match kind {
                "DocumentRef" => (MetadataKind::Document, "Проба документа"),
                "CatalogRef" => (MetadataKind::Catalog, "Замер временных таблиц"),
                other => panic!("неожиданный тип эталона: {other}"),
            };
            MetadataObject::new(name, name, kind).with_reference_type_presentation(synonym)
        })
        .collect();
    assert_eq!(targets.len(), 56);
    let declarations = MetadataCatalog::new("unf-basis-test", targets.clone()).unwrap();
    let columns: Vec<_> = declarations
        .value_types()
        .map(|ty| ColumnType::plain(TypeRef::Value(Arc::clone(ty))))
        .collect();
    assert_eq!(columns.len(), 56);
    let document = MetadataObject::new("probe", "ЗамерДокумента", MetadataKind::Document)
        .with_document_number_settings(DocumentNumberSettings {
            number_type: DocumentNumberType::String,
            number_length: 9,
        })
        .with_document_attributes(vec![
            bsl_orm::DocumentAttribute::new("basis", "СсылкаПробы", columns)
                .with_fill_from_filling_value(true)
                .with_fill_value(
                    SerializedValueGraph::capture(
                        &[Value::Undefined],
                        &RuntimeShapes::seeded(vec![], vec![], None),
                        &GraphLimits::default(),
                    )
                    .unwrap(),
                ),
        ]);
    let mut objects = targets;
    objects.push(document);
    let catalog = Arc::new(MetadataCatalog::new("unf-basis-test", objects).unwrap());
    let code = ConfigurationCodeSnapshot::new("unf-basis-test", vec![ConfigurationModule::new(
        "session", "МодульСеанса", "Процедура УстановкаПараметровСеанса(Имена) КонецПроцедуры Функция СоздатьПробу() Экспорт Возврат Документы.ЗамерДокумента.СоздатьДокумент(); КонецФункции",
        ModuleKind::Session, ModuleExecution::new().with_server(true),
    )]).unwrap();
    let factory = SessionFactory::new(
        code,
        catalog,
        "МодульСеанса",
        SessionKind::ThinClient,
        "8.3.27.2342",
    )
    .unwrap();
    // Проверка количества типов проведена на платформе; здесь воспроизводятся
    // операции над значениями, без подмены ещё не приобретённого DefinedType.
    let script =
        include_str!("../../../../tests/conformance/measure/1c/measure-unf-event-basis.bsl")
            .split_once("КонецПопытки;")
            .unwrap()
            .1;
    let oracle = include_str!(
        "../../../../tests/conformance/measure/1c/measure-unf-event-basis.platform.txt"
    )
    .split_once('\n')
    .unwrap()
    .1
    .trim();
    assert_eq!(replay(&mut session(&factory), script), oracle);
}

#[test]
fn value_storage_document_attribute_matches_native() {
    use open_bsl::{ColumnType, TypeRef};
    let ty = bsl_orm::library()
        .types()
        .iter()
        .copied()
        .find(|ty| ty.name == "ХранилищеЗначения")
        .unwrap();
    let factory = factory_attributes(
        Some(DocumentNumberSettings {
            number_type: DocumentNumberType::String,
            number_length: 9,
        }),
        None,
        vec![
            bsl_orm::DocumentAttribute::new(
                "storage",
                "СсылкаПробы",
                vec![ColumnType::plain(TypeRef::Object(ty))],
            )
            .with_fill_from_filling_value(true),
        ],
    );
    for (script, oracle) in [
        (
            include_str!(
                "../../../../tests/conformance/measure/1c/measure-document-fill-value-storage.bsl"
            ),
            include_str!(
                "../../../../tests/conformance/measure/1c/measure-document-fill-value-storage.platform.txt"
            ),
        ),
        (
            include_str!(
                "../../../../tests/conformance/measure/1c/measure-document-fill-value-storage-preserve.bsl"
            ),
            include_str!(
                "../../../../tests/conformance/measure/1c/measure-document-fill-value-storage-preserve.platform.txt"
            ),
        ),
    ] {
        assert_eq!(replay(&mut session(&factory), script), oracle.trim());
    }
}

#[test]
fn fixed_length_document_number_matches_native() {
    let document = MetadataObject::new("doc", "ЗамерДокумента", MetadataKind::Document)
        .with_document_number_settings(DocumentNumberSettings {
            number_type: DocumentNumberType::String,
            number_length: 9,
        })
        .with_document_number_fixed_length(true)
        .with_document_attributes(vec![]);
    let code = ConfigurationCodeSnapshot::new(
        "fixed-number",
        vec![ConfigurationModule::new(
            "session",
            "МодульСеанса",
            "Процедура УстановкаПараметровСеанса(Имена) КонецПроцедуры",
            ModuleKind::Session,
            ModuleExecution::new().with_server(true),
        )],
    )
    .unwrap();
    let factory = SessionFactory::new(
        code,
        Arc::new(MetadataCatalog::new("fixed-number", vec![document]).unwrap()),
        "МодульСеанса",
        SessionKind::ThinClient,
        "8.3.27.2342",
    )
    .unwrap();
    assert_eq!(replay(&mut session(&factory), include_str!("../../../../tests/conformance/measure/1c/measure-document-object-assignment-fixed.bsl")), include_str!("../../../../tests/conformance/measure/1c/measure-document-object-assignment-fixed.ru.platform.txt").trim());
}

#[test]
fn document_write_declaration_rejects_unconnected_persistence() {
    let factory = factory(
        Some(DocumentNumberSettings {
            number_type: DocumentNumberType::String,
            number_length: 9,
        }),
        Some("Процедура НеВызывается() Экспорт Записать(); КонецПроцедуры"),
    );
    let mut session = session(&factory);
    for expression in [
        "Записать()",
        "Write(Неопределено)",
        "Write(Неопределено, Неопределено)",
    ] {
        let module = session.instantiate(&format!("Функция Проверить() Экспорт ОбъектПробы = Документы.ЗамерДокумента.СоздатьДокумент(); ОбъектПробы.{expression}; КонецФункции")).unwrap();
        let error = session.call(&module, "Проверить", vec![]).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("запись объекта документа в информационную базу пока не подключена")
        );
    }
    let module = session.instantiate("Функция Проверить() Экспорт ОбъектПробы = Документы.ЗамерДокумента.СоздатьДокумент(); Возврат ОбъектПробы.Write(1, 2, 3); КонецФункции").unwrap();
    assert!(session.call(&module, "Проверить", vec![]).is_err());
}

#[test]
fn document_write_declaration_retains_existing_export_arity() {
    for name in ["Write", "Записать"] {
        let factory = factory(
            Some(DocumentNumberSettings {
                number_type: DocumentNumberType::String,
                number_length: 9,
            }),
            Some(&format!(
                "Функция {name}(А, Б, В) Экспорт Возврат А + Б + В; КонецФункции"
            )),
        );
        let mut session = session(&factory);
        let module = session.instantiate(&format!("Функция Проверить() Экспорт ОбъектПробы = Документы.ЗамерДокумента.СоздатьДокумент(); Возврат ОбъектПробы.{name}(1, 2, 3); КонецФункции")).unwrap();
        assert_eq!(
            session.call(&module, "Проверить", vec![]).unwrap().0,
            Value::number_from_i64(6)
        );
    }
}

#[test]
fn failed_document_write_keeps_reference_and_standard_properties_without_handlers() {
    let factory = factory(
        Some(DocumentNumberSettings {
            number_type: DocumentNumberType::String,
            number_length: 9,
        }),
        Some(
            "Перем ВызовыЗаписи; Процедура ПередЗаписью(Отказ, РежимЗаписи, РежимПроведения) ВызовыЗаписи = ВызовыЗаписи + 1; КонецПроцедуры Функция ЧислоВызовов() Экспорт Возврат ВызовыЗаписи; КонецФункции ВызовыЗаписи = 0;",
        ),
    );
    let mut session = session(&factory);
    let module = session
        .instantiate(
            r#"
        Функция Проверить() Экспорт
            ПробаОбъект = Документы.ЗамерДокумента.СоздатьДокумент();
            ПробаОбъект.Номер = "123";
            ПробаОбъект.Дата = '20261002';
            ПробаСсылка = ПробаОбъект.Ссылка;
            ПробаОбъект.ДополнительныеСвойства.Вставить("Маркер", 42);
            Попытка ПробаОбъект.Записать(); Исключение КонецПопытки;
            Возврат ПробаОбъект.ЭтоНовый() И ПробаОбъект.Ссылка = ПробаСсылка
                И ПробаОбъект.Номер = "123" И ПробаОбъект.Дата = '20261002'
                И Не ПробаОбъект.Проведен И Не ПробаОбъект.ПометкаУдаления
                И ПробаОбъект.ДополнительныеСвойства.Маркер = 42
                И ПробаОбъект.ЧислоВызовов() = 0;
        КонецФункции
    "#,
        )
        .unwrap();
    assert_eq!(
        session.call(&module, "Проверить", vec![]).unwrap().0,
        Value::Boolean(true)
    );
}

#[test]
fn missing_enumeration_document_filling_replays_native_observations() {
    use open_bsl::{ColumnType, GraphLimits, RuntimeShapes, SerializedValueGraph, TypeRef};
    let target = MetadataObject::new("enum", "ЗамерПеречисления", MetadataKind::Enumeration)
        .with_reference_storage_number(54);
    let document = MetadataObject::new("doc", "ЗамерДокумента", MetadataKind::Document)
        .with_document_number_settings(DocumentNumberSettings {
            number_type: DocumentNumberType::String,
            number_length: 9,
        });
    let declarations = Arc::new(
        MetadataCatalog::new("missing-enum-fill", vec![target.clone(), document.clone()]).unwrap(),
    );
    let ty = declarations.value_types().next().unwrap().clone();
    let mut builder = open_bsl::Engine::builder();
    let mut default = Value::Undefined;
    for value_type in declarations.transferable_value_types() {
        if value_type.names()[0] == "ПеречислениеСсылка.ЗамерПеречисления"
        {
            default = value_type
                .materialize(b"7664d5d3-bca6-4114-8409-3272f9787060")
                .unwrap();
        }
        builder = builder.register_value_type(value_type);
    }
    let engine = builder.build().unwrap();
    let shapes = RuntimeShapes::seeded(vec![], vec![], Some(engine.registry()));
    let attribute = bsl_orm::DocumentAttribute::new(
        "attribute",
        "СсылкаПробы",
        vec![ColumnType::plain(TypeRef::Value(ty))],
    )
    .with_fill_from_filling_value(true)
    .with_fill_value(
        SerializedValueGraph::capture(&[default], &shapes, &GraphLimits::default()).unwrap(),
    );
    let catalog = Arc::new(
        MetadataCatalog::new(
            "missing-enum-fill",
            vec![target, document.with_document_attributes(vec![attribute])],
        )
        .unwrap(),
    );
    let code = ConfigurationCodeSnapshot::new("missing-enum-fill", vec![ConfigurationModule::new(
        "session", "МодульСеанса",
        "Процедура УстановкаПараметровСеанса(Имена) КонецПроцедуры Функция СоздатьПробу() Экспорт Возврат Документы.ЗамерДокумента.СоздатьДокумент(); КонецФункции",
        ModuleKind::Session, ModuleExecution::new().with_server(true),
    )]).unwrap();
    let factory = SessionFactory::new(
        code,
        catalog,
        "МодульСеанса",
        SessionKind::ThinClient,
        "8.3.27.2342",
    )
    .unwrap();
    let script =
        include_str!("../../../../tests/conformance/measure/1c/measure-dangling-enum-filling.bsl");
    for (language, oracle) in [
        (
            "en",
            include_str!(
                "../../../../tests/conformance/measure/1c/measure-dangling-enum-filling.platform.txt"
            ),
        ),
        (
            "ru",
            include_str!(
                "../../../../tests/conformance/measure/1c/measure-dangling-enum-storage-russian-ui.platform.txt"
            ),
        ),
    ] {
        let mut session = factory
            .open(
                Principal::new("user", "Тест", vec![]),
                SessionResources::new(Box::new(()), 1).with_system_language(language),
            )
            .unwrap();
        // Проверяется отдельный блок создания и заполнения; полный сценарий
        // с метаданными воспроизводится в `document_attribute_metadata.rs`.
        let expected = oracle
            .lines()
            .skip(4)
            .take(3)
            .collect::<Vec<_>>()
            .join("\n");
        let filling = script.split_once("КонецПопытки;").unwrap().1;
        assert_eq!(replay(&mut session, filling), expected);
    }
}
