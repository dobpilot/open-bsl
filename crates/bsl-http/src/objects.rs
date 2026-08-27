//! Объекты `HTTPЗапрос`, `HTTPОтвет` и `HTTPСоединение`.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use bsl_rt::encoding::Encoding;
use bsl_rt::{
    Arity, BslObject, BslString, BslValue, CallContext, CallOutcome, ClientIdentity,
    ComponentError, EnumValue, HttpClient, HttpClientConfig, HttpWireRequest, HttpWireResponse,
    MethodDescriptor, NetworkError, ObjectProtocol, PendingHostCall, PropertyDescriptor,
    ProxyConfig, ProxyMode, RtError, RtResult, SecretBytes, SecretString, TlsConfig,
    TypeDescriptor, folded_eq,
};

pub static HTTP_REQUEST_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "HTTPЗапрос",
    type_display: "HTTPЗапрос",
    type_names: &["HTTPRequest"],
};

pub static HTTP_RESPONSE_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "HTTPОтвет",
    type_display: "HTTPОтвет",
    type_names: &["HTTPResponse"],
};

pub static HTTP_CONNECTION_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "HTTPСоединение",
    type_display: "HTTPСоединение",
    type_names: &["HTTPConnection"],
};

pub static INTERNET_PROXY_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ИнтернетПрокси",
    type_display: "ИнтернетПрокси",
    type_names: &["InternetProxy"],
};

pub static SYSTEM_ROOTS_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "СертификатыУдостоверяющихЦентровОС",
    type_display: "СертификатыУдостоверяющихЦентровОС",
    type_names: &["CertificationAuthorityCertificatesOS"],
};

pub static SECURE_CONNECTION_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ЗащищенноеСоединениеOpenSSL",
    type_display: "ЗащищенноеСоединениеOpenSSL",
    type_names: &["OpenSSLSecureConnection"],
};

pub static FILE_ROOTS_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "СертификатыУдостоверяющихЦентровФайл",
    type_display: "FileCertificationAuthorityCertificates",
    type_names: &["FileCertificationAuthorityCertificates"],
};

pub static FILE_CLIENT_CERTIFICATE_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "СертификатКлиентаФайл",
    type_display: "FileClientCertificate",
    type_names: &["FileClientCertificate"],
};

/// Дескриптор платформенного Windows-сертификата.
///
/// Конструктор намеренно не регистрируется: host пока не умеет извлекать
/// закрытый ключ из хранилища Windows. Сам тип нужен Connector для
/// безопасной проверки `ТипЗнч` даже при пустом клиентском сертификате.
pub static WINDOWS_CLIENT_CERTIFICATE_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "СертификатКлиентаWindows",
    type_display: "WindowsClientCertificate",
    type_names: &["WindowsClientCertificate"],
};

#[derive(Debug, Clone)]
struct ProxyEndpoint {
    server: String,
    port: u16,
}

#[derive(Debug)]
struct ProxyData {
    use_system: bool,
    http: Option<ProxyEndpoint>,
    https: Option<ProxyEndpoint>,
    exclusions: BslValue,
    exclude_local: bool,
    username: SecretString,
    password: SecretString,
}

#[derive(Debug)]
pub struct InternetProxyObject {
    data: RefCell<ProxyData>,
}

impl ObjectProtocol for InternetProxyObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &INTERNET_PROXY_TYPE
    }

    fn property_table(&self) -> &'static [PropertyDescriptor] {
        PROXY_PROPERTIES
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        PROXY_METHODS
    }
}

pub fn new_internet_proxy(arguments: &[BslValue]) -> RtResult<BslValue> {
    let use_system = optional_bool(arguments, 0, true, "Новый ИнтернетПрокси")?;
    Ok(BslValue::new_object(InternetProxyObject {
        data: RefCell::new(ProxyData {
            use_system,
            http: None,
            https: None,
            exclusions: BslValue::new_array(Vec::new()),
            // ИЗМЕРЕНО: у `Новый ИнтернетПрокси(Ложь)` локальные адреса
            // исключены, у системного объекта — нет.
            exclude_local: !use_system,
            username: SecretString::default(),
            password: SecretString::default(),
        }),
    }))
}

fn proxy(receiver: &dyn ObjectProtocol) -> RtResult<&InternetProxyObject> {
    receiver
        .downcast_ref::<InternetProxyObject>()
        .ok_or(RtError::NotAnObject)
}

fn proxy_exclusions(
    receiver: &dyn ObjectProtocol,
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    Ok(proxy(receiver)?.data.borrow().exclusions.clone())
}

