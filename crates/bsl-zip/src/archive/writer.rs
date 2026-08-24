//! Поверхность записи: `ЗаписьZipФайла` и сборка контейнера.

use super::*;

// --------------------------------------------------------------------------
// Писатель встроенного языка
// --------------------------------------------------------------------------

/// Куда писатель денет архив по `Записать()`.
///
/// Цель ЛЕНИВА: измерено, что после `Новый ЗаписьZipФайла(имя)` файла ещё
/// нет и появляется он только на `Записать()`, а несуществующий каталог
/// платформа не заводит, а объявляет ошибкой («Каталог не обнаружен»).
pub(crate) enum WriteTarget {
    /// Имя файла. Существующий файл перезаписывается (измерено на цели,
    /// в которой лежал посторонний текст).
    File(std::path::PathBuf),
    /// Поток. `Записать()` его НЕ закрывает — измерено: после `Записать`
    /// ручной `Закрыть` потока проходит.
    Stream(BslValue),
}

/// Способ сжатия из `МетодСжатияZIP`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum WriteMethod {
    /// `Сжатие` / `Deflate` — способ хранения 8, умолчание конструктора.
    #[default]
    Deflate,
    /// `Копирование` / `Copy` — способ хранения 0, данные как есть
    /// (измерено: `сжат` совпадает с `разм` до байта).
    Stored,
}

/// Режим сохранения путей (`РежимСохраненияПутейZIP`).
///
/// Умолчание ИЗМЕРЕНО и оно неочевидно: пропущенный аргумент ведёт себя как
/// `НеСохранятьПути`, а не как `СохранятьОтносительныеПути`. На одном и том
/// же дереве `Добавить(маска, , Рекурсивно)` даёт плоские `f1.txt`,
/// `f2.txt`, а `Добавить(маска, СохранятьОтносительныеПути, Рекурсивно)` —
/// `a/f1.txt`, `a/b/f2.txt`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PathMode {
    /// Путь относительно каталога маски.
    Relative,
    /// Полный путь без ведущего слэша (измерено: `/tmp/x/f.txt` ложится
    /// как `tmp/x/f.txt`).
    Full,
    /// Только имя файла.
    Flat,
}

/// Режим обхода подкаталогов (`РежимОбработкиПодкаталоговZIP`).
/// Умолчание — `НеОбрабатывать` (измерено).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SubdirMode {
    Skip,
    Recurse,
}

/// Запись, накопленная `Добавить` и ещё не выложенная в архив.
///
/// Данные читаются немедленно в `Добавить` — у платформы `Добавить` тоже
/// читает файл в момент вызова, а не при записи архива («Файл не
/// обнаружен» — ответ `Добавить`, а не `Записать`). Сжатие же откладывается
/// до `Записать`: крейт `zip` делает его сам при `write_all`.
pub(crate) struct PendingEntry {
    /// Имя в архиве: прямые слэши, у каталога — слэш на конце.
    pub(crate) name: String,
    /// Несжатые данные.
    pub(crate) data: Vec<u8>,
    pub(crate) method: WriteMethod,
    /// Время и дата MS-DOS из времени изменения исходного файла.
    pub(crate) time: u16,
    pub(crate) date: u16,
    /// Запись-каталог: данных нет, а внешние атрибуты помечают её каталогом.
    pub(crate) directory: bool,
}

/// Состояние писателя — `ЗаписьZipФайла` либо `ЗаписьФайлаАрхива`.
///
/// Отличий от читателя два, и оба измерены. Во-первых, «открыт» здесь
/// значит «есть цель»: `ПолучитьДвоичныеДанные()` на писателе с целью
/// отвечает «Архив уже открыт!», а `Записать()` на писателе без цели —
/// «Архив не открыт!». Во-вторых, `Записать()` не только выкладывает
/// архив, но и ОЧИЩАЕТ накопленное: после него тот же файл добавляется
/// повторно без ошибки о дубле, а `ПолучитьДвоичныеДанные()` отдаёт архив
/// только из того, что добавлено после.
#[derive(Default)]
pub struct WriterState {
    pub(crate) target: Option<WriteTarget>,
    pub(crate) method: WriteMethod,
    pub(crate) comment: String,
    pub(crate) entries: Vec<PendingEntry>,
    /// Имена, ПО КОТОРЫМ проверяется уникальность, — до подстановки полного
    /// пути пустому имени (см. [`plan_name`]). Именно поэтому два пустых
    /// каталога в плоском режиме сталкиваются друг с другом, хотя в архив
    /// легли бы с разными полными путями.
    pub(crate) used: Vec<String>,
}

