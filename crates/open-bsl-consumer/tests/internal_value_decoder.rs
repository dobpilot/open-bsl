use open_bsl::{RuntimeShapes, Value, value_from_string_internal};

#[test]
fn internal_scalar_decoder_is_available_through_facade_names_only() {
    let mut shapes = RuntimeShapes::seeded(vec![], vec![], None);
    assert_eq!(
        value_from_string_internal(r#"{"N",17}"#, &mut shapes).unwrap(),
        Value::number_from_i64(17)
    );
    assert_eq!(
        value_from_string_internal(r#"{"B",1}"#, &mut shapes).unwrap(),
        Value::Boolean(true)
    );
    assert_eq!(
        value_from_string_internal(r#"{"U"}"#, &mut shapes).unwrap(),
        Value::Undefined
    );
    assert_eq!(
        value_from_string_internal(r#"{"S","а""б"}"#, &mut shapes).unwrap(),
        Value::Str("а\"б".into())
    );
}

#[test]
fn internal_decoder_preserves_malformed_input_errors() {
    let mut shapes = RuntimeShapes::seeded(vec![], vec![], None);
    assert!(value_from_string_internal(r#"{"N",17} trailing"#, &mut shapes).is_err());
    assert!(value_from_string_internal(r#"{"B",2}"#, &mut shapes).is_err());
}
