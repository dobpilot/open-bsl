use bsl_rt::{BslObject, BslValue, DirEntry, FileMetadata, FileSystem, FixedTimeZone, RtError};
use std::{
    cell::{Cell, RefCell},
    io,
    rc::Rc,
};

#[derive(Debug, Default)]
struct Files {
    calls: Cell<usize>,
    endless: bool,
    symbolic_branch: bool,
    root_entries: Option<Vec<DirEntry>>,
    invalid_name: Option<&'static str>,
    removed: RefCell<Vec<String>>,
    read_paths: RefCell<Vec<String>>,
    branch_error: Option<io::ErrorKind>,
    star_metadata: Option<FileMetadata>,
}
impl FileSystem for Files {
    fn remove_path(&self, path: &str) -> io::Result<()> {
        self.removed.borrow_mut().push(path.to_owned());
        Ok(())
    }
    fn read(&self, _: &str) -> io::Result<Vec<u8>> {
        panic!("поиск не читает содержимое")
    }
    fn write(&self, _: &str, _: &[u8]) -> io::Result<()> {
        panic!("поиск не пишет")
    }
    fn create_dir_all(&self, _: &str) -> io::Result<()> {
        panic!("поиск не создаёт каталоги")
    }
    fn open(&self, _: &str, _: bsl_rt::FileOpenOptions) -> io::Result<Box<dyn bsl_rt::FileHandle>> {
        panic!("поиск не открывает поток")
    }
    fn path_separator(&self) -> io::Result<String> {
        self.calls.set(self.calls.get() + 1);
        Ok("/".into())
    }
    fn metadata(&self, path: &str) -> io::Result<FileMetadata> {
        self.calls.set(self.calls.get() + 1);
        if path == "root/branch"
            && let Some(kind) = self.branch_error
        {
            return Err(kind.into());
        }
        match path {
            "root" | "root/branch" => Ok(FileMetadata::directory(None)),
            "root/a*" if self.star_metadata.is_some() => Ok(self.star_metadata.clone().unwrap()),
            "root/я.txt" | "root/plain" | "root/branch/deep.txt" => {
                Ok(FileMetadata::file(None).with_size(3))
            }
            _ => Err(io::ErrorKind::NotFound.into()),
        }
    }
    fn read_dir<'a>(
        &'a self,
        path: &str,
    ) -> io::Result<Box<dyn Iterator<Item = io::Result<DirEntry>> + 'a>> {
        self.calls.set(self.calls.get() + 1);
        self.read_paths.borrow_mut().push(path.to_owned());
        if path == "root/branch"
            && let Some(kind) = self.branch_error
        {
            return Err(kind.into());
        }
        let entries = if path == "root" && self.root_entries.is_some() {
            self.root_entries.clone().unwrap()
        } else if let Some(name) = self.invalid_name {
            vec![DirEntry::new(name, true)]
        } else if self.endless {
            vec![DirEntry::new("branch", true)]
        } else {
            match path {
                "root" => vec![
                    DirEntry::new("branch", !self.symbolic_branch)
                        .with_symlink(self.symbolic_branch),
                    DirEntry::new("я.txt", false),
                    DirEntry::new("plain", false),
                ],
                "root/branch" => vec![DirEntry::new("deep.txt", false)],
                "root/branch " | "root/branch /" => vec![DirEntry::new("raw.txt", false)],
                "root/plain" => return Err(io::ErrorKind::NotADirectory.into()),
                "denied" => return Err(io::ErrorKind::PermissionDenied.into()),
                _ => return Err(io::ErrorKind::NotFound.into()),
            }
        };
        Ok(Box::new(entries.into_iter().map(Ok)))
    }
}

fn paths(value: BslValue) -> Vec<String> {
    let BslValue::Object(array) = value else {
        panic!("ожидался массив")
    };
    let BslObject::Array(items) = &*array else {
        panic!("ожидался массив")
    };
    let mut shapes = bsl_rt::RuntimeShapes::seeded(Vec::new(), Vec::new(), None);
    let mut ctx = bsl_rt::CallContext::minimal(&mut shapes, |_, _| unreachable!());
    items
        .borrow()
        .iter()
        .map(|value| {
            let BslValue::Object(object) = value else {
                panic!("ожидался объект")
            };
            let BslObject::Extension(object) = &**object else {
                panic!("ожидался Файл")
            };
            let BslValue::Str(path) = object.get_property("ПолноеИмя", &mut ctx).unwrap()
            else {
                panic!("ожидалась строка")
            };
            path.to_string()
        })
        .collect()
}

