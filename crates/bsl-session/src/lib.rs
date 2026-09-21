//! Состояние параметров одного сеанса конфигурации 1С.
//!
//! Контейнер воспроизводит измеренную ленивую инициализацию через
//! `УстановкаПараметровСеанса`. Он принадлежит одному `open_bsl::State` и
//! обращается к модулю сеанса через фасадный мост существующей VM.

#![warn(missing_docs)]

use std::cell::RefCell;
use std::error::Error;
use std::fmt;
use std::rc::Rc;
use std::sync::Arc;

use bsl_config::{ConfigurationCodeSnapshot, ModuleKind};
use bsl_orm::{DataAccess, MetadataCatalog, MetadataKind, Principal, SessionData};
use open_bsl::{
    Arity, CallContext, Engine, Error as OpenBslError, FunctionCode, FunctionDescriptor,
    FunctionKind, GlobalDescriptor, LibraryDescriptor, MethodDescriptor, ModuleGraphRecipe,
    ModuleInstance, ModuleRecipe, ObjectMembersDescriptor, ObjectProtocol, RtError, RtResult,
    State, TypeDescriptor, Value, folded_eq,
};

#[derive(Debug, Clone)]
enum ParameterState {
    Unset,
    Initializing,
    Set(Value),
}

#[derive(Debug, Clone)]
struct ParameterSlot {
    name: String,
    state: ParameterState,
}

/// Контейнер параметров одного сеанса.
#[derive(Debug)]
pub struct SessionParameters {
    handler_module: String,
    slots: RefCell<Vec<ParameterSlot>>,
}

impl SessionParameters {
    /// Создаёт контейнер из полного списка параметров конфигурации.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку для пустого имени модуля, пустого имени параметра
    /// или двух регистронезависимо одинаковых имён.
    pub fn new(
        handler_module: impl Into<String>,
        names: impl IntoIterator<Item = String>,
    ) -> Result<Self, ParameterError> {
        let handler_module = handler_module.into();
        if handler_module.is_empty() {
            return Err(ParameterError::EmptyHandlerModule);
        }
        let mut slots = Vec::<ParameterSlot>::new();
        for name in names {
            if name.is_empty() {
                return Err(ParameterError::EmptyParameterName);
            }
            if slots.iter().any(|slot| folded_eq(&slot.name, &name)) {
                return Err(ParameterError::DuplicateParameterName(name));
            }
            slots.push(ParameterSlot {
                name,
                state: ParameterState::Unset,
            });
        }
        Ok(Self {
            handler_module,
            slots: RefCell::new(slots),
        })
    }

    /// Возвращает имя модуля обработчика конфигурации.
    #[must_use]
    pub fn handler_module(&self) -> &str {
        &self.handler_module
    }

    /// Устанавливает значение параметра без запуска обработчика.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку для имени, которого нет в конфигурации.
    pub fn set(&self, name: &str, value: Value) -> Result<(), ParameterError> {
        let mut slots = self.slots.borrow_mut();
        let slot = find_slot_mut(&mut slots, name)
            .ok_or_else(|| ParameterError::UnknownParameter(name.into()))?;
        slot.state = ParameterState::Set(value);
        Ok(())
    }

    /// Возвращает уже установленное значение без ленивой инициализации.
    /// `None` означает известный, но ещё не установленный параметр.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку для имени, которого нет в конфигурации.
    pub fn peek(&self, name: &str) -> Result<Option<Value>, ParameterError> {
        let slots = self.slots.borrow();
        let slot =
            find_slot(&slots, name).ok_or_else(|| ParameterError::UnknownParameter(name.into()))?;
        Ok(match &slot.state {
            ParameterState::Set(value) => Some(value.clone()),
            ParameterState::Unset | ParameterState::Initializing => None,
        })
    }

    /// Сбрасывает все значения в состояние `Unset`.
    pub fn clear(&self) {
        for slot in self.slots.borrow_mut().iter_mut() {
            slot.state = ParameterState::Unset;
        }
    }

