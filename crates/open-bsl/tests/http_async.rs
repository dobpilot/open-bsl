#![cfg(feature = "http")]

use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use open_bsl::{
    Engine, HttpClient, HttpClientConfig, HttpClientFactory, HttpCompletionSink, HttpWireRequest,
    HttpWireResponse, NetworkError, RequestHandle,
};

#[derive(Debug, Clone)]
struct ImmediateFactory {
    requests: Arc<Mutex<Vec<HttpWireRequest>>>,
}

impl HttpClientFactory for ImmediateFactory {
    fn create(&self, _config: HttpClientConfig) -> Result<Arc<dyn HttpClient>, NetworkError> {
        Ok(Arc::new(ImmediateClient {
            requests: Arc::clone(&self.requests),
        }))
    }
}

#[derive(Debug)]
struct ImmediateClient {
    requests: Arc<Mutex<Vec<HttpWireRequest>>>,
}

impl HttpClient for ImmediateClient {
    fn submit(
        &self,
        request: HttpWireRequest,
        sink: Box<dyn HttpCompletionSink>,
    ) -> Result<Box<dyn RequestHandle>, NetworkError> {
        self.requests.lock().unwrap().push(request);
        sink.complete(Ok(HttpWireResponse {
            status: 202,
            headers: vec![
                ("X-Test".into(), "first".into()),
                ("x-test".into(), "second".into()),
            ],
            body: vec![0xef, 0xbb, 0xbf, 0xd0, 0x90, 0xd0, 0xb1],
        }));
        Ok(Box::new(ImmediateHandle))
    }
}

#[derive(Debug)]
struct ImmediateHandle;

impl RequestHandle for ImmediateHandle {
    fn cancel(&mut self) {}
}

#[test]
fn all_async_http_methods_use_the_execution_promise_table() {
    let requests = Arc::new(Mutex::new(Vec::new()));
    let engine = Engine::builder().build().unwrap();
    let module = engine
        .compile(
            "Асинх Процедура Проверить()\n\
             Соединение = Новый HTTPСоединение(\"example.test\");\n\
             Запрос = Новый HTTPЗапрос(\"/resource\");\n\
             Запрос.УстановитьТелоИзДвоичныхДанных(Base64Значение(\"AQID\"));\n\
             Ждать Соединение.ВызватьHTTPМетодАсинх(\"OPTIONS\", Запрос);\n\
             Ждать Соединение.УдалитьАсинх(Запрос);\n\
             Ответ = Ждать Соединение.ПолучитьАсинх(Запрос);\n\
             Ждать Соединение.ПолучитьЗаголовкиАсинх(Запрос);\n\
             Ждать Соединение.ИзменитьАсинх(Запрос);\n\
             Ждать Соединение.ОтправитьДляОбработкиАсинх(Запрос);\n\
             Ждать Соединение.ЗаписатьАсинх(Запрос);\n\
             Если Ответ.КодСостояния <> 202 Тогда ВызватьИсключение \"status\"; КонецЕсли;\n\
             Если Ответ.Заголовки.Получить(\"X-Test\") <> \"second, first\" Тогда ВызватьИсключение \"headers\"; КонецЕсли;\n\
             Если ПолучитьHexСтрокуИзДвоичныхДанных(Ответ.ПолучитьТелоКакДвоичныеДанные()) <> \"EFBBBFD090D0B1\" Тогда ВызватьИсключение \"binary body\"; КонецЕсли;\n\
             Если Ответ.ПолучитьТелоКакСтроку() <> \"Аб\" Тогда ВызватьИсключение \"string body\"; КонецЕсли;\n\
             Если Ответ.ПолучитьТелоКакПоток().Размер() <> 7 Тогда ВызватьИсключение \"stream body\"; КонецЕсли;\n\
             КонецПроцедуры\n\
             Проверить();",
        )
        .unwrap();

    for jit in [false, true] {
        let factory = ImmediateFactory {
            requests: Arc::clone(&requests),
        };
        let mut state = engine.state_builder().network(factory).jit(jit).build();
        state.run(&module).unwrap();
    }

    let methods: Vec<_> = requests
        .lock()
        .unwrap()
        .iter()
        .map(|request| request.method.clone())
        .collect();
    assert_eq!(
        methods,
        [
            "OPTIONS", "DELETE", "GET", "HEAD", "PATCH", "POST", "PUT", "OPTIONS", "DELETE", "GET",
            "HEAD", "PATCH", "POST", "PUT"
        ]
    );
}