impl std::fmt::Debug for WriterState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WriterState")
            .field("open", &self.target.is_some())
            .field("method", &self.method)
            .field("entries", &self.entries.len())
            .finish()
    }
}

/// `Новый ЗаписьZipФайла([Файл][, Пароль][, Комментарий][, МетодСжатия]
/// [, УровеньСжатия][, МетодШифрования][, КодировкаИмён])` и
/// `Новый ЗаписьФайлаАрхива([Файл][, Пароль][, ТипФайлаАрхива]
/// [, Комментарий][, ...])`.
///
/// Хвосты у двух типов РАЗНЫЕ, и это измерено по одному аргументу: у
/// zip-варианта третий — комментарий (строка проходит и оказывается
/// комментарием архива), у архивного третий — `ТипФайлаАрхива` (строка,
/// число, булево и члены остальных перечислений отвергаются с
/// «Несоответствие типов (параметр номер '3')»), а комментарий у него
/// четвёртый. Мест у них тоже разное число: у zip-варианта семь, у
/// архивного восемь — измерено с настоящим путём первым аргументом (см.
/// якорь `ZIP.WRITER.TAIL` и `tests/conformance/measure/measure-zip.bsl`).
///
/// # Errors
///
/// [`RtError::Zip`] на непустом пароле, на любом методе шифрования, на
/// методе сжатия BZIP2 и на формате архива, кроме ZIP: всё это платформа
/// умеет, а здесь честный отказ вместо тихой подмены;
/// [`RtError::TypeError`], если аргумент не того типа.
pub fn new_archive_writer(zip: bool, args: &[BslValue]) -> RtResult<BslValue> {
    let kind = if zip {
        ArchiveKind::Zip
    } else {
        ArchiveKind::Archive
    };
    let state = Rc::new(RefCell::new(WriterState::default()));
    configure(kind, &state, args, "ЗаписьZipФайла")?;
    Ok(BslValue::new_object(WriterObject { kind, state }))
}

/// `Открыть(Файл[, ...])` у писателя — те же аргументы, что у его
/// конструктора.
///
/// # Errors
///
/// [`RtError::Zip`], если архив уже открыт (измерено: «Архив уже открыт!:
/// <путь>») либо если запрошено неподдержанное; [`RtError::TypeError`] на
/// аргументе не того типа.
pub fn writer_open(writer: &WriterObject, args: &[BslValue]) -> RtResult<()> {
    let kind = writer.kind;
    let state = writer.state.clone();
    if args.is_empty() {
        return Err(RtError::MethodNotApplicable {
            method: "Открыть",
            receiver: writer.descriptor().name,
        });
    }
    if state.borrow().target.is_some() {
        return Err(zip_err("архив уже открыт"));
    }
    configure(kind, &state, args, "Открыть")
}

/// Разобрать общий для конструктора и `Открыть` список аргументов.
pub(crate) fn configure(
    kind: ArchiveKind,
    state: &Rc<RefCell<WriterState>>,
    args: &[BslValue],
    op: &'static str,
) -> RtResult<()> {
    let arg = |i: usize| args.get(i).unwrap_or(&BslValue::Undefined);
    let target = write_target(arg(0), op)?;
    check_password(arg(1))?;

    let (comment, method) = match kind {
        // `ЗаписьZipФайла`: комментарий, метод, уровень, шифрование,
        // кодировка имён.
        ArchiveKind::Zip => {
            let comment = optional_text(arg(2), op)?;
            let method = write_method(arg(3), op)?;
            check_level(arg(4), op)?;
            check_encryption(arg(5), op)?;
            check_names_encoding(arg(6), op)?;
            (comment, method)
        }
        // `ЗаписьФайлаАрхива`: тип архива, комментарий — и хвост из четырёх
        // мест, которые принимают ТОЛЬКО пустое. Это измерено направленно
        // (якорь `ZIP.WRITER.TAIL`): пустой хвост любой длины до восьмого
        // места включительно платформа принимает, а непустое пятое отвергает
        // на семнадцати типах — строке, числе, дате, булеве, массиве,
        // структуре, соответствии, `ДвоичныеДанные`, `ПотокВПамяти` и членах
        // всех семи перечислений ZIP вместе с `ТипФайлаАрхива`; шестое и
        // седьмое — на пяти и четырёх из тех же типов, восьмое — на строке и
        // двух перечислениях.
        // Отказ здесь честнее догадки: любой домысленный тип означал бы
        // МОЛЧАЛИВОЕ игнорирование настройки, которую платформа явно чем-то
        // считает.
        ArchiveKind::Archive => {
            check_archive_type(arg(2))?;
            let comment = optional_text(arg(3), op)?;
            for (i, value) in args.iter().enumerate().skip(4) {
                if !matches!(value, BslValue::Undefined) {
                    return Err(RtError::TypeError {
                        expected: "Неопределено",
                        op: match i {
                            4 => "ЗаписьФайлаАрхива (аргумент 5)",
                            5 => "ЗаписьФайлаАрхива (аргумент 6)",
                            6 => "ЗаписьФайлаАрхива (аргумент 7)",
                            _ => "ЗаписьФайлаАрхива (аргумент 8)",
                        },
                    });
                }
            }
            (comment, WriteMethod::Deflate)
        }
    };

    let mut state = state.borrow_mut();
    state.target = target;
    state.method = method;
    state.comment = comment;
    Ok(())
}

