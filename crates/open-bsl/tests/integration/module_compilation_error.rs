use std::{cell::RefCell, rc::Rc};

use open_bsl::{
    Arity, CallContext, Engine, Error, FunctionCode, FunctionDescriptor, FunctionKind,
    LibraryDescriptor, ModuleGraphRecipe, ModuleRecipe, ModuleVisibility, RtError, RtResult, Value,
};

const DIAGNOSTIC: &str = "переменная «СовсемНеизвестноеИмя» читается до присваивания";

fn fail(_context: &mut CallContext<'_>, _args: &[Value]) -> RtResult<Value> {
    Err(RtError::ModuleCompilation(DIAGNOSTIC.into()))
}

fn record(context: &mut CallContext<'_>, args: &[Value]) -> RtResult<Value> {
    let [Value::Str(marker)] = args else {
        panic!("проба принимает один строковый маркер");
    };
    context
        .host_data()?
        .downcast_ref::<RefCell<Vec<String>>>()
        .unwrap()
        .borrow_mut()
        .push(marker.to_string());
    Ok(Value::Undefined)
}

const FUNCTIONS: &[FunctionDescriptor] = &[
    FunctionDescriptor {
        code: FunctionCode::new(1),
        names: &["СбойКомпиляции"],
        arity: Arity::exact(0),
        kind: FunctionKind::Procedure,
        call: Some(fail),
    },
    FunctionDescriptor {
        code: FunctionCode::new(2),
        names: &["Отметить"],
        arity: Arity::exact(1),
        kind: FunctionKind::Procedure,
        call: Some(record),
    },
];

#[test]
fn module_compilation_error_bypasses_nested_handlers_and_preserves_other_states() {
    let common = |name: &str, source: &str| ModuleRecipe {
        name: name.into(),
        source: source.into(),
        imports: Vec::new(),
        visibility: ModuleVisibility::Named,
    };
    let engine = Engine::builder()
        .register_library(
            LibraryDescriptor::new("compilation-error-probe", "1").with_functions(FUNCTIONS),
        )
        .configuration(ModuleGraphRecipe {
            modules: vec![
                common(
                    "Дочерний",
                    r#"
            Функция Ответ() Экспорт
                Отметить("export");
                Возврат 24;
            КонецФункции
            Отметить("before");
            СбойКомпиляции();
            Отметить("after");
            "#,
                ),
                common(
                    "Родитель",
                    "Функция Исправный() Экспорт Возврат 42; КонецФункции
             Функция Обратиться() Экспорт Возврат Дочерний.Ответ(); КонецФункции",
                ),
            ],
            eager_init: false,
        })
        .build()
        .unwrap();
    let good = engine
        .compile_entry("Возврат Родитель.Исправный();")
        .unwrap();
    let bad = engine
        .compile_entry(
            r#"
            Попытка
                Попытка
                    Родитель.Обратиться();
                Исключение
                    Отметить("inner-handler");
                КонецПопытки;
            Исключение
                Отметить("outer-handler");
            КонецПопытки;
            Отметить("after-call");
            "#,
        )
        .unwrap();
    let trace = Rc::new(RefCell::new(Vec::<String>::new()));
    let mut state = engine.state_builder().host_data(trace.clone()).build();
    assert_eq!(state.run(&good).unwrap(), Value::number_from_i64(42));
    assert!(trace.borrow().is_empty());
    let error = state.run(&bad).unwrap_err();
    assert!(matches!(
        error,
        Error::Runtime(RtError::ModuleCompilation(ref message)) if message == DIAGNOSTIC
    ));
    assert_eq!(*trace.borrow(), ["before"]);

    let other_trace = Rc::new(RefCell::new(Vec::<String>::new()));
    let mut other = engine
        .state_builder()
        .host_data(other_trace.clone())
        .build();
    assert_eq!(other.run(&good).unwrap(), Value::number_from_i64(42));
    assert!(other_trace.borrow().is_empty());
}

#[test]
fn caller_and_argument_run_before_fatal_child_initialization() {
    let common = |name: &str, source: &str| ModuleRecipe {
        name: name.into(),
        source: source.into(),
        imports: Vec::new(),
        visibility: ModuleVisibility::Named,
    };
    let engine = Engine::builder()
        .register_library(
            LibraryDescriptor::new("compilation-error-probe", "1").with_functions(FUNCTIONS),
        )
        .configuration(ModuleGraphRecipe {
            modules: vec![
                common(
                    "Дочерний",
                    "Функция Ответ(Аргумент) Экспорт
                        Отметить(\"child-body\"); Возврат Аргумент;
                     КонецФункции
                     СбойКомпиляции();",
                ),
                common(
                    "Родитель",
                    "Функция Аргумент()
                        Отметить(\"argument\"); Возврат 7;
                     КонецФункции
                     Функция Обратиться() Экспорт
                        Отметить(\"caller\");
                        Возврат Дочерний.Ответ(Аргумент());
                     КонецФункции",
                ),
            ],
            eager_init: false,
        })
        .build()
        .unwrap();
    let module = engine
        .compile_entry(
            "Попытка Родитель.Обратиться();
             Исключение Отметить(\"handler\"); КонецПопытки;
             Отметить(\"after\");",
        )
        .unwrap();
    let trace = Rc::new(RefCell::new(Vec::<String>::new()));
    let mut state = engine.state_builder().host_data(trace.clone()).build();
    assert!(matches!(
        state.run(&module),
        Err(Error::Runtime(RtError::ModuleCompilation(ref message))) if message == DIAGNOSTIC
    ));
    assert_eq!(*trace.borrow(), ["caller", "argument"]);
}
