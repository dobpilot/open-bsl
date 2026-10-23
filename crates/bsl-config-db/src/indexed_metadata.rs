//! Подробные метаданные только для достигнутых объектов одного источника.

use super::*;
use open_sdbl::metadata::{ConfigTypeIdentity, ExtensionMetadata, Guid};
use open_sdbl_db::{ConfigurationMetadataCache, ConfigurationMetadataInputs, Limits};

pub(super) struct MetadataState {
    inputs: ConfigurationMetadataInputs,
    header_positions: HashMap<String, usize>,
    extensions: Vec<ExtensionMetadata>,
    resources: HashMap<String, ConfigResource>,
    decoded: ConfigurationMetadataCache,
    admitted: HashSet<String>,
    runtime: Option<Arc<RuntimeMetadataFacts>>,
    snapshot: Option<Arc<MetadataSnapshot>>,
    projection: Option<(Arc<RuntimeMetadataFacts>, Arc<MetadataCatalog>)>,
    // Общая таблица содержит только неизменяемые дескрипторы типов полного
    // индекса; подробные данные объектов и источники сеанса сюда не входят.
    reference_families: Option<document_attributes::ReferenceFamilies>,
    route_points: HashMap<String, Option<Vec<open_sdbl::metadata::BusinessProcessRoutePoint>>>,
}

impl std::fmt::Debug for DatabaseConfigurationSource {
    fn fmt(&self, output: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        output
            .debug_struct("DatabaseConfigurationSource")
            .field("identity", &self.index.identity())
            .field("objects", &self.headers.len())
            .finish_non_exhaustive()
    }
}

impl DatabaseConfigurationSource {
    /// Строит полный индекс ORM с адресным приобретением подробностей.
    /// Права, разделители, языки и параметры сеанса приобретаются до запуска.
    ///
    /// # Errors
    /// Возвращает ошибку неполного индекса, метаданных или соединения.
    pub fn metadata_catalog(self: &Arc<Self>) -> Result<MetadataCatalog, String> {
        let facts = self.runtime_metadata(&[])?;
        let catalog = self.project(&facts, false)?;
        Ok(catalog.with_object_source(self.clone()))
    }

    /// Возвращает общий снимок с подробностями запрошенных объектов и зависимостей.
    /// Пустой список приобретает только обязательные входы запуска.
    /// Снимок повторно используется между сеансами этой фабрики.
    ///
    /// # Errors
    /// Возвращает ошибку отсутствующего описателя или приобретения ресурсов.
    pub fn metadata_snapshot(&self, objects: &[&str]) -> Result<Arc<MetadataSnapshot>, String> {
        loop {
            let facts = self.runtime_metadata(objects)?;
            let mut state = self
                .metadata
                .lock()
                .map_err(|_| "состояние метаданных отравлено")?;
            let state = state.as_mut().ok_or("метаданные не приобретены")?;
            // Другой поток мог приобрести новые ресурсы между двумя блокировками.
            // Полный снимок публикуется только для готового текущего состава.
            if !state
                .runtime
                .as_ref()
                .is_some_and(|known| Arc::ptr_eq(known, &facts))
            {
                continue;
            }
            if let Some(snapshot) = &state.snapshot {
                return Ok(snapshot.clone());
            }
            let started = std::env::var_os("OPEN_BSL_STARTUP_TIMINGS")
                .is_some_and(|value| value == "1")
                .then(std::time::Instant::now);
            let resolved = state
                .decoded
                .finalize_runtime(&facts)
                .map_err(|error| error.to_string())?;
            metadata_timing("indexed.metadata_resolve", started);
            let snapshot = Arc::new(resolved.snapshot);
            state.snapshot = Some(snapshot.clone());
            return Ok(snapshot);
        }
    }

