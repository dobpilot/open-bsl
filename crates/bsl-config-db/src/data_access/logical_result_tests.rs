use super::*;
use open_sdbl::query::{ColumnKind, PostgresBackend, QueryCompiler};

fn snapshot(composite: bool) -> (open_sdbl::metadata::MetadataSnapshot, String) {
    use open_sdbl::metadata::*;
    let id = "9893e2d6-f3f8-4d73-bb06-19bf26d216ab";
    let descriptors = parse_config_descriptors(id, include_bytes!("../../tests/fixtures/functional-options/native/9893e2d6-f3f8-4d73-bb06-19bf26d216ab.deflate")).unwrap();
    let name = descriptors
        .iter()
        .find(|item| item.object_guid.as_str() == id)
        .unwrap()
        .name
        .clone();
    // Нативный Config задаёт имя владельца; физическая раскладка здесь синтетическая.
    let types = if composite { vec!["L", "S"] } else { vec!["L"] };
    let columns = if composite {
        vec![
            ("_fld2_l", "boolean"),
            ("_fld2_s", "character varying(120)"),
            ("_fld2_type", "bytea"),
        ]
    } else {
        vec![("_fld2", "boolean")]
    };
    let snapshot = resolve_metadata(
        DbNames::from_entries(vec![
            DbNameEntry {
                guid: id.parse().unwrap(),
                alias: "Const".into(),
                number: 1,
            },
            DbNameEntry {
                guid: id.parse().unwrap(),
                alias: "Fld".into(),
                number: 2,
            },
        ]),
        descriptors,
        SchemaStorage {
            tables: vec![SchemaTable {
                name: "Const1".into(),
                number: 1,
                owner: None,
                inline_name: None,
                columns: vec![SchemaColumn {
                    name: "Fld2".into(),
                    types: types
                        .into_iter()
                        .map(|tag| ColumnType {
                            tag: tag.into(),
                            reference_target: None,
                        })
                        .collect(),
                }],
                indexes: vec![],
            }],
            anomalies: vec![],
        },
        vec![LiveTable {
            name: "_const1".into(),
            columns: columns
                .into_iter()
                .map(|(name, data_type)| LiveColumn {
                    name: name.into(),
                    data_type: data_type.into(),
                })
                .collect(),
            indexes: vec![],
        }],
    )
    .snapshot;
    (snapshot, name)
}

fn compiled() -> open_sdbl::query::CompiledQuery {
    let (snapshot, name) = snapshot(false);
    QueryCompiler::new(&snapshot, PostgresBackend)
        .compile(&format!(
            "SELECT CASE WHEN TRUE THEN FALSE ELSE \"profile\" END AS Value FROM Constant.{name}"
        ))
        .unwrap()
}

#[test]
fn active_member_alone_is_converted_to_one_logical_value() {
    let query = compiled();
    let plan = query.logical_projection.as_ref().unwrap();
    assert_eq!(plan.len(), 1);
    let LogicalResultValue::Composite {
        discriminator,
        members,
    } = &plan[0].value
    else {
        panic!("ожидался составной рецепт")
    };
    for tag in [2, 5] {
        let mut row = vec![Cell::Null; query.columns.len()];
        row[*discriminator] = Cell::Bytes(vec![tag]);
        for member in members {
            row[member.column.unwrap()] = if member.type_tag != tag {
                Cell::Bytes(vec![0xff])
            } else if tag == 2 {
                Cell::Bool(false)
            } else {
                Cell::Text("profile".into())
            };
        }
        let mut calls = 0;
        let values = materialize_row(row, plan, &mut |index, cell| {
            calls += 1;
            super::super::value_from_cell(cell, &query.columns[index].kind)
        })
        .unwrap();
        assert_eq!(calls, 1);
        assert_eq!(
            values,
            vec![if tag == 2 {
                Value::Boolean(false)
            } else {
                Value::Str("profile".into())
            }]
        );
    }
}

#[test]
fn invalid_discriminator_and_indices_fail_explicitly() {
    let query = compiled();
    let plan = query.logical_projection.as_ref().unwrap();
    let LogicalResultValue::Composite { discriminator, .. } = &plan[0].value else {
        panic!("ожидался составной рецепт")
    };
    for bytes in [vec![], vec![2, 0], vec![255]] {
        let mut row = vec![Cell::Null; query.columns.len()];
        row[*discriminator] = Cell::Bytes(bytes);
        assert!(materialize_row(row, plan, &mut |_, _| Ok(Value::Undefined)).is_err());
    }
    let mut invalid = plan.clone();
    invalid[0].value = LogicalResultValue::Scalar {
        column: query.columns.len(),
    };
    assert!(
        materialize_row(
            vec![Cell::Null; query.columns.len()],
            &invalid,
            &mut |_, _| Ok(Value::Undefined)
        )
        .is_err()
    );
    assert!(
        query
            .columns
            .iter()
            .any(|column| matches!(column.kind, ColumnKind::Boolean))
    );
}

