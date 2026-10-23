use bsl_orm::{MetadataCatalog, MetadataKind, MetadataObject, Principal, SessionData};
use open_bsl::{Engine, GraphLimits, RuntimeShapes, SerializedValueGraph, Value};
use std::{
    cell::RefCell,
    io::{self, Write},
    rc::Rc,
    sync::Arc,
};

#[derive(Clone)]
struct Output(Rc<RefCell<Vec<u8>>>);
impl Write for Output {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.borrow_mut().extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
fn catalog() -> Arc<MetadataCatalog> {
    Arc::new(
        MetadataCatalog::new(
            "transfer-test",
            vec![MetadataObject::new(
                "catalog",
                "ЗамерВременныхТаблиц",
                MetadataKind::Catalog,
            )],
        )
        .unwrap(),
    )
}
fn engine(catalog: &Arc<MetadataCatalog>) -> Engine {
    let mut builder = Engine::builder().register_library(bsl_orm::library());
    for ty in catalog.transferable_value_types() {
        builder = builder.register_value_type(ty);
    }
    builder.build().unwrap()
}
#[test]
fn the_native_storage_probe_runs_unchanged_and_matches_its_oracle() {
    let catalog = catalog();
    let engine = engine(&catalog);
    let output = Output(Rc::new(RefCell::new(vec![])));
    let mut state = engine
        .state_builder()
        .stdout(output.clone())
        .host_data(Rc::new(SessionData::new(
            catalog,
            Principal::new("user", "Тест", vec![]),
        )))
        .build();
    state
        .exec(include_str!(
            "../../../tests/conformance/measure/1c/measure-catalog-reference-storage.bsl"
        ))
        .unwrap();
    let actual = String::from_utf8(output.0.borrow().clone()).unwrap();
    // Сравнения выполнены тем же BSL; отличается только язык представления Булево.
    let expected = include_str!(
        "../../../tests/conformance/measure/1c/measure-catalog-reference-storage.platform.txt"
    )
    .replace("=Yes", "=Да");
    assert_eq!(actual.trim(), expected.trim());
}
#[test]
fn the_graph_retains_no_host_data_and_restores_identity_in_an_independent_state() {
    let catalog = catalog();
    let weak_catalog = Arc::downgrade(&catalog);
    assert!(catalog.value_types().all(|ty| !ty.has_materializer()));
    let engine = engine(&catalog);
    let parent_data = Rc::new(SessionData::new(
        Arc::clone(&catalog),
        Principal::new("parent", "Родитель", vec![]),
    ));
    let weak_data = Rc::downgrade(&parent_data);
    let mut parent = engine.state_builder().host_data(parent_data).build();
    let original = parent.exec(r#"Возврат Справочники.ЗамерВременныхТаблиц.ПолучитьСсылку(Новый УникальныйИдентификатор("12345678-1234-4321-abcd-0123456789ab"));"#).unwrap();
    let mut shapes = RuntimeShapes::seeded(vec![], vec![], Some(engine.registry()));
    let graph = SerializedValueGraph::capture(
        std::slice::from_ref(&original),
        &shapes,
        &GraphLimits::default(),
    )
    .unwrap();
    drop(parent);
    assert!(weak_data.upgrade().is_none());
    let restored = graph.materialize(&mut shapes).unwrap().remove(0);
    assert_eq!(restored, original);
    let child_data = Rc::new(SessionData::new(
        Arc::clone(&catalog),
        Principal::new("child", "Потомок", vec![]),
    ));
    let mut child = engine.state_builder().host_data(child_data).build();
    let module = engine
        .compile("Функция Проверить(Ссылка) Экспорт Возврат Ссылка.Метаданные().Имя; КонецФункции")
        .unwrap();
    let instance = child.instantiate(&module).unwrap();
    let result = instance
        .call(&mut child, "Проверить", vec![restored])
        .unwrap()
        .0;
    assert_eq!(
        open_bsl::format_value(&result, None).unwrap(),
        "ЗамерВременныхТаблиц"
    );
    drop(instance);
    drop(child);
    drop(module);
    drop(original);
    drop(shapes);
    drop(engine);
    drop(catalog);
    // Граф содержит только имена/UUID; цепочка каталог → тип → каталог не образуется.
    assert!(weak_catalog.upgrade().is_none());
}

fn check_worker_isolation(
    context: &mut open_bsl::CallContext<'_>,
    _: &[Value],
) -> open_bsl::RtResult<Value> {
    if matches!(
        context.host_data(),
        Err(open_bsl::RtError::CapabilityMissing {
            capability: open_bsl::Capability::HostData,
            ..
        })
    ) {
        Ok(Value::Undefined)
    } else {
        Err(open_bsl::RtError::DynamicError(
            "worker получил данные родителя".into(),
        ))
    }
}

const WORKER_CHECK: &[open_bsl::FunctionDescriptor] = &[open_bsl::FunctionDescriptor {
    code: open_bsl::FunctionCode::new(1),
    names: &["ПроверитьИзоляциюWorker"],
    arity: open_bsl::Arity::exact(0),
    kind: open_bsl::FunctionKind::Procedure,
    call: check_worker_isolation,
}];

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn an_os_worker_restores_the_reference_parameter_without_parent_host_data() {
    let catalog = catalog();
    let mut builder = Engine::builder().register_library(bsl_orm::library());
    for ty in catalog.transferable_value_types() {
        builder = builder.register_value_type(ty);
    }
    let engine = builder.register_library(open_bsl::LibraryDescriptor::new("worker-check", "1").with_functions(WORKER_CHECK)).common_module("Работы", r#"
        Процедура Проверить(Ссылка) Экспорт
            Если ТипЗнч(Ссылка) <> Тип("CatalogRef.ЗамерВременныхТаблиц") Тогда ВызватьИсключение "type"; КонецЕсли;
            Если Строка(Ссылка.УникальныйИдентификатор()) <> "12345678-1234-4321-abcd-0123456789ab" Тогда ВызватьИсключение "uuid"; КонецЕсли;
            Если Ссылка.Метаданные().Имя <> "ЗамерВременныхТаблиц" Тогда ВызватьИсключение "metadata"; КонецЕсли;
            ПроверитьИзоляциюWorker();
        КонецПроцедуры
    "#).build().unwrap();
    let mut parent = engine
        .state_builder()
        .host_data(Rc::new(SessionData::new(
            catalog,
            Principal::new("parent", "Родитель", vec![]),
        )))
        .build();
    let module = engine.compile_entry(r#"
        Аргументы = Новый Массив;
        Аргументы.Добавить(Справочники.ЗамерВременныхТаблиц.ПолучитьСсылку(Новый УникальныйИдентификатор("12345678-1234-4321-abcd-0123456789ab")));
        Задание = ФоновыеЗадания.Выполнить("Работы.Проверить", Аргументы);
        Задание = Задание.ОжидатьЗавершенияВыполнения();
        Если Задание.Состояние = СостояниеФоновогоЗадания.ЗавершеноАварийно Тогда ВызватьИсключение Задание.ИнформацияОбОшибке.Описание; КонецЕсли;
        Возврат Задание.Состояние = СостояниеФоновогоЗадания.Завершено;
    "#).unwrap();
    assert_eq!(parent.run(&module).unwrap(), Value::Boolean(true));
}
