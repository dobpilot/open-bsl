#![cfg(not(target_arch = "wasm32"))]

use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, mpsc};
use std::time::Duration;

use bsl_http::SystemHttpClientFactory;
use bsl_rt::{
    ClientIdentity, HttpClientConfig, HttpClientFactory, HttpCompletionSink, HttpWireRequest,
    HttpWireResponse, NetworkError, ProxyMode, RequestHandle, SecretBytes, SecretString, TlsConfig,
};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use rustls::server::WebPkiClientVerifier;
use rustls::{RootCertStore, ServerConfig, ServerConnection, StreamOwned};

// Изолированная тестовая PKI: закрытые ключи предназначены только для
// loopback-сервера и не являются учётными данными.
const CA_DER_BASE64: &str = "MIIBmjCCAUGgAwIBAgIURwpnrNEEbgjrukb2EubRkI7tbz8wCgYIKoZIzj0EAwIwGzEZMBcGA1UEAwwQb3Blbi1ic2wtdGVzdC1jYTAeFw0yNjA4MjYwODQyNDJaFw0zNjA4MjMwODQyNDJaMBsxGTAXBgNVBAMMEG9wZW4tYnNsLXRlc3QtY2EwWTATBgcqhkjOPQIBBggqhkjOPQMBBwNCAAQa4/ZIR+28A5zDLny2ZbBZYDm0PAs94KVbzRUKkFGSRPbhddF6/cXP2hLXF74BrCk2neky8qLGApNrkaKPMzlxo2MwYTAdBgNVHQ4EFgQUWkD9jsvkIGBTve4sndJ+XgRA18QwHwYDVR0jBBgwFoAUWkD9jsvkIGBTve4sndJ+XgRA18QwDwYDVR0TAQH/BAUwAwEB/zAOBgNVHQ8BAf8EBAMCAQYwCgYIKoZIzj0EAwIDRwAwRAIgKV6WttxfhIHT0ShIW2oiNX0eNFK/P2/kMCwu767ptJYCIArl6sUd2Mh4IIgJ18wntjy79u3VZILzh9hWM1Ruyxdb";
const SERVER_DER_BASE64: &str = "MIIBxDCCAWqgAwIBAgIUZPOrTsAwevyuhCNH0J8PWrAKuDIwCgYIKoZIzj0EAwIwGzEZMBcGA1UEAwwQb3Blbi1ic2wtdGVzdC1jYTAeFw0yNjA4MjYwODQ0MjFaFw0zNjA4MjMwODQ0MjFaMBQxEjAQBgNVBAMMCWxvY2FsaG9zdDBZMBMGByqGSM49AgEGCCqGSM49AwEHA0IABDaqIhZEdZU8GppuhboE/yIDvz1wMnx1SGHlqVr/DACsuqQZ9/8iarEZQbdNyPR5UYeUeEjJFb5FucgxTnm4OpqjgZIwgY8wGgYDVR0RBBMwEYIJbG9jYWxob3N0hwR/AAABMAwGA1UdEwEB/wQCMAAwDgYDVR0PAQH/BAQDAgeAMBMGA1UdJQQMMAoGCCsGAQUFBwMBMB0GA1UdDgQWBBR+XXNstBKnLabJugBTCo+7SAnjvTAfBgNVHSMEGDAWgBRaQP2Oy+QgYFO97iyd0n5eBEDXxDAKBggqhkjOPQQDAgNIADBFAiEAsvxnbekllpVLjmlEmQTNh247m3+bZoGw1NEaPgGb1lACICXDvCg7Etg4tZ7VsCfLeegJAsG8bC8NbiZ3V7Eo8YVa";
const SERVER_KEY_DER_BASE64: &str = "MIGHAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBG0wawIBAQQgtQvt6X9suHzflrjitLftRv2jUxF6D99wYILLtVXDnXChRANCAAQ2qiIWRHWVPBqaboW6BP8iA789cDJ8dUhh5ala/wwArLqkGff/ImqxGUG3Tcj0eVGHlHhIyRW+RbnIMU55uDqa";