#[test]
#[cfg(unix)]
fn recursive_search_follows_directory_links_without_changing_entry_kind() {
    use std::os::unix::fs::symlink;
    let root = std::env::temp_dir().join(format!(
        "open-bsl-search-links-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&root).unwrap();
    struct Scratch(std::path::PathBuf);
    impl Drop for Scratch {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.0).unwrap();
        }
    }
    let scratch = Scratch(root);
    let root = &scratch.0;
    std::fs::create_dir_all(root.join("valid/actual")).unwrap();
    std::fs::create_dir_all(root.join("outside/nested")).unwrap();
    for path in [
        "valid/plain.txt",
        "valid/actual/direct.txt",
        "outside/remote.txt",
        "outside/nested/leaf.txt",
    ] {
        std::fs::write(root.join(path), b"x").unwrap();
    }
    symlink("../outside", root.join("valid/linked-dir")).unwrap();
    symlink("../outside/remote.txt", root.join("valid/linked-file.txt")).unwrap();
    let files = Rc::new(bsl_rt::SystemFileSystem);
    let directory = root.join("valid").to_str().unwrap().to_owned();
    let entry = files
        .read_dir(&directory)
        .unwrap()
        .map(Result::unwrap)
        .find(|entry| entry.name() == "linked-dir")
        .unwrap();
    assert!(!entry.is_dir());
    assert!(entry.is_symlink());
    assert!(!DirEntry::new("legacy", false).is_symlink());
    for (mask, mut expected) in [
        (
            "*",
            vec![
                "actual",
                "actual/direct.txt",
                "linked-dir",
                "linked-dir/nested",
                "linked-dir/nested/leaf.txt",
                "linked-dir/remote.txt",
                "linked-file.txt",
                "plain.txt",
            ],
        ),
        (
            "*.txt",
            vec![
                "actual/direct.txt",
                "linked-dir/nested/leaf.txt",
                "linked-dir/remote.txt",
                "linked-file.txt",
                "plain.txt",
            ],
        ),
    ] {
        // Множества controls.valid.* из oracle; порядок соседей задаёт host.
        let found = paths(
            bsl_rt::find_files(
                &directory,
                Some(mask),
                true,
                files.clone(),
                Rc::new(FixedTimeZone::UTC),
                &mut || Ok(()),
            )
            .unwrap(),
        );
        let mut relative: Vec<_> = found
            .iter()
            .map(|path| path.strip_prefix(&(directory.clone() + "/")).unwrap())
            .collect();
        let first_nested = relative.iter().position(|path| path.contains('/')).unwrap();
        assert!(
            relative[first_nested..]
                .iter()
                .all(|path| path.contains('/'))
        );
        relative.sort_unstable();
        expected.sort_unstable();
        assert_eq!(relative, expected);
    }
    // Цикл завершается отказом host или пределом open-bsl, не частичным успехом.
    symlink("../..", root.join("outside/nested/loop")).unwrap();
    assert!(matches!(
        bsl_rt::find_files(
            &directory,
            Some("*"),
            true,
            files,
            Rc::new(FixedTimeZone::UTC),
            &mut || Ok(())
        ),
        Err(RtError::ResourceLimit(_) | RtError::IoError(_))
    ));
}

#[test]
fn single_threaded_files_do_not_implicitly_grant_background_access() {
    let files = Files::default();
    assert!(files.background_access().is_none());
    assert_eq!(files.calls.get(), 0);
    assert!(files.removed.borrow().is_empty());
}

#[test]
fn system_background_access_observes_the_same_files_across_threads() {
    let root = std::env::temp_dir().join(format!(
        "open-bsl-background-files-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&root).unwrap();
    struct Scratch(std::path::PathBuf);
    impl Drop for Scratch {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.0).unwrap();
        }
    }
    let scratch = Scratch(root);
    let path = scratch.0.join("shared").to_str().unwrap().to_owned();
    let files = bsl_rt::SystemFileSystem;
    files.write(&path, b"before").unwrap();
    let background = files.background_access().unwrap();
    let thread_path = path.clone();
    std::thread::spawn(move || {
        assert_eq!(background.read(&thread_path).unwrap(), b"before");
        background.write(&thread_path, b"after").unwrap();
    })
    .join()
    .unwrap();
    assert_eq!(files.read(&path).unwrap(), b"after");
}

#[test]
fn formatted_deletion_uses_the_supplied_formatter_and_host() {
    let path = BslValue::Str(bsl_rt::BslString::from_str("root"));
    let args = [path, BslValue::Boolean(true)];
    let files = Files::default();
    let format_mask = |value: &BslValue, format: Option<&str>| {
        assert!(matches!(value, BslValue::Boolean(true)));
        assert!(format.is_none());
        Ok("я.txt".into())
    };
    bsl_rt::call_builtin_delete_files_formatted(&args, &files, format_mask, &mut || Ok(()))
        .unwrap();
    assert_eq!(&*files.removed.borrow(), &["root/я.txt"]);

    let untouched = Files::default();
    let result =
        bsl_rt::call_builtin_delete_files_formatted(&args, &untouched, format_mask, &mut || {
            Err(RtError::Canceled)
        });
    assert!(matches!(result, Err(RtError::Canceled)));
    assert_eq!(untouched.calls.get(), 0);
    assert!(untouched.removed.borrow().is_empty());

    let result = bsl_rt::call_builtin_delete_files_formatted(
        &args,
        &untouched,
        |_, _| Err(RtError::DynamicError("ошибка форматировщика".into())),
        &mut || Ok(()),
    );
    assert!(
        matches!(result, Err(RtError::DynamicError(message)) if message == "ошибка форматировщика")
    );
    assert_eq!(untouched.calls.get(), 0);
    assert!(untouched.removed.borrow().is_empty());

    // Прежний вход не имеет форматировщика и сохраняет прежний отказ типу.
    assert!(matches!(
        bsl_rt::call_builtin_delete_files(&args, &untouched, &mut || Ok(())),
        Err(RtError::TypeError { .. })
    ));
    assert_eq!(untouched.calls.get(), 0);
}

