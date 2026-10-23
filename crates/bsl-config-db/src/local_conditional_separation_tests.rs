//! Исполнение условного разделения и ленивого запроса параметров на собственном PostgreSQL.

use super::local_option_register_reads_tests::fixture_sql;
use super::*;
use bsl_config::{ConfigurationCodeSnapshot, ConfigurationModule, ModuleExecution, ModuleKind};
use bsl_orm::{MetadataCatalog, MetadataObject};
use bsl_session::{SessionFactory, SessionKind, SessionResources};
use open_sdbl::metadata::{
    ConfigDeclaredType, InfoBaseUser, LiveColumn, LiveTable, SchemaColumn, SchemaStorage,
    SchemaTable, SeparatedDataUse, UserRow, parse_config_descriptors, parse_db_names,
    parse_role_rights, resolve_metadata,
};
use open_sdbl_db::{
    ConnectionOptions, Credentials, DatabaseConnection, Limits, PostgresConnection, PostgresSslMode,
};
use std::sync::Arc;

const CATALOG: &str = "a7177622-2cb3-5bfd-a0d7-1ae7a4a9f4d5";
const SEPARATOR: &str = "03f9d2d0-eae4-4098-a70e-4eb569b11fd9";
const CONDITION: &str = "9893e2d6-f3f8-4d73-bb06-19bf26d216ab";
const COUNTER: &str = "СчетчикЛенивыхВызовов";

fn column(name: &str, tag: &str) -> SchemaColumn {
    SchemaColumn {
        name: name.into(),
        types: vec![open_sdbl::metadata::ColumnType {
            tag: tag.into(),
            reference_target: None,
        }],
    }
}

fn native_value(numeric: bool, shared: bool, id: &str) -> Value {
    let oracle = match (numeric, shared) {
        (false, false) => include_str!(
            "../../../tests/conformance/measure/1c/measure-common-attribute-conditional-separation.platform.txt"
        ),
        (false, true) => include_str!(
            "../../../tests/conformance/measure/1c/measure-common-attribute-conditional-separation-shared.platform.txt"
        ),
        (true, false) => include_str!(
            "../../../tests/conformance/measure/1c/measure-common-attribute-conditional-separation-number.platform.txt"
        ),
        (true, true) => include_str!(
            "../../../tests/conformance/measure/1c/measure-common-attribute-conditional-separation-number-shared.platform.txt"
        ),
    };
    let value = oracle
        .lines()
        .filter_map(|line| line.split_once('\t'))
        .find_map(|(name, value)| (name == id).then_some(value))
        .unwrap();
    if id.ends_with("_count") {
        Value::number_from_i64(value.parse().unwrap())
    } else {
        Value::Str(value.into())
    }
}

