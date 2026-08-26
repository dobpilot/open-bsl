use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use open_bsl::{
    Engine, HttpClient, HttpClientConfig, HttpClientFactory, HttpCompletionSink, HttpWireRequest,
    HttpWireResponse, NetworkError, RandomSource, RequestHandle,
};

struct FixedRandom;

impl RandomSource for FixedRandom {
    fn fill(&mut self, buffer: &mut [u8; 16]) {
        *buffer = [0; 16];
    }
}

fn connector_source() -> String {
    include_str!("fixtures/connector-http-2.6.0.bsl").to_string()
}

#[test]
fn connector_multipart_and_gzip_round_trip_real_payloads() {
    let mut source = connector_source();
    source.push_str(
        r#"

Процедура ПроверитьGZip(Данные)
    Сжатые = ЗаписатьGZip(Данные);
    Если ПрочитатьGZip(Сжатые) <> Данные Тогда
        ВызватьИсключение "gzip round-trip";
    КонецЕсли;
КонецПроцедуры

Процедура ПроверитьGZipСМеткой(Данные, Метка)
    Попытка
        ПроверитьGZip(Данные);
    Исключение
        ВызватьИсключение Метка + ": "
            + ПодробноеПредставлениеОшибки(ИнформацияОбОшибке());
    КонецПопытки;
КонецПроцедуры

ПроверитьGZipСМеткой(Base64Значение(""), "gzip-empty");
ПроверитьGZipСМеткой(Base64Значение("AQ=="), "gzip-one-byte");
БольшойБуфер = Новый БуферДвоичныхДанных(70000);
БольшойБуфер.Установить(0, 1);
БольшойБуфер.Установить(69999, 255);
ПроверитьGZipСМеткой(
    ПолучитьДвоичныеДанныеИзБуфераДвоичныхДанных(БольшойБуфер), "gzip-large");

Процедура ПроверитьMultipart()
Поля = Новый Соответствие;
Поля.Вставить("field", "value");
Файлы = Новый Массив;
Файлы.Добавить(Новый Структура(
    "Имя,ИмяФайла,Данные,Тип",
    "upload", "a.bin", Base64Значение("WFla"), "application/octet-stream"));
Запрос = Новый HTTPЗапрос;
ТипСодержимого = ЗакодироватьФайлы(Запрос, Файлы, Поля);
Граница = "00000000000040008000000000000000";
Если ТипСодержимого <> "multipart/form-data; boundary=" + Граница Тогда
    ВызватьИсключение "multipart content type";
КонецЕсли;
НоваяСтрока = Символы.ВК + Символы.ПС;
Ожидаемое = "--" + Граница + НоваяСтрока
    + "Content-Disposition: form-data; name=""field""" + НоваяСтрока
    + НоваяСтрока + "value" + НоваяСтрока
    + "--" + Граница + НоваяСтрока
    + "Content-Disposition: form-data; name=""upload""; filename=""a.bin""" + НоваяСтрока
    + "Content-Type: application/octet-stream" + НоваяСтрока
    + НоваяСтрока + "XYZ" + НоваяСтрока
    + "--" + Граница + "--" + НоваяСтрока;
Если Запрос.ПолучитьТелоКакСтроку() <> Ожидаемое Тогда
    ВызватьИсключение "multipart body";
КонецЕсли;
КонецПроцедуры

Попытка
    ПроверитьMultipart();
Исключение
    ВызватьИсключение "multipart: "
        + ПодробноеПредставлениеОшибки(ИнформацияОбОшибке());
КонецПопытки;
"#,
    );

    let engine = Engine::builder().build().unwrap();
    let module = engine.compile(&source).unwrap();
    for jit in [false, true] {
        engine
            .state_builder()
            .jit(jit)
            .random(FixedRandom)
            .build()
            .run(&module)
            .unwrap();
    }
}

#[derive(Debug, Clone)]
struct ScriptedFactory {
    configs: Arc<Mutex<Vec<HttpClientConfig>>>,
    requests: Arc<Mutex<Vec<HttpWireRequest>>>,
    responses: Arc<Mutex<VecDeque<HttpWireResponse>>>,
}