#[test]
fn repeated_scalar_fields_keep_independent_logical_positions() {
    let (snapshot, name) = snapshot(false);
    let query = QueryCompiler::new(&snapshot, PostgresBackend)
        .compile(&format!(
            "SELECT T.Value AS First, T.Value AS Second FROM Constant.{name} AS T"
        ))
        .unwrap();
    let plan = query.logical_projection.as_ref().unwrap();
    assert_eq!(
        plan.iter()
            .map(|column| column.name.as_str())
            .collect::<Vec<_>>(),
        ["First", "Second"]
    );
    let values = materialize_row(
        vec![Cell::Bool(false), Cell::Bool(true)],
        plan,
        &mut |index, cell| super::super::value_from_cell(cell, &query.columns[index].kind),
    )
    .unwrap();
    assert_eq!(values, vec![Value::Boolean(false), Value::Boolean(true)]);
    assert!(
        materialize_row(vec![Cell::Bool(false)], plan, &mut |_, _| Ok(
            Value::Undefined
        ))
        .is_err()
    );
}

#[test]
fn invalid_inactive_member_and_duplicate_tags_are_rejected() {
    let query = compiled();
    let plan = query.logical_projection.as_ref().unwrap();
    for duplicate in [false, true] {
        let mut invalid = plan.clone();
        let LogicalResultValue::Composite {
            discriminator,
            members,
        } = &mut invalid[0].value
        else {
            panic!("ожидался составной рецепт")
        };
        if duplicate {
            members[1].type_tag = members[0].type_tag;
        } else {
            members[1].column = Some(query.columns.len());
        }
        let mut row = vec![Cell::Null; query.columns.len()];
        row[*discriminator] = Cell::Bytes(vec![members[0].type_tag]);
        assert!(materialize_row(row, &invalid, &mut |_, _| Ok(Value::Undefined)).is_err());
    }
}

#[test]
fn a_storage_field_undefined_tag_has_no_payload_to_decode() {
    let (snapshot, name) = snapshot(true);
    let query = QueryCompiler::new(&snapshot, PostgresBackend)
        .compile(&format!("SELECT Value AS Value FROM Constant.{name}"))
        .unwrap();
    let plan = query.logical_projection.as_ref().unwrap();
    let LogicalResultValue::Composite {
        discriminator,
        members,
    } = &plan[0].value
    else {
        panic!("ожидался составной рецепт")
    };
    assert!(
        members
            .iter()
            .any(|member| member.type_tag == TypeValue::Undefined.tag() && member.column.is_none())
    );
    let mut row = vec![Cell::Null; query.columns.len()];
    row[*discriminator] = Cell::Bytes(vec![TypeValue::Undefined.tag()]);
    let values = materialize_row(row, plan, &mut |_, _| {
        panic!("у Неопределено нет полезной ячейки")
    })
    .unwrap();
    assert_eq!(values, vec![Value::Undefined]);
}

#[test]
fn point_recipe_reconstructs_one_typed_value_and_rejects_invalid_cells() {
    let mut logical = compiled().logical_projection.unwrap().remove(0);
    logical.value = LogicalResultValue::PointInTime {
        date: 0,
        reference: 1,
    };
    let date = open_bsl::BslDate::from_civil(2026, 10, 19, 0, 0, 0).unwrap();
    let row = materialize_row(
        vec![Cell::Text("date".into()), Cell::Text("reference".into())],
        &[logical.clone()],
        &mut |index, _| {
            Ok(if index == 0 {
                Value::Date(date)
            } else {
                Value::Undefined
            })
        },
    )
    .unwrap();
    assert_eq!(row.len(), 1);
    let object = row[0].object_ref().unwrap();
    let point = object.downcast_ref::<open_bsl::PointInTimeValue>().unwrap();
    assert_eq!(point.date(), date);
    assert_eq!(point.reference(), &Value::Undefined);
    assert!(
        materialize_row(
            vec![Cell::Null, Cell::Null],
            &[logical.clone()],
            &mut |_, _| Ok(Value::Boolean(false)),
        )
        .is_err()
    );
    logical.value = LogicalResultValue::PointInTime {
        date: 0,
        reference: 2,
    };
    assert!(
        materialize_row(
            vec![Cell::Null, Cell::Null],
            &[logical.clone()],
            &mut |_, _| panic!("invalid recipe must be rejected before conversion"),
        )
        .is_err()
    );
    logical.value = LogicalResultValue::PointInTime {
        date: 0,
        reference: 0,
    };
    assert!(
        materialize_row(vec![Cell::Null], &[logical], &mut |_, _| panic!(
            "overlapping recipe must be rejected before conversion"
        ),)
        .is_err()
    );
}
