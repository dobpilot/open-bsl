use open_bsl::{
    DirEntry, Engine, ExecutionPoll, FileHandle, FileMetadata, FileOpenOptions, FileSystem,
};
use std::{
    io::{self, Write},
    rc::Rc,
    sync::{Arc, Mutex, mpsc},
    time::{Duration, Instant},
};

#[derive(Clone, Default)]
struct Capture(Rc<Mutex<Vec<u8>>>);
impl Write for Capture {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[derive(Clone, Debug)]
struct Files {
    entered: mpsc::Sender<()>,
    release: Arc<Mutex<mpsc::Receiver<()>>>,
    finished: mpsc::Sender<()>,
}
impl FileSystem for Files {
    fn path_separator(&self) -> io::Result<String> {
        Ok("/".into())
    }
    fn background_access(&self) -> Option<Arc<dyn FileSystem + Send + Sync>> {
        Some(Arc::new(self.clone()))
    }
    fn metadata(&self, path: &str) -> io::Result<FileMetadata> {
        assert_eq!(path, "/tmp/open-bsl-error-description-absent-probe-1007");
        self.entered.send(()).unwrap();
        self.release
            .lock()
            .unwrap()
            .recv_timeout(Duration::from_secs(5))
            .unwrap();
        self.finished.send(()).unwrap();
        Err(io::ErrorKind::NotFound.into())
    }
    fn read(&self, _: &str) -> io::Result<Vec<u8>> {
        panic!("контроль не читает содержимое")
    }
    fn write(&self, _: &str, _: &[u8]) -> io::Result<()> {
        panic!("контроль не записывает файлы")
    }
    fn read_dir<'a>(
        &'a self,
        _: &str,
    ) -> io::Result<Box<dyn Iterator<Item = io::Result<DirEntry>> + 'a>> {
        panic!("контроль не перечисляет каталоги")
    }
    fn create_dir_all(&self, _: &str) -> io::Result<()> {
        panic!("контроль не создаёт каталоги")
    }
    fn open(&self, _: &str, _: FileOpenOptions) -> io::Result<Box<dyn FileHandle>> {
        panic!("контроль не открывает файлы")
    }
}

#[test]
fn handler_descriptions_survive_two_proven_pending_file_operations() {
    let engine = Engine::builder().build().unwrap();
    let module = engine
        .compile(include_str!(
            "../../../docs/research/fixtures/error-description-context/async.bsl"
        ))
        .unwrap();
    let (entered_tx, entered) = mpsc::channel();
    let (release, release_rx) = mpsc::channel();
    let (finished_tx, finished) = mpsc::channel();
    let files = Files {
        entered: entered_tx,
        release: Arc::new(Mutex::new(release_rx)),
        finished: finished_tx,
    };
    let output = Capture::default();
    let mut state = engine
        .state_builder()
        .files(files)
        .stdout(output.clone())
        .build();
    let mut execution = state.start(&module).unwrap();
    for _ in 0..2 {
        let poll = execution.poll(4096).unwrap();
        assert!(
            matches!(poll, ExecutionPoll::Waiting),
            "{poll:?}: {:?}",
            output.0.lock().unwrap()
        );
        entered.recv_timeout(Duration::from_secs(5)).unwrap();
        // Рабочий поток удерживается до разрешения хоста: ожидание не
        // зависит от скорости файловой системы или планирования потоков.
        assert!(matches!(execution.poll(1).unwrap(), ExecutionPoll::Waiting));
        release.send(()).unwrap();
        finished.recv_timeout(Duration::from_secs(5)).unwrap();
    }
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if matches!(execution.poll(4096).unwrap(), ExecutionPoll::Complete(_)) {
            break;
        }
        assert!(Instant::now() < deadline, "выполнение не завершилось");
        std::thread::yield_now();
    }
    assert_eq!(
        *output.0.lock().unwrap(),
        include_bytes!(
            "../../../docs/research/fixtures/error-description-context/async.platform.txt"
        )
    );
}
