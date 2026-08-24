//! Объектный протокол читателей, писателей и их коллекций.

use super::*;

// --- читатель ---------------------------------------------------------------

/// Получатель-читатель: чужой тип получает ту же ошибку, что и прежний
/// строковый путь.
fn reader_of<'r>(
    receiver: &'r dyn ObjectProtocol,
    method: &'static str,
) -> RtResult<&'r ReaderObject> {
    receiver
        .downcast_ref::<ReaderObject>()
        .ok_or(RtError::MethodNotApplicable {
            method,
            receiver: "ЧтениеZipФайла",
        })
}

// У читателя свойств ровно два, и оба измерены; `Кодировка`, `Формат`,
// `ИмяФайла` и `РазмерАрхива` платформа не знает.
fn reader_items(receiver: &dyn ObjectProtocol, _c: &mut CallContext<'_>) -> RtResult<BslValue> {
    entries(reader_of(receiver, "Элементы")?)
}

fn reader_comment(receiver: &dyn ObjectProtocol, _c: &mut CallContext<'_>) -> RtResult<BslValue> {
    comment(reader_of(receiver, "Комментарий")?)
}

static READER_PROPERTIES: &[PropertyDescriptor] = &[
    PropertyDescriptor {
        names: &["Элементы", "Items"],
        get: reader_items,
        set: None,
    },
    PropertyDescriptor {
        names: &["Комментарий", "Comment"],
        get: reader_comment,
        set: None,
    },
];

fn reader_open(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _c: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let reader = reader_of(receiver, "Открыть")?;
    open(reader, reader.files.as_ref(), arguments)?;
    Ok(BslValue::Undefined)
}

fn reader_close(
    receiver: &dyn ObjectProtocol,
    _arguments: &[BslValue],
    _c: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    close(reader_of(receiver, "Закрыть")?)?;
    Ok(BslValue::Undefined)
}

fn reader_extract(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _c: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let reader = reader_of(receiver, "Извлечь")?;
    extract(
        &reader.state,
        reader.files.as_ref(),
        reader.descriptor().name,
        arguments,
    )?;
    Ok(BslValue::Undefined)
}

fn reader_extract_all(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _c: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let reader = reader_of(receiver, "ИзвлечьВсе")?;
    extract_all(
        &reader.state,
        reader.files.as_ref(),
        reader.descriptor().name,
        arguments,
    )?;
    Ok(BslValue::Undefined)
}

static READER_METHODS: &[MethodDescriptor] = &[
    MethodDescriptor::new(&["Открыть", "Open"], Arity::range(1, 2), reader_open),
    MethodDescriptor::new(&["Закрыть", "Close"], Arity::exact(0), reader_close),
    MethodDescriptor::new(&["Извлечь", "Extract"], Arity::range(2, 4), reader_extract),
    MethodDescriptor::new(
        &["ИзвлечьВсе", "ExtractAll"],
        Arity::range(1, 2),
        reader_extract_all,
    ),
];

impl ObjectProtocol for ReaderObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        self.descriptor()
    }

    fn property_table(&self) -> &'static [PropertyDescriptor] {
        READER_PROPERTIES
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        READER_METHODS
    }

    // Измерено: `ЗначениеЗаполнено` от читателя — «Да»; ошибка на закрытом
    // архиве важнее, чем ноль элементов.
    fn is_filled(&self) -> RtResult<bool> {
        Ok(true)
    }
}

// --- коллекция элементов ------------------------------------------------------

fn entries_of<'r>(
    receiver: &'r dyn ObjectProtocol,
    method: &'static str,
) -> RtResult<&'r EntriesObject> {
    receiver
        .downcast_ref::<EntriesObject>()
        .ok_or(RtError::MethodNotApplicable {
            method,
            receiver: "ЭлементыZipФайла",
        })
}

fn entries_count(
    receiver: &dyn ObjectProtocol,
    _arguments: &[BslValue],
    _c: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    count(entries_of(receiver, "Количество")?).map(|len| BslValue::number_from_i64(len as i64))
}

fn entries_get(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _c: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let items = entries_of(receiver, "Получить")?;
    match arguments {
        [index] => get(items, entry_index(index)?),
        _ => Err(RtError::MethodNotApplicable {
            method: "Получить",
            receiver: items.descriptor().name,
        }),
    }
}

fn entries_find(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _c: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let items = entries_of(receiver, "Найти")?;
    // Аргумент РОВНО один: измерено, что `Элементы.Найти("шум.bin", 1)`
    // платформа отвергает — «Слишком много фактических параметров».
    match arguments {
        [name] => find(items, name),
        _ => Err(RtError::MethodNotApplicable {
            method: "Найти",
            receiver: items.descriptor().name,
        }),
    }
}

fn entries_extract(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _c: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let items = entries_of(receiver, "Извлечь")?;
    extract(
        &items.state,
        items.files.as_ref(),
        items.descriptor().name,
        arguments,
    )?;
    Ok(BslValue::Undefined)
}

fn entries_extract_all(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _c: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let items = entries_of(receiver, "ИзвлечьВсе")?;
    extract_all(
        &items.state,
        items.files.as_ref(),
        items.descriptor().name,
        arguments,
    )?;
    Ok(BslValue::Undefined)
}

