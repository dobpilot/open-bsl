//! HTTP-клиент BSL и системный адаптер транспорта.

mod objects;
#[cfg(not(target_arch = "wasm32"))]
mod system;

pub use objects::{
    new_file_client_certificate, new_file_roots, new_http_connection, new_http_request,
    new_internet_proxy, new_secure_connection, new_system_roots,
};
#[cfg(not(target_arch = "wasm32"))]
pub use system::SystemHttpClientFactory;

/// Идентификатор компонента в заголовке байткода.
pub const PACKAGE_NAME: &str = env!("CARGO_PKG_NAME");
/// Версия компонента в заголовке байткода.
pub const PACKAGE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Системная HTTP-фабрика для [`bsl_rt::HostEnv`].
#[cfg(not(target_arch = "wasm32"))]
#[must_use]
pub const fn system_factory() -> SystemHttpClientFactory {
    SystemHttpClientFactory
}

/// Дескриптор HTTP-компонента. Объекты языка добавляются следующим
/// вертикальным срезом поверх уже готового host-контракта.
pub const fn library() -> bsl_rt::LibraryDescriptor {
    bsl_rt::LibraryDescriptor::new(
        PACKAGE_NAME,
        PACKAGE_VERSION,
        bsl_rt::ObjectContextNeed::Full,
    )
    .with_dependencies(&[bsl_rt::LibraryDependency {
        package: bsl_stream::PACKAGE_NAME,
        version: bsl_stream::PACKAGE_VERSION,
    }])
    .with_constructors(CONSTRUCTORS)
    .with_types(TYPES)
    .with_object_member_groups(OBJECT_MEMBER_GROUPS)
}

const OBJECT_MEMBER_GROUPS: &[&[bsl_rt::ObjectMembersDescriptor]] = &[objects::API_MEMBERS];

fn construct_request(
    _context: &mut bsl_rt::CallContext<'_>,
    arguments: &[bsl_rt::BslValue],
) -> bsl_rt::RtResult<bsl_rt::BslValue> {
    new_http_request(arguments.first().unwrap_or(&bsl_rt::BslValue::Undefined))
}

fn construct_connection(
    context: &mut bsl_rt::CallContext<'_>,
    arguments: &[bsl_rt::BslValue],
) -> bsl_rt::RtResult<bsl_rt::BslValue> {
    new_http_connection(arguments, context)
}

fn construct_proxy(
    _context: &mut bsl_rt::CallContext<'_>,
    arguments: &[bsl_rt::BslValue],
) -> bsl_rt::RtResult<bsl_rt::BslValue> {
    new_internet_proxy(arguments)
}

fn construct_system_roots(
    _context: &mut bsl_rt::CallContext<'_>,
    _arguments: &[bsl_rt::BslValue],
) -> bsl_rt::RtResult<bsl_rt::BslValue> {
    Ok(new_system_roots())
}

fn construct_secure_connection(
    _context: &mut bsl_rt::CallContext<'_>,
    arguments: &[bsl_rt::BslValue],
) -> bsl_rt::RtResult<bsl_rt::BslValue> {
    new_secure_connection(arguments)
}

fn construct_file_roots(
    context: &mut bsl_rt::CallContext<'_>,
    arguments: &[bsl_rt::BslValue],
) -> bsl_rt::RtResult<bsl_rt::BslValue> {
    new_file_roots(arguments, context)
}

fn construct_file_client_certificate(
    context: &mut bsl_rt::CallContext<'_>,
    arguments: &[bsl_rt::BslValue],
) -> bsl_rt::RtResult<bsl_rt::BslValue> {
    new_file_client_certificate(arguments, context)
}

const CONSTRUCTORS: &[bsl_rt::ConstructorDescriptor] = &[
    bsl_rt::ConstructorDescriptor {
        code: bsl_rt::ConstructorCode::new(1),
        names: &["HTTPЗапрос", "HTTPRequest"],
        arity: bsl_rt::Arity::range(0, 1),
        call: construct_request,
    },
    bsl_rt::ConstructorDescriptor {
        code: bsl_rt::ConstructorCode::new(2),
        names: &["HTTPСоединение", "HTTPConnection"],
        arity: bsl_rt::Arity::range(1, 8),
        call: construct_connection,
    },
    bsl_rt::ConstructorDescriptor {
        code: bsl_rt::ConstructorCode::new(3),
        names: &["ИнтернетПрокси", "InternetProxy"],
        arity: bsl_rt::Arity::range(0, 1),
        call: construct_proxy,
    },
    bsl_rt::ConstructorDescriptor {
        code: bsl_rt::ConstructorCode::new(4),
        names: &[
            "СертификатыУдостоверяющихЦентровОС",
            "CertificationAuthorityCertificatesOS",
        ],
        arity: bsl_rt::Arity::exact(0),
        call: construct_system_roots,
    },
    bsl_rt::ConstructorDescriptor {
        code: bsl_rt::ConstructorCode::new(5),
        names: &["ЗащищенноеСоединениеOpenSSL", "OpenSSLSecureConnection"],
        arity: bsl_rt::Arity::range(0, 2),
        call: construct_secure_connection,
    },
    bsl_rt::ConstructorDescriptor {
        code: bsl_rt::ConstructorCode::new(6),
        names: &[
            "СертификатыУдостоверяющихЦентровФайл",
            "FileCertificationAuthorityCertificates",
        ],
        arity: bsl_rt::Arity::exact(1),
        call: construct_file_roots,
    },
    bsl_rt::ConstructorDescriptor {
        code: bsl_rt::ConstructorCode::new(7),
        names: &["СертификатКлиентаФайл", "FileClientCertificate"],
        arity: bsl_rt::Arity::range(1, 2),
        call: construct_file_client_certificate,
    },
];

const TYPES: &[&bsl_rt::TypeDescriptor] = &[
    &objects::HTTP_REQUEST_TYPE,
    &objects::HTTP_RESPONSE_TYPE,
    &objects::HTTP_CONNECTION_TYPE,
    &objects::INTERNET_PROXY_TYPE,
    &objects::SYSTEM_ROOTS_TYPE,
    &objects::SECURE_CONNECTION_TYPE,
    &objects::FILE_ROOTS_TYPE,
    &objects::FILE_CLIENT_CERTIFICATE_TYPE,
    &objects::WINDOWS_CLIENT_CERTIFICATE_TYPE,
];
