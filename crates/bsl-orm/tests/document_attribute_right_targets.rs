use bsl_orm::{
    AccessRightRequest, DataAccess, DocumentAttribute, MetadataCatalog, MetadataKind,
    MetadataObject, Principal, SessionData,
};
use open_bsl::{CallContext, Engine, RtResult, Value};
use std::{cell::Cell, rc::Rc, sync::Arc};

#[derive(Debug)]
struct RecordingAccess(Rc<Cell<usize>>);

impl DataAccess for RecordingAccess {
    fn constant_value(
        &self,
        _: &MetadataObject,
        _: &Principal,
        _: &mut CallContext<'_>,
    ) -> RtResult<Value> {
        unreachable!("проверка прав не читает константы")
    }

    fn access_right(
        &self,
        request: &AccessRightRequest,
        principal: &Principal,
        _: &mut CallContext<'_>,
    ) -> RtResult<bool> {
        assert_eq!(request.right(), "View");
        assert_eq!(request.metadata().id(), "document");
        assert_eq!(principal.id(), "user");
        let attribute = request.attribute().expect("дескриптор реквизита");
        assert_eq!(attribute.id(), "attribute");
        assert_eq!(attribute.name(), "Заполняемый");
        self.0.set(self.0.get() + 1);
        Ok(true)
    }

    fn verify_access_rights(
        &self,
        request: &AccessRightRequest,
        principal: &Principal,
        context: &mut CallContext<'_>,
    ) -> RtResult<()> {
        assert!(self.access_right(request, principal, context)?);
        Ok(())
    }
}

#[test]
fn document_attribute_target_reaches_the_current_principals_adapter() {
    let calls = Rc::new(Cell::new(0));
    let catalog = Arc::new(
        MetadataCatalog::new(
            "attribute-rights",
            vec![
                MetadataObject::new("document", "Проба", MetadataKind::Document)
                    .with_document_attributes(vec![DocumentAttribute::new(
                        "attribute",
                        "Заполняемый",
                        vec![],
                    )]),
            ],
        )
        .unwrap(),
    );
    let engine = Engine::builder()
        .register_library(bsl_orm::library())
        .build()
        .unwrap();
    let mut state = engine
        .state_builder()
        .host_data(Rc::new(
            SessionData::new(catalog, Principal::new("user", "Тест", vec![]))
                .with_data_access(Rc::new(RecordingAccess(Rc::clone(&calls)))),
        ))
        .build();
    let result = state
        .exec("Возврат ПравоДоступа(\"View\", Метаданные.Документы.Проба.Реквизиты.Заполняемый);");
    assert!(
        result.is_ok(),
        "реквизит должен дойти до адаптера: {result:?}"
    );
    assert_eq!(result.unwrap(), Value::Boolean(true));
    assert_eq!(calls.get(), 1);
    state.exec("ВыполнитьПроверкуПравДоступа(\"View\", Метаданные.Документы.Проба.Реквизиты.Заполняемый);").unwrap();
    assert_eq!(calls.get(), 2);
}

#[derive(Debug, Default)]
struct Requests(std::cell::RefCell<Vec<AccessRightRequest>>);

impl DataAccess for Requests {
    fn constant_value(
        &self,
        _: &MetadataObject,
        _: &Principal,
        _: &mut CallContext<'_>,
    ) -> RtResult<Value> {
        unreachable!("проверка прав не читает константы")
    }
    fn access_right(
        &self,
        request: &AccessRightRequest,
        _: &Principal,
        _: &mut CallContext<'_>,
    ) -> RtResult<bool> {
        self.0.borrow_mut().push(request.clone());
        Ok(true)
    }
    fn verify_access_rights(
        &self,
        request: &AccessRightRequest,
        _: &Principal,
        _: &mut CallContext<'_>,
    ) -> RtResult<()> {
        self.0.borrow_mut().push(request.clone());
        Ok(())
    }
}

