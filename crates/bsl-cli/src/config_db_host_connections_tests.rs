use super::*;
use open_sdbl::metadata::{DbNames, SchemaStorage, parse_role_rights, resolve_metadata};

const ROOT: &str = "26c30ece-33b5-498a-b409-a91a046ba66b";
const ROLE: &str = "9d3bec1a-c27a-4d3b-94e1-45dd6c85cf81";

fn store(allowed: bool) -> AccessStore {
    let metadata = resolve_metadata(
        DbNames::from_entries(vec![]),
        vec![],
        SchemaStorage {
            tables: vec![],
            anomalies: vec![],
        },
        vec![],
    )
    .snapshot;
    let mut access = AccessStore::new(&metadata);
    let mut rights = parse_role_rights(include_bytes!(
        "../../../docs/research/fixtures/configuration-access-right-complete-native-1015/RootAllow.rights.deflate"
    )).unwrap();
    rights.set_for_new_objects = false;
    rights
        .objects
        .retain(|object| object.object == ROOT.parse().unwrap());
    let root = rights.objects.first_mut().unwrap();
    let mut grant = root.rights.first().unwrap().clone();
    grant.right = Right::ActiveUsers;
    grant.allowed = allowed;
    root.rights = vec![grant];
    access.insert_rights(ROLE.parse().unwrap(), rights);
    access
}

#[test]
fn authorized_snapshot_keeps_every_real_host_connection() {
    let parent = InfoBaseConnection::new(1, "bsl-cli", "host");
    let child = InfoBaseConnection::new(2, "bsl-cli", "host");
    let actual = authorized_snapshot(
        &store(true),
        &[ROLE.parse().unwrap()],
        Some(ROOT),
        [parent.clone(), child.clone()],
    )
    .unwrap();
    assert_eq!(actual, Some(vec![parent, child]));
}

#[test]
fn denied_principal_gets_no_snapshot() {
    let actual = authorized_snapshot(
        &store(false),
        &[ROLE.parse().unwrap()],
        Some(ROOT),
        [InfoBaseConnection::new(1, "bsl-cli", "host")],
    )
    .unwrap();
    assert_eq!(actual, None);
}

#[test]
fn unknown_role_does_not_expand_another_roles_grants() {
    let actual = authorized_snapshot(
        &store(true),
        &[
            ROLE.parse().unwrap(),
            "11111111-1111-4111-8111-111111111111".parse().unwrap(),
        ],
        Some(ROOT),
        [],
    )
    .unwrap();
    assert_eq!(actual, None);
}

#[test]
fn namespace_identity_does_not_replace_configuration_root() {
    let actual = authorized_snapshot(
        &store(true),
        &[ROLE.parse().unwrap()],
        Some("22222222-2222-4222-8222-222222222222"),
        [],
    )
    .unwrap();
    assert_eq!(actual, None);
}

#[test]
fn missing_configuration_root_is_explicit() {
    assert!(
        authorized_snapshot(&store(true), &[ROLE.parse().unwrap()], None, [])
            .unwrap_err()
            .contains("корневого объекта")
    );
}

#[test]
fn acquired_computer_name_is_not_empty() {
    assert!(!computer_name().unwrap().is_empty());
}

#[test]
fn catalog_exposes_only_its_acquired_configuration_root() {
    let catalog = bsl_orm::MetadataCatalog::new("namespace", vec![]).unwrap();
    assert_eq!(catalog.configuration_object_id(), None);
    let catalog = catalog.with_configuration_object_id(ROOT).unwrap();
    assert_eq!(catalog.identity(), "namespace");
    assert_eq!(catalog.configuration_object_id(), Some(ROOT));
}
