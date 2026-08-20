//! Кодировки файлов для `ТекстовыйДокумент`.
//!
//! # Что здесь ИЗМЕРЕНО на 8.3.27
//!
//! Байты снимались шестнадцатеричным дампом файлов, которые записала САМА
//! платформа (текст «Аб», то есть U+0410 U+0431):
//!
//! * без указания кодировки и с `КодировкаТекста.UTF8` — `EF BB BF D0 90
//!   D0 B1`, то есть UTF-8 С СИГНАТУРОЙ;
//! * `КодировкаТекста.UTF16` и строка `"UTF-16LE"` — `FF FE 10 04 31 04`:
//!   UTF-16 little-endian, тоже с сигнатурой;
//! * `КодировкаТекста.ANSI` и строка `"windows-1251"` — `C0 E1`;
//! * `КодировкаТекста.OEM` и строка `"cp866"` — `80 A1`;
//! * строка `"KOI8-R"` — `E1 C2`;
//! * неизвестное имя кодировки — ошибка, а не тихий откат к UTF-8;
//! * чтение файла ЧУЖОЙ кодировкой не отказывает: негодные байты
//!   становятся U+FFFD.
//!
//! # Чего здесь НЕТ
//!
//! Платформа принимает около двух сотен имён кодировок (весь набор ICU).
//! Здесь поддержаны те, что покрывают обмен в кириллице: UTF-8, UTF-16 в
//! обоих порядках байт, UTF-32, windows-1251, cp866 и KOI8-R. Остальные
//! имена отвергаются ЯВНОЙ ошибкой — молча писать не в той кодировке хуже,
//! чем сказать, что не умеешь. Таблицы для новых однобайтовых кодировок
//! добавляются сюда же, к соседям.

use crate::{RtError, RtResult};

/// Таблица однобайтовой кодировки `windows-1251`: старшая половина, 0x80..0xFF.
/// Порождена из системных таблиц, младшая половина совпадает с ASCII.
static WINDOWS_1251: [char; 128] = [
    '\u{0402}', '\u{0403}', '\u{201a}', '\u{0453}', '\u{201e}', '\u{2026}', '\u{2020}', '\u{2021}',
    '\u{20ac}', '\u{2030}', '\u{0409}', '\u{2039}', '\u{040a}', '\u{040c}', '\u{040b}', '\u{040f}',
    '\u{0452}', '\u{2018}', '\u{2019}', '\u{201c}', '\u{201d}', '\u{2022}', '\u{2013}', '\u{2014}',
    '\u{fffd}', '\u{2122}', '\u{0459}', '\u{203a}', '\u{045a}', '\u{045c}', '\u{045b}', '\u{045f}',
    '\u{00a0}', '\u{040e}', '\u{045e}', '\u{0408}', '\u{00a4}', '\u{0490}', '\u{00a6}', '\u{00a7}',
    '\u{0401}', '\u{00a9}', '\u{0404}', '\u{00ab}', '\u{00ac}', '\u{00ad}', '\u{00ae}', '\u{0407}',
    '\u{00b0}', '\u{00b1}', '\u{0406}', '\u{0456}', '\u{0491}', '\u{00b5}', '\u{00b6}', '\u{00b7}',
    '\u{0451}', '\u{2116}', '\u{0454}', '\u{00bb}', '\u{0458}', '\u{0405}', '\u{0455}', '\u{0457}',
    '\u{0410}', '\u{0411}', '\u{0412}', '\u{0413}', '\u{0414}', '\u{0415}', '\u{0416}', '\u{0417}',
    '\u{0418}', '\u{0419}', '\u{041a}', '\u{041b}', '\u{041c}', '\u{041d}', '\u{041e}', '\u{041f}',
    '\u{0420}', '\u{0421}', '\u{0422}', '\u{0423}', '\u{0424}', '\u{0425}', '\u{0426}', '\u{0427}',
    '\u{0428}', '\u{0429}', '\u{042a}', '\u{042b}', '\u{042c}', '\u{042d}', '\u{042e}', '\u{042f}',
    '\u{0430}', '\u{0431}', '\u{0432}', '\u{0433}', '\u{0434}', '\u{0435}', '\u{0436}', '\u{0437}',
    '\u{0438}', '\u{0439}', '\u{043a}', '\u{043b}', '\u{043c}', '\u{043d}', '\u{043e}', '\u{043f}',
    '\u{0440}', '\u{0441}', '\u{0442}', '\u{0443}', '\u{0444}', '\u{0445}', '\u{0446}', '\u{0447}',
    '\u{0448}', '\u{0449}', '\u{044a}', '\u{044b}', '\u{044c}', '\u{044d}', '\u{044e}', '\u{044f}',
];