fn proxy_exclude_local(
    receiver: &dyn ObjectProtocol,
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    Ok(BslValue::Boolean(
        proxy(receiver)?.data.borrow().exclude_local,
    ))
}

fn set_proxy_exclude_local(
    receiver: &dyn ObjectProtocol,
    value: BslValue,
    _context: &mut CallContext<'_>,
) -> RtResult<()> {
    let BslValue::Boolean(value) = value else {
        return Err(type_error(
            "Булево",
            "ИнтернетПрокси.НеИспользоватьПроксиДляЛокальныхАдресов",
        ));
    };
    proxy(receiver)?.data.borrow_mut().exclude_local = value;
    Ok(())
}

fn proxy_username(
    receiver: &dyn ObjectProtocol,
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    Ok(string(proxy(receiver)?.data.borrow().username.expose()))
}

fn set_proxy_username(
    receiver: &dyn ObjectProtocol,
    value: BslValue,
    _context: &mut CallContext<'_>,
) -> RtResult<()> {
    let value = value_string(value, "ИнтернетПрокси.Пользователь")?;
    proxy(receiver)?.data.borrow_mut().username = SecretString::new(value);
    Ok(())
}

fn proxy_password(
    receiver: &dyn ObjectProtocol,
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    Ok(string(proxy(receiver)?.data.borrow().password.expose()))
}

fn set_proxy_password(
    receiver: &dyn ObjectProtocol,
    value: BslValue,
    _context: &mut CallContext<'_>,
) -> RtResult<()> {
    let value = value_string(value, "ИнтернетПрокси.Пароль")?;
    proxy(receiver)?.data.borrow_mut().password = SecretString::new(value);
    Ok(())
}

fn set_proxy_endpoint(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let protocol = required_string(arguments, 0, "ИнтернетПрокси.Установить")?;
    let endpoint = ProxyEndpoint {
        server: required_string(arguments, 1, "ИнтернетПрокси.Установить")?,
        port: required_u16(arguments, 2, "ИнтернетПрокси.Установить")?,
    };
    let mut data = proxy(receiver)?.data.borrow_mut();
    if folded_eq(&protocol, "http") {
        data.http = Some(endpoint);
    } else if folded_eq(&protocol, "https") {
        data.https = Some(endpoint);
    } else {
        return Err(http_error(
            "Прокси",
            format!("протокол прокси «{protocol}» не поддерживается HTTP-компонентом"),
        ));
    }
    Ok(BslValue::Undefined)
}

static PROXY_PROPERTIES: &[PropertyDescriptor] = &[
    PropertyDescriptor {
        names: &["НеИспользоватьПроксиДляАдресов", "BypassProxyForAddresses"],
        get: proxy_exclusions,
        set: None,
    },
    PropertyDescriptor {
        names: &[
            "НеИспользоватьПроксиДляЛокальныхАдресов",
            "BypassProxyForLocalAddresses",
        ],
        get: proxy_exclude_local,
        set: Some(set_proxy_exclude_local),
    },
    PropertyDescriptor {
        names: &["Пользователь", "User"],
        get: proxy_username,
        set: Some(set_proxy_username),
    },
    PropertyDescriptor {
        names: &["Пароль", "Password"],
        get: proxy_password,
        set: Some(set_proxy_password),
    },
];

static PROXY_METHODS: &[MethodDescriptor] = &[MethodDescriptor::new(
    &["Установить", "Set"],
    Arity::exact(3),
    set_proxy_endpoint,
)];

fn proxy_exclusion_strings(value: &BslValue) -> RtResult<Vec<String>> {
    let BslValue::Object(object) = value else {
        return Err(type_error("Массив", "Новый HTTPСоединение"));
    };
    let BslObject::Array(items) = &**object else {
        return Err(type_error("Массив", "Новый HTTPСоединение"));
    };
    items
        .borrow()
        .iter()
        .cloned()
        .map(|value| value_string(value, "Новый HTTPСоединение"))
        .collect()
}

fn proxy_mode(value: Option<&BslValue>, secure: bool) -> RtResult<ProxyMode> {
    match value {
        None | Some(BslValue::Undefined) => Ok(ProxyMode::PlatformDefault),
        Some(value) => {
            let proxy = value
                .object_ref()
                .and_then(|object| object.downcast_ref::<InternetProxyObject>())
                .ok_or_else(|| type_error("ИнтернетПрокси", "Новый HTTPСоединение"))?;
            let data = proxy.data.borrow();
            if data.use_system {
                Ok(ProxyMode::PlatformDefault)
            } else {
                let endpoint = if secure { &data.https } else { &data.http };
                let Some(endpoint) = endpoint else {
                    return Ok(ProxyMode::Direct);
                };
                let server = endpoint.server.trim_end_matches('/');
                let url = if server.contains("://") {
                    format!("{server}:{}", endpoint.port)
                } else {
                    format!("http://{server}:{}", endpoint.port)
                };
                Ok(ProxyMode::Explicit(ProxyConfig {
                    url,
                    username: (!data.username.is_empty()).then(|| data.username.clone()),
                    password: data.password.clone(),
                    exclusions: proxy_exclusion_strings(&data.exclusions)?,
                    exclude_local: data.exclude_local,
                }))
            }
        }
    }
}

