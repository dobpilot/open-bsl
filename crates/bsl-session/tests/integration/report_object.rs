use bsl_config::{ConfigurationCodeSnapshot, ConfigurationModule, ModuleExecution, ModuleKind};
use bsl_orm::{
    DocumentAttribute, DocumentTabularSection, MetadataCatalog, MetadataKind, MetadataObject,
    Principal,
};
use bsl_session::{SessionFactory, SessionKind, SessionResources};
use open_bsl::{BslObject, ColumnType, TypeId, TypeRef, Value, format_value};
use std::sync::Arc;

fn factory(known: bool) -> SessionFactory {
    factory_with_object_source(
        known,
        include_str!("../../../../docs/research/fixtures/report-object/ObjectModule.bsl"),
    )
}

fn factory_with_object_source(known: bool, object_source: &str) -> SessionFactory {
    factory_with_sources(
        known,
        object_source,
        include_str!("../../../../docs/research/fixtures/report-object/ManagerModule.bsl"),
    )
}

fn factory_with_sources(known: bool, object_source: &str, manager_source: &str) -> SessionFactory {
    let mut object = MetadataObject::new("report", "ЗамерОтчета", MetadataKind::Report)
        .with_object_module("Отчет.ЗамерОтчета.МодульОбъекта")
        .with_manager_module("Отчет.ЗамерОтчета.МодульМенеджера");
    if known {
        let string = ColumnType {
            id: TypeRef::Native(TypeId::String),
            quals: vec!["12".into()],
        };
        let boolean = ColumnType::plain(TypeRef::Native(TypeId::Boolean));
        object = object
            .with_report_attributes(vec![
                DocumentAttribute::new("text", "ТекстПробы", vec![string.clone()]),
                DocumentAttribute::new("flag", "ФлагПробы", vec![boolean.clone()]),
            ])
            .with_report_tabular_sections(vec![
                DocumentTabularSection::new("rows", "СтрокиПробы").with_attributes(vec![
                    DocumentAttribute::new("row-text", "ТекстСтроки", vec![string]),
                    DocumentAttribute::new("row-flag", "ФлагСтроки", vec![boolean]),
                ]),
            ]);
    }
    let code = ConfigurationCodeSnapshot::new(
        "report",
        vec![
            ConfigurationModule::new(
                "session", "МодульСеанса",
                "Процедура УстановкаПараметровСеанса(Имена) КонецПроцедуры Функция СоздатьПробу() Экспорт Возврат Отчеты.ЗамерОтчета.СоздатьВКонтексте(); КонецФункции",
                ModuleKind::Session, ModuleExecution::new().with_server(true),
            ),
            ConfigurationModule::new(
                "manager", "Отчет.ЗамерОтчета.МодульМенеджера",
                manager_source,
                ModuleKind::Manager, ModuleExecution::new().with_server(true),
            ).with_owner("report"),
            ConfigurationModule::new(
                "object", "Отчет.ЗамерОтчета.МодульОбъекта",
                object_source,
                ModuleKind::Object, ModuleExecution::new().with_server(true),
            ).with_owner("report"),
        ],
    ).unwrap();
    SessionFactory::new(
        code,
        Arc::new(MetadataCatalog::new("report", vec![object]).unwrap()),
        "МодульСеанса",
        SessionKind::ThinClient,
        "8.3.27.2342",
    )
    .unwrap()
}

#[test]
fn report_manager_create_export_keeps_precedence_over_the_standard_method() {
    let factory = factory_with_sources(
        true,
        include_str!("../../../../docs/research/fixtures/report-object/ObjectModule.bsl"),
        "Функция Создать() Экспорт Возврат 42; КонецФункции",
    );
    let mut session = factory
        .open(
            Principal::new("user", "Тест", vec![]),
            SessionResources::new(Box::new(()), 1),
        )
        .unwrap();
    let module = session
        .instantiate(
            "Функция Проверить() Экспорт Возврат Отчеты.ЗамерОтчета.сОзДаТь(); КонецФункции",
        )
        .unwrap();
    assert_eq!(
        session.call(&module, "Проверить", vec![]).unwrap().0,
        Value::number_from_i64(42)
    );
}

