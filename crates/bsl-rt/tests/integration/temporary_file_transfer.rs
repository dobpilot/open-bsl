//! Перенос носителя worker без переноса локального реестра и без I/O при передаче.

use std::io::{self, Cursor, Read, Seek, SeekFrom, Write};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use bsl_rt::{
    FileHandle, FileSystem, TemporaryFileRegistry, TemporaryFileResource, TransferableTemporaryFile,
};

#[derive(Debug, Default)]
struct Counts {
    io: AtomicUsize,
    closed: AtomicUsize,
    released: AtomicUsize,
    removed: AtomicUsize,
}

#[derive(Debug)]
struct Handle(Cursor<Vec<u8>>, Arc<Counts>);

impl Read for Handle {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        self.1.io.fetch_add(1, Ordering::SeqCst);
        self.0.read(bytes)
    }
}
impl Write for Handle {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.1.io.fetch_add(1, Ordering::SeqCst);
        self.0.write(bytes)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.1.io.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}
impl Seek for Handle {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        self.1.io.fetch_add(1, Ordering::SeqCst);
        self.0.seek(position)
    }
}
impl FileHandle for Handle {
    fn len(&self) -> io::Result<u64> {
        self.1.io.fetch_add(1, Ordering::SeqCst);
        Ok(self.0.get_ref().len() as u64)
    }
    fn close(&mut self) -> io::Result<()> {
        self.flush()
    }
}
impl Drop for Handle {
    fn drop(&mut self) {
        self.1.closed.fetch_add(1, Ordering::SeqCst);
    }
}

#[derive(Debug)]
struct Resource(Arc<Counts>);

impl TemporaryFileResource for Resource {
    fn path(&self) -> &str {
        "/virtual/worker-created"
    }
    fn remove(self: Box<Self>) -> io::Result<()> {
        self.0.removed.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}
impl Drop for Resource {
    fn drop(&mut self) {
        self.0.released.fetch_add(1, Ordering::SeqCst);
    }
}

#[derive(Debug, Clone)]
struct TransferHost(Arc<Counts>);

impl FileSystem for TransferHost {
    fn read(&self, _: &str) -> io::Result<Vec<u8>> {
        panic!("передача не читает файл по пути")
    }
    fn write(&self, _: &str, _: &[u8]) -> io::Result<()> {
        panic!("передача не пишет файл по пути")
    }
    fn metadata(&self, _: &str) -> io::Result<bsl_rt::FileMetadata> {
        panic!("передача не запрашивает метаданные по пути")
    }
    fn read_dir<'fs>(
        &'fs self,
        _: &str,
    ) -> io::Result<Box<dyn Iterator<Item = io::Result<bsl_rt::DirEntry>> + 'fs>> {
        panic!("передача не обходит каталоги")
    }
    fn create_dir_all(&self, _: &str) -> io::Result<()> {
        panic!("передача не создаёт каталог")
    }
    fn open(&self, _: &str, _: bsl_rt::FileOpenOptions) -> io::Result<Box<dyn FileHandle>> {
        panic!("передача не открывает носитель повторно")
    }
    fn create_temporary_file(&self, _: &[u8; 16]) -> io::Result<bsl_rt::OpenedTemporaryFile> {
        panic!("переносимая операция не подменяется прежним локальным созданием")
    }
    fn background_access(&self) -> Option<Arc<dyn FileSystem + Send + Sync>> {
        Some(Arc::new(self.clone()))
    }
    fn create_transferable_temporary_file(
        &self,
        entropy: &[u8; 16],
    ) -> io::Result<TransferableTemporaryFile> {
        assert_eq!(*entropy, [71; 16]);
        let mut bytes = Cursor::new(b"abcdef".to_vec());
        bytes.set_position(2);
        Ok(TransferableTemporaryFile::new(
            Box::new(Handle(bytes, self.0.clone())),
            Box::new(Resource(self.0.clone())),
        ))
    }
}

fn created_in_worker(counts: &Arc<Counts>) -> TransferableTemporaryFile {
    let local: std::rc::Rc<dyn FileSystem> = std::rc::Rc::new(TransferHost(counts.clone()));
    let background = local.background_access().unwrap();
    let owner = std::thread::current().id();
    std::thread::spawn(move || {
        assert_ne!(std::thread::current().id(), owner);
        background
            .create_transferable_temporary_file(&[71; 16])
            .unwrap()
    })
    .join()
    .unwrap()
}

fn assert_untouched(counts: &Counts) {
    assert_eq!(counts.io.load(Ordering::SeqCst), 0);
    assert_eq!(counts.closed.load(Ordering::SeqCst), 0);
    assert_eq!(counts.released.load(Ordering::SeqCst), 0);
    assert_eq!(counts.removed.load(Ordering::SeqCst), 0);
}

fn check_data(mut handle: Box<dyn FileHandle>) {
    assert_eq!(handle.len().unwrap(), 6);
    assert_eq!(handle.stream_position().unwrap(), 2);
    handle.write_all(b"XY").unwrap();
    handle.seek(SeekFrom::Start(0)).unwrap();
    let mut bytes = Vec::new();
    handle.read_to_end(&mut bytes).unwrap();
    assert_eq!(bytes, b"abXYef");
}

#[test]
fn transferred_result_keeps_the_original_handle_and_cleanup_resource() {
    let counts = Arc::new(Counts::default());
    let registry = TemporaryFileRegistry::default();
    let local = created_in_worker(&counts).into_local();
    let (path, handle) = local.into_registered_parts(&registry).unwrap();
    assert_eq!(path, "/virtual/worker-created");
    assert_untouched(&counts);
    check_data(handle);
    assert_eq!(counts.closed.load(Ordering::SeqCst), 1);
    let mut resources = registry.take_pending();
    assert_eq!(resources.len(), 1);
    assert_eq!(resources[0].path(), path);
    resources.pop().unwrap().remove().unwrap();
    assert_eq!(counts.removed.load(Ordering::SeqCst), 1);
    assert_eq!(counts.released.load(Ordering::SeqCst), 1);
    assert!(registry.take_pending().is_empty());
}

#[test]
fn closed_registry_returns_transferred_ownership_without_io_or_deletion() {
    let counts = Arc::new(Counts::default());
    let closed = TemporaryFileRegistry::default();
    assert!(closed.close().is_empty());
    let local = created_in_worker(&counts).into_local();
    let local = local.into_registered_parts(&closed).unwrap_err();
    assert_untouched(&counts);
    let other = TemporaryFileRegistry::default();
    let (_, handle) = local.into_registered_parts(&other).unwrap();
    assert_untouched(&counts);
    check_data(handle);
    drop(other.take_pending());
    assert_eq!(counts.removed.load(Ordering::SeqCst), 0);
    assert_eq!(counts.released.load(Ordering::SeqCst), 1);
}

#[test]
fn dropping_either_form_closes_resources_without_deleting_the_file() {
    for local in [false, true] {
        let counts = Arc::new(Counts::default());
        let transferred = created_in_worker(&counts);
        if local {
            drop(transferred.into_local());
        } else {
            drop(transferred);
        }
        assert_eq!(counts.io.load(Ordering::SeqCst), 0);
        assert_eq!(counts.closed.load(Ordering::SeqCst), 1);
        assert_eq!(counts.released.load(Ordering::SeqCst), 1);
        assert_eq!(counts.removed.load(Ordering::SeqCst), 0);
    }
}