#[derive(Debug)]
pub struct SystemRootsObject;

impl ObjectProtocol for SystemRootsObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &SYSTEM_ROOTS_TYPE
    }
}

pub fn new_system_roots() -> BslValue {
    BslValue::new_object(SystemRootsObject)
}

pub struct FileRootsObject {
    bytes: Vec<u8>,
}

impl std::fmt::Debug for FileRootsObject {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("FileRootsObject")
            .field("bytes", &format_args!("{} байт", self.bytes.len()))
            .finish()
    }
}

impl ObjectProtocol for FileRootsObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &FILE_ROOTS_TYPE
    }
}

pub fn new_file_roots(arguments: &[BslValue], context: &mut CallContext<'_>) -> RtResult<BslValue> {
    let path = required_string(arguments, 0, "Новый СертификатыУдостоверяющихЦентровФайл")?;
    let bytes = context
        .files_rc()?
        .read(&path)
        .map_err(|error| RtError::IoError(format!("{path}: {error}")))?;
    Ok(BslValue::new_object(FileRootsObject { bytes }))
}

pub struct FileClientCertificateObject {
    identity: ClientIdentity,
}

impl std::fmt::Debug for FileClientCertificateObject {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("FileClientCertificateObject")
            .field("identity", &self.identity)
            .finish()
    }
}

impl ObjectProtocol for FileClientCertificateObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &FILE_CLIENT_CERTIFICATE_TYPE
    }
}

pub fn new_file_client_certificate(
    arguments: &[BslValue],
    context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let path = required_string(arguments, 0, "Новый СертификатКлиентаФайл")?;
    let password = optional_string(arguments, 1, "Новый СертификатКлиентаФайл")?
        .map_or_else(SecretString::default, SecretString::new);
    let bytes = context
        .files_rc()?
        .read(&path)
        .map_err(|error| RtError::IoError(format!("{path}: {error}")))?;
    Ok(BslValue::new_object(FileClientCertificateObject {
        identity: ClientIdentity {
            bytes: SecretBytes::new(bytes),
            password,
        },
    }))
}

#[derive(Debug)]
pub struct SecureConnectionObject {
    client_certificate: BslValue,
    roots: BslValue,
    tls: TlsConfig,
    client_identity: Option<ClientIdentity>,
}

impl ObjectProtocol for SecureConnectionObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &SECURE_CONNECTION_TYPE
    }

    fn property_table(&self) -> &'static [PropertyDescriptor] {
        SECURE_CONNECTION_PROPERTIES
    }
}

pub fn new_secure_connection(arguments: &[BslValue]) -> RtResult<BslValue> {
    let client_certificate = arguments.first().cloned().unwrap_or(BslValue::Undefined);
    let client_identity = match &client_certificate {
        BslValue::Undefined => None,
        value => Some(
            value
                .object_ref()
                .and_then(|object| object.downcast_ref::<FileClientCertificateObject>())
                .map(|certificate| certificate.identity.clone())
                .ok_or_else(|| {
                    type_error("СертификатКлиентаФайл", "Новый ЗащищенноеСоединениеOpenSSL")
                })?,
        ),
    };
    let roots = arguments.get(1).cloned().unwrap_or(BslValue::Undefined);
    let tls = match &roots {
        BslValue::Undefined => TlsConfig::Insecure,
        value
            if value
                .object_ref()
                .is_some_and(|object| object.downcast_ref::<SystemRootsObject>().is_some()) =>
        {
            TlsConfig::SystemRoots
        }
        value => {
            if let Some(file) = value
                .object_ref()
                .and_then(|object| object.downcast_ref::<FileRootsObject>())
            {
                TlsConfig::CustomRoots(vec![file.bytes.clone()])
            } else {
                return Err(type_error(
                    "СертификатыУдостоверяющихЦентров",
                    "Новый ЗащищенноеСоединениеOpenSSL",
                ));
            }
        }
    };
    Ok(BslValue::new_object(SecureConnectionObject {
        client_certificate,
        roots,
        tls,
        client_identity,
    }))
}

