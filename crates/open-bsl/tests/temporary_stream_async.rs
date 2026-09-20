use open_bsl::{
    Engine, FileHandle, FileMetadata, FileSystem, TemporaryFileResource, TransferableTemporaryFile,
    Value,
};
use std::{
    io::{self, Cursor, Read, Seek, SeekFrom, Write},
    sync::{
        Arc, Condvar, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

#[derive(Debug, Default)]
struct Evidence {
    creates: usize,
    removes: usize,
    closes: usize,
    live: bool,
    bytes: Cursor<Vec<u8>>,
    fail_create: bool,
}

#[derive(Debug, Clone)]
struct Files(Arc<Mutex<Evidence>>);

#[derive(Debug)]
struct Handle(Arc<Mutex<Evidence>>, bool);

impl Read for Handle {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        self.0.lock().unwrap().bytes.read(bytes)
    }
}

impl Write for Handle {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().bytes.write(bytes)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Seek for Handle {
    fn seek(&mut self, offset: SeekFrom) -> io::Result<u64> {
        self.0.lock().unwrap().bytes.seek(offset)
    }
}

impl FileHandle for Handle {
    fn len(&self) -> io::Result<u64> {
        Ok(self.0.lock().unwrap().bytes.get_ref().len() as u64)
    }

    fn close(&mut self) -> io::Result<()> {
        if !std::mem::replace(&mut self.1, true) {
            self.0.lock().unwrap().closes += 1;
        }
        Ok(())
    }
}

impl Drop for Handle {
    fn drop(&mut self) {
        self.close().unwrap();
    }
}

#[derive(Debug)]
struct Resource(Arc<Mutex<Evidence>>);

impl TemporaryFileResource for Resource {
    fn path(&self) -> &str {
        "/virtual/temporary-async"
    }

    fn remove(self: Box<Self>) -> io::Result<()> {
        let mut evidence = self.0.lock().unwrap();
        evidence.live = false;
        evidence.removes += 1;
        Ok(())
    }
}

impl FileSystem for Files {
    fn supports_temporary_file_ownership(&self) -> bool {
        true
    }

    fn background_access(&self) -> Option<Arc<dyn FileSystem + Send + Sync>> {
        Some(Arc::new(self.clone()))
    }

    fn create_temporary_file(&self, _: &[u8; 16]) -> io::Result<open_bsl::OpenedTemporaryFile> {
        panic!("асинхронный путь не вызывает синхронное создание")
    }

    fn create_transferable_temporary_file(
        &self,
        _: &[u8; 16],
    ) -> io::Result<TransferableTemporaryFile> {
        let mut evidence = self.0.lock().unwrap();
        if evidence.fail_create {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
        assert!(!evidence.live);
        evidence.live = true;
        evidence.creates += 1;
        drop(evidence);
        Ok(TransferableTemporaryFile::new(
            Box::new(Handle(self.0.clone(), false)),
            Box::new(Resource(self.0.clone())),
        ))
    }

    fn read(&self, _: &str) -> io::Result<Vec<u8>> {
        panic!("без повторного открытия")
    }

    fn write(&self, _: &str, _: &[u8]) -> io::Result<()> {
        panic!("без повторного открытия")
    }

    fn open(&self, _: &str, _: open_bsl::FileOpenOptions) -> io::Result<Box<dyn FileHandle>> {
        panic!("без повторного открытия")
    }

    fn metadata(&self, path: &str) -> io::Result<FileMetadata> {
        assert_eq!(path, "/virtual/temporary-async");
        let evidence = self.0.lock().unwrap();
        if !evidence.live {
            return Err(io::ErrorKind::NotFound.into());
        }
        Ok(FileMetadata::file(None).with_size(evidence.bytes.get_ref().len() as u64))
    }

    fn create_dir_all(&self, _: &str) -> io::Result<()> {
        panic!("без системного fallback")
    }

    fn path_separator(&self) -> io::Result<String> {
        Ok("/".into())
    }

    fn read_dir<'a>(
        &'a self,
        _: &str,
    ) -> io::Result<Box<dyn Iterator<Item = io::Result<open_bsl::DirEntry>> + 'a>> {
        panic!("без обхода каталогов")
    }
}

#[derive(Debug, Clone)]
struct DeferredFiles {
    evidence: Arc<Mutex<Evidence>>,
    gate: Arc<(Mutex<bool>, Condvar)>,
    started: Arc<AtomicBool>,
}

impl FileSystem for DeferredFiles {
    fn supports_temporary_file_ownership(&self) -> bool {
        true
    }

    fn background_access(&self) -> Option<Arc<dyn FileSystem + Send + Sync>> {
        Some(Arc::new(self.clone()))
    }

    fn create_transferable_temporary_file(
        &self,
        entropy: &[u8; 16],
    ) -> io::Result<TransferableTemporaryFile> {
        self.started.store(true, Ordering::SeqCst);
        let (lock, ready) = &*self.gate;
        let mut released = lock.lock().unwrap();
        while !*released {
            released = ready.wait(released).unwrap();
        }
        Files(self.evidence.clone()).create_transferable_temporary_file(entropy)
    }

    fn read(&self, path: &str) -> io::Result<Vec<u8>> {
        Files(self.evidence.clone()).read(path)
    }

    fn write(&self, path: &str, data: &[u8]) -> io::Result<()> {
        Files(self.evidence.clone()).write(path, data)
    }

    fn open(
        &self,
        path: &str,
        options: open_bsl::FileOpenOptions,
    ) -> io::Result<Box<dyn FileHandle>> {
        Files(self.evidence.clone()).open(path, options)
    }

    fn metadata(&self, path: &str) -> io::Result<FileMetadata> {
        Files(self.evidence.clone()).metadata(path)
    }

    fn create_dir_all(&self, path: &str) -> io::Result<()> {
        Files(self.evidence.clone()).create_dir_all(path)
    }

    fn path_separator(&self) -> io::Result<String> {
        Ok("/".into())
    }

    fn read_dir<'a>(
        &'a self,
        _: &str,
    ) -> io::Result<Box<dyn Iterator<Item = io::Result<open_bsl::DirEntry>> + 'a>> {
        panic!("без обхода каталогов")
    }
}

