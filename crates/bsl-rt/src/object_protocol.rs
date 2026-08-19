use std::any::Any;
use std::fmt;
use std::rc::Rc;

use crate::{BslValue, CallContext, RtError, RtResult, TypeId};

/// Байтовый поток, который можно передать другому runtime-компоненту.
///
/// Протокол не раскрывает конкретный носитель: ZIP, читатели данных и
/// host-объекты работают с одной границей, не зная о `File` или буфере.
pub trait ByteStreamProtocol: fmt::Debug {
    /// Текущая позиция без её изменения.
    fn position(&self, op: &'static str) -> RtResult<u64>;

    /// Переводит позицию, не меняя размер носителя.
    fn set_position(&self, position: u64, op: &'static str) -> RtResult<()>;

    /// Длина носителя в байтах.
    fn len(&self, op: &'static str) -> RtResult<u64>;

    /// Читает не больше `count` байт и сдвигает позицию.
    fn read_bytes(&self, count: usize, op: &'static str) -> RtResult<Vec<u8>>;

    /// Записывает байты с текущей позиции и сдвигает её.
    fn write_bytes(&self, bytes: &[u8], op: &'static str) -> RtResult<()>;

    /// Читает поток целиком и восстанавливает исходную позицию.
    fn read_all(&self, op: &'static str) -> RtResult<Vec<u8>> {
        let len = self.len(op)?;
        let count = usize::try_from(len).map_err(|_| RtError::TypeError {
            expected: "Поток, размер которого умещается в памяти",
            op,
        })?;
        let position = self.position(op)?;
        self.set_position(0, op)?;
        let result = self.read_bytes(count, op);
        let restore = self.set_position(position, op);
        match result {
            Err(error) => Err(error),
            Ok(bytes) => {
                restore?;
                Ok(bytes)
            }
        }
    }

    /// Записывает готовый блок с текущей позиции, не закрывая поток.
    fn write_all(&self, bytes: &[u8], op: &'static str) -> RtResult<()> {
        self.write_bytes(bytes, op)
    }
}

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

/// Служебная основа безопасного downcast без обязательного метода в
/// каждой реализации [`ObjectProtocol`].
#[doc(hidden)]
pub trait ObjectDowncast {
    fn as_any(&self) -> &dyn Any;
}

impl<T: Any> ObjectDowncast for T {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Расширяемый протокол объекта BSL.
///
/// Реализация не получает стек или регистры VM. Операции, которые тип не
/// поддерживает, оставляют реализацию по умолчанию с типизированной
/// ошибкой.
pub trait ObjectProtocol: fmt::Debug + ObjectDowncast {
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

    /// Возвращает потоковую возможность объекта, если он ею является.
    fn byte_stream(&self) -> Option<&dyn ByteStreamProtocol> {
        None
    }

    /// Ключ ссылочного равенства для типов, равных «по месту в состоянии»,
    /// а не по тождеству обёртки: адрес состояния и номер внутри него.
    /// Обёртку такие типы строят на каждое обращение к коллекции, поэтому
    /// тождества значений недостаточно. `None` оставляет объекту равенство
    /// только по тождеству.
    fn identity_key(&self) -> Option<(usize, usize)> {
        None
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

    /// Возвращает конкретную реализацию объекта, если она имеет тип `T`.
    pub fn downcast_ref<T: ObjectProtocol + 'static>(&self) -> Option<&T> {
        self.0.as_ref().as_any().downcast_ref()
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

    pub fn byte_stream(&self) -> Option<&dyn ByteStreamProtocol> {
        self.0.byte_stream()
    }

    pub fn identity_key(&self) -> Option<(usize, usize)> {
        self.0.identity_key()
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
