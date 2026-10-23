use std::sync::Arc;

use bsl_config::{ConfigurationCodeSnapshot, ConfigurationModule, ModuleExecution, ModuleKind};
use bsl_orm::{MetadataCatalog, MetadataKind, MetadataObject, Principal};
use bsl_session::{SessionFactory, SessionKind, SessionResources};
use open_bsl::Value;

fn factory(source: &str) -> Result<SessionFactory, bsl_session::SessionError> {
    let module = "РегистрНакопления.Проба.МодульМенеджера";
    SessionFactory::new(
        ConfigurationCodeSnapshot::new(
            "register-key",
            vec![
                ConfigurationModule::new(
                    "session", "МодульСеанса",
                    "Процедура УстановкаПараметровСеанса(Имена) КонецПроцедуры\nФункция Проба() Экспорт Возврат РегистрыНакопления.Проба.Проверить(); КонецФункции",
                    ModuleKind::Session, ModuleExecution::new().with_server(true),
                ),
                ConfigurationModule::new(
                    "manager", module, source, ModuleKind::Manager,
                    ModuleExecution::new().with_server(true),
                ).with_owner("register"),
            ],
        ).unwrap(),
        Arc::new(MetadataCatalog::new("register-key", vec![
            MetadataObject::new("register", "Проба", MetadataKind::AccumulationRegister)
                .with_manager_module(module),
        ]).unwrap()),
        "МодульСеанса", SessionKind::ThinClient, "8.3.27.2342",
    )
}

#[test]
fn dormant_accumulation_record_set_links_and_execution_is_explicitly_unavailable() {
    let factory = factory("Функция Проверить() Экспорт Возврат 17; КонецФункции\nПроцедура НеВызвана() СоздатьНаборЗаписей(); CreateRecordSet(); КонецПроцедуры").unwrap();
    let mut session = factory
        .open(
            Principal::new("user", "Тест", vec![]),
            SessionResources::new(Box::new(()), 1),
        )
        .unwrap();
    let module = session
        .instantiate(
            "Функция Проверить() Экспорт Возврат РегистрыНакопления.Проба.Проверить(); КонецФункции",
        )
        .unwrap();
    assert_eq!(
        session.call(&module, "Проверить", vec![]).unwrap().0,
        Value::number_from_i64(17)
    );
    for name in ["СоздатьНаборЗаписей", "cReAtErEcOrDsEt"] {
        let module = session.instantiate(&format!("Функция Проверить() Экспорт Возврат РегистрыНакопления.Проба.{name}(); КонецФункции")).unwrap();
        let error = session
            .call(&module, "Проверить", vec![])
            .unwrap_err()
            .to_string();
        assert!(error.contains("СоздатьНаборЗаписей"), "{error}");
        assert!(error.contains("пока не реализован"), "{error}");
    }
}

#[test]
fn record_set_receiver_arity_is_finite_and_specific_to_registers() {
    for arguments in ["1", "1, 2"] {
        let factory = factory(&format!(
            "Функция Проверить() Экспорт Возврат СоздатьНаборЗаписей({arguments}); КонецФункции"
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
                .instantiate("Возврат РегистрыНакопления.Проба.Проверить();")
                .is_err()
        );
    }
    for kind in [MetadataKind::Constant, MetadataKind::DataProcessor] {
        assert!(
            !kind
                .receiver_methods()
                .iter()
                .any(|method| method.names().contains(&"СоздатьНаборЗаписей"))
        );
    }
}

#[test]
fn an_existing_manager_export_keeps_its_own_arity_and_value() {
    let factory = factory("Функция Проверить() Экспорт Возврат 17; КонецФункции\nФункция СоздатьНаборЗаписей(Аргумент) Экспорт Возврат Аргумент; КонецФункции").unwrap();
    let mut session = factory
        .open(
            Principal::new("user", "Тест", vec![]),
            SessionResources::new(Box::new(()), 1),
        )
        .unwrap();
    let module = session.instantiate("Функция Проверить() Экспорт Возврат РегистрыНакопления.Проба.СоздатьНаборЗаписей(23); КонецФункции").unwrap();
    assert_eq!(
        session.call(&module, "Проверить", vec![]).unwrap().0,
        Value::number_from_i64(23)
    );
}
