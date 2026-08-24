//! Тесты архивов: поверхность чтения и записи над общими фикстурами.

use super::*;
use bsl_rt::SystemFileSystem;

/// Конструкторы читателя/писателя после ABI-G берут файловую систему
/// сессии; сценарии, которым файловая система безразлична, зовут эти тёзки
/// с процессной ФС по умолчанию.
fn new_archive_reader(
    zip: bool,
    source: &BslValue,
    password: &BslValue,
    archive_type: &BslValue,
) -> RtResult<BslValue> {
    super::new_archive_reader(
        zip,
        source,
        password,
        archive_type,
        std::rc::Rc::new(SystemFileSystem),
    )
}

fn new_archive_writer(zip: bool, args: &[BslValue]) -> RtResult<BslValue> {
    super::new_archive_writer(zip, args, std::rc::Rc::new(SystemFileSystem))
}

// Тесты написаны до перевода объектов на протокол и обращаются к ним
// значениями BSL. Локальные тёзки функций поверхности принимают
// значение, снимают протокольный объект и делегируют настоящим — так
// сами сценарии остаются дословно теми, какими были измерены.

fn reader_of(value: &BslValue) -> &ReaderObject {
    value
        .object_ref()
        .and_then(|object| object.downcast_ref())
        .expect("значение должно быть читателем архива")
}

fn entries_of(value: &BslValue) -> &EntriesObject {
    value
        .object_ref()
        .and_then(|object| object.downcast_ref())
        .expect("значение должно быть коллекцией элементов")
}

fn entry_of(value: &BslValue) -> &EntryObject {
    value
        .object_ref()
        .and_then(|object| object.downcast_ref())
        .expect("значение должно быть элементом архива")
}

fn writer_of(value: &BslValue) -> &WriterObject {
    value
        .object_ref()
        .and_then(|object| object.downcast_ref())
        .expect("значение должно быть писателем архива")
}

/// Состояние чтения за значением любого из трёх видов — как прежний
/// внутренний хелпер `state`.
fn state_of(value: &BslValue) -> (&Rc<RefCell<ArchiveState>>, &'static str) {
    let object = value.object_ref().expect("значение должно быть объектом");
    if let Some(reader) = object.downcast_ref::<ReaderObject>() {
        (&reader.state, reader.descriptor().name)
    } else if let Some(entries) = object.downcast_ref::<EntriesObject>() {
        (&entries.state, entries.descriptor().name)
    } else if let Some(entry) = object.downcast_ref::<EntryObject>() {
        (&entry.state, entry.descriptor().name)
    } else {
        panic!("значение должно быть объектом чтения архива")
    }
}

fn open(value: &BslValue, args: &[BslValue]) -> RtResult<()> {
    super::open(reader_of(value), &SystemFileSystem, args)
}

fn close(value: &BslValue) -> RtResult<()> {
    super::close(reader_of(value))
}

fn entries(value: &BslValue) -> RtResult<BslValue> {
    super::entries(reader_of(value))
}

fn comment(value: &BslValue) -> RtResult<BslValue> {
    super::comment(reader_of(value))
}

fn count(value: &BslValue) -> RtResult<usize> {
    super::count(entries_of(value))
}

fn get(value: &BslValue, index: usize) -> RtResult<BslValue> {
    super::get(entries_of(value), index)
}

fn find(value: &BslValue, name: &BslValue) -> RtResult<BslValue> {
    super::find(entries_of(value), name)
}

fn entry_prop(value: &BslValue, prop: &str) -> RtResult<BslValue> {
    super::entry_prop(entry_of(value), prop)
}

fn extract(value: &BslValue, args: &[BslValue]) -> RtResult<()> {
    let (state, receiver) = state_of(value);
    super::extract(state, &SystemFileSystem, receiver, args)
}

fn extract_all(value: &BslValue, args: &[BslValue]) -> RtResult<()> {
    let (state, receiver) = state_of(value);
    super::extract_all(state, &SystemFileSystem, receiver, args)
}

fn writer_open(value: &BslValue, args: &[BslValue]) -> RtResult<()> {
    super::writer_open(writer_of(value), args)
}

fn writer_add(value: &BslValue, args: &[BslValue]) -> RtResult<()> {
    super::writer_add(writer_of(value), &SystemFileSystem, args)
}

fn writer_write(value: &BslValue) -> RtResult<()> {
    super::writer_write(writer_of(value), &SystemFileSystem)
}

fn writer_binary_data(value: &BslValue) -> RtResult<BslValue> {
    super::writer_binary_data(writer_of(value))
}

/// Сборщик эталонных архивов для тестов — тот же тонкий `ZipWriter`,
/// что живёт в `bsl-rt` для XLSX: способ хранения deflate, дата записи
/// фиксированная, чтобы байты не зависели от времени прогона.
struct ZipWriter {
    inner: zip::ZipWriter<std::io::Cursor<Vec<u8>>>,
}

impl ZipWriter {
    fn new() -> Self {
        ZipWriter {
            inner: zip::ZipWriter::new(std::io::Cursor::new(Vec::new())),
        }
    }

    fn add(&mut self, name: &str, data: &[u8]) {
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .compression_level(Some(1))
            .last_modified_time(zip::DateTime::default());
        self.inner
            .start_file(name, options)
            .expect("zip start_file не отказывает на корректном имени");
        self.inner
            .write_all(data)
            .expect("zip write_all в память не отказывает");
    }

