//! Описания и файлы реализаций для справочника BSL API.
//!
//! Дескрипторы API хранят имена и указатели на функции, но не места в
//! исходниках, поэтому генератор читает `crates/*/src/**/*.rs` и находит
//! строку дескриптора по её массиву имён. Состав справочника по-прежнему
//! задают дескрипторы: индекс только добавляет к готовой строке описание и
//! путь к файлу и ничего не перечисляет сам.

use std::collections::HashMap;
use std::path::Path;

/// Описание и файл реализации одной строки справочника.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// Первое предложение doc-комментария; `None`, если комментария нет.
    pub description: Option<String>,
    /// Путь вида `крейт/src/файл.rs`; `None`, если строка не найдена.
    pub file: Option<String>,
}

impl Entry {
    const EMPTY: Entry = Entry {
        description: None,
        file: None,
    };
}

/// Строка таблицы дескрипторов в исходниках.
struct Row {
    names: Vec<String>,
    file: usize,
    /// Номер статического среза в файле, внутри которого стоит строка.
    group: usize,
    implementation: Option<String>,
    doc: Option<String>,
}

struct FnDef {
    file: usize,
    doc: Option<String>,
}

struct TypeDef {
    package: String,
    name: String,
    file: usize,
    doc: Option<String>,
}

/// Индекс исходников workspace.
#[derive(Default)]
pub struct SourceIndex {
    /// Пути файлов вида `крейт/src/файл.rs`.
    files: Vec<String>,
    rows: Vec<Row>,
    functions: HashMap<String, Vec<FnDef>>,
    types: Vec<TypeDef>,
    builtin_fn_docs: HashMap<String, String>,
    builtin_fn_calls: HashMap<String, String>,
    builtin_method_docs: HashMap<String, String>,
    /// Ключи имён каждого статического среза: `(файл, срез)`.
    groups: HashMap<(usize, usize), std::collections::HashSet<String>>,
}

fn key<'a>(names: impl IntoIterator<Item = &'a str>) -> String {
    names.into_iter().collect::<Vec<_>>().join("\u{1f}")
}

const BUILTIN_FILE: &str = "bsl-rt/src/builtin.rs";

