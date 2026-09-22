//! Состояние параметров одного сеанса конфигурации 1С.
//!
//! Контейнер воспроизводит измеренную ленивую инициализацию через
//! `УстановкаПараметровСеанса`. Он принадлежит одному `open_bsl::State` и
//! обращается к модулю сеанса через фасадный мост существующей VM.

#![warn(missing_docs)]

use std::cell::{Cell, RefCell};
use std::error::Error;
use std::fmt;
use std::rc::Rc;
use std::sync::Arc;

use bsl_config::{ConfigurationCodeSnapshot, ModuleKind};
use bsl_orm::{DataAccess, MetadataCatalog, MetadataKind, Principal, SessionData};
use open_bsl::{
    Arity, BslObject, CallContext, ConstructorCode, ConstructorDescriptor, Engine, EnumValue,
    Error as OpenBslError, FunctionCode, FunctionDescriptor, FunctionKind, GlobalDescriptor,
    LibraryDescriptor, MethodDescriptor, ModuleGraphRecipe, ModuleInstance, ModuleRecipe,
    ObjectMembersDescriptor, ObjectProtocol, PropertyDescriptor, RtError, RtResult, State,
    TypeDescriptor, Value, background_job_type_descriptor, folded_eq,
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

/// Снимок одного активного соединения информационной базы.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InfoBaseConnection {
    number: u64,
    application_name: Arc<str>,
    computer_name: Arc<str>,
}

impl InfoBaseConnection {
    /// Создаёт описание соединения из значений, полученных хостом.
    #[must_use]
    pub fn new(
        number: u64,
        application_name: impl Into<String>,
        computer_name: impl Into<String>,
    ) -> Self {
        Self {
            number,
            application_name: Arc::from(application_name.into()),
            computer_name: Arc::from(computer_name.into()),
        }
    }

    /// Возвращает номер соединения.
    #[must_use]
    pub const fn number(&self) -> u64 {
        self.number
    }

    /// Возвращает имя клиентского приложения.
    #[must_use]
    pub fn application_name(&self) -> &str {
        &self.application_name
    }

    /// Возвращает имя компьютера.
    #[must_use]
    pub fn computer_name(&self) -> &str {
        &self.computer_name
    }
}

/// Уровень события журнала регистрации.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventLogLevel {
    /// Примечание.
    Note,
    /// Информационное событие.
    Information,
    /// Предупреждение.
    Warning,
    /// Ошибка.
    Error,
}

/// Связь записи журнала регистрации с транзакцией.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventLogTransactionMode {
    /// Запись выполняется независимо от текущей транзакции.
    Independent,
    /// Запись следует результату текущей транзакции.
    Transactional,
}

/// Неизменяемая запись, передаваемая журналу текущего сеанса.
#[derive(Debug, Clone)]
pub struct EventLogEntry {
    event_name: Arc<str>,
    level: EventLogLevel,
    metadata: Value,
    data: Value,
    comment: Arc<str>,
    transaction_mode: EventLogTransactionMode,
}

impl EventLogEntry {
    /// Возвращает имя события.
    #[must_use]
    pub fn event_name(&self) -> &str {
        &self.event_name
    }

    /// Возвращает уровень события.
    #[must_use]
    pub const fn level(&self) -> EventLogLevel {
        self.level
    }

    /// Возвращает связанный объект метаданных либо `Неопределено`.
    #[must_use]
    pub const fn metadata(&self) -> &Value {
        &self.metadata
    }

    /// Возвращает связанные данные либо `Неопределено`.
    #[must_use]
    pub const fn data(&self) -> &Value {
        &self.data
    }

    /// Возвращает комментарий.
    #[must_use]
    pub fn comment(&self) -> &str {
        &self.comment
    }

    /// Возвращает режим транзакции.
    #[must_use]
    pub const fn transaction_mode(&self) -> EventLogTransactionMode {
        self.transaction_mode
    }
}

/// Приёмник событий журнала регистрации одного сеанса.
pub trait EventLogWriter: fmt::Debug {
    /// Записывает одно событие.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку хоста, проверки прав или хранения журнала.
    fn write(&self, entry: EventLogEntry) -> RtResult<()>;
}

/// Вид стандартного хранилища настроек.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientSettingsStorage {
    /// Общее хранилище настроек конфигурации.
    Common,
    /// Системное хранилище настроек платформы.
    System,
}

/// Запрос чтения значения из стандартного хранилища настроек.
#[derive(Debug, Clone)]
pub struct ClientSettingsLoadRequest {
    storage: ClientSettingsStorage,
    object_key: Arc<str>,
    settings_key: Arc<str>,
    description: Value,
    user_name: Option<Arc<str>>,
}

impl ClientSettingsLoadRequest {
    /// Возвращает выбранное хранилище.
    #[must_use]
    pub const fn storage(&self) -> ClientSettingsStorage {
        self.storage
    }

    /// Возвращает ключ объекта.
    #[must_use]
    pub fn object_key(&self) -> &str {
        &self.object_key
    }

    /// Возвращает ключ настроек.
    #[must_use]
    pub fn settings_key(&self) -> &str {
        &self.settings_key
    }

    /// Возвращает переданное значение описания настроек.
    #[must_use]
    pub const fn description(&self) -> &Value {
        &self.description
    }

    /// Возвращает явно указанное имя пользователя.
    #[must_use]
    pub fn user_name(&self) -> Option<&str> {
        self.user_name.as_deref()
    }
}

/// Запрос записи значения в стандартное хранилище настроек.
#[derive(Debug, Clone)]
pub struct ClientSettingsSaveRequest {
    storage: ClientSettingsStorage,
    object_key: Arc<str>,
    settings_key: Arc<str>,
    settings: Value,
    description: Value,
    user_name: Option<Arc<str>>,
}

impl ClientSettingsSaveRequest {
    /// Возвращает выбранное хранилище.
    #[must_use]
    pub const fn storage(&self) -> ClientSettingsStorage {
        self.storage
    }

    /// Возвращает ключ объекта.
    #[must_use]
    pub fn object_key(&self) -> &str {
        &self.object_key
    }

    /// Возвращает ключ настроек.
    #[must_use]
    pub fn settings_key(&self) -> &str {
        &self.settings_key
    }

    /// Возвращает сохраняемое значение.
    #[must_use]
    pub const fn settings(&self) -> &Value {
        &self.settings
    }

    /// Возвращает описание настроек.
    #[must_use]
    pub const fn description(&self) -> &Value {
        &self.description
    }

    /// Возвращает явно указанное имя пользователя.
    #[must_use]
    pub fn user_name(&self) -> Option<&str> {
        self.user_name.as_deref()
    }
}