    fn finish(self) -> Vec<u8> {
        let cursor = self.inner.finish().expect("zip finish не отказывает");
        cursor.into_inner()
    }
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
// Эталонов пять. Все собраны здешним python3 (CPython 3.14) через
// `zipfile`, потому
// что `zipfile` выносит размеры в extra каталога только выше четырёх
// гигабайт. Все пять прочитаны тем же python3 обратно перед тем, как
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
    0x50, 0x4B, 0x03, 0x04, 0x14, 0x00, 0x00, 0x08, 0x08, 0x00, 0x00, 0x00, 0x21, 0x00, 0x19, 0x1C,
    0xAD, 0xDB, 0x57, 0x00, 0x00, 0x00, 0xEA, 0x00, 0x00, 0x00, 0x16, 0x00, 0x00, 0x00, 0xD0, 0xBD,
    0xD0, 0xB0, 0xD0, 0xBA, 0xD0, 0xBB, 0xD0, 0xB0, 0xD0, 0xB4, 0xD0, 0xBD, 0xD0, 0xB0, 0xD1, 0x8F,
    0x2E, 0x74, 0x78, 0x74, 0x65, 0xCE, 0xBD, 0x09, 0x80, 0x30, 0x14, 0x45, 0xE1, 0x3E, 0xBB, 0x08,
    0xC9, 0xCB, 0x9F, 0xF2, 0x96, 0x73, 0x00, 0x3B, 0x37, 0xB0, 0xB0, 0x0E, 0x41, 0xB1, 0x32, 0x33,
    0xDC, 0x6C, 0x64, 0x9D, 0x6B, 0x7B, 0xE0, 0x83, 0x83, 0x03, 0x0D, 0x15, 0xA5, 0xAF, 0x8A, 0x13,
    0x37, 0x5E, 0x14, 0x83, 0x1D, 0x15, 0x0D, 0x0F, 0xAE, 0xBE, 0x4D, 0x56, 0xED, 0x18, 0x9C, 0xE6,
    0x31, 0x88, 0xBA, 0xC0, 0x46, 0x1C, 0x23, 0x99, 0x59, 0xF9, 0xC8, 0x2A, 0x08, 0xAB, 0xB0, 0xB0,
    0x8A, 0x89, 0x55, 0xF2, 0xBF, 0x41, 0x5A, 0x16, 0xCD, 0xD9, 0x7C, 0x50, 0x4B, 0x03, 0x04, 0x14,
    0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x00, 0x21, 0x00, 0x88, 0x04, 0x12, 0x95, 0x18, 0x00, 0x00,
    0x00, 0x18, 0x00, 0x00, 0x00, 0x0A, 0x00, 0x00, 0x00, 0xD1, 0x88, 0xD1, 0x83, 0xD0, 0xBC, 0x2E,
    0x62, 0x69, 0x6E, 0x0D, 0x6E, 0xCF, 0x30, 0x91, 0xF2, 0x53, 0xB4, 0x15, 0x76, 0xD7, 0x38, 0x99,
    0xFA, 0x5B, 0xBC, 0x1D, 0x7E, 0xDF, 0x40, 0xA1, 0x02, 0x63, 0xC4, 0x50, 0x4B, 0x01, 0x02, 0x14,
    0x03, 0x14, 0x00, 0x00, 0x08, 0x08, 0x00, 0x00, 0x00, 0x21, 0x00, 0x19, 0x1C, 0xAD, 0xDB, 0x57,
    0x00, 0x00, 0x00, 0xEA, 0x00, 0x00, 0x00, 0x16, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x80, 0x01, 0x00, 0x00, 0x00, 0x00, 0xD0, 0xBD, 0xD0, 0xB0, 0xD0, 0xBA, 0xD0,
    0xBB, 0xD0, 0xB0, 0xD0, 0xB4, 0xD0, 0xBD, 0xD0, 0xB0, 0xD1, 0x8F, 0x2E, 0x74, 0x78, 0x74, 0x50,
    0x4B, 0x01, 0x02, 0x14, 0x03, 0x14, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x00, 0x21, 0x00, 0x88,
    0x04, 0x12, 0x95, 0x18, 0x00, 0x00, 0x00, 0x18, 0x00, 0x00, 0x00, 0x0A, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80, 0x01, 0x8B, 0x00, 0x00, 0x00, 0xD1, 0x88, 0xD1,
    0x83, 0xD0, 0xBC, 0x2E, 0x62, 0x69, 0x6E, 0x50, 0x4B, 0x05, 0x06, 0x00, 0x00, 0x00, 0x00, 0x02,
    0x00, 0x02, 0x00, 0x7C, 0x00, 0x00, 0x00, 0xCB, 0x00, 0x00, 0x00, 0x00, 0x00,
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
    0x50, 0x4B, 0x03, 0x04, 0x14, 0x00, 0x00, 0x08, 0x08, 0x00, 0x00, 0x00, 0x21, 0x00, 0x19, 0x1C,
    0xAD, 0xDB, 0x57, 0x00, 0x00, 0x00, 0xEA, 0x00, 0x00, 0x00, 0x0E, 0x00, 0x00, 0x00, 0xD0, 0xBE,
    0xD1, 0x82, 0xD1, 0x87, 0xD1, 0x91, 0xD1, 0x82, 0x2E, 0x74, 0x78, 0x74, 0x65, 0xCE, 0xBD, 0x09,
    0x80, 0x30, 0x14, 0x45, 0xE1, 0x3E, 0xBB, 0x08, 0xC9, 0xCB, 0x9F, 0xF2, 0x96, 0x73, 0x00, 0x3B,
    0x37, 0xB0, 0xB0, 0x0E, 0x41, 0xB1, 0x32, 0x33, 0xDC, 0x6C, 0x64, 0x9D, 0x6B, 0x7B, 0xE0, 0x83,
    0x83, 0x03, 0x0D, 0x15, 0xA5, 0xAF, 0x8A, 0x13, 0x37, 0x5E, 0x14, 0x83, 0x1D, 0x15, 0x0D, 0x0F,
    0xAE, 0xBE, 0x4D, 0x56, 0xED, 0x18, 0x9C, 0xE6, 0x31, 0x88, 0xBA, 0xC0, 0x46, 0x1C, 0x23, 0x99,
    0x59, 0xF9, 0xC8, 0x2A, 0x08, 0xAB, 0xB0, 0xB0, 0x8A, 0x89, 0x55, 0xF2, 0xBF, 0x41, 0x5A, 0x16,
    0xCD, 0xD9, 0x7C, 0x50, 0x4B, 0x01, 0x02, 0x14, 0x03, 0x14, 0x00, 0x00, 0x08, 0x08, 0x00, 0x00,
    0x00, 0x21, 0x00, 0x19, 0x1C, 0xAD, 0xDB, 0x57, 0x00, 0x00, 0x00, 0xEA, 0x00, 0x00, 0x00, 0x0E,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80, 0x01, 0x00, 0x00, 0x00,
    0x00, 0xD0, 0xBE, 0xD1, 0x82, 0xD1, 0x87, 0xD1, 0x91, 0xD1, 0x82, 0x2E, 0x74, 0x78, 0x74, 0x50,
    0x4B, 0x05, 0x06, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x01, 0x00, 0x3C, 0x00, 0x00, 0x00, 0x83,
    0x00, 0x00, 0x00, 0x6B, 0x00, 0xD0, 0x9A, 0xD0, 0xBE, 0xD0, 0xBC, 0xD0, 0xBC, 0xD0, 0xB5, 0xD0,
    0xBD, 0xD1, 0x82, 0xD0, 0xB0, 0xD1, 0x80, 0xD0, 0xB8, 0xD0, 0xB9, 0x20, 0xD0, 0xB0, 0xD1, 0x80,
    0xD1, 0x85, 0xD0, 0xB8, 0xD0, 0xB2, 0xD0, 0xB0, 0x3A, 0x20, 0xD0, 0xBE, 0xD1, 0x82, 0xD1, 0x87,
    0xD1, 0x91, 0xD1, 0x82, 0xD0, 0xBD, 0xD0, 0xBE, 0xD1, 0x81, 0xD1, 0x82, 0xD1, 0x8C, 0x20, 0xD0,
    0xB7, 0xD0, 0xB0, 0x20, 0x32, 0x30, 0x32, 0x36, 0x20, 0xD0, 0xB3, 0xD0, 0xBE, 0xD0, 0xB4, 0x2C,
    0x20, 0xD0, 0xB2, 0xD1, 0x8B, 0xD0, 0xB3, 0xD1, 0x80, 0xD1, 0x83, 0xD0, 0xB7, 0xD0, 0xBA, 0xD0,
    0xB0, 0x20, 0xD0, 0xBD, 0xD0, 0xBE, 0xD0, 0xBC, 0xD0, 0xB5, 0xD1, 0x80, 0x20, 0x31, 0x34, 0x2E,
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
    0x50, 0x4B, 0x03, 0x04, 0x14, 0x00, 0x08, 0x08, 0x08, 0x00, 0x00, 0x00, 0x21, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0E, 0x00, 0x00, 0x00, 0xD0, 0xBF,
    0xD0, 0xBE, 0xD1, 0x82, 0xD0, 0xBE, 0xD0, 0xBA, 0x2E, 0x74, 0x78, 0x74, 0x65, 0xCE, 0xBD, 0x09,
    0x80, 0x30, 0x14, 0x45, 0xE1, 0x3E, 0xBB, 0x08, 0xC9, 0xCB, 0x9F, 0xF2, 0x96, 0x73, 0x00, 0x3B,
    0x37, 0xB0, 0xB0, 0x0E, 0x41, 0xB1, 0x32, 0x33, 0xDC, 0x6C, 0x64, 0x9D, 0x6B, 0x7B, 0xE0, 0x83,
    0x83, 0x03, 0x0D, 0x15, 0xA5, 0xAF, 0x8A, 0x13, 0x37, 0x5E, 0x14, 0x83, 0x1D, 0x15, 0x0D, 0x0F,
    0xAE, 0xBE, 0x4D, 0x56, 0xED, 0x18, 0x9C, 0xE6, 0x31, 0x88, 0xBA, 0xC0, 0x46, 0x1C, 0x23, 0x99,
    0x59, 0xF9, 0xC8, 0x2A, 0x08, 0xAB, 0xB0, 0xB0, 0x8A, 0x89, 0x55, 0xF2, 0xBF, 0x41, 0x5A, 0x16,
    0xCD, 0xD9, 0x7C, 0x50, 0x4B, 0x07, 0x08, 0x19, 0x1C, 0xAD, 0xDB, 0x57, 0x00, 0x00, 0x00, 0xEA,
    0x00, 0x00, 0x00, 0x50, 0x4B, 0x03, 0x04, 0x14, 0x00, 0x08, 0x08, 0x00, 0x00, 0x00, 0x00, 0x21,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0E, 0x00, 0x00,
    0x00, 0xD1, 0x85, 0xD0, 0xB2, 0xD0, 0xBE, 0xD1, 0x81, 0xD1, 0x82, 0x2E, 0x62, 0x69, 0x6E, 0x0D,
    0x6E, 0xCF, 0x30, 0x91, 0xF2, 0x53, 0xB4, 0x15, 0x76, 0xD7, 0x38, 0x99, 0xFA, 0x5B, 0xBC, 0x1D,
    0x7E, 0xDF, 0x40, 0xA1, 0x02, 0x63, 0xC4, 0x50, 0x4B, 0x07, 0x08, 0x88, 0x04, 0x12, 0x95, 0x18,
    0x00, 0x00, 0x00, 0x18, 0x00, 0x00, 0x00, 0x50, 0x4B, 0x01, 0x02, 0x14, 0x03, 0x14, 0x00, 0x08,
    0x08, 0x08, 0x00, 0x00, 0x00, 0x21, 0x00, 0x19, 0x1C, 0xAD, 0xDB, 0x57, 0x00, 0x00, 0x00, 0xEA,
    0x00, 0x00, 0x00, 0x0E, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80,
    0x01, 0x00, 0x00, 0x00, 0x00, 0xD0, 0xBF, 0xD0, 0xBE, 0xD1, 0x82, 0xD0, 0xBE, 0xD0, 0xBA, 0x2E,
    0x74, 0x78, 0x74, 0x50, 0x4B, 0x01, 0x02, 0x14, 0x03, 0x14, 0x00, 0x08, 0x08, 0x00, 0x00, 0x00,
    0x00, 0x21, 0x00, 0x88, 0x04, 0x12, 0x95, 0x18, 0x00, 0x00, 0x00, 0x18, 0x00, 0x00, 0x00, 0x0E,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80, 0x01, 0x93, 0x00, 0x00,
    0x00, 0xD1, 0x85, 0xD0, 0xB2, 0xD0, 0xBE, 0xD1, 0x81, 0xD1, 0x82, 0x2E, 0x62, 0x69, 0x6E, 0x50,
    0x4B, 0x05, 0x06, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x02, 0x00, 0x78, 0x00, 0x00, 0x00, 0xE7,
    0x00, 0x00, 0x00, 0x00, 0x00,
];

