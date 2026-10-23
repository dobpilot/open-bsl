//! Значения и аргументы константной функциональной опции на собственном PostgreSQL.

use super::local_option_register_reads_tests::fixture_sql;
use super::*;
use open_sdbl::metadata::{
    DbNameEntry, DbNames, InfoBaseUser, LiveColumn, LiveTable, SchemaColumn, SchemaStorage,
    SchemaTable, UserRow, parse_role_rights, resolve_metadata,
};
use open_sdbl_db::{
    ConnectionOptions, Credentials, DatabaseConnection, Limits, PostgresConnection, PostgresSslMode,
};
use std::sync::Arc;

fn native<'a>(oracle: &'a str, key: &str) -> &'a str {
    oracle
        .lines()
        .filter_map(|line| line.split_once('\t'))
        .find(|(name, _)| *name == key)
        .unwrap()
        .1
}

#[test]
fn constant_option_values_and_arguments_match_native_results_on_postgres() {
    let Ok(port) = std::env::var("OPEN_SDBL_TRANSACTION_CONTROL_PORT") else {
        // Без отдельного стенда обычный workspace-прогон не подтверждает SQL.
        return;
    };
    let (catalog, source) = crate::functional_options::tests::native("native", |_| {});
    let owner: Guid = "9893e2d6-f3f8-4d73-bb06-19bf26d216ab".parse().unwrap();
    let metadata = resolve_metadata(
        DbNames::from_entries(vec![
            DbNameEntry {
                guid: owner.clone(),
                alias: "Const".into(),
                number: 4,
            },
            DbNameEntry {
                guid: owner,
                alias: "Fld".into(),
                number: 40,
            },
        ]),
        source.descriptors().to_vec(),
        SchemaStorage {
            tables: vec![SchemaTable {
                name: "Const4".into(),
                number: 4,
                owner: None,
                inline_name: None,
                columns: vec![SchemaColumn {
                    name: "Fld40".into(),
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
            name: "_const4".into(),
            columns: vec![LiveColumn {
                name: "_fld40".into(),
                data_type: "boolean".into(),
            }],
            indexes: vec![],
        }],
    )
    .snapshot;
    let catalog = crate::functional_options::project(catalog, &metadata).unwrap();
    // Физическая схема и идентификатор пользователя заданы стендом явно.
    fixture_sql(
        &port,
        "CREATE TABLE _const4 (_fld40 boolean); INSERT INTO _const4 VALUES (FALSE);",
    );
    let role_id = "3cfbfd8c-c47e-4185-8f69-d581cf8cedec";
    let user_id = "11111111-1111-1111-1111-111111111111";
    let record = format!("{{{user_id},\"FunctionalAdmin\",\"\",{{1,{role_id}}}}}");
    let mut encoded = vec![1, 0x5a];
    encoded.extend(record.bytes().map(|b| b ^ 0x5a));
    let user = InfoBaseUser::new(
        UserRow {
            name: "FunctionalAdmin",
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
    let mut access = AccessStore::new(&metadata);
    access.set_users(vec![user]);
    access.insert_rights(role_id.parse().unwrap(), parse_role_rights(include_bytes!("../tests/fixtures/functional-options/source-rights-native/3cfbfd8c-c47e-4185-8f69-d581cf8cedec.rights.deflate")).unwrap());
    let principal = Principal::new(user_id, "FunctionalAdmin", vec![role_id.into()]);
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
    let data = SessionData::new(Arc::new(catalog), principal).with_data_access(adapter);
    let engine = open_bsl::Engine::builder()
        .register_library(bsl_orm::library())
        .build()
        .unwrap();
    let mut state = engine.state_builder().host_data(Rc::new(data)).build();
    let values = include_str!(
        "../../../tests/conformance/measure/1c/measure-functional-option-constant.platform.txt"
    );
    let arguments = include_str!(
        "../../../tests/conformance/measure/1c/measure-functional-option-arguments.platform.txt"
    );
    assert_eq!(native(values, "FUNCTIONAL_OPTION.CONSTANT_INITIAL"), "No");
    assert_eq!(
        state
            .exec("Возврат ПолучитьФункциональнуюОпцию(\"ОпцияПробы\");")
            .unwrap(),
        Value::Boolean(false)
    );
    for (id, source) in [
        ("OMITTED", "ПолучитьФункциональнуюОпцию(\"ОпцияПробы\")"),
        (
            "UNDEFINED",
            "ПолучитьФункциональнуюОпцию(\"ОпцияПробы\", Неопределено)",
        ),
        ("NUMBER", "ПолучитьФункциональнуюОпцию(\"ОпцияПробы\", 1)"),
        (
            "STRING",
            "ПолучитьФункциональнуюОпцию(\"ОпцияПробы\", \"строка\")",
        ),
        (
            "ARRAY",
            "ПолучитьФункциональнуюОпцию(\"ОпцияПробы\", Новый Массив)",
        ),
        (
            "STRUCTURE",
            "ПолучитьФункциональнуюОпцию(\"ОпцияПробы\", Новый Структура)",
        ),
        (
            "FIXED_STRUCTURE",
            "ПолучитьФункциональнуюОпцию(\"ОпцияПробы\", Новый ФиксированнаяСтруктура(Новый Структура))",
        ),
        ("NAME_NUMBER", "ПолучитьФункциональнуюОпцию(1)"),
        (
            "NAME_UNDEFINED",
            "ПолучитьФункциональнуюОпцию(Неопределено)",
        ),
    ] {
        let expected = native(arguments, &format!("FUNCTIONAL_OPTION.ARGUMENT_{id}"));
        let result = state.exec(&format!("Возврат {source};"));
        if expected == "ERROR" {
            assert!(
                matches!(result, Err(open_bsl::Error::Runtime(_))),
                "{id}: {result:?}"
            );
        } else {
            assert_eq!(expected, "Boolean:No");
            assert_eq!(result.unwrap(), Value::Boolean(false), "{id}");
        }
    }
    fixture_sql(&port, "UPDATE _const4 SET _fld40 = TRUE;");
    for (id, expression) in [
        (
            "CONSTANT_TRUE",
            "ПолучитьФункциональнуюОпцию(\"ОпцияПробы\")",
        ),
        (
            "EXTRA_PARAMETERS",
            "ПолучитьФункциональнуюОпцию(\"ОпцияПробы\", Новый Структура(\"НесуществующийПараметр\",123))",
        ),
        ("ENGLISH_ALIAS", "GetFunctionalOption(\"ОпцияПробы\")"),
        ("CASE", "ПолучитьФункциональнуюОпцию(\"опцияпробы\")"),
    ] {
        assert_eq!(native(values, &format!("FUNCTIONAL_OPTION.{id}")), "Yes");
        assert_eq!(
            state.exec(&format!("Возврат {expression};")).unwrap(),
            Value::Boolean(true),
            "{id}"
        );
    }
    fixture_sql(&port, "UPDATE _const4 SET _fld40 = FALSE;");
    assert_eq!(native(values, "FUNCTIONAL_OPTION.CONSTANT_FALSE"), "No");
    assert_eq!(
        state
            .exec("Возврат ПолучитьФункциональнуюОпцию(\"ОпцияПробы\");")
            .unwrap(),
        Value::Boolean(false)
    );
    assert!(native(values, "FUNCTIONAL_OPTION.UNKNOWN").contains("functional option is not found"));
    let failure = state
        .exec("Возврат ПолучитьФункциональнуюОпцию(\"НесуществующаяОпцияПробы\");")
        .unwrap_err()
        .to_string();
    assert!(
        failure.contains("функциональная опция") && failure.contains("не найдена"),
        "{failure}"
    );
}
