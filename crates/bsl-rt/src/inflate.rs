//! Распаковка deflate (RFC 1951) — обратная сторона `deflate.rs`.
//!
//! Писатель рядом порождает только блоки с фиксированными кодами Хаффмана,
//! но читать приходится всё, что положили в архив чужие инструменты,
//! поэтому здесь разобраны все три вида блоков: без сжатия, с фиксированными
//! и с динамическими кодами.
//!
//! Вход недоверенный — это байты чужого файла, — и ни одна его порча не
//! имеет права уронить процесс или подвесить его: каждый отказ
//! возвращается как [`RtError::Zip`] с текстом, называющим причину.
//! Завершение гарантировано устройством цикла: каждый шаг либо потребляет
//! хотя бы один бит конечного входа (а его конец — ошибка «поток обрезан»,
//! а не молчаливое дочитывание нулями), либо дописывает в выход, размер
//! которого ограничен `max_out`.
//!
//! Скорость здесь не критерий: декодер спускается по коду бит за битом, без
//! таблиц быстрого просмотра. Формат от этого не страдает — потоки в ZIP
//! измеряются десятками килобайт, а правильность на битом входе важнее.

use crate::RtError;

/// Предел длины кода Хаффмана в deflate (RFC 1951, 3.2.7).
const MAX_BITS: usize = 15;

/// Базовые длины совпадений для кодов 257..285 — зеркало таблицы `LENGTHS`
/// из `deflate.rs` (RFC 1951, 3.2.5), только проиндексированное кодом.
const LENGTH_BASE: [u16; 29] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131,
    163, 195, 227, 258,
];

/// Сколько добавочных бит читать после кода длины.
const LENGTH_EXTRA: [u32; 29] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
];

/// Базовые расстояния для кодов 0..29 — зеркало `DISTANCES` из `deflate.rs`.
const DIST_BASE: [u16; 30] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
    2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577,
];

/// Сколько добавочных бит читать после кода расстояния.
const DIST_EXTRA: [u32; 30] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13,
    13,
];

/// Порядок, в котором лежат длины кодов служебного алфавита в заголовке
/// динамического блока (RFC 1951, 3.2.7). Перестановка не случайна: редкие
/// длины сдвинуты в хвост, чтобы HCLEN обрезал именно их.
const CODE_LENGTH_ORDER: [usize; 19] = [
    16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
];

fn zip_err(what: &str) -> RtError {
    RtError::Zip(format!("deflate: {what}"))
}

fn truncated() -> RtError {
    zip_err("поток обрезан")
}

fn limit_exceeded(max_out: usize) -> RtError {
    RtError::Zip(format!(
        "deflate: распакованные данные превысили предел в {max_out} байт"
    ))
}

/// Побитовое чтение, младшим битом вперёд, — зеркало `BitWriter` из
/// `deflate.rs`. Коды Хаффмана лежат наоборот, старшим битом вперёд, и
/// собираются в [`Huffman::decode`] по одному биту.
struct BitReader<'a> {
    data: &'a [u8],
    /// Байт, из которого читается очередной бит.
    pos: usize,
    /// Номер этого бита внутри байта, 0..7.
    bit: u32,
}

impl<'a> BitReader<'a> {
    fn new(data: &'a [u8]) -> Self {
        BitReader {
            data,
            pos: 0,
            bit: 0,
        }
    }

    /// Один бит. Конец входа посреди чтения — ошибка, а не нули: иначе
    /// испорченный поток мог бы «доехать» до конца блока на выдуманных
    /// данных.
    fn bit(&mut self) -> Result<u32, RtError> {
        let byte = *self.data.get(self.pos).ok_or_else(truncated)?;
        let value = u32::from(byte >> self.bit) & 1;
        self.bit += 1;
        if self.bit == 8 {
            self.bit = 0;
            self.pos += 1;
        }
        Ok(value)
    }

    /// Обычное поле из `count` бит, младшим битом вперёд. Больше 16 бит
    /// формату не нужно, поэтому `u32` хватает с запасом.
    fn bits(&mut self, count: u32) -> Result<u32, RtError> {
        let mut value = 0;
        for i in 0..count {
            value |= self.bit()? << i;
        }
        Ok(value)
    }

    /// Выравнивание на границу байта — так начинается блок без сжатия.
    fn align(&mut self) {
        if self.bit != 0 {
            self.bit = 0;
            self.pos += 1;
        }
    }

    /// `count` байт подряд, начиная с текущей позиции; вызывать только
    /// после [`BitReader::align`], иначе недочитанные биты потерялись бы.
    fn bytes(&mut self, count: usize) -> Result<&'a [u8], RtError> {
        let end = self.pos.checked_add(count).ok_or_else(truncated)?;
        let slice = self.data.get(self.pos..end).ok_or_else(truncated)?;
        self.pos = end;
        Ok(slice)
    }
}

/// Канонический код Хаффмана, восстановленный по длинам кодов (RFC 1951,
/// 3.2.2). Хранятся не сами коды, а сколько их каждой длины и символы в
/// каноническом порядке — этого достаточно, чтобы спускаться по битам, и
/// не нужно ни дерева, ни таблицы на 32 КиБ.
struct Huffman {
    /// Сколько кодов каждой длины; `counts[0]` не используется.
    counts: [u16; MAX_BITS + 1],
    /// Символы, отсортированные по длине кода, а внутри длины — по
    /// значению символа.
    symbols: Vec<u16>,
}