fn secure(receiver: &dyn ObjectProtocol) -> RtResult<&SecureConnectionObject> {
    receiver
        .downcast_ref::<SecureConnectionObject>()
        .ok_or(RtError::NotAnObject)
}

fn secure_client_certificate(
    receiver: &dyn ObjectProtocol,
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    Ok(secure(receiver)?.client_certificate.clone())
}

fn secure_roots(
    receiver: &dyn ObjectProtocol,
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    Ok(secure(receiver)?.roots.clone())
}

static SECURE_CONNECTION_PROPERTIES: &[PropertyDescriptor] = &[
    PropertyDescriptor {
        names: &["СертификатКлиента", "ClientCertificate"],
        get: secure_client_certificate,
        set: None,
    },
    PropertyDescriptor {
        names: &[
            "СертификатыУдостоверяющихЦентров",
            "CertificationAuthorityCertificates",
        ],
        get: secure_roots,
        set: None,
    },
];

#[derive(Debug)]
struct RequestData {
    resource: BslString,
    headers: BslValue,
    body: BslValue,
}

#[derive(Debug)]
pub struct HttpRequestObject {
    data: Rc<RefCell<RequestData>>,
}

impl ObjectProtocol for HttpRequestObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &HTTP_REQUEST_TYPE
    }

    fn property_table(&self) -> &'static [PropertyDescriptor] {
        REQUEST_PROPERTIES
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        REQUEST_METHODS
    }
}

pub fn new_http_request(resource: &BslValue) -> RtResult<BslValue> {
    let resource = match resource {
        BslValue::Undefined => BslString::from_str(""),
        BslValue::Str(value) => value.clone(),
        _ => return Err(type_error("Строка", "Новый HTTPЗапрос")),
    };
    Ok(BslValue::new_object(HttpRequestObject {
        data: Rc::new(RefCell::new(RequestData {
            resource,
            headers: BslValue::new_map(),
            body: memory_stream(&[])?,
        })),
    }))
}

fn request(receiver: &dyn ObjectProtocol) -> RtResult<&HttpRequestObject> {
    receiver
        .downcast_ref::<HttpRequestObject>()
        .ok_or(RtError::NotAnObject)
}

fn request_resource(
    receiver: &dyn ObjectProtocol,
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    Ok(BslValue::Str(
        request(receiver)?.data.borrow().resource.clone(),
    ))
}

fn set_request_resource(
    receiver: &dyn ObjectProtocol,
    value: BslValue,
    _context: &mut CallContext<'_>,
) -> RtResult<()> {
    let BslValue::Str(value) = value else {
        return Err(type_error("Строка", "HTTPЗапрос.АдресРесурса"));
    };
    request(receiver)?.data.borrow_mut().resource = value;
    Ok(())
}

fn request_headers(
    receiver: &dyn ObjectProtocol,
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    Ok(request(receiver)?.data.borrow().headers.clone())
}

fn set_request_headers(
    receiver: &dyn ObjectProtocol,
    value: BslValue,
    _context: &mut CallContext<'_>,
) -> RtResult<()> {
    value.map_entries()?;
    // ИЗМЕРЕНО(HTTP.REQUEST.HEADERS.ALIAS): запрос сохраняет ту же ссылку,
    // последующая вставка в исходное соответствие видна через свойство.
    request(receiver)?.data.borrow_mut().headers = value;
    Ok(())
}

static REQUEST_PROPERTIES: &[PropertyDescriptor] = &[
    PropertyDescriptor {
        names: &["АдресРесурса", "ResourceAddress"],
        get: request_resource,
        set: Some(set_request_resource),
    },
    PropertyDescriptor {
        names: &["Заголовки", "Headers"],
        get: request_headers,
        set: Some(set_request_headers),
    },
];

fn set_body_from_binary(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let bytes = arguments[0]
        .binary_data_bytes()
        .ok_or_else(|| type_error("ДвоичныеДанные", "УстановитьТелоИзДвоичныхДанных"))?;
    request(receiver)?.data.borrow_mut().body = memory_stream(bytes)?;
    Ok(BslValue::Undefined)
}