/// Запрос выборки строк стандартного хранилища настроек.
#[derive(Debug, Clone)]
pub struct ClientSettingsSelectRequest {
    storage: ClientSettingsStorage,
    filter: Value,
}

impl ClientSettingsSelectRequest {
    /// Возвращает выбранное хранилище.
    #[must_use]
    pub const fn storage(&self) -> ClientSettingsStorage {
        self.storage
    }

    /// Возвращает структуру отбора либо `Неопределено`.
    #[must_use]
    pub const fn filter(&self) -> &Value {
        &self.filter
    }
}

/// Одна строка выборки стандартного хранилища настроек.
#[derive(Debug, Clone)]
pub struct ClientSettingsEntry {
    object_key: Arc<str>,
    settings_key: Arc<str>,
    user: Arc<str>,
    settings: Value,
}

impl ClientSettingsEntry {
    /// Создаёт строку из значений, полученных от провайдера.
    #[must_use]
    pub fn new(
        object_key: impl Into<String>,
        settings_key: impl Into<String>,
        user: impl Into<String>,
        settings: Value,
    ) -> Self {
        Self {
            object_key: Arc::from(object_key.into()),
            settings_key: Arc::from(settings_key.into()),
            user: Arc::from(user.into()),
            settings,
        }
    }

    /// Возвращает ключ объекта.
    #[must_use]
    pub fn object_key(&self) -> &str {
        &self.object_key
    }

    /// Возвращает ключ настроек.
    #[must_use]
    pub fn settings_key(&self) -> &str {
        &self.settings_key
    }

    /// Возвращает имя пользователя.
    #[must_use]
    pub fn user(&self) -> &str {
        &self.user
    }

    /// Возвращает сохранённое значение.
    #[must_use]
    pub const fn settings(&self) -> &Value {
        &self.settings
    }
}

/// Состав форм начальной страницы одного пользователя.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HomePageFormsData {
    left: Arc<[Arc<str>]>,
    right: Arc<[Arc<str>]>,
}

impl HomePageFormsData {
    /// Создаёт состав из полных имён форм в левой и правой колонках.
    #[must_use]
    pub fn new(
        left: impl IntoIterator<Item = String>,
        right: impl IntoIterator<Item = String>,
    ) -> Self {
        Self {
            left: left.into_iter().map(Arc::<str>::from).collect(),
            right: right.into_iter().map(Arc::<str>::from).collect(),
        }
    }

    /// Возвращает полные имена форм левой колонки.
    #[must_use]
    pub fn left(&self) -> &[Arc<str>] {
        &self.left
    }

    /// Возвращает полные имена форм правой колонки.
    #[must_use]
    pub fn right(&self) -> &[Arc<str>] {
        &self.right
    }
}

/// Провайдер клиентских настроек одного конкретного сеанса.
pub trait ClientSettingsAdapter: fmt::Debug {
    /// Загружает настройку. `None` означает отсутствие сохранённого значения.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку проверки прав, доступа или декодирования значения.
    fn load(
        &self,
        principal: &Principal,
        request: &ClientSettingsLoadRequest,
    ) -> RtResult<Option<Value>>;

    /// Сохраняет настройку.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку проверки прав, доступа или кодирования значения.
    fn save(&self, principal: &Principal, request: &ClientSettingsSaveRequest) -> RtResult<()>;

    /// Возвращает неизменяемый снимок строк, подходящих под отбор.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку проверки прав, доступа или недопустимого отбора.
    fn select(
        &self,
        principal: &Principal,
        request: &ClientSettingsSelectRequest,
    ) -> RtResult<Vec<ClientSettingsEntry>>;

    /// Читает состав форм начальной страницы текущего пользователя.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку проверки прав или доступа.
    fn home_page_forms(&self, principal: &Principal) -> RtResult<HomePageFormsData>;

    /// Сохраняет состав форм начальной страницы текущего пользователя.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку проверки прав или доступа.
    fn set_home_page_forms(&self, principal: &Principal, forms: &HomePageFormsData)
    -> RtResult<()>;
}

/// Ресурсы одного открываемого сеанса.
pub struct SessionResources {
    connection: Box<dyn std::any::Any>,
    connection_number: u64,
    data_access: Option<Rc<dyn DataAccess>>,
    connections: Option<Arc<[InfoBaseConnection]>>,
    event_log_writer: Option<Rc<dyn EventLogWriter>>,
    client_settings: Option<Rc<dyn ClientSettingsAdapter>>,
}

impl SessionResources {
    /// Создаёт набор с отдельным соединением и его номером.
    #[must_use]
    pub fn new(connection: Box<dyn std::any::Any>, connection_number: u64) -> Self {
        Self {
            connection,
            connection_number,
            data_access: None,
            connections: None,
            event_log_writer: None,
            client_settings: None,
        }
    }

    /// Подключает адаптер данных этого сеанса.
    #[must_use]
    pub fn with_data_access(mut self, data_access: Rc<dyn DataAccess>) -> Self {
        self.data_access = Some(data_access);
        self
    }

    /// Подключает полный снимок соединений, разрешённый выбранному
    /// пользователю хостом.
    #[must_use]
    pub fn with_connections(
        mut self,
        connections: impl IntoIterator<Item = InfoBaseConnection>,
    ) -> Self {
        self.connections = Some(connections.into_iter().collect());
        self
    }

    /// Подключает журнал регистрации этого сеанса.
    #[must_use]
    pub fn with_event_log_writer(mut self, writer: Rc<dyn EventLogWriter>) -> Self {
        self.event_log_writer = Some(writer);
        self
    }

    /// Подключает провайдер клиентских настроек этого сеанса.
    #[must_use]
    pub fn with_client_settings_adapter(mut self, adapter: Rc<dyn ClientSettingsAdapter>) -> Self {
        self.client_settings = Some(adapter);
        self
    }
}

/// Дополнение [`bsl_orm::SessionData`] для runtime сеанса.
#[derive(Debug)]
pub struct SessionRuntime {
    parameters: Rc<SessionParameters>,
    kind: SessionKind,
    platform_version: Arc<str>,
    connection_string: Option<Arc<str>>,
    connection_number: u64,
    connections: Option<Arc<[InfoBaseConnection]>>,
    event_log_writer: Option<Rc<dyn EventLogWriter>>,
    client_settings: Option<Rc<dyn ClientSettingsAdapter>>,
    background_method: Option<Arc<str>>,
}

impl SessionRuntime {
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

    /// Возвращает переданную хостом строку регистрации информационной базы.
    #[must_use]
    pub fn connection_string(&self) -> Option<&str> {
        self.connection_string.as_deref()
    }

