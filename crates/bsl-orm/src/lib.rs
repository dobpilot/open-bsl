//! Объектная модель данных и метаданных конфигурации 1С.
//!
//! Крейт является runtime-компонентом для существующего интерпретатора
//! `open-bsl`. Он не разбирает и не исполняет BSL самостоятельно. Физический
//! доступ к данным будет внедряться данными сеанса; каталог ниже хранит только
//! неизменяемое описание конфигурации и связи с её BSL-модулями.

#![warn(missing_docs)]

mod exchange_plan;

use std::any::Any;
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt;
use std::rc::Rc;
use std::sync::Arc;

use open_bsl::{
    Arity, BslNumber, BslObject, CallContext, ConstructorCode, ConstructorDescriptor, EnumKind,
    EnumValue, FunctionCode, FunctionDescriptor, FunctionKind, GlobalDescriptor, GraphLimits,
    LibraryDescriptor, MethodDescriptor, ObjectMembersDescriptor, ObjectProtocol, PICTURE_TYPE,
    PropertyDescriptor, RtError, RtResult, SerializedValueGraph, TypeDescriptor, TypeRef, Value,
    ValueTypeDescriptor, call_method_from_table, folded_eq, get_property_from_table, lookup_enum,
    lookup_member, set_property_from_table,
};

mod common_attribute_separation;
pub use common_attribute_separation::CommonAttributeSeparationParameters;
mod common_attribute_content;
pub use common_attribute_content::{CommonAttributeCondition, CommonAttributeContentEntry};

mod accumulation_balance;
mod query_boundary;
pub use query_boundary::query_boundary_parts;
mod accumulation_metadata;
mod attribute_filling;
mod business_process_metadata;
mod business_process_object;
mod catalog_object;
mod catalog_selection;
mod catalog_standard_search;
mod characteristic_object;
mod data_processor_object;
mod document_movements;
mod document_object;
mod document_tabular_section;
mod functional_options;
mod information_register_get;
mod metadata_attributes;
mod metadata_find_by_type;
mod reference_uuid;
mod route_points;
mod second_factor_setting;
pub use accumulation_metadata::AccumulationRegisterKind;
pub use business_process_metadata::BusinessProcessNumberSettings;
pub use functional_options::{
    FunctionalOptionLocation, FunctionalOptionParameter, FunctionalOptionSettings,
};
pub use information_register_get::InformationRegisterPeriodicity;
pub use route_points::BusinessProcessRoutePoint;

/// Платформенные методы собственного модуля объекта бизнес-процесса.
pub fn business_process_object_receiver_methods() -> &'static [MethodDescriptor] {
    business_process_object::METHODS
}

/// Стандартные свойства собственного модуля объекта бизнес-процесса.
pub fn business_process_object_receiver_properties() -> &'static [PropertyDescriptor] {
    business_process_object::PROPERTIES
}

/// Платформенные методы собственного получателя модуля объекта справочника.
pub fn catalog_object_receiver_methods() -> &'static [MethodDescriptor] {
    catalog_object::METHODS
}

/// Платформенные свойства собственного получателя модуля объекта справочника.
pub fn catalog_object_receiver_properties() -> &'static [PropertyDescriptor] {
    catalog_object::PROPERTIES
}

/// Платформенные методы получателя модуля объекта плана видов характеристик.
pub fn characteristic_object_receiver_methods() -> &'static [MethodDescriptor] {
    characteristic_object::METHODS
}

/// Платформенные свойства получателя модуля объекта плана видов характеристик.
pub fn characteristic_object_receiver_properties() -> &'static [PropertyDescriptor] {
    characteristic_object::PROPERTIES
}

/// Платформенные методы получателя модуля объекта обработки.
pub fn data_processor_object_receiver_methods() -> &'static [MethodDescriptor] {
    data_processor_object::METHODS
}

/// Платформенные свойства получателя модуля объекта отчёта.
pub fn report_object_receiver_properties() -> &'static [PropertyDescriptor] {
    data_processor_object::REPORT_PROPERTIES
}

/// Платформенные методы получателя модуля объекта документа.
pub fn document_object_receiver_methods() -> &'static [MethodDescriptor] {
    document_object::METHODS
}

/// Платформенные свойства получателя модуля объекта документа.
pub fn document_object_receiver_properties() -> &'static [PropertyDescriptor] {
    document_object::PROPERTIES
}

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

/// Назначение расширения, явно приобретённое загрузчиком конфигурации.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigurationExtensionPurpose {
    /// Исправление конфигурации.
    Patch,
    /// Адаптация конфигурации.
    Customization,
    /// Дополнение конфигурации.
    AddOn,
}

impl ConfigurationExtensionPurpose {
    fn value(self) -> Value {
        Value::Enum(match self {
            Self::Patch => EnumValue::ConfigurationExtensionPurposePatch,
            Self::Customization => EnumValue::ConfigurationExtensionPurposeCustomization,
            Self::AddOn => EnumValue::ConfigurationExtensionPurposeAddOn,
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
    purpose: Option<ConfigurationExtensionPurpose>,
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
            purpose: None,
        })
    }

    /// Дополняет описание явно приобретённым назначением расширения.
    /// Отсутствующее назначение не выводится из имени или других свойств.
    #[must_use]
    pub fn with_purpose(mut self, purpose: ConfigurationExtensionPurpose) -> Self {
        self.purpose = Some(purpose);
        self
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
    /// Подсистема конфигурации.
    Subsystem,
    /// Язык конфигурации.
    Language,
    /// Перечисление конфигурации.
    Enumeration,
    /// Общая картинка.
    CommonPicture,
    /// Общий реквизит конфигурации.
    CommonAttribute,
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
    /// Последовательность.
    Sequence,
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
    /// Корень приобретённой конфигурации; отдельной коллекции у него нет.
    Configuration,
}

impl MetadataKind {
    /// Платформенные методы собственного получателя модуля менеджера.
    pub fn receiver_methods(self) -> &'static [MethodDescriptor] {
        manager_methods(self)
    }

    /// Возвращает русское имя коллекции этого вида метаданных.
    #[must_use]
    pub fn collection_name(self) -> Option<&'static str> {
        metadata_collections()
            .iter()
            .find(|(_, _, kind)| *kind == self)
            .map(|(name, _, _)| *name)
    }
}

/// Тип стандартного кода справочника в метаданных конфигурации.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CatalogCodeType {
    /// Числовой код.
    Number,
    /// Строковый код.
    String,
}

/// Основное представление справочника, приобретённое из метаданных.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CatalogDefaultPresentation {
    /// Представление по коду.
    AsCode,
    /// Представление по наименованию.
    AsDescription,
}

/// Приобретённый вид иерархии, независимый от признака иерархичности.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CatalogHierarchyType {
    /// Иерархия групп и элементов.
    FoldersAndItems,
    /// Иерархия только элементов.
    Items,
}

/// Приобретённые настройки стандартных реквизитов справочника.
/// Нулевая длина сохраняется как настройка и сама по себе не означает,
/// что соответствующее свойство отсутствует в объектной модели BSL.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CatalogStandardAttributes {
    /// Тип кода.
    pub code_type: CatalogCodeType,
    /// Настроенная длина кода.
    pub code_length: u32,
    /// Настроенная длина наименования.
    pub description_length: u32,
}

/// Приобретённые настройки нового объекта плана видов характеристик.
#[derive(Debug, Clone, PartialEq)]
pub struct CharacteristicSettings {
    /// Настроенная длина строкового кода.
    pub code_length: u32,
    /// Настроенная длина наименования.
    pub description_length: u32,
    /// Полный приобретённый состав типов характеристики с квалификаторами.
    /// Пустой список означает известный пустой состав, а не неизвестные данные.
    pub value_types: Vec<open_bsl::ColumnType>,
}

/// Приобретённый тип номера документа.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentNumberType {
    /// Строковый номер.
    String,
    /// Числовой номер.
    Number,
}

/// Известные настройки номера из метаданных документа.
/// Отсутствие описания отличается от известной нулевой длины номера.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DocumentNumberSettings {
    /// Настроенный тип номера.
    pub number_type: DocumentNumberType,
    /// Настроенная длина номера; ноль означает отключённую нумерацию.
    pub number_length: u32,
}

/// Приобретённая периодичность нумерации документа.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentNumberPeriodicity {
    /// Нумерация без разделения по периодам.
    Nonperiodical,
    /// Нумерация в пределах года.
    Year,
    /// Нумерация в пределах квартала.
    Quarter,
    /// Нумерация в пределах месяца.
    Month,
    /// Нумерация в пределах дня.
    Day,
}

/// Полностью приобретённая область нумерации, без состояния резервирования.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentNumberingSettings {
    /// Периодичность нумерации из конфигурации.
    pub periodicity: DocumentNumberPeriodicity,
    /// Идентичность объекта нумератора; отсутствие означает известную пустую
    /// ссылку, а не неизвестную форму описателя.
    pub numerator: Option<Arc<str>>,
}

/// Приобретённая настройка проверки заполнения реквизита.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FillChecking {
    /// Автоматическая проверка отключена.
    DontCheck,
    /// При незаполненном значении требуется сообщение об ошибке.
    ShowError,
}

/// Приобретённая настройка стандартного реквизита справочника.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogStandardAttributeFillChecking {
    /// Каноническое имя стандартного реквизита.
    pub name: String,
    /// Известный режим проверки заполнения.
    pub fill_checking: FillChecking,
}

/// Неизменяемое описание собственного реквизита документа.
/// Значение заполнения хранится снимком, без памяти объектов живого сеанса.
#[derive(Debug, Clone, PartialEq)]
pub struct DocumentAttribute {
    id: Arc<str>,
    name: Arc<str>,
    types: Arc<[open_bsl::ColumnType]>,
    fill_from_filling_value: Option<bool>,
    fill_checking: Option<FillChecking>,
    synonyms: Arc<[(Arc<str>, Arc<str>)]>,
    fill_value: Option<Arc<SerializedValueGraph>>,
}

impl DocumentAttribute {
    /// Создаёт описание из идентификатора, имени и приобретённых типов.
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        types: Vec<open_bsl::ColumnType>,
    ) -> Self {
        Self {
            id: Arc::from(id.into()),
            name: Arc::from(name.into()),
            types: types.into(),
            fill_from_filling_value: None,
            fill_checking: None,
            synonyms: Arc::from([]),
            fill_value: None,
        }
    }

    /// Сохраняет известную настройку заполнения из переданного значения.
    #[must_use]
    pub const fn with_fill_from_filling_value(mut self, enabled: bool) -> Self {
        self.fill_from_filling_value = Some(enabled);
        self
    }

    /// Сохраняет приобретённый режим проверки заполнения.
    #[must_use]
    pub const fn with_fill_checking(mut self, setting: FillChecking) -> Self {
        self.fill_checking = Some(setting);
        self
    }

    /// Возвращает режим проверки; `None` означает неизвестную настройку.
    #[must_use]
    pub const fn fill_checking(&self) -> Option<FillChecking> {
        self.fill_checking
    }

    /// Сохраняет приобретённый синоним реквизита для указанного кода языка.
    #[must_use]
    pub fn with_synonym(mut self, language: impl Into<String>, text: impl Into<String>) -> Self {
        let mut synonyms = self.synonyms.to_vec();
        synonyms.push((Arc::from(language.into()), Arc::from(text.into())));
        self.synonyms = synonyms.into();
        self
    }

    /// Возвращает синоним указанного языка либо каноническое имя реквизита.
    #[must_use]
    pub fn presentation(&self, language: &str) -> &str {
        self.synonyms
            .iter()
            .find(|(code, text)| folded_eq(code, language) && !text.is_empty())
            .map_or(self.name(), |(_, text)| text)
    }

    /// Сохраняет приобретённый снимок значения заполнения.
    /// Снимок должен содержать один корень; исполнение проверяет его при чтении.
    #[must_use]
    pub fn with_fill_value(mut self, value: SerializedValueGraph) -> Self {
        self.fill_value = Some(Arc::new(value));
        self
    }

    /// Устойчивый идентификатор реквизита в конфигурации.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }
    /// Каноническое имя реквизита.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }
    /// Приобретённые типы и квалификаторы реквизита.
    #[must_use]
    pub fn types(&self) -> &[open_bsl::ColumnType] {
        &self.types
    }
    /// Настройка заполнения; `None` означает отсутствие сведений.
    #[must_use]
    pub const fn fill_from_filling_value(&self) -> Option<bool> {
        self.fill_from_filling_value
    }
    /// Снимок значения заполнения; `None` отличается от снимка `Неопределено`.
    #[must_use]
    pub fn fill_value(&self) -> Option<&SerializedValueGraph> {
        self.fill_value.as_deref()
    }
}

/// Собственная табличная часть документа с известной идентичностью.
/// Неизвестные реквизиты строк отличаются от приобретённой пустой коллекции.
#[derive(Debug, Clone, PartialEq)]
pub struct DocumentTabularSection {
    id: Arc<str>,
    name: Arc<str>,
    attributes: Option<Arc<[DocumentAttribute]>>,
    fill_checking: Option<FillChecking>,
}

impl DocumentTabularSection {
    /// Создаёт описание части без предположений о реквизитах её строк.
    #[must_use]
    pub fn new(id: impl Into<Arc<str>>, name: impl Into<Arc<str>>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            attributes: None,
            fill_checking: None,
        }
    }

    /// Сохраняет приобретённую проверку самой части, отдельно от её столбцов.
    #[must_use]
    pub const fn with_fill_checking(mut self, setting: FillChecking) -> Self {
        self.fill_checking = Some(setting);
        self
    }

    /// Возвращает проверку самой части; `None` означает неизвестную настройку.
    #[must_use]
    pub const fn fill_checking(&self) -> Option<FillChecking> {
        self.fill_checking
    }

    /// Сохраняет полный список реквизитов строки в порядке конфигурации.
    #[must_use]
    pub fn with_attributes(mut self, attributes: Vec<DocumentAttribute>) -> Self {
        self.attributes = Some(attributes.into());
        self
    }

    /// Возвращает приобретённый идентификатор части.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Возвращает имя части в конфигурации.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Возвращает полный список; `None` означает неизвестные реквизиты строк.
    #[must_use]
    pub fn attributes(&self) -> Option<&[DocumentAttribute]> {
        self.attributes.as_deref()
    }
}

/// Неизменяемое описание одного объекта метаданных.
#[derive(Debug, Clone, PartialEq)]
pub struct MetadataObject {
    id: Arc<str>,
    name: Arc<str>,
    reference_type_presentation: Option<Arc<str>>,
    kind: MetadataKind,
    declared_types: Option<Arc<[open_bsl::ColumnType]>>,
    exchange_plan_attributes: Option<Arc<[DocumentAttribute]>>,
    manager_module: Option<Arc<str>>,
    object_module: Option<Arc<str>>,
    object_module_is_extension: bool,
    own_extension_object_module_safe_mode: Option<bool>,
    catalog_standard_attributes: Option<CatalogStandardAttributes>,
    characteristic_settings: Option<Arc<CharacteristicSettings>>,
    route_points: Option<route_points::Composition>,
    information_register: Option<information_register_get::Settings>,
    accumulation_register_kind: Option<AccumulationRegisterKind>,
    accumulation_dimensions: Option<Arc<[DocumentAttribute]>>,
    accumulation_resources: Option<Arc<[DocumentAttribute]>>,
    characteristic_attributes: Option<Arc<[DocumentAttribute]>>,
    catalog_standard_fill_checking: Option<Arc<[CatalogStandardAttributeFillChecking]>>,
    business_process_standard_fill_checking: Option<Arc<[CatalogStandardAttributeFillChecking]>>,
    catalog_owners: Option<Arc<[String]>>,
    catalog_common_attributes: Option<Arc<[DocumentAttribute]>>,
    common_attribute_auto_use: Option<bool>,
    common_attribute_data_separation: Option<bool>,
    common_attribute_separation_parameters: Option<Arc<CommonAttributeSeparationParameters>>,
    common_attribute_content: Option<Arc<[CommonAttributeContentEntry]>>,
    common_attribute_condition: Option<CommonAttributeCondition>,
    functional_option_settings: Option<Arc<FunctionalOptionSettings>>,
    catalog_attributes: Option<Arc<[DocumentAttribute]>>,
    catalog_hierarchy_type: Option<CatalogHierarchyType>,
    business_process_number: Option<BusinessProcessNumberSettings>,
    business_process_task: Option<Arc<str>>,
    business_process_attributes: Option<Arc<[DocumentAttribute]>>,
    business_process_sections: Option<Arc<[DocumentTabularSection]>>,
    data_processor_attributes: Option<Arc<[DocumentAttribute]>>,
    report_attributes: Option<Arc<[DocumentAttribute]>>,
    report_sections: Option<Arc<[DocumentTabularSection]>>,
    data_processor_sections: Option<Arc<[DocumentTabularSection]>>,
    document_number_settings: Option<DocumentNumberSettings>,
    document_numbering_settings: Option<DocumentNumberingSettings>,
    document_number_fixed_length: bool,
    document_attributes: Option<Arc<[DocumentAttribute]>>,
    document_standard_fill_checking: Option<Arc<[CatalogStandardAttributeFillChecking]>>,
    document_tabular_sections: Option<Arc<[DocumentTabularSection]>>,
    document_registers: Option<Arc<[String]>>,
    catalog_default_presentation: Option<CatalogDefaultPresentation>,
    catalog_attribute_names: Option<Arc<[String]>>,
    catalog_tabular_section_names: Option<Arc<[String]>>,
    catalog_predefined_names: Arc<[String]>,
    catalog_hierarchical: Option<bool>,
    reference_storage_number: Option<u32>,
    subsystem_parent: Option<Arc<str>>,
    subsystem_command_interface: Option<bool>,
    string_properties: Arc<[(Arc<str>, Arc<str>)]>,
}

