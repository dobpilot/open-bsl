//! Поверхность чтения: `ЧтениеZipФайла`, коллекции и элементы.

use super::*;

// --------------------------------------------------------------------------
// Поверхность встроенного языка
// --------------------------------------------------------------------------

/// Который из двух платформенных читателей стоит за объектом.
///
/// На 8.3.27 их ДВА, и это измерено: `ЧтениеZipФайла` и `ЧтениеФайлаАрхива`
/// — разные типы (`Тип("ЧтениеZipФайла") = Тип("ЧтениеФайлаАрхива")` —
/// «Нет»), у каждого своя пара «коллекция + элемент», а поверхность у них с
/// точностью до имён одна: те же четыре метода, те же свойства элемента.
/// Отсюда один набор объектов с этим тегом вместо двух параллельных: тег
/// решает только имя типа и третий параметр конструктора, которого у
/// `ЧтениеZipФайла` нет вовсе (`Новый ЧтениеZipФайла(файл, пароль, тип)` —
/// «Конструктор не найден»).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchiveKind {
    /// `ЧтениеZipФайла` / `ЭлементыZipФайла` / `ЭлементZipФайла`.
    Zip,
    /// `ЧтениеФайлаАрхива` / `ЭлементыФайлаАрхива` / `ЭлементФайлаАрхива`.
    Archive,
}

/// Один элемент архива глазами встроенного языка.
///
/// Имена посчитаны один раз при открытии, а не при каждом обращении:
/// разрешение дублей зависит от УЖЕ разобранных соседей (см.
/// [`build_items`]), так что позаписной пересчёт дал бы другой ответ.
#[derive(Debug)]
pub struct ArchiveItem {
    pub(crate) entry: RawEntry,
    /// Номер записи в каталоге — для распаковки через [`read_entry`].
    pub(crate) index: usize,
    /// `Имя` — короткое имя после подстановки и разрешения дублей.
    pub(crate) name: String,
    /// `Путь` — каталоги, со слэшем на конце; у записи в корне пусто.
    pub(crate) path: String,
    /// `ИсходноеИмя` — то же место в архиве, но без подстановки.
    pub(crate) orig_name: String,
    /// `ИсходныйПуть`.
    pub(crate) orig_path: String,
}

impl ArchiveItem {
    /// `ПолноеИмя` — измерено, что это ровно `Путь` + `Имя`, включая
    /// запись-каталог (`папка/` + `` = `папка/`).
    pub(crate) fn full_name(&self) -> String {
        format!("{}{}", self.path, self.name)
    }

    pub(crate) fn orig_full_name(&self) -> String {
        format!("{}{}", self.orig_path, self.orig_name)
    }
}

/// Открытый архив: собственные байты, комментарий и разобранные элементы.
#[derive(Debug)]
pub(crate) struct OpenArchive {
    pub(crate) data: Vec<u8>,
    /// Откуда открыт — только для текста ошибки «архив уже открыт».
    pub(crate) source: String,
    pub(crate) comment: String,
    pub(crate) items: Vec<ArchiveItem>,
}

/// Состояние объекта чтения. Пустое до `Открыть` и после `Закрыть`:
/// измерено, что на закрытом архиве и `Элементы`, и `Закрыть`, и
/// `ИзвлечьВсе` отвечают ошибкой «Архив не открыт!», а `Открыть` на уже
/// открытом — «Архив уже открыт!».
#[derive(Debug, Default)]
pub struct ArchiveState {
    pub(crate) open: Option<OpenArchive>,
    /// Номер текущего открытия, растёт на каждом успешном `Открыть`.
    ///
    /// Состояние переживает `Закрыть`/`Открыть` — это один и тот же `Rc`, —
    /// а состав архива при этом меняется целиком, так что номер записи,
    /// выданный до переоткрытия, к новому архиву не относится. Элемент
    /// запоминает номер открытия, при котором получен, и [`Self::item`]
    /// сверяет его с текущим: иначе `Извлечь` либо вылетала бы за границу
    /// более короткого каталога, либо — что хуже, потому что незаметно —
    /// молча распаковывала чужую запись, занявшую этот номер.
    pub(crate) generation: u64,
}

impl ArchiveState {
    pub(crate) fn opened(&self, op: &'static str) -> RtResult<&OpenArchive> {
        self.open
            .as_ref()
            .ok_or_else(|| zip_err(&format!("архив не открыт, «{op}» недоступно")))
    }

