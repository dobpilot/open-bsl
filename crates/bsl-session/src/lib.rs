//! Состояние параметров одного сеанса конфигурации 1С.
//!
//! Контейнер воспроизводит измеренную ленивую инициализацию через
//! `УстановкаПараметровСеанса`. Он принадлежит одному `open_bsl::State` и
//! обращается к модулю сеанса через фасадный мост существующей VM.

#![warn(missing_docs)]

mod messages;
mod planner_item;

use std::cell::{Cell, RefCell};
use std::error::Error;
use std::fmt;
use std::rc::Rc;
use std::sync::Arc;

use bsl_config::{ConfigurationCodeSnapshot, ModuleKind};
use bsl_orm::{DataAccess, MetadataCatalog, MetadataKind, Principal, SessionData};
use open_bsl::{
    Arity, BslDate, BslObject, CallContext, ConstructorCode, ConstructorDescriptor, Engine,
    EnumValue, Error as OpenBslError, FunctionCode, FunctionDescriptor, FunctionKind,
    GlobalDescriptor, LibraryDescriptor, MethodDescriptor, ModuleGraphRecipe, ModuleInstance,
    ModuleRecipe, ObjectMembersDescriptor, ObjectProtocol, PropertyDescriptor, RtError, RtResult,
    State, TypeDescriptor, Value, background_job_type_descriptor, brief_error_description,
    detailed_error_description, fixed_array_from_values, folded_eq, value_list_from_values,
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
    positions: Arc<std::collections::HashMap<String, usize>>,
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
        let mut positions = std::collections::HashMap::new();
        for name in names {
            if name.is_empty() {
                return Err(ParameterError::EmptyParameterName);
            }
            if positions.insert(name.to_uppercase(), slots.len()).is_some() {
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
            positions: Arc::new(positions),
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
        let slot = self
            .find_slot_mut(&mut slots, name)
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
        let slot = self
            .find_slot(&slots, name)
            .ok_or_else(|| ParameterError::UnknownParameter(name.into()))?;
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
            positions: Arc::clone(&self.positions),
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
                    let slot = self
                        .find_slot_mut(&mut slots, name)
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
        let result = context.call_configuration_handler(
            &self.handler_module,
            "УстановкаПараметровСеанса",
            vec![argument],
            open_bsl::ConfigurationHandlerEntryMode::Privileged,
        );

        if let Some(required) = required {
            let mut slots = self.slots.borrow_mut();
            if result.is_err() {
                for name in required {
                    if let Some(slot) = self.find_slot_mut(&mut slots, name)
                        && matches!(slot.state, ParameterState::Initializing)
                    {
                        slot.state = ParameterState::Unset;
                    }
                }
            } else {
                for name in required {
                    let slot = self
                        .find_slot_mut(&mut slots, name)
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

    /// Читает параметр, при необходимости вызывая его выборочную инициализацию.
    ///
    /// Адаптер данных использует тот же путь, что и обращение из BSL: цикл,
    /// неизвестное имя и возврат обработчика без значения остаются ошибками.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку для неизвестного имени, повторного чтения во время
    /// инициализации или ошибки обработчика модуля сеанса.
    pub fn get(&self, name: &str, context: &mut CallContext<'_>) -> RtResult<Value> {
        {
            let slots = self.slots.borrow();
            let slot = self
                .find_slot(&slots, name)
                .ok_or_else(|| parameter_access_error(name))?;
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

    // Индекс неизменяем: копия фонового сеанса разделяет только имена и
    // позиции, а значения и состояния инициализации остаются независимыми.
    fn find_slot<'a>(&self, slots: &'a [ParameterSlot], name: &str) -> Option<&'a ParameterSlot> {
        self.positions
            .get(&name.to_uppercase())
            .and_then(|&index| slots.get(index))
    }

    fn find_slot_mut<'a>(
        &self,
        slots: &'a mut [ParameterSlot],
        name: &str,
    ) -> Option<&'a mut ParameterSlot> {
        self.positions
            .get(&name.to_uppercase())
            .and_then(|&index| slots.get_mut(index))
    }
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

impl EventLogLevel {
    fn value(self) -> Value {
        Value::Enum(match self {
            Self::Note => EnumValue::EventLogLevelNote,
            Self::Information => EnumValue::EventLogLevelInformation,
            Self::Warning => EnumValue::EventLogLevelWarning,
            Self::Error => EnumValue::EventLogLevelError,
        })
    }
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
    /// Создаёт запись из подготовленных хостом значений.
    ///
    /// Конструктор сохраняет значения без преобразований и не обращается
    /// к журналу. Для доставки адаптер передаёт запись в `EventLogWriter`
    /// текущего сеанса.
    #[must_use]
    pub fn new(
        event_name: impl Into<Arc<str>>,
        level: EventLogLevel,
        metadata: Value,
        data: Value,
        comment: impl Into<Arc<str>>,
        transaction_mode: EventLogTransactionMode,
    ) -> Self {
        Self {
            event_name: event_name.into(),
            level,
            metadata,
            data,
            comment: comment.into(),
            transaction_mode,
        }
    }

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

/// Снимок настройки использования одного события журнала регистрации.
#[derive(Debug, Clone)]
pub struct EventLogEventUseData {
    use_event: bool,
    description: Value,
}

impl EventLogEventUseData {
    /// Создаёт снимок настройки события.
    #[must_use]
    pub const fn new(use_event: bool, description: Value) -> Self {
        Self {
            use_event,
            description,
        }
    }

    /// Возвращает признак регистрации события.
    #[must_use]
    pub const fn use_event(&self) -> bool {
        self.use_event
    }

    /// Возвращает произвольное описание использования события.
    #[must_use]
    pub const fn description(&self) -> &Value {
        &self.description
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

    /// Читает настройку использования именованного события.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку неизвестного события, проверки прав, хранилища или
    /// неподдерживаемой операции.
    fn event_use(&self, _event_name: &str) -> RtResult<EventLogEventUseData> {
        Err(RtError::DynamicError(
            "чтение настроек событий журнала регистрации не поддерживается адаптером".into(),
        ))
    }

    /// Записывает настройку использования именованного события.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку неизвестного события, проверки прав, хранилища или
    /// неподдерживаемой операции.
    fn set_event_use(&self, _event_name: &str, _use: &EventLogEventUseData) -> RtResult<()> {
        Err(RtError::DynamicError(
            "изменение настроек событий журнала регистрации не поддерживается адаптером".into(),
        ))
    }

    /// Возвращает включённые уровни журнала регистрации текущей базы.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку проверки прав, хранилища или неподдерживаемой
    /// операции.
    fn enabled_levels(&self) -> RtResult<Vec<EventLogLevel>> {
        Err(RtError::DynamicError(
            "чтение включённых уровней журнала регистрации не поддерживается адаптером".into(),
        ))
    }
}

/// Вид стандартного хранилища настроек.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientSettingsStorage {
    /// Общее хранилище настроек конфигурации.
    Common,
    /// Системное хранилище настроек платформы.
    System,
    /// Стандартное хранилище настроек данных форм.
    FormData,
    /// Стандартное хранилище пользовательских настроек отчётов.
    ReportsUser,
    /// Стандартное хранилище пользовательских настроек динамических списков.
    DynamicListsUser,
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

/// Запрос списка ключей стандартного хранилища настроек.
#[derive(Debug, Clone)]
pub struct ClientSettingsListRequest {
    storage: ClientSettingsStorage,
    object_key: Arc<str>,
    user_name: Option<Arc<str>>,
}

impl ClientSettingsListRequest {
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

    /// Возвращает явно указанное имя пользователя.
    #[must_use]
    pub fn user_name(&self) -> Option<&str> {
        self.user_name.as_deref()
    }
}

/// Запрос описания одной настройки.
#[derive(Debug, Clone)]
pub struct ClientSettingsDescriptionRequest {
    storage: ClientSettingsStorage,
    object_key: Arc<str>,
    settings_key: Arc<str>,
    user_name: Option<Arc<str>>,
}

impl ClientSettingsDescriptionRequest {
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

    /// Возвращает ключ настройки.
    #[must_use]
    pub fn settings_key(&self) -> &str {
        &self.settings_key
    }

    /// Возвращает явно указанное имя пользователя.
    #[must_use]
    pub fn user_name(&self) -> Option<&str> {
        self.user_name.as_deref()
    }
}

/// Запрос удаления настроек.
#[derive(Debug, Clone)]
pub struct ClientSettingsDeleteRequest {
    storage: ClientSettingsStorage,
    object_key: Option<Arc<str>>,
    settings_key: Option<Arc<str>>,
    user_name: Option<Arc<str>>,
}

impl ClientSettingsDeleteRequest {
    /// Возвращает выбранное хранилище.
    #[must_use]
    pub const fn storage(&self) -> ClientSettingsStorage {
        self.storage
    }

    /// Возвращает ключ объекта либо `None` для всех объектов.
    #[must_use]
    pub fn object_key(&self) -> Option<&str> {
        self.object_key.as_deref()
    }

    /// Возвращает ключ настройки либо `None` для всех ключей.
    #[must_use]
    pub fn settings_key(&self) -> Option<&str> {
        self.settings_key.as_deref()
    }

    /// Возвращает имя пользователя либо `None` для всех пользователей.
    #[must_use]
    pub fn user_name(&self) -> Option<&str> {
        self.user_name.as_deref()
    }
}

/// Данные объекта `ОписаниеНастроек`, прочитанные провайдером.
#[derive(Debug, Clone)]
pub struct ClientSettingsDescription {
    object_key: Value,
    settings_key: Value,
    user: Value,
    presentation: Value,
    additional_properties: Value,
}

impl ClientSettingsDescription {
    /// Создаёт описание из значений провайдера.
    #[must_use]
    pub const fn new(
        object_key: Value,
        settings_key: Value,
        user: Value,
        presentation: Value,
        additional_properties: Value,
    ) -> Self {
        Self {
            object_key,
            settings_key,
            user,
            presentation,
            additional_properties,
        }
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

    /// Загружает настройку в контексте текущего вызова сеанса.
    ///
    /// Контекст позволяет адаптеру читать параметры разделения и создавать
    /// значения в текущих таблицах имён. По умолчанию вызывает [`Self::load`].
    ///
    /// # Errors
    ///
    /// Возвращает ошибку проверки прав, доступа или декодирования значения.
    fn load_with_context(
        &self,
        principal: &Principal,
        request: &ClientSettingsLoadRequest,
        _context: &mut CallContext<'_>,
    ) -> RtResult<Option<Value>> {
        self.load(principal, request)
    }

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

    /// Возвращает ключи настроек одного объекта.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку проверки прав, доступа или неподдерживаемой
    /// операции.
    fn list(
        &self,
        _principal: &Principal,
        _request: &ClientSettingsListRequest,
    ) -> RtResult<Vec<String>> {
        Err(RtError::DynamicError(
            "чтение списка стандартного хранилища настроек не поддерживается адаптером".into(),
        ))
    }

    /// Возвращает описание одной настройки.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку отсутствия настройки, проверки прав, доступа или
    /// неподдерживаемой операции.
    fn description(
        &self,
        _principal: &Principal,
        _request: &ClientSettingsDescriptionRequest,
    ) -> RtResult<ClientSettingsDescription> {
        Err(RtError::DynamicError(
            "чтение описания стандартного хранилища настроек не поддерживается адаптером".into(),
        ))
    }

    /// Удаляет настройки, подходящие под три селектора запроса.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку проверки прав, доступа или неподдерживаемой
    /// операции.
    fn delete(
        &self,
        _principal: &Principal,
        _request: &ClientSettingsDeleteRequest,
    ) -> RtResult<()> {
        Err(RtError::DynamicError(
            "удаление из стандартного хранилища настроек не поддерживается адаптером".into(),
        ))
    }

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

/// Режим отправки отчёта об ошибке из настроек текущего пользователя.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorReportingMode {
    /// Платформа выбирает режим автоматически.
    Auto,
    /// Отчёт не отправляется.
    DontSend,
    /// Решение принимает пользователь.
    AskUser,
    /// Отчёт отправляется.
    Send,
}

impl ErrorReportingMode {
    fn value(self) -> Value {
        Value::Enum(match self {
            Self::Auto => EnumValue::ErrorReportingModeAuto,
            Self::DontSend => EnumValue::ErrorReportingModeDontSend,
            Self::AskUser => EnumValue::ErrorReportingModeAskUser,
            Self::Send => EnumValue::ErrorReportingModeSend,
        })
    }
}

/// Неизменяемые настройки обработки ошибок текущего пользователя.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorProcessingSettingsData {
    send_report_on_client: ErrorReportingMode,
    service_address: Arc<str>,
}

impl ErrorProcessingSettingsData {
    /// Создаёт снимок с режимом отправки и адресом сервиса регистрации.
    #[must_use]
    pub fn new(
        send_report_on_client: ErrorReportingMode,
        service_address: impl Into<String>,
    ) -> Self {
        Self {
            send_report_on_client,
            service_address: Arc::from(service_address.into()),
        }
    }

    /// Возвращает режим отправки отчёта на клиенте.
    #[must_use]
    pub const fn send_report_on_client(&self) -> ErrorReportingMode {
        self.send_report_on_client
    }

    /// Возвращает адрес сервиса обработки ошибок.
    #[must_use]
    pub fn service_address(&self) -> &str {
        &self.service_address
    }
}

/// Описание одного экрана, переданное хостом при открытии сеанса клиента.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClientDisplayInformation {
    width: u32,
    height: u32,
    dpi: u32,
}

/// Скорость соединения, которую хост назначил одному клиентскому сеансу.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientConnectionSpeed {
    /// Низкая скорость соединения.
    Low,
    /// Обычная скорость соединения.
    Normal,
}

impl ClientConnectionSpeed {
    const fn value(self) -> EnumValue {
        match self {
            Self::Low => EnumValue::ClientConnectionSpeedLow,
            Self::Normal => EnumValue::ClientConnectionSpeedNormal,
        }
    }
}

impl ClientDisplayInformation {
    /// Создаёт снимок размеров экрана и плотности пикселей.
    #[must_use]
    pub const fn new(width: u32, height: u32, dpi: u32) -> Self {
        Self { width, height, dpi }
    }

    /// Возвращает ширину экрана.
    #[must_use]
    pub const fn width(self) -> u32 {
        self.width
    }

    /// Возвращает высоту экрана.
    #[must_use]
    pub const fn height(self) -> u32 {
        self.height
    }

    /// Возвращает плотность экрана в точках на дюйм.
    #[must_use]
    pub const fn dpi(self) -> u32 {
        self.dpi
    }
}

/// Ресурсы одного открываемого сеанса.
pub struct SessionResources {
    connection: Box<dyn std::any::Any>,
    connection_number: u64,
    arguments: Vec<String>,
    session_number: Option<u64>,
    session_started: Option<BslDate>,
    data_access: Option<Rc<dyn DataAccess>>,
    connections: Option<Arc<[InfoBaseConnection]>>,
    event_log_writer: Option<Rc<dyn EventLogWriter>>,
    client_settings: Option<Rc<dyn ClientSettingsAdapter>>,
    error_processing_settings: Option<ErrorProcessingSettingsData>,
    database_configuration_changed_dynamically: Option<bool>,
    data_separation_common_attributes: Option<Arc<[Arc<str>]>>,
    client_displays: Option<Arc<[ClientDisplayInformation]>>,
    client_connection_speed: Option<ClientConnectionSpeed>,
    current_client_interface_variant: Option<EnumValue>,
    computer_name: Option<Arc<str>>,
    locale_code: Option<Arc<str>>,
    infobase_locale_code: Option<Arc<str>>,
    system_language: Option<Arc<str>>,
    external_resources_mode: Option<ExternalResourcesMode>,
}

impl SessionResources {
    /// Создаёт набор с отдельным соединением и его номером.
    #[must_use]
    pub fn new(connection: Box<dyn std::any::Any>, connection_number: u64) -> Self {
        Self {
            connection,
            connection_number,
            arguments: Vec::new(),
            session_number: None,
            session_started: None,
            data_access: None,
            connections: None,
            event_log_writer: None,
            client_settings: None,
            error_processing_settings: None,
            database_configuration_changed_dynamically: None,
            data_separation_common_attributes: None,
            client_displays: None,
            client_connection_speed: None,
            current_client_interface_variant: None,
            computer_name: None,
            locale_code: None,
            infobase_locale_code: None,
            system_language: None,
            external_resources_mode: None,
        }
    }

    /// Задаёт аргументы запуска тестового скрипта этого сеанса.
    #[must_use]
    pub fn with_arguments(mut self, arguments: Vec<String>) -> Self {
        self.arguments = arguments;
        self
    }

    /// Подключает адаптер данных этого сеанса.
    #[must_use]
    pub fn with_data_access(mut self, data_access: Rc<dyn DataAccess>) -> Self {
        self.data_access = Some(data_access);
        self
    }

    /// Подключает отдельный номер сеанса, назначенный хостом.
    ///
    /// Значение проверяется при открытии сеанса и не выводится из номера
    /// соединения с информационной базой.
    #[must_use]
    pub const fn with_session_number(mut self, number: u64) -> Self {
        self.session_number = Some(number);
        self
    }

    /// Задаёт неизменное время начала этого сеанса до его инициализации.
    ///
    /// Хост передаёт локальную дату своего сеанса. Значение не выводится
    /// из времени соединения с БД и не наследуется фоновым сеансом.
    #[must_use]
    pub const fn with_session_started(mut self, date: BslDate) -> Self {
        self.session_started = Some(date);
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

    /// Подключает снимок настроек обработки ошибок текущего пользователя.
    #[must_use]
    pub fn with_error_processing_settings(mut self, settings: ErrorProcessingSettingsData) -> Self {
        self.error_processing_settings = Some(settings);
        self
    }

    /// Подключает признак динамического обновления конфигурации базы данных
    /// во время жизни открываемого сеанса.
    #[must_use]
    pub const fn with_database_configuration_changed_dynamically(mut self, changed: bool) -> Self {
        self.database_configuration_changed_dynamically = Some(changed);
        self
    }

    /// Подключает имена общих реквизитов, используемых для разделения
    /// данных в конфигурации этого сеанса.
    #[must_use]
    pub fn with_data_separation_common_attributes(
        mut self,
        names: impl IntoIterator<Item = String>,
    ) -> Self {
        self.data_separation_common_attributes =
            Some(names.into_iter().map(Arc::<str>::from).collect());
        self
    }

    /// Подключает снимок экранов клиента в исходном порядке хоста.
    #[must_use]
    pub fn with_client_displays(
        mut self,
        displays: impl IntoIterator<Item = ClientDisplayInformation>,
    ) -> Self {
        self.client_displays = Some(displays.into_iter().collect());
        self
    }

    /// Подключает скорость соединения именно открываемого сеанса.
    #[must_use]
    pub const fn with_client_connection_speed(mut self, speed: ClientConnectionSpeed) -> Self {
        self.client_connection_speed = Some(speed);
        self
    }

    /// Передаёт вариант интерфейса именно открываемого сеанса.
    ///
    /// Значение должно быть членом `ВариантИнтерфейсаКлиентскогоПриложения`;
    /// фабрика проверит это до запуска кода конфигурации.
    #[must_use]
    pub const fn with_current_client_interface_variant(mut self, variant: EnumValue) -> Self {
        self.current_client_interface_variant = Some(variant);
        self
    }

    /// Подключает сетевое имя машины, исполняющей этот сеанс.
    #[must_use]
    pub fn with_computer_name(mut self, name: impl Into<Arc<str>>) -> Self {
        self.computer_name = Some(name.into());
        self
    }

    /// Задаёт код локализации только для открываемого сеанса.
    #[must_use]
    pub fn with_locale_code(mut self, code: impl Into<Arc<str>>) -> Self {
        self.locale_code = Some(code.into());
        self
    }

    /// Задаёт код локализации информационной базы для открываемого сеанса.
    #[must_use]
    pub fn with_infobase_locale_code(mut self, code: impl Into<Arc<str>>) -> Self {
        self.infobase_locale_code = Some(code.into());
        self
    }

    /// Задаёт язык системы только для открываемого сеанса.
    #[must_use]
    pub fn with_system_language(mut self, language: impl Into<Arc<str>>) -> Self {
        self.system_language = Some(language.into());
        self
    }

    /// Задаёт режим внешних ресурсов этого сеанса вместо стандартного `D`.
    #[must_use]
    pub fn with_external_resources_mode(mut self, mode: ExternalResourcesMode) -> Self {
        self.external_resources_mode = Some(mode);
        self
    }
}

/// Режим работы с внешними ресурсами текущего сеанса.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExternalResourcesMode {
    /// Стандартный режим `D`.
    Default,
    /// Альтернативный режим `A`.
    Alternative,
}

impl ExternalResourcesMode {
    const fn code(self) -> &'static str {
        match self {
            Self::Default => "D",
            // НЕ ИЗМЕРЕНО(SESSION.EXTERNAL_RESOURCES_ALTERNATIVE_MODE)
            Self::Alternative => "A",
        }
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
    session_number: Option<u64>,
    session_started: Option<BslDate>,
    connections: Option<Arc<[InfoBaseConnection]>>,
    event_log_writer: Option<Rc<dyn EventLogWriter>>,
    client_settings: Option<Rc<dyn ClientSettingsAdapter>>,
    error_processing_settings: Option<ErrorProcessingSettingsData>,
    database_configuration_changed_dynamically: Option<bool>,
    data_separation_common_attributes: Option<Arc<[Arc<str>]>>,
    client_displays: Option<Arc<[ClientDisplayInformation]>>,
    client_connection_speed: Option<ClientConnectionSpeed>,
    current_client_interface_variant: Option<EnumValue>,
    computer_name: Option<Arc<str>>,
    locale_code: Option<Arc<str>>,
    infobase_locale_code: Option<Arc<str>>,
    system_language: Option<Arc<str>>,
    external_resources_mode: Option<ExternalResourcesMode>,
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

    /// Возвращает номер текущего сеанса, если хост его передал.
    #[must_use]
    pub const fn session_number(&self) -> Option<u64> {
        self.session_number
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

    /// Возвращает настройки обработки ошибок текущего пользователя.
    #[must_use]
    pub const fn error_processing_settings(&self) -> Option<&ErrorProcessingSettingsData> {
        self.error_processing_settings.as_ref()
    }

    /// Возвращает переданный хостом признак динамического обновления
    /// конфигурации базы данных во время жизни этого сеанса.
    #[must_use]
    pub const fn database_configuration_changed_dynamically(&self) -> Option<bool> {
        self.database_configuration_changed_dynamically
    }

    /// Проверяет, принадлежит ли имя общему реквизиту разделения данных
    /// текущей конфигурации.
    #[must_use]
    pub fn is_data_separation_common_attribute(&self, name: &str) -> Option<bool> {
        self.data_separation_common_attributes
            .as_deref()
            .map(|names| names.iter().any(|known| folded_eq(known, name)))
    }

    /// Возвращает полное имя метода текущего фонового задания.
    #[must_use]
    pub fn background_method(&self) -> Option<&str> {
        self.background_method.as_deref()
    }

    /// Возвращает снимок экранов, если хост передал его сеансу клиента.
    #[must_use]
    pub fn client_displays(&self) -> Option<&[ClientDisplayInformation]> {
        self.client_displays.as_deref()
    }

    /// Возвращает скорость соединения, назначенную этому сеансу хостом.
    #[must_use]
    pub const fn client_connection_speed(&self) -> Option<ClientConnectionSpeed> {
        self.client_connection_speed
    }

    /// Возвращает сетевое имя машины текущего сеанса.
    #[must_use]
    pub fn computer_name(&self) -> Option<&str> {
        self.computer_name.as_deref()
    }

    /// Возвращает код локализации, заданный для этого сеанса.
    #[must_use]
    pub fn locale_code(&self) -> Option<&str> {
        self.locale_code.as_deref()
    }

    /// Возвращает код локализации информационной базы, переданный хостом.
    #[must_use]
    pub fn infobase_locale_code(&self) -> Option<&str> {
        self.infobase_locale_code.as_deref()
    }

    /// Возвращает язык системы, заданный для этого сеанса.
    #[must_use]
    pub fn system_language(&self) -> Option<&str> {
        self.system_language.as_deref()
    }

    /// Возвращает режим внешних ресурсов текущего сеанса.
    #[must_use]
    pub fn external_resources_mode(&self) -> &str {
        self.external_resources_mode
            .unwrap_or(ExternalResourcesMode::Default)
            .code()
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
#[derive(Clone)]
pub struct SessionFactory {
    engine: Engine,
    catalog: Arc<MetadataCatalog>,
    session_module: String,
    parameter_names: Arc<[String]>,
    separation_objects: Arc<[usize]>,
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
        Self::new_with_entry_sources(
            code,
            catalog,
            session_module,
            kind,
            platform_version,
            std::iter::empty::<String>(),
        )
    }

    /// Компилирует фабрику с зависимостями известных входных BSL-текстов.
    /// Тексты участвуют только в выборе модулей и не исполняются при создании
    /// фабрики. Входы не добавляются в исходник модуля сеанса.
    ///
    /// # Errors
    /// Возвращает ошибки [`Self::new`], а также синтаксические и семантические
    /// ошибки объявленных входных текстов при выборе замыкания.
    pub fn new_with_entry_sources(
        code: ConfigurationCodeSnapshot,
        catalog: Arc<MetadataCatalog>,
        session_module: &str,
        kind: SessionKind,
        platform_version: impl Into<String>,
        entry_sources: impl IntoIterator<Item = impl Into<String>>,
    ) -> Result<Self, SessionError> {
        Self::build_source(
            Arc::new(open_bsl::SnapshotSource::new(code)),
            catalog,
            session_module,
            kind,
            platform_version,
            entry_sources,
            false,
        )
    }

    /// Создаёт фабрику из индекса с подготовкой исходников по обращению.
    /// В начале подготавливаются модуль сеанса, входные тексты и их зависимости.
    ///
    /// # Errors
    /// Возвращает ошибки идентичности, объявлений, загрузки и подготовки
    /// достигнутых модулей либо необходимых метаданных.
    pub fn new_with_source(
        source: Arc<dyn open_bsl::ConfigurationSource>,
        catalog: Arc<MetadataCatalog>,
        session_module: &str,
        kind: SessionKind,
        platform_version: impl Into<String>,
        entry_sources: impl IntoIterator<Item = impl Into<String>>,
    ) -> Result<Self, SessionError> {
        Self::build_source(
            source,
            catalog,
            session_module,
            kind,
            platform_version,
            entry_sources,
            true,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn build_source(
        source: Arc<dyn open_bsl::ConfigurationSource>,
        catalog: Arc<MetadataCatalog>,
        session_module: &str,
        kind: SessionKind,
        platform_version: impl Into<String>,
        entry_sources: impl IntoIterator<Item = impl Into<String>>,
        on_demand: bool,
    ) -> Result<Self, SessionError> {
        let code = source.index();
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
            .filter(|module| {
                module.kind() != ModuleKind::Form
                    && (module.kind() != ModuleKind::Common || module.execution().server())
            })
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
                        // Модуль сеанса закрыт для BSL-импортов; хост
                        // вызывает обработчик в каталожном экземпляре `State`.
                        ModuleKind::Session => open_bsl::ModuleVisibility::Explicit,
                        _ => open_bsl::ModuleVisibility::Private,
                    },
                    name: module.name().into(),
                    source: if on_demand {
                        String::new()
                    } else {
                        source
                            .module_source(module.id())
                            .map_err(open_bsl::Error::Configuration)?
                            .to_string()
                    },
                    imports,
                })
            })
            .collect::<Result<Vec<_>, SessionError>>()?;
        // В индексном режиме динамические имена не становятся корнями:
        // их реальная цель подготавливается при первом обращении.
        let mut roots = vec![session_module.clone()];
        if !on_demand {
            roots.extend(
                modules
                    .iter()
                    .filter(|module| {
                        matches!(
                            module.visibility,
                            open_bsl::ModuleVisibility::Named | open_bsl::ModuleVisibility::Global
                        )
                    })
                    .map(|module| module.name.clone()),
            );
        }
        let mut builder = Engine::builder()
            .defer_common_module_name_errors(true)
            .register_library(bsl_orm::library())
            .register_library(library())
            // Фабрика исполняет модуль сеанса на сервере. Дефолтный
            // standalone-контекст фасада дополнительно включает внешнее
            // соединение, поэтому его ветви здесь нужно явно выключить.
            .preproc_symbol("ВнешнееСоединение", false)
            .configuration_roots(roots)
            .configuration_entry_sources(entry_sources);
        if !on_demand {
            builder = builder.configuration(ModuleGraphRecipe {
                modules,
                eager_init: false,
            });
        }
        if on_demand {
            builder = builder
                .configuration_source(source.clone())
                .configuration_context_source(Arc::new(source_context::SessionModuleContext::new(
                    source.index().clone(),
                    catalog.clone(),
                )));
        }
        for ty in catalog.transferable_value_types() {
            builder = builder.register_value_type(ty);
        }
        for ty in catalog.route_point_reference_types() {
            builder = builder.register_value_type(Arc::clone(ty));
        }
        for ty in catalog.catalog_selection_types() {
            builder = builder.register_value_type(Arc::clone(ty));
        }
        for ty in catalog.catalog_object_types() {
            builder = builder.register_value_type(Arc::clone(ty));
        }
        for ty in catalog.document_object_types() {
            builder = builder.register_value_type(Arc::clone(ty));
        }
        for ty in catalog
            .data_processor_object_types()
            .chain(catalog.data_processor_tabular_section_types())
            .chain(catalog.report_object_types())
            .chain(catalog.report_tabular_section_types())
        {
            builder = builder.register_value_type(Arc::clone(ty));
        }
        for ty in catalog.business_process_object_types() {
            builder = builder.register_value_type(Arc::clone(ty));
        }
        for ty in catalog.characteristic_object_types() {
            builder = builder.register_value_type(Arc::clone(ty));
        }
        for ty in catalog.information_record_set_types() {
            builder = builder.register_value_type(Arc::clone(ty));
        }
        for ty in catalog.document_tabular_section_types() {
            builder = builder.register_value_type(Arc::clone(ty));
        }
        for ty in catalog.business_process_tabular_section_types() {
            builder = builder.register_value_type(Arc::clone(ty));
        }
        for object in catalog.objects() {
            if object.kind() == MetadataKind::Catalog
                && let Some(module) = object.object_module()
            {
                builder = builder
                    .configuration_component_module("Справочники", object.name(), module)
                    .configuration_property_names(
                        module,
                        bsl_orm::catalog_object_receiver_properties()
                            .iter()
                            .flat_map(|property| property.names.iter().copied())
                            .map(str::to_owned)
                            .chain(
                                object
                                    .catalog_attribute_names()
                                    .into_iter()
                                    .flatten()
                                    .cloned(),
                            )
                            .chain(
                                object
                                    .catalog_common_attributes()
                                    .into_iter()
                                    .flatten()
                                    .map(|attribute| attribute.name().to_owned()),
                            )
                            .chain(
                                object
                                    .catalog_tabular_section_names()
                                    .into_iter()
                                    .flatten()
                                    .cloned(),
                            ),
                    )
                    .configuration_method_context(
                        module,
                        bsl_orm::catalog_object_receiver_methods(),
                    );
            }
            if object.kind() == MetadataKind::Document
                && let Some(module) = object.object_module()
            {
                builder = builder
                    .configuration_component_module("Документы", object.name(), module)
                    .configuration_property_names(
                        module,
                        bsl_orm::document_object_receiver_properties()
                            .iter()
                            .flat_map(|property| property.names.iter().copied())
                            .map(str::to_owned)
                            .chain(
                                object
                                    .document_attributes()
                                    .into_iter()
                                    .flatten()
                                    .map(|attribute| attribute.name().to_owned()),
                            )
                            .chain(
                                object
                                    .document_tabular_sections()
                                    .into_iter()
                                    .flatten()
                                    .map(|section| section.name().to_owned()),
                            ),
                    )
                    .configuration_method_context(
                        module,
                        bsl_orm::document_object_receiver_methods(),
                    );
            }
            if object.kind() == MetadataKind::BusinessProcess
                && let Some(module) = object.object_module()
            {
                builder = builder
                    .configuration_component_module("БизнесПроцессы", object.name(), module)
                    .configuration_property_names(
                        module,
                        bsl_orm::business_process_object_receiver_properties()
                            .iter()
                            .flat_map(|property| property.names.iter().copied())
                            .map(str::to_owned)
                            .chain(
                                object
                                    .business_process_attributes()
                                    .into_iter()
                                    .flatten()
                                    .map(|attribute| attribute.name().to_owned())
                                    .chain(
                                        object
                                            .business_process_tabular_sections()
                                            .into_iter()
                                            .flatten()
                                            .map(|section| section.name().to_owned()),
                                    ),
                            ),
                    )
                    .configuration_method_context(
                        module,
                        bsl_orm::business_process_object_receiver_methods(),
                    );
            }
            if matches!(
                object.kind(),
                MetadataKind::DataProcessor | MetadataKind::Report
            ) && let Some(module) = object.object_module()
            {
                let (global, attributes, sections) = if object.kind() == MetadataKind::Report {
                    (
                        "Отчеты",
                        object.report_attributes(),
                        object.report_tabular_sections(),
                    )
                } else {
                    (
                        "Обработки",
                        object.data_processor_attributes(),
                        object.data_processor_tabular_sections(),
                    )
                };
                let properties = if object.kind() == MetadataKind::Report {
                    bsl_orm::report_object_receiver_properties()
                } else {
                    &[]
                };
                builder = builder
                    .configuration_component_module(global, object.name(), module)
                    .configuration_property_names(
                        module,
                        properties
                            .iter()
                            .flat_map(|property| property.names.iter().copied())
                            .map(str::to_owned)
                            .chain(
                                attributes
                                    .into_iter()
                                    .flatten()
                                    .map(|attribute| attribute.name().to_owned()),
                            )
                            .chain(
                                sections
                                    .into_iter()
                                    .flatten()
                                    .map(|section| section.name().to_owned()),
                            ),
                    )
                    .configuration_method_context(
                        module,
                        bsl_orm::data_processor_object_receiver_methods(),
                    );
            }
            if object.kind() == MetadataKind::ChartOfCharacteristicTypes
                && let Some(module) = object.object_module()
            {
                builder = builder
                    .configuration_component_module(
                        "ПланыВидовХарактеристик",
                        object.name(),
                        module,
                    )
                    .configuration_property_names(
                        module,
                        bsl_orm::characteristic_object_receiver_properties()
                            .iter()
                            .flat_map(|property| property.names.iter().copied())
                            .map(str::to_owned)
                            .chain(
                                object
                                    .characteristic_attributes()
                                    .into_iter()
                                    .flatten()
                                    .map(|attribute| attribute.name().to_owned()),
                            ),
                    )
                    .configuration_method_context(
                        module,
                        bsl_orm::characteristic_object_receiver_methods(),
                    );
            }
            if let Some(module) = object.manager_module() {
                let global = object.kind().collection_name().ok_or_else(|| {
                    open_bsl::Error::Configuration(format!(
                        "объект «{}» не имеет коллекции менеджеров",
                        object.name()
                    ))
                })?;
                builder = builder.configuration_component_module(global, object.name(), module);
                if matches!(
                    object.kind(),
                    MetadataKind::Catalog
                        | MetadataKind::ChartOfCharacteristicTypes
                        | MetadataKind::ChartOfAccounts
                ) {
                    // Предопределённые имена являются свойствами собственного
                    // менеджера. Переменные сохраняют обычный приоритет ядра.
                    builder = builder.configuration_property_names(
                        module,
                        object.catalog_predefined_names().iter().cloned(),
                    );
                }
                if object.kind() == MetadataKind::BusinessProcess {
                    builder = builder
                        .configuration_property_names(module, ["ТочкиМаршрута", "RoutePoints"]);
                }
                let methods = object.kind().receiver_methods();
                if !methods.is_empty() {
                    builder = builder.configuration_method_context(module, methods);
                }
            }
        }
        let engine = builder.build()?;
        let parameter_names = catalog
            .objects()
            .iter()
            .filter(|object| object.kind() == MetadataKind::SessionParameter)
            .map(|object| object.name().to_string())
            .collect::<Vec<_>>()
            .into();
        // Состав каталога неизменяем. Обход всех объектов оплачивает фабрика,
        // а каждый новый сеанс посещает только связанные разделители.
        let separation_objects = catalog
            .objects()
            .iter()
            .enumerate()
            .filter_map(|(index, object)| {
                object
                    .common_attribute_separation_parameters()
                    .map(|_| index)
            })
            .collect::<Vec<_>>()
            .into();
        Ok(Self {
            engine,
            catalog,
            session_module,
            parameter_names,
            separation_objects,
            kind,
            platform_version: platform_version.into(),
            connection_string: None,
        })
    }

    /// Выбирает вид создаваемых сеансов с повторным использованием
    /// скомпилированного серверного контекста.
    /// Уже открытые сеансы сохраняют свой вид и состояние.
    #[must_use]
    pub fn with_kind(mut self, kind: SessionKind) -> Self {
        self.kind = kind;
        self
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
            arguments,
            session_number,
            session_started,
            data_access,
            connections,
            event_log_writer,
            client_settings,
            error_processing_settings,
            database_configuration_changed_dynamically,
            data_separation_common_attributes,
            client_displays,
            client_connection_speed,
            current_client_interface_variant,
            computer_name,
            locale_code,
            infobase_locale_code,
            system_language,
            external_resources_mode,
        } = resources;
        if connection_number == 0 || connection_number > i64::MAX as u64 {
            return Err(SessionError::InvalidConnectionNumber(connection_number));
        }
        if let Some(variant) = current_client_interface_variant
            && !matches!(
                variant,
                EnumValue::ClientApplicationInterfaceVariantTaxi
                    | EnumValue::ClientApplicationInterfaceVariantVersion8_2
            )
        {
            return Err(SessionError::InvalidClientInterfaceVariant);
        }
        if let Some(number) = session_number
            && (number == 0 || number > i64::MAX as u64)
        {
            return Err(SessionError::InvalidSessionNumber(number));
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
            None => {
                let parameters = SessionParameters::new(
                    self.session_module.clone(),
                    self.parameter_names.iter().cloned(),
                )?;
                // Платформа заполняет связанные параметры разделителя до обработчика.
                // Обычные параметры остаются незаполненными; унаследованные сюда не попадают.
                initialize_separation_parameters(
                    &self.catalog,
                    &self.separation_objects,
                    &parameters,
                )?;
                Rc::new(parameters)
            }
        };
        let runtime = SessionRuntime {
            parameters: Rc::clone(&parameters),
            kind: self.kind,
            platform_version: Arc::clone(&self.platform_version),
            connection_string: self.connection_string.clone(),
            connection_number,
            session_number,
            session_started,
            connections,
            event_log_writer,
            client_settings,
            error_processing_settings,
            database_configuration_changed_dynamically,
            data_separation_common_attributes,
            client_displays: if self.kind == SessionKind::BackgroundJob {
                None
            } else {
                client_displays
            },
            client_connection_speed,
            current_client_interface_variant,
            computer_name,
            locale_code,
            infobase_locale_code,
            system_language,
            external_resources_mode,
            background_method,
        };
        let system_language = runtime.system_language.clone();
        let mut data = SessionData::new(Arc::clone(&self.catalog), principal.clone())
            .with_extension(Rc::new(runtime));
        if let Some(language) = system_language {
            data = data.with_system_language(language);
        }
        if let Some(data_access) = data_access {
            data = data.with_data_access(data_access);
        }
        let mut state = self
            .engine
            .state_builder()
            .arguments(arguments)
            .message_sink(Rc::new(messages::SessionMessages::default()))
            .host_data(Rc::new(data))
            .build();
        // НЕ ИЗМЕРЕНО(SESSION.INIT.WEB_PRIVILEGE): автоматический режим
        // измерен для тонкого клиента и фонового задания; веб-вход требует
        // отдельного нативного контроля в опубликованной базе.
        state.call_configuration_handler(
            &self.session_module,
            "УстановкаПараметровСеанса",
            vec![Value::Undefined],
            open_bsl::ConfigurationHandlerEntryMode::Privileged,
        )?;
        Ok(Session {
            engine: self.engine.clone(),
            state,
            parameters,
            principal,
            connection,
            kind: self.kind,
        })
    }
}

fn initialize_separation_parameters(
    catalog: &MetadataCatalog,
    objects: &[usize],
    parameters: &SessionParameters,
) -> Result<(), SessionError> {
    for &index in objects {
        let object = &catalog.objects()[index];
        let Some(settings) = object.common_attribute_separation_parameters() else {
            continue;
        };
        let invalid = || {
            SessionError::OpenBsl(OpenBslError::Runtime(RtError::DynamicError(format!(
                "начальное значение параметра разделителя `{}` неизвестно",
                object.name()
            ))))
        };
        if let Some(id) = &settings.use_parameter {
            let parameter = catalog.object_by_id(id).ok_or_else(invalid)?;
            parameters.set(parameter.name(), Value::Boolean(false))?;
        }
        if let Some(id) = &settings.value_parameter {
            let parameter = catalog.object_by_id(id).ok_or_else(invalid)?;
            let value = match settings.value_types.as_deref() {
                Some([kind]) if kind.id == open_bsl::TypeRef::Native(open_bsl::TypeId::String) => {
                    Value::Str("".into())
                }
                Some([kind]) if kind.id == open_bsl::TypeRef::Native(open_bsl::TypeId::Number) => {
                    Value::number_from_i64(0)
                }
                _ => return Err(invalid()),
            };
            parameters.set(parameter.name(), value)?;
        }
    }
    Ok(())
}

/// Один запущенный и изолированный сеанс конфигурации.
pub struct Session {
    engine: Engine,
    state: State,
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
    /// Номер сеанса не является положительным числом, представимым в BSL.
    InvalidSessionNumber(u64),
    /// Вариант интерфейса не принадлежит соответствующему перечислению.
    InvalidClientInterfaceVariant,
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
            Self::InvalidSessionNumber(number) => {
                write!(
                    formatter,
                    "номер сеанса информационной базы недопустим: {number}"
                )
            }
            Self::InvalidClientInterfaceVariant => {
                formatter.write_str("недопустимый вариант интерфейса клиентского приложения")
            }
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
    session_number: Option<u64>,
    session_started: Option<BslDate>,
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

#[derive(Debug)]
struct ErrorProcessingManagerObject;

#[derive(Debug)]
struct ErrorProcessingSettingsObject {
    data: ErrorProcessingSettingsData,
}

#[derive(Debug)]
struct SettingsDescriptionObject {
    object_key: RefCell<Value>,
    settings_key: RefCell<Value>,
    user: RefCell<Value>,
    presentation: RefCell<Value>,
    additional_properties: Value,
}

#[derive(Debug)]
struct JobScheduleObject {
    completion_time: RefCell<Value>,
    end_time: RefCell<Value>,
    begin_time: RefCell<Value>,
    end_date: RefCell<Value>,
    begin_date: RefCell<Value>,
    day_in_month: RefCell<Value>,
    week_day_in_month: RefCell<Value>,
    detailed_daily_schedules: RefCell<Value>,
    week_days: RefCell<Value>,
    completion_interval: RefCell<Value>,
    months: RefCell<Value>,
    repeat_pause: RefCell<Value>,
    weeks_period: RefCell<Value>,
    repeat_period_in_day: RefCell<Value>,
    days_repeat_period: RefCell<Value>,
}

#[derive(Debug)]
struct UnsafeOperationProtectionDescriptionObject {
    warnings: Cell<bool>,
}

#[derive(Debug)]
struct EventLogEventUseObject {
    use_event: Cell<bool>,
    description: RefCell<Value>,
}

#[derive(Debug)]
struct EventLogAccessEventUseDescriptionObject {
    object: RefCell<Arc<str>>,
    access_fields: RefCell<Value>,
    logged_fields: RefCell<Value>,
}

#[derive(Debug)]
struct ClientSettingsObject {
    show_navigation_and_actions_panels: Cell<bool>,
    show_sections_panel: Cell<bool>,
    forms_openning_mode: Cell<EnumValue>,
    interface_variant: Cell<EnumValue>,
}

#[derive(Debug)]
struct ClientApplicationObject {
    kind: SessionKind,
}

#[derive(Debug)]
struct CommandInterfaceSettingsObject {
    sections_panel_representation: Cell<EnumValue>,
}

#[derive(Debug)]
struct ClientApplicationInterfaceSettingsObject {
    content: RefCell<Option<Value>>,
}

#[derive(Debug)]
struct ClientApplicationInterfaceContentSettingsObject {
    top: Value,
    left: Value,
    bottom: Value,
    right: Value,
}

#[derive(Debug)]
struct ClientApplicationInterfaceContentSettingsGroupObject {
    items: RefCell<Vec<Value>>,
}

#[derive(Debug)]
struct ClientApplicationInterfaceContentSettingsItemObject {
    name: Arc<str>,
}

#[derive(Debug)]
struct ClientDisplayInformationObject {
    display: ClientDisplayInformation,
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
static COLLABORATION_CONVERSATION_ID_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-session",
    name: "ИдентификаторОбсужденияСистемыВзаимодействия",
    type_display: "CollaborationSystemConversationID",
    type_names: &["CollaborationSystemConversationID"],
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
static ERROR_PROCESSING_MANAGER_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-session",
    name: "МенеджерОбработкиОшибок",
    type_display: "Error processing manager",
    type_names: &["ErrorProcessingManager"],
};
static ERROR_PROCESSING_SETTINGS_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-session",
    name: "НастройкиОбработкиОшибок",
    type_display: "Settings of error processing on the client",
    type_names: &["ErrorProcessingSettings"],
};
static SETTINGS_DESCRIPTION_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-session",
    name: "ОписаниеНастроек",
    type_display: "Setting description",
    type_names: &["SettingsDescription"],
};
static JOB_SCHEDULE_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-session",
    name: "РасписаниеРегламентногоЗадания",
    type_display: "Job schedule",
    type_names: &["JobSchedule"],
};
static UNSAFE_OPERATION_PROTECTION_DESCRIPTION_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-session",
    name: "ОписаниеЗащитыОтОпасныхДействий",
    type_display: "Unsafe operation protection description",
    type_names: &["UnsafeOperationProtectionDescription"],
};
static EVENT_LOG_EVENT_USE_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-session",
    name: "ИспользованиеСобытияЖурналаРегистрации",
    type_display: "Event log event use",
    type_names: &["EventLogEventUse"],
};
static EVENT_LOG_ACCESS_EVENT_USE_DESCRIPTION_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-session",
    name: "ОписаниеИспользованияСобытияДоступЖурналаРегистрации",
    type_display: "Description of event log access event use",
    type_names: &["EventLogAccessEventUseDescription"],
};
static CLIENT_SETTINGS_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-session",
    name: "НастройкиКлиентскогоПриложения",
    type_display: "Client application settings",
    type_names: &["ClientSettings"],
};
static CLIENT_APPLICATION_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-session",
    name: "КлиентскоеПриложение",
    type_display: "Client application",
    type_names: &["ClientApplication"],
};
static COMMAND_INTERFACE_SETTINGS_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-session",
    name: "НастройкиКомандногоИнтерфейса",
    type_display: "Command interface settings",
    type_names: &["CommandInterfaceSettings"],
};
static CLIENT_APPLICATION_INTERFACE_SETTINGS_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-session",
    name: "НастройкиИнтерфейсаКлиентскогоПриложения",
    type_display: "Panel settings",
    type_names: &["ClientApplicationInterfaceSettings"],
};
static CLIENT_APPLICATION_INTERFACE_CONTENT_SETTINGS_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-session",
    name: "НастройкиСоставаИнтерфейсаКлиентскогоПриложения",
    type_display: "Client application interface composition settings",
    type_names: &["ClientApplicationInterfaceContentSettings"],
};
static CLIENT_APPLICATION_INTERFACE_CONTENT_SETTINGS_GROUP_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-session",
    name: "ГруппаНастройкиСоставаИнтерфейсаКлиентскогоПриложения",
    type_display: "Client application interface composition settings group",
    type_names: &["ClientApplicationInterfaceContentSettingsGroup"],
};
static CLIENT_APPLICATION_INTERFACE_CONTENT_SETTINGS_ITEM_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-session",
    name: "ЭлементНастройкиСоставаИнтерфейсаКлиентскогоПриложения",
    type_display: "Client application interface composition settings item",
    type_names: &["ClientApplicationInterfaceContentSettingsItem"],
};
static CLIENT_DISPLAY_INFORMATION_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-session",
    name: "ИнформацияЭкранаКлиента",
    type_display: "ClientDisplayInformation",
    type_names: &["ClientDisplayInformation"],
};

