//! Пример из README («Свой компонент и свой объект»): хост-тип «Счётчик»
//! с конструктором, методом и свойством. Источник правды для README —
//! раздел ссылается сюда, а тест `counter_example` собирает и запускает
//! пример, так что код всегда компилируется и печатает ожидаемое.

use open_bsl::{
    Arity, CallContext, ConstructorCode, ConstructorDescriptor, Engine, LibraryDescriptor,
    MethodCode, MethodDescriptor, ObjectProtocol, PropertyCode, PropertyDescriptor, RtError,
    RtResult, TypeDescriptor, Value,
};

// Состояние объекта живёт за `Rc`: рантайм однопоточный, а обёртка
// значения может пересобираться, пока состояние остаётся общим.
#[derive(Debug)]
struct Counter {
    value: std::rc::Rc<std::cell::Cell<i64>>,
}

static COUNTER_TYPE: TypeDescriptor = TypeDescriptor::new("example-host", "Счётчик");

// Обработчик метода получает сам объект-получатель и возвращается к
// конкретному типу через downcast — без обёртки значения.
fn counter_increase(
    receiver: &dyn ObjectProtocol,
    _arguments: &[Value],
    _context: &mut CallContext<'_>,
) -> RtResult<Value> {
    let counter = receiver
        .downcast_ref::<Counter>()
        .ok_or(RtError::MethodNotApplicable {
            method: "Увеличить",
            receiver: receiver.type_descriptor().name,
        })?;
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

fn counter_value(receiver: &dyn ObjectProtocol, _context: &mut CallContext<'_>) -> RtResult<Value> {
    let counter = receiver
        .downcast_ref::<Counter>()
        .ok_or(RtError::NotAnObject)?;
    Ok(Value::number_from_i64(counter.value.get()))
}

// Таблица свойств устроена как таблица методов; `set: None` — свойство
// только для чтения, присваивание в него вернёт ошибку.
const COUNTER_PROPERTIES: &[PropertyDescriptor] = &[PropertyDescriptor {
    code: PropertyCode::new(1),
    names: &["Значение", "Value"],
    get: counter_value,
    set: None,
}];

impl ObjectProtocol for Counter {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &COUNTER_TYPE
    }

    fn property_table(&self) -> &'static [PropertyDescriptor] {
        COUNTER_PROPERTIES
    }

    // Непустая таблица включает быстрый путь VM: обработчик находится по
    // номеру имени без строковых сравнений на каждом вызове. Вызовы с
    // именем-строкой обслуживает та же таблица — `call_method` по
    // умолчанию, писать его не нужно.
    fn method_table(&self) -> &'static [MethodDescriptor] {
        COUNTER_METHODS
    }
}

fn construct_counter(_context: &mut CallContext<'_>, _arguments: &[Value]) -> RtResult<Value> {
    Ok(Value::new_object(Counter {
        value: std::rc::Rc::new(std::cell::Cell::new(0)),
    }))
}

/// Типы, которые компонент вводит в язык: по ним работает `Тип("Счётчик")`.
const COUNTER_TYPES: &[&TypeDescriptor] = &[&COUNTER_TYPE];

const COUNTER_CONSTRUCTORS: &[ConstructorDescriptor] = &[ConstructorDescriptor {
    code: ConstructorCode::new(1),
    names: &["Счётчик", "Counter"],
    arity: Arity::exact(0),
    call: construct_counter,
}];

fn counter_library() -> LibraryDescriptor {
    LibraryDescriptor {
        package: "example-host",
        object_jit: bsl_rt::ObjectJitPolicy::NativeContextCompatible,
        version: "1.0.0",
        dependencies: &[],
        functions: &[],
        constructors: COUNTER_CONSTRUCTORS,
        types: COUNTER_TYPES,
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

    // `ТипЗнч` над host-объектом называет его дескриптором: своего
    // идентификатора в закрытом реестре типов ядра у host-типа нет.
    let type_of = engine.compile("с = Новый Счётчик();\nВозврат Строка(ТипЗнч(с));")?;
    assert_eq!(engine.new_state().run(&type_of)?.to_string(), "Счётчик");

    // И `Тип("Счётчик")` находит тот же тип — как у штатных типов языка.
    let same = engine.compile("с = Новый Счётчик();\nВозврат Тип(\"Счётчик\") = ТипЗнч(с);")?;
    assert_eq!(engine.new_state().run(&same)?.to_string(), "Да");

    println!("{result}");
    Ok(())
}
