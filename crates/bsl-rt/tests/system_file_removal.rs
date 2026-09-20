#![cfg(unix)]

use bsl_rt::{FileSystem, SystemFileSystem};
use std::{fs, path::PathBuf};

struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "open-bsl-removal-review-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn removing_a_selected_directory_link_preserves_its_target() {
    for suffix in ["", "/", "///"] {
        let scratch = Scratch::new();
        let target = scratch.0.join("target");
        fs::create_dir(&target).unwrap();
        let sentinel = target.join("sentinel");
        fs::write(&sentinel, b"keep").unwrap();
        let link = scratch.0.join("selected");
        std::os::unix::fs::symlink(&target, &link).unwrap();
        let request = format!("{}{suffix}", link.display());
        let result = SystemFileSystem.remove_path(&request);
        let contents = fs::read(&sentinel)
            .unwrap_or_else(|error| panic!("цель повреждена: {suffix:?}, {result:?}, {error}"));
        assert_eq!(contents, b"keep", "{suffix:?}, {result:?}");
        assert!(result.is_ok(), "{suffix:?}, {result:?}");
        assert!(fs::symlink_metadata(&link).is_err(), "{suffix:?}");
    }
}

#[test]
fn removing_file_and_broken_links_never_removes_or_creates_the_target() {
    for present in [false, true] {
        for suffix in ["", "/", "///"] {
            let scratch = Scratch::new();
            let target = scratch.0.join("target");
            if present {
                fs::write(&target, b"keep").unwrap();
            }
            let link = scratch.0.join("selected");
            std::os::unix::fs::symlink(&target, &link).unwrap();
            SystemFileSystem
                .remove_path(&format!("{}{suffix}", link.display()))
                .unwrap();
            if present {
                assert_eq!(fs::read(&target).unwrap(), b"keep");
            } else {
                assert_eq!(
                    fs::metadata(&target).unwrap_err().kind(),
                    std::io::ErrorKind::NotFound
                );
            }
            assert_eq!(
                fs::symlink_metadata(&link).unwrap_err().kind(),
                std::io::ErrorKind::NotFound
            );
        }
    }
}

#[test]
fn actual_directories_with_trailing_separators_are_still_removed() {
    for suffix in ["", "/", "///"] {
        let scratch = Scratch::new();
        let selected = scratch.0.join("selected ");
        fs::create_dir(&selected).unwrap();
        fs::write(selected.join("child"), b"remove").unwrap();
        SystemFileSystem
            .remove_path(&format!("{}{suffix}", selected.display()))
            .unwrap();
        assert_eq!(
            fs::metadata(&selected).unwrap_err().kind(),
            std::io::ErrorKind::NotFound
        );
    }
}

#[test]
fn trailing_separators_do_not_allow_removing_an_ordinary_file() {
    for suffix in ["/", "///"] {
        let scratch = Scratch::new();
        let file = scratch.0.join("plain");
        fs::write(&file, b"keep").unwrap();
        let request = format!("{}{suffix}", file.display());
        assert_eq!(
            SystemFileSystem.remove_path(&request).unwrap_err().kind(),
            std::io::ErrorKind::NotADirectory
        );
        assert_eq!(fs::read(&file).unwrap(), b"keep");
    }
}

#[test]
fn removing_a_tree_preserves_targets_of_links_inside_it() {
    let scratch = Scratch::new();
    let target = scratch.0.join("target");
    fs::create_dir(&target).unwrap();
    let sentinel = target.join("sentinel");
    fs::write(&sentinel, b"keep").unwrap();
    let selected = scratch.0.join("selected");
    fs::create_dir(&selected).unwrap();
    std::os::unix::fs::symlink(&target, selected.join("linked")).unwrap();
    SystemFileSystem
        .remove_path(selected.to_str().unwrap())
        .unwrap();
    assert_eq!(fs::read(&sentinel).unwrap(), b"keep");
    assert!(!selected.exists());
}

#[test]
fn system_filesystem_does_not_confine_parent_components_to_a_root() {
    let scratch = Scratch::new();
    let target = scratch.0.join("target");
    fs::create_dir(&target).unwrap();
    fs::write(target.join("sentinel"), b"keep").unwrap();
    let alias = scratch.0.join("alias");
    std::os::unix::fs::symlink(&target, &alias).unwrap();
    // Обычный системный host разрешает ссылку в родительском компоненте.
    // Это не проверка политики ограничивающего host и не её замена.
    assert_eq!(
        SystemFileSystem
            .read(alias.join("sentinel").to_str().unwrap())
            .unwrap(),
        b"keep"
    );
    SystemFileSystem
        .remove_path(alias.join("sentinel").to_str().unwrap())
        .unwrap();
    assert_eq!(
        fs::metadata(target.join("sentinel")).unwrap_err().kind(),
        std::io::ErrorKind::NotFound
    );
}
