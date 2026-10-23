//! Загрузчик исходного кода конфигурации из информационной базы.
//!
//! SQL, транзакция, сборка частей и лимиты принадлежат `open-sdbl-db`.
//! Этот крейт принимает готовое приобретение и строит нейтральный снимок.

#![warn(missing_docs)]

mod data_access;
mod indexed;
pub use indexed::DatabaseConfigurationSource;
mod document_attributes;
mod functional_option_reads;
mod functional_options;

#[cfg(test)]
#[path = "route_points_tests.rs"]
mod route_points_tests;

#[cfg(test)]
#[path = "exchange_plan_class_tests.rs"]
mod exchange_plan_class_tests;

mod native_image;
mod scsu;
mod user_separation;

pub use data_access::DatabaseDataAccess;
mod database_session;
pub use database_session::{DatabaseSessionOptions, open_database_session};

use std::{collections::HashMap, error::Error, fmt};

use bsl_config::{
    ConfigurationCodeSnapshot, ConfigurationModule, ModuleExecution, ModuleKind, SnapshotError,
};
use bsl_orm::{
    CatalogCodeType, CatalogDefaultPresentation, CatalogError, CatalogStandardAttributes,
    CompatibilityMode, ConfigurationEnumValue, ConfigurationExtension, ConfigurationExtensionScope,
    ConfigurationExtensionsSnapshot, ExtensionSafeMode, MetadataCatalog, MetadataKind,
    MetadataObject, Principal, ScriptVariant,
};
use bsl_session::PrincipalCandidate;
use open_sdbl::metadata::{
    ExtensionCatalog, InfoBaseUser, MetadataSnapshot, ObjectBelonging, RuntimeMetadataFacts, Value,
    inflate_raw_deflate, parse_config_descriptor, parse_config_descriptors, parse_serialized,
};
use open_sdbl_db::{
    AcquiredConfiguration, AcquiredExtension, AcquiredExtensionObject, ConfigResource,
    ConfigSelector, DbError, ExtensionScope,
};

const SUBSYSTEM: &str = "37f2fa9a-b276-11d4-9435-004095e12fc7";

/// Режим чтения защищённых модулей одного снимка конфигурации.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LoadOptions {
    /// Восстанавливает тела, закрытые методы и инициализацию из нативного образа.
    ///
    /// По умолчанию выдаются только экспортные сигнатуры: процедуры пустые,
    /// функции возвращают `Истина`. Это заглушки, а не оригинальная реализация.
    /// Заглушки асинхронных экспортов также синхронны и не возвращают обещаний.
    /// Непрозрачные параметры без таблицы умолчаний допускаются как
    /// обязательные параметры заглушки; их платформенные флаги не трактуются.
    /// Модули с доступным исходным текстом не меняются.
    pub decompile_protected_modules: bool,
}

const COMMON: &str = "0fe48980-252d-11d6-a3c7-0050bae0a776";
const CONFIGURATION: &str = "9cd510cd-abfc-11d4-9434-004095e12fc7";
const ROLE: &str = "09736b02-9cac-4e3f-b4f7-d3e9576ab948";
const SESSION_PARAMETER: &str = "24c43748-c938-45d0-8d14-01424a72b11e";
const FUNCTIONAL_OPTION: &str = "af547940-3268-434f-a3e7-e47d6d2638c3";
const STYLE_ITEM: &str = "58848766-36ea-4076-8800-e91eb49590d7";
const SCHEDULED_JOB: &str = "11bdaf85-d5ad-4d91-bb24-aa0eee139052";
const ENUMERATION: &str = "f6a80749-5ad7-400b-8519-39dc5dff2542";
const COMMON_PICTURE: &str = "7dcd43d9-aca5-4926-b549-1842e6a4e8cf";
const COMMON_ATTRIBUTE: &str = "15794563-ccec-41f6-a83c-ec5f7b9a5bc1";
const LANGUAGE: &str = "9cd510ce-abfc-11d4-9434-004095e12fc7";

/// Преобразует записи `v8users` в кандидатов для выбора пользователя сеанса.
///
/// Идентификатор, оба имени и роли копируются в неизменяемый снимок principal.
/// Допустимость входа остаётся решением [`InfoBaseUser::can_authenticate`].
#[must_use]
pub fn principal_candidates(users: &[InfoBaseUser]) -> Vec<PrincipalCandidate> {
    users
        .iter()
        .map(|user| {
            let principal = Principal::new(
                user.data.id.as_str(),
                user.name.clone(),
                user.data.roles.iter().map(|role| role.as_str().to_owned()),
            )
            // Измерено на 8.3.27.2342: полное имя из `Data` совпадает с
            // `ПолноеИмяПользователя()` в трёх именованных тестовых сеансах.
            .with_full_name(user.data.full_name.clone());
            PrincipalCandidate::new(
                user_separation::attach(principal, user.data.data_separation.as_deref()),
                user.can_authenticate(),
            )
        })
        .collect()
}

/// Строит снимок исходников из одного результата приобретения.
///
/// Защищённые модули по умолчанию содержат только экспортные заглушки,
/// описанные в [`LoadOptions`]. Для восстановления тел нужен [`load_with_options`].
///
/// # Errors
///
/// Возвращает ошибку при активном расширении, неизвестной его активности,
/// повреждённом или отсутствующем ресурсе, неподдерживаемой кодировке и
/// некорректном итоговом графе.
pub fn load(acquired: &AcquiredConfiguration) -> Result<ConfigurationCodeSnapshot, LoadError> {
    load_with_options(acquired, LoadOptions::default())
}

/// Строит снимок с выбранным режимом чтения защищённых модулей.
///
/// # Errors
/// Возвращает ошибки [`load`]; полная декомпиляция также отклоняет
/// неподдерживаемые тела без подмены заглушками.
pub fn load_with_options(
    acquired: &AcquiredConfiguration,
    options: LoadOptions,
) -> Result<ConfigurationCodeSnapshot, LoadError> {
    let base = load_base_graph(&acquired.config_resources, options)?;
    let identity = base.snapshot.identity().to_owned();
    let mut modules = base.snapshot.modules().to_vec();
    for extension in ordered_active_extensions(&acquired.extensions)? {
        apply_structural_extension(&mut modules, &base.objects, extension, options)?;
    }
    ConfigurationCodeSnapshot::new(identity, modules).map_err(LoadError::Snapshot)
}

/// Возвращает только применяемые расширения в платформенном порядке.
/// Неизвестную активность нельзя трактовать как отключение: иначе снимок
/// мог бы выдать базовый код вместо реально исполняемого платформой.
fn ordered_active_extensions(
    extensions: &[AcquiredExtension],
) -> Result<Vec<&AcquiredExtension>, LoadError> {
    let positions =
        ordered_active_positions(extensions.iter().enumerate().map(|(index, extension)| {
            (
                index,
                extension.name.as_str(),
                extension.active,
                extension.order,
            )
        }))?;
    Ok(positions
        .into_iter()
        .map(|index| &extensions[index])
        .collect())
}

/// Выбирает позиции активных расширений и сортирует их по порядку применения.
/// Отдельная функция сохраняет проверяемым само правило, не создавая в тестах
/// `#[non_exhaustive]` значение внешнего крейта.
fn ordered_active_positions<'a>(
    extensions: impl Iterator<Item = (usize, &'a str, Option<bool>, u32)>,
) -> Result<Vec<usize>, LoadError> {
    let mut active = Vec::new();
    for (index, name, enabled, order) in extensions {
        let Some(enabled) = enabled else {
            return Err(LoadError::UnsupportedExtension {
                name: name.into(),
                activity_known: false,
            });
        };
        if enabled {
            active.push((index, order));
        }
    }
    active.sort_by_key(|(_, order)| *order);
    Ok(active.iter().map(|(index, _)| *index).collect())
}

/// Строит снимок кода и ORM-каталог для одной фабрики сеансов.
///
/// Защищённые модули используют режим по умолчанию из [`LoadOptions`].
///
/// Код разбирается один раз. Каталог использует этот же снимок, чтобы
/// связать модули менеджеров и не удерживать вторую копию большой
/// конфигурации во время подготовки фабрики.
///
/// # Errors
///
/// Возвращает те же ошибки, что [`load`] и [`load_catalog`].
pub fn load_factory_inputs(
    acquired: AcquiredConfiguration,
) -> Result<(ConfigurationCodeSnapshot, MetadataCatalog), LoadError> {
    load_factory_inputs_with_options(acquired, LoadOptions::default())
}

/// Строит фабричные входы с выбранным режимом защищённых модулей.
///
/// # Errors
/// Возвращает ошибки [`load_with_options`] и [`load_catalog`].
pub fn load_factory_inputs_with_options(
    acquired: AcquiredConfiguration,
    options: LoadOptions,
) -> Result<(ConfigurationCodeSnapshot, MetadataCatalog), LoadError> {
    let (code, catalog, _) = load_factory_inputs_with_metadata_and_options(acquired, options)?;
    Ok((code, catalog))
}

/// Строит фабричные входы и передаёт снимок SDBL адаптеру текущего сеанса.
///
/// Защищённые модули используют режим по умолчанию из [`LoadOptions`].
///
/// Снимок перемещается из результата приобретения без второй копии и без
/// повторного чтения конфигурации.
///
/// # Errors
///
/// Возвращает те же ошибки, что [`load_factory_inputs`].
pub fn load_factory_inputs_with_metadata(
    acquired: AcquiredConfiguration,
) -> Result<(ConfigurationCodeSnapshot, MetadataCatalog, MetadataSnapshot), LoadError> {
    load_factory_inputs_with_metadata_and_options(acquired, LoadOptions::default())
}

