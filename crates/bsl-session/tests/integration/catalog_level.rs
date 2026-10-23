use bsl_config::{ConfigurationCodeSnapshot, ConfigurationModule, ModuleExecution, ModuleKind};
use bsl_orm::{
    CatalogCodeType, CatalogDefaultPresentation, CatalogHierarchyType, CatalogStandardAttributes,
    DataAccess, MetadataCatalog, MetadataKind, MetadataObject, Principal, QueryRequest,
    QueryResultData,
};
use bsl_session::{Session, SessionFactory, SessionKind, SessionResources};
use open_bsl::{BslObject, CallContext, RtError, RtResult, Value};
use std::{cell::RefCell, collections::HashMap, rc::Rc, sync::Arc};

struct Adapter {
    parents: RefCell<HashMap<[u8; 16], Value>>,
    calls: RefCell<Vec<String>>,
    failed: RefCell<bool>,
}
impl DataAccess for Adapter {
    fn constant_value(
        &self,
        _: &MetadataObject,
        _: &Principal,
        _: &mut CallContext<'_>,
    ) -> RtResult<Value> {
        panic!("уровень не читает константу")
    }
    fn execute_query(
        &self,
        request: &QueryRequest,
        principal: &Principal,
        _: &mut CallContext<'_>,
    ) -> RtResult<QueryResultData> {
        assert_eq!(
            request.text(),
            "ВЫБРАТЬ ПЕРВЫЕ 1 Источник.Родитель КАК Родитель ИЗ Справочник.ЗамерВременныхТаблиц КАК Источник ГДЕ Источник.Ссылка = &Ссылка"
        );
        let parameters = request.parameters().collect::<Vec<_>>();
        assert_eq!(parameters.len(), 1);
        assert_eq!(parameters[0].0, "Ссылка");
        let (owner, uuid) = bsl_orm::catalog_reference_parts(parameters[0].1).unwrap();
        assert_eq!(owner.id(), "catalog");
        self.calls.borrow_mut().push(principal.id().into());
        if *self.failed.borrow() {
            return Err(RtError::DynamicError("отказ чтения родителя".into()));
        }
        QueryResultData::new(
            vec!["Родитель".into()],
            self.parents
                .borrow()
                .get(&uuid)
                .map(|parent| vec![vec![parent.clone()]])
                .unwrap_or_default(),
        )
        .map_err(|failure| RtError::DynamicError(failure.to_string()))
    }
}
fn factory(hierarchical: Option<bool>) -> SessionFactory {
    factory_with_source(
        hierarchical,
        include_str!("../../../../docs/research/fixtures/catalog-level/ObjectModule.bsl"),
    )
}

