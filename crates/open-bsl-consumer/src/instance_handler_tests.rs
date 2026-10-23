use open_bsl::*;
use std::cell::RefCell;
use std::rc::Rc;

const MODULE: &str = "ОбъектОбработчика";
static TYPE: TypeDescriptor = TypeDescriptor::new("handler-probe", "ОбъектОбработчика");

#[derive(Debug, Clone)]
struct Probe {
    instance: Rc<ConfigurationInstance>,
    mark: Rc<RefCell<Value>>,
}
impl ObjectProtocol for Probe {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &TYPE
    }
    fn configuration_instance(&self) -> Option<&ConfigurationInstance> {
        Some(&self.instance)
    }
    fn property_table(&self) -> &'static [PropertyDescriptor] {
        PROPERTIES
    }
    fn has_method(&self, _: &str) -> bool {
        true
    }
    fn call_method(
        &self,
        name: &str,
        args: &[Value],
        ctx: &mut CallContext<'_>,
    ) -> RtResult<Value> {
        ctx.call_configuration_instance(
            MODULE,
            &self.instance,
            name,
            args.to_vec(),
            Value::new_object(self.clone()),
        )
        .map(|(value, _)| value)
    }
}
fn probe(value: &Value) -> RtResult<&Probe> {
    value
        .object_ref()
        .and_then(|object| object.downcast_ref::<Probe>())
        .ok_or(RtError::NotAnObject)
}
fn mark(receiver: &dyn ObjectProtocol, _: &mut CallContext<'_>) -> RtResult<Value> {
    Ok(receiver
        .downcast_ref::<Probe>()
        .ok_or(RtError::NotAnObject)?
        .mark
        .borrow()
        .clone())
}
fn set_mark(receiver: &dyn ObjectProtocol, value: Value, _: &mut CallContext<'_>) -> RtResult<()> {
    *receiver
        .downcast_ref::<Probe>()
        .ok_or(RtError::NotAnObject)?
        .mark
        .borrow_mut() = value;
    Ok(())
}
static PROPERTIES: &[PropertyDescriptor] = &[PropertyDescriptor {
    names: &["Метка"],
    get: mark,
    set: Some(set_mark),
}];
fn create(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    let object = Probe {
        instance: Rc::new(ConfigurationInstance::new()),
        mark: Rc::new(RefCell::new(Value::Undefined)),
    };
    let value = Value::new_object(object.clone());
    ctx.initialize_configuration_instance(MODULE, &object.instance, value.clone())?;
    Ok(value)
}
fn invoke(ctx: &mut CallContext<'_>, args: &[Value]) -> RtResult<Value> {
    let object = probe(&args[0])?;
    let (result, params) = ctx.call_configuration_instance_handler(
        MODULE,
        &object.instance,
        "оБрАбОтКа",
        vec![Value::Boolean(true)],
        args[0].clone(),
    )?;
    Ok(Value::new_array(vec![result, params[0].clone()]))
}
static FUNCTIONS: &[FunctionDescriptor] = &[
    FunctionDescriptor {
        code: FunctionCode::new(1),
        names: &["СоздатьПробуОбработчика"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(create),
    },
    FunctionDescriptor {
        code: FunctionCode::new(2),
        names: &["ВызватьОбработчикПробы"],
        arity: Arity::exact(1),
        kind: FunctionKind::Function,
        call: Some(invoke),
    },
];
fn engine() -> Engine {
    Engine::builder()
        .register_library(LibraryDescriptor::new("handler-probe", "1").with_functions(FUNCTIONS))
        .configuration(ModuleGraphRecipe {
            eager_init: false,
            modules: vec![ModuleRecipe {
                name: MODULE.into(),
                visibility: ModuleVisibility::Private,
                imports: vec![],
                source: r#"
Перем Счетчик;
Функция Обработка(СтандартнаяОбработка)
    Счетчик = Счетчик + 1;
    ЭтотОбъект.Метка = Счетчик;
    СтандартнаяОбработка = Ложь;
    Возврат Счетчик;
КонецФункции
Функция Прочесть() Экспорт Возврат Счетчик; КонецФункции
Счетчик = 0;
"#
                .into(),
            }],
        })
        .build()
        .unwrap()
}

#[test]
fn platform_handler_retains_receiver_object_state_and_final_parameters() {
    let engine = engine();
    let module = engine.compile(r#"
Первый = СоздатьПробуОбработчика(); Второй = СоздатьПробуОбработчика();
Результат = ВызватьОбработчикПробы(Первый);
Если Результат[0] <> 1 Или Результат[1] <> Ложь Тогда ВызватьИсключение "параметры"; КонецЕсли;
Если Первый.Метка <> 1 Или Первый.Прочесть() <> 1 Или Второй.Прочесть() <> 0 Тогда ВызватьИсключение "получатель"; КонецЕсли;
Результат = ВызватьОбработчикПробы(Первый);
Возврат Результат[0];
"#).unwrap();
    for _ in 0..2 {
        assert_eq!(
            engine.new_state().run(&module).unwrap(),
            Value::number_from_i64(2)
        );
    }
}

#[test]
fn private_handler_stays_inaccessible_to_ordinary_object_calls() {
    let engine = engine();
    let module = engine
        .compile("Объект = СоздатьПробуОбработчика(); Возврат Объект.Обработка(Истина);")
        .unwrap();
    assert!(
        engine
            .new_state()
            .run(&module)
            .unwrap_err()
            .to_string()
            .contains("не экспортируется")
    );
}

#[test]
fn platform_handler_rejects_an_instance_owned_by_another_state() {
    let engine = engine();
    let create = engine
        .compile("Возврат СоздатьПробуОбработчика();")
        .unwrap();
    let mut first = engine.new_state();
    let value = first.run(&create).unwrap();
    let call = engine.compile("Функция Проверить(Объект) Экспорт Возврат ВызватьОбработчикПробы(Объект); КонецФункции").unwrap();
    let mut second = engine.new_state();
    let module = second.instantiate(&call).unwrap();
    assert!(
        module
            .call(&mut second, "Проверить", vec![value])
            .unwrap_err()
            .to_string()
            .contains("другому сеансу")
    );
}
