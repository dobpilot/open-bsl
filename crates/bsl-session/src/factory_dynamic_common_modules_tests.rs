use super::*;
use bsl_config::{ConfigurationCodeSnapshot, ConfigurationModule, ModuleExecution, ModuleKind};
use bsl_orm::{MetadataKind, MetadataObject};

#[test]
fn factory_keeps_runtime_common_module_names_in_each_session_and_excludes_unused_managers() {
    let server = ModuleExecution::new().with_server(true);
    let code = ConfigurationCodeSnapshot::new(
        "configuration",
        vec![
            ConfigurationModule::new(
                "session",
                "МодульСеанса",
                "Процедура УстановкаПараметровСеанса(Требуемые) Экспорт
                    ПараметрыСеанса.Счетчик = 40;
                    ПараметрыСеанса.Запущен = Посредник.Вызвать(\"Цель\") + Посредник.Вызвать(\"Цель\");
                 КонецПроцедуры",
                ModuleKind::Session,
                server,
            ),
            ConfigurationModule::new(
                "broker",
                "Посредник",
                "Функция Вызвать(Имя) Экспорт Модуль = Вычислить(Имя); Возврат Модуль.Следующее(); КонецФункции",
                ModuleKind::Common,
                server,
            ),
            ConfigurationModule::new(
                "target",
                "Цель",
                "Функция Следующее() Экспорт ПараметрыСеанса.Счетчик = ПараметрыСеанса.Счетчик + 1; Возврат ПараметрыСеанса.Счетчик; КонецФункции",
                ModuleKind::Common,
                server,
            ),
            ConfigurationModule::new(
                "unused",
                "НеиспользуемыйМенеджер",
                "Функция Ошибка() Экспорт Возврат НетПеременной; КонецФункции",
                ModuleKind::Manager,
                server,
            )
            .with_owner("catalog"),
        ],
    )
    .unwrap();
    let catalog = Arc::new(
        MetadataCatalog::new(
            "configuration",
            vec![
                MetadataObject::new("counter", "Счетчик", MetadataKind::SessionParameter),
                MetadataObject::new("started", "Запущен", MetadataKind::SessionParameter),
                MetadataObject::new("catalog", "Лишний", MetadataKind::Catalog)
                    .with_manager_module("НеиспользуемыйМенеджер"),
            ],
        )
        .unwrap(),
    );
    let factory = SessionFactory::new(
        code,
        catalog,
        "МодульСеанса",
        SessionKind::ThinClient,
        "8.3.27.2342",
    )
    .unwrap();
    for number in [1, 2] {
        let session = factory
            .open(
                Principal::new("user", "Тест", vec![]),
                SessionResources::new(Box::new(()), number),
            )
            .unwrap();
        assert_eq!(
            session.parameters().peek("Запущен").unwrap(),
            Some(Value::number_from_i64(83))
        );
        assert_eq!(
            session.parameters().peek("Счетчик").unwrap(),
            Some(Value::number_from_i64(42))
        );
    }
}
