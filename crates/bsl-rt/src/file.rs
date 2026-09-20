//! Объект пути, связанный с файловой системой создавшего его сеанса.
//! Первые серверные пробы: `file-object.platform.txt`. Метаданные не кешируются.

use std::{cell::RefCell, rc::Rc};

use crate::{
    Arity, BslDate, BslNumber, BslString, BslValue, CallContext, FileMetadata, FileSystem,
    MethodDescriptor, ObjectMembersDescriptor, ObjectProtocol, PropertyDescriptor, RtError,
    RtResult, TypeDescriptor, receiver_of,
};

pub(crate) static FILE_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "Файл",
    type_display: "File",
    type_names: &["File"],
};

/// Вид чтения метаданных; переносится в фоновый поток вместе с путём.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileMetadataQuery {
    Exists,
    IsFile,
    IsDirectory,
    Size,
    ReadOnly,
    Hidden,
    ModificationTime,
    ModificationUniversalTime,
}

/// Подготовленная запись атрибута без BSL-значений и часового пояса.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileMetadataUpdate {
    ReadOnly(bool),
    Hidden(bool),
    Modified(i64),
}

impl FileMetadataUpdate {
    pub(crate) fn apply(self, files: &dyn FileSystem, path: &str) -> std::io::Result<()> {
        match self {
            Self::ReadOnly(value) => files.set_read_only(path, value),
            Self::Hidden(value) => files.set_hidden(path, value),
            Self::Modified(seconds) => files.set_modified(path, seconds),
        }
    }
}

impl FileMetadataQuery {
    pub(crate) fn resolve(
        self,
        metadata: std::io::Result<FileMetadata>,
        zone: &dyn crate::TimeZone,
    ) -> RtResult<BslValue> {
        if self == Self::Exists {
            return match metadata {
                Ok(_) => Ok(BslValue::Boolean(true)),
                Err(error) if existence_error_is_false(&error) => Ok(BslValue::Boolean(false)),
                Err(error) => Err(io_error(error)),
            };
        }
        let metadata = metadata.map_err(io_error)?;
        match self {
            Self::Exists => unreachable!("существование обработано до извлечения метаданных"),
            Self::IsFile => Ok(BslValue::Boolean(metadata.is_file())),
            Self::IsDirectory => Ok(BslValue::Boolean(metadata.is_dir())),
            Self::Size => metadata_size(metadata),
            Self::ReadOnly => metadata.read_only().map(BslValue::Boolean).ok_or_else(|| {
                RtError::IoError("атрибут только чтения не предоставлен host".into())
            }),
            Self::Hidden => metadata
                .hidden()
                .map(BslValue::Boolean)
                .ok_or_else(|| RtError::IoError("невидимость не предоставлена host".into())),
            Self::ModificationTime => modification_time(metadata, true, zone),
            Self::ModificationUniversalTime => modification_time(metadata, false, zone),
        }
    }
}

fn existence_error_is_false(error: &std::io::Error) -> bool {
    if matches!(
        error.kind(),
        std::io::ErrorKind::NotFound
            | std::io::ErrorKind::NotADirectory
            | std::io::ErrorKind::PermissionDenied
    ) {
        return true;
    }
    #[cfg(target_os = "linux")]
    {
        // file-object-host-edges: серверная 1С скрывает `ENAMETOOLONG` и
        // `ELOOP` только в `Существует`; остальные методы сохраняют ошибку.
        matches!(error.raw_os_error(), Some(36 | 40))
    }
    #[cfg(not(target_os = "linux"))]
    false
}

#[derive(Clone)]
struct FileObject {
    files: Rc<dyn FileSystem>,
    zone: Rc<dyn crate::TimeZone>,
    separator: String,
    path: Rc<RefCell<FilePath>>,
}

struct FilePath {
    full_name: String,
    name: String,
    parent: String,
    base_name: String,
    extension: String,
}

impl std::fmt::Debug for FileObject {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FileObject")
            .field("full_name", &self.full_name())
            .finish_non_exhaustive()
    }
}

fn string(value: &str) -> BslValue {
    BslValue::Str(BslString::from_str(value))
}

fn io_error(error: std::io::Error) -> RtError {
    RtError::IoError(error.to_string())
}

impl FileObject {
    fn full_name(&self) -> String {
        self.path.borrow().full_name.clone()
    }

    fn operation_path(&self) -> String {
        operation_path(&self.full_name(), &self.separator)
    }

    fn read(&self, query: FileMetadataQuery) -> RtResult<BslValue> {
        query.resolve(
            self.files.metadata(&self.operation_path()),
            self.zone.as_ref(),
        )
    }

    fn update_async(
        &self,
        update: RtResult<FileMetadataUpdate>,
        ctx: &mut CallContext<'_>,
    ) -> RtResult<BslValue> {
        ctx.spawn_file_operation(
            update.map(|update| crate::FileOperationRequest::UpdateMetadata {
                path: self.operation_path(),
                update,
            }),
            self.files.clone(),
            self.zone.clone(),
        )
    }
}

impl ObjectProtocol for FileObject {
    fn identity_key(&self) -> Option<(usize, usize)> {
        Some((Rc::as_ptr(&self.path) as usize, 0))
    }
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &FILE_TYPE
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        METHODS
    }

    fn property_table(&self) -> &'static [PropertyDescriptor] {
        PROPERTIES
    }
}

// `НЕ ИЗМЕРЕНО(FILE.EDGE_BEHAVIOR)`: прочие типы объектов, I/O-конструкторы,
// управляющие символы и классы ошибок требуют проб. Измеренные относительные
// и корневые пути не превращаются в абсолютные и сохраняют число `/` корня.
pub(crate) fn normalize_path(path: &str, separator: &str) -> String {
    // file-create-layout: конечные ASCII-пробелы не входят в путь,
    // но пробел перед внутренним разделителем и начальный пробел сохраняются.
    let path = path.trim_end_matches(' ').replace(['\\', '/'], separator);
    // Измеренные `/` и `///` не сворачиваются в один корневой разделитель.
    if !path.is_empty() && path.split(separator).all(str::is_empty) {
        return path;
    }
    let absolute = path.starts_with(separator);
    let mut parts = Vec::new();
    for part in path.split(separator) {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            other => parts.push(other),
        }
    }
    let body = parts.join(separator);
    if absolute {
        format!("{separator}{body}")
    } else {
        body
    }
}

