use open_bsl::*;

fn inspect(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    let mut result = Vec::new();
    for name in ["дВа", "Три", "БезПараметров", "Отсутствует"] {
        result.push(
            match ctx.configuration_function_parameter_count("МодульПараметров", name)?
            {
                Some(count) => Value::number_from_i64(i64::try_from(count).unwrap()),
                None => Value::Undefined,
            },
        );
    }
    Ok(Value::new_array(result))
}
static FUNCTIONS: &[FunctionDescriptor] = &[FunctionDescriptor {
    code: FunctionCode::new(1),
    names: &["ПроверитьПараметрыФункций"],
    arity: Arity::exact(0),
    kind: FunctionKind::Function,
    call: inspect,
}];

#[test]
fn parameter_inspection_reads_private_and_exported_functions_without_initializing() {
    let engine = Engine::builder()
        .register_library(LibraryDescriptor::new("parameter-probe", "1").with_functions(FUNCTIONS))
        .configuration(ModuleGraphRecipe { eager_init: false, modules: vec![ModuleRecipe {
            name: "МодульПараметров".into(), visibility: ModuleVisibility::Private, imports: vec![],
            source: "Процедура Два(Первый, Второй) КонецПроцедуры Процедура Три(Первый, Второй, Третий) Экспорт КонецПроцедуры Функция БезПараметров() Возврат 17; КонецФункции ВызватьИсключение \"тело нельзя исполнять\";".into(),
        }] }).build().unwrap();
    let module = engine
        .compile("Возврат ПроверитьПараметрыФункций();")
        .unwrap();
    for _ in 0..2 {
        let Value::Object(result) = engine.new_state().run(&module).unwrap() else {
            panic!("массив")
        };
        let BslObject::Array(result) = result.as_ref() else {
            panic!("массив")
        };
        assert_eq!(
            &*result.borrow(),
            &[
                Value::number_from_i64(2),
                Value::number_from_i64(3),
                Value::number_from_i64(0),
                Value::Undefined
            ]
        );
    }
}

#[test]
fn parameter_inspection_requires_a_configuration_catalog() {
    let engine = Engine::builder()
        .register_library(LibraryDescriptor::new("parameter-probe", "1").with_functions(FUNCTIONS))
        .build()
        .unwrap();
    let module = engine
        .compile("Возврат ПроверитьПараметрыФункций();")
        .unwrap();
    assert!(engine.new_state().run(&module).is_err());
}
