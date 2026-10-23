use std::sync::Arc;

use bsl_config::{ConfigurationCodeSnapshot, ConfigurationModule, ModuleExecution, ModuleKind};
use bsl_orm::{
    CatalogCodeType, CatalogDefaultPresentation, CatalogStandardAttributes, DocumentAttribute,
    MetadataCatalog, MetadataKind, MetadataObject, Principal,
};
use bsl_session::{Session, SessionFactory, SessionKind, SessionResources};
use open_bsl::{BslObject, ColumnType, TypeId, TypeRef, Value, format_value};

fn factory(known: bool, length: &str) -> SessionFactory {
    factory_type(known, length, TypeId::String)
}

fn factory_type(known: bool, length: &str, attribute_type: TypeId) -> SessionFactory {
    let module = "Справочник.ЗамерВременныхТаблиц.МодульОбъекта";
    let settings = CatalogStandardAttributes {
        code_type: CatalogCodeType::String,
        code_length: 9,
        description_length: 25,
    };
    let mut object = MetadataObject::new("catalog", "ЗамерВременныхТаблиц", MetadataKind::Catalog)
        .with_catalog_standard_attributes(settings)
        .with_catalog_default_presentation(CatalogDefaultPresentation::AsDescription);
    if known {
        object = object
            .with_object_module(module)
            .with_catalog_common_attributes(vec![DocumentAttribute::new(
                "common",
                "НаименованиеЯзык1",
                vec![ColumnType {
                    id: TypeRef::Native(attribute_type),
                    quals: if attribute_type == TypeId::String {
                        vec![length.into(), "1".into()]
                    } else {
                        vec![]
                    },
                }],
            )]);
    }
    let catalog = MetadataCatalog::new(
        "common",
        vec![
            object,
            MetadataObject::new("excluded", "ВладелецПробы", MetadataKind::Catalog)
                .with_catalog_standard_attributes(settings)
                .with_catalog_default_presentation(CatalogDefaultPresentation::AsDescription)
                .with_catalog_common_attributes(vec![]),
        ],
    )
    .unwrap();
    let mut modules = vec![ConfigurationModule::new(
        "session",
        "МодульСеанса",
        "Процедура УстановкаПараметровСеанса(Имена) КонецПроцедуры Функция СоздатьПробу() Экспорт Возврат Справочники.ЗамерВременныхТаблиц.СоздатьЭлемент(); КонецФункции",
        ModuleKind::Session,
        ModuleExecution::new().with_server(true),
    )];
    if known {
        modules.push(
            ConfigurationModule::new(
                "object",
                module,
                include_str!(
                    "../../../../tests/conformance/measure/1c/common-attribute-runtime-module.bsl"
                ),
                ModuleKind::Object,
                ModuleExecution::new().with_server(true),
            )
            .with_owner("catalog"),
        );
    }
    SessionFactory::new(
        ConfigurationCodeSnapshot::new("common", modules).unwrap(),
        Arc::new(catalog),
        "МодульСеанса",
        SessionKind::ThinClient,
        "8.3.27.2342",
    )
    .unwrap()
}

fn session(factory: &SessionFactory) -> Session {
    factory
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
fn applied_string_common_attribute_replays_both_native_modes() {
    let factory = factory(true, "150");
    for (source, expected) in [
        (
            include_str!(
                "../../../../tests/conformance/measure/1c/measure-common-attribute-runtime.bsl"
            ),
            include_str!(
                "../../../../tests/conformance/measure/1c/measure-common-attribute-runtime.platform.txt"
            ),
        ),
        (
            include_str!(
                "../../../../tests/conformance/measure/1c/measure-common-attribute-runtime-auto.bsl"
            ),
            include_str!(
                "../../../../tests/conformance/measure/1c/measure-common-attribute-runtime-auto.platform.txt"
            ),
        ),
    ] {
        assert_eq!(replay(&mut session(&factory), source), expected.trim());
    }
}

#[test]
fn common_values_do_not_leak_between_sessions_of_the_same_factory() {
    let factory = factory(true, "150");
    let mut first = session(&factory);
    let mut second = session(&factory);
    assert_eq!(
        replay(
            &mut first,
            "Проба = Справочники.ЗамерВременныхТаблиц.СоздатьЭлемент(); Проба.НаименованиеЯзык1 = \"first\"; Сообщить(Проба.НаименованиеЯзык1);"
        ),
        "first"
    );
    assert_eq!(
        replay(
            &mut second,
            "Проба = Справочники.ЗамерВременныхТаблиц.СоздатьЭлемент(); Сообщить(?(Проба.НаименованиеЯзык1 = \"\", \"empty\", \"other\"));"
        ),
        "empty"
    );
}

#[test]
fn unknown_common_collection_preserves_unrelated_object_operations() {
    let factory = factory(false, "150");
    let mut session = session(&factory);
    assert_eq!(
        replay(
            &mut session,
            "Проба = Справочники.ЗамерВременныхТаблиц.СоздатьЭлемент(); Проба.Наименование = \"known\"; Сообщить(Проба.Наименование);"
        ),
        "known"
    );
}

#[test]
fn unsupported_common_type_fails_at_access_instead_of_fabricating_a_value() {
    let factory = factory_type(true, "150", TypeId::Boolean);
    let mut session = session(&factory);
    let module = session.instantiate("Функция Проба() Экспорт Возврат Справочники.ЗамерВременныхТаблиц.СоздатьЭлемент(); КонецФункции").unwrap();
    let error = session.call(&module, "Проба", vec![]).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("тип общего реквизита «НаименованиеЯзык1» пока не поддерживается"),
        "{error}"
    );
}
