use bsl_config::{ConfigurationCodeSnapshot, ConfigurationModule, ModuleExecution, ModuleKind};
use bsl_orm::{
    CatalogCodeType, CatalogStandardAttributes, DataAccess, MetadataCatalog, MetadataKind,
    MetadataObject, Principal, QueryRequest, QueryResultData,
};
use bsl_session::{Session, SessionFactory, SessionKind, SessionResources};
use open_bsl::{CallContext, RtError, RtResult, Value};
use std::{cell::RefCell, rc::Rc, sync::Arc};

fn factory(source: &str) -> (SessionFactory, Arc<MetadataCatalog>) {
    let catalog = Arc::new(
        MetadataCatalog::new(
            "standard-search",
            vec![
                MetadataObject::new("catalog", "Проба", MetadataKind::Catalog)
                    .with_manager_module("Справочник.Проба.МодульМенеджера")
                    .with_catalog_standard_attributes(CatalogStandardAttributes {
                        code_type: CatalogCodeType::String,
                        code_length: 9,
                        description_length: 25,
                    }),
                MetadataObject::new("foreign", "Другой", MetadataKind::Catalog),
            ],
        )
        .unwrap(),
    );
    let code = ConfigurationCodeSnapshot::new("standard-search", vec![
        ConfigurationModule::new("session", "МодульСеанса", "Процедура УстановкаПараметровСеанса(Имена) КонецПроцедуры Функция СтатическаяЗависимость() Экспорт Возврат Справочники.Проба.СтатическаяТочка(); КонецФункции", ModuleKind::Session, ModuleExecution::new().with_server(true)),
        ConfigurationModule::new("manager", "Справочник.Проба.МодульМенеджера", format!("{source} Функция СтатическаяТочка() Экспорт Возврат 17; КонецФункции"), ModuleKind::Manager, ModuleExecution::new().with_server(true)).with_owner("catalog"),
    ]).unwrap();
    (
        SessionFactory::new(
            code,
            catalog.clone(),
            "МодульСеанса",
            SessionKind::ThinClient,
            "8.3.27.2342",
        )
        .unwrap(),
        catalog,
    )
}
struct Adapter {
    catalog: Arc<MetadataCatalog>,
    failed: bool,
    foreign: bool,
    users: RefCell<Vec<String>>,
}
impl DataAccess for Adapter {
    fn constant_value(
        &self,
        _: &MetadataObject,
        _: &Principal,
        _: &mut CallContext<'_>,
    ) -> RtResult<Value> {
        panic!("поиск не читает константы")
    }
    fn execute_query(
        &self,
        request: &QueryRequest,
        principal: &Principal,
        _: &mut CallContext<'_>,
    ) -> RtResult<QueryResultData> {
        self.users.borrow_mut().push(principal.name().into());
        if self.failed {
            return Err(RtError::DynamicError("отказ доступа к поиску".into()));
        }
        assert!(!request.text().contains("ПЕРВЫЕ"));
        assert!(request.text().contains("&Значение"));
        let parameters = request.parameters().collect::<Vec<_>>();
        assert_eq!(parameters.len(), 1);
        let description = request.text().contains("Источник.Наименование");
        let owner = if self.foreign { "foreign" } else { "catalog" };
        let reference = bsl_orm::catalog_reference_value(self.catalog.clone(), owner, [17; 16])?;
        let mut rows = vec![];
        if parameters[0].1 == &Value::Str("AbC".into()) {
            rows.push(vec![
                bsl_orm::catalog_reference_value(self.catalog.clone(), "catalog", [23; 16])?,
                Value::Str(if description { "abc name" } else { "abc" }.into()),
            ]);
        }
        rows.push(vec![
            reference,
            Value::Str(if description { "AbC name" } else { "AbC" }.into()),
        ]);
        QueryResultData::new(["Ссылка".into(), "ЗначениеПоиска".into()], rows)
            .map_err(|error| RtError::DynamicError(error.to_string()))
    }
}
fn open(factory: &SessionFactory, adapter: Rc<Adapter>, user: &str) -> Session {
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
        .map(|result| result.0)
}

