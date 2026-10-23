use super::*;
use open_sdbl::metadata::{DbNames, ObjectId, SchemaStorage, resolve_metadata};
use open_sdbl::query::{PostgresBackend, RestrictionTarget};

#[test]
fn privileged_option_read_is_confined_to_its_single_constant() {
    let constant = ObjectId::from_bytes([1; 16]);
    let other = ObjectId::from_bytes([2; 16]);
    let target = RestrictionTarget {
        object: constant,
        table_part: None,
    };
    let request = open_sdbl::query::RestrictionRequest {
        targets: vec![target.clone()],
    };
    assert_eq!(
        privileged_option_decisions(&request, constant).unwrap(),
        vec![AccessDecision::unrestricted(target)]
    );
    for targets in [
        vec![],
        vec![RestrictionTarget {
            object: other,
            table_part: None,
        }],
        vec![RestrictionTarget {
            object: constant,
            table_part: Some("Часть".into()),
        }],
        vec![
            RestrictionTarget {
                object: constant,
                table_part: None,
            },
            RestrictionTarget {
                object: other,
                table_part: None,
            },
        ],
    ] {
        assert!(
            privileged_option_decisions(
                &open_sdbl::query::RestrictionRequest { targets },
                constant
            )
            .is_err()
        );
    }
}