    /// Запись по номеру, выданному при открытии `generation`.
    ///
    /// Закрытый архив проверяется ПЕРВЫМ: на нём измеренный ответ — «архив
    /// не открыт», и устаревший элемент не должен его подменять.
    pub(crate) fn item(
        &self,
        index: usize,
        generation: u64,
        op: &'static str,
    ) -> RtResult<&ArchiveItem> {
        let open = self.opened(op)?;
        if generation != self.generation {
            return Err(zip_err(&format!(
                "элемент получен при другом открытии архива, «{op}» недоступно"
            )));
        }
        open.items
            .get(index)
            .ok_or_else(|| zip_err(&format!("в архиве нет записи с номером {index}")))
    }
}

/// Символы, недопустимые в имени файла: платформа заменяет каждый из них
/// подчёркиванием (ИЗМЕРЕНО поимённо на архиве с именами `a:b.txt`,
/// `a*b.txt`, `a?b.txt`, `a<b>c.txt`, `a|b.txt`, `a"b.txt` — вышли
/// `a_b.txt`, `a_b(1).txt`, `a_b(2).txt`, `a_b_c.txt`, `a_b(3).txt`,
/// `a_b(4).txt`). Управляющие символы в этот список НЕ входят: имя с
/// байтом `01`, `09`, `1F` или `7F` платформа оставляет как есть — это
/// проверено не по печати, а по именам файлов, которые она создала при
/// распаковке.
pub(crate) const FORBIDDEN_IN_NAME: [char; 7] = [':', '*', '?', '"', '<', '>', '|'];

/// Одна компонента пути после подстановки.
///
/// Хвостовые точки и пробелы платформа срезает (измерено: `dot.` -> `dot`,
/// `two..` -> `two`, `trail ` -> `trail`, `dir /f.txt` -> `dir/f.txt`), а
/// вот ведущий пробел и точка остаются (` lead.txt` и `.hidden` приходят
/// как есть). Побочное следствие среза — то, что компонента `..` целиком
/// превращается в пустую (измерено: `../up.txt` -> `/up.txt`), поэтому
/// выйти распаковкой вверх по дереву через имя записи нельзя.
pub(crate) fn sanitize_component(component: &str) -> String {
    let mut out: String = component
        .chars()
        .map(|c| {
            if FORBIDDEN_IN_NAME.contains(&c) {
                '_'
            } else {
                c
            }
        })
        .collect();
    while out.ends_with('.') || out.ends_with(' ') {
        out.pop();
    }
    out
}

/// Разбить имя на «без последнего расширения» и «расширение».
///
/// Измерено на 8.3.27: `вложенный.txt` -> (`вложенный`, `txt`), `a.b.c.txt`
/// -> (`a.b.c`, `txt`), `noext` -> (`noext`, ``), `.hidden` -> (``,
/// `hidden`) — то есть точка ищется ПОСЛЕДНЯЯ и ведущая точка расширением
/// не считается особым случаем.
pub(crate) fn split_extension(name: &str) -> (&str, &str) {
    match name.rfind('.') {
        Some(at) => (&name[..at], &name[at + 1..]),
        None => (name, ""),
    }
}

/// Свободное имя среди соседей: занятое дополняется `(N)` перед последней
/// точкой.
///
/// ИЗМЕРЕНО, что суффикс встаёт именно перед расширением и что нумерация
/// сквозная по всем столкновениям с одним и тем же именем: `same.txt`
/// дважды дают `same.txt` и `same(1).txt`, `noext` — `noext` и `noext(1)`,
/// `.hidden` — `.hidden` и `(1).hidden`, а шесть имён, схлопнувшихся в
/// `a_b.txt`, получают `(1)`..`(4)` в порядке каталога.
pub(crate) fn unique_among(used: &mut Vec<String>, base: String) -> String {
    if !used.contains(&base) {
        used.push(base.clone());
        return base;
    }
    // Точки в имени здесь уже нет только у имени без расширения: хвостовые
    // точки срезаны подстановкой, так что `stem` + `(N)` + `.ext` собирается
    // без оговорок.
    let (stem, ext) = split_extension(&base);
    for n in 1u32.. {
        let candidate = if ext.is_empty() {
            format!("{stem}({n})")
        } else {
            format!("{stem}({n}).{ext}")
        };
        if !used.contains(&candidate) {
            used.push(candidate.clone());
            return candidate;
        }
    }
    unreachable!("свободный номер находится всегда: имён конечное число")
}

/// Узел дерева каталогов, которое строится по именам записей.
pub(crate) struct DirNode {
    /// Подкаталоги по ИСХОДНОМУ имени компоненты. Именно по исходному:
    /// измерено, что `папка/` и `папка/вложенный.txt` дают ОДИН каталог,
    /// а две РАЗНЫЕ компоненты, схлопнувшиеся в одно имя (`..` и пустая),
    /// остаются разными каталогами и второй получает `(1)`.
    pub(crate) children: Vec<(String, usize)>,
    /// Занятые отображаемые имена среди детей этого узла.
    pub(crate) used: Vec<String>,
    /// Отображаемый путь до узла включительно, со слэшем на конце.
    pub(crate) display: String,
    /// Он же исходный.
    pub(crate) original: String,
}

