//! Сжатие deflate (RFC 1951) — столько, сколько нужно ZIP-архиву XLSX.
//!
//! Внешних крейтов в этом рабочем пространстве нет, поэтому алгоритм здесь
//! свой. Он настоящий, а не «блоки без сжатия»: LZ77 со скользящим окном на
//! 32 КиБ плюс ФИКСИРОВАННЫЕ коды Хаффмана (тип блока 01).
//!
//! Почему фиксированные, а не динамические: разница в размере на разметке
//! XLSX — проценты, а кода на построение и запись деревьев потребовалось бы
//! втрое больше. Файл при этом остаётся обычным deflate, и его понимает
//! любой распаковщик, включая платформу 1С и Excel.

/// Побитовая запись, младшим битом вперёд, — так требует формат.
struct BitWriter {
    out: Vec<u8>,
    bit_buf: u32,
    bit_count: u32,
}

impl BitWriter {
    fn new() -> Self {
        BitWriter {
            out: Vec::new(),
            bit_buf: 0,
            bit_count: 0,
        }
    }

    /// Обычное поле: младший бит первым.
    fn bits(&mut self, value: u32, count: u32) {
        self.bit_buf |= value << self.bit_count;
        self.bit_count += count;
        while self.bit_count >= 8 {
            self.out.push((self.bit_buf & 0xFF) as u8);
            self.bit_buf >>= 8;
            self.bit_count -= 8;
        }
    }

    /// Код Хаффмана: он, наоборот, укладывается СТАРШИМ битом вперёд.
    fn code(&mut self, code: u32, count: u32) {
        for i in (0..count).rev() {
            self.bits((code >> i) & 1, 1);
        }
    }

    fn finish(mut self) -> Vec<u8> {
        if self.bit_count > 0 {
            self.out.push((self.bit_buf & 0xFF) as u8);
        }
        self.out
    }
}

/// Границы длин совпадений для кодов 257..285 и число добавочных бит.
const LENGTHS: [(u16, u16, u32); 29] = [
    (257, 3, 0),
    (258, 4, 0),
    (259, 5, 0),
    (260, 6, 0),
    (261, 7, 0),
    (262, 8, 0),
    (263, 9, 0),
    (264, 10, 0),
    (265, 11, 1),
    (266, 13, 1),
    (267, 15, 1),
    (268, 17, 1),
    (269, 19, 2),
    (270, 23, 2),
    (271, 27, 2),
    (272, 31, 2),
    (273, 35, 3),
    (274, 43, 3),
    (275, 51, 3),
    (276, 59, 3),
    (277, 67, 4),
    (278, 83, 4),
    (279, 99, 4),
    (280, 115, 4),
    (281, 131, 5),
    (282, 163, 5),
    (283, 195, 5),
    (284, 227, 5),
    (285, 258, 0),
];

/// То же для расстояний: код, начало диапазона, добавочные биты.
const DISTANCES: [(u16, u16, u32); 30] = [
    (0, 1, 0),
    (1, 2, 0),
    (2, 3, 0),
    (3, 4, 0),
    (4, 5, 1),
    (5, 7, 1),
    (6, 9, 2),
    (7, 13, 2),
    (8, 17, 3),
    (9, 25, 3),
    (10, 33, 4),
    (11, 49, 4),
    (12, 65, 5),
    (13, 97, 5),
    (14, 129, 6),
    (15, 193, 6),
    (16, 257, 7),
    (17, 385, 7),
    (18, 513, 8),
    (19, 769, 8),
    (20, 1025, 9),
    (21, 1537, 9),
    (22, 2049, 10),
    (23, 3073, 10),
    (24, 4097, 11),
    (25, 6145, 11),
    (26, 8193, 12),
    (27, 12289, 12),
    (28, 16385, 13),
    (29, 24577, 13),
];