/// Строит фабричные входы и снимок SDBL с выбранным режимом защищённых модулей.
///
/// # Errors
/// Возвращает ошибки [`load_factory_inputs_with_options`].
pub fn load_factory_inputs_with_metadata_and_options(
    mut acquired: AcquiredConfiguration,
    options: LoadOptions,
) -> Result<(ConfigurationCodeSnapshot, MetadataCatalog, MetadataSnapshot), LoadError> {
    retain_factory_resources(&mut acquired.config_resources)?;
    let code = load_with_options(&acquired, options)?;
    let catalog = load_catalog_for_code(&acquired, &code)?;
    Ok((code, catalog, acquired.metadata))
}

/// Оставляет в потребляемом снимке только ресурсы, из которых строятся
/// код и ORM-каталог. Формы, макеты и прочие большие ресурсы ни один
/// из двух результатов не читает, поэтому после разбора корневого каталога
/// их сжатые байты можно освободить до декодирования исходников.
fn retain_factory_resources(resources: &mut Vec<ConfigResource>) -> Result<(), LoadError> {
    let retained = {
        let resources = Resources::new(resources)?;
        factory_resource_names(&resources)?
    };
    resources.retain(|resource| retain_factory_resource(&retained, &resource.file_name));
    Ok(())
}

fn retain_factory_resource(retained: &std::collections::HashSet<String>, name: &str) -> bool {
    retained.contains(&name.to_ascii_lowercase()) || guid(name)
}

fn factory_resource_names(
    resources: &Resources<'_>,
) -> Result<std::collections::HashSet<String>, LoadError> {
    let root =
        parse_serialized(&resources.decoded("root")?).map_err(|error| malformed("root", error))?;
    let root_resource = first_guid(&root)
        .ok_or_else(|| malformed("root", "нет GUID конфигурации"))?
        .to_ascii_lowercase();
    let root_value = parse_serialized(&resources.decoded(&root_resource)?)
        .map_err(|error| malformed(&root_resource, error))?;
    let identity = configuration_identity(&root_value)
        .ok_or_else(|| malformed(&root_resource, "нет GUID объекта конфигурации"))?;
    let mut retained = std::collections::HashSet::from([
        "root".to_string(),
        root_resource,
        format!("{identity}.7").to_ascii_lowercase(),
    ]);
    for include in
        collections(&root_value).ok_or_else(|| malformed("root", "нет каталога объектов"))?
    {
        let count = scalar(include.get(2).expect("проверено при разборе"))
            .and_then(|value| value.parse::<usize>().ok())
            .expect("проверено при разборе");
        for collection in include.iter().skip(3).take(count) {
            let Some((class, ids)) = collection_ids(collection) else {
                continue;
            };
            for id in ids {
                retained.insert(id.to_ascii_lowercase());
                if class.eq_ignore_ascii_case(COMMON) {
                    retained.insert(format!("{id}.0").to_ascii_lowercase());
                } else {
                    retained.extend(
                        module_slots(class)
                            .into_iter()
                            .map(|slot| format!("{id}.{}", slot.suffix).to_ascii_lowercase()),
                    );
                }
            }
        }
    }
    Ok(retained)
}

/// Выбирает ресурсы `Config`, необходимые для исходников и ORM-каталога.
/// После чтения корня использует то же правило, что и [`load_factory_inputs`].
#[derive(Default)]
pub struct FactoryConfigSelector {
    root: Option<ConfigResource>,
    retained: std::collections::HashSet<String>,
}

impl ConfigSelector for FactoryConfigSelector {
    fn root_resource_name(&mut self, root: &ConfigResource) -> Result<String, DbError> {
        let roots = Resources::new(std::slice::from_ref(root))
            .map_err(|error| DbError::Data(error.to_string()))?;
        let root_bytes = roots
            .decoded("root")
            .map_err(|error| DbError::Data(error.to_string()))?;
        let value =
            parse_serialized(&root_bytes).map_err(|error| DbError::Data(error.to_string()))?;
        let name = first_guid(&value)
            .ok_or_else(|| DbError::Data("корень Config не содержит GUID конфигурации".into()))?
            .to_ascii_lowercase();
        self.root = Some(root.clone());
        Ok(name)
    }

    fn configure(&mut self, root_descriptor: &ConfigResource) -> Result<(), DbError> {
        let root = self
            .root
            .take()
            .ok_or_else(|| DbError::Data("описатель конфигурации пришёл до корня Config".into()))?;
        let bootstrap = [root, root_descriptor.clone()];
        let resources =
            Resources::new(&bootstrap).map_err(|error| DbError::Data(error.to_string()))?;
        self.retained =
            factory_resource_names(&resources).map_err(|error| DbError::Data(error.to_string()))?;
        Ok(())
    }

    fn retain(&mut self, name: &str) -> bool {
        retain_factory_resource(&self.retained, name)
    }
}

/// Строит неизменяемый ORM-каталог из того же приобретения конфигурации.
///
/// # Errors
///
/// Возвращает ошибку при повреждённом корне, неизвестном режиме свойств,
/// неполных данных расширения или нарушении целостности каталога.
pub fn load_catalog(acquired: &AcquiredConfiguration) -> Result<MetadataCatalog, LoadError> {
    let code = load(acquired)?;
    load_catalog_for_code(acquired, &code)
}

fn load_catalog_for_code(
    acquired: &AcquiredConfiguration,
    code: &ConfigurationCodeSnapshot,
) -> Result<MetadataCatalog, LoadError> {
    let extension_objects = project_extension_metadata_objects(acquired, code)?;
    let catalog = load_catalog_base_with_metadata(
        &acquired.config_resources,
        code,
        extension_objects,
        Some(&acquired.metadata),
    )?;
    let extensions = project_configuration_extensions(&acquired.extensions)?;
    let catalog = catalog.with_configuration_extensions(extensions);
    project_catalog_values(catalog, &acquired.metadata)
}

// Та же проекция используется для полного приобретения и нативных фикстур.
fn project_catalog_values(
    catalog: MetadataCatalog,
    metadata: &MetadataSnapshot,
) -> Result<MetadataCatalog, LoadError> {
    project_catalog_values_impl(catalog, metadata.runtime_facts(), false, None)
}

fn project_catalog_values_impl(
    catalog: MetadataCatalog,
    metadata: &RuntimeMetadataFacts,
    selective: bool,
    reference_families: Option<&document_attributes::ReferenceFamilies>,
) -> Result<MetadataCatalog, LoadError> {
    // Индекс строится один раз для снимка: для каждого значения не обходится
    // весь набор описателей конфигурации.
    let enumeration_descriptors: HashMap<_, _> = metadata
        .descriptors()
        .iter()
        .filter(|descriptor| descriptor.enumeration_value)
        .map(|descriptor| {
            (
                (
                    descriptor.resource_guid.as_str(),
                    descriptor.object_guid.as_str(),
                ),
                descriptor,
            )
        })
        .collect();
    let values = metadata
        .values()
        .iter()
        .filter(|value| {
            metadata.object_by_id(value.owner).is_some_and(|owner| {
                owner.kind == Some(open_sdbl::metadata::MetadataKind::Enumeration)
            })
        })
        .map(|value| {
            let owner_id = value.owner.to_string();
            let mut projected = ConfigurationEnumValue::new(
                owner_id.clone(),
                value.guid.as_str(),
                value.name.clone(),
            );
            if let Some(descriptor) =
                enumeration_descriptors.get(&(owner_id.as_str(), value.guid.as_str()))
            {
                for synonym in &descriptor.synonyms {
                    projected = projected.with_synonym(&synonym.language, &synonym.text);
                }
            }
            projected
        })
        .collect();
    let catalog = catalog
        .with_enumeration_values(values)
        .map_err(LoadError::Catalog)?;
    let mut catalog = catalog;
    for descriptor in metadata.descriptors() {
        if catalog
            .object_by_id(descriptor.object_guid.as_str())
            .is_some_and(|object| object.kind() == MetadataKind::BusinessProcess)
        {
            catalog = project_route_points(
                catalog,
                descriptor.object_guid.as_str(),
                descriptor
                    .business_process_route_point_reference_type
                    .as_ref()
                    .map(|id| id.as_str()),
                descriptor.business_process_route_points.as_deref(),
                &descriptor.synonyms,
            )?;
        }
    }
    let catalog = document_attributes::project_runtime_with_references(
        catalog,
        metadata,
        selective,
        reference_families,
    )?;
    functional_options::project_runtime(catalog, metadata)
}

fn project_route_points(
    catalog: MetadataCatalog,
    owner: &str,
    reference_type: Option<&str>,
    points: Option<&[open_sdbl::metadata::BusinessProcessRoutePoint]>,
    synonyms: &[open_sdbl::metadata::Synonym],
) -> Result<MetadataCatalog, LoadError> {
    let points = points.map(|points| {
        points
            .iter()
            .map(|point| bsl_orm::BusinessProcessRoutePoint {
                owner_id: point.owner_guid.as_str().to_owned(),
                point_id: point.point_guid.as_str().to_owned(),
                name: point.name.clone(),
                synonyms: point
                    .synonyms
                    .iter()
                    .map(|synonym| (synonym.language.clone(), synonym.text.clone()))
                    .collect(),
            })
            .collect()
    });
    catalog
        .with_business_process_route_points(
            owner,
            reference_type,
            points,
            &synonyms
                .iter()
                .map(|synonym| (synonym.language.clone(), synonym.text.clone()))
                .collect::<Vec<_>>(),
        )
        .map_err(LoadError::Catalog)
}