#[derive(Clone)]
struct Capture(Arc<Mutex<Vec<u8>>>);

impl Write for Capture {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn async_temporary_stream_preserves_identity_and_defers_parameter_errors() {
    let source = r#"
Асинх Процедура Проба()
	Обещание = СОЗДАНИЕ;
	Поток = Ждать Обещание;
	Если ТипЗнч(Поток) <> Тип("ФайловыйПоток") Или Поток.Размер() <> 0 Или Поток.ТекущаяПозиция() <> 0 Тогда ВызватьИсключение "result"; КонецЕсли;
	Если Не Поток.ДоступноЧтение Или Не Поток.ДоступнаЗапись Или Не Поток.ДоступноИзменениеПозиции Тогда ВызватьИсключение "access"; КонецЕсли;
	Имя = Поток.FileName;
	Файл = Новый Файл(Имя);
	Если Не Файл.Существует() Тогда ВызватьИсключение "carrier"; КонецЕсли;
	Поток.Close();
	Если Ждать Обещание <> Поток Или Поток.ИмяФайла <> Имя Тогда ВызватьИсключение "identity"; КонецЕсли;
	Ошибочное = FileStreams.CreateTempFileAsync(8, "8");
	Для Номер = 1 По 2 Цикл
		Попытка
			Ждать Ошибочное;
			ВызватьИсключение "missing error";
		Исключение
		КонецПопытки;
	КонецЦикла;
	Если Имя <> "/virtual/temporary-async" Тогда ВызватьИсключение "name"; КонецЕсли;
КонецПроцедуры
Проба();
"#;
    let engine = Engine::builder().build().unwrap();
    for dynamic in [false, true] {
        let source = source.replace(
            "СОЗДАНИЕ",
            if dynamic {
                "Вычислить(\"FileStreams.CreateTempFileAsync(65535,8192)\")"
            } else {
                "ФайловыеПотоки.СоздатьВременныйФайлАсинх()"
            },
        );
        let module = engine.compile(&source).unwrap();
        for bytecode in [false, true] {
            let module = if bytecode {
                engine.load_bytecode(&module.bytecode().unwrap()).unwrap()
            } else {
                module.clone()
            };
            let evidence = Arc::new(Mutex::new(Evidence::default()));
            let cleanup_evidence = evidence.clone();
            let mut state = engine
                .state_builder()
                .files(Files(evidence.clone()))
                .temporary_file_cleanup(move |resources| {
                    for resource in resources {
                        resource.remove()?;
                    }
                    assert_eq!(cleanup_evidence.lock().unwrap().removes, 1);
                    Ok(())
                })
                .build();
            assert_eq!(state.run(&module).unwrap(), Value::Undefined);
            {
                let evidence = evidence.lock().unwrap();
                assert_eq!(evidence.creates, 1);
                assert_eq!(evidence.closes, 1);
                assert_eq!(evidence.removes, 0);
                assert!(evidence.live);
            }
            drop(state);
            let evidence = evidence.lock().unwrap();
            assert_eq!(evidence.removes, 1);
            assert!(!evidence.live);
        }
    }
}

#[test]
fn measured_async_temporary_parameters_keep_the_call_and_await_stages() {
    let cases = [
        ("", true, false),
        ("65535,8192", true, false),
        ("0,0,0", false, true),
        ("0", true, false),
        ("8,0", true, false),
        ("1", true, false),
        ("8,1", true, false),
        ("8", true, false),
        ("8,8", true, false),
        ("8192", true, false),
        ("8,8192", true, false),
        ("65535", true, false),
        ("8,65535", true, false),
        ("65536", true, false),
        ("8,65536", true, false),
        ("Неопределено", true, false),
        ("8,Неопределено", true, false),
        ("Null", false, false),
        ("8,Null", false, false),
        ("-1", false, false),
        ("8,-1", false, false),
        ("1.5", false, false),
        ("8,1.5", false, false),
        ("\"8\"", false, false),
        ("8,\"8\"", false, false),
        ("Ложь", false, false),
        ("8,Ложь", false, false),
        ("Истина", false, false),
        ("8,Истина", false, false),
        ("Дата(2020,1,2)", false, false),
        ("8,Дата(2020,1,2)", false, false),
        ("Новый Массив", false, false),
        ("8,Новый Массив", false, false),
        ("Новый Структура", false, false),
        ("8,Новый Структура", false, false),
    ];
    for optimizations in [
        bsl_compiler::Optimizations::default(),
        bsl_compiler::Optimizations::all(),
    ] {
        let engine = Engine::builder()
            .optimizations(optimizations)
            .build()
            .unwrap();
        for method in [
            "ФайловыеПотоки.СоздатьВременныйФайлАсинх",
            "FileStreams.CreateTempFileAsync",
        ] {
            for &(arguments, success, call_error) in &cases {
                let expression = format!("{method}({arguments})").replace('"', "\"\"");
                let check = if call_error {
                    r#"
	Если Обещание <> Неопределено Или Стадия <> "call" Тогда ВызватьИсключение "call stage"; КонецЕсли;
"#
                } else if success {
                    r#"
	Если Стадия <> "ok" Или ТипЗнч(Результат) <> Тип("ФайловыйПоток") Тогда ВызватьИсключение "success"; КонецЕсли;
	Если Ждать Обещание <> Результат Тогда ВызватьИсключение "repeat"; КонецЕсли;
	Результат.Закрыть();
"#
                } else {
                    r#"
	Если Стадия <> "await" Тогда ВызватьИсключение "await stage"; КонецЕсли;
	ПовторнаяОшибка = Ложь;
	Попытка Ждать Обещание; Исключение ПовторнаяОшибка = Истина; КонецПопытки;
	Если Не ПовторнаяОшибка Тогда ВызватьИсключение "repeat error"; КонецЕсли;
"#
                };
                let source = format!(
                    r#"
Асинх Процедура Проверить()
	Обещание = Неопределено; Стадия = "call";
	Попытка
		Обещание = Вычислить("{expression}"); Стадия = "await";
		Результат = Ждать Обещание; Стадия = "ok";
	Исключение
	КонецПопытки;
{check}
КонецПроцедуры
Проверить();
"#
                );
                let compiled = engine.compile(&source).unwrap();
                for bytecode in [false, true] {
                    let module = if bytecode {
                        engine.load_bytecode(&compiled.bytecode().unwrap()).unwrap()
                    } else {
                        compiled.clone()
                    };
                    let evidence = Arc::new(Mutex::new(Evidence::default()));
                    let mut state = engine
                        .state_builder()
                        .files(Files(evidence.clone()))
                        .temporary_file_cleanup(|resources| {
                            for resource in resources {
                                resource.remove()?;
                            }
                            Ok(())
                        })
                        .build();
                    state.run(&module).unwrap_or_else(|error| {
                        panic!("{method}({arguments}), bytecode={bytecode}: {error:?}")
                    });
                    assert_eq!(
                        evidence.lock().unwrap().creates,
                        usize::from(success),
                        "{method}({arguments}), bytecode={bytecode}"
                    );
                }
            }
        }
    }
}

#[test]
fn waiting_for_temporary_creation_does_not_block_a_neighbor_task() {
    let engine = Engine::builder().build().unwrap();
    let module = engine
        .compile(
            r#"
Асинх Процедура Долгая()
	Поток = Ждать FileStreams.CreateTempFileAsync();
	Поток.Close();
КонецПроцедуры
Асинх Процедура Соседняя()
	Сообщить("neighbor");
КонецПроцедуры
Долгая();
Соседняя();
"#,
        )
        .unwrap();
    let evidence = Arc::new(Mutex::new(Evidence::default()));
    let gate = Arc::new((Mutex::new(false), Condvar::new()));
    let started = Arc::new(AtomicBool::new(false));
    let output = Arc::new(Mutex::new(Vec::new()));
    let files = DeferredFiles {
        evidence: evidence.clone(),
        gate: gate.clone(),
        started: started.clone(),
    };
    let mut state = engine
        .state_builder()
        .files(files)
        .stdout(Capture(output.clone()))
        .temporary_file_cleanup(|resources| {
            for resource in resources {
                resource.remove()?;
            }
            Ok(())
        })
        .build();
    let mut execution = state.start(&module).unwrap();
    let mut waiting = false;
    for _ in 0..1_000 {
        match execution.poll(16).unwrap() {
            open_bsl::ExecutionPoll::Waiting => waiting = true,
            open_bsl::ExecutionPoll::Runnable => {}
            open_bsl::ExecutionPoll::Complete(_) => panic!("операция ещё закрыта затвором"),
        }
        if waiting && started.load(Ordering::SeqCst) {
            break;
        }
        std::thread::yield_now();
    }
    assert!(waiting && started.load(Ordering::SeqCst));
    assert_eq!(&*output.lock().unwrap(), b"neighbor\n");
    let (lock, ready) = &*gate;
    *lock.lock().unwrap() = true;
    ready.notify_one();
    let mut complete = false;
    for _ in 0..10_000 {
        if matches!(
            execution.poll(16).unwrap(),
            open_bsl::ExecutionPoll::Complete(Value::Undefined)
        ) {
            complete = true;
            break;
        }
        std::thread::yield_now();
    }
    assert!(complete);
    drop(execution);
    drop(state);
    assert_eq!(evidence.lock().unwrap().removes, 1);
}

#[test]
fn dropping_execution_waits_for_started_temporary_creation_before_state_cleanup() {
    let engine = Engine::builder().build().unwrap();
    let module = engine
        .compile(
            r#"
Асинх Процедура Запустить()
	Ждать FileStreams.CreateTempFileAsync();
КонецПроцедуры
Запустить();
"#,
        )
        .unwrap();
    let evidence = Arc::new(Mutex::new(Evidence::default()));
    let gate = Arc::new((Mutex::new(false), Condvar::new()));
    let started = Arc::new(AtomicBool::new(false));
    let mut state = engine
        .state_builder()
        .files(DeferredFiles {
            evidence: evidence.clone(),
            gate: gate.clone(),
            started: started.clone(),
        })
        .temporary_file_cleanup(|resources| {
            for resource in resources {
                resource.remove()?;
            }
            Ok(())
        })
        .build();
    let mut execution = state.start(&module).unwrap();
    for _ in 0..10_000 {
        let _ = execution.poll(16).unwrap();
        if started.load(Ordering::SeqCst) {
            break;
        }
        std::thread::yield_now();
    }
    assert!(started.load(Ordering::SeqCst));

    let release = std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(20));
        let (lock, ready) = &*gate;
        *lock.lock().unwrap() = true;
        ready.notify_one();
    });
    drop(execution);
    drop(state);
    release.join().unwrap();

    let evidence = evidence.lock().unwrap();
    assert_eq!(evidence.creates, 1);
    assert_eq!(evidence.removes, 1);
    assert!(!evidence.live);
}

#[test]
fn host_creation_failure_is_repeatable_at_await() {
    let engine = Engine::builder().build().unwrap();
    let module = engine
        .compile(
            r#"
Асинх Процедура Проверить()
	Обещание = FileStreams.CreateTempFileAsync();
	Для Повтор = 1 По 2 Цикл
		ОшибкаБыла = Ложь;
		Попытка Ждать Обещание; Исключение ОшибкаБыла = Истина; КонецПопытки;
		Если Не ОшибкаБыла Тогда ВызватьИсключение "host error"; КонецЕсли;
	КонецЦикла;
КонецПроцедуры
Проверить();
"#,
        )
        .unwrap();
    let evidence = Arc::new(Mutex::new(Evidence {
        fail_create: true,
        ..Evidence::default()
    }));
    let mut state = engine
        .state_builder()
        .files(Files(evidence.clone()))
        .build();
    state.run(&module).unwrap();
    assert_eq!(evidence.lock().unwrap().creates, 0);
}
