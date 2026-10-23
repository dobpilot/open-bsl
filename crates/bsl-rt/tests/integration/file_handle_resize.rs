//! Изменение размера открытого host-файла; не измерение семантики BSL.

use bsl_rt::FileHandle;
use std::io::{self, Read, Seek, SeekFrom, Write};

#[derive(Debug)]
struct LegacyHandle;

impl Read for LegacyHandle {
    fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
        panic!("default set_len не должен читать")
    }
}

impl Write for LegacyHandle {
    fn write(&mut self, _: &[u8]) -> io::Result<usize> {
        panic!("default set_len не должен писать")
    }

    fn flush(&mut self) -> io::Result<()> {
        panic!("default set_len не должен сбрасывать буфер")
    }
}

impl Seek for LegacyHandle {
    fn seek(&mut self, _: SeekFrom) -> io::Result<u64> {
        panic!("default set_len не должен менять позицию")
    }
}

impl FileHandle for LegacyHandle {
    fn len(&self) -> io::Result<u64> {
        panic!("default set_len не должен читать длину")
    }

    fn close(&mut self) -> io::Result<()> {
        panic!("default set_len не должен закрывать дескриптор")
    }
}

#[test]
fn old_host_refuses_resize_without_calling_other_operations() {
    let handle: &mut dyn FileHandle = &mut LegacyHandle;
    for length in [0, 6, u64::MAX] {
        assert_eq!(
            handle.set_len(length).unwrap_err().kind(),
            io::ErrorKind::Unsupported
        );
    }
}

struct Scratch(std::path::PathBuf);

impl Scratch {
    fn create() -> (Self, std::fs::File) {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "open-bsl-handle-resize-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
        ));
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(&path)
            .unwrap();
        (Self(path), file)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        std::fs::remove_file(&self.0).unwrap();
    }
}

#[test]
fn system_resize_preserves_position_and_zero_fills_extension() {
    let (_scratch, mut file) = Scratch::create();
    let handle: &mut dyn FileHandle = &mut file;
    handle.write_all(b"abcdef").unwrap();
    handle.seek(SeekFrom::Start(5)).unwrap();
    handle.set_len(2).unwrap();
    assert_eq!(handle.len().unwrap(), 2);
    assert_eq!(handle.stream_position().unwrap(), 5);
    handle.set_len(6).unwrap();
    assert_eq!(handle.len().unwrap(), 6);
    assert_eq!(handle.stream_position().unwrap(), 5);
    handle.seek(SeekFrom::Start(0)).unwrap();
    let mut bytes = Vec::new();
    handle.read_to_end(&mut bytes).unwrap();
    assert_eq!(bytes, b"ab\0\0\0\0");
    handle.set_len(0).unwrap();
    assert_eq!(handle.len().unwrap(), 0);
    assert_eq!(handle.stream_position().unwrap(), 6);
}

#[test]
fn read_only_resize_fails_without_changing_file_or_position() {
    let (scratch, mut file) = Scratch::create();
    file.write_all(b"preserved").unwrap();
    drop(file);
    let mut file = std::fs::File::open(&scratch.0).unwrap();
    let handle: &mut dyn FileHandle = &mut file;
    handle.seek(SeekFrom::Start(3)).unwrap();
    assert!(handle.set_len(1).is_err());
    assert_eq!(handle.stream_position().unwrap(), 3);
    assert_eq!(handle.len().unwrap(), 9);
    assert_eq!(std::fs::read(&scratch.0).unwrap(), b"preserved");
}
