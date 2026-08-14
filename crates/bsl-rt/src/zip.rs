//! Контейнер ZIP: писатель для XLSX и читатель произвольных архивов.
//!
//! Внешних крейтов в этом рабочем пространстве нет и не предвидится, поэтому
//! формат разобран здесь. Писатель [`ZipWriter`] умеет ровно то, что нужно
//! табличному документу; читатель [`ZipArchive`] — наоборот, обязан принять
//! всё, что кладут в архив чужие инструменты, и ни на каком мусоре не
//! уронить процесс.
//!
//! Разделение обязанностей у читателя одно и жёсткое: истина — центральный
//! каталог. Локальный заголовок нужен лишь затем, чтобы по его собственным
//! длинам имени и extra найти начало данных; размеры, CRC и метод берутся из
//! каталога. Иначе записи с дескриптором данных (бит 3 общих флагов), у
//! которых в локальном заголовке законно стоят нули, читались бы как пустые.
//!
//! Не поддерживается намеренно: шифрование (бит 0 — честная ошибка, а не
//! попытка расшифровать), многотомные архивы и способы хранения, кроме 0 и 8;
//! всё это распознаётся и называется в тексте ошибки.

use crate::RtError;

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
///
/// Писатель намеренно не умеет Zip64, шифрование и потоковую запись с
/// дескриптором данных: файлы табличного документа заведомо меньше четырёх
/// гигабайт, а размеры и контрольные суммы известны до записи. Читатель
/// [`ZipArchive`] рядом всё это, наоборот, разбирает — он имеет дело с
/// чужими архивами.
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

// --------------------------------------------------------------------------
// Читатель
// --------------------------------------------------------------------------

/// Сигнатура локального заголовка записи (APPNOTE 4.3.7).
const SIG_LOCAL: u32 = 0x0403_4B50;
/// Сигнатура записи центрального каталога (APPNOTE 4.3.12).
const SIG_CENTRAL: u32 = 0x0201_4B50;
/// Сигнатура записи конца центрального каталога (APPNOTE 4.3.16).
const SIG_EOCD: u32 = 0x0605_4B50;
/// Сигнатура записи конца каталога Zip64 (APPNOTE 4.3.14).
const SIG_EOCD64: u32 = 0x0606_4B50;
/// Сигнатура локатора записи конца каталога Zip64 (APPNOTE 4.3.15).
const SIG_EOCD64_LOCATOR: u32 = 0x0706_4B50;

/// Длина неизменяемой части локального заголовка.
const LOCAL_HEADER_LEN: usize = 30;
/// Длина неизменяемой части записи каталога.
const CENTRAL_HEADER_LEN: usize = 46;
/// Длина записи конца каталога без комментария.
const EOCD_LEN: usize = 22;
/// Длина записи конца каталога Zip64 (версия 1, без расширяемых данных).
const EOCD64_LEN: usize = 56;
/// Длина локатора записи конца каталога Zip64.
const EOCD64_LOCATOR_LEN: usize = 20;

/// Комментарий архива не длиннее 65535 байт — его длина двухбайтовая, так
/// что дальше этого запись конца каталога от конца файла не отодвигается.
const MAX_COMMENT: usize = 0xFFFF;

/// Значение, которым 32-битное поле объявляет себя вынесенным в Zip64.
const MAX_U32: u32 = 0xFFFF_FFFF;
/// То же для 16-битных полей (число записей, номер диска).
const MAX_U16: u16 = 0xFFFF;

/// Бит 0 общих флагов — данные записи зашифрованы.
const FLAG_ENCRYPTED: u16 = 1;
/// Бит 3 — размеры и CRC вынесены в дескриптор данных после самих данных.
const FLAG_DATA_DESCRIPTOR: u16 = 1 << 3;
/// Бит 11 — имя записи в UTF-8, а не в однобайтовой кодовой странице.
const FLAG_UTF8_NAME: u16 = 1 << 11;

/// Способ хранения 0 — данные лежат как есть.
const METHOD_STORED: u16 = 0;
/// Способ хранения 8 — поток deflate (RFC 1951), см. [`crate::inflate`].
const METHOD_DEFLATED: u16 = 8;

fn zip_err(what: &str) -> RtError {
    RtError::Zip(format!("ZIP: {what}"))
}

fn truncated() -> RtError {
    zip_err("архив обрезан или испорчен")
}

/// Срез `len` байт по смещению `at`. Единственный способ добраться до байт
/// архива: любое поле формата берётся отсюда, поэтому ни арифметика
/// смещений, ни выход за конец файла не могут превратиться в панику.
fn slice_at(data: &[u8], at: usize, len: usize) -> Result<&[u8], RtError> {
    let end = at.checked_add(len).ok_or_else(truncated)?;
    data.get(at..end).ok_or_else(truncated)
}

fn u16_at(data: &[u8], at: usize) -> Result<u16, RtError> {
    // Срез ровно из двух байт, индексы заведомо в границах.
    let b = slice_at(data, at, 2)?;
    Ok(u16::from_le_bytes([b[0], b[1]]))
}

fn u32_at(data: &[u8], at: usize) -> Result<u32, RtError> {
    let b = slice_at(data, at, 4)?;
    Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

fn u64_at(data: &[u8], at: usize) -> Result<u64, RtError> {
    let b = slice_at(data, at, 8)?;
    Ok(u64::from_le_bytes([
        b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
    ]))
}

/// Смещение или размер из архива в адрес этой платформы. На 64 битах
/// преобразование не отказывает никогда, на 32 — отказывает раньше, чем
/// вычисление уедет по модулю.
fn to_usize(value: u64) -> Result<usize, RtError> {
    usize::try_from(value)
        .map_err(|_| zip_err("запись архива не помещается в адресное пространство"))
}

/// Одна запись архива — так, как её описывает центральный каталог.
///
/// Имя хранится СЫРЫМИ байтами: в однобайтовых архивах кодовая страница
/// именем не задана, а какую из них берёт платформа 1С — вопрос отдельной
/// задачи, и угадывать его здесь нечего. Декодированное имя отдаётся только
/// когда его объявил сам архив, битом 11 общих флагов.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) struct ZipEntry {
    name: Vec<u8>,
    utf8_name: bool,
    method: u16,
    crc: u32,
    compressed_size: u64,
    size: u64,
    local_offset: u64,
    encrypted: bool,
    data_descriptor: bool,
}

#[cfg_attr(not(test), allow(dead_code))]
impl ZipEntry {
    /// Имя записи как оно лежит в каталоге, без всякого перекодирования.
    pub(crate) fn name_bytes(&self) -> &[u8] {
        &self.name
    }

    /// Имя записи текстом — только если архив объявил его в UTF-8 (бит 11) и
    /// байты действительно образуют UTF-8. Во всех прочих случаях `None`, и
    /// работать нужно с [`ZipEntry::name_bytes`].
    pub(crate) fn name(&self) -> Option<&str> {
        if self.utf8_name {
            std::str::from_utf8(&self.name).ok()
        } else {
            None
        }
    }

    /// Способ хранения из каталога: 0 — как есть, 8 — deflate, прочие
    /// читателю неизвестны.
    pub(crate) fn method(&self) -> u16 {
        self.method
    }

    /// Контрольная сумма распакованных данных из каталога.
    pub(crate) fn crc(&self) -> u32 {
        self.crc
    }

    /// Размер данных в архиве.
    pub(crate) fn compressed_size(&self) -> u64 {
        self.compressed_size
    }

    /// Размер распакованных данных.
    pub(crate) fn size(&self) -> u64 {
        self.size
    }

    /// Смещение локального заголовка записи от начала файла.
    pub(crate) fn local_offset(&self) -> u64 {
        self.local_offset
    }

    /// Данные записи зашифрованы (бит 0). Расшифровки нет: [`ZipArchive::read`]
    /// на такой записи отказывает, не читая данных.
    pub(crate) fn is_encrypted(&self) -> bool {
        self.encrypted
    }

    /// Размеры и CRC записи вынесены в дескриптор данных (бит 3). Чтению это
    /// безразлично — всё нужное есть в каталоге, — но знать полезно.
    pub(crate) fn has_data_descriptor(&self) -> bool {
        self.data_descriptor
    }

    /// Запись — это каталог: формат обозначает его завершающим слэшем в
    /// имени, отдельного признака в нём нет.
    pub(crate) fn is_directory(&self) -> bool {
        self.name.last() == Some(&b'/')
    }

    /// Имя для текста ошибки. Не подменяет [`ZipEntry::name`]: однобайтовые
    /// имена здесь показываются с заменяющими символами, и это годится
    /// только для сообщения человеку.
    fn name_for_message(&self) -> String {
        String::from_utf8_lossy(&self.name).into_owned()
    }
}

/// Разобранный архив: байты и центральный каталог, приведённый к записям.
///
/// Данные не копируются — [`ZipArchive`] живёт не дольше среза, из которого
/// разобран, — а распаковка происходит в [`ZipArchive::read`] по требованию.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) struct ZipArchive<'a> {
    data: &'a [u8],
    entries: Vec<ZipEntry>,
}

/// Короткая форма для сообщений: сами байты архива в отладочном выводе не
/// нужны, а вот сколько их и сколько записей нашлось — нужно.
impl std::fmt::Debug for ZipArchive<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ZipArchive({} байт, записей {})",
            self.data.len(),
            self.entries.len()
        )
    }
}

