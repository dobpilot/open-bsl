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
        include_str!("../../../tests/conformance/measure/1c/catalog-folder-flag-module.bsl"),
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
        .with_catalog_default_presentation(CatalogDefaultPresentation::AsDescription);
    if let Some(hierarchical) = hierarchical {
        object = object.with_catalog_hierarchical(hierarchical);
    }
    let catalog = MetadataCatalog::new(
        "folder-flag",
        vec![
            object,
            MetadataObject::new("other", "ЗамерЧужойСсылки", MetadataKind::Catalog)
                .with_catalog_standard_attributes(settings)
                .with_catalog_default_presentation(CatalogDefaultPresentation::AsDescription),
        ],
    )
    .unwrap();
    let code = ConfigurationCodeSnapshot::new("folder-flag", vec![
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
fn new_item_folder_flag_matches_native_body_and_read_only_behavior() {
    assert_eq!(
        replay(
            &mut session(Some(false)),
            include_str!("../../../tests/conformance/measure/1c/measure-catalog-folder-flag.bsl")
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-catalog-folder-flag.platform.txt"
        )
        .trim()
    );
}

#[test]
fn explicit_variables_keep_priority_over_the_folder_flag() {
    let mut session = session_with_source(
        Some(false),
        r#"
        Перем ЭтоГруппа;
        Функция ПрочитатьПеременную() Экспорт Возврат ЭтоГруппа; КонецФункции
        Функция ПрочитатьЛокальную() Экспорт Перем ЭтоГруппа; ЭтоГруппа = 19; Возврат ЭтоГруппа; КонецФункции
        Функция ПрочитатьПараметр(ЭтоГруппа) Экспорт Возврат ЭтоГруппа; КонецФункции
        ЭтоГруппа = 17;
    "#,
    );
    let actual = replay(
        &mut session,
        r#"
        ОбъектПробы = Справочники.ЗамерВременныхТаблиц.СоздатьЭлемент();
        Сообщить(ОбъектПробы.ПрочитатьПеременную());
        Сообщить(ОбъектПробы.ПрочитатьЛокальную());
        Сообщить(ОбъектПробы.ПрочитатьПеременную());
        Сообщить(ОбъектПробы.ПрочитатьПараметр(23));
        Сообщить(?(ОбъектПробы.IsFolder, "YES", "NO"));
    "#,
    );
    assert_eq!(actual, "17\n19\n17\n23\nNO");
}

#[test]
fn dynamic_folder_flag_reads_and_assignment_use_the_same_receiver() {
    let actual = replay(
        &mut session(Some(false)),
        r#"
        ОбъектПробы = Справочники.ЗамерВременныхТаблиц.СоздатьЭлемент();
        Сообщить(?(Вычислить("ОбъектПробы.iSfOlDeR"), "YES", "NO"));
        Попытка
            Выполнить("ОбъектПробы.эТоГрУпПа = Истина;");
            ИтогПробы = "accepted";
        Исключение
            ИтогПробы = "rejected";
        КонецПопытки;
        Сообщить(ИтогПробы);
        Сообщить(?(ОбъектПробы.IsFolder, "YES", "NO"));
    "#,
    );
    assert_eq!(actual, "NO\nrejected\nNO");
}
