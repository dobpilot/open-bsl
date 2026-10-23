//! Открытие сеанса конфигурации поверх информационной базы.
//!
//! Хост (например, `bsl-cli`) разбирает свои параметры и передаёт сюда
//! соединение, учётные данные и [`DatabaseSessionOptions`]; выбор
//! пользователя и его языка, проверка прав ролей, снимок подключений и
//! ресурсы сеанса собираются здесь, одинаково для любого хоста.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use bsl_session::{
    EventLogWriter, InfoBaseConnection, Session, SessionFactory, SessionKind, SessionResources,
    select_principal,
};
use open_bsl::ConfigurationSource;
use open_sdbl::metadata::Guid;
use open_sdbl_db::{AccessStore, BlockingDatabaseSession, Credentials, DatabaseConnection, Limits};

use crate::{DatabaseConfigurationSource, DatabaseDataAccess, LoadOptions, principal_candidates};

#[path = "database_session_host_connections.rs"]
mod host_connections;
#[path = "database_session_language.rs"]
mod language;

use host_connections::computer_name;

/// Параметры открытия сеанса, которые задаёт хост.
pub struct DatabaseSessionOptions {
    /// Параметры чтения конфигурации из базы.
    pub load: LoadOptions,
    /// Исполняемый текст основной конфигурации из файлов вместо модулей базы.
    pub file_source: Option<Arc<dyn ConfigurationSource>>,
    /// Файловые исходники расширений по идентификатору расширения в базе.
    pub extension_sources: Vec<(String, Arc<dyn ConfigurationSource>)>,
    /// Имя пользователя информационной базы; `None` — выбор по правилам
    /// платформы.
    pub principal: Option<String>,
    /// Вид клиентского приложения сеанса.
    pub session_kind: SessionKind,
    /// Версия платформы, которую видит код конфигурации.
    pub platform_version: String,
    /// `СтрокаСоединенияИнформационнойБазы()`; `None` — значение по умолчанию сеанса.
    pub infobase_connection_string: Option<String>,
    /// Код локализации сеанса.
    pub locale_code: Option<String>,
    /// Код локализации информационной базы.
    pub infobase_locale_code: Option<String>,
    /// Язык системы (интерфейса платформы).
    pub system_language: Option<String>,
    /// Имя приложения в снимке подключений (`ПолучитьСоединенияИнформационнойБазы`).
    pub application_name: String,
    /// Аргументы запуска, видимые как `ПараметрЗапуска`/аргументы командной строки.
    pub arguments: Vec<String>,
    /// Приёмник независимых событий журнала регистрации.
    pub event_log_writer: Option<Rc<dyn EventLogWriter>>,
}

// Протокол диагностических строк совпадает с вложенными этапами фасада.
fn startup_timing(stage: &str, start: std::time::Instant) {
    if open_bsl::startup_timings_enabled() {
        eprintln!(
            "open-bsl.startup\t{stage}\t{:.6}",
            start.elapsed().as_secs_f64()
        );
    }
}

