use std::sync::Arc;

use bsl_config::{ConfigurationCodeSnapshot, ConfigurationModule, ModuleExecution, ModuleKind};
use bsl_orm::{MetadataCatalog, MetadataKind, MetadataObject, Principal};
use bsl_session::{SessionFactory, SessionKind, SessionResources};
use open_bsl::Value;

fn factory(source: &str) -> Result<SessionFactory, bsl_session::SessionError> {
    let module = "РегистрСведений.Проба.МодульМенеджера";
    SessionFactory::new(
        ConfigurationCodeSnapshot::new(
            "register-get",
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
        Arc::new(MetadataCatalog::new("register-get", vec![
            MetadataObject::new("register", "Проба", MetadataKind::InformationRegister)
                .with_manager_module(module),
        ]).unwrap()),
        "МодульСеанса", SessionKind::ThinClient, "8.3.27.2342",
    )
}

#[test]
fn bare_get_uses_the_manager_receiver_and_preserves_same_named_variables() {
    let factory = factory("Перем Получить; Получить = 19; Функция Проверить() Экспорт Возврат Получить; КонецФункции\nФункция Читать() Экспорт Возврат Получить(Новый Структура(\"Измерение\",7)); КонецФункции\nФункция ЧитатьАнглийский() Экспорт Возврат gEt(Новый Структура(\"Измерение\",7)); КонецФункции").unwrap();
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
        Value::number_from_i64(19)
    );
    for name in ["Читать", "ЧитатьАнглийский"] {
        let module = session
            .instantiate(&format!(
                "Функция Проверить() Экспорт Возврат РегистрыСведений.Проба.{name}(); КонецФункции"
            ))
            .unwrap();
        let error = session
            .call(&module, "Проверить", vec![])
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("периодичность регистра неизвестна"),
            "{error}"
        );
    }
}

#[test]
fn bare_get_has_a_finite_platform_arity() {
    let factory =
        factory("Функция Проверить() Экспорт Возврат Получить(1,2,3); КонецФункции").unwrap();
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
