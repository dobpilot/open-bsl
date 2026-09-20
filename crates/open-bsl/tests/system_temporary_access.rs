//! Проверка системного носителя без ещё не реализованного права автоочистки.

#![cfg(target_os = "linux")]

use bsl_rt::SystemFileSystem;
use open_bsl::{Engine, FileSystem, Value};
use std::io::{Read, Seek, SeekFrom, Write};

const CHILD_ROOT: &str = "OPEN_BSL_TEMP_ACCESS_ROOT";

fn assert_bsl_read(path: &str, expected: Option<i64>) {
    for optimized in [false, true] {
        let engine = Engine::builder()
            .optimizations(if optimized {
                bsl_compiler::Optimizations::all()
            } else {
                bsl_compiler::Optimizations::default()
            })
            .build()
            .unwrap();
        for dynamic in [false, true] {
            let expression = format!("Новый ДвоичныеДанные(\"{}\")", path.replace('"', "\"\""));
            let expression = if dynamic {
                format!("Вычислить(\"{}\")", expression.replace('"', "\"\""))
            } else {
                expression
            };
            let module = engine
                .compile(&format!(
                    "ДанныеПробы = {expression}; Возврат ДанныеПробы.Размер();"
                ))
                .unwrap();
            for bytecode in [false, true] {
                let module = if bytecode {
                    engine.load_bytecode(&module.bytecode().unwrap()).unwrap()
                } else {
                    module.clone()
                };
                let result = engine.new_state().run(&module);
                if let Some(size) = expected {
                    assert_eq!(result.unwrap(), Value::number_from_i64(size));
                } else {
                    assert!(matches!(
                        result,
                        Err(open_bsl::Error::Runtime(open_bsl::RtError::IoError(_)))
                    ));
                }
            }
        }
    }
}

#[test]
fn system_temporary_access_child() {
    let Some(root) = std::env::var_os(CHILD_ROOT).map(std::path::PathBuf::from) else {
        return;
    };
    let (first_path, mut first) = SystemFileSystem
        .create_temporary_file(&[1; 16])
        .unwrap()
        .into_parts();
    let (second_path, second) = SystemFileSystem
        .create_temporary_file(&[2; 16])
        .unwrap()
        .into_parts();
    let mut bytes = Vec::new();
    for size in [1, 8, 8191, 8192, 8193] {
        bytes = vec![0; size];
        bytes[0] = 65;
        bytes[size - 1] = 90;
        first.seek(SeekFrom::Start(0)).unwrap();
        first.write_all(&bytes).unwrap();
        assert_eq!(
            SystemFileSystem.metadata(&first_path).unwrap().size(),
            Some(size as u64)
        );
        assert!(SystemFileSystem.read(&first_path).is_err());
    }
    first.seek(SeekFrom::Start(0)).unwrap();
    let mut original_read = Vec::new();
    first.read_to_end(&mut original_read).unwrap();
    assert_eq!(original_read, bytes);
    assert_eq!(
        SystemFileSystem.read(&first_path).unwrap_err().kind(),
        std::io::ErrorKind::PermissionDenied
    );
    let background = SystemFileSystem.background_access().unwrap();
    let background_path = first_path.clone();
    std::thread::spawn(move || assert!(background.read(&background_path).is_err()))
        .join()
        .unwrap();

    assert_bsl_read(&first_path, None);

    // file-temp-sharing подтверждает доступ по новой записи каталога:
    // hard link/rename сохраняют запрет, подмена по прежнему пути — нет.
    let alias = root.join("alias");
    std::fs::hard_link(&first_path, &alias).unwrap();
    let moved = root.join("moved");
    std::fs::rename(&first_path, &moved).unwrap();
    std::fs::write(&first_path, b"replacement").unwrap();
    assert_eq!(SystemFileSystem.read(&first_path).unwrap(), b"replacement");
    assert!(SystemFileSystem.read(alias.to_str().unwrap()).is_err());
    assert!(SystemFileSystem.read(moved.to_str().unwrap()).is_err());
    first.close().unwrap();
    drop(first);
    assert_eq!(
        SystemFileSystem.read(moved.to_str().unwrap()).unwrap(),
        bytes
    );
    assert_eq!(
        SystemFileSystem.read(alias.to_str().unwrap()).unwrap(),
        bytes
    );
    assert!(SystemFileSystem.read(&second_path).is_err());
    drop(second);
    assert!(SystemFileSystem.read(&second_path).unwrap().is_empty());
    assert_bsl_read(moved.to_str().unwrap(), Some(8193));
    // Освобождение обоих дескрипторов не удаляло ни носители, ни подмену.
    assert!(alias.exists() && moved.exists());
    assert_eq!(std::fs::read(first_path).unwrap(), b"replacement");

    let transferable = SystemFileSystem
        .create_transferable_temporary_file(&[3; 16])
        .unwrap();
    let transfer_path = std::thread::spawn(move || {
        let registry = open_bsl::TemporaryFileRegistry::default();
        let (path, handle) = transferable
            .into_local()
            .into_registered_parts(&registry)
            .unwrap();
        drop(handle);
        registry
            .take_pending()
            .into_iter()
            .next()
            .unwrap()
            .remove()
            .unwrap();
        path
    })
    .join()
    .unwrap();
    assert!(!std::path::Path::new(&transfer_path).exists());
}

#[test]
fn system_temporary_read_access_follows_the_opened_carrier_not_its_old_path() {
    let root = std::env::temp_dir().join(format!(
        "open-bsl-temp-access-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&root).unwrap();
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "system_temporary_access_child", "--nocapture"])
        .env(CHILD_ROOT, &root)
        .env("TEMP", &root)
        .env_remove("TMP")
        .output()
        .unwrap();
    // Каталог создан только этой пробой; после завершения дочернего процесса
    // его известные тестовые файлы больше никому не принадлежат.
    std::fs::remove_dir_all(&root).unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
