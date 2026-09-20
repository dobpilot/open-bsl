//! Поиск файлов через сервисы создавшего сеанс host.

use std::{io, rc::Rc};

fn is_measured_filesystem_loop(error: &io::Error) -> bool {
    // `ErrorKind::FilesystemLoop` пока нестабилен. Число ELOOP относится
    // только к измеренному Linux-host и не расширяет правило на другие ОС.
    cfg!(target_os = "linux") && error.raw_os_error() == Some(40)
}

use crate::{BslValue, FileSystem, RtError, RtResult, TimeZone, file, file_mask_matches};

/// Подготовленный запрос без BSL-значений; переносится в поток обхода.
#[derive(Debug, Clone)]
pub struct FileSearchRequest {
    pub path: String,
    pub mask: Option<String>,
    pub recursive: bool,
}

/// Нейтральный результат обхода; файловые объекты создаются только в VM.
#[derive(Debug)]
pub struct FileSearchPaths {
    paths: Vec<String>,
    separator: String,
}

impl FileSearchPaths {
    /// Привязывает найденные пути к исходным сервисам сеанса.
    pub fn into_value(self, files: Rc<dyn FileSystem>, zone: Rc<dyn TimeZone>) -> BslValue {
        BslValue::new_array(
            self.paths
                .into_iter()
                .map(|path| {
                    file::file_from_path(path, files.clone(), zone.clone(), &self.separator)
                })
                .collect(),
        )
    }
}

/// Переносимая ошибка файловой операции без Rc и BSL-исключения.
#[derive(Debug)]
pub enum FileOperationError {
    Io(String),
    ResourceLimit(String),
    Canceled,
    /// Поток обхода перехватил панику host; её payload не переносится в BSL.
    HostPanic,
}

/// Совместимое имя ошибки ранее выделенного файлового поиска.
pub type FileSearchError = FileOperationError;

impl From<FileOperationError> for RtError {
    fn from(error: FileOperationError) -> Self {
        match error {
            FileSearchError::Io(message) => Self::IoError(message),
            FileSearchError::ResourceLimit(message) => Self::ResourceLimit(message),
            FileSearchError::Canceled => Self::Canceled,
            FileSearchError::HostPanic => {
                Self::DynamicError("паника host при фоновой файловой операции".into())
            }
        }
    }
}

/// Выполняет общий обход без создания BSL-объектов.
///
/// # Errors
/// Ошибка host, защитного предела либо кооперативной отмены.
pub fn search_file_paths(
    request: &FileSearchRequest,
    files: &dyn FileSystem,
    canceled: &mut dyn FnMut() -> bool,
) -> Result<FileSearchPaths, FileSearchError> {
    let mut checkpoint = || {
        if canceled() {
            Err(FileSearchError::Canceled)
        } else {
            Ok(())
        }
    };
    checkpoint()?;
    let separator = files
        .path_separator()
        .map_err(|error| FileSearchError::Io(error.to_string()))?;
    if separator.is_empty() {
        return Err(FileSearchError::Io(
            "host вернул пустой разделитель пути".into(),
        ));
    }
    let paths = find_paths(
        file::trim_path_end(&request.path),
        request.mask.as_deref(),
        request.recursive,
        FindPathPolicy {
            root_permission_denied_is_empty: true,
            sfile_results: true,
        },
        files,
        &separator,
        &mut checkpoint,
    )?;
    Ok(FileSearchPaths { paths, separator })
}

/// Удаляет путь либо выбранных маской непосредственных детей через host.
///
/// # Errors
/// Возвращает ошибку аргументов, отмены, выбора путей либо удаления.
/// При ошибке удаления уже удалённые элементы не восстанавливаются.
pub fn call_builtin_delete_files(
    args: &[BslValue],
    files: &dyn FileSystem,
    checkpoint: &mut dyn FnMut() -> RtResult<()>,
) -> RtResult<BslValue> {
    let (path, mask) = prepare_delete_files(args, None)?;
    delete_paths(&path, mask.as_deref(), files, checkpoint)?;
    Ok(BslValue::Undefined)
}

