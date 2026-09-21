//! Объектная модель данных и метаданных конфигурации 1С.
//!
//! Крейт является runtime-компонентом для существующего интерпретатора
//! `open-bsl`. Он не разбирает и не исполняет BSL самостоятельно. Физический
//! доступ к данным будет внедряться данными сеанса; каталог ниже хранит только
//! неизменяемое описание конфигурации и связи с её BSL-модулями.

#![warn(missing_docs)]

use std::any::Any;
use std::collections::HashSet;
use std::error::Error;
use std::fmt;
use std::rc::Rc;
use std::sync::Arc;

use open_bsl::{
    Arity, BslObject, CallContext, EnumKind, EnumValue, FunctionCode, FunctionDescriptor,
    FunctionKind, GlobalDescriptor, LibraryDescriptor, MethodDescriptor, ObjectMembersDescriptor,
    ObjectProtocol, PropertyDescriptor, RtError, RtResult, TypeDescriptor, Value,
    call_method_from_table, folded_eq, get_property_from_table,
};

/// Основной язык программирования конфигурации.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptVariant {
    /// Русский вариант.
    Russian,
    /// Английский вариант.
    English,
}

impl ScriptVariant {
    fn value(self) -> Value {
        Value::Enum(match self {
            Self::Russian => EnumValue::ScriptVariantRussian,
            Self::English => EnumValue::ScriptVariantEnglish,
        })
    }
}

/// Область действия расширения конфигурации.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigurationExtensionScope {
    /// Вся информационная база.
    InfoBase,
    /// Текущая область разделённых данных.
    DataSeparation,
}

impl ConfigurationExtensionScope {
    fn value(self) -> Value {
        Value::Enum(match self {
            Self::InfoBase => EnumValue::ConfigurationExtensionScopeInfoBase,
            Self::DataSeparation => EnumValue::ConfigurationExtensionScopeDataSeparation,
        })
    }
}

/// Безопасный режим расширения конфигурации.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExtensionSafeMode {
    /// Основной или стандартный безопасный профиль выбирается булевым значением.
    Enabled(bool),
    /// Используется именованный профиль безопасности.
    Profile(Arc<str>),
}

impl ExtensionSafeMode {
    /// Создаёт значение с именованным профилем безопасности.
    #[must_use]
    pub fn profile(name: impl Into<String>) -> Self {
        Self::Profile(Arc::from(name.into()))
    }

    fn value(&self) -> Value {
        match self {
            Self::Enabled(value) => Value::Boolean(*value),
            Self::Profile(name) => Value::Str(name.as_ref().into()),
        }
    }
}

/// Важность проблемы применения расширения конфигурации.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtensionApplicationIssueSeverity {
    /// Критичная проблема.
    Critical,
    /// Проблема низкой важности.
    Low,
    /// Проблема обычной важности.
    Moderate,
}

impl ExtensionApplicationIssueSeverity {
    fn value(self) -> Value {
        Value::Enum(match self {
            Self::Critical => EnumValue::ConfigurationExtensionApplicationIssueSeverityCritical,
            Self::Low => EnumValue::ConfigurationExtensionApplicationIssueSeverityLow,
            Self::Moderate => EnumValue::ConfigurationExtensionApplicationIssueSeverityModerate,
        })
    }
}

/// Неизменяемое описание расширения конфигурации.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigurationExtension {
    id: [u8; 16],
    name: Arc<str>,
    version: Arc<str>,
    hash_sum: Arc<str>,
    safe_mode: ExtensionSafeMode,
    used_in_distributed_infobase: bool,
    active: bool,
    scope: ConfigurationExtensionScope,
}

impl ConfigurationExtension {
    /// Создаёт описание расширения, полученное загрузчиком конфигурации.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку, если имя расширения пусто.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: [u8; 16],
        name: impl Into<String>,
        version: impl Into<String>,
        hash_sum: impl Into<String>,
        safe_mode: ExtensionSafeMode,
        used_in_distributed_infobase: bool,
        active: bool,
        scope: ConfigurationExtensionScope,
    ) -> Result<Self, CatalogError> {
        let name = name.into();
        if name.is_empty() {
            return Err(CatalogError::EmptyExtensionName);
        }
        Ok(Self {
            id,
            name: Arc::from(name),
            version: Arc::from(version.into()),
            hash_sum: Arc::from(hash_sum.into()),
            safe_mode,
            used_in_distributed_infobase,
            active,
            scope,
        })
    }

    /// Возвращает идентификатор расширения.
    #[must_use]
    pub const fn id(&self) -> &[u8; 16] {
        &self.id
    }

    /// Возвращает имя расширения.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }
}

/// Неизменяемая проблема применения расширения в текущем сеансе.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtensionApplicationIssue {
    extension_id: [u8; 16],
    description: Arc<str>,
    severity: ExtensionApplicationIssueSeverity,
}

impl ExtensionApplicationIssue {
    /// Создаёт описание проблемы для указанного расширения.
    #[must_use]
    pub fn new(
        extension_id: [u8; 16],
        description: impl Into<String>,
        severity: ExtensionApplicationIssueSeverity,
    ) -> Self {
        Self {
            extension_id,
            description: Arc::from(description.into()),
            severity,
        }
    }
}

/// Полный снимок расширений и проблем их применения для одного сеанса.
#[derive(Debug, Clone)]
pub struct ConfigurationExtensionsSnapshot {
    database: Arc<[ConfigurationExtension]>,
    session_applied: Arc<[usize]>,
    session_disabled: Arc<[usize]>,
    issues: Arc<[ExtensionApplicationIssue]>,
}

impl ConfigurationExtensionsSnapshot {
    /// Проверяет и создаёт полный снимок расширений.
    ///
    /// Идентификаторы активных и отключённых расширений ссылаются на
    /// элементы `database`. Пустые коллекции означают измеренно пустой
    /// результат, а отсутствие снимка представляется на уровне каталога.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку для повторного идентификатора или имени,
    /// неизвестной ссылки, пересечения активного и отключённого наборов и
    /// проблемы, ссылающейся на неизвестное расширение.
    pub fn new(
        database: Vec<ConfigurationExtension>,
        session_applied_ids: impl IntoIterator<Item = [u8; 16]>,
        session_disabled_ids: impl IntoIterator<Item = [u8; 16]>,
        issues: Vec<ExtensionApplicationIssue>,
    ) -> Result<Self, CatalogError> {
        for (index, extension) in database.iter().enumerate() {
            if database[..index]
                .iter()
                .any(|known| known.id == extension.id)
            {
                return Err(CatalogError::DuplicateExtensionIdentity);
            }
            if database[..index]
                .iter()
                .any(|known| folded_eq(&known.name, &extension.name))
            {
                return Err(CatalogError::DuplicateExtensionName {
                    name: extension.name.to_string(),
                });
            }
        }
        let resolve = |id: [u8; 16]| {
            database
                .iter()
                .position(|extension| extension.id == id)
                .ok_or(CatalogError::UnknownExtensionIdentity)
        };
        let session_applied = session_applied_ids
            .into_iter()
            .map(resolve)
            .collect::<Result<Vec<_>, _>>()?;
        let session_disabled = session_disabled_ids
            .into_iter()
            .map(resolve)
            .collect::<Result<Vec<_>, _>>()?;
        if session_applied
            .iter()
            .any(|index| session_disabled.contains(index))
        {
            return Err(CatalogError::ExtensionBothAppliedAndDisabled);
        }
        if issues.iter().any(|issue| {
            !database
                .iter()
                .any(|extension| extension.id == issue.extension_id)
        }) {
            return Err(CatalogError::UnknownIssueExtension);
        }
        Ok(Self {
            database: database.into(),
            session_applied: session_applied.into(),
            session_disabled: session_disabled.into(),
            issues: issues.into(),
        })
    }
}

