//! Поверхность встроенного языка: «ДокументPDF» и его коллекции.

use super::*;

// ---------------------------------------------------------------------------
// Поверхность встроенного языка: `ДокументPDF`
// ---------------------------------------------------------------------------
//
// Всё, что здесь есть, снято с 8.3.27 скриптом
// `tests/conformance/measure/measure-pdf-read.bsl`; его вывод лежит рядом
// в `.platform.txt` и сверяется построчно.
//
// * `Новый ДокументPDF` — БЕЗ аргументов: и путь, и `ДвоичныеДанные` в
//   конструкторе платформа отвергает;
// * `Прочитать(ИмяФайла)` берёт только имя файла: `ДвоичныеДанные`
//   отвергнуты, вызов без аргументов — ошибка;
// * `Страницы` до первого чтения — `Неопределено`, и после НЕУДАЧНОГО
//   чтения снова `Неопределено`: документ возвращается в непрочитанное
//   состояние, а ошибку в нём даёт уже `Количество()`;
// * коллекция `КоллекцияСтраницPDF` умеет `Количество()`, `Получить(i)`,
//   `Индекс(Страница)`, `[i]` и `Для Каждого`; `Получить` вне диапазона
//   отдаёт `Неопределено` (и на 99, и на -1), а `[i]` — ошибку;
// * `КоличествоСтраниц` у документа НЕТ (ошибка);
// * страница только ЧИТАЕТСЯ: присваивание в `Ширина` — ошибка.
//
// Вложения сняты тем же способом (скрипт задачи —
// `tests/conformance/measure/measure-pdf-attachments.bsl`), и устроены они
// НЕ так, как страницы:
//
// * `Вложения` есть ВСЕГДА, в том числе до `Прочитать`: это пустая
//   `КоллекцияВложенийPDF`, а не `Неопределено`, и в неё уже можно
//   добавлять;
// * `Новый КоллекцияВложенийPDF` платформа строит, а `Новый ВложениеPDF` —
//   нет: вложение появляется только через `Добавить`;
// * коллекция умеет `Количество()`, `Получить(i)`, `[i]`, `Индекс(Влож)`,
//   `Найти(Имя)`, `Добавить(Имя, Данные[, ТипСодержимого[, ТипСвязи]])`,
//   `Удалить(i)`, `Очистить()` и `Для Каждого`; `Вставить` не пробовали, а
//   потому и не заявляем — здесь его нет до замера;
// * у вложения ровно четыре свойства — `ИмяФайла`, `ТипСодержимого`,
//   `Содержимое`, `ТипСвязи`, — и все ЧЕТЫРЕ ПИШУТСЯ (в отличие от
//   страницы, которая только читается);
// * `Записать(ИмяФайла[, Пароль])` у документа — процедура;
// * членов про ЭЛЕКТРОННУЮ ПОДПИСЬ у «ДокументPDF» на 8.3.27 НЕТ вовсе:
//   пробовано шесть написаний — чтения свойств `ЭлектронныеПодписи` и
//   `Подписи`, вызовы `ПолучитьЭлектронныеПодписи()`,
//   `ДобавитьЭлектроннуюПодпись()`, `ПроверитьЭлектроннуюПодпись()` и
//   `Подписать()`, — и все шесть кончаются исключением (текст ошибки
//   скрипт не печатает, только «нет» из ветки `Исключение`); четырёх типов
//   `ПодписьPDF`, `ЭлектроннаяПодписьPDF`, `PDFSignature` и
//   `КоллекцияПодписейPDF` платформа не знает. Поэтому здесь их тоже нет:
//   честная ошибка «нет такого метода» приходит сама, из общих таблиц, и
//   заводить ради неё пустую заглушку значило бы придумать платформе
//   поверхность, которой у неё нет.

/// Состояние `ДокументPDF`: разобранный файл либо ничего, если чтения ещё
/// не было или последнее чтение не удалось.
///
/// Вложения живут ОТДЕЛЬНО от разобранного файла, и это не украшение:
/// измерено, что `Вложения` у свежего документа — пустая коллекция, в
/// которую можно добавлять ещё до `Прочитать`. Коллекция и её элементы
/// держат тот же `Rc`, поэтому `Док.Вложения = Док.Вложения` — «Да»
/// (измерено), а `Прочитать` заменяет СОДЕРЖИМОЕ вектора, из-за чего уже
/// полученная коллекция видит новые вложения.
#[derive(Debug, Default)]
pub struct PdfDocState {
    pub(crate) file: Option<PdfFile>,
    pub(crate) attachments: Rc<RefCell<Vec<PdfAttachment>>>,
}