#[test]
fn deletion_selects_only_matching_children_and_observes_cancellation() {
    let string = |s| BslValue::Str(bsl_rt::BslString::from_str(s));
    let files = Files::default();
    bsl_rt::call_builtin_delete_files(&[string("root"), string("*.txt")], &files, &mut || Ok(()))
        .unwrap();
    assert_eq!(&*files.removed.borrow(), &["root/я.txt"]);
    files.removed.borrow_mut().clear();
    let result =
        bsl_rt::call_builtin_delete_files(&[string("root"), string("*")], &files, &mut || {
            if files.removed.borrow().is_empty() {
                Ok(())
            } else {
                Err(RtError::Canceled)
            }
        });
    assert!(matches!(result, Err(RtError::Canceled)));
    assert_eq!(&*files.removed.borrow(), &["root/branch"]);
    let untouched = Files::default();
    assert!(matches!(
        bsl_rt::call_builtin_delete_files(&[string("root")], &untouched, &mut || Err(
            RtError::Canceled
        )),
        Err(RtError::Canceled)
    ));
    assert!(untouched.removed.borrow().is_empty());
    assert_eq!(untouched.calls.get(), 0);
}

#[test]
fn missing_or_non_directory_search_roots_return_empty_arrays() {
    // file-search-edges.platform.txt: четыре find.missing/file результата.
    for path in ["missing", "root/plain"] {
        for recursive in [false, true] {
            assert!(
                paths(
                    bsl_rt::find_files(
                        path,
                        Some("*"),
                        recursive,
                        Rc::new(Files::default()),
                        Rc::new(FixedTimeZone::UTC),
                        &mut || Ok(()),
                    )
                    .unwrap()
                )
                .is_empty()
            );
        }
    }
    assert!(
        paths(
            bsl_rt::find_files(
                "denied",
                Some("*"),
                false,
                Rc::new(Files::default()),
                Rc::new(FixedTimeZone::UTC),
                &mut || Ok(()),
            )
            .unwrap()
        )
        .is_empty()
    );
    assert!(matches!(
        bsl_rt::call_builtin_delete_files(
            &[
                BslValue::Str(bsl_rt::BslString::from_str("denied")),
                BslValue::Str(bsl_rt::BslString::from_str("*")),
            ],
            &Files::default(),
            &mut || Ok(())
        ),
        Err(RtError::IoError(_))
    ));
}

#[test]
fn direct_and_recursive_search_return_host_bound_files() {
    let files = Rc::new(Files::default());
    for mask in [None, Some("")] {
        assert_eq!(
            paths(
                bsl_rt::find_files(
                    "root",
                    mask,
                    false,
                    files.clone(),
                    Rc::new(FixedTimeZone::UTC),
                    &mut || Ok(())
                )
                .unwrap()
            ),
            ["root"]
        );
    }
    assert!(
        paths(
            bsl_rt::find_files(
                "missing",
                None,
                false,
                files.clone(),
                Rc::new(FixedTimeZone::UTC),
                &mut || Ok(())
            )
            .unwrap()
        )
        .is_empty()
    );
    assert_eq!(
        paths(
            bsl_rt::find_files(
                "root",
                Some("*.txt"),
                false,
                files.clone(),
                Rc::new(FixedTimeZone::UTC),
                &mut || Ok(())
            )
            .unwrap()
        ),
        ["root/я.txt"]
    );
    assert_eq!(
        paths(
            bsl_rt::find_files(
                "root",
                Some("*.txt"),
                true,
                files,
                Rc::new(FixedTimeZone::UTC),
                &mut || Ok(())
            )
            .unwrap()
        ),
        ["root/я.txt", "root/branch/deep.txt"]
    );
}

#[test]
fn broken_links_stop_matches_and_child_selection_independently() {
    let cases: &[(&[&str], &str, &[&str])] = &[
        (
            &["я.txt", "broken.txt", "tail.txt", "branch"],
            "*",
            &["root/я.txt"],
        ),
        (
            &["я.txt", "broken.txt", "tail.txt", "branch"],
            "*.txt",
            &["root/я.txt"],
        ),
        (
            &["plain", "broken.txt", "tail.txt", "branch"],
            "*.txt",
            &["root/broken.txt", "root/tail.txt"],
        ),
        (
            &["plain", "broken.txt", "tail.txt", "branch"],
            "*",
            &["root/plain"],
        ),
        (
            &["branch", "я.txt", "broken.txt", "tail.txt"],
            "*.txt",
            &["root/я.txt", "root/branch/deep.txt"],
        ),
        (
            &["branch", "я.txt", "broken.txt", "tail.txt"],
            "*",
            &["root/branch", "root/я.txt", "root/branch/deep.txt"],
        ),
        (&["broken.txt", "я.txt", "branch"], "*", &[]),
        (&["broken.txt", "я.txt", "branch"], "*.*", &[]),
        (
            &["broken.txt", "я.txt", "branch"],
            "broken.txt",
            &["root/broken.txt"],
        ),
        (
            &["я.txt", "tail.txt", "broken.txt"],
            "*.txt",
            &["root/я.txt", "root/tail.txt"],
        ),
    ];
    for (names, mask, expected) in cases {
        let files = Rc::new(Files {
            root_entries: Some(
                names
                    .iter()
                    .map(|name| {
                        DirEntry::new(*name, *name == "branch").with_symlink(*name == "broken.txt")
                    })
                    .collect(),
            ),
            ..Files::default()
        });
        let found = paths(
            bsl_rt::find_files(
                "root",
                Some(mask),
                true,
                files,
                Rc::new(FixedTimeZone::UTC),
                &mut || Ok(()),
            )
            .unwrap(),
        );
        assert_eq!(found, *expected, "{names:?}, {mask}");
    }
}

