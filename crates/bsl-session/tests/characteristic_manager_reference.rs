use bsl_config::{ConfigurationCodeSnapshot, ConfigurationModule, ModuleExecution, ModuleKind};
use bsl_orm::{MetadataCatalog, MetadataKind, MetadataObject, Principal};
use bsl_session::{SessionFactory, SessionKind, SessionResources};
use open_bsl::Value;
use std::sync::Arc;

#[test]
fn own_characteristic_manager_resolves_reference_methods_during_startup() {
    let manager_name = "ПланВидовХарактеристик.Проба.МодульМенеджера";
    let code = ConfigurationCodeSnapshot::new("reference", vec![
        ConfigurationModule::new("session", "МодульСеанса",
            "Процедура УстановкаПараметровСеанса(Имена) ПараметрыСеанса.Проба = ПланыВидовХарактеристик.Проба.Проверить(); КонецПроцедуры",
            ModuleKind::Session, ModuleExecution::new().with_server(true)),
        ConfigurationModule::new("manager", manager_name,
            "Функция Проверить() Экспорт Если ПустаяСсылка().УникальныйИдентификатор() <> Новый УникальныйИдентификатор(\"00000000-0000-0000-0000-000000000000\") Тогда ВызватьИсключение \"непустая ссылка\"; КонецЕсли; Возврат ПолучитьСсылку(Новый УникальныйИдентификатор(\"12345678-1234-1234-1234-123456789abc\")).УникальныйИдентификатор(); КонецФункции",
            ModuleKind::Manager, ModuleExecution::new().with_server(true)).with_owner("characteristic"),
    ]).unwrap();
    let catalog = MetadataCatalog::new(
        "reference",
        vec![
            MetadataObject::new(
                "characteristic",
                "Проба",
                MetadataKind::ChartOfCharacteristicTypes,
            )
            .with_manager_module(manager_name),
            MetadataObject::new("parameter", "Проба", MetadataKind::SessionParameter),
        ],
    )
    .unwrap();
    let factory = SessionFactory::new(
        code,
        Arc::new(catalog),
        "МодульСеанса",
        SessionKind::ThinClient,
        "8.3.27.2342",
    )
    .unwrap();
    let mut session = factory
        .open(
            Principal::new("user", "Тест", vec![]),
            SessionResources::new(Box::new(()), 1),
        )
        .unwrap();
    let module = session
        .instantiate(
            "Функция Проверить() Экспорт Возврат Строка(ПараметрыСеанса.Проба); КонецФункции",
        )
        .unwrap();
    assert_eq!(
        session.call(&module, "Проверить", vec![]).unwrap().0,
        Value::Str("12345678-1234-1234-1234-123456789abc".into())
    );
}

#[test]
fn characteristic_reference_declarations_preserve_existing_two_argument_exports() {
    for name in ["ПолучитьСсылку", "GetRef", "ПустаяСсылка", "EmptyRef"] {
        let manager_name = "ПланВидовХарактеристик.Проба.МодульМенеджера";
        let code = ConfigurationCodeSnapshot::new("reference-export", vec![
            ConfigurationModule::new("session", "МодульСеанса",
                format!("Процедура УстановкаПараметровСеанса(Имена) ПараметрыСеанса.Проба = ПланыВидовХарактеристик.Проба.{name}(17, 2); КонецПроцедуры"),
                ModuleKind::Session, ModuleExecution::new().with_server(true)),
            ConfigurationModule::new("manager", manager_name,
                format!("Функция {name}(Первый, Второй) Экспорт Возврат Первый + Второй; КонецФункции"),
                ModuleKind::Manager, ModuleExecution::new().with_server(true)).with_owner("characteristic"),
        ]).unwrap();
        let catalog = MetadataCatalog::new(
            "reference-export",
            vec![
                MetadataObject::new(
                    "characteristic",
                    "Проба",
                    MetadataKind::ChartOfCharacteristicTypes,
                )
                .with_manager_module(manager_name),
                MetadataObject::new("parameter", "Проба", MetadataKind::SessionParameter),
            ],
        )
        .unwrap();
        let factory = SessionFactory::new(
            code,
            Arc::new(catalog),
            "МодульСеанса",
            SessionKind::ThinClient,
            "8.3.27.2342",
        )
        .unwrap();
        let mut session = factory
            .open(
                Principal::new("user", "Тест", vec![]),
                SessionResources::new(Box::new(()), 1),
            )
            .unwrap();
        let module = session.instantiate(&format!("Функция Проверить() Экспорт Возврат ПараметрыСеанса.Проба + ПланыВидовХарактеристик.Проба.{}(17, 2); КонецФункции", name.to_lowercase())).unwrap();
        assert_eq!(
            session.call(&module, "Проверить", vec![]).unwrap().0,
            Value::number_from_i64(38)
        );
    }
}
