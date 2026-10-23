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
    let code = ConfigurationCodeSnapshot::new("document-lock-state", vec![
        ConfigurationModule::new("session", "МодульСеанса", "Процедура УстановкаПараметровСеанса(Имена) КонецПроцедуры Функция Связать() Экспорт Возврат Документы.Контроль.СоздатьДокумент(); КонецФункции", ModuleKind::Session, ModuleExecution::new().with_server(true)),
        ConfigurationModule::new("object", owner_module, source, ModuleKind::Object, ModuleExecution::new().with_server(true)).with_owner("document"),
    ]).unwrap();
    let catalog = MetadataCatalog::new(
        "document-lock-state",
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
fn new_document_lock_state_and_body_match_the_native_control() {
    let oracle = include_str!(
        "../../../docs/research/fixtures/document-lock-state-native-1015/platform.oracle"
    );
    assert!(oracle.contains("locked.value=FALSE") && oracle.contains("locked.body=ERROR"));
    let factory = factory(include_str!(
        "../../../docs/research/fixtures/document-lock-state-native-1015/ObjectModule.bsl"
    ));
    let mut session = factory
        .open(
            Principal::new("user", "", vec![]),
            SessionResources::new(Box::new(()), 1),
        )
        .unwrap();
    for name in ["Заблокирован", "iSlOcKeD", "ПроверитьЗаблокирован"]
    {
        let module = session.instantiate(&format!("Функция Проверить() Экспорт Возврат Документы.Контроль.СоздатьДокумент().{name}(); КонецФункции")).unwrap();
        assert_eq!(
            session.call(&module, "Проверить", vec![]).unwrap().0,
            Value::Boolean(false)
        );
    }
    let module = session.instantiate("Функция Проверить() Экспорт Возврат Документы.Контроль.СоздатьДокумент().ТелоЗаблокирован(); КонецФункции").unwrap();
    assert_eq!(
        session.call(&module, "Проверить", vec![]).unwrap().0,
        Value::Str("ERROR".into())
    );
    let module = session.instantiate("Функция Проверить() Экспорт Возврат Документы.Контроль.СоздатьДокумент().IsLocked(1); КонецФункции").unwrap();
    assert!(session.call(&module, "Проверить", vec![]).is_err());
    assert!(
        session
            .instantiate("Функция Проверить() Экспорт Возврат Заблокирован(); КонецФункции")
            .is_err()
    );
}

#[test]
fn local_document_lock_state_function_keeps_its_precedence() {
    let factory = factory(
        "Функция Заблокирован(ЗначениеПробы) Возврат ЗначениеПробы; КонецФункции Функция Проверить() Экспорт Возврат Заблокирован(17); КонецФункции",
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
    for name in ["Заблокирован", "IsLocked"] {
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