fn project_configuration_extensions(
    extensions: &[AcquiredExtension],
) -> Result<ConfigurationExtensionsSnapshot, LoadError> {
    let mut ordered = extensions.iter().collect::<Vec<_>>();
    ordered.sort_by_key(|extension| extension.order);
    let mut database = Vec::with_capacity(ordered.len());
    let mut applied = Vec::new();
    let mut disabled = Vec::new();
    for extension in ordered {
        let description = project_extension_description(
            &extension.identity,
            &extension.name,
            extension.active,
            extension.runtime_version.as_deref(),
            extension.hash_sum.as_deref(),
            extension.safe_mode,
            extension.security_profile_name.as_deref(),
            extension.scope_kind,
            extension.used_in_distributed_infobase,
        )?;
        let description = project_extension_purpose(description, extension.purpose);
        let identity = *description.id();
        database.push(description);
        if extension.active.expect("активность проверена при проекции")
        {
            applied.push(identity);
        } else {
            disabled.push(identity);
        }
    }
    ConfigurationExtensionsSnapshot::new(database, applied, disabled, Vec::new())
        .map_err(LoadError::Catalog)
}

// Преобразование измеренных полей отделено от ресурсов расширения:
// его проверка не требует повторно строить весь каталог конфигурации.
#[allow(clippy::too_many_arguments)]
fn project_extension_description(
    identity: &str,
    name: &str,
    active: Option<bool>,
    version: Option<&str>,
    hash_sum: Option<&str>,
    safe_mode: Option<bool>,
    security_profile: Option<&str>,
    scope: Option<ExtensionScope>,
    distributed: Option<bool>,
) -> Result<ConfigurationExtension, LoadError> {
    let incomplete = || LoadError::IncompleteExtensionMetadata(name.to_owned());
    let identity = extension_identity(identity).ok_or_else(incomplete)?;
    let active = active.ok_or_else(incomplete)?;
    let version = version.ok_or_else(incomplete)?;
    let hash_sum = hash_sum.ok_or_else(incomplete)?;
    let (safe_mode, scope) =
        extension_settings(safe_mode, security_profile, scope).ok_or_else(incomplete)?;
    let distributed = distributed.ok_or_else(incomplete)?;
    ConfigurationExtension::new(
        identity,
        name,
        version,
        hash_sum,
        safe_mode,
        distributed,
        active,
        scope,
    )
    .map_err(LoadError::Catalog)
}

fn project_extension_purpose(
    description: ConfigurationExtension,
    purpose: Option<open_sdbl::metadata::ConfigurationExtensionPurpose>,
) -> ConfigurationExtension {
    use bsl_orm::ConfigurationExtensionPurpose as HostPurpose;
    use open_sdbl::metadata::ConfigurationExtensionPurpose as AcquiredPurpose;
    // Неизвестное назначение остаётся неизвестным: имя и остальные флаги
    // расширения не заменяют приобретённое свойство его конфигурации.
    let purpose = match purpose {
        Some(AcquiredPurpose::Patch) => HostPurpose::Patch,
        Some(AcquiredPurpose::Customization) => HostPurpose::Customization,
        Some(AcquiredPurpose::AddOn) => HostPurpose::AddOn,
        _ => return description,
    };
    description.with_purpose(purpose)
}

#[cfg(test)]
#[path = "extension_purpose_projection_tests.rs"]
mod extension_purpose_projection_tests;

fn extension_settings(
    safe: Option<bool>,
    profile: Option<&str>,
    scope: Option<ExtensionScope>,
) -> Option<(ExtensionSafeMode, ConfigurationExtensionScope)> {
    let safe = safe?;
    let profile = profile?;
    let scope = match scope? {
        ExtensionScope::InfoBase => ConfigurationExtensionScope::InfoBase,
        _ => return None,
    };
    // Пустое имя означает измеренное отсутствие профиля. Неизвестное имя
    // не заменяется булевым режимом: такой снимок нельзя считать полным.
    let safe_mode = if profile.is_empty() {
        ExtensionSafeMode::Enabled(safe)
    } else {
        ExtensionSafeMode::profile(profile)
    };
    Some((safe_mode, scope))
}

fn extension_identity(identity: &str) -> Option<[u8; 16]> {
    canonical_uuid_from_physical_hex(identity)
}

// Общая запись физического UUID используется расширениями и внутренними ссылками.
fn canonical_uuid_from_physical_hex(identity: &str) -> Option<[u8; 16]> {
    if identity.len() != 32 || !identity.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    let mut physical = [0; 16];
    for (index, byte) in physical.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&identity[index * 2..index * 2 + 2], 16).ok()?;
    }
    // `identity` сохраняет физический `_IDRRef`. Платформенное свойство
    // расширения содержит UUID в каноническом порядке полей; перестановка
    // подтверждена отдельным замером расширения на 1С 8.3.27.2342.
    Some(canonical_uuid_from_physical(physical))
}

// Порядок полей UUID в физических ссылках 1С противоположен порядку
// канонической записи. Это обратная операция `Guid::to_1c_bytes`.
fn canonical_uuid_from_physical(physical: [u8; 16]) -> [u8; 16] {
    open_bsl::uuid::from_1c_bytes(physical)
}

#[cfg(test)]
fn load_catalog_base(
    source: &[ConfigResource],
    code: &ConfigurationCodeSnapshot,
    extension_objects: Vec<MetadataObject>,
) -> Result<MetadataCatalog, LoadError> {
    load_catalog_base_with_metadata(source, code, extension_objects, None)
}

#[cfg(test)]
fn catalog_predefined_names(metadata: Option<&MetadataSnapshot>) -> HashMap<String, Vec<String>> {
    catalog_predefined_runtime_names(metadata.map(MetadataSnapshot::runtime_facts))
}

fn catalog_predefined_runtime_names(
    metadata: Option<&RuntimeMetadataFacts>,
) -> std::collections::HashMap<String, Vec<String>> {
    // Снимок уже декодировал `.1c` при приобретении. Повторно читать
    // ресурсы ради имён или хранить их до создания сеанса не требуется.
    let mut predefined = std::collections::HashMap::<String, Vec<String>>::new();
    for (metadata, value) in metadata
        .into_iter()
        .flat_map(|metadata| metadata.values().iter().map(move |value| (metadata, value)))
    {
        if let Some(owner) = metadata.object_by_id(value.owner)
            && matches!(
                owner.kind,
                Some(
                    open_sdbl::metadata::MetadataKind::Catalog
                        | open_sdbl::metadata::MetadataKind::ChartOfCharacteristicTypes
                        | open_sdbl::metadata::MetadataKind::ChartOfAccounts
                )
            )
        {
            predefined
                .entry(owner.guid.as_str().to_ascii_lowercase())
                .or_default()
                .push(value.name.clone());
        }
    }
    predefined
}

fn load_catalog_base_with_metadata(
    source: &[ConfigResource],
    code: &ConfigurationCodeSnapshot,
    extension_objects: Vec<MetadataObject>,
    metadata: Option<&MetadataSnapshot>,
) -> Result<MetadataCatalog, LoadError> {
    load_catalog_base_projected(source, Some(code), extension_objects, metadata, None, false)
}

fn load_catalog_base_projected(
    source: &[ConfigResource],
    code: Option<&ConfigurationCodeSnapshot>,
    extension_objects: Vec<MetadataObject>,
    metadata: Option<&MetadataSnapshot>,
    discovery: Option<&[MetadataObject]>,
    selected_only: bool,
) -> Result<MetadataCatalog, LoadError> {
    load_catalog_base_with_cached_descriptors(
        source,
        code,
        extension_objects,
        metadata.map(MetadataSnapshot::runtime_facts),
        discovery,
        selected_only,
        None,
    )
}

