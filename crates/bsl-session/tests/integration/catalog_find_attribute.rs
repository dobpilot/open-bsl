use std::{cell::RefCell, rc::Rc, sync::Arc};

use bsl_config::{ConfigurationCodeSnapshot, ConfigurationModule, ModuleExecution, ModuleKind};
use bsl_orm::{
    DataAccess, FindByAttributeRequest, MetadataCatalog, MetadataKind, MetadataObject, Principal,
};
use bsl_session::{Session, SessionFactory, SessionKind, SessionResources};
use open_bsl::{CallContext, RtError, RtResult, Value};

fn factory(source: &str) -> (SessionFactory, Arc<MetadataCatalog>) {
    let catalog = Arc::new(
        MetadataCatalog::new(
            "find-attribute",
            vec![
                MetadataObject::new("catalog", "Проба", MetadataKind::Catalog)
                    .with_manager_module("Справочник.Проба.МодульМенеджера")
                    .with_catalog_attribute_names(["ИскомоеЗначение"]),
                MetadataObject::new("foreign", "Другой", MetadataKind::Catalog),
            ],
        )
        .unwrap(),
    );
    let code = ConfigurationCodeSnapshot::new(
        "find-attribute",
        vec![
            ConfigurationModule::new(
                "session",
                "МодульСеанса",
                "Процедура УстановкаПараметровСеанса(Имена) КонецПроцедуры Функция СтатическаяЗависимость() Экспорт Возврат Справочники.Проба.СтатическаяТочка(); КонецФункции",
                ModuleKind::Session,
                ModuleExecution::new().with_server(true),
            ),
            ConfigurationModule::new(
                "manager",
                "Справочник.Проба.МодульМенеджера",
                format!("{source} Функция СтатическаяТочка() Экспорт Возврат 17; КонецФункции"),
                ModuleKind::Manager,
                ModuleExecution::new().with_server(true),
            )
            .with_owner("catalog"),
        ],
    )
    .unwrap();
    (
        SessionFactory::new(
            code,
            Arc::clone(&catalog),
            "МодульСеанса",
            SessionKind::ThinClient,
            "8.3.27.2342",
        )
        .unwrap(),
        catalog,
    )
}

