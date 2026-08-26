//! Системный HTTP-адаптер: единственное место, где видны Tokio и reqwest.

use std::sync::{Arc, OnceLock};

use bsl_rt::{
    HttpClient, HttpClientConfig, HttpClientFactory, HttpCompletionSink, HttpWireRequest,
    HttpWireResponse, NetworkError, NetworkErrorKind, ProxyMode, RequestHandle, TlsConfig,
};

static RUNTIME: OnceLock<Result<tokio::runtime::Runtime, String>> = OnceLock::new();

fn runtime() -> Result<&'static tokio::runtime::Runtime, NetworkError> {
    RUNTIME
        .get_or_init(|| {
            tokio::runtime::Builder::new_multi_thread()
                .thread_name("open-bsl-http")
                .enable_all()
                .build()
                .map_err(|error| error.to_string())
        })
        .as_ref()
        .map_err(|message| NetworkError::new(NetworkErrorKind::Io, message.clone()))
}

/// Стандартная фабрика процесса. Состояния в ней нет: пул соединений
/// принадлежит каждому построенному `reqwest::Client`.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemHttpClientFactory;

impl HttpClientFactory for SystemHttpClientFactory {
    fn create(&self, config: HttpClientConfig) -> Result<Arc<dyn HttpClient>, NetworkError> {
        if config.use_os_authentication {
            return Err(NetworkError::new(
                NetworkErrorKind::Unsupported,
                "аутентификация операционной системы пока не поддерживается",
            ));
        }

        let mut builder = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .no_gzip()
            .no_brotli()
            .no_deflate()
            .no_zstd();
        if let Some(timeout) = config.timeout {
            builder = builder.timeout(timeout);
        }
        builder = match &config.proxy {
            ProxyMode::PlatformDefault => builder,
            ProxyMode::Direct => builder.no_proxy(),
            ProxyMode::Explicit(proxy) => {
                let mut exclusions = proxy.exclusions.clone();
                if proxy.exclude_local {
                    append_local_proxy_exclusions(&mut exclusions, &config.host);
                }
                let mut value = reqwest::Proxy::all(&proxy.url)
                    .map_err(|error| network_error(NetworkErrorKind::Proxy, error))?;
                if let Some(username) = &proxy.username {
                    value = value.basic_auth(username.expose(), proxy.password.expose());
                }
                value = value.no_proxy(reqwest::NoProxy::from_string(&exclusions.join(",")));
                builder.proxy(value)
            }
        };
        builder = match &config.tls {
            TlsConfig::Plain | TlsConfig::SystemRoots => builder,
            TlsConfig::Insecure => builder
                .danger_accept_invalid_certs(true)
                .danger_accept_invalid_hostnames(true),
            TlsConfig::CustomRoots(roots) => {
                let mut builder = builder;
                for root in roots {
                    let certificate = reqwest::Certificate::from_pem(root)
                        .or_else(|_| reqwest::Certificate::from_der(root))
                        .map_err(|error| network_error(NetworkErrorKind::Tls, error))?;
                    builder = builder.add_root_certificate(certificate);
                }
                builder
            }
        };
        if let Some(identity) = &config.client_identity {
            if !identity.password.is_empty() {
                return Err(NetworkError::new(
                    NetworkErrorKind::Unsupported,
                    "зашифрованный клиентский ключ пока не поддерживается rustls-адаптером",
                ));
            }
            let identity = reqwest::Identity::from_pem(identity.bytes.expose())
                .map_err(|error| network_error(NetworkErrorKind::Tls, error))?;
            builder = builder.identity(identity);
        }
        let client = builder.build().map_err(|error| classify_reqwest(&error))?;
        Ok(Arc::new(SystemHttpClient { client, config }))
    }
}

/// Правило `НеИспользоватьПроксиДляЛокальныхАдресов` измерено
/// на 8.3.27: точные loopback-адреса и имя без точки. Весь
/// `127.0.0.0/8` локальным не считается: `127.0.0.2` идёт через proxy.
fn append_local_proxy_exclusions(exclusions: &mut Vec<String>, host: &str) {
    exclusions.extend(["localhost", "127.0.0.1", "::1"].map(str::to_string));
    if !host.is_empty() && !host.contains(['.', ':', '[', ']']) {
        exclusions.push(host.to_string());
    }
}

