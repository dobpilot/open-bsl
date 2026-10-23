use bsl_rt::{
    FileOperationError, FileOperationRequest as Request, FileOperationResult as Output, FileSystem,
    perform_file_operation,
};
use std::{
    cell::{Cell, RefCell},
    io,
};

fn measured_formatter(value: &bsl_rt::BslValue, _: Option<&str>) -> bsl_rt::RtResult<String> {
    Ok(match value {
        bsl_rt::BslValue::Number(_) => "17".into(),
        bsl_rt::BslValue::Undefined => String::new(),
        _ => panic!("неожиданное значение тестового форматировщика"),
    })
}

#[derive(Debug, Default)]
struct Files {
    paths: RefCell<Vec<String>>,
    canceled: Cell<bool>,
    cancel_on_change: bool,
    cancel_on_temp: bool,
    separators: Cell<usize>,
    cancel_on_separator: bool,
    metadata: Option<bsl_rt::FileMetadata>,
    metadata_calls: Cell<usize>,
    cancel_on_metadata: bool,
    updates: RefCell<Vec<bsl_rt::FileMetadataUpdate>>,
    update_error: bool,
    change_error: bool,
    temp_error: bool,
    separator_error: bool,
    read_dir_error: bool,
    cancel_on_read_dir: bool,
}

impl Files {
    fn record_update(&self, update: bsl_rt::FileMetadataUpdate) -> io::Result<()> {
        self.updates.borrow_mut().push(update);
        self.canceled.set(self.cancel_on_change);
        if self.update_error {
            Err(io::Error::from(io::ErrorKind::PermissionDenied))
        } else {
            Ok(())
        }
    }
}
impl FileSystem for Files {
    fn set_read_only(&self, _: &str, value: bool) -> io::Result<()> {
        self.record_update(bsl_rt::FileMetadataUpdate::ReadOnly(value))
    }
    fn set_hidden(&self, _: &str, value: bool) -> io::Result<()> {
        self.record_update(bsl_rt::FileMetadataUpdate::Hidden(value))
    }
    fn set_modified(&self, _: &str, value: i64) -> io::Result<()> {
        self.record_update(bsl_rt::FileMetadataUpdate::Modified(value))
    }
    fn read(&self, _: &str) -> io::Result<Vec<u8>> {
        panic!("не читаем файл")
    }
    fn write(&self, _: &str, _: &[u8]) -> io::Result<()> {
        panic!("не пишем файл")
    }
    fn metadata(&self, _: &str) -> io::Result<bsl_rt::FileMetadata> {
        self.metadata_calls.set(self.metadata_calls.get() + 1);
        self.canceled.set(self.cancel_on_metadata);
        self.metadata
            .clone()
            .ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))
    }
    fn open(&self, _: &str, _: bsl_rt::FileOpenOptions) -> io::Result<Box<dyn bsl_rt::FileHandle>> {
        panic!("не открываем файл")
    }
    fn create_dir_all(&self, path: &str) -> io::Result<()> {
        self.paths.borrow_mut().push(path.to_owned());
        self.canceled.set(self.cancel_on_change);
        if self.change_error {
            Err(io::ErrorKind::PermissionDenied.into())
        } else {
            Ok(())
        }
    }
    fn remove_path(&self, path: &str) -> io::Result<()> {
        self.create_dir_all(path)
    }
    fn temporary_directory(&self) -> io::Result<String> {
        self.canceled.set(self.cancel_on_temp);
        if self.temp_error {
            Err(io::ErrorKind::PermissionDenied.into())
        } else {
            Ok("virtual-temp".into())
        }
    }
    fn path_separator(&self) -> io::Result<String> {
        self.separators.set(self.separators.get() + 1);
        if self.cancel_on_separator {
            self.canceled.set(true);
        }
        if self.separator_error {
            Err(io::ErrorKind::PermissionDenied.into())
        } else {
            Ok("|".into())
        }
    }
    fn read_dir<'a>(
        &'a self,
        _: &str,
    ) -> io::Result<Box<dyn Iterator<Item = io::Result<bsl_rt::DirEntry>> + 'a>> {
        if self.cancel_on_read_dir {
            self.canceled.set(true);
        }
        if self.read_dir_error {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
        Ok(Box::new(
            ["a.txt", "b.txt"]
                .into_iter()
                .map(|name| Ok(bsl_rt::DirEntry::new(name, false))),
        ))
    }
}

