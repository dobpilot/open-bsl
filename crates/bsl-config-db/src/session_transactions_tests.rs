use super::*;
use open_sdbl::metadata::{DbNames, SchemaStorage, resolve_metadata};
use open_sdbl_db::{
    ConnectionOptions, Credentials, DatabaseConnection, Limits, PostgresConnection, PostgresSslMode,
};

pub(super) fn control() -> Option<(Rc<DatabaseDataAccess>, open_bsl::State, Principal)> {
    let port = std::env::var("OPEN_SDBL_TRANSACTION_CONTROL_PORT").ok()?;
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
    let connection = Rc::new(RefCell::new(
        BlockingDatabaseSession::connect(
            &target,
            &Credentials::new(None, None, None),
            Limits::default(),
        )
        .unwrap(),
    ));
    let snapshot = resolve_metadata(
        DbNames::from_entries(vec![]),
        vec![],
        SchemaStorage {
            tables: vec![],
            anomalies: vec![],
        },
        vec![],
    )
    .snapshot;
    let id = "11111111-1111-1111-1111-111111111111";
    let record = format!("{{{id},\"control\",\"\",{{0}}}}");
    let mut encoded = vec![1, 0x5a];
    encoded.extend(record.bytes().map(|byte| byte ^ 0x5a));
    let user = open_sdbl::metadata::InfoBaseUser::new(
        open_sdbl::metadata::UserRow {
            name: "control",
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
    let mut access = AccessStore::new(&snapshot);
    access.set_users(vec![user]);
    let principal = Principal::new(id, "control", vec![]);
    let adapter = Rc::new(DatabaseDataAccess::new(
        connection, snapshot, access, &principal,
    ));
    let catalog =
        std::sync::Arc::new(bsl_orm::MetadataCatalog::new("transaction-control", vec![]).unwrap());
    let session =
        Rc::new(SessionData::new(catalog, principal.clone()).with_data_access(adapter.clone()));
    let engine = open_bsl::Engine::builder()
        .register_library(bsl_orm::library())
        .build()
        .unwrap();
    Some((
        adapter,
        engine.state_builder().host_data(session).build(),
        principal,
    ))
}

#[test]
fn nested_transaction_keeps_the_actual_connection_until_outer_commit() {
    let Some((adapter, mut state, _)) = control() else {
        return;
    };
    assert!(state.exec("ЗафиксироватьТранзакцию();").is_err());
    state
        .exec("НачатьТранзакцию(); НачатьТранзакцию();")
        .unwrap();
    let first = adapter
        .connection
        .borrow_mut()
        .query("SELECT pg_backend_pid()::text, txid_current()::text", 2)
        .unwrap();
    assert_eq!(
        state
            .exec("ЗафиксироватьТранзакцию(); Возврат ТранзакцияАктивна();")
            .unwrap(),
        Value::Boolean(true)
    );
    assert_eq!(
        adapter
            .connection
            .borrow_mut()
            .query("SELECT pg_backend_pid()::text, txid_current()::text", 2)
            .unwrap(),
        first
    );
    assert_eq!(
        state
            .exec("ЗафиксироватьТранзакцию(); Возврат ТранзакцияАктивна();")
            .unwrap(),
        Value::Boolean(false)
    );
    assert!(state.exec("ОтменитьТранзакцию();").is_err());
}

#[test]
fn inner_rollback_blocks_reads_and_leaves_the_other_session_independent() {
    let Some((adapter, mut state, principal)) = control() else {
        return;
    };
    let (_, mut other, _) = control().unwrap();
    assert_eq!(state.exec("НачатьТранзакцию(); НачатьТранзакцию(); ОтменитьТранзакцию(); Возврат ТранзакцияАктивна();").unwrap(), Value::Boolean(true));
    assert!(adapter.check_principal(&principal).is_err());
    assert_eq!(
        state
            .exec("НачатьТранзакцию(); ЗафиксироватьТранзакцию(); Возврат ТранзакцияАктивна();")
            .unwrap(),
        Value::Boolean(true)
    );
    assert!(adapter.check_principal(&principal).is_err());
    assert_eq!(
        other.exec("Возврат ТранзакцияАктивна();").unwrap(),
        Value::Boolean(false)
    );
    assert_eq!(
        state
            .exec("ЗафиксироватьТранзакцию(); Возврат ТранзакцияАктивна();")
            .unwrap(),
        Value::Boolean(false)
    );
    assert!(adapter.check_principal(&principal).is_ok());
    assert_eq!(
        adapter
            .connection
            .borrow_mut()
            .query("SELECT current_setting('transaction_read_only')", 1)
            .unwrap(),
        vec![vec![Cell::Text("on".into())]]
    );
}

#[test]
fn sdk_failure_clears_actual_state_and_closed_connections_never_report_active() {
    let Some((adapter, mut state, _)) = control() else {
        return;
    };
    state.exec("НачатьТранзакцию();").unwrap();
    assert!(
        adapter
            .connection
            .borrow_mut()
            .query("SELECT 1/0", 1)
            .is_err()
    );
    assert_eq!(
        state.exec("Возврат ТранзакцияАктивна();").unwrap(),
        Value::Boolean(false)
    );
    state
        .exec("НачатьТранзакцию(); ОтменитьТранзакцию();")
        .unwrap();
    adapter.connection.borrow_mut().close().unwrap();
    assert!(state.exec("Возврат ТранзакцияАктивна();").is_err());
    assert!(state.exec("НачатьТранзакцию();").is_err());
}

#[test]
fn ordinary_sdk_mode_rejects_changes_and_closed_connections() {
    let Some((adapter, mut state, _)) = control() else {
        return;
    };
    assert_eq!(
        state.exec("Возврат МонопольныйРежим();").unwrap(),
        Value::Boolean(false)
    );
    assert_eq!(
        state.exec("Возврат ExclusiveMode();").unwrap(),
        Value::Boolean(false)
    );
    assert!(state.exec("УстановитьМонопольныйРежим(Истина);").is_err());
    assert!(state.exec("SetExclusiveMode(False);").is_err());
    adapter.connection.borrow_mut().close().unwrap();
    assert!(state.exec("Возврат МонопольныйРежим();").is_err());
}

#[test]
fn ordinary_sdk_mode_rejects_a_different_principal() {
    let Some((adapter, _, _)) = control() else {
        return;
    };
    let engine = open_bsl::Engine::builder()
        .register_library(bsl_orm::library())
        .build()
        .unwrap();
    let catalog = std::sync::Arc::new(bsl_orm::MetadataCatalog::new("other", vec![]).unwrap());
    let principal = Principal::new("22222222-2222-2222-2222-222222222222", "other", vec![]);
    let session = Rc::new(SessionData::new(catalog, principal).with_data_access(adapter));
    let mut state = engine.state_builder().host_data(session).build();
    let err = state.exec("Возврат МонопольныйРежим();").unwrap_err();
    assert!(
        err.to_string()
            .contains("пользователь запроса не совпадает")
    );
}
