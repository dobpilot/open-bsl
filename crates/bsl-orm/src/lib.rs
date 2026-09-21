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
    Arity, CallContext, EnumKind, EnumValue, FunctionCode, FunctionDescriptor, FunctionKind,
    GlobalDescriptor, LibraryDescriptor, MethodDescriptor, ObjectMembersDescriptor, ObjectProtocol,
    PropertyDescriptor, RtError, RtResult, TypeDescriptor, Value, folded_eq,
    get_property_from_table,
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

/// Данные ORM, принадлежащие ровно одному `open_bsl::State`.
pub struct SessionData {
    catalog: Arc<MetadataCatalog>,
    principal: Principal,
    extension: Option<Rc<dyn Any>>,
}

impl SessionData {
    /// Создаёт неизменяемые данные ORM одного сеанса.
    #[must_use]
    pub fn new(catalog: Arc<MetadataCatalog>, principal: Principal) -> Self {
        Self {
            catalog,
            principal,
            extension: None,
        }
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

    fn has_method(&self, _: &str) -> bool {
        self.catalog
            .object_by_id(&self.object_id)
            .is_some_and(|object| object.manager_module().is_some())
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
];

const TYPES: &[&TypeDescriptor] = &[
    &METADATA_TYPE,
    &METADATA_COLLECTION_TYPE,
    &METADATA_VALUE_TYPE,
    &MAIN_ROLES_TYPE,
    &METADATA_OBJECT_PROPERTIES_TYPE,
    &MANAGERS_TYPE,
    &MANAGER_TYPE,
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
    ObjectMembersDescriptor::new(&MANAGER_TYPE).with_properties(MANAGER_PROPERTIES),
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
