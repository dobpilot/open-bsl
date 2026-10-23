//! Проверка поколения файла и ограниченное чтение до публикации в кэше.
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use std::time::SystemTime;

const MAX_RESOURCE_BYTES: u64 = 256 * 1024 * 1024;

#[derive(Clone, Debug)]
enum ModuleContent {
    Text(Arc<str>),
    Image(Arc<[u8]>),
}

#[derive(Debug)]
pub(super) struct Resource {
    pub path: PathBuf,
    source_path: PathBuf,
    len: u64,
    modified: SystemTime,
    bytes: OnceLock<Result<Arc<[u8]>, String>>,
    module: OnceLock<Result<ModuleContent, String>>,
}
impl Resource {
    pub fn new(root: &Path, path: &Path) -> Result<Arc<Self>, String> {
        let source_path = path.to_path_buf();
        let path = path
            .canonicalize()
            .map_err(|error| format!("{}: {error}", path.display()))?;
        if !path.starts_with(root) {
            return Err(format!("файл вне выбранного каталога: {}", path.display()));
        }
        let info = path.metadata().map_err(|error| error.to_string())?;
        if !info.is_file() || info.len() > MAX_RESOURCE_BYTES {
            return Err(format!(
                "{}: недопустимый размер или вид файла",
                path.display()
            ));
        }
        Ok(Arc::new(Self {
            path,
            source_path,
            len: info.len(),
            modified: info.modified().map_err(|error| error.to_string())?,
            bytes: OnceLock::new(),
            module: OnceLock::new(),
        }))
    }
    fn check(&self) -> Result<(), String> {
        let info = self
            .path
            .metadata()
            .map_err(|error| format!("{}: {error}", self.path.display()))?;
        if self
            .source_path
            .canonicalize()
            .map_err(|error| error.to_string())?
            != self.path
            || !info.is_file()
            || info.len() != self.len
            || info.modified().map_err(|error| error.to_string())? != self.modified
        {
            return Err(format!(
                "источник изменён: {}; создайте новую фабрику",
                self.path.display()
            ));
        }
        Ok(())
    }
    pub fn read_uncached(&self) -> Result<Arc<[u8]>, String> {
        self.check()?;
        let file = std::fs::File::open(&self.path)
            .map_err(|error| format!("{}: {error}", self.path.display()))?;
        let mut bytes = Vec::new();
        file.take(MAX_RESOURCE_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|error| error.to_string())?;
        if bytes.len() as u64 > MAX_RESOURCE_BYTES {
            return Err(format!("{}: превышен предел ресурса", self.path.display()));
        }
        self.check()?;
        Ok(bytes.into())
    }
    fn module(&self) -> Result<&ModuleContent, String> {
        self.module
            .get_or_init(|| {
                let bytes = self.read_uncached()?;
                // EDT сохраняет защищённый контейнер под именем `Module.bsl`.
                // Нативная пара с XML подтверждена `native-edt-binary-module.json`.
                if bytes.starts_with(&[0xff, 0xff, 0xff, 0x7f]) {
                    Ok(ModuleContent::Image(bytes))
                } else {
                    decode(&bytes).map(ModuleContent::Text)
                }
            })
            .as_ref()
            .map_err(Clone::clone)
    }
    pub fn module_image(&self) -> Result<Option<Arc<[u8]>>, String> {
        self.module().map(|content| match content {
            ModuleContent::Image(bytes) => Some(Arc::clone(bytes)),
            ModuleContent::Text(_) => None,
        })
    }
    pub fn module_text(&self) -> Result<Arc<str>, String> {
        match self.module()? {
            ModuleContent::Text(text) => Ok(Arc::clone(text)),
            ModuleContent::Image(_) => Err("нет BSL-текста: нативный образ модуля".into()),
        }
    }
    pub fn read(&self) -> Result<Arc<[u8]>, String> {
        self.bytes.get_or_init(|| self.read_uncached()).clone()
    }
    pub fn index_text(&self) -> Result<Arc<str>, String> {
        decode(&self.read_uncached()?)
    }
}

pub(super) use bsl_config::decode_configuration_text as decode;
