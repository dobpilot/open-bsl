use bsl_rt::{BslNumber, BslValue, BuiltinMethod, NameInterner, call_builtin_method};

fn number(value: i64) -> BslValue {
    BslValue::number_from_i64(value)
}

#[test]
fn insertion_extends_with_undefined_and_keeps_aliases_and_values() {
    let array = BslValue::new_array(vec![number(10), number(20)]);
    let alias = array.clone();
    let value = BslValue::new_array(vec![number(99)]);
    call_builtin_method(BuiltinMethod::Insert, &array, &[number(3), value.clone()]).unwrap();
    assert_eq!(
        call_builtin_method(BuiltinMethod::Count, &alias, &[]).unwrap(),
        number(4)
    );
    assert_eq!(
        call_builtin_method(BuiltinMethod::Get, &alias, &[number(2)]).unwrap(),
        BslValue::Undefined
    );
    assert_eq!(
        call_builtin_method(BuiltinMethod::Get, &alias, &[number(3)]).unwrap(),
        value
    );
    call_builtin_method(BuiltinMethod::Insert, &array, &[number(1)]).unwrap();
    assert_eq!(
        call_builtin_method(BuiltinMethod::Get, &alias, &[number(1)]).unwrap(),
        BslValue::Undefined
    );
    assert_eq!(
        call_builtin_method(BuiltinMethod::Get, &alias, &[number(2)]).unwrap(),
        number(20)
    );
    assert_eq!(
        call_builtin_method(BuiltinMethod::Get, &alias, &[number(4)]).unwrap(),
        value
    );
}

#[test]
fn failed_insert_preserves_the_array_including_impossible_allocation() {
    let array = BslValue::new_array(vec![number(10), number(20)]);
    for args in [
        vec![],
        vec![number(1), number(99), number(100)],
        vec![number(-1), number(99)],
        vec![BslValue::Null, number(99)],
        vec![BslValue::Undefined, number(99)],
        vec![number(i64::MAX), number(99)],
    ] {
        assert!(call_builtin_method(BuiltinMethod::Insert, &array, &args).is_err());
        assert_eq!(
            call_builtin_method(BuiltinMethod::Count, &array, &[]).unwrap(),
            number(2)
        );
        assert_eq!(
            call_builtin_method(BuiltinMethod::Get, &array, &[number(0)]).unwrap(),
            number(10)
        );
        assert_eq!(
            call_builtin_method(BuiltinMethod::Get, &array, &[number(1)]).unwrap(),
            number(20)
        );
    }
}

#[test]
fn method_indices_are_prepared_without_changing_bracket_indices() {
    let array = BslValue::new_array(vec![number(10), number(20)]);
    for index in [
        number(1),
        BslValue::Number(BslNumber::parse_canonical("1.9").unwrap()),
        BslValue::Str("1".into()),
        BslValue::Boolean(true),
    ] {
        assert_eq!(
            call_builtin_method(BuiltinMethod::Get, &array, std::slice::from_ref(&index)).unwrap(),
            number(20)
        );
        if index != number(1) {
            assert!(array.get_index(&index, &NameInterner::new()).is_err());
        }
    }
}

#[test]
fn setter_checks_index_and_arity_before_mutation() {
    let array = BslValue::new_array(vec![number(10), number(20)]);
    for args in [
        vec![],
        vec![number(0)],
        vec![number(2), number(99)],
        vec![number(0), number(99), number(1)],
    ] {
        assert!(call_builtin_method(BuiltinMethod::BufSet, &array, &args).is_err());
        assert_eq!(
            array.get_index(&number(0), &NameInterner::new()).unwrap(),
            number(10)
        );
        assert_eq!(
            array.get_index(&number(1), &NameInterner::new()).unwrap(),
            number(20)
        );
    }
    assert_eq!(
        call_builtin_method(
            BuiltinMethod::BufSet,
            &array,
            &[BslValue::Boolean(true), number(99)]
        )
        .unwrap(),
        BslValue::Undefined
    );
    assert_eq!(
        call_builtin_method(BuiltinMethod::Get, &array, &[number(1)]).unwrap(),
        number(99)
    );
}
