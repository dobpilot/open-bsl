use super::*;

#[test]
fn array_bounds_inside_load_handler_are_catchable_with_their_cause() {
    let factory = factory(include_str!(
        "../../../../../docs/research/fixtures/settings-storage-index-error-1019/manager.bsl"
    ));
    assert_handler_error_is_catchable(&factory, |error| {
        matches!(error, RtError::IndexOutOfBounds { index: 0, len: 0 })
    });
}