pub(crate) fn construct_file(ctx: &mut CallContext<'_>, args: &[BslValue]) -> RtResult<BslValue> {
    // `НЕ ИЗМЕРЕНО(FILE.EDGE_BEHAVIOR)`: прочие классы объектов и ошибки
    // их представления требуют проб; измеренные типы используют Строка.
    let path = match args {
        [] => String::new(),
        [path] => ctx.format_value(path, None)?,
        _ => {
            return Err(RtError::InvalidBytecode(
                "неверная арность конструктора Файл",
            ));
        }
    };
    let files = ctx.files_rc()?;
    let separator = files.path_separator().map_err(io_error)?;
    if separator.is_empty() {
        return Err(RtError::IoError(
            "host вернул пустой разделитель пути".into(),
        ));
    }
    Ok(file_from_path(path, files, ctx.zone_rc()?, &separator))
}

/// Создаёт объект пути из сервисов сеанса. Разделитель уже проверен вызывающим.
/// Поиск и BSL-конструктор должны пользоваться одной нормализацией и таблицей членов.
pub(crate) fn file_from_path(
    path: String,
    files: Rc<dyn FileSystem>,
    zone: Rc<dyn crate::TimeZone>,
    separator: &str,
) -> BslValue {
    BslValue::new_object(FileObject {
        files,
        zone,
        separator: separator.to_owned(),
        path: Rc::new(RefCell::new(file_path(&path, separator))),
    })
}

fn is_measured_directory_noop_path(path: &str) -> bool {
    matches!(
        path,
        "." | "./"
            | ".."
            | "../"
            | "./."
            | "././"
            | ".//"
            | "../."
            | ".././"
            | "..//"
            | "./.."
            | "./../"
            | "./../."
            | "../../"
            | "/./"
            | "/../"
    )
}

/// Возвращает исходную строку для измеренных форм `СоздатьКаталог`, которые
/// успешно завершаются без обращения к файловому host.
#[must_use]
pub fn prepare_create_directory_noop(args: &[BslValue]) -> Option<BslValue> {
    let [value @ BslValue::Str(path)] = args else {
        return None;
    };
    is_measured_directory_noop_path(&path.to_string()).then(|| value.clone())
}

// file-path-whitespace: набор Unicode White_Space дополнен U+001C…U+001F.
// Это правило Файл и поиска, не общее изменение строковых СокрЛП/СокрП.
pub(crate) fn trim_path_end(path: &str) -> &str {
    path.trim_end_matches(|ch: char| ch.is_whitespace() || matches!(ch, '\u{1c}'..='\u{1f}'))
}

/// Путь файловой операции отделён от лексического представления `Файл`:
/// NUL остаётся видимым в свойствах, но завершает путь до обращения к host.
/// Остальная нормализация переиспользует тот же разбор и измеренное удаление
/// конечных пробельных символов, что и `ПолноеИмя`.
pub(crate) fn operation_path(path: &str, separator: &str) -> String {
    let path = path.split('\0').next().unwrap_or_default();
    let mut normalized = normalize_path(path, separator);
    normalized.truncate(trim_path_end(&normalized).len());
    normalized
}

/// Готовит измеренный операционный путь перед обращением к файловой системе
/// host: NUL завершает имя, конечные пробельные символы удаляются, а
/// разделители приводятся к сообщённому host виду.
///
/// Лексическое имя объекта или потока эта функция не меняет.
///
/// # Errors
///
/// Возвращает ошибку host при чтении разделителя пути и отвергает пустой
/// разделитель, с которым нормализация сегментов неоднозначна.
pub fn prepare_file_operation_path(path: &str, files: &dyn FileSystem) -> std::io::Result<String> {
    let separator = files.path_separator()?;
    if separator.is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "host вернул пустой разделитель пути",
        ));
    }
    Ok(operation_path(path, &separator))
}

fn file_path(path: &str, separator: &str) -> FilePath {
    let mut full_name = normalize_path(path, separator);
    full_name.truncate(trim_path_end(&full_name).len());
    let name_start = full_name
        .rfind(separator)
        .map_or(0, |index| index + separator.len());
    if let Some(star) = full_name[name_start..]
        .find('*')
        .map(|index| name_start + index)
    {
        let parent = full_name.rsplit_once(separator).map_or_else(
            || "sfile://".to_owned(),
            |(parent, _)| format!("sfile://{parent}{separator}"),
        );
        let name = full_name[name_start..star].to_owned();
        return FilePath {
            full_name: format!("sfile://sfile://{full_name}"),
            name: name.clone(),
            parent,
            base_name: name,
            extension: String::new(),
        };
    }
    let (parent, name) = full_name.rsplit_once(separator).map_or_else(
        || (String::new(), full_name.clone()),
        |(parent, name)| (format!("{parent}{separator}"), name.to_owned()),
    );
    let (base_name, extension) = match name.rfind('.') {
        Some(index) => (name[..index].to_owned(), name[index..].to_owned()),
        None => (name.clone(), String::new()),
    };
    FilePath {
        full_name,
        name,
        parent,
        base_name,
        extension,
    }
}

macro_rules! property {
    ($get:ident, $field:ident) => {
        fn $get(receiver: &dyn ObjectProtocol, _ctx: &mut CallContext<'_>) -> RtResult<BslValue> {
            Ok(string(
                &receiver_of::<FileObject>(receiver, "Файл")?
                    .path
                    .borrow()
                    .$field,
            ))
        }
    };
}

property!(get_full_name, full_name);
property!(get_name, name);
property!(get_parent, parent);
property!(get_base_name, base_name);
property!(get_extension, extension);

fn exists(
    receiver: &dyn ObjectProtocol,
    _args: &[BslValue],
    _ctx: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let file = receiver_of::<FileObject>(receiver, "Существует")?;
    // `НЕ ИЗМЕРЕНО(FILE.EDGE_BEHAVIOR)`: прочие классы ошибок host остаются
    // ловимыми. Измеренные NotFound/NotADirectory/PermissionDenied, а на
    // Linux также ENAMETOOLONG/ELOOP, превращаются в Ложь.
    file.read(FileMetadataQuery::Exists)
}

fn is_file(
    receiver: &dyn ObjectProtocol,
    _args: &[BslValue],
    _ctx: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    receiver_of::<FileObject>(receiver, "ЭтоФайл")?.read(FileMetadataQuery::IsFile)
}

fn is_directory(
    receiver: &dyn ObjectProtocol,
    _args: &[BslValue],
    _ctx: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    receiver_of::<FileObject>(receiver, "ЭтоКаталог")?.read(FileMetadataQuery::IsDirectory)
}

