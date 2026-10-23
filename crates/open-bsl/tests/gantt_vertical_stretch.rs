use open_bsl::{Engine, Value};

#[test]
fn gantt_vertical_stretch_resolves_all_native_member_aliases() {
    let engine = Engine::builder().build().unwrap();
    for (russian, english) in [
        ("НеРастягивать", "None"),
        ("РастягиватьСтроки", "StretchRows"),
        ("РастягиватьСтрокиИДанные", "StretchRowsAndData"),
    ] {
        let source = format!(
            "Возврат РастягиваниеПоВертикалиДиаграммыГанта.{russian} = GanttChartVerticalStretch.{english};"
        );
        let module = engine.compile_entry(&source).unwrap();
        assert_eq!(
            engine.new_state().run(&module).unwrap(),
            Value::Boolean(true)
        );
    }
    let module = engine.compile_entry(
        "Возврат растягиваниеповертикалидиаграммыганта.нерастягивать = GanttChartVerticalStretch.None;",
    ).unwrap();
    assert_eq!(
        engine.new_state().run(&module).unwrap(),
        Value::Boolean(true)
    );
}

#[test]
fn gantt_vertical_stretch_presentations_match_the_native_receipt() {
    let receipt = include_str!(
        "../../../tests/conformance/measure/measure-gantt-vertical-stretch.platform.txt"
    );
    let engine = Engine::builder().build().unwrap();
    for (id, expression) in [
        (
            "gantt_stretch.root",
            "РастягиваниеПоВертикалиДиаграммыГанта",
        ),
        (
            "gantt_stretch.type",
            "ТипЗнч(РастягиваниеПоВертикалиДиаграммыГанта.НеРастягивать)",
        ),
        (
            "gantt_stretch.None",
            "РастягиваниеПоВертикалиДиаграммыГанта.НеРастягивать",
        ),
        (
            "gantt_stretch.StretchRows",
            "РастягиваниеПоВертикалиДиаграммыГанта.РастягиватьСтроки",
        ),
        (
            "gantt_stretch.StretchRowsAndData",
            "РастягиваниеПоВертикалиДиаграммыГанта.РастягиватьСтрокиИДанные",
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
