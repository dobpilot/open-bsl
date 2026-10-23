use open_bsl::{
    BslObject, RtError, RuntimeShapes, Value, value_from_string_internal,
    value_from_string_internal_with_resolver,
};

#[test]
fn a_facade_only_resolver_materializes_a_nested_scalar_object() {
    let mut shapes = RuntimeShapes::seeded(vec![], vec![], None);
    let mut calls = 0;
    let mut resolve = |kind: &str, payload: &str| {
        assert_eq!(kind, "fc01b5df-97fe-449b-83d4-218a090e681e");
        assert_eq!(payload, "12345678-1234-4321-abcd-0123456789ab");
        calls += 1;
        Ok(Some(Value::number_from_i64(17)))
    };
    let value = value_from_string_internal_with_resolver(
        r##"{"#",51e7a0d2-530b-11d4-b98a-008048da3034,{1,{"#",fc01b5df-97fe-449b-83d4-218a090e681e,12345678-1234-4321-abcd-0123456789ab}}}"##,
        &mut shapes,
        &mut resolve,
    ).unwrap();
    let Value::Object(object) = value else {
        panic!("ожидался массив")
    };
    let BslObject::Array(items) = object.as_ref() else {
        panic!("ожидался массив")
    };
    assert_eq!(&*items.borrow(), &[Value::number_from_i64(17)]);
    assert_eq!(calls, 1);
}

#[test]
fn default_and_unresolved_scalar_objects_retain_opaque_transit() {
    let text =
        r##"{"#",fc01b5df-97fe-449b-83d4-218a090e681e,12345678-1234-4321-abcd-0123456789ab}"##;
    let mut shapes = RuntimeShapes::seeded(vec![], vec![], None);
    let plain = value_from_string_internal(text, &mut shapes).unwrap();
    let resolved =
        value_from_string_internal_with_resolver(text, &mut shapes, &mut |_, _| Ok(None)).unwrap();
    assert!(
        matches!(plain, Value::Object(ref object) if matches!(object.as_ref(), BslObject::VstrOpaque(_)))
    );
    assert!(
        matches!(resolved, Value::Object(ref object) if matches!(object.as_ref(), BslObject::VstrOpaque(_)))
    );
}

#[test]
fn resolver_errors_propagate_without_a_successful_replacement() {
    let mut shapes = RuntimeShapes::seeded(vec![], vec![], None);
    let result = value_from_string_internal_with_resolver(
        r##"{"#",fc01b5df-97fe-449b-83d4-218a090e681e,bad}"##,
        &mut shapes,
        &mut |_, _| Err(RtError::DynamicError("отказ резолвера".into())),
    );
    assert!(matches!(result, Err(RtError::DynamicError(text)) if text == "отказ резолвера"));
}

#[test]
fn repeated_scalar_representations_share_the_existing_decode_cache() {
    let mut shapes = RuntimeShapes::seeded(vec![], vec![], None);
    let mut calls = 0;
    let mut resolve = |_: &str, payload: &str| {
        calls += 1;
        Ok(Some(Value::Object(std::rc::Rc::new(BslObject::Uuid(
            open_bsl::uuid::parse(payload)?,
        )))))
    };
    let value = value_from_string_internal_with_resolver(
        r##"{"#",51e7a0d2-530b-11d4-b98a-008048da3034,{2,{"#",fc01b5df-97fe-449b-83d4-218a090e681e,12345678-1234-4321-abcd-0123456789ab},{"#",fc01b5df-97fe-449b-83d4-218a090e681e,12345678-1234-4321-abcd-0123456789ab}}}"##,
        &mut shapes, &mut resolve,
    ).unwrap();
    let Value::Object(object) = value else {
        panic!("ожидался массив")
    };
    let BslObject::Array(items) = object.as_ref() else {
        panic!("ожидался массив")
    };
    let items = items.borrow();
    let [Value::Object(first), Value::Object(second)] = items.as_slice() else {
        panic!("ожидались два UUID")
    };
    assert!(std::rc::Rc::ptr_eq(first, second));
    assert_eq!(calls, 1);
}

#[test]
fn a_truncated_object_fails_before_host_resolution() {
    let mut shapes = RuntimeShapes::seeded(vec![], vec![], None);
    let mut calls = 0;
    let result = value_from_string_internal_with_resolver(
        r##"{"#",fc01b5df-97fe-449b-83d4-218a090e681e,12345678-1234-4321-abcd-0123456789ab"##,
        &mut shapes,
        &mut |_, _| {
            calls += 1;
            Ok(Some(Value::Undefined))
        },
    );
    assert!(result.is_err());
    assert_eq!(calls, 0);
}
