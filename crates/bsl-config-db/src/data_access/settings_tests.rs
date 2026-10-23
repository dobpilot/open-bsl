use super::*;

#[test]
fn only_no_row_is_known_absence() {
    assert_eq!(one_payload(vec![]).unwrap(), None);
    assert_eq!(
        one_payload(vec![vec![Cell::Bytes(vec![1, 2])]]).unwrap(),
        Some(vec![1, 2])
    );
}

#[test]
fn duplicate_wrong_width_and_wrong_type_rows_are_errors() {
    for rows in [
        vec![vec![], vec![]],
        vec![vec![]],
        vec![vec![Cell::Null]],
        vec![vec![Cell::Bytes(vec![]), Cell::Null]],
    ] {
        assert!(one_payload(rows).is_err());
    }
}

#[test]
fn settings_scope_uses_the_requested_store_and_rejects_other_stores() {
    assert_eq!(
        settings_table(ClientSettingsStorage::Common).unwrap(),
        "_commonsettings"
    );
    assert_eq!(
        settings_table(ClientSettingsStorage::System).unwrap(),
        "_systemsettings"
    );
    for storage in [
        ClientSettingsStorage::FormData,
        ClientSettingsStorage::ReportsUser,
        ClientSettingsStorage::DynamicListsUser,
    ] {
        assert!(settings_table(storage).is_err());
    }
}
