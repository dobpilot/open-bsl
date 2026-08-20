//! Регистронезависимые ключи имён без аллокаций.
//!
//! Имена полей и колонок в BSL сравниваются без учёта регистра. Наивная
//! нормализация — `to_uppercase()` в `String` на каждый вызов — платит
//! аллокацией и таблицами Unicode; первая же посимвольная замена на
//! итераторах оказалась ещё дороже (+48% на генерации `table_compare`).
//! Поэтому здесь всё байтовое: единица свёртки — ASCII-байт либо
//! двухбайтовая кириллическая пара UTF-8, приведённая к верхнему регистру
//! прямо в байтах; символ вне этих алфавитов уводит операцию на медленный
//! путь через полноценно свёрнутую строку, поэтому результат совпадает со
//! сравнением через `String::to_uppercase` на любых входах.
//!
//! Числовой ключ (`folded_hash`) сам по себе имени НЕ идентифицирует:
//! хеш не инъективен, и совпадение чисел обязано перепроверяться
//! настоящим сравнением ([`folded_eq`]) — иначе коллизия молча склеила
//! бы два разных поля.

use std::hash::Hasher;

/// Хешер имён и интерн-кэшей — FxHash: восемь байтов за шаг, умножение с
/// поворотом. Ключи — имена полей и срезы исходного текста; их миллионы,
/// стойкий к затравке SipHash стандартной таблицы на них заметен в
/// профиле, а атак на затравку внутри одного процесса интерпретатора нет.
#[derive(Default)]
pub(crate) struct FxHasher(u64);

impl Hasher for FxHasher {
    fn write(&mut self, bytes: &[u8]) {
        const SEED: u64 = 0x51_7c_c1_b7_27_22_0a_95;
        let mut chunks = bytes.chunks_exact(8);
        for chunk in &mut chunks {
            let word = u64::from_le_bytes(chunk.try_into().expect("ровно восемь байтов"));
            self.0 = (self.0.rotate_left(5) ^ word).wrapping_mul(SEED);
        }
        let mut tail = 0u64;
        for &b in chunks.remainder().iter().rev() {
            tail = (tail << 8) | u64::from(b);
        }
        self.0 = (self.0.rotate_left(5) ^ tail).wrapping_mul(SEED);
    }

    fn finish(&self) -> u64 {
        self.0
    }
}

/// Следующая свёрнутая единица имени: ASCII-байт в верхнем регистре или
/// кириллическая пара UTF-8 одним `u16` (старший байт — первый).
/// `Some(Err(()))` — символ вне быстрых алфавитов, вызывающему пора на
/// медленный путь; `None` — байты кончились.
///
/// Принимаются ровно те двухбайтовые пары, чью верхнерегистровую форму
/// свёртка знает точно: `а`–`я`, `ё` и уже верхние `А`–`Я`, `Ё`. Прочие
/// буквы блоков `D0`/`D1` (украинские, сербские, `ѐ` и т.п.) уходят в
/// `Err`, а не проскакивают как есть, — иначе байтовый путь разошёлся бы
/// с `to_uppercase` медленного.
#[inline]
fn next_unit(bytes: &[u8], i: &mut usize) -> Option<Result<u16, ()>> {
    let first = *bytes.get(*i)?;
    if first < 0x80 {
        *i += 1;
        return Some(Ok(u16::from(first.to_ascii_uppercase())));
    }
    if (first == 0xD0 || first == 0xD1)
        && let Some(&second) = bytes.get(*i + 1)
    {
        let folded = match (first, second) {
            // а..п -> А..П
            (0xD0, 0xB0..=0xBF) => Some((0xD0, second - 0x20)),
            // р..я -> Р..Я
            (0xD1, 0x80..=0x8F) => Some((0xD0, second + 0x20)),
            // ё -> Ё
            (0xD1, 0x91) => Some((0xD0, 0x81)),
            // уже верхний регистр: А..Я и Ё
            (0xD0, 0x81 | 0x90..=0xAF) => Some((first, second)),
            _ => None,
        };
        if let Some((f, s)) = folded {
            *i += 2;
            return Some(Ok(u16::from_be_bytes([f, s])));
        }
    }
    Some(Err(()))
}

/// Полностью свёрнутая строка — медленный путь для символов вне ASCII и
/// кириллицы. Совпадает с `String::to_uppercase` вместе с его
/// многосимвольными разложениями.
fn folded_string(s: &str) -> String {
    s.to_uppercase()
}

/// Равны ли имена без учёта регистра. Единственный судья равенства.
/// Одинаковое написание закрывает memcmp (это горячий случай попадания
/// в корзину); дальше — байтовый проход до первого расхождения;
/// экзотика — через свёрнутые строки. Порядок проверок замерен на
/// генерации `table_compare`.
#[inline]
pub fn folded_eq(a: &str, b: &str) -> bool {
    if a == b {
        return true;
    }
    let (ab, bb) = (a.as_bytes(), b.as_bytes());
    let (mut i, mut j) = (0, 0);
    loop {
        match (next_unit(ab, &mut i), next_unit(bb, &mut j)) {
            (Some(Ok(x)), Some(Ok(y))) => {
                if x != y {
                    return false;
                }
            }
            (None, None) => return true,
            (Some(Err(())), _) | (_, Some(Err(()))) => return folded_string(a) == folded_string(b),
            _ => return false,
        }
    }
}