#[test]
fn multiple_and_nonmatching_broken_links_match_the_native_ordered_oracle() {
    const TREE: &str = include_str!(
        "../../../tests/conformance/measure/filesystem/file-search-late-links.host.txt"
    );
    const ORACLE: &str = include_str!(
        "../../../tests/conformance/measure/filesystem/file-search-late-links.platform.txt"
    );

    // Снимок find сохраняет порядок read_dir каждого каталога измеренного
    // дерева. Тест не обращается к оставленному измерителем scratch.
    #[derive(Debug)]
    struct MeasuredTree;
    impl MeasuredTree {
        fn entries() -> impl Iterator<Item = (&'static str, &'static str, &'static str)> {
            TREE.lines().map(|line| {
                let mut fields = line.split('\t');
                (
                    fields.next().unwrap(),
                    fields.next().unwrap(),
                    fields.next().unwrap(),
                )
            })
        }
    }
    impl FileSystem for MeasuredTree {
        fn read(&self, _: &str) -> io::Result<Vec<u8>> {
            panic!("поиск не читает содержимое")
        }
        fn write(&self, _: &str, _: &[u8]) -> io::Result<()> {
            panic!("поиск не пишет")
        }
        fn create_dir_all(&self, _: &str) -> io::Result<()> {
            panic!("поиск не создаёт каталоги")
        }
        fn open(
            &self,
            _: &str,
            _: bsl_rt::FileOpenOptions,
        ) -> io::Result<Box<dyn bsl_rt::FileHandle>> {
            panic!("поиск не открывает поток")
        }
        fn path_separator(&self) -> io::Result<String> {
            Ok("/".into())
        }
        fn metadata(&self, path: &str) -> io::Result<FileMetadata> {
            let relative = path.strip_prefix("<root>/").unwrap();
            match Self::entries().find(|(name, _, _)| *name == relative) {
                Some((_, "d", _)) => Ok(FileMetadata::directory(None)),
                Some((_, "f", _)) => Ok(FileMetadata::file(None).with_size(0)),
                Some((_, "l", target)) => {
                    assert!(target.starts_with("absent-"));
                    Err(io::ErrorKind::NotFound.into())
                }
                None => Err(io::ErrorKind::NotFound.into()),
                other => panic!("неизвестный вид записи {other:?}"),
            }
        }
        fn read_dir<'a>(
            &'a self,
            path: &str,
        ) -> io::Result<Box<dyn Iterator<Item = io::Result<DirEntry>> + 'a>> {
            assert!(self.metadata(path)?.is_dir());
            let prefix = format!("{}/", path.strip_prefix("<root>/").unwrap());
            let entries: Vec<_> = Self::entries()
                .filter_map(|(name, kind, _)| {
                    let child = name.strip_prefix(&prefix)?;
                    if child.contains('/') {
                        return None;
                    }
                    Some(Ok(
                        DirEntry::new(child, kind == "d").with_symlink(kind == "l")
                    ))
                })
                .collect();
            Ok(Box::new(entries.into_iter()))
        }
    }

    assert!(ORACLE.contains("file.end\tfile-search-late-links-1"));
    let mut measured = 0;
    for line in ORACLE.lines().filter(|line| line.starts_with("late.")) {
        let (id, output) = line.split_once('\t').unwrap();
        let (case, request) = id.strip_prefix("late.").unwrap().split_once('.').unwrap();
        let (request, language) = request.rsplit_once('.').unwrap();
        assert!(matches!(language, "ru" | "en"));
        let (mask, mode) = request.rsplit_once('.').unwrap();
        assert!(matches!(mode, "flat" | "recursive"));
        let mut fields = output.strip_prefix("ok|count=").unwrap().split('|');
        let count: usize = fields.next().unwrap().parse().unwrap();
        let expected: Vec<_> = fields.collect();
        assert_eq!(expected.len(), count, "{id}");
        let found = paths(
            bsl_rt::find_files(
                &format!("<root>/{case}"),
                Some(mask),
                mode == "recursive",
                Rc::new(MeasuredTree),
                Rc::new(FixedTimeZone::UTC),
                &mut || Ok(()),
            )
            .unwrap(),
        );
        assert_eq!(found, expected, "{id}");
        measured += 1;
    }
    assert_eq!(measured, 128);
}