#[cfg_attr(not(test), allow(dead_code))]
impl<'a> ZipArchive<'a> {
    /// Разобрать архив по центральному каталогу.
    ///
    /// # Errors
    ///
    /// [`RtError::Zip`] на любом входе, который не является читаемым
    /// архивом: короче записи конца каталога, без неё самой, с каталогом за
    /// границами файла, с числом записей или размером каталога, не
    /// совпавшими с фактически разобранным, с многотомностью или с
    /// испорченным полем Zip64.
    pub(crate) fn parse(data: &'a [u8]) -> Result<ZipArchive<'a>, RtError> {
        let eocd = find_eocd(data)?;

        let mut disk = u64::from(u16_at(data, eocd + 4)?);
        let mut cd_disk = u64::from(u16_at(data, eocd + 6)?);
        let mut here = u64::from(u16_at(data, eocd + 8)?);
        let mut total = u64::from(u16_at(data, eocd + 10)?);
        let mut cd_size = u64::from(u32_at(data, eocd + 12)?);
        let mut cd_offset = u64::from(u32_at(data, eocd + 16)?);

        // Признак Zip64 — выставленные в максимум поля. Отсутствие локатора
        // при этом не ошибка само по себе: архив ровно с 65535 записями
        // пишет 0xFFFF и без всякого Zip64, а несуразное смещение каталога
        // поймает проверка границ ниже. А вот локатор, который нашёлся, но
        // ведёт не на запись конца каталога Zip64, — уже порча.
        let maxed = disk == u64::from(MAX_U16)
            || cd_disk == u64::from(MAX_U16)
            || here == u64::from(MAX_U16)
            || total == u64::from(MAX_U16)
            || cd_size == u64::from(MAX_U32)
            || cd_offset == u64::from(MAX_U32);
        if maxed {
            if let Some(locator) = eocd.checked_sub(EOCD64_LOCATOR_LEN) {
                if u32_at(data, locator)? == SIG_EOCD64_LOCATOR {
                    if u32_at(data, locator + 4)? != 0 || u32_at(data, locator + 16)? > 1 {
                        return Err(zip_err("многотомные архивы не поддерживаются"));
                    }
                    let at = to_usize(u64_at(data, locator + 8)?)?;
                    // Дальше читаются поля фиксированной части записи
                    // (APPNOTE 4.3.14); объявленный в ней размер может быть и
                    // больше — за счёт расширяемых данных, которые нам не
                    // нужны, — но меньше EOCD64_LEN она не бывает.
                    let record = slice_at(data, at, EOCD64_LEN)?;
                    if u32_at(record, 0)? != SIG_EOCD64 {
                        return Err(zip_err(
                            "локатор Zip64 указывает не на запись конца каталога",
                        ));
                    }
                    disk = u64::from(u32_at(record, 16)?);
                    cd_disk = u64::from(u32_at(record, 20)?);
                    here = u64_at(record, 24)?;
                    total = u64_at(record, 32)?;
                    cd_size = u64_at(record, 40)?;
                    cd_offset = u64_at(record, 48)?;
                }
            }
        }

        if disk != 0 || cd_disk != 0 || here != total {
            return Err(zip_err("многотомные архивы не поддерживаются"));
        }

        let start = to_usize(cd_offset)?;
        let len = to_usize(cd_size)?;
        let end = start.checked_add(len).ok_or_else(truncated)?;
        if end > data.len() {
            return Err(zip_err("центральный каталог выходит за границу файла"));
        }
        // Каталог читается из среза ровно по объявленной длине: запись,
        // которая вылезла бы за неё, упрётся в конец среза и станет ошибкой,
        // а не молча съест соседние байты.
        let region = data.get(..end).ok_or_else(truncated)?;

        let mut entries = Vec::new();
        let mut at = start;
        while at < end {
            let (entry, next) = parse_central_entry(region, at)?;
            entries.push(entry);
            at = next;
        }
        if entries.len() as u64 != total {
            return Err(zip_err(&format!(
                "в записи конца каталога объявлено {total} записей, а в каталоге их {}",
                entries.len()
            )));
        }

        Ok(ZipArchive { data, entries })
    }

    /// Записи в порядке центрального каталога.
    pub(crate) fn entries(&self) -> &[ZipEntry] {
        &self.entries
    }

    /// Прочитать и распаковать запись с номером `index`.
    ///
    /// Размеры, способ и CRC берутся из центрального каталога; локальный
    /// заголовок нужен только затем, чтобы по его собственным длинам имени и
    /// extra найти начало данных.
    ///
    /// # Errors
    ///
    /// [`RtError::Zip`], если записи с таким номером нет, данные зашифрованы,
    /// способ хранения не 0 и не 8, локальный заголовок не на месте, данные
    /// выходят за границу файла, распакованное короче объявленного или
    /// контрольная сумма не совпала с каталожной.
    pub(crate) fn read(&self, index: usize) -> Result<Vec<u8>, RtError> {
        let entry = self
            .entries
            .get(index)
            .ok_or_else(|| zip_err(&format!("в архиве нет записи с номером {index}")))?;

        // Отказ до всякого чтения данных: расшифровки здесь нет, и делать
        // вид, что данные прочитаны, нельзя.
        if entry.encrypted {
            return Err(zip_err(&format!(
                "запись «{}» зашифрована, а зашифрованные архивы не поддерживаются",
                entry.name_for_message()
            )));
        }

        let header = to_usize(entry.local_offset)?;
        if u32_at(self.data, header)? != SIG_LOCAL {
            return Err(zip_err(&format!(
                "у записи «{}» нет локального заголовка по объявленному смещению",
                entry.name_for_message()
            )));
        }
        // Длины имени и extra берутся из локального заголовка: они законно
        // отличаются от каталожных (Zip64 и метки времени пишут в extra
        // по-разному в двух местах).
        let name_len = usize::from(u16_at(self.data, header + 26)?);
        let extra_len = usize::from(u16_at(self.data, header + 28)?);
        let at = header
            .checked_add(LOCAL_HEADER_LEN)
            .and_then(|v| v.checked_add(name_len))
            .and_then(|v| v.checked_add(extra_len))
            .ok_or_else(truncated)?;

        let size = to_usize(entry.size)?;
        let packed = slice_at(self.data, at, to_usize(entry.compressed_size)?)?;

        let out = match entry.method {
            METHOD_STORED => {
                if packed.len() != size {
                    return Err(zip_err(&format!(
                        "у записи «{}» способ хранения 0, но размеры не совпадают",
                        entry.name_for_message()
                    )));
                }
                packed.to_vec()
            }
            METHOD_DEFLATED => {
                let out = crate::inflate::inflate(packed, size)?;
                // Предел в `inflate` не даёт распаковать больше объявленного,
                // а вот меньше — признак того, что поток обрезан по границе
                // блока, и молчать об этом нельзя.
                if out.len() != size {
                    return Err(zip_err(&format!(
                        "у записи «{}» размер не совпал: распаковано {} байт вместо {size}",
                        entry.name_for_message(),
                        out.len()
                    )));
                }
                out
            }
            other => {
                return Err(zip_err(&format!(
                    "у записи «{}» способ хранения {other} не поддерживается",
                    entry.name_for_message()
                )))
            }
        };

        if crc32(&out) != entry.crc {
            return Err(zip_err(&format!(
                "у записи «{}» не совпала контрольная сумма CRC-32",
                entry.name_for_message()
            )));
        }
        Ok(out)
    }
}

/// Найти запись конца центрального каталога.
///
/// Она стоит последней, но за ней ещё может лежать комментарий архива
/// переменной длины, поэтому её ищут сканированием от конца назад.
/// Сигнатуры мало: те же четыре байта могут случайно оказаться и в
/// комментарии, и в сжатых данных, — кандидат считается настоящим, только
/// если объявленная в нём длина комментария доводит ровно до конца файла.
fn find_eocd(data: &[u8]) -> Result<usize, RtError> {
    if data.len() < EOCD_LEN {
        return Err(zip_err(
            "файл короче записи конца каталога — это не архив ZIP",
        ));
    }
    let last = data.len() - EOCD_LEN;
    let first = last.saturating_sub(MAX_COMMENT);
    for at in (first..=last).rev() {
        if u32_at(data, at)? != SIG_EOCD {
            continue;
        }
        let comment = usize::from(u16_at(data, at + 20)?);
        if at + EOCD_LEN + comment == data.len() {
            return Ok(at);
        }
    }
    Err(zip_err(
        "не найдена запись конца центрального каталога — это не архив ZIP",
    ))
}

/// Разобрать одну запись каталога, начиная с `at`; вернуть её и смещение
/// следующей. `region` обрезан концом каталога, поэтому запись, вылезающая
/// за него, отказывает здесь же.
fn parse_central_entry(region: &[u8], at: usize) -> Result<(ZipEntry, usize), RtError> {
    if u32_at(region, at)? != SIG_CENTRAL {
        return Err(zip_err("в центральном каталоге запись без сигнатуры"));
    }
    let flags = u16_at(region, at + 8)?;
    let method = u16_at(region, at + 10)?;
    let crc = u32_at(region, at + 16)?;
    let compressed = u32_at(region, at + 20)?;
    let size = u32_at(region, at + 24)?;
    let name_len = usize::from(u16_at(region, at + 28)?);
    let extra_len = usize::from(u16_at(region, at + 30)?);
    let comment_len = usize::from(u16_at(region, at + 32)?);
    let mut disk = u64::from(u16_at(region, at + 34)?);
    let offset = u32_at(region, at + 42)?;

    let name_at = at.checked_add(CENTRAL_HEADER_LEN).ok_or_else(truncated)?;
    let name = slice_at(region, name_at, name_len)?.to_vec();
    let extra_at = name_at.checked_add(name_len).ok_or_else(truncated)?;
    let extra = slice_at(region, extra_at, extra_len)?;
    // Комментарий записи не нужен, но его длину надо пройти, чтобы попасть в
    // следующую запись, — и убедиться, что он помещается в каталог.
    let comment_at = extra_at.checked_add(extra_len).ok_or_else(truncated)?;
    slice_at(region, comment_at, comment_len)?;
    let next = comment_at.checked_add(comment_len).ok_or_else(truncated)?;

    let mut entry = ZipEntry {
        name,
        utf8_name: flags & FLAG_UTF8_NAME != 0,
        method,
        crc,
        compressed_size: u64::from(compressed),
        size: u64::from(size),
        local_offset: u64::from(offset),
        encrypted: flags & FLAG_ENCRYPTED != 0,
        data_descriptor: flags & FLAG_DATA_DESCRIPTOR != 0,
    };

    read_zip64_extra(extra, &mut entry, &mut disk, compressed, size, offset)?;
    if disk != 0 {
        return Err(zip_err("многотомные архивы не поддерживаются"));
    }

    Ok((entry, next))
}

/// Достать из extra-полей записи значения Zip64 (идентификатор 0x0001).
///
/// Порядок значений в поле задан жёстко — несжатый размер, сжатый размер,
/// смещение локального заголовка, номер диска, — но присутствуют ТОЛЬКО те,
/// что в самой записи выставлены в максимум. Поэтому разбор идёт по этому
/// правилу, а не по длине блока: писатели с одним вынесенным значением
/// (только размеры или только смещение) встречаются постоянно.
fn read_zip64_extra(
    extra: &[u8],
    entry: &mut ZipEntry,
    disk: &mut u64,
    compressed: u32,
    size: u32,
    offset: u32,
) -> Result<(), RtError> {
    let mut at = 0;
    while at + 4 <= extra.len() {
        let id = u16_at(extra, at)?;
        let len = usize::from(u16_at(extra, at + 2)?);
        let body = slice_at(extra, at + 4, len)
            .map_err(|_| zip_err("extra-поле записи выходит за её границу"))?;
        at += 4 + len;
        if id != 1 {
            continue;
        }

        let mut cursor = 0;
        if size == MAX_U32 {
            entry.size = take_zip64_u64(body, &mut cursor)?;
        }
        if compressed == MAX_U32 {
            entry.compressed_size = take_zip64_u64(body, &mut cursor)?;
        }
        if offset == MAX_U32 {
            entry.local_offset = take_zip64_u64(body, &mut cursor)?;
        }
        if *disk == u64::from(MAX_U16) {
            *disk =
                u64::from(u32_at(body, cursor).map_err(|_| zip_err("поле Zip64 записи обрезано"))?);
        }
    }
    Ok(())
}

/// Очередное восьмибайтовое значение поля Zip64.
fn take_zip64_u64(body: &[u8], cursor: &mut usize) -> Result<u64, RtError> {
    let value = u64_at(body, *cursor).map_err(|_| zip_err("поле Zip64 записи обрезано"))?;
    *cursor += 8;
    Ok(value)
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

    // ---------------------------------------------------------------------
    // Эталонные архивы
    //
    // Эталонов шесть. Пять собраны здешним python3 (CPython 3.14) через
    // `zipfile`, шестой (`REF_ZIP64_CD`) — руками через `struct.pack`, потому
    // что `zipfile` выносит размеры в extra каталога только выше четырёх
    // гигабайт. Все шесть прочитаны тем же python3 обратно перед тем, как
    // попасть сюда: список имён, размеры, способы и содержимое сошлись, а у
    // собранного руками сверх того прошёл `zipfile.testzip()`. Даты записей
    // выставлены в 1980-01-01, иначе байты зависели бы от времени прогона.
    // Здешний python3 собран с zlib-ng
    // (`zlib.ZLIB_RUNTIME_VERSION` — `1.3.1.zlib-ng`), поэтому СЖАТЫЕ байты
    // на другой машине при том же содержимом могут выйти иными: эталон
    // проверяется как байты, а не как воспроизводимая команда.
    //
    // Общий пролог команд происхождения:
    //
    // ```python
    // import io, struct, zipfile, zlib
    // text = ("Товар;Цена\n"
    //         + "".join(f"Гвоздь-{i%3};{i*7}\n" for i in range(12))).encode()
    // noise = bytes(((i * 97 + 13) & 0xFF) for i in range(24))
    // def info(name, method):
    //     zi = zipfile.ZipInfo(name, date_time=(1980, 1, 1, 0, 0, 0))
    //     zi.compress_type = method
    //     return zi
    // ```
    // ---------------------------------------------------------------------

    /// Записи обоих способов хранения: сжимаемый текст (метод 8) и шум
    /// (метод 0). Имена кириллические, поэтому zipfile ставит бит 11.
    ///
    /// ```python
    /// buf = io.BytesIO()
    /// with zipfile.ZipFile(buf, 'w') as zf:
    ///     zf.writestr(info('накладная.txt', zipfile.ZIP_DEFLATED), text)
    ///     zf.writestr(info('шум.bin', zipfile.ZIP_STORED), noise)
    /// print(list(buf.getvalue()))
    /// ```
    const REF_MIXED: [u8; 349] = [
        0x50, 0x4B, 0x03, 0x04, 0x14, 0x00, 0x00, 0x08, 0x08, 0x00, 0x00, 0x00, 0x21, 0x00, 0x19,
        0x1C, 0xAD, 0xDB, 0x57, 0x00, 0x00, 0x00, 0xEA, 0x00, 0x00, 0x00, 0x16, 0x00, 0x00, 0x00,
        0xD0, 0xBD, 0xD0, 0xB0, 0xD0, 0xBA, 0xD0, 0xBB, 0xD0, 0xB0, 0xD0, 0xB4, 0xD0, 0xBD, 0xD0,
        0xB0, 0xD1, 0x8F, 0x2E, 0x74, 0x78, 0x74, 0x65, 0xCE, 0xBD, 0x09, 0x80, 0x30, 0x14, 0x45,
        0xE1, 0x3E, 0xBB, 0x08, 0xC9, 0xCB, 0x9F, 0xF2, 0x96, 0x73, 0x00, 0x3B, 0x37, 0xB0, 0xB0,
        0x0E, 0x41, 0xB1, 0x32, 0x33, 0xDC, 0x6C, 0x64, 0x9D, 0x6B, 0x7B, 0xE0, 0x83, 0x83, 0x03,
        0x0D, 0x15, 0xA5, 0xAF, 0x8A, 0x13, 0x37, 0x5E, 0x14, 0x83, 0x1D, 0x15, 0x0D, 0x0F, 0xAE,
        0xBE, 0x4D, 0x56, 0xED, 0x18, 0x9C, 0xE6, 0x31, 0x88, 0xBA, 0xC0, 0x46, 0x1C, 0x23, 0x99,
        0x59, 0xF9, 0xC8, 0x2A, 0x08, 0xAB, 0xB0, 0xB0, 0x8A, 0x89, 0x55, 0xF2, 0xBF, 0x41, 0x5A,
        0x16, 0xCD, 0xD9, 0x7C, 0x50, 0x4B, 0x03, 0x04, 0x14, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00,
        0x00, 0x21, 0x00, 0x88, 0x04, 0x12, 0x95, 0x18, 0x00, 0x00, 0x00, 0x18, 0x00, 0x00, 0x00,
        0x0A, 0x00, 0x00, 0x00, 0xD1, 0x88, 0xD1, 0x83, 0xD0, 0xBC, 0x2E, 0x62, 0x69, 0x6E, 0x0D,
        0x6E, 0xCF, 0x30, 0x91, 0xF2, 0x53, 0xB4, 0x15, 0x76, 0xD7, 0x38, 0x99, 0xFA, 0x5B, 0xBC,
        0x1D, 0x7E, 0xDF, 0x40, 0xA1, 0x02, 0x63, 0xC4, 0x50, 0x4B, 0x01, 0x02, 0x14, 0x03, 0x14,
        0x00, 0x00, 0x08, 0x08, 0x00, 0x00, 0x00, 0x21, 0x00, 0x19, 0x1C, 0xAD, 0xDB, 0x57, 0x00,
        0x00, 0x00, 0xEA, 0x00, 0x00, 0x00, 0x16, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x80, 0x01, 0x00, 0x00, 0x00, 0x00, 0xD0, 0xBD, 0xD0, 0xB0, 0xD0, 0xBA,
        0xD0, 0xBB, 0xD0, 0xB0, 0xD0, 0xB4, 0xD0, 0xBD, 0xD0, 0xB0, 0xD1, 0x8F, 0x2E, 0x74, 0x78,
        0x74, 0x50, 0x4B, 0x01, 0x02, 0x14, 0x03, 0x14, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x00,
        0x21, 0x00, 0x88, 0x04, 0x12, 0x95, 0x18, 0x00, 0x00, 0x00, 0x18, 0x00, 0x00, 0x00, 0x0A,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80, 0x01, 0x8B, 0x00,
        0x00, 0x00, 0xD1, 0x88, 0xD1, 0x83, 0xD0, 0xBC, 0x2E, 0x62, 0x69, 0x6E, 0x50, 0x4B, 0x05,
        0x06, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x02, 0x00, 0x7C, 0x00, 0x00, 0x00, 0xCB, 0x00,
        0x00, 0x00, 0x00, 0x00,
    ];

    /// Архив с комментарием: запись конца каталога отодвинута от конца файла
    /// на длину комментария, и читатель, смотрящий только на последние 22
    /// байта, такого архива не увидит.
    ///
    /// ```python
    /// buf = io.BytesIO()
    /// with zipfile.ZipFile(buf, 'w') as zf:
    ///     zf.writestr(info('отчёт.txt', zipfile.ZIP_DEFLATED), text)
    ///     zf.comment = ("Комментарий архива: отчётность за 2026 год, "
    ///                   "выгрузка номер 14.").encode()
    /// print(list(buf.getvalue()))
    /// ```
    const REF_COMMENT: [u8; 320] = [
        0x50, 0x4B, 0x03, 0x04, 0x14, 0x00, 0x00, 0x08, 0x08, 0x00, 0x00, 0x00, 0x21, 0x00, 0x19,
        0x1C, 0xAD, 0xDB, 0x57, 0x00, 0x00, 0x00, 0xEA, 0x00, 0x00, 0x00, 0x0E, 0x00, 0x00, 0x00,
        0xD0, 0xBE, 0xD1, 0x82, 0xD1, 0x87, 0xD1, 0x91, 0xD1, 0x82, 0x2E, 0x74, 0x78, 0x74, 0x65,
        0xCE, 0xBD, 0x09, 0x80, 0x30, 0x14, 0x45, 0xE1, 0x3E, 0xBB, 0x08, 0xC9, 0xCB, 0x9F, 0xF2,
        0x96, 0x73, 0x00, 0x3B, 0x37, 0xB0, 0xB0, 0x0E, 0x41, 0xB1, 0x32, 0x33, 0xDC, 0x6C, 0x64,
        0x9D, 0x6B, 0x7B, 0xE0, 0x83, 0x83, 0x03, 0x0D, 0x15, 0xA5, 0xAF, 0x8A, 0x13, 0x37, 0x5E,
        0x14, 0x83, 0x1D, 0x15, 0x0D, 0x0F, 0xAE, 0xBE, 0x4D, 0x56, 0xED, 0x18, 0x9C, 0xE6, 0x31,
        0x88, 0xBA, 0xC0, 0x46, 0x1C, 0x23, 0x99, 0x59, 0xF9, 0xC8, 0x2A, 0x08, 0xAB, 0xB0, 0xB0,
        0x8A, 0x89, 0x55, 0xF2, 0xBF, 0x41, 0x5A, 0x16, 0xCD, 0xD9, 0x7C, 0x50, 0x4B, 0x01, 0x02,
        0x14, 0x03, 0x14, 0x00, 0x00, 0x08, 0x08, 0x00, 0x00, 0x00, 0x21, 0x00, 0x19, 0x1C, 0xAD,
        0xDB, 0x57, 0x00, 0x00, 0x00, 0xEA, 0x00, 0x00, 0x00, 0x0E, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80, 0x01, 0x00, 0x00, 0x00, 0x00, 0xD0, 0xBE, 0xD1,
        0x82, 0xD1, 0x87, 0xD1, 0x91, 0xD1, 0x82, 0x2E, 0x74, 0x78, 0x74, 0x50, 0x4B, 0x05, 0x06,
        0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x01, 0x00, 0x3C, 0x00, 0x00, 0x00, 0x83, 0x00, 0x00,
        0x00, 0x6B, 0x00, 0xD0, 0x9A, 0xD0, 0xBE, 0xD0, 0xBC, 0xD0, 0xBC, 0xD0, 0xB5, 0xD0, 0xBD,
        0xD1, 0x82, 0xD0, 0xB0, 0xD1, 0x80, 0xD0, 0xB8, 0xD0, 0xB9, 0x20, 0xD0, 0xB0, 0xD1, 0x80,
        0xD1, 0x85, 0xD0, 0xB8, 0xD0, 0xB2, 0xD0, 0xB0, 0x3A, 0x20, 0xD0, 0xBE, 0xD1, 0x82, 0xD1,
        0x87, 0xD1, 0x91, 0xD1, 0x82, 0xD0, 0xBD, 0xD0, 0xBE, 0xD1, 0x81, 0xD1, 0x82, 0xD1, 0x8C,
        0x20, 0xD0, 0xB7, 0xD0, 0xB0, 0x20, 0x32, 0x30, 0x32, 0x36, 0x20, 0xD0, 0xB3, 0xD0, 0xBE,
        0xD0, 0xB4, 0x2C, 0x20, 0xD0, 0xB2, 0xD1, 0x8B, 0xD0, 0xB3, 0xD1, 0x80, 0xD1, 0x83, 0xD0,
        0xB7, 0xD0, 0xBA, 0xD0, 0xB0, 0x20, 0xD0, 0xBD, 0xD0, 0xBE, 0xD0, 0xBC, 0xD0, 0xB5, 0xD1,
        0x80, 0x20, 0x31, 0x34, 0x2E,
    ];

    /// Архив с дескрипторами данных: zipfile поверх потока без `tell`
    /// вынужден ставить бит 3 и писать в локальный заголовок нули, а CRC и
    /// размеры — после данных.
    ///
    /// ```python
    /// class NoSeek:
    ///     def __init__(self): self.buf = bytearray()
    ///     def write(self, data): self.buf.extend(data); return len(data)
    ///     def flush(self): pass
    /// sink = NoSeek()
    /// with zipfile.ZipFile(sink, 'w') as zf:
    ///     zf.writestr(info('поток.txt', zipfile.ZIP_DEFLATED), text)
    ///     zf.writestr(info('хвост.bin', zipfile.ZIP_STORED), noise)
    /// print(list(sink.buf))
    /// ```
    const REF_DESCRIPTOR: [u8; 373] = [
        0x50, 0x4B, 0x03, 0x04, 0x14, 0x00, 0x08, 0x08, 0x08, 0x00, 0x00, 0x00, 0x21, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0E, 0x00, 0x00, 0x00,
        0xD0, 0xBF, 0xD0, 0xBE, 0xD1, 0x82, 0xD0, 0xBE, 0xD0, 0xBA, 0x2E, 0x74, 0x78, 0x74, 0x65,
        0xCE, 0xBD, 0x09, 0x80, 0x30, 0x14, 0x45, 0xE1, 0x3E, 0xBB, 0x08, 0xC9, 0xCB, 0x9F, 0xF2,
        0x96, 0x73, 0x00, 0x3B, 0x37, 0xB0, 0xB0, 0x0E, 0x41, 0xB1, 0x32, 0x33, 0xDC, 0x6C, 0x64,
        0x9D, 0x6B, 0x7B, 0xE0, 0x83, 0x83, 0x03, 0x0D, 0x15, 0xA5, 0xAF, 0x8A, 0x13, 0x37, 0x5E,
        0x14, 0x83, 0x1D, 0x15, 0x0D, 0x0F, 0xAE, 0xBE, 0x4D, 0x56, 0xED, 0x18, 0x9C, 0xE6, 0x31,
        0x88, 0xBA, 0xC0, 0x46, 0x1C, 0x23, 0x99, 0x59, 0xF9, 0xC8, 0x2A, 0x08, 0xAB, 0xB0, 0xB0,
        0x8A, 0x89, 0x55, 0xF2, 0xBF, 0x41, 0x5A, 0x16, 0xCD, 0xD9, 0x7C, 0x50, 0x4B, 0x07, 0x08,
        0x19, 0x1C, 0xAD, 0xDB, 0x57, 0x00, 0x00, 0x00, 0xEA, 0x00, 0x00, 0x00, 0x50, 0x4B, 0x03,
        0x04, 0x14, 0x00, 0x08, 0x08, 0x00, 0x00, 0x00, 0x00, 0x21, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0E, 0x00, 0x00, 0x00, 0xD1, 0x85, 0xD0,
        0xB2, 0xD0, 0xBE, 0xD1, 0x81, 0xD1, 0x82, 0x2E, 0x62, 0x69, 0x6E, 0x0D, 0x6E, 0xCF, 0x30,
        0x91, 0xF2, 0x53, 0xB4, 0x15, 0x76, 0xD7, 0x38, 0x99, 0xFA, 0x5B, 0xBC, 0x1D, 0x7E, 0xDF,
        0x40, 0xA1, 0x02, 0x63, 0xC4, 0x50, 0x4B, 0x07, 0x08, 0x88, 0x04, 0x12, 0x95, 0x18, 0x00,
        0x00, 0x00, 0x18, 0x00, 0x00, 0x00, 0x50, 0x4B, 0x01, 0x02, 0x14, 0x03, 0x14, 0x00, 0x08,
        0x08, 0x08, 0x00, 0x00, 0x00, 0x21, 0x00, 0x19, 0x1C, 0xAD, 0xDB, 0x57, 0x00, 0x00, 0x00,
        0xEA, 0x00, 0x00, 0x00, 0x0E, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x80, 0x01, 0x00, 0x00, 0x00, 0x00, 0xD0, 0xBF, 0xD0, 0xBE, 0xD1, 0x82, 0xD0, 0xBE,
        0xD0, 0xBA, 0x2E, 0x74, 0x78, 0x74, 0x50, 0x4B, 0x01, 0x02, 0x14, 0x03, 0x14, 0x00, 0x08,
        0x08, 0x00, 0x00, 0x00, 0x00, 0x21, 0x00, 0x88, 0x04, 0x12, 0x95, 0x18, 0x00, 0x00, 0x00,
        0x18, 0x00, 0x00, 0x00, 0x0E, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x80, 0x01, 0x93, 0x00, 0x00, 0x00, 0xD1, 0x85, 0xD0, 0xB2, 0xD0, 0xBE, 0xD1, 0x81,
        0xD1, 0x82, 0x2E, 0x62, 0x69, 0x6E, 0x50, 0x4B, 0x05, 0x06, 0x00, 0x00, 0x00, 0x00, 0x02,
        0x00, 0x02, 0x00, 0x78, 0x00, 0x00, 0x00, 0xE7, 0x00, 0x00, 0x00, 0x00, 0x00,
    ];

    /// Архив, у которого поле Zip64 стоит в ЛОКАЛЬНОМ заголовке: `force_zip64`
    /// заставляет zipfile написать его независимо от размера. В каталоге
    /// маленькой записи поля Zip64 при этом нет — длины extra у двух
    /// заголовков одной записи законно разные.
    ///
    /// ```python
    /// buf = io.BytesIO()
    /// with zipfile.ZipFile(buf, 'w') as zf:
    ///     with zf.open(info('большой.bin', zipfile.ZIP_STORED), 'w',
    ///                  force_zip64=True) as f:
    ///         f.write(noise)
    /// print(list(buf.getvalue()))
    /// ```
    const REF_ZIP64: [u8; 178] = [
        0x50, 0x4B, 0x03, 0x04, 0x2D, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x00, 0x21, 0x00, 0x88,
        0x04, 0x12, 0x95, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x12, 0x00, 0x14, 0x00,
        0xD0, 0xB1, 0xD0, 0xBE, 0xD0, 0xBB, 0xD1, 0x8C, 0xD1, 0x88, 0xD0, 0xBE, 0xD0, 0xB9, 0x2E,
        0x62, 0x69, 0x6E, 0x01, 0x00, 0x10, 0x00, 0x18, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x18, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0D, 0x6E, 0xCF, 0x30, 0x91, 0xF2, 0x53,
        0xB4, 0x15, 0x76, 0xD7, 0x38, 0x99, 0xFA, 0x5B, 0xBC, 0x1D, 0x7E, 0xDF, 0x40, 0xA1, 0x02,
        0x63, 0xC4, 0x50, 0x4B, 0x01, 0x02, 0x2D, 0x03, 0x2D, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00,
        0x00, 0x21, 0x00, 0x88, 0x04, 0x12, 0x95, 0x18, 0x00, 0x00, 0x00, 0x18, 0x00, 0x00, 0x00,
        0x12, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80, 0x01, 0x00,
        0x00, 0x00, 0x00, 0xD0, 0xB1, 0xD0, 0xBE, 0xD0, 0xBB, 0xD1, 0x8C, 0xD1, 0x88, 0xD0, 0xBE,
        0xD0, 0xB9, 0x2E, 0x62, 0x69, 0x6E, 0x50, 0x4B, 0x05, 0x06, 0x00, 0x00, 0x00, 0x00, 0x01,
        0x00, 0x01, 0x00, 0x40, 0x00, 0x00, 0x00, 0x5C, 0x00, 0x00, 0x00, 0x00, 0x00,
    ];

    /// Zip64 в КАТАЛОГЕ: запись конца каталога Zip64, локатор и поля 0x0001 в
    /// записях. Собран руками по спецификации — zipfile выносит размеры в
    /// extra каталога только для файлов больше четырёх гигабайт, а такой
    /// эталон в исходник не вошьёшь. Значения в поле 0x0001 идут в
    /// фиксированном порядке и присутствуют только те, что в самой записи
    /// выставлены в максимум, поэтому четыре записи сделаны разными: у
    /// `a64.txt` в поле только размеры, у `b64.txt` размеры и смещение, у
    /// `c64.txt` размеры, смещение и номер диска, а у `d64.txt` — ОДНО
    /// смещение при настоящих размерах в самой записи (обычный случай
    /// большого архива с мелкими файлами; полезная часть поля — ровно восемь
    /// байт, и разбор по длине блока принял бы смещение за несжатый размер).
    /// Правильность сборки подтверждена тем, что этот же архив читает
    /// zipfile и на нём проходит `testzip()`.
    ///
    /// ```python
    /// names = ['a64.txt', 'b64.txt', 'c64.txt', 'd64.txt']
    /// bodies = [b'A', b'BB', b'CCC', b'DDDD']
    /// # что вынесено в поле 0x0001: у четвёртой записи только смещение
    /// spill = [('size', 'comp'), ('size', 'comp', 'off'),
    ///          ('size', 'comp', 'off', 'disk'), ('off',)]
    /// out, offsets = bytearray(), []
    /// for name, body in zip(names, bodies):
    ///     offsets.append(len(out))
    ///     out += struct.pack('<IHHHHHIIIHH', 0x04034B50, 45, 0, 0, 0, 0x21,
    ///                        zlib.crc32(body), len(body), len(body), len(name), 0)
    ///     out += name.encode() + body
    /// cd_offset = len(out)
    /// for index, (name, body) in enumerate(zip(names, bodies)):
    ///     s = spill[index]
    ///     payload = b''
    ///     if 'size' in s: payload += struct.pack('<Q', len(body))
    ///     if 'comp' in s: payload += struct.pack('<Q', len(body))
    ///     if 'off' in s: payload += struct.pack('<Q', offsets[index])
    ///     if 'disk' in s: payload += struct.pack('<I', 0)
    ///     extra = struct.pack('<HH', 1, len(payload)) + payload
    ///     out += struct.pack('<IHHHHHHIIIHHHHHII', 0x02014B50, 45, 45, 0, 0, 0,
    ///                        0x21, zlib.crc32(body),
    ///                        0xFFFFFFFF if 'comp' in s else len(body),
    ///                        0xFFFFFFFF if 'size' in s else len(body),
    ///                        len(name), len(extra), 0,
    ///                        0xFFFF if 'disk' in s else 0, 0, 0,
    ///                        0xFFFFFFFF if 'off' in s else offsets[index])
    ///     out += name.encode() + extra
    /// cd_size, eocd64_at = len(out) - cd_offset, len(out)
    /// out += struct.pack('<IQHHIIQQQQ', 0x06064B50, 44, 45, 45, 0, 0,
    ///                    len(names), len(names), cd_size, cd_offset)
    /// out += struct.pack('<IIQI', 0x07064B50, 0, eocd64_at, 1)
    /// out += struct.pack('<IHHHHIIH', 0x06054B50, 0, 0, 0xFFFF, 0xFFFF,
    ///                    0xFFFFFFFF, 0xFFFFFFFF, 0)
    /// print(list(out))
    /// ```
    const REF_ZIP64_CD: [u8; 560] = [
        0x50, 0x4B, 0x03, 0x04, 0x2D, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x21, 0x00, 0x8B,
        0x9E, 0xD9, 0xD3, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x07, 0x00, 0x00, 0x00,
        0x61, 0x36, 0x34, 0x2E, 0x74, 0x78, 0x74, 0x41, 0x50, 0x4B, 0x03, 0x04, 0x2D, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x21, 0x00, 0xC4, 0x1F, 0x44, 0x1B, 0x02, 0x00, 0x00, 0x00,
        0x02, 0x00, 0x00, 0x00, 0x07, 0x00, 0x00, 0x00, 0x62, 0x36, 0x34, 0x2E, 0x74, 0x78, 0x74,
        0x42, 0x42, 0x50, 0x4B, 0x03, 0x04, 0x2D, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x21,
        0x00, 0x67, 0xE6, 0x1C, 0xB9, 0x03, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x07, 0x00,
        0x00, 0x00, 0x63, 0x36, 0x34, 0x2E, 0x74, 0x78, 0x74, 0x43, 0x43, 0x43, 0x50, 0x4B, 0x03,
        0x04, 0x2D, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x21, 0x00, 0xE2, 0x3A, 0x05, 0xA7,
        0x04, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x07, 0x00, 0x00, 0x00, 0x64, 0x36, 0x34,
        0x2E, 0x74, 0x78, 0x74, 0x44, 0x44, 0x44, 0x44, 0x50, 0x4B, 0x01, 0x02, 0x2D, 0x00, 0x2D,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x21, 0x00, 0x8B, 0x9E, 0xD9, 0xD3, 0xFF, 0xFF,
        0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x07, 0x00, 0x14, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x61, 0x36, 0x34, 0x2E, 0x74, 0x78,
        0x74, 0x01, 0x00, 0x10, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x50, 0x4B, 0x01, 0x02, 0x2D, 0x00, 0x2D, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x21, 0x00, 0xC4, 0x1F, 0x44, 0x1B, 0xFF, 0xFF, 0xFF, 0xFF,
        0xFF, 0xFF, 0xFF, 0xFF, 0x07, 0x00, 0x1C, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0x62, 0x36, 0x34, 0x2E, 0x74, 0x78, 0x74, 0x01,
        0x00, 0x18, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x26, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x50, 0x4B, 0x01,
        0x02, 0x2D, 0x00, 0x2D, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x21, 0x00, 0x67, 0xE6,
        0x1C, 0xB9, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x07, 0x00, 0x20, 0x00, 0x00,
        0x00, 0xFF, 0xFF, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0x63, 0x36,
        0x34, 0x2E, 0x74, 0x78, 0x74, 0x01, 0x00, 0x1C, 0x00, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x4D, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x50, 0x4B, 0x01, 0x02, 0x2D, 0x00, 0x2D, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x21, 0x00, 0xE2, 0x3A, 0x05, 0xA7, 0x04, 0x00, 0x00,
        0x00, 0x04, 0x00, 0x00, 0x00, 0x07, 0x00, 0x0C, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0x64, 0x36, 0x34, 0x2E, 0x74, 0x78, 0x74,
        0x01, 0x00, 0x08, 0x00, 0x75, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x50, 0x4B, 0x06,
        0x06, 0x2C, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x2D, 0x00, 0x2D, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x30, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x9E, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x50, 0x4B, 0x06, 0x07, 0x00, 0x00, 0x00,
        0x00, 0xCE, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x50, 0x4B,
        0x05, 0x06, 0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
        0xFF, 0xFF, 0xFF, 0x00, 0x00,
    ];

    /// Пустой архив: одна запись конца каталога и ничего больше.
    ///
    /// ```python
    /// buf = io.BytesIO()
    /// with zipfile.ZipFile(buf, 'w'):
    ///     pass
    /// print(list(buf.getvalue()))
    /// ```
    const REF_EMPTY: [u8; 22] = [
        0x50, 0x4B, 0x05, 0x06, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    ];

    /// Смещение номера диска в поле 0x0001 третьей записи [`REF_ZIP64_CD`] —
    /// единственное значение эталона, которое не выводится из соседних
    /// байтов поиском, поэтому записано числом и проверяется тестом.
    const ZIP64_CD_DISK_AT: usize = 393;

    /// Текст, который лежит в эталонах, — тот же генератор, что в командах
    /// происхождения.
    fn reference_text() -> Vec<u8> {
        let mut text = String::from("Товар;Цена\n");
        for i in 0..12 {
            text.push_str(&format!("Гвоздь-{};{}\n", i % 3, i * 7));
        }
        text.into_bytes()
    }

    /// Несжимаемые 24 байта эталонов.
    fn reference_noise() -> Vec<u8> {
        (0..24u32).map(|i| ((i * 97 + 13) & 0xFF) as u8).collect()
    }

    /// Смещение записи каталога с данным именем. Пробы правят эталоны, и
    /// искать место правки поиском надёжнее, чем считать смещения руками:
    /// перегенерация эталона не превратит тест в проверку соседних байт.
    fn central_at(data: &[u8], name: &[u8]) -> usize {
        let mut found = None;
        for at in 0..data.len().saturating_sub(CENTRAL_HEADER_LEN) {
            if data[at..at + 4] != SIG_CENTRAL.to_le_bytes() {
                continue;
            }
            let len = usize::from(u16::from_le_bytes([data[at + 28], data[at + 29]]));
            if data.get(at + CENTRAL_HEADER_LEN..at + CENTRAL_HEADER_LEN + len) == Some(name) {
                assert!(found.is_none(), "имя встретилось в каталоге дважды");
                found = Some(at);
            }
        }
        found.expect("запись каталога с таким именем не найдена")
    }

    /// Смещение локального заголовка записи, объявленное в её записи каталога.
    fn local_at(data: &[u8], central: usize) -> usize {
        u32::from_le_bytes([
            data[central + 42],
            data[central + 43],
            data[central + 44],
            data[central + 45],
        ]) as usize
    }

    /// Смещение записи конца каталога у эталона БЕЗ комментария — считается
    /// от конца файла, а не через [`find_eocd`], чтобы проба не проверяла
    /// поиск сама собой.
    fn eocd_at(data: &[u8]) -> usize {
        let at = data.len() - EOCD_LEN;
        assert_eq!(data[at..at + 4], SIG_EOCD.to_le_bytes());
        at
    }

    /// Выставить биты общих флагов в записи каталога и в локальном заголовке
    /// сразу — так их ставит настоящий архиватор.
    fn set_flags(data: &mut [u8], central: usize, set: u16, clear: u16) {
        let local = local_at(data, central);
        for at in [central + 8, local + 6] {
            let flags = (u16::from_le_bytes([data[at], data[at + 1]]) | set) & !clear;
            data[at..at + 2].copy_from_slice(&flags.to_le_bytes());
        }
    }

    // ---------------------------------------------------------------------
    // Круговой прогон и эталоны
    // ---------------------------------------------------------------------

    /// Собранное нашим писателем читается нашим же читателем: имена, способы,
    /// размеры и байты содержимого.
    #[test]
    fn an_archive_from_our_own_writer_reads_back() {
        let text = "<c r=\"A1\" t=\"s\"><v>0</v></c>".repeat(300);
        let noise = reference_noise();
        let mut z = ZipWriter::new();
        z.add("xl/sheet.xml", text.as_bytes());
        z.add(
            "документы/накладная.txt",
            "Организация «Ромашка»".as_bytes(),
        );
        z.add("шум.bin", &noise);
        z.add("папка/", b"");
        let bytes = z.finish();

        let archive = ZipArchive::parse(&bytes).expect("наш же архив обязан разобраться");
        let names: Vec<&str> = archive
            .entries()
            .iter()
            .map(|e| e.name().expect("бит 11 наш писатель ставит всегда"))
            .collect();
        assert_eq!(
            names,
            [
                "xl/sheet.xml",
                "документы/накладная.txt",
                "шум.bin",
                "папка/"
            ]
        );

        let entries = archive.entries();
        assert_eq!(
            entries[0].method(),
            METHOD_DEFLATED,
            "разметка обязана сжаться"
        );
        assert!(entries[0].compressed_size() < entries[0].size());
        assert_eq!(entries[2].method(), METHOD_STORED, "шум сжимать незачем");
        assert_eq!(entries[2].compressed_size(), noise.len() as u64);
        assert!(entries[3].is_directory(), "имя со слэшем — это каталог");
        assert!(!entries[0].is_directory());
        assert!(entries.iter().all(|e| !e.is_encrypted()));
        assert!(entries.iter().all(|e| !e.has_data_descriptor()));

        assert_eq!(archive.read(0).expect("метод 8"), text.as_bytes());
        assert_eq!(
            archive.read(1).expect("метод 8"),
            "Организация «Ромашка»".as_bytes()
        );
        assert_eq!(archive.read(2).expect("метод 0"), noise);
        assert_eq!(archive.read(3).expect("пустая запись"), Vec::<u8>::new());
        assert!(archive.read(4).is_err(), "записи с номером 4 нет");
    }

    #[test]
    fn the_reference_archive_with_stored_and_deflated_entries_reads() {
        let archive = ZipArchive::parse(&REF_MIXED).expect("эталон zipfile обязан разобраться");
        assert_eq!(archive.entries().len(), 2);

        let entry = &archive.entries()[0];
        assert_eq!(entry.name(), Some("накладная.txt"));
        assert_eq!(entry.method(), METHOD_DEFLATED);
        assert_eq!(entry.size(), reference_text().len() as u64);
        assert!(entry.compressed_size() < entry.size());
        assert_eq!(
            entry.crc(),
            crc32(&reference_text()),
            "каталожная сумма — это сумма распакованных данных"
        );
        assert!(!entry.is_encrypted());
        assert!(!entry.has_data_descriptor());
        assert_eq!(archive.read(0).expect("метод 8"), reference_text());

        let entry = &archive.entries()[1];
        assert_eq!(entry.name(), Some("шум.bin"));
        assert_eq!(entry.method(), METHOD_STORED);
        assert_eq!(entry.size(), reference_noise().len() as u64);
        assert_eq!(entry.compressed_size(), entry.size());
        assert_eq!(entry.crc(), crc32(&reference_noise()));
        assert_eq!(archive.read(1).expect("метод 0"), reference_noise());
    }

    /// Комментарий отодвигает запись конца каталога от конца файла, а
    /// подтверждается кандидат тем, что объявленная в нём длина комментария
    /// доводит ровно до конца: сигнатуры мало, те же четыре байта могут
    /// лежать и в самом комментарии.
    #[test]
    fn a_comment_does_not_hide_the_end_of_directory_record() {
        let archive = ZipArchive::parse(&REF_COMMENT).expect("эталон с комментарием разбирается");
        assert_eq!(archive.entries().len(), 1);
        assert_eq!(archive.entries()[0].name(), Some("отчёт.txt"));
        assert_eq!(archive.read(0).expect("метод 8"), reference_text());

        let mut data = REF_COMMENT.to_vec();
        let fake = data.len() - 30;
        data[fake..fake + 4].copy_from_slice(&SIG_EOCD.to_le_bytes());
        let last = (0..=data.len() - 4).rfind(|&at| data[at..at + 4] == SIG_EOCD.to_le_bytes());
        assert_eq!(
            last,
            Some(fake),
            "подложная сигнатура обязана быть последней в файле"
        );
        let archive = ZipArchive::parse(&data).expect("настоящая запись всё равно находится");
        assert_eq!(archive.read(0).expect("метод 8"), reference_text());
    }

    /// При бите 3 размеры и CRC в локальном заголовке нулевые, а настоящие
    /// лежат в каталоге. Читатель, берущий размеры из локального заголовка,
    /// прочитал бы обе записи пустыми.
    #[test]
    fn entries_with_a_data_descriptor_are_read_by_the_directory_sizes() {
        assert_eq!(
            REF_DESCRIPTOR[14..26],
            [0u8; 12],
            "в локальном заголовке эталона обязаны быть нули"
        );
        let archive = ZipArchive::parse(&REF_DESCRIPTOR).expect("эталон разбирается");
        assert_eq!(archive.entries().len(), 2);
        assert!(archive.entries().iter().all(|e| e.has_data_descriptor()));
        assert_eq!(archive.read(0).expect("метод 8"), reference_text());
        assert_eq!(archive.read(1).expect("метод 0"), reference_noise());
    }

    /// Длины имени и extra берутся из ЛОКАЛЬНОГО заголовка: здесь в нём есть
    /// поле Zip64, которого нет в каталоге, и читатель, пропускающий extra по
    /// каталожной длине, начал бы читать данные на двадцать байт раньше.
    #[test]
    fn a_zip64_local_header_is_skipped_by_its_own_extra_length() {
        let name_len = usize::from(u16::from_le_bytes([REF_ZIP64[26], REF_ZIP64[27]]));
        let extra_len = usize::from(u16::from_le_bytes([REF_ZIP64[28], REF_ZIP64[29]]));
        assert!(
            extra_len >= 20,
            "в локальном заголовке ожидается поле Zip64"
        );
        assert_eq!(
            u16::from_le_bytes([REF_ZIP64[30 + name_len], REF_ZIP64[31 + name_len]]),
            1,
            "и это должно быть поле с идентификатором 0x0001"
        );
        let central = central_at(&REF_ZIP64, "большой.bin".as_bytes());
        assert_eq!(
            u16::from_le_bytes([REF_ZIP64[central + 30], REF_ZIP64[central + 31]]),
            0,
            "а в каталоге extra нет вовсе"
        );

        let archive = ZipArchive::parse(&REF_ZIP64).expect("эталон Zip64 разбирается");
        let entry = &archive.entries()[0];
        assert_eq!(entry.name(), Some("большой.bin"));
        assert_eq!(entry.size(), reference_noise().len() as u64);
        assert_eq!(archive.read(0).expect("метод 0"), reference_noise());
    }

    /// Значения поля 0x0001 идут в фиксированном порядке, но присутствуют
    /// только те, что в записи выставлены в максимум. У четырёх записей
    /// эталона набор разный, поэтому разбор по фиксированному размеру блока
    /// здесь заведомо разъезжается: у второй, третьей и четвёртой записи
    /// смещение локального заголовка тоже лежит в поле, и без него запись
    /// просто не найти, а у четвёртой оно в поле ОДНО — при настоящих
    /// размерах в самой записи, так что позиционный читатель принял бы это
    /// смещение за несжатый размер.
    #[test]
    fn zip64_values_in_the_directory_are_taken_in_order_and_only_when_maxed() {
        let archive = ZipArchive::parse(&REF_ZIP64_CD).expect("эталон Zip64 разбирается");
        let bodies: [&[u8]; 4] = [b"A", b"BB", b"CCC", b"DDDD"];
        // Смещения локальных заголовков в эталоне: 30 байт заголовка, семь
        // байт имени и тело. У второй, третьей и четвёртой записи в самом
        // каталоге на этом месте стоит 0xFFFFFFFF, так что взяться им
        // неоткуда, кроме поля 0x0001.
        let offsets = [0u64, 38, 77, 117];
        assert_eq!(archive.entries().len(), 4);
        for (index, body) in bodies.iter().enumerate() {
            let entry = &archive.entries()[index];
            assert_eq!(entry.name(), None, "бит 11 в эталоне не выставлен");
            assert_eq!(entry.name_bytes()[1..], b"64.txt"[..]);
            assert_eq!(entry.size(), body.len() as u64);
            assert_eq!(entry.compressed_size(), body.len() as u64);
            assert_eq!(entry.local_offset(), offsets[index]);
            assert_eq!(&archive.read(index).expect("метод 0"), body);
        }

        // У четвёртой записи размеры в каталоге настоящие, а в максимум
        // выставлено только смещение: полезная часть поля 0x0001 — ровно
        // восемь байт, и они обязаны стать смещением, а не размером.
        let central = central_at(&REF_ZIP64_CD, b"d64.txt");
        assert_eq!(
            u32::from_le_bytes([
                REF_ZIP64_CD[central + 20],
                REF_ZIP64_CD[central + 21],
                REF_ZIP64_CD[central + 22],
                REF_ZIP64_CD[central + 23],
            ]),
            4,
            "сжатый размер четвёртой записи в каталоге настоящий"
        );
        assert_eq!(
            u32::from_le_bytes([
                REF_ZIP64_CD[central + 24],
                REF_ZIP64_CD[central + 25],
                REF_ZIP64_CD[central + 26],
                REF_ZIP64_CD[central + 27],
            ]),
            4,
            "и несжатый тоже"
        );
        assert_eq!(
            u32::from_le_bytes([
                REF_ZIP64_CD[central + 42],
                REF_ZIP64_CD[central + 43],
                REF_ZIP64_CD[central + 44],
                REF_ZIP64_CD[central + 45],
            ]),
            MAX_U32,
            "а вот смещение вынесено в поле 0x0001"
        );
        let name_len = usize::from(u16::from_le_bytes([
            REF_ZIP64_CD[central + 28],
            REF_ZIP64_CD[central + 29],
        ]));
        let extra_at = central + CENTRAL_HEADER_LEN + name_len;
        assert_eq!(
            u16::from_le_bytes([REF_ZIP64_CD[extra_at], REF_ZIP64_CD[extra_at + 1]]),
            1,
            "здесь ожидается поле 0x0001"
        );
        assert_eq!(
            u16::from_le_bytes([REF_ZIP64_CD[extra_at + 2], REF_ZIP64_CD[extra_at + 3]]),
            8,
            "и в нём ровно одно восьмибайтовое значение"
        );
    }

    /// Номер диска, вынесенный в поле 0x0001, читается по тому же правилу —
    /// и ненулевой означает многотомный архив.
    #[test]
    fn a_zip64_entry_on_another_disk_is_rejected() {
        let mut data = REF_ZIP64_CD.to_vec();
        assert_eq!(
            u32::from_le_bytes([
                data[ZIP64_CD_DISK_AT],
                data[ZIP64_CD_DISK_AT + 1],
                data[ZIP64_CD_DISK_AT + 2],
                data[ZIP64_CD_DISK_AT + 3],
            ]),
            0,
            "здесь ожидается номер диска из поля Zip64"
        );
        data[ZIP64_CD_DISK_AT..ZIP64_CD_DISK_AT + 4].copy_from_slice(&1u32.to_le_bytes());
        let e = ZipArchive::parse(&data).expect_err("запись объявлена на другом томе");
        assert!(e.to_string().contains("многотомн"), "непонятный текст: {e}");
    }

    #[test]
    fn an_empty_archive_has_no_entries() {
        let archive = ZipArchive::parse(&REF_EMPTY).expect("пустой эталон разбирается");
        assert!(archive.entries().is_empty());
        let bytes = ZipWriter::new().finish();
        let archive = ZipArchive::parse(&bytes).expect("наш пустой архив разбирается");
        assert!(archive.entries().is_empty());
    }

    // ---------------------------------------------------------------------
    // Порча, обрезка и неподдерживаемое
    // ---------------------------------------------------------------------

    #[test]
    fn input_that_is_not_an_archive_is_rejected() {
        let junk: [&[u8]; 5] = [
            b"",
            b"PK",
            b"PK\x05\x06",
            &[0u8; 100],
            "Организация «Ромашка» — не архив".as_bytes(),
        ];
        for input in junk {
            let e = ZipArchive::parse(input).expect_err("это не архив");
            assert!(
                e.to_string().contains("не архив ZIP"),
                "непонятный текст: {e}"
            );
        }
    }

    /// Обрезка на каждой длине префикса: ни паники, ни зависания, а если
    /// обрезанное всё-таки разобралось — прочитанное обязано совпадать с
    /// прочитанным из целого архива, а не быть тихо другим.
    #[test]
    fn every_truncation_of_a_reference_archive_is_an_error_or_the_same_data() {
        for bytes in [&REF_MIXED[..], &REF_DESCRIPTOR[..], &REF_ZIP64_CD[..]] {
            let whole = ZipArchive::parse(bytes).expect("целый эталон разбирается");
            let names: Vec<Vec<u8>> = whole
                .entries()
                .iter()
                .map(|e| e.name_bytes().to_vec())
                .collect();
            let full: Vec<Vec<u8>> = (0..whole.entries().len())
                .map(|i| whole.read(i).expect("целый эталон читается"))
                .collect();

            for cut in 0..bytes.len() {
                let Ok(part) = ZipArchive::parse(&bytes[..cut]) else {
                    continue;
                };
                for (index, entry) in part.entries().iter().enumerate() {
                    let Ok(out) = part.read(index) else {
                        continue;
                    };
                    assert_eq!(
                        names.get(index).map(Vec::as_slice),
                        Some(entry.name_bytes()),
                        "обрезка {cut} дала другую запись"
                    );
                    assert_eq!(out, full[index], "обрезка {cut} дала другие данные");
                }
            }
        }
    }

    /// Порча одним битом в любом месте: интересна не конкретная ошибка, а то,
    /// что читатель всегда возвращает управление.
    #[test]
    fn a_single_bit_flip_anywhere_never_panics() {
        for byte in 0..REF_MIXED.len() {
            for bit in 0..8 {
                let mut data = REF_MIXED;
                data[byte] ^= 1 << bit;
                if let Ok(archive) = ZipArchive::parse(&data) {
                    for index in 0..archive.entries().len() {
                        if let Ok(out) = archive.read(index) {
                            assert_eq!(
                                out.len() as u64,
                                archive.entries()[index].size(),
                                "прочитано не столько, сколько объявлено"
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn a_bit_flip_in_the_directory_crc_is_reported() {
        let mut data = REF_MIXED.to_vec();
        let central = central_at(&data, "накладная.txt".as_bytes());
        data[central + 16] ^= 1;
        let archive = ZipArchive::parse(&data).expect("каталог при этом цел");
        let e = archive.read(0).expect_err("контрольная сумма не совпадёт");
        assert!(e.to_string().contains("CRC"), "непонятный текст: {e}");
        assert!(
            e.to_string().contains("накладная.txt"),
            "в ошибке должно быть имя записи: {e}"
        );
        assert_eq!(
            archive.read(1).expect("соседняя запись цела"),
            reference_noise()
        );
    }

    #[test]
    fn a_bit_flip_inside_the_compressed_data_is_reported() {
        let mut data = REF_MIXED.to_vec();
        let central = central_at(&data, "накладная.txt".as_bytes());
        let local = local_at(&data, central);
        let name_len = usize::from(u16::from_le_bytes([data[local + 26], data[local + 27]]));
        let at = local + LOCAL_HEADER_LEN + name_len + 5;
        data[at] ^= 0x40;
        let archive = ZipArchive::parse(&data).expect("каталог при этом цел");
        let e = archive.read(0).expect_err("данные испорчены");
        let text = e.to_string();
        assert!(
            text.contains("CRC") || text.contains("deflate") || text.contains("размер"),
            "непонятный текст: {text}"
        );
    }

    #[test]
    fn a_local_header_that_is_not_where_the_directory_says_is_reported() {
        let mut data = REF_MIXED.to_vec();
        let central = central_at(&data, "шум.bin".as_bytes());
        let local = local_at(&data, central);
        data[local + 3] ^= 0xFF;
        let archive = ZipArchive::parse(&data).expect("каталог при этом цел");
        let e = archive
            .read(1)
            .expect_err("сигнатуры локального заголовка нет");
        assert!(
            e.to_string().contains("локального заголовка"),
            "непонятный текст: {e}"
        );
        assert_eq!(
            archive.read(0).expect("соседняя запись цела"),
            reference_text()
        );
    }

    #[test]
    fn a_compressed_size_beyond_the_end_of_the_file_is_reported() {
        let mut data = REF_MIXED.to_vec();
        let central = central_at(&data, "накладная.txt".as_bytes());
        let len = data.len() as u32;
        data[central + 20..central + 24].copy_from_slice(&len.to_le_bytes());
        let archive = ZipArchive::parse(&data).expect("каталог при этом цел");
        let e = archive.read(0).expect_err("данные не помещаются в файл");
        assert!(e.to_string().contains("обрезан"), "непонятный текст: {e}");
    }

    #[test]
    fn an_unsupported_method_is_reported_with_its_number() {
        let mut data = REF_MIXED.to_vec();
        let central = central_at(&data, "накладная.txt".as_bytes());
        data[central + 10..central + 12].copy_from_slice(&9u16.to_le_bytes());
        let archive = ZipArchive::parse(&data).expect("каталог при этом цел");
        assert_eq!(archive.entries()[0].method(), 9);
        let e = archive.read(0).expect_err("deflate64 читатель не умеет");
        assert!(
            e.to_string().contains("способ хранения 9"),
            "номер способа обязан быть в тексте: {e}"
        );
    }

    /// При способе 0 сжатый размер обязан равняться несжатому: расхождение —
    /// порча каталога, а не повод прочитать что-нибудь.
    #[test]
    fn a_stored_entry_whose_sizes_disagree_is_reported() {
        let mut data = REF_MIXED.to_vec();
        let central = central_at(&data, "шум.bin".as_bytes());
        let bigger = reference_noise().len() as u32 + 1;
        data[central + 24..central + 28].copy_from_slice(&bigger.to_le_bytes());
        let archive = ZipArchive::parse(&data).expect("каталог при этом цел");
        let e = archive.read(1).expect_err("размеры не сходятся");
        assert!(
            e.to_string().contains("способ хранения 0"),
            "непонятный текст: {e}"
        );
    }

    /// Распакованное короче объявленного — ошибка, а не молчаливо короткие
    /// данные: CRC такую порчу не ловит, потому что распакован-то ровно тот
    /// поток, что лежит в архиве.
    #[test]
    fn a_deflated_entry_shorter_than_declared_is_reported() {
        let mut data = REF_MIXED.to_vec();
        let central = central_at(&data, "накладная.txt".as_bytes());
        let bigger = reference_text().len() as u32 + 100;
        data[central + 24..central + 28].copy_from_slice(&bigger.to_le_bytes());
        let archive = ZipArchive::parse(&data).expect("каталог при этом цел");
        let e = archive
            .read(0)
            .expect_err("распакуется меньше объявленного");
        assert!(
            e.to_string().contains("размер не совпал"),
            "непонятный текст: {e}"
        );
    }

    #[test]
    fn an_entry_count_that_disagrees_with_the_directory_is_reported() {
        let mut data = REF_MIXED.to_vec();
        let eocd = eocd_at(&data);
        data[eocd + 8..eocd + 10].copy_from_slice(&3u16.to_le_bytes());
        data[eocd + 10..eocd + 12].copy_from_slice(&3u16.to_le_bytes());
        let e = ZipArchive::parse(&data).expect_err("в каталоге две записи, а не три");
        assert!(
            e.to_string().contains("объявлено 3"),
            "непонятный текст: {e}"
        );
    }

    #[test]
    fn a_directory_size_that_disagrees_with_the_records_is_reported() {
        let eocd = eocd_at(&REF_MIXED);
        let size = u32::from_le_bytes([
            REF_MIXED[eocd + 12],
            REF_MIXED[eocd + 13],
            REF_MIXED[eocd + 14],
            REF_MIXED[eocd + 15],
        ]);
        // Отдельной сверки размера каталога нет: расхождение ловится границами
        // среза, из которого читается каталог, и текст отказа зависит от того,
        // с какой стороны разъехалось. Оба текста сняты с прогона.
        for (wrong, expected) in [
            (size - 1, "обрезан"),
            (size + 1, "обрезан"),
            // Больше остатка файла (349 − 203 = 146 байт) — каталог не
            // помещается в файл, и это видно ещё до разбора записей.
            (size + 40, "за границу"),
        ] {
            let mut data = REF_MIXED.to_vec();
            data[eocd + 12..eocd + 16].copy_from_slice(&wrong.to_le_bytes());
            let e = ZipArchive::parse(&data)
                .expect_err("объявленный размер каталога не сходится с записями");
            assert!(
                e.to_string().contains(expected),
                "размер {wrong} вместо {size}: непонятный текст: {e}"
            );
        }
    }

    #[test]
    fn an_archive_spanning_several_disks_is_rejected() {
        let mut data = REF_MIXED.to_vec();
        let eocd = eocd_at(&data);
        data[eocd + 4..eocd + 6].copy_from_slice(&1u16.to_le_bytes());
        let e = ZipArchive::parse(&data).expect_err("номер тома не нулевой");
        assert!(e.to_string().contains("многотомн"), "непонятный текст: {e}");

        let mut data = REF_MIXED.to_vec();
        let central = central_at(&data, "шум.bin".as_bytes());
        data[central + 34..central + 36].copy_from_slice(&1u16.to_le_bytes());
        let e = ZipArchive::parse(&data).expect_err("запись объявлена на другом томе");
        assert!(e.to_string().contains("многотомн"), "непонятный текст: {e}");
    }

    /// Локатор нашёлся, а записи конца каталога Zip64 по нему нет — это уже
    /// порча, а не архив без Zip64: молча вернуться к 32-битным полям здесь
    /// значило бы читать каталог по смещению 0xFFFFFFFF.
    #[test]
    fn a_zip64_locator_pointing_at_junk_is_reported() {
        let at = REF_ZIP64_CD.len() - EOCD_LEN - EOCD64_LOCATOR_LEN - EOCD64_LEN;
        assert_eq!(
            REF_ZIP64_CD[at..at + 4],
            SIG_EOCD64.to_le_bytes(),
            "здесь ожидается запись конца каталога Zip64"
        );

        let mut data = REF_ZIP64_CD.to_vec();
        data[at + 3] ^= 0xFF;
        let e = ZipArchive::parse(&data).expect_err("сигнатура EOCD64 испорчена");
        assert!(e.to_string().contains("Zip64"), "непонятный текст: {e}");

        let mut data = REF_ZIP64_CD.to_vec();
        let locator = data.len() - EOCD_LEN - EOCD64_LOCATOR_LEN;
        data[locator + 8..locator + 16].copy_from_slice(&u64::MAX.to_le_bytes());
        let e = ZipArchive::parse(&data).expect_err("локатор ведёт за границу файла");
        assert!(!e.to_string().is_empty(), "ошибка без текста");
    }

    /// Шифрование распознаётся и называется — до всякого чтения данных.
    /// Каталог при этом разбирается, и незашифрованные записи читаются.
    #[test]
    fn an_encrypted_entry_is_reported_before_its_data_are_touched() {
        let mut data = REF_MIXED.to_vec();
        let central = central_at(&data, "накладная.txt".as_bytes());
        set_flags(&mut data, central, FLAG_ENCRYPTED, 0);

        let archive = ZipArchive::parse(&data).expect("каталог зашифрованного архива разбирается");
        assert!(archive.entries()[0].is_encrypted());
        assert!(!archive.entries()[1].is_encrypted());
        let e = archive.read(0).expect_err("расшифровки здесь нет");
        assert!(
            e.to_string().contains("зашифрован"),
            "непонятный текст: {e}"
        );
        assert_eq!(
            archive.read(1).expect("вторая запись не зашифрована"),
            reference_noise()
        );
    }

    /// Имя без бита 11 остаётся сырыми байтами: какую однобайтовую кодовую
    /// страницу применяет платформа — вопрос поверхности BSL, и решать его
    /// догадкой здесь нечего.
    #[test]
    fn a_name_without_the_utf8_flag_stays_raw_bytes() {
        // «прайс1.txt» в CP866 — ровно десять байт, столько же, сколько
        // «шум.bin» в UTF-8, так что длины полей менять не приходится.
        const CP866: [u8; 10] = [0xAF, 0xE0, 0xA0, 0xA9, 0xE1, 0x31, 0x2E, 0x74, 0x78, 0x74];
        let mut data = REF_MIXED.to_vec();
        let central = central_at(&data, "шум.bin".as_bytes());
        let local = local_at(&data, central);
        assert_eq!("шум.bin".len(), CP866.len());
        data[central + CENTRAL_HEADER_LEN..central + CENTRAL_HEADER_LEN + CP866.len()]
            .copy_from_slice(&CP866);
        data[local + LOCAL_HEADER_LEN..local + LOCAL_HEADER_LEN + CP866.len()]
            .copy_from_slice(&CP866);
        set_flags(&mut data, central, 0, FLAG_UTF8_NAME);

        let archive = ZipArchive::parse(&data).expect("однобайтовое имя разбору не мешает");
        let entry = &archive.entries()[1];
        assert_eq!(
            entry.name(),
            None,
            "декодировать нечем — кодовая страница не объявлена"
        );
        assert_eq!(entry.name_bytes(), CP866);
        assert!(!entry.is_directory());
        assert_eq!(
            archive.read(1).expect("данные не тронуты"),
            reference_noise()
        );
    }

    /// Бит 11 стоит, а байты имени не UTF-8 — тоже сырые байты и никакой
    /// паники: доверять флагу чужого архива нельзя.
    #[test]
    fn a_broken_utf8_name_with_the_flag_set_stays_raw_bytes() {
        let mut data = REF_MIXED.to_vec();
        let central = central_at(&data, "шум.bin".as_bytes());
        data[central + CENTRAL_HEADER_LEN] = 0xFF;
        let archive = ZipArchive::parse(&data).expect("испорченное имя разбору не мешает");
        let entry = &archive.entries()[1];
        assert_eq!(entry.name(), None, "это не UTF-8");
        assert_eq!(entry.name_bytes()[0], 0xFF);
        assert_eq!(
            archive.read(1).expect("данные не тронуты"),
            reference_noise()
        );
    }
}
