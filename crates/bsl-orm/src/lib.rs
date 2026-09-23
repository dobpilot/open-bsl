//! Объектная модель данных и метаданных конфигурации 1С.
//!
//! Крейт является runtime-компонентом для существующего интерпретатора
//! `open-bsl`. Он не разбирает и не исполняет BSL самостоятельно. Физический
//! доступ к данным будет внедряться данными сеанса; каталог ниже хранит только
//! неизменяемое описание конфигурации и связи с её BSL-модулями.

#![warn(missing_docs)]

use std::any::Any;
use std::cell::{Cell, RefCell};
use std::collections::HashSet;
use std::error::Error;
use std::fmt;
use std::rc::Rc;
use std::sync::Arc;

use open_bsl::{
    Arity, BslObject, CallContext, ConstructorCode, ConstructorDescriptor, EnumKind, EnumValue,
    FunctionCode, FunctionDescriptor, FunctionKind, GlobalDescriptor, GraphLimits,
    LibraryDescriptor, MethodDescriptor, ObjectMembersDescriptor, ObjectProtocol,
    PropertyDescriptor, RtError, RtResult, SerializedValueGraph, TypeDescriptor, Value,
    call_method_from_table, folded_eq, get_property_from_table, lookup_enum, lookup_member,
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

/// Режим обновления предопределённых данных информационной базы.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PredefinedDataUpdateMode {
    /// Режим определяется настройками объектов и узла распределённой базы.
    Auto,
    /// Предопределённые данные обновляются автоматически.
    AutoUpdate,
    /// Автоматическое обновление отключено.
    DontAutoUpdate,
}

impl PredefinedDataUpdateMode {
    fn value(self) -> Value {
        Value::Enum(match self {
            Self::Auto => EnumValue::PredefinedDataUpdateAuto,
            Self::AutoUpdate => EnumValue::PredefinedDataUpdateAutoUpdate,
            Self::DontAutoUpdate => EnumValue::PredefinedDataUpdateDontAutoUpdate,
        })
    }

    fn from_value(value: &Value) -> RtResult<Self> {
        match value {
            Value::Enum(EnumValue::PredefinedDataUpdateAuto) => Ok(Self::Auto),
            Value::Enum(EnumValue::PredefinedDataUpdateAutoUpdate) => Ok(Self::AutoUpdate),
            Value::Enum(EnumValue::PredefinedDataUpdateDontAutoUpdate) => Ok(Self::DontAutoUpdate),
            _ => Err(RtError::TypeError {
                expected: "ОбновлениеПредопределенныхДанных",
                op: "УстановитьОбновлениеПредопределенныхДанныхИнформационнойБазы",
            }),
        }
    }
}

/// Режим совместимости конфигурации с версией платформы.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompatibilityMode {
    /// Совместимость с 8.1.
    Version8_1,
    /// Совместимость с 8.2.13.
    Version8_2_13,
    /// Совместимость с 8.2.16.
    Version8_2_16,
    /// Совместимость с 8.3.1.
    Version8_3_1,
    /// Совместимость с 8.3.2.
    Version8_3_2,
    /// Совместимость с 8.3.3.
    Version8_3_3,
    /// Совместимость с 8.3.4.
    Version8_3_4,
    /// Совместимость с 8.3.5.
    Version8_3_5,
    /// Совместимость с 8.3.6.
    Version8_3_6,
    /// Совместимость с 8.3.7.
    Version8_3_7,
    /// Совместимость с 8.3.8.
    Version8_3_8,
    /// Совместимость с 8.3.9.
    Version8_3_9,
    /// Совместимость с 8.3.10.
    Version8_3_10,
    /// Совместимость с 8.3.11.
    Version8_3_11,
    /// Совместимость с 8.3.12.
    Version8_3_12,
    /// Совместимость с 8.3.13.
    Version8_3_13,
    /// Совместимость с 8.3.14.
    Version8_3_14,
    /// Совместимость с 8.3.15.
    Version8_3_15,
    /// Совместимость с 8.3.16.
    Version8_3_16,
    /// Совместимость с 8.3.17.
    Version8_3_17,
    /// Совместимость с 8.3.18.
    Version8_3_18,
    /// Совместимость с 8.3.19.
    Version8_3_19,
    /// Совместимость с 8.3.20.
    Version8_3_20,
    /// Совместимость с 8.3.21.
    Version8_3_21,
    /// Совместимость с 8.3.22.
    Version8_3_22,
    /// Совместимость с 8.3.23.
    Version8_3_23,
    /// Совместимость с 8.3.24.
    Version8_3_24,
    /// Совместимость с 8.3.25.
    Version8_3_25,
    /// Совместимость с 8.3.26.
    Version8_3_26,
    /// Ограничение совместимости не используется.
    DontUse,
}

impl CompatibilityMode {
    fn value(self) -> Value {
        Value::Enum(match self {
            Self::Version8_1 => EnumValue::CompatibilityModeVersion8_1,
            Self::Version8_2_13 => EnumValue::CompatibilityModeVersion8_2_13,
            Self::Version8_2_16 => EnumValue::CompatibilityModeVersion8_2_16,
            Self::Version8_3_1 => EnumValue::CompatibilityModeVersion8_3_1,
            Self::Version8_3_2 => EnumValue::CompatibilityModeVersion8_3_2,
            Self::Version8_3_3 => EnumValue::CompatibilityModeVersion8_3_3,
            Self::Version8_3_4 => EnumValue::CompatibilityModeVersion8_3_4,
            Self::Version8_3_5 => EnumValue::CompatibilityModeVersion8_3_5,
            Self::Version8_3_6 => EnumValue::CompatibilityModeVersion8_3_6,
            Self::Version8_3_7 => EnumValue::CompatibilityModeVersion8_3_7,
            Self::Version8_3_8 => EnumValue::CompatibilityModeVersion8_3_8,
            Self::Version8_3_9 => EnumValue::CompatibilityModeVersion8_3_9,
            Self::Version8_3_10 => EnumValue::CompatibilityModeVersion8_3_10,
            Self::Version8_3_11 => EnumValue::CompatibilityModeVersion8_3_11,
            Self::Version8_3_12 => EnumValue::CompatibilityModeVersion8_3_12,
            Self::Version8_3_13 => EnumValue::CompatibilityModeVersion8_3_13,
            Self::Version8_3_14 => EnumValue::CompatibilityModeVersion8_3_14,
            Self::Version8_3_15 => EnumValue::CompatibilityModeVersion8_3_15,
            Self::Version8_3_16 => EnumValue::CompatibilityModeVersion8_3_16,
            Self::Version8_3_17 => EnumValue::CompatibilityModeVersion8_3_17,
            Self::Version8_3_18 => EnumValue::CompatibilityModeVersion8_3_18,
            Self::Version8_3_19 => EnumValue::CompatibilityModeVersion8_3_19,
            Self::Version8_3_20 => EnumValue::CompatibilityModeVersion8_3_20,
            Self::Version8_3_21 => EnumValue::CompatibilityModeVersion8_3_21,
            Self::Version8_3_22 => EnumValue::CompatibilityModeVersion8_3_22,
            Self::Version8_3_23 => EnumValue::CompatibilityModeVersion8_3_23,
            Self::Version8_3_24 => EnumValue::CompatibilityModeVersion8_3_24,
            Self::Version8_3_25 => EnumValue::CompatibilityModeVersion8_3_25,
            Self::Version8_3_26 => EnumValue::CompatibilityModeVersion8_3_26,
            Self::DontUse => EnumValue::CompatibilityModeDontUse,
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
    /// Язык конфигурации.
    Language,
    /// Перечисление конфигурации.
    Enumeration,
    /// Общая картинка.
    CommonPicture,
    /// Справочник.
    Catalog,
    /// Документ.
    Document,
    /// Журнал документов.
    DocumentJournal,
    /// Регистр сведений.
    InformationRegister,
    /// Регистр накопления.
    AccumulationRegister,
    /// Регистр бухгалтерии.
    AccountingRegister,
    /// Регистр расчёта.
    CalculationRegister,
    /// План счетов.
    ChartOfAccounts,
    /// План видов характеристик.
    ChartOfCharacteristicTypes,
    /// План видов расчёта.
    ChartOfCalculationTypes,
    /// План обмена.
    ExchangePlan,
    /// Константа.
    Constant,
    /// Отчёт.
    Report,
    /// Обработка.
    DataProcessor,
    /// Бизнес-процесс.
    BusinessProcess,
    /// Задача.
    Task,
    /// Хранилище настроек.
    SettingsStorage,
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

/// Неизменяемое описание значения конфигурационного перечисления.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigurationEnumValue {
    owner_id: Arc<str>,
    value_id: Arc<str>,
    name: Arc<str>,
}

impl ConfigurationEnumValue {
    /// Создаёт описание из устойчивых идентификаторов приобретённого снимка.
    #[must_use]
    pub fn new(
        owner_id: impl Into<String>,
        value_id: impl Into<String>,
        name: impl Into<String>,
    ) -> Self {
        Self {
            owner_id: Arc::from(owner_id.into()),
            value_id: Arc::from(value_id.into()),
            name: Arc::from(name.into()),
        }
    }

    /// Возвращает идентификатор объекта-перечисления.
    #[must_use]
    pub fn owner_id(&self) -> &str {
        &self.owner_id
    }

    /// Возвращает идентификатор значения перечисления.
    #[must_use]
    pub fn value_id(&self) -> &str {
        &self.value_id
    }

    /// Возвращает имя значения в конфигурации.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }
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
    configuration_changed: Option<bool>,
    enumeration_values: Arc<[ConfigurationEnumValue]>,
}

#[derive(Debug)]
struct ConfigurationDescription {
    name: Arc<str>,
    version: Arc<str>,
    script_variant: ScriptVariant,
    compatibility_mode: CompatibilityMode,
    main_role_ids: Arc<[Arc<str>]>,
    primary_language_id: Option<Arc<str>>,
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
            configuration_changed: None,
            enumeration_values: Arc::from([]),
        })
    }