impl Huffman {
    /// Построить код по длинам. Переполненный набор (сумма `2^-len` больше
    /// единицы) — ошибка сразу: такой набор не описывает никакого кода.
    /// Неполный набор допустим — так пишут блок с единственным расстоянием,
    /// — а вот попадание при чтении в неназначенный код ловится в
    /// [`Huffman::decode`].
    fn new(lengths: &[u8]) -> Result<Self, RtError> {
        let mut counts = [0u16; MAX_BITS + 1];
        for &len in lengths {
            let len = usize::from(len);
            if len > MAX_BITS {
                return Err(zip_err("длина кода Хаффмана больше 15 бит"));
            }
            counts[len] += 1;
        }

        // Классическая проверка по неравенству Крафта: на каждой длине
        // остаётся столько кодов, сколько ещё можно раздать. Длина 0
        // («символа в потоке нет») в счёт не идёт, отсюда `skip(1)`.
        let mut left: i32 = 1;
        for &count in counts.iter().skip(1) {
            left = (left << 1) - i32::from(count);
            if left < 0 {
                return Err(zip_err("переполненный набор кодов Хаффмана"));
            }
        }

        // Начало участка каждой длины в `symbols`.
        let mut offsets = [0usize; MAX_BITS + 2];
        for len in 1..=MAX_BITS {
            offsets[len + 1] = offsets[len] + usize::from(counts[len]);
        }
        let mut symbols = vec![0u16; offsets[MAX_BITS + 1]];
        for (symbol, &len) in lengths.iter().enumerate() {
            if len != 0 {
                let slot = usize::from(len);
                // Алфавиты deflate не длиннее 288 символов, так что номер
                // символа заведомо помещается в `u16`.
                symbols[offsets[slot]] = symbol as u16;
                offsets[slot] += 1;
            }
        }

        Ok(Huffman { counts, symbols })
    }

    /// Прочитать один символ: код набирается старшим битом вперёд, и на
    /// каждой длине проверяется, попал ли он в участок кодов этой длины.
    fn decode(&self, r: &mut BitReader<'_>) -> Result<u16, RtError> {
        // `code` — набранные биты, `first` — первый код текущей длины,
        // `index` — сколько символов уже пройдено на меньших длинах.
        let mut code: i32 = 0;
        let mut first: i32 = 0;
        let mut index: i32 = 0;
        for len in 1..=MAX_BITS {
            code |= r.bit()? as i32;
            let count = i32::from(self.counts[len]);
            if code - first < count {
                let at = (index + (code - first)) as usize;
                return self
                    .symbols
                    .get(at)
                    .copied()
                    .ok_or_else(|| zip_err("неверный код Хаффмана"));
            }
            index += count;
            first = (first + count) << 1;
            code <<= 1;
        }
        Err(zip_err("неверный код Хаффмана"))
    }
}

/// Фиксированный код литералов и длин (RFC 1951, 3.2.6). Раскладка задана
/// самим стандартом, поэтому длины записаны числами, а не выведены.
fn fixed_literal() -> Result<Huffman, RtError> {
    let mut lengths = [0u8; 288];
    for (symbol, len) in lengths.iter_mut().enumerate() {
        *len = match symbol {
            0..=143 => 8,
            144..=255 => 9,
            256..=279 => 7,
            _ => 8,
        };
    }
    Huffman::new(&lengths)
}

/// Фиксированный код расстояний: 32 кода по пять бит. Коды 30 и 31 в
/// потоке встречаться не должны, но в наборе они есть — иначе он был бы
/// неполным; отказ по ним даётся при чтении, где видно, что это расстояние.
fn fixed_distance() -> Result<Huffman, RtError> {
    Huffman::new(&[5u8; 32])
}

/// Заголовок динамического блока: служебный алфавит длин кодов, а по нему —
/// длины кодов литералов и расстояний (RFC 1951, 3.2.7).
fn dynamic_tables(r: &mut BitReader<'_>) -> Result<(Huffman, Huffman), RtError> {
    let hlit = r.bits(5)? as usize + 257;
    let hdist = r.bits(5)? as usize + 1;
    let hclen = r.bits(4)? as usize + 4;

    let mut code_lengths = [0u8; 19];
    for &slot in CODE_LENGTH_ORDER.iter().take(hclen) {
        code_lengths[slot] = r.bits(3)? as u8;
    }
    let code_length_code = Huffman::new(&code_lengths)?;

    // HLIT не больше 288, HDIST не больше 32 — оба поля по построению
    // упираются в размер алфавита, отдельной проверки границ не нужно.
    let total = hlit + hdist;
    let mut lengths = vec![0u8; total];
    let mut i = 0;
    while i < total {
        let symbol = code_length_code.decode(r)?;
        let (value, repeat) = match symbol {
            0..=15 => (symbol as u8, 1),
            // 16 — повторить ПРЕДЫДУЩУЮ длину; в начале таблицы повторять
            // нечего, и молча подставлять ноль нельзя: это была бы другая
            // таблица, а не та, что писал кодировщик.
            16 => {
                let prev = match i.checked_sub(1).and_then(|p| lengths.get(p)) {
                    Some(&prev) => prev,
                    None => return Err(zip_err("повтор длины кода в начале таблицы")),
                };
                (prev, 3 + r.bits(2)? as usize)
            }
            17 => (0, 3 + r.bits(3)? as usize),
            18 => (0, 11 + r.bits(7)? as usize),
            // Алфавит служебных длин — 19 символов, так что сюда попасть
            // нельзя; отказ вместо паники на случай будущих правок.
            _ => return Err(zip_err("неверный код в таблице длин кодов")),
        };
        if repeat > total - i {
            return Err(zip_err("повтор длин кодов выходит за границу таблицы"));
        }
        for _ in 0..repeat {
            lengths[i] = value;
            i += 1;
        }
    }

    let literal = Huffman::new(&lengths[..hlit])?;
    let distance = Huffman::new(&lengths[hlit..])?;
    Ok((literal, distance))
}

