//! Кадрирование Debug Adapter Protocol.
//!
//! Кадр — заголовок `Content-Length: N`, пустая строка, ровно `N` БАЙТ
//! тела. Байт, а не символов: длина считается по UTF-8, и кириллица в
//! именах переменных занимает по два байта.
//!
//! Читатель обязан переживать и склейку, и разрыв: TCP границ сообщений
//! не хранит, поэтому одно чтение приносит то полтора кадра, то треть
//! одного. Отсюда буфер, который живёт между вызовами.

use std::io::{self, Read, Write};

/// Предел длины кадра.
///
/// Заголовок приходит из сети, и `Content-Length` в нём — чужое число.
/// Без предела выделение по нему укладывает процесс раньше, чем тот
/// успевает отказать: та же беда, от которой в разборе текстового
/// байт-кода стоит проверка «записей N, а строк осталось M».
const MAX_FRAME: usize = 16 * 1024 * 1024;

/// Собирает кадры из потока байтов, приходящих кусками произвольного
/// размера.
pub struct FrameReader {
    buf: Vec<u8>,
}

impl FrameReader {
    #[must_use]
    pub fn new() -> Self {
        Self { buf: Vec::new() }
    }

    /// Добавляет прочитанный кусок во внутренний буфер.
    pub fn feed(&mut self, chunk: &[u8]) {
        self.buf.extend_from_slice(chunk);
    }

    /// Извлекает следующий целый кадр, если он уже собрался.
    ///
    /// `Ok(None)` — данных пока мало; это не ошибка, надо дочитать.
    ///
    /// # Errors
    ///
    /// Заголовок без `Content-Length`, нечисловая длина, длина сверх
    /// предела или тело не в UTF-8.
    pub fn next_frame(&mut self) -> Result<Option<String>, String> {
        let Some(head_end) = find_header_end(&self.buf) else {
            return Ok(None);
        };
        let head = std::str::from_utf8(&self.buf[..head_end])
            .map_err(|_| "заголовок кадра не в UTF-8".to_string())?;
        let len = content_length(head)?;
        if len > MAX_FRAME {
            return Err(format!(
                "кадр длиной {len} байт — больше предела {MAX_FRAME}"
            ));
        }
        let body_start = head_end + 4;
        if self.buf.len() < body_start + len {
            return Ok(None);
        }
        let body = self.buf[body_start..body_start + len].to_vec();
        self.buf.drain(..body_start + len);
        String::from_utf8(body)
            .map(Some)
            .map_err(|_| "тело кадра не в UTF-8".to_string())
    }
}

impl Default for FrameReader {
    fn default() -> Self {
        Self::new()
    }
}

/// Конец заголовка — первая пустая строка, то есть `\r\n\r\n`.
fn find_header_end(buf: &[u8]) -> Option<usize> {
    buf.windows(4).position(|w| w == b"\r\n\r\n")
}

fn content_length(head: &str) -> Result<usize, String> {
    for line in head.split("\r\n") {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        // Имя заголовка нечувствительно к регистру — так требует HTTP, на
        // чьём синтаксисе построен DAP.
        if name.trim().eq_ignore_ascii_case("Content-Length") {
            return value
                .trim()
                .parse::<usize>()
                .map_err(|_| format!("Content-Length: «{}» не число", value.trim()));
        }
    }
    Err("в заголовке кадра нет Content-Length".to_string())
}

/// Пишет один кадр.
///
/// # Errors
///
/// Ошибка записи в поток.
pub fn write_frame(out: &mut impl Write, body: &str) -> io::Result<()> {
    write!(out, "Content-Length: {}\r\n\r\n", body.len())?;
    out.write_all(body.as_bytes())?;
    out.flush()
}

