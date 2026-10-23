use super::*;
use std::io::Write;

#[derive(Clone)]
struct Output(Rc<RefCell<Vec<u8>>>);

impl Write for Output {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.borrow_mut().extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn settings_reproduce_native_correspondence_identity_and_rejected_assignment_effects() {
    let engine = open_bsl::Engine::builder()
        .register_library(library())
        .build()
        .unwrap();
    for (source, expected) in [
        (
            include_str!(
                "../../../docs/research/fixtures/second-factor-template-global/setting-arity.bsl"
            ),
            include_str!(
                "../../../docs/research/fixtures/second-factor-template-global/setting-arity.platform.txt"
            ),
        ),
        (
            include_str!(
                "../../../docs/research/fixtures/second-factor-template-global/setting-map.bsl"
            ),
            include_str!(
                "../../../docs/research/fixtures/second-factor-template-global/setting-map.platform.txt"
            ),
        ),
        (
            include_str!(
                "../../../docs/research/fixtures/second-factor-template-global/setting-rejection.bsl"
            ),
            include_str!(
                "../../../docs/research/fixtures/second-factor-template-global/setting-rejection.platform.txt"
            ),
        ),
    ] {
        let output = Output(Rc::new(RefCell::new(Vec::new())));
        let mut state = engine.state_builder().stdout(output.clone()).build();
        state.exec(source).unwrap();
        assert_eq!(
            String::from_utf8(output.0.borrow().clone()).unwrap(),
            expected
        );
    }
}

#[test]
fn setting_constructor_keeps_finite_arity_and_property_aliases() {
    let engine = open_bsl::Engine::builder()
        .register_library(library())
        .build()
        .unwrap();
    assert!(
        engine
            .compile("Возврат Новый SecondAuthenticationFactorSetting(1);")
            .is_err()
    );
    assert_eq!(engine.new_state().exec(
        "Настройка = Новый SecondAuthenticationFactorSetting; Настройка.SettingTemplateName = \"контроль\"; Параметры = Новый Соответствие; Параметры.Вставить(\"значение\", 23); Настройка.Parameters = Параметры; Возврат Настройка.ИмяШаблонаНастройки + \":\" + Строка(Настройка.Параметры[\"значение\"]);"
    ).unwrap(), Value::Str("контроль:23".into()));
}