const CA_PEM: &str = r#"-----BEGIN CERTIFICATE-----
MIIBmjCCAUGgAwIBAgIURwpnrNEEbgjrukb2EubRkI7tbz8wCgYIKoZIzj0EAwIw
GzEZMBcGA1UEAwwQb3Blbi1ic2wtdGVzdC1jYTAeFw0yNjA4MjYwODQyNDJaFw0z
NjA4MjMwODQyNDJaMBsxGTAXBgNVBAMMEG9wZW4tYnNsLXRlc3QtY2EwWTATBgcq
hkjOPQIBBggqhkjOPQMBBwNCAAQa4/ZIR+28A5zDLny2ZbBZYDm0PAs94KVbzRUK
kFGSRPbhddF6/cXP2hLXF74BrCk2neky8qLGApNrkaKPMzlxo2MwYTAdBgNVHQ4E
FgQUWkD9jsvkIGBTve4sndJ+XgRA18QwHwYDVR0jBBgwFoAUWkD9jsvkIGBTve4s
ndJ+XgRA18QwDwYDVR0TAQH/BAUwAwEB/zAOBgNVHQ8BAf8EBAMCAQYwCgYIKoZI
zj0EAwIDRwAwRAIgKV6WttxfhIHT0ShIW2oiNX0eNFK/P2/kMCwu767ptJYCIArl
6sUd2Mh4IIgJ18wntjy79u3VZILzh9hWM1Ruyxdb
-----END CERTIFICATE-----
"#;

const CLIENT_IDENTITY_PEM: &str = r#"-----BEGIN CERTIFICATE-----
MIIBsjCCAVegAwIBAgIUZPOrTsAwevyuhCNH0J8PWrAKuDEwCgYIKoZIzj0EAwIw
GzEZMBcGA1UEAwwQb3Blbi1ic2wtdGVzdC1jYTAeFw0yNjA4MjYwODQzMDNaFw0z
NjA4MjMwODQzMDNaMB8xHTAbBgNVBAMMFG9wZW4tYnNsLXRlc3QtY2xpZW50MFkw
EwYHKoZIzj0CAQYIKoZIzj0DAQcDQgAEQkS2z0KxnS0kadd6APJTHPzFd56F2zpz
xQbmQZgUIWe/aLjNNKLNY16BRMZWxMip0NOHlsWZTuIOmLBiQoPWEaN1MHMwDAYD
VR0TAQH/BAIwADAOBgNVHQ8BAf8EBAMCB4AwEwYDVR0lBAwwCgYIKwYBBQUHAwIw
HQYDVR0OBBYEFIjzV7+7udXcx/epdg37kLIud8uyMB8GA1UdIwQYMBaAFFpA/Y7L
5CBgU73uLJ3Sfl4EQNfEMAoGCCqGSM49BAMCA0kAMEYCIQDx852IzMd2SuRlIuGv
htkNJ4Ot9syVa6QB38eqviq2mgIhAI3932mgfskCt0m6zwCyij5yK7T/hHuJB8zo
w9GF7FsK
-----END CERTIFICATE-----
-----BEGIN PRIVATE KEY-----
MIGHAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBG0wawIBAQQgbblbNCAzH9dd5rBZ
dC3CaD2lZSlGuQRuSiZ73ncGwI+hRANCAARCRLbPQrGdLSRp13oA8lMc/MV3noXb
OnPFBuZBmBQhZ79ouM00os1jXoFExlbEyKnQ04eWxZlO4g6YsGJCg9YR
-----END PRIVATE KEY-----
"#;

struct ChannelSink(mpsc::Sender<Result<HttpWireResponse, NetworkError>>);

impl HttpCompletionSink for ChannelSink {
    fn complete(self: Box<Self>, result: Result<HttpWireResponse, NetworkError>) {
        let _ = self.0.send(result);
    }
}

fn decode(text: &str) -> Vec<u8> {
    bsl_rt::encoding::decode_base64(text).expect("test certificate is valid Base64")
}

fn tls_server_config(require_client_certificate: bool) -> Arc<ServerConfig> {
    let builder = ServerConfig::builder();
    let builder = if require_client_certificate {
        let mut roots = RootCertStore::empty();
        roots
            .add(CertificateDer::from(decode(CA_DER_BASE64)))
            .expect("test CA");
        let verifier = WebPkiClientVerifier::builder(Arc::new(roots))
            .build()
            .expect("client verifier");
        builder.with_client_cert_verifier(verifier)
    } else {
        builder.with_no_client_auth()
    };
    let certificate = CertificateDer::from(decode(SERVER_DER_BASE64));
    let key = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(decode(SERVER_KEY_DER_BASE64)));
    Arc::new(
        builder
            .with_single_cert(vec![certificate], key)
            .expect("server certificate and key"),
    )
}