impl SourceIndex {
    /// Читает исходники крейтов workspace с корнем `root`.
    ///
    /// # Errors
    ///
    /// Возвращает текст ошибки, если каталога `crates` нет или файл не читается.
    pub fn load(root: &Path) -> Result<Self, String> {
        let crates = root.join("crates");
        let entries = std::fs::read_dir(&crates).map_err(|error| {
            format!(
                "исходники workspace не найдены в «{}»: {error}",
                crates.display()
            )
        })?;
        let mut files = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|error| error.to_string())?;
            let source = entry.path().join("src");
            if source.is_dir() {
                collect(&source, &mut files)?;
            }
        }
        files.sort();
        let mut texts = Vec::new();
        for path in files {
            let text = std::fs::read_to_string(&path)
                .map_err(|error| format!("не удалось прочитать «{}»: {error}", path.display()))?;
            let relative = path
                .strip_prefix(&crates)
                .expect("файл найден внутри crates")
                .to_string_lossy()
                .replace('\\', "/");
            texts.push((relative, text));
        }
        Ok(Self::from_files(texts))
    }

    /// Строит индекс из пар «путь от `crates/`, текст».
    pub fn from_files(files: Vec<(String, String)>) -> Self {
        let mut index = Self::default();
        for (path, text) in files {
            let file = index.files.len();
            index.files.push(path);
            index.scan(file, &text);
        }
        for row in &index.rows {
            index
                .groups
                .entry((row.file, row.group))
                .or_default()
                .insert(key(row.names.iter().map(String::as_str)));
        }
        index
    }

    fn crate_of(&self, file: usize) -> &str {
        self.files[file].split('/').next().unwrap_or("")
    }

    fn scan(&mut self, file: usize, text: &str) {
        let lines: Vec<&str> = text.lines().collect();
        let mut group = 0;
        let mut line = 0;
        while line < lines.len() {
            let current = lines[line];
            let trimmed = current.trim_start();
            if is_slice_opening(trimmed) {
                group += 1;
            }
            if let Some(name) = fn_name(trimmed) {
                self.functions
                    .entry(name.to_string())
                    .or_default()
                    .push(FnDef {
                        file,
                        doc: doc_above(&lines, line),
                    });
            }
            if (trimmed.contains("TypeDescriptor {") || trimmed.contains("TypeDescriptor::new("))
                && let Some(mut ty) = type_definition(&lines, line, file)
            {
                if ty.package.is_empty() {
                    ty.package = self.crate_of(file).to_string();
                }
                self.types.push(ty);
            }
            if let Some(start) = names_start(&lines, line) {
                let (names, end) = string_array(&lines, line, start);
                if !names.is_empty() {
                    let head = row_head(&lines, line);
                    self.rows.push(Row {
                        names,
                        file,
                        group,
                        implementation: implementation(&lines, head, end),
                        doc: doc_above(&lines, head),
                    });
                }
                line = end.max(line);
            }
            line += 1;
        }
        if self.files[file] == BUILTIN_FILE {
            self.builtin_fn_docs = variant_docs(&lines, "pub enum BuiltinFn {");
            self.builtin_method_docs = variant_docs(&lines, "pub enum BuiltinMethod {");
            self.builtin_fn_calls = builtin_calls(&lines);
        }
    }

    /// Строка дескриптора с именами `names` из таблицы `table` компонента
    /// `package`. Соседние имена таблицы выбирают кандидата, когда такие же
    /// имена есть в нескольких таблицах.
    pub fn row(&self, package: &str, names: &[&str], table: &[&[&str]]) -> Entry {
        self.member(package, None, names, table)
    }

    /// Как [`SourceIndex::row`], но для члена типа `owner`: при равенстве
    /// таблиц выигрывает строка рядом с определением типа.
    pub fn member(
        &self,
        package: &str,
        owner: Option<&str>,
        names: &[&str],
        table: &[&[&str]],
    ) -> Entry {
        let owner_file = owner.and_then(|owner| {
            self.types
                .iter()
                .find(|ty| ty.name == owner && ty.package == package)
                .map(|ty| ty.file)
        });
        let directory = |file: usize| self.files[file].rsplit_once('/').map(|(dir, _)| dir);
        let candidates: Vec<&Row> = self
            .rows
            .iter()
            .filter(|row| {
                row.names
                    .iter()
                    .map(String::as_str)
                    .eq(names.iter().copied())
            })
            .collect();
        let Some(row) = candidates.iter().copied().max_by_key(|row| {
            let group = self.groups.get(&(row.file, row.group));
            let neighbours = table
                .iter()
                .filter(|other| {
                    group.is_some_and(|group| group.contains(&key(other.iter().copied())))
                })
                .count();
            // При равном счёте выигрывает срез без лишних строк, затем крейт
            // компонента, затем первый файл.
            let extra = group.map_or(0, |group| group.len().saturating_sub(neighbours));
            (
                neighbours,
                std::cmp::Reverse(extra),
                owner_file == Some(row.file),
                owner_file.is_some_and(|owner| directory(owner) == directory(row.file)),
                self.crate_of(row.file) == package,
                std::cmp::Reverse(row.file),
            )
        }) else {
            return Entry::EMPTY;
        };
        let function = row
            .implementation
            .as_deref()
            .and_then(|name| self.function(name, row.file));
        Entry {
            description: function
                .and_then(|function| function.doc.clone())
                .or_else(|| row.doc.clone()),
            file: Some(self.files[function.map_or(row.file, |function| function.file)].clone()),
        }
    }

    /// Определение функции `name`: в файле `near`, иначе в его крейте,
    /// иначе единственное в workspace.
    fn function(&self, name: &str, near: usize) -> Option<&FnDef> {
        let name = name.rsplit("::").next().unwrap_or(name);
        let definitions = self.functions.get(name)?;
        definitions
            .iter()
            .find(|definition| definition.file == near)
            .or_else(|| {
                definitions
                    .iter()
                    .find(|definition| self.crate_of(definition.file) == self.crate_of(near))
            })
            .or_else(|| (definitions.len() == 1).then(|| &definitions[0]))
    }

    /// Определение типа `name` компонента `package`.
    pub fn ty(&self, package: &str, name: &str) -> Entry {
        self.types
            .iter()
            .find(|ty| ty.name == name && ty.package == package)
            .or_else(|| self.types.iter().find(|ty| ty.name == name))
            .map_or(Entry::EMPTY, |ty| Entry {
                description: ty.doc.clone(),
                file: Some(self.files[ty.file].clone()),
            })
    }

    /// Встроенная функция по имени варианта `BuiltinFn`.
    pub fn builtin_fn(&self, variant: &str) -> Entry {
        // Только функции самого `bsl-rt`: одноимённая функция другого крейта
        // реализацией встроенной функции не является.
        let function = self
            .builtin_fn_calls
            .get(variant)
            .and_then(|name| self.function(name, self.builtin_file()?))
            .filter(|function| self.crate_of(function.file) == "bsl-rt");
        Entry {
            description: self
                .builtin_fn_docs
                .get(variant)
                .cloned()
                .or_else(|| function.and_then(|function| function.doc.clone())),
            file: Some(function.map_or(BUILTIN_FILE.to_string(), |function| {
                self.files[function.file].clone()
            })),
        }
    }

    /// Встроенный метод по имени варианта `BuiltinMethod`. Реализация
    /// разнесена по типам получателей, поэтому ссылка ведёт на таблицу.
    pub fn builtin_method(&self, variant: &str) -> Entry {
        Entry {
            description: self.builtin_method_docs.get(variant).cloned(),
            file: Some(BUILTIN_FILE.to_string()),
        }
    }

    fn builtin_file(&self) -> Option<usize> {
        self.files.iter().position(|file| file == BUILTIN_FILE)
    }
}

