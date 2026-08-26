//! Нейтральный host-контракт HTTP без зависимостей от BSL и транспорта.

use std::fmt;
use std::sync::Arc;
use std::time::Duration;

/// Строка с учётными данными, которая не раскрывается через `Debug`.
#[derive(Clone, Default, PartialEq, Eq)]
pub struct SecretString(String);

impl SecretString {
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    #[must_use]
    pub fn expose(&self) -> &str {
        &self.0
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl fmt::Debug for SecretString {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("<скрыто>")
    }
}

/// Секретные байты ключевого материала без раскрытия через `Debug`.
#[derive(Clone, PartialEq, Eq)]
pub struct SecretBytes(Vec<u8>);

impl SecretBytes {
    #[must_use]
    pub fn new(value: impl Into<Vec<u8>>) -> Self {
        Self(value.into())
    }

    #[must_use]
    pub fn expose(&self) -> &[u8] {
        &self.0
    }
}

impl fmt::Debug for SecretBytes {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("<скрыто>")
    }
}

/// Снимок клиентского сертификата из файла. Пароль хранится отдельно:
/// rustls принимает незашифрованный PEM, а непустой пароль приводит к
/// явному отказу системного адаптера, не попадая в диагностику.
#[derive(Clone, PartialEq, Eq)]
pub struct ClientIdentity {
    pub bytes: SecretBytes,
    pub password: SecretString,
}

impl fmt::Debug for ClientIdentity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ClientIdentity")
            .field("bytes", &self.bytes)
            .field("password", &self.password)
            .finish()
    }
}

/// Явная конфигурация одного прокси-сервера.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProxyConfig {
    pub url: String,
    pub username: Option<SecretString>,
    pub password: SecretString,
    pub exclusions: Vec<String>,
    pub exclude_local: bool,
}

/// Политика выбора прокси.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ProxyMode {
    /// Системная политика host-приложения, включая переменные окружения.
    #[default]
    PlatformDefault,
    /// Гарантированно прямое соединение.
    Direct,
    /// Явно заданный прокси.
    Explicit(ProxyConfig),
}

/// Настройки TLS одного HTTP-соединения.
#[derive(Clone, PartialEq, Eq)]
pub enum TlsConfig {
    Plain,
    SystemRoots,
    CustomRoots(Vec<Vec<u8>>),
    Insecure,
}

impl fmt::Debug for TlsConfig {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Plain => formatter.write_str("Plain"),
            Self::SystemRoots => formatter.write_str("SystemRoots"),
            Self::CustomRoots(roots) => formatter
                .debug_tuple("CustomRoots")
                .field(&format_args!("{} сертификатов", roots.len()))
                .finish(),
            Self::Insecure => formatter.write_str("Insecure"),
        }
    }
}

/// Неизменяемая конфигурация транспортного клиента одного `HTTPСоединение`.
#[derive(Clone)]
pub struct HttpClientConfig {
    pub host: String,
    pub port: u16,
    pub username: Option<String>,
    pub password: SecretString,
    pub proxy: ProxyMode,
    pub timeout: Option<Duration>,
    pub tls: TlsConfig,
    pub client_identity: Option<ClientIdentity>,
    pub use_os_authentication: bool,
}

impl fmt::Debug for HttpClientConfig {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("HttpClientConfig")
            .field("host", &self.host)
            .field("port", &self.port)
            .field("username", &self.username.as_deref().map(|_| "<скрыто>"))
            .field("password", &self.password)
            .field("proxy", &self.proxy)
            .field("timeout", &self.timeout)
            .field("tls", &self.tls)
            .field("client_identity", &self.client_identity)
            .field("use_os_authentication", &self.use_os_authentication)
            .finish()
    }
}

/// Снимок HTTP-запроса, полностью владеющий отправляемыми данными.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpWireRequest {
    pub method: String,
    pub resource: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

/// Полученный HTTP-ответ. Любой код состояния является успешным ответом
/// транспорта; ошибки сети представлены отдельно.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpWireResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

/// Стабильная категория сетевого отказа, не зависящая от `reqwest`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum NetworkErrorKind {
    Dns,
    Timeout,
    Tls,
    Proxy,
    Protocol,
    Io,
    Cancelled,
    Unsupported,
}

/// Сетевой отказ с безопасным для диагностики текстом.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkError {
    pub kind: NetworkErrorKind,
    pub message: String,
}

impl NetworkError {
    #[must_use]
    pub fn new(kind: NetworkErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

impl fmt::Display for NetworkError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.message)
    }
}

impl std::error::Error for NetworkError {}

/// Одноразовый приёмник результата внешней HTTP-операции.
pub trait HttpCompletionSink: Send {
    fn complete(self: Box<Self>, result: Result<HttpWireResponse, NetworkError>);
}

/// Отменяемая внешняя операция. `Drop` реализации также обязан отменять её.
pub trait RequestHandle: fmt::Debug + Send {
    fn cancel(&mut self);
}

/// Настроенный и пригодный для параллельных запросов HTTP-клиент.
pub trait HttpClient: fmt::Debug + Send + Sync {
    /// Запускает запрос и ровно один раз завершает `sink`.
    ///
    /// # Errors
    ///
    /// Ошибка до принятия операции транспортом. После `Ok` любой отказ
    /// доставляется только в `sink`.
    fn submit(
        &self,
        request: HttpWireRequest,
        sink: Box<dyn HttpCompletionSink>,
    ) -> Result<Box<dyn RequestHandle>, NetworkError>;
}

/// Фабрика клиентов, внедряемая host-приложением в одну BSL-сессию.
pub trait HttpClientFactory: fmt::Debug {
    /// # Errors
    ///
    /// Неподдерживаемая или некорректная конфигурация транспорта.
    fn create(&self, config: HttpClientConfig) -> Result<Arc<dyn HttpClient>, NetworkError>;
}

/// Материализация результата HTTP в BSL-потоке после доставки транспортом.
/// Функция не исполняется в Tokio и потому вправе строить `BslValue` с `Rc`.
pub type HttpResponseMapper = fn(
    Result<HttpWireResponse, NetworkError>,
    &mut crate::RuntimeShapes,
) -> crate::RtResult<crate::BslValue>;

/// Преобразование синхронного отказа транспорта до принятия операции.
pub type HttpErrorMapper = fn(NetworkError) -> crate::RtError;

/// Узкая возможность `Execution`, через которую компонент заводит обещание
/// для внешней HTTP-операции. Конкретный future и канал остаются в host-слое.
pub trait HttpPromiseSpawner {
    /// Запускает операцию и возвращает непрозрачное BSL-обещание.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку создания обещания или синхронный отказ транспорта
    /// до принятия запроса.
    fn spawn_http(
        &mut self,
        client: Arc<dyn HttpClient>,
        request: HttpWireRequest,
        mapper: HttpResponseMapper,
        error_mapper: HttpErrorMapper,
    ) -> crate::RtResult<crate::BslValue>;
}
