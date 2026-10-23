use std::{cell::RefCell, rc::Rc, sync::Arc};

use bsl_config::{ConfigurationCodeSnapshot, ConfigurationModule, ModuleExecution, ModuleKind};
use bsl_orm::{
    DataAccess, DocumentAttribute, FindByAttributeRequest, MetadataCatalog, MetadataKind,
    MetadataObject, Principal,
};
use bsl_session::{Session, SessionFactory, SessionKind, SessionResources};
use open_bsl::{BslObject, CallContext, ColumnType, RtError, RtResult, TypeId, TypeRef, Value};

struct Adapter {
    value: RefCell<Value>,
    requests: RefCell<Vec<String>>,
    failed: RefCell<bool>,
}
impl DataAccess for Adapter {
    fn constant_value(
        &self,
        _: &MetadataObject,
        _: &Principal,
        _: &mut CallContext<'_>,
    ) -> RtResult<Value> {
        panic!("поиск документа не читает константу")
    }
    fn find_by_attribute(
        &self,
        metadata: &MetadataObject,
        request: &FindByAttributeRequest,
        principal: &Principal,
        _: &mut CallContext<'_>,
    ) -> RtResult<Option<Value>> {
        assert_eq!(metadata.kind(), MetadataKind::Document);
        assert_eq!(request.attribute(), "Заполняемый");
        assert!(request.parent().is_none());
        assert!(request.owner().is_none());
        self.requests.borrow_mut().push(principal.id().to_owned());
        if *self.failed.borrow() {
            return Err(RtError::DynamicError("ошибка источника".into()));
        }
        Ok(match request.value() {
            Value::Str(text) if text.to_string().trim_end_matches(' ') == "AbC" => {
                Some(self.value.borrow().clone())
            }
            _ => None,
        })
    }
}
fn factory(known: bool) -> SessionFactory {
    factory_with_identity(known, "find-document")
}

