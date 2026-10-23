//! Индекс модулей и адресное чтение их эффективных ресурсов.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use bsl_config::{ConfigurationIndex, ConfigurationModuleDeclaration, ConfigurationSource};
use open_sdbl::metadata::{
    ConfigHeader, ContentKey, ExtensionInfo, extension_configuration_root,
    parse_config_header_bounded, parse_extension_catalog, parse_extension_index,
    parse_extension_info, parse_extension_objects,
};
use open_sdbl_db::{BlockingDatabaseSession, ConfigurationBootstrap};

use super::*;

#[derive(Clone)]
enum Location {
    Empty,
    Base(String),
    Extension { key: ContentKey, name: String },
}

/// Выбранный источник кода с индексом и одним соединением для адресного чтения.
/// Соединения исполняемых сеансов создаются хостом отдельно и сюда не попадают.
/// Индекс не содержит текстов и не разбирает тела методов.
pub struct DatabaseConfigurationSource {
    index: ConfigurationIndex,
    connection: Mutex<Box<dyn ResourceReader>>,
    locations: HashMap<String, Location>,
    options: LoadOptions,
    roots: [ConfigResource; 2],
    headers: Vec<ObjectHeader>,
    extensions: Vec<(ConfigurationExtension, bool)>,
    resource_names: HashSet<String>,
    owners: HashMap<open_sdbl::metadata::Guid, usize>,
    owner_modules: HashMap<String, Vec<usize>>,
    extension_count: usize,
    metadata: Mutex<Option<metadata::MetadataState>>,
}

struct ObjectHeader {
    class: String,
    header: ConfigHeader,
    location: Location,
    subsystem: Option<MetadataObject>,
}

#[path = "indexed_metadata.rs"]
mod metadata;

struct Discovery<'a> {
    connection: &'a mut dyn ResourceReader,
    options: LoadOptions,
    modules: Vec<ConfigurationModuleDeclaration>,
    locations: HashMap<String, Location>,
    objects: HashMap<String, BaseObject>,
    headers: Vec<ObjectHeader>,
    extensions: Vec<(ConfigurationExtension, bool)>,
}

trait ResourceReader: Send {
    fn bootstrap(&mut self) -> Result<ConfigurationBootstrap, String>;
    fn names(&mut self) -> Result<Vec<String>, String>;
    fn base(&mut self, names: &[&str]) -> Result<Vec<ConfigResource>, String>;
    fn extension(&mut self, keys: &[ContentKey]) -> Result<Vec<ConfigResource>, String>;
    fn ensure_users(&mut self, _: &mut open_sdbl_db::AccessStore) -> Result<(), String> {
        Err("источник не предоставляет пользователей".into())
    }
    fn ensure_rights(
        &mut self,
        _: &mut open_sdbl_db::AccessStore,
        _: &[open_sdbl::metadata::Guid],
    ) -> Result<Vec<open_sdbl::metadata::Guid>, String> {
        Err("источник не предоставляет права".into())
    }
    fn metadata_inputs(&mut self) -> Result<open_sdbl_db::ConfigurationMetadataInputs, String> {
        Err("источник не предоставляет физические метаданные".into())
    }
    fn extension_metadata(
        &mut self,
    ) -> Result<Vec<open_sdbl::metadata::ExtensionMetadata>, String> {
        Err("источник не предоставляет структуры расширений".into())
    }
}

impl ResourceReader for BlockingDatabaseSession {
    fn ensure_users(&mut self, store: &mut open_sdbl_db::AccessStore) -> Result<(), String> {
        BlockingDatabaseSession::ensure_users(self, store).map_err(|error| error.to_string())
    }
    fn ensure_rights(
        &mut self,
        store: &mut open_sdbl_db::AccessStore,
        roles: &[open_sdbl::metadata::Guid],
    ) -> Result<Vec<open_sdbl::metadata::Guid>, String> {
        BlockingDatabaseSession::ensure_rights(self, store, roles)
            .map_err(|error| error.to_string())
    }
    fn metadata_inputs(&mut self) -> Result<open_sdbl_db::ConfigurationMetadataInputs, String> {
        self.configuration_metadata_inputs()
            .map_err(|error| error.to_string())
    }
    fn extension_metadata(
        &mut self,
    ) -> Result<Vec<open_sdbl::metadata::ExtensionMetadata>, String> {
        self.configuration_extension_metadata()
            .map_err(|error| error.to_string())
    }
    fn bootstrap(&mut self) -> Result<ConfigurationBootstrap, String> {
        self.configuration_bootstrap(&mut FactoryConfigSelector::default())
            .map_err(|error| error.to_string())
    }
    fn names(&mut self) -> Result<Vec<String>, String> {
        self.configuration_resource_names()
            .map_err(|error| error.to_string())
    }
    fn base(&mut self, names: &[&str]) -> Result<Vec<ConfigResource>, String> {
        self.configuration_resources(names)
            .map_err(|error| error.to_string())
    }
    fn extension(&mut self, keys: &[ContentKey]) -> Result<Vec<ConfigResource>, String> {
        self.extension_resources(keys)
            .map_err(|error| error.to_string())
    }
}

