use open_bsl::*;

fn current_mode(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    ctx.privileged_mode().map(Value::Boolean)
}

static FUNCTIONS: &[FunctionDescriptor] = &[FunctionDescriptor {
    code: FunctionCode::new(1),
    names: &["РежимВходаПробы"],
    arity: Arity::exact(0),
    kind: FunctionKind::Function,
    call: Some(current_mode),
}];

#[test]
fn explicit_handler_entry_is_available_through_the_facade_and_scoped_to_one_call() {
    let engine = Engine::builder()
        .register_library(LibraryDescriptor::new("entry-mode-probe", "1").with_functions(FUNCTIONS))
        .configuration(ModuleGraphRecipe {
            eager_init: false,
            modules: vec![ModuleRecipe {
                name: "ПроизвольныйМодуль".into(),
                visibility: ModuleVisibility::Private,
                imports: vec![],
                source: "Функция Проверить() Возврат РежимВходаПробы(); КонецФункции".into(),
            }],
        })
        .build()
        .unwrap();
    let mut state = engine.new_state();
    assert_eq!(
        state
            .call_configuration_handler_with_mode(
                "ПроизвольныйМодуль",
                "Проверить",
                vec![],
                ConfigurationHandlerEntryMode::Privileged,
            )
            .unwrap()
            .0,
        Value::Boolean(true)
    );
    assert_eq!(
        state
            .call_configuration_handler("ПроизвольныйМодуль", "Проверить", vec![])
            .unwrap()
            .0,
        Value::Boolean(false)
    );
}
