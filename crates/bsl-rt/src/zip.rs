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
//!
//! Поверх читателя во второй половине файла лежит поверхность встроенного
//! языка — `ЧтениеZipФайла` и `ЧтениеФайлаАрхива` со своими коллекциями и
//! элементами ([`ArchiveState`]). Между ней и форматом проходит важная
//! граница: формат хранит имя записи БАЙТАМИ, а всё, что платформа делает с
//! именем дальше — декодирование как UTF-8 с заменяющими символами,
//! подстановка недопустимых в имени файла знаков, срез хвостовых точек и
//! пробелов, разрешение столкновений суффиксом `(N)` и разделение на пару
//! `Имя`/`ИсходноеИмя`, — измерено на 8.3.27 и живёт уже там.

use std::cell::RefCell;
use std::rc::Rc;

use crate::{BslObject, BslValue, RtError, RtResult};

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
/// Имя хранится СЫРЫМИ байтами, а декодированное отдаётся только когда его
/// объявил сам архив, битом 11 общих флагов. Платформа, как выяснилось
/// замером, поступает иначе: она декодирует имя КАК UTF-8 независимо от
/// бита 11, подставляя `U+FFFD` на негодных байтах (имя `привет.txt` в
/// CP866 8.3.27 показывает как `\u{FFFD}\u{A22}\u{FFFD}\u{FFFD}.txt` — это
/// ровно `String::from_utf8_lossy` от тех же байтов). Это решение
/// поверхности встроенного языка, а не формата, поэтому оно и живёт ниже,
/// в [`ArchiveState`], а здесь имя остаётся байтами.
///
/// Часть полей и разборщиков читают только юнит-тесты: поверхности BSL
/// хватает размеров, шифрования и даты, а способ хранения, CRC и смещение
/// она смотрит уже внутри [`read_entry`]. Проверять их снаружи всё равно
/// нужно — иначе разбор каталога держался бы на одном лишь чтении данных.
#[cfg_attr(not(test), allow(dead_code))]
#[derive(Debug)]
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
    /// Поля времени и даты MS-DOS как они лежат в каталоге. Разбираются
    /// не здесь, а в [`dos_datetime`]: правило нормализации у платформы
    /// своё и измерено отдельно.
    mod_time: u16,
    mod_date: u16,
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

    /// Время изменения записи так, как его показывает встроенный язык.
    pub(crate) fn modified(&self) -> crate::BslDate {
        dos_datetime(self.mod_time, self.mod_date)
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
/// Объекту встроенного языка срез не годится (он переживает и файл, и
/// поток, из которых прочитан), поэтому [`ArchiveState`] ниже забирает
/// записи себе через [`ZipArchive::into_parts`] и распаковывает уже
/// свободной функцией [`read_entry`].
pub(crate) struct ZipArchive<'a> {
    data: &'a [u8],
    entries: Vec<ZipEntry>,
    comment: Vec<u8>,
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

        // Комментарий архива лежит сразу за записью конца каталога, и его
        // длина уже проверена в `find_eocd`: кандидат считается настоящим
        // только тогда, когда она доводит ровно до конца файла.
        let comment_len = usize::from(u16_at(data, eocd + 20)?);
        let comment = slice_at(data, eocd + EOCD_LEN, comment_len)?.to_vec();

        Ok(ZipArchive {
            data,
            entries,
            comment,
        })
    }

    /// Записи в порядке центрального каталога.
    pub(crate) fn entries(&self) -> &[ZipEntry] {
        &self.entries
    }

    /// Отдать записи и комментарий наружу: объекту встроенного языка нужны
    /// собственные данные, а не заимствованные у среза.
    pub(crate) fn into_parts(self) -> (Vec<ZipEntry>, Vec<u8>) {
        (self.entries, self.comment)
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
        read_entry(self.data, entry)
    }
}

/// Прочитать и распаковать одну запись из байтов архива.
///
/// Тело [`ZipArchive::read`], вынесенное отдельно: объект встроенного языка
/// владеет байтами и записями порознь (см. [`ZipArchive::into_parts`]), и
/// собирать ради чтения временный [`ZipArchive`] ему незачем.
///
/// # Errors
///
/// [`RtError::Zip`], если данные зашифрованы, способ хранения не 0 и не 8,
/// локальный заголовок не на месте, данные выходят за границу файла,
/// распакованное короче объявленного или контрольная сумма не совпала с
/// каталожной.
pub(crate) fn read_entry(data: &[u8], entry: &ZipEntry) -> Result<Vec<u8>, RtError> {
    // Отказ до всякого чтения данных: расшифровки здесь нет, и делать
    // вид, что данные прочитаны, нельзя.
    if entry.encrypted {
        return Err(zip_err(&format!(
            "запись «{}» зашифрована, а зашифрованные архивы не поддерживаются",
            entry.name_for_message()
        )));
    }

    let header = to_usize(entry.local_offset)?;
    if u32_at(data, header)? != SIG_LOCAL {
        return Err(zip_err(&format!(
            "у записи «{}» нет локального заголовка по объявленному смещению",
            entry.name_for_message()
        )));
    }
    // Длины имени и extra берутся из локального заголовка: они законно
    // отличаются от каталожных (Zip64 и метки времени пишут в extra
    // по-разному в двух местах).
    let name_len = usize::from(u16_at(data, header + 26)?);
    let extra_len = usize::from(u16_at(data, header + 28)?);
    let at = header
        .checked_add(LOCAL_HEADER_LEN)
        .and_then(|v| v.checked_add(name_len))
        .and_then(|v| v.checked_add(extra_len))
        .ok_or_else(truncated)?;

    let size = to_usize(entry.size)?;
    let packed = slice_at(data, at, to_usize(entry.compressed_size)?)?;

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
    let mod_time = u16_at(region, at + 12)?;
    let mod_date = u16_at(region, at + 14)?;
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
        mod_time,
        mod_date,
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

// --------------------------------------------------------------------------
// Поверхность встроенного языка
// --------------------------------------------------------------------------

/// Который из двух платформенных читателей стоит за объектом.
///
/// На 8.3.27 их ДВА, и это измерено: `ЧтениеZipФайла` и `ЧтениеФайлаАрхива`
/// — разные типы (`Тип("ЧтениеZipФайла") = Тип("ЧтениеФайлаАрхива")` —
/// «Нет»), у каждого своя пара «коллекция + элемент», а поверхность у них с
/// точностью до имён одна: те же четыре метода, те же свойства элемента.
/// Отсюда один набор объектов с этим тегом вместо двух параллельных: тег
/// решает только имя типа и третий параметр конструктора, которого у
/// `ЧтениеZipФайла` нет вовсе (`Новый ЧтениеZipФайла(файл, пароль, тип)` —
/// «Конструктор не найден»).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchiveKind {
    /// `ЧтениеZipФайла` / `ЭлементыZipФайла` / `ЭлементZipФайла`.
    Zip,
    /// `ЧтениеФайлаАрхива` / `ЭлементыФайлаАрхива` / `ЭлементФайлаАрхива`.
    Archive,
}

/// Один элемент архива глазами встроенного языка.
///
/// Имена посчитаны один раз при открытии, а не при каждом обращении:
/// разрешение дублей зависит от УЖЕ разобранных соседей (см.
/// [`build_items`]), так что позаписной пересчёт дал бы другой ответ.
#[derive(Debug)]
pub struct ArchiveItem {
    entry: ZipEntry,
    /// `Имя` — короткое имя после подстановки и разрешения дублей.
    name: String,
    /// `Путь` — каталоги, со слэшем на конце; у записи в корне пусто.
    path: String,
    /// `ИсходноеИмя` — то же место в архиве, но без подстановки.
    orig_name: String,
    /// `ИсходныйПуть`.
    orig_path: String,
}

impl ArchiveItem {
    /// `ПолноеИмя` — измерено, что это ровно `Путь` + `Имя`, включая
    /// запись-каталог (`папка/` + `` = `папка/`).
    fn full_name(&self) -> String {
        format!("{}{}", self.path, self.name)
    }

    fn orig_full_name(&self) -> String {
        format!("{}{}", self.orig_path, self.orig_name)
    }
}

/// Открытый архив: собственные байты, комментарий и разобранные элементы.
#[derive(Debug)]
struct OpenArchive {
    data: Vec<u8>,
    /// Откуда открыт — только для текста ошибки «архив уже открыт».
    source: String,
    comment: String,
    items: Vec<ArchiveItem>,
}

/// Состояние объекта чтения. Пустое до `Открыть` и после `Закрыть`:
/// измерено, что на закрытом архиве и `Элементы`, и `Закрыть`, и
/// `ИзвлечьВсе` отвечают ошибкой «Архив не открыт!», а `Открыть` на уже
/// открытом — «Архив уже открыт!».
#[derive(Debug, Default)]
pub struct ArchiveState {
    open: Option<OpenArchive>,
    /// Номер текущего открытия, растёт на каждом успешном `Открыть`.
    ///
    /// Состояние переживает `Закрыть`/`Открыть` — это один и тот же `Rc`, —
    /// а состав архива при этом меняется целиком, так что номер записи,
    /// выданный до переоткрытия, к новому архиву не относится. Элемент
    /// запоминает номер открытия, при котором получен, и [`Self::item`]
    /// сверяет его с текущим: иначе `Извлечь` либо вылетала бы за границу
    /// более короткого каталога, либо — что хуже, потому что незаметно —
    /// молча распаковывала чужую запись, занявшую этот номер.
    generation: u64,
}

impl ArchiveState {
    fn opened(&self, op: &'static str) -> RtResult<&OpenArchive> {
        self.open
            .as_ref()
            .ok_or_else(|| zip_err(&format!("архив не открыт, «{op}» недоступно")))
    }

    /// Запись по номеру, выданному при открытии `generation`.
    ///
    /// Закрытый архив проверяется ПЕРВЫМ: на нём измеренный ответ — «архив
    /// не открыт», и устаревший элемент не должен его подменять.
    fn item(&self, index: usize, generation: u64, op: &'static str) -> RtResult<&ArchiveItem> {
        let open = self.opened(op)?;
        if generation != self.generation {
            return Err(zip_err(&format!(
                "элемент получен при другом открытии архива, «{op}» недоступно"
            )));
        }
        open.items
            .get(index)
            .ok_or_else(|| zip_err(&format!("в архиве нет записи с номером {index}")))
    }
}

/// Символы, недопустимые в имени файла: платформа заменяет каждый из них
/// подчёркиванием (ИЗМЕРЕНО поимённо на архиве с именами `a:b.txt`,
/// `a*b.txt`, `a?b.txt`, `a<b>c.txt`, `a|b.txt`, `a"b.txt` — вышли
/// `a_b.txt`, `a_b(1).txt`, `a_b(2).txt`, `a_b_c.txt`, `a_b(3).txt`,
/// `a_b(4).txt`). Управляющие символы в этот список НЕ входят: имя с
/// байтом `01`, `09`, `1F` или `7F` платформа оставляет как есть — это
/// проверено не по печати, а по именам файлов, которые она создала при
/// распаковке.
const FORBIDDEN_IN_NAME: [char; 7] = [':', '*', '?', '"', '<', '>', '|'];

/// Одна компонента пути после подстановки.
///
/// Хвостовые точки и пробелы платформа срезает (измерено: `dot.` -> `dot`,
/// `two..` -> `two`, `trail ` -> `trail`, `dir /f.txt` -> `dir/f.txt`), а
/// вот ведущий пробел и точка остаются (` lead.txt` и `.hidden` приходят
/// как есть). Побочное следствие среза — то, что компонента `..` целиком
/// превращается в пустую (измерено: `../up.txt` -> `/up.txt`), поэтому
/// выйти распаковкой вверх по дереву через имя записи нельзя.
fn sanitize_component(component: &str) -> String {
    let mut out: String = component
        .chars()
        .map(|c| {
            if FORBIDDEN_IN_NAME.contains(&c) {
                '_'
            } else {
                c
            }
        })
        .collect();
    while out.ends_with('.') || out.ends_with(' ') {
        out.pop();
    }
    out
}

/// Разбить имя на «без последнего расширения» и «расширение».
///
/// Измерено на 8.3.27: `вложенный.txt` -> (`вложенный`, `txt`), `a.b.c.txt`
/// -> (`a.b.c`, `txt`), `noext` -> (`noext`, ``), `.hidden` -> (``,
/// `hidden`) — то есть точка ищется ПОСЛЕДНЯЯ и ведущая точка расширением
/// не считается особым случаем.
fn split_extension(name: &str) -> (&str, &str) {
    match name.rfind('.') {
        Some(at) => (&name[..at], &name[at + 1..]),
        None => (name, ""),
    }
}

/// Свободное имя среди соседей: занятое дополняется `(N)` перед последней
/// точкой.
///
/// ИЗМЕРЕНО, что суффикс встаёт именно перед расширением и что нумерация
/// сквозная по всем столкновениям с одним и тем же именем: `same.txt`
/// дважды дают `same.txt` и `same(1).txt`, `noext` — `noext` и `noext(1)`,
/// `.hidden` — `.hidden` и `(1).hidden`, а шесть имён, схлопнувшихся в
/// `a_b.txt`, получают `(1)`..`(4)` в порядке каталога.
fn unique_among(used: &mut Vec<String>, base: String) -> String {
    if !used.contains(&base) {
        used.push(base.clone());
        return base;
    }
    // Точки в имени здесь уже нет только у имени без расширения: хвостовые
    // точки срезаны подстановкой, так что `stem` + `(N)` + `.ext` собирается
    // без оговорок.
    let (stem, ext) = split_extension(&base);
    for n in 1u32.. {
        let candidate = if ext.is_empty() {
            format!("{stem}({n})")
        } else {
            format!("{stem}({n}).{ext}")
        };
        if !used.contains(&candidate) {
            used.push(candidate.clone());
            return candidate;
        }
    }
    unreachable!("свободный номер находится всегда: имён конечное число")
}

/// Узел дерева каталогов, которое строится по именам записей.
struct DirNode {
    /// Подкаталоги по ИСХОДНОМУ имени компоненты. Именно по исходному:
    /// измерено, что `папка/` и `папка/вложенный.txt` дают ОДИН каталог,
    /// а две РАЗНЫЕ компоненты, схлопнувшиеся в одно имя (`..` и пустая),
    /// остаются разными каталогами и второй получает `(1)`.
    children: Vec<(String, usize)>,
    /// Занятые отображаемые имена среди детей этого узла.
    used: Vec<String>,
    /// Отображаемый путь до узла включительно, со слэшем на конце.
    display: String,
    /// Он же исходный.
    original: String,
}

/// Посчитать отображаемые имена всех записей архива.
///
/// Порядок обхода — каталожный, и это существенно: разрешение дублей
/// зависит от того, кто занял имя раньше.
fn build_items(entries: Vec<ZipEntry>) -> Vec<ArchiveItem> {
    let mut nodes = vec![DirNode {
        children: Vec::new(),
        used: Vec::new(),
        display: String::new(),
        original: String::new(),
    }];
    let mut items = Vec::with_capacity(entries.len());

    for entry in entries {
        // Обратный слэш платформа считает разделителем, а не знаком имени:
        // измерено, что `dir\back.txt` приходит как `dir/back.txt` ОБОИМИ
        // именами, и исходным тоже.
        let raw = String::from_utf8_lossy(entry.name_bytes()).replace('\\', "/");
        let is_dir = raw.ends_with('/');
        let trimmed = if is_dir {
            raw.strip_suffix('/').unwrap_or(&raw)
        } else {
            &raw
        };
        let mut parts: Vec<&str> = trimmed.split('/').collect();
        // У записи-каталога собственного короткого имени нет: измерено, что
        // у `папка/` `Имя` пустое, а `ПолноеИмя` и `Путь` — оба `папка/`.
        let leaf = if is_dir {
            ""
        } else {
            parts.pop().unwrap_or("")
        };

        let mut node = 0usize;
        for part in parts {
            node = resolve_dir(&mut nodes, node, part);
        }

        let (name, orig_name) = if is_dir {
            (String::new(), String::new())
        } else {
            let display = unique_among(&mut nodes[node].used, sanitize_component(leaf));
            (display, leaf.to_string())
        };
        items.push(ArchiveItem {
            entry,
            name,
            path: nodes[node].display.clone(),
            orig_name,
            orig_path: nodes[node].original.clone(),
        });
    }
    items
}