impl DatabaseConfigurationSource {
    /// Приобретает корень, заголовки объектов и индекс расширений без чтения тел.
    ///
    /// # Errors
    /// Возвращает ошибки соединения, повреждённых заголовков и неподдержанных
    /// структурных операций расширений. Частичный индекс не публикуется.
    pub fn acquire(
        connection: BlockingDatabaseSession,
        options: LoadOptions,
    ) -> Result<Self, String> {
        Self::from_reader(Box::new(connection), options)
    }

    /// Приобретает индекс и ORM для отдельного файлового источника кода.
    /// Тела модулей БД, в том числе модуля сеанса расширений, не читаются.
    ///
    /// # Errors
    /// Возвращает ошибки соединения, описаний или структуры расширений.
    pub fn acquire_orm(
        connection: BlockingDatabaseSession,
        options: LoadOptions,
    ) -> Result<Self, String> {
        Self::from_reader_mode(Box::new(connection), options, false)
    }

    /// Приобретает пользователей через уже обнаруженный формат хранения источника.
    ///
    /// # Errors
    /// Возвращает ошибку соединения или декодирования пользователей SDK.
    pub fn ensure_users(&self, store: &mut open_sdbl_db::AccessStore) -> Result<(), String> {
        self.connection
            .lock()
            .map_err(|_| "соединение источника отравлено")?
            .ensure_users(store)
    }

    /// Приобретает права выбранных ролей, сохраняя диагностику отсутствующих записей SDK.
    ///
    /// # Errors
    /// Возвращает ошибку соединения или декодирования прав.
    pub fn ensure_rights(
        &self,
        store: &mut open_sdbl_db::AccessStore,
        roles: &[open_sdbl::metadata::Guid],
    ) -> Result<Vec<open_sdbl::metadata::Guid>, String> {
        self.connection
            .lock()
            .map_err(|_| "соединение источника отравлено")?
            .ensure_rights(store, roles)
    }

    fn from_reader(
        connection: Box<dyn ResourceReader>,
        options: LoadOptions,
    ) -> Result<Self, String> {
        Self::from_reader_mode(connection, options, true)
    }

    fn from_reader_mode(
        mut connection: Box<dyn ResourceReader>,
        options: LoadOptions,
        read_code_bodies: bool,
    ) -> Result<Self, String> {
        let bootstrap = connection.bootstrap()?;
        let names = connection.names()?;
        let mut discovery = Discovery {
            connection: connection.as_mut(),
            options,
            modules: Vec::new(),
            locations: HashMap::new(),
            objects: HashMap::new(),
            headers: Vec::new(),
            extensions: Vec::new(),
        };
        let identity = discovery.base(&bootstrap, &names)?;
        discovery.extensions_mode(&bootstrap, read_code_bodies)?;
        let index = ConfigurationIndex::new(identity, discovery.modules)
            .map_err(|error| error.to_string())?;
        let locations = discovery.locations;
        let headers = discovery.headers;
        let extensions = discovery.extensions;
        let mut owners = HashMap::new();
        for (position, entry) in headers.iter().enumerate() {
            for id in &entry.header.member_guids {
                owners.entry(id.clone()).or_insert(position);
            }
            // Идентичность определяемого типа связывает объявление реквизита
            // с ресурсом определения до приобретения его подробностей.
            for ty in [
                &entry.header.reference_type,
                &entry.header.object_type,
                &entry.header.route_point_reference_type,
                &entry.header.defined_type,
            ] {
                if let open_sdbl::metadata::ConfigTypeIdentity::Known(id) = ty {
                    owners.entry(id.clone()).or_insert(position);
                }
            }
        }
        let mut owner_modules: HashMap<String, Vec<usize>> = HashMap::new();
        for (position, module) in index.modules().iter().enumerate() {
            if let Some(owner) = module.owner() {
                owner_modules
                    .entry(owner.to_owned())
                    .or_default()
                    .push(position);
            }
        }
        work_count("indexed.discovered_objects", headers.len());
        work_count("indexed.discovered_modules", index.modules().len());
        Ok(Self {
            index,
            connection: Mutex::new(connection),
            locations,
            options,
            owners,
            owner_modules,
            roots: [bootstrap.root, bootstrap.descriptor],
            headers,
            extensions,
            resource_names: names
                .into_iter()
                .map(|name| name.to_ascii_lowercase())
                .collect(),
            extension_count: bootstrap.extensions.len(),
            metadata: Mutex::new(None),
        })
    }
}

