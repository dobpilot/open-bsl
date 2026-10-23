use std::sync::Arc;

use bsl_config::{ConfigurationCodeSnapshot, ConfigurationModule, ModuleExecution, ModuleKind};
use bsl_orm::{
    CatalogCodeType, CatalogDefaultPresentation, CatalogStandardAttributes, MetadataCatalog,
    MetadataKind, MetadataObject, Principal,
};
use bsl_session::{Session, SessionFactory, SessionKind, SessionResources};
use open_bsl::{BslObject, Value, format_value};

fn session(owners: Option<Vec<String>>) -> Session {
    let unknown = owners.is_none();
    let module = "Справочник.ЗамерВладельца.МодульОбъекта";
    let settings = CatalogStandardAttributes {
        code_type: CatalogCodeType::String,
        code_length: 9,
        description_length: 25,
    };
    let mut object = MetadataObject::new("catalog", "ЗамерВладельца", MetadataKind::Catalog)
        .with_object_module(module)
        .with_catalog_standard_attributes(settings)
        .with_catalog_default_presentation(CatalogDefaultPresentation::AsDescription);
    if let Some(owners) = owners {
        object = object.with_catalog_owners(owners);
    }
    let mut first = MetadataObject::new("first", "ВладелецПробы", MetadataKind::Catalog)
        .with_catalog_standard_attributes(settings)
        .with_catalog_default_presentation(CatalogDefaultPresentation::AsDescription);
    if !unknown {
        first = first.with_catalog_owners(vec![]);
    }
    let catalog = MetadataCatalog::new(
        "parent",
        vec![
            object,
            first,
            MetadataObject::new("second", "ВторойВладелецПробы", MetadataKind::Catalog)
                .with_catalog_standard_attributes(settings)
                .with_catalog_default_presentation(CatalogDefaultPresentation::AsDescription)
                .with_catalog_owners(vec![]),
        ],
    )
    .unwrap();
    let code = ConfigurationCodeSnapshot::new("parent", vec![
        ConfigurationModule::new("session", "МодульСеанса",
            "Процедура УстановкаПараметровСеанса(Имена) КонецПроцедуры\nФункция СоздатьПробу() Экспорт Возврат Справочники.ЗамерВладельца.СоздатьЭлемент(); КонецФункции",
            ModuleKind::Session, ModuleExecution::new().with_server(true)),
        ConfigurationModule::new("object", module,
            include_str!("../../../../tests/conformance/measure/1c/catalog-owner-module.bsl"),
            ModuleKind::Object, ModuleExecution::new().with_server(true)).with_owner("catalog"),
    ]).unwrap();
    SessionFactory::new(
        code,
        Arc::new(catalog),
        "МодульСеанса",
        SessionKind::ThinClient,
        "8.3.27.2342",
    )
    .unwrap()
    .open(
        Principal::new("user", "Тест", vec![]),
        SessionResources::new(Box::new(()), 1),
    )
    .unwrap()
}

fn replay(session: &mut Session, source: &str) -> String {
    let source = source.replace("Сообщить(", "РезультатыПробы.Добавить(");
    let module = session.instantiate(&format!("Функция Проверить() Экспорт РезультатыПробы = Новый Массив; {source}\n Возврат РезультатыПробы; КонецФункции")).unwrap();
    let Value::Object(result) = session.call(&module, "Проверить", vec![]).unwrap().0
    else {
        panic!("массив результатов");
    };
    let BslObject::Array(result) = result.as_ref() else {
        panic!("массив результатов");
    };
    result
        .borrow()
        .iter()
        .map(|value| format_value(value, None).unwrap())
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn single_owner_matches_all_native_observations() {
    assert_eq!(
        replay(
            &mut session(Some(vec!["first".into()])),
            include_str!("../../../../tests/conformance/measure/1c/measure-catalog-owner.bsl")
        ),
        include_str!("../../../../tests/conformance/measure/1c/measure-catalog-owner.platform.txt")
            .trim()
    );
}

#[test]
fn multiple_owners_match_all_native_observations() {
    assert_eq!(
        replay(
            &mut session(Some(vec!["first".into(), "second".into()])),
            include_str!(
                "../../../../tests/conformance/measure/1c/measure-catalog-owner-multiple.bsl"
            )
        ),
        include_str!(
            "../../../../tests/conformance/measure/1c/measure-catalog-owner-multiple.platform.txt"
        )
        .trim()
    );
}

#[test]
fn unknown_owner_data_does_not_prevent_unrelated_object_creation() {
    let mut session = session(None);
    assert_eq!(
        replay(
            &mut session,
            r#"
        Проба = Справочники.ВладелецПробы.СоздатьЭлемент();
        Проба.Наименование = "unrelated";
        Сообщить(Проба.Наименование);
    "#
        ),
        "unrelated"
    );
    let module = session.instantiate("Функция Проба() Экспорт Возврат Справочники.ВладелецПробы.СоздатьЭлемент().Владелец; КонецФункции").unwrap();
    let error = session.call(&module, "Проба", vec![]).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("владельцы справочника неизвестны"),
        "{error}"
    );
}

#[test]
fn owner_memory_is_isolated_between_objects_and_sessions() {
    let source = r#"
        Первый = Справочники.ЗамерВладельца.СоздатьЭлемент();
        Второй = Справочники.ЗамерВладельца.СоздатьЭлемент();
        Первый.Владелец = Справочники.ВладелецПробы.ПолучитьСсылку(Новый УникальныйИдентификатор("11111111-2222-4333-8444-555555555555"));
        Сообщить(?(ЗначениеЗаполнено(Первый.Владелец), "filled", "empty"));
        Сообщить(?(ЗначениеЗаполнено(Второй.Владелец), "filled", "empty"));
    "#;
    let mut first = session(Some(vec!["first".into()]));
    let mut second = session(Some(vec!["first".into()]));
    assert_eq!(replay(&mut first, source), "filled\nempty");
    assert_eq!(replay(&mut second, source), "filled\nempty");
}
