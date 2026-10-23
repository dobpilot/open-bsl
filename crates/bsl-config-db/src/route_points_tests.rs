use super::*;
use open_sdbl::metadata::{
    DbNames, SchemaStorage, parse_config_resource_bounded, resolve_metadata,
};

const OWNER: &str = "6db4bbe0-ce4c-4389-ad25-9a10c1857192";

fn project(empty: bool) -> MetadataCatalog {
    let mut owner = parse_config_resource_bounded(
        OWNER,
        include_bytes!("../tests/fixtures/business-route-points/business-process.deflate"),
        100000,
    )
    .unwrap();
    let graph = parse_config_resource_bounded(
        &format!("{OWNER}.7"),
        if empty {
            include_bytes!("../tests/fixtures/business-route-points/empty-route-points.deflate")
        } else {
            include_bytes!("../tests/fixtures/business-route-points/route-points.deflate")
        },
        100000,
    )
    .unwrap();
    let (graph_owner, points) = graph.route_points.unwrap();
    assert_eq!(graph_owner.as_str(), OWNER);
    owner
        .descriptors
        .iter_mut()
        .find(|descriptor| descriptor.object_guid.as_str() == OWNER)
        .unwrap()
        .business_process_route_points = Some(points);
    let metadata = resolve_metadata(
        DbNames::from_entries(vec![]),
        owner.descriptors,
        SchemaStorage {
            tables: vec![],
            anomalies: vec![],
        },
        vec![],
    )
    .snapshot;
    let catalog = MetadataCatalog::new(
        "native-route-control",
        vec![
            MetadataObject::new(OWNER, "ЗамерБизнесПроцесса", MetadataKind::BusinessProcess),
            MetadataObject::new("language-ru", "Русский", MetadataKind::Language)
                .with_string_property("КодЯзыка", "ru"),
        ],
    )
    .unwrap()
    .with_configuration(
        "Контроль",
        "1",
        bsl_orm::ScriptVariant::Russian,
        bsl_orm::CompatibilityMode::DontUse,
        Vec::<String>::new(),
    )
    .unwrap()
    .with_primary_language("language-ru")
    .unwrap();
    crate::project_catalog_values(catalog, &metadata).unwrap()
}

#[test]
fn native_sdk_composition_reaches_the_orm_without_global_point_indexing() {
    let catalog = project(false);
    let points = catalog.business_process_route_points(OWNER).unwrap();
    assert_eq!(
        points
            .iter()
            .map(|point| point.name.as_str())
            .collect::<Vec<_>>(),
        [
            "Действие1",
            "Действие2",
            "Условие1",
            "Старт",
            "Завершение",
            "ВыборВарианта1",
            "Разделение1",
            "Слияние1",
            "Обработка1",
            "ВложенныйПроцесс1",
        ]
    );
    assert!(points.iter().all(|point| point.owner_id == OWNER && catalog.object_by_id(&point.point_id).is_none()));
    assert_eq!(
        points[5].synonyms,
        vec![("ru".into(), "Выбор варианта1".into())]
    );
    let ty = catalog.route_point_reference_types().next().unwrap();
    assert!(
        ty.identity()
            .contains("bf5448cf-35b6-48b0-ba80-5b9ae9f506c5")
    );
    assert!(
        project(true)
            .business_process_route_points(OWNER)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn native_ten_point_collection_executes_through_the_existing_session_facade() {
    use bsl_config::{ConfigurationCodeSnapshot, ConfigurationModule, ModuleExecution, ModuleKind};
    use bsl_session::{SessionFactory, SessionKind, SessionResources};
    use std::sync::Arc;
    let catalog = project(false);
    let code = ConfigurationCodeSnapshot::new(
        "native-route-control",
        vec![ConfigurationModule::new(
            "session",
            "МодульСеанса",
            "Процедура УстановкаПараметровСеанса(Имена) КонецПроцедуры",
            ModuleKind::Session,
            ModuleExecution::new().with_server(true),
        )],
    )
    .unwrap();
    let factory = SessionFactory::new(
        code,
        Arc::new(catalog),
        "МодульСеанса",
        SessionKind::ThinClient,
        "8.3.27.2342",
    )
    .unwrap();
    let mut session = factory
        .open(
            bsl_orm::Principal::new("user", "Тест", vec![]),
            SessionResources::new(Box::new(()), 1),
        )
        .unwrap();
    let module = session.instantiate(r#"
Функция Проверить() Экспорт
    Коллекция = БизнесПроцессы.ЗамерБизнесПроцесса.ТочкиМаршрута;
    Первая = Коллекция.Действие1;
    Коллекция.Действие1 = Коллекция.Действие2;
    Коллекция.Действие1 = 42;
    Счет = 0;
    Порядок = "";
    Для Каждого Точка Из Коллекция Цикл
        Счет = Счет + 1;
        Порядок = Порядок + Строка(Точка) + "/";
    КонецЦикла;
    Возврат Счет = 10 И Коллекция.Количество() = 10
        И Первая = Коллекция.действие1 И Первая <> Коллекция.Действие2
        И ЗначениеЗаполнено(Первая)
        И ТипЗнч(Коллекция) = Тип("ТочкиМаршрутаБизнесПроцесса")
        И ТипЗнч(Коллекция) = Тип("BusinessProcessRoutePoints")
        И ТипЗнч(Первая) = Тип("BusinessProcessRoutePointRef.ЗамерБизнесПроцесса")
        И Строка(ТипЗнч(Первая)) = "Замер бизнес процесса(route point)"
        И Порядок = "Действие1/Действие2/Условие1/Старт/Завершение/Выбор варианта1/Разделение1/Слияние1/Обработка1/Вложенный процесс1/";
КонецФункции
"#).unwrap();
    assert_eq!(
        session.call(&module, "Проверить", vec![]).unwrap().0,
        open_bsl::Value::Boolean(true)
    );
}