fn collect(directory: &Path, files: &mut Vec<std::path::PathBuf>) -> Result<(), String> {
    let entries = std::fs::read_dir(directory).map_err(|error| error.to_string())?;
    for entry in entries {
        let path = entry.map_err(|error| error.to_string())?.path();
        if path.is_dir() {
            collect(&path, files)?;
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            let name = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("");
            // Тестовые таблицы повторяют имена настоящих и сбивали бы выбор.
            if name != "tests.rs" && !name.ends_with("_tests.rs") {
                files.push(path);
            }
        }
    }
    Ok(())
}

/// Начало статического среза: `static ИМЯ: &[…] = &[` или `const …`.
fn is_slice_opening(line: &str) -> bool {
    let line = line
        .strip_prefix("pub(crate) ")
        .or_else(|| line.strip_prefix("pub "))
        .unwrap_or(line);
    (line.starts_with("static ") || line.starts_with("const ")) && line.contains(": &[")
}

/// Имя функции, объявленной в строке.
fn fn_name(line: &str) -> Option<&str> {
    let mut rest = line;
    if let Some(after) = rest.strip_prefix("pub") {
        rest = match after.strip_prefix('(') {
            Some(scoped) => scoped.split_once(')')?.1.trim_start(),
            None => after.strip_prefix(' ')?,
        };
    }
    for prefix in ["const ", "async ", "unsafe "] {
        rest = rest.strip_prefix(prefix).unwrap_or(rest);
    }
    let rest = rest.strip_prefix("fn ")?;
    let end = rest
        .find(|c: char| !(c.is_alphanumeric() || c == '_'))
        .unwrap_or(rest.len());
    (end > 0).then(|| &rest[..end])
}