/// Вид объекта метаданных, нужный исполняемому серверному коду.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum MetadataKind {
    /// Справочник.
    Catalog,
    /// Регистр сведений.
    InformationRegister,
    /// Константа.
    Constant,
    /// Функциональная опция.
    FunctionalOption,
    /// Элемент стиля.
    StyleItem,
    /// Роль.
    Role,
    /// Параметр сеанса.
    SessionParameter,
    /// Общий модуль.
    CommonModule,
    /// Регламентное задание.
    ScheduledJob,
}

/// Неизменяемое описание одного объекта метаданных.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetadataObject {
    id: Arc<str>,
    name: Arc<str>,
    kind: MetadataKind,
    manager_module: Option<Arc<str>>,
    string_properties: Arc<[(Arc<str>, Arc<str>)]>,
}

impl MetadataObject {
    /// Создаёт объект без связанного модуля менеджера.
    #[must_use]
    pub fn new(id: impl Into<String>, name: impl Into<String>, kind: MetadataKind) -> Self {
        Self {
            id: Arc::from(id.into()),
            name: Arc::from(name.into()),
            kind,
            manager_module: None,
            string_properties: Arc::from([]),
        }
    }

    /// Связывает объект с именованным модулем менеджера конфигурации.
    #[must_use]
    pub fn with_manager_module(mut self, module: impl Into<String>) -> Self {
        self.manager_module = Some(Arc::from(module.into()));
        self
    }

    /// Добавляет строковое свойство объекта метаданных.
    #[must_use]
    pub fn with_string_property(
        mut self,
        name: impl Into<String>,
        value: impl Into<String>,
    ) -> Self {
        let mut properties = self.string_properties.to_vec();
        properties.push((Arc::from(name.into()), Arc::from(value.into())));
        self.string_properties = properties.into();
        self
    }

    #[must_use]
    /// Возвращает устойчивый идентификатор объекта.
    pub fn id(&self) -> &str {
        &self.id
    }

    #[must_use]
    /// Возвращает имя объекта в конфигурации.
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    /// Возвращает вид объекта.
    pub const fn kind(&self) -> MetadataKind {
        self.kind
    }

    #[must_use]
    /// Возвращает имя связанного модуля менеджера.
    pub fn manager_module(&self) -> Option<&str> {
        self.manager_module.as_deref()
    }
}

/// Неизменяемые метаданные одной конфигурации.
#[derive(Debug)]
pub struct MetadataCatalog {
    identity: Arc<str>,
    objects: Arc<[MetadataObject]>,
    configuration: Option<ConfigurationDescription>,
    configuration_extensions: Option<Arc<ConfigurationExtensionsSnapshot>>,
}

#[derive(Debug)]
struct ConfigurationDescription {
    name: Arc<str>,
    version: Arc<str>,
    script_variant: ScriptVariant,
    main_role_ids: Arc<[Arc<str>]>,
}

impl MetadataCatalog {
    /// Проверяет и создаёт каталог.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку для пустой идентичности, пустых идентификаторов и
    /// имён, повторных идентификаторов или повторных имён одного вида.
    pub fn new(
        identity: impl Into<String>,
        objects: Vec<MetadataObject>,
    ) -> Result<Self, CatalogError> {
        let identity = identity.into();
        if identity.is_empty() {
            return Err(CatalogError::EmptyConfigurationIdentity);
        }
        let mut ids = HashSet::with_capacity(objects.len());
        for (index, object) in objects.iter().enumerate() {
            if object.id.is_empty() {
                return Err(CatalogError::EmptyObjectIdentity);
            }
            if object.name.is_empty() {
                return Err(CatalogError::EmptyObjectName {
                    object: object.id.to_string(),
                });
            }
            if !ids.insert(object.id.as_ref()) {
                return Err(CatalogError::DuplicateObjectIdentity {
                    object: object.id.to_string(),
                });
            }
            if objects[..index]
                .iter()
                .any(|other| other.kind == object.kind && folded_eq(&other.name, &object.name))
            {
                return Err(CatalogError::DuplicateObjectName {
                    name: object.name.to_string(),
                });
            }
            if object.manager_module.as_deref() == Some("") {
                return Err(CatalogError::EmptyManagerModule {
                    object: object.id.to_string(),
                });
            }
            for (property_index, (name, _)) in object.string_properties.iter().enumerate() {
                if name.is_empty() {
                    return Err(CatalogError::EmptyPropertyName {
                        object: object.id.to_string(),
                    });
                }
                if object.string_properties[..property_index]
                    .iter()
                    .any(|(known, _)| folded_eq(known, name))
                {
                    return Err(CatalogError::DuplicatePropertyName {
                        object: object.id.to_string(),
                        property: name.to_string(),
                    });
                }
            }
        }
        Ok(Self {
            identity: identity.into(),
            objects: objects.into(),
            configuration: None,
            configuration_extensions: None,
        })
    }

    /// Добавляет свойства корневого объекта метаданных конфигурации.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку для пустого имени, неизвестной основной роли,
    /// объекта другого вида или повторной роли.
    pub fn with_configuration(
        mut self,
        name: impl Into<String>,
        version: impl Into<String>,
        script_variant: ScriptVariant,
        main_role_ids: impl IntoIterator<Item = String>,
    ) -> Result<Self, CatalogError> {
        let name = name.into();
        if name.is_empty() {
            return Err(CatalogError::EmptyConfigurationName);
        }
        let mut roles = Vec::<Arc<str>>::new();
        for id in main_role_ids {
            let role = self
                .object_by_id(&id)
                .filter(|object| object.kind == MetadataKind::Role)
                .ok_or_else(|| CatalogError::UnknownMainRole(id.clone()))?;
            if roles.iter().any(|known| known.as_ref() == role.id()) {
                return Err(CatalogError::DuplicateMainRole(id));
            }
            roles.push(Arc::clone(&role.id));
        }
        self.configuration = Some(ConfigurationDescription {
            name: name.into(),
            version: Arc::from(version.into()),
            script_variant,
            main_role_ids: roles.into(),
        });
        Ok(self)
    }

    /// Добавляет полный снимок расширений конфигурации текущего сеанса.
    #[must_use]
    pub fn with_configuration_extensions(
        mut self,
        extensions: ConfigurationExtensionsSnapshot,
    ) -> Self {
        self.configuration_extensions = Some(Arc::new(extensions));
        self
    }

    #[must_use]
    /// Возвращает идентичность конфигурации.
    pub fn identity(&self) -> &str {
        &self.identity
    }

    #[must_use]
    /// Возвращает объекты в порядке источника.
    pub fn objects(&self) -> &[MetadataObject] {
        &self.objects
    }

    #[must_use]
    /// Находит объект по устойчивому идентификатору.
    pub fn object_by_id(&self, id: &str) -> Option<&MetadataObject> {
        self.objects.iter().find(|object| object.id.as_ref() == id)
    }

    #[must_use]
    /// Находит объект указанного вида по регистронезависимому имени.
    pub fn object(&self, kind: MetadataKind, name: &str) -> Option<&MetadataObject> {
        self.objects
            .iter()
            .find(|object| object.kind == kind && folded_eq(&object.name, name))
    }