/// `Новый ДокументPDF` — пустой документ без источника.
pub fn new_pdf_document(files: Rc<dyn bsl_rt::FileSystem>) -> BslValue {
    BslValue::new_object(DocumentObject {
        state: Rc::new(RefCell::new(PdfDocState::default())),
        files,
    })
}

/// `Новый КоллекцияВложенийPDF` — коллекция сама по себе, без документа.
///
/// Платформа такой конструктор ЗНАЕТ (измерено), хотя присоединить готовую
/// коллекцию к документу нечем: `Вложения` только читается. Значит, всё,
/// что с ней можно делать, — наполнять и разглядывать; ровно это здесь и
/// получается, потому что коллекция документа отличается от отдельной лишь
/// тем, кто ещё держит тот же `Rc`.
pub fn new_pdf_attachments() -> BslValue {
    BslValue::new_object(AttachmentsObject {
        items: Rc::new(RefCell::new(Vec::new())),
    })
}

/// `ДокументPDF` — держатель общего состояния: коллекция страниц и
/// страница — окна в тот же документ, а не снимки.
#[derive(Debug)]
pub struct DocumentObject {
    pub(crate) state: Rc<RefCell<PdfDocState>>,
    /// Файловая система сессии (ABI-G): пришла к документу при построении и
    /// держится здесь, потому что `Прочитать`/`Записать` — методы, а под JIT
    /// метод исполняется по натуральному пути без доступа к контексту.
    pub(crate) files: Rc<dyn bsl_rt::FileSystem>,
}

/// `КоллекцияСтраницPDF` — окно в тот же документ.
#[derive(Debug)]
pub struct PagesObject {
    pub(crate) state: Rc<RefCell<PdfDocState>>,
}

/// `СтраницаPDF` — то же состояние плюс НОМЕР страницы с нуля.
#[derive(Debug)]
pub struct PageObject {
    pub(crate) state: Rc<RefCell<PdfDocState>>,
    pub(crate) index: usize,
}

/// `КоллекцияВложенийPDF` — вектор вложений, ОБЩИЙ с документом, если
/// коллекция получена из него; отдельным `Rc` она бывает и сама по себе.
#[derive(Debug)]
pub struct AttachmentsObject {
    pub(crate) items: Rc<RefCell<Vec<PdfAttachment>>>,
}

/// `ВложениеPDF` — тот же вектор плюс НОМЕР вложения с нуля.
#[derive(Debug)]
pub struct AttachmentObject {
    pub(crate) items: Rc<RefCell<Vec<PdfAttachment>>>,
    pub(crate) index: usize,
}

pub(crate) static DOCUMENT_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ДокументPDF",
    type_display: "Документ PDF",
    type_names: &["PDFDocument"],
};

pub(crate) static PAGES_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "КоллекцияСтраницPDF",
    type_display: "КоллекцияСтраницPDF",
    type_names: &["PDFPagesCollection"],
};

pub(crate) static PAGE_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "СтраницаPDF",
    type_display: "СтраницаPDF",
    type_names: &["PDFPage"],
};

pub(crate) static ATTACHMENTS_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "КоллекцияВложенийPDF",
    type_display: "КоллекцияВложенийPDF",
    type_names: &["PDFAttachmentCollection"],
};

pub(crate) static ATTACHMENT_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ВложениеPDF",
    type_display: "ВложениеPDF",
    type_names: &["PDFAttachment"],
};

