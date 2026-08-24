//! Потоки: `ПотокВПамяти`, `ФайловыйПоток` и менеджер `ФайловыеПотоки`.
//!
//! Оба потока — один и тот же набор операций над разным носителем, поэтому
//! состояние у них общее ([`StreamData`]), а различает их вариант
//! [`StreamKind`]: `Memory` и `File` — РАЗНЫЕ типы
//! платформы (`Тип("ПотокВПамяти") = Тип("ФайловыйПоток")` даёт «Нет» —
//! измерено), и держать различие в теге объекта дешевле, чем читать поле
//! через `RefCell` в каждом `ТипЗнч`.
//!
//! # Что измерено на 8.3.27
//!
//! **`Размер()` и `ТекущаяПозиция()` — МЕТОДЫ, а `ДоступнаЗапись`,
//! `ДоступноЧтение` и `ДоступноИзменениеПозиции` — СВОЙСТВА.** Угадать это
//! было нельзя: у `ДвоичныеДанные` `Размер()` метод, у
//! `БуферДвоичныхДанных` `Размер` свойство. Проверено обеими формами на
//! каждом из пяти имён.
//!
//! **Закрытый поток жив наполовину.** `Закрыть()` идемпотентно; после него
//! `ТекущаяПозиция()` продолжает отдавать ПОСЛЕДНЮЮ позицию, а три признака
//! доступности — прежние значения, зато `Размер()`, `Перейти`, `Записать` и
//! `Прочитать` отвечают ошибкой. Отсюда `backing: Option<..>`: закрытие
//! отпускает носитель, но не состояние.
//!
//! **`Перейти(Смещение, Точка)` возвращает НОВУЮ позицию числом.** Уход за
//! конец разрешён и размера не меняет (`Перейти(100, Начало)` на
//! шестибайтовом потоке отдаёт 100), а уход за начало ОБРЕЗАЕТСЯ НУЛЁМ, а
//! не отвергается, — измерено от всех трёх точек отсчёта. Дробное смещение
//! при этом отвергается, хотя у `Записать`/`Прочитать` дробное СМЕЩЕНИЕ В
//! БУФЕРЕ принимается и усекается (а дробное КОЛИЧЕСТВО — снова ошибка).
//! Асимметрия неожиданная, но измерена в обе стороны.
//!
//! **Числовой аргумент `Новый ПотокВПамяти(N)` — начальная ЁМКОСТЬ, не
//! предел:** размер такого потока 0, и запись восьми байтов в поток,
//! созданный с двойкой, проходит и даёт размер 8.
//!
//! **`Новый ПотокВПамяти(Буфер)` делит с буфером ПАМЯТЬ.** Запись в поток
//! видна в исходном буфере и наоборот; размер при этом фиксирован размером
//! буфера, и запись за его конец — ошибка. Отсюда [`Backing::Buffer`] с тем
//! же `Rc<RefCell<..>>`, что и у буфера, а не копия байтов.
//!
//! **Дыра допустима**: переход за конец с последующей записью удлиняет
//! поток, а пропущенные байты оказываются нулевыми (измерено и в памяти, и
//! в файле).
//!
//! # Совместимость режима открытия и доступа
//!
//! Снято полной таблицей 6 x 4 на существующем и на отсутствующем файле.
//! Доступ по умолчанию — `ЧтениеИЗапись`. Правило одно: режим, который
//! ПИШЕТ в файл сам по себе, несовместим с `ДоступКФайлу.Чтение`, и
//! «сам по себе» здесь считается по факту, а не по названию:
//!
//! * `Открыть` — файл обязан существовать, записи не делает, поэтому
//!   работает с любым доступом;
//! * `ОткрытьИлиСоздать` — с `Чтение` работает на СУЩЕСТВУЮЩЕМ файле и
//!   отказывает на отсутствующем: создание требует записи;
//! * `Создать` (обрезает), `Обрезать` и `Дописать` — всегда требуют записи,
//!   даже когда файл уже есть;
//! * `СоздатьНовый` — требует записи и отказывает, если файл уже есть;
//! * `Обрезать` — отказывает, если файла НЕТ (в отличие от `Создать`).

use std::cell::RefCell;

use std::io::{Read, Seek, SeekFrom, Write};
use std::rc::Rc;

use bsl_rt::{
    Arity, BslNumber, BslValue, ByteStreamProtocol, CallContext, EnumValue, FileCreate, FileHandle,
    FileOpenOptions, FileSystem, MethodDescriptor, ObjectProtocol, PropertyDescriptor, RtError,
    RtResult, TypeDescriptor,
};

/// Режим открытия файла — член `РежимОткрытияФайла`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileOpenMode {
    Open,
    OpenOrCreate,
    Create,
    CreateNew,
    Truncate,
    Append,
}

impl FileOpenMode {
    /// Член перечисления — в режим. `None` означает член ЧУЖОГО
    /// перечисления: платформа такой аргумент отвергает (измерено на
    /// `Новый ФайловыйПоток(Путь, ПорядокБайтов.LittleEndian)`).
    pub fn from_enum(e: EnumValue) -> Option<Self> {
        match e {
            EnumValue::FileOpenModeOpen => Some(FileOpenMode::Open),
            EnumValue::FileOpenModeOpenOrCreate => Some(FileOpenMode::OpenOrCreate),
            EnumValue::FileOpenModeCreate => Some(FileOpenMode::Create),
            EnumValue::FileOpenModeCreateNew => Some(FileOpenMode::CreateNew),
            EnumValue::FileOpenModeTruncate => Some(FileOpenMode::Truncate),
            EnumValue::FileOpenModeAppend => Some(FileOpenMode::Append),
            _ => None,
        }
    }

    /// Обязателен ли доступ на запись при таком режиме. `exists` — есть ли
    /// файл сейчас: у `ОткрытьИлиСоздать` ответ зависит именно от этого
    /// (измерено обеими половинами таблицы).
    fn needs_write(self, exists: bool) -> bool {
        match self {
            FileOpenMode::Open => false,
            FileOpenMode::OpenOrCreate => !exists,
            FileOpenMode::Create
            | FileOpenMode::CreateNew
            | FileOpenMode::Truncate
            | FileOpenMode::Append => true,
        }
    }
}

/// Доступ к файлу — член `ДоступКФайлу`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileAccess {
    Read,
    Write,
    ReadWrite,
}

impl FileAccess {
    /// Член перечисления — в доступ; `None` — член чужого перечисления.
    pub fn from_enum(e: EnumValue) -> Option<Self> {
        match e {
            EnumValue::FileAccessRead => Some(FileAccess::Read),
            EnumValue::FileAccessWrite => Some(FileAccess::Write),
            EnumValue::FileAccessReadAndWrite => Some(FileAccess::ReadWrite),
            _ => None,
        }
    }

    fn can_read(self) -> bool {
        matches!(self, FileAccess::Read | FileAccess::ReadWrite)
    }

    fn can_write(self) -> bool {
        matches!(self, FileAccess::Write | FileAccess::ReadWrite)
    }
}

/// Точка отсчёта у `Перейти` — член `ПозицияВПотоке`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamOrigin {
    Begin,
    Current,
    End,
}

impl StreamOrigin {
    /// Член перечисления — в точку отсчёта; `None` — член чужого
    /// перечисления (измерено: `Перейти(0, ПорядокБайтов.LittleEndian)`
    /// платформа отвергает).
    pub fn from_enum(e: EnumValue) -> Option<Self> {
        match e {
            EnumValue::StreamPositionBegin => Some(StreamOrigin::Begin),
            EnumValue::StreamPositionCurrent => Some(StreamOrigin::Current),
            EnumValue::StreamPositionEnd => Some(StreamOrigin::End),
            _ => None,
        }
    }
}

/// Носитель потока. Закрытый поток теряет носитель, но не позицию и не
/// признаки доступности (измерено).
#[derive(Debug)]
enum Backing {
    /// Собственный растущий буфер `ПотокВПамяти`.
    Owned(Vec<u8>),
    /// `Новый ПотокВПамяти(Буфер)` — ТА ЖЕ память, что у буфера, и потому
    /// фиксированный размер.
    Buffer(BslValue),
    /// Открытый файл за переносимым дескриптором (ABI-G0): раньше был голым
    /// `std::fs::File`, теперь `Box<dyn FileHandle>` — тот же контракт
    /// `Read`/`Write`/`Seek` плюс `len`/`close`, но открыть его может и
    /// файловая система сессии, а не только процессная.
    File(Box<dyn FileHandle>),
}

/// Состояние потока — общее для обоих типов.
#[derive(Debug)]
pub struct StreamData {
    backing: Option<Backing>,
    pos: u64,
    can_read: bool,
    can_write: bool,
    can_seek: bool,
}

impl StreamData {
    fn open(&self, op: &'static str) -> RtResult<&Backing> {
        self.backing.as_ref().ok_or(RtError::IoError(format!(
            "поток закрыт, операция «{op}» больше недоступна"
        )))
    }

    fn open_mut(&mut self, op: &'static str) -> RtResult<&mut Backing> {
        self.backing.as_mut().ok_or(RtError::IoError(format!(
            "поток закрыт, операция «{op}» больше недоступна"
        )))
    }

    /// Длина носителя в байтах.
    fn len(&self, op: &'static str) -> RtResult<u64> {
        match self.open(op)? {
            Backing::Owned(bytes) => Ok(bytes.len() as u64),
            Backing::Buffer(buffer) => Ok(buffer
                .binary_buffer_len()
                .expect("в носитель попадает только буфер")
                as u64),
            Backing::File(file) => file
                .len()
                .map_err(|e| RtError::IoError(format!("Размер: {e}"))),
        }
    }

    /// Текущая позиция. Нужна снаружи модуля читателю и писателю
    /// (`crate::datarw`): те сверяют её со своей ожидаемой и отвергают
    /// операцию, если позицию подвинули мимо них.
    pub(crate) fn position(&self) -> u64 {
        self.pos
    }

    /// Поставить позицию. Нужна `crate::datarw`: `Пропустить` у читателя
    /// именно ПЕРЕВОДИТ позицию, а не вычитывает байты (измерено:
    /// `Пропустить(100)` над шестнадцатью байтами уводит поток на 100), а
    /// неудавшееся чтение целого откатывает её обратно.
    pub(crate) fn set_position(&mut self, pos: u64) {
        self.pos = pos;
    }

