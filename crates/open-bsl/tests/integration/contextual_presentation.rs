use open_bsl::{
    Arity, CallContext, Engine, FunctionCode, FunctionDescriptor, FunctionKind, LibraryDescriptor,
    ObjectProtocol, RtResult, TypeDescriptor, Value,
};
use std::rc::Rc;

static TYPE: TypeDescriptor = TypeDescriptor::new("presentation-test", "ПробаПредставления");
#[derive(Debug)]
struct Presented(bool);
static TYPES: &[&TypeDescriptor] = &[&TYPE];
impl ObjectProtocol for Presented {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &TYPE
    }
    fn presentation(&self) -> RtResult<String> {
        Ok("legacy".into())
    }
    fn presentation_with_context(&self, context: &CallContext<'_>) -> RtResult<String> {
        if !self.0 {
            return self.presentation();
        }
        let data = context.host_data()?;
        Ok(data.downcast_ref::<String>().unwrap().clone())
    }
}
fn render(context: &mut CallContext<'_>, args: &[Value]) -> RtResult<Value> {
    let value = Value::new_object(Presented(args[0] == Value::Boolean(true)));
    context
        .format_value(&value, None)
        .map(|text| Value::Str(text.as_str().into()))
}
fn create(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    Ok(Value::new_object(Presented(true)))
}
static FUNCTIONS: &[FunctionDescriptor] = &[
    FunctionDescriptor {
        code: FunctionCode::new(1),
        names: &["ПредставитьПробу"],
        arity: Arity::exact(1),
        kind: FunctionKind::Function,
        call: Some(render),
    },
    FunctionDescriptor {
        code: FunctionCode::new(2),
        names: &["СоздатьПробуПредставления"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(create),
    },
];

#[test]
fn component_context_uses_invoking_state_data_and_preserves_legacy_presentation() {
    for opts in [
        bsl_compiler::Optimizations::default(),
        bsl_compiler::Optimizations::all(),
    ] {
        let engine = Engine::builder()
            .optimizations(opts)
            .register_library(
                LibraryDescriptor::new("presentation-test", "1")
                    .with_types(TYPES)
                    .with_functions(FUNCTIONS),
            )
            .build()
            .unwrap();
        let mut ru = engine
            .state_builder()
            .host_data(Rc::new(String::from("ru")))
            .build();
        let mut en = engine
            .state_builder()
            .host_data(Rc::new(String::from("en")))
            .build();
        for _ in 0..2 {
            assert_eq!(
                ru.exec("Возврат ПредставитьПробу(Истина);").unwrap(),
                Value::Str("ru".into())
            );
            assert_eq!(
                en.exec("Возврат ПредставитьПробу(Истина);").unwrap(),
                Value::Str("en".into())
            );
        }
        for source in [
            "Возврат Строка(СоздатьПробуПредставления());",
            r#"Возврат Формат(СоздатьПробуПредставления(), "");"#,
        ] {
            assert_eq!(ru.exec(source).unwrap(), Value::Str("ru".into()));
            assert_eq!(en.exec(source).unwrap(), Value::Str("en".into()));
        }
        for source in [
            r#"Возврат "prefix:" + СоздатьПробуПредставления();"#,
            r#"ТекстПробы = "prefix:"; ТекстПробы = ТекстПробы + СоздатьПробуПредставления(); Возврат ТекстПробы;"#,
        ] {
            assert_eq!(ru.exec(source).unwrap(), Value::Str("prefix:ru".into()));
            assert_eq!(en.exec(source).unwrap(), Value::Str("prefix:en".into()));
        }
        let mut absent = engine.new_state();
        for source in [
            "Возврат Строка(СоздатьПробуПредставления());",
            r#"Возврат Формат(СоздатьПробуПредставления(), "");"#,
            "Сообщить(СоздатьПробуПредставления());",
        ] {
            assert!(absent.exec(source).is_err(), "{source}");
        }
        assert!(absent.exec("Возврат ПредставитьПробу(Истина);").is_err());
        assert_eq!(
            absent.exec("Возврат ПредставитьПробу(Ложь);").unwrap(),
            Value::Str("legacy".into())
        );
    }
}

#[test]
fn both_string_operation_arguments_use_the_invoking_states_presentation() {
    for opts in [
        bsl_compiler::Optimizations::default(),
        bsl_compiler::Optimizations::all(),
    ] {
        let engine = Engine::builder()
            .optimizations(opts)
            .register_library(
                LibraryDescriptor::new("presentation-test", "1")
                    .with_types(TYPES)
                    .with_functions(FUNCTIONS),
            )
            .build()
            .unwrap();
        for spelling in ["ru", "en"] {
            let mut state = engine
                .state_builder()
                .host_data(Rc::new(spelling.to_string()))
                .build();
            for source in [
                format!("Возврат СтрСравнить(\"{spelling}\", СоздатьПробуПредставления());"),
                format!("Возврат StrCompare(СоздатьПробуПредставления(), \"{spelling}\");"),
                "Возврат СтрСравнить(СоздатьПробуПредставления(), СоздатьПробуПредставления());"
                    .into(),
            ] {
                assert_eq!(state.exec(&source).unwrap(), Value::number_from_i64(0));
            }
            for source in [
                format!(
                    "Возврат СтрЧислоВхождений(\"{spelling}-{spelling}\", СоздатьПробуПредставления());"
                ),
                format!(
                    "Возврат StrOccurrenceCount(СоздатьПробуПредставления(), \"{spelling}\") + 1;"
                ),
            ] {
                let compiled = engine.compile(&source).unwrap();
                let reloaded = engine.load_bytecode(&compiled.bytecode().unwrap()).unwrap();
                assert_eq!(state.run(&compiled).unwrap(), Value::number_from_i64(2));
                assert_eq!(state.run(&reloaded).unwrap(), Value::number_from_i64(2));
            }
        }
        let mut absent = engine.new_state();
        for source in [
            "Возврат СтрСравнить(\"legacy\", СоздатьПробуПредставления());",
            "Возврат СтрЧислоВхождений(\"legacy\", СоздатьПробуПредставления());",
        ] {
            assert!(absent.exec(source).is_err(), "{source}");
        }
    }
}
