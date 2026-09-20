use open_bsl::{DirEntry, Engine, ExecutionPoll, FileMetadata, FileSystem};
use std::{
    io,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};

#[derive(Clone, Debug)]
struct Files {
    enabled: bool,
    panic_on_read: bool,
    block_metadata: bool,
    block_update: bool,
    updates: Arc<Mutex<Vec<open_bsl::FileMetadataUpdate>>>,
    separators: Arc<AtomicUsize>,
    entered: mpsc::Sender<()>,
    release: Arc<Mutex<mpsc::Receiver<()>>>,
    finished: mpsc::Sender<()>,
    next_calls: Arc<AtomicUsize>,
}

struct Entries {
    next_calls: Arc<AtomicUsize>,
    finished: mpsc::Sender<()>,
    yielded: bool,
}

impl Files {
    fn record_update(&self, path: &str, update: open_bsl::FileMetadataUpdate) -> io::Result<()> {
        assert_eq!(path, "virtual/a.txt");
        if self.block_update {
            self.entered.send(()).unwrap();
            self.release
                .lock()
                .unwrap()
                .recv_timeout(Duration::from_secs(5))
                .unwrap();
        }
        self.updates.lock().unwrap().push(update);
        self.finished.send(()).unwrap();
        Ok(())
    }
}
impl Iterator for Entries {
    type Item = io::Result<DirEntry>;
    fn next(&mut self) -> Option<Self::Item> {
        self.next_calls.fetch_add(1, Ordering::Relaxed);
        if std::mem::replace(&mut self.yielded, true) {
            None
        } else {
            Some(Ok(DirEntry::new("a.txt", false)))
        }
    }
}
impl Drop for Entries {
    fn drop(&mut self) {
        let _ = self.finished.send(());
    }
}
impl FileSystem for Files {
    fn set_read_only(&self, path: &str, value: bool) -> io::Result<()> {
        self.record_update(path, open_bsl::FileMetadataUpdate::ReadOnly(value))
    }
    fn set_hidden(&self, path: &str, value: bool) -> io::Result<()> {
        self.record_update(path, open_bsl::FileMetadataUpdate::Hidden(value))
    }
    fn set_modified(&self, path: &str, value: i64) -> io::Result<()> {
        self.record_update(path, open_bsl::FileMetadataUpdate::Modified(value))
    }
    fn background_access(&self) -> Option<Arc<dyn FileSystem + Send + Sync>> {
        self.enabled
            .then(|| Arc::new(self.clone()) as Arc<dyn FileSystem + Send + Sync>)
    }
    fn read(&self, _: &str) -> io::Result<Vec<u8>> {
        panic!("поиск не читает файл")
    }
    fn write(&self, _: &str, _: &[u8]) -> io::Result<()> {
        panic!("поиск не пишет файл")
    }
    fn create_dir_all(&self, _: &str) -> io::Result<()> {
        panic!("поиск не создаёт каталог")
    }
    fn open(
        &self,
        _: &str,
        _: open_bsl::FileOpenOptions,
    ) -> io::Result<Box<dyn open_bsl::FileHandle>> {
        panic!("поиск не открывает файл")
    }
    fn metadata(&self, path: &str) -> io::Result<FileMetadata> {
        assert_eq!(path, "virtual/a.txt");
        if self.block_metadata {
            self.entered.send(()).unwrap();
            self.release
                .lock()
                .unwrap()
                .recv_timeout(Duration::from_secs(5))
                .unwrap();
            self.finished.send(()).unwrap();
        }
        Ok(FileMetadata::file(Some(1_577_934_245))
            .with_size(7)
            .with_read_only(false)
            .with_hidden(true))
    }
    fn path_separator(&self) -> io::Result<String> {
        self.separators.fetch_add(1, Ordering::Relaxed);
        Ok("/".into())
    }
    fn read_dir<'a>(
        &'a self,
        path: &str,
    ) -> io::Result<Box<dyn Iterator<Item = io::Result<DirEntry>> + 'a>> {
        assert!(!self.panic_on_read, "test host panic");
        assert_eq!(path, "virtual");
        self.entered.send(()).unwrap();
        self.release
            .lock()
            .unwrap()
            .recv_timeout(Duration::from_secs(5))
            .unwrap();
        Ok(Box::new(Entries {
            next_calls: self.next_calls.clone(),
            finished: self.finished.clone(),
            yielded: false,
        }))
    }
}

fn host(
    enabled: bool,
) -> (
    Files,
    mpsc::Receiver<()>,
    mpsc::Sender<()>,
    mpsc::Receiver<()>,
) {
    let (entered, started) = mpsc::channel();
    let (release, waiting) = mpsc::channel();
    let (finished, stopped) = mpsc::channel();
    (
        Files {
            enabled,
            panic_on_read: false,
            block_metadata: false,
            block_update: false,
            updates: Arc::new(Mutex::new(Vec::new())),
            separators: Arc::new(AtomicUsize::new(0)),
            entered,
            release: Arc::new(Mutex::new(waiting)),
            finished,
            next_calls: Arc::new(AtomicUsize::new(0)),
        },
        started,
        release,
        stopped,
    )
}

const SEARCH: &str = r#"
Асинх Процедура Проверить()
    ОбещаниеПоиска = FindFilesAsync("virtual", "*.txt");
    Если Строка(ТипЗнч(ОбещаниеПоиска)) <> "Обещание" Тогда ВызватьИсключение "нет обещания"; КонецЕсли;
    Найденные = Ждать ОбещаниеПоиска;
    Если Найденные.Количество() <> 1 Тогда ВызватьИсключение "неверное количество"; КонецЕсли;
    Если Найденные[0].Имя <> "a.txt" Или Найденные[0].Размер() <> 7 Тогда ВызватьИсключение "потерян host"; КонецЕсли;
    Снова = Ждать ОбещаниеПоиска;
    Если Снова.Количество() <> 1 Тогда ВызватьИсключение "повторное ожидание"; КонецЕсли;