    /// Возвращает номер соединения текущего сеанса.
    #[must_use]
    pub const fn connection_number(&self) -> u64 {
        self.connection_number
    }

    /// Возвращает разрешённый текущему пользователю снимок соединений.
    #[must_use]
    pub fn connections(&self) -> Option<&[InfoBaseConnection]> {
        self.connections.as_deref()
    }

    /// Возвращает журнал регистрации текущего сеанса.
    #[must_use]
    pub fn event_log_writer(&self) -> Option<&Rc<dyn EventLogWriter>> {
        self.event_log_writer.as_ref()
    }

    /// Возвращает провайдер клиентских настроек текущего сеанса.
    #[must_use]
    pub fn client_settings(&self) -> Option<&Rc<dyn ClientSettingsAdapter>> {
        self.client_settings.as_ref()
    }

    /// Возвращает полное имя метода текущего фонового задания.
    #[must_use]
    pub fn background_method(&self) -> Option<&str> {
        self.background_method.as_deref()
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
    platform_version: Arc<str>,
    connection_string: Option<Arc<str>>,
}

impl SessionFactory {
    /// Проверяет совместимость кода и метаданных и один раз компилирует
    /// конфигурационный граф для указанного вида сеанса.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку пустой версии платформы, несовпадающей идентичности,
    /// отсутствующего объекта владельца или модуля сеанса, а также ошибку
    /// сборки open-bsl.
    pub fn new(
        code: ConfigurationCodeSnapshot,
        catalog: Arc<MetadataCatalog>,
        session_module: &str,
        kind: SessionKind,
        platform_version: impl Into<String>,
    ) -> Result<Self, SessionError> {
        let platform_version = platform_version.into();
        if platform_version.is_empty() {
            return Err(SessionError::EmptyPlatformVersion);
        }
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
            .filter(|module| module.kind() != ModuleKind::Common || module.execution().server())
            .map(|module| {
                let mut imports = Vec::new();
                for import in module.imports() {
                    let target = code.module(import.module()).ok_or_else(|| {
                        SessionError::MissingImportedModule {
                            module: module.name().into(),
                            imported: import.module().into(),
                        }
                    })?;
                    if target.kind() != ModuleKind::Common || target.execution().server() {
                        imports.push((import.alias().to_string(), target.name().to_string()));
                    }
                }
                Ok(ModuleRecipe {
                    visibility: match module.kind() {
                        ModuleKind::Common if module.execution().global() => {
                            open_bsl::ModuleVisibility::Global
                        }
                        ModuleKind::Common => open_bsl::ModuleVisibility::Named,
                        // Модуль сеанса видит только синтетический host-entry,
                        // который запускает обработчик начала сеанса.
                        ModuleKind::Session => open_bsl::ModuleVisibility::Explicit,
                        _ => open_bsl::ModuleVisibility::Private,
                    },
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
            platform_version: platform_version.into(),
            connection_string: None,
        })
    }

    /// Задаёт строку регистрации информационной базы для создаваемых сеансов.
    /// Строка сохраняется без разбора и преобразования.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку, если передана пустая строка.
    pub fn with_connection_string(
        mut self,
        connection_string: impl Into<String>,
    ) -> Result<Self, SessionError> {
        let connection_string = connection_string.into();
        if connection_string.is_empty() {
            return Err(SessionError::EmptyConnectionString);
        }
        self.connection_string = Some(Arc::from(connection_string));
        Ok(self)
    }

    /// Открывает изолированный сеанс на уже созданном отдельном соединении.
    /// Соединение передаётся во владение и не может быть унаследовано другим
    /// сеансом случайным клонированием.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку недопустимого номера соединения, создания контейнера,
    /// инициализации модуля или стартового
    /// `УстановкаПараметровСеанса(Неопределено)`. Частичный [`Session`] при
    /// ошибке не возвращается.
    pub fn open(
        &self,
        principal: Principal,
        resources: SessionResources,
    ) -> Result<Session, SessionError> {
        self.open_with_resources(principal, resources, None, None)
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
        resources: SessionResources,
        method_name: &str,
        parent: &Session,
    ) -> Result<Session, SessionError> {
        if self.kind != SessionKind::BackgroundJob {
            return Err(SessionError::NotBackgroundFactory);
        }
        if method_name.is_empty() {
            return Err(SessionError::EmptyBackgroundMethod);
        }
        self.open_with_resources(
            principal,
            resources,
            Some(Rc::new(parent.parameters.copy_for_background())),
            Some(Arc::from(method_name)),
        )
    }

    fn open_with_resources(
        &self,
        principal: Principal,
        resources: SessionResources,
        parameters: Option<Rc<SessionParameters>>,
        background_method: Option<Arc<str>>,
    ) -> Result<Session, SessionError> {
        let SessionResources {
            connection,
            connection_number,
            data_access,
            connections,
            event_log_writer,
            client_settings,
        } = resources;
        if connection_number == 0 || connection_number > i64::MAX as u64 {
            return Err(SessionError::InvalidConnectionNumber(connection_number));
        }
        if let Some(connections) = connections.as_deref() {
            for connection in connections {
                if connection.number == 0 || connection.number > i64::MAX as u64 {
                    return Err(SessionError::InvalidListedConnectionNumber(
                        connection.number,
                    ));
                }
            }
            if !connections
                .iter()
                .any(|connection| connection.number == connection_number)
            {
                return Err(SessionError::MissingCurrentConnection(connection_number));
            }
        }
        let parameters = match parameters {
            Some(parameters) => parameters,
            None => Rc::new(SessionParameters::new(
                self.session_module.clone(),
                self.parameter_names.iter().cloned(),
            )?),
        };
        let runtime = SessionRuntime {
            parameters: Rc::clone(&parameters),
            kind: self.kind,
            platform_version: Arc::clone(&self.platform_version),
            connection_string: self.connection_string.clone(),
            connection_number,
            connections,
            event_log_writer,
            client_settings,
            background_method,
        };
        let mut data = SessionData::new(Arc::clone(&self.catalog), principal.clone())
            .with_extension(Rc::new(runtime));
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
    /// Версия платформы не задана.
    EmptyPlatformVersion,
    /// Строка регистрации информационной базы пуста.
    EmptyConnectionString,
    /// Номер соединения не является положительным числом, представимым в BSL.
    InvalidConnectionNumber(u64),
    /// Номер соединения из снимка не представим в BSL.
    InvalidListedConnectionNumber(u64),
    /// В снимке отсутствует соединение создаваемого сеанса.
    MissingCurrentConnection(u64),
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
    /// Имя метода фонового задания пусто.
    EmptyBackgroundMethod,
    /// Ошибка объявления параметров.
    Parameters(ParameterError),
    /// Ошибка open-bsl.
    OpenBsl(OpenBslError),
}

impl fmt::Display for SessionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyPlatformVersion => formatter.write_str("версия платформы пуста"),
            Self::EmptyConnectionString => {
                formatter.write_str("строка соединения с информационной базой пуста")
            }
            Self::InvalidConnectionNumber(number) => write!(
                formatter,
                "номер соединения с информационной базой недопустим: {number}"
            ),
            Self::InvalidListedConnectionNumber(number) => write!(
                formatter,
                "снимок содержит недопустимый номер соединения: {number}"
            ),
            Self::MissingCurrentConnection(number) => write!(
                formatter,
                "текущее соединение `{number}` отсутствует в снимке соединений"
            ),
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
            Self::EmptyBackgroundMethod => formatter.write_str("имя метода фонового задания пусто"),
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

#[derive(Debug)]
struct SystemInformationObject {
    platform_version: Arc<str>,
}

#[derive(Debug)]
struct InfoBaseSessionObject {
    background_method: Option<Arc<str>>,
}

#[derive(Debug)]
struct CurrentBackgroundJobObject {
    method_name: Arc<str>,
}

#[derive(Debug)]
struct InfoBaseConnectionObject {
    connection: InfoBaseConnection,
}

#[derive(Debug)]
struct StandardSettingsStorageObject {
    storage: ClientSettingsStorage,
}

#[derive(Debug)]
struct StandardSettingsSelectionObject {
    rows: Rc<[ClientSettingsEntry]>,
    position: Cell<usize>,
}

#[derive(Debug)]
struct HomePageSettingsObject;

#[derive(Debug)]
struct HomePageFormsObject {
    left: Value,
    right: Value,
}

static PARAMETERS_TYPE: TypeDescriptor = TypeDescriptor::new("bsl-session", "ПараметрыСеанса");
static SYSTEM_INFORMATION_TYPE: TypeDescriptor =
    TypeDescriptor::new("bsl-session", "СистемнаяИнформация");
static INFOBASE_SESSION_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-session",
    name: "СеансИнформационнойБазы",
    type_display: "Active session",
    type_names: &["InfoBaseSession"],
};
static INFOBASE_CONNECTION_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-session",
    name: "СоединениеИнформационнойБазы",
    type_display: "Active connection",
    type_names: &["InfoBaseConnection"],
};
static STANDARD_SETTINGS_STORAGE_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-session",
    name: "СтандартноеХранилищеНастроекМенеджер",
    type_display: "Standard settings storage",
    type_names: &["StandardSettingsStorageManager"],
};
static STANDARD_SETTINGS_SELECTION_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-session",
    name: "СтандартноеХранилищеНастроекВыборка",
    type_display: "Standard settings storage Selected data",
    type_names: &["StandardSettingsStorageSelection"],
};
static HOME_PAGE_SETTINGS_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-session",
    name: "НастройкиНачальнойСтраницы",
    type_display: "Home page settings",
    type_names: &["HomePageSettings"],
};
static HOME_PAGE_FORMS_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-session",
    name: "СоставФормНачальнойСтраницы",
    type_display: "Home page forms",
    type_names: &["HomePageForms"],
};

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

