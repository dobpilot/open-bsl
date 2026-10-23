use bsl_orm::{MetadataCatalog, MetadataKind, MetadataObject, Principal, SessionData, library};
use open_bsl::Engine;
use std::{rc::Rc, sync::Arc};

#[test]
fn unknown_register_declarations_fail_explicitly_after_method_resolution() {
    let engine = Engine::builder()
        .register_library(library())
        .build()
        .unwrap();
    let catalog = Arc::new(
        MetadataCatalog::new(
            "cfg",
            vec![MetadataObject::new(
                "register",
                "Контроль",
                MetadataKind::InformationRegister,
            )],
        )
        .unwrap(),
    );
    let session = SessionData::new(catalog, Principal::new("user", "Тест", vec![]));
    let error = engine
        .state_builder()
        .host_data(Rc::new(session))
        .build()
        .exec("Возврат РегистрыСведений.Контроль.Получить(Новый Структура(\"Измерение\", 7));")
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("периодичность регистра неизвестна"),
        "{error}"
    );
}

use bsl_orm::{
    DataAccess, DocumentAttribute, InformationRegisterPeriodicity, QueryRequest, QueryResultData,
};
use open_bsl::{CallContext, ColumnType, RtError, RtResult, TypeId, TypeRef, Value};
use std::cell::RefCell;

#[derive(Debug, Default)]
struct Access {
    calls: RefCell<Vec<QueryRequest>>,
}

impl DataAccess for Access {
    fn constant_value(
        &self,
        _: &MetadataObject,
        _: &Principal,
        _: &mut CallContext<'_>,
    ) -> RtResult<Value> {
        Err(RtError::DynamicError("константы не используются".into()))
    }

    fn execute_query(
        &self,
        request: &QueryRequest,
        principal: &Principal,
        _: &mut CallContext<'_>,
    ) -> RtResult<QueryResultData> {
        assert_eq!(principal.id(), "user");
        self.calls.borrow_mut().push(request.clone());
        // Адаптер проверяет переданный параметр, а не разбирает BSL или SQL.
        let key = request.parameters().last().unwrap().1;
        let rows = if *key == Value::number_from_i64(7) {
            vec![vec![Value::number_from_i64(13)]]
        } else {
            vec![]
        };
        QueryResultData::new(vec!["РесурсЧтения0".into()], rows)
            .map_err(|error| RtError::DynamicError(error.to_string()))
    }
}

fn declared_catalog(periodicity: InformationRegisterPeriodicity) -> Arc<MetadataCatalog> {
    let field = |id, name, digits: &str| {
        DocumentAttribute::new(
            id,
            name,
            vec![ColumnType {
                id: TypeRef::Native(TypeId::Number),
                quals: vec![digits.into(), "0".into(), "0".into()],
            }],
        )
    };
    Arc::new(
        MetadataCatalog::new(
            "cfg",
            vec![MetadataObject::new(
                "register",
                "Контроль",
                MetadataKind::InformationRegister,
            )],
        )
        .unwrap()
        .with_information_register_metadata(
            "register",
            Some(periodicity),
            Some(vec![field("dimension", "Измерение", "5")]),
            Some(vec![field("resource", "Ресурс", "15")]),
        )
        .unwrap(),
    )
}

#[test]
fn direct_read_uses_the_principal_bound_adapter_and_numeric_conversion() {
    let engine = Engine::builder()
        .register_library(library())
        .build()
        .unwrap();
    let access = Rc::new(Access::default());
    let session = SessionData::new(
        declared_catalog(InformationRegisterPeriodicity::Nonperiodic),
        Principal::new("user", "Тест", vec![]),
    )
    .with_data_access(access.clone());
    let mut state = engine.state_builder().host_data(Rc::new(session)).build();
    for (input, normalized, resource) in [
        ("7", 7, 13),
        ("\"7\"", 7, 13),
        ("Неопределено", 0, 0),
        ("Истина", 1, 0),
        ("Ложь", 0, 0),
        ("\"не число\"", 0, 0),
        ("\"\"", 0, 0),
        ("0.6", 1, 0),
    ] {
        let result = state.exec(&format!("Возврат РегистрыСведений.Контроль.Get(Новый Структура(\"измерение\", {input})).Ресурс;")).unwrap();
        assert_eq!(result, Value::number_from_i64(resource), "{input}");
        assert_eq!(
            access
                .calls
                .borrow()
                .last()
                .unwrap()
                .parameters()
                .next()
                .unwrap()
                .1,
            &Value::number_from_i64(normalized)
        );
    }
    let calls = access.calls.borrow();
    assert_eq!(
        calls[0].text(),
        "ВЫБРАТЬ Данные.Ресурс КАК РесурсЧтения0 ИЗ РегистрСведений.Контроль КАК Данные ГДЕ Данные.Измерение = &ИзмерениеЧтения0"
    );
}