/// Первый аргумент: имя файла, поток либо ничего.
///
/// `ДвоичныеДанные` платформа НЕ принимает (измерено: «Несоответствие типов
/// (параметр номер '1') (Некорректное имя файла)»), пустую строку — тоже
/// («Некорректное имя файла»).
pub(crate) fn write_target(source: &BslValue, op: &'static str) -> RtResult<Option<WriteTarget>> {
    match source {
        BslValue::Undefined => Ok(None),
        BslValue::Str(s) => {
            let path = s.to_string();
            if path.is_empty() {
                return Err(zip_err("некорректное имя файла"));
            }
            Ok(Some(WriteTarget::File(std::path::PathBuf::from(path))))
        }
        _ if source.byte_stream().is_some() => Ok(Some(WriteTarget::Stream(source.clone()))),
        _ => Err(RtError::TypeError {
            expected: "Строка или Поток",
            op,
        }),
    }
}

/// Пароль. Шифрования здесь нет ни одного вида, поэтому непустой пароль —
/// отказ, а не молчаливо НЕзашифрованный архив: платформа с паролем пишет
/// архив, у которого `Зашифрован` = «Да» (измерено), и отдать вместо него
/// открытые данные было бы худшим из возможных ответов.
pub(crate) fn check_password(password: &BslValue) -> RtResult<()> {
    match password {
        BslValue::Undefined => Ok(()),
        BslValue::Str(s) if s.to_string().is_empty() => Ok(()),
        BslValue::Str(_) => Err(zip_err(
            "шифрование архива не поддерживается: уберите пароль",
        )),
        _ => Err(RtError::TypeError {
            expected: "Строка",
            op: "Пароль",
        }),
    }
}

/// Необязательный строковый аргумент — у обоих писателей это комментарий
/// архива, только на разных местах.
pub(crate) fn optional_text(value: &BslValue, op: &'static str) -> RtResult<String> {
    match value {
        BslValue::Undefined => Ok(String::new()),
        BslValue::Str(s) => Ok(s.to_string()),
        _ => Err(RtError::TypeError {
            expected: "Строка",
            op,
        }),
    }
}

/// `МетодСжатияZIP`. BZIP2 платформа умеет (измерено: 25 байт данных легли
/// в 60), а здесь его нет — честный отказ.
pub(crate) fn write_method(value: &BslValue, op: &'static str) -> RtResult<WriteMethod> {
    match value {
        BslValue::Undefined => Ok(WriteMethod::Deflate),
        BslValue::Enum(EnumValue::ZipMethodDeflate) => Ok(WriteMethod::Deflate),
        BslValue::Enum(EnumValue::ZipMethodCopy) => Ok(WriteMethod::Stored),
        BslValue::Enum(EnumValue::ZipMethodBzip2) => Err(zip_err(
            "метод сжатия BZIP2 не поддерживается, доступны «Сжатие» и «Копирование»",
        )),
        _ => Err(RtError::TypeError {
            expected: "МетодСжатияZIP",
            op,
        }),
    }
}

/// `УровеньСжатияZIP` принимается и на байты НЕ влияет.
///
/// Уровни у платформы различимы, и разница измерена на 19297 байтах:
/// `Минимальный` — 623 байта, `Оптимальный` — 628, `Максимальный` — 633
/// (да, именно в таком порядке). Здесь deflate всегда работает на уровне 1
/// (самый быстрый поиск совпадений в `flate2`), а уровень платформы
/// игнорируется — поэтому все три значения дают одни и те же байты.
/// Совместимости это не касается: уровень не записывается в формат и на
/// распаковку не влияет.
pub(crate) fn check_level(value: &BslValue, op: &'static str) -> RtResult<()> {
    match value {
        BslValue::Undefined => Ok(()),
        BslValue::Enum(e) if e.kind() == EnumKind::ZipCompressionLevel => Ok(()),
        _ => Err(RtError::TypeError {
            expected: "УровеньСжатияZIP",
            op,
        }),
    }
}

