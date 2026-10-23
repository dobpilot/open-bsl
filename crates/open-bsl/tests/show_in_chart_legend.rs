use open_bsl::{Engine, Value};

#[test]
fn chart_legend_visibility_matches_native_aliases_types_and_presentations() {
    let engine = Engine::builder().build().unwrap();
    let module = engine
        .compile(
            r#"
Возврат ОтображениеВЛегендеДиаграммы.Авто = ShowInChartLegend.Auto
 И ОтображениеВЛегендеДиаграммы.Отображать = showinchartlegend.show
 И ОтображениеВЛегендеДиаграммы.НеОтображать = SHOWINCHARTLEGEND.DONTSHOW
 И ТипЗнч(ShowInChartLegend.Auto) = Тип("ОтображениеВЛегендеДиаграммы")
 И Тип("ShowInChartLegend") = ТипЗнч(ShowInChartLegend.DontShow);
"#,
        )
        .unwrap();
    assert_eq!(
        engine.new_state().run(&module).unwrap(),
        Value::Boolean(true)
    );
    let module = engine
        .compile(
            r#"
Результат = Строка(ShowInChartLegend.Auto) + ";" + Строка(ShowInChartLegend.Show) + ";" + Строка(ShowInChartLegend.DontShow) + ";";
Возврат Результат;
"#,
        )
        .unwrap();
    assert_eq!(
        engine.new_state().run(&module).unwrap(),
        Value::Str("Auto;Show;Do not show;".into())
    );
}

#[test]
fn chart_legend_visibility_rejects_primitive_ordering() {
    let engine = Engine::builder().build().unwrap();
    let module = engine.compile("Попытка Результат = ShowInChartLegend.Auto < ShowInChartLegend.Show; Возврат Ложь; Исключение Возврат Истина; КонецПопытки;").unwrap();
    assert_eq!(
        engine.new_state().run(&module).unwrap(),
        Value::Boolean(true)
    );
}
