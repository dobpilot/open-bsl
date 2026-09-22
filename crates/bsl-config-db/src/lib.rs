//! Загрузчик исходного кода конфигурации из информационной базы.
//!
//! SQL, транзакция, сборка частей и лимиты принадлежат `open-sdbl-db`.
//! Этот крейт принимает готовое приобретение и строит нейтральный снимок.

#![warn(missing_docs)]

use std::{collections::HashMap, error::Error, fmt, sync::Arc};

use bsl_config::{
    ConfigurationCodeSnapshot, ConfigurationModule, ModuleExecution, ModuleKind, SnapshotError,
};
use open_sdbl::metadata::{Value, inflate_raw_deflate, parse_config_descriptor, parse_serialized};
use open_sdbl_db::{AcquiredConfiguration, ConfigResource};

const COMMON: &str = "0fe48980-252d-11d6-a3c7-0050bae0a776";
const CONFIGURATION: &str = "9cd510cd-abfc-11d4-9434-004095e12fc7";

/// Строит снимок исходников из одного результата приобретения.
///
/// # Errors
///
/// Возвращает ошибку при активном расширении, неизвестной его активности,
/// повреждённом или отсутствующем ресурсе, неподдерживаемой кодировке и
/// некорректном итоговом графе.
pub fn load(acquired: &AcquiredConfiguration) -> Result<ConfigurationCodeSnapshot, LoadError> {
    for extension in &acquired.extensions {
        if extension.active.is_none() {
            return Err(LoadError::UnsupportedExtension {
                name: extension.name.clone(),
                activity_known: false,
            });
        }
    }
    let mut resources = acquired.config_resources.clone();
    let mut active = acquired
        .extensions
        .iter()
        .filter(|extension| extension.active == Some(true))
        .collect::<Vec<_>>();
    active.sort_by_key(|extension| extension.order);
    for extension in active {
        apply_source_overlay(&mut resources, &extension.name, &extension.resources)?;
    }
    load_base(&resources)
}

fn apply_source_overlay(
    base: &mut [ConfigResource],
    extension: &str,
    resources: &[ConfigResource],
) -> Result<(), LoadError> {
    for resource in resources {
        let Some((_, suffix)) = resource.file_name.rsplit_once('.') else {
            return Err(LoadError::UnsupportedExtension {
                name: extension.into(),
                activity_known: true,
            });
        };
        if suffix.parse::<u16>().is_err() {
            return Err(LoadError::UnsupportedExtension {
                name: extension.into(),
                activity_known: true,
            });
        }
        let Some(target) = base
            .iter_mut()
            .find(|target| target.file_name.eq_ignore_ascii_case(&resource.file_name))
        else {
            return Err(LoadError::UnsupportedExtension {
                name: extension.into(),
                activity_known: true,
            });
        };
        target.compressed = Arc::clone(&resource.compressed);
    }
    Ok(())
}

fn load_base(resources: &[ConfigResource]) -> Result<ConfigurationCodeSnapshot, LoadError> {
    let resources = Resources::new(resources)?;
    let root = parse_serialized(&resources.decoded("root")?).map_err(|e| malformed("root", e))?;
    let root_resource = first_guid(&root)
        .ok_or_else(|| malformed("root", "нет GUID конфигурации"))?
        .to_ascii_lowercase();
    let root_value = parse_serialized(&resources.decoded(&root_resource)?)
        .map_err(|e| malformed(&root_resource, e))?;
    let identity = configuration_identity(&root_value)
        .ok_or_else(|| malformed(&root_resource, "нет GUID объекта конфигурации"))?
        .to_ascii_lowercase();
    let collections = collections(&root_value)
        .ok_or_else(|| malformed(&root_resource, "нет каталога объектов"))?;

    let mut modules = Vec::new();
    push_module(
        &mut modules,
        &resources,
        &identity,
        "МодульСеанса".into(),
        Slot {
            suffix: "7",
            kind: ModuleKind::Session,
            prefix: "",
            label: "",
        },
        None,
    )?;
    for include in collections {
        let count = scalar(include.get(2).expect("проверено при разборе"))
            .and_then(|value| value.parse::<usize>().ok())
            .expect("проверено при разборе");
        for collection in include.iter().skip(3).take(count) {
            let Some((class, ids)) = collection_ids(collection) else {
                continue;
            };
            for id in ids {
                let descriptor = parse_config_descriptor(id, resources.compressed(id)?)
                    .map_err(|e| malformed(id, e))?
                    .ok_or_else(|| malformed(id, "нет описателя объекта"))?;
                if class.eq_ignore_ascii_case(COMMON) {
                    let source = resources.module_source(id, "0")?.unwrap_or_default();
                    let execution = common_execution(&resources.decoded(id)?)?;
                    modules.push(ConfigurationModule::new(
                        format!("{id}.0"),
                        descriptor.name,
                        source,
                        ModuleKind::Common,
                        execution,
                    ));
                    continue;
                }
                for slot in module_slots(class) {
                    push_module(
                        &mut modules,
                        &resources,
                        id,
                        format!("{}.{}.{}", slot.prefix, descriptor.name, slot.label),
                        slot,
                        Some(id),
                    )?;
                }
            }
        }
    }
    ConfigurationCodeSnapshot::new(identity, modules).map_err(LoadError::Snapshot)
}

