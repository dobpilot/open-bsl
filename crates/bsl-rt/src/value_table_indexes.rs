//! Коллекция описаний индексов `ТаблицыЗначений`.
//!
//! Индекс хранит измеренную объектную поверхность, но не дублирует поиск
//! строк: `Найти` и `НайтиСтроки` остаются единственными алгоритмами таблицы.

use std::cell::RefCell;
use std::rc::Rc;

use crate::{
    Arity, BslValue, CallContext, MethodDescriptor, ObjectMembersDescriptor, ObjectProtocol,
    RtError, RtResult, TypeDescriptor, ValueTableData, receiver_of,
};

pub(crate) static VALUE_TABLE_INDEXES_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "КоллекцияИндексовТаблицыЗначений",
    type_display: "Value table index collection",
    type_names: &["ValueTableIndexCollection"],
};

pub(crate) static VALUE_TABLE_INDEX_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ИндексТаблицыЗначений",
    type_display: "Value table index",
    type_names: &["ValueTableIndex"],
};

#[derive(Debug)]
pub(crate) struct ValueTableIndexesObject {
    data: Rc<RefCell<ValueTableData>>,
}

impl ObjectProtocol for ValueTableIndexesObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &VALUE_TABLE_INDEXES_TYPE
    }

    fn identity_key(&self) -> Option<(usize, usize)> {
        Some((Rc::as_ptr(&self.data) as usize, 0))
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        METHODS
    }

    fn get_index(&self, index: &BslValue) -> RtResult<BslValue> {
        let position = exact_index(index)?;
        let data = self.data.borrow();
        let entry = data
            .indexes
            .get(position)
            .ok_or(RtError::IndexOutOfBounds {
                index: i64::try_from(position).unwrap_or(i64::MAX),
                len: data.indexes.len(),
            })?;
        Ok(new_index(self.data.clone(), entry.token.clone()))
    }

    fn collection_len(&self) -> RtResult<usize> {
        Ok(self.data.borrow().indexes.len())
    }
}

#[derive(Debug)]
struct ValueTableIndexObject {
    data: Rc<RefCell<ValueTableData>>,
    token: Rc<()>,
}

impl ObjectProtocol for ValueTableIndexObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &VALUE_TABLE_INDEX_TYPE
    }

    fn identity_key(&self) -> Option<(usize, usize)> {
        Some((Rc::as_ptr(&self.token) as usize, 0))
    }
}

#[derive(Debug)]
pub(crate) struct ValueTableIndexData {
    pub columns: String,
    token: Rc<()>,
}

impl ValueTableIndexData {
    pub(crate) fn independent_copy(&self) -> Self {
        Self {
            columns: self.columns.clone(),
            token: Rc::new(()),
        }
    }
}

fn exact_index(value: &BslValue) -> RtResult<usize> {
    let BslValue::Number(number) = value else {
        return Err(RtError::BadIndex);
    };
    number
        .to_i64_exact()
        .and_then(|value| usize::try_from(value).ok())
        .ok_or(RtError::BadIndex)
}

fn indexes(receiver: &dyn ObjectProtocol) -> RtResult<&ValueTableIndexesObject> {
    receiver_of::<ValueTableIndexesObject>(receiver, "ИндексыТаблицыЗначений")
}

fn count(
    receiver: &dyn ObjectProtocol,
    _arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    Ok(BslValue::number_from_i64(
        indexes(receiver)?.data.borrow().indexes.len() as i64,
    ))
}

fn add(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let spec = match arguments {
        [] => String::new(),
        [value] => value.as_str("Индексы.Добавить")?.to_string(),
        _ => unreachable!("арность проверена дескриптором"),
    };
    let indexes = indexes(receiver)?;
    let mut data = indexes.data.borrow_mut();
    for name in spec
        .split(',')
        .map(str::trim)
        .filter(|name| !name.is_empty())
    {
        if data.column_index(name).is_none() {
            return Err(RtError::UnknownColumn(name.to_string()));
        }
    }
    data.indexes
        .try_reserve(1)
        .map_err(|_| RtError::ResourceLimit("не удалось добавить индекс таблицы".into()))?;
    let token = Rc::new(());
    data.indexes.push(ValueTableIndexData {
        columns: spec,
        token: token.clone(),
    });
    drop(data);
    Ok(new_index(indexes.data.clone(), token))
}

fn delete(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let indexes = indexes(receiver)?;
    let item = arguments[0]
        .object_ref()
        .and_then(|object| object.downcast_ref::<ValueTableIndexObject>())
        .filter(|item| Rc::ptr_eq(&item.data, &indexes.data))
        .ok_or(RtError::TypeError {
            expected: "ИндексТаблицыЗначений этой коллекции",
            op: "Индексы.Удалить",
        })?;
    let mut data = indexes.data.borrow_mut();
    let position = data
        .indexes
        .iter()
        .position(|entry| Rc::ptr_eq(&entry.token, &item.token))
        .ok_or(RtError::BadIndex)?;
    data.indexes.remove(position);
    Ok(BslValue::Undefined)
}

fn clear(
    receiver: &dyn ObjectProtocol,
    _arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    indexes(receiver)?.data.borrow_mut().indexes.clear();
    Ok(BslValue::Undefined)
}

static METHODS: &[MethodDescriptor] = &[
    MethodDescriptor::new(&["Количество", "Count"], Arity::exact(0), count),
    MethodDescriptor::new(&["Добавить", "Add"], Arity::range(0, 1), add),
    MethodDescriptor::new(&["Удалить", "Delete"], Arity::exact(1), delete).as_procedure(),
    MethodDescriptor::new(&["Очистить", "Clear"], Arity::exact(0), clear).as_procedure(),
];

pub(crate) const API_MEMBERS: &[ObjectMembersDescriptor] = &[
    ObjectMembersDescriptor::new(&VALUE_TABLE_INDEXES_TYPE).with_methods(METHODS),
    ObjectMembersDescriptor::new(&VALUE_TABLE_INDEX_TYPE),
];

#[must_use]
pub(crate) fn new_indexes(data: Rc<RefCell<ValueTableData>>) -> BslValue {
    BslValue::new_object(ValueTableIndexesObject { data })
}

fn new_index(data: Rc<RefCell<ValueTableData>>, token: Rc<()>) -> BslValue {
    BslValue::new_object(ValueTableIndexObject { data, token })
}

pub(crate) fn is_index(object: &crate::ObjectRef) -> bool {
    object.downcast_ref::<ValueTableIndexObject>().is_some()
}