fn set_body_from_string(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let BslValue::Str(text) = &arguments[0] else {
        return Err(type_error("Строка", "УстановитьТелоИзСтроки"));
    };
    let encoding = match arguments.get(1) {
        None | Some(BslValue::Undefined) | Some(BslValue::Enum(EnumValue::TextEncodingUtf8)) => {
            Encoding::Utf8
        }
        Some(BslValue::Enum(
            EnumValue::TextEncodingUtf16
            | EnumValue::TextEncodingAnsi
            | EnumValue::TextEncodingSystem
            | EnumValue::TextEncodingOem,
        )) => {
            return Err(http_error(
                "encoding",
                "для тела HTTP пока измерена только кодировка UTF-8",
            ));
        }
        _ => return Err(type_error("КодировкаТекста", "УстановитьТелоИзСтроки")),
    };
    let use_bom = match arguments.get(2) {
        None
        | Some(BslValue::Undefined)
        | Some(BslValue::Enum(EnumValue::ByteOrderMarkDoNotUse)) => false,
        Some(BslValue::Enum(EnumValue::ByteOrderMarkUse)) => true,
        _ => {
            return Err(type_error(
                "ИспользованиеByteOrderMark",
                "УстановитьТелоИзСтроки",
            ));
        }
    };
    let text = text.to_string();
    let bytes = if use_bom {
        encoding.encode(&text)
    } else {
        encoding.encode_without_signature(&text)
    };
    request(receiver)?.data.borrow_mut().body = memory_stream(&bytes)?;
    Ok(BslValue::Undefined)
}

fn request_body_binary(
    receiver: &dyn ObjectProtocol,
    _arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let body = request(receiver)?.data.borrow().body.clone();
    let bytes = body
        .byte_stream()
        .ok_or(RtError::NotAnObject)?
        .read_all("HTTPЗапрос.ПолучитьТелоКакДвоичныеДанные")?;
    Ok(BslValue::binary_data_of(bytes))
}

fn request_body_string(
    receiver: &dyn ObjectProtocol,
    _arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let body = request(receiver)?.data.borrow().body.clone();
    let bytes = body
        .byte_stream()
        .ok_or(RtError::NotAnObject)?
        .read_all("HTTPЗапрос.ПолучитьТелоКакСтроку")?;
    Ok(BslValue::Str(BslString::from_utf8_string(
        Encoding::Utf8.decode(&bytes)?,
    )))
}

fn request_body_stream(
    receiver: &dyn ObjectProtocol,
    _arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    Ok(request(receiver)?.data.borrow().body.clone())
}

static REQUEST_METHODS: &[MethodDescriptor] = &[
    MethodDescriptor::new(
        &["УстановитьТелоИзСтроки", "SetBodyFromString"],
        Arity::range(1, 3),
        set_body_from_string,
    ),
    MethodDescriptor::new(
        &["УстановитьТелоИзДвоичныхДанных", "SetBodyFromBinaryData"],
        Arity::exact(1),
        set_body_from_binary,
    ),
    MethodDescriptor::new(
        &["ПолучитьТелоКакДвоичныеДанные", "GetBodyAsBinaryData"],
        Arity::exact(0),
        request_body_binary,
    ),
    MethodDescriptor::new(
        &["ПолучитьТелоКакСтроку", "GetBodyAsString"],
        Arity::exact(0),
        request_body_string,
    ),
    MethodDescriptor::new(
        &["ПолучитьТелоКакПоток", "GetBodyAsStream"],
        Arity::exact(0),
        request_body_stream,
    ),
];

#[derive(Debug)]
pub struct HttpResponseObject {
    response: HttpWireResponse,
    headers: BslValue,
}

impl HttpResponseObject {
    fn from_wire(response: HttpWireResponse) -> RtResult<BslValue> {
        let headers = BslValue::new_map();
        let mut combined: Vec<(String, String)> = Vec::new();
        for (name, value) in &response.headers {
            if let Some((_, previous)) = combined
                .iter_mut()
                .find(|(known, _)| bsl_rt::folded_eq(known, name))
            {
                // ИЗМЕРЕНО на 8.3.27: два `X-Duplicate: first`,
                // `X-Duplicate: second` дают `second, first`.
                *previous = format!("{value}, {previous}");
            } else {
                combined.push((name.clone(), value.clone()));
            }
        }
        for (name, value) in combined {
            headers.map_insert(string(&name), string(&value))?;
        }
        Ok(BslValue::new_object(Self { response, headers }))
    }
}

impl ObjectProtocol for HttpResponseObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &HTTP_RESPONSE_TYPE
    }

    fn property_table(&self) -> &'static [PropertyDescriptor] {
        RESPONSE_PROPERTIES
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        RESPONSE_METHODS
    }
}

fn response(receiver: &dyn ObjectProtocol) -> RtResult<&HttpResponseObject> {
    receiver
        .downcast_ref::<HttpResponseObject>()
        .ok_or(RtError::NotAnObject)
}