/// Неизменяемое описание значения конфигурационного перечисления.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigurationEnumValue {
    owner_id: Arc<str>,
    value_id: Arc<str>,
    name: Arc<str>,
    synonyms: Arc<[(Arc<str>, Arc<str>)]>,
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
            synonyms: Arc::from([]),
        }
    }

    /// Добавляет приобретённый синоним значения для указанного кода языка.
    /// При отсутствии синонима основного языка используется имя значения.
    #[must_use]
    pub fn with_synonym(mut self, language: impl Into<String>, text: impl Into<String>) -> Self {
        let mut synonyms = self.synonyms.to_vec();
        synonyms.push((Arc::from(language.into()), Arc::from(text.into())));
        self.synonyms = synonyms.into();
        self
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
    /// Сохраняет приобретённый состав типов самого объекта, например константы.
    /// `None` при создании объекта сохраняет отличие неизвестных типов от пустых.
    #[must_use]
    pub fn with_declared_types(mut self, types: Vec<open_bsl::ColumnType>) -> Self {
        self.declared_types = Some(types.into());
        self
    }

    /// Возвращает полный приобретённый состав типов; `None` означает неизвестность.
    #[must_use]
    pub fn declared_types(&self) -> Option<&[open_bsl::ColumnType]> {
        self.declared_types.as_deref()
    }

    /// Возвращает известные связи параметров разделителя; `None` — неизвестные связи.
    #[must_use]
    pub fn common_attribute_separation_parameters(
        &self,
    ) -> Option<&CommonAttributeSeparationParameters> {
        self.common_attribute_separation_parameters.as_deref()
    }

    /// Сохраняет явно известный режим разделения данных общего реквизита.
    /// Неизвестный режим нельзя подменять выключенным разделением.
    #[must_use]
    pub const fn with_common_attribute_data_separation(mut self, enabled: bool) -> Self {
        self.common_attribute_data_separation = Some(enabled);
        self
    }

    /// Сохраняет приобретённую настройку автоиспользования общего реквизита.
    /// Без этой настройки значение остаётся неизвестным, а не выключенным.
    #[must_use]
    pub const fn with_common_attribute_auto_use(mut self, enabled: bool) -> Self {
        self.common_attribute_auto_use = Some(enabled);
        self
    }

    /// Создаёт объект без связанного модуля менеджера.
    #[must_use]
    pub fn new(id: impl Into<String>, name: impl Into<String>, kind: MetadataKind) -> Self {
        Self {
            id: Arc::from(id.into()),
            name: Arc::from(name.into()),
            reference_type_presentation: None,
            kind,
            declared_types: None,
            exchange_plan_attributes: None,
            manager_module: None,
            object_module: None,
            object_module_is_extension: false,
            own_extension_object_module_safe_mode: None,
            catalog_standard_attributes: None,
            characteristic_settings: None,
            route_points: None,
            information_register: None,
            accumulation_register_kind: None,
            accumulation_dimensions: None,
            accumulation_resources: None,
            characteristic_attributes: None,
            catalog_standard_fill_checking: None,
            business_process_standard_fill_checking: None,
            catalog_owners: None,
            catalog_common_attributes: None,
            common_attribute_auto_use: None,
            common_attribute_data_separation: None,
            common_attribute_separation_parameters: None,
            common_attribute_content: None,
            common_attribute_condition: None,
            functional_option_settings: None,
            catalog_attributes: None,
            catalog_hierarchy_type: None,
            business_process_number: None,
            business_process_task: None,
            business_process_attributes: None,
            business_process_sections: None,
            data_processor_attributes: None,
            report_attributes: None,
            report_sections: None,
            data_processor_sections: None,
            document_number_settings: None,
            document_numbering_settings: None,
            document_number_fixed_length: false,
            document_attributes: None,
            document_standard_fill_checking: None,
            document_tabular_sections: None,
            document_registers: None,
            catalog_default_presentation: None,
            catalog_attribute_names: None,
            catalog_tabular_section_names: None,
            catalog_predefined_names: Arc::from([]),
            catalog_hierarchical: None,
            reference_storage_number: None,
            subsystem_parent: None,
            subsystem_command_interface: None,
            string_properties: Arc::from([]),
        }
    }

    /// Сохраняет приобретённый основной номер объекта из DBNames.
    #[must_use]
    pub const fn with_reference_storage_number(mut self, number: u32) -> Self {
        self.reference_storage_number = Some(number);
        self
    }

    /// Возвращает приобретённый основной номер объекта из DBNames.
    #[must_use]
    pub const fn reference_storage_number(&self) -> Option<u32> {
        self.reference_storage_number
    }

    /// Связывает объект с именованным модулем менеджера конфигурации.
    #[must_use]
    pub fn with_manager_module(mut self, module: impl Into<String>) -> Self {
        self.manager_module = Some(Arc::from(module.into()));
        self
    }

    /// Связывает объект с именованным модулем объекта конфигурации.
    /// Прежняя политика источника расширения снимается.
    #[must_use]
    pub fn with_object_module(mut self, module: impl Into<String>) -> Self {
        self.object_module = Some(Arc::from(module.into()));
        self.object_module_is_extension = false;
        self.own_extension_object_module_safe_mode = None;
        self
    }

    /// Связывает объект с окончательным текстом модуля из расширения.
    /// Даже у заимствованного объекта имя модуля может остаться прежним.
    /// Отдельная политика собственного события при этом снимается.
    #[must_use]
    pub fn with_extension_object_module(mut self, module: impl Into<String>) -> Self {
        self.object_module = Some(Arc::from(module.into()));
        self.object_module_is_extension = true;
        self.own_extension_object_module_safe_mode = None;
        self
    }

    /// Связывает собственный объект расширения с модулем и приобретённой
    /// настройкой безопасности его источника. Политика применяется к событию
    /// выдачи нового кода, а не ко всем методам объекта.
    #[must_use]
    pub fn with_own_extension_object_module(
        mut self,
        module: impl Into<String>,
        safe_mode: bool,
    ) -> Self {
        self.object_module = Some(Arc::from(module.into()));
        self.object_module_is_extension = true;
        self.own_extension_object_module_safe_mode = Some(safe_mode);
        self
    }

    /// Возвращает известную политику события собственного объекта расширения.
    /// Отсутствие политики не подменяется выключенным безопасным режимом.
    #[must_use]
    pub const fn own_extension_object_module_safe_mode(&self) -> Option<bool> {
        self.own_extension_object_module_safe_mode
    }

    /// Сохраняет известные настройки стандартных реквизитов справочника.
    /// Отсутствие настройки отличается от нулевой длины реквизита.
    #[must_use]
    pub const fn with_catalog_standard_attributes(
        mut self,
        attributes: CatalogStandardAttributes,
    ) -> Self {
        self.catalog_standard_attributes = Some(attributes);
        self
    }

    /// Сохраняет полные приобретённые настройки плана видов характеристик.
    /// Не подставляет настройки справочника или состав типов по умолчанию.
    #[must_use]
    pub fn with_characteristic_settings(mut self, settings: CharacteristicSettings) -> Self {
        self.characteristic_settings = Some(Arc::new(settings));
        self
    }

    /// Возвращает настройки плана; `None` означает неизвестные данные.
    #[must_use]
    pub fn characteristic_settings(&self) -> Option<&CharacteristicSettings> {
        self.characteristic_settings.as_deref()
    }

    /// Сохраняет приобретённые настройки номера документа.
    /// Не вычисляет их по SQL-хранилищу и не подставляет значения по умолчанию.
    #[must_use]
    pub const fn with_document_number_settings(mut self, settings: DocumentNumberSettings) -> Self {
        self.document_number_settings = Some(settings);
        self
    }

    /// Возвращает настройки номера; `None` означает неизвестную форму снимка.
    #[must_use]
    pub const fn document_number_settings(&self) -> Option<DocumentNumberSettings> {
        self.document_number_settings
    }

    /// Сохраняет полностью приобретённую область нумерации документа.
    /// Состояние резервов сеанса в неизменяемые метаданные не включается.
    #[must_use]
    pub fn with_document_numbering_settings(mut self, settings: DocumentNumberingSettings) -> Self {
        self.document_numbering_settings = Some(settings);
        self
    }

    /// Возвращает известную область нумерации; `None` означает неизвестную
    /// или неполную форму, включая неизвестную ссылку на нумератор.
    #[must_use]
    pub fn document_numbering_settings(&self) -> Option<&DocumentNumberingSettings> {
        self.document_numbering_settings.as_ref()
    }

    /// Задаёт фиксированную допустимую длину строкового номера.
    #[must_use]
    pub const fn with_document_number_fixed_length(mut self, fixed: bool) -> Self {
        self.document_number_fixed_length = fixed;
        self
    }

    /// Возвращает признак фиксированной длины строкового номера.
    #[must_use]
    pub const fn document_number_fixed_length(&self) -> bool {
        self.document_number_fixed_length
    }

    /// Сохраняет полный приобретённый список собственных реквизитов документа.
    /// Пустой список отличается от неизвестной или неполной коллекции.
    #[must_use]
    pub fn with_document_attributes(mut self, attributes: Vec<DocumentAttribute>) -> Self {
        self.document_attributes = Some(attributes.into());
        self
    }

    /// Возвращает полную коллекцию; `None` означает, что она не приобретена.
    #[must_use]
    pub fn document_attributes(&self) -> Option<&[DocumentAttribute]> {
        self.document_attributes.as_deref()
    }

    /// Сохраняет полный приобретённый список проверок стандартных полей документа.
    #[must_use]
    pub fn with_document_standard_fill_checking(
        mut self,
        settings: Vec<CatalogStandardAttributeFillChecking>,
    ) -> Self {
        self.document_standard_fill_checking = Some(settings.into());
        self
    }

    /// Возвращает полный список; `None` означает неизвестные настройки.
    #[must_use]
    pub fn document_standard_fill_checking(
        &self,
    ) -> Option<&[CatalogStandardAttributeFillChecking]> {
        self.document_standard_fill_checking.as_deref()
    }

    /// Сохраняет полный список собственных табличных частей документа.
    #[must_use]
    pub fn with_document_tabular_sections(mut self, sections: Vec<DocumentTabularSection>) -> Self {
        self.document_tabular_sections = Some(sections.into());
        self
    }

    /// Возвращает полный список; `None` означает неизвестный состав частей.
    #[must_use]
    pub fn document_tabular_sections(&self) -> Option<&[DocumentTabularSection]> {
        self.document_tabular_sections.as_deref()
    }

    /// Сохраняет полный список идентификаторов регистров движений документа.
    /// Порядок задан конфигурацией; пустой список отличается от неизвестного.
    #[must_use]
    pub fn with_document_registers(mut self, registers: Vec<String>) -> Self {
        self.document_registers = Some(registers.into());
        self
    }

    /// Возвращает приобретённый список; `None` означает неизвестные связи.
    #[must_use]
    pub fn document_registers(&self) -> Option<&[String]> {
        self.document_registers.as_deref()
    }

    /// Сохраняет известную настройку основного представления справочника.
    /// Она не выводится из типа кода или длины наименования.
    #[must_use]
    pub const fn with_catalog_default_presentation(
        mut self,
        presentation: CatalogDefaultPresentation,
    ) -> Self {
        self.catalog_default_presentation = Some(presentation);
        self
    }

    /// Сохраняет полный приобретённый список собственных реквизитов справочника.
    /// Пустой список означает известное отсутствие реквизитов; поля табличных
    /// частей в этот список не включаются. Типы и значения здесь не задаются.
    #[must_use]
    pub fn with_catalog_attribute_names(
        mut self,
        names: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.catalog_attribute_names =
            Some(names.into_iter().map(Into::into).collect::<Vec<_>>().into());
        self
    }

    /// Сохраняет полный приобретённый список собственных табличных частей.
    /// Имена реквизитов строк сюда не включаются. Пустой список означает
    /// известное отсутствие частей; значения строк здесь не задаются.
    #[must_use]
    pub fn with_catalog_tabular_section_names(
        mut self,
        names: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.catalog_tabular_section_names =
            Some(names.into_iter().map(Into::into).collect::<Vec<_>>().into());
        self
    }

    /// Сохраняет полный приобретённый список собственных реквизитов отчёта.
    #[must_use]
    pub fn with_report_attributes(mut self, attributes: Vec<DocumentAttribute>) -> Self {
        self.report_attributes = Some(attributes.into());
        self
    }

    /// Возвращает реквизиты отчёта; `None` означает неизвестный состав.
    pub fn report_attributes(&self) -> Option<&[DocumentAttribute]> {
        self.report_attributes.as_deref()
    }

    /// Сохраняет полный приобретённый список табличных частей отчёта.
    #[must_use]
    pub fn with_report_tabular_sections(mut self, sections: Vec<DocumentTabularSection>) -> Self {
        self.report_sections = Some(sections.into());
        self
    }

    /// Возвращает табличные части отчёта; `None` означает неизвестный состав.
    pub fn report_tabular_sections(&self) -> Option<&[DocumentTabularSection]> {
        self.report_sections.as_deref()
    }

    fn executable_attributes(&self) -> Option<&[DocumentAttribute]> {
        if self.kind == MetadataKind::Report {
            self.report_attributes()
        } else {
            self.data_processor_attributes()
        }
    }

    fn executable_sections(&self) -> Option<&[DocumentTabularSection]> {
        if self.kind == MetadataKind::Report {
            self.report_tabular_sections()
        } else {
            self.data_processor_tabular_sections()
        }
    }

    /// Сохраняет полный список собственных реквизитов обработки.
    #[must_use]
    pub fn with_data_processor_attributes(mut self, attributes: Vec<DocumentAttribute>) -> Self {
        self.data_processor_attributes = Some(attributes.into());
        self
    }

    /// Возвращает полный список реквизитов обработки; `None` означает неизвестность.
    #[must_use]
    pub fn data_processor_attributes(&self) -> Option<&[DocumentAttribute]> {
        self.data_processor_attributes.as_deref()
    }

    /// Сохраняет полный список собственных табличных частей обработки.
    #[must_use]
    pub fn with_data_processor_tabular_sections(
        mut self,
        sections: Vec<DocumentTabularSection>,
    ) -> Self {
        self.data_processor_sections = Some(sections.into());
        self
    }

    /// Возвращает полный список частей обработки; `None` означает неизвестность.
    #[must_use]
    pub fn data_processor_tabular_sections(&self) -> Option<&[DocumentTabularSection]> {
        self.data_processor_sections.as_deref()
    }

    /// Запоминает приобретённый признак иерархического справочника.
    /// Он определяет доступность записи родителя нового элемента;
    /// отсутствие сведений не подменяется значением `false`.
    #[must_use]
    pub const fn with_catalog_hierarchical(mut self, hierarchical: bool) -> Self {
        self.catalog_hierarchical = Some(hierarchical);
        self
    }

    /// Сохраняет приобретённые имена предопределённых элементов справочника
    /// или плана видов характеристик.
    /// Имена описывают свойства менеджера; UUID записей здесь не задаются.
    #[must_use]
    pub fn with_catalog_predefined_names(
        mut self,
        names: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.catalog_predefined_names =
            names.into_iter().map(Into::into).collect::<Vec<_>>().into();
        self
    }

    /// Возвращает приобретённые имена предопределённых элементов менеджера.
    #[must_use]
    pub fn catalog_predefined_names(&self) -> &[String] {
        &self.catalog_predefined_names
    }

    /// Указывает родительскую подсистему для вложенного объекта.
    #[must_use]
    pub fn with_subsystem_parent(mut self, parent: impl Into<String>) -> Self {
        self.subsystem_parent = Some(Arc::from(parent.into()));
        self
    }

    /// Запоминает признак включения подсистемы в командный интерфейс.
    #[must_use]
    pub fn with_subsystem_command_interface(mut self, included: bool) -> Self {
        self.subsystem_command_interface = Some(included);
        self
    }

    /// Задаёт приобретённое представление типа ссылки этого объекта.
    /// Идентичность типа и квалифицированные имена поиска сохраняются.
    /// При отсутствии значения используется прежнее имя объекта.
    #[must_use]
    pub fn with_reference_type_presentation(mut self, presentation: impl Into<String>) -> Self {
        self.reference_type_presentation = Some(Arc::from(presentation.into()));
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

    /// Возвращает имя связанного модуля объекта.
    #[must_use]
    pub fn object_module(&self) -> Option<&str> {
        self.object_module.as_deref()
    }

    /// Возвращает признак окончательного текста модуля объекта из расширения.
    /// При отсутствии модуля возвращает `false`.
    #[must_use]
    pub const fn object_module_is_extension(&self) -> bool {
        self.object_module_is_extension
    }

    /// Возвращает известные настройки стандартных реквизитов справочника.
    /// `None` означает отсутствие сведений, а не стандартные значения.
    #[must_use]
    pub const fn catalog_standard_attributes(&self) -> Option<CatalogStandardAttributes> {
        self.catalog_standard_attributes
    }

    /// Сохраняет полную таблицу проверок стандартных реквизитов в её порядке.
    /// Пустой список означает известное отсутствие настроек.
    #[must_use]
    pub fn with_catalog_standard_fill_checking(
        mut self,
        settings: Vec<CatalogStandardAttributeFillChecking>,
    ) -> Self {
        self.catalog_standard_fill_checking = Some(settings.into());
        self
    }

    /// Возвращает полную таблицу проверок; `None` означает неизвестную форму.
    #[must_use]
    pub fn catalog_standard_fill_checking(
        &self,
    ) -> Option<&[CatalogStandardAttributeFillChecking]> {
        self.catalog_standard_fill_checking.as_deref()
    }

    /// Сохраняет полный приобретённый список владельцев справочника в его порядке.
    /// Пустой список означает известное отсутствие владельцев.
    #[must_use]
    pub fn with_catalog_owners(mut self, owners: Vec<String>) -> Self {
        self.catalog_owners = Some(owners.into());
        self
    }

    /// Возвращает приобретённые идентификаторы владельцев справочника.
    /// `None` означает неизвестный список, а не отсутствие владельцев.
    #[must_use]
    pub fn catalog_owners(&self) -> Option<&[String]> {
        self.catalog_owners.as_deref()
    }

    /// Возвращает полный применимый состав общих реквизитов справочника.
    /// `None` означает неизвестный состав; собственные реквизиты сюда не входят.
    #[must_use]
    pub fn catalog_common_attributes(&self) -> Option<&[DocumentAttribute]> {
        self.catalog_common_attributes.as_deref()
    }

    /// Возвращает полный приобретённый состав реквизитов плана видов характеристик.
    /// `None` означает неизвестный состав, а не пустой список.
    #[must_use]
    pub fn characteristic_attributes(&self) -> Option<&[DocumentAttribute]> {
        self.characteristic_attributes.as_deref()
    }

    /// Сохраняет полный состав реквизитов плана видов характеристик.
    /// Использует тот же неизменяемый описатель, что реквизиты документов и справочников.
    #[must_use]
    pub fn with_characteristic_attributes(mut self, attributes: Vec<DocumentAttribute>) -> Self {
        self.characteristic_attributes = Some(attributes.into());
        self
    }

    /// Возвращает полные собственные реквизиты справочника с типами и настройками.
    /// `None` означает неизвестный состав, а не пустую коллекцию.
    #[must_use]
    pub fn catalog_attributes(&self) -> Option<&[DocumentAttribute]> {
        self.catalog_attributes.as_deref()
    }

    /// Сохраняет собственные реквизиты отдельно от применимых общих реквизитов.
    #[must_use]
    pub fn with_catalog_attributes(mut self, attributes: Vec<DocumentAttribute>) -> Self {
        self.catalog_attributes = Some(attributes.into());
        self
    }

    /// Сохраняет полный применимый состав общих реквизитов отдельно от собственных.
    #[must_use]
    pub fn with_catalog_common_attributes(mut self, attributes: Vec<DocumentAttribute>) -> Self {
        self.catalog_common_attributes = Some(attributes.into());
        self
    }

    /// Возвращает приобретённую настройку основного представления.
    /// `None` означает отсутствие сведений, а не выбор по наименованию.
    #[must_use]
    pub const fn catalog_default_presentation(&self) -> Option<CatalogDefaultPresentation> {
        self.catalog_default_presentation
    }

    /// Возвращает собственные имена реквизитов без полей табличных частей.
    /// `None` означает, что полный список не приобретён; известный пустой
    /// список возвращается как `Some` с пустым срезом.
    #[must_use]
    pub fn catalog_attribute_names(&self) -> Option<&[String]> {
        self.catalog_attribute_names.as_deref()
    }

    /// Возвращает собственные имена табличных частей без реквизитов строк.
    /// `None` означает, что полный список не приобретён.
    #[must_use]
    pub fn catalog_tabular_section_names(&self) -> Option<&[String]> {
        self.catalog_tabular_section_names.as_deref()
    }

    /// Возвращает известный признак иерархии справочника.
    /// `None` означает отсутствие приобретённых сведений.
    #[must_use]
    pub const fn catalog_hierarchical(&self) -> Option<bool> {
        self.catalog_hierarchical
    }

    /// Сохраняет приобретённый вид иерархии, даже если иерархия выключена.
    #[must_use]
    pub const fn with_catalog_hierarchy_type(mut self, kind: CatalogHierarchyType) -> Self {
        self.catalog_hierarchy_type = Some(kind);
        self
    }

    /// Возвращает приобретённый вид; `None` означает неизвестные сведения.
    #[must_use]
    pub const fn catalog_hierarchy_type(&self) -> Option<CatalogHierarchyType> {
        self.catalog_hierarchy_type
    }

    /// Возвращает идентификатор родительской подсистемы, если она есть.
    #[must_use]
    pub fn subsystem_parent(&self) -> Option<&str> {
        self.subsystem_parent.as_deref()
    }

    /// Возвращает приобретённый признак включения подсистемы.
    #[must_use]
    pub const fn subsystem_command_interface(&self) -> Option<bool> {
        self.subsystem_command_interface
    }
}

/// Источник подробностей объектов одного неизменяемого индекса.
pub trait MetadataObjectSource: Send + Sync + std::fmt::Debug {
    /// Приобретает подробности достигнутой группы перед отдельными обращениями.
    /// По умолчанию источник загружает каждый объект отдельно.
    ///
    /// # Errors
    /// Возвращает ошибку приобретения подробностей или их зависимостей.
    fn prepare_objects(&self, _ids: &[&str]) -> Result<(), String> {
        Ok(())
    }

    /// Загружает подробности одного объекта и необходимые ему зависимости.
    ///
    /// # Errors
    /// Возвращает описание ошибки источника или неполного описания объекта.
    fn load_object(&self, id: &str) -> Result<MetadataObject, String>;
}

#[cfg(test)]
#[path = "lazy_metadata_tests.rs"]
mod lazy_metadata_tests;

type MetadataCollectionKey = (std::mem::Discriminant<MetadataKind>, Option<Arc<str>>);

/// Индекс метаданных с необязательной адресной загрузкой подробностей.
#[derive(Debug)]
pub struct MetadataCatalog {
    identity: Arc<str>,
    objects: Arc<[MetadataObject]>,
    object_index: HashMap<Arc<str>, usize>,
    collection_positions: HashMap<MetadataCollectionKey, Vec<usize>>,
    common_attribute_positions: Vec<usize>,
    object_source: Option<Arc<dyn MetadataObjectSource>>,
    file_source: Option<file_source::Attached>,
    file_validations: Vec<std::sync::OnceLock<Result<(), String>>>,
    object_details: Vec<std::sync::OnceLock<Result<MetadataObject, String>>>,
    common_attribute_ids: Option<Arc<[String]>>,
    configuration: Option<ConfigurationDescription>,
    configuration_object_id: Option<Arc<str>>,
    configuration_extensions: Option<Arc<ConfigurationExtensionsSnapshot>>,
    configuration_changed: Option<bool>,
    default_interface_id: Option<[u8; 16]>,
    enumeration_values: Arc<[ConfigurationEnumValue]>,
    reference_types: Vec<(Arc<str>, Arc<ValueTypeDescriptor>)>,
    catalog_object_types: Vec<(Arc<str>, Arc<ValueTypeDescriptor>)>,
    document_object_types: Vec<(Arc<str>, Arc<ValueTypeDescriptor>)>,
    business_process_object_types: Vec<(Arc<str>, Arc<ValueTypeDescriptor>)>,
    data_processor_object_types: Vec<(Arc<str>, Arc<ValueTypeDescriptor>)>,
    report_object_types: Vec<(Arc<str>, Arc<ValueTypeDescriptor>)>,
    report_section_types: Vec<document_tabular_section::Types>,
    data_processor_section_types: Vec<document_tabular_section::Types>,
    characteristic_object_types: Vec<(Arc<str>, Arc<ValueTypeDescriptor>)>,
    document_section_types: Vec<document_tabular_section::Types>,
    business_section_types: Vec<document_tabular_section::Types>,
    information_record_set_types: Vec<(Arc<str>, Arc<ValueTypeDescriptor>)>,
    catalog_selection_types: Vec<(Arc<str>, Arc<ValueTypeDescriptor>)>,
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
    /// Присоединяет типы объекта, приобретённые из авторитетного описания БД.
    ///
    /// # Errors
    /// Возвращает ошибку для отсутствующего владельца.
    pub fn with_object_declared_types(
        mut self,
        owner: &str,
        types: Vec<open_bsl::ColumnType>,
    ) -> Result<Self, String> {
        let index = self
            .object_index
            .get(owner)
            .copied()
            .ok_or_else(|| format!("владелец типов «{owner}» отсутствует"))?;
        Arc::make_mut(&mut self.objects)[index].declared_types = Some(types.into());
        Ok(self)
    }

    /// Присоединяет полный приобретённый состав собственных реквизитов плана обмена.
    /// `None` сохраняет неизвестность; пустой список подтверждает отсутствие реквизитов.
    ///
    /// # Errors
    /// Возвращает ошибку для отсутствующего владельца или другого вида объекта.
    pub fn with_exchange_plan_attributes(
        mut self,
        owner: &str,
        attributes: Option<Vec<DocumentAttribute>>,
    ) -> Result<Self, String> {
        let index = self
            .object_index
            .get(owner)
            .copied()
            .filter(|&i| self.objects[i].kind == MetadataKind::ExchangePlan)
            .ok_or_else(|| format!("план обмена «{owner}» отсутствует"))?;
        Arc::make_mut(&mut self.objects)[index].exchange_plan_attributes =
            attributes.map(Into::into);
        Ok(self)
    }

    /// Приобретает метаданные достигнутой группы через источник подробностей.
    /// Корневой индекс и членство коллекций не меняются; BSL не исполняется.
    /// Для полностью приобретённого каталога дополнительной работы нет.
    ///
    /// # Errors
    /// Возвращает ошибку источника подробностей или их зависимостей.
    pub fn prepare_objects(&self, ids: &[&str]) -> Result<(), String> {
        if let Some(source) = &self.object_source {
            source.prepare_objects(ids)?;
        }
        Ok(())
    }

    /// Присоединяет приобретённые настройки и собственные реквизиты плана видов характеристик.
    /// Неизвестные данные сохраняются как `None`, известный пустой состав — как `Some([])`.
    ///
    /// # Errors
    /// Возвращает ошибку для отсутствующего владельца или объекта другого вида.
    pub fn with_characteristic_metadata(
        mut self,
        owner: &str,
        settings: Option<CharacteristicSettings>,
        attributes: Option<Vec<DocumentAttribute>>,
    ) -> Result<Self, CatalogError> {
        let index = self
            .objects
            .iter()
            .position(|object| {
                object.id() == owner && object.kind() == MetadataKind::ChartOfCharacteristicTypes
            })
            .ok_or_else(|| CatalogError::UnknownCharacteristicOwner(owner.into()))?;
        let object = &mut Arc::make_mut(&mut self.objects)[index];
        object.characteristic_settings = settings.map(Arc::new);
        object.characteristic_attributes = attributes.map(Into::into);
        Ok(self)
    }

    /// Сохраняет полный корневой список общих реквизитов конфигурации.
    /// Пустой список означает известное отсутствие общих реквизитов.
    #[must_use]
    pub fn with_common_attribute_ids(mut self, ids: Vec<String>) -> Self {
        self.common_attribute_positions = ids
            .iter()
            .filter_map(|id| self.object_index.get(id.as_str()).copied())
            .collect();
        self.common_attribute_ids = Some(ids.into());
        self
    }

    /// Возвращает полный корневой список; `None` означает неизвестный состав.
    #[must_use]
    pub fn common_attribute_ids(&self) -> Option<&[String]> {
        self.common_attribute_ids.as_deref()
    }

    /// Присоединяет приобретённые связи параметров разделяющего общего реквизита.
    ///
    /// # Errors
    /// Возвращает ошибку для отсутствующего или неразделяющего общего реквизита либо связи,
    /// которая не указывает на объявленный параметр сеанса.
    pub fn with_common_attribute_separation_parameters(
        mut self,
        owner: &str,
        settings: CommonAttributeSeparationParameters,
    ) -> Result<Self, CatalogError> {
        let index = self
            .objects
            .iter()
            .position(|object| {
                object.id() == owner
                    && object.kind() == MetadataKind::CommonAttribute
                    && object.common_attribute_data_separation == Some(true)
            })
            .ok_or_else(|| CatalogError::InvalidCommonAttributeSeparation(owner.into()))?;
        for parameter in [&settings.value_parameter, &settings.use_parameter]
            .into_iter()
            .flatten()
        {
            if !self
                .object_by_id(parameter)
                .is_some_and(|object| object.kind() == MetadataKind::SessionParameter)
            {
                return Err(CatalogError::InvalidCommonAttributeSeparation(owner.into()));
            }
        }
        Arc::make_mut(&mut self.objects)[index].common_attribute_separation_parameters =
            Some(Arc::new(settings));
        Ok(self)
    }

    /// Присоединяет полный применимый состав общих реквизитов справочника.
    ///
    /// # Errors
    /// Возвращает ошибку для отсутствующего объекта или объекта другого вида.
    pub fn with_catalog_common_attributes(
        mut self,
        owner: &str,
        attributes: Vec<DocumentAttribute>,
    ) -> Result<Self, CatalogError> {
        let index = self
            .objects
            .iter()
            .position(|object| object.id() == owner && object.kind() == MetadataKind::Catalog)
            .ok_or_else(|| CatalogError::UnknownCatalogOwner(owner.into()))?;
        Arc::make_mut(&mut self.objects)[index].catalog_common_attributes = Some(attributes.into());
        Ok(self)
    }

    /// Присоединяет полные собственные реквизиты приобретённого справочника.
    ///
    /// # Errors
    /// Возвращает ошибку для отсутствующего объекта или объекта другого вида.
    pub fn with_catalog_attributes(
        mut self,
        owner: &str,
        attributes: Vec<DocumentAttribute>,
    ) -> Result<Self, CatalogError> {
        let index = self
            .objects
            .iter()
            .position(|object| object.id() == owner && object.kind() == MetadataKind::Catalog)
            .ok_or_else(|| CatalogError::UnknownCatalogOwner(owner.into()))?;
        Arc::make_mut(&mut self.objects)[index].catalog_attributes = Some(attributes.into());
        Ok(self)
    }

    /// Неизменяемые описания типов изменяемых объектов справочников.
    /// У них нет декодера графа: объявление типа не разрешает перенос
    /// переменных модуля объекта или данных живого сеанса.
    pub fn catalog_object_types(&self) -> impl Iterator<Item = &Arc<ValueTypeDescriptor>> {
        self.catalog_object_types.iter().map(|(_, ty)| ty)
    }

    /// Типы объектов документов для регистрации без данных живого сеанса.
    pub fn document_object_types(&self) -> impl Iterator<Item = &Arc<ValueTypeDescriptor>> {
        self.document_object_types.iter().map(|(_, ty)| ty)
    }

    /// Неизменяемые типы объектов бизнес-процессов для регистрации в движке.
    /// Регистрация описания не разрешает перенос состояния экземпляра.
    pub fn business_process_object_types(&self) -> impl Iterator<Item = &Arc<ValueTypeDescriptor>> {
        self.business_process_object_types.iter().map(|(_, ty)| ty)
    }

    /// Типы объектов обработки; описание не разрешает перенос памяти экземпляра.
    pub fn data_processor_object_types(&self) -> impl Iterator<Item = &Arc<ValueTypeDescriptor>> {
        self.data_processor_object_types.iter().map(|(_, ty)| ty)
    }

    /// Неизменяемые описания типов объектов отчётов без памяти экземпляров.
    pub fn report_object_types(&self) -> impl Iterator<Item = &Arc<ValueTypeDescriptor>> {
        self.report_object_types.iter().map(|(_, ty)| ty)
    }

    /// Описания типов собственных табличных частей отчётов и их строк.
    pub fn report_tabular_section_types(&self) -> impl Iterator<Item = &Arc<ValueTypeDescriptor>> {
        self.report_section_types
            .iter()
            .flat_map(|types| [&types.section_type, &types.row_type])
    }

    /// Типы собственных частей обработки и их строк без изменяемых данных сеанса.
    pub fn data_processor_tabular_section_types(
        &self,
    ) -> impl Iterator<Item = &Arc<ValueTypeDescriptor>> {
        self.data_processor_section_types
            .iter()
            .flat_map(|types| [&types.section_type, &types.row_type])
    }

    /// Типы объектов планов видов характеристик без данных живого сеанса.
    pub fn characteristic_object_types(&self) -> impl Iterator<Item = &Arc<ValueTypeDescriptor>> {
        self.characteristic_object_types.iter().map(|(_, ty)| ty)
    }

    /// Типы собственных наборов записей регистров сведений.
    /// Регистрация не разрешает перенос строк или живого сеанса через граф.
    pub fn information_record_set_types(&self) -> impl Iterator<Item = &Arc<ValueTypeDescriptor>> {
        self.information_record_set_types.iter().map(|(_, ty)| ty)
    }

    /// Типы приобретённых табличных частей документов и их строк.
    /// Описания разделяются сеансами; строки и значения в них не хранятся.
    pub fn document_tabular_section_types(
        &self,
    ) -> impl Iterator<Item = &Arc<ValueTypeDescriptor>> {
        self.document_section_types
            .iter()
            .flat_map(|types| [&types.section_type, &types.row_type])
    }

    /// Типы приобретённых табличных частей бизнес-процессов и их строк.
    /// Описания не содержат изменяемых значений сеанса.
    pub fn business_process_tabular_section_types(
        &self,
    ) -> impl Iterator<Item = &Arc<ValueTypeDescriptor>> {
        self.business_section_types
            .iter()
            .flat_map(|types| [&types.section_type, &types.row_type])
    }

    /// Неизменяемые типы ссылок для регистрации в движке конфигурации.
    /// Описания создаются один раз вместе с каталогом и разделяются сеансами.
    pub fn value_types(&self) -> impl Iterator<Item = &Arc<ValueTypeDescriptor>> {
        self.reference_types
            .iter()
            .map(|(_, description)| description)
    }

    /// Типы выборок конкретных справочников для регистрации в движке.
    /// Изменяемые курсоры не имеют декодера графа значений.
    pub fn catalog_selection_types(&self) -> impl Iterator<Item = &Arc<ValueTypeDescriptor>> {
        self.catalog_selection_types.iter().map(|(_, ty)| ty)
    }

    /// Типы ссылок с декодерами для движка и графа неизменяемых значений.
    /// Декодеры удерживают только исходный каталог, не образуя цикл с описаниями.
    pub fn transferable_value_types(
        self: &Arc<Self>,
    ) -> impl Iterator<Item = Arc<ValueTypeDescriptor>> + '_ {
        self.reference_types.iter().map(|(id, ty)| {
            let catalog = Arc::clone(self);
            let id = Arc::clone(id);
            Arc::new(
                ty.as_ref()
                    .clone()
                    .with_materializer(Arc::new(move |bytes| {
                        if catalog
                            .object_by_id(&id)
                            .is_some_and(|object| object.kind == MetadataKind::Enumeration)
                        {
                            let member = std::str::from_utf8(bytes).map_err(|_| {
                                RtError::DynamicError(
                                    "повреждён идентификатор значения перечисления".into(),
                                )
                            })?;
                            return enumeration_reference_value(Arc::clone(&catalog), &id, member);
                        }
                        let uuid = bytes.try_into().map_err(|_| {
                            RtError::DynamicError("длина UUID ссылки не равна 16".into())
                        })?;
                        configuration_reference_value(Arc::clone(&catalog), &id, uuid)
                    })),
            )
        })
    }

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
        let mut names = open_bsl::NameInterner::new();
        let mut object_names = HashSet::with_capacity(objects.len());
        for object in &objects {
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
            // Интернер использует тот же `folded_eq`, включая проверку
            // коллизий: проверка имён не требует попарного обхода каталога.
            let parent = if object.kind == MetadataKind::Subsystem {
                object.subsystem_parent.as_deref()
            } else {
                None
            };
            let name = names.intern(&object.name);
            if !object_names.insert((std::mem::discriminant(&object.kind), parent, name)) {
                return Err(CatalogError::DuplicateObjectName {
                    name: object.name.to_string(),
                });
            }
            if object.manager_module.as_deref() == Some("") {
                return Err(CatalogError::EmptyManagerModule {
                    object: object.id.to_string(),
                });
            }
            if object.object_module.as_deref() == Some("") {
                return Err(CatalogError::EmptyObjectModule {
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
        for object in &objects {
            if let Some(parent) = &object.subsystem_parent
                && (object.kind != MetadataKind::Subsystem
                    || !objects.iter().any(|candidate| {
                        candidate.id == *parent && candidate.kind == MetadataKind::Subsystem
                    }))
            {
                return Err(CatalogError::UnknownSubsystemParent(parent.to_string()));
            }
        }
        let reference_types = objects
            .iter()
            .filter(|object| {
                matches!(
                    object.kind,
                    MetadataKind::Catalog
                        | MetadataKind::Document
                        | MetadataKind::Enumeration
                        | MetadataKind::ChartOfCharacteristicTypes
                        | MetadataKind::ChartOfCalculationTypes
                        | MetadataKind::ChartOfAccounts
                        | MetadataKind::BusinessProcess
                        | MetadataKind::Task
                        | MetadataKind::ExchangePlan
                )
            })
            .map(|object| {
                let (identity_kind, russian, english) = if object.kind == MetadataKind::Document {
                    ("document-ref", "ДокументСсылка", "DocumentRef")
                } else if object.kind == MetadataKind::Enumeration {
                    ("enum-ref", "ПеречислениеСсылка", "EnumRef")
                } else if object.kind == MetadataKind::ChartOfCharacteristicTypes {
                    (
                        "characteristic-ref",
                        "ПланВидовХарактеристикСсылка",
                        "ChartOfCharacteristicTypesRef",
                    )
                } else if object.kind == MetadataKind::ChartOfCalculationTypes {
                    (
                        "calculation-type-ref",
                        "ПланВидовРасчетаСсылка",
                        "ChartOfCalculationTypesRef",
                    )
                } else if object.kind == MetadataKind::ChartOfAccounts {
                    (
                        "chart-of-accounts-ref",
                        "ПланСчетовСсылка",
                        "ChartOfAccountsRef",
                    )
                } else if object.kind == MetadataKind::BusinessProcess {
                    (
                        "business-process-ref",
                        "БизнесПроцессСсылка",
                        "BusinessProcessRef",
                    )
                } else if object.kind == MetadataKind::Task {
                    ("task-ref", "ЗадачаСсылка", "TaskRef")
                } else if object.kind == MetadataKind::ExchangePlan {
                    ("exchange-plan-ref", "ПланОбменаСсылка", "ExchangePlanRef")
                } else {
                    ("catalog-ref", "СправочникСсылка", "CatalogRef")
                };
                (
                    Arc::clone(&object.id),
                    Arc::new(ValueTypeDescriptor::new(
                        "bsl-orm",
                        format!("{identity}/{identity_kind}/{}", object.id),
                        object
                            .reference_type_presentation
                            .as_deref()
                            .unwrap_or(&object.name)
                            .to_owned(),
                        [
                            format!("{russian}.{}", object.name),
                            format!("{english}.{}", object.name),
                        ],
                    )),
                )
            })
            .collect();
        let catalog_object_types = objects
            .iter()
            .filter(|object| object.kind == MetadataKind::Catalog)
            .map(|object| {
                (
                    Arc::clone(&object.id),
                    Arc::new(ValueTypeDescriptor::new(
                        "bsl-orm",
                        format!("{identity}/catalog-object/{}", object.id),
                        format!("Справочник объект: {}", object.name),
                        [
                            format!("СправочникОбъект.{}", object.name),
                            format!("CatalogObject.{}", object.name),
                        ],
                    )),
                )
            })
            .collect();
        let document_object_types = objects
            .iter()
            .filter(|object| object.kind == MetadataKind::Document)
            .map(|object| {
                (
                    Arc::clone(&object.id),
                    Arc::new(ValueTypeDescriptor::new(
                        "bsl-orm",
                        format!("{identity}/document-object/{}", object.id),
                        format!("Документ объект: {}", object.name),
                        [
                            format!("ДокументОбъект.{}", object.name),
                            format!("DocumentObject.{}", object.name),
                        ],
                    )),
                )
            })
            .collect();
        let data_processor_object_types = objects
            .iter()
            .filter(|object| object.kind == MetadataKind::DataProcessor)
            .map(|object| {
                (
                    Arc::clone(&object.id),
                    Arc::new(ValueTypeDescriptor::new(
                        "bsl-orm",
                        format!("{identity}/data-processor-object/{}", object.id),
                        format!("Обработка объект: {}", object.name),
                        [
                            format!("ОбработкаОбъект.{}", object.name),
                            format!("DataProcessorObject.{}", object.name),
                        ],
                    )),
                )
            })
            .collect();
        let report_object_types = objects
            .iter()
            .filter(|object| object.kind == MetadataKind::Report)
            .map(|object| {
                (
                    Arc::clone(&object.id),
                    Arc::new(ValueTypeDescriptor::new(
                        "bsl-orm",
                        format!("{identity}/report-object/{}", object.id),
                        format!("Report Object: {}", object.name),
                        [
                            format!("ОтчетОбъект.{}", object.name),
                            format!("ReportObject.{}", object.name),
                        ],
                    )),
                )
            })
            .collect();
        let report_section_types = document_tabular_section::report_types(&identity, &objects);
        let data_processor_section_types =
            document_tabular_section::processor_types(&identity, &objects);
        let business_process_object_types = objects
            .iter()
            .filter(|object| object.kind == MetadataKind::BusinessProcess)
            .map(|object| {
                (
                    Arc::clone(&object.id),
                    Arc::new(ValueTypeDescriptor::new(
                        "bsl-orm",
                        format!("{identity}/business-process-object/{}", object.id),
                        // НЕ ИЗМЕРЕНО(TYPE.BUSINESS_PROCESS_OBJECT_PRESENTATION)
                        // Каноническое имя не подменяет измерение локали и синонима.
                        format!("БизнесПроцессОбъект.{}", object.name),
                        [
                            format!("БизнесПроцессОбъект.{}", object.name),
                            format!("BusinessProcessObject.{}", object.name),
                        ],
                    )),
                )
            })
            .collect();
        let characteristic_object_types = objects
            .iter()
            .filter(|object| object.kind == MetadataKind::ChartOfCharacteristicTypes)
            .map(|object| {
                (
                    Arc::clone(&object.id),
                    Arc::new(ValueTypeDescriptor::new(
                        "bsl-orm",
                        format!("{identity}/characteristic-object/{}", object.id),
                        format!("План видов характеристик объект: {}", object.name),
                        [
                            format!("ПланВидовХарактеристикОбъект.{}", object.name),
                            format!("ChartOfCharacteristicTypesObject.{}", object.name),
                        ],
                    )),
                )
            })
            .collect();
        let catalog_selection_types = objects
            .iter()
            .filter(|object| object.kind == MetadataKind::Catalog)
            .map(|object| {
                (
                    Arc::clone(&object.id),
                    Arc::new(ValueTypeDescriptor::new(
                        "bsl-orm",
                        format!("{identity}/catalog-selection/{}", object.id),
                        format!("Справочник выборка: {}", object.name),
                        [
                            format!("СправочникВыборка.{}", object.name),
                            format!("CatalogSelection.{}", object.name),
                        ],
                    )),
                )
            })
            .collect();
        let document_section_types = document_tabular_section::types(&identity, &objects);
        let information_record_set_types = objects
            .iter()
            .filter(|object| object.kind() == MetadataKind::InformationRegister)
            .map(|object| {
                (
                    Arc::clone(&object.id),
                    Arc::new(ValueTypeDescriptor::new(
                        "bsl-orm",
                        format!("{identity}/information-record-set/{}", object.id()),
                        format!("Регистр сведений набор записей: {}", object.name()),
                        [
                            format!("РегистрСведенийНаборЗаписей.{}", object.name()),
                            format!("InformationRegisterRecordSet.{}", object.name()),
                        ],
                    )),
                )
            })
            .collect();
        let business_section_types = document_tabular_section::business_types(&identity, &objects);
        let object_index = objects
            .iter()
            .enumerate()
            .map(|(index, object)| (Arc::clone(&object.id), index))
            .collect();
        // Членство коллекций неизменно. Один проход сохраняет порядок каталога,
        // чтобы длина и доступ по индексу не фильтровали все объекты при каждом шаге.
        let mut collection_positions = HashMap::<_, Vec<usize>>::new();
        for (position, object) in objects.iter().enumerate() {
            let parent = if object.kind == MetadataKind::Subsystem {
                object.subsystem_parent.clone()
            } else {
                None
            };
            collection_positions
                .entry((std::mem::discriminant(&object.kind), parent))
                .or_default()
                .push(position);
        }
        Ok(Self {
            identity: identity.into(),
            objects: objects.into(),
            object_index,
            collection_positions,
            common_attribute_positions: Vec::new(),
            object_source: None,
            file_source: None,
            file_validations: Vec::new(),
            object_details: Vec::new(),
            common_attribute_ids: None,
            configuration: None,
            configuration_object_id: None,
            configuration_extensions: None,
            configuration_changed: None,
            default_interface_id: None,
            enumeration_values: Arc::from([]),
            reference_types,
            catalog_object_types,
            document_object_types,
            business_process_object_types,
            data_processor_object_types,
            report_object_types,
            report_section_types,
            data_processor_section_types,
            characteristic_object_types,
            business_section_types,
            document_section_types,
            information_record_set_types,
            catalog_selection_types,
        })
    }

    /// Присоединяет приобретённые реквизиты к документу до создания сеансов.
    /// Сохраняет остальные свойства каталога и идентичности его типов.
    ///
    /// # Errors
    /// Возвращает ошибку для неизвестного владельца или владельца другого вида.
    pub fn with_document_attributes(
        mut self,
        owner: &str,
        attributes: Vec<DocumentAttribute>,
    ) -> Result<Self, CatalogError> {
        let index = self
            .objects
            .iter()
            .position(|object| object.id() == owner && object.kind() == MetadataKind::Document)
            .ok_or_else(|| CatalogError::UnknownDocumentOwner(owner.into()))?;
        Arc::make_mut(&mut self.objects)[index].document_attributes = Some(attributes.into());
        Ok(self)
    }

    /// Присоединяет приобретённые табличные части к собственному документу.
    ///
    /// # Errors
    /// Возвращает ошибку для неизвестного владельца или владельца другого вида.
    pub fn with_document_tabular_sections(
        mut self,
        owner: &str,
        sections: Vec<DocumentTabularSection>,
    ) -> Result<Self, CatalogError> {
        let index = self
            .objects
            .iter()
            .position(|object| object.id() == owner && object.kind() == MetadataKind::Document)
            .ok_or_else(|| CatalogError::UnknownDocumentOwner(owner.into()))?;
        Arc::make_mut(&mut self.objects)[index].document_tabular_sections = Some(sections.into());
        document_tabular_section::update_types(
            &self.identity,
            &self.objects,
            index,
            &mut self.document_section_types,
        );
        Ok(self)
    }

    /// Присоединяет полные приобретённые коллекции обработки до создания сеансов.
    /// `None` означает неизвестность, пустой список — известное отсутствие.
    /// Сохраняет идентичности типов и обновляет производные типы строк и частей.
    ///
    /// # Errors
    /// Возвращает ошибку для отсутствующего владельца или другого вида объекта.
    pub fn with_data_processor_metadata(
        mut self,
        owner: &str,
        attributes: Option<Vec<DocumentAttribute>>,
        sections: Option<Vec<DocumentTabularSection>>,
    ) -> Result<Self, CatalogError> {
        let index = self
            .objects
            .iter()
            .position(|object| object.id() == owner && object.kind() == MetadataKind::DataProcessor)
            .ok_or_else(|| CatalogError::UnknownDataProcessorOwner(owner.into()))?;
        let object = &mut Arc::make_mut(&mut self.objects)[index];
        object.data_processor_attributes = attributes.map(Into::into);
        object.data_processor_sections = sections.map(Into::into);
        document_tabular_section::update_types(
            &self.identity,
            &self.objects,
            index,
            &mut self.data_processor_section_types,
        );
        Ok(self)
    }

    /// Присоединяет полные приобретённые коллекции отчёта до создания сеансов.
    /// `None` означает неизвестность, пустой список — проверенное отсутствие.
    /// Идентичности объектов сохраняются; производные типы частей обновляются.
    ///
    /// # Errors
    /// Возвращает ошибку для отсутствующего владельца или другого вида объекта.
    pub fn with_report_metadata(
        mut self,
        owner: &str,
        attributes: Option<Vec<DocumentAttribute>>,
        sections: Option<Vec<DocumentTabularSection>>,
    ) -> Result<Self, CatalogError> {
        let index = self
            .objects
            .iter()
            .position(|object| object.id() == owner && object.kind() == MetadataKind::Report)
            .ok_or_else(|| CatalogError::UnknownReportOwner(owner.into()))?;
        let object = &mut Arc::make_mut(&mut self.objects)[index];
        object.report_attributes = attributes.map(Into::into);
        object.report_sections = sections.map(Into::into);
        document_tabular_section::update_types(
            &self.identity,
            &self.objects,
            index,
            &mut self.report_section_types,
        );
        Ok(self)
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
        let mut names = open_bsl::NameInterner::new();
        let mut member_names = std::collections::HashSet::new();
        for value in &values {
            if value.value_id.is_empty() || value.name.is_empty() {
                return Err(CatalogError::InvalidEnumerationValue);
            }
            let owner = self
                .object_by_id(&value.owner_id)
                .filter(|owner| owner.kind == MetadataKind::Enumeration)
                .ok_or_else(|| CatalogError::UnknownEnumerationOwner(value.owner_id.to_string()))?;
            if !member_names.insert((owner.id.as_ref(), names.intern(&value.name))) {
                return Err(CatalogError::DuplicateEnumerationValue {
                    owner: owner.name.to_string(),
                    name: value.name.to_string(),
                });
            }
        }
        self.enumeration_values = values.into();
        Ok(self)
    }

    /// Присоединяет идентификатор корневого объекта метаданных конфигурации.
    /// Идентичность каталога служит пространством имён типов; права адресуют
    /// самостоятельный идентификатор объекта из приобретённых метаданных.
    ///
    /// # Errors
    /// Возвращает ошибку для пустого идентификатора.
    pub fn with_configuration_object_id(
        mut self,
        id: impl Into<String>,
    ) -> Result<Self, CatalogError> {
        let id = id.into();
        if id.is_empty() {
            return Err(CatalogError::EmptyConfigurationIdentity);
        }
        self.configuration_object_id = Some(id.into());
        Ok(self)
    }

    /// Возвращает приобретённый идентификатор корневого объекта конфигурации.
    /// Идентичность пространства имён каталога не заменяет этот идентификатор.
    #[must_use]
    pub fn configuration_object_id(&self) -> Option<&str> {
        self.configuration_object_id.as_deref()
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

    /// Сохраняет приобретённый UUID основного интерфейса конфигурации.
    /// Нулевой UUID подтверждает отсутствие интерфейса. Без вызова этого
    /// метода настройка остаётся неизвестной. Чтение назначенного интерфейса
    /// пока завершается явной ошибкой неподдерживаемых метаданных.
    #[must_use]
    pub const fn with_default_interface_id(mut self, id: [u8; 16]) -> Self {
        self.default_interface_id = Some(id);
        self
    }

    #[must_use]
    /// Возвращает идентичность каталога, задающую пространство имён его типов.
    pub fn identity(&self) -> &str {
        &self.identity
    }

    #[must_use]
    /// Возвращает объекты в порядке источника.
    pub fn objects(&self) -> &[MetadataObject] {
        &self.objects
    }

    /// Присоединяет источник подробностей к уже проверенному индексу.
    /// Повторные обращения всех сеансов используют один неизменяемый результат.
    #[must_use]
    pub fn with_object_source(mut self, source: Arc<dyn MetadataObjectSource>) -> Self {
        self.object_details = (0..self.objects.len())
            .map(|_| std::sync::OnceLock::new())
            .collect();
        self.object_source = Some(source);
        self
    }

    /// Возвращает подробности объекта, загружая их при первом обращении.
    /// `None` означает отсутствие идентификатора в индексе, а не ошибку чтения.
    ///
    /// # Errors
    /// Возвращает ошибку источника либо несовпадения подробностей с индексом.
    pub fn materialized_object_by_id(&self, id: &str) -> RtResult<Option<&MetadataObject>> {
        let Some(&index) = self.object_index.get(id) else {
            return Ok(None);
        };
        let header = &self.objects[index];
        let Some(source) = &self.object_source else {
            if let Some(attached) = &self.file_source {
                self.file_validations[index]
                    .get_or_init(|| file_source::validate_object(attached, header))
                    .as_ref()
                    .map_err(|error| RtError::DynamicError(error.clone()))?;
            }
            return Ok(Some(header));
        };
        self.object_details[index]
            .get_or_init(|| {
                let mut object = source.load_object(id)?;
                if object.id() != header.id()
                    || object.kind() != header.kind()
                    || !folded_eq(object.name(), header.name())
                {
                    return Err(format!(
                        "подробности объекта «{id}» не соответствуют индексу конфигурации"
                    ));
                }
                if let Some(attached) = &self.file_source {
                    file_source::validate_object(attached, &object)?;
                    file_source::bind_modules(attached, &mut object);
                }
                Ok(object)
            })
            .as_ref()
            .map(Some)
            .map_err(|message| RtError::DynamicError(message.clone()))
    }

    #[must_use]
    /// Находит объект по устойчивому идентификатору.
    pub fn object_by_id(&self, id: &str) -> Option<&MetadataObject> {
        let &index = self.object_index.get(id)?;
        self.object_details
            .get(index)
            .and_then(std::sync::OnceLock::get)
            .and_then(|result| result.as_ref().ok())
            .or_else(|| self.objects.get(index))
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
    full_name: Arc<str>,
    role_ids: Arc<[Arc<str>]>,
    interface_language_id: Option<Arc<str>>,
    data_separation: Option<Arc<SerializedValueGraph>>,
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
            full_name: Arc::from(""),
            role_ids: role_ids
                .into_iter()
                .map(Arc::<str>::from)
                .collect::<Vec<_>>()
                .into(),
            interface_language_id: None,
            data_separation: None,
        }
    }

    /// Сохраняет полное имя пользователя на момент открытия сеанса.
    #[must_use]
    pub fn with_full_name(mut self, full_name: impl Into<String>) -> Self {
        self.full_name = Arc::from(full_name.into());
        self
    }

    /// Сохраняет выбранный для пользователя язык интерфейса.
    #[must_use]
    pub fn with_interface_language_id(mut self, id: impl Into<String>) -> Self {
        self.interface_language_id = Some(Arc::from(id.into()));
        self
    }

    /// Сохраняет приобретённый снимок структуры разделения данных пользователя.
    ///
    /// Один корень типа `Структура` проверяется при чтении BSL-свойства.
    /// Отсутствие снимка отличается от известной пустой структуры.
    #[must_use]
    pub fn with_data_separation(mut self, snapshot: SerializedValueGraph) -> Self {
        self.data_separation = Some(Arc::new(snapshot));
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

    /// Возвращает полное имя пользователя, отдельное от краткого имени.
    #[must_use]
    pub fn full_name(&self) -> &str {
        &self.full_name
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

/// Поиск собственного реквизита справочника или документа в данных текущего сеанса.
#[derive(Debug, Clone)]
pub struct FindByAttributeRequest {
    attribute: Rc<str>,
    value: Value,
    parent: Option<Value>,
    owner: Option<Value>,
}

impl FindByAttributeRequest {
    /// Приобретённое имя собственного реквизита.
    #[must_use]
    pub fn attribute(&self) -> &str {
        &self.attribute
    }
    /// Значение поиска без приведения к другому типу.
    #[must_use]
    pub const fn value(&self) -> &Value {
        &self.value
    }
    /// Явно указанный родитель; отсутствие означает поиск без этого отбора.
    #[must_use]
    pub fn parent(&self) -> Option<&Value> {
        self.parent.as_ref()
    }
    /// Явно указанный владелец; отсутствие означает поиск без этого отбора.
    #[must_use]
    pub fn owner(&self) -> Option<&Value> {
        self.owner.as_ref()
    }
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
    metadata_value: Value,
    attribute: Option<DocumentAttribute>,
    subject: AccessRightSubject,
    standard_attribute: Option<Rc<str>>,
}

/// Снимок политики паролей пользователей, прочитанный текущим сеансом.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserPasswordPolicySnapshot {
    minimum_password_length: u32,
}

/// Полный снимок настроек блокировки аутентификации пользователей базы.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthenticationLockSettingsSnapshot {
    lock_duration: BslNumber,
    max_unsuccessful_attempts_count: BslNumber,
    user_name_addition_codes: Arc<str>,
}

/// Числовая настройка политики паролей информационной базы.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserPasswordNumericSetting {
    /// Максимальный срок действия пароля в секундах.
    MaxEffectivePeriod,
    /// Минимальный срок действия пароля в секундах.
    MinEffectivePeriod,
    /// Число последних паролей, которые нельзя повторять.
    ReuseLimit,
    /// Срок предупреждения об истечении пароля в секундах.
    ExpirationNotificationPeriod,
}

/// Настройка политики паролей, изменяемая через адаптер информационной базы.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserPasswordSetting {
    /// Проверка сложности паролей.
    StrengthCheck,
    /// Минимальная длина пароля.
    MinimumLength,
    /// Числовая настройка срока или повторения паролей.
    Numeric(UserPasswordNumericSetting),
}

impl AuthenticationLockSettingsSnapshot {
    /// Создаёт снимок из двух числовых настроек и строки кодов дополнения.
    #[must_use]
    pub fn new(
        lock_duration: BslNumber,
        max_unsuccessful_attempts_count: BslNumber,
        user_name_addition_codes: impl Into<Arc<str>>,
    ) -> Self {
        Self {
            lock_duration,
            max_unsuccessful_attempts_count,
            user_name_addition_codes: user_name_addition_codes.into(),
        }
    }

    /// Возвращает длительность блокировки в секундах.
    #[must_use]
    pub const fn lock_duration(&self) -> &BslNumber {
        &self.lock_duration
    }

    /// Возвращает предельное число неуспешных попыток.
    #[must_use]
    pub const fn max_unsuccessful_attempts_count(&self) -> &BslNumber {
        &self.max_unsuccessful_attempts_count
    }

    /// Возвращает строку кодов дополнения имени пользователя.
    #[must_use]
    pub fn user_name_addition_codes(&self) -> &str {
        &self.user_name_addition_codes
    }
}

impl UserPasswordPolicySnapshot {
    /// Создаёт снимок с минимальной длиной пароля.
    #[must_use]
    pub const fn new(minimum_password_length: u32) -> Self {
        Self {
            minimum_password_length,
        }
    }

    /// Возвращает минимальную длину пароля.
    #[must_use]
    pub const fn minimum_password_length(&self) -> u32 {
        self.minimum_password_length
    }
}

impl AccessRightRequest {
    /// Возвращает имя проверяемого права.
    #[must_use]
    pub fn right(&self) -> &str {
        &self.right
    }

    /// Возвращает объект метаданных, владельца реквизита или корень конфигурации.
    #[must_use]
    pub const fn metadata(&self) -> &MetadataObject {
        &self.metadata
    }

    /// Возвращает реквизит документа; `metadata()` при этом содержит его владельца.
    #[must_use]
    pub const fn attribute(&self) -> Option<&DocumentAttribute> {
        self.attribute.as_ref()
    }

    /// Возвращает исходное BSL-значение проверенных метаданных.
    ///
    /// Значение принадлежит каталогу текущего сеанса и сохраняется без
    /// повторного создания объекта, в том числе для записи в журнал.
    #[must_use]
    pub const fn metadata_value(&self) -> &Value {
        &self.metadata_value
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
    /// установки. Написание ключей соответствует текущей структуре параметров.
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
    /// Выдаёт код справочника по измеренным настройкам серии конфигурации.
    /// Получает префикс после прикладного обработчика и пользователя сеанса.
    /// Адаптер использует уже принадлежащее этому сеансу соединение.
    ///
    /// # Errors
    /// Возвращает ошибку неизвестных настроек, прав, источника данных или
    /// неподдерживаемой выдачи кода. По умолчанию выдача не поддерживается.
    fn allocate_catalog_code(
        &self,
        _metadata: &MetadataObject,
        _principal: &Principal,
        _prefix: &str,
        _context: &mut CallContext<'_>,
    ) -> RtResult<Value> {
        Err(RtError::DynamicError(
            "выдача кода справочника не поддерживается адаптером данных".into(),
        ))
    }

    /// Выдаёт новый номер документа в области нумерации конфигурации.
    /// Получает дату документа и префикс после прикладного обработчика.
    /// Адаптер принадлежит сеансу, но резервирование номера может быть общим
    /// для нескольких сеансов информационной базы.
    ///
    /// # Errors
    /// Возвращает ошибку неизвестных настроек, прав, резервирования или
    /// неподдерживаемого адаптера. По умолчанию генерация не поддерживается.
    fn allocate_document_number(
        &self,
        _metadata: &MetadataObject,
        _principal: &Principal,
        _date: open_bsl::BslDate,
        _prefix: &str,
        _context: &mut CallContext<'_>,
    ) -> RtResult<Value> {
        Err(RtError::DynamicError(
            "выдача номера документа не поддерживается адаптером данных".into(),
        ))
    }

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

    /// Записывает константу от имени текущего пользователя.
    /// Адаптер отвечает за приведение типа, права, обработчики и сохранение
    /// в транзакции своего соединения. По умолчанию запись недоступна.
    ///
    /// # Errors
    /// Возвращает ошибку отсутствующей возможности записи, проверки прав,
    /// преобразования значения, обработчика или источника данных.
    fn set_constant_value(
        &self,
        _metadata: &MetadataObject,
        _value: &Value,
        _principal: &Principal,
        _context: &mut CallContext<'_>,
    ) -> RtResult<()> {
        Err(RtError::DynamicError(
            "запись константы не поддерживается адаптером данных".into(),
        ))
    }

    /// Ищет доступную ссылку по собственному реквизиту справочника или документа.
    /// `None` означает успешный поиск без совпадения, а не ошибку чтения.
    ///
    /// # Errors
    /// Возвращает ошибку неподдерживаемого поиска, метаданных, прав, RLS
    /// либо источника данных. По умолчанию поиск явно не поддерживается.
    fn find_by_attribute(
        &self,
        _metadata: &MetadataObject,
        _request: &FindByAttributeRequest,
        _principal: &Principal,
        _context: &mut CallContext<'_>,
    ) -> RtResult<Option<Value>> {
        Err(RtError::DynamicError(
            "поиск по реквизиту не поддерживается адаптером данных".into(),
        ))
    }

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

    /// Выполняет запрос, допускающий отсутствие результата после уничтожения
    /// временной таблицы. Прежние адаптеры автоматически возвращают результат.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку адаптера данных, проверки прав или компиляции запроса.
    fn execute_query_result(
        &self,
        request: &QueryRequest,
        principal: &Principal,
        context: &mut CallContext<'_>,
    ) -> RtResult<Option<QueryResultData>> {
        self.execute_query(request, principal, context).map(Some)
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

    /// Строит навигационную ссылку для прикладного значения текущего сеанса.
    ///
    /// `arguments` сохраняет позиции перегруженного платформенного вызова,
    /// включая `Неопределено` на месте пропущенного аргумента.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку неподдерживаемого вида ссылки, источника данных,
    /// прав, внешнего хранилища навигационных данных или самого значения.
    fn navigation_link(
        &self,
        _arguments: &[Value],
        _principal: &Principal,
        _context: &mut CallContext<'_>,
    ) -> RtResult<String> {
        Err(RtError::DynamicError(
            "навигационные ссылки не поддерживаются адаптером данных".into(),
        ))
    }

    /// Ищет политику паролей пользователей по имени в текущей информационной базе.
    ///
    /// `None` в `name` означает вызов платформенного метода без необязательного
    /// аргумента. `None` в результате означает, что политика не найдена.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку источника данных, административных прав или
    /// неподдерживаемой операции.
    fn user_password_policy(
        &self,
        _name: Option<&str>,
        _principal: &Principal,
        _context: &mut CallContext<'_>,
    ) -> RtResult<Option<UserPasswordPolicySnapshot>> {
        Err(RtError::DynamicError(
            "политики паролей пользователей не поддерживаются адаптером данных".into(),
        ))
    }

    /// Возвращает минимальную длину паролей пользователей информационной базы.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку источника данных, административных прав или
    /// неподдерживаемой операции.
    fn user_password_minimum_length(
        &self,
        _principal: &Principal,
        _context: &mut CallContext<'_>,
    ) -> RtResult<u32> {
        Err(RtError::DynamicError(
            "минимальная длина паролей пользователей не поддерживается адаптером данных".into(),
        ))
    }

    /// Проверяет, включена ли проверка сложности паролей пользователей базы.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку источника данных, административных прав или
    /// неподдерживаемой операции.
    fn user_password_strength_check(
        &self,
        _principal: &Principal,
        _context: &mut CallContext<'_>,
    ) -> RtResult<bool> {
        Err(RtError::DynamicError(
            "проверка сложности паролей пользователей не поддерживается адаптером данных".into(),
        ))
    }

    /// Читает числовую настройку политики паролей пользователей базы.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку источника данных, административных прав или
    /// неподдерживаемой операции.
    fn user_password_numeric_setting(
        &self,
        _setting: UserPasswordNumericSetting,
        _principal: &Principal,
        _context: &mut CallContext<'_>,
    ) -> RtResult<BslNumber> {
        Err(RtError::DynamicError(
            "числовые настройки паролей пользователей не поддерживаются адаптером данных".into(),
        ))
    }

    /// Изменяет настройку политики паролей пользователей базы.
    ///
    /// Значение передаётся без преобразования: его проверка и сохранение
    /// принадлежат конкретному адаптеру информационной базы.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку источника данных, административных прав, проверки
    /// значения или неподдерживаемой операции.
    fn set_user_password_setting(
        &self,
        _setting: UserPasswordSetting,
        _value: &Value,
        _principal: &Principal,
        _context: &mut CallContext<'_>,
    ) -> RtResult<()> {
        Err(RtError::DynamicError(
            "изменение настроек паролей пользователей не поддерживается адаптером данных".into(),
        ))
    }

    /// Читает полный снимок настроек блокировки аутентификации пользователей.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку источника данных, административных прав или
    /// неподдерживаемой операции.
    fn authentication_lock_settings(
        &self,
        _principal: &Principal,
        _context: &mut CallContext<'_>,
    ) -> RtResult<AuthenticationLockSettingsSnapshot> {
        Err(RtError::DynamicError(
            "настройки блокировки аутентификации не поддерживаются адаптером данных".into(),
        ))
    }

    /// Записывает полный снимок настроек блокировки аутентификации пользователей.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку источника данных, административных прав, проверки
    /// значений или неподдерживаемой операции.
    fn set_authentication_lock_settings(
        &self,
        _settings: &AuthenticationLockSettingsSnapshot,
        _principal: &Principal,
        _context: &mut CallContext<'_>,
    ) -> RtResult<()> {
        Err(RtError::DynamicError(
            "изменение настроек блокировки аутентификации не поддерживается адаптером данных"
                .into(),
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
    /// пользователя. Проверка корня конфигурации делегируется адаптеру и
    /// в привилегированном режиме: право и явный субъект требуют проверки.
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

    /// Проверяет право текущего пользователя с записью отказа в журнал.
    ///
    /// Адаптер отвечает за проверку права и событие `ОтказВДоступе` при
    /// отказе: без обеих операций он должен вернуть ошибку.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку при отказе в доступе, недоступном журнале или
    /// отсутствии у адаптера такой операции.
    fn verify_access_rights(
        &self,
        _request: &AccessRightRequest,
        _principal: &Principal,
        _context: &mut CallContext<'_>,
    ) -> RtResult<()> {
        Err(RtError::DynamicError(
            "проверка прав с записью отказа не поддерживается адаптером данных".into(),
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
    reference_uuids: RefCell<reference_uuid::ReferenceUuids>,
    system_language: Option<Arc<str>>,
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
            reference_uuids: RefCell::new(reference_uuid::ReferenceUuids::default()),
            system_language: None,
        }
    }

    /// Сохраняет язык интерфейса этого сеанса отдельно от общего каталога.
    #[must_use]
    pub fn with_system_language(mut self, language: impl Into<Arc<str>>) -> Self {
        self.system_language = Some(language.into());
        self
    }

    /// Возвращает явно переданный язык интерфейса сеанса.
    #[must_use]
    pub fn system_language(&self) -> Option<&str> {
        self.system_language.as_deref()
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
    /// Родитель вложенной подсистемы отсутствует или имеет другой вид.
    UnknownSubsystemParent(String),
    /// Имя связанного модуля менеджера пусто.
    EmptyManagerModule {
        /// Идентификатор объекта.
        object: String,
    },
    /// Имя связанного модуля объекта пусто.
    EmptyObjectModule {
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
    /// Владелец реквизитов отсутствует или не является документом.
    UnknownDocumentOwner(String),
    /// Владелец общих реквизитов отсутствует или не является справочником.
    UnknownCatalogOwner(String),
    /// Связи общего реквизита указывают на отсутствующие параметры или неверного владельца.
    InvalidCommonAttributeSeparation(String),
    /// Полный состав общего реквизита содержит неизвестные или неверные связи.
    InvalidCommonAttributeContent(String),
    /// Владелец настроек отсутствует или не является планом видов характеристик.
    UnknownCharacteristicOwner(String),
    /// Владелец настроек отсутствует или не является регистром накопления.
    UnknownAccumulationRegisterOwner(String),
    /// Владелец настроек отсутствует или не является регистром сведений.
    UnknownInformationRegisterOwner(String),
    /// Владелец настроек отсутствует или не является бизнес-процессом.
    UnknownBusinessProcessOwner(String),
    /// Владелец коллекций отсутствует или не является обработкой.
    UnknownDataProcessorOwner(String),
    /// Владелец реквизитов или табличных частей отчёта отсутствует или имеет другой вид.
    UnknownReportOwner(String),
    /// Владелец настроек отсутствует или не является функциональной опцией.
    UnknownFunctionalOptionOwner(String),
    /// Настроенная задача отсутствует или относится к другому виду метаданных.
    InvalidBusinessProcessTask(String),
    /// Некорректный владелец, тип или полный состав точек маршрута.
    InvalidRoutePoints(String),
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
            Self::UnknownSubsystemParent(parent) => {
                write!(
                    formatter,
                    "родитель подсистемы `{parent}` отсутствует в метаданных"
                )
            }
            Self::EmptyManagerModule { object } => {
                write!(formatter, "имя модуля менеджера объекта `{object}` пусто")
            }
            Self::EmptyObjectModule { object } => {
                write!(formatter, "имя модуля объекта `{object}` пусто")
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
            Self::UnknownDocumentOwner(owner) => write!(
                formatter,
                "владелец реквизитов `{owner}` отсутствует или не является документом"
            ),
            Self::InvalidCommonAttributeSeparation(owner) => write!(
                formatter,
                "некорректные связи параметров разделения `{owner}`"
            ),
            Self::InvalidCommonAttributeContent(owner) => write!(
                formatter,
                "некорректный полный состав общего реквизита `{owner}`"
            ),
            Self::UnknownCatalogOwner(owner) => write!(
                formatter,
                "владелец общих реквизитов `{owner}` отсутствует или не является справочником"
            ),
            Self::InvalidRoutePoints(owner) => {
                write!(formatter, "некорректные точки маршрута `{owner}`")
            }
            Self::InvalidBusinessProcessTask(task) => {
                write!(formatter, "некорректная задача бизнес-процесса `{task}`")
            }
            Self::UnknownReportOwner(owner) => write!(
                formatter,
                "владелец метаданных отчёта «{owner}» отсутствует или имеет другой вид"
            ),
            Self::UnknownDataProcessorOwner(owner) => write!(
                formatter,
                "владелец коллекций `{owner}` отсутствует или не является обработкой"
            ),
            Self::UnknownFunctionalOptionOwner(owner) => write!(
                formatter,
                "владелец настроек `{owner}` отсутствует или не является функциональной опцией"
            ),
            Self::UnknownBusinessProcessOwner(owner) => write!(
                formatter,
                "владелец настроек `{owner}` отсутствует или не является бизнес-процессом"
            ),
            Self::UnknownInformationRegisterOwner(owner) => write!(
                formatter,
                "владелец настроек `{owner}` отсутствует или не является регистром сведений"
            ),
            Self::UnknownAccumulationRegisterOwner(owner) => write!(
                formatter,
                "владелец настроек `{owner}` отсутствует или не является регистром накопления"
            ),
            Self::UnknownCharacteristicOwner(owner) => write!(
                formatter,
                "владелец настроек `{owner}` отсутствует или не является планом видов характеристик"
            ),
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
    subsystem_parent: Option<Arc<str>>,
}

impl MetadataCollection {
    fn positions(&self) -> &[usize] {
        if self.kind == MetadataKind::CommonAttribute {
            return &self.catalog.common_attribute_positions;
        }
        let parent = if self.kind == MetadataKind::Subsystem {
            self.subsystem_parent.clone()
        } else {
            None
        };
        self.catalog
            .collection_positions
            .get(&(std::mem::discriminant(&self.kind), parent))
            .map_or(&[], Vec::as_slice)
    }

    fn object(&self, position: usize) -> &MetadataObject {
        let object = &self.catalog.objects[position];
        if self.kind == MetadataKind::CommonAttribute {
            // Корневой состав задан отдельно; уже приобретённые подробности
            // общих реквизитов остаются видны, как при поиске по идентификатору.
            self.catalog.object_by_id(&object.id).unwrap_or(object)
        } else {
            object
        }
    }

    fn members(&self) -> impl Iterator<Item = &MetadataObject> {
        self.positions()
            .iter()
            .map(|&position| self.object(position))
    }

    fn find(&self, name: &str) -> Option<&MetadataObject> {
        self.members().find(|object| folded_eq(&object.name, name))
    }
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
struct MetadataPropertyEnumeration(EnumKind);

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
struct ConfigurationReferenceValue {
    catalog: Arc<MetadataCatalog>,
    object_id: Arc<str>,
    ty: Arc<ValueTypeDescriptor>,
    uuid: [u8; 16],
}

#[derive(Debug)]
struct EnumerationReferenceValue {
    catalog: Arc<MetadataCatalog>,
    object_id: Arc<str>,
    ty: Arc<ValueTypeDescriptor>,
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
struct UserPasswordPoliciesManager;

#[derive(Debug)]
struct IntegrationServicesManager;

#[derive(Debug)]
struct InfoBaseUserValue {
    principal: Principal,
    data_separation: RefCell<Option<Value>>,
}

#[derive(Debug)]
struct UserPasswordPolicyValue {
    snapshot: UserPasswordPolicySnapshot,
}

#[derive(Debug)]
struct AuthenticationLockManager;

#[derive(Debug)]
struct AuthenticationLockSettingsValue {
    lock_duration: RefCell<BslNumber>,
    max_unsuccessful_attempts_count: RefCell<BslNumber>,
    user_name_addition_codes: RefCell<Arc<str>>,
}

#[derive(Debug)]
struct RandomPasswordGenerator;

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
    // Та же живая структура доступна через свойство и метод установки параметра.
    parameters: Value,
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

#[derive(Debug)]
struct DataCompositionTemplateComposerValue;

#[derive(Debug)]
struct DataCompositionProcessorValue;

#[derive(Debug, Default)]
struct DataCompositionValueCollectionOutputProcessorValue {
    show_output_percent: Cell<bool>,
}

static METADATA_TYPE: TypeDescriptor =
    TypeDescriptor::new("bsl-orm", "ОбъектМетаданныхКонфигурация");
static METADATA_COLLECTION_TYPE: TypeDescriptor =
    TypeDescriptor::new("bsl-orm", "КоллекцияОбъектовМетаданных");
static METADATA_VALUE_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-orm",
    name: "ОбъектМетаданныхКонфигурации",
    type_display: "Metadata object",
    type_names: &["ОбъектМетаданных", "MetadataObject"],
};
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
static USER_PASSWORD_POLICIES_MANAGER_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-orm",
    name: "МенеджерПолитикПаролейПользователей",
    type_display: "User password policy manager",
    type_names: &["UserPasswordPolicyManager"],
};
static USER_PASSWORD_POLICY_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-orm",
    name: "ПолитикаПаролейПользователей",
    type_display: "User password policy",
    type_names: &["UserPasswordPolicy"],
};
static AUTHENTICATION_LOCK_MANAGER_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-orm",
    name: "МенеджерБлокировкиАутентификацииПользователейИнформационнойБазы",
    type_display: "Manager of infobase users authentication lock",
    type_names: &["InfoBaseUserAuthenticationLockManager"],
};
static AUTHENTICATION_LOCK_SETTINGS_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-orm",
    name: "НастройкиБлокировкиАутентификацииПользователейИнформационнойБазы",
    type_display: "Settings for infobase users authentication lock",
    type_names: &["InfoBaseUserAuthenticationLockSettings"],
};
static RANDOM_PASSWORD_GENERATOR_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-orm",
    name: "ГенераторСлучайныхПаролей",
    type_display: "Random password generator",
    type_names: &["RandomPasswordGenerator"],
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
static QUERY_SCHEMA_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-orm",
    name: "СхемаЗапроса",
    type_display: "Query schema",
    type_names: &["QuerySchema"],
};
static DATA_COMPOSITION_SETTINGS_COMPOSER_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-orm",
    name: "КомпоновщикНастроекКомпоновкиДанных",
    type_display: "Data composition settings composer",
    type_names: &["DataCompositionSettingsComposer"],
};
static DATA_COMPOSITION_AVAILABLE_SETTINGS_SOURCE_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-orm",
    name: "ИсточникДоступныхНастроекКомпоновкиДанных",
    type_display: "Data composition available settings source",
    type_names: &["DataCompositionAvailableSettingsSource"],
};
static DATA_COMPOSITION_DETAILS_PROCESS_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-orm",
    name: "ОбработкаРасшифровкиКомпоновкиДанных",
    type_display: "Data composition details processor",
    type_names: &["DataCompositionDetailsProcess"],
};
static CONFIGURATION_UPDATE_DESCRIPTION_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-orm",
    name: "ОписаниеОбновленияКонфигурации",
    type_display: "Configuration update description",
    type_names: &["ConfigurationUpdateDescription"],
};
static DATA_COMPOSITION_TEMPLATE_COMPOSER_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-orm",
    name: "КомпоновщикМакетаКомпоновкиДанных",
    type_display: "Composer of data composition template",
    type_names: &["DataCompositionTemplateComposer"],
};
static DATA_COMPOSITION_PROCESSOR_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-orm",
    name: "ПроцессорКомпоновкиДанных",
    type_display: "Data composition processor",
    type_names: &["DataCompositionProcessor"],
};
static DATA_COMPOSITION_VALUE_COLLECTION_OUTPUT_PROCESSOR_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-orm",
    name: "ПроцессорВыводаРезультатаКомпоновкиДанныхВКоллекциюЗначений",
    type_display: "Processor of output of data composition result in values collection",
    type_names: &["DataCompositionResultValueCollectionOutputProcessor"],
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
        if let Some(value) = file_source::root_enum(&self.catalog, name)? {
            return Ok(value);
        }
        if (folded_eq(name, "Имя")
            || folded_eq(name, "Name")
            || folded_eq(name, "Версия")
            || folded_eq(name, "Version"))
            && let Some(id) = self.catalog.configuration_object_id()
            && let Some(value) = file_source::property(&self.catalog, id, name, context)?
        {
            return Ok(value);
        }
        for (ru, en, kind) in [
            ("ОбщиеФормы", "CommonForms", "CommonForm"),
            ("ОбщиеМакеты", "CommonTemplates", "CommonTemplate"),
        ] {
            if (folded_eq(name, ru) || folded_eq(name, en))
                && let Some(value) = file_source::collection(&self.catalog, None, kind)?
            {
                return Ok(value);
            }
        }
        if let Some(kind) = collection_kind(name) {
            if kind == MetadataKind::CommonModule
                && let Some(value) = file_source::collection(&self.catalog, None, "CommonModule")?
            {
                return Ok(value);
            }
            if kind == MetadataKind::Subsystem {
                return metadata_root_subsystems(self, context);
            }
            if kind == MetadataKind::CommonAttribute {
                return metadata_root_common_attributes(self, context);
            }
            return Ok(Value::new_object(MetadataCollection {
                catalog: Arc::clone(&self.catalog),
                kind,
                subsystem_parent: None,
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

    fn method_table(&self) -> &'static [MethodDescriptor] {
        metadata_find_by_type::METHODS
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
        self.find(name)
            .map(|object| metadata_object_value(&self.catalog, object))
            .ok_or_else(|| RtError::UnknownProperty(name.into()))
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
        let positions = self.positions();
        let &position = positions.get(index).ok_or(RtError::IndexOutOfBounds {
            index: index as i64,
            len: positions.len(),
        })?;
        Ok(metadata_object_value(&self.catalog, self.object(position)))
    }

    fn collection_len(&self) -> RtResult<usize> {
        Ok(self.positions().len())
    }
}

impl ObjectProtocol for MetadataValue {
    fn method_table(&self) -> &'static [MethodDescriptor] {
        METADATA_VALUE_METHODS
    }

    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &METADATA_VALUE_TYPE
    }

    fn property_table(&self) -> &'static [PropertyDescriptor] {
        if self
            .catalog
            .object_by_id(&self.object_id)
            .is_some_and(|object| object.kind == MetadataKind::CommonAttribute)
        {
            METADATA_VALUE_PROPERTIES
        } else {
            // Первые четыре свойства относятся только к общему реквизиту.
            &METADATA_VALUE_PROPERTIES[4..]
        }
    }

    fn get_property(&self, name: &str, context: &mut CallContext<'_>) -> RtResult<Value> {
        if let Some(value) = file_source::property(&self.catalog, &self.object_id, name, context)? {
            return Ok(value);
        }

        if (folded_eq(name, "АвтоИспользование")
            || folded_eq(name, "AutoUse")
            || folded_eq(name, "РазделениеДанных")
            || folded_eq(name, "DataSeparation")
            || folded_eq(name, "Состав")
            || folded_eq(name, "Content")
            || folded_eq(name, "УсловноеРазделение")
            || folded_eq(name, "ConditionalSeparation"))
            && self
                .catalog
                .materialized_object_by_id(&self.object_id)?
                .is_some_and(|object| object.kind == MetadataKind::CommonAttribute)
        {
            return get_property_from_table(
                METADATA_VALUE_PROPERTIES,
                METADATA_VALUE_TYPE.name,
                self,
                name,
                context,
            );
        }
        if folded_eq(name, "Имя")
            || folded_eq(name, "Name")
            || folded_eq(name, "Подсистемы")
            || folded_eq(name, "Subsystems")
            || folded_eq(name, "ВключатьВКомандныйИнтерфейс")
            || folded_eq(name, "IncludeInCommandInterface")
            || folded_eq(name, "КодЯзыка")
            || folded_eq(name, "LanguageCode")
            || folded_eq(name, "Реквизиты")
            || folded_eq(name, "Attributes")
        {
            return get_property_from_table(
                METADATA_VALUE_PROPERTIES,
                METADATA_VALUE_TYPE.name,
                self,
                name,
                context,
            );
        }
        let object = self
            .catalog
            .materialized_object_by_id(&self.object_id)?
            .ok_or_else(|| {
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
        let object = self.catalog.materialized_object_by_id(id)?.ok_or_else(|| {
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

impl ObjectProtocol for MetadataPropertyEnumeration {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &COMPATIBILITY_MODE_PROPERTY_TYPE
    }

    fn get_property(&self, name: &str, _: &mut CallContext<'_>) -> RtResult<Value> {
        // Точка и индекс дают одинаковые члены, включая измеренный режим совместимости.
        lookup_member(self.0, name)
            .map(Value::Enum)
            .ok_or_else(|| RtError::UnknownProperty(name.into()))
    }

    fn get_index(&self, index: &Value) -> RtResult<Value> {
        let Value::Str(member) = index else {
            return Err(RtError::TypeError {
                expected: "Строка",
                op: "ПеречислимоеСвойствоОбъектаМетаданных[]",
            });
        };
        lookup_member(self.0, &member.to_string())
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
        if self
            .catalog
            .object_by_id(&self.object_id)
            .is_some_and(|object| object.kind() == MetadataKind::BusinessProcess)
        {
            BUSINESS_PROCESS_MANAGER_PROPERTIES
        } else {
            MANAGER_PROPERTIES
        }
    }

    fn get_property(&self, name: &str, context: &mut CallContext<'_>) -> RtResult<Value> {
        let object = self
            .catalog
            .materialized_object_by_id(&self.object_id)?
            .ok_or_else(|| {
                RtError::DynamicError("объект менеджера отсутствует в каталоге".into())
            })?;
        if object.kind() == MetadataKind::Enumeration
            && let Some(value) = self.catalog.enumeration_value(&self.object_id, name)
        {
            return enumeration_reference_value(
                Arc::clone(&self.catalog),
                &self.object_id,
                &value.value_id,
            );
        }
        if matches!(
            object.kind(),
            MetadataKind::Catalog
                | MetadataKind::ChartOfCharacteristicTypes
                | MetadataKind::ChartOfAccounts
        ) && let Some(name) = object
            .catalog_predefined_names()
            .iter()
            .find(|known| folded_eq(known, name))
        {
            // GUID предопределённых данных не является UUID записи. Обычный
            // адаптер запроса разрешает ссылку в базе текущего сеанса.
            let prefix = if object.kind() == MetadataKind::ChartOfCharacteristicTypes {
                "ПланВидовХарактеристик"
            } else if object.kind() == MetadataKind::ChartOfAccounts {
                "ПланСчетов"
            } else {
                "Справочник"
            };
            let request = QueryRequest {
                text: format!(
                    "ВЫБРАТЬ ЗНАЧЕНИЕ({prefix}.{}.{name}) КАК Ссылка",
                    object.name()
                )
                .into(),
                parameters: Rc::from([]),
                temp_tables_manager: None,
            };
            let (principal, data_access) = current_data_access(context)?;
            let result = data_access.execute_query(&request, &principal, context)?;
            let value = result
                .rows()
                .first()
                .and_then(|row| row.first())
                .ok_or_else(|| {
                    RtError::DynamicError("предопределённый элемент не вернул ссылку".into())
                })?;
            if matches!(value, Value::Null) {
                return Err(RtError::DynamicError(format!(
                    "{prefix}.{}.{name}. Предопределенный элемент не найден в данных информационной базы",
                    object.name()
                )));
            }
            if reference_matches_owner(value, &self.catalog, object.id(), object.kind()) {
                return Ok(value.clone());
            }
            return Err(RtError::DynamicError(
                "адаптер вернул несовместимый тип предопределённого элемента".into(),
            ));
        }
        get_property_from_table(
            self.property_table(),
            MANAGER_TYPE.name,
            self,
            name,
            context,
        )
    }

    fn set_property(
        &self,
        name: &str,
        value: Value,
        context: &mut CallContext<'_>,
    ) -> RtResult<()> {
        if self
            .catalog
            .materialized_object_by_id(&self.object_id)?
            .is_some_and(|object| {
                matches!(
                    object.kind(),
                    MetadataKind::Catalog
                        | MetadataKind::ChartOfCharacteristicTypes
                        | MetadataKind::ChartOfAccounts
                ) && object
                    .catalog_predefined_names()
                    .iter()
                    .any(|known| folded_eq(known, name))
            })
        {
            return Err(RtError::PropertyReadOnly {
                property: name.into(),
                receiver: MANAGER_TYPE.name,
            });
        }
        set_property_from_table(
            self.property_table(),
            MANAGER_TYPE.name,
            self,
            name,
            value,
            context,
        )
    }

    fn call_method(
        &self,
        name: &str,
        arguments: &[Value],
        context: &mut CallContext<'_>,
    ) -> RtResult<Value> {
        let object = self
            .catalog
            .materialized_object_by_id(&self.object_id)?
            .ok_or_else(|| {
                RtError::DynamicError("объект менеджера отсутствует в каталоге".into())
            })?;
        let methods = manager_methods(object.kind());
        let existing_lookup_export = if (folded_eq(name, "НайтиПоРеквизиту")
            || folded_eq(name, "FindByAttribute")
            || (object.kind() == MetadataKind::Catalog
                && [
                    "НайтиПоКоду",
                    "FindByCode",
                    "НайтиПоНаименованию",
                    "FindByDescription",
                ]
                .iter()
                .any(|known| folded_eq(name, known)))
            || (matches!(
                object.kind(),
                MetadataKind::Catalog
                    | MetadataKind::InformationRegister
                    | MetadataKind::AccumulationRegister
                    | MetadataKind::Report
                    | MetadataKind::ChartOfAccounts
            ) && (folded_eq(name, "ПолучитьМакет") || folded_eq(name, "GetTemplate")))
            || (object.kind() == MetadataKind::Catalog
                && (folded_eq(name, "СоздатьГруппу") || folded_eq(name, "CreateFolder")))
            || (object.kind() == MetadataKind::Document
                && (folded_eq(name, "СоздатьДокумент") || folded_eq(name, "CreateDocument")))
            || (matches!(
                object.kind(),
                MetadataKind::DataProcessor | MetadataKind::Report
            ) && (folded_eq(name, "Создать") || folded_eq(name, "Create")))
            || (object.kind() == MetadataKind::SettingsStorage
                && (folded_eq(name, "Загрузить") || folded_eq(name, "Load")))
            || (matches!(
                object.kind(),
                MetadataKind::Task
                    | MetadataKind::BusinessProcess
                    | MetadataKind::ChartOfCharacteristicTypes
            ) && ["ПустаяСсылка", "EmptyRef", "ПолучитьСсылку", "GetRef"]
                .iter()
                .any(|known| folded_eq(name, known)))
            || (object.kind() == MetadataKind::BusinessProcess
                && (folded_eq(name, "СоздатьБизнесПроцесс")
                    || folded_eq(name, "CreateBusinessProcess")))
            || (object.kind() == MetadataKind::InformationRegister
                && [
                    "Получить",
                    "Get",
                    "СрезПервых",
                    "SliceFirst",
                    "СрезПоследних",
                    "SliceLast",
                    "ПолучитьПоследнее",
                    "GetLast",
                    "Выбрать",
                    "Select",
                ]
                .iter()
                .any(|known| folded_eq(name, known)))
            || (object.kind() == MetadataKind::Constant
                && [
                    "Установить",
                    "Set",
                    "СоздатьМенеджерЗначения",
                    "CreateValueManager",
                ]
                .iter()
                .any(|known| folded_eq(name, known)))
            || (object.kind() == MetadataKind::AccumulationRegister
                && [
                    "СоздатьНаборЗаписей",
                    "CreateRecordSet",
                    "Остатки",
                    "Balance",
                ]
                .iter()
                .any(|known| folded_eq(name, known))))
            && let Some(module) = object.manager_module()
        {
            context.configuration_has_export(module, name)?
        } else {
            false
        };
        if method_exists(methods, name) && !existing_lookup_export {
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

impl ObjectProtocol for ConfigurationReferenceValue {
    fn reference_order_key(&self) -> Option<open_bsl::ReferenceOrderKey> {
        let metadata = self.catalog.object_by_id(&self.object_id)?;
        // Межвидовой порядок измерен пока только для ссылок документов.
        // Для других классов ключ не публикуется по аналогии.
        (metadata.kind() == MetadataKind::Document).then_some(open_bsl::ReferenceOrderKey {
            storage_number: metadata.reference_storage_number(),
            uuid: self.uuid,
        })
    }
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &CONFIGURATION_REFERENCE_TYPE
    }
    fn immutable_snapshot(&self) -> Option<&[u8]> {
        Some(&self.uuid)
    }

    fn value_type(&self) -> TypeRef {
        TypeRef::Value(Arc::clone(&self.ty))
    }
    fn method_table(&self) -> &'static [MethodDescriptor] {
        CONFIGURATION_REFERENCE_METHODS
    }
    fn is_filled(&self) -> RtResult<bool> {
        Ok(self.uuid != [0; 16])
    }
    fn value_eq(&self, other: &open_bsl::ObjectRef) -> Option<bool> {
        Some(other.downcast_ref::<Self>().is_some_and(|other| {
            TypeRef::Value(Arc::clone(&self.ty)) == TypeRef::Value(Arc::clone(&other.ty))
                && self.uuid == other.uuid
        }))
    }
    fn value_hash_key(&self) -> Option<u64> {
        use std::hash::{Hash, Hasher};
        let mut hash = std::collections::hash_map::DefaultHasher::new();
        self.ty.package().hash(&mut hash);
        self.ty.identity().hash(&mut hash);
        self.uuid.hash(&mut hash);
        Some(hash.finish())
    }
    fn display(&self) -> String {
        format!(
            "{}:{}",
            self.ty.identity(),
            Value::Object(Rc::new(BslObject::Uuid(self.uuid)))
        )
    }
    fn presentation(&self) -> RtResult<String> {
        if self.uuid == [0; 16] {
            return Ok(String::new());
        }
        Err(RtError::DynamicError(
            "получение представления заполненной ссылки не подключено к адаптеру данных".into(),
        ))
    }
}

/// Создаёт ссылку справочника по метаданным и UUID в каноническом порядке байтов.
/// Не читает и не записывает строку информационной базы.
///
/// # Errors
/// Возвращает ошибку, если объект или его тип ссылки отсутствует в каталоге.
pub fn catalog_reference_value(
    catalog: Arc<MetadataCatalog>,
    object_id: &str,
    uuid: [u8; 16],
) -> RtResult<Value> {
    if !catalog
        .materialized_object_by_id(object_id)?
        .is_some_and(|object| object.kind() == MetadataKind::Catalog)
    {
        return Err(RtError::DynamicError(
            "тип ссылки отсутствует в каталоге".into(),
        ));
    }
    configuration_reference_value(catalog, object_id, uuid)
}

// Общая память ссылки не содержит соединения или данных строки. Вид и
// собственный тип задаёт приобретённое описание, а не значение UUID.
fn reference_matches_owner(
    value: &Value,
    catalog: &MetadataCatalog,
    owner: &str,
    kind: MetadataKind,
) -> bool {
    value
        .object_ref()
        .and_then(|value| value.downcast_ref::<ConfigurationReferenceValue>())
        .is_some_and(|reference| {
            reference.object_id.as_ref() == owner
                && reference.catalog.identity() == catalog.identity()
                && reference
                    .catalog
                    .object_by_id(&reference.object_id)
                    .is_some_and(|metadata| metadata.kind() == kind)
        })
}

/// Создаёт ссылку объявленного объекта конфигурации по каноническому UUID.
/// Соединение и строка информационной базы не сохраняются в значении.
///
/// # Errors
/// Возвращает ошибку, если в каталоге отсутствует объявленный тип ссылки владельца.
pub fn configuration_reference_value(
    catalog: Arc<MetadataCatalog>,
    object_id: &str,
    uuid: [u8; 16],
) -> RtResult<Value> {
    if catalog
        .materialized_object_by_id(object_id)?
        .is_some_and(|object| object.kind() == MetadataKind::Enumeration)
    {
        // Константа и менеджер должны возвращать один и тот же тип значения.
        let identity = if uuid == [0; 16] {
            String::new()
        } else {
            open_bsl::uuid::format(&uuid)
        };
        return enumeration_reference_value(catalog, object_id, &identity);
    }
    let ty = catalog
        .reference_types
        .iter()
        .find(|(id, _)| id.as_ref() == object_id)
        .map(|(_, ty)| Arc::clone(ty))
        .ok_or_else(|| RtError::DynamicError("тип ссылки отсутствует в каталоге".into()))?;
    Ok(Value::new_object(ConfigurationReferenceValue {
        object_id: object_id.into(),
        catalog,
        ty,
        uuid,
    }))
}

/// Возвращает метаданные справочника и канонический UUID ссылки.
/// Для значений других видов возвращает `None` без преобразования в строку.
pub fn catalog_reference_parts(value: &Value) -> Option<(&MetadataObject, [u8; 16])> {
    let (object, uuid) = configuration_reference_parts(value)?;
    (object.kind() == MetadataKind::Catalog).then_some((object, uuid))
}

/// Возвращает метаданные и канонический UUID типизированной ссылки конфигурации.
/// Для значений других типов возвращает `None`, не обращаясь к базе данных.
pub fn configuration_reference_parts(value: &Value) -> Option<(&MetadataObject, [u8; 16])> {
    let reference = value
        .object_ref()?
        .downcast_ref::<ConfigurationReferenceValue>()?;
    let object = reference.catalog.object_by_id(&reference.object_id)?;
    Some((object, reference.uuid))
}

/// Идентификатор метаданных справочника для объявленного типа ссылки.
/// Для типов других компонентов и других видов значений возвращает `None`.
pub fn catalog_reference_type_id(ty: &TypeRef) -> Option<&str> {
    let TypeRef::Value(description) = ty else {
        return None;
    };
    if description.package() != "bsl-orm" {
        return None;
    }
    description
        .identity()
        .rsplit_once("/catalog-ref/")
        .map(|(_, id)| id)
}

fn catalog_reference(manager: &ManagerValue, uuid: [u8; 16]) -> RtResult<Value> {
    catalog_reference_value(Arc::clone(&manager.catalog), &manager.object_id, uuid)
}

fn catalog_empty_ref(
    receiver: &dyn ObjectProtocol,
    _: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let manager = receiver
        .downcast_ref::<ManagerValue>()
        .ok_or(RtError::NotAnObject)?;
    catalog_reference(manager, [0; 16])
}

fn document_empty_ref(
    receiver: &dyn ObjectProtocol,
    _: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let manager = receiver
        .downcast_ref::<ManagerValue>()
        .ok_or(RtError::NotAnObject)?;
    configuration_reference_value(Arc::clone(&manager.catalog), &manager.object_id, [0; 16])
}

fn configuration_get_ref(
    receiver: &dyn ObjectProtocol,
    args: &[Value],
    context: &mut CallContext<'_>,
) -> RtResult<Value> {
    let manager = receiver
        .downcast_ref::<ManagerValue>()
        .ok_or(RtError::NotAnObject)?;
    let argument = args.first().unwrap_or(&Value::Undefined);
    let generated;
    let value = if matches!(argument, Value::Undefined) {
        let uuid = session(context)?
            .reference_uuids
            .borrow_mut()
            .next(context.unix_millis()?, context.random()?)?;
        generated = Value::Object(Rc::new(BslObject::Uuid(uuid)));
        &generated
    } else {
        argument
    };
    let Value::Object(object) = value else {
        return Err(RtError::TypeError {
            expected: "УникальныйИдентификатор",
            op: "ПолучитьСсылку",
        });
    };
    let BslObject::Uuid(uuid) = object.as_ref() else {
        return Err(RtError::TypeError {
            expected: "УникальныйИдентификатор",
            op: "ПолучитьСсылку",
        });
    };
    configuration_reference_value(Arc::clone(&manager.catalog), &manager.object_id, *uuid)
}

fn configuration_reference_uuid(
    receiver: &dyn ObjectProtocol,
    _: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let reference = receiver
        .downcast_ref::<ConfigurationReferenceValue>()
        .ok_or(RtError::NotAnObject)?;
    Ok(Value::Object(Rc::new(BslObject::Uuid(reference.uuid))))
}

fn configuration_reference_metadata(
    receiver: &dyn ObjectProtocol,
    _: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let reference = receiver
        .downcast_ref::<ConfigurationReferenceValue>()
        .ok_or(RtError::NotAnObject)?;
    let object = reference
        .catalog
        .materialized_object_by_id(&reference.object_id)?
        .ok_or_else(|| RtError::DynamicError("метаданные ссылки отсутствуют в каталоге".into()))?;
    Ok(metadata_object_value(&reference.catalog, object))
}

fn configuration_reference_is_empty(
    receiver: &dyn ObjectProtocol,
    _: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let reference = receiver
        .downcast_ref::<ConfigurationReferenceValue>()
        .ok_or(RtError::NotAnObject)?;
    Ok(Value::Boolean(reference.uuid == [0; 16]))
}

static CONFIGURATION_REFERENCE_TYPE: TypeDescriptor =
    TypeDescriptor::new("bsl-orm", "СсылкаКонфигурации");
static CONFIGURATION_REFERENCE_METHODS: &[MethodDescriptor] = &[
    MethodDescriptor::new(
        &["Пустая", "IsEmpty"],
        Arity::exact(0),
        configuration_reference_is_empty,
    ),
    MethodDescriptor::new(
        &["УникальныйИдентификатор", "UUID"],
        Arity::exact(0),
        configuration_reference_uuid,
    ),
    MethodDescriptor::new(
        &["Метаданные", "Metadata"],
        Arity::exact(0),
        configuration_reference_metadata,
    ),
];
static CATALOG_MANAGER_METHODS: &[MethodDescriptor] = &[
    MethodDescriptor::new(
        &["ПустаяСсылка", "EmptyRef"],
        Arity::exact(0),
        catalog_empty_ref,
    ),
    MethodDescriptor::new(
        &["ПолучитьСсылку", "GetRef"],
        Arity::range(0, 1),
        configuration_get_ref,
    ),
    MethodDescriptor::new(
        &["СоздатьЭлемент", "CreateItem"],
        Arity::exact(0),
        catalog_object::create,
    ),
    MethodDescriptor::new(
        &["СоздатьГруппу", "CreateFolder"],
        Arity::exact(0),
        catalog_object::create_folder,
    ),
    MethodDescriptor::new(
        &["Выбрать", "Select"],
        Arity::range(0, 4),
        catalog_selection::select,
    ),
    MethodDescriptor::new(
        &["НайтиПоРеквизиту", "FindByAttribute"],
        Arity::range(2, 4),
        manager_find_by_attribute,
    ),
    MethodDescriptor::new(
        &["ПолучитьМакет", "GetTemplate"],
        Arity::exact(1),
        catalog_get_template,
    ),
    MethodDescriptor::new(
        &["НайтиПоКоду", "FindByCode"],
        Arity::range(1, 4),
        catalog_standard_search::code,
    ),
    MethodDescriptor::new(
        &["НайтиПоНаименованию", "FindByDescription"],
        Arity::range(1, 4),
        catalog_standard_search::description,
    ),
];

// Получение настоящего макета требует приобретённого ресурса.
// Конечная сигнатура связывает исходный модуль, но не подменяет результат.
fn catalog_get_template(
    receiver: &dyn ObjectProtocol,
    arguments: &[Value],
    context: &mut CallContext<'_>,
) -> RtResult<Value> {
    if let Some(value) = file_source::receiver_template(receiver, arguments, context)? {
        return Ok(value);
    }
    Err(RtError::DynamicError(
        "Справочник.ПолучитьМакет: получение макета пока не поддерживается".into(),
    ))
}

fn enumeration_empty_ref(
    receiver: &dyn ObjectProtocol,
    _: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let manager = receiver
        .downcast_ref::<ManagerValue>()
        .ok_or(RtError::NotAnObject)?;
    enumeration_reference_value(Arc::clone(&manager.catalog), &manager.object_id, "")
}

fn enumeration_count(
    receiver: &dyn ObjectProtocol,
    _: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let manager = receiver
        .downcast_ref::<ManagerValue>()
        .ok_or(RtError::NotAnObject)?;
    let count = manager
        .catalog
        .enumeration_values
        .iter()
        .filter(|value| value.owner_id == manager.object_id)
        .count();
    Ok(Value::number_from_i64(count as i64))
}

fn collection_index_number(value: &Value) -> RtResult<BslNumber> {
    Ok(match value {
        Value::Number(number) => number.clone(),
        Value::Str(text) => {
            open_bsl::parse_number(&text.to_string(), &open_bsl::NumberFormat::default())?
        }
        Value::Boolean(value) => BslNumber::from_i64(i64::from(*value)),
        _ => return Err(RtError::BadIndex),
    })
}

fn enumeration_get(
    receiver: &dyn ObjectProtocol,
    arguments: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let manager = receiver
        .downcast_ref::<ManagerValue>()
        .ok_or(RtError::NotAnObject)?;
    let number = collection_index_number(&arguments[0])?;
    let index = number
        .trunc_to_scale(0)
        .to_i64_exact()
        .and_then(|value| usize::try_from(value).ok())
        .ok_or(RtError::BadIndex)?;
    let value = manager
        .catalog
        .enumeration_values
        .iter()
        .filter(|value| value.owner_id == manager.object_id)
        .nth(index)
        .ok_or(RtError::BadIndex)?;
    enumeration_reference_value(
        Arc::clone(&manager.catalog),
        &manager.object_id,
        &value.value_id,
    )
}

fn enumeration_index(
    receiver: &dyn ObjectProtocol,
    arguments: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let manager = receiver
        .downcast_ref::<ManagerValue>()
        .ok_or(RtError::NotAnObject)?;
    let value = arguments[0]
        .object_ref()
        .and_then(|object| object.downcast_ref::<EnumerationReferenceValue>())
        .ok_or(RtError::BadIndex)?;
    let ty = manager
        .catalog
        .reference_types
        .iter()
        .find(|(id, _)| id == &manager.object_id)
        .map(|(_, ty)| ty)
        .ok_or(RtError::BadIndex)?;
    if TypeRef::Value(Arc::clone(ty)) != value.value_type() {
        return Err(RtError::BadIndex);
    }
    let index = manager
        .catalog
        .enumeration_values
        .iter()
        .filter(|member| member.owner_id == manager.object_id)
        .position(|member| member.value_id == value.value_id)
        .ok_or(RtError::BadIndex)?;
    Ok(Value::number_from_i64(index as i64))
}

static ENUMERATION_MANAGER_METHODS: &[MethodDescriptor] = &[
    MethodDescriptor::new(
        &["ПустаяСсылка", "EmptyRef"],
        Arity::exact(0),
        enumeration_empty_ref,
    ),
    MethodDescriptor::new(&["Количество", "Count"], Arity::exact(0), enumeration_count),
    MethodDescriptor::new(&["Получить", "Get"], Arity::exact(1), enumeration_get),
    MethodDescriptor::new(&["Индекс", "IndexOf"], Arity::exact(1), enumeration_index),
];

static MANAGER_PUBLIC_METHODS: &[MethodDescriptor] = &[
    exchange_plan::METHODS[0],
    exchange_plan::METHODS[2],
    CHART_OF_ACCOUNTS_MANAGER_METHODS[0],
    SETTINGS_STORAGE_MANAGER_METHODS[0],
    business_process_object::MANAGER_METHODS[0],
    accumulation_balance::METHOD,
    CONSTANT_MANAGER_METHODS[0],
    CONSTANT_MANAGER_METHODS[1],
    CONSTANT_MANAGER_METHODS[2],
    ENUMERATION_MANAGER_METHODS[1],
    ENUMERATION_MANAGER_METHODS[2],
    ENUMERATION_MANAGER_METHODS[3],
    CATALOG_MANAGER_METHODS[0],
    CATALOG_MANAGER_METHODS[1],
    CATALOG_MANAGER_METHODS[2],
    CATALOG_MANAGER_METHODS[3],
    CATALOG_MANAGER_METHODS[4],
    CATALOG_MANAGER_METHODS[7],
    CATALOG_MANAGER_METHODS[8],
    INFORMATION_REGISTER_MANAGER_METHODS[0],
    INFORMATION_REGISTER_MANAGER_METHODS[1],
    INFORMATION_REGISTER_MANAGER_METHODS[2],
    INFORMATION_REGISTER_MANAGER_METHODS[3],
    INFORMATION_REGISTER_MANAGER_METHODS[4],
    INFORMATION_REGISTER_MANAGER_METHODS[5],
    INFORMATION_REGISTER_MANAGER_METHODS[6],
    INFORMATION_REGISTER_MANAGER_METHODS[7],
    INFORMATION_REGISTER_MANAGER_METHODS[8],
    DATA_PROCESSOR_MANAGER_METHODS[0],
    DATA_PROCESSOR_MANAGER_METHODS[1],
    DOCUMENT_MANAGER_METHODS[0],
    DOCUMENT_MANAGER_METHODS[1],
    REPORT_MANAGER_METHODS[0],
    REPORT_MANAGER_METHODS[1],
];

// Сигнатура нужна при связывании исходного модуля менеджера обработки.
// Без приобретённых ресурсов выдавать пустой макет было бы неверно.
static DATA_PROCESSOR_MANAGER_METHODS: &[MethodDescriptor] = &[
    MethodDescriptor::new(
        &["ПолучитьМакет", "GetTemplate"],
        Arity::exact(1),
        data_processor_get_template,
    ),
    MethodDescriptor::new(
        &["Создать", "Create"],
        Arity::exact(0),
        data_processor_object::create,
    ),
];

fn data_processor_get_template(
    receiver: &dyn ObjectProtocol,
    arguments: &[Value],
    context: &mut CallContext<'_>,
) -> RtResult<Value> {
    if let Some(value) = file_source::receiver_template(receiver, arguments, context)? {
        return Ok(value);
    }
    Err(RtError::DynamicError(
        "Обработка.ПолучитьМакет: получение макета пока не поддерживается".into(),
    ))
}

// Сигнатура отчёта проверена отдельно по синтакс-помощнику.
// Декларация не подменяет отсутствующие ресурсы успешным пустым макетом.
static REPORT_MANAGER_METHODS: &[MethodDescriptor] = &[
    MethodDescriptor::new(
        &["ПолучитьМакет", "GetTemplate"],
        Arity::exact(1),
        report_get_template,
    ),
    MethodDescriptor::new(
        &["Создать", "Create"],
        Arity::exact(0),
        data_processor_object::create,
    ),
];

fn report_get_template(
    receiver: &dyn ObjectProtocol,
    arguments: &[Value],
    context: &mut CallContext<'_>,
) -> RtResult<Value> {
    if let Some(value) = file_source::receiver_template(receiver, arguments, context)? {
        return Ok(value);
    }
    Err(RtError::DynamicError(
        "Отчет.ПолучитьМакет: получение макета пока не поддерживается".into(),
    ))
}

// Задачи и бизнес-процессы используют общие фабрики конфигурационных ссылок.
// Сигнатуры проверены по синтакс-помощнику и нативным контролям.
// Нативный прямой вызов без аргументов возвращает `Неопределено`.
// Именованный макет требует настоящего приобретённого ресурса.
fn chart_get_template(
    receiver: &dyn ObjectProtocol,
    arguments: &[Value],
    context: &mut CallContext<'_>,
) -> RtResult<Value> {
    if let Some(value) = file_source::receiver_template(receiver, arguments, context)? {
        return Ok(value);
    }
    if arguments.is_empty() {
        return Ok(Value::Undefined);
    }
    Err(RtError::DynamicError(
        "ПланСчетов.ПолучитьМакет: получение макета пока не поддерживается".into(),
    ))
}

static CHART_OF_ACCOUNTS_MANAGER_METHODS: &[MethodDescriptor] = &[
    MethodDescriptor::new(
        &["ПолучитьМакет", "GetTemplate"],
        Arity::range(0, 1),
        chart_get_template,
    ),
    MethodDescriptor::new(
        &["ПустаяСсылка", "EmptyRef"],
        Arity::exact(0),
        document_empty_ref,
    ),
];

static CONFIGURATION_REFERENCE_MANAGER_METHODS: &[MethodDescriptor] = &[
    MethodDescriptor::new(
        &["ПустаяСсылка", "EmptyRef"],
        Arity::exact(0),
        document_empty_ref,
    ),
    MethodDescriptor::new(
        &["ПолучитьСсылку", "GetRef"],
        Arity::range(0, 1),
        configuration_get_ref,
    ),
];

// Документ имеет собственный контекст менеджера; сигнатура проверена
// отдельно по синтакс-помощнику, а не перенесена по аналогии с обработкой.
static DOCUMENT_MANAGER_METHODS: &[MethodDescriptor] = &[
    MethodDescriptor::new(
        &["НайтиПоРеквизиту", "FindByAttribute"],
        Arity::exact(2),
        manager_find_by_attribute,
    ),
    MethodDescriptor::new(
        &["СоздатьДокумент", "CreateDocument"],
        Arity::exact(0),
        document_object::create,
    ),
    MethodDescriptor::new(
        &["ПолучитьМакет", "GetTemplate"],
        Arity::exact(1),
        document_get_template,
    ),
    MethodDescriptor::new(
        &["ПустаяСсылка", "EmptyRef"],
        Arity::exact(0),
        document_empty_ref,
    ),
    MethodDescriptor::new(
        &["ПолучитьСсылку", "GetRef"],
        Arity::range(0, 1),
        configuration_get_ref,
    ),
];

fn document_get_template(
    receiver: &dyn ObjectProtocol,
    arguments: &[Value],
    context: &mut CallContext<'_>,
) -> RtResult<Value> {
    if let Some(value) = file_source::receiver_template(receiver, arguments, context)? {
        return Ok(value);
    }
    Err(RtError::DynamicError(
        "Документ.ПолучитьМакет: получение макета пока не поддерживается".into(),
    ))
}

// Сигнатуры платформы нужны и в неисполняемых процедурах исходного модуля.
// До реализации записей нельзя подменять их создание пустым объектом.
static INFORMATION_REGISTER_MANAGER_METHODS: &[MethodDescriptor] = &[
    MethodDescriptor::new(
        &["СоздатьМенеджерЗаписи", "CreateRecordManager"],
        Arity::exact(0),
        information_register_create_record_manager,
    ),
    MethodDescriptor::new(
        &["СоздатьНаборЗаписей", "CreateRecordSet"],
        Arity::exact(0),
        information_register_create_record_set,
    ),
    // Метод подтверждён синтакс-помощником именно для менеджера регистра.
    // Сигнатура связывает исходник, но не подменяет отсутствующий ресурс.
    MethodDescriptor::new(
        &["ПолучитьМакет", "GetTemplate"],
        Arity::exact(1),
        information_register_get_template,
    ),
    MethodDescriptor::new(
        &["СоздатьКлючЗаписи", "CreateRecordKey"],
        Arity::exact(1),
        information_register_create_record_key,
    ),
    information_register_get::METHOD,
    // Два необязательных аргумента подтверждены синтакс-помощником.
    // Декларация не подменяет неизвестную периодичность и данные регистра.
    MethodDescriptor::new(
        &["СрезПервых", "SliceFirst"],
        Arity::range(0, 2),
        information_register_slice_first,
    ),
    MethodDescriptor::new(
        &["СрезПоследних", "SliceLast"],
        Arity::range(0, 2),
        information_register_slice_last,
    ),
    // Объединение двух сигнатур служит только разрешению исходного модуля.
    // Выбор формы требует приобретённой периодичности и реализации выборки.
    MethodDescriptor::new(
        &["Выбрать", "Select"],
        Arity::range(0, 4),
        information_register_select,
    ),
    // Сигнатура периодического регистра подтверждена нативным контролем.
    // Она разрешает исходник, но не подменяет отсутствующее чтение записи.
    MethodDescriptor::new(
        &["ПолучитьПоследнее", "GetLast"],
        Arity::range(0, 2),
        information_register_get_last,
    ),
];

fn information_register_get_last(
    _: &dyn ObjectProtocol,
    _: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    Err(RtError::DynamicError(
        "РегистрСведений.ПолучитьПоследнее: чтение последней записи пока не поддерживается".into(),
    ))
}

fn information_register_select(
    _: &dyn ObjectProtocol,
    _: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    Err(RtError::DynamicError(
        "выборка регистра сведений пока не поддерживается".into(),
    ))
}

fn information_register_slice_first(
    _: &dyn ObjectProtocol,
    _: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    Err(RtError::DynamicError(
        "РегистрСведений.СрезПервых: получение среза пока не поддерживается".into(),
    ))
}

fn information_register_slice_last(
    _: &dyn ObjectProtocol,
    _: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    Err(RtError::DynamicError(
        "РегистрСведений.СрезПоследних: получение среза пока не поддерживается".into(),
    ))
}

fn information_register_create_record_key(
    _: &dyn ObjectProtocol,
    _: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    // Сигнатура связывает исходный менеджер; без измеренных ключевых полей
    // создавать успешное значение ключа было бы подменой реализации.
    Err(RtError::DynamicError(
        "РегистрСведений.СоздатьКлючЗаписи: создание ключа записи пока не поддерживается".into(),
    ))
}

fn information_register_get_template(
    receiver: &dyn ObjectProtocol,
    arguments: &[Value],
    context: &mut CallContext<'_>,
) -> RtResult<Value> {
    if let Some(value) = file_source::receiver_template(receiver, arguments, context)? {
        return Ok(value);
    }
    Err(RtError::DynamicError(
        "РегистрСведений.ПолучитьМакет: получение макета пока не поддерживается".into(),
    ))
}

fn information_register_create_record_manager(
    _: &dyn ObjectProtocol,
    _: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    Err(RtError::DynamicError(
        "РегистрСведений.СоздатьМенеджерЗаписи: менеджер записи пока не реализован".into(),
    ))
}

fn information_register_create_record_set(
    _: &dyn ObjectProtocol,
    _: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    Err(RtError::DynamicError(
        "РегистрСведений.СоздатьНаборЗаписей: набор записей пока не реализован".into(),
    ))
}

static ACCUMULATION_REGISTER_MANAGER_METHODS: &[MethodDescriptor] = &[
    accumulation_balance::METHOD,
    // Сигнатура платформы связывает и неисполняемые методы менеджера.
    // Без настоящего ресурса получение макета завершается явным отказом.
    MethodDescriptor::new(
        &["ПолучитьМакет", "GetTemplate"],
        Arity::exact(1),
        accumulation_register_get_template,
    ),
    MethodDescriptor::new(
        &["СоздатьНаборЗаписей", "CreateRecordSet"],
        Arity::exact(0),
        accumulation_register_create_record_set,
    ),
];

fn accumulation_register_get_template(
    receiver: &dyn ObjectProtocol,
    arguments: &[Value],
    context: &mut CallContext<'_>,
) -> RtResult<Value> {
    if let Some(value) = file_source::receiver_template(receiver, arguments, context)? {
        return Ok(value);
    }
    Err(RtError::DynamicError(
        "РегистрНакопления.ПолучитьМакет: получение макета пока не поддерживается".into(),
    ))
}

fn accumulation_register_create_record_set(
    _: &dyn ObjectProtocol,
    _: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    // Сигнатура связывает исходный менеджер. Набор и его операции нельзя
    // подменять успешным пустым объектом до реализации поведения записей.
    Err(RtError::DynamicError(
        "РегистрНакопления.СоздатьНаборЗаписей: набор записей пока не реализован".into(),
    ))
}

// Декодер связывает только неизменяемую идентичность с каталогом получателя.
// Пустой идентификатор обозначает пустую ссылку конкретного перечисления.
fn enumeration_reference_value(
    catalog: Arc<MetadataCatalog>,
    object_id: &str,
    value_id: &str,
) -> RtResult<Value> {
    if !catalog
        .materialized_object_by_id(object_id)?
        .is_some_and(|object| object.kind == MetadataKind::Enumeration)
    {
        return Err(RtError::DynamicError(
            "значение перечисления отсутствует в каталоге".into(),
        ));
    }
    // Список значений ограничивает поиск по имени, но не идентичность ссылки:
    // платформа сохраняет UUID удалённого значения в реквизитах метаданных.
    // Объявленные компонентом идентификаторы сохраняют прежний формат.
    let value_id: Arc<str> = if value_id.is_empty()
        || catalog.enumeration_values.iter().any(|value| {
            value.owner_id.as_ref() == object_id && value.value_id.as_ref() == value_id
        }) {
        value_id.into()
    } else {
        open_bsl::uuid::format(&open_bsl::uuid::parse(value_id)?).into()
    };
    let ty = catalog
        .reference_types
        .iter()
        .find(|(id, _)| id.as_ref() == object_id)
        .map(|(_, ty)| Arc::clone(ty))
        .ok_or_else(|| RtError::DynamicError("тип перечисления отсутствует в каталоге".into()))?;
    Ok(Value::new_object(EnumerationReferenceValue {
        catalog,
        object_id: object_id.into(),
        ty,
        value_id,
    }))
}

fn enumeration_reference_is_empty(
    receiver: &dyn ObjectProtocol,
    _: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let value = receiver
        .downcast_ref::<EnumerationReferenceValue>()
        .ok_or(RtError::NotAnObject)?;
    Ok(Value::Boolean(value.value_id.is_empty()))
}

fn enumeration_reference_metadata(
    receiver: &dyn ObjectProtocol,
    _: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let value = receiver
        .downcast_ref::<EnumerationReferenceValue>()
        .ok_or(RtError::NotAnObject)?;
    let owner = value
        .catalog
        .materialized_object_by_id(&value.object_id)?
        .ok_or(RtError::NotAnObject)?;
    Ok(metadata_object_value(&value.catalog, owner))
}

static ENUMERATION_REFERENCE_METHODS: &[MethodDescriptor] = &[
    MethodDescriptor::new(
        &["Пустая", "IsEmpty"],
        Arity::exact(0),
        enumeration_reference_is_empty,
    ),
    MethodDescriptor::new(
        &["Метаданные", "Metadata"],
        Arity::exact(0),
        enumeration_reference_metadata,
    ),
];

// НЕ ИЗМЕРЕНО(PREDEFINED.CONFIG_ENUM.REFERENCE_SURFACE): базовые операции
// измерены отдельными пробами; полный контракт переноса и языковых
// вариантов ещё не закрыт в реестре замеров.
impl ObjectProtocol for EnumerationReferenceValue {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &ENUMERATION_REFERENCE_TYPE
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        ENUMERATION_REFERENCE_METHODS
    }

    fn is_filled(&self) -> RtResult<bool> {
        Ok(!self.value_id.is_empty())
    }

    fn value_type(&self) -> TypeRef {
        TypeRef::Value(Arc::clone(&self.ty))
    }

    fn immutable_snapshot(&self) -> Option<&[u8]> {
        Some(self.value_id.as_bytes())
    }

    fn value_eq(&self, other: &open_bsl::ObjectRef) -> Option<bool> {
        Some(other.downcast_ref::<Self>().is_some_and(|other| {
            TypeRef::Value(Arc::clone(&self.ty)) == TypeRef::Value(Arc::clone(&other.ty))
                && self.value_id == other.value_id
        }))
    }

    fn value_hash_key(&self) -> Option<u64> {
        use std::hash::{Hash, Hasher};
        let mut hash = std::collections::hash_map::DefaultHasher::new();
        self.ty.package().hash(&mut hash);
        self.ty.identity().hash(&mut hash);
        self.value_id.hash(&mut hash);
        Some(hash.finish())
    }

    fn presentation(&self) -> RtResult<String> {
        if self.value_id.is_empty()
            || self
                .catalog
                .enumeration_values
                .iter()
                .any(|value| value.owner_id == self.object_id && value.value_id == self.value_id)
        {
            return Ok(self.display());
        }
        Err(RtError::DynamicError(
            "представление отсутствующего значения перечисления требует данных сеанса".into(),
        ))
    }

    fn presentation_with_context(&self, context: &CallContext<'_>) -> RtResult<String> {
        if let Ok(text) = self.presentation() {
            return Ok(text);
        }
        let number = self
            .catalog
            .materialized_object_by_id(&self.object_id)?
            .and_then(MetadataObject::reference_storage_number)
            .ok_or_else(|| {
                RtError::DynamicError(
                    "нет приобретённого номера DBNames для представления перечисления".into(),
                )
            })?;
        let label = match session(context)?.system_language() {
            Some(language) if folded_eq(language, "ru") => "Объект не найден",
            Some(language) if folded_eq(language, "en") => "Object not found",
            _ => {
                return Err(RtError::DynamicError(
                    "язык представления отсутствующего значения перечисления не подключён".into(),
                ));
            }
        };
        let physical = open_bsl::uuid::to_1c_bytes(open_bsl::uuid::parse(&self.value_id)?);
        use std::fmt::Write;
        let mut text = format!("<{label}> ({number}:");
        for byte in physical {
            write!(text, "{byte:02x}").expect("запись в строку не завершается ошибкой");
        }
        text.push(')');
        Ok(text)
    }

    fn display(&self) -> String {
        self.catalog
            .enumeration_values
            .iter()
            .find(|value| value.owner_id == self.object_id && value.value_id == self.value_id)
            .map_or_else(String::new, |value| {
                let language = self
                    .catalog
                    .primary_language_id()
                    .and_then(|id| self.catalog.object_by_id(id))
                    .and_then(|object| {
                        object
                            .string_properties
                            .iter()
                            .find(|(name, _)| name.as_ref() == "КодЯзыка")
                    })
                    .map(|(_, code)| code.as_ref());
                language
                    .and_then(|language| {
                        value
                            .synonyms
                            .iter()
                            .find(|(code, _)| code.as_ref() == language)
                    })
                    .map_or_else(|| value.name.to_string(), |(_, text)| text.to_string())
            })
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

impl ObjectProtocol for UserPasswordPoliciesManager {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &USER_PASSWORD_POLICIES_MANAGER_TYPE
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        USER_PASSWORD_POLICIES_MANAGER_METHODS
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

impl ObjectProtocol for UserPasswordPolicyValue {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &USER_PASSWORD_POLICY_TYPE
    }

    fn property_table(&self) -> &'static [PropertyDescriptor] {
        USER_PASSWORD_POLICY_PROPERTIES
    }
}

impl ObjectProtocol for AuthenticationLockManager {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &AUTHENTICATION_LOCK_MANAGER_TYPE
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        AUTHENTICATION_LOCK_MANAGER_METHODS
    }
}

impl ObjectProtocol for AuthenticationLockSettingsValue {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &AUTHENTICATION_LOCK_SETTINGS_TYPE
    }

    fn property_table(&self) -> &'static [PropertyDescriptor] {
        AUTHENTICATION_LOCK_SETTINGS_PROPERTIES
    }
}

impl ObjectProtocol for RandomPasswordGenerator {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &RANDOM_PASSWORD_GENERATOR_TYPE
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        RANDOM_PASSWORD_GENERATOR_METHODS
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
        if let Value::Str(name) = index {
            return query_selection_field(self, &name.shared_utf8());
        }
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

impl ObjectProtocol for DataCompositionTemplateComposerValue {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &DATA_COMPOSITION_TEMPLATE_COMPOSER_TYPE
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        DATA_COMPOSITION_TEMPLATE_COMPOSER_METHODS
    }
}

impl ObjectProtocol for DataCompositionProcessorValue {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &DATA_COMPOSITION_PROCESSOR_TYPE
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        DATA_COMPOSITION_PROCESSOR_METHODS
    }
}

impl ObjectProtocol for DataCompositionValueCollectionOutputProcessorValue {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &DATA_COMPOSITION_VALUE_COLLECTION_OUTPUT_PROCESSOR_TYPE
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        DATA_COMPOSITION_VALUE_COLLECTION_OUTPUT_PROCESSOR_METHODS
    }

    fn property_table(&self) -> &'static [PropertyDescriptor] {
        DATA_COMPOSITION_VALUE_COLLECTION_OUTPUT_PROCESSOR_PROPERTIES
    }
}