impl ObjectProtocol for SystemInformationObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &SYSTEM_INFORMATION_TYPE
    }

    fn property_table(&self) -> &'static [PropertyDescriptor] {
        SYSTEM_INFORMATION_PROPERTIES
    }
}

impl ObjectProtocol for InfoBaseSessionObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &INFOBASE_SESSION_TYPE
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        INFOBASE_SESSION_METHODS
    }
}

impl ObjectProtocol for CurrentBackgroundJobObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        background_job_type_descriptor()
    }

    fn property_table(&self) -> &'static [PropertyDescriptor] {
        CURRENT_BACKGROUND_JOB_PROPERTIES
    }
}

impl ObjectProtocol for InfoBaseConnectionObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &INFOBASE_CONNECTION_TYPE
    }

    fn property_table(&self) -> &'static [PropertyDescriptor] {
        INFOBASE_CONNECTION_PROPERTIES
    }
}

impl ObjectProtocol for StandardSettingsStorageObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &STANDARD_SETTINGS_STORAGE_TYPE
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        STANDARD_SETTINGS_STORAGE_METHODS
    }
}

impl ObjectProtocol for StandardSettingsSelectionObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &STANDARD_SETTINGS_SELECTION_TYPE
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        STANDARD_SETTINGS_SELECTION_METHODS
    }

    fn property_table(&self) -> &'static [PropertyDescriptor] {
        STANDARD_SETTINGS_SELECTION_PROPERTIES
    }
}

impl ObjectProtocol for HomePageSettingsObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &HOME_PAGE_SETTINGS_TYPE
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        HOME_PAGE_SETTINGS_METHODS
    }
}

impl ObjectProtocol for HomePageFormsObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &HOME_PAGE_FORMS_TYPE
    }

    fn property_table(&self) -> &'static [PropertyDescriptor] {
        HOME_PAGE_FORMS_PROPERTIES
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

fn session_data<'a>(ctx: &'a CallContext<'_>) -> RtResult<&'a SessionData> {
    ctx.host_data()?
        .downcast_ref::<SessionData>()
        .ok_or_else(|| RtError::DynamicError("данные сеанса не переданы".into()))
}

fn runtime<'a>(ctx: &'a CallContext<'_>) -> RtResult<&'a SessionRuntime> {
    session_data(ctx)?
        .extension::<SessionRuntime>()
        .ok_or_else(|| RtError::DynamicError("runtime сеанса не передан".into()))
}

fn parameters(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    Ok(Value::new_object(ParametersObject {
        parameters: Rc::clone(runtime(ctx)?.parameters()),
    }))
}

fn current_run_mode(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    match runtime(ctx)?.kind() {
        SessionKind::WebClient | SessionKind::ThinClient => {
            Ok(Value::Enum(EnumValue::ClientRunModeManagedApplication))
        }
        // Измерено на платформе 8.3.27 настоящим фоновым заданием:
        // `measure-session-current-run-mode.bsl`.
        SessionKind::BackgroundJob => Ok(Value::Undefined),
    }
}

fn infobase_connection_string(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    let value = runtime(ctx)?.connection_string().ok_or_else(|| {
        RtError::DynamicError("строка соединения с информационной базой не передана хостом".into())
    })?;
    Ok(Value::Str(value.into()))
}