/// Архив, у которого поле Zip64 стоит в ЛОКАЛЬНОМ заголовке: `force_zip64`
/// заставляет zipfile написать его независимо от размера. В каталоге
/// маленькой записи поля Zip64 при этом нет — длины extra у двух
/// заголовков одной записи законно разные.
const REF_ZIP64: [u8; 178] = [
    0x50, 0x4B, 0x03, 0x04, 0x2D, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x00, 0x21, 0x00, 0x88, 0x04,
    0x12, 0x95, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x12, 0x00, 0x14, 0x00, 0xD0, 0xB1,
    0xD0, 0xBE, 0xD0, 0xBB, 0xD1, 0x8C, 0xD1, 0x88, 0xD0, 0xBE, 0xD0, 0xB9, 0x2E, 0x62, 0x69, 0x6E,
    0x01, 0x00, 0x10, 0x00, 0x18, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x18, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x0D, 0x6E, 0xCF, 0x30, 0x91, 0xF2, 0x53, 0xB4, 0x15, 0x76, 0xD7, 0x38,
    0x99, 0xFA, 0x5B, 0xBC, 0x1D, 0x7E, 0xDF, 0x40, 0xA1, 0x02, 0x63, 0xC4, 0x50, 0x4B, 0x01, 0x02,
    0x2D, 0x03, 0x2D, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x00, 0x21, 0x00, 0x88, 0x04, 0x12, 0x95,
    0x18, 0x00, 0x00, 0x00, 0x18, 0x00, 0x00, 0x00, 0x12, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x80, 0x01, 0x00, 0x00, 0x00, 0x00, 0xD0, 0xB1, 0xD0, 0xBE, 0xD0, 0xBB,
    0xD1, 0x8C, 0xD1, 0x88, 0xD0, 0xBE, 0xD0, 0xB9, 0x2E, 0x62, 0x69, 0x6E, 0x50, 0x4B, 0x05, 0x06,
    0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x01, 0x00, 0x40, 0x00, 0x00, 0x00, 0x5C, 0x00, 0x00, 0x00,
    0x00, 0x00,
];

