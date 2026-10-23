use open_bsl::{
    Arity, CallContext, Engine, FunctionCode, FunctionDescriptor, FunctionKind, LibraryDescriptor,
    ObjectProtocol, RtResult, TypeDescriptor, Value,
};

static TYPE: TypeDescriptor = TypeDescriptor::new("iteration-probe", "КоллекцияБезИндекса");

#[derive(Debug)]
struct Collection;

impl ObjectProtocol for Collection {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &TYPE
    }

    fn collection_item(&self, index: &Value) -> RtResult<Value> {
        if index == &Value::number_from_i64(0) {
            Ok(Value::number_from_i64(10))
        } else {
            Ok(Value::number_from_i64(20))
        }
    }

    fn collection_len(&self) -> RtResult<usize> {
        Ok(2)
    }
}

fn make(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    Ok(Value::new_object(Collection))
}

const FUNCTIONS: &[FunctionDescriptor] = &[FunctionDescriptor {
    code: FunctionCode::new(1),
    names: &["КоллекцияДляОбхода"],
    arity: Arity::exact(0),
    kind: FunctionKind::Function,
    call: make,
}];

#[test]
fn iteration_does_not_require_public_indexing() {
    let engine = Engine::builder()
        .register_library(LibraryDescriptor::new("iteration-probe", "1").with_functions(FUNCTIONS))
        .build()
        .unwrap();
    let module = engine
        .compile(
            r#"
Коллекция = КоллекцияДляОбхода();
ИндексЗапрещен = Ложь;
Попытка
    Элемент = Коллекция[0];
Исключение
    ИндексЗапрещен = Истина;
КонецПопытки;
Если Не ИндексЗапрещен Тогда ВызватьИсключение "Индекс доступен"; КонецЕсли;
Сумма = 0;
Для Каждого Элемент Из Коллекция Цикл
    Сумма = Сумма + Элемент;
КонецЦикла;
СуммаПослеПродолжения = 0;
Для Каждого Элемент Из Коллекция Цикл
    Если Элемент = 10 Тогда Продолжить; КонецЕсли;
    СуммаПослеПродолжения = СуммаПослеПродолжения + Элемент;
КонецЦикла;
Если СуммаПослеПродолжения <> 20 Тогда ВызватьИсключение "Продолжение обхода"; КонецЕсли;
Для Каждого Элемент Из Коллекция Цикл
    Если Элемент <> 10 Тогда ВызватьИсключение "Прерывание обхода"; КонецЕсли;
    Прервать;
КонецЦикла;
Возврат Сумма;
"#,
        )
        .unwrap();
    for bytecode in [false, true] {
        let loaded;
        let module = if bytecode {
            loaded = engine.load_bytecode(&module.bytecode().unwrap()).unwrap();
            &loaded
        } else {
            &module
        };
        assert_eq!(
            engine.new_state().run(module).unwrap(),
            Value::number_from_i64(30)
        );
    }
}

#[test]
fn invalid_iteration_registers_are_rejected_before_execution() {
    let engine = Engine::builder().build().unwrap();
    let module = engine
        .compile("Массив = Новый Массив; Массив.Добавить(10); Для Каждого Элемент Из Массив Цикл КонецЦикла;")
        .unwrap();
    let text = module.bytecode().unwrap();
    let operation = text.find("GetCollectionItem ").unwrap();
    for operand in ["dst=", "obj=", "idx="] {
        let mut invalid = text.clone();
        let start = operation + invalid[operation..].find(operand).unwrap() + operand.len();
        let end = start + invalid[start..].find(char::is_whitespace).unwrap();
        invalid.replace_range(start..end, "255");
        if let Ok(module) = engine.load_bytecode(&invalid) {
            assert!(engine.new_state().run(&module).is_err());
        }
    }
}