#[test]
fn report_creation_rejects_unknown_field_composition() {
    let factory = factory(false);
    let mut session = factory
        .open(
            Principal::new("user", "Тест", vec![]),
            SessionResources::new(Box::new(()), 1),
        )
        .unwrap();
    let module = session
        .instantiate(
            "Функция СоздатьПробу() Экспорт Возврат Отчеты.ЗамерОтчета.Создать(); КонецФункции",
        )
        .unwrap();
    let error = session.call(&module, "СоздатьПробу", vec![]).unwrap_err();
    assert!(error.to_string().contains("неизвестны"), "{error}");
}

#[test]
fn report_objects_replay_the_native_controls() {
    let factory = factory(true);
    let mut session = factory
        .open(
            Principal::new("user", "Тест", vec![]),
            SessionResources::new(Box::new(()), 1),
        )
        .unwrap();
    // Меняется только приёмник вывода; нативные выражения сохраняются.
    let source = include_str!("../../../../docs/research/fixtures/report-object/measure.bsl")
        .replace("Сообщить(", "РезультатыПробы.Добавить(");
    let module = session.instantiate(&format!(
        "Функция Проверить() Экспорт РезультатыПробы = Новый Массив;\n{source}\nВозврат РезультатыПробы; КонецФункции",
    )).unwrap();
    let Value::Object(result) = session.call(&module, "Проверить", vec![]).unwrap().0
    else {
        panic!("ожидался массив результатов")
    };
    let BslObject::Array(result) = result.as_ref() else {
        panic!("ожидался массив результатов")
    };
    let actual = result
        .borrow()
        .iter()
        .map(|value| format_value(value, None).unwrap())
        .collect::<Vec<_>>()
        .join("\n");
    assert_eq!(
        actual,
        include_str!("../../../../docs/research/fixtures/report-object/measure.platform.txt")
            .trim_end()
    );
}

#[test]
fn report_metadata_retains_type_identity_and_unknown_composition() {
    let catalog = MetadataCatalog::new(
        "metadata",
        vec![
            MetadataObject::new("report", "Отчет", MetadataKind::Report),
            MetadataObject::new("processor", "Обработка", MetadataKind::DataProcessor),
        ],
    )
    .unwrap();
    let ty = Arc::clone(catalog.report_object_types().next().unwrap());
    let catalog = catalog
        .with_report_metadata("report", Some(vec![]), Some(vec![]))
        .unwrap();
    assert!(Arc::ptr_eq(
        &ty,
        catalog.report_object_types().next().unwrap()
    ));
    assert!(
        catalog
            .object_by_id("report")
            .unwrap()
            .report_attributes()
            .unwrap()
            .is_empty()
    );
    assert!(matches!(
        MetadataCatalog::new(
            "wrong",
            vec![MetadataObject::new(
                "processor",
                "Обработка",
                MetadataKind::DataProcessor
            )]
        )
        .unwrap()
        .with_report_metadata("processor", Some(vec![]), Some(vec![])),
        Err(bsl_orm::CatalogError::UnknownReportOwner(_))
    ));
    let catalog = catalog.with_report_metadata("report", None, None).unwrap();
    assert!(
        catalog
            .object_by_id("report")
            .unwrap()
            .report_attributes()
            .is_none()
    );
    assert!(
        catalog
            .object_by_id("report")
            .unwrap()
            .report_tabular_sections()
            .is_none()
    );
}