fn push_module(
    out: &mut Vec<ConfigurationModule>,
    resources: &Resources<'_>,
    owner: &str,
    name: String,
    slot: Slot,
    metadata_owner: Option<&str>,
) -> Result<(), LoadError> {
    let Some(source) = resources.module_source(owner, slot.suffix)? else {
        return Ok(());
    };
    let module = ConfigurationModule::new(
        format!("{owner}.{}", slot.suffix),
        name,
        source,
        slot.kind,
        ModuleExecution::default().with_server(true),
    );
    out.push(match metadata_owner {
        Some(owner) => module.with_owner(owner),
        None => module,
    });
    Ok(())
}

#[derive(Clone, Copy)]
struct Slot {
    suffix: &'static str,
    kind: ModuleKind,
    prefix: &'static str,
    label: &'static str,
}

const fn manager(suffix: &'static str, prefix: &'static str) -> Slot {
    Slot {
        suffix,
        kind: ModuleKind::Manager,
        prefix,
        label: "МодульМенеджера",
    }
}
const fn object(suffix: &'static str, prefix: &'static str) -> Slot {
    Slot {
        suffix,
        kind: ModuleKind::Object,
        prefix,
        label: "МодульОбъекта",
    }
}
const fn records(suffix: &'static str, prefix: &'static str) -> Slot {
    Slot {
        suffix,
        kind: ModuleKind::RecordSet,
        prefix,
        label: "МодульНабораЗаписей",
    }
}

fn module_slots(class: &str) -> Vec<Slot> {
    match class.to_ascii_lowercase().as_str() {
        "cf4abea6-37b2-11d4-940f-008048da11f9" => {
            vec![manager("3", "Справочник"), object("0", "Справочник")]
        }
        "061d872a-5787-460e-95ac-ed74ea3a3e84" => {
            vec![manager("2", "Документ"), object("0", "Документ")]
        }
        "0195e80c-b157-11d4-9435-004095e12fc7" => {
            vec![manager("1", "Константа"), object("0", "Константа")]
        }
        "13134201-f60b-11d5-a3c7-0050bae0a776" => vec![
            manager("2", "РегистрСведений"),
            records("1", "РегистрСведений"),
        ],
        "b64d9a40-1642-11d6-a3c7-0050bae0a776" => vec![
            manager("2", "РегистрНакопления"),
            records("1", "РегистрНакопления"),
        ],
        "2deed9b8-0056-4ffe-a473-c20a6c32a0bc" => vec![
            manager("7", "РегистрБухгалтерии"),
            records("6", "РегистрБухгалтерии"),
        ],
        "f2de87a8-64e5-45eb-a22d-b3aedab050e7" => vec![
            manager("2", "РегистрРасчета"),
            records("1", "РегистрРасчета"),
        ],
        "238e7e88-3c5f-48b2-8a3b-81ebbecb20ed" => {
            vec![manager("15", "ПланСчетов"), object("14", "ПланСчетов")]
        }
        "82a1b659-b220-4d94-a9bd-14d757b95a48" => vec![
            manager("16", "ПланВидовХарактеристик"),
            object("15", "ПланВидовХарактеристик"),
        ],
        "30b100d6-b29f-47ac-aec7-cb8ca8a54767" => vec![
            manager("3", "ПланВидовРасчета"),
            object("0", "ПланВидовРасчета"),
        ],
        "857c4a91-e5f4-4fac-86ec-787626f1e108" => {
            vec![manager("3", "ПланОбмена"), object("2", "ПланОбмена")]
        }
        "4612bd75-71b7-4a5c-8cc5-2b0b65f9fa0d" => vec![manager("1", "ЖурналДокументов")],
        "631b75a0-29e2-11d6-a3c7-0050bae0a776" => vec![manager("2", "Отчет"), object("0", "Отчет")],
        "bf845118-327b-4682-b5c6-285d2a0eb296" => {
            vec![manager("2", "Обработка"), object("0", "Обработка")]
        }
        "fcd3404e-1523-48ce-9bc0-ecdb822684a1" => {
            vec![manager("8", "БизнесПроцесс"), object("6", "БизнесПроцесс")]
        }
        "3e63355c-1378-4953-be9b-1deb5fb6bec5" => {
            vec![manager("7", "Задача"), object("6", "Задача")]
        }
        "46b4cd97-fd13-4eaa-aba2-3bddd7699218" => vec![manager("8", "ХранилищеНастроек")],
        _ => vec![],
    }
}