fn size(
    receiver: &dyn ObjectProtocol,
    _args: &[BslValue],
    _ctx: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    receiver_of::<FileObject>(receiver, "Размер")?.read(FileMetadataQuery::Size)
}

fn metadata_size(metadata: FileMetadata) -> RtResult<BslValue> {
    if !metadata.is_file() {
        return Err(RtError::IoError(
            "размер доступен только для обычного файла".into(),
        ));
    }
    let size = metadata
        .size()
        .ok_or_else(|| RtError::IoError("размер файла не предоставлен host".into()))?;
    Ok(BslValue::Number(BslNumber::from_i128(i128::from(size))))
}

fn modification_time(
    metadata: FileMetadata,
    local: bool,
    zone: &dyn crate::TimeZone,
) -> RtResult<BslValue> {
    let op = if local {
        "ПолучитьВремяИзменения"
    } else {
        "ПолучитьУниверсальноеВремяИзменения"
    };
    let seconds = metadata
        .modified()
        .ok_or_else(|| RtError::IoError("время изменения не предоставлено host".into()))?;
    // `НЕ ИЗМЕРЕНО(FILE.TIME_EDGE_BEHAVIOR)`: полный диапазон переполнения
    // UTC-счётчика ещё не измерен. Минимальный UTC-результат 1С имеет поля
    // 0-0-0, не представимые BslDate; пока это явная ошибка, не ложный oracle.
    let date = if local {
        crate::local_date_from_utc_seconds(seconds, op, zone).unwrap_or(BslDate::empty())
    } else {
        let ticks = (seconds as u64)
            .wrapping_mul(10_000)
            .wrapping_add(crate::UNIX_EPOCH_SECONDS as u64 * 10_000);
        BslDate::from_raw_ticks(ticks)
    };
    Ok(BslValue::Date(date))
}

fn prepare_modification_time(
    value: &BslValue,
    local: bool,
    zone: &dyn crate::TimeZone,
) -> RtResult<i64> {
    let op = if local {
        "УстановитьВремяИзменения"
    } else {
        "УстановитьУниверсальноеВремяИзменения"
    };
    // `НЕ ИЗМЕРЕНО(FILE.TIME_EDGE_BEHAVIOR)`: измерены строки из 8/14 цифр,
    // шесть краевых пробельных символов и NUL; остальные формы требуют проб.
    let date = match value {
        BslValue::Date(date) => *date,
        BslValue::Str(value) => {
            parse_file_time_string(&value.to_string()).ok_or(RtError::DateOutOfRange { op })?
        }
        _ => {
            return Err(RtError::TypeError {
                expected: "Дата",
                op,
            });
        }
    };
    let seconds = crate::pseudo_unix_seconds(date);
    let seconds = if local {
        let offset = zone.offset_for_local(seconds);
        let candidate = seconds - i64::from(offset);
        // В пропуске зоны файловый setter, в отличие от общей конверсии,
        // записал -1; измерено на Moscow 2011-03-27 02:30.
        if zone.offset_seconds(candidate) != offset {
            -1
        } else {
            candidate
        }
    } else {
        // `НЕ ИЗМЕРЕНО(FILE.TIME_EDGE_BEHAVIOR)`: перенос наблюдавшегося
        // unsigned-переполнения только на BSL-границе, не внутрь host.
        ((seconds as u64).wrapping_mul(10_000) / 10_000) as i64
    };
    Ok(seconds)
}

fn parse_file_time_string(value: &str) -> Option<BslDate> {
    // file-time-string-arguments: 1С прекращает вход по первому NUL и снимает
    // только измеренные внешние пробелы. U+200B и U+FEFF не снимаются.
    let value = value
        .split('\0')
        .next()
        .unwrap_or_default()
        .trim_matches([' ', '\t', '\n', '\r', '\u{85}', '\u{a0}']);
    // В файловых setter восемь нулей означают ту же пустую дату, что и
    // четырнадцать нулей. Общий конструктор `Дата` этим не меняется.
    if value == "00000000" {
        Some(BslDate::empty())
    } else {
        BslDate::parse_digits(value)
    }
}

fn set_modification_time(
    receiver: &dyn ObjectProtocol,
    args: &[BslValue],
    local: bool,
) -> RtResult<BslValue> {
    let file = receiver_of::<FileObject>(
        receiver,
        if local {
            "УстановитьВремяИзменения"
        } else {
            "УстановитьУниверсальноеВремяИзменения"
        },
    )?;
    let seconds = prepare_modification_time(&args[0], local, file.zone.as_ref())?;
    FileMetadataUpdate::Modified(seconds)
        .apply(file.files.as_ref(), &file.operation_path())
        .map_err(io_error)?;
    Ok(BslValue::Undefined)
}

fn get_local_time(
    receiver: &dyn ObjectProtocol,
    _args: &[BslValue],
    _ctx: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    receiver_of::<FileObject>(receiver, "ПолучитьВремяИзменения")?
        .read(FileMetadataQuery::ModificationTime)
}

fn get_utc_time(
    receiver: &dyn ObjectProtocol,
    _args: &[BslValue],
    _ctx: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    receiver_of::<FileObject>(receiver, "ПолучитьУниверсальноеВремяИзменения")?
        .read(FileMetadataQuery::ModificationUniversalTime)
}

fn set_local_time(
    receiver: &dyn ObjectProtocol,
    args: &[BslValue],
    _ctx: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    set_modification_time(receiver, args, true)
}

fn set_utc_time(
    receiver: &dyn ObjectProtocol,
    args: &[BslValue],
    _ctx: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    set_modification_time(receiver, args, false)
}

fn get_read_only(
    receiver: &dyn ObjectProtocol,
    _args: &[BslValue],
    _ctx: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    receiver_of::<FileObject>(receiver, "ПолучитьТолькоЧтение")?.read(FileMetadataQuery::ReadOnly)
}

pub(crate) fn boolean_argument(value: &BslValue, op: &'static str) -> RtResult<bool> {
    // file-search-recursion-arguments, включая async: условные слова Да/Нет
    // здесь не принимаются. Остальные преобразования переиспользуют общую
    // конверсию.
    if let BslValue::Str(text) = value {
        let text = text.to_string();
        if crate::folded_eq(text.trim(), "Да") || crate::folded_eq(text.trim(), "Нет") {
            return Err(RtError::TypeError {
                expected: "Булево",
                op,
            });
        }
    }
    value.as_condition()
}

