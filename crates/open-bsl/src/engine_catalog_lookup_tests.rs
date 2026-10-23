use super::*;
use bsl_config::{
    ConfigurationCodeSnapshot, ConfigurationModule, ModuleExecution, ModuleKind, SnapshotSource,
};

#[test]
fn source_and_surface_lookup_preserve_module_identity_and_unicode_case() {
    let server = ModuleExecution::new().with_server(true);
    let source = Arc::new(SnapshotSource::new(
        ConfigurationCodeSnapshot::new(
            "lookup",
            vec![
                ConfigurationModule::new(
                    "unused",
                    "Лишний",
                    "Новый НесуществующийТип;",
                    ModuleKind::Common,
                    ModuleExecution::new().with_client(true),
                ),
                ConfigurationModule::new(
                    "target",
                    "i",
                    "Функция Тест() Экспорт Возврат 17; КонецФункции",
                    ModuleKind::Common,
                    server,
                ),
                ConfigurationModule::new(
                    "another",
                    "Второй",
                    "Функция Другой() Экспорт Возврат 23; КонецФункции",
                    ModuleKind::Common,
                    server,
                ),
            ],
        )
        .unwrap(),
    ));
    let engine = Engine::builder()
        .configuration_source(source)
        .configuration_roots(["i"])
        .build()
        .unwrap();
    let catalog = engine.indexed.as_ref().unwrap();
    let target = catalog.module_id("i").unwrap();
    assert_eq!(catalog.module_id("I"), Some(target));
    assert_eq!(catalog.module_id("ı"), Some(target));
    assert!(catalog.module_id("Нет").is_none());
    assert!(
        catalog
            .source(target.index())
            .unwrap()
            .contains("Возврат 17")
    );
    assert_eq!(
        catalog.surface(target.index()).unwrap().functions[0].name,
        "Тест"
    );
    let second = catalog.module_id("ВТОРОЙ").unwrap();
    assert!(
        catalog
            .source(second.index())
            .unwrap()
            .contains("Возврат 23")
    );
    assert_eq!(
        catalog.surface(second.index()).unwrap().functions[0].name,
        "Другой"
    );
}