    /// Прочитать не больше `count` байтов с текущей позиции, сдвинув её на
    /// фактически прочитанное. У конца отдаёт СКОЛЬКО ЕСТЬ, а не ошибку
    /// (измерено) — вплоть до пустого среза.
    ///
    /// Общая часть [`read`] и чтения в `crate::datarw`: носитель у обоих
    /// один, и различаются они только тем, куда байты потом кладутся.
    pub(crate) fn read_bytes(&mut self, count: usize, op: &'static str) -> RtResult<Vec<u8>> {
        if !self.can_read {
            return Err(RtError::IoError(
                "поток открыт без доступа на чтение".to_string(),
            ));
        }
        let pos = self.pos;
        let chunk: Vec<u8> = match self.open_mut(op)? {
            Backing::Owned(bytes) => slice_from(bytes, pos, count),
            Backing::Buffer(source) => source
                .binary_buffer_slice(pos, count)
                .expect("в носитель попадает только буфер"),
            Backing::File(file) => {
                file.seek(SeekFrom::Start(pos))
                    .map_err(|e| RtError::IoError(format!("{op}: {e}")))?;
                // `vec![0; count]` на неразмещаемом размере ПАНИКУЕТ, а
                // количество приходит из пользовательского текста (`Прочитать`
                // у `ЧтениеДанных`) — значит, отказ обязан быть
                // перехватываемым. Тот же приём, что в
                // `bindata::new_binary_buffer` в `bsl-rt`; после `try_reserve_exact`
                // `resize` уже не размещает память заново и упасть не может.
                let mut chunk = Vec::new();
                chunk
                    .try_reserve_exact(count)
                    .map_err(|_| RtError::TypeError {
                        expected: "Размер, который удаётся разместить в памяти",
                        op,
                    })?;
                chunk.resize(count, 0u8);
                let mut filled = 0;
                while filled < count {
                    let n = file
                        .read(&mut chunk[filled..])
                        .map_err(|e| RtError::IoError(format!("{op}: {e}")))?;
                    if n == 0 {
                        break;
                    }
                    filled += n;
                }
                chunk.truncate(filled);
                chunk
            }
        };
        // Переполниться эта сумма не может: у своей памяти и у потока над
        // буфером `slice_from` за концом отдаёт пустой срез, а файл на такой
        // позиции либо не отдаёт ничего, либо падает ещё на `seek`.
        self.pos = pos + chunk.len() as u64;
        Ok(chunk)
    }

    /// Записать байты в текущую позицию, сдвинув её на длину записанного.
    ///
    /// Общая часть [`write`] и записи в `crate::datarw`. Конец записи
    /// считается `checked_add` ДО любого побочного эффекта: `Перейти`
    /// разрешает уход за конец, позиция может стоять у самого края `u64`, и
    /// в голом сложении это переполнилось бы (см. комментарий у вызова).
    pub(crate) fn write_bytes(&mut self, chunk: &[u8], op: &'static str) -> RtResult<()> {
        if !self.can_write {
            return Err(RtError::IoError(
                "поток открыт без доступа на запись".to_string(),
            ));
        }
        let pos = self.pos;
        // Копировать здесь нечего: платформа 8.3.27.2074 на записи у края
        // `u64` не бросает исключение, а ПАДАЕТ по SIGSEGV, унося сеанс
        // целиком (измерено 08.08.2026, подробности и запрет на такую пробу —
        // в шапке `measure-stream.bsl`). Ловимая ошибка не выбрана из
        // вариантов, а осталась единственным поведением лучше падения.
        let end = pos
            .checked_add(chunk.len() as u64)
            .ok_or(RtError::TypeError {
                expected: "Позиция, умещающаяся в потоке",
                op,
            })?;
        match self.open_mut(op)? {
            Backing::Owned(bytes) => {
                let end = usize::try_from(end).map_err(|_| RtError::TypeError {
                    expected: "Позиция, умещающаяся в памяти",
                    op,
                })?;
                if bytes.len() < end {
                    bytes
                        .try_reserve(end - bytes.len())
                        .map_err(|_| RtError::TypeError {
                            expected: "Размер, который удаётся разместить в памяти",
                            op,
                        })?;
                    bytes.resize(end, 0);
                }
                let start = end - chunk.len();
                bytes[start..end].copy_from_slice(chunk);
            }
            Backing::Buffer(target) => {
                let len = target
                    .binary_buffer_len()
                    .expect("в носитель попадает только буфер");
                if end > len as u64 {
                    return Err(RtError::IndexOutOfBounds {
                        index: end as i64,
                        len,
                    });
                }
                let start = pos as usize;
                target.binary_buffer_write(start, chunk)?;
            }
            Backing::File(file) => {
                file.seek(SeekFrom::Start(pos))
                    .map_err(|e| RtError::IoError(format!("{op}: {e}")))?;
                file.write_all(chunk)
                    .map_err(|e| RtError::IoError(format!("{op}: {e}")))?;
            }
        }
        self.pos = end;
        Ok(())
    }
}

// --- доступ к объекту ---------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StreamKind {
    Memory,
    File,
}

pub(crate) static MEMORY_STREAM_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ПотокВПамяти",
    type_display: "Файловый поток",
    type_names: &["ПотокВПамяти", "MemoryStream"],
};

pub(crate) static FILE_STREAM_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ФайловыйПоток",
    type_display: "Файловый поток",
    type_names: &["ФайловыйПоток", "FileStream"],
};

#[derive(Debug)]
struct StreamObject {
    kind: StreamKind,
    data: Rc<RefCell<StreamData>>,
}

impl ByteStreamProtocol for StreamObject {
    fn position(&self, op: &'static str) -> RtResult<u64> {
        self.data
            .try_borrow()
            .map(|data| data.position())
            .map_err(|_| RtError::IoError(format!("{op}: поток уже занят другой операцией")))
    }

    fn set_position(&self, position: u64, op: &'static str) -> RtResult<()> {
        self.data
            .try_borrow_mut()
            .map_err(|_| RtError::IoError(format!("{op}: поток уже занят другой операцией")))?
            .set_position(position);
        Ok(())
    }

    fn len(&self, op: &'static str) -> RtResult<u64> {
        self.data
            .try_borrow()
            .map_err(|_| RtError::IoError(format!("{op}: поток уже занят другой операцией")))?
            .len(op)
    }

    fn read_bytes(&self, count: usize, op: &'static str) -> RtResult<Vec<u8>> {
        self.data
            .try_borrow_mut()
            .map_err(|_| RtError::IoError(format!("{op}: поток уже занят другой операцией")))?
            .read_bytes(count, op)
    }

    fn write_bytes(&self, bytes: &[u8], op: &'static str) -> RtResult<()> {
        self.data
            .try_borrow_mut()
            .map_err(|_| RtError::IoError(format!("{op}: поток уже занят другой операцией")))?
            .write_bytes(bytes, op)
    }
}

// Обработчики статической таблицы методов потока: получатель приходит от
// вызывающего (VM отдаёт регистр без пересборки обёртки), пары имён — те
// же, что были у этого типа в `BUILTIN_METHOD_NAMES`.
fn stream_write(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    write(receiver, arguments)?;
    Ok(BslValue::Undefined)
}

fn stream_read(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    read(receiver, arguments)
}

fn stream_close(
    receiver: &dyn ObjectProtocol,
    _arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    close(receiver)?;
    Ok(BslValue::Undefined)
}

fn stream_size(
    receiver: &dyn ObjectProtocol,
    _arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    size(receiver)
}

fn stream_current_position(
    receiver: &dyn ObjectProtocol,
    _arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    current_position(receiver)
}

fn stream_seek(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    seek(receiver, arguments)
}

const STREAM_METHODS: &[MethodDescriptor] = &[
    MethodDescriptor::new(&["Записать", "Write"], Arity::exact(3), stream_write),
    MethodDescriptor::new(&["Прочитать", "Read"], Arity::exact(3), stream_read),
    MethodDescriptor::new(&["Закрыть", "Close"], Arity::exact(0), stream_close),
    MethodDescriptor::new(&["Размер", "Size"], Arity::exact(0), stream_size),
    MethodDescriptor::new(
        &["ТекущаяПозиция", "CurrentPosition"],
        Arity::exact(0),
        stream_current_position,
    ),
    MethodDescriptor::new(&["Перейти", "Seek"], Arity::exact(2), stream_seek),
];

impl ObjectProtocol for StreamObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        match self.kind {
            StreamKind::Memory => &MEMORY_STREAM_TYPE,
            StreamKind::File => &FILE_STREAM_TYPE,
        }
    }

    fn property_table(&self) -> &'static [PropertyDescriptor] {
        STREAM_PROPERTIES
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        STREAM_METHODS
    }

    fn byte_stream(&self) -> Option<&dyn ByteStreamProtocol> {
        Some(self)
    }
}

/// Внутренности потока; для любого другого объекта — «метод не применим».
fn data<'a>(v: &'a dyn ObjectProtocol, op: &'static str) -> RtResult<&'a Rc<RefCell<StreamData>>> {
    v.downcast_ref::<StreamObject>()
        .map(|object| &object.data)
        .ok_or_else(|| RtError::MethodNotApplicable {
            method: op,
            receiver: v.type_descriptor().name,
        })
}

// --- преобразование чисел --------------------------------------------------------

/// Целое `0..=u64::MAX` из числа BSL — тот же путь, что у буфера: верхняя
/// половина диапазона не влезает в `i64` и добирается вычитанием `2^63`.
fn to_u64(n: &BslNumber) -> Option<u64> {
    if n.is_negative() || !n.is_integer() {
        return None;
    }
    if let Some(v) = n.to_i64_exact() {
        return u64::try_from(v).ok();
    }
    let half = BslNumber::from_i128(1i128 << 63);
    let rest = n.sub(&half).ok()?;
    let rest = u64::try_from(rest.to_i64_exact()?).ok()?;
    (1u64 << 63).checked_add(rest)
}

fn from_u64(v: u64) -> BslValue {
    BslValue::Number(BslNumber::from_i128(v as i128))
}

/// Смещение В БУФЕРЕ либо количество байтов: неотрицательное целое,
/// умещающееся в память. Дробная часть здесь ОТБРАСЫВАЕТСЯ или
/// отвергается — в зависимости от `truncate`, и это не вкусовщина, а
/// измеренная асимметрия: `Записать(Б, 1.5, 1)` платформа принимает и пишет
/// байт 1, а `Записать(Б, 0, 1.5)` отвергает. У `Прочитать` ровно так же.
fn count_of(v: &BslValue, truncate: bool, op: &'static str) -> RtResult<usize> {
    let bad = || RtError::TypeError {
        expected: "Целое неотрицательное число",
        op,
    };
    let n = match v {
        BslValue::Number(n) => n,
        _ => return Err(bad()),
    };
    let n = if truncate {
        n.trunc_to_scale(0)
    } else {
        if !n.is_integer() {
            return Err(bad());
        }
        n.clone()
    };
    usize::try_from(to_u64(&n).ok_or_else(bad)?).map_err(|_| bad())
}

/// Буфер двоичных данных из аргумента: `Записать`/`Прочитать` работают
/// только с ним (измерено — число и `ДвоичныеДанные` платформа отвергает).
fn buffer_of<'a>(v: &'a BslValue, op: &'static str) -> RtResult<&'a BslValue> {
    if v.binary_buffer_len().is_some() {
        Ok(v)
    } else {
        Err(RtError::TypeError {
            expected: "БуферДвоичныхДанных",
            op,
        })
    }
}