/// `МетодШифрованияZIP` — любой означает шифрование, которого здесь нет.
pub(crate) fn check_encryption(value: &BslValue, op: &'static str) -> RtResult<()> {
    match value {
        BslValue::Undefined => Ok(()),
        BslValue::Enum(e) if e.kind() == EnumKind::ZipEncryptionMethod => Err(zip_err(&format!(
            "шифрование «{}» не поддерживается",
            e.display_text()
        ))),
        _ => Err(RtError::TypeError {
            expected: "МетодШифрованияZIP",
            op,
        }),
    }
}

/// `КодировкаИменФайловВZipФайле`. Имена здесь всегда UTF-8 с битом 11 —
/// ровно то, что платформа пишет и по умолчанию (`Авто`), и по явному
/// `UTF8`: в её собственном архиве имя записи лежит в UTF-8, флаг 0x0800
/// выставлен.
pub(crate) fn check_names_encoding(value: &BslValue, op: &'static str) -> RtResult<()> {
    match value {
        BslValue::Undefined => Ok(()),
        BslValue::Enum(e) if e.kind() == EnumKind::ZipFileNamesEncoding => Ok(()),
        _ => Err(RtError::TypeError {
            expected: "КодировкаИменФайловВZipФайле",
            op,
        }),
    }
}

/// `Добавить(Путь[, РежимСохраненияПутей][, РежимОбработкиПодкаталогов])`.
///
/// Арность ИЗМЕРЕНА: четвёртый аргумент — «Слишком много фактических
/// параметров». Пропущенные режимы — не `Неопределено`: переданное
/// `Неопределено` платформа отвергает («Несоответствие типов (параметр
/// номер '2')»), а пропуск означает `НеСохранятьПути` и
/// `НеОбрабатывать`.
///
/// # Errors
///
/// [`RtError::Zip`], если файла или каталога маски нет, если имя в архиве
/// уже занято (измерено: «Файл с таким именем в архиве уже существует») или
/// если файл не читается; [`RtError::TypeError`] на режиме не того типа.
pub fn writer_add(
    writer: &WriterObject,
    files: &dyn FileSystem,
    args: &[BslValue],
) -> RtResult<()> {
    let state = &writer.state;
    let (path, mode, subdirs) = match args {
        [path] => (path, None, None),
        [path, mode] => (path, Some(mode), None),
        [path, mode, subdirs] => (path, Some(mode), Some(subdirs)),
        _ => {
            return Err(RtError::MethodNotApplicable {
                method: "Добавить",
                receiver: writer.descriptor().name,
            });
        }
    };
    let path = add_path(path)?;
    let mode = path_mode(mode)?;
    let subdirs = subdir_mode(subdirs)?;
    let mut state = state.borrow_mut();
    add_by_pattern(&mut state, files, &path, mode, subdirs)
}

/// Путь или маска первым аргументом `Добавить`.
///
/// Строгой типизации здесь у платформы НЕТ: `Добавить(1)` она встречает не
/// «Несоответствием типов», а «Файл не обнаружен '1'» — то есть число
/// становится именем. Пустая строка и `Неопределено` — «Некорректное имя
/// файла».
pub(crate) fn add_path(value: &BslValue) -> RtResult<String> {
    let text = match value {
        BslValue::Str(s) => s.to_string(),
        BslValue::Number(n) => n.to_string(),
        _ => return Err(zip_err("некорректное имя файла")),
    };
    if text.is_empty() {
        return Err(zip_err("некорректное имя файла"));
    }
    Ok(text)
}

/// Второй аргумент `Добавить`.
pub(crate) fn path_mode(mode: Option<&BslValue>) -> RtResult<PathMode> {
    match mode {
        None => Ok(PathMode::Flat),
        Some(BslValue::Enum(EnumValue::ZipStoreRelativePath)) => Ok(PathMode::Relative),
        Some(BslValue::Enum(EnumValue::ZipStoreFullPath)) => Ok(PathMode::Full),
        Some(BslValue::Enum(EnumValue::ZipDontStorePath)) => Ok(PathMode::Flat),
        Some(_) => Err(RtError::TypeError {
            expected: "РежимСохраненияПутейZIP",
            op: "Добавить",
        }),
    }
}

/// Третий аргумент `Добавить`.
pub(crate) fn subdir_mode(mode: Option<&BslValue>) -> RtResult<SubdirMode> {
    match mode {
        None => Ok(SubdirMode::Skip),
        Some(BslValue::Enum(EnumValue::ZipDontProcessSubdirs)) => Ok(SubdirMode::Skip),
        Some(BslValue::Enum(EnumValue::ZipProcessSubdirsRecursively)) => Ok(SubdirMode::Recurse),
        Some(_) => Err(RtError::TypeError {
            expected: "РежимОбработкиПодкаталоговZIP",
            op: "Добавить",
        }),
    }
}