/// `ДокументPDF.Прочитать(ИмяФайла[, Пароль])`.
///
/// Пароль ПРИНИМАЕТСЯ И НЕ ИСПОЛЬЗУЕТСЯ, как у `Новый ЧтениеZipФайла`:
/// расшифровки здесь нет, и зашифрованный файл отвергается независимо от
/// того, назвали пароль или нет. Платформа с паролем такой файл читает
/// (измерено: `Прочитать(файл, "secret")` на RC4-40 отдало одну страницу),
/// и это единственное объявленное расхождение скрипта замеров чтения.
/// Симметричная граница на записи — в [`fn@write`].
///
/// Источником может быть ТОЛЬКО имя файла: `ДвоичныеДанные` платформа
/// отвергает (измерено). Принимает ли она ПОТОК, выяснить этой оснасткой
/// нельзя — на такой пробе платформа показывает модальное окно, то есть
/// немой таймаут, и в реестр открытых вопросов её не занести: одна строка
/// в `measure-all.bsl` унесла бы весь сеанс замеров. Поэтому здесь
/// принимается ровно то написание, которое измерено, — строка.
///
/// # Errors
///
/// [`RtError::Pdf`], если аргументов нет вовсе, если первым передано не имя
/// файла, если файл не читается с диска или не разбирается как PDF. При
/// любой ошибке документ возвращается в НЕПРОЧИТАННОЕ состояние — измерено,
/// что после неудачного чтения платформа снова отдаёт на `Страницы`
/// `Неопределено`, а ошибку даёт уже `Количество()` на нём. Вложения при
/// этом тоже забываются: измерено, что после неудачного чтения коллекция
/// пуста, а удачное чтение заменяет её содержимым нового файла.
pub fn read(document: &DocumentObject, args: &[BslValue]) -> RtResult<()> {
    let state = &document.state;
    // Сначала забываем прежнее, потом читаем: иначе неудача оставила бы
    // документ с чужими страницами. Вложения забываются тем же движением —
    // уже полученная коллекция при этом остаётся той же самой, меняется
    // только её содержимое.
    {
        let mut state = state.borrow_mut();
        state.file = None;
        state.attachments.borrow_mut().clear();
    }
    let (Some(source), true) = (args.first(), args.len() <= 2) else {
        return Err(pdf_err(
            "ДокументPDF.Прочитать ожидает имя файла и необязательный пароль",
        ));
    };
    let BslValue::Str(name) = source else {
        return Err(pdf_err(format!(
            "ДокументPDF.Прочитать ожидает имя файла строкой, получено «{}»",
            source.type_name()
        )));
    };
    let name = name.to_string();
    let bytes = document
        .files
        .read(&name)
        .map_err(|e| pdf_err(format!("не удалось прочитать файл «{name}»: {e}")))?;
    let file = PdfFile::parse(&bytes)?;
    {
        let mut state = state.borrow_mut();
        state.attachments.borrow_mut().clone_from(&file.attachments);
        state.file = Some(file);
    }
    Ok(())
}

/// `ДокументPDF.Записать(ИмяФайла[, Пароль])` — процедура (измерено:
/// обращение к ней как к функции платформа отвергает).
///
/// Пишется ИНКРЕМЕНТАЛЬНОЕ ОБНОВЛЕНИЕ поверх прочитанных байт, см.
/// [`PdfFile::write_with_attachments`]; поэтому документ, который ничего не
/// читал, записать нечем. Платформа в этом случае тоже отвечает ошибкой,
/// хотя и оставляет на диске файл с одной пустой страницей A4 — этого
/// последнего мы не делаем сознательно: пустую страницу неоткуда взять, а
/// придумать её значило бы записать не тот документ, который просили.
///
/// # Errors
///
/// [`RtError::Pdf`], если аргументов нет или их больше двух, если имя файла
/// не строка, если документ ещё ничего не прочитал, если задан непустой
/// пароль (шифрования здесь нет) или если файл не записался на диск.
pub fn write(document: &DocumentObject, args: &[BslValue]) -> RtResult<()> {
    let state = &document.state;
    let (Some(target), true) = (args.first(), args.len() <= 2) else {
        return Err(pdf_err(
            "ДокументPDF.Записать ожидает имя файла и необязательный пароль",
        ));
    };
    let BslValue::Str(name) = target else {
        return Err(pdf_err(format!(
            "ДокументPDF.Записать ожидает имя файла строкой, получено «{}»",
            target.type_name()
        )));
    };
    // Пароль ПРИНИМАЕТСЯ, но работать с ним нечем: шифрования здесь нет ни
    // на чтении, ни на записи. Платформа на незашифрованном документе
    // отвечает на любой непустой пароль «Неверный пароль», то есть тоже
    // ошибкой.
    if let Some(password) = args.get(1) {
        let empty = match password {
            BslValue::Str(text) => text.len_utf16() == 0,
            BslValue::Undefined => true,
            _ => false,
        };
        if !empty {
            return Err(pdf_err(
                "ДокументPDF.Записать: шифрование PDF не поддерживается, пароль неприменим",
            ));
        }
    }
    let state = state.borrow();
    let file = state.file.as_ref().ok_or_else(|| {
        pdf_err("ДокументPDF.Записать: документ ничего не прочитал, записывать нечего")
    })?;
    let bytes = file.write_with_attachments(&state.attachments.borrow())?;
    let name = name.to_string();
    document
        .files
        .write(&name, &bytes)
        .map_err(|e| pdf_err(format!("не удалось записать файл «{name}»: {e}")))?;
    Ok(())
}