/// Отрезок `[смещение, смещение + количество)` внутри буфера длиной `len`.
/// Выход за буфер — ошибка (измерено: `Записать(Б, 3, 4)` на четырёх байтах
/// отвергается).
fn slice_in_buffer(
    offset: &BslValue,
    count: &BslValue,
    len: usize,
    op: &'static str,
) -> RtResult<(usize, usize)> {
    let offset = count_of(offset, true, op)?;
    let count = count_of(count, false, op)?;
    let end = offset.checked_add(count).ok_or(RtError::BadIndex)?;
    if end > len {
        return Err(RtError::IndexOutOfBounds {
            index: end as i64,
            len,
        });
    }
    Ok((offset, count))
}

// --- конструкторы ------------------------------------------------------------------

/// `Новый ПотокВПамяти([ЁмкостьЛибоБуфер])`.
///
/// Без аргумента — пустой растущий поток. Число — начальная ЁМКОСТЬ (размер
/// при этом ноль, и поток свободно растёт за неё — измерено).
/// `БуферДвоичныхДанных` — поток НАД ним: та же память и фиксированный
/// размер.
///
/// # Errors
///
/// [`RtError::TypeError`], если аргумент не число, не буфер и не
/// `Неопределено` (платформа отвергает `-1`, `2.5`, строку `"4"` и
/// `ДвоичныеДанные` — измерено), а также если ёмкость не удалось разместить
/// в памяти: отказ лучше падения процесса на числе из пользовательского
/// текста.
pub fn new_memory_stream(arg: &BslValue) -> RtResult<BslValue> {
    const OP: &str = "Новый ПотокВПамяти";
    let backing = match arg {
        BslValue::Undefined => Backing::Owned(Vec::new()),
        BslValue::Number(n) => {
            let bad = || RtError::TypeError {
                expected: "Целое неотрицательное число",
                op: OP,
            };
            if !n.is_integer() {
                return Err(bad());
            }
            let capacity = usize::try_from(to_u64(n).ok_or_else(bad)?).map_err(|_| bad())?;
            let mut bytes = Vec::new();
            bytes
                .try_reserve_exact(capacity)
                .map_err(|_| RtError::TypeError {
                    expected: "Ёмкость, которую удаётся разместить в памяти",
                    op: OP,
                })?;
            Backing::Owned(bytes)
        }
        value if value.binary_buffer_len().is_some() => Backing::Buffer(value.clone()),
        _ => {
            return Err(RtError::TypeError {
                expected: "Число либо БуферДвоичныхДанных",
                op: OP,
            });
        }
    };
    Ok(stream_value(
        StreamKind::Memory,
        Rc::new(RefCell::new(StreamData {
            backing: Some(backing),
            pos: 0,
            can_read: true,
            can_write: true,
            can_seek: true,
        })),
    ))
}

/// `Новый ФайловыйПоток(Имя, Режим[, Доступ])`.
///
/// # Errors
///
/// [`RtError::TypeError`], если имя не строка, режим не член
/// `РежимОткрытияФайла`, доступ не член `ДоступКФайлу` (всё измерено — числа
/// платформа отвергает) либо режим несовместим с доступом (см. таблицу в
/// заголовке модуля). [`RtError::IoError`] — отказ операционной системы:
/// нет файла у `Открыть`/`Обрезать`, файл уже есть у `СоздатьНовый`, нет
/// каталога, пустое имя.
pub fn new_file_stream(
    path: &BslValue,
    mode: &BslValue,
    access: &BslValue,
    files: &dyn FileSystem,
) -> RtResult<BslValue> {
    const OP: &str = "Новый ФайловыйПоток";
    let BslValue::Str(path) = path else {
        return Err(RtError::TypeError {
            expected: "Строка",
            op: OP,
        });
    };
    let path = path.to_string();
    let mode = match mode {
        BslValue::Enum(e) => FileOpenMode::from_enum(*e),
        _ => None,
    }
    .ok_or(RtError::TypeError {
        expected: "РежимОткрытияФайла",
        op: OP,
    })?;
    // Доступ по умолчанию — `ЧтениеИЗапись`: измерено сравнением строки
    // «без доступа» с явной `ЧтениеИЗапись` во всей таблице режимов.
    let access = match access {
        BslValue::Undefined => FileAccess::ReadWrite,
        BslValue::Enum(e) => FileAccess::from_enum(*e).ok_or(RtError::TypeError {
            expected: "ДоступКФайлу",
            op: OP,
        })?,
        _ => {
            return Err(RtError::TypeError {
                expected: "ДоступКФайлу",
                op: OP,
            });
        }
    };
    open_file_stream(&path, mode, access, files)
}

/// Поток над ГОТОВЫМИ байтами — носитель для `ЧтениеДанных` поверх
/// `ДвоичныеДанные`: своего потока у такого источника нет, а весь остальной
/// код читателя работает с одним видом носителя.
pub(crate) fn data_over_bytes(bytes: Vec<u8>) -> BslValue {
    stream_value(
        StreamKind::Memory,
        Rc::new(RefCell::new(StreamData {
            backing: Some(Backing::Owned(bytes)),
            pos: 0,
            can_read: true,
            can_write: true,
            can_seek: true,
        })),
    )
}

/// Внутренности файлового потока — то же, что [`open_file_stream`], но без
/// обёртки в `BslValue`: `ЧтениеДанных`/`ЗаписьДанных` по ИМЕНИ ФАЙЛА держат
/// такой поток внутри себя, наружу его не отдавая.
pub(crate) fn data_over_file(
    path: &str,
    mode: FileOpenMode,
    access: FileAccess,
    files: &dyn FileSystem,
) -> RtResult<BslValue> {
    open_file_stream(path, mode, access, files)
}

/// Общая часть конструктора и методов менеджера.
fn open_file_stream(
    path: &str,
    mode: FileOpenMode,
    access: FileAccess,
    files: &dyn FileSystem,
) -> RtResult<BslValue> {
    Ok(stream_value(
        StreamKind::File,
        open_file_data(path, mode, access, files)?,
    ))
}

fn stream_value(kind: StreamKind, data: Rc<RefCell<StreamData>>) -> BslValue {
    BslValue::new_object(StreamObject { kind, data })
}

/// Открытие файла и построение состояния потока — через файловую систему
/// сессии (ABI-G): она приходит вызывающему из `CallContext` (у тестов —
/// процессная `SystemFileSystem`).
fn open_file_data(
    path: &str,
    mode: FileOpenMode,
    access: FileAccess,
    files: &dyn FileSystem,
) -> RtResult<Rc<RefCell<StreamData>>> {
    // Существование файла спрашивается ЗАРАНЕЕ, потому что от него зависит и
    // совместимость режима с доступом (`ОткрытьИлиСоздать` + `Чтение`), и
    // правило создания: с одним лишь `read` создание не запрашивается вовсе,
    // а платформа в этом случае просто открывает уже существующий файл.
    let exists = files.metadata(path).is_ok();
    if mode.needs_write(exists) && !access.can_write() {
        return Err(RtError::IoError(format!(
            "{path}: режим открытия требует доступа на запись"
        )));
    }
    // Правило создания зависит от существования файла ровно как раньше:
    // `ОткрытьИлиСоздать`/`Дописать` создают ТОЛЬКО когда файла нет.
    let create = match mode {
        FileOpenMode::Open | FileOpenMode::Truncate => FileCreate::Never,
        FileOpenMode::OpenOrCreate | FileOpenMode::Append => {
            if exists {
                FileCreate::Never
            } else {
                FileCreate::OpenOrCreate
            }
        }
        FileOpenMode::Create => FileCreate::OpenOrCreate,
        FileOpenMode::CreateNew => FileCreate::CreateNew,
    };
    // Обрезание НЕ создаёт: на отсутствующем файле платформа отвечает
    // ошибкой (измерено), в отличие от `Создать`.
    let truncate = matches!(mode, FileOpenMode::Create | FileOpenMode::Truncate);
    let options = if access.can_read() && access.can_write() {
        FileOpenOptions::read_write(create)
    } else if access.can_write() {
        FileOpenOptions::write(create)
    } else {
        // Только чтение: создание здесь всегда `Never` (проверка выше это
        // гарантирует), обрезание недостижимо.
        FileOpenOptions::read()
    }
    .truncate(truncate);
    let handle = files
        .open(path, options)
        .map_err(|e| RtError::IoError(format!("{path}: {e}")))?;
    // `Дописать` — это только НАЧАЛЬНАЯ позиция в конце файла, а не режим
    // `O_APPEND`: измерено, что после `Перейти(0, Начало)` запись ложится в
    // начало файла и размера не меняет.
    let pos = if mode == FileOpenMode::Append {
        handle
            .len()
            .map_err(|e| RtError::IoError(format!("{path}: {e}")))?
    } else {
        0
    };
    Ok(Rc::new(RefCell::new(StreamData {
        backing: Some(Backing::File(handle)),
        pos,
        can_read: access.can_read(),
        can_write: access.can_write(),
        can_seek: true,
    })))
}

/// Голое имя `ФайловыеПотоки` как выражение.
///
/// Каждое обращение даёт НОВЫЙ объект: измерено, что
/// `ФайловыеПотоки = ФайловыеПотоки` — «Нет». Поэтому менеджер и не может
/// быть константой в таблице чанка, как голое имя перечисления, — его
/// строит отдельная инструкция.
pub fn new_file_streams_manager(files: Rc<dyn FileSystem>) -> BslValue {
    BslValue::new_object(FileStreamsManager { files })
}

pub(crate) static FILE_STREAMS_MANAGER_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "МенеджерФайловыхПотоков",
    type_display: "Менеджер файловых потоков",
    type_names: &["FileStreamsManager"],
};

#[derive(Debug)]
struct FileStreamsManager {
    /// Файловая система сессии: менеджер открывает файлы в СВОИХ методах,
    /// а те под JIT идут нативным путём без контекста, — потому владеет `Rc`.
    files: Rc<dyn FileSystem>,
}

// --- методы менеджера -------------------------------------------------------------

/// Имя файла из аргумента метода менеджера.
fn manager_path(args: &[BslValue], op: &'static str) -> RtResult<String> {
    match args {
        [BslValue::Str(path)] => Ok(path.to_string()),
        [_] => Err(RtError::TypeError {
            expected: "Строка",
            op,
        }),
        _ => Err(RtError::MethodNotApplicable {
            method: op,
            receiver: "МенеджерФайловыхПотоков",
        }),
    }
}

/// `ФайловыеПотоки.Открыть(Имя, Режим[, Доступ])` — то же, что конструктор.
///
/// # Errors
///
/// Те же, что у [`new_file_stream`], плюс «метод не применим» при неверном
/// числе аргументов (одного платформа не берёт — измерено).
pub fn manager_open(args: &[BslValue], files: &dyn FileSystem) -> RtResult<BslValue> {
    match args {
        [path, mode] => new_file_stream(path, mode, &BslValue::Undefined, files),
        [path, mode, access] => new_file_stream(path, mode, access, files),
        _ => Err(RtError::MethodNotApplicable {
            method: "Открыть",
            receiver: "МенеджерФайловыхПотоков",
        }),
    }
}

