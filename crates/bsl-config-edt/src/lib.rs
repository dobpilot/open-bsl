//! Читатель нативного проекта EDT поверх общего файлового XML-движка.

use bsl_config::{ConfigurationArtifacts, ConfigurationIndex, ConfigurationSource};
use bsl_config_xml::{ConfigurationFileFormat, XmlConfigurationSource};
use std::path::Path;
use std::sync::Arc;

/// Выбранный EDT-проект с ленивым чтением модулей и ресурсов.
pub struct EdtConfigurationSource(XmlConfigurationSource);
impl EdtConfigurationSource {
    /// Принимает корень проекта или его каталог `src`.
    ///
    /// # Errors
    /// Возвращает ошибку нативного описания, каталога или идентичности объекта.
    pub fn open(root: impl AsRef<Path>) -> Result<Self, String> {
        XmlConfigurationSource::open_format(root, ConfigurationFileFormat::Edt).map(Self)
    }
}
impl ConfigurationSource for EdtConfigurationSource {
    fn index(&self) -> &ConfigurationIndex {
        self.0.index()
    }
    fn module_image(&self, id: &str) -> Result<Option<Arc<[u8]>>, String> {
        self.0.module_image(id)
    }
    fn module_source(&self, id: &str) -> Result<Arc<str>, String> {
        self.0.module_source(id)
    }
    fn artifacts(&self) -> Option<&dyn ConfigurationArtifacts> {
        self.0.artifacts()
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

#[cfg(test)]
#[path = "native_import_tests.rs"]
mod native_import_tests;
