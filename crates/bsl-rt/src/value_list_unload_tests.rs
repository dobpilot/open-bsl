use super::*;
use crate::{NameInterner, RuntimeShapes};

#[test]
fn unloaded_array_slots_are_independent_and_contain_values() {
    let list = ValueListObject {
        items: RefCell::new(vec![
            new_item(&[
                BslValue::number_from_i64(17),
                BslValue::Str("представление".into()),
            ]),
            new_item(&[BslValue::Undefined]),
        ]),
    };
    let mut shapes = RuntimeShapes::seeded(Vec::new(), Vec::new(), None);
    let mut context = CallContext::minimal(&mut shapes, |value, _| Ok(value.to_string()));
    let result = unload_values(&list, &[], &mut context).unwrap();
    let names = NameInterner::new();
    let zero = BslValue::number_from_i64(0);
    assert_eq!(
        result.get_index(&zero, &names).unwrap(),
        BslValue::number_from_i64(17)
    );
    assert_eq!(
        result
            .get_index(&BslValue::number_from_i64(1), &names)
            .unwrap(),
        BslValue::Undefined
    );
    result.set_index(&zero, BslValue::Boolean(false)).unwrap();
    assert_eq!(
        list.items.borrow()[0]
            .object_ref()
            .unwrap()
            .downcast_ref::<ValueListItemObject>()
            .unwrap()
            .value,
        BslValue::number_from_i64(17)
    );
}

#[test]
fn an_empty_list_returns_a_new_empty_array_on_each_call() {
    let list = ValueListObject {
        items: RefCell::new(Vec::new()),
    };
    let mut shapes = RuntimeShapes::seeded(Vec::new(), Vec::new(), None);
    let mut context = CallContext::minimal(&mut shapes, |value, _| Ok(value.to_string()));
    let first = unload_values(&list, &[], &mut context).unwrap();
    let second = unload_values(&list, &[], &mut context).unwrap();
    if let (BslValue::Object(first), BslValue::Object(second)) = (&first, &second) {
        assert!(!std::rc::Rc::ptr_eq(first, second));
    } else {
        panic!("Выгрузка должна возвращать массив");
    }
    assert_eq!(first.collection_len().unwrap(), 0);
}