    fn runtime_metadata(&self, objects: &[&str]) -> Result<Arc<RuntimeMetadataFacts>, String> {
        let mut state = self
            .metadata
            .lock()
            .map_err(|_| "состояние метаданных отравлено")?;
        if state.is_none() {
            let mut connection = self
                .connection
                .lock()
                .map_err(|_| "соединение источника отравлено")?;
            let mut route_points = HashMap::new();
            for entry in self
                .headers
                .iter()
                .filter(|entry| metadata_kind(&entry.class) == Some(MetadataKind::BusinessProcess))
            {
                let id = entry.header.object_guid.as_str();
                let name = format!("{id}.7");
                if matches!(entry.location, Location::Base(_))
                    && self.resource_names.contains(&name)
                {
                    let resource =
                        read_location(connection.as_mut(), &Location::Base(name.clone()))?
                            .ok_or("нет карты маршрута")?;
                    let parsed = open_sdbl::metadata::parse_config_resource_bounded(
                        &name,
                        &resource.compressed,
                        32 * 1024 * 1024,
                    )
                    .map_err(|error| error.to_string())?;
                    route_points.insert(
                        id.to_string(),
                        parsed.route_points.map(|(_, points)| points),
                    );
                }
            }
            *state = Some(MetadataState {
                inputs: connection.metadata_inputs()?,
                header_positions: {
                    let mut positions = HashMap::new();
                    for (index, entry) in self.headers.iter().enumerate() {
                        positions
                            .entry(entry.header.object_guid.to_string())
                            .or_insert(index);
                    }
                    positions
                },
                extensions: connection.extension_metadata()?,
                resources: self
                    .roots
                    .iter()
                    .cloned()
                    .map(|resource| (resource.file_name.to_ascii_lowercase(), resource))
                    .collect(),
                decoded: ConfigurationMetadataCache::new(Limits::default()),
                admitted: HashSet::new(),
                runtime: None,
                snapshot: None,
                projection: None,
                reference_families: None,
                route_points,
            });
        }
        let state = state.as_mut().expect("состояние создано");
        let mut requested = objects
            .iter()
            .map(|id| id.to_ascii_lowercase())
            .collect::<HashSet<_>>();
        if state.runtime.is_none() {
            for entry in &self.headers {
                if matches!(
                    metadata_kind(&entry.class),
                    Some(
                        MetadataKind::Role
                            | MetadataKind::CommonAttribute
                            | MetadataKind::SessionParameter
                            | MetadataKind::Language
                    )
                ) {
                    requested.insert(entry.header.object_guid.to_string());
                }
            }
        }
        let trace = std::env::var_os("OPEN_BSL_STARTUP_TIMINGS").is_some_and(|value| value == "1");
        loop {
            let started = trace.then(std::time::Instant::now);
            let changed = self.acquire_details(state, &requested)?;
            if changed {
                metadata_timing("indexed.metadata_acquire", started);
            }
            if !changed && let Some(facts) = &state.runtime {
                return Ok(facts.clone());
            }
            state.runtime = None;
            state.snapshot = None;
            // Неизменяемый ресурс разбирается один раз. Повторное разрешение
            // добавляет связи новых объектов, не распаковывая прежние байты.
            let started = trace.then(std::time::Instant::now);
            for (name, resource) in &state.resources {
                if !state.admitted.contains(name) {
                    state
                        .decoded
                        .admit(resource)
                        .map_err(|error| error.to_string())?;
                    state.admitted.insert(name.clone());
                }
            }
            metadata_timing("indexed.metadata_admit", started);
            let started = trace.then(std::time::Instant::now);
            let resolved = state
                .decoded
                .resolve_runtime(&state.inputs, state.extensions.clone());
            metadata_timing("indexed.metadata_runtime", started);
            let resolved = match resolved {
                Ok(resolved) => resolved,
                Err(DbError::ConfigurationDescriptorsMissing { objects }) => {
                    requested = objects.iter().map(ToString::to_string).collect();
                    continue;
                }
                Err(error) => return Err(error.to_string()),
            };
            requested.clear();
            for descriptor in resolved.descriptors() {
                let dependencies = descriptor
                    .reference_types
                    .iter()
                    .chain(
                        descriptor
                            .declared_types
                            .iter()
                            .flatten()
                            .filter_map(|ty| ty.reference.as_ref()),
                    )
                    .chain(
                        descriptor
                            .separation
                            .iter()
                            .filter_map(|settings| settings.conditional_separation.as_ref()),
                    )
                    .chain(descriptor.functional_option_location_guid.iter())
                    .chain(descriptor.functional_option_parameter_use.iter().flatten())
                    .chain(
                        descriptor
                            .configuration_functional_option_parameters
                            .iter()
                            .flatten(),
                    );
                for dependency in dependencies {
                    if let Some(owner) = self.owner(dependency)
                        && !state
                            .resources
                            .contains_key(owner.header.object_guid.as_str())
                    {
                        requested.insert(owner.header.object_guid.to_string());
                    }
                }
            }
            if requested.is_empty() {
                let facts = Arc::new(resolved);
                state.runtime = Some(facts.clone());
                work_count("indexed.selected_metadata_resources", state.resources.len());
                return Ok(facts);
            }
        }
    }

