use crate::*;

fn state() -> open_bsl::State {
    state_with_output(std::io::sink())
}

fn state_with_output(output: impl std::io::Write + 'static) -> open_bsl::State {
    let mut builder = open_bsl::Engine::builder()
        .register_library(library())
        .configuration_method_context(
            "КонтрольОбмена",
            MetadataKind::ExchangePlan.receiver_methods(),
        )
        .common_module(
            "КонтрольОбмена",
            &format!(
                "{}\nФункция НеСоздавать() Экспорт Возврат 23; КонецФункции
                 Функция СоздатьПозже() Экспорт Возврат СоздатьУзел(); КонецФункции
                 Функция ТекущийУзелПозже() Экспорт Возврат ЭтотУзел(); КонецФункции
                 Функция СсылкаПозже(Идентификатор) Экспорт Возврат ПолучитьСсылку(Идентификатор); КонецФункции",
                include_str!("../../../docs/research/fixtures/exchange-plan-session-reference-1016/input-xml/ExchangePlans/ЗамерОбмена/Ext/ManagerModule.bsl")
            ),
        );
    let catalog = Arc::new(
        MetadataCatalog::new(
            "configuration",
            vec![
                MetadataObject::new("plan", "Контроль", MetadataKind::ExchangePlan)
                    .with_manager_module("КонтрольОбмена"),
                MetadataObject::new("native-plan", "ЗамерОбмена", MetadataKind::ExchangePlan)
                    .with_manager_module("КонтрольОбмена"),
                MetadataObject::new("other-plan", "ДругойОбмен", MetadataKind::ExchangePlan),
            ],
        )
        .unwrap(),
    );
    for ty in catalog.transferable_value_types() {
        builder = builder.register_value_type(ty);
    }
    let engine = builder.build().unwrap();
    engine
        .state_builder()
        .stdout(output)
        .host_data(Rc::new(SessionData::new(
            catalog,
            Principal::new("user", "Тест", vec![]),
        )))
        .build()
}

#[derive(Clone, Default)]
struct Output(Rc<RefCell<Vec<u8>>>);

impl std::io::Write for Output {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.borrow_mut().extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn empty_exchange_plan_references_match_the_complete_native_control() {
    let output = Output::default();
    let mut state = state_with_output(output.clone());
    state
        .exec(include_str!(
            "../../../docs/research/fixtures/exchange-plan-session-reference-1016/measure.bsl"
        ))
        .unwrap();
    let actual = String::from_utf8(output.0.borrow().clone()).unwrap();
    assert_eq!(actual.replace("\r\n", "\n"), include_str!("../../../docs/research/fixtures/exchange-plan-session-reference-1016/measure.platform.txt").replace("\r\n", "\n"));
}

#[test]
fn constructed_exchange_plan_references_match_the_complete_native_control() {
    let output = Output::default();
    let mut state = state_with_output(output.clone());
    state.exec(include_str!("../../../docs/research/fixtures/exchange-plan-session-reference-1016/measure-get-ref.bsl")).unwrap();
    let actual = String::from_utf8(output.0.borrow().clone()).unwrap();
    assert_eq!(actual.replace("\r\n", "\n"), include_str!("../../../docs/research/fixtures/exchange-plan-session-reference-1016/measure-get-ref.platform.txt").replace("\r\n", "\n"));
    assert_eq!(state.eval("ПланыОбмена.ЗамерОбмена.СсылкаПозже(Новый УникальныйИдентификатор(\"00000000-0000-0000-0000-000000000000\")) = ПланыОбмена.ЗамерОбмена.ПустаяСсылка()").unwrap(), Value::Boolean(true));
}

#[test]
fn current_node_is_explicitly_unavailable_in_both_aliases_and_implicit_calls() {
    let mut state = state();
    for expression in [
        "ПланыОбмена.Контроль.ЭтотУзел()",
        "ExchangePlans.Контроль.ThisNode()",
        "ПланыОбмена.Контроль.ТекущийУзелПозже()",
    ] {
        assert!(
            state
                .eval(expression)
                .unwrap_err()
                .to_string()
                .contains("текущий узел плана обмена не приобретён")
        );
    }
}

#[test]
fn dormant_node_creation_compiles_in_its_real_manager_context() {
    assert_eq!(
        state()
            .exec("Возврат ПланыОбмена.Контроль.НеСоздавать();")
            .unwrap(),
        Value::number_from_i64(23)
    );
}

#[test]
fn node_creation_fails_explicitly_through_both_aliases_and_implicit_receiver() {
    let mut state = state();
    for expression in [
        "ПланыОбмена.Контроль.СоздатьУзел()",
        "ExchangePlans.Контроль.CreateNode()",
        "ПланыОбмена.Контроль.СоздатьПозже()",
    ] {
        assert!(
            state
                .eval(expression)
                .unwrap_err()
                .to_string()
                .contains("создание узла плана обмена не поддерживается")
        );
    }
}

#[test]
fn node_creation_keeps_its_zero_argument_signature() {
    let mut state = state();
    assert!(
        !state
            .eval("ПланыОбмена.Контроль.СоздатьУзел(1)")
            .unwrap_err()
            .to_string()
            .contains("создание узла плана обмена не поддерживается")
    );
}