/// Посчитать отображаемые имена всех записей архива.
///
/// Порядок обхода — каталожный, и это существенно: разрешение дублей
/// зависит от того, кто занял имя раньше.
pub(crate) fn build_items(entries: Vec<RawEntry>) -> Vec<ArchiveItem> {
    let mut nodes = vec![DirNode {
        children: Vec::new(),
        used: Vec::new(),
        display: String::new(),
        original: String::new(),
    }];
    let mut items = Vec::with_capacity(entries.len());

    for (index, entry) in entries.into_iter().enumerate() {
        // Обратный слэш платформа считает разделителем, а не знаком имени:
        // измерено, что `dir\back.txt` приходит как `dir/back.txt` ОБОИМИ
        // именами, и исходным тоже.
        let raw = String::from_utf8_lossy(entry.name_bytes()).replace('\\', "/");
        let is_dir = raw.ends_with('/');
        let trimmed = if is_dir {
            raw.strip_suffix('/').unwrap_or(&raw)
        } else {
            &raw
        };
        let mut parts: Vec<&str> = trimmed.split('/').collect();
        // У записи-каталога собственного короткого имени нет: измерено, что
        // у `папка/` `Имя` пустое, а `ПолноеИмя` и `Путь` — оба `папка/`.
        let leaf = if is_dir {
            ""
        } else {
            parts.pop().unwrap_or("")
        };

        let mut node = 0usize;
        for part in parts {
            node = resolve_dir(&mut nodes, node, part);
        }

        let (name, orig_name) = if is_dir {
            (String::new(), String::new())
        } else {
            let display = unique_among(&mut nodes[node].used, sanitize_component(leaf));
            (display, leaf.to_string())
        };
        items.push(ArchiveItem {
            entry,
            index,
            name,
            path: nodes[node].display.clone(),
            orig_name,
            orig_path: nodes[node].original.clone(),
        });
    }
    items
}

/// Найти или завести подкаталог `part` у узла `parent`.
pub(crate) fn resolve_dir(nodes: &mut Vec<DirNode>, parent: usize, part: &str) -> usize {
    if let Some((_, at)) = nodes[parent].children.iter().find(|(orig, _)| orig == part) {
        return *at;
    }
    let display = unique_among(&mut nodes[parent].used, sanitize_component(part));
    let node = DirNode {
        children: Vec::new(),
        used: Vec::new(),
        display: format!("{}{display}/", nodes[parent].display),
        original: format!("{}{part}/", nodes[parent].original),
    };
    nodes.push(node);
    let at = nodes.len() - 1;
    nodes[parent].children.push((part.to_string(), at));
    at
}

// --- объекты компонента -----------------------------------------------------

/// `ЧтениеZipФайла` либо `ЧтениеФайлаАрхива` — тег решает только имена
/// типов, поверхность у обоих одна (измерено).
#[derive(Debug)]
pub struct ReaderObject {
    pub(crate) kind: ArchiveKind,
    pub(crate) state: Rc<RefCell<ArchiveState>>,
}

/// `ЭлементыZipФайла` / `ЭлементыФайлаАрхива` — не снимок, а окно в то же
/// состояние читателя, как `Таблица.Колонки`.
#[derive(Debug)]
pub struct EntriesObject {
    pub(crate) kind: ArchiveKind,
    pub(crate) state: Rc<RefCell<ArchiveState>>,
}

/// `ЭлементZipФайла` / `ЭлементФайлаАрхива` — то же состояние плюс номер
/// записи в каталоге и номер открытия, при котором элемент получен.
#[derive(Debug)]
pub struct EntryObject {
    pub(crate) kind: ArchiveKind,
    pub(crate) state: Rc<RefCell<ArchiveState>>,
    pub(crate) index: usize,
    pub(crate) generation: u64,
}

/// `ЗаписьZipФайла` / `ЗаписьФайлаАрхива`.
#[derive(Debug)]
pub struct WriterObject {
    pub(crate) kind: ArchiveKind,
    pub(crate) state: Rc<RefCell<WriterState>>,
}

pub(crate) static ZIP_READER_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ЧтениеZipФайла",
    legacy_type_id: Some(TypeId::ZipFileReader),
};

pub(crate) static ARCHIVE_READER_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ЧтениеФайлаАрхива",
    legacy_type_id: Some(TypeId::ArchiveFileReader),
};

pub(crate) static ZIP_ENTRIES_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ЭлементыZipФайла",
    legacy_type_id: Some(TypeId::ZipFileEntries),
};

