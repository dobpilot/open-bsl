use std::{rc::Rc, sync::Arc};

use bsl_orm::{
    ConfigurationExtension, ConfigurationExtensionScope, ConfigurationExtensionsSnapshot,
    ExtensionSafeMode, MetadataCatalog, Principal, SessionData,
};
use open_bsl::{Engine, Value};
use open_sdbl::metadata::ConfigurationExtensionPurpose;

#[test]
fn acquired_purpose_preserves_each_value_and_unknown_in_bsl() {
    for (purpose, member) in [
        (Some(ConfigurationExtensionPurpose::Patch), Some("Patch")),
        (
            Some(ConfigurationExtensionPurpose::Customization),
            Some("Customization"),
        ),
        (Some(ConfigurationExtensionPurpose::AddOn), Some("AddOn")),
        (None, None),
    ] {
        let description = ConfigurationExtension::new(
            [1; 16],
            "EFНеОпределяетНазначение",
            "1.2",
            "hash",
            ExtensionSafeMode::Enabled(true),
            false,
            true,
            ConfigurationExtensionScope::InfoBase,
        )
        .unwrap();
        let description = super::project_extension_purpose(description, purpose);
        let snapshot = ConfigurationExtensionsSnapshot::new(
            vec![description.clone()],
            [*description.id()],
            [],
            Vec::new(),
        )
        .unwrap();
        let catalog = MetadataCatalog::new("purpose-projection", Vec::new())
            .unwrap()
            .with_configuration_extensions(snapshot);
        let session = SessionData::new(
            Arc::new(catalog),
            Principal::new("test", "Тест", Vec::new()),
        );
        let engine = Engine::builder()
            .register_library(bsl_orm::library())
            .build()
            .unwrap();
        let source = match member {
            Some(member) => format!(
                "Расширение = РасширенияКонфигурации.Получить()[0]; Возврат Расширение.Назначение = Метаданные.СвойстваОбъектов.НазначениеРасширенияКонфигурации.{member} И Расширение.Purpose = Расширение.Назначение;"
            ),
            None => "Возврат РасширенияКонфигурации.Получить()[0].Назначение;".to_owned(),
        };
        let module = engine.compile(&source).unwrap();
        let result = engine
            .state_builder()
            .host_data(Rc::new(session))
            .build()
            .run(&module);
        if member.is_some() {
            assert_eq!(result.unwrap(), Value::Boolean(true));
        } else {
            assert!(
                result
                    .unwrap_err()
                    .to_string()
                    .contains("назначение расширения не приобретено")
            );
        }
    }
}