#[derive(Debug)]
struct SystemHttpClient {
    client: reqwest::Client,
    config: HttpClientConfig,
}

impl HttpClient for SystemHttpClient {
    fn submit(
        &self,
        request: HttpWireRequest,
        sink: Box<dyn HttpCompletionSink>,
    ) -> Result<Box<dyn RequestHandle>, NetworkError> {
        let method = reqwest::Method::from_bytes(request.method.as_bytes())
            .map_err(|error| network_error(NetworkErrorKind::Protocol, error))?;
        let scheme = match self.config.tls {
            TlsConfig::Plain => "http",
            TlsConfig::SystemRoots | TlsConfig::CustomRoots(_) | TlsConfig::Insecure => "https",
        };
        let url = format!(
            "{scheme}://{}:{}{}",
            self.config.host, self.config.port, request.resource
        );
        let mut outgoing = self.client.request(method, url);
        for (name, value) in request.headers {
            outgoing = outgoing.header(name, value);
        }
        if let Some(username) = &self.config.username {
            outgoing = outgoing.basic_auth(username, Some(self.config.password.expose()));
        }
        outgoing = outgoing.body(request.body);

        let join = runtime()?.spawn(async move {
            let result = send(outgoing).await;
            sink.complete(result);
        });
        Ok(Box::new(SystemRequestHandle {
            abort: Some(join.abort_handle()),
        }))
    }
}

async fn send(request: reqwest::RequestBuilder) -> Result<HttpWireResponse, NetworkError> {
    let response = request
        .send()
        .await
        .map_err(|error| classify_reqwest(&error))?;
    let status = response.status().as_u16();
    let headers = response
        .headers()
        .iter()
        .map(|(name, value)| {
            (
                name.as_str().to_string(),
                String::from_utf8_lossy(value.as_bytes()).into_owned(),
            )
        })
        .collect();
    let body = response
        .bytes()
        .await
        .map_err(|error| classify_reqwest(&error))?
        .to_vec();
    Ok(HttpWireResponse {
        status,
        headers,
        body,
    })
}

#[derive(Debug)]
struct SystemRequestHandle {
    abort: Option<tokio::task::AbortHandle>,
}

impl RequestHandle for SystemRequestHandle {
    fn cancel(&mut self) {
        if let Some(abort) = self.abort.take() {
            abort.abort();
        }
    }
}

impl Drop for SystemRequestHandle {
    fn drop(&mut self) {
        self.cancel();
    }
}

fn network_error(kind: NetworkErrorKind, error: impl std::fmt::Display) -> NetworkError {
    NetworkError::new(kind, error.to_string())
}