/// Пустой архив: одна запись конца каталога и ничего больше.
const REF_EMPTY: [u8; 22] = [
    0x50, 0x4B, 0x05, 0x06, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];

/// CRC-32 через `flate2::Crc` — для сверки эталонных архивов.
fn crc32(data: &[u8]) -> u32 {
    let mut crc = flate2::Crc::new();
    crc.update(data);
    crc.sum()
}

/// Собрать минимальный ZIP-архив из пар (имя, данные), допуская дубликаты
/// имён. Крейт `zip` отвергает дубликаты, а тестам `build_items` нужен
/// архив с повторяющимися именами. Сжатие — deflate через `flate2`,
/// метод выбирается по результату: если deflate не ужимает, данные
/// лежат как есть (метод 0).
fn build_test_archive(entries: &[(&str, &[u8])]) -> Vec<u8> {
    use std::io::Write as _;
    let mut out: Vec<u8> = Vec::new();
    let mut central: Vec<u8> = Vec::new();
    let mut offsets: Vec<u32> = Vec::new();
    for (name, data) in entries {
        let offset = out.len() as u32;
        offsets.push(offset);
        let crc = crc32(data);
        let deflated = {
            let mut enc = flate2::write::DeflateEncoder::new(
                Vec::with_capacity(data.len() / 2),
                flate2::Compression::default(),
            );
            let _ = enc.write_all(data);
            enc.finish().unwrap_or_else(|_| data.to_vec())
        };
        let (method, packed) = if deflated.len() < data.len() {
            (8u16, deflated)
        } else {
            (0u16, data.to_vec())
        };
        let name_bytes = name.as_bytes();
        let name_len = u16::try_from(name_bytes.len()).unwrap();
        // Локальный заголовок
        out.extend_from_slice(&0x0403_4B50u32.to_le_bytes());
        out.extend_from_slice(&20u16.to_le_bytes());
        out.extend_from_slice(&0x0800u16.to_le_bytes()); // UTF-8
        out.extend_from_slice(&method.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes()); // время
        out.extend_from_slice(&0u16.to_le_bytes()); // дата
        out.extend_from_slice(&crc.to_le_bytes());
        out.extend_from_slice(&(packed.len() as u32).to_le_bytes());
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out.extend_from_slice(&name_len.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes()); // extra
        out.extend_from_slice(name_bytes);
        out.extend_from_slice(&packed);
        // Запись каталога
        central.extend_from_slice(&0x0201_4B50u32.to_le_bytes());
        central.extend_from_slice(&20u16.to_le_bytes());
        central.extend_from_slice(&20u16.to_le_bytes());
        central.extend_from_slice(&0x0800u16.to_le_bytes());
        central.extend_from_slice(&method.to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes());
        central.extend_from_slice(&crc.to_le_bytes());
        central.extend_from_slice(&(packed.len() as u32).to_le_bytes());
        central.extend_from_slice(&(data.len() as u32).to_le_bytes());
        central.extend_from_slice(&name_len.to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes()); // extra
        central.extend_from_slice(&0u16.to_le_bytes()); // комментарий
        central.extend_from_slice(&0u16.to_le_bytes()); // диск
        central.extend_from_slice(&0u16.to_le_bytes()); // внутр. атрибуты
        central.extend_from_slice(&0u32.to_le_bytes()); // внешние атрибуты
        central.extend_from_slice(&offset.to_le_bytes());
        central.extend_from_slice(name_bytes);
    }
    let cd_start = out.len() as u32;
    let cd_size = central.len() as u32;
    let count = entries.len() as u16;
    out.extend_from_slice(&central);
    out.extend_from_slice(&0x0605_4B50u32.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&count.to_le_bytes());
    out.extend_from_slice(&count.to_le_bytes());
    out.extend_from_slice(&cd_size.to_le_bytes());
    out.extend_from_slice(&cd_start.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out
}

/// Разобрать центральный каталог вручную, сохраняя дубликаты имён.
/// Крейт `zip` дедуплицирует через `IndexMap`, а тестам `build_items`
/// нужны все записи. Разбор минимальный: только поля, нужные
/// [`RawEntry`] и [`build_items`].
fn parse_entries_from_central(data: &[u8]) -> Vec<RawEntry> {
    let mut entries = Vec::new();
    let mut at = 0;
    while at + CENTRAL_HEADER_LEN <= data.len() {
        if u32::from_le_bytes([data[at], data[at + 1], data[at + 2], data[at + 3]]) != SIG_CENTRAL {
            at += 1;
            continue;
        }
        let method = u16::from_le_bytes([data[at + 10], data[at + 11]]);
        let mod_time = u16::from_le_bytes([data[at + 12], data[at + 13]]);
        let mod_date = u16::from_le_bytes([data[at + 14], data[at + 15]]);
        let crc = u32::from_le_bytes([data[at + 16], data[at + 17], data[at + 18], data[at + 19]]);
        let compressed_size =
            u32::from_le_bytes([data[at + 20], data[at + 21], data[at + 22], data[at + 23]]) as u64;
        let size =
            u32::from_le_bytes([data[at + 24], data[at + 25], data[at + 26], data[at + 27]]) as u64;
        let name_len = usize::from(u16::from_le_bytes([data[at + 28], data[at + 29]]));
        let extra_len = usize::from(u16::from_le_bytes([data[at + 30], data[at + 31]]));
        let comment_len = usize::from(u16::from_le_bytes([data[at + 32], data[at + 33]]));
        let flags = u16::from_le_bytes([data[at + 8], data[at + 9]]);
        let name_at = at + CENTRAL_HEADER_LEN;
        if name_at + name_len > data.len() {
            break;
        }
        let name = data[name_at..name_at + name_len].to_vec();
        let is_dir = name.last() == Some(&b'/');
        entries.push(RawEntry {
            name,
            method,
            crc,
            compressed_size,
            size,
            encrypted: flags & FLAG_ENCRYPTED != 0,
            is_directory: is_dir,
            mod_time,
            mod_date,
        });
        at = name_at + name_len + extra_len + comment_len;
    }
    entries
}

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

    let (entries, _) = parse_archive(&bytes).expect("наш же архив обязан разобраться");
    let names: Vec<&str> = entries
        .iter()
        .map(|e| std::str::from_utf8(e.name_bytes()).expect("бит 11 наш писатель ставит всегда"))
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

    assert_eq!(
        entries[0].method(),
        METHOD_DEFLATED,
        "разметка обязана сжаться"
    );
    assert!(entries[0].compressed_size() < entries[0].size());
    // Шум (несжимаемые данные) теперь тоже хранится через deflate —
    // крейт `zip` не выбирает Stored автоматически, поэтому сжатый
    // размер может быть больше оригинального.
    assert!(entries[3].is_directory(), "имя со слэшем — это каталог");
    assert!(!entries[0].is_directory());
    assert!(entries.iter().all(|e| !e.is_encrypted()));

    assert_eq!(
        read_entry(&bytes, 0, &entries[0]).expect("метод 8"),
        text.as_bytes()
    );
    assert_eq!(
        read_entry(&bytes, 1, &entries[1]).expect("метод 8"),
        "Организация «Ромашка»".as_bytes()
    );
    assert_eq!(read_entry(&bytes, 2, &entries[2]).expect("метод 0"), noise);
    assert_eq!(
        read_entry(&bytes, 3, &entries[3]).expect("пустая запись"),
        Vec::<u8>::new()
    );
    assert_eq!(entries.len(), 4, "записей четыре, не больше");
}

