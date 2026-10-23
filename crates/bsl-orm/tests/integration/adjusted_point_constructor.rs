use open_bsl::{Engine, Error, SemaError, Value};

fn engine() -> Engine {
    Engine::builder()
        .register_library(bsl_orm::library())
        .build()
        .unwrap()
}

#[test]
fn dormant_adjusted_point_constructor_accepts_measured_aliases_and_arities() {
    let engine = engine();
    for (name, args) in [
        ("МоментВремениСУточнениемПериода", "Дата(2024, 1, 2)"),
        ("PointInTimeWithPeriodAdjustment", "Дата(2024, 1, 2), 1"),
        (
            "pointintimewithperiodadjustment",
            "Дата(2024, 1, 2), 1, Неопределено",
        ),
    ] {
        let module = engine
            .compile(&format!(
                "Функция НеВызывается() Возврат Новый {name}({args}); КонецФункции Возврат 7;"
            ))
            .unwrap();
        assert_eq!(
            engine.new_state().run(&module).unwrap(),
            Value::number_from_i64(7)
        );
    }
}

#[test]
fn adjusted_point_type_aliases_match_the_native_type_presentation() {
    let engine = engine();
    let module = engine.compile("Возврат Тип(\"МоментВремениСУточнениемПериода\") = Тип(\"PointInTimeWithPeriodAdjustment\") И Строка(Тип(\"PointInTimeWithPeriodAdjustment\")) = \"Point in time with period adjustment\";").unwrap();
    assert_eq!(
        engine.new_state().run(&module).unwrap(),
        Value::Boolean(true)
    );
}

#[test]
fn actual_adjusted_point_construction_fails_explicitly() {
    let engine = engine();
    for args in [
        "Дата(2024, 1, 2)",
        "Дата(2024, 1, 2), 1",
        "Дата(2024, 1, 2), 1, Неопределено",
    ] {
        let module = engine
            .compile(&format!(
                "Возврат Новый PointInTimeWithPeriodAdjustment({args});"
            ))
            .unwrap();
        let error = engine.new_state().run(&module).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("объектная модель момента времени с уточнением периода не реализована"),
            "{error:?}"
        );
    }
}

#[test]
fn adjusted_point_declaration_keeps_arity_and_unregistered_diagnostics() {
    let engine = engine();
    for args in ["", "1, 2, 3, 4"] {
        let error = engine
            .compile(&format!(
                "Возврат Новый PointInTimeWithPeriodAdjustment({args});"
            ))
            .err()
            .unwrap();
        assert!(
            matches!(
                error,
                Error::Semantic(SemaError::ArgumentCountMismatch { .. })
            ),
            "{error:?}"
        );
    }
    let error = Engine::builder()
        .build()
        .unwrap()
        .compile("Возврат Новый МоментВремениСУточнениемПериода(1);")
        .err()
        .unwrap();
    assert!(
        matches!(error, Error::Semantic(SemaError::UnsupportedConstructor(_))),
        "{error:?}"
    );
}