fn infobase_connection_number(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    Ok(Value::number_from_i64(
        runtime(ctx)?.connection_number() as i64
    ))
}

fn infobase_connections(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    let connections = runtime(ctx)?.connections().ok_or_else(|| {
        RtError::DynamicError(
            "снимок соединений информационной базы недоступен текущему пользователю".into(),
        )
    })?;
    Ok(Value::new_array(
        connections
            .iter()
            .cloned()
            .map(|connection| Value::new_object(InfoBaseConnectionObject { connection }))
            .collect(),
    ))
}

fn event_log_level(value: Option<&Value>) -> RtResult<EventLogLevel> {
    match value.unwrap_or(&Value::Undefined) {
        Value::Undefined => Ok(EventLogLevel::Note),
        Value::Enum(EnumValue::EventLogLevelNote) => Ok(EventLogLevel::Note),
        Value::Enum(EnumValue::EventLogLevelInformation) => Ok(EventLogLevel::Information),
        Value::Enum(EnumValue::EventLogLevelWarning) => Ok(EventLogLevel::Warning),
        Value::Enum(EnumValue::EventLogLevelError) => Ok(EventLogLevel::Error),
        _ => Err(RtError::TypeError {
            expected: "УровеньЖурналаРегистрации",
            op: "ЗаписьЖурналаРегистрации",
        }),
    }
}

fn event_log_transaction_mode(value: Option<&Value>) -> RtResult<EventLogTransactionMode> {
    match value.unwrap_or(&Value::Undefined) {
        Value::Undefined => Ok(EventLogTransactionMode::Independent),
        Value::Enum(EnumValue::EventLogEntryTransactionModeIndependent) => {
            Ok(EventLogTransactionMode::Independent)
        }
        Value::Enum(EnumValue::EventLogEntryTransactionModeTransactional) => {
            Ok(EventLogTransactionMode::Transactional)
        }
        _ => Err(RtError::TypeError {
            expected: "РежимТранзакцииЗаписиЖурналаРегистрации",
            op: "ЗаписьЖурналаРегистрации",
        }),
    }
}

fn write_log_event(ctx: &mut CallContext<'_>, arguments: &[Value]) -> RtResult<Value> {
    let event_name = match &arguments[0] {
        Value::Str(value) => Arc::from(value.to_string()),
        _ => {
            return Err(RtError::TypeError {
                expected: "Строка",
                op: "ЗаписьЖурналаРегистрации",
            });
        }
    };
    let comment = match arguments.get(4).unwrap_or(&Value::Undefined) {
        Value::Undefined => Arc::from(""),
        Value::Str(value) => Arc::from(value.to_string()),
        _ => {
            return Err(RtError::TypeError {
                expected: "Строка",
                op: "ЗаписьЖурналаРегистрации",
            });
        }
    };
    let entry = EventLogEntry {
        event_name,
        level: event_log_level(arguments.get(1))?,
        metadata: arguments.get(2).cloned().unwrap_or(Value::Undefined),
        data: arguments.get(3).cloned().unwrap_or(Value::Undefined),
        comment,
        transaction_mode: event_log_transaction_mode(arguments.get(5))?,
    };
    let writer = Rc::clone(runtime(ctx)?.event_log_writer().ok_or_else(|| {
        RtError::DynamicError("журнал регистрации не подключён для текущего сеанса".into())
    })?);
    writer.write(entry)?;
    Ok(Value::Undefined)
}

fn client_settings_context<'a>(
    ctx: &'a CallContext<'_>,
) -> RtResult<(&'a Principal, Rc<dyn ClientSettingsAdapter>)> {
    let data = session_data(ctx)?;
    let adapter = data
        .extension::<SessionRuntime>()
        .and_then(SessionRuntime::client_settings)
        .cloned()
        .ok_or_else(|| {
            RtError::DynamicError(
                "провайдер клиентских настроек не подключён для текущего сеанса".into(),
            )
        })?;
    Ok((data.principal(), adapter))
}

fn settings_storage(receiver: &dyn ObjectProtocol) -> RtResult<&StandardSettingsStorageObject> {
    receiver
        .downcast_ref::<StandardSettingsStorageObject>()
        .ok_or(RtError::NotAnObject)
}

fn settings_text(value: &Value, operation: &'static str) -> RtResult<Arc<str>> {
    match value {
        Value::Str(value) => Ok(Arc::from(value.to_string())),
        _ => Err(RtError::TypeError {
            expected: "Строка",
            op: operation,
        }),
    }
}

fn optional_settings_key(arguments: &[Value], operation: &'static str) -> RtResult<Arc<str>> {
    match arguments.get(1).unwrap_or(&Value::Undefined) {
        Value::Undefined => Ok(Arc::from("")),
        value => settings_text(value, operation),
    }
}

fn optional_user_name(
    arguments: &[Value],
    index: usize,
    operation: &'static str,
) -> RtResult<Option<Arc<str>>> {
    match arguments.get(index).unwrap_or(&Value::Undefined) {
        Value::Undefined => Ok(None),
        value => settings_text(value, operation).map(Some),
    }
}

fn load_client_setting(
    receiver: &dyn ObjectProtocol,
    arguments: &[Value],
    ctx: &mut CallContext<'_>,
) -> RtResult<Value> {
    const OPERATION: &str = "Загрузить";
    let request = ClientSettingsLoadRequest {
        storage: settings_storage(receiver)?.storage,
        object_key: settings_text(&arguments[0], OPERATION)?,
        settings_key: optional_settings_key(arguments, OPERATION)?,
        description: arguments.get(2).cloned().unwrap_or(Value::Undefined),
        user_name: optional_user_name(arguments, 3, OPERATION)?,
    };
    let (principal, adapter) = client_settings_context(ctx)?;
    Ok(adapter
        .load(principal, &request)?
        .unwrap_or(Value::Undefined))
}

fn save_client_setting(
    receiver: &dyn ObjectProtocol,
    arguments: &[Value],
    ctx: &mut CallContext<'_>,
) -> RtResult<Value> {
    const OPERATION: &str = "Сохранить";
    let request = ClientSettingsSaveRequest {
        storage: settings_storage(receiver)?.storage,
        object_key: settings_text(&arguments[0], OPERATION)?,
        settings_key: optional_settings_key(arguments, OPERATION)?,
        settings: arguments[2].clone(),
        description: arguments.get(3).cloned().unwrap_or(Value::Undefined),
        user_name: optional_user_name(arguments, 4, OPERATION)?,
    };
    let (principal, adapter) = client_settings_context(ctx)?;
    adapter.save(principal, &request)?;
    Ok(Value::Undefined)
}