/// Свойство `ДокументPDF.Страницы` (`Pages`).
///
/// До первого чтения — и после НЕУДАЧНОГО чтения — свойство отдаёт
/// `Неопределено`, а не ошибку и не пустую коллекцию (измерено:
/// `ТипЗнч(Док.Страницы)` у свежего документа — «Не определено»). Ошибку
/// в этом состоянии даёт уже `Количество()`, потому что метода у
/// `Неопределено` нет.
///
/// # Errors
///
/// [`RtError::UnknownColumn`] на любом другом имени свойства: других
/// свойств у документа нет (измерено — `КоличествоСтраниц` платформа не
/// знает).
pub fn document_property(document: &DocumentObject, name: &str) -> RtResult<BslValue> {
    if name.eq_ignore_ascii_case("Вложения") || name.eq_ignore_ascii_case("Attachments") {
        // В отличие от `Страницы`, коллекция вложений есть и до чтения:
        // измерено, что у свежего документа это `КоллекцияВложенийPDF` с
        // нулём элементов, а не `Неопределено`.
        let items = document.state.borrow().attachments.clone();
        return Ok(BslValue::new_object(AttachmentsObject { items }));
    }
    if !name.eq_ignore_ascii_case("Страницы") && !name.eq_ignore_ascii_case("Pages") {
        return Err(RtError::UnknownColumn(name.to_string()));
    }
    if document.state.borrow().file.is_none() {
        return Ok(BslValue::Undefined);
    }
    Ok(BslValue::new_object(PagesObject {
        state: document.state.clone(),
    }))
}

/// Число вложений — общий путь `Количество()` и `Для Каждого`.
///
/// # Errors
///
/// [`RtError::MethodNotApplicable`], если получатель — не коллекция
/// вложений и не вложение.
pub fn attachment_count(attachments: &AttachmentsObject) -> RtResult<usize> {
    Ok(attachments.items.borrow().len())
}

/// `Вложения[Номер]` — вне диапазона ОШИБКА (измерено: `Вложения[99]`
/// платформа отвергает).
///
/// # Errors
///
/// [`RtError::IndexOutOfBounds`], если такого вложения нет.
pub fn attachment_at(attachments: &AttachmentsObject, index: usize) -> RtResult<BslValue> {
    let len = attachments.items.borrow().len();
    if index >= len {
        return Err(RtError::IndexOutOfBounds {
            index: index as i64,
            len,
        });
    }
    Ok(BslValue::new_object(AttachmentObject {
        items: attachments.items.clone(),
        index,
    }))
}

/// `Вложения.Получить(Номер)` — вне диапазона `Неопределено`, и на 99, и
/// на -1 (измерено), ровно как у страниц.
///
/// # Errors
///
/// [`RtError::TypeError`], если номер не число.
pub fn attachment_get(attachments: &AttachmentsObject, index: &BslValue) -> RtResult<BslValue> {
    let len = attachment_count(attachments)?;
    let BslValue::Number(number) = index else {
        return Err(RtError::TypeError {
            expected: "Число",
            op: "КоллекцияВложенийPDF.Получить",
        });
    };
    let Some(number) = number.to_i64_exact() else {
        return Ok(BslValue::Undefined);
    };
    match usize::try_from(number) {
        Ok(i) if i < len => attachment_at(attachments, i),
        _ => Ok(BslValue::Undefined),
    }
}

/// `Вложения.Индекс(Вложение)` — номер в этой же коллекции.
///
/// Строже, чем у страниц: чужое ЗНАЧЕНИЕ (число, массив) платформа
/// отвергает ошибкой типа, а не отдаёт -1 (измерено). Вложение из другой
/// коллекции — как раз тот случай, когда -1 законно.
///
/// # Errors
///
/// [`RtError::TypeError`], если аргумент не `ВложениеPDF`.
pub fn attachment_index_of(attachments: &AttachmentsObject, item: &BslValue) -> RtResult<BslValue> {
    let len = attachments.items.borrow().len();
    let found = match item
        .object_ref()
        .and_then(|object| object.downcast_ref::<AttachmentObject>())
    {
        Some(other) if Rc::ptr_eq(&attachments.items, &other.items) && other.index < len => {
            other.index as i64
        }
        Some(_) => -1,
        None => {
            return Err(RtError::TypeError {
                expected: "ВложениеPDF",
                op: "КоллекцияВложенийPDF.Индекс",
            });
        }
    };
    Ok(BslValue::Number(BslNumber::from_i64(found)))
}