// Кэш принадлежит адресному источнику и содержит факты тех же неизменяемых
// ресурсов. Внешние загрузчики сохраняют самостоятельную проверку исходных байтов.
fn load_catalog_base_with_cached_descriptors(
    source: &[ConfigResource],
    code: Option<&ConfigurationCodeSnapshot>,
    extension_objects: Vec<MetadataObject>,
    metadata: Option<&RuntimeMetadataFacts>,
    discovery: Option<&[MetadataObject]>,
    selected_only: bool,
    cache: Option<&open_sdbl_db::ConfigurationMetadataCache>,
) -> Result<MetadataCatalog, LoadError> {
    let discovery_index = discovery
        .into_iter()
        .flatten()
        .map(|object| (object.id().to_ascii_lowercase(), object))
        .collect::<HashMap<_, _>>();
    let storage_numbers: std::collections::HashMap<_, _> = metadata
        .into_iter()
        .flat_map(RuntimeMetadataFacts::objects)
        .filter_map(|object| {
            object
                .number
                .map(|number| (object.guid.as_str().to_ascii_lowercase(), number))
        })
        .collect();
    let mut predefined = catalog_predefined_runtime_names(metadata);
    let resources = Resources::new(source)?;
    let root = parse_serialized(&resources.decoded("root")?).map_err(|e| malformed("root", e))?;
    let root_resource = first_guid(&root)
        .ok_or_else(|| malformed("root", "нет GUID конфигурации"))?
        .to_ascii_lowercase();
    let root_value = parse_serialized(&resources.decoded(&root_resource)?)
        .map_err(|e| malformed(&root_resource, e))?;
    let identity = configuration_identity(&root_value)
        .ok_or_else(|| malformed(&root_resource, "нет GUID объекта конфигурации"))?
        .to_ascii_lowercase();
    let properties = configuration_properties(&root_value)
        .ok_or_else(|| malformed(&root_resource, "нет свойств конфигурации"))?;
    let configuration_name = configuration_name(properties)
        .ok_or_else(|| malformed(&root_resource, "нет имени конфигурации"))?;

    let mut objects = Vec::new();
    let mut language_codes = std::collections::HashMap::new();
    let mut seen_subsystems = std::collections::HashSet::new();
    let mut common_attribute_ids = None;
    for include in collections(&root_value)
        .ok_or_else(|| malformed(&root_resource, "нет каталога объектов"))?
    {
        let count = scalar(include.get(2).expect("проверено при разборе"))
            .and_then(|value| value.parse::<usize>().ok())
            .expect("проверено при разборе");
        for collection in include.iter().skip(3).take(count) {
            let common_collection = list(collection)
                .and_then(|values| values.first())
                .and_then(scalar)
                .is_some_and(|class| class.eq_ignore_ascii_case(COMMON_ATTRIBUTE));
            let Some((class, ids)) = collection_ids(collection) else {
                if common_collection {
                    return Err(malformed(
                        &root_resource,
                        "неполная коллекция общих реквизитов",
                    ));
                }
                continue;
            };
            // Корневая коллекция независимо сверена с XML demo/buh/unf и
            // нативной separator-базой. Не фильтруем неизвестные описатели.
            if common_collection {
                if common_attribute_ids.is_some() {
                    return Err(malformed(
                        &root_resource,
                        "повторная коллекция общих реквизитов",
                    ));
                }
                let values = list(collection).expect("проверено при разборе");
                let count = values
                    .get(1)
                    .and_then(scalar)
                    .and_then(|value| value.parse::<usize>().ok());
                let distinct: std::collections::HashSet<_> =
                    ids.iter().map(|id| id.to_ascii_lowercase()).collect();
                if count != Some(ids.len())
                    || values.len() != ids.len() + 2
                    || distinct.len() != ids.len()
                {
                    return Err(malformed(
                        &root_resource,
                        "неполная коллекция общих реквизитов",
                    ));
                }
                common_attribute_ids = Some(ids.iter().map(|id| id.to_ascii_lowercase()).collect());
            }
            let Some(kind) = metadata_kind(class) else {
                continue;
            };
            if kind == MetadataKind::Subsystem && discovery.is_none() {
                for id in ids {
                    append_subsystem(&resources, id, None, &mut seen_subsystems, &mut objects)?;
                }
                continue;
            }
            for id in ids {
                if selected_only && !discovery_index.contains_key(&id.to_ascii_lowercase()) {
                    continue;
                }
                if discovery.is_some() && !resources.0.contains_key(&id.to_ascii_lowercase()) {
                    let header = discovery_index
                        .get(&id.to_ascii_lowercase())
                        .copied()
                        .ok_or_else(|| malformed(id, "объект отсутствует в индексе"))?;
                    objects.push(header.clone());
                    continue;
                }
                if kind == MetadataKind::Subsystem {
                    append_subsystem(&resources, id, None, &mut seen_subsystems, &mut objects)?;
                    continue;
                }
                let descriptors = projected_descriptors(id, &resources, cache)?;
                let descriptor = descriptors
                    .iter()
                    .find(|descriptor| descriptor.object_guid.as_str().eq_ignore_ascii_case(id))
                    .ok_or_else(|| malformed(id, "нет описателя объекта"))?;
                let mut object =
                    MetadataObject::new(id.to_ascii_lowercase(), descriptor.name.clone(), kind);
                if kind == MetadataKind::CommonAttribute {
                    // Режим берётся из явной классификации SDK по DBNames,
                    // а не из наличия настроек в описателе Config.
                    if let Some(separation) = metadata.and_then(|facts| {
                        id.parse()
                            .ok()
                            .and_then(|id| facts.common_attribute_separation(&id))
                    }) {
                        object = object.with_common_attribute_data_separation(separation);
                    }
                }
                if kind == MetadataKind::CommonAttribute
                    && let Some(usage) = &descriptor.common_attribute_usage
                {
                    object = object.with_common_attribute_auto_use(match usage.auto_use {
                        open_sdbl::metadata::CommonAttributeUse::Use => true,
                        open_sdbl::metadata::CommonAttributeUse::DontUse => false,
                    });
                }
                if let Some(number) = storage_numbers.get(&id.to_ascii_lowercase()) {
                    object = object.with_reference_storage_number(*number);
                }
                if kind == MetadataKind::Document
                    && let Some(settings) = &descriptor.document_numbering
                {
                    use bsl_orm::DocumentNumberPeriodicity as OrmPeriod;
                    use open_sdbl::metadata::DocumentNumberPeriodicity as SourcePeriod;
                    object = object.with_document_numbering_settings(
                        bsl_orm::DocumentNumberingSettings {
                            periodicity: match settings.periodicity {
                                SourcePeriod::Nonperiodical => OrmPeriod::Nonperiodical,
                                SourcePeriod::Year => OrmPeriod::Year,
                                SourcePeriod::Quarter => OrmPeriod::Quarter,
                                SourcePeriod::Month => OrmPeriod::Month,
                                SourcePeriod::Day => OrmPeriod::Day,
                            },
                            numerator: settings
                                .numerator
                                .as_ref()
                                .map(|id| std::sync::Arc::from(id.as_str())),
                        },
                    );
                }
                if kind == MetadataKind::Document
                    && let Some(registers) = &descriptor.document_registers
                {
                    // SDK подтвердил всю коллекцию: сохраняем порядок и все виды
                    // регистров, не подменяя неизвестные связи пустым списком.
                    object = object.with_document_registers(
                        registers.iter().map(|id| id.as_str().to_owned()).collect(),
                    );
                }
                if matches!(
                    kind,
                    MetadataKind::Catalog
                        | MetadataKind::ChartOfCharacteristicTypes
                        | MetadataKind::ChartOfAccounts
                ) {
                    object = object.with_catalog_predefined_names(
                        predefined
                            .remove(&id.to_ascii_lowercase())
                            .unwrap_or_default(),
                    );
                }
                object = project_catalog_descriptor_fields(object, descriptor, &descriptors)?;
                if kind == MetadataKind::Document
                    && let Some(settings) = descriptor
                        .document_number
                        .and_then(document_number_settings)
                {
                    object = object
                        .with_document_number_settings(settings)
                        .with_document_number_fixed_length(descriptor.document_number.is_some_and(
                            |number| {
                                number.number_type == open_sdbl::metadata::CatalogCodeType::String
                                    && !number.variable_length
                            },
                        ));
                }
                if kind == MetadataKind::Language {
                    let value = parse_serialized(&resources.decoded(id)?)
                        .map_err(|error| malformed(id, error))?;
                    let language_code = language_code(&value)
                        .ok_or_else(|| malformed(id, "неизвестна форма описателя языка"))?;
                    language_codes.insert(id.to_ascii_lowercase(), language_code.to_owned());
                    object = object.with_string_property("КодЯзыка", language_code);
                }
                if let Some(code) = code {
                    if let Some(module) = code.modules().iter().find(|module| {
                        module.kind() == ModuleKind::Manager
                            && module
                                .owner()
                                .is_some_and(|owner| owner.eq_ignore_ascii_case(id))
                    }) {
                        object = object.with_manager_module(module.name());
                    }
                    object = bind_object_module(object, code);
                } else if let Some(header) = discovery_index.get(object.id()) {
                    if let Some(module) = header.manager_module() {
                        object = object.with_manager_module(module);
                    }
                    if let Some(module) = header.object_module() {
                        object = if let Some(safe) = header.own_extension_object_module_safe_mode()
                        {
                            object.with_own_extension_object_module(module, safe)
                        } else if header.object_module_is_extension() {
                            object.with_extension_object_module(module)
                        } else {
                            object.with_object_module(module)
                        };
                    }
                }
                if kind == MetadataKind::ScheduledJob {
                    object = scheduled_job_metadata::project(object, &resources)?;
                }
                objects.push(object);
            }
        }
    }
    if let Some(discovery) = discovery {
        // Вложенные подсистемы присутствуют в индексе, хотя корневая
        // коллекция Config перечисляет только их родителей.
        let mut present = objects
            .iter()
            .map(|object| object.id().to_owned())
            .collect::<std::collections::HashSet<_>>();
        for header in discovery {
            if present.insert(header.id().to_owned()) {
                objects.push(header.clone());
            }
        }
    }
    objects.extend(extension_objects);

    let role_ids = objects
        .iter()
        .filter(|object| object.kind() == MetadataKind::Role)
        .map(|object| object.id().to_owned())
        .collect::<std::collections::HashSet<_>>();
    let main_roles = properties
        .get(39)
        .map(|value| collect_known_guids(value, &role_ids))
        .unwrap_or_default();
    let version = properties
        .get(15)
        .and_then(scalar)
        .ok_or_else(|| malformed(&root_resource, "нет версии конфигурации"))?;
    let script_variant = match properties.get(3).and_then(scalar) {
        Some("1") => ScriptVariant::Russian,
        Some("0") => ScriptVariant::English,
        _ => {
            return Err(malformed(
                &root_resource,
                "неизвестен вариант встроенного языка",
            ));
        }
    };
    let compatibility_mode = compatibility_mode(
        properties
            .get(26)
            .and_then(scalar)
            .ok_or_else(|| malformed(&root_resource, "нет режима совместимости"))?,
    )
    .ok_or_else(|| malformed(&root_resource, "неизвестен режим совместимости"))?;
    let primary_language = properties
        .get(10)
        .and_then(first_guid)
        .filter(|id| !id.bytes().all(|byte| byte == b'0' || byte == b'-'))
        .map(str::to_ascii_lowercase);
    let default_interface = configuration_default_interface_id(properties);
    // Нативные RU/EN-контроли измеряют выбор основного языка и fallback.
    // Используем приобретённые SDK-синонимы, не разбирая Config повторно.
    if let Some(language) = primary_language
        .as_ref()
        .and_then(|id| language_codes.get(id))
    {
        objects = project_task_runtime_reference_presentations(objects, metadata, language);
    }
    MetadataCatalog::new(identity, objects)
        // Права адресуют корневой объект, а не пространство имён каталога.
        .and_then(|catalog| catalog.with_configuration_object_id(root_resource.clone()))
        .map(|catalog| match common_attribute_ids {
            Some(ids) => catalog.with_common_attribute_ids(ids),
            None => catalog,
        })
        .and_then(|catalog| {
            catalog.with_configuration(
                configuration_name,
                version,
                script_variant,
                compatibility_mode,
                main_roles,
            )
        })
        .and_then(|catalog| match primary_language {
            Some(primary_language) => catalog.with_primary_language(primary_language),
            None => Ok(catalog),
        })
        .map(|catalog| match default_interface {
            Some(id) => catalog.with_default_interface_id(id),
            None => catalog,
        })
        .and_then(|catalog| {
            ConfigurationExtensionsSnapshot::new(Vec::new(), Vec::new(), Vec::new(), Vec::new())
                .map(|extensions| catalog.with_configuration_extensions(extensions))
        })
        .map_err(LoadError::Catalog)
}

