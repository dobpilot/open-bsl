//! Связывание файлового комплекта по проверенным структурам расширений БД.
use super::*;
use bsl_config::{
    ConfigurationArtifact, ConfigurationArtifacts, ConfigurationDescription, ConfigurationProperty,
};

struct Origin {
    source: Arc<dyn ConfigurationSource>,
    id: Arc<str>,
}
type PropertyCache = std::sync::OnceLock<Result<Option<Arc<ConfigurationProperty>>, String>>;

struct FileBundle {
    index: ConfigurationIndex,
    options: LoadOptions,
    module_cache: std::sync::Mutex<HashMap<String, Result<Arc<str>, String>>>,
    root: String,
    modules: HashMap<String, Origin>,
    descriptions: Vec<ConfigurationDescription>,
    origins: HashMap<String, Vec<Origin>>,
    properties: HashMap<String, PropertyCache>,
}
impl FileBundle {
    fn new(source: Arc<dyn ConfigurationSource>, options: LoadOptions) -> Result<Self, String> {
        let artifacts = source.artifacts().ok_or("нет файловых описаний")?;
        let descriptions = artifacts.descriptions().to_vec();
        let root = artifacts.root_identity().to_owned();
        let origins = descriptions
            .iter()
            .map(|d| {
                (
                    d.id.to_string(),
                    vec![Origin {
                        source: Arc::clone(&source),
                        id: Arc::clone(&d.id),
                    }],
                )
            })
            .collect();
        let modules = source
            .index()
            .modules()
            .iter()
            .map(|m| {
                (
                    m.id().into(),
                    Origin {
                        source: Arc::clone(&source),
                        id: m.id().into(),
                    },
                )
            })
            .collect();
        let properties = descriptions
            .iter()
            .map(|d| (d.id.to_string(), std::sync::OnceLock::new()))
            .collect();
        Ok(Self {
            index: source.index().clone(),
            options,
            module_cache: std::sync::Mutex::new(HashMap::new()),
            root,
            modules,
            descriptions,
            origins,
            properties,
        })
    }
    fn append(
        &mut self,
        source: Arc<dyn ConfigurationSource>,
        targets: &HashMap<String, String>,
        info: &ExtensionInfo,
    ) -> Result<(), String> {
        let artifacts = source
            .artifacts()
            .ok_or("нет файловых описаний расширения")?;
        let root = artifacts.root_identity();
        let mut declarations = self.index.modules().to_vec();
        let mut module_positions: HashMap<_, _> = declarations
            .iter()
            .enumerate()
            .map(|(i, m)| (m.id().to_owned(), i))
            .collect();
        let mut description_positions: HashMap<_, _> = self
            .descriptions
            .iter()
            .enumerate()
            .map(|(i, d)| (d.id.to_string(), i))
            .collect();
        let mut names = open_bsl::NameInterner::new();
        let mut names_by_owner: HashMap<_, _> = self
            .descriptions
            .iter()
            .enumerate()
            .map(|(i, d)| ((d.owner.clone(), d.kind.clone(), names.intern(&d.name)), i))
            .collect();
        for module in source.index().modules() {
            if module.kind() == ModuleKind::Session {
                return Err("модуль сеанса файлового расширения не поддерживается".into());
            }
            let local_owner = module
                .owner()
                .unwrap_or_else(|| module.id().split('/').next().unwrap_or(module.id()));
            let owner = targets.get(local_owner).ok_or_else(|| {
                format!(
                    "нет проверенной связи владельца модуля «{}» ({local_owner})",
                    module.name()
                )
            })?;
            let id = format!("{owner}/{:?}", module.kind());
            let position = module_positions.get(&id).copied();
            let name = position.map_or(module.name(), |p| declarations[p].name());
            let mut effective =
                ConfigurationModuleDeclaration::new(&id, name, module.kind(), module.execution())
                    .with_imports(module.imports().to_vec())
                    .with_extension_source(true);
            if module.owner().is_some() {
                effective = effective.with_owner(owner);
            }
            if owner == local_owner
                && module.kind() == ModuleKind::Object
                && info.security_profile_name.as_deref() == Some("")
                && let Some(safe) = info.safe_mode
            {
                effective = effective.with_own_extension_source(safe);
            }
            if let Some(position) = position {
                declarations[position] = effective;
            } else {
                module_positions.insert(id.clone(), declarations.len());
                declarations.push(effective);
            }
            self.modules.insert(
                id,
                Origin {
                    source: Arc::clone(&source),
                    id: module.id().into(),
                },
            );
        }
        // Заимствованный общий модуль может менять только флаги. Его тело
        // остаётся в выбранной основной выгрузке, а не в резервной копии БД.
        let local_bodies: HashSet<_> = source
            .index()
            .modules()
            .iter()
            .filter(|m| m.kind() == ModuleKind::Common)
            .filter_map(|m| m.id().split('/').next())
            .collect();
        for description in artifacts
            .descriptions()
            .iter()
            .filter(|d| d.kind.as_ref() == "CommonModule" && !local_bodies.contains(d.id.as_ref()))
        {
            let owner = targets
                .get(description.id.as_ref())
                .ok_or("нет проверенного владельца заимствованного общего модуля")?;
            let id = format!("{owner}/Common");
            let position = module_positions
                .get(&id)
                .copied()
                .ok_or("нет основного файлового тела заимствованного общего модуля")?;
            let properties = artifacts
                .description(&description.id)?
                .ok_or("нет описания общего модуля расширения")?;
            let old = &declarations[position];
            declarations[position] = ConfigurationModuleDeclaration::new(
                old.id(),
                old.name(),
                old.kind(),
                ModuleExecution::from_properties(&properties)?,
            )
            .with_imports(old.imports().to_vec())
            .with_extension_source(old.source_is_extension());
        }
        for description in artifacts.descriptions() {
            if description.id.as_ref() == root {
                continue;
            }
            let target = targets
                .get(description.id.as_ref())
                .map(String::as_str)
                .unwrap_or(&description.id);
            let mut effective = description.clone();
            effective.id = target.into();
            if let Some(owner) = &description.owner {
                effective.owner = Some(
                    targets
                        .get(owner.as_ref())
                        .map_or_else(|| Arc::clone(owner), |id| Arc::from(id.as_str())),
                );
            }
            let name_key = (
                effective.owner.clone(),
                effective.kind.clone(),
                names.intern(&effective.name),
            );
            if let Some(&position) = names_by_owner.get(&name_key)
                && self.descriptions[position].id != effective.id
            {
                return Err(format!(
                    "нет проверенной связи заимствованного ресурса «{}» ({})",
                    effective.name, effective.id
                ));
            }
            names_by_owner.insert(
                name_key,
                description_positions
                    .get(target)
                    .copied()
                    .unwrap_or(self.descriptions.len()),
            );
            if let Some(&position) = description_positions.get(target) {
                let previous = &mut self.descriptions[position];
                if previous.kind != effective.kind || previous.name != effective.name {
                    return Err(format!(
                        "несовместимое файловое заимствование «{}»",
                        description.name
                    ));
                }
                *previous = effective;
            } else {
                description_positions.insert(target.to_owned(), self.descriptions.len());
                self.descriptions.push(effective);
            }
            self.origins.entry(target.into()).or_default().push(Origin {
                source: Arc::clone(&source),
                id: Arc::clone(&description.id),
            });
        }
        self.index = ConfigurationIndex::new(self.index.identity(), declarations)
            .map_err(|error| error.to_string())?;
        Ok(())
    }
}
impl ConfigurationSource for FileBundle {
    fn index(&self) -> &ConfigurationIndex {
        &self.index
    }
    fn module_source(&self, id: &str) -> Result<Arc<str>, String> {
        let origin = self.modules.get(id).ok_or("нет файлового модуля")?;
        let mut cache = self
            .module_cache
            .lock()
            .map_err(|_| "кэш модулей отравлен")?;
        cache
            .entry(id.into())
            .or_insert_with(|| {
                if let Some(image) = origin.source.module_image(&origin.id)? {
                    if !image.starts_with(&[0xff, 0xff, 0xff, 0x7f]) {
                        return Err(format!("{id}: неизвестный формат нативного образа"));
                    }
                    // Образ приобретён из выбранного каталога. Политика та же,
                    // что для БД; чтение исходника из БД здесь не допускается.
                    return decode_module_payload(id, image.to_vec(), self.options, false)
                        .map_err(|error| format!("{id}: {error}"))?
                        .map(|(source, _)| Arc::from(source))
                        .ok_or_else(|| {
                            format!("{id}: в нативном образе нет текста или интерфейса")
                        });
                }
                origin.source.module_source(&origin.id)
            })
            .clone()
    }
    fn module_declarations(&self, id: &str) -> Result<bsl_config::ModuleDeclarationSource, String> {
        if self.options.decompile_protected_modules {
            let origin = self.modules.get(id).ok_or("нет файлового модуля")?;
            if let Some(image) = origin.source.module_image(&origin.id)? {
                if !image.starts_with(&[0xff, 0xff, 0xff, 0x7f]) {
                    return Err(format!("{id}: неизвестный формат нативного образа"));
                }
                let (source, complete) =
                    decode_module_payload(id, image.to_vec(), self.options, true)
                        .map_err(|error| format!("{id}: {error}"))?
                        .ok_or_else(|| format!("{id}: нет объявлений нативного модуля"))?;
                return Ok(if complete {
                    bsl_config::ModuleDeclarationSource::Complete(source.into())
                } else {
                    bsl_config::ModuleDeclarationSource::Declarations(source.into())
                });
            }
        }
        self.module_source(id)
            .map(bsl_config::ModuleDeclarationSource::Complete)
    }
    fn artifacts(&self) -> Option<&dyn ConfigurationArtifacts> {
        Some(self)
    }
}
impl ConfigurationArtifacts for FileBundle {
    fn root_identity(&self) -> &str {
        &self.root
    }
    fn descriptions(&self) -> &[ConfigurationDescription] {
        &self.descriptions
    }
    fn description(&self, id: &str) -> Result<Option<Arc<ConfigurationProperty>>, String> {
        let Some(origins) = self.origins.get(id) else {
            return Ok(None);
        };
        let cache = self
            .properties
            .get(id)
            .ok_or("нет кэша описания комплекта")?;
        cache
            .get_or_init(|| {
                let mut result: Option<Arc<ConfigurationProperty>> = None;
                for origin in origins {
                    let properties = origin
                        .source
                        .artifacts()
                        .ok_or("нет файловых описаний")?
                        .description(&origin.id)?
                        .ok_or("нет объявленного описания расширения")?;
                    result = Some(match result {
                        None => properties,
                        Some(base) => Arc::new(merge(&base, &properties)?),
                    });
                }
                Ok(result)
            })
            .clone()
    }
    fn artifact(&self, id: &str) -> Result<Option<Arc<ConfigurationArtifact>>, String> {
        let Some(origins) = self.origins.get(id) else {
            return Ok(None);
        };
        for origin in origins.iter().rev() {
            if let Some(artifact) = origin
                .source
                .artifacts()
                .ok_or("нет файловых ресурсов")?
                .artifact(&origin.id)?
            {
                let mut artifact = (*artifact).clone();
                artifact.description = self
                    .descriptions
                    .iter()
                    .find(|d| d.id.as_ref() == id)
                    .ok_or("нет эффективного описания")?
                    .clone();
                return Ok(Some(Arc::new(artifact)));
            }
        }
        Ok(None)
    }
}