pub(crate) static ARCHIVE_ENTRIES_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ЭлементыФайлаАрхива",
    legacy_type_id: Some(TypeId::ArchiveFileEntries),
};

pub(crate) static ZIP_ENTRY_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ЭлементZipФайла",
    legacy_type_id: Some(TypeId::ZipFileEntry),
};

pub(crate) static ARCHIVE_ENTRY_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ЭлементФайлаАрхива",
    legacy_type_id: Some(TypeId::ArchiveFileEntry),
};

pub(crate) static ZIP_WRITER_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ЗаписьZipФайла",
    legacy_type_id: Some(TypeId::ZipFileWriter),
};

pub(crate) static ARCHIVE_WRITER_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ЗаписьФайлаАрхива",
    legacy_type_id: Some(TypeId::ArchiveFileWriter),
};

impl ReaderObject {
    pub(crate) fn descriptor(&self) -> &'static TypeDescriptor {
        match self.kind {
            ArchiveKind::Zip => &ZIP_READER_TYPE,
            ArchiveKind::Archive => &ARCHIVE_READER_TYPE,
        }
    }
}

impl EntriesObject {
    pub(crate) fn descriptor(&self) -> &'static TypeDescriptor {
        match self.kind {
            ArchiveKind::Zip => &ZIP_ENTRIES_TYPE,
            ArchiveKind::Archive => &ARCHIVE_ENTRIES_TYPE,
        }
    }
}

impl EntryObject {
    pub(crate) fn descriptor(&self) -> &'static TypeDescriptor {
        match self.kind {
            ArchiveKind::Zip => &ZIP_ENTRY_TYPE,
            ArchiveKind::Archive => &ARCHIVE_ENTRY_TYPE,
        }
    }
}

impl WriterObject {
    pub(crate) fn descriptor(&self) -> &'static TypeDescriptor {
        match self.kind {
            ArchiveKind::Zip => &ZIP_WRITER_TYPE,
            ArchiveKind::Archive => &ARCHIVE_WRITER_TYPE,
        }
    }
}

// --- конструкторы и методы -----------------------------------------------------

/// `Новый ЧтениеZipФайла([Источник][, Пароль])` и
/// `Новый ЧтениеФайлаАрхива([Источник][, Пароль][, ТипФайлаАрхива])`.
///
/// Источник — имя файла либо поток; `ДвоичныеДанные` платформа НЕ принимает
/// (измерено: «Несоответствие типов (параметр номер '1') (Некорректное имя
/// файла)»), и здесь их тоже нет. Без источника объект создаётся закрытым —
/// это законная форма (`Новый ЧтениеZipФайла()` с последующим `Открыть`).
///
/// Пароль ПРИНИМАЕТСЯ И НЕ ИСПОЛЬЗУЕТСЯ: расшифровки здесь нет вовсе, а до
/// первой попытки прочитать зашифрованную запись он не нужен и платформе
/// (измерено: на незашифрованном архиве `Новый ЧтениеZipФайла(файл,
/// "пусто")` проходит молча).
///
/// # Errors
///
/// [`RtError::Zip`], если источник не читается или не является архивом ZIP;
/// [`RtError::TypeError`], если третьим аргументом передан не член
/// `ТипФайлаАрхива`.
pub fn new_archive_reader(
    zip: bool,
    source: &BslValue,
    password: &BslValue,
    archive_type: &BslValue,
) -> RtResult<BslValue> {
    let kind = if zip {
        ArchiveKind::Zip
    } else {
        ArchiveKind::Archive
    };
    check_archive_type(archive_type)?;
    let state = Rc::new(RefCell::new(ArchiveState::default()));
    if !matches!(source, BslValue::Undefined) {
        let (bytes, from) = read_source(source, "ЧтениеZipФайла")?;
        open_bytes(&state, bytes, from)?;
    }
    // Пароль хранить негде и незачем — см. doc comment.
    let _ = password;
    Ok(BslValue::new_object(ReaderObject { kind, state }))
}

/// Третий аргумент конструктора `ЧтениеФайлаАрхива`.
///
/// ИЗМЕРЕНО, что он типизирован: `Неопределено` платформа принимает, член
/// `ТипФайлаАрхива` тоже, а строку, число, булево и члена ЧУЖОГО
/// перечисления отвергает с «Несоответствие типов (параметр номер '3')».
/// Читаем мы только ZIP, поэтому всякий другой объявленный формат — честный
/// отказ, а не молчаливая попытка разобрать файл как ZIP.
pub(crate) fn check_archive_type(value: &BslValue) -> RtResult<()> {
    match value {
        BslValue::Undefined => Ok(()),
        BslValue::Enum(EnumValue::ArchiveTypeZip) => Ok(()),
        BslValue::Enum(e) if e.kind() == EnumKind::ArchiveFileType => Err(zip_err(&format!(
            "формат архива «{}» не поддерживается",
            e.display_text()
        ))),
        _ => Err(RtError::TypeError {
            expected: "ТипФайлаАрхива",
            op: "ЧтениеФайлаАрхива",
        }),
    }
}