#[test]
fn invalid_filters_fail_before_access_and_period_is_not_rounded() {
    let engine = Engine::builder()
        .register_library(library())
        .build()
        .unwrap();
    let access = Rc::new(Access::default());
    let session = SessionData::new(
        declared_catalog(InformationRegisterPeriodicity::Day),
        Principal::new("user", "Тест", vec![]),
    )
    .with_data_access(access.clone());
    let mut state = engine.state_builder().host_data(Rc::new(session)).build();
    for args in [
        "",
        "Неопределено, Новый Структура(\"Измерение\", 7)",
        "Дата(2024,1,2)",
        "Дата(2024,1,2), 7",
        "Дата(2024,1,2), Новый Структура",
        "Дата(2024,1,2), Новый Структура(\"Измерение,Ресурс\",7,13)",
    ] {
        assert!(
            state
                .exec(&format!(
                    "Возврат РегистрыСведений.Контроль.Получить({args});"
                ))
                .is_err()
        );
    }
    assert!(access.calls.borrow().is_empty());
    state.exec("Возврат РегистрыСведений.Контроль.Получить(Дата(2024,1,2,12,34,56), Новый Структура(\"Измерение\",7));").unwrap();
    let calls = access.calls.borrow();
    let Value::Date(date) = calls[0].parameters().next().unwrap().1 else {
        panic!()
    };
    let date = date.to_civil();
    assert_eq!((date.hour, date.minute, date.second), (12, 34, 56));
    assert!(calls[0].text().contains("Данные.Период = &ПериодЧтения"));
}

#[test]
fn settings_reject_another_owner_kind() {
    let catalog = || {
        MetadataCatalog::new(
            "cfg",
            vec![MetadataObject::new(
                "catalog",
                "Контроль",
                MetadataKind::Catalog,
            )],
        )
        .unwrap()
    };
    assert!(
        catalog()
            .with_information_register_metadata("catalog", None, None, None)
            .is_err()
    );
    assert!(
        catalog()
            .with_information_register_metadata("absent", None, None, None)
            .is_err()
    );
}

#[test]
fn an_existing_manager_export_keeps_its_result_and_arity() {
    let engine = Engine::builder()
        .register_library(library())
        .common_module(
            "МенеджерКонтроля",
            "Функция Получить(Значение) Экспорт Возврат Значение; КонецФункции",
        )
        .build()
        .unwrap();
    let catalog = Arc::new(
        MetadataCatalog::new(
            "cfg",
            vec![
                MetadataObject::new("register", "Контроль", MetadataKind::InformationRegister)
                    .with_manager_module("МенеджерКонтроля"),
            ],
        )
        .unwrap(),
    );
    let session = SessionData::new(catalog, Principal::new("user", "Тест", vec![]));
    let mut state = engine.state_builder().host_data(Rc::new(session)).build();
    assert_eq!(
        state
            .exec("Возврат РегистрыСведений.Контроль.Получить(23);")
            .unwrap(),
        Value::number_from_i64(23)
    );
    assert!(
        state
            .exec("Возврат РегистрыСведений.Контроль.Получить();")
            .is_err()
    );
}

#[derive(Debug)]
struct Denied;

impl DataAccess for Denied {
    fn constant_value(
        &self,
        _: &MetadataObject,
        _: &Principal,
        _: &mut CallContext<'_>,
    ) -> RtResult<Value> {
        Err(RtError::DynamicError("чтение запрещено".into()))
    }

    fn execute_query(
        &self,
        _: &QueryRequest,
        principal: &Principal,
        _: &mut CallContext<'_>,
    ) -> RtResult<QueryResultData> {
        assert_eq!(principal.id(), "denied");
        Err(RtError::DynamicError("чтение запрещено".into()))
    }
}

#[test]
fn denied_read_is_not_an_empty_result_or_another_sessions_value() {
    let engine = Engine::builder()
        .register_library(library())
        .build()
        .unwrap();
    let catalog = declared_catalog(InformationRegisterPeriodicity::Nonperiodic);
    let allowed = SessionData::new(Arc::clone(&catalog), Principal::new("user", "Тест", vec![]))
        .with_data_access(Rc::new(Access::default()));
    let mut allowed = engine.state_builder().host_data(Rc::new(allowed)).build();
    let source =
        "Возврат РегистрыСведений.Контроль.Получить(Новый Структура(\"Измерение\",7)).Ресурс;";
    assert_eq!(allowed.exec(source).unwrap(), Value::number_from_i64(13));
    let denied = SessionData::new(catalog, Principal::new("denied", "Другой", vec![]))
        .with_data_access(Rc::new(Denied));
    let mut denied = engine.state_builder().host_data(Rc::new(denied)).build();
    let error = denied.exec(source).unwrap_err().to_string();
    assert!(error.contains("чтение запрещено"), "{error}");
    assert_eq!(allowed.exec(source).unwrap(), Value::number_from_i64(13));
}