/// `ФайловыеПотоки.ОткрытьДляЧтения(Имя)` — `Открыть` плюс доступ `Чтение`
/// (измерено: на существующем файле размер сохраняется, признак записи
/// «Нет», а отсутствующий файл — ошибка).
///
/// # Errors
///
/// Те же, что у [`new_file_stream`].
pub fn manager_open_for_read(args: &[BslValue], files: &dyn FileSystem) -> RtResult<BslValue> {
    let path = manager_path(args, "ОткрытьДляЧтения")?;
    open_file_stream(&path, FileOpenMode::Open, FileAccess::Read, files)
}

/// `ФайловыеПотоки.ОткрытьДляЗаписи(Имя)` — `ОткрытьИлиСоздать` плюс доступ
/// `Запись`. Существующий файл НЕ обрезается: измерено, что размер остаётся
/// прежним, — поэтому здесь не `Создать`.
///
/// # Errors
///
/// Те же, что у [`new_file_stream`].
pub fn manager_open_for_write(args: &[BslValue], files: &dyn FileSystem) -> RtResult<BslValue> {
    let path = manager_path(args, "ОткрытьДляЗаписи")?;
    open_file_stream(&path, FileOpenMode::OpenOrCreate, FileAccess::Write, files)
}

/// `ФайловыеПотоки.ОткрытьДляДописывания(Имя)` — `Дописать` плюс доступ
/// `Запись` (измерено: позиция при открытии равна размеру, чтение
/// недоступно).
///
/// # Errors
///
/// Те же, что у [`new_file_stream`].
pub fn manager_open_for_append(args: &[BslValue], files: &dyn FileSystem) -> RtResult<BslValue> {
    let path = manager_path(args, "ОткрытьДляДописывания")?;
    open_file_stream(&path, FileOpenMode::Append, FileAccess::Write, files)
}

/// `ФайловыеПотоки.Создать(Имя)` — `Создать` с доступом по умолчанию
/// (измерено: размер 0, доступны и чтение, и запись).
///
/// # Errors
///
/// Те же, что у [`new_file_stream`].
pub fn manager_create(args: &[BslValue], files: &dyn FileSystem) -> RtResult<BslValue> {
    let path = manager_path(args, "Создать")?;
    open_file_stream(&path, FileOpenMode::Create, FileAccess::ReadWrite, files)
}

/// Получатель-менеджер: чужой тип получает ту же ошибку «метод не применим».
fn manager_of<'r>(
    receiver: &'r dyn ObjectProtocol,
    method: &'static str,
) -> RtResult<&'r FileStreamsManager> {
    receiver
        .downcast_ref::<FileStreamsManager>()
        .ok_or(RtError::MethodNotApplicable {
            method,
            receiver: "МенеджерФайловыхПотоков",
        })
}

// Методы менеджера файловых потоков: файловую систему берут у получателя,
// который запомнил её при построении.
fn manager_method_open(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let manager = manager_of(receiver, "Открыть")?;
    manager_open(arguments, manager.files.as_ref())
}

fn manager_method_open_for_read(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let manager = manager_of(receiver, "ОткрытьДляЧтения")?;
    manager_open_for_read(arguments, manager.files.as_ref())
}

fn manager_method_open_for_write(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let manager = manager_of(receiver, "ОткрытьДляЗаписи")?;
    manager_open_for_write(arguments, manager.files.as_ref())
}

fn manager_method_open_for_append(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let manager = manager_of(receiver, "ОткрытьДляДописывания")?;
    manager_open_for_append(arguments, manager.files.as_ref())
}

fn manager_method_create(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let manager = manager_of(receiver, "Создать")?;
    manager_create(arguments, manager.files.as_ref())
}

const FILE_STREAMS_MANAGER_METHODS: &[MethodDescriptor] = &[
    MethodDescriptor::new(
        &["Открыть", "Open"],
        Arity::range(2, 3),
        manager_method_open,
    ),
    MethodDescriptor::new(
        &["ОткрытьДляЧтения", "OpenForRead"],
        Arity::exact(1),
        manager_method_open_for_read,
    ),
    MethodDescriptor::new(
        &["ОткрытьДляЗаписи", "OpenForWrite"],
        Arity::exact(1),
        manager_method_open_for_write,
    ),
    MethodDescriptor::new(
        &["ОткрытьДляДописывания", "OpenForAppend"],
        Arity::exact(1),
        manager_method_open_for_append,
    ),
    MethodDescriptor::new(
        &["Создать", "Create"],
        Arity::exact(1),
        manager_method_create,
    ),
];

impl ObjectProtocol for FileStreamsManager {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &FILE_STREAMS_MANAGER_TYPE
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        FILE_STREAMS_MANAGER_METHODS
    }
}

// --- признаки доступности ------------------------------------------------------------

// Три признака доступности — свойства (вызов со скобками платформа
// отвергает, измерено), и все три только на чтение. Обработчики читают
// состояние через общий `flag`, как читал прежний строковый `if`.
fn stream_can_write(
    receiver: &dyn ObjectProtocol,
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    flag(receiver, StreamFlag::Writable)
}

fn stream_can_read(
    receiver: &dyn ObjectProtocol,
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    flag(receiver, StreamFlag::Readable)
}

fn stream_can_seek(
    receiver: &dyn ObjectProtocol,
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    flag(receiver, StreamFlag::Seekable)
}

static STREAM_PROPERTIES: &[PropertyDescriptor] = &[
    PropertyDescriptor {
        names: &["ДоступнаЗапись", "CanWrite"],
        get: stream_can_write,
        set: None,
    },
    PropertyDescriptor {
        names: &["ДоступноЧтение", "CanRead"],
        get: stream_can_read,
        set: None,
    },
    PropertyDescriptor {
        names: &["ДоступноИзменениеПозиции", "CanSeek"],
        get: stream_can_seek,
        set: None,
    },
];

/// Какой из трёх признаков спрашивают: `ДоступнаЗапись`, `ДоступноЧтение`
/// либо `ДоступноИзменениеПозиции`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StreamFlag {
    Writable,
    Readable,
    Seekable,
}

/// `Поток.ДоступнаЗапись` / `.ДоступноЧтение` / `.ДоступноИзменениеПозиции`
/// — именно СВОЙСТВА (вызов со скобками платформа отвергает — измерено).
/// После `Закрыть()` продолжают отдавать прежние значения, тоже измерено.
///
/// # Errors
///
/// «Метод не применим», если получатель не поток; [`RtError::IoError`],
/// если состояние потока занято другой операцией.
fn flag(v: &dyn ObjectProtocol, which: StreamFlag) -> RtResult<BslValue> {
    let name = match which {
        StreamFlag::Writable => "ДоступнаЗапись",
        StreamFlag::Readable => "ДоступноЧтение",
        StreamFlag::Seekable => "ДоступноИзменениеПозиции",
    };
    let d = data(v, name)?;
    let d = d
        .try_borrow()
        .map_err(|_| RtError::IoError(format!("{name}: поток уже занят другой операцией")))?;
    Ok(BslValue::Boolean(match which {
        StreamFlag::Writable => d.can_write,
        StreamFlag::Readable => d.can_read,
        StreamFlag::Seekable => d.can_seek,
    }))
}

// --- навигация -------------------------------------------------------------------------

/// `Поток.Размер()` — МЕТОД, в отличие от свойства `Размер` у буфера.
///
/// # Errors
///
/// [`RtError::IoError`], если поток закрыт (измерено) либо файл не отдал
/// свои метаданные.
pub fn size(v: &dyn ObjectProtocol) -> RtResult<BslValue> {
    let d = data(v, "Размер")?;
    let len = d.borrow().len("Размер")?;
    Ok(from_u64(len))
}

/// `Поток.ТекущаяПозиция()` — тоже метод. На ЗАКРЫТОМ потоке продолжает
/// работать и отдаёт последнюю позицию (измерено — в отличие от `Размер()`,
/// который на нём уже ошибка).
///
/// # Errors
///
/// «Метод не применим», если получатель не поток.
pub fn current_position(v: &dyn ObjectProtocol) -> RtResult<BslValue> {
    let d = data(v, "ТекущаяПозиция")?;
    let pos = d.borrow().pos;
    Ok(from_u64(pos))
}

/// `Поток.Перейти(Смещение, ПозицияВПотоке)` -> новая позиция числом.
///
/// # Errors
///
/// [`RtError::TypeError`] на дробном смещении, на нечисловом смещении и на
/// точке отсчёта не из `ПозицияВПотоке` (всё измерено);
/// [`RtError::IoError`] на закрытом потоке; «метод не применим» при ином
/// числе аргументов — платформа требует ровно два.
pub fn seek(v: &dyn ObjectProtocol, args: &[BslValue]) -> RtResult<BslValue> {
    const OP: &str = "Перейти";
    let [offset, origin] = args else {
        return Err(RtError::MethodNotApplicable {
            method: OP,
            receiver: v.type_descriptor().name,
        });
    };
    let offset = match offset {
        BslValue::Number(n) if n.is_integer() => n.to_i64_exact().ok_or(RtError::TypeError {
            expected: "Целое число, умещающееся в позицию потока",
            op: OP,
        })?,
        _ => {
            return Err(RtError::TypeError {
                expected: "Целое число",
                op: OP,
            });
        }
    };
    let origin = match origin {
        BslValue::Enum(e) => StreamOrigin::from_enum(*e),
        _ => None,
    }
    .ok_or(RtError::TypeError {
        expected: "ПозицияВПотоке",
        op: OP,
    })?;
    let d = data(v, OP)?;
    let mut d = d.borrow_mut();
    let base = match origin {
        StreamOrigin::Begin => 0,
        StreamOrigin::Current => d.pos,
        StreamOrigin::End => d.len(OP)?,
    };
    // Закрытый поток `Перейти` не обслуживает — а `Конец` уже спросил бы
    // длину и упал бы сам; для двух других точек проверяем явно.
    d.open(OP)?;
    // Уход за начало ОБРЕЗАЕТСЯ нулём от всех трёх точек отсчёта
    // (измерено), уход за конец разрешён и размера не меняет.
    let pos = i128::from(base) + i128::from(offset);
    let pos = u64::try_from(pos.max(0)).map_err(|_| RtError::TypeError {
        expected: "Позиция, умещающаяся в потоке",
        op: OP,
    })?;
    d.pos = pos;
    Ok(from_u64(pos))
}

// --- чтение и запись --------------------------------------------------------------------

