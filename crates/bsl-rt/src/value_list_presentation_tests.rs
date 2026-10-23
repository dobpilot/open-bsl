use super::*;
use crate::RuntimeShapes;

#[test]
fn measured_mixed_strings_compare_before_case_and_yo_ties() {
    for (left, right) in [
        ("еA", "Еa"),
        ("еB", "Еa"),
        ("ЁA", "Еz"),
        ("ЕA", "еz"),
        ("е", "ё"),
        ("ё", "Ё"),
    ] {
        assert!(
            presentation_key(left) < presentation_key(right),
            "{left} / {right}"
        );
    }
}

#[test]
fn sorting_preserves_item_identity_and_equal_keys_in_both_directions() {
    let list = ValueListObject {
        items: RefCell::new(vec![
            new_item(&[BslValue::number_from_i64(1), BslValue::Str("same".into())]),
            new_item(&[BslValue::number_from_i64(2), BslValue::Str("same".into())]),
        ]),
    };
    let before = list.items.borrow().clone();
    let mut shapes = RuntimeShapes::seeded(Vec::new(), Vec::new(), None);
    let mut context = CallContext::minimal(&mut shapes, |value, _| Ok(value.to_string()));
    for direction in [
        EnumValue::SortDirectionAscending,
        EnumValue::SortDirectionDescending,
    ] {
        sort_by_presentation(&list, &[BslValue::Enum(direction)], &mut context).unwrap();
        for (actual, expected) in list.items.borrow().iter().zip(&before) {
            assert!(std::ptr::eq(
                actual.object_ref().unwrap(),
                expected.object_ref().unwrap()
            ));
        }
    }
    assert!(sort_by_presentation(&list, &[BslValue::number_from_i64(1)], &mut context).is_err());
    assert_eq!(
        list.items.borrow()[0]
            .object_ref()
            .unwrap()
            .downcast_ref::<ValueListItemObject>()
            .unwrap()
            .value,
        BslValue::number_from_i64(1)
    );
}