fn select_client_settings(
    receiver: &dyn ObjectProtocol,
    arguments: &[Value],
    ctx: &mut CallContext<'_>,
) -> RtResult<Value> {
    let filter = arguments.first().cloned().unwrap_or(Value::Undefined);
    if !matches!(filter, Value::Undefined)
        && !matches!(&filter, Value::Object(object) if matches!(object.as_ref(), BslObject::Structure(_)))
    {
        return Err(RtError::TypeError {
            expected: "Структура",
            op: "Выбрать",
        });
    }
    let request = ClientSettingsSelectRequest {
        storage: settings_storage(receiver)?.storage,
        filter,
    };
    let (principal, adapter) = client_settings_context(ctx)?;
    Ok(Value::new_object(StandardSettingsSelectionObject {
        rows: adapter.select(principal, &request)?.into(),
        position: Cell::new(usize::MAX),
    }))
}

fn next_client_setting(
    receiver: &dyn ObjectProtocol,
    _: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let selection = receiver
        .downcast_ref::<StandardSettingsSelectionObject>()
        .ok_or(RtError::NotAnObject)?;
    let next = if selection.position.get() == usize::MAX {
        0
    } else {
        selection.position.get().saturating_add(1)
    };
    if next < selection.rows.len() {
        selection.position.set(next);
        Ok(Value::Boolean(true))
    } else {
        selection.position.set(selection.rows.len());
        Ok(Value::Boolean(false))
    }
}

fn current_client_setting(receiver: &dyn ObjectProtocol) -> RtResult<&ClientSettingsEntry> {
    let selection = receiver
        .downcast_ref::<StandardSettingsSelectionObject>()
        .ok_or(RtError::NotAnObject)?;
    selection
        .rows
        .get(selection.position.get())
        .ok_or_else(|| RtError::DynamicError("выборка находится вне набора данных".into()))
}

fn selected_object_key(receiver: &dyn ObjectProtocol, _: &mut CallContext<'_>) -> RtResult<Value> {
    Ok(Value::Str(
        current_client_setting(receiver)?.object_key().into(),
    ))
}

fn selected_settings_key(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    Ok(Value::Str(
        current_client_setting(receiver)?.settings_key().into(),
    ))
}

fn selected_user(receiver: &dyn ObjectProtocol, _: &mut CallContext<'_>) -> RtResult<Value> {
    Ok(Value::Str(current_client_setting(receiver)?.user().into()))
}

fn selected_settings(receiver: &dyn ObjectProtocol, _: &mut CallContext<'_>) -> RtResult<Value> {
    Ok(current_client_setting(receiver)?.settings().clone())
}

fn construct_home_page_settings(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    Ok(Value::new_object(HomePageSettingsObject))
}

fn home_page_forms_value(forms: &HomePageFormsData) -> Value {
    Value::new_object(HomePageFormsObject {
        left: Value::new_array(
            forms
                .left()
                .iter()
                .map(|name| Value::Str(name.as_ref().into()))
                .collect(),
        ),
        right: Value::new_array(
            forms
                .right()
                .iter()
                .map(|name| Value::Str(name.as_ref().into()))
                .collect(),
        ),
    })
}

fn construct_home_page_forms(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    Ok(home_page_forms_value(&HomePageFormsData::default()))
}

fn get_home_page_forms(
    receiver: &dyn ObjectProtocol,
    _: &[Value],
    ctx: &mut CallContext<'_>,
) -> RtResult<Value> {
    receiver
        .downcast_ref::<HomePageSettingsObject>()
        .ok_or(RtError::NotAnObject)?;
    let (principal, adapter) = client_settings_context(ctx)?;
    Ok(home_page_forms_value(&adapter.home_page_forms(principal)?))
}

fn form_names(value: &Value) -> RtResult<Vec<String>> {
    let Value::Object(object) = value else {
        return Err(RtError::TypeError {
            expected: "Массив",
            op: "УстановитьСоставФорм",
        });
    };
    let BslObject::Array(items) = object.as_ref() else {
        return Err(RtError::TypeError {
            expected: "Массив",
            op: "УстановитьСоставФорм",
        });
    };
    items
        .borrow()
        .iter()
        .map(|item| match item {
            Value::Str(value) => Ok(value.to_string()),
            _ => Err(RtError::TypeError {
                expected: "Строка",
                op: "УстановитьСоставФорм",
            }),
        })
        .collect()
}

fn set_home_page_forms(
    receiver: &dyn ObjectProtocol,
    arguments: &[Value],
    ctx: &mut CallContext<'_>,
) -> RtResult<Value> {
    receiver
        .downcast_ref::<HomePageSettingsObject>()
        .ok_or(RtError::NotAnObject)?;
    let Value::Object(object) = &arguments[0] else {
        return Err(RtError::TypeError {
            expected: "СоставФормНачальнойСтраницы",
            op: "УстановитьСоставФорм",
        });
    };
    let BslObject::Extension(object) = object.as_ref() else {
        return Err(RtError::TypeError {
            expected: "СоставФормНачальнойСтраницы",
            op: "УстановитьСоставФорм",
        });
    };
    let forms = object
        .downcast_ref::<HomePageFormsObject>()
        .ok_or(RtError::TypeError {
            expected: "СоставФормНачальнойСтраницы",
            op: "УстановитьСоставФорм",
        })?;
    let forms = HomePageFormsData::new(form_names(&forms.left)?, form_names(&forms.right)?);
    let (principal, adapter) = client_settings_context(ctx)?;
    adapter.set_home_page_forms(principal, &forms)?;
    Ok(Value::Undefined)
}

fn home_page_left(receiver: &dyn ObjectProtocol, _: &mut CallContext<'_>) -> RtResult<Value> {
    Ok(receiver
        .downcast_ref::<HomePageFormsObject>()
        .ok_or(RtError::NotAnObject)?
        .left
        .clone())
}

fn home_page_right(receiver: &dyn ObjectProtocol, _: &mut CallContext<'_>) -> RtResult<Value> {
    Ok(receiver
        .downcast_ref::<HomePageFormsObject>()
        .ok_or(RtError::NotAnObject)?
        .right
        .clone())
}

fn common_settings_storage(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    Ok(Value::new_object(StandardSettingsStorageObject {
        storage: ClientSettingsStorage::Common,
    }))
}

fn system_settings_storage(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    Ok(Value::new_object(StandardSettingsStorageObject {
        storage: ClientSettingsStorage::System,
    }))
}

fn infobase_connection_number_property(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let connection = receiver
        .downcast_ref::<InfoBaseConnectionObject>()
        .ok_or(RtError::NotAnObject)?;
    Ok(Value::number_from_i64(connection.connection.number as i64))
}

