use open_bsl::{
    Engine, FileHandle, FileMetadata, FileSystem, OpenedTemporaryFile, TemporaryFileResource, Value,
};
use std::{
    cell::{Cell, RefCell},
    io::{self, Cursor, Read, Seek, SeekFrom, Write},
    rc::Rc,
};

#[derive(Debug, Default)]
struct State {
    creates: Cell<usize>,
    removes: Cell<usize>,
    closes: Cell<usize>,
    bytes: RefCell<Cursor<Vec<u8>>>,
    live: Cell<bool>,
    entropy: RefCell<Vec<[u8; 16]>>,
    unowned: Cell<bool>,
    close_registry: RefCell<Option<open_bsl::TemporaryFileRegistry>>,
}

#[derive(Debug, Clone)]
struct Files(Rc<State>, bool);

#[derive(Debug)]
struct Handle(Rc<State>, bool);
impl Read for Handle {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        self.0.bytes.borrow_mut().read(bytes)
    }
}
impl Write for Handle {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.bytes.borrow_mut().write(bytes)
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
impl Seek for Handle {
    fn seek(&mut self, offset: SeekFrom) -> io::Result<u64> {
        self.0.bytes.borrow_mut().seek(offset)
    }
}
impl FileHandle for Handle {
    fn len(&self) -> io::Result<u64> {
        Ok(self.0.bytes.borrow().get_ref().len() as u64)
    }
    fn close(&mut self) -> io::Result<()> {
        if !std::mem::replace(&mut self.1, true) {
            self.0.closes.set(self.0.closes.get() + 1);
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
struct Resource(Rc<State>);
impl TemporaryFileResource for Resource {
    fn path(&self) -> &str {
        "/virtual/temporary"
    }
    fn remove(self: Box<Self>) -> io::Result<()> {
        self.0.live.set(false);
        self.0.removes.set(self.0.removes.get() + 1);
        Ok(())
    }
}
impl FileSystem for Files {
    fn supports_temporary_file_ownership(&self) -> bool {
        self.1
    }
    fn create_temporary_file(&self, entropy: &[u8; 16]) -> io::Result<OpenedTemporaryFile> {
        self.0.entropy.borrow_mut().push(*entropy);
        assert!(!self.0.live.replace(true), "одна проба создаёт один файл");
        self.0.creates.set(self.0.creates.get() + 1);
        if let Some(registry) = self.0.close_registry.borrow_mut().take() {
            assert!(registry.close().is_empty());
        }
        if self.0.unowned.get() {
            return Ok(OpenedTemporaryFile::new(
                "/virtual/temporary".into(),
                Box::new(Handle(self.0.clone(), false)),
            ));
        }
        Ok(OpenedTemporaryFile::new_owned(
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
    fn create_dir_all(&self, _: &str) -> io::Result<()> {
        panic!("без системного fallback")
    }
    fn metadata(&self, path: &str) -> io::Result<FileMetadata> {
        assert_eq!(path, "/virtual/temporary");
        if !self.0.live.get() {
            return Err(io::ErrorKind::NotFound.into());
        }
        Ok(FileMetadata::file(None).with_size(self.0.bytes.borrow().get_ref().len() as u64))
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

#[test]
fn a_temporary_stream_uses_the_created_handle_and_waits_for_session_cleanup() {
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
            for bytecode in [false, true] {
                let source = r#"
ПотокПробы = СОЗДАНИЕ;
Если ТипЗнч(ПотокПробы) <> Тип("ФайловыйПоток") Тогда ВызватьИсключение "type"; КонецЕсли;
Если Не ПотокПробы.ДоступноЧтение Или Не ПотокПробы.ДоступнаЗапись Или Не ПотокПробы.ДоступноИзменениеПозиции Тогда ВызватьИсключение "access"; КонецЕсли;
Если ПотокПробы.Размер() <> 0 Или ПотокПробы.ТекущаяПозиция() <> 0 Тогда ВызватьИсключение "initial"; КонецЕсли;
ИмяПробы = ПотокПробы.ИмяФайла;
ФайлПробы = Новый Файл(ИмяПробы);
Если Не ФайлПробы.Существует() Тогда ВызватьИсключение "deferred creation"; КонецЕсли;
БуферПробы = Новый БуферДвоичныхДанных(2); БуферПробы[0] = 65; БуферПробы[1] = 90;
ПотокПробы.Записать(БуферПробы, 0, 2);
Если ФайлПробы.Размер() <> 2 Тогда ВызватьИсключение "write"; КонецЕсли;
ПотокПробы.Перейти(0, ПозицияВПотоке.Начало);
Если ПотокПробы.Прочитать(БуферПробы, 0, 2) <> 2 Или БуферПробы[0] <> 65 Или БуферПробы[1] <> 90 Тогда ВызватьИсключение "read"; КонецЕсли;
ПотокПробы.Закрыть();
Если ПотокПробы.FileName <> ИмяПробы Или Не ФайлПробы.Существует() Тогда ВызватьИсключение "closed"; КонецЕсли;
Возврат ИмяПробы;
"#.replace("СОЗДАНИЕ", if dynamic { "Вычислить(\"FileStreams.CreateTempFile(65535,8192)\")" } else { "ФайловыеПотоки.СоздатьВременныйФайл()" });
                let module = engine.compile(&source).unwrap();
                let module = if bytecode {
                    engine.load_bytecode(&module.bytecode().unwrap()).unwrap()
                } else {
                    module
                };
                let evidence = Rc::new(State::default());
                let mut state = engine
                    .state_builder()
                    .files(Files(evidence.clone(), true))
                    .temporary_file_cleanup(|resources| {
                        for resource in resources {
                            resource.remove()?;
                        }
                        Ok(())
                    })
                    .build();
                assert_eq!(
                    state.run(&module).unwrap(),
                    Value::Str("/virtual/temporary".into())
                );
                assert_eq!(evidence.creates.get(), 1);
                assert_eq!(evidence.closes.get(), 1);
                assert!(evidence.live.get());
                assert_eq!(evidence.removes.get(), 0);
                drop(state);
                assert_eq!(evidence.removes.get(), 1);
                assert!(!evidence.live.get());
            }
        }
    }
}

fn minimal(shapes: &mut open_bsl::RuntimeShapes) -> open_bsl::CallContext<'_> {
    open_bsl::CallContext::minimal(shapes, bsl_format::format_value)
}

#[test]
fn a_saved_manager_retains_its_session_and_rejects_creation_after_that_session_closes() {
    struct FixedRandom;
    impl open_bsl::RandomSource for FixedRandom {
        fn fill(&mut self, bytes: &mut [u8; 16]) {
            *bytes = [0xa1; 16];
        }
    }
    let engine = Engine::builder().build().unwrap();
    for close_first in [false, true] {
        let evidence = Rc::new(State::default());
        let mut first = engine
            .state_builder()
            .files(Files(evidence.clone(), true))
            .random(FixedRandom)
            .temporary_file_cleanup(|resources| {
                for resource in resources {
                    resource.remove()?;
                }
                Ok(())
            })
            .build();
        let manager = first.eval("FileStreams").unwrap();
        let mut first = Some(first);
        if close_first {
            drop(first.take());
        }
        let second = engine.new_state();
        let registry = second.temporary_files();
        let mut shapes = open_bsl::RuntimeShapes::seeded(vec![], vec![], None);
        let mut context = minimal(&mut shapes);
        context.set_temporary_files(registry.clone());
        let result = manager
            .object_ref()
            .unwrap()
            .call_method("CreateTempFile", &[], &mut context);
        if close_first {
            assert!(result.is_err());
            assert_eq!(evidence.creates.get(), 0);
        } else {
            let stream = result.unwrap();
            assert_eq!(evidence.creates.get(), 1);
            assert_eq!(&*evidence.entropy.borrow(), &[[0xa1; 16]]);
            stream
                .object_ref()
                .unwrap()
                .call_method("Close", &[], &mut context)
                .unwrap();
            drop(stream);
            drop(second);
            assert_eq!(evidence.removes.get(), 0);
            first.as_mut().unwrap().cleanup_temporary_files();
            assert_eq!(evidence.removes.get(), 1);
        }
        assert!(
            registry.take_pending().is_empty(),
            "не перенести ресурс в чужой сеанс"
        );
    }
}

#[test]
fn legacy_manager_does_not_adopt_the_callers_registry_or_file_authority() {
    let evidence = Rc::new(State::default());
    let manager = bsl_stream::new_file_streams_manager(Rc::new(Files(evidence.clone(), true)));
    let mut shapes = open_bsl::RuntimeShapes::seeded(vec![], vec![], None);
    let mut context = minimal(&mut shapes);
    let registry = open_bsl::TemporaryFileRegistry::default();
    context.set_temporary_files(registry.clone());
    assert!(
        manager
            .object_ref()
            .unwrap()
            .call_method("CreateTempFile", &[], &mut context)
            .is_err()
    );
    assert_eq!(evidence.creates.get(), 0);
    assert!(registry.take_pending().is_empty());
}

#[test]
fn a_host_without_ownership_is_refused_before_creating_a_file() {
    let engine = Engine::builder().build().unwrap();
    let evidence = Rc::new(State::default());
    let mut state = engine
        .state_builder()
        .files(Files(evidence.clone(), false))
        .build();
    let error = state
        .exec("Возврат FileStreams.CreateTempFile();")
        .unwrap_err();
    assert!(matches!(
        error,
        open_bsl::Error::Runtime(open_bsl::RtError::IoError(_))
    ));
    assert_eq!(evidence.creates.get(), 0);
    assert!(!evidence.live.get());
}

#[test]
fn a_broken_host_contract_does_not_turn_the_path_into_deletion_authority() {
    let engine = Engine::builder().build().unwrap();
    for unowned in [false, true] {
        let evidence = Rc::new(State::default());
        evidence.unowned.set(unowned);
        let mut state = engine
            .state_builder()
            .files(Files(evidence.clone(), true))
            .temporary_file_cleanup(|_| panic!("не зарегистрирован ни один ресурс"))
            .build();
        if !unowned {
            *evidence.close_registry.borrow_mut() = Some(state.temporary_files());
        }
        assert!(state.exec("Возврат FileStreams.CreateTempFile();").is_err());
        assert_eq!(evidence.creates.get(), 1);
        assert_eq!(evidence.closes.get(), 1);
        assert!(state.temporary_files().take_pending().is_empty());
        drop(state);
        assert!(evidence.live.get());
        assert_eq!(evidence.removes.get(), 0);
    }
}

#[test]
fn bsl_errors_do_not_delete_the_file_before_the_host_cleanup() {
    let engine = Engine::builder().build().unwrap();
    for cleanup in [false, true] {
        let evidence = Rc::new(State::default());
        let builder = engine.state_builder().files(Files(evidence.clone(), true));
        let mut state = if cleanup {
            builder
                .temporary_file_cleanup(|resources| {
                    for resource in resources {
                        resource.remove()?;
                    }
                    Ok(())
                })
                .build()
        } else {
            builder.build()
        };
        assert!(
            state
                .exec("П = FileStreams.CreateTempFile(); ВызватьИсключение \"failure\";")
                .is_err()
        );
        assert_eq!(evidence.creates.get(), 1);
        assert!(evidence.live.get());
        assert_eq!(evidence.removes.get(), 0);
        drop(state);
        assert_eq!(evidence.closes.get(), 1);
        assert_eq!(evidence.removes.get(), usize::from(cleanup));
        assert_eq!(evidence.live.get(), !cleanup);
    }
}

#[test]
fn measured_temporary_parameters_match_for_both_aliases_before_host_io() {
    let engine = Engine::builder().build().unwrap();
    let cases = [
        ("0", true),
        ("1", true),
        ("8", true),
        ("8192", true),
        ("65535", true),
        ("65536", true),
        ("Неопределено", true),
        ("Null", false),
        ("-1", false),
        ("1.5", false),
        ("\"8\"", false),
        ("Ложь", false),
        ("Истина", false),
        ("Дата(2020,1,2)", false),
        ("Новый Массив", false),
        ("Новый Структура", false),
    ];
    for method in [
        "ФайловыеПотоки.СоздатьВременныйФайл",
        "FileStreams.CreateTempFile",
    ] {
        let cases = cases
            .iter()
            .flat_map(|&(value, success)| {
                [(value.to_owned(), success), (format!("8,{value}"), success)]
            })
            .chain([
                (String::new(), true),
                ("65535,8192".into(), true),
                ("0,0,0".into(), false),
            ]);
        for (arguments, success) in cases {
            let evidence = Rc::new(State::default());
            let mut state = engine
                .state_builder()
                .files(Files(evidence.clone(), true))
                .build();
            let source = format!("Возврат {method}({arguments});");
            let result = state.exec(&source);
            assert_eq!(result.is_ok(), success, "{source}: {result:?}");
            assert_eq!(evidence.creates.get(), usize::from(success), "{source}");
            if let Ok(stream) = result {
                let mut shapes = open_bsl::RuntimeShapes::seeded(vec![], vec![], None);
                let mut context = minimal(&mut shapes);
                let object = stream.object_ref().unwrap();
                assert!(object.type_descriptor().answers_to("FileStream"));
                assert_eq!(
                    object.call_method("Size", &[], &mut context).unwrap(),
                    Value::number_from_i64(0)
                );
                let name = object.get_property("FileName", &mut context).unwrap();
                object.call_method("Close", &[], &mut context).unwrap();
                assert_eq!(object.get_property("ИмяФайла", &mut context).unwrap(), name);
                assert!(
                    object
                        .set_property("FileName", Value::Str("other".into()), &mut context)
                        .is_err()
                );
            }
        }
    }
}
