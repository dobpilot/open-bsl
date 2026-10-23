use bsl_config::{ConfigurationCodeSnapshot, ConfigurationModule, ModuleExecution, ModuleKind};
use bsl_orm::{MetadataCatalog, MetadataKind, MetadataObject, Principal};
use bsl_session::{SessionFactory, SessionKind, SessionResources};
use std::sync::Arc;

fn factory() -> SessionFactory {
    let code = ConfigurationCodeSnapshot::new(
        "dynamic-state",
        vec![ConfigurationModule::new(
            "session",
            "МодульСеанса",
            "Процедура УстановкаПараметровСеанса(Имена) Экспорт
                ПараметрыСеанса.Замер = КонфигурацияБазыДанныхИзмененаДинамически();
                Если ПараметрыСеанса.Замер <> DataBaseConfigurationChangedDynamically() Тогда
                    ВызватьИсключение \"aliases\";
                КонецЕсли;
            КонецПроцедуры",
            ModuleKind::Session,
            ModuleExecution::new().with_server(true),
        )],
    )
    .unwrap();
    let catalog = MetadataCatalog::new(
        "dynamic-state",
        vec![MetadataObject::new(
            "sample",
            "Замер",
            MetadataKind::SessionParameter,
        )],
    )
    .unwrap();
    SessionFactory::new(
        code,
        Arc::new(catalog),
        "МодульСеанса",
        SessionKind::ThinClient,
        "8.3.27.2342",
    )
    .unwrap()
}

#[test]
fn startup_reads_host_state_and_new_sessions_do_not_inherit_it() {
    let factory = factory();
    for changed in [true, false] {
        let mut session = factory
            .open(
                Principal::new("user", "Тест", vec![]),
                SessionResources::new(Box::new(()), 1)
                    .with_database_configuration_changed_dynamically(changed),
            )
            .unwrap();
        let expected = if changed { "Истина" } else { "Ложь" };
        session.execute(&format!(
            "Если ПараметрыСеанса.Замер <> {expected} Или DataBaseConfigurationChangedDynamically() <> {expected} Тогда ВызватьИсключение \"host session state\"; КонецЕсли;"
        )).unwrap();
    }
}

#[test]
fn an_arbitrary_host_still_must_supply_dynamic_state() {
    let error = factory()
        .open(
            Principal::new("user", "Тест", vec![]),
            SessionResources::new(Box::new(()), 1),
        )
        .err()
        .expect("ожидался отказ инициализации без ресурса");
    assert!(
        error.to_string().contains(
            "признак динамического обновления конфигурации базы данных не передан хостом"
        )
    );
}