fn infobase_connection_application_name(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let connection = receiver
        .downcast_ref::<InfoBaseConnectionObject>()
        .ok_or(RtError::NotAnObject)?;
    Ok(Value::Str(
        connection.connection.application_name.as_ref().into(),
    ))
}

fn infobase_connection_computer_name(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let connection = receiver
        .downcast_ref::<InfoBaseConnectionObject>()
        .ok_or(RtError::NotAnObject)?;
    Ok(Value::Str(
        connection.connection.computer_name.as_ref().into(),
    ))
}

fn security_mode_flag(arguments: &[Value], operation: &'static str) -> RtResult<bool> {
    match &arguments[0] {
        Value::Boolean(value) => Ok(*value),
        Value::Str(_) => Err(RtError::DynamicError(format!(
            "{operation}: именованные профили безопасности не поддерживаются"
        ))),
        _ => Err(RtError::TypeError {
            expected: "Булево",
            op: operation,
        }),
    }
}

fn set_safe_mode(ctx: &mut CallContext<'_>, arguments: &[Value]) -> RtResult<Value> {
    ctx.set_safe_mode(security_mode_flag(arguments, "УстановитьБезопасныйРежим")?)?;
    Ok(Value::Undefined)
}

fn safe_mode(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    Ok(Value::Boolean(ctx.safe_mode()?))
}

fn set_privileged_mode(ctx: &mut CallContext<'_>, arguments: &[Value]) -> RtResult<Value> {
    ctx.set_privileged_mode(security_mode_flag(
        arguments,
        "УстановитьПривилегированныйРежим",
    )?)?;
    Ok(Value::Undefined)
}

fn privileged_mode(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    Ok(Value::Boolean(ctx.privileged_mode()?))
}

fn current_infobase_session(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    Ok(Value::new_object(InfoBaseSessionObject {
        background_method: runtime(ctx)?.background_method.clone(),
    }))
}

fn current_background_job(
    receiver: &dyn ObjectProtocol,
    _: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let session = receiver
        .downcast_ref::<InfoBaseSessionObject>()
        .ok_or(RtError::NotAnObject)?;
    Ok(session
        .background_method
        .as_ref()
        .map_or(Value::Undefined, |name| {
            Value::new_object(CurrentBackgroundJobObject {
                method_name: Arc::clone(name),
            })
        }))
}

fn current_background_job_method_name(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let job = receiver
        .downcast_ref::<CurrentBackgroundJobObject>()
        .ok_or(RtError::NotAnObject)?;
    Ok(Value::Str(job.method_name.as_ref().into()))
}

fn construct_system_information(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    let platform_version = runtime(ctx)?.platform_version.clone();
    Ok(Value::new_object(SystemInformationObject {
        platform_version,
    }))
}

fn system_information_app_version(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let information = receiver
        .downcast_ref::<SystemInformationObject>()
        .ok_or(RtError::NotAnObject)?;
    Ok(Value::Str(information.platform_version.as_ref().into()))
}

const CONSTRUCTORS: &[ConstructorDescriptor] = &[
    ConstructorDescriptor {
        code: ConstructorCode::new(1),
        names: &["СистемнаяИнформация", "SystemInfo"],
        arity: Arity::exact(0),
        call: construct_system_information,
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(2),
        names: &["НастройкиНачальнойСтраницы", "HomePageSettings"],
        arity: Arity::exact(0),
        call: construct_home_page_settings,
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(3),
        names: &["СоставФормНачальнойСтраницы", "HomePageForms"],
        arity: Arity::exact(0),
        call: construct_home_page_forms,
    },
];

const CONSTRUCTOR_TYPES: &[(ConstructorCode, &TypeDescriptor)] = &[
    (ConstructorCode::new(1), &SYSTEM_INFORMATION_TYPE),
    (ConstructorCode::new(2), &HOME_PAGE_SETTINGS_TYPE),
    (ConstructorCode::new(3), &HOME_PAGE_FORMS_TYPE),
];

static SYSTEM_INFORMATION_PROPERTIES: &[PropertyDescriptor] = &[PropertyDescriptor {
    names: &["ВерсияПриложения", "AppVersion"],
    get: system_information_app_version,
    set: None,
}];

static INFOBASE_SESSION_METHODS: &[MethodDescriptor] = &[MethodDescriptor::new(
    &["ПолучитьФоновоеЗадание", "GetBackgroundJob"],
    Arity::exact(0),
    current_background_job,
)];

static CURRENT_BACKGROUND_JOB_PROPERTIES: &[PropertyDescriptor] = &[PropertyDescriptor {
    names: &["ИмяМетода", "MethodName"],
    get: current_background_job_method_name,
    set: None,
}];

static INFOBASE_CONNECTION_PROPERTIES: &[PropertyDescriptor] = &[
    PropertyDescriptor {
        names: &["НомерСоединения", "ConnectionNumber"],
        get: infobase_connection_number_property,
        set: None,
    },
    PropertyDescriptor {
        names: &["ИмяПриложения", "ApplicationName"],
        get: infobase_connection_application_name,
        set: None,
    },
    PropertyDescriptor {
        names: &["ИмяКомпьютера", "ComputerName"],
        get: infobase_connection_computer_name,
        set: None,
    },
];

static STANDARD_SETTINGS_STORAGE_METHODS: &[MethodDescriptor] = &[
    MethodDescriptor::new(
        &["Загрузить", "Load"],
        Arity::range(1, 4),
        load_client_setting,
    ),
    MethodDescriptor::new(
        &["Сохранить", "Save"],
        Arity::range(3, 5),
        save_client_setting,
    )
    .as_procedure(),
    MethodDescriptor::new(
        &["Выбрать", "Select"],
        Arity::range(0, 1),
        select_client_settings,
    ),
];

static STANDARD_SETTINGS_SELECTION_METHODS: &[MethodDescriptor] = &[MethodDescriptor::new(
    &["Следующий", "Next"],
    Arity::exact(0),
    next_client_setting,
)];

static STANDARD_SETTINGS_SELECTION_PROPERTIES: &[PropertyDescriptor] = &[
    PropertyDescriptor {
        names: &["КлючОбъекта", "ObjectKey"],
        get: selected_object_key,
        set: None,
    },
    PropertyDescriptor {
        names: &["КлючНастроек", "SettingsKey"],
        get: selected_settings_key,
        set: None,
    },
    PropertyDescriptor {
        names: &["Пользователь", "User"],
        get: selected_user,
        set: None,
    },
    PropertyDescriptor {
        names: &["Настройки", "Settings"],
        get: selected_settings,
        set: None,
    },
];

