use std::fmt;
use std::rc::Rc;

use crate::{BslValue, CallContext, RtError, RtResult, TypeId};

/// Неизменяемое описание типа объекта, принадлежащее компоненту.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TypeDescriptor {
    pub package: &'static str,
    pub name: &'static str,
    /// Временная совместимость с закрытым реестром `TypeId`. Официальные
    /// типы сохраняют свой прежний идентификатор; новый host-тип может
    /// оставить `None`, пока типы-значения не переведены на дескрипторы.
    pub legacy_type_id: Option<TypeId>,
}

/// Расширяемый протокол объекта BSL.
///
/// Реализация не получает стек или регистры VM. Операции, которые тип не
/// поддерживает, оставляют реализацию по умолчанию с типизированной
/// ошибкой.
pub trait ObjectProtocol: fmt::Debug {
    fn type_descriptor(&self) -> &'static TypeDescriptor;

    fn get_property(&self, _name: &str, _context: &mut CallContext<'_>) -> RtResult<BslValue> {
        Err(RtError::NotAnObject)
    }

    fn set_property(
        &self,
        _name: &str,
        _value: BslValue,
        _context: &mut CallContext<'_>,
    ) -> RtResult<()> {
        Err(RtError::NotAnObject)
    }

    fn call_method(
        &self,
        name: &str,
        _arguments: &[BslValue],
        _context: &mut CallContext<'_>,
    ) -> RtResult<BslValue> {
        Err(RtError::UnknownMethod {
            method: name.to_string(),
            receiver: self.type_descriptor().name,
        })
    }

    fn get_index(&self, _index: &BslValue) -> RtResult<BslValue> {
        Err(RtError::NotIndexable)
    }

    fn set_index(&self, _index: &BslValue, _value: BslValue) -> RtResult<()> {
        Err(RtError::NotIndexable)
    }

    fn collection_len(&self) -> RtResult<usize> {
        Err(RtError::TypeError {
            expected: "Коллекция",
            op: "Количество",
        })
    }

    fn is_filled(&self) -> RtResult<bool> {
        Err(RtError::TypeError {
            expected: "Значение, у которого есть признак заполненности",
            op: "ЗначениеЗаполнено",
        })
    }

    fn display(&self) -> String {
        self.type_descriptor().name.to_string()
    }
}

/// Ссылочное значение внешнего объекта. Клонирование сохраняет
/// тождественность через `Rc`, как у встроенных ссылочных объектов.
#[derive(Clone)]
pub struct ObjectRef(Rc<dyn ObjectProtocol>);

impl ObjectRef {
    pub fn new(object: impl ObjectProtocol + 'static) -> Self {
        Self(Rc::new(object))
    }

    pub fn type_descriptor(&self) -> &'static TypeDescriptor {
        self.0.type_descriptor()
    }

    pub fn get_property(&self, name: &str, context: &mut CallContext<'_>) -> RtResult<BslValue> {
        self.0.get_property(name, context)
    }

    pub fn set_property(
        &self,
        name: &str,
        value: BslValue,
        context: &mut CallContext<'_>,
    ) -> RtResult<()> {
        self.0.set_property(name, value, context)
    }

    pub fn call_method(
        &self,
        name: &str,
        arguments: &[BslValue],
        context: &mut CallContext<'_>,
    ) -> RtResult<BslValue> {
        self.0.call_method(name, arguments, context)
    }

    pub fn get_index(&self, index: &BslValue) -> RtResult<BslValue> {
        self.0.get_index(index)
    }

    pub fn set_index(&self, index: &BslValue, value: BslValue) -> RtResult<()> {
        self.0.set_index(index, value)
    }

    pub fn collection_len(&self) -> RtResult<usize> {
        self.0.collection_len()
    }

    pub fn is_filled(&self) -> RtResult<bool> {
        self.0.is_filled()
    }

    pub fn display(&self) -> String {
        self.0.display()
    }
}

impl fmt::Debug for ObjectRef {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("ObjectRef")
            .field(&self.type_descriptor())
            .finish()
    }
}