fn metadata_collection_find(
    receiver: &dyn ObjectProtocol,
    arguments: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let [Value::Str(wanted)] = arguments else {
        return Err(RtError::TypeError {
            expected: "Строка",
            op: "Найти",
        });
    };
    let wanted = wanted.to_string();
    if let Some(collection) = receiver.downcast_ref::<file_source::Collection>() {
        return Ok(collection.find(&wanted).unwrap_or(Value::Undefined));
    }
    if let Some(collection) = receiver.downcast_ref::<metadata_attributes::Collection>() {
        return Ok(collection.find(&wanted).unwrap_or(Value::Undefined));
    }
    let collection = receiver
        .downcast_ref::<MetadataCollection>()
        .ok_or(RtError::NotAnObject)?;
    Ok(collection
        .find(&wanted)
        .map(|object| metadata_object_value(&collection.catalog, object))
        .unwrap_or(Value::Undefined))
}

fn metadata_collection_count(
    receiver: &dyn ObjectProtocol,
    _: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let count = i64::try_from(receiver.collection_len()?)
        .map_err(|_| RtError::DynamicError("слишком много объектов метаданных".into()))?;
    Ok(Value::number_from_i64(count))
}

fn metadata_value_common_attribute_data_separation(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let value = receiver
        .downcast_ref::<MetadataValue>()
        .ok_or(RtError::NotAnObject)?;
    let object = value
        .catalog
        .materialized_object_by_id(&value.object_id)?
        .ok_or_else(|| RtError::DynamicError("объект метаданных отсутствует в каталоге".into()))?;
    let enabled = object.common_attribute_data_separation.ok_or_else(|| {
        RtError::DynamicError("режим разделения общего реквизита неизвестен".into())
    })?;
    Ok(Value::Enum(if enabled {
        EnumValue::CommonAttributeDataSeparationSeparate
    } else {
        EnumValue::CommonAttributeDataSeparationDontUse
    }))
}

