//! Передача временного носителя и права очистки без повторного открытия пути.

use std::cell::Cell;
use std::io::{self, Cursor, Read, Seek, SeekFrom, Write};
use std::rc::Rc;

use bsl_rt::{FileHandle, OpenedTemporaryFile, TemporaryFileRegistry, TemporaryFileResource};

#[derive(Debug, Default)]
struct Counts {
    removed: Cell<usize>,
    released: Cell<usize>,
    closed: Cell<usize>,
}

#[derive(Debug)]
struct Handle(Cursor<Vec<u8>>, Rc<Counts>);

impl Read for Handle {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        self.0.read(bytes)
    }
}
impl Write for Handle {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.write(bytes)
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
impl Seek for Handle {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        self.0.seek(position)
    }
}
impl FileHandle for Handle {
    fn len(&self) -> io::Result<u64> {
        Ok(self.0.get_ref().len() as u64)
    }
    fn close(&mut self) -> io::Result<()> {
        Ok(())
    }
}
impl Drop for Handle {
    fn drop(&mut self) {
        self.1.closed.set(self.1.closed.get() + 1);
    }
}

#[derive(Debug)]
struct Resource(Rc<Counts>);

impl TemporaryFileResource for Resource {
    fn path(&self) -> &str {
        "/virtual/created"
    }
    fn remove(self: Box<Self>) -> io::Result<()> {
        self.0.removed.set(self.0.removed.get() + 1);
        Ok(())
    }
}
impl Drop for Resource {
    fn drop(&mut self) {
        self.0.released.set(self.0.released.get() + 1);
    }
}

fn handle(counts: &Rc<Counts>) -> Box<dyn FileHandle> {
    let mut bytes = Cursor::new(b"abcdef".to_vec());
    bytes.set_position(2);
    Box::new(Handle(bytes, counts.clone()))
}

fn owned(counts: &Rc<Counts>) -> OpenedTemporaryFile {
    OpenedTemporaryFile::new_owned(handle(counts), Box::new(Resource(counts.clone())))
}

fn check_handle(mut handle: Box<dyn FileHandle>) {
    assert_eq!(handle.stream_position().unwrap(), 2);
    let mut bytes = Vec::new();
    handle.read_to_end(&mut bytes).unwrap();
    assert_eq!(bytes, b"cdef");
}

#[test]
fn registering_transfers_the_original_resource_without_touching_the_handle() {
    let counts = Rc::new(Counts::default());
    let registry = TemporaryFileRegistry::default();
    let opened = owned(&counts);
    assert!(registry.take_pending().is_empty());
    let (path, handle) = opened.into_registered_parts(&registry).unwrap();
    assert_eq!(path, "/virtual/created");
    assert_eq!(counts.closed.get(), 0);
    assert_eq!(counts.removed.get(), 0);
    assert_eq!(counts.released.get(), 0);
    check_handle(handle);
    assert_eq!(counts.closed.get(), 1);
    let resources = registry.take_pending();
    assert_eq!(resources.len(), 1);
    assert_eq!(resources[0].path(), path);
    for resource in resources {
        resource.remove().unwrap();
    }
    assert_eq!(counts.removed.get(), 1);
    assert_eq!(counts.released.get(), 1);
    assert!(registry.take_pending().is_empty());
}

#[test]
fn closed_registry_returns_the_whole_record_for_another_session() {
    let counts = Rc::new(Counts::default());
    let closed = TemporaryFileRegistry::default();
    assert!(closed.close().is_empty());
    let opened = owned(&counts).into_registered_parts(&closed).unwrap_err();
    assert_eq!(counts.closed.get(), 0);
    assert_eq!(counts.released.get(), 0);
    assert_eq!(counts.removed.get(), 0);
    assert!(closed.take_pending().is_empty());
    let other = TemporaryFileRegistry::default();
    let (_, handle) = opened.into_registered_parts(&other).unwrap();
    check_handle(handle);
    assert_eq!(other.take_pending().len(), 1);
    assert_eq!(counts.released.get(), 1);
    assert_eq!(counts.removed.get(), 0);
}

#[test]
fn a_path_alone_cannot_be_registered_as_deletion_authority() {
    let counts = Rc::new(Counts::default());
    let registry = TemporaryFileRegistry::default();
    let opened = OpenedTemporaryFile::new("/virtual/unowned".into(), handle(&counts));
    let opened = opened.into_registered_parts(&registry).unwrap_err();
    assert!(registry.take_pending().is_empty());
    assert_eq!(counts.closed.get(), 0);
    let (path, handle) = opened.into_parts();
    assert_eq!(path, "/virtual/unowned");
    check_handle(handle);
}

#[test]
fn legacy_split_releases_cleanup_capability_without_deleting_the_file() {
    let counts = Rc::new(Counts::default());
    let (path, handle) = owned(&counts).into_parts();
    assert_eq!(path, "/virtual/created");
    assert_eq!(counts.released.get(), 1);
    assert_eq!(counts.removed.get(), 0);
    assert_eq!(counts.closed.get(), 0);
    check_handle(handle);
}

#[test]
fn dropping_an_unregistered_record_closes_but_does_not_delete() {
    let counts = Rc::new(Counts::default());
    drop(owned(&counts));
    assert_eq!(counts.closed.get(), 1);
    assert_eq!(counts.released.get(), 1);
    assert_eq!(counts.removed.get(), 0);
}
