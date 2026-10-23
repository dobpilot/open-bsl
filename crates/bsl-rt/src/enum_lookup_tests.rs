//! Индексы сохраняют порядок написаний.

use super::*;

fn linear_enum(name: &str) -> Option<EnumKind> {
    let upper = name.to_uppercase();
    enum_names()
        .find(|(name, _)| name.to_uppercase() == upper)
        .map(|(_, kind)| kind)
}

fn linear_member(kind: EnumKind, member: &str) -> Option<EnumValue> {
    let upper = member.to_uppercase();
    MEMBERS
        .iter()
        .find(|(candidate, _, ru, en, _)| {
            *candidate == kind && (ru.to_uppercase() == upper || en.to_uppercase() == upper)
        })
        .map(|(_, value, ..)| *value)
}

#[test]
fn enum_lookup_matches_every_original_name_and_case_variant() {
    for (name, _) in enum_names() {
        for spelling in [name.to_string(), name.to_lowercase(), name.to_uppercase()] {
            assert_eq!(lookup_enum(&spelling), linear_enum(&spelling), "{spelling}");
        }
    }
    assert_eq!(lookup_enum(" НесуществующееПеречисление "), None);
}

#[test]
fn member_lookup_matches_every_original_kind_and_spelling() {
    for (kind, _, ru, en, _) in MEMBERS {
        for name in [*ru, *en] {
            for spelling in [name.to_owned(), name.to_lowercase(), name.to_uppercase()] {
                assert_eq!(
                    lookup_member(*kind, &spelling),
                    linear_member(*kind, &spelling),
                    "{kind:?}.{spelling}",
                );
            }
        }
    }
}

#[test]
fn sort_direction_has_no_ascending_alias() {
    // Измерено: `measure-sort-direction-alias.platform.txt`.
    for name in ["Ascending", "aScEnDiNg", "Descending"] {
        assert_eq!(lookup_member(EnumKind::SortDirection, name), None, "{name}");
    }
    assert_eq!(
        lookup_member(EnumKind::SortDirection, "Asc"),
        Some(EnumValue::SortDirectionAscending),
    );
}