    /// Создаёт независимую копию для фонового сеанса.
    /// Установленные и неустановленные значения копируются, а незавершённая
    /// синхронная инициализация превращается в `Unset`.
    #[must_use]
    pub fn copy_for_background(&self) -> Self {
        let slots = self
            .slots
            .borrow()
            .iter()
            .map(|slot| ParameterSlot {
                name: slot.name.clone(),
                state: match &slot.state {
                    ParameterState::Set(value) => ParameterState::Set(value.clone()),
                    ParameterState::Unset | ParameterState::Initializing => ParameterState::Unset,
                },
            })
            .collect();
        Self {
            handler_module: self.handler_module.clone(),
            slots: RefCell::new(slots),
        }
    }

    /// Вызывает стартовую или выборочную инициализацию параметров.
    /// `None` передаёт обработчику `Неопределено`, `Some` — массив имён.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку вызова обработчика. При выборочном вызове также
    /// проверяет существование каждого имени и то, что обработчик установил
    /// запрошенные значения.
    pub fn initialize(
        &self,
        context: &mut CallContext<'_>,
        required: Option<&[String]>,
    ) -> RtResult<()> {
        if let Some(required) = required {
            {
                let mut slots = self.slots.borrow_mut();
                for name in required {
                    let slot = find_slot_mut(&mut slots, name)
                        .ok_or_else(|| parameter_access_error(name))?;
                    match slot.state {
                        ParameterState::Unset => slot.state = ParameterState::Initializing,
                        ParameterState::Initializing => return Err(parameter_access_error(name)),
                        ParameterState::Set(_) => {}
                    }
                }
            }
        }

        let argument = required.map_or(Value::Undefined, |required| {
            Value::new_array(
                required
                    .iter()
                    .map(|name| Value::Str(name.as_str().into()))
                    .collect(),
            )
        });
        let result = context.call_configuration_module(
            &self.handler_module,
            "УстановкаПараметровСеанса",
            vec![argument],
            None,
        );

        if let Some(required) = required {
            let mut slots = self.slots.borrow_mut();
            if result.is_err() {
                for name in required {
                    if let Some(slot) = find_slot_mut(&mut slots, name)
                        && matches!(slot.state, ParameterState::Initializing)
                    {
                        slot.state = ParameterState::Unset;
                    }
                }
            } else {
                for name in required {
                    let slot = find_slot_mut(&mut slots, name)
                        .ok_or_else(|| parameter_access_error(name))?;
                    if matches!(slot.state, ParameterState::Initializing) {
                        slot.state = ParameterState::Unset;
                        return Err(parameter_access_error(name));
                    }
                }
            }
        }
        result.map(|_| ())
    }

    fn get(&self, name: &str, context: &mut CallContext<'_>) -> RtResult<Value> {
        {
            let slots = self.slots.borrow();
            let slot = find_slot(&slots, name).ok_or_else(|| parameter_access_error(name))?;
            match &slot.state {
                ParameterState::Set(value) => return Ok(value.clone()),
                ParameterState::Initializing => return Err(parameter_access_error(name)),
                ParameterState::Unset => {}
            }
        }
        self.initialize(context, Some(&[name.to_string()]))?;
        self.peek(name)
            .map_err(|_| parameter_access_error(name))?
            .ok_or_else(|| parameter_access_error(name))
    }
}

fn find_slot<'a>(slots: &'a [ParameterSlot], name: &str) -> Option<&'a ParameterSlot> {
    slots.iter().find(|slot| folded_eq(&slot.name, name))
}

fn find_slot_mut<'a>(slots: &'a mut [ParameterSlot], name: &str) -> Option<&'a mut ParameterSlot> {
    slots.iter_mut().find(|slot| folded_eq(&slot.name, name))
}

fn parameter_access_error(name: &str) -> RtError {
    RtError::DynamicError(format!("Error getting value of context attribute ({name})"))
}

/// Дополнение [`bsl_orm::SessionData`] для runtime сеанса.
#[derive(Debug)]
pub struct SessionRuntime {
    parameters: Rc<SessionParameters>,
    kind: SessionKind,
}

