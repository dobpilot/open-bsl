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

    fn display(&self) -> String {
        // file-constructor-more-types: пустой СписокЗначений на платформе
        // печатается пустой строкой. Заполненный список не измерен.
        if self.items.borrow().is_empty() {
            String::new()
        } else {
            VALUE_LIST_TYPE.name.to_owned()
        }
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
    check: BslValue,
    picture: BslValue,
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
    let item = new_item(arguments);
    let list = value_list(receiver)?;
    let mut items = list.items.borrow_mut();
    items
        .try_reserve(1)
        .map_err(|_| RtError::ResourceLimit("не удалось расширить список значений".into()))?;
    items.push(item.clone());
    Ok(item)
}

fn new_item(arguments: &[BslValue]) -> BslValue {
    BslValue::new_object(ValueListItemObject {
        value: arguments.first().cloned().unwrap_or(BslValue::Undefined),
        presentation: arguments.get(1).cloned().unwrap_or(BslValue::Undefined),
        check: arguments
            .get(2)
            .cloned()
            .unwrap_or(BslValue::Boolean(false)),
        picture: arguments.get(3).cloned().unwrap_or(BslValue::Undefined),
    })
}

fn integer_argument(value: &BslValue) -> RtResult<i64> {
    let BslValue::Number(number) = BslValue::array_method_index(value)? else {
        unreachable!("array_method_index всегда возвращает число");
    };
    number.to_i64_exact().ok_or(RtError::BadIndex)
}

fn list_index(value: &BslValue) -> RtResult<usize> {
    usize::try_from(integer_argument(value)?).map_err(|_| RtError::BadIndex)
}

fn own_item_position(items: &[BslValue], item: &BslValue) -> RtResult<Option<usize>> {
    let valid_item = item
        .object_ref()
        .and_then(|object| object.downcast_ref::<ValueListItemObject>())
        .is_some();
    if !valid_item {
        return Err(RtError::TypeError {
            expected: "ЭлементСпискаЗначений",
            op: "СписокЗначений",
        });
    }
    Ok(items.iter().position(|candidate| candidate == item))
}

fn delete(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let list = value_list(receiver)?;
    let mut items = list.items.borrow_mut();
    let Some(position) = own_item_position(&items, &arguments[0])? else {
        return Err(RtError::BadIndex);
    };
    items.remove(position);
    Ok(BslValue::Undefined)
}

fn clear(
    receiver: &dyn ObjectProtocol,
    _arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    value_list(receiver)?.items.borrow_mut().clear();
    Ok(BslValue::Undefined)
}

fn insert(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let position = list_index(&arguments[0])?;
    let item = new_item(&arguments[1..]);
    let list = value_list(receiver)?;
    let mut items = list.items.borrow_mut();
    if position > items.len() {
        return Err(RtError::IndexOutOfBounds {
            index: i64::try_from(position).unwrap_or(i64::MAX),
            len: items.len(),
        });
    }
    items
        .try_reserve(1)
        .map_err(|_| RtError::ResourceLimit("не удалось расширить список значений".into()))?;
    items.insert(position, item.clone());
    Ok(item)
}

fn get(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let position = list_index(&arguments[0])?;
    let items = value_list(receiver)?.items.borrow();
    items
        .get(position)
        .cloned()
        .ok_or(RtError::IndexOutOfBounds {
            index: i64::try_from(position).unwrap_or(i64::MAX),
            len: items.len(),
        })
}

fn copy(
    receiver: &dyn ObjectProtocol,
    _arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let source = value_list(receiver)?.items.borrow();
    let mut items = Vec::new();
    items
        .try_reserve(source.len())
        .map_err(|_| RtError::ResourceLimit("не удалось скопировать список значений".into()))?;
    for item in source.iter() {
        let object = item
            .object_ref()
            .and_then(|object| object.downcast_ref::<ValueListItemObject>())
            .ok_or(RtError::NotAnObject)?;
        items.push(BslValue::new_object(ValueListItemObject {
            value: object.value.clone(),
            presentation: object.presentation.clone(),
            check: object.check.clone(),
            picture: object.picture.clone(),
        }));
    }
    Ok(BslValue::new_object(ValueListObject {
        items: RefCell::new(items),
    }))
}

fn move_item(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let offset = integer_argument(&arguments[1])?;
    let list = value_list(receiver)?;
    let mut items = list.items.borrow_mut();
    // Платформа для элемента другого списка выбирает первый элемент
    // получателя; три позиции чужого списка дали один результат.
    let position = own_item_position(&items, &arguments[0])?.unwrap_or(0);
    let target = i64::try_from(position)
        .ok()
        .and_then(|position| position.checked_add(offset))
        .and_then(|position| usize::try_from(position).ok())
        .filter(|target| *target < items.len())
        .ok_or(RtError::BadIndex)?;
    if target != position {
        let item = items.remove(position);
        items.insert(target, item);
    }
    Ok(BslValue::Undefined)
}

fn index_of(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let items = value_list(receiver)?.items.borrow();
    let position = own_item_position(&items, &arguments[0])?
        .and_then(|position| i64::try_from(position).ok())
        .unwrap_or(-1);
    Ok(BslValue::number_from_i64(position))
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

fn item_check(receiver: &dyn ObjectProtocol, _context: &mut CallContext<'_>) -> RtResult<BslValue> {
    Ok(value_list_item(receiver)?.check.clone())
}

fn item_picture(
    receiver: &dyn ObjectProtocol,
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    Ok(value_list_item(receiver)?.picture.clone())
}

static VALUE_LIST_METHODS: &[MethodDescriptor] = &[
    MethodDescriptor::new(&["Количество", "Count"], Arity::exact(0), count),
    MethodDescriptor::new(&["Добавить", "Add"], Arity::range(0, 4), add),
    MethodDescriptor::new(&["Удалить", "Delete"], Arity::exact(1), delete).as_procedure(),
    MethodDescriptor::new(&["Очистить", "Clear"], Arity::exact(0), clear).as_procedure(),
    MethodDescriptor::new(&["Вставить", "Insert"], Arity::range(1, 5), insert),
    MethodDescriptor::new(&["Получить", "Get"], Arity::exact(1), get),
    MethodDescriptor::new(&["Скопировать", "Copy"], Arity::exact(0), copy),
    MethodDescriptor::new(&["Сдвинуть", "Move"], Arity::exact(2), move_item).as_procedure(),
    MethodDescriptor::new(&["Индекс", "IndexOf"], Arity::exact(1), index_of),
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
    PropertyDescriptor {
        names: &["Пометка", "Check"],
        get: item_check,
        set: None,
    },
    PropertyDescriptor {
        names: &["Картинка", "Picture"],
        get: item_picture,
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

#[cfg(test)]
mod tests {
    #[test]
    fn an_empty_value_list_has_the_measured_empty_string_representation() {
        assert_eq!(super::new_value_list().to_string(), "");
    }
}