/// Таблица однобайтовой кодировки `cp866`: старшая половина, 0x80..0xFF.
/// Порождена из системных таблиц, младшая половина совпадает с ASCII.
static CP866: [char; 128] = [
    '\u{0410}', '\u{0411}', '\u{0412}', '\u{0413}', '\u{0414}', '\u{0415}', '\u{0416}', '\u{0417}',
    '\u{0418}', '\u{0419}', '\u{041a}', '\u{041b}', '\u{041c}', '\u{041d}', '\u{041e}', '\u{041f}',
    '\u{0420}', '\u{0421}', '\u{0422}', '\u{0423}', '\u{0424}', '\u{0425}', '\u{0426}', '\u{0427}',
    '\u{0428}', '\u{0429}', '\u{042a}', '\u{042b}', '\u{042c}', '\u{042d}', '\u{042e}', '\u{042f}',
    '\u{0430}', '\u{0431}', '\u{0432}', '\u{0433}', '\u{0434}', '\u{0435}', '\u{0436}', '\u{0437}',
    '\u{0438}', '\u{0439}', '\u{043a}', '\u{043b}', '\u{043c}', '\u{043d}', '\u{043e}', '\u{043f}',
    '\u{2591}', '\u{2592}', '\u{2593}', '\u{2502}', '\u{2524}', '\u{2561}', '\u{2562}', '\u{2556}',
    '\u{2555}', '\u{2563}', '\u{2551}', '\u{2557}', '\u{255d}', '\u{255c}', '\u{255b}', '\u{2510}',
    '\u{2514}', '\u{2534}', '\u{252c}', '\u{251c}', '\u{2500}', '\u{253c}', '\u{255e}', '\u{255f}',
    '\u{255a}', '\u{2554}', '\u{2569}', '\u{2566}', '\u{2560}', '\u{2550}', '\u{256c}', '\u{2567}',
    '\u{2568}', '\u{2564}', '\u{2565}', '\u{2559}', '\u{2558}', '\u{2552}', '\u{2553}', '\u{256b}',
    '\u{256a}', '\u{2518}', '\u{250c}', '\u{2588}', '\u{2584}', '\u{258c}', '\u{2590}', '\u{2580}',
    '\u{0440}', '\u{0441}', '\u{0442}', '\u{0443}', '\u{0444}', '\u{0445}', '\u{0446}', '\u{0447}',
    '\u{0448}', '\u{0449}', '\u{044a}', '\u{044b}', '\u{044c}', '\u{044d}', '\u{044e}', '\u{044f}',
    '\u{0401}', '\u{0451}', '\u{0404}', '\u{0454}', '\u{0407}', '\u{0457}', '\u{040e}', '\u{045e}',
    '\u{00b0}', '\u{2219}', '\u{00b7}', '\u{221a}', '\u{2116}', '\u{00a4}', '\u{25a0}', '\u{00a0}',
];