/// Удаляет файлы с преобразованием маски форматировщиком BSL-сеанса.
///
/// Прежний вход без форматировщика сохраняет строгие строковые аргументы.
///
/// # Errors
/// Ошибка аргументов, форматирования, отмены, выбора путей либо удаления.
pub fn call_builtin_delete_files_formatted(
    args: &[BslValue],
    files: &dyn FileSystem,
    formatter: crate::component::ValueFormatter,
    checkpoint: &mut dyn FnMut() -> RtResult<()>,
) -> RtResult<BslValue> {
    let (path, mask) = prepare_delete_files(args, Some(formatter))?;
    delete_paths(&path, mask.as_deref(), files, checkpoint)?;
    Ok(BslValue::Undefined)
}

pub(crate) fn prepare_delete_files(
    args: &[BslValue],
    formatter: Option<crate::component::ValueFormatter>,
) -> RtResult<(String, Option<String>)> {
    if !(1..=2).contains(&args.len()) {
        return Err(RtError::InvalidBytecode(
            "УдалитьФайлы: ожидалось от 1 до 2 аргументов",
        ));
    }
    let path = args[0].as_str("УдалитьФайлы")?.to_string();
    // Любое значение маски проходит общий форматировщик `Строка`; точное
    // представление типа не дублируется в файловой подсистеме.
    let mask = match (args.get(1), formatter) {
        (None | Some(BslValue::Undefined), _) => None,
        (Some(value), Some(formatter)) => Some(formatter(value, None)?),
        (Some(value), None) => Some(value.as_str("УдалитьФайлы")?.to_string()),
    };
    Ok((path, mask))
}

pub(crate) fn delete_paths<E: From<FileSearchError>>(
    path: &str,
    mask: Option<&str>,
    files: &dyn FileSystem,
    checkpoint: &mut dyn FnMut() -> Result<(), E>,
) -> Result<(), E> {
    let io_error = |error: io::Error| E::from(FileSearchError::Io(error.to_string()));
    let targets = if let Some(mask) = mask.filter(|mask| !mask.is_empty()) {
        checkpoint()?;
        let separator = files.path_separator().map_err(io_error)?;
        if separator.is_empty() {
            return Err(FileSearchError::Io("host вернул пустой разделитель пути".into()).into());
        }
        find_paths(
            path,
            Some(mask),
            false,
            FindPathPolicy {
                root_permission_denied_is_empty: false,
                sfile_results: false,
            },
            files,
            &separator,
            checkpoint,
        )?
    } else {
        vec![path.to_owned()]
    };
    for target in targets {
        checkpoint()?;
        files
            .remove_path(&target)
            .map_err(|error| E::from(FileSearchError::Io(format!("{target}: {error}"))))?;
    }
    Ok(())
}

/// Вызывает `НайтиФайлы` с BSL-аргументами и сервисами сеанса.
///
/// # Errors
/// Возвращает ошибку арности, преобразования аргументов либо [`find_files`].
pub fn call_builtin_find_files(
    args: &[BslValue],
    files: Rc<dyn FileSystem>,
    zone: Rc<dyn TimeZone>,
    formatter: crate::component::ValueFormatter,
    checkpoint: &mut dyn FnMut() -> RtResult<()>,
) -> RtResult<BslValue> {
    let request = prepare_file_search(args, formatter)?;
    find_files(
        &request.path,
        request.mask.as_deref(),
        request.recursive,
        files,
        zone,
        checkpoint,
    )
}

/// Преобразует BSL-аргументы перед переносом запроса в другой поток.
///
/// # Errors
/// Ошибка арности либо преобразования аргументов. Асинхронный вызывающий
/// сохраняет её в обещании, а не выбрасывает до возврата обещания.
pub fn prepare_file_search(
    args: &[BslValue],
    formatter: crate::component::ValueFormatter,
) -> RtResult<FileSearchRequest> {
    if !(1..=3).contains(&args.len()) {
        return Err(RtError::InvalidBytecode(
            "НайтиФайлы: ожидалось от 1 до 3 аргументов",
        ));
    }
    // Путь и маска используют общий форматировщик `Строка`; флаг рекурсии
    // проходит измеренный файловый вариант условного преобразования до I/O.
    let path = formatter(&args[0], None)?;
    let mask = match args.get(1) {
        None | Some(BslValue::Undefined) => None,
        Some(value) => Some(formatter(value, None)?),
    };
    let recursive = match args.get(2) {
        None => false,
        Some(value) => file::boolean_argument(value, "НайтиФайлы")?,
    };
    Ok(FileSearchRequest {
        path,
        mask,
        recursive,
    })
}

