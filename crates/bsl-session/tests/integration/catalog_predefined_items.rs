use std::{cell::RefCell, rc::Rc, sync::Arc};

use bsl_config::{ConfigurationCodeSnapshot, ConfigurationModule, ModuleExecution, ModuleKind};
use bsl_orm::{
    DataAccess, MetadataCatalog, MetadataKind, MetadataObject, Principal, QueryRequest,
    QueryResultData,
};
use bsl_session::{Session, SessionFactory, SessionKind, SessionResources};
use open_bsl::{CallContext, RtResult, Value};

fn catalog() -> Arc<MetadataCatalog> {
    Arc::new(
        MetadataCatalog::new(
            "predefined",
            vec![
                MetadataObject::new("catalog", "ЗамерВременныхТаблиц", MetadataKind::Catalog)
                    .with_catalog_predefined_names(["Основной"])
                    .with_manager_module("Справочник.ЗамерВременныхТаблиц.МодульМенеджера"),
            ],
        )
        .unwrap(),
    )
}

fn factory(
    catalog: Arc<MetadataCatalog>,
    manager_source: &str,
) -> Result<SessionFactory, bsl_session::SessionError> {
    let code = ConfigurationCodeSnapshot::new("predefined", vec![
        ConfigurationModule::new("session", "МодульСеанса",
            "Процедура УстановкаПараметровСеанса(Имена) КонецПроцедуры\nФункция Прочитать() Экспорт Возврат Справочники.ЗамерВременныхТаблиц.ПрочитатьПредопределенный(); КонецФункции",
            ModuleKind::Session, ModuleExecution::new().with_server(true)),
        ConfigurationModule::new("manager", "Справочник.ЗамерВременныхТаблиц.МодульМенеджера",
            manager_source, ModuleKind::Manager, ModuleExecution::new().with_server(true))
            .with_owner("catalog"),
    ]).unwrap();
    SessionFactory::new(
        code,
        catalog,
        "МодульСеанса",
        SessionKind::ThinClient,
        "8.3.27.2342",
    )
}

struct Adapter {
    catalog: Arc<MetadataCatalog>,
    requests: RefCell<Vec<(String, String)>>,
}

struct MissingRecord;

impl DataAccess for MissingRecord {
    fn constant_value(
        &self,
        _: &MetadataObject,
        _: &Principal,
        _: &mut CallContext<'_>,
    ) -> RtResult<Value> {
        panic!("свойство менеджера не читает константу");
    }

    fn execute_query(
        &self,
        _: &QueryRequest,
        _: &Principal,
        _: &mut CallContext<'_>,
    ) -> RtResult<QueryResultData> {
        QueryResultData::new(vec!["Ссылка".into()], vec![vec![Value::Null]])
            .map_err(|error| open_bsl::RtError::DynamicError(error.to_string()))
    }
}

impl DataAccess for Adapter {
    fn constant_value(
        &self,
        _: &MetadataObject,
        _: &Principal,
        _: &mut CallContext<'_>,
    ) -> RtResult<Value> {
        panic!("свойство менеджера не читает константу");
    }

    fn execute_query(
        &self,
        request: &QueryRequest,
        principal: &Principal,
        _: &mut CallContext<'_>,
    ) -> RtResult<QueryResultData> {
        self.requests
            .borrow_mut()
            .push((principal.id().to_string(), request.text().to_string()));
        QueryResultData::new(
            vec!["Ссылка".into()],
            vec![vec![bsl_orm::catalog_reference_value(
                Arc::clone(&self.catalog),
                "catalog",
                [17; 16],
            )?]],
        )
        .map_err(|error| open_bsl::RtError::DynamicError(error.to_string()))
    }
}

fn evaluate(session: &mut Session, body: &str) -> Value {
    let module = session
        .instantiate(&format!("Функция Проверить() Экспорт {body} КонецФункции"))
        .unwrap();
    session.call(&module, "Проверить", vec![]).unwrap().0
}

