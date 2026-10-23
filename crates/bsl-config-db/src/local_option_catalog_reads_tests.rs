//! Параметризованные опции справочника на собственном PostgreSQL.

use super::local_option_register_reads_tests::{check_native_result, fixture_sql};
use super::*;
use open_sdbl::metadata::{
    DbNameEntry, DbNames, InfoBaseUser, LiveColumn, LiveTable, SchemaColumn, SchemaStorage,
    SchemaTable, UserRow, parse_role_rights, resolve_metadata,
};
use open_sdbl_db::{
    ConnectionOptions, Credentials, DatabaseConnection, Limits, PostgresConnection, PostgresSslMode,
};
use std::sync::Arc;

#[test]
fn catalog_option_reference_parameters_match_native_results_on_postgres() {
    let Ok(port) = std::env::var("OPEN_SDBL_TRANSACTION_CONTROL_PORT") else {
        // Без отдельного стенда обычный workspace-прогон не подтверждает SQL.
        return;
    };
    let mut state = catalog_state(&port, false, false, false, false, false);
    let oracle = include_str!(
        "../../../tests/conformance/measure/1c/measure-functional-option-catalog.platform.txt"
    );
    for (id, option) in [
        ("BOOL_EMPTY", "ОпцияСправочника"),
        ("NUMBER_EMPTY", "ЧисловаяОпцияСправочника"),
    ] {
        check_native_result(&mut state, oracle, "CATALOG", id, option, "Новый Структура");
    }
    let ids = [
        "00112233-4455-6677-8899-aabbccddeeff",
        "10213243-5465-7687-98a9-bacbdcedfe0f",
        "10213243-5465-7687-98a9-bacbdcedfe10",
    ];
    let rows = ids
        .iter()
        .zip(["TRUE,FALSE,10", "TRUE,TRUE,20", "FALSE,FALSE,0"])
        .map(|(id, values)| {
            let guid: Guid = id.parse().unwrap();
            let hex = guid
                .to_1c_bytes()
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>();
            format!("(decode('{hex}','hex'),{values})")
        })
        .collect::<Vec<_>>();
    fixture_sql(
        &port,
        &format!("INSERT INTO _reference7 VALUES {};", rows.join(",")),
    );
    let parameter = |name: &str, id: &str| {
        format!(
            "Новый Структура(\"{name}\", Справочники.ЗамерВременныхТаблиц.ПолучитьСсылку(Новый УникальныйИдентификатор(\"{id}\")))"
        )
    };
    for (id, option, parameters) in [
        (
            "BOOL_FALSE",
            "ОпцияСправочника",
            parameter("ОбъектОпции", ids[0]),
        ),
        (
            "BOOL_TRUE",
            "ОпцияСправочника",
            parameter("ОбъектОпции", ids[1]),
        ),
        (
            "NUMBER",
            "ЧисловаяОпцияСправочника",
            parameter("ОбъектОпции", ids[1]),
        ),
        ("BOOL_PARTIAL", "ОпцияСправочника", "Новый Структура".into()),
        (
            "NUMBER_PARTIAL",
            "ЧисловаяОпцияСправочника",
            "Новый Структура".into(),
        ),
        (
            "EMPTY_REF",
            "ОпцияСправочника",
            "Новый Структура(\"ОбъектОпции\", Справочники.ЗамерВременныхТаблиц.ПустаяСсылка())"
                .into(),
        ),
        (
            "NUMBER_EMPTY_REF",
            "ЧисловаяОпцияСправочника",
            "Новый Структура(\"ОбъектОпции\", Справочники.ЗамерВременныхТаблиц.ПустаяСсылка())"
                .into(),
        ),
        (
            "GROUP",
            "ОпцияСправочника",
            parameter("ОбъектОпции", ids[2]),
        ),
        (
            "UNKNOWN_PARAMETER",
            "ОпцияСправочника",
            parameter("НесуществующийПараметр", ids[0]),
        ),
    ] {
        check_native_result(&mut state, oracle, "CATALOG", id, option, &parameters);
    }
}

