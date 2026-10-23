//! Обратный вызов обработчика сеанса из связанного метода запроса.

use super::*;
use bsl_config::{ConfigurationCodeSnapshot, ConfigurationModule, ModuleExecution, ModuleKind};
use bsl_orm::{DataAccess, MetadataKind, MetadataObject, QueryRequest, QueryResultData};

struct LazyQuery;

impl DataAccess for LazyQuery {
    fn constant_value(
        &self,
        _: &MetadataObject,
        _: &Principal,
        _: &mut CallContext<'_>,
    ) -> RtResult<Value> {
        Err(RtError::DynamicError("константа не запрашивалась".into()))
    }

    fn execute_query(
        &self,
        _: &QueryRequest,
        _: &Principal,
        context: &mut CallContext<'_>,
    ) -> RtResult<QueryResultData> {
        let parameters = Rc::clone(
            context
                .host_data()?
                .downcast_ref::<SessionData>()
                .unwrap()
                .extension::<SessionRuntime>()
                .unwrap()
                .parameters(),
        );
        let value = parameters.get("Ленивый", context)?;
        QueryResultData::new(vec!["Значение".into()], vec![vec![value]])
            .map_err(|failure| RtError::DynamicError(failure.to_string()))
    }
}

#[test]
fn a_linked_query_method_can_initialize_a_parameter_in_its_session() {
    let code = ConfigurationCodeSnapshot::new(
        "linked-query",
        vec![ConfigurationModule::new(
            "session",
            "МодульСеанса",
            "Процедура УстановкаПараметровСеанса(Имена)
         Если Имена <> Неопределено Тогда ПараметрыСеанса.Ленивый = 42; КонецЕсли;
         КонецПроцедуры",
            ModuleKind::Session,
            ModuleExecution::new().with_server(true),
        )],
    )
    .unwrap();
    let catalog = MetadataCatalog::new(
        "linked-query",
        vec![MetadataObject::new(
            "lazy",
            "Ленивый",
            MetadataKind::SessionParameter,
        )],
    )
    .unwrap();
    let factory = SessionFactory::new(
        code,
        Arc::new(catalog),
        "МодульСеанса",
        SessionKind::ThinClient,
        "8.3.27.2342",
    )
    .unwrap();
    let mut session = factory
        .open(
            Principal::new("user", "", vec![]),
            SessionResources::new(Box::new(()), 1).with_data_access(Rc::new(LazyQuery)),
        )
        .unwrap();
    assert_eq!(session.parameters().peek("Ленивый").unwrap(), None);
    let probe = session
        .instantiate(
            "Функция Проверка() Экспорт
         ЗапросПробы = Новый Запрос(\"ВЫБРАТЬ 42 КАК Значение\");
         ВыборкаПробы = ЗапросПробы.Выполнить().Выбрать();
         ВыборкаПробы.Следующий(); Возврат ВыборкаПробы.Значение;
         КонецФункции",
        )
        .unwrap();
    assert_eq!(
        session.call(&probe, "Проверка", vec![]).unwrap().0,
        Value::number_from_i64(42)
    );
    assert_eq!(
        session.parameters().peek("Ленивый").unwrap(),
        Some(Value::number_from_i64(42))
    );
    let other = factory
        .open(
            Principal::new("user", "", vec![]),
            SessionResources::new(Box::new(()), 2).with_data_access(Rc::new(LazyQuery)),
        )
        .unwrap();
    assert_eq!(other.parameters().peek("Ленивый").unwrap(), None);
}