/// Имя вложения из аргумента: строка как есть, число — своим
/// представлением, пустое имя — ошибка.
///
/// Так меряется платформа, и правило у `Найти` и `Добавить` одно:
/// `Найти(1)` отвечает `Неопределено` (то есть 1 стало именем «1» и не
/// нашлось), а `Найти("")` и `Добавить("", Данные)` — «Несоответствие
/// типов (параметр номер '1')».
pub(crate) fn attachment_name_arg(value: &BslValue, op: &'static str) -> RtResult<String> {
    let name = match value {
        BslValue::Str(text) => text.to_string(),
        // Целое число платформа принимает и превращает в имя (измерено:
        // после `ИмяФайла = 1` свойство отдаёт «1»). Дробное отвергается
        // здесь же: у него представление зависит от разделителя, и
        // придумывать его имени файла незачем.
        BslValue::Number(number) => match number.to_i64_exact() {
            Some(number) => number.to_string(),
            None => {
                return Err(RtError::TypeError {
                    expected: "Строка",
                    op,
                });
            }
        },
        _ => {
            return Err(RtError::TypeError {
                expected: "Строка",
                op,
            });
        }
    };
    if name.is_empty() {
        return Err(RtError::TypeError {
            expected: "непустое имя файла",
            op,
        });
    }
    Ok(name)
}

/// `Вложения.Найти(Имя)` — вложение с таким `ИмяФайла` либо
/// `Неопределено`. Аргумент ровно один (измерено).
///
/// # Errors
///
/// [`RtError::TypeError`], если имя пустое или не приводится к строке.
pub fn attachment_find(attachments: &AttachmentsObject, args: &[BslValue]) -> RtResult<BslValue> {
    let items = &attachments.items;
    let [name] = args else {
        return Err(pdf_err(
            "КоллекцияВложенийPDF.Найти ожидает ровно одно имя файла",
        ));
    };
    // Нестроковое и нечисловое значение платформа не бракует, а просто не
    // находит (измерено на `Найти(Новый Массив)` — «Не определено»).
    let name = match name {
        BslValue::Str(_) | BslValue::Number(_) => {
            attachment_name_arg(name, "КоллекцияВложенийPDF.Найти")?
        }
        _ => return Ok(BslValue::Undefined),
    };
    let found = items.borrow().iter().position(|item| item.name == name);
    match found {
        Some(index) => Ok(BslValue::new_object(AttachmentObject {
            items: items.clone(),
            index,
        })),
        None => Ok(BslValue::Undefined),
    }
}

/// `Вложения.Добавить(Имя, Данные[, ТипСодержимого[, ТипСвязи]])` —
/// процедура.
///
/// Одноимённое вложение СНИМАЕТСЯ, а новое дописывается В КОНЕЦ — то
/// есть коллекция ведёт себя как дерево имён, которым она и станет при
/// записи. Измерено: после `Добавить("а")`, `Добавить("б")` и повторного
/// `Добавить("а")` вложений двое, и порядок «б», «а», причём у «а» новые
/// тип содержимого и данные.
///
/// # Errors
///
/// [`RtError::Pdf`], если аргументов меньше двух или больше четырёх;
/// [`RtError::TypeError`], если имя пустое, если данные не
/// `ДвоичныеДанные`, если тип содержимого не строка или связь — не член
/// `ТипСвязиВложенияPDF`.
pub fn attachment_add(attachments: &AttachmentsObject, args: &[BslValue]) -> RtResult<()> {
    let items = &attachments.items;
    if args.len() < 2 || args.len() > 4 {
        return Err(pdf_err(
            "КоллекцияВложенийPDF.Добавить ожидает имя файла, данные и \
             необязательные тип содержимого и тип связи",
        ));
    }
    let name = attachment_name_arg(&args[0], "КоллекцияВложенийPDF.Добавить")?;
    let Some(data) = args[1].binary_data_bytes().map(<[u8]>::to_vec) else {
        return Err(RtError::TypeError {
            expected: "ДвоичныеДанные",
            op: "КоллекцияВложенийPDF.Добавить",
        });
    };
    let content_type = match args.get(2) {
        None | Some(BslValue::Undefined) => String::new(),
        Some(BslValue::Str(text)) => text.to_string(),
        Some(_) => {
            return Err(RtError::TypeError {
                expected: "Строка",
                op: "КоллекцияВложенийPDF.Добавить",
            });
        }
    };
    let relation = match args.get(3) {
        None => PdfRelation::Unspecified,
        Some(value) => relation_of(value, "КоллекцияВложенийPDF.Добавить")?,
    };
    let mut items = items.borrow_mut();
    let fresh = PdfAttachment {
        name,
        content_type,
        relation,
        data,
    };
    items.retain(|item| item.name != fresh.name);
    items.push(fresh);
    Ok(())
}