/// Байты источника вместе с его именем для сообщений.
pub(crate) fn read_source(source: &BslValue, op: &'static str) -> RtResult<(Vec<u8>, String)> {
    match source {
        BslValue::Str(s) => {
            let path = s.to_string();
            let bytes = std::fs::read(&path)
                .map_err(|e| zip_err(&format!("не удалось прочитать файл «{path}»: {e}")))?;
            Ok((bytes, path))
        }
        _ if source.byte_stream().is_some() => {
            let bytes = source
                .byte_stream()
                .expect("условие проверило протокол")
                .read_all(op)?;
            Ok((bytes, "поток".to_string()))
        }
        _ => Err(RtError::TypeError {
            expected: "Строка или Поток",
            op,
        }),
    }
}

/// Разобрать байты и сделать их состоянием открытого архива.
pub(crate) fn open_bytes(
    state: &Rc<RefCell<ArchiveState>>,
    data: Vec<u8>,
    source: String,
) -> RtResult<()> {
    let (entries, comment) = parse_archive(&data)?;
    let items = build_items(entries);
    // Комментарий декодируется так же, как имена, — lossy UTF-8 (измерено
    // на архиве с комментарием `привет-комментарий`).
    let comment = String::from_utf8_lossy(&comment).into_owned();
    let mut state = state.borrow_mut();
    state.open = Some(OpenArchive {
        data,
        source,
        comment,
        items,
    });
    // Номер открытия растёт только здесь и только после успешного разбора:
    // неудачное `Открыть` состав архива не меняет, а `Закрыть` его не
    // трогает — элементы закрытого архива и так отвергаются по `opened`.
    state.generation += 1;
    Ok(())
}

/// `Открыть(Источник[, Пароль])`.
///
/// # Errors
///
/// [`RtError::Zip`], если архив уже открыт или источник не является
/// читаемым архивом ZIP.
pub fn open(reader: &ReaderObject, args: &[BslValue]) -> RtResult<()> {
    let state = reader.state.clone();
    if let Some(open) = &state.borrow().open {
        return Err(zip_err(&format!("архив уже открыт: {}", open.source)));
    }
    let source = args.first().ok_or_else(|| RtError::MethodNotApplicable {
        method: "Открыть",
        receiver: reader.descriptor().name,
    })?;
    let (bytes, from) = read_source(source, "Открыть")?;
    open_bytes(&state, bytes, from)
}

/// `Закрыть()`.
///
/// # Errors
///
/// [`RtError::Zip`], если архив уже закрыт: измерено, что второй `Закрыть`
/// подряд платформа считает ошибкой, а не тихим повтором.
pub fn close(reader: &ReaderObject) -> RtResult<()> {
    let mut state = reader.state.borrow_mut();
    state.opened("Закрыть")?;
    state.open = None;
    Ok(())
}

/// Свойство `Элементы`.
///
/// # Errors
///
/// [`RtError::Zip`], если архив не открыт — измерено, что чтение свойства
/// на закрытом объекте это ошибка, а не пустая коллекция.
pub fn entries(reader: &ReaderObject) -> RtResult<BslValue> {
    reader.state.borrow().opened("Элементы")?;
    Ok(BslValue::new_object(EntriesObject {
        kind: reader.kind,
        state: reader.state.clone(),
    }))
}

/// Свойство `Комментарий` — комментарий всего архива.
///
/// # Errors
///
/// [`RtError::Zip`], если архив не открыт.
pub fn comment(reader: &ReaderObject) -> RtResult<BslValue> {
    let state = reader.state.borrow();
    Ok(BslValue::Str(BslString::from_str(
        &state.opened("Комментарий")?.comment,
    )))
}

/// Число элементов открытого архива.
///
/// # Errors
///
/// [`RtError::Zip`], если архив не открыт.
pub fn count(entries: &EntriesObject) -> RtResult<usize> {
    let state = entries.state.borrow();
    Ok(state.opened("Количество")?.items.len())
}