/// Код литерала или длины в фиксированном дереве: `(код, сколько бит)`.
/// Раскладка задана самим RFC и потому записана числами, а не выведена.
fn literal_code(symbol: u16) -> (u32, u32) {
    match symbol {
        0..=143 => (0x30 + u32::from(symbol), 8),
        144..=255 => (0x190 + u32::from(symbol) - 144, 9),
        256..=279 => (u32::from(symbol) - 256, 7),
        _ => (0xC0 + u32::from(symbol) - 280, 8),
    }
}

const WINDOW: usize = 32 * 1024;
const MIN_MATCH: usize = 3;
const MAX_MATCH: usize = 258;
/// Сколько звеньев цепочки просматривать. Ограничение нужно: без него на
/// длинных однообразных участках поиск вырождается в квадратичный.
const MAX_CHAIN: usize = 64;

fn hash3(data: &[u8], pos: usize) -> usize {
    let a = u32::from(data[pos]);
    let b = u32::from(data[pos + 1]);
    let c = u32::from(data[pos + 2]);
    ((a << 10 ^ b << 5 ^ c).wrapping_mul(2_654_435_761)) as usize % (1 << 15)
}

/// Сжать данные одним блоком с фиксированными кодами.
pub fn deflate(data: &[u8]) -> Vec<u8> {
    let mut w = BitWriter::new();
    // Последний блок, тип 01 — фиксированные коды.
    w.bits(1, 1);
    w.bits(1, 2);

    if data.len() < MIN_MATCH {
        for &b in data {
            let (c, n) = literal_code(u16::from(b));
            w.code(c, n);
        }
        let (c, n) = literal_code(256);
        w.code(c, n);
        return w.finish();
    }

    // Хеш-цепочки: голова на каждый хеш и «предыдущая позиция» для каждой
    // позиции входа. Классическая схема zlib, только упрощённая.
    let mut head = vec![usize::MAX; 1 << 15];
    let mut prev = vec![usize::MAX; data.len()];

    let mut pos = 0;
    while pos < data.len() {
        let mut best_len = 0;
        let mut best_dist = 0;
        if pos + MIN_MATCH <= data.len() {
            let h = hash3(data, pos);
            let mut cand = head[h];
            let mut chain = 0;
            while cand != usize::MAX && chain < MAX_CHAIN {
                if pos - cand > WINDOW {
                    break;
                }
                let max = MAX_MATCH.min(data.len() - pos);
                let mut len = 0;
                while len < max && data[cand + len] == data[pos + len] {
                    len += 1;
                }
                if len > best_len {
                    best_len = len;
                    best_dist = pos - cand;
                    if len == max {
                        break;
                    }
                }
                cand = prev[cand];
                chain += 1;
            }
            prev[pos] = head[h];
            head[h] = pos;
        }

        if best_len >= MIN_MATCH {
            let &(code, base, extra) = LENGTHS
                .iter()
                .rev()
                .find(|&&(_, base, _)| usize::from(base) <= best_len)
                .expect("длина всегда попадает в таблицу");
            let (c, n) = literal_code(code);
            w.code(c, n);
            if extra > 0 {
                w.bits((best_len - usize::from(base)) as u32, extra);
            }
            let &(dist_code, dist_base, dist_extra) = DISTANCES
                .iter()
                .rev()
                .find(|&&(_, base, _)| usize::from(base) <= best_dist)
                .expect("расстояние всегда попадает в таблицу");
            w.code(u32::from(dist_code), 5);
            if dist_extra > 0 {
                w.bits((best_dist - usize::from(dist_base)) as u32, dist_extra);
            }
            // Позиции внутри совпадения тоже надо занести в цепочки, иначе
            // следующие совпадения будут находиться заметно хуже.
            for i in 1..best_len {
                let p = pos + i;
                if p + MIN_MATCH <= data.len() {
                    let h = hash3(data, p);
                    prev[p] = head[h];
                    head[h] = p;
                }
            }
            pos += best_len;
        } else {
            let (c, n) = literal_code(u16::from(data[pos]));
            w.code(c, n);
            pos += 1;
        }
    }

    let (c, n) = literal_code(256);
    w.code(c, n);
    w.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Проверка не «сжалось», а «распаковывается обратно»: свой распаковщик
    /// писать незачем, но обратимость обязана быть доказана. Здесь она
    /// доказывается на структуре потока — литералы и совпадения читаются
    /// тем же кодом, что и писались.
    fn inflate_fixed(bits: &[u8]) -> Vec<u8> {
        struct Reader<'a> {
            data: &'a [u8],
            pos: usize,
            bit: u32,
        }
        impl Reader<'_> {
            fn bits(&mut self, count: u32) -> u32 {
                let mut v = 0;
                for i in 0..count {
                    let byte = self.data[self.pos];
                    let b = (byte >> self.bit) & 1;
                    v |= u32::from(b) << i;
                    self.bit += 1;
                    if self.bit == 8 {
                        self.bit = 0;
                        self.pos += 1;
                    }
                }
                v
            }
            fn code(&mut self, count: u32) -> u32 {
                let mut v = 0;
                for _ in 0..count {
                    v = (v << 1) | self.bits(1);
                }
                v
            }
        }
        let mut r = Reader {
            data: bits,
            pos: 0,
            bit: 0,
        };
        assert_eq!(r.bits(1), 1, "должен быть последний блок");
        assert_eq!(r.bits(2), 1, "должны быть фиксированные коды");
        let mut out: Vec<u8> = Vec::new();
        loop {
            // Разбор фиксированного дерева: 7 бит, при нужде дочитываем.
            let mut code = r.code(7);
            let symbol = if code <= 0b0010111 {
                256 + code
            } else {
                code = (code << 1) | r.bits(1);
                if (0b00110000..=0b10111111).contains(&code) {
                    code - 0b00110000
                } else if (0b11000000..=0b11000111).contains(&code) {
                    280 + code - 0b11000000
                } else {
                    code = (code << 1) | r.bits(1);
                    144 + code - 0b110010000
                }
            };
            match symbol {
                256 => break,
                0..=255 => out.push(symbol as u8),
                _ => {
                    let &(_, base, extra) = LENGTHS
                        .iter()
                        .find(|&&(code_of, _, _)| u32::from(code_of) == symbol)
                        .expect("код длины из таблицы");
                    let len = usize::from(base) + r.bits(extra) as usize;
                    let dist_code = r.code(5);
                    let &(_, dist_base, dist_extra) = DISTANCES
                        .iter()
                        .find(|&&(code_of, _, _)| u32::from(code_of) == dist_code)
                        .expect("код расстояния из таблицы");
                    let dist = usize::from(dist_base) + r.bits(dist_extra) as usize;
                    for _ in 0..len {
                        let b = out[out.len() - dist];
                        out.push(b);
                    }
                }
            }
        }
        out
    }

    fn roundtrip(data: &[u8]) {
        let packed = deflate(data);
        assert_eq!(inflate_fixed(&packed), data, "распаковка не совпала");
    }

    #[test]
    fn empty_and_short_input() {
        roundtrip(b"");
        roundtrip(b"a");
        roundtrip(b"ab");
        roundtrip(b"abc");
    }

    #[test]
    fn repeats_compress_and_decompress() {
        roundtrip(&b"abcabcabcabcabcabc".repeat(20));
        roundtrip(&vec![b'x'; 5000]);
    }

    #[test]
    fn xml_markup_compresses_threefold_or_better() {
        let text = "<c r=\"A1\" s=\"1\" t=\"s\"><v>0</v></c>".repeat(500);
        let packed = deflate(text.as_bytes());
        assert_eq!(inflate_fixed(&packed), text.as_bytes());
        assert!(
            packed.len() * 3 < text.len(),
            "ожидалось сжатие хотя бы втрое, получилось {} из {}",
            packed.len(),
            text.len()
        );
    }

    #[test]
    fn unicode_and_random_bytes() {
        roundtrip("Организация; сумма 1 000,01 ₽".repeat(50).as_bytes());
        let bytes: Vec<u8> = (0..=255u8).cycle().take(3000).collect();
        roundtrip(&bytes);
    }
}
