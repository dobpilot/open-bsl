//! Индекс ключевых слов сохраняет поиск по исходной двуязычной таблице.

use super::*;

fn linear_lookup(ident: &str) -> Option<Keyword> {
    let upper = ident.to_uppercase();
    SPELLINGS
        .iter()
        .find(|(ru, en, _)| ru.to_uppercase() == upper || en.to_uppercase() == upper)
        .map(|(_, _, keyword)| *keyword)
}

#[test]
fn lookup_matches_the_original_order_and_unicode_case_conversion() {
    for name in SPELLINGS.iter().flat_map(|(ru, en, _)| [*ru, *en]) {
        for spelling in [name.to_owned(), name.to_lowercase(), name.to_uppercase()] {
            assert_eq!(lookup(&spelling), linear_lookup(&spelling), "{spelling}");
        }
    }
    assert_eq!(lookup("ıf"), Some(Keyword::If));
    for ident in [" If", "If ", "ЕслиЕщё", "", "Сумма", "Если\0"] {
        assert_eq!(lookup(ident), None, "{ident}");
    }
}