#[test]
fn async_directory_request_uses_the_session_formatter_for_non_strings() {
    let number = bsl_rt::BslValue::number_from_i64(17);
    assert!(matches!(
        bsl_rt::prepare_file_operation(
            bsl_rt::BuiltinFn::CreateDirectoryAsync,
            std::slice::from_ref(&number),
            measured_formatter,
        ),
        Ok(Request::CreateDirectory(path)) if path == "17"
    ));
    assert_eq!(
        bsl_rt::prepare_create_directory_noop(std::slice::from_ref(&number)),
        None
    );
    let files = Files::default();
    assert!(matches!(
        bsl_rt::call_builtin_create_directory_formatted(
            std::slice::from_ref(&number),
            &files,
            measured_formatter,
        ),
        Ok(bsl_rt::BslValue::Undefined)
    ));
    assert_eq!(&*files.paths.borrow(), &["17"]);

    assert!(matches!(
        bsl_rt::prepare_file_operation(
            bsl_rt::BuiltinFn::CreateDirectoryAsync,
            &[bsl_rt::BslValue::Undefined],
            measured_formatter,
        ),
        Ok(Request::CreateDirectory(path)) if path.is_empty()
    ));
}

#[test]
fn cancellation_wins_over_errors_from_every_global_host_operation() {
    for cancel in [false, true] {
        let search = || bsl_rt::FileSearchRequest {
            path: "root".into(),
            mask: Some("*".into()),
            recursive: false,
        };
        for (request, files, expected_changes) in [
            (
                Request::CreateDirectory("root".into()),
                Files {
                    change_error: true,
                    cancel_on_change: cancel,
                    ..Default::default()
                },
                vec!["root"],
            ),
            (
                Request::Delete {
                    path: "root".into(),
                    mask: None,
                },
                Files {
                    change_error: true,
                    cancel_on_change: cancel,
                    ..Default::default()
                },
                vec!["root"],
            ),
            (
                Request::Delete {
                    path: "root".into(),
                    mask: Some("*".into()),
                },
                Files {
                    change_error: true,
                    cancel_on_change: cancel,
                    ..Default::default()
                },
                vec!["root|a.txt"],
            ),
            (
                Request::TemporaryDirectory,
                Files {
                    temp_error: true,
                    cancel_on_temp: cancel,
                    ..Default::default()
                },
                vec![],
            ),
            (
                Request::TemporaryDirectory,
                Files {
                    separator_error: true,
                    cancel_on_separator: cancel,
                    ..Default::default()
                },
                vec![],
            ),
            (
                Request::CreateDirectory("root".into()),
                Files {
                    separator_error: true,
                    cancel_on_separator: cancel,
                    ..Default::default()
                },
                vec![],
            ),
            (
                Request::Search(search()),
                Files {
                    read_dir_error: true,
                    cancel_on_read_dir: cancel,
                    ..Default::default()
                },
                vec![],
            ),
            (
                Request::Delete {
                    path: "root".into(),
                    mask: Some("*".into()),
                },
                Files {
                    read_dir_error: true,
                    cancel_on_read_dir: cancel,
                    ..Default::default()
                },
                vec![],
            ),
        ] {
            let label = format!("{request:?}");
            let is_search = matches!(request, Request::Search(_));
            let result = perform_file_operation(request, &files, &mut || files.canceled.get());
            if cancel {
                assert!(
                    matches!(result, Err(FileOperationError::Canceled)),
                    "{label}: {result:?}"
                );
            } else if is_search {
                assert!(
                    matches!(result, Ok(Output::Search(_))),
                    "{label}: {result:?}"
                );
            } else {
                assert!(
                    matches!(result, Err(FileOperationError::Io(_))),
                    "{label}: {result:?}"
                );
            }
            assert_eq!(*files.paths.borrow(), expected_changes, "{label}");
        }
    }
}

