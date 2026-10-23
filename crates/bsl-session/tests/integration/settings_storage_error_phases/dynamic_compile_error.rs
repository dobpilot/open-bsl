use super::*;

#[test]
fn expression_compilation_inside_the_handler_is_a_catchable_execution_error() {
    let factory = factory(include_str!(
        "../../../../../docs/research/fixtures/settings-storage-dynamic-compile-error-1018/manager.bsl"
    ));
    assert_handler_error_is_catchable(&factory, |error| {
        matches!(error, RtError::DynamicError(_))
            && error.to_string().contains("НеСуществующаяФункцияПробы")
    });
}