impl HttpClientFactory for ScriptedFactory {
    fn create(&self, config: HttpClientConfig) -> Result<Arc<dyn HttpClient>, NetworkError> {
        self.configs.lock().unwrap().push(config);
        Ok(Arc::new(ScriptedClient {
            requests: Arc::clone(&self.requests),
            responses: Arc::clone(&self.responses),
        }))
    }
}

#[derive(Debug)]
struct ScriptedClient {
    requests: Arc<Mutex<Vec<HttpWireRequest>>>,
    responses: Arc<Mutex<VecDeque<HttpWireResponse>>>,
}

impl HttpClient for ScriptedClient {
    fn submit(
        &self,
        request: HttpWireRequest,
        sink: Box<dyn HttpCompletionSink>,
    ) -> Result<Box<dyn RequestHandle>, NetworkError> {
        self.requests.lock().unwrap().push(request);
        let response = self
            .responses
            .lock()
            .unwrap()
            .pop_front()
            .expect("для каждого запроса задан ответ");
        sink.complete(Ok(response));
        Ok(Box::new(ScriptedHandle))
    }
}

#[derive(Debug)]
struct ScriptedHandle;

impl RequestHandle for ScriptedHandle {
    fn cancel(&mut self) {}
}

fn response(status: u16, headers: &[(&str, &str)], body: &[u8]) -> HttpWireResponse {
    HttpWireResponse {
        status,
        headers: headers
            .iter()
            .map(|(name, value)| ((*name).to_string(), (*value).to_string()))
            .collect(),
        body: body.to_vec(),
    }
}