/// Найти или завести подкаталог `part` у узла `parent`.
fn resolve_dir(nodes: &mut Vec<DirNode>, parent: usize, part: &str) -> usize {
    if let Some((_, at)) = nodes[parent].children.iter().find(|(orig, _)| orig == part) {
        return *at;
    }
    let display = unique_among(&mut nodes[parent].used, sanitize_component(part));
    let node = DirNode {
        children: Vec::new(),
        used: Vec::new(),
        display: format!("{}{display}/", nodes[parent].display),
        original: format!("{}{part}/", nodes[parent].original),
    };
    nodes.push(node);
    let at = nodes.len() - 1;
    nodes[parent].children.push((part.to_string(), at));
    at
}

/// Время изменения записи из полей MS-DOS.
///
/// Правило ИЗМЕРЕНО по краям всех полей, и оно арифметическое, а не
/// зажимающее: компоненты нормализуются, как если бы их сложили. Месяц 0
/// уходит в декабрь прошлого года, месяц 13 — в январь следующего, день 0 —
/// в последний день предыдущего месяца, 30 февраля — во 2 марта, а час 25
/// добавляет сутки (2000-й год: месяц 13 -> `2001-01-01`, месяц 0 ->
/// `1999-12-01`, день 0 января -> `1999-12-31`, `2001-02-30` ->
/// `2001-03-02`, час 25 -> `2000-01-02 01:00`, минута 61 -> `01:01`,
/// поле секунд 31 -> `00:01:02`). Нулевые поля целиком дают `1979-11-30`.
fn dos_datetime(time: u16, date: u16) -> crate::BslDate {
    let year = 1980 + i64::from(date >> 9);
    let month = i64::from((date >> 5) & 0xF);
    let day = i64::from(date & 0x1F);
    let hour = i64::from(time >> 11);
    let minute = i64::from((time >> 5) & 0x3F);
    let second = i64::from(time & 0x1F) * 2;

    // Месяц нормализуется в паре с годом, остальное — простым сложением
    // секунд поверх первого числа этого месяца.
    let months = year * 12 + month - 1;
    let (y, m) = (months.div_euclid(12), months.rem_euclid(12) + 1);
    let Some(first) = crate::BslDate::from_civil(y, m as u32, 1, 0, 0, 0) else {
        return crate::BslDate::empty();
    };
    let shift = (day - 1) * 86_400 + hour * 3600 + minute * 60 + second;
    crate::BslDate::from_seconds(first.seconds() + shift).unwrap_or_else(crate::BslDate::empty)
}

// --- доступ к объектам --------------------------------------------------------

/// Состояние читателя за значением любого из трёх видов.
fn state<'a>(v: &'a BslValue, op: &'static str) -> RtResult<&'a Rc<RefCell<ArchiveState>>> {
    match v {
        BslValue::Object(o) => match &**o {
            BslObject::ArchiveReader(_, s)
            | BslObject::ArchiveEntries(_, s)
            | BslObject::ArchiveEntry(_, s, ..) => Ok(s),
            _ => Err(RtError::MethodNotApplicable {
                method: op,
                receiver: v.type_name(),
            }),
        },
        _ => Err(RtError::MethodNotApplicable {
            method: op,
            receiver: v.type_name(),
        }),
    }
}

/// Объект чтения ли это (`ЧтениеZipФайла` либо `ЧтениеФайлаАрхива`).
pub fn is_reader(v: &BslValue) -> bool {
    matches!(v, BslValue::Object(o) if matches!(&**o, BslObject::ArchiveReader(..)))
}

/// Коллекция элементов ли это.
pub fn is_entries(v: &BslValue) -> bool {
    matches!(v, BslValue::Object(o) if matches!(&**o, BslObject::ArchiveEntries(..)))
}

// --- конструкторы и методы -----------------------------------------------------

/// `Новый ЧтениеZipФайла([Источник][, Пароль])` и
/// `Новый ЧтениеФайлаАрхива([Источник][, Пароль][, ТипФайлаАрхива])`.
///
/// Источник — имя файла либо поток; `ДвоичныеДанные` платформа НЕ принимает
/// (измерено: «Несоответствие типов (параметр номер '1') (Некорректное имя
/// файла)»), и здесь их тоже нет. Без источника объект создаётся закрытым —
/// это законная форма (`Новый ЧтениеZipФайла()` с последующим `Открыть`).
///
/// Пароль ПРИНИМАЕТСЯ И НЕ ИСПОЛЬЗУЕТСЯ: расшифровки здесь нет вовсе, а до
/// первой попытки прочитать зашифрованную запись он не нужен и платформе
/// (измерено: на незашифрованном архиве `Новый ЧтениеZipФайла(файл,
/// "пусто")` проходит молча).
///
/// # Errors
///
/// [`RtError::Zip`], если источник не читается или не является архивом ZIP;
/// [`RtError::TypeError`], если третьим аргументом передан не член
/// `ТипФайлаАрхива`.
pub fn new_archive_reader(
    zip: bool,
    source: &BslValue,
    password: &BslValue,
    archive_type: &BslValue,
) -> RtResult<BslValue> {
    let kind = if zip {
        ArchiveKind::Zip
    } else {
        ArchiveKind::Archive
    };
    check_archive_type(archive_type)?;
    let state = Rc::new(RefCell::new(ArchiveState::default()));
    if !matches!(source, BslValue::Undefined) {
        let (bytes, from) = read_source(source, "ЧтениеZipФайла")?;
        open_bytes(&state, bytes, from)?;
    }
    // Пароль хранить негде и незачем — см. doc comment.
    let _ = password;
    Ok(BslValue::Object(Rc::new(BslObject::ArchiveReader(
        kind, state,
    ))))
}

/// Третий аргумент конструктора `ЧтениеФайлаАрхива`.
///
/// ИЗМЕРЕНО, что он типизирован: `Неопределено` платформа принимает, член
/// `ТипФайлаАрхива` тоже, а строку, число, булево и члена ЧУЖОГО
/// перечисления отвергает с «Несоответствие типов (параметр номер '3')».
/// Читаем мы только ZIP, поэтому всякий другой объявленный формат — честный
/// отказ, а не молчаливая попытка разобрать файл как ZIP.
fn check_archive_type(value: &BslValue) -> RtResult<()> {
    match value {
        BslValue::Undefined => Ok(()),
        BslValue::Enum(crate::EnumValue::ArchiveTypeZip) => Ok(()),
        BslValue::Enum(e) if e.kind() == crate::EnumKind::ArchiveFileType => Err(zip_err(
            &format!("формат архива «{}» не поддерживается", e.display_text()),
        )),
        _ => Err(RtError::TypeError {
            expected: "ТипФайлаАрхива",
            op: "ЧтениеФайлаАрхива",
        }),
    }
}

/// Байты источника вместе с его именем для сообщений.
fn read_source(source: &BslValue, op: &'static str) -> RtResult<(Vec<u8>, String)> {
    match source {
        BslValue::Str(s) => {
            let path = s.to_string();
            let bytes = std::fs::read(&path)
                .map_err(|e| zip_err(&format!("не удалось прочитать файл «{path}»: {e}")))?;
            Ok((bytes, path))
        }
        _ if crate::stream::is_stream(source) => {
            let bytes = crate::stream::read_all(source, op)?;
            Ok((bytes, "поток".to_string()))
        }
        _ => Err(RtError::TypeError {
            expected: "Строка или Поток",
            op,
        }),
    }
}

/// Разобрать байты и сделать их состоянием открытого архива.
fn open_bytes(state: &Rc<RefCell<ArchiveState>>, data: Vec<u8>, source: String) -> RtResult<()> {
    let (entries, comment) = ZipArchive::parse(&data)?.into_parts();
    let items = build_items(entries);
    // Комментарий декодируется так же, как имена, — lossy UTF-8 (измерено
    // на архиве с комментарием `привет-комментарий`).
    let comment = String::from_utf8_lossy(&comment).into_owned();
    let mut state = state.borrow_mut();
    state.open = Some(OpenArchive {
        data,
        source,
        comment,
        items,
    });
    // Номер открытия растёт только здесь и только после успешного разбора:
    // неудачное `Открыть` состав архива не меняет, а `Закрыть` его не
    // трогает — элементы закрытого архива и так отвергаются по `opened`.
    state.generation += 1;
    Ok(())
}

/// `Открыть(Источник[, Пароль])`.
///
/// # Errors
///
/// [`RtError::Zip`], если архив уже открыт или источник не является
/// читаемым архивом ZIP.
pub fn open(obj: &BslValue, args: &[BslValue]) -> RtResult<()> {
    let state = state(obj, "Открыть")?.clone();
    if let Some(open) = &state.borrow().open {
        return Err(zip_err(&format!("архив уже открыт: {}", open.source)));
    }
    let source = args.first().ok_or_else(|| RtError::MethodNotApplicable {
        method: "Открыть",
        receiver: obj.type_name(),
    })?;
    let (bytes, from) = read_source(source, "Открыть")?;
    open_bytes(&state, bytes, from)
}

/// `Закрыть()`.
///
/// # Errors
///
/// [`RtError::Zip`], если архив уже закрыт: измерено, что второй `Закрыть`
/// подряд платформа считает ошибкой, а не тихим повтором.
pub fn close(obj: &BslValue) -> RtResult<()> {
    let state = state(obj, "Закрыть")?;
    let mut state = state.borrow_mut();
    state.opened("Закрыть")?;
    state.open = None;
    Ok(())
}

/// Свойство `Элементы`.
///
/// # Errors
///
/// [`RtError::Zip`], если архив не открыт — измерено, что чтение свойства
/// на закрытом объекте это ошибка, а не пустая коллекция.
pub fn entries(obj: &BslValue) -> RtResult<BslValue> {
    let state = state(obj, "Элементы")?;
    state.borrow().opened("Элементы")?;
    let kind = reader_kind(obj)?;
    Ok(BslValue::Object(Rc::new(BslObject::ArchiveEntries(
        kind,
        state.clone(),
    ))))
}

/// Свойство `Комментарий` — комментарий всего архива.
///
/// # Errors
///
/// [`RtError::Zip`], если архив не открыт.
pub fn comment(obj: &BslValue) -> RtResult<BslValue> {
    let state = state(obj, "Комментарий")?;
    let state = state.borrow();
    Ok(BslValue::Str(crate::BslString::from_str(
        &state.opened("Комментарий")?.comment,
    )))
}

/// Тег объекта — он же решает имена типов коллекции и элемента.
fn reader_kind(obj: &BslValue) -> RtResult<ArchiveKind> {
    match obj {
        BslValue::Object(o) => match &**o {
            BslObject::ArchiveReader(kind, _)
            | BslObject::ArchiveEntries(kind, _)
            | BslObject::ArchiveEntry(kind, ..) => Ok(*kind),
            _ => Err(RtError::NotAnObject),
        },
        _ => Err(RtError::NotAnObject),
    }
}

/// Число элементов открытого архива.
///
/// # Errors
///
/// [`RtError::Zip`], если архив не открыт.
pub fn count(obj: &BslValue) -> RtResult<usize> {
    let state = state(obj, "Количество")?;
    let state = state.borrow();
    Ok(state.opened("Количество")?.items.len())
}

/// Элемент по номеру — общий путь `Коллекция[i]` и `Получить(i)`.
///
/// # Errors
///
/// [`RtError::Zip`], если архив не открыт; [`RtError::IndexOutOfBounds`],
/// если номера в архиве нет.
pub fn get(obj: &BslValue, index: usize) -> RtResult<BslValue> {
    let state = state(obj, "Получить")?;
    let (len, generation) = {
        let state = state.borrow();
        (state.opened("Получить")?.items.len(), state.generation)
    };
    if index >= len {
        return Err(RtError::IndexOutOfBounds {
            index: index as i64,
            len,
        });
    }
    Ok(BslValue::Object(Rc::new(BslObject::ArchiveEntry(
        reader_kind(obj)?,
        state.clone(),
        index,
        generation,
    ))))
}

/// `Найти(Имя)` — первый элемент с таким ИСХОДНЫМ коротким именем,
/// `Неопределено`, если такого нет.
///
/// Сравнение идёт с `ИсходноеИмя` и без учёта регистра — измерено:
/// `Найти("ШУМ.BIN")` находит `шум.bin`, `Найти("вложенный.txt")` находит
/// запись `папка/вложенный.txt` (то есть ищется короткое имя, а не полное),
/// `Найти("")` находит запись-каталог `папка/`, а `Найти("отчёт:2026.txt")`
/// находит запись с этим ИСХОДНЫМ именем, хотя её `Имя` после подстановки —
/// `отчёт_2026.txt`. Обратное не работает: `Найти("отчёт_2026.txt")`,
/// `Найти("дубль(1).txt")` и `Найти("папка/вложенный.txt")` дают
/// `Неопределено` — по отображаемому имени, по имени после разрешения
/// дублей и по полному имени платформа не ищет.
///
/// ОДНА измеренная точка сюда не укладывается, и правила за ней найти не
/// удалось: на архиве с записью `a:b.txt` платформа не находит её ни по
/// `a:b.txt`, ни по `a_b.txt`, тогда как `отчёт:2026.txt` по исходному
/// имени находится. Похоже на разбор `a:` как имени диска, но проверить
/// это нечем, и здесь работает общее правило — такая запись находится.
///
/// # Errors
///
/// [`RtError::Zip`], если архив не открыт.
pub fn find(obj: &BslValue, name: &BslValue) -> RtResult<BslValue> {
    let wanted = name.as_str("Найти")?.to_string().to_uppercase();
    let state = state(obj, "Найти")?;
    let (found, generation) = {
        let state = state.borrow();
        let open = state.opened("Найти")?;
        (
            open.items
                .iter()
                .position(|i| i.orig_name.to_uppercase() == wanted),
            state.generation,
        )
    };
    match found {
        Some(index) => Ok(BslValue::Object(Rc::new(BslObject::ArchiveEntry(
            reader_kind(obj)?,
            state.clone(),
            index,
            generation,
        )))),
        None => Ok(BslValue::Undefined),
    }
}