// Общая подготовка настоящего SDK-соединения на отдельном стенде PostgreSQL.
fn catalog_state(
    port: &str,
    allow_read: bool,
    selection_fields: bool,
    hierarchical: bool,
    string_lookup: bool,
    typed_lookup: bool,
) -> open_bsl::State {
    let (catalog, native) =
        crate::functional_options::tests::native("catalog-native", |descriptors| {
            if typed_lookup {
                let owner = descriptors
                    .iter()
                    .find(|d| d.name == "ЗамерВременныхТаблиц")
                    .unwrap();
                let owner_id = owner.object_guid.clone();
                let reference_type = owner.object_reference_type.clone();
                let template = descriptors
                    .iter()
                    .find(|d| d.name == "ОпцияБулево")
                    .unwrap()
                    .clone();
                descriptors
                    .iter_mut()
                    .find(|d| d.name == "ОпцияБулево")
                    .unwrap()
                    .name = "БулевоПробы".into();
                for (id, name, tag) in [
                    ("11ee17ab-6a5a-5b39-a73b-3bdac8abdb60", "ДатаПробы", "D"),
                    ("6358221f-fd6e-57f4-a537-4b35928d115d", "СсылкаПробы", "#"),
                ] {
                    let mut field = template.clone();
                    field.object_guid = id.parse().unwrap();
                    field.name = name.into();
                    let kind = &mut field.declared_types.as_mut().unwrap()[0];
                    kind.tag = tag.into();
                    kind.reference = if tag == "#" {
                        reference_type.clone()
                    } else {
                        None
                    };
                    kind.qualifiers.clear();
                    descriptors
                        .iter_mut()
                        .find(|d| d.object_guid == owner_id)
                        .unwrap()
                        .catalog_attributes
                        .as_mut()
                        .unwrap()
                        .push(field.object_guid.clone());
                    descriptors.push(field);
                }
            }
            if string_lookup {
                let owner = descriptors
                    .iter_mut()
                    .find(|d| d.name == "ЗамерВременныхТаблиц")
                    .unwrap();
                // XML этого замера явно задаёт отсутствие иерархии и владельцев.
                owner.catalog_hierarchical = Some(hierarchical);
                owner.catalog_owners = Some(vec![]);
                let field = descriptors
                    .iter_mut()
                    .find(|d| d.name == "ОпцияБулево")
                    .unwrap();
                // Тип и имя тестового профиля повторяют XML нативного замера.
                field.name = "ИскомоеЗначение".into();
                field.declared_types.as_mut().unwrap()[0].tag = "S".into();
            }
            if hierarchical {
                // Синтетический профиль SQL повторяет измеренный флаг XML замера.
                let owner = descriptors
                    .iter_mut()
                    .find(|d| d.name == "ЗамерВременныхТаблиц")
                    .unwrap();
                owner.catalog_hierarchical = Some(true);
            }
        });
    let owner = native
        .descriptors()
        .iter()
        .find(|d| d.name == "ЗамерВременныхТаблиц")
        .unwrap();
    let mut names = vec![DbNameEntry {
        guid: owner.object_guid.clone(),
        alias: "Reference".into(),
        number: 7,
    }];
    let mut columns = vec![
        SchemaColumn {
            name: "ID".into(),
            types: vec![open_sdbl::metadata::ColumnType {
                tag: "R".into(),
                reference_target: Some("Reference7".into()),
            }],
        },
        SchemaColumn {
            name: "Folder".into(),
            types: vec![open_sdbl::metadata::ColumnType {
                tag: "L".into(),
                reference_target: None,
            }],
        },
    ];
    let mut live = vec![
        LiveColumn {
            name: "_idrref".into(),
            data_type: "bytea".into(),
        },
        LiveColumn {
            name: "_folder".into(),
            data_type: "boolean".into(),
        },
    ];
    let mut ddl = vec!["_idrref bytea".to_string(), "_folder boolean".into()];
    if selection_fields {
        for (name, physical, tag, sql_type) in [
            ("Code", "_code", "S", "varchar(9)"),
            ("Description", "_description", "S", "varchar(25)"),
            ("Marked", "_marked", "L", "boolean"),
        ] {
            columns.push(SchemaColumn {
                name: name.into(),
                types: vec![open_sdbl::metadata::ColumnType {
                    tag: tag.into(),
                    reference_target: None,
                }],
            });
            live.push(LiveColumn {
                name: physical.into(),
                data_type: sql_type.into(),
            });
            ddl.push(format!("{physical} {sql_type}"));
        }
    }
    if hierarchical {
        columns.push(SchemaColumn {
            name: "ParentID".into(),
            types: vec![open_sdbl::metadata::ColumnType {
                tag: "R".into(),
                reference_target: Some("Reference7".into()),
            }],
        });
        live.push(LiveColumn {
            name: "_parentidrref".into(),
            data_type: "bytea".into(),
        });
        ddl.push("_parentidrref bytea".into());
    }
    let mut field_names = Vec::new();
    for (index, id) in owner
        .catalog_attributes
        .as_ref()
        .unwrap()
        .iter()
        .enumerate()
    {
        let field = native
            .descriptors()
            .iter()
            .find(|d| &d.object_guid == id)
            .unwrap();
        field_names.push(field.name.as_str());
        let [kind] = field.declared_types.as_deref().unwrap() else {
            panic!("один тип поля")
        };
        let (tag, sql_type) = match kind.tag.as_str() {
            "B" => ("L", "boolean"),
            "N" => ("N", "numeric(15,2)"),
            "S" => ("S", "varchar(20)"),
            "D" => ("T", "timestamp without time zone"),
            "#" => ("R", "bytea"),
            other => panic!("неизмеренный тип {other}"),
        };
        let number = 70 + index as u32;
        names.push(DbNameEntry {
            guid: id.clone(),
            alias: "Fld".into(),
            number,
        });
        columns.push(SchemaColumn {
            name: format!("Fld{number}"),
            types: vec![open_sdbl::metadata::ColumnType {
                tag: tag.into(),
                reference_target: (tag == "R").then(|| "Reference7".into()),
            }],
        });
        let physical = if tag == "R" {
            format!("_fld{number}rref")
        } else {
            format!("_fld{number}")
        };
        live.push(LiveColumn {
            name: physical.clone(),
            data_type: sql_type.into(),
        });
        ddl.push(format!("{physical} {sql_type}"));
    }
    if typed_lookup {
        assert_eq!(
            field_names,
            ["БулевоПробы", "ЧислоПробы", "ДатаПробы", "СсылкаПробы"]
        );
    } else {
        assert_eq!(
            field_names,
            [
                if string_lookup {
                    "ИскомоеЗначение"
                } else {
                    "ОпцияБулево"
                },
                "ЧислоПробы"
            ]
        );
    }
    // Схема и UUID строк заданы стендом; описатели и привязки взяты из Config.
    let metadata = resolve_metadata(
        DbNames::from_entries(names),
        native.descriptors().to_vec(),
        SchemaStorage {
            tables: vec![SchemaTable {
                name: "Reference7".into(),
                number: 7,
                owner: None,
                inline_name: None,
                columns,
                indexes: vec![],
            }],
            anomalies: vec![],
        },
        vec![LiveTable {
            name: "_reference7".into(),
            columns: live,
            indexes: vec![],
        }],
    )
    .snapshot;
    let objects = catalog
        .objects()
        .iter()
        .cloned()
        .map(|object| {
            let descriptor = native
                .descriptors()
                .iter()
                .find(|descriptor| descriptor.object_guid.as_str() == object.id())
                .unwrap();
            crate::project_catalog_descriptor_fields(object, descriptor, native.descriptors())
                .unwrap()
        })
        .collect();
    let catalog = bsl_orm::MetadataCatalog::new(catalog.identity(), objects).unwrap();
    let catalog = crate::functional_options::project(catalog, &metadata).unwrap();
    fixture_sql(
        port,
        &format!(
            "DROP TABLE IF EXISTS _reference7; CREATE TABLE _reference7 ({});",
            ddl.join(", ")
        ),
    );
    let role_id = "ae8541a3-7d32-419d-be2a-71ff2cce2566";
    let user_id = "11111111-1111-1111-1111-111111111111";
    let record = format!("{{{user_id},\"FunctionalDenied\",\"\",{{1,{role_id}}}}}");
    let mut encoded = vec![1, 0x5a];
    encoded.extend(record.bytes().map(|b| b ^ 0x5a));
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
    let mut rights = parse_role_rights(include_bytes!("../tests/fixtures/functional-options/source-rights-native/ae8541a3-7d32-419d-be2a-71ff2cce2566.rights.deflate")).unwrap();
    if allow_read {
        // Положительный SQL-контроль использует явно синтетическую полную роль.
        // Это не свидетельство декодирования прав рабочего пользователя.
        rights.objects.clear();
        rights.set_for_new_objects = true;
        rights.set_for_attributes_by_default = true;
    }
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
    let catalog = Arc::new(catalog);
    let mut builder = open_bsl::Engine::builder().register_library(bsl_orm::library());
    for ty in catalog.transferable_value_types() {
        builder = builder.register_value_type(ty);
    }
    for ty in catalog.catalog_selection_types() {
        builder = builder.register_value_type(Arc::clone(ty));
    }
    let engine = builder.build().unwrap();
    let data = SessionData::new(catalog, principal).with_data_access(adapter);
    engine.state_builder().host_data(Rc::new(data)).build()
}

