use open_bsl::{Engine, ModuleGraphRecipe, ModuleRecipe, ModuleVisibility};
use std::{cell::RefCell, io, io::Write, rc::Rc};

#[derive(Clone)]
struct Output(Rc<RefCell<Vec<u8>>>);

impl Write for Output {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.borrow_mut().extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn the_native_image_control_source_matches_the_platform_receipt() {
    let engine = Engine::builder()
        .configuration(ModuleGraphRecipe {
            modules: vec![ModuleRecipe {
                name: "ЗамерыТранзитивногоРодителя".into(),
                source: include_str!(
                    "../../../tests/conformance/measure/1c/native-image-control-module.bsl"
                )
                .into(),
                imports: vec![],
                visibility: ModuleVisibility::Named,
            }],
            eager_init: false,
        })
        .build()
        .unwrap();
    let output = Output(Rc::new(RefCell::new(Vec::new())));
    let mut state = engine.state_builder().stdout(output.clone()).build();
    let module = engine
        .compile_entry(include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image.bsl"
        ))
        .unwrap();
    state.run(&module).unwrap();
    let actual = String::from_utf8(output.0.borrow().clone()).unwrap();
    let expected =
        include_str!("../../../tests/conformance/measure/1c/measure-native-image.platform.txt");
    assert_eq!(actual.trim(), expected.trim());
}