/// Свойства элемента архива.
///
/// Набор ИЗМЕРЕН перебором кандидатов: `Размер`, `МетодСжатия`,
/// `УровеньСжатия`, `Комментарий`, `ДатаМодификации`, `КонтрольнаяСумма`,
/// `CRC`, `Атрибуты`, `ЭтоКаталог` и `Индекс` платформа не знает вовсе.
/// Английского имени у `ВремяИзменения` нет: восемь правдоподобных
/// написаний (`ModificationTime`, `ModifiedAt`, `ModificationDate`,
/// `LastModified`, `DateModified`, `ModifiedTime`, `ChangeTime`,
/// `ModifiedDate`) 8.3.27 отвергает.
///
/// # Errors
///
/// [`RtError::UnknownColumn`], если такого свойства у элемента нет;
/// [`RtError::Zip`], если архив уже закрыт либо элемент получен при
/// предыдущем его открытии.
pub fn entry_prop(obj: &BslValue, prop: &str) -> RtResult<BslValue> {
    let (state, index, generation) = match obj {
        BslValue::Object(o) => match &**o {
            BslObject::ArchiveEntry(_, s, i, g) => (s, *i, *g),
            _ => return Err(RtError::NotAnObject),
        },
        _ => return Err(RtError::NotAnObject),
    };
    let state = state.borrow();
    let item = state.item(index, generation, "ЭлементZipФайла")?;

    let text = |s: String| Ok(BslValue::Str(crate::BslString::from_str(&s)));
    // Размеры записи приходят из чужого каталога: у Zip64 это произвольные
    // восемь байт, ничем не ограниченные (см. `read_zip64_extra`). Через
    // `i64` их пускать нельзя — старший бит завернулся бы в знак, и
    // `РазмерНесжатого` отдал бы отрицательное число, которого платформа
    // дать не может; `i128` вмещает любой `u64` без потерь.
    let number = |n: u64| {
        Ok(BslValue::Number(bsl_number::BslNumber::from_parts(
            i128::from(n),
            0,
        )))
    };
    match prop {
        _ if eq(prop, "Имя", "Name") => text(item.name.clone()),
        _ if eq(prop, "ПолноеИмя", "FullName") => text(item.full_name()),
        _ if eq(prop, "Путь", "Path") => text(item.path.clone()),
        _ if eq(prop, "ИмяБезРасширения", "BaseName") => {
            text(split_extension(&item.name).0.to_string())
        }
        _ if eq(prop, "Расширение", "Extension") => {
            text(split_extension(&item.name).1.to_string())
        }
        _ if eq(prop, "ИсходноеИмя", "OriginalName") => text(item.orig_name.clone()),
        _ if eq(prop, "ИсходноеПолноеИмя", "OriginalFullName") => {
            text(item.orig_full_name())
        }
        _ if eq(prop, "ИсходныйПуть", "OriginalPath") => text(item.orig_path.clone()),
        _ if eq(prop, "ИсходноеИмяБезРасширения", "OriginalBaseName") => {
            text(split_extension(&item.orig_name).0.to_string())
        }
        _ if eq(prop, "ИсходноеРасширение", "OriginalExtension") => {
            text(split_extension(&item.orig_name).1.to_string())
        }
        _ if eq(prop, "РазмерНесжатого", "UncompressedSize") => {
            number(item.entry.size())
        }
        _ if eq(prop, "РазмерСжатого", "CompressedSize") => {
            number(item.entry.compressed_size())
        }
        _ if eq(prop, "Зашифрован", "Encrypted") => {
            Ok(BslValue::Boolean(item.entry.is_encrypted()))
        }
        // Английского написания у этого свойства нет — измерено.
        _ if prop.eq_ignore_ascii_case("ВремяИзменения") => {
            Ok(BslValue::Date(item.entry.modified()))
        }
        _ => Err(RtError::UnknownColumn(prop.to_string())),
    }
}

/// Оба написания одного свойства.
fn eq(name: &str, ru: &str, en: &str) -> bool {
    name.eq_ignore_ascii_case(ru) || name.eq_ignore_ascii_case(en)
}

/// `Извлечь(Элемент, Каталог[, Режим][, Пароль])`.
///
/// Арность ИЗМЕРЕНА: два аргумента обязательны, пятый платформа отвергает,
/// а четвёртый — это пароль (на зашифрованном архиве, открытом без пароля,
/// `Извлечь(Э, Куда, Режим, "pass123")` распаковывает запись). Расшифровки
/// здесь нет, поэтому пароль принимается и не используется.
///
/// # Errors
///
/// [`RtError::Zip`] на закрытом архиве, на элементе чужого архива, на
/// элементе, полученном при другом открытии ЭТОГО архива, на зашифрованной
/// записи, на неподдержанном способе хранения и на любой ошибке записи
/// файла; [`RtError::TypeError`], если первым аргументом передан не элемент
/// архива, а вторым — не строка.
pub fn extract(obj: &BslValue, args: &[BslValue]) -> RtResult<()> {
    let state = state(obj, "Извлечь")?;
    let (item, dir, mode) = match args {
        [item, dir] => (item, dir, None),
        [item, dir, mode] => (item, dir, Some(mode)),
        // Четвёртый аргумент — пароль; см. doc comment.
        [item, dir, mode, _] => (item, dir, Some(mode)),
        _ => {
            return Err(RtError::MethodNotApplicable {
                method: "Извлечь",
                receiver: obj.type_name(),
            })
        }
    };
    let (index, generation) = match item {
        BslValue::Object(o) => match &**o {
            // Элемент обязан быть из ЭТОГО архива: сам он свой читатель
            // помнит, но `Извлечь` — метод читателя, и распаковывать чужую
            // запись, ничего не сказав, хуже, чем отказать. Тождества
            // состояния для этого мало — оно переживает переоткрытие, — и
            // номер открытия сверяет `ArchiveState::item` ниже.
            BslObject::ArchiveEntry(_, s, i, g) if Rc::ptr_eq(s, state) => (*i, *g),
            BslObject::ArchiveEntry(..) => {
                return Err(zip_err("элемент принадлежит другому архиву"))
            }
            _ => {
                return Err(RtError::TypeError {
                    expected: "ЭлементZipФайла",
                    op: "Извлечь",
                })
            }
        },
        _ => {
            return Err(RtError::TypeError {
                expected: "ЭлементZipФайла",
                op: "Извлечь",
            })
        }
    };

    let restore = restore_paths(mode, "Извлечь")?;
    let dir = destination(dir, "Извлечь")?;
    let state = state.borrow();
    let open = state.opened("Извлечь")?;
    extract_item(
        open,
        state.item(index, generation, "Извлечь")?,
        &dir,
        restore,
    )
}

/// `ИзвлечьВсе(Каталог[, Режим])`.
///
/// # Errors
///
/// Те же, что у [`extract`], плюс [`RtError::Zip`] на записи с пустым
/// именем: распаковать её некуда (платформа на таком архиве тоже
/// отказывает).
pub fn extract_all(obj: &BslValue, args: &[BslValue]) -> RtResult<()> {
    let state = state(obj, "ИзвлечьВсе")?;
    let (dir, mode) = match args {
        [dir] => (dir, None),
        [dir, mode] => (dir, Some(mode)),
        _ => {
            return Err(RtError::MethodNotApplicable {
                method: "ИзвлечьВсе",
                receiver: obj.type_name(),
            })
        }
    };
    let restore = restore_paths(mode, "ИзвлечьВсе")?;
    let dir = destination(dir, "ИзвлечьВсе")?;
    let state = state.borrow();
    let open = state.opened("ИзвлечьВсе")?;
    for item in &open.items {
        extract_item(open, item, &dir, restore)?;
    }
    Ok(())
}

/// Режим восстановления путей.
///
/// Умолчание — восстанавливать: измерено, что `ИзвлечьВсе(Куда)` без режима
/// создаёт подкаталоги. А вот ПЕРЕДАННОЕ `Неопределено` здесь не проходит —
/// тоже измерено, обоими методами сразу («Несоответствие типов (параметр
/// номер \'2\')» у `ИзвлечьВсе` и «номер \'3\'» у `Извлечь`), в отличие от
/// пароля, который `Неопределено` принимает. Поэтому пропущенный аргумент
/// это `None`, а не `Undefined`: у платформы «не передан» и «передано
/// Неопределено» — разные вещи, и различить их можно только по числу
/// фактических аргументов (пропуск в середине, `Ф(а, , б)`, эта реализация
/// не поддерживает вовсе).
fn restore_paths(mode: Option<&BslValue>, op: &'static str) -> RtResult<bool> {
    match mode {
        None => Ok(true),
        Some(BslValue::Enum(crate::EnumValue::RestorePaths)) => Ok(true),
        Some(BslValue::Enum(crate::EnumValue::DontRestorePaths)) => Ok(false),
        Some(_) => Err(RtError::TypeError {
            expected: "РежимВосстановленияПутейФайловZIP",
            op,
        }),
    }
}

/// Каталог назначения. Пустая строка — ошибка (измерено: «Некорректный путь
/// для распаковки»), а несуществующий каталог создаётся (тоже измерено).
fn destination(dir: &BslValue, op: &'static str) -> RtResult<std::path::PathBuf> {
    let dir = dir.as_str(op)?.to_string();
    if dir.is_empty() {
        return Err(zip_err("некорректный путь для распаковки"));
    }
    Ok(std::path::PathBuf::from(dir))
}

/// Распаковать одну запись.
fn extract_item(
    open: &OpenArchive,
    item: &ArchiveItem,
    dir: &std::path::Path,
    restore: bool,
) -> RtResult<()> {
    let mkdir = |path: &std::path::Path| {
        std::fs::create_dir_all(path).map_err(|e| {
            zip_err(&format!(
                "не удалось создать каталог «{}»: {e}",
                path.display()
            ))
        })
    };

    if item.entry.is_directory() {
        // В плоском режиме каталоги не создаются вовсе — измерено:
        // `ИзвлечьВсе(Куда, НеВосстанавливать)` на архиве с записью
        // `папка/` не оставляет никакой `папка`.
        if restore {
            mkdir(&dir.join(relative_path(item)))?;
        }
        return Ok(());
    }
    if item.name.is_empty() {
        return Err(zip_err("у записи архива пустое имя, распаковать её некуда"));
    }

    let target = if restore {
        dir.join(relative_path(item))
    } else {
        dir.join(&item.name)
    };
    if let Some(parent) = target.parent() {
        mkdir(parent)?;
    }
    let bytes = read_entry(&open.data, &item.entry)?;
    std::fs::write(&target, bytes)
        .map_err(|e| zip_err(&format!("не удалось записать «{}»: {e}", target.display())))
}

/// Путь записи относительно каталога распаковки.
///
/// Пустые компоненты выбрасываются — измерено, что `папка//двойной.txt`
/// платформа кладёт в `папка/двойной.txt`, хотя `ПолноеИмя` показывает обе
/// подряд идущие черты. Компоненты `.` и `..` после подстановки не
/// возникают (см. [`sanitize_component`]), но проверка оставлена: путь
/// строится из чужих данных, и выход вверх по дереву не должен зависеть от
/// рассуждения о другой функции.
fn relative_path(item: &ArchiveItem) -> std::path::PathBuf {
    let mut path = std::path::PathBuf::new();
    for part in item.full_name().split('/') {
        if part.is_empty() || part == "." || part == ".." {
            continue;
        }
        path.push(part);
    }
    path
}

// --------------------------------------------------------------------------
// Писатель встроенного языка
// --------------------------------------------------------------------------

/// Куда писатель денет архив по `Записать()`.
///
/// Цель ЛЕНИВА: измерено, что после `Новый ЗаписьZipФайла(имя)` файла ещё
/// нет и появляется он только на `Записать()`, а несуществующий каталог
/// платформа не заводит, а объявляет ошибкой («Каталог не обнаружен»).
enum WriteTarget {
    /// Имя файла. Существующий файл перезаписывается (измерено на цели,
    /// в которой лежал посторонний текст).
    File(std::path::PathBuf),
    /// Поток. `Записать()` его НЕ закрывает — измерено: после `Записать`
    /// ручной `Закрыть` потока проходит.
    Stream(BslValue),
}

/// Способ сжатия из `МетодСжатияZIP`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum WriteMethod {
    /// `Сжатие` / `Deflate` — способ хранения 8, умолчание конструктора.
    #[default]
    Deflate,
    /// `Копирование` / `Copy` — способ хранения 0, данные как есть
    /// (измерено: `сжат` совпадает с `разм` до байта).
    Stored,
}

/// Режим сохранения путей (`РежимСохраненияПутейZIP`).
///
/// Умолчание ИЗМЕРЕНО и оно неочевидно: пропущенный аргумент ведёт себя как
/// `НеСохранятьПути`, а не как `СохранятьОтносительныеПути`. На одном и том
/// же дереве `Добавить(маска, , Рекурсивно)` даёт плоские `f1.txt`,
/// `f2.txt`, а `Добавить(маска, СохранятьОтносительныеПути, Рекурсивно)` —
/// `a/f1.txt`, `a/b/f2.txt`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PathMode {
    /// Путь относительно каталога маски.
    Relative,
    /// Полный путь без ведущего слэша (измерено: `/tmp/x/f.txt` ложится
    /// как `tmp/x/f.txt`).
    Full,
    /// Только имя файла.
    Flat,
}

/// Режим обхода подкаталогов (`РежимОбработкиПодкаталоговZIP`).
/// Умолчание — `НеОбрабатывать` (измерено).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SubdirMode {
    Skip,
    Recurse,
}

/// Запись, накопленная `Добавить` и ещё не выложенная в архив.
///
/// Данные сжимаются сразу в `Добавить`, а не в `Записать`: у платформы
/// `Добавить` тоже читает файл немедленно — она отвечает «Файл не
/// обнаружен» в момент вызова, а не при записи архива.
struct PendingEntry {
    /// Имя в архиве: прямые слэши, у каталога — слэш на конце.
    name: String,
    packed: Vec<u8>,
    method: u16,
    raw_len: u32,
    crc: u32,
    /// Время и дата MS-DOS из времени изменения исходного файла.
    time: u16,
    date: u16,
    /// Запись-каталог: данных нет, а внешние атрибуты помечают её каталогом.
    directory: bool,
}

/// Состояние писателя — `ЗаписьZipФайла` либо `ЗаписьФайлаАрхива`.
///
/// Отличий от читателя два, и оба измерены. Во-первых, «открыт» здесь
/// значит «есть цель»: `ПолучитьДвоичныеДанные()` на писателе с целью
/// отвечает «Архив уже открыт!», а `Записать()` на писателе без цели —
/// «Архив не открыт!». Во-вторых, `Записать()` не только выкладывает
/// архив, но и ОЧИЩАЕТ накопленное: после него тот же файл добавляется
/// повторно без ошибки о дубле, а `ПолучитьДвоичныеДанные()` отдаёт архив
/// только из того, что добавлено после.
#[derive(Default)]
pub struct WriterState {
    target: Option<WriteTarget>,
    method: WriteMethod,
    comment: String,
    entries: Vec<PendingEntry>,
    /// Имена, ПО КОТОРЫМ проверяется уникальность, — до подстановки полного
    /// пути пустому имени (см. [`plan_name`]). Именно поэтому два пустых
    /// каталога в плоском режиме сталкиваются друг с другом, хотя в архив
    /// легли бы с разными полными путями.
    used: Vec<String>,
}

impl std::fmt::Debug for WriterState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WriterState")
            .field("open", &self.target.is_some())
            .field("method", &self.method)
            .field("entries", &self.entries.len())
            .finish()
    }
}

/// Состояние писателя за значением.
fn writer_state<'a>(v: &'a BslValue, op: &'static str) -> RtResult<&'a Rc<RefCell<WriterState>>> {
    match v {
        BslValue::Object(o) => match &**o {
            BslObject::ArchiveWriter(_, s) => Ok(s),
            _ => Err(RtError::MethodNotApplicable {
                method: op,
                receiver: v.type_name(),
            }),
        },
        _ => Err(RtError::MethodNotApplicable {
            method: op,
            receiver: v.type_name(),
        }),
    }
}

/// Тег писателя: он решает, какой у конструктора и у `Открыть` хвост
/// аргументов.
fn writer_kind(v: &BslValue) -> RtResult<ArchiveKind> {
    match v {
        BslValue::Object(o) => match &**o {
            BslObject::ArchiveWriter(kind, _) => Ok(*kind),
            _ => Err(RtError::NotAnObject),
        },
        _ => Err(RtError::NotAnObject),
    }
}

/// Объект записи ли это (`ЗаписьZipФайла` либо `ЗаписьФайлаАрхива`).
pub fn is_writer(v: &BslValue) -> bool {
    matches!(v, BslValue::Object(o) if matches!(&**o, BslObject::ArchiveWriter(..)))
}

