use bsl_config::{ConfigurationCodeSnapshot, ConfigurationModule, ModuleExecution, ModuleKind};
use bsl_orm::{
    DocumentNumberSettings, DocumentNumberType, MetadataCatalog, MetadataKind, MetadataObject,
    Principal,
};
use bsl_session::{SessionFactory, SessionKind, SessionResources};
use open_bsl::Value;
use std::sync::Arc;

fn factory(source: &str) -> SessionFactory {
    let owner_module = "Документ.Контроль.МодульОбъекта";
    let code = ConfigurationCodeSnapshot::new("document-modified", vec![
        ConfigurationModule::new("session", "МодульСеанса", "Процедура УстановкаПараметровСеанса(Имена) КонецПроцедуры Функция Связать() Экспорт Возврат Документы.Контроль.СоздатьДокумент(); КонецФункции", ModuleKind::Session, ModuleExecution::new().with_server(true)),
        ConfigurationModule::new("object", owner_module, source, ModuleKind::Object, ModuleExecution::new().with_server(true)).with_owner("document"),
    ]).unwrap();
    let catalog = MetadataCatalog::new(
        "document-modified",
        vec![
            MetadataObject::new("document", "Контроль", MetadataKind::Document)
                .with_object_module(owner_module)
                .with_document_attributes(vec![])
                .with_document_tabular_sections(vec![])
                .with_document_number_settings(DocumentNumberSettings {
                    number_type: DocumentNumberType::String,
                    number_length: 9,
                }),
        ],
    )
    .unwrap();
    SessionFactory::new(
        code,
        Arc::new(catalog),
        "МодульСеанса",
        SessionKind::ThinClient,
        "8.3.27.2342",
    )
    .unwrap()
}

#[test]
fn document_modified_binds_bare_calls_and_reports_unavailable_tracking() {
    let factory = factory(
        "Процедура НеВызвана() Модифицированность(); КонецПроцедуры Функция СобственныйВызов() Экспорт Возврат mOdIfIeD(); КонецФункции",
    );
    let mut session = factory
        .open(
            Principal::new("user", "", vec![]),
            SessionResources::new(Box::new(()), 1),
        )
        .unwrap();
    for name in ["Модифицированность", "mOdIfIeD", "СобственныйВызов"]
    {
        let module = session.instantiate(&format!("Функция Проверить() Экспорт Возврат Документы.Контроль.СоздатьДокумент().{name}(); КонецФункции")).unwrap();
        let error = session
            .call(&module, "Проверить", vec![])
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("Модифицированность") && error.contains("не подключено"),
            "{error}"
        );
    }
    let module = session.instantiate("Функция Проверить() Экспорт Возврат Документы.Контроль.СоздатьДокумент().Modified(1); КонецФункции").unwrap();
    let error = session
        .call(&module, "Проверить", vec![])
        .unwrap_err()
        .to_string();
    assert!(!error.contains("не подключено"), "{error}");
    assert!(
        session
            .instantiate("Функция Проверить() Экспорт Возврат Модифицированность(); КонецФункции")
            .is_err()
    );
}

#[test]
fn existing_local_document_function_keeps_its_precedence() {
    let factory = factory(
        "Функция Модифицированность(ЗначениеПробы) Возврат ЗначениеПробы; КонецФункции Функция Проверить() Экспорт Возврат Модифицированность(17); КонецФункции",
    );
    let mut session = factory
        .open(
            Principal::new("user", "", vec![]),
            SessionResources::new(Box::new(()), 1),
        )
        .unwrap();
    let module = session.instantiate("Функция Проверить() Экспорт Возврат Документы.Контроль.СоздатьДокумент().Проверить(); КонецФункции").unwrap();
    assert_eq!(
        session.call(&module, "Проверить", vec![]).unwrap().0,
        Value::number_from_i64(17)
    );
}

#[test]
fn an_existing_same_named_export_preserves_its_result_and_arity() {
    for name in ["Модифицированность", "Modified"] {
        let factory = factory(&format!(
            "Функция {name}(ЗначениеПробы) Экспорт Возврат ЗначениеПробы; КонецФункции"
        ));
        let mut session = factory
            .open(
                Principal::new("user", "", vec![]),
                SessionResources::new(Box::new(()), 1),
            )
            .unwrap();
        let module = session.instantiate(&format!("Функция Проверить() Экспорт Возврат Документы.Контроль.СоздатьДокумент().{name}(17); КонецФункции")).unwrap();
        assert_eq!(
            session.call(&module, "Проверить", vec![]).unwrap().0,
            Value::number_from_i64(17)
        );
    }
}
