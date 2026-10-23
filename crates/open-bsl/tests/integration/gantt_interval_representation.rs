use open_bsl::{Engine, Value};

#[test]
fn gantt_interval_representation_resolves_all_native_member_aliases() {
    let engine = Engine::builder().build().unwrap();
    for (russian, english) in [
        ("Плоский", "Flat"),
        ("Объемный", "ThreeDimensional"),
        ("Ромб", "Rhomb"),
        ("Градиент", "Gradient"),
    ] {
        let source = format!(
            "Возврат ОтображениеИнтервалаДиаграммыГанта.{russian} = GanttChartIntervalRepresentation.{english};"
        );
        let module = engine.compile_entry(&source).unwrap();
        assert_eq!(
            engine.new_state().run(&module).unwrap(),
            Value::Boolean(true)
        );
    }
    let module = engine.compile_entry(
        "Возврат отображениеинтерваладиаграммыганта.плоский = GanttChartIntervalRepresentation.Flat;",
    ).unwrap();
    assert_eq!(
        engine.new_state().run(&module).unwrap(),
        Value::Boolean(true)
    );
}

#[test]
fn gantt_interval_presentations_match_the_native_receipt() {
    let receipt = include_str!(
        "../../../../tests/conformance/measure/measure-gantt-interval-representation.platform.txt"
    );
    let engine = Engine::builder().build().unwrap();
    for (id, expression) in [
        ("gantt.ROOT", "ОтображениеИнтервалаДиаграммыГанта"),
        (
            "gantt.TYPE",
            "ТипЗнч(ОтображениеИнтервалаДиаграммыГанта.Плоский)",
        ),
        ("gantt.Flat", "ОтображениеИнтервалаДиаграммыГанта.Плоский"),
        (
            "gantt.ThreeDimensional",
            "ОтображениеИнтервалаДиаграммыГанта.Объемный",
        ),
        ("gantt.Rhomb", "ОтображениеИнтервалаДиаграммыГанта.Ромб"),
        (
            "gantt.Gradient",
            "ОтображениеИнтервалаДиаграммыГанта.Градиент",
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