/// `Новый ЗаписьZipФайла([Файл][, Пароль][, Комментарий][, МетодСжатия]
/// [, УровеньСжатия][, МетодШифрования][, КодировкаИмён])` и
/// `Новый ЗаписьФайлаАрхива([Файл][, Пароль][, ТипФайлаАрхива]
/// [, Комментарий][, ...])`.
///
/// Хвосты у двух типов РАЗНЫЕ, и это измерено по одному аргументу: у
/// zip-варианта третий — комментарий (строка проходит и оказывается
/// комментарием архива), у архивного третий — `ТипФайлаАрхива` (строка,
/// число, булево и члены остальных перечислений отвергаются с
/// «Несоответствие типов (параметр номер '3')»), а комментарий у него
/// четвёртый. Мест у них тоже разное число: у zip-варианта семь, у
/// архивного восемь — измерено с настоящим путём первым аргументом (см.
/// якорь `ZIP.WRITER.TAIL` и `tests/conformance/measure/measure-zip.bsl`).
///
/// # Errors
///
/// [`RtError::Zip`] на непустом пароле, на любом методе шифрования, на
/// методе сжатия BZIP2 и на формате архива, кроме ZIP: всё это платформа
/// умеет, а здесь честный отказ вместо тихой подмены;
/// [`RtError::TypeError`], если аргумент не того типа.
pub fn new_archive_writer(zip: bool, args: &[BslValue]) -> RtResult<BslValue> {
    let kind = if zip {
        ArchiveKind::Zip
    } else {
        ArchiveKind::Archive
    };
    let state = Rc::new(RefCell::new(WriterState::default()));
    configure(kind, &state, args, "ЗаписьZipФайла")?;
    Ok(BslValue::Object(Rc::new(BslObject::ArchiveWriter(
        kind, state,
    ))))
}

/// `Открыть(Файл[, ...])` у писателя — те же аргументы, что у его
/// конструктора.
///
/// # Errors
///
/// [`RtError::Zip`], если архив уже открыт (измерено: «Архив уже открыт!:
/// <путь>») либо если запрошено неподдержанное; [`RtError::TypeError`] на
/// аргументе не того типа.
pub fn writer_open(obj: &BslValue, args: &[BslValue]) -> RtResult<()> {
    let kind = writer_kind(obj)?;
    let state = writer_state(obj, "Открыть")?.clone();
    if args.is_empty() {
        return Err(RtError::MethodNotApplicable {
            method: "Открыть",
            receiver: obj.type_name(),
        });
    }
    if state.borrow().target.is_some() {
        return Err(zip_err("архив уже открыт"));
    }
    configure(kind, &state, args, "Открыть")
}

/// Разобрать общий для конструктора и `Открыть` список аргументов.
fn configure(
    kind: ArchiveKind,
    state: &Rc<RefCell<WriterState>>,
    args: &[BslValue],
    op: &'static str,
) -> RtResult<()> {
    let arg = |i: usize| args.get(i).unwrap_or(&BslValue::Undefined);
    let target = write_target(arg(0), op)?;
    check_password(arg(1))?;

    let (comment, method) = match kind {
        // `ЗаписьZipФайла`: комментарий, метод, уровень, шифрование,
        // кодировка имён.
        ArchiveKind::Zip => {
            let comment = optional_text(arg(2), op)?;
            let method = write_method(arg(3), op)?;
            check_level(arg(4), op)?;
            check_encryption(arg(5), op)?;
            check_names_encoding(arg(6), op)?;
            (comment, method)
        }
        // `ЗаписьФайлаАрхива`: тип архива, комментарий — и хвост из четырёх
        // мест, которые принимают ТОЛЬКО пустое. Это измерено направленно
        // (якорь `ZIP.WRITER.TAIL`): пустой хвост любой длины до восьмого
        // места включительно платформа принимает, а непустое пятое отвергает
        // на семнадцати типах — строке, числе, дате, булеве, массиве,
        // структуре, соответствии, `ДвоичныеДанные`, `ПотокВПамяти` и членах
        // всех семи перечислений ZIP вместе с `ТипФайлаАрхива`; шестое и
        // седьмое — на пяти и четырёх из тех же типов, восьмое — на строке и
        // двух перечислениях.
        // Отказ здесь честнее догадки: любой домысленный тип означал бы
        // МОЛЧАЛИВОЕ игнорирование настройки, которую платформа явно чем-то
        // считает.
        ArchiveKind::Archive => {
            check_archive_type(arg(2))?;
            let comment = optional_text(arg(3), op)?;
            for (i, value) in args.iter().enumerate().skip(4) {
                if !matches!(value, BslValue::Undefined) {
                    return Err(RtError::TypeError {
                        expected: "Неопределено",
                        op: match i {
                            4 => "ЗаписьФайлаАрхива (аргумент 5)",
                            5 => "ЗаписьФайлаАрхива (аргумент 6)",
                            6 => "ЗаписьФайлаАрхива (аргумент 7)",
                            _ => "ЗаписьФайлаАрхива (аргумент 8)",
                        },
                    });
                }
            }
            (comment, WriteMethod::Deflate)
        }
    };

    let mut state = state.borrow_mut();
    state.target = target;
    state.method = method;
    state.comment = comment;
    Ok(())
}

/// Первый аргумент: имя файла, поток либо ничего.
///
/// `ДвоичныеДанные` платформа НЕ принимает (измерено: «Несоответствие типов
/// (параметр номер '1') (Некорректное имя файла)»), пустую строку — тоже
/// («Некорректное имя файла»).
fn write_target(source: &BslValue, op: &'static str) -> RtResult<Option<WriteTarget>> {
    match source {
        BslValue::Undefined => Ok(None),
        BslValue::Str(s) => {
            let path = s.to_string();
            if path.is_empty() {
                return Err(zip_err("некорректное имя файла"));
            }
            Ok(Some(WriteTarget::File(std::path::PathBuf::from(path))))
        }
        _ if crate::stream::is_stream(source) => Ok(Some(WriteTarget::Stream(source.clone()))),
        _ => Err(RtError::TypeError {
            expected: "Строка или Поток",
            op,
        }),
    }
}

/// Пароль. Шифрования здесь нет ни одного вида, поэтому непустой пароль —
/// отказ, а не молчаливо НЕзашифрованный архив: платформа с паролем пишет
/// архив, у которого `Зашифрован` = «Да» (измерено), и отдать вместо него
/// открытые данные было бы худшим из возможных ответов.
fn check_password(password: &BslValue) -> RtResult<()> {
    match password {
        BslValue::Undefined => Ok(()),
        BslValue::Str(s) if s.to_string().is_empty() => Ok(()),
        BslValue::Str(_) => Err(zip_err(
            "шифрование архива не поддерживается: уберите пароль",
        )),
        _ => Err(RtError::TypeError {
            expected: "Строка",
            op: "Пароль",
        }),
    }
}

/// Необязательный строковый аргумент — у обоих писателей это комментарий
/// архива, только на разных местах.
fn optional_text(value: &BslValue, op: &'static str) -> RtResult<String> {
    match value {
        BslValue::Undefined => Ok(String::new()),
        BslValue::Str(s) => Ok(s.to_string()),
        _ => Err(RtError::TypeError {
            expected: "Строка",
            op,
        }),
    }
}

/// `МетодСжатияZIP`. BZIP2 платформа умеет (измерено: 25 байт данных легли
/// в 60), а здесь его нет — честный отказ.
fn write_method(value: &BslValue, op: &'static str) -> RtResult<WriteMethod> {
    match value {
        BslValue::Undefined => Ok(WriteMethod::Deflate),
        BslValue::Enum(crate::EnumValue::ZipMethodDeflate) => Ok(WriteMethod::Deflate),
        BslValue::Enum(crate::EnumValue::ZipMethodCopy) => Ok(WriteMethod::Stored),
        BslValue::Enum(crate::EnumValue::ZipMethodBzip2) => Err(zip_err(
            "метод сжатия BZIP2 не поддерживается, доступны «Сжатие» и «Копирование»",
        )),
        _ => Err(RtError::TypeError {
            expected: "МетодСжатияZIP",
            op,
        }),
    }
}

/// `УровеньСжатияZIP` принимается и на байты НЕ влияет.
///
/// Уровни у платформы различимы, и разница измерена на 19297 байтах:
/// `Минимальный` — 623 байта, `Оптимальный` — 628, `Максимальный` — 633
/// (да, именно в таком порядке). Здесь deflate один — LZ77 с
/// фиксированными кодами Хаффмана (см. [`crate::deflate`]), — поэтому все
/// три уровня дают одни и те же байты. Совместимости это не касается:
/// уровень не записывается в формат и на распаковку не влияет.
fn check_level(value: &BslValue, op: &'static str) -> RtResult<()> {
    match value {
        BslValue::Undefined => Ok(()),
        BslValue::Enum(e) if e.kind() == crate::EnumKind::ZipCompressionLevel => Ok(()),
        _ => Err(RtError::TypeError {
            expected: "УровеньСжатияZIP",
            op,
        }),
    }
}

/// `МетодШифрованияZIP` — любой означает шифрование, которого здесь нет.
fn check_encryption(value: &BslValue, op: &'static str) -> RtResult<()> {
    match value {
        BslValue::Undefined => Ok(()),
        BslValue::Enum(e) if e.kind() == crate::EnumKind::ZipEncryptionMethod => Err(zip_err(
            &format!("шифрование «{}» не поддерживается", e.display_text()),
        )),
        _ => Err(RtError::TypeError {
            expected: "МетодШифрованияZIP",
            op,
        }),
    }
}

/// `КодировкаИменФайловВZipФайле`. Имена здесь всегда UTF-8 с битом 11 —
/// ровно то, что платформа пишет и по умолчанию (`Авто`), и по явному
/// `UTF8`: в её собственном архиве имя записи лежит в UTF-8, флаг 0x0800
/// выставлен.
fn check_names_encoding(value: &BslValue, op: &'static str) -> RtResult<()> {
    match value {
        BslValue::Undefined => Ok(()),
        BslValue::Enum(e) if e.kind() == crate::EnumKind::ZipFileNamesEncoding => Ok(()),
        _ => Err(RtError::TypeError {
            expected: "КодировкаИменФайловВZipФайле",
            op,
        }),
    }
}

/// `Добавить(Путь[, РежимСохраненияПутей][, РежимОбработкиПодкаталогов])`.
///
/// Арность ИЗМЕРЕНА: четвёртый аргумент — «Слишком много фактических
/// параметров». Пропущенные режимы — не `Неопределено`: переданное
/// `Неопределено` платформа отвергает («Несоответствие типов (параметр
/// номер '2')»), а пропуск означает `НеСохранятьПути` и
/// `НеОбрабатывать`.
///
/// # Errors
///
/// [`RtError::Zip`], если файла или каталога маски нет, если имя в архиве
/// уже занято (измерено: «Файл с таким именем в архиве уже существует») или
/// если файл не читается; [`RtError::TypeError`] на режиме не того типа.
pub fn writer_add(obj: &BslValue, args: &[BslValue]) -> RtResult<()> {
    let state = writer_state(obj, "Добавить")?;
    let (path, mode, subdirs) = match args {
        [path] => (path, None, None),
        [path, mode] => (path, Some(mode), None),
        [path, mode, subdirs] => (path, Some(mode), Some(subdirs)),
        _ => {
            return Err(RtError::MethodNotApplicable {
                method: "Добавить",
                receiver: obj.type_name(),
            })
        }
    };
    let path = add_path(path)?;
    let mode = path_mode(mode)?;
    let subdirs = subdir_mode(subdirs)?;
    let mut state = state.borrow_mut();
    add_by_pattern(&mut state, &path, mode, subdirs)
}

/// Путь или маска первым аргументом `Добавить`.
///
/// Строгой типизации здесь у платформы НЕТ: `Добавить(1)` она встречает не
/// «Несоответствием типов», а «Файл не обнаружен '1'» — то есть число
/// становится именем. Пустая строка и `Неопределено` — «Некорректное имя
/// файла».
fn add_path(value: &BslValue) -> RtResult<String> {
    let text = match value {
        BslValue::Str(s) => s.to_string(),
        BslValue::Number(n) => n.to_string(),
        _ => return Err(zip_err("некорректное имя файла")),
    };
    if text.is_empty() {
        return Err(zip_err("некорректное имя файла"));
    }
    Ok(text)
}

/// Второй аргумент `Добавить`.
fn path_mode(mode: Option<&BslValue>) -> RtResult<PathMode> {
    match mode {
        None => Ok(PathMode::Flat),
        Some(BslValue::Enum(crate::EnumValue::ZipStoreRelativePath)) => Ok(PathMode::Relative),
        Some(BslValue::Enum(crate::EnumValue::ZipStoreFullPath)) => Ok(PathMode::Full),
        Some(BslValue::Enum(crate::EnumValue::ZipDontStorePath)) => Ok(PathMode::Flat),
        Some(_) => Err(RtError::TypeError {
            expected: "РежимСохраненияПутейZIP",
            op: "Добавить",
        }),
    }
}

/// Третий аргумент `Добавить`.
fn subdir_mode(mode: Option<&BslValue>) -> RtResult<SubdirMode> {
    match mode {
        None => Ok(SubdirMode::Skip),
        Some(BslValue::Enum(crate::EnumValue::ZipDontProcessSubdirs)) => Ok(SubdirMode::Skip),
        Some(BslValue::Enum(crate::EnumValue::ZipProcessSubdirsRecursively)) => {
            Ok(SubdirMode::Recurse)
        }
        Some(_) => Err(RtError::TypeError {
            expected: "РежимОбработкиПодкаталоговZIP",
            op: "Добавить",
        }),
    }
}

/// Маска ли это — знаки `*` и `?` ищутся только в ПОСЛЕДНЕЙ компоненте.
///
/// ИЗМЕРЕНО, что маска в середине пути маской не считается:
/// `Добавить("/т/*/*.txt")` не находит ничего (и кладёт запись-каталог
/// самого `/т/`), а `Добавить("/т/по?/вложенный.txt")` отвечает «Файл не
/// обнаружен» с этим самым путём, знаки вопроса и всё.
fn split_pattern(path: &str) -> (String, String) {
    // Обратный слэш считается разделителем, как и у читателя, где это
    // измерено на именах записей. Побочное следствие: файл, в имени
    // которого на этой файловой системе законно стоит `\`, ляжет в архив
    // под именем после последнего такого знака.
    let normalized = path.replace('\\', "/");
    match normalized.rfind('/') {
        Some(at) => (
            normalized[..at + 1].to_string(),
            normalized[at + 1..].to_string(),
        ),
        None => (String::new(), normalized),
    }
}

/// Совпадение имени с маской `*`/`?`.
///
/// Сравнение с учётом регистра: на этой платформе имена файлов
/// регистрозависимы, и маска `*.txt` файла `ВЕРХ.TXT` не находит
/// (измерено).
fn mask_matches(mask: &str, name: &str) -> bool {
    let mask: Vec<char> = mask.chars().collect();
    let name: Vec<char> = name.chars().collect();
    // Классический двухуказательный разбор со звёздочкой-точкой возврата:
    // рекурсия по маске из чужого ввода могла бы уйти сколь угодно глубоко.
    let (mut m, mut n) = (0usize, 0usize);
    let (mut star, mut back) = (usize::MAX, 0usize);
    while n < name.len() {
        if m < mask.len() && (mask[m] == '?' || mask[m] == name[n]) {
            m += 1;
            n += 1;
        } else if m < mask.len() && mask[m] == '*' {
            star = m;
            back = n;
            m += 1;
        } else if star != usize::MAX {
            back += 1;
            m = star + 1;
            n = back;
        } else {
            return false;
        }
    }
    while m < mask.len() && mask[m] == '*' {
        m += 1;
    }
    m == mask.len()
}

