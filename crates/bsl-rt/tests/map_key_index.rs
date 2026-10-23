use bsl_rt::{BslObject, BslValue, NameInterner};
use std::rc::Rc;

#[test]
fn map_key_indices_and_iteration_are_independent() {
    let names = NameInterner::default();
    let map = BslValue::new_map();
    let type_key = BslValue::number_from_i64(1).type_of().unwrap();
    let uuid_key = BslValue::Object(Rc::new(BslObject::Uuid([1; 16])));
    let keys = [type_key.clone(), BslValue::number_from_i64(0), uuid_key];
    for (key, value) in keys.iter().zip([11, 22, 33]) {
        map.set_index(key, BslValue::number_from_i64(value))
            .unwrap();
        assert_eq!(
            map.get_index(key, &names).unwrap(),
            BslValue::number_from_i64(value)
        );
    }
    let missing = BslValue::Str("".into()).type_of().unwrap();
    assert_eq!(
        map.get_index(&missing, &names).unwrap(),
        BslValue::Undefined
    );
    map.set_index(&type_key, BslValue::number_from_i64(12))
        .unwrap();
    for (position, (key, value)) in keys.iter().zip([12, 22, 33]).enumerate() {
        let pair = map
            .collection_item(&BslValue::number_from_i64(position as i64), &names)
            .unwrap();
        assert_eq!(pair.get_field_by_name("Ключ").unwrap(), *key);
        assert_eq!(
            pair.get_field_by_name("Значение").unwrap(),
            BslValue::number_from_i64(value)
        );
    }
}
