use super::*;
use bsl_orm::{CompatibilityMode, MetadataCatalog, ScriptVariant};
use open_sdbl::metadata::{
    DbNames, InfoBaseUser, SchemaStorage, UserRow, parse_role_rights, resolve_metadata,
};
use std::sync::Arc;

const ROOT: &str = "26c30ece-33b5-498a-b409-a91a046ba66b";
const ALLOW: &str = "9d3bec1a-c27a-4d3b-94e1-45dd6c85cf81";
const DENY: &str = "114a9841-1e1b-487a-8340-d833179eb15f";
const MISSING: &str = "33333333-3333-4333-8333-333333333333";

struct Host {
    access: AccessStore,
}

impl std::fmt::Debug for Host {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Host").finish_non_exhaustive()
    }
}

impl DataAccess for Host {
    fn constant_value(
        &self,
        _: &MetadataObject,
        _: &Principal,
        _: &mut CallContext<'_>,
    ) -> RtResult<Value> {
        Err(error("неожиданное чтение константы"))
    }

    fn access_right(
        &self,
        request: &AccessRightRequest,
        principal: &Principal,
        context: &mut CallContext<'_>,
    ) -> RtResult<bool> {
        access_right(&self.access, request, principal, context)
    }
}

fn state(roles: &[&str]) -> open_bsl::State {
    let id = "11111111-1111-4111-8111-111111111111";
    let principal = Principal::new(id, "Проба", roles.iter().map(|id| (*id).to_owned()));
    let record = format!(
        r#"{{{id},"Проба","",{{{}{}}}}}"#,
        roles.len(),
        roles.iter().map(|id| format!(",{id}")).collect::<String>()
    );
    let mut bytes = vec![1, 0x5a];
    bytes.extend(record.bytes().map(|byte| byte ^ 0x5a));
    let user = InfoBaseUser::new(
        UserRow {
            name: "Проба",
            description: "",
            os_name: "",
            email: "",
            show_in_list: false,
            standard_authentication: true,
            administrative: false,
        },
        &bytes,
    )
    .unwrap();
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
    access.set_users(vec![user.clone()]);
    access.set_current_user(Some(user));

    access.insert_rights(ALLOW.parse().unwrap(), parse_role_rights(include_bytes!("../../../../docs/research/fixtures/configuration-access-right-complete-native-1015/RootAllow.rights.deflate")).unwrap());
    access.insert_rights(DENY.parse().unwrap(), parse_role_rights(include_bytes!("../../../../docs/research/fixtures/configuration-access-right-complete-native-1015/RootDeny.rights.deflate")).unwrap());
    let catalog = Arc::new(
        MetadataCatalog::new(
            "catalog-namespace",
            vec![
                MetadataObject::new(ALLOW, "RootAllow", MetadataKind::Role),
                MetadataObject::new(DENY, "RootDeny", MetadataKind::Role),
                MetadataObject::new(MISSING, "Missing", MetadataKind::Role),
            ],
        )
        .unwrap()
        .with_configuration_object_id(ROOT)
        .unwrap()
        .with_configuration(
            "Configuration",
            "1",
            ScriptVariant::English,
            CompatibilityMode::Version8_3_24,
            [],
        )
        .unwrap(),
    );
    let data = SessionData::new(catalog, principal).with_data_access(Rc::new(Host { access }));
    open_bsl::Engine::builder()
        .register_library(bsl_orm::library())
        .register_library(bsl_session::library())
        .build()
        .unwrap()
        .state_builder()
        .host_data(Rc::new(data))
        .build()
}

