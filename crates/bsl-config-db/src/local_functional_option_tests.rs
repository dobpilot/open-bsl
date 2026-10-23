//! Сквозная проверка границы привилегированного чтения функциональной опции.

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
fn privileged_constant_option_does_not_grant_ordinary_reads_on_postgres() {
    let Ok(port) = std::env::var("OPEN_SDBL_TRANSACTION_CONTROL_PORT") else {
        // Без собственного внешнего стенда этот тест не подтверждает исполнение SQL.
        return;
    };
    let (catalog, native) = crate::functional_options::tests::native("native", |_| {});
    let owner: Guid = "9893e2d6-f3f8-4d73-bb06-19bf26d216ab".parse().unwrap();
    let metadata = resolve_metadata(
        DbNames::from_entries(vec![
            DbNameEntry {
                guid: owner.clone(),
                alias: "Const".into(),
                number: 1,
            },
            DbNameEntry {
                guid: owner.clone(),
                alias: "Fld".into(),
                number: 2,
            },
        ]),
        native.descriptors().to_vec(),
        SchemaStorage {
            // Имена и типы взяты из Config; физические номера заданы стендом.
            tables: vec![SchemaTable {
                name: "Const1".into(),
                number: 1,
                owner: None,
                inline_name: None,
                columns: vec![SchemaColumn {
                    name: "Fld2".into(),
                    types: vec![open_sdbl::metadata::ColumnType {
                        tag: "L".into(),
                        reference_target: None,
                    }],
                }],
                indexes: vec![],
            }],
            anomalies: vec![],
        },
        vec![LiveTable {
            name: "_const1".into(),
            columns: vec![LiveColumn {
                name: "_fld2".into(),
                data_type: "boolean".into(),
            }],
            indexes: vec![],
        }],
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
            "CREATE TABLE _const1 (_fld2 boolean NOT NULL); INSERT INTO _const1 VALUES (TRUE);",
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
    let role_id = "22222222-2222-2222-2222-222222222222";
    let record = format!("{{{user_id},\"FunctionalDenied\",\"\",{{1,{role_id}}}}}");
    let mut encoded = vec![1, 0x5a];
    encoded.extend(record.bytes().map(|byte| byte ^ 0x5a));
    let user = InfoBaseUser::new(
        UserRow {
            name: "FunctionalDenied",
            description: "",
            os_name: "",
            email: "",
            show_in_list: false,
            standard_authentication: true,
            administrative: false,
        },
        &encoded,
    )
    .unwrap();
    let mut rights = parse_role_rights(include_bytes!(
        "../tests/fixtures/balance-access/read.deflate"
    ))
    .unwrap();
    rights.objects[0].object = owner;
    assert_eq!(rights.objects[0].rights[0].right, Right::Read);
    rights.objects[0].rights[0].allowed = false;
    let mut access = AccessStore::new(&metadata);
    access.set_users(vec![user]);
    access.insert_rights(role_id.parse().unwrap(), rights);
    let principal = Principal::new(user_id, "FunctionalDenied", vec![role_id.into()]);
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
        metadata,
        access,
        &principal,
    ));
    let session = SessionData::new(Arc::new(catalog), principal).with_data_access(adapter);
    let engine = open_bsl::Engine::builder()
        .register_library(bsl_orm::library())
        .build()
        .unwrap();
    let mut state = engine.state_builder().host_data(Rc::new(session)).build();
    let denied = [
        "Возврат Константы.ЗначениеОпцииПробы.Получить();",
        "Возврат ПолучитьФункциональнуюОпцию(\"ОпцияПробы\");",
    ];
    // Нативная квитанция запрещает обычное чтение и до, и после привилегированной опции.
    for _ in 0..2 {
        for source in denied {
            let failure = state.exec(source).unwrap_err().to_string();
            assert!(
                failure.contains("is denied for user \"FunctionalDenied\""),
                "{failure}"
            );
        }
        for source in [
            "Возврат ПолучитьФункциональнуюОпцию(\"ОпцияПробыПривилегированная\");",
            "Возврат GetFunctionalOption(\"ОпцияПробыПривилегированная\");",
        ] {
            assert_eq!(state.exec(source).unwrap(), Value::Boolean(true));
        }
    }
}