/// Блок без сжатия: выравнивание, LEN и его дополнение NLEN, затем байты
/// как есть.
fn stored_block(r: &mut BitReader<'_>, out: &mut Vec<u8>, max_out: usize) -> Result<(), RtError> {
    r.align();
    let header = r.bytes(4)?;
    let len = u16::from(header[0]) | (u16::from(header[1]) << 8);
    let nlen = u16::from(header[2]) | (u16::from(header[3]) << 8);
    if len != !nlen {
        return Err(zip_err(
            "в блоке без сжатия LEN не совпал с дополнением NLEN",
        ));
    }
    let len = usize::from(len);
    if len > max_out.saturating_sub(out.len()) {
        return Err(limit_exceeded(max_out));
    }
    out.extend_from_slice(r.bytes(len)?);
    Ok(())
}

/// Блок с кодами Хаффмана — фиксированными или динамическими, разница
/// только в том, откуда взялись таблицы.
fn coded_block(
    r: &mut BitReader<'_>,
    out: &mut Vec<u8>,
    max_out: usize,
    literal: &Huffman,
    distance: &Huffman,
) -> Result<(), RtError> {
    loop {
        let symbol = literal.decode(r)?;
        match symbol {
            0..=255 => {
                if out.len() >= max_out {
                    return Err(limit_exceeded(max_out));
                }
                out.push(symbol as u8);
            }
            256 => return Ok(()),
            257..=285 => {
                let idx = usize::from(symbol - 257);
                let len = usize::from(LENGTH_BASE[idx]) + r.bits(LENGTH_EXTRA[idx])? as usize;

                let dist_symbol = distance.decode(r)?;
                let dist_idx = usize::from(dist_symbol);
                if dist_idx >= DIST_BASE.len() {
                    // Коды 30 и 31 зарезервированы и в потоке недопустимы.
                    return Err(zip_err("неверный код расстояния"));
                }
                let dist =
                    usize::from(DIST_BASE[dist_idx]) + r.bits(DIST_EXTRA[dist_idx])? as usize;
                if dist > out.len() {
                    return Err(zip_err(
                        "расстояние ссылается за начало распакованных данных",
                    ));
                }
                if len > max_out.saturating_sub(out.len()) {
                    return Err(limit_exceeded(max_out));
                }

                // Окно — сам выходной буфер, отдельного кольцевого не нужно.
                // Копирование обязано идти байт за байтом: источник и
                // приёмник законно перекрываются, и на этом перекрытии
                // держится размножение (`расстояние = 1`, длина 258 — это
                // 258 одинаковых байт), поэтому ни `copy_within`, ни
                // `extend_from_within` здесь не подходят.
                let start = out.len() - dist;
                for i in 0..len {
                    let byte = out[start + i];
                    out.push(byte);
                }
            }
            // 286 и 287 попадают в фиксированный код, но значения им не
            // назначены — корректный поток их не содержит.
            _ => return Err(zip_err("неверный код длины совпадения")),
        }
    }
}