fn response_status(
    receiver: &dyn ObjectProtocol,
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    Ok(BslValue::number_from_i64(i64::from(
        response(receiver)?.response.status,
    )))
}

fn response_headers(
    receiver: &dyn ObjectProtocol,
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    Ok(response(receiver)?.headers.clone())
}

static RESPONSE_PROPERTIES: &[PropertyDescriptor] = &[
    PropertyDescriptor {
        names: &["КодСостояния", "StatusCode"],
        get: response_status,
        set: None,
    },
    PropertyDescriptor {
        names: &["Заголовки", "Headers"],
        get: response_headers,
        set: None,
    },
];

fn response_body_binary(
    receiver: &dyn ObjectProtocol,
    _arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    Ok(BslValue::binary_data_of(
        response(receiver)?.response.body.clone(),
    ))
}

fn response_body_string(
    receiver: &dyn ObjectProtocol,
    _arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let bytes = &response(receiver)?.response.body;
    Ok(BslValue::Str(BslString::from_utf8_string(
        Encoding::Utf8.decode(bytes)?,
    )))
}

fn response_body_stream(
    receiver: &dyn ObjectProtocol,
    _arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    memory_stream(&response(receiver)?.response.body)
}

static RESPONSE_METHODS: &[MethodDescriptor] = &[
    MethodDescriptor::new(
        &["ПолучитьТелоКакДвоичныеДанные", "GetBodyAsBinaryData"],
        Arity::exact(0),
        response_body_binary,
    ),
    MethodDescriptor::new(
        &["ПолучитьТелоКакСтроку", "GetBodyAsString"],
        Arity::exact(0),
        response_body_string,
    ),
    MethodDescriptor::new(
        &["ПолучитьТелоКакПоток", "GetBodyAsStream"],
        Arity::exact(0),
        response_body_stream,
    ),
];

#[derive(Debug)]
pub struct HttpConnectionObject {
    client: Arc<dyn HttpClient>,
}

impl ObjectProtocol for HttpConnectionObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &HTTP_CONNECTION_TYPE
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        CONNECTION_METHODS
    }
}

pub fn new_http_connection(
    arguments: &[BslValue],
    context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let host = required_string(arguments, 0, "Новый HTTPСоединение")?;
    let tls = match arguments.get(6) {
        None | Some(BslValue::Undefined) => TlsConfig::Plain,
        Some(value) => value
            .object_ref()
            .and_then(|object| object.downcast_ref::<SecureConnectionObject>())
            .map(|secure| secure.tls.clone())
            .ok_or_else(|| type_error("ЗащищенноеСоединениеOpenSSL", "Новый HTTPСоединение"))?,
    };
    // НЕ ИЗМЕРЕНО(HTTP.CONNECTION.DEFAULT_PORT): документация называет 80,
    // но сетевой oracle с реальной платформой ещё не снят.
    let default_port = if matches!(tls, TlsConfig::Plain) {
        80
    } else {
        443
    };
    let port = optional_u16(arguments, 1, default_port, "Новый HTTPСоединение")?;
    let username = optional_string(arguments, 2, "Новый HTTPСоединение")?;
    let password = optional_string(arguments, 3, "Новый HTTPСоединение")?
        .map_or_else(SecretString::default, SecretString::new);
    let proxy = proxy_mode(arguments.get(4), !matches!(tls, TlsConfig::Plain))?;
    // ИЗМЕРЕНО на 8.3.27: один deadline охватывает ожидание
    // заголовков и тела; короткие паузы в сумме его не сбрасывают.
    let timeout = optional_duration(arguments, 5, "Новый HTTPСоединение")?;
    let use_os_authentication = optional_bool(arguments, 7, false, "Новый HTTPСоединение")?;
    let config = HttpClientConfig {
        host,
        port,
        username,
        password,
        proxy,
        timeout,
        tls,
        client_identity: arguments
            .get(6)
            .and_then(BslValue::object_ref)
            .and_then(|object| object.downcast_ref::<SecureConnectionObject>())
            .and_then(|secure| secure.client_identity.clone()),
        use_os_authentication,
    };
    let client = context
        .network_rc()?
        .create(config)
        .map_err(network_to_rt)?;
    Ok(BslValue::new_object(HttpConnectionObject { client }))
}

fn connection(receiver: &dyn ObjectProtocol) -> RtResult<&HttpConnectionObject> {
    receiver
        .downcast_ref::<HttpConnectionObject>()
        .ok_or(RtError::NotAnObject)
}

