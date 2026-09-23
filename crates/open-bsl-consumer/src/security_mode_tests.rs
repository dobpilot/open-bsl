use open_bsl::*;

fn set_safe(ctx: &mut CallContext<'_>, arguments: &[Value]) -> RtResult<Value> {
    ctx.set_safe_mode(arguments[0] == Value::Boolean(true))?;
    Ok(Value::Undefined)
}

fn safe(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    Ok(Value::Boolean(ctx.safe_mode()?))
}

fn read_from_nested_module(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    ctx.call_configuration_module("ПроверкаРежима", "Прочитать", Vec::new(), None)
        .map(|(value, _)| value)
}

fn disable_from_nested_module(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    ctx.call_configuration_module("ПроверкаРежима", "Снять", Vec::new(), None)
        .map(|(value, _)| value)
}

const LIBRARY: LibraryDescriptor = LibraryDescriptor::new("consumer-security-mode-test", "1")
    .with_functions(&[
        FunctionDescriptor {
            code: FunctionCode::new(1),
            names: &["ВключитьБезопасныйРежим"],
            arity: Arity::exact(1),
            kind: FunctionKind::Procedure,
            call: set_safe,
        },
        FunctionDescriptor {
            code: FunctionCode::new(2),
            names: &["ПроверитьБезопасныйРежим"],
            arity: Arity::exact(0),
            kind: FunctionKind::Function,
            call: safe,
        },
        FunctionDescriptor {
            code: FunctionCode::new(3),
            names: &["ПрочитатьРежимВМодуле"],
            arity: Arity::exact(0),
            kind: FunctionKind::Function,
            call: read_from_nested_module,
        },
        FunctionDescriptor {
            code: FunctionCode::new(4),
            names: &["СнятьРежимВМодуле"],
            arity: Arity::exact(0),
            kind: FunctionKind::Function,
            call: disable_from_nested_module,
        },
    ]);

struct FacadeControl;

impl SecurityModeControl for FacadeControl {
    fn safe_mode(&self) -> bool {
        false
    }

    fn set_safe_mode(&self, _: bool) -> RtResult<()> {
        Ok(())
    }

    fn set_safe_mode_disabled(&self, _: bool) -> RtResult<()> {
        Ok(())
    }

    fn safe_mode_disabled(&self) -> bool {
        false
    }

    fn privileged_mode(&self) -> bool {
        false
    }

    fn set_privileged_mode(&self, _: bool) -> RtResult<()> {
        Ok(())
    }
}

#[test]
fn a_component_controls_the_current_frame_through_facade_names_only() {
    let facade_control: &dyn SecurityModeControl = &FacadeControl;
    assert!(!facade_control.safe_mode());

    let engine = Engine::builder()
        .register_library(LIBRARY)
        .common_module(
            "ПроверкаРежима",
            r#"
            Функция Прочитать() Экспорт
                Возврат ПроверитьБезопасныйРежим();
            КонецФункции

            Процедура Снять() Экспорт
                ВключитьБезопасныйРежим(Ложь);
            КонецПроцедуры
            "#,
        )
        .build()
        .unwrap();
    let mut state = engine.new_state();
    assert_eq!(
        state
            .exec("ВключитьБезопасныйРежим(Истина); Возврат ПроверитьБезопасныйРежим();")
            .unwrap(),
        Value::Boolean(true)
    );
    assert_eq!(
        state
            .exec(
                r#"
                ВключитьБезопасныйРежим(Истина);
                ВложенныйВидит = ПрочитатьРежимВМодуле();
                Попытка
                    СнятьРежимВМодуле();
                    ОшибкаСнятия = Ложь;
                Исключение
                    ОшибкаСнятия = Истина;
                КонецПопытки;
                Возврат ВложенныйВидит И ОшибкаСнятия И ПроверитьБезопасныйРежим();
                "#,
            )
            .unwrap(),
        Value::Boolean(true)
    );

    let mut shapes = RuntimeShapes::seeded(Vec::new(), Vec::new(), None);
    let context = CallContext::minimal(&mut shapes, format_value);
    assert!(matches!(
        context.safe_mode(),
        Err(RtError::CapabilityMissing {
            capability: Capability::SecurityModes,
            path: ContextKind::Reduced,
        })
    ));
}
