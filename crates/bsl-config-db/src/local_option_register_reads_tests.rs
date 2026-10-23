//! Параметризованные опции регистра через Session-адаптер и собственный PostgreSQL.

use super::*;
use open_sdbl::metadata::{
    DbNameEntry, DbNames, InfoBaseUser, LiveColumn, LiveTable, SchemaColumn, SchemaStorage,
    SchemaTable, UserRow, parse_role_rights, resolve_metadata,
};
use open_sdbl_db::{
    ConnectionOptions, Credentials, DatabaseConnection, Limits, PostgresConnection, PostgresSslMode,
};
use std::sync::Arc;

pub(super) fn fixture_sql(port: &str, sql: &str) {
    let result = std::process::Command::new("psql")
        .args([
            "-X",
            "-w",
            "-h",
            "127.0.0.1",
            "-p",
            port,
            "-U",
            "sdbl_control",
            "-d",
            "postgres",
            "-v",
            "ON_ERROR_STOP=1",
            "-c",
            sql,
        ])
        .env("PGPASSFILE", "/dev/null")
        .env_remove("PGPASSWORD")
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
}

fn check_native(state: &mut open_bsl::State, id: &str, option: &str, parameters: &str) {
    let oracle = include_str!(
        "../../../tests/conformance/measure/1c/measure-functional-option-register.platform.txt"
    );
    check_native_result(state, oracle, "REGISTER", id, option, parameters);
}

pub(super) fn check_native_result(
    state: &mut open_bsl::State,
    oracle: &str,
    family: &str,
    id: &str,
    option: &str,
    parameters: &str,
) {
    let key = format!("FUNCTIONAL_OPTION.{family}_{id}");
    let expected = oracle
        .lines()
        .filter_map(|line| line.split_once('\t'))
        .find(|(name, _)| *name == key)
        .unwrap()
        .1;
    for name in ["ПолучитьФункциональнуюОпцию", "GetFunctionalOption"] {
        let result = state.exec(&format!("Возврат {name}(\"{option}\", {parameters});"));
        match expected {
            "Boolean:Yes" => assert_eq!(result.unwrap(), Value::Boolean(true), "{key}/{name}"),
            "Boolean:No" => assert_eq!(result.unwrap(), Value::Boolean(false), "{key}/{name}"),
            "Null:" => assert_eq!(result.unwrap(), Value::Null, "{key}/{name}"),
            number if number.starts_with("Number:") => assert_eq!(
                result.unwrap(),
                Value::number_from_i64(number[7..].parse().unwrap()),
                "{key}/{name}"
            ),
            failure if failure.starts_with("ERROR:") => {
                let failure = result.unwrap_err().to_string();
                assert!(
                    failure.contains("должны быть заданы все параметры источника"),
                    "{key}/{name}: {failure}"
                );
            }
            other => panic!("неподдержанная строка нативного эталона {key}: {other}"),
        }
    }
}