fn attribute_argument(value: &BslValue, op: &'static str) -> RtResult<bool> {
    // Setter атрибута имеет собственный измеренный словарь: в отличие от
    // общего условия и флага рекурсии он принимает `yes`/`no`, но не Да/Нет.
    if let BslValue::Str(text) = value {
        let text = text.to_string();
        let text = text.trim_matches(|character: char| {
            character.is_whitespace()
                || matches!(character, '\u{001c}' | '\u{001d}' | '\u{001e}' | '\u{001f}')
        });
        if crate::folded_eq(text, "Истина")
            || crate::folded_eq(text, "True")
            || crate::folded_eq(text, "yes")
        {
            return Ok(true);
        }
        if crate::folded_eq(text, "Ложь")
            || crate::folded_eq(text, "False")
            || crate::folded_eq(text, "no")
        {
            return Ok(false);
        }
        return Err(RtError::TypeError {
            expected: "Булево",
            op,
        });
    }
    value.as_condition()
}

fn set_read_only(
    receiver: &dyn ObjectProtocol,
    args: &[BslValue],
    _ctx: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let file = receiver_of::<FileObject>(receiver, "УстановитьТолькоЧтение")?;
    let value = attribute_argument(&args[0], "Файл")?;
    FileMetadataUpdate::ReadOnly(value)
        .apply(file.files.as_ref(), &file.operation_path())
        .map_err(io_error)?;
    Ok(BslValue::Undefined)
}

fn get_hidden(
    receiver: &dyn ObjectProtocol,
    _args: &[BslValue],
    _ctx: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    receiver_of::<FileObject>(receiver, "ПолучитьНевидимость")?.read(FileMetadataQuery::Hidden)
}

fn set_hidden(
    receiver: &dyn ObjectProtocol,
    args: &[BslValue],
    _ctx: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let file = receiver_of::<FileObject>(receiver, "УстановитьНевидимость")?;
    let value = attribute_argument(&args[0], "Файл")?;
    FileMetadataUpdate::Hidden(value)
        .apply(file.files.as_ref(), &file.operation_path())
        .map_err(io_error)?;
    Ok(BslValue::Undefined)
}

macro_rules! async_reader {
    ($handler:ident, $query:ident) => {
        fn $handler(
            receiver: &dyn ObjectProtocol,
            _args: &[BslValue],
            ctx: &mut CallContext<'_>,
        ) -> RtResult<BslValue> {
            let file = receiver_of::<FileObject>(receiver, "Файл")?;
            ctx.spawn_file_operation(
                Ok(crate::FileOperationRequest::Metadata {
                    path: file.operation_path(),
                    query: FileMetadataQuery::$query,
                }),
                file.files.clone(),
                file.zone.clone(),
            )
        }
    };
}

async_reader!(exists_async, Exists);
async_reader!(is_file_async, IsFile);
async_reader!(is_directory_async, IsDirectory);
async_reader!(size_async, Size);
async_reader!(read_only_async, ReadOnly);
async_reader!(hidden_async, Hidden);
async_reader!(local_time_async, ModificationTime);
async_reader!(utc_time_async, ModificationUniversalTime);

fn set_read_only_async(
    receiver: &dyn ObjectProtocol,
    args: &[BslValue],
    ctx: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let file = receiver_of::<FileObject>(receiver, "УстановитьТолькоЧтениеАсинх")?;
    file.update_async(
        attribute_argument(&args[0], "Файл").map(FileMetadataUpdate::ReadOnly),
        ctx,
    )
}

fn set_hidden_async(
    receiver: &dyn ObjectProtocol,
    args: &[BslValue],
    ctx: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let file = receiver_of::<FileObject>(receiver, "УстановитьНевидимостьАсинх")?;
    file.update_async(
        attribute_argument(&args[0], "Файл").map(FileMetadataUpdate::Hidden),
        ctx,
    )
}

fn set_local_time_async(
    receiver: &dyn ObjectProtocol,
    args: &[BslValue],
    ctx: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let file = receiver_of::<FileObject>(receiver, "УстановитьВремяИзмененияАсинх")?;
    file.update_async(
        prepare_modification_time(&args[0], true, file.zone.as_ref())
            .map(FileMetadataUpdate::Modified),
        ctx,
    )
}

fn set_utc_time_async(
    receiver: &dyn ObjectProtocol,
    args: &[BslValue],
    ctx: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let file = receiver_of::<FileObject>(receiver, "УстановитьУниверсальноеВремяИзмененияАсинх")?;
    file.update_async(
        prepare_modification_time(&args[0], false, file.zone.as_ref())
            .map(FileMetadataUpdate::Modified),
        ctx,
    )
}

fn initialize_async(
    receiver: &dyn ObjectProtocol,
    args: &[BslValue],
    ctx: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let file = receiver_of::<FileObject>(receiver, "ИнициализироватьАсинх")?;
    let path = match ctx.format_value(&args[0], None) {
        Ok(path) => file_path(&path, &file.separator),
        Err(error) => return ctx.ready_file_promise(Err(error)),
    };
    // Значение разделяет путь с приёмником. При отказе исполнителя
    // исходный объект не меняется; при успехе мутация видна до возврата.
    let promise = ctx.ready_file_promise(Ok(BslValue::new_object(file.clone())))?;
    *file.path.borrow_mut() = path;
    Ok(promise)
}

fn unsupported_mobile_library_method(
    receiver: &dyn ObjectProtocol,
    _: &[BslValue],
    _: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    receiver_of::<FileObject>(receiver, "Файл")?;
    // `НЕ ИЗМЕРЕНО(FILE.MOBILE_LIBRARY)`: успешный мобильный контекст
    // потребует отдельной host-возможности и модели значения Картинка.
    Err(crate::ComponentError::raise(
        crate::PACKAGE_NAME,
        "контекст",
        "метод мобильной библиотеки файлов недоступен в этом контексте",
    ))
}

// Begin использует ту же подготовку и тот же нейтральный запрос, что Async.
// Описание проверяется до подготовки операции; ошибки значения setter
// сохраняются для отложенного обработчика (file-begin-arity.platform.txt).
macro_rules! begin_reader {
    ($handler:ident, $query:ident) => {
        fn $handler(
            receiver: &dyn ObjectProtocol,
            args: &[BslValue],
            ctx: &mut CallContext<'_>,
        ) -> RtResult<BslValue> {
            let description = crate::NotificationDescription::from_value(&args[0])?;
            let file = receiver_of::<FileObject>(receiver, "Файл")?;
            ctx.begin_file_operation(
                crate::FileNotificationOperation::Pending {
                    request: Ok(crate::FileOperationRequest::Metadata {
                        path: file.operation_path(),
                        query: FileMetadataQuery::$query,
                    }),
                    files: file.files.clone(),
                    zone: file.zone.clone(),
                },
                description,
                true,
            )?;
            Ok(BslValue::Undefined)
        }
    };
}