    /// Добавляет значения конфигурационных перечислений из того же снимка.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку для пустого идентификатора или имени, неизвестного
    /// владельца, владельца другого вида и повторного имени у владельца.
    pub fn with_enumeration_values(
        mut self,
        values: Vec<ConfigurationEnumValue>,
    ) -> Result<Self, CatalogError> {
        for (index, value) in values.iter().enumerate() {
            if value.value_id.is_empty() || value.name.is_empty() {
                return Err(CatalogError::InvalidEnumerationValue);
            }
            let owner = self
                .object_by_id(&value.owner_id)
                .filter(|owner| owner.kind == MetadataKind::Enumeration)
                .ok_or_else(|| CatalogError::UnknownEnumerationOwner(value.owner_id.to_string()))?;
            if values[..index]
                .iter()
                .any(|known| known.owner_id == owner.id && folded_eq(&known.name, &value.name))
            {
                return Err(CatalogError::DuplicateEnumerationValue {
                    owner: owner.name.to_string(),
                    name: value.name.to_string(),
                });
            }
        }
        self.enumeration_values = values.into();
        Ok(self)
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
        compatibility_mode: CompatibilityMode,
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
            compatibility_mode,
            main_role_ids: roles.into(),
            primary_language_id: None,
        });
        Ok(self)
    }

    /// Связывает свойства конфигурации с её основным языком.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку, если идентификатор отсутствует в каталоге или
    /// обозначает объект другого вида.
    pub fn with_primary_language(mut self, id: impl Into<String>) -> Result<Self, CatalogError> {
        let id = id.into();
        let language_id = Arc::clone(
            &self
                .object_by_id(&id)
                .filter(|object| object.kind == MetadataKind::Language)
                .ok_or_else(|| CatalogError::UnknownPrimaryLanguage(id.clone()))?
                .id,
        );
        let description = self
            .configuration
            .as_mut()
            .ok_or(CatalogError::UnknownPrimaryLanguage(id))?;
        description.primary_language_id = Some(language_id);
        Ok(self)
    }

    /// Возвращает идентификатор основного языка конфигурации.
    #[must_use]
    pub fn primary_language_id(&self) -> Option<&str> {
        self.configuration.as_ref()?.primary_language_id.as_deref()
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

    /// Добавляет признак различия основной конфигурации и конфигурации базы.
    #[must_use]
    pub const fn with_configuration_changed(mut self, changed: bool) -> Self {
        self.configuration_changed = Some(changed);
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

    /// Находит значение перечисления по владельцу и регистронезависимому имени.
    #[must_use]
    pub fn enumeration_value(&self, owner_id: &str, name: &str) -> Option<&ConfigurationEnumValue> {
        self.enumeration_values
            .iter()
            .find(|value| value.owner_id.as_ref() == owner_id && folded_eq(&value.name, name))
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
    interface_language_id: Option<Arc<str>>,
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
            interface_language_id: None,
        }
    }

    /// Сохраняет выбранный для пользователя язык интерфейса.
    #[must_use]
    pub fn with_interface_language_id(mut self, id: impl Into<String>) -> Self {
        self.interface_language_id = Some(Arc::from(id.into()));
        self
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

    /// Возвращает идентификатор выбранного языка интерфейса.
    #[must_use]
    pub fn interface_language_id(&self) -> Option<&str> {
        self.interface_language_id.as_deref()
    }

    fn has_role(&self, id: &str) -> bool {
        self.role_ids
            .iter()
            .any(|candidate| candidate.as_ref() == id)
    }
}

/// Неизменяемый запрос к адаптеру данных текущего сеанса.
#[derive(Debug, Clone)]
pub struct QueryRequest {
    text: Rc<str>,
    parameters: Rc<[(Rc<str>, Value)]>,
    temp_tables_manager: Option<TempTablesManagerHandle>,
}

/// Запрос поиска ссылок в данных текущего сеанса.
#[derive(Debug, Clone)]
pub struct FindByRefRequest {
    references: Value,
    search_area: Option<Value>,
    include_objects: Option<Value>,
    exclude_objects: Option<Value>,
}

impl FindByRefRequest {
    /// Возвращает массив искомых ссылок.
    #[must_use]
    pub const fn references(&self) -> &Value {
        &self.references
    }

    /// Возвращает явно заданную область поиска.
    #[must_use]
    pub const fn search_area(&self) -> Option<&Value> {
        self.search_area.as_ref()
    }

    /// Возвращает явно включённые объекты метаданных.
    #[must_use]
    pub const fn include_objects(&self) -> Option<&Value> {
        self.include_objects.as_ref()
    }

    /// Возвращает явно исключённые объекты метаданных.
    #[must_use]
    pub const fn exclude_objects(&self) -> Option<&Value> {
        self.exclude_objects.as_ref()
    }
}

/// Непрозрачный идентификатор менеджера временных таблиц у адаптера данных.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TempTablesManagerHandle(u64);

impl TempTablesManagerHandle {
    /// Создаёт идентификатор из значения, уникального в пределах соединения.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Возвращает значение для сопоставления с ресурсом адаптера.
    #[must_use]
    pub const fn value(self) -> u64 {
        self.0
    }
}

/// Неизменяемый запрос значения функциональной опции.
#[derive(Debug, Clone)]
pub struct FunctionalOptionRequest {
    name: Rc<str>,
    parameters: Rc<[(Rc<str>, Value)]>,
}

/// Именованное пространство управляемой блокировки данных.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataLockRequest {
    lock_space: Rc<str>,
}

/// Чей набор ролей проверяет [`DataAccess::access_right`].
#[derive(Debug, Clone)]
pub enum AccessRightSubject {
    /// Пользователь текущего сеанса.
    CurrentPrincipal,
    /// Явно переданный пользователь информационной базы.
    Principal(Principal),
    /// Явно переданная роль конфигурации.
    Role(Arc<str>),
}

/// Неизменяемый запрос проверки права на объект метаданных.
#[derive(Debug, Clone)]
pub struct AccessRightRequest {
    right: Rc<str>,
    metadata: MetadataObject,
    subject: AccessRightSubject,
    standard_attribute: Option<Rc<str>>,
}

impl AccessRightRequest {
    /// Возвращает имя проверяемого права.
    #[must_use]
    pub fn right(&self) -> &str {
        &self.right
    }

    /// Возвращает объект метаданных, для которого проверяется право.
    #[must_use]
    pub const fn metadata(&self) -> &MetadataObject {
        &self.metadata
    }

    /// Возвращает явно выбранный субъект либо текущего пользователя.
    #[must_use]
    pub const fn subject(&self) -> &AccessRightSubject {
        &self.subject
    }

    /// Возвращает путь стандартного реквизита или табличной части.
    #[must_use]
    pub fn standard_attribute(&self) -> Option<&str> {
        self.standard_attribute.as_deref()
    }
}

impl DataLockRequest {
    /// Возвращает имя пространства блокировок в синтаксисе платформы.
    #[must_use]
    pub fn lock_space(&self) -> &str {
        &self.lock_space
    }
}

impl QueryRequest {
    /// Возвращает текст запроса BSL.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Перечисляет окончательные значения параметров в порядке их первой
    /// установки.
    pub fn parameters(&self) -> impl Iterator<Item = (&str, &Value)> {
        self.parameters
            .iter()
            .map(|(name, value)| (name.as_ref(), value))
    }

    /// Возвращает менеджер временных таблиц, назначенный запросу.
    #[must_use]
    pub const fn temp_tables_manager(&self) -> Option<TempTablesManagerHandle> {
        self.temp_tables_manager
    }
}

impl FunctionalOptionRequest {
    /// Возвращает имя функциональной опции в языке текущей конфигурации.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Перечисляет переданные параметры в порядке полей исходной структуры.
    pub fn parameters(&self) -> impl Iterator<Item = (&str, &Value)> {
        self.parameters
            .iter()
            .map(|(name, value)| (name.as_ref(), value))
    }
}

/// Именованные строки, которые адаптер данных вернул для одного запроса.
#[derive(Debug, Clone)]
pub struct QueryResultData {
    columns: Rc<[Rc<str>]>,
    rows: Rc<[Rc<[Value]>]>,
}

impl QueryResultData {
    /// Проверяет и создаёт линейный результат запроса.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку для пустого или повторного имени колонки и строки,
    /// ширина которой не совпадает с числом колонок.
    pub fn new(
        columns: impl IntoIterator<Item = String>,
        rows: impl IntoIterator<Item = Vec<Value>>,
    ) -> Result<Self, QueryResultDataError> {
        let columns = columns.into_iter().map(Rc::<str>::from).collect::<Vec<_>>();
        for (index, name) in columns.iter().enumerate() {
            if name.is_empty() {
                return Err(QueryResultDataError::EmptyColumnName { index });
            }
            if columns[..index]
                .iter()
                .any(|candidate| folded_eq(candidate, name))
            {
                return Err(QueryResultDataError::DuplicateColumnName {
                    name: name.to_string(),
                });
            }
        }
        let rows = rows
            .into_iter()
            .enumerate()
            .map(|(index, row)| {
                if row.len() != columns.len() {
                    return Err(QueryResultDataError::RowWidth {
                        row: index,
                        expected: columns.len(),
                        actual: row.len(),
                    });
                }
                Ok(Rc::<[Value]>::from(row))
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            columns: columns.into(),
            rows: rows.into(),
        })
    }

    /// Возвращает имена колонок в порядке результата.
    #[must_use]
    pub fn columns(&self) -> &[Rc<str>] {
        &self.columns
    }

    /// Возвращает строки в порядке результата.
    #[must_use]
    pub fn rows(&self) -> &[Rc<[Value]>] {
        &self.rows
    }
}

/// Ошибка формы результата, возвращённого адаптером данных.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryResultDataError {
    /// Имя колонки пусто.
    EmptyColumnName {
        /// Номер колонки.
        index: usize,
    },
    /// Имя колонки повторяется без учёта регистра.
    DuplicateColumnName {
        /// Повторившееся имя.
        name: String,
    },
    /// Число значений строки не совпадает с числом колонок.
    RowWidth {
        /// Номер строки.
        row: usize,
        /// Ожидаемое число значений.
        expected: usize,
        /// Фактическое число значений.
        actual: usize,
    },
}

impl fmt::Display for QueryResultDataError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyColumnName { index } => write!(formatter, "имя колонки {index} пусто"),
            Self::DuplicateColumnName { name } => {
                write!(formatter, "имя колонки `{name}` повторяется")
            }
            Self::RowWidth {
                row,
                expected,
                actual,
            } => write!(
                formatter,
                "строка {row} содержит {actual} значений вместо {expected}"
            ),
        }
    }
}