/// Позиция `&[` массива имён дескриптора в строке `index`: поле
/// `names: &[`, первый аргумент `…Descriptor::конструктор(&[` в той же
/// строке или — после переноса rustfmt — в начале следующей.
fn names_start(lines: &[&str], index: usize) -> Option<usize> {
    let line = lines[index];
    if let Some(position) = line.find("names: &[") {
        return Some(position + "names: ".len());
    }
    if let Some(position) = line.find("Descriptor::") {
        let open = line[position..].find("(&[")?;
        return Some(position + open + 1);
    }
    let trimmed = line.trim_start();
    let previous = lines.get(index.checked_sub(1)?)?.trim_end();
    (trimmed.starts_with("&[") && previous.contains("Descriptor::") && previous.ends_with('('))
        .then(|| line.len() - trimmed.len())
}

/// Строковые литералы массива, начатого `&[` в строке `line` с позиции
/// `start`, и номер строки, где массив закрыт.
fn string_array(lines: &[&str], line: usize, start: usize) -> (Vec<String>, usize) {
    let mut names = Vec::new();
    let mut current = line;
    let mut text = &lines[line][start + 2..];
    loop {
        let mut rest = text;
        loop {
            let trimmed = rest.trim_start();
            if let Some(after) = trimmed.strip_prefix(']') {
                let _ = after;
                return (names, current);
            }
            if let Some(after) = trimmed.strip_prefix('"') {
                let Some(close) = after.find('"') else {
                    return (Vec::new(), current);
                };
                names.push(after[..close].to_string());
                rest = after[close + 1..].trim_start();
                rest = rest.strip_prefix(',').unwrap_or(rest);
                continue;
            }
            if trimmed.is_empty() {
                break;
            }
            // Не литерал: массив составлен из констант, имена не видны.
            return (Vec::new(), current);
        }
        current += 1;
        if current >= lines.len() || current > line + 8 {
            return (Vec::new(), line);
        }
        text = lines[current];
    }
}

/// Первая строка дескриптора: для литерала структуры — строка с
/// `Descriptor {` выше массива имён.
fn row_head(lines: &[&str], line: usize) -> usize {
    if lines[line].contains("Descriptor::") {
        return line;
    }
    if line > 0
        && lines[line - 1].contains("Descriptor::")
        && lines[line - 1].trim_end().ends_with('(')
    {
        return line - 1;
    }
    (line.saturating_sub(4)..line)
        .rev()
        .find(|&candidate| lines[candidate].contains("Descriptor {"))
        .unwrap_or(line)
}

/// Имя функции, реализующей строку дескриптора: `call: Some(f)`,
/// `get: f` или последний аргумент `…Descriptor::конструктор(…, f)`.
fn implementation(lines: &[&str], head: usize, names_end: usize) -> Option<String> {
    let last = (names_end + 8).min(lines.len());
    let mut text = String::new();
    for line in &lines[head..last] {
        text.push_str(line.trim());
        text.push(' ');
    }
    if lines[head].contains("Descriptor::") {
        let constructor = text.find("Descriptor::")?;
        let open = constructor + text[constructor..].find('(')?;
        let mut depth = 0usize;
        let mut last_comma = open + 1;
        for (position, c) in text[open..].char_indices() {
            match c {
                '(' | '[' => depth += 1,
                ')' | ']' => {
                    depth -= 1;
                    if depth == 0 {
                        return identifier(&text[last_comma..open + position]);
                    }
                }
                ',' if depth == 1 => last_comma = open + position + 1,
                _ => {}
            }
        }
        return None;
    }
    // Литерал структуры: поля до закрывающей скобки строки.
    let body = &text[..text
        .find("},")
        .or_else(|| text.find('}'))
        .unwrap_or(text.len())];
    for field in ["call: Some(", "get: "] {
        if let Some(position) = body.find(field) {
            return identifier(&body[position + field.len()..]);
        }
    }
    None
}