/// Член перечисления `ТипСвязиВложенияPDF` из значения языка.
pub(crate) fn relation_of(value: &BslValue, op: &'static str) -> RtResult<PdfRelation> {
    let BslValue::Enum(member) = value else {
        return Err(RtError::TypeError {
            expected: "ТипСвязиВложенияPDF",
            op,
        });
    };
    match member {
        EnumValue::PdfRelationSource => Ok(PdfRelation::Source),
        EnumValue::PdfRelationData => Ok(PdfRelation::Data),
        EnumValue::PdfRelationAlternative => Ok(PdfRelation::Alternative),
        EnumValue::PdfRelationSupplement => Ok(PdfRelation::Supplement),
        EnumValue::PdfRelationUnspecified => Ok(PdfRelation::Unspecified),
        _ => Err(RtError::TypeError {
            expected: "ТипСвязиВложенияPDF",
            op,
        }),
    }
}

/// Член перечисления по связи — обратное [`relation_of`].
pub(crate) fn relation_enum(relation: PdfRelation) -> EnumValue {
    match relation {
        PdfRelation::Source => EnumValue::PdfRelationSource,
        PdfRelation::Data => EnumValue::PdfRelationData,
        PdfRelation::Alternative => EnumValue::PdfRelationAlternative,
        PdfRelation::Supplement => EnumValue::PdfRelationSupplement,
        PdfRelation::Unspecified => EnumValue::PdfRelationUnspecified,
    }
}

/// `Вложения.Удалить(Номер)` либо `Удалить(Вложение)`.
///
/// Номер ВНЕ ДИАПАЗОНА не ошибка: измерено, что и `Удалить(99)`, и
/// `Удалить(-1)` удаляют ПОСЛЕДНЕЕ вложение, а не жалуются.
///
/// Что делает платформа с ПУСТОЙ коллекцией, этой оснасткой не выяснить:
/// `Удалить(0)` на пустой она встречает модальным окном, то есть немым
/// таймаутом, и `Попытка` его не ловит (проверено отдельной пробой —
/// вывод обрывается ровно перед вызовом). Строке в реестре открытых
/// вопросов там не место: она унесла бы весь сеанс замеров. Здесь пустая
/// коллекция остаётся пустой — это единственное продолжение правила
/// «номер вне диапазона удаляет последнее» на случай, когда последнего
/// нет.
///
/// # Errors
///
/// [`RtError::Pdf`], если аргумент не один;
/// [`RtError::TypeError`], если это не число и не `ВложениеPDF`.
pub fn attachment_delete(attachments: &AttachmentsObject, args: &[BslValue]) -> RtResult<()> {
    let items = &attachments.items;
    let [what] = args else {
        return Err(pdf_err(
            "КоллекцияВложенийPDF.Удалить ожидает ровно один аргумент",
        ));
    };
    let len = items.borrow().len();
    if len == 0 {
        return Ok(());
    }
    let index = match what {
        BslValue::Number(number) => match number.to_i64_exact() {
            Some(number) => match usize::try_from(number) {
                Ok(index) if index < len => index,
                _ => len - 1,
            },
            None => len - 1,
        },
        _ => match what
            .object_ref()
            .and_then(|object| object.downcast_ref::<AttachmentObject>())
        {
            Some(other) if Rc::ptr_eq(items, &other.items) && other.index < len => other.index,
            Some(_) => {
                return Err(RtError::TypeError {
                    expected: "ВложениеPDF этой же коллекции",
                    op: "КоллекцияВложенийPDF.Удалить",
                });
            }
            None => {
                return Err(RtError::TypeError {
                    expected: "Число или ВложениеPDF",
                    op: "КоллекцияВложенийPDF.Удалить",
                });
            }
        },
    };
    items.borrow_mut().remove(index);
    Ok(())
}

/// `Вложения.Очистить()` — аргументов не берёт (измерено).
///
/// # Errors
///
/// [`RtError::MethodNotApplicable`], если получатель не коллекция вложений.
pub fn attachment_clear(attachments: &AttachmentsObject) -> RtResult<()> {
    attachments.items.borrow_mut().clear();
    Ok(())
}