/// Маска ли это — знаки `*` и `?` ищутся только в ПОСЛЕДНЕЙ компоненте.
///
/// ИЗМЕРЕНО, что маска в середине пути маской не считается:
/// `Добавить("/т/*/*.txt")` не находит ничего (и кладёт запись-каталог
/// самого `/т/`), а `Добавить("/т/по?/вложенный.txt")` отвечает «Файл не
/// обнаружен» с этим самым путём, знаки вопроса и всё.
pub(crate) fn split_pattern(path: &str) -> (String, String) {
    // Обратный слэш считается разделителем, как и у читателя, где это
    // измерено на именах записей. Побочное следствие: файл, в имени
    // которого на этой файловой системе законно стоит `\`, ляжет в архив
    // под именем после последнего такого знака.
    let normalized = path.replace('\\', "/");
    match normalized.rfind('/') {
        Some(at) => (
            normalized[..at + 1].to_string(),
            normalized[at + 1..].to_string(),
        ),
        None => (String::new(), normalized),
    }
}

/// Совпадение имени с маской `*`/`?`.
///
/// Сравнение с учётом регистра: на этой платформе имена файлов
/// регистрозависимы, и маска `*.txt` файла `ВЕРХ.TXT` не находит
/// (измерено).
pub(crate) fn mask_matches(mask: &str, name: &str) -> bool {
    let mask: Vec<char> = mask.chars().collect();
    let name: Vec<char> = name.chars().collect();
    // Классический двухуказательный разбор со звёздочкой-точкой возврата:
    // рекурсия по маске из чужого ввода могла бы уйти сколь угодно глубоко.
    let (mut m, mut n) = (0usize, 0usize);
    let (mut star, mut back) = (usize::MAX, 0usize);
    while n < name.len() {
        if m < mask.len() && (mask[m] == '?' || mask[m] == name[n]) {
            m += 1;
            n += 1;
        } else if m < mask.len() && mask[m] == '*' {
            star = m;
            back = n;
            m += 1;
        } else if star != usize::MAX {
            back += 1;
            m = star + 1;
            n = back;
        } else {
            return false;
        }
    }
    while m < mask.len() && mask[m] == '*' {
        m += 1;
    }
    m == mask.len()
}

/// Разложить `Добавить` на записи и сложить их в состояние.
pub(crate) fn add_by_pattern(
    state: &mut WriterState,
    files: &dyn FileSystem,
    path: &str,
    mode: PathMode,
    subdirs: SubdirMode,
) -> RtResult<()> {
    let (base, mut pattern) = split_pattern(path);
    // Слэш на конце — это ИЗМЕРЕННОЕ сокращение для `<каталог>/*`:
    // `Добавить("/т/под/")` кладёт ровно то же, что `Добавить("/т/под/*")`,
    // тогда как тот же каталог, названный без слэша, платформа молча
    // пропускает. Разбор оставил здесь пустую маску, поэтому подставляем
    // всесовпадающую и уходим в общую ветку маски — с тем же базовым
    // каталогом и тем же режимом подкаталогов.
    if pattern.is_empty()
        && (path.ends_with('/') || path.ends_with('\\'))
        && files.metadata(&base).is_ok_and(|meta| meta.is_dir())
    {
        pattern = "*".to_string();
    }
    if !pattern.contains('*') && !pattern.contains('?') {
        // Не маска, а имя. Каталог по такому имени платформа молча
        // пропускает: `Добавить("/т/под")` — «прошло» и ноль записей, даже
        // с рекурсией.
        let meta = files
            .metadata(path)
            .map_err(|_| zip_err(&format!("файл не обнаружен «{path}»")))?;
        if meta.is_dir() {
            return Ok(());
        }
        let name = plan_name(mode, &pattern, &pattern, path, false);
        return add_file(state, files, path, name, meta.modified());
    }

    // Путь файловой системы базового каталога: пустой — это «.», как и
    // раньше строил `PathBuf::from(".")`. Отображаемое имя (`base`) при
    // этом остаётся прежним — на нём строятся имена записей.
    let dir_path = if base.is_empty() { "." } else { &base };
    let meta = files.metadata(dir_path).map_err(|_| {
        // Платформа называет в этой ошибке каталог со слэшем, а не всю
        // маску (измерено).
        zip_err(&format!("файл не обнаружен «{base}»"))
    })?;
    if !meta.is_dir() {
        return Err(zip_err(&format!("файл не обнаружен «{base}»")));
    }
    walk_dir(
        state, files, dir_path, &base, "", &pattern, mode, subdirs, true,
    )
}

