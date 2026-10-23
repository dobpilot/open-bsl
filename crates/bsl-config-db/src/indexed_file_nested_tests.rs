use super::*;

#[test]
fn native_adopted_form_by_name_is_confined_to_verified_owner_and_kind() {
    let header = parse_extension_objects(include_bytes!("../../../docs/research/fixtures/file-configuration-native-1022/native-adopted-form/extension-header.txt")).unwrap().remove(0);
    assert_eq!(header.belonging, Some(ObjectBelonging::Adopted));
    assert!(header.extends.is_none());
    let description = ConfigurationDescription {
        id: header.id.as_str().into(),
        owner: Some("extension-owner".into()),
        kind: "Form".into(),
        name: header.name.as_str().into(),
    };
    let original = ConfigurationDescription {
        id: "base-form".into(),
        owner: Some("verified-owner".into()),
        kind: "Form".into(),
        name: "ФОРМАЭЛЕМЕНТА".into(),
    };
    let mut unrelated = original.clone();
    unrelated.owner = Some("another-owner".into());
    assert!(
        nested_target(
            &[unrelated.clone()],
            &description,
            "verified-owner",
            &header
        )
        .is_err()
    );
    let mut wrong_kind = original.clone();
    wrong_kind.kind = "Template".into();
    assert!(nested_target(&[wrong_kind], &description, "verified-owner", &header).is_err());
    assert_eq!(
        nested_target(
            &[unrelated, original.clone()],
            &description,
            "verified-owner",
            &header
        )
        .unwrap(),
        "base-form"
    );
    assert!(
        nested_target(
            &[original.clone(), original],
            &description,
            "verified-owner",
            &header
        )
        .is_err()
    );
}
