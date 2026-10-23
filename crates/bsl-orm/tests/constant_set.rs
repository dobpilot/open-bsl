use bsl_orm::{
    DataAccess, MetadataCatalog, MetadataKind, MetadataObject, Principal, SessionData, library,
};
use open_bsl::RtError;
use open_bsl::{CallContext, Engine, RtResult, Value};
use std::cell::{Cell, RefCell};
use std::{rc::Rc, sync::Arc};

#[derive(Debug)]
struct ReadOnly;

impl DataAccess for ReadOnly {
    fn constant_value(
        &self,
        _: &MetadataObject,
        _: &Principal,
        _: &mut CallContext<'_>,
    ) -> RtResult<Value> {
        Ok(Value::number_from_i64(17))
    }
}

fn catalog() -> Arc<MetadataCatalog> {
    Arc::new(
        MetadataCatalog::new(
            "cfg",
            vec![MetadataObject::new(
                "constant",
                "Проба",
                MetadataKind::Constant,
            )],
        )
        .unwrap(),
    )
}

#[test]
fn read_only_adapter_rejects_set_without_replacing_get() {
    let engine = Engine::builder()
        .register_library(library())
        .build()
        .unwrap();
    let session = SessionData::new(catalog(), Principal::new("user", "Тест", vec![]))
        .with_data_access(Rc::new(ReadOnly));
    let mut state = engine.state_builder().host_data(Rc::new(session)).build();
    for name in ["Установить", "sEt"] {
        let error = state
            .exec(&format!("Константы.Проба.{name}(23);"))
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("запись константы не поддерживается"),
            "{error}"
        );
        assert_eq!(
            state.exec("Возврат Константы.Проба.Получить();").unwrap(),
            Value::number_from_i64(17)
        );
    }
}

#[derive(Debug)]
struct Writer {
    value: RefCell<Value>,
    writes: Cell<usize>,
    reject: Cell<bool>,
}

impl DataAccess for Writer {
    fn constant_value(
        &self,
        _: &MetadataObject,
        _: &Principal,
        _: &mut CallContext<'_>,
    ) -> RtResult<Value> {
        Ok(self.value.borrow().clone())
    }
    fn set_constant_value(
        &self,
        metadata: &MetadataObject,
        value: &Value,
        principal: &Principal,
        _: &mut CallContext<'_>,
    ) -> RtResult<()> {
        assert_eq!(metadata.id(), "constant");
        assert_eq!(principal.id(), "user");
        self.writes.set(self.writes.get() + 1);
        if self.reject.get() {
            return Err(RtError::DynamicError("запись запрещена".into()));
        }
        *self.value.borrow_mut() = value.clone();
        Ok(())
    }
}

#[test]
fn set_delegates_to_the_current_writer_and_propagates_failure() {
    let engine = Engine::builder()
        .register_library(library())
        .build()
        .unwrap();
    let writer = Rc::new(Writer {
        value: RefCell::new(Value::number_from_i64(17)),
        writes: Cell::new(0),
        reject: Cell::new(false),
    });
    let session = SessionData::new(catalog(), Principal::new("user", "Тест", vec![]))
        .with_data_access(writer.clone());
    let mut state = engine.state_builder().host_data(Rc::new(session)).build();
    for args in ["", "1,2"] {
        assert!(
            state
                .exec(&format!("Константы.Проба.Установить({args});"))
                .is_err()
        );
    }
    assert_eq!(writer.writes.get(), 0);
    state.exec("Константы.Проба.Set(23);").unwrap();
    assert_eq!(writer.writes.get(), 1);
    assert_eq!(
        state.exec("Возврат Константы.Проба.Получить();").unwrap(),
        Value::number_from_i64(23)
    );
    writer.reject.set(true);
    let error = state
        .exec("Константы.Проба.Установить(99);")
        .unwrap_err()
        .to_string();
    assert!(error.contains("запись запрещена"), "{error}");
    assert_eq!(*writer.value.borrow(), Value::number_from_i64(23));
    let isolated = SessionData::new(catalog(), Principal::new("other", "Другой", vec![]))
        .with_data_access(Rc::new(ReadOnly));
    let mut isolated = engine.state_builder().host_data(Rc::new(isolated)).build();
    assert_eq!(
        isolated
            .exec("Возврат Константы.Проба.Получить();")
            .unwrap(),
        Value::number_from_i64(17)
    );
}

#[test]
fn a_same_named_export_keeps_its_result_and_arity() {
    let engine = Engine::builder()
        .register_library(library())
        .common_module(
            "МенеджерПробы",
            "Функция Установить(Первый,Второй) Экспорт Возврат Первый+Второй; КонецФункции",
        )
        .build()
        .unwrap();
    let catalog = Arc::new(
        MetadataCatalog::new(
            "cfg",
            vec![
                MetadataObject::new("constant", "Проба", MetadataKind::Constant)
                    .with_manager_module("МенеджерПробы"),
            ],
        )
        .unwrap(),
    );
    let session = SessionData::new(catalog, Principal::new("user", "Тест", vec![]));
    let mut state = engine.state_builder().host_data(Rc::new(session)).build();
    assert_eq!(
        state
            .exec("Возврат Константы.Проба.Установить(4,5);")
            .unwrap(),
        Value::number_from_i64(9)
    );
    assert!(state.exec("Константы.Проба.Установить(1);").is_err());
}
