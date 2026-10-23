use bsl_rt::{BslValue, TypeId, lookup_enum, lookup_member};

#[test]
fn reached_spreadsheet_detail_values_match_native_aliases_and_presentation() {
    let kind = lookup_enum("ИспользованиеРасшифровкиТабличногоДокумента").unwrap();
    assert_eq!(Some(kind), lookup_enum("spreadsheetdocumentdetailuse"));
    let none = lookup_member(kind, "бЕзОбРаБоТкИ").unwrap();
    let cell = lookup_member(kind, "яЧеЙкА").unwrap();
    assert_eq!(Some(none), lookup_member(kind, "WithoutProcessing"));
    assert_eq!(Some(cell), lookup_member(kind, "Cell"));
    assert_ne!(none, cell);
    let BslValue::Type(ty) = BslValue::Enum(none).type_of().unwrap() else {
        panic!("ожидался тип перечисления");
    };
    for name in [
        "ИспользованиеРасшифровкиТабличногоДокумента",
        "SpreadsheetDocumentDetailUse",
    ] {
        assert_eq!(TypeId::lookup(name), ty.native());
    }
    assert_eq!(
        BslValue::Enum(none).type_of().unwrap(),
        BslValue::Enum(cell).type_of().unwrap()
    );
    let actual = format!(
        "enum.type={}\nenum.none.presentation={}\nenum.cell.presentation={}\nenum.aliases=YES\nenum.type.lookup=YES\nenum.distinct=YES\ncompleted=YES",
        ty.name(),
        none.display_text(),
        cell.display_text()
    );
    let expected = include_str!(
        "../../../docs/research/fixtures/spreadsheet-detail-use-native-1015/platform.oracle"
    )
    .trim_start_matches('\u{feff}')
    .replace("\r\n", "\n");
    assert_eq!(actual, expected.trim());
}