#[test]
fn report_module_memory_cannot_be_read_in_another_session() {
    let factory = factory(true);
    let principal = || Principal::new("user", "Тест", vec![]);
    let mut first = factory
        .open(principal(), SessionResources::new(Box::new(()), 1))
        .unwrap();
    let mut second = factory
        .open(principal(), SessionResources::new(Box::new(()), 2))
        .unwrap();
    let create = first
        .instantiate(
            "Функция СоздатьПробу() Экспорт Возврат Отчеты.ЗамерОтчета.Создать(); КонецФункции",
        )
        .unwrap();
    let report = first.call(&create, "СоздатьПробу", vec![]).unwrap().0;
    let read = second
        .instantiate(
            "Функция Прочитать(ОтчетПробы) Экспорт Возврат ОтчетПробы.СчетчикПробы; КонецФункции",
        )
        .unwrap();
    let error = second.call(&read, "Прочитать", vec![report]).unwrap_err();
    assert!(error.to_string().contains("другому сеансу"), "{error}");
}

#[test]
fn report_settings_composer_compiles_in_dormant_handlers_and_fails_explicitly_when_read() {
    let factory = factory_with_object_source(
        true,
        "Функция ПрочитатьКомпоновщик() Экспорт Возврат КомпоновщикНастроек; КонецФункции Функция ReadComposer() Экспорт Возврат SettingsComposer; КонецФункции Функция Прочитать() Экспорт Возврат 7; КонецФункции",
    );
    let mut session = factory
        .open(
            Principal::new("user", "Тест", vec![]),
            SessionResources::new(Box::new(()), 1),
        )
        .unwrap();
    let module = session
        .instantiate(
            "Функция СоздатьПробу() Экспорт Возврат Отчеты.ЗамерОтчета.Создать(); КонецФункции",
        )
        .unwrap();
    assert!(matches!(
        session.call(&module, "СоздатьПробу", vec![]).unwrap().0,
        Value::Object(_)
    ));
    for expression in [
        "КомпоновщикНастроек",
        "SettingsComposer",
        "ПрочитатьКомпоновщик()",
        "ReadComposer()",
    ] {
        let source = format!(
            "Функция Проверить() Экспорт Возврат Отчеты.ЗамерОтчета.Создать().{expression}; КонецФункции"
        );
        let module = session.instantiate(&source).unwrap();
        let error = session.call(&module, "Проверить", vec![]).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("объектная модель настроек компоновки данных не реализована"),
            "{error}"
        );
    }
    for name in ["КомпоновщикНастроек", "SettingsComposer"] {
        let source = format!(
            "Процедура Проверить() Экспорт Отчеты.ЗамерОтчета.Создать().{name} = Неопределено; КонецПроцедуры"
        );
        let module = session.instantiate(&source).unwrap();
        let error = session.call(&module, "Проверить", vec![]).unwrap_err();
        assert!(error.to_string().contains("только для чтения"), "{error}");
    }
}

#[test]
fn report_schema_property_preserves_read_write_declaration_and_explicit_unsupported_access() {
    let factory = factory_with_object_source(
        true,
        "Функция ПрочитатьСхему() Экспорт Возврат СхемаКомпоновкиДанных; КонецФункции Функция ReadSchema() Экспорт Возврат DataCompositionSchema; КонецФункции",
    );
    let mut session = factory
        .open(
            Principal::new("user", "Тест", vec![]),
            SessionResources::new(Box::new(()), 1),
        )
        .unwrap();
    for expression in [
        "СхемаКомпоновкиДанных",
        "DataCompositionSchema",
        "ПрочитатьСхему()",
        "ReadSchema()",
    ] {
        let module = session.instantiate(&format!("Функция Проверить() Экспорт Возврат Отчеты.ЗамерОтчета.Создать().{expression}; КонецФункции")).unwrap();
        let error = session.call(&module, "Проверить", vec![]).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("объектная модель схемы компоновки данных не реализована"),
            "{error}"
        );
    }
    for name in ["СхемаКомпоновкиДанных", "DataCompositionSchema"] {
        let module = session.instantiate(&format!("Процедура Проверить() Экспорт Отчеты.ЗамерОтчета.Создать().{name} = Неопределено; КонецПроцедуры")).unwrap();
        let error = session.call(&module, "Проверить", vec![]).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("объектная модель схемы компоновки данных не реализована"),
            "{error}"
        );
    }
}