/// `Поток.Записать(Буфер, СмещениеВБуфере, Количество)`.
///
/// Ровно три аргумента: платформа отвергает и один, и два (измерено).
/// Запись за концом потока удлиняет его, пропущенные байты нулевые; у
/// потока НАД БУФЕРОМ размер фиксирован и выход за него — ошибка.
///
/// # Errors
///
/// [`RtError::TypeError`] на нечисловых или дробных аргументах количества и
/// на буфере не того типа; [`RtError::IndexOutOfBounds`], если отрезок не
/// лежит в буфере; [`RtError::IoError`] на закрытом потоке, на потоке без
/// доступа на запись и на отказе файловой системы.
pub fn write(v: &dyn ObjectProtocol, args: &[BslValue]) -> RtResult<()> {
    const OP: &str = "Записать";
    let [buf, offset, count] = args else {
        return Err(RtError::MethodNotApplicable {
            method: OP,
            receiver: v.type_descriptor().name,
        });
    };
    let src = buffer_of(buf, OP)?;
    // Байты снимаются в отдельный вектор ДО того, как берётся изменяемое
    // заимствование приёмника: у потока над буфером источник и приёмник
    // могут оказаться одним и тем же `RefCell`, и два заимствования разом
    // уронили бы процесс.
    let bytes = src.binary_buffer_bytes().expect("тип проверен `buffer_of`");
    let (offset, count) = slice_in_buffer(offset, count, bytes.len(), OP)?;
    let chunk = bytes[offset..offset + count].to_vec();
    let d = data(v, OP)?;
    // `Перейти` разрешает уход за конец, поэтому позиция может стоять у самого
    // края `u64` (два перехода по `9223372036854775807` дают `2^64 - 2` — и у
    // нас, и у платформы, измерено контрактом `measure-stream.bsl`). Проверку
    // на это, как и всю работу с носителем, делает `write_bytes`: край
    // отвергается ловимой ошибкой, ничего не записав и не сдвинув позицию.
    d.borrow_mut().write_bytes(&chunk, OP)
}

/// `Поток.Прочитать(Буфер, СмещениеВБуфере, Количество)` -> сколько байтов
/// прочитано.
///
/// Чтение у конца отдаёт СКОЛЬКО ЕСТЬ, а не ошибку (измерено: остаток в один
/// байт при запросе четырёх даёт 1, в самом конце — 0).
///
/// # Errors
///
/// Те же, что у [`write`], плюс отсутствие доступа на чтение.
pub fn read(v: &dyn ObjectProtocol, args: &[BslValue]) -> RtResult<BslValue> {
    const OP: &str = "Прочитать";
    let [buf, offset, count] = args else {
        return Err(RtError::MethodNotApplicable {
            method: OP,
            receiver: v.type_descriptor().name,
        });
    };
    let dst = buffer_of(buf, OP)?;
    let len = dst.binary_buffer_len().expect("тип проверен `buffer_of`");
    let (offset, count) = slice_in_buffer(offset, count, len, OP)?;
    let d = data(v, OP)?;
    // Байты сначала снимаются, и только потом берётся изменяемое
    // заимствование приёмника: у потока НАД БУФЕРОМ источник и приёмник могут
    // оказаться одним и тем же `RefCell`, и два заимствования разом уронили бы
    // процесс.
    let chunk = d.borrow_mut().read_bytes(count, OP)?;
    dst.binary_buffer_write(offset, &chunk)?;
    Ok(from_u64(chunk.len() as u64))
}

/// Не больше `count` байтов начиная с `pos`; за концом — пусто.
fn slice_from(bytes: &[u8], pos: u64, count: usize) -> Vec<u8> {
    let Ok(pos) = usize::try_from(pos) else {
        return Vec::new();
    };
    if pos >= bytes.len() {
        return Vec::new();
    }
    let end = bytes.len().min(pos.saturating_add(count));
    bytes[pos..end].to_vec()
}

/// `Поток.Закрыть()` — отпускает носитель. Идемпотентно: повторное закрытие
/// платформа принимает (измерено).
///
/// # Errors
///
/// «Метод не применим», если получатель не поток.
pub fn close(v: &dyn ObjectProtocol) -> RtResult<()> {
    let d = data(v, "Закрыть")?;
    let mut state = d.borrow_mut();
    // Закон `close` (ABI-G0): у файлового носителя сначала ЯВНОЕ закрытие
    // дескриптора; на отказе носитель СОХРАНЯЕТСЯ — поток остаётся
    // полузакрыт (`Размер()` ещё работает, позиция и признаки целы), и
    // повторный `Закрыть()` пробует снова. Носитель снимается только после
    // успеха. У памяти закрывать нечего.
    if let Some(Backing::File(handle)) = state.backing.as_mut() {
        handle
            .close()
            .map_err(|e| RtError::IoError(format!("Закрыть: {e}")))?;
    }
    state.backing = None;
    Ok(())
}

#[cfg(test)]
mod tests {

    /// Два потока — РАЗНЫЕ типы с ОДНИМ представлением. Измерено:
    /// `Строка(Тип("ПотокВПамяти"))` и `Строка(Тип("ФайловыйПоток"))` оба
    /// дают «Файловый поток», а `Тип("ПотокВПамяти") = Тип("ФайловыйПоток")`
    /// — «Нет». Ровно это закрепляет и фикстура `binary-streams`.
    #[test]
    fn both_streams_print_the_same_name_but_stay_different_types() {
        assert_ne!(MEMORY_STREAM_TYPE.name, FILE_STREAM_TYPE.name);
        assert_eq!(MEMORY_STREAM_TYPE.type_display, "Файловый поток");
        assert_eq!(FILE_STREAM_TYPE.type_display, "Файловый поток");
        assert!(MEMORY_STREAM_TYPE.answers_to("ПотокВПамяти"));
        assert!(MEMORY_STREAM_TYPE.answers_to("MemoryStream"));
        assert!(FILE_STREAM_TYPE.answers_to("ФайловыйПоток"));
        // Обратная сторона совпадения представлений: по нему находится
        // тоже только один из двух — так же несимметрично, как у платформы.
        assert!(FILE_STREAM_TYPE.answers_to("Файловый поток"));
    }

    /// НАПРАВЛЕНИЕ асимметрии: на общее представление «Файловый поток»
    /// откликаются ОБА типа, а `Тип("Файловый поток")` обязан отдать
    /// именно `ФайловыйПоток` — измерено. После ABI-D это свойство
    /// принадлежит КАТАЛОГУ типов реестра (объявленный владелец псевдонима),
    /// а не порядку списка, поэтому разрешение идёт через собранный реестр,
    /// а не заполнением поля напрямую.
    #[test]
    fn the_shared_display_name_resolves_to_the_file_stream() {
        let mut builder = bsl_rt::RuntimeBuilder::new();
        builder.register(bsl_rt::core_library());
        builder.register(bsl_binbuf::library());
        builder.register(crate::library());
        let registry = builder.build().expect("реестр собирается");
        let rt = bsl_rt::RuntimeShapes::seeded(Vec::new(), Vec::new(), Some(&registry));

        let file = match rt.resolve_type("Файловый поток") {
            Some(bsl_rt::TypeRef::Object(d)) => d,
            other => panic!("ожидался тип объекта, получено {other:?}"),
        };
        assert_eq!(file.name, FILE_STREAM_TYPE.name);
        let memory = match rt.resolve_type("ПотокВПамяти") {
            Some(bsl_rt::TypeRef::Object(d)) => d,
            other => panic!("ожидался тип объекта, получено {other:?}"),
        };
        assert_eq!(memory.name, MEMORY_STREAM_TYPE.name);
    }

    /// Библиотека без объявленного владельца общего написания отвергается на
    /// сборке (ABI-D): та же пара потоков, но `with_type_aliases` не задан —
    /// каталог не может разрешить «Файловый поток» и даёт `AmbiguousTypeAlias`.
    #[test]
    fn a_shared_spelling_without_a_declared_owner_is_rejected() {
        static AMBIGUOUS_TYPES: &[&bsl_rt::TypeDescriptor] =
            &[&MEMORY_STREAM_TYPE, &FILE_STREAM_TYPE];
        let ambiguous = bsl_rt::LibraryDescriptor::new(
            crate::PACKAGE_NAME,
            crate::PACKAGE_VERSION,
            bsl_rt::ObjectJitPolicy::NativeContextCompatible,
        )
        .with_types(AMBIGUOUS_TYPES);
        let mut builder = bsl_rt::RuntimeBuilder::new();
        builder.register(bsl_rt::core_library());
        builder.register(ambiguous);
        assert!(matches!(
            builder.build(),
            Err(bsl_rt::RegistryError::AmbiguousTypeAlias(_))
        ));
    }
    use super::*;
    use bsl_rt::SystemFileSystem;

    // Конструкторы потоков и методы менеджера после ABI-G берут файловую
    // систему сессии; сценарии, которым она безразлична, зовут эти тёзки с
    // процессной ФС по умолчанию.
    fn new_file_stream(path: &BslValue, mode: &BslValue, access: &BslValue) -> RtResult<BslValue> {
        super::new_file_stream(path, mode, access, &SystemFileSystem)
    }
    fn new_file_streams_manager() -> BslValue {
        super::new_file_streams_manager(std::rc::Rc::new(SystemFileSystem))
    }
    fn manager_open(args: &[BslValue]) -> RtResult<BslValue> {
        super::manager_open(args, &SystemFileSystem)
    }
    fn manager_open_for_read(args: &[BslValue]) -> RtResult<BslValue> {
        super::manager_open_for_read(args, &SystemFileSystem)
    }
    fn manager_open_for_write(args: &[BslValue]) -> RtResult<BslValue> {
        super::manager_open_for_write(args, &SystemFileSystem)
    }
    fn manager_open_for_append(args: &[BslValue]) -> RtResult<BslValue> {
        super::manager_open_for_append(args, &SystemFileSystem)
    }
    fn manager_create(args: &[BslValue]) -> RtResult<BslValue> {
        super::manager_create(args, &SystemFileSystem)
    }

    /// Поток за значением: обработчики и хелперы принимают объект.
    fn st(v: &BslValue) -> &dyn ObjectProtocol {
        v.object_ref().expect("поток").as_dyn()
    }

    fn num(v: i64) -> BslValue {
        BslValue::Number(BslNumber::from_i64(v))
    }

    fn frac(units: i128, scale: i32) -> BslValue {
        BslValue::Number(BslNumber::from_parts(units, scale).expect("масштаб теста допустим"))
    }

    fn buffer(bytes: &[u8]) -> BslValue {
        BslValue::binary_buffer_of(bytes.to_vec())
    }

    fn bytes_of(b: &BslValue) -> Vec<u8> {
        b.binary_buffer_bytes().expect("буфер")
    }

    fn as_u64(v: &BslValue) -> u64 {
        match v {
            BslValue::Number(n) => n.to_i64_exact().expect("целое") as u64,
            _ => panic!("не число"),
        }
    }

    fn memory() -> BslValue {
        new_memory_stream(&BslValue::Undefined).unwrap()
    }

