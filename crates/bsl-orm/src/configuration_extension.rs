//! Расширения конфигурации: неизменяемый снимок сеанса и его представление
//! через менеджер `РасширенияКонфигурации`.

use super::*;

/// Область действия расширения конфигурации.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigurationExtensionScope {
    /// Вся информационная база.
    InfoBase,
    /// Текущая область разделённых данных.
    DataSeparation,
}

impl ConfigurationExtensionScope {
    pub(crate) fn value(self) -> Value {
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

    pub(crate) fn value(&self) -> Value {
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
    pub(crate) fn value(self) -> Value {
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
    pub(crate) fn value(self) -> Value {
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

pub(crate) static CONFIGURATION_EXTENSIONS_MANAGER_TYPE: TypeDescriptor =
    TypeDescriptor::new("bsl-orm", "МенеджерРасширенийКонфигурации");

pub(crate) static CONFIGURATION_EXTENSION_TYPE: TypeDescriptor =
    TypeDescriptor::new("bsl-orm", "РасширениеКонфигурации");

pub(crate) static EXTENSION_APPLICATION_ISSUE_TYPE: TypeDescriptor = TypeDescriptor::new(
    "bsl-orm",
    "ИнформацияОПроблемеПримененияРасширенияКонфигурации",
);

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

pub(crate) static CONFIGURATION_EXTENSIONS_MANAGER_METHODS: &[MethodDescriptor] = &[
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

pub(crate) static CONFIGURATION_EXTENSION_PROPERTIES: &[PropertyDescriptor] = &[
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

pub(crate) static EXTENSION_APPLICATION_ISSUE_PROPERTIES: &[PropertyDescriptor] = &[
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

pub(crate) fn configuration_extensions(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    Ok(Value::new_object(ConfigurationExtensionsManager {
        catalog: Arc::clone(session(ctx)?.catalog()),
    }))
}