#[cfg(test)]
fn project_task_reference_presentations(
    objects: Vec<MetadataObject>,
    metadata: Option<&MetadataSnapshot>,
    language: &str,
) -> Vec<MetadataObject> {
    project_task_runtime_reference_presentations(
        objects,
        metadata.map(MetadataSnapshot::runtime_facts),
        language,
    )
}

fn project_task_runtime_reference_presentations(
    objects: Vec<MetadataObject>,
    metadata: Option<&RuntimeMetadataFacts>,
    language: &str,
) -> Vec<MetadataObject> {
    objects
        .into_iter()
        .map(|object| {
            if object.kind() != MetadataKind::Task {
                return object;
            }
            let presentation = metadata
                .and_then(|snapshot| {
                    snapshot
                        .objects()
                        .iter()
                        .find(|acquired| acquired.guid.as_str().eq_ignore_ascii_case(object.id()))
                })
                .and_then(|acquired| acquired.synonym(language));
            match presentation {
                Some(presentation) => object.with_reference_type_presentation(presentation),
                None => object,
            }
        })
        .collect()
}

fn language_code(value: &Value) -> Option<&str> {
    let fields = list_path(value, &[1])?;
    if fields.len() != 3 || fields.first().and_then(scalar) != Some("0") {
        return None;
    }
    fields.get(2).and_then(scalar)
}

fn append_subsystem(
    resources: &Resources<'_>,
    id: &str,
    parent: Option<&str>,
    seen: &mut std::collections::HashSet<String>,
    objects: &mut Vec<MetadataObject>,
) -> Result<(), LoadError> {
    let id = id.to_ascii_lowercase();
    if !seen.insert(id.clone()) {
        return Err(malformed(&id, "подсистема повторяется или образует цикл"));
    }
    let descriptor = parse_config_descriptor(&id, resources.compressed(&id)?)
        .map_err(|error| malformed(&id, error))?
        .ok_or_else(|| malformed(&id, "нет описателя подсистемы"))?;
    let value =
        parse_serialized(&resources.decoded(&id)?).map_err(|error| malformed(&id, error))?;
    objects.push(subsystem_object(&value, &id, &descriptor.name, parent)?);
    let children = subsystem_children(&value, &id)?;
    for child in children {
        append_subsystem(resources, child, Some(&id), seen, objects)?;
    }
    Ok(())
}

fn subsystem_object(
    value: &Value,
    id: &str,
    name: &str,
    parent: Option<&str>,
) -> Result<MetadataObject, LoadError> {
    let included = match list_path(value, &[1])
        .and_then(|fields| fields.get(4))
        .and_then(scalar)
    {
        Some("1") => true,
        Some("0") => false,
        _ => return Err(malformed(id, "неизвестен признак командного интерфейса")),
    };
    let mut object = MetadataObject::new(id, name, MetadataKind::Subsystem)
        .with_subsystem_command_interface(included);
    if let Some(parent) = parent {
        object = object.with_subsystem_parent(parent);
    }
    Ok(object)
}

fn subsystem_children<'a>(value: &'a Value, id: &str) -> Result<Vec<&'a str>, LoadError> {
    let tail = list(value)
        .and_then(|entries| entries.last())
        .ok_or_else(|| malformed(id, "нет коллекции вложенных подсистем"))?;
    let (class, children) = collection_ids(tail)
        .ok_or_else(|| malformed(id, "неверная коллекция вложенных подсистем"))?;
    let count = list(tail)
        .and_then(|entries| entries.get(1))
        .and_then(scalar)
        .and_then(|count| count.parse::<usize>().ok())
        .ok_or_else(|| malformed(id, "нет числа вложенных подсистем"))?;
    if !class.eq_ignore_ascii_case(SUBSYSTEM) || children.len() != count {
        return Err(malformed(id, "неполная коллекция вложенных подсистем"));
    }
    Ok(children)
}

fn project_extension_metadata_objects(
    acquired: &AcquiredConfiguration,
    code: &ConfigurationCodeSnapshot,
) -> Result<Vec<MetadataObject>, LoadError> {
    let mut result = Vec::new();
    for extension in ordered_active_extensions(&acquired.extensions)? {
        let catalog = extension
            .catalog
            .as_ref()
            .ok_or_else(|| LoadError::IncompleteExtensionStructure(extension.name.clone()))?;
        let objects = extension
            .objects
            .as_deref()
            .ok_or_else(|| LoadError::IncompleteExtensionStructure(extension.name.clone()))?;
        let objects = objects
            .iter()
            .map(|entry| &entry.object)
            .collect::<Vec<_>>();
        let first = result.len();
        project_extension_metadata_catalog(&mut result, catalog, &objects, code, &extension.name)?;
        let resources = Resources::new(&extension.resources)?;
        for object in &mut result[first..] {
            *object = project_own_catalog_metadata(object.clone(), &resources)?;
        }
    }
    Ok(result)
}

fn project_catalog_descriptor_fields(
    mut object: MetadataObject,
    descriptor: &open_sdbl::metadata::ConfigDescriptor,
    descriptors: &[open_sdbl::metadata::ConfigDescriptor],
) -> Result<MetadataObject, LoadError> {
    if object.kind() == MetadataKind::Catalog
        && let Some(owners) = &descriptor.catalog_owners
    {
        object =
            object.with_catalog_owners(owners.iter().map(|id| id.as_str().to_owned()).collect());
    }
    if let Some(names) = catalog_attribute_names(descriptor, descriptors)? {
        object = object.with_catalog_attribute_names(names);
    }
    if let Some(names) = catalog_tabular_section_names(descriptor, descriptors)? {
        object = object.with_catalog_tabular_section_names(names);
    }
    if let Some(hierarchical) = descriptor.catalog_hierarchical {
        object = object.with_catalog_hierarchical(hierarchical);
    }
    if let Some(kind) = descriptor.catalog_hierarchy_type {
        object = object.with_catalog_hierarchy_type(match kind {
            open_sdbl::metadata::CatalogHierarchyType::FoldersAndItems => {
                bsl_orm::CatalogHierarchyType::FoldersAndItems
            }
            open_sdbl::metadata::CatalogHierarchyType::Items => {
                bsl_orm::CatalogHierarchyType::Items
            }
        });
    }
    if let Some(attributes) = descriptor.catalog_standard_attributes {
        object = object.with_catalog_standard_attributes(catalog_standard_attributes(attributes));
    }
    if let Some(settings) = &descriptor.catalog_standard_fill_checking {
        object = object.with_catalog_standard_fill_checking(
            settings
                .iter()
                .map(|setting| bsl_orm::CatalogStandardAttributeFillChecking {
                    name: setting.name.clone(),
                    fill_checking: fill_checking(setting.fill_checking),
                })
                .collect(),
        );
    }
    if let Some(presentation) = descriptor.catalog_default_presentation {
        object =
            object.with_catalog_default_presentation(catalog_default_presentation(presentation));
    }
    Ok(object)
}

fn project_own_catalog_metadata(
    object: MetadataObject,
    resources: &Resources<'_>,
) -> Result<MetadataObject, LoadError> {
    if object.kind() != MetadataKind::Catalog {
        return Ok(object);
    }
    // SDK уже знает форму собственного описателя. Сохраняем только известные
    // поля, как для основной конфигурации; заголовок не заменяет их умолчаниями.
    let descriptors = parse_config_descriptors(object.id(), resources.compressed(object.id())?)
        .map_err(|error| malformed(object.id(), error))?;
    let Some(descriptor) = descriptors.iter().find(|descriptor| {
        descriptor
            .object_guid
            .as_str()
            .eq_ignore_ascii_case(object.id())
    }) else {
        return Ok(object);
    };
    project_catalog_descriptor_fields(object, descriptor, &descriptors)
}

