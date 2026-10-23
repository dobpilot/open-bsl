use std::sync::Arc;

use bsl_config::{ConfigurationCodeSnapshot, ConfigurationModule, ModuleExecution, ModuleKind};
use bsl_orm::{MetadataCatalog, MetadataKind, MetadataObject, Principal};
use bsl_session::{SessionFactory, SessionKind, SessionResources};
use open_bsl::Value;

fn factory(source: &str) -> SessionFactory {
    let module = "РегистрНакопления.Проба.МодульМенеджера";
    SessionFactory::new(
        ConfigurationCodeSnapshot::new(
            "register-balance",
            vec![
                ConfigurationModule::new(
                    "session",
                    "МодульСеанса",
                    "Процедура УстановкаПараметровСеанса(Имена) КонецПроцедуры Функция Проба() Экспорт Возврат РегистрыНакопления.Проба.Проверить(); КонецФункции",
                    ModuleKind::Session,
                    ModuleExecution::new().with_server(true),
                ),
                ConfigurationModule::new(
                    "manager",
                    module,
                    source,
                    ModuleKind::Manager,
                    ModuleExecution::new().with_server(true),
                )
                .with_owner("register"),
            ],
        )
        .unwrap(),
        Arc::new(
            MetadataCatalog::new(
                "register-balance",
                vec![
                    MetadataObject::new("register", "Проба", MetadataKind::AccumulationRegister)
                        .with_manager_module(module),
                ],
            )
            .unwrap(),
        ),
        "МодульСеанса",
        SessionKind::ThinClient,
        "8.3.27.2342",
    )
    .unwrap()
}

#[test]
fn bare_balance_uses_the_receiver_and_preserves_module_and_local_variables() {
    let factory = factory(
        "Перем Остатки; Остатки = 19;
        Функция Проверить() Экспорт Возврат Остатки; КонецФункции
        Функция Локальная() Экспорт Balance = 23; Возврат Balance; КонецФункции
        Функция Читать() Экспорт Возврат оСтАтКи(); КонецФункции
        Функция ЧитатьАнглийский() Экспорт Возврат bAlAnCe(); КонецФункции",
    );
    let mut session = factory
        .open(
            Principal::new("user", "Тест", vec![]),
            SessionResources::new(Box::new(()), 1),
        )
        .unwrap();
    for (name, expected) in [("Проверить", 19), ("Локальная", 23)] {
        let module = session.instantiate(&format!(
            "Функция Проверить() Экспорт Возврат РегистрыНакопления.Проба.{name}(); КонецФункции"
        )).unwrap();
        assert_eq!(
            session.call(&module, "Проверить", vec![]).unwrap().0,
            Value::number_from_i64(expected)
        );
    }
    for name in ["Читать", "ЧитатьАнглийский"] {
        let module = session.instantiate(&format!(
            "Функция Проверить() Экспорт Возврат РегистрыНакопления.Проба.{name}(); КонецФункции"
        )).unwrap();
        let error = session
            .call(&module, "Проверить", vec![])
            .unwrap_err()
            .to_string();
        assert!(error.contains("вид регистра неизвестен"), "{error}");
    }
    assert!(session.instantiate("Возврат Остатки();").is_err());
}

#[test]
fn bare_balance_rejects_more_than_four_arguments() {
    for name in ["Остатки", "Balance"] {
        let factory = factory(&format!(
            "Функция Проверить() Экспорт Возврат {name}(1,2,3,4,5); КонецФункции"
        ));
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
}