#[test]
fn all_sync_http_methods_use_the_same_wire_mapping() {
    let requests = Arc::new(Mutex::new(Vec::new()));
    let engine = Engine::builder().build().unwrap();
    let module = engine
        .compile(
            "Соединение = Новый HTTPСоединение(\"example.test\");\n\
             Запрос = Новый HTTPЗапрос(\"/resource\");\n\
             Соединение.ВызватьHTTPМетод(\"OPTIONS\", Запрос);\n\
             Соединение.Удалить(Запрос);\n\
             Ответ = Соединение.Получить(Запрос);\n\
             Соединение.ПолучитьЗаголовки(Запрос);\n\
             Соединение.Изменить(Запрос);\n\
             Соединение.ОтправитьДляОбработки(Запрос);\n\
             Соединение.Записать(Запрос);\n\
             Если Ответ.КодСостояния <> 202 Тогда ВызватьИсключение \"status\"; КонецЕсли;",
        )
        .unwrap();

    for jit in [false, true] {
        engine
            .state_builder()
            .network(ImmediateFactory {
                requests: Arc::clone(&requests),
            })
            .jit(jit)
            .build()
            .run(&module)
            .unwrap();
    }

    let methods: Vec<_> = requests
        .lock()
        .unwrap()
        .iter()
        .map(|request| request.method.clone())
        .collect();
    assert_eq!(
        methods,
        [
            "OPTIONS", "DELETE", "GET", "HEAD", "PATCH", "POST", "PUT", "OPTIONS", "DELETE", "GET",
            "HEAD", "PATCH", "POST", "PUT"
        ]
    );
}

#[derive(Default)]
struct ControlledTransport {
    sink: Mutex<Option<Box<dyn HttpCompletionSink>>>,
    request: Mutex<Option<HttpWireRequest>>,
    cancelled: AtomicBool,
}

#[derive(Clone)]
struct ControlledFactory(Arc<ControlledTransport>);

impl fmt::Debug for ControlledFactory {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ControlledFactory")
    }
}

impl HttpClientFactory for ControlledFactory {
    fn create(&self, _config: HttpClientConfig) -> Result<Arc<dyn HttpClient>, NetworkError> {
        Ok(Arc::new(ControlledClient(Arc::clone(&self.0))))
    }
}

struct ControlledClient(Arc<ControlledTransport>);

impl fmt::Debug for ControlledClient {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ControlledClient")
    }
}

impl HttpClient for ControlledClient {
    fn submit(
        &self,
        _request: HttpWireRequest,
        sink: Box<dyn HttpCompletionSink>,
    ) -> Result<Box<dyn RequestHandle>, NetworkError> {
        *self.0.request.lock().unwrap() = Some(_request);
        *self.0.sink.lock().unwrap() = Some(sink);
        Ok(Box::new(ControlledHandle(Arc::clone(&self.0))))
    }
}

struct ControlledHandle(Arc<ControlledTransport>);

impl fmt::Debug for ControlledHandle {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ControlledHandle")
    }
}

impl RequestHandle for ControlledHandle {
    fn cancel(&mut self) {
        self.0.cancelled.store(true, Ordering::SeqCst);
    }
}

fn one_async_get(engine: &Engine) -> open_bsl::Module {
    engine
        .compile(
            "Асинх Процедура Получить()\n\
             Соединение = Новый HTTPСоединение(\"example.test\");\n\
             Ждать Соединение.ПолучитьАсинх(Новый HTTPЗапрос(\"/\"));\n\
             КонецПроцедуры\n\
             Получить();",
        )
        .unwrap()
}

#[test]
fn finite_poll_waits_without_blocking_and_state_is_reusable_after_completion() {
    let transport = Arc::new(ControlledTransport::default());
    let engine = Engine::builder().build().unwrap();
    let module = one_async_get(&engine);
    let mut state = engine
        .state_builder()
        .network(ControlledFactory(Arc::clone(&transport)))
        .build();
    let mut execution = state.start(&module).unwrap();

    assert_eq!(execution.poll(1).unwrap(), open_bsl::ExecutionPoll::Waiting);
    transport
        .sink
        .lock()
        .unwrap()
        .take()
        .unwrap()
        .complete(Ok(HttpWireResponse {
            status: 200,
            headers: Vec::new(),
            body: Vec::new(),
        }));
    assert_eq!(
        execution.poll(1).unwrap(),
        open_bsl::ExecutionPoll::Complete(open_bsl::Value::Undefined)
    );
    assert!(execution.poll(1).is_err());
    drop(execution);

    assert_eq!(state.eval("2 + 2").unwrap().to_string(), "4");
}