#[test]
fn catalog_attribute_lookup_executes_on_postgres_with_explicit_read_rights() {
    let Ok(port) = std::env::var("OPEN_SDBL_TRANSACTION_CONTROL_PORT") else {
        return;
    };
    let mut state = catalog_state(&port, true, false, false, false, false);
    fixture_sql(
        &port,
        "INSERT INTO _reference7 VALUES (decode('00112233445566778899aabbccddeeff','hex'), FALSE, TRUE, 17);",
    );
    assert_eq!(
        state
            .exec(
                r#"
        Менеджер = Справочники.ЗамерВременныхТаблиц;
        Найдено = Менеджер.НайтиПоРеквизиту("ЧислоПробы", 17);
        Если Найдено.Пустая() Тогда ВызватьИсключение "positive SQL match"; КонецЕсли;
        Если Менеджер.FindByAttribute("ЧислоПробы", 18) <> Менеджер.ПустаяСсылка() Тогда
            ВызватьИсключение "missing match";
        КонецЕсли;
        Возврат Менеджер.НайтиПоРеквизиту("ОпцияБулево", Истина) = Найдено;
    "#
            )
            .unwrap(),
        Value::Boolean(true)
    );
}

#[test]
fn catalog_selection_replays_populated_native_cursor_on_postgres() {
    let Ok(port) = std::env::var("OPEN_SDBL_TRANSACTION_CONTROL_PORT") else {
        return;
    };
    let mut state = catalog_state(&port, true, true, false, false, false);
    fixture_sql(
        &port,
        "INSERT INTO _reference7 (_idrref, _folder, _code, _description, _marked, _fld70, _fld71) VALUES (decode('00112233445566778899aabbccddeeff','hex'), FALSE, '002', 'Alpha', FALSE, TRUE, 17), (decode('102132435465768798a9bacbdcedfe0f','hex'), FALSE, '001', 'Zulu', TRUE, FALSE, 18);",
    );
    let source = include_str!(
        "../../../tests/conformance/measure/1c/measure-catalog-selection-populated.bsl"
    );
    // Записи подготовлены SQL стенда; исполняется неизменный блок нативного контроля.
    let control = source
        .split("// Начало проверяемого контракта выборки.")
        .nth(1)
        .unwrap()
        .replace("Сообщить(", "СтрокиПробы.Добавить(");
    let expected = include_str!(
        "../../../tests/conformance/measure/1c/measure-catalog-selection-populated.platform.txt"
    );
    assert_native_lines(&mut state, &control, expected);
}

