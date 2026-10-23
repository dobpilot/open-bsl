use std::{cell::RefCell, rc::Rc, sync::Arc};

use bsl_config::{ConfigurationCodeSnapshot, ConfigurationModule, ModuleExecution, ModuleKind};
use bsl_orm::{
    DataAccess, MetadataCatalog, MetadataKind, MetadataObject, Principal, QueryRequest,
    QueryResultData,
};
use bsl_session::{Session, SessionFactory, SessionKind, SessionResources};
use open_bsl::{BslObject, CallContext, RtResult, Value, format_value};

struct Adapter {
    value: RefCell<Value>,
    requests: RefCell<Vec<(String, String)>>,
}
impl DataAccess for Adapter {
    fn constant_value(
        &self,
        _: &MetadataObject,
        _: &Principal,
        _: &mut CallContext<'_>,
    ) -> RtResult<Value> {
        panic!("предопределённое свойство не читает константу")
    }
    fn execute_query(
        &self,
        request: &QueryRequest,
        principal: &Principal,
        _: &mut CallContext<'_>,
    ) -> RtResult<QueryResultData> {
        self.requests
            .borrow_mut()
            .push((principal.id().to_owned(), request.text().to_string()));
        QueryResultData::new(
            vec!["Ссылка".into()],
            vec![vec![self.value.borrow().clone()]],
        )
        .map_err(|error| open_bsl::RtError::DynamicError(error.to_string()))
    }
}

fn session() -> (Session, Rc<Adapter>) {
    let catalog = Arc::new(
        MetadataCatalog::new(
            "characteristic-predefined",
            vec![
                MetadataObject::new(
                    "probe",
                    "ЗамерХарактеристик",
                    MetadataKind::ChartOfCharacteristicTypes,
                )
                .with_catalog_predefined_names(["Основной"])
                .with_manager_module("ПланВидовХарактеристик.ЗамерХарактеристик.МодульМенеджера"),
                MetadataObject::new(
                    "other",
                    "ДругиеХарактеристики",
                    MetadataKind::ChartOfCharacteristicTypes,
                ),
                MetadataObject::new("catalog", "ДругойСправочник", MetadataKind::Catalog),
            ],
        )
        .unwrap(),
    );
    let code = ConfigurationCodeSnapshot::new(
        "characteristic-predefined",
        vec![
            ConfigurationModule::new(
                "session",
                "МодульСеанса",
                "Процедура УстановкаПараметровСеанса(Имена) КонецПроцедуры Функция Прочитать() Экспорт Возврат ПланыВидовХарактеристик.ЗамерХарактеристик.ПрочитатьПредопределенный(); КонецФункции",
                ModuleKind::Session,
                ModuleExecution::new().with_server(true),
            ),
            ConfigurationModule::new(
                "manager",
                "ПланВидовХарактеристик.ЗамерХарактеристик.МодульМенеджера",
                include_str!(
                    "../../../../docs/research/fixtures/characteristic-predefined/ManagerModule.bsl"
                ),
                ModuleKind::Manager,
                ModuleExecution::new().with_server(true),
            )
            .with_owner("probe"),
        ],
    )
    .unwrap();
    let factory = SessionFactory::new(
        code,
        catalog,
        "МодульСеанса",
        SessionKind::ThinClient,
        "8.3.27.2342",
    )
    .unwrap();
    let adapter = Rc::new(Adapter {
        value: RefCell::new(Value::Undefined),
        requests: RefCell::new(vec![]),
    });
    let session = factory
        .open(
            Principal::new("user", "Тест", vec![]),
            SessionResources::new(Box::new(()), 1).with_data_access(adapter.clone()),
        )
        .unwrap();
    (session, adapter)
}

fn evaluate(session: &mut Session, body: &str) -> Value {
    let module = session
        .instantiate(&format!("Функция Проверить() Экспорт {body} КонецФункции"))
        .unwrap();
    session.call(&module, "Проверить", vec![]).unwrap().0
}

#[test]
fn characteristic_predefined_manager_replays_native_control_with_session_adapter() {
    let (mut session, adapter) = session();
    let reference = evaluate(
        &mut session,
        "Возврат ПланыВидовХарактеристик.ЗамерХарактеристик.ПолучитьСсылку(Новый УникальныйИдентификатор(\"00000000-0000-0000-0000-000000000017\"));",
    );
    *adapter.value.borrow_mut() = reference;
    let source = include_str!(
        "../../../../docs/research/fixtures/characteristic-predefined/measure-manager.bsl"
    )
    .replace("Сообщить(", "РезультатыПробы.Добавить(");
    let result = evaluate(
        &mut session,
        &format!("РезультатыПробы = Новый Массив; {source} Возврат РезультатыПробы;"),
    );
    let Value::Object(result) = result else {
        panic!("ожидался массив");
    };
    let BslObject::Array(result) = result.as_ref() else {
        panic!("ожидался массив");
    };
    let actual = result
        .borrow()
        .iter()
        .map(|value| format_value(value, None).unwrap())
        .collect::<Vec<_>>()
        .join("\n");
    assert_eq!(
        actual,
        include_str!(
            "../../../../docs/research/fixtures/characteristic-predefined/measure-manager.platform.txt"
        )
        .trim_end()
    );
    assert_eq!(adapter.requests.borrow().len(), 4);
    assert!(adapter.requests.borrow().iter().all(|(principal, text)| principal == "user" && text == "ВЫБРАТЬ ЗНАЧЕНИЕ(ПланВидовХарактеристик.ЗамерХарактеристик.Основной) КАК Ссылка"));
}

#[test]
fn characteristic_predefined_manager_rejects_missing_and_incompatible_adapter_values() {
    let (mut session, adapter) = session();
    let module = session.instantiate("Функция Проверить() Экспорт Возврат ПланыВидовХарактеристик.ЗамерХарактеристик.Основной; КонецФункции").unwrap();
    for (value, expected) in [
        (Value::Null, "Предопределенный элемент не найден"),
        (
            Value::Boolean(true),
            "несовместимый тип предопределённого элемента",
        ),
    ] {
        *adapter.value.borrow_mut() = value;
        let error = session.call(&module, "Проверить", vec![]).unwrap_err();
        assert!(error.to_string().contains(expected), "{error}");
    }
    for expression in [
        "ПланыВидовХарактеристик.ДругиеХарактеристики.ПустаяСсылка()",
        "Справочники.ДругойСправочник.ПустаяСсылка()",
    ] {
        *adapter.value.borrow_mut() = evaluate(&mut session, &format!("Возврат {expression};"));
        let error = session.call(&module, "Проверить", vec![]).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("несовместимый тип предопределённого элемента"),
            "{error}"
        );
    }
}