/// Элемент по номеру — общий путь `Коллекция[i]` и `Получить(i)`.
///
/// # Errors
///
/// [`RtError::Zip`], если архив не открыт; [`RtError::IndexOutOfBounds`],
/// если номера в архиве нет.
pub fn get(entries: &EntriesObject, index: usize) -> RtResult<BslValue> {
    let (len, generation) = {
        let state = entries.state.borrow();
        (state.opened("Получить")?.items.len(), state.generation)
    };
    if index >= len {
        return Err(RtError::IndexOutOfBounds {
            index: index as i64,
            len,
        });
    }
    Ok(BslValue::new_object(EntryObject {
        kind: entries.kind,
        state: entries.state.clone(),
        index,
        generation,
    }))
}

/// `Найти(Имя)` — первый элемент с таким ИСХОДНЫМ коротким именем,
/// `Неопределено`, если такого нет.
///
/// Сравнение идёт с `ИсходноеИмя` и без учёта регистра — измерено:
/// `Найти("ШУМ.BIN")` находит `шум.bin`, `Найти("вложенный.txt")` находит
/// запись `папка/вложенный.txt` (то есть ищется короткое имя, а не полное),
/// `Найти("")` находит запись-каталог `папка/`, а `Найти("отчёт:2026.txt")`
/// находит запись с этим ИСХОДНЫМ именем, хотя её `Имя` после подстановки —
/// `отчёт_2026.txt`. Обратное не работает: `Найти("отчёт_2026.txt")`,
/// `Найти("дубль(1).txt")` и `Найти("папка/вложенный.txt")` дают
/// `Неопределено` — по отображаемому имени, по имени после разрешения
/// дублей и по полному имени платформа не ищет.
///
/// ОДНА измеренная точка сюда не укладывается, и правила за ней найти не
/// удалось: на архиве с записью `a:b.txt` платформа не находит её ни по
/// `a:b.txt`, ни по `a_b.txt`, тогда как `отчёт:2026.txt` по исходному
/// имени находится. Похоже на разбор `a:` как имени диска, но проверить
/// это нечем, и здесь работает общее правило — такая запись находится.
///
/// # Errors
///
/// [`RtError::Zip`], если архив не открыт.
pub fn find(entries: &EntriesObject, name: &BslValue) -> RtResult<BslValue> {
    let wanted = text_of(name, "Найти")?.to_string().to_uppercase();
    let (found, generation) = {
        let state = entries.state.borrow();
        let open = state.opened("Найти")?;
        (
            open.items
                .iter()
                .position(|i| i.orig_name.to_uppercase() == wanted),
            state.generation,
        )
    };
    match found {
        Some(index) => Ok(BslValue::new_object(EntryObject {
            kind: entries.kind,
            state: entries.state.clone(),
            index,
            generation,
        })),
        None => Ok(BslValue::Undefined),
    }
}

/// Свойства элемента архива.
///
/// Набор ИЗМЕРЕН перебором кандидатов: `Размер`, `МетодСжатия`,
/// `УровеньСжатия`, `Комментарий`, `ДатаМодификации`, `КонтрольнаяСумма`,
/// `CRC`, `Атрибуты`, `ЭтоКаталог` и `Индекс` платформа не знает вовсе.
/// Английского имени у `ВремяИзменения` нет: восемь правдоподобных
/// написаний (`ModificationTime`, `ModifiedAt`, `ModificationDate`,
/// `LastModified`, `DateModified`, `ModifiedTime`, `ChangeTime`,
/// `ModifiedDate`) 8.3.27 отвергает.
///
/// # Errors
///
/// [`RtError::UnknownColumn`], если такого свойства у элемента нет;
/// [`RtError::Zip`], если архив уже закрыт либо элемент получен при
/// предыдущем его открытии.
pub fn entry_prop(entry: &EntryObject, prop: &str) -> RtResult<BslValue> {
    let state = entry.state.borrow();
    let item = state.item(entry.index, entry.generation, "ЭлементZipФайла")?;

    let text = |s: String| Ok(BslValue::Str(BslString::from_str(&s)));
    // Размеры записи приходят из чужого каталога: у Zip64 это произвольные
    // восемь байт, ничем не ограниченные. Через `i64` их пускать нельзя —
    // старший бит завернулся бы в знак, и `РазмерНесжатого` отдал бы
    // отрицательное число, которого платформа дать не может; `i128`
    // вмещает любой `u64` без потерь.
    let number = |n: u64| Ok(BslValue::Number(BslNumber::from_parts(i128::from(n), 0)));
    match prop {
        _ if eq(prop, "Имя", "Name") => text(item.name.clone()),
        _ if eq(prop, "ПолноеИмя", "FullName") => text(item.full_name()),
        _ if eq(prop, "Путь", "Path") => text(item.path.clone()),
        _ if eq(prop, "ИмяБезРасширения", "BaseName") => {
            text(split_extension(&item.name).0.to_string())
        }
        _ if eq(prop, "Расширение", "Extension") => {
            text(split_extension(&item.name).1.to_string())
        }
        _ if eq(prop, "ИсходноеИмя", "OriginalName") => text(item.orig_name.clone()),
        _ if eq(prop, "ИсходноеПолноеИмя", "OriginalFullName") => {
            text(item.orig_full_name())
        }
        _ if eq(prop, "ИсходныйПуть", "OriginalPath") => text(item.orig_path.clone()),
        _ if eq(prop, "ИсходноеИмяБезРасширения", "OriginalBaseName") => {
            text(split_extension(&item.orig_name).0.to_string())
        }
        _ if eq(prop, "ИсходноеРасширение", "OriginalExtension") => {
            text(split_extension(&item.orig_name).1.to_string())
        }
        _ if eq(prop, "РазмерНесжатого", "UncompressedSize") => {
            number(item.entry.size())
        }
        _ if eq(prop, "РазмерСжатого", "CompressedSize") => {
            number(item.entry.compressed_size())
        }
        _ if eq(prop, "Зашифрован", "Encrypted") => {
            Ok(BslValue::Boolean(item.entry.is_encrypted()))
        }
        // Английского написания у этого свойства нет — измерено.
        _ if prop.eq_ignore_ascii_case("ВремяИзменения") => {
            Ok(BslValue::Date(item.entry.modified()))
        }
        _ => Err(RtError::UnknownColumn(prop.to_string())),
    }
}