#[test]
fn original_predefined_manager_property_links_and_uses_the_session_adapter() {
    let catalog = catalog();
    let factory = factory(
        Arc::clone(&catalog),
        include_str!("../../../../tests/conformance/measure/1c/catalog-predefined-manager.bsl"),
    );
    assert!(factory.is_ok(), "{:?}", factory.as_ref().err());
    let adapter = Rc::new(Adapter {
        catalog,
        requests: RefCell::new(Vec::new()),
    });
    let mut session = factory
        .unwrap()
        .open(
            Principal::new("user", "Тест", vec![]),
            SessionResources::new(Box::new(()), 1).with_data_access(adapter.clone()),
        )
        .unwrap();
    let bare = evaluate(
        &mut session,
        "Возврат Справочники.ЗамерВременныхТаблиц.ПрочитатьПредопределенный();",
    );
    let qualified = evaluate(
        &mut session,
        "Возврат Справочники.ЗамерВременныхТаблиц.оСнОвНоЙ;",
    );
    assert_eq!(bare, qualified);
    assert_eq!(bsl_orm::catalog_reference_parts(&bare).unwrap().1, [17; 16]);
    assert_eq!(
        evaluate(
            &mut session,
            "Возврат Справочники.ЗамерВременныхТаблиц.ПрочитатьПараметр(\"parameter\");"
        ),
        Value::Str("parameter".into())
    );
    assert_eq!(
        evaluate(
            &mut session,
            "Возврат Справочники.ЗамерВременныхТаблиц.ПроверитьЗапись();"
        ),
        Value::Str("rejected".into())
    );
    assert_eq!(adapter.requests.borrow().len(), 2);
    assert!(
        adapter
            .requests
            .borrow()
            .iter()
            .all(|(user, text)| user == "user"
                && text.contains("ЗНАЧЕНИЕ(Справочник.ЗамерВременныхТаблиц.Основной)"))
    );
}

#[test]
fn manager_variables_keep_their_priority_and_unknown_names_stay_unknown() {
    let catalog_value = catalog();
    let source = "Перем Основной;\nФункция ПрочитатьПредопределенный() Экспорт Возврат Основной; КонецФункции\nФункция ПрочитатьЛокальный() Экспорт Перем Основной; Основной = 19; Возврат Основной; КонецФункции\nОсновной = 17;";
    let mut session = factory(catalog_value, source)
        .unwrap()
        .open(
            Principal::new("user", "Тест", vec![]),
            SessionResources::new(Box::new(()), 1),
        )
        .unwrap();
    assert_eq!(
        evaluate(
            &mut session,
            "Возврат Справочники.ЗамерВременныхТаблиц.ПрочитатьПредопределенный();"
        ),
        Value::number_from_i64(17)
    );
    assert_eq!(
        evaluate(
            &mut session,
            "Возврат Справочники.ЗамерВременныхТаблиц.ПрочитатьЛокальный();"
        ),
        Value::number_from_i64(19)
    );
    let error = factory(
        catalog(),
        "Функция ПрочитатьПредопределенный() Экспорт Возврат НеОбъявлено; КонецФункции",
    )
    .err()
    .unwrap();
    assert!(error.to_string().contains("НеОбъявлено"));
}

#[test]
fn different_sessions_resolve_with_their_own_adapters() {
    let catalog = catalog();
    let factory = factory(
        Arc::clone(&catalog),
        include_str!("../../../../tests/conformance/measure/1c/catalog-predefined-manager.bsl"),
    )
    .unwrap();
    let first = Rc::new(Adapter {
        catalog: Arc::clone(&catalog),
        requests: RefCell::new(Vec::new()),
    });
    let second = Rc::new(Adapter {
        catalog,
        requests: RefCell::new(Vec::new()),
    });
    for (user, adapter) in [("first", &first), ("second", &second)] {
        let mut session = factory
            .open(
                Principal::new(user, user, vec![]),
                SessionResources::new(Box::new(()), 1).with_data_access(adapter.clone()),
            )
            .unwrap();
        evaluate(
            &mut session,
            "Возврат ПредопределенноеЗначение(\"Справочник.ЗамерВременныхТаблиц.Основной\");",
        );
    }
    assert_eq!(first.requests.borrow()[0].0, "first");
    assert_eq!(second.requests.borrow()[0].0, "second");
}

