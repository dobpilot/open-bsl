//! Нейтральный снимок исходного кода конфигурации 1С.
//!
//! Крейт описывает уже собранный логический граф модулей. Он не знает, был
//! ли код прочитан из информационной базы, XML-выгрузки Конфигуратора или
//! проекта EDT, и не содержит подключений, учётных данных и состояния
//! компилятора.

use std::collections::HashSet;
use std::error::Error;
use std::fmt;
use std::sync::Arc;

/// Неизменяемый снимок всего исходного кода одной конфигурации.
#[derive(Debug, Clone)]
pub struct ConfigurationCodeSnapshot {
    identity: Arc<str>,
    modules: Arc<[ConfigurationModule]>,
}

impl ConfigurationCodeSnapshot {
    /// Проверяет и создаёт снимок одного источника конфигурации.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку при пустой идентичности конфигурации, пустом или
    /// повторном идентификаторе модуля, пустом имени модуля или импорта,
    /// либо при ссылке импорта на отсутствующий модуль.
    pub fn new(
        identity: impl Into<String>,
        modules: Vec<ConfigurationModule>,
    ) -> Result<Self, SnapshotError> {
        let identity = identity.into();
        if identity.is_empty() {
            return Err(SnapshotError::EmptyConfigurationIdentity);
        }

        let mut ids = HashSet::with_capacity(modules.len());
        for module in &modules {
            if module.id.is_empty() {
                return Err(SnapshotError::EmptyModuleIdentity);
            }
            if module.name.is_empty() {
                return Err(SnapshotError::EmptyModuleName {
                    module: module.id.to_string(),
                });
            }
            if !ids.insert(module.id.as_ref()) {
                return Err(SnapshotError::DuplicateModuleIdentity {
                    module: module.id.to_string(),
                });
            }
            if let Some(owner) = &module.owner
                && owner.is_empty()
            {
                return Err(SnapshotError::EmptyOwnerIdentity {
                    module: module.id.to_string(),
                });
            }
        }

        for module in &modules {
            for import in module.imports.iter() {
                if import.alias.is_empty() {
                    return Err(SnapshotError::EmptyImportAlias {
                        module: module.id.to_string(),
                    });
                }
                if !ids.contains(import.module.as_ref()) {
                    return Err(SnapshotError::UnknownImportedModule {
                        module: module.id.to_string(),
                        imported: import.module.to_string(),
                    });
                }
            }
        }

        Ok(Self {
            identity: identity.into(),
            modules: modules.into(),
        })
    }

    /// Возвращает устойчивую идентичность конфигурации источника.
    #[must_use]
    pub fn identity(&self) -> &str {
        &self.identity
    }

    /// Возвращает модули в порядке, выбранном загрузчиком.
    #[must_use]
    pub fn modules(&self) -> &[ConfigurationModule] {
        &self.modules
    }

    /// Находит модуль по его устойчивому идентификатору.
    #[must_use]
    pub fn module(&self, id: &str) -> Option<&ConfigurationModule> {
        self.modules.iter().find(|module| module.id.as_ref() == id)
    }
}

/// Один окончательный исходный модуль конфигурации.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigurationModule {
    id: Arc<str>,
    name: Arc<str>,
    source: Arc<str>,
    kind: ModuleKind,
    execution: ModuleExecution,
    imports: Arc<[ModuleImport]>,
    owner: Option<Arc<str>>,
}

impl ConfigurationModule {
    /// Создаёт описание модуля. Проверка ссылочной целостности выполняется
    /// при создании [`ConfigurationCodeSnapshot`].
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        source: impl Into<String>,
        kind: ModuleKind,
        execution: ModuleExecution,
    ) -> Self {
        Self {
            id: Arc::from(id.into()),
            name: Arc::from(name.into()),
            source: Arc::from(source.into()),
            kind,
            execution,
            imports: Arc::from([]),
            owner: None,
        }
    }

    /// Задаёт импорты модуля целиком.
    #[must_use]
    pub fn with_imports(mut self, imports: Vec<ModuleImport>) -> Self {
        self.imports = imports.into();
        self
    }

    /// Связывает модуль с объектом метаданных конфигурации.
    #[must_use]
    pub fn with_owner(mut self, owner: impl Into<String>) -> Self {
        self.owner = Some(Arc::from(owner.into()));
        self
    }

    /// Возвращает устойчивый идентификатор модуля.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Возвращает BSL-имя модуля.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Возвращает единственный окончательный исходный текст модуля.
    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Возвращает вид модуля.
    #[must_use]
    pub const fn kind(&self) -> ModuleKind {
        self.kind
    }

    /// Возвращает атрибуты исполнения модуля.
    #[must_use]
    pub const fn execution(&self) -> ModuleExecution {
        self.execution
    }

    /// Возвращает импорты модуля.
    #[must_use]
    pub fn imports(&self) -> &[ModuleImport] {
        &self.imports
    }

    /// Возвращает идентификатор объекта-владельца, если он есть.
    #[must_use]
    pub fn owner(&self) -> Option<&str> {
        self.owner.as_deref()
    }
}

