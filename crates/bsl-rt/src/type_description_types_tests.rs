use crate::builtin::{BuiltinMethod, call_builtin_method};
use crate::{BslObject, BslValue, TypeId, TypeRef};
use std::rc::Rc;

#[test]
fn type_enumeration_returns_an_independent_array_of_type_values() {
    let types = vec![
        TypeRef::Native(TypeId::String),
        TypeRef::Native(TypeId::Number),
    ];
    let description = BslValue::Object(Rc::new(BslObject::TypeDescription(types.clone())));
    let first = call_builtin_method(BuiltinMethod::Types, &description, &[]).unwrap();
    let BslValue::Object(first_object) = &first else {
        panic!("массив типов")
    };
    let BslObject::Array(first_items) = first_object.as_ref() else {
        panic!("массив типов")
    };
    assert_eq!(
        *first_items.borrow(),
        types
            .iter()
            .cloned()
            .map(BslValue::Type)
            .collect::<Vec<_>>()
    );
    call_builtin_method(BuiltinMethod::Clear, &first, &[]).unwrap();
    let second = call_builtin_method(BuiltinMethod::Types, &description, &[]).unwrap();
    let BslValue::Object(second_object) = second else {
        panic!("массив типов")
    };
    let BslObject::Array(second_items) = second_object.as_ref() else {
        panic!("массив типов")
    };
    assert_eq!(second_items.borrow().len(), 2);
    assert_eq!(BuiltinMethod::lookup("Типы"), Some(BuiltinMethod::Types));
    assert_eq!(BuiltinMethod::lookup("types"), Some(BuiltinMethod::Types));
}

#[test]
fn type_membership_matches_the_native_positive_and_negative_cases() {
    let description = BslValue::Object(Rc::new(BslObject::TypeDescription(vec![TypeRef::Native(
        TypeId::String,
    )])));
    for (ty, expected) in [(TypeId::String, true), (TypeId::Number, false)] {
        assert_eq!(
            call_builtin_method(
                BuiltinMethod::ContainsType,
                &description,
                &[BslValue::Type(TypeRef::Native(ty))]
            )
            .unwrap(),
            BslValue::Boolean(expected)
        );
    }
    assert_eq!(
        BuiltinMethod::lookup("СодержитТип"),
        Some(BuiltinMethod::ContainsType)
    );
    assert_eq!(
        BuiltinMethod::lookup("ContainsType"),
        Some(BuiltinMethod::ContainsType)
    );
}

fn qualified_description() -> BslValue {
    use crate::table::ColumnType;
    BslValue::new_qualified_type_description(vec![
        ColumnType::plain(TypeRef::Native(TypeId::Boolean)),
        ColumnType {
            id: TypeRef::Native(TypeId::String),
            quals: vec!["100".into(), "1".into()],
        },
        ColumnType::plain(TypeRef::Native(TypeId::Date)),
        ColumnType {
            id: TypeRef::Native(TypeId::Number),
            quals: vec!["15".into(), "2".into(), "0".into()],
        },
    ])
}

#[test]
fn qualified_description_applies_the_native_characteristic_bounds() {
    let description = qualified_description();
    let receipt = include_str!(
        "../../../docs/research/fixtures/characteristic-value-type-qualifiers/measure.platform.txt"
    );
    let adjust =
        |value| call_builtin_method(BuiltinMethod::AdjustValue, &description, &[value]).unwrap();
    let text = adjust(BslValue::Str(crate::BslString::from_str(&"a".repeat(120))));
    assert_eq!(text.as_str("проба").unwrap().units().len(), 100);
    assert!(receipt.lines().any(|line| line == "string.length=100"));
    for (input, output, id) in [
        ("1.234", "1.23", "number.round"),
        ("-1.235", "-1.24", "number.negative"),
        ("12345678901234.56", "9999999999999.99", "number.large"),
    ] {
        assert!(receipt.lines().any(|line| line == format!("{id}={output}")));
        assert_eq!(
            adjust(BslValue::Number(
                crate::BslNumber::parse_canonical(input).unwrap()
            )),
            BslValue::Number(crate::BslNumber::parse_canonical(output).unwrap())
        );
    }
    assert_eq!(adjust(BslValue::Boolean(true)), BslValue::Boolean(true));
    let date = BslValue::Date(crate::BslDate::from_civil(2024, 2, 3, 4, 5, 6).unwrap());
    assert_eq!(adjust(date.clone()), date);
    assert_eq!(adjust(BslValue::Undefined), BslValue::Undefined);
    assert_eq!(adjust(BslValue::Null), BslValue::Undefined);
    assert_eq!(
        call_builtin_method(BuiltinMethod::AdjustValue, &description, &[]).unwrap(),
        BslValue::Undefined
    );
    assert_eq!(
        description.type_of().unwrap(),
        BslValue::Type(TypeRef::Native(TypeId::TypeDescription))
    );
}

#[test]
fn qualified_description_retains_native_qualifiers_through_internal_text_and_columns() {
    let description = qualified_description();
    let mut shapes = crate::RuntimeShapes::seeded(vec![], vec![], None);
    let text = crate::vstr::value_to_string_internal(&description, &shapes).unwrap();
    let receipt = include_str!(
        "../../../docs/research/fixtures/characteristic-value-type-qualifiers/measure-snapshot.platform.txt"
    );
    assert!(receipt.contains("{\"S\",100,1}"));
    assert!(receipt.contains("{\"N\",15,2,0}"));
    assert!(text.contains("{\"S\",100,1}"));
    assert!(text.contains("{\"N\",15,2,0}"));
    let restored = crate::vstr::value_from_string_internal(&text, &mut shapes).unwrap();
    let value = BslValue::Str(crate::BslString::from_str(&"a".repeat(120)));
    let adjusted = call_builtin_method(
        BuiltinMethod::AdjustValue,
        &restored,
        std::slice::from_ref(&value),
    )
    .unwrap();
    assert_eq!(adjusted.as_str("проба").unwrap().units().len(), 100);
    let table = crate::table::ValueTableData::new();
    let columns = BslValue::Object(Rc::new(BslObject::TableColumns(table.clone())));
    columns
        .table_add_column(
            &BslValue::Str(crate::BslString::from_str("Значение")),
            &restored,
        )
        .unwrap();
    assert_eq!(table.borrow().adjust_to_column_type(0, value), adjusted);
}

#[test]
fn qualified_description_keeps_the_existing_explicit_graph_transfer_rejection() {
    let shapes = crate::RuntimeShapes::seeded(vec![], vec![], None);
    let legacy = BslValue::Object(Rc::new(BslObject::TypeDescription(vec![])));
    for description in [legacy, qualified_description()] {
        assert!(matches!(
            crate::value_graph::SerializedValueGraph::capture(
                &[description],
                &shapes,
                &crate::value_graph::GraphLimits::default(),
            ),
            Err(crate::RtError::ResourceLimit(_))
        ));
    }
}
