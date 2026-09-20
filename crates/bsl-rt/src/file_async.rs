//! Нейтральные запросы общих файловых операций для фонового исполнения.

use crate::{
    BslValue, BuiltinFn, FileOperationError, FileSearchPaths, FileSearchRequest, FileSystem,
    RtError, RtResult, TimeZone,
};
use std::rc::Rc;

/// Файловый запрос без непереносимых BSL-значений.
#[derive(Debug)]
pub enum FileOperationRequest {
    UpdateMetadata {
        path: String,
        update: crate::FileMetadataUpdate,
    },
    Metadata {
        path: String,
        query: crate::FileMetadataQuery,
    },
    Search(FileSearchRequest),
    CreateDirectory(String),
    Delete {
        path: String,
        mask: Option<String>,
    },
    TemporaryDirectory,
}

/// Нейтральный результат, преобразуемый в BSL только в потоке VM.
#[derive(Debug)]
pub enum FileOperationResult {
    Metadata {
        metadata: std::io::Result<crate::FileMetadata>,
        query: crate::FileMetadataQuery,
    },
    Search(FileSearchPaths),
    String(String),
    Undefined,
}

impl FileOperationResult {
    /// Восстанавливает BSL-результат с исходными сервисами сеанса.
    ///
    /// # Errors
    /// Ошибка чтения, отсутствующего атрибута или преобразования даты.
    pub fn into_value(
        self,
        files: Rc<dyn FileSystem>,
        zone: Rc<dyn TimeZone>,
    ) -> RtResult<BslValue> {
        match self {
            Self::Metadata { metadata, query } => query.resolve(metadata, zone.as_ref()),
            Self::Search(paths) => Ok(paths.into_value(files, zone)),
            Self::String(text) => Ok(BslValue::Str(crate::BslString::from_utf8_string(text))),
            Self::Undefined => Ok(BslValue::Undefined),
        }
    }
}

/// Подготавливает запрос; вызывающий сохраняет ошибку в созданном обещании.
///
/// # Errors
/// Ошибка вида вызова, арности либо преобразования аргументов.
pub fn prepare_file_operation(
    builtin: BuiltinFn,
    args: &[BslValue],
    formatter: crate::component::ValueFormatter,
) -> RtResult<FileOperationRequest> {
    match builtin {
        BuiltinFn::FindFilesAsync => {
            crate::prepare_file_search(args, formatter).map(FileOperationRequest::Search)
        }
        BuiltinFn::DeleteFilesAsync => {
            let (path, mask) = crate::file_search::prepare_delete_files(args, Some(formatter))?;
            Ok(FileOperationRequest::Delete { path, mask })
        }
        BuiltinFn::CreateDirectoryAsync => {
            let [path] = args else {
                return Err(RtError::InvalidBytecode(
                    "СоздатьКаталогАсинх: ожидался 1 аргумент",
                ));
            };
            // Тонкий клиент форматирует все значения через `Строка`; пустое
            // представление остаётся запросом и отказывает только при `Ждать`.
            Ok(FileOperationRequest::CreateDirectory(formatter(
                path, None,
            )?))
        }
        BuiltinFn::TempFilesDirAsync if args.is_empty() => {
            Ok(FileOperationRequest::TemporaryDirectory)
        }
        _ => Err(RtError::InvalidBytecode(
            "неверная асинхронная файловая операция",
        )),
    }
}

/// Вызывает синхронный `СоздатьКаталог` с форматированием аргумента
/// в контексте BSL-сеанса.
///
/// # Errors
/// Ошибка арности, форматирования, пути либо файловой системы host.
pub fn call_builtin_create_directory_formatted(
    args: &[BslValue],
    files: &dyn FileSystem,
    formatter: crate::component::ValueFormatter,
) -> RtResult<BslValue> {
    let [value] = args else {
        return Err(RtError::InvalidBytecode(
            "СоздатьКаталог: ожидался 1 аргумент",
        ));
    };
    let path = formatter(value, None)?;
    let formatted = BslValue::Str(crate::BslString::from_utf8_string(path.clone()));
    if crate::prepare_create_directory_noop(std::slice::from_ref(&formatted)).is_none() {
        create_directory_path(files, &path, &mut || false).map_err(RtError::from)?;
    }
    Ok(BslValue::Undefined)
}