fn factory_with_identity(known: bool, identity: &str) -> SessionFactory {
    let mut document = MetadataObject::new("doc", "ЗамерДокумента", MetadataKind::Document)
        .with_manager_module("Документ.ЗамерДокумента.МодульМенеджера");
    if known {
        document = document.with_document_attributes(vec![DocumentAttribute::new(
            "attr",
            "Заполняемый",
            vec![ColumnType::plain(TypeRef::Native(TypeId::String))],
        )]);
    }
    let catalog = Arc::new(
        MetadataCatalog::new(
            identity,
            vec![
                document,
                MetadataObject::new("other", "ДругойДокумент", MetadataKind::Document),
                MetadataObject::new("catalog", "СправочникПробы", MetadataKind::Catalog),
            ],
        )
        .unwrap(),
    );
    let code = ConfigurationCodeSnapshot::new(identity, vec![
        ConfigurationModule::new("session", "МодульСеанса", "Процедура УстановкаПараметровСеанса(Имена) КонецПроцедуры Функция Связь() Экспорт Возврат Документы.ЗамерДокумента.НайтиПробу(); КонецФункции", ModuleKind::Session, ModuleExecution::new().with_server(true)),
        ConfigurationModule::new("manager", "Документ.ЗамерДокумента.МодульМенеджера", include_str!("../../../docs/research/fixtures/document-find-attribute/ManagerModule.bsl"), ModuleKind::Manager, ModuleExecution::new().with_server(true)).with_owner("doc"),
    ]).unwrap();
    SessionFactory::new(
        code,
        catalog,
        "МодульСеанса",
        SessionKind::ThinClient,
        "8.3.27.2342",
    )
    .unwrap()
}
fn open(factory: &SessionFactory, principal: &str) -> (Session, Rc<Adapter>) {
    let adapter = Rc::new(Adapter {
        value: RefCell::new(Value::Undefined),
        requests: RefCell::new(vec![]),
        failed: RefCell::new(false),
    });
    let mut session = factory
        .open(
            Principal::new(principal, principal, vec![]),
            SessionResources::new(Box::new(()), 1).with_data_access(adapter.clone()),
        )
        .unwrap();
    *adapter.value.borrow_mut() = evaluate(&mut session, "Документы.ЗамерДокумента.ПолучитьСсылку(Новый УникальныйИдентификатор(\"12345678-1234-4321-abcd-0123456789ab\"))").unwrap();
    (session, adapter)
}
fn evaluate(session: &mut Session, expression: &str) -> Result<Value, bsl_session::SessionError> {
    let module = session.instantiate(&format!(
        "Функция Проверить() Экспорт Возврат {expression}; КонецФункции"
    ))?;
    session
        .call(&module, "Проверить", vec![])
        .map(|(value, _)| value)
}
#[test]
fn document_attribute_lookup_replays_the_native_thirteen_controls() {
    let factory = factory(true);
    let (mut session, adapter) = open(&factory, "Первый");
    let source =
        include_str!("../../../docs/research/fixtures/document-find-attribute/measure.bsl");
    let (declaration, _) = source.split_once("КонецПроцедуры").unwrap();
    let declaration = declaration.replace("Сообщить(", "РезультатыПробы.Добавить(");
    let body = source
        .split_once("ДокументПробы.Записать();")
        .unwrap()
        .1
        .split_once("ОтменитьТранзакцию();")
        .unwrap()
        .0;
    let module = session.instantiate(&format!("Перем РезультатыПробы;\n{declaration}КонецПроцедуры\nФункция Проверить() Экспорт РезультатыПробы = Новый Массив; {body} Возврат РезультатыПробы; КонецФункции")).unwrap();
    let Value::Object(value) = session.call(&module, "Проверить", vec![]).unwrap().0
    else {
        panic!("массив наблюдений")
    };
    let BslObject::Array(values) = value.as_ref() else {
        panic!("массив наблюдений")
    };
    let actual = values
        .borrow()
        .iter()
        .map(|value| {
            let Value::Str(text) = value else {
                panic!("строка наблюдения")
            };
            text.to_string()
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    assert_eq!(
        actual,
        include_str!(
            "../../../docs/research/fixtures/document-find-attribute/measure.platform.txt"
        )
    );
    assert_eq!(&*adapter.requests.borrow(), &vec!["Первый"; 10]);
}
#[test]
fn document_attribute_lookup_keeps_sessions_separate_and_rejects_wrong_references() {
    let factory = factory(true);
    let (mut first, adapter) = open(&factory, "Первый");
    let (mut second, other) = open(&factory, "Второй");
    let expression = "Документы.ЗамерДокумента.НайтиПоРеквизиту(\"Заполняемый\", \"AbC\")";
    assert!(evaluate(&mut first, expression).is_ok());
    *other.failed.borrow_mut() = true;
    assert!(evaluate(&mut second, expression).is_err());
    assert!(evaluate(&mut first, expression).is_ok());
    for expression_of_wrong_value in [
        "Документы.ДругойДокумент.ПустаяСсылка()",
        "Справочники.СправочникПробы.ПустаяСсылка()",
        "17",
    ] {
        *adapter.value.borrow_mut() = evaluate(&mut first, expression_of_wrong_value).unwrap();
        assert!(evaluate(&mut first, expression).is_err());
    }
    assert!(
        adapter
            .requests
            .borrow()
            .iter()
            .all(|name| name == "Первый")
    );
    let (mut foreign_session, _) = open(&factory_with_identity(true, "foreign-config"), "Чужой");
    *adapter.value.borrow_mut() = evaluate(
        &mut foreign_session,
        "Документы.ЗамерДокумента.ПустаяСсылка()",
    )
    .unwrap();
    assert!(evaluate(&mut first, expression).is_err());
    assert_eq!(&*other.requests.borrow(), &["Второй"]);
    let (mut unknown, unknown_adapter) = open(&self::factory(false), "Неизвестный");
    assert!(evaluate(&mut unknown, expression).is_err());
    assert!(unknown_adapter.requests.borrow().is_empty());
}