fn identifier(text: &str) -> Option<String> {
    let text = text.trim();
    let end = text
        .find(|c: char| !(c.is_alphanumeric() || c == '_' || c == ':'))
        .unwrap_or(text.len());
    let name = text[..end].trim_end_matches(':');
    (!name.is_empty() && !name.starts_with(|c: char| c.is_ascii_digit()) && name != "None")
        .then(|| name.to_string())
}

/// Первое предложение doc-комментария над строкой `line`; атрибуты
/// `#[…]` между комментарием и строкой пропускаются.
fn doc_above(lines: &[&str], line: usize) -> Option<String> {
    let mut current = line;
    while current > 0 && lines[current - 1].trim_start().starts_with("#[") {
        current -= 1;
    }
    let mut doc = Vec::new();
    while current > 0 {
        let previous = lines[current - 1].trim_start();
        let Some(text) = previous.strip_prefix("///") else {
            break;
        };
        doc.push(text.strip_prefix(' ').unwrap_or(text));
        current -= 1;
    }
    doc.reverse();
    first_sentence(&doc)
}

/// Первое предложение первого абзаца с текстом ссылок rustdoc вместо
/// самих ссылок; `|` экранируется для таблицы Markdown.
fn first_sentence(doc: &[&str]) -> Option<String> {
    let paragraph = doc
        .iter()
        .take_while(|line| !line.trim().is_empty())
        .map(|line| line.trim())
        .collect::<Vec<_>>()
        .join(" ");
    let mut text = String::new();
    let mut code = false;
    let mut depth = 0usize;
    let chars: Vec<char> = paragraph.chars().collect();
    let mut index = 0;
    while index < chars.len() {
        let c = chars[index];
        match c {
            '`' => code = !code,
            '(' if !code => depth += 1,
            ')' if !code => depth = depth.saturating_sub(1),
            _ => {}
        }
        // Точка внутри скобок — это «см.» или «т. е.», а не конец предложения.
        if !code && depth == 0 && c == '.' && chars.get(index + 1).is_none_or(|next| *next == ' ') {
            text.push('.');
            break;
        }
        text.push(c);
        index += 1;
    }
    let text = strip_links(text.trim()).replace('|', "\\|");
    (!text.is_empty()).then_some(text)
}

/// `[текст](адрес)` и `[текст]` превращаются в `текст`.
fn strip_links(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(open) = rest.find('[') {
        let Some(close) = rest[open..].find(']').map(|close| open + close) else {
            break;
        };
        out.push_str(&rest[..open]);
        out.push_str(&rest[open + 1..close]);
        rest = &rest[close + 1..];
        if let Some(after) = rest.strip_prefix('(') {
            rest = after.split_once(')').map_or("", |(_, tail)| tail);
        }
    }
    out.push_str(rest);
    out
}

/// Определение типа с полями `package` и `name`, начатое в строке `line`.
fn type_definition(lines: &[&str], line: usize, file: usize) -> Option<TypeDef> {
    let last = (line + 6).min(lines.len());
    let text: String = lines[line..last].iter().map(|line| line.trim()).collect();
    let literal = |field: &str| {
        let position = text.find(field)? + field.len();
        let rest = text[position..].strip_prefix('"')?;
        Some(rest[..rest.find('"')?].to_string())
    };
    let (package, name) = if let Some(position) = text.find("TypeDescriptor::new(") {
        let arguments = &text[position + "TypeDescriptor::new(".len()..];
        let arguments = &arguments[..arguments.find(')')?];
        let (package, name) = arguments.split_once(',')?;
        let unquote = |text: &str| {
            // rustfmt оставляет запятую после последнего аргумента.
            let text = text.trim().trim_end_matches(',').trim_end();
            text.strip_prefix('"')
                .and_then(|text| text.strip_suffix('"'))
                .map(str::to_string)
        };
        (unquote(package).unwrap_or_default(), unquote(name)?)
    } else {
        // `package: crate::PACKAGE_NAME` — не литерал; пакет тогда — крейт файла.
        (literal("package: ").unwrap_or_default(), literal("name: ")?)
    };
    let head = (line.saturating_sub(3)..=line)
        .find(|&candidate| {
            let trimmed = lines[candidate].trim_start();
            let trimmed = trimmed
                .strip_prefix("pub(crate) ")
                .or_else(|| trimmed.strip_prefix("pub "))
                .unwrap_or(trimmed);
            trimmed.starts_with("static ") || trimmed.starts_with("const ")
        })
        .unwrap_or(line);
    Some(TypeDef {
        package,
        name,
        file,
        doc: doc_above(lines, head),
    })
}