begin_reader!(begin_exists, Exists);
begin_reader!(begin_is_file, IsFile);
begin_reader!(begin_is_directory, IsDirectory);
begin_reader!(begin_size, Size);
begin_reader!(begin_read_only, ReadOnly);
begin_reader!(begin_hidden, Hidden);
begin_reader!(begin_local_time, ModificationTime);
begin_reader!(begin_utc_time, ModificationUniversalTime);

macro_rules! begin_writer {
    ($handler:ident, $prepare:expr) => {
        fn $handler(
            receiver: &dyn ObjectProtocol,
            args: &[BslValue],
            ctx: &mut CallContext<'_>,
        ) -> RtResult<BslValue> {
            let description = crate::NotificationDescription::from_value(&args[0])?;
            let file = receiver_of::<FileObject>(receiver, "Файл")?;
            let update: RtResult<FileMetadataUpdate> = $prepare(&args[1], file);
            ctx.begin_file_operation(
                crate::FileNotificationOperation::Pending {
                    request: update.map(|update| crate::FileOperationRequest::UpdateMetadata {
                        path: file.operation_path(),
                        update,
                    }),
                    files: file.files.clone(),
                    zone: file.zone.clone(),
                },
                description,
                false,
            )?;
            Ok(BslValue::Undefined)
        }
    };
}

begin_writer!(begin_set_read_only, |value, _: &FileObject| {
    attribute_argument(value, "Файл").map(FileMetadataUpdate::ReadOnly)
});
begin_writer!(begin_set_hidden, |value, _: &FileObject| {
    attribute_argument(value, "Файл").map(FileMetadataUpdate::Hidden)
});
begin_writer!(begin_set_local_time, |value, file: &FileObject| {
    prepare_modification_time(value, true, file.zone.as_ref()).map(FileMetadataUpdate::Modified)
});
begin_writer!(begin_set_utc_time, |value, file: &FileObject| {
    prepare_modification_time(value, false, file.zone.as_ref()).map(FileMetadataUpdate::Modified)
});

fn begin_initialize(
    receiver: &dyn ObjectProtocol,
    args: &[BslValue],
    ctx: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let description = crate::NotificationDescription::from_value(&args[0])?;
    let file = receiver_of::<FileObject>(receiver, "НачатьИнициализацию")?;
    let path = ctx
        .format_value(&args[1], None)
        .map(|path| file_path(&path, &file.separator));
    let result = path
        .as_ref()
        .map(|_| BslValue::new_object(file.clone()))
        .map_err(Clone::clone);
    ctx.begin_file_operation(
        crate::FileNotificationOperation::Ready(result),
        description,
        true,
    )?;
    // Регистрация не исполняет обработчик. Отказ host оставляет путь прежним.
    if let Ok(path) = path {
        *file.path.borrow_mut() = path;
    }
    Ok(BslValue::Undefined)
}

