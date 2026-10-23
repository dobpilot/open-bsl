//! Сведения host о виде и размере объекта, без предположений о BSL-ошибках.

use bsl_rt::{FileMetadata, FileSystem, SystemFileSystem};

#[test]
fn legacy_metadata_constructors_do_not_invent_a_size() {
    let file = FileMetadata::file(Some(123));
    assert!(file.is_file());
    assert!(!file.is_dir());
    assert_eq!(file.modified(), Some(123));
    assert_eq!(file.size(), None);
    assert_eq!(file.read_only(), None);
    assert_eq!(file.hidden(), None);
    let directory = FileMetadata::directory(None);
    assert!(!directory.is_file());
    assert!(directory.is_dir());
    assert_eq!(directory.size(), None);
    assert_eq!(directory.read_only(), None);
    assert_eq!(directory.hidden(), None);
    let other = FileMetadata::other(None);
    assert!(!other.is_file());
    assert!(!other.is_dir());
    assert_eq!(other.read_only(), None);
    assert_eq!(other.hidden(), None);
    assert_eq!(other.clone().with_hidden(true).hidden(), Some(true));
    assert_eq!(other.clone().with_read_only(false).read_only(), Some(false));
    assert_eq!(other.with_size(0).size(), Some(0));
}

#[cfg(target_os = "linux")]
#[test]
fn system_fifo_and_unix_socket_are_measured_files() {
    let root = std::env::temp_dir().join(format!(
        "open-bsl-special-file-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&root).unwrap();
    let scratch = Scratch(root);
    let fifo = scratch.0.join("fifo");
    let status = std::process::Command::new("/usr/bin/mkfifo")
        .arg(&fifo)
        .status()
        .unwrap();
    assert!(status.success());
    let fifo = SystemFileSystem.metadata(fifo.to_str().unwrap()).unwrap();
    assert!(fifo.is_file());
    assert!(!fifo.is_dir());
    assert_eq!(fifo.size(), Some(0));

    let socket = scratch.0.join("socket");
    let listener = std::os::unix::net::UnixListener::bind(&socket).unwrap();
    let socket = SystemFileSystem.metadata(socket.to_str().unwrap()).unwrap();
    assert!(socket.is_file());
    assert!(!socket.is_dir());
    assert_eq!(socket.size(), Some(0));
    drop(listener);
}

#[cfg(target_os = "linux")]
#[test]
fn system_character_device_is_a_measured_file() {
    let device = SystemFileSystem.metadata("/dev/null").unwrap();
    assert!(device.is_file());
    assert!(!device.is_dir());
    assert_eq!(device.size(), Some(0));
}

#[cfg(unix)]
#[test]
fn system_read_only_preserves_data_and_does_not_grant_write_to_other_users() {
    use std::os::unix::fs::PermissionsExt;
    let root = std::env::temp_dir().join(format!(
        "open-bsl-read-only-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&root).unwrap();
    let scratch = Scratch(root);
    let path = scratch.0.join("file");
    std::fs::write(&path, b"preserved").unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o750)).unwrap();
    let link = scratch.0.join("link");
    std::os::unix::fs::symlink(&path, &link).unwrap();
    SystemFileSystem
        .set_read_only(link.to_str().unwrap(), true)
        .unwrap();
    assert_eq!(
        std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o550
    );
    assert_eq!(
        SystemFileSystem
            .metadata(path.to_str().unwrap())
            .unwrap()
            .read_only(),
        Some(true)
    );
    SystemFileSystem
        .set_read_only(path.to_str().unwrap(), false)
        .unwrap();
    assert_eq!(
        std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o750
    );
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o440)).unwrap();
    SystemFileSystem
        .set_read_only(path.to_str().unwrap(), false)
        .unwrap();
    assert_eq!(
        std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o640
    );
    assert_eq!(
        SystemFileSystem
            .metadata(path.to_str().unwrap())
            .unwrap()
            .read_only(),
        Some(false)
    );
    assert_eq!(std::fs::read(&path).unwrap(), b"preserved");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o660)).unwrap();
    SystemFileSystem
        .set_read_only(path.to_str().unwrap(), true)
        .unwrap();
    assert_eq!(
        std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o460
    );
    assert_eq!(
        SystemFileSystem
            .metadata(path.to_str().unwrap())
            .unwrap()
            .read_only(),
        Some(true)
    );
    assert!(std::fs::symlink_metadata(&link).unwrap().is_symlink());
    let missing = scratch.0.join("missing");
    assert!(
        SystemFileSystem
            .set_read_only(missing.to_str().unwrap(), false)
            .is_err()
    );
    assert!(!missing.exists());
    #[cfg(target_os = "linux")]
    {
        let hidden = scratch.0.join(".hidden");
        std::fs::write(&hidden, b"preserved").unwrap();
        let permissions = std::fs::metadata(&hidden).unwrap().permissions().mode();
        for value in [true, false] {
            SystemFileSystem
                .set_hidden(hidden.to_str().unwrap(), value)
                .unwrap();
            assert_eq!(
                SystemFileSystem
                    .metadata(hidden.to_str().unwrap())
                    .unwrap()
                    .hidden(),
                Some(false)
            );
            assert_eq!(std::fs::read(&hidden).unwrap(), b"preserved");
            assert_eq!(
                std::fs::metadata(&hidden).unwrap().permissions().mode(),
                permissions
            );
        }
        assert!(
            SystemFileSystem
                .set_hidden(missing.to_str().unwrap(), true)
                .is_err()
        );
        assert!(!missing.exists());
    }
}

