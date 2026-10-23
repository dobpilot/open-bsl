use bsl_config::{ConfigurationCodeSnapshot, ConfigurationModule, ModuleExecution, ModuleKind};
use bsl_orm::{ConfigurationEnumValue, MetadataCatalog, MetadataKind, MetadataObject, Principal};
use bsl_session::{SessionFactory, SessionKind, SessionResources};
use open_bsl::{Engine, GraphLimits, RuntimeShapes, SerializedValueGraph, Value};
use std::sync::Arc;

fn catalog(member: Option<&str>) -> Arc<MetadataCatalog> {
    Arc::new(
        MetadataCatalog::new(
            "configuration",
            vec![MetadataObject::new(
                "enum",
                "Статусы",
                MetadataKind::Enumeration,
            )],
        )
        .unwrap()
        .with_enumeration_values(
            member
                .map(|id| ConfigurationEnumValue::new("enum", id, "Активен"))
                .into_iter()
                .collect(),
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

fn factory(catalog: Arc<MetadataCatalog>, kind: SessionKind) -> SessionFactory {
    let code = ConfigurationCodeSnapshot::new(
        "configuration",
        vec![ConfigurationModule::new(
            "session",
            "МодульСеанса",
            "Процедура УстановкаПараметровСеанса(Требуемые) КонецПроцедуры",
            ModuleKind::Session,
            ModuleExecution::new().with_server(true),
        )],
    )
    .unwrap();
    SessionFactory::new(code, catalog, "МодульСеанса", kind, "8.3.27.2342").unwrap()
}

#[test]
fn enumeration_graph_restores_in_an_independent_background_session() {
    let parent_catalog = catalog(Some("member"));
    let parent_engine = engine(&parent_catalog);
    let parent_shapes = RuntimeShapes::seeded(vec![], vec![], Some(parent_engine.registry()));
    let mut parent = factory(parent_catalog, SessionKind::ThinClient)
        .open(
            Principal::new("parent", "Родитель", vec![]),
            SessionResources::new(Box::new("parent connection"), 1),
        )
        .unwrap();
    let producer = parent
        .instantiate(
            "Функция Значение() Экспорт Возврат Перечисления.Статусы.Активен; КонецФункции",
        )
        .unwrap();
    let value = parent.call(&producer, "Значение", vec![]).unwrap().0;
    let graph =
        SerializedValueGraph::capture(&[value], &parent_shapes, &GraphLimits::default()).unwrap();

    // Получатель имеет собственные каталог, пользователя и соединение.
    let child_catalog = catalog(Some("member"));
    let child_engine = engine(&child_catalog);
    let mut child_shapes = RuntimeShapes::seeded(vec![], vec![], Some(child_engine.registry()));
    let mut child = factory(child_catalog, SessionKind::BackgroundJob)
        .open_background(
            Principal::new("child", "Получатель", vec![]),
            SessionResources::new(Box::new("child connection"), 2),
            "Обработка.Проверка",
            &parent,
        )
        .unwrap();
    drop(parent);
    drop(parent_shapes);
    drop(parent_engine);
    let restored = graph.materialize(&mut child_shapes).unwrap();
    let consumer = child
        .instantiate(
            "Функция Проверить(Значение) Экспорт Возврат Значение = Перечисления.Статусы.Активен И ТипЗнч(Значение) = Тип(\"EnumRef.Статусы\"); КонецФункции",
        )
        .unwrap();
    assert_eq!(
        child.call(&consumer, "Проверить", restored).unwrap().0,
        Value::Boolean(true)
    );
    assert_eq!(child.principal().id(), "child");

    let missing_member_engine = engine(&catalog(Some("other-member")));
    let mut missing_member_shapes =
        RuntimeShapes::seeded(vec![], vec![], Some(missing_member_engine.registry()));
    assert!(graph.materialize(&mut missing_member_shapes).is_err());
    let mut missing_type_shapes = RuntimeShapes::seeded(vec![], vec![], None);
    assert!(graph.materialize(&mut missing_type_shapes).is_err());
}
