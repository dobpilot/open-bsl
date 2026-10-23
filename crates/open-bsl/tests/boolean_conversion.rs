use std::io::Write;
use std::rc::Rc;
use std::sync::Mutex;

use open_bsl::Engine;

#[derive(Clone, Default)]
struct Capture(Rc<Mutex<Vec<u8>>>);

impl Write for Capture {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn boolean_conversion_matches_the_platform_measurement() {
    let engine = Engine::builder().build().unwrap();
    let module = engine
        .compile(include_str!(
            "../../../tests/conformance/measure/1c/measure-boolean-conversion.bsl"
        ))
        .unwrap();
    let capture = Capture::default();
    engine
        .state_builder()
        .stdout(capture.clone())
        .build()
        .run(&module)
        .unwrap();
    let actual = String::from_utf8(capture.0.lock().unwrap().clone()).unwrap();
    assert_eq!(
        actual,
        include_str!(
            "../../../tests/conformance/measure/1c/measure-boolean-conversion.platform.txt"
        )
    );
}