#[test]
fn catalog_manager_reads_preserve_denied_principal_errors() {
    let Ok(port) = std::env::var("OPEN_SDBL_TRANSACTION_CONTROL_PORT") else {
        return;
    };
    let mut state = catalog_state(&port, false, true, false, false, false);
    for operation in [
        "Справочники.ЗамерВременныхТаблиц.НайтиПоРеквизиту(\"ЧислоПробы\", 17)",
        "Справочники.ЗамерВременныхТаблиц.Select()",
    ] {
        let error = state.exec(&format!("Возврат {operation};")).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("is denied for user \"FunctionalDenied\""),
            "{error}"
        );
    }
}

fn assert_native_lines(state: &mut open_bsl::State, control: &str, expected: &str) {
    let captured = state
        .exec(&format!(
            "СтрокиПробы = Новый Массив;\n{control}\nВозврат СтрокиПробы;"
        ))
        .unwrap();
    let Value::Object(object) = captured else {
        panic!("массив строк")
    };
    let BslObject::Array(lines) = object.as_ref() else {
        panic!("массив строк")
    };
    let actual = lines
        .borrow()
        .iter()
        .map(|line| {
            let Value::Str(line) = line else {
                panic!("строка контроля")
            };
            line.to_string()
        })
        .collect::<Vec<_>>();
    assert_eq!(actual, expected.lines().collect::<Vec<_>>());
}