#[test]
fn links_with_space_names_match_the_native_ordered_oracle() {
    const TREE: &str = include_str!(
        "../../../tests/conformance/measure/filesystem/file-search-link-spaces.host.txt"
    );
    const ORACLE: &str = include_str!(
        "../../../tests/conformance/measure/filesystem/file-search-link-spaces.platform.txt"
    );

    #[derive(Debug)]
    struct MeasuredTree;
    impl MeasuredTree {
        fn entries() -> impl Iterator<Item = (&'static str, &'static str, &'static str)> {
            TREE.lines().map(|line| {
                let mut fields = line.split('\t');
                (
                    fields.next().unwrap(),
                    fields.next().unwrap(),
                    fields.next().unwrap_or(""),
                )
            })
        }

        fn carrier(path: &str) -> String {
            path.replace("/first space/space link dir", "/targets/linked target")
                .replace("/late space/space link dir", "/targets/linked target")
        }
    }
    impl FileSystem for MeasuredTree {
        fn read(&self, _: &str) -> io::Result<Vec<u8>> {
            panic!("поиск не читает содержимое")
        }
        fn write(&self, _: &str, _: &[u8]) -> io::Result<()> {
            panic!("поиск не пишет")
        }
        fn create_dir_all(&self, _: &str) -> io::Result<()> {
            panic!("поиск не создаёт каталоги")
        }
        fn open(
            &self,
            _: &str,
            _: bsl_rt::FileOpenOptions,
        ) -> io::Result<Box<dyn bsl_rt::FileHandle>> {
            panic!("поиск не открывает поток")
        }
        fn path_separator(&self) -> io::Result<String> {
            Ok("/".into())
        }
        fn metadata(&self, path: &str) -> io::Result<FileMetadata> {
            if let Some(("l", _, target)) = Self::entries().find(|(_, name, _)| *name == path) {
                return if target == "absent target" {
                    Err(io::ErrorKind::NotFound.into())
                } else {
                    Ok(FileMetadata::directory(None))
                };
            }
            let carrier = Self::carrier(path);
            match Self::entries().find(|(_, name, _)| *name == carrier) {
                Some(("d", _, _)) => Ok(FileMetadata::directory(None)),
                Some(("f", _, _)) => Ok(FileMetadata::file(None).with_size(0)),
                Some((kind, _, _)) => panic!("неизвестный вид записи {kind}"),
                None => Err(io::ErrorKind::NotFound.into()),
            }
        }
        fn read_dir<'a>(
            &'a self,
            path: &str,
        ) -> io::Result<Box<dyn Iterator<Item = io::Result<DirEntry>> + 'a>> {
            assert!(self.metadata(path)?.is_dir());
            let prefix = format!("{}/", Self::carrier(path));
            let entries: Vec<_> = Self::entries()
                .filter_map(|(kind, name, _)| {
                    let child = name.strip_prefix(&prefix)?;
                    if child.contains('/') {
                        return None;
                    }
                    Some(Ok(
                        DirEntry::new(child, kind == "d").with_symlink(kind == "l")
                    ))
                })
                .collect();
            Ok(Box::new(entries.into_iter()))
        }
    }

    assert!(ORACLE.contains("file.end\tfile-search-link-spaces-1"));
    let mut measured = 0;
    for line in ORACLE.lines().filter(|line| line.starts_with("spaces.")) {
        let (id, output) = line.split_once('\t').unwrap();
        let (case, request) = id.strip_prefix("spaces.").unwrap().split_once('.').unwrap();
        let (request, language) = request.rsplit_once('.').unwrap();
        assert!(matches!(language, "ru" | "en"));
        let (mask, mode) = request.rsplit_once('.').unwrap();
        assert!(matches!(mode, "flat" | "recursive"));
        let mask = mask.replace("<space>", " ");
        let mut fields = output.strip_prefix("ok|count=").unwrap().split('|');
        let count: usize = fields.next().unwrap().parse().unwrap();
        let expected: Vec<_> = fields.collect();
        assert_eq!(expected.len(), count, "{id}");
        let found = paths(
            bsl_rt::find_files(
                &format!("<root>/{}", case.replace('-', " ")),
                Some(&mask),
                mode == "recursive",
                Rc::new(MeasuredTree),
                Rc::new(FixedTimeZone::UTC),
                &mut || Ok(()),
            )
            .unwrap(),
        );
        assert_eq!(found, expected, "{id}");
        measured += 1;
    }
    assert_eq!(measured, 64);
}

