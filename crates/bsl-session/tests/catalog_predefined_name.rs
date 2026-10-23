use std::sync::Arc;

use bsl_config::{ConfigurationCodeSnapshot, ConfigurationModule, ModuleExecution, ModuleKind};
use bsl_orm::{
    CatalogCodeType, CatalogDefaultPresentation, CatalogStandardAttributes, MetadataCatalog,
    MetadataKind, MetadataObject, Principal,
};
use bsl_session::{Session, SessionFactory, SessionKind, SessionResources};
use open_bsl::{BslObject, Value, format_value};

fn session(hierarchical: Option<bool>) -> Session {
    session_with_source(
        hierarchical,
        include_str!("../../../tests/conformance/measure/1c/catalog-predefined-name-module.bsl"),
    )
}

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
        .with_catalog_default_presentation(CatalogDefaultPresentation::AsDescription)
        .with_catalog_predefined_names(["Основной"]);
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
fn predefined_name_matches_native_oracle() {
    assert_eq!(
        replay(
            &mut session(Some(false)),
            include_str!(
                "../../../tests/conformance/measure/1c/measure-catalog-predefined-name.bsl"
            )
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-catalog-predefined-name.platform.txt"
        )
        .trim()
    );
}

#[test]
fn predefined_name_preserves_explicit_variable_priority() {
    let mut session = session_with_source(
        Some(false),
        r#"
        Перем ИмяПредопределенныхДанных;
        Функция Модульная() Экспорт Возврат ИмяПредопределенныхДанных; КонецФункции
        Функция Локальная() Экспорт Перем ИмяПредопределенныхДанных; ИмяПредопределенныхДанных = "local"; Возврат ИмяПредопределенныхДанных; КонецФункции
        Функция Параметр(ИмяПредопределенныхДанных) Экспорт Возврат ИмяПредопределенныхДанных; КонецФункции
        ИмяПредопределенныхДанных = "module";
    "#,
    );
    let module = session.instantiate(r#"
        Функция Проверить() Экспорт
            ОбъектПробы = Справочники.ЗамерВременныхТаблиц.СоздатьЭлемент();
            Возврат ОбъектПробы.Модульная() = "module" И ОбъектПробы.Локальная() = "local"
                И ОбъектПробы.Модульная() = "module" И ОбъектПробы.Параметр("parameter") = "parameter"
                И ОбъектПробы.PredefinedDataName = "";
        КонецФункции
    "#).unwrap();
    assert_eq!(
        session.call(&module, "Проверить", vec![]).unwrap().0,
        Value::Boolean(true)
    );
}

#[test]
fn predefined_flag_matches_native_body_name_transitions_and_write_rejection() {
    let mut session = session_with_source(
        Some(false),
        include_str!("../../../tests/conformance/measure/1c/catalog-predefined-flag-module.bsl"),
    );
    let actual = replay(
        &mut session,
        include_str!("../../../tests/conformance/measure/1c/measure-catalog-predefined-flag.bsl"),
    );
    assert_eq!(
        actual.trim(),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-catalog-predefined-flag.platform.txt"
        )
        .trim()
    );
}