pub(crate) fn create_directory_path(
    files: &dyn FileSystem,
    path: &str,
    canceled: &mut dyn FnMut() -> bool,
) -> Result<(), FileOperationError> {
    // file-create-paths{,-async} и file-create-directory-edges: NUL завершает
    // путь I/O, измеренные конечные пробельные символы удаляются, а точки и
    // оба разделителя сворачиваются до обращения к файловой системе.
    // Исходная строка запроса остаётся результатом асинхронного метода.
    let path = crate::prepare_file_operation_path(path, files)
        .map_err(|error| FileOperationError::Io(error.to_string()))?;
    if path.is_empty() || path.contains('*') {
        return Err(FileOperationError::Io(
            "недопустимый путь создания каталога".into(),
        ));
    }
    if canceled() {
        return Err(FileOperationError::Canceled);
    }
    files
        .create_dir_all(&path)
        .map_err(|error| FileOperationError::Io(format!("{path}: {error}")))?;
    Ok(())
}

pub(crate) fn temporary_directory_path<E: From<FileOperationError>>(
    files: &dyn FileSystem,
    checkpoint: &mut dyn FnMut() -> Result<(), E>,
) -> Result<String, E> {
    let io_error = |error: std::io::Error| E::from(FileOperationError::Io(error.to_string()));
    checkpoint()?;
    let mut directory = files.temporary_directory().map_err(io_error)?;
    checkpoint()?;
    let separator = files.path_separator().map_err(io_error)?;
    if !directory.ends_with(&separator) {
        directory.push_str(&separator);
    }
    Ok(directory)
}

/// Исполняет операцию через выбранный host, без второго алгоритма обхода.
///
/// # Errors
/// Ошибка host, защитного предела либо отмены. Сделанные изменения не откатываются.
/// Отмена проверяется также после ошибочного результата и имеет приоритет над ним.
pub fn perform_file_operation(
    request: FileOperationRequest,
    files: &dyn FileSystem,
    canceled: &mut dyn FnMut() -> bool,
) -> Result<FileOperationResult, FileOperationError> {
    if canceled() {
        return Err(FileOperationError::Canceled);
    }
    let result = match request {
        FileOperationRequest::UpdateMetadata { path, update } => update
            .apply(files, &path)
            .map(|()| FileOperationResult::Undefined)
            .map_err(|error| FileOperationError::Io(error.to_string())),
        FileOperationRequest::Metadata { path, query } => Ok(FileOperationResult::Metadata {
            metadata: files.metadata(&path),
            query,
        }),
        FileOperationRequest::Search(request) => {
            crate::search_file_paths(&request, files, canceled).map(FileOperationResult::Search)
        }
        FileOperationRequest::CreateDirectory(path) => {
            create_directory_path(files, &path, canceled)
                .map(|()| FileOperationResult::String(path))
        }
        FileOperationRequest::TemporaryDirectory => temporary_directory_path(files, &mut || {
            if canceled() {
                Err(FileOperationError::Canceled)
            } else {
                Ok(())
            }
        })
        .map(FileOperationResult::String),
        FileOperationRequest::Delete { path, mask } => {
            crate::file_search::delete_paths(&path, mask.as_deref(), files, &mut || {
                if canceled() {
                    Err(FileOperationError::Canceled)
                } else {
                    Ok(())
                }
            })
            .map(|()| FileOperationResult::Undefined)
        }
    };
    // Ошибка host не обходит финальную проверку: отмена могла прийти
    // во время операции, в том числе после частичного изменения ФС.
    if canceled() {
        return Err(FileOperationError::Canceled);
    }
    result
}
