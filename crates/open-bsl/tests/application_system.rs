#![cfg(target_os = "linux")]

use open_bsl::{
    ApplicationCompletionSink, ApplicationExit, ApplicationLauncher, ApplicationRequest,
    ApplicationResult, Engine, JobStateDto, StateBuilder, SystemApplicationLauncher,
    jobs::{BackgroundJobConfig, BackgroundStateFactory},
};
use std::{
    io,
    sync::{Arc, mpsc},
    time::{Duration, Instant},
};

#[path = "../../../tests/support/application_system.rs"]
mod application_system;

#[test]
fn native_child() {
    application_system::native_child();
}

#[derive(Clone, Debug)]
struct ObservedLauncher(mpsc::Sender<Result<ApplicationResult, io::ErrorKind>>);

struct ObservedSink {
    sink: Box<dyn ApplicationCompletionSink>,
    finished: mpsc::Sender<Result<ApplicationResult, io::ErrorKind>>,
}

impl ApplicationCompletionSink for ObservedSink {
    fn complete(self: Box<Self>, result: io::Result<ApplicationResult>) {
        let observed = result.as_ref().copied().map_err(io::Error::kind);
        self.sink.complete(result);
        let _ = self.finished.send(observed);
    }
}

impl ApplicationLauncher for ObservedLauncher {
    fn submit(
        &self,
        request: ApplicationRequest,
        sink: Box<dyn ApplicationCompletionSink>,
    ) -> io::Result<()> {
        SystemApplicationLauncher.submit(
            request,
            Box::new(ObservedSink {
                sink,
                finished: self.0.clone(),
            }),
        )
    }
}

struct Profile(ObservedLauncher);
impl BackgroundStateFactory for Profile {
    fn configure(&self, builder: StateBuilder) -> Result<StateBuilder, String> {
        Ok(builder.application_launcher(self.0.clone()))
    }
}