static METHODS: &[MethodDescriptor] = &[
    MethodDescriptor::new(
        &[
            "ПолучитьПредставлениеФайлаБиблиотекиМобильногоУстройства",
            "GetMobileDeviceLibraryFilePresentation",
        ],
        Arity::exact(0),
        unsupported_mobile_library_method,
    ),
    MethodDescriptor::new(
        &[
            "ПолучитьКартинкуПредставленияФайлаБиблиотекиМобильногоУстройства",
            "GetMobileDeviceLibraryFileThumbnail",
        ],
        Arity::exact(0),
        unsupported_mobile_library_method,
    ),
    MethodDescriptor::new(
        &[
            "ПолучитьКартинкуПредставленияФайлаБиблиотекиМобильногоУстройстваАсинх",
            "GetMobileDeviceLibraryFileThumbnailAsync",
        ],
        Arity::exact(0),
        unsupported_mobile_library_method,
    ),
    MethodDescriptor::new(
        &[
            "НачатьПолучениеКартинкиПредставленияФайлаБиблиотекиМобильногоУстройства",
            "BeginGetMobileDeviceLibraryFileThumbnail",
        ],
        Arity::exact(1),
        unsupported_mobile_library_method,
    )
    .as_procedure(),
    MethodDescriptor::new(
        &["НачатьИнициализацию", "BeginInitialization"],
        Arity::exact(2),
        begin_initialize,
    )
    .as_procedure(),
    MethodDescriptor::new(
        &["НачатьПроверкуСуществования", "BeginCheckingExistence"],
        Arity::exact(1),
        begin_exists,
    )
    .as_procedure(),
    MethodDescriptor::new(
        &["НачатьПроверкуЭтоФайл", "BeginCheckingIsFile"],
        Arity::exact(1),
        begin_is_file,
    )
    .as_procedure(),
    MethodDescriptor::new(
        &["НачатьПроверкуЭтоКаталог", "BeginCheckingIsDirectory"],
        Arity::exact(1),
        begin_is_directory,
    )
    .as_procedure(),
    MethodDescriptor::new(
        &["НачатьПолучениеРазмера", "BeginGettingSize"],
        Arity::exact(1),
        begin_size,
    )
    .as_procedure(),
    MethodDescriptor::new(
        &["НачатьПолучениеТолькоЧтения", "BeginGettingReadOnly"],
        Arity::exact(1),
        begin_read_only,
    )
    .as_procedure(),
    MethodDescriptor::new(
        &["НачатьПолучениеНевидимости", "BeginGettingHidden"],
        Arity::exact(1),
        begin_hidden,
    )
    .as_procedure(),
    MethodDescriptor::new(
        &[
            "НачатьПолучениеВремениИзменения",
            "BeginGettingModificationTime",
        ],
        Arity::exact(1),
        begin_local_time,
    )
    .as_procedure(),
    MethodDescriptor::new(
        &[
            "НачатьПолучениеУниверсальногоВремениИзменения",
            "BeginGettingModificationUniversalTime",
        ],
        Arity::exact(1),
        begin_utc_time,
    )
    .as_procedure(),
    MethodDescriptor::new(
        &["НачатьУстановкуТолькоЧтения", "BeginSettingReadOnly"],
        Arity::exact(2),
        begin_set_read_only,
    )
    .as_procedure(),
    MethodDescriptor::new(
        &["НачатьУстановкуНевидимости", "BeginSettingHidden"],
        Arity::exact(2),
        begin_set_hidden,
    )
    .as_procedure(),
    MethodDescriptor::new(
        &[
            "НачатьУстановкуВремениИзменения",
            "BeginSettingModificationTime",
        ],
        Arity::exact(2),
        begin_set_local_time,
    )
    .as_procedure(),
    MethodDescriptor::new(
        &[
            "НачатьУстановкуУниверсальногоВремениИзменения",
            "BeginSettingModificationUniversalTime",
        ],
        Arity::exact(2),
        begin_set_utc_time,
    )
    .as_procedure(),
    MethodDescriptor::new(
        &["ИнициализироватьАсинх", "InitializeAsync"],
        Arity::exact(1),
        initialize_async,
    ),
    MethodDescriptor::new(
        &["УстановитьТолькоЧтениеАсинх", "SetReadOnlyAsync"],
        Arity::exact(1),
        set_read_only_async,
    ),
    MethodDescriptor::new(
        &["УстановитьНевидимостьАсинх", "SetHiddenAsync"],
        Arity::exact(1),
        set_hidden_async,
    ),
    MethodDescriptor::new(
        &["УстановитьВремяИзмененияАсинх", "SetModificationTimeAsync"],
        Arity::exact(1),
        set_local_time_async,
    ),
    MethodDescriptor::new(
        &[
            "УстановитьУниверсальноеВремяИзмененияАсинх",
            "SetModificationUniversalTimeAsync",
        ],
        Arity::exact(1),
        set_utc_time_async,
    ),
    MethodDescriptor::new(
        &["СуществуетАсинх", "ExistsAsync"],
        Arity::exact(0),
        exists_async,
    ),
    MethodDescriptor::new(
        &["ЭтоФайлАсинх", "IsFileAsync"],
        Arity::exact(0),
        is_file_async,
    ),
    MethodDescriptor::new(
        &["ЭтоКаталогАсинх", "IsDirectoryAsync"],
        Arity::exact(0),
        is_directory_async,
    ),
    MethodDescriptor::new(&["РазмерАсинх", "SizeAsync"], Arity::exact(0), size_async),
    MethodDescriptor::new(
        &["ПолучитьТолькоЧтениеАсинх", "GetReadOnlyAsync"],
        Arity::exact(0),
        read_only_async,
    ),
    MethodDescriptor::new(
        &["ПолучитьНевидимостьАсинх", "GetHiddenAsync"],
        Arity::exact(0),
        hidden_async,
    ),
    MethodDescriptor::new(
        &["ПолучитьВремяИзмененияАсинх", "GetModificationTimeAsync"],
        Arity::exact(0),
        local_time_async,
    ),
    MethodDescriptor::new(
        &[
            "ПолучитьУниверсальноеВремяИзмененияАсинх",
            "GetModificationUniversalTimeAsync",
        ],
        Arity::exact(0),
        utc_time_async,
    ),
    MethodDescriptor::new(
        &["ПолучитьНевидимость", "GetHidden"],
        Arity::exact(0),
        get_hidden,
    ),
    MethodDescriptor::new(
        &["УстановитьНевидимость", "SetHidden"],
        Arity::exact(1),
        set_hidden,
    )
    .as_procedure(),
    MethodDescriptor::new(
        &["ПолучитьТолькоЧтение", "GetReadOnly"],
        Arity::exact(0),
        get_read_only,
    ),
    MethodDescriptor::new(
        &["УстановитьТолькоЧтение", "SetReadOnly"],
        Arity::exact(1),
        set_read_only,
    )
    .as_procedure(),
    MethodDescriptor::new(
        &["ПолучитьВремяИзменения", "GetModificationTime"],
        Arity::exact(0),
        get_local_time,
    ),
    MethodDescriptor::new(
        &[
            "ПолучитьУниверсальноеВремяИзменения",
            "GetModificationUniversalTime",
        ],
        Arity::exact(0),
        get_utc_time,
    ),
    MethodDescriptor::new(
        &["УстановитьВремяИзменения", "SetModificationTime"],
        Arity::exact(1),
        set_local_time,
    )
    .as_procedure(),
    MethodDescriptor::new(
        &[
            "УстановитьУниверсальноеВремяИзменения",
            "SetModificationUniversalTime",
        ],
        Arity::exact(1),
        set_utc_time,
    )
    .as_procedure(),
    MethodDescriptor::new(&["Существует", "Exists"], Arity::exact(0), exists),
    MethodDescriptor::new(&["ЭтоФайл", "IsFile"], Arity::exact(0), is_file),
    MethodDescriptor::new(
        &["ЭтоКаталог", "IsDirectory"],
        Arity::exact(0),
        is_directory,
    ),
    MethodDescriptor::new(&["Размер", "Size"], Arity::exact(0), size),
];

static PROPERTIES: &[PropertyDescriptor] = &[
    PropertyDescriptor {
        names: &["ПолноеИмя", "FullName"],
        get: get_full_name,
        set: None,
    },
    PropertyDescriptor {
        names: &["Имя", "Name"],
        get: get_name,
        set: None,
    },
    PropertyDescriptor {
        names: &["Путь", "Path"],
        get: get_parent,
        set: None,
    },
    PropertyDescriptor {
        names: &["ИмяБезРасширения", "BaseName"],
        get: get_base_name,
        set: None,
    },
    PropertyDescriptor {
        names: &["Расширение", "Extension"],
        get: get_extension,
        set: None,
    },
];

