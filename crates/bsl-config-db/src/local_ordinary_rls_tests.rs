//! Сквозной контроль RLS на собственном локальном PostgreSQL.

use super::*;
use bsl_orm::{DocumentAttribute, MetadataCatalog};
use open_bsl::{ColumnType, TypeRef};
use open_sdbl::metadata::{
    DbNameEntry, DbNames, InfoBaseUser, LiveColumn, LiveTable, UserRow, parse_config_descriptors,
    parse_role_rights, parse_schema_storage, resolve_metadata,
};
use open_sdbl_db::{
    ConnectionOptions, Credentials, DatabaseConnection, Limits, PostgresConnection, PostgresSslMode,
};
use std::sync::Arc;

const USER_ID: &str = "11111111-1111-1111-1111-111111111111";

fn setup(category: &str) -> Option<(open_bsl::State, String, String, String)> {
    let port = std::env::var("OPEN_SDBL_TRANSACTION_CONTROL_PORT").ok()?;
    let owner: Guid = "c72d739a-05e3-4e9e-bcc7-3bf8baf5dc63".parse().unwrap();
    let descriptors = parse_config_descriptors(
        owner.as_str(),
        include_bytes!("../tests/fixtures/runtime-object-099/ОстаткиПорядок.deflate"),
    )
    .unwrap();
    let register = descriptors
        .iter()
        .find(|descriptor| descriptor.object_guid == owner)
        .unwrap();
    let register_name = register.name.clone();
    let dimensions = register.accumulation_register_dimensions.as_ref().unwrap();
    let resources = register.accumulation_register_resources.as_ref().unwrap();
    let field_name = |id: &Guid| {
        descriptors
            .iter()
            .find(|descriptor| &descriptor.object_guid == id)
            .unwrap()
            .name
            .clone()
    };
    let dimension = field_name(&dimensions[0]);
    let resource = field_name(&resources[0]);
    let attributes = |ids: &[Guid], native, qualifiers: &[&str]| {
        ids.iter()
            .map(|id| {
                DocumentAttribute::new(
                    id.as_str(),
                    field_name(id),
                    vec![ColumnType {
                        id: TypeRef::Native(native),
                        quals: qualifiers.iter().map(|value| (*value).into()).collect(),
                    }],
                )
            })
            .collect()
    };
    let catalog = MetadataCatalog::new(
        "balance-access",
        vec![MetadataObject::new(
            owner.as_str(),
            &register.name,
            MetadataKind::AccumulationRegister,
        )],
    )
    .unwrap()
    .with_accumulation_register_metadata(
        owner.as_str(),
        Some(bsl_orm::AccumulationRegisterKind::Balance),
        Some(attributes(dimensions, TypeId::String, &["12", "1"])),
        Some(attributes(resources, TypeId::Number, &["15", "2", "0"])),
    )
    .unwrap();
    let fields = dimensions.iter().chain(resources).collect::<Vec<_>>();
    let mut names = vec![
        DbNameEntry {
            guid: owner.clone(),
            alias: "AccumRg".into(),
            number: 1,
        },
        DbNameEntry {
            guid: owner,
            alias: "AccumRgT".into(),
            number: 2,
        },
    ];
    let mut columns = vec!["{\"Period\",0,{1,{\"T\",0,0,\"\",0}},\"\",0}".to_owned()];
    let mut live_columns = vec![LiveColumn {
        name: "_period".into(),
        data_type: "timestamp without time zone".into(),
    }];
    for (index, id) in fields.iter().enumerate() {
        let number = index as u32 + 10;
        names.push(DbNameEntry {
            guid: (*id).clone(),
            alias: "Fld".into(),
            number,
        });
        let tag = if index < dimensions.len() { "S" } else { "N" };
        columns.push(format!(
            "{{\"Fld{number}\",0,{{1,{{\"{tag}\",2147483657,0,\"\",0}}}},\"\",0}}"
        ));
        live_columns.push(LiveColumn {
            name: format!("_fld{number}"),
            data_type: if tag == "S" {
                "mvarchar(12)"
            } else {
                "numeric(15,2)"
            }
            .into(),
        });
    }
    for (name, tag, sql_type) in [
        ("Active", "L", "boolean"),
        ("RecordKind", "N", "numeric(1,0)"),
        ("Splitter", "N", "numeric(10,0)"),
    ] {
        columns.push(format!(
            "{{\"{name}\",0,{{1,{{\"{tag}\",0,0,\"\",0}}}},\"\",0}}"
        ));
        live_columns.push(LiveColumn {
            name: format!("_{}", name.to_ascii_lowercase()),
            data_type: sql_type.into(),
        });
    }
    // Физические номера синтетические; настоящий PostgreSQL проверяет исполнение
    // через адаптер, но не приобретение Config или физическую схему платформы.
    let tables = [("AccumRg1", 1), ("AccumRgT2", 2)].map(|(name, number)| {
        format!(
            "{{\"{name}\",\"N\",{number},\"\",{{{},{}}},{{0}},{{0}},1,\"R\",{{0}},{{0}},\"\",0}}",
            columns.len(),
            columns.join(",")
        )
    });
    let schema =
        parse_schema_storage(format!("{{0,{{2,{}}}}}", tables.join(",")).as_bytes()).unwrap();
    let live = ["_accumrg1", "_accumrgt2"].map(|name| LiveTable {
        name: name.into(),
        columns: live_columns.clone(),
        indexes: vec![],
    });
    let ddl = live_columns
        .iter()
        .map(|column| {
            format!(
                "{} {}",
                column.name,
                column.data_type.replace("mvarchar", "varchar")
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    let values = live_columns
        .iter()
        .map(|column| match column.name.as_str() {
            "_period" => "'2026-01-01'",
            "_active" => "TRUE",
            "_recordkind" | "_splitter" => "0",
            _ if column.data_type.starts_with("mvarchar") => "'a'",
            _ => "13",
        })
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        "CREATE TABLE IF NOT EXISTS _accumrgt2 ({ddl}); INSERT INTO _accumrgt2 SELECT {values} WHERE NOT EXISTS (SELECT 1 FROM _accumrgt2);"
    );
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
            &sql,
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
    let snapshot = resolve_metadata(
        DbNames::from_entries(names),
        descriptors,
        schema,
        live.into(),
    )
    .snapshot;
    let role: Guid = "22222222-2222-2222-2222-222222222222".parse().unwrap();
    let record = format!("{{{USER_ID},\"balance-user\",\"\",{{1,{role}}}}}");
    let mut encoded = vec![1, 0x5a];
    encoded.extend(record.bytes().map(|byte| byte ^ 0x5a));
    let user = InfoBaseUser::new(
        UserRow {
            name: "balance-user",
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
    assert_eq!(user.data.id.as_str(), USER_ID);
    let mut rights = parse_role_rights(include_bytes!(
        "../tests/fixtures/balance-access/read-rls.deflate"
    ))
    .unwrap();
    rights.objects[0].rights[0].restrictions[0].condition =
        format!("ГДЕ {dimension} = \"{category}\"");
    let mut access = AccessStore::new(&snapshot);
    access.set_users(vec![user]);
    access.insert_rights(role.clone(), rights);
    let principal = Principal::new(USER_ID, "balance-user", vec![role.as_str().to_owned()]);
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
        snapshot,
        access,
        &principal,
    ));
    let session = SessionData::new(Arc::new(catalog), principal).with_data_access(adapter);
    let engine = open_bsl::Engine::builder()
        .register_library(bsl_orm::library())
        .build()
        .unwrap();
    Some((
        engine.state_builder().host_data(Rc::new(session)).build(),
        register_name,
        dimension,
        resource,
    ))
}

#[test]
fn ordinary_rls_is_checked_through_the_actual_session_adapter() {
    for category in ["a", "b"] {
        let Some((mut state, owner, dimension, resource)) = setup(category) else {
            return;
        };
        for (projection, expected) in [
            (dimension.clone(), Value::Str("a".into())),
            (format!("{resource}Остаток"), Value::number_from_i64(13)),
        ] {
            let source = format!(
                "ЗапросПробы = Новый Запрос(\"ВЫБРАТЬ {projection} КАК ПолеКонтроля ИЗ РегистрНакопления.{owner}.Остатки()\"); ВыборкаПробы = ЗапросПробы.Выполнить().Выбрать(); Если ВыборкаПробы.Следующий() Тогда Возврат ВыборкаПробы.ПолеКонтроля; КонецЕсли; Возврат Неопределено;"
            );
            if category == "a" {
                assert_eq!(state.exec(&source).unwrap(), expected);
            } else {
                assert!(
                    state
                        .exec(&source)
                        .unwrap_err()
                        .to_string()
                        .contains("недостаточно прав")
                );
            }
        }
        let source = format!(
            "ЗапросПробы = Новый Запрос(\"ВЫБРАТЬ РАЗРЕШЕННЫЕ {dimension}, {resource}Остаток ИЗ РегистрНакопления.{owner}.Остатки()\"); Возврат ЗапросПробы.Выполнить().Выбрать().Следующий();"
        );
        assert_eq!(
            state.exec(&source).unwrap(),
            Value::Boolean(category == "a")
        );
        for filter in ["a", "b"] {
            let source = format!(
                r#"Возврат РегистрыНакопления.{owner}.Остатки(,
                    Новый Структура("{dimension}", "{filter}"),
                    "{dimension}", "{resource}").Количество();"#,
            );
            if category == "b" && filter == "a" {
                assert!(
                    state
                        .exec(&source)
                        .unwrap_err()
                        .to_string()
                        .contains("недостаточно прав")
                );
            } else {
                let count = i64::from(category == "a" && filter == "a");
                assert_eq!(state.exec(&source).unwrap(), Value::number_from_i64(count));
            }
        }
    }
}

#[test]
fn an_unchecked_nested_placement_is_rejected_before_sql_capture() {
    let Some((mut state, owner, dimension, _)) = setup("a") else {
        return;
    };
    let text = format!(
        "ВЫБРАТЬ РАЗРЕШЕННЫЕ Вложенный.{dimension} КАК guard_probe_before_access \
         ПОМЕСТИТЬ ВТКонтроль \
         ИЗ (ВЫБРАТЬ {dimension} ИЗ РегистрНакопления.{owner}.Остатки()) КАК Вложенный; \
         ВЫБРАТЬ РАЗРЕШЕННЫЕ guard_probe_before_access ИЗ ВТКонтроль"
    );
    let source =
        format!("ЗапросПробы = Новый Запрос(\"{text}\"); Возврат ЗапросПробы.Выполнить();");
    let failure = state.exec(&source).unwrap_err().to_string();
    assert!(
        failure.contains("требуется `ВЫБРАТЬ РАЗРЕШЕННЫЕ`"),
        "{failure}"
    );
    if let Ok(path) = std::env::var("OPEN_BSL_RLS_CONTROL_POSTGRES_LOG") {
        // Отдельный runner включает log_statement только у собственного сервера.
        // Отказ после SQL-захвата уже был бы слишком поздним для этой проверки.
        let statements = std::fs::read_to_string(path).unwrap();
        assert!(
            !statements.contains("guard_probe_before_access"),
            "непроверенный источник временной таблицы прочитан до отказа доступа"
        );
    }
}