    /// Возвращает идентификаторы основных ролей конфигурации.
    /// Пустой итератор также означает, что свойства конфигурации ещё не
    /// переданы загрузчиком.
    pub fn main_role_ids(&self) -> impl Iterator<Item = &str> {
        self.configuration
            .as_ref()
            .into_iter()
            .flat_map(|description| description.main_role_ids.iter().map(AsRef::as_ref))
    }
}

/// Замороженное описание пользователя текущего сеанса.
#[derive(Debug, Clone)]
pub struct Principal {
    id: Arc<str>,
    name: Arc<str>,
    role_ids: Arc<[Arc<str>]>,
}

impl Principal {
    /// Создаёт снимок пользователя и назначенных ему ролей.
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        role_ids: impl IntoIterator<Item = String>,
    ) -> Self {
        Self {
            id: Arc::from(id.into()),
            name: Arc::from(name.into()),
            role_ids: role_ids
                .into_iter()
                .map(Arc::<str>::from)
                .collect::<Vec<_>>()
                .into(),
        }
    }

    #[must_use]
    /// Возвращает идентификатор пользователя информационной базы.
    pub fn id(&self) -> &str {
        &self.id
    }

    #[must_use]
    /// Возвращает имя пользователя информационной базы.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Перечисляет идентификаторы назначенных ролей.
    pub fn role_ids(&self) -> impl Iterator<Item = &str> {
        self.role_ids.iter().map(AsRef::as_ref)
    }

    fn has_role(&self, id: &str) -> bool {
        self.role_ids
            .iter()
            .any(|candidate| candidate.as_ref() == id)
    }
}

/// Доступ к данным конфигурации в пределах одного сеанса.
///
/// Реализация принадлежит адаптеру источника данных. `bsl-orm` передаёт ей
/// метаданные объекта, неизменяемого пользователя сеанса и текущий контекст
/// вызова, через который доступны параметры того же сеанса.
pub trait DataAccess {
    /// Читает значение константы с учётом прав текущего пользователя.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку источника данных, проверки прав, RLS или
    /// инициализации требуемого параметра сеанса.
    fn constant_value(
        &self,
        metadata: &MetadataObject,
        principal: &Principal,
        context: &mut CallContext<'_>,
    ) -> RtResult<Value>;
}

/// Данные ORM, принадлежащие ровно одному `open_bsl::State`.
pub struct SessionData {
    catalog: Arc<MetadataCatalog>,
    principal: Principal,
    data_access: Option<Rc<dyn DataAccess>>,
    extension: Option<Rc<dyn Any>>,
}

impl SessionData {
    /// Создаёт неизменяемые данные ORM одного сеанса.
    #[must_use]
    pub fn new(catalog: Arc<MetadataCatalog>, principal: Principal) -> Self {
        Self {
            catalog,
            principal,
            data_access: None,
            extension: None,
        }
    }

    /// Подключает адаптер данных, принадлежащий этому сеансу.
    #[must_use]
    pub fn with_data_access(mut self, data_access: Rc<dyn DataAccess>) -> Self {
        self.data_access = Some(data_access);
        self
    }

    /// Добавляет сеансовые данные верхнего слоя, например контейнер
    /// параметров сеанса. Значение принадлежит тому же `State`.
    #[must_use]
    pub fn with_extension(mut self, extension: Rc<dyn Any>) -> Self {
        self.extension = Some(extension);
        self
    }

    #[must_use]
    /// Возвращает каталог конфигурации.
    pub fn catalog(&self) -> &Arc<MetadataCatalog> {
        &self.catalog
    }

    #[must_use]
    /// Возвращает снимок пользователя сеанса.
    pub const fn principal(&self) -> &Principal {
        &self.principal
    }

    /// Возвращает расширение указанного типа, если верхний слой его передал.
    #[must_use]
    pub fn extension<T: Any>(&self) -> Option<&T> {
        self.extension.as_deref()?.downcast_ref()
    }
}

impl fmt::Debug for SessionData {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SessionData")
            .field("catalog", &self.catalog)
            .field("principal", &self.principal)
            .field("has_data_access", &self.data_access.is_some())
            .field("has_extension", &self.extension.is_some())
            .finish()
    }
}

/// Ошибка целостности каталога метаданных.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CatalogError {
    /// Идентичность конфигурации пуста.
    EmptyConfigurationIdentity,
    /// Идентификатор объекта пуст.
    EmptyObjectIdentity,
    /// Имя объекта пусто.
    EmptyObjectName {
        /// Идентификатор объекта.
        object: String,
    },
    /// Идентификатор объекта повторяется.
    DuplicateObjectIdentity {
        /// Повторный идентификатор.
        object: String,
    },
    /// Имя объекта одного вида повторяется.
    DuplicateObjectName {
        /// Повторное имя.
        name: String,
    },
    /// Имя связанного модуля менеджера пусто.
    EmptyManagerModule {
        /// Идентификатор объекта.
        object: String,
    },
    /// Имя дополнительного свойства пусто.
    EmptyPropertyName {
        /// Идентификатор объекта.
        object: String,
    },
    /// Имя дополнительного свойства повторяется.
    DuplicatePropertyName {
        /// Идентификатор объекта.
        object: String,
        /// Повторное имя.
        property: String,
    },
    /// Имя конфигурации пусто.
    EmptyConfigurationName,
    /// Основная роль отсутствует или ссылается не на роль.
    UnknownMainRole(String),
    /// Основная роль повторяется.
    DuplicateMainRole(String),
    /// Имя расширения пусто.
    EmptyExtensionName,
    /// Идентификатор расширения повторяется.
    DuplicateExtensionIdentity,
    /// Имя расширения повторяется.
    DuplicateExtensionName {
        /// Повторное имя.
        name: String,
    },
    /// Набор ссылается на неизвестное расширение.
    UnknownExtensionIdentity,
    /// Расширение одновременно активно и отключено в сеансе.
    ExtensionBothAppliedAndDisabled,
    /// Проблема ссылается на неизвестное расширение.
    UnknownIssueExtension,
}

impl fmt::Display for CatalogError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyConfigurationIdentity => {
                formatter.write_str("идентичность конфигурации пуста")
            }
            Self::EmptyObjectIdentity => formatter.write_str("идентификатор объекта пуст"),
            Self::EmptyObjectName { object } => {
                write!(formatter, "имя объекта `{object}` пусто")
            }
            Self::DuplicateObjectIdentity { object } => {
                write!(formatter, "идентификатор объекта `{object}` повторяется")
            }
            Self::DuplicateObjectName { name } => {
                write!(formatter, "имя объекта `{name}` повторяется")
            }
            Self::EmptyManagerModule { object } => {
                write!(formatter, "имя модуля менеджера объекта `{object}` пусто")
            }
            Self::EmptyPropertyName { object } => {
                write!(formatter, "имя свойства объекта `{object}` пусто")
            }
            Self::DuplicatePropertyName { object, property } => write!(
                formatter,
                "свойство `{property}` объекта `{object}` повторяется"
            ),
            Self::EmptyConfigurationName => formatter.write_str("имя конфигурации пусто"),
            Self::UnknownMainRole(role) => {
                write!(formatter, "основная роль `{role}` отсутствует в метаданных")
            }
            Self::DuplicateMainRole(role) => {
                write!(formatter, "основная роль `{role}` повторяется")
            }
            Self::EmptyExtensionName => formatter.write_str("имя расширения пусто"),
            Self::DuplicateExtensionIdentity => {
                formatter.write_str("идентификатор расширения повторяется")
            }
            Self::DuplicateExtensionName { name } => {
                write!(formatter, "имя расширения `{name}` повторяется")
            }
            Self::UnknownExtensionIdentity => {
                formatter.write_str("набор сеанса ссылается на неизвестное расширение")
            }
            Self::ExtensionBothAppliedAndDisabled => {
                formatter.write_str("расширение одновременно активно и отключено в текущем сеансе")
            }
            Self::UnknownIssueExtension => {
                formatter.write_str("проблема ссылается на неизвестное расширение")
            }
        }
    }
}