#[test]
fn absent_predefined_record_rejects_reading_instead_of_fabricating_a_reference() {
    let mut session = factory(
        catalog(),
        include_str!("../../../../tests/conformance/measure/1c/catalog-predefined-manager.bsl"),
    )
    .unwrap()
    .open(
        Principal::new("user", "Тест", vec![]),
        SessionResources::new(Box::new(()), 1).with_data_access(Rc::new(MissingRecord)),
    )
    .unwrap();
    let module = session.instantiate("Функция Проверить() Экспорт Возврат Справочники.ЗамерВременныхТаблиц.Основной; КонецФункции").unwrap();
    let error = session
        .call(&module, "Проверить", vec![])
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("Предопределенный элемент не найден в данных информационной базы"),
        "{error}"
    );
}

#[test]
fn predefined_property_semantics_replay_the_native_oracle_with_an_injected_record() {
    let catalog = catalog();
    let adapter = Rc::new(Adapter {
        catalog: Arc::clone(&catalog),
        requests: RefCell::new(Vec::new()),
    });
    let mut session = factory(
        catalog,
        include_str!("../../../../tests/conformance/measure/1c/catalog-predefined-manager.bsl"),
    )
    .unwrap()
    .open(
        Principal::new("user", "Тест", vec![]),
        SessionResources::new(Box::new(()), 1).with_data_access(adapter),
    )
    .unwrap();
    let checks = [
        (
            "bare",
            "Справочники.ЗамерВременныхТаблиц.ПрочитатьПредопределенный() = Справочники.ЗамерВременныхТаблиц.Основной",
        ),
        (
            "parameter",
            "Справочники.ЗамерВременныхТаблиц.ПрочитатьПараметр(\"parameter\")",
        ),
        (
            "write",
            "Справочники.ЗамерВременныхТаблиц.ПроверитьЗапись()",
        ),
        (
            "filled",
            "ЗначениеЗаполнено(Справочники.ЗамерВременныхТаблиц.Основной)",
        ),
        (
            "type",
            "ТипЗнч(Справочники.ЗамерВременныхТаблиц.Основной) = Тип(\"СправочникСсылка.ЗамерВременныхТаблиц\")",
        ),
        (
            "function",
            "ПредопределенноеЗначение(\"Справочник.ЗамерВременныхТаблиц.Основной\") = Справочники.ЗамерВременныхТаблиц.Основной",
        ),
        (
            "metadata.uuid",
            "Строка(Справочники.ЗамерВременныхТаблиц.Основной.УникальныйИдентификатор()) = \"9b6065e7-092d-4be9-bed8-5b580763721c\"",
        ),
    ];
    let actual = checks
        .into_iter()
        .map(|(name, expression)| {
            let value = evaluate(&mut session, &format!("Возврат {expression};"));
            let text = match value {
                Value::Boolean(true) => "YES".to_string(),
                Value::Boolean(false) => "NO".to_string(),
                Value::Str(text) => text.to_string(),
                other => panic!("неожиданный результат {other:?}"),
            };
            format!("predefined.{name}={text}")
        })
        .collect::<Vec<_>>();
    // UUID записи зависит от базы. Его передачу адаптером проверяет
    // отдельный тест; остальные строки воспроизводят нативный эталон.
    let expected = include_str!(
        "../../../../tests/conformance/measure/1c/measure-catalog-predefined-items.platform.txt"
    )
    .lines()
    .filter(|line| !line.starts_with("predefined.uuid="))
    .collect::<Vec<_>>();
    assert_eq!(actual, expected);
}
