//! Частичные отказы выбранного host: завершённые удаления не откатываются.
//! Это проверка политики open-bsl, не замена платформенного oracle.

use open_bsl::{DirEntry, Engine, FileHandle, FileMetadata, FileOpenOptions, FileSystem, Value};
use std::{
    collections::BTreeSet,
    io,
    sync::{Arc, Mutex},
};

#[derive(Clone, Copy, Debug, PartialEq)]
enum Failure {
    Enumeration,
    BeforeRemoval,
    AfterRemoval,
    InsideTree,
}

#[derive(Debug)]
struct Tree {
    remaining: BTreeSet<&'static str>,
    attempts: Vec<String>,
}

#[derive(Clone, Debug)]
struct Files {
    failure: Failure,
    tree: Arc<Mutex<Tree>>,
}

impl Files {
    fn new(failure: Failure) -> Self {
        Self {
            failure,
            tree: Arc::new(Mutex::new(Tree {
                remaining: BTreeSet::from(["a.txt", "b.txt", "c.txt"]),
                attempts: Vec::new(),
            })),
        }
    }
}

impl FileSystem for Files {
    fn background_access(&self) -> Option<Arc<dyn FileSystem + Send + Sync>> {
        Some(Arc::new(self.clone()))
    }
    fn path_separator(&self) -> io::Result<String> {
        Ok("/".into())
    }
    fn read(&self, _: &str) -> io::Result<Vec<u8>> {
        panic!("удаление не читает содержимое")
    }
    fn write(&self, _: &str, _: &[u8]) -> io::Result<()> {
        panic!("откат не должен создавать файлы")
    }
    fn create_dir_all(&self, _: &str) -> io::Result<()> {
        panic!("откат не должен создавать каталоги")
    }
    fn open(&self, _: &str, _: FileOpenOptions) -> io::Result<Box<dyn FileHandle>> {
        panic!("удаление не открывает потоки")
    }
    fn metadata(&self, path: &str) -> io::Result<FileMetadata> {
        if path == "root" {
            return Ok(FileMetadata::directory(None));
        }
        if self
            .tree
            .lock()
            .unwrap()
            .remaining
            .contains(path.strip_prefix("root/").unwrap())
        {
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
        let entries = ["a.txt", "b.txt", "c.txt"]
            .into_iter()
            .map(|name| {
                if name == "c.txt" && self.failure == Failure::Enumeration {
                    Err(io::Error::new(
                        io::ErrorKind::PermissionDenied,
                        "enumeration failure",
                    ))
                } else {
                    Ok(DirEntry::new(name, false))
                }
            })
            .collect::<Vec<_>>();
        Ok(Box::new(entries.into_iter()))
    }
    fn remove_path(&self, path: &str) -> io::Result<()> {
        let mut tree = self.tree.lock().unwrap();
        tree.attempts.push(path.into());
        if self.failure == Failure::InsideTree {
            assert_eq!(path, "root");
            tree.remaining.remove("a.txt");
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "tree failure",
            ));
        }
        let name = path.strip_prefix("root/").unwrap();
        if name == "b.txt" {
            if self.failure == Failure::AfterRemoval {
                tree.remaining.remove(name);
            }
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "removal failure",
            ));
        }
        assert!(tree.remaining.remove(name));
        Ok(())
    }
}

#[test]
fn partial_deletion_preserves_effects_stops_at_the_first_failure_and_keeps_the_host() {
    for failure in [
        Failure::Enumeration,
        Failure::BeforeRemoval,
        Failure::AfterRemoval,
        Failure::InsideTree,
    ] {
        let arguments = if failure == Failure::InsideTree {
            "\"root\""
        } else {
            "\"root\", \"*.txt\""
        };
        for (method, asynchronous) in [
            ("УдалитьФайлы", false),
            ("DeleteFiles", false),
            ("УдалитьФайлыАсинх", true),
            ("DeleteFilesAsync", true),
        ] {
            for dynamic in [false, true] {
                let call = format!("{method}({arguments})");
                let operation = match (asynchronous, dynamic) {
                    (false, false) => format!("{call};"),
                    (false, true) => format!("Выполнить(\"{};\");", call.replace('"', "\"\"")),
                    (true, false) => format!(
                        "ОбещаниеУдаления = {call}; ОтветУдаления = Ждать ОбещаниеУдаления;"
                    ),
                    (true, true) => format!(
                        "ОбещаниеУдаления = Вычислить(\"{}\"); ОтветУдаления = Ждать ОбещаниеУдаления;",
                        call.replace('"', "\"\"")
                    ),
                };
                let repeat = if asynchronous {
                    r#"
    Попытка
        ОтветПовтора = Ждать ОбещаниеУдаления;
        ИтогУдаления.Добавить("unexpected repeat success");
    Исключение
        ИтогУдаления.Добавить(ИнформацияОбОшибке().Описание);
    КонецПопытки;
"#
                } else {
                    ""
                };
                let source = format!(
                    r#"
Перем ИтогУдаления;
Асинх Процедура ПроверитьУдаление()
    Попытка
        {operation}
        ИтогУдаления.Добавить("unexpected success");
    Исключение
        ИтогУдаления.Добавить(ИнформацияОбОшибке().Описание);
    КонецПопытки;
{repeat}
КонецПроцедуры
ИтогУдаления = Новый Массив;
ПроверитьУдаление();
Возврат ИтогУдаления;
"#
                );
                for optimizations in [
                    bsl_compiler::Optimizations::default(),
                    bsl_compiler::Optimizations::all(),
                ] {
                    let engine = Engine::builder()
                        .optimizations(optimizations)
                        .build()
                        .unwrap();
                    let module = engine.compile(&source).unwrap();
                    let loaded = engine.load_bytecode(&module.bytecode().unwrap()).unwrap();
                    for module in [&module, &loaded] {
                        let files = Files::new(failure);
                        let value = engine
                            .state_builder()
                            .files(files.clone())
                            .build()
                            .run(module)
                            .unwrap();
                        let names = bsl_rt::NameInterner::default();
                        let message = value.get_index(&Value::number_from_i64(0), &names).unwrap();
                        if asynchronous {
                            assert_eq!(
                                value.get_index(&Value::number_from_i64(1), &names).unwrap(),
                                message
                            );
                        }
                        let Value::Str(message) = message else {
                            panic!("ожидалась строка ошибки")
                        };
                        let message = message.to_string();
                        let (error, attempts, remaining) = match failure {
                            Failure::Enumeration => (
                                "enumeration failure",
                                vec![],
                                vec!["a.txt", "b.txt", "c.txt"],
                            ),
                            Failure::BeforeRemoval => (
                                "removal failure",
                                vec!["root/a.txt", "root/b.txt"],
                                vec!["b.txt", "c.txt"],
                            ),
                            Failure::AfterRemoval => (
                                "removal failure",
                                vec!["root/a.txt", "root/b.txt"],
                                vec!["c.txt"],
                            ),
                            Failure::InsideTree => {
                                ("tree failure", vec!["root"], vec!["b.txt", "c.txt"])
                            }
                        };
                        assert!(
                            message.contains(error),
                            "{method}, {failure:?}, dynamic={dynamic}: {message}"
                        );
                        let tree = files.tree.lock().unwrap();
                        assert_eq!(tree.attempts, attempts);
                        assert_eq!(
                            tree.remaining.iter().copied().collect::<Vec<_>>(),
                            remaining
                        );
                    }
                }
            }
        }
    }
}