fn native_result(subject: &str, key: &str) -> bool {
    let text = match subject {
        "admin" => include_str!(
            "../../../../docs/research/fixtures/configuration-access-right-complete-native-1015/admin.oracle"
        ),
        "deny" => include_str!(
            "../../../../docs/research/fixtures/configuration-access-right-complete-native-1015/deny.oracle"
        ),
        _ => unreachable!(),
    };
    let prefix = format!("{key}=");
    let value = text
        .trim_start_matches('\u{feff}')
        .lines()
        .find_map(|line| line.strip_prefix(&prefix))
        .unwrap()
        .trim();
    match value {
        "YES" => true,
        "NO" => false,
        _ => panic!("ожидался измеренный булев результат: {key}={value}"),
    }
}

#[test]
fn acquired_root_rights_match_native_current_role_and_user_results() {
    for (subject, roles) in [("admin", vec![ALLOW]), ("deny", vec![DENY])] {
        let mut state = state(&roles);
        for (key, expression) in [
            ("right.current", r#"AccessRight("SaveUserData", Metadata)"#),
            (
                "right.role.deny",
                r#"AccessRight("SaveUserData", Metadata, Metadata.Roles.RootDeny)"#,
            ),
            (
                "right.administration",
                r#"ПравоДоступа("Администрирование", Метаданные)"#,
            ),
        ] {
            assert_eq!(
                state.exec(&format!("Возврат {expression};")).unwrap(),
                Value::Boolean(native_result(subject, key)),
                "{subject}/{key}"
            );
        }
        let explicit = state
            .exec(r#"Возврат AccessRight("SaveUserData", Metadata, InfoBaseUsers.CurrentUser());"#);
        if subject == "admin" {
            assert_eq!(
                explicit.unwrap(),
                Value::Boolean(native_result(subject, "right.self"))
            );
        } else {
            assert!(explicit.unwrap_err().to_string().contains("административ"));
        }
        for (key, expression) in [
            (
                "right.privileged.current",
                r#"AccessRight("SaveUserData", Metadata)"#,
            ),
            (
                "right.privileged.role.deny",
                r#"AccessRight("SaveUserData", Metadata, Metadata.Roles.RootDeny)"#,
            ),
            (
                "right.privileged.self",
                r#"AccessRight("SaveUserData", Metadata, InfoBaseUsers.CurrentUser())"#,
            ),
            (
                "right.privileged.administration",
                r#"AccessRight("Administration", Metadata)"#,
            ),
        ] {
            assert_eq!(
                state
                    .exec(&format!(
                        "УстановитьПривилегированныйРежим(Истина); Возврат {expression};"
                    ))
                    .unwrap(),
                Value::Boolean(native_result(subject, key)),
                "{subject}/{key}"
            );
        }
    }
}

#[test]
fn unknown_and_inapplicable_root_rights_fail_before_privileged_bypass() {
    for privileged in [false, true] {
        for right in ["Read", "UnknownProbeRight"] {
            let source = format!(
                "УстановитьПривилегированныйРежим({}); Возврат AccessRight(\"{right}\", Metadata);",
                if privileged {
                    "Истина"
                } else {
                    "Ложь"
                }
            );
            assert!(state(&[ALLOW]).exec(&source).is_err());
        }
    }
}

#[test]
fn unread_role_data_never_turns_into_a_grant_or_a_refusal() {
    let mut state = state(&[ALLOW, MISSING]);
    assert!(
        state
            .exec(r#"Возврат AccessRight("SaveUserData", Metadata);"#)
            .is_err()
    );
    assert!(state.exec(r#"УстановитьПривилегированныйРежим(Истина); Возврат AccessRight("SaveUserData", Metadata, Metadata.Roles.Missing);"#).is_err());
}

#[test]
fn the_acquired_root_identity_matches_the_native_rights_owner() {
    let bytes = open_sdbl::metadata::inflate_raw_deflate(include_bytes!("../../../../docs/research/fixtures/configuration-access-right-complete-native-1015/root-index.deflate")).unwrap();
    let value = crate::parse_serialized(&bytes).unwrap();
    assert_eq!(crate::first_guid(&value), Some(ROOT));
}

#[path = "document_attribute_rights_tests.rs"]
mod document_attributes;