pub(crate) const API_MEMBERS: &[ObjectMembersDescriptor] =
    &[ObjectMembersDescriptor::new(&FILE_TYPE)
        .with_methods(METHODS)
        .with_properties(PROPERTIES)];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sfile_path_uses_the_measured_lexical_properties() {
        let path = file_path("/tmp/star/a*.txt", "/");
        assert_eq!(path.full_name, "sfile://sfile:///tmp/star/a*.txt");
        assert_eq!(path.parent, "sfile:///tmp/star/");
        assert_eq!(path.name, "a");
        assert_eq!(path.base_name, "a");
        assert!(path.extension.is_empty());
    }

    #[test]
    fn exists_hides_only_the_measured_metadata_errors() {
        for kind in [
            std::io::ErrorKind::NotFound,
            std::io::ErrorKind::NotADirectory,
            std::io::ErrorKind::PermissionDenied,
        ] {
            assert!(existence_error_is_false(&std::io::Error::from(kind)));
        }
        #[cfg(target_os = "linux")]
        for errno in [36, 40] {
            assert!(existence_error_is_false(
                &std::io::Error::from_raw_os_error(errno)
            ));
        }
        assert!(!existence_error_is_false(&std::io::Error::other(
            "неизмеренная ошибка"
        )));
    }

    #[test]
    fn begin_refuses_legacy_and_reduced_hosts_before_path_mutation() {
        struct Legacy;
        impl crate::HostPromiseSpawner for Legacy {
            fn spawn_http(
                &mut self,
                _: std::sync::Arc<dyn crate::HttpClient>,
                _: crate::HttpWireRequest,
                _: crate::HttpResponseMapper,
                _: crate::HttpErrorMapper,
            ) -> RtResult<BslValue> {
                panic!("файловая операция не должна обращаться к HTTP")
            }
        }
        fn format(value: &BslValue, _: Option<&str>) -> RtResult<String> {
            let BslValue::Str(value) = value else {
                panic!("только путь")
            };
            Ok(value.to_string())
        }
        let files: Rc<dyn FileSystem> = Rc::new(crate::SystemFileSystem);
        let zone: Rc<dyn crate::TimeZone> = Rc::new(crate::FixedTimeZone::new(0).unwrap());
        let file = file_from_path("before".into(), files.clone(), zone.clone(), "/");
        let file = file.object_ref().unwrap();
        let mut shapes = crate::RuntimeShapes::seeded(Vec::new(), Vec::new(), None);
        let description = crate::notification::construct_notification(
            &mut CallContext::minimal(&mut shapes, format),
            &[],
        )
        .unwrap();
        let arguments = [description, string("after")];
        let error = file.call_method(
            "BeginInitialization",
            &arguments,
            &mut CallContext::minimal(&mut shapes, format),
        );
        assert!(matches!(error, Err(RtError::CapabilityMissing { .. })));
        let mut legacy = Legacy;
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let random = crate::HostEnv::default().random();
        let mut context = CallContext::interpreter(crate::InterpreterServices {
            runtime_shapes: &mut shapes,
            formatter: format,
            stdout: &mut stdout,
            stderr: &mut stderr,
            zone: &zone,
            files: &files,
            random: &random,
            network: None,
            host_promises: Some(&mut legacy),
            function_caller: None,
            background_jobs: None,
            temp_storage: None,
            message_sink: None,
        });
        assert!(matches!(
            file.call_method("НачатьИнициализацию", &arguments, &mut context),
            Err(RtError::IoError(_))
        ));
        assert_eq!(
            file.get_property("FullName", &mut context).unwrap(),
            string("before")
        );
        // Даже готовая ошибка аргумента не регистрируется через прежний host.
        assert!(matches!(
            file.call_method(
                "BeginSettingHidden",
                &[arguments[0].clone(), BslValue::new_array(vec![])],
                &mut context
            ),
            Err(RtError::IoError(_))
        ));
    }

    #[test]
    fn file_attribute_arguments_do_not_change_the_condition_aliases() {
        for (word, condition) in [("Да", true), ("Нет", false)] {
            let value = string(word);
            assert_eq!(value.as_condition().unwrap(), condition);
            assert!(matches!(
                attribute_argument(&value, "Файл"),
                Err(RtError::TypeError { .. })
            ));
            assert!(matches!(
                boolean_argument(&value, "Файл"),
                Err(RtError::TypeError { .. })
            ));
        }
        for (word, expected) in [
            ("Истина", true),
            ("Ложь", false),
            ("True", true),
            ("False", false),
            (" tRuE ", true),
            ("\u{001c}True\u{001f}", true),
            ("yes", true),
            ("NO", false),
        ] {
            assert_eq!(attribute_argument(&string(word), "Файл").unwrap(), expected);
        }
        for word in ["yes", "no"] {
            assert!(string(word).as_condition().is_err());
            assert!(boolean_argument(&string(word), "НайтиФайлы").is_err());
        }
        assert!(attribute_argument(&string("\u{200b}True\u{200b}"), "Файл").is_err());
    }

    #[test]
    fn initialization_without_a_promise_executor_does_not_mutate_the_path() {
        let value = file_from_path(
            "before.txt".into(),
            Rc::new(crate::SystemFileSystem),
            Rc::new(crate::FixedTimeZone::new(0).unwrap()),
            "/",
        );
        let object = value.object_ref().unwrap();
        let mut shapes = crate::RuntimeShapes::seeded(Vec::new(), Vec::new(), None);
        let mut ctx = CallContext::minimal(&mut shapes, |value, _| {
            Ok(value.as_str("test formatter")?.to_string())
        });
        assert!(matches!(
            initialize_async(object.as_dyn(), &[string("after.txt")], &mut ctx),
            Err(RtError::CapabilityMissing {
                capability: crate::Capability::HostPromises,
                ..
            })
        ));
        assert_eq!(
            get_full_name(object.as_dyn(), &mut ctx).unwrap(),
            string("before.txt")
        );
    }

    #[test]
    fn universal_file_time_keeps_the_measured_fraction_after_wrapping() {
        let zone = crate::FixedTimeZone::new(0).unwrap();
        // mtime: file-time-edges.host.txt; дробь: file-date-neighbors.platform.txt.
        let result = modification_time(
            FileMetadata::file(Some(1_844_674_407_370_954)),
            false,
            &zone,
        )
        .unwrap();
        let BslValue::Date(date) = result else {
            panic!("ожидалась дата")
        };
        let epoch = BslDate::from_civil(1970, 1, 1, 0, 0, 0).unwrap();
        assert_eq!(date.difference(epoch).to_canonical(), "-1.1616");
    }

    #[test]
    fn file_time_string_parser_keeps_the_measured_whitespace_nul_and_zero_rules() {
        let date = BslDate::from_civil(2020, 1, 15, 0, 0, 0).unwrap();
        for value in [
            "20200115",
            " 20200115 ",
            "\t20200115\t",
            "\n20200115\n",
            "\r20200115\r",
            "\u{85}20200115\u{85}",
            "\u{a0}20200115\u{a0}",
            "20200115\0ignored",
        ] {
            assert_eq!(parse_file_time_string(value), Some(date), "{value:?}");
        }
        for value in [
            "",
            " ",
            "\u{200b}20200115",
            "20200115\u{feff}",
            "\x0020200115",
            "2020\x000115",
        ] {
            assert_eq!(parse_file_time_string(value), None, "{value:?}");
        }
        assert_eq!(parse_file_time_string("00000000"), Some(BslDate::empty()));
        assert_eq!(
            parse_file_time_string("00000000000000"),
            Some(BslDate::empty())
        );
    }

    #[test]
    fn every_file_member_name_occurs_in_the_platform_probe_suite() {
        const PROBES: &str = concat!(
            include_str!("../../../tests/conformance/measure/filesystem/file-object.bsl"),
            include_str!("../../../tests/conformance/measure/filesystem/file-constructor.bsl"),
            include_str!("../../../tests/conformance/measure/filesystem/file-async-client.bsl"),
            include_str!("../../../tests/conformance/measure/filesystem/file-async-read.bsl"),
            include_str!("../../../tests/conformance/measure/filesystem/file-async-write.bsl"),
            include_str!("../../../tests/conformance/measure/filesystem/file-callback-methods.bsl"),
            include_str!(
                "../../../tests/conformance/measure/filesystem/file-mobile-methods-client.bsl"
            ),
        );
        for method in METHODS {
            for name in method.names() {
                assert!(PROBES.contains(name), "нет платформенной пробы {name}");
            }
        }
        for property in PROPERTIES {
            for name in property.names {
                assert!(PROBES.contains(name), "нет платформенной пробы {name}");
            }
        }
    }

    #[test]
    fn the_complete_file_method_inventory_keeps_names_arities_and_call_kinds() {
        let expected = [
            (
                "НачатьПроверкуСуществования",
                "BeginCheckingExistence",
                1,
                true,
            ),
            (
                "НачатьПроверкуЭтоКаталог",
                "BeginCheckingIsDirectory",
                1,
                true,
            ),
            ("НачатьПроверкуЭтоФайл", "BeginCheckingIsFile", 1, true),
            (
                "НачатьПолучениеКартинкиПредставленияФайлаБиблиотекиМобильногоУстройства",
                "BeginGetMobileDeviceLibraryFileThumbnail",
                1,
                true,
            ),
            ("НачатьПолучениеНевидимости", "BeginGettingHidden", 1, true),
            (
                "НачатьПолучениеВремениИзменения",
                "BeginGettingModificationTime",
                1,
                true,
            ),
            (
                "НачатьПолучениеУниверсальногоВремениИзменения",
                "BeginGettingModificationUniversalTime",
                1,
                true,
            ),
            (
                "НачатьПолучениеТолькоЧтения",
                "BeginGettingReadOnly",
                1,
                true,
            ),
            ("НачатьПолучениеРазмера", "BeginGettingSize", 1, true),
            ("НачатьИнициализацию", "BeginInitialization", 2, true),
            ("НачатьУстановкуНевидимости", "BeginSettingHidden", 2, true),
            (
                "НачатьУстановкуВремениИзменения",
                "BeginSettingModificationTime",
                2,
                true,
            ),
            (
                "НачатьУстановкуУниверсальногоВремениИзменения",
                "BeginSettingModificationUniversalTime",
                2,
                true,
            ),
            (
                "НачатьУстановкуТолькоЧтения",
                "BeginSettingReadOnly",
                2,
                true,
            ),
            ("Существует", "Exists", 0, false),
            ("СуществуетАсинх", "ExistsAsync", 0, false),
            ("ПолучитьНевидимость", "GetHidden", 0, false),
            ("ПолучитьНевидимостьАсинх", "GetHiddenAsync", 0, false),
            (
                "ПолучитьПредставлениеФайлаБиблиотекиМобильногоУстройства",
                "GetMobileDeviceLibraryFilePresentation",
                0,
                false,
            ),
            (
                "ПолучитьКартинкуПредставленияФайлаБиблиотекиМобильногоУстройства",
                "GetMobileDeviceLibraryFileThumbnail",
                0,
                false,
            ),
            (
                "ПолучитьКартинкуПредставленияФайлаБиблиотекиМобильногоУстройстваАсинх",
                "GetMobileDeviceLibraryFileThumbnailAsync",
                0,
                false,
            ),
            ("ПолучитьВремяИзменения", "GetModificationTime", 0, false),
            (
                "ПолучитьВремяИзмененияАсинх",
                "GetModificationTimeAsync",
                0,
                false,
            ),
            (
                "ПолучитьУниверсальноеВремяИзменения",
                "GetModificationUniversalTime",
                0,
                false,
            ),
            (
                "ПолучитьУниверсальноеВремяИзмененияАсинх",
                "GetModificationUniversalTimeAsync",
                0,
                false,
            ),
            ("ПолучитьТолькоЧтение", "GetReadOnly", 0, false),
            ("ПолучитьТолькоЧтениеАсинх", "GetReadOnlyAsync", 0, false),
            ("ИнициализироватьАсинх", "InitializeAsync", 1, false),
            ("ЭтоКаталог", "IsDirectory", 0, false),
            ("ЭтоКаталогАсинх", "IsDirectoryAsync", 0, false),
            ("ЭтоФайл", "IsFile", 0, false),
            ("ЭтоФайлАсинх", "IsFileAsync", 0, false),
            ("УстановитьНевидимость", "SetHidden", 1, true),
            ("УстановитьНевидимостьАсинх", "SetHiddenAsync", 1, false),
            ("УстановитьВремяИзменения", "SetModificationTime", 1, true),
            (
                "УстановитьВремяИзмененияАсинх",
                "SetModificationTimeAsync",
                1,
                false,
            ),
            (
                "УстановитьУниверсальноеВремяИзменения",
                "SetModificationUniversalTime",
                1,
                true,
            ),
            (
                "УстановитьУниверсальноеВремяИзмененияАсинх",
                "SetModificationUniversalTimeAsync",
                1,
                false,
            ),
            ("УстановитьТолькоЧтение", "SetReadOnly", 1, true),
            ("УстановитьТолькоЧтениеАсинх", "SetReadOnlyAsync", 1, false),
            ("Размер", "Size", 0, false),
            ("РазмерАсинх", "SizeAsync", 0, false),
        ];

        assert_eq!(METHODS.len(), expected.len());
        for (ru, en, arity, procedure) in expected {
            let descriptor = METHODS
                .iter()
                .find(|descriptor| descriptor.names() == [ru, en])
                .unwrap_or_else(|| panic!("нет метода {ru}/{en}"));
            assert!(descriptor.arity().accepts(arity));
            if arity > 0 {
                assert!(!descriptor.arity().accepts(arity - 1));
            }
            assert!(!descriptor.arity().accepts(arity + 1));
            assert_eq!(descriptor.check_result_use(true).is_err(), procedure);
        }
    }
}
