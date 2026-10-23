//! Ресурсы и исполняемое окружение сеанса: адаптеры, режим внешних
//! ресурсов и вид сеанса.

use super::*;

/// Ресурсы одного открываемого сеанса.
pub struct SessionResources {
    pub(crate) connection: Box<dyn std::any::Any>,
    pub(crate) connection_number: u64,
    pub(crate) arguments: Vec<String>,
    pub(crate) session_number: Option<u64>,
    pub(crate) session_started: Option<BslDate>,
    pub(crate) data_access: Option<Rc<dyn DataAccess>>,
    pub(crate) connections: Option<Arc<[InfoBaseConnection]>>,
    pub(crate) event_log_writer: Option<Rc<dyn EventLogWriter>>,
    pub(crate) client_settings: Option<Rc<dyn ClientSettingsAdapter>>,
    pub(crate) error_processing_settings: Option<ErrorProcessingSettingsData>,
    pub(crate) database_configuration_changed_dynamically: Option<bool>,
    pub(crate) data_separation_common_attributes: Option<Arc<[Arc<str>]>>,
    pub(crate) client_displays: Option<Arc<[ClientDisplayInformation]>>,
    pub(crate) client_connection_speed: Option<ClientConnectionSpeed>,
    pub(crate) current_client_interface_variant: Option<EnumValue>,
    pub(crate) computer_name: Option<Arc<str>>,
    pub(crate) locale_code: Option<Arc<str>>,
    pub(crate) infobase_locale_code: Option<Arc<str>>,
    pub(crate) system_language: Option<Arc<str>>,
    pub(crate) external_resources_mode: Option<ExternalResourcesMode>,
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
    pub(crate) parameters: Rc<SessionParameters>,
    pub(crate) kind: SessionKind,
    pub(crate) platform_version: Arc<str>,
    pub(crate) connection_string: Option<Arc<str>>,
    pub(crate) connection_number: u64,
    pub(crate) session_number: Option<u64>,
    pub(crate) session_started: Option<BslDate>,
    pub(crate) connections: Option<Arc<[InfoBaseConnection]>>,
    pub(crate) event_log_writer: Option<Rc<dyn EventLogWriter>>,
    pub(crate) client_settings: Option<Rc<dyn ClientSettingsAdapter>>,
    pub(crate) error_processing_settings: Option<ErrorProcessingSettingsData>,
    pub(crate) database_configuration_changed_dynamically: Option<bool>,
    pub(crate) data_separation_common_attributes: Option<Arc<[Arc<str>]>>,
    pub(crate) client_displays: Option<Arc<[ClientDisplayInformation]>>,
    pub(crate) client_connection_speed: Option<ClientConnectionSpeed>,
    pub(crate) current_client_interface_variant: Option<EnumValue>,
    pub(crate) computer_name: Option<Arc<str>>,
    pub(crate) locale_code: Option<Arc<str>>,
    pub(crate) infobase_locale_code: Option<Arc<str>>,
    pub(crate) system_language: Option<Arc<str>>,
    pub(crate) external_resources_mode: Option<ExternalResourcesMode>,
    pub(crate) background_method: Option<Arc<str>>,
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