/// Путь дочернего элемента каталога — тем же способом, что `entry.path()`
/// (`Path::join`), только над строкой: `FileSystem` работает с `&str`.
fn child_path(dir: &str, name: &str) -> String {
    std::path::Path::new(dir)
        .join(name)
        .to_string_lossy()
        .into_owned()
}

/// Обойти один каталог: файлы по маске, подкаталоги — вглубь на месте.
///
/// Порядок обхода — тот, что отдаёт файловая система, и это ИЗМЕРЕНО: на
/// каталоге, где `ls -U` показывает `данные.dat`, `файл2.txt`, `файл1.txt`,
/// платформа кладёт записи в этом же порядке, а не по алфавиту и не по
/// времени. Подкаталог обходится ровно на своём месте в этом порядке.
///
/// `selected` — каталог назван самой маской (или это её базовый каталог).
/// От этого зависит запись-каталог: ПУСТОЙ выбранный каталог не даёт
/// ничего, а пустой каталог, до которого дошла рекурсия, даёт запись
/// (измерено обоими способами на одном дереве).
#[allow(clippy::too_many_arguments)]
pub(crate) fn walk_dir(
    state: &mut WriterState,
    files: &dyn FileSystem,
    dir: &str,
    dir_display: &str,
    rel: &str,
    mask: &str,
    mode: PathMode,
    subdirs: SubdirMode,
    selected: bool,
) -> RtResult<()> {
    let reader = files.read_dir(dir).map_err(|e| {
        zip_err(&format!(
            "не удалось прочитать каталог «{dir_display}»: {e}"
        ))
    })?;
    let mut matched_here = 0usize;
    let mut children = 0usize;
    for entry in reader {
        let entry = entry.map_err(|e| {
            zip_err(&format!(
                "не удалось прочитать каталог «{dir_display}»: {e}"
            ))
        })?;
        children += 1;
        let name = entry.name().to_string();
        let path = child_path(dir, &name);
        // Каталог ли это — решается по САМОМУ элементу каталога, без
        // перехода по символической ссылке. Иначе ссылка на предка
        // зациклила бы рекурсию и уронила процесс переполнением стека, а
        // вход здесь чужой: маску задаёт скрипт, дерево — файловая система.
        if entry.is_dir() {
            if subdirs == SubdirMode::Recurse {
                let child_rel = format!("{rel}{name}/");
                let child_display = format!("{dir_display}{name}/");
                walk_dir(
                    state,
                    files,
                    &path,
                    &child_display,
                    &child_rel,
                    mask,
                    mode,
                    subdirs,
                    mask_matches(mask, &name),
                )?;
            }
            continue;
        }
        if !mask_matches(mask, &name) {
            continue;
        }
        // А вот содержимое и время берутся по ПУТИ: ссылка на файл ложится
        // в архив тем, на что указывает.
        let Ok(meta) = files.metadata(&path) else {
            continue;
        };
        matched_here += 1;
        let rel_name = format!("{rel}{name}");
        let full = format!("{dir_display}{name}");
        let planned = plan_name(mode, &name, &rel_name, &full, false);
        add_file(state, files, &path, planned, meta.modified())?;
    }

    // Каталог, в котором маска не нашла ни одного файла, платформа
    // записывает САМА — записью-каталогом. Исключение одно: выбранный
    // маской (или заданный ею базовый) каталог, оказавшийся пустым, не даёт
    // ничего.
    if matched_here == 0 && !(selected && children == 0) {
        let planned = plan_name(mode, "", rel, dir_display, true);
        let stamp = files.metadata(dir).ok().and_then(|m| m.modified());
        add_directory(state, planned, stamp)?;
    }
    Ok(())
}

/// Имя записи по режиму путей: `(ключ, имя в архиве)`.
///
/// Ключ — то, по чему проверяется уникальность, а имя — то, что ложится в
/// архив. Расходятся они на пустом ключе: у записи-каталога в плоском
/// режиме и у базового каталога в относительном имя пусто, и платформа
/// подставляет вместо него ПОЛНЫЙ путь, продолжая при этом считать занятым
/// пустое имя. Отсюда измеренное: два пустых каталога в плоском режиме
/// сталкиваются («Файл с таким именем в архиве уже существует:  -
/// /т/пусто»), хотя в архив легли бы под разными полными путями.
pub(crate) fn plan_name(
    mode: PathMode,
    file_name: &str,
    rel: &str,
    full: &str,
    directory: bool,
) -> (String, String) {
    let full_key = {
        let stripped = full.strip_prefix('/').unwrap_or(full);
        if directory && !stripped.is_empty() && !stripped.ends_with('/') {
            format!("{stripped}/")
        } else {
            stripped.to_string()
        }
    };
    let key = match mode {
        PathMode::Flat => file_name.to_string(),
        PathMode::Relative => rel.to_string(),
        PathMode::Full => full_key.clone(),
    };
    let name = if key.is_empty() {
        full_key
    } else {
        key.clone()
    };
    (key, name)
}