/// Читает кусок из потока в читатель кадров.
///
/// Возвращает `false`, когда поток закончился.
///
/// # Errors
///
/// Ошибка чтения из потока.
pub fn pump(src: &mut impl Read, reader: &mut FrameReader) -> io::Result<bool> {
    let mut chunk = [0u8; 4096];
    let n = src.read(&mut chunk)?;
    if n == 0 {
        return Ok(false);
    }
    reader.feed(&chunk[..n]);
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::{FrameReader, write_frame};

    fn framed(body: &str) -> Vec<u8> {
        let mut out = Vec::new();
        write_frame(&mut out, body).expect("запись в вектор");
        out
    }

    #[test]
    fn one_whole_frame_is_read() {
        let mut r = FrameReader::new();
        r.feed(&framed("{\"seq\":1}"));
        assert_eq!(r.next_frame().unwrap().as_deref(), Some("{\"seq\":1}"));
        assert_eq!(r.next_frame().unwrap(), None);
    }

    #[test]
    fn two_frames_glued_into_one_read_are_both_returned() {
        let mut r = FrameReader::new();
        let mut both = framed("{\"a\":1}");
        both.extend_from_slice(&framed("{\"b\":2}"));
        r.feed(&both);
        assert_eq!(r.next_frame().unwrap().as_deref(), Some("{\"a\":1}"));
        assert_eq!(r.next_frame().unwrap().as_deref(), Some("{\"b\":2}"));
        assert_eq!(r.next_frame().unwrap(), None);
    }

    #[test]
    fn a_frame_torn_across_reads_is_assembled() {
        let bytes = framed("{\"seq\":7}");
        // Рвём по каждому возможному месту: граница может лечь и внутрь
        // заголовка, и внутрь тела, и ровно между ними.
        for cut in 1..bytes.len() {
            let mut r = FrameReader::new();
            r.feed(&bytes[..cut]);
            assert_eq!(r.next_frame().unwrap(), None, "разрыв на {cut}");
            r.feed(&bytes[cut..]);
            assert_eq!(
                r.next_frame().unwrap().as_deref(),
                Some("{\"seq\":7}"),
                "разрыв на {cut}"
            );
        }
    }

    #[test]
    fn length_counts_bytes_not_characters() {
        // «да» — два символа и ЧЕТЫРЕ байта. Кадр, посчитанный в символах,
        // обрезал бы тело и увёл читатель на полкадра вперёд.
        let body = "{\"м\":\"да\"}";
        assert_eq!(body.len(), 13, "байт");
        assert_eq!(body.chars().count(), 10, "символов");
        let bytes = framed(body);
        // Заголовок обязан нести число БАЙТ, а оно здесь отличается от
        // числа символов — иначе тест ничего бы не доказывал.
        assert!(
            bytes.starts_with(b"Content-Length: 13\r\n\r\n"),
            "{}",
            String::from_utf8_lossy(&bytes)
        );
        let mut r = FrameReader::new();
        r.feed(&bytes);
        assert_eq!(r.next_frame().unwrap().as_deref(), Some(body));
    }

    #[test]
    fn a_header_without_content_length_is_an_error() {
        let mut r = FrameReader::new();
        r.feed(b"X-Other: 1\r\n\r\n{}");
        assert!(r.next_frame().is_err());
    }

    #[test]
    fn a_non_numeric_length_is_an_error() {
        let mut r = FrameReader::new();
        r.feed("Content-Length: много\r\n\r\n{}".as_bytes());
        assert!(r.next_frame().is_err());
    }

    #[test]
    fn an_absurd_length_is_refused_before_allocating() {
        let mut r = FrameReader::new();
        r.feed(b"Content-Length: 99999999999\r\n\r\n");
        assert!(r.next_frame().is_err());
    }

    #[test]
    fn the_header_name_is_case_insensitive() {
        let mut r = FrameReader::new();
        r.feed(b"content-length: 2\r\n\r\n{}");
        assert_eq!(r.next_frame().unwrap().as_deref(), Some("{}"));
    }
}
