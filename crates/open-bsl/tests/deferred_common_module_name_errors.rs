use open_bsl::{
    Arity, CallContext, Engine, Error, FunctionCode, FunctionDescriptor, FunctionKind,
    LibraryDescriptor, ModuleGraphRecipe, ModuleRecipe, ModuleVisibility, RtError, RtResult, Value,
};
use std::{cell::RefCell, rc::Rc};

fn record(context: &mut CallContext<'_>, args: &[Value]) -> RtResult<Value> {
    let [Value::Str(marker)] = args else {
        panic!("строковый маркер");
    };
    context
        .host_data()?
        .downcast_ref::<RefCell<Vec<String>>>()
        .unwrap()
        .borrow_mut()
        .push(marker.to_string());
    Ok(Value::Undefined)
}

const FUNCTIONS: &[FunctionDescriptor] = &[FunctionDescriptor {
    code: FunctionCode::new(1),
    names: &["Отметить"],
    arity: Arity::exact(1),
    kind: FunctionKind::Procedure,
    call: Some(record),
}];
const PROBE: LibraryDescriptor =
    LibraryDescriptor::new("deferred-error-probe", "1").with_functions(FUNCTIONS);

fn module(name: &str, source: &str, visibility: ModuleVisibility) -> ModuleRecipe {
    ModuleRecipe {
        name: name.into(),
        source: source.into(),
        imports: vec![],
        visibility,
    }
}

fn faulty(visibility: ModuleVisibility) -> ModuleRecipe {
    module(
        "Дочерний",
        "Функция Исправный(А) Экспорт Отметить(\"child\"); Возврат А; КонецФункции
         Функция Ошибка() Экспорт Возврат СовсемНеизвестноеИмя; КонецФункции",
        visibility,
    )
}

fn recipe() -> ModuleGraphRecipe {
    ModuleGraphRecipe {
        modules: vec![
            faulty(ModuleVisibility::Named),
            module(
                "След",
                "Перем Значение Экспорт; Значение = \"\";
                 Процедура Отметить(Метка) Экспорт Значение = Значение + Метка; КонецПроцедуры",
                ModuleVisibility::Named,
            ),
            module(
                "Родитель",
                "Функция Исправный() Экспорт Возврат 42; КонецФункции
                 Функция Аргумент() Отметить(\"argument\"); Возврат 7; КонецФункции
                 Функция Обратиться() Экспорт
                    Отметить(\"caller\");
                    Возврат Дочерний.Исправный(Аргумент());
                 КонецФункции",
                ModuleVisibility::Named,
            ),
        ],
        eager_init: false,
    }
}

#[test]
fn default_and_non_common_modules_keep_strict_name_diagnostics() {
    assert!(
        Engine::builder()
            .register_library(PROBE)
            .configuration(recipe())
            .build()
            .is_err()
    );
    for visibility in [ModuleVisibility::Private, ModuleVisibility::Explicit] {
        assert!(
            Engine::builder()
                .register_library(PROBE)
                .defer_common_module_name_errors(true)
                .configuration(ModuleGraphRecipe {
                    modules: vec![module(
                        "Строгий",
                        "Функция Ошибка() Экспорт Возврат СовсемНеизвестноеИмя; КонецФункции",
                        visibility,
                    )],
                    eager_init: false,
                })
                .build()
                .is_err()
        );
    }
}

#[test]
fn unused_child_is_lazy_and_access_fails_after_caller_and_argument() {
    for (rooted, visibility) in [
        (false, ModuleVisibility::Named),
        (true, ModuleVisibility::Named),
        (false, ModuleVisibility::Global),
        (true, ModuleVisibility::Global),
    ] {
        let mut recipe = recipe();
        recipe.modules[0].visibility = visibility;
        let mut builder = Engine::builder()
            .register_library(PROBE)
            .defer_common_module_name_errors(true)
            .configuration(recipe);
        if rooted {
            builder = builder.configuration_roots(["Родитель"]);
        }
        let engine = builder.build().unwrap();
        let good = engine
            .compile_entry("Возврат Родитель.Исправный();")
            .unwrap();
        let bad = engine
            .compile_entry(
                "Попытка
                    Попытка Родитель.Обратиться();
                    Исключение Отметить(\"inner\"); КонецПопытки;
                 Исключение Отметить(\"outer\"); КонецПопытки;
                 Отметить(\"after\");",
            )
            .unwrap();
        let trace = Rc::new(RefCell::new(Vec::<String>::new()));
        let mut state = engine.state_builder().host_data(trace.clone()).build();
        assert_eq!(state.run(&good).unwrap(), Value::number_from_i64(42));
        assert!(matches!(
            state.run(&bad),
            Err(Error::Runtime(RtError::ModuleCompilation(ref message)))
                if message.contains("СовсемНеизвестноеИмя")
        ));
        assert_eq!(*trace.borrow(), ["caller", "argument"]);
        assert_eq!(
            engine.new_state().run(&good).unwrap(),
            Value::number_from_i64(42)
        );
    }
}

#[test]
fn top_level_name_failure_prevents_the_entire_common_module_from_running() {
    let engine = Engine::builder()
        .register_library(PROBE)
        .defer_common_module_name_errors(true)
        .configuration(ModuleGraphRecipe {
            modules: vec![module(
                "Ошибка",
                "Функция Исправный() Экспорт Отметить(\"export\"); Возврат 42; КонецФункции
                 Отметить(\"before\"); Значение = СовсемНеизвестноеИмя;",
                ModuleVisibility::Named,
            )],
            eager_init: false,
        })
        .build()
        .unwrap();
    let module = engine.compile_entry("Возврат Ошибка.Исправный();").unwrap();
    let trace = Rc::new(RefCell::new(Vec::<String>::new()));
    let mut state = engine.state_builder().host_data(trace.clone()).build();
    assert!(matches!(
        state.run(&module),
        Err(Error::Runtime(RtError::ModuleCompilation(_)))
    ));
    assert!(trace.borrow().is_empty());
}
