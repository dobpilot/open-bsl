use open_bsl::{Engine, Value};

#[test]
fn chart_legend_placement_resolves_all_native_member_aliases() {
    let engine = Engine::builder().build().unwrap();
    for (russian, english) in [
        ("Авто", "Auto"),
        ("Низ", "Bottom"),
        ("Верх", "Top"),
        ("Лево", "Left"),
        ("Право", "Right"),
        ("Нет", "None"),
        ("УказываетсяРасположение", "UseCoordinates"),
    ] {
        let source = format!(
            "Возврат РасположениеЛегендыДиаграммы.{russian} = ChartLegendPlacement.{english};"
        );
        let module = engine.compile_entry(&source).unwrap();
        assert_eq!(
            engine.new_state().run(&module).unwrap(),
            Value::Boolean(true)
        );
    }
    let module = engine
        .compile_entry("Возврат расположениелегендыдиаграммы.авто = ChartLegendPlacement.Auto;")
        .unwrap();
    assert_eq!(
        engine.new_state().run(&module).unwrap(),
        Value::Boolean(true)
    );
}

#[test]
fn chart_legend_presentations_match_the_native_receipt() {
    let receipt = include_str!(
        "../../../../tests/conformance/measure/measure-chart-legend-placement.platform.txt"
    );
    let engine = Engine::builder().build().unwrap();
    for (id, expression) in [
        ("legend.root", "РасположениеЛегендыДиаграммы"),
        ("legend.type", "ТипЗнч(РасположениеЛегендыДиаграммы.Авто)"),
        ("legend.Auto", "РасположениеЛегендыДиаграммы.Авто"),
        ("legend.Bottom", "РасположениеЛегендыДиаграммы.Низ"),
        ("legend.Top", "РасположениеЛегендыДиаграммы.Верх"),
        ("legend.Left", "РасположениеЛегендыДиаграммы.Лево"),
        ("legend.Right", "РасположениеЛегендыДиаграммы.Право"),
        ("legend.None", "РасположениеЛегендыДиаграммы.Нет"),
        (
            "legend.UseCoordinates",
            "РасположениеЛегендыДиаграммы.УказываетсяРасположение",
        ),
    ] {
        let expected = receipt
            .lines()
            .find_map(|line| {
                let (key, value) = line.split_once('\t')?;
                (key == id).then_some(value)
            })
            .unwrap();
        let module = engine
            .compile_entry(&format!("Возврат Строка({expression});"))
            .unwrap();
        assert_eq!(
            engine.new_state().run(&module).unwrap(),
            Value::Str(expected.into()),
            "{id}"
        );
    }
}