impl Error for QueryResultDataError {}

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

    /// Выполняет запрос с учётом прав и RLS текущего пользователя.
    ///
    /// Реализация получает текущий [`CallContext`], чтобы чтение требуемого
    /// параметра сеанса прошло через его обычную ленивую инициализацию.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку источника данных, проверки прав, RLS, компиляции
    /// запроса или инициализации параметра сеанса. Реализация по умолчанию
    /// явно отвергает запросы.
    fn execute_query(
        &self,
        _request: &QueryRequest,
        _principal: &Principal,
        _context: &mut CallContext<'_>,
    ) -> RtResult<QueryResultData> {
        Err(RtError::DynamicError(
            "выполнение запросов не поддерживается адаптером данных".into(),
        ))
    }

    /// Ищет ссылки в доступных текущему пользователю данных.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку источника данных, прав, разделения данных или
    /// неподдерживаемой операции.
    fn find_by_ref(
        &self,
        _request: &FindByRefRequest,
        _principal: &Principal,
        _context: &mut CallContext<'_>,
    ) -> RtResult<QueryResultData> {
        Err(RtError::DynamicError(
            "поиск по ссылкам не поддерживается адаптером данных".into(),
        ))
    }

    /// Возвращает значение функциональной опции для текущего сеанса.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку неизвестной опции, неполного набора параметров,
    /// источника данных, проверки прав или неподдерживаемой операции.
    fn functional_option(
        &self,
        _request: &FunctionalOptionRequest,
        _principal: &Principal,
        _context: &mut CallContext<'_>,
    ) -> RtResult<Value> {
        Err(RtError::DynamicError(
            "функциональные опции не поддерживаются адаптером данных".into(),
        ))
    }

    /// Создаёт менеджер временных таблиц на соединении текущего сеанса.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку источника данных, соединения или неподдерживаемой
    /// операции.
    fn create_temp_tables_manager(
        &self,
        _principal: &Principal,
        _context: &mut CallContext<'_>,
    ) -> RtResult<TempTablesManagerHandle> {
        Err(RtError::DynamicError(
            "временные таблицы не поддерживаются адаптером данных".into(),
        ))
    }

    /// Закрывает менеджер и удаляет принадлежащие ему временные таблицы.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку неизвестного идентификатора, соединения, удаления
    /// таблиц или неподдерживаемой операции.
    fn close_temp_tables_manager(
        &self,
        _manager: TempTablesManagerHandle,
        _principal: &Principal,
        _context: &mut CallContext<'_>,
    ) -> RtResult<()> {
        Err(RtError::DynamicError(
            "временные таблицы не поддерживаются адаптером данных".into(),
        ))
    }

    /// Проверяет эффективное право на объект метаданных.
    ///
    /// Адаптер применяет иерархию прав ролей и самостоятельно проверяет
    /// административное право текущего пользователя для явно указанного
    /// субъекта.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку отсутствующих данных ролей, неизвестного права,
    /// неприменимого объекта либо недостаточного административного права.
    fn access_right(
        &self,
        _request: &AccessRightRequest,
        _principal: &Principal,
        _context: &mut CallContext<'_>,
    ) -> RtResult<bool> {
        Err(RtError::DynamicError(
            "проверка прав метаданных не поддерживается адаптером данных".into(),
        ))
    }

    /// Устанавливает управляемые блокировки на соединении текущего сеанса.
    /// Адаптер отвечает за транзакцию, время жизни и снятие блокировок.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку источника данных, проверки прав, транзакции или
    /// неподдерживаемого механизма блокировок.
    fn lock_data(
        &self,
        _requests: &[DataLockRequest],
        _principal: &Principal,
        _context: &mut CallContext<'_>,
    ) -> RtResult<()> {
        Err(RtError::DynamicError(
            "управляемые блокировки данных не поддерживаются адаптером данных".into(),
        ))
    }

    /// Начинает транзакцию на соединении текущего сеанса.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку источника данных или неподдерживаемой транзакции.
    fn begin_transaction(
        &self,
        _principal: &Principal,
        _context: &mut CallContext<'_>,
    ) -> RtResult<()> {
        Err(RtError::DynamicError(
            "транзакции не поддерживаются адаптером данных".into(),
        ))
    }

    /// Фиксирует текущую транзакцию соединения сеанса.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку источника данных, состояния или фиксации транзакции.
    fn commit_transaction(
        &self,
        _principal: &Principal,
        _context: &mut CallContext<'_>,
    ) -> RtResult<()> {
        Err(RtError::DynamicError(
            "транзакции не поддерживаются адаптером данных".into(),
        ))
    }

    /// Отменяет текущую транзакцию соединения сеанса.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку источника данных, состояния или отката транзакции.
    fn rollback_transaction(
        &self,
        _principal: &Principal,
        _context: &mut CallContext<'_>,
    ) -> RtResult<()> {
        Err(RtError::DynamicError(
            "транзакции не поддерживаются адаптером данных".into(),
        ))
    }

    /// Возвращает состояние транзакции на соединении текущего сеанса.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку источника данных или неподдерживаемой транзакции.
    fn transaction_active(
        &self,
        _principal: &Principal,
        _context: &mut CallContext<'_>,
    ) -> RtResult<bool> {
        Err(RtError::DynamicError(
            "транзакции не поддерживаются адаптером данных".into(),
        ))
    }

    /// Возвращает состояние монопольного режима текущего соединения.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку источника данных или неподдерживаемой
    /// операции.
    fn exclusive_mode(
        &self,
        _principal: &Principal,
        _context: &mut CallContext<'_>,
    ) -> RtResult<bool> {
        Err(RtError::DynamicError(
            "монопольный режим не поддерживается адаптером данных".into(),
        ))
    }

    /// Изменяет монопольный режим текущего соединения.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку источника данных, прав, конкурирующего сеанса
    /// или неподдерживаемой операции.
    fn set_exclusive_mode(
        &self,
        _enabled: bool,
        _principal: &Principal,
        _context: &mut CallContext<'_>,
    ) -> RtResult<()> {
        Err(RtError::DynamicError(
            "монопольный режим не поддерживается адаптером данных".into(),
        ))
    }

    /// Запускает или останавливает обработку сервисов интеграции.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку источника данных, прав, разделителей сеанса или
    /// неподдерживаемой операции.
    fn set_integration_services_processing(
        &self,
        _enabled: bool,
        _principal: &Principal,
        _context: &mut CallContext<'_>,
    ) -> RtResult<()> {
        Err(RtError::DynamicError(
            "обработка сервисов интеграции не поддерживается адаптером данных".into(),
        ))
    }

    /// Возвращает менеджер именованного сервиса интеграции.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку неизвестного сервиса, источника данных, прав или
    /// неподдерживаемой операции.
    fn integration_service(
        &self,
        _name: &str,
        _principal: &Principal,
        _context: &mut CallContext<'_>,
    ) -> RtResult<Value> {
        Err(RtError::DynamicError(
            "сервисы интеграции не поддерживаются адаптером данных".into(),
        ))
    }

    /// Возвращает режим обновления предопределённых данных информационной базы.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку источника данных, прав или неподдерживаемой операции.
    fn info_base_predefined_data_update(
        &self,
        _principal: &Principal,
        _context: &mut CallContext<'_>,
    ) -> RtResult<PredefinedDataUpdateMode> {
        Err(RtError::DynamicError(
            "режим обновления предопределённых данных не поддерживается адаптером данных".into(),
        ))
    }

    /// Изменяет режим обновления предопределённых данных информационной базы.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку источника данных, административных прав,
    /// разделителей сеанса или неподдерживаемой операции.
    fn set_info_base_predefined_data_update(
        &self,
        _mode: PredefinedDataUpdateMode,
        _principal: &Principal,
        _context: &mut CallContext<'_>,
    ) -> RtResult<()> {
        Err(RtError::DynamicError(
            "режим обновления предопределённых данных не поддерживается адаптером данных".into(),
        ))
    }

    /// Инициализирует предопределённые данные текущей области базы.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку источника данных, прав, разделителей сеанса или
    /// неподдерживаемой операции.
    fn initialize_predefined_data(
        &self,
        _principal: &Principal,
        _context: &mut CallContext<'_>,
    ) -> RtResult<()> {
        Err(RtError::DynamicError(
            "инициализация предопределённых данных не поддерживается адаптером данных".into(),
        ))
    }
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
    /// Основной язык отсутствует или ссылается не на язык.
    UnknownPrimaryLanguage(String),
    /// Имя расширения пусто.
    EmptyExtensionName,
    /// Идентификатор расширения повторяется.
    DuplicateExtensionIdentity,
    /// Имя расширения повторяется.
    DuplicateExtensionName {
        /// Повторное имя.
        name: String,
    },
    /// Идентификатор или имя значения перечисления пусы.
    InvalidEnumerationValue,
    /// Владелец значения отсутствует или не является перечислением.
    UnknownEnumerationOwner(String),
    /// Имя значения повторяется у одного перечисления.
    DuplicateEnumerationValue {
        /// Имя перечисления.
        owner: String,
        /// Повторившееся имя значения.
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
            Self::UnknownPrimaryLanguage(language) => {
                write!(
                    formatter,
                    "основной язык `{language}` отсутствует в метаданных"
                )
            }
            Self::EmptyExtensionName => formatter.write_str("имя расширения пусто"),
            Self::DuplicateExtensionIdentity => {
                formatter.write_str("идентификатор расширения повторяется")
            }
            Self::DuplicateExtensionName { name } => {
                write!(formatter, "имя расширения `{name}` повторяется")
            }
            Self::InvalidEnumerationValue => {
                formatter.write_str("идентификатор или имя значения перечисления пусы")
            }
            Self::UnknownEnumerationOwner(owner) => write!(
                formatter,
                "владелец значения перечисления `{owner}` отсутствует или имеет другой вид"
            ),
            Self::DuplicateEnumerationValue { owner, name } => write!(
                formatter,
                "значение `{name}` перечисления `{owner}` повторяется"
            ),
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

fn current_data_access(context: &CallContext<'_>) -> RtResult<(Principal, Rc<dyn DataAccess>)> {
    let session = session(context)?;
    let data_access = session.data_access.as_ref().ok_or_else(|| {
        RtError::DynamicError("адаптер данных текущего сеанса не подключён".into())
    })?;
    Ok((session.principal.clone(), Rc::clone(data_access)))
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
struct CompatibilityModeProperty;

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
struct EnumerationReferenceValue {
    catalog: Arc<MetadataCatalog>,
    value_id: Arc<str>,
}

#[derive(Debug)]
struct PictureLibrary {
    catalog: Arc<MetadataCatalog>,
}

#[derive(Debug)]
struct PictureValue {
    catalog: Arc<MetadataCatalog>,
    object_id: Arc<str>,
}

#[derive(Debug)]
struct ConfigurationExtensionsManager {
    catalog: Arc<MetadataCatalog>,
}

#[derive(Debug)]
struct InfoBaseUsersManager;

#[derive(Debug)]
struct IntegrationServicesManager;

#[derive(Debug)]
struct InfoBaseUserValue {
    principal: Principal,
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

#[derive(Debug)]
struct QueryValue {
    state: RefCell<QueryState>,
}

#[derive(Debug)]
struct QueryState {
    text: Rc<str>,
    parameters: Vec<(Rc<str>, Value)>,
    temp_tables_manager: Option<Value>,
}

struct TempTablesManagerValue {
    handle: TempTablesManagerHandle,
    data_access: Rc<dyn DataAccess>,
}

impl fmt::Debug for TempTablesManagerValue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TempTablesManagerValue")
            .field("handle", &self.handle)
            .finish_non_exhaustive()
    }
}

#[derive(Debug)]
struct QueryResultValue {
    data: Rc<QueryResultData>,
}

#[derive(Debug)]
struct QuerySelectionValue {
    data: Rc<QueryResultData>,
    /// Ноль означает позицию до первой строки; после конца остаётся номер
    /// последней выбранной строки, как в измерении платформы.
    cursor: Cell<usize>,
}

#[derive(Debug)]
struct DeflationValue {
    level: i8,
}

#[derive(Debug)]
struct ValueStorageValue {
    graph: SerializedValueGraph,
    _compression_level: Option<i8>,
}

#[derive(Debug, Default)]
struct DataLockValue {
    requests: RefCell<Vec<DataLockRequest>>,
}

#[derive(Debug)]
struct DataLockItemValue;

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
static COMPATIBILITY_MODE_PROPERTY_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-orm",
    name: "Metadata object enumerated property",
    type_display: "Metadata object enumerated property",
    type_names: &[],
};
static MANAGERS_TYPE: TypeDescriptor = TypeDescriptor::new("bsl-orm", "МенеджерыОбъектов");
static MANAGER_TYPE: TypeDescriptor = TypeDescriptor::new("bsl-orm", "МенеджерОбъекта");
static ENUMERATION_REFERENCE_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-orm",
    name: "ПеречислениеСсылка",
    type_display: "Enumeration reference",
    type_names: &["EnumerationRef"],
};
static PICTURE_LIBRARY_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-orm",
    name: "БиблиотекаКартинок",
    type_display: "Picture library",
    type_names: &["PictureLib"],
};
static PICTURE_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-orm",
    name: "Картинка",
    type_display: "Picture",
    type_names: &["Picture"],
};
static CONFIGURATION_EXTENSIONS_MANAGER_TYPE: TypeDescriptor =
    TypeDescriptor::new("bsl-orm", "МенеджерРасширенийКонфигурации");
