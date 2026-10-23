use super::*;
use bsl_config::{
    ConfigurationCodeSnapshot, ConfigurationModule, ModuleExecution, ModuleKind, SnapshotSource,
};

#[test]
fn reached_programs_share_untouched_visible_module_entries() {
    let server = ModuleExecution::new().with_server(true);
    let source = Arc::new(SnapshotSource::new(
        ConfigurationCodeSnapshot::new(
            "test",
            vec![
                ConfigurationModule::new(
                    "a",
                    "А",
                    "Функция Тест() Экспорт Возврат Б.Тест(); КонецФункции",
                    ModuleKind::Common,
                    server,
                ),
                ConfigurationModule::new(
                    "b",
                    "Б",
                    "Функция Тест() Экспорт Возврат 17; КонецФункции",
                    ModuleKind::Common,
                    server,
                ),
                ConfigurationModule::new("unused", "Лишний", "", ModuleKind::Common, server),
            ],
        )
        .unwrap(),
    ));
    let engine = Engine::builder()
        .configuration_source(source)
        .configuration_roots(["А"])
        .build()
        .unwrap();
    let catalog = engine.indexed.as_ref().unwrap();
    catalog.module_code(ModuleId::new(1)).unwrap();
    let unused_entry = |module: usize| {
        catalog.programs[module]
            .get()
            .unwrap()
            .program
            .imports
            .iter()
            .find(|import| import.alias == "Лишний")
            .unwrap()
    };
    assert!(std::ptr::eq(unused_entry(0), unused_entry(1)));
    assert!(catalog.programs[2].get().is_none());
}
