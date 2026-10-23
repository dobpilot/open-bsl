use super::*;

#[test]
fn typed_host_schema_preserves_duplicate_names_and_normal_row_state() {
    let types = vec![crate::ColumnType {
        id: crate::TypeRef::Native(crate::TypeId::String),
        quals: vec!["12".into(), "0".into()],
    }];
    let table = BslValue::new_table_with_typed_columns(vec![
        ("Категория".into(), types.clone()),
        ("Категория".into(), types.clone()),
    ]);
    let BslValue::Object(object) = &table else {
        panic!()
    };
    let BslObject::ValueTable(data) = &**object else {
        panic!()
    };
    {
        let data = data.borrow();
        assert_eq!(data.column_names, ["Категория", "Категория"]);
        assert_eq!(data.column_types, [Some(types.clone()), Some(types)]);
        assert_eq!(data.row_count(), 0);
    }
    table.table_add_row().unwrap();
    assert_eq!(data.borrow().row_count(), 1);
    table
        .get_field_by_name("Колонки")
        .unwrap()
        .table_add_column(&BslValue::Str("Категория".into()), &BslValue::Undefined)
        .unwrap();
    assert_eq!(data.borrow().column_names.len(), 2);
}
