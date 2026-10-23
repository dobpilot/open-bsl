use super::*;

#[test]
fn permitted_rows_keep_their_raw_cells_unchanged() {
    let rows = vec![vec![Cell::Number("13".into()), Cell::Bool(false)]];
    let before = rows.clone();
    validate_query_rows(&rows, 2, Some(1)).unwrap();
    assert_eq!(rows, before);
    validate_query_rows(&[], 2, Some(1)).unwrap();
}

#[test]
fn forbidden_sentinel_is_rejected_before_its_null_payload_is_decoded() {
    let rows = vec![vec![Cell::Null, Cell::Bool(true)]];
    assert!(
        validate_query_rows(&rows, 2, Some(1))
            .unwrap_err()
            .to_string()
            .contains("недостаточно прав")
    );
}

#[test]
fn a_forbidden_row_after_a_permitted_row_rejects_the_whole_result() {
    let rows = vec![
        vec![Cell::Number("13".into()), Cell::Bool(false)],
        vec![Cell::Null, Cell::Bool(true)],
    ];
    assert!(validate_query_rows(&rows, 2, Some(1)).is_err());
}

#[test]
fn missing_or_non_boolean_access_markers_are_not_permissions() {
    for marker in [
        Cell::Null,
        Cell::Number("0".into()),
        Cell::Text("false".into()),
    ] {
        assert!(validate_query_rows(&[vec![Cell::Null, marker]], 2, Some(1)).is_err());
    }
    assert!(validate_query_rows(&[vec![Cell::Null]], 2, Some(1)).is_err());
    assert!(validate_query_rows(&[], 2, Some(2)).is_err());
}

#[test]
fn an_unmarked_query_keeps_ordinary_width_validation() {
    validate_query_rows(&[vec![Cell::Number("13".into())]], 1, None).unwrap();
    assert!(validate_query_rows(&[vec![]], 1, None).is_err());
}
