use bsl_rt::{BslValue, lookup_enum, lookup_member};

#[test]
fn reached_chart_label_location_matches_native_alias_and_presentation() {
    let russian = lookup_enum("ПоложениеПодписейКДиаграмме").unwrap();
    let english = lookup_enum("chartlabellocation").unwrap();
    assert_eq!(russian, english);
    let value = lookup_member(russian, "кРаЙаВтО").unwrap();
    assert_eq!(Some(value), lookup_member(english, "EdgeAuto"));
    let BslValue::Type(ty) = BslValue::Enum(value).type_of().unwrap() else {
        panic!("ожидался тип члена перечисления");
    };
    for name in ["ПоложениеПодписейКДиаграмме", "ChartLabelLocation"] {
        assert_eq!(bsl_rt::TypeId::lookup(name), ty.native());
    }
    let actual = format!(
        "enum.alias=YES\nenum.type={}\nenum.presentation={}\nenum.type.lookup=YES\ncompleted=YES",
        ty.name(),
        value.display_text()
    );
    let expected = include_str!(
        "../../../../docs/research/fixtures/chart-label-location-native-1015/platform.oracle"
    )
    .trim_start_matches('\u{feff}')
    .replace("\r\n", "\n");
    assert_eq!(actual, expected.trim());
}