/// Описания вариантов перечисления, начатого строкой `header`.
fn variant_docs(lines: &[&str], header: &str) -> HashMap<String, String> {
    let mut docs = HashMap::new();
    let Some(start) = lines.iter().position(|line| line.trim_start() == header) else {
        return docs;
    };
    for line in start + 1..lines.len() {
        let trimmed = lines[line].trim();
        if trimmed == "}" {
            break;
        }
        if trimmed.starts_with("//") || trimmed.starts_with("#[") || trimmed.is_empty() {
            continue;
        }
        let end = trimmed
            .find(|c: char| !(c.is_alphanumeric() || c == '_'))
            .unwrap_or(trimmed.len());
        if let Some(doc) = doc_above(lines, line) {
            docs.insert(trimmed[..end].to_string(), doc);
        }
    }
    docs
}

/// Свободная функция, которую вызывает ветка `BuiltinFn::Вариант =>`.
fn builtin_calls(lines: &[&str]) -> HashMap<String, String> {
    let mut calls = HashMap::new();
    for (line, text) in lines.iter().enumerate() {
        let Some((pattern, body)) = text.split_once("=>") else {
            continue;
        };
        let variants: Vec<&str> = pattern
            .split('|')
            .filter_map(|part| part.trim().strip_prefix("BuiltinFn::"))
            .collect();
        if variants.is_empty() {
            continue;
        }
        let body = if body.trim() == "{" || body.trim().is_empty() {
            lines.get(line + 1).copied().unwrap_or("")
        } else {
            body
        };
        let Some(call) = free_call(body) else {
            continue;
        };
        for variant in variants {
            calls.entry(variant.to_string()).or_insert(call.clone());
        }
    }
    calls
}

