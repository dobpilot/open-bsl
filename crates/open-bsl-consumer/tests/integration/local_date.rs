use open_bsl::{BslDate, FixedTimeZone, local_date_from_utc_seconds};

#[test]
fn host_start_date_conversion_needs_only_the_facade() {
    let zone = FixedTimeZone::new(3600).unwrap();
    let actual = local_date_from_utc_seconds(0, "начало сеанса", &zone).unwrap();
    assert_eq!(actual, BslDate::from_civil(1970, 1, 1, 1, 0, 0).unwrap());
}