fn catalog_attribute_names(
    owner: &open_sdbl::metadata::ConfigDescriptor,
    descriptors: &[open_sdbl::metadata::ConfigDescriptor],
) -> Result<Option<Vec<String>>, LoadError> {
    let Some(attributes) = &owner.catalog_attributes else {
        return Ok(None);
    };
    // Собственную коллекцию и её полноту доказывает декодер SDK. Здесь
    // разрешаются лишь перечисленные GUID; соседние поля табличных частей
    // не становятся реквизитами объекта из-за совпадения формы описателя.
    attributes
        .iter()
        .map(|id| {
            descriptors
                .iter()
                .find(|descriptor| &descriptor.object_guid == id)
                .map(|descriptor| descriptor.name.clone())
                .ok_or_else(|| malformed(id.as_str(), "нет описателя собственного реквизита"))
        })
        .collect::<Result<Vec<_>, _>>()
        .map(Some)
}

fn catalog_tabular_section_names(
    owner: &open_sdbl::metadata::ConfigDescriptor,
    descriptors: &[open_sdbl::metadata::ConfigDescriptor],
) -> Result<Option<Vec<String>>, LoadError> {
    let Some(sections) = &owner.catalog_tabular_sections else {
        return Ok(None);
    };
    // Полный список собственных частей приобретён SDK. Вложенные реквизиты
    // строк не участвуют в разрешении имени свойства объекта.
    sections
        .iter()
        .map(|id| {
            descriptors
                .iter()
                .find(|descriptor| &descriptor.object_guid == id)
                .map(|descriptor| descriptor.name.clone())
                .ok_or_else(|| malformed(id.as_str(), "нет описателя табличной части"))
        })
        .collect::<Result<Vec<_>, _>>()
        .map(Some)
}

fn document_number_settings(
    settings: open_sdbl::metadata::DocumentNumberSettings,
) -> Option<bsl_orm::DocumentNumberSettings> {
    use open_sdbl::metadata::CatalogCodeType;
    Some(bsl_orm::DocumentNumberSettings {
        number_type: match settings.number_type {
            CatalogCodeType::String => bsl_orm::DocumentNumberType::String,
            CatalogCodeType::Number => bsl_orm::DocumentNumberType::Number,
        },
        number_length: settings.length,
    })
}

fn fill_checking(setting: open_sdbl::metadata::FillChecking) -> bsl_orm::FillChecking {
    match setting {
        open_sdbl::metadata::FillChecking::DontCheck => bsl_orm::FillChecking::DontCheck,
        open_sdbl::metadata::FillChecking::ShowError => bsl_orm::FillChecking::ShowError,
    }
}

fn catalog_standard_attributes(
    attributes: open_sdbl::metadata::CatalogStandardAttributes,
) -> CatalogStandardAttributes {
    CatalogStandardAttributes {
        code_type: match attributes.code_type {
            open_sdbl::metadata::CatalogCodeType::Number => CatalogCodeType::Number,
            open_sdbl::metadata::CatalogCodeType::String => CatalogCodeType::String,
        },
        code_length: attributes.code_length,
        description_length: attributes.description_length,
    }
}

fn catalog_default_presentation(
    presentation: open_sdbl::metadata::CatalogDefaultPresentation,
) -> CatalogDefaultPresentation {
    match presentation {
        open_sdbl::metadata::CatalogDefaultPresentation::AsCode => {
            CatalogDefaultPresentation::AsCode
        }
        open_sdbl::metadata::CatalogDefaultPresentation::AsDescription => {
            CatalogDefaultPresentation::AsDescription
        }
    }
}

fn bind_object_module(object: MetadataObject, code: &ConfigurationCodeSnapshot) -> MetadataObject {
    match code.modules().iter().find(|module| {
        module.kind() == ModuleKind::Object
            && module
                .owner()
                .is_some_and(|owner| owner.eq_ignore_ascii_case(object.id()))
    }) {
        Some(module) if module.own_extension_safe_mode().is_some() => object
            .with_own_extension_object_module(
                module.name(),
                module
                    .own_extension_safe_mode()
                    .expect("проверена известная политика"),
            ),
        Some(module) if module.source_is_extension() => {
            object.with_extension_object_module(module.name())
        }
        Some(module) => object.with_object_module(module.name()),
        None => object,
    }
}

fn project_extension_metadata_catalog(
    result: &mut Vec<MetadataObject>,
    catalog: &ExtensionCatalog,
    objects: &[&open_sdbl::metadata::ExtensionObject],
    code: &ConfigurationCodeSnapshot,
    extension: &str,
) -> Result<(), LoadError> {
    for class in &catalog.classes {
        let Some(kind) = metadata_kind(class.class.as_str()) else {
            continue;
        };
        for id in &class.objects {
            let header = objects
                .iter()
                .copied()
                .find(|object| object.id.as_str().eq_ignore_ascii_case(id.as_str()))
                .ok_or_else(|| LoadError::IncompleteExtensionStructure(extension.into()))?;
            match header.belonging {
                Some(ObjectBelonging::Own) => {
                    let mut object = MetadataObject::new(id.as_str(), header.name.clone(), kind);
                    if let Some(module) = code.modules().iter().find(|module| {
                        module.kind() == ModuleKind::Manager
                            && module
                                .owner()
                                .is_some_and(|owner| owner.eq_ignore_ascii_case(id.as_str()))
                    }) {
                        object = object.with_manager_module(module.name());
                    }
                    object = bind_object_module(object, code);
                    result.push(object);
                }
                Some(ObjectBelonging::Adopted) => {}
                None => {
                    return Err(LoadError::IncompleteExtensionStructure(extension.into()));
                }
                Some(_) => {
                    return Err(LoadError::UnsupportedExtensionOperation {
                        extension: extension.into(),
                        object: id.to_string(),
                    });
                }
            }
        }
    }
    Ok(())
}

#[derive(Debug)]
struct BaseObject {
    class: String,
    name: String,
}

struct LoadedBase {
    snapshot: ConfigurationCodeSnapshot,
    objects: HashMap<String, BaseObject>,
}

fn apply_structural_extension(
    modules: &mut Vec<ConfigurationModule>,
    base_objects: &HashMap<String, BaseObject>,
    extension: &AcquiredExtension,
    options: LoadOptions,
) -> Result<(), LoadError> {
    let catalog = extension
        .catalog
        .as_ref()
        .ok_or_else(|| LoadError::IncompleteExtensionStructure(extension.name.clone()))?;
    let objects = extension
        .objects
        .as_deref()
        .ok_or_else(|| LoadError::IncompleteExtensionStructure(extension.name.clone()))?;
    let resources = Resources::with_options(&extension.resources, options)?;

    reject_extension_session_module(&resources, catalog, &extension.name)?;
    for class in &catalog.classes {
        for local_id in &class.objects {
            let header = extension_object(objects, local_id.as_str())
                .ok_or_else(|| LoadError::IncompleteExtensionStructure(extension.name.clone()))?;
            apply_extension_object(
                modules,
                base_objects,
                &resources,
                class.class.as_str(),
                header,
                &extension.name,
            )?;
            // Настройка принадлежит фактическому собственному источнику.
            // Заимствованная замена создаёт модуль через обычный билдер и
            // снимает эту политику; отсутствие замены сохраняет прежний модуль.
            if header.belonging == Some(ObjectBelonging::Own)
                && extension.security_profile_name.as_deref() == Some("")
                && let Some(safe_mode) = extension.safe_mode
            {
                stamp_own_extension_policy(modules, local_id.as_str(), safe_mode);
            }
        }
    }
    Ok(())
}

fn stamp_own_extension_policy(modules: &mut [ConfigurationModule], owner: &str, safe_mode: bool) {
    for module in modules.iter_mut().filter(|module| {
        module.kind() == ModuleKind::Object
            && module.source_is_extension()
            && module
                .owner()
                .is_some_and(|id| id.eq_ignore_ascii_case(owner))
    }) {
        *module = module.clone().with_own_extension_source(safe_mode);
    }
}

fn reject_extension_session_module(
    resources: &Resources<'_>,
    catalog: &ExtensionCatalog,
    extension: &str,
) -> Result<(), LoadError> {
    if resources
        .module_source(catalog.configuration.as_str(), "7")?
        .is_some()
    {
        return Err(LoadError::UnsupportedExtensionOperation {
            extension: extension.into(),
            object: catalog.configuration.to_string(),
        });
    }
    Ok(())
}

fn extension_object<'a>(
    objects: &'a [AcquiredExtensionObject],
    id: &str,
) -> Option<&'a open_sdbl::metadata::ExtensionObject> {
    objects
        .iter()
        .find(|entry| entry.object.id.as_str().eq_ignore_ascii_case(id))
        .map(|entry| &entry.object)
}