impl Error for CatalogError {}

fn session<'a>(ctx: &'a CallContext<'_>) -> RtResult<&'a SessionData> {
    ctx.host_data()?
        .downcast_ref::<SessionData>()
        .ok_or_else(|| RtError::DynamicError("данные сеанса имеют неверный тип".into()))
}

#[derive(Debug)]
struct MetadataRoot {
    catalog: Arc<MetadataCatalog>,
}

#[derive(Debug)]
struct MetadataCollection {
    catalog: Arc<MetadataCatalog>,
    kind: MetadataKind,
}

#[derive(Debug)]
struct MetadataValue {
    catalog: Arc<MetadataCatalog>,
    object_id: Arc<str>,
}

#[derive(Debug)]
struct MainRoles {
    catalog: Arc<MetadataCatalog>,
}

#[derive(Debug)]
struct MetadataObjectProperties;

#[derive(Debug)]
struct ManagersRoot {
    catalog: Arc<MetadataCatalog>,
    kind: MetadataKind,
}

#[derive(Debug)]
struct ManagerValue {
    catalog: Arc<MetadataCatalog>,
    object_id: Arc<str>,
}

#[derive(Debug)]
struct ConfigurationExtensionsManager {
    catalog: Arc<MetadataCatalog>,
}

#[derive(Debug)]
struct ConfigurationExtensionValue {
    snapshot: Arc<ConfigurationExtensionsSnapshot>,
    index: usize,
}

#[derive(Debug)]
struct ExtensionApplicationIssueValue {
    snapshot: Arc<ConfigurationExtensionsSnapshot>,
    index: usize,
}

static METADATA_TYPE: TypeDescriptor =
    TypeDescriptor::new("bsl-orm", "ОбъектМетаданныхКонфигурация");
static METADATA_COLLECTION_TYPE: TypeDescriptor =
    TypeDescriptor::new("bsl-orm", "КоллекцияОбъектовМетаданных");
static METADATA_VALUE_TYPE: TypeDescriptor =
    TypeDescriptor::new("bsl-orm", "ОбъектМетаданныхКонфигурации");
static MAIN_ROLES_TYPE: TypeDescriptor =
    TypeDescriptor::new("bsl-orm", "КоллекцияЗначенийСвойстваОбъектаМетаданных");
static METADATA_OBJECT_PROPERTIES_TYPE: TypeDescriptor =
    TypeDescriptor::new("bsl-orm", "ПеречислимыеСвойстваОбъектовМетаданных");
static MANAGERS_TYPE: TypeDescriptor = TypeDescriptor::new("bsl-orm", "МенеджерыОбъектов");
static MANAGER_TYPE: TypeDescriptor = TypeDescriptor::new("bsl-orm", "МенеджерОбъекта");
static CONFIGURATION_EXTENSIONS_MANAGER_TYPE: TypeDescriptor =
    TypeDescriptor::new("bsl-orm", "МенеджерРасширенийКонфигурации");
static CONFIGURATION_EXTENSION_TYPE: TypeDescriptor =
    TypeDescriptor::new("bsl-orm", "РасширениеКонфигурации");
static EXTENSION_APPLICATION_ISSUE_TYPE: TypeDescriptor = TypeDescriptor::new(
    "bsl-orm",
    "ИнформацияОПроблемеПримененияРасширенияКонфигурации",
);

impl ObjectProtocol for MetadataRoot {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &METADATA_TYPE
    }

    fn get_property(&self, name: &str, context: &mut CallContext<'_>) -> RtResult<Value> {
        if let Some(kind) = collection_kind(name) {
            return Ok(Value::new_object(MetadataCollection {
                catalog: Arc::clone(&self.catalog),
                kind,
            }));
        }
        get_property_from_table(
            METADATA_ROOT_PROPERTIES,
            METADATA_TYPE.name,
            self,
            name,
            context,
        )
    }

    fn property_table(&self) -> &'static [PropertyDescriptor] {
        METADATA_ROOT_PROPERTIES
    }
}

impl ObjectProtocol for MetadataCollection {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &METADATA_COLLECTION_TYPE
    }

    fn get_property(&self, name: &str, _: &mut CallContext<'_>) -> RtResult<Value> {
        metadata_value(&self.catalog, self.kind, name)
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        METADATA_COLLECTION_METHODS
    }

    fn get_index(&self, index: &Value) -> RtResult<Value> {
        let Value::Number(index) = index else {
            return Err(RtError::BadIndex);
        };
        let index = index
            .to_i64_exact()
            .and_then(|index| usize::try_from(index).ok())
            .ok_or(RtError::BadIndex)?;
        let object = self
            .catalog
            .objects
            .iter()
            .filter(|object| object.kind == self.kind)
            .nth(index)
            .ok_or(RtError::IndexOutOfBounds {
                index: index as i64,
                len: self.collection_len()?,
            })?;
        Ok(metadata_object_value(&self.catalog, object))
    }

    fn collection_len(&self) -> RtResult<usize> {
        Ok(self
            .catalog
            .objects
            .iter()
            .filter(|object| object.kind == self.kind)
            .count())
    }
}

impl ObjectProtocol for MetadataValue {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &METADATA_VALUE_TYPE
    }

    fn property_table(&self) -> &'static [PropertyDescriptor] {
        METADATA_VALUE_PROPERTIES
    }

    fn get_property(&self, name: &str, context: &mut CallContext<'_>) -> RtResult<Value> {
        if folded_eq(name, "Имя") || folded_eq(name, "Name") {
            return get_property_from_table(
                METADATA_VALUE_PROPERTIES,
                METADATA_VALUE_TYPE.name,
                self,
                name,
                context,
            );
        }
        let object = self.catalog.object_by_id(&self.object_id).ok_or_else(|| {
            RtError::DynamicError("объект метаданных отсутствует в каталоге".into())
        })?;
        object
            .string_properties
            .iter()
            .find(|(property, _)| folded_eq(property, name))
            .map(|(_, value)| Value::Str(value.as_ref().into()))
            .ok_or_else(|| RtError::UnknownProperty(name.into()))
    }

    fn identity_key(&self) -> Option<(usize, usize)> {
        let index = self
            .catalog
            .objects
            .iter()
            .position(|object| object.id == self.object_id)?;
        Some((Arc::as_ptr(&self.catalog) as usize, index))
    }
}

impl ObjectProtocol for MainRoles {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &MAIN_ROLES_TYPE
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        MAIN_ROLES_METHODS
    }

    fn collection_len(&self) -> RtResult<usize> {
        Ok(self
            .catalog
            .configuration
            .as_ref()
            .map_or(0, |description| description.main_role_ids.len()))
    }

    fn get_index(&self, index: &Value) -> RtResult<Value> {
        let Value::Number(index) = index else {
            return Err(RtError::BadIndex);
        };
        let index = index
            .to_i64_exact()
            .and_then(|index| usize::try_from(index).ok())
            .ok_or(RtError::BadIndex)?;
        let description = self.catalog.configuration.as_ref().ok_or_else(|| {
            RtError::DynamicError("свойства конфигурации отсутствуют в каталоге метаданных".into())
        })?;
        let id = description
            .main_role_ids
            .get(index)
            .ok_or(RtError::IndexOutOfBounds {
                index: index as i64,
                len: description.main_role_ids.len(),
            })?;
        let object = self.catalog.object_by_id(id).ok_or_else(|| {
            RtError::DynamicError("основная роль отсутствует в каталоге метаданных".into())
        })?;
        Ok(metadata_object_value(&self.catalog, object))
    }
}