/// Возвращает массив файловых объектов, сохраняющих переданные ФС и часовой пояс.
///
/// Без непустой маски проверяет сам путь; с маской обходит каталог в порядке host.
/// Проверка отмены вызывается перед обращениями к host и между элементами.
/// Защитные пределы open-bsl: 256 уровней каталогов и 1 000 000 элементов.
/// Это ограничения обхода, а не утверждение о пределах платформы 1С.
///
/// # Errors
/// Возвращает ошибку проверки отмены, доступа к ФС, некорректного имени элемента
/// или превышения защитного предела. Частичный массив при ошибке не возвращается.
pub fn find_files(
    path: &str,
    mask: Option<&str>,
    recursive: bool,
    files: Rc<dyn FileSystem>,
    zone: Rc<dyn TimeZone>,
    checkpoint: &mut dyn FnMut() -> RtResult<()>,
) -> RtResult<BslValue> {
    let io_error = |error: io::Error| RtError::IoError(error.to_string());
    checkpoint()?;
    let separator = files.path_separator().map_err(io_error)?;
    if separator.is_empty() {
        return Err(RtError::IoError(
            "host вернул пустой разделитель пути".into(),
        ));
    }
    let paths = find_paths(
        file::trim_path_end(path),
        mask,
        recursive,
        FindPathPolicy {
            root_permission_denied_is_empty: true,
            sfile_results: true,
        },
        files.as_ref(),
        &separator,
        checkpoint,
    )?;
    Ok(BslValue::new_array(
        paths
            .into_iter()
            .map(|path| file::file_from_path(path, files.clone(), zone.clone(), &separator))
            .collect(),
    ))
}

fn measured_sfile_mask_matches(mask: &str, name: &str) -> bool {
    let bytes = mask.as_bytes();
    if bytes.iter().all(|byte| *byte == b'?') {
        return bytes.len() >= 3 && bytes.len() == name.len();
    }
    if bytes.iter().all(|byte| matches!(byte, b'*' | b'?')) {
        return false;
    }
    file_mask_matches(mask, name)
}

#[derive(Clone, Copy)]
struct FindPathPolicy {
    root_permission_denied_is_empty: bool,
    sfile_results: bool,
}