КонецПроцедуры
Проверить();
"#;

struct FilePromiseProbe {
    files: std::rc::Rc<dyn FileSystem>,
    zone: std::rc::Rc<dyn open_bsl::TimeZone>,
}

impl std::fmt::Debug for FilePromiseProbe {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FilePromiseProbe").finish_non_exhaustive()
    }
}

static PROBE_TYPE: open_bsl::TypeDescriptor =
    open_bsl::TypeDescriptor::new("file-promise-probe", "FilePromiseProbe");

impl open_bsl::ObjectProtocol for FilePromiseProbe {
    fn type_descriptor(&self) -> &'static open_bsl::TypeDescriptor {
        &PROBE_TYPE
    }

    fn method_table(&self) -> &'static [open_bsl::MethodDescriptor] {
        const METHODS: &[open_bsl::MethodDescriptor] = &[open_bsl::MethodDescriptor::new(
            &["Search"],
            open_bsl::Arity::exact(1),
            probe_search,
        )];
        METHODS
    }
}

fn probe_search(
    receiver: &dyn open_bsl::ObjectProtocol,
    args: &[open_bsl::Value],
    ctx: &mut open_bsl::CallContext<'_>,
) -> open_bsl::RtResult<open_bsl::Value> {
    let probe = bsl_rt::receiver_of::<FilePromiseProbe>(receiver, "Search")?;
    let request = match &args[0] {
        open_bsl::Value::Str(path) => Ok(open_bsl::FileOperationRequest::Search(
            open_bsl::FileSearchRequest {
                path: path.to_string(),
                mask: Some("*.txt".into()),
                recursive: false,
            },
        )),
        _ => Err(open_bsl::RtError::TypeError {
            expected: "Строка",
            op: "Search",
        }),
    };
    ctx.spawn_file_operation(request, probe.files.clone(), probe.zone.clone())
}

fn construct_probe(
    ctx: &mut open_bsl::CallContext<'_>,
    _: &[open_bsl::Value],
) -> open_bsl::RtResult<open_bsl::Value> {
    Ok(open_bsl::Value::new_object(FilePromiseProbe {
        files: ctx.files_rc()?,
        zone: ctx.zone_rc()?,
    }))
}

#[test]
fn component_file_promises_survive_direct_and_dynamic_calls() {
    const TYPES: &[&open_bsl::TypeDescriptor] = &[&PROBE_TYPE];
    const CONSTRUCTORS: &[open_bsl::ConstructorDescriptor] = &[open_bsl::ConstructorDescriptor {
        code: open_bsl::ConstructorCode::new(1),
        names: &["FilePromiseProbe"],
        arity: open_bsl::Arity::exact(0),
        call: construct_probe,
    }];
    let library = open_bsl::LibraryDescriptor::new("file-promise-probe", "1.0.0")
        .with_types(TYPES)
        .with_constructors(CONSTRUCTORS);
    let engine = Engine::builder().register_library(library).build().unwrap();
    for expression in [
        "Проба.Search(ПутьПробы)",
        "Вычислить(\"Проба.Search(ПутьПробы)\")",
        "Вычислить(\"Вычислить(\"\"Проба.Search(ПутьПробы)\"\")\")",
    ] {
        let source = format!(
            r#"
Асинх Процедура Проверить()
    Проба = Новый FilePromiseProbe();
    ПутьПробы = "virtual";
    ОбещаниеПоиска = {expression};
    Найденные = Ждать ОбещаниеПоиска;
    Если Найденные.Количество() <> 1 Или Найденные[0].Размер() <> 7 Тогда ВызватьИсключение "host"; КонецЕсли;
    Снова = Ждать ОбещаниеПоиска;
    Если Снова.Количество() <> 1 Тогда ВызватьИсключение "repeat"; КонецЕсли;
    ПутьПробы = Неопределено;
    ОбещаниеОшибки = {expression};
    ОшибкаПоймана = Ложь;
    Попытка
        РезультатОшибки = Ждать ОбещаниеОшибки;
    Исключение
        Если ИнформацияОбОшибке().Описание <> "ожидался тип «Строка» для операции «Search»" Тогда ВызватьИсключение; КонецЕсли;
        ОшибкаПоймана = Истина;
    КонецПопытки;
    Если Не ОшибкаПоймана Тогда ВызватьИсключение "no error"; КонецЕсли;
КонецПроцедуры
Проверить();
"#
        );
        let module = engine.compile(&source).unwrap();
        let (files, entered, release, finished) = host(true);
        let mut state = engine.state_builder().files(files).build();
        let mut execution = state.start(&module).unwrap();
        assert!(matches!(
            execution.poll(128).unwrap(),
            ExecutionPoll::Waiting
        ));
        entered.recv_timeout(Duration::from_secs(5)).unwrap();
        release.send(()).unwrap();
        finished.recv_timeout(Duration::from_secs(5)).unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            match execution.poll(128).unwrap() {
                ExecutionPoll::Complete(_) => break,
                _ => assert!(
                    Instant::now() < deadline,
                    "component promise did not finish"
                ),
            }
            std::thread::yield_now();
        }
    }
}

#[test]
fn search_waits_without_blocking_the_driver_and_keeps_host_objects() {
    let engine = Engine::builder().build().unwrap();
    let module = engine.compile(SEARCH).unwrap();
    let (files, entered, release, finished) = host(true);
    let mut state = engine.state_builder().files(files).build();
    let mut execution = state.start(&module).unwrap();
    assert!(matches!(
        execution.poll(32).unwrap(),
        ExecutionPoll::Waiting
    ));
    entered.recv_timeout(Duration::from_secs(5)).unwrap();
    assert_eq!(
        engine.new_state().exec("Возврат Истина;").unwrap(),
        open_bsl::Value::Boolean(true)
    );
    release.send(()).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if matches!(execution.poll(32).unwrap(), ExecutionPoll::Complete(_)) {
            break;
        }
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(1));
    }
    finished.recv_timeout(Duration::from_secs(5)).unwrap();
}

