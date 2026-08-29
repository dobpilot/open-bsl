//! Тест, который не даёт реестру открытых вопросов разъехаться с кодом и
//! со скриптом замеров.
//!
//! Без него реестр протухает за месяц: метку в коде поправят, а запись
//! останется; вопрос закроют замером, а строка в скрипте продолжит
//! печататься; новый спорный выбор пометят комментарием — и он не попадёт
//! в сеанс у платформы, потому что его никто не увидит.
//!
//! Сверка идёт В ОБЕ СТОРОНЫ и по трём источникам:
//!
//! * метки `НЕ ИЗМЕРЕНО(ИД)` в `crates/**/*.rs`;
//! * `bsl_rt::open_questions::OPEN_QUESTIONS` и `MEASURED_ANCHORS`;
//! * ИД-строки в `tests/conformance/measure/*.bsl`.
//!
//! Регулярок нет намеренно: поиск подстроки и разбор до `)` — и никакой
//! зависимости ради одного теста.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use bsl_rt::open_questions::{MEASURED_ANCHORS, OPEN_QUESTIONS};

/// Собирается по кускам, чтобы этот файл сам не выглядел как метка: он
/// исключён из обхода (см. `EXEMPT`), но полагаться на одно исключение,
/// когда можно не давать поводов, незачем.
fn marker() -> String {
    format!("{} {}", "НЕ", "ИЗМЕРЕНО")
}

/// Файлы, где слова метки встречаются как ТЕКСТ (документация формата), а
/// не как сама метка.
const EXEMPT: &[&str] = &[
    "crates/bsl-rt/src/open_questions.rs",
    "crates/bsl-rt/tests/open_questions_registry.rs",
];

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("корень репозитория недостижим из CARGO_MANIFEST_DIR")
}

fn rust_sources(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries =
        fs::read_dir(dir).unwrap_or_else(|e| panic!("не читается {}: {e}", dir.display()));
    for entry in entries {
        let path = entry.expect("битая запись каталога").path();
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        if path.is_dir() {
            // `target` может лежать и внутри крейта, если его собирали
            // отдельно; своих исходников там нет, зато есть чужие.
            if name != "target" {
                rust_sources(&path, out);
            }
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

/// Все метки файла: `(ИД, номер строки)`. Метка без `(ИД)` — ошибка, а не
/// пропуск: именно так новый неразмеченный комментарий и ловится.
fn markers_in(text: &str, path_for_msg: &str) -> Vec<(String, usize)> {
    let marker = marker();
    let mut out = Vec::new();
    for (line_no, line) in text.lines().enumerate() {
        let mut rest = line;
        while let Some(pos) = rest.find(&marker) {
            let after = &rest[pos + marker.len()..];
            let id = after
                .strip_prefix('(')
                .and_then(|s| s.find(')').map(|end| &s[..end]))
                .unwrap_or_else(|| {
                    panic!(
                        "{path_for_msg}:{}: метка без идентификатора. Нужен вид \
                         `{marker}(ОБЛАСТЬ.ВОПРОС)`, а ИД — завести в \
                         bsl_rt::open_questions и в measure-all.bsl",
                        line_no + 1
                    )
                });
            assert!(
                !id.is_empty(),
                "{path_for_msg}:{}: пустой идентификатор метки",
                line_no + 1
            );
            out.push((id.to_string(), line_no + 1));
            rest = after;
        }
    }
    out
}

/// ИД-строки скрипта замеров: строковый литерал целиком состоящий из
/// заглавной латиницы, цифр, точек и подчёркиваний, содержащий хотя бы одну
/// точку И хотя бы одну букву. Под это правило подходят и
/// `М("DATE.ADD_MONTH_CLAMP", ...)`, и ручные
/// `Сообщить("EXEC...." + Символ(9) + ...)` без обёртки, а форматные строки
/// (`"ЧГ=0; ЧРД=."`, `"ДЛФ=Д"`) — нет: они на кириллице и со знаком `=`.
///
/// Буква требуется не для красоты: без неё под правило попадает номер
/// версии XML («1.0») — обычный аргумент, а не идентификатор.
fn script_ids(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.trim_start();
        if line.starts_with("//") {
            continue;
        }
        for piece in string_literals(line) {
            let segments_filled = piece.split('.').all(|s| !s.is_empty());
            if piece.contains('.')
                && segments_filled
                && piece.chars().any(|c| c.is_ascii_uppercase())
                && piece
                    .chars()
                    .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '.' || c == '_')
            {
                out.push(piece);
            }
        }
    }
    out
}

/// Строковые литералы строки исходника. Разбор именно посимвольный, а не
/// `split('"')`: в BSL кавычка внутри строки удваивается, и наивное
/// разбиение рвёт такой литерал на куски. До XML это было незаметно —
/// удвоенных кавычек в замерах просто не встречалось, — а разметка состоит
/// из них целиком (`"<а х=""1""/>"`), и обломок вроде `1.0` уходил в ИД.
fn string_literals(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] != '"' {
            i += 1;
            continue;
        }
        i += 1;
        let mut lit = String::new();
        while i < chars.len() {
            if chars[i] == '"' {
                if chars.get(i + 1) == Some(&'"') {
                    lit.push('"');
                    i += 2;
                    continue;
                }
                i += 1;
                break;
            }
            lit.push(chars[i]);
            i += 1;
        }
        out.push(lit);
    }
    out
}

