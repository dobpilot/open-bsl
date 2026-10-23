use super::*;
use bsl_config::{ConfigurationCodeSnapshot, ConfigurationModule, ModuleExecution, ModuleKind};
use bsl_orm::{CommonAttributeSeparationParameters, MetadataObject};
use open_bsl::{ColumnType, TypeId, TypeRef};

fn factory(kind: SessionKind) -> SessionFactory {
    factory_with_value_types(kind, true)
}

fn factory_with_value_types(kind: SessionKind, known: bool) -> SessionFactory {
    let code = ConfigurationCodeSnapshot::new(
        "separation-test",
        vec![ConfigurationModule::new(
            "session",
            "МодульСеанса",
            "Процедура УстановкаПараметровСеанса(Имена) Экспорт
                Если Имена = Неопределено Тогда
                    Флаг = ПараметрыСеанса.Использование;
                    Текст = ПараметрыСеанса.ЗначениеСтроки;
                    Номер = ПараметрыСеанса.ЗначениеЧисла;
                Иначе
                    ВызватьИсключение \"ordinary-request:\" + Имена[0];
                КонецЕсли;
             КонецПроцедуры",
            ModuleKind::Session,
            ModuleExecution::new().with_server(true),
        )],
    )
    .unwrap();
    let mut catalog = MetadataCatalog::new(
        "separation-test",
        vec![
            MetadataObject::new(
                "text-attribute",
                "СтроковыйРазделитель",
                MetadataKind::CommonAttribute,
            )
            .with_common_attribute_data_separation(true),
            MetadataObject::new(
                "number-attribute",
                "ЧисловойРазделитель",
                MetadataKind::CommonAttribute,
            )
            .with_common_attribute_data_separation(true),
            MetadataObject::new("use", "Использование", MetadataKind::SessionParameter),
            MetadataObject::new("text", "ЗначениеСтроки", MetadataKind::SessionParameter),
            MetadataObject::new("number", "ЗначениеЧисла", MetadataKind::SessionParameter),
            MetadataObject::new(
                "ordinary",
                "ОбычныйПараметр",
                MetadataKind::SessionParameter,
            ),
        ],
    )
    .unwrap();
    for (owner, parameter, kind) in [
        ("text-attribute", "text", TypeId::String),
        ("number-attribute", "number", TypeId::Number),
    ] {
        catalog = catalog
            .with_common_attribute_separation_parameters(
                owner,
                CommonAttributeSeparationParameters {
                    value_parameter: Some(parameter.into()),
                    use_parameter: Some("use".into()),
                    value_types: known.then(|| {
                        vec![ColumnType {
                            id: TypeRef::Native(kind),
                            quals: vec![],
                        }]
                    }),
                },
            )
            .unwrap();
    }
    SessionFactory::new(code, Arc::new(catalog), "МодульСеанса", kind, "8.3.27.2342").unwrap()
}

#[test]
fn fresh_sessions_seed_only_bound_parameters_before_the_original_handler() {
    let factory = factory(SessionKind::ThinClient);
    let mut first = factory
        .open(
            Principal::new("user", "Тест", vec![]),
            SessionResources::new(Box::new(()), 1),
        )
        .unwrap();
    assert_eq!(
        first.parameters().peek("Использование").unwrap(),
        Some(Value::Boolean(false))
    );
    assert_eq!(
        first.parameters().peek("ЗначениеСтроки").unwrap(),
        Some(Value::Str("".into()))
    );
    assert_eq!(
        first.parameters().peek("ЗначениеЧисла").unwrap(),
        Some(Value::number_from_i64(0))
    );
    assert_eq!(first.parameters().peek("ОбычныйПараметр").unwrap(), None);
    let error = first
        .execute("Проба = ПараметрыСеанса.ОбычныйПараметр;")
        .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("ordinary-request:ОбычныйПараметр")
    );
    first
        .parameters()
        .set("ЗначениеСтроки", Value::Str("parent".into()))
        .unwrap();
    let second = factory
        .open(
            Principal::new("user", "Тест", vec![]),
            SessionResources::new(Box::new(()), 2),
        )
        .unwrap();
    assert_eq!(
        second.parameters().peek("ЗначениеСтроки").unwrap(),
        Some(Value::Str("".into()))
    );
}

#[test]
fn inherited_background_parameters_are_not_replaced_with_fresh_defaults() {
    let parent = factory(SessionKind::ThinClient)
        .open(
            Principal::new("user", "Тест", vec![]),
            SessionResources::new(Box::new(()), 1),
        )
        .unwrap();
    parent
        .parameters()
        .set("Использование", Value::Boolean(true))
        .unwrap();
    parent
        .parameters()
        .set("ЗначениеСтроки", Value::Str("parent".into()))
        .unwrap();
    parent
        .parameters()
        .set("ЗначениеЧисла", Value::number_from_i64(7))
        .unwrap();
    let child = factory(SessionKind::BackgroundJob)
        .open_background(
            Principal::new("user", "Тест", vec![]),
            SessionResources::new(Box::new(()), 2),
            "Проба",
            &parent,
        )
        .unwrap();
    assert_eq!(
        child.parameters().peek("Использование").unwrap(),
        Some(Value::Boolean(true))
    );
    assert_eq!(
        child.parameters().peek("ЗначениеСтроки").unwrap(),
        Some(Value::Str("parent".into()))
    );
    assert_eq!(
        child.parameters().peek("ЗначениеЧисла").unwrap(),
        Some(Value::number_from_i64(7))
    );
}

#[test]
fn unknown_separation_value_type_is_not_replaced_with_an_empty_value() {
    let factory = factory_with_value_types(SessionKind::ThinClient, false);
    let result = factory.open(
        Principal::new("user", "Тест", vec![]),
        SessionResources::new(Box::new(()), 1),
    );
    let error = match result {
        Ok(_) => panic!("неизвестный тип не должен получать начальное значение"),
        Err(error) => error,
    };
    assert!(
        error
            .to_string()
            .contains("начальное значение параметра разделителя")
    );
}