#[test]
fn measured_metadata_errors_match_the_native_ordered_oracle() {
    const TREE: &str = include_str!(
        "../../../tests/conformance/measure/filesystem/file-search-metadata-errors.host.txt"
    );
    const ORACLE: &str = include_str!(
        "../../../tests/conformance/measure/filesystem/file-search-metadata-errors.platform.txt"
    );

    #[derive(Debug)]
    struct MeasuredTree;
    impl MeasuredTree {
        fn entries() -> impl Iterator<Item = (&'static str, &'static str, &'static str)> {
            TREE.lines().map(|line| {
                let mut fields = line.split('\t');
                let kind = fields.next().unwrap();
                let path = fields.next().unwrap();
                let _mode = fields.next().unwrap();
                let target = fields.next().unwrap_or("");
                (kind, path, target)
            })
        }
    }
    impl FileSystem for MeasuredTree {
        fn read(&self, _: &str) -> io::Result<Vec<u8>> {
            panic!("поиск не читает содержимое")
        }
        fn write(&self, _: &str, _: &[u8]) -> io::Result<()> {
            panic!("поиск не пишет")
        }
        fn create_dir_all(&self, _: &str) -> io::Result<()> {
            panic!("поиск не создаёт каталоги")
        }
        fn open(
            &self,
            _: &str,
            _: bsl_rt::FileOpenOptions,
        ) -> io::Result<Box<dyn bsl_rt::FileHandle>> {
            panic!("поиск не открывает поток")
        }
        fn path_separator(&self) -> io::Result<String> {
            Ok("/".into())
        }
        fn metadata(&self, path: &str) -> io::Result<FileMetadata> {
            match Self::entries().find(|(_, name, _)| *name == path) {
                Some(("d", _, _)) => Ok(FileMetadata::directory(None)),
                Some(("f", _, _)) => Ok(FileMetadata::file(None).with_size(0)),
                Some(("l", _, "error loop")) => Err(io::Error::from_raw_os_error(40)),
                Some((kind, _, _)) => panic!("неизвестный вид записи {kind}"),
                None => Err(io::ErrorKind::NotFound.into()),
            }
        }
        fn read_dir<'a>(
            &'a self,
            path: &str,
        ) -> io::Result<Box<dyn Iterator<Item = io::Result<DirEntry>> + 'a>> {
            if path == "<root>/permission/denied dir" {
                return Err(io::ErrorKind::PermissionDenied.into());
            }
            assert!(self.metadata(path)?.is_dir());
            let prefix = format!("{path}/");
            let entries: Vec<_> = Self::entries()
                .filter_map(|(kind, name, _)| {
                    let child = name.strip_prefix(&prefix)?;
                    if child.contains('/') {
                        return None;
                    }
                    Some(Ok(
                        DirEntry::new(child, kind == "d").with_symlink(kind == "l")
                    ))
                })
                .collect();
            Ok(Box::new(entries.into_iter()))
        }
    }

    assert!(ORACLE.contains("file.end\tfile-search-metadata-errors-1"));
    let mut measured = 0;
    for line in ORACLE.lines().filter(|line| line.starts_with("metadata.")) {
        let (id, output) = line.split_once('\t').unwrap();
        let (case, request) = id
            .strip_prefix("metadata.")
            .unwrap()
            .split_once('.')
            .unwrap();
        let (request, language) = request.rsplit_once('.').unwrap();
        assert!(matches!(language, "ru" | "en"));
        let (mask, mode) = request.rsplit_once('.').unwrap();
        assert!(matches!(mode, "flat" | "recursive"));
        let mut fields = output.strip_prefix("ok|count=").unwrap().split('|');
        let count: usize = fields.next().unwrap().parse().unwrap();
        let expected: Vec<_> = fields.collect();
        assert_eq!(expected.len(), count, "{id}");
        let found = paths(
            bsl_rt::find_files(
                &format!("<root>/{case}"),
                Some(mask),
                mode == "recursive",
                Rc::new(MeasuredTree),
                Rc::new(FixedTimeZone::UTC),
                &mut || Ok(()),
            )
            .unwrap(),
        );
        assert_eq!(found, expected, "{id}");
        measured += 1;
    }
    assert_eq!(measured, 24);
}

#[test]
fn whitespace_search_keeps_raw_matching_and_entry_kind_but_trims_descent() {
    for raw_is_directory in [false, true] {
        let files = Rc::new(Files {
            root_entries: Some(vec![
                DirEntry::new("branch ", raw_is_directory),
                DirEntry::new("branch", true),
            ]),
            ..Default::default()
        });
        let found = paths(
            bsl_rt::find_files(
                "root",
                Some("*"),
                true,
                files.clone(),
                Rc::new(FixedTimeZone::UTC),
                &mut || Ok(()),
            )
            .unwrap(),
        );
        let mut expected = vec!["root/branch", "root/branch", "root/branch/deep.txt"];
        if raw_is_directory {
            expected.push("root/branch/deep.txt");
        }
        assert_eq!(found, expected);
        assert!(
            !files
                .read_paths
                .borrow()
                .iter()
                .any(|p| p == "root/branch ")
        );

        assert_eq!(
            paths(
                bsl_rt::find_files(
                    "root",
                    Some("branch "),
                    false,
                    files.clone(),
                    Rc::new(FixedTimeZone::UTC),
                    &mut || Ok(())
                )
                .unwrap()
            ),
            ["root/branch"]
        );
        assert_eq!(
            paths(
                bsl_rt::find_files(
                    "root/branch ",
                    Some("*"),
                    true,
                    files.clone(),
                    Rc::new(FixedTimeZone::UTC),
                    &mut || Ok(())
                )
                .unwrap()
            ),
            ["root/branch/deep.txt"]
        );
        assert_eq!(
            paths(
                bsl_rt::find_files(
                    "root/branch /",
                    Some("*"),
                    true,
                    files.clone(),
                    Rc::new(FixedTimeZone::UTC),
                    &mut || Ok(())
                )
                .unwrap()
            ),
            ["root/branch /raw.txt"]
        );
        assert_eq!(
            paths(
                bsl_rt::find_files(
                    "root/branch ",
                    None,
                    false,
                    files.clone(),
                    Rc::new(FixedTimeZone::UTC),
                    &mut || Ok(())
                )
                .unwrap()
            ),
            ["root/branch"]
        );

        // Удалению передаётся исходное имя, не совпавший после обрезания сосед.
        let args = [
            BslValue::Str(bsl_rt::BslString::from_str("root")),
            BslValue::Str(bsl_rt::BslString::from_str("branch ")),
        ];
        bsl_rt::call_builtin_delete_files(&args, files.as_ref(), &mut || Ok(())).unwrap();
        assert_eq!(&*files.removed.borrow(), &["root/branch "]);
    }
}