#[test]
fn the_native_constant_source_compiles_with_a_confined_privileged_decision() {
    let id = "9893e2d6-f3f8-4d73-bb06-19bf26d216ab";
    let descriptors = open_sdbl::metadata::parse_config_descriptors(
        id,
        include_bytes!("../tests/fixtures/functional-options/native/9893e2d6-f3f8-4d73-bb06-19bf26d216ab.deflate"),
    ).unwrap();
    let name = descriptors
        .iter()
        .find(|descriptor| descriptor.object_guid.as_str() == id)
        .unwrap()
        .name
        .clone();
    let snapshot = resolve_metadata(
        DbNames::from_entries(vec![
            open_sdbl::metadata::DbNameEntry {
                guid: id.parse().unwrap(),
                alias: "Const".into(),
                number: 1,
            },
            open_sdbl::metadata::DbNameEntry {
                guid: id.parse().unwrap(),
                alias: "Fld".into(),
                number: 2,
            },
        ]),
        descriptors,
        SchemaStorage {
            // Физическая схема здесь задана тестом; нативный Config даёт имя и тип константы.
            tables: vec![open_sdbl::metadata::SchemaTable {
                name: "Const1".into(),
                number: 1,
                owner: None,
                inline_name: None,
                columns: vec![open_sdbl::metadata::SchemaColumn {
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
        vec![open_sdbl::metadata::LiveTable {
            name: "_const1".into(),
            columns: vec![open_sdbl::metadata::LiveColumn {
                name: "_fld2".into(),
                data_type: "boolean".into(),
            }],
            indexes: vec![],
        }],
    )
    .snapshot;
    let object = snapshot
        .object_id(SdblMetadataKind::Constant, &name)
        .unwrap();
    let compiler = QueryCompiler::new(&snapshot, PostgresBackend);
    let source = format!("ВЫБРАТЬ ПЕРВЫЕ 1 К.Значение КАК Значение ИЗ Константа.{name} КАК К");
    let prepared = compiler
        .prepare_with_options(&source, &PrepareOptions::new().restricted())
        .unwrap();
    let decisions = privileged_option_decisions(prepared.restriction_request(), object).unwrap();
    let compiled = prepared
        .compile_batch(
            &snapshot,
            &CompileOptions::new().decisions(&decisions),
            &mut TempTablesManager::new(),
        )
        .unwrap()
        .unwrap();
    assert_eq!(compiled.columns.len(), 1);
    assert_eq!(compiled.columns[0].kind, ColumnKind::Boolean);
    assert!(compiled.sql.contains("_const1"));
    let denied = [AccessDecision::denied(
        prepared.restriction_request().targets[0].clone(),
    )];
    assert!(require_allowed_for_restrictions(&source, &denied).is_err());
    assert!(require_allowed_for_restrictions(&source, &decisions).is_ok());
    assert!(require_allowed_for_restrictions(&source, &denied).is_err());
}

#[test]
fn balance_date_parameters_preserve_the_measured_second_boundaries() {
    for (digits, hour, minute, second) in [
        ("20261006000000", 0, 0, 0),
        ("20261006115959", 11, 59, 59),
        ("20261006120000", 12, 0, 0),
        ("20261006120001", 12, 0, 1),
    ] {
        let value = Value::Date(BslDate::parse_digits(digits).unwrap());
        assert_eq!(
            parameter_value(&value).unwrap(),
            ParameterValue::Date(ParameterDate::new(2026, 10, 6, hour, minute, second).unwrap()),
            "{digits}"
        );
    }
}

#[test]
fn undefined_lookup_comparison_does_not_become_a_null_parameter() {
    let snapshot = resolve_metadata(
        DbNames::from_entries(Vec::new()),
        Vec::new(),
        SchemaStorage {
            tables: Vec::new(),
            anomalies: Vec::new(),
        },
        Vec::new(),
    )
    .snapshot;
    let compiler = QueryCompiler::new(&snapshot, PostgresBackend);
    for value in ["\"\"", "\"17\"", "17", "ИСТИНА"] {
        let query = compiler
            .compile(&format!(
                "ВЫБРАТЬ 1 КАК Результат ГДЕ {value} = НЕОПРЕДЕЛЕНО"
            ))
            .unwrap();
        assert!(!query.sql.to_ascii_uppercase().contains("IS NULL"));
        assert!(!query.sql.contains("$1"));
    }
}

#[test]
fn the_compiler_preserves_one_managers_temporary_tables_across_queries() {
    let snapshot = resolve_metadata(
        DbNames::from_entries(Vec::new()),
        Vec::new(),
        SchemaStorage {
            tables: Vec::new(),
            anomalies: Vec::new(),
        },
        Vec::new(),
    )
    .snapshot;
    let compiler = QueryCompiler::new(&snapshot, PostgresBackend);
    let mut manager = TempTablesManager::new();
    let place = compiler
        .prepare_with_options(
            "ВЫБРАТЬ 1 КАК Значение ПОМЕСТИТЬ ВТ",
            &PrepareOptions::new()
                .restricted()
                .temporary_tables(&manager),
        )
        .unwrap()
        .compile_batch(&snapshot, &CompileOptions::new(), &mut manager)
        .unwrap()
        .unwrap();
    assert_eq!(place.columns[0].label, "Количество");
    assert!(manager.contains("ВТ"));
    assert_ne!(manager, TempTablesManager::new());

    let read = compiler
        .prepare_with_options(
            "ВЫБРАТЬ Значение ИЗ ВТ",
            &PrepareOptions::new()
                .restricted()
                .temporary_tables(&manager),
        )
        .unwrap()
        .compile_batch(&snapshot, &CompileOptions::new(), &mut manager)
        .unwrap()
        .unwrap();
    assert_eq!(read.columns[0].label, "Значение");
    assert!(read.sql.contains("WITH"));

    let append = compiler
        .prepare_with_options(
            "ВЫБРАТЬ 2 КАК Значение ДОБАВИТЬ ВТ",
            &PrepareOptions::new()
                .restricted()
                .temporary_tables(&manager),
        )
        .unwrap()
        .compile_batch(&snapshot, &CompileOptions::new(), &mut manager)
        .unwrap()
        .unwrap();
    assert_eq!(append.columns[0].label, "Количество");
    assert!(manager.contains("ВТ"));

    let ordinary = compiler.compile("ВЫБРАТЬ 1 КАК Количество").unwrap();
    assert_eq!(ordinary.columns[0].label, "Количество");

    let drop_result = compiler
        .prepare_with_options(
            "УНИЧТОЖИТЬ ВТ",
            &PrepareOptions::new()
                .restricted()
                .temporary_tables(&manager),
        )
        .unwrap()
        .compile_batch(&snapshot, &CompileOptions::new(), &mut manager)
        .unwrap();
    assert!(drop_result.is_none());
    assert!(manager.is_empty());
    assert_ne!(manager, TempTablesManager::new());

    let mut closed_scratch = TempTablesManager::new();
    compiler
        .prepare_with_options(
            "ВЫБРАТЬ 1 КАК Добавить",
            &PrepareOptions::new()
                .restricted()
                .temporary_tables(&closed_scratch),
        )
        .unwrap()
        .compile_batch(&snapshot, &CompileOptions::new(), &mut closed_scratch)
        .unwrap();
    assert_eq!(closed_scratch, TempTablesManager::new());
}

#[test]
fn a_replaced_constant_cannot_read_a_base_constant_with_the_same_name() {
    let constant = MetadataObject::new(
        "11111111-1111-1111-1111-111111111111",
        "Ставка",
        MetadataKind::Constant,
    );
    assert!(ensure_same_config_object(&constant, "22222222-2222-2222-2222-222222222222").is_err());
    assert!(ensure_same_config_object(&constant, "11111111-1111-1111-1111-111111111111").is_ok());
}

#[test]
fn restricted_reads_require_every_selection_to_be_allowed() {
    assert!(!all_selections_allowed("ВЫБРАТЬ 1 КАК Номер").unwrap());
    assert!(all_selections_allowed("ВЫБРАТЬ РАЗРЕШЕННЫЕ 1 КАК Номер").unwrap());
    assert!(
        !all_selections_allowed("ВЫБРАТЬ РАЗРЕШЕННЫЕ 1 КАК Номер; ВЫБРАТЬ 2 КАК Номер").unwrap()
    );
    assert!(!all_selections_allowed("ВЫБРАТЬ \"ALLOWED\" КАК Номер").unwrap());
    assert!(!all_selections_allowed("ВЫБРАТЬ &РАЗРЕШЕННЫЕ КАК Номер").unwrap());
    assert!(
        !all_selections_allowed("ВЫБРАТЬ &РАЗРЕШЕННЫЕ КАК Номер; ВЫБРАТЬ РАЗРЕШЕННЫЕ 2 КАК Другой")
            .unwrap()
    );

    let target = RestrictionTarget {
        object: ObjectId::from_bytes([1; 16]),
        table_part: None,
    };
    let denied = [AccessDecision::denied(target.clone())];
    assert!(require_allowed_for_restrictions("ВЫБРАТЬ 1 КАК Номер", &denied).is_err());
    assert!(require_allowed_for_restrictions("ВЫБРАТЬ РАЗРЕШЕННЫЕ 1 КАК Номер", &denied).is_ok());
    let allowed = [AccessDecision::unrestricted(target)];
    assert!(require_allowed_for_restrictions("ВЫБРАТЬ 1 КАК Номер", &allowed).is_ok());
}

#[test]
fn scalar_parameter_conversion_preserves_value_or_refuses_loss() {
    assert_eq!(parameter_value(&Value::Null).unwrap(), ParameterValue::Null);
    assert!(parameter_value(&Value::Undefined).is_err());
    assert_eq!(
        parameter_value(&Value::Number(
            BslNumber::parse_canonical("-15.50").unwrap()
        ))
        .unwrap(),
        ParameterValue::Number {
            unscaled: -155,
            scale: 1
        }
    );
    assert!(parameter_value(&Value::Date(BslDate::from_ticks(1).unwrap())).is_err());
    assert!(parameter_value(&Value::Str(BslString::from_units(vec![0xD800]))).is_err());
}

#[test]
fn materialized_rows_keep_scalar_types_and_reject_unknown_references() {
    let metadata = resolve_metadata(
        DbNames::from_entries(Vec::new()),
        Vec::new(),
        SchemaStorage {
            tables: Vec::new(),
            anomalies: Vec::new(),
        },
        Vec::new(),
    )
    .snapshot;
    assert_eq!(
        materialized_parameter(
            Cell::Number("15.50".into()),
            &ColumnKind::Number {
                precision: None,
                scale: None,
            },
            &metadata,
        )
        .unwrap(),
        ParameterValue::Number {
            unscaled: 155,
            scale: 1,
        }
    );
    assert_eq!(
        materialized_parameter(Cell::Null, &ColumnKind::String { length: None }, &metadata)
            .unwrap(),
        ParameterValue::Null
    );
    let object = ObjectId::from_bytes([1; 16]);
    assert_eq!(
        materialized_parameter(
            Cell::Bytes(vec![2; 16]),
            &ColumnKind::Reference {
                targets: vec![object],
                runtime_typed: false,
            },
            &metadata,
        )
        .unwrap(),
        ParameterValue::Reference {
            object,
            id: [2; 16],
        }
    );
    assert!(
        materialized_parameter(
            Cell::Bytes(vec![0; 20]),
            &ColumnKind::Reference {
                targets: Vec::new(),
                runtime_typed: true,
            },
            &metadata,
        )
        .is_err()
    );
    assert!(materialized_parameter(Cell::Uuid([0; 16]), &ColumnKind::Uuid, &metadata).is_err());
}

#[test]
fn typed_value_table_parameter_copies_rows_at_execution_time() {
    let source = Value::new_table();
    let Value::Object(object) = &source else {
        unreachable!("конструктор возвращает таблицу")
    };
    let BslObject::ValueTable(table) = object.as_ref() else {
        unreachable!("конструктор возвращает таблицу")
    };
    let row_id = {
        let mut table = table.borrow_mut();
        table.add_typed_column(
            "Значение",
            Some(vec![open_bsl::TypeRef::Native(TypeId::Number)]),
        );
        let row_id = table.add_row().unwrap();
        table.set_cell(row_id, 0, Value::number_from_i64(1));
        row_id
    };
    let placed = parameter_value(&source).unwrap();
    table
        .borrow_mut()
        .set_cell(row_id, 0, Value::number_from_i64(2));
    let current = parameter_value(&source).unwrap();
    let ParameterValue::Table { columns, rows } = placed else {
        panic!("ожидалась копия таблицы")
    };
    assert_eq!(
        columns,
        [ParameterColumn::new(
            "Значение",
            ColumnKind::Number {
                precision: None,
                scale: None
            }
        )]
    );
    assert_eq!(
        rows,
        [vec![ParameterValue::Number {
            unscaled: 1,
            scale: 0
        }]]
    );
    let ParameterValue::Table { rows, .. } = current else {
        panic!("ожидалась новая копия таблицы")
    };
    assert_eq!(
        rows,
        [vec![ParameterValue::Number {
            unscaled: 2,
            scale: 0
        }]]
    );
}

#[test]
fn placed_table_definition_keeps_copied_parameter_rows() {
    let snapshot = resolve_metadata(
        DbNames::from_entries(Vec::new()),
        Vec::new(),
        SchemaStorage {
            tables: Vec::new(),
            anomalies: Vec::new(),
        },
        Vec::new(),
    )
    .snapshot;
    let source = Value::new_table();
    let Value::Object(object) = &source else {
        unreachable!("конструктор возвращает таблицу")
    };
    let BslObject::ValueTable(table) = object.as_ref() else {
        unreachable!("конструктор возвращает таблицу")
    };
    let row_id = {
        let mut table = table.borrow_mut();
        table.add_typed_column(
            "Значение",
            Some(vec![open_bsl::TypeRef::Native(TypeId::Number)]),
        );
        let row_id = table.add_row().unwrap();
        table.set_cell(row_id, 0, Value::number_from_i64(1));
        row_id
    };
    let copied = parameter_value(&source).unwrap();
    let parameters = [QueryParameter::new("ИсходнаяТаблица", copied)];
    let compiler = QueryCompiler::new(&snapshot, PostgresBackend);
    let mut manager = TempTablesManager::new();
    compiler
        .prepare_with_options(
            "ВЫБРАТЬ Источник.Значение КАК Значение ПОМЕСТИТЬ ВТ ИЗ &ИсходнаяТаблица КАК Источник",
            &PrepareOptions::new()
                .restricted()
                .temporary_tables(&manager),
        )
        .unwrap()
        .compile_batch(
            &snapshot,
            &CompileOptions::new().parameters(&parameters),
            &mut manager,
        )
        .unwrap();
    let compiled = compiler
        .prepare_with_options(
            "ВЫБРАТЬ Значение ИЗ ВТ",
            &PrepareOptions::new()
                .restricted()
                .temporary_tables(&manager),
        )
        .unwrap()
        .compile_batch(&snapshot, &CompileOptions::new(), &mut manager)
        .unwrap()
        .unwrap();
    table
        .borrow_mut()
        .set_cell(row_id, 0, Value::number_from_i64(2));
    let later = compiler
        .prepare_with_options(
            "ВЫБРАТЬ Значение ИЗ ВТ",
            &PrepareOptions::new()
                .restricted()
                .temporary_tables(&manager),
        )
        .unwrap()
        .compile_batch(&snapshot, &CompileOptions::new(), &mut manager)
        .unwrap()
        .unwrap();
    assert_eq!(compiled.sql, later.sql);
    assert!(compiled.sql.contains("1"));
}

#[test]
fn untyped_value_table_parameter_is_rejected_before_sql() {
    let source = Value::new_table();
    let Value::Object(object) = &source else {
        unreachable!("конструктор возвращает таблицу")
    };
    let BslObject::ValueTable(table) = object.as_ref() else {
        unreachable!("конструктор возвращает таблицу")
    };
    table.borrow_mut().add_column("Значение");
    assert!(parameter_value(&source).is_err());
}

#[test]
fn query_cells_keep_undefined_distinct_from_null() {
    assert!(matches!(
        value_from_cell(Cell::Null, &ColumnKind::Undefined).unwrap(),
        Value::Undefined
    ));
    assert!(matches!(
        value_from_cell(Cell::Null, &ColumnKind::Null).unwrap(),
        Value::Null
    ));
    assert!(
        value_from_cell(
            Cell::Text("1".into()),
            &ColumnKind::Number {
                precision: None,
                scale: None,
            }
        )
        .is_err()
    );
}

fn reference_catalog() -> std::sync::Arc<bsl_orm::MetadataCatalog> {
    std::sync::Arc::new(
        bsl_orm::MetadataCatalog::new(
            "reference-test",
            vec![MetadataObject::new(
                "11111111-2222-3333-4444-555555555555",
                "Пример",
                MetadataKind::Catalog,
            )],
        )
        .unwrap(),
    )
}

#[test]
fn catalog_parameter_preserves_its_type_and_physical_rrref_bytes() {
    let catalog = reference_catalog();
    let uuid: Guid = "12345678-1234-4321-abcd-0123456789ab".parse().unwrap();
    let value = bsl_orm::catalog_reference_value(
        catalog,
        "11111111-2222-3333-4444-555555555555",
        uuid.to_bytes(),
    )
    .unwrap();
    assert_eq!(
        parameter_value(&value).unwrap(),
        ParameterValue::Reference {
            object: ObjectId::from_bytes(
                "11111111-2222-3333-4444-555555555555"
                    .parse::<Guid>()
                    .unwrap()
                    .to_bytes()
            ),
            id: [
                0xab, 0xcd, 0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0x43, 0x21, 0x12, 0x34, 0x12, 0x34,
                0x56, 0x78
            ],
        }
    );
}

#[test]
fn catalog_result_materialization_inverts_physical_rrref_and_keeps_type() {
    let catalog = reference_catalog();
    let uuid: Guid = "12345678-1234-4321-abcd-0123456789ab".parse().unwrap();
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
    let owner: Guid = "11111111-2222-3333-4444-555555555555".parse().unwrap();
    let kind = ColumnKind::Reference {
        targets: vec![ObjectId::from_bytes(owner.to_bytes())],
        runtime_typed: false,
    };
    let value = reference_from_cell(
        Cell::Bytes(uuid.to_1c_bytes().to_vec()),
        &kind,
        &metadata,
        std::sync::Arc::clone(&catalog),
    )
    .unwrap();
    let (object, actual) = bsl_orm::catalog_reference_parts(&value).unwrap();
    assert_eq!(object.id(), owner.as_str());
    assert_eq!(actual, uuid.to_bytes());
    let expected =
        bsl_orm::catalog_reference_value(catalog, owner.as_str(), uuid.to_bytes()).unwrap();
    assert_eq!(value, expected);
    assert!(
        reference_from_cell(
            Cell::Bytes(vec![0; 15]),
            &kind,
            &metadata,
            reference_catalog()
        )
        .is_err()
    );
    assert!(
        reference_from_cell(
            Cell::Bytes(vec![0; 16]),
            &ColumnKind::Reference {
                targets: vec![ObjectId::from_bytes([9; 16])],
                runtime_typed: false
            },
            &metadata,
            reference_catalog()
        )
        .is_err()
    );
}

#[test]
fn value_table_parameter_keeps_catalog_reference_column_and_cell() {
    let catalog = reference_catalog();
    let mut builder = open_bsl::Engine::builder().register_library(bsl_orm::library());
    for ty in catalog.value_types() {
        builder = builder.register_value_type(std::sync::Arc::clone(ty));
    }
    let engine = builder.build().unwrap();
    let session = Rc::new(SessionData::new(
        catalog,
        Principal::new("user", "Тест", vec![]),
    ));
    let mut state = engine.state_builder().host_data(session).build();
    let value = state
        .exec(
            r#"
        Т = Новый ТаблицаЗначений;
        Т.Колонки.Добавить("Ссылка", Новый ОписаниеТипов("CatalogRef.Пример"));
        Стр = Т.Добавить();
        Стр.Ссылка = Справочники.Пример.ПустаяСсылка();
        Возврат Т;
    "#,
        )
        .unwrap();
    let ParameterValue::Table { columns, rows } = parameter_value(&value).unwrap() else {
        panic!("таблица параметра");
    };
    let object = ObjectId::from_bytes(
        "11111111-2222-3333-4444-555555555555"
            .parse::<Guid>()
            .unwrap()
            .to_bytes(),
    );
    assert_eq!(
        columns[0].kind,
        ColumnKind::Reference {
            targets: vec![object],
            runtime_typed: false
        }
    );
    assert_eq!(
        rows[0][0],
        ParameterValue::Reference {
            object,
            id: [0; 16]
        }
    );
}

#[test]
fn standard_catalog_search_is_compiled_and_reports_fields_for_access_checks() {
    use open_sdbl::metadata::{
        DbNameEntry, FieldId, StandardFieldId, parse_config_descriptors, parse_schema_storage,
    };
    let owner = "90c54cdb-3887-427a-bb26-a00e68a7f036";
    let descriptors = parse_config_descriptors(
        owner,
        include_bytes!("../tests/fixtures/catalog-predefined-items/order-kinds.deflate"),
    )
    .unwrap();
    let schema = parse_schema_storage(br#"{0,{1,{"Reference1","N",1,"",{3,{"ID",0,{1,{"R",0,0,"Reference1",2}},"",0},{"Code",0,{1,{"S",2147483657,0,"",0}},"",0},{"Description",0,{1,{"S",2147483673,0,"",0}},"",0}},{0},{0},1,"R",{0},{0},"",0}}}"#).unwrap();
    let snapshot = resolve_metadata(
        DbNames::from_entries(vec![DbNameEntry {
            guid: owner.parse().unwrap(),
            alias: "Reference".into(),
            number: 1,
        }]),
        descriptors,
        schema,
        vec![open_sdbl::metadata::LiveTable {
            name: "_reference1".into(),
            columns: vec![
                open_sdbl::metadata::LiveColumn {
                    name: "_idrref".into(),
                    data_type: "bytea".into(),
                },
                open_sdbl::metadata::LiveColumn {
                    name: "_code".into(),
                    data_type: "mvarchar(9)".into(),
                },
                open_sdbl::metadata::LiveColumn {
                    name: "_description".into(),
                    data_type: "mvarchar(25)".into(),
                },
            ],
            indexes: vec![],
        }],
    )
    .snapshot;
    let compiler = QueryCompiler::new(&snapshot, PostgresBackend);
    for (field, predicate, standard) in [
        (
            "Код",
            "СОКРП(Источник.Код) = СОКРП(&Значение)",
            StandardFieldId::Code,
        ),
        (
            "Наименование",
            "СОКРП(Источник.Наименование) = СОКРП(&Значение)",
            StandardFieldId::Description,
        ),
        (
            "Наименование",
            "ЛЕВ(Источник.Наименование, ДЛИНАСТРОКИ(&Значение)) = &Значение",
            StandardFieldId::Description,
        ),
    ] {
        let source = format!(
            "ВЫБРАТЬ Источник.Ссылка КАК Ссылка, Источник.{field} КАК ЗначениеПоиска ИЗ Справочник.ВидыЗаказовПокупателей КАК Источник ГДЕ {predicate}"
        );
        let prepared = compiler
            .prepare_with_options(&source, &PrepareOptions::new().restricted())
            .unwrap();
        assert!(
            prepared
                .field_usage()
                .fields
                .iter()
                .any(|used| used.field == FieldId::Standard(standard))
        );
        let parameters = [QueryParameter::new(
            "Значение",
            ParameterValue::String("AbC".into()),
        )];
        let mut manager = TempTablesManager::new();
        assert!(
            prepared
                .compile_batch(
                    &snapshot,
                    &CompileOptions::new().parameters(&parameters),
                    &mut manager
                )
                .is_err()
        );
        let decisions = prepared
            .restriction_request()
            .targets
            .iter()
            .cloned()
            .map(AccessDecision::unrestricted)
            .collect::<Vec<_>>();
        prepared
            .compile_batch(
                &snapshot,
                &CompileOptions::new()
                    .parameters(&parameters)
                    .decisions(&decisions),
                &mut manager,
            )
            .unwrap();
    }
}

#[test]
fn balance_query_shape_compiles_with_native_members_and_bound_parameters() {
    use open_sdbl::metadata::{
        ColumnType, DbNameEntry, LiveColumn, LiveTable, SchemaColumn, SchemaTable,
        parse_config_descriptors,
    };
    let unread_role = "22222222-2222-2222-2222-222222222222";
    let record = format!(
        "{{11111111-1111-1111-1111-111111111111,\"balance-user\",\"\",{{1,{unread_role}}}}}"
    );
    let mut encoded = vec![1, 0x5a];
    encoded.extend(record.bytes().map(|byte| byte ^ 0x5a));
    let selected_user = open_sdbl::metadata::InfoBaseUser::new(
        open_sdbl::metadata::UserRow {
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
    let id = "c72d739a-05e3-4e9e-bcc7-3bf8baf5dc63";
    let descriptors = parse_config_descriptors(
        id,
        include_bytes!("../tests/fixtures/runtime-object-099/ОстаткиПорядок.deflate"),
    )
    .unwrap();
    let owner = descriptors
        .iter()
        .find(|item| item.object_guid.as_str() == id)
        .unwrap();
    let dimensions = owner.accumulation_register_dimensions.as_ref().unwrap();
    let resources = owner.accumulation_register_resources.as_ref().unwrap();
    let mut names = vec![
        DbNameEntry {
            guid: id.parse().unwrap(),
            alias: "AccumRg".into(),
            number: 1,
        },
        DbNameEntry {
            guid: id.parse().unwrap(),
            alias: "AccumRgT".into(),
            number: 2,
        },
    ];
    let mut fields = vec![("Period".to_owned(), "T", "timestamp without time zone")];
    for (index, guid) in dimensions.iter().chain(resources).enumerate() {
        names.push(DbNameEntry {
            guid: guid.clone(),
            alias: "Fld".into(),
            number: 10 + index as u32,
        });
        let (tag, sql_type) = if index < dimensions.len() {
            ("S", "mvarchar(12)")
        } else {
            ("N", "numeric(15,2)")
        };
        fields.push((format!("Fld{}", 10 + index), tag, sql_type));
    }
    let table = |name: &str, number, fields: &[(String, &str, &str)]| SchemaTable {
        name: name.into(),
        number,
        owner: None,
        inline_name: None,
        columns: fields
            .iter()
            .map(|(name, tag, _)| SchemaColumn {
                name: name.clone(),
                types: vec![ColumnType {
                    tag: (*tag).into(),
                    reference_target: None,
                }],
            })
            .collect(),
        indexes: vec![],
    };
    let live = |name: &str, fields: &[(String, &str, &str)]| LiveTable {
        name: name.into(),
        columns: fields
            .iter()
            .map(|(name, _, sql_type)| LiveColumn {
                name: format!("_{}", name.to_ascii_lowercase()),
                data_type: (*sql_type).into(),
            })
            .collect(),
        indexes: vec![],
    };
    let mut movements = fields.clone();
    movements.extend([
        ("Active".into(), "L", "boolean"),
        ("RecordKind".into(), "N", "numeric(1,0)"),
    ]);
    let mut totals = fields;
    totals.push(("Splitter".into(), "N", "numeric(10,0)"));
    // Физическая схема синтетическая: тест проверяет компиляцию, а не чтение БД.
    let snapshot = resolve_metadata(
        DbNames::from_entries(names),
        descriptors.clone(),
        SchemaStorage {
            tables: vec![
                table("AccumRg1", 1, &movements),
                table("AccumRgT2", 2, &totals),
            ],
            anomalies: vec![],
        },
        vec![live("_accumrg1", &movements), live("_accumrgt2", &totals)],
    )
    .snapshot;
    let field_name = |guid: &open_sdbl::metadata::Guid| {
        descriptors
            .iter()
            .find(|item| &item.object_guid == guid)
            .unwrap()
            .name
            .as_str()
    };
    let dimension = field_name(&dimensions[0]);
    let resource = field_name(&resources[0]);
    #[derive(Debug, Default)]
    struct Capture(RefCell<Vec<QueryRequest>>);
    impl DataAccess for Capture {
        fn constant_value(
            &self,
            _: &MetadataObject,
            _: &Principal,
            _: &mut CallContext<'_>,
        ) -> RtResult<Value> {
            Err(error("константы в этом контроле не читаются"))
        }
        fn execute_query(
            &self,
            request: &QueryRequest,
            principal: &Principal,
            _: &mut CallContext<'_>,
        ) -> RtResult<QueryResultData> {
            assert_eq!(principal.id(), "balance-user");
            self.0.borrow_mut().push(request.clone());
            QueryResultData::new(
                ["ПолеОстатков0".into(), "ПолеОстатков1".into()],
                Vec::<Vec<Value>>::new(),
            )
            .map_err(|failure| error(failure.to_string()))
        }
    }
    let attributes = |guids: &[Guid], native, qualifiers: &[&str]| {
        guids
            .iter()
            .map(|guid| {
                bsl_orm::DocumentAttribute::new(
                    guid.as_str(),
                    field_name(guid),
                    vec![open_bsl::ColumnType {
                        id: open_bsl::TypeRef::Native(native),
                        quals: qualifiers.iter().map(|value| (*value).into()).collect(),
                    }],
                )
            })
            .collect::<Vec<_>>()
    };
    let catalog = bsl_orm::MetadataCatalog::new(
        "balance-compiler",
        vec![MetadataObject::new(
            id,
            &owner.name,
            MetadataKind::AccumulationRegister,
        )],
    )
    .unwrap()
    .with_accumulation_register_metadata(
        id,
        Some(bsl_orm::AccumulationRegisterKind::Balance),
        Some(attributes(dimensions, TypeId::String, &["12", "1"])),
        Some(attributes(resources, TypeId::Number, &["15", "2", "0"])),
    )
    .unwrap();
    let capture = Rc::new(Capture::default());
    let host = SessionData::new(
        std::sync::Arc::new(catalog),
        Principal::new("balance-user", "Тест", vec![]),
    )
    .with_data_access(capture.clone());
    let engine = open_bsl::Engine::builder()
        .register_library(bsl_orm::library())
        .build()
        .unwrap();
    let mut state = engine.state_builder().host_data(Rc::new(host)).build();
    let compiler = QueryCompiler::new(&snapshot, PostgresBackend);
    for boundary in ["", "Дата(2026, 10, 6, 12, 0, 0)"] {
        state.exec(&format!(
            "Возврат РегистрыНакопления.{}.Остатки({boundary}, Новый Структура(\"{dimension}\", \"a\"), \"{dimension}\", \"{resource}\");",
            owner.name
        )).unwrap();
        let calls = capture.0.borrow();
        let request = calls.last().unwrap();
        let parameters = request
            .parameters()
            .map(|(name, value)| QueryParameter::new(name, parameter_value(value).unwrap()))
            .collect::<Vec<_>>();
        let compiled = compiler
            .compile_with(
                request.text(),
                &CompileOptions::new().parameters(&parameters),
            )
            .unwrap();
        let prepared = compiler
            .prepare_with_options(request.text(), &PrepareOptions::new().restricted())
            .unwrap();
        assert!(!prepared.restriction_request().targets.is_empty());
        let access_without_principal = AccessStore::new(&snapshot);
        assert!(
            check_user_field_access(&access_without_principal, &snapshot, prepared.field_usage(),)
                .is_err()
        );
        assert!(
            user_decisions(
                &access_without_principal,
                &snapshot,
                prepared.restriction_request(),
                &[],
                &SessionParameters::new(),
            )
            .is_err()
        );
        let mut access_with_unread_role = AccessStore::new(&snapshot);
        access_with_unread_role.set_current_user(Some(selected_user.clone()));
        let failure =
            check_user_field_access(&access_with_unread_role, &snapshot, prepared.field_usage())
                .unwrap_err();
        assert!(failure.to_string().contains(unread_role));
        assert!(
            user_decisions(
                &access_with_unread_role,
                &snapshot,
                prepared.restriction_request(),
                &[],
                &SessionParameters::new(),
            )
            .is_err()
        );
        assert!(
            prepared
                .compile_batch(
                    &snapshot,
                    &CompileOptions::new().parameters(&parameters),
                    &mut TempTablesManager::new(),
                )
                .is_err()
        );
        let decisions = prepared
            .restriction_request()
            .targets
            .iter()
            .cloned()
            .map(AccessDecision::unrestricted)
            .collect::<Vec<_>>();
        prepared
            .compile_batch(
                &snapshot,
                &CompileOptions::new()
                    .parameters(&parameters)
                    .decisions(&decisions),
                &mut TempTablesManager::new(),
            )
            .unwrap();
        assert_eq!(
            compiled
                .columns
                .iter()
                .map(|column| column.label.as_str())
                .collect::<Vec<_>>(),
            ["ПолеОстатков0", "ПолеОстатков1"]
        );
        assert!(compiled.sql.contains("_accumrgt2"));
        if !boundary.is_empty() {
            assert!(compiled.sql.contains("_accumrg1"));
            assert!(
                compiled
                    .sql
                    .contains("\"__movement_base\".\"_period\" < TIMESTAMP '2026-10-06 12:00:00'"),
                "{}",
                compiled.sql
            );
        }
    }
    assert_eq!(capture.0.borrow().len(), 2);
}

#[test]
fn document_attribute_lookup_keeps_its_native_field_and_restricted_query_path() {
    use open_sdbl::metadata::{
        ColumnType, DbNameEntry, FieldId, LiveColumn, LiveTable, SchemaColumn, SchemaTable,
        parse_config_descriptors,
    };
    let owner = "571e1bd9-0f64-4a05-a276-dba4fe242ad0";
    let descriptors = parse_config_descriptors(
        owner,
        include_bytes!("../tests/fixtures/document-owned-attributes/event.deflate"),
    )
    .unwrap();
    let attribute = descriptors
        .iter()
        .find(|descriptor| descriptor.name == "КонтактыПеренесены")
        .unwrap()
        .object_guid
        .clone();
    // Собственные имя и тип получены из нативного Config. Физические номера
    // и схема ограничены тестом, чтобы проверить права до исполнения SQL.
    let snapshot = resolve_metadata(
        DbNames::from_entries(vec![
            DbNameEntry {
                guid: owner.parse().unwrap(),
                alias: "Document".into(),
                number: 1,
            },
            DbNameEntry {
                guid: attribute,
                alias: "Fld".into(),
                number: 2,
            },
        ]),
        descriptors,
        SchemaStorage {
            tables: vec![SchemaTable {
                name: "Document1".into(),
                number: 1,
                owner: None,
                inline_name: None,
                columns: vec![
                    SchemaColumn {
                        name: "ID".into(),
                        types: vec![ColumnType {
                            tag: "R".into(),
                            reference_target: Some("Document1".into()),
                        }],
                    },
                    SchemaColumn {
                        name: "Fld2".into(),
                        types: vec![ColumnType {
                            tag: "L".into(),
                            reference_target: None,
                        }],
                    },
                ],
                indexes: vec![],
            }],
            anomalies: vec![],
        },
        vec![LiveTable {
            name: "_document1".into(),
            columns: vec![
                LiveColumn {
                    name: "_idrref".into(),
                    data_type: "bytea".into(),
                },
                LiveColumn {
                    name: "_fld2".into(),
                    data_type: "boolean".into(),
                },
            ],
            indexes: vec![],
        }],
    )
    .snapshot;
    let object = snapshot
        .object_id(SdblMetadataKind::Document, "Событие")
        .unwrap();
    let field = snapshot.attribute_id(object, "КонтактыПеренесены").unwrap();
    let prepared = QueryCompiler::new(&snapshot, PostgresBackend)
        .prepare_with_options("ВЫБРАТЬ ПЕРВЫЕ 1 Источник.Ссылка КАК Ссылка ИЗ Документ.Событие КАК Источник ГДЕ Источник.КонтактыПеренесены = &Значение", &PrepareOptions::new().restricted())
        .unwrap();
    assert!(
        prepared
            .field_usage()
            .fields
            .iter()
            .any(|used| used.field == FieldId::Metadata(field))
    );
    assert_eq!(prepared.restriction_request().targets.len(), 1);
    let target = prepared.restriction_request().targets[0].clone();
    assert_eq!(target.object, object);
    let parameters = [QueryParameter::new(
        "Значение",
        ParameterValue::Boolean(false),
    )];
    let mut manager = TempTablesManager::new();
    assert!(
        prepared
            .compile_batch(
                &snapshot,
                &CompileOptions::new().parameters(&parameters),
                &mut manager
            )
            .is_err()
    );
    assert!(
        require_allowed_for_restrictions(
            "поиск документа",
            &[AccessDecision::denied(target.clone())]
        )
        .is_err()
    );
    let decisions = [AccessDecision::unrestricted(target)];
    let compiled = prepared
        .compile_batch(
            &snapshot,
            &CompileOptions::new()
                .parameters(&parameters)
                .decisions(&decisions),
            &mut manager,
        )
        .unwrap()
        .unwrap();
    assert_eq!(compiled.columns.len(), 1);
    assert!(matches!(
        compiled.columns[0].kind,
        ColumnKind::Reference { .. }
    ));
    assert!(compiled.sql.contains("_document1"));
    assert!(compiled.sql.contains("_fld2"));
}