impl SessionRuntime {
    /// Создаёт runtime с контейнером параметров.
    #[must_use]
    pub fn new(parameters: Rc<SessionParameters>, kind: SessionKind) -> Self {
        Self { parameters, kind }
    }

    /// Возвращает контейнер параметров текущего сеанса.
    #[must_use]
    pub fn parameters(&self) -> &Rc<SessionParameters> {
        &self.parameters
    }

    /// Возвращает вид текущего сеанса.
    #[must_use]
    pub const fn kind(&self) -> SessionKind {
        self.kind
    }
}

/// Вид запуска конфигурационного сеанса.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionKind {
    /// Веб-клиент.
    WebClient,
    /// Тонкий клиент.
    ThinClient,
    /// Фоновое задание.
    BackgroundJob,
}

/// Пользователь информационной базы до выбора principal.
#[derive(Debug, Clone)]
pub struct PrincipalCandidate {
    principal: Principal,
    can_authenticate: bool,
}

impl PrincipalCandidate {
    /// Создаёт кандидата из декодированной записи `v8users`.
    #[must_use]
    pub fn new(principal: Principal, can_authenticate: bool) -> Self {
        Self {
            principal,
            can_authenticate,
        }
    }

    /// Возвращает снимок пользователя.
    #[must_use]
    pub const fn principal(&self) -> &Principal {
        &self.principal
    }

    /// Разрешён ли хотя бы один способ аутентификации.
    #[must_use]
    pub const fn can_authenticate(&self) -> bool {
        self.can_authenticate
    }
}

/// Выбирает principal по имени или идентификатору.
///
/// При пустом `v8users` возвращает платформенного пустого пользователя с
/// главными ролями конфигурации. Из непустого списка участвуют только записи
/// с хотя бы одним способом аутентификации.
///
/// # Errors
///
/// Возвращает ошибку неоднозначного автоматического выбора, неизвестного
/// пользователя, явного выбора запрещённой записи или отсутствия допустимых
/// пользователей.
pub fn select_principal(
    candidates: &[PrincipalCandidate],
    selection: Option<&str>,
    main_roles: impl IntoIterator<Item = String>,
) -> Result<Principal, PrincipalSelectionError> {
    if candidates.is_empty() {
        if let Some(selection) = selection {
            return Err(PrincipalSelectionError::Unknown(selection.into()));
        }
        return Ok(Principal::new("", "", main_roles));
    }
    if let Some(selection) = selection {
        let candidate = candidates
            .iter()
            .find(|candidate| {
                candidate.principal.id() == selection
                    || folded_eq(candidate.principal.name(), selection)
            })
            .ok_or_else(|| PrincipalSelectionError::Unknown(selection.into()))?;
        if !candidate.can_authenticate {
            return Err(PrincipalSelectionError::Forbidden(
                candidate.principal.name().into(),
            ));
        }
        return Ok(candidate.principal.clone());
    }

    let eligible = candidates
        .iter()
        .filter(|candidate| candidate.can_authenticate)
        .collect::<Vec<_>>();
    match eligible.as_slice() {
        [] => Err(PrincipalSelectionError::NoSelectableUsers),
        [candidate] => Ok(candidate.principal.clone()),
        _ => Err(PrincipalSelectionError::Ambiguous(
            eligible
                .iter()
                .map(|candidate| candidate.principal.name().to_string())
                .collect(),
        )),
    }
}

/// Ошибка выбора пользователя информационной базы.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrincipalSelectionError {
    /// Имя или идентификатор не найдены.
    Unknown(String),
    /// Запись существует, но не имеет разрешённого способа аутентификации.
    Forbidden(String),
    /// В непустом `v8users` нет допустимых пользователей.
    NoSelectableUsers,
    /// Для автоматического выбора найдено несколько пользователей.
    Ambiguous(Vec<String>),
}

impl fmt::Display for PrincipalSelectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unknown(value) => write!(formatter, "пользователь `{value}` не найден"),
            Self::Forbidden(value) => write!(
                formatter,
                "пользователю `{value}` не разрешён вход в информационную базу"
            ),
            Self::NoSelectableUsers => {
                formatter.write_str("в информационной базе нет пользователей с разрешённым входом")
            }
            Self::Ambiguous(names) => write!(
                formatter,
                "требуется выбрать пользователя: {}",
                names.join(", ")
            ),
        }
    }
}