fn metadata_value_common_attribute_auto_use(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let value = receiver
        .downcast_ref::<MetadataValue>()
        .ok_or(RtError::NotAnObject)?;
    let object = value
        .catalog
        .materialized_object_by_id(&value.object_id)?
        .ok_or_else(|| RtError::DynamicError("объект метаданных отсутствует в каталоге".into()))?;
    let enabled = object.common_attribute_auto_use.ok_or_else(|| {
        RtError::DynamicError("автоиспользование общего реквизита неизвестно".into())
    })?;
    Ok(Value::Enum(if enabled {
        EnumValue::CommonAttributeAutoUseUse
    } else {
        EnumValue::CommonAttributeAutoUseDontUse
    }))
}

fn metadata_value_full_name(
    receiver: &dyn ObjectProtocol,
    _: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    if receiver.downcast_ref::<file_source::View>().is_some() {
        return Err(RtError::DynamicError(
            "полное имя этого вида метаданных пока не поддерживается".into(),
        ));
    }
    let value = receiver
        .downcast_ref::<MetadataValue>()
        .ok_or(RtError::NotAnObject)?;
    let object = value
        .catalog
        .materialized_object_by_id(&value.object_id)?
        .ok_or_else(|| RtError::DynamicError("объект метаданных отсутствует в каталоге".into()))?;
    let variant = value
        .catalog
        .configuration
        .as_ref()
        .ok_or_else(|| {
            RtError::DynamicError("вариант встроенного языка конфигурации неизвестен".into())
        })?
        .script_variant;
    // Нативные контроли обоих вариантов языка закрепляют префикс,
    // независимо от языка обращения к объекту и методу.
    let prefix = match (object.kind, variant) {
        (MetadataKind::Catalog, ScriptVariant::Russian) => "Справочник",
        (MetadataKind::Catalog, ScriptVariant::English) => "Catalog",
        (MetadataKind::CommonAttribute, ScriptVariant::Russian) => "ОбщийРеквизит",
        (MetadataKind::CommonAttribute, ScriptVariant::English) => "CommonAttribute",
        (MetadataKind::Constant, ScriptVariant::Russian) => "Константа",
        (MetadataKind::Constant, ScriptVariant::English) => "Constant",
        (MetadataKind::FunctionalOption, ScriptVariant::Russian) => "ФункциональнаяОпция",
        (MetadataKind::FunctionalOption, ScriptVariant::English) => "FunctionalOption",
        _ => {
            return Err(RtError::DynamicError(
                "полное имя этого вида метаданных пока не поддерживается".into(),
            ));
        }
    };
    Ok(Value::Str(
        format!("{prefix}.{}", object.name()).as_str().into(),
    ))
}