fn extension_target<'a>(
    base_objects: &'a HashMap<String, BaseObject>,
    class: &str,
    object: &'a open_sdbl::metadata::ExtensionObject,
    extension: &str,
) -> Result<(&'a str, &'a str), LoadError> {
    let local_id = object.id.as_str();
    let target = match object.belonging {
        Some(ObjectBelonging::Own) => (local_id, object.name.as_str()),
        Some(ObjectBelonging::Adopted) => {
            let unsupported = || LoadError::UnsupportedExtensionOperation {
                extension: extension.into(),
                object: local_id.into(),
            };
            let (target_id, base) = if let Some(target) = &object.extends {
                (
                    target.as_str(),
                    base_objects.get(target.as_str()).ok_or_else(unsupported)?,
                )
            } else if [
                STYLE_ITEM,
                COMMON_PICTURE,
                LANGUAGE,
                ENUMERATION,
                "c045099e-13b9-4fb6-9d50-fca00202971e",
                "2f1a5187-fb0e-4b05-9489-dc5dd6412348",
                "cf4abea6-37b2-11d4-940f-008048da11f9",
            ]
            .iter()
            .any(|measured| class.eq_ignore_ascii_case(measured))
            {
                // На 1С 8.3.27 исходное legacy-v1 расширение и v3 с нулевой
                // целью находят объекты семи измеренных классов по имени. Другой регистр
                // отклоняется проверкой свойства Name: без сведений о её
                // отключении принимаем только точное совпадение. Явный GUID
                // и неизмеренные классы этим правилом не подменяются.
                let (id, base) = base_objects
                    .iter()
                    .find(|(_, base)| {
                        base.class.eq_ignore_ascii_case(class) && base.name == object.name
                    })
                    .ok_or_else(unsupported)?;
                (id.as_str(), base)
            } else {
                return Err(unsupported());
            };
            if !base.class.eq_ignore_ascii_case(class) {
                return Err(LoadError::UnsupportedExtensionOperation {
                    extension: extension.into(),
                    object: local_id.into(),
                });
            }
            (target_id, base.name.as_str())
        }
        None => {
            return Err(LoadError::IncompleteExtensionStructure(extension.into()));
        }
        Some(_) => {
            return Err(LoadError::UnsupportedExtensionOperation {
                extension: extension.into(),
                object: local_id.into(),
            });
        }
    };
    Ok(target)
}

fn apply_extension_object(
    modules: &mut Vec<ConfigurationModule>,
    base_objects: &HashMap<String, BaseObject>,
    resources: &Resources<'_>,
    class: &str,
    object: &open_sdbl::metadata::ExtensionObject,
    extension: &str,
) -> Result<(), LoadError> {
    let local_id = object.id.as_str();
    let (target_id, target_name) = extension_target(base_objects, class, object, extension)?;

    if class.eq_ignore_ascii_case(COMMON) {
        let source = resources.module_source(local_id, "0")?;
        let existing = modules
            .iter()
            .find(|module| module.id().eq_ignore_ascii_case(&format!("{target_id}.0")));
        let extension_source =
            source.is_some() || existing.is_some_and(ConfigurationModule::source_is_extension);
        let source = source
            .or_else(|| existing.map(|module| module.source().to_owned()))
            .unwrap_or_default();
        let execution = common_execution(&resources.decoded(local_id)?)?;
        replace_module(
            modules,
            ConfigurationModule::new(
                format!("{target_id}.0"),
                target_name,
                source,
                ModuleKind::Common,
                execution,
            )
            .with_extension_source(extension_source),
        );
        return Ok(());
    }

    for slot in module_slots(class) {
        let Some(source) = resources.module_source(local_id, slot.suffix)? else {
            continue;
        };
        replace_module(
            modules,
            ConfigurationModule::new(
                format!("{target_id}.{}", slot.suffix),
                format!("{}.{}.{}", slot.prefix, target_name, slot.label),
                source,
                slot.kind,
                ModuleExecution::default().with_server(true),
            )
            .with_owner(target_id)
            .with_extension_source(true),
        );
    }
    Ok(())
}

fn replace_module(modules: &mut Vec<ConfigurationModule>, replacement: ConfigurationModule) {
    if let Some(index) = modules
        .iter()
        .position(|module| module.id().eq_ignore_ascii_case(replacement.id()))
    {
        modules[index] = replacement;
    } else {
        modules.push(replacement);
    }
}

#[cfg(test)]
fn load_base(resources: &[ConfigResource]) -> Result<ConfigurationCodeSnapshot, LoadError> {
    // Нативные контроли проверяют полные тела независимо от политики хоста.
    // Режим по умолчанию проверяется отдельно через `LoadOptions`.
    load_base_graph(
        resources,
        LoadOptions {
            decompile_protected_modules: true,
        },
    )
    .map(|base| base.snapshot)
}