/// Таблица однобайтовой кодировки `KOI8-R`: старшая половина, 0x80..0xFF.
/// Порождена из системных таблиц, младшая половина совпадает с ASCII.
static KOI8_R: [char; 128] = [
    '\u{2500}', '\u{2502}', '\u{250c}', '\u{2510}', '\u{2514}', '\u{2518}', '\u{251c}', '\u{2524}',
    '\u{252c}', '\u{2534}', '\u{253c}', '\u{2580}', '\u{2584}', '\u{2588}', '\u{258c}', '\u{2590}',
    '\u{2591}', '\u{2592}', '\u{2593}', '\u{2320}', '\u{25a0}', '\u{2219}', '\u{221a}', '\u{2248}',
    '\u{2264}', '\u{2265}', '\u{00a0}', '\u{2321}', '\u{00b0}', '\u{00b2}', '\u{00b7}', '\u{00f7}',
    '\u{2550}', '\u{2551}', '\u{2552}', '\u{0451}', '\u{2553}', '\u{2554}', '\u{2555}', '\u{2556}',
    '\u{2557}', '\u{2558}', '\u{2559}', '\u{255a}', '\u{255b}', '\u{255c}', '\u{255d}', '\u{255e}',
    '\u{255f}', '\u{2560}', '\u{2561}', '\u{0401}', '\u{2562}', '\u{2563}', '\u{2564}', '\u{2565}',
    '\u{2566}', '\u{2567}', '\u{2568}', '\u{2569}', '\u{256a}', '\u{256b}', '\u{256c}', '\u{00a9}',
    '\u{044e}', '\u{0430}', '\u{0431}', '\u{0446}', '\u{0434}', '\u{0435}', '\u{0444}', '\u{0433}',
    '\u{0445}', '\u{0438}', '\u{0439}', '\u{043a}', '\u{043b}', '\u{043c}', '\u{043d}', '\u{043e}',
    '\u{043f}', '\u{044f}', '\u{0440}', '\u{0441}', '\u{0442}', '\u{0443}', '\u{0436}', '\u{0432}',
    '\u{044c}', '\u{044b}', '\u{0437}', '\u{0448}', '\u{044d}', '\u{0449}', '\u{0447}', '\u{044a}',
    '\u{042e}', '\u{0410}', '\u{0411}', '\u{0426}', '\u{0414}', '\u{0415}', '\u{0424}', '\u{0413}',
    '\u{0425}', '\u{0418}', '\u{0419}', '\u{041a}', '\u{041b}', '\u{041c}', '\u{041d}', '\u{041e}',
    '\u{041f}', '\u{042f}', '\u{0420}', '\u{0421}', '\u{0422}', '\u{0423}', '\u{0416}', '\u{0412}',
    '\u{042c}', '\u{042b}', '\u{0417}', '\u{0428}', '\u{042d}', '\u{0429}', '\u{0427}', '\u{042a}',
];

/// Кодировка, разобранная из члена перечисления или строки.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Encoding {
    /// UTF-8 с сигнатурой — то, что платформа пишет по умолчанию.
    Utf8,
    Utf16Le,
    Utf16Be,
    Utf32Le,
    Utf32Be,
    Windows1251,
    Cp866,
    Koi8R,
}

impl Encoding {
    /// Имя кодировки строкой. Регистр не важен: платформа принимает и
    /// `UTF-8`, и `utf-8`.
    ///
    /// # Errors
    ///
    /// [`RtError::TextDoc`], если имя не поддержано.
    pub fn by_name(name: &str) -> RtResult<Self> {
        let key = name.trim().to_uppercase().replace('_', "-");
        Ok(match key.as_str() {
            "UTF-8" | "UTF8" => Encoding::Utf8,
            // Без указания порядка байт платформа пишет little-endian —
            // измерено на `КодировкаТекста.UTF16`.
            "UTF-16" | "UTF16" | "UTF-16LE" | "UTF16LE" => Encoding::Utf16Le,
            "UTF-16BE" | "UTF16BE" => Encoding::Utf16Be,
            "UTF-32" | "UTF32" | "UTF-32LE" | "UTF32LE" => Encoding::Utf32Le,
            "UTF-32BE" | "UTF32BE" => Encoding::Utf32Be,
            "WINDOWS-1251" | "CP1251" | "ANSI" => Encoding::Windows1251,
            "CP866" | "IBM866" | "OEM" => Encoding::Cp866,
            "KOI8-R" => Encoding::Koi8R,
            _ => {
                return Err(RtError::TextDoc(format!(
                    "кодировка «{name}» не поддержана"
                )));
            }
        })
    }

