use super::*;
use crate::RuntimeShapes;

fn construct(arguments: &[BslValue]) -> RtResult<BslValue> {
    let mut shapes = RuntimeShapes::seeded(Vec::new(), Vec::new(), None);
    let mut context = CallContext::minimal(&mut shapes, |value, _| Ok(value.to_string()));
    construct_formatted_string(&mut context, arguments)
}

#[test]
fn concatenates_strings_nested_values_and_arrays() {
    let nested = construct(&[BslValue::Str("два".into())]).unwrap();
    let array = BslValue::new_array(vec![BslValue::Str("три".into()), nested.clone()]);
    let value = construct(&[BslValue::Str("один".into()), nested, array]).unwrap();

    assert_eq!(value.to_string(), "одиндватридва");
    assert!(value.is_filled().unwrap());
}

#[test]
fn formatting_positions_affect_equality_but_not_plain_text() {
    let first = construct(&[
        BslValue::Str("текст".into()),
        BslValue::Undefined,
        BslValue::Undefined,
        BslValue::Undefined,
        BslValue::Str("first".into()),
    ])
    .unwrap();
    let second = construct(&[
        BslValue::Str("текст".into()),
        BslValue::Undefined,
        BslValue::Undefined,
        BslValue::Undefined,
        BslValue::Str("second".into()),
    ])
    .unwrap();

    assert_eq!(first.to_string(), "текст");
    assert!(!first.eq_value(&second));
}

#[test]
fn rejects_empty_and_unsupported_content() {
    assert!(construct(&[]).is_err());
    assert!(construct(&[BslValue::number_from_i64(1)]).is_err());
}

#[test]
fn array_content_equality_uses_the_measured_immutable_snapshot() {
    let array = BslValue::new_array(vec![BslValue::Str("до".into())]);
    let first = construct(std::slice::from_ref(&array)).unwrap();
    let BslValue::Object(object) = &array else {
        panic!("массив")
    };
    let BslObject::Array(items) = object.as_ref() else {
        panic!("массив")
    };
    items.borrow_mut()[0] = BslValue::Str("после".into());
    let second = construct(&[array]).unwrap();
    let third = construct(&[BslValue::new_array(vec![BslValue::Str("до".into())])]).unwrap();
    assert_eq!(first.to_string(), "до");
    assert_eq!(second.to_string(), "после");
    assert!(!first.eq_value(&second));
    assert!(first.eq_value(&third));
}
