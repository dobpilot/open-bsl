//! Исполняемость двустороннего скрипта замеров фоновых заданий.
//!
//! Скрипт `tests/conformance/measure/measure-background-jobs.bsl`
//! объявлен двусторонним: платформа исполняет его с целями из общего
//! модуля `cfg-src-server/CommonModules`, а open-bsl обязан исполнять
//! его С ТЕМ ЖЕ каталогом общих модулей — иначе пробы отвечают ошибкой
//! возможности и скрипт не является содержательной пробой. Этот runner
//! строит движок из тех же исходников модулей и закрепляет, что каждая
//! проба `JOB.*` отвечает содержательной строкой.
#![cfg(not(target_arch = "wasm32"))]

use std::cell::RefCell;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::rc::Rc;

/// Общий буфер stdout сеанса — как в `job_review_fixes.rs`.
#[derive(Default, Clone)]
struct SharedWriter(Rc<RefCell<Vec<u8>>>);

impl Write for SharedWriter {
    fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
        self.0.borrow_mut().extend_from_slice(buffer);
        Ok(buffer.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("корень репозитория")
}

/// Читает BSL-файл, снимая UTF-8 BOM: исходники платформенного каталога
/// хранятся с сигнатурой.
fn read_bsl(path: &Path) -> String {
    let text = std::fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("не читается {}: {error}", path.display()));
    text.trim_start_matches('\u{feff}').to_string()
}

/// Идентификаторы проб скрипта — строки вида `М("JOB....", ...)`.
fn probe_ids(script: &str) -> Vec<String> {
    script
        .lines()
        .filter_map(|line| {
            let rest = line.trim().strip_prefix("М(\"")?;
            let id = rest.split('"').next()?;
            id.starts_with("JOB.").then(|| id.to_string())
        })
        .collect()
}