#[test]
fn dropping_waiting_execution_cancels_request_and_ignores_late_completion() {
    let transport = Arc::new(ControlledTransport::default());
    let engine = Engine::builder().build().unwrap();
    let module = one_async_get(&engine);
    let mut state = engine
        .state_builder()
        .network(ControlledFactory(Arc::clone(&transport)))
        .build();
    let mut execution = state.start(&module).unwrap();

    assert_eq!(execution.poll(1).unwrap(), open_bsl::ExecutionPoll::Waiting);
    drop(execution);
    assert!(transport.cancelled.load(Ordering::SeqCst));

    transport
        .sink
        .lock()
        .unwrap()
        .take()
        .unwrap()
        .complete(Ok(HttpWireResponse {
            status: 200,
            headers: Vec::new(),
            body: Vec::new(),
        }));
    assert_eq!(state.eval("6 * 7").unwrap().to_string(), "42");
}

#[test]
fn async_request_is_snapshotted_and_transport_error_is_raised_at_await() {
    let transport = Arc::new(ControlledTransport::default());
    let engine = Engine::builder().build().unwrap();
    let module = engine
        .compile(
            "Асинх Процедура Проверить()\n\
             Соединение = Новый HTTPСоединение(\"example.test\");\n\
             Запрос = Новый HTTPЗапрос(\"/before\");\n\
             Запрос.Заголовки.Вставить(\"X-Version\", \"before\");\n\
             Обещание = Соединение.ОтправитьДляОбработкиАсинх(Запрос);\n\
             Запрос.АдресРесурса = \"/after\";\n\
             Запрос.Заголовки.Вставить(\"X-Version\", \"after\");\n\
             Ошибка = Ложь;\n\
             Попытка\n\
                 Ждать Обещание;\n\
             Исключение\n\
                 Ошибка = Истина;\n\
             КонецПопытки;\n\
             Если Не Ошибка Тогда ВызватьИсключение \"ошибка транспорта потеряна\"; КонецЕсли;\n\
             КонецПроцедуры\n\
             Проверить();",
        )
        .unwrap();
    let mut state = engine
        .state_builder()
        .network(ControlledFactory(Arc::clone(&transport)))
        .build();
    let mut execution = state.start(&module).unwrap();

    assert_eq!(execution.poll(1).unwrap(), open_bsl::ExecutionPoll::Waiting);
    let request = transport.request.lock().unwrap().clone().unwrap();
    assert_eq!(request.resource, "/before");
    assert_eq!(request.headers, [("X-Version".into(), "before".into())]);

    transport
        .sink
        .lock()
        .unwrap()
        .take()
        .unwrap()
        .complete(Err(NetworkError::new(
            open_bsl::NetworkErrorKind::Io,
            "искусственный отказ",
        )));
    assert_eq!(
        execution.poll(1).unwrap(),
        open_bsl::ExecutionPoll::Complete(open_bsl::Value::Undefined)
    );
}

#[derive(Debug, Clone)]
struct ConfigFactory {
    configs: Arc<Mutex<Vec<HttpClientConfig>>>,
}

impl HttpClientFactory for ConfigFactory {
    fn create(&self, config: HttpClientConfig) -> Result<Arc<dyn HttpClient>, NetworkError> {
        self.configs.lock().unwrap().push(config);
        Ok(Arc::new(ImmediateClient {
            requests: Arc::new(Mutex::new(Vec::new())),
        }))
    }
}