/// Логический вид модуля конфигурации.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ModuleKind {
    /// Модуль сеанса.
    Session,
    /// Общий модуль.
    Common,
    /// Модуль менеджера объекта метаданных.
    Manager,
    /// Модуль объекта.
    Object,
    /// Модуль набора записей регистра.
    RecordSet,
}

/// Контексты и свойства исполнения одного модуля.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ModuleExecution {
    server: bool,
    client: bool,
    external_connection: bool,
    global: bool,
    privileged: bool,
    server_call: bool,
}

impl ModuleExecution {
    /// Создаёт пустой набор атрибутов исполнения.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            server: false,
            client: false,
            external_connection: false,
            global: false,
            privileged: false,
            server_call: false,
        }
    }

    /// Задаёт доступность на сервере.
    #[must_use]
    pub const fn with_server(mut self, enabled: bool) -> Self {
        self.server = enabled;
        self
    }

    /// Задаёт доступность в клиентском контексте.
    #[must_use]
    pub const fn with_client(mut self, enabled: bool) -> Self {
        self.client = enabled;
        self
    }

    /// Задаёт доступность во внешнем соединении.
    #[must_use]
    pub const fn with_external_connection(mut self, enabled: bool) -> Self {
        self.external_connection = enabled;
        self
    }

    /// Задаёт глобальность общего модуля.
    #[must_use]
    pub const fn with_global(mut self, enabled: bool) -> Self {
        self.global = enabled;
        self
    }

    /// Задаёт привилегированный режим общего модуля.
    #[must_use]
    pub const fn with_privileged(mut self, enabled: bool) -> Self {
        self.privileged = enabled;
        self
    }

    /// Задаёт возможность серверного вызова из клиентского контекста.
    #[must_use]
    pub const fn with_server_call(mut self, enabled: bool) -> Self {
        self.server_call = enabled;
        self
    }

    /// Доступен ли модуль на сервере.
    #[must_use]
    pub const fn server(self) -> bool {
        self.server
    }

    /// Доступен ли модуль в клиентском контексте.
    #[must_use]
    pub const fn client(self) -> bool {
        self.client
    }

    /// Доступен ли модуль во внешнем соединении.
    #[must_use]
    pub const fn external_connection(self) -> bool {
        self.external_connection
    }

    /// Является ли общий модуль глобальным.
    #[must_use]
    pub const fn global(self) -> bool {
        self.global
    }

    /// Исполняется ли общий модуль в привилегированном режиме.
    #[must_use]
    pub const fn privileged(self) -> bool {
        self.privileged
    }

    /// Разрешён ли серверный вызов из клиентского контекста.
    #[must_use]
    pub const fn server_call(self) -> bool {
        self.server_call
    }
}

/// Именованный импорт другого модуля снимка.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleImport {
    alias: Arc<str>,
    module: Arc<str>,
}

impl ModuleImport {
    /// Создаёт импорт `alias` целевого модуля `module`.
    #[must_use]
    pub fn new(alias: impl Into<String>, module: impl Into<String>) -> Self {
        Self {
            alias: Arc::from(alias.into()),
            module: Arc::from(module.into()),
        }
    }

    /// Возвращает имя импорта в модуле-потребителе.
    #[must_use]
    pub fn alias(&self) -> &str {
        &self.alias
    }

    /// Возвращает устойчивый идентификатор целевого модуля.
    #[must_use]
    pub fn module(&self) -> &str {
        &self.module
    }
}

/// Ошибка целостности нейтрального снимка.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SnapshotError {
    /// Пустая идентичность конфигурации.
    EmptyConfigurationIdentity,
    /// Пустой идентификатор модуля.
    EmptyModuleIdentity,
    /// Пустое имя модуля.
    EmptyModuleName { module: String },
    /// Повторный идентификатор модуля.
    DuplicateModuleIdentity { module: String },
    /// Пустой идентификатор объекта-владельца.
    EmptyOwnerIdentity { module: String },
    /// Пустой псевдоним импорта.
    EmptyImportAlias { module: String },
    /// Импорт ссылается на отсутствующий модуль.
    UnknownImportedModule { module: String, imported: String },
}

impl fmt::Display for SnapshotError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyConfigurationIdentity => {
                formatter.write_str("идентичность конфигурации пуста")
            }
            Self::EmptyModuleIdentity => formatter.write_str("идентификатор модуля пуст"),
            Self::EmptyModuleName { module } => {
                write!(formatter, "имя модуля `{module}` пусто")
            }
            Self::DuplicateModuleIdentity { module } => {
                write!(formatter, "идентификатор модуля `{module}` повторяется")
            }
            Self::EmptyOwnerIdentity { module } => {
                write!(formatter, "идентификатор владельца модуля `{module}` пуст")
            }
            Self::EmptyImportAlias { module } => {
                write!(formatter, "псевдоним импорта модуля `{module}` пуст")
            }
            Self::UnknownImportedModule { module, imported } => write!(
                formatter,
                "модуль `{module}` импортирует отсутствующий модуль `{imported}`"
            ),
        }
    }
}

impl Error for SnapshotError {}

#[cfg(test)]
#[path = "snapshot_tests.rs"]
mod tests;
