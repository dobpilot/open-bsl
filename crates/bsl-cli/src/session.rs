//! Host-политика CLI и завершение запуска до выхода процесса.

use bsl_rt::{BslValue, TemporaryFileResource};
use std::io;

pub(crate) fn builder(engine: &open_bsl::Engine) -> open_bsl::StateBuilder {
    engine
        .state_builder()
        .application_launcher(bsl_rt::SystemApplicationLauncher)
        .temporary_file_cleanup(cleanup)
}

pub(crate) fn cleanup(resources: Vec<Box<dyn TemporaryFileResource>>) -> io::Result<()> {
    // Ошибка одного ресурса не лишает остальные попытки удаления. Путь
    // не используется: право очистки остаётся у создавшего ресурс host.
    let mut first_error = None;
    for resource in resources {
        if let Err(error) = resource.remove()
            && first_error.is_none()
        {
            first_error = Some(error);
        }
    }
    first_error.map_or(Ok(()), Err)
}

pub(crate) fn finish(state: open_bsl::State, outcome: Result<BslValue, open_bsl::Error>) -> i32 {
    let code = match outcome {
        Ok(BslValue::Undefined) => 0,
        Ok(value) => {
            crate::print_value(&value);
            0
        }
        Err(open_bsl::Error::Runtime(error)) => {
            eprintln!("ошибка выполнения: {error}");
            1
        }
        Err(error) => {
            eprintln!("ошибка выполнения: {error}");
            1
        }
    };
    // В частности, ошибка BSL не должна привести к process::exit с живым State.
    drop(state);
    code
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::{cell::RefCell, io::Write, rc::Rc};

    #[derive(Debug)]
    pub(crate) struct Resource {
        pub(crate) removed: Rc<RefCell<Vec<usize>>>,
        pub(crate) id: usize,
        pub(crate) failure: bool,
    }

    impl TemporaryFileResource for Resource {
        fn path(&self) -> &str {
            "секретный-путь"
        }

        fn remove(self: Box<Self>) -> io::Result<()> {
            self.removed.borrow_mut().push(self.id);
            if self.failure {
                let kind = if self.id == 0 {
                    io::ErrorKind::PermissionDenied
                } else {
                    io::ErrorKind::Other
                };
                Err(io::Error::new(kind, self.path()))
            } else {
                Ok(())
            }
        }
    }

    #[derive(Clone)]
    struct Output(Rc<RefCell<Vec<u8>>>);
    impl Write for Output {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.0.borrow_mut().extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn finishing_source_and_loaded_bytecode_cleans_every_resource_before_returning() {
        let engine = crate::engine().unwrap();
        for script in ["Возврат;", "ВызватьИсключение \"ошибка BSL\";"]
        {
            let source = engine.compile(script).unwrap();
            let listing = engine.image_bytecode(&source, Some(script)).unwrap();
            let loaded = engine.load_bytecode(&listing).unwrap();
            for module in [&source, &loaded] {
                for failure in [false, true] {
                    let diagnostic = Rc::new(RefCell::new(Vec::new()));
                    let mut state = builder(&engine).stderr(Output(diagnostic.clone())).build();
                    let registry = state.temporary_files();
                    let removed = Rc::new(RefCell::new(Vec::new()));
                    for id in 0..3 {
                        registry
                            .register(Box::new(Resource {
                                removed: removed.clone(),
                                id,
                                failure: failure && id < 2,
                            }))
                            .unwrap();
                    }
                    let outcome = state.run(module);
                    let code = i32::from(outcome.is_err());
                    assert!(removed.borrow().is_empty());
                    assert_eq!(finish(state, outcome), code);
                    assert_eq!(*removed.borrow(), [0, 1, 2]);
                    assert!(registry.take_pending().is_empty());
                    let late = registry.register(Box::new(Resource {
                        removed: removed.clone(),
                        id: 3,
                        failure: false,
                    }));
                    assert!(late.is_err());
                    drop(late);
                    assert_eq!(*removed.borrow(), [0, 1, 2]);
                    assert_eq!(
                        String::from_utf8(diagnostic.borrow().clone()).unwrap(),
                        if failure {
                            "Ошибка очистки временных файлов: PermissionDenied\n"
                        } else {
                            ""
                        }
                    );
                }
            }
        }
        assert!(cleanup(Vec::new()).is_ok());
    }
}
