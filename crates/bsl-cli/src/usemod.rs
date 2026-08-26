//! Файловый препроцессор `//@use`/`//@используй`.
//!
//! Превращает граф файлов BSL в не зависящий от файловой системы
//! `ModuleGraphRecipe` фасада: разбор шапки, пути относительно импортёра,
//! канонизация и дедуп по каноническому пути, поиск циклов. Директива —
//! возможность CLI: `Engine` её текст не разбирает никогда. Каталожное имя
//! модуля — корневой псевдоним (импорт из главного скрипта); файл,
//! достижимый только транзитивно, получает канонический путь как имя.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Одна директива шапки: путь как записан и псевдоним.
#[derive(Debug, Clone)]
pub struct UseDirective {
    pub path: String,
    pub alias: String,
}

/// Разбирает директивы шапки. Ошибка — синтаксически кривая директива либо
/// директива ниже шапки: строка, начинающаяся с `//@use(` или
/// `//@используй(`, либо валидна и стоит в шапке, либо валит компиляцию —
/// молчаливого «это просто комментарий» нет.
pub fn parse_directives(source: &str) -> Result<Vec<UseDirective>, String> {
    let mut directives = Vec::new();
    let mut in_header = true;
    for (no, raw) in source.lines().enumerate() {
        let line = raw.trim_start_matches('\u{feff}').trim();
        let is_directive = line.starts_with("//@use(") || line.starts_with("//@используй(");
        if is_directive {
            if !in_header {
                return Err(format!(
                    "строка {}: директива //@используй допустима только в шапке файла",
                    no + 1
                ));
            }
            directives.push(parse_one(no + 1, line)?);
            continue;
        }
        if line.is_empty() || line.starts_with("//") {
            continue;
        }
        // Первый оператор или объявление — конец шапки.
        in_header = false;
    }
    Ok(directives)
}

fn parse_one(no: usize, line: &str) -> Result<UseDirective, String> {
    let inner = line
        .strip_prefix("//@use(")
        .or_else(|| line.strip_prefix("//@используй("))
        .expect("вызывается только для строк с префиксом директивы");
    let inner = inner
        .strip_suffix(')')
        .ok_or_else(|| format!("строка {no}: у директивы нет закрывающей скобки"))?;
    // Русская и английская связки равноправны; ищем последнюю — путь может
    // содержать пробелы в кавычках, а псевдоним пробелов не содержит.
    let split = inner
        .rfind(" как ")
        .map(|at| (at, " как ".len()))
        .or_else(|| inner.rfind(" as ").map(|at| (at, " as ".len())));
    let Some((at, sep_len)) = split else {
        return Err(format!(
            "строка {no}: ожидалось «путь как Псевдоним» либо «path as Alias»"
        ));
    };
    let raw_path = inner[..at].trim();
    let alias = inner[at + sep_len..].trim();
    let path = raw_path
        .strip_prefix('"')
        .and_then(|p| p.strip_suffix('"'))
        .or_else(|| {
            raw_path
                .strip_prefix('\'')
                .and_then(|p| p.strip_suffix('\''))
        })
        .unwrap_or(raw_path);
    if path.is_empty() {
        return Err(format!("строка {no}: путь директивы пуст"));
    }
    if alias.is_empty()
        || alias
            .chars()
            .next()
            .is_some_and(|first| first.is_ascii_digit())
        || !alias.chars().all(|c| c.is_alphanumeric() || c == '_')
    {
        return Err(format!(
            "строка {no}: «{alias}» не годится в псевдонимы модуля"
        ));
    }
    Ok(UseDirective {
        path: path.to_string(),
        alias: alias.to_string(),
    })
}

/// Загруженный файл графа до присвоения каталожных имён.
struct LoadedFile {
    canonical: PathBuf,
    source: String,
    /// Импорты файла: (псевдоним, канонический путь цели).
    imports: Vec<(String, PathBuf)>,
}