    pub(crate) fn query_metadata(&self, source: &str) -> Result<Arc<MetadataSnapshot>, String> {
        use open_sdbl::query::{QueryConstantReferences, query_metadata_references};
        let references = query_metadata_references(source).map_err(|error| error.to_string())?;
        let mut selected = Vec::new();
        for reference in references.objects {
            if let Some(entry) = self.headers.iter().find(|entry| {
                query_kind(metadata_kind(&entry.class)) == Some(reference.kind)
                    && open_bsl::folded_eq(&entry.header.name, &reference.name)
            }) {
                selected.push(entry.header.object_guid.as_str());
            }
        }
        for entry in &self.headers {
            if metadata_kind(&entry.class) != Some(MetadataKind::Constant) {
                continue;
            }
            let needed = match &references.constants {
                QueryConstantReferences::All => true,
                QueryConstantReferences::Named(names) => names
                    .iter()
                    .any(|name| open_bsl::folded_eq(name, &entry.header.name)),
                _ => false,
            };
            if needed {
                selected.push(entry.header.object_guid.as_str());
            }
        }
        self.metadata_snapshot(&selected)
    }

    fn owner(&self, id: &Guid) -> Option<&ObjectHeader> {
        self.owners.get(id).map(|position| &self.headers[*position])
    }

