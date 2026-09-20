//! Учёт собственных временных файлов без политики удаления и восстановления.

use std::cell::RefCell;
use std::fmt::Debug;
use std::io;
use std::rc::Rc;

/// Принадлежащий host временный ресурс с правом удаления конкретного объекта.
///
/// Реализация не должна удалять чужую подмену по сохранённому пути. Путь
/// служит описанием, а доказательство владения хранится внутри реализации.
/// Освобождение дескриптора без [`Self::remove`] не должно удалять файл.
pub trait TemporaryFileResource: Debug {
    /// Исходный путь для host; сам по себе не даёт права удаления.
    fn path(&self) -> &str;

    /// Удаляет только собственный ресурс по правилам создавшего его host.
    ///
    /// # Errors
    ///
    /// Отказ удаления или невозможность безопасно подтвердить владение.
    fn remove(self: Box<Self>) -> io::Result<()>;
}

/// Callback получает владение пакетом, включая ответственность за ошибки
/// отдельных ресурсов. Итоговая ошибка предназначена для диагностики,
/// а не изменения результата BSL. Callback не должен паниковать: фасад
/// вызывает его также при освобождении сеанса.
pub type TemporaryFileCleanup = dyn FnMut(Vec<Box<dyn TemporaryFileResource>>) -> io::Result<()>;

#[derive(Debug, Default)]
struct Registry {
    closed: bool,
    pending: Vec<Box<dyn TemporaryFileResource>>,
}

/// Локальный учёт файлов одного сеанса; клоны разделяют именно этот учёт.
///
/// Не связан с временным хранилищем BSL. Не сканирует каталоги и не удаляет
/// файлы при Drop; host явно забирает ресурсы и выбирает политику очистки.
#[derive(Clone, Debug, Default)]
pub struct TemporaryFileRegistry(Rc<RefCell<Registry>>);

impl TemporaryFileRegistry {
    /// Закрыт ли исходный сеанс. Позволяет отказаться до создания нового
    /// носителя; окончательная проверка остаётся в `register`.
    #[must_use]
    pub fn is_closed(&self) -> bool {
        self.0.borrow().closed
    }

    /// Передаёт владение ресурсом сеансу без файловых операций.
    ///
    /// # Errors
    ///
    /// После закрытия учёта возвращает переданный ресурс без удаления.
    pub fn register(
        &self,
        resource: Box<dyn TemporaryFileResource>,
    ) -> Result<(), Box<dyn TemporaryFileResource>> {
        let mut registry = self.0.borrow_mut();
        if registry.closed {
            return Err(resource);
        }
        registry.pending.push(resource);
        Ok(())
    }

    /// Передаёт накопленный пакет host, оставляя сеанс открытым.
    /// Повторный вызов без новых регистраций возвращает пустой пакет.
    pub fn take_pending(&self) -> Vec<Box<dyn TemporaryFileResource>> {
        std::mem::take(&mut self.0.borrow_mut().pending)
    }

    /// Закрывает учёт и передаёт последний пакет host. Поздняя регистрация
    /// будет отклонена даже через сохранённый клон этого дескриптора.
    pub fn close(&self) -> Vec<Box<dyn TemporaryFileResource>> {
        let mut registry = self.0.borrow_mut();
        registry.closed = true;
        std::mem::take(&mut registry.pending)
    }
}