impl ObjectProtocol for MetadataObjectProperties {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &METADATA_OBJECT_PROPERTIES_TYPE
    }

    fn property_table(&self) -> &'static [PropertyDescriptor] {
        METADATA_OBJECT_PROPERTIES
    }
}

impl ObjectProtocol for ManagersRoot {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &MANAGERS_TYPE
    }

    fn get_property(&self, name: &str, _: &mut CallContext<'_>) -> RtResult<Value> {
        let object = self
            .catalog
            .object(self.kind, name)
            .ok_or_else(|| RtError::UnknownProperty(name.into()))?;
        Ok(Value::new_object(ManagerValue {
            catalog: Arc::clone(&self.catalog),
            object_id: Arc::clone(&object.id),
        }))
    }
}

impl ObjectProtocol for ManagerValue {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &MANAGER_TYPE
    }

    fn property_table(&self) -> &'static [PropertyDescriptor] {
        MANAGER_PROPERTIES
    }

    fn call_method(
        &self,
        name: &str,
        arguments: &[Value],
        context: &mut CallContext<'_>,
    ) -> RtResult<Value> {
        let object = self.catalog.object_by_id(&self.object_id).ok_or_else(|| {
            RtError::DynamicError("объект менеджера отсутствует в каталоге".into())
        })?;
        let methods = manager_methods(object.kind());
        if method_exists(methods, name) {
            return call_method_from_table(
                methods,
                MANAGER_TYPE.name,
                self,
                name,
                arguments,
                context,
            );
        }
        let module = object
            .manager_module()
            .ok_or_else(|| RtError::UnknownMethod {
                method: name.into(),
                receiver: MANAGER_TYPE.name,
            })?;
        context
            .call_configuration_module(
                module,
                name,
                arguments.to_vec(),
                Some(Value::new_object(ManagerValue {
                    catalog: Arc::clone(&self.catalog),
                    object_id: Arc::clone(&self.object_id),
                })),
            )
            .map(|(value, _)| value)
    }

    fn has_method(&self, name: &str) -> bool {
        self.catalog
            .object_by_id(&self.object_id)
            .is_some_and(|object| {
                method_exists(manager_methods(object.kind()), name)
                    || object.manager_module().is_some()
            })
    }
}

impl ObjectProtocol for ConfigurationExtensionsManager {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &CONFIGURATION_EXTENSIONS_MANAGER_TYPE
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        CONFIGURATION_EXTENSIONS_MANAGER_METHODS
    }
}

impl ObjectProtocol for ConfigurationExtensionValue {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &CONFIGURATION_EXTENSION_TYPE
    }

    fn property_table(&self) -> &'static [PropertyDescriptor] {
        CONFIGURATION_EXTENSION_PROPERTIES
    }

    fn identity_key(&self) -> Option<(usize, usize)> {
        Some((Arc::as_ptr(&self.snapshot) as usize, self.index))
    }
}

impl ObjectProtocol for ExtensionApplicationIssueValue {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &EXTENSION_APPLICATION_ISSUE_TYPE
    }

    fn property_table(&self) -> &'static [PropertyDescriptor] {
        EXTENSION_APPLICATION_ISSUE_PROPERTIES
    }
}

fn metadata_collection_find(
    receiver: &dyn ObjectProtocol,
    arguments: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let collection = receiver
        .downcast_ref::<MetadataCollection>()
        .ok_or(RtError::NotAnObject)?;
    let [Value::Str(wanted)] = arguments else {
        return Err(RtError::TypeError {
            expected: "Строка",
            op: "Найти",
        });
    };
    let wanted = wanted.to_string();
    Ok(collection
        .catalog
        .object(collection.kind, &wanted)
        .map(|object| metadata_object_value(&collection.catalog, object))
        .unwrap_or(Value::Undefined))
}

fn metadata_value_name(receiver: &dyn ObjectProtocol, _: &mut CallContext<'_>) -> RtResult<Value> {
    let value = receiver
        .downcast_ref::<MetadataValue>()
        .ok_or(RtError::NotAnObject)?;
    let object = value
        .catalog
        .object_by_id(&value.object_id)
        .ok_or_else(|| RtError::DynamicError("объект метаданных отсутствует в каталоге".into()))?;
    Ok(Value::Str(object.name().into()))
}

fn configuration_description(root: &MetadataRoot) -> RtResult<&ConfigurationDescription> {
    root.catalog.configuration.as_ref().ok_or_else(|| {
        RtError::DynamicError("свойства конфигурации отсутствуют в каталоге метаданных".into())
    })
}

fn metadata_root_name(receiver: &dyn ObjectProtocol, _: &mut CallContext<'_>) -> RtResult<Value> {
    let root = receiver
        .downcast_ref::<MetadataRoot>()
        .ok_or(RtError::NotAnObject)?;
    Ok(Value::Str(
        configuration_description(root)?.name.as_ref().into(),
    ))
}

fn metadata_root_version(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let root = receiver
        .downcast_ref::<MetadataRoot>()
        .ok_or(RtError::NotAnObject)?;
    Ok(Value::Str(
        configuration_description(root)?.version.as_ref().into(),
    ))
}

fn metadata_root_script_variant(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let root = receiver
        .downcast_ref::<MetadataRoot>()
        .ok_or(RtError::NotAnObject)?;
    Ok(configuration_description(root)?.script_variant.value())
}

fn metadata_root_main_roles(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let root = receiver
        .downcast_ref::<MetadataRoot>()
        .ok_or(RtError::NotAnObject)?;
    configuration_description(root)?;
    Ok(Value::new_object(MainRoles {
        catalog: Arc::clone(&root.catalog),
    }))
}

fn metadata_root_object_properties(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    receiver
        .downcast_ref::<MetadataRoot>()
        .ok_or(RtError::NotAnObject)?;
    Ok(Value::new_object(MetadataObjectProperties))
}

fn metadata_object_script_variants(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    receiver
        .downcast_ref::<MetadataObjectProperties>()
        .ok_or(RtError::NotAnObject)?;
    Ok(Value::EnumType(EnumKind::ScriptVariant))
}

fn main_roles_contains(
    receiver: &dyn ObjectProtocol,
    arguments: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let roles = receiver
        .downcast_ref::<MainRoles>()
        .ok_or(RtError::NotAnObject)?;
    let [argument] = arguments else {
        return Err(RtError::MethodNotApplicable {
            method: "Содержит",
            receiver: MAIN_ROLES_TYPE.name,
        });
    };
    let Some(role) = argument
        .object_ref()
        .and_then(|value| value.downcast_ref::<MetadataValue>())
        .filter(|value| Arc::ptr_eq(&value.catalog, &roles.catalog))
        .and_then(|value| roles.catalog.object_by_id(&value.object_id))
        .filter(|object| object.kind == MetadataKind::Role)
    else {
        return Ok(Value::Boolean(false));
    };
    let contains = roles
        .catalog
        .configuration
        .as_ref()
        .is_some_and(|description| {
            description
                .main_role_ids
                .iter()
                .any(|id| id.as_ref() == role.id())
        });
    Ok(Value::Boolean(contains))
}

