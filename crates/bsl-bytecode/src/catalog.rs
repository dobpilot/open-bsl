//! Граница между индексом модулей и подготовленным исполняемым кодом.

use bsl_rt::RtError;

use crate::{ConfigurationProgram, ModuleId, ModuleProgram};

/// Каталог со стабильными номерами модулей и подготовкой кода по требованию.
///
/// Поиск имени и проверка наличия подготовленного кода не должны загружать
/// исходники. Возвращённая программа неизменна до конца жизни каталога;
/// подготовка следующего модуля не меняет ранее опубликованные номера.
pub trait ExecutionCatalog {
    /// Возвращает число объявленных модулей, включая ещё не подготовленные.
    fn module_count(&self) -> usize;

    /// Возвращает имя объявленного модуля без подготовки его кода.
    fn module_name(&self, module: ModuleId) -> Option<&str>;

    /// Возвращает уже подготовленную программу без запуска подготовки.
    fn prepared_module(&self, module: ModuleId) -> Option<&ModuleProgram>;

    /// Возвращает исполняемую программу, при необходимости подготавливая её.
    ///
    /// # Errors
    /// Возвращает ошибку загрузки, компиляции или проверки программы либо
    /// `RtError::InvalidBytecode`, если номер отсутствует в индексе.
    fn module_code(&self, module: ModuleId) -> Result<&ModuleProgram, RtError>;

    /// Находит номер по имени без учёта регистра и без подготовки кода.
    fn module_id(&self, name: &str) -> Option<ModuleId> {
        (0..self.module_count()).find_map(|index| {
            let id = ModuleId::new(u32::try_from(index).ok()?);
            self.module_name(id)
                .filter(|candidate| bsl_rt::folded_eq(candidate, name))
                .map(|_| id)
        })
    }
}

impl ExecutionCatalog for ConfigurationProgram {
    fn module_count(&self) -> usize {
        self.modules.len()
    }

    fn module_name(&self, module: ModuleId) -> Option<&str> {
        self.module(module).map(|module| module.name.as_str())
    }

    fn prepared_module(&self, module: ModuleId) -> Option<&ModuleProgram> {
        self.module(module)
    }

    fn module_code(&self, module: ModuleId) -> Result<&ModuleProgram, RtError> {
        self.module(module).ok_or(RtError::InvalidBytecode(
            "номер модуля вне каталога конфигурации",
        ))
    }
}