fn common_execution(bytes: &[u8]) -> Result<ModuleExecution, LoadError> {
    let value = parse_serialized(bytes).map_err(|e| malformed("общий модуль", e))?;
    let flags = list_path(&value, &[1])
        .ok_or_else(|| malformed("общий модуль", "нет свойств исполнения"))?;
    let flag = |index| {
        flags
            .get(index)
            .and_then(scalar)
            .and_then(bool_atom)
            .unwrap_or(false)
    };
    Ok(ModuleExecution::default()
        .with_client(flag(2) || flag(7))
        .with_server(flag(3))
        .with_external_connection(flag(4))
        .with_privileged(flag(5))
        .with_global(flag(6))
        .with_server_call(flag(9)))
}

fn collections(value: &Value) -> Option<Vec<&[Value]>> {
    let root = list(value)?;
    let count = scalar(root.get(2)?)?.parse::<usize>().ok()?;
    let mut result = Vec::new();
    for group in root.iter().skip(3).take(count) {
        let group = list(group)?;
        let versioned = list(group.get(1)?)?;
        let include = if scalar(versioned.first()?) == Some("6") {
            list(versioned.get(1)?)?
        } else {
            versioned
        };
        include.get(2).and_then(scalar)?.parse::<usize>().ok()?;
        result.push(include);
    }
    (!result.is_empty()).then_some(result)
}

fn configuration_identity(value: &Value) -> Option<&str> {
    let root = list(value)?;
    let count = scalar(root.get(2)?)?.parse::<usize>().ok()?;
    let group = root.iter().skip(3).take(count).find(|group| {
        list(group)
            .and_then(|group| group.first())
            .and_then(scalar)
            .is_some_and(|class| class.eq_ignore_ascii_case(CONFIGURATION))
    })?;
    let self_reference = list_path(group, &[1, 1, 1, 1, 1])?;
    let identity = scalar(self_reference.get(2)?)?;
    guid(identity).then_some(identity)
}

fn collection_ids(value: &Value) -> Option<(&str, Vec<&str>)> {
    let values = list(value)?;
    let class = scalar(values.first()?)?;
    let count = scalar(values.get(1)?)?.parse::<usize>().ok()?;
    Some((
        class,
        values
            .iter()
            .skip(2)
            .take(count)
            .filter_map(scalar)
            .filter(|s| guid(s))
            .collect(),
    ))
}

