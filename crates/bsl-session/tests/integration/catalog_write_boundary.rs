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
fn original_write_procedure_compiles_but_persistence_fails_explicitly() {
    let mut session = session_with_source(
        Some(false),
        "Процедура Отключить() Экспорт Записать(); КонецПроцедуры",
    );
    for method in ["Отключить", "Write", "зАпИсАтЬ"] {
        let module = session.instantiate(&format!("Функция Проверить() Экспорт Возврат Справочники.ЗамерВременныхТаблиц.СоздатьЭлемент().{method}(); КонецФункции")).unwrap();
        assert!(
            session
                .call(&module, "Проверить", vec![])
                .unwrap_err()
                .to_string()
                .contains("запись объекта справочника в информационную базу пока не подключена")
        );
    }
    let module = session.instantiate("Функция Проверить() Экспорт ОбъектПробы = Справочники.ЗамерВременныхТаблиц.СоздатьЭлемент(); Попытка ОбъектПробы.Write(); Исключение КонецПопытки; Возврат ОбъектПробы.ЭтоНовый() И ОбъектПробы.Ссылка.Пустая(); КонецФункции").unwrap();
    assert_eq!(
        session.call(&module, "Проверить", vec![]).unwrap().0,
        Value::Boolean(true)
    );
}

#[test]
fn catalog_write_rejects_an_argument() {
    let mut session = session_with_source(Some(false), "");
    let module = session.instantiate("Функция Проверить() Экспорт Справочники.ЗамерВременныхТаблиц.СоздатьЭлемент().Write(17); КонецФункции").unwrap();
    let error = session.call(&module, "Проверить", vec![]).unwrap_err();
    assert!(
        format!("{error:?}").contains("MethodNotApplicable"),
        "{error:?}"
    );
}

#[test]
fn existing_write_export_is_inspected_without_executing_it() {
    let mut session = session_with_source(
        Some(false),
        "Перем Счётчик; Функция Записать() Экспорт Счётчик = Счётчик + 1; Возврат 23; КонецФункции Функция ПрочитатьСчётчик() Экспорт Возврат Счётчик; КонецФункции Счётчик = 0;",
    );
    let module = session.instantiate("Функция Проверить() Экспорт ОбъектПробы = Справочники.ЗамерВременныхТаблиц.СоздатьЭлемент(); Возврат ОбъектПробы.зАпИсАтЬ() = 23 И ОбъектПробы.ПрочитатьСчётчик() = 1; КонецФункции").unwrap();
    assert_eq!(
        session.call(&module, "Проверить", vec![]).unwrap().0,
        Value::Boolean(true)
    );
}
