#![cfg(target_os = "linux")]

use std::{
    path::Path,
    process::Command,
    time::{Duration, Instant},
};

#[path = "../../../tests/support/application_system.rs"]
mod application_system;

fn wait_for(path: &Path) {
    let deadline = Instant::now() + Duration::from_secs(15);
    while !path.exists() {
        assert!(Instant::now() < deadline, "не появился {}", path.display());
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn document_handler() {
    let Some(root) = std::env::var_os("OPEN_BSL_DOCUMENT_TEST_ROOT") else {
        return;
    };
    let args: Vec<String> = std::env::args().collect();
    let Some(pair) = args.windows(2).find(|pair| pair[0] == "--skip") else {
        return;
    };
    let path = Path::new(&pair[1]);
    assert!(path.is_absolute() && path.starts_with(root));
    std::fs::write(path.with_extension("received"), pair[1].as_bytes()).unwrap();
    wait_for(&path.with_extension("continue"));
    std::fs::write(path.with_extension("finished"), b"handler finished").unwrap();
}

#[test]
fn native_child() {
    application_system::native_child();
}

fn bsl_string(text: &str) -> String {
    format!("\"{}\"", text.replace('"', "\"\""))
}

fn isolated_command(root: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_bsl-cli"));
    command
        .env("XDG_DATA_HOME", root.join("data"))
        .env("XDG_CONFIG_HOME", root.join("config"))
        .env("XDG_DATA_DIRS", root.join("empty"))
        .env("XDG_CONFIG_DIRS", root.join("empty"))
        .env("XDG_CURRENT_DESKTOP", "")
        .env("OPEN_BSL_DOCUMENT_TEST_ROOT", root);
    command
}

#[test]
fn documents_open_without_waiting_for_the_handler_and_reject_exit_waits() {
    let scratch = application_system::scratch();
    let root = &scratch.0;
    std::fs::create_dir_all(root.join("data/applications")).unwrap();
    std::fs::create_dir(root.join("config")).unwrap();
    std::fs::create_dir(root.join("empty")).unwrap();
    let handler = root.join("handler");
    std::os::unix::fs::symlink(std::env::current_exe().unwrap(), &handler).unwrap();
    std::fs::write(root.join("data/applications/open-bsl-test.desktop"), format!(
        "[Desktop Entry]\nType=Application\nName=Open BSL isolated test\nExec={} --exact document_handler --nocapture --skip %f\nMimeType=text/plain;\nTerminal=false\n", handler.display()
    )).unwrap();
    std::fs::write(
        root.join("config/mimeapps.list"),
        "[Default Applications]\ntext/plain=open-bsl-test.desktop;\n",
    )
    .unwrap();
    let script = root.join("test.bsl");
    let listing = root.join("test.bslc");
    let mut index = 0;
    for optimize in [false, true] {
        for bytecode in [false, true] {
            for dynamic in [false, true] {
                for (sync, asynchronous) in [
                    ("RunApp", "RunAppAsync"),
                    ("ЗапуститьПриложение", "ЗапуститьПриложениеАсинх"),
                ] {
                    for is_async in [false, true] {
                        index += 1;
                        // Начальный дефис и shell-символы не становятся опцией/кодом.
                        let name = format!("-document $(literal) {index}.txt");
                        let document = root.join(&name);
                        std::fs::write(&document, b"owned test document\n").unwrap();
                        let command = bsl_string(&format!("'{name}'"));
                        let cwd = bsl_string(root.to_str().unwrap());
                        let call = if is_async {
                            format!("Р = {asynchronous}({command}, {cwd}, Ложь);")
                        } else {
                            format!("{sync}({command}, {cwd}, Ложь, Р);")
                        };
                        let call = if dynamic {
                            format!("Выполнить({});", bsl_string(&call))
                        } else {
                            call
                        };
                        let source = if is_async {
                            format!(
                                "Асинх Процедура Тест()\nР = 123; {call}\nЕсли Ждать Р <> Неопределено Тогда ВызватьИсключение \"result\"; КонецЕсли;\nКонецПроцедуры\nТест();"
                            )
                        } else {
                            format!(
                                "Р = 123; {call}\nЕсли Р <> Неопределено Тогда ВызватьИсключение \"result\"; КонецЕсли;"
                            )
                        };
                        std::fs::write(&script, source).unwrap();
                        let mut command = isolated_command(root);
                        if optimize {
                            command.arg("--optimize");
                        }
                        if bytecode {
                            let output = command
                                .arg("--emit-bytecode")
                                .arg(&script)
                                .arg(&listing)
                                .output()
                                .unwrap();
                            assert!(
                                output.status.success(),
                                "{}",
                                String::from_utf8_lossy(&output.stderr)
                            );
                            command = isolated_command(root);
                            command.arg("--run-bytecode").arg(&listing);
                        } else {
                            command.arg(&script);
                        }
                        let output = command.output().unwrap();
                        assert!(
                            output.status.success(),
                            "{}",
                            String::from_utf8_lossy(&output.stderr)
                        );
                        wait_for(&document.with_extension("received"));
                        assert_eq!(
                            std::fs::read_to_string(document.with_extension("received")).unwrap(),
                            document.to_str().unwrap()
                        );
                        assert!(
                            !document.with_extension("finished").exists(),
                            "CLI ждал завершения приложения"
                        );
                        std::fs::write(document.with_extension("continue"), b"continue").unwrap();
                        wait_for(&document.with_extension("finished"));
                    }
                }
            }
        }
    }
    let unobserved = root.join("unobserved.txt");
    std::fs::write(&unobserved, b"owned test document\n").unwrap();
    std::fs::write(
        &script,
        format!("RunAppAsync({});", bsl_string(unobserved.to_str().unwrap())),
    )
    .unwrap();
    let output = isolated_command(root).arg(&script).output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    wait_for(&unobserved.with_extension("received"));
    std::fs::write(unobserved.with_extension("continue"), b"continue").unwrap();
    wait_for(&unobserved.with_extension("finished"));

    let document = root.join("must-not-open.txt");
    std::fs::write(&document, b"owned test document\n").unwrap();
    std::fs::write(&script, format!(
        "Код = 123; ОшибкаБыла = Ложь; Попытка RunApp({},, Истина, Код); Исключение ОшибкаБыла = Истина; КонецПопытки; Если Не ОшибкаБыла Или Код <> 123 Тогда ВызватьИсключение \"wait\"; КонецЕсли;",
        bsl_string(document.to_str().unwrap())
    )).unwrap();
    let output = isolated_command(root).arg(&script).output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!document.with_extension("received").exists());

    // Обычный executable по-прежнему исполняется напрямую и возвращает exit status.
    std::fs::write(root.join("launch-test-marker"), b"open-bsl-child-17").unwrap();
    std::fs::write(&script, format!(
        "Код = 123; RunApp({}, {}, Истина, Код); Если Код <> 17 Тогда ВызватьИсключение \"executable\"; КонецЕсли;",
        bsl_string(&format!("'{}' --exact native_child --nocapture", handler.display())),
        bsl_string(root.to_str().unwrap())
    )).unwrap();
    let output = isolated_command(root).arg(&script).output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Удаляется только собственная ассоциация внутри scratch, не настройка пользователя.
    std::fs::remove_file(root.join("data/applications/open-bsl-test.desktop")).unwrap();
    for missing_gio in [false, true] {
        std::fs::write(&script, format!(
            "Код = 123; ОшибкаБыла = Ложь; Попытка RunApp({},, Ложь, Код); Исключение ОшибкаБыла = Истина; КонецПопытки; Если Не ОшибкаБыла Или Код <> 123 Тогда ВызватьИсключение \"association failure\"; КонецЕсли;",
            bsl_string(document.to_str().unwrap())
        )).unwrap();
        let mut command = isolated_command(root);
        if missing_gio {
            command.env("PATH", root.join("empty"));
        }
        let output = command.arg(&script).output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(!document.with_extension("received").exists());
    }
}