    fn tmp(name: &str) -> String {
        format!(
            "{}/open-bsl-stream-test-{}-{}.bin",
            std::env::temp_dir().display(),
            std::process::id(),
            name
        )
    }

    fn write_file(path: &str, bytes: &[u8]) {
        std::fs::write(path, bytes).expect("временный файл");
    }

    fn enum_val(e: EnumValue) -> BslValue {
        BslValue::Enum(e)
    }

    #[test]
    fn a_new_memory_stream_is_empty_readable_and_writable() {
        let s = memory();
        assert_eq!(as_u64(&size(st(&s)).unwrap()), 0);
        assert_eq!(as_u64(&current_position(st(&s)).unwrap()), 0);
        assert_eq!(
            flag(st(&s), StreamFlag::Writable).unwrap(),
            BslValue::Boolean(true)
        );
        assert_eq!(
            flag(st(&s), StreamFlag::Readable).unwrap(),
            BslValue::Boolean(true)
        );
        assert_eq!(
            flag(st(&s), StreamFlag::Seekable).unwrap(),
            BslValue::Boolean(true)
        );
    }

    /// Числовой аргумент — ёмкость, а не предел: измерено, что поток растёт
    /// за неё.
    #[test]
    fn the_numeric_argument_is_a_capacity_not_a_limit() {
        let s = new_memory_stream(&num(2)).unwrap();
        assert_eq!(as_u64(&size(st(&s)).unwrap()), 0);
        write(st(&s), &[buffer(&[1, 2, 3, 4]), num(0), num(4)]).unwrap();
        assert_eq!(as_u64(&size(st(&s)).unwrap()), 4);
    }

    #[test]
    fn the_constructor_rejects_what_the_platform_rejects() {
        assert!(new_memory_stream(&num(-1)).is_err());
        assert!(new_memory_stream(&frac(25, 1)).is_err());
        assert!(new_memory_stream(&BslValue::Str(bsl_rt::BslString::from_str("4"))).is_err());
        assert!(new_memory_stream(&BslValue::binary_data_of(vec![1, 2])).is_err());
    }

    #[test]
    fn writing_advances_the_position_and_grows_the_stream() {
        let s = memory();
        write(st(&s), &[buffer(&[10, 20, 30, 40]), num(0), num(4)]).unwrap();
        assert_eq!(as_u64(&size(st(&s)).unwrap()), 4);
        assert_eq!(as_u64(&current_position(st(&s)).unwrap()), 4);
        write(st(&s), &[buffer(&[10, 20, 30, 40]), num(1), num(2)]).unwrap();
        assert_eq!(as_u64(&size(st(&s)).unwrap()), 6);
        assert_eq!(as_u64(&current_position(st(&s)).unwrap()), 6);
    }

    #[test]
    fn seeking_from_all_three_origins() {
        let s = memory();
        write(st(&s), &[buffer(&[1, 2, 3, 4, 5, 6]), num(0), num(6)]).unwrap();
        let begin = enum_val(EnumValue::StreamPositionBegin);
        let current = enum_val(EnumValue::StreamPositionCurrent);
        let end = enum_val(EnumValue::StreamPositionEnd);
        assert_eq!(as_u64(&seek(st(&s), &[num(0), begin.clone()]).unwrap()), 0);
        assert_eq!(as_u64(&seek(st(&s), &[num(2), begin.clone()]).unwrap()), 2);
        assert_eq!(
            as_u64(&seek(st(&s), &[num(1), current.clone()]).unwrap()),
            3
        );
        assert_eq!(
            as_u64(&seek(st(&s), &[num(-1), current.clone()]).unwrap()),
            2
        );
        assert_eq!(as_u64(&seek(st(&s), &[num(0), end.clone()]).unwrap()), 6);
        assert_eq!(as_u64(&seek(st(&s), &[num(-2), end.clone()]).unwrap()), 4);
    }

    /// Уход за начало обрезается нулём от ВСЕХ трёх точек, за конец —
    /// разрешён и размера не меняет. Измерено.
    #[test]
    fn seeking_past_the_ends_clamps_at_zero_and_never_resizes() {
        let s = memory();
        write(st(&s), &[buffer(&[1, 2, 3, 4, 5, 6]), num(0), num(6)]).unwrap();
        let begin = enum_val(EnumValue::StreamPositionBegin);
        let current = enum_val(EnumValue::StreamPositionCurrent);
        let end = enum_val(EnumValue::StreamPositionEnd);
        assert_eq!(as_u64(&seek(st(&s), &[num(-1), begin.clone()]).unwrap()), 0);
        assert_eq!(as_u64(&seek(st(&s), &[num(-100), end]).unwrap()), 0);
        assert_eq!(as_u64(&seek(st(&s), &[num(-100), current]).unwrap()), 0);
        assert_eq!(as_u64(&seek(st(&s), &[num(100), begin]).unwrap()), 100);
        assert_eq!(as_u64(&size(st(&s)).unwrap()), 6);
    }

    #[test]
    fn seeking_rejects_a_fractional_offset_and_a_foreign_enum() {
        let s = memory();
        let begin = enum_val(EnumValue::StreamPositionBegin);
        assert!(seek(st(&s), &[frac(15, 1), begin.clone()]).is_err());
        assert!(seek(st(&s), &[num(0), num(5)]).is_err());
        assert!(seek(st(&s), &[num(0), enum_val(EnumValue::ByteOrderLittle)]).is_err());
        assert!(seek(st(&s), &[num(0)]).is_err());
        assert!(seek(st(&s), &[num(0), begin.clone(), num(1)]).is_err());
    }

    #[test]
    fn a_hole_left_by_seeking_reads_back_as_zeroes() {
        let s = memory();
        let src = buffer(&[10, 20, 30, 40]);
        write(st(&s), &[src.clone(), num(0), num(2)]).unwrap();
        seek(st(&s), &[num(6), enum_val(EnumValue::StreamPositionBegin)]).unwrap();
        write(st(&s), &[src, num(2), num(2)]).unwrap();
        assert_eq!(as_u64(&size(st(&s)).unwrap()), 8);
        seek(st(&s), &[num(0), enum_val(EnumValue::StreamPositionBegin)]).unwrap();
        let dst = buffer(&[0; 8]);
        assert_eq!(
            as_u64(&read(st(&s), &[dst.clone(), num(0), num(8)]).unwrap()),
            8
        );
        assert_eq!(bytes_of(&dst), vec![10, 20, 0, 0, 0, 0, 30, 40]);
    }

    #[test]
    fn reading_near_the_end_returns_what_is_left() {
        let s = memory();
        write(st(&s), &[buffer(&[1, 2, 3]), num(0), num(3)]).unwrap();
        seek(st(&s), &[num(2), enum_val(EnumValue::StreamPositionBegin)]).unwrap();
        let dst = buffer(&[0; 4]);
        assert_eq!(
            as_u64(&read(st(&s), &[dst.clone(), num(0), num(4)]).unwrap()),
            1
        );
        assert_eq!(bytes_of(&dst), vec![3, 0, 0, 0]);
        assert_eq!(as_u64(&current_position(st(&s)).unwrap()), 3);
        assert_eq!(
            as_u64(&read(st(&s), &[dst.clone(), num(0), num(4)]).unwrap()),
            0
        );
        assert_eq!(as_u64(&read(st(&s), &[dst, num(0), num(0)]).unwrap()), 0);
    }

    /// Дробное СМЕЩЕНИЕ в буфере усекается, дробное КОЛИЧЕСТВО отвергается —
    /// измеренная асимметрия, одинаковая у записи и у чтения.
    #[test]
    fn a_fractional_buffer_offset_truncates_but_a_fractional_count_is_rejected() {
        let s = memory();
        let src = buffer(&[10, 20, 30, 40]);
        write(st(&s), &[src.clone(), frac(15, 1), num(1)]).unwrap();
        seek(st(&s), &[num(0), enum_val(EnumValue::StreamPositionBegin)]).unwrap();
        let dst = buffer(&[0; 4]);
        assert_eq!(
            as_u64(&read(st(&s), &[dst.clone(), num(0), num(1)]).unwrap()),
            1
        );
        assert_eq!(bytes_of(&dst)[0], 20);
        assert!(write(st(&s), &[src.clone(), num(0), frac(15, 1)]).is_err());
        assert!(read(st(&s), &[dst, num(0), frac(15, 1)]).is_err());
    }

    #[test]
    fn read_and_write_check_the_buffer_bounds_and_the_argument_types() {
        let s = memory();
        let src = buffer(&[10, 20, 30, 40]);
        assert!(write(st(&s), &[src.clone(), num(3), num(4)]).is_err());
        assert!(write(st(&s), &[src.clone(), num(-1), num(1)]).is_err());
        assert!(write(st(&s), &[src.clone(), num(0), num(-1)]).is_err());
        assert!(write(st(&s), &[num(5), num(0), num(1)]).is_err());
        assert!(write(st(&s), std::slice::from_ref(&src)).is_err());
        assert!(read(st(&s), &[src.clone(), num(3), num(4)]).is_err());
        assert!(read(st(&s), &[src]).is_err());
    }

    /// Поток над буфером делит с ним память в обе стороны и не растёт.
    #[test]
    fn a_stream_over_a_buffer_shares_its_memory_and_its_fixed_size() {
        let buf = buffer(&[1, 2, 3, 4]);
        let s = new_memory_stream(&buf).unwrap();
        assert_eq!(as_u64(&size(st(&s)).unwrap()), 4);
        write(st(&s), &[buffer(&[9, 9, 9, 9]), num(0), num(2)]).unwrap();
        assert_eq!(bytes_of(&buf), vec![9, 9, 3, 4]);
        buf.binary_buffer_write(3, &[77]).unwrap();
        seek(st(&s), &[num(3), enum_val(EnumValue::StreamPositionBegin)]).unwrap();
        let dst = buffer(&[0]);
        read(st(&s), &[dst.clone(), num(0), num(1)]).unwrap();
        assert_eq!(bytes_of(&dst), vec![77]);
        seek(st(&s), &[num(4), enum_val(EnumValue::StreamPositionBegin)]).unwrap();
        assert!(write(st(&s), &[buffer(&[9, 9, 9, 9]), num(0), num(4)]).is_err());
        assert_eq!(as_u64(&size(st(&s)).unwrap()), 4);
    }

    /// Уход за конец `Перейти` разрешает, поэтому позиция доводится до
    /// `2^64 - 2` (сама позиция измерена: платформа принимает оба перехода и
    /// печатает то же число) — и конец записи в голом `u64` переполнялся бы,
    /// роняя процесс мимо `Попытка`. Платформа на этой записи падает сама, так
    /// что эталона нет; ожидается ловимая ошибка на обоих носителях.
    #[test]
    fn write_after_seek_to_u64_edge_is_error_not_panic() {
        let begin = enum_val(EnumValue::StreamPositionBegin);
        let current = enum_val(EnumValue::StreamPositionCurrent);

        let s = memory();
        seek(st(&s), &[num(i64::MAX), begin.clone()]).unwrap();
        seek(st(&s), &[num(i64::MAX), current.clone()]).unwrap();
        assert!(write(st(&s), &[buffer(&[1, 2, 3, 4]), num(0), num(4)]).is_err());

        let s = new_memory_stream(&buffer(&[0; 4])).unwrap();
        seek(st(&s), &[num(i64::MAX), begin]).unwrap();
        seek(st(&s), &[num(i64::MAX), current]).unwrap();
        assert!(write(st(&s), &[buffer(&[1, 2, 3, 4]), num(0), num(4)]).is_err());
    }