/// Разложить `Добавить` на записи и сложить их в состояние.
fn add_by_pattern(
    state: &mut WriterState,
    path: &str,
    mode: PathMode,
    subdirs: SubdirMode,
) -> RtResult<()> {
    let (base, mut pattern) = split_pattern(path);
    // Слэш на конце — это ИЗМЕРЕННОЕ сокращение для `<каталог>/*`:
    // `Добавить("/т/под/")` кладёт ровно то же, что `Добавить("/т/под/*")`,
    // тогда как тот же каталог, названный без слэша, платформа молча
    // пропускает. Разбор оставил здесь пустую маску, поэтому подставляем
    // всесовпадающую и уходим в общую ветку маски — с тем же базовым
    // каталогом и тем же режимом подкаталогов.
    if pattern.is_empty()
        && (path.ends_with('/') || path.ends_with('\\'))
        && std::fs::metadata(&base).is_ok_and(|meta| meta.is_dir())
    {
        pattern = "*".to_string();
    }
    if !pattern.contains('*') && !pattern.contains('?') {
        // Не маска, а имя. Каталог по такому имени платформа молча
        // пропускает: `Добавить("/т/под")` — «прошло» и ноль записей, даже
        // с рекурсией.
        let full = std::path::PathBuf::from(path);
        let meta = std::fs::metadata(&full)
            .map_err(|_| zip_err(&format!("файл не обнаружен «{path}»")))?;
        if meta.is_dir() {
            return Ok(());
        }
        let name = plan_name(mode, &pattern, &pattern, path, false);
        return add_file(state, &full, name, &meta);
    }

    let dir = if base.is_empty() {
        std::path::PathBuf::from(".")
    } else {
        std::path::PathBuf::from(&base)
    };
    let meta = std::fs::metadata(&dir).map_err(|_| {
        // Платформа называет в этой ошибке каталог со слэшем, а не всю
        // маску (измерено).
        zip_err(&format!("файл не обнаружен «{base}»"))
    })?;
    if !meta.is_dir() {
        return Err(zip_err(&format!("файл не обнаружен «{base}»")));
    }
    walk_dir(state, &dir, &base, "", &pattern, mode, subdirs, true)
}

/// Обойти один каталог: файлы по маске, подкаталоги — вглубь на месте.
///
/// Порядок обхода — тот, что отдаёт файловая система, и это ИЗМЕРЕНО: на
/// каталоге, где `ls -U` показывает `данные.dat`, `файл2.txt`, `файл1.txt`,
/// платформа кладёт записи в этом же порядке, а не по алфавиту и не по
/// времени. Подкаталог обходится ровно на своём месте в этом порядке.
///
/// `selected` — каталог назван самой маской (или это её базовый каталог).
/// От этого зависит запись-каталог: ПУСТОЙ выбранный каталог не даёт
/// ничего, а пустой каталог, до которого дошла рекурсия, даёт запись
/// (измерено обоими способами на одном дереве).
#[allow(clippy::too_many_arguments)]
fn walk_dir(
    state: &mut WriterState,
    dir: &std::path::Path,
    dir_display: &str,
    rel: &str,
    mask: &str,
    mode: PathMode,
    subdirs: SubdirMode,
    selected: bool,
) -> RtResult<()> {
    let reader = std::fs::read_dir(dir).map_err(|e| {
        zip_err(&format!(
            "не удалось прочитать каталог «{dir_display}»: {e}"
        ))
    })?;
    let mut matched_here = 0usize;
    let mut children = 0usize;
    for entry in reader {
        let entry = entry.map_err(|e| {
            zip_err(&format!(
                "не удалось прочитать каталог «{dir_display}»: {e}"
            ))
        })?;
        children += 1;
        let name = entry.file_name().to_string_lossy().into_owned();
        let path = entry.path();
        // Каталог ли это — решается по САМОМУ элементу каталога, без
        // перехода по символической ссылке. Иначе ссылка на предка
        // зациклила бы рекурсию и уронила процесс переполнением стека, а
        // вход здесь чужой: маску задаёт скрипт, дерево — файловая система.
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        if kind.is_dir() {
            if subdirs == SubdirMode::Recurse {
                let child_rel = format!("{rel}{name}/");
                let child_display = format!("{dir_display}{name}/");
                walk_dir(
                    state,
                    &path,
                    &child_display,
                    &child_rel,
                    mask,
                    mode,
                    subdirs,
                    mask_matches(mask, &name),
                )?;
            }
            continue;
        }
        if !mask_matches(mask, &name) {
            continue;
        }
        // А вот содержимое и время берутся по ПУТИ: ссылка на файл ложится
        // в архив тем, на что указывает.
        let Ok(meta) = std::fs::metadata(&path) else {
            continue;
        };
        matched_here += 1;
        let rel_name = format!("{rel}{name}");
        let full = format!("{dir_display}{name}");
        let planned = plan_name(mode, &name, &rel_name, &full, false);
        add_file(state, &path, planned, &meta)?;
    }

    // Каталог, в котором маска не нашла ни одного файла, платформа
    // записывает САМА — записью-каталогом. Исключение одно: выбранный
    // маской (или заданный ею базовый) каталог, оказавшийся пустым, не даёт
    // ничего.
    if matched_here == 0 && !(selected && children == 0) {
        let planned = plan_name(mode, "", rel, dir_display, true);
        let stamp = std::fs::metadata(dir).ok();
        add_directory(state, planned, stamp.as_ref())?;
    }
    Ok(())
}

/// Имя записи по режиму путей: `(ключ, имя в архиве)`.
///
/// Ключ — то, по чему проверяется уникальность, а имя — то, что ложится в
/// архив. Расходятся они на пустом ключе: у записи-каталога в плоском
/// режиме и у базового каталога в относительном имя пусто, и платформа
/// подставляет вместо него ПОЛНЫЙ путь, продолжая при этом считать занятым
/// пустое имя. Отсюда измеренное: два пустых каталога в плоском режиме
/// сталкиваются («Файл с таким именем в архиве уже существует:  -
/// /т/пусто»), хотя в архив легли бы под разными полными путями.
fn plan_name(
    mode: PathMode,
    file_name: &str,
    rel: &str,
    full: &str,
    directory: bool,
) -> (String, String) {
    let full_key = {
        let stripped = full.strip_prefix('/').unwrap_or(full);
        if directory && !stripped.is_empty() && !stripped.ends_with('/') {
            format!("{stripped}/")
        } else {
            stripped.to_string()
        }
    };
    let key = match mode {
        PathMode::Flat => file_name.to_string(),
        PathMode::Relative => rel.to_string(),
        PathMode::Full => full_key.clone(),
    };
    let name = if key.is_empty() {
        full_key
    } else {
        key.clone()
    };
    (key, name)
}

/// Занять имя или отказать так же, как платформа.
fn reserve(state: &mut WriterState, key: String, source: &str) -> RtResult<()> {
    if state.used.contains(&key) {
        return Err(zip_err(&format!(
            "файл с таким именем в архиве уже существует: {key} — {source}"
        )));
    }
    state.used.push(key);
    Ok(())
}

/// Прочитать файл, сжать и запомнить запись.
fn add_file(
    state: &mut WriterState,
    path: &std::path::Path,
    planned: (String, String),
    meta: &std::fs::Metadata,
) -> RtResult<()> {
    let (key, name) = planned;
    reserve(state, key, &path.display().to_string())?;
    let data = std::fs::read(path)
        .map_err(|e| zip_err(&format!("не удалось прочитать «{}»: {e}", path.display())))?;
    let raw_len = u32::try_from(data.len())
        .map_err(|_| zip_err(&format!("файл «{}» больше 4 ГиБ", path.display())))?;
    let crc = crc32(&data);
    let (method, packed) = match state.method {
        WriteMethod::Stored => (METHOD_STORED, data),
        // В отличие от [`ZipWriter::add`], который выбирает способ по
        // результату, здесь способ задан конструктором: платформа с
        // `МетодСжатияZIP.Сжатие` пишет deflate даже там, где он длиннее
        // исходных данных (измерено: 13 байт легли в 16).
        WriteMethod::Deflate => (METHOD_DEFLATED, crate::deflate::deflate(&data)),
    };
    let (time, date) = dos_fields(meta);
    state.entries.push(PendingEntry {
        name,
        packed,
        method,
        raw_len,
        crc,
        time,
        date,
        directory: false,
    });
    Ok(())
}

/// Запомнить запись-каталог: имя со слэшем, нулевые данные и время
/// изменения самого каталога.
fn add_directory(
    state: &mut WriterState,
    planned: (String, String),
    meta: Option<&std::fs::Metadata>,
) -> RtResult<()> {
    let (key, mut name) = planned;
    reserve(state, key, &name.clone())?;
    if !name.ends_with('/') {
        name.push('/');
    }
    let (time, date) = match meta {
        Some(meta) => dos_fields(meta),
        None => (0, 0),
    };
    state.entries.push(PendingEntry {
        name,
        packed: Vec::new(),
        method: METHOD_STORED,
        raw_len: 0,
        crc: 0,
        time,
        date,
        directory: true,
    });
    Ok(())
}

/// Время изменения файла в полях MS-DOS.
///
/// Момент берётся БЕЗ поправки на зону — так же, как его отдаёт
/// `ТекущаяДата` (см. `BslValue::current_date`): в `std` нет способа узнать
/// смещение локальной зоны, а тип даты в 1С зоны не хранит вовсе. Файл
/// раньше 1980 года формату не представим, поэтому такие даты зажимаются в
/// начало 1980-го — иначе поле года ушло бы в минус.
fn dos_fields(meta: &std::fs::Metadata) -> (u16, u16) {
    let Ok(modified) = meta.modified() else {
        return (0, 0);
    };
    let Ok(since) = modified.duration_since(std::time::UNIX_EPOCH) else {
        return (0, 0);
    };
    let secs = since.as_secs() as i64 + crate::date::UNIX_EPOCH_SECONDS;
    let Some(date) = crate::BslDate::from_seconds(secs) else {
        return (0, 0);
    };
    let civil = date.to_civil();
    if civil.year < 1980 {
        // 1980-01-01 00:00:00 — наименьшее, что поля MS-DOS выражают.
        return (0, (1 << 5) | 1);
    }
    let year = u16::try_from(civil.year - 1980).unwrap_or(0) & 0x7F;
    let dos_date = (year << 9) | ((civil.month as u16 & 0xF) << 5) | (civil.day as u16 & 0x1F);
    let dos_time = ((civil.hour as u16 & 0x1F) << 11)
        | ((civil.minute as u16 & 0x3F) << 5)
        | ((civil.second as u16 / 2) & 0x1F);
    (dos_time, dos_date)
}

/// `Записать()` — выложить архив в цель и закрыть его.
///
/// # Errors
///
/// [`RtError::Zip`], если архив не открыт (измерено: «Архив не открыт!» —
/// в том числе на втором `Записать` подряд) либо если цель не пишется.
pub fn writer_write(obj: &BslValue) -> RtResult<()> {
    let state = writer_state(obj, "Записать")?;
    let mut state = state.borrow_mut();
    let Some(target) = state.target.take() else {
        return Err(zip_err("архив не открыт"));
    };
    let bytes = build_archive(&state.entries, &state.comment)?;
    // Цель снята со состояния ДО записи, поэтому неудачная запись тоже
    // оставляет архив закрытым. Что делает в этом случае платформа, НЕ
    // измерено (снят только сам отказ на несуществующем каталоге, без
    // повторной попытки); выбрано закрывать, потому что иначе повторный
    // `Записать` молча пытался бы писать туда же второй раз.
    state.entries.clear();
    state.used.clear();
    match target {
        WriteTarget::File(path) => std::fs::write(&path, &bytes)
            .map_err(|e| zip_err(&format!("не удалось записать «{}»: {e}", path.display()))),
        WriteTarget::Stream(stream) => crate::stream::write_all(&stream, &bytes, "Записать"),
    }
}

/// `ПолучитьДвоичныеДанные()` — архив из накопленного, не трогая цель.
///
/// # Errors
///
/// [`RtError::Zip`], если архив открыт: измерено, что на писателе с целью
/// платформа отвечает «Архив уже открыт!», и только после `Записать()` (или
/// у писателя, созданного без цели) отдаёт данные.
pub fn writer_binary_data(obj: &BslValue) -> RtResult<BslValue> {
    let state = writer_state(obj, "ПолучитьДвоичныеДанные")?;
    let state = state.borrow();
    if state.target.is_some() {
        return Err(zip_err("архив уже открыт"));
    }
    let bytes = build_archive(&state.entries, &state.comment)?;
    Ok(BslValue::Object(Rc::new(BslObject::BinaryData(
        bytes.into(),
    ))))
}