struct Scratch(std::path::PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn system_metadata_reports_size_and_follows_links_and_special_files() {
    let root = std::env::temp_dir().join(format!(
        "open-bsl-file-metadata-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&root).unwrap();
    let scratch = Scratch(root);
    let path = scratch.0.join("file");
    std::fs::write(&path, b"abc").unwrap();
    let file = SystemFileSystem.metadata(path.to_str().unwrap()).unwrap();
    assert!(file.is_file());
    assert!(!file.is_dir());
    assert_eq!(file.size(), Some(3));
    let before = std::fs::metadata(&path).unwrap().accessed().unwrap();
    for seconds in [1_594_815_296, 0] {
        SystemFileSystem
            .set_modified(path.to_str().unwrap(), seconds)
            .unwrap();
        assert_eq!(
            SystemFileSystem
                .metadata(path.to_str().unwrap())
                .unwrap()
                .modified(),
            Some(seconds)
        );
        assert_eq!(
            std::fs::metadata(&path).unwrap().accessed().unwrap(),
            before
        );
    }
    #[cfg(unix)]
    {
        SystemFileSystem
            .set_modified(path.to_str().unwrap(), -1)
            .unwrap();
        assert_eq!(
            SystemFileSystem
                .metadata(path.to_str().unwrap())
                .unwrap()
                .modified(),
            Some(-1)
        );
        std::fs::File::open(&path)
            .unwrap()
            .set_times(
                std::fs::FileTimes::new()
                    .set_modified(std::time::UNIX_EPOCH - std::time::Duration::from_millis(500)),
            )
            .unwrap();
        assert_eq!(
            SystemFileSystem
                .metadata(path.to_str().unwrap())
                .unwrap()
                .modified(),
            Some(-1)
        );
    }
    assert_eq!(std::fs::read(&path).unwrap(), b"abc");
    let absent = scratch.0.join("absent");
    assert_eq!(
        SystemFileSystem
            .set_modified(absent.to_str().unwrap(), 0)
            .unwrap_err()
            .kind(),
        std::io::ErrorKind::NotFound
    );
    assert!(!absent.exists());
    let directory = SystemFileSystem
        .metadata(scratch.0.to_str().unwrap())
        .unwrap();
    assert!(directory.is_dir());
    assert!(!directory.is_file());
    #[cfg(unix)]
    {
        let link = scratch.0.join("link");
        std::os::unix::fs::symlink(&path, &link).unwrap();
        let target = SystemFileSystem.metadata(link.to_str().unwrap()).unwrap();
        assert!(target.is_file());
        assert_eq!(target.size(), Some(3));
        SystemFileSystem
            .set_modified(link.to_str().unwrap(), 123)
            .unwrap();
        assert_eq!(
            SystemFileSystem
                .metadata(path.to_str().unwrap())
                .unwrap()
                .modified(),
            Some(123)
        );
        SystemFileSystem
            .set_modified(scratch.0.to_str().unwrap(), 456)
            .unwrap();
        assert_eq!(
            SystemFileSystem
                .metadata(scratch.0.to_str().unwrap())
                .unwrap()
                .modified(),
            Some(456)
        );
        let socket = scratch.0.join("socket");
        let listener = std::os::unix::net::UnixListener::bind(&socket).unwrap();
        let special = SystemFileSystem.metadata(socket.to_str().unwrap()).unwrap();
        assert!(special.is_file());
        assert!(!special.is_dir());
        assert_eq!(special.size(), Some(0));
        drop(listener);
    }
}
