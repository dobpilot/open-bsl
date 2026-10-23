use std::sync::Arc;

use bsl_config::{ConfigurationCodeSnapshot, ConfigurationModule, ModuleExecution, ModuleKind};
use bsl_orm::{MetadataCatalog, MetadataKind, MetadataObject, Principal};
use bsl_session::{SessionFactory, SessionKind, SessionResources};
use open_bsl::Value;

fn factory(source: &str) -> Result<SessionFactory, bsl_session::SessionError> {
    let module = "РегистрСведений.Проба.МодульМенеджера";
    SessionFactory::new(
        ConfigurationCodeSnapshot::new(
            "register-key",
            vec![
                ConfigurationModule::new(
                    "session", "МодульСеанса",
                    "Процедура УстановкаПараметровСеанса(Имена) КонецПроцедуры\nФункция Проба() Экспорт Возврат РегистрыСведений.Проба.Проверить(); КонецФункции",
                    ModuleKind::Session, ModuleExecution::new().with_server(true),
                ),
                ConfigurationModule::new(
                    "manager", module, source, ModuleKind::Manager,
                    ModuleExecution::new().with_server(true),
                ).with_owner("register"),
            ],
        ).unwrap(),
        Arc::new(MetadataCatalog::new("register-key", vec![
            MetadataObject::new("register", "Проба", MetadataKind::InformationRegister)
                .with_manager_module(module),
        ]).unwrap()),
        "МодульСеанса", SessionKind::ThinClient, "8.3.27.2342",
    )
}

#[test]
fn dormant_record_key_links_and_actual_retrieval_is_explicitly_unavailable() {
    let factory = factory("Функция Проверить() Экспорт Возврат 17; КонецФункции\nПроцедура НеВызвана() СоздатьКлючЗаписи(\"КлючПробы\"); CreateRecordKey(\"КлючПробы\"); КонецПроцедуры").unwrap();
    let mut session = factory
        .open(
            Principal::new("user", "Тест", vec![]),
            SessionResources::new(Box::new(()), 1),
        )
        .unwrap();
    let module = session
        .instantiate(
            "Функция Проверить() Экспорт Возврат РегистрыСведений.Проба.Проверить(); КонецФункции",
        )
        .unwrap();
    assert_eq!(
        session.call(&module, "Проверить", vec![]).unwrap().0,
        Value::number_from_i64(17)
    );
    for name in ["СоздатьКлючЗаписи", "cReAtErEcOrDkEy"] {
        let module = session.instantiate(&format!("Функция Проверить() Экспорт Возврат РегистрыСведений.Проба.{name}(\"КлючПробы\"); КонецФункции")).unwrap();
        let error = session
            .call(&module, "Проверить", vec![])
            .unwrap_err()
            .to_string();
        assert!(error.contains("СоздатьКлючЗаписи"), "{error}");
        assert!(error.contains("пока не поддерживается"), "{error}");
    }
}

#[test]
fn record_key_receiver_arity_is_finite_and_specific_to_information_registers() {
    for arguments in ["", "1, 2"] {
        let factory = factory(&format!(
            "Функция Проверить() Экспорт Возврат СоздатьКлючЗаписи({arguments}); КонецФункции"
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
                .instantiate("Возврат РегистрыСведений.Проба.Проверить();")
                .is_err()
        );
    }
    for kind in [MetadataKind::Constant, MetadataKind::AccumulationRegister] {
        assert!(
            !kind
                .receiver_methods()
                .iter()
                .any(|method| method.names().contains(&"СоздатьКлючЗаписи"))
        );
    }
}