impl Error for PrincipalSelectionError {}

/// Скомпилированная конфигурация, из которой открываются изолированные сеансы.
pub struct SessionFactory {
    engine: Engine,
    catalog: Arc<MetadataCatalog>,
    session_module: String,
    parameter_names: Arc<[String]>,
    startup: open_bsl::Module,
    kind: SessionKind,
}

impl SessionFactory {
    /// Проверяет совместимость кода и метаданных и один раз компилирует
    /// конфигурационный граф для указанного вида сеанса.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку несовпадающей идентичности, отсутствующего объекта
    /// владельца или модуля сеанса, а также ошибку сборки open-bsl.
    pub fn new(
        code: ConfigurationCodeSnapshot,
        catalog: Arc<MetadataCatalog>,
        session_module: &str,
        kind: SessionKind,
    ) -> Result<Self, SessionError> {
        if code.identity() != catalog.identity() {
            return Err(SessionError::ConfigurationIdentityMismatch {
                code: code.identity().into(),
                metadata: catalog.identity().into(),
            });
        }
        for module in code
            .modules()
            .iter()
            .filter(|module| module.kind() == ModuleKind::Manager)
        {
            let owner = module
                .owner()
                .ok_or_else(|| SessionError::ManagerWithoutOwner {
                    module: module.name().into(),
                })?;
            if catalog.object_by_id(owner).is_none() {
                return Err(SessionError::MissingManagerOwner {
                    module: module.name().into(),
                    owner: owner.into(),
                });
            }
        }
        let session_module = code
            .modules()
            .iter()
            .find(|module| {
                module.kind() == ModuleKind::Session && folded_eq(module.name(), session_module)
            })
            .map(|module| module.name().to_string())
            .ok_or_else(|| SessionError::MissingSessionModule(session_module.into()))?;

        let modules = code
            .modules()
            .iter()
            .map(|module| {
                let imports = module
                    .imports()
                    .iter()
                    .map(|import| {
                        let target = code.module(import.module()).ok_or_else(|| {
                            SessionError::MissingImportedModule {
                                module: module.name().into(),
                                imported: import.module().into(),
                            }
                        })?;
                        Ok((import.alias().to_string(), target.name().to_string()))
                    })
                    .collect::<Result<Vec<_>, SessionError>>()?;
                Ok(ModuleRecipe {
                    name: module.name().into(),
                    source: module.source().into(),
                    imports,
                })
            })
            .collect::<Result<Vec<_>, SessionError>>()?;
        let engine = Engine::builder()
            .register_library(bsl_orm::library())
            .register_library(library())
            .configuration(ModuleGraphRecipe {
                modules,
                eager_init: false,
            })
            .build()?;
        let startup_source = format!(
            "Процедура ЗапуститьСеанс() Экспорт\n{session_module}.УстановкаПараметровСеанса(Неопределено);\nКонецПроцедуры"
        );
        let startup = engine.compile_entry(&startup_source)?;
        let parameter_names = catalog
            .objects()
            .iter()
            .filter(|object| object.kind() == MetadataKind::SessionParameter)
            .map(|object| object.name().to_string())
            .collect::<Vec<_>>()
            .into();
        Ok(Self {
            engine,
            catalog,
            session_module,
            parameter_names,
            startup,
            kind,
        })
    }

    /// Открывает изолированный сеанс на уже созданном отдельном соединении.
    /// Соединение передаётся во владение и не может быть унаследовано другим
    /// сеансом случайным клонированием.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку создания контейнера, инициализации модуля или
    /// стартового `УстановкаПараметровСеанса(Неопределено)`. Частичный
    /// [`Session`] при ошибке не возвращается.
    pub fn open(
        &self,
        principal: Principal,
        connection: Box<dyn std::any::Any>,
    ) -> Result<Session, SessionError> {
        self.open_with_resources(principal, connection, None, None)
    }

