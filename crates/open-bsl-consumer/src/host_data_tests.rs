use std::cell::Cell;
use std::rc::Rc;

use open_bsl::*;

pub(super) fn data<'a>(ctx: &'a CallContext<'_>) -> RtResult<&'a Cell<i64>> {
    ctx.host_data()?
        .downcast_ref::<Cell<i64>>()
        .ok_or_else(|| RtError::DynamicError("хост передал данные другого типа".into()))
}

fn read(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    Ok(Value::number_from_i64(data(ctx)?.get()))
}

fn absent(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    Ok(Value::Boolean(matches!(
        ctx.host_data(),
        Err(RtError::CapabilityMissing {
            capability: Capability::HostData,
            ..
        })
    )))
}

#[derive(Debug)]
struct HostObject;
static HOST_TYPE: TypeDescriptor = TypeDescriptor::new("host-data-test", "ОбъектХоста");
fn construct(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    data(ctx)?;
    Ok(Value::new_object(HostObject))
}
fn method(_: &dyn ObjectProtocol, _: &[Value], ctx: &mut CallContext<'_>) -> RtResult<Value> {
    read(ctx, &[])
}
fn get(_: &dyn ObjectProtocol, ctx: &mut CallContext<'_>) -> RtResult<Value> {
    read(ctx, &[])
}
fn set(_: &dyn ObjectProtocol, value: Value, ctx: &mut CallContext<'_>) -> RtResult<()> {
    // Для пробы важен путь записи свойства, а не преобразование числа.
    assert_eq!(value, Value::number_from_i64(17));
    data(ctx)?.set(17);
    Ok(())
}
impl ObjectProtocol for HostObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &HOST_TYPE
    }
    fn method_table(&self) -> &'static [MethodDescriptor] {
        const METHODS: &[MethodDescriptor] = &[MethodDescriptor::new(
            &["Прочитать"],
            Arity::exact(0),
            method,
        )];
        METHODS
    }
    fn property_table(&self) -> &'static [PropertyDescriptor] {
        const PROPERTIES: &[PropertyDescriptor] = &[PropertyDescriptor {
            names: &["Значение"],
            get,
            set: Some(set),
        }];
        PROPERTIES
    }
}
pub(super) const fn library() -> LibraryDescriptor {
    const TYPES: &[&TypeDescriptor] = &[&HOST_TYPE];
    const LIBRARY: LibraryDescriptor = LibraryDescriptor::new("host-data-test", "1")
        .with_functions(&[
            FunctionDescriptor {
                code: FunctionCode::new(1),
                names: &["ДанныеХоста"],
                arity: Arity::exact(0),
                kind: FunctionKind::Function,
                call: read,
            },
            FunctionDescriptor {
                code: FunctionCode::new(2),
                names: &["НетДанныхХоста"],
                arity: Arity::exact(0),
                kind: FunctionKind::Function,
                call: absent,
            },
        ])
        .with_types(TYPES)
        .with_constructors(&[ConstructorDescriptor {
            code: ConstructorCode::new(1),
            names: &["ОбъектХоста"],
            arity: Arity::exact(0),
            call: construct,
        }]);
    LIBRARY
}

#[test]
fn all_component_contexts_receive_only_their_states_data() {
    let engine = Engine::builder()
        .register_library(library())
        .build()
        .unwrap();
    let mut state = engine
        .state_builder()
        .host_data(Rc::new(Cell::new(42_i64)))
        .build();
    assert_eq!(
        state.exec("Возврат ДанныеХоста();").unwrap(),
        Value::number_from_i64(42)
    );
    assert_eq!(state.exec("Объект = Новый ОбъектХоста; Объект.Значение = 17; Возврат Объект.Прочитать() + Объект.Значение;").unwrap(), Value::number_from_i64(34));
    let mut other = engine.new_state();
    for source in ["Возврат ДанныеХоста();", "Возврат Новый ОбъектХоста;"]
    {
        assert!(matches!(
            other.exec(source),
            Err(Error::Runtime(RtError::CapabilityMissing {
                capability: Capability::HostData,
                ..
            }))
        ));
    }
    let mut shapes = RuntimeShapes::seeded(Vec::new(), Vec::new(), None);
    let ctx = CallContext::minimal(&mut shapes, format_value);
    assert!(matches!(
        ctx.host_data(),
        Err(RtError::CapabilityMissing {
            capability: Capability::HostData,
            path: ContextKind::Reduced
        })
    ));
}

#[test]
fn background_job_does_not_inherit_foreground_host_data() {
    let engine = Engine::builder().register_library(library())
        .common_module("Работы", "Процедура Проверить() Экспорт Если НЕ НетДанныхХоста() Тогда ВызватьИсключение \"утечка данных хоста\"; КонецЕсли; КонецПроцедуры")
        .build().unwrap();
    let mut state = engine
        .state_builder()
        .host_data(Rc::new(Cell::new(42_i64)))
        .build();
    assert_eq!(
        state
            .exec(
                r#"
        Задание = ФоновыеЗадания.Выполнить("Работы.Проверить");
        Задание = Задание.ОжидатьЗавершенияВыполнения();
        Возврат Задание.Состояние = СостояниеФоновогоЗадания.Завершено;
    "#
            )
            .unwrap(),
        Value::Boolean(true)
    );
}