/// Загружает файловый граф главного скрипта. Возвращает рецепт каталога;
/// исходник entry остаётся у вызывающего — директивы в нём для парсера
/// обычные комментарии.
///
/// # Errors
///
/// Ошибки чтения файлов, разбора директив, дублей псевдонимов и циклов —
/// человекочитаемой строкой с путём.
pub fn load_graph(
    entry_path: &Path,
    entry_source: &str,
) -> Result<open_bsl::ModuleGraphRecipe, String> {
    let entry_directives = parse_directives(entry_source)
        .map_err(|error| format!("{}: {error}", entry_path.display()))?;
    let entry_dir = entry_path.parent().unwrap_or(Path::new("."));

    let mut files: Vec<LoadedFile> = Vec::new();
    let mut by_canonical: HashMap<PathBuf, usize> = HashMap::new();
    let mut visiting: Vec<PathBuf> = Vec::new();
    let mut root_names: Vec<(PathBuf, String)> = Vec::new();

    check_alias_dupes(entry_path, &entry_directives)?;
    for directive in &entry_directives {
        let canonical = canonicalize(entry_dir, &directive.path, entry_path)?;
        load_file(&canonical, &mut files, &mut by_canonical, &mut visiting)?;
        // Корневой псевдоним — стабильное имя общего модуля. Два корневых
        // имени одного файла — ошибка (план фоновых заданий).
        if let Some((_, existing)) = root_names.iter().find(|(path, _)| *path == canonical) {
            if !folded_eq(existing, &directive.alias) {
                return Err(format!(
                    "{}: файл {} подключён в корне дважды под разными именами: «{existing}» и «{}»",
                    entry_path.display(),
                    canonical.display(),
                    directive.alias
                ));
            }
        } else {
            root_names.push((canonical.clone(), directive.alias.clone()));
        }
    }

    // Каталожное имя: корневой псевдоним либо канонический путь.
    let name_of = |canonical: &Path| -> String {
        root_names
            .iter()
            .find(|(path, _)| path == canonical)
            .map(|(_, alias)| alias.clone())
            .unwrap_or_else(|| canonical.display().to_string())
    };
    // Case-insensitive конфликт корневых имён — ошибка.
    for (i, (_, a)) in root_names.iter().enumerate() {
        if root_names[..i].iter().any(|(_, b)| folded_eq(a, b)) {
            return Err(format!(
                "{}: корневое имя модуля «{a}» повторяется",
                entry_path.display()
            ));
        }
    }

    let modules = files
        .iter()
        .map(|file| open_bsl::ModuleRecipe {
            name: name_of(&file.canonical),
            source: file.source.clone(),
            imports: file
                .imports
                .iter()
                .map(|(alias, target)| (alias.clone(), name_of(target)))
                .collect(),
        })
        .collect();
    Ok(open_bsl::ModuleGraphRecipe {
        modules,
        // Семантика расширения CLI: тела модулей выполняются до главного
        // скрипта в post-order файлового графа.
        eager_init: true,
    })
}

fn load_file(
    canonical: &Path,
    files: &mut Vec<LoadedFile>,
    by_canonical: &mut HashMap<PathBuf, usize>,
    visiting: &mut Vec<PathBuf>,
) -> Result<(), String> {
    if by_canonical.contains_key(canonical) {
        return Ok(());
    }
    if visiting.iter().any(|p| p == canonical) {
        return Err(format!("цикл импортов через {}", canonical.display()));
    }
    let source = std::fs::read_to_string(canonical)
        .map_err(|error| format!("{}: {error}", canonical.display()))?;
    let directives =
        parse_directives(&source).map_err(|error| format!("{}: {error}", canonical.display()))?;
    let dir = canonical.parent().unwrap_or(Path::new(".")).to_path_buf();
    check_alias_dupes(canonical, &directives)?;
    visiting.push(canonical.to_path_buf());
    let mut imports = Vec::with_capacity(directives.len());
    for directive in &directives {
        let target = canonicalize(&dir, &directive.path, canonical)?;
        load_file(&target, files, by_canonical, visiting)?;
        imports.push((directive.alias.clone(), target));
    }
    visiting.pop();
    by_canonical.insert(canonical.to_path_buf(), files.len());
    files.push(LoadedFile {
        canonical: canonical.to_path_buf(),
        source,
        imports,
    });
    Ok(())
}

fn check_alias_dupes(path: &Path, directives: &[UseDirective]) -> Result<(), String> {
    for (i, directive) in directives.iter().enumerate() {
        if directives[..i]
            .iter()
            .any(|other| folded_eq(&other.alias, &directive.alias))
        {
            return Err(format!(
                "{}: псевдоним «{}» повторяется",
                path.display(),
                directive.alias
            ));
        }
    }
    Ok(())
}

fn canonicalize(dir: &Path, relative: &str, importer: &Path) -> Result<PathBuf, String> {
    let joined = dir.join(relative);
    joined.canonicalize().map_err(|error| {
        format!(
            "{}: файл «{relative}» не найден: {error}",
            importer.display()
        )
    })
}

fn folded_eq(a: &str, b: &str) -> bool {
    a.to_uppercase() == b.to_uppercase()
}