/// Собрать архив из накопленных записей.
///
/// Zip64 здесь нет намеренно, как и в [`ZipWriter`]: за границей в четыре
/// гигабайта честный отказ, а не молчаливо испорченный каталог.
fn build_archive(entries: &[PendingEntry], comment: &str) -> RtResult<Vec<u8>> {
    let too_big = || zip_err("архив больше 4 ГиБ не поддерживается");
    let mut out: Vec<u8> = Vec::new();
    let mut offsets: Vec<u32> = Vec::with_capacity(entries.len());
    for e in entries {
        offsets.push(u32::try_from(out.len()).map_err(|_| too_big())?);
        let packed_len = u32::try_from(e.packed.len()).map_err(|_| too_big())?;
        let name = e.name.as_bytes();
        let name_len = u16::try_from(name.len())
            .map_err(|_| zip_err(&format!("имя записи «{}» длиннее 65535 байт", e.name)))?;
        out.extend_from_slice(&SIG_LOCAL.to_le_bytes());
        out.extend_from_slice(&20u16.to_le_bytes());
        out.extend_from_slice(&FLAG_UTF8_NAME.to_le_bytes());
        out.extend_from_slice(&e.method.to_le_bytes());
        out.extend_from_slice(&e.time.to_le_bytes());
        out.extend_from_slice(&e.date.to_le_bytes());
        out.extend_from_slice(&e.crc.to_le_bytes());
        out.extend_from_slice(&packed_len.to_le_bytes());
        out.extend_from_slice(&e.raw_len.to_le_bytes());
        out.extend_from_slice(&name_len.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(name);
        out.extend_from_slice(&e.packed);
    }

    let start = u32::try_from(out.len()).map_err(|_| too_big())?;
    for (e, offset) in entries.iter().zip(&offsets) {
        let packed_len = u32::try_from(e.packed.len()).map_err(|_| too_big())?;
        let name = e.name.as_bytes();
        let name_len = name.len() as u16;
        out.extend_from_slice(&SIG_CENTRAL.to_le_bytes());
        out.extend_from_slice(&20u16.to_le_bytes());
        out.extend_from_slice(&20u16.to_le_bytes());
        out.extend_from_slice(&FLAG_UTF8_NAME.to_le_bytes());
        out.extend_from_slice(&e.method.to_le_bytes());
        out.extend_from_slice(&e.time.to_le_bytes());
        out.extend_from_slice(&e.date.to_le_bytes());
        out.extend_from_slice(&e.crc.to_le_bytes());
        out.extend_from_slice(&packed_len.to_le_bytes());
        out.extend_from_slice(&e.raw_len.to_le_bytes());
        out.extend_from_slice(&name_len.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes()); // extra
        out.extend_from_slice(&0u16.to_le_bytes()); // комментарий записи
        out.extend_from_slice(&0u16.to_le_bytes()); // номер диска
        out.extend_from_slice(&0u16.to_le_bytes()); // внутренние атрибуты
                                                    // Внешние атрибуты: у каталога поднят бит `FILE_ATTRIBUTE_DIRECTORY`,
                                                    // чтобы посторонние распаковщики видели каталог не только по слэшу
                                                    // в имени.
        let external: u32 = if e.directory { 0x10 } else { 0 };
        out.extend_from_slice(&external.to_le_bytes());
        out.extend_from_slice(&offset.to_le_bytes());
        out.extend_from_slice(name);
    }
    let size = u32::try_from(out.len()).map_err(|_| too_big())? - start;
    let count = u16::try_from(entries.len())
        .map_err(|_| zip_err("в архиве больше 65535 записей, что требует Zip64"))?;
    let comment = comment.as_bytes();
    if comment.len() > MAX_COMMENT {
        return Err(zip_err("комментарий архива длиннее 65535 байт"));
    }
    out.extend_from_slice(&SIG_EOCD.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&count.to_le_bytes());
    out.extend_from_slice(&count.to_le_bytes());
    out.extend_from_slice(&size.to_le_bytes());
    out.extend_from_slice(&start.to_le_bytes());
    out.extend_from_slice(&(comment.len() as u16).to_le_bytes());
    out.extend_from_slice(comment);
    Ok(out)
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

    // ---------------------------------------------------------------------
    // Поверхность встроенного языка
    //
    // Ожидаемые значения ниже — не рассуждение, а вывод 8.3.27 на тех же
    // именах: те же архивы собирались python3 zipfile, читались платформой
    // и печатались по свойствам `Имя`/`ПолноеИмя`/`Путь`/`Исходное*`.
    // ---------------------------------------------------------------------

    /// Отображаемые и исходные имена всех записей архива, собранного
    /// здешним писателем из перечисленных имён.
    fn names_of(names: &[&str]) -> Vec<(String, String)> {
        let mut z = ZipWriter::new();
        for name in names {
            z.add(name, b"x");
        }
        let bytes = z.finish();
        let (entries, _) = ZipArchive::parse(&bytes)
            .expect("собственный архив читается")
            .into_parts();
        build_items(entries)
            .iter()
            .map(|i| (i.full_name(), i.orig_full_name()))
            .collect()
    }

    fn shown(names: &[&str]) -> Vec<String> {
        names_of(names).into_iter().map(|(full, _)| full).collect()
    }

    /// Недопустимые в имени файла знаки становятся подчёркиванием, а
    /// столкнувшиеся имена получают `(N)` перед расширением. Обратный слэш
    /// при этом РАЗДЕЛИТЕЛЬ, а не знак имени, и виден таким даже в
    /// `ИсходноеПолноеИмя`.
    #[test]
    fn forbidden_characters_and_collisions_match_the_platform() {
        let pairs = names_of(&[
            "a:b.txt",
            "a*b.txt",
            "a?b.txt",
            "a<b>c.txt",
            "a|b.txt",
            "a\"b.txt",
            "dir\\back.txt",
            "../up.txt",
            "/abs.txt",
        ]);
        let shown: Vec<&str> = pairs.iter().map(|(full, _)| full.as_str()).collect();
        assert_eq!(
            shown,
            [
                "a_b.txt",
                "a_b(1).txt",
                "a_b(2).txt",
                "a_b_c.txt",
                "a_b(3).txt",
                "a_b(4).txt",
                "dir/back.txt",
                // Компонента `..` срезается вместе с точками и остаётся
                // пустой, а пустая компонента следующей записи с ней уже
                // сталкивается — отсюда `(1)`.
                "/up.txt",
                "(1)/abs.txt",
            ]
        );
        let original: Vec<&str> = pairs.iter().map(|(_, orig)| orig.as_str()).collect();
        assert_eq!(
            original,
            [
                "a:b.txt",
                "a*b.txt",
                "a?b.txt",
                "a<b>c.txt",
                "a|b.txt",
                "a\"b.txt",
                "dir/back.txt",
                "../up.txt",
                "/abs.txt",
            ]
        );
    }

    /// Суффикс встаёт перед ПОСЛЕДНЕЙ точкой, а у имени без точки —
    /// в конец. Ведущая точка расширением считается (`.hidden` ->
    /// `(1).hidden`), хвостовая срезается вместе с пробелами.
    #[test]
    fn the_collision_suffix_goes_before_the_last_dot() {
        assert_eq!(
            shown(&[
                "noext",
                "noext",
                "a.b.c.txt",
                "a.b.c.txt",
                ".hidden",
                ".hidden",
                "x.",
                "x."
            ]),
            [
                "noext",
                "noext(1)",
                "a.b.c.txt",
                "a.b.c(1).txt",
                ".hidden",
                "(1).hidden",
                "x",
                "x(1)",
            ]
        );
        assert_eq!(
            shown(&["trail .txt", "trail ", "two..", "dir /f.txt"]),
            ["trail .txt", "trail", "two", "dir/f.txt"]
        );
    }

    /// Один и тот же каталог у записи-каталога и у файла внутри него —
    /// ОДИН узел: `(1)` здесь не появляется. У самой записи-каталога
    /// короткого имени нет, а `ПолноеИмя` совпадает с `Путь`.
    #[test]
    fn a_directory_entry_and_a_file_inside_it_share_one_node() {
        let mut z = ZipWriter::new();
        z.add("папка/", b"");
        z.add("папка/вложенный.txt", b"x");
        z.add("папка//двойной.txt", b"x");
        let bytes = z.finish();
        let (entries, _) = ZipArchive::parse(&bytes).unwrap().into_parts();
        let items = build_items(entries);

        assert_eq!(items[0].name, "");
        assert_eq!(items[0].path, "папка/");
        assert_eq!(items[0].full_name(), "папка/");
        assert_eq!(items[1].name, "вложенный.txt");
        assert_eq!(items[1].path, "папка/");
        assert_eq!(items[1].full_name(), "папка/вложенный.txt");
        // Пустая компонента сохраняется в имени, но каталогом на диске не
        // становится — см. `relative_path`.
        assert_eq!(items[2].full_name(), "папка//двойной.txt");
        assert_eq!(
            relative_path(&items[2]),
            std::path::Path::new("папка/двойной.txt")
        );
    }

    /// Имя без бита 11 всё равно декодируется как UTF-8 — с заменяющими
    /// символами на негодных байтах. Это НЕ догадка: платформа на имени
    /// «привет.txt» в CP866 показывает ровно эти шесть кодовых точек.
    #[test]
    fn a_single_byte_name_is_decoded_as_lossy_utf8() {
        const CP866: [u8; 10] = [0xAF, 0xE0, 0xA8, 0xA2, 0xA5, 0xE2, 0x2E, 0x74, 0x78, 0x74];
        let mut data = REF_MIXED.to_vec();
        let central = central_at(&data, "шум.bin".as_bytes());
        let local = local_at(&data, central);
        data[central + CENTRAL_HEADER_LEN..central + CENTRAL_HEADER_LEN + CP866.len()]
            .copy_from_slice(&CP866);
        data[local + LOCAL_HEADER_LEN..local + LOCAL_HEADER_LEN + CP866.len()]
            .copy_from_slice(&CP866);
        set_flags(&mut data, central, 0, FLAG_UTF8_NAME);

        let (entries, _) = ZipArchive::parse(&data).unwrap().into_parts();
        let items = build_items(entries);
        assert_eq!(items[1].name, "\u{FFFD}\u{A22}\u{FFFD}\u{FFFD}.txt");
    }

    /// Разбор даты MS-DOS по краям всех полей — таблица снята на платформе
    /// архивом, у которого поля выставлены руками.
    #[test]
    fn dos_dates_are_normalized_arithmetically() {
        let date = |y: i64, m: u16, d: u16| ((y as u16 - 1980) << 9) | (m << 5) | d;
        let time = |h: u16, mi: u16, s2: u16| (h << 11) | (mi << 5) | s2;
        let civil = |t: u16, d: u16| {
            let c = dos_datetime(t, d).to_civil();
            (c.year, c.month, c.day, c.hour, c.minute, c.second)
        };

        assert_eq!(
            civil(time(23, 59, 29), date(2000, 12, 31)),
            (2000, 12, 31, 23, 59, 58)
        );
        assert_eq!(civil(0, date(2000, 13, 1)), (2001, 1, 1, 0, 0, 0));
        assert_eq!(civil(0, date(2000, 0, 1)), (1999, 12, 1, 0, 0, 0));
        assert_eq!(civil(0, date(2000, 1, 0)), (1999, 12, 31, 0, 0, 0));
        assert_eq!(civil(0, date(2001, 2, 30)), (2001, 3, 2, 0, 0, 0));
        assert_eq!(
            civil(time(25, 0, 0), date(2000, 1, 1)),
            (2000, 1, 2, 1, 0, 0)
        );
        assert_eq!(
            civil(time(0, 61, 0), date(2000, 1, 1)),
            (2000, 1, 1, 1, 1, 0)
        );
        assert_eq!(
            civil(time(0, 0, 31), date(2000, 1, 1)),
            (2000, 1, 1, 0, 1, 2)
        );
        // Оба поля нулевые — это самый частый мусор в чужих архивах.
        assert_eq!(civil(0, 0), (1979, 11, 30, 0, 0, 0));
    }

    /// Имя без расширения и расширение — по последней точке.
    #[test]
    fn the_extension_is_taken_from_the_last_dot() {
        assert_eq!(split_extension("вложенный.txt"), ("вложенный", "txt"));
        assert_eq!(split_extension("a.b.c.txt"), ("a.b.c", "txt"));
        assert_eq!(split_extension("noext"), ("noext", ""));
        assert_eq!(split_extension(".hidden"), ("", "hidden"));
        assert_eq!(split_extension(""), ("", ""));
    }

    /// Готовый архив на диске — его путь как строка встроенного языка,
    /// пригодная и для конструктора, и для `Открыть`.
    fn archive_file(bytes: &[u8], name: &str) -> BslValue {
        let dir = std::env::temp_dir().join(format!("open-bsl-zip-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        std::fs::write(&path, bytes).unwrap();
        BslValue::Str(crate::BslString::from_str(path.to_str().unwrap()))
    }

    /// Объект чтения над готовым архивом — то, что видит встроенный язык.
    fn reader_over(bytes: &[u8], name: &str) -> BslValue {
        new_archive_reader(
            true,
            &archive_file(bytes, name),
            &BslValue::Undefined,
            &BslValue::Undefined,
        )
        .expect("архив открывается")
    }

    /// Пустой каталог для распаковки, свой у каждой пробы.
    fn output_dir(name: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("open-bsl-zip-out-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    fn prop(entry: &BslValue, name: &str) -> String {
        entry_prop(entry, name).unwrap().to_string()
    }

    /// Свойства элемента и `Найти` — против измеренного вывода платформы.
    #[test]
    fn entry_properties_and_find_follow_the_platform() {
        let reader = reader_over(&REF_MIXED, "props.zip");
        let items = entries(&reader).expect("архив открыт");
        assert_eq!(count(&items).unwrap(), 2);

        let first = get(&items, 0).unwrap();
        assert_eq!(prop(&first, "Имя"), "накладная.txt");
        assert_eq!(prop(&first, "FullName"), "накладная.txt");
        assert_eq!(prop(&first, "Путь"), "");
        assert_eq!(prop(&first, "ИмяБезРасширения"), "накладная");
        assert_eq!(prop(&first, "Расширение"), "txt");
        assert_eq!(prop(&first, "ИсходноеИмя"), "накладная.txt");
        assert_eq!(prop(&first, "РазмерНесжатого"), "234");
        assert_eq!(prop(&first, "Зашифрован"), "Нет");
        assert!(
            entry_prop(&first, "Размер").is_err(),
            "у платформы такого свойства нет"
        );

        // Регистр в имени не значим, а искать надо по короткому имени.
        let found = find(
            &items,
            &BslValue::Str(crate::BslString::from_str("ШУМ.BIN")),
        )
        .unwrap();
        assert_eq!(prop(&found, "Имя"), "шум.bin");
        let missing = find(
            &items,
            &BslValue::Str(crate::BslString::from_str("нет.txt")),
        )
        .unwrap();
        assert!(matches!(missing, BslValue::Undefined));
    }

    /// Подстановка недопустимого знака `Найти` не мешает: ищется ИСХОДНОЕ
    /// имя, а не отображаемое. Обе строки измерены на платформе фикстурой
    /// `zip-read`.
    #[test]
    fn find_matches_the_original_name_not_the_substituted_one() {
        let mut z = ZipWriter::new();
        z.add("отчёт:2026.txt", b"x");
        let reader = reader_over(&z.finish(), "find.zip");
        let items = entries(&reader).unwrap();

        let by_original = find(
            &items,
            &BslValue::Str(crate::BslString::from_str("отчёт:2026.txt")),
        )
        .unwrap();
        assert_eq!(prop(&by_original, "Имя"), "отчёт_2026.txt");
        let by_shown = find(
            &items,
            &BslValue::Str(crate::BslString::from_str("отчёт_2026.txt")),
        )
        .unwrap();
        assert!(matches!(by_shown, BslValue::Undefined));
    }

    /// Закрытый архив отвечает ошибкой на всё, а повторное открытие —
    /// работает.
    #[test]
    fn a_closed_archive_refuses_everything_and_reopens() {
        let reader = reader_over(&REF_MIXED, "closed.zip");
        close(&reader).expect("первый Закрыть проходит");
        assert!(close(&reader).is_err(), "второй Закрыть — ошибка");
        assert!(entries(&reader).is_err(), "Элементы на закрытом — ошибка");

        let dir = std::env::temp_dir().join(format!("open-bsl-zip-{}", std::process::id()));
        let path = dir.join("closed.zip");
        let source = BslValue::Str(crate::BslString::from_str(path.to_str().unwrap()));
        open(&reader, std::slice::from_ref(&source)).expect("после Закрыть открывается снова");
        assert!(
            open(&reader, &[source]).is_err(),
            "открытый второй раз — ошибка"
        );
        assert_eq!(count(&entries(&reader).unwrap()).unwrap(), 2);
    }

    /// Распаковка обоими режимами: с путями и плоско. Запись-каталог
    /// создаёт каталог только в первом режиме.
    #[test]
    fn extract_all_honours_the_path_mode() {
        let mut z = ZipWriter::new();
        z.add("папка/", b"");
        z.add("папка/вложенный.txt", "содержимое".as_bytes());
        z.add("верхний.txt", b"top");
        let reader = reader_over(&z.finish(), "extract.zip");

        let root = std::env::temp_dir().join(format!("open-bsl-zip-out-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let deep = root.join("сПутями");
        let flat = root.join("плоско");
        let arg =
            |p: &std::path::Path| BslValue::Str(crate::BslString::from_str(p.to_str().unwrap()));

        // Каталога назначения ещё нет — измерено, что он создаётся.
        extract_all(&reader, &[arg(&deep)]).expect("распаковка с путями");
        assert!(deep.join("папка").is_dir());
        assert_eq!(
            std::fs::read(deep.join("папка/вложенный.txt")).unwrap(),
            "содержимое".as_bytes()
        );
        assert!(deep.join("верхний.txt").is_file());

        extract_all(
            &reader,
            &[
                arg(&flat),
                BslValue::Enum(crate::EnumValue::DontRestorePaths),
            ],
        )
        .expect("плоская распаковка");
        assert!(flat.join("вложенный.txt").is_file());
        assert!(!flat.join("папка").exists(), "каталогов быть не должно");

        // Пустая строка каталога — ошибка, а `Неопределено` режимом не
        // считается (оба измерены).
        assert!(extract_all(&reader, &[BslValue::Str(crate::BslString::from_str(""))]).is_err());
        assert!(extract_all(&reader, &[arg(&flat), BslValue::Undefined]).is_err());
    }

    /// Извлечение одной записи: элемент чужого архива не принимается, а
    /// зашифрованная запись даёт внятный отказ, а не пустой файл.
    #[test]
    fn extract_checks_the_element_and_refuses_encrypted_entries() {
        let reader = reader_over(&REF_MIXED, "one.zip");
        let other = reader_over(&REF_MIXED, "one-more.zip");
        let items = entries(&reader).unwrap();
        let dir = std::env::temp_dir().join(format!("open-bsl-zip-one-{}", std::process::id()));
        let arg = BslValue::Str(crate::BslString::from_str(dir.to_str().unwrap()));

        let alien = get(&entries(&other).unwrap(), 0).unwrap();
        let e = extract(&reader, &[alien, arg.clone()]).expect_err("элемент чужого архива");
        assert!(e.to_string().contains("другому архиву"), "текст: {e}");
        assert!(extract(&reader, &[BslValue::Undefined, arg.clone()]).is_err());

        let first = get(&items, 0).unwrap();
        extract(&reader, &[first, arg.clone()]).expect("своя запись распаковывается");
        assert!(dir.join("накладная.txt").is_file());

        let mut data = REF_MIXED.to_vec();
        let central = central_at(&data, "накладная.txt".as_bytes());
        set_flags(&mut data, central, FLAG_ENCRYPTED, 0);
        let locked = reader_over(&data, "locked.zip");
        let entry = get(&entries(&locked).unwrap(), 0).unwrap();
        let e = extract(&locked, &[entry, arg]).expect_err("расшифровки здесь нет");
        assert!(e.to_string().contains("зашифрован"), "текст: {e}");
    }

    /// Архив на одну запись — короче эталона, поэтому номер из эталона в
    /// него не попадает.
    fn one_entry_archive(name: &str) -> BslValue {
        let mut z = ZipWriter::new();
        z.add("один.txt", "один".as_bytes());
        archive_file(&z.finish(), name)
    }

    /// Элемент, взятый до `Закрыть`, после переоткрытия на БОЛЕЕ КОРОТКИЙ
    /// архив отвечает ошибкой, а не паникой: номер записи действителен
    /// только внутри того открытия, при котором элемент выдан.
    #[test]
    fn extract_of_a_stale_entry_errors_instead_of_panicking() {
        let reader = reader_over(&REF_MIXED, "stale.zip");
        let stale = get(&entries(&reader).unwrap(), 1).unwrap();
        assert_eq!(prop(&stale, "Имя"), "шум.bin");

        let smaller = one_entry_archive("stale-small.zip");
        close(&reader).expect("Закрыть проходит");
        open(&reader, std::slice::from_ref(&smaller)).expect("открывается снова");
        assert_eq!(count(&entries(&reader).unwrap()).unwrap(), 1);

        let dir = output_dir("stale");
        let arg = BslValue::Str(crate::BslString::from_str(dir.to_str().unwrap()));
        let e = extract(&reader, &[stale, arg]).expect_err("элемент от прошлого открытия");
        assert!(e.to_string().contains("другом открытии"), "текст: {e}");
        assert!(!dir.exists(), "распаковывать было нечего");
    }

    /// Тот же элемент, но новый архив НЕ короче: номер в него попадает, и
    /// молчаливая выдача занявшей его чужой записи была бы хуже отказа.
    #[test]
    fn a_stale_entry_never_extracts_the_record_that_took_its_number() {
        let reader = reader_over(&REF_MIXED, "stale-same.zip");
        let stale = get(&entries(&reader).unwrap(), 0).unwrap();
        assert_eq!(prop(&stale, "Имя"), "накладная.txt");

        let mut z = ZipWriter::new();
        z.add("подмена.txt", "чужое".as_bytes());
        z.add("вторая.txt", "ещё чужое".as_bytes());
        let other = archive_file(&z.finish(), "stale-same-other.zip");
        close(&reader).expect("Закрыть проходит");
        open(&reader, std::slice::from_ref(&other)).expect("открывается снова");
        assert_eq!(count(&entries(&reader).unwrap()).unwrap(), 2);

        let dir = output_dir("stale-same");
        let arg = BslValue::Str(crate::BslString::from_str(dir.to_str().unwrap()));
        let e = extract(&reader, &[stale, arg.clone()]).expect_err("элемент от прошлого открытия");
        assert!(e.to_string().contains("другом открытии"), "текст: {e}");
        assert!(
            !dir.join("подмена.txt").exists(),
            "чужая запись под тем же номером распакована не была"
        );
        assert!(!dir.join("накладная.txt").exists());

        // Свежий элемент того же читателя работает как обычно.
        let fresh = get(&entries(&reader).unwrap(), 0).unwrap();
        extract(&reader, &[fresh, arg]).expect("своя запись распаковывается");
        assert_eq!(
            std::fs::read(dir.join("подмена.txt")).unwrap(),
            "чужое".as_bytes()
        );
    }

    /// Свойства устаревшего элемента тоже отвергаются — и на закрытом
    /// архиве это по-прежнему измеренная ошибка «архив не открыт».
    #[test]
    fn properties_of_a_stale_entry_are_refused() {
        let reader = reader_over(&REF_MIXED, "stale-prop.zip");
        let stale = get(&entries(&reader).unwrap(), 1).unwrap();
        assert_eq!(prop(&stale, "Имя"), "шум.bin");

        let smaller = one_entry_archive("stale-prop-small.zip");
        close(&reader).expect("Закрыть проходит");
        let e = entry_prop(&stale, "Имя").expect_err("на закрытом архиве свойств нет");
        assert!(e.to_string().contains("не открыт"), "текст: {e}");

        open(&reader, std::slice::from_ref(&smaller)).expect("открывается снова");
        let e = entry_prop(&stale, "Имя").expect_err("элемент от прошлого открытия");
        assert!(e.to_string().contains("другом открытии"), "текст: {e}");
        let fresh = find(
            &entries(&reader).unwrap(),
            &BslValue::Str(crate::BslString::from_str("один.txt")),
        )
        .unwrap();
        assert_eq!(prop(&fresh, "Имя"), "один.txt");
    }

    /// Размеры Zip64 приходят из каталога произвольными восемью байтами:
    /// размер со старшим битом обязан остаться ПОЛОЖИТЕЛЬНЫМ числом, а не
    /// завернуться в знак. Поле 0x0001 приписано к записи руками — здешний
    /// писатель extra не пишет, а `zipfile` выносит размеры в него только
    /// выше четырёх гигабайт.
    #[test]
    fn a_zip64_size_that_does_not_fit_i64_stays_positive() {
        let mut z = ZipWriter::new();
        z.add("огромный.bin", "x".as_bytes());
        let bytes = z.finish();
        let central = central_at(&bytes, "огромный.bin".as_bytes());
        let name_len = usize::from(u16::from_le_bytes([
            bytes[central + 28],
            bytes[central + 29],
        ]));
        assert_eq!(
            u16::from_le_bytes([bytes[central + 30], bytes[central + 31]]),
            0,
            "здешний писатель extra-полей не пишет"
        );

        // Заголовок поля (идентификатор 0x0001 и длина) плюс одно
        // восьмибайтовое значение: вынесен ТОЛЬКО несжатый размер, поэтому
        // в теле поля он и стоит первым и единственным.
        let mut field = vec![0x01, 0x00, 0x08, 0x00];
        field.extend_from_slice(&0x8000_0000_0000_0000u64.to_le_bytes());
        let field_len = u16::try_from(field.len()).unwrap();
        let insert_at = central + CENTRAL_HEADER_LEN + name_len;
        let mut data = bytes[..insert_at].to_vec();
        data.extend_from_slice(&field);
        data.extend_from_slice(&bytes[insert_at..]);
        // Признак выноса — максимум на месте самого размера в записи.
        data[central + 24..central + 28].copy_from_slice(&MAX_U32.to_le_bytes());
        data[central + 30..central + 32].copy_from_slice(&field_len.to_le_bytes());
        // Каталог вырос ровно на длину поля; без правки записи конца
        // каталога проба упёрлась бы в ошибку разбора, а не в размер.
        let eocd = eocd_at(&data);
        let cd_size = u32::from_le_bytes([
            data[eocd + 12],
            data[eocd + 13],
            data[eocd + 14],
            data[eocd + 15],
        ]) + u32::from(field_len);
        data[eocd + 12..eocd + 16].copy_from_slice(&cd_size.to_le_bytes());

        let reader = reader_over(&data, "zip64-size.zip");
        let entry = get(&entries(&reader).unwrap(), 0).unwrap();
        assert_eq!(prop(&entry, "РазмерНесжатого"), "9223372036854775808");
        assert_eq!(prop(&entry, "UncompressedSize"), "9223372036854775808");
    }

    /// Комментарий архива читается и декодируется как UTF-8.
    #[test]
    fn the_archive_comment_is_readable() {
        let mut bytes = ZipWriter::new().finish();
        let comment = "привет-комментарий".as_bytes();
        let at = bytes.len() - 2;
        bytes[at..].copy_from_slice(&(comment.len() as u16).to_le_bytes());
        bytes.extend_from_slice(comment);

        let reader = reader_over(&bytes, "comment.zip");
        assert_eq!(comment_of(&reader), "привет-комментарий");
        assert_eq!(count(&entries(&reader).unwrap()).unwrap(), 0);
    }

    fn comment_of(reader: &BslValue) -> String {
        comment(reader).unwrap().to_string()
    }

    /// Третий аргумент конструктора: `Zip` проходит, остальные форматы —
    /// честный отказ, чужой тип — ошибка типа.
    #[test]
    fn the_archive_type_argument_only_accepts_zip() {
        let dir = std::env::temp_dir().join(format!("open-bsl-zip-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("kind.zip");
        std::fs::write(&path, REF_MIXED).unwrap();
        let source = BslValue::Str(crate::BslString::from_str(path.to_str().unwrap()));

        let ok = new_archive_reader(
            false,
            &source,
            &BslValue::Undefined,
            &BslValue::Enum(crate::EnumValue::ArchiveTypeZip),
        )
        .expect("ZIP поддержан");
        assert_eq!(ok.type_name(), "ЧтениеФайлаАрхива");

        let e = new_archive_reader(
            false,
            &source,
            &BslValue::Undefined,
            &BslValue::Enum(crate::EnumValue::ArchiveTypeTar),
        )
        .expect_err("TAR здесь не читается");
        assert!(e.to_string().contains("не поддерживается"), "текст: {e}");
        assert!(new_archive_reader(
            false,
            &source,
            &BslValue::Undefined,
            &BslValue::Boolean(true)
        )
        .is_err());
    }

    /// Испорченный вход не открывается вовсе — ошибка приходит из
    /// конструктора, а не из первой попытки прочитать запись.
    #[test]
    fn a_broken_archive_fails_in_the_constructor() {
        let dir = std::env::temp_dir().join(format!("open-bsl-zip-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("broken.zip");
        std::fs::write(&path, b"not a zip at all, not even close").unwrap();
        let source = BslValue::Str(crate::BslString::from_str(path.to_str().unwrap()));
        assert!(
            new_archive_reader(true, &source, &BslValue::Undefined, &BslValue::Undefined).is_err()
        );

        let missing = BslValue::Str(crate::BslString::from_str("/несуществующий/архив.zip"));
        assert!(
            new_archive_reader(true, &missing, &BslValue::Undefined, &BslValue::Undefined).is_err()
        );
    }
    // --- писатель ------------------------------------------------------------

    /// Дерево для проб писателя: `f0.txt`, `a/f1.txt`, `a/b/f2.txt`,
    /// `c/f3.dat` и пустой `пуст`. Имя каталога уникально на тест, иначе
    /// параллельные тесты растащили бы друг у друга файлы.
    fn write_tree(tag: &str) -> std::path::PathBuf {
        let root = std::env::temp_dir().join(format!("open-bsl-zipw-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("a/b")).unwrap();
        std::fs::create_dir_all(root.join("c")).unwrap();
        std::fs::create_dir_all(root.join("пуст")).unwrap();
        std::fs::write(root.join("f0.txt"), b"nol").unwrap();
        std::fs::write(root.join("a/f1.txt"), b"odin").unwrap();
        std::fs::write(root.join("a/b/f2.txt"), b"dva").unwrap();
        std::fs::write(root.join("c/f3.dat"), b"tri").unwrap();
        root
    }

    fn writer(target: &std::path::Path) -> BslValue {
        new_archive_writer(
            true,
            &[BslValue::Str(crate::BslString::from_str(
                target.to_str().unwrap(),
            ))],
        )
        .expect("писатель строится")
    }

    fn str_value(text: &str) -> BslValue {
        BslValue::Str(crate::BslString::from_str(text))
    }

    /// Имена записей собранного архива — по каталогу, а не по локальным
    /// заголовкам.
    fn entry_names(bytes: &[u8]) -> Vec<String> {
        let (entries, _) = ZipArchive::parse(bytes)
            .expect("свой архив читается")
            .into_parts();
        entries
            .iter()
            .map(|e| String::from_utf8_lossy(e.name_bytes()).into_owned())
            .collect()
    }

    /// Собрать архив писателем и вернуть его байты, не трогая файловую
    /// систему целью.
    fn built(state_owner: &BslValue) -> Vec<u8> {
        match writer_binary_data(state_owner).expect("данные отдаются") {
            BslValue::Object(o) => match &*o {
                BslObject::BinaryData(bytes) => bytes.to_vec(),
                _ => panic!("не двоичные данные"),
            },
            _ => panic!("не объект"),
        }
    }

    /// Три режима путей на ОДНОМ файле дают три разных имени, и все три
    /// измерены на 8.3.27.
    #[test]
    fn the_three_path_modes_name_one_file_the_measured_way() {
        let root = write_tree("modes");
        let file = root.join("f0.txt");
        let file = str_value(file.to_str().unwrap());

        let flat = new_archive_writer(true, &[]).unwrap();
        writer_add(&flat, std::slice::from_ref(&file)).unwrap();
        assert_eq!(entry_names(&built(&flat)), vec!["f0.txt"]);

        let relative = new_archive_writer(true, &[]).unwrap();
        writer_add(
            &relative,
            &[
                file.clone(),
                BslValue::Enum(crate::EnumValue::ZipStoreRelativePath),
            ],
        )
        .unwrap();
        assert_eq!(entry_names(&built(&relative)), vec!["f0.txt"]);

        let full = new_archive_writer(true, &[]).unwrap();
        writer_add(
            &full,
            &[file, BslValue::Enum(crate::EnumValue::ZipStoreFullPath)],
        )
        .unwrap();
        // Полный путь ложится БЕЗ ведущего слэша — измерено.
        let names = entry_names(&built(&full));
        assert_eq!(names.len(), 1);
        assert!(!names[0].starts_with('/'), "имя: {}", names[0]);
        assert!(names[0].ends_with("/f0.txt"), "имя: {}", names[0]);
    }

    /// Маска берёт только последнюю компоненту пути, `?` — ровно один знак,
    /// регистр значим.
    #[test]
    fn the_mask_matches_the_measured_way() {
        assert!(mask_matches("*", "f0.txt"));
        assert!(mask_matches("*.txt", "f0.txt"));
        assert!(!mask_matches("*.txt", "f0.dat"));
        assert!(mask_matches("f?.txt", "f0.txt"));
        assert!(!mask_matches("f?.txt", "f10.txt"));
        assert!(mask_matches("*.*", "a.b"));
        assert!(!mask_matches("*.txt", "ВЕРХ.TXT"));
        assert!(mask_matches("*a*b*", "xxayybzz"));
        assert!(!mask_matches("*a*b*c", "ab"));
        // Маска ищется только в последней компоненте.
        assert_eq!(
            split_pattern("/т/каталог/*.txt"),
            ("/т/каталог/".to_string(), "*.txt".to_string())
        );
    }

    /// Рекурсия с относительными путями сохраняет подкаталоги, плоский
    /// режим их роняет — оба ответа измерены на одном дереве.
    #[test]
    fn recursion_keeps_subpaths_only_in_the_relative_mode() {
        let root = write_tree("recurse");
        let mask = str_value(&format!("{}/*", root.display()));

        let relative = new_archive_writer(true, &[]).unwrap();
        writer_add(
            &relative,
            &[
                mask.clone(),
                BslValue::Enum(crate::EnumValue::ZipStoreRelativePath),
                BslValue::Enum(crate::EnumValue::ZipProcessSubdirsRecursively),
            ],
        )
        .unwrap();
        let mut names = entry_names(&built(&relative));
        names.sort();
        assert_eq!(names, vec!["a/b/f2.txt", "a/f1.txt", "c/f3.dat", "f0.txt"]);

        let flat = new_archive_writer(true, &[]).unwrap();
        writer_add(
            &flat,
            &[
                mask,
                BslValue::Enum(crate::EnumValue::ZipDontStorePath),
                BslValue::Enum(crate::EnumValue::ZipProcessSubdirsRecursively),
            ],
        )
        .unwrap();
        let mut names = entry_names(&built(&flat));
        names.sort();
        assert_eq!(names, vec!["f0.txt", "f1.txt", "f2.txt", "f3.dat"]);
    }

    /// Без третьего аргумента подкаталоги не обходятся вовсе.
    #[test]
    fn without_the_subdirectory_mode_only_the_named_directory_is_taken() {
        let root = write_tree("flat");
        let writer = new_archive_writer(true, &[]).unwrap();
        writer_add(&writer, &[str_value(&format!("{}/*", root.display()))]).unwrap();
        assert_eq!(entry_names(&built(&writer)), vec!["f0.txt"]);
    }

    /// Каталог, в котором маска ничего не нашла, платформа записывает
    /// записью-каталогом — но только если до него дошла РЕКУРСИЯ, а не сама
    /// маска. Обе половины измерены: `*` не оставляет от пустого `пуст`
    /// ничего, `*.txt` оставляет запись `пуст/`.
    #[test]
    fn a_directory_without_matches_becomes_an_entry_unless_it_was_selected() {
        let root = write_tree("dirs");

        let selected = new_archive_writer(true, &[]).unwrap();
        writer_add(
            &selected,
            &[
                str_value(&format!("{}/*", root.display())),
                BslValue::Enum(crate::EnumValue::ZipStoreRelativePath),
                BslValue::Enum(crate::EnumValue::ZipProcessSubdirsRecursively),
            ],
        )
        .unwrap();
        let names = entry_names(&built(&selected));
        assert!(
            !names.iter().any(|n| n.ends_with("пуст/")),
            "имена: {names:?}"
        );

        let unselected = new_archive_writer(true, &[]).unwrap();
        writer_add(
            &unselected,
            &[
                str_value(&format!("{}/*.txt", root.display())),
                BslValue::Enum(crate::EnumValue::ZipStoreRelativePath),
                BslValue::Enum(crate::EnumValue::ZipProcessSubdirsRecursively),
            ],
        )
        .unwrap();
        let mut names = entry_names(&built(&unselected));
        names.sort();
        assert_eq!(
            names,
            vec!["a/b/f2.txt", "a/f1.txt", "c/", "f0.txt", "пуст/"]
        );

        // Пустой каталог, названный САМОЙ маской, не даёт ничего — тоже
        // измерено.
        let empty_base = new_archive_writer(true, &[]).unwrap();
        writer_add(
            &empty_base,
            &[
                str_value(&format!("{}/пуст/*", root.display())),
                BslValue::Enum(crate::EnumValue::ZipStoreRelativePath),
                BslValue::Enum(crate::EnumValue::ZipProcessSubdirsRecursively),
            ],
        )
        .unwrap();
        assert!(entry_names(&built(&empty_base)).is_empty());
    }

    /// Столкновение имён — ошибка, и она останавливает `Добавить`, оставляя
    /// в архиве всё, что успело лечь до неё. Ключ уникальности — ИМЯ ДО
    /// подстановки полного пути, поэтому два пустых каталога в плоском
    /// режиме сталкиваются пустыми именами.
    #[test]
    fn colliding_names_stop_the_add_and_keep_what_came_before() {
        let root = write_tree("dup");
        std::fs::create_dir_all(root.join("пуст2")).unwrap();

        let writer = new_archive_writer(true, &[]).unwrap();
        let file = str_value(root.join("f0.txt").to_str().unwrap());
        writer_add(&writer, std::slice::from_ref(&file)).unwrap();
        let e = writer_add(&writer, &[file]).expect_err("второй раз то же имя");
        assert!(e.to_string().contains("уже существует"), "текст: {e}");
        assert_eq!(entry_names(&built(&writer)), vec!["f0.txt"]);

        let flat = new_archive_writer(true, &[]).unwrap();
        let e = writer_add(
            &flat,
            &[
                str_value(&format!("{}/*.нет", root.display())),
                BslValue::Enum(crate::EnumValue::ZipDontStorePath),
                BslValue::Enum(crate::EnumValue::ZipProcessSubdirsRecursively),
            ],
        )
        .expect_err("два пустых имени подряд");
        assert!(e.to_string().contains("уже существует"), "текст: {e}");
    }

    /// `Копирование` кладёт данные как есть, `Сжатие` — deflate даже там,
    /// где он длиннее (измерено: 13 байт легли в 16).
    #[test]
    fn the_compression_method_decides_the_storage_method() {
        let root = write_tree("method");
        let file = str_value(root.join("f0.txt").to_str().unwrap());

        let stored = new_archive_writer(
            true,
            &[
                BslValue::Undefined,
                BslValue::Undefined,
                BslValue::Undefined,
                BslValue::Enum(crate::EnumValue::ZipMethodCopy),
            ],
        )
        .unwrap();
        writer_add(&stored, std::slice::from_ref(&file)).unwrap();
        let bytes = built(&stored);
        let (entries, _) = ZipArchive::parse(&bytes).unwrap().into_parts();
        assert_eq!(entries[0].method(), METHOD_STORED);
        assert_eq!(entries[0].compressed_size(), entries[0].size());

        let deflated = new_archive_writer(true, &[]).unwrap();
        writer_add(&deflated, &[file]).unwrap();
        let bytes = built(&deflated);
        let (entries, _) = ZipArchive::parse(&bytes).unwrap().into_parts();
        assert_eq!(entries[0].method(), METHOD_DEFLATED);
        assert_eq!(read_entry(&bytes, &entries[0]).unwrap(), b"nol");
    }

    /// Всё, чего здесь нет, отвергается в конструкторе — молча открытый
    /// архив вместо зашифрованного был бы худшим ответом.
    #[test]
    fn encryption_and_bzip2_are_refused_instead_of_silently_dropped() {
        let password = new_archive_writer(true, &[BslValue::Undefined, str_value("секрет")])
            .expect_err("пароль здесь не работает");
        assert!(
            password.to_string().contains("шифрование"),
            "текст: {password}"
        );

        // Пустой пароль шифрованием не считается.
        assert!(new_archive_writer(true, &[BslValue::Undefined, str_value("")]).is_ok());

        let method = new_archive_writer(
            true,
            &[
                BslValue::Undefined,
                BslValue::Undefined,
                BslValue::Undefined,
                BslValue::Enum(crate::EnumValue::ZipMethodBzip2),
            ],
        )
        .expect_err("BZIP2 здесь не пишется");
        assert!(method.to_string().contains("BZIP2"), "текст: {method}");

        let encryption = new_archive_writer(
            true,
            &[
                BslValue::Undefined,
                BslValue::Undefined,
                BslValue::Undefined,
                BslValue::Undefined,
                BslValue::Undefined,
                BslValue::Enum(crate::EnumValue::ZipEncryptionAes256),
            ],
        )
        .expect_err("шифрования нет");
        assert!(
            encryption.to_string().contains("не поддерживается"),
            "текст: {encryption}"
        );

        // Уровень сжатия, наоборот, принимается — он на байты не влияет.
        assert!(new_archive_writer(
            true,
            &[
                BslValue::Undefined,
                BslValue::Undefined,
                BslValue::Undefined,
                BslValue::Undefined,
                BslValue::Enum(crate::EnumValue::ZipLevelMaximal),
            ]
        )
        .is_ok());
    }

    /// У архивного писателя третий аргумент — тип архива, четвёртый —
    /// комментарий, а хвост (места с пятого по восьмое) платформа
    /// принимает только пустым: ИЗМЕРЕНО на семнадцати типах подряд, см.
    /// якорь `ZIP.WRITER.TAIL`.
    #[test]
    fn the_archive_writer_has_its_own_argument_tail() {
        let ok = new_archive_writer(
            false,
            &[
                BslValue::Undefined,
                BslValue::Undefined,
                BslValue::Enum(crate::EnumValue::ArchiveTypeZip),
                str_value("комментарий"),
            ],
        )
        .expect("ZIP и комментарий");
        assert_eq!(ok.type_name(), "ЗаписьФайлаАрхива");
        let bytes = built(&ok);
        let (_, comment) = ZipArchive::parse(&bytes).unwrap().into_parts();
        assert_eq!(String::from_utf8_lossy(&comment), "комментарий");

        assert!(new_archive_writer(
            false,
            &[
                BslValue::Undefined,
                BslValue::Undefined,
                BslValue::Enum(crate::EnumValue::ArchiveTypeTar)
            ]
        )
        .is_err());
        assert!(new_archive_writer(
            false,
            &[
                BslValue::Undefined,
                BslValue::Undefined,
                BslValue::Undefined,
                BslValue::Undefined,
                str_value("пятый")
            ]
        )
        .is_err());

        // Восьмое место у архивного писателя ЕСТЬ (у zip-варианта его нет)
        // и типизировано так же пусто. Резолвер не пускает сюда девятый
        // аргумент, поэтому границу проверяет тест в bsl-sema, а здесь —
        // сама пара «пустое принято, непустое отвергнуто».
        let empty_tail = vec![BslValue::Undefined; 8];
        assert!(new_archive_writer(false, &empty_tail).is_ok());
        let mut eighth = empty_tail;
        eighth[7] = BslValue::Enum(crate::EnumValue::ArchiveTypeZip);
        assert!(new_archive_writer(false, &eighth).is_err());
    }

    /// Состояние: `Записать` требует цели, отдаёт архив ОДИН раз и
    /// очищает накопленное, а `ПолучитьДвоичныеДанные` работает только на
    /// закрытом архиве.
    #[test]
    fn writing_closes_the_archive_and_clears_the_entries() {
        let root = write_tree("state");
        let target = root.join("out.zip");
        let file = str_value(root.join("f0.txt").to_str().unwrap());

        let w = writer(&target);
        writer_add(&w, std::slice::from_ref(&file)).unwrap();
        // Пока цель есть — данных не отдаём: измерено «Архив уже открыт!».
        assert!(writer_binary_data(&w).is_err());
        writer_write(&w).unwrap();
        assert_eq!(
            entry_names(&std::fs::read(&target).unwrap()),
            vec!["f0.txt"]
        );
        // Второй `Записать` — «Архив не открыт!».
        assert!(writer_write(&w).is_err());
        // Список записей очищен: тот же файл добавляется снова без ошибки о
        // дубле, и в данных он один.
        writer_add(&w, &[file]).unwrap();
        assert_eq!(entry_names(&built(&w)), vec!["f0.txt"]);

        // `Открыть` даёт цель заново, повторный — ошибка.
        let reopened = str_value(root.join("out2.zip").to_str().unwrap());
        writer_open(&w, std::slice::from_ref(&reopened)).unwrap();
        assert!(writer_open(&w, &[reopened]).is_err());
        writer_write(&w).unwrap();
        assert_eq!(
            entry_names(&std::fs::read(root.join("out2.zip")).unwrap()),
            vec!["f0.txt"]
        );
    }

    /// Отсутствующий файл и отсутствующий каталог маски — ошибки, а
    /// каталог по имени без маски платформа молча пропускает.
    #[test]
    fn missing_paths_are_errors_and_a_plain_directory_is_skipped() {
        let root = write_tree("missing");
        let w = new_archive_writer(true, &[]).unwrap();

        assert!(writer_add(&w, &[str_value(&format!("{}/нет.txt", root.display()))]).is_err());
        assert!(writer_add(&w, &[str_value(&format!("{}/нет/*", root.display()))]).is_err());
        // Каталог без маски: ни ошибки, ни записей.
        writer_add(&w, &[str_value(root.join("a").to_str().unwrap())]).unwrap();
        assert!(entry_names(&built(&w)).is_empty());
        // Пустое имя и не-строка — «некорректное имя файла».
        assert!(writer_add(&w, &[str_value("")]).is_err());
        assert!(writer_add(&w, &[BslValue::Undefined]).is_err());
        // Режим не того типа — ошибка типа, и переданное `Неопределено`
        // режимом не считается (измерено на платформе).
        let file = str_value(root.join("f0.txt").to_str().unwrap());
        assert!(writer_add(&w, &[file.clone(), BslValue::Undefined]).is_err());
        assert!(writer_add(&w, &[file, BslValue::Boolean(true)]).is_err());
    }

    /// Слэш на конце превращает каталог в маску `*`: измерено, что
    /// `Добавить("/т/a/")` кладёт то же, что `Добавить("/т/a/*")`, тогда как
    /// `Добавить("/т/a")` не кладёт ничего.
    #[test]
    fn a_trailing_slash_on_a_directory_means_the_all_matching_mask() {
        let root = write_tree("slash");
        let dir = root.join("a");

        let slashed = new_archive_writer(true, &[]).unwrap();
        writer_add(
            &slashed,
            &[
                str_value(&format!("{}/", dir.display())),
                BslValue::Enum(crate::EnumValue::ZipStoreRelativePath),
                BslValue::Enum(crate::EnumValue::ZipProcessSubdirsRecursively),
            ],
        )
        .unwrap();
        let mut names = entry_names(&built(&slashed));
        names.sort();
        assert_eq!(names, vec!["b/f2.txt", "f1.txt"]);

        // Тот же каталог без слэша — по-прежнему ноль записей и никакой
        // ошибки, даже с рекурсией.
        let plain = new_archive_writer(true, &[]).unwrap();
        writer_add(
            &plain,
            &[
                str_value(dir.to_str().unwrap()),
                BslValue::Enum(crate::EnumValue::ZipStoreRelativePath),
                BslValue::Enum(crate::EnumValue::ZipProcessSubdirsRecursively),
            ],
        )
        .unwrap();
        assert!(entry_names(&built(&plain)).is_empty());

        // Слэш на конце НЕ существующего каталога остаётся ошибкой, а не
        // пустой удачей.
        let missing = new_archive_writer(true, &[]).unwrap();
        assert!(writer_add(&missing, &[str_value(&format!("{}/нет/", root.display()))]).is_err());
    }

    /// Комментарий архива ложится в запись конца каталога, и наш же
    /// читатель его оттуда достаёт.
    #[test]
    fn the_comment_survives_a_round_trip() {
        let w = new_archive_writer(
            true,
            &[
                BslValue::Undefined,
                BslValue::Undefined,
                str_value("комментарий архива"),
            ],
        )
        .unwrap();
        let bytes = built(&w);
        let (_, comment) = ZipArchive::parse(&bytes).unwrap().into_parts();
        assert_eq!(String::from_utf8_lossy(&comment), "комментарий архива");
    }

    /// Записи-каталоги узнаются читателем как каталоги, а не как пустые
    /// файлы.
    #[test]
    fn directory_entries_read_back_as_directories() {
        let root = write_tree("dirent");
        let w = new_archive_writer(true, &[]).unwrap();
        writer_add(
            &w,
            &[
                str_value(&format!("{}/*.txt", root.display())),
                BslValue::Enum(crate::EnumValue::ZipStoreRelativePath),
                BslValue::Enum(crate::EnumValue::ZipProcessSubdirsRecursively),
            ],
        )
        .unwrap();
        let bytes = built(&w);
        let (entries, _) = ZipArchive::parse(&bytes).unwrap().into_parts();
        // Порядок записей — файловой системы (см. `walk_dir`), поэтому
        // сравнивается СОСТАВ, а не последовательность.
        let mut dirs: Vec<String> = entries
            .iter()
            .filter(|e| e.is_directory())
            .map(|e| String::from_utf8_lossy(e.name_bytes()).into_owned())
            .collect();
        dirs.sort();
        assert_eq!(dirs, vec!["c/", "пуст/"]);
    }
}
