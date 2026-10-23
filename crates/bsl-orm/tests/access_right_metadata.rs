use bsl_orm::{
    AccessRightRequest, CompatibilityMode, DataAccess, MetadataCatalog, MetadataKind,
    MetadataObject, Principal, ScriptVariant, SessionData,
};
use open_bsl::{CallContext, Engine, RtResult, Value};
use std::{cell::RefCell, rc::Rc, sync::Arc};

const ROOT: &str = "26c30ece-33b5-498a-b409-a91a046ba66b";
const CATALOG: &str = "1316b080-3b2c-501b-ace3-a1234a3fedd1";

#[derive(Debug, Default)]
struct Capture(RefCell<Vec<(MetadataKind, String, Value)>>);

impl Capture {
    fn record(&self, request: &AccessRightRequest) {
        self.0.borrow_mut().push((
            request.metadata().kind(),
            request.metadata().id().to_owned(),
            request.metadata_value().clone(),
        ));
    }
}

impl DataAccess for Capture {
    fn constant_value(
        &self,
        _: &MetadataObject,
        _: &Principal,
        _: &mut CallContext<'_>,
    ) -> RtResult<Value> {
        panic!("чтение константы не запрашивалось")
    }

    fn access_right(
        &self,
        request: &AccessRightRequest,
        _: &Principal,
        _: &mut CallContext<'_>,
    ) -> RtResult<bool> {
        self.record(request);
        Ok(true)
    }

    fn verify_access_rights(
        &self,
        request: &AccessRightRequest,
        _: &Principal,
        _: &mut CallContext<'_>,
    ) -> RtResult<()> {
        self.record(request);
        Ok(())
    }
}

fn state(engine: &Engine, root_id: bool, capture: Rc<Capture>) -> open_bsl::State {
    let mut catalog = MetadataCatalog::new(
        "same-namespace",
        vec![MetadataObject::new(
            CATALOG,
            "ВладелецПробы",
            MetadataKind::Catalog,
        )],
    )
    .unwrap()
    .with_configuration(
        "Проба",
        "1",
        ScriptVariant::Russian,
        CompatibilityMode::DontUse,
        [],
    )
    .unwrap();
    if root_id {
        catalog = catalog.with_configuration_object_id(ROOT).unwrap();
    }
    let data = SessionData::new(Arc::new(catalog), Principal::new("probe", "probe", []))
        .with_data_access(capture);
    engine.state_builder().host_data(Rc::new(data)).build()
}

fn engine() -> Engine {
    Engine::builder()
        .register_library(bsl_orm::library())
        .build()
        .unwrap()
}

#[test]
fn both_operations_preserve_the_original_root_and_object_values() {
    let engine = engine();
    let capture = Rc::new(Capture::default());
    let mut state = state(&engine, true, capture.clone());
    for (expression, right, kind, id) in [
        (
            "Метаданные",
            "СохранениеДанныхПользователя",
            MetadataKind::Configuration,
            ROOT,
        ),
        (
            "Метаданные.Справочники.ВладелецПробы",
            "Чтение",
            MetadataKind::Catalog,
            CATALOG,
        ),
    ] {
        let value = state.exec(&format!("Возврат {expression};")).unwrap();
        for operation in ["ПравоДоступа", "ВыполнитьПроверкуПравДоступа"]
        {
            let module = engine.compile(&format!(
                "Процедура Проверить(Мета) Экспорт {operation}(\"{right}\", Мета); КонецПроцедуры"
            )).unwrap();
            let instance = state.instantiate(&module).unwrap();
            instance
                .call(&mut state, "Проверить", vec![value.clone()])
                .unwrap();
            let calls = capture.0.borrow();
            let (actual_kind, actual_id, actual_value) = calls.last().unwrap();
            assert_eq!(*actual_kind, kind);
            assert_eq!(actual_id, id);
            assert!(std::ptr::eq(
                value.object_ref().unwrap(),
                actual_value.object_ref().unwrap()
            ));
        }
    }
    assert_eq!(capture.0.borrow().len(), 4);
}

#[test]
fn neither_operation_admits_a_foreign_root_or_object_with_the_same_identity() {
    let engine = engine();
    let capture = Rc::new(Capture::default());
    let mut current = state(&engine, true, capture.clone());
    let mut foreign = state(&engine, true, Rc::new(Capture::default()));
    for expression in ["Метаданные", "Метаданные.Справочники.ВладелецПробы"]
    {
        let value = foreign.exec(&format!("Возврат {expression};")).unwrap();
        for operation in ["ПравоДоступа", "ВыполнитьПроверкуПравДоступа"]
        {
            let module = engine.compile(&format!(
                "Процедура Проверить(Мета) Экспорт {operation}(\"Чтение\", Мета); КонецПроцедуры"
            )).unwrap();
            let instance = current.instantiate(&module).unwrap();
            assert!(
                instance
                    .call(&mut current, "Проверить", vec![value.clone()])
                    .is_err()
            );
        }
    }
    assert!(capture.0.borrow().is_empty());
}

#[test]
fn neither_operation_dispatches_a_root_without_its_acquired_identity() {
    let engine = engine();
    let capture = Rc::new(Capture::default());
    let mut state = state(&engine, false, capture.clone());
    for operation in ["ПравоДоступа", "ВыполнитьПроверкуПравДоступа"]
    {
        let error = state
            .exec(&format!(
                "{operation}(\"СохранениеДанныхПользователя\", Метаданные);"
            ))
            .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("идентификатор корневого объекта конфигурации не приобретён"),
            "{error:?}"
        );
    }
    assert!(capture.0.borrow().is_empty());
}