fn exercise(
    host: &str,
    tls: TlsConfig,
    identity: Option<ClientIdentity>,
    require_client_certificate: bool,
) -> Result<HttpWireResponse, NetworkError> {
    let listener = TcpListener::bind("0.0.0.0:0").expect("local TLS server");
    let port = listener.local_addr().unwrap().port();
    let config = tls_server_config(require_client_certificate);
    let server = std::thread::spawn(move || -> Result<(), String> {
        let (stream, _) = listener.accept().map_err(|error| error.to_string())?;
        let connection = ServerConnection::new(config).map_err(|error| error.to_string())?;
        let mut stream = StreamOwned::new(connection, stream);
        let mut request = Vec::new();
        let mut buffer = [0u8; 1024];
        while !request.windows(4).any(|bytes| bytes == b"\r\n\r\n") {
            let count = stream
                .read(&mut buffer)
                .map_err(|error| error.to_string())?;
            if count == 0 {
                return Err("connection closed before request headers".to_string());
            }
            request.extend_from_slice(&buffer[..count]);
        }
        if !request.starts_with(b"GET /tls HTTP/1.1\r\n") {
            return Err(format!(
                "unexpected request: {}",
                String::from_utf8_lossy(&request)
            ));
        }
        stream
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 6\r\nConnection: close\r\n\r\ntls-ok")
            .map_err(|error| error.to_string())?;
        stream.flush().map_err(|error| error.to_string())?;
        Ok(())
    });

    let client = SystemHttpClientFactory.create(HttpClientConfig {
        host: host.to_string(),
        port,
        username: None,
        password: SecretString::default(),
        proxy: ProxyMode::Direct,
        timeout: Some(Duration::from_secs(5)),
        tls,
        client_identity: identity,
        use_os_authentication: false,
    })?;
    let (sender, receiver) = mpsc::channel();
    let _handle: Box<dyn RequestHandle> = client.submit(
        HttpWireRequest {
            method: "GET".to_string(),
            resource: "/tls".to_string(),
            headers: Vec::new(),
            body: Vec::new(),
        },
        Box::new(ChannelSink(sender)),
    )?;
    let result = receiver
        .recv_timeout(Duration::from_secs(5))
        .expect("TLS request completion");
    let server_result = server.join().expect("TLS server thread");
    if result.is_ok() {
        server_result.expect("successful client request reaches TLS server");
    }
    result
}

#[test]
fn tls_trust_hostname_and_insecure_modes_are_distinct() {
    let system_error = exercise("127.0.0.1", TlsConfig::SystemRoots, None, false)
        .expect_err("local CA is absent from system roots");
    assert!(!system_error.message.is_empty());

    let response = exercise(
        "127.0.0.1",
        TlsConfig::CustomRoots(vec![CA_PEM.as_bytes().to_vec()]),
        None,
        false,
    )
    .expect("explicit CA trusts the local server");
    assert_eq!(response.status, 200);
    assert_eq!(response.body, b"tls-ok");

    exercise(
        "127.0.0.2",
        TlsConfig::CustomRoots(vec![CA_PEM.as_bytes().to_vec()]),
        None,
        false,
    )
    .expect_err("trusted certificate with a wrong hostname is rejected");

    assert_eq!(
        exercise("127.0.0.2", TlsConfig::Insecure, None, false)
            .expect("explicit insecure mode accepts trust and hostname failures")
            .status,
        200
    );
}

#[test]
fn tls_client_certificate_is_sent_only_when_configured() {
    exercise(
        "127.0.0.1",
        TlsConfig::CustomRoots(vec![CA_PEM.as_bytes().to_vec()]),
        None,
        true,
    )
    .expect_err("mutual TLS server rejects a client without an identity");

    let response = exercise(
        "127.0.0.1",
        TlsConfig::CustomRoots(vec![CA_PEM.as_bytes().to_vec()]),
        Some(ClientIdentity {
            bytes: SecretBytes::new(CLIENT_IDENTITY_PEM.as_bytes().to_vec()),
            password: SecretString::default(),
        }),
        true,
    )
    .expect("mutual TLS succeeds with the configured client identity");
    assert_eq!(response.status, 200);
}