#[test]
fn the_reference_archive_with_stored_and_deflated_entries_reads() {
    let (entries, _) = parse_archive(&REF_MIXED).expect("эталон zipfile обязан разобраться");
    assert_eq!(entries.len(), 2);

    let entry = &entries[0];
    assert_eq!(
        std::str::from_utf8(entry.name_bytes()).ok(),
        Some("накладная.txt")
    );
    assert_eq!(entry.method(), METHOD_DEFLATED);
    assert_eq!(entry.size(), reference_text().len() as u64);
    assert!(entry.compressed_size() < entry.size());
    assert_eq!(
        entry.crc(),
        crc32(&reference_text()),
        "каталожная сумма — это сумма распакованных данных"
    );
    assert!(!entry.is_encrypted());
    assert_eq!(
        read_entry(&REF_MIXED, 0, &entries[0]).expect("метод 8"),
        reference_text()
    );

    let entry = &entries[1];
    assert_eq!(
        std::str::from_utf8(entry.name_bytes()).ok(),
        Some("шум.bin")
    );
    assert_eq!(entry.method(), METHOD_STORED);
    assert_eq!(entry.size(), reference_noise().len() as u64);
    assert_eq!(entry.compressed_size(), entry.size());
    assert_eq!(entry.crc(), crc32(&reference_noise()));
    assert_eq!(
        read_entry(&REF_MIXED, 1, &entries[1]).expect("метод 0"),
        reference_noise()
    );
}

/// Комментарий отодвигает запись конца каталога от конца файла, а
/// подтверждается кандидат тем, что объявленная в нём длина комментария
/// доводит ровно до конца: сигнатуры мало, те же четыре байта могут
/// лежать и в самом комментарии.
#[test]
fn a_comment_does_not_hide_the_end_of_directory_record() {
    let (entries, _) = parse_archive(&REF_COMMENT).expect("эталон с комментарием разбирается");
    assert_eq!(entries.len(), 1);
    assert_eq!(
        std::str::from_utf8(entries[0].name_bytes()).ok(),
        Some("отчёт.txt")
    );
    assert_eq!(
        read_entry(&REF_COMMENT, 0, &entries[0]).expect("метод 8"),
        reference_text()
    );

    let mut data = REF_COMMENT.to_vec();
    let fake = data.len() - 30;
    data[fake..fake + 4].copy_from_slice(&SIG_EOCD.to_le_bytes());
    let last = (0..=data.len() - 4).rfind(|&at| data[at..at + 4] == SIG_EOCD.to_le_bytes());
    assert_eq!(
        last,
        Some(fake),
        "подложная сигнатура обязана быть последней в файле"
    );
    let (entries, _) = parse_archive(&data).expect("настоящая запись всё равно находится");
    assert_eq!(
        read_entry(&data, 0, &entries[0]).expect("метод 8"),
        reference_text()
    );
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
    let (entries, _) = parse_archive(&REF_DESCRIPTOR).expect("эталон разбирается");
    assert_eq!(entries.len(), 2);
    assert_eq!(
        read_entry(&REF_DESCRIPTOR, 0, &entries[0]).expect("метод 8"),
        reference_text()
    );
    assert_eq!(
        read_entry(&REF_DESCRIPTOR, 1, &entries[1]).expect("метод 0"),
        reference_noise()
    );
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

    let (entries, _) = parse_archive(&REF_ZIP64).expect("эталон Zip64 разбирается");
    let entry = &entries[0];
    assert_eq!(
        std::str::from_utf8(entry.name_bytes()).ok(),
        Some("большой.bin")
    );
    assert_eq!(entry.size(), reference_noise().len() as u64);
    assert_eq!(
        read_entry(&REF_ZIP64, 0, &entries[0]).expect("метод 0"),
        reference_noise()
    );
}

