use open_bsl::{BslNumber, Locale, NumberFormat, RtError, RtResult, parse_number};

#[test]
fn numeric_parser_uses_only_facade_names_for_format_result_and_errors() {
    let format = NumberFormat {
        locale: Locale::Ru,
        ..NumberFormat::default()
    };
    let parsed: RtResult<BslNumber> = parse_number("1,5", &format);
    assert_eq!(parsed.unwrap().to_string(), "1.5");
    assert!(matches!(
        parse_number("not a number", &format),
        Err(RtError::Num(_))
    ));
}
