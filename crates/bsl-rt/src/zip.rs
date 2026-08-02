//! Минимальный писатель ZIP — ровно столько, сколько нужно для XLSX.
//!
//! Внешних крейтов в этом рабочем пространстве нет и не предвидится, поэтому
//! архив собирается здесь. Данные сжимаются методом 8 (deflate) — тем же,
//! что использует платформа 1С в своих XLSX. Если сжатие не дало выигрыша,
//! запись уходит методом 0 («сохранение»): так поступают все архиваторы, и
//! на крошечных частях вроде `_rels/.rels` это честнее.
//!
//! Не поддерживается намеренно: Zip64, шифрование, потоковая запись с
//! дескриптором данных. Файлы табличного документа заведомо меньше четырёх
//! гигабайт, а размеры и контрольные суммы известны до записи.

/// CRC-32 (полином `0xEDB88320`) — тот же, что у ZIP и PNG. Таблица
/// строится на первом обращении: 256 слов дешевле, чем побитовый цикл на
/// каждом байте, и всё равно считается один раз за запуск.
fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &b in data {
        crc ^= u32::from(b);
        for _ in 0..8 {
            let bit = crc & 1;
            crc >>= 1;
            if bit != 0 {
                crc ^= 0xEDB8_8320;
            }
        }
    }
    !crc
}

/// Один файл в архиве: сжатые данные, метод и исходный размер.
struct Entry {
    name: String,
    packed: Vec<u8>,
    method: u16,
    raw_len: u32,
    crc: u32,
    offset: u32,
}

/// Сборщик архива. Записи копятся в памяти: разметка XLSX — десятки
/// килобайт, а знание всех смещений заранее упрощает центральный каталог.
#[derive(Default)]
pub struct ZipWriter {
    out: Vec<u8>,
    entries: Vec<Entry>,
}

impl ZipWriter {
    pub fn new() -> Self {
        ZipWriter::default()
    }

    /// Добавить файл. Имя — с прямыми слэшами и без ведущего слэша, как
    /// требует формат.
    pub fn add(&mut self, name: &str, data: &[u8]) {
        let offset = self.out.len() as u32;
        let crc = crc32(data);
        let packed = crate::deflate::deflate(data);
        // Метод выбирается по результату: раздувать мелочь незачем.
        let (method, packed) = if packed.len() < data.len() {
            (8u16, packed)
        } else {
            (0u16, data.to_vec())
        };
        self.out.extend_from_slice(&0x0403_4B50u32.to_le_bytes()); // сигнатура
        self.out.extend_from_slice(&20u16.to_le_bytes()); // версия
                                                          // Бит 11 — имена в UTF-8; его ставит и платформа.
        self.out.extend_from_slice(&0x0800u16.to_le_bytes());
        self.out.extend_from_slice(&method.to_le_bytes());
        self.out.extend_from_slice(&0u16.to_le_bytes()); // время
        self.out.extend_from_slice(&0u16.to_le_bytes()); // дата
        self.out.extend_from_slice(&crc.to_le_bytes());
        self.out
            .extend_from_slice(&(packed.len() as u32).to_le_bytes());
        self.out
            .extend_from_slice(&(data.len() as u32).to_le_bytes());
        self.out
            .extend_from_slice(&(name.len() as u16).to_le_bytes());
        self.out.extend_from_slice(&0u16.to_le_bytes()); // extra
        self.out.extend_from_slice(name.as_bytes());
        self.out.extend_from_slice(&packed);
        self.entries.push(Entry {
            name: name.to_string(),
            packed,
            method,
            raw_len: data.len() as u32,
            crc,
            offset,
        });
    }

    /// Закрыть архив: центральный каталог и запись его конца.
    pub fn finish(mut self) -> Vec<u8> {
        let start = self.out.len() as u32;
        for e in &self.entries {
            self.out.extend_from_slice(&0x0201_4B50u32.to_le_bytes());
            self.out.extend_from_slice(&20u16.to_le_bytes()); // версия создателя
            self.out.extend_from_slice(&20u16.to_le_bytes()); // версия для распаковки
            self.out.extend_from_slice(&0x0800u16.to_le_bytes()); // флаги
            self.out.extend_from_slice(&e.method.to_le_bytes());
            self.out.extend_from_slice(&0u16.to_le_bytes());
            self.out.extend_from_slice(&0u16.to_le_bytes());
            self.out.extend_from_slice(&e.crc.to_le_bytes());
            self.out
                .extend_from_slice(&(e.packed.len() as u32).to_le_bytes());
            self.out.extend_from_slice(&e.raw_len.to_le_bytes());
            self.out
                .extend_from_slice(&(e.name.len() as u16).to_le_bytes());
            self.out.extend_from_slice(&0u16.to_le_bytes()); // extra
            self.out.extend_from_slice(&0u16.to_le_bytes()); // комментарий
            self.out.extend_from_slice(&0u16.to_le_bytes()); // номер диска
            self.out.extend_from_slice(&0u16.to_le_bytes()); // внутренние атрибуты
            self.out.extend_from_slice(&0u32.to_le_bytes()); // внешние атрибуты
            self.out.extend_from_slice(&e.offset.to_le_bytes());
            self.out.extend_from_slice(e.name.as_bytes());
        }
        let size = self.out.len() as u32 - start;
        let number = self.entries.len() as u16;
        self.out.extend_from_slice(&0x0605_4B50u32.to_le_bytes());
        self.out.extend_from_slice(&0u16.to_le_bytes()); // номер диска
        self.out.extend_from_slice(&0u16.to_le_bytes()); // диск с каталогом
        self.out.extend_from_slice(&number.to_le_bytes());
        self.out.extend_from_slice(&number.to_le_bytes());
        self.out.extend_from_slice(&size.to_le_bytes());
        self.out.extend_from_slice(&start.to_le_bytes());
        self.out.extend_from_slice(&0u16.to_le_bytes()); // комментарий
        self.out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Контрольные значения CRC-32 из спецификации PNG — независимая от нас
    /// сверка, а не «что посчитали, то и записали».
    #[test]
    fn crc32_matches_known_values() {
        assert_eq!(crc32(b""), 0);
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
        assert_eq!(
            crc32(b"The quick brown fox jumps over the lazy dog"),
            0x414F_A339
        );
    }

    /// Сжатие включено: повторяющаяся разметка обязана ужаться, а метод в
    /// заголовке — стать восьмым.
    #[test]
    fn data_are_compressed_with_method_eight() {
        let data = "<c r=\"A1\" t=\"s\"><v>0</v></c>".repeat(300);
        let mut z = ZipWriter::new();
        z.add("xl/sheet.xml", data.as_bytes());
        let bytes = z.finish();
        let method = u16::from_le_bytes([bytes[8], bytes[9]]);
        assert_eq!(method, 8, "должен быть deflate");
        assert!(bytes.len() * 3 < data.len(), "должно ужаться втрое");
    }

    #[test]
    fn the_archive_starts_with_a_signature_and_ends_with_the_directory_record() {
        let mut z = ZipWriter::new();
        z.add("a.txt", "привет".as_bytes());
        let bytes = z.finish();
        assert_eq!(&bytes[..4], &0x0403_4B50u32.to_le_bytes());
        assert!(bytes.windows(4).any(|w| w == 0x0605_4B50u32.to_le_bytes()));
    }
}
