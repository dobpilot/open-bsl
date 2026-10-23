use open_bsl::{
    Arity, CallContext, Engine, Error, ExecutionPoll, FunctionCode, FunctionDescriptor,
    FunctionKind, LibraryDescriptor, ModuleGraphRecipe, ModuleRecipe, ModuleVisibility, RtError,
    RtResult, State, Value,
};
use std::{cell::Cell, rc::Rc};

fn fail(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    Err(RtError::CriticalConfiguration {
        operation: "Проба",
        cause: Box::new(RtError::Raised(Value::Str("critical-probe".into()))),
    })
}

fn tick(context: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    let count = context.host_data()?.downcast_ref::<Cell<usize>>().unwrap();
    count.set(count.get() + 1);
    Ok(Value::number_from_i64(count.get() as i64))
}

const FUNCTIONS: &[FunctionDescriptor] = &[
    FunctionDescriptor {
        code: FunctionCode::new(1),
        names: &["КритическаяПроба"],
        arity: Arity::exact(0),
        kind: FunctionKind::Procedure,
        call: Some(fail),
    },
    FunctionDescriptor {
        code: FunctionCode::new(2),
        names: &["ОтметитьПробу"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(tick),
    },
];

fn engine() -> Engine {
    Engine::builder()
        .register_library(LibraryDescriptor::new("critical-probe", "1").with_functions(FUNCTIONS))
        .configuration(ModuleGraphRecipe {
            eager_init: false,
            modules: vec![ModuleRecipe {
                name: "Контекст".into(),
                source: "Процедура Сбой() Экспорт КритическаяПроба(); КонецПроцедуры Функция Хорошо() Экспорт Возврат ОтметитьПробу(); КонецФункции".into(),
                imports: Vec::new(),
                visibility: ModuleVisibility::Named,
            }],
        })
        .build()
        .unwrap()
}

fn critical(error: &Error) {
    let Error::Runtime(RtError::CriticalConfiguration { operation, cause }) = error else {
        panic!("ожидалась критическая ошибка, получено {error:?}");
    };
    assert_eq!(*operation, "Проба");
    assert!(
        matches!(cause.as_ref(), RtError::Raised(Value::Str(text)) if text.to_string() == "critical-probe")
    );
    assert!(std::error::Error::source(error).unwrap().source().is_some());
}

fn cannot_reenter(state: &mut State, engine: &Engine, count: &Cell<usize>) {
    let module = engine.compile("Возврат ОтметитьПробу();").unwrap();
    critical(&state.run(&module).unwrap_err());
    critical(&state.start(&module).err().unwrap());
    critical(&state.instantiate(&module).err().unwrap());
    critical(&state.exec("Возврат ОтметитьПробу();").unwrap_err());
    critical(&state.exec("Если").unwrap_err());
    critical(&state.eval("ОтметитьПробу()").unwrap_err());
    critical(
        &state
            .call_configuration_handler(
                "Контекст",
                "Хорошо",
                vec![],
                open_bsl::ConfigurationHandlerEntryMode::Inherit,
            )
            .unwrap_err(),
    );
    assert_eq!(count.get(), 0);
}

#[test]
fn critical_failure_skips_nested_catches_and_rejects_every_later_entry() {
    let engine = engine();
    let count = Rc::new(Cell::new(0_usize));
    let mut state = engine.state_builder().host_data(count.clone()).build();
    let good = engine
        .compile("Функция Хорошо() Экспорт Возврат ОтметитьПробу(); КонецФункции")
        .unwrap();
    let instance = state.instantiate(&good).unwrap();
    let bad = engine.compile("Попытка Попытка КритическаяПроба(); Исключение ОтметитьПробу(); КонецПопытки; Исключение ОтметитьПробу(); КонецПопытки; ОтметитьПробу();").unwrap();
    critical(&state.run(&bad).unwrap_err());
    critical(&instance.call(&mut state, "Хорошо", vec![]).unwrap_err());
    cannot_reenter(&mut state, &engine, &count);

    let other_count = Rc::new(Cell::new(0_usize));
    let mut other = engine
        .state_builder()
        .host_data(other_count.clone())
        .build();
    assert_eq!(
        other.exec("Возврат ОтметитьПробу();").unwrap(),
        Value::number_from_i64(1)
    );
    assert_eq!(other_count.get(), 1);
    assert_eq!(count.get(), 0);
}

#[test]
fn critical_failure_from_initialization_export_and_configuration_handler_is_terminal() {
    let engine = engine();
    for entry in 0..3 {
        let count = Rc::new(Cell::new(0_usize));
        let mut state = engine.state_builder().host_data(count.clone()).build();
        let error = match entry {
            0 => {
                let module = engine.compile("КритическаяПроба();").unwrap();
                state.instantiate(&module).err().unwrap()
            }
            1 => {
                let module = engine
                    .compile("Процедура Сбой() Экспорт КритическаяПроба(); КонецПроцедуры")
                    .unwrap();
                let instance = state.instantiate(&module).unwrap();
                instance.call(&mut state, "Сбой", vec![]).unwrap_err()
            }
            _ => state
                .call_configuration_handler(
                    "Контекст",
                    "Сбой",
                    vec![],
                    open_bsl::ConfigurationHandlerEntryMode::Inherit,
                )
                .unwrap_err(),
        };
        critical(&error);
        cannot_reenter(&mut state, &engine, &count);
    }
}

#[test]
fn pollable_and_async_critical_failures_are_terminal_too() {
    let engine = engine();
    for source in [
        "Попытка КритическаяПроба(); Исключение ОтметитьПробу(); КонецПопытки;",
        r#"Попытка Выполнить("КритическаяПроба();"); Исключение ОтметитьПробу(); КонецПопытки;"#,
        "Асинх Процедура Сбой() Попытка КритическаяПроба(); Исключение ОтметитьПробу(); КонецПопытки; КонецПроцедуры Сбой();",
    ] {
        let count = Rc::new(Cell::new(0_usize));
        let mut state = engine.state_builder().host_data(count.clone()).build();
        let module = engine.compile(source).unwrap();
        let mut execution = state.start(&module).unwrap();
        assert_eq!(execution.poll(0).unwrap(), ExecutionPoll::Runnable);
        loop {
            match execution.poll(1) {
                Err(error) => {
                    critical(&error);
                    break;
                }
                Ok(ExecutionPoll::Runnable | ExecutionPoll::Waiting) => {}
                Ok(ExecutionPoll::Complete(_)) => {
                    panic!("критический сбой не может завершиться успешно")
                }
            }
        }
        critical(&execution.poll(0).unwrap_err());
        drop(execution);
        cannot_reenter(&mut state, &engine, &count);
    }
}

#[test]
fn ordinary_raised_errors_remain_catchable_and_do_not_disable_state() {
    let engine = engine();
    let count = Rc::new(Cell::new(0_usize));
    let mut state = engine.state_builder().host_data(count.clone()).build();
    assert!(state.exec("ВызватьИсключение \"обычная\";").is_err());
    assert_eq!(state.exec("Попытка ВызватьИсключение \"обычная\"; Исключение Возврат ОтметитьПробу(); КонецПопытки;").unwrap(), Value::number_from_i64(1));
    assert_eq!(
        state.exec("Возврат ОтметитьПробу();").unwrap(),
        Value::number_from_i64(2)
    );
}