#[test]
fn register_option_parameters_match_native_results_on_postgres() {
    let Ok(port) = std::env::var("OPEN_SDBL_TRANSACTION_CONTROL_PORT") else {
        // Без отдельного стенда обычный workspace-прогон не подтверждает SQL.
        return;
    };
    let (catalog, native) = crate::functional_options::tests::native("register-native", |_| {});
    let owner = native
        .descriptors()
        .iter()
        .find(|d| d.name == "ОпцииПробы")
        .unwrap();
    let mut names = vec![DbNameEntry {
        guid: owner.object_guid.clone(),
        alias: "InfoRg".into(),
        number: 1,
    }];
    let fields = owner
        .information_register_dimensions
        .as_ref()
        .unwrap()
        .iter()
        .chain(owner.information_register_resources.as_ref().unwrap())
        .collect::<Vec<_>>();
    let mut columns = Vec::new();
    let mut live = Vec::new();
    let mut ddl = Vec::new();
    let mut actual_names = Vec::new();
    for (index, id) in fields.iter().enumerate() {
        let field = native
            .descriptors()
            .iter()
            .find(|d| &d.object_guid == *id)
            .unwrap();
        actual_names.push(field.name.as_str());
        let [kind] = field.declared_types.as_deref().unwrap() else {
            panic!("один тип поля")
        };
        let (tag, sql_type) = match kind.tag.as_str() {
            "B" => ("L", "boolean"),
            "N" => ("N", "numeric(15,2)"),
            other => panic!("неизмеренный тип {other}"),
        };
        let number = 10 + index as u32;
        names.push(DbNameEntry {
            guid: (*id).clone(),
            alias: "Fld".into(),
            number,
        });
        columns.push(SchemaColumn {
            name: format!("Fld{number}"),
            types: vec![open_sdbl::metadata::ColumnType {
                tag: tag.into(),
                reference_target: None,
            }],
        });
        live.push(LiveColumn {
            name: format!("_fld{number}"),
            data_type: sql_type.into(),
        });
        ddl.push(format!("_fld{number} {sql_type}"));
    }
    assert_eq!(
        actual_names,
        ["ПервыйКлюч", "ВторойКлюч", "ПризнакОпции", "ЧислоОпции"]
    );
    // Описатели и привязки нативные; физические номера и таблица заданы стендом.
    let metadata = resolve_metadata(
        DbNames::from_entries(names),
        native.descriptors().to_vec(),
        SchemaStorage {
            tables: vec![SchemaTable {
                name: "InfoRg1".into(),
                number: 1,
                owner: None,
                inline_name: None,
                columns,
                indexes: vec![],
            }],
            anomalies: vec![],
        },
        vec![LiveTable {
            name: "_inforg1".into(),
            columns: live,
            indexes: vec![],
        }],
    )
    .snapshot;
    let catalog = crate::functional_options::project(catalog, &metadata).unwrap();
    fixture_sql(
        &port,
        &format!("CREATE TABLE _inforg1 ({});", ddl.join(", ")),
    );
    let user_id = "11111111-1111-1111-1111-111111111111";
    let role_id = "ae8541a3-7d32-419d-be2a-71ff2cce2566";
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
    let mut access = AccessStore::new(&metadata);
    access.set_users(vec![user]);
    // Этот независимый нативный профиль не даёт чтения того же владельца.
    access.insert_rights(role_id.parse().unwrap(), parse_role_rights(include_bytes!("../tests/fixtures/functional-options/source-rights-native/ae8541a3-7d32-419d-be2a-71ff2cce2566.rights.deflate")).unwrap());
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
    let data = SessionData::new(Arc::new(catalog), principal).with_data_access(adapter);
    let engine = open_bsl::Engine::builder()
        .register_library(bsl_orm::library())
        .build()
        .unwrap();
    let mut state = engine.state_builder().host_data(Rc::new(data)).build();
    check_native(&mut state, "BOOL_EMPTY", "ОпцияРегистра", "Новый Структура");
    check_native(
        &mut state,
        "NUMBER_EMPTY",
        "ЧисловаяОпцияРегистра",
        "Новый Структура",
    );
    // Три строки точно повторяют входные значения нативного скрипта замера.
    fixture_sql(
        &port,
        "INSERT INTO _inforg1 VALUES (1,1,FALSE,10),(1,2,TRUE,20),(2,1,FALSE,30);",
    );
    for (id, option, parameters) in [
        (
            "BOOL_FALSE",
            "ОпцияРегистра",
            "Новый Структура(\"ПараметрПервый,ПараметрВторой\",1,1)",
        ),
        (
            "BOOL_TRUE",
            "ОпцияРегистра",
            "Новый Структура(\"ПараметрПервый,ПараметрВторой\",1,2)",
        ),
        (
            "NUMBER",
            "ЧисловаяОпцияРегистра",
            "Новый Структура(\"ПараметрПервый,ПараметрВторой\",1,2)",
        ),
        (
            "BOOL_PARTIAL_TRUE",
            "ОпцияРегистра",
            "Новый Структура(\"ПараметрПервый\",1)",
        ),
        (
            "BOOL_PARTIAL_FALSE",
            "ОпцияРегистра",
            "Новый Структура(\"ПараметрПервый\",2)",
        ),
        (
            "BOOL_PARTIAL_SECOND",
            "ОпцияРегистра",
            "Новый Структура(\"ПараметрВторой\",1)",
        ),
        (
            "NUMBER_PARTIAL",
            "ЧисловаяОпцияРегистра",
            "Новый Структура(\"ПараметрПервый\",2)",
        ),
        (
            "BOOL_NO_MATCH",
            "ОпцияРегистра",
            "Новый Структура(\"ПараметрПервый,ПараметрВторой\",99,99)",
        ),
        (
            "NUMBER_NO_MATCH",
            "ЧисловаяОпцияРегистра",
            "Новый Структура(\"ПараметрПервый,ПараметрВторой\",99,99)",
        ),
    ] {
        check_native(&mut state, id, option, parameters);
    }
}
