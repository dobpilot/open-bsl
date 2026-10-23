use std::sync::Arc;

use bsl_config::{ConfigurationCodeSnapshot, ConfigurationModule, ModuleExecution, ModuleKind};
use bsl_orm::{MetadataCatalog, MetadataKind, MetadataObject, Principal};
use bsl_session::{SessionFactory, SessionKind, SessionResources};
use open_bsl::Value;

#[test]
fn a_session_factory_registers_object_aliases_separately_from_references() {
    let source = r#"
Процедура УстановкаПараметровСеанса(Имена)
КонецПроцедуры
Функция Проверить() Экспорт
    Возврат Тип("БизнесПроцессОбъект.Процесс") = Тип("businessprocessobject.Процесс")
        И Тип("BusinessProcessObject.Процесс") <> Тип("БизнесПроцессСсылка.Процесс");
КонецФункции
"#;
    let code = ConfigurationCodeSnapshot::new(
        "business-types",
        vec![ConfigurationModule::new(
            "session",
            "МодульСеанса",
            source,
            ModuleKind::Session,
            ModuleExecution::new().with_server(true),
        )],
    )
    .unwrap();
    let catalog = MetadataCatalog::new(
        "business-types",
        vec![MetadataObject::new(
            "process",
            "Процесс",
            MetadataKind::BusinessProcess,
        )],
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
    let module = session.instantiate(source).unwrap();
    assert_eq!(
        session.call(&module, "Проверить", vec![]).unwrap().0,
        Value::Boolean(true)
    );
}