    /// Тот же буфер и источником, и приёмником: два заимствования одного
    /// `RefCell` уронили бы процесс, поэтому байты копируются заранее.
    #[test]
    fn a_stream_over_a_buffer_survives_that_buffer_as_its_own_argument() {
        let buf = buffer(&[1, 2, 3, 4]);
        let s = new_memory_stream(&buf).unwrap();
        write(st(&s), &[buf.clone(), num(0), num(2)]).unwrap();
        assert_eq!(bytes_of(&buf), vec![1, 2, 3, 4]);
        seek(st(&s), &[num(0), enum_val(EnumValue::StreamPositionBegin)]).unwrap();
        read(st(&s), &[buf.clone(), num(0), num(2)]).unwrap();
        assert_eq!(bytes_of(&buf), vec![1, 2, 3, 4]);
    }

    #[test]
    fn a_closed_stream_keeps_its_position_and_flags_but_nothing_else() {
        let s = memory();
        write(st(&s), &[buffer(&[1, 2, 3, 4]), num(0), num(4)]).unwrap();
        close(st(&s)).unwrap();
        assert_eq!(as_u64(&current_position(st(&s)).unwrap()), 4);
        assert_eq!(
            flag(st(&s), StreamFlag::Writable).unwrap(),
            BslValue::Boolean(true)
        );
        assert_eq!(
            flag(st(&s), StreamFlag::Readable).unwrap(),
            BslValue::Boolean(true)
        );
        assert_eq!(
            flag(st(&s), StreamFlag::Seekable).unwrap(),
            BslValue::Boolean(true)
        );
        assert!(size(st(&s)).is_err());
        assert!(seek(st(&s), &[num(0), enum_val(EnumValue::StreamPositionBegin)]).is_err());
        assert!(write(st(&s), &[buffer(&[1]), num(0), num(1)]).is_err());
        assert!(read(st(&s), &[buffer(&[0]), num(0), num(1)]).is_err());
        // Повторное закрытие платформа принимает.
        close(st(&s)).unwrap();
    }

    #[test]
    fn stream_methods_reject_a_receiver_that_is_not_a_stream() {
        // Не-поток здесь — объект другого типа: значение-не-объект ABI
        // метода уже не пропускает (получатель — `&dyn ObjectProtocol`).
        let not_a_stream = new_file_streams_manager();
        assert!(size(st(&not_a_stream)).is_err());
        assert!(current_position(st(&not_a_stream)).is_err());
        assert!(close(st(&not_a_stream)).is_err());
        assert!(flag(st(&not_a_stream), StreamFlag::Readable).is_err());
    }

    #[test]
    fn streams_are_equal_by_identity() {
        let a = memory();
        let b = memory();
        assert!(a.eq_value(&a));
        assert!(!a.eq_value(&b));
        let same = a.clone();
        assert!(a.eq_value(&same));
    }

    // --- файловые потоки ------------------------------------------------------

    fn open(path: &str, mode: EnumValue, access: Option<EnumValue>) -> RtResult<BslValue> {
        new_file_stream(
            &BslValue::Str(bsl_rt::BslString::from_str(path)),
            &enum_val(mode),
            &match access {
                Some(a) => enum_val(a),
                None => BslValue::Undefined,
            },
        )
    }

    /// Полная таблица «режим x доступ» на СУЩЕСТВУЮЩЕМ файле — та же, что
    /// снята с платформы.
    #[test]
    fn the_open_mode_table_on_an_existing_file_matches_the_platform() {
        let path = tmp("modes-existing");
        let refill = || write_file(&path, b"0123456789ABC");

        refill();
        let s = open(&path, EnumValue::FileOpenModeOpen, None).unwrap();
        assert_eq!(as_u64(&size(st(&s)).unwrap()), 13);
        assert_eq!(as_u64(&current_position(st(&s)).unwrap()), 0);

        refill();
        let s = open(
            &path,
            EnumValue::FileOpenModeOpen,
            Some(EnumValue::FileAccessRead),
        )
        .unwrap();
        assert_eq!(
            flag(st(&s), StreamFlag::Writable).unwrap(),
            BslValue::Boolean(false)
        );
        assert_eq!(
            flag(st(&s), StreamFlag::Readable).unwrap(),
            BslValue::Boolean(true)
        );

        refill();
        let s = open(
            &path,
            EnumValue::FileOpenModeOpen,
            Some(EnumValue::FileAccessWrite),
        )
        .unwrap();
        assert_eq!(
            flag(st(&s), StreamFlag::Readable).unwrap(),
            BslValue::Boolean(false)
        );

        // `ОткрытьИлиСоздать` + `Чтение` на существующем файле РАБОТАЕТ.
        refill();
        let s = open(
            &path,
            EnumValue::FileOpenModeOpenOrCreate,
            Some(EnumValue::FileAccessRead),
        )
        .unwrap();
        assert_eq!(as_u64(&size(st(&s)).unwrap()), 13);

        // `Создать` обрезает, `Чтение` с ним несовместимо.
        refill();
        assert!(
            open(
                &path,
                EnumValue::FileOpenModeCreate,
                Some(EnumValue::FileAccessRead)
            )
            .is_err()
        );
        let s = open(&path, EnumValue::FileOpenModeCreate, None).unwrap();
        assert_eq!(as_u64(&size(st(&s)).unwrap()), 0);

        // `СоздатьНовый` на существующем файле — всегда ошибка.
        refill();
        assert!(open(&path, EnumValue::FileOpenModeCreateNew, None).is_err());

        // `Обрезать` обрезает и тоже требует записи.
        refill();
        assert!(
            open(
                &path,
                EnumValue::FileOpenModeTruncate,
                Some(EnumValue::FileAccessRead)
            )
            .is_err()
        );
        let s = open(&path, EnumValue::FileOpenModeTruncate, None).unwrap();
        assert_eq!(as_u64(&size(st(&s)).unwrap()), 0);

        // `Дописать` ставит позицию в конец и с `Чтение` несовместим.
        refill();
        assert!(
            open(
                &path,
                EnumValue::FileOpenModeAppend,
                Some(EnumValue::FileAccessRead)
            )
            .is_err()
        );
        let s = open(&path, EnumValue::FileOpenModeAppend, None).unwrap();
        assert_eq!(as_u64(&size(st(&s)).unwrap()), 13);
        assert_eq!(as_u64(&current_position(st(&s)).unwrap()), 13);

        let _ = std::fs::remove_file(&path);
    }

    /// Та же таблица на ОТСУТСТВУЮЩЕМ файле: в фикстуру она не попадает —
    /// удалить файл из BSL нечем, — поэтому живёт здесь.
    #[test]
    fn the_open_mode_table_on_a_missing_file_matches_the_platform() {
        let path = tmp("modes-missing");
        let clear = || {
            let _ = std::fs::remove_file(&path);
        };

        clear();
        assert!(open(&path, EnumValue::FileOpenModeOpen, None).is_err());
        assert!(open(&path, EnumValue::FileOpenModeTruncate, None).is_err());
        assert!(
            open(
                &path,
                EnumValue::FileOpenModeOpenOrCreate,
                Some(EnumValue::FileAccessRead)
            )
            .is_err()
        );

        for mode in [
            EnumValue::FileOpenModeOpenOrCreate,
            EnumValue::FileOpenModeCreate,
            EnumValue::FileOpenModeCreateNew,
            EnumValue::FileOpenModeAppend,
        ] {
            clear();
            let s = open(&path, mode, None).unwrap();
            assert_eq!(as_u64(&size(st(&s)).unwrap()), 0, "режим {mode:?}");
            assert_eq!(
                as_u64(&current_position(st(&s)).unwrap()),
                0,
                "режим {mode:?}"
            );
            clear();
            assert!(
                open(&path, mode, Some(EnumValue::FileAccessRead)).is_err(),
                "режим {mode:?} с доступом только на чтение"
            );
        }
        clear();
    }

    #[test]
    fn the_file_constructor_rejects_what_the_platform_rejects() {
        let path = tmp("bad-args");
        write_file(&path, b"x");
        let name = BslValue::Str(bsl_rt::BslString::from_str(&path));
        assert!(new_file_stream(&name, &num(1), &BslValue::Undefined).is_err());
        assert!(new_file_stream(&name, &enum_val(EnumValue::FileOpenModeOpen), &num(1)).is_err());
        assert!(
            new_file_stream(
                &num(5),
                &enum_val(EnumValue::FileOpenModeOpen),
                &BslValue::Undefined
            )
            .is_err()
        );
        assert!(
            new_file_stream(
                &BslValue::Str(bsl_rt::BslString::from_str("")),
                &enum_val(EnumValue::FileOpenModeOpen),
                &BslValue::Undefined
            )
            .is_err()
        );
        let _ = std::fs::remove_file(&path);
    }

    /// Дыра в файле, рост и чтение обратно — то же, что измерено.
    #[test]
    fn a_file_stream_grows_over_a_hole_and_reads_it_back_as_zeroes() {
        let path = tmp("hole");
        let _ = std::fs::remove_file(&path);
        let s = manager_create(&[BslValue::Str(bsl_rt::BslString::from_str(&path))]).unwrap();
        assert_eq!(as_u64(&size(st(&s)).unwrap()), 0);
        let src = buffer(&[10, 20, 30, 40]);
        write(st(&s), &[src.clone(), num(0), num(4)]).unwrap();
        assert_eq!(as_u64(&size(st(&s)).unwrap()), 4);
        seek(st(&s), &[num(10), enum_val(EnumValue::StreamPositionBegin)]).unwrap();
        assert_eq!(as_u64(&size(st(&s)).unwrap()), 4);
        write(st(&s), &[src, num(0), num(2)]).unwrap();
        assert_eq!(as_u64(&size(st(&s)).unwrap()), 12);
        seek(st(&s), &[num(0), enum_val(EnumValue::StreamPositionBegin)]).unwrap();
        let dst = buffer(&[0; 12]);
        assert_eq!(
            as_u64(&read(st(&s), &[dst.clone(), num(0), num(12)]).unwrap()),
            12
        );
        assert_eq!(
            bytes_of(&dst),
            vec![10, 20, 30, 40, 0, 0, 0, 0, 0, 0, 10, 20]
        );
        close(st(&s)).unwrap();
        let _ = std::fs::remove_file(&path);
    }

