use super::*;

#[test]
fn point_in_time_and_date_boundaries_are_captured_as_query_parameters() {
    let mut state = open_bsl::Engine::builder()
        .register_library(bsl_orm::library())
        .build()
        .unwrap()
        .state_builder()
        .build();
    for expression in [
        "Новый МоментВремени(Дата(2026, 10, 19, 12))",
        "Новый Граница(Дата(2026, 10, 19, 12))",
        "Новый Boundary(Дата(2026, 10, 19, 12), BoundaryType.Excluding)",
        "Новый Граница(Новый МоментВремени(Дата(2026, 10, 19, 12)))",
    ] {
        let value = state.exec(&format!("Возврат {expression};")).unwrap();
        assert!(parameter_value(&value).is_ok(), "{expression}");
    }
}

#[test]
fn arbitrary_boundary_payloads_are_not_silently_query_dates() {
    let mut state = open_bsl::Engine::builder()
        .register_library(bsl_orm::library())
        .build()
        .unwrap()
        .state_builder()
        .build();
    for expression in ["1", "Истина", "\"дата\"", "Null", "Неопределено"] {
        let value = state
            .exec(&format!("Возврат Новый Граница({expression});"))
            .unwrap();
        assert!(parameter_value(&value).is_err(), "{expression}");
    }
}

#[test]
fn temporal_capture_preserves_reference_identity_physical_bytes_and_boundary_kind() {
    let owner = "11111111-2222-3333-4444-555555555555";
    let catalog = std::sync::Arc::new(
        bsl_orm::MetadataCatalog::new(
            "temporal-reference",
            vec![MetadataObject::new(owner, "Пример", MetadataKind::Document)],
        )
        .unwrap(),
    );
    let date = BslDate::from_civil(2026, 10, 19, 12, 0, 0).unwrap();
    for uuid in [
        [0; 16],
        [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16],
    ] {
        let reference =
            bsl_orm::configuration_reference_value(catalog.clone(), owner, uuid).unwrap();
        let object = ObjectId::from_bytes(owner.parse::<Guid>().unwrap().to_bytes());
        let id = open_bsl::uuid::to_1c_bytes(uuid);
        assert!(bsl_orm::catalog_reference_parts(&reference).is_none());
        let point = open_bsl::new_point_in_time(date, reference).unwrap();
        let ParameterValue::PointInTime(captured) = parameter_value(&point).unwrap() else {
            panic!("point in time")
        };
        assert_eq!(
            captured.date,
            ParameterDate::new(2026, 10, 19, 12, 0, 0).unwrap()
        );
        assert_eq!(captured.reference, Some(ParameterReference { object, id }));
    }
    let absent = open_bsl::new_point_in_time(date, Value::Undefined).unwrap();
    let ParameterValue::PointInTime(point) = parameter_value(&absent).unwrap() else {
        panic!("point in time")
    };
    assert_eq!(point.reference, None);
    let mut state = open_bsl::Engine::builder()
        .register_library(bsl_orm::library())
        .build()
        .unwrap()
        .state_builder()
        .build();
    let value = state
        .exec("Возврат Новый Граница(Дата(2026, 10, 19, 12), ВидГраницы.Исключая);")
        .unwrap();
    assert_eq!(
        parameter_value(&value).unwrap(),
        ParameterValue::Boundary(ParameterBoundary {
            value: ParameterBoundaryValue::Date(point.date),
            kind: BoundaryKind::Excluding,
        })
    );
    let fractional = open_bsl::new_point_in_time(
        BslDate::from_ticks(date.ticks() + 1).unwrap(),
        Value::Undefined,
    )
    .unwrap();
    assert!(parameter_value(&fractional).is_err());
}
