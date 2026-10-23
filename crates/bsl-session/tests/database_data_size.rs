use bsl_config::{ConfigurationCodeSnapshot, ConfigurationModule, ModuleExecution, ModuleKind};
use bsl_orm::{MetadataCatalog, MetadataKind, MetadataObject, Principal};
use bsl_session::{SessionFactory, SessionKind, SessionResources};
use std::sync::Arc;

fn factory() -> SessionFactory {
    let catalog = Arc::new(
        MetadataCatalog::new(
            "database-size",
            vec![
                MetadataObject::new(
                    "register",
                    "ХранилищеФайлов",
                    MetadataKind::InformationRegister,
                )
                .with_manager_module("РегистрСведений.ХранилищеФайлов.МодульМенеджера"),
            ],
        )
        .unwrap(),
    );
    let source = "Функция РазмерПробы() Экспорт ВключитьОбъекты = Новый Массив; ВключитьОбъекты.Добавить(Метаданные.РегистрыСведений.ХранилищеФайлов); Возврат ПолучитьРазмерДанныхБазыДанных(, ВключитьОбъекты); КонецФункции";
    let code = ConfigurationCodeSnapshot::new("database-size", vec![
        ConfigurationModule::new("session", "МодульСеанса", "Процедура УстановкаПараметровСеанса(Имена) КонецПроцедуры Функция Зависимость() Экспорт Возврат РегистрыСведений.ХранилищеФайлов.РазмерПробы(); КонецФункции", ModuleKind::Session, ModuleExecution::new().with_server(true)),
        ConfigurationModule::new("manager", "РегистрСведений.ХранилищеФайлов.МодульМенеджера", source, ModuleKind::Manager, ModuleExecution::new().with_server(true)).with_owner("register"),
    ]).unwrap();
    SessionFactory::new(
        code,
        catalog,
        "МодульСеанса",
        SessionKind::ThinClient,
        "8.3.27.2342",
    )
    .unwrap()
}
#[test]
fn original_dormant_size_call_binds_but_execution_cannot_invent_a_number() {
    let mut session = factory()
        .open(
            Principal::new("user", "Проба", vec![]),
            SessionResources::new(Box::new(()), 1),
        )
        .unwrap();
    for expression in [
        "РегистрыСведений.ХранилищеФайлов.РазмерПробы()",
        "ПолучитьРазмерДанныхБазыДанных()",
        "GetDatabaseDataSize(Неопределено)",
        "gEtDaTaBaSeDaTaSiZe(, Новый Массив)",
        "ПолучитьРазмерДанныхБазыДанных(, , Новый Массив)",
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
            error.contains(
                "ПолучитьРазмерДанныхБазыДанных: получение размера данных пока не поддерживается"
            ),
            "{error}"
        );
    }
    let error = session
        .instantiate(
            "Функция Проверить() Экспорт Возврат GetDatabaseDataSize(, , , 17); КонецФункции",
        )
        .err()
        .expect("четвёртый аргумент отвергается до исполнения")
        .to_string();
    assert!(error.contains("GetDatabaseDataSize"), "{error}");
}
