use open_sdbl::metadata::{Guid, Right, parse_role_rights};

#[test]
fn compiled_native_settings_role_rights_match_authenticated_controls() {
    let root: Guid = include_str!(
        "../../../../docs/research/fixtures/client-settings-rights-1017/rights/configuration-id.txt"
    )
    .trim()
    .parse()
    .unwrap();
    let profiles: &[(&[u8], bool, bool, bool)] = &[
        (
            include_bytes!(
                "../../../../docs/research/fixtures/client-settings-rights-1017/rights/ControlAdmin.deflate"
            ),
            true,
            true,
            true,
        ),
        (
            include_bytes!(
                "../../../../docs/research/fixtures/client-settings-rights-1017/rights/SettingsNone.deflate"
            ),
            false,
            false,
            false,
        ),
        (
            include_bytes!(
                "../../../../docs/research/fixtures/client-settings-rights-1017/rights/SettingsSave.deflate"
            ),
            false,
            true,
            false,
        ),
        (
            include_bytes!(
                "../../../../docs/research/fixtures/client-settings-rights-1017/rights/SettingsDataAdmin.deflate"
            ),
            true,
            false,
            false,
        ),
        (
            include_bytes!(
                "../../../../docs/research/fixtures/client-settings-rights-1017/rights/SettingsAdmin.deflate"
            ),
            true,
            false,
            true,
        ),
    ];
    for (bytes, data_admin, save, admin) in profiles {
        let rights = parse_role_rights(bytes).unwrap();
        assert_eq!(
            rights.grants(&root, &Right::DataAdministration),
            *data_admin
        );
        assert_eq!(rights.grants(&root, &Right::SaveUserData), *save);
        assert_eq!(rights.grants(&root, &Right::Administration), *admin);
    }
}

fn access() -> super::AccessStore {
    use open_sdbl::metadata::{DbNames, SchemaStorage, resolve_metadata};
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
    let mut access = super::AccessStore::new(&metadata);
    for (id, bytes) in [
        ("251f0dd8-3c43-5a12-bbde-b91b05f7a50d", include_bytes!("../../../../docs/research/fixtures/client-settings-rights-1017/rights/SettingsDataAdmin.deflate").as_slice()),
        ("0b7cbb72-885f-5c1e-afbc-17a227b7fbe2", include_bytes!("../../../../docs/research/fixtures/client-settings-rights-1017/rights/SettingsNone.deflate").as_slice()),
        ("db3e95dc-98ed-5926-bb0f-63999503c890", include_bytes!("../../../../docs/research/fixtures/client-settings-rights-1017/rights/SettingsSave.deflate").as_slice()),
    ] { access.insert_rights(id.parse().unwrap(), parse_role_rights(bytes).unwrap()); }
    access
}

#[test]
fn settings_namespace_permissions_match_native_controls() {
    let access = access();
    let root = include_str!(
        "../../../../docs/research/fixtures/client-settings-rights-1017/rights/configuration-id.txt"
    )
    .trim();
    for (role, allowed) in [
        ("251f0dd8-3c43-5a12-bbde-b91b05f7a50d", true),
        ("0b7cbb72-885f-5c1e-afbc-17a227b7fbe2", false),
        ("db3e95dc-98ed-5926-bb0f-63999503c890", false),
    ] {
        let principal = super::Principal::new("probe", "SettingsProbe", [role.to_owned()]);
        assert!(
            super::authorize_settings_user(&access, &principal, "SettingsProbe", Some(root), false)
                .is_ok()
        );
        for target in ["settingsprobe", "", "GhostUser", "ControlAdmin"] {
            assert_eq!(
                super::authorize_settings_user(&access, &principal, target, Some(root), false)
                    .is_ok(),
                allowed,
                "{role}/{target}"
            );
            assert!(
                super::authorize_settings_user(&access, &principal, target, None, true).is_ok()
            );
        }
    }
}

#[test]
fn missing_settings_rights_are_an_error_instead_of_an_inferred_permission() {
    let access = access();
    let principal = super::Principal::new(
        "probe",
        "SettingsProbe",
        ["33333333-3333-4333-8333-333333333333".to_owned()],
    );
    let root = include_str!(
        "../../../../docs/research/fixtures/client-settings-rights-1017/rights/configuration-id.txt"
    )
    .trim();
    let failure =
        super::authorize_settings_user(&access, &principal, "GhostUser", Some(root), false)
            .unwrap_err();
    assert!(failure.to_string().contains("не приобретены"));
    assert!(super::authorize_settings_user(&access, &principal, "GhostUser", None, false).is_err());
    assert!(
        super::authorize_settings_user(&access, &principal, "SettingsProbe", None, false).is_ok()
    );
}
