use bsl_rt::{BslValue, TypeId, lookup_enum, lookup_member};
#[test]
fn reached_pattern_matches_native_aliases_and_type() {
    let kind = lookup_enum("ТипУзораТабличногоДокумента").unwrap();
    assert_eq!(Some(kind), lookup_enum("spreadsheetdocumentpatterntype"));
    let value = lookup_member(kind, "бЕзУзОрА").unwrap();
    assert_eq!(Some(value), lookup_member(kind, "WithoutPattern"));
    assert_eq!(value.display_text(), "Без узора");
    let BslValue::Type(ty) = BslValue::Enum(value).type_of().unwrap() else {
        panic!("ожидался тип перечисления");
    };
    for name in [
        "ТипУзораТабличногоДокумента",
        "SpreadsheetDocumentPatternType",
    ] {
        assert_eq!(TypeId::lookup(name), ty.native());
    }
    assert_eq!(ty.name(), "ТипУзораТабличногоДокумента");
    let BslValue::Type(meta) = BslValue::EnumType(kind).type_of().unwrap() else {
        panic!("ожидался метатип перечисления");
    };
    assert_eq!(meta.name(), "ПеречислениеТипУзораТабличногоДокумента");
}
