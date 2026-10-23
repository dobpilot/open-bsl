use std::sync::Arc;

use bsl_config::{ConfigurationCodeSnapshot, ConfigurationModule, ModuleExecution, ModuleKind};
use bsl_orm::{MetadataCatalog, MetadataKind, MetadataObject, Principal};
use bsl_session::{Session, SessionFactory, SessionKind, SessionResources};
use open_bsl::Value;

fn session() -> Session {
    let manager = "Документ.ЗамерДокумента.МодульМенеджера";
    let catalog = Arc::new(
        MetadataCatalog::new(
            "document-reference",
            vec![
                MetadataObject::new("document", "ЗамерДокумента", MetadataKind::Document)
                    .with_manager_module(manager),
                MetadataObject::new("catalog", "ЗамерВременныхТаблиц", MetadataKind::Catalog),
            ],
        )
        .unwrap(),
    );
    let source = ConfigurationCodeSnapshot::new("document-reference", vec![
        ConfigurationModule::new("session", "МодульСеанса",
            "Процедура УстановкаПараметровСеанса(Имена) КонецПроцедуры\nФункция Проверить() Экспорт Возврат Документы.ЗамерДокумента.ПроверитьСсылку(); КонецФункции",
            ModuleKind::Session, ModuleExecution::new().with_server(true)),
        ConfigurationModule::new("manager", manager,
            include_str!("../../../tests/conformance/measure/1c/cfg-src-document-reference/Documents/ЗамерДокумента/Ext/ManagerModule.bsl"),
            ModuleKind::Manager, ModuleExecution::new().with_server(true)).with_owner("document"),
    ]).unwrap();
    SessionFactory::new(
        source,
        catalog,
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

fn evaluate(session: &mut Session, expression: &str) -> Value {
    let module = session
        .instantiate(&format!(
            "Функция Проверить() Экспорт Возврат {expression}; КонецФункции"
        ))
        .unwrap();
    session.call(&module, "Проверить", vec![]).unwrap().0
}

#[test]
fn empty_document_reference_replays_the_native_oracle() {
    let mut session = session();
    let result = evaluate(&mut session, "Документы.ЗамерДокумента.ПроверитьСсылку()");
    let Value::Object(result) = result else {
        panic!("ожидался массив")
    };
    let open_bsl::BslObject::Array(result) = result.as_ref() else {
        panic!("ожидался массив")
    };
    let actual = result
        .borrow()
        .iter()
        .enumerate()
        .map(|(index, value)| {
            format!(
                "document.empty.{index}={}",
                open_bsl::format_value(value, None).unwrap()
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    assert_eq!(
        actual,
        include_str!(
            "../../../tests/conformance/measure/1c/measure-empty-document-reference.platform.txt"
        )
        .trim()
    );
}

#[test]
fn empty_document_graph_retains_type_and_catalog_helpers_reject_it() {
    let mut session = session();
    assert_eq!(
        evaluate(
            &mut session,
            "Новый ХранилищеЗначения(Документы.ЗамерДокумента.EmptyRef()).Получить() = Документы.ЗамерДокумента.ПустаяСсылка()"
        ),
        Value::Boolean(true)
    );
    let reference = evaluate(&mut session, "Документы.ЗамерДокумента.eMpTyReF()");
    assert!(bsl_orm::catalog_reference_parts(&reference).is_none());
    let Value::Type(ty) = reference.type_of().unwrap() else {
        panic!("ожидался тип")
    };
    assert!(bsl_orm::catalog_reference_type_id(&ty).is_none());
    let catalog = Arc::new(
        MetadataCatalog::new(
            "other",
            vec![MetadataObject::new(
                "document",
                "Документ",
                MetadataKind::Document,
            )],
        )
        .unwrap(),
    );
    assert!(bsl_orm::catalog_reference_value(catalog, "document", [0; 16]).is_err());
    assert_eq!(
        evaluate(
            &mut session,
            "Документы.ЗамерДокумента.ПустаяСсылка().uUiD() = Новый УникальныйИдентификатор(\"00000000-0000-0000-0000-000000000000\")"
        ),
        Value::Boolean(true)
    );
    assert!(
        session
            .instantiate("Возврат Документы.ЗамерДокумента.ПустаяСсылка(1);")
            .is_err()
    );
}
