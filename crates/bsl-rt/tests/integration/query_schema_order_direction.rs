use bsl_rt::{BslValue, TypeId, lookup_enum, lookup_member};

#[test]
fn reached_query_order_directions_match_native_names_and_values() {
    let kind = lookup_enum("НаправлениеПорядкаСхемыЗапроса").unwrap();
    assert_eq!(Some(kind), lookup_enum("queryschemaorderdirection"));
    let asc = lookup_member(kind, "пОвОзРаСтАнИю").unwrap();
    let desc = lookup_member(kind, "пОуБыВаНиЮ").unwrap();
    assert_eq!(Some(asc), lookup_member(kind, "Ascending"));
    assert_eq!(Some(desc), lookup_member(kind, "Descending"));
    assert!(lookup_member(kind, "Asc").is_none());
    let BslValue::Type(ty) = BslValue::Enum(asc).type_of().unwrap() else {
        panic!("тип перечисления");
    };
    for name in [
        "НаправлениеПорядкаСхемыЗапроса",
        "QuerySchemaOrderDirection",
    ] {
        assert_eq!(TypeId::lookup(name), ty.native());
    }
    assert_ne!(asc, desc);
    let actual = format!(
        "enum.type={}\nenum.presentation={}\nenum.alias.asc=ERROR\nenum.alias.ascending=YES\nenum.type.lookup=YES\nenum.desc.alias=YES\nenum.desc.presentation={}\nenum.distinct=YES\ncompleted=YES",
        ty.name(),
        asc.display_text(),
        desc.display_text()
    );
    let expected = include_str!(
        "../../../../docs/research/fixtures/query-schema-order-direction-native-1015/platform.oracle"
    )
    .trim_start_matches('\u{feff}')
    .replace("\r\n", "\n");
    assert_eq!(actual, expected.trim());
}