fn request_state(engine: &Engine, requests: Rc<Requests>) -> open_bsl::State {
    let catalog = Arc::new(
        MetadataCatalog::new(
            "same-fingerprint",
            vec![
                MetadataObject::new("document", "Проба", MetadataKind::Document)
                    .with_document_attributes(vec![DocumentAttribute::new(
                        "attribute",
                        "Заполняемый",
                        vec![],
                    )]),
                MetadataObject::new("role", "РольПробы", MetadataKind::Role),
            ],
        )
        .unwrap(),
    );
    engine
        .state_builder()
        .host_data(Rc::new(
            SessionData::new(catalog, Principal::new("user", "Тест", vec![]))
                .with_data_access(requests),
        ))
        .build()
}

#[test]
fn attribute_requests_preserve_subject_path_identity_and_parent_targets() {
    let engine = Engine::builder()
        .register_library(bsl_orm::library())
        .build()
        .unwrap();
    let requests = Rc::new(Requests::default());
    let mut state = request_state(&engine, Rc::clone(&requests));
    state.exec(r#"
        Реквизит = Метаданные.Документы.Проба.Реквизиты.Заполняемый;
        Если Реквизит <> Метаданные.Документы.Проба.Реквизиты[0] Тогда ВызватьИсключение "Идентичность"; КонецЕсли;
        ПравоДоступа("View", Реквизит, Метаданные.Роли.РольПробы, "ПутьПробы");
        ВыполнитьПроверкуПравДоступа("View", Реквизит, "ПутьПробы");
        ПравоДоступа("Read", Метаданные.Документы.Проба);
        ПравоДоступа("View", Реквизит, ПользователиИнформационнойБазы.ТекущийПользователь());
    "#).unwrap();
    let requests = requests.0.borrow();
    assert_eq!(requests.len(), 4);
    assert!(
        matches!(requests[0].subject(), bsl_orm::AccessRightSubject::Role(id) if id.as_ref() == "role")
    );
    assert!(matches!(
        requests[1].subject(),
        bsl_orm::AccessRightSubject::CurrentPrincipal
    ));
    for request in &requests[..2] {
        assert_eq!(request.attribute().unwrap().id(), "attribute");
        assert_eq!(request.metadata().id(), "document");
        assert_eq!(request.standard_attribute(), Some("ПутьПробы"));
    }
    assert_eq!(requests[0].metadata_value(), requests[1].metadata_value());
    assert!(
        matches!(requests[3].subject(), bsl_orm::AccessRightSubject::Principal(principal) if principal.id() == "user")
    );
    assert_eq!(requests[3].attribute().unwrap().id(), "attribute");
    assert!(requests[2].attribute().is_none());
    assert_eq!(requests[2].metadata().id(), "document");
}

#[test]
fn another_catalogs_attribute_is_rejected_before_either_adapter_operation() {
    let engine = Engine::builder()
        .register_library(bsl_orm::library())
        .build()
        .unwrap();
    let requests = Rc::new(Requests::default());
    let mut first = request_state(&engine, Rc::clone(&requests));
    let foreign = first
        .exec("Возврат Метаданные.Документы.Проба.Реквизиты.Заполняемый;")
        .unwrap();
    let mut second = request_state(&engine, Rc::clone(&requests));
    let module = engine.compile(r#"
        Функция Проверить(Объект) Экспорт Возврат ПравоДоступа("View", Объект); КонецФункции
        Процедура ПроверитьОтказ(Объект) Экспорт ВыполнитьПроверкуПравДоступа("View", Объект); КонецПроцедуры
    "#).unwrap();
    let instance = second.instantiate(&module).unwrap();
    for name in ["Проверить", "ПроверитьОтказ"] {
        assert!(
            instance
                .call(&mut second, name, vec![foreign.clone()])
                .is_err()
        );
    }
    assert!(requests.0.borrow().is_empty());
}