/// Синхронный HTTP-метод не блокирует поток сам: запрос снимается с
/// BSL-объектов здесь, а транспорт и ожидание достаются вызывающей стороне
/// типизированной host-операцией. VM паркует execution (worker пула
/// заданий в это время исполняет другие задания), строковый путь доводит
/// операцию блокирующе — наблюдаемая семантика прежнего блокирующего
/// вызова в обоих случаях сохранена, включая ловимость ошибок транспорта.
fn call_sync(
    receiver: &dyn ObjectProtocol,
    method: &str,
    request_value: &BslValue,
) -> RtResult<CallOutcome> {
    let wire = snapshot_request(method, request_value)?;
    Ok(CallOutcome::Pending(PendingHostCall::HttpSync {
        client: Arc::clone(&connection(receiver)?.client),
        request: wire,
        mapper: materialize_response,
        error_mapper: network_to_rt,
    }))
}

fn snapshot_request(method: &str, request_value: &BslValue) -> RtResult<HttpWireRequest> {
    let request = request_value
        .object_ref()
        .and_then(|object| object.downcast_ref::<HttpRequestObject>())
        .ok_or_else(|| type_error("HTTPЗапрос", "HTTPСоединение"))?;
    let data = request.data.borrow();
    let headers = data
        .headers
        .map_entries()?
        .into_iter()
        .map(|(name, value)| {
            Ok((
                value_string(name, "заголовок HTTP")?,
                value_string(value, "заголовок HTTP")?,
            ))
        })
        .collect::<RtResult<Vec<_>>>()?;
    let body = data
        .body
        .byte_stream()
        .ok_or(RtError::NotAnObject)?
        .read_all("отправка HTTP-запроса")?;
    let wire = HttpWireRequest {
        method: method.to_string(),
        resource: data.resource.to_string(),
        headers,
        body,
    };
    drop(data);
    Ok(wire)
}