/// BSL-объекты proxy/TLS преобразуются в нейтральный host-контракт при
/// создании соединения. Фабрика видит политику, но не BSL-объекты.
#[test]
fn proxy_and_tls_objects_become_transport_configuration() {
    let configs = Arc::new(Mutex::new(Vec::new()));
    let engine = Engine::builder().build().unwrap();
    let module = engine
        .compile(
            "СистемныйПрокси = Новый ИнтернетПрокси;\n\
             ПрямойПрокси = Новый ИнтернетПрокси(Ложь);\n\
             ЯвныйПрокси = Новый ИнтернетПрокси(Ложь);\n\
             ЯвныйПрокси.Установить(\"http\", \"proxy.test\", 3128);\n\
             ЯвныйПрокси.Пользователь = \"proxy-user\";\n\
             ЯвныйПрокси.Пароль = \"proxy-password\";\n\
             ЯвныйПрокси.НеИспользоватьПроксиДляАдресов.Добавить(\"internal.test\");\n\
             ЯвныйПрокси.НеИспользоватьПроксиДляЛокальныхАдресов = Ложь;\n\
             Корни = Новый СертификатыУдостоверяющихЦентровОС;\n\
             TLS = Новый ЗащищенноеСоединениеOpenSSL(Неопределено, Корни);\n\
             БезПроверки = Новый ЗащищенноеСоединениеOpenSSL;\n\
             С1 = Новый HTTPСоединение(\"plain.test\", Неопределено, Неопределено, Неопределено, СистемныйПрокси);\n\
             С2 = Новый HTTPСоединение(\"direct.test\", Неопределено, Неопределено, Неопределено, ПрямойПрокси);\n\
             СЯ = Новый HTTPСоединение(\"explicit.test\", Неопределено, Неопределено, Неопределено, ЯвныйПрокси);\n\
             С3 = Новый HTTPСоединение(\"secure.test\", Неопределено, Неопределено, Неопределено, Неопределено, Неопределено, TLS);\n\
             С4 = Новый HTTPСоединение(\"insecure.test\", Неопределено, Неопределено, Неопределено, Неопределено, Неопределено, БезПроверки);\n\
             Путь = ПолучитьИмяВременногоФайла(\".pem\");\n\
             ПолучитьДвоичныеДанныеИзСтроки(\"не сертификат\", КодировкаТекста.UTF8, Ложь).Записать(Путь);\n\
             КорниФайл = Новый СертификатыУдостоверяющихЦентровФайл(Путь);\n\
             КлиентФайл = Новый СертификатКлиентаФайл(Путь, \"\");\n\
             TLSФайл = Новый ЗащищенноеСоединениеOpenSSL(КлиентФайл, КорниФайл);\n\
             С5 = Новый HTTPСоединение(\"file.test\", Неопределено, Неопределено, Неопределено, Неопределено, Неопределено, TLSФайл);\n\
             УдалитьФайлы(Путь);",
        )
        .unwrap();
    engine
        .state_builder()
        .network(ConfigFactory {
            configs: Arc::clone(&configs),
        })
        .build()
        .run(&module)
        .unwrap();

    let configs = configs.lock().unwrap();
    assert_eq!(configs.len(), 6);
    assert!(matches!(
        configs[0].proxy,
        open_bsl::ProxyMode::PlatformDefault
    ));
    assert!(matches!(configs[1].proxy, open_bsl::ProxyMode::Direct));
    assert_eq!(configs[0].port, 80);
    assert_eq!(configs[1].port, 80);
    let open_bsl::ProxyMode::Explicit(proxy) = &configs[2].proxy else {
        panic!("ожидался явный прокси: {:?}", configs[2].proxy);
    };
    assert_eq!(proxy.url, "http://proxy.test:3128");
    assert_eq!(proxy.username.as_ref().unwrap().expose(), "proxy-user");
    assert_eq!(proxy.password.expose(), "proxy-password");
    assert_eq!(proxy.exclusions, ["internal.test"]);
    assert!(!proxy.exclude_local);
    assert!(matches!(configs[3].tls, open_bsl::TlsConfig::SystemRoots));
    assert!(matches!(configs[4].tls, open_bsl::TlsConfig::Insecure));
    assert_eq!(configs[3].port, 443);
    assert_eq!(configs[4].port, 443);
    assert!(matches!(
        configs[5].tls,
        open_bsl::TlsConfig::CustomRoots(_)
    ));
    assert_eq!(
        configs[5].client_identity.as_ref().unwrap().bytes.expose(),
        "не сертификат".as_bytes()
    );
}