/// Распаковать поток deflate (RFC 1951) целиком.
///
/// `data` — сырой поток без обёрток zlib и gzip, ровно то, что лежит в
/// записи ZIP со способом хранения 8. Байты после последнего блока
/// игнорируются: за потоком в архиве обычно идёт дескриптор данных или
/// следующая запись.
///
/// `max_out` — жёсткий предел размера распакованных данных. Он
/// обязателен: без него испорченный или злонамеренный поток раздувает
/// буфер до исчерпания памяти, а размер архивы объявляют заранее, так что
/// вызывающему есть что подставить.
///
/// # Errors
///
/// [`RtError::Zip`] на любой порче входа: поток кончился посреди чтения,
/// зарезервированный тип блока, LEN без дополнения NLEN, переполненный или
/// не назначенный код Хаффмана, расстояние за пределами уже распакованных
/// данных, превышение `max_out`. Текст ошибки называет причину.
// Единственный будущий потребитель — читатель ZIP, он живёт в этом же
// крейте, поэтому наружу функция не выносится; пока читателя нет, вне
// тестовой сборки она законно «не нужна».
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn inflate(data: &[u8], max_out: usize) -> Result<Vec<u8>, RtError> {
    let mut r = BitReader::new(data);
    let mut out = Vec::new();
    loop {
        let last = r.bit()? == 1;
        match r.bits(2)? {
            0 => stored_block(&mut r, &mut out, max_out)?,
            1 => {
                let (literal, distance) = (fixed_literal()?, fixed_distance()?);
                coded_block(&mut r, &mut out, max_out, &literal, &distance)?;
            }
            2 => {
                let (literal, distance) = dynamic_tables(&mut r)?;
                coded_block(&mut r, &mut out, max_out, &literal, &distance)?;
            }
            // Поле в два бита даёт только 0..3, так что здесь ровно
            // BTYPE = 11 — значение, зарезервированное стандартом.
            _ => return Err(zip_err("неверный тип блока")),
        }
        if last {
            return Ok(out);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::deflate::deflate;

    /// Эталонный поток из блока БЕЗ СЖАТИЯ. Происхождение (CPython 3.14,
    /// zlib 1.3.1):
    ///
    /// ```python
    /// import zlib
    /// text = "Организация «Ромашка», ИНН 7701234567, отчёт за 2026 год.\n".encode()
    /// c = zlib.compressobj(0, zlib.DEFLATED, -15)
    /// print(list(c.compress(text) + c.flush()))
    /// ```
    const REF_STORED: [u8; 96] = [
        0x01, 0x5B, 0x00, 0xA4, 0xFF, 0xD0, 0x9E, 0xD1, 0x80, 0xD0, 0xB3, 0xD0, 0xB0, 0xD0, 0xBD,
        0xD0, 0xB8, 0xD0, 0xB7, 0xD0, 0xB0, 0xD1, 0x86, 0xD0, 0xB8, 0xD1, 0x8F, 0x20, 0xC2, 0xAB,
        0xD0, 0xA0, 0xD0, 0xBE, 0xD0, 0xBC, 0xD0, 0xB0, 0xD1, 0x88, 0xD0, 0xBA, 0xD0, 0xB0, 0xC2,
        0xBB, 0x2C, 0x20, 0xD0, 0x98, 0xD0, 0x9D, 0xD0, 0x9D, 0x20, 0x37, 0x37, 0x30, 0x31, 0x32,
        0x33, 0x34, 0x35, 0x36, 0x37, 0x2C, 0x20, 0xD0, 0xBE, 0xD1, 0x82, 0xD1, 0x87, 0xD1, 0x91,
        0xD1, 0x82, 0x20, 0xD0, 0xB7, 0xD0, 0xB0, 0x20, 0x32, 0x30, 0x32, 0x36, 0x20, 0xD0, 0xB3,
        0xD0, 0xBE, 0xD0, 0xB4, 0x2E, 0x0A,
    ];

    /// Текст, из которого собран [`REF_STORED`].
    const STORED_TEXT: &str = "Организация «Ромашка», ИНН 7701234567, отчёт за 2026 год.\n";

    /// Эталонный поток из блока с ДИНАМИЧЕСКИМИ кодами. Происхождение
    /// (CPython 3.14, zlib 1.3.1); текст тот же, что собирает
    /// [`dynamic_reference_text`]:
    ///
    /// ```python
    /// import zlib
    /// text = ("Номенклатура;Количество;Цена\n"
    ///         + "".join(f"Товар-{i%7};{i};{i*13}\n" for i in range(60))).encode()
    /// c = zlib.compressobj(9, zlib.DEFLATED, -15)
    /// print(list(c.compress(text) + c.flush()))
    /// ```
    const REF_DYNAMIC: [u8; 360] = [
        0x5D, 0xD3, 0x3F, 0x6A, 0x9E, 0x31, 0x0C, 0x06, 0xF0, 0x3D, 0x77, 0x29, 0x58, 0xFF, 0x65,
        0x9E, 0xCB, 0xA5, 0xE9, 0xD0, 0x3B, 0x14, 0xBA, 0x74, 0xE8, 0xFC, 0x11, 0x08, 0x94, 0xA6,
        0xCD, 0x19, 0xFC, 0xDE, 0xA8, 0xA2, 0xD3, 0x2B, 0xA1, 0x4D, 0xB6, 0xD0, 0x0F, 0x5B, 0x3A,
        0xDF, 0xCF, 0xC7, 0xF9, 0x73, 0xDE, 0xCE, 0xDF, 0xF3, 0xFB, 0xBC, 0x9F, 0xC7, 0xF5, 0x72,
        0x7D, 0xB9, 0x9E, 0xCF, 0x03, 0xE7, 0x5B, 0x1D, 0xBC, 0x9F, 0x5F, 0xD7, 0xD7, 0xF3, 0x76,
        0x7D, 0xBE, 0x5E, 0xCE, 0xEB, 0xF9, 0xC0, 0xF9, 0xF9, 0xFF, 0xE6, 0xE3, 0xE9, 0xFC, 0xA8,
        0xD3, 0xD7, 0xBA, 0xFE, 0xFC, 0x69, 0xA1, 0xE2, 0x9E, 0x20, 0x54, 0xC8, 0x3D, 0xC3, 0xA8,
        0xF0, 0x7B, 0x46, 0x50, 0xB1, 0xEF, 0x19, 0x85, 0xC2, 0xF8, 0x9E, 0x31, 0x18, 0xDC, 0xEE,
        0x19, 0x87, 0x23, 0xB2, 0x37, 0x0F, 0x6C, 0xEA, 0xDD, 0x13, 0xB4, 0xB4, 0xB7, 0xDF, 0x20,
        0x8A, 0xDE, 0x9F, 0x56, 0x21, 0x57, 0x17, 0x50, 0xC1, 0x55, 0xBA, 0x81, 0x18, 0x64, 0xDE,
        0x15, 0x54, 0xE5, 0xBE, 0xBB, 0x83, 0xAA, 0x3C, 0x79, 0xBC, 0x43, 0x95, 0x6F, 0xEB, 0x14,
        0x72, 0xF0, 0xCA, 0x61, 0x09, 0x30, 0xD3, 0xB0, 0x24, 0x58, 0x74, 0x58, 0x36, 0x58, 0xA3,
        0x5B, 0x78, 0xD5, 0xD3, 0xAE, 0x6E, 0x61, 0x02, 0x87, 0x74, 0x0B, 0xD7, 0x17, 0xA4, 0x8F,
        0x5F, 0x11, 0xF0, 0xDE, 0xDD, 0xC2, 0x0A, 0x21, 0xEE, 0x16, 0x36, 0x08, 0x5B, 0xB7, 0xB0,
        0x43, 0x24, 0x87, 0x25, 0x20, 0x46, 0xC3, 0x92, 0x10, 0xD7, 0x61, 0xD9, 0x90, 0x88, 0x6E,
        0x91, 0x55, 0x03, 0xB1, 0xC6, 0x8C, 0x10, 0x74, 0x49, 0xB7, 0x08, 0x43, 0xC9, 0xBB, 0x45,
        0x04, 0xCA, 0xBB, 0x5B, 0xA4, 0x06, 0x4A, 0xB9, 0x5B, 0xC4, 0xA0, 0x66, 0xDD, 0x22, 0x0E,
        0xF5, 0x1C, 0x96, 0x80, 0x26, 0x0D, 0x4B, 0x42, 0xB7, 0x0E, 0xCB, 0x86, 0xAD, 0xE8, 0x16,
        0x5D, 0x35, 0xC6, 0xAB, 0x5B, 0x94, 0x60, 0x22, 0xDD, 0xA2, 0x0C, 0x53, 0xEF, 0x16, 0x15,
        0x98, 0xED, 0x6E, 0xD1, 0x5A, 0x8B, 0xE0, 0x6E, 0xD1, 0x5A, 0x8D, 0xB4, 0xB1, 0x3D, 0x0E,
        0xDB, 0x39, 0x2C, 0x01, 0x27, 0x1A, 0x96, 0x84, 0xB3, 0x0E, 0xCB, 0x86, 0x4B, 0x74, 0x8B,
        0xAD, 0x5A, 0xBE, 0xD5, 0x2D, 0x46, 0x70, 0x97, 0x6E, 0x31, 0x86, 0x87, 0x77, 0x8B, 0x09,
        0x3C, 0xF7, 0xD8, 0x65, 0x45, 0x2C, 0xEE, 0x16, 0x33, 0x04, 0x59, 0xB7, 0x58, 0xAD, 0x38,
        0xE7, 0xB0, 0x04, 0x42, 0x69, 0x58, 0x12, 0x61, 0x3A, 0x2C, 0x1B, 0xE1, 0xF1, 0xF4, 0x0F,
    ];

    /// Текст, из которого собран [`REF_DYNAMIC`], — тот же генератор, что в
    /// комментарии рядом с эталоном.
    fn dynamic_reference_text() -> Vec<u8> {
        let mut text = String::from("Номенклатура;Количество;Цена\n");
        for i in 0..60 {
            text.push_str(&format!("Товар-{};{};{}\n", i % 7, i, i * 13));
        }
        text.into_bytes()
    }

    /// Эталонный МНОГОБЛОЧНЫЙ поток. Оба эталона выше — из одного блока, и
    /// наш кодировщик рядом тоже пишет ровно один, так что цикл по блокам
    /// без такого эталона не проверялся бы ничем. `Z_SYNC_FLUSH` посреди
    /// сжатия разрывает поток: получаются три блока — с фиксированными
    /// кодами, пустой блок без сжатия (маркер синхронизации `00 00 FF FF`)
    /// и финальный. Происхождение (CPython 3.14, zlib 1.3.1); части те же,
    /// что собирает [`multi_reference_text`]:
    ///
    /// ```python
    /// import zlib
    /// part1 = "Организация «Ромашка», ИНН 7701234567.\n".encode() * 3
    /// part2 = b"Invoice INV-2026-0814: total 1234.56 EUR, due 2026-09-01.\n" * 2
    /// c = zlib.compressobj(9, zlib.DEFLATED, -15)
    /// print(list(c.compress(part1) + c.flush(zlib.Z_SYNC_FLUSH)
    ///            + c.compress(part2) + c.flush()))
    /// ```
    const REF_MULTI: [u8; 130] = [
        0xBA, 0x30, 0xEF, 0x62, 0xC3, 0x85, 0xCD, 0x17, 0x36, 0x5C, 0xD8, 0x7B, 0x61, 0xC7, 0x85,
        0xED, 0x17, 0x36, 0x5C, 0x6C, 0xBB, 0xB0, 0xE3, 0x62, 0xBF, 0xC2, 0xA1, 0xD5, 0x17, 0x16,
        0x5C, 0xD8, 0x77, 0x61, 0x0F, 0x50, 0xA0, 0xE3, 0xC2, 0xAE, 0x0B, 0x1B, 0x0E, 0xED, 0xD6,
        0x51, 0xB8, 0x30, 0xE3, 0xC2, 0xDC, 0x0B, 0x73, 0x15, 0xCC, 0xCD, 0x0D, 0x0C, 0x8D, 0x8C,
        0x4D, 0x4C, 0xCD, 0xCC, 0xF5, 0xB8, 0x2E, 0x0C, 0xA0, 0x6E, 0x00, 0x00, 0x00, 0x00, 0xFF,
        0xFF, 0xF3, 0xCC, 0x2B, 0xCB, 0xCF, 0x4C, 0x4E, 0x55, 0xF0, 0xF4, 0x0B, 0xD3, 0x35, 0x32,
        0x30, 0x32, 0xD3, 0x35, 0xB0, 0x30, 0x34, 0xB1, 0x52, 0x28, 0xC9, 0x2F, 0x49, 0xCC, 0x51,
        0x00, 0xA9, 0xD2, 0x33, 0x35, 0x53, 0x70, 0x0D, 0x0D, 0xD2, 0x51, 0x48, 0x29, 0x4D, 0x55,
        0x80, 0xA8, 0xB0, 0xD4, 0x35, 0x30, 0xA4, 0x44, 0x27, 0x00,
    ];

    /// Текст, из которого собран [`REF_MULTI`], — те же две части, что в
    /// комментарии рядом с эталоном. Части намеренно непохожи: русская
    /// фраза до разрыва и латинская после, чтобы результат «только первого
    /// блока» не мог случайно совпасть с целым.
    fn multi_reference_text() -> Vec<u8> {
        let mut text = "Организация «Ромашка», ИНН 7701234567.\n".repeat(3);
        text.push_str(&"Invoice INV-2026-0814: total 1234.56 EUR, due 2026-09-01.\n".repeat(2));
        text.into_bytes()
    }

    /// Детерминированный xorshift: внешних крейтов в рабочем пространстве
    /// нет, а «несжимаемый» вход обязан воспроизводиться от прогона к
    /// прогону, иначе падение теста нечем повторить.
    fn xorshift_bytes(n: usize, seed: u64) -> Vec<u8> {
        let mut state = seed;
        (0..n)
            .map(|_| {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                (state & 0xFF) as u8
            })
            .collect()
    }

    /// Побитовая сборка потока для рукописных проб — младшим битом вперёд,
    /// как в формате; коды Хаффмана укладываются старшим вперёд.
    #[derive(Default)]
    struct Pen {
        out: Vec<u8>,
        acc: u32,
        count: u32,
    }

    impl Pen {
        fn bits(&mut self, value: u32, count: u32) {
            self.acc |= value << self.count;
            self.count += count;
            while self.count >= 8 {
                self.out.push((self.acc & 0xFF) as u8);
                self.acc >>= 8;
                self.count -= 8;
            }
        }

        fn code(&mut self, code: u32, count: u32) {
            for i in (0..count).rev() {
                self.bits((code >> i) & 1, 1);
            }
        }

        fn finish(mut self) -> Vec<u8> {
            if self.count > 0 {
                self.out.push((self.acc & 0xFF) as u8);
            }
            self.out
        }
    }

    fn roundtrip(data: &[u8]) {
        let packed = deflate(data);
        let back = inflate(&packed, data.len() + 1).expect("наш же поток обязан распаковаться");
        assert_eq!(back, data, "распаковка не совпала с исходными данными");
    }

    #[test]
    fn our_own_fixed_code_blocks_come_back_unchanged() {
        roundtrip(b"");
        roundtrip(b"a");
        roundtrip(b"ab");
        roundtrip("Организация «Ромашка» — отчёт за 2026 год".as_bytes());
        roundtrip(&b"abcabcabcabcabcabc".repeat(20));
        roundtrip(&vec![b'x'; 5000]);
        roundtrip("Номенклатура; количество; цена\n".repeat(200).as_bytes());
        roundtrip(&(0..=255u8).cycle().take(3000).collect::<Vec<u8>>());
    }

    #[test]
    fn incompressible_noise_comes_back_unchanged() {
        for &size in &[1usize, 2, 3, 17, 1000, 40_000] {
            roundtrip(&xorshift_bytes(size, 0x2026_0814));
        }
    }

    /// Размножение байта — единственный случай, где источник копирования
    /// заходит на ещё не записанный участок выхода. Поток собран руками,
    /// чтобы расстояние 1 было в нём заведомо, а не по усмотрению нашего
    /// же кодировщика.
    #[test]
    fn a_match_with_distance_one_repeats_the_byte() {
        let mut pen = Pen::default();
        pen.bits(1, 1); // последний блок
        pen.bits(1, 2); // фиксированные коды
        pen.code(0x30 + 0x41, 8); // литерал «A»
        pen.code(1, 7); // символ 257 — длина 3
        pen.code(0, 5); // расстояние 1
        pen.code(0xC5, 8); // символ 285 — длина 258
        pen.code(0, 5); // снова расстояние 1
        pen.code(0, 7); // символ 256 — конец блока
        let out = inflate(&pen.finish(), 1024).expect("поток обязан распаковаться");
        assert_eq!(out, vec![b'A'; 1 + 3 + 258]);

        // И то же самое через наш кодировщик, целиком.
        let packed = deflate(&vec![b'z'; 300]);
        let back = inflate(&packed, 300).expect("поток обязан распаковаться");
        assert_eq!(back, vec![b'z'; 300]);
    }

    #[test]
    fn the_reference_stored_block_unpacks() {
        assert_eq!(REF_STORED[0] & 1, 1, "эталон — один последний блок");
        assert_eq!((REF_STORED[0] >> 1) & 3, 0, "и он должен быть без сжатия");
        let out = inflate(&REF_STORED, 1024).expect("эталон zlib обязан распаковаться");
        assert_eq!(String::from_utf8(out).expect("UTF-8"), STORED_TEXT);
    }

    #[test]
    fn the_reference_dynamic_block_unpacks() {
        assert_eq!(REF_DYNAMIC[0] & 1, 1, "эталон — один последний блок");
        assert_eq!(
            (REF_DYNAMIC[0] >> 1) & 3,
            2,
            "и он должен быть с динамическими кодами"
        );
        let out = inflate(&REF_DYNAMIC, 4096).expect("эталон zlib обязан распаковаться");
        assert_eq!(out, dynamic_reference_text());
    }

    /// Поток из нескольких блоков читается до конца, а не до первого
    /// блока: остановка по BFINAL — единственное, что отличает «поток
    /// кончился» от «кончился блок», и настоящие архивы почти всегда
    /// многоблочные (zlib начинает новый блок примерно каждые 16 К
    /// символов).
    #[test]
    fn the_reference_multi_block_stream_unpacks_every_block() {
        assert_eq!(
            REF_MULTI[0] & 1,
            0,
            "первый блок эталона не последний, значит блоков больше одного"
        );
        assert_eq!(
            (REF_MULTI[0] >> 1) & 3,
            1,
            "и он должен быть с фиксированными кодами"
        );
        let out = inflate(&REF_MULTI, 4096).expect("эталон zlib обязан распаковаться");
        let expected = multi_reference_text();
        assert_eq!(
            out.len(),
            expected.len(),
            "распакована часть потока, а не весь"
        );
        assert_eq!(out, expected);
    }

    /// Байты после последнего блока — не ошибка: в архиве за потоком идёт
    /// дескриптор данных или следующая запись.
    #[test]
    fn trailing_bytes_after_the_last_block_are_ignored() {
        let mut stream = REF_STORED.to_vec();
        stream.extend_from_slice(b"PK\x03\x04");
        let out = inflate(&stream, 1024).expect("хвост не должен мешать");
        assert_eq!(String::from_utf8(out).expect("UTF-8"), STORED_TEXT);
    }

    #[test]
    fn every_truncation_of_a_reference_stream_is_an_error_or_the_whole_result() {
        for stream in [&REF_STORED[..], &REF_DYNAMIC[..]] {
            let full = inflate(stream, 65_536).expect("целый эталон распаковывается");
            for cut in 0..stream.len() {
                match inflate(&stream[..cut], 65_536) {
                    Ok(out) => assert_eq!(out, full, "обрезка {cut} дала неполный результат"),
                    Err(e) => assert!(!e.to_string().is_empty(), "ошибка без текста"),
                }
            }
        }
    }

    /// Порча одним битом в любом месте: интересна не конкретная ошибка, а
    /// то, что распаковщик всегда возвращает управление — ни паники, ни
    /// зависания, ни выхода за `max_out`.
    #[test]
    fn a_single_bit_flip_anywhere_never_panics() {
        for byte in 0..REF_DYNAMIC.len() {
            for bit in 0..8 {
                let mut stream = REF_DYNAMIC;
                stream[byte] ^= 1 << bit;
                match inflate(&stream, 65_536) {
                    Ok(out) => assert!(out.len() <= 65_536, "предел выхода нарушен"),
                    Err(e) => assert!(!e.to_string().is_empty(), "ошибка без текста"),
                }
            }
        }
    }

    #[test]
    fn a_stored_block_with_a_broken_nlen_is_rejected() {
        let mut stream = REF_STORED;
        stream[3] ^= 0xFF;
        let e = inflate(&stream, 1024).expect_err("NLEN не дополняет LEN");
        assert!(e.to_string().contains("NLEN"), "непонятный текст: {e}");
    }

    #[test]
    fn the_reserved_block_type_is_rejected() {
        // BFINAL = 1, BTYPE = 11.
        let e = inflate(&[0b0000_0111], 1024).expect_err("тип блока 11 зарезервирован");
        assert!(e.to_string().contains("тип блока"), "непонятный текст: {e}");
    }

    #[test]
    fn a_match_reaching_before_the_start_of_the_output_is_rejected() {
        let mut pen = Pen::default();
        pen.bits(1, 1); // последний блок
        pen.bits(1, 2); // фиксированные коды
        pen.code(0b0000001, 7); // символ 257 — совпадение длиной 3
        pen.code(0, 5); // код расстояния 0 — расстояние 1, а выход пуст
        let e = inflate(&pen.finish(), 1024).expect_err("копировать нечего");
        assert!(
            e.to_string().contains("расстояние"),
            "непонятный текст: {e}"
        );
    }

    /// Коды расстояний 30 и 31 в фиксированном наборе есть (иначе он был бы
    /// неполным), но значений им не назначено. Проверка границы здесь не
    /// косметическая: без неё `DIST_BASE[30]` на массиве из тридцати
    /// элементов — паника, то есть ровно то, чего модуль обещает не
    /// допускать ни на какой порче входа.
    #[test]
    fn a_reserved_distance_code_is_rejected() {
        let mut pen = Pen::default();
        pen.bits(1, 1); // последний блок
        pen.bits(1, 2); // фиксированные коды
        pen.code(0x30 + 0x41, 8); // литерал «A» — чтобы копировать было откуда
        pen.code(1, 7); // символ 257 — длина 3
        pen.code(30, 5); // код расстояния 30 — зарезервирован
        let e = inflate(&pen.finish(), 1024).expect_err("код расстояния 30 недопустим");
        assert!(e.to_string().contains("расстояни"), "непонятный текст: {e}");
    }

    /// Симметрично расстояниям: символы 286 и 287 в фиксированный код
    /// литералов попадают, но значения им не назначены.
    #[test]
    fn a_length_symbol_with_no_assigned_value_is_rejected() {
        let mut pen = Pen::default();
        pen.bits(1, 1); // последний блок
        pen.bits(1, 2); // фиксированные коды
        pen.code(0x30 + 0x41, 8); // литерал «A»
        pen.code(0xC6, 8); // символ 286 — зарезервирован
        let e = inflate(&pen.finish(), 1024).expect_err("символ 286 недопустим");
        assert!(
            e.to_string().contains("длины совпадения"),
            "непонятный текст: {e}"
        );
    }

    #[test]
    fn a_repeat_of_the_previous_code_length_at_the_start_is_rejected() {
        let mut pen = Pen::default();
        pen.bits(1, 1); // последний блок
        pen.bits(2, 2); // динамические коды
        pen.bits(0, 5); // HLIT  = 257
        pen.bits(0, 5); // HDIST = 1
        pen.bits(0, 4); // HCLEN = 4: длины кодов 16, 17, 18, 0
        pen.bits(1, 3); // код 16 — один бит
        pen.bits(1, 3); // код 17 — один бит
        pen.bits(0, 3);
        pen.bits(0, 3);
        pen.code(0, 1); // первый же символ таблицы — 16, повторять нечего
        let e = inflate(&pen.finish(), 1024).expect_err("повторять нечего");
        assert!(e.to_string().contains("повтор"), "непонятный текст: {e}");
    }

    #[test]
    fn an_over_subscribed_huffman_set_is_rejected() {
        let mut pen = Pen::default();
        pen.bits(1, 1); // последний блок
        pen.bits(2, 2); // динамические коды
        pen.bits(0, 5); // HLIT  = 257
        pen.bits(0, 5); // HDIST = 1
        pen.bits(0, 4); // HCLEN = 4
        pen.bits(1, 3); // три кода длиной в один бит — на один больше,
        pen.bits(1, 3); // чем бывает: набор переполнен
        pen.bits(1, 3);
        pen.bits(0, 3);
        let e = inflate(&pen.finish(), 1024).expect_err("набор кодов переполнен");
        assert!(
            e.to_string().contains("переполненный"),
            "непонятный текст: {e}"
        );
    }

    #[test]
    fn the_output_limit_is_enforced() {
        let short = inflate(&REF_DYNAMIC, 100).expect_err("предел меньше распакованных данных");
        assert!(
            short.to_string().contains("предел"),
            "непонятный текст: {short}"
        );
        let stored = inflate(&REF_STORED, 10).expect_err("предел меньше блока без сжатия");
        assert!(
            stored.to_string().contains("предел"),
            "непонятный текст: {stored}"
        );

        // Предел ровно по размеру данных — уже не ошибка.
        let exact = inflate(&REF_STORED, STORED_TEXT.len()).expect("ровный предел допустим");
        assert_eq!(exact.len(), STORED_TEXT.len());
        let exact =
            inflate(&REF_DYNAMIC, dynamic_reference_text().len()).expect("ровный предел допустим");
        assert_eq!(exact, dynamic_reference_text());
    }

    /// Предел на ЛИТЕРАЛЬНОМ пути, в отдельности. В эталонах есть и
    /// литералы, и совпадения, поэтому проба по ним не различает две
    /// проверки: любая из оставшихся поймала бы перелив. Здесь совпадений в
    /// потоке нет вовсе, так что отказать может только проверка при записи
    /// литерала.
    #[test]
    fn the_output_limit_is_enforced_on_a_stream_of_literals_alone() {
        let letters = |pen: &mut Pen| {
            pen.bits(1, 1); // последний блок
            pen.bits(1, 2); // фиксированные коды
            for letter in [0x41u32, 0x42, 0x43, 0x44] {
                pen.code(0x30 + letter, 8); // литералы «A», «B», «C», «D»
            }
            pen.code(0, 7); // символ 256 — конец блока
        };

        let mut pen = Pen::default();
        letters(&mut pen);
        let stream = pen.finish();

        let e = inflate(&stream, 3).expect_err("четыре литерала в три байта не помещаются");
        assert!(e.to_string().contains("предел"), "непонятный текст: {e}");

        // Граница ровная: предел по числу литералов ошибкой уже не является.
        let exact = inflate(&stream, 4).expect("ровный предел допустим");
        assert_eq!(exact, b"ABCD");
    }

    /// Предел на пути СОВПАДЕНИЙ, в отдельности. Литерал в предел
    /// укладывается, и оборвать копирование 258 байт может только проверка
    /// перед совпадением — а именно она и есть защита от архивной бомбы:
    /// 258 байт выхода стоят в потоке около полусотни бит.
    #[test]
    fn the_output_limit_is_enforced_in_the_middle_of_a_match() {
        let mut pen = Pen::default();
        pen.bits(1, 1); // последний блок
        pen.bits(1, 2); // фиксированные коды
        pen.code(0x30 + 0x41, 8); // литерал «A» — в предел укладывается
        pen.code(0xC5, 8); // символ 285 — длина 258
        pen.code(0, 5); // расстояние 1 — размножение байта
        pen.code(0, 7); // символ 256 — конец блока
        let stream = pen.finish();

        let e = inflate(&stream, 50).expect_err("совпадение выходит за предел");
        assert!(e.to_string().contains("предел"), "непонятный текст: {e}");

        // Тот же поток целиком укладывается ровно в 259 байт.
        let exact = inflate(&stream, 259).expect("ровный предел допустим");
        assert_eq!(exact, vec![b'A'; 259]);
    }
}
