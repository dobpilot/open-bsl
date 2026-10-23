//! Выбор файлового источника; разбор нативных форматов остаётся в адаптерах.
use open_bsl::ConfigurationSource;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum FileSourceOptions {
    Xml(PathBuf),
    Edt(PathBuf),
}
impl FileSourceOptions {
    pub(super) fn open(&self) -> Result<Arc<dyn ConfigurationSource>, String> {
        match self {
            Self::Xml(root) => open_bsl::XmlConfigurationSource::open(root)
                .map(|source| Arc::new(source) as Arc<dyn ConfigurationSource>),
            Self::Edt(root) => open_bsl::EdtConfigurationSource::open(root)
                .map(|source| Arc::new(source) as Arc<dyn ConfigurationSource>),
        }
    }
    pub(super) fn extension(
        &self,
        root: &std::path::Path,
    ) -> Result<Arc<dyn ConfigurationSource>, String> {
        match self {
            Self::Xml(_) => Self::Xml(root.into()),
            Self::Edt(_) => Self::Edt(root.into()),
        }
        .open()
    }
}

type FileSelection = (Option<FileSourceOptions>, Vec<(String, PathBuf)>);

pub(super) fn selection(
    xml: Option<String>,
    edt: Option<String>,
    extensions: Vec<String>,
) -> Result<FileSelection, String> {
    if xml.is_some() && edt.is_some() {
        return Err("--config-xml и --config-edt взаимоисключающие".into());
    }
    let source = match (xml, edt) {
        (Some(path), _) if !path.is_empty() => Some(FileSourceOptions::Xml(path.into())),
        (_, Some(path)) if !path.is_empty() => Some(FileSourceOptions::Edt(path.into())),
        (None, None) => None,
        _ => return Err("каталог файловой конфигурации пуст".into()),
    };
    if source.is_none() && !extensions.is_empty() {
        return Err("--config-extension требует --config-xml или --config-edt".into());
    }
    let mut bindings = Vec::<(String, PathBuf)>::new();
    for value in extensions {
        let (id, path) = value
            .split_once('=')
            .ok_or("--config-extension требует UUID=КАТАЛОГ")?;
        if id.is_empty() || path.is_empty() {
            return Err("--config-extension требует непустые UUID и каталог".into());
        }
        let id = open_bsl::uuid::format(
            &open_bsl::uuid::parse(id)
                .map_err(|_| "неверный UUID расширения --config-extension")?,
        );
        if bindings.iter().any(|(known, _)| known == &id) {
            return Err(format!("повтор каталога расширения «{id}»"));
        }
        bindings.push((id, path.into()));
    }
    Ok((source, bindings))
}