#[test]
fn file_metadata_promises_are_nonblocking_and_keep_the_objects_zone() {
    let engine = Engine::builder().build().unwrap();
    for (ru, en, expected) in [
        ("СуществуетАсинх", "ExistsAsync", "Истина"),
        ("ЭтоФайлАсинх", "IsFileAsync", "Истина"),
        ("ЭтоКаталогАсинх", "IsDirectoryAsync", "Ложь"),
        ("РазмерАсинх", "SizeAsync", "7"),
        ("ПолучитьТолькоЧтениеАсинх", "GetReadOnlyAsync", "Ложь"),
        ("ПолучитьНевидимостьАсинх", "GetHiddenAsync", "Истина"),
        (
            "ПолучитьВремяИзмененияАсинх",
            "GetModificationTimeAsync",
            "Дата(2020, 1, 2, 6, 4, 5)",
        ),
        (
            "ПолучитьУниверсальноеВремяИзмененияАсинх",
            "GetModificationUniversalTimeAsync",
            "Дата(2020, 1, 2, 3, 4, 5)",
        ),
    ] {
        for name in [ru, en] {
            for dynamic in [false, true] {
                let call = format!("ФайлПробы.{name}()");
                let expression = if dynamic {
                    format!("Вычислить(\"{call}\")")
                } else {
                    call
                };
                let source = format!(
                    r#"
Асинх Процедура Проверить()
    ФайлПробы = Новый Файл("virtual/a.txt");
    ОбещаниеПробы = {expression};
    ОбещаниеПеременыПути = ФайлПробы.InitializeAsync("changed/path");
    Если Строка(ТипЗнч(ОбещаниеПробы)) <> "Обещание" Тогда ВызватьИсключение "promise"; КонецЕсли;
    РезультатПробы = Ждать ОбещаниеПробы;
    Если РезультатПробы <> {expected} Тогда ВызватьИсключение "metadata result"; КонецЕсли;
    ПовторПробы = Ждать ОбещаниеПробы;
    Если ПовторПробы <> РезультатПробы Тогда ВызватьИсключение "repeat"; КонецЕсли;
КонецПроцедуры
Проверить();
"#
                );
                let module = engine.compile(&source).unwrap();
                let (mut files, entered, release, finished) = host(true);
                files.block_metadata = true;
                let mut state = engine
                    .state_builder()
                    .files(files)
                    .zone(open_bsl::FixedTimeZone::new(3 * 3600).unwrap())
                    .build();
                let mut execution = state.start(&module).unwrap();
                assert!(matches!(
                    execution.poll(128).unwrap(),
                    ExecutionPoll::Waiting
                ));
                entered.recv_timeout(Duration::from_secs(5)).unwrap();
                release.send(()).unwrap();
                finished.recv_timeout(Duration::from_secs(5)).unwrap();
                let deadline = Instant::now() + Duration::from_secs(5);
                while !matches!(execution.poll(128).unwrap(), ExecutionPoll::Complete(_)) {
                    assert!(Instant::now() < deadline, "{name}");
                    std::thread::yield_now();
                }
            }
        }
    }
}

#[test]
fn dropping_execution_cancels_after_blocked_io_without_reading_entries() {
    let engine = Engine::builder().build().unwrap();
    let module = engine.compile(SEARCH).unwrap();
    let (files, entered, release, finished) = host(true);
    let calls = files.next_calls.clone();
    let mut state = engine.state_builder().files(files).build();
    let mut execution = state.start(&module).unwrap();
    assert!(matches!(
        execution.poll(32).unwrap(),
        ExecutionPoll::Waiting
    ));
    entered.recv_timeout(Duration::from_secs(5)).unwrap();
    drop(execution);
    release.send(()).unwrap();
    finished.recv_timeout(Duration::from_secs(5)).unwrap();
    assert_eq!(calls.load(Ordering::Relaxed), 0);
}

#[test]
fn dynamic_file_promises_belong_to_the_caller_and_do_not_block_evaluation() {
    fn evaluate(expression: &str) -> String {
        format!("Вычислить(\"{}\")", expression.replace('"', "\"\""))
    }
    let call = "FindFilesAsync(\"virtual\", \"*.txt\")";
    let assignment = format!("ОбещаниеПоиска = {call};");
    let dynamic_assignments = [
        format!("ОбещаниеПоиска = {};", evaluate(call)),
        format!("ОбещаниеПоиска = {};", evaluate(&evaluate(call))),
        format!(
            "ОбещаниеПоиска = Неопределено; Выполнить(\"{}\");",
            assignment.replace('"', "\"\"")
        ),
    ];
    for replacement in dynamic_assignments {
        let engine = Engine::builder().build().unwrap();
        let module = engine
            .compile(&SEARCH.replace(&assignment, &replacement))
            .unwrap();
        let (files, entered, release, finished) = host(true);
        let mut state = engine.state_builder().files(files).build();
        let mut execution = state.start(&module).unwrap();
        assert!(matches!(
            execution.poll(32).unwrap(),
            ExecutionPoll::Waiting
        ));
        entered.recv_timeout(Duration::from_secs(5)).unwrap();
        assert_eq!(
            engine.new_state().exec("Возврат Истина;").unwrap(),
            open_bsl::Value::Boolean(true)
        );
        release.send(()).unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if matches!(execution.poll(32).unwrap(), ExecutionPoll::Complete(_)) {
                break;
            }
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(1));
        }
        finished.recv_timeout(Duration::from_secs(5)).unwrap();
    }
}