fn manager_name(receiver: &dyn ObjectProtocol, _: &mut CallContext<'_>) -> RtResult<Value> {
    let manager = receiver
        .downcast_ref::<ManagerValue>()
        .ok_or(RtError::NotAnObject)?;
    let object = manager
        .catalog
        .object_by_id(&manager.object_id)
        .ok_or_else(|| RtError::DynamicError("объект менеджера отсутствует в каталоге".into()))?;
    Ok(Value::Str(object.name().into()))
}

fn constant_value(
    receiver: &dyn ObjectProtocol,
    _: &[Value],
    context: &mut CallContext<'_>,
) -> RtResult<Value> {
    let manager = receiver
        .downcast_ref::<ManagerValue>()
        .ok_or(RtError::NotAnObject)?;
    let metadata = manager
        .catalog
        .object_by_id(&manager.object_id)
        .cloned()
        .ok_or_else(|| RtError::DynamicError("объект менеджера отсутствует в каталоге".into()))?;
    let (principal, data_access) = {
        let session = session(context)?;
        let data_access = session.data_access.as_ref().ok_or_else(|| {
            RtError::DynamicError("доступ к данным конфигурации не подключён".into())
        })?;
        (session.principal.clone(), Rc::clone(data_access))
    };
    data_access.constant_value(&metadata, &principal, context)
}

fn extension_snapshot(
    manager: &ConfigurationExtensionsManager,
) -> RtResult<&Arc<ConfigurationExtensionsSnapshot>> {
    manager
        .catalog
        .configuration_extensions
        .as_ref()
        .ok_or_else(|| {
            RtError::DynamicError(
                "сведения о расширениях конфигурации не загружены для текущего сеанса".into(),
            )
        })
}

fn optional_structure_field(
    structure: &Value,
    names: &[&str],
    context: &mut CallContext<'_>,
) -> RtResult<Option<Value>> {
    let Value::Object(object) = structure else {
        return Err(RtError::TypeError {
            expected: "Структура",
            op: "РасширенияКонфигурации.Получить",
        });
    };
    if !matches!(object.as_ref(), BslObject::Structure(_)) {
        return Err(RtError::TypeError {
            expected: "Структура",
            op: "РасширенияКонфигурации.Получить",
        });
    }
    for name in names {
        let field = context.runtime_shapes().names.intern(name);
        match structure.get_field(field) {
            Ok(Value::Undefined) => return Ok(None),
            Ok(value) => return Ok(Some(value)),
            Err(RtError::UnknownField(_)) => {}
            Err(error) => return Err(error),
        }
    }
    Ok(None)
}

#[derive(Default)]
struct ConfigurationExtensionFilter {
    id: Option<[u8; 16]>,
    name: Option<String>,
}

fn configuration_extension_filter(
    value: &Value,
    context: &mut CallContext<'_>,
) -> RtResult<ConfigurationExtensionFilter> {
    if matches!(value, Value::Undefined) {
        return Ok(ConfigurationExtensionFilter::default());
    }
    let id = optional_structure_field(value, &["УникальныйИдентификатор", "UUID"], context)?
        .map(|value| match value {
            Value::Object(object) => match object.as_ref() {
                BslObject::Uuid(bytes) => Ok(*bytes),
                _ => Err(RtError::TypeError {
                    expected: "УникальныйИдентификатор",
                    op: "РасширенияКонфигурации.Получить",
                }),
            },
            _ => Err(RtError::TypeError {
                expected: "УникальныйИдентификатор",
                op: "РасширенияКонфигурации.Получить",
            }),
        })
        .transpose()?;
    let name = optional_structure_field(value, &["Имя", "Name"], context)?
        .map(|value| match value {
            Value::Str(name) => Ok(name.to_string()),
            _ => Err(RtError::TypeError {
                expected: "Строка",
                op: "РасширенияКонфигурации.Получить",
            }),
        })
        .transpose()?;
    Ok(ConfigurationExtensionFilter { id, name })
}

fn configuration_extension_source(value: Option<&Value>) -> RtResult<EnumValue> {
    match value {
        None | Some(Value::Undefined) => Ok(EnumValue::ConfigurationExtensionsSourceDatabase),
        Some(Value::Enum(
            source @ (EnumValue::ConfigurationExtensionsSourceDatabase
            | EnumValue::ConfigurationExtensionsSourceSessionApplied
            | EnumValue::ConfigurationExtensionsSourceSessionDisabled),
        )) => Ok(*source),
        Some(_) => Err(RtError::TypeError {
            expected: "ИсточникРасширенийКонфигурации",
            op: "РасширенияКонфигурации.Получить",
        }),
    }
}

fn configuration_extensions_get(
    receiver: &dyn ObjectProtocol,
    arguments: &[Value],
    context: &mut CallContext<'_>,
) -> RtResult<Value> {
    let manager = receiver
        .downcast_ref::<ConfigurationExtensionsManager>()
        .ok_or(RtError::NotAnObject)?;
    let filter =
        configuration_extension_filter(arguments.first().unwrap_or(&Value::Undefined), context)?;
    let source = configuration_extension_source(arguments.get(1))?;
    let snapshot = Arc::clone(extension_snapshot(manager)?);
    let indexes: Box<dyn Iterator<Item = usize>> = match source {
        EnumValue::ConfigurationExtensionsSourceDatabase => Box::new(0..snapshot.database.len()),
        EnumValue::ConfigurationExtensionsSourceSessionApplied => {
            Box::new(snapshot.session_applied.iter().copied())
        }
        EnumValue::ConfigurationExtensionsSourceSessionDisabled => {
            Box::new(snapshot.session_disabled.iter().copied())
        }
        _ => unreachable!("источник проверен перед выборкой"),
    };
    Ok(Value::new_array(
        indexes
            .filter(|&index| {
                let extension = &snapshot.database[index];
                filter.id.is_none_or(|id| extension.id == id)
                    && filter
                        .name
                        .as_deref()
                        .is_none_or(|name| folded_eq(&extension.name, name))
            })
            .map(|index| {
                Value::new_object(ConfigurationExtensionValue {
                    snapshot: Arc::clone(&snapshot),
                    index,
                })
            })
            .collect(),
    ))
}

fn configuration_extension_issues(
    receiver: &dyn ObjectProtocol,
    _: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let manager = receiver
        .downcast_ref::<ConfigurationExtensionsManager>()
        .ok_or(RtError::NotAnObject)?;
    let snapshot = Arc::clone(extension_snapshot(manager)?);
    Ok(Value::new_array(
        (0..snapshot.issues.len())
            .map(|index| {
                Value::new_object(ExtensionApplicationIssueValue {
                    snapshot: Arc::clone(&snapshot),
                    index,
                })
            })
            .collect(),
    ))
}

fn extension_value(receiver: &dyn ObjectProtocol) -> RtResult<&ConfigurationExtension> {
    let value = receiver
        .downcast_ref::<ConfigurationExtensionValue>()
        .ok_or(RtError::NotAnObject)?;
    value
        .snapshot
        .database
        .get(value.index)
        .ok_or_else(|| RtError::DynamicError("расширение отсутствует в снимке конфигурации".into()))
}

fn extension_id(receiver: &dyn ObjectProtocol, _: &mut CallContext<'_>) -> RtResult<Value> {
    Ok(Value::Object(Rc::new(BslObject::Uuid(
        extension_value(receiver)?.id,
    ))))
}

fn extension_name(receiver: &dyn ObjectProtocol, _: &mut CallContext<'_>) -> RtResult<Value> {
    Ok(Value::Str(extension_value(receiver)?.name.as_ref().into()))
}