fn nested_target(
    base: &[ConfigurationDescription],
    description: &ConfigurationDescription,
    owner: &str,
    header: &open_sdbl::metadata::ExtensionObject,
) -> Result<String, String> {
    if !open_bsl::folded_eq(&header.name, &description.name) {
        return Err("имя ресурса не совпадает с заголовком расширения".into());
    }
    if header.belonging == Some(ObjectBelonging::Own) {
        return Ok(description.id.to_string());
    }
    if header.belonging != Some(ObjectBelonging::Adopted) {
        return Err("не приобретена принадлежность ресурса расширения".into());
    }
    let mut matches = base.iter().filter(|candidate| {
        candidate.kind == description.kind
            && candidate.owner.as_deref() == Some(owner)
            && header.extends.as_ref().map_or_else(
                || open_bsl::folded_eq(&candidate.name, &description.name),
                |target| candidate.id.as_ref() == target.as_str(),
            )
    });
    let target = matches
        .next()
        .ok_or("не найден заимствованный ресурс проверенного владельца")?;
    if matches.next().is_some() {
        return Err("неоднозначное имя заимствованного ресурса".into());
    }
    Ok(target.id.to_string())
}

fn merge(
    base: &ConfigurationProperty,
    overlay: &ConfigurationProperty,
) -> Result<ConfigurationProperty, String> {
    let mut result = base.clone();
    let mut children = base.children.to_vec();
    for property in overlay.children.iter() {
        // Структурное расширение коллекций требует измеренного соответствия
        // вложенных владельцев. Непроверенный частичный список не публикуется.
        if matches!(
            property.name.as_ref(),
            "attributes" | "tabularSections" | "commands"
        ) && !property.children.is_empty()
        {
            return Err(format!(
                "не поддержано файловое расширение коллекции «{}»",
                property.name
            ));
        }
        if let Some(existing) = children.iter_mut().find(|p| p.name == property.name) {
            *existing = property.clone();
        } else {
            children.push(property.clone());
        }
    }
    result.children = children.into();
    Ok(result)
}

