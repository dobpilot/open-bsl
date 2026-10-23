//! Нативная граница доступа опций справочника и регистра на собственном PostgreSQL.

use super::*;
use open_sdbl::metadata::{
    DbNameEntry, DbNames, InfoBaseUser, LiveColumn, LiveTable, Right, SchemaColumn, SchemaStorage,
    SchemaTable, UserRow, parse_role_rights, resolve_metadata,
};
use open_sdbl_db::{
    ConnectionOptions, Credentials, DatabaseConnection, Limits, PostgresConnection, PostgresSslMode,
};
use std::sync::Arc;

#[test]
fn privileged_catalog_and_register_options_keep_ordinary_access_denied_on_postgres() {
    let Ok(port) = std::env::var("OPEN_SDBL_TRANSACTION_CONTROL_PORT") else {
        // Обычный workspace-прогон без стенда не подтверждает исполнение SQL.
        return;
    };
    let (catalog, native) =
        crate::functional_options::tests::native("source-rights-native", |_| {});
    let mut names = Vec::new();
    let mut tables = Vec::new();
    let mut live = Vec::new();
    let mut ddl = Vec::new();
    let mut owners = Vec::new();
    for (owner_name, alias, number) in [
        ("ЗамерВременныхТаблиц", "Reference", 1),
        ("ОпцииПробы", "InfoRg", 2),
        ("ЗначениеОпцииПробы", "Const", 3),
    ] {
        let owner = native
            .descriptors()
            .iter()
            .find(|d| d.name == owner_name)
            .unwrap();
        owners.push(owner.object_guid.clone());
        names.push(DbNameEntry {
            guid: owner.object_guid.clone(),
            alias: alias.into(),
            number,
        });
        let fields = if alias == "Reference" {
            owner.catalog_attributes.clone().unwrap()
        } else if alias == "Const" {
            vec![owner.object_guid.clone()]
        } else {
            owner
                .information_register_dimensions
                .as_ref()
                .unwrap()
                .iter()
                .chain(owner.information_register_resources.as_ref().unwrap())
                .cloned()
                .collect()
        };
        let mut columns = Vec::new();
        let mut physical = Vec::new();
        let mut definitions = Vec::new();
        let mut values = Vec::new();
        for (index, id) in fields.iter().enumerate() {
            let field = native
                .descriptors()
                .iter()
                .find(|d| &d.object_guid == id)
                .unwrap();
            let [kind] = field.declared_types.as_deref().unwrap() else {
                panic!("один тип поля")
            };
            let (tag, sql_type, value) = match kind.tag.as_str() {
                "B" => ("L", "boolean", "TRUE"),
                "N" => ("N", "numeric(15,2)", "1"),
                other => panic!("неизмеренный тип источника {other}"),
            };
            let number = 10 + number * 10 + index as u32;
            names.push(DbNameEntry {
                guid: id.clone(),
                alias: "Fld".into(),
                number,
            });
            let name = format!("Fld{number}");
            columns.push(SchemaColumn {
                name: name.clone(),
                types: vec![open_sdbl::metadata::ColumnType {
                    tag: tag.into(),
                    reference_target: None,
                }],
            });
            physical.push(LiveColumn {
                name: format!("_{}", name.to_ascii_lowercase()),
                data_type: sql_type.into(),
            });
            definitions.push(format!("_{} {sql_type}", name.to_ascii_lowercase()));
            values.push(value);
        }
        if alias == "Reference" {
            columns.push(SchemaColumn {
                name: "Folder".into(),
                types: vec![open_sdbl::metadata::ColumnType {
                    tag: "L".into(),
                    reference_target: None,
                }],
            });
            physical.push(LiveColumn {
                name: "_folder".into(),
                data_type: "boolean".into(),
            });
            definitions.push("_folder boolean".into());
            // В существующем SDK ЭтоГруппа — отрицание физического Folder.
            values.push("TRUE");
        }
        let name = format!("{alias}{number}");
        ddl.push(format!(
            "CREATE TABLE _{} ({}); INSERT INTO _{} VALUES ({});",
            name.to_ascii_lowercase(),
            definitions.join(", "),
            name.to_ascii_lowercase(),
            values.join(", ")
        ));
        tables.push(SchemaTable {
            name: name.clone(),
            number,
            owner: None,
            inline_name: None,
            columns,
            indexes: vec![],
        });
        live.push(LiveTable {
            name: format!("_{}", name.to_ascii_lowercase()),
            columns: physical,
            indexes: vec![],
        });
    }
    // Config-ресурсы и права ролей нативные; физическая схема задана стендом.
    let metadata = resolve_metadata(
        DbNames::from_entries(names),
        native.descriptors().to_vec(),
        SchemaStorage {
            tables,
            anomalies: vec![],
        },
        live,
    )
    .snapshot;
    let catalog = crate::functional_options::project(catalog, &metadata).unwrap();
    let fixture = std::process::Command::new("psql")
        .args([
            "-X",
            "-w",
            "-h",
            "127.0.0.1",
            "-p",
            &port,
            "-U",
            "sdbl_control",
            "-d",
            "postgres",
            "-v",
            "ON_ERROR_STOP=1",
            "-c",
            &ddl.join(" "),
        ])
        .env("PGPASSFILE", "/dev/null")
        .env_remove("PGPASSWORD")
        .output()
        .unwrap();
    assert!(
        fixture.status.success(),
        "{}",
        String::from_utf8_lossy(&fixture.stderr)
    );
    let user_id = "11111111-1111-1111-1111-111111111111";
    let catalog = Arc::new(catalog);
    for (granted, role_id, profile) in [
        (false, "ae8541a3-7d32-419d-be2a-71ff2cce2566", include_bytes!("../tests/fixtures/functional-options/source-rights-native/ae8541a3-7d32-419d-be2a-71ff2cce2566.rights.deflate").as_slice()),
        (true, "3cfbfd8c-c47e-4185-8f69-d581cf8cedec", include_bytes!("../tests/fixtures/functional-options/source-rights-native/3cfbfd8c-c47e-4185-8f69-d581cf8cedec.rights.deflate").as_slice()),
    ] {
        let current_rights = parse_role_rights(profile).unwrap();
        for owner in &owners {
            assert_eq!(current_rights.grants(owner, &Right::Read), granted);
        }
        let name = if granted { "FunctionalAdmin" } else { "FunctionalDenied" };
        let record = format!("{{{user_id},\"{name}\",\"\",{{1,{role_id}}}}}");
        let mut encoded = vec![1, 0x5a];
        encoded.extend(record.bytes().map(|byte| byte ^ 0x5a));
        let user = InfoBaseUser::new(UserRow { name, description: "", os_name: "", email: "", show_in_list: false, standard_authentication: true, administrative: false }, &encoded).unwrap();
        let mut access = AccessStore::new(&metadata);
        access.set_users(vec![user]);
        access.insert_rights(role_id.parse().unwrap(), current_rights);
        let principal = Principal::new(user_id, name, vec![role_id.into()]);
        let target = DatabaseConnection::Postgres(PostgresConnection {
            options: ConnectionOptions {
                host: "127.0.0.1".into(),
                port: port.parse().unwrap(),
                database: "postgres".into(),
                user: "sdbl_control".into(),
                socks5_proxy: None,
            },
            sslmode: PostgresSslMode::Disable,
            trust_ca_file: None,
        });
        let connection = BlockingDatabaseSession::connect(
            &target,
            &Credentials::new(None, None, None),
            Limits::default(),
        )
        .unwrap();
        let adapter = Rc::new(DatabaseDataAccess::new(
            Rc::new(RefCell::new(connection)),
            metadata.clone(),
            access,
            &principal,
        ));
        let data = SessionData::new(Arc::clone(&catalog), principal).with_data_access(adapter);
        let engine = open_bsl::Engine::builder()
            .register_library(bsl_orm::library())
            .build()
            .unwrap();
        let mut state = engine.state_builder().host_data(Rc::new(data)).build();
        for (normal, privileged, owner) in [
            ("ОпцияПробы", "ОпцияПробыПривилегированная", "Константы.ЗначениеОпцииПробы"),
            (
                "ОпцияСправочникаОбычная",
                "ОпцияСправочника",
                "Справочники.ЗамерВременныхТаблиц",
            ),
            (
                "ОпцияРегистраОбычная",
                "ОпцияРегистра",
                "РегистрыСведений.ОпцииПробы",
            ),
        ] {
            if granted {
                for right in ["Чтение", "Read"] {
                    assert_eq!(
                        state
                            .exec(&format!(
                                "Возврат AccessRight(\"{right}\", Metadata.{owner});"
                            ))
                            .unwrap(),
                        Value::Boolean(true)
                    );
                }
                continue;
            }
            for _ in 0..2 {
                let permission = format!("Возврат ПравоДоступа(\"Чтение\", Метаданные.{owner});");
                assert_eq!(state.exec(&permission).unwrap(), Value::Boolean(false));
                for call in ["ПолучитьФункциональнуюОпцию", "GetFunctionalOption"]
                {
                    let failure = state
                        .exec(&format!("Возврат {call}(\"{normal}\");"))
                        .unwrap_err()
                        .to_string();
                    assert!(
                        failure.contains("is denied for user \"FunctionalDenied\""),
                        "{failure}"
                    );
                    assert_eq!(
                        state
                            .exec(&format!("Возврат {call}(\"{privileged}\");"))
                            .unwrap(),
                        Value::Boolean(true)
                    );
                }
                assert_eq!(state.exec(&permission).unwrap(), Value::Boolean(false));
            }
        }
    }
}
