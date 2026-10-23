use open_bsl::{Engine, Value};

#[test]
fn data_composition_chart_legend_placement_matches_native_aliases_types_and_presentations() {
    let engine = Engine::builder().build().unwrap();
    let mut state = engine.new_state();
    for (russian, english, presentation) in [
        ("Нет", "None", "None"),
        ("Лево", "Left", "Left"),
        ("Право", "Right", "Right"),
        ("Верх", "Top", "Top"),
        ("Низ", "Bottom", "Bottom"),
    ] {
        let source = format!(
            "ЗначениеПробы = РасположениеЛегендыДиаграммыКомпоновкиДанных.{russian};
            Возврат ЗначениеПробы = DataCompositionChartLegendPlacement.{english}
                И Строка(ЗначениеПробы) = \"{presentation}\"
                И Строка(ТипЗнч(ЗначениеПробы)) = \"DataCompositionChartLegendPlacement\"
                И ТипЗнч(ЗначениеПробы) = Тип(\"DataCompositionChartLegendPlacement\")
                И ТипЗнч(ЗначениеПробы) = Тип(\"РасположениеЛегендыДиаграммыКомпоновкиДанных\");"
        );
        assert_eq!(state.exec(&source).unwrap(), Value::Boolean(true));
    }
    assert_eq!(
        state
            .exec(
                "Возврат Строка(РасположениеЛегендыДиаграммыКомпоновкиДанных)
                    = \"EnumDataCompositionChartLegendPlacement\"
                    И Строка(ТипЗнч(DataCompositionChartLegendPlacement))
                    = \"EnumDataCompositionChartLegendPlacement\";",
            )
            .unwrap(),
        Value::Boolean(true)
    );
}

#[test]
fn variables_keep_precedence_over_the_data_composition_chart_legend_placement_enum() {
    let engine = Engine::builder().build().unwrap();
    let mut state = engine.new_state();
    for source in [
        "Перем РасположениеЛегендыДиаграммыКомпоновкиДанных;
        РасположениеЛегендыДиаграммыКомпоновкиДанных = 17;
        Возврат РасположениеЛегендыДиаграммыКомпоновкиДанных;",
        "Функция Проба()
            DataCompositionChartLegendPlacement = 17;
            Возврат DataCompositionChartLegendPlacement;
        КонецФункции
        Возврат Проба();",
    ] {
        assert_eq!(state.exec(source).unwrap(), Value::number_from_i64(17));
    }
}

#[test]
fn primitive_ordering_of_data_composition_chart_legend_placements_is_catchable() {
    let engine = Engine::builder().build().unwrap();
    assert_eq!(engine.new_state().exec(
        "Попытка ЗначениеПробы = DataCompositionChartLegendPlacement.None < DataCompositionChartLegendPlacement.Left; Исключение Возврат Истина; КонецПопытки Возврат Ложь;"
    ).unwrap(), Value::Boolean(true));
}

#[test]
fn data_composition_chart_legend_is_distinct_from_the_chart_legend() {
    let engine = Engine::builder().build().unwrap();
    assert_eq!(engine.new_state().exec(
        "Возврат ТипЗнч(DataCompositionChartLegendPlacement.None) <> ТипЗнч(ChartLegendPlacement.None);"
    ).unwrap(), Value::Boolean(true));
    assert!(matches!(
        engine.compile("Возврат DataCompositionChartLegendPlacement.Auto;"),
        Err(open_bsl::Error::Semantic(_))
    ));
}