/// Значения поля 0x0001 идут в фиксированном порядке, но присутствуют
/// только те, что в записи выставлены в максимум. У четырёх записей
/// эталона набор разный, поэтому разбор по фиксированному размеру блока
/// здесь заведомо разъезжается: у второй, третьей и четвёртой записи
/// смещение локального заголовка тоже лежит в поле, и без него запись
/// просто не найти, а у четвёртой оно в поле ОДНО — при настоящих
/// размерах в самой записи, так что позиционный читатель принял бы это
/// Номер диска, вынесенный в поле 0x0001, читается по тому же правилу —
#[test]
fn an_empty_archive_has_no_entries() {
    let (entries, _) = parse_archive(&REF_EMPTY).expect("пустой эталон разбирается");
    assert!(entries.is_empty());
    let bytes = ZipWriter::new().finish();
    let (entries, _) = parse_archive(&bytes).expect("наш пустой архив разбирается");
    assert!(entries.is_empty());
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
        assert!(parse_archive(input).is_err(), "это не архив: {:?}", input);
    }
}

/// Локатор нашёлся, а записи конца каталога Zip64 по нему нет — это уже
/// порча, а не архив без Zip64: молча вернуться к 32-битным полям здесь
/// Шифрование распознаётся и называется — до всякого чтения данных.
/// Каталог при этом разбирается, и незашифрованные записи читаются.
#[test]
fn an_encrypted_entry_is_reported_before_its_data_are_touched() {
    let mut data = REF_MIXED.to_vec();
    let central = central_at(&data, "накладная.txt".as_bytes());
    set_flags(&mut data, central, FLAG_ENCRYPTED, 0);

    let (entries, _) = parse_archive(&data).expect("каталог зашифрованного архива разбирается");
    assert!(entries[0].is_encrypted());
    assert!(!entries[1].is_encrypted());
    let e = read_entry(&data, 0, &entries[0]).expect_err("расшифровки здесь нет");
    assert!(
        e.to_string().contains("зашифрован"),
        "непонятный текст: {e}"
    );
    assert_eq!(
        read_entry(&data, 1, &entries[1]).expect("вторая запись не зашифрована"),
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
    data[local + LOCAL_HEADER_LEN..local + LOCAL_HEADER_LEN + CP866.len()].copy_from_slice(&CP866);
    set_flags(&mut data, central, 0, FLAG_UTF8_NAME);

    let (entries, _) = parse_archive(&data).expect("однобайтовое имя разбору не мешает");
    let entry = &entries[1];
    assert_eq!(
        std::str::from_utf8(entry.name_bytes()).ok(),
        None,
        "декодировать нечем — кодовая страница не объявлена"
    );
    assert_eq!(entry.name_bytes(), CP866);
    assert!(!entry.is_directory());
    assert_eq!(
        read_entry(&data, 1, &entries[1]).expect("данные не тронуты"),
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
    let (entries, _) = parse_archive(&data).expect("испорченное имя разбору не мешает");
    let entry = &entries[1];
    assert_eq!(
        std::str::from_utf8(entry.name_bytes()).ok(),
        None,
        "это не UTF-8"
    );
    assert_eq!(entry.name_bytes()[0], 0xFF);
    assert_eq!(
        read_entry(&data, 1, &entries[1]).expect("данные не тронуты"),
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
/// из перечисленных имён. Дубликаты допускаются — для этого используется
/// [`build_test_archive`], а не [`ZipWriter`], который отвергает их.
/// Крейт `zip` тоже дедуплицирует записи по имени, поэтому разбор
/// каталога идёт вручную через [`parse_entries_from_central`].
fn names_of(names: &[&str]) -> Vec<(String, String)> {
    let entries: Vec<(&str, &[u8])> = names.iter().map(|n| (*n, b"x" as &[u8])).collect();
    let bytes = build_test_archive(&entries);
    let entries = parse_entries_from_central(&bytes);
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
    let (entries, _) = parse_archive(&bytes).unwrap();
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
    data[local + LOCAL_HEADER_LEN..local + LOCAL_HEADER_LEN + CP866.len()].copy_from_slice(&CP866);
    set_flags(&mut data, central, 0, FLAG_UTF8_NAME);

    let (entries, _) = parse_archive(&data).unwrap();
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
    BslValue::Str(BslString::from_str(path.to_str().unwrap()))
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
    let dir = std::env::temp_dir().join(format!("open-bsl-zip-out-{}-{name}", std::process::id()));
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
    let found = find(&items, &BslValue::Str(BslString::from_str("ШУМ.BIN"))).unwrap();
    assert_eq!(prop(&found, "Имя"), "шум.bin");
    let missing = find(&items, &BslValue::Str(BslString::from_str("нет.txt"))).unwrap();
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
        &BslValue::Str(BslString::from_str("отчёт:2026.txt")),
    )
    .unwrap();
    assert_eq!(prop(&by_original, "Имя"), "отчёт_2026.txt");
    let by_shown = find(
        &items,
        &BslValue::Str(BslString::from_str("отчёт_2026.txt")),
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
    let source = BslValue::Str(BslString::from_str(path.to_str().unwrap()));
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
    let arg = |p: &std::path::Path| BslValue::Str(BslString::from_str(p.to_str().unwrap()));

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
        &[arg(&flat), BslValue::Enum(EnumValue::DontRestorePaths)],
    )
    .expect("плоская распаковка");
    assert!(flat.join("вложенный.txt").is_file());
    assert!(!flat.join("папка").exists(), "каталогов быть не должно");

    // Пустая строка каталога — ошибка, а `Неопределено` режимом не
    // считается (оба измерены).
    assert!(extract_all(&reader, &[BslValue::Str(BslString::from_str(""))]).is_err());
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
    let arg = BslValue::Str(BslString::from_str(dir.to_str().unwrap()));

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
    let arg = BslValue::Str(BslString::from_str(dir.to_str().unwrap()));
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
    let arg = BslValue::Str(BslString::from_str(dir.to_str().unwrap()));
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
        &BslValue::Str(BslString::from_str("один.txt")),
    )
    .unwrap();
    assert_eq!(prop(&fresh, "Имя"), "один.txt");
}

/// Размеры Zip64 приходят из каталога произвольными восемью байтами:
/// размер со старшим битом обязан остаться ПОЛОЖИТЕЛЬНЫМ числом, а не
/// завернуться в знак. Поле 0x0001 приписано к записи руками — здешний
/// писатель extra не пишет, а `zipfile` выносит размеры в него только
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
    let source = BslValue::Str(BslString::from_str(path.to_str().unwrap()));

    let ok = new_archive_reader(
        false,
        &source,
        &BslValue::Undefined,
        &BslValue::Enum(EnumValue::ArchiveTypeZip),
    )
    .expect("ZIP поддержан");
    assert_eq!(ok.type_name(), "ЧтениеФайлаАрхива");

    let e = new_archive_reader(
        false,
        &source,
        &BslValue::Undefined,
        &BslValue::Enum(EnumValue::ArchiveTypeTar),
    )
    .expect_err("TAR здесь не читается");
    assert!(e.to_string().contains("не поддерживается"), "текст: {e}");
    assert!(
        new_archive_reader(
            false,
            &source,
            &BslValue::Undefined,
            &BslValue::Boolean(true)
        )
        .is_err()
    );
}

/// Испорченный вход не открывается вовсе — ошибка приходит из
/// конструктора, а не из первой попытки прочитать запись.
#[test]
fn a_broken_archive_fails_in_the_constructor() {
    let dir = std::env::temp_dir().join(format!("open-bsl-zip-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("broken.zip");
    std::fs::write(&path, b"not a zip at all, not even close").unwrap();
    let source = BslValue::Str(BslString::from_str(path.to_str().unwrap()));
    assert!(new_archive_reader(true, &source, &BslValue::Undefined, &BslValue::Undefined).is_err());

    let missing = BslValue::Str(BslString::from_str("/несуществующий/архив.zip"));
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
        &[BslValue::Str(BslString::from_str(target.to_str().unwrap()))],
    )
    .expect("писатель строится")
}

fn str_value(text: &str) -> BslValue {
    BslValue::Str(BslString::from_str(text))
}

/// Имена записей собранного архива — по каталогу, а не по локальным
/// заголовкам.
fn entry_names(bytes: &[u8]) -> Vec<String> {
    let (entries, _) = parse_archive(bytes).expect("свой архив читается");
    entries
        .iter()
        .map(|e| String::from_utf8_lossy(e.name_bytes()).into_owned())
        .collect()
}

/// Собрать архив писателем и вернуть его байты, не трогая файловую
/// систему целью.
fn built(state_owner: &BslValue) -> Vec<u8> {
    writer_binary_data(state_owner)
        .expect("данные отдаются")
        .binary_data_bytes()
        .expect("не двоичные данные")
        .to_vec()
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
            BslValue::Enum(EnumValue::ZipStoreRelativePath),
        ],
    )
    .unwrap();
    assert_eq!(entry_names(&built(&relative)), vec!["f0.txt"]);

    let full = new_archive_writer(true, &[]).unwrap();
    writer_add(&full, &[file, BslValue::Enum(EnumValue::ZipStoreFullPath)]).unwrap();
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
            BslValue::Enum(EnumValue::ZipStoreRelativePath),
            BslValue::Enum(EnumValue::ZipProcessSubdirsRecursively),
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
            BslValue::Enum(EnumValue::ZipDontStorePath),
            BslValue::Enum(EnumValue::ZipProcessSubdirsRecursively),
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
            BslValue::Enum(EnumValue::ZipStoreRelativePath),
            BslValue::Enum(EnumValue::ZipProcessSubdirsRecursively),
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
            BslValue::Enum(EnumValue::ZipStoreRelativePath),
            BslValue::Enum(EnumValue::ZipProcessSubdirsRecursively),
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
            BslValue::Enum(EnumValue::ZipStoreRelativePath),
            BslValue::Enum(EnumValue::ZipProcessSubdirsRecursively),
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
            BslValue::Enum(EnumValue::ZipDontStorePath),
            BslValue::Enum(EnumValue::ZipProcessSubdirsRecursively),
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
            BslValue::Enum(EnumValue::ZipMethodCopy),
        ],
    )
    .unwrap();
    writer_add(&stored, std::slice::from_ref(&file)).unwrap();
    let bytes = built(&stored);
    let (entries, _) = parse_archive(&bytes).unwrap();
    assert_eq!(entries[0].method(), METHOD_STORED);
    assert_eq!(entries[0].compressed_size(), entries[0].size());

    let deflated = new_archive_writer(true, &[]).unwrap();
    writer_add(&deflated, &[file]).unwrap();
    let bytes = built(&deflated);
    let (entries, _) = parse_archive(&bytes).unwrap();
    assert_eq!(entries[0].method(), METHOD_DEFLATED);
    assert_eq!(read_entry(&bytes, 0, &entries[0]).unwrap(), b"nol");
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
            BslValue::Enum(EnumValue::ZipMethodBzip2),
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
            BslValue::Enum(EnumValue::ZipEncryptionAes256),
        ],
    )
    .expect_err("шифрования нет");
    assert!(
        encryption.to_string().contains("не поддерживается"),
        "текст: {encryption}"
    );

    // Уровень сжатия, наоборот, принимается — он на байты не влияет.
    assert!(
        new_archive_writer(
            true,
            &[
                BslValue::Undefined,
                BslValue::Undefined,
                BslValue::Undefined,
                BslValue::Undefined,
                BslValue::Enum(EnumValue::ZipLevelMaximal),
            ]
        )
        .is_ok()
    );
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
            BslValue::Enum(EnumValue::ArchiveTypeZip),
            str_value("комментарий"),
        ],
    )
    .expect("ZIP и комментарий");
    assert_eq!(ok.type_name(), "ЗаписьФайлаАрхива");
    let bytes = built(&ok);
    let (_, comment) = parse_archive(&bytes).unwrap();
    assert_eq!(String::from_utf8_lossy(&comment), "комментарий");

    assert!(
        new_archive_writer(
            false,
            &[
                BslValue::Undefined,
                BslValue::Undefined,
                BslValue::Enum(EnumValue::ArchiveTypeTar)
            ]
        )
        .is_err()
    );
    assert!(
        new_archive_writer(
            false,
            &[
                BslValue::Undefined,
                BslValue::Undefined,
                BslValue::Undefined,
                BslValue::Undefined,
                str_value("пятый")
            ]
        )
        .is_err()
    );

    // Восьмое место у архивного писателя ЕСТЬ (у zip-варианта его нет)
    // и типизировано так же пусто. Резолвер не пускает сюда девятый
    // аргумент, поэтому границу проверяет тест в bsl-sema, а здесь —
    // сама пара «пустое принято, непустое отвергнуто».
    let empty_tail = vec![BslValue::Undefined; 8];
    assert!(new_archive_writer(false, &empty_tail).is_ok());
    let mut eighth = empty_tail;
    eighth[7] = BslValue::Enum(EnumValue::ArchiveTypeZip);
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
            BslValue::Enum(EnumValue::ZipStoreRelativePath),
            BslValue::Enum(EnumValue::ZipProcessSubdirsRecursively),
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
            BslValue::Enum(EnumValue::ZipStoreRelativePath),
            BslValue::Enum(EnumValue::ZipProcessSubdirsRecursively),
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
    let (_, comment) = parse_archive(&bytes).unwrap();
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
            BslValue::Enum(EnumValue::ZipStoreRelativePath),
            BslValue::Enum(EnumValue::ZipProcessSubdirsRecursively),
        ],
    )
    .unwrap();
    let bytes = built(&w);
    let (entries, _) = parse_archive(&bytes).unwrap();
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

