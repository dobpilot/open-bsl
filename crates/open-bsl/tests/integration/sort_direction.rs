use open_bsl::{Engine, Value};

#[test]
fn sort_direction_has_both_measured_members() {
    let engine = Engine::builder().build().unwrap();
    let value = engine
        .new_state()
        .exec(
            r#"
Возврат НаправлениеСортировки.Возр = SortDirection.Asc
    И НаправлениеСортировки.Убыв = SortDirection.Desc
    И НаправлениеСортировки.Возр <> НаправлениеСортировки.Убыв
    И Строка(НаправлениеСортировки.Возр) = "По возрастанию"
    И Строка(НаправлениеСортировки.Убыв) = "По убыванию";
"#,
        )
        .unwrap();
    assert_eq!(value, Value::Boolean(true));
}
