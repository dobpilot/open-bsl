use super::*;

#[test]
fn numeric_conversion_inside_load_handler_is_catchable_with_its_cause() {
    let factory = factory(include_str!(
        "../../../../../docs/research/fixtures/settings-storage-conversion-error-1017/manager.bsl"
    ));
    assert_handler_error_is_catchable(&factory, |error| matches!(error, RtError::Num(_)));
}
