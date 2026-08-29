//! Минимальный `СписокЗначений`, необходимый canonical query Connector.

use std::cell::RefCell;

use crate::{
    Arity, BslValue, CallContext, EnumValue, MethodDescriptor, ObjectMembersDescriptor,
    ObjectProtocol, PropertyDescriptor, RtError, RtResult, TypeDescriptor,
};

pub(crate) static VALUE_LIST_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "СписокЗначений",
    type_display: "Value list",
    type_names: &["ValueList"],
};

pub(crate) static VALUE_LIST_ITEM_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ЭлементСпискаЗначений",
    type_display: "Value list item",
    type_names: &["ValueListItem"],
};

#[derive(Debug)]
struct ValueListObject {
    items: RefCell<Vec<BslValue>>,
}

impl ObjectProtocol for ValueListObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &VALUE_LIST_TYPE
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        VALUE_LIST_METHODS
    }

    fn get_index(&self, index: &BslValue) -> RtResult<BslValue> {
        let BslValue::Number(index) = index else {
            return Err(RtError::BadIndex);
        };
        let index = index
            .to_i64_exact()
            .and_then(|index| usize::try_from(index).ok())
            .ok_or(RtError::BadIndex)?;
        let items = self.items.borrow();
        items.get(index).cloned().ok_or(RtError::IndexOutOfBounds {
            index: i64::try_from(index).unwrap_or(i64::MAX),
            len: items.len(),
        })
    }

    fn collection_len(&self) -> RtResult<usize> {
        Ok(self.items.borrow().len())
    }
}

#[derive(Debug)]
struct ValueListItemObject {
    value: BslValue,
    presentation: BslValue,
}

impl ObjectProtocol for ValueListItemObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &VALUE_LIST_ITEM_TYPE
    }

    fn property_table(&self) -> &'static [PropertyDescriptor] {
        VALUE_LIST_ITEM_PROPERTIES
    }
}

fn value_list(receiver: &dyn ObjectProtocol) -> RtResult<&ValueListObject> {
    receiver
        .downcast_ref::<ValueListObject>()
        .ok_or(RtError::NotAnObject)
}

fn value_list_item(receiver: &dyn ObjectProtocol) -> RtResult<&ValueListItemObject> {
    receiver
        .downcast_ref::<ValueListItemObject>()
        .ok_or(RtError::NotAnObject)
}

fn count(
    receiver: &dyn ObjectProtocol,
    _arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    Ok(BslValue::number_from_i64(
        value_list(receiver)?.items.borrow().len() as i64,
    ))
}

fn add(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let value = arguments.first().cloned().unwrap_or(BslValue::Undefined);
    let presentation = arguments.get(1).cloned().unwrap_or(BslValue::Undefined);
    let item = BslValue::new_object(ValueListItemObject {
        value,
        presentation,
    });
    value_list(receiver)?.items.borrow_mut().push(item.clone());
    Ok(item)
}

fn sort_by_value(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    if !matches!(
        arguments[0],
        BslValue::Enum(EnumValue::SortDirectionAscending)
    ) {
        return Err(RtError::TypeError {
            expected: "НаправлениеСортировки.Возр",
            op: "СписокЗначений.СортироватьПоЗначению",
        });
    }
    let list = value_list(receiver)?;
    let items = list.items.borrow();
    let mut keyed = Vec::with_capacity(items.len());
    for item in items.iter().cloned() {
        let object = item
            .object_ref()
            .and_then(|object| object.downcast_ref::<ValueListItemObject>())
            .ok_or(RtError::NotAnObject)?;
        let BslValue::Str(value) = &object.value else {
            return Err(RtError::TypeError {
                expected: "Строковое значение элемента",
                op: "СписокЗначений.СортироватьПоЗначению",
            });
        };
        keyed.push((value.to_string(), item));
    }
    drop(items);
    // `sort_by` стабилен: одинаковые значения сохраняют порядок вставки,
    // как показал oracle `HTTP.VALUELIST.SORT.STABLE`.
    keyed.sort_by(|left, right| left.0.cmp(&right.0));
    *list.items.borrow_mut() = keyed.into_iter().map(|(_, item)| item).collect();
    Ok(BslValue::Undefined)
}

fn item_value(receiver: &dyn ObjectProtocol, _context: &mut CallContext<'_>) -> RtResult<BslValue> {
    Ok(value_list_item(receiver)?.value.clone())
}

fn item_presentation(
    receiver: &dyn ObjectProtocol,
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    Ok(value_list_item(receiver)?.presentation.clone())
}

static VALUE_LIST_METHODS: &[MethodDescriptor] = &[
    MethodDescriptor::new(&["Количество", "Count"], Arity::exact(0), count),
    MethodDescriptor::new(&["Добавить", "Add"], Arity::range(0, 2), add),
    MethodDescriptor::new(
        &["СортироватьПоЗначению", "SortByValue"],
        Arity::exact(1),
        sort_by_value,
    ),
];

static VALUE_LIST_ITEM_PROPERTIES: &[PropertyDescriptor] = &[
    PropertyDescriptor {
        names: &["Значение", "Value"],
        get: item_value,
        set: None,
    },
    PropertyDescriptor {
        names: &["Представление", "Presentation"],
        get: item_presentation,
        set: None,
    },
];

pub(crate) const API_MEMBERS: &[ObjectMembersDescriptor] = &[
    ObjectMembersDescriptor::new(&VALUE_LIST_TYPE).with_methods(VALUE_LIST_METHODS),
    ObjectMembersDescriptor::new(&VALUE_LIST_ITEM_TYPE).with_properties(VALUE_LIST_ITEM_PROPERTIES),
];

#[must_use]
pub(crate) fn new_value_list() -> BslValue {
    BslValue::new_object(ValueListObject {
        items: RefCell::new(Vec::new()),
    })
}