    fn acquire_details(
        &self,
        state: &mut MetadataState,
        requested: &HashSet<String>,
    ) -> Result<bool, String> {
        let mut changed = false;
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| "соединение источника отравлено")?;
        for id in requested {
            if state.resources.contains_key(id) {
                continue;
            }
            let entry = state
                .header_positions
                .get(id)
                .map(|&index| &self.headers[index]);
            let location = match entry {
                Some(entry) => entry.location.clone(),
                None if self.resource_names.contains(id) => Location::Base(id.clone()),
                None => return Err(format!("нет описателя «{id}» в индексе ресурсов")),
            };
            let resource = read_location(connection.as_mut(), &location)?
                .ok_or_else(|| format!("нет описателя «{id}»"))?;
            let mut acquired = vec![(id.clone(), resource)];
            // Имя метода ссылается на заголовок общего модуля. Публикуем его
            // вместе с заданием; исходник и байткод модуля здесь не нужны.
            if entry.is_some_and(|entry| {
                metadata_kind(&entry.class) == Some(MetadataKind::ScheduledJob)
            }) && let Some(module_id) = scheduled_job_metadata::referenced_module(&acquired[0].1)
                .map_err(|error| error.to_string())?
                && !state.resources.contains_key(&module_id)
            {
                let module = self
                    .headers
                    .iter()
                    .find(|entry| {
                        entry.header.object_guid.as_str() == module_id
                            && metadata_kind(&entry.class) == Some(MetadataKind::CommonModule)
                    })
                    .ok_or_else(|| {
                        malformed(&module_id, "ссылка задания ведёт не на общий модуль").to_string()
                    })?;
                let resource = read_location(connection.as_mut(), &module.location)?
                    .ok_or_else(|| format!("нет описателя «{module_id}»"))?;
                acquired.push((module_id, resource));
            }
            // Эти суффиксы принадлежат данным предопределённых значений,
            // а не исходникам. Правила разбора остаются в SDK.
            let suffix = match entry.and_then(|entry| metadata_kind(&entry.class)) {
                Some(MetadataKind::Catalog) => Some("1c"),
                Some(MetadataKind::ChartOfAccounts) => Some("9"),
                Some(MetadataKind::ChartOfCharacteristicTypes | MetadataKind::BusinessProcess) => {
                    Some("7")
                }
                _ => None,
            };
            if let Some(suffix) = suffix {
                let name = format!("{id}.{suffix}");
                if matches!(location, Location::Base(_)) && self.resource_names.contains(&name) {
                    let resource =
                        read_location(connection.as_mut(), &Location::Base(name.clone()))?
                            .ok_or("нет ресурса предопределённых значений")?;
                    acquired.push((name, resource));
                }
            }
            // Описатель публикуется вместе с зависимыми данными. Ошибка чтения
            // не должна превращать повторный запрос в успешный неполный снимок.
            state.runtime = None;
            state.snapshot = None;
            state.projection = None;
            state.resources.extend(acquired);
            changed = true;
        }
        Ok(changed)
    }

    fn header_objects(
        &self,
        selected: Option<&HashMap<String, ConfigResource>>,
    ) -> Result<Vec<MetadataObject>, String> {
        self.headers
            .iter()
            .filter(|entry| {
                selected.is_none_or(|resources| {
                    resources.contains_key(entry.header.object_guid.as_str())
                })
            })
            .filter_map(|entry| metadata_kind(&entry.class).map(|kind| (entry, kind)))
            .map(|(entry, kind)| {
                let id = entry.header.object_guid.as_str();
                let mut object = entry
                    .subsystem
                    .clone()
                    .unwrap_or_else(|| MetadataObject::new(id, &entry.header.name, kind));
                if let Some(sections) = &entry.header.sections {
                    let sections = sections
                        .iter()
                        .map(|section| {
                            bsl_orm::DocumentTabularSection::new(
                                section.object_guid.as_str(),
                                section.name.as_str(),
                            )
                        })
                        .collect::<Vec<_>>();
                    object = match kind {
                        MetadataKind::Document => object.with_document_tabular_sections(sections),
                        MetadataKind::DataProcessor => {
                            object.with_data_processor_tabular_sections(sections)
                        }
                        MetadataKind::Report => object.with_report_tabular_sections(sections),
                        MetadataKind::Catalog => object.with_catalog_tabular_section_names(
                            sections.iter().map(|section| section.name().to_string()),
                        ),
                        _ => object,
                    };
                }
                Ok(self.bind_modules(object))
            })
            .collect()
    }

    fn bind_modules(&self, mut object: MetadataObject) -> MetadataObject {
        let id = object.id().to_owned();
        for module in self
            .owner_modules
            .get(&id)
            .into_iter()
            .flatten()
            .map(|position| &self.index.modules()[*position])
        {
            object = match module.kind() {
                ModuleKind::Manager => object.with_manager_module(module.name()),
                ModuleKind::Object => {
                    let mut object = object.with_object_module(module.name());
                    if module.source_is_extension() {
                        object = object.with_extension_object_module(module.name());
                    }
                    if let Some(safe) = module.own_extension_safe_mode() {
                        object = object.with_own_extension_object_module(module.name(), safe);
                    }
                    object
                }
                _ => object,
            };
        }
        object
    }

    fn project(
        &self,
        snapshot: &RuntimeMetadataFacts,
        selected_only: bool,
    ) -> Result<MetadataCatalog, String> {
        let mut state = self
            .metadata
            .lock()
            .map_err(|_| "состояние метаданных отравлено")?;
        let state = state.as_mut().ok_or("метаданные не приобретены")?;
        let headers = self.header_objects(selected_only.then_some(&state.resources))?;
        let included = headers
            .iter()
            .map(|object| object.id())
            .collect::<HashSet<_>>();
        let resources = state
            .resources
            .values()
            .filter(|resource| {
                self.resource_names
                    .contains(&resource.file_name.to_ascii_lowercase())
                    || self
                        .roots
                        .iter()
                        .any(|root| root.file_name == resource.file_name)
            })
            .cloned()
            .collect::<Vec<_>>();
        let mut extensions = Vec::new();
        for entry in self
            .headers
            .iter()
            .filter(|entry| matches!(entry.location, Location::Extension { .. }))
        {
            let Some(header) = headers
                .iter()
                .find(|header| header.id() == entry.header.object_guid.as_str())
            else {
                continue;
            };
            let mut object = header.clone();
            if state.resources.contains_key(object.id()) {
                let selected = state.resources.values().cloned().collect::<Vec<_>>();
                object = project_own_catalog_metadata(
                    object,
                    &Resources::new(&selected).map_err(|error| error.to_string())?,
                )
                .map_err(|error| error.to_string())?;
            }
            extensions.push(object);
        }
        let base_ids = self
            .headers
            .iter()
            .filter(|entry| matches!(entry.location, Location::Base(_)))
            .map(|entry| entry.header.object_guid.as_str())
            .collect::<HashSet<_>>();
        let base_headers = headers
            .iter()
            .filter(|header| base_ids.contains(header.id()))
            .cloned()
            .collect::<Vec<_>>();
        let base = load_catalog_base_with_cached_descriptors(
            &resources,
            None,
            extensions,
            Some(snapshot),
            Some(&base_headers),
            selected_only,
            Some(&state.decoded),
        )
        .map_err(|error| error.to_string())?;
        let mut catalog =
            project_catalog_values_impl(base, snapshot, true, state.reference_families.as_ref())
                .map_err(|error| error.to_string())?;
        let values = self
            .headers
            .iter()
            .filter(|entry| included.contains(entry.header.object_guid.as_str()))
            .flat_map(|entry| {
                entry
                    .header
                    .enumeration_values
                    .iter()
                    .flatten()
                    .map(|value| {
                        let mut value_out = ConfigurationEnumValue::new(
                            entry.header.object_guid.as_str(),
                            value.object_guid.as_str(),
                            &value.name,
                        );
                        for synonym in &value.synonyms {
                            value_out = value_out.with_synonym(&synonym.language, &synonym.text);
                        }
                        value_out
                    })
            })
            .collect();
        catalog = catalog
            .with_enumeration_values(values)
            .map_err(|error| error.to_string())?;
        for entry in &self.headers {
            let id = entry.header.object_guid.as_str();
            if !included.contains(id) {
                continue;
            }
            if metadata_kind(&entry.class) == Some(MetadataKind::BusinessProcess) {
                let reference_type = match &entry.header.route_point_reference_type {
                    ConfigTypeIdentity::Known(id) => Some(id.as_str()),
                    _ => None,
                };
                catalog = project_route_points(
                    catalog,
                    id,
                    reference_type,
                    state
                        .route_points
                        .get(id)
                        .and_then(|points| points.as_deref()),
                    &entry.header.synonyms,
                )
                .map_err(|error| error.to_string())?;
            }
            if metadata_kind(&entry.class) == Some(MetadataKind::BusinessProcess)
                && !state.resources.contains_key(id)
                && let Some(sections) = &entry.header.sections
            {
                catalog = catalog
                    .with_business_process_metadata(
                        id,
                        None,
                        None,
                        Some(
                            sections
                                .iter()
                                .map(|section| {
                                    bsl_orm::DocumentTabularSection::new(
                                        section.object_guid.as_str(),
                                        section.name.as_str(),
                                    )
                                })
                                .collect(),
                        ),
                    )
                    .map_err(|error| error.to_string())?;
            }
        }
        if self.extensions.len() != self.extension_count {
            return Err("неполные сведения расширений".into());
        }
        let applied: Vec<_> = self
            .extensions
            .iter()
            .filter(|(_, active)| *active)
            .map(|(entry, _)| *entry.id())
            .collect();
        let disabled: Vec<_> = self
            .extensions
            .iter()
            .filter(|(_, active)| !*active)
            .map(|(entry, _)| *entry.id())
            .collect();
        catalog = catalog.with_configuration_extensions(
            ConfigurationExtensionsSnapshot::new(
                self.extensions
                    .iter()
                    .map(|(entry, _)| entry.clone())
                    .collect(),
                applied,
                disabled,
                Vec::new(),
            )
            .map_err(|error| error.to_string())?,
        );
        if !selected_only && state.reference_families.is_none() {
            state.reference_families = Some(document_attributes::reference_families(&catalog));
        }
        Ok(catalog)
    }

    #[cfg(test)]
    pub(super) fn projected_catalog(
        &self,
        snapshot: &Arc<MetadataSnapshot>,
    ) -> Result<Arc<MetadataCatalog>, String> {
        let facts = {
            let state = self
                .metadata
                .lock()
                .map_err(|_| "состояние метаданных отравлено")?;
            state.as_ref().and_then(|state| {
                state
                    .snapshot
                    .as_ref()
                    .filter(|known| Arc::ptr_eq(known, snapshot))
                    .and(state.runtime.clone())
            })
        };
        match facts {
            Some(facts) => self.projected_runtime_catalog(&facts),
            None => self.project(snapshot.runtime_facts(), true).map(Arc::new),
        }
    }

    fn projected_runtime_catalog(
        &self,
        snapshot: &Arc<RuntimeMetadataFacts>,
    ) -> Result<Arc<MetadataCatalog>, String> {
        {
            let state = self
                .metadata
                .lock()
                .map_err(|_| "состояние метаданных отравлено")?;
            if let Some((known, catalog)) =
                state.as_ref().and_then(|state| state.projection.as_ref())
                && Arc::ptr_eq(known, snapshot)
            {
                return Ok(catalog.clone());
            }
        }
        let catalog = Arc::new(self.project(snapshot, true)?);
        let mut state = self
            .metadata
            .lock()
            .map_err(|_| "состояние метаданных отравлено")?;
        let state = state.as_mut().ok_or("метаданные не приобретены")?;
        // Публикация другого снимка могла произойти во время проекции.
        // Такой каталог подходит вызывающему, но не заменяет текущий кэш.
        if state
            .runtime
            .as_ref()
            .is_some_and(|known| Arc::ptr_eq(known, snapshot))
        {
            state.projection = Some((snapshot.clone(), catalog.clone()));
        }
        Ok(catalog)
    }
}

