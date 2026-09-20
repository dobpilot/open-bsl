//! Неизменяемые снимки `Структуры` и `Соответствия`.

use crate::{
    Arity, BslObject, BslValue, CallContext, MethodDescriptor, ObjectMembersDescriptor,
    ObjectProtocol, RtError, RtResult, TypeDescriptor, folded_eq, receiver_of,
};

pub(crate) static FIXED_STRUCTURE_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ФиксированнаяСтруктура",
    type_display: "Fixed structure",
    type_names: &["FixedStructure"],
};

pub(crate) static FIXED_MAP_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ФиксированноеСоответствие",
    type_display: "Fixed map",
    type_names: &["FixedMap"],
};

#[derive(Debug)]
struct FixedStructureObject {
    fields: Vec<(String, BslValue)>,
}

impl FixedStructureObject {
    fn find(&self, name: &str) -> Option<BslValue> {
        self.fields
            .iter()
            .find(|(candidate, _)| folded_eq(candidate, name))
            .map(|(_, value)| value.clone())
    }
}

impl ObjectProtocol for FixedStructureObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &FIXED_STRUCTURE_TYPE
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        FIXED_STRUCTURE_METHODS
    }

    fn get_property(&self, name: &str, _context: &mut CallContext<'_>) -> RtResult<BslValue> {
        self.find(name)
            .ok_or_else(|| RtError::UnknownProperty(name.to_string()))
    }
}

#[derive(Debug)]
struct FixedMapObject {
    entries: Vec<(BslValue, BslValue)>,
}

impl FixedMapObject {
    fn get(&self, key: &BslValue) -> BslValue {
        self.entries
            .iter()
            .find(|(candidate, _)| candidate == key)
            .map(|(_, value)| value.clone())
            .unwrap_or(BslValue::Undefined)
    }
}

impl ObjectProtocol for FixedMapObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &FIXED_MAP_TYPE
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        FIXED_MAP_METHODS
    }
}

fn fixed_structure(receiver: &dyn ObjectProtocol) -> RtResult<&FixedStructureObject> {
    receiver_of::<FixedStructureObject>(receiver, "ФиксированнаяСтруктура")
}

fn fixed_map(receiver: &dyn ObjectProtocol) -> RtResult<&FixedMapObject> {
    receiver_of::<FixedMapObject>(receiver, "ФиксированноеСоответствие")
}

fn structure_count(
    receiver: &dyn ObjectProtocol,
    _arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    Ok(BslValue::number_from_i64(
        fixed_structure(receiver)?.fields.len() as i64,
    ))
}

fn structure_property(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let name = arguments[0]
        .as_str("ФиксированнаяСтруктура.Свойство")?
        .to_string();
    let value = fixed_structure(receiver)?.find(&name);
    Ok(BslValue::Boolean(value.is_some()))
}

fn map_count(
    receiver: &dyn ObjectProtocol,
    _arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    Ok(BslValue::number_from_i64(
        fixed_map(receiver)?.entries.len() as i64,
    ))
}

fn map_get(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    Ok(fixed_map(receiver)?.get(&arguments[0]))
}

static FIXED_STRUCTURE_METHODS: &[MethodDescriptor] = &[
    MethodDescriptor::new(&["Количество", "Count"], Arity::exact(0), structure_count),
    MethodDescriptor::new(
        &["Свойство", "Property"],
        Arity::range(1, 2),
        structure_property,
    ),
];

static FIXED_MAP_METHODS: &[MethodDescriptor] = &[
    MethodDescriptor::new(&["Количество", "Count"], Arity::exact(0), map_count),
    MethodDescriptor::new(&["Получить", "Get"], Arity::exact(1), map_get),
];

pub(crate) const API_MEMBERS: &[ObjectMembersDescriptor] = &[
    ObjectMembersDescriptor::new(&FIXED_STRUCTURE_TYPE).with_methods(FIXED_STRUCTURE_METHODS),
    ObjectMembersDescriptor::new(&FIXED_MAP_TYPE).with_methods(FIXED_MAP_METHODS),
];

pub(crate) fn construct_fixed_structure(
    context: &mut CallContext<'_>,
    arguments: &[BslValue],
) -> RtResult<BslValue> {
    let Some(source) = arguments.first() else {
        return Ok(BslValue::new_object(FixedStructureObject {
            fields: Vec::new(),
        }));
    };
    let BslValue::Object(object) = source else {
        return Err(RtError::TypeError {
            expected: "Структура",
            op: "ФиксированнаяСтруктура",
        });
    };
    let BslObject::Structure(storage) = object.as_ref() else {
        return Err(RtError::TypeError {
            expected: "Структура",
            op: "ФиксированнаяСтруктура",
        });
    };
    let storage = storage.borrow();
    let mut fields = Vec::new();
    fields
        .try_reserve(storage.len())
        .map_err(|_| RtError::ResourceLimit("не удалось создать фиксированную структуру".into()))?;
    for index in 0..storage.len() {
        let (name, value) = storage.entry_at(index).ok_or(RtError::NotAnObject)?;
        let name = context
            .runtime_shapes()
            .names
            .name(name)
            .ok_or(RtError::UnknownField(name))?
            .to_string();
        fields.push((name, value));
    }
    Ok(BslValue::new_object(FixedStructureObject { fields }))
}

pub(crate) fn construct_fixed_map(
    _context: &mut CallContext<'_>,
    arguments: &[BslValue],
) -> RtResult<BslValue> {
    let BslValue::Object(object) = &arguments[0] else {
        return Err(RtError::TypeError {
            expected: "Соответствие",
            op: "ФиксированноеСоответствие",
        });
    };
    let BslObject::Map(source) = object.as_ref() else {
        return Err(RtError::TypeError {
            expected: "Соответствие",
            op: "ФиксированноеСоответствие",
        });
    };
    let source = source.borrow();
    let mut entries = Vec::new();
    entries.try_reserve(source.len()).map_err(|_| {
        RtError::ResourceLimit("не удалось создать фиксированное соответствие".into())
    })?;
    for index in 0..source.len() {
        entries.push(source.entry_at(index).ok_or(RtError::NotAnObject)?);
    }
    Ok(BslValue::new_object(FixedMapObject { entries }))
}