impl ConfigurationSource for DatabaseConfigurationSource {
    fn index(&self) -> &ConfigurationIndex {
        &self.index
    }

    fn module_source(&self, id: &str) -> Result<Arc<str>, String> {
        self.text(id, false).map(|(text, _)| text)
    }

    fn module_declarations(&self, id: &str) -> Result<bsl_config::ModuleDeclarationSource, String> {
        self.text(id, true).map(|(text, complete)| {
            if complete {
                bsl_config::ModuleDeclarationSource::Complete(text)
            } else {
                bsl_config::ModuleDeclarationSource::Declarations(text)
            }
        })
    }
}

impl DatabaseConfigurationSource {
    fn text(&self, id: &str, declarations: bool) -> Result<(Arc<str>, bool), String> {
        let location = self
            .locations
            .get(id)
            .ok_or_else(|| format!("нет ресурса модуля «{id}»"))?;
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| "соединение источника отравлено".to_string())?;
        let resource = read_location(connection.as_mut(), location)?;
        drop(connection);
        let Some(resource) = resource else {
            return Ok((Arc::from(""), true));
        };
        let (owner, suffix) = resource
            .file_name
            .rsplit_once('.')
            .ok_or_else(|| format!("ресурс «{}» не является модулем", resource.file_name))?;
        // Декодер и политика защищённых модулей общие с загрузчиком снимка.
        let source = Resources::with_options(std::slice::from_ref(&resource), self.options)
            .and_then(|resources| resources.module_text(owner, suffix, declarations))
            .map_err(|error| error.to_string())?;
        let (text, complete) = source.unwrap_or_else(|| (String::new(), true));
        Ok((text.into(), complete))
    }
}

fn read_location(
    connection: &mut dyn ResourceReader,
    location: &Location,
) -> Result<Option<ConfigResource>, String> {
    let mut resources = match location {
        Location::Empty => return Ok(None),
        Location::Base(name) => connection.base(&[name.as_str()]),
        Location::Extension { key, .. } => connection.extension(std::slice::from_ref(key)),
    }?;
    if resources.len() != 1 {
        return Err("адресное чтение не вернуло единственный ресурс".into());
    }
    let mut resource = resources.pop().expect("проверено количество");
    if let Location::Extension { name, .. } = location {
        resource.file_name = name.clone();
    }
    Ok(Some(resource))
}

