use std::sync::Arc;

use bsl_config::{ConfigurationCodeSnapshot, ConfigurationModule, ModuleExecution, ModuleKind};
use bsl_orm::{
    CatalogCodeType, CatalogDefaultPresentation, CatalogStandardAttributes, MetadataCatalog,
    MetadataKind, MetadataObject, Principal,
};
use bsl_session::{Session, SessionFactory, SessionKind, SessionResources};
use open_bsl::Value;

fn session_with_source(hierarchical: Option<bool>, source: &str) -> Session {
    let module = "Справочник.ЗамерВременныхТаблиц.МодульОбъекта";
    let settings = CatalogStandardAttributes {
        code_type: CatalogCodeType::String,
        code_length: 9,
        description_length: 25,
    };
    let mut object = MetadataObject::new("catalog", "ЗамерВременныхТаблиц", MetadataKind::Catalog)
        .with_object_module(module)
        .with_catalog_standard_attributes(settings)
        .with_catalog_default_presentation(CatalogDefaultPresentation::AsDescription);
    if let Some(hierarchical) = hierarchical {
        object = object.with_catalog_hierarchical(hierarchical);
    }
    let catalog = MetadataCatalog::new(
        "deletion-mark",
        vec![
            object,
            MetadataObject::new("other", "ЗамерЧужойСсылки", MetadataKind::Catalog)
                .with_catalog_standard_attributes(settings)
                .with_catalog_default_presentation(CatalogDefaultPresentation::AsDescription),
        ],
    )
    .unwrap();
    let code = ConfigurationCodeSnapshot::new("deletion-mark", vec![
        ConfigurationModule::new("session", "МодульСеанса",
            "Процедура УстановкаПараметровСеанса(Имена) КонецПроцедуры\nФункция СоздатьПробу() Экспорт Возврат Справочники.ЗамерВременныхТаблиц.СоздатьЭлемент(); КонецФункции",
            ModuleKind::Session, ModuleExecution::new().with_server(true)),
        ConfigurationModule::new("object", module,
            source,
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

#[test]
fn catalog_lock_resolves_own_and_qualified_calls_but_fails_explicitly() {
    let mut session = session_with_source(
        Some(false),
        "Процедура Русский() Экспорт Заблокировать(); КонецПроцедуры\nПроцедура Английский() Экспорт lOcK(); КонецПроцедуры",
    );
    for method in ["Русский", "Английский", "Заблокировать", "lOcK"] {
        let module = session.instantiate(&format!("Процедура Проверить() Экспорт Справочники.ЗамерВременныхТаблиц.СоздатьЭлемент().{method}(); КонецПроцедуры")).unwrap();
        let error = session
            .call(&module, "Проверить", vec![])
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("блокировка объекта справочника пока не подключена"),
            "{error}"
        );
    }
}

#[test]
fn catalog_lock_rejects_an_argument() {
    let mut session = session_with_source(Some(false), "");
    let module = session.instantiate("Процедура Проверить() Экспорт Справочники.ЗамерВременныхТаблиц.СоздатьЭлемент().Lock(17); КонецПроцедуры").unwrap();
    let error = session.call(&module, "Проверить", vec![]).unwrap_err();
    assert!(
        format!("{error:?}").contains("MethodNotApplicable"),
        "{error:?}"
    );
}

#[test]
fn existing_catalog_lock_exports_preserve_their_arity() {
    for name in ["Заблокировать", "Lock"] {
        let mut session = session_with_source(
            Some(false),
            &format!(
                "Функция {name}(Значение) Экспорт Возврат Значение; КонецФункции\nФункция Собственный() Экспорт Возврат {name}(23); КонецФункции"
            ),
        );
        for call in ["Собственный()".to_owned(), format!("{name}(23)")] {
            let module = session.instantiate(&format!("Функция Проверить() Экспорт Возврат Справочники.ЗамерВременныхТаблиц.СоздатьЭлемент().{call}; КонецФункции")).unwrap();
            assert_eq!(
                session.call(&module, "Проверить", vec![]).unwrap().0,
                Value::number_from_i64(23)
            );
        }
    }
}
