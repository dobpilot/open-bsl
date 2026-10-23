use open_bsl::{Engine, Value};

#[test]
fn time_scale_day_format_resolves_all_native_member_aliases() {
    let engine = Engine::builder().build().unwrap();
    for (russian, english) in [
        ("ДеньМесяца", "MonthDay"),
        ("ДеньНедели", "WeekDay"),
        ("ДеньМесяцаДеньНедели", "MonthDayWeekDay"),
        ("ДеньНеделиДеньМесяца", "WeekDayMonthDay"),
    ] {
        let source =
            format!("Возврат ФорматДняШкалыВремени.{russian} = TimeScaleDayFormat.{english};");
        let module = engine.compile_entry(&source).unwrap();
        assert_eq!(
            engine.new_state().run(&module).unwrap(),
            Value::Boolean(true)
        );
    }
    let module = engine
        .compile_entry("Возврат форматдняшкалывремени.деньмесяца = TimeScaleDayFormat.MonthDay;")
        .unwrap();
    assert_eq!(
        engine.new_state().run(&module).unwrap(),
        Value::Boolean(true)
    );
}

#[test]
fn time_scale_day_format_presentations_match_the_native_receipt() {
    let receipt = include_str!(
        "../../../../tests/conformance/measure/measure-time-scale-day-format.platform.txt"
    );
    let engine = Engine::builder().build().unwrap();
    for (id, expression) in [
        ("time_scale_day_format.root", "ФорматДняШкалыВремени"),
        (
            "time_scale_day_format.type",
            "ТипЗнч(ФорматДняШкалыВремени.ДеньМесяца)",
        ),
        (
            "time_scale_day_format.MonthDay",
            "ФорматДняШкалыВремени.ДеньМесяца",
        ),
        (
            "time_scale_day_format.WeekDay",
            "ФорматДняШкалыВремени.ДеньНедели",
        ),
        (
            "time_scale_day_format.MonthDayWeekDay",
            "ФорматДняШкалыВремени.ДеньМесяцаДеньНедели",
        ),
        (
            "time_scale_day_format.WeekDayMonthDay",
            "ФорматДняШкалыВремени.ДеньНеделиДеньМесяца",
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
