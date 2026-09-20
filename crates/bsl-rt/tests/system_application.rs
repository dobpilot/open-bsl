use bsl_rt::{
    ApplicationCompletionSink, ApplicationExit, ApplicationLauncher, ApplicationRequest,
    ApplicationResult, ApplicationTarget, SecretString, SystemApplicationLauncher,
};
use std::{sync::mpsc, time::Duration};

#[path = "../../../tests/support/application_system.rs"]
mod application_system;
use application_system::scratch;

fn child_executable(scratch: &application_system::Scratch) -> std::path::PathBuf {
    let executable = scratch.0.join("child $(not-a-shell).exe");
    // Копия иногда получает ETXTBSY при параллельном запуске тестов.
    // Ссылка не открывает будущий executable для записи; имя с пробелами
    // и shell-символами по-прежнему проходит через настоящий launcher.
    #[cfg(unix)]
    std::os::unix::fs::symlink(std::env::current_exe().unwrap(), &executable).unwrap();
    #[cfg(not(unix))]
    std::fs::copy(std::env::current_exe().unwrap(), &executable).unwrap();
    executable
}

struct Sink(mpsc::Sender<std::io::Result<ApplicationResult>>);
impl ApplicationCompletionSink for Sink {
    fn complete(self: Box<Self>, result: std::io::Result<ApplicationResult>) {
        let _ = self.0.send(result);
    }
}
#[test]
fn native_child() {
    application_system::native_child();
}

#[cfg(target_os = "linux")]
#[test]
fn documents_deliver_open_errors_without_claiming_process_exit() {
    let scratch = scratch();
    let (sender, receiver) = mpsc::channel();
    SystemApplicationLauncher
        .submit(
            ApplicationRequest {
                target: ApplicationTarget::Document {
                    path: scratch.0.join("missing.txt").to_str().unwrap().into(),
                },
                wait_for_exit: false,
                working_directory: None,
            },
            Box::new(Sink(sender)),
        )
        .unwrap();
    assert!(
        receiver
            .recv_timeout(Duration::from_secs(10))
            .unwrap()
            .is_err()
    );
}

#[test]
fn system_launcher_preserves_argv_cwd_and_reports_exit_and_spawn_errors() {
    let scratch = scratch();
    let executable = child_executable(&scratch);
    std::fs::write(scratch.0.join("launch-test-marker"), b"open-bsl-child-17").unwrap();
    let arguments = [
        "--exact",
        "native_child",
        "--nocapture",
        "--skip",
        "literal $(not-a-shell) and \"quotes\"",
    ];
    let (sender, receiver) = mpsc::channel();
    SystemApplicationLauncher
        .submit(
            ApplicationRequest {
                target: ApplicationTarget::Executable {
                    program: executable.to_str().unwrap().into(),
                    arguments: arguments
                        .iter()
                        .map(|arg| SecretString::new(*arg))
                        .collect(),
                },
                wait_for_exit: true,
                working_directory: Some(scratch.0.to_str().unwrap().into()),
            },
            Box::new(Sink(sender)),
        )
        .unwrap();
    assert_eq!(
        receiver
            .recv_timeout(Duration::from_secs(10))
            .unwrap()
            .unwrap(),
        ApplicationResult::Exited(ApplicationExit { code: Some(17) })
    );
    assert_eq!(
        std::fs::read_to_string(scratch.0.join("received-arguments")).unwrap(),
        arguments.join("\n")
    );
    let (sender, receiver) = mpsc::channel();
    SystemApplicationLauncher
        .submit(
            ApplicationRequest {
                target: ApplicationTarget::Executable {
                    program: scratch.0.join("missing").to_str().unwrap().into(),
                    arguments: Vec::new(),
                },
                wait_for_exit: true,
                working_directory: None,
            },
            Box::new(Sink(sender)),
        )
        .unwrap();
    assert_eq!(
        receiver
            .recv_timeout(Duration::from_secs(10))
            .unwrap()
            .unwrap_err()
            .kind(),
        std::io::ErrorKind::NotFound
    );
}

#[cfg(target_os = "linux")]
#[test]
fn parsed_unix_command_reaches_the_real_child_without_expansion() {
    let scratch = scratch();
    let executable = child_executable(&scratch);
    std::fs::write(scratch.0.join("launch-test-marker"), b"open-bsl-child-17").unwrap();
    let command = format!(
        "'{}' --exact native_child --nocapture --skip 'literal $NAME * and \"quotes\"'",
        executable.to_str().unwrap().replace('\'', "'\\''"),
    );
    let (sender, receiver) = mpsc::channel();
    SystemApplicationLauncher
        .submit(
            ApplicationRequest {
                target: ApplicationTarget::from_unix_command_line(&command).unwrap(),
                wait_for_exit: true,
                working_directory: Some(scratch.0.to_str().unwrap().into()),
            },
            Box::new(Sink(sender)),
        )
        .unwrap();
    assert_eq!(
        receiver
            .recv_timeout(Duration::from_secs(10))
            .unwrap()
            .unwrap(),
        ApplicationResult::Exited(ApplicationExit { code: Some(17) })
    );
    assert_eq!(
        std::fs::read_to_string(scratch.0.join("received-arguments")).unwrap(),
        [
            "--exact",
            "native_child",
            "--nocapture",
            "--skip",
            "literal $NAME * and \"quotes\""
        ]
        .join("\n"),
    );
}

#[cfg(target_os = "linux")]
#[test]
fn losing_the_result_receiver_does_not_kill_or_leave_a_zombie() {
    let scratch = scratch();
    std::fs::write(scratch.0.join("launch-test-marker"), b"open-bsl-child-wait").unwrap();
    let (sender, receiver) = mpsc::channel();
    SystemApplicationLauncher
        .submit(
            ApplicationRequest {
                target: ApplicationTarget::Executable {
                    program: std::env::current_exe().unwrap().to_str().unwrap().into(),
                    arguments: ["--exact", "native_child", "--nocapture"]
                        .into_iter()
                        .map(SecretString::new)
                        .collect(),
                },
                wait_for_exit: true,
                working_directory: Some(scratch.0.to_str().unwrap().into()),
            },
            Box::new(Sink(sender)),
        )
        .unwrap();
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while !scratch.0.join("child-ready").exists() {
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(5));
    }
    let pid: u32 = std::fs::read_to_string(scratch.0.join("child-ready"))
        .unwrap()
        .parse()
        .unwrap();
    drop(receiver);
    std::fs::write(scratch.0.join("child-continue"), b"continue").unwrap();
    while std::path::Path::new(&format!("/proc/{pid}")).exists() {
        assert!(
            std::time::Instant::now() < deadline,
            "дочерний процесс остался живым или zombie"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(
        std::fs::read(scratch.0.join("child-completed")).unwrap(),
        b"continued after waiter drop"
    );
}
