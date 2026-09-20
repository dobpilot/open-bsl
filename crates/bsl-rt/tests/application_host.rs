use bsl_rt::{
    ApplicationCompletionSink, ApplicationExit, ApplicationLauncher, ApplicationRequest,
    ApplicationResult, ApplicationTarget, HostEnv, SecretString,
};
use std::sync::{Arc, Mutex, mpsc};

#[derive(Debug, Default)]
struct FakeLauncher(Arc<Mutex<Vec<ApplicationRequest>>>);
impl ApplicationLauncher for FakeLauncher {
    fn submit(
        &self,
        request: ApplicationRequest,
        sink: Box<dyn ApplicationCompletionSink>,
    ) -> std::io::Result<()> {
        self.0.lock().unwrap().push(request);
        sink.complete(Ok(ApplicationResult::Exited(ApplicationExit {
            code: Some(17),
        })));
        Ok(())
    }
}
struct Sink(mpsc::Sender<std::io::Result<ApplicationResult>>);
impl ApplicationCompletionSink for Sink {
    fn complete(self: Box<Self>, result: std::io::Result<ApplicationResult>) {
        self.0.send(result).unwrap();
    }
}

#[test]
fn application_launch_is_opt_in_and_preserves_structured_arguments() {
    assert!(HostEnv::process().application_launcher().is_none());
    let requests = Arc::new(Mutex::new(Vec::new()));
    let env = HostEnv::process().with_application_launcher(FakeLauncher(requests.clone()));
    let untouched = HostEnv::process();
    let request = ApplicationRequest {
        target: ApplicationTarget::Executable {
            program: "virtual/program".into(),
            arguments: vec![
                SecretString::new("a b"),
                SecretString::new("$(not-a-shell)"),
                SecretString::new("test-secret-token"),
            ],
        },
        wait_for_exit: true,
        working_directory: Some("virtual/work".into()),
    };
    assert!(!format!("{request:?}").contains("test-secret-token"));
    let (sender, receiver) = mpsc::channel();
    env.application_launcher()
        .unwrap()
        .submit(request.clone(), Box::new(Sink(sender)))
        .unwrap();
    assert_eq!(
        receiver.recv().unwrap().unwrap(),
        ApplicationResult::Exited(ApplicationExit { code: Some(17) })
    );
    assert_eq!(&*requests.lock().unwrap(), &[request]);
    assert!(untouched.application_launcher().is_none());
}

#[test]
fn accepted_completion_outlives_the_environment_without_cancellation() {
    type Pending = Arc<Mutex<Option<Box<dyn ApplicationCompletionSink>>>>;
    struct Deferred(Pending);
    impl std::fmt::Debug for Deferred {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("Deferred")
        }
    }
    impl ApplicationLauncher for Deferred {
        fn submit(
            &self,
            request: ApplicationRequest,
            sink: Box<dyn ApplicationCompletionSink>,
        ) -> std::io::Result<()> {
            assert!(matches!(request.target, ApplicationTarget::Document { .. }));
            *self.0.lock().unwrap() = Some(sink);
            Ok(())
        }
    }
    let pending: Pending = Arc::new(Mutex::new(None));
    let env = HostEnv::process().with_application_launcher(Deferred(pending.clone()));
    let (sender, receiver) = mpsc::channel();
    env.application_launcher()
        .unwrap()
        .submit(
            ApplicationRequest {
                target: ApplicationTarget::Document {
                    path: "virtual/document.txt".into(),
                },
                wait_for_exit: true,
                working_directory: None,
            },
            Box::new(Sink(sender)),
        )
        .unwrap();
    assert!(matches!(
        receiver.try_recv(),
        Err(mpsc::TryRecvError::Empty)
    ));
    drop(env);
    pending
        .lock()
        .unwrap()
        .take()
        .unwrap()
        .complete(Ok(ApplicationResult::Exited(ApplicationExit {
            code: None,
        })));
    assert_eq!(
        receiver.recv().unwrap().unwrap(),
        ApplicationResult::Exited(ApplicationExit { code: None })
    );
}
