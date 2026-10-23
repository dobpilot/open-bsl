use super::*;

#[test]
fn selection_string_indices_reuse_case_insensitive_column_lookup() {
    let selection = QuerySelectionValue {
        data: Rc::new(
            QueryResultData::new(["Значение".into()], [vec![Value::number_from_i64(42)]]).unwrap(),
        ),
        cursor: Cell::new(1),
    };
    for name in ["Значение", "зНаЧеНиЕ"] {
        assert_eq!(
            selection.get_index(&Value::Str(name.into())).unwrap(),
            selection.get_index(&Value::number_from_i64(0)).unwrap()
        );
    }
    assert!(matches!(
        selection.get_index(&Value::Str("НетПоля".into())),
        Err(RtError::UnknownProperty(_))
    ));
}