fn check_conditional_profile(numeric: bool, shared: bool) {
    let Ok(port) = std::env::var("OPEN_SDBL_TRANSACTION_CONTROL_PORT") else {
        // Полный workspace-прогон без собственного стенда не исполняет SQL.
        return;
    };
    let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/common-content-condition");
    let mut descriptors = std::fs::read_to_string(base.join("resources.txt")).unwrap().lines().flat_map(|id| {
            let bytes = if id == SEPARATOR {
                include_bytes!("../tests/fixtures/global-separation-condition/03f9d2d0-eae4-4098-a70e-4eb569b11fd9.deflate").to_vec()
            } else { std::fs::read(base.join(format!("{id}.deflate"))).unwrap() };
            parse_config_descriptors(id, &bytes).unwrap()
        }).collect::<Vec<_>>();
    let index = descriptors
        .iter()
        .position(|d| d.object_guid.as_str() == SEPARATOR)
        .unwrap();
    let settings = descriptors[index].separation.as_mut().unwrap();
    assert_eq!(
        settings.conditional_separation.as_ref().unwrap().as_str(),
        CONDITION
    );
    let value_id = settings.value_parameter.clone().unwrap();
    let use_id = settings.use_parameter.clone().unwrap();
    // Исходный CF подтверждает String Independent и идентичность привязок.
    // Остальные типизированные профили явно заданы по отдельным нативным XML;
    // это проверка исполнения SDK, не декодирования новых вариантов CF.
    settings.mode = if shared {
        SeparatedDataUse::IndependentAndShared
    } else {
        SeparatedDataUse::Independent
    };
    if numeric {
        let ty = ConfigDeclaredType {
            tag: "N".into(),
            reference: None,
            qualifiers: ["10", "0", "1"]
                .into_iter()
                .map(|s| open_sdbl::metadata::Value::Atom(s.into()))
                .collect(),
        };
        descriptors[index].declared_types = Some(vec![ty.clone()]);
        descriptors
            .iter_mut()
            .find(|d| d.object_guid == value_id)
            .unwrap()
            .declared_types = Some(vec![ty]);
    }
    let name = |id: &str| {
        descriptors
            .iter()
            .find(|d| d.object_guid.as_str() == id)
            .unwrap_or_else(|| panic!("описатель {id} отсутствует"))
            .name
            .clone()
    };
    let value_name = name(value_id.as_str());
    let use_name = name(use_id.as_str());
    let mut objects = vec![
        MetadataObject::new(CATALOG, name(CATALOG), MetadataKind::Catalog),
        MetadataObject::new(SEPARATOR, name(SEPARATOR), MetadataKind::CommonAttribute)
            .with_common_attribute_data_separation(true),
        MetadataObject::new(CONDITION, name(CONDITION), MetadataKind::Constant),
        MetadataObject::new(
            value_id.as_str(),
            &value_name,
            MetadataKind::SessionParameter,
        ),
        MetadataObject::new(use_id.as_str(), &use_name, MetadataKind::SessionParameter),
        MetadataObject::new(
            "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa",
            COUNTER,
            MetadataKind::SessionParameter,
        ),
    ];
    // Полный состав исходного общего реквизита содержит неявный каталог.
    let implicit = "1316b080-3b2c-501b-ace3-a1234a3fedd1";
    objects.push(MetadataObject::new(
        implicit,
        name(implicit),
        MetadataKind::Catalog,
    ));
    let metadata = resolve_metadata(
        parse_db_names(include_bytes!(
            "../tests/fixtures/conditional-session-control/db-names.deflate"
        ))
        .unwrap(),
        descriptors,
        SchemaStorage {
            tables: vec![
                SchemaTable {
                    name: "Reference11".into(),
                    number: 11,
                    owner: None,
                    inline_name: None,
                    columns: vec![
                        column("Description", "S"),
                        column("Fld20", if numeric { "N" } else { "S" }),
                    ],
                    indexes: vec![],
                },
                SchemaTable {
                    name: "Const12".into(),
                    number: 12,
                    owner: None,
                    inline_name: None,
                    columns: vec![column("Fld30", "L")],
                    indexes: vec![],
                },
            ],
            anomalies: vec![],
        },
        vec![
            LiveTable {
                name: "_reference11".into(),
                columns: vec![
                    LiveColumn {
                        name: "_description".into(),
                        data_type: "mvarchar(25)".into(),
                    },
                    LiveColumn {
                        name: "_fld20".into(),
                        data_type: if numeric {
                            "numeric(10,0)"
                        } else {
                            "mvarchar(10)"
                        }
                        .into(),
                    },
                ],
                indexes: vec![],
            },
            LiveTable {
                name: "_const12".into(),
                columns: vec![LiveColumn {
                    name: "_fld30".into(),
                    data_type: "boolean".into(),
                }],
                indexes: vec![],
            },
        ],
    )
    .snapshot;
    assert_eq!(metadata.separators().count(), 1);
    let catalog = crate::document_attributes::project(
        MetadataCatalog::new("conditional-session-control", objects).unwrap(),
        &metadata,
    )
    .unwrap();
    let rows = if numeric {
        "('empty',0),('A',1),('B',2)"
    } else {
        "('empty',''),('A','A'),('B','B')"
    };
    fixture_sql(
        &port,
        &format!(
            "DROP TABLE IF EXISTS _reference11; DROP TABLE IF EXISTS _const12; CREATE TABLE _reference11 (_description varchar(25), _fld20 {}); INSERT INTO _reference11 VALUES {rows}; CREATE TABLE _const12 (_fld30 boolean); INSERT INTO _const12 VALUES (FALSE);",
            if numeric {
                "numeric(10,0)"
            } else {
                "varchar(10)"
            }
        ),
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
    let connection = Rc::new(RefCell::new(
        BlockingDatabaseSession::connect(
            &target,
            &Credentials::new(None, None, None),
            Limits::default(),
        )
        .unwrap(),
    ));
    let adapter = Rc::new(DatabaseDataAccess::new(
        Rc::clone(&connection),
        metadata,
        access,
        &principal,
    ));
    let source = format!(
        r#"
            Процедура УстановкаПараметровСеанса(Имена)
                Если Имена = Неопределено Тогда ПараметрыСеанса.{COUNTER} = 0; Возврат; КонецЕсли;
                ПараметрыСеанса.{COUNTER} = ПараметрыСеанса.{COUNTER} + 1;
                Для Каждого Имя Из Имена Цикл
                    Если Имя = "{use_name}" Тогда ПараметрыСеанса.{use_name} = Истина;
                    ИначеЕсли Имя = "{value_name}" Тогда ПараметрыСеанса.{value_name} = {};
                    Иначе ВызватьИсключение "Неожиданный параметр сеанса"; КонецЕсли;
                КонецЦикла;
            КонецПроцедуры
        "#,
        if numeric { "2" } else { "\"B\"" }
    );
    let code = ConfigurationCodeSnapshot::new(
        "conditional-session-control",
        vec![ConfigurationModule::new(
            "session",
            "МодульСеанса",
            source,
            ModuleKind::Session,
            ModuleExecution::new().with_server(true),
        )],
    )
    .unwrap();
    let factory = SessionFactory::new(
        code,
        Arc::new(catalog),
        "МодульСеанса",
        SessionKind::ThinClient,
        "8.3.27.2342",
    )
    .unwrap();
    let mut session = factory
        .open(
            principal,
            SessionResources::new(Box::new(Rc::clone(&connection)), 1).with_data_access(adapter),
        )
        .unwrap();
    let probe = session.instantiate(r#"
            Функция КоличествоПробы() Экспорт
                ВыборкаПробы = Новый Запрос("ВЫБРАТЬ КОЛИЧЕСТВО(*) КАК Количество ИЗ Справочник.ЗамерВременныхТаблиц").Выполнить().Выбрать();
                ВыборкаПробы.Следующий(); Возврат ВыборкаПробы.Количество;
            КонецФункции
            Функция ИменаПробы() Экспорт
                ВыборкаПробы = Новый Запрос("ВЫБРАТЬ Наименование КАК Имя ИЗ Справочник.ЗамерВременныхТаблиц УПОРЯДОЧИТЬ ПО Имя").Выполнить().Выбрать();
                ИменаСтрок = ""; Пока ВыборкаПробы.Следующий() Цикл ИменаСтрок = ИменаСтрок + ВыборкаПробы.Имя + ";"; КонецЦикла; Возврат ИменаСтрок;
            КонецФункции
        "#).unwrap();
    // Сброс — инструментирование Rust-стенда, не замер платформенного Clear.
    session.parameters().clear();
    session
        .parameters()
        .set(COUNTER, Value::number_from_i64(0))
        .unwrap();
    let count = |session: &mut bsl_session::Session| {
        session.call(&probe, "КоличествоПробы", vec![]).map(|r| r.0)
    };
    let names = |session: &mut bsl_session::Session| {
        session.call(&probe, "ИменаПробы", vec![]).map(|r| r.0)
    };
    assert_eq!(
        count(&mut session).unwrap(),
        native_value(numeric, shared, "conditional.initial_empty_count")
    );
    assert_eq!(
        names(&mut session).unwrap(),
        native_value(numeric, shared, "conditional.disabled_c_names")
    );
    assert_eq!(session.parameters().peek(&use_name).unwrap(), None);
    assert_eq!(session.parameters().peek(&value_name).unwrap(), None);
    assert_eq!(
        session.parameters().peek(COUNTER).unwrap(),
        Some(Value::number_from_i64(0))
    );
    // Изменение fixture-строки моделирует уже зафиксированное значение;
    // поддержка BSL-записи констант и нативного межсеансового кэша не заявляется.
    fixture_sql(&port, "UPDATE _const12 SET _fld30 = TRUE;");
    // Нативный Shared-контроль задаёт значение явно. Протокол запроса
    // отсутствующего Shared-значения исследуется отдельно; он не измерен
    // этим скриптом и не заменяет проверку известного значения области.
    if shared {
        session
            .parameters()
            .set(
                &value_name,
                if numeric {
                    Value::number_from_i64(2)
                } else {
                    Value::Str("B".into())
                },
            )
            .unwrap();
    }
    assert_eq!(count(&mut session).unwrap(), Value::number_from_i64(1));
    assert_eq!(
        names(&mut session).unwrap(),
        native_value(numeric, shared, "conditional.enabled_b_names")
    );
    assert_eq!(
        session.parameters().peek(COUNTER).unwrap(),
        Some(Value::number_from_i64(if shared { 1 } else { 2 }))
    );
    session
        .parameters()
        .set(
            &value_name,
            if numeric {
                Value::number_from_i64(2)
            } else {
                Value::Str("B".into())
            },
        )
        .unwrap();
    assert_eq!(
        names(&mut session).unwrap(),
        native_value(numeric, shared, "conditional.enabled_b_names")
    );
    fixture_sql(&port, "UPDATE _const12 SET _fld30 = FALSE;");
    session
        .parameters()
        .set(
            &value_name,
            if numeric {
                Value::number_from_i64(3)
            } else {
                Value::Str("C".into())
            },
        )
        .unwrap();
    assert_eq!(count(&mut session).unwrap(), Value::number_from_i64(1));
    assert_eq!(
        names(&mut session).unwrap(),
        native_value(numeric, shared, "conditional.disabled_c_names")
    );
    session.parameters().clear();
    session
        .parameters()
        .set(COUNTER, Value::number_from_i64(0))
        .unwrap();
    session
        .parameters()
        .set(&use_name, Value::Boolean(false))
        .unwrap();
    fixture_sql(&port, "UPDATE _const12 SET _fld30 = TRUE;");
    if shared {
        assert_eq!(count(&mut session).unwrap(), Value::number_from_i64(3));
        assert_eq!(
            names(&mut session).unwrap(),
            Value::Str("A;B;empty;".into())
        );
    } else {
        let failure = count(&mut session).unwrap_err().to_string();
        assert!(
            failure.contains("independent usage is disabled"),
            "{failure}"
        );
    }
    assert_eq!(session.parameters().peek(&value_name).unwrap(), None);
    assert_eq!(
        session.parameters().peek(COUNTER).unwrap(),
        Some(Value::number_from_i64(0))
    );
}

#[test]
fn string_independent_condition_matches_the_native_areas_on_postgres() {
    check_conditional_profile(false, false);
}

#[test]
fn string_shared_condition_matches_the_native_areas_on_postgres() {
    check_conditional_profile(false, true);
}

#[test]
fn number_independent_condition_matches_the_native_areas_on_postgres() {
    check_conditional_profile(true, false);
}

#[test]
fn number_shared_condition_matches_the_native_areas_on_postgres() {
    check_conditional_profile(true, true);
}