/// Занять имя или отказать так же, как платформа.
pub(crate) fn reserve(state: &mut WriterState, key: String, source: &str) -> RtResult<()> {
    if state.used.contains(&key) {
        return Err(zip_err(&format!(
            "файл с таким именем в архиве уже существует: {key} — {source}"
        )));
    }
    state.used.push(key);
    Ok(())
}

/// Прочитать файл и запомнить запись.
pub(crate) fn add_file(
    state: &mut WriterState,
    files: &dyn FileSystem,
    path: &str,
    planned: (String, String),
    modified: Option<i64>,
) -> RtResult<()> {
    let (key, name) = planned;
    // Файл читается ДО занятия имени: если он нечитаем, имя не занимается, и
    // законный следующий файл с тем же плоским ключом не получит ложное «уже
    // существует» при пустом архиве (состояние-до-эффекта). После успешного
    // чтения `reserve` и `push` идут подряд без фаллибельных операций между
    // ними, то есть занятие имени и укладка записи атомарны.
    let data = files
        .read(path)
        .map_err(|e| zip_err(&format!("не удалось прочитать «{path}»: {e}")))?;
    if data.len() > u32::MAX as usize {
        return Err(zip_err(&format!("файл «{path}» больше 4 ГиБ")));
    }
    reserve(state, key, path)?;
    let (time, date) = dos_fields(modified);
    state.entries.push(PendingEntry {
        name,
        data,
        method: state.method,
        time,
        date,
        directory: false,
    });
    Ok(())
}

/// Запомнить запись-каталог: имя со слэшем, нулевые данные и время
/// изменения самого каталога.
pub(crate) fn add_directory(
    state: &mut WriterState,
    planned: (String, String),
    modified: Option<i64>,
) -> RtResult<()> {
    let (key, mut name) = planned;
    reserve(state, key, &name.clone())?;
    if !name.ends_with('/') {
        name.push('/');
    }
    // `dos_fields(None)` уже даёт `(0, 0)`.
    let (time, date) = dos_fields(modified);
    state.entries.push(PendingEntry {
        name,
        data: Vec::new(),
        method: WriteMethod::Stored,
        time,
        date,
        directory: true,
    });
    Ok(())
}

/// Время изменения файла в полях MS-DOS.
///
/// Момент берётся БЕЗ поправки на зону — так же, как его отдаёт
/// `ТекущаяДата` (см. `BslValue::current_date`): в `std` нет способа узнать
/// смещение локальной зоны, а тип даты в 1С зоны не хранит вовсе. Файл
/// раньше 1980 года формату не представим, поэтому такие даты зажимаются в
/// начало 1980-го — иначе поле года ушло бы в минус.
pub(crate) fn dos_fields(modified: Option<i64>) -> (u16, u16) {
    // `modified` — секунды Unix из `FileMetadata` (та же величина, что
    // раньше давал `SystemTime::duration_since(UNIX_EPOCH).as_secs()`),
    // поэтому байты полей не меняются.
    let Some(unix_secs) = modified else {
        return (0, 0);
    };
    let secs = unix_secs + UNIX_EPOCH_SECONDS;
    let Some(date) = BslDate::from_seconds(secs) else {
        return (0, 0);
    };
    let civil = date.to_civil();
    if civil.year < 1980 {
        // 1980-01-01 00:00:00 — наименьшее, что поля MS-DOS выражают.
        return (0, (1 << 5) | 1);
    }
    let year = u16::try_from(civil.year - 1980).unwrap_or(0) & 0x7F;
    let dos_date = (year << 9) | ((civil.month as u16 & 0xF) << 5) | (civil.day as u16 & 0x1F);
    let dos_time = ((civil.hour as u16 & 0x1F) << 11)
        | ((civil.minute as u16 & 0x3F) << 5)
        | ((civil.second as u16 / 2) & 0x1F);
    (dos_time, dos_date)
}

