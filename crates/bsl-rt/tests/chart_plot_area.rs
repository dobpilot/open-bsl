use bsl_rt::{BslValue, TypeId, lookup_enum, lookup_member};
#[test]
fn reached_plot_area_matches_native_aliases_and_type() {
    let kind = lookup_enum("РасположениеОбластиПостроенияДиаграммы").unwrap();
    assert_eq!(Some(kind), lookup_enum("chartplotareaplacement"));
    let value = lookup_member(kind, "уКаЗыВаЕтСяРаСпОлОжЕнИе").unwrap();
    assert_eq!(Some(value), lookup_member(kind, "UseCoordinates"));
    assert_eq!(value.display_text(), "Use coordinates");
    let BslValue::Type(ty) = BslValue::Enum(value).type_of().unwrap() else {
        panic!("ожидался тип перечисления");
    };
    for name in [
        "РасположениеОбластиПостроенияДиаграммы",
        "ChartPlotAreaPlacement",
    ] {
        assert_eq!(TypeId::lookup(name), ty.native());
    }
    assert_eq!(ty.name(), "ChartPlotAreaPlacement");
    let BslValue::Type(meta) = BslValue::EnumType(kind).type_of().unwrap() else {
        panic!("ожидался метатип перечисления");
    };
    assert_eq!(meta.name(), "EnumChartPlotAreaPlacement");
}
