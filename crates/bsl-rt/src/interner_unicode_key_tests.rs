use super::*;

#[test]
fn unicode_uppercase_expansions_keep_the_same_numeric_name() {
    for (first, second) in [
        ("Straße", "STRASSE"),
        ("ı", "I"),
        ("ﬃ", "FFI"),
        ("ѐ", "Ѐ"),
        ("aStraße", "ASTRASSE"),
    ] {
        assert!(folded_eq(first, second));
        assert_eq!(folded_hash(first), folded_hash(second));
        for (first, second) in [(first, second), (second, first)] {
            let mut names = NameInterner::new();
            let id = names.intern(first);
            assert_eq!(names.lookup(second), Some(id));
            assert_eq!(names.intern(second), id);
            assert_eq!(names.name(id), Some(first));
        }
    }
}

#[test]
fn every_scalar_and_its_uppercase_have_equal_folded_hashes() {
    for scalar in (0..=0x10ffff).filter_map(char::from_u32) {
        let original = scalar.to_string();
        let uppercase = original.to_uppercase();
        assert_eq!(
            folded_hash(&original),
            folded_hash(&uppercase),
            "{scalar:?}"
        );
    }
}