impl DatabaseConfigurationSource {
    /// Связывает файловые расширения в порядке применения, приобретённом из БД.
    /// Код БД не читается и не участвует в эффективном комплекте.
    ///
    /// # Errors
    /// Возвращает ошибку идентичности, отсутствующего входа или неподдержанной операции.
    pub fn bind_file_sources(
        &self,
        base: Arc<dyn ConfigurationSource>,
        extensions: Vec<(String, Arc<dyn ConfigurationSource>)>,
    ) -> Result<Arc<dyn ConfigurationSource>, String> {
        if self.extension_count == 0 {
            if !extensions.is_empty() {
                return Err("заданы каталоги неизвестных расширений".into());
            }
            return Ok(Arc::new(FileBundle::new(base, self.options)?));
        }
        let mut supplied = HashMap::new();
        for (id, source) in extensions {
            if supplied.insert(id.to_ascii_lowercase(), source).is_some() {
                return Err("повтор файлового расширения".into());
            }
        }
        let mut bundle = FileBundle::new(base, self.options)?;
        let objects: HashMap<_, _> = self
            .headers
            .iter()
            .filter(|h| !matches!(h.location, Location::Extension { .. }))
            .map(|h| {
                (
                    h.header.object_guid.to_string(),
                    BaseObject {
                        class: h.class.clone(),
                        name: h.header.name.clone(),
                    },
                )
            })
            .collect();
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| "соединение источника отравлено")?;
        let bootstrap = connection.bootstrap()?;
        let mut rows: Vec<_> = bootstrap.extensions.iter().collect();
        rows.sort_by_key(|row| row.order);
        for row in rows {
            let bytes: [u8; 16] = row
                .identity
                .as_slice()
                .try_into()
                .map_err(|_| "неверный UUID расширения")?;
            let id = open_bsl::uuid::format(&canonical_uuid_from_physical(bytes));
            let source = supplied.remove(&id);
            let info = parse_extension_info(&row.info).ok_or("нет сведений расширения")?;
            match info.active {
                Some(false) => continue,
                None => return Err("не приобретена активность расширения".into()),
                Some(true) => {}
            }
            let source = source.ok_or_else(|| {
                format!(
                    "нет файлового источника активного расширения «{}» ({id})",
                    row.name
                )
            })?;
            let resource = read_location(
                connection.as_mut(),
                &Location::Extension {
                    key: info.root_key,
                    name: "root".into(),
                },
            )?
            .ok_or("нет корня расширения")?;
            let root = inflate_raw_deflate(&resource.compressed).map_err(|e| e.to_string())?;
            let descriptor_id = extension_configuration_root(&root)
                .map_err(|e| e.to_string())?
                .ok_or("нет корня конфигурации расширения")?;
            let resources: HashMap<_, _> = parse_extension_index(&root)
                .map_err(|e| e.to_string())?
                .into_iter()
                .map(|entry| (entry.name.to_ascii_lowercase(), entry.key))
                .collect();
            let mut read = |name: &str| -> Result<Vec<u8>, String> {
                let key = *resources
                    .get(name)
                    .ok_or_else(|| format!("нет описания расширения {name}"))?;
                let resource = read_location(
                    connection.as_mut(),
                    &Location::Extension {
                        key,
                        name: name.into(),
                    },
                )?
                .ok_or("нет описания расширения")?;
                inflate_raw_deflate(&resource.compressed).map_err(|e| e.to_string())
            };
            let catalog = parse_extension_catalog(&read(descriptor_id.as_str())?)
                .map_err(|e| e.to_string())?;
            if !source.artifacts().is_some_and(|a| {
                a.root_identity()
                    .eq_ignore_ascii_case(catalog.configuration.as_str())
            }) {
                return Err(format!(
                    "UUID файлового расширения «{}» не совпадает с БД",
                    row.name
                ));
            }
            let file_ids: HashSet<_> = source
                .artifacts()
                .ok_or("нет описаний расширения")?
                .descriptions()
                .iter()
                .map(|d| d.id.as_ref())
                .collect();
            let mut targets = HashMap::new();
            for class in catalog.classes {
                for object_id in class.objects {
                    let headers = parse_extension_objects(&read(object_id.as_str())?)
                        .map_err(|e| e.to_string())?;
                    let object = headers
                        .iter()
                        .find(|h| h.id == object_id)
                        .ok_or("нет собственного заголовка расширения")?;
                    if !file_ids.contains(object_id.as_str()) {
                        return Err(format!(
                            "в файловом расширении «{}» нет объекта {object_id}",
                            row.name
                        ));
                    }
                    let (target, _) =
                        extension_target(&objects, class.class.as_str(), object, &row.name)
                            .map_err(|e| e.to_string())?;
                    targets.insert(object_id.to_string(), target.to_owned());
                    for child in headers.iter().filter(|h| h.id != object_id) {
                        if let Some(target) = &child.extends {
                            targets.insert(child.id.to_string(), target.to_string());
                        } else if child.belonging == Some(ObjectBelonging::Own) {
                            targets.insert(child.id.to_string(), child.id.to_string());
                        }
                    }
                }
            }
            // Формы и макеты имеют собственные заголовки ресурсов, но не
            // входят в список объектов верхнего уровня корня расширения.
            // При заимствовании по имени заголовок явно сообщает `Adopted`;
            // поиск ограничен уже проверенным владельцем и видом описания.
            for description in source
                .artifacts()
                .ok_or("нет описаний расширения")?
                .descriptions()
            {
                if targets.contains_key(description.id.as_ref())
                    || !matches!(description.kind.as_ref(), "Form" | "Template")
                {
                    continue;
                }
                let Some(owner) = description
                    .owner
                    .as_ref()
                    .and_then(|owner| targets.get(owner.as_ref()))
                else {
                    return Err(format!(
                        "нет проверенного владельца ресурса «{}»",
                        description.name
                    ));
                };
                let headers =
                    parse_extension_objects(&read(&description.id)?).map_err(|e| e.to_string())?;
                let header = headers
                    .iter()
                    .find(|header| header.id.as_str() == description.id.as_ref())
                    .ok_or("нет собственного заголовка ресурса расширения")?;
                let target = nested_target(&bundle.descriptions, description, owner, header)?;
                targets.insert(description.id.to_string(), target);
            }
            bundle.append(source, &targets, &info)?;
        }
        if !supplied.is_empty() {
            return Err("заданы каталоги неизвестных расширений".into());
        }
        bundle.properties = bundle
            .origins
            .keys()
            .map(|id| (id.clone(), std::sync::OnceLock::new()))
            .collect();
        Ok(Arc::new(bundle))
    }
}

#[cfg(test)]
#[path = "indexed_file_sources_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "indexed_file_nested_tests.rs"]
mod nested_tests;
