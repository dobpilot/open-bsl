use open_bsl::{
    Arity, CallContext, ConfigurationHandlerEntryMode, Engine, Error, FunctionCode,
    FunctionDescriptor, FunctionKind, LibraryDescriptor, ModuleGraphRecipe, ModuleRecipe,
    ModuleVisibility, RtError, RtResult, Value,
};
use std::rc::Rc;

fn fail(context: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    Err(context
        .host_data()?
        .downcast_ref::<RtError>()
        .unwrap()
        .clone())
}

const FUNCTIONS: &[FunctionDescriptor] = &[FunctionDescriptor {
    code: FunctionCode::new(1),
    names: &["СбойПробы"],
    arity: Arity::exact(0),
    kind: FunctionKind::Procedure,
    call: Some(fail),
}];

fn engine(source: &str) -> Engine {
    Engine::builder()
        .register_library(
            LibraryDescriptor::new("handler-policy-probe", "1").with_functions(FUNCTIONS),
        )
        .configuration(ModuleGraphRecipe {
            eager_init: false,
            modules: vec![ModuleRecipe {
                name: "Контекст".into(),
                source: source.into(),
                imports: Vec::new(),
                visibility: ModuleVisibility::Named,
            }],
        })
        .build()
        .unwrap()
}

#[test]
fn critical_entry_preserves_preexisting_noncatchable_error_classes_in_both_phases() {
    for initialization in [false, true] {
        for fault in [
            RtError::Canceled,
            RtError::ModuleCompilation("compile-probe".into()),
            RtError::InvalidBytecode("image-probe"),
        ] {
            let source = if initialization {
                "Процедура Обработчик() КонецПроцедуры СбойПробы();"
            } else {
                "Процедура Обработчик() СбойПробы(); КонецПроцедуры"
            };
            let engine = engine(source);
            let mut state = engine
                .state_builder()
                .host_data(Rc::new(fault.clone()))
                .build();
            let error = state
                .call_configuration_handler(
                    "Контекст",
                    "Обработчик",
                    vec![],
                    ConfigurationHandlerEntryMode::Critical("Проба"),
                )
                .unwrap_err();
            assert!(matches!(error, Error::Runtime(actual) if actual == fault));
            assert_eq!(
                state.exec("Возврат 42;").unwrap(),
                Value::number_from_i64(42)
            );
        }
    }
}

#[test]
fn inherited_entry_keeps_ordinary_raised_errors_unchanged_in_both_phases() {
    for source in [
        "Процедура Обработчик() КонецПроцедуры СбойПробы();",
        "Процедура Обработчик() СбойПробы(); КонецПроцедуры",
    ] {
        let engine = engine(source);
        let fault = RtError::Raised(Value::Str("ordinary-probe".into()));
        let mut state = engine
            .state_builder()
            .host_data(Rc::new(fault.clone()))
            .build();
        let error = state
            .call_configuration_handler(
                "Контекст",
                "Обработчик",
                vec![],
                open_bsl::ConfigurationHandlerEntryMode::Inherit,
            )
            .unwrap_err();
        assert!(matches!(error, Error::Runtime(actual) if actual == fault));
        assert_eq!(
            state.exec("Возврат 42;").unwrap(),
            Value::number_from_i64(42)
        );
    }
}

#[test]
fn exception_caught_inside_critical_handler_does_not_escape_or_terminate_the_state() {
    let engine = engine(
        "Функция Обработчик() Попытка СбойПробы(); Исключение Возврат 42; КонецПопытки; Возврат 0; КонецФункции",
    );
    let mut state = engine
        .state_builder()
        .host_data(Rc::new(RtError::Raised(Value::Str("caught-probe".into()))))
        .build();
    let result = state
        .call_configuration_handler(
            "Контекст",
            "Обработчик",
            vec![],
            ConfigurationHandlerEntryMode::Critical("Проба"),
        )
        .unwrap();
    assert_eq!(result.0, Value::number_from_i64(42));
    assert_eq!(
        state.exec("Возврат 42;").unwrap(),
        Value::number_from_i64(42)
    );
}

#[test]
fn critical_host_entry_preserves_errors_from_target_preparation() {
    // Произвольное тело допустимо в рецепте модуля хоста. Это не эталон
    // исходного модуля менеджера 1С: там такие операторы отвергаются.
    let engine = engine("Процедура Обработчик() КонецПроцедуры СбойПробы();");
    let fault = RtError::Raised(Value::Str("preparation-probe".into()));
    let mut state = engine
        .state_builder()
        .host_data(Rc::new(fault.clone()))
        .build();
    let error = state
        .call_configuration_handler(
            "Контекст",
            "Обработчик",
            vec![],
            ConfigurationHandlerEntryMode::Critical("Проба"),
        )
        .unwrap_err();
    assert!(matches!(error, Error::Runtime(actual) if actual == fault));
    assert_eq!(
        state.exec("Возврат 42;").unwrap(),
        Value::number_from_i64(42)
    );
}
