use open_bsl::{
    Arity, CallContext, Engine, FunctionCode, FunctionDescriptor, FunctionKind, LibraryDescriptor,
    ModuleGraphRecipe, ModuleRecipe, ModuleVisibility, RtError, RtResult, Value,
};

fn inspect(context: &mut CallContext<'_>, args: &[Value]) -> RtResult<Value> {
    let [Value::Str(module), Value::Str(name)] = args else {
        return Err(RtError::TypeError {
            expected: "Строка",
            op: "ПроверитьЭкспорт",
        });
    };
    context
        .configuration_has_export(&module.to_string(), &name.to_string())
        .map(Value::Boolean)
}
const FUNCTIONS: &[FunctionDescriptor] = &[FunctionDescriptor {
    code: FunctionCode::new(1),
    names: &["ПроверитьЭкспорт"],
    arity: Arity::exact(2),
    kind: FunctionKind::Function,
    call: Some(inspect),
}];

fn engine(configuration: bool) -> Engine {
    let builder = Engine::builder().register_library(
        LibraryDescriptor::new("export-inspection", "1").with_functions(FUNCTIONS),
    );
    let builder = if configuration {
        builder.configuration(ModuleGraphRecipe {
            modules: vec![ModuleRecipe {
                name: "Владелец".into(),
                source: "Функция Экспортная() Экспорт ВызватьИсключение \"экспорт исполнился\"; КонецФункции Функция Закрытая() Возврат 17; КонецФункции ВызватьИсключение \"тело исполнилось\";".into(),
                imports: vec![], visibility: ModuleVisibility::Private,
            }], eager_init: false,
        })
    } else {
        builder
    };
    builder.build().unwrap()
}

#[test]
fn export_inspection_does_not_execute_module_body_or_procedure() {
    let engine = engine(true);
    let module = engine.compile("Возврат ПроверитьЭкспорт(\"вЛаДеЛеЦ\", \"эКсПоРтНаЯ\") И Не ПроверитьЭкспорт(\"Владелец\", \"Закрытая\") И Не ПроверитьЭкспорт(\"Владелец\", \"Отсутствующая\");").unwrap();
    assert_eq!(
        engine.new_state().run(&module).unwrap(),
        Value::Boolean(true)
    );
}

#[test]
fn unavailable_configuration_information_is_an_error() {
    for configuration in [true, false] {
        let engine = engine(configuration);
        let module = engine
            .compile("Возврат ПроверитьЭкспорт(\"ОтсутствующийМодуль\", \"Экспортная\");")
            .unwrap();
        assert!(engine.new_state().run(&module).is_err());
    }
}