/// Свойства `ВложениеPDF`. Все четыре имени и все четыре английских
/// синонима измерены: `FileName`, `MIMEType`, `Content` и
/// `RelationshipType`. Перебором проверено и то, чего у платформы НЕТ:
/// `ContentType`, `MediaType`, `Mime`, `Relationship`, `AFRelationship`,
/// `Relation`, `Имя`, `Описание`, `Размер` и `Данные`.
///
/// # Errors
///
/// [`RtError::Pdf`], если вложение уже удалено из коллекции;
/// [`RtError::UnknownColumn`] на неизвестном имени.
pub fn attachment_property(attachment: &AttachmentObject, name: &str) -> RtResult<BslValue> {
    let items = attachment.items.borrow();
    let item = items
        .get(attachment.index)
        .ok_or_else(|| pdf_err("вложение уже удалено из коллекции"))?;
    if name.eq_ignore_ascii_case("ИмяФайла") || name.eq_ignore_ascii_case("FileName") {
        return Ok(BslValue::Str(BslString::from_str(&item.name)));
    }
    if name.eq_ignore_ascii_case("ТипСодержимого") || name.eq_ignore_ascii_case("MIMEType")
    {
        return Ok(BslValue::Str(BslString::from_str(&item.content_type)));
    }
    if name.eq_ignore_ascii_case("Содержимое") || name.eq_ignore_ascii_case("Content") {
        return Ok(BslValue::binary_data_of(item.data.clone()));
    }
    if name.eq_ignore_ascii_case("ТипСвязи") || name.eq_ignore_ascii_case("RelationshipType")
    {
        return Ok(BslValue::Enum(relation_enum(item.relation)));
    }
    Err(RtError::UnknownColumn(name.to_string()))
}

/// Присваивание в свойство `ВложениеPDF`. Пишутся ВСЕ ЧЕТЫРЕ (измерено —
/// в отличие от страницы, которая только читается).
///
/// # Errors
///
/// [`RtError::Pdf`], если вложение уже удалено из коллекции;
/// [`RtError::TypeError`] на значении не того типа;
/// [`RtError::UnknownColumn`] на неизвестном имени.
pub fn set_attachment_property(
    attachment: &AttachmentObject,
    name: &str,
    value: &BslValue,
) -> RtResult<()> {
    let mut items = attachment.items.borrow_mut();
    let item = items
        .get_mut(attachment.index)
        .ok_or_else(|| pdf_err("вложение уже удалено из коллекции"))?;
    if name.eq_ignore_ascii_case("ИмяФайла") || name.eq_ignore_ascii_case("FileName") {
        // Число платформа принимает и превращает в строку (измерено:
        // после `ИмяФайла = 1` свойство отдаёт «1»).
        item.name = attachment_name_arg(value, "ВложениеPDF.ИмяФайла")?;
        return Ok(());
    }
    if name.eq_ignore_ascii_case("ТипСодержимого") || name.eq_ignore_ascii_case("MIMEType")
    {
        let BslValue::Str(text) = value else {
            return Err(RtError::TypeError {
                expected: "Строка",
                op: "ВложениеPDF.ТипСодержимого",
            });
        };
        item.content_type = text.to_string();
        return Ok(());
    }
    if name.eq_ignore_ascii_case("Содержимое") || name.eq_ignore_ascii_case("Content") {
        let Some(bytes) = value.binary_data_bytes() else {
            return Err(RtError::TypeError {
                expected: "ДвоичныеДанные",
                op: "ВложениеPDF.Содержимое",
            });
        };
        item.data = bytes.to_vec();
        return Ok(());
    }
    if name.eq_ignore_ascii_case("ТипСвязи") || name.eq_ignore_ascii_case("RelationshipType")
    {
        item.relation = relation_of(value, "ВложениеPDF.ТипСвязи")?;
        return Ok(());
    }
    Err(RtError::UnknownColumn(name.to_string()))
}

/// Число страниц — общий путь `Количество()` и `Для Каждого`.
///
/// # Errors
///
/// [`RtError::Pdf`], если документ ничего не прочитал.
pub fn page_count(pages: &PagesObject) -> RtResult<usize> {
    let state = pages.state.borrow();
    let file = state
        .file
        .as_ref()
        .ok_or_else(|| pdf_err("у ДокументPDF ещё нет страниц: сначала нужен Прочитать"))?;
    Ok(file.page_count())
}

