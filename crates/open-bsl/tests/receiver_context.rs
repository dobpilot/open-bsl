use std::{cell::Cell, rc::Rc};

use open_bsl::{
    Arity, CallContext, Engine, FunctionCode, FunctionDescriptor, FunctionKind, GlobalDescriptor,
    LibraryDescriptor, MethodDescriptor, ModuleGraphRecipe, ModuleRecipe, ModuleVisibility,
    ObjectProtocol, RtResult, TypeDescriptor, Value, call_method_from_table,
};

static TYPE: TypeDescriptor = TypeDescriptor::new("receiver-test", "МенеджерТеста");
const TYPES: &[&TypeDescriptor] = &[&TYPE];

#[derive(Clone, Debug)]
struct Manager(Rc<Cell<i64>>);

impl ObjectProtocol for Manager {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &TYPE
    }
    fn method_table(&self) -> &'static [MethodDescriptor] {
        METHODS
    }
    fn call_method(
        &self,
        name: &str,
        args: &[Value],
        context: &mut CallContext<'_>,
    ) -> RtResult<Value> {
        call_method_from_table(METHODS, TYPE.name, self, name, args, context)
    }
}

fn own(receiver: &dyn ObjectProtocol, _: &[Value], _: &mut CallContext<'_>) -> RtResult<Value> {
    let manager = receiver.downcast_ref::<Manager>().unwrap();
    manager.0.set(manager.0.get() + 1);
    Ok(Value::number_from_i64(manager.0.get()))
}

fn invoke(
    receiver: &dyn ObjectProtocol,
    _: &[Value],
    context: &mut CallContext<'_>,
) -> RtResult<Value> {
    let manager = receiver.downcast_ref::<Manager>().unwrap();
    context
        .call_configuration_module(
            "Владелец",
            "Проверить",
            vec![],
            Some(Value::new_object(manager.clone())),
        )
        .map(|(value, _)| value)
}

const OWN: &[MethodDescriptor] = &[MethodDescriptor::new(&["СвойМетод"], Arity::exact(0), own)];
const METHODS: &[MethodDescriptor] = &[
    MethodDescriptor::new(&["СвойМетод"], Arity::exact(0), own),
    MethodDescriptor::new(&["Проверить"], Arity::exact(0), invoke),
];

fn make(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    Ok(Value::new_object(Manager(Rc::new(Cell::new(0)))))
}

const FUNCTIONS: &[FunctionDescriptor] = &[FunctionDescriptor {
    code: FunctionCode::new(1),
    names: &["ПолучитьМенеджерТеста"],
    arity: Arity::exact(0),
    kind: FunctionKind::Function,
    call: make,
}];
const GLOBALS: &[GlobalDescriptor] = &[GlobalDescriptor {
    names: &["МенеджерТеста"],
    function: FunctionCode::new(1),
}];

fn engine(source: &str) -> Result<Engine, open_bsl::Error> {
    Engine::builder()
        .register_library(
            LibraryDescriptor::new("receiver-test", "1")
                .with_types(TYPES)
                .with_functions(FUNCTIONS)
                .with_globals(GLOBALS),
        )
        .configuration(ModuleGraphRecipe {
            modules: vec![
                ModuleRecipe {
                    name: "Владелец".into(),
                    source: source.into(),
                    imports: vec![],
                    visibility: ModuleVisibility::Private,
                },
                ModuleRecipe {
                    name: "Глобальный".into(),
                    source: "Функция СвойМетод() Экспорт Возврат 999; КонецФункции".into(),
                    imports: vec![],
                    visibility: ModuleVisibility::Global,
                },
                ModuleRecipe {
                    name: "Другой".into(),
                    source: "Функция Проверить() Экспорт Возврат СвойМетод(); КонецФункции".into(),
                    imports: vec![],
                    visibility: ModuleVisibility::Explicit,
                },
            ],
            eager_init: false,
        })
        .configuration_method_context("Владелец", OWN)
        .build()
}

#[test]
fn own_receiver_methods_survive_nested_dynamic_code_and_precede_global_exports() {
    let engine = engine(
        r#"
        Функция Проверить() Экспорт
            Первый = СвойМетод();
            Второй = Вычислить("СвойМетод()");
            Третий = Вычислить("Вычислить(""СвойМетод()"")");
            Возврат Первый * 100 + Второй * 10 + Третий;
        КонецФункции
    "#,
    )
    .unwrap();
    let module = engine
        .compile_entry("Возврат МенеджерТеста.Проверить();")
        .unwrap();
    assert_eq!(
        engine.new_state().run(&module).unwrap(),
        Value::number_from_i64(123)
    );
    let other = engine.compile_entry("Возврат Другой.Проверить();").unwrap();
    assert_eq!(
        engine.new_state().run(&other).unwrap(),
        Value::number_from_i64(999)
    );
    assert!(engine.compile("Возврат СвойМетод();").is_err());
}

#[test]
fn ordinary_local_functions_keep_their_existing_priority() {
    let engine = engine("Функция СвойМетод() Возврат 17; КонецФункции Функция Проверить() Экспорт Возврат СвойМетод(); КонецФункции").unwrap();
    let module = engine
        .compile_entry("Возврат МенеджерТеста.Проверить();")
        .unwrap();
    assert_eq!(
        engine.new_state().run(&module).unwrap(),
        Value::number_from_i64(17)
    );
}

#[test]
fn declared_receiver_arity_is_checked_and_invalid_contexts_are_rejected() {
    let engine = engine("Функция Проверить() Экспорт Возврат СвойМетод(1); КонецФункции").unwrap();
    let module = engine
        .compile_entry("Возврат МенеджерТеста.Проверить();")
        .unwrap();
    let error = engine.new_state().run(&module).unwrap_err();
    assert!(error.to_string().contains("аргумент"), "{error}");
    assert!(
        Engine::builder()
            .configuration_method_context("Отсутствует", OWN)
            .build()
            .is_err()
    );
    assert!(
        Engine::builder()
            .common_module("Модуль", "")
            .configuration_method_context("Модуль", OWN)
            .configuration_method_context("модуль", OWN)
            .build()
            .is_err()
    );
}