#[test]
fn connector_public_api_drives_requests_cookies_and_redirects() {
    let mut source = connector_source();
    source.push_str(
        r#"

Сессия = СоздатьСессию();
Сессия.Аутентификация = НоваяАутентификацияBearer("token-1");
ЛокальныйJSON = JsonВОбъект("{""probe"":true}", "utf-8");
Если ЛокальныйJSON.Получить("probe") <> Истина Тогда
    ВызватьИсключение "local-json";
КонецЕсли;
Параметры = Новый Соответствие;
Параметры.Вставить("q", "a b");
Если Get("http://example.test/start", Параметры, Неопределено, Сессия).КодСостояния <> 200 Тогда
    ВызватьИсключение "get";
КонецЕсли;

Данные = Новый Соответствие;
Данные.Вставить("name", "value x");
Если Post("http://example.test/form", Данные, Неопределено, Сессия).КодСостояния <> 200 Тогда
    ВызватьИсключение "form";
КонецЕсли;

Если Get("http://example.test/redirect", Неопределено, Неопределено, Сессия).КодСостояния <> 200 Тогда
    ВызватьИсключение "redirect";
КонецЕсли;

Если Post("http://example.test/preserve", "payload", Неопределено, Сессия).КодСостояния <> 200 Тогда
    ВызватьИсключение "307";
КонецЕсли;

Если Post("http://example.test/permanent", "payload-308", Неопределено, Сессия).КодСостояния <> 200 Тогда
    ВызватьИсключение "308";
КонецЕсли;

Json = Новый Структура("name", "value");
JsonОтвет = PostJson("http://example.test/json", Json, Неопределено, Сессия);
Если JsonОтвет.Получить("ok") <> Истина Тогда
    ВызватьИсключение "json";
КонецЕсли;

ПараметрыПовтора = НовыеПараметры();
ПараметрыПовтора.МаксимальноеКоличествоПовторов = 1;
ПараметрыПовтора.МаксимальноеВремяПовторов = 600;
Если Get("http://example.test/retry", Неопределено, ПараметрыПовтора, Сессия).КодСостояния <> 200 Тогда
    ВызватьИсключение "retry-after";
КонецЕсли;

СессияЛимита = СоздатьСессию();
СессияЛимита.МаксимальноеКоличествоПеренаправлений = 0;
ОшибкаЛимита = Ложь;
Попытка
    Get("http://example.test/limit", Неопределено, Неопределено, СессияЛимита);
Исключение
    ОшибкаЛимита = Истина;
КонецПопытки;
Если Не ОшибкаЛимита Тогда
    ВызватьИсключение "redirect-limit";
КонецЕсли;

BasicСессия = СоздатьСессию();
BasicСессия.Аутентификация = НоваяАутентификацияBasic("user", "password");
Если Get("http://basic.test/auth", Неопределено, Неопределено, BasicСессия).КодСостояния <> 200 Тогда
    ВызватьИсключение "basic";
КонецЕсли;
"#,
    );

    let engine = Engine::builder().build().unwrap();
    let module = engine.compile(&source).unwrap();
    for jit in [false, true] {
        let configs = Arc::new(Mutex::new(Vec::new()));
        let requests = Arc::new(Mutex::new(Vec::new()));
        let responses = Arc::new(Mutex::new(VecDeque::from([
            response(200, &[("Set-Cookie", "session=abc; Path=/")], b"get"),
            response(200, &[], b"form"),
            response(302, &[("Location", "http://redirect.test/final")], b""),
            response(200, &[], b"redirected"),
            response(307, &[("Location", "/preserved")], b""),
            response(200, &[], b"preserved"),
            response(308, &[("Location", "http://other.test/final-308")], b""),
            response(200, &[], b"preserved-308"),
            response(
                200,
                &[("Content-Type", "application/json")],
                br#"{"ok":true}"#,
            ),
            response(503, &[("Retry-After", "-1")], b"retry"),
            response(200, &[], b"retried"),
            response(302, &[("Location", "/over-limit")], b""),
            response(200, &[], b"must-still-trigger-limit"),
            response(200, &[], b"basic"),
        ])));
        let result = engine
            .state_builder()
            .jit(jit)
            .network(ScriptedFactory {
                configs: Arc::clone(&configs),
                requests: Arc::clone(&requests),
                responses: Arc::clone(&responses),
            })
            .build()
            .run(&module);
        if let Err(error) = result {
            panic!(
                "Connector flow, jit={jit}, requests={:?}, responses left={}: {error}",
                requests.lock().unwrap(),
                responses.lock().unwrap().len()
            );
        }

        assert!(responses.lock().unwrap().is_empty(), "jit={jit}");
        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 14, "jit={jit}");
        assert_eq!(requests[0].method, "GET");
        assert_eq!(requests[0].resource, "/start?q=a%20b");
        assert!(
            requests[0]
                .headers
                .iter()
                .any(|(name, value)| name == "Authorization" && value == "Bearer token-1")
        );
        assert_eq!(requests[1].method, "POST");
        assert_eq!(requests[1].resource, "/form");
        assert_eq!(requests[1].body, b"name=value%20x");
        assert!(
            requests[1]
                .headers
                .iter()
                .any(|(name, value)| name == "Cookie" && value == "session=abc")
        );
        assert_eq!(requests[2].resource, "/redirect");
        assert_eq!(requests[3].method, "GET");
        assert_eq!(requests[3].resource, "/final");
        assert!(requests[3].body.is_empty());
        assert_eq!(requests[4].resource, "/preserve");
        assert_eq!(requests[5].method, "POST");
        assert_eq!(requests[5].resource, "/preserved");
        assert_eq!(requests[5].body, b"payload");
        assert_eq!(requests[6].resource, "/permanent");
        assert_eq!(requests[7].method, "POST");
        assert_eq!(requests[7].resource, "/final-308");
        assert_eq!(requests[7].body, b"payload-308");
        assert_eq!(requests[8].resource, "/json");
        assert_eq!(requests[8].body, b"{\n \"name\": \"value\"\n}");
        assert_eq!(requests[9].resource, "/retry");
        assert_eq!(requests[10].resource, "/retry");
        assert_eq!(requests[11].resource, "/limit");
        assert_eq!(requests[12].resource, "/over-limit");
        assert_eq!(requests[13].resource, "/auth");
        drop(requests);

        let configs = configs.lock().unwrap();
        assert_eq!(configs.len(), 5, "пул соединений Connector, jit={jit}");
        assert_eq!(configs[1].host, "redirect.test");
        assert_eq!(configs[2].host, "other.test");
        assert_eq!(configs[4].host, "basic.test");
        assert_eq!(configs[4].username.as_deref(), Some("user"));
        assert_eq!(configs[4].password.expose(), "password");
    }
}