fn factory_with_source(hierarchical: Option<bool>, source: &str) -> SessionFactory {
    let mut catalog = MetadataObject::new("catalog", "ЗамерВременныхТаблиц", MetadataKind::Catalog)
        .with_object_module("Справочник.ЗамерВременныхТаблиц.МодульОбъекта")
        .with_catalog_hierarchy_type(CatalogHierarchyType::FoldersAndItems)
        .with_catalog_standard_attributes(CatalogStandardAttributes {
            code_type: CatalogCodeType::String,
            code_length: 9,
            description_length: 25,
        })
        .with_catalog_default_presentation(CatalogDefaultPresentation::AsDescription);
    if let Some(value) = hierarchical {
        catalog = catalog.with_catalog_hierarchical(value);
    }
    let metadata = Arc::new(
        MetadataCatalog::new(
            "catalog-level",
            vec![
                catalog,
                MetadataObject::new("other", "Другой", MetadataKind::Catalog),
            ],
        )
        .unwrap(),
    );
    let code = ConfigurationCodeSnapshot::new("catalog-level", vec![
        ConfigurationModule::new("session", "МодульСеанса", "Процедура УстановкаПараметровСеанса(Имена) КонецПроцедуры Функция Проба() Экспорт Возврат Справочники.ЗамерВременныхТаблиц.СоздатьЭлемент(); КонецФункции", ModuleKind::Session, ModuleExecution::new().with_server(true)),
        ConfigurationModule::new("object", "Справочник.ЗамерВременныхТаблиц.МодульОбъекта", source, ModuleKind::Object, ModuleExecution::new().with_server(true)).with_owner("catalog"),
    ]).unwrap();
    SessionFactory::new(
        code,
        metadata,
        "МодульСеанса",
        SessionKind::ThinClient,
        "8.3.27.2342",
    )
    .unwrap()
}
fn open(factory: &SessionFactory, principal: &str) -> (Session, Rc<Adapter>) {
    let adapter = Rc::new(Adapter {
        parents: RefCell::new(HashMap::new()),
        calls: RefCell::new(vec![]),
        failed: RefCell::new(false),
    });
    let session = factory
        .open(
            Principal::new(principal, principal, vec![]),
            SessionResources::new(Box::new(()), 1).with_data_access(adapter.clone()),
        )
        .unwrap();
    (session, adapter)
}
fn evaluate(session: &mut Session, body: &str) -> Result<Value, bsl_session::SessionError> {
    let module =
        session.instantiate(&format!("Функция Проверить() Экспорт {body} КонецФункции"))?;
    session
        .call(&module, "Проверить", vec![])
        .map(|(value, _)| value)
}
fn reference(session: &mut Session, suffix: u8) -> Value {
    evaluate(session,&format!("Возврат Справочники.ЗамерВременныхТаблиц.ПолучитьСсылку(Новый УникальныйИдентификатор(\"00000000-0000-0000-0000-{suffix:012}\"));")).unwrap()
}
#[test]
fn catalog_object_level_replays_native_levels_and_selection_guard() {
    let (mut session, adapter) = open(&factory(Some(true)), "Первый");
    let root = reference(&mut session, 17);
    let child = reference(&mut session, 18);
    let empty = reference(&mut session, 0);
    adapter
        .parents
        .borrow_mut()
        .insert(bsl_orm::catalog_reference_parts(&root).unwrap().1, empty);
    adapter
        .parents
        .borrow_mut()
        .insert(bsl_orm::catalog_reference_parts(&child).unwrap().1, root);
    let source = include_str!("../../../../docs/research/fixtures/catalog-level/measure.bsl");
    let (declaration, _) = source.split_once("КонецПроцедуры").unwrap();
    let declaration = declaration.replace("Сообщить(", "РезультатыПробы.Добавить(");
    let body = source.split_once("НачатьТранзакцию();").unwrap().1.split_once("ОтменитьТранзакцию();").unwrap().0
        .replace("\n    ГруппаПробы.Записать();","ГруппаПробы.УстановитьСсылкуНового(Справочники.ЗамерВременныхТаблиц.ПолучитьСсылку(Новый УникальныйИдентификатор(\"00000000-0000-0000-0000-000000000017\")));")
        .replace("ВтораяГруппаПробы.Записать();","ВтораяГруппаПробы.УстановитьСсылкуНового(Справочники.ЗамерВременныхТаблиц.ПолучитьСсылку(Новый УникальныйИдентификатор(\"00000000-0000-0000-0000-000000000018\")));");
    // Запись остаётся нативной подготовкой. В runtime ссылки предков
    // заранее установлены в объекте и строки читаются внедрённым адаптером.
    let body = body.replace("ГруппаПробы.Ссылка", "ГруппаПробы.ПолучитьСсылкуНового()");
    let module = session.instantiate(&format!("Перем РезультатыПробы; {declaration}КонецПроцедуры Функция Проверить() Экспорт РезультатыПробы = Новый Массив; {body} Возврат РезультатыПробы; КонецФункции")).unwrap();
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
                panic!("строка")
            };
            text.to_string()
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    assert_eq!(
        actual,
        include_str!("../../../../docs/research/fixtures/catalog-level/measure.platform.txt")
    );
    assert_eq!(&*adapter.calls.borrow(), &vec!["Первый"; 4]);
}
#[test]
fn catalog_level_uses_its_session_and_rejects_cycles_unknown_hierarchy_and_wrong_values() {
    let factory = factory(Some(true));
    let (mut first, adapter) = open(&factory, "Первый");
    let (mut second, other) = open(&factory, "Второй");
    let parent = reference(&mut first, 17);
    let key = bsl_orm::catalog_reference_parts(&parent).unwrap().1;
    let empty = reference(&mut first, 0);
    adapter.parents.borrow_mut().insert(key, empty);
    *other.failed.borrow_mut() = true;
    let body = "ОбъектПробы = Справочники.ЗамерВременныхТаблиц.СоздатьЭлемент(); ОбъектПробы.Родитель = Справочники.ЗамерВременныхТаблиц.ПолучитьСсылку(Новый УникальныйИдентификатор(\"00000000-0000-0000-0000-000000000017\")); Возврат ОбъектПробы.Уровень();";
    assert_eq!(
        evaluate(&mut first, body).unwrap(),
        Value::number_from_i64(1)
    );
    assert!(evaluate(&mut second, body).is_err());
    adapter.parents.borrow_mut().insert(key, parent);
    assert!(evaluate(&mut first, body).is_err());
    for wrong in [
        Value::Null,
        Value::number_from_i64(17),
        evaluate(&mut first, "Возврат Справочники.Другой.ПустаяСсылка();").unwrap(),
    ] {
        adapter.parents.borrow_mut().insert(key, wrong);
        assert!(evaluate(&mut first, body).is_err());
    }
    assert!(adapter.calls.borrow().iter().all(|name| name == "Первый"));
    assert_eq!(&*other.calls.borrow(), &["Второй"]);
    let (mut unknown, _) = open(&self::factory(None), "Неизвестный");
    assert!(
        evaluate(
            &mut unknown,
            "Возврат Справочники.ЗамерВременныхТаблиц.СоздатьЭлемент().Уровень();"
        )
        .is_err()
    );
    let (mut flat, flat_adapter) = open(&self::factory(Some(false)), "Плоский");
    assert_eq!(
        evaluate(
            &mut flat,
            "Возврат Справочники.ЗамерВременныхТаблиц.СоздатьЭлемент().Уровень();"
        )
        .unwrap(),
        Value::number_from_i64(0)
    );
    assert_eq!(
        evaluate(
            &mut flat,
            "Возврат Справочники.ЗамерВременныхТаблиц.СоздатьЭлемент().ПрочитатьТело();"
        )
        .unwrap(),
        Value::Str("ERROR".into()),
    );
    assert!(
        evaluate(
            &mut flat,
            "Возврат Справочники.ЗамерВременныхТаблиц.СоздатьЭлемент().Уровень(17);"
        )
        .is_err()
    );
    assert!(
        flat.instantiate("Функция Проверить() Экспорт Возврат Уровень(); КонецФункции")
            .is_err()
    );
    assert!(flat_adapter.calls.borrow().is_empty());
}

#[test]
fn a_preexisting_object_export_keeps_its_level_name_and_arity() {
    for name in ["Уровень", "Level"] {
        let source = format!("Функция {name}(Знач Проба) Экспорт Возврат Проба + 1; КонецФункции");
        let (mut session, adapter) = open(&factory_with_source(Some(true), &source), "Экспорт");
        assert_eq!(
            evaluate(
                &mut session,
                &format!("Возврат Справочники.ЗамерВременныхТаблиц.СоздатьЭлемент().{name}(23);")
            )
            .unwrap(),
            Value::number_from_i64(24),
        );
        assert!(adapter.calls.borrow().is_empty());
    }
}
