//! Измеренное начальное состояние графической схемы без элементов.

use bsl_rt::{
    Arity, BslValue, CallContext, MethodDescriptor, ObjectMembersDescriptor, ObjectProtocol,
    PropertyDescriptor, RtError, RtResult, TypeDescriptor,
};

pub(crate) static SCHEMA_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ГрафическаяСхема",
    type_display: "Графическая схема",
    type_names: &["GraphicalSchema"],
};

pub(crate) static ITEMS_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ЭлементыГрафическойСхемы",
    type_display: "Элементы графической схемы",
    type_names: &["GraphicalSchemaItems"],
};

#[derive(Debug)]
struct Schema;

#[derive(Debug)]
struct Items;

pub(crate) fn construct(_: &mut CallContext<'_>, _: &[BslValue]) -> RtResult<BslValue> {
    Ok(BslValue::new_object(Schema))
}

fn items(_: &dyn ObjectProtocol, _: &mut CallContext<'_>) -> RtResult<BslValue> {
    // На 1С повторные чтения свойства сравниваются как разные обёртки.
    // Эта коллекция описывает только новую пустую схему, а не карту маршрута.
    Ok(BslValue::new_object(Items))
}

fn count(_: &dyn ObjectProtocol, _: &[BslValue], _: &mut CallContext<'_>) -> RtResult<BslValue> {
    Ok(BslValue::number_from_i64(0))
}

fn find(
    _: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    if !matches!(arguments, [BslValue::Str(_)]) {
        return Err(RtError::TypeError {
            expected: "Строка",
            op: "Найти",
        });
    }
    Ok(BslValue::Undefined)
}

const PROPERTIES: &[PropertyDescriptor] = &[PropertyDescriptor {
    names: &["ЭлементыГрафическойСхемы", "GraphicalSchemaItems"],
    get: items,
    set: None,
}];

const METHODS: &[MethodDescriptor] = &[
    MethodDescriptor::new(&["Количество", "Count"], Arity::exact(0), count),
    MethodDescriptor::new(&["Найти", "Find"], Arity::exact(1), find),
];

impl ObjectProtocol for Schema {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &SCHEMA_TYPE
    }

    fn property_table(&self) -> &'static [PropertyDescriptor] {
        PROPERTIES
    }

    fn display(&self) -> String {
        SCHEMA_TYPE.type_display.into()
    }
}

impl ObjectProtocol for Items {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &ITEMS_TYPE
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        METHODS
    }

    fn collection_len(&self) -> RtResult<usize> {
        Ok(0)
    }

    fn display(&self) -> String {
        ITEMS_TYPE.type_display.into()
    }
}

pub(crate) const API_MEMBERS: &[ObjectMembersDescriptor] = &[
    ObjectMembersDescriptor::new(&SCHEMA_TYPE).with_properties(PROPERTIES),
    ObjectMembersDescriptor::new(&ITEMS_TYPE).with_methods(METHODS),
];
