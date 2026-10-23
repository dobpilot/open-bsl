use super::*;

const RU: &str = "855375b9-327b-53fa-b11c-6454c2c7a1a6";
const EN: &str = "220d033d-4a5f-45ed-926b-af14b6cf30bc";

fn catalog() -> MetadataCatalog {
    MetadataCatalog::new(
        "configuration",
        vec![
            bsl_orm::MetadataObject::new(RU, "Русский", MetadataKind::Language),
            bsl_orm::MetadataObject::new(EN, "English", MetadataKind::Language),
            bsl_orm::MetadataObject::new("other", "Другое", MetadataKind::Catalog),
        ],
    )
    .unwrap()
    .with_configuration(
        "Проверка",
        "1.0",
        bsl_orm::ScriptVariant::Russian,
        bsl_orm::CompatibilityMode::Version8_3_24,
        Vec::new(),
    )
    .unwrap()
    .with_primary_language(RU)
    .unwrap()
}

fn principal() -> Principal {
    Principal::new("user", "Пользователь", Vec::new())
}

#[test]
fn selected_language_and_known_nil_are_resolved_separately() {
    let catalog = catalog();
    for (selected, expected) in [
        (RU, RU),
        (EN, EN),
        ("00000000-0000-0000-0000-000000000000", RU),
    ] {
        let selected = selected.parse().unwrap();
        let resolved = resolve(principal(), Some(&selected), &catalog).unwrap();
        assert_eq!(resolved.interface_language_id(), Some(expected));
    }
    let empty = Principal::new("", "", Vec::new());
    assert_eq!(
        resolve(empty, None, &catalog)
            .unwrap()
            .interface_language_id(),
        Some(RU)
    );
}

#[test]
fn unknown_selected_and_missing_metadata_fail_before_startup() {
    let catalog = catalog();
    assert!(resolve(principal(), None, &catalog).is_err());
    let missing = "11111111-1111-1111-1111-111111111111".parse().unwrap();
    assert!(resolve(principal(), Some(&missing), &catalog).is_err());
    let nil = "00000000-0000-0000-0000-000000000000".parse().unwrap();
    let no_primary = MetadataCatalog::new("configuration", Vec::new()).unwrap();
    assert!(resolve(principal(), Some(&nil), &no_primary).is_err());
}

#[test]
fn native_user_records_reach_the_cli_language_resolver() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(
        "../../docs/research/fixtures/approved-sdk-production-integration-1016/current-language-session-audit/native-user-records-1017",
    );
    let catalog = catalog();
    for (name, expected) in [
        ("LangRussian", RU),
        ("LangEnglish", EN),
        ("LangDefault", RU),
        ("LangAdmin", EN),
    ] {
        let bytes = std::fs::read(root.join(format!("{name}.data"))).unwrap();
        let user = open_sdbl::metadata::decode_user_data(&bytes).unwrap();
        let principal = Principal::new(user.id.as_str(), &user.name, Vec::new());
        let resolved = resolve(principal, user.interface_language.as_ref(), &catalog).unwrap();
        assert_eq!(resolved.interface_language_id(), Some(expected), "{name}");
        assert_eq!(
            std::fs::read(root.join(format!("{name}.data"))).unwrap(),
            bytes
        );
    }
}
