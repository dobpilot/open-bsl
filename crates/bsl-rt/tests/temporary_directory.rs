//! Выбор временного каталога — политика стандартного host, не oracle 1С.

use bsl_rt::{FileSystem, SystemFileSystem};
use std::path::{Path, PathBuf};
use std::process::Command;

const CHILD_ROOT: &str = "OPEN_BSL_TEMP_TEST_ROOT";
const CHILD_CASE: &str = "OPEN_BSL_TEMP_TEST_CASE";

#[derive(Debug)]
struct NoTemporaryDirectory;

impl FileSystem for NoTemporaryDirectory {
    fn read(&self, _: &str) -> std::io::Result<Vec<u8>> {
        panic!("получение временного каталога не должно читать файлы")
    }

    fn write(&self, _: &str, _: &[u8]) -> std::io::Result<()> {
        panic!("получение временного каталога не должно писать файлы")
    }

    fn metadata(&self, _: &str) -> std::io::Result<bsl_rt::FileMetadata> {
        panic!("default не должен угадывать путь через metadata")
    }

    fn read_dir<'fs>(
        &'fs self,
        _: &str,
    ) -> std::io::Result<Box<dyn Iterator<Item = std::io::Result<bsl_rt::DirEntry>> + 'fs>> {
        panic!("default не должен обходить каталоги")
    }

    fn create_dir_all(&self, _: &str) -> std::io::Result<()> {
        panic!("получение временного каталога не должно создавать каталоги")
    }

    fn open(
        &self,
        _: &str,
        _: bsl_rt::FileOpenOptions,
    ) -> std::io::Result<Box<dyn bsl_rt::FileHandle>> {
        panic!("получение временного каталога не должно открывать файлы")
    }
}

#[test]
fn custom_filesystem_defaults_to_unsupported_without_other_file_operations() {
    let files: &dyn FileSystem = &NoTemporaryDirectory;
    assert!(!files.supports_temporary_file_ownership());
    assert_eq!(
        files
            .create_transferable_temporary_file(&[31; 16])
            .unwrap_err()
            .kind(),
        std::io::ErrorKind::Unsupported
    );
    assert_eq!(
        files.create_temporary_file(&[31; 16]).unwrap_err().kind(),
        std::io::ErrorKind::Unsupported
    );
    assert_eq!(
        files.set_hidden("unavailable", true).unwrap_err().kind(),
        std::io::ErrorKind::Unsupported
    );
    assert_eq!(
        files.temporary_directory().unwrap_err().kind(),
        std::io::ErrorKind::Unsupported
    );
    assert_eq!(
        files.set_modified("unavailable", 0).unwrap_err().kind(),
        std::io::ErrorKind::Unsupported
    );
    assert_eq!(
        files
            .set_read_only("unavailable", false)
            .unwrap_err()
            .kind(),
        std::io::ErrorKind::Unsupported
    );
}

