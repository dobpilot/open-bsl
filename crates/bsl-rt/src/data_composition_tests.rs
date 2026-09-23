use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use super::DataCompositionField;
use crate::{BslString, BslValue};

fn field(path: &str) -> BslValue {
    BslValue::new_object(DataCompositionField {
        path: BslString::from(path),
    })
}

fn hash(value: &BslValue) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

#[test]
fn field_preserves_spelling_but_compares_and_hashes_without_case() {
    let upper = field("Документ.Ссылка");
    let lower = field("документ.ссылка");

    assert_eq!(upper, lower);
    assert_eq!(hash(&upper), hash(&lower));
    assert_eq!(upper.object_ref().unwrap().display(), "Документ.Ссылка");
}

#[test]
fn empty_field_is_not_filled() {
    let value = field("");
    assert!(!value.object_ref().unwrap().is_filled().unwrap());
}