/// `Записать()` — выложить архив в цель и закрыть его.
///
/// # Errors
///
/// [`RtError::Zip`], если архив не открыт (измерено: «Архив не открыт!» —
/// в том числе на втором `Записать` подряд) либо если цель не пишется.
pub fn writer_write(writer: &WriterObject, files: &dyn FileSystem) -> RtResult<()> {
    let mut state = writer.state.borrow_mut();
    let Some(target) = state.target.take() else {
        return Err(zip_err("архив не открыт"));
    };
    let bytes = build_archive(&state.entries, &state.comment)?;
    // Цель снята со состояния ДО записи, поэтому неудачная запись тоже
    // оставляет архив закрытым. Что делает в этом случае платформа, НЕ
    // измерено (снят только сам отказ на несуществующем каталоге, без
    // повторной попытки); выбрано закрывать, потому что иначе повторный
    // `Записать` молча пытался бы писать туда же второй раз.
    state.entries.clear();
    state.used.clear();
    match target {
        WriteTarget::File(path) => files
            .write(&path.to_string_lossy(), &bytes)
            .map_err(|e| zip_err(&format!("не удалось записать «{}»: {e}", path.display()))),
        // Приёмник проверен при `Открыть`, но объект живёт между вызовами:
        // повторная проверка с типизированной ошибкой вместо `expect`. По
        // контракту `ObjectProtocol::byte_stream` (отвечает одинаково всю
        // жизнь объекта) сюда не попасть, но `expect` уронил бы процесс.
        WriteTarget::Stream(stream) => match stream.byte_stream() {
            Some(bs) => bs.write_all(&bytes, "Записать"),
            None => Err(RtError::TypeError {
                expected: "Поток",
                op: "Записать",
            }),
        },
    }
}

/// `ПолучитьДвоичныеДанные()` — архив из накопленного, не трогая цель.
///
/// # Errors
///
/// [`RtError::Zip`], если архив открыт: измерено, что на писателе с целью
/// платформа отвечает «Архив уже открыт!», и только после `Записать()` (или
/// у писателя, созданного без цели) отдаёт данные.
pub fn writer_binary_data(writer: &WriterObject) -> RtResult<BslValue> {
    let state = writer.state.borrow();
    if state.target.is_some() {
        return Err(zip_err("архив уже открыт"));
    }
    let bytes = build_archive(&state.entries, &state.comment)?;
    Ok(BslValue::binary_data_of(bytes))
}

/// Собрать архив из накопленных записей через крейт `zip`.
pub(crate) fn build_archive(entries: &[PendingEntry], comment: &str) -> RtResult<Vec<u8>> {
    let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    if !comment.is_empty() {
        zip.set_comment(comment);
    }
    for e in entries {
        let dt = zip::DateTime::try_from_msdos(e.date, e.time).unwrap_or_default();
        let method = match e.method {
            WriteMethod::Stored => zip::CompressionMethod::Stored,
            WriteMethod::Deflate => zip::CompressionMethod::Deflated,
        };
        // Уровень 1 (самый быстрый поиск совпадений) — только для deflate;
        // для Stored крейт `zip` отвергает任何 уровень.
        let mut options = zip::write::SimpleFileOptions::default()
            .compression_method(method)
            .last_modified_time(dt);
        if method == zip::CompressionMethod::Deflated {
            options = options.compression_level(Some(1));
        }
        if e.directory {
            zip.add_directory(&e.name, options)
                .map_err(zip_error_to_rt)?;
        } else {
            zip.start_file(&e.name, options).map_err(zip_error_to_rt)?;
            zip.write_all(&e.data)
                .map_err(|e| zip_err(&format!("ошибка записи: {e}")))?;
        }
    }
    let cursor = zip.finish().map_err(zip_error_to_rt)?;
    Ok(cursor.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Нечитаемый файл не занимает имя: прежде `add_file` вызывал `reserve`
    /// ДО чтения, и после отказа чтения законный следующий файл с тем же
    /// плоским ключом получал ложное «уже существует» при пустом архиве.
    #[test]
    fn an_unreadable_file_does_not_reserve_its_name() {
        let readable = std::env::temp_dir().join("open-bsl-zip-dup.txt");
        std::fs::write(&readable, b"payload").expect("создать читаемый файл");
        let readable = readable.to_string_lossy().into_owned();
        let mut state = WriterState {
            target: None,
            method: WriteMethod::Stored,
            comment: String::new(),
            entries: Vec::new(),
            used: Vec::new(),
        };
        let planned = ("dup.txt".to_string(), "dup.txt".to_string());
        assert!(
            add_file(
                &mut state,
                &SystemFileSystem,
                "/no/such/open-bsl/file",
                planned.clone(),
                None,
            )
            .is_err(),
            "нечитаемый файл — ошибка"
        );
        assert!(state.used.is_empty(), "имя нечитаемого файла не занято");
        assert!(
            add_file(&mut state, &SystemFileSystem, &readable, planned, None).is_ok(),
            "законный файл с тем же ключом добавляется"
        );
        assert_eq!(state.entries.len(), 1);
    }
}