impl ObjectProtocol for ClientDisplayInformationObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &CLIENT_DISPLAY_INFORMATION_TYPE
    }

    fn property_table(&self) -> &'static [PropertyDescriptor] {
        CLIENT_DISPLAY_INFORMATION_PROPERTIES
    }
}

fn client_display_information(
    receiver: &dyn ObjectProtocol,
) -> RtResult<&ClientDisplayInformationObject> {
    receiver
        .downcast_ref::<ClientDisplayInformationObject>()
        .ok_or(RtError::NotAnObject)
}

fn client_display_width(receiver: &dyn ObjectProtocol, _: &mut CallContext<'_>) -> RtResult<Value> {
    Ok(Value::number_from_i64(
        client_display_information(receiver)?.display.width() as i64,
    ))
}

fn client_display_height(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    Ok(Value::number_from_i64(
        client_display_information(receiver)?.display.height() as i64,
    ))
}

fn client_display_dpi(receiver: &dyn ObjectProtocol, _: &mut CallContext<'_>) -> RtResult<Value> {
    Ok(Value::number_from_i64(
        client_display_information(receiver)?.display.dpi() as i64,
    ))
}

static CLIENT_DISPLAY_INFORMATION_PROPERTIES: &[PropertyDescriptor] = &[
    PropertyDescriptor {
        names: &["Ширина", "Width"],
        get: client_display_width,
        set: None,
    },
    PropertyDescriptor {
        names: &["Высота", "Height"],
        get: client_display_height,
        set: None,
    },
    PropertyDescriptor {
        names: &["DPI"],
        get: client_display_dpi,
        set: None,
    },
];

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

    fn property_table(&self) -> &'static [PropertyDescriptor] {
        INFOBASE_SESSION_PROPERTIES
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

