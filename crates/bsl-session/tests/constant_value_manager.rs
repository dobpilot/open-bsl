use std::sync::Arc;

use bsl_config::{ConfigurationCodeSnapshot, ConfigurationModule, ModuleExecution, ModuleKind};
use bsl_orm::{MetadataCatalog, MetadataKind, MetadataObject, Principal};
use bsl_session::{SessionFactory, SessionKind, SessionResources};

fn factory(source: &str) -> Result<SessionFactory, bsl_session::SessionError> {
    let module = "Константа.Проба.МодульМенеджера";
    SessionFactory::new(
        ConfigurationCodeSnapshot::new(
            "constant-set",
            vec![
                ConfigurationModule::new(
                    "session", "МодульСеанса",
                    "Процедура УстановкаПараметровСеанса(Имена) КонецПроцедуры\nФункция Проба() Экспорт Возврат Константы.Проба.Проверить(); КонецФункции",
                    ModuleKind::Session, ModuleExecution::new().with_server(true),
                ),
                ConfigurationModule::new(
                    "manager", module, source, ModuleKind::Manager,
                    ModuleExecution::new().with_server(true),
                ).with_owner("register"),
            ],
        ).unwrap(),
        Arc::new(MetadataCatalog::new("constant-set", vec![
            MetadataObject::new("register", "Проба", MetadataKind::Constant)
                .with_manager_module(module),
        ]).unwrap()),
        "МодульСеанса", SessionKind::ThinClient, "8.3.27.2342",
    )
}

#[test]
fn value_manager_calls_resolve_without_fabricating_instances() {
    let factory = factory("Функция Проверить() Экспорт Возврат 17; КонецФункции\nФункция СоздатьРусский() Экспорт Возврат СоздатьМенеджерЗначения(); КонецФункции\nФункция СоздатьАнглийский() Экспорт Возврат cReAtEvAlUeMaNaGeR(); КонецФункции").unwrap();
    let mut session = factory
        .open(
            Principal::new("user", "Тест", vec![]),
            SessionResources::new(Box::new(()), 1),
        )
        .unwrap();
    for expression in [
        "Константы.Проба.СоздатьРусский()",
        "Константы.Проба.СоздатьАнглийский()",
        "Константы.Проба.СоздатьМенеджерЗначения()",
        "Константы.Проба.cReAtEvAlUeMaNaGeR()",
    ] {
        let module = session
            .instantiate(&format!(
                "Функция Проверить() Экспорт Возврат {expression}; КонецФункции"
            ))
            .unwrap();
        let error = session
            .call(&module, "Проверить", vec![])
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("менеджер значения константы не поддерживается"),
            "{error}"
        );
    }
}

#[test]
fn existing_value_manager_exports_preserve_their_arity() {
    for name in ["СоздатьМенеджерЗначения", "CreateValueManager"] {
        let factory = factory(&format!("Функция {name}(Значение) Экспорт Возврат Значение; КонецФункции\nФункция Проверить() Экспорт Возврат {name}(23); КонецФункции")).unwrap();
        let mut session = factory
            .open(
                Principal::new("user", "Тест", vec![]),
                SessionResources::new(Box::new(()), 1),
            )
            .unwrap();
        for expression in [
            "Константы.Проба.Проверить()".to_owned(),
            format!("Константы.Проба.{name}(23)"),
        ] {
            let module = session
                .instantiate(&format!(
                    "Функция Проверить() Экспорт Возврат {expression}; КонецФункции"
                ))
                .unwrap();
            assert_eq!(
                session.call(&module, "Проверить", vec![]).unwrap().0,
                open_bsl::Value::number_from_i64(23)
            );
        }
    }
}

#[test]
fn own_value_manager_call_rejects_an_argument() {
    let factory =
        factory("Функция Проверить() Экспорт Возврат СоздатьМенеджерЗначения(1); КонецФункции")
            .unwrap();
    let mut session = factory
        .open(
            Principal::new("user", "Тест", vec![]),
            SessionResources::new(Box::new(()), 1),
        )
        .unwrap();
    assert!(
        session
            .instantiate("Возврат Константы.Проба.Проверить();")
            .is_err()
    );
}
