use super::*;

#[test]
fn missing_property_inside_load_handler_is_catchable_with_its_cause() {
    let factory = factory(include_str!(
        "../../../../../docs/research/fixtures/settings-storage-property-error-1017/manager.bsl"
    ));
    assert_handler_error_is_catchable(&factory, |error| matches!(error, RtError::UnknownField(_)));
}