fn metadata_value_name(receiver: &dyn ObjectProtocol, _: &mut CallContext<'_>) -> RtResult<Value> {
    if let Some(attribute) = receiver.downcast_ref::<metadata_attributes::Attribute>() {
        return Ok(Value::Str(attribute.name().into()));
    }
    let value = receiver
        .downcast_ref::<MetadataValue>()
        .ok_or(RtError::NotAnObject)?;
    let object = value
        .catalog
        .materialized_object_by_id(&value.object_id)?
        .ok_or_else(|| RtError::DynamicError("объект метаданных отсутствует в каталоге".into()))?;
    Ok(Value::Str(object.name().into()))
}

fn metadata_value_attributes(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let value = receiver
        .downcast_ref::<MetadataValue>()
        .ok_or(RtError::NotAnObject)?;
    let object = value
        .catalog
        .materialized_object_by_id(&value.object_id)?
        .ok_or_else(|| RtError::DynamicError("объект метаданных отсутствует в каталоге".into()))?;
    if object.kind() != MetadataKind::Document {
        return Err(RtError::UnknownProperty("Реквизиты".into()));
    }
    let attributes = object.document_attributes.as_ref().ok_or_else(|| {
        RtError::DynamicError("коллекция реквизитов документа не приобретена".into())
    })?;
    Ok(metadata_attributes::collection(
        Arc::clone(&value.catalog),
        Arc::clone(&value.object_id),
        Arc::clone(attributes),
    ))
}

fn metadata_collection_get(
    receiver: &dyn ObjectProtocol,
    arguments: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    // `Получить` приводит аргумент к числу; строковый индекс `[...]`
    // отдельно ищет имя реквизита. Различие подтверждено замером платформы.
    let number = collection_index_number(&arguments[0])?.trunc_to_scale(0);
    receiver.get_index(&Value::Number(number))
}

fn metadata_value_subsystems(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let value = receiver
        .downcast_ref::<MetadataValue>()
        .ok_or(RtError::NotAnObject)?;
    let object = value
        .catalog
        .materialized_object_by_id(&value.object_id)?
        .ok_or_else(|| RtError::DynamicError("объект метаданных отсутствует в каталоге".into()))?;
    if object.kind != MetadataKind::Subsystem {
        return Err(RtError::UnknownProperty("Подсистемы".into()));
    }
    Ok(Value::new_object(MetadataCollection {
        catalog: Arc::clone(&value.catalog),
        kind: MetadataKind::Subsystem,
        subsystem_parent: Some(Arc::clone(&object.id)),
    }))
}

fn metadata_value_subsystem_command_interface(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let value = receiver
        .downcast_ref::<MetadataValue>()
        .ok_or(RtError::NotAnObject)?;
    let object = value
        .catalog
        .materialized_object_by_id(&value.object_id)?
        .ok_or_else(|| RtError::DynamicError("объект метаданных отсутствует в каталоге".into()))?;
    if object.kind != MetadataKind::Subsystem {
        return Err(RtError::UnknownProperty(
            "ВключатьВКомандныйИнтерфейс".into(),
        ));
    }
    object
        .subsystem_command_interface
        .map(Value::Boolean)
        .ok_or_else(|| RtError::DynamicError("признак подсистемы отсутствует в метаданных".into()))
}

fn metadata_value_language_code(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let value = receiver
        .downcast_ref::<MetadataValue>()
        .ok_or(RtError::NotAnObject)?;
    let object = value
        .catalog
        .materialized_object_by_id(&value.object_id)?
        .ok_or_else(|| RtError::DynamicError("объект метаданных отсутствует в каталоге".into()))?;
    if object.kind != MetadataKind::Language {
        return Err(RtError::UnknownProperty("КодЯзыка".into()));
    }
    object
        .string_properties
        .iter()
        .find(|(name, _)| folded_eq(name, "КодЯзыка"))
        .map(|(_, code)| Value::Str(code.as_ref().into()))
        .ok_or_else(|| RtError::DynamicError("код языка отсутствует в метаданных".into()))
}

fn configuration_description(root: &MetadataRoot) -> RtResult<&ConfigurationDescription> {
    root.catalog.configuration.as_ref().ok_or_else(|| {
        RtError::DynamicError("свойства конфигурации отсутствуют в каталоге метаданных".into())
    })
}

fn metadata_root_common_attributes(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let root = receiver
        .downcast_ref::<MetadataRoot>()
        .ok_or(RtError::NotAnObject)?;
    let ids = root.catalog.common_attribute_ids().ok_or_else(|| {
        RtError::DynamicError("состав общих реквизитов конфигурации неизвестен".into())
    })?;
    // Публикуем только полный состав корня, а не применимые поля справочников.
    for id in ids {
        if !root
            .catalog
            .materialized_object_by_id(id)?
            .is_some_and(|object| object.kind == MetadataKind::CommonAttribute)
        {
            return Err(RtError::DynamicError(
                "неполный состав общих реквизитов конфигурации".into(),
            ));
        }
    }
    Ok(Value::new_object(MetadataCollection {
        catalog: Arc::clone(&root.catalog),
        kind: MetadataKind::CommonAttribute,
        subsystem_parent: None,
    }))
}

fn metadata_root_name(receiver: &dyn ObjectProtocol, _: &mut CallContext<'_>) -> RtResult<Value> {
    let root = receiver
        .downcast_ref::<MetadataRoot>()
        .ok_or(RtError::NotAnObject)?;
    Ok(Value::Str(
        configuration_description(root)?.name.as_ref().into(),
    ))
}

fn metadata_root_default_interface(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let root = receiver
        .downcast_ref::<MetadataRoot>()
        .ok_or(RtError::NotAnObject)?;
    // Нативные пустой и назначенный интерфейсы различаются UUID настройки.
    // Неизвестные данные и неподдержанный объект не превращаем в отсутствие.
    match root.catalog.default_interface_id {
        Some(id) if id == [0; 16] => Ok(Value::Undefined),
        Some(_) => Err(RtError::DynamicError(
            "метаданные назначенного основного интерфейса не поддерживаются".into(),
        )),
        None => Err(RtError::DynamicError(
            "основной интерфейс конфигурации не приобретён".into(),
        )),
    }
}

fn metadata_root_primary_language(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let root = receiver
        .downcast_ref::<MetadataRoot>()
        .ok_or(RtError::NotAnObject)?;
    let language = root
        .catalog
        .primary_language_id()
        .and_then(|id| root.catalog.object_by_id(id))
        .ok_or_else(|| {
            RtError::DynamicError("основной язык отсутствует в каталоге метаданных".into())
        })?;
    Ok(metadata_object_value(&root.catalog, language))
}