fn classify_reqwest(error: &reqwest::Error) -> NetworkError {
    let kind = if error.is_timeout() {
        NetworkErrorKind::Timeout
    } else if error.is_builder() {
        NetworkErrorKind::Protocol
    } else if error.is_connect() {
        NetworkErrorKind::Io
    } else {
        NetworkErrorKind::Protocol
    };
    network_error(kind, error)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::mpsc;
    use std::time::Duration;

    struct ChannelSink(mpsc::Sender<Result<HttpWireResponse, NetworkError>>);

    impl HttpCompletionSink for ChannelSink {
        fn complete(self: Box<Self>, result: Result<HttpWireResponse, NetworkError>) {
            let _ = self.0.send(result);
        }
    }

    fn submit(
        client: &Arc<dyn HttpClient>,
        method: &str,
        resource: &str,
    ) -> (
        mpsc::Receiver<Result<HttpWireResponse, NetworkError>>,
        Box<dyn RequestHandle>,
    ) {
        let (sender, receiver) = mpsc::channel();
        let handle = client
            .submit(
                HttpWireRequest {
                    method: method.to_string(),
                    resource: resource.to_string(),
                    headers: Vec::new(),
                    body: Vec::new(),
                },
                Box::new(ChannelSink(sender)),
            )
            .expect("запуск запроса");
        (receiver, handle)
    }

    fn read_request(stream: &mut std::net::TcpStream) -> String {
        let mut request = Vec::new();
        let mut buffer = [0u8; 1024];
        while !request.windows(4).any(|bytes| bytes == b"\r\n\r\n") {
            let count = stream.read(&mut buffer).expect("чтение запроса");
            assert_ne!(count, 0, "соединение закрыто до конца заголовков");
            request.extend_from_slice(&buffer[..count]);
        }
        String::from_utf8(request).expect("ASCII-запрос")
    }

    fn test_config(tls: TlsConfig) -> HttpClientConfig {
        HttpClientConfig {
            host: "127.0.0.1".to_string(),
            port: 80,
            username: None,
            password: bsl_rt::SecretString::default(),
            proxy: ProxyMode::Direct,
            timeout: Some(Duration::from_secs(5)),
            tls,
            client_identity: None,
            use_os_authentication: false,
        }
    }

    /// BSL-конструкторы только снимают байты файла. `reqwest` принимает
    /// произвольный DER корень до рукопожатия, но непригодный клиентский
    /// PEM отвергает уже при создании транспорта.
    #[test]
    fn client_identity_format_is_checked_at_transport_creation() {
        assert!(
            SystemHttpClientFactory
                .create(test_config(TlsConfig::CustomRoots(vec![
                    b"not a certificate".to_vec(),
                ])))
                .is_ok()
        );

        let mut identity_config = test_config(TlsConfig::Insecure);
        identity_config.client_identity = Some(bsl_rt::ClientIdentity {
            bytes: bsl_rt::SecretBytes::new(b"not an identity".to_vec()),
            password: bsl_rt::SecretString::default(),
        });
        let identity = SystemHttpClientFactory.create(identity_config).unwrap_err();
        assert_eq!(identity.kind, NetworkErrorKind::Tls);
    }

    #[test]
    fn system_adapter_preserves_non_success_status_headers_and_body() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("локальный сервер");
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("подключение клиента");
            let mut buffer = [0u8; 4096];
            let count = stream.read(&mut buffer).expect("чтение запроса");
            let request = String::from_utf8_lossy(&buffer[..count]);
            assert!(request.starts_with("PATCH /resource?q=1 HTTP/1.1\r\n"));
            assert!(request.to_ascii_lowercase().contains("x-test: value\r\n"));
            assert!(request.ends_with("body"));
            stream
                .write_all(
                    b"HTTP/1.1 418 I'm a teapot\r\nContent-Length: 6\r\nContent-Encoding: gzip\r\nX-Reply: yes\r\nConnection: close\r\n\r\nanswer",
                )
                .expect("запись ответа");
        });

        let mut config = test_config(TlsConfig::Plain);
        config.host = address.ip().to_string();
        config.port = address.port();
        let client = SystemHttpClientFactory
            .create(config)
            .expect("построение клиента");
        let (sender, receiver) = mpsc::channel();
        let _handle = client
            .submit(
                HttpWireRequest {
                    method: "PATCH".into(),
                    resource: "/resource?q=1".into(),
                    headers: vec![("X-Test".into(), "value".into())],
                    body: b"body".to_vec(),
                },
                Box::new(ChannelSink(sender)),
            )
            .expect("запуск запроса");
        let response = receiver
            .recv_timeout(Duration::from_secs(5))
            .expect("результат запроса")
            .expect("HTTP-ответ");
        assert_eq!(response.status, 418);
        assert_eq!(response.body, b"answer");
        assert!(
            response
                .headers
                .iter()
                .any(|(name, value)| name == "x-reply" && value == "yes")
        );
        assert!(
            response
                .headers
                .iter()
                .any(|(name, value)| name == "content-encoding" && value == "gzip")
        );
        server.join().unwrap();
    }

    #[test]
    fn system_adapter_returns_every_status_class_without_following_redirects() {
        for (status, reason) in [
            (204, "No Content"),
            (302, "Found"),
            (404, "Not Found"),
            (503, "Unavailable"),
        ] {
            let listener = TcpListener::bind("127.0.0.1:0").expect("локальный сервер");
            let address = listener.local_addr().unwrap();
            let server = std::thread::spawn(move || {
                let (mut stream, _) = listener.accept().expect("подключение клиента");
                assert!(read_request(&mut stream).starts_with("GET /status HTTP/1.1\r\n"));
                write!(
                    stream,
                    "HTTP/1.1 {status} {reason}\r\nContent-Length: 0\r\nLocation: /must-not-follow\r\nConnection: close\r\n\r\n"
                )
                .expect("запись ответа");
            });

            let mut config = test_config(TlsConfig::Plain);
            config.host = address.ip().to_string();
            config.port = address.port();
            let client = SystemHttpClientFactory.create(config).unwrap();
            let (receiver, _handle) = submit(&client, "GET", "/status");
            let response = receiver
                .recv_timeout(Duration::from_secs(5))
                .expect("результат запроса")
                .expect("HTTP-ответ");
            assert_eq!(response.status, status);
            server.join().unwrap();
        }
    }

    #[test]
    fn system_adapter_classifies_a_response_deadline_as_timeout() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("локальный сервер");
        let address = listener.local_addr().unwrap();
        let (accepted_sender, accepted_receiver) = mpsc::channel();
        let (release_sender, release_receiver) = mpsc::channel();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("подключение клиента");
            let _ = read_request(&mut stream);
            accepted_sender.send(()).unwrap();
            let _ = release_receiver.recv_timeout(Duration::from_secs(5));
        });

        let mut config = test_config(TlsConfig::Plain);
        config.host = address.ip().to_string();
        config.port = address.port();
        config.timeout = Some(Duration::from_millis(100));
        let client = SystemHttpClientFactory.create(config).unwrap();
        let (receiver, _handle) = submit(&client, "GET", "/timeout");
        accepted_receiver
            .recv_timeout(Duration::from_secs(5))
            .expect("сервер принял запрос");
        let error = receiver
            .recv_timeout(Duration::from_secs(5))
            .expect("результат запроса")
            .unwrap_err();
        assert_eq!(error.kind, NetworkErrorKind::Timeout);
        release_sender.send(()).unwrap();
        server.join().unwrap();
    }

    #[test]
    fn system_adapter_keeps_two_requests_in_flight() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("локальный сервер");
        let address = listener.local_addr().unwrap();
        let (accepted_sender, accepted_receiver) = mpsc::channel();
        let (release_sender, release_receiver) = mpsc::channel();
        let server = std::thread::spawn(move || {
            let mut streams = Vec::new();
            for _ in 0..2 {
                let (mut stream, _) = listener.accept().expect("подключение клиента");
                let _ = read_request(&mut stream);
                streams.push(stream);
            }
            accepted_sender.send(()).unwrap();
            release_receiver
                .recv_timeout(Duration::from_secs(5))
                .expect("разрешение ответить");
            for mut stream in streams {
                stream
                    .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
                    .expect("запись ответа");
            }
        });

        let mut config = test_config(TlsConfig::Plain);
        config.host = address.ip().to_string();
        config.port = address.port();
        let client = SystemHttpClientFactory.create(config).unwrap();
        let (first, _first_handle) = submit(&client, "GET", "/first");
        let (second, _second_handle) = submit(&client, "GET", "/second");
        accepted_receiver
            .recv_timeout(Duration::from_secs(5))
            .expect("оба запроса одновременно дошли до сервера");
        release_sender.send(()).unwrap();
        for receiver in [first, second] {
            assert_eq!(
                receiver
                    .recv_timeout(Duration::from_secs(5))
                    .expect("результат запроса")
                    .unwrap()
                    .status,
                200
            );
        }
        server.join().unwrap();
    }

    #[test]
    fn system_adapter_reuses_an_idle_http_connection() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("локальный сервер");
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("первое подключение клиента");
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            for resource in ["/first", "/second"] {
                assert!(
                    read_request(&mut stream).starts_with(&format!("GET {resource} HTTP/1.1\r\n"))
                );
                stream
                    .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n")
                    .expect("запись ответа");
            }
        });

        let mut config = test_config(TlsConfig::Plain);
        config.host = address.ip().to_string();
        config.port = address.port();
        let client = SystemHttpClientFactory.create(config).unwrap();
        for resource in ["/first", "/second"] {
            let (receiver, _handle) = submit(&client, "GET", resource);
            assert_eq!(
                receiver
                    .recv_timeout(Duration::from_secs(5))
                    .expect("результат запроса")
                    .unwrap()
                    .status,
                200
            );
        }
        server.join().unwrap();
    }

    /// Платформа 8.3.27 принимает IPv6-адрес сервера в скобках
    /// и при флаге локальных адресов обходит явный proxy.
    #[test]
    fn bracketed_ipv6_loopback_bypasses_explicit_proxy() {
        let target = TcpListener::bind("[::1]:0").expect("IPv6-сервер");
        let target_address = target.local_addr().unwrap();
        let target_thread = std::thread::spawn(move || {
            let (mut stream, _) = target.accept().expect("прямое IPv6-подключение");
            let mut buffer = [0u8; 1024];
            let count = stream.read(&mut buffer).expect("чтение IPv6-запроса");
            assert!(
                String::from_utf8_lossy(&buffer[..count]).starts_with("GET /ipv6 HTTP/1.1\r\n")
            );
            stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
                .expect("запись IPv6-ответа");
        });

        let proxy = TcpListener::bind("127.0.0.1:0").expect("локальный proxy");
        proxy.set_nonblocking(true).unwrap();
        let proxy_address = proxy.local_addr().unwrap();
        let mut config = test_config(TlsConfig::Plain);
        config.host = "[::1]".to_string();
        config.port = target_address.port();
        config.proxy = ProxyMode::Explicit(bsl_rt::ProxyConfig {
            url: format!("http://{proxy_address}"),
            username: None,
            password: bsl_rt::SecretString::default(),
            exclusions: Vec::new(),
            exclude_local: true,
        });
        let client = SystemHttpClientFactory.create(config).unwrap();
        let (sender, receiver) = mpsc::channel();
        let _handle = client
            .submit(
                HttpWireRequest {
                    method: "GET".into(),
                    resource: "/ipv6".into(),
                    headers: Vec::new(),
                    body: Vec::new(),
                },
                Box::new(ChannelSink(sender)),
            )
            .unwrap();
        assert_eq!(
            receiver
                .recv_timeout(Duration::from_secs(5))
                .expect("результат IPv6-запроса")
                .unwrap()
                .status,
            200
        );
        target_thread.join().unwrap();
        assert!(matches!(
            proxy.accept(),
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock
        ));
    }

    #[test]
    fn explicit_proxy_receives_the_absolute_request_target() {
        let proxy = TcpListener::bind("127.0.0.1:0").expect("локальный proxy");
        let proxy_address = proxy.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = proxy.accept().expect("подключение к proxy");
            let request = read_request(&mut stream);
            assert!(
                request.starts_with("GET http://target.invalid:8080/through-proxy HTTP/1.1\r\n")
            );
            assert!(
                request
                    .to_ascii_lowercase()
                    .contains("proxy-authorization: basic ")
            );
            stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
                .expect("ответ proxy");
        });

        let mut config = test_config(TlsConfig::Plain);
        config.host = "target.invalid".to_string();
        config.port = 8080;
        config.proxy = ProxyMode::Explicit(bsl_rt::ProxyConfig {
            url: format!("http://{proxy_address}"),
            username: Some(bsl_rt::SecretString::new("proxy-user")),
            password: bsl_rt::SecretString::new("proxy-password"),
            exclusions: Vec::new(),
            exclude_local: false,
        });
        let client = SystemHttpClientFactory.create(config).unwrap();
        let (receiver, _handle) = submit(&client, "GET", "/through-proxy");
        assert_eq!(
            receiver
                .recv_timeout(Duration::from_secs(5))
                .expect("результат proxy-запроса")
                .unwrap()
                .status,
            200
        );
        server.join().unwrap();
    }

    #[test]
    fn local_proxy_exclusions_match_the_measured_host_classes() {
        let mut exclusions = Vec::new();
        append_local_proxy_exclusions(&mut exclusions, "single-label");
        assert!(exclusions.iter().any(|item| item == "single-label"));
        assert!(exclusions.iter().any(|item| item == "127.0.0.1"));
        assert!(exclusions.iter().any(|item| item == "::1"));

        for host in ["dotted.example", "127.0.0.2", "[::2]"] {
            let mut exclusions = Vec::new();
            append_local_proxy_exclusions(&mut exclusions, host);
            assert!(!exclusions.iter().any(|item| item == host));
        }
    }
}
