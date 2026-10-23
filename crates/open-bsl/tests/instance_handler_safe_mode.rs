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
    let (result, params) = ctx.call_configuration_instance_handler_without_safe_mode(
        MODULE,
        &object.instance,
        "оБрАбОтКа",
        vec![Value::Boolean(true)],
        args[0].clone(),
    )?;
    Ok(Value::new_array(vec![result, params[0].clone()]))
}
fn safe(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    Ok(Value::Boolean(ctx.safe_mode()?))
}
fn set_safe(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    ctx.set_safe_mode(true)?;
    Ok(Value::Undefined)
}
fn ordinary(ctx: &mut CallContext<'_>, args: &[Value]) -> RtResult<Value> {
    let object = probe(&args[0])?;
    ctx.call_configuration_instance_handler(
        MODULE,
        &object.instance,
        "Обычный",
        vec![],
        args[0].clone(),
    )
    .map(|(v, _)| v)
}
static FUNCTIONS: &[FunctionDescriptor] = &[
    FunctionDescriptor {
        code: FunctionCode::new(3),
        names: &["РежимПробы"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: safe,
    },
    FunctionDescriptor {
        code: FunctionCode::new(4),
        names: &["ВключитьРежимПробы"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: set_safe,
    },
    FunctionDescriptor {
        code: FunctionCode::new(5),
        names: &["ОбычныйВызовПробы"],
        arity: Arity::exact(1),
        kind: FunctionKind::Function,
        call: ordinary,
    },
    FunctionDescriptor {
        code: FunctionCode::new(1),
        names: &["СоздатьПробуОбработчика"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: create,
    },
    FunctionDescriptor {
        code: FunctionCode::new(2),
        names: &["ВызватьОбработчикПробы"],
        arity: Arity::exact(1),
        kind: FunctionKind::Function,
        call: invoke,
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
Функция Обработка(СтандартнаяОбработка)
    Если РежимПробы() Тогда ВызватьИсключение "режим обработчика"; КонецЕсли;
    СтандартнаяОбработка = Ложь;
    Если ЭтотОбъект.Метка = Истина Тогда ВызватьИсключение "ошибка пробы"; КонецЕсли;
    Возврат РежимПробы();
КонецФункции
Функция Обычный() Возврат РежимПробы(); КонецФункции
"#
                .into(),
            }],
        })
        .build()
        .unwrap()
}

#[test]
fn base_handler_suppression_is_scoped_on_success_and_error() {
    let engine = engine();
    let module = engine.compile(r#"
Проба = СоздатьПробуОбработчика();
Если РежимПробы() Тогда ВызватьИсключение "чужой режим"; КонецЕсли;
ВключитьРежимПробы();
Если Не ОбычныйВызовПробы(Проба) Тогда ВызватьИсключение "обычный режим"; КонецЕсли;
Результат = ВызватьОбработчикПробы(Проба);
Если Результат[0] <> Ложь Или Результат[1] <> Ложь Тогда ВызватьИсключение "результат"; КонецЕсли;
Если Не РежимПробы() Тогда ВызватьИсключение "возврат режима"; КонецЕсли;
Проба.Метка = Истина;
ОшибкаПерехвачена = Ложь;
Попытка ВызватьОбработчикПробы(Проба); Исключение ОшибкаПерехвачена = Истина; КонецПопытки;
Если Не ОшибкаПерехвачена Или Не РежимПробы() Или Не ОбычныйВызовПробы(Проба) Тогда ВызватьИсключение "ошибка восстановления"; КонецЕсли;
Возврат Истина;
"#).unwrap();
    for _ in 0..2 {
        assert_eq!(
            engine.new_state().run(&module).unwrap(),
            Value::Boolean(true)
        );
    }
}