impl ObjectProtocol for SettingsDescriptionObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &SETTINGS_DESCRIPTION_TYPE
    }

    fn property_table(&self) -> &'static [PropertyDescriptor] {
        SETTINGS_DESCRIPTION_PROPERTIES
    }
}

impl ObjectProtocol for JobScheduleObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &JOB_SCHEDULE_TYPE
    }

    fn property_table(&self) -> &'static [PropertyDescriptor] {
        JOB_SCHEDULE_PROPERTIES
    }
}

impl ObjectProtocol for UnsafeOperationProtectionDescriptionObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &UNSAFE_OPERATION_PROTECTION_DESCRIPTION_TYPE
    }

    fn property_table(&self) -> &'static [PropertyDescriptor] {
        UNSAFE_OPERATION_PROTECTION_DESCRIPTION_PROPERTIES
    }
}

impl ObjectProtocol for EventLogEventUseObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &EVENT_LOG_EVENT_USE_TYPE
    }

    fn property_table(&self) -> &'static [PropertyDescriptor] {
        EVENT_LOG_EVENT_USE_PROPERTIES
    }
}

impl ObjectProtocol for EventLogAccessEventUseDescriptionObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &EVENT_LOG_ACCESS_EVENT_USE_DESCRIPTION_TYPE
    }

    fn property_table(&self) -> &'static [PropertyDescriptor] {
        EVENT_LOG_ACCESS_EVENT_USE_DESCRIPTION_PROPERTIES
    }
}

impl ObjectProtocol for ClientSettingsObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &CLIENT_SETTINGS_TYPE
    }

    fn property_table(&self) -> &'static [PropertyDescriptor] {
        CLIENT_SETTINGS_PROPERTIES
    }
}

impl ObjectProtocol for ClientApplicationObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &CLIENT_APPLICATION_TYPE
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        CLIENT_APPLICATION_METHODS
    }
}

impl ObjectProtocol for CommandInterfaceSettingsObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &COMMAND_INTERFACE_SETTINGS_TYPE
    }

    fn property_table(&self) -> &'static [PropertyDescriptor] {
        COMMAND_INTERFACE_SETTINGS_PROPERTIES
    }
}

impl ObjectProtocol for ClientApplicationInterfaceSettingsObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &CLIENT_APPLICATION_INTERFACE_SETTINGS_TYPE
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        CLIENT_APPLICATION_INTERFACE_SETTINGS_METHODS
    }
}

impl ObjectProtocol for ClientApplicationInterfaceContentSettingsObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &CLIENT_APPLICATION_INTERFACE_CONTENT_SETTINGS_TYPE
    }

    fn property_table(&self) -> &'static [PropertyDescriptor] {
        CLIENT_APPLICATION_INTERFACE_CONTENT_SETTINGS_PROPERTIES
    }
}

impl ObjectProtocol for ClientApplicationInterfaceContentSettingsGroupObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &CLIENT_APPLICATION_INTERFACE_CONTENT_SETTINGS_GROUP_TYPE
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        CLIENT_APPLICATION_INTERFACE_CONTENT_SETTINGS_GROUP_METHODS
    }
}

impl ObjectProtocol for ClientApplicationInterfaceContentSettingsItemObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &CLIENT_APPLICATION_INTERFACE_CONTENT_SETTINGS_ITEM_TYPE
    }

    fn property_table(&self) -> &'static [PropertyDescriptor] {
        CLIENT_APPLICATION_INTERFACE_CONTENT_SETTINGS_ITEM_PROPERTIES
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

impl ObjectProtocol for ErrorProcessingManagerObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &ERROR_PROCESSING_MANAGER_TYPE
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        ERROR_PROCESSING_MANAGER_METHODS
    }
}