/// Синхронный метод паркует execution, а не блокирует поток: пока
/// транспорт не ответил, poll возвращает `Waiting`, и поток свободен для
/// другой работы (worker пула заданий в это время исполняет соседние
/// задания). Ответ, доставленный из чужого потока, ложится в регистр
/// назначения вызова, и исполнение продолжается за инструкцией.
#[test]
fn a_sync_method_parks_the_execution_instead_of_blocking() {
    let transport = Arc::new(ControlledTransport::default());
    let engine = Engine::builder().build().unwrap();
    let module = engine
        .compile(
            "Соединение = Новый HTTPСоединение(\"example.test\");\n\
             Ответ = Соединение.Получить(Новый HTTPЗапрос(\"/parked\"));\n\
             Если Ответ.КодСостояния <> 200 Тогда ВызватьИсключение \"status\"; КонецЕсли;\n\
             Если Ответ.ПолучитьТелоКакСтроку() <> \"тело\" Тогда ВызватьИсключение \"body\"; КонецЕсли;",
        )
        .unwrap();
    let mut state = engine
        .state_builder()
        .network(ControlledFactory(Arc::clone(&transport)))
        .build();
    let mut execution = state.start(&module).unwrap();

    assert_eq!(execution.poll(1).unwrap(), open_bsl::ExecutionPoll::Waiting);
    let request = transport.request.lock().unwrap().clone().unwrap();
    assert_eq!(request.method, "GET");
    assert_eq!(request.resource, "/parked");

    transport
        .sink
        .lock()
        .unwrap()
        .take()
        .unwrap()
        .complete(Ok(HttpWireResponse {
            status: 200,
            headers: Vec::new(),
            body: "тело".as_bytes().to_vec(),
        }));
    assert_eq!(
        execution.poll(1).unwrap(),
        open_bsl::ExecutionPoll::Complete(open_bsl::Value::Undefined)
    );
}

/// Ошибка транспорта припаркованного синхронного вызова разматывается с
/// `pc` на самой инструкции — «Попытка» вокруг вызова ловит её, как при
/// прежнем блокирующем пути.
#[test]
fn a_sync_transport_error_is_catchable_at_the_call_site() {
    let transport = Arc::new(ControlledTransport::default());
    let engine = Engine::builder().build().unwrap();
    let module = engine
        .compile(
            "Соединение = Новый HTTPСоединение(\"example.test\");\n\
             Ошибка = Ложь;\n\
             Попытка\n\
                 Соединение.Получить(Новый HTTPЗапрос(\"/\"));\n\
             Исключение\n\
                 Ошибка = Истина;\n\
             КонецПопытки;\n\
             Если Не Ошибка Тогда ВызватьИсключение \"ошибка транспорта потеряна\"; КонецЕсли;",
        )
        .unwrap();
    let mut state = engine
        .state_builder()
        .network(ControlledFactory(Arc::clone(&transport)))
        .build();
    let mut execution = state.start(&module).unwrap();

    assert_eq!(execution.poll(1).unwrap(), open_bsl::ExecutionPoll::Waiting);
    transport
        .sink
        .lock()
        .unwrap()
        .take()
        .unwrap()
        .complete(Err(NetworkError::new(
            open_bsl::NetworkErrorKind::Io,
            "искусственный отказ",
        )));
    assert_eq!(
        execution.poll(1).unwrap(),
        open_bsl::ExecutionPoll::Complete(open_bsl::Value::Undefined)
    );
}

/// Сброс execution, припаркованного на синхронном вызове, отменяет
/// host-операцию: транспорт получает `cancel`, поздний ответ никому не
/// доставляется.
#[test]
fn dropping_an_execution_parked_on_a_sync_call_cancels_the_request() {
    let transport = Arc::new(ControlledTransport::default());
    let engine = Engine::builder().build().unwrap();
    let module = engine
        .compile(
            "Соединение = Новый HTTPСоединение(\"example.test\");\n\
             Соединение.Получить(Новый HTTPЗапрос(\"/\"));",
        )
        .unwrap();
    let mut state = engine
        .state_builder()
        .network(ControlledFactory(Arc::clone(&transport)))
        .build();
    let mut execution = state.start(&module).unwrap();

    assert_eq!(execution.poll(1).unwrap(), open_bsl::ExecutionPoll::Waiting);
    drop(execution);
    assert!(transport.cancelled.load(Ordering::SeqCst));

    transport
        .sink
        .lock()
        .unwrap()
        .take()
        .unwrap()
        .complete(Ok(HttpWireResponse {
            status: 200,
            headers: Vec::new(),
            body: Vec::new(),
        }));
    assert_eq!(state.eval("6 * 7").unwrap().to_string(), "42");
}