static INFO_BASE_USERS_MANAGER_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-orm",
    name: "МенеджерПользователейИнформационнойБазы",
    type_display: "InfoBase users manager",
    type_names: &["InfoBaseUsersManager"],
};
static INFO_BASE_USER_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-orm",
    name: "ПользовательИнформационнойБазы",
    type_display: "InfoBase user",
    type_names: &["InfoBaseUser"],
};
static INTEGRATION_SERVICES_MANAGER_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-orm",
    name: "МенеджерСервисовИнтеграции",
    type_display: "Integration services manager",
    type_names: &["IntegrationServicesManager"],
};
static CONFIGURATION_EXTENSION_TYPE: TypeDescriptor =
    TypeDescriptor::new("bsl-orm", "РасширениеКонфигурации");
static EXTENSION_APPLICATION_ISSUE_TYPE: TypeDescriptor = TypeDescriptor::new(
    "bsl-orm",
    "ИнформацияОПроблемеПримененияРасширенияКонфигурации",
);
static QUERY_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-orm",
    name: "Запрос",
    type_display: "Query",
    type_names: &["Query"],
};
static TEMP_TABLES_MANAGER_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-orm",
    name: "МенеджерВременныхТаблиц",
    type_display: "Temporary Table Manager",
    type_names: &["TempTablesManager"],
};
static QUERY_RESULT_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-orm",
    name: "РезультатЗапроса",
    type_display: "Query result",
    type_names: &["QueryResult"],
};
static QUERY_SELECTION_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-orm",
    name: "ВыборкаИзРезультатаЗапроса",
    type_display: "Selected data from query result",
    type_names: &["QueryResultSelection"],
};
static DEFLATION_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-orm",
    name: "СжатиеДанных",
    type_display: "Compress Data",
    type_names: &["Deflation"],
};
static VALUE_STORAGE_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-orm",
    name: "ХранилищеЗначения",
    type_display: "Value storage",
    type_names: &["ValueStorage"],
};
static DATA_LOCK_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-orm",
    name: "БлокировкаДанных",
    type_display: "Data lock",
    type_names: &["DataLock"],
};
static DATA_LOCK_ITEM_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-orm",
    name: "ЭлементБлокировкиДанных",
    type_display: "Data lock item",
    type_names: &["DataLockItem"],
};

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

    fn display(&self) -> String {
        self.catalog
            .object_by_id(&self.object_id)
            .filter(|object| object.kind == MetadataKind::Language)
            .map_or_else(
                || METADATA_VALUE_TYPE.name.into(),
                |object| object.name().into(),
            )
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

impl ObjectProtocol for CompatibilityModeProperty {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &COMPATIBILITY_MODE_PROPERTY_TYPE
    }

    fn get_index(&self, index: &Value) -> RtResult<Value> {
        let Value::Str(member) = index else {
            return Err(RtError::TypeError {
                expected: "Строка",
                op: "ПеречислимоеСвойствоОбъектаМетаданных[]",
            });
        };
        lookup_member(EnumKind::CompatibilityMode, &member.to_string())
            .map(Value::Enum)
            .ok_or_else(|| RtError::UnknownProperty(member.to_string()))
    }

    fn display(&self) -> String {
        "MetadataObjectsPropertyEnumed".into()
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

// НЕ ИЗМЕРЕНО(PREDEFINED.CONFIG_ENUM.REFERENCE_SURFACE): до замера
// конкретного перечисления значение имеет общий тип, печатает имя и
// сравнивается по устойчивому GUID в одном снимке.
impl ObjectProtocol for EnumerationReferenceValue {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &ENUMERATION_REFERENCE_TYPE
    }

    fn is_filled(&self) -> RtResult<bool> {
        Ok(true)
    }

    fn identity_key(&self) -> Option<(usize, usize)> {
        let index = self
            .catalog
            .enumeration_values
            .iter()
            .position(|value| value.value_id == self.value_id)?;
        Some((Arc::as_ptr(&self.catalog) as usize, index))
    }

    fn display(&self) -> String {
        self.catalog
            .enumeration_values
            .iter()
            .find(|value| value.value_id == self.value_id)
            .map_or_else(String::new, |value| value.name.to_string())
    }
}

impl ObjectProtocol for PictureLibrary {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &PICTURE_LIBRARY_TYPE
    }

    fn get_property(&self, name: &str, _: &mut CallContext<'_>) -> RtResult<Value> {
        let object = self
            .catalog
            .object(MetadataKind::CommonPicture, name)
            .ok_or_else(|| RtError::UnknownProperty(name.into()))?;
        Ok(Value::new_object(PictureValue {
            catalog: Arc::clone(&self.catalog),
            object_id: Arc::clone(&object.id),
        }))
    }
}

impl ObjectProtocol for PictureValue {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &PICTURE_TYPE
    }

    fn is_filled(&self) -> RtResult<bool> {
        Ok(true)
    }

    fn identity_key(&self) -> Option<(usize, usize)> {
        let index = self
            .catalog
            .objects
            .iter()
            .position(|object| object.id == self.object_id)?;
        Some((Arc::as_ptr(&self.catalog) as usize, index))
    }

    fn display(&self) -> String {
        "Picture".into()
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

impl ObjectProtocol for InfoBaseUsersManager {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &INFO_BASE_USERS_MANAGER_TYPE
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        INFO_BASE_USERS_MANAGER_METHODS
    }
}

impl ObjectProtocol for IntegrationServicesManager {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &INTEGRATION_SERVICES_MANAGER_TYPE
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        INTEGRATION_SERVICES_MANAGER_METHODS
    }

    fn get_property(&self, name: &str, context: &mut CallContext<'_>) -> RtResult<Value> {
        let (principal, data_access) = current_data_access(context)?;
        data_access.integration_service(name, &principal, context)
    }
}

impl ObjectProtocol for InfoBaseUserValue {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &INFO_BASE_USER_TYPE
    }

    fn property_table(&self) -> &'static [PropertyDescriptor] {
        INFO_BASE_USER_PROPERTIES
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

impl ObjectProtocol for QueryValue {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &QUERY_TYPE
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        QUERY_METHODS
    }

    fn property_table(&self) -> &'static [PropertyDescriptor] {
        QUERY_PROPERTIES
    }
}

impl ObjectProtocol for TempTablesManagerValue {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &TEMP_TABLES_MANAGER_TYPE
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        TEMP_TABLES_MANAGER_METHODS
    }
}

impl ObjectProtocol for QueryResultValue {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &QUERY_RESULT_TYPE
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        QUERY_RESULT_METHODS
    }
}

impl ObjectProtocol for QuerySelectionValue {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &QUERY_SELECTION_TYPE
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        QUERY_SELECTION_METHODS
    }

    fn get_property(&self, name: &str, _: &mut CallContext<'_>) -> RtResult<Value> {
        query_selection_field(self, name)
    }

    fn get_index(&self, index: &Value) -> RtResult<Value> {
        let Value::Number(index) = index else {
            return Err(RtError::BadIndex);
        };
        let index = index
            .to_i64_exact()
            .and_then(|index| usize::try_from(index).ok())
            .ok_or(RtError::BadIndex)?;
        query_selection_value(self, index)
    }
}

impl ObjectProtocol for DeflationValue {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &DEFLATION_TYPE
    }
}

impl ObjectProtocol for ValueStorageValue {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &VALUE_STORAGE_TYPE
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        VALUE_STORAGE_METHODS
    }
}

impl ObjectProtocol for DataLockValue {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &DATA_LOCK_TYPE
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        DATA_LOCK_METHODS
    }
}

