use bsl_rt::{BslNumber, BslValue, BuiltinMethod, NameInterner, call_builtin_method};

fn number(value: i64) -> BslValue {
    BslValue::number_from_i64(value)
}

fn table() -> (BslValue, BslValue, BslValue) {
    let table = BslValue::new_table();
    let columns = table.get_field_by_name("Колонки").unwrap();
    for name in ["Левая", "Правая"] {
        columns
            .table_add_column(&BslValue::Str(name.into()), &BslValue::Undefined)
            .unwrap();
    }
    let first = table.table_add_row().unwrap();
    first.set_field_by_name("Левая", number(10)).unwrap();
    first.set_field_by_name("Правая", number(20)).unwrap();
    let second = table.table_add_row().unwrap();
    second.set_field_by_name("Левая", number(30)).unwrap();
    second.set_field_by_name("Правая", number(40)).unwrap();
    (table, columns, first)
}

#[test]
fn measured_method_indices_select_live_rows_columns_and_cells() {
    for index in [
        number(1),
        BslValue::Number(BslNumber::parse_canonical("1.9").unwrap()),
        BslValue::Str(" 1 ".into()),
        BslValue::Str("1.9".into()),
        BslValue::Boolean(true),
    ] {
        let (table, columns, first) = table();
        let row =
            call_builtin_method(BuiltinMethod::Get, &table, std::slice::from_ref(&index)).unwrap();
        assert_eq!(row.get_field_by_name("Левая").unwrap(), number(30));
        let column =
            call_builtin_method(BuiltinMethod::Get, &columns, std::slice::from_ref(&index))
                .unwrap();
        assert_eq!(
            column.get_field_by_name("Имя").unwrap(),
            BslValue::Str("Правая".into())
        );
        assert_eq!(
            call_builtin_method(BuiltinMethod::Get, &first, std::slice::from_ref(&index)).unwrap(),
            number(20)
        );
        call_builtin_method(BuiltinMethod::BufSet, &row, &[index, number(99)]).unwrap();
        let again = table.get_index(&number(1), &NameInterner::new()).unwrap();
        assert_eq!(again.get_field_by_name("Правая").unwrap(), number(99));
        assert_eq!(first.get_field_by_name("Правая").unwrap(), number(20));
    }
}

#[test]
fn negative_fraction_bad_arity_and_named_column_do_not_mutate_the_table() {
    let (table, columns, first) = table();
    for index in [
        BslValue::Number(BslNumber::parse_canonical("-0.9").unwrap()),
        number(2),
        number(i64::MAX),
        BslValue::Str("Левая".into()),
        BslValue::Undefined,
        BslValue::Null,
    ] {
        for object in [&table, &columns, &first] {
            assert!(
                call_builtin_method(BuiltinMethod::Get, object, std::slice::from_ref(&index))
                    .is_err()
            );
        }
        assert!(call_builtin_method(BuiltinMethod::BufSet, &first, &[index, number(99)]).is_err());
    }
    for object in [&table, &columns, &first] {
        for args in [vec![], vec![number(0), number(1)]] {
            assert!(call_builtin_method(BuiltinMethod::Get, object, &args).is_err());
        }
    }
    for args in [
        vec![],
        vec![number(0)],
        vec![number(0), number(99), number(1)],
    ] {
        assert!(call_builtin_method(BuiltinMethod::BufSet, &first, &args).is_err());
    }
    for object in [&table, &columns] {
        assert!(
            call_builtin_method(BuiltinMethod::BufSet, object, &[number(0), number(99)]).is_err()
        );
    }
    assert_eq!(first.get_field_by_name("Левая").unwrap(), number(10));
    assert_eq!(first.get_field_by_name("Правая").unwrap(), number(20));
    // Строковый индексатор по имени сохраняется, хотя метод требует число.
    assert_eq!(
        first
            .get_index(&BslValue::Str("Левая".into()), &NameInterner::new())
            .unwrap(),
        number(10)
    );
    assert_eq!(table.collection_len().unwrap(), 2);
    assert_eq!(columns.collection_len().unwrap(), 2);
}

#[test]
fn deleted_row_remains_invalid_through_method_access() {
    let (table, _, first) = table();
    table.delete_element(&number(0)).unwrap();
    assert!(call_builtin_method(BuiltinMethod::Get, &first, &[number(0)]).is_err());
    assert!(call_builtin_method(BuiltinMethod::BufSet, &first, &[number(0), number(99)]).is_err());
    let remaining = call_builtin_method(BuiltinMethod::Get, &table, &[number(0)]).unwrap();
    assert_eq!(remaining.get_field_by_name("Левая").unwrap(), number(30));
}
