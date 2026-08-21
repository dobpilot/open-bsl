//! «УникальныйИдентификатор» — значение UUID из шестнадцати байтов.
//!
//! Конструктор без аргументов порождает случайный идентификатор версии 4:
//! шестнадцать байтов из источника случайности ПРОГОНА
//! (`bsl_rt::RandomSource`, см. модуль `env`) с выставленными битами
//! версии и варианта. Сам источник живёт там, а здесь остаётся чистая
//! функция над готовыми байтами — так тестовая последовательность не может
//! случайно выдать не-UUID.
//!
//! Строковая форма — `8-4-4-4-12` шестнадцатеричными цифрами в нижнем
//! регистре. Разбор принимает обе высоты цифр, печать всегда нижним
//! регистром — зафиксировано фикстурой `uuid`, эталон которой снят с
//! платформы: круг «строка -> УИД -> строка» приводит верхний регистр к
//! нижнему.

use crate::{RtError, RtResult};

/// Идентификатор версии 4 по RFC 4122 из шестнадцати сырых байтов:
/// тринадцатый шестнадцатеричный знак — всегда `4`, семнадцатый — из
/// `89ab`. Чистая функция: откуда байты, решает окружение прогона.
#[must_use]
pub fn v4_from_bytes(mut bytes: [u8; 16]) -> [u8; 16] {
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    bytes
}

fn hex_digit(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

fn bad_lexical() -> RtError {
    RtError::TypeError {
        expected: "строка вида xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx",
        op: "Новый УникальныйИдентификатор",
    }
}

/// Разбор канонической записи `8-4-4-4-12`. Регистр цифр безразличен;
/// другие формы (фигурные скобки, запись без дефисов) отвергаются.
///
/// # Errors
///
/// [`RtError::TypeError`] на любой строке не в канонической форме.
pub fn parse(text: &str) -> RtResult<[u8; 16]> {
    let bytes = text.as_bytes();
    if bytes.len() != 36 {
        return Err(bad_lexical());
    }
    let mut out = [0u8; 16];
    let mut nibbles = 0usize;
    for (i, &c) in bytes.iter().enumerate() {
        if matches!(i, 8 | 13 | 18 | 23) {
            if c != b'-' {
                return Err(bad_lexical());
            }
            continue;
        }
        let d = hex_digit(c).ok_or_else(bad_lexical)?;
        out[nibbles / 2] |= if nibbles.is_multiple_of(2) { d << 4 } else { d };
        nibbles += 1;
    }
    Ok(out)
}

/// Каноническая печать: нижний регистр, дефисы по позициям `8-4-4-4-12`.
pub fn format(bytes: &[u8; 16]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(36);
    for (i, b) in bytes.iter().enumerate() {
        if matches!(i, 4 | 6 | 8 | 10) {
            out.push('-');
        }
        out.push(HEX[usize::from(b >> 4)] as char);
        out.push(HEX[usize::from(b & 0x0f)] as char);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_lowers_the_case_and_keeps_the_bytes() {
        let b = parse("ABCDEF12-3456-7890-abcd-ef1234567890").expect("каноническая форма");
        assert_eq!(format(&b), "abcdef12-3456-7890-abcd-ef1234567890");
    }

    #[test]
    fn non_canonical_forms_are_errors() {
        for bad in [
            "",
            "abcdef1234567890abcdef1234567890",
            "{abcdef12-3456-7890-abcd-ef1234567890}",
            "abcdef12-3456-7890-abcd-ef123456789",
            "abcdef12-3456-7890-abcd-ef12345678900",
            "abcdef12+3456-7890-abcd-ef1234567890",
            "абвгдеё2-3456-7890-abcd-ef1234567890",
        ] {
            assert!(parse(bad).is_err(), "принята негодная форма: {bad:?}");
        }
    }

    /// Крайние входы, а не случайные: функция чистая, и проверять её на
    /// случайных байтах — значит проверять источник, а не её.
    #[test]
    fn v4_from_bytes_sets_version_and_variant_bits() {
        for raw in [[0x00u8; 16], [0xffu8; 16], [0x5au8; 16], [0xa5u8; 16]] {
            let b = v4_from_bytes(raw);
            assert_eq!(b[6] >> 4, 4, "версия не 4");
            assert_eq!(b[8] >> 6, 0b10, "вариант не RFC 4122");
            // Остальные четырнадцать байтов проходят насквозь: заданная
            // тестом последовательность обязана быть узнаваема в
            // результате, иначе подменять источник бессмысленно.
            for i in [0, 1, 2, 3, 4, 5, 7, 9, 10, 11, 12, 13, 14, 15] {
                assert_eq!(b[i], raw[i], "байт {i} изменён");
            }
            // А внутри шестого и восьмого сохраняются младшие биты.
            assert_eq!(b[6] & 0x0f, raw[6] & 0x0f);
            assert_eq!(b[8] & 0x3f, raw[8] & 0x3f);
        }
    }
}
