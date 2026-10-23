use bsl_config::{ConfigurationCodeSnapshot, ConfigurationModule, ModuleExecution, ModuleKind};
use bsl_orm::{
    DataAccess, MetadataCatalog, MetadataObject, Principal, QueryRequest, QueryResultData,
};
use bsl_session::{SessionFactory, SessionKind, SessionResources};
use open_bsl::{CallContext, RtError, RtResult, Value};
use std::{cell::RefCell, rc::Rc, sync::Arc};

#[derive(Debug, Default)]
struct ModeObserver(RefCell<Vec<bool>>);

impl DataAccess for ModeObserver {
    fn constant_value(
        &self,
        _: &MetadataObject,
        _: &Principal,
        _: &mut CallContext<'_>,
    ) -> RtResult<Value> {
        Err(RtError::DynamicError(
            "константа в контроле не используется".into(),
        ))
    }

    fn execute_query(
        &self,
        _: &QueryRequest,
        principal: &Principal,
        context: &mut CallContext<'_>,
    ) -> RtResult<QueryResultData> {
        assert_eq!(principal.id(), "user");
        self.0.borrow_mut().push(context.privileged_mode()?);
        QueryResultData::new(["Результат".into()], [vec![Value::number_from_i64(1)]])
            .map_err(|error| RtError::DynamicError(error.to_string()))
    }
}

#[test]
fn query_callback_observes_effective_frame_mode_and_session_isolation() {
    let code = ConfigurationCodeSnapshot::new(
        "mode-control",
        vec![ConfigurationModule::new(
            "session",
            "МодульСеанса",
            "Процедура УстановкаПараметровСеанса(Имена) Экспорт КонецПроцедуры",
            ModuleKind::Session,
            ModuleExecution::new().with_server(true),
        )],
    )
    .unwrap();
    let factory = SessionFactory::new(
        code,
        Arc::new(MetadataCatalog::new("mode-control", vec![]).unwrap()),
        "МодульСеанса",
        SessionKind::ThinClient,
        "8.3.27.2342",
    )
    .unwrap();
    let observer = Rc::new(ModeObserver::default());
    let mut session = factory
        .open(
            Principal::new("user", "Тест", vec![]),
            SessionResources::new(Box::new(()), 1).with_data_access(observer.clone()),
        )
        .unwrap();
    session
        .execute(
            r#"
        Запрос = Новый Запрос("ВЫБРАТЬ 1");
        Запрос.Выполнить();
        УстановитьПривилегированныйРежим(Истина);
        Запрос.Выполнить();
        УстановитьБезопасныйРежим(Истина);
        Запрос.Выполнить();
        УстановитьОтключениеБезопасногоРежима(Истина);
        Запрос.Выполнить();
        УстановитьОтключениеБезопасногоРежима(Ложь);
        Запрос.Выполнить();
        УстановитьБезопасныйРежим(Ложь);
        Запрос.Выполнить();
        УстановитьПривилегированныйРежим(Ложь);
        Запрос.Выполнить();
    "#,
        )
        .unwrap();
    // Нативный `SECURITY.SAFE_SUPPRESSION.BALANCED` даёт `No:No`:
    // выключение подавления не восстанавливает безопасный режим в этом кадре.
    assert_eq!(
        *observer.0.borrow(),
        [false, true, false, true, true, true, false]
    );
    session
        .execute("УстановитьПривилегированныйРежим(Истина);")
        .unwrap();
    let mut other = factory
        .open(
            Principal::new("user", "Тест", vec![]),
            SessionResources::new(Box::new(()), 1).with_data_access(observer.clone()),
        )
        .unwrap();
    other
        .execute(r#"Запрос = Новый Запрос("ВЫБРАТЬ 1"); Запрос.Выполнить();"#)
        .unwrap();
    assert_eq!(observer.0.borrow().last(), Some(&false));
}