#[test]
fn measured_descendant_errors_are_skipped_and_cancellation_wins() {
    for kind in [
        io::ErrorKind::NotFound,
        io::ErrorKind::NotADirectory,
        io::ErrorKind::PermissionDenied,
    ] {
        for name in ["branch ", "branch"] {
            let files = Rc::new(Files {
                root_entries: Some(vec![
                    DirEntry::new(name, true),
                    DirEntry::new("plain", false),
                ]),
                branch_error: Some(kind),
                ..Default::default()
            });
            let found = bsl_rt::find_files(
                "root",
                Some("*"),
                true,
                files.clone(),
                Rc::new(FixedTimeZone::UTC),
                &mut || Ok(()),
            );
            if kind == io::ErrorKind::PermissionDenied
                || (name == "branch " && kind != io::ErrorKind::PermissionDenied)
            {
                assert_eq!(paths(found.unwrap()), ["root/branch", "root/plain"]);
            } else {
                assert!(matches!(found, Err(RtError::IoError(_))));
            }
        }
    }
    let files = Rc::new(Files {
        root_entries: Some(vec![DirEntry::new("branch ", true)]),
        branch_error: Some(io::ErrorKind::NotFound),
        ..Default::default()
    });
    let result = bsl_rt::find_files(
        "root",
        Some("*"),
        true,
        files.clone(),
        Rc::new(FixedTimeZone::UTC),
        &mut || {
            if files.calls.get() >= 3 {
                Err(RtError::Canceled)
            } else {
                Ok(())
            }
        },
    );
    assert!(matches!(result, Err(RtError::Canceled)));
    assert_eq!(&*files.read_paths.borrow(), &["root", "root/branch"]);
}

#[test]
fn cancellation_after_link_metadata_prevents_descent() {
    let files = Rc::new(Files {
        symbolic_branch: true,
        ..Files::default()
    });
    let result = bsl_rt::find_files(
        "root",
        Some("*"),
        true,
        files.clone(),
        Rc::new(FixedTimeZone::UTC),
        &mut || {
            if files.calls.get() >= 3 {
                Err(RtError::Canceled)
            } else {
                Ok(())
            }
        },
    );
    assert!(matches!(result, Err(RtError::Canceled)));
    // Разделитель, корневой итератор и metadata ссылки; дочерний каталог не открыт.
    assert_eq!(files.calls.get(), 3);
}

#[test]
fn cancellation_precedes_io_and_depth_is_bounded() {
    let files = Rc::new(Files::default());
    assert!(matches!(
        bsl_rt::find_files(
            "root",
            Some("*"),
            true,
            files.clone(),
            Rc::new(FixedTimeZone::UTC),
            &mut || Err(RtError::Canceled)
        ),
        Err(RtError::Canceled)
    ));
    assert_eq!(files.calls.get(), 0);
    let files = Rc::new(Files {
        endless: true,
        ..Files::default()
    });
    assert!(matches!(
        bsl_rt::find_files(
            "root",
            Some("nomatch"),
            true,
            files.clone(),
            Rc::new(FixedTimeZone::UTC),
            &mut || Ok(())
        ),
        Err(RtError::ResourceLimit(_))
    ));
    assert!(files.calls.get() <= 257);
}

#[test]
fn cancellation_during_walk_discards_partial_results() {
    let files = Rc::new(Files::default());
    let mut checkpoints = 0;
    let result = bsl_rt::find_files(
        "root",
        Some("*"),
        true,
        files.clone(),
        Rc::new(FixedTimeZone::UTC),
        &mut || {
            checkpoints += 1;
            if checkpoints == 6 {
                Err(RtError::Canceled)
            } else {
                Ok(())
            }
        },
    );
    assert!(matches!(result, Err(RtError::Canceled)));
    // Корень уже прочитан, но перед входом в первый подкаталог пришла отмена.
    assert_eq!(files.calls.get(), 2);
}

