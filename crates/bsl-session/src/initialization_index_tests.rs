use super::*;

#[test]
fn parameter_index_preserves_unicode_names_and_error_order() {
    for (first, second) in [("Параметр", "пАРАМЕТР"), ("I", "ı"), ("Straße", "STRASSE")]
    {
        let parameters = SessionParameters::new("Модуль", [first.to_string()]).unwrap();
        parameters.set(second, Value::Boolean(true)).unwrap();
        assert_eq!(parameters.peek(first).unwrap(), Some(Value::Boolean(true)));
        assert_eq!(parameters.slots.borrow()[0].name, first);
        assert!(matches!(
            SessionParameters::new("Модуль", [first.to_string(), second.to_string()]),
            Err(ParameterError::DuplicateParameterName(name)) if name == second
        ));
    }
    assert!(matches!(
        SessionParameters::new("Модуль", ["A", "a", ""].map(str::to_string)),
        Err(ParameterError::DuplicateParameterName(name)) if name == "a"
    ));
    assert!(matches!(
        SessionParameters::new("Модуль", ["A", "", "a"].map(str::to_string)),
        Err(ParameterError::EmptyParameterName)
    ));
}

#[test]
fn indexed_parameters_preserve_spaces_and_background_isolation_after_clear() {
    let parameters =
        SessionParameters::new("Модуль", ["Тест", " Тест", "Тест "].map(str::to_string)).unwrap();
    parameters.set(" ТЕСТ", Value::Boolean(true)).unwrap();
    let child = parameters.copy_for_background();
    parameters.clear();
    assert_eq!(parameters.peek(" Тест").unwrap(), None);
    assert_eq!(child.peek(" Тест").unwrap(), Some(Value::Boolean(true)));
    assert_eq!(child.peek("Тест").unwrap(), None);
    assert_eq!(child.peek("Тест ").unwrap(), None);
    child.set("тест", Value::Boolean(false)).unwrap();
    assert_eq!(parameters.peek("Тест").unwrap(), None);
    assert!(matches!(
        child.set("Нет", Value::Undefined),
        Err(ParameterError::UnknownParameter(name)) if name == "Нет"
    ));
}

#[test]
fn factory_selects_only_separation_objects_in_catalog_order() {
    use bsl_config::{ConfigurationModule, ModuleExecution};
    use bsl_orm::{CommonAttributeSeparationParameters, MetadataObject};

    let code = ConfigurationCodeSnapshot::new(
        "indexed",
        vec![ConfigurationModule::new(
            "session",
            "МодульСеанса",
            "Процедура УстановкаПараметровСеанса(Имена) Экспорт КонецПроцедуры",
            ModuleKind::Session,
            ModuleExecution::new().with_server(true),
        )],
    )
    .unwrap();
    let mut objects = (0..100)
        .map(|index| {
            MetadataObject::new(
                format!("id{index}"),
                format!("Объект{index}"),
                MetadataKind::Catalog,
            )
        })
        .collect::<Vec<_>>();
    objects.push(
        MetadataObject::new("separator", "Разделитель", MetadataKind::CommonAttribute)
            .with_common_attribute_data_separation(true),
    );
    objects.push(MetadataObject::new(
        "use",
        "Использование",
        MetadataKind::SessionParameter,
    ));
    let catalog = MetadataCatalog::new("indexed", objects)
        .unwrap()
        .with_common_attribute_separation_parameters(
            "separator",
            CommonAttributeSeparationParameters {
                value_parameter: None,
                use_parameter: Some("use".into()),
                value_types: None,
            },
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
    assert_eq!(factory.separation_objects.as_ref(), &[100]);
    for number in [1, 2] {
        let session = factory
            .open(
                Principal::new("user", "Тест", vec![]),
                SessionResources::new(Box::new(()), number),
            )
            .unwrap();
        assert_eq!(
            session.parameters().peek("Использование").unwrap(),
            Some(Value::Boolean(false))
        );
        session
            .parameters()
            .set("Использование", Value::Boolean(true))
            .unwrap();
    }
}