impl ObjectProtocol for ErrorProcessingSettingsObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &ERROR_PROCESSING_SETTINGS_TYPE
    }

    fn property_table(&self) -> &'static [PropertyDescriptor] {
        ERROR_PROCESSING_SETTINGS_PROPERTIES
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

fn client_application(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    Ok(Value::new_object(ClientApplicationObject {
        kind: runtime(ctx)?.kind(),
    }))
}

fn client_application_type(
    receiver: &dyn ObjectProtocol,
    _: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let application = receiver
        .downcast_ref::<ClientApplicationObject>()
        .ok_or(RtError::NotAnObject)?;
    match application.kind {
        // НЕ ИЗМЕРЕНО(SESSION.CLIENT_APPLICATION.WEB_TYPE) — для веб-клиента
        // пока нет прямого платформенного прогона.
        SessionKind::WebClient => Ok(Value::Enum(EnumValue::ClientApplicationTypeWebClient)),
        SessionKind::ThinClient => Ok(Value::Enum(EnumValue::ClientApplicationTypeThinClient)),
        SessionKind::BackgroundJob => Ok(Value::Undefined),
    }
}

fn client_displays_information(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    let runtime = runtime(ctx)?;
    if runtime.kind() == SessionKind::BackgroundJob {
        return Ok(Value::Undefined);
    }
    let displays = runtime
        .client_displays()
        .ok_or_else(|| RtError::DynamicError("снимок экранов клиента не передан хостом".into()))?;
    Ok(fixed_array_from_values(
        displays
            .iter()
            .map(|display| Value::new_object(ClientDisplayInformationObject { display: *display }))
            .collect(),
    ))
}

fn computer_name(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    let name = runtime(ctx)?
        .computer_name()
        .ok_or_else(|| RtError::DynamicError("сетевое имя компьютера не передано хостом".into()))?;
    Ok(Value::Str(name.into()))
}

fn current_locale_code(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    let code = runtime(ctx)?
        .locale_code()
        .ok_or_else(|| RtError::DynamicError("код локализации не передан хостом".into()))?;
    Ok(Value::Str(code.into()))
}

fn infobase_locale_code(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    let code = runtime(ctx)?.infobase_locale_code().ok_or_else(|| {
        RtError::DynamicError("код локализации информационной базы не передан хостом".into())
    })?;
    Ok(Value::Str(code.into()))
}

fn current_system_language(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    let language = runtime(ctx)?
        .system_language()
        .ok_or_else(|| RtError::DynamicError("язык системы не передан хостом".into()))?;
    Ok(Value::Str(language.into()))
}

fn get_external_resources_mode(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    Ok(Value::Str(runtime(ctx)?.external_resources_mode().into()))
}

fn client_connection_speed(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    let speed = runtime(ctx)?.client_connection_speed().ok_or_else(|| {
        RtError::DynamicError("скорость клиентского соединения не передана хостом".into())
    })?;
    Ok(Value::Enum(speed.value()))
}

fn current_client_interface_variant(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    let variant = runtime(ctx)?
        .current_client_interface_variant
        .ok_or_else(|| {
            RtError::DynamicError("вариант интерфейса клиента не передан хостом".into())
        })?;
    Ok(Value::Enum(variant))
}

fn infobase_sessions(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    // Список сеансов требует права `АктивныеПользователи` и живого снимка
    // сервера; текущий сеанс не заменяет такой список.
    Err(RtError::DynamicError(
        "список сеансов информационной базы не предоставлен хостом".into(),
    ))
}

fn scheduled_jobs(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    // Регламентные задания требуют отдельного планировщика; фоновые
    // задания не являются его заменой.
    Err(RtError::DynamicError(
        "регламентные задания не поддерживаются этим хостом".into(),
    ))
}

fn collaboration_system(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    Err(RtError::DynamicError(
        "система взаимодействия не предоставлена хостом".into(),
    ))
}

fn construct_collaboration_filter(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    Err(RtError::DynamicError(
        "отбор системы взаимодействия не поддерживается этим хостом".into(),
    ))
}

fn construct_collaboration_conversation_id(
    _: &mut CallContext<'_>,
    _: &[Value],
) -> RtResult<Value> {
    // Идентификатор нельзя подменять строкой: это отдельный тип платформы.
    Err(RtError::DynamicError(
        "идентификатор обсуждения системы взаимодействия не поддерживается этим хостом".into(),
    ))
}

fn construct_collaboration_conversation_context(
    _: &mut CallContext<'_>,
    _: &[Value],
) -> RtResult<Value> {
    Err(RtError::DynamicError(
        "контекст обсуждения системы взаимодействия не поддерживается этим хостом".into(),
    ))
}

fn construct_collaboration_application(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    // Идентификаторы и коллекции приложений — типизированные значения
    // системы взаимодействия; без её сервиса их нельзя заменять строкой.
    Err(RtError::DynamicError(
        "объекты приложений системы взаимодействия не поддерживаются этим хостом".into(),
    ))
}

fn construct_server_administration(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    // На платформе конструктор сразу подключается к ras. Декларация
    // позволяет связать исходный модуль, но не подменяет сетевое соединение.
    Err(RtError::DynamicError(
        "транспорт администрирования сервера ras не подключён".into(),
    ))
}

fn construct_internet_mail_message(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    Err(RtError::DynamicError(
        "интернет-почтовое сообщение не поддерживается этим хостом".into(),
    ))
}

fn infobase_connection_string(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    let value = runtime(ctx)?.connection_string().ok_or_else(|| {
        RtError::DynamicError("строка соединения с информационной базой не передана хостом".into())
    })?;
    Ok(Value::Str(value.into()))
}

fn infobase_url(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    // Строка SQL-соединения не содержит клиентский адрес запуска базы.
    // Подмена её навигационной ссылкой дала бы вызывающему ложный URL.
    Err(RtError::DynamicError(
        "навигационная ссылка информационной базы не предоставлена хостом".into(),
    ))
}

fn infobase_connection_number(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    Ok(Value::number_from_i64(
        runtime(ctx)?.connection_number() as i64
    ))
}

fn infobase_session_number(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    let number = runtime(ctx)?.session_number().ok_or_else(|| {
        RtError::DynamicError("номер сеанса информационной базы не передан хостом".into())
    })?;
    Ok(Value::number_from_i64(number as i64))
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
    let entry = EventLogEntry::new(
        event_name,
        event_log_level(arguments.get(1))?,
        arguments.get(2).cloned().unwrap_or(Value::Undefined),
        arguments.get(3).cloned().unwrap_or(Value::Undefined),
        comment,
        event_log_transaction_mode(arguments.get(5))?,
    );
    let writer = Rc::clone(runtime(ctx)?.event_log_writer().ok_or_else(|| {
        RtError::DynamicError("журнал регистрации не подключён для текущего сеанса".into())
    })?);
    writer.write(entry)?;
    Ok(Value::Undefined)
}

fn event_log_event_name(value: &Value, operation: &'static str) -> RtResult<String> {
    let Value::Str(name) = value else {
        return Err(RtError::TypeError {
            expected: "Строка",
            op: operation,
        });
    };
    Ok(name.to_string())
}

fn get_event_log_event_use(ctx: &mut CallContext<'_>, arguments: &[Value]) -> RtResult<Value> {
    let name = event_log_event_name(
        &arguments[0],
        "ПолучитьИспользованиеСобытияЖурналаРегистрации",
    )?;
    let writer = Rc::clone(runtime(ctx)?.event_log_writer().ok_or_else(|| {
        RtError::DynamicError("журнал регистрации не подключён для текущего сеанса".into())
    })?);
    let setting = writer.event_use(&name)?;
    Ok(Value::new_object(EventLogEventUseObject {
        use_event: Cell::new(setting.use_event()),
        description: RefCell::new(setting.description().clone()),
    }))
}

fn set_event_log_event_use(ctx: &mut CallContext<'_>, arguments: &[Value]) -> RtResult<Value> {
    let name = event_log_event_name(
        &arguments[0],
        "УстановитьИспользованиеСобытияЖурналаРегистрации",
    )?;
    let object = arguments[1]
        .object_ref()
        .and_then(|object| object.downcast_ref::<EventLogEventUseObject>())
        .ok_or(RtError::TypeError {
            expected: "ИспользованиеСобытияЖурналаРегистрации",
            op: "УстановитьИспользованиеСобытияЖурналаРегистрации",
        })?;
    let setting =
        EventLogEventUseData::new(object.use_event.get(), object.description.borrow().clone());
    let writer = Rc::clone(runtime(ctx)?.event_log_writer().ok_or_else(|| {
        RtError::DynamicError("журнал регистрации не подключён для текущего сеанса".into())
    })?);
    writer.set_event_use(&name, &setting)?;
    Ok(Value::Undefined)
}

fn get_event_log_using(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    let writer = Rc::clone(runtime(ctx)?.event_log_writer().ok_or_else(|| {
        RtError::DynamicError("журнал регистрации не подключён для текущего сеанса".into())
    })?);
    Ok(Value::new_array(
        writer
            .enabled_levels()?
            .into_iter()
            .map(EventLogLevel::value)
            .collect(),
    ))
}