impl bsl_orm::MetadataObjectSource for DatabaseConfigurationSource {
    fn prepare_objects(&self, ids: &[&str]) -> Result<(), String> {
        if !ids.is_empty() {
            self.runtime_metadata(ids)?;
        }
        Ok(())
    }

    fn load_object(&self, id: &str) -> Result<MetadataObject, String> {
        if let Some(object) = self
            .headers
            .iter()
            .find(|entry| entry.header.object_guid.as_str() == id)
            .and_then(|entry| entry.subsystem.as_ref())
        {
            return Ok(object.clone());
        }
        let snapshot = self.runtime_metadata(&[id])?;
        if self.headers.iter().any(|entry| {
            entry.header.object_guid.as_str() == id
                && metadata_kind(&entry.class) == Some(MetadataKind::CommonAttribute)
        }) {
            return self
                .project(&snapshot, false)?
                .object_by_id(id)
                .cloned()
                .ok_or_else(|| format!("объект «{id}» отсутствует в каталоге"));
        }
        let catalog = self.projected_runtime_catalog(&snapshot)?;
        catalog
            .object_by_id(id)
            .cloned()
            .map(|object| self.bind_modules(object))
            .ok_or_else(|| format!("объект «{id}» отсутствует в каталоге"))
    }
}

fn query_kind(kind: Option<MetadataKind>) -> Option<open_sdbl::metadata::MetadataKind> {
    use open_sdbl::metadata::MetadataKind as Target;
    Some(match kind? {
        MetadataKind::Catalog => Target::Catalog,
        MetadataKind::Document => Target::Document,
        MetadataKind::DocumentJournal => Target::DocumentJournal,
        MetadataKind::Enumeration => Target::Enumeration,
        MetadataKind::InformationRegister => Target::InformationRegister,
        MetadataKind::AccumulationRegister => Target::AccumulationRegister,
        MetadataKind::AccountingRegister => Target::AccountingRegister,
        MetadataKind::CalculationRegister => Target::CalculationRegister,
        MetadataKind::ChartOfCharacteristicTypes => Target::ChartOfCharacteristicTypes,
        MetadataKind::ChartOfCalculationTypes => Target::ChartOfCalculationTypes,
        MetadataKind::ChartOfAccounts => Target::ChartOfAccounts,
        MetadataKind::Constant => Target::Constant,
        MetadataKind::ExchangePlan => Target::ExchangePlan,
        MetadataKind::BusinessProcess => Target::BusinessProcess,
        MetadataKind::Task => Target::Task,
        MetadataKind::Sequence => Target::Sequence,
        _ => return None,
    })
}

fn metadata_timing(stage: &str, started: Option<std::time::Instant>) {
    if let Some(started) = started {
        eprintln!(
            "open-bsl.startup\t{stage}\t{:.6}",
            started.elapsed().as_secs_f64()
        );
    }
}

#[cfg(test)]
#[path = "runtime_metadata_readiness_tests.rs"]
mod runtime_readiness_tests;