static HOME_PAGE_SETTINGS_METHODS: &[MethodDescriptor] = &[
    MethodDescriptor::new(
        &["ПолучитьСоставФорм", "GetForms"],
        Arity::exact(0),
        get_home_page_forms,
    ),
    MethodDescriptor::new(
        &["УстановитьСоставФорм", "SetForms"],
        Arity::exact(1),
        set_home_page_forms,
    )
    .as_procedure(),
];

static HOME_PAGE_FORMS_PROPERTIES: &[PropertyDescriptor] = &[
    PropertyDescriptor {
        names: &["ЛеваяКолонка", "LeftColumn"],
        get: home_page_left,
        set: None,
    },
    PropertyDescriptor {
        names: &["ПраваяКолонка", "RightColumn"],
        get: home_page_right,
        set: None,
    },
];

const FUNCTIONS: &[FunctionDescriptor] = &[
    FunctionDescriptor {
        code: FunctionCode::new(1),
        names: &["ПолучитьПараметрыСеанса"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: parameters,
    },
    FunctionDescriptor {
        code: FunctionCode::new(2),
        names: &["ТекущийРежимЗапуска", "CurrentRunMode"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: current_run_mode,
    },
    FunctionDescriptor {
        code: FunctionCode::new(3),
        names: &[
            "СтрокаСоединенияИнформационнойБазы",
            "InfoBaseConnectionString",
        ],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: infobase_connection_string,
    },
    FunctionDescriptor {
        code: FunctionCode::new(4),
        names: &[
            "ПолучитьТекущийСеансИнформационнойБазы",
            "GetCurrentInfoBaseSession",
        ],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: current_infobase_session,
    },
    FunctionDescriptor {
        code: FunctionCode::new(5),
        names: &[
            "НомерСоединенияИнформационнойБазы",
            "InfoBaseConnectionNumber",
        ],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: infobase_connection_number,
    },
    FunctionDescriptor {
        code: FunctionCode::new(6),
        names: &["УстановитьБезопасныйРежим", "SetSafeMode"],
        arity: Arity::exact(1),
        kind: FunctionKind::Procedure,
        call: set_safe_mode,
    },
    FunctionDescriptor {
        code: FunctionCode::new(7),
        names: &["БезопасныйРежим", "SafeMode"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: safe_mode,
    },
    FunctionDescriptor {
        code: FunctionCode::new(8),
        names: &["УстановитьПривилегированныйРежим", "SetPrivilegedMode"],
        arity: Arity::exact(1),
        kind: FunctionKind::Procedure,
        call: set_privileged_mode,
    },
    FunctionDescriptor {
        code: FunctionCode::new(9),
        names: &["ПривилегированныйРежим", "PrivilegedMode"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: privileged_mode,
    },
    FunctionDescriptor {
        code: FunctionCode::new(10),
        names: &[
            "ПолучитьСоединенияИнформационнойБазы",
            "GetInfoBaseConnections",
        ],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: infobase_connections,
    },
    FunctionDescriptor {
        code: FunctionCode::new(11),
        names: &["ЗаписьЖурналаРегистрации", "WriteLogEvent"],
        arity: Arity::range(1, 6),
        kind: FunctionKind::Procedure,
        call: write_log_event,
    },
    FunctionDescriptor {
        code: FunctionCode::new(12),
        names: &["ПолучитьХранилищеОбщихНастроек"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: common_settings_storage,
    },
    FunctionDescriptor {
        code: FunctionCode::new(13),
        names: &["ПолучитьХранилищеСистемныхНастроек"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: system_settings_storage,
    },
];

const GLOBALS: &[GlobalDescriptor] = &[
    GlobalDescriptor {
        names: &["ПараметрыСеанса", "SessionParameters"],
        function: FunctionCode::new(1),
    },
    GlobalDescriptor {
        names: &["ХранилищеОбщихНастроек", "CommonSettingsStorage"],
        function: FunctionCode::new(12),
    },
    GlobalDescriptor {
        names: &["ХранилищеСистемныхНастроек", "SystemSettingsStorage"],
        function: FunctionCode::new(13),
    },
];

const TYPES: &[&TypeDescriptor] = &[
    &PARAMETERS_TYPE,
    &SYSTEM_INFORMATION_TYPE,
    &INFOBASE_SESSION_TYPE,
    &INFOBASE_CONNECTION_TYPE,
    &STANDARD_SETTINGS_STORAGE_TYPE,
    &STANDARD_SETTINGS_SELECTION_TYPE,
    &HOME_PAGE_SETTINGS_TYPE,
    &HOME_PAGE_FORMS_TYPE,
];

const API_MEMBERS: &[ObjectMembersDescriptor] = &[
    ObjectMembersDescriptor::new(&PARAMETERS_TYPE)
        .with_methods(PARAMETERS_METHODS)
        .with_dynamic_properties(),
    ObjectMembersDescriptor::new(&SYSTEM_INFORMATION_TYPE)
        .with_properties(SYSTEM_INFORMATION_PROPERTIES),
    ObjectMembersDescriptor::new(&INFOBASE_SESSION_TYPE).with_methods(INFOBASE_SESSION_METHODS),
    ObjectMembersDescriptor::new(&INFOBASE_CONNECTION_TYPE)
        .with_properties(INFOBASE_CONNECTION_PROPERTIES),
    ObjectMembersDescriptor::new(&STANDARD_SETTINGS_STORAGE_TYPE)
        .with_methods(STANDARD_SETTINGS_STORAGE_METHODS),
    ObjectMembersDescriptor::new(&STANDARD_SETTINGS_SELECTION_TYPE)
        .with_methods(STANDARD_SETTINGS_SELECTION_METHODS)
        .with_properties(STANDARD_SETTINGS_SELECTION_PROPERTIES),
    ObjectMembersDescriptor::new(&HOME_PAGE_SETTINGS_TYPE).with_methods(HOME_PAGE_SETTINGS_METHODS),
    ObjectMembersDescriptor::new(&HOME_PAGE_FORMS_TYPE).with_properties(HOME_PAGE_FORMS_PROPERTIES),
];

const OBJECT_MEMBER_GROUPS: &[&[ObjectMembersDescriptor]] = &[API_MEMBERS];

const LIBRARY: LibraryDescriptor = LibraryDescriptor::new("bsl-session", env!("CARGO_PKG_VERSION"))
    .with_functions(FUNCTIONS)
    .with_constructors(CONSTRUCTORS)
    .with_constructor_types(CONSTRUCTOR_TYPES)
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
