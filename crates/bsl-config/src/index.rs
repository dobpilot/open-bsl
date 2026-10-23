//! Объявления модулей отдельно от загружаемых по требованию исходников.

use std::sync::Arc;

use crate::{
    ConfigurationCodeSnapshot, ConfigurationModule, ModuleExecution, ModuleKind, SnapshotError,
};

/// Описание модуля без исходника и без исполняемого тела.
#[derive(Debug, Clone)]
pub struct ConfigurationModuleDeclaration(ConfigurationModule);

impl ConfigurationModuleDeclaration {
    /// Создаёт объявление; целостность проверяет `ConfigurationIndex::new`.
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        kind: ModuleKind,
        execution: ModuleExecution,
    ) -> Self {
        Self(ConfigurationModule::new(id, name, "", kind, execution))
    }

    /// Извлекает объявление, не сохраняя исходник модуля.
    #[must_use]
    pub fn from_module(module: &ConfigurationModule) -> Self {
        let mut declaration = module.clone();
        declaration.source = Arc::from("");
        Self(declaration)
    }

    /// Задаёт объявленные импорты.
    #[must_use]
    pub fn with_imports(mut self, imports: Vec<crate::ModuleImport>) -> Self {
        self.0 = self.0.with_imports(imports);
        self
    }

    /// Задаёт объект-владелец.
    #[must_use]
    pub fn with_owner(mut self, owner: impl Into<String>) -> Self {
        self.0 = self.0.with_owner(owner);
        self
    }

    /// Задаёт происхождение текста из расширения.
    #[must_use]
    pub fn with_extension_source(mut self, extension: bool) -> Self {
        self.0 = self.0.with_extension_source(extension);
        self
    }

    /// Задаёт политику собственного объекта расширения.
    #[must_use]
    pub fn with_own_extension_source(mut self, safe_mode: bool) -> Self {
        self.0 = self.0.with_own_extension_source(safe_mode);
        self
    }

    /// Возвращает устойчивый идентификатор модуля.
    pub fn id(&self) -> &str {
        self.0.id()
    }
    /// Возвращает имя модуля.
    pub fn name(&self) -> &str {
        self.0.name()
    }
    /// Возвращает вид модуля.
    pub fn kind(&self) -> ModuleKind {
        self.0.kind()
    }
    /// Возвращает атрибуты исполнения.
    pub fn execution(&self) -> ModuleExecution {
        self.0.execution()
    }
    /// Возвращает объявленные импорты.
    pub fn imports(&self) -> &[crate::ModuleImport] {
        self.0.imports()
    }
    /// Возвращает объект-владелец.
    pub fn owner(&self) -> Option<&str> {
        self.0.owner()
    }
    /// Возвращает происхождение текста из расширения.
    pub fn source_is_extension(&self) -> bool {
        self.0.source_is_extension()
    }
    /// Возвращает политику собственного объекта расширения.
    pub fn own_extension_safe_mode(&self) -> Option<bool> {
        self.0.own_extension_safe_mode()
    }
}

/// Неизменяемый индекс одного выбранного источника конфигурации.
#[derive(Debug, Clone)]
pub struct ConfigurationIndex {
    identity: Arc<str>,
    modules: Arc<[ConfigurationModuleDeclaration]>,
}

impl ConfigurationIndex {
    /// Проверяет идентичности и ссылки объявлений без загрузки исходников.
    ///
    /// # Errors
    /// Возвращает те же ошибки целостности, что `ConfigurationCodeSnapshot::new`.
    pub fn new(
        identity: impl Into<String>,
        modules: Vec<ConfigurationModuleDeclaration>,
    ) -> Result<Self, SnapshotError> {
        let identity = identity.into();
        ConfigurationCodeSnapshot::new(
            &identity,
            modules.iter().map(|module| module.0.clone()).collect(),
        )?;
        Ok(Self {
            identity: identity.into(),
            modules: modules.into(),
        })
    }

    /// Возвращает идентичность выбранной конфигурации.
    pub fn identity(&self) -> &str {
        &self.identity
    }
    /// Возвращает объявления в устойчивом порядке загрузчика.
    pub fn modules(&self) -> &[ConfigurationModuleDeclaration] {
        &self.modules
    }
    /// Находит объявление по идентификатору без загрузки текста.
    pub fn module(&self, id: &str) -> Option<&ConfigurationModuleDeclaration> {
        self.modules.iter().find(|module| module.id() == id)
    }
}

/// Текст для обнаружения объявлений без обязательного восстановления тел.
#[derive(Debug, Clone)]
pub enum ModuleDeclarationSource {
    /// Полный исходник: его можно сохранить для последующей компиляции.
    Complete(Arc<str>),
    /// Только объявления: этот текст нельзя исполнять вместо исходника.
    Declarations(Arc<str>),
}

/// Выбранный источник исходников: БД, файловая выгрузка либо готовый снимок.
///
/// Реализация возвращает единственный эффективный текст модуля. Запрос одного
/// текста не должен загружать остальные тела. Изменяемые ресурсы сеанса BSL
/// этому контракту не принадлежат.
pub trait ConfigurationSource: Send + Sync {
    /// Возвращает индекс без загрузки исходников модулей.
    fn index(&self) -> &ConfigurationIndex;

    /// Возвращает описания и ресурсы, если источник предоставляет эту поверхность.
    fn artifacts(&self) -> Option<&dyn crate::ConfigurationArtifacts> {
        None
    }

    /// Возвращает нативный образ модуля, если вместо BSL поставлен образ.
    /// Интерпретация образа и политика защищённых экспортов принадлежат хосту.
    ///
    /// # Errors
    /// Возвращает ошибку приобретения или изменения файлового ресурса.
    fn module_image(&self, _id: &str) -> Result<Option<Arc<[u8]>>, String> {
        Ok(None)
    }

    /// Загружает эффективный текст одного объявленного модуля.
    ///
    /// # Errors
    /// Возвращает описание ошибки источника или отсутствующего модуля.
    fn module_source(&self, id: &str) -> Result<Arc<str>, String>;

    /// Загружает объявления, по возможности не восстанавливая тела методов.
    /// Обычный источник возвращает полный текст для повторного использования.
    ///
    /// # Errors
    /// Возвращает ошибку источника или неподдержанного формата объявлений.
    fn module_declarations(&self, id: &str) -> Result<ModuleDeclarationSource, String> {
        self.module_source(id)
            .map(ModuleDeclarationSource::Complete)
    }
}

/// Адаптер существующего полного снимка к адресному контракту источника.
pub struct SnapshotSource {
    index: ConfigurationIndex,
    snapshot: ConfigurationCodeSnapshot,
}

impl SnapshotSource {
    /// Принимает уже проверенный снимок, не копируя байты исходников.
    pub fn new(snapshot: ConfigurationCodeSnapshot) -> Self {
        Self {
            index: ConfigurationIndex {
                identity: snapshot.identity.clone(),
                modules: snapshot
                    .modules()
                    .iter()
                    .map(ConfigurationModuleDeclaration::from_module)
                    .collect(),
            },
            snapshot,
        }
    }
}

impl ConfigurationSource for SnapshotSource {
    fn index(&self) -> &ConfigurationIndex {
        &self.index
    }

    fn module_source(&self, id: &str) -> Result<Arc<str>, String> {
        self.snapshot
            .module(id)
            .map(|module| module.source.clone())
            .ok_or_else(|| format!("модуль «{id}» отсутствует в источнике"))
    }
}
