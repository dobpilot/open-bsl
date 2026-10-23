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
        check: match arguments.get(2) {
            Some(BslValue::Undefined) | None => BslValue::Boolean(false),
            Some(value) => value.clone(),
        },
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

fn presentation_key(text: &str) -> (Vec<char>, Vec<(bool, bool)>) {
    // Сравнение таблицы сворачивает регистр латиницы и не различает е/ё
    // вторичным ключом так, как измеренный метод списка. Поэтому её
    // `collate` здесь неприменим; контроль содержит смешанные строки.
    // НЕ ИЗМЕРЕНО(VLIST.PRESENTATION.UNICODE): прочие алфавиты пока
    // сравниваются по кодам символов, без полной таблицы колляции Unicode.
    let mut primary = Vec::new();
    let mut secondary = Vec::new();
    for character in text.chars() {
        let folded = match character {
            'А'..='Я' => char::from_u32(character as u32 + 32).unwrap_or(character),
            'Ё' | 'ё' => 'е',
            _ => character,
        };
        primary.push(folded);
        secondary.push((
            matches!(character, 'Ё' | 'ё'),
            matches!(character, 'А'..='Я' | 'Ё'),
        ));
    }
    (primary, secondary)
}

fn sort_by_presentation(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let descending = match arguments.first() {
        None | Some(BslValue::Undefined | BslValue::Enum(EnumValue::SortDirectionAscending)) => {
            false
        }
        Some(BslValue::Enum(EnumValue::SortDirectionDescending)) => true,
        _ => {
            return Err(RtError::TypeError {
                expected: "НаправлениеСортировки",
                op: "СписокЗначений.СортироватьПоПредставлению",
            });
        }
    };
    let list = value_list(receiver)?;
    let items = list.items.borrow();
    let mut keyed = Vec::with_capacity(items.len());
    for item in items.iter().cloned() {
        let object = item
            .object_ref()
            .and_then(|object| object.downcast_ref::<ValueListItemObject>())
            .ok_or(RtError::NotAnObject)?;
        let text = match &object.presentation {
            BslValue::Undefined => String::new(),
            BslValue::Str(text) => text.to_string(),
            // Числовое представление допустимо на платформе. Преобразуется
            // только ключ, само свойство существующего элемента не меняется.
            // НЕ ИЗМЕРЕНО(VLIST.PRESENTATION.OTHER_TYPES): прочие типы
            // используют общий форматтер до отдельного нативного контроля.
            value => context.format_value(value, None)?,
        };
        keyed.push((presentation_key(&text), item));
    }
    drop(items);
    // Стабильность равных ключей измерена для обоих направлений;
    // переворачивается сравнение, а не готовый список.
    keyed.sort_by(|left, right| {
        if descending {
            right.0.cmp(&left.0)
        } else {
            left.0.cmp(&right.0)
        }
    });
    *list.items.borrow_mut() = keyed.into_iter().map(|(_, item)| item).collect();
    Ok(BslValue::Undefined)
}

fn unload_values(
    receiver: &dyn ObjectProtocol,
    _arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let items = value_list(receiver)?.items.borrow();
    let mut values = Vec::with_capacity(items.len());
    for item in items.iter() {
        let object = item
            .object_ref()
            .and_then(|object| object.downcast_ref::<ValueListItemObject>())
            .ok_or(RtError::NotAnObject)?;
        // Массив имеет собственные ячейки, но изменяемые значения сохраняют
        // идентичность: изменение вложенной структуры измерено на 1С.
        values.push(object.value.clone());
    }
    Ok(BslValue::new_array(values))
}

fn find_by_value(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    // Нативный поиск возвращает первый существующий элемент, сохраняя его тождество.
    for item in value_list(receiver)?.items.borrow().iter() {
        let object = item.object_ref().ok_or(RtError::NotAnObject)?;
        if value_list_item(object.as_dyn())?.value == arguments[0] {
            return Ok(item.clone());
        }
    }
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
    MethodDescriptor::new(
        &["ВыгрузитьЗначения", "UnloadValues"],
        Arity::exact(0),
        unload_values,
    ),
    MethodDescriptor::new(
        &["СортироватьПоПредставлению", "SortByPresentation"],
        Arity::range(0, 1),
        sort_by_presentation,
    )
    .as_procedure(),
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
        &["НайтиПоЗначению", "FindByValue"],
        Arity::exact(1),
        find_by_value,
    ),
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

/// Создаёт список значений из готовых значений элементов.
///
/// Представление и картинка каждого элемента остаются
/// `Неопределено`, а пометка — `Ложь`, как при вызове BSL-метода
/// `Добавить` только с первым аргументом.
#[must_use]
pub fn value_list_from_values(values: impl IntoIterator<Item = BslValue>) -> BslValue {
    BslValue::new_object(ValueListObject {
        items: RefCell::new(values.into_iter().map(|value| new_item(&[value])).collect()),
    })
}

#[cfg(test)]
#[path = "value_list_presentation_tests.rs"]
mod presentation_tests;

#[cfg(test)]
#[path = "value_list_unload_tests.rs"]
mod unload_tests;

#[cfg(test)]
#[path = "value_list_find_tests.rs"]
mod find_tests;

#[cfg(test)]
mod tests {
    use crate::BslValue;

    #[test]
    fn an_empty_value_list_has_the_measured_empty_string_representation() {
        assert_eq!(super::new_value_list().to_string(), "");
    }

    #[test]
    fn rust_factory_uses_the_same_default_item_shape_as_add() {
        let list = super::value_list_from_values([BslValue::Str("ключ".into())]);
        assert_eq!(list.collection_len().unwrap(), 1);
        let item = list
            .get_index(&BslValue::number_from_i64(0), &crate::NameInterner::new())
            .unwrap();
        let object = item
            .object_ref()
            .and_then(|object| object.downcast_ref::<super::ValueListItemObject>())
            .unwrap();
        assert_eq!(object.value, BslValue::Str("ключ".into()));
        assert_eq!(object.presentation, BslValue::Undefined);
        assert_eq!(object.check, BslValue::Boolean(false));
        assert_eq!(object.picture, BslValue::Undefined);
    }

    #[test]
    fn an_undefined_check_position_uses_the_measured_default() {
        let item = super::new_item(&[
            BslValue::Undefined,
            BslValue::Undefined,
            BslValue::Undefined,
            BslValue::Undefined,
        ]);

        let item = item
            .object_ref()
            .and_then(|object| object.downcast_ref::<super::ValueListItemObject>())
            .expect("new_item возвращает элемент списка значений");
        assert_eq!(item.check, BslValue::Boolean(false));
    }
}
