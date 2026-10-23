use std::sync::Arc;

use bsl_config::{ConfigurationCodeSnapshot, ConfigurationModule, ModuleExecution, ModuleKind};
use bsl_orm::{MetadataCatalog, MetadataKind, MetadataObject, Principal};
use bsl_session::{SessionFactory, SessionKind, SessionResources};
use open_bsl::Value;

fn factory(source: &str) -> SessionFactory {
    let manager = "РегистрСведений.Контроль.МодульМенеджера";
    SessionFactory::new(
        ConfigurationCodeSnapshot::new(
            "last-record",
            vec![
                ConfigurationModule::new(
                    "session", "МодульСеанса",
                    "Процедура УстановкаПараметровСеанса(Имена) КонецПроцедуры Функция Проба() Экспорт Возврат РегистрыСведений.Контроль.Проверить(); КонецФункции",
                    ModuleKind::Session, ModuleExecution::new().with_server(true),
                ),
                ConfigurationModule::new(
                    "manager", manager, source, ModuleKind::Manager,
                    ModuleExecution::new().with_server(true),
                ).with_owner("register"),
            ],
        ).unwrap(),
        Arc::new(MetadataCatalog::new("last-record", vec![
            MetadataObject::new("register", "Контроль", MetadataKind::InformationRegister)
                .with_manager_module(manager),
        ]).unwrap()),
        "МодульСеанса", SessionKind::ThinClient, "8.3.27.2342",
    ).unwrap()
}

#[test]
fn last_record_uses_the_existing_receiver_scope_without_fabricated_data() {
    let factory = factory(
        "Функция Проверить() Экспорт Возврат 17; КонецФункции Процедура НеВызвана() ПолучитьПоследнее(); gEtLaSt(, Новый Структура); КонецПроцедуры Функция Собственный() Экспорт Возврат ПолучитьПоследнее(); КонецФункции",
    );
    let mut session = factory
        .open(
            Principal::new("user", "Тест", vec![]),
            SessionResources::new(Box::new(()), 1),
        )
        .unwrap();
    let script = session
        .instantiate(
            "Функция Вызвать() Экспорт Возврат РегистрыСведений.Контроль.Проверить(); КонецФункции",
        )
        .unwrap();
    assert_eq!(
        session.call(&script, "Вызвать", vec![]).unwrap().0,
        Value::number_from_i64(17)
    );
    for name in ["ПолучитьПоследнее", "gEtLaSt"] {
        for arguments in ["", "Дата(2024,1,1)", "Дата(2024,1,1), Новый Структура"]
        {
            let script = session.instantiate(&format!("Функция Вызвать() Экспорт Возврат РегистрыСведений.Контроль.{name}({arguments}); КонецФункции")).unwrap();
            let error = session
                .call(&script, "Вызвать", vec![])
                .unwrap_err()
                .to_string();
            assert!(
                error.contains("ПолучитьПоследнее") && error.contains("пока не поддерживается"),
                "{error}"
            );
        }
    }
    let script = session.instantiate("Функция Вызвать() Экспорт Возврат РегистрыСведений.Контроль.Собственный(); КонецФункции").unwrap();
    assert!(
        session
            .call(&script, "Вызвать", vec![])
            .unwrap_err()
            .to_string()
            .contains("пока не поддерживается")
    );
    assert!(
        session
            .instantiate("Функция Вызвать() Экспорт Возврат ПолучитьПоследнее(); КонецФункции")
            .is_err()
    );
    for kind in [
        MetadataKind::Catalog,
        MetadataKind::Constant,
        MetadataKind::Document,
    ] {
        assert!(
            !kind
                .receiver_methods()
                .iter()
                .any(|method| method.names().contains(&"ПолучитьПоследнее"))
        );
    }
}

#[test]
fn last_record_retains_existing_exports_and_deferred_arity_checks() {
    for name in ["ПолучитьПоследнее", "GetLast"] {
        let invalid = factory(&format!(
            "Функция Проверить() Экспорт Возврат {name}(1,2,3); КонецФункции"
        ));
        let mut session = invalid
            .open(
                Principal::new("user", "Тест", vec![]),
                SessionResources::new(Box::new(()), 1),
            )
            .unwrap();
        let script = session.instantiate("Функция Вызвать() Экспорт Возврат РегистрыСведений.Контроль.Проверить(); КонецФункции").unwrap();
        let error = session
            .call(&script, "Вызвать", vec![])
            .unwrap_err()
            .to_string();
        assert!(error.contains("аргумент"), "{error}");
        let existing = factory(&format!(
            "Функция Проверить() Экспорт Возврат {name}(1,2,3); КонецФункции Функция {name}(А,Б,В) Экспорт Возврат А+Б+В; КонецФункции"
        ));
        let mut session = existing
            .open(
                Principal::new("user", "Тест", vec![]),
                SessionResources::new(Box::new(()), 1),
            )
            .unwrap();
        for call in ["Проверить()".to_owned(), format!("{name}(1,2,3)")] {
            let script = session.instantiate(&format!("Функция Вызвать() Экспорт Возврат РегистрыСведений.Контроль.{call}; КонецФункции")).unwrap();
            assert_eq!(
                session.call(&script, "Вызвать", vec![]).unwrap().0,
                Value::number_from_i64(6)
            );
        }
    }
}