static ENTRIES_METHODS: &[MethodDescriptor] = &[
    MethodDescriptor::new(&["Количество", "Count"], Arity::exact(0), entries_count),
    MethodDescriptor::new(&["Получить", "Get"], Arity::exact(1), entries_get),
    MethodDescriptor::new(&["Найти", "Find"], Arity::exact(1), entries_find),
    MethodDescriptor::new(&["Извлечь", "Extract"], Arity::range(2, 4), entries_extract),
    MethodDescriptor::new(
        &["ИзвлечьВсе", "ExtractAll"],
        Arity::range(1, 2),
        entries_extract_all,
    ),
];

impl ObjectProtocol for EntriesObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        self.descriptor()
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        ENTRIES_METHODS
    }

    fn get_index(&self, index: &BslValue) -> RtResult<BslValue> {
        get(self, entry_index(index)?)
    }

    fn collection_len(&self) -> RtResult<usize> {
        count(self)
    }

    fn is_filled(&self) -> RtResult<bool> {
        Ok(true)
    }
}

/// Номер элемента из значения-индекса — та же семантика, что у `[]`
/// встроенных коллекций.
pub(crate) fn entry_index(index: &BslValue) -> RtResult<usize> {
    let BslValue::Number(number) = index else {
        return Err(RtError::BadIndex);
    };
    let index = number.to_i64_exact().ok_or(RtError::BadIndex)?;
    usize::try_from(index).map_err(|_| RtError::BadIndex)
}

// --- элемент архива -----------------------------------------------------------

fn entry_of<'r>(
    receiver: &'r dyn ObjectProtocol,
    method: &'static str,
) -> RtResult<&'r EntryObject> {
    receiver
        .downcast_ref::<EntryObject>()
        .ok_or(RtError::MethodNotApplicable {
            method,
            receiver: "ЭлементZipФайла",
        })
}

fn entry_extract(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _c: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    // Статус-кво прежней диспетчеризации: `Извлечь`/`ИзвлечьВсе`
    // принимали любой из трёх объектов чтения, включая элемент.
    let entry = entry_of(receiver, "Извлечь")?;
    extract(
        &entry.state,
        entry.files.as_ref(),
        entry.descriptor().name,
        arguments,
    )?;
    Ok(BslValue::Undefined)
}

fn entry_extract_all(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _c: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let entry = entry_of(receiver, "ИзвлечьВсе")?;
    extract_all(
        &entry.state,
        entry.files.as_ref(),
        entry.descriptor().name,
        arguments,
    )?;
    Ok(BslValue::Undefined)
}

static ENTRY_METHODS: &[MethodDescriptor] = &[
    MethodDescriptor::new(&["Извлечь", "Extract"], Arity::range(2, 4), entry_extract),
    MethodDescriptor::new(
        &["ИзвлечьВсе", "ExtractAll"],
        Arity::range(1, 2),
        entry_extract_all,
    ),
];

impl ObjectProtocol for EntryObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        self.descriptor()
    }

    /// Свойства элемента остаются строковым путём: их четырнадцать, все
    /// читаются из одной записи каталога под одним заимствованием
    /// состояния, и таблица из четырнадцати обёрток вокруг `entry_prop`
    /// была бы просто её копией. Свёртка внутри `entry_prop` — общая.
    fn get_property(&self, name: &str, _context: &mut CallContext<'_>) -> RtResult<BslValue> {
        entry_prop(self, name)
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        ENTRY_METHODS
    }

    fn is_filled(&self) -> RtResult<bool> {
        Ok(true)
    }
}

// --- писатель -----------------------------------------------------------------

fn writer_of<'r>(
    receiver: &'r dyn ObjectProtocol,
    method: &'static str,
) -> RtResult<&'r WriterObject> {
    receiver
        .downcast_ref::<WriterObject>()
        .ok_or(RtError::MethodNotApplicable {
            method,
            receiver: "ЗаписьZipФайла",
        })
}

fn writer_method_open(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _c: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    writer_open(writer_of(receiver, "Открыть")?, arguments)?;
    Ok(BslValue::Undefined)
}

fn writer_method_add(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _c: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let writer = writer_of(receiver, "Добавить")?;
    writer_add(writer, writer.files.as_ref(), arguments)?;
    Ok(BslValue::Undefined)
}

fn writer_method_write(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _c: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let writer = writer_of(receiver, "Записать")?;
    // Аргументов нет: измерено, что `Записать(имя)` платформа встречает
    // «Слишком много фактических параметров».
    if !arguments.is_empty() {
        return Err(RtError::MethodNotApplicable {
            method: "Записать",
            receiver: writer.descriptor().name,
        });
    }
    writer_write(writer, writer.files.as_ref())?;
    Ok(BslValue::Undefined)
}

fn writer_method_binary_data(
    receiver: &dyn ObjectProtocol,
    _arguments: &[BslValue],
    _c: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    writer_binary_data(writer_of(receiver, "ПолучитьДвоичныеДанные")?)
}

static WRITER_METHODS: &[MethodDescriptor] = &[
    MethodDescriptor::new(&["Открыть", "Open"], Arity::range(1, 7), writer_method_open),
    MethodDescriptor::new(&["Добавить", "Add"], Arity::range(1, 3), writer_method_add),
    MethodDescriptor::new(&["Записать", "Write"], Arity::exact(0), writer_method_write),
    MethodDescriptor::new(
        &["ПолучитьДвоичныеДанные", "GetBinaryData"],
        Arity::exact(0),
        writer_method_binary_data,
    ),
];

impl ObjectProtocol for WriterObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        self.descriptor()
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        WRITER_METHODS
    }

    // `ЗначениеЗаполнено` от писателя — измеренная ошибка «Проверка
    // мутабельных значений на заполненность не поддерживается»; её отдаёт
    // реализация протокола по умолчанию.
}