fn load_base_graph(
    resources: &[ConfigResource],
    options: LoadOptions,
) -> Result<LoadedBase, LoadError> {
    let resources = Resources::with_options(resources, options)?;
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
    let mut objects = HashMap::new();
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
                objects.insert(
                    id.to_ascii_lowercase(),
                    BaseObject {
                        class: class.to_ascii_lowercase(),
                        name: descriptor.name.clone(),
                    },
                );
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
    let snapshot =
        ConfigurationCodeSnapshot::new(identity, modules).map_err(LoadError::Snapshot)?;
    Ok(LoadedBase { snapshot, objects })
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
        "857c4a91-e5f4-4fac-86ec-787626f1c108" => {
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

fn configuration_properties(value: &Value) -> Option<&[Value]> {
    let root = list(value)?;
    let count = scalar(root.get(2)?)?.parse::<usize>().ok()?;
    let configuration = root.iter().skip(3).take(count).find(|group| {
        list(group)
            .and_then(|group| group.first())
            .and_then(scalar)
            .is_some_and(|class| class.eq_ignore_ascii_case(CONFIGURATION))
    })?;
    list_path(configuration, &[1, 1])
}

fn configuration_default_interface_id(properties: &[Value]) -> Option<[u8; 16]> {
    // Нативная пара CF/XML с пустым и назначенным интерфейсами установила
    // поле 11 профиля 68/61. Соседние нулевые GUID не несут этой настройки.
    if properties.len() != 61
        || !matches!(properties.first(), Some(Value::Atom(tag)) if tag == "68")
    {
        return None;
    }
    let Value::Atom(id) = properties.get(11)? else {
        return None;
    };
    open_bsl::uuid::parse(id).ok()
}

fn configuration_name(properties: &[Value]) -> Option<&str> {
    let descriptor = list_path(properties.get(1)?, &[1])?;
    scalar(descriptor.get(2)?)
}

fn metadata_kind(class: &str) -> Option<MetadataKind> {
    match class.to_ascii_lowercase().as_str() {
        SUBSYSTEM => Some(MetadataKind::Subsystem),
        LANGUAGE => Some(MetadataKind::Language),
        ENUMERATION => Some(MetadataKind::Enumeration),
        COMMON_PICTURE => Some(MetadataKind::CommonPicture),
        COMMON_ATTRIBUTE => Some(MetadataKind::CommonAttribute),
        "cf4abea6-37b2-11d4-940f-008048da11f9" => Some(MetadataKind::Catalog),
        "061d872a-5787-460e-95ac-ed74ea3a3e84" => Some(MetadataKind::Document),
        "4612bd75-71b7-4a5c-8cc5-2b0b65f9fa0d" => Some(MetadataKind::DocumentJournal),
        "13134201-f60b-11d5-a3c7-0050bae0a776" => Some(MetadataKind::InformationRegister),
        "b64d9a40-1642-11d6-a3c7-0050bae0a776" => Some(MetadataKind::AccumulationRegister),
        "2deed9b8-0056-4ffe-a473-c20a6c32a0bc" => Some(MetadataKind::AccountingRegister),
        "f2de87a8-64e5-45eb-a22d-b3aedab050e7" => Some(MetadataKind::CalculationRegister),
        "238e7e88-3c5f-48b2-8a3b-81ebbecb20ed" => Some(MetadataKind::ChartOfAccounts),
        "82a1b659-b220-4d94-a9bd-14d757b95a48" => Some(MetadataKind::ChartOfCharacteristicTypes),
        "30b100d6-b29f-47ac-aec7-cb8ca8a54767" => Some(MetadataKind::ChartOfCalculationTypes),
        "857c4a91-e5f4-4fac-86ec-787626f1c108" => Some(MetadataKind::ExchangePlan),
        "0195e80c-b157-11d4-9435-004095e12fc7" => Some(MetadataKind::Constant),
        "631b75a0-29e2-11d6-a3c7-0050bae0a776" => Some(MetadataKind::Report),
        "bf845118-327b-4682-b5c6-285d2a0eb296" => Some(MetadataKind::DataProcessor),
        "fcd3404e-1523-48ce-9bc0-ecdb822684a1" => Some(MetadataKind::BusinessProcess),
        "3e63355c-1378-4953-be9b-1deb5fb6bec5" => Some(MetadataKind::Task),
        "46b4cd97-fd13-4eaa-aba2-3bddd7699218" => Some(MetadataKind::SettingsStorage),
        "bc587f20-35d9-11d6-a3c7-0050bae0a776" => Some(MetadataKind::Sequence),
        FUNCTIONAL_OPTION => Some(MetadataKind::FunctionalOption),
        STYLE_ITEM => Some(MetadataKind::StyleItem),
        ROLE => Some(MetadataKind::Role),
        SESSION_PARAMETER => Some(MetadataKind::SessionParameter),
        COMMON => Some(MetadataKind::CommonModule),
        SCHEDULED_JOB => Some(MetadataKind::ScheduledJob),
        _ => None,
    }
}

fn collect_known_guids(value: &Value, known: &std::collections::HashSet<String>) -> Vec<String> {
    let mut result = Vec::new();
    collect_known_guids_into(value, known, &mut result);
    result
}

fn collect_known_guids_into(
    value: &Value,
    known: &std::collections::HashSet<String>,
    result: &mut Vec<String>,
) {
    match value {
        Value::Atom(value) | Value::String(value) => {
            let value = value.to_ascii_lowercase();
            if known.contains(&value) && !result.contains(&value) {
                result.push(value);
            }
        }
        Value::List(values) => {
            for value in values {
                collect_known_guids_into(value, known, result);
            }
        }
        Value::Null => {}
    }
}

fn compatibility_mode(value: &str) -> Option<CompatibilityMode> {
    Some(match value {
        "80100" => CompatibilityMode::Version8_1,
        "80213" => CompatibilityMode::Version8_2_13,
        "80216" => CompatibilityMode::Version8_2_16,
        "80301" => CompatibilityMode::Version8_3_1,
        "80302" => CompatibilityMode::Version8_3_2,
        "80303" => CompatibilityMode::Version8_3_3,
        "80304" => CompatibilityMode::Version8_3_4,
        "80305" => CompatibilityMode::Version8_3_5,
        "80306" => CompatibilityMode::Version8_3_6,
        "80307" => CompatibilityMode::Version8_3_7,
        "80308" => CompatibilityMode::Version8_3_8,
        "80309" => CompatibilityMode::Version8_3_9,
        "80310" => CompatibilityMode::Version8_3_10,
        "80311" => CompatibilityMode::Version8_3_11,
        "80312" => CompatibilityMode::Version8_3_12,
        "80313" => CompatibilityMode::Version8_3_13,
        "80314" => CompatibilityMode::Version8_3_14,
        "80315" => CompatibilityMode::Version8_3_15,
        "80316" => CompatibilityMode::Version8_3_16,
        "80317" => CompatibilityMode::Version8_3_17,
        "80318" => CompatibilityMode::Version8_3_18,
        "80319" => CompatibilityMode::Version8_3_19,
        "80320" => CompatibilityMode::Version8_3_20,
        "80321" => CompatibilityMode::Version8_3_21,
        "80322" => CompatibilityMode::Version8_3_22,
        "80323" => CompatibilityMode::Version8_3_23,
        "80324" => CompatibilityMode::Version8_3_24,
        "80325" => CompatibilityMode::Version8_3_25,
        "80326" => CompatibilityMode::Version8_3_26,
        // ИЗМЕРЕНО на 8.3.27.2342: `НеИспользовать` хранится как
        // текущая версия формата, а объект метаданных возвращает `DontUse`.
        "80327" => CompatibilityMode::DontUse,
        _ => return None,
    })
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

struct Resources<'a>(HashMap<String, &'a ConfigResource>, LoadOptions);
impl<'a> Resources<'a> {
    fn new(resources: &'a [ConfigResource]) -> Result<Self, LoadError> {
        // Сохраняем внутренний путь чтения и полные нативные контроли.
        // Загрузчики снимка всегда передают выбор хоста через `with_options`.
        Self::with_options(
            resources,
            LoadOptions {
                decompile_protected_modules: true,
            },
        )
    }
    fn with_options(
        resources: &'a [ConfigResource],
        options: LoadOptions,
    ) -> Result<Self, LoadError> {
        let mut result = HashMap::with_capacity(resources.len());
        for resource in resources {
            let name = resource.file_name.to_ascii_lowercase();
            if result.insert(name.clone(), resource).is_some() {
                return Err(LoadError::DuplicateResource(name));
            }
        }
        Ok(Self(result, options))
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
        self.module_text(owner, suffix, false)
            .map(|source| source.map(|(text, _)| text))
    }
    fn module_text(
        &self,
        owner: &str,
        suffix: &str,
        declarations: bool,
    ) -> Result<Option<(String, bool)>, LoadError> {
        let name = format!("{owner}.{suffix}").to_ascii_lowercase();
        let Some(resource) = self.0.get(&name) else {
            return Ok(None);
        };
        let decoded =
            inflate_raw_deflate(resource.compressed.as_ref()).map_err(|e| malformed(&name, e))?;
        decode_module_payload(&name, decoded, self.1, declarations)
    }
}

fn decode_module_payload(
    name: &str,
    decoded: Vec<u8>,
    options: LoadOptions,
    declarations: bool,
) -> Result<Option<(String, bool)>, LoadError> {
    let bytes = if decoded.starts_with(&[0xff, 0xff, 0xff, 0x7f]) {
        if let Some(bytes) = container_entry(&decoded, "text")? {
            bytes
        } else if let Some(image) = container_entry(&decoded, "image")? {
            let source = if options.decompile_protected_modules && declarations {
                native_image::declarations(&image)
            } else if options.decompile_protected_modules {
                native_image::decompile(&image)
            } else {
                native_image::exported_interface(&image)
            };
            return source
                .map(|source| {
                    Some((
                        source,
                        !(options.decompile_protected_modules && declarations),
                    ))
                })
                .map_err(|reason| LoadError::UnsupportedModuleImage {
                    module: name.into(),
                    reason,
                });
        } else {
            return Ok(None);
        }
    } else {
        decoded
    };
    decode_text(name, &bytes).map(|source| Some((source, true)))
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

fn container_entry(container: &[u8], wanted: &str) -> Result<Option<Vec<u8>>, LoadError> {
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
                if name == wanted {
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
    /// Нативный образ повреждён или не может быть полностью восстановлен.
    UnsupportedModuleImage {
        /// Адрес ресурса модуля в выбранном снимке конфигурации.
        module: String,
        /// Место и причина отказа восстановления.
        reason: String,
    },
    /// Расширение нельзя применить без подмены эффективного кода базовым.
    UnsupportedExtension {
        /// Имя расширения из каталога базы.
        name: String,
        /// Распознан ли признак активности.
        activity_known: bool,
    },
    /// Структурное описание расширения отсутствует или неполно.
    IncompleteExtensionStructure(String),
    /// Операцию расширения нельзя свести к одному итоговому графу модулей.
    UnsupportedExtensionOperation {
        /// Имя расширения из каталога базы.
        extension: String,
        /// Локальный идентификатор объекта расширения.
        object: String,
    },
    /// Приобретение не содержит всех свойств расширения для ORM-снимка.
    IncompleteExtensionMetadata(String),
    /// Итоговый нейтральный снимок некорректен.
    Snapshot(SnapshotError),
    /// Итоговый ORM-каталог некорректен.
    Catalog(CatalogError),
}
impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingResource(n) => write!(f, "нет ресурса конфигурации `{n}`"),
            Self::DuplicateResource(n) => write!(f, "ресурс конфигурации `{n}` повторяется"),
            Self::MalformedResource(n, r) => write!(f, "ресурс конфигурации `{n}` повреждён: {r}"),
            Self::MalformedContainer(r) => write!(f, "контейнер модуля повреждён: {r}"),
            Self::UnsupportedEncoding(n) => write!(f, "кодировка модуля `{n}` не поддерживается"),
            Self::UnsupportedModuleImage { module, reason } => {
                write!(f, "образ модуля `{module}` не поддерживается: {reason}")
            }
            Self::UnsupportedExtension {
                name,
                activity_known: true,
            } => write!(f, "активное расширение `{name}` пока нельзя применить"),
            Self::UnsupportedExtension {
                name,
                activity_known: false,
            } => write!(f, "у расширения `{name}` не распознана активность"),
            Self::IncompleteExtensionStructure(name) => write!(
                f,
                "приобретение не содержит полного структурного описания расширения `{name}`"
            ),
            Self::UnsupportedExtensionOperation { extension, object } => write!(
                f,
                "операцию расширения `{extension}` над объектом `{object}` нельзя применить"
            ),
            Self::IncompleteExtensionMetadata(name) => write!(
                f,
                "приобретение не содержит полного описания расширения `{name}`"
            ),
            Self::Snapshot(e) => write!(f, "снимок кода некорректен: {e}"),
            Self::Catalog(e) => write!(f, "каталог метаданных некорректен: {e}"),
        }
    }
}
impl Error for LoadError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Snapshot(error) => Some(error),
            Self::Catalog(error) => Some(error),
            _ => None,
        }
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

#[cfg(test)]
#[path = "native_user_full_names_tests.rs"]
mod native_user_full_names_tests;

mod scheduled_job_metadata;

#[cfg(test)]
#[path = "catalog_predefined_tests.rs"]
mod catalog_predefined_tests;

#[cfg(test)]
#[path = "configuration_extension_projection_tests.rs"]
mod configuration_extension_projection_tests;

#[cfg(test)]
#[path = "native_test_output.rs"]
mod native_test_output;

#[cfg(test)]
#[path = "default_interface_tests.rs"]
mod default_interface_tests;

// Исходные описатели отличаются от разрешённого снимка: разрешение именованных
// типов дополняет поля снимка, а проекции нужны первоначальные факты ресурса.
fn projected_descriptors<'a>(
    id: &str,
    resources: &Resources<'_>,
    cache: Option<&'a open_sdbl_db::ConfigurationMetadataCache>,
) -> Result<std::borrow::Cow<'a, [open_sdbl::metadata::ConfigDescriptor]>, LoadError> {
    if let Some(parsed) = cache.and_then(|cache| cache.parsed_resource(id)) {
        return Ok(std::borrow::Cow::Borrowed(&parsed.descriptors));
    }
    parse_config_descriptors(id, resources.compressed(id)?)
        .map(std::borrow::Cow::Owned)
        .map_err(|error| malformed(id, error))
}

#[cfg(test)]
#[path = "cached_projection_tests.rs"]
mod cached_projection_tests;