#[test]
fn open_questions_registry_matches_source_markers() {
    let root = repo_root();

    let mut sources = Vec::new();
    rust_sources(&root.join("crates"), &mut sources);
    sources.sort();

    // ИД -> где встретился в коде.
    let mut in_code: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for path in &sources {
        let rel = path
            .strip_prefix(&root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");
        if EXEMPT.contains(&rel.as_str()) {
            continue;
        }
        let text = fs::read_to_string(path).unwrap_or_else(|e| panic!("не читается {rel}: {e}"));
        for (id, line) in markers_in(&text, &rel) {
            in_code.entry(id).or_default().push(format!("{rel}:{line}"));
        }
    }

    let known: Vec<&str> = OPEN_QUESTIONS.iter().map(|q| q.id).collect();

    // 1. Каждая метка в коде есть в реестре.
    let unknown: Vec<String> = in_code
        .iter()
        .filter(|(id, _)| !known.contains(&id.as_str()))
        .map(|(id, at)| format!("{id} ({})", at.join(", ")))
        .collect();
    assert!(
        unknown.is_empty(),
        "метки с ИД, которых нет в bsl_rt::open_questions::OPEN_QUESTIONS:\n  {}",
        unknown.join("\n  ")
    );

    // 2. Каждый ИД реестра встречается в коде хотя бы раз.
    let orphans: Vec<&str> = known
        .iter()
        .copied()
        .filter(|id| !in_code.contains_key(*id))
        .collect();
    assert!(
        orphans.is_empty(),
        "ИД реестра осиротели — в коде нет ни одной метки:\n  {}\n\
         Либо вопрос закрыт замером (тогда убрать запись из реестра и строку \
         из measure-all.bsl), либо метку потеряли при правке.",
        orphans.join("\n  ")
    );
}

#[test]
fn measure_script_covers_every_registry_id() {
    let root = repo_root();
    let dir = root.join("tests/conformance/measure");

    let mut ids: Vec<(String, String)> = Vec::new(); // (ИД, файл)
    let mut files: Vec<PathBuf> = fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("не читается {}: {e}", dir.display()))
        .map(|e| e.expect("битая запись каталога").path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("bsl"))
        .collect();
    files.sort();
    assert!(
        !files.is_empty(),
        "в {} нет скриптов замеров",
        dir.display()
    );

    for path in &files {
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let text = fs::read_to_string(path).unwrap_or_else(|e| panic!("не читается {name}: {e}"));
        for id in script_ids(&text) {
            ids.push((id, name.clone()));
        }
    }

    let questions: Vec<&str> = OPEN_QUESTIONS.iter().map(|q| q.id).collect();
    let anchors: Vec<&str> = MEASURED_ANCHORS.iter().map(|a| a.id).collect();

    // 1. Каждый ИД скрипта известен: либо вопрос, либо якорь, либо
    //    ПОДЗАМЕР существующего вопроса (`ВОПРОС.ЧТО_ИМЕННО`): один и тот
    //    же вопрос бывает нужно задать платформе с нескольких сторон.
    let unknown: Vec<String> = ids
        .iter()
        .filter(|(id, _)| {
            !questions.contains(&id.as_str())
                && !anchors.contains(&id.as_str())
                && !questions
                    .iter()
                    .any(|q| id.starts_with(q) && id.as_bytes().get(q.len()) == Some(&b'.'))
        })
        .map(|(id, file)| format!("{id} ({file})"))
        .collect();
    assert!(
        unknown.is_empty(),
        "ИД в скриптах замеров, которых нет в реестре:\n  {}",
        unknown.join("\n  ")
    );

    // 2. Каждый ИД реестра и каждый якорь — РОВНО ОДНА строка.
    for expected in questions.iter().chain(anchors.iter()) {
        let hits: Vec<&str> = ids
            .iter()
            .filter(|(id, _)| id == expected)
            .map(|(_, file)| file.as_str())
            .collect();
        assert_eq!(
            hits.len(),
            1,
            "{expected}: строк в скриптах замеров {} (нужна ровно одна){}",
            hits.len(),
            if hits.is_empty() {
                String::new()
            } else {
                format!(", встречен в: {}", hits.join(", "))
            }
        );
    }
}

/// Таблица «Оставшийся реестр замеров» плана фоновых заданий не имеет
/// права называть решение, которого нет в реестре открытых вопросов:
/// строка таблицы без полного комплекта `НЕ ИЗМЕРЕНО` — это решение,
/// принятое молча. Тест ловит и обратный дрейф терминологии: ID в
/// таблице обязан быть либо вопросом реестра, либо якорем.
#[test]
fn the_job_plan_remaining_table_is_registered() {
    let doc = repo_root().join("docs/archive/plans/background-jobs.md");
    let text =
        fs::read_to_string(&doc).unwrap_or_else(|e| panic!("не читается {}: {e}", doc.display()));
    let section_start = text
        .find("Оставшийся реестр замеров")
        .expect("в плане нет раздела оставшихся замеров");
    let section = &text[section_start..];
    let section_end = section.find("\n## ").unwrap_or(section.len());
    let section = &section[..section_end];

    let known: Vec<&str> = OPEN_QUESTIONS
        .iter()
        .map(|q| q.id)
        .chain(MEASURED_ANCHORS.iter().map(|a| a.id))
        .collect();
    let mut unregistered = Vec::new();
    for line in section.lines() {
        let Some(rest) = line.strip_prefix("| `") else {
            continue;
        };
        let Some(id) = rest.split('`').next() else {
            continue;
        };
        if id.starts_with("JOB.") && !known.contains(&id) {
            unregistered.push(id.to_string());
        }
    }
    assert!(
        !unregistered.is_empty() || section.contains("| `JOB."),
        "таблица оставшихся замеров не разобралась — формат сменился?"
    );
    assert!(
        unregistered.is_empty(),
        "решения из таблицы плана без комплекта НЕ ИЗМЕРЕНО:\n  {}",
        unregistered.join("\n  ")
    );
}