fn call_http_method(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<CallOutcome> {
    let method = required_string(arguments, 0, "ВызватьHTTPМетод")?;
    call_sync(receiver, &method, &arguments[1])
}

macro_rules! short_method {
    ($name:ident, $method:literal) => {
        fn $name(
            receiver: &dyn ObjectProtocol,
            arguments: &[BslValue],
            _context: &mut CallContext<'_>,
        ) -> RtResult<CallOutcome> {
            call_sync(receiver, $method, &arguments[0])
        }
    };
}

short_method!(delete, "DELETE");
short_method!(get, "GET");
short_method!(head, "HEAD");
short_method!(patch, "PATCH");
short_method!(post, "POST");
short_method!(put, "PUT");

fn materialize_response(
    result: Result<HttpWireResponse, NetworkError>,
    _runtime_shapes: &mut bsl_rt::RuntimeShapes,
) -> RtResult<BslValue> {
    HttpResponseObject::from_wire(result.map_err(network_to_rt)?)
}

fn call_async(
    receiver: &dyn ObjectProtocol,
    method: &str,
    request_value: &BslValue,
    context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let wire = snapshot_request(method, request_value)?;
    context.spawn_http(
        Arc::clone(&connection(receiver)?.client),
        wire,
        materialize_response,
        network_to_rt,
    )
}

fn call_http_method_async(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let method = required_string(arguments, 0, "ВызватьHTTPМетодАсинх")?;
    call_async(receiver, &method, &arguments[1], context)
}

macro_rules! short_async_method {
    ($name:ident, $method:literal) => {
        fn $name(
            receiver: &dyn ObjectProtocol,
            arguments: &[BslValue],
            context: &mut CallContext<'_>,
        ) -> RtResult<BslValue> {
            call_async(receiver, $method, &arguments[0], context)
        }
    };
}

short_async_method!(delete_async, "DELETE");
short_async_method!(get_async, "GET");
short_async_method!(head_async, "HEAD");
short_async_method!(patch_async, "PATCH");
short_async_method!(post_async, "POST");
short_async_method!(put_async, "PUT");

static CONNECTION_METHODS: &[MethodDescriptor] = &[
    MethodDescriptor::suspending(
        &["ВызватьHTTPМетод", "CallHTTPMethod"],
        Arity::exact(2),
        call_http_method,
    ),
    MethodDescriptor::suspending(&["Удалить", "Delete"], Arity::exact(1), delete),
    MethodDescriptor::suspending(&["Получить", "Get"], Arity::exact(1), get),
    MethodDescriptor::suspending(&["ПолучитьЗаголовки", "Head"], Arity::exact(1), head),
    MethodDescriptor::suspending(&["Изменить", "Patch"], Arity::exact(1), patch),
    MethodDescriptor::suspending(&["ОтправитьДляОбработки", "Post"], Arity::exact(1), post),
    MethodDescriptor::suspending(&["Записать", "Put"], Arity::exact(1), put),
    MethodDescriptor::new(
        &["ВызватьHTTPМетодАсинх", "CallHTTPMethodAsync"],
        Arity::exact(2),
        call_http_method_async,
    ),
    MethodDescriptor::new(
        &["УдалитьАсинх", "DeleteAsync"],
        Arity::exact(1),
        delete_async,
    ),
    MethodDescriptor::new(&["ПолучитьАсинх", "GetAsync"], Arity::exact(1), get_async),
    MethodDescriptor::new(
        &["ПолучитьЗаголовкиАсинх", "HeadAsync"],
        Arity::exact(1),
        head_async,
    ),
    MethodDescriptor::new(
        &["ИзменитьАсинх", "PatchAsync"],
        Arity::exact(1),
        patch_async,
    ),
    MethodDescriptor::new(
        &["ОтправитьДляОбработкиАсинх", "PostAsync"],
        Arity::exact(1),
        post_async,
    ),
    MethodDescriptor::new(&["ЗаписатьАсинх", "PutAsync"], Arity::exact(1), put_async),
];

fn memory_stream(bytes: &[u8]) -> RtResult<BslValue> {
    let stream = bsl_stream::new_memory_stream(&BslValue::Undefined)?;
    let protocol = stream.byte_stream().ok_or(RtError::NotAnObject)?;
    protocol.write_all(bytes, "HTTP-тело")?;
    protocol.set_position(0, "HTTP-тело")?;
    Ok(stream)
}

fn string(value: &str) -> BslValue {
    BslValue::Str(BslString::from_str(value))
}

fn value_string(value: BslValue, op: &'static str) -> RtResult<String> {
    match value {
        BslValue::Str(value) => Ok(value.to_string()),
        _ => Err(type_error("Строка", op)),
    }
}

fn required_string(arguments: &[BslValue], index: usize, op: &'static str) -> RtResult<String> {
    arguments
        .get(index)
        .cloned()
        .map(|value| value_string(value, op))
        .unwrap_or_else(|| Err(type_error("Строка", op)))
}

fn optional_string(
    arguments: &[BslValue],
    index: usize,
    op: &'static str,
) -> RtResult<Option<String>> {
    match arguments.get(index) {
        None | Some(BslValue::Undefined) => Ok(None),
        Some(BslValue::Str(value)) => Ok(Some(value.to_string())),
        _ => Err(type_error("Строка", op)),
    }
}

fn optional_u16(
    arguments: &[BslValue],
    index: usize,
    default: u16,
    op: &'static str,
) -> RtResult<u16> {
    match arguments.get(index) {
        None | Some(BslValue::Undefined) => Ok(default),
        Some(BslValue::Number(value)) => value
            .to_i64_exact()
            .and_then(|value| u16::try_from(value).ok())
            .ok_or_else(|| type_error("Порт 0..65535", op)),
        _ => Err(type_error("Число", op)),
    }
}

fn required_u16(arguments: &[BslValue], index: usize, op: &'static str) -> RtResult<u16> {
    match arguments.get(index) {
        Some(BslValue::Number(value)) => value
            .to_i64_exact()
            .and_then(|value| u16::try_from(value).ok())
            .ok_or_else(|| type_error("Порт 0..65535", op)),
        _ => Err(type_error("Число", op)),
    }
}

fn optional_duration(
    arguments: &[BslValue],
    index: usize,
    op: &'static str,
) -> RtResult<Option<Duration>> {
    match arguments.get(index) {
        None | Some(BslValue::Undefined) => Ok(None),
        Some(BslValue::Number(value)) => value
            .to_i64_exact()
            .and_then(|value| u64::try_from(value).ok())
            .map(Duration::from_secs)
            .map(Some)
            .ok_or_else(|| type_error("Целое неотрицательное число секунд", op)),
        _ => Err(type_error("Число", op)),
    }
}

fn optional_bool(
    arguments: &[BslValue],
    index: usize,
    default: bool,
    op: &'static str,
) -> RtResult<bool> {
    match arguments.get(index) {
        None | Some(BslValue::Undefined) => Ok(default),
        Some(BslValue::Boolean(value)) => Ok(*value),
        _ => Err(type_error("Булево", op)),
    }
}

fn type_error(expected: &'static str, op: &'static str) -> RtError {
    RtError::TypeError { expected, op }
}

fn http_error(kind: &'static str, message: impl Into<String>) -> RtError {
    ComponentError::raise(crate::PACKAGE_NAME, kind, message)
}

fn network_to_rt(error: NetworkError) -> RtError {
    http_error("transport", error.message)
}