/// Свёрнутый байтовый образ имени: для быстрых алфавитов — их
/// верхнерегистровые байты UTF-8, для экзотики — байты полностью
/// свёрнутой строки; в обоих случаях это в точности байты
/// `to_uppercase`. Два имени равны без учёта регистра тогда и только
/// тогда, когда равны их образы, — на этом держатся кэши образов.
pub(crate) fn folded_bytes(s: &str) -> Vec<u8> {
    let bytes = s.as_bytes();
    let mut i = 0;
    let mut out = Vec::with_capacity(bytes.len());
    loop {
        match next_unit(bytes, &mut i) {
            Some(Ok(unit)) => {
                if unit <= 0xFF {
                    out.push(unit as u8);
                } else {
                    out.extend_from_slice(&unit.to_be_bytes());
                }
            }
            Some(Err(())) => return folded_string(s).into_bytes(),
            None => return out,
        }
    }
}

/// То же, но в буфер на стеке — для запросов в горячих поисках, где
/// аллокация на вызов свела бы выгоду на нет. `None` — имя длиннее
/// буфера либо с экзотикой: вызывающему нужен аллоцирующий путь.
#[inline]
pub(crate) fn folded_bytes_into<'b>(s: &str, buf: &'b mut [u8; 64]) -> Option<&'b [u8]> {
    let bytes = s.as_bytes();
    let mut i = 0;
    let mut used = 0;
    loop {
        match next_unit(bytes, &mut i) {
            Some(Ok(unit)) => {
                let width = if unit <= 0xFF { 1 } else { 2 };
                if used + width > buf.len() {
                    return None;
                }
                if width == 1 {
                    buf[used] = unit as u8;
                } else {
                    buf[used..used + 2].copy_from_slice(&unit.to_be_bytes());
                }
                used += width;
            }
            Some(Err(())) => return None,
            None => return Some(&buf[..used]),
        }
    }
}

/// Хешер для карт, чей ключ — уже готовый хеш ([`folded_hash`]):
/// пропускает число как есть, вместо того чтобы хешировать хеш.
#[derive(Default)]
pub(crate) struct PassHasher(u64);

impl Hasher for PassHasher {
    fn write(&mut self, _bytes: &[u8]) {
        unreachable!("PassHasher принимает только write_u64");
    }

    fn write_u64(&mut self, n: u64) {
        self.0 = n;
    }

    fn finish(&self) -> u64 {
        self.0
    }
}

/// Числовой ключ имени в свёрнутом регистре — для корзин индексов. Пары
/// с равным ключом обязаны перепроверяться через [`folded_eq`].
///
/// Быстрый путь хеширует единицы свёртки пачками по четыре; медленный —
/// байты полностью свёрнутой строки. Пути не обязаны совпадать между
/// собой: имя детерминированно попадает в один из них по своему
/// СВЁРНУТОМУ написанию, поэтому равные без учёта регистра имена всегда
/// получают один и тот же ключ.
#[inline]
pub(crate) fn folded_hash(s: &str) -> u64 {
    let bytes = s.as_bytes();
    let mut i = 0;
    let mut h = FxHasher::default();
    let mut acc = 0u64;
    let mut pending = 0u32;
    loop {
        match next_unit(bytes, &mut i) {
            Some(Ok(unit)) => {
                acc = (acc << 16) | u64::from(unit);
                pending += 1;
                if pending == 4 {
                    h.write_u64(acc);
                    acc = 0;
                    pending = 0;
                }
            }
            Some(Err(())) => {
                // Экзотика: хеш всей свёрнутой строки. Свёртка может
                // снова содержать не-ASCII — хешируются её байты, без
                // повторного захода в быстрый путь.
                let folded = folded_string(s);
                let mut h = FxHasher::default();
                h.write(folded.as_bytes());
                return h.finish();
            }
            None => {
                if pending > 0 {
                    h.write_u64(acc);
                }
                return h.finish();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folded_eq_matches_uppercase_comparison() {
        for (a, b, expected) in [
            ("Колонка", "кОлОнКа", true),
            ("Column", "COLUMN", true),
            ("ёлка", "ЁЛКА", true),
            ("Ёж", "ёж", true),
            ("größe", "GRÖSSE", true),
            ("Колонка", "Колонки", false),
            ("на", "но", false),
            ("ѐ", "Ѐ", true),
        ] {
            assert_eq!(folded_eq(a, b), expected, "{a} / {b}");
            assert_eq!(
                folded_eq(a, b),
                a.to_uppercase() == b.to_uppercase(),
                "расхождение со строковой свёрткой: {a} / {b}"
            );
        }
    }

    #[test]
    fn equal_names_in_any_case_share_the_hash() {
        for (a, b) in [
            ("Колонка", "кОлОнКа"),
            ("Column", "COLUMN"),
            ("ёлка", "ЁЛКА"),
            ("größe", "GRÖSSE"),
            ("ѐж", "Ѐж"),
        ] {
            assert!(folded_eq(a, b), "{a} == {b}");
            assert_eq!(folded_hash(a), folded_hash(b), "{a} / {b}");
        }
    }
}