    /// Открывает изолированный сеанс с адаптером данных конфигурации.
    /// Адаптер принадлежит создаваемому сеансу и вызывается объектами
    /// `bsl-orm` с его пользователем и параметрами.
    ///
    /// # Errors
    ///
    /// Те же ошибки, что у [`SessionFactory::open`].
    pub fn open_with_data_access(
        &self,
        principal: Principal,
        connection: Box<dyn std::any::Any>,
        data_access: Rc<dyn DataAccess>,
    ) -> Result<Session, SessionError> {
        self.open_with_resources(principal, connection, Some(data_access), None)
    }

    /// Открывает фоновый сеанс с копией параметров родителя.
    /// Последующие изменения контейнеров независимы.
    ///
    /// # Errors
    ///
    /// Те же ошибки, что у [`SessionFactory::open`]. Фабрика обязана быть
    /// собрана для [`SessionKind::BackgroundJob`].
    pub fn open_background(
        &self,
        principal: Principal,
        connection: Box<dyn std::any::Any>,
        parent: &Session,
    ) -> Result<Session, SessionError> {
        if self.kind != SessionKind::BackgroundJob {
            return Err(SessionError::NotBackgroundFactory);
        }
        self.open_with_resources(
            principal,
            connection,
            None,
            Some(Rc::new(parent.parameters.copy_for_background())),
        )
    }

    /// Открывает фоновый сеанс с отдельным адаптером данных и копией
    /// параметров родителя.
    ///
    /// # Errors
    ///
    /// Те же ошибки, что у [`SessionFactory::open_background`].
    pub fn open_background_with_data_access(
        &self,
        principal: Principal,
        connection: Box<dyn std::any::Any>,
        data_access: Rc<dyn DataAccess>,
        parent: &Session,
    ) -> Result<Session, SessionError> {
        if self.kind != SessionKind::BackgroundJob {
            return Err(SessionError::NotBackgroundFactory);
        }
        self.open_with_resources(
            principal,
            connection,
            Some(data_access),
            Some(Rc::new(parent.parameters.copy_for_background())),
        )
    }

    fn open_with_resources(
        &self,
        principal: Principal,
        connection: Box<dyn std::any::Any>,
        data_access: Option<Rc<dyn DataAccess>>,
        parameters: Option<Rc<SessionParameters>>,
    ) -> Result<Session, SessionError> {
        let parameters = match parameters {
            Some(parameters) => parameters,
            None => Rc::new(SessionParameters::new(
                self.session_module.clone(),
                self.parameter_names.iter().cloned(),
            )?),
        };
        let mut data =
            SessionData::new(Arc::clone(&self.catalog), principal.clone()).with_extension(Rc::new(
                SessionRuntime::new(Rc::clone(&parameters), self.kind),
            ));
        if let Some(data_access) = data_access {
            data = data.with_data_access(data_access);
        }
        let mut state = self.engine.state_builder().host_data(Rc::new(data)).build();
        let startup = state.instantiate(&self.startup)?;
        startup.call(&mut state, "ЗапуститьСеанс", Vec::new())?;
        Ok(Session {
            engine: self.engine.clone(),
            state,
            startup,
            parameters,
            principal,
            connection,
            kind: self.kind,
        })
    }
}

/// Один запущенный и изолированный сеанс конфигурации.
pub struct Session {
    engine: Engine,
    state: State,
    startup: ModuleInstance,
    parameters: Rc<SessionParameters>,
    principal: Principal,
    connection: Box<dyn std::any::Any>,
    kind: SessionKind,
}

impl Session {
    /// Компилирует и исполняет верхний уровень тестового BSL-модуля в общем
    /// каталоге и состоянии этого сеанса.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку компиляции, связывания или исполнения.
    pub fn execute(&mut self, source: &str) -> Result<(), SessionError> {
        let module = self.engine.compile_entry(source)?;
        self.state.instantiate(&module)?;
        Ok(())
    }

