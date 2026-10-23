//! Описания и ресурсы одного выбранного источника конфигурации.

use std::sync::Arc;

use crate::{ConfigurationIndex, ConfigurationSource, ModuleDeclarationSource};

/// Нейтральное дерево свойств описания. Имена свойств не содержат XML-префиксов.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigurationProperty {
    /// Имя свойства или элемента коллекции.
    pub name: Arc<str>,
    /// Текстовое значение; пустое значение отличается от отсутствующего узла.
    pub value: Arc<str>,
    /// Именованные параметры описания.
    pub attributes: Arc<[(Arc<str>, Arc<str>)]>,
    /// Вложенные свойства в порядке источника.
    pub children: Arc<[ConfigurationProperty]>,
}

impl ConfigurationProperty {
    /// Находит непосредственное вложенное свойство по нейтральному имени.
    #[must_use]
    pub fn child(&self, name: &str) -> Option<&Self> {
        self.children
            .iter()
            .find(|child| child.name.as_ref() == name)
    }

    /// Возвращает именованный параметр, сохраняя различие пустого и отсутствующего.
    #[must_use]
    pub fn attribute(&self, name: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find(|(key, _)| key.as_ref() == name)
            .map(|(_, value)| value.as_ref())
    }
}

/// Объявление объекта или вложенного ресурса без загрузки его содержимого.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigurationDescription {
    /// Нативная идентичность объекта или владельца с локальным идентификатором.
    pub id: Arc<str>,
    /// Идентичность владельца; у корня конфигурации её нет.
    pub owner: Option<Arc<str>>,
    /// Вид описания, например `Catalog`, `Form` или `Template`.
    pub kind: Arc<str>,
    /// Исходное имя для отображения и существующего поиска BSL.
    pub name: Arc<str>,
}

/// Назначение файлового ресурса конфигурации.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigurationArtifactKind {
    /// Описание формы; её модуль загружается через `ConfigurationSource`.
    Form,
    /// Общий или подчинённый макет.
    Template,
}

/// Полное неизменяемое содержимое ресурса выбранного источника.
#[derive(Debug, Clone)]
pub struct ConfigurationArtifact {
    /// Проверенное объявление ресурса и его владельца.
    pub description: ConfigurationDescription,
    /// Назначение ресурса.
    pub kind: ConfigurationArtifactKind,
    /// Объявленный вид содержимого, например `TextDocument`.
    pub representation: Arc<str>,
    /// Байты источника без подмены пустым значением при ошибке чтения.
    pub bytes: Arc<[u8]>,
}

/// Дополнительная поверхность выбранного источника для метаданных и ресурсов.
/// Соединения, права пользователя и изменяемое состояние сеанса сюда не входят.
pub trait ConfigurationArtifacts: Send + Sync {
    /// Нативный UUID корневого объекта; это не внутреннее имя хранилища БД.
    fn root_identity(&self) -> &str;
    /// Полный индекс объявленных объектов без приобретения тяжёлых ресурсов.
    fn descriptions(&self) -> &[ConfigurationDescription];
    /// Приобретает описание; `None` означает отсутствие объявления.
    ///
    /// # Errors
    /// Возвращает ошибку чтения, изменения источника или неподдержанного описания.
    fn description(&self, id: &str) -> Result<Option<Arc<ConfigurationProperty>>, String>;
    /// Приобретает форму или макет; отсутствие объявления отличается от ошибки.
    ///
    /// # Errors
    /// Возвращает ошибку чтения, изменения источника или неподдержанного ресурса.
    fn artifact(&self, id: &str) -> Result<Option<Arc<ConfigurationArtifact>>, String>;
}

/// Источник с проверенной связью нативного корня и пространства имён фабрики.
pub struct BoundConfigurationSource {
    source: Arc<dyn ConfigurationSource>,
    index: ConfigurationIndex,
}

impl BoundConfigurationSource {
    /// Проверяет нативный корень перед назначением пространства имён БД.
    ///
    /// # Errors
    /// Возвращает ошибку несовпадения корня, отсутствия описаний или индекса.
    pub fn new(
        source: Arc<dyn ConfigurationSource>,
        native_root: &str,
        namespace: &str,
    ) -> Result<Self, String> {
        let artifacts = source
            .artifacts()
            .ok_or("источник не предоставляет описания конфигурации")?;
        if !artifacts.root_identity().eq_ignore_ascii_case(native_root) {
            return Err(format!(
                "UUID файловой конфигурации «{}» не совпадает с UUID базы «{native_root}»",
                artifacts.root_identity()
            ));
        }
        let index = ConfigurationIndex::new(namespace, source.index().modules().to_vec())
            .map_err(|error| error.to_string())?;
        Ok(Self { source, index })
    }
}

impl ConfigurationSource for BoundConfigurationSource {
    fn index(&self) -> &ConfigurationIndex {
        &self.index
    }
    fn module_image(&self, id: &str) -> Result<Option<Arc<[u8]>>, String> {
        self.source.module_image(id)
    }
    fn module_source(&self, id: &str) -> Result<Arc<str>, String> {
        self.source.module_source(id)
    }
    fn module_declarations(&self, id: &str) -> Result<ModuleDeclarationSource, String> {
        self.source.module_declarations(id)
    }
    fn artifacts(&self) -> Option<&dyn ConfigurationArtifacts> {
        self.source.artifacts()
    }
}

/// Декодирует текст ресурса UTF-8 либо UTF-16 с маркером порядка байтов.
///
/// # Errors
/// Возвращает ошибку некорректной последовательности или неподдержанной кодировки.
pub fn decode_configuration_text(bytes: &[u8]) -> Result<Arc<str>, String> {
    if bytes.starts_with(&[0xff, 0xfe]) || bytes.starts_with(&[0xfe, 0xff]) {
        if !bytes.len().is_multiple_of(2) {
            return Err("нечётная длина UTF-16".into());
        }
        let little = bytes[0] == 0xff;
        let units: Vec<_> = bytes[2..]
            .chunks_exact(2)
            .map(|pair| {
                if little {
                    u16::from_le_bytes([pair[0], pair[1]])
                } else {
                    u16::from_be_bytes([pair[0], pair[1]])
                }
            })
            .collect();
        return String::from_utf16(&units)
            .map(Arc::from)
            .map_err(|error| error.to_string());
    }
    std::str::from_utf8(bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(bytes))
        .map(Arc::from)
        .map_err(|error| format!("источник не является UTF-8/UTF-16: {error}"))
}

impl crate::ModuleExecution {
    /// Приобретает флаги общего модуля из нейтрального описания источника.
    /// Пропущенные булевы свойства имеют нативное значение `false`.
    ///
    /// # Errors
    /// Возвращает ошибку, если булево свойство содержит другое значение.
    pub fn from_properties(properties: &ConfigurationProperty) -> Result<Self, String> {
        let flag = |name: &str| match properties.child(name).map(|p| p.value.trim()) {
            None | Some("false") => Ok(false),
            Some("true") => Ok(true),
            Some(other) => Err(format!("{name}: недопустимый булев флаг «{other}»")),
        };
        Ok(Self::new()
            .with_server(flag("server")?)
            .with_client(flag("clientManagedApplication")? || flag("clientOrdinaryApplication")?)
            .with_external_connection(flag("externalConnection")?)
            .with_global(flag("global")?)
            .with_privileged(flag("privileged")?)
            .with_server_call(flag("serverCall")?))
    }
}
