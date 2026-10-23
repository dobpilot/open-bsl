use std::sync::Arc;

use bsl_config::{ConfigurationCodeSnapshot, ConfigurationModule, ModuleExecution, ModuleKind};
use bsl_orm::{
    BusinessProcessRoutePoint, MetadataCatalog, MetadataKind, MetadataObject, Principal,
};
use bsl_session::{SessionFactory, SessionKind, SessionResources};
use open_bsl::Value;

fn catalog() -> MetadataCatalog {
    let objects = ["first", "second", "empty", "unknown"]
        .into_iter()
        .map(|id| {
            let object = MetadataObject::new(id, id, MetadataKind::BusinessProcess);
            if id == "first" {
                object.with_manager_module("БизнесПроцесс.first.МодульМенеджера")
            } else {
                object
            }
        })
        .collect();
    let mut catalog = MetadataCatalog::new("route-control", objects).unwrap();
    for owner in ["first", "second", "empty"] {
        let points = if owner == "empty" {
            vec![]
        } else {
            ["Действие1", "Действие2"]
                .into_iter()
                .enumerate()
                .map(|(index, name)| BusinessProcessRoutePoint {
                    owner_id: owner.into(),
                    point_id: format!("point{index}"),
                    name: name.into(),
                    synonyms: vec![],
                })
                .collect()
        };
        catalog = catalog
            .with_business_process_route_points(
                owner,
                Some(&format!("type-{owner}")),
                Some(points),
                &[],
            )
            .unwrap();
    }
    catalog
}

fn factory(manager: &str) -> SessionFactory {
    let execution = ModuleExecution::new().with_server(true);
    SessionFactory::new(ConfigurationCodeSnapshot::new("route-control", vec![
        ConfigurationModule::new("session", "МодульСеанса", "Процедура УстановкаПараметровСеанса(Имена) КонецПроцедуры Функция Маркер() Экспорт Возврат БизнесПроцессы.first.Проверить(); КонецФункции", ModuleKind::Session, execution),
        ConfigurationModule::new("manager", "БизнесПроцесс.first.МодульМенеджера", manager, ModuleKind::Manager, execution).with_owner("first"),
    ]).unwrap(), Arc::new(catalog()), "МодульСеанса", SessionKind::ThinClient, "8.3.27.2342").unwrap()
}

#[test]
fn measured_named_references_and_collection_assignment_preserve_identity() {
    let factory =
        factory("Функция Проверить() Экспорт Возврат ТочкиМаршрута.Действие1; КонецФункции");
    let mut session = factory
        .open(
            Principal::new("user", "Тест", vec![]),
            SessionResources::new(Box::new(()), 1),
        )
        .unwrap();
    let module = session
        .instantiate(
            r#"
Функция Проверить() Экспорт
    Точки = БизнесПроцессы.first.ТочкиМаршрута;
    Первая = Точки.Действие1;
    Точки.Действие1 = Точки.Действие2;
    Точки.Действие1 = 42;
    Счет = 0;
    Для Каждого Точка Из Точки Цикл Счет = Счет + 1; КонецЦикла;
    Возврат Точки.Количество() = 2 И Счет = 2
        И Первая = Точки.действие1
        И Первая <> Точки.Действие2
        И Первая = БизнесПроцессы.first.Проверить()
        И Первая = БизнесПроцессы.first.RoutePoints.Действие1
        И Первая <> БизнесПроцессы.second.ТочкиМаршрута.Действие1
        И ЗначениеЗаполнено(Первая)
        И Строка(Первая) = "Действие1"
        И ТипЗнч(Первая) = Тип("BusinessProcessRoutePointRef.first")
        И ТипЗнч(Первая) = Тип("ТочкаМаршрутаБизнесПроцессаСсылка.first")
        И БизнесПроцессы.empty.ТочкиМаршрута.Количество() = 0;
КонецФункции
"#,
        )
        .unwrap();
    assert_eq!(
        session.call(&module, "Проверить", vec![]).unwrap().0,
        Value::Boolean(true)
    );
    for expression in [
        r#"Тип("ТочкиМаршрута")"#,
        "БизнесПроцессы.unknown.ТочкиМаршрута",
        "БизнесПроцессы.first.ТочкиМаршрута.НетТакойТочки",
    ] {
        let module = session
            .instantiate(&format!(
                "Функция Проверить() Экспорт Возврат {expression}; КонецФункции"
            ))
            .unwrap();
        assert!(session.call(&module, "Проверить", vec![]).is_err());
    }
    let module = session
        .instantiate(
            "Процедура Проверить() Экспорт БизнесПроцессы.first.ТочкиМаршрута = 42; КонецПроцедуры",
        )
        .unwrap();
    assert!(session.call(&module, "Проверить", vec![]).is_err());
}

#[test]
fn variables_keep_the_existing_precedence_over_the_manager_property() {
    let factory = factory(
        "Перем ТочкиМаршрута;\nФункция Проверить() Экспорт ТочкиМаршрута = 17; Возврат ТочкиМаршрута; КонецФункции\nФункция Локальная() Экспорт Перем RoutePoints; RoutePoints = 23; Возврат RoutePoints; КонецФункции",
    );
    let mut session = factory
        .open(
            Principal::new("user", "Тест", vec![]),
            SessionResources::new(Box::new(()), 1),
        )
        .unwrap();
    for (name, expected) in [("Проверить", 17), ("Локальная", 23)] {
        let module = session
            .instantiate(&format!(
                "Функция Проба() Экспорт Возврат БизнесПроцессы.first.{name}(); КонецФункции"
            ))
            .unwrap();
        assert_eq!(
            session.call(&module, "Проба", vec![]).unwrap().0,
            Value::number_from_i64(expected)
        );
    }
}
