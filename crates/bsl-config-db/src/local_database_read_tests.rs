//! Проверка адаптера на отдельном локальном PostgreSQL, без рабочих баз.

use super::*;
use bsl_orm::MetadataCatalog;
use open_sdbl::metadata::{
    ConfigDeclaredType, DbNameEntry, DbNames, InfoBaseUser, LiveColumn, LiveTable, Right, UserRow,
    parse_role_rights, parse_schema_storage, resolve_metadata,
};
use open_sdbl_db::{
    ConnectionOptions, Credentials, DatabaseConnection, Limits, PostgresConnection, PostgresSslMode,
};
use std::sync::Arc;

const USER: &str = "11111111-1111-1111-1111-111111111111";
const ROLE: &str = "22222222-2222-2222-2222-222222222222";

fn setup(read_allowed: bool, other_principal: bool) -> Option<(open_bsl::State, String)> {
    // Отсутствие переменной означает отсутствие внешнего тестового стенда.
    // Полный workspace-прогон без неё не подтверждает выполнение SQL этих тестов.
    let port = std::env::var("OPEN_SDBL_TRANSACTION_CONTROL_PORT").ok()?;
    let (catalog, source) = crate::functional_options::tests::native("register-native", |_| {});
    let mut descriptors = source.descriptors().to_vec();
    let owner = descriptors
        .iter_mut()
        .find(|descriptor| descriptor.name == "ОпцииПробы")
        .unwrap();
    let owner_id = owner.object_guid.clone();
    let dimension = owner.information_register_dimensions.as_ref().unwrap()[0].clone();
    let resource = owner.information_register_resources.as_ref().unwrap()[0].clone();
    // Физическая схема и числовой профиль намеренно синтетические. Они проверяют
    // настоящий путь ORM → SDK → PostgreSQL, а не приобретение Config платформы.
    owner.information_register_dimensions = Some(vec![dimension.clone()]);
    owner.information_register_resources = Some(vec![resource.clone()]);
    let mut names = vec![DbNameEntry {
        guid: owner_id.clone(),
        alias: "InfoRg".into(),
        number: 1,
    }];
    let mut columns = Vec::new();
    let mut live = Vec::new();
    let mut field_names = Vec::new();
    for (index, id) in [dimension, resource].iter().enumerate() {
        let field = descriptors
            .iter_mut()
            .find(|descriptor| &descriptor.object_guid == id)
            .unwrap();
        field_names.push(field.name.clone());
        let digits = if index == 0 { "5" } else { "15" };
        field.declared_types = Some(vec![ConfigDeclaredType {
            tag: "N".into(),
            reference: None,
            qualifiers: [digits, "0", "0"]
                .into_iter()
                .map(|part| open_sdbl::metadata::Value::Atom(part.into()))
                .collect(),
        }]);
        let number = index as u32 + 10;
        names.push(DbNameEntry {
            guid: id.clone(),
            alias: "Fld".into(),
            number,
        });
        columns.push(format!(
            "{{\"Fld{number}\",0,{{1,{{\"N\",2147483657,0,\"\",0}}}},\"\",0}}"
        ));
        live.push(LiveColumn {
            name: format!("_fld{number}"),
            data_type: format!("numeric({digits},0)"),
        });
    }
    let schema = parse_schema_storage(
        format!(
            "{{0,{{1,{{\"InfoRg1\",\"N\",1,\"\",{{2,{}}},{{0}},{{0}},1,\"R\",{{0}},{{0}},\"\",0}}}}}}",
            columns.join(",")
        )
        .as_bytes(),
    )
    .unwrap();
    let metadata = resolve_metadata(
        DbNames::from_entries(names),
        descriptors,
        schema,
        vec![LiveTable {
            name: "_inforg1".into(),
            columns: live,
            indexes: vec![],
        }],
    )
    .snapshot;
    let catalog: MetadataCatalog = crate::document_attributes::project(catalog, &metadata).unwrap();
    let mut rights = parse_role_rights(include_bytes!(
        "../tests/fixtures/balance-access/read.deflate"
    ))
    .unwrap();
    rights.set_for_attributes_by_default = false;
    rights.objects[0].object = owner_id;
    assert_eq!(rights.objects[0].rights[0].right, Right::Read);
    rights.objects[0].rights[0].allowed = read_allowed;
    let record = format!("{{{USER},\"read-control\",\"\",{{1,{ROLE}}}}}");
    let mut encoded = vec![1, 0x5a];
    encoded.extend(record.bytes().map(|byte| byte ^ 0x5a));
    let user = InfoBaseUser::new(
        UserRow {
            name: "read-control",
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
    access.insert_rights(ROLE.parse().unwrap(), rights);
    let principal = Principal::new(USER, "read-control", vec![ROLE.to_owned()]);
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
    let principal = if other_principal {
        Principal::new("33333333-3333-3333-3333-333333333333", "other", vec![])
    } else {
        principal
    };
    let session = SessionData::new(Arc::new(catalog), principal).with_data_access(adapter);
    let engine = open_bsl::Engine::builder()
        .register_library(bsl_orm::library())
        .build()
        .unwrap();
    let source = format!(
        "Возврат РегистрыСведений.ОпцииПробы.Получить(Новый Структура(\"{}\", 7)).{};",
        field_names[0], field_names[1]
    );
    Some((
        engine.state_builder().host_data(Rc::new(session)).build(),
        source,
    ))
}

#[test]
fn numeric_register_get_reads_the_actual_postgres_row() {
    let Some((mut state, source)) = setup(true, false) else {
        return;
    };
    assert_eq!(state.exec(&source).unwrap(), Value::number_from_i64(13));
    assert_eq!(
        state.exec(&source.replace(", 7)", ", 8)")).unwrap(),
        Value::number_from_i64(0)
    );
}

#[test]
fn denied_register_read_does_not_return_the_existing_postgres_row() {
    let Some((mut state, source)) = setup(false, false) else {
        return;
    };
    let error = state.exec(&source).unwrap_err().to_string();
    assert!(
        error.contains("field access") && error.contains("denied"),
        "{error}"
    );
}

#[test]
fn another_principal_cannot_use_the_register_connection() {
    let Some((mut state, source)) = setup(true, true) else {
        return;
    };
    let error = state.exec(&source).unwrap_err().to_string();
    assert!(
        error.contains("пользователь запроса не совпадает"),
        "{error}"
    );
}
