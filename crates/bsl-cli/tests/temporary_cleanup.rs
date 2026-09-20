//! Сквозные проверки границы очистки временных ресурсов CLI.

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_SCRATCH: AtomicU64 = AtomicU64::new(0);

struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "open-bsl-cli-old-temporary-{}-{}",
            std::process::id(),
            NEXT_SCRATCH.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn run(root: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_bsl-cli"))
        .args(args)
        .env("TEMP", root)
        .env("TMP", root)
        .output()
        .unwrap()
}

fn output_path(output: &Output) -> PathBuf {
    let stdout = String::from_utf8(output.stdout.clone()).unwrap();
    PathBuf::from(stdout.trim())
}

fn emit(root: &Path, stem: &str, source: &str) -> (PathBuf, PathBuf) {
    let script = root.join(format!("{stem}.bsl"));
    let bytecode = root.join(format!("{stem}.bslc"));
    std::fs::write(&script, source).unwrap();
    let output = run(
        root,
        &[
            "--emit-bytecode",
            script.to_str().unwrap(),
            bytecode.to_str().unwrap(),
        ],
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    (script, bytecode)
}

#[test]
fn a_new_cli_process_does_not_adopt_or_remove_an_old_temporary_file() {
    let scratch = Scratch::new();
    let script = scratch.0.join("empty.bsl");
    let bytecode = scratch.0.join("empty.bslc");
    let leftover = scratch.0.join("v8_deadbeef_deadbeef_deadbeef_deadbeef.tmp");
    std::fs::write(&script, "Возврат;\n").unwrap();
    std::fs::write(&leftover, b"old process").unwrap();

    let emit = run(
        &scratch.0,
        &[
            "--emit-bytecode",
            script.to_str().unwrap(),
            bytecode.to_str().unwrap(),
        ],
    );
    assert!(
        emit.status.success(),
        "{}",
        String::from_utf8_lossy(&emit.stderr)
    );
    assert_eq!(std::fs::read(&leftover).unwrap(), b"old process");

    for args in [
        vec![script.to_str().unwrap()],
        vec!["--run-bytecode", bytecode.to_str().unwrap()],
    ] {
        let output = run(&scratch.0, &args);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(std::fs::read(&leftover).unwrap(), b"old process");
    }
}

#[cfg(target_os = "linux")]
#[test]
fn cli_cleans_sync_and_async_bsl_temporary_files_for_source_and_bytecode() {
    let scratch = Scratch::new();
    for (stem, source, success) in [
        (
            "sync-success",
            "Поток = FileStreams.CreateTempFile();\nСообщить(Поток.FileName);\n",
            true,
        ),
        (
            "sync-error",
            "Поток = FileStreams.CreateTempFile();\nСообщить(Поток.FileName);\nВызватьИсключение \"ожидаемая ошибка\";\n",
            false,
        ),
        (
            "async-success",
            "Асинх Процедура П()\n\tПоток = Ждать FileStreams.CreateTempFileAsync();\n\tСообщить(Поток.FileName);\nКонецПроцедуры\nП();\n",
            true,
        ),
        (
            "async-error",
            "Асинх Процедура П()\n\tПоток = Ждать FileStreams.CreateTempFileAsync();\n\tСообщить(Поток.FileName);\n\tВызватьИсключение \"ожидаемая ошибка\";\nКонецПроцедуры\nП();\n",
            false,
        ),
    ] {
        let (script, bytecode) = emit(&scratch.0, stem, source);
        for args in [
            vec![script.to_str().unwrap()],
            vec!["--run-bytecode", bytecode.to_str().unwrap()],
        ] {
            let output = run(&scratch.0, &args);
            assert_eq!(
                output.status.success(),
                success,
                "{stem}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            let path = output_path(&output);
            assert!(path.starts_with(&scratch.0), "{stem}: {}", path.display());
            assert!(!path.exists(), "{stem}: {}", path.display());
        }
    }
}

#[cfg(target_os = "linux")]
#[test]
fn cli_refuses_to_remove_a_replacement_without_changing_success() {
    let scratch = Scratch::new();
    let (script, bytecode) = emit(
        &scratch.0,
        "replacement",
        r#"
Поток = FileStreams.CreateTempFile();
Имя = Поток.FileName;
Сообщить(Имя);
Поток.Close();
УдалитьФайлы(Имя);
Запись = Новый ЗаписьТекста(Имя);
Запись.Записать("replacement");
Запись.Закрыть();
"#,
    );
    for args in [
        vec![script.to_str().unwrap()],
        vec!["--run-bytecode", bytecode.to_str().unwrap()],
    ] {
        let output = run(&scratch.0, &args);
        assert!(output.status.success());
        let path = output_path(&output);
        assert_eq!(std::fs::read(&path).unwrap(), b"\xef\xbb\xbfreplacement");
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(stderr.contains("Ошибка очистки временных файлов: PermissionDenied"));
        assert!(!stderr.contains(path.to_str().unwrap()));
    }
}

#[cfg(target_os = "linux")]
#[test]
fn killing_cli_does_not_run_cleanup_or_adopt_the_file_later() {
    use std::io::BufRead;

    let scratch = Scratch::new();
    let (script, _) = emit(
        &scratch.0,
        "killed",
        "Поток = FileStreams.CreateTempFile();\nСообщить(Поток.FileName);\nПока Истина Цикл КонецЦикла;\n",
    );
    let mut child = Command::new(env!("CARGO_BIN_EXE_bsl-cli"))
        .arg(&script)
        .env("TEMP", &scratch.0)
        .env("TMP", &scratch.0)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut line = String::new();
    std::io::BufReader::new(child.stdout.take().unwrap())
        .read_line(&mut line)
        .unwrap();
    let path = PathBuf::from(line.trim());
    assert!(path.exists());
    child.kill().unwrap();
    assert!(!child.wait().unwrap().success());
    assert!(path.exists());

    let empty = scratch.0.join("after-kill.bsl");
    std::fs::write(&empty, "Возврат;\n").unwrap();
    assert!(run(&scratch.0, &[empty.to_str().unwrap()]).status.success());
    assert!(path.exists());
    std::fs::remove_file(path).unwrap();
}