#[test]
fn create_directory_matches_the_measured_existing_parent_and_file_cases() {
    let root = std::env::temp_dir().join(format!(
        "open-bsl-create-directory-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&root).unwrap();
    let scratch = Scratch(root);
    let path = scratch.0.join("parent/child");
    let args = [bsl_rt::BslValue::Str(bsl_rt::BslString::from_str(
        path.to_str().unwrap(),
    ))];
    let mut shapes = bsl_rt::RuntimeShapes::seeded(Vec::new(), Vec::new(), None);
    for name in ["СоздатьКаталог", "CreateDirectory"] {
        let builtin = bsl_rt::BuiltinFn::lookup(name).expect("имя зарегистрировано");
        assert!(builtin.is_procedure());
        assert_eq!(builtin.arity_range(), (1, 1));
        assert!(matches!(
            bsl_rt::call_builtin_files(builtin, &args, &mut shapes, &SystemFileSystem),
            Ok(bsl_rt::BslValue::Undefined)
        ));
        assert!(path.is_dir());
    }
    let file = scratch.0.join("ordinary-file");
    std::fs::write(&file, b"preserved").unwrap();
    let args = [bsl_rt::BslValue::Str(bsl_rt::BslString::from_str(
        file.to_str().unwrap(),
    ))];
    let builtin = bsl_rt::BuiltinFn::lookup("СоздатьКаталог").unwrap();
    assert!(matches!(
        bsl_rt::call_builtin_files(builtin, &args, &mut shapes, &SystemFileSystem),
        Err(bsl_rt::RtError::IoError(_))
    ));
    assert_eq!(std::fs::read(file).unwrap(), b"preserved");

    // Фактические пути подтверждены внешним снимком file-create-layout.host.txt.
    // Поиск BSL не используется как свидетельство существования имён ОС.
    for (input, actual) in [
        ("missing/../leaf", "leaf"),
        ("ordinary-file/../leaf2", "leaf2"),
        ("branch\\leaf", "branch/leaf"),
        (" spaced /leaf ", " spaced /leaf"),
        ("nul\0tail", "nul"),
        ("colon/C:leaf", "colon/C:leaf"),
    ] {
        let path = format!("{}/{input}", scratch.0.to_str().unwrap());
        let args = [bsl_rt::BslValue::Str(bsl_rt::BslString::from_str(&path))];
        bsl_rt::call_builtin_files(builtin, &args, &mut shapes, &SystemFileSystem).unwrap();
        assert!(scratch.0.join(actual).is_dir(), "{input:?}");
    }
    assert!(!scratch.0.join("missing").exists());
    #[cfg(unix)]
    {
        assert!(!scratch.0.join(" spaced /leaf ").exists());
        assert!(!scratch.0.join("branch\\leaf").exists());
    }
    assert_eq!(
        std::fs::read(scratch.0.join("ordinary-file")).unwrap(),
        b"preserved"
    );
}

#[test]
fn temporary_path_child() {
    let Some(root) = std::env::var_os(CHILD_ROOT).map(PathBuf::from) else {
        return;
    };
    let case = std::env::var(CHILD_CASE).unwrap();
    #[cfg(unix)]
    if case == "non_utf8" {
        assert_eq!(
            SystemFileSystem
                .create_temporary_file(&[46; 16])
                .unwrap_err()
                .kind(),
            std::io::ErrorKind::InvalidInput
        );
        assert_eq!(
            std::fs::read_dir(std::env::var_os("TEMP").unwrap())
                .unwrap()
                .count(),
            0
        );
        return;
    }
    let result = SystemFileSystem.temporary_path(".bin", &[43; 16]);
    let opened = SystemFileSystem.create_temporary_file(&[45; 16]);
    let directory = SystemFileSystem.temporary_directory();
    let mut shapes = bsl_rt::RuntimeShapes::seeded(Vec::new(), Vec::new(), None);
    let directory_builtin = bsl_rt::call_builtin_files(
        bsl_rt::BuiltinFn::TempFilesDir,
        &[],
        &mut shapes,
        &SystemFileSystem,
    );
    if matches!(case.as_str(), "missing" | "file") {
        assert!(opened.is_err());
        assert!(
            result.is_err(),
            "некорректный TEMP не должен включать fallback"
        );
        assert!(!root.join("missing").exists());
        assert!(directory.is_err());
        assert!(matches!(
            directory_builtin,
            Err(bsl_rt::RtError::IoError(_))
        ));
    } else {
        let expected = match case.as_str() {
            "temp" | "trailing_temp" => root.join("temp"),
            "tmp" | "empty_temp" => root.join("tmp"),
            "fallback" | "empty_both" => std::env::temp_dir(),
            _ => panic!("неизвестный сценарий"),
        };
        let path = PathBuf::from(result.unwrap());
        // Linux-политика доверяет процессам того же Unix UID;
        // другие платформы ещё не имеют системного ресурса очистки.
        assert_eq!(
            SystemFileSystem.supports_temporary_file_ownership(),
            cfg!(target_os = "linux")
        );
        let (opened_path, mut handle) = opened.unwrap().into_parts();
        let opened_path = PathBuf::from(opened_path);
        assert_eq!(opened_path.parent(), Some(expected.as_path()));
        assert_eq!(handle.len().unwrap(), 0);
        std::io::Write::write_all(&mut handle, b"owned").unwrap();
        assert_eq!(
            SystemFileSystem
                .create_temporary_file(&[45; 16])
                .unwrap_err()
                .kind(),
            std::io::ErrorKind::AlreadyExists
        );
        std::io::Seek::seek(&mut handle, std::io::SeekFrom::Start(0)).unwrap();
        let mut contents = Vec::new();
        std::io::Read::read_to_end(&mut handle, &mut contents).unwrap();
        assert_eq!(contents, b"owned");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(&opened_path)
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o077,
                0
            );
        }
        drop(handle);
        assert_eq!(
            std::fs::read(&opened_path).unwrap(),
            b"owned",
            "закрытие не удаляет файл"
        );
        std::fs::remove_file(&opened_path).unwrap();
        #[cfg(unix)]
        if case == "temp" {
            std::os::unix::fs::symlink(root.join("file"), &opened_path).unwrap();
            assert_eq!(
                SystemFileSystem
                    .create_temporary_file(&[45; 16])
                    .unwrap_err()
                    .kind(),
                std::io::ErrorKind::AlreadyExists
            );
            assert_eq!(std::fs::read(root.join("file")).unwrap(), b"ordinary file");
            std::fs::remove_file(&opened_path).unwrap();
        }
        assert_eq!(PathBuf::from(directory.unwrap()), expected);
        let bsl_rt::BslValue::Str(directory_builtin) = directory_builtin.unwrap() else {
            panic!("временный каталог должен быть строкой");
        };
        let directory_builtin = directory_builtin.to_string();
        assert!(directory_builtin.ends_with(std::path::MAIN_SEPARATOR));
        let mut expected_text = expected.to_string_lossy().into_owned();
        if !expected_text.ends_with(std::path::MAIN_SEPARATOR) {
            expected_text.push(std::path::MAIN_SEPARATOR);
        }
        assert_eq!(directory_builtin, expected_text);
        assert_eq!(PathBuf::from(directory_builtin), expected);
        assert_eq!(path.parent(), Some(expected.as_path()));
        assert!(!path.exists(), "выданное имя не должно оставлять файл");
        assert_eq!(path.extension().unwrap(), "bin");
        if case == "temp" {
            let nested = PathBuf::from(SystemFileSystem.temporary_path(".a/b", &[44; 16]).unwrap());
            let parent = nested.parent().unwrap();
            assert_eq!(parent.parent(), Some(expected.as_path()));
            assert!(
                !parent.exists(),
                "резервирование не оставляет родителя суффикса"
            );
            assert!(!nested.exists());
            std::fs::create_dir(parent).unwrap();
            let neighbor = parent.join("keep");
            std::fs::write(&neighbor, b"keep").unwrap();
            assert!(SystemFileSystem.temporary_path(".a/b", &[44; 16]).is_err());
            assert_eq!(std::fs::read(&neighbor).unwrap(), b"keep");
            std::fs::remove_file(neighbor).unwrap();
            std::fs::remove_dir(parent).unwrap();

            let registry = bsl_rt::TemporaryFileRegistry::default();
            let owned = SystemFileSystem.create_temporary_file(&[47; 16]).unwrap();
            let (owned_path, owned_handle) = owned.into_registered_parts(&registry).unwrap();
            drop(owned_handle);
            assert!(std::path::Path::new(&owned_path).exists());
            let resources = registry.take_pending();
            assert_eq!(resources.len(), 1);
            resources.into_iter().next().unwrap().remove().unwrap();
            assert!(!std::path::Path::new(&owned_path).exists());

            let registry = bsl_rt::TemporaryFileRegistry::default();
            let replaced = SystemFileSystem.create_temporary_file(&[48; 16]).unwrap();
            let (replaced_path, replaced_handle) =
                replaced.into_registered_parts(&registry).unwrap();
            drop(replaced_handle);
            let moved_path = root.join("moved-owned-temp");
            std::fs::rename(&replaced_path, &moved_path).unwrap();
            std::fs::write(&replaced_path, b"replacement").unwrap();
            let error = registry
                .take_pending()
                .into_iter()
                .next()
                .unwrap()
                .remove()
                .unwrap_err();
            assert_eq!(error.kind(), std::io::ErrorKind::PermissionDenied);
            assert_eq!(std::fs::read(&replaced_path).unwrap(), b"replacement");
            assert!(moved_path.exists());
            std::fs::remove_file(replaced_path).unwrap();
            std::fs::remove_file(moved_path).unwrap();
        }
    }
    for directory in ["temp", "tmp", "fallback"] {
        assert_eq!(std::fs::read_dir(root.join(directory)).unwrap().count(), 0);
    }
}

struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}

fn run_case(root: &Path, case: &str) {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args(["--exact", "temporary_path_child", "--nocapture"])
        .env(CHILD_ROOT, root)
        .env(CHILD_CASE, case)
        .env_remove("TEMP")
        .env_remove("TMP")
        .env("TMPDIR", root.join("fallback"));
    match case {
        "trailing_temp" => {
            command.env(
                "TEMP",
                format!(
                    "{}{sep}",
                    root.join("temp").display(),
                    sep = std::path::MAIN_SEPARATOR
                ),
            );
        }
        "temp" => {
            command
                .env("TEMP", root.join("temp"))
                .env("TMP", root.join("tmp"));
        }
        "tmp" => {
            command.env("TMP", root.join("tmp"));
        }
        "empty_temp" => {
            command.env("TEMP", "").env("TMP", root.join("tmp"));
        }
        "empty_both" => {
            command.env("TEMP", "").env("TMP", "");
        }
        "missing" | "file" => {
            command
                .env("TEMP", root.join(case))
                .env("TMP", root.join("tmp"));
        }
        "fallback" => {}
        #[cfg(unix)]
        "non_utf8" => {
            use std::os::unix::ffi::OsStringExt;
            command.env("TEMP", root.join(std::ffi::OsString::from_vec(vec![0xff])));
        }
        _ => unreachable!(),
    }
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "{case}: {}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn standard_host_uses_temp_then_tmp_without_fallback_on_invalid_path() {
    let root = std::env::temp_dir().join(format!(
        "open-bsl-temp-env-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&root).unwrap();
    let scratch = Scratch(root);
    for directory in ["temp", "tmp", "fallback"] {
        std::fs::create_dir(scratch.0.join(directory)).unwrap();
    }
    std::fs::write(scratch.0.join("file"), b"ordinary file").unwrap();
    for case in [
        "temp",
        "trailing_temp",
        "tmp",
        "empty_temp",
        "fallback",
        "empty_both",
        "missing",
        "file",
    ] {
        run_case(&scratch.0, case);
    }
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStringExt;
        std::fs::create_dir(scratch.0.join(std::ffi::OsString::from_vec(vec![0xff]))).unwrap();
        run_case(&scratch.0, "non_utf8");
    }
}