fn unload_event_log(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    // Запись отдельных событий не даёт доступа к чтению и выгрузке журнала.
    Err(RtError::DynamicError(
        "выгрузка журнала регистрации не поддерживается этим хостом".into(),
    ))
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
    let principal = principal.clone();
    Ok(adapter
        .load_with_context(&principal, &request, ctx)?
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

fn list_client_settings(
    receiver: &dyn ObjectProtocol,
    arguments: &[Value],
    ctx: &mut CallContext<'_>,
) -> RtResult<Value> {
    const OPERATION: &str = "ПолучитьСписок";
    let request = ClientSettingsListRequest {
        storage: settings_storage(receiver)?.storage,
        object_key: settings_text(&arguments[0], OPERATION)?,
        user_name: optional_user_name(arguments, 1, OPERATION)?,
    };
    let (principal, adapter) = client_settings_context(ctx)?;
    let keys = adapter.list(principal, &request)?;
    Ok(value_list_from_values(
        keys.into_iter().map(|key| Value::Str(key.as_str().into())),
    ))
}

fn describe_client_setting(
    receiver: &dyn ObjectProtocol,
    arguments: &[Value],
    ctx: &mut CallContext<'_>,
) -> RtResult<Value> {
    const OPERATION: &str = "ПолучитьОписание";
    let request = ClientSettingsDescriptionRequest {
        storage: settings_storage(receiver)?.storage,
        object_key: settings_text(&arguments[0], OPERATION)?,
        settings_key: optional_settings_key(arguments, OPERATION)?,
        user_name: optional_user_name(arguments, 2, OPERATION)?,
    };
    let (principal, adapter) = client_settings_context(ctx)?;
    let description = adapter.description(principal, &request)?;
    Ok(Value::new_object(SettingsDescriptionObject {
        object_key: RefCell::new(description.object_key),
        settings_key: RefCell::new(description.settings_key),
        user: RefCell::new(description.user),
        presentation: RefCell::new(description.presentation),
        additional_properties: description.additional_properties,
    }))
}

fn delete_selector(value: &Value, operation: &'static str) -> RtResult<Option<Arc<str>>> {
    match value {
        Value::Undefined => Ok(None),
        value => settings_text(value, operation).map(Some),
    }
}

fn delete_client_settings(
    receiver: &dyn ObjectProtocol,
    arguments: &[Value],
    ctx: &mut CallContext<'_>,
) -> RtResult<Value> {
    const OPERATION: &str = "Удалить";
    let request = ClientSettingsDeleteRequest {
        storage: settings_storage(receiver)?.storage,
        object_key: delete_selector(&arguments[0], OPERATION)?,
        settings_key: delete_selector(&arguments[1], OPERATION)?,
        user_name: delete_selector(&arguments[2], OPERATION)?,
    };
    let (principal, adapter) = client_settings_context(ctx)?;
    adapter.delete(principal, &request)?;
    Ok(Value::Undefined)
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

fn form_data_settings_storage(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    Ok(Value::new_object(StandardSettingsStorageObject {
        storage: ClientSettingsStorage::FormData,
    }))
}

fn reports_user_settings_storage(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    Ok(Value::new_object(StandardSettingsStorageObject {
        storage: ClientSettingsStorage::ReportsUser,
    }))
}

fn reports_variants_storage(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    // В конфигурации может быть выбрано собственное хранилище вариантов.
    // Пока снимок не сообщает этот выбор, стандартное было бы догадкой.
    Err(RtError::DynamicError(
        "хранилище вариантов отчетов не определено в снимке конфигурации".into(),
    ))
}

fn user_work_history(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    // История работы связана с текущим пользователем и его правом сохранения
    // данных; отсутствие сервиса нельзя подменять пустым менеджером.
    Err(RtError::DynamicError(
        "история работы пользователя не предоставлена текущему сеансу".into(),
    ))
}

fn deliverable_notification_send(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    // Отправка уведомлений требует отдельного транспорта; пустой менеджер
    // скрыл бы отсутствие сервиса текущего сеанса.
    Err(RtError::DynamicError(
        "отправка доставляемых уведомлений не предоставлена текущему сеансу".into(),
    ))
}

fn crypto_tools(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    // Без поставщика криптографии нельзя подменять сведения о его модуле
    // пустым менеджером или данными другого сеанса.
    Err(RtError::DynamicError(
        "средства криптографии не предоставлены текущему сеансу".into(),
    ))
}

fn get_infobase_time_zone(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    // Пояс информационной базы — отдельная настройка, которую нельзя
    // заменять часовым поясом компьютера или текущего сеанса.
    Err(RtError::DynamicError(
        "часовой пояс информационной базы не предоставлен текущему сеансу".into(),
    ))
}

fn find_marked_for_deletion(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    // Поиск обязан учитывать права чтения, RLS и разделение данных текущего
    // сеанса; пустой массив выдал бы отсутствие доступа за отсутствие данных.
    Err(RtError::DynamicError(
        "поиск помеченных на удаление объектов не поддерживается текущим сеансом".into(),
    ))
}

fn access_parameters(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    // Результат зависит от полей объекта, прав пользователя или роли и RLS.
    // Пустая структура ошибочно означала бы отсутствие ограничений.
    Err(RtError::DynamicError(
        "параметры доступа для текущего сеанса пока не поддерживаются".into(),
    ))
}

fn available_locale_codes(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    // Список локалей платформы не выводится из локали текущего сеанса.
    Err(RtError::DynamicError(
        "доступные коды локализации не предоставлены текущему сеансу".into(),
    ))
}

fn full_text_search(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    // Индекс, права и разделение данных принадлежат текущему сеансу.
    // Пустой менеджер сообщил бы, что разрешённый поиск ничего не нашёл.
    Err(RtError::DynamicError(
        "полнотекстовый поиск не предоставлен текущему сеансу".into(),
    ))
}

fn additional_authentication_settings(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    // Настройки аутентификации принадлежат информационной базе и доступны
    // только с её разрешениями. Пустой менеджер скрыл бы отсутствие данных.
    Err(RtError::DynamicError(
        "дополнительные настройки аутентификации не предоставлены текущему сеансу".into(),
    ))
}

fn get_server_path_separator(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    let separator = ctx
        .files()?
        .path_separator()
        .map_err(|error| RtError::IoError(error.to_string()))?;
    Ok(Value::Str(separator.as_str().into()))
}

fn get_licensing_client_name(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    // Имя хранится в информационной базе и доступно только администратору;
    // значение процесса или пустая строка подменили бы проверку прав.
    Err(RtError::DynamicError(
        "имя клиента лицензирования не предоставлено текущему сеансу".into(),
    ))
}

fn get_configuration_id(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    // Платформенный идентификатор зашифрован; GUID метаданных и отпечаток
    // ресурсов не могут заменить его для проверки лицензии.
    Err(RtError::DynamicError(
        "идентификатор конфигурации не предоставлен текущему сеансу".into(),
    ))
}

fn get_licensing_client_additional_parameter(
    _: &mut CallContext<'_>,
    _: &[Value],
) -> RtResult<Value> {
    // Дополнительный параметр хранится в базе и доступен администратору.
    Err(RtError::DynamicError(
        "дополнительный параметр клиента лицензирования не предоставлен текущему сеансу".into(),
    ))
}

fn set_licensing_client_parameters(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    // Успех без записи в базу ложно подтвердил бы смену учётных данных.
    Err(RtError::DynamicError(
        "запись настроек клиента лицензирования не предоставлена текущему сеансу".into(),
    ))
}

fn locale_code_presentation(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    // Представление зависит от платформенного каталога локалей.
    Err(RtError::DynamicError(
        "представление кода локализации не предоставлено текущему сеансу".into(),
    ))
}

fn dynamic_lists_user_settings_storage(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    Ok(Value::new_object(StandardSettingsStorageObject {
        storage: ClientSettingsStorage::DynamicListsUser,
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

fn set_safe_mode_disabled(ctx: &mut CallContext<'_>, arguments: &[Value]) -> RtResult<Value> {
    ctx.set_safe_mode_disabled(security_mode_flag(
        arguments,
        "УстановитьОтключениеБезопасногоРежима",
    )?)?;
    Ok(Value::Undefined)
}

fn get_safe_mode_disabled(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    Ok(Value::Boolean(ctx.safe_mode_disabled()?))
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

fn set_data_separation_safe_mode(
    ctx: &mut CallContext<'_>,
    arguments: &[Value],
) -> RtResult<Value> {
    let Value::Str(common_attribute) = &arguments[0] else {
        return Err(RtError::TypeError {
            expected: "Строка",
            op: "УстановитьБезопасныйРежимРазделенияДанных",
        });
    };
    // Флаг использует измеренное обычное преобразование в Булево;
    // ошибка преобразования предшествует поиску имени и изменению счётчика.
    let enabled = arguments[1]
        .as_condition()
        .map_err(|_| RtError::TypeError {
            expected: "Булево",
            op: "УстановитьБезопасныйРежимРазделенияДанных",
        })?;
    let common_attribute =
        checked_data_separation_common_attribute(ctx, common_attribute.shared_utf8())?;
    // Счётчики кадров, вложенная видимость и очистка измерены на конфигурации
    // с разделяющим общим реквизитом; наблюдение хоста не подменяет геттер зоны.
    ctx.set_data_separation_safe_mode(&common_attribute, enabled)?;
    Ok(Value::Undefined)
}

fn checked_data_separation_common_attribute(
    ctx: &CallContext<'_>,
    common_attribute: Rc<str>,
) -> RtResult<Rc<str>> {
    let runtime = session_data(ctx)?
        .extension::<SessionRuntime>()
        .ok_or_else(|| RtError::DynamicError("runtime сеанса не подключён".into()))?;
    match runtime.is_data_separation_common_attribute(&common_attribute) {
        Some(true) => {}
        Some(false) => {
            return Err(RtError::DynamicError(format!(
                "общий реквизит разделения данных «{common_attribute}» не найден"
            )));
        }
        None => {
            return Err(RtError::DynamicError(
                "общие реквизиты разделения данных не переданы хостом".into(),
            ));
        }
    }
    Ok(common_attribute)
}

fn data_separation_safe_mode(ctx: &mut CallContext<'_>, arguments: &[Value]) -> RtResult<Value> {
    let Value::Str(common_attribute) = &arguments[0] else {
        return Err(RtError::TypeError {
            expected: "Строка",
            op: "БезопасныйРежимРазделенияДанных",
        });
    };
    checked_data_separation_common_attribute(ctx, common_attribute.shared_utf8())?;
    // Счётчик VM не включает исходный безопасный режим из `default.vrd`.
    // Без сведений от хоста нельзя выдавать его за ответ платформы.
    Err(RtError::DynamicError(
        "безопасный режим разделения данных зоны не передан хостом".into(),
    ))
}

fn refresh_reusable_values(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    // У интерпретатора пока нет повторно используемых возвращаемых значений:
    // каждый вызов функции исполняется. Очистка пустого кэша сохраняет
    // наблюдаемое состояние платформенной процедуры и успешно завершается.
    Ok(Value::Undefined)
}

fn attach_add_in(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    Err(RtError::DynamicError(
        "загрузчик внешних компонент не передан хостом".into(),
    ))
}

fn database_configuration_changed_dynamically(
    ctx: &mut CallContext<'_>,
    _: &[Value],
) -> RtResult<Value> {
    runtime(ctx)?
        .database_configuration_changed_dynamically()
        .map(Value::Boolean)
        .ok_or_else(|| {
            RtError::DynamicError(
                "признак динамического обновления конфигурации базы данных не передан хостом"
                    .into(),
            )
        })
}

fn error_processing(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    Ok(Value::new_object(ErrorProcessingManagerObject))
}

fn error_processing_brief(
    receiver: &dyn ObjectProtocol,
    arguments: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    receiver
        .downcast_ref::<ErrorProcessingManagerObject>()
        .ok_or(RtError::NotAnObject)?;
    brief_error_description(&arguments[0])
}

fn error_processing_detail(
    receiver: &dyn ObjectProtocol,
    arguments: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    receiver
        .downcast_ref::<ErrorProcessingManagerObject>()
        .ok_or(RtError::NotAnObject)?;
    detailed_error_description(&arguments[0])
}

fn error_processing_user_settings(
    receiver: &dyn ObjectProtocol,
    _: &[Value],
    ctx: &mut CallContext<'_>,
) -> RtResult<Value> {
    receiver
        .downcast_ref::<ErrorProcessingManagerObject>()
        .ok_or(RtError::NotAnObject)?;
    let data = runtime(ctx)?
        .error_processing_settings()
        .cloned()
        .ok_or_else(|| {
            RtError::DynamicError(
                "настройки обработки ошибок текущего пользователя не переданы хостом".into(),
            )
        })?;
    Ok(Value::new_object(ErrorProcessingSettingsObject { data }))
}

fn error_settings_send_report(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let settings = receiver
        .downcast_ref::<ErrorProcessingSettingsObject>()
        .ok_or(RtError::NotAnObject)?;
    Ok(settings.data.send_report_on_client().value())
}

fn error_settings_service_address(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let settings = receiver
        .downcast_ref::<ErrorProcessingSettingsObject>()
        .ok_or(RtError::NotAnObject)?;
    Ok(Value::Str(settings.data.service_address().into()))
}

fn current_infobase_session(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    Ok(Value::new_object(InfoBaseSessionObject {
        background_method: runtime(ctx)?.background_method.clone(),
        session_number: runtime(ctx)?.session_number(),
        session_started: runtime(ctx)?.session_started,
    }))
}

fn current_infobase_session_number(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let session = receiver
        .downcast_ref::<InfoBaseSessionObject>()
        .ok_or(RtError::NotAnObject)?;
    let number = session.session_number.ok_or_else(|| {
        RtError::DynamicError("номер сеанса информационной базы не передан хостом".into())
    })?;
    Ok(Value::number_from_i64(number as i64))
}

fn current_infobase_session_started(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let session = receiver
        .downcast_ref::<InfoBaseSessionObject>()
        .ok_or(RtError::NotAnObject)?;
    let date = session.session_started.ok_or_else(|| {
        RtError::DynamicError("начало сеанса информационной базы не передано хостом".into())
    })?;
    Ok(Value::Date(date))
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

fn construct_settings_description(context: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    Ok(Value::new_object(SettingsDescriptionObject {
        object_key: RefCell::new(Value::Undefined),
        settings_key: RefCell::new(Value::Undefined),
        user: RefCell::new(Value::Str("".into())),
        presentation: RefCell::new(Value::Str("".into())),
        additional_properties: Value::new_structure(
            context.runtime_shapes().shapes.empty(),
            Vec::new(),
        ),
    }))
}

fn settings_description(receiver: &dyn ObjectProtocol) -> RtResult<&SettingsDescriptionObject> {
    receiver
        .downcast_ref::<SettingsDescriptionObject>()
        .ok_or(RtError::NotAnObject)
}

fn settings_description_object_key(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    Ok(settings_description(receiver)?.object_key.borrow().clone())
}

fn set_settings_description_object_key(
    receiver: &dyn ObjectProtocol,
    value: Value,
    _: &mut CallContext<'_>,
) -> RtResult<()> {
    *settings_description(receiver)?.object_key.borrow_mut() = value;
    Ok(())
}

fn settings_description_settings_key(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    Ok(settings_description(receiver)?
        .settings_key
        .borrow()
        .clone())
}

fn set_settings_description_settings_key(
    receiver: &dyn ObjectProtocol,
    value: Value,
    _: &mut CallContext<'_>,
) -> RtResult<()> {
    *settings_description(receiver)?.settings_key.borrow_mut() = value;
    Ok(())
}

fn string_settings_description_property(value: Value, op: &'static str) -> RtResult<Value> {
    if matches!(value, Value::Str(_)) {
        Ok(value)
    } else {
        Err(RtError::TypeError {
            expected: "Строка",
            op,
        })
    }
}

fn settings_description_user(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    Ok(settings_description(receiver)?.user.borrow().clone())
}

fn set_settings_description_user(
    receiver: &dyn ObjectProtocol,
    value: Value,
    _: &mut CallContext<'_>,
) -> RtResult<()> {
    *settings_description(receiver)?.user.borrow_mut() =
        string_settings_description_property(value, "ОписаниеНастроек.Пользователь")?;
    Ok(())
}

fn settings_description_presentation(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    Ok(settings_description(receiver)?
        .presentation
        .borrow()
        .clone())
}

fn set_settings_description_presentation(
    receiver: &dyn ObjectProtocol,
    value: Value,
    _: &mut CallContext<'_>,
) -> RtResult<()> {
    *settings_description(receiver)?.presentation.borrow_mut() =
        string_settings_description_property(value, "ОписаниеНастроек.Представление")?;
    Ok(())
}

fn settings_description_additional_properties(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    Ok(settings_description(receiver)?
        .additional_properties
        .clone())
}

fn construct_job_schedule(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    let empty_date = || RefCell::new(Value::Date(BslDate::empty()));
    let zero = || RefCell::new(Value::number_from_i64(0));
    let empty_array = || RefCell::new(Value::new_array(Vec::new()));
    Ok(Value::new_object(JobScheduleObject {
        completion_time: empty_date(),
        end_time: empty_date(),
        begin_time: empty_date(),
        end_date: empty_date(),
        begin_date: empty_date(),
        day_in_month: zero(),
        week_day_in_month: zero(),
        detailed_daily_schedules: empty_array(),
        week_days: empty_array(),
        completion_interval: zero(),
        months: empty_array(),
        repeat_pause: zero(),
        weeks_period: RefCell::new(Value::number_from_i64(1)),
        repeat_period_in_day: zero(),
        days_repeat_period: zero(),
    }))
}

fn construct_unsafe_operation_protection_description(
    _: &mut CallContext<'_>,
    _: &[Value],
) -> RtResult<Value> {
    Ok(Value::new_object(
        UnsafeOperationProtectionDescriptionObject {
            warnings: Cell::new(true),
        },
    ))
}

fn construct_event_log_event_use(_: &mut CallContext<'_>, arguments: &[Value]) -> RtResult<Value> {
    let use_event = match arguments.first() {
        None => true,
        Some(Value::Boolean(value)) => *value,
        Some(_) => {
            return Err(RtError::TypeError {
                expected: "Булево",
                op: "Новый ИспользованиеСобытияЖурналаРегистрации",
            });
        }
    };
    let description = arguments.get(1).cloned().unwrap_or(Value::Undefined);
    Ok(Value::new_object(EventLogEventUseObject {
        use_event: Cell::new(use_event),
        description: RefCell::new(description),
    }))
}

fn event_log_event_use(receiver: &dyn ObjectProtocol) -> RtResult<&EventLogEventUseObject> {
    receiver
        .downcast_ref::<EventLogEventUseObject>()
        .ok_or(RtError::NotAnObject)
}

fn event_log_use(receiver: &dyn ObjectProtocol, _: &mut CallContext<'_>) -> RtResult<Value> {
    Ok(Value::Boolean(
        event_log_event_use(receiver)?.use_event.get(),
    ))
}

fn set_event_log_use(
    receiver: &dyn ObjectProtocol,
    value: Value,
    _: &mut CallContext<'_>,
) -> RtResult<()> {
    event_log_event_use(receiver)?
        .use_event
        .set(value.as_condition()?);
    Ok(())
}

fn event_log_use_description(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    Ok(event_log_event_use(receiver)?.description.borrow().clone())
}

fn set_event_log_use_description(
    receiver: &dyn ObjectProtocol,
    value: Value,
    _: &mut CallContext<'_>,
) -> RtResult<()> {
    *event_log_event_use(receiver)?.description.borrow_mut() = value;
    Ok(())
}

fn construct_event_log_access_event_use_description(
    _: &mut CallContext<'_>,
    arguments: &[Value],
) -> RtResult<Value> {
    let object = match arguments.first() {
        None | Some(Value::Undefined) => Arc::from(""),
        Some(Value::Str(value)) => Arc::from(value.to_string()),
        Some(_) => {
            return Err(RtError::TypeError {
                expected: "Строка",
                op: "Новый ОписаниеИспользованияСобытияДоступЖурналаРегистрации",
            });
        }
    };
    Ok(Value::new_object(EventLogAccessEventUseDescriptionObject {
        object: RefCell::new(object),
        access_fields: RefCell::new(Value::new_array(Vec::new())),
        logged_fields: RefCell::new(Value::new_array(Vec::new())),
    }))
}

fn event_log_access_event_use_description(
    receiver: &dyn ObjectProtocol,
) -> RtResult<&EventLogAccessEventUseDescriptionObject> {
    receiver
        .downcast_ref::<EventLogAccessEventUseDescriptionObject>()
        .ok_or(RtError::NotAnObject)
}

fn event_log_access_object(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    Ok(Value::Str(
        event_log_access_event_use_description(receiver)?
            .object
            .borrow()
            .as_ref()
            .into(),
    ))
}

fn set_event_log_access_object(
    receiver: &dyn ObjectProtocol,
    value: Value,
    _: &mut CallContext<'_>,
) -> RtResult<()> {
    let Value::Str(value) = value else {
        return Err(RtError::TypeError {
            expected: "Строка",
            op: "ОписаниеИспользованияСобытияДоступЖурналаРегистрации.Объект",
        });
    };
    *event_log_access_event_use_description(receiver)?
        .object
        .borrow_mut() = Arc::from(value.to_string());
    Ok(())
}

fn event_log_access_fields(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    Ok(event_log_access_event_use_description(receiver)?
        .access_fields
        .borrow()
        .clone())
}

fn event_log_logged_fields(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    Ok(event_log_access_event_use_description(receiver)?
        .logged_fields
        .borrow()
        .clone())
}

fn event_log_field_array(value: Value, op: &'static str) -> RtResult<Value> {
    if matches!(&value, Value::Object(object) if matches!(&**object, BslObject::Array(_))) {
        Ok(value)
    } else {
        Err(RtError::TypeError {
            expected: "Массив",
            op,
        })
    }
}

fn set_event_log_access_fields(
    receiver: &dyn ObjectProtocol,
    value: Value,
    _: &mut CallContext<'_>,
) -> RtResult<()> {
    *event_log_access_event_use_description(receiver)?
        .access_fields
        .borrow_mut() = event_log_field_array(
        value,
        "ОписаниеИспользованияСобытияДоступЖурналаРегистрации.ПоляДоступа",
    )?;
    Ok(())
}

fn set_event_log_logged_fields(
    receiver: &dyn ObjectProtocol,
    value: Value,
    _: &mut CallContext<'_>,
) -> RtResult<()> {
    *event_log_access_event_use_description(receiver)?
        .logged_fields
        .borrow_mut() = event_log_field_array(
        value,
        "ОписаниеИспользованияСобытияДоступЖурналаРегистрации.ПоляРегистрации",
    )?;
    Ok(())
}

fn construct_client_settings(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    Ok(Value::new_object(ClientSettingsObject {
        show_navigation_and_actions_panels: Cell::new(true),
        show_sections_panel: Cell::new(true),
        forms_openning_mode: Cell::new(EnumValue::ApplicationFormsOpenningModeTabs),
        interface_variant: Cell::new(EnumValue::ClientApplicationInterfaceVariantVersion8_2),
    }))
}

fn client_settings(receiver: &dyn ObjectProtocol) -> RtResult<&ClientSettingsObject> {
    receiver
        .downcast_ref::<ClientSettingsObject>()
        .ok_or(RtError::NotAnObject)
}

fn show_navigation_and_actions_panels(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    Ok(Value::Boolean(
        client_settings(receiver)?
            .show_navigation_and_actions_panels
            .get(),
    ))
}

fn set_show_navigation_and_actions_panels(
    receiver: &dyn ObjectProtocol,
    value: Value,
    _: &mut CallContext<'_>,
) -> RtResult<()> {
    let Value::Boolean(value) = value else {
        return Err(RtError::TypeError {
            expected: "Булево",
            op: "НастройкиКлиентскогоПриложения.ОтображатьПанелиНавигацииИДействий",
        });
    };
    client_settings(receiver)?
        .show_navigation_and_actions_panels
        .set(value);
    Ok(())
}

fn show_sections_panel(receiver: &dyn ObjectProtocol, _: &mut CallContext<'_>) -> RtResult<Value> {
    Ok(Value::Boolean(
        client_settings(receiver)?.show_sections_panel.get(),
    ))
}

fn set_show_sections_panel(
    receiver: &dyn ObjectProtocol,
    value: Value,
    _: &mut CallContext<'_>,
) -> RtResult<()> {
    let Value::Boolean(value) = value else {
        return Err(RtError::TypeError {
            expected: "Булево",
            op: "НастройкиКлиентскогоПриложения.ОтображатьПанельРазделов",
        });
    };
    client_settings(receiver)?.show_sections_panel.set(value);
    Ok(())
}

fn application_forms_openning_mode(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    Ok(Value::Enum(
        client_settings(receiver)?.forms_openning_mode.get(),
    ))
}

fn set_application_forms_openning_mode(
    receiver: &dyn ObjectProtocol,
    value: Value,
    _: &mut CallContext<'_>,
) -> RtResult<()> {
    let Value::Enum(
        value @ (EnumValue::ApplicationFormsOpenningModeTabs
        | EnumValue::ApplicationFormsOpenningModeSingleWindows),
    ) = value
    else {
        return Err(RtError::TypeError {
            expected: "РежимОткрытияФормПриложения",
            op: "НастройкиКлиентскогоПриложения.РежимОткрытияФормПриложения",
        });
    };
    client_settings(receiver)?.forms_openning_mode.set(value);
    Ok(())
}

fn client_application_interface_variant(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    Ok(Value::Enum(
        client_settings(receiver)?.interface_variant.get(),
    ))
}

fn set_client_application_interface_variant(
    receiver: &dyn ObjectProtocol,
    value: Value,
    _: &mut CallContext<'_>,
) -> RtResult<()> {
    let Value::Enum(
        value @ (EnumValue::ClientApplicationInterfaceVariantVersion8_2
        | EnumValue::ClientApplicationInterfaceVariantTaxi),
    ) = value
    else {
        return Err(RtError::TypeError {
            expected: "ВариантИнтерфейсаКлиентскогоПриложения",
            op: "НастройкиКлиентскогоПриложения.ВариантИнтерфейсаКлиентскогоПриложения",
        });
    };
    client_settings(receiver)?.interface_variant.set(value);
    Ok(())
}

fn construct_command_interface_settings(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    Ok(Value::new_object(CommandInterfaceSettingsObject {
        sections_panel_representation: Cell::new(
            EnumValue::SectionsPanelRepresentationPictureAndText,
        ),
    }))
}

fn command_interface_settings(
    receiver: &dyn ObjectProtocol,
) -> RtResult<&CommandInterfaceSettingsObject> {
    receiver
        .downcast_ref::<CommandInterfaceSettingsObject>()
        .ok_or(RtError::NotAnObject)
}

fn sections_panel_representation(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    Ok(Value::Enum(
        command_interface_settings(receiver)?
            .sections_panel_representation
            .get(),
    ))
}

fn set_sections_panel_representation(
    receiver: &dyn ObjectProtocol,
    value: Value,
    _: &mut CallContext<'_>,
) -> RtResult<()> {
    let Value::Enum(
        value @ (EnumValue::SectionsPanelRepresentationPicture
        | EnumValue::SectionsPanelRepresentationPictureAndText
        | EnumValue::SectionsPanelRepresentationPictureOnTopAndText
        | EnumValue::SectionsPanelRepresentationPictureOnLeftAndText
        | EnumValue::SectionsPanelRepresentationText),
    ) = value
    else {
        return Err(RtError::TypeError {
            expected: "ОтображениеПанелиРазделов",
            op: "НастройкиКомандногоИнтерфейса.ОтображениеПанелиРазделов",
        });
    };
    command_interface_settings(receiver)?
        .sections_panel_representation
        .set(value);
    Ok(())
}

fn new_client_application_interface_content_settings_group() -> Value {
    Value::new_object(ClientApplicationInterfaceContentSettingsGroupObject {
        items: RefCell::new(Vec::new()),
    })
}

fn construct_client_application_interface_settings(
    _: &mut CallContext<'_>,
    _: &[Value],
) -> RtResult<Value> {
    Ok(Value::new_object(
        ClientApplicationInterfaceSettingsObject {
            content: RefCell::new(None),
        },
    ))
}

fn construct_client_application_interface_content_settings(
    _: &mut CallContext<'_>,
    _: &[Value],
) -> RtResult<Value> {
    Ok(Value::new_object(
        ClientApplicationInterfaceContentSettingsObject {
            top: new_client_application_interface_content_settings_group(),
            left: new_client_application_interface_content_settings_group(),
            bottom: new_client_application_interface_content_settings_group(),
            right: new_client_application_interface_content_settings_group(),
        },
    ))
}

fn construct_client_application_interface_content_settings_group(
    _: &mut CallContext<'_>,
    _: &[Value],
) -> RtResult<Value> {
    Ok(new_client_application_interface_content_settings_group())
}

fn construct_client_application_interface_content_settings_item(
    _: &mut CallContext<'_>,
    arguments: &[Value],
) -> RtResult<Value> {
    let Value::Str(name) = &arguments[0] else {
        return Err(RtError::TypeError {
            expected: "Строка",
            op: "Новый ЭлементНастройкиСоставаИнтерфейсаКлиентскогоПриложения",
        });
    };
    Ok(Value::new_object(
        ClientApplicationInterfaceContentSettingsItemObject {
            name: Arc::from(name.to_string()),
        },
    ))
}

fn client_application_interface_settings(
    receiver: &dyn ObjectProtocol,
) -> RtResult<&ClientApplicationInterfaceSettingsObject> {
    receiver
        .downcast_ref::<ClientApplicationInterfaceSettingsObject>()
        .ok_or(RtError::NotAnObject)
}

fn get_client_application_interface_content(
    receiver: &dyn ObjectProtocol,
    _: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    client_application_interface_settings(receiver)?
        .content
        .borrow()
        .clone()
        .ok_or_else(|| {
            RtError::DynamicError(
                "исходный состав интерфейса клиентского приложения не предоставлен".into(),
            )
        })
}

fn set_client_application_interface_content(
    receiver: &dyn ObjectProtocol,
    arguments: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let Some(content) = arguments.first() else {
        return Err(RtError::DynamicError(
            "состав интерфейса клиентского приложения не передан".into(),
        ));
    };
    let valid = content
        .object_ref()
        .and_then(|object| object.downcast_ref::<ClientApplicationInterfaceContentSettingsObject>())
        .is_some();
    if !valid {
        return Err(RtError::TypeError {
            expected: "НастройкиСоставаИнтерфейсаКлиентскогоПриложения",
            op: "УстановитьСостав",
        });
    }
    *client_application_interface_settings(receiver)?
        .content
        .borrow_mut() = Some(content.clone());
    Ok(Value::Undefined)
}

fn client_application_interface_content_settings(
    receiver: &dyn ObjectProtocol,
) -> RtResult<&ClientApplicationInterfaceContentSettingsObject> {
    receiver
        .downcast_ref::<ClientApplicationInterfaceContentSettingsObject>()
        .ok_or(RtError::NotAnObject)
}

macro_rules! client_application_interface_content_property {
    ($function:ident, $field:ident) => {
        fn $function(receiver: &dyn ObjectProtocol, _: &mut CallContext<'_>) -> RtResult<Value> {
            Ok(client_application_interface_content_settings(receiver)?
                .$field
                .clone())
        }
    };
}

client_application_interface_content_property!(client_interface_content_top, top);
client_application_interface_content_property!(client_interface_content_left, left);
client_application_interface_content_property!(client_interface_content_bottom, bottom);
client_application_interface_content_property!(client_interface_content_right, right);

fn client_application_interface_content_settings_group(
    receiver: &dyn ObjectProtocol,
) -> RtResult<&ClientApplicationInterfaceContentSettingsGroupObject> {
    receiver
        .downcast_ref::<ClientApplicationInterfaceContentSettingsGroupObject>()
        .ok_or(RtError::NotAnObject)
}

fn add_client_application_interface_content(
    receiver: &dyn ObjectProtocol,
    arguments: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let value = &arguments[0];
    let Some(object) = value.object_ref() else {
        return Err(RtError::TypeError {
            expected: "Элемент или группа настройки состава интерфейса клиентского приложения",
            op: "Добавить",
        });
    };
    if object
        .downcast_ref::<ClientApplicationInterfaceContentSettingsItemObject>()
        .is_none()
        && object
            .downcast_ref::<ClientApplicationInterfaceContentSettingsGroupObject>()
            .is_none()
    {
        return Err(RtError::TypeError {
            expected: "Элемент или группа настройки состава интерфейса клиентского приложения",
            op: "Добавить",
        });
    }
    let group = client_application_interface_content_settings_group(receiver)?;
    let mut items = group.items.borrow_mut();
    items.try_reserve(1).map_err(|_| {
        RtError::ResourceLimit("не удалось расширить группу состава интерфейса".into())
    })?;
    items.push(value.clone());
    Ok(Value::Undefined)
}

fn count_client_application_interface_content(
    receiver: &dyn ObjectProtocol,
    _: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let len = client_application_interface_content_settings_group(receiver)?
        .items
        .borrow()
        .len();
    Ok(Value::number_from_i64(
        i64::try_from(len).unwrap_or(i64::MAX),
    ))
}

fn get_client_application_interface_content_item(
    receiver: &dyn ObjectProtocol,
    arguments: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let Value::Number(index) = &arguments[0] else {
        return Err(RtError::BadIndex);
    };
    let index = index.to_i64_exact().ok_or(RtError::BadIndex)?;
    let position = usize::try_from(index).map_err(|_| RtError::BadIndex)?;
    let items = client_application_interface_content_settings_group(receiver)?
        .items
        .borrow();
    items
        .get(position)
        .cloned()
        .ok_or(RtError::IndexOutOfBounds {
            index,
            len: items.len(),
        })
}

fn client_application_interface_content_item_name(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let item = receiver
        .downcast_ref::<ClientApplicationInterfaceContentSettingsItemObject>()
        .ok_or(RtError::NotAnObject)?;
    Ok(Value::Str(item.name.as_ref().into()))
}

fn unsafe_operation_protection_description(
    receiver: &dyn ObjectProtocol,
) -> RtResult<&UnsafeOperationProtectionDescriptionObject> {
    receiver
        .downcast_ref::<UnsafeOperationProtectionDescriptionObject>()
        .ok_or(RtError::NotAnObject)
}

fn unsafe_operation_warnings(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    Ok(Value::Boolean(
        unsafe_operation_protection_description(receiver)?
            .warnings
            .get(),
    ))
}

fn set_unsafe_operation_warnings(
    receiver: &dyn ObjectProtocol,
    value: Value,
    _: &mut CallContext<'_>,
) -> RtResult<()> {
    let Value::Boolean(value) = value else {
        return Err(RtError::TypeError {
            expected: "Булево",
            op: "ОписаниеЗащитыОтОпасныхДействий.ПредупреждатьОбОпасныхДействиях",
        });
    };
    unsafe_operation_protection_description(receiver)?
        .warnings
        .set(value);
    Ok(())
}

fn job_schedule(receiver: &dyn ObjectProtocol) -> RtResult<&JobScheduleObject> {
    receiver
        .downcast_ref::<JobScheduleObject>()
        .ok_or(RtError::NotAnObject)
}

fn job_schedule_date(value: Value, op: &'static str) -> RtResult<Value> {
    if matches!(value, Value::Date(_)) {
        Ok(value)
    } else {
        Err(RtError::TypeError {
            expected: "Дата",
            op,
        })
    }
}

fn job_schedule_number(value: Value, op: &'static str) -> RtResult<Value> {
    if matches!(value, Value::Number(_)) {
        Ok(value)
    } else {
        Err(RtError::TypeError {
            expected: "Число",
            op,
        })
    }
}

fn job_schedule_array(value: Value, op: &'static str) -> RtResult<Value> {
    if matches!(&value, Value::Object(object) if matches!(&**object, BslObject::Array(_))) {
        Ok(value)
    } else {
        Err(RtError::TypeError {
            expected: "Массив",
            op,
        })
    }
}

macro_rules! job_schedule_property {
    ($get:ident, $set:ident, $field:ident, $check:ident, $op:literal) => {
        fn $get(receiver: &dyn ObjectProtocol, _: &mut CallContext<'_>) -> RtResult<Value> {
            Ok(job_schedule(receiver)?.$field.borrow().clone())
        }

        fn $set(
            receiver: &dyn ObjectProtocol,
            value: Value,
            _: &mut CallContext<'_>,
        ) -> RtResult<()> {
            *job_schedule(receiver)?.$field.borrow_mut() = $check(value, $op)?;
            Ok(())
        }
    };
}

job_schedule_property!(
    job_schedule_completion_time,
    set_job_schedule_completion_time,
    completion_time,
    job_schedule_date,
    "РасписаниеРегламентногоЗадания.ВремяЗавершения"
);
job_schedule_property!(
    job_schedule_end_time,
    set_job_schedule_end_time,
    end_time,
    job_schedule_date,
    "РасписаниеРегламентногоЗадания.ВремяКонца"
);
job_schedule_property!(
    job_schedule_begin_time,
    set_job_schedule_begin_time,
    begin_time,
    job_schedule_date,
    "РасписаниеРегламентногоЗадания.ВремяНачала"
);
job_schedule_property!(
    job_schedule_end_date,
    set_job_schedule_end_date,
    end_date,
    job_schedule_date,
    "РасписаниеРегламентногоЗадания.ДатаКонца"
);
job_schedule_property!(
    job_schedule_begin_date,
    set_job_schedule_begin_date,
    begin_date,
    job_schedule_date,
    "РасписаниеРегламентногоЗадания.ДатаНачала"
);
job_schedule_property!(
    job_schedule_day_in_month,
    set_job_schedule_day_in_month,
    day_in_month,
    job_schedule_number,
    "РасписаниеРегламентногоЗадания.ДеньВМесяце"
);
job_schedule_property!(
    job_schedule_week_day_in_month,
    set_job_schedule_week_day_in_month,
    week_day_in_month,
    job_schedule_number,
    "РасписаниеРегламентногоЗадания.ДеньНеделиВМесяце"
);
job_schedule_property!(
    job_schedule_detailed_daily_schedules,
    set_job_schedule_detailed_daily_schedules,
    detailed_daily_schedules,
    job_schedule_array,
    "РасписаниеРегламентногоЗадания.ДетальныеРасписанияДня"
);
job_schedule_property!(
    job_schedule_week_days,
    set_job_schedule_week_days,
    week_days,
    job_schedule_array,
    "РасписаниеРегламентногоЗадания.ДниНедели"
);
job_schedule_property!(
    job_schedule_completion_interval,
    set_job_schedule_completion_interval,
    completion_interval,
    job_schedule_number,
    "РасписаниеРегламентногоЗадания.ИнтервалЗавершения"
);
job_schedule_property!(
    job_schedule_months,
    set_job_schedule_months,
    months,
    job_schedule_array,
    "РасписаниеРегламентногоЗадания.Месяцы"
);
job_schedule_property!(
    job_schedule_repeat_pause,
    set_job_schedule_repeat_pause,
    repeat_pause,
    job_schedule_number,
    "РасписаниеРегламентногоЗадания.ПаузаПовтора"
);
job_schedule_property!(
    job_schedule_weeks_period,
    set_job_schedule_weeks_period,
    weeks_period,
    job_schedule_number,
    "РасписаниеРегламентногоЗадания.ПериодНедель"
);
job_schedule_property!(
    job_schedule_repeat_period_in_day,
    set_job_schedule_repeat_period_in_day,
    repeat_period_in_day,
    job_schedule_number,
    "РасписаниеРегламентногоЗадания.ПериодПовтораВТечениеДня"
);
job_schedule_property!(
    job_schedule_days_repeat_period,
    set_job_schedule_days_repeat_period,
    days_repeat_period,
    job_schedule_number,
    "РасписаниеРегламентногоЗадания.ПериодПовтораДней"
);

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
        code: ConstructorCode::new(24),
        names: &["АдминистрированиеСервера", "ServerAdministration"],
        arity: Arity::range(0, 2),
        call: Some(construct_server_administration),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(23),
        names: &[
            "СовместноеИспользованиеПриложенийСистемыВзаимодействия",
            "CollaborationSystemApplicationLinks",
        ],
        arity: Arity::exact(0),
        call: Some(construct_collaboration_application),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(22),
        names: &[
            "КоллекцияИдентификаторовПриложенийСистемыВзаимодействия",
            "CollaborationSystemApplicationIDCollection",
        ],
        arity: Arity::exact(0),
        call: Some(construct_collaboration_application),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(21),
        names: &[
            "ИдентификаторСообщенияСистемыВзаимодействия",
            "CollaborationSystemMessageID",
        ],
        arity: Arity::exact(1),
        call: Some(construct_collaboration_application),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(20),
        names: &[
            "ИдентификаторПриложенияСистемыВзаимодействия",
            "CollaborationSystemApplicationID",
        ],
        arity: Arity::exact(1),
        call: Some(construct_collaboration_application),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(19),
        names: &["ИнтернетПочтовоеСообщение", "InternetMailMessage"],
        arity: Arity::exact(0),
        call: Some(construct_internet_mail_message),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(18),
        names: &[
            "КонтекстОбсужденияСистемыВзаимодействия",
            "CollaborationSystemConversationContext",
        ],
        arity: Arity::exact(1),
        call: Some(construct_collaboration_conversation_context),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(17),
        names: &[
            "ИдентификаторОбсужденияСистемыВзаимодействия",
            "CollaborationSystemConversationID",
        ],
        arity: Arity::exact(1),
        call: Some(construct_collaboration_conversation_id),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(15),
        names: &[
            "ОтборОбсужденийСистемыВзаимодействия",
            "CollaborationSystemConversationsFilter",
        ],
        arity: Arity::exact(0),
        call: Some(construct_collaboration_filter),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(16),
        names: &[
            "ОтборСообщенийСистемыВзаимодействия",
            "CollaborationSystemMessagesFilter",
        ],
        arity: Arity::exact(0),
        call: Some(construct_collaboration_filter),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(1),
        names: &["СистемнаяИнформация", "SystemInfo"],
        arity: Arity::exact(0),
        call: Some(construct_system_information),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(2),
        names: &["НастройкиНачальнойСтраницы", "HomePageSettings"],
        arity: Arity::exact(0),
        call: Some(construct_home_page_settings),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(3),
        names: &["СоставФормНачальнойСтраницы", "HomePageForms"],
        arity: Arity::exact(0),
        call: Some(construct_home_page_forms),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(4),
        names: &["ОписаниеНастроек", "SettingsDescription"],
        arity: Arity::exact(0),
        call: Some(construct_settings_description),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(5),
        names: &["РасписаниеРегламентногоЗадания", "JobSchedule"],
        arity: Arity::exact(0),
        call: Some(construct_job_schedule),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(6),
        names: &[
            "ОписаниеЗащитыОтОпасныхДействий",
            "UnsafeOperationProtectionDescription",
        ],
        arity: Arity::exact(0),
        call: Some(construct_unsafe_operation_protection_description),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(7),
        names: &["ИспользованиеСобытияЖурналаРегистрации", "EventLogEventUse"],
        arity: Arity::range(0, 2),
        call: Some(construct_event_log_event_use),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(8),
        names: &["НастройкиКлиентскогоПриложения", "ClientSettings"],
        arity: Arity::exact(0),
        call: Some(construct_client_settings),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(9),
        names: &["НастройкиКомандногоИнтерфейса", "CommandInterfaceSettings"],
        arity: Arity::exact(0),
        call: Some(construct_command_interface_settings),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(10),
        names: &[
            "НастройкиИнтерфейсаКлиентскогоПриложения",
            "ClientApplicationInterfaceSettings",
        ],
        arity: Arity::exact(0),
        call: Some(construct_client_application_interface_settings),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(11),
        names: &[
            "НастройкиСоставаИнтерфейсаКлиентскогоПриложения",
            "ClientApplicationInterfaceContentSettings",
        ],
        arity: Arity::exact(0),
        call: Some(construct_client_application_interface_content_settings),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(12),
        names: &[
            "ГруппаНастройкиСоставаИнтерфейсаКлиентскогоПриложения",
            "ClientApplicationInterfaceContentSettingsGroup",
        ],
        arity: Arity::exact(0),
        call: Some(construct_client_application_interface_content_settings_group),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(13),
        names: &[
            "ЭлементНастройкиСоставаИнтерфейсаКлиентскогоПриложения",
            "ClientApplicationInterfaceContentSettingsItem",
        ],
        arity: Arity::exact(1),
        call: Some(construct_client_application_interface_content_settings_item),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(14),
        names: &[
            "ОписаниеИспользованияСобытияДоступЖурналаРегистрации",
            "EventLogAccessEventUseDescription",
        ],
        arity: Arity::range(0, 1),
        call: Some(construct_event_log_access_event_use_description),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(25),
        names: &["ЭлементПланировщика", "PlannerItem"],
        arity: Arity::exact(0),
        call: Some(planner_item::construct),
    },
];

const CONSTRUCTOR_TYPES: &[(ConstructorCode, &TypeDescriptor)] = &[
    (ConstructorCode::new(1), &SYSTEM_INFORMATION_TYPE),
    (ConstructorCode::new(2), &HOME_PAGE_SETTINGS_TYPE),
    (ConstructorCode::new(3), &HOME_PAGE_FORMS_TYPE),
    (ConstructorCode::new(4), &SETTINGS_DESCRIPTION_TYPE),
    (ConstructorCode::new(5), &JOB_SCHEDULE_TYPE),
    (
        ConstructorCode::new(6),
        &UNSAFE_OPERATION_PROTECTION_DESCRIPTION_TYPE,
    ),
    (ConstructorCode::new(7), &EVENT_LOG_EVENT_USE_TYPE),
    (ConstructorCode::new(8), &CLIENT_SETTINGS_TYPE),
    (ConstructorCode::new(9), &COMMAND_INTERFACE_SETTINGS_TYPE),
    (
        ConstructorCode::new(10),
        &CLIENT_APPLICATION_INTERFACE_SETTINGS_TYPE,
    ),
    (
        ConstructorCode::new(11),
        &CLIENT_APPLICATION_INTERFACE_CONTENT_SETTINGS_TYPE,
    ),
    (
        ConstructorCode::new(12),
        &CLIENT_APPLICATION_INTERFACE_CONTENT_SETTINGS_GROUP_TYPE,
    ),
    (
        ConstructorCode::new(13),
        &CLIENT_APPLICATION_INTERFACE_CONTENT_SETTINGS_ITEM_TYPE,
    ),
    (
        ConstructorCode::new(14),
        &EVENT_LOG_ACCESS_EVENT_USE_DESCRIPTION_TYPE,
    ),
    (ConstructorCode::new(25), &planner_item::TYPE),
    (
        ConstructorCode::new(17),
        &COLLABORATION_CONVERSATION_ID_TYPE,
    ),
];

static SYSTEM_INFORMATION_PROPERTIES: &[PropertyDescriptor] = &[PropertyDescriptor {
    names: &["ВерсияПриложения", "AppVersion"],
    get: system_information_app_version,
    set: None,
}];

static SETTINGS_DESCRIPTION_PROPERTIES: &[PropertyDescriptor] = &[
    PropertyDescriptor {
        names: &["КлючОбъекта", "ObjectKey"],
        get: settings_description_object_key,
        set: Some(set_settings_description_object_key),
    },
    PropertyDescriptor {
        names: &["КлючНастроек", "SettingsKey"],
        get: settings_description_settings_key,
        set: Some(set_settings_description_settings_key),
    },
    PropertyDescriptor {
        names: &["Пользователь", "User"],
        get: settings_description_user,
        set: Some(set_settings_description_user),
    },
    PropertyDescriptor {
        names: &["Представление", "Presentation"],
        get: settings_description_presentation,
        set: Some(set_settings_description_presentation),
    },
    PropertyDescriptor {
        names: &["ДополнительныеСвойства", "AdditionalProperties"],
        get: settings_description_additional_properties,
        set: None,
    },
];

static JOB_SCHEDULE_PROPERTIES: &[PropertyDescriptor] = &[
    PropertyDescriptor {
        names: &["ВремяЗавершения", "CompletionTime"],
        get: job_schedule_completion_time,
        set: Some(set_job_schedule_completion_time),
    },
    PropertyDescriptor {
        names: &["ВремяКонца", "EndTime"],
        get: job_schedule_end_time,
        set: Some(set_job_schedule_end_time),
    },
    PropertyDescriptor {
        names: &["ВремяНачала", "BeginTime"],
        get: job_schedule_begin_time,
        set: Some(set_job_schedule_begin_time),
    },
    PropertyDescriptor {
        names: &["ДатаКонца", "EndDate"],
        get: job_schedule_end_date,
        set: Some(set_job_schedule_end_date),
    },
    PropertyDescriptor {
        names: &["ДатаНачала", "BeginDate"],
        get: job_schedule_begin_date,
        set: Some(set_job_schedule_begin_date),
    },
    PropertyDescriptor {
        names: &["ДеньВМесяце", "DayInMonth"],
        get: job_schedule_day_in_month,
        set: Some(set_job_schedule_day_in_month),
    },
    PropertyDescriptor {
        names: &["ДеньНеделиВМесяце", "WeekDayInMonth"],
        get: job_schedule_week_day_in_month,
        set: Some(set_job_schedule_week_day_in_month),
    },
    PropertyDescriptor {
        names: &["ДетальныеРасписанияДня", "DetailedDailySchedules"],
        get: job_schedule_detailed_daily_schedules,
        set: Some(set_job_schedule_detailed_daily_schedules),
    },
    PropertyDescriptor {
        names: &["ДниНедели", "WeekDays"],
        get: job_schedule_week_days,
        set: Some(set_job_schedule_week_days),
    },
    PropertyDescriptor {
        names: &["ИнтервалЗавершения", "CompletionInterval"],
        get: job_schedule_completion_interval,
        set: Some(set_job_schedule_completion_interval),
    },
    PropertyDescriptor {
        names: &["Месяцы", "Months"],
        get: job_schedule_months,
        set: Some(set_job_schedule_months),
    },
    PropertyDescriptor {
        names: &["ПаузаПовтора", "RepeatPause"],
        get: job_schedule_repeat_pause,
        set: Some(set_job_schedule_repeat_pause),
    },
    PropertyDescriptor {
        names: &["ПериодНедель", "WeeksPeriod"],
        get: job_schedule_weeks_period,
        set: Some(set_job_schedule_weeks_period),
    },
    PropertyDescriptor {
        names: &["ПериодПовтораВТечениеДня", "RepeatPeriodInDay"],
        get: job_schedule_repeat_period_in_day,
        set: Some(set_job_schedule_repeat_period_in_day),
    },
    PropertyDescriptor {
        names: &["ПериодПовтораДней", "DaysRepeatPeriod"],
        get: job_schedule_days_repeat_period,
        set: Some(set_job_schedule_days_repeat_period),
    },
];

static UNSAFE_OPERATION_PROTECTION_DESCRIPTION_PROPERTIES: &[PropertyDescriptor] =
    &[PropertyDescriptor {
        names: &["ПредупреждатьОбОпасныхДействиях", "UnsafeOperationWarnings"],
        get: unsafe_operation_warnings,
        set: Some(set_unsafe_operation_warnings),
    }];

static EVENT_LOG_EVENT_USE_PROPERTIES: &[PropertyDescriptor] = &[
    PropertyDescriptor {
        names: &["Использование", "Use"],
        get: event_log_use,
        set: Some(set_event_log_use),
    },
    PropertyDescriptor {
        names: &["ОписаниеИспользования", "UseDescription"],
        get: event_log_use_description,
        set: Some(set_event_log_use_description),
    },
];

static EVENT_LOG_ACCESS_EVENT_USE_DESCRIPTION_PROPERTIES: &[PropertyDescriptor] = &[
    PropertyDescriptor {
        names: &["Объект", "Object"],
        get: event_log_access_object,
        set: Some(set_event_log_access_object),
    },
    PropertyDescriptor {
        names: &["ПоляДоступа", "AccessFields"],
        get: event_log_access_fields,
        set: Some(set_event_log_access_fields),
    },
    PropertyDescriptor {
        names: &["ПоляРегистрации", "LoggedFields"],
        get: event_log_logged_fields,
        set: Some(set_event_log_logged_fields),
    },
];

static CLIENT_SETTINGS_PROPERTIES: &[PropertyDescriptor] = &[
    PropertyDescriptor {
        names: &[
            "ОтображатьПанелиНавигацииИДействий",
            "ShowNavigationAndActionsPanels",
        ],
        get: show_navigation_and_actions_panels,
        set: Some(set_show_navigation_and_actions_panels),
    },
    PropertyDescriptor {
        names: &["ОтображатьПанельРазделов", "ShowSectionsPanel"],
        get: show_sections_panel,
        set: Some(set_show_sections_panel),
    },
    PropertyDescriptor {
        names: &[
            "РежимОткрытияФормПриложения",
            "ApplicationFormsOpenningMode",
        ],
        get: application_forms_openning_mode,
        set: Some(set_application_forms_openning_mode),
    },
    PropertyDescriptor {
        names: &[
            "ВариантИнтерфейсаКлиентскогоПриложения",
            "ClientApplicationInterfaceVariant",
        ],
        get: client_application_interface_variant,
        set: Some(set_client_application_interface_variant),
    },
];

static COMMAND_INTERFACE_SETTINGS_PROPERTIES: &[PropertyDescriptor] = &[PropertyDescriptor {
    names: &["ОтображениеПанелиРазделов", "SectionsPanelRepresentation"],
    get: sections_panel_representation,
    set: Some(set_sections_panel_representation),
}];

static CLIENT_APPLICATION_METHODS: &[MethodDescriptor] = &[MethodDescriptor::new(
    &["ТипПриложения", "ApplicationType"],
    Arity::exact(0),
    client_application_type,
)];

static CLIENT_APPLICATION_INTERFACE_SETTINGS_METHODS: &[MethodDescriptor] = &[
    MethodDescriptor::new(
        &["ПолучитьСостав", "GetContent"],
        Arity::exact(0),
        get_client_application_interface_content,
    ),
    MethodDescriptor::new(
        &["УстановитьСостав", "SetContent"],
        Arity::range(0, 1),
        set_client_application_interface_content,
    )
    .as_procedure(),
];

static CLIENT_APPLICATION_INTERFACE_CONTENT_SETTINGS_PROPERTIES: &[PropertyDescriptor] = &[
    PropertyDescriptor {
        names: &["Верх", "Top"],
        get: client_interface_content_top,
        set: None,
    },
    PropertyDescriptor {
        names: &["Лево", "Left"],
        get: client_interface_content_left,
        set: None,
    },
    PropertyDescriptor {
        names: &["Низ", "Bottom"],
        get: client_interface_content_bottom,
        set: None,
    },
    PropertyDescriptor {
        names: &["Право", "Right"],
        get: client_interface_content_right,
        set: None,
    },
];

static CLIENT_APPLICATION_INTERFACE_CONTENT_SETTINGS_GROUP_METHODS: &[MethodDescriptor] = &[
    MethodDescriptor::new(
        &["Добавить", "Add"],
        Arity::exact(1),
        add_client_application_interface_content,
    )
    .as_procedure(),
    MethodDescriptor::new(
        &["Количество", "Count"],
        Arity::exact(0),
        count_client_application_interface_content,
    ),
    MethodDescriptor::new(
        &["Получить", "Get"],
        Arity::exact(1),
        get_client_application_interface_content_item,
    ),
];

static CLIENT_APPLICATION_INTERFACE_CONTENT_SETTINGS_ITEM_PROPERTIES: &[PropertyDescriptor] =
    &[PropertyDescriptor {
        names: &["Имя", "Name"],
        get: client_application_interface_content_item_name,
        set: None,
    }];

static INFOBASE_SESSION_METHODS: &[MethodDescriptor] = &[MethodDescriptor::new(
    &["ПолучитьФоновоеЗадание", "GetBackgroundJob"],
    Arity::exact(0),
    current_background_job,
)];

static INFOBASE_SESSION_PROPERTIES: &[PropertyDescriptor] = &[
    PropertyDescriptor {
        names: &["НомерСеанса", "SessionNumber"],
        get: current_infobase_session_number,
        set: None,
    },
    PropertyDescriptor {
        names: &["НачалоСеанса", "SessionStarted"],
        get: current_infobase_session_started,
        set: None,
    },
];

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
    MethodDescriptor::new(
        &["ПолучитьСписок", "GetList"],
        Arity::range(1, 2),
        list_client_settings,
    ),
    MethodDescriptor::new(
        &["ПолучитьОписание", "GetDescription"],
        Arity::range(1, 3),
        describe_client_setting,
    ),
    MethodDescriptor::new(
        &["Удалить", "Delete"],
        Arity::exact(3),
        delete_client_settings,
    )
    .as_procedure(),
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

static ERROR_PROCESSING_MANAGER_METHODS: &[MethodDescriptor] = &[
    MethodDescriptor::new(
        &["КраткоеПредставлениеОшибки", "BriefErrorDescription"],
        Arity::exact(1),
        error_processing_brief,
    ),
    MethodDescriptor::new(
        &["ПодробноеПредставлениеОшибки", "DetailErrorDescription"],
        Arity::exact(1),
        error_processing_detail,
    ),
    MethodDescriptor::new(
        &["ПолучитьНастройкиПользователя", "GetUserSettings"],
        Arity::exact(0),
        error_processing_user_settings,
    ),
];

static ERROR_PROCESSING_SETTINGS_PROPERTIES: &[PropertyDescriptor] = &[
    PropertyDescriptor {
        names: &["ОтправлятьОтчетНаКлиенте", "SendReportOnClient"],
        get: error_settings_send_report,
        set: None,
    },
    PropertyDescriptor {
        names: &[
            "АдресСервисаОбработкиОшибок",
            "ErrorProcessingServiceAddress",
        ],
        get: error_settings_service_address,
        set: None,
    },
];

fn get_sessions_lock(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    // Текущий режим блокировки хранится в базе, а не выводится из параметров
    // отдельного сеанса. Без администрационного адаптера он неизвестен.
    Err(RtError::DynamicError(
        "чтение блокировки сеансов не предоставлено этим хостом".into(),
    ))
}

fn set_sessions_lock(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    // Принять настройку без записи в базу означало бы сообщить ложный успех.
    Err(RtError::DynamicError(
        "запись блокировки сеансов не предоставлена этим хостом".into(),
    ))
}

const FUNCTIONS: &[FunctionDescriptor] = &[
    FunctionDescriptor {
        code: FunctionCode::new(61),
        names: &["КодЛокализацииИнформационнойБазы", "InfoBaseLocaleCode"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(infobase_locale_code),
    },
    FunctionDescriptor {
        code: FunctionCode::new(60),
        names: &[
            "ТекущийВариантИнтерфейсаКлиентскогоПриложения",
            "ClientApplicationInterfaceCurrentVariant",
        ],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(current_client_interface_variant),
    },
    FunctionDescriptor {
        code: FunctionCode::new(58),
        names: &["ПолучитьБлокировкуСеансов", "GetSessionsLock"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(get_sessions_lock),
    },
    FunctionDescriptor {
        code: FunctionCode::new(59),
        names: &["УстановитьБлокировкуСеансов", "SetSessionsLock"],
        arity: Arity::exact(1),
        kind: FunctionKind::Procedure,
        call: Some(set_sessions_lock),
    },
    FunctionDescriptor {
        code: FunctionCode::new(57),
        names: &[
            "ДополнительныеНастройкиАутентификации",
            "AdditionalAuthenticationSettings",
        ],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(additional_authentication_settings),
    },
    FunctionDescriptor {
        code: FunctionCode::new(56),
        names: &[
            "УстановитьНастройкиКлиентаЛицензирования",
            "SetLicensingClientParameters",
        ],
        arity: Arity::range(0, 3),
        kind: FunctionKind::Procedure,
        call: Some(set_licensing_client_parameters),
    },
    FunctionDescriptor {
        code: FunctionCode::new(55),
        names: &[
            "ПолучитьДополнительныйПараметрКлиентаЛицензирования",
            "GetLicensingClientAdditionalParameter",
        ],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(get_licensing_client_additional_parameter),
    },
    FunctionDescriptor {
        code: FunctionCode::new(54),
        names: &["ПолучитьИдентификаторКонфигурации", "GetConfigurationID"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(get_configuration_id),
    },
    FunctionDescriptor {
        code: FunctionCode::new(53),
        names: &["ПолучитьИмяКлиентаЛицензирования", "GetLicensingClientName"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(get_licensing_client_name),
    },
    FunctionDescriptor {
        code: FunctionCode::new(52),
        names: &["ПолучитьРазделительПутиСервера", "GetServerPathSeparator"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(get_server_path_separator),
    },
    FunctionDescriptor {
        code: FunctionCode::new(51),
        names: &["ПолнотекстовыйПоиск", "FullTextSearch"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(full_text_search),
    },
    FunctionDescriptor {
        code: FunctionCode::new(50),
        names: &["ПредставлениеКодаЛокализации", "LocaleCodePresentation"],
        arity: Arity::exact(1),
        kind: FunctionKind::Function,
        call: Some(locale_code_presentation),
    },
    FunctionDescriptor {
        code: FunctionCode::new(49),
        names: &[
            "ПолучитьДопустимыеКодыЛокализации",
            "GetAvailableLocaleCodes",
        ],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(available_locale_codes),
    },
    FunctionDescriptor {
        code: FunctionCode::new(48),
        names: &["ПараметрыДоступа", "AccessParameters"],
        arity: Arity::range(3, 4),
        kind: FunctionKind::Function,
        call: Some(access_parameters),
    },
    FunctionDescriptor {
        code: FunctionCode::new(47),
        names: &["НайтиПомеченныеНаУдаление", "FindMarkedForDeletion"],
        arity: Arity::range(0, 3),
        kind: FunctionKind::Function,
        call: Some(find_marked_for_deletion),
    },
    FunctionDescriptor {
        code: FunctionCode::new(39),
        names: &["ПолучитьИсториюРаботыПользователяСеанса"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(user_work_history),
    },
    FunctionDescriptor {
        code: FunctionCode::new(38),
        names: &["ПолучитьХранилищеВариантовОтчетовСеанса"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(reports_variants_storage),
    },
    FunctionDescriptor {
        code: FunctionCode::new(35),
        names: &["ВыгрузитьЖурналРегистрации", "UnloadEventLog"],
        arity: Arity::range(1, 5),
        kind: FunctionKind::Procedure,
        call: Some(unload_event_log),
    },
    FunctionDescriptor {
        code: FunctionCode::new(34),
        names: &[
            "ПолучитьНавигационнуюСсылкуИнформационнойБазы",
            "GetInfoBaseURL",
        ],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(infobase_url),
    },
    FunctionDescriptor {
        code: FunctionCode::new(33),
        names: &[
            "ПолучитьСкоростьКлиентскогоСоединения",
            "GetClientConnectionSpeed",
        ],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(client_connection_speed),
    },
    FunctionDescriptor {
        code: FunctionCode::new(32),
        names: &["НомерСеансаИнформационнойБазы", "InfoBaseSessionNumber"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(infobase_session_number),
    },
    FunctionDescriptor {
        code: FunctionCode::new(31),
        names: &["СистемаВзаимодействия", "CollaborationSystem"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(collaboration_system),
    },
    FunctionDescriptor {
        code: FunctionCode::new(30),
        names: &["РегламентныеЗадания", "ScheduledJobs"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(scheduled_jobs),
    },
    FunctionDescriptor {
        code: FunctionCode::new(29),
        names: &["ПолучитьСеансыИнформационнойБазы", "GetInfoBaseSessions"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(infobase_sessions),
    },
    FunctionDescriptor {
        code: FunctionCode::new(28),
        names: &["ИмяКомпьютера", "ComputerName"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(computer_name),
    },
    FunctionDescriptor {
        code: FunctionCode::new(27),
        names: &[
            "ПолучитьИнформациюЭкрановКлиента",
            "GetClientDisplaysInformation",
        ],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(client_displays_information),
    },
    FunctionDescriptor {
        code: FunctionCode::new(1),
        names: &["ПолучитьПараметрыСеанса"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(parameters),
    },
    FunctionDescriptor {
        code: FunctionCode::new(2),
        names: &["ТекущийРежимЗапуска", "CurrentRunMode"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(current_run_mode),
    },
    FunctionDescriptor {
        code: FunctionCode::new(3),
        names: &[
            "СтрокаСоединенияИнформационнойБазы",
            "InfoBaseConnectionString",
        ],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(infobase_connection_string),
    },
    FunctionDescriptor {
        code: FunctionCode::new(4),
        names: &[
            "ПолучитьТекущийСеансИнформационнойБазы",
            "GetCurrentInfoBaseSession",
        ],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(current_infobase_session),
    },
    FunctionDescriptor {
        code: FunctionCode::new(5),
        names: &[
            "НомерСоединенияИнформационнойБазы",
            "InfoBaseConnectionNumber",
        ],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(infobase_connection_number),
    },
    FunctionDescriptor {
        code: FunctionCode::new(6),
        names: &["УстановитьБезопасныйРежим", "SetSafeMode"],
        arity: Arity::exact(1),
        kind: FunctionKind::Procedure,
        call: Some(set_safe_mode),
    },
    FunctionDescriptor {
        code: FunctionCode::new(7),
        names: &["БезопасныйРежим", "SafeMode"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(safe_mode),
    },
    FunctionDescriptor {
        code: FunctionCode::new(8),
        names: &["УстановитьПривилегированныйРежим", "SetPrivilegedMode"],
        arity: Arity::exact(1),
        kind: FunctionKind::Procedure,
        call: Some(set_privileged_mode),
    },
    FunctionDescriptor {
        code: FunctionCode::new(9),
        names: &["ПривилегированныйРежим", "PrivilegedMode"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(privileged_mode),
    },
    FunctionDescriptor {
        code: FunctionCode::new(10),
        names: &[
            "ПолучитьСоединенияИнформационнойБазы",
            "GetInfoBaseConnections",
        ],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(infobase_connections),
    },
    FunctionDescriptor {
        code: FunctionCode::new(14),
        names: &[
            "УстановитьОтключениеБезопасногоРежима",
            "SetSafeModeDisabled",
        ],
        arity: Arity::exact(1),
        kind: FunctionKind::Procedure,
        call: Some(set_safe_mode_disabled),
    },
    FunctionDescriptor {
        code: FunctionCode::new(15),
        names: &[
            "ОбновитьПовторноИспользуемыеЗначения",
            "RefreshReusableValues",
        ],
        arity: Arity::exact(0),
        kind: FunctionKind::Procedure,
        call: Some(refresh_reusable_values),
    },
    FunctionDescriptor {
        code: FunctionCode::new(16),
        names: &["ПолучитьОбработкуОшибокСеанса"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(error_processing),
    },
    FunctionDescriptor {
        code: FunctionCode::new(17),
        names: &[
            "КонфигурацияБазыДанныхИзмененаДинамически",
            "DataBaseConfigurationChangedDynamically",
        ],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(database_configuration_changed_dynamically),
    },
    FunctionDescriptor {
        code: FunctionCode::new(18),
        names: &["ПолучитьОтключениеБезопасногоРежима", "GetSafeModeDisabled"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(get_safe_mode_disabled),
    },
    FunctionDescriptor {
        code: FunctionCode::new(11),
        names: &["ЗаписьЖурналаРегистрации", "WriteLogEvent"],
        arity: Arity::range(1, 6),
        kind: FunctionKind::Procedure,
        call: Some(write_log_event),
    },
    FunctionDescriptor {
        code: FunctionCode::new(12),
        names: &["ПолучитьХранилищеОбщихНастроек"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(common_settings_storage),
    },
    FunctionDescriptor {
        code: FunctionCode::new(13),
        names: &["ПолучитьХранилищеСистемныхНастроек"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(system_settings_storage),
    },
    FunctionDescriptor {
        code: FunctionCode::new(19),
        names: &["ПолучитьХранилищеНастроекДанныхФорм"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(form_data_settings_storage),
    },
    FunctionDescriptor {
        code: FunctionCode::new(25),
        names: &[
            "ПолучитьСтандартноеХранилищеПользовательскихНастроекОтчетов",
            "GetStandardReportsUserSettingsStorage",
        ],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(reports_user_settings_storage),
    },
    FunctionDescriptor {
        code: FunctionCode::new(26),
        names: &[
            "ПолучитьСтандартноеХранилищеПользовательскихНастроекДинамическихСписков",
            "GetStandardDynamicListsUserSettingsStorage",
        ],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(dynamic_lists_user_settings_storage),
    },
    FunctionDescriptor {
        code: FunctionCode::new(20),
        names: &[
            "УстановитьБезопасныйРежимРазделенияДанных",
            "SetDataSeparationSafeMode",
        ],
        arity: Arity::exact(2),
        kind: FunctionKind::Procedure,
        call: Some(set_data_separation_safe_mode),
    },
    FunctionDescriptor {
        code: FunctionCode::new(36),
        names: &["БезопасныйРежимРазделенияДанных", "DataSeparationSafeMode"],
        arity: Arity::exact(1),
        kind: FunctionKind::Function,
        call: Some(data_separation_safe_mode),
    },
    FunctionDescriptor {
        code: FunctionCode::new(21),
        names: &["ПодключитьВнешнююКомпоненту", "AttachAddIn"],
        arity: Arity::range(1, 4),
        kind: FunctionKind::Function,
        call: Some(attach_add_in),
    },
    FunctionDescriptor {
        code: FunctionCode::new(22),
        names: &[
            "ПолучитьИспользованиеСобытияЖурналаРегистрации",
            "GetEventLogEventUse",
        ],
        arity: Arity::exact(1),
        kind: FunctionKind::Function,
        call: Some(get_event_log_event_use),
    },
    FunctionDescriptor {
        code: FunctionCode::new(23),
        names: &[
            "УстановитьИспользованиеСобытияЖурналаРегистрации",
            "SetEventLogEventUse",
        ],
        arity: Arity::exact(2),
        kind: FunctionKind::Procedure,
        call: Some(set_event_log_event_use),
    },
    FunctionDescriptor {
        code: FunctionCode::new(24),
        names: &[
            "ПолучитьИспользованиеЖурналаРегистрации",
            "GetEventLogUsing",
        ],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(get_event_log_using),
    },
    FunctionDescriptor {
        code: FunctionCode::new(40),
        names: &["КлиентскоеПриложение", "ClientApplication"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(client_application),
    },
    FunctionDescriptor {
        code: FunctionCode::new(41),
        names: &["ТекущийКодЛокализации", "CurrentLocaleCode"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(current_locale_code),
    },
    FunctionDescriptor {
        code: FunctionCode::new(42),
        names: &["ТекущийЯзыкСистемы", "CurrentSystemLanguage"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(current_system_language),
    },
    FunctionDescriptor {
        code: FunctionCode::new(43),
        names: &["ПолучитьРежимВнешнихРесурсов", "GetExternalResourcesMode"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(get_external_resources_mode),
    },
    FunctionDescriptor {
        code: FunctionCode::new(44),
        names: &[
            "ОтправкаДоставляемыхУведомлений",
            "DeliverableNotificationSend",
        ],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(deliverable_notification_send),
    },
    FunctionDescriptor {
        code: FunctionCode::new(45),
        names: &["СредстваКриптографии", "CryptoTools"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(crypto_tools),
    },
    FunctionDescriptor {
        code: FunctionCode::new(46),
        names: &[
            "ПолучитьЧасовойПоясИнформационнойБазы",
            "GetInfoBaseTimeZone",
        ],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(get_infobase_time_zone),
    },
];

const GLOBALS: &[GlobalDescriptor] = &[
    GlobalDescriptor {
        names: &[
            "ДополнительныеНастройкиАутентификации",
            "AdditionalAuthenticationSettings",
        ],
        function: FunctionCode::new(57),
    },
    GlobalDescriptor {
        names: &["ПолнотекстовыйПоиск", "FullTextSearch"],
        function: FunctionCode::new(51),
    },
    GlobalDescriptor {
        names: &["СредстваКриптографии", "CryptoTools"],
        function: FunctionCode::new(45),
    },
    GlobalDescriptor {
        names: &[
            "ОтправкаДоставляемыхУведомлений",
            "DeliverableNotificationSend",
        ],
        function: FunctionCode::new(44),
    },
    GlobalDescriptor {
        names: &["КлиентскоеПриложение", "ClientApplication"],
        function: FunctionCode::new(40),
    },
    GlobalDescriptor {
        names: &["ИсторияРаботыПользователя", "UserWorkHistory"],
        function: FunctionCode::new(39),
    },
    GlobalDescriptor {
        names: &["ХранилищеВариантовОтчетов", "ReportsVariantsStorage"],
        function: FunctionCode::new(38),
    },
    GlobalDescriptor {
        names: &["СистемаВзаимодействия", "CollaborationSystem"],
        function: FunctionCode::new(31),
    },
    GlobalDescriptor {
        names: &["РегламентныеЗадания", "ScheduledJobs"],
        function: FunctionCode::new(30),
    },
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
    GlobalDescriptor {
        names: &["ХранилищеНастроекДанныхФорм", "FormDataSettingsStorage"],
        function: FunctionCode::new(19),
    },
    GlobalDescriptor {
        names: &[
            "ХранилищеПользовательскихНастроекОтчетов",
            "ReportsUserSettingsStorage",
        ],
        function: FunctionCode::new(25),
    },
    GlobalDescriptor {
        names: &[
            "ХранилищеПользовательскихНастроекДинамическихСписков",
            "DynamicListsUserSettingsStorage",
        ],
        function: FunctionCode::new(26),
    },
    GlobalDescriptor {
        names: &["ОбработкаОшибок", "ErrorProcessing"],
        function: FunctionCode::new(16),
    },
];

const TYPES: &[&TypeDescriptor] = &[
    &COLLABORATION_CONVERSATION_ID_TYPE,
    &PARAMETERS_TYPE,
    &SYSTEM_INFORMATION_TYPE,
    &INFOBASE_SESSION_TYPE,
    &INFOBASE_CONNECTION_TYPE,
    &STANDARD_SETTINGS_STORAGE_TYPE,
    &STANDARD_SETTINGS_SELECTION_TYPE,
    &HOME_PAGE_SETTINGS_TYPE,
    &HOME_PAGE_FORMS_TYPE,
    &ERROR_PROCESSING_MANAGER_TYPE,
    &ERROR_PROCESSING_SETTINGS_TYPE,
    &SETTINGS_DESCRIPTION_TYPE,
    &JOB_SCHEDULE_TYPE,
    &UNSAFE_OPERATION_PROTECTION_DESCRIPTION_TYPE,
    &EVENT_LOG_EVENT_USE_TYPE,
    &EVENT_LOG_ACCESS_EVENT_USE_DESCRIPTION_TYPE,
    &CLIENT_SETTINGS_TYPE,
    &CLIENT_APPLICATION_TYPE,
    &COMMAND_INTERFACE_SETTINGS_TYPE,
    &CLIENT_APPLICATION_INTERFACE_SETTINGS_TYPE,
    &CLIENT_APPLICATION_INTERFACE_CONTENT_SETTINGS_TYPE,
    &CLIENT_APPLICATION_INTERFACE_CONTENT_SETTINGS_GROUP_TYPE,
    &CLIENT_APPLICATION_INTERFACE_CONTENT_SETTINGS_ITEM_TYPE,
    &CLIENT_DISPLAY_INFORMATION_TYPE,
    &planner_item::TYPE,
];

const API_MEMBERS: &[ObjectMembersDescriptor] = &[
    ObjectMembersDescriptor::new(&PARAMETERS_TYPE)
        .with_methods(PARAMETERS_METHODS)
        .with_dynamic_properties(),
    ObjectMembersDescriptor::new(&SYSTEM_INFORMATION_TYPE)
        .with_properties(SYSTEM_INFORMATION_PROPERTIES),
    ObjectMembersDescriptor::new(&INFOBASE_SESSION_TYPE)
        .with_methods(INFOBASE_SESSION_METHODS)
        .with_properties(INFOBASE_SESSION_PROPERTIES),
    ObjectMembersDescriptor::new(&INFOBASE_CONNECTION_TYPE)
        .with_properties(INFOBASE_CONNECTION_PROPERTIES),
    ObjectMembersDescriptor::new(&STANDARD_SETTINGS_STORAGE_TYPE)
        .with_methods(STANDARD_SETTINGS_STORAGE_METHODS),
    ObjectMembersDescriptor::new(&STANDARD_SETTINGS_SELECTION_TYPE)
        .with_methods(STANDARD_SETTINGS_SELECTION_METHODS)
        .with_properties(STANDARD_SETTINGS_SELECTION_PROPERTIES),
    ObjectMembersDescriptor::new(&HOME_PAGE_SETTINGS_TYPE).with_methods(HOME_PAGE_SETTINGS_METHODS),
    ObjectMembersDescriptor::new(&HOME_PAGE_FORMS_TYPE).with_properties(HOME_PAGE_FORMS_PROPERTIES),
    ObjectMembersDescriptor::new(&ERROR_PROCESSING_MANAGER_TYPE)
        .with_methods(ERROR_PROCESSING_MANAGER_METHODS),
    ObjectMembersDescriptor::new(&ERROR_PROCESSING_SETTINGS_TYPE)
        .with_properties(ERROR_PROCESSING_SETTINGS_PROPERTIES),
    ObjectMembersDescriptor::new(&SETTINGS_DESCRIPTION_TYPE)
        .with_properties(SETTINGS_DESCRIPTION_PROPERTIES),
    ObjectMembersDescriptor::new(&JOB_SCHEDULE_TYPE).with_properties(JOB_SCHEDULE_PROPERTIES),
    ObjectMembersDescriptor::new(&UNSAFE_OPERATION_PROTECTION_DESCRIPTION_TYPE)
        .with_properties(UNSAFE_OPERATION_PROTECTION_DESCRIPTION_PROPERTIES),
    ObjectMembersDescriptor::new(&EVENT_LOG_EVENT_USE_TYPE)
        .with_properties(EVENT_LOG_EVENT_USE_PROPERTIES),
    ObjectMembersDescriptor::new(&EVENT_LOG_ACCESS_EVENT_USE_DESCRIPTION_TYPE)
        .with_properties(EVENT_LOG_ACCESS_EVENT_USE_DESCRIPTION_PROPERTIES),
    ObjectMembersDescriptor::new(&CLIENT_SETTINGS_TYPE).with_properties(CLIENT_SETTINGS_PROPERTIES),
    ObjectMembersDescriptor::new(&CLIENT_APPLICATION_TYPE).with_methods(CLIENT_APPLICATION_METHODS),
    ObjectMembersDescriptor::new(&COMMAND_INTERFACE_SETTINGS_TYPE)
        .with_properties(COMMAND_INTERFACE_SETTINGS_PROPERTIES),
    ObjectMembersDescriptor::new(&CLIENT_APPLICATION_INTERFACE_SETTINGS_TYPE)
        .with_methods(CLIENT_APPLICATION_INTERFACE_SETTINGS_METHODS),
    ObjectMembersDescriptor::new(&CLIENT_APPLICATION_INTERFACE_CONTENT_SETTINGS_TYPE)
        .with_properties(CLIENT_APPLICATION_INTERFACE_CONTENT_SETTINGS_PROPERTIES),
    ObjectMembersDescriptor::new(&CLIENT_APPLICATION_INTERFACE_CONTENT_SETTINGS_GROUP_TYPE)
        .with_methods(CLIENT_APPLICATION_INTERFACE_CONTENT_SETTINGS_GROUP_METHODS),
    ObjectMembersDescriptor::new(&CLIENT_APPLICATION_INTERFACE_CONTENT_SETTINGS_ITEM_TYPE)
        .with_properties(CLIENT_APPLICATION_INTERFACE_CONTENT_SETTINGS_ITEM_PROPERTIES),
    ObjectMembersDescriptor::new(&CLIENT_DISPLAY_INFORMATION_TYPE)
        .with_properties(CLIENT_DISPLAY_INFORMATION_PROPERTIES),
    ObjectMembersDescriptor::new(&planner_item::TYPE).with_properties(planner_item::PROPERTIES),
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

#[cfg(test)]
#[path = "factory_dynamic_common_modules_tests.rs"]
mod factory_dynamic_common_modules_tests;

#[cfg(test)]
#[path = "factory_separation_parameters_tests.rs"]
mod factory_separation_parameters_tests;

#[cfg(test)]
#[path = "settings_call_context_tests.rs"]
mod settings_call_context_tests;

#[cfg(test)]
#[path = "factory_kind_tests.rs"]
mod factory_kind_tests;

#[cfg(test)]
#[path = "session_initialization_privilege_tests.rs"]
mod session_initialization_privilege_tests;

#[cfg(test)]
#[path = "linked_query_parameter_tests.rs"]
mod linked_query_parameter_tests;

#[path = "source_context.rs"]
mod source_context;

#[cfg(test)]
#[path = "initialization_index_tests.rs"]
mod initialization_index_tests;

#[cfg(test)]
#[path = "file_source_session_tests.rs"]
mod file_source_session_tests;
