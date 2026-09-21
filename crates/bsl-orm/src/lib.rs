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
    Arity, CallContext, FunctionCode, FunctionDescriptor, FunctionKind, GlobalDescriptor,
    LibraryDescriptor, ObjectProtocol, RtError, RtResult, TypeDescriptor, Value, folded_eq,
};

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
        }
    }

    /// Связывает объект с именованным модулем менеджера конфигурации.
    #[must_use]
    pub fn with_manager_module(mut self, module: impl Into<String>) -> Self {
        self.manager_module = Some(Arc::from(module.into()));
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
        }
        Ok(Self {
            identity: identity.into(),
            objects: objects.into(),
        })
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
struct ManagersRoot {
    catalog: Arc<MetadataCatalog>,
    kind: MetadataKind,
}

#[derive(Debug)]
struct ManagerValue {
    catalog: Arc<MetadataCatalog>,
    object_id: Arc<str>,
}

static METADATA_TYPE: TypeDescriptor = TypeDescriptor::new("bsl-orm", "МетаданныеКонфигурации");
static METADATA_COLLECTION_TYPE: TypeDescriptor =
    TypeDescriptor::new("bsl-orm", "КоллекцияОбъектовМетаданных");
static METADATA_VALUE_TYPE: TypeDescriptor =
    TypeDescriptor::new("bsl-orm", "ОбъектМетаданныхКонфигурации");
static MANAGERS_TYPE: TypeDescriptor = TypeDescriptor::new("bsl-orm", "МенеджерыОбъектов");
static MANAGER_TYPE: TypeDescriptor = TypeDescriptor::new("bsl-orm", "МенеджерОбъекта");

impl ObjectProtocol for MetadataRoot {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &METADATA_TYPE
    }

    fn get_property(&self, name: &str, _: &mut CallContext<'_>) -> RtResult<Value> {
        let kind = collection_kind(name).ok_or_else(|| RtError::UnknownProperty(name.into()))?;
        Ok(Value::new_object(MetadataCollection {
            catalog: Arc::clone(&self.catalog),
            kind,
        }))
    }
}

impl ObjectProtocol for MetadataCollection {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &METADATA_COLLECTION_TYPE
    }

    fn get_property(&self, name: &str, _: &mut CallContext<'_>) -> RtResult<Value> {
        metadata_value(&self.catalog, self.kind, name)
    }

    fn call_method(
        &self,
        name: &str,
        arguments: &[Value],
        _: &mut CallContext<'_>,
    ) -> RtResult<Value> {
        if !folded_eq(name, "Найти") && !folded_eq(name, "Find") {
            return Err(RtError::UnknownMethod {
                method: name.into(),
                receiver: METADATA_COLLECTION_TYPE.name,
            });
        }
        let [Value::Str(wanted)] = arguments else {
            return Err(RtError::TypeError {
                expected: "Строка",
                op: "Найти",
            });
        };
        let wanted = wanted.to_string();
        Ok(self
            .catalog
            .object(self.kind, &wanted)
            .map(|object| metadata_object_value(&self.catalog, object))
            .unwrap_or(Value::Undefined))
    }

    fn has_method(&self, name: &str) -> bool {
        folded_eq(name, "Найти") || folded_eq(name, "Find")
    }
}

impl ObjectProtocol for MetadataValue {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &METADATA_VALUE_TYPE
    }

    fn get_property(&self, name: &str, _: &mut CallContext<'_>) -> RtResult<Value> {
        let object = self.catalog.object_by_id(&self.object_id).ok_or_else(|| {
            RtError::DynamicError("объект метаданных отсутствует в каталоге".into())
        })?;
        if folded_eq(name, "Имя") || folded_eq(name, "Name") {
            Ok(Value::Str(object.name().into()))
        } else {
            Err(RtError::UnknownProperty(name.into()))
        }
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

    fn get_property(&self, name: &str, _: &mut CallContext<'_>) -> RtResult<Value> {
        let object = self.catalog.object_by_id(&self.object_id).ok_or_else(|| {
            RtError::DynamicError("объект менеджера отсутствует в каталоге".into())
        })?;
        if folded_eq(name, "Имя") || folded_eq(name, "Name") {
            Ok(Value::Str(object.name().into()))
        } else {
            Err(RtError::UnknownProperty(name.into()))
        }
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

fn collection_kind(name: &str) -> Option<MetadataKind> {
    [
        ("Справочники", "Catalogs", MetadataKind::Catalog),
        (
            "РегистрыСведений",
            "InformationRegisters",
            MetadataKind::InformationRegister,
        ),
        ("Константы", "Constants", MetadataKind::Constant),
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
    &MANAGERS_TYPE,
    &MANAGER_TYPE,
];

const LIBRARY: LibraryDescriptor = LibraryDescriptor::new("bsl-orm", env!("CARGO_PKG_VERSION"))
    .with_functions(FUNCTIONS)
    .with_globals(GLOBALS)
    .with_types(TYPES);

/// Возвращает runtime-библиотеку ORM для регистрации в `EngineBuilder`.
#[must_use]
pub const fn library() -> LibraryDescriptor {
    LIBRARY
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