    /// Компилирует и создаёт вызываемый экземпляр тестового модуля в этом
    /// сеансе.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку компиляции, связывания или инициализации.
    pub fn instantiate(&mut self, source: &str) -> Result<ModuleInstance, SessionError> {
        let module = self.engine.compile_entry(source)?;
        Ok(self.state.instantiate(&module)?)
    }

    /// Вызывает экспорт созданного через [`Session::instantiate`] модуля.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку чужого экземпляра, поиска, арности или исполнения.
    pub fn call(
        &mut self,
        module: &ModuleInstance,
        name: &str,
        args: Vec<Value>,
    ) -> Result<(Value, Vec<Value>), SessionError> {
        Ok(module.call(&mut self.state, name, args)?)
    }

    /// Возвращает параметры этого сеанса.
    #[must_use]
    pub fn parameters(&self) -> &Rc<SessionParameters> {
        &self.parameters
    }

    /// Возвращает замороженный снимок пользователя.
    #[must_use]
    pub const fn principal(&self) -> &Principal {
        &self.principal
    }

    /// Возвращает вид сеанса.
    #[must_use]
    pub const fn kind(&self) -> SessionKind {
        self.kind
    }

    /// Возвращает ссылку на принадлежащее сеансу соединение указанного типа.
    #[must_use]
    pub fn connection<T: std::any::Any>(&self) -> Option<&T> {
        self.connection.downcast_ref()
    }

    /// Подтверждает, что стартовый экземпляр остаётся владельцем общего
    /// состояния модулей до конца сеанса.
    #[must_use]
    pub fn startup_instance(&self) -> &ModuleInstance {
        &self.startup
    }
}

/// Ошибка построения или запуска конфигурационного сеанса.
#[derive(Debug)]
pub enum SessionError {
    /// Код и метаданные относятся к разным конфигурациям.
    ConfigurationIdentityMismatch {
        /// Идентичность снимка кода.
        code: String,
        /// Идентичность каталога метаданных.
        metadata: String,
    },
    /// Модуль менеджера не называет владельца.
    ManagerWithoutOwner {
        /// Имя модуля менеджера.
        module: String,
    },
    /// В метаданных нет владельца модуля менеджера.
    MissingManagerOwner {
        /// Имя модуля менеджера.
        module: String,
        /// Идентификатор владельца.
        owner: String,
    },
    /// В снимке нет выбранного модуля сеанса.
    MissingSessionModule(String),
    /// Импорт ссылается на отсутствующий модуль.
    MissingImportedModule {
        /// Имя импортирующего модуля.
        module: String,
        /// Идентификатор отсутствующего модуля.
        imported: String,
    },
    /// Фабрика собрана не для фонового задания.
    NotBackgroundFactory,
    /// Ошибка объявления параметров.
    Parameters(ParameterError),
    /// Ошибка open-bsl.
    OpenBsl(OpenBslError),
}

impl fmt::Display for SessionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConfigurationIdentityMismatch { code, metadata } => write!(
                formatter,
                "код конфигурации `{code}` не совпадает с метаданными `{metadata}`"
            ),
            Self::ManagerWithoutOwner { module } => {
                write!(
                    formatter,
                    "модуль менеджера `{module}` не называет владельца"
                )
            }
            Self::MissingManagerOwner { module, owner } => write!(
                formatter,
                "владелец `{owner}` модуля менеджера `{module}` отсутствует в метаданных"
            ),
            Self::MissingSessionModule(module) => {
                write!(formatter, "модуль сеанса `{module}` не найден")
            }
            Self::MissingImportedModule { module, imported } => write!(
                formatter,
                "модуль `{module}` импортирует отсутствующий модуль `{imported}`"
            ),
            Self::NotBackgroundFactory => {
                formatter.write_str("фабрика не предназначена для фонового задания")
            }
            Self::Parameters(error) => error.fmt(formatter),
            Self::OpenBsl(error) => error.fmt(formatter),
        }
    }
}

impl Error for SessionError {}

impl From<ParameterError> for SessionError {
    fn from(error: ParameterError) -> Self {
        Self::Parameters(error)
    }
}

impl From<OpenBslError> for SessionError {
    fn from(error: OpenBslError) -> Self {
        Self::OpenBsl(error)
    }
}