fn list(value: &Value) -> Option<&[Value]> {
    if let Value::List(values) = value {
        Some(values)
    } else {
        None
    }
}
fn scalar(value: &Value) -> Option<&str> {
    match value {
        Value::Atom(v) | Value::String(v) => Some(v),
        _ => None,
    }
}
fn list_path<'a>(value: &'a Value, path: &[usize]) -> Option<&'a [Value]> {
    let mut value = value;
    for index in path {
        value = list(value)?.get(*index)?;
    }
    list(value)
}
fn first_guid(value: &Value) -> Option<&str> {
    match value {
        Value::Atom(v) | Value::String(v) if guid(v) => Some(v),
        Value::List(values) => values.iter().find_map(first_guid),
        _ => None,
    }
}
fn guid(value: &str) -> bool {
    value.len() == 36
        && value.bytes().enumerate().all(|(i, b)| {
            if matches!(i, 8 | 13 | 18 | 23) {
                b == b'-'
            } else {
                b.is_ascii_hexdigit()
            }
        })
}
fn bool_atom(value: &str) -> Option<bool> {
    match value {
        "0" => Some(false),
        "1" => Some(true),
        _ => None,
    }
}

struct Resources<'a>(HashMap<String, &'a ConfigResource>);
impl<'a> Resources<'a> {
    fn new(resources: &'a [ConfigResource]) -> Result<Self, LoadError> {
        let mut result = HashMap::with_capacity(resources.len());
        for resource in resources {
            let name = resource.file_name.to_ascii_lowercase();
            if result.insert(name.clone(), resource).is_some() {
                return Err(LoadError::DuplicateResource(name));
            }
        }
        Ok(Self(result))
    }
    fn compressed(&self, name: &str) -> Result<&[u8], LoadError> {
        self.0
            .get(&name.to_ascii_lowercase())
            .map(|r| r.compressed.as_ref())
            .ok_or_else(|| LoadError::MissingResource(name.into()))
    }
    fn decoded(&self, name: &str) -> Result<Vec<u8>, LoadError> {
        inflate_raw_deflate(self.compressed(name)?).map_err(|e| malformed(name, e))
    }
    fn module_source(&self, owner: &str, suffix: &str) -> Result<Option<String>, LoadError> {
        let name = format!("{owner}.{suffix}").to_ascii_lowercase();
        let Some(resource) = self.0.get(&name) else {
            return Ok(None);
        };
        let decoded =
            inflate_raw_deflate(resource.compressed.as_ref()).map_err(|e| malformed(&name, e))?;
        let bytes = if decoded.starts_with(&[0xff, 0xff, 0xff, 0x7f]) {
            let Some(bytes) = container_text(&decoded)? else {
                return Ok(None);
            };
            bytes
        } else {
            decoded
        };
        decode_text(&name, &bytes).map(Some)
    }
}

fn decode_text(name: &str, bytes: &[u8]) -> Result<String, LoadError> {
    let bytes = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(bytes);
    if let Some(bytes) = bytes.strip_prefix(&[0xff, 0xfe]) {
        if bytes.len() % 2 != 0 {
            return Err(malformed(name, "нечётный UTF-16LE"));
        }
        let units = bytes
            .chunks_exact(2)
            .map(|p| u16::from_le_bytes([p[0], p[1]]))
            .collect::<Vec<_>>();
        return String::from_utf16(&units).map_err(|_| LoadError::UnsupportedEncoding(name.into()));
    }
    std::str::from_utf8(bytes)
        .map(str::to_owned)
        .map_err(|_| LoadError::UnsupportedEncoding(name.into()))
}

fn container_text(container: &[u8]) -> Result<Option<Vec<u8>>, LoadError> {
    if container.len() < 16 {
        return Err(LoadError::MalformedContainer("короткий заголовок".into()));
    }
    let toc = document(container, 16, 0x200)?;
    let marker = i32::MAX.to_le_bytes();
    for entry in toc.split(|_| false) {
        let mut start = 0;
        while let Some(relative) = entry[start..].windows(4).position(|part| part == marker) {
            let part = &entry[start..start + relative];
            if part.len() == 8 {
                let description = u32::from_le_bytes(part[..4].try_into().unwrap()) as usize;
                let data = u32::from_le_bytes(part[4..].try_into().unwrap()) as usize;
                let description = document(container, description, 0)?;
                if description.len() < 20 {
                    return Err(LoadError::MalformedContainer(
                        "короткое описание файла".into(),
                    ));
                }
                let name = utf16_name(&description[20..])?;
                if name == "text" {
                    return document(container, data, 0).map(Some);
                }
            }
            start += relative + 4;
            if start >= entry.len() {
                break;
            }
        }
    }
    Ok(None)
}