/// `Страницы.Получить(Номер)` — вне диапазона отдаёт `Неопределено`, и на
/// 99, и на -1 (измерено). Этим он и отличается от `Страницы[Номер]`.
///
/// # Errors
///
/// [`RtError::Pdf`], если документ ничего не прочитал;
/// [`RtError::TypeError`], если номер не число.
pub fn page_get(pages: &PagesObject, index: &BslValue) -> RtResult<BslValue> {
    let count = page_count(pages)?;
    let BslValue::Number(number) = index else {
        return Err(RtError::TypeError {
            expected: "Число",
            op: "КоллекцияСтраницPDF.Получить",
        });
    };
    let Some(number) = number.to_i64_exact() else {
        return Ok(BslValue::Undefined);
    };
    match usize::try_from(number) {
        Ok(i) if i < count => page_at(pages, i),
        _ => Ok(BslValue::Undefined),
    }
}

/// `Страницы[Номер]` — вне диапазона ОШИБКА (измерено: `Страницы[-1]`
/// платформа отвергает, хотя `Получить(-1)` отдаёт `Неопределено`).
///
/// # Errors
///
/// [`RtError::Pdf`], если документ ничего не прочитал;
/// [`RtError::IndexOutOfBounds`], если такой страницы нет.
pub fn page_at(pages: &PagesObject, index: usize) -> RtResult<BslValue> {
    let count = page_count(pages)?;
    if index >= count {
        return Err(RtError::IndexOutOfBounds {
            index: index as i64,
            len: count,
        });
    }
    Ok(BslValue::new_object(PageObject {
        state: pages.state.clone(),
        index,
    }))
}

/// `Страницы.Индекс(Страница)` — номер страницы в этой же коллекции, и
/// `-1`, если страница чужая. Номер измерен: `Индекс(Страницы[1])` — 1.
///
/// # Errors
///
/// [`RtError::Pdf`], если документ ничего не прочитал.
pub fn page_index_of(pages: &PagesObject, page: &BslValue) -> RtResult<BslValue> {
    let count = page_count(pages)?;
    let found = match page
        .object_ref()
        .and_then(|object| object.downcast_ref::<PageObject>())
    {
        Some(other) if Rc::ptr_eq(&pages.state, &other.state) && other.index < count => {
            other.index as i64
        }
        _ => -1,
    };
    Ok(BslValue::Number(BslNumber::from_i64(found)))
}

/// Свойства `СтраницаPDF`. Все восемь имён и оба языка измерены.
///
/// # Errors
///
/// [`RtError::Pdf`], если документ успел забыть прочитанное;
/// [`RtError::UnknownColumn`] на неизвестном имени.
pub fn page_property(page_object: &PageObject, name: &str) -> RtResult<BslValue> {
    let number = |value: i64| Ok(BslValue::Number(BslNumber::from_i64(value)));
    let state = page_object.state.borrow();
    let page = state
        .file
        .as_ref()
        .and_then(|file| file.page(page_object.index))
        .ok_or_else(|| pdf_err("страница относится к документу, который уже перечитан"))?;
    if name.eq_ignore_ascii_case("Номер") || name.eq_ignore_ascii_case("Number") {
        // Номер СТРАНИЦЫ с единицы, в отличие от номера в коллекции
        // (измерено: `Страницы[0].Номер` — 1).
        return number(page_object.index as i64 + 1);
    }
    if name.eq_ignore_ascii_case("Ширина") || name.eq_ignore_ascii_case("Width") {
        return number(page.width_mm());
    }
    if name.eq_ignore_ascii_case("Высота") || name.eq_ignore_ascii_case("Height") {
        return number(page.height_mm());
    }
    if name.eq_ignore_ascii_case("Ориентация") || name.eq_ignore_ascii_case("Orientation")
    {
        // Ориентация — ЧИСЛО, а не член перечисления (измерено:
        // `ТипЗнч(Страницы[0].Ориентация)` — «Число»).
        return number(page.rotate());
    }
    // Поля страницы приходят из `/TrimBox`; правило — в `margins_of`.
    for (ru, en, which) in [
        ("ПолеСлева", "LeftMargin", PdfMargin::Left),
        ("ПолеСправа", "RightMargin", PdfMargin::Right),
        ("ПолеСверху", "TopMargin", PdfMargin::Top),
        ("ПолеСнизу", "BottomMargin", PdfMargin::Bottom),
    ] {
        if name.eq_ignore_ascii_case(ru) || name.eq_ignore_ascii_case(en) {
            return number(page.margin(which));
        }
    }
    Err(RtError::UnknownColumn(name.to_string()))
}