    /// `Дописать` — только начальная позиция: после перехода в начало запись
    /// ложится туда, а не в конец (измерено).
    #[test]
    fn append_only_sets_the_initial_position() {
        let path = tmp("append");
        write_file(&path, b"0123456789ABC");
        let s = open(&path, EnumValue::FileOpenModeAppend, None).unwrap();
        assert_eq!(as_u64(&current_position(st(&s)).unwrap()), 13);
        write(st(&s), &[buffer(&[1, 2, 3]), num(0), num(3)]).unwrap();
        assert_eq!(as_u64(&size(st(&s)).unwrap()), 16);
        seek(st(&s), &[num(0), enum_val(EnumValue::StreamPositionBegin)]).unwrap();
        write(st(&s), &[buffer(&[10, 20]), num(0), num(2)]).unwrap();
        assert_eq!(as_u64(&size(st(&s)).unwrap()), 16);
        close(st(&s)).unwrap();
        assert_eq!(std::fs::read(&path).unwrap()[..4], [10, 20, b'2', b'3']);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn access_restricts_the_operations() {
        let path = tmp("access");
        write_file(&path, b"0123456789");
        let name = BslValue::Str(bsl_rt::BslString::from_str(&path));
        let ro = manager_open_for_read(std::slice::from_ref(&name)).unwrap();
        assert!(write(st(&ro), &[buffer(&[1]), num(0), num(1)]).is_err());
        assert_eq!(
            as_u64(&read(st(&ro), &[buffer(&[0]), num(0), num(1)]).unwrap()),
            1
        );
        close(st(&ro)).unwrap();

        let wo = manager_open_for_write(std::slice::from_ref(&name)).unwrap();
        // `ОткрытьДляЗаписи` НЕ обрезает — измерено.
        assert_eq!(as_u64(&size(st(&wo)).unwrap()), 10);
        write(st(&wo), &[buffer(&[1]), num(0), num(1)]).unwrap();
        assert!(read(st(&wo), &[buffer(&[0]), num(0), num(1)]).is_err());
        close(st(&wo)).unwrap();

        let ap = manager_open_for_append(std::slice::from_ref(&name)).unwrap();
        assert_eq!(as_u64(&current_position(st(&ap)).unwrap()), 10);
        assert_eq!(
            flag(st(&ap), StreamFlag::Readable).unwrap(),
            BslValue::Boolean(false)
        );
        close(st(&ap)).unwrap();

        let cr = manager_create(std::slice::from_ref(&name)).unwrap();
        assert_eq!(as_u64(&size(st(&cr)).unwrap()), 0);
        close(st(&cr)).unwrap();
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn the_manager_checks_its_own_arity_and_argument_types() {
        let path = tmp("manager");
        write_file(&path, b"0123456789");
        let name = BslValue::Str(bsl_rt::BslString::from_str(&path));
        assert!(manager_open(std::slice::from_ref(&name)).is_err());
        assert!(manager_open(&[name.clone(), enum_val(EnumValue::FileOpenModeOpen)]).is_ok());
        assert!(
            manager_open(&[
                name.clone(),
                enum_val(EnumValue::FileOpenModeOpen),
                enum_val(EnumValue::FileAccessRead)
            ])
            .is_ok()
        );
        assert!(manager_open_for_read(&[num(5)]).is_err());
        assert!(manager_open_for_read(&[name.clone(), num(5)]).is_err());
        assert!(manager_open_for_read(&[]).is_err());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn a_missing_directory_is_a_catchable_error() {
        let s = new_file_stream(
            &BslValue::Str(bsl_rt::BslString::from_str(
                "/tmp/open-bsl-stream-test-no-such-dir/x.bin",
            )),
            &enum_val(EnumValue::FileOpenModeCreate),
            &BslValue::Undefined,
        );
        assert!(matches!(s, Err(RtError::IoError(_))));
    }

    /// Закон `close` файлового потока (ABI-G0), судья — `Размер()`, потому
    /// что закрытый поток жив наполовину (измерено): после `Закрыть()`
    /// позиция и признаки целы, а ошибкой отвечает `Размер()`. Дескриптор
    /// отказывает на ПЕРВОМ `close`; проверяется вся вилка.
    #[test]
    fn a_file_stream_obeys_the_close_law() {
        use std::cell::Cell;

        #[derive(Debug)]
        struct FlakyHandle {
            closes: Rc<Cell<u32>>,
        }

        impl std::io::Read for FlakyHandle {
            fn read(&mut self, _buf: &mut [u8]) -> std::io::Result<usize> {
                Ok(0)
            }
        }
        impl std::io::Write for FlakyHandle {
            fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
                Ok(buf.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        impl std::io::Seek for FlakyHandle {
            fn seek(&mut self, _pos: SeekFrom) -> std::io::Result<u64> {
                Ok(0)
            }
        }
        impl FileHandle for FlakyHandle {
            fn len(&self) -> std::io::Result<u64> {
                Ok(42)
            }
            fn close(&mut self) -> std::io::Result<()> {
                let n = self.closes.get();
                self.closes.set(n + 1);
                if n == 0 {
                    Err(std::io::Error::other("первый close падает"))
                } else {
                    Ok(())
                }
            }
        }

        let closes = Rc::new(Cell::new(0));
        let data = Rc::new(RefCell::new(StreamData {
            backing: Some(Backing::File(Box::new(FlakyHandle {
                closes: closes.clone(),
            }))),
            pos: 7,
            can_read: true,
            can_write: false,
            can_seek: true,
        }));
        let value = BslValue::new_object(StreamObject {
            kind: StreamKind::File,
            data: data.clone(),
        });
        let obj = value.object_ref().expect("поток").as_dyn();

        // Первый `Закрыть()` — отказ дескриптора; носитель сохранён.
        assert!(close(obj).is_err());
        assert_eq!(closes.get(), 1, "дескриптор закрывали один раз");
        // Поток полузакрыт: `Размер()` ещё работает, позиция и признаки целы.
        assert!(data.borrow().len("Размер").is_ok(), "Размер() после отказа");
        assert_eq!(data.borrow().position(), 7, "позиция цела");
        assert!(!data.borrow().can_write, "признак записи цел");
        assert!(data.borrow().can_read, "признак чтения цел");

        // Второй `Закрыть()` доходит до дескриптора и успешен.
        assert!(close(obj).is_ok());
        assert_eq!(closes.get(), 2, "второй close дошёл до дескриптора");
        // Теперь поток закрыт: `Размер()` отвечает ошибкой.
        assert!(data.borrow().len("Размер").is_err(), "поток закрыт");

        // Третий `Закрыть()` успешен и до дескриптора уже не идёт.
        assert!(close(obj).is_ok());
        assert_eq!(closes.get(), 2, "третий close дескриптора не касается");
    }

    /// Второй долгоживущий дескриптор (после `ЗаписьТекста`) — `ФайловыйПоток`
    /// — тоже открывается через файловую систему СЕССИИ (ABI-G0): in-memory
    /// ФС с рабочим `open`/`FileHandle` проводит запись, чтение и закрытие,
    /// реального диска не касаясь.
    #[test]
    fn a_file_stream_opens_through_the_session_file_system() {
        use std::collections::HashMap;
        use std::io::Cursor;

        type Store = Rc<RefCell<HashMap<String, Vec<u8>>>>;

        #[derive(Debug, Default)]
        struct MemFs {
            store: Store,
        }

        #[derive(Debug)]
        struct MemHandle {
            store: Store,
            path: String,
            cursor: Cursor<Vec<u8>>,
        }

        impl MemHandle {
            fn sync(&self) {
                self.store
                    .borrow_mut()
                    .insert(self.path.clone(), self.cursor.get_ref().clone());
            }
        }

        impl std::io::Read for MemHandle {
            fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
                self.cursor.read(buf)
            }
        }
        impl std::io::Write for MemHandle {
            fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
                let n = self.cursor.write(buf)?;
                self.sync();
                Ok(n)
            }
            fn flush(&mut self) -> std::io::Result<()> {
                self.sync();
                Ok(())
            }
        }
        impl std::io::Seek for MemHandle {
            fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
                self.cursor.seek(pos)
            }
        }
        impl FileHandle for MemHandle {
            fn len(&self) -> std::io::Result<u64> {
                Ok(self.cursor.get_ref().len() as u64)
            }
            fn close(&mut self) -> std::io::Result<()> {
                self.sync();
                Ok(())
            }
        }

        impl bsl_rt::FileSystem for MemFs {
            fn read(&self, path: &str) -> std::io::Result<Vec<u8>> {
                self.store.borrow().get(path).cloned().ok_or_else(|| {
                    std::io::Error::new(std::io::ErrorKind::NotFound, path.to_string())
                })
            }
            fn write(&self, path: &str, data: &[u8]) -> std::io::Result<()> {
                self.store
                    .borrow_mut()
                    .insert(path.to_string(), data.to_vec());
                Ok(())
            }
            fn metadata(&self, path: &str) -> std::io::Result<bsl_rt::FileMetadata> {
                if self.store.borrow().contains_key(path) {
                    Ok(bsl_rt::FileMetadata::file(Some(0)))
                } else {
                    Err(std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        path.to_string(),
                    ))
                }
            }
            fn read_dir<'fs>(
                &'fs self,
                path: &str,
            ) -> std::io::Result<Box<dyn Iterator<Item = std::io::Result<bsl_rt::DirEntry>> + 'fs>>
            {
                Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    path.to_string(),
                ))
            }
            fn create_dir_all(&self, _path: &str) -> std::io::Result<()> {
                Ok(())
            }
            fn open(
                &self,
                path: &str,
                options: FileOpenOptions,
            ) -> std::io::Result<Box<dyn FileHandle>> {
                let existing = self.store.borrow().get(path).cloned();
                let bytes = if options.should_truncate() {
                    Vec::new()
                } else {
                    existing.unwrap_or_default()
                };
                Ok(Box::new(MemHandle {
                    store: Rc::clone(&self.store),
                    path: path.to_string(),
                    cursor: Cursor::new(bytes),
                }))
            }
        }

        let mem = MemFs::default();
        let data = open_file_data(
            "/поток.bin",
            FileOpenMode::Create,
            FileAccess::ReadWrite,
            &mem,
        )
        .unwrap();

        // Запись идёт через дескриптор in-memory ФС.
        data.borrow_mut().write_bytes(b"hello", "Записать").unwrap();
        assert_eq!(
            mem.store.borrow().get("/поток.bin").map(Vec::as_slice),
            Some(b"hello".as_slice()),
            "запись легла в память"
        );

        // Чтение с начала.
        data.borrow_mut().set_position(0);
        let got = data.borrow_mut().read_bytes(5, "Прочитать").unwrap();
        assert_eq!(got, b"hello");

        // Закрытие через объект — по закону `close`.
        let value = stream_value(StreamKind::File, data);
        close(value.object_ref().expect("поток").as_dyn()).unwrap();
        assert!(
            !std::path::Path::new("/поток.bin").exists(),
            "реального диска работа не касалась"
        );
    }
}
