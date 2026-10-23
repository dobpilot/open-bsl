use super::*;

#[test]
fn native_unqualified_string_matches_only_its_explicit_default_qualifiers() {
    let column = open_bsl::ColumnType {
        id: TypeRef::Native(open_bsl::TypeId::String),
        quals: Vec::new(),
    };
    let file = |length: &str, policy: &str| open_bsl::ColumnType {
        id: column.id.clone(),
        quals: vec![length.into(), policy.into()],
    };
    assert!(compatible_qualifiers(&file("0", "1"), &column));
    assert!(!compatible_qualifiers(&file("10", "1"), &column));
    assert!(!compatible_qualifiers(&file("0", "0"), &column));
    let fixed = open_bsl::ColumnType {
        id: column.id.clone(),
        quals: vec!["10".into(), "0".into()],
    };
    assert!(!compatible_qualifiers(&file("10", "1"), &fixed));
    assert!(compatible_qualifiers(&file("10", "0"), &fixed));
}

#[test]
fn native_unqualified_date_matches_datetime_but_rejects_date_or_time_only() {
    let column = open_bsl::ColumnType {
        id: TypeRef::Native(open_bsl::TypeId::Date),
        quals: Vec::new(),
    };
    let file = |fractions: &str| open_bsl::ColumnType {
        id: column.id.clone(),
        quals: vec![fractions.into()],
    };
    assert!(compatible_qualifiers(&file("DT"), &column));
    assert!(!compatible_qualifiers(&file("D"), &column));
    assert!(!compatible_qualifiers(&file("T"), &column));
}

#[test]
fn native_typeset_preserves_the_defined_type_reference_for_expansion() {
    let node = ConfigurationProperty {
        name: "typeSet".into(),
        value: "cfg:DefinedType.БезопасныйРежим".into(),
        attributes: Arc::from([]),
        children: Arc::from([]),
    };
    assert_eq!(type_names(&node), ["DefinedType.БезопасныйРежим"]);
    let edt = ConfigurationProperty {
        name: "typeSets".into(),
        value: "DefinedType.БезопасныйРежим".into(),
        ..node
    };
    assert_eq!(type_names(&edt), ["DefinedType.БезопасныйРежим"]);
}

#[test]
fn native_reference_family_uses_the_existing_orm_reference_descriptors() {
    struct Source(open_bsl::ConfigurationIndex);
    impl open_bsl::ConfigurationSource for Source {
        fn index(&self) -> &open_bsl::ConfigurationIndex {
            &self.0
        }
        fn module_source(&self, _: &str) -> Result<Arc<str>, String> {
            panic!("type expansion must not read modules")
        }
    }
    let first = Arc::new(ValueTypeDescriptor::new(
        "bsl-orm",
        "test/ref/one",
        "Первый",
        ["CatalogRef.Первый", "СправочникСсылка.Первый"],
    ));
    let second = Arc::new(ValueTypeDescriptor::new(
        "bsl-orm",
        "test/ref/two",
        "Второй",
        ["CatalogRef.Второй", "СправочникСсылка.Второй"],
    ));
    let attached = Attached::new(
        Arc::new(Source(
            open_bsl::ConfigurationIndex::new("root", Vec::new()).unwrap(),
        )),
        &[
            ("one".into(), first.clone()),
            ("two".into(), second.clone()),
        ],
    );
    let node = ConfigurationProperty {
        name: "typeSet".into(),
        value: "cfg:CatalogRef".into(),
        attributes: Arc::from([]),
        children: Arc::from([]),
    };
    let columns = source_types(
        &attached,
        &node,
        &|_| panic!("existing family must not create another type"),
        &mut HashSet::new(),
    )
    .unwrap();
    assert_eq!(columns.len(), 2);
    assert_eq!(columns[0].id, TypeRef::Value(first));
    assert_eq!(columns[1].id, TypeRef::Value(second));
}