#[test]
fn catalog_selection_parent_predicates_replay_native_control_on_postgres() {
    let Ok(port) = std::env::var("OPEN_SDBL_TRANSACTION_CONTROL_PORT") else {
        return;
    };
    let mut state = catalog_state(&port, true, true, true, false, false);
    let group: Guid = "12345678-1234-4321-abcd-0123456789ab".parse().unwrap();
    let group_hex = group
        .to_1c_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    fixture_sql(
        &port,
        &format!(
            "INSERT INTO _reference7 (_idrref, _folder, _code, _description, _marked, _parentidrref) VALUES (decode('{group_hex}','hex'), TRUE, '010', 'Group', FALSE, decode(repeat('00',16),'hex')), (decode('00112233445566778899aabbccddeeff','hex'), FALSE, '001', 'Root', FALSE, decode(repeat('00',16),'hex')), (decode('102132435465768798a9bacbdcedfe0f','hex'), FALSE, '002', 'Child', FALSE, decode('{group_hex}','hex'));"
        ),
    );
    state.exec("ПробаРодителя = Справочники.ЗамерВременныхТаблиц.Выбрать(Справочники.ЗамерВременныхТаблиц.ПустаяСсылка());").unwrap();
    let source =
        include_str!("../../../tests/conformance/measure/1c/measure-catalog-selection-parent.bsl");
    let control = source
        .split("// Начало проверяемого контракта выборки.")
        .nth(1)
        .unwrap()
        .replace("Сообщить(", "СтрокиПробы.Добавить(");
    assert_native_lines(
        &mut state,
        &control,
        include_str!(
            "../../../tests/conformance/measure/1c/measure-catalog-selection-parent.platform.txt"
        ),
    );
}

