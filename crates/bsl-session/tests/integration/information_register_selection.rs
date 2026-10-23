use std::sync::Arc;

use bsl_config::{ConfigurationCodeSnapshot, ConfigurationModule, ModuleExecution, ModuleKind};
use bsl_orm::{MetadataCatalog, MetadataKind, MetadataObject, Principal};
use bsl_session::{SessionFactory, SessionKind, SessionResources};
use open_bsl::Value;

fn factory(source: &str) -> Result<SessionFactory, bsl_session::SessionError> {
    let module = "РегистрСведений.СвойстваРасширений.МодульМенеджера";
    SessionFactory::new(
        ConfigurationCodeSnapshot::new(
            "register-factories",
            vec![
                ConfigurationModule::new(
                    "session", "МодульСеанса",
                    "Процедура УстановкаПараметровСеанса(Имена) КонецПроцедуры\nФункция Проба() Экспорт Возврат РегистрыСведений.СвойстваРасширений.Проверить(); КонецФункции",
                    ModuleKind::Session, ModuleExecution::new().with_server(true),
                ),
                ConfigurationModule::new(
                    "manager", module, source, ModuleKind::Manager,
                    ModuleExecution::new().with_server(true),
                ).with_owner("register"),
            ],
        ).unwrap(),
        Arc::new(MetadataCatalog::new("register-factories", vec![
            MetadataObject::new("register", "СвойстваРасширений", MetadataKind::InformationRegister)
                .with_manager_module(module),
        ]).unwrap()),
        "МодульСеанса", SessionKind::ThinClient, "8.3.27.2342",
    )
}

#[test]
fn selections_resolve_without_fabricating_rows() {
    let factory = factory("Функция Проверить() Экспорт Возврат 17; КонецФункции Процедура НеВызвана() Выбрать(,,Новый Структура, \"Возр\"); КонецПроцедуры Функция Собственный() Экспорт Возврат sElEcT(); КонецФункции").unwrap();
    let mut session = factory
        .open(
            Principal::new("user", "Тест", vec![]),
            SessionResources::new(Box::new(()), 1),
        )
        .unwrap();
    for name in ["Выбрать", "sElEcT"] {
        for arguments in [
            "",
            "Неопределено",
            "Неопределено, Неопределено",
            "Неопределено, Неопределено, Новый Структура",
            "Неопределено, Неопределено, Новый Структура, \"Возр\"",
        ] {
            let module = session.instantiate(&format!("Функция Вызвать() Экспорт Возврат РегистрыСведений.СвойстваРасширений.{name}({arguments}); КонецФункции")).unwrap();
            let error = session
                .call(&module, "Вызвать", vec![])
                .unwrap_err()
                .to_string();
            assert!(
                error.contains("выборка регистра сведений пока не поддерживается"),
                "{error}"
            );
        }
    }
    let module = session.instantiate("Функция Вызвать() Экспорт Возврат РегистрыСведений.СвойстваРасширений.Собственный(); КонецФункции").unwrap();
    assert!(
        session
            .call(&module, "Вызвать", vec![])
            .unwrap_err()
            .to_string()
            .contains("выборка регистра сведений пока не поддерживается")
    );
    assert!(session.instantiate("Возврат Выбрать();").is_err());
}

#[test]
fn selection_preserves_exports_and_checks_deferred_arity() {
    for name in ["Выбрать", "Select"] {
        let invalid = factory(&format!(
            "Функция Проверить() Экспорт Возврат {name}(1,2,3,4,5); КонецФункции"
        ))
        .unwrap();
        let mut session = invalid
            .open(
                Principal::new("user", "Тест", vec![]),
                SessionResources::new(Box::new(()), 1),
            )
            .unwrap();
        assert!(
            session
                .instantiate("Возврат РегистрыСведений.СвойстваРасширений.Проверить();")
                .is_err()
        );
        let existing = factory(&format!("Функция Проверить() Экспорт Возврат {name}(1,2,3,4,5); КонецФункции Функция {name}(А,Б,В,Г,Д) Экспорт Возврат А+Б+В+Г+Д; КонецФункции")).unwrap();
        let mut session = existing
            .open(
                Principal::new("user", "Тест", vec![]),
                SessionResources::new(Box::new(()), 1),
            )
            .unwrap();
        for call in ["Проверить()".to_owned(), format!("{name}(1,2,3,4,5)")] {
            let module = session.instantiate(&format!("Функция Вызвать() Экспорт Возврат РегистрыСведений.СвойстваРасширений.{call}; КонецФункции")).unwrap();
            assert_eq!(
                session.call(&module, "Вызвать", vec![]).unwrap().0,
                Value::number_from_i64(15)
            );
        }
    }
}