#[test]
fn native_string_code_and_description_matching_uses_the_current_principal() {
    let (factory, catalog) =
        factory("Функция ПоискИзМенеджера() Экспорт Возврат НайтиПоКоду(\"AbC\"); КонецФункции");
    let adapter = Rc::new(Adapter {
        catalog,
        failed: false,
        foreign: false,
        users: RefCell::new(vec![]),
    });
    for user in ["Первый", "Второй"] {
        let mut session = open(&factory, adapter.clone(), user);
        for (expression, found) in [
            ("Справочники.Проба.ПоискИзМенеджера()", true),
            ("Справочники.Проба.fInDbYcOdE(\"AbC\")", true),
            ("Справочники.Проба.НайтиПоКоду(\"AbC \")", true),
            ("Справочники.Проба.НайтиПоКоду(\"abc\")", false),
            ("Справочники.Проба.НайтиПоКоду(\" AbC\")", false),
            ("Справочники.Проба.НайтиПоКоду(\"missing\")", false),
            (
                "Справочники.Проба.FindByDescription(\"AbC name \", Истина)",
                true,
            ),
            (
                "Справочники.Проба.НайтиПоНаименованию(\"abc name\", Истина)",
                false,
            ),
            ("Справочники.Проба.НайтиПоНаименованию(\"AbC\")", true),
            (
                "Справочники.Проба.НайтиПоНаименованию(\"AbC\", Истина)",
                false,
            ),
            ("Справочники.Проба.НайтиПоНаименованию(\"abc\")", false),
            ("Справочники.Проба.НайтиПоНаименованию(\"\")", true),
        ] {
            let value = evaluate(&mut session, expression).unwrap();
            assert_eq!(
                bsl_orm::catalog_reference_parts(&value).unwrap().1,
                if found { [17; 16] } else { [0; 16] },
                "{expression}"
            );
            assert_eq!(adapter.users.borrow().last().unwrap(), user);
        }
    }
}

#[test]
fn search_failure_and_foreign_results_never_become_empty_references() {
    let (factory, catalog) = factory("");
    for (failed, foreign, expected) in [
        (true, false, "отказ доступа"),
        (false, true, "несовместимую ссылку"),
    ] {
        let adapter = Rc::new(Adapter {
            catalog: catalog.clone(),
            failed,
            foreign,
            users: RefCell::new(vec![]),
        });
        let mut session = open(&factory, adapter, "Тест");
        let error = evaluate(&mut session, "Справочники.Проба.НайтиПоКоду(\"AbC\")")
            .unwrap_err()
            .to_string();
        assert!(error.contains(expected), "{error}");
    }
}

#[test]
fn existing_same_named_export_preserves_its_arity_and_result() {
    let (factory, catalog) =
        factory("Функция НайтиПоКоду(А, Б, В, Г, Д) Экспорт Возврат 23; КонецФункции");
    let adapter = Rc::new(Adapter {
        catalog,
        failed: true,
        foreign: false,
        users: RefCell::new(vec![]),
    });
    let mut session = open(&factory, adapter.clone(), "Тест");
    assert_eq!(
        evaluate(&mut session, "Справочники.Проба.НайтиПоКоду(1,2,3,4,5)").unwrap(),
        Value::number_from_i64(23)
    );
    assert!(adapter.users.borrow().is_empty());
}

#[test]
fn invalid_arity_and_unsupported_modes_do_not_read_data() {
    let (factory, catalog) = factory("");
    let adapter = Rc::new(Adapter {
        catalog,
        failed: true,
        foreign: false,
        users: RefCell::new(vec![]),
    });
    let mut session = open(&factory, adapter.clone(), "Тест");
    for expression in [
        "Справочники.Проба.НайтиПоКоду()",
        "Справочники.Проба.НайтиПоКоду(1,2,3,4,5)",
        "Справочники.Проба.НайтиПоНаименованию()",
        "Справочники.Проба.НайтиПоНаименованию(1,2,3,4,5)",
        "Справочники.Проба.НайтиПоКоду(\"AbC\", Истина)",
        "Справочники.Проба.НайтиПоКоду(\"AbC\", Ложь, 17)",
    ] {
        assert!(evaluate(&mut session, expression).is_err(), "{expression}");
    }
    assert!(adapter.users.borrow().is_empty());
}
