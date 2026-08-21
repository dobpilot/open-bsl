//! Пример из README («Свой компонент и свой объект»): хост-тип «Счётчик»
//! с конструктором, методом и свойством. Источник правды для README —
//! раздел ссылается сюда, а тест `counter_example` собирает и запускает
//! пример, так что код всегда компилируется и печатает ожидаемое.

use open_bsl::{
    Arity, CallContext, ConstructorCode, ConstructorDescriptor, Engine, LibraryDescriptor,
    MethodCode, MethodDescriptor, ObjectProtocol, RtError, RtResult, TypeDescriptor, Value,
    call_method_from_table,
};

// Состояние объекта живёт за `Rc`: рантайм однопоточный, а обёртка
// значения может пересобираться, пока состояние остаётся общим.
#[derive(Debug)]
struct Counter {
    value: std::rc::Rc<std::cell::Cell<i64>>,
}

static COUNTER_TYPE: TypeDescriptor = TypeDescriptor {
    package: "example-host",
    name: "Счётчик",
    legacy_type_id: None,
};

// Обработчик метода получает значение-получателя от вызывающего и
// возвращается к конкретному типу через downcast.
fn counter_of<'v>(receiver: &'v Value, method: &'static str) -> RtResult<&'v Counter> {
    receiver
        .object_ref()
        .and_then(|object| object.downcast_ref::<Counter>())
        .ok_or(RtError::MethodNotApplicable {
            method,
            receiver: receiver.type_name(),
        })
}

fn counter_increase(
    receiver: &Value,
    _arguments: &[Value],
    _context: &mut CallContext<'_>,
) -> RtResult<Value> {
    let counter = counter_of(receiver, "Увеличить")?;
    counter.value.set(counter.value.get() + 1);
    Ok(Value::Undefined)
}

// Таблица методов типа: плотные коды от единицы, русское и английское
// написания. Имена регистронезависимы.
const COUNTER_METHODS: &[MethodDescriptor] = &[MethodDescriptor {
    code: MethodCode::new(1),
    names: &["Увеличить", "Increase"],
    call: counter_increase,
}];

impl ObjectProtocol for Counter {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &COUNTER_TYPE
    }

    // Свойства обслуживаются парой get_property/set_property.
    fn get_property(&self, name: &str, _context: &mut CallContext<'_>) -> RtResult<Value> {
        if matches!(name.to_uppercase().as_str(), "ЗНАЧЕНИЕ" | "VALUE") {
            Ok(Value::number_from_i64(self.value.get()))
        } else {
            Err(RtError::UnknownColumn(name.to_string()))
        }
    }

    // Вход для вызовов с именем-строкой — реализуется той же таблицей.
    fn call_method(
        &self,
        name: &str,
        arguments: &[Value],
        context: &mut CallContext<'_>,
    ) -> RtResult<Value> {
        let receiver = Value::new_object(Counter {
            value: self.value.clone(),
        });
        call_method_from_table(
            COUNTER_METHODS,
            COUNTER_TYPE.name,
            &receiver,
            name,
            arguments,
            context,
        )
    }

    // Непустая таблица включает быстрый путь VM: обработчик находится по
    // номеру имени без строковых сравнений на каждом вызове.
    fn method_table(&self) -> &'static [MethodDescriptor] {
        COUNTER_METHODS
    }
}

fn construct_counter(_context: &mut CallContext<'_>, _arguments: &[Value]) -> RtResult<Value> {
    Ok(Value::new_object(Counter {
        value: std::rc::Rc::new(std::cell::Cell::new(0)),
    }))
}

const COUNTER_CONSTRUCTORS: &[ConstructorDescriptor] = &[ConstructorDescriptor {
    code: ConstructorCode::new(1),
    names: &["Счётчик", "Counter"],
    arity: Arity::exact(0),
    call: construct_counter,
}];

fn counter_library() -> LibraryDescriptor {
    LibraryDescriptor {
        package: "example-host",
        version: "1.0.0",
        dependencies: &[],
        functions: &[],
        constructors: COUNTER_CONSTRUCTORS,
    }
}

fn main() -> Result<(), open_bsl::Error> {
    let engine = Engine::builder()
        .register_library(counter_library())
        .build()?;
    let module = engine
        .compile("с = Новый Счётчик();\nс.Увеличить();\nс.Увеличить();\nВозврат с.Значение;")?;
    let result = engine.new_state().run(&module)?;
    assert_eq!(result.to_string(), "2");
    println!("{result}");
    Ok(())
}
