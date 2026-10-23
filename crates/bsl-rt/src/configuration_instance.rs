//! Память конкретного объекта для существующего исполнителя его модуля.

use std::any::Any;
use std::cell::{RefCell, RefMut};
use std::fmt;

use crate::{RtError, RtResult};

/// Экземпляр модуля, принадлежащий одному объекту компонента.
/// Компонент хранит его вместе с объектом; исполнитель заполняет непрозрачную
/// память своими типами. Здесь нет исходников, соединения или второго VM.
#[derive(Default)]
pub struct ConfigurationInstance {
    storage: RefCell<Option<Box<dyn Any>>>,
}

impl ConfigurationInstance {
    /// Создаёт ещё не инициализированный экземпляр.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Служебный доступ исполнителя к памяти экземпляра.
    /// Компонент вызывает модуль через `CallContext`, не разбирая содержимое.
    ///
    /// # Errors
    /// Возвращает ошибку, если память уже заимствована исполнителем.
    pub fn storage(&self) -> RtResult<RefMut<'_, Option<Box<dyn Any>>>> {
        self.storage
            .try_borrow_mut()
            .map_err(|_| RtError::DynamicError("память экземпляра модуля уже заимствована".into()))
    }
}

impl fmt::Debug for ConfigurationInstance {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ConfigurationInstance")
            .finish_non_exhaustive()
    }
}