fn utf16_name(bytes: &[u8]) -> Result<String, LoadError> {
    let end = bytes
        .chunks_exact(2)
        .position(|p| p == [0, 0])
        .unwrap_or(bytes.len() / 2);
    let units = bytes[..end * 2]
        .chunks_exact(2)
        .map(|p| u16::from_le_bytes([p[0], p[1]]))
        .collect::<Vec<_>>();
    String::from_utf16(&units)
        .map_err(|_| LoadError::MalformedContainer("имя файла не UTF-16LE".into()))
}

fn document(container: &[u8], mut offset: usize, minimum: usize) -> Result<Vec<u8>, LoadError> {
    let mut out = Vec::new();
    let mut size = None;
    let mut blocks = 0;
    loop {
        blocks += 1;
        if blocks > 1_000_000
            || offset
                .checked_add(31)
                .is_none_or(|end| end > container.len())
        {
            return Err(LoadError::MalformedContainer(
                "цепочка блоков повреждена".into(),
            ));
        }
        let header = &container[offset..offset + 31];
        let hex = |range: std::ops::Range<usize>| {
            std::str::from_utf8(&header[range])
                .ok()
                .and_then(|s| usize::from_str_radix(s, 16).ok())
                .ok_or_else(|| LoadError::MalformedContainer("неверный заголовок блока".into()))
        };
        let total = hex(2..10)?;
        let block = hex(11..19)?.max(minimum);
        let next = hex(20..28)?;
        let total = *size.get_or_insert(total);
        let take = block.min(total.saturating_sub(out.len()));
        let start = offset + 31;
        let end = start
            .checked_add(take)
            .filter(|end| *end <= container.len())
            .ok_or_else(|| LoadError::MalformedContainer("блок выходит за контейнер".into()))?;
        out.extend_from_slice(&container[start..end]);
        if out.len() >= total {
            out.truncate(total);
            return Ok(out);
        }
        if next == i32::MAX as usize {
            return Err(LoadError::MalformedContainer(
                "цепочка блоков оборвана".into(),
            ));
        }
        offset = next;
    }
}

fn malformed(name: &str, reason: impl fmt::Display) -> LoadError {
    LoadError::MalformedResource(name.into(), reason.to_string())
}

/// Ошибка загрузки исходников конфигурации.
#[derive(Debug)]
pub enum LoadError {
    /// Обязательный ресурс отсутствует.
    MissingResource(String),
    /// Имя ресурса повторяется без учёта регистра.
    DuplicateResource(String),
    /// Ресурс не соответствует ожидаемому формату.
    MalformedResource(String, String),
    /// Контейнер исходника повреждён.
    MalformedContainer(String),
    /// Кодировка исходника не поддерживается.
    UnsupportedEncoding(String),
    /// Расширение нельзя применить без подмены эффективного кода базовым.
    UnsupportedExtension {
        /// Имя расширения из каталога базы.
        name: String,
        /// Распознан ли признак активности.
        activity_known: bool,
    },
    /// Итоговый нейтральный снимок некорректен.
    Snapshot(SnapshotError),
}
impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingResource(n) => write!(f, "нет ресурса конфигурации `{n}`"),
            Self::DuplicateResource(n) => write!(f, "ресурс конфигурации `{n}` повторяется"),
            Self::MalformedResource(n, r) => write!(f, "ресурс конфигурации `{n}` повреждён: {r}"),
            Self::MalformedContainer(r) => write!(f, "контейнер модуля повреждён: {r}"),
            Self::UnsupportedEncoding(n) => write!(f, "кодировка модуля `{n}` не поддерживается"),
            Self::UnsupportedExtension {
                name,
                activity_known: true,
            } => write!(f, "активное расширение `{name}` пока нельзя применить"),
            Self::UnsupportedExtension {
                name,
                activity_known: false,
            } => write!(f, "у расширения `{name}` не распознана активность"),
            Self::Snapshot(e) => write!(f, "снимок кода некорректен: {e}"),
        }
    }
}
impl Error for LoadError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        if let Self::Snapshot(e) = self {
            Some(e)
        } else {
            None
        }
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
