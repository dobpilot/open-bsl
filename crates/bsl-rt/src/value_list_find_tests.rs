use super::*;
use crate::RuntimeShapes;

#[test]
fn first_matching_item_retains_identity_and_list_order() {
    let first = new_item(&[BslValue::number_from_i64(1)]);
    let second = new_item(&[BslValue::number_from_i64(1)]);
    let list = ValueListObject {
        items: RefCell::new(vec![first.clone(), second.clone()]),
    };
    let mut shapes = RuntimeShapes::seeded(vec![], vec![], None);
    let mut context = CallContext::minimal(&mut shapes, |value, _| Ok(value.to_string()));
    assert_eq!(
        find_by_value(&list, &[BslValue::number_from_i64(1)], &mut context).unwrap(),
        first
    );
    assert_eq!(
        find_by_value(&list, &[BslValue::Boolean(true)], &mut context).unwrap(),
        BslValue::Undefined
    );
    assert_eq!(list.items.borrow().as_slice(), &[first, second]);
}

#[test]
fn strings_are_case_sensitive_and_undefined_can_be_found() {
    let upper = new_item(&[BslValue::Str("А".into())]);
    let lower = new_item(&[BslValue::Str("а".into())]);
    let empty = new_item(&[BslValue::Undefined]);
    let list = ValueListObject {
        items: RefCell::new(vec![upper.clone(), lower.clone(), empty.clone()]),
    };
    let mut shapes = RuntimeShapes::seeded(vec![], vec![], None);
    let mut context = CallContext::minimal(&mut shapes, |value, _| Ok(value.to_string()));
    assert_eq!(
        find_by_value(&list, &[BslValue::Str("А".into())], &mut context).unwrap(),
        upper
    );
    assert_eq!(
        find_by_value(&list, &[BslValue::Str("а".into())], &mut context).unwrap(),
        lower
    );
    assert_eq!(
        find_by_value(&list, &[BslValue::Undefined], &mut context).unwrap(),
        empty
    );
}