/// Скрипт замеров фоновых заданий исполняется open-bsl с тем же
/// каталогом общих модулей, что использует платформа (`cfg-src-server`):
/// каждая проба `JOB.*` печатает ровно одну содержательную строку, и ни
/// одна не отвечает ошибкой возможности.
#[test]
fn the_background_jobs_measure_script_runs_with_the_platform_catalog() {
    let root = repo_root();
    let modules_dir = root.join("tests/conformance/measure/1c/cfg-src-server/CommonModules");
    let mut entries: Vec<PathBuf> = std::fs::read_dir(&modules_dir)
        .unwrap_or_else(|error| panic!("не читается {}: {error}", modules_dir.display()))
        .map(|entry| entry.expect("запись каталога").path())
        .collect();
    entries.sort();
    // Каталог собирается из тех же исходников, что грузит платформа, —
    // рецептом графа с импортами «каждый видит каждого»: на платформе
    // серверный общий модуль зовёт любой другой по имени. Модуль,
    // который open-bsl пока не компилирует (platform-only файловые
    // барьеры на «Новый Файл»), пропускается с записью — кандидаты
    // проверяются накопительно и до неподвижной точки, чтобы порядок
    // обхода не решал судьбу модулей, зовущих друг друга.
    let build_engine = |modules: &[(String, String)]| {
        let recipe = open_bsl::ModuleGraphRecipe {
            modules: modules
                .iter()
                .enumerate()
                .map(|(index, (name, source))| open_bsl::ModuleRecipe {
                    name: name.clone(),
                    source: source.clone(),
                    // Импорты — все РАНЕЕ включённые модули: граф импортов
                    // обязан быть ациклическим, а порядок включения
                    // неподвижной точки даёт топологический порядок.
                    imports: modules[..index]
                        .iter()
                        .map(|(other, _)| (other.clone(), other.clone()))
                        .collect(),
                })
                .collect(),
            eager_init: false,
        };
        open_bsl::Engine::builder().configuration(recipe).build()
    };
    let mut candidates: Vec<(String, String)> = Vec::new();
    for entry in entries {
        let source = entry.join("Ext/Module.bsl");
        if !source.is_file() {
            continue;
        }
        let name = entry
            .file_name()
            .expect("имя модуля")
            .to_string_lossy()
            .into_owned();
        candidates.push((name, read_bsl(&source)));
    }
    let mut included: Vec<(String, String)> = Vec::new();
    let skipped: Vec<(String, String)>;
    loop {
        let mut progressed = false;
        let mut still_out = Vec::new();
        for (name, source) in candidates {
            let mut attempt = included.clone();
            attempt.push((name.clone(), source.clone()));
            match build_engine(&attempt) {
                Ok(_) => {
                    included = attempt;
                    progressed = true;
                }
                Err(error) => still_out.push((name, source, error.to_string())),
            }
        }
        candidates = still_out
            .iter()
            .map(|(name, source, _)| (name.clone(), source.clone()))
            .collect();
        if !progressed || candidates.is_empty() {
            skipped = still_out
                .into_iter()
                .map(|(name, _, error)| (name, error))
                .collect();
            break;
        }
    }
    let included_names: Vec<&str> = included.iter().map(|(name, _)| name.as_str()).collect();
    for required in ["ЗамерыФоновыхЗаданийДвусторонние", "ЗамерыФоновыйСосед"]
    {
        assert!(
            included_names.contains(&required),
            "модуль «{required}» обязан компилироваться open-bsl, \
             включены: {included_names:?}, пропущены: {skipped:?}"
        );
    }
    let engine = build_engine(&included).expect("движок с платформенным каталогом");

    let script_path = root.join("tests/conformance/measure/measure-background-jobs.bsl");
    let script = read_bsl(&script_path);
    let ids = probe_ids(&script);
    assert!(
        !ids.is_empty(),
        "скрипт замеров не содержит ни одной пробы JOB.*"
    );

    let out = SharedWriter::default();
    let mut state = engine.state_builder().stdout(out.clone()).build();
    let module = engine.compile_entry(&script).expect("скрипт компилируется");
    state.run(&module).expect("скрипт исполняется");

    let text = String::from_utf8(out.0.borrow().clone()).expect("вывод UTF-8");

    // Ожидание задаётся ОДНОЙ таблицей на пробу: `Equal` — ответы обеих
    // сторон совпадают дословно, `Diverges` — расхождение с записанной
    // причиной. Платформенная половина не переписывается руками, а
    // читается из снятого вывода `measure-background-jobs.platform.txt`,
    // поэтому файл, классификация и golden не могут разойтись молча.
    enum Expected {
        /// Обе стороны отвечают одинаково; строка — этот общий ответ.
        Equal(&'static str),
        /// Намеренное расхождение: ответ платформы, ответ open-bsl.
        Diverges {
            platform: &'static str,
            open_bsl: &'static str,
        },
    }

    let expected: &[(&str, Expected)] = &[
        (
            "JOB.LIST.FILTER_ORDER",
            Expected::Equal("равная граница содержит наше: да"),
        ),
        ("JOB.MODULE.INIT", Expected::Equal("без сообщений")),
        // У платформенных общих модулей тел НЕТ вовсе: модуль с телом
        // грузится молча и падает при первом обращении. Тела модулей
        // open-bsl — осознанное расширение.
        (
            "JOB.MODULE.INIT.КАСАНИЕ",
            Expected::Diverges {
                platform: "Задание завершено с ошибками; без сообщений; ошибка задания: \
                           Ошибка инициализации модуля: ОбщийМодуль.ЗамерыФоновыйСосед.Модуль",
                open_bsl: "Завершено; отклик: 42",
            },
        ),
        // Цели нет на обеих сторонах (объявить `Асинх` в общем модуле
        // платформы нельзя — он отравляет весь модуль), но тексты отказа
        // свои у каждой стороны.
        (
            "JOB.ASYNC.TARGET",
            Expected::Diverges {
                platform: "ошибка: Ошибка при вызове метода контекста (Выполнить)",
                open_bsl: "ошибка: в модуле «ЗамерыФоновыхЗаданийДвусторонние» нет метода \
                           «АсинхЦель»",
            },
        ),
        (
            "JOB.TEMP.LIFETIME",
            Expected::Equal("повторная запись — ошибка; чтение Неопределено"),
        ),
        (
            "JOB.TEMP.READ_YOUR_WRITES",
            Expected::Equal("задание прочитало: Неопределено"),
        ),
        (
            "JOB.TEMP.STAGED_DELETE",
            Expected::Equal("после terminal вызыватель видит хранимое"),
        ),
        (
            "JOB.TEMP.NESTED_CAPABILITY",
            Expected::Equal("после внука вызыватель видит дедово"),
        ),
        // Файловая база игнорирует чужой `seanceId`; наша семантика —
        // клиент-серверная, её авторитет — замеры `Q78.FOREIGN.*`.
        (
            "JOB.TEMP.CALLER_CLOSE_RACE",
            Expected::Diverges {
                platform: "чтение живое; запись прошла",
                open_bsl: "чтение Неопределено; запись — ошибка",
            },
        ),
    ];

    let platform_path = root.join("tests/conformance/measure/measure-background-jobs.platform.txt");
    let platform_text = std::fs::read_to_string(&platform_path)
        .unwrap_or_else(|error| panic!("не читается {}: {error}", platform_path.display()));

    // Обе стороны разбираются В ТОЧНУЮ таблицу «ID -> ответ»: строка без
    // разделителя, дубль ID и лишний ID — отказ, а не молчаливый пропуск.
    // Сравнение дословное: нормализация пробелов скрыла бы расхождение,
    // ради поиска которого замер и делается.
    let parse = |text: &str, side: &str| -> Vec<(String, String)> {
        let mut rows: Vec<(String, String)> = Vec::new();
        for line in text.lines() {
            if line.is_empty() {
                continue;
            }
            let (id, answer) = line
                .split_once('\t')
                .unwrap_or_else(|| panic!("{side}: строка без табуляции: {line:?}"));
            assert!(
                !id.is_empty() && !id.contains(' '),
                "{side}: негодный идентификатор пробы: {id:?}"
            );
            assert!(
                !rows.iter().any(|(known, _)| known == id),
                "{side}: идентификатор {id} встречается дважды"
            );
            rows.push((id.to_string(), answer.to_string()));
        }
        rows
    };
    let ours = parse(&text, "open-bsl");
    let theirs = parse(&platform_text, "платформа");
    let answer = |rows: &[(String, String)], id: &str, side: &str| -> String {
        rows.iter()
            .find(|(known, _)| known == id)
            .unwrap_or_else(|| panic!("{side}: нет строки пробы {id}"))
            .1
            .clone()
    };

    // Наборы идентификаторов совпадают у скрипта и обеих сторон: лишняя
    // или потерянная строка — расхождение, а не мелочь.
    let expected_ids: Vec<&str> = expected.iter().map(|(id, _)| *id).collect();
    assert_eq!(
        ids, expected_ids,
        "таблица ожиданий разошлась с набором проб скрипта"
    );
    for (side, rows) in [("open-bsl", &ours), ("платформа", &theirs)] {
        let seen: Vec<&str> = rows.iter().map(|(id, _)| id.as_str()).collect();
        assert_eq!(
            seen, expected_ids,
            "{side}: набор строк вывода не совпадает с набором проб скрипта"
        );
    }

    for (id, expectation) in expected {
        let ours = answer(&ours, id, "open-bsl");
        let theirs = answer(&theirs, id, "платформа");
        match expectation {
            Expected::Equal(both) => {
                assert_eq!(ours, *both, "ответ open-bsl на {id} разошёлся с ожидаемым");
                assert_eq!(
                    theirs, *both,
                    "проба {id} объявлена совпадающей, но платформенный вывод другой"
                );
            }
            Expected::Diverges { platform, open_bsl } => {
                assert_eq!(
                    ours, *open_bsl,
                    "ответ open-bsl на {id} разошёлся с ожидаемым"
                );
                assert_eq!(
                    theirs, *platform,
                    "платформенный ответ на {id} разошёлся с записанным в таблице"
                );
                assert_ne!(
                    ours, theirs,
                    "проба {id} объявлена расхождением, но стороны отвечают одинаково"
                );
            }
        }
    }
}