fn metadata_root_subsystems(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let root = receiver
        .downcast_ref::<MetadataRoot>()
        .ok_or(RtError::NotAnObject)?;
    Ok(Value::new_object(MetadataCollection {
        catalog: Arc::clone(&root.catalog),
        kind: MetadataKind::Subsystem,
        subsystem_parent: None,
    }))
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

fn metadata_object_common_attribute_data_separation(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    receiver
        .downcast_ref::<MetadataObjectProperties>()
        .ok_or(RtError::NotAnObject)?;
    Ok(Value::new_object(MetadataPropertyEnumeration(
        EnumKind::CommonAttributeDataSeparation,
    )))
}

fn metadata_object_common_attribute_auto_use(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    receiver
        .downcast_ref::<MetadataObjectProperties>()
        .ok_or(RtError::NotAnObject)?;
    Ok(Value::new_object(MetadataPropertyEnumeration(
        EnumKind::CommonAttributeAutoUse,
    )))
}

fn metadata_object_common_attribute_use(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    receiver
        .downcast_ref::<MetadataObjectProperties>()
        .ok_or(RtError::NotAnObject)?;
    Ok(Value::new_object(MetadataPropertyEnumeration(
        EnumKind::CommonAttributeUse,
    )))
}

fn metadata_object_extension_purpose(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    receiver
        .downcast_ref::<MetadataObjectProperties>()
        .ok_or(RtError::NotAnObject)?;
    Ok(Value::new_object(MetadataPropertyEnumeration(
        EnumKind::ConfigurationExtensionPurpose,
    )))
}

fn metadata_object_compatibility_modes(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    receiver
        .downcast_ref::<MetadataObjectProperties>()
        .ok_or(RtError::NotAnObject)?;
    Ok(Value::new_object(MetadataPropertyEnumeration(
        EnumKind::CompatibilityMode,
    )))
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

fn manager_find_by_attribute(
    receiver: &dyn ObjectProtocol,
    args: &[Value],
    context: &mut CallContext<'_>,
) -> RtResult<Value> {
    let manager = receiver
        .downcast_ref::<ManagerValue>()
        .ok_or(RtError::NotAnObject)?;
    let object = manager
        .catalog
        .materialized_object_by_id(&manager.object_id)?
        .ok_or(RtError::NotAnObject)?;
    let Value::Str(name) = &args[0] else {
        return Err(RtError::TypeError {
            expected: "Строка",
            op: "НайтиПоРеквизиту",
        });
    };
    let name = name.to_string();
    let name = if object.kind() == MetadataKind::Document {
        object
            .document_attributes()
            .ok_or_else(|| {
                RtError::DynamicError("собственные реквизиты документа не приобретены".into())
            })?
            .iter()
            .map(DocumentAttribute::name)
            .find(|known| folded_eq(known, &name))
    } else {
        object
            .catalog_attribute_names()
            .ok_or_else(|| {
                RtError::DynamicError("собственные реквизиты справочника не приобретены".into())
            })?
            .iter()
            .map(String::as_str)
            .find(|known| folded_eq(known, &name))
    }
    .ok_or_else(|| RtError::UnknownProperty(name.clone()))?;
    if matches!(&args[1], Value::Object(value) if matches!(value.as_ref(), BslObject::Array(_))) {
        return Err(RtError::TypeError {
            expected: "Значение реквизита",
            op: "НайтиПоРеквизиту",
        });
    }
    let request = FindByAttributeRequest {
        attribute: Rc::from(name),
        value: args[1].clone(),
        parent: args
            .get(2)
            .filter(|value| !matches!(value, Value::Undefined))
            .cloned(),
        owner: args
            .get(3)
            .filter(|value| !matches!(value, Value::Undefined))
            .cloned(),
    };
    let (principal, adapter) = current_data_access(context)?;
    match adapter.find_by_attribute(object, &request, &principal, context)? {
        None => {
            configuration_reference_value(Arc::clone(&manager.catalog), &manager.object_id, [0; 16])
        }
        Some(value)
            if reference_matches_owner(
                &value,
                &manager.catalog,
                &manager.object_id,
                object.kind(),
            ) =>
        {
            Ok(value)
        }
        Some(_) => Err(RtError::DynamicError(
            "адаптер поиска вернул несовместимую ссылку".into(),
        )),
    }
}

fn manager_name(receiver: &dyn ObjectProtocol, _: &mut CallContext<'_>) -> RtResult<Value> {
    let manager = receiver
        .downcast_ref::<ManagerValue>()
        .ok_or(RtError::NotAnObject)?;
    let object = manager
        .catalog
        .materialized_object_by_id(&manager.object_id)?
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
        .materialized_object_by_id(&manager.object_id)?
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

fn set_constant_value(
    receiver: &dyn ObjectProtocol,
    arguments: &[Value],
    context: &mut CallContext<'_>,
) -> RtResult<Value> {
    let manager = receiver
        .downcast_ref::<ManagerValue>()
        .ok_or(RtError::NotAnObject)?;
    let metadata = manager
        .catalog
        .materialized_object_by_id(&manager.object_id)?
        .ok_or_else(|| RtError::DynamicError("объект менеджера отсутствует в каталоге".into()))?;
    let (principal, data_access) = current_data_access(context)?;
    // Успех означает завершённую операцию адаптера, а не замену чтения
    // локальным значением. Адаптер SDK для чтения запись явно отвергает.
    data_access.set_constant_value(metadata, &arguments[0], &principal, context)?;
    Ok(Value::Undefined)
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
    // Справка объявляет строку, но прямой замер свойства на 8.3.27
    // даёт `ДвоичныеДанные`. Строковый Base64 остаётся формой данных хоста.
    let bytes =
        open_bsl::decode_base64(extension_value(receiver)?.hash_sum.as_ref()).ok_or_else(|| {
            RtError::DynamicError("хеш расширения содержит некорректный Base64".into())
        })?;
    Ok(Value::binary_data_of(bytes))
}

fn extension_purpose(receiver: &dyn ObjectProtocol, _: &mut CallContext<'_>) -> RtResult<Value> {
    extension_value(receiver)?
        .purpose
        .map(ConfigurationExtensionPurpose::value)
        .ok_or_else(|| RtError::DynamicError("назначение расширения не приобретено".into()))
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
            parameters: Value::new_dictionary_structure(Vec::new()),
            temp_tables_manager: None,
        }),
    }))
}

fn new_query_schema(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    Err(RtError::DynamicError(
        "объектная модель схемы запроса не реализована".into(),
    ))
}

fn new_query_schema_expression(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    // Выражение участвует в построении схемы запроса; пустой объект скрыл бы
    // отсутствие её объектной модели и изменил результат работы конфигурации.
    Err(RtError::DynamicError(
        "объектная модель выражения схемы запроса не реализована".into(),
    ))
}

fn new_object_deletion(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    // Объект создаётся для последующего `Записать`, который удаляет запись
    // базы. Заглушка с успешным вызовом скрыла бы несделанное удаление.
    Err(RtError::DynamicError(
        "удаление объекта через текущий сеанс пока не поддерживается".into(),
    ))
}

fn new_configuration_metadata_object(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    // Двоичные данные `.cf` или `.cfe` описывают самостоятельную конфигурацию;
    // метаданные текущей базы не могут заменять результат их разбора.
    Err(RtError::DynamicError(
        "разбор файла конфигурации из двоичных данных пока не поддерживается".into(),
    ))
}

fn new_data_composition_settings_composer(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    Err(RtError::DynamicError(
        "объектная модель настроек компоновки данных не реализована".into(),
    ))
}

fn new_data_composition_available_settings_source(
    _: &mut CallContext<'_>,
    _: &[Value],
) -> RtResult<Value> {
    Err(RtError::DynamicError(
        "объектная модель источника доступных настроек не реализована".into(),
    ))
}

fn new_data_composition_details_process(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    Err(RtError::DynamicError(
        "объектная модель обработки расшифровки компоновки данных не реализована".into(),
    ))
}

fn new_configuration_update_description(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    Err(RtError::DynamicError(
        "разбор описания обновления конфигурации из CFU не реализован".into(),
    ))
}

fn new_report_builder(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    Err(RtError::DynamicError(
        "объектная модель построителя отчёта не реализована".into(),
    ))
}

fn new_query_builder(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    // Обычный `Запрос` не хранит отбор, порядок и выбранные поля построителя.
    Err(RtError::DynamicError(
        "объектная модель построителя запроса не реализована".into(),
    ))
}

fn new_data_source_description(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    Err(RtError::DynamicError(
        "объектная модель описания источника данных не реализована".into(),
    ))
}

fn new_configuration_description(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    Err(RtError::DynamicError(
        "разбор описания конфигурации из файла cf/cfe не реализован".into(),
    ))
}

fn new_standard_period(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    Err(RtError::DynamicError(
        "объектная модель стандартного периода не реализована".into(),
    ))
}

fn new_data_composition_schema(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    Err(RtError::DynamicError(
        "объектная модель схемы компоновки данных не реализована".into(),
    ))
}

fn new_data_composition_spreadsheet_output(
    _: &mut CallContext<'_>,
    _: &[Value],
) -> RtResult<Value> {
    Err(RtError::DynamicError(
        "вывод результата компоновки данных в табличный документ не реализован".into(),
    ))
}

static ADJUSTED_POINT_IN_TIME_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-orm",
    name: "МоментВремениСУточнениемПериода",
    type_display: "Point in time with period adjustment",
    type_names: &["PointInTimeWithPeriodAdjustment"],
};

// Объявление позволяет компилировать исходный менеджер обработки, как и
// соседняя граница запроса. Фактическое создание требует объектной модели.
fn new_adjusted_point_in_time(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    Err(RtError::DynamicError(
        "объектная модель момента времени с уточнением периода не реализована".into(),
    ))
}

fn new_data_composition_user_settings(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    Err(RtError::DynamicError(
        "объектная модель пользовательских настроек компоновки данных не реализована".into(),
    ))
}

fn new_data_composition_template_composer(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    Ok(Value::new_object(DataCompositionTemplateComposerValue))
}

fn compose_data_composition_template(
    _: &dyn ObjectProtocol,
    _: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    Err(RtError::DynamicError(
        "движок компоновки макета данных не реализован".into(),
    ))
}

fn new_data_composition_processor(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    Ok(Value::new_object(DataCompositionProcessorValue))
}

fn data_composition_processing_unavailable(
    _: &dyn ObjectProtocol,
    _: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    Err(RtError::DynamicError(
        "движок компоновки данных не реализован".into(),
    ))
}

fn new_data_composition_value_collection_output_processor(
    _: &mut CallContext<'_>,
    _: &[Value],
) -> RtResult<Value> {
    Ok(Value::new_object(
        DataCompositionValueCollectionOutputProcessorValue::default(),
    ))
}

fn data_composition_output_unavailable(
    _: &dyn ObjectProtocol,
    _: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    Err(RtError::DynamicError(
        "движок вывода результата компоновки данных не реализован".into(),
    ))
}

fn show_output_percent(receiver: &dyn ObjectProtocol, _: &mut CallContext<'_>) -> RtResult<Value> {
    let output = receiver
        .downcast_ref::<DataCompositionValueCollectionOutputProcessorValue>()
        .ok_or(RtError::NotAnObject)?;
    Ok(Value::Boolean(output.show_output_percent.get()))
}

fn set_show_output_percent(
    receiver: &dyn ObjectProtocol,
    value: Value,
    _: &mut CallContext<'_>,
) -> RtResult<()> {
    let output = receiver
        .downcast_ref::<DataCompositionValueCollectionOutputProcessorValue>()
        .ok_or(RtError::NotAnObject)?;
    let Value::Boolean(value) = value else {
        return Err(RtError::TypeError {
            expected: "Булево",
            op: "ОтображатьПроцентВывода",
        });
    };
    output.show_output_percent.set(value);
    Ok(())
}

fn new_random_password_generator(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    Ok(Value::new_object(RandomPasswordGenerator))
}