/// Строка за значением — семантика приватного `BslValue::as_str`.
pub(crate) fn text_of<'a>(value: &'a BslValue, op: &'static str) -> RtResult<&'a BslString> {
    match value {
        BslValue::Str(s) => Ok(s),
        _ => Err(RtError::TypeError {
            expected: "Строка",
            op,
        }),
    }
}

/// Оба написания одного свойства.
pub(crate) fn eq(name: &str, ru: &str, en: &str) -> bool {
    name.eq_ignore_ascii_case(ru) || name.eq_ignore_ascii_case(en)
}

/// `Извлечь(Элемент, Каталог[, Режим][, Пароль])`.
///
/// Арность ИЗМЕРЕНА: два аргумента обязательны, пятый платформа отвергает,
/// а четвёртый — это пароль (на зашифрованном архиве, открытом без пароля,
/// `Извлечь(Э, Куда, Режим, "pass123")` распаковывает запись). Расшифровки
/// здесь нет, поэтому пароль принимается и не используется.
///
/// # Errors
///
/// [`RtError::Zip`] на закрытом архиве, на элементе чужого архива, на
/// элементе, полученном при другом открытии ЭТОГО архива, на зашифрованной
/// записи, на неподдержанном способе хранения и на любой ошибке записи
/// файла; [`RtError::TypeError`], если первым аргументом передан не элемент
/// архива, а вторым — не строка.
pub fn extract(
    state: &Rc<RefCell<ArchiveState>>,
    receiver: &'static str,
    args: &[BslValue],
) -> RtResult<()> {
    let (item, dir, mode) = match args {
        [item, dir] => (item, dir, None),
        [item, dir, mode] => (item, dir, Some(mode)),
        // Четвёртый аргумент — пароль; см. doc comment.
        [item, dir, mode, _] => (item, dir, Some(mode)),
        _ => {
            return Err(RtError::MethodNotApplicable {
                method: "Извлечь",
                receiver,
            });
        }
    };
    // Элемент обязан быть из ЭТОГО архива: сам он свой читатель помнит, но
    // `Извлечь` — метод читателя, и распаковывать чужую запись, ничего не
    // сказав, хуже, чем отказать. Тождества состояния для этого мало — оно
    // переживает переоткрытие, — и номер открытия сверяет
    // `ArchiveState::item` ниже.
    let (index, generation) = match item
        .object_ref()
        .and_then(|object| object.downcast_ref::<EntryObject>())
    {
        Some(entry) if Rc::ptr_eq(&entry.state, state) => (entry.index, entry.generation),
        Some(_) => return Err(zip_err("элемент принадлежит другому архиву")),
        None => {
            return Err(RtError::TypeError {
                expected: "ЭлементZipФайла",
                op: "Извлечь",
            });
        }
    };

    let restore = restore_paths(mode, "Извлечь")?;
    let dir = destination(dir, "Извлечь")?;
    let state = state.borrow();
    let open = state.opened("Извлечь")?;
    extract_item(
        open,
        state.item(index, generation, "Извлечь")?,
        &dir,
        restore,
    )
}

