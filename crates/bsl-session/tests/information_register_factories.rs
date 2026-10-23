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
fn dormant_factories_link_but_execute_with_named_unavailable_errors() {
    let factory = factory("Функция Проверить() Экспорт Возврат 17; КонецФункции\nПроцедура НеВызвана() СоздатьМенеджерЗаписи(); CreateRecordSet(); КонецПроцедуры").unwrap();
    let mut session = factory
        .open(
            Principal::new("user", "Тест", vec![]),
            SessionResources::new(Box::new(()), 1),
        )
        .unwrap();
    let module = session.instantiate("Функция Проверить() Экспорт Возврат РегистрыСведений.СвойстваРасширений.Проверить(); КонецФункции").unwrap();
    assert_eq!(
        session.call(&module, "Проверить", vec![]).unwrap().0,
        Value::number_from_i64(17)
    );
    for (name, expected) in [
        ("СоздатьМенеджерЗаписи", "СоздатьМенеджерЗаписи"),
        ("cReAtErEcOrDmAnAgEr", "СоздатьМенеджерЗаписи"),
        ("СоздатьНаборЗаписей", "СоздатьНаборЗаписей"),
        ("cReAtErEcOrDsEt", "СоздатьНаборЗаписей"),
    ] {
        let module = session.instantiate(&format!("Функция Проверить() Экспорт Возврат РегистрыСведений.СвойстваРасширений.{name}(); КонецФункции")).unwrap();
        let error = session
            .call(&module, "Проверить", vec![])
            .unwrap_err()
            .to_string();
        assert!(error.contains(expected), "{error}");
        assert!(error.contains("пока не реализован"), "{error}");
    }
}

#[test]
fn factory_arity_is_checked_in_the_finite_receiver_context() {
    for name in [
        "СоздатьМенеджерЗаписи",
        "CreateRecordManager",
        "СоздатьНаборЗаписей",
        "CreateRecordSet",
    ] {
        let factory = factory(&format!(
            "Функция Проверить() Экспорт Возврат {name}(1); КонецФункции"
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
                .instantiate("Возврат РегистрыСведений.СвойстваРасширений.Проверить();")
                .is_err()
        );
    }
    for kind in [MetadataKind::Catalog, MetadataKind::Constant] {
        assert!(!kind.receiver_methods().iter().any(|method| {
            method
                .names()
                .iter()
                .any(|name| *name == "СоздатьМенеджерЗаписи" || *name == "СоздатьНаборЗаписей")
        }));
    }
}

#[test]
fn slices_declare_own_and_qualified_calls_without_fabricating_rows() {
    for (russian, english) in [("СрезПервых", "sLiCeFiRsT"), ("СрезПоследних", "sLiCeLaSt")]
    {
        let factory = factory(&format!("Функция Проверить() Экспорт Возврат 17; КонецФункции Процедура НеВызвана() {russian}(); {english}(, Новый Структура); КонецПроцедуры Функция Собственный() Экспорт Возврат {russian}(); КонецФункции")).unwrap();
        let mut session = factory
            .open(
                Principal::new("user", "Тест", vec![]),
                SessionResources::new(Box::new(()), 1),
            )
            .unwrap();
        for name in [russian, english] {
            for arguments in ["", "Дата(2024, 1, 1)", "Дата(2024, 1, 1), Новый Структура"]
            {
                let probe = session.instantiate(&format!("Функция Вызвать() Экспорт Возврат РегистрыСведений.СвойстваРасширений.{name}({arguments}); КонецФункции")).unwrap();
                let error = session
                    .call(&probe, "Вызвать", vec![])
                    .unwrap_err()
                    .to_string();
                assert!(
                    error.contains(russian) && error.contains("пока не поддерживается"),
                    "{error}"
                );
            }
        }
        let probe = session.instantiate("Функция Вызвать() Экспорт Возврат РегистрыСведений.СвойстваРасширений.Собственный(); КонецФункции").unwrap();
        assert!(
            session
                .call(&probe, "Вызвать", vec![])
                .unwrap_err()
                .to_string()
                .contains("пока не поддерживается")
        );
        assert!(
            session
                .instantiate(&format!("Возврат {russian}();"))
                .is_err()
        );
    }
}

#[test]
fn slices_keep_deferred_arity_and_existing_manager_exports() {
    for name in ["СрезПервых", "SliceFirst", "СрезПоследних", "SliceLast"] {
        let invalid = factory(&format!(
            "Функция Проверить() Экспорт Возврат {name}(1, 2, 3); КонецФункции"
        ))
        .unwrap();
        let mut session = invalid
            .open(
                Principal::new("user", "Тест", vec![]),
                SessionResources::new(Box::new(()), 1),
            )
            .unwrap();
        let error = session
            .instantiate("Возврат РегистрыСведений.СвойстваРасширений.Проверить();")
            .err()
            .unwrap()
            .to_string();
        assert!(error.contains("аргумент"), "{error}");
        let existing = factory(&format!("Функция Проверить() Экспорт Возврат {name}(1, 2, 3); КонецФункции Функция {name}(А, Б, В) Экспорт Возврат А + Б + В; КонецФункции")).unwrap();
        let mut session = existing
            .open(
                Principal::new("user", "Тест", vec![]),
                SessionResources::new(Box::new(()), 1),
            )
            .unwrap();
        for method in ["Проверить", name] {
            let arguments = if method == "Проверить" {
                ""
            } else {
                "1, 2, 3"
            };
            let probe = session.instantiate(&format!("Функция Вызвать() Экспорт Возврат РегистрыСведений.СвойстваРасширений.{method}({arguments}); КонецФункции")).unwrap();
            assert_eq!(
                session.call(&probe, "Вызвать", vec![]).unwrap().0,
                Value::number_from_i64(6)
            );
        }
    }
}
