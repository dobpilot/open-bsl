use super::*;

#[test]
fn flat_arrays_capture_current_scalars_and_reference_owners() {
    let reference = bsl_orm::catalog_reference_value(
        std::sync::Arc::new(
            bsl_orm::MetadataCatalog::new(
                "array-test",
                vec![MetadataObject::new(
                    "11111111-2222-3333-4444-555555555555",
                    "Пример",
                    MetadataKind::Catalog,
                )],
            )
            .unwrap(),
        ),
        "11111111-2222-3333-4444-555555555555",
        [3; 16],
    )
    .unwrap();
    let scalar = Value::Boolean(true);
    let value = Value::new_array(vec![scalar.clone(), reference.clone()]);
    let captured = parameter_value(&value).unwrap();
    assert_eq!(
        captured,
        ParameterValue::List(vec![
            parameter_value(&scalar).unwrap(),
            parameter_value(&reference).unwrap(),
        ])
    );
    let Value::Object(object) = &value else {
        panic!("массив")
    };
    let BslObject::Array(items) = object.as_ref() else {
        panic!("массив")
    };
    items.borrow_mut().push(Value::Null);
    assert_eq!(
        parameter_value(&value).unwrap(),
        ParameterValue::List(vec![
            parameter_value(&scalar).unwrap(),
            parameter_value(&reference).unwrap(),
            ParameterValue::Null,
        ])
    );
    let ParameterValue::List(previous) = captured else {
        panic!("список")
    };
    assert_eq!(previous.len(), 2);
    assert_eq!(items.borrow().len(), 3);
    assert_eq!(
        parameter_value(&Value::new_array(vec![])).unwrap(),
        ParameterValue::List(vec![])
    );
}

#[test]
fn nested_and_cyclic_arrays_fail_without_recursive_traversal() {
    let cyclic = Value::new_array(vec![]);
    let Value::Object(object) = &cyclic else {
        panic!("массив")
    };
    let BslObject::Array(items) = object.as_ref() else {
        panic!("массив")
    };
    items.borrow_mut().push(cyclic.clone());
    assert!(parameter_value(&cyclic).is_err());
    items.borrow_mut().clear();
    assert!(parameter_value(&Value::new_array(vec![Value::new_array(vec![])])).is_err());
}
