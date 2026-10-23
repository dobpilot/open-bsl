use std::sync::Arc;

use bsl_config::{ConfigurationCodeSnapshot, ConfigurationModule, ModuleExecution, ModuleKind};
use bsl_orm::{
    CatalogCodeType, CatalogDefaultPresentation, CatalogStandardAttributes, MetadataCatalog,
    MetadataKind, MetadataObject, Principal,
};
use bsl_session::{Session, SessionFactory, SessionKind, SessionResources};
use open_bsl::{BslObject, Value, format_value};

fn session(hierarchical: Option<bool>) -> Session {
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
        "parent",
        vec![
            object,
            MetadataObject::new("other", "ЗамерЧужойСсылки", MetadataKind::Catalog)
                .with_catalog_standard_attributes(settings)
                .with_catalog_default_presentation(CatalogDefaultPresentation::AsDescription),
        ],
    )
    .unwrap();
    let code = ConfigurationCodeSnapshot::new("parent", vec![
        ConfigurationModule::new("session", "МодульСеанса",
            "Процедура УстановкаПараметровСеанса(Имена) КонецПроцедуры\nФункция СоздатьПробу() Экспорт Возврат Справочники.ЗамерВременныхТаблиц.СоздатьЭлемент(); КонецФункции",
            ModuleKind::Session, ModuleExecution::new().with_server(true)),
        ConfigurationModule::new("object", module,
            include_str!("../../../../tests/conformance/measure/1c/catalog-parent-module.bsl"),
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
fn nonhierarchical_parent_matches_native_read_and_write_contract() {
    assert_eq!(
        replay(
            &mut session(Some(false)),
            include_str!("../../../../tests/conformance/measure/1c/measure-catalog-parent.bsl")
        ),
        include_str!(
            "../../../../tests/conformance/measure/1c/measure-catalog-parent.ru.platform.txt"
        )
        .trim()
    );
}

#[test]
fn hierarchical_parent_matches_native_conversion_and_selection_contract() {
    assert_eq!(replay(&mut session(Some(true)), include_str!("../../../../tests/conformance/measure/1c/measure-catalog-parent-hierarchical.bsl")),
        include_str!("../../../../tests/conformance/measure/1c/measure-catalog-parent-hierarchical.ru.platform.txt").trim());
}

#[test]
fn missing_hierarchy_rejects_writes_without_changing_parent() {
    let mut session = session(None);
    let actual = replay(
        &mut session,
        r#"
        ОбъектПробы = Справочники.ЗамерВременныхТаблиц.СоздатьЭлемент();
        Попытка
            ОбъектПробы.Родитель = Справочники.ЗамерВременныхТаблиц.ПустаяСсылка();
            ВызватьИсключение "запись без метаданных выполнена";
        Исключение
            Сообщить(КраткоеПредставлениеОшибки(ИнформацияОбОшибке()));
        КонецПопытки;
        Сообщить(ОбъектПробы.Родитель.Пустая());
    "#,
    );
    assert!(
        actual.contains("признак иерархии справочника неизвестен"),
        "{actual}"
    );
    assert!(actual.ends_with("Да"), "{actual}");
}

#[test]
fn parent_values_are_per_object_and_reference_emptiness_has_zero_arity() {
    let mut session = session(Some(true));
    let actual = replay(
        &mut session,
        r#"
        ПервыйОбъект = Справочники.ЗамерВременныхТаблиц.СоздатьЭлемент();
        ВторойОбъект = Справочники.ЗамерВременныхТаблиц.СоздатьЭлемент();
        ПервыйОбъект.Родитель = Справочники.ЗамерВременныхТаблиц.ПолучитьСсылку(Новый УникальныйИдентификатор("11111111-1111-1111-1111-111111111111"));
        Сообщить(ПервыйОбъект.Родитель.iSeMpTy());
        Сообщить(ВторойОбъект.Родитель.Пустая());
        Попытка
            ПервыйОбъект.Родитель.Пустая(1);
            Сообщить("arity=accepted");
        Исключение
            Сообщить("arity=rejected");
        КонецПопытки;
    "#,
    );
    assert_eq!(actual, "Нет\nДа\narity=rejected");
}