#[test]
fn connector_digest_and_aws4_match_independent_vectors() {
    let mut source = connector_source();
    source.push_str(
        r#"

DigestСессия = СоздатьСессию();
DigestСессия.Аутентификация = НоваяАутентификацияDigest("Mufasa", "Circle Of Life");
Если Get("http://digest.test/dir/index.html", Неопределено, Неопределено, DigestСессия).КодСостояния <> 200 Тогда
    ВызватьИсключение "digest";
КонецЕсли;

AWSСессия = СоздатьСессию();
AWSСессия.Аутентификация = НоваяАутентификацияAWS4(
    "AKIAIOSFODNN7EXAMPLE",
    "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY",
    "s3",
    "us-east-1");
AWSПараметры = НовыеПараметры();
AWSПараметры.Заголовки.Вставить("x-amz-date", "20130524T000000Z");
Если Get(
    "https://examplebucket.s3.amazonaws.com/?lifecycle=",
    Неопределено,
    AWSПараметры,
    AWSСессия).КодСостояния <> 200 Тогда
    ВызватьИсключение "aws4";
КонецЕсли;
"#,
    );

    let engine = Engine::builder().build().unwrap();
    let module = engine.compile(&source).unwrap();
    for jit in [false, true] {
        let configs = Arc::new(Mutex::new(Vec::new()));
        let requests = Arc::new(Mutex::new(Vec::new()));
        let responses = Arc::new(Mutex::new(VecDeque::from([
            response(
                401,
                &[(
                    "WWW-Authenticate",
                    "Digest realm=\"testrealm@host.com\", qop=\"auth\", nonce=\"dcd98b7102dd2f0e8b11d0f600bfb0c093\", opaque=\"5ccc069c403ebaf9f0171e9517f40e41\", algorithm=\"MD5\"",
                )],
                b"",
            ),
            response(200, &[], b"digest-ok"),
            response(200, &[], b"aws-ok"),
        ])));
        let result = engine
            .state_builder()
            .jit(jit)
            .random(FixedRandom)
            .network(ScriptedFactory {
                configs: Arc::clone(&configs),
                requests: Arc::clone(&requests),
                responses: Arc::clone(&responses),
            })
            .build()
            .run(&module);
        if let Err(error) = result {
            panic!(
                "Connector auth, jit={jit}, requests={:?}, responses left={}: {error}",
                requests.lock().unwrap(),
                responses.lock().unwrap().len()
            );
        }

        assert!(responses.lock().unwrap().is_empty(), "jit={jit}");
        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 3, "jit={jit}");
        assert_eq!(requests[0].resource, "/dir/index.html");
        assert_eq!(requests[1].resource, "/dir/index.html");
        assert_eq!(
            requests[1]
                .headers
                .iter()
                .find(|(name, _)| name.eq_ignore_ascii_case("authorization"))
                .map(|(_, value)| value.as_str()),
            Some(
                "Digest username=\"Mufasa\", realm=\"testrealm@host.com\", nonce=\"dcd98b7102dd2f0e8b11d0f600bfb0c093\", uri=\"/dir/index.html\", response=\"e14032188dd763dfad9092795bc08573\", opaque=\"5ccc069c403ebaf9f0171e9517f40e41\", algorithm=\"MD5\", qop=\"auth\", nc=00000001, cnonce=\"0000000000004000\""
            ),
            "jit={jit}"
        );

        assert_eq!(requests[2].resource, "/?lifecycle=");
        assert_eq!(
            requests[2]
                .headers
                .iter()
                .find(|(name, _)| name.eq_ignore_ascii_case("authorization"))
                .map(|(_, value)| value.as_str()),
            Some(
                "AWS4-HMAC-SHA256 Credential=AKIAIOSFODNN7EXAMPLE/20130524/us-east-1/s3/aws4_request, SignedHeaders=host;x-amz-content-sha256;x-amz-date, Signature=fea454ca298b7da1c68078a5d1bdbfbbe0d65c699e0f91ac7a200a0136783543"
            ),
            "jit={jit}"
        );
    }
}