#[test]
fn invalid_host_names_never_escape_the_directory() {
    for name in [
        "",
        ".",
        "..",
        "../outside",
        "/outside",
        "nested/file",
        "..\\outside",
    ] {
        let files = Rc::new(Files {
            invalid_name: Some(name),
            ..Files::default()
        });
        let result = bsl_rt::find_files(
            "root",
            Some("*"),
            true,
            files.clone(),
            Rc::new(FixedTimeZone::UTC),
            &mut || Ok(()),
        );
        assert!(matches!(result, Err(RtError::IoError(_))), "{name:?}");
        assert_eq!(files.calls.get(), 2);
    }
}

#[test]
fn sfile_selection_matches_the_server_matrix() {
    let names = [
        ("star", "a*"),
        ("prefix", "*a"),
        ("middle", "a*b"),
        ("double", "a**"),
        ("only", "*"),
        ("extension", "a*.txt"),
        ("tail3", "ab*"),
        ("tail4", "abc*"),
        ("leading3", "*ab"),
        ("leading4", "*abc"),
        ("inner4", "ab*c"),
        ("only2", "**"),
        ("only3", "***"),
    ];
    let oracle = include_str!(
        "../../../tests/conformance/measure/filesystem/file-search-sfile-selection.platform.txt"
    );
    let mut checked = 0;
    for line in oracle.lines() {
        let Some((id, expected)) = line.split_once('\t') else {
            continue;
        };
        let Some((case, kind)) = id.split_once('.') else {
            continue;
        };
        if case == "file" {
            continue;
        }
        let name = names
            .iter()
            .find_map(|(id, name)| (*id == case).then_some(*name))
            .unwrap_or_else(|| panic!("неизвестный случай oracle {case}"));
        let mask = if let Some(length) = kind
            .strip_prefix('q')
            .filter(|length| length.bytes().all(|byte| byte.is_ascii_digit()))
        {
            "?".repeat(length.parse().unwrap())
        } else {
            match kind {
                "all" => "*".to_owned(),
                "name" => name.to_owned(),
                "literal" => name.replace('*', "\\*"),
                "question" => name.replace('*', "?"),
                "wildq.left" => "*?".to_owned(),
                "wildq.right" => "?*".to_owned(),
                "wildq.two" => "*??".to_owned(),
                "wildq.middle" => "?*?".to_owned(),
                _ => panic!("неизвестная маска oracle {kind}"),
            }
        };
        let files = Rc::new(Files {
            root_entries: Some(vec![DirEntry::new(name, false)]),
            ..Files::default()
        });
        let found = paths(
            bsl_rt::find_files(
                "root",
                Some(&mask),
                false,
                files,
                Rc::new(FixedTimeZone::UTC),
                &mut || Ok(()),
            )
            .unwrap(),
        );
        assert_eq!(found.len().to_string(), expected, "{id}");
        if expected == "1" {
            assert_eq!(found, [format!("sfile://sfile://{name}")], "{id}");
        }
        checked += 1;
    }
    assert_eq!(checked, 208);
}

#[test]
fn direct_sfile_path_without_a_mask_is_empty_before_host_access() {
    let files = Rc::new(Files::default());
    let found = bsl_rt::find_files(
        "root/a*",
        None,
        false,
        files.clone(),
        Rc::new(FixedTimeZone::UTC),
        &mut || Ok(()),
    )
    .unwrap();

    assert!(paths(found).is_empty());
    // Платформенная проба `star.path`: после получения разделителя metadata
    // особого пути не запрашивается.
    assert_eq!(files.calls.get(), 1);
}

#[test]
fn measured_sfile_directory_is_never_descended() {
    for (mask, expected) in [("a\\*", 1), ("*", 0), ("*.txt", 0)] {
        let files = Rc::new(Files {
            root_entries: Some(vec![DirEntry::new("a*", true)]),
            ..Files::default()
        });
        let found = paths(
            bsl_rt::find_files(
                "root",
                Some(mask),
                true,
                files.clone(),
                Rc::new(FixedTimeZone::UTC),
                &mut || Ok(()),
            )
            .unwrap(),
        );

        assert_eq!(found.len(), expected, "{mask}");
        assert_eq!(&*files.read_paths.borrow(), &["root"], "{mask}");
    }
}

#[test]
fn measured_sfile_links_have_one_selection_rule_for_all_target_kinds() {
    for metadata in [
        Some(FileMetadata::directory(None)),
        Some(FileMetadata::file(None)),
        None,
    ] {
        for (mask, expected) in [("a\\*", 1), ("a*", 1), ("a?", 1), ("*", 0)] {
            let files = Rc::new(Files {
                root_entries: Some(vec![DirEntry::new("a*", false).with_symlink(true)]),
                star_metadata: metadata.clone(),
                ..Files::default()
            });
            let found = paths(
                bsl_rt::find_files(
                    "root",
                    Some(mask),
                    true,
                    files.clone(),
                    Rc::new(FixedTimeZone::UTC),
                    &mut || Ok(()),
                )
                .unwrap(),
            );

            assert_eq!(found.len(), expected, "{mask}");
            assert_eq!(&*files.read_paths.borrow(), &["root"], "{mask}");
        }
    }
}