impl ObjectProtocol for DataLockItemValue {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &DATA_LOCK_ITEM_TYPE
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

fn metadata_root_compatibility_mode(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let root = receiver
        .downcast_ref::<MetadataRoot>()
        .ok_or(RtError::NotAnObject)?;
    Ok(configuration_description(root)?.compatibility_mode.value())
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

fn metadata_object_compatibility_modes(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    receiver
        .downcast_ref::<MetadataObjectProperties>()
        .ok_or(RtError::NotAnObject)?;
    Ok(Value::new_object(CompatibilityModeProperty))
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

fn new_query(_: &mut CallContext<'_>, arguments: &[Value]) -> RtResult<Value> {
    let text = match arguments {
        [] => Rc::from(""),
        [Value::Str(text)] => Rc::from(text.to_string()),
        [_] => {
            return Err(RtError::TypeError {
                expected: "Строка",
                op: "Новый Запрос",
            });
        }
        _ => {
            return Err(RtError::DynamicError(
                "Новый Запрос ожидает не больше одного аргумента".into(),
            ));
        }
    };
    Ok(Value::new_object(QueryValue {
        state: RefCell::new(QueryState {
            text,
            parameters: Vec::new(),
            temp_tables_manager: None,
        }),
    }))
}

fn new_temp_tables_manager(context: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    let (principal, data_access) = current_data_access(context)?;
    let handle = data_access.create_temp_tables_manager(&principal, context)?;
    Ok(Value::new_object(TempTablesManagerValue {
        handle,
        data_access,
    }))
}

fn new_deflation(_: &mut CallContext<'_>, arguments: &[Value]) -> RtResult<Value> {
    let level = match arguments {
        [] => -1,
        [Value::Number(level)] => level
            .to_i64_exact()
            .and_then(|level| i8::try_from(level).ok())
            .filter(|level| (-1..=9).contains(level))
            .ok_or_else(|| {
                RtError::DynamicError("степень сжатия должна быть целым числом от -1 до 9".into())
            })?,
        [_] => {
            return Err(RtError::TypeError {
                expected: "Число",
                op: "Новый СжатиеДанных",
            });
        }
        _ => {
            return Err(RtError::DynamicError(
                "Новый СжатиеДанных ожидает не больше одного аргумента".into(),
            ));
        }
    };
    Ok(Value::new_object(DeflationValue { level }))
}

fn new_value_storage(context: &mut CallContext<'_>, arguments: &[Value]) -> RtResult<Value> {
    let (value, compression_level) = match arguments {
        [value] => (value, None),
        [value, compression] => {
            let compression = compression
                .object_ref()
                .and_then(|object| object.downcast_ref::<DeflationValue>())
                .ok_or(RtError::TypeError {
                    expected: "СжатиеДанных",
                    op: "Новый ХранилищеЗначения",
                })?;
            (value, Some(compression.level))
        }
        _ => {
            return Err(RtError::DynamicError(
                "Новый ХранилищеЗначения ожидает один или два аргумента".into(),
            ));
        }
    };
    let graph = SerializedValueGraph::capture_detached(
        std::slice::from_ref(value),
        context.runtime_shapes(),
        &GraphLimits::default(),
    )?;
    Ok(Value::new_object(ValueStorageValue {
        graph,
        _compression_level: compression_level,
    }))
}

fn value_storage_get(
    receiver: &dyn ObjectProtocol,
    _: &[Value],
    context: &mut CallContext<'_>,
) -> RtResult<Value> {
    let storage = receiver
        .downcast_ref::<ValueStorageValue>()
        .ok_or(RtError::NotAnObject)?;
    let mut values = storage.graph.materialize(context.runtime_shapes())?;
    if values.len() != 1 {
        return Err(RtError::DynamicError(
            "снимок ХранилищеЗначения содержит неверное число корней".into(),
        ));
    }
    Ok(values.remove(0))
}

fn new_data_lock(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    Ok(Value::new_object(DataLockValue::default()))
}

fn data_lock_add(
    receiver: &dyn ObjectProtocol,
    arguments: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let lock = receiver
        .downcast_ref::<DataLockValue>()
        .ok_or(RtError::NotAnObject)?;
    match arguments {
        [] => {}
        [Value::Str(lock_space)] => lock.requests.borrow_mut().push(DataLockRequest {
            lock_space: Rc::from(lock_space.to_string()),
        }),
        [_] => {
            return Err(RtError::TypeError {
                expected: "Строка",
                op: "БлокировкаДанных.Добавить",
            });
        }
        _ => {
            return Err(RtError::DynamicError(
                "БлокировкаДанных.Добавить ожидает не больше одного аргумента".into(),
            ));
        }
    }
    Ok(Value::new_object(DataLockItemValue))
}

fn data_lock_lock(
    receiver: &dyn ObjectProtocol,
    _: &[Value],
    context: &mut CallContext<'_>,
) -> RtResult<Value> {
    let lock = receiver
        .downcast_ref::<DataLockValue>()
        .ok_or(RtError::NotAnObject)?;
    let requests = lock.requests.borrow().clone();
    let (principal, data_access) = current_data_access(context)?;
    data_access.lock_data(&requests, &principal, context)?;
    Ok(Value::Undefined)
}

fn begin_transaction(context: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    let (principal, data_access) = current_data_access(context)?;
    data_access.begin_transaction(&principal, context)?;
    Ok(Value::Undefined)
}

fn commit_transaction(context: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    let (principal, data_access) = current_data_access(context)?;
    data_access.commit_transaction(&principal, context)?;
    Ok(Value::Undefined)
}

fn rollback_transaction(context: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    let (principal, data_access) = current_data_access(context)?;
    data_access.rollback_transaction(&principal, context)?;
    Ok(Value::Undefined)
}

fn transaction_active(context: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    let (principal, data_access) = current_data_access(context)?;
    Ok(Value::Boolean(
        data_access.transaction_active(&principal, context)?,
    ))
}

fn exclusive_mode(context: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    let (principal, data_access) = current_data_access(context)?;
    Ok(Value::Boolean(
        data_access.exclusive_mode(&principal, context)?,
    ))
}

fn set_exclusive_mode(context: &mut CallContext<'_>, arguments: &[Value]) -> RtResult<Value> {
    let [Value::Boolean(enabled)] = arguments else {
        return Err(RtError::TypeError {
            expected: "Булево",
            op: "УстановитьМонопольныйРежим",
        });
    };
    let (principal, data_access) = current_data_access(context)?;
    data_access.set_exclusive_mode(*enabled, &principal, context)?;
    Ok(Value::Undefined)
}

fn integration_services(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    Ok(Value::new_object(IntegrationServicesManager))
}

fn set_integration_services_processing(
    receiver: &dyn ObjectProtocol,
    enabled: bool,
    context: &mut CallContext<'_>,
) -> RtResult<Value> {
    receiver
        .downcast_ref::<IntegrationServicesManager>()
        .ok_or(RtError::NotAnObject)?;
    let (principal, data_access) = current_data_access(context)?;
    // НЕ ИЗМЕРЕНО(INTEGRATION_SERVICES.PROCESSING): синтакс-помощник задаёт
    // назначение операций, но их побочные эффекты и ошибки ещё не сняты на 1С.
    data_access.set_integration_services_processing(enabled, &principal, context)?;
    Ok(Value::Undefined)
}

fn execute_integration_services_processing(
    receiver: &dyn ObjectProtocol,
    _: &[Value],
    context: &mut CallContext<'_>,
) -> RtResult<Value> {
    set_integration_services_processing(receiver, true, context)
}

fn stop_integration_services_processing(
    receiver: &dyn ObjectProtocol,
    _: &[Value],
    context: &mut CallContext<'_>,
) -> RtResult<Value> {
    set_integration_services_processing(receiver, false, context)
}

fn info_base_predefined_data_update(context: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    let (principal, data_access) = current_data_access(context)?;
    Ok(data_access
        .info_base_predefined_data_update(&principal, context)?
        .value())
}

fn set_info_base_predefined_data_update(
    context: &mut CallContext<'_>,
    arguments: &[Value],
) -> RtResult<Value> {
    let mode = PredefinedDataUpdateMode::from_value(&arguments[0])?;
    let (principal, data_access) = current_data_access(context)?;
    data_access.set_info_base_predefined_data_update(mode, &principal, context)?;
    Ok(Value::Undefined)
}

fn initialize_predefined_data(context: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    let (principal, data_access) = current_data_access(context)?;
    data_access.initialize_predefined_data(&principal, context)?;
    Ok(Value::Undefined)
}

fn query_text(receiver: &dyn ObjectProtocol, _: &mut CallContext<'_>) -> RtResult<Value> {
    let query = receiver
        .downcast_ref::<QueryValue>()
        .ok_or(RtError::NotAnObject)?;
    Ok(Value::Str(query.state.borrow().text.as_ref().into()))
}

fn set_query_text(
    receiver: &dyn ObjectProtocol,
    value: Value,
    _: &mut CallContext<'_>,
) -> RtResult<()> {
    let query = receiver
        .downcast_ref::<QueryValue>()
        .ok_or(RtError::NotAnObject)?;
    let Value::Str(value) = value else {
        return Err(RtError::TypeError {
            expected: "Строка",
            op: "Запрос.Текст",
        });
    };
    query.state.borrow_mut().text = Rc::from(value.to_string());
    Ok(())
}

fn temp_tables_manager(value: &Value) -> RtResult<&TempTablesManagerValue> {
    value
        .object_ref()
        .and_then(|object| object.downcast_ref::<TempTablesManagerValue>())
        .ok_or_else(|| RtError::TypeError {
            expected: "МенеджерВременныхТаблиц",
            op: "Запрос.МенеджерВременныхТаблиц",
        })
}

fn query_temp_tables_manager(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let query = receiver
        .downcast_ref::<QueryValue>()
        .ok_or(RtError::NotAnObject)?;
    Ok(query
        .state
        .borrow()
        .temp_tables_manager
        .clone()
        .unwrap_or(Value::Undefined))
}

fn set_query_temp_tables_manager(
    receiver: &dyn ObjectProtocol,
    value: Value,
    context: &mut CallContext<'_>,
) -> RtResult<()> {
    let query = receiver
        .downcast_ref::<QueryValue>()
        .ok_or(RtError::NotAnObject)?;
    let manager = temp_tables_manager(&value)?;
    let (_, data_access) = current_data_access(context)?;
    if !Rc::ptr_eq(&manager.data_access, &data_access) {
        return Err(RtError::DynamicError(
            "менеджер временных таблиц принадлежит другому сеансу".into(),
        ));
    }
    query.state.borrow_mut().temp_tables_manager = Some(value);
    Ok(())
}

fn close_temp_tables_manager(
    receiver: &dyn ObjectProtocol,
    _: &[Value],
    context: &mut CallContext<'_>,
) -> RtResult<Value> {
    let manager = receiver
        .downcast_ref::<TempTablesManagerValue>()
        .ok_or(RtError::NotAnObject)?;
    let (principal, data_access) = current_data_access(context)?;
    if !Rc::ptr_eq(&manager.data_access, &data_access) {
        return Err(RtError::DynamicError(
            "менеджер временных таблиц принадлежит другому сеансу".into(),
        ));
    }
    data_access.close_temp_tables_manager(manager.handle, &principal, context)?;
    Ok(Value::Undefined)
}

fn query_set_parameter(
    receiver: &dyn ObjectProtocol,
    arguments: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let query = receiver
        .downcast_ref::<QueryValue>()
        .ok_or(RtError::NotAnObject)?;
    let [Value::Str(name), value] = arguments else {
        return Err(RtError::TypeError {
            expected: "Строка, Произвольный",
            op: "Запрос.УстановитьПараметр",
        });
    };
    let name = name.to_string();
    let mut state = query.state.borrow_mut();
    if let Some((_, current)) = state
        .parameters
        .iter_mut()
        .find(|(candidate, _)| folded_eq(candidate, &name))
    {
        *current = value.clone();
    } else {
        state.parameters.push((Rc::from(name), value.clone()));
    }
    Ok(Value::Undefined)
}

fn query_execute(
    receiver: &dyn ObjectProtocol,
    _: &[Value],
    context: &mut CallContext<'_>,
) -> RtResult<Value> {
    let query = receiver
        .downcast_ref::<QueryValue>()
        .ok_or(RtError::NotAnObject)?;
    let request = {
        let state = query.state.borrow();
        let temp_tables_manager = match &state.temp_tables_manager {
            None => None,
            Some(value) => {
                let manager = temp_tables_manager(value)?;
                Some(manager.handle)
            }
        };
        QueryRequest {
            text: Rc::clone(&state.text),
            parameters: state.parameters.clone().into(),
            temp_tables_manager,
        }
    };
    let (principal, data_access) = {
        let session = session(context)?;
        let data_access = session.data_access.as_ref().ok_or_else(|| {
            RtError::DynamicError("доступ к данным конфигурации не подключён".into())
        })?;
        (session.principal.clone(), Rc::clone(data_access))
    };
    let data = data_access.execute_query(&request, &principal, context)?;
    Ok(Value::new_object(QueryResultValue {
        data: Rc::new(data),
    }))
}

fn query_result(receiver: &dyn ObjectProtocol) -> RtResult<&QueryResultValue> {
    receiver
        .downcast_ref::<QueryResultValue>()
        .ok_or(RtError::NotAnObject)
}

fn query_result_is_empty(
    receiver: &dyn ObjectProtocol,
    _: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    Ok(Value::Boolean(query_result(receiver)?.data.rows.is_empty()))
}

fn require_default_traversal(arguments: &[Value], operation: &str) -> RtResult<()> {
    if arguments.is_empty() {
        return Ok(());
    }
    Err(RtError::DynamicError(format!(
        "{operation}: поддерживается только прямой обход по умолчанию"
    )))
}

fn query_result_select(
    receiver: &dyn ObjectProtocol,
    arguments: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    require_default_traversal(arguments, "РезультатЗапроса.Выбрать")?;
    Ok(Value::new_object(QuerySelectionValue {
        data: Rc::clone(&query_result(receiver)?.data),
        cursor: Cell::new(0),
    }))
}

fn query_result_unload(
    receiver: &dyn ObjectProtocol,
    arguments: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    require_default_traversal(arguments, "РезультатЗапроса.Выгрузить")?;
    query_result_data_to_table(&query_result(receiver)?.data)
}

fn query_result_data_to_table(result: &QueryResultData) -> RtResult<Value> {
    let table = Value::new_table();
    let columns = table.get_field_by_name("Колонки")?;
    for name in result.columns.iter() {
        columns.table_add_column(&Value::Str(name.as_ref().into()), &Value::Undefined)?;
    }
    for values in result.rows.iter() {
        let row = table.table_add_row()?;
        for (name, value) in result.columns.iter().zip(values.iter()) {
            row.set_field_by_name(name, value.clone())?;
        }
    }
    Ok(table)
}

fn optional_array(value: &Value, operation: &'static str) -> RtResult<Option<Value>> {
    match value {
        Value::Undefined => Ok(None),
        Value::Object(object) if matches!(&**object, BslObject::Array(_)) => {
            Ok(Some(value.clone()))
        }
        _ => Err(RtError::TypeError {
            expected: "Массив или Неопределено",
            op: operation,
        }),
    }
}

fn find_by_ref(context: &mut CallContext<'_>, arguments: &[Value]) -> RtResult<Value> {
    let [references, search_area, include_objects, exclude_objects] = arguments else {
        return Err(RtError::InvalidBytecode(
            "НайтиПоСсылкам получила неверное число аргументов",
        ));
    };
    if !matches!(references, Value::Object(object) if matches!(&**object, BslObject::Array(_))) {
        return Err(RtError::TypeError {
            expected: "Массив",
            op: "НайтиПоСсылкам",
        });
    }
    let request = FindByRefRequest {
        references: references.clone(),
        search_area: optional_array(search_area, "НайтиПоСсылкам")?,
        include_objects: optional_array(include_objects, "НайтиПоСсылкам")?,
        exclude_objects: optional_array(exclude_objects, "НайтиПоСсылкам")?,
    };
    let (principal, data_access) = current_data_access(context)?;
    let result = data_access.find_by_ref(&request, &principal, context)?;
    query_result_data_to_table(&result)
}

fn query_selection(receiver: &dyn ObjectProtocol) -> RtResult<&QuerySelectionValue> {
    receiver
        .downcast_ref::<QuerySelectionValue>()
        .ok_or(RtError::NotAnObject)
}

fn query_selection_next(
    receiver: &dyn ObjectProtocol,
    _: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let selection = query_selection(receiver)?;
    let cursor = selection.cursor.get();
    if cursor < selection.data.rows.len() {
        selection.cursor.set(cursor + 1);
        Ok(Value::Boolean(true))
    } else {
        Ok(Value::Boolean(false))
    }
}

fn query_selection_current_row(selection: &QuerySelectionValue) -> Option<&[Value]> {
    selection
        .cursor
        .get()
        .checked_sub(1)
        .and_then(|index| selection.data.rows.get(index))
        .map(AsRef::as_ref)
}

fn query_selection_value(selection: &QuerySelectionValue, index: usize) -> RtResult<Value> {
    let Some(row) = query_selection_current_row(selection) else {
        return Ok(Value::Undefined);
    };
    row.get(index).cloned().ok_or(RtError::IndexOutOfBounds {
        index: index as i64,
        len: row.len(),
    })
}

fn query_selection_field(selection: &QuerySelectionValue, name: &str) -> RtResult<Value> {
    let index = selection
        .data
        .columns
        .iter()
        .position(|candidate| folded_eq(candidate, name))
        .ok_or_else(|| RtError::UnknownProperty(name.into()))?;
    query_selection_value(selection, index)
}

fn query_selection_get(
    receiver: &dyn ObjectProtocol,
    arguments: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let [Value::Number(index)] = arguments else {
        return Err(RtError::TypeError {
            expected: "Число",
            op: "ВыборкаИзРезультатаЗапроса.Получить",
        });
    };
    let index = index
        .to_i64_exact()
        .and_then(|index| usize::try_from(index).ok())
        .ok_or(RtError::BadIndex)?;
    query_selection_value(query_selection(receiver)?, index)
}

static QUERY_PROPERTIES: &[PropertyDescriptor] = &[
    PropertyDescriptor {
        names: &["Текст", "Text"],
        get: query_text,
        set: Some(set_query_text),
    },
    PropertyDescriptor {
        names: &["МенеджерВременныхТаблиц", "TempTablesManager"],
        get: query_temp_tables_manager,
        set: Some(set_query_temp_tables_manager),
    },
];

static QUERY_METHODS: &[MethodDescriptor] = &[
    MethodDescriptor::new(
        &["УстановитьПараметр", "SetParameter"],
        Arity::exact(2),
        query_set_parameter,
    ),
    MethodDescriptor::new(&["Выполнить", "Execute"], Arity::exact(0), query_execute),
];

static TEMP_TABLES_MANAGER_METHODS: &[MethodDescriptor] = &[MethodDescriptor::new(
    &["Закрыть", "Close"],
    Arity::exact(0),
    close_temp_tables_manager,
)
.as_procedure()];

static QUERY_RESULT_METHODS: &[MethodDescriptor] = &[
    MethodDescriptor::new(
        &["Пустой", "IsEmpty"],
        Arity::exact(0),
        query_result_is_empty,
    ),
    MethodDescriptor::new(
        &["Выбрать", "Select"],
        Arity::range(0, 3),
        query_result_select,
    ),
    MethodDescriptor::new(
        &["Выгрузить", "Unload"],
        Arity::range(0, 1),
        query_result_unload,
    ),
];

static QUERY_SELECTION_METHODS: &[MethodDescriptor] = &[
    MethodDescriptor::new(
        &["Следующий", "Next"],
        Arity::exact(0),
        query_selection_next,
    ),
    MethodDescriptor::new(&["Получить", "Get"], Arity::exact(1), query_selection_get),
];

static VALUE_STORAGE_METHODS: &[MethodDescriptor] = &[MethodDescriptor::new(
    &["Получить", "Get"],
    Arity::exact(0),
    value_storage_get,
)];

static DATA_LOCK_METHODS: &[MethodDescriptor] = &[
    MethodDescriptor::new(&["Добавить", "Add"], Arity::range(0, 1), data_lock_add),
    MethodDescriptor::new(&["Заблокировать", "Lock"], Arity::exact(0), data_lock_lock),
];

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
        names: &["РежимСовместимости", "CompatibilityMode"],
        get: metadata_root_compatibility_mode,
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

static METADATA_OBJECT_PROPERTIES: &[PropertyDescriptor] = &[
    PropertyDescriptor {
        names: &["ВариантВстроенногоЯзыка", "ScriptVariant"],
        get: metadata_object_script_variants,
        set: None,
    },
    PropertyDescriptor {
        names: &["РежимСовместимости", "CompatibilityMode"],
        get: metadata_object_compatibility_modes,
        set: None,
    },
];

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

static INFO_BASE_USERS_MANAGER_METHODS: &[MethodDescriptor] = &[MethodDescriptor::new(
    &["ТекущийПользователь", "CurrentUser"],
    Arity::exact(0),
    current_user,
)];

static INTEGRATION_SERVICES_MANAGER_METHODS: &[MethodDescriptor] = &[
    MethodDescriptor::new(
        &["ВыполнитьОбработку", "ExecuteProcessing"],
        Arity::exact(0),
        execute_integration_services_processing,
    ),
    MethodDescriptor::new(
        &["ОстановитьОбработку", "StopProcessing"],
        Arity::exact(0),
        stop_integration_services_processing,
    ),
];

static INFO_BASE_USER_PROPERTIES: &[PropertyDescriptor] = &[PropertyDescriptor {
    names: &["Имя", "Name"],
    get: current_user_name,
    set: None,
}];

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
        ("Документы", "Documents", MetadataKind::Document),
        (
            "ЖурналыДокументов",
            "DocumentJournals",
            MetadataKind::DocumentJournal,
        ),
        ("Отчеты", "Reports", MetadataKind::Report),
        ("Обработки", "DataProcessors", MetadataKind::DataProcessor),
        (
            "РегистрыСведений",
            "InformationRegisters",
            MetadataKind::InformationRegister,
        ),
        ("Константы", "Constants", MetadataKind::Constant),
        ("ПланыОбмена", "ExchangePlans", MetadataKind::ExchangePlan),
        (
            "ОбщиеКартинки",
            "CommonPictures",
            MetadataKind::CommonPicture,
        ),
        (
            "ФункциональныеОпции",
            "FunctionalOptions",
            MetadataKind::FunctionalOption,
        ),
        ("ЭлементыСтиля", "StyleItems", MetadataKind::StyleItem),
        ("Языки", "Languages", MetadataKind::Language),
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

fn documents(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    managers(ctx, MetadataKind::Document)
}

fn document_journals(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    managers(ctx, MetadataKind::DocumentJournal)
}

fn reports(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    managers(ctx, MetadataKind::Report)
}

fn data_processors(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    managers(ctx, MetadataKind::DataProcessor)
}

fn information_registers(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    managers(ctx, MetadataKind::InformationRegister)
}

fn constants(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    managers(ctx, MetadataKind::Constant)
}

fn enumerations(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    managers(ctx, MetadataKind::Enumeration)
}

fn exchange_plans(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    managers(ctx, MetadataKind::ExchangePlan)
}

fn configuration_extensions(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    Ok(Value::new_object(ConfigurationExtensionsManager {
        catalog: Arc::clone(session(ctx)?.catalog()),
    }))
}

fn info_base_users(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    Ok(Value::new_object(InfoBaseUsersManager))
}

fn picture_library(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    Ok(Value::new_object(PictureLibrary {
        catalog: Arc::clone(session(ctx)?.catalog()),
    }))
}

fn configuration_changed(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    let changed = session(ctx)?
        .catalog()
        .configuration_changed
        .ok_or_else(|| {
            RtError::DynamicError(
                "признак изменения конфигурации не предоставлен источником метаданных".into(),
            )
        })?;
    Ok(Value::Boolean(changed))
}

fn current_user(
    receiver: &dyn ObjectProtocol,
    _: &[Value],
    context: &mut CallContext<'_>,
) -> RtResult<Value> {
    receiver
        .downcast_ref::<InfoBaseUsersManager>()
        .ok_or(RtError::NotAnObject)?;
    Ok(Value::new_object(InfoBaseUserValue {
        principal: session(context)?.principal().clone(),
    }))
}

fn current_user_name(receiver: &dyn ObjectProtocol, _: &mut CallContext<'_>) -> RtResult<Value> {
    let user = receiver
        .downcast_ref::<InfoBaseUserValue>()
        .ok_or(RtError::NotAnObject)?;
    Ok(Value::Str(user.principal.name().into()))
}

fn user_name(context: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    Ok(Value::Str(session(context)?.principal().name().into()))
}

fn current_language(context: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    let session = session(context)?;
    let id = session
        .principal()
        .interface_language_id()
        .or_else(|| session.catalog().primary_language_id())
        .ok_or_else(|| RtError::DynamicError("текущий язык не предоставлен сеансом".into()))?;
    let language = session
        .catalog()
        .object_by_id(id)
        .filter(|object| object.kind() == MetadataKind::Language)
        .ok_or_else(|| {
            RtError::DynamicError("текущий язык отсутствует в каталоге метаданных".into())
        })?;
    Ok(metadata_object_value(session.catalog(), language))
}

fn predefined_value(context: &mut CallContext<'_>, arguments: &[Value]) -> RtResult<Value> {
    let [Value::Str(path)] = arguments else {
        return Err(RtError::TypeError {
            expected: "Строка",
            op: "ПредопределенноеЗначение",
        });
    };
    let path = path.to_string();
    let parts = path.split('.').collect::<Vec<_>>();
    if let [kind, member] = parts.as_slice()
        && let Some(value) = lookup_enum(kind).and_then(|kind| lookup_member(kind, member))
    {
        return Ok(Value::Enum(value));
    }
    if let [kind, owner_name, member] = parts.as_slice()
        && (folded_eq(kind, "Перечисление")
            || folded_eq(kind, "Enum")
            || folded_eq(kind, "Enumeration"))
    {
        let catalog = Arc::clone(session(context)?.catalog());
        if let Some(owner) = catalog.object(MetadataKind::Enumeration, owner_name)
            && let Some(value) = catalog.enumeration_value(owner.id(), member)
        {
            return Ok(Value::new_object(EnumerationReferenceValue {
                catalog: Arc::clone(&catalog),
                value_id: Arc::clone(&value.value_id),
            }));
        }
    }
    Err(RtError::DynamicError(format!(
        "предопределённое значение `{path}` не найдено"
    )))
}

fn functional_option(context: &mut CallContext<'_>, arguments: &[Value]) -> RtResult<Value> {
    let [Value::Str(name), tail @ ..] = arguments else {
        return Err(RtError::TypeError {
            expected: "Строка, Структура",
            op: "ПолучитьФункциональнуюОпцию",
        });
    };
    let parameters = match tail {
        [] => Vec::new(),
        [Value::Object(object)] => {
            let BslObject::Structure(storage) = object.as_ref() else {
                return Err(RtError::TypeError {
                    expected: "Структура",
                    op: "ПолучитьФункциональнуюОпцию",
                });
            };
            let storage = storage.borrow();
            (0..storage.len())
                .map(|index| {
                    let (name, value) = storage.entry_at(index).ok_or_else(|| {
                        RtError::DynamicError(
                            "структура параметров функциональной опции повреждена".into(),
                        )
                    })?;
                    let name = context.runtime_shapes().names.name(name).ok_or_else(|| {
                        RtError::DynamicError(
                            "имя параметра функциональной опции отсутствует".into(),
                        )
                    })?;
                    Ok((Rc::from(name), value))
                })
                .collect::<RtResult<Vec<_>>>()?
        }
        _ => {
            return Err(RtError::TypeError {
                expected: "Структура",
                op: "ПолучитьФункциональнуюОпцию",
            });
        }
    };
    let request = FunctionalOptionRequest {
        name: Rc::from(name.to_string()),
        parameters: parameters.into(),
    };
    let (principal, data_access) = current_data_access(context)?;
    data_access.functional_option(&request, &principal, context)
}

fn access_right(context: &mut CallContext<'_>, arguments: &[Value]) -> RtResult<Value> {
    let [right, metadata, tail @ ..] = arguments else {
        return Err(RtError::DynamicError(
            "ПравоДоступа ожидает от двух до четырёх аргументов".into(),
        ));
    };
    let Value::Str(right) = right else {
        return Err(RtError::TypeError {
            expected: "Строка",
            op: "ПравоДоступа",
        });
    };
    let right = right.to_string();
    if right.is_empty() {
        return Err(RtError::DynamicError("имя права пусто".into()));
    }

    let (catalog, metadata) = {
        let session = session(context)?;
        let value = metadata
            .object_ref()
            .and_then(|object| object.downcast_ref::<MetadataValue>())
            .filter(|value| Arc::ptr_eq(&value.catalog, session.catalog()))
            .ok_or_else(|| RtError::TypeError {
                expected: "ОбъектМетаданных",
                op: "ПравоДоступа",
            })?;
        let metadata = session
            .catalog()
            .object_by_id(&value.object_id)
            .cloned()
            .ok_or_else(|| {
                RtError::DynamicError("объект метаданных отсутствует в каталоге".into())
            })?;
        (Arc::clone(session.catalog()), metadata)
    };

    let subject = match tail.first() {
        None | Some(Value::Undefined) => AccessRightSubject::CurrentPrincipal,
        Some(value) => {
            let object = value.object_ref().ok_or_else(|| RtError::TypeError {
                expected: "ПользовательИнформационнойБазы или ОбъектМетаданныхРоль",
                op: "ПравоДоступа",
            })?;
            if let Some(user) = object.downcast_ref::<InfoBaseUserValue>() {
                AccessRightSubject::Principal(user.principal.clone())
            } else if let Some(role) = object
                .downcast_ref::<MetadataValue>()
                .filter(|value| Arc::ptr_eq(&value.catalog, &catalog))
                .and_then(|value| catalog.object_by_id(&value.object_id))
                .filter(|object| object.kind() == MetadataKind::Role)
            {
                AccessRightSubject::Role(Arc::from(role.id()))
            } else {
                return Err(RtError::TypeError {
                    expected: "ПользовательИнформационнойБазы или ОбъектМетаданныхРоль",
                    op: "ПравоДоступа",
                });
            }
        }
    };
    let standard_attribute = match tail.get(1) {
        None | Some(Value::Undefined) => None,
        Some(Value::Str(value)) if !value.to_string().is_empty() => {
            Some(Rc::from(value.to_string()))
        }
        Some(_) => {
            return Err(RtError::TypeError {
                expected: "непустая Строка",
                op: "ПравоДоступа",
            });
        }
    };
    let request = AccessRightRequest {
        right: Rc::from(right),
        metadata,
        subject,
        standard_attribute,
    };
    if context.privileged_mode()? {
        return Ok(Value::Boolean(true));
    }
    let (principal, data_access) = current_data_access(context)?;
    Ok(Value::Boolean(
        data_access.access_right(&request, &principal, context)?,
    ))
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

const CONSTRUCTORS: &[ConstructorDescriptor] = &[
    ConstructorDescriptor {
        code: ConstructorCode::new(1),
        names: &["Запрос", "Query"],
        arity: Arity::range(0, 1),
        call: new_query,
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(2),
        names: &["СжатиеДанных", "Deflation"],
        arity: Arity::range(0, 1),
        call: new_deflation,
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(3),
        names: &["ХранилищеЗначения", "ValueStorage"],
        arity: Arity::range(1, 2),
        call: new_value_storage,
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(4),
        names: &["БлокировкаДанных", "DataLock"],
        arity: Arity::exact(0),
        call: new_data_lock,
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(5),
        names: &["МенеджерВременныхТаблиц", "TempTablesManager"],
        arity: Arity::exact(0),
        call: new_temp_tables_manager,
    },
];

const CONSTRUCTOR_TYPES: &[(ConstructorCode, &TypeDescriptor)] = &[
    (ConstructorCode::new(1), &QUERY_TYPE),
    (ConstructorCode::new(2), &DEFLATION_TYPE),
    (ConstructorCode::new(3), &VALUE_STORAGE_TYPE),
    (ConstructorCode::new(4), &DATA_LOCK_TYPE),
    (ConstructorCode::new(5), &TEMP_TABLES_MANAGER_TYPE),
];

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
    FunctionDescriptor {
        code: FunctionCode::new(7),
        names: &["ПолучитьПользователейИнформационнойБазыORM"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: info_base_users,
    },
    FunctionDescriptor {
        code: FunctionCode::new(8),
        names: &["ПолучитьПеречисленияORM"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: enumerations,
    },
    FunctionDescriptor {
        code: FunctionCode::new(9),
        names: &["ПолучитьБиблиотекуКартинокORM"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: picture_library,
    },
    FunctionDescriptor {
        code: FunctionCode::new(10),
        names: &["КонфигурацияИзменена", "ConfigurationChanged"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: configuration_changed,
    },
    FunctionDescriptor {
        code: FunctionCode::new(11),
        names: &["ПолучитьПланыОбменаORM"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: exchange_plans,
    },
    FunctionDescriptor {
        code: FunctionCode::new(12),
        names: &["НачатьТранзакцию", "BeginTransaction"],
        arity: Arity::exact(0),
        kind: FunctionKind::Procedure,
        call: begin_transaction,
    },
    FunctionDescriptor {
        code: FunctionCode::new(13),
        names: &["ЗафиксироватьТранзакцию", "CommitTransaction"],
        arity: Arity::exact(0),
        kind: FunctionKind::Procedure,
        call: commit_transaction,
    },
    FunctionDescriptor {
        code: FunctionCode::new(14),
        names: &["ОтменитьТранзакцию", "RollbackTransaction"],
        arity: Arity::exact(0),
        kind: FunctionKind::Procedure,
        call: rollback_transaction,
    },
    FunctionDescriptor {
        code: FunctionCode::new(15),
        names: &["ПравоДоступа", "AccessRight"],
        arity: Arity::range(2, 4),
        kind: FunctionKind::Function,
        call: access_right,
    },
    FunctionDescriptor {
        code: FunctionCode::new(16),
        names: &["ИмяПользователя", "UserName"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: user_name,
    },
    FunctionDescriptor {
        code: FunctionCode::new(17),
        names: &["ПредопределенноеЗначение", "PredefinedValue"],
        arity: Arity::exact(1),
        kind: FunctionKind::Function,
        call: predefined_value,
    },
    FunctionDescriptor {
        code: FunctionCode::new(18),
        names: &["МонопольныйРежим", "ExclusiveMode"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: exclusive_mode,
    },
    FunctionDescriptor {
        code: FunctionCode::new(19),
        names: &["УстановитьМонопольныйРежим", "SetExclusiveMode"],
        arity: Arity::exact(1),
        kind: FunctionKind::Procedure,
        call: set_exclusive_mode,
    },
    FunctionDescriptor {
        code: FunctionCode::new(20),
        names: &["ПолучитьСервисыИнтеграцииORM"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: integration_services,
    },
    FunctionDescriptor {
        code: FunctionCode::new(21),
        names: &[
            "ПолучитьОбновлениеПредопределенныхДанныхИнформационнойБазы",
            "GetInfoBasePredefinedData",
        ],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: info_base_predefined_data_update,
    },
    FunctionDescriptor {
        code: FunctionCode::new(22),
        names: &[
            "УстановитьОбновлениеПредопределенныхДанныхИнформационнойБазы",
            "SetInfoBasePredefinedDataUpdate",
        ],
        arity: Arity::exact(1),
        kind: FunctionKind::Procedure,
        call: set_info_base_predefined_data_update,
    },
    FunctionDescriptor {
        code: FunctionCode::new(23),
        names: &[
            "ИнициализироватьПредопределенныеДанные",
            "InitializePredefinedData",
        ],
        arity: Arity::exact(0),
        kind: FunctionKind::Procedure,
        call: initialize_predefined_data,
    },
    FunctionDescriptor {
        code: FunctionCode::new(24),
        names: &["ПолучитьФункциональнуюОпцию", "GetFunctionalOption"],
        arity: Arity::range(1, 2),
        kind: FunctionKind::Function,
        call: functional_option,
    },
    FunctionDescriptor {
        code: FunctionCode::new(25),
        names: &["ТранзакцияАктивна", "TransactionActive"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: transaction_active,
    },
    FunctionDescriptor {
        code: FunctionCode::new(26),
        names: &["ТекущийЯзык", "CurrentLanguage"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: current_language,
    },
    FunctionDescriptor {
        code: FunctionCode::new(27),
        names: &["НайтиПоСсылкам", "FindByRef"],
        arity: Arity::range(1, 4),
        kind: FunctionKind::Function,
        call: find_by_ref,
    },
    FunctionDescriptor {
        code: FunctionCode::new(28),
        names: &["ПолучитьДокументыORM"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: documents,
    },
    FunctionDescriptor {
        code: FunctionCode::new(29),
        names: &["ПолучитьЖурналыДокументовORM"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: document_journals,
    },
    FunctionDescriptor {
        code: FunctionCode::new(30),
        names: &["ПолучитьОтчетыORM"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: reports,
    },
    FunctionDescriptor {
        code: FunctionCode::new(31),
        names: &["ПолучитьОбработкиORM"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: data_processors,
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
        names: &["Документы", "Documents"],
        function: FunctionCode::new(28),
    },
    GlobalDescriptor {
        names: &["ЖурналыДокументов", "DocumentJournals"],
        function: FunctionCode::new(29),
    },
    GlobalDescriptor {
        names: &["Отчеты", "Reports"],
        function: FunctionCode::new(30),
    },
    GlobalDescriptor {
        names: &["Обработки", "DataProcessors"],
        function: FunctionCode::new(31),
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
    GlobalDescriptor {
        names: &["ПользователиИнформационнойБазы", "InfoBaseUsers"],
        function: FunctionCode::new(7),
    },
    GlobalDescriptor {
        names: &["Перечисления", "Enums"],
        function: FunctionCode::new(8),
    },
    GlobalDescriptor {
        names: &["ПланыОбмена", "ExchangePlans"],
        function: FunctionCode::new(11),
    },
    GlobalDescriptor {
        names: &["БиблиотекаКартинок", "PictureLib"],
        function: FunctionCode::new(9),
    },
    GlobalDescriptor {
        names: &["СервисыИнтеграции", "IntegrationServices"],
        function: FunctionCode::new(20),
    },
];

const TYPES: &[&TypeDescriptor] = &[
    &METADATA_TYPE,
    &METADATA_COLLECTION_TYPE,
    &METADATA_VALUE_TYPE,
    &MAIN_ROLES_TYPE,
    &METADATA_OBJECT_PROPERTIES_TYPE,
    &COMPATIBILITY_MODE_PROPERTY_TYPE,
    &MANAGERS_TYPE,
    &MANAGER_TYPE,
    &ENUMERATION_REFERENCE_TYPE,
    &PICTURE_LIBRARY_TYPE,
    &PICTURE_TYPE,
    &CONFIGURATION_EXTENSIONS_MANAGER_TYPE,
    &INFO_BASE_USERS_MANAGER_TYPE,
    &INFO_BASE_USER_TYPE,
    &INTEGRATION_SERVICES_MANAGER_TYPE,
    &CONFIGURATION_EXTENSION_TYPE,
    &EXTENSION_APPLICATION_ISSUE_TYPE,
    &QUERY_TYPE,
    &TEMP_TABLES_MANAGER_TYPE,
    &QUERY_RESULT_TYPE,
    &QUERY_SELECTION_TYPE,
    &DEFLATION_TYPE,
    &VALUE_STORAGE_TYPE,
    &DATA_LOCK_TYPE,
    &DATA_LOCK_ITEM_TYPE,
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
    ObjectMembersDescriptor::new(&COMPATIBILITY_MODE_PROPERTY_TYPE),
    ObjectMembersDescriptor::new(&MANAGERS_TYPE).with_dynamic_properties(),
    ObjectMembersDescriptor::new(&MANAGER_TYPE)
        .with_methods(CONSTANT_MANAGER_METHODS)
        .with_properties(MANAGER_PROPERTIES),
    ObjectMembersDescriptor::new(&ENUMERATION_REFERENCE_TYPE),
    ObjectMembersDescriptor::new(&PICTURE_LIBRARY_TYPE).with_dynamic_properties(),
    ObjectMembersDescriptor::new(&PICTURE_TYPE),
    ObjectMembersDescriptor::new(&CONFIGURATION_EXTENSIONS_MANAGER_TYPE)
        .with_methods(CONFIGURATION_EXTENSIONS_MANAGER_METHODS),
    ObjectMembersDescriptor::new(&INFO_BASE_USERS_MANAGER_TYPE)
        .with_methods(INFO_BASE_USERS_MANAGER_METHODS),
    ObjectMembersDescriptor::new(&INFO_BASE_USER_TYPE).with_properties(INFO_BASE_USER_PROPERTIES),
    ObjectMembersDescriptor::new(&INTEGRATION_SERVICES_MANAGER_TYPE)
        .with_methods(INTEGRATION_SERVICES_MANAGER_METHODS)
        .with_dynamic_properties(),
    ObjectMembersDescriptor::new(&CONFIGURATION_EXTENSION_TYPE)
        .with_properties(CONFIGURATION_EXTENSION_PROPERTIES),
    ObjectMembersDescriptor::new(&EXTENSION_APPLICATION_ISSUE_TYPE)
        .with_properties(EXTENSION_APPLICATION_ISSUE_PROPERTIES),
    ObjectMembersDescriptor::new(&QUERY_TYPE)
        .with_methods(QUERY_METHODS)
        .with_properties(QUERY_PROPERTIES),
    ObjectMembersDescriptor::new(&TEMP_TABLES_MANAGER_TYPE)
        .with_methods(TEMP_TABLES_MANAGER_METHODS),
    ObjectMembersDescriptor::new(&QUERY_RESULT_TYPE).with_methods(QUERY_RESULT_METHODS),
    ObjectMembersDescriptor::new(&QUERY_SELECTION_TYPE)
        .with_methods(QUERY_SELECTION_METHODS)
        .with_dynamic_properties(),
    ObjectMembersDescriptor::new(&VALUE_STORAGE_TYPE).with_methods(VALUE_STORAGE_METHODS),
    ObjectMembersDescriptor::new(&DATA_LOCK_TYPE).with_methods(DATA_LOCK_METHODS),
    ObjectMembersDescriptor::new(&DATA_LOCK_ITEM_TYPE),
];

const OBJECT_MEMBER_GROUPS: &[&[ObjectMembersDescriptor]] = &[API_MEMBERS];

const LIBRARY: LibraryDescriptor = LibraryDescriptor::new("bsl-orm", env!("CARGO_PKG_VERSION"))
    .with_functions(FUNCTIONS)
    .with_globals(GLOBALS)
    .with_constructors(CONSTRUCTORS)
    .with_constructor_types(CONSTRUCTOR_TYPES)
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
