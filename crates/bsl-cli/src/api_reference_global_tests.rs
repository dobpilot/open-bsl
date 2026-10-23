use super::render;
use bsl_rt::*;

#[test]
fn globals_are_rendered_from_the_registered_descriptor() {
    fn get(_: &mut CallContext<'_>, _: &[BslValue]) -> RtResult<BslValue> {
        Ok(BslValue::Undefined)
    }
    const LIBRARY: LibraryDescriptor = LibraryDescriptor::new("global-reference", "1")
        .with_functions(&[FunctionDescriptor {
            code: FunctionCode::new(1),
            names: &["ПолучитьОбъект"],
            arity: Arity::exact(0),
            kind: FunctionKind::Function,
            call: Some(get),
        }])
        .with_globals(&[GlobalDescriptor {
            names: &["ОбъектХоста", "HostObject"],
            function: FunctionCode::new(1),
        }]);
    let mut builder = RuntimeBuilder::new();
    builder.register(core_library()).register(LIBRARY);
    let text = render(
        &builder.build().unwrap(),
        &crate::api_reference_sources::SourceIndex::default(),
    );
    assert!(text.contains("#### Глобальные свойства"));
    assert!(text.contains("`ОбъектХоста`, `HostObject`"));
    assert!(text.contains("| `ОбъектХоста`, `HostObject` | `ПолучитьОбъект` |"));
}