impl Discovery<'_> {
    fn replace(&mut self, module: ConfigurationModuleDeclaration, location: Option<Location>) {
        if let Some(location) = location {
            self.locations.insert(module.id().into(), location);
        } else {
            // Отсутствующий ресурс общего модуля означает отсутствие его
            // тела, как в прежнем `load_base_graph`; существующую замену сохраняем.
            self.locations
                .entry(module.id().into())
                .or_insert(Location::Empty);
        }
        if let Some(existing) = self
            .modules
            .iter_mut()
            .find(|known| known.id() == module.id())
        {
            *existing = module;
        } else {
            self.modules.push(module);
        }
    }

    fn base(
        &mut self,
        bootstrap: &ConfigurationBootstrap,
        names: &[String],
    ) -> Result<String, String> {
        let roots = [bootstrap.root.clone(), bootstrap.descriptor.clone()];
        let resources =
            Resources::with_options(&roots, self.options).map_err(|error| error.to_string())?;
        let value = parse_serialized(
            &resources
                .decoded(&bootstrap.descriptor.file_name)
                .map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())?;
        let identity = configuration_identity(&value)
            .ok_or("нет идентичности конфигурации")?
            .to_ascii_lowercase();
        let names: HashMap<_, _> = names
            .iter()
            .map(|name| (name.to_ascii_lowercase(), name))
            .collect();
        let session = format!("{identity}.7");
        if let Some(name) = names.get(&session) {
            self.replace(
                ConfigurationModuleDeclaration::new(
                    &session,
                    "МодульСеанса",
                    ModuleKind::Session,
                    ModuleExecution::new().with_server(true),
                ),
                Some(Location::Base((*name).clone())),
            );
        }
        let mut objects = Vec::new();
        for include in collections(&value).ok_or("нет каталога объектов")? {
            let count = scalar(include.get(2).ok_or("нет размера каталога")?)
                .and_then(|value| value.parse::<usize>().ok())
                .ok_or("неизвестен размер каталога")?;
            for collection in include.iter().skip(3).take(count) {
                if let Some((class, ids)) = collection_ids(collection) {
                    objects.extend(ids.into_iter().map(|id| (class, id)));
                }
            }
        }
        // Заголовки читаются небольшими пакетами: один SQL на объект сделал
        // бы сетевую задержку основной ценой индекса большой конфигурации.
        for batch in objects.chunks(64) {
            let requested = batch
                .iter()
                .map(|(_, id)| {
                    names
                        .get(&id.to_ascii_lowercase())
                        .map_or(*id, |name| name.as_str())
                })
                .collect::<Vec<_>>();
            let mut resources = HashMap::new();
            for resource in self.connection.base(&requested)? {
                let key = resource.file_name.to_ascii_lowercase();
                if resources.insert(key.clone(), resource).is_some() {
                    return Err(LoadError::DuplicateResource(key).to_string());
                }
            }
            for (class, id) in batch {
                let resource = resources
                    .remove(&id.to_ascii_lowercase())
                    .ok_or_else(|| format!("нет заголовка «{id}»"))?;
                self.base_object(class, id, &resource, &names)?;
            }
            if !resources.is_empty() {
                return Err("адресное чтение вернуло лишние ресурсы".into());
            }
        }
        Ok(identity)
    }

    fn base_object(
        &mut self,
        class: &str,
        id: &str,
        resource: &ConfigResource,
        names: &HashMap<String, &String>,
    ) -> Result<(), String> {
        let header = parse_config_header_bounded(id, &resource.compressed, 32 * 1024 * 1024)
            .map_err(|error| error.to_string())?
            .ok_or_else(|| format!("нет заголовка «{id}»"))?;
        if self
            .headers
            .iter()
            .any(|entry| entry.header.object_guid == header.object_guid)
        {
            return Err(format!("повтор объекта «{id}»"));
        }
        let subsystem = if class.eq_ignore_ascii_case(SUBSYSTEM) {
            let bytes =
                inflate_raw_deflate(&resource.compressed).map_err(|error| error.to_string())?;
            let value = parse_serialized(&bytes).map_err(|error| error.to_string())?;
            Some((
                subsystem_object(&value, id, &header.name, None)
                    .map_err(|error| error.to_string())?,
                subsystem_children(&value, id)
                    .map_err(|error| error.to_string())?
                    .into_iter()
                    .map(str::to_string)
                    .collect::<Vec<_>>(),
            ))
        } else {
            None
        };
        self.headers.push(ObjectHeader {
            class: class.to_ascii_lowercase(),
            header: header.clone(),
            location: Location::Base(resource.file_name.clone()),
            subsystem: subsystem.as_ref().map(|(object, _)| object.clone()),
        });
        self.objects.insert(
            id.to_ascii_lowercase(),
            BaseObject {
                class: class.to_ascii_lowercase(),
                name: header.name.clone(),
            },
        );
        if class.eq_ignore_ascii_case(COMMON) {
            let bytes =
                inflate_raw_deflate(&resource.compressed).map_err(|error| error.to_string())?;
            let execution = common_execution(&bytes).map_err(|error| error.to_string())?;
            let source = format!("{id}.0");
            self.replace(
                ConfigurationModuleDeclaration::new(
                    &source,
                    header.name,
                    ModuleKind::Common,
                    execution,
                ),
                names
                    .get(&source)
                    .map(|name| Location::Base((*name).clone())),
            );
        } else {
            for slot in module_slots(class) {
                let source = format!("{id}.{}", slot.suffix);
                if let Some(name) = names.get(&source) {
                    self.replace(
                        ConfigurationModuleDeclaration::new(
                            &source,
                            format!("{}.{}.{}", slot.prefix, header.name, slot.label),
                            slot.kind,
                            ModuleExecution::new().with_server(true),
                        )
                        .with_owner(id),
                        Some(Location::Base((*name).clone())),
                    );
                }
            }
        }
        if let Some((_, children)) = subsystem {
            for child in children {
                let resource = read_location(self.connection, &Location::Base(child.clone()))?
                    .ok_or("нет вложенной подсистемы")?;
                self.base_object(SUBSYSTEM, &child, &resource, names)?;
                let entry = self
                    .headers
                    .iter_mut()
                    .find(|entry| entry.header.object_guid.as_str() == child)
                    .expect("подсистема добавлена");
                entry.subsystem = entry
                    .subsystem
                    .take()
                    .map(|object| object.with_subsystem_parent(id));
            }
        }
        Ok(())
    }

    fn extensions_mode(
        &mut self,
        bootstrap: &ConfigurationBootstrap,
        read_code_bodies: bool,
    ) -> Result<(), String> {
        let info = bootstrap
            .extensions
            .iter()
            .map(|row| parse_extension_info(&row.info))
            .collect::<Vec<_>>();
        let orders = bootstrap
            .extensions
            .iter()
            .map(|row| {
                u32::try_from(row.order).map_err(|_| {
                    format!(
                        "extension {:?} reports the negative application order {}",
                        row.name, row.order
                    )
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let positions = ordered_active_positions(bootstrap.extensions.iter().enumerate().map(
            |(index, row)| {
                (
                    index,
                    row.name.as_str(),
                    info[index].as_ref().and_then(|info| info.active),
                    orders[index],
                )
            },
        ))
        .map_err(|error| error.to_string())?;
        let mut all = (0..bootstrap.extensions.len()).collect::<Vec<_>>();
        all.sort_by_key(|position| bootstrap.extensions[*position].order);
        for position in all {
            let row = &bootstrap.extensions[position];
            let info = info[position].as_ref().ok_or("нет сведений расширения")?;
            let active = positions.contains(&position);
            self.extension_mode(row, info, active, read_code_bodies)?;
        }
        Ok(())
    }

    #[cfg(test)]
    fn extension(
        &mut self,
        row: &open_sdbl_db::ExtensionCatalogRow,
        info: &ExtensionInfo,
        active: bool,
    ) -> Result<(), String> {
        self.extension_mode(row, info, active, true)
    }

    fn extension_mode(
        &mut self,
        row: &open_sdbl_db::ExtensionCatalogRow,
        info: &ExtensionInfo,
        active: bool,
        read_code_bodies: bool,
    ) -> Result<(), String> {
        let name = row.name.as_str();
        let root = read_location(
            self.connection,
            &Location::Extension {
                key: info.root_key,
                name: "root".into(),
            },
        )?
        .ok_or("нет корня расширения")?;
        let bytes = inflate_raw_deflate(&root.compressed).map_err(|error| error.to_string())?;
        let root_name = extension_configuration_root(&bytes)
            .map_err(|error| error.to_string())?
            .ok_or("нет GUID конфигурации расширения")?;
        let mut locations = HashMap::new();
        for resource in parse_extension_index(&bytes).map_err(|error| error.to_string())? {
            let key = resource.name.to_ascii_lowercase();
            let location = Location::Extension {
                key: resource.key,
                name: resource.name,
            };
            if locations.insert(key.clone(), location).is_some() {
                return Err(LoadError::DuplicateResource(key).to_string());
            }
        }
        let root_location = locations
            .get(root_name.as_str())
            .ok_or("нет описателя конфигурации расширения")?;
        let resource =
            read_location(self.connection, root_location)?.ok_or("нет описателя расширения")?;
        let bytes = inflate_raw_deflate(&resource.compressed).map_err(|error| error.to_string())?;
        let catalog = parse_extension_catalog(&bytes).map_err(|error| error.to_string())?;
        let identity = row
            .identity
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        let description = project_extension_description(
            &identity,
            name,
            info.active,
            info.runtime_version.as_deref(),
            info.hash_sum.as_deref(),
            info.safe_mode,
            info.security_profile_name.as_deref(),
            (row.scope == Some(1)).then_some(ExtensionScope::InfoBase),
            match row.used_in_distributed_infobase {
                Some(0) => Some(false),
                Some(1) => Some(true),
                _ => None,
            },
        )
        .map_err(|error| error.to_string());
        // Неполная тестовая/legacy запись по-прежнему пригодна для кода;
        // полноту ORM-проекции проверяет metadata_catalog.
        if let Ok(description) = description {
            self.extensions.push((
                project_extension_purpose(description, catalog.purpose),
                active,
            ));
        }
        if !active {
            return Ok(());
        }
        if read_code_bodies
            && let Some(location) = locations.get(&format!("{}.7", catalog.configuration))
        {
            let resource = read_location(self.connection, location)?
                .ok_or("нет ресурса модуля сеанса расширения")?;
            let resources = Resources::with_options(std::slice::from_ref(&resource), self.options)
                .map_err(|error| error.to_string())?;
            if resources
                .module_source(catalog.configuration.as_str(), "7")
                .map_err(|error| error.to_string())?
                .is_some()
            {
                return Err(LoadError::UnsupportedExtensionOperation {
                    extension: name.into(),
                    object: catalog.configuration.to_string(),
                }
                .to_string());
            }
        }
        let mut seen = HashSet::new();
        for class in catalog.classes {
            for id in class.objects {
                if !seen.insert(id.clone()) {
                    return Err("повтор объекта расширения".into());
                }
                let location = locations
                    .get(id.as_str())
                    .ok_or("нет описания объекта расширения")?;
                let resource =
                    read_location(self.connection, location)?.ok_or("нет описания расширения")?;
                let bytes =
                    inflate_raw_deflate(&resource.compressed).map_err(|error| error.to_string())?;
                let headers = parse_extension_objects(&bytes).map_err(|error| error.to_string())?;
                let object = headers
                    .iter()
                    .find(|header| header.id == id)
                    .ok_or("нет собственного заголовка объекта расширения")?;
                let (target, target_name) =
                    extension_target(&self.objects, class.class.as_str(), object, name)
                        .map_err(|error| error.to_string())?;
                let (target, target_name) = (target.to_string(), target_name.to_string());
                if object.belonging == Some(ObjectBelonging::Own) {
                    let header = parse_config_header_bounded(
                        id.as_str(),
                        &resource.compressed,
                        32 * 1024 * 1024,
                    )
                    .map_err(|error| error.to_string())?
                    .unwrap_or_else(|| ConfigHeader {
                        object_guid: id.clone(),
                        name: object.name.clone(),
                        member_guids: vec![id.clone()],
                        synonyms: vec![],
                        enumeration_values: None,
                        route_point_reference_type:
                            open_sdbl::metadata::ConfigTypeIdentity::Unknown,
                        reference_type: open_sdbl::metadata::ConfigTypeIdentity::Unknown,
                        object_type: open_sdbl::metadata::ConfigTypeIdentity::Unknown,
                        defined_type: open_sdbl::metadata::ConfigTypeIdentity::Unknown,
                        sections: None,
                    });
                    self.headers.push(ObjectHeader {
                        class: class.class.to_string(),
                        header,
                        location: location.clone(),
                        subsystem: None,
                    });
                }
                if class.class.as_str().eq_ignore_ascii_case(COMMON) {
                    let execution = common_execution(&bytes).map_err(|error| error.to_string())?;
                    let local_source = format!("{id}.0");
                    let location = locations.get(&local_source).cloned();
                    let extension_source = location.is_some()
                        || self.modules.iter().any(|module| {
                            module.id() == format!("{target}.0") && module.source_is_extension()
                        });
                    self.replace(
                        ConfigurationModuleDeclaration::new(
                            format!("{target}.0"),
                            target_name,
                            ModuleKind::Common,
                            execution,
                        )
                        .with_extension_source(extension_source),
                        location,
                    );
                } else {
                    for slot in module_slots(class.class.as_str()) {
                        let Some(location) = locations.get(&format!("{id}.{}", slot.suffix)) else {
                            continue;
                        };
                        let mut module = ConfigurationModuleDeclaration::new(
                            format!("{target}.{}", slot.suffix),
                            format!("{}.{}.{}", slot.prefix, target_name, slot.label),
                            slot.kind,
                            ModuleExecution::new().with_server(true),
                        )
                        .with_owner(&target)
                        .with_extension_source(true);
                        if slot.kind == ModuleKind::Object
                            && object.belonging == Some(ObjectBelonging::Own)
                            && info.security_profile_name.as_deref() == Some("")
                            && let Some(safe) = info.safe_mode
                        {
                            module = module.with_own_extension_source(safe);
                        }
                        self.replace(module, Some(location.clone()));
                    }
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "indexed_tests.rs"]
mod tests;

fn work_count(stage: &str, count: usize) {
    if std::env::var_os("OPEN_BSL_STARTUP_TIMINGS").is_some_and(|value| value == "1") {
        eprintln!("open-bsl.work\t{stage}\t{count}");
    }
}

#[path = "indexed_file_sources.rs"]
mod file_sources;
