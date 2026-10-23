use bsl_config::{ConfigurationCodeSnapshot, ConfigurationModule, ModuleExecution, ModuleKind};
use bsl_orm::{MetadataCatalog, MetadataKind, MetadataObject, Principal};
use bsl_session::{SessionFactory, SessionKind, SessionResources};
use open_bsl::Value;
use std::sync::Arc;

#[test]
fn reference_factory_declarations_preserve_existing_manager_exports() {
    for (kind, plural, manager) in [
        (MetadataKind::Task, "Задачи", "Задача.Проба.МодульМенеджера"),
        (
            MetadataKind::BusinessProcess,
            "БизнесПроцессы",
            "БизнесПроцесс.Проба.МодульМенеджера",
        ),
    ] {
        for name in ["ПустаяСсылка", "EmptyRef", "ПолучитьСсылку", "GetRef"]
        {
            let code = ConfigurationCodeSnapshot::new("task-export", vec![
            ConfigurationModule::new("session", "МодульСеанса", format!("Процедура УстановкаПараметровСеанса(Имена) КонецПроцедуры Функция Связать() Экспорт Возврат {plural}.Проба.{name}(17, 2); КонецФункции"), ModuleKind::Session, ModuleExecution::new().with_server(true)),
            ConfigurationModule::new("manager", manager, format!("Функция {name}(Первый, Второй) Экспорт Возврат Первый + Второй; КонецФункции"), ModuleKind::Manager, ModuleExecution::new().with_server(true)).with_owner("task"),
        ]).unwrap();
            let catalog = MetadataCatalog::new(
                "task-export",
                vec![MetadataObject::new("task", "Проба", kind).with_manager_module(manager)],
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
                .instantiate(&format!(
                    "Функция Проверить() Экспорт Возврат {plural}.Проба.{}(17, 2); КонецФункции",
                    name.to_lowercase()
                ))
                .unwrap();
            assert_eq!(
                session.call(&module, "Проверить", vec![]).unwrap().0,
                Value::number_from_i64(19)
            );
        }
    }
}