/// Первая свободная функция `имя(` в выражении, не метод через точку.
fn free_call(text: &str) -> Option<String> {
    let chars: Vec<char> = text.chars().collect();
    let mut index = 0;
    while index < chars.len() {
        if chars[index].is_alphabetic() || chars[index] == '_' {
            let start = index;
            while index < chars.len()
                && (chars[index].is_alphanumeric() || chars[index] == '_' || chars[index] == ':')
            {
                index += 1;
            }
            let name: String = chars[start..index].iter().collect();
            let after_dot = start > 0 && chars[start - 1] == '.';
            if chars.get(index) == Some(&'(')
                && !after_dot
                && !matches!(name.as_str(), "Ok" | "Err" | "Some")
                && name.starts_with(|c: char| c.is_lowercase())
            {
                return Some(name);
            }
        } else {
            index += 1;
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn index(files: &[(&str, &str)]) -> SourceIndex {
        SourceIndex::from_files(
            files
                .iter()
                .map(|(path, text)| (path.to_string(), text.to_string()))
                .collect(),
        )
    }

    #[test]
    fn method_rows_link_to_the_documented_implementation() {
        let index = index(&[
            (
                "bsl-zip/src/objects.rs",
                "static METHODS: &[MethodDescriptor] = &[\n    MethodDescriptor::new(&[\"Открыть\", \"Open\"], Arity::range(1, 2), open),\n];\n",
            ),
            (
                "bsl-zip/src/reader.rs",
                "/// Открывает архив по пути. Второе предложение.\npub(crate) fn open() {}\n",
            ),
        ]);
        assert_eq!(
            index.row("bsl-zip", &["Открыть", "Open"], &[&["Открыть", "Open"]]),
            Entry {
                description: Some("Открывает архив по пути.".into()),
                file: Some("bsl-zip/src/reader.rs".into()),
            }
        );
    }

    #[test]
    fn struct_rows_without_docs_link_to_the_getter_file() {
        let index = index(&[(
            "bsl-stream/src/lib.rs",
            "const PROPS: &[PropertyDescriptor] = &[\n    PropertyDescriptor {\n        names: &[\n            \"ДоступнаЗапись\",\n            \"CanWrite\",\n        ],\n        get: can_write,\n        set: None,\n    },\n];\nfn can_write() {}\n",
        )]);
        assert_eq!(
            index.row(
                "bsl-stream",
                &["ДоступнаЗапись", "CanWrite"],
                &[&["ДоступнаЗапись", "CanWrite"]]
            ),
            Entry {
                description: None,
                file: Some("bsl-stream/src/lib.rs".into()),
            }
        );
    }

    #[test]
    fn neighbouring_names_choose_between_tables_with_the_same_method() {
        let index = index(&[
            (
                "bsl-a/src/first.rs",
                "static A: &[MethodDescriptor] = &[\n    MethodDescriptor::new(&[\"Количество\"], Arity::exact(0), first_count),\n    MethodDescriptor::new(&[\"Первый\"], Arity::exact(0), first),\n];\n/// Счёт первого.\nfn first_count() {}\n",
            ),
            (
                "bsl-a/src/second.rs",
                "static B: &[MethodDescriptor] = &[\n    MethodDescriptor::new(&[\"Количество\"], Arity::exact(0), second_count),\n    MethodDescriptor::new(&[\"Второй\"], Arity::exact(0), second),\n];\n/// Счёт второго.\nfn second_count() {}\n",
            ),
        ]);
        let entry = index.row("bsl-a", &["Количество"], &[&["Количество"], &["Второй"]]);
        assert_eq!(entry.description.as_deref(), Some("Счёт второго."));
        assert_eq!(entry.file.as_deref(), Some("bsl-a/src/second.rs"));
    }

    #[test]
    fn descriptions_drop_rustdoc_links_and_escape_pipes() {
        assert_eq!(
            first_sentence(&["Читает [`Поток`](crate::Stream) a|b. Дальше."]).as_deref(),
            Some("Читает `Поток` a\\|b.")
        );
        assert_eq!(
            first_sentence(&["Версия `0.1.0` без точки"]).as_deref(),
            Some("Версия `0.1.0` без точки")
        );
    }

    #[test]
    fn types_and_builtins_are_found_by_name() {
        let index = index(&[(
            "bsl-rt/src/builtin.rs",
            "/// Описание типа.\npub static T: TypeDescriptor = TypeDescriptor {\n    package: \"bsl-rt\",\n    name: \"Файл\",\n    type_display: \"Файл\",\n    type_names: &[],\n};\npub enum BuiltinFn {\n    /// Длина строки.\n    StrLen,\n}\nfn call() { match f {\n    BuiltinFn::StrLen => str_len(args),\n} }\nfn str_len() {}\n",
        )]);
        assert_eq!(
            index.ty("bsl-rt", "Файл"),
            Entry {
                description: Some("Описание типа.".into()),
                file: Some("bsl-rt/src/builtin.rs".into()),
            }
        );
        assert_eq!(
            index.builtin_fn("StrLen").description.as_deref(),
            Some("Длина строки.")
        );
    }

    #[test]
    fn a_missing_source_tree_is_an_error() {
        let error = SourceIndex::load(Path::new("/nonexistent-open-bsl-root"))
            .err()
            .expect("ошибка без исходников");
        assert!(error.contains("исходники workspace не найдены"), "{error}");
    }
}
