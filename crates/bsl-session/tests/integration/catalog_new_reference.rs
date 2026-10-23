use std::sync::Arc;

use bsl_config::{ConfigurationCodeSnapshot, ConfigurationModule, ModuleExecution, ModuleKind};
use bsl_orm::{
    CatalogCodeType, CatalogDefaultPresentation, CatalogStandardAttributes, MetadataCatalog,
    MetadataKind, MetadataObject, Principal,
};
use bsl_session::{Session, SessionFactory, SessionKind, SessionResources};
use open_bsl::{BslObject, Value, format_value};

fn session() -> Session {
    let module = "Справочник.ЗамерВременныхТаблиц.МодульОбъекта";
    let settings = CatalogStandardAttributes {
        code_type: CatalogCodeType::String,
        code_length: 9,
        description_length: 25,
    };
    let object = MetadataObject::new("catalog", "ЗамерВременныхТаблиц", MetadataKind::Catalog)
        .with_object_module(module)
        .with_catalog_standard_attributes(settings)
        .with_catalog_default_presentation(CatalogDefaultPresentation::AsDescription);
    let catalog = MetadataCatalog::new(
        "new-reference",
        vec![
            object,
            MetadataObject::new("document", "ЗамерДокумента", MetadataKind::Document),
            MetadataObject::new("other", "ЗамерЧужойСсылки", MetadataKind::Catalog)
                .with_catalog_standard_attributes(settings)
                .with_catalog_default_presentation(CatalogDefaultPresentation::AsDescription),
        ],
    )
    .unwrap();
    let code = ConfigurationCodeSnapshot::new("new-reference", vec![
        ConfigurationModule::new("session", "МодульСеанса",
            "Процедура УстановкаПараметровСеанса(Имена) КонецПроцедуры\nФункция СоздатьПробу() Экспорт Возврат Справочники.ЗамерВременныхТаблиц.СоздатьЭлемент(); КонецФункции",
            ModuleKind::Session, ModuleExecution::new().with_server(true)),
        ConfigurationModule::new("object", module,
            include_str!("../../../../tests/conformance/measure/1c/catalog-new-reference-module.bsl"),
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
fn staged_new_reference_matches_the_native_oracle() {
    assert_eq!(
        replay(
            &mut session(),
            include_str!(
                "../../../../tests/conformance/measure/1c/measure-catalog-new-reference.bsl"
            )
        ),
        include_str!(
            "../../../../tests/conformance/measure/1c/measure-catalog-new-reference.platform.txt"
        )
        .trim()
    );
}

#[test]
fn dynamic_reference_methods_share_the_instance_and_reject_invalid_arity() {
    let actual = replay(
        &mut session(),
        r#"
        ОбъектПробы = Справочники.ЗамерВременныхТаблиц.СоздатьЭлемент();
        ЭталонПробы = Справочники.ЗамерВременныхТаблиц.ПолучитьСсылку(Новый УникальныйИдентификатор("11111111-1111-4111-8111-111111111111"));
        Выполнить("ОбъектПробы.sEtNeWoBjEcTrEf(ЭталонПробы);");
        Сообщить("dynamic=" + ?(Вычислить("ОбъектПробы.gEtNeWoBjEcTrEf()") = ЭталонПробы, "YES", "NO"));
        Попытка
            ЗначениеПробы = Вычислить("ОбъектПробы.GetNewObjectRef(17)");
            ИтогПробы = "accepted";
        Исключение
            ИтогПробы = "rejected";
        КонецПопытки;
        Сообщить("get.arity=" + ИтогПробы);
        Попытка
            Выполнить("ОбъектПробы.SetNewObjectRef();");
            ИтогПробы = "accepted";
        Исключение
            ИтогПробы = "rejected";
        КонецПопытки;
        Сообщить("set.zero=" + ИтогПробы);
        Попытка
            Выполнить("ОбъектПробы.SetNewObjectRef(ЭталонПробы, ЭталонПробы);");
            ИтогПробы = "accepted";
        Исключение
            ИтогПробы = "rejected";
        КонецПопытки;
        Сообщить("set.two=" + ИтогПробы);
        Сообщить("retained=" + ?(ОбъектПробы.ПолучитьСсылкуНового() = ЭталонПробы, "YES", "NO"));
    "#,
    );
    assert_eq!(
        actual,
        "dynamic=YES\nget.arity=rejected\nset.zero=rejected\nset.two=rejected\nretained=YES"
    );
}