#[derive(Clone, Copy)]
enum Reply {
    Found,
    Missing,
    Failed,
    Foreign,
}
struct Adapter {
    catalog: Arc<MetadataCatalog>,
    reply: Reply,
    principals: RefCell<Vec<String>>,
}
impl DataAccess for Adapter {
    fn constant_value(
        &self,
        _: &MetadataObject,
        _: &Principal,
        _: &mut CallContext<'_>,
    ) -> RtResult<Value> {
        panic!("поиск не читает константу")
    }
    fn find_by_attribute(
        &self,
        metadata: &MetadataObject,
        request: &FindByAttributeRequest,
        principal: &Principal,
        _: &mut CallContext<'_>,
    ) -> RtResult<Option<Value>> {
        assert_eq!(metadata.id(), "catalog");
        assert_eq!(request.attribute(), "ИскомоеЗначение");
        assert_eq!(request.value(), &Value::Str("AbC".into()));
        assert!(request.parent().is_none() && request.owner().is_none());
        self.principals.borrow_mut().push(principal.name().into());
        match self.reply {
            Reply::Found => {
                bsl_orm::catalog_reference_value(Arc::clone(&self.catalog), "catalog", [17; 16])
                    .map(Some)
            }
            Reply::Missing => Ok(None),
            Reply::Failed => Err(RtError::DynamicError("отказ доступа к поиску".into())),
            Reply::Foreign => {
                bsl_orm::catalog_reference_value(Arc::clone(&self.catalog), "foreign", [17; 16])
                    .map(Some)
            }
        }
    }
}
fn open(factory: &SessionFactory, adapter: Rc<dyn DataAccess>, user: &str) -> Session {
    factory
        .open(
            Principal::new(user, user, vec![]),
            SessionResources::new(Box::new(()), 1).with_data_access(adapter),
        )
        .unwrap()
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
fn original_manager_lookup_and_english_alias_use_the_session_adapter() {
    let (factory, catalog) = factory(
        "Функция НайтиПробу() Экспорт Возврат НайтиПоРеквизиту(\"иСкОмОеЗнАчЕнИе\", \"AbC\"); КонецФункции",
    );
    let adapter = Rc::new(Adapter {
        catalog,
        reply: Reply::Found,
        principals: RefCell::new(vec![]),
    });
    let mut session = open(&factory, adapter.clone(), "Первый");
    for expression in [
        "Справочники.Проба.НайтиПробу()",
        "Справочники.Проба.fInDbYaTtRiBuTe(\"ИскомоеЗначение\", \"AbC\")",
    ] {
        let result = evaluate(&mut session, expression).unwrap();
        assert_eq!(
            bsl_orm::catalog_reference_parts(&result).unwrap().1,
            [17; 16]
        );
    }
    assert_eq!(&*adapter.principals.borrow(), &["Первый", "Первый"]);
}

#[test]
fn sessions_of_one_factory_keep_their_lookup_adapters_and_principals() {
    let (factory, catalog) = factory("");
    let found = Rc::new(Adapter {
        catalog: Arc::clone(&catalog),
        reply: Reply::Found,
        principals: RefCell::new(vec![]),
    });
    let missing = Rc::new(Adapter {
        catalog,
        reply: Reply::Missing,
        principals: RefCell::new(vec![]),
    });
    let mut first = open(&factory, found.clone(), "Первый");
    let mut second = open(&factory, missing.clone(), "Второй");
    let expression = "Справочники.Проба.НайтиПоРеквизиту(\"ИскомоеЗначение\", \"AbC\")";
    for (use_first, uuid) in [(true, [17; 16]), (false, [0; 16]), (true, [17; 16])] {
        let session = if use_first { &mut first } else { &mut second };
        let result = evaluate(session, expression).unwrap();
        let (owner, actual) = bsl_orm::catalog_reference_parts(&result).unwrap();
        assert_eq!(owner.id(), "catalog");
        assert_eq!(actual, uuid);
    }
    assert_eq!(&*found.principals.borrow(), &["Первый", "Первый"]);
    assert_eq!(&*missing.principals.borrow(), &["Второй"]);
}

#[test]
fn lookup_does_not_hide_failure_or_accept_a_foreign_reference() {
    for (reply, message) in [
        (Reply::Failed, "отказ доступа к поиску"),
        (Reply::Foreign, "несовместимую ссылку"),
    ] {
        let (factory, catalog) = factory("");
        let mut session = open(
            &factory,
            Rc::new(Adapter {
                catalog,
                reply,
                principals: RefCell::new(vec![]),
            }),
            "Тест",
        );
        assert!(
            evaluate(
                &mut session,
                "Справочники.Проба.НайтиПоРеквизиту(\"ИскомоеЗначение\", \"AbC\")"
            )
            .unwrap_err()
            .to_string()
            .contains(message)
        );
    }
}

struct PreviousAdapter;
impl DataAccess for PreviousAdapter {
    fn constant_value(
        &self,
        _: &MetadataObject,
        _: &Principal,
        _: &mut CallContext<'_>,
    ) -> RtResult<Value> {
        panic!("поиск не читает константу")
    }
}

#[test]
fn an_existing_manager_export_keeps_its_lookup_name() {
    for (parameters, arguments) in [
        ("", ""),
        ("Имя, Значение", "\"ИскомоеЗначение\", \"AbC\""),
        ("Первый, Второй, Третий, Четвертый, Пятый", "1, 2, 3, 4, 5"),
    ] {
        let (factory, _) = factory(&format!(
            "Функция НайтиПоРеквизиту({parameters}) Экспорт Возврат 23; КонецФункции"
        ));
        let mut session = open(&factory, Rc::new(PreviousAdapter), "Тест");
        assert_eq!(
            evaluate(
                &mut session,
                &format!("Справочники.Проба.нАйТиПоРеКвИзИтУ({arguments})")
            )
            .unwrap(),
            Value::number_from_i64(23)
        );
    }
}
#[test]
fn previous_adapter_reports_unsupported_lookup_instead_of_no_match() {
    let (factory, _) = factory("");
    let mut session = open(&factory, Rc::new(PreviousAdapter), "Тест");
    assert!(
        evaluate(
            &mut session,
            "Справочники.Проба.НайтиПоРеквизиту(\"ИскомоеЗначение\", \"AbC\")"
        )
        .unwrap_err()
        .to_string()
        .contains("поиск по реквизиту не поддерживается")
    );
}

#[test]
fn invalid_lookup_arguments_fail_before_reading_data() {
    let (factory, catalog) = factory("");
    let adapter = Rc::new(Adapter {
        catalog,
        reply: Reply::Found,
        principals: RefCell::new(vec![]),
    });
    let mut session = open(&factory, adapter.clone(), "Тест");
    for arguments in [
        "\"Код\", \"AbC\"",
        "\"Наименование\", \"AbC\"",
        "\"Неизвестное\", \"AbC\"",
        "17, \"AbC\"",
        "\"ИскомоеЗначение\", Новый Массив",
    ] {
        assert!(
            evaluate(
                &mut session,
                &format!("Справочники.Проба.НайтиПоРеквизиту({arguments})")
            )
            .is_err(),
            "{arguments}"
        );
    }
    assert!(adapter.principals.borrow().is_empty());
}

#[test]
fn lookup_rejects_argument_counts_outside_the_native_signature() {
    let (factory, _) = factory("");
    let mut session = open(&factory, Rc::new(PreviousAdapter), "Тест");
    for arguments in ["", "\"ИскомоеЗначение\"", "1, 2, 3, 4, 5"] {
        let error = evaluate(
            &mut session,
            &format!("Справочники.Проба.НайтиПоРеквизиту({arguments})"),
        )
        .unwrap_err()
        .to_string();
        assert!(!error.contains("поиск по реквизиту не поддерживается"));
    }
}