fn random_password(
    receiver: &dyn ObjectProtocol,
    _: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    receiver
        .downcast_ref::<RandomPasswordGenerator>()
        .ok_or(RtError::NotAnObject)?;
    Err(RtError::DynamicError(
        "криптографический источник генерации паролей не предоставлен хостом".into(),
    ))
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

/// Создаёт снимок `ХранилищеЗначения` из внутренней сериализации платформы.
/// Строка содержит значение без заголовка и сжатия поля информационной базы.
/// Использует тот же декодер и отделение данных, что существующие операции BSL.
///
/// # Errors
/// Возвращает ошибку для неверного или неподдерживаемого внутреннего формата,
/// непереносимого значения либо превышения ограничений снимка.
pub fn value_storage_from_internal(text: &str, context: &mut CallContext<'_>) -> RtResult<Value> {
    let value = open_bsl::value_from_string_internal(text, context.runtime_shapes())?;
    new_value_storage(context, &[value])
}

/// Создаёт снимок хранилища с явным резолвером скалярных объектов хоста.
/// Резолвер восстанавливает конфигурационные значения до создания снимка;
/// без распознанного вида действуют прежние ограничения переносимости.
///
/// # Errors
/// Возвращает ошибки формата, резолвера и создания снимка.
pub fn value_storage_from_internal_with_resolver(
    text: &str,
    context: &mut CallContext<'_>,
    resolver: &mut dyn FnMut(&str, &str) -> RtResult<Option<Value>>,
) -> RtResult<Value> {
    let value = open_bsl::value_from_string_internal_with_resolver(
        text,
        context.runtime_shapes(),
        resolver,
    )?;
    new_value_storage(context, &[value])
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

fn lock_data_for_edit(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    Err(RtError::DynamicError(
        "блокировка данных для редактирования не поддерживается адаптером сеанса".into(),
    ))
}

fn unlock_data_for_edit(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    Err(RtError::DynamicError(
        "снятие блокировки данных для редактирования не поддерживается адаптером сеанса".into(),
    ))
}

fn erase_infobase_data(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    // Адаптер информационной базы здесь только читает данные. Удаление
    // всей базы или области не может стать пустой успешной операцией.
    Err(RtError::DynamicError(
        "УдалитьДанныеИнформационнойБазы недоступно в хосте только для чтения".into(),
    ))
}

fn filter_criteria(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    // Глобальное имя связывает исходный неисполняемый метод конфигурации.
    // Без приобретённого состава нельзя выдавать пустой менеджер критериев
    // или подменять выполнение отбора успешным результатом.
    Err(RtError::DynamicError(
        "критерии отбора конфигурации пока не поддерживаются этим хостом".into(),
    ))
}

fn ws_references(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    // Имена позволяют связать исходный модуль без переписывания текста.
    // Без приобретённого описания нельзя подменять состав WS-ссылок пустым.
    Err(RtError::DynamicError(
        "WS-ссылки конфигурации пока не поддерживаются этим хостом".into(),
    ))
}

fn get_database_data_size(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    // Сигнатура связывает неисполняемый обработчик исходной конфигурации.
    // Объём доступных пользователю данных зависит от прав и разделителей;
    // физический размер таблицы не может подменить этот подсчёт.
    Err(RtError::DynamicError(
        "ПолучитьРазмерДанныхБазыДанных: получение размера данных пока не поддерживается".into(),
    ))
}

fn delete_objects(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    // Объявление позволяет компилировать исходный модуль, но не предоставляет
    // хосту только для чтения право удалять данные или менять выходную таблицу.
    Err(RtError::DynamicError(
        "УдалитьОбъекты недоступно в хосте только для чтения".into(),
    ))
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

fn query_parameters(receiver: &dyn ObjectProtocol, _: &mut CallContext<'_>) -> RtResult<Value> {
    let query = receiver
        .downcast_ref::<QueryValue>()
        .ok_or(RtError::NotAnObject)?;
    Ok(query.state.borrow().parameters.clone())
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
    context: &mut CallContext<'_>,
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
    let parameters = query.state.borrow().parameters.clone();
    let shapes = context.runtime_shapes();
    let field = shapes.names.intern(&name);
    parameters.structure_insert_spelled(
        field,
        &name,
        value.clone(),
        &mut shapes.shapes,
        &shapes.names,
    )?;
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
        // Конструктор создаёт структуру, а свойство не позволяет заменить её.
        // Снимок запроса собирается до вызова хоста, не удерживая заимствование.
        let Value::Object(parameters) = &state.parameters else {
            unreachable!("параметры запроса всегда являются структурой");
        };
        let BslObject::Structure(parameters) = parameters.as_ref() else {
            unreachable!("параметры запроса всегда являются структурой");
        };
        let parameters = parameters.borrow();
        let names = &context.runtime_shapes().names;
        let parameters = (0..parameters.len())
            .map(|index| {
                let (field, value) = parameters
                    .entry_at(index)
                    .ok_or(RtError::InvalidBytecode("индекс параметра запроса неверен"))?;
                let name = parameters
                    .spelling_at(index, names)
                    .ok_or(RtError::UnknownField(field))?;
                Ok((Rc::from(name), value))
            })
            .collect::<RtResult<Vec<_>>>()?;
        QueryRequest {
            text: Rc::clone(&state.text),
            parameters: parameters.into(),
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
    let Some(data) = data_access.execute_query_result(&request, &principal, context)? else {
        return Ok(Value::Undefined);
    };
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
        names: &["Параметры", "Parameters"],
        get: query_parameters,
        set: None,
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

static DATA_COMPOSITION_TEMPLATE_COMPOSER_METHODS: &[MethodDescriptor] = &[MethodDescriptor::new(
    &["Выполнить", "Execute"],
    Arity::range(2, 7),
    compose_data_composition_template,
)];

static DATA_COMPOSITION_PROCESSOR_METHODS: &[MethodDescriptor] = &[
    MethodDescriptor::new(
        &["Инициализировать", "Initialize"],
        Arity::range(1, 6),
        data_composition_processing_unavailable,
    )
    .as_procedure(),
    MethodDescriptor::new(
        &["Сбросить", "Reset"],
        Arity::exact(0),
        data_composition_processing_unavailable,
    )
    .as_procedure(),
    MethodDescriptor::new(
        &["Следующий", "Next"],
        Arity::exact(0),
        data_composition_processing_unavailable,
    ),
];

static DATA_COMPOSITION_VALUE_COLLECTION_OUTPUT_PROCESSOR_METHODS: &[MethodDescriptor] = &[
    MethodDescriptor::new(
        &["УстановитьОбъект", "SetObject"],
        Arity::range(0, 1),
        data_composition_output_unavailable,
    )
    .as_procedure(),
    MethodDescriptor::new(
        &["Вывести", "Output"],
        Arity::range(1, 2),
        data_composition_output_unavailable,
    ),
    MethodDescriptor::new(
        &["ВывестиЭлемент", "OutputItem"],
        Arity::exact(1),
        data_composition_output_unavailable,
    )
    .as_procedure(),
    MethodDescriptor::new(
        &["НачатьВывод", "BeginOutput"],
        Arity::exact(0),
        data_composition_output_unavailable,
    )
    .as_procedure(),
    MethodDescriptor::new(
        &["ЗакончитьВывод", "EndOutput"],
        Arity::exact(0),
        data_composition_output_unavailable,
    ),
];

static DATA_COMPOSITION_VALUE_COLLECTION_OUTPUT_PROCESSOR_PROPERTIES: &[PropertyDescriptor] =
    &[PropertyDescriptor {
        names: &["ОтображатьПроцентВывода", "ShowOutputPercent"],
        get: show_output_percent,
        set: Some(set_show_output_percent),
    }];

static METADATA_COLLECTION_METHODS: &[MethodDescriptor] = &[
    MethodDescriptor::new(
        &["Получить", "Get"],
        Arity::exact(1),
        metadata_collection_get,
    ),
    MethodDescriptor::new(
        &["Найти", "Find"],
        Arity::exact(1),
        metadata_collection_find,
    ),
    MethodDescriptor::new(
        &["Количество", "Count"],
        Arity::exact(0),
        metadata_collection_count,
    ),
];

static METADATA_ROOT_PROPERTIES: &[PropertyDescriptor] = &[
    PropertyDescriptor {
        names: &["ОбщиеФормы", "CommonForms"],
        get: file_source::common_forms,
        set: None,
    },
    PropertyDescriptor {
        names: &["ОбщиеМакеты", "CommonTemplates"],
        get: file_source::common_templates,
        set: None,
    },
    PropertyDescriptor {
        names: &["ОсновнойИнтерфейс", "DefaultInterface"],
        get: metadata_root_default_interface,
        set: None,
    },
    PropertyDescriptor {
        names: &["ОбщиеРеквизиты", "CommonAttributes"],
        get: metadata_root_common_attributes,
        set: None,
    },
    PropertyDescriptor {
        names: &["ОсновнойЯзык", "DefaultLanguage"],
        get: metadata_root_primary_language,
        set: None,
    },
    PropertyDescriptor {
        names: &["Подсистемы", "Subsystems"],
        get: metadata_root_subsystems,
        set: None,
    },
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
        names: &[
            "НазначениеРасширенияКонфигурации",
            "ConfigurationExtensionPurpose",
        ],
        get: metadata_object_extension_purpose,
        set: None,
    },
    PropertyDescriptor {
        names: &[
            "РазделениеДанныхОбщегоРеквизита",
            "CommonAttributeDataSeparation",
        ],
        get: metadata_object_common_attribute_data_separation,
        set: None,
    },
    PropertyDescriptor {
        names: &["АвтоИспользованиеОбщегоРеквизита", "CommonAttributeAutoUse"],
        get: metadata_object_common_attribute_auto_use,
        set: None,
    },
    PropertyDescriptor {
        names: &["ИспользованиеОбщегоРеквизита", "CommonAttributeUse"],
        get: metadata_object_common_attribute_use,
        set: None,
    },
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

static METADATA_VALUE_METHODS: &[MethodDescriptor] = &[MethodDescriptor::new(
    &["ПолноеИмя", "FullName"],
    Arity::exact(0),
    metadata_value_full_name,
)];

static METADATA_VALUE_PROPERTIES: &[PropertyDescriptor] = &[
    PropertyDescriptor {
        names: &["РазделениеДанных", "DataSeparation"],
        get: metadata_value_common_attribute_data_separation,
        set: None,
    },
    PropertyDescriptor {
        names: &["АвтоИспользование", "AutoUse"],
        get: metadata_value_common_attribute_auto_use,
        set: None,
    },
    PropertyDescriptor {
        names: &["УсловноеРазделение", "ConditionalSeparation"],
        get: common_attribute_content::get_condition,
        set: None,
    },
    PropertyDescriptor {
        names: &["Состав", "Content"],
        get: common_attribute_content::get,
        set: None,
    },
    PropertyDescriptor {
        names: &["Реквизиты", "Attributes"],
        get: metadata_value_attributes,
        set: None,
    },
    metadata_attributes::PROPERTIES[1],
    metadata_attributes::PROPERTIES[2],
    PropertyDescriptor {
        names: &["КодЯзыка", "LanguageCode"],
        get: metadata_value_language_code,
        set: None,
    },
    PropertyDescriptor {
        names: &["Имя", "Name"],
        get: metadata_value_name,
        set: None,
    },
    PropertyDescriptor {
        names: &["Подсистемы", "Subsystems"],
        get: metadata_value_subsystems,
        set: None,
    },
    PropertyDescriptor {
        names: &["ВключатьВКомандныйИнтерфейс", "IncludeInCommandInterface"],
        get: metadata_value_subsystem_command_interface,
        set: None,
    },
    PropertyDescriptor {
        names: &["Формы", "Forms"],
        get: file_source::forms,
        set: None,
    },
    PropertyDescriptor {
        names: &["Макеты", "Templates"],
        get: file_source::templates,
        set: None,
    },
];

static MANAGER_PROPERTIES: &[PropertyDescriptor] = &[PropertyDescriptor {
    names: &["Имя", "Name"],
    get: manager_name,
    set: None,
}];

static BUSINESS_PROCESS_MANAGER_PROPERTIES: &[PropertyDescriptor] = &[
    MANAGER_PROPERTIES[0],
    PropertyDescriptor {
        names: &["ТочкиМаршрута", "RoutePoints"],
        get: route_points::manager_property,
        set: None,
    },
];

fn create_constant_value_manager(
    _: &dyn ObjectProtocol,
    _: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    // Декларация позволяет разрешить исходный модуль, но отдельный объект
    // менеджера значения требует реализации и измерений его операций.
    Err(RtError::DynamicError(
        "менеджер значения константы не поддерживается".into(),
    ))
}

static CONSTANT_MANAGER_METHODS: &[MethodDescriptor] = &[
    MethodDescriptor::new(&["Получить", "Get"], Arity::exact(0), constant_value),
    MethodDescriptor::new(&["Установить", "Set"], Arity::exact(1), set_constant_value),
    MethodDescriptor::new(
        &["СоздатьМенеджерЗначения", "CreateValueManager"],
        Arity::exact(0),
        create_constant_value_manager,
    ),
];

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

static USER_PASSWORD_POLICIES_MANAGER_METHODS: &[MethodDescriptor] = &[MethodDescriptor::new(
    &["НайтиПоИмени", "FindByName"],
    Arity::range(0, 1),
    find_user_password_policy,
)];

static AUTHENTICATION_LOCK_MANAGER_METHODS: &[MethodDescriptor] = &[
    MethodDescriptor::new(
        &["ПолучитьНастройки", "GetSettings"],
        Arity::exact(0),
        get_authentication_lock_settings,
    ),
    MethodDescriptor::new(
        &["УстановитьНастройки", "SetSettings"],
        Arity::range(0, 1),
        set_authentication_lock_settings,
    )
    .as_procedure(),
];

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

static INFO_BASE_USER_PROPERTIES: &[PropertyDescriptor] = &[
    PropertyDescriptor {
        names: &["Имя", "Name"],
        get: current_user_name,
        set: None,
    },
    PropertyDescriptor {
        names: &["РазделениеДанных", "DataSeparation"],
        get: current_user_data_separation,
        set: None,
    },
    PropertyDescriptor {
        names: &["УникальныйИдентификатор", "UUID"],
        get: current_user_uuid,
        set: None,
    },
];

static USER_PASSWORD_POLICY_PROPERTIES: &[PropertyDescriptor] = &[PropertyDescriptor {
    names: &["МинимальнаяДлинаПаролей", "PasswordMinLength"],
    get: user_password_minimum_length,
    set: None,
}];

static AUTHENTICATION_LOCK_SETTINGS_PROPERTIES: &[PropertyDescriptor] = &[
    PropertyDescriptor {
        names: &["ДлительностьБлокировки", "LockDuration"],
        get: authentication_lock_duration,
        set: Some(set_authentication_lock_duration),
    },
    PropertyDescriptor {
        names: &[
            "МаксимальноеКоличествоНеуспешныхПопыток",
            "MaxUnsuccessfulAttemptsCount",
        ],
        get: authentication_lock_max_unsuccessful_attempts_count,
        set: Some(set_authentication_lock_max_unsuccessful_attempts_count),
    },
    PropertyDescriptor {
        names: &["КодыДополненияИмениПользователя", "UserNameAdditionCodes"],
        get: authentication_lock_user_name_addition_codes,
        set: Some(set_authentication_lock_user_name_addition_codes),
    },
];

static RANDOM_PASSWORD_GENERATOR_METHODS: &[MethodDescriptor] = &[MethodDescriptor::new(
    &["СлучайныйПароль", "RandomPassword"],
    Arity::range(0, 2),
    random_password,
)];

static CONFIGURATION_EXTENSION_PROPERTIES: &[PropertyDescriptor] = &[
    PropertyDescriptor {
        names: &["Назначение", "Purpose"],
        get: extension_purpose,
        set: None,
    },
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

static SETTINGS_STORAGE_MANAGER_METHODS: &[MethodDescriptor] = &[MethodDescriptor::new(
    &["Загрузить", "Load"],
    Arity::range(1, 4),
    settings_storage_load,
)];

fn settings_storage_load(
    receiver: &dyn ObjectProtocol,
    arguments: &[Value],
    context: &mut CallContext<'_>,
) -> RtResult<Value> {
    let manager = receiver
        .downcast_ref::<ManagerValue>()
        .ok_or(RtError::NotAnObject)?;
    let object = manager
        .catalog
        .materialized_object_by_id(&manager.object_id)?
        .ok_or_else(|| RtError::DynamicError("объект менеджера отсутствует в каталоге".into()))?;
    let module = object
        .manager_module()
        .ok_or_else(|| RtError::DynamicError("модуль хранилища настроек не подключён".into()))?;
    let user = match arguments.get(3) {
        None | Some(Value::Undefined) => Value::Str(session(context)?.principal().name().into()),
        Some(Value::Str(name)) if name.to_string().is_empty() => {
            Value::Str(session(context)?.principal().name().into())
        }
        Some(value) => value.clone(),
    };
    // Нативный обработчик выбирается по русскому имени, затем по английскому,
    // независимо от варианта языка и порядка объявлений. Проверка метаданных
    // не запускает модуль; при отсутствии обоих имён сохраняется старая диагностика.
    let handler = if context
        .configuration_function_parameter_count(module, "ОбработкаЗагрузки")?
        .is_none()
        && context
            .configuration_function_parameter_count(module, "LoadProcessing")?
            .is_some()
    {
        "LoadProcessing"
    } else {
        "ОбработкаЗагрузки"
    };
    // Пользовательское хранилище выполняет собственный обработчик; оно не
    // подменяется стандартным хранилищем. Значение возвращается из параметра
    // `Настройки`, как в нативном замере вызовов менеджера и его модуля.
    // На 8.3.27.2342 явное исключение, деление на ноль, отсутствующее поле,
    // ошибка преобразования строки в число и выход за границы массива
    // требуют перезапуска.
    // Мост преобразует ошибку только после подготовки
    // цели; ошибки подготовки возвращаются в исходном виде. Прочие классы
    // ошибок обработчика ещё требуют отдельных нативных замеров.
    // НЕ ИЗМЕРЕНО(SETTINGSSTORAGE.LOAD_HANDLER_ERROR)
    let (_, parameters) = context.call_configuration_handler_with_mode(
        module,
        handler,
        vec![
            arguments[0].clone(),
            arguments.get(1).cloned().unwrap_or(Value::Undefined),
            Value::Undefined,
            arguments.get(2).cloned().unwrap_or(Value::Undefined),
            user,
        ],
        open_bsl::ConfigurationHandlerEntryMode::Critical("ОбработкаЗагрузки"),
    )?;
    Ok(parameters[2].clone())
}

fn manager_methods(kind: MetadataKind) -> &'static [MethodDescriptor] {
    match kind {
        MetadataKind::Enumeration => ENUMERATION_MANAGER_METHODS,
        MetadataKind::Constant => CONSTANT_MANAGER_METHODS,
        MetadataKind::Catalog => CATALOG_MANAGER_METHODS,
        MetadataKind::InformationRegister => INFORMATION_REGISTER_MANAGER_METHODS,
        MetadataKind::AccumulationRegister => ACCUMULATION_REGISTER_MANAGER_METHODS,
        MetadataKind::DataProcessor => DATA_PROCESSOR_MANAGER_METHODS,
        MetadataKind::Report => REPORT_MANAGER_METHODS,
        MetadataKind::Document => DOCUMENT_MANAGER_METHODS,
        MetadataKind::BusinessProcess => business_process_object::MANAGER_METHODS,
        MetadataKind::Task => CONFIGURATION_REFERENCE_MANAGER_METHODS,
        MetadataKind::ExchangePlan => exchange_plan::METHODS,
        MetadataKind::ChartOfAccounts => CHART_OF_ACCOUNTS_MANAGER_METHODS,
        MetadataKind::ChartOfCharacteristicTypes => characteristic_object::MANAGER_METHODS,
        MetadataKind::SettingsStorage => SETTINGS_STORAGE_MANAGER_METHODS,
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

fn metadata_collections() -> &'static [(&'static str, &'static str, MetadataKind)] {
    &[
        ("Подсистемы", "Subsystems", MetadataKind::Subsystem),
        (
            "ОбщиеРеквизиты",
            "CommonAttributes",
            MetadataKind::CommonAttribute,
        ),
        ("Перечисления", "Enums", MetadataKind::Enumeration),
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
        (
            "РегистрыНакопления",
            "AccumulationRegisters",
            MetadataKind::AccumulationRegister,
        ),
        (
            "РегистрыБухгалтерии",
            "AccountingRegisters",
            MetadataKind::AccountingRegister,
        ),
        (
            "РегистрыРасчета",
            "CalculationRegisters",
            MetadataKind::CalculationRegister,
        ),
        (
            "ПланыСчетов",
            "ChartsOfAccounts",
            MetadataKind::ChartOfAccounts,
        ),
        (
            "ПланыВидовХарактеристик",
            "ChartsOfCharacteristicTypes",
            MetadataKind::ChartOfCharacteristicTypes,
        ),
        (
            "ПланыВидовРасчета",
            "ChartsOfCalculationTypes",
            MetadataKind::ChartOfCalculationTypes,
        ),
        (
            "БизнесПроцессы",
            "BusinessProcesses",
            MetadataKind::BusinessProcess,
        ),
        ("Задачи", "Tasks", MetadataKind::Task),
        (
            "ХранилищаНастроек",
            "SettingsStorages",
            MetadataKind::SettingsStorage,
        ),
        ("Последовательности", "Sequences", MetadataKind::Sequence),
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
}

fn collection_kind(name: &str) -> Option<MetadataKind> {
    metadata_collections()
        .iter()
        .find(|(ru, en, _)| folded_eq(name, ru) || folded_eq(name, en))
        .map(|(_, _, kind)| *kind)
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

fn accumulation_registers(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    managers(ctx, MetadataKind::AccumulationRegister)
}

fn accounting_registers(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    managers(ctx, MetadataKind::AccountingRegister)
}

fn calculation_registers(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    managers(ctx, MetadataKind::CalculationRegister)
}

fn charts_of_accounts(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    managers(ctx, MetadataKind::ChartOfAccounts)
}

fn charts_of_characteristic_types(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    managers(ctx, MetadataKind::ChartOfCharacteristicTypes)
}

fn charts_of_calculation_types(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    managers(ctx, MetadataKind::ChartOfCalculationTypes)
}

fn business_processes(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    managers(ctx, MetadataKind::BusinessProcess)
}

fn tasks(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    managers(ctx, MetadataKind::Task)
}

fn settings_storages(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    managers(ctx, MetadataKind::SettingsStorage)
}

fn sequences(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    managers(ctx, MetadataKind::Sequence)
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

fn user_password_policies(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    Ok(Value::new_object(UserPasswordPoliciesManager))
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
        data_separation: RefCell::new(None),
    }))
}

fn current_user_data_separation(
    receiver: &dyn ObjectProtocol,
    context: &mut CallContext<'_>,
) -> RtResult<Value> {
    let user = receiver
        .downcast_ref::<InfoBaseUserValue>()
        .ok_or(RtError::NotAnObject)?;
    if let Some(value) = user.data_separation.borrow().as_ref() {
        return Ok(value.clone());
    }
    let graph = user.principal.data_separation.as_ref().ok_or_else(|| {
        RtError::DynamicError("разделение данных пользователя не предоставлено источником".into())
    })?;
    if graph.root_count() != 1 {
        return Err(RtError::DynamicError(
            "снимок разделения данных пользователя должен иметь один корень".into(),
        ));
    }
    let mut values = graph.materialize(context.runtime_shapes())?;
    let value = values.pop().expect("число корней проверено");
    if !matches!(&value, Value::Object(object) if matches!(object.as_ref(), BslObject::Structure(_)))
    {
        return Err(RtError::TypeError {
            expected: "Структура",
            op: "ПользовательИнформационнойБазы.РазделениеДанных",
        });
    }
    // Изменения возвращённой структуры принадлежат этому объекту пользователя;
    // следующий `ТекущийПользователь` снова материализует неизменяемый снимок.
    *user.data_separation.borrow_mut() = Some(value.clone());
    Ok(value)
}

fn current_user_uuid(receiver: &dyn ObjectProtocol, _: &mut CallContext<'_>) -> RtResult<Value> {
    let user = receiver
        .downcast_ref::<InfoBaseUserValue>()
        .ok_or(RtError::NotAnObject)?;
    Ok(Value::Object(Rc::new(BslObject::Uuid(
        open_bsl::uuid::parse(user.principal.id())?,
    ))))
}

fn current_user_name(receiver: &dyn ObjectProtocol, _: &mut CallContext<'_>) -> RtResult<Value> {
    let user = receiver
        .downcast_ref::<InfoBaseUserValue>()
        .ok_or(RtError::NotAnObject)?;
    Ok(Value::Str(user.principal.name().into()))
}

fn find_user_password_policy(
    receiver: &dyn ObjectProtocol,
    arguments: &[Value],
    context: &mut CallContext<'_>,
) -> RtResult<Value> {
    receiver
        .downcast_ref::<UserPasswordPoliciesManager>()
        .ok_or(RtError::NotAnObject)?;
    let name = match arguments.first() {
        None => None,
        Some(Value::Str(name)) => Some(name.to_string()),
        Some(_) => {
            return Err(RtError::TypeError {
                expected: "Строка",
                op: "НайтиПоИмени",
            });
        }
    };
    let (principal, data_access) = current_data_access(context)?;
    Ok(data_access
        .user_password_policy(name.as_deref(), &principal, context)?
        .map_or(Value::Undefined, |snapshot| {
            Value::new_object(UserPasswordPolicyValue { snapshot })
        }))
}

fn user_password_minimum_length(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let policy = receiver
        .downcast_ref::<UserPasswordPolicyValue>()
        .ok_or(RtError::NotAnObject)?;
    Ok(Value::number_from_i64(
        policy.snapshot.minimum_password_length().into(),
    ))
}

fn info_base_user_password_minimum_length(
    context: &mut CallContext<'_>,
    _: &[Value],
) -> RtResult<Value> {
    let (principal, data_access) = current_data_access(context)?;
    Ok(Value::number_from_i64(
        data_access
            .user_password_minimum_length(&principal, context)?
            .into(),
    ))
}

fn info_base_user_password_strength_check(
    context: &mut CallContext<'_>,
    _: &[Value],
) -> RtResult<Value> {
    let (principal, data_access) = current_data_access(context)?;
    Ok(Value::Boolean(
        data_access.user_password_strength_check(&principal, context)?,
    ))
}

fn user_password_numeric_setting(
    context: &mut CallContext<'_>,
    setting: UserPasswordNumericSetting,
) -> RtResult<Value> {
    let (principal, data_access) = current_data_access(context)?;
    Ok(Value::Number(data_access.user_password_numeric_setting(
        setting, &principal, context,
    )?))
}

fn user_password_max_effective_period(
    context: &mut CallContext<'_>,
    _: &[Value],
) -> RtResult<Value> {
    user_password_numeric_setting(context, UserPasswordNumericSetting::MaxEffectivePeriod)
}

fn user_password_min_effective_period(
    context: &mut CallContext<'_>,
    _: &[Value],
) -> RtResult<Value> {
    user_password_numeric_setting(context, UserPasswordNumericSetting::MinEffectivePeriod)
}

fn user_password_reuse_limit(context: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    user_password_numeric_setting(context, UserPasswordNumericSetting::ReuseLimit)
}

fn user_password_expiration_notification_period(
    context: &mut CallContext<'_>,
    _: &[Value],
) -> RtResult<Value> {
    user_password_numeric_setting(
        context,
        UserPasswordNumericSetting::ExpirationNotificationPeriod,
    )
}

fn set_user_password_setting(
    context: &mut CallContext<'_>,
    arguments: &[Value],
    setting: UserPasswordSetting,
) -> RtResult<Value> {
    let value = &arguments[0];
    let (principal, data_access) = current_data_access(context)?;
    data_access.set_user_password_setting(setting, value, &principal, context)?;
    Ok(Value::Undefined)
}

fn set_user_password_strength_check(
    context: &mut CallContext<'_>,
    arguments: &[Value],
) -> RtResult<Value> {
    set_user_password_setting(context, arguments, UserPasswordSetting::StrengthCheck)
}

fn set_user_password_minimum_length(
    context: &mut CallContext<'_>,
    arguments: &[Value],
) -> RtResult<Value> {
    set_user_password_setting(context, arguments, UserPasswordSetting::MinimumLength)
}

fn set_user_password_max_effective_period(
    context: &mut CallContext<'_>,
    arguments: &[Value],
) -> RtResult<Value> {
    set_user_password_setting(
        context,
        arguments,
        UserPasswordSetting::Numeric(UserPasswordNumericSetting::MaxEffectivePeriod),
    )
}

fn set_user_password_min_effective_period(
    context: &mut CallContext<'_>,
    arguments: &[Value],
) -> RtResult<Value> {
    set_user_password_setting(
        context,
        arguments,
        UserPasswordSetting::Numeric(UserPasswordNumericSetting::MinEffectivePeriod),
    )
}

fn set_user_password_reuse_limit(
    context: &mut CallContext<'_>,
    arguments: &[Value],
) -> RtResult<Value> {
    set_user_password_setting(
        context,
        arguments,
        UserPasswordSetting::Numeric(UserPasswordNumericSetting::ReuseLimit),
    )
}

fn set_user_password_expiration_notification_period(
    context: &mut CallContext<'_>,
    arguments: &[Value],
) -> RtResult<Value> {
    set_user_password_setting(
        context,
        arguments,
        UserPasswordSetting::Numeric(UserPasswordNumericSetting::ExpirationNotificationPeriod),
    )
}

fn authentication_lock_manager(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    Ok(Value::new_object(AuthenticationLockManager))
}

fn authentication_lock_settings_value(
    receiver: &dyn ObjectProtocol,
) -> RtResult<&AuthenticationLockSettingsValue> {
    receiver
        .downcast_ref::<AuthenticationLockSettingsValue>()
        .ok_or(RtError::NotAnObject)
}

fn get_authentication_lock_settings(
    receiver: &dyn ObjectProtocol,
    _: &[Value],
    context: &mut CallContext<'_>,
) -> RtResult<Value> {
    receiver
        .downcast_ref::<AuthenticationLockManager>()
        .ok_or(RtError::NotAnObject)?;
    let (principal, data_access) = current_data_access(context)?;
    let settings = data_access.authentication_lock_settings(&principal, context)?;
    Ok(Value::new_object(AuthenticationLockSettingsValue {
        lock_duration: RefCell::new(settings.lock_duration().clone()),
        max_unsuccessful_attempts_count: RefCell::new(
            settings.max_unsuccessful_attempts_count().clone(),
        ),
        user_name_addition_codes: RefCell::new(Arc::from(settings.user_name_addition_codes())),
    }))
}

fn set_authentication_lock_settings(
    receiver: &dyn ObjectProtocol,
    arguments: &[Value],
    context: &mut CallContext<'_>,
) -> RtResult<Value> {
    receiver
        .downcast_ref::<AuthenticationLockManager>()
        .ok_or(RtError::NotAnObject)?;
    let Some(settings) = arguments
        .first()
        .and_then(Value::object_ref)
        .and_then(|object| object.downcast_ref::<AuthenticationLockSettingsValue>())
    else {
        return Err(RtError::TypeError {
            expected: "НастройкиБлокировкиАутентификацииПользователейИнформационнойБазы",
            op: "УстановитьНастройки",
        });
    };
    let snapshot = AuthenticationLockSettingsSnapshot::new(
        settings.lock_duration.borrow().clone(),
        settings.max_unsuccessful_attempts_count.borrow().clone(),
        Arc::clone(&settings.user_name_addition_codes.borrow()),
    );
    let (principal, data_access) = current_data_access(context)?;
    data_access.set_authentication_lock_settings(&snapshot, &principal, context)?;
    Ok(Value::Undefined)
}

fn authentication_lock_number(value: Value, op: &'static str) -> RtResult<BslNumber> {
    match value {
        Value::Number(value) => Ok(value),
        Value::Str(value) => BslNumber::parse_canonical(&value.to_string()).map_err(RtError::Num),
        _ => Err(RtError::TypeError {
            expected: "Число",
            op,
        }),
    }
}

fn authentication_lock_string(value: Value, op: &'static str) -> RtResult<Arc<str>> {
    match value {
        Value::Str(value) => Ok(Arc::from(value.to_string())),
        Value::Number(value) => Ok(Arc::from(value.to_string())),
        _ => Err(RtError::TypeError {
            expected: "Строка",
            op,
        }),
    }
}

fn authentication_lock_duration(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    Ok(Value::Number(
        authentication_lock_settings_value(receiver)?
            .lock_duration
            .borrow()
            .clone(),
    ))
}

fn set_authentication_lock_duration(
    receiver: &dyn ObjectProtocol,
    value: Value,
    _: &mut CallContext<'_>,
) -> RtResult<()> {
    *authentication_lock_settings_value(receiver)?
        .lock_duration
        .borrow_mut() = authentication_lock_number(
        value,
        "НастройкиБлокировкиАутентификацииПользователейИнформационнойБазы.ДлительностьБлокировки",
    )?;
    Ok(())
}

fn authentication_lock_max_unsuccessful_attempts_count(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    Ok(Value::Number(
        authentication_lock_settings_value(receiver)?
            .max_unsuccessful_attempts_count
            .borrow()
            .clone(),
    ))
}

fn set_authentication_lock_max_unsuccessful_attempts_count(
    receiver: &dyn ObjectProtocol,
    value: Value,
    _: &mut CallContext<'_>,
) -> RtResult<()> {
    *authentication_lock_settings_value(receiver)?
        .max_unsuccessful_attempts_count
        .borrow_mut() = authentication_lock_number(
        value,
        "НастройкиБлокировкиАутентификацииПользователейИнформационнойБазы.МаксимальноеКоличествоНеуспешныхПопыток",
    )?;
    Ok(())
}

fn authentication_lock_user_name_addition_codes(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    Ok(Value::Str(
        authentication_lock_settings_value(receiver)?
            .user_name_addition_codes
            .borrow()
            .as_ref()
            .into(),
    ))
}

fn set_authentication_lock_user_name_addition_codes(
    receiver: &dyn ObjectProtocol,
    value: Value,
    _: &mut CallContext<'_>,
) -> RtResult<()> {
    *authentication_lock_settings_value(receiver)?
        .user_name_addition_codes
        .borrow_mut() = authentication_lock_string(
        value,
        "НастройкиБлокировкиАутентификацииПользователейИнформационнойБазы.КодыДополненияИмениПользователя",
    )?;
    Ok(())
}

fn user_name(context: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    Ok(Value::Str(session(context)?.principal().name().into()))
}

fn user_full_name(context: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    Ok(Value::Str(session(context)?.principal().full_name().into()))
}

fn standard_odata_interface_content(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    // Состав OData хранится в информационной базе и не равен всему каталогу
    // метаданных; без чтения настройки нельзя возвращать массив объектов.
    Err(RtError::DynamicError(
        "состав стандартного интерфейса OData не предоставлен этим хостом".into(),
    ))
}

fn set_standard_odata_interface_content(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    // Эта процедура меняет настройку информационной базы. Без адаптера
    // записи нельзя объявлять успех и оставлять прежний состав OData.
    Err(RtError::DynamicError(
        "запись состава стандартного интерфейса OData не предоставлена этим хостом".into(),
    ))
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
        .materialized_object_by_id(id)?
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
    if let [kind, owner_name, member] = parts.as_slice()
        && (folded_eq(kind, "Справочник")
            || folded_eq(kind, "Catalog")
            || folded_eq(kind, "ПланВидовХарактеристик")
            || folded_eq(kind, "ChartOfCharacteristicTypes"))
    {
        let owner_kind = if folded_eq(kind, "ПланВидовХарактеристик")
            || folded_eq(kind, "ChartOfCharacteristicTypes")
        {
            MetadataKind::ChartOfCharacteristicTypes
        } else {
            MetadataKind::Catalog
        };
        let catalog = Arc::clone(session(context)?.catalog());
        if let Some(owner) = catalog.object(owner_kind, owner_name)
            && owner
                .catalog_predefined_names()
                .iter()
                .any(|known| folded_eq(known, member))
        {
            return ManagerValue {
                object_id: Arc::clone(&owner.id),
                catalog,
            }
            .get_property(member, context);
        }
    }
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
            return enumeration_reference_value(
                Arc::clone(&catalog),
                &value.owner_id,
                &value.value_id,
            );
        }
    }
    Err(RtError::DynamicError(format!(
        "предопределённое значение `{path}` не найдено"
    )))
}

fn predefined_value_full_name(
    context: &mut CallContext<'_>,
    arguments: &[Value],
) -> RtResult<Value> {
    let [value] = arguments else {
        unreachable!("реестр проверяет арность функции");
    };
    if let Value::Enum(value) = value {
        // Язык имени задаёт конфигурация текущего сеанса, а не написание
        // вызова или язык интерфейса. Каталог без описания сохраняет прежний путь.
        let russian = context
            .host_data()
            .ok()
            .and_then(|data| data.downcast_ref::<SessionData>())
            .and_then(|data| data.catalog.configuration.as_ref())
            .is_some_and(|description| description.script_variant == ScriptVariant::Russian);
        let name = if russian {
            format!("{}.{}", value.kind().ru_name(), value.member_name())
        } else {
            format!("{}.{}", value.kind().english_name(), value.english_name())
        };
        return Ok(Value::Str(name.as_str().into()));
    }
    if let Some(reference) = value
        .object_ref()
        .and_then(|object| object.downcast_ref::<EnumerationReferenceValue>())
        && let Some(owner) = reference
            .catalog
            .materialized_object_by_id(&reference.object_id)?
    {
        // НЕ ИЗМЕРЕНО(PREDEFINED.FULL_NAME.CONFIG_ENUM): каталог без
        // приобретённого варианта встроенного языка сохраняет русский путь.
        // Для приобретённой конфигурации оба варианта и пустая ссылка измерены.
        let prefix = match reference
            .catalog
            .configuration
            .as_ref()
            .map(|description| description.script_variant)
        {
            Some(ScriptVariant::English) => "Enum",
            _ => "Перечисление",
        };
        let member = if reference.value_id.is_empty() {
            if prefix == "Enum" {
                "EmptyRef"
            } else {
                "ПустаяСсылка"
            }
        } else {
            reference
                .catalog
                .enumeration_values
                .iter()
                .find(|member| {
                    member.owner_id == reference.object_id && member.value_id == reference.value_id
                })
                .map(|member| member.name.as_ref())
                .ok_or_else(|| {
                    RtError::DynamicError(
                        "полное имя этого предопределённого значения недоступно".into(),
                    )
                })?
        };
        let name = format!("{prefix}.{}.{member}", owner.name);
        return Ok(Value::Str(name.as_str().into()));
    }
    Err(RtError::DynamicError(
        "полное имя этого предопределённого значения недоступно".into(),
    ))
}

fn get_choice_data(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    // Данные выбора зависят от метаданных и прав текущего пользователя сеанса.
    Err(RtError::DynamicError(
        "ПолучитьДанныеВыбора не поддерживается текущим адаптером данных".into(),
    ))
}

fn navigation_link(context: &mut CallContext<'_>, arguments: &[Value]) -> RtResult<Value> {
    let (principal, data_access) = current_data_access(context)?;
    let link = data_access.navigation_link(arguments, &principal, context)?;
    Ok(Value::Str(link.as_str().into()))
}

fn common_template(context: &mut CallContext<'_>, arguments: &[Value]) -> RtResult<Value> {
    if let Ok(session) = session(context) {
        let catalog = Arc::clone(session.catalog());
        if let Some(value) = file_source::template(&catalog, None, arguments, context)? {
            return Ok(value);
        }
    }
    // Код конфигурации может ссылаться на макет даже без его выполнения.
    // Пока ресурсов макетов нет, выдавать пустой документ было бы ошибкой.
    Err(RtError::DynamicError(
        "получение общего макета пока не поддерживается".into(),
    ))
}

fn external_data_processors(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    // Глобальное имя разрешается при сборке модуля, а загрузка внешних
    // обработок требует отдельного источника кода текущего сеанса.
    Err(RtError::DynamicError(
        "внешние обработки не подключены к текущему сеансу".into(),
    ))
}

fn external_reports(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    Err(RtError::DynamicError(
        "внешние отчёты не подключены к текущему сеансу".into(),
    ))
}

fn data_history(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    // История версий требует службы текущего сеанса, включая операции записи.
    // Подставлять пустой менеджер или доступ к SQL напрямую нельзя.
    Err(RtError::DynamicError(
        "история данных не подключена к текущему сеансу".into(),
    ))
}

fn second_authentication_factor_settings_templates(
    _: &mut CallContext<'_>,
    _: &[Value],
) -> RtResult<Value> {
    // Это платформенная служба управления настройками 2ФА, а не общий
    // модуль конфигурации. Без службы текущего пользователя нельзя
    // подставлять пустые шаблоны или имитировать успешное изменение.
    Err(RtError::DynamicError(
        "шаблоны настроек второго фактора аутентификации не подключены к текущему сеансу".into(),
    ))
}

fn functional_option(context: &mut CallContext<'_>, arguments: &[Value]) -> RtResult<Value> {
    let [Value::Str(name), tail @ ..] = arguments else {
        return Err(RtError::TypeError {
            expected: "Строка, Структура",
            op: "ПолучитьФункциональнуюОпцию",
        });
    };
    let parameters = match tail {
        [] | [Value::Undefined] => Vec::new(),
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

fn access_right_metadata(
    context: &CallContext<'_>,
    metadata: &Value,
    operation: &'static str,
) -> RtResult<(
    Arc<MetadataCatalog>,
    MetadataObject,
    Option<DocumentAttribute>,
)> {
    let session = session(context)?;
    let invalid_metadata = || RtError::TypeError {
        expected: "ОбъектМетаданных",
        op: operation,
    };
    let object = metadata.object_ref().ok_or_else(invalid_metadata)?;
    let mut attribute = None;
    let metadata = if let Some(value) = object
        .downcast_ref::<MetadataValue>()
        .filter(|value| Arc::ptr_eq(&value.catalog, session.catalog()))
    {
        session
            .catalog()
            .materialized_object_by_id(&value.object_id)?
            .cloned()
            .ok_or_else(|| {
                RtError::DynamicError("объект метаданных отсутствует в каталоге".into())
            })?
    } else if let Some(value) = object
        .downcast_ref::<metadata_attributes::Attribute>()
        .filter(|value| Arc::ptr_eq(&value.catalog, session.catalog()))
    {
        attribute = Some(value.descriptor().clone());
        session
            .catalog()
            .materialized_object_by_id(&value.owner_id)?
            .cloned()
            .ok_or_else(|| {
                RtError::DynamicError("владелец реквизита отсутствует в каталоге".into())
            })?
    } else if let Some(root) = object
        .downcast_ref::<MetadataRoot>()
        .filter(|root| Arc::ptr_eq(&root.catalog, session.catalog()))
    {
        MetadataObject::new(
            root.catalog
                .configuration_object_id
                .as_deref()
                .ok_or_else(|| {
                    RtError::DynamicError(
                        "идентификатор корневого объекта конфигурации не приобретён".into(),
                    )
                })?,
            configuration_description(root)?.name.as_ref(),
            MetadataKind::Configuration,
        )
    } else {
        return Err(invalid_metadata());
    };
    Ok((Arc::clone(session.catalog()), metadata, attribute))
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

    let metadata_value = metadata.clone();
    let (catalog, metadata, attribute) = access_right_metadata(context, metadata, "ПравоДоступа")?;

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
        Some(Value::Str(value)) => Some(Rc::from(value.to_string())),
        Some(_) => {
            return Err(RtError::TypeError {
                expected: "Строка",
                op: "ПравоДоступа",
            });
        }
    };
    let request = AccessRightRequest {
        right: Rc::from(right),
        metadata,
        metadata_value,
        attribute,
        subject,
        standard_attribute,
    };
    // Адаптер проверяет имя, путь и применимость до обхода выдачи прав.
    // Привилегированный кадр не делает неизвестное право допустимым;
    // явно переданные пользователь и роль сохраняют собственные права.
    let (principal, data_access) = current_data_access(context)?;
    let granted = data_access.access_right(&request, &principal, context)?;
    check_standard_path_after_adapter(&request, "ПравоДоступа")?;
    Ok(Value::Boolean(granted))
}

fn check_standard_path_after_adapter(
    request: &AccessRightRequest,
    operation: &'static str,
) -> RtResult<()> {
    // Сначала адаптер сохраняет приоритет неизвестного права. Успешный
    // пользовательский адаптер всё равно не делает пустой путь допустимым.
    if request.standard_attribute() == Some("") {
        return Err(RtError::TypeError {
            expected: "непустая Строка",
            op: operation,
        });
    }
    Ok(())
}

fn verify_access_rights(context: &mut CallContext<'_>, arguments: &[Value]) -> RtResult<Value> {
    let [right, metadata, tail @ ..] = arguments else {
        return Err(RtError::DynamicError(
            "ВыполнитьПроверкуПравДоступа ожидает два или три аргумента".into(),
        ));
    };
    let Value::Str(right) = right else {
        return Err(RtError::TypeError {
            expected: "Строка",
            op: "ВыполнитьПроверкуПравДоступа",
        });
    };
    if right.to_string().is_empty() {
        return Err(RtError::DynamicError("имя права пусто".into()));
    }
    let metadata_value = metadata.clone();
    let (_, metadata, attribute) =
        access_right_metadata(context, metadata, "ВыполнитьПроверкуПравДоступа")?;
    let standard_attribute = match tail.first() {
        None | Some(Value::Undefined) => None,
        Some(Value::Str(value)) => Some(Rc::from(value.to_string())),
        Some(_) => {
            return Err(RtError::TypeError {
                expected: "Строка",
                op: "ВыполнитьПроверкуПравДоступа",
            });
        }
    };
    let request = AccessRightRequest {
        right: Rc::from(right.to_string()),
        metadata,
        metadata_value,
        attribute,
        subject: AccessRightSubject::CurrentPrincipal,
        standard_attribute,
    };
    let (principal, data_access) = current_data_access(context)?;
    data_access.verify_access_rights(&request, &principal, context)?;
    check_standard_path_after_adapter(&request, "ВыполнитьПроверкуПравДоступа")?;
    Ok(Value::Undefined)
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
        code: ConstructorCode::new(28),
        names: &[
            "МоментВремениСУточнениемПериода",
            "PointInTimeWithPeriodAdjustment",
        ],
        arity: Arity::range(1, 3),
        call: Some(new_adjusted_point_in_time),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(27),
        names: &[
            "НастройкаВторогоФактораАутентификации",
            "SecondAuthenticationFactorSetting",
        ],
        arity: Arity::exact(0),
        call: Some(second_factor_setting::construct),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(26),
        names: &[
            "ОписаниеОбновленияКонфигурации",
            "ConfigurationUpdateDescription",
        ],
        arity: Arity::exact(1),
        call: Some(new_configuration_update_description),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(25),
        names: &[
            "ОбработкаРасшифровкиКомпоновкиДанных",
            "DataCompositionDetailsProcess",
        ],
        arity: Arity::exact(2),
        call: Some(new_data_composition_details_process),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(24),
        names: &["ПостроительЗапроса", "QueryBuilder"],
        arity: Arity::range(0, 1),
        call: Some(new_query_builder),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(23),
        names: &[
            "ОбъектМетаданныхКонфигурация",
            "ConfigurationMetadataObject",
        ],
        arity: Arity::exact(1),
        call: Some(new_configuration_metadata_object),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(22),
        names: &["УдалениеОбъекта", "ObjectDeletion"],
        arity: Arity::exact(1),
        call: Some(new_object_deletion),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(21),
        names: &["ВыражениеСхемыЗапроса", "QuerySchemaExpression"],
        arity: Arity::exact(1),
        call: Some(new_query_schema_expression),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(20),
        names: &[
            "ПользовательскиеНастройкиКомпоновкиДанных",
            "DataCompositionUserSettings",
        ],
        arity: Arity::exact(0),
        call: Some(new_data_composition_user_settings),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(19),
        names: &["Граница", "Boundary"],
        arity: Arity::range(1, 2),
        call: Some(query_boundary::construct),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(18),
        names: &[
            "ПроцессорВыводаРезультатаКомпоновкиДанныхВТабличныйДокумент",
            "DataCompositionResultSpreadsheetDocumentOutputProcessor",
        ],
        arity: Arity::exact(0),
        call: Some(new_data_composition_spreadsheet_output),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(17),
        names: &["СхемаКомпоновкиДанных", "DataCompositionSchema"],
        arity: Arity::exact(0),
        call: Some(new_data_composition_schema),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(16),
        names: &["СтандартныйПериод", "StandardPeriod"],
        arity: Arity::range(0, 2),
        call: Some(new_standard_period),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(15),
        names: &["ОписаниеКонфигурации", "ConfigurationDescription"],
        arity: Arity::exact(1),
        call: Some(new_configuration_description),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(14),
        names: &["ОписаниеИсточникаДанных", "DataSourceDescription"],
        arity: Arity::exact(1),
        call: Some(new_data_source_description),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(13),
        names: &["ПостроительОтчета", "ReportBuilder"],
        arity: Arity::range(0, 1),
        call: Some(new_report_builder),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(1),
        names: &["Запрос", "Query"],
        arity: Arity::range(0, 1),
        call: Some(new_query),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(2),
        names: &["СжатиеДанных", "Deflation"],
        arity: Arity::range(0, 1),
        call: Some(new_deflation),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(3),
        names: &["ХранилищеЗначения", "ValueStorage"],
        arity: Arity::range(1, 2),
        call: Some(new_value_storage),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(4),
        names: &["БлокировкаДанных", "DataLock"],
        arity: Arity::exact(0),
        call: Some(new_data_lock),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(5),
        names: &["МенеджерВременныхТаблиц", "TempTablesManager"],
        arity: Arity::exact(0),
        call: Some(new_temp_tables_manager),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(6),
        names: &["СхемаЗапроса", "QuerySchema"],
        arity: Arity::exact(0),
        call: Some(new_query_schema),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(7),
        names: &["ГенераторСлучайныхПаролей", "RandomPasswordGenerator"],
        arity: Arity::exact(0),
        call: Some(new_random_password_generator),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(8),
        names: &[
            "КомпоновщикМакетаКомпоновкиДанных",
            "DataCompositionTemplateComposer",
        ],
        arity: Arity::exact(0),
        call: Some(new_data_composition_template_composer),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(9),
        names: &["ПроцессорКомпоновкиДанных", "DataCompositionProcessor"],
        arity: Arity::exact(0),
        call: Some(new_data_composition_processor),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(10),
        names: &[
            "ПроцессорВыводаРезультатаКомпоновкиДанныхВКоллекциюЗначений",
            "DataCompositionResultValueCollectionOutputProcessor",
        ],
        arity: Arity::exact(0),
        call: Some(new_data_composition_value_collection_output_processor),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(11),
        names: &[
            "КомпоновщикНастроекКомпоновкиДанных",
            "DataCompositionSettingsComposer",
        ],
        arity: Arity::exact(0),
        call: Some(new_data_composition_settings_composer),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(12),
        names: &[
            "ИсточникДоступныхНастроекКомпоновкиДанных",
            "DataCompositionAvailableSettingsSource",
        ],
        arity: Arity::range(1, 2),
        call: Some(new_data_composition_available_settings_source),
    },
];

const CONSTRUCTOR_TYPES: &[(ConstructorCode, &TypeDescriptor)] = &[
    (ConstructorCode::new(19), &query_boundary::TYPE),
    (ConstructorCode::new(28), &ADJUSTED_POINT_IN_TIME_TYPE),
    (ConstructorCode::new(27), &second_factor_setting::TYPE),
    (ConstructorCode::new(1), &QUERY_TYPE),
    (ConstructorCode::new(2), &DEFLATION_TYPE),
    (ConstructorCode::new(3), &VALUE_STORAGE_TYPE),
    (ConstructorCode::new(4), &DATA_LOCK_TYPE),
    (ConstructorCode::new(5), &TEMP_TABLES_MANAGER_TYPE),
    (ConstructorCode::new(6), &QUERY_SCHEMA_TYPE),
    (ConstructorCode::new(7), &RANDOM_PASSWORD_GENERATOR_TYPE),
    (
        ConstructorCode::new(8),
        &DATA_COMPOSITION_TEMPLATE_COMPOSER_TYPE,
    ),
    (ConstructorCode::new(9), &DATA_COMPOSITION_PROCESSOR_TYPE),
    (
        ConstructorCode::new(10),
        &DATA_COMPOSITION_VALUE_COLLECTION_OUTPUT_PROCESSOR_TYPE,
    ),
    (
        ConstructorCode::new(11),
        &DATA_COMPOSITION_SETTINGS_COMPOSER_TYPE,
    ),
    (
        ConstructorCode::new(12),
        &DATA_COMPOSITION_AVAILABLE_SETTINGS_SOURCE_TYPE,
    ),
    (
        ConstructorCode::new(25),
        &DATA_COMPOSITION_DETAILS_PROCESS_TYPE,
    ),
    (
        ConstructorCode::new(26),
        &CONFIGURATION_UPDATE_DESCRIPTION_TYPE,
    ),
];

const FUNCTIONS: &[FunctionDescriptor] = &[
    FunctionDescriptor {
        code: FunctionCode::new(74),
        names: &["ПолучитьКритерииОтбораORM"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(filter_criteria),
    },
    FunctionDescriptor {
        code: FunctionCode::new(73),
        names: &["WSСсылки", "WSReferences"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(ws_references),
    },
    FunctionDescriptor {
        code: FunctionCode::new(72),
        names: &["ПолучитьРазмерДанныхБазыДанных", "GetDatabaseDataSize"],
        arity: Arity::range(0, 3),
        kind: FunctionKind::Function,
        call: Some(get_database_data_size),
    },
    FunctionDescriptor {
        code: FunctionCode::new(71),
        names: &["УдалитьОбъекты", "DeleteObjects"],
        arity: Arity::range(1, 6),
        kind: FunctionKind::Procedure,
        call: Some(delete_objects),
    },
    FunctionDescriptor {
        code: FunctionCode::new(69),
        names: &["ПолучитьДанныеВыбора", "GetChoiceData"],
        arity: Arity::exact(2),
        kind: FunctionKind::Function,
        call: Some(get_choice_data),
    },
    FunctionDescriptor {
        code: FunctionCode::new(68),
        names: &[
            "УстановитьСоставСтандартногоИнтерфейсаOData",
            "SetStandardODataInterfaceContent",
        ],
        arity: Arity::exact(1),
        kind: FunctionKind::Procedure,
        call: Some(set_standard_odata_interface_content),
    },
    FunctionDescriptor {
        code: FunctionCode::new(67),
        names: &[
            "ПолучитьСоставСтандартногоИнтерфейсаOData",
            "GetStandardODataInterfaceContent",
        ],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(standard_odata_interface_content),
    },
    FunctionDescriptor {
        code: FunctionCode::new(66),
        names: &["ПолноеИмяПользователя", "UserFullName"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(user_full_name),
    },
    FunctionDescriptor {
        code: FunctionCode::new(64),
        names: &["УдалитьДанныеИнформационнойБазы", "EraseInfoBaseData"],
        arity: Arity::exact(0),
        kind: FunctionKind::Procedure,
        call: Some(erase_infobase_data),
    },
    FunctionDescriptor {
        code: FunctionCode::new(63),
        names: &["РазблокироватьДанныеДляРедактирования", "UnlockDataForEdit"],
        arity: Arity::range(0, 2),
        kind: FunctionKind::Procedure,
        call: Some(unlock_data_for_edit),
    },
    FunctionDescriptor {
        code: FunctionCode::new(62),
        names: &["ЗаблокироватьДанныеДляРедактирования", "LockDataForEdit"],
        arity: Arity::range(1, 3),
        kind: FunctionKind::Procedure,
        call: Some(lock_data_for_edit),
    },
    FunctionDescriptor {
        code: FunctionCode::new(1),
        names: &["ПолучитьМетаданныеORM"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(metadata),
    },
    FunctionDescriptor {
        code: FunctionCode::new(2),
        names: &["ПолучитьСправочникиORM"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(catalogs),
    },
    FunctionDescriptor {
        code: FunctionCode::new(3),
        names: &["ПолучитьРегистрыСведенийORM"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(information_registers),
    },
    FunctionDescriptor {
        code: FunctionCode::new(4),
        names: &["ПолучитьКонстантыORM"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(constants),
    },
    FunctionDescriptor {
        code: FunctionCode::new(5),
        names: &["РольДоступна", "IsInRole"],
        arity: Arity::exact(1),
        kind: FunctionKind::Function,
        call: Some(role_available),
    },
    FunctionDescriptor {
        code: FunctionCode::new(6),
        names: &["ПолучитьРасширенияКонфигурацииORM"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(configuration_extensions),
    },
    FunctionDescriptor {
        code: FunctionCode::new(7),
        names: &["ПолучитьПользователейИнформационнойБазыORM"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(info_base_users),
    },
    FunctionDescriptor {
        code: FunctionCode::new(8),
        names: &["ПолучитьПеречисленияORM"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(enumerations),
    },
    FunctionDescriptor {
        code: FunctionCode::new(9),
        names: &["ПолучитьБиблиотекуКартинокORM"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(picture_library),
    },
    FunctionDescriptor {
        code: FunctionCode::new(10),
        names: &["КонфигурацияИзменена", "ConfigurationChanged"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(configuration_changed),
    },
    FunctionDescriptor {
        code: FunctionCode::new(11),
        names: &["ПолучитьПланыОбменаORM"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(exchange_plans),
    },
    FunctionDescriptor {
        code: FunctionCode::new(12),
        names: &["НачатьТранзакцию", "BeginTransaction"],
        arity: Arity::exact(0),
        kind: FunctionKind::Procedure,
        call: Some(begin_transaction),
    },
    FunctionDescriptor {
        code: FunctionCode::new(13),
        names: &["ЗафиксироватьТранзакцию", "CommitTransaction"],
        arity: Arity::exact(0),
        kind: FunctionKind::Procedure,
        call: Some(commit_transaction),
    },
    FunctionDescriptor {
        code: FunctionCode::new(14),
        names: &["ОтменитьТранзакцию", "RollbackTransaction"],
        arity: Arity::exact(0),
        kind: FunctionKind::Procedure,
        call: Some(rollback_transaction),
    },
    FunctionDescriptor {
        code: FunctionCode::new(15),
        names: &["ПравоДоступа", "AccessRight"],
        arity: Arity::range(2, 4),
        kind: FunctionKind::Function,
        call: Some(access_right),
    },
    FunctionDescriptor {
        code: FunctionCode::new(16),
        names: &["ИмяПользователя", "UserName"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(user_name),
    },
    FunctionDescriptor {
        code: FunctionCode::new(17),
        names: &["ПредопределенноеЗначение", "PredefinedValue"],
        arity: Arity::exact(1),
        kind: FunctionKind::Function,
        call: Some(predefined_value),
    },
    FunctionDescriptor {
        code: FunctionCode::new(18),
        names: &["МонопольныйРежим", "ExclusiveMode"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(exclusive_mode),
    },
    FunctionDescriptor {
        code: FunctionCode::new(19),
        names: &["УстановитьМонопольныйРежим", "SetExclusiveMode"],
        arity: Arity::exact(1),
        kind: FunctionKind::Procedure,
        call: Some(set_exclusive_mode),
    },
    FunctionDescriptor {
        code: FunctionCode::new(20),
        names: &["ПолучитьСервисыИнтеграцииORM"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(integration_services),
    },
    FunctionDescriptor {
        code: FunctionCode::new(21),
        names: &[
            "ПолучитьОбновлениеПредопределенныхДанныхИнформационнойБазы",
            "GetInfoBasePredefinedData",
        ],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(info_base_predefined_data_update),
    },
    FunctionDescriptor {
        code: FunctionCode::new(22),
        names: &[
            "УстановитьОбновлениеПредопределенныхДанныхИнформационнойБазы",
            "SetInfoBasePredefinedDataUpdate",
        ],
        arity: Arity::exact(1),
        kind: FunctionKind::Procedure,
        call: Some(set_info_base_predefined_data_update),
    },
    FunctionDescriptor {
        code: FunctionCode::new(23),
        names: &[
            "ИнициализироватьПредопределенныеДанные",
            "InitializePredefinedData",
        ],
        arity: Arity::exact(0),
        kind: FunctionKind::Procedure,
        call: Some(initialize_predefined_data),
    },
    FunctionDescriptor {
        code: FunctionCode::new(24),
        names: &["ПолучитьФункциональнуюОпцию", "GetFunctionalOption"],
        arity: Arity::range(1, 2),
        kind: FunctionKind::Function,
        call: Some(functional_option),
    },
    FunctionDescriptor {
        code: FunctionCode::new(25),
        names: &["ТранзакцияАктивна", "TransactionActive"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(transaction_active),
    },
    FunctionDescriptor {
        code: FunctionCode::new(26),
        names: &["ТекущийЯзык", "CurrentLanguage"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(current_language),
    },
    FunctionDescriptor {
        code: FunctionCode::new(27),
        names: &["НайтиПоСсылкам", "FindByRef"],
        arity: Arity::range(1, 4),
        kind: FunctionKind::Function,
        call: Some(find_by_ref),
    },
    FunctionDescriptor {
        code: FunctionCode::new(28),
        names: &["ПолучитьДокументыORM"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(documents),
    },
    FunctionDescriptor {
        code: FunctionCode::new(29),
        names: &["ПолучитьЖурналыДокументовORM"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(document_journals),
    },
    FunctionDescriptor {
        code: FunctionCode::new(30),
        names: &["ПолучитьОтчетыORM"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(reports),
    },
    FunctionDescriptor {
        code: FunctionCode::new(31),
        names: &["ПолучитьОбработкиORM"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(data_processors),
    },
    FunctionDescriptor {
        code: FunctionCode::new(32),
        names: &["ПолучитьРегистрыНакопленияORM"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(accumulation_registers),
    },
    FunctionDescriptor {
        code: FunctionCode::new(33),
        names: &["ПолучитьРегистрыБухгалтерииORM"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(accounting_registers),
    },
    FunctionDescriptor {
        code: FunctionCode::new(34),
        names: &["ПолучитьРегистрыРасчетаORM"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(calculation_registers),
    },
    FunctionDescriptor {
        code: FunctionCode::new(35),
        names: &["ПолучитьПланыСчетовORM"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(charts_of_accounts),
    },
    FunctionDescriptor {
        code: FunctionCode::new(36),
        names: &["ПолучитьПланыВидовХарактеристикORM"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(charts_of_characteristic_types),
    },
    FunctionDescriptor {
        code: FunctionCode::new(37),
        names: &["ПолучитьПланыВидовРасчетаORM"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(charts_of_calculation_types),
    },
    FunctionDescriptor {
        code: FunctionCode::new(38),
        names: &["ПолучитьБизнесПроцессыORM"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(business_processes),
    },
    FunctionDescriptor {
        code: FunctionCode::new(39),
        names: &["ПолучитьЗадачиORM"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(tasks),
    },
    FunctionDescriptor {
        code: FunctionCode::new(40),
        names: &["ПолучитьХранилищаНастроекORM"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(settings_storages),
    },
    FunctionDescriptor {
        code: FunctionCode::new(41),
        names: &["ПолучитьПоследовательностиORM"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(sequences),
    },
    FunctionDescriptor {
        code: FunctionCode::new(42),
        names: &[
            "ПолучитьПолноеИмяПредопределенногоЗначения",
            "GetPredefinedValueFullName",
        ],
        arity: Arity::exact(1),
        kind: FunctionKind::Function,
        call: Some(predefined_value_full_name),
    },
    FunctionDescriptor {
        code: FunctionCode::new(43),
        names: &["ПолучитьНавигационнуюСсылку", "GetURL"],
        arity: Arity::range(1, 4),
        kind: FunctionKind::Function,
        call: Some(navigation_link),
    },
    FunctionDescriptor {
        code: FunctionCode::new(44),
        names: &["ПолучитьПолитикиПаролейПользователейORM"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(user_password_policies),
    },
    FunctionDescriptor {
        code: FunctionCode::new(45),
        names: &[
            "ПолучитьМинимальнуюДлинуПаролейПользователей",
            "GetUserPasswordMinLength",
        ],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(info_base_user_password_minimum_length),
    },
    FunctionDescriptor {
        code: FunctionCode::new(46),
        names: &["ПолучитьБлокировкуАутентификацииORM"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(authentication_lock_manager),
    },
    FunctionDescriptor {
        code: FunctionCode::new(47),
        names: &[
            "ПолучитьПроверкуСложностиПаролейПользователей",
            "GetUserPasswordStrengthCheck",
        ],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(info_base_user_password_strength_check),
    },
    FunctionDescriptor {
        code: FunctionCode::new(48),
        names: &[
            "ПолучитьМаксимальныйСрокДействияПаролейПользователей",
            "GetUserPasswordMaxEffectivePeriod",
        ],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(user_password_max_effective_period),
    },
    FunctionDescriptor {
        code: FunctionCode::new(49),
        names: &[
            "ПолучитьМинимальныйСрокДействияПаролейПользователей",
            "GetUserPasswordMinEffectivePeriod",
        ],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(user_password_min_effective_period),
    },
    FunctionDescriptor {
        code: FunctionCode::new(50),
        names: &[
            "ПолучитьОграничениеПовторенияПаролейПользователейСредиПоследних",
            "GetUserPasswordReuseLimit",
        ],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(user_password_reuse_limit),
    },
    FunctionDescriptor {
        code: FunctionCode::new(51),
        names: &[
            "ПолучитьСрокПредупрежденияОбИстеченииСрокаДействияПаролейПользователей",
            "GetUserPasswordExpirationNotificationPeriod",
        ],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(user_password_expiration_notification_period),
    },
    FunctionDescriptor {
        code: FunctionCode::new(52),
        names: &[
            "УстановитьПроверкуСложностиПаролейПользователей",
            "SetUserPasswordStrengthCheck",
        ],
        arity: Arity::exact(1),
        kind: FunctionKind::Procedure,
        call: Some(set_user_password_strength_check),
    },
    FunctionDescriptor {
        code: FunctionCode::new(53),
        names: &[
            "УстановитьМинимальнуюДлинуПаролейПользователей",
            "SetUserPasswordMinLength",
        ],
        arity: Arity::exact(1),
        kind: FunctionKind::Procedure,
        call: Some(set_user_password_minimum_length),
    },
    FunctionDescriptor {
        code: FunctionCode::new(54),
        names: &[
            "УстановитьМаксимальныйСрокДействияПаролейПользователей",
            "SetUserPasswordMaxEffectivePeriod",
        ],
        arity: Arity::exact(1),
        kind: FunctionKind::Procedure,
        call: Some(set_user_password_max_effective_period),
    },
    FunctionDescriptor {
        code: FunctionCode::new(55),
        names: &[
            "УстановитьМинимальныйСрокДействияПаролейПользователей",
            "SetUserPasswordMinEffectivePeriod",
        ],
        arity: Arity::exact(1),
        kind: FunctionKind::Procedure,
        call: Some(set_user_password_min_effective_period),
    },
    FunctionDescriptor {
        code: FunctionCode::new(56),
        names: &[
            "УстановитьОграничениеПовторенияПаролейПользователейСредиПоследних",
            "SetUserPasswordReuseLimit",
        ],
        arity: Arity::exact(1),
        kind: FunctionKind::Procedure,
        call: Some(set_user_password_reuse_limit),
    },
    FunctionDescriptor {
        code: FunctionCode::new(57),
        names: &[
            "УстановитьСрокПредупрежденияОбИстеченииСрокаДействияПаролейПользователей",
            "SetUserPasswordExpirationNotificationPeriod",
        ],
        arity: Arity::exact(1),
        kind: FunctionKind::Procedure,
        call: Some(set_user_password_expiration_notification_period),
    },
    FunctionDescriptor {
        code: FunctionCode::new(58),
        names: &["ПолучитьОбщийМакет", "GetCommonTemplate"],
        arity: Arity::exact(1),
        kind: FunctionKind::Function,
        call: Some(common_template),
    },
    FunctionDescriptor {
        code: FunctionCode::new(59),
        names: &["ВыполнитьПроверкуПравДоступа", "VerifyAccessRights"],
        arity: Arity::range(2, 3),
        kind: FunctionKind::Procedure,
        call: Some(verify_access_rights),
    },
    FunctionDescriptor {
        code: FunctionCode::new(60),
        names: &["ПолучитьВнешниеОбработкиORM"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(external_data_processors),
    },
    FunctionDescriptor {
        code: FunctionCode::new(61),
        names: &["ПолучитьВнешниеОтчетыORM"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(external_reports),
    },
    FunctionDescriptor {
        code: FunctionCode::new(65),
        names: &["ПолучитьИсториюДанныхORM"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(data_history),
    },
    FunctionDescriptor {
        code: FunctionCode::new(70),
        names: &["ПолучитьШаблоныНастроекВторогоФактораАутентификацииORM"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(second_authentication_factor_settings_templates),
    },
];

const GLOBALS: &[GlobalDescriptor] = &[
    GlobalDescriptor {
        names: &["КритерииОтбора", "FilterCriteria"],
        function: FunctionCode::new(74),
    },
    GlobalDescriptor {
        names: &["WSСсылки", "WSReferences"],
        function: FunctionCode::new(73),
    },
    GlobalDescriptor {
        names: &[
            "ШаблоныНастроекВторогоФактораАутентификации",
            "SecondAuthenticationFactorSettingsTemplates",
        ],
        function: FunctionCode::new(70),
    },
    GlobalDescriptor {
        names: &["ВнешниеОбработки", "ExternalDataProcessors"],
        function: FunctionCode::new(60),
    },
    GlobalDescriptor {
        names: &["ВнешниеОтчеты", "ExternalReports"],
        function: FunctionCode::new(61),
    },
    GlobalDescriptor {
        names: &["ИсторияДанных", "DataHistory"],
        function: FunctionCode::new(65),
    },
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
        names: &["РегистрыНакопления", "AccumulationRegisters"],
        function: FunctionCode::new(32),
    },
    GlobalDescriptor {
        names: &["РегистрыБухгалтерии", "AccountingRegisters"],
        function: FunctionCode::new(33),
    },
    GlobalDescriptor {
        names: &["РегистрыРасчета", "CalculationRegisters"],
        function: FunctionCode::new(34),
    },
    GlobalDescriptor {
        names: &["ПланыСчетов", "ChartsOfAccounts"],
        function: FunctionCode::new(35),
    },
    GlobalDescriptor {
        names: &["ПланыВидовХарактеристик", "ChartsOfCharacteristicTypes"],
        function: FunctionCode::new(36),
    },
    GlobalDescriptor {
        names: &["ПланыВидовРасчета", "ChartsOfCalculationTypes"],
        function: FunctionCode::new(37),
    },
    GlobalDescriptor {
        names: &["БизнесПроцессы", "BusinessProcesses"],
        function: FunctionCode::new(38),
    },
    GlobalDescriptor {
        names: &["Задачи", "Tasks"],
        function: FunctionCode::new(39),
    },
    GlobalDescriptor {
        names: &["ХранилищаНастроек", "SettingsStorages"],
        function: FunctionCode::new(40),
    },
    GlobalDescriptor {
        names: &["Последовательности", "Sequences"],
        function: FunctionCode::new(41),
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
        names: &["ПолитикиПаролейПользователей", "UserPasswordPolicies"],
        function: FunctionCode::new(44),
    },
    GlobalDescriptor {
        names: &["БлокировкаАутентификации", "AuthenticationLock"],
        function: FunctionCode::new(46),
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
    &query_boundary::TYPE,
    &second_factor_setting::TYPE,
    &METADATA_TYPE,
    &METADATA_COLLECTION_TYPE,
    &METADATA_VALUE_TYPE,
    &MAIN_ROLES_TYPE,
    &METADATA_OBJECT_PROPERTIES_TYPE,
    &COMPATIBILITY_MODE_PROPERTY_TYPE,
    &MANAGERS_TYPE,
    &MANAGER_TYPE,
    &ENUMERATION_REFERENCE_TYPE,
    &route_points::COLLECTION_TYPE,
    &route_points::REFERENCE_TYPE,
    &CONFIGURATION_REFERENCE_TYPE,
    &catalog_object::TYPE,
    &characteristic_object::TYPE,
    &document_object::TYPE,
    &business_process_object::TYPE,
    &data_processor_object::TYPE,
    &data_processor_object::REPORT_TYPE,
    &document_movements::TYPE,
    &document_movements::RECORD_SET_TYPE,
    &document_tabular_section::TYPE,
    &document_tabular_section::ROW_TYPE,
    &catalog_object::EXCHANGE_TYPE,
    &catalog_object::RECIPIENTS_TYPE,
    &PICTURE_LIBRARY_TYPE,
    &CONFIGURATION_EXTENSIONS_MANAGER_TYPE,
    &INFO_BASE_USERS_MANAGER_TYPE,
    &INFO_BASE_USER_TYPE,
    &USER_PASSWORD_POLICIES_MANAGER_TYPE,
    &USER_PASSWORD_POLICY_TYPE,
    &AUTHENTICATION_LOCK_MANAGER_TYPE,
    &AUTHENTICATION_LOCK_SETTINGS_TYPE,
    &RANDOM_PASSWORD_GENERATOR_TYPE,
    &INTEGRATION_SERVICES_MANAGER_TYPE,
    &CONFIGURATION_EXTENSION_TYPE,
    &EXTENSION_APPLICATION_ISSUE_TYPE,
    &QUERY_TYPE,
    &QUERY_SCHEMA_TYPE,
    &DATA_COMPOSITION_SETTINGS_COMPOSER_TYPE,
    &DATA_COMPOSITION_AVAILABLE_SETTINGS_SOURCE_TYPE,
    &DATA_COMPOSITION_DETAILS_PROCESS_TYPE,
    &CONFIGURATION_UPDATE_DESCRIPTION_TYPE,
    &DATA_COMPOSITION_TEMPLATE_COMPOSER_TYPE,
    &DATA_COMPOSITION_PROCESSOR_TYPE,
    &DATA_COMPOSITION_VALUE_COLLECTION_OUTPUT_PROCESSOR_TYPE,
    &TEMP_TABLES_MANAGER_TYPE,
    &QUERY_RESULT_TYPE,
    &QUERY_SELECTION_TYPE,
    &catalog_selection::TYPE,
    &DEFLATION_TYPE,
    &VALUE_STORAGE_TYPE,
    &DATA_LOCK_TYPE,
    &DATA_LOCK_ITEM_TYPE,
    &common_attribute_content::COLLECTION_TYPE,
    &common_attribute_content::ITEM_TYPE,
    &ADJUSTED_POINT_IN_TIME_TYPE,
];

const API_MEMBERS: &[ObjectMembersDescriptor] = &[
    ObjectMembersDescriptor::new(&query_boundary::TYPE).with_properties(query_boundary::PROPERTIES),
    ObjectMembersDescriptor::new(&common_attribute_content::COLLECTION_TYPE)
        .with_methods(common_attribute_content::METHODS),
    ObjectMembersDescriptor::new(&common_attribute_content::ITEM_TYPE)
        .with_properties(common_attribute_content::ITEM_PROPERTIES),
    ObjectMembersDescriptor::new(&second_factor_setting::TYPE)
        .with_properties(second_factor_setting::PROPERTIES),
    ObjectMembersDescriptor::new(&METADATA_TYPE)
        .with_methods(metadata_find_by_type::METHODS)
        .with_properties(METADATA_ROOT_PROPERTIES)
        .with_dynamic_properties(),
    ObjectMembersDescriptor::new(&METADATA_COLLECTION_TYPE)
        .with_methods(METADATA_COLLECTION_METHODS)
        .with_dynamic_properties(),
    ObjectMembersDescriptor::new(&METADATA_VALUE_TYPE)
        .with_methods(METADATA_VALUE_METHODS)
        .with_properties(METADATA_VALUE_PROPERTIES)
        .with_dynamic_properties(),
    ObjectMembersDescriptor::new(&MAIN_ROLES_TYPE).with_methods(MAIN_ROLES_METHODS),
    ObjectMembersDescriptor::new(&METADATA_OBJECT_PROPERTIES_TYPE)
        .with_properties(METADATA_OBJECT_PROPERTIES),
    ObjectMembersDescriptor::new(&COMPATIBILITY_MODE_PROPERTY_TYPE),
    ObjectMembersDescriptor::new(&MANAGERS_TYPE).with_dynamic_properties(),
    ObjectMembersDescriptor::new(&MANAGER_TYPE)
        .with_methods(MANAGER_PUBLIC_METHODS)
        .with_properties(BUSINESS_PROCESS_MANAGER_PROPERTIES),
    ObjectMembersDescriptor::new(&ENUMERATION_REFERENCE_TYPE)
        .with_methods(ENUMERATION_REFERENCE_METHODS),
    ObjectMembersDescriptor::new(&route_points::COLLECTION_TYPE)
        .with_methods(route_points::METHODS)
        .with_dynamic_properties(),
    ObjectMembersDescriptor::new(&route_points::REFERENCE_TYPE),
    ObjectMembersDescriptor::new(&CONFIGURATION_REFERENCE_TYPE)
        .with_methods(CONFIGURATION_REFERENCE_METHODS),
    ObjectMembersDescriptor::new(&data_processor_object::REPORT_TYPE)
        .with_methods(data_processor_object::METHODS)
        .with_properties(data_processor_object::REPORT_PROPERTIES),
    ObjectMembersDescriptor::new(&data_processor_object::TYPE)
        .with_methods(data_processor_object::METHODS),
    ObjectMembersDescriptor::new(&business_process_object::TYPE)
        .with_methods(business_process_object::METHODS)
        .with_properties(business_process_object::PROPERTIES),
    ObjectMembersDescriptor::new(&document_object::TYPE)
        .with_methods(document_object::METHODS)
        .with_properties(document_object::PROPERTIES),
    ObjectMembersDescriptor::new(&document_tabular_section::TYPE)
        .with_methods(document_tabular_section::METHODS),
    ObjectMembersDescriptor::new(&document_movements::TYPE)
        .with_methods(document_movements::METHODS)
        .with_dynamic_properties(),
    ObjectMembersDescriptor::new(&document_movements::RECORD_SET_TYPE)
        .with_methods(document_movements::RECORD_SET_METHODS)
        .with_properties(document_movements::RECORD_SET_PROPERTIES),
    ObjectMembersDescriptor::new(&document_tabular_section::ROW_TYPE)
        .with_properties(document_tabular_section::ROW_PROPERTIES)
        .with_dynamic_properties(),
    ObjectMembersDescriptor::new(&characteristic_object::TYPE)
        .with_methods(characteristic_object::METHODS)
        .with_properties(characteristic_object::PROPERTIES),
    ObjectMembersDescriptor::new(&catalog_object::TYPE)
        .with_methods(catalog_object::METHODS)
        .with_properties(catalog_object::PROPERTIES),
    ObjectMembersDescriptor::new(&catalog_object::EXCHANGE_TYPE)
        .with_properties(catalog_object::EXCHANGE_PROPERTIES),
    ObjectMembersDescriptor::new(&catalog_object::RECIPIENTS_TYPE)
        .with_properties(catalog_object::RECIPIENTS_PROPERTIES),
    ObjectMembersDescriptor::new(&PICTURE_LIBRARY_TYPE).with_dynamic_properties(),
    ObjectMembersDescriptor::new(&CONFIGURATION_EXTENSIONS_MANAGER_TYPE)
        .with_methods(CONFIGURATION_EXTENSIONS_MANAGER_METHODS),
    ObjectMembersDescriptor::new(&INFO_BASE_USERS_MANAGER_TYPE)
        .with_methods(INFO_BASE_USERS_MANAGER_METHODS),
    ObjectMembersDescriptor::new(&INFO_BASE_USER_TYPE).with_properties(INFO_BASE_USER_PROPERTIES),
    ObjectMembersDescriptor::new(&USER_PASSWORD_POLICIES_MANAGER_TYPE)
        .with_methods(USER_PASSWORD_POLICIES_MANAGER_METHODS),
    ObjectMembersDescriptor::new(&USER_PASSWORD_POLICY_TYPE)
        .with_properties(USER_PASSWORD_POLICY_PROPERTIES),
    ObjectMembersDescriptor::new(&AUTHENTICATION_LOCK_MANAGER_TYPE)
        .with_methods(AUTHENTICATION_LOCK_MANAGER_METHODS),
    ObjectMembersDescriptor::new(&AUTHENTICATION_LOCK_SETTINGS_TYPE)
        .with_properties(AUTHENTICATION_LOCK_SETTINGS_PROPERTIES),
    ObjectMembersDescriptor::new(&RANDOM_PASSWORD_GENERATOR_TYPE)
        .with_methods(RANDOM_PASSWORD_GENERATOR_METHODS),
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
    ObjectMembersDescriptor::new(&catalog_selection::TYPE)
        .with_methods(catalog_selection::METHODS)
        .with_dynamic_properties(),
    ObjectMembersDescriptor::new(&QUERY_SELECTION_TYPE)
        .with_methods(QUERY_SELECTION_METHODS)
        .with_dynamic_properties(),
    ObjectMembersDescriptor::new(&VALUE_STORAGE_TYPE).with_methods(VALUE_STORAGE_METHODS),
    ObjectMembersDescriptor::new(&DATA_LOCK_TYPE).with_methods(DATA_LOCK_METHODS),
    ObjectMembersDescriptor::new(&DATA_LOCK_ITEM_TYPE),
    ObjectMembersDescriptor::new(&DATA_COMPOSITION_TEMPLATE_COMPOSER_TYPE)
        .with_methods(DATA_COMPOSITION_TEMPLATE_COMPOSER_METHODS),
    ObjectMembersDescriptor::new(&DATA_COMPOSITION_PROCESSOR_TYPE)
        .with_methods(DATA_COMPOSITION_PROCESSOR_METHODS),
    ObjectMembersDescriptor::new(&DATA_COMPOSITION_VALUE_COLLECTION_OUTPUT_PROCESSOR_TYPE)
        .with_methods(DATA_COMPOSITION_VALUE_COLLECTION_OUTPUT_PROCESSOR_METHODS)
        .with_properties(DATA_COMPOSITION_VALUE_COLLECTION_OUTPUT_PROCESSOR_PROPERTIES),
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

#[cfg(test)]
#[path = "exchange_plan_tests.rs"]
mod exchange_plan_tests;

#[cfg(test)]
#[path = "catalog_object_metadata_tests.rs"]
mod catalog_object_metadata_tests;

#[cfg(test)]
#[path = "document_object_metadata_tests.rs"]
mod document_object_metadata_tests;

#[cfg(test)]
#[path = "catalog_check_filling_tests.rs"]
mod catalog_check_filling_tests;

#[cfg(test)]
#[path = "characteristic_settings_tests.rs"]
mod characteristic_settings_tests;

#[cfg(test)]
#[path = "common_attribute_separation_tests.rs"]
mod common_attribute_separation_tests;

#[cfg(test)]
#[path = "common_attribute_content_tests.rs"]
mod common_attribute_content_tests;

#[cfg(test)]
#[path = "characteristic_reference_tests.rs"]
mod characteristic_reference_tests;

#[cfg(test)]
#[path = "configuration_extension_purpose_tests.rs"]
mod configuration_extension_purpose_tests;

#[cfg(test)]
#[path = "document_check_filling_message_tests.rs"]
mod document_check_filling_message_tests;

#[cfg(test)]
#[path = "business_process_check_message_tests.rs"]
mod business_process_check_message_tests;

#[cfg(test)]
#[path = "configuration_extension_hash_tests.rs"]
mod configuration_extension_hash_tests;

#[cfg(test)]
#[path = "metadata_type_names_tests.rs"]
mod metadata_type_names_tests;

#[cfg(test)]
#[path = "user_data_separation_tests.rs"]
mod user_data_separation_tests;

#[cfg(test)]
#[path = "user_uuid_tests.rs"]
mod user_uuid_tests;

#[cfg(test)]
#[path = "query_parameters_property_tests.rs"]
mod query_parameters_property_tests;

#[cfg(test)]
#[path = "enumeration_constant_tests.rs"]
mod enumeration_constant_tests;

#[cfg(test)]
#[path = "query_selection_index_tests.rs"]
mod query_selection_index_tests;

#[cfg(test)]
#[path = "metadata_find_by_type_tests.rs"]
mod metadata_find_by_type_tests;

#[cfg(test)]
#[path = "configuration_access_right_tests.rs"]
mod configuration_access_right_tests;

#[cfg(test)]
#[path = "filter_criteria_global_tests.rs"]
mod filter_criteria_global_tests;

#[cfg(test)]
#[path = "default_interface_tests.rs"]
mod default_interface_tests;

#[cfg(test)]
#[path = "query_boundary_tests.rs"]
mod query_boundary_tests;

#[cfg(test)]
#[path = "metadata_collection_index_tests.rs"]
mod metadata_collection_index_tests;

mod file_source;

#[cfg(test)]
#[path = "file_source_tests.rs"]
mod file_source_tests;
