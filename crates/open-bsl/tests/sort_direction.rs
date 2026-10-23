use open_bsl::{Engine, Value};

#[test]
fn sort_direction_has_both_measured_members_and_keeps_the_old_alias() {
    let engine = Engine::builder().build().unwrap();
    let value = engine
        .new_state()
        .exec(
            r#"
Возврат НаправлениеСортировки.Возр = SortDirection.Asc
    И НаправлениеСортировки.Убыв = SortDirection.Desc
    И НаправлениеСортировки.Возр <> НаправлениеСортировки.Убыв
    И SortDirection.Ascending = SortDirection.Asc
    И Строка(НаправлениеСортировки.Возр) = "Ascending"
    И Строка(НаправлениеСортировки.Убыв) = "Descending";
"#,
        )
        .unwrap();
    assert_eq!(value, Value::Boolean(true));
}
