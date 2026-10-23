//! Передача собственных временных ресурсов host, без файловой системы процесса.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::io::{self, Write};
use std::rc::Rc;

use open_bsl::{Engine, TemporaryFileRegistry, TemporaryFileResource, Value};

type Files = Rc<RefCell<BTreeMap<String, u64>>>;

#[derive(Debug)]
struct Resource {
    path: String,
    id: u64,
    files: Files,
}

impl TemporaryFileResource for Resource {
    fn path(&self) -> &str {
        &self.path
    }

    fn remove(self: Box<Self>) -> io::Result<()> {
        // Виртуальный host сравнивает и удаляет в одной операции.
        // Это не модель атомарности std::fs::remove_file.
        let mut files = self.files.borrow_mut();
        if files.get(&self.path) == Some(&self.id) {
            files.remove(&self.path);
        }
        Ok(())
    }
}

fn register(registry: &TemporaryFileRegistry, files: &Files, path: &str, id: u64) {
    files.borrow_mut().insert(path.into(), id);
    registry
        .register(Box::new(Resource {
            path: path.into(),
            id,
            files: files.clone(),
        }))
        .unwrap();
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
fn cleanup_is_session_local_and_not_an_invocation_epilogue() {
    let engine = Engine::builder().build().unwrap();
    let files = Files::default();
    let batches = Rc::new(RefCell::new(Vec::new()));
    let seen = batches.clone();
    let mut first = engine
        .state_builder()
        .temporary_file_cleanup(move |resources| {
            seen.borrow_mut().push(resources.len());
            for resource in resources {
                resource.remove()?;
            }
            Ok(())
        })
        .build();
    let second = engine.new_state();
    register(&first.temporary_files(), &files, "first", 1);
    register(&second.temporary_files(), &files, "second", 2);
    assert_eq!(
        first.exec("Возврат 7;").unwrap(),
        Value::Number(open_bsl::BslNumber::from_i64(7))
    );
    assert!(first.exec("ВызватьИсключение \"ошибка\";").is_err());
    assert_eq!(files.borrow().len(), 2);
    assert!(batches.borrow().is_empty());
    first.cleanup_temporary_files();
    first.cleanup_temporary_files();
    assert_eq!(*batches.borrow(), vec![1]);
    assert_eq!(files.borrow().len(), 1);
    register(&first.temporary_files(), &files, "third", 3);
    drop(first);
    assert_eq!(*batches.borrow(), vec![1, 1]);
    drop(second);
    assert_eq!(files.borrow().get("second"), Some(&2));
}

#[test]
fn cleanup_never_converts_a_saved_path_into_deletion_authority() {
    let engine = Engine::builder().build().unwrap();
    let files = Files::default();
    let mut state = engine
        .state_builder()
        .temporary_file_cleanup(|resources| {
            for resource in resources {
                resource.remove()?;
            }
            Ok(())
        })
        .build();
    register(&state.temporary_files(), &files, "replaced", 1);
    register(&state.temporary_files(), &files, "renamed", 2);
    files.borrow_mut().insert("replaced".into(), 99);
    let id = files.borrow_mut().remove("renamed").unwrap();
    files.borrow_mut().insert("new-name".into(), id);
    state.cleanup_temporary_files();
    assert_eq!(files.borrow().get("replaced"), Some(&99));
    assert_eq!(files.borrow().get("new-name"), Some(&2));
}

#[test]
fn closing_state_returns_late_resources_and_does_not_scan_old_ones() {
    let engine = Engine::builder().build().unwrap();
    let files = Files::default();
    let state = engine.new_state();
    let registry = state.temporary_files();
    register(&registry, &files, "old", 1);
    drop(state);
    let resource = registry
        .register(Box::new(Resource {
            path: "late".into(),
            id: 2,
            files: files.clone(),
        }))
        .unwrap_err();
    assert_eq!(resource.path(), "late");
    drop(resource);
    drop(engine.new_state());
    assert_eq!(files.borrow().get("old"), Some(&1));
}

#[test]
fn callback_failure_is_diagnostic_and_does_not_replace_bsl_result() {
    for source in ["Возврат 7;", "ВызватьИсключение \"BSL failure\";"] {
        let engine = Engine::builder().build().unwrap();
        let output = Output(Rc::default());
        let mut state = engine
            .state_builder()
            .stderr(output.clone())
            .temporary_file_cleanup(|_| {
                Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "secret-path",
                ))
            })
            .build();
        register(&state.temporary_files(), &Files::default(), "file", 1);
        let result = state.exec(source);
        state.cleanup_temporary_files();
        assert_eq!(result.is_ok(), source.starts_with("Возврат"));
        assert!(!output.0.borrow().is_empty());
        assert!(!String::from_utf8_lossy(&output.0.borrow()).contains("secret-path"));
        let diagnostic = output.0.borrow().clone();
        drop(state);
        assert_eq!(*output.0.borrow(), diagnostic);
    }
}

#[test]
fn drop_reports_cleanup_errors_after_bsl_failure() {
    let output = Output(Rc::default());
    let engine = Engine::builder().build().unwrap();
    let mut state = engine
        .state_builder()
        .stderr(output.clone())
        .temporary_file_cleanup(|_| Err(io::ErrorKind::PermissionDenied.into()))
        .build();
    register(&state.temporary_files(), &Files::default(), "file", 1);
    assert!(state.exec("ВызватьИсключение \"BSL failure\";").is_err());
    assert!(output.0.borrow().is_empty());
    drop(state);
    assert!(String::from_utf8_lossy(&output.0.borrow()).contains("PermissionDenied"));
}

#[test]
fn callback_can_register_the_next_batch_without_a_refcell_borrow() {
    let engine = Engine::builder().build().unwrap();
    let holder = Rc::new(RefCell::new(None::<TemporaryFileRegistry>));
    let callback_holder = holder.clone();
    let files = Files::default();
    let callback_files = files.clone();
    let mut calls = 0;
    let mut state = engine
        .state_builder()
        .temporary_file_cleanup(move |resources| {
            calls += 1;
            for resource in resources {
                resource.remove()?;
            }
            if calls == 1 {
                register(
                    callback_holder.borrow().as_ref().unwrap(),
                    &callback_files,
                    "next",
                    2,
                );
            }
            Ok(())
        })
        .build();
    *holder.borrow_mut() = Some(state.temporary_files());
    register(&state.temporary_files(), &files, "first", 1);
    state.cleanup_temporary_files();
    assert_eq!(files.borrow().get("next"), Some(&2));
    drop(state);
    assert!(files.borrow().is_empty());
}
