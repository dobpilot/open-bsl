use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug, PartialEq)]
pub struct ObservedRequest {
    request_line: String,
    body: Vec<u8>,
}

fn find_header_end(bytes: &[u8]) -> Option<usize> {
    bytes
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|position| position + 4)
}

fn read_request(stream: &mut TcpStream) -> ObservedRequest {
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .expect("тайм-аут локального соединения");
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 1024];
    let header_end = loop {
        let count = stream.read(&mut buffer).expect("чтение HTTP-запроса");
        assert_ne!(count, 0, "соединение закрылось до конца HTTP-заголовков");
        bytes.extend_from_slice(&buffer[..count]);
        if let Some(header_end) = find_header_end(&bytes) {
            break header_end;
        }
    };
    let (request_line, content_length) = {
        let headers = std::str::from_utf8(&bytes[..header_end]).expect("ASCII-заголовки запроса");
        let request_line = headers.lines().next().unwrap().to_string();
        let content_length = headers
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                name.eq_ignore_ascii_case("content-length").then(|| {
                    value
                        .trim()
                        .parse::<usize>()
                        .expect("Content-Length — число")
                })
            })
            .unwrap_or(0);
        (request_line, content_length)
    };
    while bytes.len() < header_end + content_length {
        let count = stream.read(&mut buffer).expect("чтение тела HTTP-запроса");
        assert_ne!(count, 0, "соединение закрылось до конца HTTP-тела");
        bytes.extend_from_slice(&buffer[..count]);
    }

    ObservedRequest {
        request_line,
        body: bytes[header_end..header_end + content_length].to_vec(),
    }
}

pub fn start_server() -> (
    u16,
    Arc<Mutex<Vec<ObservedRequest>>>,
    thread::JoinHandle<()>,
) {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("локальный HTTP-сервер");
    listener
        .set_nonblocking(true)
        .expect("неблокирующий accept");
    let port = listener.local_addr().unwrap().port();
    let observed = Arc::new(Mutex::new(Vec::new()));
    let thread_observed = Arc::clone(&observed);
    let handle = thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(10);
        while thread_observed.lock().unwrap().len() < 2 {
            let (mut stream, _) = match listener.accept() {
                Ok(connection) => connection,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    assert!(
                        Instant::now() < deadline,
                        "HTTP-клиент не прислал два запроса"
                    );
                    thread::sleep(Duration::from_millis(1));
                    continue;
                }
                Err(error) => panic!("accept локального HTTP-сервера: {error}"),
            };
            let request = read_request(&mut stream);
            let (status, body) = if request.request_line.starts_with("POST /sync ") {
                ("201 Created", b"sync-ok".as_slice())
            } else {
                assert!(
                    request.request_line.starts_with("GET /async "),
                    "неожиданный запрос: {}",
                    request.request_line
                );
                ("202 Accepted", b"async-ok".as_slice())
            };
            thread_observed.lock().unwrap().push(request);
            write!(
                stream,
                "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            )
            .expect("заголовок локального HTTP-ответа");
            stream.write_all(body).expect("тело локального HTTP-ответа");
        }
    });
    (port, observed, handle)
}

pub fn source(port: u16) -> String {
    format!(
        "Асинх Процедура ПроверитьАсинх(Соединение)\n\
         \tОтвет = Ждать Соединение.ПолучитьАсинх(Новый HTTPЗапрос(\"/async\"));\n\
         \tЕсли Ответ.КодСостояния <> 202 Или Ответ.ПолучитьТелоКакСтроку() <> \"async-ok\" Тогда\n\
         \t\tВызватьИсключение \"неверный асинхронный ответ\";\n\
         \tКонецЕсли;\n\
         КонецПроцедуры\n\
         Прокси = Новый ИнтернетПрокси(Ложь);\n\
         Соединение = Новый HTTPСоединение(\"127.0.0.1\", {port}, Неопределено, Неопределено, Прокси, 5);\n\
         Запрос = Новый HTTPЗапрос(\"/sync\");\n\
         Запрос.УстановитьТелоИзСтроки(\"payload\");\n\
         Ответ = Соединение.ОтправитьДляОбработки(Запрос);\n\
         Если Ответ.КодСостояния <> 201 Или Ответ.ПолучитьТелоКакСтроку() <> \"sync-ok\" Тогда\n\
         \tВызватьИсключение \"неверный синхронный ответ\";\n\
         КонецЕсли;\n\
         ПроверитьАсинх(Соединение);"
    )
}

pub fn assert_requests(observed: &Mutex<Vec<ObservedRequest>>) {
    assert_eq!(
        *observed.lock().unwrap(),
        [
            ObservedRequest {
                request_line: "POST /sync HTTP/1.1".into(),
                body: b"payload".to_vec(),
            },
            ObservedRequest {
                request_line: "GET /async HTTP/1.1".into(),
                body: Vec::new(),
            },
        ]
    );
}