fn bsl_string(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

#[test]
fn real_application_survives_job_cancellation_and_runtime_shutdown_and_is_reaped() {
    check_application_lifetime(false);
}

#[test]
fn async_application_survives_job_cancellation_and_runtime_shutdown_and_is_reaped() {
    check_application_lifetime(true);
}

fn check_application_lifetime(asynchronous: bool) {
    for (action, depth) in ["complete", "cancel", "shutdown"]
        .into_iter()
        .flat_map(|action| [0, 3].map(move |depth| (action, depth)))
    {
        let scratch = application_system::scratch();
        std::fs::write(scratch.0.join("launch-test-marker"), b"open-bsl-child-wait").unwrap();
        let command = format!(
            "'{}' --exact native_child --nocapture",
            std::env::current_exe()
                .unwrap()
                .to_str()
                .unwrap()
                .replace('\'', "'\\''")
        );
        let mut call = format!(
            "{}({}, {}, Истина{});",
            if asynchronous {
                "П = RunAppAsync"
            } else {
                "RunApp"
            },
            bsl_string(&command),
            bsl_string(scratch.0.to_str().unwrap()),
            if asynchronous { "" } else { ", Код" },
        );
        for _ in 0..depth {
            call = format!("Выполнить({});", bsl_string(&call));
        }
        if asynchronous {
            call.push_str(" Код = Ждать П;");
        }
        let long_job = if asynchronous {
            format!(
                "Асинх Процедура Дождаться()\nП = Неопределено;\n{call}\nЕсли Код <> 17 Тогда ВызватьИсключение \"exit\"; КонецЕсли;\nКонецПроцедуры\nПроцедура Длинная() Экспорт\nДождаться();\nКонецПроцедуры"
            )
        } else {
            format!(
                "Процедура Длинная() Экспорт\nКод = 12345;\n{call}\nЕсли Код <> 17 Тогда ВызватьИсключение \"exit\"; КонецЕсли;\nКонецПроцедуры"
            )
        };
        let (finished, completed) = mpsc::channel();
        let mut builder = Engine::builder()
            .background_jobs(BackgroundJobConfig {
                workers: Some(1),
                ..Default::default()
            })
            .common_module(
                "Работы",
                &format!(
                    r#"
{long_job}
Процедура Короткая() Экспорт
КонецПроцедуры
"#
                ),
            );
        let profile = builder.register_host_profile(Arc::new(Profile(ObservedLauncher(finished))));
        let engine = builder.build().unwrap();
        let runtime = engine.job_runtime().unwrap();
        let mut state = engine
            .state_builder()
            .host_profile(profile)
            .unwrap()
            .build();
        state
            .run(
                &engine
                    .compile_entry("ФоновыеЗадания.Выполнить(\"Работы.Длинная\");")
                    .unwrap(),
            )
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        while !scratch.0.join("child-ready").exists() {
            assert!(
                Instant::now() < deadline,
                "дочерняя программа не начала работу"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
        let pid: u32 = std::fs::read_to_string(scratch.0.join("child-ready"))
            .unwrap()
            .parse()
            .unwrap();
        let slow = runtime
            .snapshots()
            .into_iter()
            .find(|job| job.method_name == "Работы.Длинная")
            .unwrap();
        state
            .run(
                &engine
                    .compile_entry("ФоновыеЗадания.Выполнить(\"Работы.Короткая\");")
                    .unwrap(),
            )
            .unwrap();
        let quick = runtime
            .snapshots()
            .into_iter()
            .find(|job| job.method_name == "Работы.Короткая")
            .unwrap();
        let quick_done = runtime
            .wait_terminal(&[quick.id], Some(Duration::from_secs(2)))
            .unwrap();
        let detached = if action == "shutdown" {
            runtime.shutdown(Duration::from_secs(2)).detached_workers
        } else {
            0
        };
        if action == "cancel" {
            runtime.cancel(slow.id).unwrap();
            runtime
                .wait_terminal(&[slow.id], Some(Duration::from_secs(2)))
                .unwrap();
        }
        let waiting_state = runtime.snapshot(slow.id).unwrap().state;
        let child_alive = std::path::Path::new(&format!("/proc/{pid}")).exists();
        let before_release = completed.try_recv();
        let not_completed = matches!(&before_release, Err(mpsc::TryRecvError::Empty));
        // Разрешаем процессу завершиться до утверждений, чтобы даже ошибка
        // теста не оставила его ждать таймаута. Sink вызывается после wait/reap.
        std::fs::write(scratch.0.join("child-continue"), b"continue").unwrap();
        let exit = match before_release {
            Ok(result) => result,
            Err(_) => completed.recv_timeout(Duration::from_secs(5)).unwrap(),
        }
        .unwrap();
        if action != "shutdown" {
            assert!(
                runtime
                    .wait_terminal(&[slow.id], Some(Duration::from_secs(5)))
                    .unwrap()
            );
        }
        assert!(
            quick_done,
            "worker занят внешним процессом: {action}, depth={depth}"
        );
        assert_eq!(detached, 0);
        assert_eq!(
            runtime.snapshot(quick.id).unwrap().state,
            JobStateDto::Completed
        );
        assert_eq!(
            waiting_state,
            if action == "complete" {
                JobStateDto::Running
            } else {
                JobStateDto::Canceled
            }
        );
        assert!(
            child_alive && not_completed,
            "внешний процесс завершён вместе с ожидателем"
        );
        assert_eq!(
            exit,
            ApplicationResult::Exited(ApplicationExit { code: Some(17) })
        );
        assert!(
            !std::path::Path::new(&format!("/proc/{pid}")).exists(),
            "процесс не собран"
        );
        assert_eq!(
            std::fs::read(scratch.0.join("child-completed")).unwrap(),
            b"continued after waiter drop"
        );
        assert_eq!(
            runtime.snapshot(slow.id).unwrap().state,
            if action == "complete" {
                JobStateDto::Completed
            } else {
                JobStateDto::Canceled
            }
        );
    }
}
