//! Совместный доступ к открытым временным носителям системного host на Linux.
//!
//! Это учёт дескрипторов внутри процесса, не право удаления и не блокировка
//! сторонних процессов. Исходный поток и metadata остаются доступны.

use crate::FileHandle;
use std::collections::HashSet;
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex, MutexGuard};

type Identity = (u64, u64);

static OPEN_TEMPORARY_FILES: LazyLock<Mutex<HashSet<Identity>>> =
    LazyLock::new(|| Mutex::new(HashSet::new()));

fn open_files() -> MutexGuard<'static, HashSet<Identity>> {
    OPEN_TEMPORARY_FILES
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn identity(file: &File) -> io::Result<Identity> {
    let metadata = file.metadata()?;
    Ok((metadata.dev(), metadata.ino()))
}

fn path_identity(path: &Path) -> io::Result<Identity> {
    let metadata = std::fs::symlink_metadata(path)?;
    Ok((metadata.dev(), metadata.ino()))
}

/// Право удалить только запись каталога, которая всё ещё указывает
/// на созданный inode. Проверка и unlink не атомарны относительно процесса
/// с тем же UID; эта граница доверия явно зафиксирована в дизайне.
#[derive(Debug)]
pub(crate) struct Resource {
    path: PathBuf,
    path_text: String,
    identity: Identity,
}

impl crate::TemporaryFileResource for Resource {
    fn path(&self) -> &str {
        &self.path_text
    }

    fn remove(self: Box<Self>) -> io::Result<()> {
        if path_identity(&self.path)? != self.identity {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "временный путь больше не указывает на принадлежащий ресурс",
            ));
        }
        std::fs::remove_file(&self.path)
    }
}

pub(crate) fn create(path: PathBuf, path_text: String) -> io::Result<(TemporaryFile, Resource)> {
    use std::os::unix::fs::OpenOptionsExt;

    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&path)?;
    let resource = Resource {
        path,
        path_text,
        identity: identity(&file)?,
    };
    Ok((TemporaryFile::new(file)?, resource))
}

/// Проверяет тот же дескриптор, из которого будут прочитаны данные.
pub(crate) fn read(path: &str) -> io::Result<Vec<u8>> {
    let mut file = File::open(path)?;
    let identity = identity(&file)?;
    if open_files().contains(&identity) {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "нарушение совместного доступа к временному файлу",
        ));
    }
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;
    Ok(bytes)
}

#[derive(Debug)]
pub(crate) struct TemporaryFile {
    file: File,
    identity: Identity,
}

impl TemporaryFile {
    pub(crate) fn new(file: File) -> io::Result<Self> {
        let identity = identity(&file)?;
        // create_new уже создал новый inode; живой открытый носитель не
        // переиспользуется ОС, поэтому счётчик повторных регистраций не нужен.
        open_files().insert(identity);
        Ok(Self { file, identity })
    }
}

impl Drop for TemporaryFile {
    fn drop(&mut self) {
        open_files().remove(&self.identity);
    }
}

impl Read for TemporaryFile {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        self.file.read(bytes)
    }
}

impl Write for TemporaryFile {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.file.write(bytes)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()
    }
}

impl Seek for TemporaryFile {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        self.file.seek(position)
    }
}

impl FileHandle for TemporaryFile {
    fn len(&self) -> io::Result<u64> {
        FileHandle::len(&self.file)
    }

    fn set_len(&mut self, length: u64) -> io::Result<()> {
        self.file.set_len(length)
    }

    fn close(&mut self) -> io::Result<()> {
        // Как у прежнего std::fs::File: flush не освобождает дескриптор.
        // После успешного close компонент освобождает носитель вместе с учётом.
        FileHandle::close(&mut self.file)
    }
}
