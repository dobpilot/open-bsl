use open_bsl::{BslString, ColumnType, TypeId, TypeRef, Value, adjust_to_types};

#[test]
fn component_adjustment_reuses_number_conversion_and_qualifiers_through_facade() {
    let types = [ColumnType {
        id: TypeRef::Native(TypeId::Number),
        quals: vec!["9".into(), "0".into()],
    }];
    for (input, expected) in [
        (Value::Str(BslString::from_str("7")), 7),
        (Value::Boolean(true), 1),
        (Value::Undefined, 0),
        (Value::Str(BslString::from_str("invalid")), 0),
        (Value::number_from_i64(12_345_678_901), 999_999_999),
    ] {
        assert_eq!(
            adjust_to_types(input, &types),
            Value::number_from_i64(expected)
        );
    }
    // Ограничение не меняет подписанные числовые колонки ради кодов справочника.
    assert_eq!(
        adjust_to_types(Value::number_from_i64(-2), &types),
        Value::number_from_i64(-2)
    );
}

#[test]
fn component_string_qualifier_uses_existing_utf16_length_and_empty_defaults() {
    let types = [ColumnType {
        id: TypeRef::Native(TypeId::String),
        quals: vec!["3".into()],
    }];
    assert_eq!(
        adjust_to_types(Value::Str(BslString::from_str("абвгд")), &types),
        Value::Str(BslString::from_str("абв"))
    );
    assert_eq!(
        adjust_to_types(Value::Null, &types),
        Value::Str(BslString::from_str(""))
    );
    let plain = [ColumnType::plain(TypeRef::Native(TypeId::String))];
    assert_eq!(
        adjust_to_types(Value::number_from_i64(12), &plain),
        Value::Str(BslString::from_str("12"))
    );
    assert_eq!(
        adjust_to_types(Value::number_from_i64(12), &[]),
        Value::number_from_i64(12)
    );
}
