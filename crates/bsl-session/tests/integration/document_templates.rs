use std::sync::Arc;

use bsl_config::{ConfigurationCodeSnapshot, ConfigurationModule, ModuleExecution, ModuleKind};
use bsl_orm::{MetadataCatalog, MetadataKind, MetadataObject, Principal};
use bsl_session::{SessionFactory, SessionKind, SessionResources};
use open_bsl::Value;

fn factory(source: &str) -> Result<SessionFactory, bsl_session::SessionError> {
    let module = "Документ.ЗаказПокупателя.МодульМенеджера";
    SessionFactory::new(
        ConfigurationCodeSnapshot::new(
            "document-template",
            vec![
                ConfigurationModule::new(
                    "session", "МодульСеанса",
                    "Процедура УстановкаПараметровСеанса(Имена) КонецПроцедуры\nФункция Проба() Экспорт Возврат Документы.ЗаказПокупателя.Проверить(); КонецФункции",
                    ModuleKind::Session, ModuleExecution::new().with_server(true),
                ),
                ConfigurationModule::new(
                    "manager", module, source, ModuleKind::Manager,
                    ModuleExecution::new().with_server(true),
                ).with_owner("document"),
            ],
        ).unwrap(),
        Arc::new(MetadataCatalog::new("document-template", vec![
            MetadataObject::new("document", "ЗаказПокупателя", MetadataKind::Document)
                .with_manager_module(module),
        ]).unwrap()),
        "МодульСеанса", SessionKind::ThinClient, "8.3.27.2342",
    )
}

#[test]
fn dormant_template_links_and_actual_retrieval_is_explicitly_unavailable() {
    let factory = factory("Функция Проверить() Экспорт Возврат 17; КонецФункции\nПроцедура НеВызвана() ПолучитьМакет(\"МакетОтчета\"); GetTemplate(\"МакетОтчета\"); КонецПроцедуры").unwrap();
    let mut session = factory
        .open(
            Principal::new("user", "Тест", vec![]),
            SessionResources::new(Box::new(()), 1),
        )
        .unwrap();
    let module = session
        .instantiate("Функция Проверить() Экспорт Возврат Документы.ЗаказПокупателя.Проверить(); КонецФункции")
        .unwrap();
    assert_eq!(
        session.call(&module, "Проверить", vec![]).unwrap().0,
        Value::number_from_i64(17)
    );
    for name in ["ПолучитьМакет", "gEtTeMpLaTe"] {
        let module = session.instantiate(&format!("Функция Проверить() Экспорт Возврат Документы.ЗаказПокупателя.{name}(\"МакетОтчета\"); КонецФункции")).unwrap();
        let error = session
            .call(&module, "Проверить", vec![])
            .unwrap_err()
            .to_string();
        assert!(error.contains("ПолучитьМакет"), "{error}");
        assert!(error.contains("пока не поддерживается"), "{error}");
    }
}

#[test]
fn template_receiver_arity_is_finite_and_specific_to_documents() {
    for arguments in ["", "1, 2"] {
        let factory = factory(&format!(
            "Функция Проверить() Экспорт Возврат ПолучитьМакет({arguments}); КонецФункции"
        ))
        .unwrap();
        let mut session = factory
            .open(
                Principal::new("user", "Тест", vec![]),
                SessionResources::new(Box::new(()), 1),
            )
            .unwrap();
        assert!(
            session
                .instantiate("Возврат Документы.ЗаказПокупателя.Проверить();")
                .is_err()
        );
    }
    assert!(
        !MetadataKind::Constant
            .receiver_methods()
            .iter()
            .any(|method| method.names().contains(&"ПолучитьМакет"))
    );
}

#[test]
fn bare_template_call_uses_the_receiver_without_consuming_same_named_variables() {
    let factory = factory("Перем ПолучитьМакет; ПолучитьМакет = 19; Функция Проверить() Экспорт Возврат ПолучитьМакет; КонецФункции\nФункция ЧитатьМакет() Экспорт Возврат ПолучитьМакет(\"МакетОтчета\"); КонецФункции").unwrap();
    let mut session = factory
        .open(
            Principal::new("user", "Тест", vec![]),
            SessionResources::new(Box::new(()), 1),
        )
        .unwrap();
    let module = session.instantiate("Функция Проверить() Экспорт Возврат Документы.ЗаказПокупателя.Проверить(); КонецФункции").unwrap();
    assert_eq!(
        session.call(&module, "Проверить", vec![]).unwrap().0,
        Value::number_from_i64(19)
    );
    let module = session.instantiate("Функция Проверить() Экспорт Возврат Документы.ЗаказПокупателя.ЧитатьМакет(); КонецФункции").unwrap();
    let error = session
        .call(&module, "Проверить", vec![])
        .unwrap_err()
        .to_string();
    assert!(error.contains("Документ.ПолучитьМакет"), "{error}");
    assert!(
        session
            .instantiate("Возврат ПолучитьМакет(\"МакетОтчета\");")
            .is_err()
    );
}