/// Открывает сеанс конфигурации базы `connection` для исполнения
/// `entry_source`: читает конфигурацию, выбирает пользователя и язык,
/// проверяет права его ролей и собирает ресурсы сеанса.
///
/// # Errors
///
/// Возвращает текст ошибки соединения, чтения конфигурации или
/// пользователей, выбора пользователя или языка, отсутствующих прав ролей
/// либо открытия сеанса.
pub fn open_database_session(
    connection: &DatabaseConnection,
    credentials: &Credentials,
    entry_source: &str,
    options: DatabaseSessionOptions,
) -> Result<Session, String> {
    // Вместе с `open-sdbl-db` в процесс обычно связан HTTP-компонент. Их
    // признаки Rustls включают два провайдера, поэтому выбор явный.
    let _ = rustls::crypto::ring::default_provider().install_default();
    let phase = std::time::Instant::now();
    let acquire = if options.file_source.is_some() {
        DatabaseConfigurationSource::acquire_orm
    } else {
        DatabaseConfigurationSource::acquire
    };
    let configuration_source = Arc::new(acquire(
        BlockingDatabaseSession::connect(connection, credentials, Limits::default())
            .map_err(|error| error.to_string())?,
        options.load,
    )?);
    startup_timing("cli.acquire", phase);
    let phase = std::time::Instant::now();
    let mut catalog = configuration_source.metadata_catalog()?;
    let executable_source: Arc<dyn ConfigurationSource> = if let Some(file_source) =
        options.file_source
    {
        let bound: Arc<dyn ConfigurationSource> =
            Arc::new(open_bsl::BoundConfigurationSource::new(
                file_source,
                catalog
                    .configuration_object_id()
                    .ok_or("нет UUID корневого объекта БД")?,
                configuration_source.index().identity(),
            )?);
        let selected = configuration_source.bind_file_sources(bound, options.extension_sources)?;
        catalog = catalog.with_file_source(selected.clone())?;
        selected
    } else {
        configuration_source.clone()
    };
    let metadata = configuration_source.metadata_snapshot(&[])?;
    let mut access = AccessStore::new(&metadata);
    configuration_source.ensure_users(&mut access)?;
    let users = access
        .users()
        .ok_or_else(|| "список пользователей базы не загружен".to_string())?;
    let candidates = principal_candidates(users);
    let selected = (!candidates.is_empty())
        .then(|| {
            select_principal(
                &candidates,
                options.principal.as_deref(),
                std::iter::empty(),
            )
        })
        .transpose()
        .map_err(|error| error.to_string())?;
    startup_timing("cli.decode", phase);
    let phase = std::time::Instant::now();
    let principal = match selected {
        Some(principal) => principal,
        None => select_principal(
            &candidates,
            options.principal.as_deref(),
            catalog.main_role_ids().map(str::to_string),
        )
        .map_err(|error| error.to_string())?,
    };
    // Неизвестный формат записи отличается от известного отсутствия выбора.
    // Язык фиксируем до запуска исходного модуля и открытия нового сеанса.
    let selected_language = access
        .users()
        .and_then(|users| {
            users
                .iter()
                .find(|user| user.data.id.as_str() == principal.id())
        })
        .and_then(|user| user.data.interface_language.as_ref());
    let principal = language::resolve(principal, selected_language, &catalog)?;
    let role_ids = principal
        .role_ids()
        .map(str::parse::<Guid>)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    let missing = configuration_source.ensure_rights(&mut access, &role_ids)?;
    if !missing.is_empty() {
        return Err(format!(
            "права ролей отсутствуют: {}",
            missing
                .iter()
                .map(|role| access.role_name(role))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }

    // Источник конфигурации держит отдельное соединение для поздних чтений.
    // В снимок прикладных сеансов входит только открываемый ниже Session.
    let computer_name = computer_name()?;
    let connections = host_connections::authorized_snapshot(
        &access,
        &role_ids,
        catalog.configuration_object_id(),
        [InfoBaseConnection::new(
            1,
            &options.application_name,
            &computer_name,
        )],
    )?;

    startup_timing("cli.access", phase);
    let phase = std::time::Instant::now();
    let factory = SessionFactory::new_with_source(
        executable_source,
        Arc::new(catalog),
        "МодульСеанса",
        options.session_kind,
        options.platform_version,
        [entry_source.to_owned()],
    )
    .map_err(|error| error.to_string())?;
    startup_timing("cli.factory", phase);
    let phase = std::time::Instant::now();
    let factory = if let Some(value) = options.infobase_connection_string {
        factory
            .with_connection_string(value)
            .map_err(|error| error.to_string())?
    } else {
        factory
    };
    let connection = Rc::new(RefCell::new(
        BlockingDatabaseSession::connect(connection, credentials, Limits::default())
            .map_err(|error| error.to_string())?,
    ));
    let data_access = Rc::new(
        DatabaseDataAccess::new(Rc::clone(&connection), metadata, access, &principal)
            .with_configuration_source(configuration_source),
    );
    // Хост присваивает начало своему сеансу непосредственно перед открытием.
    // Преобразование совпадает с локальной датой стандартного host-окружения.
    let session_started = open_bsl::local_date_from_utc_seconds(
        open_bsl::Clock::unix_millis(&mut open_bsl::SystemClock).div_euclid(1000),
        "НачалоСеанса",
        &open_bsl::SystemTimeZone::new(),
    )
    .map_err(|error| error.to_string())?;
    let mut resources = SessionResources::new(Box::new(connection), 1)
        .with_session_started(session_started)
        .with_computer_name(computer_name)
        // Это начальное состояние нового сеанса, а не свойство снимка базы.
        .with_database_configuration_changed_dynamically(false)
        .with_arguments(options.arguments)
        .with_data_access(data_access.clone())
        .with_client_settings_adapter(data_access)
        .with_current_client_interface_variant(
            open_bsl::EnumValue::ClientApplicationInterfaceVariantTaxi,
        );
    if let Some(writer) = options.event_log_writer {
        resources = resources.with_event_log_writer(writer);
    }
    if let Some(connections) = connections {
        resources = resources.with_connections(connections);
    }
    if let Some(code) = &options.locale_code {
        resources = resources.with_locale_code(code.as_str());
    }
    if let Some(code) = &options.infobase_locale_code {
        resources = resources.with_infobase_locale_code(code.as_str());
    }
    if let Some(language) = &options.system_language {
        resources = resources.with_system_language(language.as_str());
    }
    startup_timing("cli.resources", phase);
    let phase = std::time::Instant::now();
    let session = factory
        .open(principal, resources)
        .map_err(|error| error.to_string())?;
    startup_timing("cli.session_open", phase);
    Ok(session)
}
