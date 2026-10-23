//! Гонки после снимка целей удаления принадлежат контракту host.

use bsl_rt::{
    BslString, BslValue, DirEntry, FileHandle, FileMetadata, FileOpenOptions, FileSystem,
    call_builtin_delete_files,
};
use std::{cell::RefCell, collections::BTreeSet, io};

#[derive(Clone, Copy, Debug)]
enum Race {
    LateEntry,
    VanishedSelected,
}

#[derive(Debug)]
struct Files {
    race: Race,
    remaining: RefCell<BTreeSet<&'static str>>,
    attempts: RefCell<Vec<String>>,
}

impl Files {
    fn new(race: Race) -> Self {
        Self {
            race,
            remaining: RefCell::new(BTreeSet::from(["a.txt", "b.txt", "c.txt"])),
            attempts: RefCell::new(Vec::new()),
        }
    }
}

impl FileSystem for Files {
    fn path_separator(&self) -> io::Result<String> {
        Ok("/".into())
    }

    fn read(&self, _: &str) -> io::Result<Vec<u8>> {
        panic!("удаление не читает содержимое")
    }

    fn write(&self, _: &str, _: &[u8]) -> io::Result<()> {
        panic!("удаление не записывает содержимое")
    }

    fn create_dir_all(&self, _: &str) -> io::Result<()> {
        panic!("удаление не создаёт каталоги")
    }

    fn open(&self, _: &str, _: FileOpenOptions) -> io::Result<Box<dyn FileHandle>> {
        panic!("удаление не открывает поток")
    }

    fn metadata(&self, path: &str) -> io::Result<FileMetadata> {
        if path == "root" {
            return Ok(FileMetadata::directory(None));
        }
        let name = path.strip_prefix("root/").unwrap();
        if self.remaining.borrow().contains(name) {
            Ok(FileMetadata::file(None))
        } else {
            Err(io::ErrorKind::NotFound.into())
        }
    }

    fn read_dir<'a>(
        &'a self,
        path: &str,
    ) -> io::Result<Box<dyn Iterator<Item = io::Result<DirEntry>> + 'a>> {
        assert_eq!(path, "root");
        Ok(Box::new(
            ["a.txt", "b.txt", "c.txt"]
                .into_iter()
                .map(|name| Ok(DirEntry::new(name, false))),
        ))
    }

    fn remove_path(&self, path: &str) -> io::Result<()> {
        self.attempts.borrow_mut().push(path.into());
        let name = path.strip_prefix("root/").unwrap();
        let mut remaining = self.remaining.borrow_mut();
        if name == "a.txt" {
            match self.race {
                Race::LateEntry => {
                    remaining.insert("late.txt");
                }
                Race::VanishedSelected => {
                    remaining.remove("b.txt");
                }
            }
        }
        if remaining.remove(name) {
            Ok(())
        } else {
            Err(io::ErrorKind::NotFound.into())
        }
    }
}

fn args() -> [BslValue; 2] {
    [
        BslValue::Str(BslString::from_str("root")),
        BslValue::Str(BslString::from_str("*.txt")),
    ]
}

#[test]
fn entry_created_after_selection_is_not_deleted() {
    let files = Files::new(Race::LateEntry);
    assert!(call_builtin_delete_files(&args(), &files, &mut || Ok(())).is_ok());
    assert_eq!(
        files.attempts.into_inner(),
        ["root/a.txt", "root/b.txt", "root/c.txt"]
    );
    assert_eq!(files.remaining.into_inner(), BTreeSet::from(["late.txt"]));
}

#[test]
fn vanished_selected_entry_stops_without_reenumeration_or_rollback() {
    let files = Files::new(Race::VanishedSelected);
    assert!(call_builtin_delete_files(&args(), &files, &mut || Ok(())).is_err());
    assert_eq!(files.attempts.into_inner(), ["root/a.txt", "root/b.txt"]);
    assert_eq!(files.remaining.into_inner(), BTreeSet::from(["c.txt"]));
}