/// In-memory файловая система проводит ВСЕ обращения bsl-zip к диску
/// (ABI-G0): архив пишется из дерева в памяти и распаковывается обратно в
/// память, реального диска не касаясь. Так проверяется, что
/// `Добавить`/`Записать`/`Извлечь` ходят в файловую систему СЕССИИ
/// (`metadata`, `read_dir`, `read`, `write`, `create_dir_all`), а не в
/// `std::fs`.
#[test]
fn the_archive_goes_through_the_session_file_system() {
    use std::collections::{HashMap, HashSet};

    #[derive(Debug, Default)]
    struct MemFs {
        files: RefCell<HashMap<String, Vec<u8>>>,
        dirs: RefCell<HashSet<String>>,
    }

    fn not_found(path: &str) -> std::io::Error {
        std::io::Error::new(std::io::ErrorKind::NotFound, path.to_string())
    }

    impl bsl_rt::FileSystem for MemFs {
        fn read(&self, path: &str) -> std::io::Result<Vec<u8>> {
            self.files
                .borrow()
                .get(path)
                .cloned()
                .ok_or_else(|| not_found(path))
        }

        fn write(&self, path: &str, data: &[u8]) -> std::io::Result<()> {
            self.files
                .borrow_mut()
                .insert(path.to_string(), data.to_vec());
            Ok(())
        }

        fn metadata(&self, path: &str) -> std::io::Result<bsl_rt::FileMetadata> {
            if self.dirs.borrow().contains(path.trim_end_matches('/')) {
                Ok(bsl_rt::FileMetadata::directory(Some(0)))
            } else if self.files.borrow().contains_key(path) {
                Ok(bsl_rt::FileMetadata::file(Some(0)))
            } else {
                Err(not_found(path))
            }
        }

        fn read_dir<'fs>(
            &'fs self,
            path: &str,
        ) -> std::io::Result<Box<dyn Iterator<Item = std::io::Result<bsl_rt::DirEntry>> + 'fs>>
        {
            if !self.dirs.borrow().contains(path.trim_end_matches('/')) {
                return Err(not_found(path));
            }
            let prefix = format!("{}/", path.trim_end_matches('/'));
            let mut out: Vec<bsl_rt::DirEntry> = Vec::new();
            let immediate = |full: &str, is_dir: bool| -> Option<bsl_rt::DirEntry> {
                full.strip_prefix(&prefix).and_then(|rest| {
                    (!rest.is_empty() && !rest.contains('/'))
                        .then(|| bsl_rt::DirEntry::new(rest.to_string(), is_dir))
                })
            };
            for f in self.files.borrow().keys() {
                out.extend(immediate(f, false));
            }
            for d in self.dirs.borrow().iter() {
                out.extend(immediate(d, true));
            }
            out.sort_by(|a, b| a.name().cmp(b.name()));
            Ok(Box::new(out.into_iter().map(Ok)))
        }

        fn create_dir_all(&self, path: &str) -> std::io::Result<()> {
            let mut acc = String::new();
            for comp in path.split('/').filter(|c| !c.is_empty()) {
                acc.push('/');
                acc.push_str(comp);
                self.dirs.borrow_mut().insert(acc.clone());
            }
            Ok(())
        }

        fn open(
            &self,
            path: &str,
            _options: bsl_rt::FileOpenOptions,
        ) -> std::io::Result<Box<dyn bsl_rt::FileHandle>> {
            // bsl-zip работает «файлом целиком»: дескриптор ему не нужен.
            Err(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                path.to_string(),
            ))
        }
    }

    let mem = MemFs::default();
    mem.dirs.borrow_mut().insert("/т".to_string());
    mem.files
        .borrow_mut()
        .insert("/т/a.txt".to_string(), b"AAA".to_vec());
    mem.files
        .borrow_mut()
        .insert("/т/b.txt".to_string(), b"BBB".to_vec());

    let sv = |s: &str| BslValue::Str(BslString::from_str(s));

    // Записать архив из каталога в памяти: metadata + read_dir + read + write.
    let writer = new_archive_writer(true, &[sv("/архив.zip")]).unwrap();
    super::writer_add(writer_of(&writer), &mem, &[sv("/т/*")]).unwrap();
    super::writer_write(writer_of(&writer), &mem).unwrap();

    let bytes = mem
        .files
        .borrow()
        .get("/архив.zip")
        .cloned()
        .expect("архив лёг в память");
    assert!(bytes.starts_with(b"PK"), "подпись ZIP");
    assert!(
        !std::path::Path::new("/архив.zip").exists(),
        "реального диска работа не касалась"
    );

    // Распаковать обратно в память: read + create_dir_all + write.
    let reader = new_archive_reader(
        true,
        &BslValue::Undefined,
        &BslValue::Undefined,
        &BslValue::Undefined,
    )
    .unwrap();
    super::open(reader_of(&reader), &mem, &[sv("/архив.zip")]).unwrap();
    super::extract_all(
        &reader_of(&reader).state,
        &mem,
        "ЧтениеZipФайла",
        &[sv("/распак")],
    )
    .unwrap();

    let files = mem.files.borrow();
    assert!(
        files
            .keys()
            .any(|k| k.starts_with("/распак") && k.ends_with("a.txt")),
        "a.txt распакован в память"
    );
    assert!(
        files
            .keys()
            .any(|k| k.starts_with("/распак") && k.ends_with("b.txt")),
        "b.txt распакован в память"
    );
}
