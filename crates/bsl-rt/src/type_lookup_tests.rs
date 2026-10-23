//! Поиск сохраняет прежнюю нормализацию и порядок таблиц типов.

use super::*;

fn linear_lookup(name: &str) -> Option<TypeId> {
    let key = squash(name);
    NAMES
        .iter()
        .find(|(_, ru, en)| squash(ru) == key || squash(en) == key)
        .map(|(id, _, _)| *id)
        .or_else(|| {
            IDENTIFIERS
                .iter()
                .find(|(_, name)| squash(name) == key)
                .map(|(id, _)| *id)
        })
}

#[test]
fn lookup_preserves_normalization_and_first_match_of_the_original_tables() {
    for name in NAMES
        .iter()
        .flat_map(|(_, ru, en)| [*ru, *en])
        .chain(IDENTIFIERS.iter().map(|(_, name)| *name))
    {
        let spaced = name.chars().map(|ch| format!("{ch} ")).collect::<String>();
        for spelling in [
            name.to_owned(),
            name.to_lowercase(),
            format!("\t {spaced}\n"),
        ] {
            assert_eq!(
                TypeId::lookup(&spelling),
                linear_lookup(&spelling),
                "{spelling}"
            );
        }
    }
}

#[test]
fn lookup_does_not_remove_internal_non_space_whitespace() {
    assert_eq!(TypeId::lookup("\t М а с с и в \n"), Some(TypeId::Array));
    for name in ["М\tассив", "М\nассив", "М\u{a0}ассив", "НетТакогоТипа", ""]
    {
        assert_eq!(TypeId::lookup(name), None, "{name}");
    }
}