#[test]
fn metadata_writes_check_cancellation_before_and_after_success_or_error() {
    use bsl_rt::FileMetadataUpdate as Update;
    for update in [
        Update::ReadOnly(true),
        Update::Hidden(false),
        Update::Modified(123),
    ] {
        for update_error in [false, true] {
            let files = Files {
                cancel_on_change: true,
                update_error,
                ..Files::default()
            };
            let request = || Request::UpdateMetadata {
                path: "file".into(),
                update,
            };
            assert!(matches!(
                perform_file_operation(request(), &files, &mut || true),
                Err(FileOperationError::Canceled)
            ));
            assert!(files.updates.borrow().is_empty());
            assert!(matches!(
                perform_file_operation(request(), &files, &mut || files.canceled.get()),
                Err(FileOperationError::Canceled)
            ));
            assert_eq!(*files.updates.borrow(), [update]);
        }
        let files = Files {
            update_error: true,
            ..Files::default()
        };
        assert!(matches!(
            perform_file_operation(
                Request::UpdateMetadata {
                    path: "file".into(),
                    update
                },
                &files,
                &mut || false
            ),
            Err(FileOperationError::Io(_))
        ));
    }
}

#[test]
fn metadata_keeps_errors_and_checks_cancellation_after_host_returns() {
    use bsl_rt::{BslValue, FileMetadataQuery as Query, FixedTimeZone, RtError};
    use std::rc::Rc;
    let files = Rc::new(Files::default());
    let zone = Rc::new(FixedTimeZone::new(0).unwrap());
    for query in [
        Query::Exists,
        Query::IsFile,
        Query::IsDirectory,
        Query::Size,
        Query::ReadOnly,
        Query::Hidden,
        Query::ModificationTime,
        Query::ModificationUniversalTime,
    ] {
        let output = perform_file_operation(
            Request::Metadata {
                path: "missing".into(),
                query,
            },
            files.as_ref(),
            &mut || false,
        )
        .unwrap();
        let result = output.into_value(files.clone(), zone.clone());
        if query == Query::Exists {
            assert_eq!(result, Ok(BslValue::Boolean(false)));
        } else {
            assert!(matches!(result, Err(RtError::IoError(_))));
        }
    }
    let files = Rc::new(Files {
        metadata: Some(bsl_rt::FileMetadata::file(None)),
        ..Files::default()
    });
    for query in [
        Query::Size,
        Query::ReadOnly,
        Query::Hidden,
        Query::ModificationTime,
        Query::ModificationUniversalTime,
    ] {
        let output = perform_file_operation(
            Request::Metadata {
                path: "unknown-fields".into(),
                query,
            },
            files.as_ref(),
            &mut || false,
        )
        .unwrap();
        assert!(matches!(
            output.into_value(files.clone(), zone.clone()),
            Err(RtError::IoError(_))
        ));
    }
    let canceled = Files {
        cancel_on_metadata: true,
        ..Files::default()
    };
    let request = || Request::Metadata {
        path: "missing".into(),
        query: Query::Exists,
    };
    assert!(matches!(
        perform_file_operation(request(), &canceled, &mut || true),
        Err(FileOperationError::Canceled)
    ));
    assert_eq!(canceled.metadata_calls.get(), 0);
    assert!(matches!(
        perform_file_operation(request(), &canceled, &mut || canceled.canceled.get()),
        Err(FileOperationError::Canceled)
    ));
    assert_eq!(canceled.metadata_calls.get(), 1);
}

#[test]
fn results_keep_original_directory_string_and_host_temp_separator() {
    let files = Files::default();
    let Output::String(created) = perform_file_operation(
        Request::CreateDirectory("root/new/../dot/".into()),
        &files,
        &mut || false,
    )
    .unwrap() else {
        panic!("ожидалась строка")
    };
    assert_eq!(created, "root/new/../dot/");
    assert_eq!(&*files.paths.borrow(), &["root|dot"]);
    let Output::String(temp) =
        perform_file_operation(Request::TemporaryDirectory, &files, &mut || false).unwrap()
    else {
        panic!("ожидалась строка")
    };
    assert_eq!(temp, "virtual-temp|");
    assert!(matches!(
        perform_file_operation(
            Request::Delete {
                path: "root".into(),
                mask: None
            },
            &files,
            &mut || false
        )
        .unwrap(),
        Output::Undefined
    ));
}