    fn single_byte_table(self) -> Option<&'static [char; 128]> {
        match self {
            Encoding::Windows1251 => Some(&WINDOWS_1251),
            Encoding::Cp866 => Some(&CP866),
            Encoding::Koi8R => Some(&KOI8_R),
            _ => None,
        }
    }

    /// Сигнатура (BOM) этой кодировки — то, что [`Encoding::encode`] ставит
    /// перед текстом. У однобайтовых и у UTF-32 её нет: измерено, что
    /// платформа пишет сигнатуру только для UTF-8 и UTF-16.
    fn signature(self) -> &'static [u8] {
        match self {
            Encoding::Utf8 => &[0xEF, 0xBB, 0xBF],
            Encoding::Utf16Le => &[0xFF, 0xFE],
            Encoding::Utf16Be => &[0xFE, 0xFF],
            Encoding::Utf32Le | Encoding::Utf32Be => &[],
            Encoding::Windows1251 | Encoding::Cp866 | Encoding::Koi8R => &[],
        }
    }

    /// Текст в байты. Сигнатура ставится у UTF-8 и UTF-16 — так пишет
    /// платформа в ФАЙЛ (измерено на `ЗаписьТекста`/`ТекстовыйДокумент`).
    ///
    /// В ПОТОК та же платформа сигнатуру не пишет: `ЗаписьДанных` кладёт «Аб»
    /// ровно четырьмя байтами UTF-8, без `EF BB BF` (измерено). Для этого
    /// случая есть [`Encoding::encode_without_signature`].
    pub fn encode(self, text: &str) -> Vec<u8> {
        let mut out = self.signature().to_vec();
        out.extend_from_slice(&self.encode_without_signature(text));
        out
    }

    /// Текст в байты БЕЗ сигнатуры — то, что пишет `ЗаписьДанных` (измерено).
    pub fn encode_without_signature(self, text: &str) -> Vec<u8> {
        match self {
            Encoding::Utf8 => text.as_bytes().to_vec(),
            Encoding::Utf16Le | Encoding::Utf16Be => {
                let le = self == Encoding::Utf16Le;
                let mut out = Vec::with_capacity(text.len() * 2);
                for unit in text.encode_utf16() {
                    let b = if le {
                        unit.to_le_bytes()
                    } else {
                        unit.to_be_bytes()
                    };
                    out.extend_from_slice(&b);
                }
                out
            }
            Encoding::Utf32Le | Encoding::Utf32Be => {
                let le = self == Encoding::Utf32Le;
                let mut out = Vec::with_capacity(text.chars().count() * 4);
                for c in text.chars() {
                    let b = if le {
                        (c as u32).to_le_bytes()
                    } else {
                        (c as u32).to_be_bytes()
                    };
                    out.extend_from_slice(&b);
                }
                out
            }
            _ => {
                let table = self.single_byte_table().expect("однобайтовая таблица");
                text.chars()
                    .map(|c| {
                        if (c as u32) < 0x80 {
                            c as u8
                        } else {
                            // Символ, которого в кодировке нет, уходит
                            // вопросительным знаком — так же поступает
                            // подавляющее большинство однобайтовых
                            // преобразователей.
                            table
                                .iter()
                                .position(|t| *t == c)
                                .map_or(b'?', |i| (i + 0x80) as u8)
                        }
                    })
                    .collect()
            }
        }
    }

    /// Байты в текст. Негодные последовательности становятся U+FFFD, а не
    /// ошибкой — измерено на чтении файла чужой кодировкой. Ведущая
    /// сигнатура (BOM) снимается: файл, который платформа сама записала, с
    /// неё и начинается.
    pub fn decode(self, bytes: &[u8]) -> String {
        let s = self.decode_without_bom(bytes);
        s.strip_prefix('\u{feff}').unwrap_or(&s).to_string()
    }

    /// Байты в текст БЕЗ снятия ведущей сигнатуры — чтение куска ПОТОКА
    /// (`ЧтениеДанных`), где начало куска ничем не выделено и U+FEFF в нём
    /// такой же символ, как любой другой.
    pub fn decode_without_bom(self, bytes: &[u8]) -> String {
        match self {
            Encoding::Utf8 => String::from_utf8_lossy(bytes).into_owned(),
            Encoding::Utf16Le | Encoding::Utf16Be => {
                let le = self == Encoding::Utf16Le;
                let units: Vec<u16> = bytes
                    .chunks_exact(2)
                    .map(|p| {
                        if le {
                            u16::from_le_bytes([p[0], p[1]])
                        } else {
                            u16::from_be_bytes([p[0], p[1]])
                        }
                    })
                    .collect();
                String::from_utf16_lossy(&units)
            }
            Encoding::Utf32Le | Encoding::Utf32Be => {
                let le = self == Encoding::Utf32Le;
                bytes
                    .chunks_exact(4)
                    .map(|p| {
                        let v = if le {
                            u32::from_le_bytes([p[0], p[1], p[2], p[3]])
                        } else {
                            u32::from_be_bytes([p[0], p[1], p[2], p[3]])
                        };
                        char::from_u32(v).unwrap_or('\u{fffd}')
                    })
                    .collect()
            }
            _ => {
                let table = self.single_byte_table().expect("однобайтовая таблица");
                bytes
                    .iter()
                    .map(|b| {
                        if *b < 0x80 {
                            *b as char
                        } else {
                            table[(*b - 0x80) as usize]
                        }
                    })
                    .collect()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Байты сверены с тем, что записала САМА платформа: текст «Аб».
    #[test]
    fn bytes_match_what_the_platform_writes() {
        assert_eq!(
            Encoding::Utf8.encode("Аб"),
            vec![0xEF, 0xBB, 0xBF, 0xD0, 0x90, 0xD0, 0xB1]
        );
        assert_eq!(
            Encoding::Utf16Le.encode("Аб"),
            vec![0xFF, 0xFE, 0x10, 0x04, 0x31, 0x04]
        );
        assert_eq!(Encoding::Windows1251.encode("Аб"), vec![0xC0, 0xE1]);
        assert_eq!(Encoding::Cp866.encode("Аб"), vec![0x80, 0xA1]);
        assert_eq!(Encoding::Koi8R.encode("Аб"), vec![0xE1, 0xC2]);
    }

    #[test]
    fn every_encoding_round_trips_cyrillic() {
        for enc in [
            Encoding::Utf8,
            Encoding::Utf16Le,
            Encoding::Utf16Be,
            Encoding::Utf32Le,
            Encoding::Utf32Be,
            Encoding::Windows1251,
            Encoding::Cp866,
            Encoding::Koi8R,
        ] {
            let text = "Привет, мир! Ёжик.";
            assert_eq!(enc.decode(&enc.encode(text)), text, "{enc:?}");
        }
    }

    /// Чужая кодировка при чтении — не отказ, а U+FFFD (измерено).
    #[test]
    fn wrong_encoding_yields_replacement_characters() {
        let bytes = Encoding::Windows1251.encode("Аб");
        assert!(Encoding::Utf8.decode(&bytes).contains('\u{fffd}'));
    }

    #[test]
    fn unknown_name_is_rejected_rather_than_silently_utf8() {
        assert!(Encoding::by_name("нет-такой").is_err());
        assert!(Encoding::by_name("utf-8").is_ok());
        assert!(Encoding::by_name("WINDOWS-1251").is_ok());
    }
}