/// `ИзвлечьВсе(Каталог[, Режим])`.
///
/// # Errors
///
/// Те же, что у [`extract`], плюс [`RtError::Zip`] на записи с пустым
/// именем: распаковать её некуда (платформа на таком архиве тоже
/// отказывает).
pub fn extract_all(
    state: &Rc<RefCell<ArchiveState>>,
    receiver: &'static str,
    args: &[BslValue],
) -> RtResult<()> {
    let (dir, mode) = match args {
        [dir] => (dir, None),
        [dir, mode] => (dir, Some(mode)),
        _ => {
            return Err(RtError::MethodNotApplicable {
                method: "ИзвлечьВсе",
                receiver,
            });
        }
    };
    let restore = restore_paths(mode, "ИзвлечьВсе")?;
    let dir = destination(dir, "ИзвлечьВсе")?;
    let state = state.borrow();
    let open = state.opened("ИзвлечьВсе")?;
    for item in &open.items {
        extract_item(open, item, &dir, restore)?;
    }
    Ok(())
}

/// Режим восстановления путей.
///
/// Умолчание — восстанавливать: измерено, что `ИзвлечьВсе(Куда)` без режима
/// создаёт подкаталоги. А вот ПЕРЕДАННОЕ `Неопределено` здесь не проходит —
/// тоже измерено, обоими методами сразу («Несоответствие типов (параметр
/// номер \'2\')» у `ИзвлечьВсе` и «номер \'3\'» у `Извлечь`), в отличие от
/// пароля, который `Неопределено` принимает. Поэтому пропущенный аргумент
/// это `None`, а не `Undefined`: у платформы «не передан» и «передано
/// Неопределено» — разные вещи, и различить их можно только по числу
/// фактических аргументов (пропуск в середине, `Ф(а, , б)`, эта реализация
/// не поддерживает вовсе).
pub(crate) fn restore_paths(mode: Option<&BslValue>, op: &'static str) -> RtResult<bool> {
    match mode {
        None => Ok(true),
        Some(BslValue::Enum(EnumValue::RestorePaths)) => Ok(true),
        Some(BslValue::Enum(EnumValue::DontRestorePaths)) => Ok(false),
        Some(_) => Err(RtError::TypeError {
            expected: "РежимВосстановленияПутейФайловZIP",
            op,
        }),
    }
}

/// Каталог назначения. Пустая строка — ошибка (измерено: «Некорректный путь
/// для распаковки»), а несуществующий каталог создаётся (тоже измерено).
pub(crate) fn destination(dir: &BslValue, op: &'static str) -> RtResult<std::path::PathBuf> {
    let dir = text_of(dir, op)?.to_string();
    if dir.is_empty() {
        return Err(zip_err("некорректный путь для распаковки"));
    }
    Ok(std::path::PathBuf::from(dir))
}

/// Распаковать одну запись.
pub(crate) fn extract_item(
    open: &OpenArchive,
    item: &ArchiveItem,
    dir: &std::path::Path,
    restore: bool,
) -> RtResult<()> {
    let mkdir = |path: &std::path::Path| {
        std::fs::create_dir_all(path).map_err(|e| {
            zip_err(&format!(
                "не удалось создать каталог «{}»: {e}",
                path.display()
            ))
        })
    };

    if item.entry.is_directory() {
        // В плоском режиме каталоги не создаются вовсе — измерено:
        // `ИзвлечьВсе(Куда, НеВосстанавливать)` на архиве с записью
        // `папка/` не оставляет никакой `папка`.
        if restore {
            mkdir(&dir.join(relative_path(item)))?;
        }
        return Ok(());
    }
    if item.name.is_empty() {
        return Err(zip_err("у записи архива пустое имя, распаковать её некуда"));
    }

    let target = if restore {
        dir.join(relative_path(item))
    } else {
        dir.join(&item.name)
    };
    if let Some(parent) = target.parent() {
        mkdir(parent)?;
    }
    let bytes = read_entry(&open.data, item.index, &item.entry)?;
    std::fs::write(&target, bytes)
        .map_err(|e| zip_err(&format!("не удалось записать «{}»: {e}", target.display())))
}

/// Путь записи относительно каталога распаковки.
///
/// Пустые компоненты выбрасываются — измерено, что `папка//двойной.txt`
/// платформа кладёт в `папка/двойной.txt`, хотя `ПолноеИмя` показывает обе
/// подряд идущие черты. Компоненты `.` и `..` после подстановки не
/// возникают (см. [`sanitize_component`]), но проверка оставлена: путь
/// строится из чужих данных, и выход вверх по дереву не должен зависеть от
/// рассуждения о другой функции.
pub(crate) fn relative_path(item: &ArchiveItem) -> std::path::PathBuf {
    let mut path = std::path::PathBuf::new();
    for part in item.full_name().split('/') {
        if part.is_empty() || part == "." || part == ".." {
            continue;
        }
        path.push(part);
    }
    path
}
