use open_bsl::*;
use std::rc::Rc;

const OBJECT_MODULE: &str = "МодульОбъектаПробы";
static PROBE_TYPE: TypeDescriptor = TypeDescriptor::new("instance-probe", "ОбъектПробы");

#[derive(Debug, Clone)]
struct Probe(Rc<ConfigurationInstance>);

impl ObjectProtocol for Probe {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &PROBE_TYPE
    }
    fn configuration_instance(&self) -> Option<&ConfigurationInstance> {
        Some(&self.0)
    }
    fn call_method(
        &self,
        name: &str,
        args: &[Value],
        ctx: &mut CallContext<'_>,
    ) -> RtResult<Value> {
        ctx.call_configuration_instance(
            OBJECT_MODULE,
            &self.0,
            name,
            args.to_vec(),
            Value::new_object(self.clone()),
        )
        .map(|(value, _)| value)
    }
    fn has_method(&self, _: &str) -> bool {
        true
    }
}

fn create(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    let object = Probe(Rc::new(ConfigurationInstance::new()));
    let value = Value::new_object(object.clone());
    ctx.initialize_configuration_instance(OBJECT_MODULE, &object.0, value.clone())?;
    Ok(value)
}

static FUNCTIONS: &[FunctionDescriptor] = &[FunctionDescriptor {
    code: FunctionCode::new(1),
    names: &["СоздатьОбъектПробы"],
    arity: Arity::exact(0),
    kind: FunctionKind::Function,
    call: create,
}];

fn engine() -> Engine {
    Engine::builder()
        .register_library(LibraryDescriptor::new("instance-probe", "1").with_functions(FUNCTIONS))
        .configuration(ModuleGraphRecipe {
            modules: vec![
                ModuleRecipe {
                    name: OBJECT_MODULE.into(), visibility: ModuleVisibility::Private,
                    imports: vec![("Общий".into(), "Общий".into())],
                    source: r#"
Перем Счетчик;
Функция Коснуться() Экспорт Счетчик = Счетчик + 1; Возврат Счетчик; КонецФункции
Функция ОбщийСчетчик() Экспорт Возврат Общий.Коснуться(); КонецФункции
Функция Динамический() Экспорт Возврат Вычислить("Коснуться()"); КонецФункции
Функция Рекурсия(Другой, Глубина) Экспорт
    Если Глубина = 0 Тогда Возврат Коснуться(); КонецЕсли;
    Возврат Другой.Рекурсия(ЭтотОбъект, Глубина - 1);
КонецФункции
Функция Ошибка() Экспорт Счетчик = Счетчик + 1; ВызватьИсключение "проба"; КонецФункции
Функция НеЭкспорт() Возврат 1; КонецФункции
Функция Умолчание(Число = 7) Экспорт Возврат Число; КонецФункции
Счетчик = 0;
"#.into(),
                },
                ModuleRecipe {
                    name: "Общий".into(), visibility: ModuleVisibility::Named,
                    imports: Vec::new(),
                    source: "Перем Счетчик; Функция Коснуться() Экспорт Счетчик = Счетчик + 1; Возврат Счетчик; КонецФункции Счетчик = 0;".into(),
                },
            ], eager_init: false,
        }).build().unwrap()
}

#[test]
fn object_variables_and_dynamic_calls_are_separate_but_common_modules_are_shared() {
    let engine = engine();
    let module = engine.compile(r#"
Первый = СоздатьОбъектПробы(); Второй = СоздатьОбъектПробы();
Если Первый.Коснуться() <> 1 Или Второй.Коснуться() <> 1 Или Первый.Динамический() <> 2 Тогда ВызватьИсключение "память объектов"; КонецЕсли;
Если Первый.Рекурсия(Второй, 2) <> 3 Тогда ВызватьИсключение "обратный вызов"; КонецЕсли;
Если Первый.ОбщийСчетчик() <> 1 Или Второй.ОбщийСчетчик() <> 2 Тогда ВызватьИсключение "общий модуль"; КонецЕсли;
Возврат Истина;
"#).unwrap();
    assert_eq!(
        engine.new_state().run(&module).unwrap(),
        Value::Boolean(true)
    );
    assert_eq!(
        engine.new_state().run(&module).unwrap(),
        Value::Boolean(true)
    );
}

#[test]
fn object_exception_retains_memory_and_export_checks_do_not_execute_code() {
    let engine = engine();
    let module = engine.compile(r#"
Объект = СоздатьОбъектПробы();
Попытка Объект.Ошибка(); Исключение КонецПопытки;
Попытка Объект.НеЭкспорт(); Исключение КонецПопытки;
Попытка Объект.Неизвестный(); Исключение КонецПопытки;
Попытка Объект.Коснуться(1); Исключение КонецПопытки;
Если Объект.Умолчание() <> 7 Или Объект.Умолчание(Неопределено) <> Неопределено Тогда ВызватьИсключение "умолчание"; КонецЕсли;
Возврат Объект.Коснуться();
"#).unwrap();
    assert_eq!(
        engine.new_state().run(&module).unwrap(),
        Value::number_from_i64(2)
    );
}

#[test]
fn instance_rejects_a_foreign_state_and_does_not_keep_its_object_alive() {
    let engine = engine();
    let create = engine.compile("Возврат СоздатьОбъектПробы();").unwrap();
    let mut first = engine.new_state();
    let value = first.run(&create).unwrap();
    let Value::Object(object) = &value else {
        panic!("объект")
    };
    let BslObject::Extension(object) = object.as_ref() else {
        panic!("компонент")
    };
    let weak = Rc::downgrade(&object.downcast_ref::<Probe>().unwrap().0);
    let call = engine
        .compile("Функция Проверить(Объект) Экспорт Возврат Объект.Коснуться(); КонецФункции")
        .unwrap();
    let mut second = engine.new_state();
    let second_module = second.instantiate(&call).unwrap();
    let error = second_module
        .call(&mut second, "Проверить", vec![value.clone()])
        .unwrap_err();
    assert!(error.to_string().contains("другому сеансу"));
    drop(second);
    drop(value);
    assert!(weak.upgrade().is_none());
}