// Общий выбор путей для поиска объектов и удаления по маске. Не создаёт
// объект Файл: удалению нужны исходные пути host без повторной нормализации.
fn find_paths<E: From<FileSearchError>>(
    path: &str,
    mask: Option<&str>,
    recursive: bool,
    policy: FindPathPolicy,
    files: &dyn FileSystem,
    separator: &str,
    checkpoint: &mut dyn FnMut() -> Result<(), E>,
) -> Result<Vec<String>, E> {
    let io_error = |error: io::Error| E::from(FileSearchError::Io(error.to_string()));
    let Some(mask) = mask.filter(|mask| !mask.is_empty()) else {
        if policy.sfile_results
            && path
                .rsplit(separator)
                .next()
                .is_some_and(|name| name.contains('*'))
        {
            return Ok(Vec::new());
        }
        checkpoint()?;
        let metadata = files.metadata(path);
        checkpoint()?;
        return match metadata {
            Ok(_) => Ok(vec![path.to_owned()]),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Vec::new()),
            Err(error) => Err(io_error(error)),
        };
    };

    // `НЕ ИЗМЕРЕНО(FIND.TRAVERSAL)`: прочие ошибки обхода и иные sfile-формы
    // требуют отдельных проб. Измерены порядок host без сортировки, текущий
    // уровень перед спуском, каталог `a*`, его три ссылочных вида и обычные
    // ссылки.
    let mut stack = vec![(path.to_owned(), 0usize, false)];
    let mut result = Vec::new();
    let mut examined = 0usize;
    while let Some((directory, depth, trimmed_directory)) = stack.pop() {
        checkpoint()?;
        let entries = files.read_dir(&directory);
        checkpoint()?;
        let mut entries = match entries {
            Ok(entries) => entries,
            Err(error)
                if depth == 0
                    && matches!(
                        error.kind(),
                        io::ErrorKind::NotFound | io::ErrorKind::NotADirectory
                    ) =>
            {
                return Ok(Vec::new());
            }
            Err(error)
                if depth == 0
                    && policy.root_permission_denied_is_empty
                    && error.kind() == io::ErrorKind::PermissionDenied =>
            {
                // file-search-access-order: FindFiles скрывает отказ чтения
                // корня как пустой результат. Удаление использует тот же
                // обход с выключенной политикой и сохраняет ошибку host.
                return Ok(Vec::new());
            }
            Err(error)
                if trimmed_directory
                    && matches!(
                        error.kind(),
                        io::ErrorKind::NotFound | io::ErrorKind::NotADirectory
                    ) =>
            {
                // file-search-spaces: исчезновение только нормализованного
                // адреса не теряет уже выданные записи и соседние каталоги.
                continue;
            }
            Err(error) if depth > 0 && error.kind() == io::ErrorKind::PermissionDenied => {
                // file-search-metadata-errors: серверная 1С сохраняет
                // текущий уровень и пропускает недоступный дочерний каталог.
                continue;
            }
            Err(error) => return Err(io_error(error)),
        };
        let mut children = Vec::new();
        let level_start = result.len();
        let mut matches_stopped = false;
        let mut children_stopped = !recursive;
        loop {
            checkpoint()?;
            let next = entries.next();
            checkpoint()?;
            let Some(entry) = next else { break };
            let entry = entry.map_err(io_error)?;
            examined += 1;
            if examined > 1_000_000 {
                return Err(FileSearchError::ResourceLimit(
                    "превышен предел элементов поиска файлов".into(),
                )
                .into());
            }
            let name = entry.name();
            if name.is_empty()
                || matches!(name, "." | "..")
                || name.contains(['/', '\\'])
                || name.contains(separator)
            {
                return Err(FileSearchError::Io("host вернул имя элемента с путём".into()).into());
            }
            let child = if directory.is_empty() || directory.ends_with(separator) {
                format!("{directory}{name}")
            } else {
                format!("{directory}{separator}{name}")
            };
            let is_sfile = policy.sfile_results && name.contains('*');
            let matched = if is_sfile {
                measured_sfile_mask_matches(mask, name)
            } else {
                file_mask_matches(mask, name)
            };
            let mut broken_link = false;
            let is_dir =
                if entry.is_symlink() && ((!matches_stopped && matched) || !children_stopped) {
                    checkpoint()?;
                    let metadata = files.metadata(&child);
                    checkpoint()?;
                    match metadata {
                        Ok(metadata) => metadata.is_dir(),
                        Err(error)
                            if error.kind() == io::ErrorKind::NotFound
                                || is_measured_filesystem_loop(&error) =>
                        {
                            broken_link = true;
                            false
                        }
                        Err(error) => return Err(io_error(error)),
                    }
                } else {
                    entry.is_dir()
                };
            if broken_link {
                children_stopped = true;
            }
            if matched && !matches_stopped {
                // file-search-broken-{masks,middle}: первая найденная запись
                // отличается от последующих; полный набор допускающих её
                // масок остаётся частью FIND.TRAVERSAL.
                if broken_link && (matches!(mask, "*" | "*.*") || result.len() > level_start) {
                    matches_stopped = true;
                } else {
                    result.push(if is_sfile {
                        name.to_owned()
                    } else {
                        child.clone()
                    });
                }
            }
            if !children_stopped && is_dir && !is_sfile {
                if depth >= 255 {
                    return Err(FileSearchError::ResourceLimit(
                        "превышена глубина поиска файлов".into(),
                    )
                    .into());
                }
                // Кандидат выбран по исходному виду записи. Файл с пробелом
                // не превращается в каталог-двойник; два настоящих каталога
                // могут дать два обхода одного нормализованного адреса.
                let directory = file::trim_path_end(&child);
                children.push((directory.to_owned(), directory.len() != child.len()));
            }
            if matches_stopped && children_stopped {
                break;
            }
        }
        // Обратная укладка сохраняет порядок host при извлечении из стека.
        stack.extend(
            children
                .into_iter()
                .rev()
                .map(|(child, trimmed)| (child, depth + 1, trimmed)),
        );
    }
    Ok(result)
}