#[test]
fn directory_creation_normalizes_before_host_io_without_changing_the_result() {
    for (original, normalized) in [
        ("root/missing/../leaf", "root|leaf"),
        ("root/block/../leaf", "root|leaf"),
        ("root/branch\\middle/leaf/", "root|branch|middle|leaf"),
        ("root//branch/./leaf///", "root|branch|leaf"),
        ("root/nul\0/tail", "root|nul"),
        ("root/ spaced /leaf ", "root| spaced |leaf"),
        ("root/tab\t", "root|tab"),
        ("root/next-line\n", "root|next-line"),
        ("root/return\r", "root|return"),
        ("root/next-line-85\u{85}", "root|next-line-85"),
        ("root/nbsp\u{a0}", "root|nbsp"),
        ("root/delete\u{7f}", "root|delete\u{7f}"),
        ("root/zero-width\u{200b}", "root|zero-width\u{200b}"),
    ] {
        let files = Files::default();
        let Output::String(result) = perform_file_operation(
            Request::CreateDirectory(original.into()),
            &files,
            &mut || false,
        )
        .unwrap() else {
            panic!("ожидалась исходная строка");
        };
        assert_eq!(result, original);
        assert_eq!(&*files.paths.borrow(), &[normalized]);
        assert_eq!(files.metadata_calls.get(), 0);
        assert_eq!(files.separators.get(), 1);

        let mut shapes = bsl_rt::RuntimeShapes::seeded(Vec::new(), Vec::new(), None);
        let args = [bsl_rt::BslValue::Str(bsl_rt::BslString::from_str(original))];
        assert!(matches!(
            bsl_rt::call_builtin_files(
                bsl_rt::BuiltinFn::CreateDirectory,
                &args,
                &mut shapes,
                &files
            ),
            Ok(bsl_rt::BslValue::Undefined)
        ));
        assert_eq!(&*files.paths.borrow(), &[normalized, normalized]);
    }

    for path in ["", "\0tail", "root/star*"] {
        let files = Files::default();
        assert!(matches!(
            perform_file_operation(Request::CreateDirectory(path.into()), &files, &mut || false),
            Err(FileOperationError::Io(_))
        ));
        assert!(files.paths.borrow().is_empty());
        assert_eq!(files.metadata_calls.get(), 0);
    }

    let files = Files {
        cancel_on_separator: true,
        ..Default::default()
    };
    assert!(matches!(
        perform_file_operation(
            Request::CreateDirectory("root/leaf".into()),
            &files,
            &mut || files.canceled.get()
        ),
        Err(FileOperationError::Canceled)
    ));
    assert!(files.paths.borrow().is_empty());
    assert_eq!(files.separators.get(), 1);
}

#[test]
fn cancellation_precedes_io_and_never_rolls_back_completed_changes() {
    let files = Files {
        cancel_on_change: true,
        ..Default::default()
    };
    assert!(matches!(
        perform_file_operation(Request::CreateDirectory("root".into()), &files, &mut || {
            true
        }),
        Err(FileOperationError::Canceled)
    ));
    assert!(files.paths.borrow().is_empty());
    assert!(matches!(
        perform_file_operation(Request::CreateDirectory("root".into()), &files, &mut || {
            files.canceled.get()
        }),
        Err(FileOperationError::Canceled)
    ));
    assert_eq!(&*files.paths.borrow(), &["root"]);
    files.canceled.set(false);
    files.paths.borrow_mut().clear();
    assert!(matches!(
        perform_file_operation(
            Request::Delete {
                path: "root".into(),
                mask: Some("*.txt".into())
            },
            &files,
            &mut || files.canceled.get()
        ),
        Err(FileOperationError::Canceled)
    ));
    assert_eq!(&*files.paths.borrow(), &["root|a.txt"]);
}

#[test]
fn cancellation_after_temp_lookup_skips_the_next_host_call() {
    let files = Files {
        cancel_on_temp: true,
        ..Default::default()
    };
    assert!(matches!(
        perform_file_operation(Request::TemporaryDirectory, &files, &mut || files
            .canceled
            .get()),
        Err(FileOperationError::Canceled)
    ));
    assert_eq!(files.separators.get(), 0);
}
