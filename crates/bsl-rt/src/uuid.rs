//! «УникальныйИдентификатор» — значение UUID из шестнадцати байтов.
//!
//! Конструктор без аргументов порождает случайный идентификатор версии 4:
//! шестнадцать байтов из системного источника случайности с выставленными
//! битами версии и варианта. Байты читаются из `/dev/urandom` (дескриптор
//! держится открытым на поток), а там, где источника нет, — выводятся из
//! ключей `RandomState`, которые стандартная библиотека сеет от ОС.
//! Криптографическая стойкость не обещается: УИД платформы — идентификатор
//! обмена, а не секрет.
//!
//! Строковая форма — `8-4-4-4-12` шестнадцатеричными цифрами в нижнем
//! регистре. Разбор принимает обе высоты цифр, печать всегда нижним
//! регистром — зафиксировано фикстурой `uuid`, эталон которой снят с
//! платформы: круг «строка -> УИД -> строка» приводит верхний регистр к
//! нижнему.

use crate::{RtError, RtResult};
use std::cell::RefCell;
use std::io::Read;

thread_local! {
    /// Открытый `/dev/urandom` на поток: дескриптор дешевле открывать один
    /// раз, чем на каждый идентификатор.
    static URANDOM: RefCell<Option<std::fs::File>> = const { RefCell::new(None) };
}

fn fill_from_urandom(buf: &mut [u8; 16]) -> bool {
    URANDOM.with(|slot| {
        let mut slot = slot.borrow_mut();
        if slot.is_none() {
            *slot = std::fs::File::open("/dev/urandom").ok();
        }
        match slot.as_mut() {
            Some(f) => f.read_exact(buf).is_ok(),
            None => false,
        }
    })
}

/// Запасной источник без `/dev/urandom`: два независимых `RandomState`
/// приходят со случайными ключами от ОС, и их хеши дают шестнадцать
/// байтов. Качество ниже криптографического, но для идентификатора
/// достаточно.
fn fill_from_random_state(buf: &mut [u8; 16]) {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};
    for half in 0..2 {
        let mut h = RandomState::new().build_hasher();
        h.write_u64(half as u64);
        buf[half * 8..][..8].copy_from_slice(&h.finish().to_le_bytes());
    }
}

/// Случайный идентификатор версии 4 по RFC 4122: тринадцатый
/// шестнадцатеричный знак — всегда `4`, семнадцатый — из `89ab`.
pub fn random_v4() -> [u8; 16] {
    let mut b = [0u8; 16];
    if !fill_from_urandom(&mut b) {
        fill_from_random_state(&mut b);
    }
    b[6] = (b[6] & 0x0f) | 0x40;
    b[8] = (b[8] & 0x3f) | 0x80;
    b
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

    #[test]
    fn random_v4_sets_version_and_variant_bits() {
        for _ in 0..64 {
            let b = random_v4();
            assert_eq!(b[6] >> 4, 4, "версия не 4");
            assert_eq!(b[8] >> 6, 0b10, "вариант не RFC 4122");
        }
    }

    #[test]
    fn two_random_identifiers_differ() {
        // Совпадение двух подряд случайных УИД — событие порядка 2^-122;
        // его появление означает сломанный источник, а не невезение.
        assert_ne!(random_v4(), random_v4());
    }
}