#[test]
fn unavailable_host_and_bad_arguments_reject_promises_at_await() {
    let engine = Engine::builder().build().unwrap();
    let (files, entered, _, _) = host(false);
    let mut state = engine.state_builder().files(files).build();
    for call in [
        "НайтиФайлыАсинх(\"virtual\", \"*\")",
        "НайтиФайлыАсинх(\"virtual\", \"*\", Неопределено)",
        "СоздатьКаталогАсинх(\"virtual\")",
        "УдалитьФайлыАсинх(\"virtual\")",
        "КаталогВременныхФайловАсинх()",
        "ФайлПробы.ExistsAsync()",
        "ФайлПробы.IsFileAsync()",
        "ФайлПробы.IsDirectoryAsync()",
        "ФайлПробы.SizeAsync()",
        "ФайлПробы.GetReadOnlyAsync()",
        "ФайлПробы.GetHiddenAsync()",
        "ФайлПробы.GetModificationTimeAsync()",
        "ФайлПробы.GetModificationUniversalTimeAsync()",
        "ФайлПробы.SetReadOnlyAsync(Истина)",
        "ФайлПробы.SetHiddenAsync(Истина)",
        "ФайлПробы.SetModificationTimeAsync(Дата(2020, 1, 2))",
        "ФайлПробы.SetModificationUniversalTimeAsync(Дата(2020, 1, 2))",
    ]
    .into_iter()
    .flat_map(|call| {
        [
            call.to_owned(),
            format!("Вычислить(\"{}\")", call.replace('"', "\"\"")),
        ]
    }) {
        let script = format!(
            r#"
Асинх Процедура Проверить()
    ФайлПробы = Новый Файл("virtual/a.txt");
    ОбещаниеПоиска = {call};
    Если Строка(ТипЗнч(ОбещаниеПоиска)) <> "Обещание" Тогда ВызватьИсключение; КонецЕсли;
    БылаОшибка = Ложь;
    Попытка
        РезультатПоиска = Ждать ОбещаниеПоиска;
    Исключение
        Если СтрНайти(ИнформацияОбОшибке().Описание, "другому запуску") > 0 Тогда
            ВызватьИсключение "неверный владелец обещания";
        КонецЕсли;
        БылаОшибка = Истина;
    КонецПопытки;
    Если Не БылаОшибка Тогда ВызватьИсключение "ожидалась ошибка"; КонецЕсли;
КонецПроцедуры
Проверить();
"#
        );
        state.exec(&script).unwrap();
    }
    assert!(entered.try_recv().is_err());
}

#[test]
fn host_panic_rejects_the_promise_instead_of_leaving_it_pending() {
    let engine = Engine::builder().build().unwrap();
    let (mut files, _, _, _) = host(true);
    files.panic_on_read = true;
    let mut state = engine.state_builder().files(files).build();
    state
        .exec(
            r#"
Асинх Процедура Проверить()
    ОбещаниеПоиска = НайтиФайлыАсинх("virtual", "*");
    БылаОшибка = Ложь;
    Попытка
        РезультатПоиска = Ждать ОбещаниеПоиска;
    Исключение
        БылаОшибка = Истина;
    КонецПопытки;
    Если Не БылаОшибка Тогда ВызватьИсключение "ожидалась ошибка host"; КонецЕсли;
КонецПроцедуры
Проверить();
"#,
        )
        .unwrap();
}

#[test]
fn initialization_keeps_aliases_and_hash_keys_without_background_access() {
    let engine = Engine::builder().build().unwrap();
    for (argument, expected) in [
        ("\"\"", ""),
        ("Неопределено", ""),
        ("123", "123"),
        ("\"virtual/../b.txt/\"", "b.txt"),
    ] {
        for expression in [
            "ФайлПробы.ИнициализироватьАсинх(ЗначениеПробы)",
            "Вычислить(\"ФайлПробы.InitializeAsync(ЗначениеПробы)\")",
            "Вычислить(\"Вычислить(\"\"ФайлПробы.InitializeAsync(ЗначениеПробы)\"\")\")",
        ] {
            let source = format!(
                r#"
Асинх Процедура Проверить()
    ФайлПробы = Новый Файл("before.txt");
    ПсевдонимПробы = ФайлПробы;
    КлючиПробы = Новый Соответствие;
    КлючиПробы.Вставить(ФайлПробы, 123);
    ЗначениеПробы = {argument};
    ОбещаниеПробы = {expression};
    Если ФайлПробы.ПолноеИмя <> "{expected}" Или ПсевдонимПробы.ПолноеИмя <> "{expected}" Тогда ВызватьИсключение "deferred mutation"; КонецЕсли;
    РезультатПробы = Ждать ОбещаниеПробы;
    Если РезультатПробы <> ФайлПробы Или КлючиПробы.Получить(РезультатПробы) <> 123 Тогда ВызватьИсключение "identity"; КонецЕсли;
    НезависимыйПробы = Новый Файл(ФайлПробы.ПолноеИмя);
    Если НезависимыйПробы = ФайлПробы Тогда ВызватьИсключение "path equality"; КонецЕсли;
    ВтороеОбещаниеПробы = РезультатПробы.InitializeAsync("after.txt");
    ВторойРезультатПробы = Ждать ВтороеОбещаниеПробы;
    ПовторПробы = Ждать ОбещаниеПробы;
    Если ПовторПробы.ПолноеИмя <> "after.txt" Или ФайлПробы.ПолноеИмя <> "after.txt" Тогда ВызватьИсключение "snapshot"; КонецЕсли;
    Если КлючиПробы.Получить(ПовторПробы) <> 123 Тогда ВызватьИсключение "changed hash"; КонецЕсли;
    ОшибкаАрностиПробы = Ложь;
    Попытка
        ОшибочноеОбещаниеПробы = Вычислить("ФайлПробы.InitializeAsync()");
    Исключение
        ОшибкаАрностиПробы = Истина;
    КонецПопытки;
    Если Не ОшибкаАрностиПробы Или ФайлПробы.ПолноеИмя <> "after.txt" Тогда ВызватьИсключение "arity mutation"; КонецЕсли;
КонецПроцедуры
Проверить();
"#
            );
            let (files, entered, _, _) = host(false);
            let separators = files.separators.clone();
            let mut state = engine.state_builder().files(files).build();
            state.exec(&source).unwrap();
            assert_eq!(
                separators.load(Ordering::Relaxed),
                2,
                "только два конструктора читают разделитель"
            );
            assert!(entered.try_recv().is_err());
        }
    }
}

