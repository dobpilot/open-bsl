//! Хранилища настроек: запросы к адаптеру, `СтандартноеХранилищеНастроекМенеджер`,
//! `ОписаниеНастроек` и настройки домашней страницы.

use super::*;

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
struct SettingsDescriptionObject {
    object_key: RefCell<Value>,
    settings_key: RefCell<Value>,
    user: RefCell<Value>,
    presentation: RefCell<Value>,
    additional_properties: Value,
}

pub(crate) static STANDARD_SETTINGS_STORAGE_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-session",
    name: "СтандартноеХранилищеНастроекМенеджер",
    type_display: "Стандартное хранилище настроек",
    type_names: &["StandardSettingsStorageManager"],
};

pub(crate) static STANDARD_SETTINGS_SELECTION_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-session",
    name: "СтандартноеХранилищеНастроекВыборка",
    type_display: "Standard settings storage Selected data",
    type_names: &["StandardSettingsStorageSelection"],
};

pub(crate) static HOME_PAGE_SETTINGS_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-session",
    name: "НастройкиНачальнойСтраницы",
    type_display: "Настройки начальной страницы",
    type_names: &["HomePageSettings"],
};

pub(crate) static HOME_PAGE_FORMS_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-session",
    name: "СоставФормНачальнойСтраницы",
    type_display: "Состав форм начальной страницы",
    type_names: &["HomePageForms"],
};

pub(crate) static SETTINGS_DESCRIPTION_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-session",
    name: "ОписаниеНастроек",
    type_display: "Описание настройки",
    type_names: &["SettingsDescription"],
};

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

pub(crate) fn construct_home_page_settings(
    _: &mut CallContext<'_>,
    _: &[Value],
) -> RtResult<Value> {
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

pub(crate) fn construct_home_page_forms(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
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

pub(crate) fn common_settings_storage(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    Ok(Value::new_object(StandardSettingsStorageObject {
        storage: ClientSettingsStorage::Common,
    }))
}

pub(crate) fn system_settings_storage(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    Ok(Value::new_object(StandardSettingsStorageObject {
        storage: ClientSettingsStorage::System,
    }))
}

pub(crate) fn form_data_settings_storage(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    Ok(Value::new_object(StandardSettingsStorageObject {
        storage: ClientSettingsStorage::FormData,
    }))
}

pub(crate) fn reports_user_settings_storage(
    _: &mut CallContext<'_>,
    _: &[Value],
) -> RtResult<Value> {
    Ok(Value::new_object(StandardSettingsStorageObject {
        storage: ClientSettingsStorage::ReportsUser,
    }))
}

pub(crate) fn reports_variants_storage(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    // В конфигурации может быть выбрано собственное хранилище вариантов.
    // Пока снимок не сообщает этот выбор, стандартное было бы догадкой.
    Err(RtError::DynamicError(
        "хранилище вариантов отчетов не определено в снимке конфигурации".into(),
    ))
}

pub(crate) fn user_work_history(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    // История работы связана с текущим пользователем и его правом сохранения
    // данных; отсутствие сервиса нельзя подменять пустым менеджером.
    Err(RtError::DynamicError(
        "история работы пользователя не предоставлена текущему сеансу".into(),
    ))
}

pub(crate) fn dynamic_lists_user_settings_storage(
    _: &mut CallContext<'_>,
    _: &[Value],
) -> RtResult<Value> {
    Ok(Value::new_object(StandardSettingsStorageObject {
        storage: ClientSettingsStorage::DynamicListsUser,
    }))
}

pub(crate) fn construct_settings_description(
    context: &mut CallContext<'_>,
    _: &[Value],
) -> RtResult<Value> {
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

pub(crate) static SETTINGS_DESCRIPTION_PROPERTIES: &[PropertyDescriptor] = &[
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

pub(crate) static STANDARD_SETTINGS_STORAGE_METHODS: &[MethodDescriptor] = &[
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

pub(crate) static STANDARD_SETTINGS_SELECTION_METHODS: &[MethodDescriptor] =
    &[MethodDescriptor::new(
        &["Следующий", "Next"],
        Arity::exact(0),
        next_client_setting,
    )];

pub(crate) static STANDARD_SETTINGS_SELECTION_PROPERTIES: &[PropertyDescriptor] = &[
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

pub(crate) static HOME_PAGE_SETTINGS_METHODS: &[MethodDescriptor] = &[
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

pub(crate) static HOME_PAGE_FORMS_PROPERTIES: &[PropertyDescriptor] = &[
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