/// Ошибка объявления параметров конфигурации.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParameterError {
    /// Имя модуля обработчика пусто.
    EmptyHandlerModule,
    /// Имя параметра пусто.
    EmptyParameterName,
    /// Имя параметра повторяется без учёта регистра.
    DuplicateParameterName(String),
    /// Параметр отсутствует в конфигурации.
    UnknownParameter(String),
}

impl fmt::Display for ParameterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyHandlerModule => formatter.write_str("имя модуля сеанса пусто"),
            Self::EmptyParameterName => formatter.write_str("имя параметра сеанса пусто"),
            Self::DuplicateParameterName(name) => {
                write!(formatter, "параметр сеанса `{name}` повторяется")
            }
            Self::UnknownParameter(name) => {
                write!(formatter, "параметр сеанса `{name}` не найден")
            }
        }
    }
}

impl Error for ParameterError {}

#[derive(Debug)]
struct ParametersObject {
    parameters: Rc<SessionParameters>,
}

static PARAMETERS_TYPE: TypeDescriptor = TypeDescriptor::new("bsl-session", "ПараметрыСеанса");

impl ObjectProtocol for ParametersObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &PARAMETERS_TYPE
    }

    fn get_property(&self, name: &str, context: &mut CallContext<'_>) -> RtResult<Value> {
        self.parameters.get(name, context)
    }

    fn set_property(&self, name: &str, value: Value, _: &mut CallContext<'_>) -> RtResult<()> {
        self.parameters
            .set(name, value)
            .map_err(|_| parameter_access_error(name))
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        PARAMETERS_METHODS
    }
}

fn clear_parameters(
    receiver: &dyn ObjectProtocol,
    _: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let parameters = receiver
        .downcast_ref::<ParametersObject>()
        .ok_or(RtError::NotAnObject)?;
    parameters.parameters.clear();
    Ok(Value::Undefined)
}

static PARAMETERS_METHODS: &[MethodDescriptor] =
    &[
        MethodDescriptor::new(&["Очистить", "Clear"], Arity::exact(0), clear_parameters)
            .as_procedure(),
    ];

fn runtime<'a>(ctx: &'a CallContext<'_>) -> RtResult<&'a SessionRuntime> {
    ctx.host_data()?
        .downcast_ref::<SessionData>()
        .and_then(SessionData::extension::<SessionRuntime>)
        .ok_or_else(|| RtError::DynamicError("runtime сеанса не передан".into()))
}

fn parameters(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    Ok(Value::new_object(ParametersObject {
        parameters: Rc::clone(runtime(ctx)?.parameters()),
    }))
}

const FUNCTIONS: &[FunctionDescriptor] = &[FunctionDescriptor {
    code: FunctionCode::new(1),
    names: &["ПолучитьПараметрыСеанса"],
    arity: Arity::exact(0),
    kind: FunctionKind::Function,
    call: parameters,
}];

const GLOBALS: &[GlobalDescriptor] = &[GlobalDescriptor {
    names: &["ПараметрыСеанса", "SessionParameters"],
    function: FunctionCode::new(1),
}];

const TYPES: &[&TypeDescriptor] = &[&PARAMETERS_TYPE];

const API_MEMBERS: &[ObjectMembersDescriptor] = &[ObjectMembersDescriptor::new(&PARAMETERS_TYPE)
    .with_methods(PARAMETERS_METHODS)
    .with_dynamic_properties()];

const OBJECT_MEMBER_GROUPS: &[&[ObjectMembersDescriptor]] = &[API_MEMBERS];

const LIBRARY: LibraryDescriptor = LibraryDescriptor::new("bsl-session", env!("CARGO_PKG_VERSION"))
    .with_functions(FUNCTIONS)
    .with_globals(GLOBALS)
    .with_types(TYPES)
    .with_object_member_groups(OBJECT_MEMBER_GROUPS);

/// Возвращает runtime-библиотеку параметров сеанса.
#[must_use]
pub const fn library() -> LibraryDescriptor {
    LIBRARY
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