#[test]
#[cfg(unix)]
fn deleting_a_link_with_trailing_separators_never_deletes_its_target() {
    use std::{fs, path::PathBuf};
    struct Scratch(PathBuf);
    impl Drop for Scratch {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "open-bsl-delete-link-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir(&root).unwrap();
    let scratch = Scratch(root);
    let target = scratch.0.join("target");
    fs::create_dir(&target).unwrap();
    let sentinel = target.join("sentinel");
    fs::write(&sentinel, b"preserved").unwrap();
    let link = scratch.0.join("selected");
    for optimizations in [
        bsl_compiler::Optimizations::default(),
        bsl_compiler::Optimizations::all(),
    ] {
        let engine = Engine::builder()
            .optimizations(optimizations)
            .build()
            .unwrap();
        for bytecode in [false, true] {
            for (sync_name, async_name) in [
                ("УдалитьФайлы", "УдалитьФайлыАсинх"),
                ("DeleteFiles", "DeleteFilesAsync"),
            ] {
                for asynchronous in [false, true] {
                    for dynamic in [false, true] {
                        for suffix in ["", "/", "///"] {
                            std::os::unix::fs::symlink(&target, &link).unwrap();
                            let path = format!("{}{suffix}", link.display()).replace('"', "\"\"");
                            let call = if asynchronous {
                                let expression = format!("{async_name}(ПутьПробы)");
                                let expression = if dynamic {
                                    format!("Вычислить(\"{expression}\")")
                                } else {
                                    expression
                                };
                                format!(
                                    "ОбещаниеПробы = {expression}; РезультатПробы = Ждать ОбещаниеПробы;"
                                )
                            } else {
                                let statement = format!("{sync_name}(ПутьПробы);");
                                if dynamic {
                                    format!("Выполнить(\"{statement}\");")
                                } else {
                                    statement
                                }
                            };
                            let module = engine.compile(&format!(
                                "Асинх Процедура Проверить() ПутьПробы = \"{path}\"; {call} КонецПроцедуры Проверить();"
                            )).unwrap();
                            let module = if bytecode {
                                engine.load_bytecode(&module.bytecode().unwrap()).unwrap()
                            } else {
                                module
                            };
                            engine.state_builder().build().run(&module).unwrap();
                            assert_eq!(fs::read(&sentinel).unwrap(), b"preserved");
                            assert_eq!(
                                fs::symlink_metadata(&link).unwrap_err().kind(),
                                io::ErrorKind::NotFound
                            );
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn invalid_recursion_aliases_do_not_start_a_host_search() {
    for optimizations in [
        bsl_compiler::Optimizations::default(),
        bsl_compiler::Optimizations::all(),
    ] {
        let engine = Engine::builder()
            .optimizations(optimizations)
            .build()
            .unwrap();
        for word in ["Да", "Нет"] {
            for (sync_name, async_name) in [
                ("НайтиФайлы", "НайтиФайлыАсинх"),
                ("FindFiles", "FindFilesAsync"),
            ] {
                for dynamic in [false, true] {
                    let (files, entered, release, _finished) = host(true);
                    let separators = files.separators.clone();
                    // Ошибочная реализация не должна зависнуть внутри двойника.
                    release.send(()).unwrap();
                    release.send(()).unwrap();
                    let mut state = engine.state_builder().files(files).build();
                    let expression = |name: &str| {
                        let call = format!("{name}(КореньПробы, МаскаПробы, АргументПробы)");
                        if dynamic {
                            format!("Вычислить(\"{call}\")")
                        } else {
                            call
                        }
                    };
                    let result = state.exec(&format!(
                        "КореньПробы = \"virtual\"; МаскаПробы = \"*.txt\"; АргументПробы = \"{word}\"; РезультатПробы = {};",
                        expression(sync_name),
                    ));
                    assert!(matches!(
                        result,
                        Err(open_bsl::Error::Runtime(bsl_rt::RtError::TypeError { .. }))
                    ));
                    state
                        .exec(&format!(
                            r#"
Асинх Процедура Проверить()
    КореньПробы = "virtual"; МаскаПробы = "*.txt"; АргументПробы = "{word}";
    ОбещаниеПробы = {};
    Если ТипЗнч(ОбещаниеПробы) <> Тип("Обещание") Тогда ВызватьИсключение "promise"; КонецЕсли;
    ОшибкаПробы = Ложь;
    Попытка РезультатПробы = Ждать ОбещаниеПробы; Исключение ОшибкаПробы = Истина; КонецПопытки;
    Если Не ОшибкаПробы Тогда ВызватьИсключение "accepted alias"; КонецЕсли;
КонецПроцедуры
Проверить();
"#,
                            expression(async_name)
                        ))
                        .unwrap();
                    assert!(entered.try_recv().is_err());
                    assert_eq!(separators.load(Ordering::Relaxed), 0);
                }
            }
        }
    }
}

#[test]
fn every_measured_begin_method_delivers_the_expected_arguments() {
    // Имена и форма аргументов взяты из нативной матрицы 26 вызовов.
    const CASES: &[(&str, &str, &str, &str)] = &[
        (
            "НачатьПроверкуСуществования",
            "BeginCheckingExistence",
            "",
            "Истина",
        ),
        ("НачатьПроверкуЭтоФайл", "BeginCheckingIsFile", "", "Истина"),
        (
            "НачатьПроверкуЭтоКаталог",
            "BeginCheckingIsDirectory",
            "",
            "Ложь",
        ),
        ("НачатьПолучениеРазмера", "BeginGettingSize", "", "7"),
        (
            "НачатьПолучениеТолькоЧтения",
            "BeginGettingReadOnly",
            "",
            "Ложь",
        ),
        (
            "НачатьПолучениеНевидимости",
            "BeginGettingHidden",
            "",
            "Истина",
        ),
        (
            "НачатьПолучениеВремениИзменения",
            "BeginGettingModificationTime",
            "",
            "Дата(2020, 1, 2, 6, 4, 5)",
        ),
        (
            "НачатьПолучениеУниверсальногоВремениИзменения",
            "BeginGettingModificationUniversalTime",
            "",
            "Дата(2020, 1, 2, 3, 4, 5)",
        ),
        (
            "НачатьУстановкуТолькоЧтения",
            "BeginSettingReadOnly",
            ", Истина",
            "",
        ),
        (
            "НачатьУстановкуНевидимости",
            "BeginSettingHidden",
            ", Истина",
            "",
        ),
        (
            "НачатьУстановкуВремениИзменения",
            "BeginSettingModificationTime",
            ", Дата(2020, 1, 2, 6, 4, 5)",
            "",
        ),
        (
            "НачатьУстановкуУниверсальногоВремениИзменения",
            "BeginSettingModificationUniversalTime",
            ", Дата(2020, 1, 2, 3, 4, 5)",
            "",
        ),
        (
            "НачатьИнициализацию",
            "BeginInitialization",
            ", \"new-path\"",
            "ФайлПробы",
        ),
    ];
    for optimizations in [
        bsl_compiler::Optimizations::default(),
        bsl_compiler::Optimizations::all(),
    ] {
        let engine = Engine::builder()
            .optimizations(optimizations)
            .build()
            .unwrap();
        for &(ru, en, argument, expected) in CASES {
            for name in [ru, en] {
                for dynamic in [false, true] {
                    let call = format!("ФайлПробы.{name}(Описание{argument});");
                    let expression = format!("РезультатОшибки = {call}").replace('"', "\"\"");
                    let call = if dynamic {
                        format!("Выполнить(\"{}\");", call.replace('"', "\"\""))
                    } else {
                        call
                    };
                    let parameters = if expected.is_empty() {
                        "Данные"
                    } else {
                        "Результат, Данные"
                    };
                    let check = if expected.is_empty() {
                        String::new()
                    } else {
                        "Если Результат <> Данные.Ожидаемый Тогда ВызватьИсключение \"результат\"; КонецЕсли;".into()
                    };
                    let source = format!(
                        r#"
Процедура Доставлено({parameters}) Экспорт
    Если Не Данные.ВозвратБыл Тогда ВызватьИсключение "ранняя доставка"; КонецЕсли;
    {check}
    Данные.События.Добавить(Истина);
КонецПроцедуры
ФайлПробы = Новый Файл("virtual/a.txt");
Данные = Новый Структура("ВозвратБыл,Ожидаемый,События", Ложь, {}, Новый Массив);
Описание = Новый ОписаниеОповещения("Доставлено", ЭтотОбъект, Данные);
ОшибкаПозиции = Ложь;
Попытка Выполнить("{expression}"); Исключение ОшибкаПозиции = Истина; КонецПопытки;
Если Не ОшибкаПозиции Или ФайлПробы.ПолноеИмя <> "virtual/a.txt" Тогда
    ВызватьИсключение "Begin в выражении";
КонецЕсли;
{call}
Если Данные.События.Количество() <> 0 Тогда ВызватьИсключение "синхронный callback"; КонецЕсли;
Данные.ВозвратБыл = Истина;
Возврат Данные.События;
"#,
                        if expected.is_empty() {
                            "Неопределено"
                        } else {
                            expected
                        }
                    );
                    for bytecode in [false, true] {
                        let module = engine.compile(&source).unwrap();
                        let module = if bytecode {
                            engine.load_bytecode(&module.bytecode().unwrap()).unwrap()
                        } else {
                            module
                        };
                        let (files, _entered, _release, _finished) = host(true);
                        let updates = files.updates.clone();
                        let result = engine
                            .state_builder()
                            .files(files)
                            .zone(open_bsl::FixedTimeZone::new(3 * 3600).unwrap())
                            .build()
                            .run(&module)
                            .unwrap_or_else(|error| panic!("{name}: {error:?}"));
                        let open_bsl::Value::Object(object) = result else {
                            panic!("массив")
                        };
                        let bsl_rt::BslObject::Array(array) = &*object else {
                            panic!("массив")
                        };
                        assert_eq!(
                            &*array.borrow(),
                            &[open_bsl::Value::Boolean(true)],
                            "{name}"
                        );
                        if expected.is_empty() {
                            assert_eq!(updates.lock().unwrap().len(), 1, "{name}");
                        } else {
                            assert!(updates.lock().unwrap().is_empty(), "{name}");
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn invalid_attribute_aliases_never_reach_the_host_setter() {
    for optimizations in [
        bsl_compiler::Optimizations::default(),
        bsl_compiler::Optimizations::all(),
    ] {
        let engine = Engine::builder()
            .optimizations(optimizations)
            .build()
            .unwrap();
        let (files, _entered, _release, _finished) = host(true);
        let updates = files.updates.clone();
        let mut state = engine.state_builder().files(files).build();
        for word in ["Да", "Нет"] {
            for (sync_name, async_name) in [
                ("УстановитьТолькоЧтение", "УстановитьТолькоЧтениеАсинх"),
                ("SetReadOnly", "SetReadOnlyAsync"),
                ("УстановитьНевидимость", "УстановитьНевидимостьАсинх"),
                ("SetHidden", "SetHiddenAsync"),
            ] {
                for dynamic in [false, true] {
                    let statement = format!("ФайлПробы.{sync_name}(АргументПробы);");
                    let statement = if dynamic {
                        format!("Выполнить(\"{statement}\");")
                    } else {
                        statement
                    };
                    let result = state.exec(&format!(
                        "ФайлПробы = Новый Файл(\"virtual/a.txt\"); АргументПробы = \"{word}\"; {statement}"
                    ));
                    assert!(matches!(
                        result,
                        Err(open_bsl::Error::Runtime(bsl_rt::RtError::TypeError { .. }))
                    ));
                    let expression = format!("ФайлПробы.{async_name}(АргументПробы)");
                    let expression = if dynamic {
                        format!("Вычислить(\"{expression}\")")
                    } else {
                        expression
                    };
                    state
                        .exec(&format!(
                            r#"
Асинх Процедура Проверить()
    ФайлПробы = Новый Файл("virtual/a.txt");
    АргументПробы = "{word}";
    ОбещаниеПробы = {expression};
    Если ТипЗнч(ОбещаниеПробы) <> Тип("Обещание") Тогда ВызватьИсключение "promise"; КонецЕсли;
    Для ПовторПробы = 1 По 2 Цикл
        ОшибкаПробы = Ложь;
        Попытка РезультатПробы = Ждать ОбещаниеПробы; Исключение ОшибкаПробы = Истина; КонецПопытки;
        Если Не ОшибкаПробы Тогда ВызватьИсключение "accepted alias"; КонецЕсли;
    КонецЦикла;
КонецПроцедуры
Проверить();
"#
                        ))
                        .unwrap();
                    assert!(updates.lock().unwrap().is_empty());
                }
            }
        }
    }
}

#[test]
fn metadata_setters_return_promises_and_do_not_roll_back_a_canceled_write() {
    use open_bsl::FileMetadataUpdate as Update;
    let engine = Engine::builder().build().unwrap();
    for (ru, en, argument, expected) in [
        (
            "УстановитьТолькоЧтениеАсинх",
            "SetReadOnlyAsync",
            "Истина",
            Update::ReadOnly(true),
        ),
        (
            "УстановитьНевидимостьАсинх",
            "SetHiddenAsync",
            "Истина",
            Update::Hidden(true),
        ),
        (
            "УстановитьВремяИзмененияАсинх",
            "SetModificationTimeAsync",
            "Дата(2020, 1, 2, 6, 4, 5)",
            Update::Modified(1_577_934_245),
        ),
        (
            "УстановитьУниверсальноеВремяИзмененияАсинх",
            "SetModificationUniversalTimeAsync",
            "Дата(2020, 1, 2, 3, 4, 5)",
            Update::Modified(1_577_934_245),
        ),
    ] {
        for name in [ru, en] {
            for (dynamic, cancel) in [false, true]
                .into_iter()
                .flat_map(|dynamic| [false, true].map(move |cancel| (dynamic, cancel)))
            {
                let call = format!("ФайлПробы.{name}({argument})");
                let expression = if dynamic {
                    format!("Вычислить(\"{call}\")")
                } else {
                    call
                };
                let source = format!(
                    r#"
Асинх Процедура Проверить()
    ФайлПробы = Новый Файл("virtual/a.txt");
    ОбещаниеПробы = {expression};
    Если Строка(ТипЗнч(ОбещаниеПробы)) <> "Обещание" Тогда ВызватьИсключение "promise"; КонецЕсли;
    РезультатПробы = Ждать ОбещаниеПробы;
    Если РезультатПробы <> Неопределено Тогда ВызватьИсключение "result"; КонецЕсли;
    ПовторПробы = Ждать ОбещаниеПробы;
    Если ПовторПробы <> Неопределено Тогда ВызватьИсключение "repeat"; КонецЕсли;
    ОбещаниеОшибки = ФайлПробы.{name}(Неопределено);
    ПойманаОшибка = Ложь;
    Попытка
        РезультатОшибки = Ждать ОбещаниеОшибки;
    Исключение
        ПойманаОшибка = Истина;
    КонецПопытки;
    Если Не ПойманаОшибка Тогда ВызватьИсключение "no error"; КонецЕсли;
КонецПроцедуры
Проверить();
"#
                );
                let module = engine.compile(&source).unwrap();
                let (mut files, entered, release, finished) = host(true);
                files.block_update = true;
                let updates = files.updates.clone();
                let mut state = engine
                    .state_builder()
                    .files(files)
                    .zone(open_bsl::FixedTimeZone::new(3 * 3600).unwrap())
                    .build();
                let mut execution = state.start(&module).unwrap();
                assert!(matches!(
                    execution.poll(128).unwrap(),
                    ExecutionPoll::Waiting
                ));
                entered.recv_timeout(Duration::from_secs(5)).unwrap();
                assert!(updates.lock().unwrap().is_empty());
                if cancel {
                    drop(execution);
                } else {
                    release.send(()).unwrap();
                    let deadline = Instant::now() + Duration::from_secs(5);
                    while !matches!(execution.poll(128).unwrap(), ExecutionPoll::Complete(_)) {
                        assert!(Instant::now() < deadline, "{name}");
                        std::thread::yield_now();
                    }
                }
                if cancel {
                    release.send(()).unwrap();
                }
                finished.recv_timeout(Duration::from_secs(5)).unwrap();
                assert_eq!(*updates.lock().unwrap(), [expected]);
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn single_worker_pool_runs_other_jobs_and_wakes_or_cancels_file_waiters() {
    use open_bsl::jobs::{BackgroundJobConfig, BackgroundStateFactory};
    use open_bsl::{JobStateDto, StateBuilder};

    struct Profile(Files);
    impl BackgroundStateFactory for Profile {
        fn configure(&self, builder: StateBuilder) -> Result<StateBuilder, String> {
            Ok(builder.files(self.0.clone()))
        }
    }

    for (cancel, dynamic, operation, module_reference) in [false, true]
        .into_iter()
        .flat_map(|cancel| [false, true].map(|dynamic| (cancel, dynamic)))
        .flat_map(|(cancel, dynamic)| {
            ["search", "read", "write", "begin_read", "begin_write"]
                .map(|operation| (cancel, dynamic, operation))
        })
        .flat_map(|(cancel, dynamic, operation)| {
            [false, true].map(|module_reference| (cancel, dynamic, operation, module_reference))
        })
    {
        let (mut files, entered, release, finished) = host(true);
        files.block_metadata = matches!(operation, "read" | "begin_read");
        files.block_update = matches!(operation, "write" | "begin_write");
        let next_calls = files.next_calls.clone();
        let updates = files.updates.clone();
        let call = match operation {
            "read" => "ФайлПробы.SizeAsync()",
            "write" => "ФайлПробы.SetHiddenAsync(Истина)",
            "begin_read" => {
                "ФайлПробы.BeginGettingSize(Новый NotifyDescription(\"Прочитано\", ЭтотОбъект))"
            }
            "begin_write" => {
                "ФайлПробы.BeginSettingHidden(Новый NotifyDescription(\"Записано\", ЭтотОбъект), Истина)"
            }
            _ => "НайтиФайлыАсинх(\"virtual\", \"*.txt\")",
        };
        let notification = operation.starts_with("begin_");
        let call = if dynamic {
            format!(
                "{}(\"{}\")",
                if notification {
                    "Выполнить"
                } else {
                    "Вычислить"
                },
                call.replace('"', "\"\"")
            )
        } else {
            call.to_owned()
        };
        let mut builder = Engine::builder()
            .background_jobs(BackgroundJobConfig {
                workers: Some(1),
                ..Default::default()
            })
            .common_module(
                "ПоисковыеРаботы",
                &r#"
Асинх Процедура Поиск() Экспорт
    ФайлПробы = Новый Файл("virtual/a.txt");
    Найденные = Ждать ВЫЗОВ;
    Если УСЛОВИЕ Тогда
        ВызватьИсключение "ошибка результата поиска";
    КонецЕсли;
КонецПроцедуры
Процедура Длинная() Экспорт
    ЗАПУСК;
КонецПроцедуры
Процедура Короткая() Экспорт
КонецПроцедуры
Процедура Прочитано(Результат, Данные) Экспорт
    Если Результат <> 7 Тогда ВызватьИсключение "callback result"; КонецЕсли;
    Сообщить("notification delivered");
КонецПроцедуры
Процедура Записано(Данные) Экспорт
    Если Данные <> Неопределено Тогда ВызватьИсключение "callback data"; КонецЕсли;
    Сообщить("notification delivered");
КонецПроцедуры
"#
                .replace(
                    "Найденные = Ждать ВЫЗОВ;",
                    if notification {
                        "ВЫЗОВ; Найденные = Неопределено;"
                    } else {
                        "Найденные = Ждать ВЫЗОВ;"
                    },
                )
                .replace(
                    "ЗАПУСК",
                    if module_reference {
                        "ЭтотОбъект.Поиск()"
                    } else {
                        "Поиск()"
                    },
                )
                .replace("ВЫЗОВ", &call)
                .replace(
                    "УСЛОВИЕ",
                    match operation {
                        "read" => "Найденные <> 7",
                        "write" => "Найденные <> Неопределено",
                        "begin_read" | "begin_write" => "Ложь",
                        _ => "Найденные.Количество() <> 1 Или Найденные[0].Размер() <> 7",
                    },
                ),
            );
        let profile = builder.register_host_profile(Arc::new(Profile(files)));
        let engine = builder.build().unwrap();
        let runtime = engine.job_runtime().unwrap();
        let mut state = engine
            .state_builder()
            .host_profile(profile)
            .unwrap()
            .build();
        let start = engine
            .compile_entry("ФоновыеЗадания.Выполнить(\"ПоисковыеРаботы.Длинная\");")
            .unwrap();
        state.run(&start).unwrap();
        entered.recv_timeout(Duration::from_secs(5)).unwrap();
        let waiting = runtime
            .snapshots()
            .into_iter()
            .find(|job| job.method_name == "ПоисковыеРаботы.Длинная")
            .unwrap();
        let quick = engine
            .compile_entry("ФоновыеЗадания.Выполнить(\"ПоисковыеРаботы.Короткая\");")
            .unwrap();
        state.run(&quick).unwrap();
        let quick = runtime
            .snapshots()
            .into_iter()
            .find(|job| job.method_name == "ПоисковыеРаботы.Короткая")
            .unwrap();
        let quick_completed = runtime
            .wait_terminal(&[quick.id], Some(Duration::from_secs(2)))
            .unwrap();
        let waiting_state = runtime.snapshot(waiting.id).unwrap().state;
        let canceled_before_release = if cancel {
            runtime.cancel(waiting.id).unwrap();
            runtime
                .wait_terminal(&[waiting.id], Some(Duration::from_secs(2)))
                .unwrap()
        } else {
            false
        };
        // Освобождаем тестовый host до утверждений, чтобы сбой проверки
        // не оставил файловый поток ждать аварийного таймаута.
        release.send(()).unwrap();
        assert!(
            quick_completed,
            "другое задание не получило единственный worker"
        );
        assert_eq!(
            runtime.snapshot(quick.id).unwrap().state,
            JobStateDto::Completed
        );
        assert_eq!(waiting_state, JobStateDto::Running);
        assert!(
            runtime
                .wait_terminal(&[waiting.id], Some(Duration::from_secs(5)))
                .unwrap()
        );
        finished.recv_timeout(Duration::from_secs(5)).unwrap();
        let done = runtime.snapshot(waiting.id).unwrap();
        if notification {
            let messages = runtime.take_messages(waiting.id, false).unwrap();
            assert_eq!(messages.len(), usize::from(!cancel));
        }
        if matches!(operation, "write" | "begin_write") {
            assert_eq!(
                *updates.lock().unwrap(),
                [open_bsl::FileMetadataUpdate::Hidden(true)]
            );
        }
        if cancel {
            assert!(
                canceled_before_release,
                "отмена ждала завершения файлового I/O"
            );
            assert_eq!(done.state, JobStateDto::Canceled);
            assert_eq!(next_calls.load(Ordering::Relaxed), 0);
        } else {
            assert_eq!(done.state, JobStateDto::Completed, "{:?}", done.error);
        }
    }
}
