//! `ХранилищеЗначения` и `СжатиеДанных`.

use super::*;

#[derive(Debug)]
struct DeflationValue {
    level: i8,
}

#[derive(Debug)]
pub(crate) struct ValueStorageValue {
    graph: SerializedValueGraph,
    _compression_level: Option<i8>,
}

pub(crate) static DEFLATION_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-orm",
    name: "СжатиеДанных",
    type_display: "Compress Data",
    type_names: &["Deflation"],
};

pub(crate) static VALUE_STORAGE_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-orm",
    name: "ХранилищеЗначения",
    type_display: "Value storage",
    type_names: &["ValueStorage"],
};

impl ObjectProtocol for DeflationValue {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &DEFLATION_TYPE
    }
}

impl ObjectProtocol for ValueStorageValue {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &VALUE_STORAGE_TYPE
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        VALUE_STORAGE_METHODS
    }
}

pub(crate) fn new_deflation(_: &mut CallContext<'_>, arguments: &[Value]) -> RtResult<Value> {
    let level = match arguments {
        [] => -1,
        [Value::Number(level)] => level
            .to_i64_exact()
            .and_then(|level| i8::try_from(level).ok())
            .filter(|level| (-1..=9).contains(level))
            .ok_or_else(|| {
                RtError::DynamicError("степень сжатия должна быть целым числом от -1 до 9".into())
            })?,
        [_] => {
            return Err(RtError::TypeError {
                expected: "Число",
                op: "Новый СжатиеДанных",
            });
        }
        _ => {
            return Err(RtError::DynamicError(
                "Новый СжатиеДанных ожидает не больше одного аргумента".into(),
            ));
        }
    };
    Ok(Value::new_object(DeflationValue { level }))
}

/// Создаёт снимок `ХранилищеЗначения` из внутренней сериализации платформы.
/// Строка содержит значение без заголовка и сжатия поля информационной базы.
/// Использует тот же декодер и отделение данных, что существующие операции BSL.
///
/// # Errors
/// Возвращает ошибку для неверного или неподдерживаемого внутреннего формата,
/// непереносимого значения либо превышения ограничений снимка.
pub fn value_storage_from_internal(text: &str, context: &mut CallContext<'_>) -> RtResult<Value> {
    let value = open_bsl::value_from_string_internal(text, context.runtime_shapes())?;
    new_value_storage(context, &[value])
}

/// Создаёт снимок хранилища с явным резолвером скалярных объектов хоста.
/// Резолвер восстанавливает конфигурационные значения до создания снимка;
/// без распознанного вида действуют прежние ограничения переносимости.
///
/// # Errors
/// Возвращает ошибки формата, резолвера и создания снимка.
pub fn value_storage_from_internal_with_resolver(
    text: &str,
    context: &mut CallContext<'_>,
    resolver: &mut dyn FnMut(&str, &str) -> RtResult<Option<Value>>,
) -> RtResult<Value> {
    let value = open_bsl::value_from_string_internal_with_resolver(
        text,
        context.runtime_shapes(),
        resolver,
    )?;
    new_value_storage(context, &[value])
}

pub(crate) fn new_value_storage(
    context: &mut CallContext<'_>,
    arguments: &[Value],
) -> RtResult<Value> {
    let (value, compression_level) = match arguments {
        [value] => (value, None),
        [value, compression] => {
            let compression = compression
                .object_ref()
                .and_then(|object| object.downcast_ref::<DeflationValue>())
                .ok_or(RtError::TypeError {
                    expected: "СжатиеДанных",
                    op: "Новый ХранилищеЗначения",
                })?;
            (value, Some(compression.level))
        }
        _ => {
            return Err(RtError::DynamicError(
                "Новый ХранилищеЗначения ожидает один или два аргумента".into(),
            ));
        }
    };
    let graph = SerializedValueGraph::capture_detached(
        std::slice::from_ref(value),
        context.runtime_shapes(),
        &GraphLimits::default(),
    )?;
    Ok(Value::new_object(ValueStorageValue {
        graph,
        _compression_level: compression_level,
    }))
}

fn value_storage_get(
    receiver: &dyn ObjectProtocol,
    _: &[Value],
    context: &mut CallContext<'_>,
) -> RtResult<Value> {
    let storage = receiver
        .downcast_ref::<ValueStorageValue>()
        .ok_or(RtError::NotAnObject)?;
    let mut values = storage.graph.materialize(context.runtime_shapes())?;
    if values.len() != 1 {
        return Err(RtError::DynamicError(
            "снимок ХранилищеЗначения содержит неверное число корней".into(),
        ));
    }
    Ok(values.remove(0))
}

pub(crate) static VALUE_STORAGE_METHODS: &[MethodDescriptor] = &[MethodDescriptor::new(
    &["Получить", "Get"],
    Arity::exact(0),
    value_storage_get,
)];