#[test]
fn catalog_attribute_parent_predicates_execute_on_postgres() {
    let Ok(port) = std::env::var("OPEN_SDBL_TRANSACTION_CONTROL_PORT") else {
        return;
    };
    let mut state = catalog_state(&port, true, true, true, false, false);
    let group: Guid = "12345678-1234-4321-abcd-0123456789ab".parse().unwrap();
    let group_hex = group
        .to_1c_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    fixture_sql(
        &port,
        &format!(
            "INSERT INTO _reference7 (_idrref, _folder, _code, _description, _marked, _parentidrref, _fld71) VALUES (decode('102132435465768798a9bacbdcedfe0f','hex'), FALSE, '001', 'Child', FALSE, decode('{group_hex}','hex'), 17);"
        ),
    );
    let source = include_str!(
        "../../../tests/conformance/measure/1c/measure-catalog-find-attribute-parent.bsl"
    );
    // Стенд использует измеренный числовой реквизит; родительские вызовы сохранены.
    // Это SQL-контроль композиции двух измеренных профилей, а не новый замер 1С.
    let control = source
        .split("// Начало проверяемого контракта поиска.")
        .nth(1)
        .unwrap()
        .replace(
            "\"\"ИскомоеЗначение\"\", \"\"AbC\"\"",
            "\"\"ЧислоПробы\"\", 17",
        )
        .replace("Сообщить(", "СтрокиПробы.Добавить(");
    assert_native_lines(
        &mut state,
        &control,
        include_str!(
            "../../../tests/conformance/measure/1c/measure-catalog-find-attribute-parent.platform.txt"
        ),
    );
}

#[test]
fn catalog_string_attribute_lookup_replays_native_control_on_postgres() {
    let Ok(port) = std::env::var("OPEN_SDBL_TRANSACTION_CONTROL_PORT") else {
        return;
    };
    let mut state = catalog_state(&port, true, true, false, true, false);
    let reference: Guid = "12345678-1234-4321-abcd-0123456789ab".parse().unwrap();
    let hex = reference
        .to_1c_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    fixture_sql(
        &port,
        &format!(
            "INSERT INTO _reference7 (_idrref, _folder, _code, _description, _marked, _fld70) VALUES (decode('{hex}','hex'), FALSE, '001', 'Alpha', FALSE, 'AbC'), (decode('00112233445566778899aabbccddeeff','hex'), FALSE, '002', 'NumericString', FALSE, '17'), (decode('102132435465768798a9bacbdcedfe0f','hex'), FALSE, '003', 'EmptyString', FALSE, '');"
        ),
    );
    let source =
        include_str!("../../../tests/conformance/measure/1c/measure-catalog-find-attribute.bsl");
    let control = source
        .split("// Начало проверяемого контракта поиска.")
        .nth(1)
        .unwrap()
        .replace("Сообщить(", "СтрокиПробы.Добавить(");
    assert_native_lines(
        &mut state,
        &control,
        include_str!(
            "../../../tests/conformance/measure/1c/measure-catalog-find-attribute.platform.txt"
        ),
    );
}

#[test]
fn catalog_typed_attribute_lookup_replays_native_control_on_postgres() {
    let Ok(port) = std::env::var("OPEN_SDBL_TRANSACTION_CONTROL_PORT") else {
        return;
    };
    let mut state = catalog_state(&port, true, true, false, false, true);
    let reference: Guid = "12345678-1234-4321-abcd-0123456789ab".parse().unwrap();
    let hex = reference
        .to_1c_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    fixture_sql(
        &port,
        &format!(
            "INSERT INTO _reference7 (_idrref, _folder, _code, _description, _marked, _fld70, _fld71, _fld72, _fld73rref) VALUES (decode('{hex}','hex'), FALSE, '001', 'Types', FALSE, TRUE, 17, TIMESTAMP '2026-01-02', decode('{hex}','hex'));"
        ),
    );
    state.exec("Возврат Справочники.ЗамерВременныхТаблиц.НайтиПоРеквизиту(\"ДатаПробы\", Дата(2026,1,2));").unwrap();
    let source = include_str!(
        "../../../tests/conformance/measure/1c/measure-catalog-find-attribute-types.bsl"
    );
    let control = source
        .split("// Начало проверяемого контракта поиска.")
        .nth(1)
        .unwrap()
        .replace("Сообщить(", "СтрокиПробы.Добавить(");
    let setup = "СсылкаЗаписиПробы = Справочники.ЗамерВременныхТаблиц.ПолучитьСсылку(Новый УникальныйИдентификатор(\"12345678-1234-4321-abcd-0123456789ab\"));";
    assert_native_lines(
        &mut state,
        &format!("{setup}\n{control}"),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-catalog-find-attribute-types.platform.txt"
        ),
    );
}