fn extension_version(receiver: &dyn ObjectProtocol, _: &mut CallContext<'_>) -> RtResult<Value> {
    Ok(Value::Str(
        extension_value(receiver)?.version.as_ref().into(),
    ))
}

fn extension_hash_sum(receiver: &dyn ObjectProtocol, _: &mut CallContext<'_>) -> RtResult<Value> {
    Ok(Value::Str(
        extension_value(receiver)?.hash_sum.as_ref().into(),
    ))
}

fn extension_safe_mode(receiver: &dyn ObjectProtocol, _: &mut CallContext<'_>) -> RtResult<Value> {
    Ok(extension_value(receiver)?.safe_mode.value())
}

fn extension_used_in_distributed_infobase(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    Ok(Value::Boolean(
        extension_value(receiver)?.used_in_distributed_infobase,
    ))
}

fn extension_active(receiver: &dyn ObjectProtocol, _: &mut CallContext<'_>) -> RtResult<Value> {
    Ok(Value::Boolean(extension_value(receiver)?.active))
}

fn extension_scope(receiver: &dyn ObjectProtocol, _: &mut CallContext<'_>) -> RtResult<Value> {
    Ok(extension_value(receiver)?.scope.value())
}

fn extension_issue(
    receiver: &dyn ObjectProtocol,
) -> RtResult<(&ExtensionApplicationIssueValue, &ExtensionApplicationIssue)> {
    let value = receiver
        .downcast_ref::<ExtensionApplicationIssueValue>()
        .ok_or(RtError::NotAnObject)?;
    let issue = value.snapshot.issues.get(value.index).ok_or_else(|| {
        RtError::DynamicError("проблема применения отсутствует в снимке конфигурации".into())
    })?;
    Ok((value, issue))
}

fn extension_issue_extension(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let (value, issue) = extension_issue(receiver)?;
    let index = value
        .snapshot
        .database
        .iter()
        .position(|extension| extension.id == issue.extension_id)
        .ok_or_else(|| {
            RtError::DynamicError("расширение проблемы отсутствует в снимке конфигурации".into())
        })?;
    Ok(Value::new_object(ConfigurationExtensionValue {
        snapshot: Arc::clone(&value.snapshot),
        index,
    }))
}

fn extension_issue_description(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    Ok(Value::Str(
        extension_issue(receiver)?.1.description.as_ref().into(),
    ))
}

fn extension_issue_severity(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    Ok(extension_issue(receiver)?.1.severity.value())
}

static METADATA_COLLECTION_METHODS: &[MethodDescriptor] = &[MethodDescriptor::new(
    &["Найти", "Find"],
    Arity::exact(1),
    metadata_collection_find,
)];

static METADATA_ROOT_PROPERTIES: &[PropertyDescriptor] = &[
    PropertyDescriptor {
        names: &["Имя", "Name"],
        get: metadata_root_name,
        set: None,
    },
    PropertyDescriptor {
        names: &["Версия", "Version"],
        get: metadata_root_version,
        set: None,
    },
    PropertyDescriptor {
        names: &["ВариантВстроенногоЯзыка", "ScriptVariant"],
        get: metadata_root_script_variant,
        set: None,
    },
    PropertyDescriptor {
        names: &["ОсновныеРоли", "DefaultRoles"],
        get: metadata_root_main_roles,
        set: None,
    },
    PropertyDescriptor {
        names: &["СвойстваОбъектов", "ObjectProperties"],
        get: metadata_root_object_properties,
        set: None,
    },
];

static MAIN_ROLES_METHODS: &[MethodDescriptor] = &[MethodDescriptor::new(
    &["Содержит", "Contains"],
    Arity::exact(1),
    main_roles_contains,
)];

static METADATA_OBJECT_PROPERTIES: &[PropertyDescriptor] = &[PropertyDescriptor {
    names: &["ВариантВстроенногоЯзыка", "ScriptVariant"],
    get: metadata_object_script_variants,
    set: None,
}];

static METADATA_VALUE_PROPERTIES: &[PropertyDescriptor] = &[PropertyDescriptor {
    names: &["Имя", "Name"],
    get: metadata_value_name,
    set: None,
}];

static MANAGER_PROPERTIES: &[PropertyDescriptor] = &[PropertyDescriptor {
    names: &["Имя", "Name"],
    get: manager_name,
    set: None,
}];

static CONSTANT_MANAGER_METHODS: &[MethodDescriptor] = &[MethodDescriptor::new(
    &["Получить", "Get"],
    Arity::exact(0),
    constant_value,
)];

static CONFIGURATION_EXTENSIONS_MANAGER_METHODS: &[MethodDescriptor] = &[
    MethodDescriptor::new(
        &["Получить", "Get"],
        Arity::range(0, 2),
        configuration_extensions_get,
    ),
    MethodDescriptor::new(
        &[
            "ПолучитьИнформациюОПроблемахПримененияВСеансе",
            "GetSessionApplicationIssuesInformation",
        ],
        Arity::exact(0),
        configuration_extension_issues,
    ),
];

static CONFIGURATION_EXTENSION_PROPERTIES: &[PropertyDescriptor] = &[
    PropertyDescriptor {
        names: &["УникальныйИдентификатор", "UUID"],
        get: extension_id,
        set: None,
    },
    PropertyDescriptor {
        names: &["Имя", "Name"],
        get: extension_name,
        set: None,
    },
    PropertyDescriptor {
        names: &["Версия", "Version"],
        get: extension_version,
        set: None,
    },
    PropertyDescriptor {
        names: &["ХешСумма", "HashSum"],
        get: extension_hash_sum,
        set: None,
    },
    PropertyDescriptor {
        names: &["БезопасныйРежим", "SafeMode"],
        get: extension_safe_mode,
        set: None,
    },
    PropertyDescriptor {
        names: &[
            "ИспользуетсяВРаспределеннойИнформационнойБазе",
            "UsedInDistributedInfoBase",
        ],
        get: extension_used_in_distributed_infobase,
        set: None,
    },
    PropertyDescriptor {
        names: &["Активно", "Active"],
        get: extension_active,
        set: None,
    },
    PropertyDescriptor {
        names: &["ОбластьДействия", "Scope"],
        get: extension_scope,
        set: None,
    },
];

static EXTENSION_APPLICATION_ISSUE_PROPERTIES: &[PropertyDescriptor] = &[
    PropertyDescriptor {
        names: &["Расширение", "Extension"],
        get: extension_issue_extension,
        set: None,
    },
    PropertyDescriptor {
        names: &["Описание", "Description"],
        get: extension_issue_description,
        set: None,
    },
    PropertyDescriptor {
        names: &["Важность", "Severity"],
        get: extension_issue_severity,
        set: None,
    },
];

fn manager_methods(kind: MetadataKind) -> &'static [MethodDescriptor] {
    match kind {
        MetadataKind::Constant => CONSTANT_MANAGER_METHODS,
        _ => &[],
    }
}

fn method_exists(methods: &[MethodDescriptor], name: &str) -> bool {
    methods.iter().any(|method| {
        method
            .names()
            .iter()
            .any(|candidate| folded_eq(candidate, name))
    })
}

