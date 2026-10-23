use std::{rc::Rc, sync::Arc};

use bsl_orm::{
    ConfigurationExtension, ConfigurationExtensionsSnapshot, MetadataCatalog, Principal,
    SessionData,
};
use open_bsl::{Engine, Value};
use open_sdbl_db::ExtensionScope;

fn description(name: &str, digit: char, active: bool, profile: &str) -> ConfigurationExtension {
    super::project_extension_description(
        &digit.to_string().repeat(32),
        name,
        Some(active),
        Some("1.2"),
        Some("55K0byUrioB3aHqMgHSxrIug2Ls="),
        Some(true),
        Some(profile),
        Some(ExtensionScope::InfoBase),
        Some(false),
    )
    .unwrap()
}

#[test]
fn projected_descriptions_preserve_properties_and_sources_in_bsl() {
    let early = description("Раннее", '1', true, "");
    let disabled = description("Отключенное", '2', false, "Профиль");
    let late = description("Позднее", '3', true, "");
    let snapshot = ConfigurationExtensionsSnapshot::new(
        vec![early.clone(), disabled.clone(), late.clone()],
        [*early.id(), *late.id()],
        [*disabled.id()],
        Vec::new(),
    )
    .unwrap();
    let catalog = MetadataCatalog::new("projection", Vec::new())
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
    let module = engine.compile(r#"
        Все = ConfigurationExtensions.Get(, ConfigurationExtensionsSource.Database);
        Активные = РасширенияКонфигурации.Получить(, ИсточникРасширенийКонфигурации.СеансАктивные);
        Отключенные = РасширенияКонфигурации.Получить(, ИсточникРасширенийКонфигурации.СеансОтключенные);
        Если Все.Количество() <> 3 Или Активные.Количество() <> 2 Или Отключенные.Количество() <> 1 Тогда
            Возврат Ложь;
        КонецЕсли;
        Раннее = Все[0];
        Возврат Раннее.Name = "Раннее" И Все[1].Name = "Отключенное" И Все[2].Name = "Позднее"
            И Активные[0].Name = "Раннее" И Активные[1].Name = "Позднее"
            И Раннее.Version = "1.2" И Base64Строка(Раннее.HashSum) = "55K0byUrioB3aHqMgHSxrIug2Ls="
            И Раннее.SafeMode = Истина И Раннее.UsedInDistributedInfoBase = Ложь И Раннее.Active = Истина
            И Раннее.Scope = ConfigurationExtensionScope.InfoBase
            И Раннее.UUID = Новый УникальныйИдентификатор("11111111-1111-1111-1111-111111111111")
            И Отключенные[0].Name = "Отключенное" И Отключенные[0].Active = Ложь
            И Отключенные[0].SafeMode = "Профиль";
    "#).unwrap();
    assert_eq!(
        engine
            .state_builder()
            .host_data(Rc::new(session))
            .build()
            .run(&module)
            .unwrap(),
        Value::Boolean(true)
    );
}

#[test]
fn inactive_descriptions_reject_each_unknown_required_field() {
    for field in 0..8 {
        let identity = if field == 7 {
            "not-an-identity"
        } else {
            "22222222222222222222222222222222"
        };
        let error = super::project_extension_description(
            identity,
            "Неполное",
            if field == 0 { None } else { Some(false) },
            if field == 1 { None } else { Some("1.2") },
            if field == 2 {
                None
            } else {
                Some("platform-hash")
            },
            if field == 3 { None } else { Some(true) },
            if field == 4 { None } else { Some("") },
            if field == 6 {
                None
            } else {
                Some(ExtensionScope::InfoBase)
            },
            if field == 5 { None } else { Some(false) },
        )
        .unwrap_err();
        assert!(
            matches!(error, super::LoadError::IncompleteExtensionMetadata(ref name) if name == "Неполное"),
            "поле {field}: {error}"
        );
    }
}
