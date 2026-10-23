use super::*;

fn state() -> open_bsl::State {
    open_bsl::Engine::builder()
        .register_library(library())
        .build()
        .unwrap()
        .state_builder()
        .build()
}

#[test]
fn boundary_preserves_measured_values_and_default_kind() {
    let mut state = state();
    for expression in [
        "Дата(2026, 10, 19, 12)",
        "Новый МоментВремени(Дата(2026, 10, 19, 12))",
        "1",
        "Истина",
        "\"проба\"",
        "Null",
        "Неопределено",
    ] {
        let source = format!(
            "ЗначениеПробы = {expression}; ГраницаПробы = Новый Граница(ЗначениеПробы); \
             Если ТипЗнч(ГраницаПробы.Значение) <> ТипЗнч(ЗначениеПробы) Тогда Возврат Ложь; КонецЕсли; \
             Если ТипЗнч(ЗначениеПробы) <> Тип(\"Null\") Тогда \
             Если ГраницаПробы.Value <> ЗначениеПробы Тогда Возврат Ложь; КонецЕсли; КонецЕсли; \
             Возврат ГраницаПробы.BoundaryType = ВидГраницы.Включая;"
        );
        assert_eq!(state.exec(&source).unwrap(), Value::Boolean(true));
    }
}

#[test]
fn boundary_aliases_and_measured_equality_match_native_control() {
    assert_eq!(
        state()
            .exec(
                "ГраницаПробы = Новый Boundary(1); \
                 Возврат ТипЗнч(ГраницаПробы) = Тип(\"Граница\") \
                 И ТипЗнч(ГраницаПробы) = Тип(\"Boundary\") \
                 И ГраницаПробы = Новый Граница(1) \
                 И Новый Граница(Неопределено) = Новый Boundary(Undefined) \
                 И ГраницаПробы <> Новый Граница(Неопределено) \
                 И ГраницаПробы <> Новый Граница(1, ВидГраницы.Исключая);"
            )
            .unwrap(),
        Value::Boolean(true)
    );
}

#[test]
fn boundary_properties_are_readonly_and_invalid_kind_is_rejected() {
    let mut state = state();
    for source in [
        "ГраницаПробы = Новый Граница(1); ГраницаПробы.Value = 2;",
        "ГраницаПробы = Новый Граница(1); ГраницаПробы.ВидГраницы = ВидГраницы.Исключая;",
        "Возврат Новый Граница(1).МоментВремени;",
        "Возврат Новый Boundary(1).PointInTime;",
        "Возврат Новый Граница(1, Неопределено);",
        "Возврат Новый Boundary(1, 1);",
    ] {
        let source = format!(
            "ГраницаПроверки = Новый Граница(1); Попытка {source} Возврат Ложь; \
             Исключение Возврат Истина; КонецПопытки;"
        );
        assert_eq!(state.exec(&source).unwrap(), Value::Boolean(true));
    }
}

#[test]
fn boundary_date_and_point_equality_and_english_presentation_match_native_control() {
    assert_eq!(
        state()
            .exec(
                "ДатаПробы = Дата(2026, 10, 19, 12); \
             МоментПробы = Новый МоментВремени(ДатаПробы); \
             Возврат Новый Граница(ДатаПробы) = Новый Граница(ДатаПробы) \
             И Новый Граница(МоментПробы) = Новый Boundary(МоментПробы) \
             И Строка(ТипЗнч(Новый Граница(1))) = \"Bound\" \
             И Строка(Новый Граница(1)) = \"Boundary\";"
            )
            .unwrap(),
        Value::Boolean(true)
    );
}