fn collection_kind(name: &str) -> Option<MetadataKind> {
    [
        ("Справочники", "Catalogs", MetadataKind::Catalog),
        (
            "РегистрыСведений",
            "InformationRegisters",
            MetadataKind::InformationRegister,
        ),
        ("Константы", "Constants", MetadataKind::Constant),
        (
            "ФункциональныеОпции",
            "FunctionalOptions",
            MetadataKind::FunctionalOption,
        ),
        ("ЭлементыСтиля", "StyleItems", MetadataKind::StyleItem),
        ("Роли", "Roles", MetadataKind::Role),
        (
            "ПараметрыСеанса",
            "SessionParameters",
            MetadataKind::SessionParameter,
        ),
        ("ОбщиеМодули", "CommonModules", MetadataKind::CommonModule),
        (
            "РегламентныеЗадания",
            "ScheduledJobs",
            MetadataKind::ScheduledJob,
        ),
    ]
    .into_iter()
    .find(|(ru, en, _)| folded_eq(name, ru) || folded_eq(name, en))
    .map(|(_, _, kind)| kind)
}

fn metadata_value(
    catalog: &Arc<MetadataCatalog>,
    kind: MetadataKind,
    name: &str,
) -> RtResult<Value> {
    let object = catalog
        .object(kind, name)
        .ok_or_else(|| RtError::UnknownProperty(name.into()))?;
    Ok(metadata_object_value(catalog, object))
}

fn metadata_object_value(catalog: &Arc<MetadataCatalog>, object: &MetadataObject) -> Value {
    Value::new_object(MetadataValue {
        catalog: Arc::clone(catalog),
        object_id: Arc::clone(&object.id),
    })
}

fn metadata(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    Ok(Value::new_object(MetadataRoot {
        catalog: Arc::clone(session(ctx)?.catalog()),
    }))
}

fn managers(ctx: &mut CallContext<'_>, kind: MetadataKind) -> RtResult<Value> {
    Ok(Value::new_object(ManagersRoot {
        catalog: Arc::clone(session(ctx)?.catalog()),
        kind,
    }))
}

fn catalogs(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    managers(ctx, MetadataKind::Catalog)
}

fn information_registers(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    managers(ctx, MetadataKind::InformationRegister)
}

fn constants(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    managers(ctx, MetadataKind::Constant)
}

fn configuration_extensions(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    Ok(Value::new_object(ConfigurationExtensionsManager {
        catalog: Arc::clone(session(ctx)?.catalog()),
    }))
}

fn role_available(ctx: &mut CallContext<'_>, arguments: &[Value]) -> RtResult<Value> {
    let [argument] = arguments else {
        return Err(RtError::DynamicError(
            "РольДоступна ожидает один аргумент".into(),
        ));
    };
    let session = session(ctx)?;
    let role = match argument {
        Value::Str(name) => {
            let name = name.to_string();
            session.catalog.object(MetadataKind::Role, &name)
        }
        value => value
            .object_ref()
            .and_then(|value| value.downcast_ref::<MetadataValue>())
            .and_then(|value| value.catalog.object_by_id(&value.object_id))
            .filter(|object| object.kind == MetadataKind::Role),
    }
    .ok_or_else(|| RtError::DynamicError("роль конфигурации не найдена".into()))?;
    Ok(Value::Boolean(session.principal.has_role(role.id())))
}

const FUNCTIONS: &[FunctionDescriptor] = &[
    FunctionDescriptor {
        code: FunctionCode::new(1),
        names: &["ПолучитьМетаданныеORM"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: metadata,
    },
    FunctionDescriptor {
        code: FunctionCode::new(2),
        names: &["ПолучитьСправочникиORM"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: catalogs,
    },
    FunctionDescriptor {
        code: FunctionCode::new(3),
        names: &["ПолучитьРегистрыСведенийORM"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: information_registers,
    },
    FunctionDescriptor {
        code: FunctionCode::new(4),
        names: &["ПолучитьКонстантыORM"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: constants,
    },
    FunctionDescriptor {
        code: FunctionCode::new(5),
        names: &["РольДоступна", "IsInRole"],
        arity: Arity::exact(1),
        kind: FunctionKind::Function,
        call: role_available,
    },
    FunctionDescriptor {
        code: FunctionCode::new(6),
        names: &["ПолучитьРасширенияКонфигурацииORM"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: configuration_extensions,
    },
];

const GLOBALS: &[GlobalDescriptor] = &[
    GlobalDescriptor {
        names: &["Метаданные", "Metadata"],
        function: FunctionCode::new(1),
    },
    GlobalDescriptor {
        names: &["Справочники", "Catalogs"],
        function: FunctionCode::new(2),
    },
    GlobalDescriptor {
        names: &["РегистрыСведений", "InformationRegisters"],
        function: FunctionCode::new(3),
    },
    GlobalDescriptor {
        names: &["Константы", "Constants"],
        function: FunctionCode::new(4),
    },
    GlobalDescriptor {
        names: &["РасширенияКонфигурации", "ConfigurationExtensions"],
        function: FunctionCode::new(6),
    },
];

const TYPES: &[&TypeDescriptor] = &[
    &METADATA_TYPE,
    &METADATA_COLLECTION_TYPE,
    &METADATA_VALUE_TYPE,
    &MAIN_ROLES_TYPE,
    &METADATA_OBJECT_PROPERTIES_TYPE,
    &MANAGERS_TYPE,
    &MANAGER_TYPE,
    &CONFIGURATION_EXTENSIONS_MANAGER_TYPE,
    &CONFIGURATION_EXTENSION_TYPE,
    &EXTENSION_APPLICATION_ISSUE_TYPE,
];

const API_MEMBERS: &[ObjectMembersDescriptor] = &[
    ObjectMembersDescriptor::new(&METADATA_TYPE)
        .with_properties(METADATA_ROOT_PROPERTIES)
        .with_dynamic_properties(),
    ObjectMembersDescriptor::new(&METADATA_COLLECTION_TYPE)
        .with_methods(METADATA_COLLECTION_METHODS)
        .with_dynamic_properties(),
    ObjectMembersDescriptor::new(&METADATA_VALUE_TYPE)
        .with_properties(METADATA_VALUE_PROPERTIES)
        .with_dynamic_properties(),
    ObjectMembersDescriptor::new(&MAIN_ROLES_TYPE).with_methods(MAIN_ROLES_METHODS),
    ObjectMembersDescriptor::new(&METADATA_OBJECT_PROPERTIES_TYPE)
        .with_properties(METADATA_OBJECT_PROPERTIES),
    ObjectMembersDescriptor::new(&MANAGERS_TYPE).with_dynamic_properties(),
    ObjectMembersDescriptor::new(&MANAGER_TYPE)
        .with_methods(CONSTANT_MANAGER_METHODS)
        .with_properties(MANAGER_PROPERTIES),
    ObjectMembersDescriptor::new(&CONFIGURATION_EXTENSIONS_MANAGER_TYPE)
        .with_methods(CONFIGURATION_EXTENSIONS_MANAGER_METHODS),
    ObjectMembersDescriptor::new(&CONFIGURATION_EXTENSION_TYPE)
        .with_properties(CONFIGURATION_EXTENSION_PROPERTIES),
    ObjectMembersDescriptor::new(&EXTENSION_APPLICATION_ISSUE_TYPE)
        .with_properties(EXTENSION_APPLICATION_ISSUE_PROPERTIES),
];

const OBJECT_MEMBER_GROUPS: &[&[ObjectMembersDescriptor]] = &[API_MEMBERS];

const LIBRARY: LibraryDescriptor = LibraryDescriptor::new("bsl-orm", env!("CARGO_PKG_VERSION"))
    .with_functions(FUNCTIONS)
    .with_globals(GLOBALS)
    .with_types(TYPES)
    .with_object_member_groups(OBJECT_MEMBER_GROUPS);

/// Возвращает runtime-библиотеку ORM для регистрации в `EngineBuilder`.
#[must_use]
pub const fn library() -> LibraryDescriptor {
    LIBRARY
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
